//! `wdbx typed inspect|commit`: the typed-host ledger contract, through the real binary.
#![cfg(unix)]

use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BINARY: &str = env!("CARGO_BIN_EXE_wdbx");

struct Fixture {
    base: PathBuf,
    store: PathBuf,
    config: PathBuf,
    key: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.base).ok();
    }
}

fn private_file(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    std::io::Write::write_all(&mut file, bytes).unwrap();
}

fn config(valid_until: i64) -> Value {
    json!({
        "schema": "wdbx-typed-host-config-v1",
        "namespace": vec![1; 32],
        "current": {
            "policy": vec![3; 32],
            "consent": vec![4; 32],
            "valid_from": 0,
            "valid_until": valid_until,
            "grants": [{ "principal": vec![5; 32], "role": "service", "writer": vec![6; 16] }]
        },
        "history": []
    })
}

fn fixture_with(config: &Value) -> Fixture {
    let base = abi_foundation::temp_path::temp_file_path("wdbx-typed-cli", "d");
    std::fs::create_dir_all(&base).unwrap();
    let store = base.join("store");
    std::fs::create_dir(&store).unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config_path = base.join("config.json");
    private_file(&config_path, config.to_string().as_bytes());
    let key = base.join("host.bin");
    private_file(&key, &[9_u8; 32]);
    Fixture {
        base,
        store,
        config: config_path,
        key,
    }
}

fn fixture() -> Fixture {
    fixture_with(&config(4_000_000_000_000))
}

fn auth() -> Value {
    json!({
        "schema": "wdbx-typed-host-authorization-v1",
        "authorization": {
            "allowed": true, "writer_allowed": true, "principal": vec![5; 32], "role": "service",
            "namespace": vec![1; 32], "policy": vec![3; 32], "consent": vec![4; 32]
        }
    })
}

fn transaction(sequence: u64, value: &str) -> Value {
    let observed = if sequence <= 1 {
        json!([])
    } else {
        json!([{ "writer": vec![6; 16], "sequence": sequence - 1 }])
    };
    json!({
        "schema": "wdbx-typed-transaction-input-v1",
        "transaction": {
            "writer": vec![6; 16], "sequence": sequence, "nonce": vec![sequence; 32],
            "observed": observed, "mutations": [{ "kv": { "key": "invoice", "value": value } }]
        }
    })
}

impl Fixture {
    fn inspect_with(&self, key: &Path) -> Output {
        Command::new(BINARY)
            .args(["typed", "inspect"])
            .args([&self.store, &self.config, key])
            .output()
            .unwrap()
    }

    fn inspect(&self) -> Value {
        let output = self.inspect_with(&self.key);
        assert!(output.status.success(), "{}", stderr(&output));
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn commit_command(&self, auth: &Value, transaction: &Value) -> Command {
        let request = tempfile_dir(&self.base);
        let auth_path = request.join("auth.json");
        let transaction_path = request.join("transaction.json");
        private_file(&auth_path, auth.to_string().as_bytes());
        private_file(&transaction_path, transaction.to_string().as_bytes());
        let mut command = Command::new(BINARY);
        command.args(["typed", "commit"]).args([
            &self.store,
            &self.config,
            &self.key,
            &auth_path,
            &transaction_path,
        ]);
        command
    }

    fn commit(&self, auth: &Value, transaction: &Value) -> Output {
        self.commit_command(auth, transaction).output().unwrap()
    }
}

fn tempfile_dir(base: &Path) -> PathBuf {
    let directory = base.join(format!("request-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: &Output, code: i32, name: &str) {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(
        stderr(output).contains(name),
        "expected {name}: {}",
        stderr(output)
    );
}

#[test]
fn fresh_store_inspects_empty_without_creating_files() {
    let fixture = fixture();
    let inspection = fixture.inspect();
    assert_eq!(
        inspection,
        json!({
            "schema": "wdbx-typed-inspection-v1",
            "namespace": "01".repeat(32),
            "frontier": [],
            "rows": []
        })
    );
    assert_eq!(std::fs::read_dir(&fixture.store).unwrap().count(), 0);
}

#[test]
fn commits_publish_contiguous_sequences_that_new_processes_read() {
    let fixture = fixture();
    let first = fixture.commit(&auth(), &transaction(1, "sealed-one"));
    assert!(first.status.success(), "{}", stderr(&first));
    let receipt: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(receipt["sequence"], 1);
    assert_eq!(receipt["writer_id"], "06".repeat(16));
    let second = fixture.commit(&auth(), &transaction(2, "sealed-two"));
    assert!(second.status.success(), "{}", stderr(&second));
    let inspection = fixture.inspect();
    assert_eq!(
        inspection["frontier"],
        json!([{ "writer_id": "06".repeat(16), "sequence": 2 }])
    );
    let rows = inspection["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["current"], true);
    assert_eq!(rows[0]["conflict_ids"].as_array().unwrap().len(), 1);
    assert_eq!(
        rows[0]["mutation"],
        json!({ "kv": { "key": "invoice", "value": "sealed-two" } })
    );
}

#[test]
fn a_transaction_that_observed_another_frontier_is_stale() {
    let fixture = fixture();
    assert!(
        fixture
            .commit(&auth(), &transaction(1, "first"))
            .status
            .success()
    );
    assert_refused(
        &fixture.commit(&auth(), &transaction(1, "replayed")),
        3,
        "StaleFrontier",
    );
    let mut skipped = transaction(2, "skips the frontier");
    skipped["transaction"]["observed"] = json!([]);
    assert_refused(&fixture.commit(&auth(), &skipped), 3, "StaleFrontier");
    assert_refused(
        &fixture.commit(&auth(), &transaction(3, "ahead")),
        3,
        "StaleFrontier",
    );
    assert_eq!(fixture.inspect()["frontier"][0]["sequence"], 1);
}

#[test]
fn foreign_writers_and_mismatched_authorization_are_denied() {
    let fixture = fixture();
    let mut foreign = transaction(1, "foreign");
    foreign["transaction"]["writer"] = json!(vec![7; 16]);
    assert_refused(&fixture.commit(&auth(), &foreign), 1, "AdmissionDenied");
    for (field, value) in [
        ("allowed", json!(false)),
        ("writer_allowed", json!(false)),
        ("role", json!("admin")),
        ("principal", json!(vec![8; 32])),
        ("namespace", json!(vec![2; 32])),
        ("policy", json!(vec![0; 32])),
        ("consent", json!(vec![0; 32])),
    ] {
        let mut denied = auth();
        denied["authorization"][field] = value;
        assert_refused(
            &fixture.commit(&denied, &transaction(1, "denied")),
            1,
            "AdmissionDenied",
        );
    }
    let mut extra = auth();
    extra["authorization"]["admin"] = json!(true);
    assert_refused(
        &fixture.commit(&extra, &transaction(1, "denied")),
        1,
        "InvalidInput",
    );
    assert_eq!(fixture.inspect()["frontier"], json!([]));
}

#[test]
fn an_expired_epoch_admits_no_commit() {
    let fixture = fixture_with(&config(1));
    assert_refused(
        &fixture.commit(&auth(), &transaction(1, "late")),
        1,
        "AdmissionDenied",
    );
}

#[test]
fn shared_or_relative_paths_are_refused() {
    let fixture = fixture();
    std::fs::set_permissions(&fixture.key, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_refused(&fixture.inspect_with(&fixture.key), 1, "UnsafePath");
    std::fs::set_permissions(&fixture.key, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(&fixture.store, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_refused(&fixture.inspect_with(&fixture.key), 1, "UnsafePath");
    std::fs::set_permissions(&fixture.store, std::fs::Permissions::from_mode(0o700)).unwrap();
    let relative = Command::new(BINARY)
        .args(["typed", "inspect", "store"])
        .args([&fixture.config, &fixture.key])
        .output()
        .unwrap();
    assert_refused(&relative, 1, "UnsafePath");
    let usage = Command::new(BINARY).arg("typed").output().unwrap();
    assert_refused(&usage, 2, "Usage");
}

#[test]
fn another_host_key_cannot_authenticate_the_ledger() {
    let fixture = fixture();
    assert!(
        fixture
            .commit(&auth(), &transaction(1, "sealed"))
            .status
            .success()
    );
    let other = fixture.base.join("other.bin");
    private_file(&other, &[10_u8; 32]);
    assert_refused(&fixture.inspect_with(&other), 1, "AdmissionInvalid");
}

#[test]
fn a_held_typed_lock_reports_writer_busy() {
    let fixture = fixture();
    let lock = File::create(fixture.store.join("typed.lock")).unwrap();
    lock.lock().unwrap();
    assert_refused(
        &fixture.commit(&auth(), &transaction(1, "blocked")),
        75,
        "WriterBusy",
    );
    drop(lock);
    assert!(
        fixture
            .commit(&auth(), &transaction(1, "admitted"))
            .status
            .success()
    );
}

#[test]
fn concurrent_processes_at_one_sequence_admit_exactly_one() {
    let fixture = fixture();
    let children: Vec<_> = (0..4)
        .map(|index| {
            fixture
                .commit_command(&auth(), &transaction(1, &format!("racer-{index}")))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    let outcomes: Vec<_> = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect();
    let admitted = outcomes.iter().filter(|o| o.status.success()).count();
    assert_eq!(admitted, 1);
    for refused in outcomes.iter().filter(|o| !o.status.success()) {
        let message = stderr(refused);
        assert!(
            message.contains("StaleFrontier") || message.contains("WriterBusy"),
            "{message}"
        );
    }
    let inspection = fixture.inspect();
    assert_eq!(inspection["frontier"][0]["sequence"], 1);
    assert_eq!(
        inspection["rows"][0]["conflict_ids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

fn stored_envelope(fixture: &Fixture) -> String {
    abi_wdbx::read_v2_root(&fixture.store, &abi_wdbx::v2::ObjectSecurity::plaintext())
        .unwrap()
        .preferred_value("invoice")
        .unwrap()
        .to_owned()
}

#[test]
fn superseded_values_must_authenticate_before_inspect_or_commit() {
    use abi_wdbx::{V2Mutation, V2Store};
    let source = fixture();
    assert!(
        source
            .commit(&auth(), &transaction(1, "first"))
            .status
            .success()
    );
    let first = stored_envelope(&source);
    assert!(
        source
            .commit(&auth(), &transaction(2, "second"))
            .status
            .success()
    );
    let second = stored_envelope(&source);
    for tamper in [false, true] {
        let target = fixture();
        let mut envelope: Value = serde_json::from_str(&first).unwrap();
        if tamper {
            envelope["value"] = json!("forged superseded value");
        }
        let mut store = V2Store::open(&target.store).unwrap();
        store
            .commit(vec![V2Mutation::PutKv {
                key: "invoice".into(),
                value: envelope.to_string(),
            }])
            .unwrap();
        store
            .commit(vec![V2Mutation::PutKv {
                key: "invoice".into(),
                value: second.clone(),
            }])
            .unwrap();
        drop(store);
        // The ordinary writer builds internally consistent V2 objects; only
        // the historical envelope's keyed authentication is invalid.
        if tamper {
            assert_refused(&target.inspect_with(&target.key), 1, "AdmissionInvalid");
            assert_refused(
                &target.commit(&auth(), &transaction(3, "third")),
                1,
                "AdmissionInvalid",
            );
            assert_eq!(
                abi_wdbx::read_v2_root(&target.store, &abi_wdbx::v2::ObjectSecurity::plaintext())
                    .unwrap()
                    .committed_transactions(),
                2
            );
        } else {
            assert_eq!(target.inspect()["frontier"][0]["sequence"], 2);
            assert!(
                target
                    .commit(&auth(), &transaction(3, "third"))
                    .status
                    .success()
            );
        }
    }
}

#[test]
fn typed_stores_reject_temporal_nodes_and_edges() {
    use abi_wdbx::v2::{V2TemporalKind, V2TemporalRecord};
    use abi_wdbx::{V2Mutation, V2Store};
    let source = fixture();
    assert!(
        source
            .commit(&auth(), &transaction(1, "valid"))
            .status
            .success()
    );
    let value = stored_envelope(&source);
    for kind in [V2TemporalKind::Node, V2TemporalKind::Edge] {
        let target = fixture();
        let mut store = V2Store::open(&target.store).unwrap();
        store
            .commit(vec![
                V2Mutation::PutKv {
                    key: "invoice".into(),
                    value: value.clone(),
                },
                V2Mutation::PutTemporal {
                    key: "extra".into(),
                    record: V2TemporalRecord {
                        kind,
                        fields: serde_json::Map::new(),
                    },
                },
            ])
            .unwrap();
        drop(store);
        assert_refused(&target.inspect_with(&target.key), 1, "StoreCorrupt");
        assert_refused(
            &target.commit(&auth(), &transaction(2, "next")),
            1,
            "StoreCorrupt",
        );
        assert_eq!(
            abi_wdbx::read_v2_root(&target.store, &abi_wdbx::v2::ObjectSecurity::plaintext())
                .unwrap()
                .committed_transactions(),
            1
        );
    }
}
