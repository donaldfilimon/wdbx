//! Cross-language differential test for the v3 episode digest.
//!
//! `tools/abbey_cbor_episode_v1.py` also reimplements the store's derivation
//! of an episode's header and payload from an `EpisodeWrite` wire form. This
//! test drives a real `EpisodeStore` through every event variant, including a
//! full proposal chain whose later records carry their predecessor as parent,
//! and requires the Python digest for each write to equal the digest in the
//! receipt the store actually appended. The two golden digests the memory
//! tests pin are also re-derived by the script on its own.
//!
//! Same rule as `v3_cross_language_commitment`: it needs `python3` (or
//! `WDBX_PYTHON`) and fails, rather than skips, without it.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, AttributionResult, AuthorizationState, EdgeReason, EpisodeEvent,
    EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MediaOutcome,
    MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, RetentionClass, StorePolicy,
    TerminalReason, TerminalStatus, VoiceEvidence, VoiceTransition,
};
use serde_json::{Value, json};
use uuid::Uuid;

const GUILD: &str = "discord-123456789012345678";
/// A guild whose policy carries a live consent epoch, so voice evidence is admissible.
const VOICE_GUILD: &str = "discord-voice-00000000000001";
const VOICE_EPOCH: u64 = 3;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-xlang-{}", Uuid::new_v4())))
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

fn python() -> String {
    std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".to_owned())
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("abbey_cbor_episode_v1.py")
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("write to string");
        output
    })
}

/// Deterministic 32-byte pattern `index * mul + add` (wrapping), as the golden tests use.
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

fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

fn service() -> ActorRef {
    actor("abbey-service", ActorKind::Service)
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guilds: BTreeMap::from([
            (
                GUILD.into(),
                GuildEpisodePolicy {
                    learning_enabled: true,
                    policy_version: "policy_v1".into(),
                    token_budget: 1_000,
                    storage_budget_bytes: 1024 * 1024,
                    current_consent_epoch: None,
                },
            ),
            (
                VOICE_GUILD.into(),
                GuildEpisodePolicy {
                    learning_enabled: true,
                    policy_version: "policy_v1".into(),
                    token_budget: 1_000,
                    storage_budget_bytes: 1024 * 1024,
                    current_consent_epoch: Some(VOICE_EPOCH),
                },
            ),
        ]),
    }
}

fn write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guild_ref: GUILD.into(),
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

/// A write bound to the voice guild's consent epoch, the only shape that may carry voice evidence.
fn voice_write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        guild_ref: VOICE_GUILD.into(),
        consent_epoch: Some(VOICE_EPOCH),
        source_type: EpisodeSource::DiscordVoice,
        ..write(request_id, operation_id, event)
    }
}

fn candidate(seed: u8, class: MemoryClass, retention: RetentionClass) -> MemoryCandidate {
    let embedding = class == MemoryClass::Embedding;
    MemoryCandidate {
        class,
        retention,
        payload_commitment: pattern(5, seed),
        payload_bytes: 256 + u64::from(seed) * 300,
        dimension: embedding.then_some(384),
        embedding_version: embedding.then(|| "abbey-embedding-v1".to_owned()),
        member_scoped: seed.is_multiple_of(2),
        supersedes: None,
        forgets: None,
    }
}

fn voice() -> VoiceEvidence {
    VoiceEvidence {
        consent_epoch: VOICE_EPOCH,
        participant_count: 2,
        authorization_state: AuthorizationState::Closed,
        attribution: AttributionResult::Attributed,
        stt: MediaOutcome::Succeeded,
        tts: MediaOutcome::NotAttempted,
        playback: MediaOutcome::Cancelled,
        barge_in_count: 1,
        transitions: vec![
            VoiceTransition::Opened,
            VoiceTransition::Attested,
            VoiceTransition::ParticipantChangeClosed,
        ],
        terminal_reason: TerminalReason::ParticipantChange,
    }
}

/// One appended record: the write as the store saw it, and what the store committed.
struct Appended {
    name: String,
    write: EpisodeWrite,
    previous_digest: Option<[u8; 32]>,
    episode_digest: [u8; 32],
}

fn append(
    log: &mut Vec<Appended>,
    store: &mut EpisodeStore,
    name: &str,
    write: EpisodeWrite,
) -> [u8; 32] {
    let receipt = store
        .propose_write(&write)
        .unwrap_or_else(|error| panic!("{name}: the store must accept this write: {error:?}"));
    log.push(Appended {
        name: name.to_owned(),
        write,
        previous_digest: receipt.previous_digest,
        episode_digest: receipt.episode_digest,
    });
    receipt.episode_digest
}

/// Every event variant, in sequences the store admits.
fn corpus(store: &mut EpisodeStore) -> Vec<Appended> {
    let mut log = Vec::new();
    voice_chain(&mut log, store);
    short_chain(&mut log, store);
    let (candidates, pair) = memory_candidates(&mut log, store);
    memory_edges(&mut log, store, &candidates, pair);
    log
}

/// A full proposal lifecycle in the voice guild, with voice evidence; each
/// record after the first carries its predecessor as parent.
fn voice_chain(log: &mut Vec<Appended>, store: &mut EpisodeStore) {
    // A full proposal chain in the voice guild: each record after the first
    // carries its predecessor, and the header binds a consent epoch.
    let op = "op-chain-0123456789abcdef";
    append(
        log,
        store,
        "proposal",
        voice_write(
            "req-chain-1",
            op,
            EpisodeEvent::Proposal {
                requested_by: actor("member-1", ActorKind::HumanSubject),
                proposed_by: service(),
            },
        ),
    );
    append(
        log,
        store,
        "approval",
        voice_write(
            "req-chain-2",
            op,
            EpisodeEvent::Approval {
                approved_by: actor("admin-1", ActorKind::GuildAdministrator),
            },
        ),
    );
    append(
        log,
        store,
        "execution with voice",
        voice_write(
            "req-chain-3",
            op,
            EpisodeEvent::Execution {
                executed_by: service(),
                voice: Some(voice()),
            },
        ),
    );
    append(
        log,
        store,
        "compensation",
        voice_write(
            "req-chain-4",
            op,
            EpisodeEvent::Compensation {
                compensated_by: service(),
                exact_restore_observed: true,
            },
        ),
    );
    append(
        log,
        store,
        "terminal compensated",
        voice_write(
            "req-chain-5",
            op,
            EpisodeEvent::Terminal {
                status: TerminalStatus::Compensated,
                reason: TerminalReason::Completed,
            },
        ),
    );
}

/// A short lifecycle: proposal, approval, execution without voice, terminal failed.
fn short_chain(log: &mut Vec<Appended>, store: &mut EpisodeStore) {
    let op = "op-short-0123456789abcdef";
    append(
        log,
        store,
        "proposal (short)",
        write(
            "req-short-1",
            op,
            EpisodeEvent::Proposal {
                requested_by: actor("owner-1", ActorKind::GuildOwner),
                proposed_by: service(),
            },
        ),
    );
    append(
        log,
        store,
        "approval (short)",
        write(
            "req-short-2",
            op,
            EpisodeEvent::Approval {
                approved_by: actor("manager-1", ActorKind::GuildManager),
            },
        ),
    );
    append(
        log,
        store,
        "execution without voice",
        write(
            "req-short-3",
            op,
            EpisodeEvent::Execution {
                executed_by: service(),
                voice: None,
            },
        ),
    );
    append(
        log,
        store,
        "terminal failed",
        write(
            "req-short-4",
            op,
            EpisodeEvent::Terminal {
                status: TerminalStatus::Failed,
                reason: TerminalReason::ExplicitStop,
            },
        ),
    );
}

/// Memory candidates across classes and retentions, one superseding another,
/// plus a like-scoped pair for a contradiction. Returns the digests the edges need.
fn memory_candidates(
    log: &mut Vec<Appended>,
    store: &mut EpisodeStore,
) -> (Vec<[u8; 32]>, [[u8; 32]; 2]) {
    let classes = [
        (MemoryClass::Embedding, RetentionClass::Durable),
        (MemoryClass::Fact, RetentionClass::Session),
        (MemoryClass::Experience, RetentionClass::Operational),
        (MemoryClass::Summary, RetentionClass::Durable),
    ];
    let mut candidates = Vec::new();
    for (index, (class, retention)) in classes.into_iter().enumerate() {
        let seed = u8::try_from(index + 2).expect("small seed");
        candidates.push(append(
            log,
            store,
            "memory candidate",
            write(
                &format!("req-memory-{index}"),
                &format!("memory-{index}-0123456789abcdef"),
                EpisodeEvent::MemoryCandidate {
                    recorded_by: service(),
                    candidate: candidate(seed, class, retention),
                },
            ),
        ));
    }
    append(
        log,
        store,
        "memory candidate superseding another",
        write(
            "req-memory-supersedes",
            "memory-supersedes-0123456789abcdef",
            EpisodeEvent::MemoryCandidate {
                recorded_by: service(),
                candidate: MemoryCandidate {
                    supersedes: Some(candidates[3]),
                    ..candidate(9, MemoryClass::Summary, RetentionClass::Durable)
                },
            },
        ),
    );
    // A contradiction is admitted only between candidates with the same
    // `member_scoped`, so the pair differs in payload commitment alone, as the
    // golden fixture does.
    let mut pair = [[0_u8; 32]; 2];
    for (index, side) in pair.iter_mut().enumerate() {
        let seed = u8::try_from(20 + index * 2).expect("small seed");
        *side = append(
            log,
            store,
            "memory candidate (contradiction side)",
            write(
                &format!("req-memory-side-{index}"),
                &format!("memory-side-{index}-0123456789abcdef"),
                EpisodeEvent::MemoryCandidate {
                    recorded_by: service(),
                    candidate: candidate(seed, MemoryClass::Fact, RetentionClass::Durable),
                },
            ),
        );
    }
    (candidates, pair)
}

/// Every edge kind: a quarantine, a contradiction, and a resolution of the quarantine.
fn memory_edges(
    log: &mut Vec<Appended>,
    store: &mut EpisodeStore,
    candidates: &[[u8; 32]],
    pair: [[u8; 32]; 2],
) {
    let quarantine = append(
        log,
        store,
        "quarantines edge",
        write(
            "req-edge-quarantine",
            "edge-quarantine-0123456789abcdef",
            EpisodeEvent::MemoryEdge {
                recorded_by: service(),
                edge: MemoryEdge {
                    kind: MemoryEdgeKind::Quarantines,
                    target: candidates[0],
                    counterpart: None,
                    reason: EdgeReason::SourceUntrusted,
                },
            },
        ),
    );
    append(
        log,
        store,
        "contradicts edge",
        write(
            "req-edge-contradicts",
            "edge-contradicts-0123456789abcdef",
            EpisodeEvent::MemoryEdge {
                recorded_by: service(),
                edge: MemoryEdge {
                    kind: MemoryEdgeKind::Contradicts,
                    target: pair[0].min(pair[1]),
                    counterpart: Some(pair[0].max(pair[1])),
                    reason: EdgeReason::ConflictingObservation,
                },
            },
        ),
    );
    append(
        log,
        store,
        "resolves edge",
        write(
            "req-edge-resolves",
            "edge-resolves-0123456789abcdef",
            EpisodeEvent::MemoryEdge {
                recorded_by: actor("admin-1", ActorKind::GuildAdministrator),
                edge: MemoryEdge {
                    kind: MemoryEdgeKind::Resolves,
                    target: quarantine,
                    counterpart: None,
                    reason: EdgeReason::ReviewedValid,
                },
            },
        ),
    );
}

fn run_python(lines: &[String]) -> Vec<Value> {
    let mut child = Command::new(python())
        .arg(script())
        .arg("episode-differential")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|error| {
            panic!(
                "cannot start `{}` for the cross-language check (set WDBX_PYTHON to a Python 3 interpreter): {error}",
                python()
            )
        });
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        for line in lines {
            writeln!(stdin, "{line}").expect("write case to python");
        }
    }
    let output = child.wait_with_output().expect("python exits");
    assert!(
        output.status.success(),
        "python episode-differential exited with {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("python prints UTF-8");
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("python prints one JSON object per line"))
        .collect()
}

#[test]
fn python_reproduces_every_receipt_digest_the_store_appended() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let appended = corpus(&mut store);

    let lines: Vec<String> = appended
        .iter()
        .map(|record| {
            json!({
                "write": serde_json::to_value(&record.write).expect("write serialises"),
                "previous_digest": record.previous_digest.map(|digest| hex(&digest)),
            })
            .to_string()
        })
        .collect();
    let results = run_python(&lines);
    assert_eq!(
        results.len(),
        appended.len(),
        "python answered every record"
    );

    let mut chained = 0;
    for (record, result) in appended.iter().zip(results) {
        assert_eq!(
            result["ok"],
            Value::Bool(true),
            "{}: python refused the write: {result}",
            record.name
        );
        assert_eq!(
            result["digest"],
            Value::String(hex(&record.episode_digest)),
            "{}: python digest differs from the store's receipt",
            record.name
        );
        if record.previous_digest.is_some() {
            chained += 1;
        }
    }
    assert!(
        chained >= 7,
        "the corpus must exercise the parent path; saw {chained} chained records"
    );
    let kinds: std::collections::BTreeSet<&str> = appended
        .iter()
        .map(|record| match record.write.event {
            EpisodeEvent::Proposal { .. } => "proposal",
            EpisodeEvent::Approval { .. } => "approval",
            EpisodeEvent::Execution { .. } => "execution",
            EpisodeEvent::Compensation { .. } => "compensation",
            EpisodeEvent::Terminal { .. } => "terminal",
            EpisodeEvent::MemoryCandidate { .. } => "memory_candidate",
            EpisodeEvent::MemoryEdge { .. } => "memory_edge",
        })
        .collect();
    assert_eq!(
        kinds.len(),
        7,
        "every event variant must be covered: {kinds:?}"
    );
}

#[test]
fn python_re_derives_the_pinned_golden_episode_digests_standalone() {
    let status = Command::new(python())
        .arg(script())
        .arg("verify-episode-goldens")
        .status()
        .unwrap_or_else(|error| panic!("cannot start `{}`: {error}", python()));
    assert!(
        status.success(),
        "verify-episode-goldens exited with {status}"
    );
}
