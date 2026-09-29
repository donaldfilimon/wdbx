use crate::{Result, error};
use std::sync::atomic::{AtomicBool, Ordering};

/// Finds every index in `0..len` whose predicate holds, in index order.
/// Natively this is the threaded raft scan; the kernel ships a sequential one.
pub trait Search: Sync {
    fn find_all(
        &self,
        len: usize,
        predicate: &(dyn Fn(usize) -> bool + Sync),
        chunk: usize,
        workers: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<usize>>;
}

/// Single-threaded search with the raft's chunking, validation and progress.
pub struct Sequential;

impl Search for Sequential {
    fn find_all(
        &self,
        len: usize,
        predicate: &(dyn Fn(usize) -> bool + Sync),
        chunk: usize,
        workers: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<usize>> {
        if chunk == 0 {
            return Err(error("BudgetExceeded", "Invalid raft chunk size"));
        }
        let chunk = chunk.min(4096);
        let mut next = 0;
        let mut found = Vec::new();
        loop {
            if workers == 0 || workers > 64 {
                return Err(error("BudgetExceeded", "Invalid raft worker count"));
            }
            if cancel.load(Ordering::Relaxed) {
                return Err(error("Cancelled", "Raft cancelled"));
            }
            if next == len {
                progress(next, len);
                return Ok(found);
            }
            let end = (next + chunk).min(len);
            found.extend((next..end).filter(|&i| predicate(i)));
            next = end;
            if cancel.load(Ordering::Relaxed) {
                return Err(error("Cancelled", "Raft cancelled"));
            }
            progress(next, len);
            if next == len {
                return Ok(found);
            }
        }
    }
}
