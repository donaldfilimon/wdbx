//! The shard's durable state: its term, its membership, and the writes it
//! has acknowledged.
//!
//! The state file is the shard's *own* record, not a second canonical
//! episode. It holds a fencing epoch, the identities of the shard's
//! members, and the digests of the writes the shard acknowledged. It holds
//! no episode content, no payload, and no participant.
//!
//! The voters' ledgers remain the durable truth about *content*. The state
//! file is the durable truth about *agreement*: which writes the shard
//! promised, and under which term.

use crate::v3::replication::error::ShardError;
use crate::v3::replication::sidecar::{checksum, read_bounded_optional, write_atomic};
use crate::v3::replication::term::{ShardTerm, VoterId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The shard's state file, beside the shard's voters.
pub(super) const SHARD_STATE_FILE: &str = "shard.v1.json";

/// The voter's own term file, inside the voter's own directory.
pub(super) const VOTER_TERM_FILE: &str = "voter.v1.json";

/// The profile these files belong to.
const PROFILE: &str = "wdbx-quorum-shard-v1";

/// One write the shard acknowledged.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct AcknowledgedEntry {
    /// Canonical episode digest of the acknowledged write.
    pub(super) digest: [u8; 32],
    /// The ledger sequence the write took on its voters.
    pub(super) sequence: u64,
    /// The request identifier the write carried.
    pub(super) request_id: String,
    /// The term the acknowledgement was formed under.
    pub(super) term: u64,
}

/// The body of the shard's durable state.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ShardStateBody {
    profile: String,
    term: u64,
    members: Vec<String>,
    acknowledged: Vec<AcknowledgedEntry>,
}

impl ShardStateBody {
    /// Build the body of a shard's durable state.
    pub(super) fn new(
        term: ShardTerm,
        members: &[VoterId],
        acknowledged: &[AcknowledgedEntry],
    ) -> Self {
        Self {
            profile: PROFILE.to_string(),
            term: term.as_u64(),
            members: members
                .iter()
                .map(VoterId::as_str)
                .map(str::to_string)
                .collect(),
            acknowledged: acknowledged.to_vec(),
        }
    }

    /// The fencing epoch the shard was in.
    pub(super) fn term(&self) -> Result<ShardTerm, ShardError> {
        ShardTerm::from_u64(self.term)
    }

    /// The shard's members.
    pub(super) fn member_ids(&self) -> Result<Vec<VoterId>, ShardError> {
        self.members
            .iter()
            .map(|member| VoterId::new(member))
            .collect()
    }

    /// The writes the shard acknowledged.
    pub(super) fn acknowledged(&self) -> &[AcknowledgedEntry] {
        &self.acknowledged
    }
}

/// The body of a voter's durable term.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct VoterTermBody {
    profile: String,
    term: u64,
}

/// A sidecar file with a checksum over its own body.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Checksummed<T> {
    body: T,
    checksum: [u8; 32],
}

/// Read the shard's durable state, or the state of a fresh shard.
pub(super) fn load_shard_state(path: &Path) -> Result<ShardStateBody, ShardError> {
    let body: ShardStateBody =
        load_checksummed(path, ShardStateBody::new(ShardTerm::FIRST, &[], &[]))?;
    if body.profile != PROFILE {
        return Err(ShardError::Corrupt);
    }
    Ok(body)
}

/// Replace the shard's durable state atomically.
pub(super) fn store_shard_state(path: &Path, body: &ShardStateBody) -> Result<(), ShardError> {
    store_checksummed(path, body)
}

/// Read a voter's durable term, or the first epoch for a fresh voter.
pub(super) fn load_voter_term(path: &Path) -> Result<ShardTerm, ShardError> {
    let body: VoterTermBody = load_checksummed(
        path,
        VoterTermBody {
            profile: PROFILE.to_string(),
            term: ShardTerm::FIRST.as_u64(),
        },
    )?;
    if body.profile != PROFILE {
        return Err(ShardError::Corrupt);
    }
    ShardTerm::from_u64(body.term)
}

/// Replace a voter's durable term atomically.
pub(super) fn store_voter_term(path: &Path, term: ShardTerm) -> Result<(), ShardError> {
    store_checksummed(
        path,
        &VoterTermBody {
            profile: PROFILE.to_string(),
            term: term.as_u64(),
        },
    )
}

/// Read a checksummed sidecar, or the default when the file is absent.
fn load_checksummed<T>(path: &Path, default: T) -> Result<T, ShardError>
where
    T: DeserializeOwned + Serialize,
{
    let Some(raw) = read_bounded_optional(path)? else {
        return Ok(default);
    };
    let file: Checksummed<T> = serde_json::from_slice(&raw).map_err(|_| ShardError::Corrupt)?;
    let body = serde_json::to_vec(&file.body).map_err(|_| ShardError::Corrupt)?;
    if file.checksum != checksum(&body) {
        return Err(ShardError::Corrupt);
    }
    Ok(file.body)
}

/// Replace a checksummed sidecar atomically.
fn store_checksummed<T>(path: &Path, body: &T) -> Result<(), ShardError>
where
    T: Serialize,
{
    let encoded = serde_json::to_vec(body).map_err(|_| ShardError::Corrupt)?;
    let file = Checksummed {
        body,
        checksum: checksum(&encoded),
    };
    let mut bytes = serde_json::to_vec(&file).map_err(|_| ShardError::Corrupt)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}
