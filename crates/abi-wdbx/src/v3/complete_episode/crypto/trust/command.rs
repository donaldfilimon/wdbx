//! Private typed-command encoding, not an ingress byte decoder.
use super::*;
impl KeyRegistration {
    pub(super) fn wire(&self) -> C {
        C::Array(vec![
            text(self.id.as_str()),
            C::Bytes(self.key.to_bytes().to_vec()),
            text(&self.principal.0),
            C::Unsigned(0),
            text(&self.profile),
            C::Array(
                self.grants
                    .iter()
                    .map(|g| C::Bytes(g.bytes.clone()))
                    .collect(),
            ),
        ])
    }
}
impl TrustCommand {
    pub(super) fn canonical_bytes(
        &self,
        namespace: &OpaqueRef,
        admin: &OpaqueRef,
        expected_revision: u64,
    ) -> Result<Vec<u8>> {
        let operation = match &self.operation {
            Operation::Register(r) => {
                let C::Array(mut args) = r.wire() else {
                    return Err(TrustError::Encoding);
                };
                args.insert(0, C::Unsigned(0));
                C::Array(args)
            }
            Operation::Activate(id) => C::Array(vec![C::Unsigned(1), text(id.as_str())]),
            Operation::Retire(id) => C::Array(vec![C::Unsigned(2), text(id.as_str())]),
            Operation::Revoke(id, reason, claimed) => C::Array(vec![
                C::Unsigned(3),
                text(id.as_str()),
                C::Unsigned(match reason {
                    RevocationReason::Administration => 0,
                    RevocationReason::SuspectedCompromise => 1,
                }),
                claimed.map_or(C::Null, C::Unsigned),
            ]),
            Operation::Rotate(old, new) => {
                C::Array(vec![C::Unsigned(4), text(old.as_str()), text(new.as_str())])
            }
        };
        encode(C::Array(vec![
            text("wdbx-trust-command-prototype-v1"),
            text(&namespace.0),
            text(&self.id.0),
            text(&admin.0),
            C::Unsigned(expected_revision),
            operation,
        ]))
    }
}
