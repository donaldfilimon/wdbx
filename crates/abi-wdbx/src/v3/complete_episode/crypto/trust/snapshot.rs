//! Immutable trust metadata; mathematical key snapshots grant no authority.
use super::*;
#[derive(Clone)]
pub(in crate::v3::complete_episode::crypto) struct TrustSnapshot {
    namespace: OpaqueRef,
    revision: u64,
    digest: [u8; 32],
    keys: BTreeMap<SignerKeyId, Entry>,
}
redacted!(TrustSnapshot);
impl TrustState {
    pub(in crate::v3::complete_episode::crypto) fn snapshot(&self) -> Result<super::TrustSnapshot> {
        let keys = self
            .keys
            .values()
            .map(|e| {
                C::Array(vec![
                    e.registration.wire(),
                    C::Unsigned(match e.status {
                        KeyStatus::Pending => 0,
                        KeyStatus::Active => 1,
                        KeyStatus::Retired => 2,
                        KeyStatus::Revoked => 3,
                    }),
                    C::Unsigned(e.since),
                    e.revocation.map_or(C::Null, |(reason, time)| {
                        C::Array(vec![
                            C::Unsigned(match reason {
                                RevocationReason::Administration => 0,
                                RevocationReason::SuspectedCompromise => 1,
                            }),
                            time.map_or(C::Null, C::Unsigned),
                        ])
                    }),
                ])
            })
            .collect();
        let history = self
            .order
            .iter()
            .map(|id| C::Bytes(self.history[id].receipt.command_digest.to_vec()))
            .collect();
        let bytes = encode(C::Array(vec![
            text("wdbx-p1-trust-snapshot-prototype-v1"),
            text(&self.namespace.0),
            C::Unsigned(self.revision),
            C::Array(keys),
            C::Array(history),
        ]))?;
        Ok(TrustSnapshot {
            namespace: self.namespace.clone(),
            revision: self.revision,
            digest: hash(&bytes),
            keys: self.keys.clone(),
        })
    }
}
impl TrustSnapshot {
    pub(in crate::v3::complete_episode::crypto) fn namespace(&self) -> &OpaqueRef {
        &self.namespace
    }
    pub(in crate::v3::complete_episode::crypto) fn principal(
        &self,
        id: &SignerKeyId,
    ) -> Option<&OpaqueRef> {
        self.keys.get(id).map(|e| &e.registration.principal)
    }
    pub(in crate::v3::complete_episode::crypto) fn revocation(
        &self,
        id: &SignerKeyId,
    ) -> Option<(RevocationReason, Option<u64>)> {
        self.keys.get(id).and_then(|e| e.revocation)
    }
    pub(in crate::v3::complete_episode::crypto) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(in crate::v3::complete_episode::crypto) fn revision(&self) -> u64 {
        self.revision
    }
    pub(in crate::v3::complete_episode::crypto) fn status(
        &self,
        id: &SignerKeyId,
    ) -> Option<KeyStatus> {
        self.keys.get(id).map(|e| e.status)
    }
    pub(in crate::v3::complete_episode::crypto) fn permits(
        &self,
        id: &SignerKeyId,
        principal: &OpaqueRef,
        binding: &ScopeBinding,
    ) -> bool {
        self.keys.get(id).is_some_and(|entry| {
            entry.status == KeyStatus::Active
                && entry.registration.principal == *principal
                && entry
                    .registration
                    .grants
                    .iter()
                    .any(|g| &g.binding == binding)
        })
    }
    pub(in crate::v3::complete_episode::crypto) fn verification_keys(
        &self,
    ) -> std::result::Result<KeySnapshot, SnapshotError> {
        KeySnapshot::new(
            self.namespace.clone(),
            self.revision,
            self.keys.values().map(|e| e.registration.key).collect(),
        )
    }
}
