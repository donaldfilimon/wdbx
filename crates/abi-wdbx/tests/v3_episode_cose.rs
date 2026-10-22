//! Private `COSE_Sign1` prototype: container round trip, tamper rejection,
//! and trust-state transitions.
//!
//! This exercises a prototype, not a ratified profile. Nothing here is a
//! claim about cross-language verification, admission semantics, or key
//! custody; see the crate's `v3::episode::cose` module for what the
//! prototype does and does not claim.

/// The prototype's integration coverage.
///
/// The module name matches the test target, so
/// `cargo test -p abi-wdbx v3_episode_cose` selects these tests.
mod v3_episode_cose {
    use abi_wdbx::v3::episode::cose_prototype::{
        CoseError, CoseSign1, Integrity, KeyStatus, TrustError, TrustList, decode_container, verify,
    };
    use abi_wdbx::v3::episode::{EpisodeSigner, SignerKeyId, verify_digest};
    use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    /// Fixed synthetic seed behind the golden container.
    const GOLDEN_SEED: u8 = 1;
    /// Fixed synthetic digest behind the golden container.
    const GOLDEN_DIGEST: [u8; 32] = [0x5a; 32];
    /// The pinned container bytes.
    const GOLDEN_HEX: &str = include_str!("golden/cose-sign1-p1/container.hex");
    /// The pinned SHA-256 over the container bytes.
    const GOLDEN_SHA256: &str = include_str!("golden/cose-sign1-p1/container.sha256");

    /// A throwaway directory for one test.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("wdbx-cose-sign1-{}", Uuid::new_v4())))
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn container(&self) -> PathBuf {
            self.0.join("episode.cose")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
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

    /// Build the protected header the profile encodes, by hand.
    ///
    /// This is deliberately not the crate's encoder: a tamper test that reused
    /// the encoder would only prove the encoder agrees with itself.
    fn protected_header(alg: i64, kid: &str) -> Vec<u8> {
        let argument = u64::try_from(-1_i128 - i128::from(alg)).expect("negative algorithm");
        let mut header = vec![0xa2, 0x01];
        header.push(0x20 | u8::try_from(argument).expect("small algorithm"));
        header.push(0x04);
        push_byte_string(&mut header, kid.as_bytes());
        header
    }

    /// Build a `COSE_Sign1` container by hand.
    fn build_container(protected: &[u8], payload: &[u8; 32], signature: &[u8; 64]) -> Vec<u8> {
        let mut container = vec![0xd2, 0x84];
        push_byte_string(&mut container, protected);
        container.push(0xa0);
        push_byte_string(&mut container, payload);
        push_byte_string(&mut container, signature);
        container
    }

    /// Build the RFC 9052 `Sig_structure` by hand, with a chosen external AAD.
    fn sig_structure(protected: &[u8], aad: &[u8], payload: &[u8; 32]) -> Vec<u8> {
        let mut structure = vec![0x84];
        structure.push(0x60 | u8::try_from("Signature1".len()).expect("short context"));
        structure.extend_from_slice(b"Signature1");
        push_byte_string(&mut structure, protected);
        push_byte_string(&mut structure, aad);
        push_byte_string(&mut structure, payload);
        structure
    }

    fn push_byte_string(output: &mut Vec<u8>, bytes: &[u8]) {
        output.push(0x58);
        output.push(u8::try_from(bytes.len()).expect("short byte string"));
        output.extend_from_slice(bytes);
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
    }

    fn unhex(input: &str) -> Vec<u8> {
        let trimmed = input.trim();
        assert_eq!(trimmed.len() % 2, 0, "golden hex must have byte pairs");
        trimmed
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let pair = core::str::from_utf8(pair).expect("golden hex is ASCII");
                u8::from_str_radix(pair, 16).expect("golden hex byte")
            })
            .collect()
    }

    #[test]
    fn a_container_round_trips_and_verifies_as_valid() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");

        let parsed = decode_container(&container).expect("decode");
        assert_eq!(parsed.key_id(), signer.key_id());
        assert_eq!(parsed.alg(), -19);
        assert_eq!(parsed.payload(), &digest);
        assert_eq!(parsed.kid(), signer.key_id().as_str().as_bytes());
        assert_eq!(parsed.signature().len(), 64);

        let verification =
            verify(&container, &digest, resolver(&[signer.verifying_key()])).expect("verify");
        assert_eq!(verification.integrity, Integrity::Valid);
        assert_eq!(verification.key_id.as_ref(), Some(signer.key_id()));
    }

    #[test]
    fn the_container_is_detached_and_the_ledger_signature_is_untouched() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");
        let parsed = decode_container(&container).expect("decode");

        // The detached episode signature is still a raw signature over the bare
        // digest, and the COSE signature is a different 64 bytes over a
        // different message. Neither is the other's stand-in.
        let episode = signer.sign_digest(&digest);
        assert_ne!(parsed.signature(), &episode.signature);
        assert_eq!(
            verify_digest(&signer.verifying_key(), &digest, &episode),
            abi_wdbx::v3::episode::SignatureStatus::Valid(signer.key_id().clone())
        );
    }

    #[test]
    fn a_transplanted_key_id_does_not_verify() {
        let author = signer(1);
        let other = signer(2);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &author).expect("container");
        let parsed = decode_container(&container).expect("decode");

        // The same signature bytes under another key's identifier.
        let transplanted = build_container(
            &protected_header(-19, other.key_id().as_str()),
            &digest,
            parsed.signature(),
        );
        let verification = verify(&transplanted, &digest, resolver(&[other.verifying_key()]))
            .expect("verification");
        assert_eq!(verification.integrity, Integrity::Invalid);
        assert_eq!(verification.key_id.as_ref(), Some(other.key_id()));

        // A resolver that hands back the wrong key for the named identifier does
        // not rescue the container either: a signature is never credited to a key
        // it was not made under.
        let confused =
            verify(&transplanted, &digest, |_| Some(author.verifying_key())).expect("verification");
        assert_eq!(confused.integrity, Integrity::Invalid);
        assert_eq!(confused.key_id.as_ref(), Some(other.key_id()));
    }

    #[test]
    fn a_container_against_another_digest_is_invalid() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");

        let verification = verify(&container, &[0x11; 32], resolver(&[signer.verifying_key()]))
            .expect("verification");
        assert_eq!(verification.integrity, Integrity::Invalid);
        assert_eq!(verification.key_id.as_ref(), Some(signer.key_id()));
    }

    #[test]
    fn a_tampered_container_is_never_valid() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");
        let parsed = decode_container(&container).expect("decode");
        let held = [signer.verifying_key()];
        let keys = resolver(&held);

        // A tampered payload no longer attests the digest.
        let mut payload = digest;
        payload[0] ^= 1;
        let tampered_payload = build_container(parsed.protected(), &payload, parsed.signature());
        assert_eq!(
            verify(&tampered_payload, &digest, &keys)
                .expect("verification")
                .integrity,
            Integrity::Invalid
        );

        // A tampered algorithm is refused outright rather than reinterpreted.
        let tampered_alg = build_container(
            &protected_header(-8, signer.key_id().as_str()),
            &digest,
            parsed.signature(),
        );
        assert!(matches!(
            decode_container(&tampered_alg),
            Err(CoseError::UnsupportedAlgorithm)
        ));

        // A tampered key identifier no longer names the signing key, so the
        // container can no longer be credited to it.
        let mut broken_kid = signer.key_id().as_str().to_owned();
        broken_kid.pop();
        broken_kid.push('0');
        let tampered_kid = build_container(
            &protected_header(-19, &broken_kid),
            &digest,
            parsed.signature(),
        );
        assert_ne!(
            verify(&tampered_kid, &digest, &keys)
                .expect("verification")
                .integrity,
            Integrity::Valid
        );

        // A key identifier that is not a key identifier at all is refused.
        let malformed_kid = format!("{}0", signer.key_id().as_str());
        let malformed = build_container(
            &protected_header(-19, &malformed_kid),
            &digest,
            parsed.signature(),
        );
        assert!(matches!(
            decode_container(&malformed),
            Err(CoseError::InvalidKeyId)
        ));

        // A tampered signature does not verify.
        let mut signature = *parsed.signature();
        signature[0] ^= 1;
        let tampered_signature = build_container(parsed.protected(), &digest, &signature);
        assert_eq!(
            verify(&tampered_signature, &digest, &keys)
                .expect("verification")
                .integrity,
            Integrity::Invalid
        );

        // A signature made over a different external AAD does not verify: the
        // AAD is bound into the signed bytes.
        let structure = sig_structure(parsed.protected(), b"wdbx-cose-sign1-p2", &digest);
        let foreign = SigningKey::from_bytes(&[1_u8; 32])
            .sign(&structure)
            .to_bytes();
        let wrong_aad = build_container(parsed.protected(), &digest, &foreign);
        assert_eq!(
            verify(&wrong_aad, &digest, &keys)
                .expect("verification")
                .integrity,
            Integrity::Invalid
        );
    }

    #[test]
    fn trailing_bytes_and_oversized_containers_are_refused() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");

        let mut trailing = container.clone();
        trailing.push(0x00);
        assert!(matches!(
            decode_container(&trailing),
            Err(CoseError::TrailingBytes)
        ));

        let oversized = vec![0_u8; 513];
        assert!(matches!(
            decode_container(&oversized),
            Err(CoseError::ContainerTooLarge)
        ));
    }

    #[test]
    fn a_container_with_the_wrong_shape_is_refused() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");
        let parsed = decode_container(&container).expect("decode");

        // Not a COSE_Sign1 tag.
        let mut wrong_tag = container.clone();
        wrong_tag[0] = 0xc2;
        assert!(matches!(
            decode_container(&wrong_tag),
            Err(CoseError::UnexpectedTag)
        ));

        // Not four elements.
        let mut wrong_count = container.clone();
        wrong_count[1] = 0x83;
        assert!(matches!(
            decode_container(&wrong_count),
            Err(CoseError::UnexpectedElementCount)
        ));

        // An unprotected header that is not empty.
        let mut unprotected = container.clone();
        let offset = 2 + 2 + parsed.protected().len();
        unprotected[offset] = 0xa1;
        assert!(matches!(
            decode_container(&unprotected),
            Err(CoseError::UnprotectedHeaderNotEmpty)
        ));

        // A protected header that names a label this profile does not define.
        let mut unknown_label = protected_header(-19, signer.key_id().as_str());
        unknown_label[0] = 0xa3;
        unknown_label.push(0x05);
        unknown_label.push(0xf6);
        let unknown = build_container(&unknown_label, &digest, parsed.signature());
        assert!(matches!(
            decode_container(&unknown),
            Err(CoseError::UnknownProtectedLabel)
        ));

        // A protected header that asks for critical-header processing.
        let mut critical = protected_header(-19, signer.key_id().as_str());
        critical[0] = 0xa3;
        critical.insert(3, 0x02);
        critical.insert(4, 0x81);
        critical.insert(5, 0x01);
        let critical = build_container(&critical, &digest, parsed.signature());
        assert!(matches!(
            decode_container(&critical),
            Err(CoseError::CriticalHeaderUnsupported)
        ));
    }

    #[test]
    fn an_unresolved_key_is_unverifiable_and_not_invalid() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");

        let verification = verify(&container, &digest, |_| None).expect("verification");
        assert_eq!(verification.integrity, Integrity::Unverifiable);
        assert_eq!(verification.key_id.as_ref(), Some(signer.key_id()));
    }

    #[test]
    fn trust_transitions_keep_history_and_separate_the_two_axes() {
        let first = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &first).expect("container");
        let key_id = first.key_id().clone();
        let key = first.verifying_key();

        let mut trust = TrustList::new();
        assert!(trust.is_empty());
        assert_eq!(trust.status(&key_id), None);

        trust.register(&key_id, key).expect("register");
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Pending));
        assert!(!trust.admits(&key_id), "a pending key admits nothing");
        assert_eq!(trust.revision(), 1);

        trust.activate(&key_id).expect("activate");
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Active));
        assert!(trust.admits(&key_id));
        assert_eq!(trust.revision(), 2);

        trust.retire(&key_id).expect("retire");
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Retired));
        assert!(!trust.admits(&key_id), "a retired key admits nothing new");

        // The mathematics still hold for what the key signed while it was in
        // service: retirement does not rewrite history.
        let verification =
            verify(&container, &digest, |id| trust.resolve(id)).expect("verification");
        assert_eq!(verification.integrity, Integrity::Valid);
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Retired));

        trust.revoke(&key_id).expect("revoke");
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Revoked));
        assert!(!trust.admits(&key_id), "a revoked key admits nothing");

        // Revocation is terminal.
        assert!(matches!(trust.activate(&key_id), Err(TrustError::Revoked)));
        assert!(matches!(trust.retire(&key_id), Err(TrustError::Revoked)));
        assert!(matches!(
            trust.rotate(
                &key_id,
                &signer(2).key_id().clone(),
                signer(2).verifying_key()
            ),
            Err(TrustError::Revoked)
        ));

        // The two axes stay separate: the mathematics still verify, and the
        // trust state still says the key is revoked.
        let verification =
            verify(&container, &digest, |id| trust.resolve(id)).expect("verification");
        assert_eq!(verification.integrity, Integrity::Valid);
        assert_eq!(trust.status(&key_id), Some(KeyStatus::Revoked));
    }

    #[test]
    fn rotation_retires_the_old_key_and_activates_the_new_one() {
        let old = signer(1);
        let new = signer(2);
        let mut trust = TrustList::new();
        trust
            .register(old.key_id(), old.verifying_key())
            .expect("register");
        trust.activate(old.key_id()).expect("activate");

        trust
            .rotate(old.key_id(), new.key_id(), new.verifying_key())
            .expect("rotate");

        assert_eq!(trust.status(old.key_id()), Some(KeyStatus::Retired));
        assert_eq!(trust.status(new.key_id()), Some(KeyStatus::Active));
        assert!(trust.admits(new.key_id()));
        assert!(!trust.admits(old.key_id()));
        assert!(
            trust.resolve(old.key_id()).is_some(),
            "history still resolves"
        );
        assert_eq!(trust.revision(), 3);
    }

    #[test]
    fn a_container_stored_beside_the_record_still_verifies() {
        let signer = signer(1);
        let digest = [0x5a; 32];
        let container = CoseSign1::encode(&digest, &signer).expect("container");

        let scratch = Scratch::new();
        fs::create_dir_all(scratch.path()).expect("scratch directory");
        fs::write(scratch.container(), &container).expect("detached container");

        let stored = fs::read(scratch.container()).expect("detached container");
        let verification =
            verify(&stored, &digest, resolver(&[signer.verifying_key()])).expect("verification");
        assert_eq!(verification.integrity, Integrity::Valid);
    }

    #[test]
    fn the_golden_container_bytes_are_pinned() {
        let signer = signer(GOLDEN_SEED);
        let container = CoseSign1::encode(&GOLDEN_DIGEST, &signer).expect("container");

        assert_eq!(hex(&container), GOLDEN_HEX.trim());
        assert_eq!(
            hex(&sha256(&container)),
            GOLDEN_SHA256.trim(),
            "the pinned digest must describe the pinned bytes"
        );
        assert_eq!(unhex(GOLDEN_HEX), container);

        // The golden container is a real container, not a blob of bytes.
        let parsed = decode_container(&container).expect("decode");
        assert_eq!(parsed.key_id(), signer.key_id());
        assert_eq!(parsed.payload(), &GOLDEN_DIGEST);
        let verification = verify(
            &container,
            &GOLDEN_DIGEST,
            resolver(&[signer.verifying_key()]),
        )
        .expect("verification");
        assert_eq!(verification.integrity, Integrity::Valid);
    }

    fn sha256(bytes: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes).into()
    }
}
