use crate::{Result, error};
use std::sync::atomic::{AtomicBool, Ordering};

/// One checkpoint chunk of a raft scan and the disjoint raft ranges it is
/// split into (half-open `[start, end)` candidate indexes).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RaftChunk {
    pub start: usize,
    pub end: usize,
    pub rafts: Vec<(usize, usize)>,
}

/// Candidates per raft when `len` candidates are split across `workers`.
pub fn raft_size(len: usize, workers: usize) -> usize {
    len.div_ceil(workers.max(1)).max(1)
}

/// The partition a raft scan of `len` candidates uses: checkpoint chunks of
/// `min(chunk, 4096)`, each split into rafts of `ceil(chunk_len / workers)`.
/// `RaftCursor::advance` in specimen-core splits with the same `raft_size`.
pub fn raft_plan(len: usize, chunk: usize, workers: usize) -> Result<Vec<RaftChunk>> {
    if chunk == 0 {
        return Err(error("BudgetExceeded", "Invalid raft chunk size"));
    }
    if workers == 0 || workers > 64 {
        return Err(error("BudgetExceeded", "Invalid raft worker count"));
    }
    let chunk = chunk.min(4096);
    let mut plan = Vec::new();
    let mut start = 0;
    while start < len {
        let end = (start + chunk).min(len);
        let size = raft_size(end - start, workers);
        let rafts = (start..end)
            .step_by(size)
            .map(|s| (s, (s + size).min(end)))
            .collect();
        plan.push(RaftChunk { start, end, rafts });
        start = end;
    }
    Ok(plan)
}

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
