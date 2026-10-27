//! Opt-in differential evidence. Passing these tests is not strict qualification.
use super::profile::{CryptoLimits, parse_p1};
use super::verify::*;
use super::verify_tests::{expected, key, payload, snapshot};
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::{Value, json};
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").unwrap();
        output
    })
}
fn unhex(value: &Value) -> Vec<u8> {
    let text = value.as_str().expect("hex string");
    assert!(text.len().is_multiple_of(2));
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn script() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wdbx_complete_p1_crypto.py")
}
fn python() -> std::ffi::OsString {
    std::env::var_os("WDBX_CRYPTO_PYTHON").unwrap_or_else(|| "python3".into())
}
fn witness(request: &Value) -> Value {
    let mut child = Command::new(python())
        .arg(script())
        .arg("request")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("P1 witness interpreter is required; set WDBX_CRYPTO_PYTHON explicitly");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "P1 witness dependency/backend failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).expect("one witness JSON response")
}
fn python_verify(container: &[u8], bytes: &[u8]) -> Value {
    witness(&json!({"op":"verify","container_hex":hex(container),
        "payload_hex":hex(bytes),"public_key_hex":hex(&key(1).verifying_key().to_bytes())}))
}

#[test]
fn python_and_rust_independently_sign_identical_full_payload_p1() {
    for seed in [1, 2, 77] {
        let bytes = payload(&key(seed));
        let result = witness(&json!({"op":"sign","payload_hex":hex(&bytes),
            "synthetic_seed_hex":hex(&[seed;32])}));
        assert_eq!(result["ok"], true);
        let container = unhex(&result["container_hex"]);
        let parsed = parse_p1(&container, &bytes, CryptoLimits::default()).unwrap();
        assert_eq!(parsed.sig_structure(), unhex(&result["sig_structure_hex"]));
        assert_eq!(
            container,
            sign_p1(&bytes, &key(seed), CryptoLimits::default()).unwrap()
        );
        assert_eq!(
            unhex(&result["public_key_hex"]),
            key(seed).verifying_key().to_bytes()
        );
        let report = verify_p1(parsed, &expected(&bytes), &snapshot(seed));
        assert_eq!(report.integrity(), Integrity::Valid);
    }
}

#[test]
fn independent_python_verifies_rust_and_refuses_signature_mutations() {
    let bytes = payload(&key(1));
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    assert_eq!(
        python_verify(&container, &bytes),
        json!({"ok":true,"valid":true,"reason":null})
    );
    let mut altered = container.clone();
    *altered.last_mut().unwrap() ^= 1;
    let mut scalar = container.clone();
    let start = scalar.len() - 32;
    scalar[start..].fill(0xff);
    let mut r = container;
    let start = r.len() - 64;
    r[start..start + 32].fill(0);
    r[start] = 1;
    let mut noncanonical_r = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let start = noncanonical_r.len() - 64;
    noncanonical_r[start..start + 32].fill(0xff);
    for mutation in [altered, scalar, r, noncanonical_r] {
        assert_eq!(
            python_verify(&mutation, &bytes),
            json!({"ok":true,"valid":false,"reason":"InvalidSignature"})
        );
        let report = verify_p1(
            parse_p1(&mutation, &bytes, CryptoLimits::default()).unwrap(),
            &expected(&bytes),
            &snapshot(1),
        );
        assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));
    }
}

#[test]
fn malformed_profile_bytes_are_refused_in_both_languages() {
    let bytes = payload(&key(1));
    let original = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    for index in [0, 1, 2, 6, 82, 83] {
        let mut container = original.clone();
        container[index] ^= 1;
        assert!(
            parse_p1(&container, &bytes, CryptoLimits::default()).is_err(),
            "index {index}"
        );
        assert_eq!(
            python_verify(&container, &bytes)["ok"],
            false,
            "index {index}"
        );
    }
    for length in [0, 1, 4, 80, original.len() - 1] {
        assert!(parse_p1(&original[..length], &bytes, CryptoLimits::default()).is_err());
        assert_eq!(python_verify(&original[..length], &bytes)["ok"], false);
    }
}

#[test]
fn backend_characterization_preserves_known_low_order_disagreement() {
    let mut public = [0; 32];
    public[0] = 1;
    let mut signature = [0; 64];
    signature[0] = 1;
    let key = VerifyingKey::from_bytes(&public).unwrap();
    assert!(key.is_weak());
    assert!(
        key.verify_strict(b"probe", &Signature::from_bytes(&signature))
            .is_err()
    );
    let output = Command::new(python())
        .arg(script())
        .arg("backend-probe")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "missing explicit backend dependency"
    );
    let probe: Value = serde_json::from_slice(&output.stdout).unwrap();
    let backend = std::env::var("WDBX_CRYPTO_BACKEND").unwrap_or_else(|_| "pyca".to_owned());
    let accepts = match backend.as_str() {
        "pyca" => true,
        "libsodium" => false,
        _ => panic!("explicit supported witness backend required"),
    };
    assert_eq!(probe["raw_weak_key_accepted"], accepts);
    assert_eq!(probe["strict_parity_qualified"], false);
    let qualification = Command::new(python())
        .arg(script())
        .args(["backend-probe", "--qualify"])
        .output()
        .unwrap();
    assert_eq!(
        qualification.status.code(),
        Some(1),
        "a characterized difference must fail qualification"
    );
}

#[test]
fn independently_generated_ed25519ph_is_refused_as_pure_message_p1() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/completion/execution/evidence/2026-10-22-continuation/libsodium-dependencies/ph-vector.json");
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(path).expect("synthetic native Ed25519ph fixture is required"),
    )
    .unwrap();
    let bytes = unhex(&fixture["payload_hex"]);
    let container = unhex(&fixture["container_hex"]);
    assert_eq!(
        unhex(&fixture["public_key_hex"]),
        key(1).verifying_key().to_bytes()
    );
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &expected(&bytes),
        &snapshot(1),
    );
    assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));
    assert_eq!(
        python_verify(&container, &bytes),
        json!({"ok":true,"valid":false,"reason":"InvalidSignature"})
    );
}
