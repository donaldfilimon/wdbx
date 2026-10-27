use super::*;
use crate::v3::complete_episode::{GuildRef, Scope, ScopeBinding, TenantRef};
use ed25519_dalek::SigningKey;
fn scope(name: &str) -> ScopeBinding {
    ScopeBinding::new((
        TenantRef::new("synthetic-tenant").unwrap(),
        Scope::Guild(GuildRef::new(name).unwrap()),
    ))
}
fn auth() -> AdminAuthorization {
    AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![scope("a")],
    )
    .unwrap()
}
fn registration(seed: u8) -> KeyRegistration {
    let key = SigningKey::from_bytes(&[seed; 32]).verifying_key();
    KeyRegistration::new(
        key.as_bytes(),
        SignerKeyId::for_key(&key),
        "writer",
        KeyPurpose::P1CanonicalWriter,
        PROFILE_ID,
        vec![scope("a")],
    )
    .unwrap()
}
fn command(id: &str, operation: Operation) -> TrustCommand {
    TrustCommand::new(id, operation).unwrap()
}
#[test]
fn exact_retry_after_revocation_is_historical_and_immutable() {
    let mut state = TrustState::new("synthetic-local").unwrap();
    let registration = registration(1);
    let kid = registration.id.clone();
    state
        .apply(
            command("register", Operation::register(registration)),
            &auth(),
            0,
        )
        .unwrap();
    let activate = command("activate", Operation::Activate(kid.clone()));
    let receipt = state
        .apply(activate.clone(), &auth(), 1)
        .unwrap()
        .receipt()
        .clone();
    state
        .apply(
            command(
                "revoke",
                Operation::Revoke(kid.clone(), RevocationReason::SuspectedCompromise, Some(0)),
            ),
            &auth(),
            2,
        )
        .unwrap();
    let before = state.snapshot().unwrap().digest();
    assert_eq!(
        state.apply(activate.clone(), &auth(), 1).unwrap(),
        TrustCommandResult::HistoricalReplay(receipt)
    );
    assert_eq!(state.snapshot().unwrap().digest(), before);
    assert_eq!(state.status(&kid), Some(KeyStatus::Revoked));
    assert_eq!(
        state.apply(activate, &auth(), 3),
        Err(TrustError::ReplayConflict)
    );
}

fn setup(status: KeyStatus) -> (TrustState, SignerKeyId) {
    let mut state = TrustState::new("synthetic-local").unwrap();
    let r = registration(1);
    let id = r.id.clone();
    state
        .apply(command("register", Operation::register(r)), &auth(), 0)
        .unwrap();
    if matches!(status, KeyStatus::Active | KeyStatus::Retired) {
        state
            .apply(
                command("activate", Operation::Activate(id.clone())),
                &auth(),
                1,
            )
            .unwrap();
    }
    if status == KeyStatus::Retired {
        state
            .apply(command("retire", Operation::Retire(id.clone())), &auth(), 2)
            .unwrap();
    }
    if status == KeyStatus::Revoked {
        state
            .apply(
                command(
                    "revoke",
                    Operation::Revoke(id.clone(), RevocationReason::Administration, None),
                ),
                &auth(),
                1,
            )
            .unwrap();
    }
    (state, id)
}
#[test]
fn complete_single_key_transition_matrix_and_failure_atomicity() {
    for status in [
        KeyStatus::Pending,
        KeyStatus::Active,
        KeyStatus::Retired,
        KeyStatus::Revoked,
    ] {
        for tag in 0..4 {
            let (mut state, id) = setup(status);
            let op = match tag {
                0 => Operation::register(registration(1)),
                1 => Operation::Activate(id.clone()),
                2 => Operation::Retire(id.clone()),
                _ => Operation::Revoke(id.clone(), RevocationReason::Administration, None),
            };
            let before = state.snapshot().unwrap().digest();
            let revision = state.revision();
            let history = state.history.len();
            let result = state.apply(command("matrix", op), &auth(), revision);
            let expected = match (tag, status) {
                (1, KeyStatus::Pending) => Some(KeyStatus::Active),
                (2, KeyStatus::Active) => Some(KeyStatus::Retired),
                (3, KeyStatus::Pending | KeyStatus::Active | KeyStatus::Retired) => {
                    Some(KeyStatus::Revoked)
                }
                _ => None,
            };
            if let Some(expected) = expected {
                assert!(matches!(result, Ok(TrustCommandResult::Applied(_))));
                assert_eq!(state.status(&id), Some(expected));
                assert_eq!(state.revision(), revision + 1);
            } else {
                assert!(result.is_err());
                assert_eq!(state.snapshot().unwrap().digest(), before);
                assert_eq!(state.history.len(), history);
                assert_eq!(state.revision(), revision);
                assert!(!state.history.contains_key(&opaque("matrix").unwrap()));
            }
        }
    }
}
#[test]
fn rotation_is_atomic_and_requires_all_metadata_equal() {
    for mismatch in 0..5 {
        let (mut state, old) = setup(KeyStatus::Active);
        let mut replacement = registration(2);
        match mismatch {
            1 => replacement.principal = opaque("other-writer").unwrap(),
            2 => replacement.grants = grants(vec![scope("b")]).unwrap(),
            3 => replacement.profile = "other-profile".to_owned(),
            4 => replacement.purpose = KeyPurpose::LegacyDigest,
            _ => (),
        }
        let new = replacement.id.clone();
        // Test injection creates metadata mismatch that constructor normally rejects.
        state.keys.insert(
            new.clone(),
            Entry {
                registration: replacement,
                status: KeyStatus::Pending,
                since: 2,
                revocation: None,
            },
        );
        let authorized = AdminAuthorization::for_synthetic_host_policy(
            "synthetic-local",
            "synthetic-admin",
            vec![scope("a"), scope("b")],
        )
        .unwrap();
        let before = state.snapshot().unwrap().digest();
        let result = state.apply(
            command("rotate", Operation::Rotate(old.clone(), new.clone())),
            &authorized,
            2,
        );
        if mismatch == 0 {
            assert!(result.is_ok());
            assert_eq!(state.status(&old), Some(KeyStatus::Retired));
            assert_eq!(state.status(&new), Some(KeyStatus::Active));
            assert_eq!(state.keys[&old].since, state.keys[&new].since);
        } else {
            assert_eq!(result, Err(TrustError::InvalidTransition));
            assert_eq!(before, state.snapshot().unwrap().digest());
        }
    }
    for old_status in [
        KeyStatus::Pending,
        KeyStatus::Active,
        KeyStatus::Retired,
        KeyStatus::Revoked,
    ] {
        for new_status in [
            KeyStatus::Pending,
            KeyStatus::Active,
            KeyStatus::Retired,
            KeyStatus::Revoked,
        ] {
            let (mut state, old) = setup(old_status);
            let r = registration(2);
            let new = r.id.clone();
            state.keys.insert(
                new.clone(),
                Entry {
                    registration: r,
                    status: new_status,
                    since: 1,
                    revocation: None,
                },
            );
            let before = state.snapshot().unwrap().digest();
            let revision = state.revision();
            let result = state.apply(
                command("rotate", Operation::Rotate(old, new)),
                &auth(),
                revision,
            );
            if old_status == KeyStatus::Active && new_status == KeyStatus::Pending {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
                assert_eq!(before, state.snapshot().unwrap().digest());
            }
        }
    }
}
#[test]
fn current_authorization_precedes_replay_and_no_failure_reserves_id() {
    let mut state = TrustState::new("synthetic-local").unwrap();
    let registration = registration(1);
    let cmd = command("id", Operation::register(registration));
    let denied = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![scope("b")],
    )
    .unwrap();
    assert_eq!(
        state.apply(cmd.clone(), &denied, 0),
        Err(TrustError::UnauthorizedAdmin)
    );
    assert!(state.history.is_empty());
    let receipt = state.apply(cmd.clone(), &auth(), 0).unwrap();
    let before = state.snapshot().unwrap().digest();
    assert_eq!(
        state.apply(cmd.clone(), &denied, 0),
        Err(TrustError::UnauthorizedAdmin)
    );
    let foreign = AdminAuthorization::for_synthetic_host_policy(
        "foreign",
        "synthetic-admin",
        vec![scope("a")],
    )
    .unwrap();
    assert_eq!(
        state.apply(cmd.clone(), &foreign, 0),
        Err(TrustError::UnauthorizedAdmin)
    );
    let changed = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "other-admin",
        vec![scope("a")],
    )
    .unwrap();
    assert_eq!(
        state.apply(cmd.clone(), &changed, 0),
        Err(TrustError::ReplayConflict)
    );
    assert_eq!(before, state.snapshot().unwrap().digest());
    assert_eq!(state.history.len(), 1);
    assert_eq!(
        state.apply(cmd, &auth(), 0).unwrap().receipt(),
        receipt.receipt()
    );
}
#[test]
fn stale_revision_unknown_key_and_invalid_transition_leave_id_reusable() {
    let (mut state, id) = setup(KeyStatus::Pending);
    let cmd = command("retry", Operation::Activate(id));
    assert_eq!(
        state.apply(cmd.clone(), &auth(), 0),
        Err(TrustError::StaleTrustRevision)
    );
    assert_eq!(
        state.apply(cmd, &auth(), 1).unwrap().receipt().revision(),
        2
    );
    let unknown = registration(3).id;
    assert_eq!(
        state.apply(command("unknown", Operation::Activate(unknown)), &auth(), 2),
        Err(TrustError::UnknownKey)
    );
    assert!(!state.history.contains_key(&opaque("unknown").unwrap()));
    let changed = command("retry", Operation::Retire(registration(1).id));
    assert_eq!(
        state.apply(changed, &auth(), 1),
        Err(TrustError::ReplayConflict)
    );
}
#[test]
fn registration_identifiers_grants_purpose_and_key_refusals() {
    let key = SigningKey::from_bytes(&[1; 32]).verifying_key();
    let id = SignerKeyId::for_key(&key);
    for n in [0, 31, 33] {
        assert!(
            KeyRegistration::new(
                &vec![1; n],
                id.clone(),
                "writer",
                KeyPurpose::P1CanonicalWriter,
                PROFILE_ID,
                vec![scope("a")]
            )
            .is_err()
        );
    }
    assert!(matches!(
        KeyRegistration::new(
            key.as_bytes(),
            registration(2).id,
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            vec![scope("a")]
        ),
        Err(TrustError::InvalidKeyRegistration)
    ));
    assert!(matches!(
        KeyRegistration::new(
            key.as_bytes(),
            id.clone(),
            "writer",
            KeyPurpose::LegacyDigest,
            PROFILE_ID,
            vec![scope("a")]
        ),
        Err(TrustError::WrongKeyPurpose)
    ));
    assert!(matches!(
        KeyRegistration::new(
            key.as_bytes(),
            id.clone(),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            "other",
            vec![scope("a")]
        ),
        Err(TrustError::WrongKeyPurpose)
    ));
    assert!(matches!(
        KeyRegistration::new(
            key.as_bytes(),
            id.clone(),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            vec![scope("a"), scope("a")]
        ),
        Err(TrustError::DuplicateGrant)
    ));
    let weak = VerifyingKey::from_bytes(&[0; 32]).unwrap();
    assert!(matches!(
        KeyRegistration::new(
            weak.as_bytes(),
            SignerKeyId::for_key(&weak),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            vec![scope("a")]
        ),
        Err(TrustError::InvalidKeyRegistration)
    ));
    assert!(TrustCommand::new("", Operation::Activate(id.clone())).is_err());
    let grants64: Vec<_> = (0..64).map(|i| scope(&format!("g{i}"))).collect();
    assert!(
        KeyRegistration::new(
            key.as_bytes(),
            id.clone(),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            grants64.clone()
        )
        .is_ok()
    );
    let mut too_many = grants64;
    too_many.push(scope("extra"));
    assert!(matches!(
        KeyRegistration::new(
            key.as_bytes(),
            id,
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            too_many
        ),
        Err(TrustError::ResourceLimit)
    ));
}
#[test]
fn snapshots_are_immutable_and_math_keys_remain_resolvable_in_all_states() {
    let (mut state, id) = setup(KeyStatus::Active);
    let old = state.snapshot().unwrap();
    let before = old.digest();
    assert!(old.permits(&id, &opaque("writer").unwrap(), &scope("a")));
    state
        .apply(
            command(
                "revoke",
                Operation::Revoke(id.clone(), RevocationReason::Administration, None),
            ),
            &auth(),
            2,
        )
        .unwrap();
    assert_eq!(old.digest(), before);
    assert_eq!(old.status(&id), Some(KeyStatus::Active));
    assert!(
        !state
            .snapshot()
            .unwrap()
            .permits(&id, &opaque("writer").unwrap(), &scope("a"))
    );
    for status in [
        KeyStatus::Pending,
        KeyStatus::Active,
        KeyStatus::Retired,
        KeyStatus::Revoked,
    ] {
        let (state, id) = setup(status);
        let snapshot = state.snapshot().unwrap();
        let key_snapshot = snapshot.verification_keys().unwrap();
        let signed = super::super::verify_tests::key(1);
        let payload = super::super::verify_tests::payload(&signed);
        let container = super::super::verify::sign_p1(
            &payload,
            &signed,
            super::super::profile::CryptoLimits::default(),
        )
        .unwrap();
        let parsed = super::super::profile::parse_p1(
            &container,
            &payload,
            super::super::profile::CryptoLimits::default(),
        )
        .unwrap();
        let expected = super::super::verify_tests::expected(&payload);
        let report = super::super::verify::verify_p1(parsed, &expected, &key_snapshot);
        assert_eq!(report.signer(), &id);
        assert_eq!(report.integrity(), super::super::verify::Integrity::Valid);
        assert_ne!(snapshot.digest(), key_snapshot.digest());
    }
}
#[test]
fn canonical_command_binds_grant_order_principal_revision_and_optional_zero() {
    let key = SigningKey::from_bytes(&[1; 32]).verifying_key();
    let id = SignerKeyId::for_key(&key);
    let make = |scopes| {
        KeyRegistration::new(
            key.as_bytes(),
            id.clone(),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            scopes,
        )
        .unwrap()
    };
    let a = command(
        "reg",
        Operation::register(make(vec![scope("a"), scope("b")])),
    );
    let b = command(
        "reg",
        Operation::register(make(vec![scope("b"), scope("a")])),
    );
    let ns = opaque("synthetic-local").unwrap();
    let admin = opaque("synthetic-admin").unwrap();
    assert_eq!(
        a.canonical_bytes(&ns, &admin, 0),
        b.canonical_bytes(&ns, &admin, 0)
    );
    assert_ne!(
        a.canonical_bytes(&ns, &admin, 0),
        a.canonical_bytes(&ns, &admin, 1)
    );
    let none = command(
        "rev",
        Operation::Revoke(id.clone(), RevocationReason::Administration, None),
    );
    let zero = command(
        "rev",
        Operation::Revoke(id, RevocationReason::Administration, Some(0)),
    );
    assert_ne!(
        none.canonical_bytes(&ns, &admin, 1),
        zero.canonical_bytes(&ns, &admin, 1)
    );
}
#[test]
fn resource_boundaries_overflow_and_failed_id_correction() {
    let (mut state, id) = setup(KeyStatus::Pending);
    assert_eq!(
        state.apply(
            command("correct", Operation::Retire(id.clone())),
            &auth(),
            1
        ),
        Err(TrustError::InvalidTransition)
    );
    state
        .apply(command("correct", Operation::Activate(id)), &auth(), 1)
        .unwrap();
    state.revision = u64::MAX;
    let before = state.snapshot().unwrap().digest();
    assert_eq!(
        state.apply(
            command("overflow", Operation::register(registration(2))),
            &auth(),
            u64::MAX
        ),
        Err(TrustError::RevisionOverflow)
    );
    assert_eq!(before, state.snapshot().unwrap().digest());
    // Boundary injection isolates successful-history capacity without executing
    // thousands of unrelated transitions. Existing successful records stay exact.
    let (mut full, _) = setup(KeyStatus::Pending);
    for index in 1..HISTORY_CAP - 1 {
        let id = opaque(&format!("synthetic-{index}")).unwrap();
        full.order.push(id.clone());
        full.history.insert(
            id,
            History {
                command: vec![0],
                receipt: TrustReceipt {
                    revision: index as u64,
                    command_digest: hash(&[0]),
                },
                grants: grants(vec![scope("a")]).unwrap(),
            },
        );
    }
    full.apply(
        command("last-admitted", Operation::register(registration(2))),
        &auth(),
        full.revision(),
    )
    .unwrap();
    let before = full.snapshot().unwrap().digest();
    assert_eq!(
        full.apply(
            command("new", Operation::register(registration(3))),
            &auth(),
            full.revision()
        ),
        Err(TrustError::ResourceLimit)
    );
    assert_eq!(before, full.snapshot().unwrap().digest());
    assert!(matches!(
        full.apply(
            command("register", Operation::register(registration(1))),
            &auth(),
            0
        ),
        Ok(TrustCommandResult::HistoricalReplay(_))
    ));
    assert_eq!(full.history.len(), HISTORY_CAP);
    let mut keys = TrustState::new("synthetic-local").unwrap();
    for index in 0..KEY_CAP - 1 {
        let key = SigningKey::from_bytes(&hash(&(index as u64).to_be_bytes())).verifying_key();
        let registration = KeyRegistration::new(
            key.as_bytes(),
            SignerKeyId::for_key(&key),
            "writer",
            KeyPurpose::P1CanonicalWriter,
            PROFILE_ID,
            vec![scope("a")],
        )
        .unwrap();
        keys.keys.insert(
            registration.id.clone(),
            Entry {
                registration,
                status: KeyStatus::Pending,
                since: 0,
                revocation: None,
            },
        );
    }
    keys.apply(
        command("last-admitted", Operation::register(registration(1))),
        &auth(),
        0,
    )
    .unwrap();
    let before = keys.snapshot().unwrap().digest();
    assert_eq!(
        keys.apply(
            command("full", Operation::register(registration(2))),
            &auth(),
            1
        ),
        Err(TrustError::ResourceLimit)
    );
    assert_eq!(before, keys.snapshot().unwrap().digest());
    assert_eq!(keys.keys.len(), KEY_CAP);
}
#[test]
fn old_history_scope_is_authorized_before_conflict_or_receipt() {
    let (mut state, _) = setup(KeyStatus::Pending);
    let key = SigningKey::from_bytes(&[2; 32]).verifying_key();
    let other = KeyRegistration::new(
        key.as_bytes(),
        SignerKeyId::for_key(&key),
        "writer",
        KeyPurpose::P1CanonicalWriter,
        PROFILE_ID,
        vec![scope("b")],
    )
    .unwrap();
    let only_b = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![scope("b")],
    )
    .unwrap();
    let before = state.snapshot().unwrap().digest();
    assert_eq!(
        state.apply(command("register", Operation::register(other)), &only_b, 0),
        Err(TrustError::UnauthorizedAdmin)
    );
    assert_eq!(before, state.snapshot().unwrap().digest());
}
#[test]
fn independent_command_and_snapshot_vectors_and_typed_rejections() {
    let vector: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/completion/execution/evidence/2026-10-22-continuation/trust/commands.json"
    )))
    .unwrap();
    let decode = |hex: &str| {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let hex = |bytes: &[u8]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let registrations: Vec<_> = vector["public_keys_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| {
            let public = decode(value.as_str().unwrap());
            let key = VerifyingKey::from_bytes(public.as_slice().try_into().unwrap()).unwrap();
            KeyRegistration::new(
                &public,
                SignerKeyId::for_key(&key),
                "writer",
                KeyPurpose::P1CanonicalWriter,
                PROFILE_ID,
                vec![scope("a")],
            )
            .unwrap()
        })
        .collect();
    let old = registrations[0].id.clone();
    let new = registrations[1].id.clone();
    let operations = vec![
        Operation::register(registrations[0].clone()),
        Operation::Activate(old.clone()),
        Operation::register(registrations[1].clone()),
        Operation::Rotate(old.clone(), new),
        Operation::Revoke(old.clone(), RevocationReason::SuspectedCompromise, None),
    ];
    let mut state = TrustState::new("synthetic-local").unwrap();
    for (case, operation) in vector["cases"].as_array().unwrap().iter().zip(operations) {
        let command = command(case["command_id"].as_str().unwrap(), operation);
        let revision = case["expected_revision"].as_u64().unwrap();
        let bytes = command
            .canonical_bytes(&state.namespace, &auth().principal, revision)
            .unwrap();
        assert_eq!(hex(&bytes), case["command_hex"].as_str().unwrap());
        let receipt = state.apply(command, &auth(), revision).unwrap();
        assert_eq!(
            hex(&receipt.receipt().command_digest()),
            case["command_sha256"].as_str().unwrap()
        );
        assert_eq!(
            hex(&state.snapshot().unwrap().digest()),
            case["snapshot_sha256"].as_str().unwrap()
        );
    }
    let zero = command(
        "revoke-old",
        Operation::Revoke(old, RevocationReason::SuspectedCompromise, Some(0)),
    )
    .canonical_bytes(&state.namespace, &auth().principal, 4)
    .unwrap();
    assert_eq!(
        hex(&zero),
        vector["claimed_zero_command_hex"].as_str().unwrap()
    );
    for refusal in vector["typed_rejections"].as_array().unwrap() {
        let registration = || registrations[0].clone();
        let result = match refusal["case"].as_str().unwrap() {
            "duplicate-grants" => grants(vec![scope("a"), scope("a")]).map(|_| ()),
            "weak-key" => {
                let key = VerifyingKey::from_bytes(&[0; 32]).unwrap();
                KeyRegistration::new(
                    key.as_bytes(),
                    SignerKeyId::for_key(&key),
                    "writer",
                    KeyPurpose::P1CanonicalWriter,
                    PROFILE_ID,
                    vec![scope("a")],
                )
                .map(|_| ())
            }
            "wrong-purpose" => {
                let r = registration();
                KeyRegistration::new(
                    r.key.as_bytes(),
                    r.id,
                    "writer",
                    KeyPurpose::LegacyDigest,
                    PROFILE_ID,
                    vec![scope("a")],
                )
                .map(|_| ())
            }
            "wrong-profile" => {
                let r = registration();
                KeyRegistration::new(
                    r.key.as_bytes(),
                    r.id,
                    "writer",
                    KeyPurpose::P1CanonicalWriter,
                    "other",
                    vec![scope("a")],
                )
                .map(|_| ())
            }
            "empty-command-id" => {
                TrustCommand::new("", Operation::register(registration())).map(|_| ())
            }
            _ => panic!("unknown synthetic typed refusal"),
        };
        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            refusal["error"].as_str().unwrap()
        );
    }
}
