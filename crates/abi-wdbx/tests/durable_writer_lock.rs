//! Process-level writer-lock regressions for the v1 durable store.

use abi_wdbx::format::StorePaths;
use abi_wdbx::{DurableError, DurableStore};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const ROOT_ENV: &str = "ABI_DURABLE_WRITER_LOCK_ROOT";
const MODE_ENV: &str = "ABI_DURABLE_WRITER_LOCK_MODE";
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_STEP: Duration = Duration::from_millis(5);

struct Fixture {
    root: PathBuf,
    paths: StorePaths,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = abi_foundation::temp_path::temp_file_path(name, "store");
        let store = root.join("data");
        std::fs::create_dir_all(&store).expect("create fixture store");
        Self {
            root,
            paths: StorePaths {
                dir: store,
                base: "durable".to_string(),
            },
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn(root: &Path, mode: &str) -> Self {
        let child = Command::new(std::env::current_exe().expect("current integration test binary"))
            .args([
                "--exact",
                "child_process_entry",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ROOT_ENV, root)
            .env(MODE_ENV, mode)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn writer-lock child");
        Self(Some(child))
    }

    fn wait(mut self) -> ExitStatus {
        let deadline = Instant::now() + CHILD_TIMEOUT;
        loop {
            match self.0.as_mut().expect("child is present").try_wait() {
                Ok(Some(status)) => {
                    self.0 = None;
                    return status;
                }
                Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_STEP),
                Ok(None) => {
                    let child = self.0.as_mut().expect("child is present");
                    let _ = child.kill();
                    let _ = child.wait();
                    self.0 = None;
                    panic!("writer-lock child exceeded {CHILD_TIMEOUT:?}");
                }
                Err(error) => panic!("poll writer-lock child: {error}"),
            }
        }
    }

    fn terminate(mut self) {
        let child = self.0.as_mut().expect("child is present");
        child.kill().expect("terminate lock-owning child");
        child.wait().expect("reap lock-owning child");
        self.0 = None;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn paths(root: &Path) -> StorePaths {
    StorePaths {
        dir: root.join("data"),
        base: "durable".to_string(),
    }
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + CHILD_TIMEOUT;
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(POLL_STEP);
    }
}

fn store_artifacts(paths: &StorePaths) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(&paths.dir)
        .expect("read store artifacts")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (!name.ends_with(".writer.lock")).then(|| {
                (
                    name,
                    std::fs::read(entry.path()).expect("read store artifact"),
                )
            })
        })
        .collect()
}

#[test]
fn child_process_entry() {
    let Some(root) = std::env::var_os(ROOT_ENV).map(PathBuf::from) else {
        return;
    };
    let mode = std::env::var(MODE_ENV).expect("child mode");
    let paths = paths(&root);
    match mode.as_str() {
        "expect-busy" => assert!(matches!(
            DurableStore::open(paths),
            Err(DurableError::WriterBusy { .. })
        )),
        "open-success" => {
            let mut store = DurableStore::open(paths).expect("child opens released store");
            store.put("child", "completed").expect("child writes");
        }
        "own-until-terminated" => {
            let mut store = DurableStore::open(paths).expect("child owns writer lock");
            store.put("owner", "child").expect("owning child writes");
            std::fs::write(root.join("ready"), b"ready\n").expect("publish lock ownership");
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        other => panic!("unknown child mode {other:?}"),
    }
}

#[test]
fn parent_held_store_rejects_child_without_mutating_durable_artifacts() {
    if std::env::var_os(ROOT_ENV).is_some() {
        return;
    }
    let fixture = Fixture::new("abi_durable_parent_lock");
    {
        let mut seed = DurableStore::open(fixture.paths.clone()).expect("seed store");
        seed.put("checkpoint", "stable").expect("seed value");
        seed.checkpoint().expect("seed checkpoint");
    }
    let owner = DurableStore::open(fixture.paths.clone()).expect("parent owns writer lock");
    let before = store_artifacts(&fixture.paths);

    let status = ChildGuard::spawn(&fixture.root, "expect-busy").wait();
    assert!(
        status.success(),
        "child did not observe WriterBusy: {status}"
    );
    assert_eq!(store_artifacts(&fixture.paths), before);
    drop(owner);
}

#[test]
fn child_opens_after_normal_parent_drop() {
    if std::env::var_os(ROOT_ENV).is_some() {
        return;
    }
    let fixture = Fixture::new("abi_durable_parent_drop");
    let owner = DurableStore::open(fixture.paths.clone()).expect("parent owns writer lock");
    drop(owner);

    let status = ChildGuard::spawn(&fixture.root, "open-success").wait();
    assert!(
        status.success(),
        "child could not open released store: {status}"
    );
    let reopened = DurableStore::open(fixture.paths.clone()).expect("parent reopens after child");
    assert_eq!(reopened.get("child"), Some("completed"));
}

#[test]
fn parent_opens_after_terminating_lock_owning_child() {
    if std::env::var_os(ROOT_ENV).is_some() {
        return;
    }
    let fixture = Fixture::new("abi_durable_child_termination");
    let child = ChildGuard::spawn(&fixture.root, "own-until-terminated");
    wait_for(&fixture.root.join("ready"));
    child.terminate();

    let reopened = DurableStore::open(fixture.paths.clone())
        .expect("process termination releases the child writer lock");
    assert_eq!(reopened.get("owner"), Some("child"));
}
