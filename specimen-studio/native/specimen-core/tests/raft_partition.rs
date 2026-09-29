//! Pins the kernel's `raft_plan` (what the specification diagram shows) to the
//! partition the desktop's threaded raft scan actually runs.
use specimen_core::scheduler::raft_find_all_progress;
use specimen_kernel::search::raft_plan;
use std::{collections::BTreeMap, sync::Mutex, sync::atomic::AtomicBool, thread};

/// Runs the threaded scan and returns, per checkpoint, the index ranges each
/// worker thread visited.
fn observed(len: usize, chunk: usize, workers: usize) -> Vec<Vec<(usize, usize)>> {
    let items: Vec<usize> = (0..len).collect();
    let seen = Mutex::new(Vec::new());
    let checkpoints = Mutex::new(vec![0]);
    raft_find_all_progress(
        &items,
        |i| {
            seen.lock().unwrap().push((thread::current().id(), *i));
            false
        },
        chunk,
        workers,
        &AtomicBool::new(false),
        &|covered, _| checkpoints.lock().unwrap().push(covered),
    )
    .unwrap();
    let mut by_thread: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (id, i) in seen.into_inner().unwrap() {
        let e = by_thread.entry(format!("{id:?}")).or_insert((i, i));
        e.0 = e.0.min(i);
        e.1 = e.1.max(i + 1);
    }
    let bounds = checkpoints.into_inner().unwrap();
    bounds
        .windows(2)
        .filter(|w| w[1] > w[0])
        .map(|w| {
            let mut rafts: Vec<_> = by_thread
                .values()
                .copied()
                .filter(|&(s, _)| s >= w[0] && s < w[1])
                .collect();
            rafts.sort();
            rafts
        })
        .collect()
}

#[test]
fn threaded_scan_uses_the_planned_raft_ranges() {
    for (len, chunk, workers) in [(10, 4, 3), (1000, 256, 8), (17, 5, 64), (9_999, 5000, 3)] {
        let plan = raft_plan(len, chunk, workers, usize::MAX).unwrap();
        let expected: Vec<_> = plan.chunks.iter().map(|c| c.rafts.clone()).collect();
        assert_eq!(
            observed(len, chunk, workers),
            expected,
            "len={len} chunk={chunk} workers={workers}"
        );
    }
}
