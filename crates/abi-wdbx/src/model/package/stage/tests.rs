//! Real file reads and object paths; only sync failure is injected.
use super::*;
use std::{
    cell::{Cell, RefCell},
    os::unix::fs::{MetadataExt, PermissionsExt},
};
fn state() -> ValidatedModelState {
    let text = include_str!("../../../../tests/fixtures/model-v1/inference.hex");
    let raw = text
        .trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let limits = crate::model::limits::LoadLimits::default();
    super::super::validate_model(
        super::super::validate_framing(
            super::super::read_bounded_snapshot(raw.as_slice(), limits).unwrap(),
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap()
}
fn directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("wdbx-sync-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
struct RecordingDurability {
    files: Cell<usize>,
    directories: Cell<usize>,
    fail_on: usize,
    inodes: RefCell<Vec<u64>>,
}
impl RecordingDurability {
    fn new(fail_on: usize) -> Self {
        Self {
            files: Cell::new(0),
            directories: Cell::new(0),
            fail_on,
            inodes: RefCell::new(Vec::new()),
        }
    }
}
impl Durability for RecordingDurability {
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        let call = self.files.get() + 1;
        self.files.set(call);
        self.inodes.borrow_mut().push(file.metadata()?.ino());
        if call == self.fail_on {
            return Err(std::io::ErrorKind::Other.into());
        }
        file.sync_all()
    }
    fn sync_directory(&self, path: &Path) -> std::io::Result<()> {
        self.directories.set(self.directories.get() + 1);
        File::open(path)?.sync_all()
    }
}
#[test]
fn verified_existing_file_is_synced_before_durable_success_and_sync_failure_refuses() {
    let model = state();
    let root = directory();
    let object = import_staged(&model, &root, || false).unwrap();
    let inode = fs::metadata(&object.path).unwrap().ino();
    let ok = RecordingDurability::new(usize::MAX);
    let result = import_with_durability(&model, &root, || false, &ok).unwrap();
    assert!(!result.created);
    assert_eq!(ok.files.get(), 1);
    assert_eq!(ok.directories.get(), 1);
    assert_eq!(*ok.inodes.borrow(), [inode]);
    let fail = RecordingDurability::new(1);
    let error = import_with_durability(&model, &root, || false, &fail)
        .expect_err("unsynced existing object accepted");
    assert_eq!(error.field, "existing-file-sync");
    assert_eq!(fail.directories.get(), 0);
    assert_eq!(*fail.inodes.borrow(), [inode]);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn no_clobber_collision_cannot_skip_existing_file_sync_or_ignore_its_failure() {
    let model = state();
    let root = directory();
    let name = model
        .framed()
        .package_digest()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let path = root.join(format!("{name}.wdbx"));
    let checks = Cell::new(0);
    let durability = RecordingDurability::new(2);
    let error = import_with_durability(
        &model,
        &root,
        || {
            let n = checks.get() + 1;
            checks.set(n);
            if n == 4 {
                fs::write(&path, model.framed().snapshot().as_bytes()).unwrap();
            }
            false
        },
        &durability,
    )
    .expect_err("collision accepted without verified-file sync");
    assert_eq!(error.field, "existing-file-sync");
    assert_eq!(durability.files.get(), 2);
    assert_eq!(durability.directories.get(), 0);
    assert_eq!(
        durability.inodes.borrow()[1],
        fs::metadata(&path).unwrap().ino()
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        model.framed().snapshot().as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}
/// Removes this call's temporary during the post-link directory sync, so the
/// later unlink fails after the object is already linked and durable (N02 M3).
struct TemporaryVanishes;
impl Durability for TemporaryVanishes {
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        file.sync_all()
    }
    fn sync_directory(&self, path: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().ends_with(".partial") {
                fs::remove_file(entry.path())?;
            }
        }
        File::open(path)?.sync_all()
    }
}
#[test]
fn stage_reports_success_when_only_temporary_unlink_fails() {
    let model = state();
    let root = directory();
    let object = import_with_durability(&model, &root, || false, &TemporaryVanishes)
        .expect("durable linked object reported as failure");
    assert!(object.created);
    assert_eq!(
        fs::read(&object.path).unwrap(),
        model.framed().snapshot().as_bytes()
    );
    let again = import_staged(&model, &root, || false).unwrap();
    assert!(!again.created);
    fs::remove_dir_all(root).unwrap();
}
/// Replaces this call's temporary with a non-empty directory during the
/// post-link sync, so `remove_file` really fails and the name is retained.
struct TemporaryPinned;
impl Durability for TemporaryPinned {
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        file.sync_all()
    }
    fn sync_directory(&self, path: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().ends_with(".partial") && entry.path().is_file() {
                fs::remove_file(entry.path())?;
                fs::create_dir(entry.path())?;
                fs::write(entry.path().join("pin"), b"x")?;
            }
        }
        File::open(path)?.sync_all()
    }
}
#[test]
fn failed_temporary_unlink_after_durable_link_is_retained_not_reported() {
    let model = state();
    let root = directory();
    let object = import_with_durability(&model, &root, || false, &TemporaryPinned)
        .expect("durable linked object reported as failure");
    assert!(object.created);
    let mut names = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".partial")));
    assert_eq!(
        fs::read(&object.path).unwrap(),
        model.framed().snapshot().as_bytes()
    );
    fs::remove_dir_all(root).unwrap();
}
