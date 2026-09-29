use crate::{Result, scheduler};
use specimen_kernel::{
    host::{Env, Host},
    search::Search,
};
use std::{sync::atomic::AtomicBool, time::Instant};

/// Wall clock, v4 UUIDs and a monotonic origin for the desktop and TUI.
pub struct SystemEnv {
    origin: Instant,
}

impl Default for SystemEnv {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Env for SystemEnv {
    fn now_rfc3339(&self) -> String {
        crate::now()
    }
    fn now_millis(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
    fn uid(&self) -> String {
        crate::uid()
    }
    fn monotonic_ms(&self) -> f64 {
        self.origin.elapsed().as_secs_f64() * 1000.0
    }
}

/// Native services: system clock, threaded raft scan, wgpu layers.
#[derive(Default)]
pub struct NativeHost {
    pub env: SystemEnv,
}

impl NativeHost {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn host(&self) -> Host<'_> {
        Host {
            env: &self.env,
            search: &RaftSearch,
            accel: Some(&crate::neural::WgpuAccelerator),
        }
    }
}

/// The threaded raft scan behind the kernel's `Search` seam.
pub struct RaftSearch;

impl Search for RaftSearch {
    fn find_all(
        &self,
        len: usize,
        predicate: &(dyn Fn(usize) -> bool + Sync),
        chunk: usize,
        workers: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<usize>> {
        let items: Vec<usize> = (0..len).collect();
        scheduler::raft_find_all_progress(
            &items,
            |i| predicate(*i),
            chunk,
            workers,
            cancel,
            progress,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specimen_kernel::search::Sequential;
    use std::sync::Mutex;

    type Outcome = (Result<Vec<usize>>, Vec<(usize, usize)>);

    fn run(s: &dyn Search, len: usize, chunk: usize, workers: usize, cancel: bool) -> Outcome {
        let seen = Mutex::new(Vec::new());
        let flag = AtomicBool::new(cancel);
        let out = s.find_all(
            len,
            &|i| i % 3 == 0 || i % 7 == 2,
            chunk,
            workers,
            &flag,
            &|a, b| seen.lock().unwrap().push((a, b)),
        );
        (out, seen.into_inner().unwrap())
    }

    #[test]
    fn sequential_matches_raft() {
        for (len, chunk, workers) in [
            (0, 1, 1),
            (1, 1, 1),
            (10_000, 256, 8),
            (9_999, 5000, 3),
            (17, 4, 64),
        ] {
            let (a, pa) = run(&RaftSearch, len, chunk, workers, false);
            let (b, pb) = run(&Sequential, len, chunk, workers, false);
            assert_eq!(a.unwrap(), b.unwrap(), "len={len} chunk={chunk}");
            assert_eq!(pa, pb, "progress len={len} chunk={chunk}");
        }
    }

    #[test]
    fn sequential_matches_raft_errors() {
        for (chunk, workers, cancel) in [(0, 1, false), (8, 0, false), (8, 65, false), (8, 4, true)]
        {
            let (a, _) = run(&RaftSearch, 100, chunk, workers, cancel);
            let (b, _) = run(&Sequential, 100, chunk, workers, cancel);
            assert_eq!(
                a.unwrap_err().code,
                b.unwrap_err().code,
                "chunk={chunk} workers={workers} cancel={cancel}"
            );
        }
    }
}
