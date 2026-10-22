//! Shared fixtures for the quota administration and retention window tests.
//!
//! The helpers here are the harness the test targets use: a disposable store
//! directory, the policy builders, the memory-candidate writes, and the
//! byte-level helpers the content-free assertion needs.
//!
//! The module is shared by more than one test target, so a helper that only
//! one of them uses is not dead code.

#![allow(dead_code)]

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass,
    QuotaAdjustmentRequest, RetentionClass, RetentionWindows, StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const QUOTA_FILE: &str = "episodes.v1.quotas.jsonl";
pub const LEDGER_FILE: &str = "episodes.v1.jsonl";

/// A disposable store directory that removes itself.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-quota-windows-{}", Uuid::new_v4())))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

/// The guild policy under test.
///
/// `token_budget` is the policy's own budget, which an adjustment may
/// override; `ttl_ms` is the single fallback window, which a per-class
/// window may override.
pub fn guild_policy(
    token_budget: u64,
    ttl_ms: Option<u64>,
    windows: Option<RetentionWindows>,
) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: Some(7),
        retention_ttl_ms: ttl_ms,
        retention_windows: windows,
        authorized_principals: None,
    }
}

pub fn policy(token_budget: u64) -> StorePolicy {
    policy_with(token_budget, None, None)
}

pub fn policy_with(
    token_budget: u64,
    ttl_ms: Option<u64>,
    windows: Option<RetentionWindows>,
) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (
                "guild_ref".into(),
                guild_policy(token_budget, ttl_ms, windows.clone()),
            ),
            (
                "other_guild".into(),
                guild_policy(token_budget, ttl_ms, windows),
            ),
        ]),
    }
}

pub fn write(
    request_id: &str,
    operation_id: &str,
    token_cost: u64,
    event: EpisodeEvent,
) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: "guild_ref".into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event,
        token_cost,
        expected_commitment: None,
        quiet: false,
    }
}

pub fn candidate(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped: true,
        supersedes: None,
        forgets: None,
    }
}

pub fn session(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Session,
        ..candidate(seed)
    }
}

pub fn operational(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Operational,
        ..candidate(seed)
    }
}

pub fn memory(
    request_id: &str,
    operation_id: &str,
    token_cost: u64,
    candidate: MemoryCandidate,
) -> EpisodeWrite {
    write(
        request_id,
        operation_id,
        token_cost,
        EpisodeEvent::MemoryCandidate {
            recorded_by: actor("abbey_service", ActorKind::Service),
            candidate,
        },
    )
}

pub fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    assert!(receipt.redacted);
    preview
}

pub fn rejected(store: &mut EpisodeStore, event: &EpisodeWrite) -> EpisodeStoreError {
    let error = store.propose_write(event).expect_err("rejected write");
    assert_eq!(
        store
            .preview_commitment(event)
            .map(|_| ())
            .unwrap_err()
            .to_string(),
        error.to_string(),
        "preview and append agree on the rejection"
    );
    error
}

/// Every file under the store directory with its exact bytes.
pub fn store_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("store dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("read store file");
                files.insert(path, bytes);
            }
        }
    }
    files
}

pub fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

pub fn adjustment(
    guild_ref: &str,
    token_budget: u64,
    storage_budget_bytes: u64,
    authority: ActorRef,
) -> QuotaAdjustmentRequest {
    QuotaAdjustmentRequest {
        guild_ref: guild_ref.into(),
        token_budget,
        storage_budget_bytes,
        authority,
    }
}

/// The owner of the guild: the authority the governance order admits.
pub fn owner() -> ActorRef {
    actor("guild_owner", ActorKind::GuildOwner)
}

/// The service that writes memory candidates.
pub fn service() -> ActorRef {
    actor("abbey_service", ActorKind::Service)
}

pub fn digest_encodings(bytes: &[u8; 32]) -> Vec<Vec<u8>> {
    use std::fmt::Write as _;
    let hex = bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    });
    let numbers: Vec<String> = bytes.iter().map(u8::to_string).collect();
    vec![
        bytes.to_vec(),
        hex.into_bytes(),
        format!("[{}]", numbers.join(",")).into_bytes(),
    ]
}

/// Whether the haystack holds the needle at any byte offset.
pub fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
