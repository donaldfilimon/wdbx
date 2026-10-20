//! Independent provisional mapping evidence, not admission or a wire schema.
use super::super::CompleteEpisode;
use super::{draft_adapter, fixtures};
use crate::v3::commitment::{CanonicalDecodeLimits, CanonicalValue, EpisodeCommitment};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
#[derive(Debug, PartialEq, Eq)]
pub(super) enum WitnessError {
    Spawn,
    Stdin,
    Wait,
    Write,
    Exit,
    Utf8,
    Json,
    Shape,
    Hex,
    Digest,
    Count,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Response {
    Success { bytes: Vec<u8>, digest: [u8; 32] },
    Invalid,
}
pub(super) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(&mut text, "{byte:02x}").unwrap();
        text
    })
}
fn unhex(s: &str) -> Result<Vec<u8>, WitnessError> {
    if !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(WitnessError::Hex);
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| {
            u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).map_err(|_| WitnessError::Hex)
        })
        .collect()
}
fn parse_output(bytes: &[u8], count: usize) -> Result<Vec<Response>, WitnessError> {
    let text = std::str::from_utf8(bytes).map_err(|_| WitnessError::Utf8)?;
    let responses = text
        .lines()
        .map(|line| {
            let v: Value = super::draft_shape::parse(line).map_err(|_| WitnessError::Json)?;
            let o = v.as_object().ok_or(WitnessError::Shape)?;
            match o.get("ok").and_then(Value::as_bool) {
                Some(true)
                    if o.len() == 3 && o.contains_key("bytes") && o.contains_key("digest") =>
                {
                    let bytes = unhex(o["bytes"].as_str().ok_or(WitnessError::Shape)?)?;
                    let digest: [u8; 32] = unhex(o["digest"].as_str().ok_or(WitnessError::Shape)?)?
                        .try_into()
                        .map_err(|_| WitnessError::Digest)?;
                    if digest != <[u8; 32]>::from(Sha256::digest(&bytes)) {
                        return Err(WitnessError::Digest);
                    }
                    Ok(Response::Success { bytes, digest })
                }
                Some(false) if o.len() == 2 && o.get("error") == Some(&json!("FixtureError")) => {
                    Ok(Response::Invalid)
                }
                _ => Err(WitnessError::Shape),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if responses.len() != count {
        return Err(WitnessError::Count);
    }
    Ok(responses)
}
pub(super) fn run_command(
    mut command: Command,
    lines: &[String],
) -> Result<Vec<Response>, WitnessError> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| WitnessError::Spawn)?;
    let mut stdin = child.stdin.take().ok_or(WitnessError::Stdin)?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for line in lines {
                writeln!(stdin, "{line}").map_err(|_| WitnessError::Write)?;
            }
            Ok::<_, WitnessError>(())
        });
        let output = child.wait_with_output().map_err(|_| WitnessError::Wait);
        writer.join().map_err(|_| WitnessError::Write)??;
        output
    })?;
    if !output.status.success() {
        return Err(WitnessError::Exit);
    }
    parse_output(&output.stdout, lines.len())
}
fn run(executable: &str, lines: &[String]) -> Result<Vec<Response>, WitnessError> {
    let mut command = Command::new(executable);
    command
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/wdbx_complete_episode_draft.py"),
        )
        .arg("differential");
    run_command(command, lines)
}
pub(super) fn witness(values: &[Value]) -> Vec<Response> {
    let lines: Vec<_> = values.iter().map(Value::to_string).collect();
    run(
        &std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".into()),
        &lines,
    )
    .expect("draft witness must run; set WDBX_PYTHON")
}
pub(super) fn fixture(populated: bool) -> Value {
    super::draft_shape::parse(if populated {
        include_str!("provisional-populated.json")
    } else {
        include_str!("provisional-minimal.json")
    })
    .unwrap()
}
pub(super) fn compare(values: &[Value]) {
    let responses = witness(values);
    for (v, response) in values.iter().zip(responses) {
        let episode = CompleteEpisode::try_new(
            draft_adapter::input(v).expect("synthetic fixture construction"),
        )
        .expect("valid synthetic draft");
        let bytes = episode.canonical_bytes().unwrap();
        let digest = episode.digest().unwrap();
        assert_eq!(
            response,
            Response::Success {
                bytes: bytes.clone(),
                digest
            }
        );
        let typed = super::DecodedCompleteEpisode::from_canonical_bytes(
            &bytes,
            CanonicalDecodeLimits::default(),
        )
        .expect("private bounded typed inverse");
        assert_eq!(
            typed.input(),
            &super::decode_normalize::normalize(
                draft_adapter::input(v).expect("synthetic fixture construction"),
            )
        );
        assert_eq!(typed.episode().canonical_bytes().unwrap(), bytes);
        assert_eq!(typed.episode().digest().unwrap(), digest);
        let limits = CanonicalDecodeLimits::default();
        let decoded = CanonicalValue::from_canonical_bytes(&bytes, limits)
            .expect("bounded generic syntax only");
        assert_eq!(decoded.canonical_bytes().unwrap(), bytes);
        assert!(EpisodeCommitment::from_canonical_bytes(&bytes, limits).is_err());
    }
}
#[test]
fn pinned_fixtures_and_duplex_batch() {
    for (populated, typed, pin, digest) in [
        (
            false,
            fixtures::minimal(),
            include_str!("provisional-minimal.hex"),
            "9d3f1c50a6740f327f461744419faade90948a4084c1f690ef4f968a9ab62a2a",
        ),
        (
            true,
            fixtures::populated(),
            include_str!("provisional-populated.hex"),
            "468026897ab6faf66ca198ad688e23174d599d9604717863b072826950a07a14",
        ),
    ] {
        let v = fixture(populated);
        assert_eq!(draft_adapter::input(&v).unwrap(), typed);
        let episode = CompleteEpisode::try_new(typed).unwrap();
        assert_eq!(hex(&episode.canonical_bytes().unwrap()), pin.trim());
        assert_eq!(hex(&episode.digest().unwrap()), digest);
        compare(&[v]);
    }
    let values = vec![fixture(true); 64];
    assert!(values.iter().map(|v| v.to_string().len()).sum::<usize>() > 65_536);
    compare(&values);
}
#[test]
fn strict_requests_and_protocol_failures() {
    let mut bad = Vec::new();
    for text in [
        "",
        "{",
        "null",
        "[]",
        "{}",
        r#"{"identity":{},"identity":{}}"#,
    ] {
        bad.push(text.to_owned());
    }
    for (path, replacement) in [
        ("/time/received_at", json!(true)),
        ("/time/received_at", json!(1.0)),
        ("/time/received_at", Value::Null),
        ("/identity/episode_id", json!("GG")),
        ("/identity/contract/digest", json!("00")),
        ("/decision/route", json!("UnknownVariant")),
        ("/context/habituation", json!({"unknown":false})),
        ("/context/habituation", json!({"known":0,"extra":0})),
    ] {
        let mut v = fixture(false);
        *v.pointer_mut(path).unwrap() = replacement;
        bad.push(v.to_string());
    }
    let mut extra = fixture(false);
    extra["time"]["extra"] = json!(0);
    bad.push(extra.to_string());
    let duplicate = fixture(false).to_string().replace(
        "\"received_at\":99",
        "\"received_at\":99,\"received_at\":99",
    );
    bad.push(duplicate);
    bad.push(
        fixture(false)
            .to_string()
            .replace("\"received_at\":99", "\"received_at\":NaN"),
    );
    for line in &bad {
        assert!(
            super::draft_shape::parse(line)
                .and_then(|v| draft_adapter::input(&v).map(|_| ()))
                .is_err()
        );
    }
    let responses = run(
        &std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".into()),
        &bad,
    )
    .unwrap();
    assert!(responses.iter().all(|r| *r == Response::Invalid));
    assert_eq!(
        run("/definitely-nonexistent-wdbx-draft-python", &[]),
        Err(WitnessError::Spawn)
    );
    for bytes in [
        b"".as_slice(),
        b"{}\n",
        b"null\n",
        b"bad\n",
        b"\xff\n",
        b"{\"ok\":true,\"bytes\":\"00\",\"digest\":\"00\"}\n",
        b"{\"ok\":false,\"error\":\"Secret\"}\n",
        b"{\"ok\":false,\"error\":\"FixtureError\",\"extra\":0}\n",
        b"{\"ok\":false}\n",
        b"{\"ok\":1,\"error\":\"FixtureError\"}\n",
    ] {
        assert!(parse_output(bytes, 1).is_err());
    }
    assert!(parse_output(b"{\"ok\":false,\"error\":\"FixtureError\"}\n", 0).is_err());
    for response in [
        json!({"ok":true,"bytes":"GG","digest":"00".repeat(32)}),
        json!({"ok":true,"bytes":"00","digest":"00".repeat(32)}),
        json!({"ok":true,"bytes":"00"}),
        json!({"ok":true,"digest":"00".repeat(32)}),
        json!({"ok":true,"bytes":"00","digest":"00".repeat(32),"extra":0}),
        json!({"ok":false,"bytes":"00","digest":"00".repeat(32)}),
    ] {
        assert!(parse_output(format!("{response}\n").as_bytes(), 1).is_err());
    }
    assert!(
        parse_output(
            b"{\"ok\":false,\"ok\":false,\"error\":\"FixtureError\"}\n",
            1
        )
        .is_err()
    );
    assert!(unhex("GG").is_err());
    assert!(unhex("0").is_err());
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 7"]);
    assert_eq!(run_command(command, &[]), Err(WitnessError::Exit));
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exec 0<&-; sleep 0.1; exit 0"]);
    assert_eq!(
        run_command(command, &["x".repeat(1_048_576)]),
        Err(WitnessError::Write)
    );
}
