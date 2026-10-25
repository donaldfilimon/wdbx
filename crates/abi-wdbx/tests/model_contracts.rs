//! N01 catalog/golden witnesses, not a production model parser or runtime test.
use std::{fs, path::PathBuf, process::Command};

use abi_wdbx::model::contract::tensors::{AxisKind, Dtype, TENSOR_CATALOG, TensorId};
use abi_wdbx::model::{contract::*, error::ErrorKind, limits::*};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model-v1")
}
fn decode_hex(text: &str) -> Vec<u8> {
    let text = text.trim();
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}
fn sha(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}
fn number(bytes: &[u8], offset: usize, size: usize) -> u64 {
    let mut word = [0; 8];
    word[..size].copy_from_slice(&bytes[offset..offset + size]);
    u64::from_le_bytes(word)
}

#[test]
fn catalogs_are_exact_and_closed() {
    assert_eq!(MAGIC, *b"WDBXMOD1");
    assert_eq!((HEADER_BYTES, DIRECTORY_ENTRY_BYTES), (96, 80));
    assert_eq!((V1.major, V1.minor), (1, 0));
    assert_eq!(COMPONENT_CATALOG.map(|c| c.kind), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(COMPONENT_CATALOG.iter().all(|c| c.version == V1));
    assert_eq!(
        TENSOR_CATALOG.map(|t| t.id as u16),
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
    assert_eq!(
        TENSOR_CATALOG
            .iter()
            .filter(|t| t.adamw_decay)
            .map(|t| t.id as u16)
            .collect::<Vec<_>>(),
        [4, 5, 6, 7, 10, 12, 13, 14]
    );
    assert_eq!(TENSOR_CATALOG[1].dtype, Dtype::F64);
    assert!(
        TENSOR_CATALOG
            .iter()
            .enumerate()
            .all(|(i, t)| i == 1 || t.dtype == Dtype::F32)
    );
    assert_eq!(
        TENSOR_CATALOG[3].axes,
        [AxisKind::HiddenOut, AxisKind::HiddenIn]
    );
    assert_eq!(TENSOR_CATALOG[8].axes, [AxisKind::Edge]);
    assert_eq!(
        TENSOR_CATALOG[9].axes,
        [AxisKind::Token, AxisKind::HiddenIn]
    );
    assert_eq!(
        TensorId::try_from(17).unwrap_err().kind,
        ErrorKind::NonCanonical
    );
    assert!(AxisKind::try_from(0).is_err());
    assert!(Dtype::try_from(0).is_err());
    assert!(architecture::Purpose::try_from(2).is_err());
    assert!(architecture::OperatorSet::try_from(2).is_err());
    assert!(topology::NodeRole::try_from(4).is_err());
    assert!(training::StreamId::try_from(6).is_err());
    assert!(provenance::DataRootKind::try_from(5).is_err());
}

#[test]
fn policy_limits_only_narrow_profile() {
    let limits = LoadLimits {
        artifact_bytes: u64::MAX,
        metadata_bytes: 1,
        tokenizer_bytes: 0,
        loading_bytes: u64::MAX,
    }
    .effective();
    assert_eq!(limits.artifact_bytes, 64 * 1024 * 1024);
    assert_eq!(limits.metadata_bytes, 1);
    assert_eq!(limits.tokenizer_bytes, 0);
    assert_eq!(limits.loading_bytes, 256 * 1024 * 1024);
    assert_eq!(
        (MAX_NODES, MAX_EDGES, MAX_HIDDEN_WIDTH, MAX_RELATIONS),
        (128, 1024, 64, 8)
    );
    assert_eq!(
        (MAX_TRAINABLE_SCALARS, MAX_TRAINING_SCALAR_OPS),
        (1_000_000, 500_000_000)
    );
    assert_eq!(
        (MAX_TRAINER_VISITS, MAX_ENCODER_VISITS),
        (100_000_000, 8_000_000)
    );
}

#[test]
fn frozen_bytes_and_independent_identity_formulas() {
    let manifest: Value =
        serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    for row in manifest["fixtures"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let bytes =
            decode_hex(&fs::read_to_string(fixtures().join(format!("{name}.hex"))).unwrap());
        assert_eq!(hex(&sha(&bytes)), row["artifact_sha256"], "{name}");
        assert_eq!(
            fs::read_to_string(fixtures().join(format!("{name}.sha256")))
                .unwrap()
                .trim(),
            row["artifact_sha256"].as_str().unwrap()
        );
        if row["expected"] != "ValidatedModelState" {
            continue;
        }
        assert_eq!(&bytes[..8], MAGIC);
        assert_eq!(number(&bytes, 16, 8), u64::try_from(bytes.len()).unwrap());
        let count = usize::try_from(number(&bytes, 24, 4)).unwrap();
        let directory_end = 96 + count * 80;
        assert_eq!(sha(&bytes[96..directory_end]), bytes[32..64]);
        let mut package = PACKAGE_DOMAIN.to_vec();
        package.extend_from_slice(&bytes);
        assert_eq!(hex(&sha(&package)), row["package_digest"]);
        let mut state = STATE_DOMAIN.to_vec();
        let mut provenance_start = 0;
        let mut provenance_digest = Vec::new();
        let mut next = directory_end;
        for index in 0..count {
            let entry = &bytes[96 + index * 80..96 + (index + 1) * 80];
            let kind = number(entry, 0, 4);
            let offset = usize::try_from(number(entry, 16, 8)).unwrap();
            let length = usize::try_from(number(entry, 24, 8)).unwrap();
            assert_eq!(offset, next);
            next += length;
            assert_eq!(sha(&bytes[offset..offset + length]), entry[32..64]);
            if kind <= 7 {
                state.extend_from_slice(&entry[..8]);
                state.extend_from_slice(&entry[32..64]);
            }
            if kind == 8 {
                provenance_start = offset;
                provenance_digest = entry[32..64].to_vec();
            }
        }
        assert_eq!(next, bytes.len());
        let root = sha(&state);
        assert_eq!(hex(&root), row["state_root"]);
        let mut generation = GENERATION_DOMAIN.to_vec();
        generation.extend_from_slice(&bytes[provenance_start..provenance_start + 48]);
        // All frozen positive diagnostics have absent parent generation.
        assert_eq!(bytes[provenance_start + 49], 0);
        generation.push(0);
        generation.extend_from_slice(&root);
        generation.extend(provenance_digest);
        assert_eq!(hex(&sha(&generation)), row["generation_id"]);
    }
}

#[test]
fn independent_python_full_schema_witness_is_required() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let python = std::env::var_os("WDBX_PYTHON").unwrap_or_else(|| "python3".into());
    let result = Command::new(python)
        .current_dir(root)
        .args([
            "-m",
            "unittest",
            "discover",
            "-s",
            "tools/tests",
            "-p",
            "test_wdbx_model_reference.py",
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Python is required for the independent native-model contract witness; never skip");
    assert!(
        result.status.success(),
        "independent witness failed:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn frozen_tensor_descriptors_match_every_catalog_axis() {
    let bytes = decode_hex(&fs::read_to_string(fixtures().join("inference.hex")).unwrap());
    let entry = &bytes[96 + 2 * 80..96 + 3 * 80];
    let start = usize::try_from(number(entry, 16, 8)).unwrap();
    let length = usize::try_from(number(entry, 24, 8)).unwrap();
    let data = &bytes[start..start + length];
    assert_eq!(number(data, 0, 4), 16);
    let mut pos = 4;
    for spec in TENSOR_CATALOG {
        assert_eq!(number(data, pos, 2), spec.id as u64);
        assert_eq!(data[pos + 2], spec.dtype as u8);
        assert_eq!(usize::from(data[pos + 3]), spec.axes.len());
        pos += 4;
        let mut scalars = 1;
        for axis in spec.axes {
            assert_eq!(data[pos], *axis as u8);
            let dimension = number(data, pos + 1, 4);
            let expected = match axis {
                AxisKind::Node | AxisKind::Xyz => 3,
                AxisKind::Edge | AxisKind::HiddenIn | AxisKind::HiddenOut => 2,
                AxisKind::Token => 260,
                AxisKind::Relation => 1,
            };
            assert_eq!(dimension, expected);
            scalars *= dimension;
            pos += 5;
            let ids: &[u64] = match axis {
                AxisKind::Node => &[10, 30, 90],
                AxisKind::Edge => &[7, 42],
                _ => &[],
            };
            for id in ids {
                assert_eq!(number(data, pos, 8), *id);
                pos += 8;
            }
        }
        let width = if spec.dtype == Dtype::F32 { 4 } else { 8 };
        assert_eq!(number(data, pos, 8), scalars * width);
        pos += 8 + usize::try_from(scalars * width).unwrap();
    }
    assert_eq!(pos, data.len());
}
