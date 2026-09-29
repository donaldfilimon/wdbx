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

/// Largest checkpoint chunk a raft scan takes; larger requests are capped.
pub const MAX_RAFT_CHUNK: usize = 4096;
/// Largest candidate count `raft_plan` will describe.
pub const MAX_PLAN_CANDIDATES: usize = 10_000_000;

/// Candidates per raft when `len` candidates are split across `workers`.
pub fn raft_size(len: usize, workers: usize) -> usize {
    len.div_ceil(workers.max(1)).max(1)
}

/// A raft scan's partition: totals plus the first chunks in full.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RaftPlan {
    /// Effective checkpoint chunk size (the request capped at `MAX_RAFT_CHUNK`).
    pub chunk_size: usize,
    /// Total checkpoint chunks the scan takes.
    pub checkpoints: usize,
    /// Most rafts any chunk is split into.
    pub max_rafts: usize,
    /// The first `limit` chunks with their raft ranges.
    pub chunks: Vec<RaftChunk>,
}

fn chunk_at(start: usize, len: usize, chunk: usize, workers: usize) -> RaftChunk {
    let end = (start + chunk).min(len);
    let size = raft_size(end - start, workers);
    let rafts = (start..end)
        .step_by(size)
        .map(|s| (s, (s + size).min(end)))
        .collect();
    RaftChunk { start, end, rafts }
}

/// The partition a raft scan of `len` candidates uses: checkpoint chunks of
/// `min(chunk, MAX_RAFT_CHUNK)`, each split into rafts of
/// `ceil(chunk_len / workers)`. Only the first `limit` chunks are built.
/// `RaftCursor::advance` in specimen-core splits with the same `raft_size`.
pub fn raft_plan(len: usize, chunk: usize, workers: usize, limit: usize) -> Result<RaftPlan> {
    if chunk == 0 {
        return Err(error("BudgetExceeded", "Invalid raft chunk size"));
    }
    if workers == 0 || workers > 64 {
        return Err(error("BudgetExceeded", "Invalid raft worker count"));
    }
    if len > MAX_PLAN_CANDIDATES {
        return Err(error(
            "BudgetExceeded",
            format!("A raft plan covers at most {MAX_PLAN_CANDIDATES} candidates"),
        ));
    }
    let chunk = chunk.min(MAX_RAFT_CHUNK);
    let chunks = (0..len)
        .step_by(chunk)
        .take(limit)
        .map(|start| chunk_at(start, len, chunk, workers))
        .collect();
    Ok(RaftPlan {
        chunk_size: chunk,
        checkpoints: len.div_ceil(chunk),
        // The first chunk is the largest, so it has the most rafts.
        max_rafts: if len == 0 {
            0
        } else {
            chunk_at(0, len, chunk, workers).rafts.len()
        },
        chunks,
    })
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
        let chunk = chunk.min(MAX_RAFT_CHUNK);
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
