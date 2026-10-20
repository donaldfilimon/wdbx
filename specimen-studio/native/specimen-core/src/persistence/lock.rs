//! Exclusive studio writer lock.
//!
//! `flock` is per process on macOS, so a second open in the same process would
//! succeed. The in-process set closes that gap. A different process still
//! blocks on the lock file, which is what the desktop and the TUI share.
use crate::{Result, error};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static HELD: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

pub struct WriterLock {
    key: PathBuf,
    _file: File,
}

impl WriterLock {
    pub fn acquire(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        let key = root.canonicalize()?;
        let mut held = HELD
            .lock()
            .map_err(|_| error("Storage", "Store lock poisoned"))?;
        if held.contains(&key) {
            return Err(busy());
        }
        let path = key.join("writer.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock() {
            Ok(()) => {
                held.insert(key.clone());
                Ok(Self { key, _file: file })
            }
            Err(TryLockError::WouldBlock) => Err(busy()),
            Err(TryLockError::Error(err)) => Err(error("Storage", err)),
        }
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        if let Ok(mut held) = HELD.lock() {
            held.remove(&self.key);
        }
    }
}

fn busy() -> crate::Error {
    error("WriterBusy", "The desktop app holds this store.")
}
