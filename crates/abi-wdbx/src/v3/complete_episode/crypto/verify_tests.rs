//! Private P1 verification: mathematical integrity is never admission authority.
use super::super::primitives::*;
use super::super::{COMPLETE_EPISODE_DOMAIN, CompleteEpisode, DecodedCompleteEpisode};
use super::profile::{CryptoLimits, parse_p1};
use super::verify::*;
use crate::v3::commitment::CanonicalDecodeLimits;
use crate::v3::episode::SignerKeyId;
use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};

pub(super) fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub(super) fn payload(signing_key: &SigningKey) -> Vec<u8> {
    let hex = include_str!("../tests/provisional-minimal.hex").trim();
    let bytes: Vec<_> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let (mut input, _) =
        DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
            .unwrap()
            .into_parts();
    input.origin.signer = SignerKeyId::for_key(&signing_key.verifying_key());
    CompleteEpisode::try_new(input)
        .unwrap()
        .canonical_bytes()
        .unwrap()
}

pub(super) fn expected(bytes: &[u8]) -> ExpectedContext {
    let decoded =
        DecodedCompleteEpisode::from_canonical_bytes(bytes, CanonicalDecodeLimits::default())
            .unwrap();
    ExpectedContext {
        profile: super::profile::PROFILE_ID,
        domain: COMPLETE_EPISODE_DOMAIN,
        version: 1,
        contract: decoded.input().identity.contract.digest,
        binding: decoded.input().identity.binding.clone(),
        namespace: OpaqueRef::new("synthetic-local").unwrap(),
    }
}

pub(super) fn snapshot(seed: u8) -> KeySnapshot {
    KeySnapshot::new(
        OpaqueRef::new("synthetic-local").unwrap(),
        3,
        vec![key(seed).verifying_key()],
    )
    .unwrap()
}

#[test]
fn strict_full_payload_signature_retains_originals_and_bound_report() {
    let bytes = payload(&key(1));
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let keys = snapshot(1);
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &expected(&bytes),
        &keys,
    );
    assert_eq!(report.integrity(), Integrity::Valid);
    assert_eq!(report.reason(), None);
    assert_eq!(report.payload(), bytes);
    assert_eq!(report.container(), container);
    assert_eq!(
        report.payload_digest(),
        <[u8; 32]>::from(Sha256::digest(&bytes))
    );
    assert_eq!(
        report.container_digest(),
        <[u8; 32]>::from(Sha256::digest(&container))
    );
    assert_eq!(report.snapshot_revision(), 3);
    assert_eq!(report.snapshot_digest(), keys.digest());
    assert_eq!(report.namespace(), &expected(&bytes).namespace);
    assert_eq!(
        report.identity(),
        &parse_p1(&container, &bytes, CryptoLimits::default())
            .unwrap()
            .identity()
            .clone()
    );
    assert_eq!(
        report.signer(),
        &SignerKeyId::for_key(&key(1).verifying_key())
    );
    let typed =
        DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
            .unwrap();
    assert_eq!(report.origin_actor(), &typed.input().origin.actor);
    assert_eq!(report.authority(), &typed.input().authority);
    assert_eq!(format!("{report:?}"), "VerificationReport { .. }");
}

#[test]
fn unresolved_key_is_unverifiable_and_never_rescued_by_other_keys() {
    let bytes = payload(&key(1));
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &expected(&bytes),
        &snapshot(2),
    );
    assert_eq!(report.integrity(), Integrity::Unverifiable);
    assert_eq!(report.reason(), Some(VerificationReason::UnknownKey));
    assert_eq!(report.payload(), bytes);
}

#[test]
fn changed_signature_and_digest_signature_are_not_valid_p1() {
    let bytes = payload(&key(1));
    let original = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let mut altered = original.clone();
    *altered.last_mut().unwrap() ^= 1;
    let mut legacy = original;
    let signature = key(1).sign(&Sha256::digest(&bytes)).to_bytes();
    let start = legacy.len() - 64;
    legacy[start..].copy_from_slice(&signature);
    for container in [altered, legacy] {
        let report = verify_p1(
            parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
            &expected(&bytes),
            &snapshot(1),
        );
        assert_eq!(report.integrity(), Integrity::Invalid);
        assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));
    }
}

#[test]
fn trusted_context_mismatches_are_invalid_before_authority_can_be_inferred() {
    let bytes = payload(&key(1));
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    for reason in [
        VerificationReason::UnsupportedProfile,
        VerificationReason::DomainMismatch,
        VerificationReason::ContractMismatch,
        VerificationReason::ScopeMismatch,
        VerificationReason::NamespaceMismatch,
    ] {
        let mut context = expected(&bytes);
        match reason {
            VerificationReason::UnsupportedProfile => context.profile = "other",
            VerificationReason::DomainMismatch => context.domain = "other",
            VerificationReason::ContractMismatch => context.contract = Digest32::new([99; 32]),
            VerificationReason::ScopeMismatch => {
                context.binding.tenant = TenantRef::new("other").unwrap()
            }
            VerificationReason::NamespaceMismatch => {
                context.namespace = OpaqueRef::new("foreign").unwrap()
            }
            _ => unreachable!(),
        }
        let report = verify_p1(
            parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
            &context,
            &snapshot(1),
        );
        assert_eq!(report.integrity(), Integrity::Invalid);
        assert_eq!(report.reason(), Some(reason));
    }
    let mut context = expected(&bytes);
    context.version = 2;
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &context,
        &snapshot(1),
    );
    assert_eq!(report.reason(), Some(VerificationReason::DomainMismatch));
}

#[test]
fn signing_refuses_origin_mismatch_and_verification_rejects_signed_substitution() {
    let bytes = payload(&key(1));
    assert!(sign_p1(&bytes, &key(2), CryptoLimits::default()).is_err());
    let mut container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    // Change payload origin to another key while preserving a structurally valid envelope.
    let different = payload(&key(2));
    let parsed = parse_p1(&container, &different, CryptoLimits::default()).unwrap();
    let signature = key(1).sign(parsed.sig_structure()).to_bytes();
    let start = container.len() - 64;
    container[start..].copy_from_slice(&signature);
    let report = verify_p1(
        parse_p1(&container, &different, CryptoLimits::default()).unwrap(),
        &expected(&different),
        &snapshot(1),
    );
    assert_eq!(
        report.reason(),
        Some(VerificationReason::KeyIdentityMismatch)
    );
}

#[test]
fn snapshots_reject_weak_duplicate_and_over_limit_keys_and_pin_order() {
    let namespace = OpaqueRef::new("synthetic-local").unwrap();
    let weak = VerifyingKey::from_bytes(&[0; 32]).unwrap();
    assert!(KeySnapshot::new(namespace.clone(), 0, vec![weak]).is_err());
    assert!(KeySnapshot::new(namespace.clone(), 0, vec![key(1).verifying_key(); 2]).is_err());
    assert!(KeySnapshot::new(namespace.clone(), 0, vec![key(1).verifying_key(); 1025]).is_err());
    let first = KeySnapshot::new(
        namespace.clone(),
        0,
        vec![key(1).verifying_key(), key(2).verifying_key()],
    )
    .unwrap();
    let second = KeySnapshot::new(
        namespace.clone(),
        0,
        vec![key(2).verifying_key(), key(1).verifying_key()],
    )
    .unwrap();
    assert_eq!(first.digest(), second.digest());
    let revised = KeySnapshot::new(
        namespace,
        1,
        vec![key(1).verifying_key(), key(2).verifying_key()],
    )
    .unwrap();
    assert_ne!(first.digest(), revised.digest());
}

#[test]
fn strict_verification_refuses_noncanonical_scalar_and_low_order_r() {
    let bytes = payload(&key(1));
    let original = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let start = original.len() - 64;
    let mut scalar = original.clone();
    scalar[start + 32..].fill(0xff);
    let mut low_order_r = original;
    low_order_r[start..start + 32].fill(0);
    low_order_r[start] = 1;
    for container in [scalar, low_order_r] {
        let report = verify_p1(
            parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
            &expected(&bytes),
            &snapshot(1),
        );
        assert_eq!(report.integrity(), Integrity::Invalid);
        assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));
    }
}

#[test]
fn altered_aad_and_changed_canonical_payload_are_rejected_without_reencoding() {
    let bytes = payload(&key(1));
    let mut container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let parsed = parse_p1(&container, &bytes, CryptoLimits::default()).unwrap();
    let mut wrong_structure = parsed.sig_structure().to_vec();
    let aad = super::profile::PROFILE_ID.as_bytes();
    let start = wrong_structure
        .windows(aad.len())
        .position(|w| w == aad)
        .unwrap();
    wrong_structure[start] ^= 1;
    let signature = key(1).sign(&wrong_structure).to_bytes();
    let start = container.len() - 64;
    container[start..].copy_from_slice(&signature);
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &expected(&bytes),
        &snapshot(1),
    );
    assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));

    let (mut input, _) =
        DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
            .unwrap()
            .into_parts();
    input.time.received_at = UtcMillis::new(9_999);
    let changed = CompleteEpisode::try_new(input)
        .unwrap()
        .canonical_bytes()
        .unwrap();
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let parsed = parse_p1(&container, &changed, CryptoLimits::default()).unwrap();
    let report = verify_p1(parsed, &expected(&changed), &snapshot(1));
    assert_eq!(report.reason(), Some(VerificationReason::InvalidSignature));
    assert_eq!(report.payload(), changed);
}
