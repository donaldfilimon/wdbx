//! Cross-language *encoding* witness for the private `COSE_Sign1` prototype.
//!
//! `tools/wdbx_cose_sign1_p1.py` is an independent, standard-library-only
//! Python reimplementation of the prototype's container **encoding** and of its
//! RFC 9052 `Sig_structure` construction. This test feeds one container to
//! both sides and requires the protected header, key identifier, algorithm,
//! payload and rebuilt `Sig_structure` bytes to be identical, and the same
//! malformed containers to be refused with the same error names.
//!
//! **What this test does not claim.** The Python side does not verify
//! signatures and cannot: this environment has no Python cryptography
//! library, and this repository's design guidance forbids hand-rolling
//! cryptography in Python. The cross-language claim here is *encoding
//! agreement* — both languages read the same bytes and rebuild the same signed
//! structure — and nothing more. The Ed25519 checks in this file are
//! Rust-side checks over Rust-produced bytes; they are not evidence of a
//! cross-language verifier. The COSE profile stays unratified and
//! crate-internal, and no independent cryptographic review has happened.
//!
//! The test needs `python3` on `PATH` (override with `WDBX_PYTHON`). It fails,
//! rather than silently passing, when the interpreter is missing, because a
//! skipped cross-language check would otherwise read as a green one.

/// The witness's integration coverage.
///
/// The module name matches the test target, so
/// `cargo test -p abi-wdbx v3_cross_language_cose` selects these tests.
mod v3_cross_language_cose {
    use abi_wdbx::v3::commitment::CanonicalValue;
    use abi_wdbx::v3::episode::cose_prototype::{CoseSign1, Integrity, decode_container, verify};
    use abi_wdbx::v3::episode::{EpisodeSigner, SignerKeyId};
    use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::io::Write as _;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    /// The scope label every witness response carries.
    ///
    /// The witness is an encoding witness. It says nothing about signature
    /// validity, and it says so in every response it writes.
    const SCOPE: &str = "encoding-agreement-only";

    /// Fixed synthetic seed behind the golden container.
    const GOLDEN_SEED: u8 = 1;
    /// Fixed synthetic digest behind the golden container.
    const GOLDEN_DIGEST: [u8; 32] = [0x5a; 32];

    /// The profile's RFC 9052 signature context string.
    const SIGNATURE_CONTEXT: &str = "Signature1";
    /// The profile's fixed external additional authenticated data.
    const EXTERNAL_AAD: &[u8] = b"wdbx-cose-sign1-p1";

    /// The pinned container bytes.
    const GOLDEN_CONTAINER_HEX: &str = include_str!("golden/cose-sign1-p1/container.hex");
    /// The pinned SHA-256 over the container bytes.
    const GOLDEN_CONTAINER_SHA256: &str = include_str!("golden/cose-sign1-p1/container.sha256");
    /// The pinned protected-header bytes.
    const GOLDEN_PROTECTED_HEX: &str = include_str!("golden/cose-sign1-p1/protected.hex");
    /// The pinned payload bytes.
    const GOLDEN_PAYLOAD_HEX: &str = include_str!("golden/cose-sign1-p1/payload.hex");
    /// The pinned key-identifier bytes.
    const GOLDEN_KID_HEX: &str = include_str!("golden/cose-sign1-p1/kid.hex");
    /// The pinned algorithm identifier.
    const GOLDEN_ALG: &str = include_str!("golden/cose-sign1-p1/alg.txt");
    /// The pinned RFC 9052 `Sig_structure` bytes.
    const GOLDEN_SIG_STRUCTURE_HEX: &str = include_str!("golden/cose-sign1-p1/sig_structure.hex");
    /// The pinned SHA-256 over the `Sig_structure` bytes.
    const GOLDEN_SIG_STRUCTURE_SHA256: &str =
        include_str!("golden/cose-sign1-p1/sig_structure.sha256");

    /// The witness's answer for one container.
    #[derive(Debug)]
    enum Response {
        /// The container parsed; the fields are the encoding facts.
        Parsed {
            protected_hex: String,
            sig_structure_hex: String,
            kid: String,
            alg: i64,
            payload_hex: String,
            signature_bytes_hex: String,
        },
        /// The container was refused, with a Rust `CoseError` variant name.
        Refused(String),
    }

    fn python() -> String {
        std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".to_owned())
    }

    fn script() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tools")
            .join("wdbx_cose_sign1_p1.py")
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
    }

    fn unhex(text: &str) -> Vec<u8> {
        let trimmed = text.trim();
        assert_eq!(trimmed.len() % 2, 0, "hex must have byte pairs");
        trimmed
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                u8::from_str_radix(core::str::from_utf8(pair).expect("hex is ASCII"), 16)
                    .expect("hex byte")
            })
            .collect()
    }

    fn sha256(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn signer(seed: u8) -> EpisodeSigner {
        EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
    }

    fn resolver(keys: &[VerifyingKey]) -> impl Fn(&SignerKeyId) -> Option<VerifyingKey> + '_ {
        |id| {
            keys.iter()
                .copied()
                .find(|key| &SignerKeyId::for_key(key) == id)
        }
    }

    /// The RFC 9052 `Sig_structure` this profile signs.
    ///
    /// The prototype's own `sig_structure` is crate-internal (`pub(super)`), so
    /// an integration test cannot call it. This rebuilds the identical
    /// `CanonicalValue` with the same public canonical encoder the prototype
    /// uses, which makes the bytes identical. The equivalence is not taken on
    /// trust: `rust_side_verification_succeeds` shows that the crate's
    /// *internal* structure is the message the Ed25519 signature covers, and
    /// `the_signed_bytes_are_the_agreed_sig_structure` re-signs these bytes
    /// with the pinned key and compares the resulting signature bytes with the
    /// ones the container carries.
    fn rust_sig_structure(protected: &[u8], payload: &[u8; 32]) -> Vec<u8> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text(SIGNATURE_CONTEXT.to_owned()),
            CanonicalValue::Bytes(protected.to_vec()),
            CanonicalValue::Bytes(EXTERNAL_AAD.to_vec()),
            CanonicalValue::Bytes(payload.to_vec()),
        ])
        .canonical_bytes()
        .expect("canonical sig structure")
    }

    /// Parse one witness response line, rejecting anything but the exact shape.
    fn parse_response(line: &str) -> Result<Response, String> {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("witness line is not JSON: {error}"))?;
        let object = value
            .as_object()
            .ok_or("witness response is not a JSON object")?;
        let scope = object
            .get("scope")
            .and_then(Value::as_str)
            .ok_or("witness response lacks a scope label")?;
        if scope != SCOPE {
            return Err(format!("witness scope is {scope}, expected {SCOPE}"));
        }
        match object.get("ok").and_then(Value::as_bool) {
            Some(true) => {
                if object.len() != 8 {
                    return Err(format!("witness success carries {} fields", object.len()));
                }
                let text = |name: &str| {
                    object
                        .get(name)
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .ok_or_else(|| format!("witness success lacks {name}"))
                };
                let protected_hex = text("protected_hex")?;
                let sig_structure_hex = text("sig_structure_hex")?;
                let kid = text("kid")?;
                let payload_hex = text("payload_hex")?;
                let signature_bytes_hex = text("signature_bytes_hex")?;
                let alg = object
                    .get("alg")
                    .and_then(Value::as_i64)
                    .ok_or("witness success lacks an integer alg")?;
                for (name, value) in [
                    ("protected_hex", &protected_hex),
                    ("sig_structure_hex", &sig_structure_hex),
                    ("payload_hex", &payload_hex),
                    ("signature_bytes_hex", &signature_bytes_hex),
                ] {
                    if value.len() % 2 != 0
                        || !value
                            .bytes()
                            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                    {
                        return Err(format!("witness {name} is not lowercase hex"));
                    }
                }
                Ok(Response::Parsed {
                    protected_hex,
                    sig_structure_hex,
                    kid,
                    alg,
                    payload_hex,
                    signature_bytes_hex,
                })
            }
            Some(false) => {
                if object.len() != 3 {
                    return Err(format!("witness refusal carries {} fields", object.len()));
                }
                let error = object
                    .get("error")
                    .and_then(Value::as_str)
                    .ok_or("witness refusal lacks an error name")?;
                Ok(Response::Refused(error.to_owned()))
            }
            _ => Err("witness response lacks a boolean ok".to_owned()),
        }
    }

    /// Run the witness over a batch of containers and return its answers.
    ///
    /// A missing interpreter is an `Err`, never an empty answer: every caller
    /// turns that into a test failure, so an absent Python reads as red.
    fn run_witness(requests: &[String]) -> Result<Vec<Response>, String> {
        run_witness_with(&python(), requests)
    }

    /// Run the witness with an explicit interpreter.
    fn run_witness_with(interpreter: &str, requests: &[String]) -> Result<Vec<Response>, String> {
        run_command(Command::new(interpreter), requests)
    }

    /// One request line for the witness.
    fn request(container: &[u8]) -> String {
        format!("{{\"container_hex\":\"{}\"}}", hex(container))
    }

    /// The golden container, produced by the crate's own encoder.
    fn golden_container() -> Vec<u8> {
        CoseSign1::encode(&GOLDEN_DIGEST, &signer(GOLDEN_SEED)).expect("golden container")
    }

    #[test]
    fn the_container_bytes_are_pinned_by_the_golden() {
        let container = golden_container();

        assert_eq!(hex(&container), GOLDEN_CONTAINER_HEX.trim());
        assert_eq!(
            hex(&sha256(&container)),
            GOLDEN_CONTAINER_SHA256.trim(),
            "the pinned digest must describe the pinned bytes"
        );

        let parsed = decode_container(&container).expect("decode");
        assert_eq!(hex(parsed.protected()), GOLDEN_PROTECTED_HEX.trim());
        assert_eq!(hex(parsed.payload()), GOLDEN_PAYLOAD_HEX.trim());
        assert_eq!(hex(parsed.kid()), GOLDEN_KID_HEX.trim());
        assert_eq!(parsed.alg().to_string(), GOLDEN_ALG.trim());
    }

    #[test]
    fn python_and_rust_agree_on_every_encoding_fact() {
        let container = golden_container();
        let parsed = decode_container(&container).expect("decode");

        let requests = vec![request(&container)];
        let responses = run_witness(&requests).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        assert_eq!(responses.len(), 1, "the witness answers every request");
        let Response::Parsed {
            protected_hex,
            sig_structure_hex,
            kid,
            alg,
            payload_hex,
            signature_bytes_hex,
        } = &responses[0]
        else {
            panic!("the golden container must parse on both sides");
        };

        // The protected header, key identifier, algorithm and payload are the
        // same bytes on both sides.
        assert_eq!(protected_hex, &hex(parsed.protected()), "protected header");
        assert_eq!(kid, parsed.key_id().as_str(), "key identifier");
        assert_eq!(*alg, parsed.alg(), "algorithm");
        assert_eq!(payload_hex, &hex(parsed.payload()), "payload");
        assert_eq!(
            signature_bytes_hex,
            &hex(parsed.signature()),
            "the carried signature bytes"
        );

        // The RFC 9052 `Sig_structure` is the encoding agreement itself: the
        // Python witness rebuilds it from the container's own protected bytes,
        // and the Rust side rebuilds it with the crate's canonical encoder.
        let rust_structure = rust_sig_structure(parsed.protected(), parsed.payload());
        assert_eq!(
            sig_structure_hex,
            &hex(&rust_structure),
            "the two languages must rebuild the same Sig_structure"
        );
        assert_eq!(
            sig_structure_hex.trim(),
            GOLDEN_SIG_STRUCTURE_HEX.trim(),
            "the rebuilt Sig_structure must be the pinned one"
        );
        assert_eq!(
            hex(&sha256(&rust_structure)),
            GOLDEN_SIG_STRUCTURE_SHA256.trim(),
            "the pinned digest must describe the pinned Sig_structure"
        );
    }

    #[test]
    fn the_signed_bytes_are_the_agreed_sig_structure() {
        // Rust-side evidence, and the reason the agreement above is about the
        // *signed* bytes and not merely about a printable summary: signing the
        // bytes the **Python witness** rebuilt must reproduce the very
        // signature bytes the container carries. Ed25519 signing is
        // deterministic, so equal signature bytes mean equal signed messages.
        // This is a Rust-side check over bytes the Python side produced; it
        // says nothing about a Python-side verifier, which does not exist.
        let container = golden_container();
        let parsed = decode_container(&container).expect("decode");

        let responses = run_witness(&[request(&container)]).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        let Response::Parsed {
            sig_structure_hex, ..
        } = &responses[0]
        else {
            panic!("the golden container must parse on both sides");
        };
        let python_structure = unhex(sig_structure_hex);

        let signature = SigningKey::from_bytes(&[GOLDEN_SEED; 32])
            .sign(&python_structure)
            .to_bytes();
        assert_eq!(
            hex(&signature),
            hex(parsed.signature()),
            "the python-rebuilt Sig_structure must be the message the container signed"
        );
    }

    #[test]
    fn rust_side_verification_succeeds() {
        // RUST-SIDE CHECK ONLY. The cross-language part of this file covers
        // encoding agreement; the Ed25519 verification below happens entirely
        // in Rust, over Rust-produced bytes, and says nothing about any
        // cross-language verifier. No such verifier exists.
        let container = golden_container();
        let signer = signer(GOLDEN_SEED);
        let held = [signer.verifying_key()];

        let verification =
            verify(&container, &GOLDEN_DIGEST, resolver(&held)).expect("verification");
        assert_eq!(verification.integrity, Integrity::Valid);
        assert_eq!(verification.key_id.as_ref(), Some(signer.key_id()));
    }

    #[test]
    fn a_mutated_container_is_refused_by_the_python_parser_too() {
        // The Python parser must not be vacuously permissive: a container
        // whose shape is broken is refused on both sides, with the same error
        // name. The mutation changes the payload's declared length, so the
        // container no longer carries a 32-byte digest.
        let container = golden_container();
        let mut mutated = container.clone();
        let payload_offset = 2 + 2 + unhex(GOLDEN_PROTECTED_HEX).len() + 1;
        assert_eq!(mutated[payload_offset], 0x58, "the payload length head");
        assert_eq!(mutated[payload_offset + 1], 0x20, "the payload length byte");
        mutated[payload_offset + 1] = 0x1f;

        let rust_error = decode_container(&mutated).expect_err("rust must refuse");
        let responses = run_witness(&[request(&mutated)]).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        let Response::Refused(error) = &responses[0] else {
            panic!("the python parser must refuse the mutated container");
        };
        assert_eq!(error, &format!("{rust_error:?}"), "both sides refuse alike");
        assert_eq!(error, "InvalidPayloadLength");
    }

    #[test]
    fn a_content_mutation_keeps_the_shape_and_breaks_only_the_rust_side_check() {
        // The boundary of this witness, made explicit: flipping a payload byte
        // leaves the container's *shape* intact, so both parsers accept it —
        // the Python side has no opinion about signatures and says none. Only
        // the Rust-side Ed25519 check notices, and it is a Rust-side check.
        let container = golden_container();
        let mut mutated = container.clone();
        let payload_offset = 2 + 2 + unhex(GOLDEN_PROTECTED_HEX).len() + 1;
        mutated[payload_offset + 2] ^= 0x01;

        let parsed = decode_container(&mutated).expect("the shape is still valid");
        assert_ne!(parsed.payload(), &GOLDEN_DIGEST);

        let responses = run_witness(&[request(&mutated)]).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        let Response::Parsed { payload_hex, .. } = &responses[0] else {
            panic!("the shape is valid, so the python parser accepts it");
        };
        assert_eq!(
            payload_hex,
            &hex(parsed.payload()),
            "both sides read it alike"
        );

        let held = [signer(GOLDEN_SEED).verifying_key()];
        let verification = verify(&mutated, &GOLDEN_DIGEST, resolver(&held)).expect("verification");
        assert_eq!(verification.integrity, Integrity::Invalid);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Explicit malformed-container corpus.
    fn both_sides_refuse_the_same_malformed_containers() {
        let kid = unhex(GOLDEN_KID_HEX);
        let signature = unhex(&hex(&SigningKey::from_bytes(&[GOLDEN_SEED; 32])
            .sign(&rust_sig_structure(
                &unhex(GOLDEN_PROTECTED_HEX),
                &GOLDEN_DIGEST,
            ))
            .to_bytes()));
        let container = golden_container();

        // (name, container bytes, expected Rust error name)
        let mut cases: Vec<(&str, Vec<u8>, &str)> = Vec::new();

        let mut wrong_tag = container.clone();
        wrong_tag[0] = 0xd1;
        cases.push(("tag is not 18", wrong_tag, "UnexpectedTag"));

        let mut wrong_count = container.clone();
        wrong_count[1] = 0x85;
        cases.push(("five elements", wrong_count, "UnexpectedElementCount"));

        let mut not_an_array = container.clone();
        not_an_array[1] = 0xa2;
        cases.push(("not an array", not_an_array, "UnexpectedStructure"));

        let protected_offset = 2 + 2;
        let mut not_empty = container.clone();
        not_empty[protected_offset + unhex(GOLDEN_PROTECTED_HEX).len()] = 0xa1;
        cases.push((
            "unprotected header is not empty",
            not_empty,
            "UnprotectedHeaderNotEmpty",
        ));

        let mut short_payload = container.clone();
        short_payload[protected_offset + unhex(GOLDEN_PROTECTED_HEX).len() + 1 + 1] = 0x1f;
        cases.push(("payload is 31 bytes", short_payload, "InvalidPayloadLength"));

        let mut trailing = container.clone();
        trailing.push(0x00);
        cases.push(("trailing bytes", trailing, "TrailingBytes"));

        let mut truncated = container.clone();
        truncated.truncate(truncated.len() - 1);
        cases.push(("truncated signature", truncated, "Truncated"));

        cases.push((
            "container is too large",
            vec![0_u8; 513],
            "ContainerTooLarge",
        ));

        // Protected-header shapes, built by hand so the canonical encoder
        // cannot normalize them away. The labels are laid out in canonical
        // order (by encoded length, then by encoded bytes), so each case
        // exercises the rule it names and not the key-order rule.
        let mut unknown_label = vec![0xa3, 0x01, 0x32, 0x04];
        unknown_label.push(0x58);
        unknown_label.push(u8::try_from(kid.len()).expect("kid length"));
        unknown_label.extend_from_slice(&kid);
        unknown_label.extend_from_slice(&[0x05, 0xf6]);
        cases.push((
            "unknown protected label",
            frame(&unknown_label, &GOLDEN_DIGEST, &signature),
            "UnknownProtectedLabel",
        ));

        let mut critical = vec![0xa3, 0x01, 0x32, 0x02, 0x81, 0x01];
        critical.push(0x04);
        critical.push(0x58);
        critical.push(u8::try_from(kid.len()).expect("kid length"));
        critical.extend_from_slice(&kid);
        cases.push((
            "crit is present",
            frame(&critical, &GOLDEN_DIGEST, &signature),
            "CriticalHeaderUnsupported",
        ));

        let mut duplicate = vec![0xa3, 0x01, 0x32, 0x01, 0x32];
        duplicate.push(0x04);
        duplicate.push(0x58);
        duplicate.push(u8::try_from(kid.len()).expect("kid length"));
        duplicate.extend_from_slice(&kid);
        cases.push((
            "duplicate protected key",
            frame(&duplicate, &GOLDEN_DIGEST, &signature),
            "MalformedProtectedHeader",
        ));

        let mut wrong_order = vec![0xa2, 0x04];
        wrong_order.push(0x58);
        wrong_order.push(u8::try_from(kid.len()).expect("kid length"));
        wrong_order.extend_from_slice(&kid);
        wrong_order.extend_from_slice(&[0x01, 0x32]);
        cases.push((
            "protected keys out of canonical order",
            frame(&wrong_order, &GOLDEN_DIGEST, &signature),
            "MalformedProtectedHeader",
        ));

        let mut non_minimal = vec![0xa2, 0x01, 0x38, 0x12, 0x04];
        non_minimal.push(0x58);
        non_minimal.push(u8::try_from(kid.len()).expect("kid length"));
        non_minimal.extend_from_slice(&kid);
        cases.push((
            "non-minimal integer in the protected header",
            frame(&non_minimal, &GOLDEN_DIGEST, &signature),
            "MalformedProtectedHeader",
        ));

        let mut wrong_algorithm = vec![0xa2, 0x01, 0x27];
        wrong_algorithm.push(0x04);
        wrong_algorithm.push(0x58);
        wrong_algorithm.push(u8::try_from(kid.len()).expect("kid length"));
        wrong_algorithm.extend_from_slice(&kid);
        cases.push((
            "algorithm is not Ed25519",
            frame(&wrong_algorithm, &GOLDEN_DIGEST, &signature),
            "UnsupportedAlgorithm",
        ));

        let mut short_kid = vec![0xa2, 0x01, 0x32, 0x04];
        short_kid.push(0x58);
        short_kid.push(u8::try_from(kid.len() - 1).expect("kid length"));
        short_kid.extend_from_slice(&kid[..kid.len() - 1]);
        cases.push((
            "key identifier is 71 bytes",
            frame(&short_kid, &GOLDEN_DIGEST, &signature),
            "InvalidKeyId",
        ));

        let oversized = vec![0_u8; 129];
        cases.push((
            "protected header is too large",
            frame(&oversized, &GOLDEN_DIGEST, &signature),
            "ProtectedHeaderTooLarge",
        ));

        let requests: Vec<String> = cases.iter().map(|(_, bytes, _)| request(bytes)).collect();
        let responses = run_witness(&requests).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        assert_eq!(
            responses.len(),
            cases.len(),
            "the witness answers every case"
        );

        for ((name, bytes, expected), response) in cases.iter().zip(responses) {
            let rust_error = decode_container(bytes).expect_err("rust must refuse");
            assert_eq!(
                format!("{rust_error:?}"),
                *expected,
                "{name}: the rust error name changed"
            );
            let Response::Refused(error) = response else {
                panic!("{name}: the python parser accepted a container rust refused");
            };
            assert_eq!(error.as_str(), *expected, "{name}: the error names differ");
        }

        // A well-formed container still passes, so the corpus is not simply
        // refusing everything.
        let responses = run_witness(&[request(&container)]).unwrap_or_else(|error| {
            panic!("the encoding witness must run: {error}");
        });
        assert!(matches!(responses[0], Response::Parsed { .. }));
    }

    #[test]
    fn the_witness_verifies_the_golden_vectors_standalone() {
        let status = Command::new(python())
            .arg(script())
            .arg("verify-goldens")
            .status()
            .unwrap_or_else(|error| panic!("cannot start `{}`: {error}", python()));
        assert!(status.success(), "verify-goldens exited with {status}");
    }

    #[test]
    fn a_missing_witness_interpreter_is_a_hard_failure_not_a_skip() {
        // The runner must fail loudly when the interpreter does not exist, so
        // a missing Python reads as red rather than as a silently green test.
        let error = run_witness_with("/nonexistent/wdbx-python-witness", &[])
            .expect_err("a missing interpreter must be a hard failure, never an empty answer");
        assert!(
            error.contains("WDBX_PYTHON"),
            "the failure must name the setting: {error}"
        );

        // The same holds when the interpreter is only reachable through PATH.
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
        let mut command = Command::new("python3");
        command.env("PATH", &directory);
        let error = run_command(command, &[])
            .expect_err("an interpreter that is not on PATH must be a hard failure");
        assert!(
            error.contains("WDBX_PYTHON"),
            "the failure must name the setting: {error}"
        );
    }

    #[test]
    fn the_witness_runner_fails_loudly_without_an_interpreter() {
        // Runs as a child of
        // `a_missing_witness_interpreter_is_a_hard_failure_not_a_skip`, with
        // `WDBX_PYTHON` pointing at an interpreter that does not exist. When
        // the environment does not do that, this test asserts the opposite,
        // so the child case is never vacuous.
        let missing = python().contains("nonexistent");
        let result = run_witness(&[]);
        if missing {
            let error = result.expect_err("a missing interpreter must be a hard failure");
            assert!(
                error.contains("WDBX_PYTHON"),
                "the failure must name the setting: {error}"
            );
        } else {
            assert!(
                result.is_ok(),
                "the runner must work when the interpreter exists"
            );
        }
    }

    /// Run the witness with an explicit command, used by the PATH-stripping
    /// case above.
    fn run_command(mut command: Command, requests: &[String]) -> Result<Vec<Response>, String> {
        command
            .arg(script())
            .arg("parse-json-lines")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = command.spawn().map_err(|error| {
            format!(
                "cannot start the cross-language encoding witness \
                 (set WDBX_PYTHON to a Python 3 interpreter): {error}"
            )
        })?;
        let mut stdin = child.stdin.take().ok_or("witness lost stdin")?;
        let output = std::thread::scope(|scope| {
            let writer = scope.spawn(move || {
                for line in requests {
                    writeln!(stdin, "{line}")?;
                }
                Ok::<(), std::io::Error>(())
            });
            let output = child.wait_with_output();
            writer
                .join()
                .expect("witness stdin writer exits")
                .expect("write requests to the witness");
            output.expect("witness exits")
        });
        if !output.status.success() {
            return Err(format!("witness exited with {}", output.status));
        }
        let stdout = String::from_utf8(output.stdout)
            .map_err(|_| "witness stdout is not UTF-8".to_owned())?;
        let responses = stdout
            .lines()
            .map(parse_response)
            .collect::<Result<Vec<_>, _>>()?;
        if responses.len() != requests.len() {
            return Err(format!(
                "witness emitted {} responses for {} requests",
                responses.len(),
                requests.len()
            ));
        }
        Ok(responses)
    }

    /// Frame one container the way the profile frames it, by hand.
    fn frame(protected: &[u8], payload: &[u8], signature: &[u8]) -> Vec<u8> {
        let mut container = vec![0xd2, 0x84];
        push_byte_string(&mut container, protected);
        container.push(0xa0);
        push_byte_string(&mut container, payload);
        push_byte_string(&mut container, signature);
        container
    }

    /// Append one definite-length byte string in its shortest form.
    fn push_byte_string(output: &mut Vec<u8>, bytes: &[u8]) {
        let length = bytes.len();
        if length <= 23 {
            output.push(0x40 | u8::try_from(length).expect("short length"));
        } else if length <= 0xff {
            output.push(0x58);
            output.push(u8::try_from(length).expect("byte length"));
        } else {
            output.push(0x59);
            output.extend_from_slice(
                &u16::try_from(length)
                    .expect("two-byte length")
                    .to_be_bytes(),
            );
        }
        output.extend_from_slice(bytes);
    }
}
