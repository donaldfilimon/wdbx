//! Cross-language differential test for the `abbey-cbor-episode-v1` profile.
//!
//! `tools/abbey_cbor_episode_v1.py` is an independent, standard-library-only
//! Python reimplementation of `v3::commitment`. This test feeds the same
//! values to both encoders and requires byte-identical envelopes, identical
//! SHA-256 digests, and the same content-free error name for every refused
//! input. It is C2-style evidence for the profile encoder and the envelope
//! only: the Python side does not derive an episode's header/payload from an
//! `EpisodeWrite`, so it does not witness the store.
//!
//! The test needs `python3` on `PATH` (override with `WDBX_PYTHON`). It fails,
//! rather than silently passing, when the interpreter is missing, because a
//! skipped cross-language check would otherwise read as a green one.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use abi_wdbx::v3::commitment::{CanonicalValue, EpisodeCommitment, MAX_NESTING_DEPTH};
use serde_json::{Value, json};

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

fn interchange(value: &CanonicalValue) -> Value {
    match value {
        CanonicalValue::Unsigned(integer) => json!({ "u": integer }),
        CanonicalValue::Negative(integer) => json!({ "n": integer }),
        CanonicalValue::Bytes(bytes) => json!({ "b": hex(bytes) }),
        CanonicalValue::Text(text) => json!({ "t": text }),
        CanonicalValue::Array(items) => {
            json!({ "a": items.iter().map(interchange).collect::<Vec<_>>() })
        }
        CanonicalValue::Map(entries) => json!({
            "m": entries
                .iter()
                .map(|(key, value)| json!([interchange(key), interchange(value)]))
                .collect::<Vec<_>>()
        }),
        CanonicalValue::Bool(flag) => json!({ "bool": flag }),
        CanonicalValue::Null => json!({ "null": true }),
    }
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("write to string");
        output
    })
}

fn map(entries: Vec<(CanonicalValue, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::Map(entries)
}

fn u(integer: u64) -> CanonicalValue {
    CanonicalValue::Unsigned(integer)
}

fn n(integer: i64) -> CanonicalValue {
    CanonicalValue::Negative(integer)
}

fn t(text: &str) -> CanonicalValue {
    CanonicalValue::Text(text.into())
}

fn nested(depth: usize) -> CanonicalValue {
    let mut value = CanonicalValue::Null;
    for _ in 0..depth {
        value = CanonicalValue::Array(vec![value]);
    }
    value
}

/// A tiny deterministic generator so the random cases are reproducible from
/// the seed alone, with no dependency on a random-number crate.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

fn random_value(rng: &mut Lcg, depth: usize) -> CanonicalValue {
    const UNSIGNED_POOL: [u64; 9] = [0, 1, 23, 24, 255, 256, 65_535, 65_536, u64::MAX];
    const NEGATIVE_POOL: [i64; 6] = [-1, -24, -25, -256, -257, i64::MIN];
    const TEXT_POOL: [&str; 6] = ["", "a", "ab", "naïve", "🦀", "synthetic"];

    let leaf_only = depth >= 4;
    match rng.below(if leaf_only { 6 } else { 8 }) {
        0 => u(UNSIGNED_POOL[usize::try_from(rng.below(9)).expect("index")]),
        1 => u(rng.next()),
        2 => {
            // Includes zero one time in eight, which the profile must refuse.
            if rng.below(8) == 0 {
                n(0)
            } else {
                n(NEGATIVE_POOL[usize::try_from(rng.below(6)).expect("index")])
            }
        }
        3 => t(TEXT_POOL[usize::try_from(rng.below(6)).expect("index")]),
        4 => {
            let length = usize::try_from(rng.below(40)).expect("length");
            CanonicalValue::Bytes(
                (0..length)
                    .map(|_| u8::try_from(rng.below(256)).expect("byte"))
                    .collect(),
            )
        }
        5 => match rng.below(3) {
            0 => CanonicalValue::Bool(false),
            1 => CanonicalValue::Bool(true),
            _ => CanonicalValue::Null,
        },
        6 => {
            let length = usize::try_from(rng.below(4)).expect("length");
            CanonicalValue::Array((0..length).map(|_| random_value(rng, depth + 1)).collect())
        }
        _ => {
            // Keys come from a small pool so duplicates occur and both sides
            // must refuse them identically.
            let length = usize::try_from(rng.below(5)).expect("length");
            map((0..length)
                .map(|_| {
                    let key = match rng.below(4) {
                        0 => u(rng.below(3)),
                        1 => t(TEXT_POOL[usize::try_from(rng.below(3)).expect("index")]),
                        2 => n(-1),
                        _ => CanonicalValue::Bytes(vec![0x01]),
                    };
                    (key, random_value(rng, depth + 1))
                })
                .collect())
        }
    }
}

/// The inputs a case was built from, so the Python side sees exactly what the
/// Rust side was given (including unsorted parents and duplicate keys).
struct CaseInput {
    schema_version: u64,
    header: CanonicalValue,
    payload: CanonicalValue,
    parents: Vec<[u8; 32]>,
}

impl CaseInput {
    fn commitment(&self) -> EpisodeCommitment {
        EpisodeCommitment::new(
            self.schema_version,
            self.header.clone(),
            self.payload.clone(),
            self.parents.clone(),
        )
    }

    fn line(&self) -> String {
        json!({
            "schema_version": self.schema_version,
            "header": interchange(&self.header),
            "payload": interchange(&self.payload),
            "parent_digests": self.parents.iter().map(|parent| hex(parent)).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

fn run_python(lines: &[String]) -> Vec<Value> {
    let mut child = Command::new(python())
        .arg(script())
        .arg("differential")
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
        "python differential exited with {}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("python prints UTF-8");
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("python prints one JSON object per line"))
        .collect()
}

#[test]
fn python_reimplementation_agrees_on_bytes_digests_and_errors() {
    let inputs = all_inputs();
    let lines: Vec<String> = inputs.iter().map(|(_, input)| input.line()).collect();
    let results = run_python(&lines);
    assert_eq!(results.len(), inputs.len(), "python answered every case");

    let mut errors_seen = 0;
    for ((name, input), result) in inputs.iter().zip(results) {
        let commitment = input.commitment();
        match commitment.canonical_bytes() {
            Ok(bytes) => {
                assert_eq!(
                    result["ok"],
                    Value::Bool(true),
                    "{name}: python refused a value rust accepted: {result}"
                );
                assert_eq!(
                    result["bytes"],
                    Value::String(hex(&bytes)),
                    "{name}: envelope bytes differ"
                );
                let digest = commitment.digest().expect("digest of encodable value");
                assert_eq!(
                    result["digest"],
                    Value::String(hex(&digest)),
                    "{name}: digests differ"
                );
            }
            Err(error) => {
                errors_seen += 1;
                assert_eq!(
                    result["ok"],
                    Value::Bool(false),
                    "{name}: python accepted a value rust refused: {result}"
                );
                assert_eq!(
                    result["error"],
                    Value::String(format!("{error:?}")),
                    "{name}: error names differ"
                );
            }
        }
    }
    assert!(
        errors_seen >= 6,
        "the corpus must exercise every refusal path; saw {errors_seen}"
    );
}

#[test]
fn python_verifies_the_golden_vectors_standalone() {
    let status = Command::new(python())
        .arg(script())
        .arg("verify-goldens")
        .status()
        .unwrap_or_else(|error| panic!("cannot start `{}`: {error}", python()));
    assert!(status.success(), "verify-goldens exited with {status}");
}

fn all_inputs() -> Vec<(String, CaseInput)> {
    let mut inputs = Vec::new();
    inputs.extend(accepted_inputs());
    inputs.extend(boundary_inputs());
    inputs.extend(refused_inputs());
    let mut rng = Lcg(0x5eed_0000_abbe_0001);
    for index in 0..96 {
        let header = map((0..rng.below(4))
            .map(|key| (u(key), random_value(&mut rng, 2)))
            .collect());
        let payload = map((0..rng.below(4))
            .map(|key| {
                (
                    t(["p", "q", "r", "s"][usize::try_from(key).expect("index")]),
                    random_value(&mut rng, 2),
                )
            })
            .collect());
        let parents = (0..rng.below(4))
            .map(|_| [u8::try_from(rng.below(256)).expect("byte"); 32])
            .collect();
        inputs.push((
            format!("random #{index}"),
            CaseInput {
                schema_version: 1 + rng.below(3),
                header,
                payload,
                parents,
            },
        ));
    }
    inputs
}

/// The payload every synthetic fixture shares, `{1: "synthetic"}`.
fn synthetic_payload() -> CanonicalValue {
    map(vec![(u(1), t("synthetic"))])
}

/// A schema-1 input with the synthetic payload, named for the report.
fn synthetic_input(
    name: &str,
    header: CanonicalValue,
    parents: Vec<[u8; 32]>,
) -> (String, CaseInput) {
    (
        name.to_owned(),
        CaseInput {
            schema_version: 1,
            header,
            payload: synthetic_payload(),
            parents,
        },
    )
}

/// Inputs the profile accepts; both sides must produce identical bytes and digests.
fn accepted_inputs() -> Vec<(String, CaseInput)> {
    vec![
        synthetic_input("golden empty-parents", map(Vec::new()), Vec::new()),
        synthetic_input(
            "golden two-parents supplied in reverse",
            map(Vec::new()),
            vec![[0xee; 32], [0x11; 32]],
        ),
        synthetic_input(
            "duplicate parents preserved",
            map(Vec::new()),
            vec![[0x44; 32], [0x44; 32]],
        ),
        synthetic_input(
            "map keys sort by encoded length then bytes",
            map(vec![
                (t("aa"), CanonicalValue::Null),
                (u(24), CanonicalValue::Null),
                (t("b"), CanonicalValue::Null),
                (u(1), CanonicalValue::Null),
                (n(-1), CanonicalValue::Bool(true)),
                (
                    CanonicalValue::Bytes(vec![0x00]),
                    CanonicalValue::Bool(false),
                ),
                (CanonicalValue::Array(vec![u(1)]), u(9)),
            ]),
            Vec::new(),
        ),
        synthetic_input(
            "utf-8 text is committed as supplied bytes",
            map(vec![
                (t("naïve"), t("Ångström")),
                (t("🦀"), t("")),
                (t("e\u{301}"), t("\u{e9}")),
            ]),
            Vec::new(),
        ),
        synthetic_input(
            "empty and long byte strings",
            map(vec![
                (u(0), CanonicalValue::Bytes(Vec::new())),
                (u(1), CanonicalValue::Bytes(vec![0xab; 23])),
                (u(2), CanonicalValue::Bytes(vec![0xcd; 24])),
                (u(3), CanonicalValue::Bytes(vec![0xef; 300])),
            ]),
            Vec::new(),
        ),
        synthetic_input(
            "nested arrays and maps within the limit",
            map(vec![
                (u(0), nested(MAX_NESTING_DEPTH - 3)),
                (
                    u(1),
                    map(vec![(
                        t("inner"),
                        CanonicalValue::Array(vec![map(Vec::new())]),
                    )]),
                ),
            ]),
            Vec::new(),
        ),
        (
            "schema version above one byte".to_owned(),
            CaseInput {
                schema_version: 70_000,
                header: map(Vec::new()),
                payload: synthetic_payload(),
                parents: vec![[0x01; 32]],
            },
        ),
    ]
}

/// Integer arguments at every width boundary of the shortest-encoding rule.
fn boundary_inputs() -> Vec<(String, CaseInput)> {
    vec![
        synthetic_input(
            "unsigned boundaries",
            map(vec![
                (u(0), u(23)),
                (u(1), u(24)),
                (u(2), u(255)),
                (u(3), u(256)),
                (u(4), u(65_535)),
                (u(5), u(65_536)),
                (u(6), u(0xffff_ffff)),
                (u(7), u(0x1_0000_0000)),
                (u(8), u(u64::MAX)),
            ]),
            Vec::new(),
        ),
        synthetic_input(
            "negative boundaries",
            map(vec![
                (u(0), n(-1)),
                (u(1), n(-24)),
                (u(2), n(-25)),
                (u(3), n(-256)),
                (u(4), n(-257)),
                (u(5), n(-65_537)),
                (u(6), n(-4_294_967_297)),
                (u(7), n(i64::MIN)),
            ]),
            Vec::new(),
        ),
    ]
}

/// Inputs the profile must refuse, with the error name both sides must agree on.
fn refused_inputs() -> Vec<(String, CaseInput)> {
    vec![
        (
            "error: zero schema version".to_owned(),
            CaseInput {
                schema_version: 0,
                header: map(Vec::new()),
                payload: map(Vec::new()),
                parents: Vec::new(),
            },
        ),
        (
            "error: header must be a map".to_owned(),
            CaseInput {
                schema_version: 1,
                header: CanonicalValue::Null,
                payload: map(Vec::new()),
                parents: Vec::new(),
            },
        ),
        (
            "error: payload must be a map".to_owned(),
            CaseInput {
                schema_version: 1,
                header: map(Vec::new()),
                payload: u(1),
                parents: Vec::new(),
            },
        ),
        synthetic_input(
            "error: non-negative negative variant",
            map(vec![(u(0), n(0))]),
            Vec::new(),
        ),
        synthetic_input(
            "error: duplicate encoded map keys",
            map(vec![
                (t("k"), CanonicalValue::Null),
                (t("k"), CanonicalValue::Bool(true)),
            ]),
            Vec::new(),
        ),
        synthetic_input(
            "error: nesting limit",
            map(vec![(u(0), nested(MAX_NESTING_DEPTH + 1))]),
            Vec::new(),
        ),
    ]
}
