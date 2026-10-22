//! The quota sidecar: the durable, content-free record of every quota
//! decision.
//!
//! # The file format
//!
//! `episodes.v1.quotas.jsonl` holds a header line and one line per decision:
//!
//! - the header names the format, the number of decision lines, and a
//!   SHA-256 over those lines' bytes. A file truncated at a line boundary
//!   therefore fails the count or the checksum and is corruption, not a
//!   silently shorter history;
//! - every decision line carries its own checksum over its own fields;
//! - a trailing fragment — a final line without its terminator — is
//!   corruption, not a crash tail: the writer only ever writes whole lines,
//!   so a fragment is evidence of damage;
//! - the file is created with the first decision, never with the store, so an
//!   empty file is damage rather than a store that never adjusted a budget;
//! - every write replaces the whole file atomically, so a reader sees the old
//!   file or the new one, never a mixture.
//!
//! The decisions are content-free: a guild, two budgets, the host policy's
//! budgets in force when the decision was made, an authority, and an instant.
//! No digest, payload, event, or participant is ever written here.

use super::super::super::types::{ActorKind, ActorRef, StorePolicy};
use super::super::filesystem::{read_bounded_file_if_present, sync_directory};
use super::super::{EpisodeStoreError, GuildUsage, MAX_LEDGER_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The format marker of the sidecar's header line.
const FORMAT: &str = "wdbx-quota-index-v1";

/// What one decision did.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionKind {
    /// The decision set a budget for the guild.
    Adjustment,
    /// The decision lifted the budget the store held for the guild.
    Clear,
}

impl DecisionKind {
    /// The label the checksum covers.
    const fn label(self) -> &'static str {
        match self {
            Self::Adjustment => "adjustment",
            Self::Clear => "clear",
        }
    }
}

/// One durable quota decision line.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct QuotaLine {
    /// What the decision did.
    kind: DecisionKind,
    /// The guild the decision belongs to.
    guild_ref: String,
    /// The token budget the decision set, or the one it lifted.
    token_budget: u64,
    /// The storage budget the decision set, or the one it lifted.
    storage_budget_bytes: u64,
    /// The host policy's token budget in force when the decision was made.
    policy_token_budget: u64,
    /// The host policy's storage budget in force when the decision was made.
    policy_storage_budget_bytes: u64,
    /// The principal of the authority that made the decision.
    principal_id: String,
    /// The class of the authority that made the decision.
    authority_kind: ActorKind,
    /// The millisecond at which the decision was recorded.
    adjusted_at_ms: u64,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// The header line of the sidecar.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct QuotaHeader {
    /// The format marker.
    format: String,
    /// The number of decision lines the file holds.
    records: u64,
    /// SHA-256 over the decision lines' bytes.
    checksum: [u8; 32],
}

/// One recorded quota decision, as the index remembers it.
#[derive(Clone, Debug)]
pub(super) struct QuotaDecision {
    /// The guild the decision belongs to.
    pub(super) guild_ref: String,
    /// The token budget the decision set, or the one it lifted.
    pub(super) token_budget: u64,
    /// The storage budget the decision set, or the one it lifted.
    pub(super) storage_budget_bytes: u64,
    /// The authority that made the decision.
    pub(super) authority: ActorRef,
    /// The millisecond at which the decision was recorded.
    pub(super) adjusted_at_ms: u64,
    /// True when the decision lifted an override instead of setting one.
    pub(super) cleared: bool,
}

/// The budget override the index holds for one guild.
#[derive(Clone, Debug)]
pub(super) struct QuotaOverride {
    /// The token budget the override sets.
    pub(super) token_budget: u64,
    /// The storage budget the override sets.
    pub(super) storage_budget_bytes: u64,
    /// The host policy's token budget in force when the override was made.
    pub(super) policy_token_budget: u64,
    /// The host policy's storage budget in force when the override was made.
    pub(super) policy_storage_budget_bytes: u64,
    /// The authority that recorded the override.
    pub(super) authority: ActorRef,
    /// The millisecond at which the override was recorded.
    pub(super) adjusted_at_ms: u64,
}

impl QuotaOverride {
    /// Whether the override still applies under a host policy.
    ///
    /// An override is bound to the host policy it was made under: it applies
    /// while the policy states budgets at or below the ones in force when the
    /// override was recorded. When the host *raises* a guild's budget above
    /// them, the override no longer applies and the policy's budgets apply
    /// again.
    pub(super) fn in_force(&self, policy: (u64, u64)) -> bool {
        policy.0 <= self.policy_token_budget && policy.1 <= self.policy_storage_budget_bytes
    }

    /// The effective budgets of the override under a host policy.
    ///
    /// The result is never above the policy's budgets: the host policy is the
    /// outer bound of every budget the store enforces.
    pub(super) fn effective(&self, policy: (u64, u64)) -> (u64, u64) {
        if !self.in_force(policy) {
            return policy;
        }
        (
            self.token_budget.min(policy.0),
            self.storage_budget_bytes.min(policy.1),
        )
    }
}

/// The store's quota index: every quota decision, and the override each guild
/// currently carries.
///
/// The index is derived state beside the ledger. A store that never adjusted
/// a budget has no index file at all: the file is created with the first
/// decision, not with the store.
#[derive(Debug, Default)]
pub(in crate::v3::episode::store) struct QuotaIndex {
    /// The sidecar's path.
    path: PathBuf,
    /// Every decision line, in the order the store recorded them.
    lines: Vec<QuotaLine>,
    /// The override each guild currently carries.
    overrides: BTreeMap<String, QuotaOverride>,
    /// The guilds whose recorded usage exceeds their effective budget.
    over_budget: BTreeSet<String>,
}

impl QuotaIndex {
    /// Open the index, replaying its decisions.
    ///
    /// A missing file is a store that never adjusted a budget. A file that
    /// does not parse, whose header does not match its decisions, whose lines
    /// fail their own checksums, or that ends in a fragment is corruption:
    /// the store refuses to open rather than silently forgetting a budget.
    pub(in crate::v3::episode::store) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                ..Self::default()
            });
        };
        let (lines, overrides) = decode(&raw)?;
        Ok(Self {
            path: path.to_path_buf(),
            lines,
            overrides,
            over_budget: BTreeSet::new(),
        })
    }

    /// The override the index holds for one guild, when it holds one.
    pub(super) fn override_for(&self, guild_ref: &str) -> Option<&QuotaOverride> {
        self.overrides.get(guild_ref)
    }

    /// The budgets the write gate uses for one guild under a host policy.
    ///
    /// `None` means the host policy does not serve that guild at all.
    pub(super) fn effective_budgets(
        &self,
        policy: &StorePolicy,
        guild_ref: &str,
    ) -> Option<(u64, u64)> {
        let guild = policy.guilds.get(guild_ref)?;
        let budgets = (guild.token_budget, guild.storage_budget_bytes);
        Some(
            self.overrides
                .get(guild_ref)
                .map_or(budgets, |recorded| recorded.effective(budgets)),
        )
    }

    /// The decisions the index holds for one guild, in the order they were
    /// recorded.
    pub(super) fn history(&self, guild_ref: &str) -> Vec<QuotaDecision> {
        self.lines
            .iter()
            .filter(|line| line.guild_ref == guild_ref)
            .map(|line| QuotaDecision {
                guild_ref: line.guild_ref.clone(),
                token_budget: line.token_budget,
                storage_budget_bytes: line.storage_budget_bytes,
                authority: ActorRef {
                    principal_id: line.principal_id.clone(),
                    kind: line.authority_kind,
                },
                adjusted_at_ms: line.adjusted_at_ms,
                cleared: line.kind == DecisionKind::Clear,
            })
            .collect()
    }

    /// Detect the guilds whose recorded usage exceeds their effective budget.
    ///
    /// The store surfaces this inconsistency instead of refusing to open: a
    /// host policy lowered below the usage a guild has already recorded must
    /// not brick the store. The adjustment API refuses to *create* the state;
    /// this only reports it when the host policy creates it.
    pub(in crate::v3::episode::store) fn detect_over_budget(
        &mut self,
        policy: &StorePolicy,
        usage: &BTreeMap<String, GuildUsage>,
    ) {
        self.over_budget.clear();
        for guild_ref in policy.guilds.keys() {
            let used = usage.get(guild_ref).copied().unwrap_or_default();
            let Some((token_budget, storage_budget_bytes)) =
                self.effective_budgets(policy, guild_ref)
            else {
                continue;
            };
            if used.tokens > token_budget || used.bytes > storage_budget_bytes {
                self.over_budget.insert(guild_ref.clone());
            }
        }
    }

    /// Whether the store detected an over-budget guild when it opened.
    pub(super) fn is_over_budget(&self, guild_ref: &str) -> bool {
        self.over_budget.contains(guild_ref)
    }

    /// Record one adjustment.
    ///
    /// The sidecar is written and synchronized before the receipt that
    /// reports the decision is returned, so a receipt never claims a budget
    /// the store did not durably record.
    pub(super) fn commit_adjustment(
        &mut self,
        guild_ref: &str,
        token_budget: u64,
        storage_budget_bytes: u64,
        policy: (u64, u64),
        authority: &ActorRef,
        adjusted_at_ms: u64,
    ) -> Result<(), EpisodeStoreError> {
        let line = QuotaLine {
            kind: DecisionKind::Adjustment,
            guild_ref: guild_ref.to_string(),
            token_budget,
            storage_budget_bytes,
            policy_token_budget: policy.0,
            policy_storage_budget_bytes: policy.1,
            principal_id: authority.principal_id.clone(),
            authority_kind: authority.kind,
            adjusted_at_ms,
            checksum: [0; 32],
        };
        let line = QuotaLine {
            checksum: line_checksum(&line),
            ..line
        };
        self.write_with(Some(&line))?;
        self.lines.push(line);
        self.overrides.insert(
            guild_ref.to_string(),
            QuotaOverride {
                token_budget,
                storage_budget_bytes,
                policy_token_budget: policy.0,
                policy_storage_budget_bytes: policy.1,
                authority: authority.clone(),
                adjusted_at_ms,
            },
        );
        Ok(())
    }

    /// Record one clear: the guild's override is lifted and the host policy's
    /// budget applies again.
    pub(super) fn commit_clear(
        &mut self,
        guild_ref: &str,
        token_budget: u64,
        storage_budget_bytes: u64,
        policy: (u64, u64),
        authority: &ActorRef,
        cleared_at_ms: u64,
    ) -> Result<(), EpisodeStoreError> {
        let line = QuotaLine {
            kind: DecisionKind::Clear,
            guild_ref: guild_ref.to_string(),
            token_budget,
            storage_budget_bytes,
            policy_token_budget: policy.0,
            policy_storage_budget_bytes: policy.1,
            principal_id: authority.principal_id.clone(),
            authority_kind: authority.kind,
            adjusted_at_ms: cleared_at_ms,
            checksum: [0; 32],
        };
        let line = QuotaLine {
            checksum: line_checksum(&line),
            ..line
        };
        self.write_with(Some(&line))?;
        self.lines.push(line);
        self.overrides.remove(guild_ref);
        Ok(())
    }

    /// Replace the sidecar with its decisions plus one more.
    ///
    /// The whole file is rewritten, because the header carries a checksum
    /// over every decision line: a file that was truncated at a line
    /// boundary must fail that checksum rather than look like a shorter
    /// history.
    fn write_with(&self, extra: Option<&QuotaLine>) -> Result<(), EpisodeStoreError> {
        let mut body = Vec::new();
        for line in &self.lines {
            body.extend_from_slice(&encode_line(line)?);
        }
        if let Some(line) = extra {
            body.extend_from_slice(&encode_line(line)?);
        }
        let records = u64::try_from(self.lines.len() + usize::from(extra.is_some()))
            .map_err(|_| EpisodeStoreError::Corrupt)?;
        let header = QuotaHeader {
            format: FORMAT.to_string(),
            records,
            checksum: sha256(&body),
        };
        let mut bytes = serde_json::to_vec(&header).map_err(|_| EpisodeStoreError::Corrupt)?;
        bytes.push(b'\n');
        bytes.extend_from_slice(&body);
        if u64::try_from(bytes.len()).map_err(|_| EpisodeStoreError::Io)? > MAX_LEDGER_BYTES {
            return Err(EpisodeStoreError::StorageBudget);
        }
        write_atomic(&self.path, &bytes)
    }
}

/// SHA-256 over one decision line's own fields.
///
/// The parts are length-prefixed so no two different field sequences can
/// produce the same input. The checksum is a checksum, not a signature: it
/// detects a corrupted or half-written line, exactly as the episode digest
/// detects a mutated ledger line. It proves nothing about who wrote it.
fn line_checksum(line: &QuotaLine) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in [
        line.kind.label().as_bytes(),
        line.guild_ref.as_bytes(),
        &line.token_budget.to_le_bytes()[..],
        &line.storage_budget_bytes.to_le_bytes()[..],
        &line.policy_token_budget.to_le_bytes()[..],
        &line.policy_storage_budget_bytes.to_le_bytes()[..],
        line.principal_id.as_bytes(),
        line.authority_kind.label().as_bytes(),
        &line.adjusted_at_ms.to_le_bytes()[..],
    ] {
        hasher.update(u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// SHA-256 over a byte string.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// The serialized bytes of one decision line, terminator included.
fn encode_line(line: &QuotaLine) -> Result<Vec<u8>, EpisodeStoreError> {
    let mut bytes = serde_json::to_vec(line).map_err(|_| EpisodeStoreError::Corrupt)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// The line's bytes without its terminator.
fn strip_terminator(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\n").unwrap_or(line)
}

/// Read the sidecar's decisions and replay them into the guild overrides.
fn decode(
    raw: &[u8],
) -> Result<(Vec<QuotaLine>, BTreeMap<String, QuotaOverride>), EpisodeStoreError> {
    // The sidecar is created with the first decision: an empty file is
    // damage, not a store that never adjusted a budget.
    if raw.is_empty() {
        return Err(EpisodeStoreError::Corrupt);
    }
    // Every line the writer writes is terminated. A final line without its
    // terminator is a fragment: the writer never produces one, so it is
    // damage rather than a crash tail.
    if !raw.ends_with(b"\n") {
        return Err(EpisodeStoreError::Corrupt);
    }
    let mut parts = raw.split_inclusive(|byte| *byte == b'\n');
    let head = parts.next().ok_or(EpisodeStoreError::Corrupt)?;
    let header: QuotaHeader =
        serde_json::from_slice(strip_terminator(head)).map_err(|_| EpisodeStoreError::Corrupt)?;
    if header.format != FORMAT {
        return Err(EpisodeStoreError::Corrupt);
    }
    let mut body = Vec::new();
    let mut lines = Vec::new();
    for part in parts {
        let line: QuotaLine = serde_json::from_slice(strip_terminator(part))
            .map_err(|_| EpisodeStoreError::Corrupt)?;
        if line.checksum != line_checksum(&line) {
            return Err(EpisodeStoreError::Corrupt);
        }
        body.extend_from_slice(part);
        lines.push(line);
    }
    // The header's count and checksum cover the whole decision set: a file
    // truncated at a line boundary fails here.
    if u64::try_from(lines.len()).map_err(|_| EpisodeStoreError::Corrupt)? != header.records {
        return Err(EpisodeStoreError::Corrupt);
    }
    if sha256(&body) != header.checksum {
        return Err(EpisodeStoreError::Corrupt);
    }
    let overrides = replay(&lines);
    Ok((lines, overrides))
}

/// Replay the decisions into the override each guild carries.
fn replay(lines: &[QuotaLine]) -> BTreeMap<String, QuotaOverride> {
    let mut overrides = BTreeMap::new();
    for line in lines {
        match line.kind {
            DecisionKind::Adjustment => {
                overrides.insert(
                    line.guild_ref.clone(),
                    QuotaOverride {
                        token_budget: line.token_budget,
                        storage_budget_bytes: line.storage_budget_bytes,
                        policy_token_budget: line.policy_token_budget,
                        policy_storage_budget_bytes: line.policy_storage_budget_bytes,
                        authority: ActorRef {
                            principal_id: line.principal_id.clone(),
                            kind: line.authority_kind,
                        },
                        adjusted_at_ms: line.adjusted_at_ms,
                    },
                );
            }
            DecisionKind::Clear => {
                overrides.remove(&line.guild_ref);
            }
        }
    }
    overrides
}

/// Replace the sidecar atomically.
///
/// The bytes are written to a temporary file in the same directory, the
/// temporary file is synchronized, renamed over the target, and the directory
/// entry is synchronized. A reader therefore sees the old file or the new
/// one, never a half-written mixture.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), EpisodeStoreError> {
    let parent = path.parent().ok_or(EpisodeStoreError::Io)?;
    let name = path.file_name().ok_or(EpisodeStoreError::Io)?;
    let mut temporary = parent.to_path_buf();
    temporary.push(format!(".{}.tmp", name.to_string_lossy()));
    {
        let mut options = OpenOptions::new();
        options.create(true).write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| EpisodeStoreError::Io)?;
        if file.write_all(bytes).is_err() || file.sync_all().is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(EpisodeStoreError::Io);
        }
    }
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(EpisodeStoreError::Io);
    }
    sync_directory(parent)
}
