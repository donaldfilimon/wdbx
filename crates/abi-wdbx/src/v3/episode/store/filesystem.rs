//! Private filesystem preparation and directory-entry synchronization.
//!
//! These are path-based checks, not descriptor-relative protection against a
//! concurrent actor replacing a checked path. On Unix, synchronization errors
//! are initialization failures. Other platforms retain file synchronization
//! only: portable Rust does not provide directory-entry synchronization there.
//! These helpers do not establish power-loss evidence.

use super::{EpisodeStoreError, MAX_LEDGER_BYTES};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

/// Tolerate fork/exec-inherited descriptors without bypassing writer exclusion.
pub(super) fn acquire_writer_lock(file: &File) -> Result<(), EpisodeStoreError> {
    let deadline = Instant::now() + Duration::from_millis(50);
    acquire_writer_lock_with_retry(
        || file.try_lock(),
        || {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
            true
        },
    )
}

fn acquire_writer_lock_with_retry(
    mut try_lock: impl FnMut() -> Result<(), TryLockError>,
    mut retry: impl FnMut() -> bool,
) -> Result<(), EpisodeStoreError> {
    // A fork can briefly retain a dropped writer's open file description until
    // exec closes the inherited descriptor. Retry only contention, and never
    // unlock the successful handle: the caller retains it for the store lifetime.
    loop {
        match try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if !retry() {
                    return Err(EpisodeStoreError::WriterBusy);
                }
            }
            Err(TryLockError::Error(_)) => return Err(EpisodeStoreError::Io),
        }
    }
}

pub(super) fn prepare_directory(path: &Path) -> Result<PathBuf, EpisodeStoreError> {
    prepare_directory_with_sync(path, sync_directory)
}

fn prepare_directory_with_sync(
    path: &Path,
    mut synchronize: impl FnMut(&Path) -> Result<(), EpisodeStoreError>,
) -> Result<PathBuf, EpisodeStoreError> {
    // Ancestor directory aliases (including macOS /var) remain valid. Only
    // the requested store itself must not be a symlink.
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(EpisodeStoreError::Io);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(EpisodeStoreError::Io),
    }
    let absolute = std::path::absolute(path).map_err(|_| EpisodeStoreError::Io)?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match fs::metadata(ancestor) {
            Ok(metadata) if metadata.is_dir() => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let component = ancestor
                    .components()
                    .next_back()
                    .ok_or(EpisodeStoreError::Io)?;
                if !matches!(component, Component::Normal(_) | Component::ParentDir) {
                    return Err(EpisodeStoreError::Io);
                }
                missing.push(component.as_os_str().to_os_string());
                ancestor = ancestor.parent().ok_or(EpisodeStoreError::Io)?;
            }
            Ok(_) | Err(_) => return Err(EpisodeStoreError::Io),
        }
    }
    let mut current = ancestor.canonicalize().map_err(|_| EpisodeStoreError::Io)?;
    // An earlier failed initialization may have created this nearest existing
    // directory without successfully fencing its entry. Do not walk farther up
    // and synchronize unrelated ancestors (or the root when it has no parent).
    if let Some(parent) = current.parent() {
        synchronize(parent)?;
    }
    for component in missing.into_iter().rev() {
        if component == ".." {
            // Resolve after creating the preceding component: lexical collapse
            // would change filesystem semantics for missing/../store paths.
            current = current
                .join(component)
                .canonicalize()
                .map_err(|_| EpisodeStoreError::Io)?;
            continue;
        }
        let child = current.join(component);
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&child) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&child).map_err(|_| EpisodeStoreError::Io)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(EpisodeStoreError::Io);
                }
            }
            Err(_) => return Err(EpisodeStoreError::Io),
        }
        // Publish each newly created entry before creating the next descendant.
        synchronize(&current)?;
        current = child;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&current, fs::Permissions::from_mode(0o700))
            .map_err(|_| EpisodeStoreError::Io)?;
    }
    Ok(current)
}

/// Fence ledger/lock directory entries on Unix; validate only elsewhere.
///
/// Directory handles opened by portable `File::open` cannot be synchronized on
/// every platform (notably Windows). Preserve those platforms' prior file-only
/// durability rather than making every existing store fail to open.
pub(super) fn sync_directory(path: &Path) -> Result<(), EpisodeStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| EpisodeStoreError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(EpisodeStoreError::Io);
    }
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| EpisodeStoreError::Io)
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

pub(super) fn reject_symlink_if_present(path: &Path) -> Result<(), EpisodeStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(EpisodeStoreError::Io)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(EpisodeStoreError::Io),
    }
}

pub(super) fn owner_file(path: &Path) -> Result<File, EpisodeStoreError> {
    reject_symlink_if_present(path)?;
    let mut options = OpenOptions::new();
    options.create(true).read(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| EpisodeStoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| EpisodeStoreError::Io)?;
    }
    Ok(file)
}

pub(super) fn read_bounded_ledger(path: &Path) -> Result<Vec<u8>, EpisodeStoreError> {
    read_bounded_file(path)
}

/// Read a store file whole, bounded, creating it when it does not exist.
///
/// The retention index is read exactly like the ledger: the file is created
/// when it is missing, its size is bounded, and a symlink in its place is
/// refused.
pub(super) fn read_bounded_file(path: &Path) -> Result<Vec<u8>, EpisodeStoreError> {
    reject_symlink_if_present(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            owner_file(path)?
                .sync_data()
                .map_err(|_| EpisodeStoreError::Io)?;
        }
        Err(_) => return Err(EpisodeStoreError::Io),
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| EpisodeStoreError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_LEDGER_BYTES
    {
        return Err(EpisodeStoreError::Io);
    }
    // Keep the allocation bounded even if another actor grows the file after
    // the metadata check. The writer lock is not an adversarial security fence.
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| EpisodeStoreError::Io)?
        .take(MAX_LEDGER_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| EpisodeStoreError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| EpisodeStoreError::Io)? > MAX_LEDGER_BYTES {
        return Err(EpisodeStoreError::Io);
    }
    Ok(raw)
}

/// Read a store file whole, bounded, when it exists.
///
/// The retention index is derived state: a store that never bound a TTL has
/// no index file at all, and one that lost its index keeps its ledger. This
/// reads the file when it is there and reports its absence otherwise, without
/// creating anything.
pub(super) fn read_bounded_file_if_present(
    path: &Path,
) -> Result<Option<Vec<u8>>, EpisodeStoreError> {
    reject_symlink_if_present(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_LEDGER_BYTES
            {
                return Err(EpisodeStoreError::Io);
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(EpisodeStoreError::Io),
    }
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| EpisodeStoreError::Io)?
        .take(MAX_LEDGER_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| EpisodeStoreError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| EpisodeStoreError::Io)? > MAX_LEDGER_BYTES {
        return Err(EpisodeStoreError::Io);
    }
    Ok(Some(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            // macOS temporary paths may themselves contain a system symlink.
            let base = std::env::temp_dir().canonicalize().unwrap();
            let path = base.join(format!("wdbx-episode-filesystem-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn writer_lock_is_retained_until_the_owner_is_dropped() {
        let root = TestDirectory::new();
        let path = root.0.join("writer.lock");
        let owner = owner_file(&path).unwrap();
        let contender = owner_file(&path).unwrap();
        acquire_writer_lock(&owner).unwrap();
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        drop(owner);
        acquire_writer_lock(&contender).unwrap();
    }

    #[test]
    fn transient_writer_contention_is_retried_after_release() {
        let root = TestDirectory::new();
        let path = root.0.join("writer.lock");
        let mut owner = Some(owner_file(&path).unwrap());
        owner.as_ref().unwrap().try_lock().unwrap();
        let contender = owner_file(&path).unwrap();
        let mut attempts = 0;
        acquire_writer_lock_with_retry(
            || {
                attempts += 1;
                contender.try_lock()
            },
            || {
                // Release only after observing real contention, without relying
                // on thread scheduling or a wall-clock timeout in this test.
                drop(owner.take().expect("only one retry is needed"));
                true
            },
        )
        .unwrap();
        assert_eq!(attempts, 2);
        let another = owner_file(&path).unwrap();
        assert!(matches!(another.try_lock(), Err(TryLockError::WouldBlock)));
    }

    #[test]
    fn persistent_writer_contention_is_bounded_without_mutation() {
        let root = TestDirectory::new();
        let path = root.0.join("writer.lock");
        let ledger = root.0.join("ledger");
        fs::write(&path, b"lock sentinel").unwrap();
        fs::write(&ledger, b"ledger sentinel").unwrap();
        let owner = owner_file(&path).unwrap();
        owner.try_lock().unwrap();
        let contender = owner_file(&path).unwrap();
        let mut attempts = 0;
        let mut waits = 0;
        let result = acquire_writer_lock_with_retry(
            || {
                attempts += 1;
                contender.try_lock()
            },
            || {
                waits += 1;
                waits < 3
            },
        );
        assert!(matches!(result, Err(EpisodeStoreError::WriterBusy)));
        assert_eq!(attempts, 3);
        assert_eq!(waits, 3);
        assert_eq!(fs::read(&path).unwrap(), b"lock sentinel");
        assert_eq!(fs::read(&ledger).unwrap(), b"ledger sentinel");
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
    }

    #[test]
    fn writer_lock_io_errors_are_not_retried() {
        let mut attempts = 0;
        let result = acquire_writer_lock_with_retry(
            || {
                attempts += 1;
                Err(TryLockError::Error(io::Error::other("injected failure")))
            },
            || panic!("real lock errors must not wait or retry"),
        );
        assert!(matches!(result, Err(EpisodeStoreError::Io)));
        assert_eq!(attempts, 1);
    }

    #[test]
    fn nested_entries_are_synchronized_before_descending() {
        let root = TestDirectory::new();
        let first = root.0.join("first");
        let store = first.join("store");
        let mut calls = Vec::new();
        let prepared = prepare_directory_with_sync(&store, |parent| {
            calls.push(parent.to_path_buf());
            if parent == root.0 {
                assert!(first.is_dir());
                assert!(!store.exists());
            }
            if parent == first {
                assert!(store.is_dir());
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(prepared, store);
        assert_eq!(
            calls,
            [
                root.0.parent().unwrap().to_path_buf(),
                root.0.clone(),
                first
            ]
        );
        assert!(!calls.contains(&store)); // Caller fences ledger/lock entries last.
    }

    #[test]
    fn failed_parent_sync_stops_creation_and_is_retried() {
        let root = TestDirectory::new();
        let first = root.0.join("first");
        let store = first.join("store");
        let result = prepare_directory_with_sync(&store, |parent| {
            if parent == root.0 {
                Err(EpisodeStoreError::Io)
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(EpisodeStoreError::Io)));
        assert!(first.is_dir());
        assert!(!store.exists());
        let mut retried_parent = false;
        prepare_directory_with_sync(&store, |parent| {
            if parent == root.0 {
                retried_parent = true;
                assert!(!store.exists());
            }
            Ok(())
        })
        .unwrap();
        assert!(retried_parent);
        assert!(store.is_dir());
    }

    #[test]
    fn final_parent_sync_failure_is_not_swallowed() {
        let root = TestDirectory::new();
        let store = root.0.join("store");
        assert!(matches!(
            prepare_directory_with_sync(&store, |parent| {
                if parent == root.0 {
                    Err(EpisodeStoreError::Io)
                } else {
                    Ok(())
                }
            }),
            Err(EpisodeStoreError::Io)
        ));
        assert!(store.is_dir());
        let mut calls = Vec::new();
        prepare_directory_with_sync(&store, |parent| {
            calls.push(parent.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(calls.last(), Some(&root.0));
    }

    #[test]
    fn existing_directory_sync_is_bounded_and_failure_propagates() {
        let root = TestDirectory::new();
        let store = root.0.join("store");
        fs::create_dir(&store).unwrap();
        let mut calls = Vec::new();
        assert!(matches!(
            prepare_directory_with_sync(&store, |parent| {
                calls.push(parent.to_path_buf());
                Err(EpisodeStoreError::Io)
            }),
            Err(EpisodeStoreError::Io)
        ));
        assert_eq!(calls.as_slice(), std::slice::from_ref(&root.0));
        calls.clear();
        prepare_directory_with_sync(&store, |parent| {
            calls.push(parent.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(calls.as_slice(), std::slice::from_ref(&root.0));
    }

    #[test]
    fn relative_dot_and_parent_components_preserve_filesystem_semantics() {
        let root = TestDirectory::new();
        let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        // Construct a relative path to our disposable directory without changing
        // the process working directory or changing its permissions.
        let mut relative = PathBuf::from(".");
        let common = cwd
            .ancestors()
            .find(|base| root.0.starts_with(base))
            .unwrap();
        for _ in cwd.strip_prefix(common).unwrap().components() {
            relative.push("..");
        }
        relative.push(root.0.strip_prefix(common).unwrap());
        let store = relative.join("./missing/../store");
        let mut calls = Vec::new();
        let prepared = prepare_directory_with_sync(&store, |parent| {
            calls.push(parent.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(prepared, root.0.join("store"));
        assert!(root.0.join("missing").is_dir());
        assert_eq!(
            calls,
            [
                root.0.parent().unwrap().to_path_buf(),
                root.0.clone(),
                root.0.clone()
            ]
        );
        let prepared =
            prepare_directory_with_sync(&root.0.join("missing/.././store"), |_| Ok(())).unwrap();
        assert_eq!(prepared, root.0.join("store"));
    }

    #[cfg(unix)]
    #[test]
    fn real_directory_sync_and_file_creation() {
        let root = TestDirectory::new();
        let store = prepare_directory(&root.0.join("nested/store")).unwrap();
        let ledger = store.join("ledger");
        assert_eq!(read_bounded_ledger(&ledger).unwrap(), Vec::<u8>::new());
        let file = owner_file(&ledger).unwrap();
        file.sync_data().unwrap();
        sync_directory(&store).unwrap();
        assert!(matches!(
            sync_directory(&ledger),
            Err(EpisodeStoreError::Io)
        ));
        assert!(matches!(
            sync_directory(&store.join("absent")),
            Err(EpisodeStoreError::Io)
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for directory in [store.parent().unwrap(), store.as_path()] {
                assert_eq!(
                    fs::metadata(directory).unwrap().permissions().mode() & 0o777,
                    0o700
                );
            }
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[cfg(not(unix))]
    #[test]
    fn unsupported_directory_sync_preserves_file_only_open_behavior() {
        let root = TestDirectory::new();
        let store = prepare_directory(&root.0.join("nested/store")).unwrap();
        let ledger = store.join("ledger");
        assert_eq!(read_bounded_ledger(&ledger).unwrap(), Vec::<u8>::new());
        owner_file(&ledger).unwrap().sync_data().unwrap();
        sync_directory(&store).unwrap();
        assert!(matches!(
            sync_directory(&ledger),
            Err(EpisodeStoreError::Io)
        ));
        assert!(matches!(
            sync_directory(&store.join("absent")),
            Err(EpisodeStoreError::Io)
        ));
    }

    #[test]
    fn rejects_non_directory_ancestors_and_oversized_ledger() {
        let root = TestDirectory::new();
        let file = root.0.join("file");
        owner_file(&file)
            .unwrap()
            .set_len(MAX_LEDGER_BYTES + 1)
            .unwrap();
        assert!(matches!(
            prepare_directory(&file.join("store")),
            Err(EpisodeStoreError::Io)
        ));
        assert!(matches!(
            read_bounded_ledger(&file),
            Err(EpisodeStoreError::Io)
        ));
        assert!(matches!(
            reject_symlink_if_present(&root.0),
            Err(EpisodeStoreError::Io)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn ancestor_alias_is_compatible_but_direct_symlinks_are_rejected() {
        use std::os::unix::fs::symlink;

        let root = TestDirectory::new();
        let target = root.0.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.0.join("link");
        symlink(&target, &link).unwrap();
        assert!(matches!(
            prepare_directory(&link),
            Err(EpisodeStoreError::Io)
        ));
        let mut calls = Vec::new();
        let prepared = prepare_directory_with_sync(&link.join("nested/store"), |parent| {
            calls.push(parent.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(prepared, target.join("nested/store"));
        assert_eq!(
            calls,
            [root.0.clone(), target.clone(), target.join("nested")]
        );
        calls.clear();
        let reopened = prepare_directory_with_sync(&link.join("nested/store"), |parent| {
            calls.push(parent.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(reopened, prepared);
        assert_eq!(calls, [target.join("nested")]);
        assert!(matches!(sync_directory(&link), Err(EpisodeStoreError::Io)));
        let missing_target = root.0.join("missing");
        let ledger = root.0.join("ledger");
        symlink(&missing_target, &ledger).unwrap();
        assert!(matches!(owner_file(&ledger), Err(EpisodeStoreError::Io)));
        assert!(matches!(
            read_bounded_ledger(&ledger),
            Err(EpisodeStoreError::Io)
        ));
        assert!(!missing_target.exists());
    }
}
