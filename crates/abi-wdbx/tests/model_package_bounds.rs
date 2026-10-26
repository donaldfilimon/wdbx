//! Hostile I/O, arithmetic, cumulative budgets and hash-valid malformed schemas.
#[path = "support/model_package.rs"]
mod support;
use abi_wdbx::model::{error::ErrorKind as E, limits::*, package::*};
use std::io::{self, Read};
use support::*;
#[test]
fn every_byte_truncation_and_trailing_probe_fail_closed() {
    let raw = fixture("inference");
    for end in 0..raw.len() {
        assert_eq!(
            read_bounded_snapshot(&raw[..end], LoadLimits::default())
                .unwrap_err()
                .kind,
            E::Truncated,
            "{end}"
        );
    }
    let mut extra = raw.clone();
    extra.push(0);
    assert_eq!(
        read_bounded_snapshot(extra.as_slice(), LoadLimits::default())
            .unwrap_err()
            .kind,
        E::TrailingBytes
    );
}
#[test]
fn short_reads_interruptions_and_cancellation_are_not_implicit_retry() {
    struct Short<'a>(&'a [u8]);
    impl Read for Short<'_> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(3).min(self.0.len());
            out[..n].copy_from_slice(&self.0[..n]);
            self.0 = &self.0[n..];
            Ok(n)
        }
    }
    let raw = fixture("inference");
    assert_eq!(
        read_bounded_snapshot(Short(&raw), LoadLimits::default())
            .unwrap()
            .as_bytes(),
        raw
    );
    struct Interrupted;
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::Interrupted.into())
        }
    }
    assert_eq!(
        read_bounded_snapshot(Interrupted, LoadLimits::default())
            .unwrap_err()
            .kind,
        E::Io
    );
    let mut reads = 0;
    assert_eq!(
        read_bounded_snapshot_with_cancel(Short(&raw), LoadLimits::default(), || {
            reads += 1;
            reads == 4
        })
        .unwrap_err()
        .kind,
        E::Cancelled
    );
    assert_eq!(reads, 4);
}
#[test]
fn outer_oversize_refuses_before_reading_the_payload() {
    struct HeaderOnly {
        head: [u8; 96],
        reads: u32,
    }
    impl Read for HeaderOnly {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            assert_eq!(self.reads, 1, "payload read before cap refusal");
            out.copy_from_slice(&self.head);
            Ok(96)
        }
    }
    let raw = fixture("inference");
    for total in [u64::MAX, MAX_ARTIFACT_BYTES + 1] {
        let mut head: [u8; 96] = raw[..96].try_into().unwrap();
        head[16..24].copy_from_slice(&total.to_le_bytes());
        assert_eq!(
            read_bounded_snapshot(HeaderOnly { head, reads: 0 }, LoadLimits::default())
                .unwrap_err()
                .kind,
            E::CapacityExceeded
        );
    }
}
#[test]
fn limits_are_cumulative_and_caller_cannot_expand_profile() {
    let raw = fixture("resumable");
    for l in [
        LoadLimits {
            artifact_bytes: raw.len() as u64 - 1,
            ..LoadLimits::default()
        },
        LoadLimits {
            loading_bytes: LOADING_OVERHEAD_BYTES,
            ..LoadLimits::default()
        },
    ] {
        assert_eq!(
            read_bounded_snapshot(raw.as_slice(), l).unwrap_err().kind,
            E::CapacityExceeded
        );
    }
    for l in [
        LoadLimits {
            metadata_bytes: 50,
            ..LoadLimits::default()
        },
        LoadLimits {
            tokenizer_bytes: 20,
            ..LoadLimits::default()
        },
        LoadLimits {
            loading_bytes: LOADING_OVERHEAD_BYTES + raw.len() as u64 + 100,
            ..LoadLimits::default()
        },
    ] {
        let f = validate_framing(
            read_bounded_snapshot(raw.as_slice(), LoadLimits::default()).unwrap(),
            l,
        )
        .unwrap();
        assert_eq!(validate_model(f, l).unwrap_err().kind, E::CapacityExceeded);
    }
}
#[test]
fn checked_ranges_reject_gaps_overlap_alias_and_u64_overflow() {
    let raw = fixture("inference");
    for (i, offset, length) in [
        (0, 737, 1),
        (1, 736, 1),
        (0, u64::MAX, u64::MAX),
        (0, 736, 0),
        (0, 736, u64::MAX),
    ] {
        let mut b = raw.clone();
        let p = 96 + i * 80;
        b[p + 16..p + 24].copy_from_slice(&offset.to_le_bytes());
        b[p + 24..p + 32].copy_from_slice(&length.to_le_bytes());
        directory_hash(&mut b);
        assert_eq!(load(&b).unwrap_err().kind, E::NonCanonical);
    }
}
#[test]
fn checksums_precede_semantics_and_framing_is_not_execution() {
    let bad = fixture("tensor-nan");
    let framed = validate_framing(
        read_bounded_snapshot(bad.as_slice(), LoadLimits::default()).unwrap(),
        LoadLimits::default(),
    )
    .unwrap();
    assert_eq!(
        validate_model(framed, LoadLimits::default())
            .unwrap_err()
            .kind,
        E::InvalidTensor
    );
    let mut corrupt = bad;
    let n = corrupt.len();
    corrupt[n - 1] ^= 1;
    assert_eq!(load(&corrupt).unwrap_err().kind, E::IntegrityMismatch);
}
#[test]
fn hash_valid_numeric_order_and_resource_corruption_reach_schema_checks() {
    let raw = fixture("resumable");
    let cases = [
        (
            3,
            4 + 2 + 1 + 1 + 1,
            u32::MAX.to_le_bytes().to_vec(),
            E::InvalidTensor,
        ),
        (5, 8, f64::INFINITY.to_le_bytes().to_vec(), E::NonCanonical),
        (6, 13, f64::NAN.to_le_bytes().to_vec(), E::NonCanonical),
        (7, 0, vec![7], E::NonCanonical),
    ];
    for (kind, pos, value, want) in cases {
        let mut b = component(&raw, kind).to_vec();
        b[pos..pos + value.len()].copy_from_slice(&value);
        let b = replace(&raw, kind, &b);
        assert_eq!(load(&b).unwrap_err().kind, want, "kind {kind}");
    }
    let mut oversized = component(&raw, 4).to_vec();
    oversized[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        load(&replace(&raw, 4, &oversized)).unwrap_err().kind,
        E::CapacityExceeded
    );
}
#[test]
fn snapshot_is_independent_of_later_caller_buffer_changes() {
    let mut raw = fixture("inference");
    let snapshot = read_bounded_snapshot(raw.as_slice(), LoadLimits::default()).unwrap();
    raw.fill(0);
    let state = validate_model(
        validate_framing(snapshot, LoadLimits::default()).unwrap(),
        LoadLimits::default(),
    )
    .unwrap();
    assert_eq!(state.components().topology.nodes.len(), 3);
}
#[test]
fn model_validation_applies_its_own_narrowed_artifact_limit() {
    let raw = fixture("inference");
    let frame = validate_framing(
        read_bounded_snapshot(raw.as_slice(), LoadLimits::default()).unwrap(),
        LoadLimits::default(),
    )
    .unwrap();
    let limits = LoadLimits {
        artifact_bytes: 1,
        ..LoadLimits::default()
    };
    assert_eq!(
        validate_model(frame, limits).unwrap_err().kind,
        E::CapacityExceeded
    );
}
#[test]
fn integrity_valid_truncations_exercise_closed_schema_field_boundaries() {
    let raw = fixture("resumable");
    let worksheet: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/model-v1/field-table.json")).unwrap();
    let mut cases = 0;
    for (key, offset) in worksheet["resumable"].as_object().unwrap() {
        let kind = if key.starts_with("architecture.") {
            1
        } else if key.starts_with("topology.") {
            2
        } else if key.starts_with("tensor.") {
            3
        } else if key.starts_with("tokenizer.") {
            4
        } else if key.starts_with("optimizer.") {
            6
        } else if key.starts_with("rng.") {
            7
        } else {
            8
        };
        let payload = component(&raw, kind);
        let offset = usize::try_from(offset.as_u64().unwrap()).unwrap();
        for end in [offset, offset + 1] {
            if end > 0 && end < payload.len() {
                let shorter = replace(&raw, kind, &payload[..end]);
                let framed = validate_framing(
                    read_bounded_snapshot(shorter.as_slice(), LoadLimits::default()).unwrap(),
                    LoadLimits::default(),
                )
                .unwrap();
                assert_eq!(
                    validate_model(framed, LoadLimits::default())
                        .unwrap_err()
                        .kind,
                    E::Truncated,
                    "{key} end {end}"
                );
                cases += 1;
            }
        }
    }
    for kind in 1..=8 {
        let payload = component(&raw, kind);
        let shorter = replace(&raw, kind, &payload[..payload.len() - 1]);
        assert_eq!(
            load(&shorter).unwrap_err().kind,
            E::Truncated,
            "component {kind}"
        );
    }
    assert!(cases > 150);
}
#[test]
fn valid_hashes_cannot_admit_missing_endpoints_duplicate_edges_or_invalid_utf8() {
    let raw = fixture("inference");
    let mut top = component(&raw, 2).to_vec();
    top[80..88].copy_from_slice(&999u64.to_le_bytes());
    assert_eq!(
        load(&replace(&raw, 2, &top)).unwrap_err().kind,
        E::InvalidTopology
    );
    let mut top = component(&raw, 2).to_vec();
    top[108..128].copy_from_slice(&component(&raw, 2)[80..100]);
    assert_eq!(
        load(&replace(&raw, 2, &top)).unwrap_err().kind,
        E::InvalidTopology
    );
    let mut top = component(&raw, 2).to_vec();
    top[30] = 255;
    assert_eq!(
        load(&replace(&raw, 2, &top)).unwrap_err().kind,
        E::NonCanonical
    );
    let mut architecture = component(&raw, 1).to_vec();
    architecture[4] = 255;
    assert_eq!(
        load(&replace(&raw, 1, &architecture)).unwrap_err().kind,
        E::NonCanonical
    );
}
#[cfg(unix)]
#[test]
fn concurrent_in_place_source_rewrite_is_not_certified_by_a_stable_descriptor() {
    struct Rewrite {
        file: std::fs::File,
        path: std::path::PathBuf,
        after: Vec<u8>,
        first: bool,
    }
    impl Read for Rewrite {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = self.file.read(out)?;
            if self.first {
                self.first = false;
                std::fs::write(&self.path, &self.after)?;
            }
            Ok(n)
        }
    }
    let directory = temporary();
    let path = directory.join("inplace.wdbx");
    std::fs::write(&path, fixture("inference")).unwrap();
    let reader = Rewrite {
        file: std::fs::File::open(&path).unwrap(),
        path: path.clone(),
        after: fixture("signed-zero"),
        first: true,
    };
    let snapshot = read_bounded_snapshot(reader, LoadLimits::default()).unwrap();
    assert_eq!(
        validate_framing(snapshot, LoadLimits::default())
            .unwrap_err()
            .kind,
        E::IntegrityMismatch
    );
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn unknown_tensor_membership_matches_independent_semantic_error_family() {
    let raw = fixture("inference");
    let mut tensor = component(&raw, 3).to_vec();
    tensor[4..6].copy_from_slice(&17u16.to_le_bytes());
    assert_eq!(
        load(&replace(&raw, 3, &tensor)).unwrap_err().kind,
        E::InvalidTensor
    );
}
#[test]
fn known_schema_and_cross_link_failures_keep_bounded_component_context() {
    for (name, kind) in [("base-byte-wrong", 4), ("shifted-node-axis", 3)] {
        let raw = fixture(name);
        let error = load(&raw).unwrap_err();
        assert_eq!(error.component, Some(kind), "{name}");
        assert!(
            error
                .offset
                .is_some_and(|offset| offset <= raw.len() as u64)
        );
    }
}
#[test]
fn writer_rejects_retained_capacity_peak_before_output_allocation() {
    use abi_wdbx::model::{contract::tensors::TensorValues, package::build_package};
    let mut c = load(&fixture("inference")).unwrap().components().clone();
    if let TensorValues::F32(values) = &mut c.tensors[0].data {
        values.reserve_exact(1_000_000);
        assert!(values.capacity() >= 1_000_000);
    }
    let limits = LoadLimits {
        loading_bytes: LOADING_OVERHEAD_BYTES + 1024 * 1024,
        ..LoadLimits::default()
    };
    let error =
        build_package(c, &[], limits).expect_err("excessive retained capacity was accepted");
    assert_eq!(error.kind, E::CapacityExceeded);
    assert_eq!(error.field, "writer-peak-budget");
}
#[test]
fn writer_accounts_for_spare_string_and_annotation_capacity() {
    use abi_wdbx::model::{
        contract::Version,
        package::{Annotation, build_package},
    };
    let limits = LoadLimits {
        loading_bytes: LOADING_OVERHEAD_BYTES + 1024 * 1024,
        ..LoadLimits::default()
    };
    let mut c = load(&fixture("inference")).unwrap().components().clone();
    c.provenance.source_cohorts[0]
        .revision
        .reserve_exact(2 * 1024 * 1024);
    let error =
        build_package(c, &[], limits).expect_err("excessive retained capacity was accepted");
    assert_eq!(error.kind, E::CapacityExceeded);
    assert_eq!(error.field, "writer-peak-budget");
    let c = load(&fixture("inference")).unwrap().components().clone();
    let mut payload = Vec::with_capacity(2 * 1024 * 1024);
    payload.push(1);
    let annotation = Annotation {
        kind: 99,
        version: Version { major: 1, minor: 0 },
        bytes: payload,
    };
    let error = build_package(c, &[annotation], limits)
        .expect_err("excessive retained capacity was accepted");
    assert_eq!(error.kind, E::CapacityExceeded);
    assert_eq!(error.field, "writer-peak-budget");
}
