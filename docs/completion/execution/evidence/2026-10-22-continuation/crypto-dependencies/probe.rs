//! Synthetic raw strict-backend probe, separate from P1 integration.
use ed25519_dalek::{Signature, VerifyingKey};
fn main() {
    let mut public = [0; 32]; public[0] = 1;
    let mut signature = [0; 64]; signature[0] = 1;
    let key = VerifyingKey::from_bytes(&public).expect("synthetic identity point");
    println!("identity_key_is_weak={}", key.is_weak());
    println!("identity_signature_strict_accepted={}", key.verify_strict(b"probe", &Signature::from_bytes(&signature)).is_ok());
}
