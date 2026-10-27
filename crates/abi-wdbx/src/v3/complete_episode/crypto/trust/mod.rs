//! Private synthetic host-policy trust model. No credential authentication,
//! filesystem, custody, remote authority, admission or persistence is provided.
use super::profile::PROFILE_ID;
use super::verify::{KeySnapshot, SnapshotError};
use crate::v3::commitment::CanonicalValue as C;
use crate::v3::complete_episode::canonical::{Tree, Wire};
use crate::v3::complete_episode::{OpaqueRef, ScopeBinding};
use crate::v3::episode::SignerKeyId;
use ed25519_dalek::VerifyingKey;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
mod apply;
mod command;
mod snapshot;
pub(super) use snapshot::TrustSnapshot;
#[cfg(test)]
mod tests;

const KEY_CAP: usize = 1024;
const GRANT_CAP: usize = 64;
const HISTORY_CAP: usize = 4096;
type Result<T> = std::result::Result<T, TrustError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum TrustError {
    #[error("unauthorized synthetic administrator")]
    UnauthorizedAdmin,
    #[error("invalid trust identifier")]
    InvalidIdentifier,
    #[error("invalid key registration")]
    InvalidKeyRegistration,
    #[error("wrong key purpose or profile")]
    WrongKeyPurpose,
    #[error("duplicate trust scope")]
    DuplicateGrant,
    #[error("trust resource limit")]
    ResourceLimit,
    #[error("unknown trust key")]
    UnknownKey,
    #[error("key already registered")]
    AlreadyRegistered,
    #[error("invalid trust transition")]
    InvalidTransition,
    #[error("trust key revoked")]
    KeyRevoked,
    #[error("stale trust revision")]
    StaleTrustRevision,
    #[error("trust command replay conflict")]
    ReplayConflict,
    #[error("trust revision overflow")]
    RevisionOverflow,
    #[error("trust encoding failed")]
    Encoding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyStatus {
    Pending,
    Active,
    Retired,
    Revoked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyPurpose {
    P1CanonicalWriter,
    LegacyDigest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RevocationReason {
    Administration,
    SuspectedCompromise,
}

#[derive(Clone)]
struct Grant {
    bytes: Vec<u8>,
    binding: ScopeBinding,
}
redacted!(Grant);
fn grants(bindings: Vec<ScopeBinding>) -> Result<Vec<Grant>> {
    if bindings.len() > GRANT_CAP {
        return Err(TrustError::ResourceLimit);
    }
    let mut out = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let bytes = encode(binding.wire::<Tree>().map_err(|_| TrustError::Encoding)?)?;
        out.push(Grant { bytes, binding });
    }
    out.sort_by(|a, b| a.bytes.cmp(&b.bytes));
    if out.windows(2).any(|p| p[0].bytes == p[1].bytes) {
        return Err(TrustError::DuplicateGrant);
    }
    Ok(out)
}
fn opaque(value: &str) -> Result<OpaqueRef> {
    OpaqueRef::new(value).map_err(|_| TrustError::InvalidIdentifier)
}
fn encode(value: C) -> Result<Vec<u8>> {
    value.canonical_bytes().map_err(|_| TrustError::Encoding)
}
fn text(value: &str) -> C {
    C::Text(value.to_owned())
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Explicit trusted synthetic fixture. It does not authenticate credentials.
#[derive(Clone)]
pub(super) struct AdminAuthorization {
    namespace: OpaqueRef,
    principal: OpaqueRef,
    grants: Vec<Grant>,
}
redacted!(AdminAuthorization);
impl AdminAuthorization {
    pub(super) fn for_synthetic_host_policy(
        namespace: &str,
        principal: &str,
        bindings: Vec<ScopeBinding>,
    ) -> Result<Self> {
        Ok(Self {
            namespace: opaque(namespace)?,
            principal: opaque(principal)?,
            grants: grants(bindings)?,
        })
    }
}
#[derive(Clone)]
pub(super) struct KeyRegistration {
    id: SignerKeyId,
    key: VerifyingKey,
    principal: OpaqueRef,
    purpose: KeyPurpose,
    profile: String,
    grants: Vec<Grant>,
}
redacted!(KeyRegistration);
impl KeyRegistration {
    pub(super) fn new(
        public: &[u8],
        id: SignerKeyId,
        principal: &str,
        purpose: KeyPurpose,
        profile: &str,
        bindings: Vec<ScopeBinding>,
    ) -> Result<Self> {
        let bytes: [u8; 32] = public
            .try_into()
            .map_err(|_| TrustError::InvalidKeyRegistration)?;
        let key =
            VerifyingKey::from_bytes(&bytes).map_err(|_| TrustError::InvalidKeyRegistration)?;
        if key.is_weak() || SignerKeyId::for_key(&key) != id {
            return Err(TrustError::InvalidKeyRegistration);
        }
        if purpose != KeyPurpose::P1CanonicalWriter || profile != PROFILE_ID {
            return Err(TrustError::WrongKeyPurpose);
        }
        if bindings.is_empty() {
            return Err(TrustError::ResourceLimit);
        }
        Ok(Self {
            id,
            key,
            principal: opaque(principal)?,
            purpose,
            profile: profile.to_owned(),
            grants: grants(bindings)?,
        })
    }
}
#[derive(Clone)]
pub(super) enum Operation {
    Register(Box<KeyRegistration>),
    Activate(SignerKeyId),
    Retire(SignerKeyId),
    Revoke(SignerKeyId, RevocationReason, Option<u64>),
    Rotate(SignerKeyId, SignerKeyId),
}
redacted!(Operation);
impl Operation {
    pub(super) fn register(registration: KeyRegistration) -> Self {
        Self::Register(Box::new(registration))
    }
}
#[derive(Clone)]
pub(super) struct TrustCommand {
    id: OpaqueRef,
    operation: Operation,
}
redacted!(TrustCommand);
impl TrustCommand {
    pub(super) fn new(id: &str, operation: Operation) -> Result<Self> {
        Ok(Self {
            id: opaque(id)?,
            operation,
        })
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct TrustReceipt {
    revision: u64,
    command_digest: [u8; 32],
}
redacted!(TrustReceipt);
impl TrustReceipt {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn command_digest(&self) -> [u8; 32] {
        self.command_digest
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) enum TrustCommandResult {
    Applied(TrustReceipt),
    HistoricalReplay(TrustReceipt),
}
redacted!(TrustCommandResult);
impl TrustCommandResult {
    pub(super) fn receipt(&self) -> &TrustReceipt {
        match self {
            Self::Applied(r) | Self::HistoricalReplay(r) => r,
        }
    }
}
#[derive(Clone)]
struct Entry {
    registration: KeyRegistration,
    status: KeyStatus,
    since: u64,
    revocation: Option<(RevocationReason, Option<u64>)>,
}
redacted!(Entry);
#[derive(Clone)]
struct History {
    command: Vec<u8>,
    receipt: TrustReceipt,
    grants: Vec<Grant>,
}
redacted!(History);
#[derive(Clone)]
pub(super) struct TrustState {
    namespace: OpaqueRef,
    revision: u64,
    keys: BTreeMap<SignerKeyId, Entry>,
    history: BTreeMap<OpaqueRef, History>,
    order: Vec<OpaqueRef>,
}
redacted!(TrustState);
impl TrustState {
    pub(super) fn new(namespace: &str) -> Result<Self> {
        Ok(Self {
            namespace: opaque(namespace)?,
            revision: 0,
            keys: BTreeMap::new(),
            history: BTreeMap::new(),
            order: Vec::new(),
        })
    }
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn status(&self, id: &SignerKeyId) -> Option<KeyStatus> {
        self.keys.get(id).map(|e| e.status)
    }
}
