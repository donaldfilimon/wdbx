//! Memory-edge episodes (amendment 2026-09-16): quarantine, contradiction, and
//! resolution as single-event operations that never alter or hide the
//! candidates they name, checked on both the write and the replay path.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EdgeReason, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass, MemoryEdge,
    MemoryEdgeKind, MemoryEdgeState, OpenContradiction, RetentionClass, StorePolicy,
    TerminalReason, TerminalStatus,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const GOLDEN_PATH: &str = "tests/golden/episode_write_memory_edge.json";
/// Digest of the golden contradiction, previewed after its two candidates are
/// admitted under [`golden_policy`]. A downstream transcription must
/// reproduce this from the golden JSON byte for byte.
const GOLDEN_DIGEST: &str = "dfc839a6d6e6a39a0e3bccf837ea232f291ae74999241422efc26167eb47d47f";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-edge-{}", Uuid::new_v4())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

fn service() -> ActorRef {
    actor("abbey_service", ActorKind::Service)
}

fn admin() -> ActorRef {
    actor("admin_ref", ActorKind::GuildAdministrator)
}

fn guild_policy() -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: Some(7),
    }
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            ("guild_ref".into(), guild_policy()),
            ("other_guild".into(), guild_policy()),
        ]),
    }
}

fn write(request_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: format!("op_{request_id}"),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: "guild_ref".into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event,
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

fn candidate(seed: u8, member_scoped: bool) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped,
        supersedes: None,
        forgets: None,
    }
}

fn memory(request_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    write(
        request_id,
        EpisodeEvent::MemoryCandidate {
            recorded_by: service(),
            candidate,
        },
    )
}

fn edge_write(request_id: &str, recorded_by: ActorRef, edge: MemoryEdge) -> EpisodeWrite {
    write(request_id, EpisodeEvent::MemoryEdge { recorded_by, edge })
}

fn quarantine(target: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Quarantines,
        target,
        counterpart: None,
        reason: EdgeReason::SourceUntrusted,
    }
}

/// A contradiction in canonical (ascending) order.
fn contradiction(a: [u8; 32], b: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Contradicts,
        target: a.min(b),
        counterpart: Some(a.max(b)),
        reason: EdgeReason::ConflictingObservation,
    }
}

fn resolution(edge: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Resolves,
        target: edge,
        counterpart: None,
        reason: EdgeReason::ReviewedValid,
    }
}

fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    preview
}

fn rejected(store: &mut EpisodeStore, event: &EpisodeWrite) -> EpisodeStoreError {
    let error = store.propose_write(event).expect_err("rejected write");
    assert_eq!(
        store
            .preview_commitment(event)
            .map(|_| ())
            .unwrap_err()
            .to_string(),
        error.to_string(),
        "preview and append agree on the rejection"
    );
    error
}

fn state(store: &EpisodeStore, digest: &[u8; 32]) -> MemoryEdgeState {
    store
        .memory_edge_state("guild_ref", digest)
        .expect("valid lookup")
        .expect("admitted candidate")
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

#[test]
fn a_quarantine_opens_closes_by_human_resolution_and_survives_reopen() {
    let scratch = Scratch::new();
    let (first, edge, reopened_edge);
    {
        let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
        first = append(&mut store, memory("request_1", candidate(1, true)));
        assert_eq!(
            state(&store, &first),
            MemoryEdgeState {
                forgotten: false,
                open_quarantine: None,
                open_contradictions: vec![],
            }
        );

        edge = append(
            &mut store,
            edge_write("request_2", service(), quarantine(first)),
        );
        let receipts = store.retrieve("guild_ref", 10).expect("receipts");
        assert_eq!(receipts.len(), 2, "the quarantined candidate stays visible");
        assert_eq!(receipts[1].event_kind, "memory_edge");
        assert_eq!(receipts[1].terminal_status, Some(TerminalStatus::Completed));
        assert_eq!(state(&store, &first).open_quarantine, Some(edge));
        assert_eq!(
            store.memory_edge_open("guild_ref", &edge).unwrap(),
            Some(true)
        );

        append(
            &mut store,
            edge_write("request_9", admin(), resolution(edge)),
        );
        assert_eq!(state(&store, &first).open_quarantine, None);
        assert_eq!(
            store.memory_edge_open("guild_ref", &edge).unwrap(),
            Some(false)
        );
        assert!(matches!(
            rejected(
                &mut store,
                &edge_write("request_10", admin(), resolution(edge))
            ),
            EpisodeStoreError::InvalidTransition
        ));

        // A closed quarantine may be followed by a new one.
        reopened_edge = append(
            &mut store,
            edge_write("request_11", service(), quarantine(first)),
        );
    }

    let mut reopened = EpisodeStore::open(scratch.path(), policy()).expect("verified reopen");
    assert_eq!(
        state(&reopened, &first).open_quarantine,
        Some(reopened_edge)
    );
    assert_eq!(
        reopened.memory_edge_open("guild_ref", &edge).unwrap(),
        Some(false)
    );
    assert!(matches!(
        rejected(
            &mut reopened,
            &edge_write("request_12", admin(), resolution(edge))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    assert_eq!(
        reopened.retrieve("guild_ref", 20).expect("receipts").len(),
        4
    );
}

#[test]
fn edge_authority_and_single_event_rules() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let first = append(&mut store, memory("request_1", candidate(1, true)));
    let edge = append(
        &mut store,
        edge_write("request_2", service(), quarantine(first)),
    );

    // One open quarantine per candidate.
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_3", service(), quarantine(first))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    // A service cannot clear its own flag, and neither can a subject.
    for (index, author) in [service(), actor("subject_ref", ActorKind::HumanSubject)]
        .into_iter()
        .enumerate()
    {
        assert!(matches!(
            rejected(
                &mut store,
                &edge_write(&format!("request_4_{index}"), author, resolution(edge))
            ),
            EpisodeStoreError::InvalidTransition
        ));
    }
    // A human cannot record a quarantine; a service must.
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_5", admin(), quarantine(first))
        ),
        EpisodeStoreError::InvalidTransition
    ));

    // The edge operation is terminal in the same append.
    let mut follow = write(
        "request_6",
        EpisodeEvent::Terminal {
            status: TerminalStatus::Completed,
            reason: TerminalReason::Completed,
        },
    );
    follow.operation_id = "op_request_2".into();
    assert!(matches!(
        rejected(&mut store, &follow),
        EpisodeStoreError::InvalidTransition
    ));
    let mut again = edge_write("request_7", service(), quarantine(first));
    again.operation_id = "op_request_2".into();
    assert!(matches!(
        rejected(&mut store, &again),
        EpisodeStoreError::Replay
    ));

    for (index, author) in [
        admin(),
        actor("owner_ref", ActorKind::GuildOwner),
        actor("manager_ref", ActorKind::GuildManager),
        actor("org_ref", ActorKind::OrganizationOwner),
    ]
    .into_iter()
    .enumerate()
    {
        // Every governance kind may preview a resolution.
        let event = edge_write(&format!("request_8_{index}"), author, resolution(edge));
        store
            .preview_commitment(&event)
            .expect("governance resolves");
    }
}

#[test]
fn quarantine_never_blocks_forgetting_or_correction() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let first = append(&mut store, memory("request_1", candidate(1, true)));
    let other = append(&mut store, memory("request_2", candidate(2, true)));
    let flag = append(
        &mut store,
        edge_write("request_3", service(), quarantine(first)),
    );
    let pair = append(
        &mut store,
        edge_write("request_4", service(), contradiction(first, other)),
    );

    // Correction of a suspect memory still works, and starts unflagged.
    let corrected = append(
        &mut store,
        memory(
            "request_5",
            MemoryCandidate {
                supersedes: Some(first),
                ..candidate(5, true)
            },
        ),
    );
    assert_eq!(state(&store, &corrected).open_quarantine, None);
    assert_eq!(state(&store, &first).open_quarantine, Some(flag));

    // Forgetting a quarantined memory is never refused.
    append(
        &mut store,
        memory(
            "request_6",
            MemoryCandidate {
                payload_commitment: [0; 32],
                payload_bytes: 0,
                forgets: Some(first),
                ..candidate(6, true)
            },
        ),
    );
    let tomb = state(&store, &first);
    assert!(tomb.forgotten);
    assert_eq!(tomb.open_quarantine, Some(flag), "edges stay in place");
    assert_eq!(
        tomb.open_contradictions,
        vec![OpenContradiction {
            counterpart: other,
            edge: pair,
        }]
    );

    // No new edge may name the tombstone...
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_7", service(), contradiction(first, corrected))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    // ...but the edges already open on it can still be resolved.
    append(
        &mut store,
        edge_write("request_8", admin(), resolution(flag)),
    );
    append(
        &mut store,
        edge_write("request_9", admin(), resolution(pair)),
    );
    assert_eq!(
        state(&store, &first),
        MemoryEdgeState {
            forgotten: true,
            open_quarantine: None,
            open_contradictions: vec![],
        }
    );
    assert_eq!(state(&store, &other).open_contradictions, vec![]);
}

#[test]
fn edge_shapes_are_closed_before_any_append() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let a = append(&mut store, memory("request_1", candidate(1, true)));
    let b = append(&mut store, memory("request_2", candidate(2, true)));
    let (low, high) = (a.min(b), a.max(b));

    let bad_shapes = [
        // Reversed order: the same disagreement must not have two encodings.
        MemoryEdge {
            target: high,
            counterpart: Some(low),
            ..contradiction(a, b)
        },
        // A candidate cannot contradict itself.
        MemoryEdge {
            counterpart: Some(low),
            target: low,
            ..contradiction(a, b)
        },
        MemoryEdge {
            counterpart: None,
            ..contradiction(a, b)
        },
        MemoryEdge {
            counterpart: Some(b),
            ..quarantine(a)
        },
        MemoryEdge {
            counterpart: Some(b),
            ..resolution(a)
        },
        MemoryEdge {
            reason: EdgeReason::SourceUntrusted,
            ..contradiction(a, b)
        },
        MemoryEdge {
            reason: EdgeReason::ConflictingObservation,
            ..quarantine(a)
        },
        MemoryEdge {
            reason: EdgeReason::ReviewedInvalid,
            ..quarantine(a)
        },
        MemoryEdge {
            reason: EdgeReason::OperatorReport,
            ..resolution(a)
        },
        quarantine([0; 32]),
    ];
    for (index, shape) in bad_shapes.into_iter().enumerate() {
        let event = edge_write(&format!("shape_{index}"), service(), shape);
        assert!(
            matches!(
                rejected(&mut store, &event),
                EpisodeStoreError::InvalidInput
            ),
            "shape {index} must be invalid input"
        );
    }
    let mut voice = edge_write("voice", service(), quarantine(a));
    voice.source_type = EpisodeSource::DiscordVoice;
    voice.consent_epoch = Some(7);
    assert!(matches!(
        rejected(&mut store, &voice),
        EpisodeStoreError::InvalidInput
    ));
    // Free text cannot ride along: the edge is a closed shape.
    let mut raw =
        serde_json::to_value(edge_write("raw", service(), quarantine(a))).expect("write to JSON");
    raw["event"]["edge"]["note"] = serde_json::Value::String("secret text".into());
    assert!(serde_json::from_value::<EpisodeWrite>(raw).is_err());
}

#[test]
fn contradictions_have_one_encoding_and_obey_scope() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let a = append(&mut store, memory("request_1", candidate(1, true)));
    let b = append(&mut store, memory("request_2", candidate(2, true)));
    let guild_wide = append(&mut store, memory("request_3", candidate(3, false)));

    let mut foreign = memory("request_4", candidate(4, true));
    foreign.guild_ref = "other_guild".into();
    let foreign = append(&mut store, foreign);
    let graph_rejections = [
        ("unknown", service(), contradiction(a, [0xee; 32])),
        ("cross_guild", service(), contradiction(a, foreign)),
        ("cross_scope", service(), contradiction(a, guild_wide)),
        ("human_flag", admin(), contradiction(a, b)),
        // Only candidates are memory; only edges are resolvable.
        ("resolve_candidate", admin(), resolution(a)),
    ];
    for (request, author, edge) in graph_rejections {
        assert!(
            matches!(
                rejected(&mut store, &edge_write(request, author, edge)),
                EpisodeStoreError::InvalidTransition
            ),
            "{request} must be refused"
        );
    }

    let pair = append(
        &mut store,
        edge_write("request_5", service(), contradiction(a, b)),
    );
    assert_eq!(
        state(&store, &a).open_contradictions,
        vec![OpenContradiction {
            counterpart: b,
            edge: pair,
        }]
    );
    assert_eq!(
        state(&store, &b).open_contradictions,
        vec![OpenContradiction {
            counterpart: a,
            edge: pair,
        }]
    );
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_6", service(), contradiction(b, a))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    // An edge digest is neither a quarantine target nor a contradiction side.
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_7", service(), quarantine(pair))
        ),
        EpisodeStoreError::InvalidTransition
    ));

    let closed = append(
        &mut store,
        edge_write("request_8", admin(), resolution(pair)),
    );
    assert_eq!(state(&store, &a).open_contradictions, vec![]);
    // A resolution is not itself resolvable.
    assert!(matches!(
        rejected(
            &mut store,
            &edge_write("request_9", admin(), resolution(closed))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    assert_eq!(store.memory_edge_open("guild_ref", &closed).unwrap(), None);
    // The pair may disagree again once the first contradiction is closed.
    append(
        &mut store,
        edge_write("request_10", service(), contradiction(a, b)),
    );
    // Lookups are guild-scoped and never guess.
    assert_eq!(store.memory_edge_state("other_guild", &a).unwrap(), None);
    assert_eq!(store.memory_edge_state("guild_ref", &pair).unwrap(), None);
    assert!(store.memory_edge_state("Bad Guild", &a).is_err());
}

#[test]
fn replay_rejects_a_resolution_whose_edge_was_removed() {
    let source = Scratch::new();
    {
        let mut store = EpisodeStore::open(source.path(), policy()).expect("open");
        let first = append(&mut store, memory("request_1", candidate(1, true)));
        let flag = append(
            &mut store,
            edge_write("request_2", service(), quarantine(first)),
        );
        append(
            &mut store,
            edge_write("request_3", admin(), resolution(flag)),
        );
    }
    let ledger = fs::read_to_string(source.path().join("episodes.v1.jsonl")).expect("ledger");
    let lines: Vec<&str> = ledger.lines().collect();
    assert_eq!(lines.len(), 3);

    // Drop the quarantine and renumber the resolution. `sequence` sits outside
    // the digest, so every digest still verifies: only the edge rule can
    // catch that the resolution now names nothing open.
    let mut resolution: serde_json::Value = serde_json::from_str(lines[2]).expect("record");
    resolution["sequence"] = serde_json::Value::from(2);
    let doctored = format!("{}\n{}\n", lines[0], resolution);
    let target = Scratch::new();
    fs::create_dir_all(target.path()).expect("scratch");
    fs::write(target.path().join("episodes.v1.jsonl"), doctored).expect("write ledger");
    assert!(matches!(
        EpisodeStore::open(target.path(), policy()),
        Err(EpisodeStoreError::Corrupt)
    ));

    // The untouched ledger still opens, so the refusal above is the rule.
    EpisodeStore::open(source.path(), policy()).expect("original replays");

    // A mutated reason breaks the digest.
    let tampered = ledger.replacen("source_untrusted", "operator_report", 1);
    assert_ne!(tampered, ledger);
    let mutated = Scratch::new();
    fs::create_dir_all(mutated.path()).expect("scratch");
    fs::write(mutated.path().join("episodes.v1.jsonl"), tampered).expect("write ledger");
    assert!(matches!(
        EpisodeStore::open(mutated.path(), policy()),
        Err(EpisodeStoreError::Corrupt)
    ));
}

/// Deterministic 32-byte pattern `index * mul + add` (wrapping), for fixtures.
fn pattern(mul: u8, add: u8) -> [u8; 32] {
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .expect("index below 32")
            .wrapping_mul(mul)
            .wrapping_add(add);
    }
    out
}

fn golden_policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guilds: BTreeMap::from([(
            "discord-123456789012345678".into(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1_000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: None,
            },
        )]),
    }
}

fn golden_base(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guild_ref: "discord-123456789012345678".into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C0,
        event,
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// Two deterministic candidates, then the contradiction between them.
fn golden_store_and_write(scratch: &Scratch) -> (EpisodeStore, EpisodeWrite) {
    let mut store = EpisodeStore::open(scratch.path(), golden_policy()).expect("open");
    let mut sides = [0_u8; 2].map(|_| [0_u8; 32]);
    for (index, side) in sides.iter_mut().enumerate() {
        let seed = u8::try_from(index).expect("two sides");
        *side = append(
            &mut store,
            golden_base(
                &format!("req-memory-side-{index}"),
                &format!("memory-side-{index}"),
                EpisodeEvent::MemoryCandidate {
                    recorded_by: actor("abbey-service", ActorKind::Service),
                    candidate: MemoryCandidate {
                        payload_commitment: pattern(5, 2 + seed),
                        ..candidate(1, true)
                    },
                },
            ),
        );
    }
    let write = golden_base(
        "req-edge-00000000deadbeef",
        "memory-edge-0123456789abcdef",
        EpisodeEvent::MemoryEdge {
            recorded_by: actor("abbey-service", ActorKind::Service),
            edge: contradiction(sides[0], sides[1]),
        },
    );
    (store, write)
}

/// The wire form and digest downstream transcriptions must match.
/// Regenerate the JSON with `WDBX_WRITE_GOLDEN=1` after a deliberate change;
/// the digest constant is then updated by hand from the failing assertion.
#[test]
fn wire_form_and_digest_are_pinned_for_transcriptions() {
    let scratch = Scratch::new();
    let (store, write) = golden_store_and_write(&scratch);
    let json = serde_json::to_string(&write).expect("write JSON");
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_PATH);
    if std::env::var_os("WDBX_WRITE_GOLDEN").is_some() {
        fs::write(&golden, format!("{json}\n")).expect("write golden");
    }
    let stored = fs::read_to_string(&golden).expect("golden fixture present");
    assert_eq!(
        stored.trim_end(),
        json,
        "golden JSON drifted from the canonical type"
    );
    assert_eq!(
        serde_json::from_str::<EpisodeWrite>(&stored).expect("golden parses"),
        write
    );
    assert!(stored.contains("\"kind\":\"memory_edge\""));
    let digest = store.preview_commitment(&write).expect("golden previews");
    assert_eq!(hex(&digest), GOLDEN_DIGEST);
}
