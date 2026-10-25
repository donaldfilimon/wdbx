//! Task-owned objects only; no head pointer, journal, registry or activation.
use super::{Result, cancelled, err, export_exact, require};
use crate::model::{ValidatedModelState, contract::PACKAGE_DOMAIN, error::ErrorKind as E};
use std::fmt::Write as _;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// Durable content object. This is not a published/selected model generation.
#[derive(Debug)]
pub struct StagedObject {
    /// Content-addressed file under the explicitly supplied private directory.
    pub path: PathBuf,
    /// Whether this call linked a fresh object rather than verifying an existing one.
    pub created: bool,
}
/// Stage captured bytes, sync file, atomically hard-link without clobbering, sync
/// directory. Unix private-directory support only; other platforms explicitly
/// return Io with field `unsupported-directory-durability` before any writes.
/// Destination must be a caller-owned private directory, not attacker-controlled.
/// Incomplete create-new files are retained for explicit cleanup on failure.
pub fn import_staged(
    model: &ValidatedModelState,
    directory: &Path,
    mut cancel: impl FnMut() -> bool,
) -> Result<StagedObject> {
    cancelled(&mut cancel)?;
    platform_directory(directory)?;
    let mut name = String::new();
    name.try_reserve_exact(64)
        .map_err(|_| err(E::CapacityExceeded, "object-name"))?;
    for byte in model.framed().package_digest().0 {
        write!(&mut name, "{byte:02x}").expect("fixed hex output");
    }
    let path = directory.join(format!("{name}.wdbx"));
    if fs::symlink_metadata(&path).is_ok() {
        verify_existing(model, &path, &mut cancel)?;
        sync_directory(directory)?;
        return Ok(StagedObject {
            path,
            created: false,
        });
    }
    let sequence = NEXT
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| err(E::CapacityExceeded, "staging-operation-counter"))?;
    let temporary = directory.join(format!(
        ".wdbx-stage-{}-{sequence}.partial",
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    private_options(&mut options);
    let mut file = options
        .open(&temporary)
        .map_err(|_| err(E::Io, "staging-create-new"))?;
    export_exact(model, &mut file, &mut cancel)?;
    file.sync_all()
        .map_err(|_| err(E::Io, "staging-file-sync"))?;
    cancelled(&mut cancel)?;
    let created = match fs::hard_link(&temporary, &path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            verify_existing(model, &path, &mut cancel)?;
            false
        }
        Err(_) => return Err(err(E::Io, "staging-no-clobber-link")),
    };
    sync_directory(directory)?;
    // Remove only this call's uniquely created, fully synced temporary name.
    fs::remove_file(&temporary).map_err(|_| err(E::Io, "staging-temporary-unlink"))?;
    sync_directory(directory)?;
    Ok(StagedObject { path, created })
}
fn verify_existing(
    model: &ValidatedModelState,
    path: &Path,
    cancel: &mut impl FnMut() -> bool,
) -> Result<()> {
    use sha2::{Digest as _, Sha256};
    use std::io::Read;
    let metadata = fs::symlink_metadata(path).map_err(|_| err(E::Io, "existing-metadata"))?;
    let total = model.framed().snapshot().as_bytes().len() as u64;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() == total,
        E::IntegrityMismatch,
        "existing-object-type-size",
    )?;
    let mut file = File::open(path).map_err(|_| err(E::Io, "existing-open"))?;
    let mut hash = Sha256::new();
    hash.update(PACKAGE_DOMAIN);
    let mut remaining = total;
    let mut work = [0; 8192];
    while remaining > 0 {
        cancelled(cancel)?;
        let n = file
            .read(&mut work[..super::size(remaining.min(8192))?])
            .map_err(|_| err(E::Io, "existing-read"))?;
        require(n > 0, E::IntegrityMismatch, "existing-truncated")?;
        hash.update(&work[..n]);
        remaining -= n as u64;
    }
    cancelled(cancel)?;
    let n = file
        .read(&mut work[..1])
        .map_err(|_| err(E::Io, "existing-eof"))?;
    let digest: [u8; 32] = hash.finalize().into();
    require(
        n == 0 && digest == model.framed().package_digest().0,
        E::IntegrityMismatch,
        "existing-object-digest",
    )
}
#[cfg(unix)]
fn platform_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::symlink_metadata(path).map_err(|_| err(E::Io, "staging-directory"))?;
    require(
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && metadata.permissions().mode().trailing_zeros() >= 6,
        E::Io,
        "private-staging-directory",
    )
}
#[cfg(not(unix))]
fn platform_directory(_: &Path) -> Result<()> {
    Err(err(E::Io, "unsupported-directory-durability"))
}
#[cfg(unix)]
fn private_options(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}
#[cfg(not(unix))]
fn private_options(_: &mut OpenOptions) {}
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| err(E::Io, "staging-directory-sync"))
}
