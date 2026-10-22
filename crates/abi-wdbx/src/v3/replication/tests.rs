//! Unit tests for the quorum-fenced episode shard.
//!
//! The integration suite (`crates/abi-wdbx/tests/v3_quorum_shard.rs`) pins
//! the public behaviour. These tests reach the crate-internal seams: the
//! durable state file, the vote term file, and the acknowledgement
//! accounting.

use super::*;
use crate::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass, StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod fencing;
mod quorum;
mod recovery;

/// A disposable directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(purpose: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("wdbx-quorum-{purpose}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The scope every test shard owns.
const SCOPE: &str = "guild_ref";

/// A store policy that admits one guild.
fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(
            SCOPE.to_string(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1_000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: None,
                retention_ttl_ms: None,
                retention_windows: None,
                authorized_principals: None,
            },
        )]),
    }
}

/// One memory-candidate write.
fn write(request_id: &str, operation_id: &str, seed: u8) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: SCOPE.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
            candidate: MemoryCandidate {
                class: MemoryClass::Fact,
                retention: RetentionClass::Durable,
                payload_commitment: [seed; 32],
                payload_bytes: 64,
                dimension: None,
                embedding_version: None,
                member_scoped: false,
                supersedes: None,
                forgets: None,
            },
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}
