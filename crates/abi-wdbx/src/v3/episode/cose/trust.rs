//! In-memory trust list with historical verification.
//!
//! The trust list answers a question the mathematics cannot: not *did this
//! signature verify*, but *may this key be trusted to sign now*. Those are
//! different questions with different answers over time, so the list keeps
//! them apart.
//!
//! **A key's status never rewrites history.** A retired or revoked key still
//! resolves, so a container signed while the key was in service can still be
//! checked mathematically. What a retired or revoked key loses is the right
//! to authorize anything new: [`TrustList::admits`] is true only for an
//! active key.
//!
//! Revocation is terminal. A revoked key never becomes active again, and no
//! transition leaves the revoked state.
//!
//! The list lives in memory and is not persisted, replicated, or shared. It is
//! a prototype of the *shape* of a trust decision, not a custody system: it
//! holds no secret material, survives no restart, and is not a substitute for
//! ABI ratification, a key ceremony, or an audit trail.

use crate::v3::episode::signing::SignerKeyId;
use ed25519_dalek::VerifyingKey;
use std::collections::BTreeMap;
use thiserror::Error;

/// The lifecycle state of one trusted key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyStatus {
    /// Registered, but not yet admitted to sign.
    Pending,
    /// Admitted to sign and to authorize new admissions.
    Active,
    /// No longer admitted to sign, but still resolvable for the historical
    /// verification of what it signed while it was active.
    Retired,
    /// Permanently withdrawn. Terminal: no transition leaves this state.
    Revoked,
}

/// Content-free failures from trust-list transitions.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum TrustError {
    /// The key is not registered with this list.
    #[error("trust key is not registered")]
    UnknownKey,
    /// The key identifier is already registered.
    #[error("trust key is already registered")]
    AlreadyRegistered,
    /// The identifier does not derive from the supplied verifying key.
    #[error("trust key identifier does not match verifying key")]
    KeyIdMismatch,
    /// The requested transition is not permitted from the current state.
    #[error("trust transition is not permitted")]
    InvalidTransition,
    /// The key is revoked; revocation is terminal.
    #[error("trust key is revoked")]
    Revoked,
    /// The revision counter cannot advance further.
    #[error("trust revision counter is exhausted")]
    RevisionExhausted,
}

/// One registered key and its current state.
#[derive(Clone, Debug)]
struct TrustEntry {
    key: VerifyingKey,
    status: KeyStatus,
    since: u64,
}

/// An in-memory trust list with a monotonic revision counter.
#[derive(Clone, Debug, Default)]
pub struct TrustList {
    keys: BTreeMap<SignerKeyId, TrustEntry>,
    revision: u64,
}

impl TrustList {
    /// An empty trust list at revision zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current revision.
    ///
    /// The revision advances by one for every accepted transition, so two
    /// readers that observe the same revision have observed the same history.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The number of registered keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no key is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The state of one key, if it is registered.
    #[must_use]
    pub fn status(&self, key_id: &SignerKeyId) -> Option<KeyStatus> {
        self.keys.get(key_id).map(|entry| entry.status)
    }

    /// Resolve a key for mathematical verification.
    ///
    /// Every registered key resolves, whatever its status. That is the point:
    /// a retired or revoked key must still be able to prove what it signed
    /// while it was in service.
    #[must_use]
    pub fn resolve(&self, key_id: &SignerKeyId) -> Option<VerifyingKey> {
        self.keys.get(key_id).map(|entry| entry.key)
    }

    /// Whether the key may authorize a new admission.
    ///
    /// Only an active key may. A pending, retired, or revoked key never
    /// authorizes anything new, however convincingly it signed.
    #[must_use]
    pub fn admits(&self, key_id: &SignerKeyId) -> bool {
        self.keys
            .get(key_id)
            .is_some_and(|entry| entry.status == KeyStatus::Active)
    }

    /// Register a key in the pending state.
    ///
    /// Registration admits nothing: the key must be activated before it can
    /// authorize anything.
    pub fn register(&mut self, key_id: &SignerKeyId, key: VerifyingKey) -> Result<(), TrustError> {
        if self.keys.contains_key(key_id) {
            return Err(TrustError::AlreadyRegistered);
        }
        if SignerKeyId::for_key(&key) != *key_id {
            return Err(TrustError::KeyIdMismatch);
        }
        let revision = self.next_revision()?;
        self.keys.insert(
            key_id.clone(),
            TrustEntry {
                key,
                status: KeyStatus::Pending,
                since: revision,
            },
        );
        self.revision = revision;
        Ok(())
    }

    /// Move a pending key into service.
    pub fn activate(&mut self, key_id: &SignerKeyId) -> Result<(), TrustError> {
        self.transition(key_id, KeyStatus::Active, |current| {
            matches!(current, KeyStatus::Pending)
        })
    }

    /// Take a key out of service without withdrawing its history.
    ///
    /// The key keeps resolving, so what it signed while it was active can
    /// still be verified; it simply authorizes nothing new.
    pub fn retire(&mut self, key_id: &SignerKeyId) -> Result<(), TrustError> {
        self.transition(key_id, KeyStatus::Retired, |current| {
            matches!(current, KeyStatus::Pending | KeyStatus::Active)
        })
    }

    /// Withdraw a key permanently.
    ///
    /// The key still resolves, so its history remains verifiable, but it is
    /// terminal: it can never be activated or retired again.
    pub fn revoke(&mut self, key_id: &SignerKeyId) -> Result<(), TrustError> {
        self.transition(key_id, KeyStatus::Revoked, |current| {
            !matches!(current, KeyStatus::Revoked)
        })
    }

    /// Replace an in-service key with a new one.
    ///
    /// The new key is registered and activated in the same revision, and the
    /// old key is retired in that revision. The old key keeps resolving, so
    /// what it signed stays verifiable.
    pub fn rotate(
        &mut self,
        old: &SignerKeyId,
        new_id: &SignerKeyId,
        new_key: VerifyingKey,
    ) -> Result<(), TrustError> {
        let current = self.keys.get(old).ok_or(TrustError::UnknownKey)?.status;
        if current == KeyStatus::Revoked {
            return Err(TrustError::Revoked);
        }
        if current != KeyStatus::Active {
            return Err(TrustError::InvalidTransition);
        }
        if self.keys.contains_key(new_id) {
            return Err(TrustError::AlreadyRegistered);
        }
        if SignerKeyId::for_key(&new_key) != *new_id {
            return Err(TrustError::KeyIdMismatch);
        }
        let revision = self.next_revision()?;
        if let Some(entry) = self.keys.get_mut(old) {
            entry.status = KeyStatus::Retired;
            entry.since = revision;
        }
        self.keys.insert(
            new_id.clone(),
            TrustEntry {
                key: new_key,
                status: KeyStatus::Active,
                since: revision,
            },
        );
        self.revision = revision;
        Ok(())
    }

    /// Apply one state transition.
    fn transition(
        &mut self,
        key_id: &SignerKeyId,
        next: KeyStatus,
        allowed: impl Fn(KeyStatus) -> bool,
    ) -> Result<(), TrustError> {
        let current = self.keys.get(key_id).ok_or(TrustError::UnknownKey)?.status;
        if current == KeyStatus::Revoked {
            return Err(TrustError::Revoked);
        }
        if !allowed(current) {
            return Err(TrustError::InvalidTransition);
        }
        let revision = self.next_revision()?;
        let entry = self.keys.get_mut(key_id).ok_or(TrustError::UnknownKey)?;
        entry.status = next;
        entry.since = revision;
        self.revision = revision;
        Ok(())
    }

    /// The revision that the next accepted transition will carry.
    fn next_revision(&self) -> Result<u64, TrustError> {
        self.revision
            .checked_add(1)
            .ok_or(TrustError::RevisionExhausted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn key_id(seed: u8) -> SignerKeyId {
        SignerKeyId::for_key(&SigningKey::from_bytes(&[seed; 32]).verifying_key())
    }

    fn verifying_key(seed: u8) -> VerifyingKey {
        SigningKey::from_bytes(&[seed; 32]).verifying_key()
    }

    #[test]
    fn mismatched_registration_does_not_change_the_list() {
        let mut list = TrustList::new();
        assert_eq!(
            list.register(&key_id(1), verifying_key(2)),
            Err(TrustError::KeyIdMismatch)
        );
        assert_eq!(list.revision(), 0);
        assert!(list.is_empty());
        assert_eq!(list.resolve(&key_id(1)), None);
    }

    #[test]
    fn rotation_requires_an_active_old_key_without_changing_history() {
        for status in [KeyStatus::Pending, KeyStatus::Retired, KeyStatus::Revoked] {
            let mut list = TrustList::new();
            let old = key_id(1);
            let new = key_id(2);
            list.register(&old, verifying_key(1)).unwrap();
            match status {
                KeyStatus::Retired => list.retire(&old).unwrap(),
                KeyStatus::Revoked => list.revoke(&old).unwrap(),
                _ => {}
            }
            let revision = list.revision();
            let expected = if status == KeyStatus::Revoked {
                TrustError::Revoked
            } else {
                TrustError::InvalidTransition
            };
            assert_eq!(list.rotate(&old, &new, verifying_key(2)), Err(expected));
            assert_eq!(list.revision(), revision);
            assert_eq!(list.status(&old), Some(status));
            assert_eq!(list.resolve(&old), Some(verifying_key(1)));
            assert_eq!(list.status(&new), None);
        }
    }

    #[test]
    fn mismatched_rotation_preserves_the_active_old_key() {
        let mut list = TrustList::new();
        let old = key_id(1);
        list.register(&old, verifying_key(1)).unwrap();
        list.activate(&old).unwrap();
        let revision = list.revision();
        assert_eq!(
            list.rotate(&old, &key_id(2), verifying_key(3)),
            Err(TrustError::KeyIdMismatch)
        );
        assert_eq!(list.revision(), revision);
        assert_eq!(list.status(&old), Some(KeyStatus::Active));
        assert_eq!(list.resolve(&old), Some(verifying_key(1)));
        assert!(list.admits(&old));
        assert_eq!(list.status(&key_id(2)), None);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn revocation_is_terminal_and_history_still_resolves() {
        let mut list = TrustList::new();
        let id = key_id(1);
        list.register(&id, verifying_key(1)).expect("register");
        list.activate(&id).expect("activate");
        list.revoke(&id).expect("revoke");

        assert_eq!(list.status(&id), Some(KeyStatus::Revoked));
        assert!(!list.admits(&id));
        assert!(list.resolve(&id).is_some());
        assert!(matches!(list.activate(&id), Err(TrustError::Revoked)));
        assert!(matches!(list.retire(&id), Err(TrustError::Revoked)));
        assert!(matches!(
            list.rotate(&id, &key_id(2), verifying_key(2)),
            Err(TrustError::Revoked)
        ));
    }

    #[test]
    fn the_revision_advances_by_one_per_accepted_transition() {
        let mut list = TrustList::new();
        let id = key_id(1);
        assert_eq!(list.revision(), 0);
        list.register(&id, verifying_key(1)).expect("register");
        assert_eq!(list.revision(), 1);
        list.activate(&id).expect("activate");
        assert_eq!(list.revision(), 2);
        assert!(matches!(
            list.activate(&id),
            Err(TrustError::InvalidTransition)
        ));
        assert_eq!(list.revision(), 2, "a refused transition does not advance");
    }
}
