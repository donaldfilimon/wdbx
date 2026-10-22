//! A durable, transactional outbox for work made while a scope is offline.
//!
//! This module answers the global constraints that "offline writes are
//! provisional; reauthorize on reconnect; auto-merge only independent edits;
//! preserve conflicting branches pending authorized resolution" and that
//! "no fixed offline expiry" applies to unsent work.
//!
//! # What an outbox is
//!
//! An [`OfflineOutbox`] is a durable, ordered queue of **proposals**.
//!
//! - Each operation has an **identifier** (its idempotency key), an optional
//!   **dependency** on another operation, a **kind tag**, a **binding** (the
//!   scope, contract revision, contract digest, policy version, and consent
//!   epoch it was written under), a **proposal**, and a **status**.
//! - The status vocabulary is closed: [`OperationStatus::Pending`],
//!   [`OperationStatus::InFlight`], [`OperationStatus::Acknowledged`],
//!   [`OperationStatus::Rejected`], and [`OperationStatus::Conflicted`].
//! - The proposal vocabulary is closed: an episode **write**, an episode
//!   **delete** (a `forgets` candidate), an **asset** transferred out of
//!   band, and a field-level **edit** of a mergeable configuration object.
//!
//! # What an outbox is not
//!
//! **The outbox is not a canonical episode writer, and it must never become
//! one.** This is the whole point of the design, so it is worth being
//! precise about it:
//!
//! - The outbox holds *proposals*. A proposed episode write is stored as the
//!   write the canonical store would compute its own commitment from. The
//!   outbox never holds a canonical episode digest: a write that carries a
//!   predicted commitment is refused at enqueue, because a digest the outbox
//!   carried would be a second canonical episode.
//! - The outbox never appends to an episode ledger. Nothing in this module
//!   can bind an episode, and nothing in it can make a record canonical.
//! - The outbox never holds episode *content*. It holds commitments — a
//!   payload commitment, a field-value digest, an asset digest — and the
//!   binding a proposal was written under. The canonical store remains the
//!   single writer of the canonical ledger, and the canonical ledger remains
//!   the only record of what was accepted.
//! - The outbox is *provisional*. An operation is a proposal until the
//!   server accepts it, and the acceptance is the canonical store's, not the
//!   outbox's.
//!
//! # Reauthorization on reconnect
//!
//! The reauthorization is a **separate, explicit step**. [`OfflineOutbox::drain`]
//! takes the *current* server policy and the canonical store, and answers a
//! per-operation verdict ([`ReauthorizeOutcome`]):
//!
//! - [`ReauthorizeOutcome::Authorized`] — the current policy accepts the
//!   operation;
//! - [`ReauthorizeOutcome::Refused`] — the current policy rejects it, with a
//!   content-free [`RefusalReason`];
//! - [`ReauthorizeOutcome::NeedsConsent`] — the consent epoch moved and the
//!   operation's bound epoch is stale;
//! - [`ReauthorizeOutcome::SchemaSkew`] — the operation was written under a
//!   contract revision or contract digest that is no longer current.
//!
//! The reauthorization does not re-implement the write gate. For a proposed
//! episode write it runs the canonical store's own non-mutating decision,
//! which runs the same validation, the same commitment check, and the same
//! quota checks as the append path. The outbox adds the checks that are
//! about the *binding*, because those are what a reconnect can change.
//!
//! # Preserving unsent work and conflicts
//!
//! No operation is dropped by the outbox's own logic. An operation the
//! current policy refuses keeps its proposal and gains a terminal status and
//! a reason. An operation that conflicts with another keeps its proposal,
//! gains [`OperationStatus::Conflicted`], and carries the recorded conflict.
//! An authorized caller inspects and resolves them with
//! [`OfflineOutbox::resolve`]; the only ways an operation leaves the outbox
//! are an explicit, authorized removal, and nothing is ever reaped
//! implicitly.
//!
//! **A removal another operation depends on is refused.** A dependent
//! operation names the operation it depends on, and removing that operation
//! would leave the dependent with a dependency the outbox does not hold: no
//! later drain could ever satisfy it, and the dependent would sit in the
//! queue for ever. The refusal ([`OutboxError::DependencyPresent`]) names
//! neither the operation nor the dependent, so the caller resolves the
//! dependents first — the leaves first — and the removal then succeeds. A
//! removal therefore never leaves behind a dependency that names an
//! operation the outbox does not hold.
//!
//! An **externally** truncated or damaged file is a different matter: the
//! outbox refuses to open it as corrupt rather than opening with fewer
//! operations than were queued.
//!
//! # Deterministic independent-edit merge
//!
//! Two edits that touch **disjoint** fields of the same mergeable object
//! merge automatically. Two edits that touch the **same** field conflict, and
//! so do two edits that carry incompatible versions of one object. The
//! merge is a pure function of the operations it is given: it never reads a
//! clock, it never compares timestamps, and it never invents a winner. See
//! the `merge` submodule for the model.
//!
//! # Resumable asset transfer
//!
//! An asset operation carries a content digest, a byte length, and a chunk
//! size — never the bytes. The chunk payload is supplied by the caller, one
//! chunk at a time; the outbox keeps the chunk's digest. A resumed transfer
//! asks only for the chunks it does not already hold, and the final digest
//! is verified from the recorded chunk digests, so a corrupted chunk is
//! detected without the outbox ever holding the asset. The digest is
//! numbered by **chunk index**, so the order a caller hands its chunks over
//! in never changes it: a correctly transferred asset is never reported as
//! corrupted. See the `asset` submodule.
//!
//! # No fixed offline expiry
//!
//! **An outbox operation never expires.** There is no TTL, no reaper, and no
//! timer anywhere in this module. The clock the outbox reads is used for one
//! thing only: the enqueue stamp a caller can observe. A proposal that was
//! made last year is still pending today, and a proposal that is still
//! pending is never discarded for being old. The minimum anti-resurrection
//! metadata is retained indefinitely, and an old client's unsent work is
//! revalidated, never silently dropped.
//!
//! # The status surface
//!
//! [`OutboxStatus`] counts the operations per status, and
//! [`OperationView`] is the content-free view of one operation: its
//! identifier, kind, status, and reason. Neither carries a proposal.
//!
//! # The on-disk format
//!
//! The outbox lives in one file, `outbox.v1.jsonl`, beside its writer lock
//! `outbox.v1.lock`. It is a JSON Lines file: a **header line**, then one
//! line per operation, in enqueue order, each line carrying a SHA-256
//! checksum over its own canonical JSON bytes. The header records the
//! number of operations the file holds and a checksum over the record set,
//! so a truncated file is refused as corrupt rather than opened with fewer
//! operations than were queued. A trailing fragment is corruption too, and is
//! not discarded: every write replaces the whole file atomically, so a torn
//! tail cannot arise from the outbox's own code path. A line that does not
//! parse, or whose checksum does not match, is corruption as well. The
//! details are in the `sidecar` submodule.
//!
//! The *set* of operations is append-only in the sense that matters: an
//! operation is only ever added, and it is only ever removed by an explicit,
//! authorized removal. The *file* is replaced rather than appended to, so a
//! status transition rewrites its own line. The cost of that replacement is
//! linear in the number of operations the outbox holds; the outbox is a
//! client-side queue, and the repository's own `episodes.v1.vectors.jsonl`
//! sidecar takes the same trade.
//!
//! # What this is not
//!
//! - **No transport.** The outbox has no network, no TLS, and no server.
//!   The caller supplies an [`OperationSink`] that stands in for the server,
//!   and the outbox records what the sink reports. There is no delivery
//!   acknowledgement from a real server here, and nothing in this module
//!   proves that a proposal reached one.
//! - **No model-weight distribution.** An asset is a blob the caller
//!   transfers. The outbox does not distribute model weights, does not
//!   decide what a model is, and does not fetch anything.
//! - **Not a general CRDT.** The merge covers a small, closed set of
//!   mergeable configuration objects and a field-level edit model. It is not
//!   a general conflict-free replicated data type, and it does not attempt
//!   to be one.
//! - **No automatic conflict resolution.** A conflict is recorded and
//!   preserved. The outbox never picks a winner, never applies a
//!   last-write-wins rule, and never compares timestamps.
//! - **No expiry, no reaping, and no garbage collection.** A terminal
//!   operation is retained until an authorized caller removes it. Nothing is
//!   ever reaped on the outbox's own initiative.
//! - **One writer per outbox.** The writer lock is held for the outbox's
//!   lifetime, exactly as the episode store holds its own.
//! - **No resnapshot.** The resnapshot and revalidation of an old client is
//!   the server's job, and the outbox holds no snapshot format.
//!
//! # Evidence
//!
//! `crates/abi-wdbx/tests/v3_offline_outbox.rs` pins the behaviour this
//! module claims: the round trip across a close and a reopen, the
//! reauthorization verdicts and their distinctness, the preservation of
//! refused and conflicted work, the authorized removal of a terminal
//! operation of any status and the room it makes, the refusal of a removal
//! another operation depends on, the determinism and the
//! order-independence of the merge, the resumable asset transfer and its
//! digest verification, the absence of any offline expiry, the outbox's not
//! being a canonical writer, the refusal of a corrupted or truncated file,
//! and the absence of episode content in the file. Everything is
//! deterministic and local: no network, and explicit sequence numbers
//! instead of a wall clock.

mod asset;
mod drain;
mod draining;
mod error;
mod merge;
mod outbox;
mod reauthorize;
mod sidecar;
mod types;

pub use asset::{
    AssetChunkOutcome, AssetState, MAX_ASSET_CHUNKS, asset_digest, asset_digest_of, chunk_digest,
};
pub use drain::{DrainReport, OperationOutcome, OperationSink, Submission, SubmissionVerdict};
pub use error::OutboxError;
pub use merge::{
    ConflictKind, ConflictSummary, EditProposal, FieldEdit, MAX_EDIT_FIELDS, MAX_FIELD_KEY_BYTES,
    MAX_OBJECT_ID_BYTES, MergeConflict, MergeObjectKind, MergePlan, MergeTarget, MergedObject,
    RecordedConflict,
};
pub use outbox::{AssetProgress, AssetVerification, OfflineOutbox};
pub use reauthorize::ReauthorizeOutcome;
pub use sidecar::{MAX_LINE_BYTES, MAX_OUTBOX_BYTES, OUTBOX_FILE, OUTBOX_LOCK};
pub use types::{
    AssetProposal, Clock, MAX_OPERATION_ID_BYTES, MAX_OPERATIONS, MAX_POLICY_VERSION_BYTES,
    MAX_SCOPE_BYTES, NewOperation, OperationBinding, OperationId, OperationKind, OperationPayload,
    OperationReason, OperationReceipt, OperationStatus, OperationView, OutboxOperation,
    OutboxStatus, RefusalReason, Resolution,
};
