//! Rust↔Python differential for the complete-episode typed decoder witness
//! (`tools/wdbx_complete_episode_decode.py`), the Task-4 slice of
//! `docs/superpowers/plans/2026-10-20-complete-episode-python-decoder.md`.
//!
//! Covered here: strict protocol checks (exit status, response count, exact
//! response schema, duplicate-key rejection, byte echo, SHA-256 digest, no
//! trailing junk); typed agreement on the positive corpus through
//! `draft_adapter::input` + `decode_normalize::normalize` against
//! `DecodedCompleteEpisode::input()` on the same original bytes; every
//! truncation point of both pinned corpora; representative trailing,
//! nonminimal-argument, reserved-information, envelope reorder/duplicate/
//! missing/extra-key and sorted-set reversal mutations with categories
//! computed live from the Rust decoder; the caller-limit zeros plus the
//! 1 MiB+1 clamp with huge caller caps on both sides; and python-only
//! ingress rows. Expectations come from the original fixtures and the Rust
//! decoder before the witness runs; the witness must agree. Missing Python
//! fails the suite, it is never skipped, and no other witness behavior is
//! relaxed.
//!
//! All previously open acceptance rows are now covered by
//! `python_decode_mutations`: exhaustive leaf mutation census
//! (`exhaustive_leaf_mutations_agree`), relational-slot mutation census
//! (`relational_slot_mutations_agree`), complete draft-corpus breadth
//! (`draft_corpus_breadth_agrees`) and ingress category census
//! (`ingress_category_census_agrees`). Reader categories that are genuinely
//! unreachable for this envelope (e.g. ParentLimit, which is inert) are
//! documented as such rather than fabricated.
use super::decode_malformed::tree;
use super::decode_normalize::normalize;
use super::fixtures::{minimal, populated};
use super::{CompleteEpisodeDecodeError as E, DecodedCompleteEpisode};
use super::{bytes, draft_adapter, draft_shape};
use crate::v3::commitment::{CanonicalDecodeError, CanonicalDecodeLimits, CanonicalValue as C};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
};

type R<T> = Result<T, String>;

/// The reader error words mirrored one-to-one by the Python witness. Every
/// other reader category is unreachable from this decoder's inputs.
fn reader_word(error: &CanonicalDecodeError) -> Option<&'static str> {
    match error {
        CanonicalDecodeError::InputLimit => Some("InputLimit"),
        CanonicalDecodeError::ValueLimit => Some("ValueLimit"),
        CanonicalDecodeError::CollectionLimit => Some("CollectionLimit"),
        CanonicalDecodeError::StringLimit => Some("StringLimit"),
        CanonicalDecodeError::ParentLimit => Some("ParentLimit"),
        CanonicalDecodeError::UnexpectedEnd => Some("UnexpectedEnd"),
        CanonicalDecodeError::TrailingBytes => Some("TrailingBytes"),
        CanonicalDecodeError::InvalidAdditionalInformation => Some("InvalidAdditionalInformation"),
        CanonicalDecodeError::NonMinimalArgument => Some("NonMinimalArgument"),
        CanonicalDecodeError::UnsupportedType => Some("UnsupportedType"),
        CanonicalDecodeError::InvalidUtf8 => Some("InvalidUtf8"),
        CanonicalDecodeError::IntegerOutOfRange => Some("IntegerOutOfRange"),
        CanonicalDecodeError::NestingLimit => Some("NestingLimit"),
        CanonicalDecodeError::LengthOutOfRange => Some("LengthOutOfRange"),
        CanonicalDecodeError::DuplicateMapKey => Some("DuplicateMapKey"),
        CanonicalDecodeError::MapKeyOrder => Some("MapKeyOrder"),
        _ => None,
    }
}

/// The typed-error words mirrored one-to-one by the Python witness.
/// `EncodedSizeLimit` and `Encoding` have no Python counterpart and are
/// unreachable from decoding.
fn episode_word(error: &crate::v3::complete_episode::CompleteEpisodeError) -> Option<&'static str> {
    use crate::v3::complete_episode::CompleteEpisodeError;
    match error {
        CompleteEpisodeError::InvalidIdentifier => Some("InvalidIdentifier"),
        CompleteEpisodeError::InvalidUuid => Some("InvalidUuid"),
        CompleteEpisodeError::InvalidRange => Some("InvalidRange"),
        CompleteEpisodeError::DuplicateReference => Some("DuplicateReference"),
        CompleteEpisodeError::ScopeMismatch => Some("ScopeMismatch"),
        CompleteEpisodeError::CollectionLimit => Some("CollectionLimit"),
        CompleteEpisodeError::InvalidPosterior => Some("InvalidPosterior"),
        CompleteEpisodeError::SelfReference => Some("SelfReference"),
        CompleteEpisodeError::InvalidArtifactKind => Some("InvalidArtifactKind"),
        _ => None,
    }
}

/// Map the Rust decode result onto the exact Python response category, or
/// `"Ok"` on acceptance. Divergence between the two sides is a test failure,
/// never a silent skip.
fn category(result: std::result::Result<DecodedCompleteEpisode, E>) -> String {
    match result {
        Ok(_) => "Ok".to_owned(),
        Err(E::Canonical(reader)) => format!(
            "Canonical:{}",
            reader_word(&reader).expect("reachable reader category")
        ),
        Err(E::InvalidShape) => "InvalidShape".to_owned(),
        Err(E::UnsupportedDomain) => "UnsupportedDomain".to_owned(),
        Err(E::UnsupportedVersion) => "UnsupportedVersion".to_owned(),
        Err(E::UnknownVariant) => "UnknownVariant".to_owned(),
        Err(E::InvalidValue(error)) => format!(
            "InvalidValue:{}",
            episode_word(&error).expect("reachable episode category")
        ),
        Err(E::NonCanonicalEncoding) => "NonCanonicalEncoding".to_owned(),
    }
}

fn hex(data: &[u8]) -> String {
    data.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(&mut text, "{byte:02x}");
        text
    })
}

fn interpreter() -> String {
    std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".to_owned())
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/wdbx_complete_episode_decode.py")
}

/// One strictly parsed witness response line; shape errors, duplicate keys,
/// invalid JSON and trailing junk all surface as `Err`.
#[derive(Debug)]
enum DecodeResponse {
    Success {
        bytes_hex: String,
        digest_hex: String,
        fixture: Value,
    },
    Invalid {
        error: String,
    },
}

fn trimmed(line: &str) -> String {
    if line.len() > 160 {
        format!("{}…", &line[..160])
    } else {
        line.to_owned()
    }
}

fn parse_response(line: &str) -> R<DecodeResponse> {
    let value = draft_shape::parse(line)
        .map_err(|error| format!("witness JSON rejected ({error}): {}", trimmed(line)))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("witness response is not a JSON object: {}", trimmed(line)))?;
    let ok = object
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("witness response lacks a boolean ok: {}", trimmed(line)))?;
    if !ok {
        let error = object
            .get("error")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("witness failure lacks an error word: {}", trimmed(line)))?;
        if object.len() != 2 {
            return Err(format!(
                "witness failure shape is not exactly {{ok, error}}: {}",
                trimmed(line)
            ));
        }
        return Ok(DecodeResponse::Invalid {
            error: error.to_owned(),
        });
    }
    let bytes_hex = object
        .get("bytes")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("witness success lacks bytes: {}", trimmed(line)))?
        .to_owned();
    let digest_hex = object
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("witness success lacks digest: {}", trimmed(line)))?
        .to_owned();
    let fixture = object
        .get("fixture")
        .cloned()
        .ok_or_else(|| format!("witness success lacks fixture: {}", trimmed(line)))?;
    if object.len() != 4 {
        return Err(format!(
            "witness success shape is not exactly {{ok, fixture, bytes, digest}}: {}",
            trimmed(line)
        ));
    }
    Ok(DecodeResponse::Success {
        bytes_hex,
        digest_hex,
        fixture,
    })
}

/// Spawn one witness protocol run over a whole request batch: write every
/// request line from a scoped writer thread, drain stdout concurrently, then
/// strictly check exit status, UTF-8, per-line shape and response count.
fn run_batch(lines: &[String]) -> R<Vec<DecodeResponse>> {
    let mut command = Command::new(interpreter());
    command
        .arg(script())
        .arg("decode-json-lines")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        format!(
            "cannot start `{}` for the complete-episode decode witness (set \
             WDBX_PYTHON to a Python 3 interpreter): {error}",
            interpreter()
        )
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "witness lost stdin".to_owned())?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for line in lines {
                writeln!(stdin, "{line}")
                    .map_err(|error| format!("witness stdin failed: {error}"))?;
            }
            Ok::<_, String>(())
        });
        let output = child
            .wait_with_output()
            .map_err(|error| format!("witness wait failed: {error}"));
        writer
            .join()
            .map_err(|_| "witness writer panicked".to_owned())?
            .map_err(|error| format!("witness stdin still failing: {error}"))?;
        output
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(format!(
            "witness exited with {output:?}; stderr: {}",
            stderr.trim()
        ));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| format!("witness stdout is not UTF-8; stderr: {}", stderr.trim()))?;
    let mut responses = Vec::with_capacity(lines.len());
    for line in text.lines() {
        responses.push(parse_response(line)?);
    }
    if responses.len() != lines.len() {
        return Err(format!(
            "witness emitted {} responses for {} requests",
            responses.len(),
            lines.len()
        ));
    }
    Ok(responses)
}

/// Caller limits shared between the two decoders: the five keys mirror the
/// `CanonicalDecodeLimits` field names exactly, so one builder drives both
/// sides. Missing keys keep each side's defaults.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Limits {
    pub max_input_bytes: Option<usize>,
    pub max_value_count: Option<usize>,
    pub max_collection_entries: Option<usize>,
    pub max_string_bytes: Option<usize>,
    pub max_parent_count: Option<usize>,
}

impl Limits {
    pub(super) fn rust(self) -> CanonicalDecodeLimits {
        let defaults = CanonicalDecodeLimits::default();
        CanonicalDecodeLimits {
            max_input_bytes: self.max_input_bytes.unwrap_or(defaults.max_input_bytes),
            max_value_count: self.max_value_count.unwrap_or(defaults.max_value_count),
            max_collection_entries: self
                .max_collection_entries
                .unwrap_or(defaults.max_collection_entries),
            max_string_bytes: self.max_string_bytes.unwrap_or(defaults.max_string_bytes),
            max_parent_count: self.max_parent_count.unwrap_or(defaults.max_parent_count),
        }
    }

    fn json(self) -> Option<Value> {
        let mut object = serde_json::Map::new();
        for (key, value) in [
            ("max_input_bytes", self.max_input_bytes),
            ("max_value_count", self.max_value_count),
            ("max_collection_entries", self.max_collection_entries),
            ("max_string_bytes", self.max_string_bytes),
            ("max_parent_count", self.max_parent_count),
        ] {
            if let Some(value) = value {
                object.insert(key.to_owned(), json!(value));
            }
        }
        (!object.is_empty()).then(|| Value::Object(object))
    }
}

const HUGEST_LEGAL_CAP: usize = (1_u128 << 63) as usize - 1;

fn limits_all_huge() -> Limits {
    // The Python witness accepts limit integers up to 2**63-1 (its portable
    // policy ceiling); larger requests become RequestError by design.
    // Both decoders clamp max_input_bytes at 1 MiB, so these caps still
    // exercise the clamp semantics.
    Limits {
        max_input_bytes: Some(HUGEST_LEGAL_CAP),
        max_value_count: Some(HUGEST_LEGAL_CAP),
        max_collection_entries: Some(HUGEST_LEGAL_CAP),
        max_string_bytes: Some(HUGEST_LEGAL_CAP),
        max_parent_count: Some(HUGEST_LEGAL_CAP),
    }
}

#[derive(Debug)]
pub(super) enum Kind {
    /// The witness must accept with an exactly echoing byte field, the
    /// independently recomputed SHA-256 digest, and a fixture that projects
    /// through `draft_adapter::input` to the Rust decoder's typed input.
    Agree { bytes: Vec<u8>, limits: Limits },
    /// The witness must return exactly the category the Rust decoder returns
    /// for the same bytes and limits; the builder may additionally pin the
    /// single-defect category deterministically.
    Reject {
        bytes: Vec<u8>,
        limits: Limits,
        rust_category: String,
    },
    /// Python-only ingress behavior with no Rust counterpart; the Rust side
    /// has no request-line or JSON-request layer.
    IngressOnly { line: String, error: &'static str },
}

pub(super) struct Case {
    name: String,
    kind: Kind,
}

/// A positive case fails before the witness even runs unless the Rust decoder
/// independently accepts the same bytes and limits.
pub(super) fn agree(name: &str, data: Vec<u8>, limits: Limits) -> Case {
    assert!(
        category(DecodedCompleteEpisode::from_canonical_bytes(
            &data,
            limits.rust()
        )) == "Ok",
        "positive case {name} must decode on the Rust side first"
    );
    Case {
        name: name.to_owned(),
        kind: Kind::Agree {
            bytes: data,
            limits,
        },
    }
}

/// A negative case computes its Rust category live; `expected` additionally
/// pins the single-defect category deterministically.
pub(super) fn reject(
    name: &str,
    data: Vec<u8>,
    limits: Limits,
    expected: Option<&'static str>,
) -> Case {
    let rust_category = category(DecodedCompleteEpisode::from_canonical_bytes(
        &data,
        limits.rust(),
    ));
    assert!(
        rust_category != "Ok",
        "negative case {name} must reject on the Rust side first"
    );
    if let Some(expected) = expected {
        assert_eq!(
            rust_category, expected,
            "negative case {name} pins a single-defect category the Rust side must keep"
        );
    }
    Case {
        name: name.to_owned(),
        kind: Kind::Reject {
            bytes: data,
            limits,
            rust_category,
        },
    }
}

fn request_line(data: &[u8], limits: Limits) -> String {
    let mut request = json!({"cbor_hex": hex(data)});
    if let Some(limits) = limits.json() {
        request["limits"] = limits;
    }
    request.to_string()
}

/// Run the batch and enforce every independent strictness layer per response.
pub(super) fn execute(cases: Vec<Case>) {
    let lines = cases
        .iter()
        .map(|case| match &case.kind {
            Kind::Agree { bytes, limits } | Kind::Reject { bytes, limits, .. } => {
                request_line(bytes, *limits)
            }
            Kind::IngressOnly { line, .. } => line.clone(),
        })
        .collect::<Vec<_>>();
    let responses = run_batch(&lines).unwrap_or_else(|error| panic!("witness run failed: {error}"));
    for (case, response) in cases.iter().zip(responses) {
        match (&case.kind, response) {
            (
                Kind::Agree { bytes, limits, .. },
                DecodeResponse::Success {
                    bytes_hex,
                    digest_hex,
                    fixture,
                },
            ) => {
                assert_eq!(
                    bytes_hex,
                    hex(bytes),
                    "case {} must echo the original bytes exactly",
                    case.name
                );
                assert_eq!(
                    digest_hex,
                    hex(&Sha256::digest(bytes)),
                    "case {} must report SHA-256 of the original bytes",
                    case.name
                );
                let typed = DecodedCompleteEpisode::from_canonical_bytes(bytes, limits.rust())
                    .expect("positive case re-check");
                let projected = draft_adapter::input(&fixture).unwrap_or_else(|error| {
                    panic!("case {} fixture failed to project: {error}", case.name)
                });
                assert_eq!(
                    typed.input(),
                    &normalize(projected),
                    "case {} python fixture must type-agree with the Rust decoder",
                    case.name
                );
            }
            (Kind::Reject { rust_category, .. }, DecodeResponse::Invalid { error }) => assert_eq!(
                error, *rust_category,
                "case {} python category must equal the Rust category",
                case.name
            ),
            (Kind::IngressOnly { error, .. }, DecodeResponse::Invalid { error: got }) => {
                assert_eq!(got, *error, "case {} ingress category", case.name)
            }
            (kind, response) => panic!(
                "case {} got an unexpected response kind: {response:?} vs {kind:?}",
                case.name
            ),
        }
    }
}

/// Path indexes used by the mutation walker: a map-entry index or an
/// array-element index.
#[derive(Clone, Copy)]
pub(super) enum Step {
    Map(usize),
    Array(usize),
}

fn reverse_array_entry(value: &mut C) {
    match value {
        C::Array(values) => values.reverse(),
        _ => panic!("seeking an array"),
    }
}

/// Read-only path search for a text key, so mutations need no hard-coded
/// group indexes.
fn find_key(value: &C, key: &str, path: &[Step], out: &mut Vec<Vec<Step>>) {
    match value {
        C::Map(entries) => {
            for (position, (k, child)) in entries.iter().enumerate() {
                let mut extended = path.to_vec();
                extended.push(Step::Map(position));
                if matches!(k, C::Text(text) if text == key) {
                    out.push(extended.clone());
                }
                find_key(child, key, &extended, out);
            }
        }
        C::Array(values) => {
            for (position, child) in values.iter().enumerate() {
                let mut extended = path.to_vec();
                extended.push(Step::Array(position));
                find_key(child, key, &extended, out);
            }
        }
        _ => {}
    }
}

pub(super) fn descend<'a>(mut value: &'a mut C, path: &[Step]) -> &'a mut C {
    for step in path {
        value = match step {
            Step::Map(position) => match &mut *value {
                C::Map(entries) => &mut entries[*position].1,
                _ => panic!("descending through a non-map"),
            },
            Step::Array(position) => match &mut *value {
                C::Array(values) => &mut values[*position],
                _ => panic!("descending through a non-array"),
            },
        };
    }
    value
}

pub(super) fn leaves(value: &C, path: Vec<Step>, output: &mut Vec<Vec<Step>>) {
    match value {
        C::Map(entries) => {
            for (i, (_, v)) in entries.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Map(i));
                leaves(v, p, output);
            }
        }
        C::Array(values) => {
            for (i, v) in values.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Array(i));
                leaves(v, p, output);
            }
        }
        _ => output.push(path),
    }
}

/// Locate the version argument in a canonical envelope's hexadecimal form.
/// The envelope is `0xac | key 0 | 0x78 0x18 <24-byte domain> | key 1 | arg`;
/// `7818<domain>` must appear exactly once.
fn version_arg_span(hexes: &str) -> usize {
    let domain = hex(b"wdbx-complete-episode-v1");
    let anchor = format!("7818{domain}");
    let first = hexes.find(&anchor).expect("domain anchor");
    assert_eq!(
        hexes.rfind(&anchor),
        Some(first),
        "domain anchor must be unique"
    );
    let anchor_end = first + anchor.len();
    assert_eq!(&hexes[anchor_end..anchor_end + 2], "01", "envelope key 1");
    assert_eq!(
        &hexes[anchor_end + 2..anchor_end + 4],
        "01",
        "version argument"
    );
    anchor_end + 2
}

fn hexes_to_bytes(hexes: &str) -> Vec<u8> {
    assert!(hexes.len().is_multiple_of(2));
    (0..hexes.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hexes[at..at + 2], 16).unwrap())
        .collect()
}

#[test]
fn positive_typed_agreement() {
    let mut corpus = Vec::new();
    corpus.push(agree("minimal", bytes(minimal()), Limits::default()));
    corpus.push(agree("populated", bytes(populated()), Limits::default()));
    // Reversing the action sequence stays type-valid with correspondingly
    // different typed order: decode accepts and the fixture projects reversed.
    let mut actions_reversed = tree(populated());
    let mut paths = Vec::new();
    find_key(&actions_reversed, "actions", &[], &mut paths);
    assert_eq!(
        paths.len(),
        1,
        "exactly one action array in the populated corpus"
    );
    reverse_array_entry(descend(&mut actions_reversed, &paths[0]));
    corpus.push(agree(
        "populated/actions-reversed",
        actions_reversed.canonical_bytes().unwrap(),
        Limits::default(),
    ));
    // max_parent_count is inert for this envelope even at zero.
    corpus.push(agree(
        "minimal/parent-limit-zero-inert",
        bytes(minimal()),
        Limits {
            max_parent_count: Some(0),
            ..Limits::default()
        },
    ));
    // Huge caller caps retain whole-corpus acceptance.
    corpus.push(agree(
        "populated/all-huge-caps",
        bytes(populated()),
        limits_all_huge(),
    ));
    execute(corpus);
}

#[test]
fn every_truncation_point_rejects_categorically() {
    let mut corpus = Vec::new();
    for (label, data) in [
        ("minimal", bytes(minimal())),
        ("populated", bytes(populated())),
    ] {
        for length in 0..data.len() {
            corpus.push(reject(
                &format!("{label}/truncation@{length}"),
                data[..length].to_vec(),
                Limits::default(),
                Some("Canonical:UnexpectedEnd"),
            ));
        }
    }
    execute(corpus);
}

#[test]
fn trailing_and_argument_encoding_reject() {
    let base = bytes(populated());
    let mut corpus = Vec::new();
    let mut one_extra = base.clone();
    one_extra.push(0);
    corpus.push(reject(
        "trailing+1",
        one_extra,
        Limits::default(),
        Some("Canonical:TrailingBytes"),
    ));
    let mut two_extra = base.clone();
    two_extra.extend_from_slice(&[0, 0]);
    corpus.push(reject(
        "trailing+2",
        two_extra,
        Limits::default(),
        Some("Canonical:TrailingBytes"),
    ));
    // Non-minimal version argument: a width-one encoding replaces the minimal
    // single byte without changing the decoded value.
    let hexes = hex(&base);
    let argument = version_arg_span(&hexes);
    let nonminimal = format!("{}1801{}", &hexes[..argument], &hexes[argument + 2..]);
    corpus.push(reject(
        "version/arg-nonminimal",
        hexes_to_bytes(&nonminimal),
        Limits::default(),
        Some("Canonical:NonMinimalArgument"),
    ));
    // Reserved additional information on the version argument (one byte in
    // place of the two-byte minimal argument).
    let reserved = format!("{}1c{}", &hexes[..argument], &hexes[argument + 2..]);
    corpus.push(reject(
        "version/arg-reserved-info",
        hexes_to_bytes(&reserved),
        Limits::default(),
        Some("Canonical:InvalidAdditionalInformation"),
    ));
    execute(corpus);
}

/// Canonical item bound walker over `decode_malformed`'s canonical-tree
/// fixture bytes (no indefinite, tag, float or reserved info there).
fn item_end(data: &[u8], start: usize) -> usize {
    let first = data[start];
    let argument = |info: u8| match info {
        0..=23 => (info as u64, start + 1),
        24 => (data[start + 1] as u64, start + 2),
        25 => (
            u16::from_be_bytes(data[start + 1..start + 3].try_into().unwrap()) as u64,
            start + 3,
        ),
        26 => (
            u32::from_be_bytes(data[start + 1..start + 5].try_into().unwrap()) as u64,
            start + 5,
        ),
        27 => (
            u64::from_be_bytes(data[start + 1..start + 9].try_into().unwrap()),
            start + 9,
        ),
        _ => panic!("indefinite or reserved argument in a canonical fixture"),
    };
    let info = first & 0x1f;
    match first >> 5 {
        0 | 1 | 7 => argument(info).1,
        2 | 3 => {
            let (count, after) = argument(info);
            after + count as usize
        }
        4 => {
            let (count, after) = argument(info);
            (0..count).fold(after, |end, _| item_end(data, end))
        }
        5 => {
            let (count, after) = argument(info);
            (0..count * 2).fold(after, |end, _| item_end(data, end))
        }
        _ => panic!("tagged fixture item"),
    }
}

/// Per-entry byte spans `(start, value_end)` of the pristine twelve-entry
/// envelope; mutations are then performed by raw byte surgery, because the
/// generic tree encoder re-canonicalizes map entry order silently.
fn envelope_entry_spans(data: &[u8]) -> Vec<(usize, usize)> {
    assert_eq!(data[0] >> 5, 5, "envelope root is a map");
    assert_eq!(data[0] & 0x1f, 12, "pristine envelope declares 12 entries");
    let mut spans = Vec::with_capacity(12);
    let mut position = 1;
    for _ in 0..12 {
        assert_eq!(data[position] >> 5, 0, "envelope keys are unsigned");
        assert!(
            data[position] & 0x1f < 24,
            "envelope keys are minimally encoded"
        );
        let end = item_end(data, position + 1);
        spans.push((position, end));
        position = end;
    }
    assert_eq!(
        position,
        data.len(),
        "envelope entries consume the whole form"
    );
    spans
}

fn envelope_from(header_count: u8, entries: &[&[u8]]) -> Vec<u8> {
    assert!(header_count <= 23, "minimal map argument");
    let mut data = vec![0xa0 | header_count];
    for entry in entries {
        data.extend_from_slice(entry);
    }
    data
}

fn envelope_entries_of(base: &[u8]) -> Vec<&[u8]> {
    envelope_entry_spans(base)
        .into_iter()
        .map(|(start, end)| &base[start..end])
        .collect()
}

#[test]
fn envelope_order_arity_and_set_reversal() {
    let mut corpus = Vec::new();
    // Swapped envelope keys 1 and 2 break reader-level canonical order; byte
    // surgery preserves the malformed order the general tree would re-sort.
    let base = bytes(minimal());
    let entries = envelope_entries_of(&base);
    let mut swapped_entries = entries.clone();
    swapped_entries.swap(1, 2);
    corpus.push(reject(
        "envelope/keys-swapped",
        envelope_from(12, &swapped_entries),
        Limits::default(),
        Some("Canonical:MapKeyOrder"),
    ));
    // Duplicating the first entry (domain) is reader-level DuplicateMapKey.
    let mut duplicated_entries = entries.clone();
    duplicated_entries.insert(1, entries[0]);
    corpus.push(reject(
        "envelope/first-entry-duplicated",
        envelope_from(13, &duplicated_entries),
        Limits::default(),
        Some("Canonical:DuplicateMapKey"),
    ));
    // A dropped envelope key under an 11-entry header is a shape rejection;
    // the category is computed live and enforced independently on both sides.
    let missing = envelope_from(11, &entries[..11]);
    corpus.push(reject(
        "envelope/last-key-dropped",
        missing,
        Limits::default(),
        None,
    ));
    // Extra envelope key 12 under a 13-entry header with a scalar value,
    // also live-computed category.
    let mut extra_entries = entries.clone();
    extra_entries.push(&[0x0c, 0x00]);
    let extra = envelope_from(13, &extra_entries);
    corpus.push(reject(
        "envelope/key-12-added",
        extra,
        Limits::default(),
        None,
    ));
    // Reversing a sorted set array: canonical set order is enforced at the
    // typed layer, so both decoders must call the result noncanonical.
    let mut consulted = tree(populated());
    let mut paths = Vec::new();
    find_key(&consulted, "consulted_parents", &[], &mut paths);
    assert_eq!(
        paths.len(),
        1,
        "exactly one consulted set in the populated corpus"
    );
    reverse_array_entry(descend(&mut consulted, &paths[0]));
    corpus.push(reject(
        "consulted/reversed",
        consulted.canonical_bytes().unwrap(),
        Limits::default(),
        Some("NonCanonicalEncoding"),
    ));
    execute(corpus);
}

#[test]
fn caller_limit_zeros_agree_on_both_sides() {
    let base = bytes(minimal());
    let zero = Limits {
        max_input_bytes: Some(0),
        ..Limits::default()
    };
    let mut corpus = vec![reject(
        "max_input_bytes=0",
        base.clone(),
        zero,
        Some("Canonical:InputLimit"),
    )];
    for (key, expected) in [
        ("max_value_count", "Canonical:ValueLimit"),
        ("max_collection_entries", "Canonical:CollectionLimit"),
        ("max_string_bytes", "Canonical:StringLimit"),
    ] {
        let limits = Limits {
            max_value_count: key.starts_with("max_value_count").then_some(0),
            max_collection_entries: key.starts_with("max_collection_entries").then_some(0),
            max_string_bytes: key.starts_with("max_string_bytes").then_some(0),
            ..Limits::default()
        };
        corpus.push(reject(key, base.clone(), limits, Some(expected)));
    }
    execute(corpus);
}

#[test]
fn oversized_input_clamps_with_huge_caller_caps_on_both_sides() {
    // One byte over the 1 MiB ingress cap, with every caller cap raised to
    // the maximum: both decoders must clamp and reject with the input limit.
    let oversized = vec![0xff_u8; (1 << 20) + 1];
    execute(vec![reject(
        "input-clamped-at-1mib-plus-one",
        oversized,
        limits_all_huge(),
        Some("Canonical:InputLimit"),
    )]);
}

#[test]
fn ingress_rejections_are_python_only() {
    // Broken request JSON rejects at the ingress layer; the Rust harness has
    // no line protocol, so this row asserts the python-only category.
    let mut corpus = vec![Case {
        name: "ingress/broken-request-json".to_owned(),
        kind: Kind::IngressOnly {
            line: "{".to_owned(),
            error: "RequestError",
        },
    }];
    // A logical line over the bounded ingress read cap is drained and
    // rejected, and the following request must still be served normally.
    corpus.push(Case {
        name: "ingress/oversized-line".to_owned(),
        kind: Kind::IngressOnly {
            line: "x".repeat(2_101_250),
            error: "IngressLimit",
        },
    });
    corpus.push(agree(
        "ingress/next-request-after-drain",
        bytes(minimal()),
        Limits::default(),
    ));
    execute(corpus);
}

#[test]
fn missing_witness_interpreter_fails_loudly() {
    // A missing interpreter is a loud failure, never a skipped green. This
    // mirrors the draft witness's Spawn-failure precedent.
    let mut command = Command::new("definitely-nonexistent-wdbx-python");
    command
        .arg(script())
        .arg("decode-json-lines")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let error = command
        .spawn()
        .err()
        .map(|error| error.to_string())
        .unwrap_or_else(|| "unexpectedly spawned".to_owned());
    assert!(
        !error.is_empty(),
        "interpreter spawn failure must be reported"
    );
}
