use crate::{Result, error};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Scheduler {
    jobs: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
    leases: Mutex<BTreeSet<String>>,
    state_queue: Mutex<StateQueue>,
    changed: Condvar,
}
pub struct Job<'a> {
    owner: &'a Scheduler,
    pub id: String,
    pub cancel: Arc<AtomicBool>,
    lease: Option<String>,
    deadline: Instant,
    state_permit: bool,
}
/// Priority applies between snapshot jobs. Model and image analysis jobs use
/// independent leases and do not occupy this state-computation lane.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Foreground,
    Residual,
    Idle,
}
#[derive(Default)]
struct StateQueue {
    active: bool,
    next_ticket: u64,
    priority_streak: usize,
    waiting: BTreeMap<u64, Priority>,
}
impl StateQueue {
    fn admitted(&mut self, priority: Priority) {
        self.priority_streak = if self.priority_streak >= 8 || priority == Priority::Idle {
            0
        } else {
            self.priority_streak + 1
        };
    }
    fn selected(&self) -> Option<u64> {
        if self.active {
            return None;
        }
        // After eight higher-priority admissions offer the oldest background task
        // a turn. FIFO within priority classes preserves bounded opportunities.
        if self.priority_streak >= 8
            && let Some((&ticket, _)) = self
                .waiting
                .iter()
                .find(|(_, p)| **p != Priority::Foreground)
        {
            return Some(ticket);
        }
        self.waiting
            .iter()
            .min_by_key(|(ticket, priority)| (**priority, **ticket))
            .map(|(&ticket, _)| ticket)
    }
}
impl Scheduler {
    pub fn begin(&self, id: String, lease: Option<String>, seconds: u64) -> Result<Job<'_>> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error("Scheduler", "Job registry unavailable"))?;
        if jobs.contains_key(&id) || jobs.len() >= 8 {
            return Err(error(
                "Busy",
                "Job identifier is active or scheduler is full",
            ));
        }
        if let Some(key) = &lease
            && !self.leases.lock().unwrap().insert(key.clone())
        {
            return Err(error("Busy", "An exclusive job already holds this lease"));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(id.clone(), cancel.clone());
        Ok(Job {
            owner: self,
            id,
            cancel,
            lease,
            deadline: Instant::now() + Duration::from_secs(seconds),
            state_permit: false,
        })
    }
    pub fn begin_state(
        &self,
        id: String,
        lease: Option<String>,
        seconds: u64,
        priority: Priority,
    ) -> Result<Job<'_>> {
        let mut job = self.begin(id, lease, seconds)?;
        let mut queue = self.state_queue.lock().unwrap();
        let ticket = queue.next_ticket;
        queue.next_ticket += 1;
        queue.waiting.insert(ticket, priority);
        loop {
            if let Err(error) = job.check() {
                queue.waiting.remove(&ticket);
                self.changed.notify_all();
                return Err(error);
            }
            if queue.selected() == Some(ticket) {
                queue.waiting.remove(&ticket);
                queue.active = true;
                queue.admitted(priority);
                job.state_permit = true;
                return Ok(job);
            }
            queue = self
                .changed
                .wait_timeout(queue, Duration::from_millis(25))
                .unwrap()
                .0;
        }
    }
    pub fn active(&self) -> Vec<String> {
        self.jobs.lock().unwrap().keys().cloned().collect()
    }
    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.jobs.lock().unwrap().get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    pub fn cancel_all(&self) {
        for flag in self.jobs.lock().unwrap().values() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
impl Job<'_> {
    pub fn check(&self) -> Result<()> {
        if Instant::now() > self.deadline {
            self.cancel.store(true, Ordering::Relaxed);
            return Err(error("BudgetExceeded", "Job deadline reached"));
        }
        if self.cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Job cancelled"));
        }
        Ok(())
    }
}
impl Drop for Job<'_> {
    fn drop(&mut self) {
        if self.state_permit {
            self.owner.state_queue.lock().unwrap().active = false;
            self.owner.changed.notify_all();
        }
        self.owner.jobs.lock().unwrap().remove(&self.id);
        if let Some(key) = &self.lease {
            self.owner.leases.lock().unwrap().remove(key);
        }
    }
}
/// One checkpoint is committed only after a whole disjoint chunk completes.
/// Retaining this cursor allows explicit resume against the SAME snapshot.
/// It is ephemeral: startup does not replay or deserialize old jobs.
#[derive(Debug, Clone)]
pub struct RaftCursor {
    snapshot_id: String,
    total: usize,
    chunk: usize,
    pub next_index: usize,
    pub found: Vec<usize>,
}
static COMPARISONS: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
struct ComparisonPermit;
impl ComparisonPermit {
    fn acquire(cancel: &AtomicBool) -> Result<Self> {
        let mut count = COMPARISONS.0.lock().unwrap();
        while *count >= 1000 {
            if cancel.load(Ordering::Relaxed) {
                return Err(error("Cancelled", "Raft cancelled"));
            }
            count = COMPARISONS
                .1
                .wait_timeout(count, Duration::from_millis(5))
                .unwrap()
                .0;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Raft cancelled"));
        }
        *count += 1;
        Ok(Self)
    }
}
impl Drop for ComparisonPermit {
    fn drop(&mut self) {
        *COMPARISONS.0.lock().unwrap() -= 1;
        COMPARISONS.1.notify_one();
    }
}
impl RaftCursor {
    pub fn new(snapshot_id: impl Into<String>, total: usize, chunk: usize) -> Result<Self> {
        if chunk == 0 {
            return Err(error("BudgetExceeded", "Invalid raft chunk size"));
        }
        Ok(Self {
            snapshot_id: snapshot_id.into(),
            total,
            chunk: chunk.min(4096),
            next_index: 0,
            found: Vec::new(),
        })
    }
    pub fn advance<T: Sync, F: Fn(&T) -> bool + Sync>(
        &mut self,
        snapshot_id: &str,
        items: &[T],
        predicate: &F,
        workers: usize,
        cancel: &AtomicBool,
    ) -> Result<bool> {
        if snapshot_id != self.snapshot_id || items.len() != self.total {
            return Err(error(
                "StaleRevision",
                "Raft checkpoint belongs to another snapshot",
            ));
        }
        if workers == 0 || workers > 64 {
            return Err(error("BudgetExceeded", "Invalid raft worker count"));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Raft cancelled"));
        }
        if self.next_index == self.total {
            return Ok(true);
        }
        let end = (self.next_index + self.chunk).min(self.total);
        let part = &items[self.next_index..end];
        let size = specimen_kernel::search::raft_size(part.len(), workers);
        let start = self.next_index;
        let results = std::thread::scope(|scope| {
            let tasks = part
                .chunks(size)
                .enumerate()
                .map(|(index, p)| {
                    scope.spawn(move || {
                        let mut found = Vec::new();
                        for (i, v) in p.iter().enumerate() {
                            let _permit = ComparisonPermit::acquire(cancel)?;
                            if predicate(v) {
                                found.push(start + index * size + i);
                            }
                        }
                        Ok(found)
                    })
                })
                .collect::<Vec<_>>();
            tasks
                .into_iter()
                .map(|task| {
                    task.join()
                        .map_err(|_| error("Scheduler", "Raft worker failed"))?
                })
                .collect::<Result<Vec<_>>>()
        })?;
        if cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Raft cancelled"));
        }
        self.found.extend(results.into_iter().flatten());
        self.next_index = end;
        Ok(end == self.total)
    }
}
pub fn raft_find_all<T: Sync, F: Fn(&T) -> bool + Sync>(
    items: &[T],
    predicate: F,
    chunk: usize,
    workers: usize,
    cancel: &AtomicBool,
) -> Result<Vec<usize>> {
    raft_find_all_progress(items, predicate, chunk, workers, cancel, &|_, _| {})
}
pub fn raft_find_all_progress<T: Sync, F: Fn(&T) -> bool + Sync>(
    items: &[T],
    predicate: F,
    chunk: usize,
    workers: usize,
    cancel: &AtomicBool,
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<usize>> {
    let mut cursor = RaftCursor::new("cycle-snapshot", items.len(), chunk)?;
    loop {
        let done = cursor.advance("cycle-snapshot", items, &predicate, workers, cancel)?;
        progress(cursor.next_index, items.len());
        if done {
            return Ok(cursor.found);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn priority_and_background_opportunities() {
        let mut q = StateQueue::default();
        q.waiting.insert(0, Priority::Idle);
        q.waiting.insert(1, Priority::Residual);
        q.waiting.insert(2, Priority::Foreground);
        assert_eq!(q.selected(), Some(2));
        q.priority_streak = 8;
        assert_eq!(q.selected(), Some(0));
        q.active = true;
        assert_eq!(q.selected(), None);
        // A residual fairness turn must return priority to foreground work.
        q.admitted(Priority::Residual);
        q.active = false;
        assert_eq!(q.selected(), Some(2));
    }
    #[test]
    fn checkpoint_resume_and_cancel_are_exact() {
        let values = (0..10001).collect::<Vec<_>>();
        let cancel = AtomicBool::new(false);
        let mut cursor = RaftCursor::new("revision-7", values.len(), 257).unwrap();
        cursor
            .advance("revision-7", &values, &|v| v % 3 == 0, 8, &cancel)
            .unwrap();
        let completed = cursor.next_index;
        let found = cursor.found.clone();
        cancel.store(true, Ordering::Relaxed);
        assert!(
            cursor
                .advance("revision-7", &values, &|_| true, 8, &cancel)
                .is_err()
        );
        assert_eq!(cursor.next_index, completed);
        assert_eq!(cursor.found, found);
        cancel.store(false, Ordering::Relaxed);
        assert!(
            cursor
                .advance("revision-8", &values, &|_| true, 8, &cancel)
                .is_err()
        );
        while !cursor
            .advance("revision-7", &values, &|v| v % 3 == 0, 8, &cancel)
            .unwrap()
        {}
        assert_eq!(
            cursor.found,
            (0..10001).filter(|v| v % 3 == 0).collect::<Vec<_>>()
        );
    }
    #[test]
    fn queued_state_job_cancels_and_releases() {
        let scheduler = Scheduler::default();
        let active = scheduler
            .begin_state("active".into(), None, 5, Priority::Foreground)
            .unwrap();
        std::thread::scope(|scope| {
            let waiting = scope.spawn(|| {
                scheduler
                    .begin_state("waiting".into(), None, 5, Priority::Idle)
                    .err()
                    .unwrap()
                    .code
            });
            while !scheduler.active().contains(&"waiting".to_owned()) {
                std::thread::yield_now();
            }
            scheduler.cancel("waiting");
            assert_eq!(waiting.join().unwrap(), "Cancelled");
        });
        drop(active);
        assert!(
            scheduler
                .begin_state("next".into(), None, 1, Priority::Residual)
                .is_ok()
        );
    }
    #[test]
    fn lease_and_coverage() {
        let s = Scheduler::default();
        let job = s.begin("a".into(), Some("phagy".into()), 2).unwrap();
        assert!(s.begin("b".into(), Some("phagy".into()), 2).is_err());
        drop(job);
        assert!(s.begin("b".into(), Some("phagy".into()), 2).is_ok());
        let data = (0..10001).collect::<Vec<_>>();
        assert_eq!(
            raft_find_all(&data, |_| true, 256, 8, &AtomicBool::new(false)).unwrap(),
            (0..10001).collect::<Vec<_>>()
        );
    }
}
