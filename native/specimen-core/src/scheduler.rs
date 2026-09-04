use crate::{Result, error};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Scheduler {
    jobs: Mutex<BTreeMap<String, Arc<AtomicBool>>>,
    leases: Mutex<BTreeSet<String>>,
}
pub struct Job<'a> {
    owner: &'a Scheduler,
    pub id: String,
    pub cancel: Arc<AtomicBool>,
    lease: Option<String>,
    deadline: Instant,
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
        })
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
        self.owner.jobs.lock().unwrap().remove(&self.id);
        if let Some(key) = &self.lease {
            self.owner.leases.lock().unwrap().remove(key);
        }
    }
}
pub fn raft_find_all<T: Sync, F: Fn(&T) -> bool + Sync>(
    items: &[T],
    predicate: F,
    chunk: usize,
    workers: usize,
    cancel: &AtomicBool,
) -> Result<Vec<usize>> {
    if chunk == 0 || workers == 0 || workers > 64 {
        return Err(error("BudgetExceeded", "Invalid raft limits"));
    }
    let mut found = Vec::new();
    for (start, part) in items.chunks(chunk.min(4096)).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Raft cancelled"));
        }
        let size = part.len().div_ceil(workers);
        let results = std::thread::scope(|scope| {
            let tasks = part
                .chunks(size.max(1))
                .enumerate()
                .map(|(index, p)| {
                    let predicate = &predicate;
                    scope.spawn(move || {
                        p.iter()
                            .enumerate()
                            .filter_map(|(i, v)| {
                                if !cancel.load(Ordering::Relaxed) && predicate(v) {
                                    Some(start * chunk.min(4096) + index * size + i)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();
            tasks
                .into_iter()
                .map(|task| {
                    task.join()
                        .map_err(|_| error("Scheduler", "Raft worker failed"))
                })
                .collect::<Result<Vec<_>>>()
        })?;
        found.extend(results.into_iter().flatten());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(error("Cancelled", "Raft cancelled"));
    }
    Ok(found)
}
#[cfg(test)]
mod tests {
    use super::*;
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
