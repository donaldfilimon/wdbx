//! Local syntax interoperability evidence, not signing or admission evidence.
use abi_wdbx::v3::commitment::{CanonicalDecodeLimits, CanonicalValue, EpisodeCommitment};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/abbey_cbor_episode_v1.py")
}
fn run(executable: &str, lines: &[String]) -> Result<Vec<Value>, String> {
    let mut command = Command::new(executable);
    command.arg(script()).arg("decode-corpus");
    run_command(command, lines)
}
fn run_command(mut command: Command, lines: &[String]) -> Result<Vec<Value>, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|_| "cannot launch Python; set WDBX_PYTHON".to_owned())?;
    let mut stdin = child.stdin.take().ok_or("missing stdin")?;
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || -> std::io::Result<()> {
            for line in lines {
                writeln!(stdin, "{line}")?;
            }
            Ok(())
        });
        let output = child
            .wait_with_output()
            .map_err(|_| "wait failed".to_owned());
        writer
            .join()
            .map_err(|_| "writer panicked")?
            .map_err(|_| "write failed")?;
        output
    })?;
    if !output.status.success() {
        return Err("witness exited unsuccessfully".into());
    }
    parse_output(&output.stdout, lines.len())
}
fn parse_output(bytes: &[u8], count: usize) -> Result<Vec<Value>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "non UTF-8 witness")?;
    let values: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).map_err(|_| "invalid JSON witness".to_owned()))
        .collect::<Result<_, _>>()?;
    if values.len() != count {
        return Err("wrong witness response count".into());
    }
    for value in &values {
        let object = value.as_object().ok_or("response not object")?;
        if object.len() != 2 {
            return Err("wrong response shape".into());
        }
        match value["ok"].as_bool() {
            Some(true) => {
                let hex = value["cbor_hex"].as_str().ok_or("missing hex")?;
                if hex.len() % 2 != 0
                    || !hex
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    return Err("invalid hex".into());
                }
            }
            Some(false)
                if value["error"]
                    .as_str()
                    .is_some_and(|error| CATEGORIES.contains(&error)) => {}
            _ => return Err("invalid response".into()),
        }
    }
    Ok(values)
}
const CATEGORIES: &[&str] = &[
    "RequestError",
    "InputLimit",
    "ValueLimit",
    "CollectionLimit",
    "StringLimit",
    "ParentLimit",
    "UnexpectedEnd",
    "TrailingBytes",
    "InvalidAdditionalInformation",
    "NonMinimalArgument",
    "UnsupportedType",
    "InvalidUtf8",
    "IntegerOutOfRange",
    "NestingLimit",
    "LengthOutOfRange",
    "AllocationFailure",
    "DuplicateMapKey",
    "MapKeyOrder",
    "InvalidEnvelope",
    "ProfileMismatch",
    "ZeroSchemaVersion",
    "HeaderMustBeMap",
    "PayloadMustBeMap",
    "InvalidParentDigest",
    "ParentOrder",
];
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").unwrap();
        output
    })
}
fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn python() -> String {
    std::env::var("WDBX_PYTHON").unwrap_or_else(|_| "python3".into())
}
fn compare(cases: &[(Vec<u8>, bool, CanonicalDecodeLimits)]) {
    let lines: Vec<_> = cases.iter().map(|(bytes, episode, limits)| json!({"cbor_hex":hex(bytes),"kind":if *episode {"episode"} else {"value"}, "limits": {"max_input_bytes":limits.max_input_bytes,"max_value_count":limits.max_value_count,"max_collection_entries":limits.max_collection_entries,"max_string_bytes":limits.max_string_bytes,"max_parent_count":limits.max_parent_count}}).to_string()).collect();
    let responses = run(&python(), &lines).expect("Python witness must run; set WDBX_PYTHON");
    for ((bytes, episode, limits), response) in cases.iter().zip(responses) {
        let rust = if *episode {
            EpisodeCommitment::from_canonical_bytes(bytes, *limits).map(|value| {
                let encoded = value.canonical_bytes().unwrap();
                assert_eq!(value.digest().unwrap(), sha256(&encoded));
                encoded
            })
        } else {
            CanonicalValue::from_canonical_bytes(bytes, *limits)
                .map(|v| v.canonical_bytes().unwrap())
        };
        let expected = match rust {
            Ok(encoded) => {
                assert_eq!(&encoded, bytes);
                json!({"ok":true,"cbor_hex":hex(bytes)})
            }
            Err(error) => json!({"ok":false,"error":format!("{error:?}")}),
        };
        assert_eq!(response, expected, "decode disagreement for synthetic case");
    }
}
fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}
#[test]
#[allow(clippy::too_many_lines)] // Explicit malformed byte corpus and boundary matrix.
fn canonical_malformed_bounds_and_duplex_corpus() {
    let defaults = CanonicalDecodeLimits::default();
    let mut cases = Vec::new();
    for value in [
        "00",
        "17",
        "1818",
        "18ff",
        "190100",
        "19ffff",
        "1a00010000",
        "1affffffff",
        "1b0000000100000000",
        "3817",
        "6365cc81",
        "a100f6",
        "a10060",
        "a10040",
        "5f",
        "7f",
        "1bffffffffffffffff",
        "20",
        "3818",
        "3b7fffffffffffffff",
        "40",
        "60",
        "62c3a9",
        "63ed9580",
        "80",
        "a0",
        "f4",
        "f5",
        "f6",
        "a220f61818f6",
        "a180f6",
        "a1a0f6",
        "",
        "1817",
        "1900ff",
        "5800",
        "7800",
        "9800",
        "b800",
        "61ff",
        "c000",
        "f7",
        "f0",
        "f800",
        "f90000",
        "fa00000000",
        "fb0000000000000000",
        "9f",
        "bf",
        "ff",
        "1c",
        "1d",
        "1e",
        "3b8000000000000000",
        "a21818f620f6",
        "a200f600f6",
        "a280f680f6",
        "0000",
        "5bffffffffffffffff",
        "7bffffffffffffffff",
        "9bffffffffffffffff",
        "bbffffffffffffffff",
    ] {
        cases.push((unhex(value), false, defaults));
    }
    for depth in [32, 33] {
        let mut bytes = vec![0x81; depth];
        bytes.push(0xf6);
        cases.push((bytes, false, defaults));
    }
    for name in ["empty-parents", "two-parents"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/golden/abbey-cbor-episode-v1/{name}.hex"));
        {
            let bytes = unhex(std::fs::read_to_string(path).unwrap().trim());
            for end in 0..bytes.len() {
                cases.push((bytes[..end].to_vec(), true, defaults));
            }
            for index in 0..bytes.len() {
                let mut mutated = bytes.clone();
                mutated[index] ^= 0x80;
                cases.push((mutated, true, defaults));
            }
            let mut trailing = bytes.clone();
            trailing.push(0);
            cases.push((trailing, true, defaults));
            cases.push((bytes, true, defaults));
        }
    }
    let base = unhex("a5007561626265792d63626f722d657069736f64652d7631010102a003a00480");
    cases.push((base.clone(), true, defaults));
    for index in 0..base.len() {
        let mut bytes = base.clone();
        bytes[index] ^= 0xff;
        cases.push((bytes, true, defaults));
    }
    for field in 0..5 {
        for limit in [0, 1, 2, 3, 5, 10, 11, 20, 31, 32, 33] {
            let mut limits = defaults;
            match field {
                0 => limits.max_input_bytes = limit,
                1 => limits.max_value_count = limit,
                2 => limits.max_collection_entries = limit,
                3 => limits.max_string_bytes = limit,
                _ => limits.max_parent_count = limit,
            }
            for bytes in [
                unhex("00"),
                unhex("820000"),
                unhex("a10000"),
                unhex("62c3a9"),
                unhex("40"),
                unhex("80"),
                base.clone(),
            ] {
                cases.push((bytes, false, limits));
            }
            cases.push((base.clone(), true, limits));
        }
    }
    for _ in 0..300 {
        let mut bytes = vec![0x79, 4, 0];
        bytes.extend(vec![b'x'; 1024]);
        cases.push((bytes, false, defaults));
    }
    compare(&cases);
}
#[test]
fn strict_requests_and_protocol_failures() {
    let lines: Vec<String> = [
        "",
        "null",
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":NaN}}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":9223372036854775808}}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":1,"max_input_bytes":1}}"#,
        "{}",
        "[]",
        "{",
        r#"{"cbor_hex":"00","kind":"value","extra":0}"#,
        r#"{"cbor_hex":"00","kind":"value","kind":"value"}"#,
        r#"{"cbor_hex":"0","kind":"value"}"#,
        r#"{"cbor_hex":" 00","kind":"value"}"#,
        r#"{"cbor_hex":"gg","kind":"value"}"#,
        r#"{"cbor_hex":"00","kind":"other"}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":null}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":true}}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":-1}}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"max_input_bytes":1.0}}"#,
        r#"{"cbor_hex":"00","kind":"value","limits":{"unknown":1}}"#,
    ]
    .iter()
    .map(|s| (*s).into())
    .collect();
    assert!(
        run(&python(), &lines)
            .unwrap()
            .iter()
            .all(|v| *v == json!({"ok":false,"error":"RequestError"}))
    );
    assert!(
        run("/nonexistent/wdbx-python-witness", &[])
            .unwrap_err()
            .contains("WDBX_PYTHON")
    );
    for bytes in [
        b"".as_slice(),
        b"{}\n",
        b"null\n",
        b"not-json\n",
        b"\xff\n",
        b"{\"ok\":true,\"cbor_hex\":\"GG\"}\n",
        b"{\"ok\":false,\"error\":\"Secret\"}\n",
    ] {
        assert!(parse_output(bytes, 1).is_err());
    }
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "exit 7"]);
    assert!(run_command(command, &[]).is_err());
}

#[test]
fn episode_shapes_parents_and_compound_depth() {
    let d = CanonicalDecodeLimits::default();
    let mut cases = Vec::new();
    let prefix = "a5007561626265792d63626f722d657069736f64652d7631";
    for suffix in [
        "010002a003a00480",
        "01f602a003a00480",
        "0101020003a00480",
        "010102a003000480",
        "010102a003a00400",
        "010102a003a0048100",
        "010102a003a00481581f",
        "010102a003a004815821",
    ] {
        cases.push((unhex(&format!("{prefix}{suffix}")), true, d));
    }
    for parents in [
        vec![],
        vec![[0; 32]],
        vec![[0; 32], [0; 32]],
        vec![[0; 32], [1; 32]],
    ] {
        let episode = EpisodeCommitment::new(
            u64::MAX,
            CanonicalValue::Map(vec![]),
            CanonicalValue::Map(vec![]),
            parents,
        );
        let bytes = episode.canonical_bytes().unwrap();
        cases.push((bytes.clone(), true, d));
        for cap in [0, 1, 2] {
            let limits = CanonicalDecodeLimits {
                max_parent_count: cap,
                ..d
            };
            cases.push((bytes.clone(), true, limits));
            cases.push((bytes.clone(), false, limits));
        }
    }
    let mut descending = unhex(&format!(
        "{prefix}010102a003a004825820{}5820{}",
        "01".repeat(32),
        "00".repeat(32)
    ));
    cases.push((descending.clone(), true, d));
    descending.truncate(descending.len() - 32);
    cases.push((descending, true, d));
    cases.push((
        unhex(&format!("{prefix}010102a003a00482")),
        true,
        CanonicalDecodeLimits {
            max_parent_count: 1,
            ..d
        },
    ));
    for depth in [31, 32] {
        let mut nested = CanonicalValue::Null;
        for _ in 0..depth {
            nested = CanonicalValue::Array(vec![nested]);
        }
        let bytes = CanonicalValue::Map(vec![(nested, CanonicalValue::Null)]).canonical_bytes();
        if let Ok(bytes) = bytes {
            cases.push((bytes, false, d));
        }
        let mut bytes = unhex("a181");
        bytes.extend(vec![0x81; depth]);
        bytes.extend([0xf6, 0xf6]);
        cases.push((bytes, false, d));
    }
    compare(&cases);
}
#[test]
fn literal_contract_and_controlled_bad_children() {
    let cases = [
        ("00", true, ""),
        ("a220f61818f6", true, ""),
        ("1817", false, "NonMinimalArgument"),
        ("61ff", false, "InvalidUtf8"),
        ("a200f600f6", false, "DuplicateMapKey"),
        ("a21818f620f6", false, "MapKeyOrder"),
    ];
    let lines: Vec<_> = cases
        .iter()
        .map(|(hex, _, _)| json!({"cbor_hex":hex,"kind":"value"}).to_string())
        .collect();
    let values = run(&python(), &lines).unwrap();
    for ((hex, ok, error), value) in cases.iter().zip(values) {
        assert_eq!(
            value,
            if *ok {
                json!({"ok":true,"cbor_hex":hex})
            } else {
                json!({"ok":false,"error":error})
            }
        );
    }
    for output in [
        "",
        "{}",
        "not-json",
        "{\"ok\":true,\"cbor_hex\":\"00\",\"extra\":1}",
        "{\"ok\":1,\"error\":\"InputLimit\"}",
    ] {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "printf '%s\\n' \"$1\"",
            "wdbx-witness-fixture",
            output,
        ]);
        assert!(run_command(command, &["synthetic".into()]).is_err());
    }
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut command = Command::new("python3");
    command.env("PATH", &directory);
    assert!(
        run_command(command, &[])
            .unwrap_err()
            .contains("WDBX_PYTHON")
    );
}
