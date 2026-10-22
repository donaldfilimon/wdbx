//! The substrate-side binding between a vector index's identifiers and the
//! episode digests the constitutional query answers.
//!
//! # Why this exists
//!
//! A vector index answers in integers: an HNSW graph answers with the slot
//! identifiers it assigned, and the exact index answers with the contiguous
//! identifiers it generated. The constitutional query answers with episode
//! digests. Something has to hold the two together, and the ranking must not
//! guess it: a digest invented from an identifier would be a second canonical
//! episode, and the substrate owns the canonical record.
//!
//! This module holds that mapping, and it holds it as a *derived index* beside
//! the ledger.
//!
//! # The file
//!
//! The index lives in one file, `episodes.v1.vectors.jsonl`, beside its
//! writer lock `episodes.v1.vectors.lock`.
//!
//! The RECORD shape matches the episode store's own sidecars: one line per
//! binding, carrying the scope, the vector identifier and the episode digest
//! and nothing else — no payload, no actor, no event — with a checksum over
//! the line's own fields, and a whole-file atomic replacement on every write.
//!
//! The WRITER-EXCLUSION discipline did not match, and now does: the index
//! takes an owner file and a lock held for its lifetime, exactly as the
//! episode store and the offline outbox do, so a second writer is refused as
//! [`VectorBindingError::WriterBusy`] instead of racing. The file also
//! carries a HEADER — a first line recording the format version, the number
//! of binding lines and a checksum over the record set — so a truncation is
//! detected rather than silently believed. The format module documents both
//! in full.
//!
//! # Scope partitioning
//!
//! A binding belongs to a scope — the guild the episode was admitted in. The
//! scope is part of the line and part of the lookup key, so a lookup for one
//! scope can never answer with another scope's digest.
//!
//! A binding may name only a digest the scope's constitutional query
//! answered. That check is made against the *caller's* admission set: the
//! substrate trusts the caller to hand over the query's own answer, and it
//! does not independently prove admission — it has no way to re-derive the
//! answer without becoming a second constitutional read model. What it does
//! do is close the forging of the set: the admission set is **sealed**, so it
//! can only be minted by the store's own constitutional query
//! ([`EpisodeStore::admitted_scope`]); it carries that answer's own scope,
//! and a result whose receipt names another guild is refused. A cross-scope
//! attempt therefore fails twice over: the digest is not admitted there, and
//! the caller is presenting the wrong answer.
//!
//! The seal is about forging, not about authority: the caller's own query
//! decides what the answer holds, and the seal says nothing about which
//! query the caller may ask for.
//!
//! # What it is not
//!
//! The index holds no episode, no payload and no second canonical record. It
//! is the substrate's own record of which vector slot stands for which
//! episode. Losing it loses the mapping, not the episodes: the ledger stays
//! the canonical record, and the vectors become unbound — counted, never
//! resolved.

mod format;
mod lock;

use crate::v3::episode::{ConstitutionalQuery, EpisodeStore, EpisodeStoreError, MemoryResults};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use format::AllBindings;

/// The file name of the binding index beside an episode store.
pub const VECTOR_BINDING_FILE: &str = "episodes.v1.vectors.jsonl";

/// The file name of the binding index's writer lock.
pub const VECTOR_BINDING_LOCK: &str = "episodes.v1.vectors.lock";

/// The greatest number of bytes the binding index may occupy.
const MAX_BINDING_BYTES: u64 = 64 * 1024 * 1024;

/// The closed failures of the binding index.
///
/// Every variant is a static label: no variant carries a path, a raw digest,
/// a scope name or any episode content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VectorBindingError {
    /// The index file could not be read, written or replaced.
    #[error("vector_binding_io")]
    Io,
    /// The index file is not a state the writer can produce.
    #[error("vector_binding_corrupt")]
    Corrupt,
    /// The scope identifier is not a bounded identifier.
    #[error("vector_binding_scope_invalid")]
    InvalidScope,
    /// The admission set belongs to another scope.
    #[error("vector_binding_scope_mismatch")]
    ScopeMismatch,
    /// The digest is not admitted in the scope.
    #[error("vector_binding_not_admitted")]
    NotAdmitted,
    /// The vector identifier already names another digest in the scope.
    #[error("vector_binding_id_conflict")]
    IdConflict,
    /// The digest is already bound to another vector identifier in the scope.
    #[error("vector_binding_digest_conflict")]
    DigestConflict,
    /// Another writer holds the index's writer lock.
    #[error("vector_binding_writer_busy")]
    WriterBusy,
}

/// The digests one scope's constitutional query admits.
///
/// The admission set is what the canonical read model answered for one scope.
/// A binding may name only a digest in that set, so the mapping cannot point
/// at an episode the caller's answer did not hold — and, because the set
/// carries the scope the answer was for, a caller cannot present one scope's
/// answer while binding into another.
///
/// The check is a check against the *caller's* answer: the substrate trusts
/// the caller to hand over the query's own answer, and it does not
/// independently prove admission. What it does do is close the forging of the
/// set: the set is **sealed**, so it can only be minted by the store's own
/// constitutional query, and it carries that answer's own scope.
///
/// # The seal
///
/// The set has no public constructor. Its fields are private, it has no
/// `Default`, and the constructor that reads an answer is crate-private, so
/// a hand-built [`MemoryResults`] — the read model's answer type stays
/// public — cannot be turned into a set by any public path. The one public
/// way to have a set is
/// [`EpisodeStore::admitted_scope`](crate::v3::episode::EpisodeStore::admitted_scope):
/// it runs the constitutional query and mints the set from that answer, so
/// the scope is read from the answer itself.
///
/// The seal is about forging, not about authority: the caller's own query —
/// its scope and its constraints — decides what the answer holds, and the
/// substrate does not decide which query the caller may ask for.
///
/// The default constitutional answer excludes a forgotten record, because the
/// tombstone means the content is gone. Such a record is therefore not
/// bindable by default, and the vector arm does not serve an erased memory
/// unless the caller's own query asked for the forgotten records.
///
/// ```no_run
/// use abi_wdbx::v3::episode::{ConstitutionalQuery, EpisodeStore, MemoryResults, StorePolicy};
/// use abi_wdbx::v3::ranking::AdmittedScope;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // The read model's answer type is public, and a caller may build one.
/// let answer = MemoryResults {
///     scope: "guild_ref".to_string(),
///     results: Vec::new(),
///     disposition_counts: Default::default(),
/// };
/// assert_eq!(answer.scope, "guild_ref");
/// // The admission set is not: the store mints it from its own answer.
/// # let policy = StorePolicy { contract_revision: 2, contract_digest: [0; 32], guilds: Default::default() };
/// # let store = EpisodeStore::open(std::env::temp_dir(), policy)?;
/// let query = ConstitutionalQuery::new("guild_ref", 16);
/// let admitted: AdmittedScope = store.admitted_scope(&query)?;
/// assert_eq!(admitted.scope(), "guild_ref");
/// # Ok(())
/// # }
/// ```
///
/// A hand-built answer cannot become a set at all:
///
/// ```compile_fail
/// use abi_wdbx::v3::episode::MemoryResults;
/// use abi_wdbx::v3::ranking::AdmittedScope;
///
/// let answer = MemoryResults {
///     scope: "guild_ref".to_string(),
///     results: Vec::new(),
///     disposition_counts: Default::default(),
/// };
/// // The constructor that reads an answer is crate-private: the sealed set
/// // can only be minted by the store's own constitutional query.
/// let forged = AdmittedScope::from_results(&answer);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedScope {
    scope: String,
    digests: BTreeSet<[u8; 32]>,
    /// The seal on the set.
    ///
    /// The field is crate-private and its type has no public constructor, so
    /// the seal cannot be forged outside the crate — and neither can the set.
    seal: AdmissionSeal,
}

/// The seal on an admission set.
///
/// The seal is crate-private on purpose and has no public constructor: it is
/// what makes an admission set unforgeable from outside the crate. A set can
/// only be minted by the store's own constitutional query, so a caller that
/// builds an answer by hand — the read model's answer type stays public —
/// has no way to turn it into a set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmissionSeal {
    /// The set was minted from the store's own constitutional answer.
    Minted,
}

impl AdmittedScope {
    /// An admission set over an explicit scope and digest list.
    ///
    /// Crate-internal on purpose: an admission set is a canonical answer, and
    /// the only public way to have one is to ask the store for it.
    #[must_use]
    pub(crate) fn new(
        scope: impl Into<String>,
        digests: impl IntoIterator<Item = [u8; 32]>,
    ) -> Self {
        Self {
            scope: scope.into(),
            digests: digests.into_iter().collect(),
            seal: AdmissionSeal::Minted,
        }
    }

    /// The admission set one scope's constitutional answer holds.
    ///
    /// The scope is read from the answer, not taken as a separate argument,
    /// so an answer for one guild can never be presented as another's. As
    /// defence in depth, a result whose receipt names another guild is
    /// refused.
    ///
    /// Crate-internal on purpose: the set is sealed, and the only public way
    /// to have one is [`EpisodeStore::admitted_scope`], which runs the query
    /// and mints the set from that answer.
    ///
    /// # Errors
    ///
    /// Returns [`VectorBindingError::ScopeMismatch`] when a result's receipt
    /// names another guild than the answer's own scope.
    pub(crate) fn from_results(results: &MemoryResults) -> Result<Self, VectorBindingError> {
        let scope = results.scope.clone();
        for result in &results.results {
            if result.receipt.guild_ref != scope {
                return Err(VectorBindingError::ScopeMismatch);
            }
        }
        Ok(Self::new(
            scope,
            results.results.iter().map(|result| result.episode_digest),
        ))
    }

    /// The scope the set belongs to.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// How many digests the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.digests.len()
    }

    /// Whether the set holds no digest.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.digests.is_empty()
    }

    /// Whether the digest is admitted in the scope.
    #[must_use]
    pub fn contains(&self, digest: &[u8; 32]) -> bool {
        self.digests.contains(digest)
    }
}

impl EpisodeStore {
    /// The admission set one scope's constitutional query admits.
    ///
    /// This is the one public way to have an admission set. The store answers
    /// the constitutional query itself and mints the set from that answer, so
    /// the set names exactly the digests the store's own read model admitted
    /// and carries the scope the query named. The set is sealed: it has no
    /// public constructor, so a hand-built answer cannot become one.
    ///
    /// The check the binding index makes against the set is therefore exact
    /// about *that* answer — and it is still a check against the caller's
    /// answer: the substrate trusts the caller to hand over the query's own
    /// answer and does not independently prove admission. The seal closes the
    /// forging of the set; it does not make the substrate the authority on
    /// which query the caller may ask for.
    ///
    /// # Errors
    ///
    /// Returns [`EpisodeStoreError`] when the store refuses the query, and
    /// [`EpisodeStoreError::InvalidInput`] when the answer is not an answer
    /// for one scope — which the store's own guild-scoped query cannot
    /// produce.
    pub fn admitted_scope(
        &self,
        query: &ConstitutionalQuery,
    ) -> Result<AdmittedScope, EpisodeStoreError> {
        let results = self.query(query)?;
        AdmittedScope::from_results(&results).map_err(|_| EpisodeStoreError::InvalidInput)
    }
}

/// The receipt of one binding decision.
///
/// The receipt is content-free: it names a vector identifier and the digest it
/// stands for, and it carries no payload and no episode content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingReceipt {
    /// The vector identifier the binding names.
    pub vector_id: u64,
    /// The episode digest the binding names.
    pub episode_digest: [u8; 32],
    /// Whether the binding was newly written.
    ///
    /// Re-binding the identical pair writes nothing and answers `false`: the
    /// binding is already there, and the index is not a log of attempts.
    pub written: bool,
}

/// The durable, scope-partitioned binding between vector identifiers and
/// episode digests.
///
/// The index is derived state beside the ledger. It holds no episode and no
/// payload: it holds the substrate's own record of which vector slot stands
/// for which episode, and every binding it holds names a digest the scope's
/// constitutional query admitted.
///
/// Every write replaces the whole file. The mapping is small, and the
/// replacement is what makes an interrupted write leave the previous index
/// intact and openable. The index holds a writer lock for its lifetime, so a
/// second writer on the same file is [`VectorBindingError::WriterBusy`]; every
/// write also re-reads and merges the file on disk, so a binding another
/// writer added is never lost.
#[derive(Clone, Debug)]
pub struct VectorBindingIndex {
    path: PathBuf,
    bindings: AllBindings,
    /// The writer lock, held for the index's lifetime.
    ///
    /// A clone shares it: a clone is another *view* of the same writer, not a
    /// second writer.
    _lock: Arc<File>,
}

impl VectorBindingIndex {
    /// Open the binding index at an explicit path.
    ///
    /// An index that does not exist yet opens empty: the file is created with
    /// the first binding, not with the index. The directory must exist: the
    /// index takes its writer lock beside the file.
    ///
    /// A clone is another *view* of the same writer, not a second writer: it
    /// shares the lock, and it holds its own snapshot of the bindings.
    ///
    /// # Errors
    ///
    /// Returns [`VectorBindingError::Io`] when the file cannot be read, and
    /// [`VectorBindingError::Corrupt`] when the file is not a state the
    /// writer can produce. Returns [`VectorBindingError::WriterBusy`] when
    /// another writer holds the index's lock.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VectorBindingError> {
        let path = path.as_ref().to_path_buf();
        let lock_path = lock_path_for(&path);
        let lock = lock::owner_file(&lock_path)?;
        lock::acquire_writer_lock(&lock)?;
        let bindings = match read_index_file(&path)? {
            Some(raw) => format::decode(&raw)?,
            None => AllBindings::new(),
        };
        Ok(Self {
            path,
            bindings,
            _lock: Arc::new(lock),
        })
    }

    /// Open the binding index beside an episode store directory.
    ///
    /// The index lives in its own file, `episodes.v1.vectors.jsonl`, beside
    /// the ledger and the store's other sidecars. A directory that does not
    /// exist yet is created; a directory that does exist keeps its own
    /// permissions — derived state never changes the mode of the directory it
    /// merely reads.
    ///
    /// # Errors
    ///
    /// Returns [`VectorBindingError::Io`] when the directory or the file
    /// cannot be prepared or read, [`VectorBindingError::WriterBusy`] when
    /// another writer holds the index's lock, and
    /// [`VectorBindingError::Corrupt`] when the file is not a state the
    /// writer can produce.
    pub fn open_beside(directory: impl AsRef<Path>) -> Result<Self, VectorBindingError> {
        let directory = prepare_directory(directory.as_ref())?;
        Self::open(directory.join(VECTOR_BINDING_FILE))
    }

    /// The path the index lives at.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// How many bindings the index holds, over every scope.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(format::count(&self.bindings)).unwrap_or(usize::MAX)
    }

    /// Whether the index holds no binding.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// The scopes the index holds a binding for.
    pub fn scopes(&self) -> impl Iterator<Item = &str> {
        self.bindings.keys().map(String::as_str)
    }

    /// The bindings of one scope, in vector-identifier order.
    pub fn bindings_in(&self, scope: &str) -> impl Iterator<Item = (u64, [u8; 32])> + '_ {
        self.bindings
            .get(scope)
            .into_iter()
            .flat_map(|entries| entries.iter().map(|(id, digest)| (*id, *digest)))
    }

    /// The digest one vector identifier names, in one scope.
    ///
    /// A lookup never crosses a scope: an identifier bound in another scope
    /// answers `None` here.
    #[must_use]
    pub fn digest_of(&self, scope: &str, vector_id: u64) -> Option<[u8; 32]> {
        self.bindings
            .get(scope)
            .and_then(|entries| entries.get(&vector_id))
            .copied()
    }

    /// The vector identifier one digest names, in one scope.
    ///
    /// When several identifiers name one digest, the lowest identifier wins:
    /// the answer is deterministic.
    #[must_use]
    pub fn vector_id_of(&self, scope: &str, digest: &[u8; 32]) -> Option<u64> {
        self.bindings.get(scope).and_then(|entries| {
            entries
                .iter()
                .find(|(_, bound)| *bound == digest)
                .map(|(id, _)| *id)
        })
    }

    /// Check one binding decision without changing anything.
    ///
    /// The decision is the same one [`Self::bind`] makes; this only answers
    /// it, so a caller can check a binding before it mutates anything.
    ///
    /// # Errors
    ///
    /// Returns [`VectorBindingError::InvalidScope`] for an unbounded scope,
    /// [`VectorBindingError::ScopeMismatch`] when the admission set belongs to
    /// another scope, [`VectorBindingError::NotAdmitted`] when the digest is
    /// not admitted in the scope, and [`VectorBindingError::IdConflict`] or
    /// [`VectorBindingError::DigestConflict`] when the pair contradicts a
    /// binding the scope already holds.
    pub fn decide_bind(
        &self,
        scope: &str,
        vector_id: u64,
        digest: [u8; 32],
        admitted: &AdmittedScope,
    ) -> Result<(), VectorBindingError> {
        check_admission(scope, digest, admitted)?;
        let effective = self.effective_bindings()?;
        check_pair(&effective, scope, vector_id, digest)
    }

    /// Bind one vector identifier to one episode digest, in one scope.
    ///
    /// The digest must be admitted in the scope, and the admission set the
    /// caller presents must belong to that scope. The binding is refused when
    /// the identifier already names another digest in the scope, or when the
    /// digest is already bound to another identifier there.
    ///
    /// Re-binding the identical pair writes nothing and answers a receipt
    /// with `written: false`.
    ///
    /// # Errors
    ///
    /// Returns [`VectorBindingError::InvalidScope`] for an unbounded scope,
    /// [`VectorBindingError::ScopeMismatch`] when the admission set belongs to
    /// another scope, [`VectorBindingError::NotAdmitted`] when the digest is
    /// not admitted in the scope, [`VectorBindingError::IdConflict`] or
    /// [`VectorBindingError::DigestConflict`] when the pair contradicts a
    /// binding the scope already holds, and [`VectorBindingError::Io`] when
    /// the index cannot be written.
    pub fn bind(
        &mut self,
        scope: &str,
        vector_id: u64,
        digest: [u8; 32],
        admitted: &AdmittedScope,
    ) -> Result<BindingReceipt, VectorBindingError> {
        check_admission(scope, digest, admitted)?;
        // The effective view is this snapshot's bindings plus the ones already
        // on disk, so a stale snapshot cannot silently contradict — or
        // silently overwrite — another writer's binding.
        let effective = self.effective_bindings()?;
        check_pair(&effective, scope, vector_id, digest)?;
        let written = self.insert(scope, vector_id, digest);
        if written && let Err(error) = self.persist() {
            self.remove(scope, vector_id);
            return Err(error);
        }
        Ok(BindingReceipt {
            vector_id,
            episode_digest: digest,
            written,
        })
    }

    /// The bindings this snapshot holds, merged with the ones on disk.
    fn effective_bindings(&self) -> Result<AllBindings, VectorBindingError> {
        let disk = match read_index_file(&self.path)? {
            Some(raw) => Some(format::decode(&raw)?),
            None => None,
        };
        format::merge(&self.bindings, disk.as_ref())
    }

    /// Insert one binding, and report whether the index changed.
    fn insert(&mut self, scope: &str, vector_id: u64, digest: [u8; 32]) -> bool {
        let entries = self.bindings.entry(scope.to_string()).or_default();
        if entries.get(&vector_id) == Some(&digest) {
            return false;
        }
        entries.insert(vector_id, digest);
        true
    }

    /// Remove one binding, when a write that added it failed.
    fn remove(&mut self, scope: &str, vector_id: u64) {
        if let Some(entries) = self.bindings.get_mut(scope) {
            entries.remove(&vector_id);
        }
    }

    /// Replace the index file atomically.
    ///
    /// The file is re-read and merged first, so a binding another writer
    /// added is never lost, and the whole file is then written beside the
    /// index and renamed over it, so an interrupted write leaves the previous
    /// index intact and openable. The lines are written in the index's own
    /// order — scope, then vector identifier — so two runs that hold the same
    /// bindings write the same bytes.
    fn persist(&mut self) -> Result<(), VectorBindingError> {
        let disk = match read_index_file(&self.path)? {
            Some(raw) => Some(format::decode(&raw)?),
            None => None,
        };
        let merged = format::merge(&self.bindings, disk.as_ref())?;
        let body = format::encode(&merged)?;
        if u64::try_from(body.len()).unwrap_or(u64::MAX) > MAX_BINDING_BYTES {
            return Err(VectorBindingError::Io);
        }
        let temporary = self.temporary_path();
        let written = (|| -> Result<(), VectorBindingError> {
            let mut options = OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|_| VectorBindingError::Io)?;
            file.write_all(&body).map_err(|_| VectorBindingError::Io)?;
            file.sync_all().map_err(|_| VectorBindingError::Io)?;
            Ok(())
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
            return written;
        }
        if fs::rename(&temporary, &self.path).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(VectorBindingError::Io);
        }
        if let Some(parent) = self.path.parent() {
            sync_directory(parent)?;
        }
        self.bindings = merged;
        Ok(())
    }

    /// The path the atomic replacement is written to.
    ///
    /// The name is unique per writer — the process, a monotonic counter and
    /// the clock — so two writers, or a writer and a crashed one, can never
    /// publish each other's bytes.
    fn temporary_path(&self) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
            });
        let name = self.path.file_name().map_or_else(
            || "vector-bindings".to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        self.path.with_file_name(format!(
            ".{name}.{}.{counter}.{nanos}.tmp",
            std::process::id()
        ))
    }
}

/// Check the scope and the admission set of one binding.
fn check_admission(
    scope: &str,
    digest: [u8; 32],
    admitted: &AdmittedScope,
) -> Result<(), VectorBindingError> {
    if !format::bounded_scope(scope) {
        return Err(VectorBindingError::InvalidScope);
    }
    if admitted.scope() != scope {
        return Err(VectorBindingError::ScopeMismatch);
    }
    if !admitted.contains(&digest) {
        return Err(VectorBindingError::NotAdmitted);
    }
    Ok(())
}

/// Check one binding against the bindings a scope holds.
fn check_pair(
    bindings: &AllBindings,
    scope: &str,
    vector_id: u64,
    digest: [u8; 32],
) -> Result<(), VectorBindingError> {
    if let Some(existing) = bindings
        .get(scope)
        .and_then(|entries| entries.get(&vector_id))
        && *existing != digest
    {
        return Err(VectorBindingError::IdConflict);
    }
    if let Some(existing) = vector_id_in(bindings, scope, &digest)
        && existing != vector_id
    {
        return Err(VectorBindingError::DigestConflict);
    }
    Ok(())
}

/// The lowest identifier that names one digest in one scope.
fn vector_id_in(bindings: &AllBindings, scope: &str, digest: &[u8; 32]) -> Option<u64> {
    bindings.get(scope).and_then(|entries| {
        entries
            .iter()
            .find(|(_, bound)| *bound == digest)
            .map(|(id, _)| *id)
    })
}

/// The path the index's writer lock lives at.
fn lock_path_for(path: &Path) -> PathBuf {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    directory.join(VECTOR_BINDING_LOCK)
}

/// Prepare the directory the index lives in.
///
/// A missing directory is created with owner-only permissions; a path that is
/// a symlink or a regular file is refused. An existing directory keeps its
/// own mode: the index is derived state and never changes the permissions of
/// the directory it merely reads.
fn prepare_directory(path: &Path) -> Result<PathBuf, VectorBindingError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(VectorBindingError::Io);
        }
        Ok(_) => {
            return fs::canonicalize(path).map_err(|_| VectorBindingError::Io);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(VectorBindingError::Io),
    }
    fs::create_dir_all(path).map_err(|_| VectorBindingError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| VectorBindingError::Io)?;
    }
    fs::canonicalize(path).map_err(|_| VectorBindingError::Io)
}

/// Read the index file whole, bounded, when it exists.
fn read_index_file(path: &Path) -> Result<Option<Vec<u8>>, VectorBindingError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_BINDING_BYTES
            {
                return Err(VectorBindingError::Io);
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(VectorBindingError::Io),
    }
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| VectorBindingError::Io)?
        .take(MAX_BINDING_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| VectorBindingError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| VectorBindingError::Io)? > MAX_BINDING_BYTES {
        return Err(VectorBindingError::Io);
    }
    Ok(Some(raw))
}

/// Synchronize a directory entry, where the platform supports it.
fn sync_directory(path: &Path) -> Result<(), VectorBindingError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| VectorBindingError::Io)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
