//! Private P1 parsing and strict signature mathematics; unratified.
//! Synthetic local trust administration is separate from signature mathematics.
//! Admission is a bounded synthetic memory model, never a real append path.
//! No credential authentication, persistence or production qualification.
//! Original input bytes are retained; byte access is deliberate disclosure.
#![allow(dead_code)]
mod admission;
mod profile;
#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "p1-crypto-witness"))]
mod python_witness_tests;
mod verify;
#[cfg(test)]
mod verify_tests;

mod trust;

#[cfg(test)]
#[test]
fn trust_snapshot_is_readable_from_crypto_siblings() {
    let state = trust::TrustState::new("synthetic-local").unwrap();
    let snapshot: trust::TrustSnapshot = state.snapshot().unwrap();
    let signing = verify_tests::key(1);
    let id = crate::v3::episode::SignerKeyId::for_key(&signing.verifying_key());
    let payload = verify_tests::payload(&signing);
    let expected = verify_tests::expected(&payload);
    assert_eq!(snapshot.namespace(), &expected.namespace);
    assert_eq!(snapshot.revision(), 0);
    assert!(snapshot.principal(&id).is_none());
    assert!(snapshot.status(&id).is_none());
    assert!(snapshot.revocation(&id).is_none());
    assert!(!snapshot.permits(&id, &expected.namespace, &expected.binding));
    assert_ne!(
        snapshot.digest(),
        snapshot.verification_keys().unwrap().digest()
    );
}
