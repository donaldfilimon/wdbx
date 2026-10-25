//! Private pure-message strict verification. A report is not an append capability.
//! Host-provisioned key snapshots contain mathematical verification material,
//! not scoped grants, trusted signing times, custody, or admission history.
use super::super::{
    Actor, Authority, COMPLETE_EPISODE_DOMAIN, Digest32, Identity, OpaqueRef, ScopeBinding,
};
use super::profile::{CryptoLimits, PROFILE_ID, ParsedP1, ProfileError, parse_p1};
use crate::v3::commitment::CanonicalValue as C;
use crate::v3::episode::SignerKeyId;
use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

/// Verifier configuration comes from the host, not unsigned transport metadata.
pub(super) struct ExpectedContext {
    pub profile: &'static str,
    pub domain: &'static str,
    pub version: u64,
    pub contract: Digest32,
    pub binding: ScopeBinding,
    pub namespace: OpaqueRef,
}
redacted!(ExpectedContext);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum SnapshotError {
    #[error("verification snapshot resource limit")]
    ResourceLimit,
    #[error("invalid verification key")]
    InvalidKey,
    #[error("duplicate verification key")]
    DuplicateKey,
    #[error("verification snapshot encoding failed")]
    Encoding,
}

/// Immutable public verification material. No key here authorizes a new write.
pub(super) struct KeySnapshot {
    namespace: OpaqueRef,
    revision: u64,
    digest: [u8; 32],
    keys: BTreeMap<SignerKeyId, VerifyingKey>,
}
redacted!(KeySnapshot);
impl KeySnapshot {
    pub(super) fn new(
        namespace: OpaqueRef,
        revision: u64,
        keys: Vec<VerifyingKey>,
    ) -> Result<Self, SnapshotError> {
        if keys.len() > 1024 {
            return Err(SnapshotError::ResourceLimit);
        }
        let mut entries = BTreeMap::new();
        for key in keys {
            if key.is_weak() {
                return Err(SnapshotError::InvalidKey);
            }
            if entries.insert(SignerKeyId::for_key(&key), key).is_some() {
                return Err(SnapshotError::DuplicateKey);
            }
        }
        let encoded = C::Array(vec![
            C::Text("wdbx-p1-key-snapshot-prototype-v1".to_owned()),
            C::Text(namespace.0.clone()),
            C::Unsigned(revision),
            C::Array(
                entries
                    .iter()
                    .map(|(id, key)| {
                        C::Array(vec![
                            C::Text(id.as_str().to_owned()),
                            C::Bytes(key.to_bytes().to_vec()),
                        ])
                    })
                    .collect(),
            ),
        ])
        .canonical_bytes()
        .map_err(|_| SnapshotError::Encoding)?;
        Ok(Self {
            namespace,
            revision,
            digest: Sha256::digest(encoded).into(),
            keys: entries,
        })
    }
    pub(super) fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Integrity {
    Valid,
    Invalid,
    Unverifiable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VerificationReason {
    UnsupportedProfile,
    DomainMismatch,
    ContractMismatch,
    ScopeMismatch,
    NamespaceMismatch,
    KeyIdentityMismatch,
    UnknownKey,
    InvalidSignature,
}

/// Only this verifier constructs reports; no deserializer or mutable accessor.
/// Original bytes and all provenance bindings survive even an unknown key.
pub(super) struct VerificationReport {
    parsed: ParsedP1,
    payload_digest: [u8; 32],
    container_digest: [u8; 32],
    namespace: OpaqueRef,
    snapshot_revision: u64,
    snapshot_digest: [u8; 32],
    integrity: Integrity,
    reason: Option<VerificationReason>,
}
redacted!(VerificationReport);
impl VerificationReport {
    pub(super) fn origin_actor(&self) -> &Actor {
        self.parsed.origin_actor()
    }
    pub(super) fn authority(&self) -> &Authority {
        self.parsed.authority()
    }
    pub(super) fn integrity(&self) -> Integrity {
        self.integrity
    }
    pub(super) fn reason(&self) -> Option<VerificationReason> {
        self.reason
    }
    pub(super) fn payload(&self) -> &[u8] {
        self.parsed.payload()
    }
    pub(super) fn container(&self) -> &[u8] {
        self.parsed.container()
    }
    pub(super) fn protected(&self) -> &[u8] {
        self.parsed.protected()
    }
    pub(super) fn profile(&self) -> &'static str {
        PROFILE_ID
    }
    pub(super) fn domain(&self) -> &'static str {
        COMPLETE_EPISODE_DOMAIN
    }
    pub(super) fn version(&self) -> u64 {
        1
    }
    pub(super) fn identity(&self) -> &Identity {
        self.parsed.identity()
    }
    pub(super) fn signer(&self) -> &SignerKeyId {
        self.parsed.key_id()
    }
    pub(super) fn payload_digest(&self) -> [u8; 32] {
        self.payload_digest
    }
    pub(super) fn container_digest(&self) -> [u8; 32] {
        self.container_digest
    }
    pub(super) fn namespace(&self) -> &OpaqueRef {
        &self.namespace
    }
    pub(super) fn snapshot_revision(&self) -> u64 {
        self.snapshot_revision
    }
    pub(super) fn snapshot_digest(&self) -> [u8; 32] {
        self.snapshot_digest
    }
}

pub(super) fn verify_p1(
    parsed: ParsedP1,
    expected: &ExpectedContext,
    keys: &KeySnapshot,
) -> VerificationReport {
    let reason = if expected.profile != PROFILE_ID {
        Some(VerificationReason::UnsupportedProfile)
    } else if expected.domain != COMPLETE_EPISODE_DOMAIN || expected.version != 1 {
        Some(VerificationReason::DomainMismatch)
    } else if expected.contract != parsed.identity().contract.digest {
        Some(VerificationReason::ContractMismatch)
    } else if expected.binding != parsed.identity().binding {
        Some(VerificationReason::ScopeMismatch)
    } else if expected.namespace != keys.namespace {
        Some(VerificationReason::NamespaceMismatch)
    } else if parsed.key_id() != parsed.origin_signer() {
        Some(VerificationReason::KeyIdentityMismatch)
    } else {
        match keys.keys.get(parsed.key_id()) {
            None => Some(VerificationReason::UnknownKey),
            Some(key) => {
                // Exactly one named key, one strict pure-message verification.
                let signature = Signature::from_bytes(parsed.signature());
                key.verify_strict(parsed.sig_structure(), &signature)
                    .err()
                    .map(|_| VerificationReason::InvalidSignature)
            }
        }
    };
    let integrity = match reason {
        None => Integrity::Valid,
        Some(VerificationReason::UnknownKey) => Integrity::Unverifiable,
        Some(_) => Integrity::Invalid,
    };
    VerificationReport {
        payload_digest: Sha256::digest(parsed.payload()).into(),
        container_digest: Sha256::digest(parsed.container()).into(),
        parsed,
        namespace: keys.namespace.clone(),
        snapshot_revision: keys.revision,
        snapshot_digest: keys.digest,
        integrity,
        reason,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum SigningError {
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error("signing key identity mismatch")]
    KeyIdentityMismatch,
    #[error("invalid signing key")]
    InvalidKey,
}

/// Synthetic/private canonical-writer primitive; never exported to adapters.
/// It performs no custody, authority check, ledger append, or time attestation.
pub(super) fn sign_p1(
    payload: &[u8],
    signer: &SigningKey,
    limits: CryptoLimits,
) -> Result<Vec<u8>, SigningError> {
    let key = signer.verifying_key();
    if key.is_weak() {
        return Err(SigningError::InvalidKey);
    }
    let kid = SignerKeyId::for_key(&key);
    let mut header = vec![0xa2, 1, 0x32, 4, 0x58, 72];
    header.extend_from_slice(kid.as_str().as_bytes());
    let mut container = vec![0xd2, 0x84, 0x58, 78];
    container.extend_from_slice(&header);
    container.extend_from_slice(&[0xa0, 0xf6, 0x58, 64]);
    container.extend_from_slice(&[0; 64]);
    let parsed = parse_p1(&container, payload, limits)?;
    if parsed.origin_signer() != &kid {
        return Err(SigningError::KeyIdentityMismatch);
    }
    let signature = signer.sign(parsed.sig_structure()).to_bytes();
    let start = container.len() - 64;
    container[start..].copy_from_slice(&signature);
    Ok(container)
}
