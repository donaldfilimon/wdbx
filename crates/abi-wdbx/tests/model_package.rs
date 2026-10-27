//! Production N02 parser against the independent, frozen N01 corpus.
use abi_wdbx::model::{
    limits::LoadLimits,
    package::{export_exact, read_bounded_snapshot, validate_framing, validate_model},
};
use std::{fs, path::PathBuf};
fn fixture(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/model-v1")
        .join(format!("{name}.hex"));
    let s = fs::read_to_string(p).unwrap();
    s.trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn frozen_independent_corpus_has_exact_validation_results_and_identities() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model-v1/manifest.json");
    let m: serde_json::Value = serde_json::from_slice(&fs::read(p).unwrap()).unwrap();
    for row in m["fixtures"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let raw = fixture(name);
        let result = read_bounded_snapshot(raw.as_slice(), LoadLimits::default())
            .and_then(|b| validate_framing(b, LoadLimits::default()))
            .and_then(|f| validate_model(f, LoadLimits::default()));
        if row["expected"] == "ValidatedModelState" {
            let state = result.unwrap_or_else(|e| panic!("{name}: {e}"));
            let h = |b: &[u8]| support::hex(b);
            assert_eq!(h(&state.state_root().0), row["state_root"], "{name}");
            assert_eq!(h(&state.generation_id().0), row["generation_id"], "{name}");
            assert_eq!(
                h(&state.framed().package_digest().0),
                row["package_digest"],
                "{name}"
            );
            let mut out = Vec::new();
            export_exact(&state, &mut out, || false).unwrap();
            assert_eq!(out, raw);
        } else {
            let error = result.unwrap_err();
            assert_eq!(
                format!("{:?}", error.kind),
                row["expected"].as_str().unwrap(),
                "{name}"
            );
        }
    }
}

#[test]
fn tokenizer_profile_bytes_have_tokenizer_family_like_the_witness() {
    use sha2::{Digest as _, Sha256};
    let bytes = include_bytes!("fixtures/model-v1/tokenizer-profile-negatives-v1.json");
    assert_eq!(
        support::hex(&Sha256::digest(bytes)),
        "d6feb7ebeb2a8446008f1bcf7dbbc8ed65345dec21acd614d886c580049e8a4a"
    );
    let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let rows = doc["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let text = row["hex"].as_str().unwrap();
        let raw: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(
            support::hex(&Sha256::digest(&raw)),
            row["artifact_sha256"],
            "{name}"
        );
        let framed = read_bounded_snapshot(raw.as_slice(), LoadLimits::default())
            .and_then(|b| validate_framing(b, LoadLimits::default()))
            .unwrap_or_else(|e| panic!("{name} must frame: {e}"));
        let error = validate_model(framed, LoadLimits::default()).unwrap_err();
        assert_eq!(
            format!("{:?}", error.kind),
            row["expected"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn n02_matrix_negatives_have_the_same_family_as_the_witness() {
    use sha2::{Digest as _, Sha256};
    let bytes = include_bytes!("fixtures/model-v1/n02-matrix-negatives-v1.json");
    assert_eq!(
        support::hex(&Sha256::digest(bytes)),
        "1a724aa256436a1c8c2f4b95edc25513ac4b7bcff19de338ac401e238f38c966"
    );
    let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let rows = doc["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let text = row["hex"].as_str().unwrap();
        let raw: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(
            support::hex(&Sha256::digest(&raw)),
            row["artifact_sha256"],
            "{name}"
        );
        let framed = read_bounded_snapshot(raw.as_slice(), LoadLimits::default())
            .and_then(|b| validate_framing(b, LoadLimits::default()))
            .unwrap_or_else(|e| panic!("{name} must frame: {e}"));
        let error = validate_model(framed, LoadLimits::default()).unwrap_err();
        assert_eq!(
            format!("{:?}", error.kind),
            row["expected"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[path = "support/model_package.rs"]
mod support;
#[test]
fn real_writer_matches_independent_bytes_including_signed_zero_and_unknown_annotations() {
    use abi_wdbx::model::package::{Annotation, build_package};
    for name in ["inference", "resumable", "signed-zero", "annotation", "bpe"] {
        let raw = fixture(name);
        let state = support::load(&raw).unwrap();
        let annotations = state
            .framed()
            .directory()
            .iter()
            .filter(|e| e.kind > 8)
            .map(|e| Annotation {
                kind: e.kind,
                version: e.version,
                bytes: raw[usize::try_from(e.offset).unwrap()
                    ..usize::try_from(e.offset + e.length).unwrap()]
                    .to_vec(),
            })
            .collect::<Vec<_>>();
        let out = build_package(
            state.components().clone(),
            &annotations,
            LoadLimits::default(),
        )
        .unwrap();
        assert_eq!(out.framed().snapshot().as_bytes(), raw, "{name}");
    }
}
#[test]
fn resume_requires_exact_external_context_and_never_upgrades_inference() {
    use abi_wdbx::model::{
        LocalPolicyContext, ResumeContext, error::ErrorKind, ids::Digest, package::check_resume,
    };
    let raw = fixture("resumable");
    let state = support::load(&raw).unwrap();
    let c = state.components();
    let context = ResumeContext {
        dataset_manifest_digest: c.objective.dataset_manifest_digest,
        split_manifest_digest: c.objective.split_manifest_digest,
        preparation_digest: c.objective.preparation_digest,
        objective_digest: state.framed().directory()[4].component_sha256,
        local_policy: LocalPolicyContext {
            audience_digest: c.provenance.audience_digest,
            policy_digest: c.provenance.policy_digest,
            policy_epoch: c.provenance.policy_epoch,
        },
    };
    assert!(check_resume(state, &context).is_ok());
    let mut wrong = context.clone();
    wrong.local_policy.audience_digest = Digest([4; 32]);
    assert_eq!(
        check_resume(support::load(&raw).unwrap(), &wrong)
            .unwrap_err()
            .kind,
        ErrorKind::ResumeMismatch
    );
    assert_eq!(
        check_resume(support::load(&fixture("inference")).unwrap(), &context)
            .unwrap_err()
            .kind,
        ErrorKind::NotResumable
    );
}
#[cfg(unix)]
#[test]
fn stage_uses_captured_bytes_and_refuses_corruption_without_overwrite() {
    use abi_wdbx::model::package::import_staged;
    let directory = support::temporary();
    let input = directory.join("mutable-input.wdbx");
    let raw = fixture("inference");
    fs::write(&input, &raw).unwrap();
    let snapshot =
        read_bounded_snapshot(fs::File::open(&input).unwrap(), LoadLimits::default()).unwrap();
    fs::write(&input, b"changed-after-read").unwrap();
    let model = validate_model(
        validate_framing(snapshot, LoadLimits::default()).unwrap(),
        LoadLimits::default(),
    )
    .unwrap();
    let result = import_staged(&model, &directory, || {
        fs::write(&input, b"changed-during-import").unwrap();
        false
    })
    .unwrap();
    assert!(result.created);
    assert_eq!(fs::read(&result.path).unwrap(), raw);
    assert!(!import_staged(&model, &directory, || false).unwrap().created);
    let mut corrupt = raw;
    corrupt[1000] ^= 1;
    fs::write(&result.path, &corrupt).unwrap();
    assert!(import_staged(&model, &directory, || false).is_err());
    assert_eq!(fs::read(&result.path).unwrap(), corrupt);
    fs::remove_dir_all(directory).unwrap();
}
#[cfg(unix)]
#[test]
fn cancellation_preserves_only_task_owned_incomplete_object_and_no_selected_head() {
    use abi_wdbx::model::{error::ErrorKind, package::import_staged};
    let directory = support::temporary();
    let model = support::load(&fixture("inference")).unwrap();
    let mut checks = 0;
    assert_eq!(
        import_staged(&model, &directory, || {
            checks += 1;
            checks == 3
        })
        .unwrap_err()
        .kind,
        ErrorKind::Cancelled
    );
    let files = fs::read_dir(&directory)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(".partial"));
    assert!(import_staged(&model, &directory, || false).unwrap().created);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    fs::remove_dir_all(directory).unwrap();
}
#[cfg(unix)]
#[test]
fn existing_object_verification_observes_cancellation_before_its_read() {
    use abi_wdbx::model::{error::ErrorKind, package::import_staged};
    let directory = support::temporary();
    let model = support::load(&fixture("inference")).unwrap();
    import_staged(&model, &directory, || false).unwrap();
    let mut checks = 0;
    let result = import_staged(&model, &directory, || {
        checks += 1;
        checks >= 2
    });
    assert_eq!(result.unwrap_err().kind, ErrorKind::Cancelled);
    fs::remove_dir_all(directory).unwrap();
}
#[cfg(unix)]
#[test]
fn concurrent_stagers_resolve_one_object_and_symlinks_cannot_supply_it() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use abi_wdbx::model::package::import_staged;
    let directory = support::temporary();
    let model = support::load(&fixture("inference")).unwrap();
    let results = std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for _ in 0..8 {
            jobs.push(scope.spawn(|| import_staged(&model, &directory, || false).unwrap()));
        }
        jobs.into_iter()
            .map(|job| job.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| r.created).count(), 1);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    assert_eq!(
        fs::metadata(&results[0].path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let replacement = directory.join("content");
    fs::rename(&results[0].path, &replacement).unwrap();
    symlink(&replacement, &results[0].path).unwrap();
    assert!(import_staged(&model, &directory, || false).is_err());
    assert_eq!(fs::read(&replacement).unwrap(), fixture("inference"));
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn legacy_authenticated_object_reader_does_not_reinterpret_model_bytes() {
    use abi_wdbx::v2::{ObjectSecurity, open_object};
    assert!(open_object(&fixture("inference"), &ObjectSecurity::plaintext(), false).is_err());
}
#[test]
fn independently_decoded_writer_handles_a_different_shape_and_trained_tokenizer() {
    use abi_wdbx::model::package::build_package;
    let c = different_shape_records();
    let result = build_package(c, &[], LoadLimits::default()).unwrap();
    let directory = support::temporary();
    let path = directory.join("diagnostic.hex");
    let bytes = result.framed().snapshot().as_bytes();
    fs::write(&path, support::hex(bytes)).unwrap();
    let python = std::env::var_os("WDBX_PYTHON").unwrap_or_else(|| "python3".into());
    let output = std::process::Command::new(python)
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/wdbx_model_reference.py"))
        .arg("validate")
        .arg(path)
        .output()
        .expect("Python witness is required");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let checked: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let hex = |b: &[u8]| support::hex(b);
    assert_eq!(checked["state_root"], hex(&result.state_root().0));
    assert_eq!(checked["generation_id"], hex(&result.generation_id().0));
    assert_eq!(
        checked["package_digest"],
        hex(&result.framed().package_digest().0)
    );
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn coordinate_signed_zero_reexports_with_exact_binary64_bits() {
    use abi_wdbx::model::{contract::tensors::TensorValues, package::build_package};
    let raw = fixture("inference");
    let mut tensor = support::component(&raw, 3).to_vec();
    // Independent N01 worksheet: tensor2's first F64 is at component-relative120.
    tensor[120..128].copy_from_slice(&0x8000_0000_0000_0000u64.to_le_bytes());
    let raw = support::replace(&raw, 3, &tensor);
    let state = support::load(&raw).unwrap();
    if let TensorValues::F64(values) = &state.components().tensors[1].data {
        assert_eq!(values[0].to_bits(), 0x8000_0000_0000_0000);
    } else {
        panic!("coordinate dtype");
    }
    let output = build_package(state.components().clone(), &[], LoadLimits::default()).unwrap();
    assert_eq!(output.framed().snapshot().as_bytes(), raw);
}
#[test]
fn partial_export_io_failure_is_not_reported_as_a_complete_artifact() {
    struct Broken {
        bytes: Vec<u8>,
    }
    impl std::io::Write for Broken {
        fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
            if self.bytes.is_empty() {
                self.bytes.extend_from_slice(&input[..3]);
                Ok(3)
            } else {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            panic!("failed export must not flush or succeed")
        }
    }
    let state = support::load(&fixture("inference")).unwrap();
    let mut output = Broken { bytes: Vec::new() };
    let error = export_exact(&state, &mut output, || false).unwrap_err();
    assert_eq!(error.kind, abi_wdbx::model::error::ErrorKind::Io);
    assert_eq!(output.bytes, &fixture("inference")[..3]);
}
#[cfg(unix)]
#[test]
fn public_destination_directory_is_refused_before_any_artifact_write() {
    use abi_wdbx::model::package::import_staged;
    use std::os::unix::fs::PermissionsExt;
    let directory = support::temporary();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
    let model = support::load(&fixture("inference")).unwrap();
    let error = import_staged(&model, &directory, || false).unwrap_err();
    assert_eq!(error.field, "private-staging-directory");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn writer_refuses_dtype_variant_alias() {
    use abi_wdbx::model::{
        contract::tensors::TensorValues, error::ErrorKind, package::build_package,
    };
    let mut c = support::load(&fixture("inference"))
        .unwrap()
        .components()
        .clone();
    c.tensors[0].data = TensorValues::F64(vec![1.0; 3]);
    assert_eq!(
        build_package(c, &[], LoadLimits::default())
            .expect_err("inconsistent raw record was accepted")
            .kind,
        ErrorKind::InvalidTensor
    );
}
#[test]
fn writer_refuses_moment_length_redistribution() {
    use abi_wdbx::model::{
        contract::training::OptimizerState, error::ErrorKind, package::build_package,
    };
    let mut c = support::load(&fixture("resumable"))
        .unwrap()
        .components()
        .clone();
    if let OptimizerState::AdamW(o) = &mut c.optimizer {
        let moved = o.moments[0].first_moment.pop().unwrap();
        o.moments[0].second_moment.push(moved);
    }
    assert_eq!(
        build_package(c, &[], LoadLimits::default())
            .expect_err("inconsistent raw record was accepted")
            .kind,
        ErrorKind::InvalidOptimizer
    );
}
#[test]
fn writer_refuses_raw_rank_shape_and_declared_payload_length_disagreement() {
    use abi_wdbx::model::{
        contract::tensors::TensorValues, error::ErrorKind, package::build_package,
    };
    for case in 0..3 {
        let mut c = support::load(&fixture("inference"))
            .unwrap()
            .components()
            .clone();
        match case {
            0 => c.tensors[0].rank = 1,
            1 => c.tensors[0].data_byte_length -= 4,
            _ => {
                if let TensorValues::F32(values) = &mut c.tensors[0].data {
                    values.pop();
                }
            }
        }
        assert_eq!(
            build_package(c, &[], LoadLimits::default())
                .expect_err("inconsistent raw record was accepted")
                .kind,
            ErrorKind::InvalidTensor,
            "case {case}"
        );
    }
}

fn different_shape_records() -> abi_wdbx::model::contract::ModelComponents {
    use abi_wdbx::model::{
        contract::{
            tensors::{Axis, AxisKind, Dtype, TENSOR_CATALOG, TensorValues},
            topology::{Edge, Node, NodeRole, Relation},
        },
        ids::{EdgeId, NodeId, RelationId},
        tokenizer::{ByteBpe, TokenizerLimits},
    };
    let model = support::load(&fixture("inference")).unwrap();
    let mut c = model.components().clone();
    c.architecture.node_count = 4;
    c.architecture.edge_count = 3;
    c.architecture.hidden_width = 3;
    c.architecture.relation_count = 2;
    c.architecture.vocab_count = 262;
    c.topology.nodes.push(Node {
        id: NodeId(100),
        role: NodeRole::Hidden,
        source_ref: None,
    });
    c.topology.node_id_high_water = 100;
    c.topology.relations.push(Relation {
        id: RelationId(1),
        label: "second relation".to_owned(),
    });
    c.topology.edges.push(Edge {
        id: EdgeId(100),
        source: c.topology.nodes[1].id,
        destination: NodeId(100),
        relation: RelationId(1),
    });
    c.topology.edge_id_high_water = 100;
    let t = ByteBpe::train(
        &["aaaa", "aaaa"],
        c.tokenizer.training_split_digest,
        262,
        TokenizerLimits::default(),
        || false,
    )
    .unwrap();
    let old = c.tokenizer.training_corpus_digest;
    c.tokenizer = t.contract().clone();
    for root in &mut c.provenance.data_roots {
        if root.kind == abi_wdbx::model::contract::provenance::DataRootKind::TrainingCorpus
            && root.content_digest == old
        {
            root.content_digest = c.tokenizer.training_corpus_digest;
        }
    }
    for (t, spec) in c.tensors.iter_mut().zip(TENSOR_CATALOG) {
        t.axes = spec
            .axes
            .iter()
            .map(|kind| match kind {
                AxisKind::Node => Axis::Node(c.topology.nodes.iter().map(|n| n.id).collect()),
                AxisKind::Edge => Axis::Edge(c.topology.edges.iter().map(|e| e.id).collect()),
                AxisKind::Relation => Axis::Relation(2),
                AxisKind::Token => Axis::Token(262),
                AxisKind::HiddenOut => Axis::HiddenOut(3),
                AxisKind::HiddenIn => Axis::HiddenIn(3),
                AxisKind::Xyz => Axis::Xyz,
            })
            .collect();
        let count = spec
            .axes
            .iter()
            .map(|kind| match kind {
                AxisKind::Node => 4,
                AxisKind::Edge | AxisKind::HiddenOut | AxisKind::HiddenIn | AxisKind::Xyz => 3,
                AxisKind::Relation => 2,
                AxisKind::Token => 262,
            })
            .product::<usize>();
        t.data_byte_length = (count * if spec.dtype == Dtype::F32 { 4 } else { 8 }) as u64;
        t.data = if spec.dtype == Dtype::F32 {
            TensorValues::F32(vec![0.125; count])
        } else {
            TensorValues::F64(vec![0.5; count])
        };
    }

    c
}
