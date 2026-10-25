//! Serialized synthetic transitions: no failed command reserves its ID.
use super::*;
impl TrustState {
    fn authorize(&self, command: &TrustCommand, auth: &AdminAuthorization) -> Result<Vec<Grant>> {
        if auth.namespace != self.namespace || auth.grants.is_empty() {
            return Err(TrustError::UnauthorizedAdmin);
        }
        let mut affected = Vec::new();
        match &command.operation {
            Operation::Register(r) => affected.extend(r.grants.iter()),
            Operation::Activate(id) | Operation::Retire(id) | Operation::Revoke(id, _, _) => {
                affected.extend(
                    self.keys
                        .get(id)
                        .ok_or(TrustError::UnknownKey)?
                        .registration
                        .grants
                        .iter(),
                );
            }
            Operation::Rotate(old, new) => {
                for id in [old, new] {
                    affected.extend(
                        self.keys
                            .get(id)
                            .ok_or(TrustError::UnknownKey)?
                            .registration
                            .grants
                            .iter(),
                    );
                }
            }
        }
        if affected
            .iter()
            .any(|g| !auth.grants.iter().any(|a| a.bytes == g.bytes))
        {
            return Err(TrustError::UnauthorizedAdmin);
        }
        Ok(affected.into_iter().cloned().collect())
    }
    pub(in crate::v3::complete_episode::crypto) fn apply(
        &mut self,
        command: TrustCommand,
        auth: &AdminAuthorization,
        expected_revision: u64,
    ) -> Result<TrustCommandResult> {
        let affected = self.authorize(&command, auth)?;
        let bytes = command.canonical_bytes(&self.namespace, &auth.principal, expected_revision)?;
        if let Some(history) = self.history.get(&command.id) {
            if history
                .grants
                .iter()
                .any(|g| !auth.grants.iter().any(|a| a.bytes == g.bytes))
            {
                return Err(TrustError::UnauthorizedAdmin);
            }
            return if history.command == bytes && history.receipt.command_digest == hash(&bytes) {
                Ok(TrustCommandResult::HistoricalReplay(
                    history.receipt.clone(),
                ))
            } else {
                Err(TrustError::ReplayConflict)
            };
        }
        if expected_revision != self.revision {
            return Err(TrustError::StaleTrustRevision);
        }
        if self.history.len() >= HISTORY_CAP {
            return Err(TrustError::ResourceLimit);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(TrustError::RevisionOverflow)?;
        // Work on a bounded copy; publish only after every transition succeeds.
        let mut keys = self.keys.clone();
        match &command.operation {
            Operation::Register(registration) => {
                if keys.contains_key(&registration.id) {
                    return Err(TrustError::AlreadyRegistered);
                }
                if keys.len() >= KEY_CAP {
                    return Err(TrustError::ResourceLimit);
                }
                keys.insert(
                    registration.id.clone(),
                    Entry {
                        registration: registration.as_ref().clone(),
                        status: KeyStatus::Pending,
                        since: next,
                        revocation: None,
                    },
                );
            }
            Operation::Activate(id) => {
                transition(&mut keys, id, KeyStatus::Pending, KeyStatus::Active, next)?
            }
            Operation::Retire(id) => {
                transition(&mut keys, id, KeyStatus::Active, KeyStatus::Retired, next)?
            }
            Operation::Revoke(id, reason, claimed) => {
                let entry = keys.get_mut(id).ok_or(TrustError::UnknownKey)?;
                if entry.status == KeyStatus::Revoked {
                    return Err(TrustError::KeyRevoked);
                }
                entry.status = KeyStatus::Revoked;
                entry.since = next;
                entry.revocation = Some((*reason, *claimed));
            }
            Operation::Rotate(old, new) => {
                let a = keys.get(old).ok_or(TrustError::UnknownKey)?;
                let b = keys.get(new).ok_or(TrustError::UnknownKey)?;
                if a.status != KeyStatus::Active || b.status != KeyStatus::Pending {
                    return Err(TrustError::InvalidTransition);
                }
                if a.registration.principal != b.registration.principal
                    || a.registration.purpose != b.registration.purpose
                    || a.registration.profile != b.registration.profile
                    || a.registration.grants.iter().map(|g| &g.bytes).ne(b
                        .registration
                        .grants
                        .iter()
                        .map(|g| &g.bytes))
                {
                    return Err(TrustError::InvalidTransition);
                }
                transition(&mut keys, old, KeyStatus::Active, KeyStatus::Retired, next)?;
                transition(&mut keys, new, KeyStatus::Pending, KeyStatus::Active, next)?;
            }
        }
        let receipt = TrustReceipt {
            revision: next,
            command_digest: hash(&bytes),
        };
        self.keys = keys;
        self.revision = next;
        self.order.push(command.id.clone());
        self.history.insert(
            command.id,
            History {
                command: bytes,
                receipt: receipt.clone(),
                grants: affected,
            },
        );
        Ok(TrustCommandResult::Applied(receipt))
    }
}
fn transition(
    keys: &mut BTreeMap<SignerKeyId, Entry>,
    id: &SignerKeyId,
    from: KeyStatus,
    to: KeyStatus,
    revision: u64,
) -> Result<()> {
    let entry = keys.get_mut(id).ok_or(TrustError::UnknownKey)?;
    if entry.status == KeyStatus::Revoked {
        return Err(TrustError::KeyRevoked);
    }
    if entry.status != from {
        return Err(TrustError::InvalidTransition);
    }
    entry.status = to;
    entry.since = revision;
    Ok(())
}
