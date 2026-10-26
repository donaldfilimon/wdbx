use super::budget::{self, CYCLE_BYTES, CYCLE_CLEANUP, JOB_BYTES, JOB_CLEANUP, Meter};
use super::*;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
#[derive(Debug, Clone)]
pub struct Lease {
    pub(super) owner: Arc<()>,
    pub(super) serial: u64,
    pub(super) generation: u64,
    pub(super) cycle: u128,
    pub(super) job: u128,
}
impl Lease {
    pub fn cycle(&self) -> u128 {
        self.cycle
    }
    pub fn job(&self) -> u128 {
        self.job
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
/// Private cycle incarnation, required even for trusted host control calls.
#[derive(Debug, Clone)]
pub struct CycleHandle {
    pub(super) owner: Arc<()>,
    pub(super) id: u128,
    pub(super) generation: u64,
    pub(super) incarnation: u64,
}
impl CycleHandle {
    pub fn id(&self) -> u128 {
        self.id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
#[derive(Debug, Clone)]
pub struct CompletedIssuer {
    pub(super) incarnation: u64,
    pub(super) owner: Arc<()>,
    pub(super) cycle: u128,
    pub(super) job: u128,
    pub(super) generation: u64,
}
impl CompletedIssuer {
    pub fn cycle_handle(&self) -> CycleHandle {
        CycleHandle {
            owner: self.owner.clone(),
            id: self.cycle,
            generation: self.generation,
            incarnation: self.incarnation,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Pending,
    Active(u64, u64),
    Revoked(u64, u64),
    Done,
    Cancelled,
}
pub(super) struct Record<'a> {
    pub comparison: &'a Comparison<'a>,
    pub state: State,
    pub contribution: Option<Box<Contribution>>,
    pub context: Option<Arc<AttachmentContext<'a>>>,
    pub depth: u8,
    pub ordinal: usize,
}
pub(super) struct Cycle<'a> {
    pub incarnation: u64,
    pub creation_generation: u64,
    pub jobs: BTreeMap<u128, Record<'a>>,
    pub limit: CycleLimit,
    pub discovery_closed: bool,
    pub sealed: bool,
    pub status: CycleStatus,
    pub visits: Vec<Visit<'a>>,
    pub extra_bytes: usize,
    pub last_now: Option<u64>,
}
/// Shared deterministic permit model, not a thread pool. Lease identity includes
/// a private allocation identity, so a second same-scope broker cannot complete it.
pub struct Broker<'a> {
    pub(super) population: Population,
    pub(super) generation: u64,
    pub(super) limits: Limits,
    pub(super) owner: Arc<()>,
    pub(super) cycles: BTreeMap<u128, Cycle<'a>>,
    foreground: VecDeque<(u128, u128)>,
    maintenance: VecDeque<(u128, u128)>,
    active: BTreeMap<u64, (u128, u128)>,
    serial: u64,
    cycle_serial: u64,
    pub(super) cleanup_reserved: usize,
    foreground_quanta: u8,
    pub(super) owned: usize,
    pub(super) pending: usize,
    pub(super) quarantined: usize,
}
impl<'a> Broker<'a> {
    pub fn new(population: Population, generation: u64, limits: Limits) -> Result<Self, Error> {
        budget::scope(&population)?;
        if generation == 0
            || limits.active == 0
            || limits.active > 1000
            || limits.pending > 100_000
            || limits.owned_bytes == 0
            || limits.owned_bytes > 16 * 1024 * 1024
            || limits.operation_work == 0
            || limits.operation_work > 1_000_000
            || limits.trace > 100_000
        {
            return Err(Error::Invalid);
        }
        let owned =
            population.tenant.len() + population.specimen.len() + population.node_type.len() + 256;
        if owned > limits.owned_bytes {
            return Err(Error::Backpressure);
        }
        Ok(Self {
            population,
            generation,
            limits,
            owner: Arc::new(()),
            cycles: BTreeMap::new(),
            foreground: VecDeque::new(),
            maintenance: VecDeque::new(),
            active: BTreeMap::new(),
            serial: 0,
            cycle_serial: 0,
            cleanup_reserved: 0,
            foreground_quanta: 0,
            owned,
            pending: 0,
            quarantined: 0,
        })
    }
    pub fn counts(&self) -> Counts {
        Counts {
            pending: self.pending,
            active: self.active.len(),
            quarantined: self.quarantined,
            owned_bytes: self.owned,
        }
    }
    pub(super) fn check(&self, p: &Population, g: u64) -> Result<(), Error> {
        budget::scope(p)?;
        if p != &self.population {
            return Err(Error::Scope);
        }
        if g != self.generation {
            return Err(Error::StaleGeneration);
        }
        Ok(())
    }
    pub fn begin(
        &mut self,
        p: &Population,
        g: u64,
        cycle: u128,
        limit: CycleLimit,
    ) -> Result<CycleHandle, Error> {
        self.check(p, g)?;
        if cycle == 0 || self.cycles.contains_key(&cycle) || limit.jobs > 100_000 {
            return Err(Error::Identity);
        }
        let incarnation = self
            .cycle_serial
            .checked_add(1)
            .ok_or(Error::Backpressure)?;
        self.reserve(CYCLE_BYTES)?;
        self.reserve_cleanup(CYCLE_CLEANUP)?;
        self.cycle_serial = incarnation;
        self.cycles.insert(
            cycle,
            Cycle {
                incarnation,
                creation_generation: g,
                jobs: BTreeMap::new(),
                limit,
                discovery_closed: false,
                sealed: false,
                status: CycleStatus::Open,
                visits: vec![],
                extra_bytes: 0,
                last_now: None,
            },
        );
        self.owned += CYCLE_BYTES;
        self.cleanup_reserved += CYCLE_CLEANUP;
        Ok(CycleHandle {
            owner: self.owner.clone(),
            id: cycle,
            generation: g,
            incarnation,
        })
    }
    pub(super) fn resolve_cycle(&self, handle: &CycleHandle, g: u64) -> Result<u128, Error> {
        if !Arc::ptr_eq(&self.owner, &handle.owner) {
            return Err(Error::Identity);
        }
        if self.generation != g {
            return Err(Error::StaleGeneration);
        }
        let cycle = self.cycles.get(&handle.id).ok_or(Error::Identity)?;
        if cycle.incarnation != handle.incarnation || cycle.creation_generation != handle.generation
        {
            return Err(Error::Identity);
        }
        Ok(handle.id)
    }
    pub(super) fn reserve(&self, bytes: usize) -> Result<(), Error> {
        if self.owned.checked_add(bytes).ok_or(Error::Backpressure)? > self.limits.owned_bytes {
            Err(Error::Backpressure)
        } else {
            Ok(())
        }
    }
    pub(super) fn reserve_cleanup(&self, additional: usize) -> Result<(), Error> {
        if self
            .cleanup_reserved
            .checked_add(additional)
            .ok_or(Error::Backpressure)?
            > self.limits.operation_work
        {
            return Err(Error::Backpressure);
        }
        Ok(())
    }
    pub(super) fn reserve_jobs(&self, cycle: u128, jobs: &[Job<'a>]) -> Result<(), Error> {
        let c = self.cycles.get(&cycle).ok_or(Error::Identity)?;
        if jobs.len() > self.limits.pending.saturating_sub(self.pending)
            || jobs.len() > c.limit.jobs.saturating_sub(c.jobs.len())
        {
            return Err(Error::Backpressure);
        }
        self.reserve(
            jobs.len()
                .checked_mul(JOB_BYTES)
                .ok_or(Error::Backpressure)?,
        )?;
        self.reserve_cleanup(
            jobs.len()
                .checked_mul(JOB_CLEANUP)
                .ok_or(Error::Backpressure)?,
        )
    }
    pub(super) fn validate_descriptors(
        &self,
        cycle: u128,
        jobs: &[Job<'a>],
        m: &mut Meter,
    ) -> Result<(), Error> {
        let c = self.cycles.get(&cycle).ok_or(Error::Identity)?;
        if c.status != CycleStatus::Open || c.sealed || c.discovery_closed {
            return Err(Error::Closed);
        }
        // All public metadata is bounded before binding/equality/index traversal.
        m.add(
            jobs.len()
                .checked_mul(JOB_BYTES)
                .ok_or(Error::Backpressure)?,
        )?;
        for j in jobs {
            m.comparison(j.comparison)?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for j in jobs {
            m.add(64)?;
            m.comparison_binding(j.comparison)?;
            m.population(&j.comparison.record.population)?;
            m.population(&self.population)?;
            if j.comparison.record.population != self.population {
                return Err(Error::Scope);
            }
            if j.id == 0 || !ids.insert(j.id) {
                return Err(Error::Identity);
            }
        }
        Ok(())
    }
    pub(super) fn ensure_new_jobs(&self, cycle: u128, jobs: &[Job<'a>]) -> Result<(), Error> {
        let c = self.cycles.get(&cycle).ok_or(Error::Identity)?;
        if jobs.iter().any(|j| c.jobs.contains_key(&j.id)) {
            return Err(Error::Identity);
        }
        Ok(())
    }
    pub fn admit(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
        jobs: &[Job<'a>],
    ) -> Result<(), Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        let mut meter = Meter::new(self.limits.operation_work);
        self.validate_descriptors(cycle, jobs, &mut meter)?;
        self.ensure_new_jobs(cycle, jobs)?;
        self.reserve_jobs(cycle, jobs)?;
        if jobs.iter().any(|j| j.comparison.attachment_group.is_some()) {
            return Err(Error::Binding);
        }
        self.insert(cycle, jobs, None, 0);
        Ok(())
    }
    pub(super) fn insert(
        &mut self,
        cycle: u128,
        jobs: &[Job<'a>],
        context: Option<Arc<AttachmentContext<'a>>>,
        depth: u8,
    ) {
        let c = self.cycles.get_mut(&cycle).expect("preflight cycle");
        for j in jobs {
            c.jobs.insert(
                j.id,
                Record {
                    comparison: j.comparison,
                    state: State::Pending,
                    contribution: None,
                    context: context.clone(),
                    depth,
                    ordinal: c.jobs.len(),
                },
            );
            match j.priority {
                Priority::Foreground => self.foreground.push_back((cycle, j.id)),
                Priority::Maintenance => self.maintenance.push_back((cycle, j.id)),
            }
        }
        self.pending += jobs.len();
        self.owned += JOB_BYTES * jobs.len();
        self.cleanup_reserved += JOB_CLEANUP * jobs.len();
    }
    pub fn dispatch(
        &mut self,
        p: &Population,
        g: u64,
        maximum: usize,
    ) -> Result<Vec<Lease>, Error> {
        self.check(p, g)?;
        if maximum > self.limits.trace || maximum > self.limits.operation_work {
            return Err(Error::Backpressure);
        }
        let n = maximum
            .min(self.limits.active - self.active.len())
            .min(self.pending);
        let end = self
            .serial
            .checked_add(n as u64)
            .ok_or(Error::Backpressure)?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let use_fg = !self.foreground.is_empty()
                && (self.maintenance.is_empty() || self.foreground_quanta < 8);
            let (cycle, job) = if use_fg {
                if !self.maintenance.is_empty() {
                    self.foreground_quanta += 1
                } else {
                    self.foreground_quanta = 0
                }
                self.foreground.pop_front()
            } else {
                self.foreground_quanta = 0;
                self.maintenance.pop_front()
            }
            .expect("pending queue");
            self.serial += 1;
            let record = self
                .cycles
                .get_mut(&cycle)
                .expect("admitted cycle")
                .jobs
                .get_mut(&job)
                .expect("admitted job");
            record.state = State::Active(self.serial, g);
            self.active.insert(self.serial, (cycle, job));
            out.push(Lease {
                owner: self.owner.clone(),
                serial: self.serial,
                generation: g,
                cycle,
                job,
            });
        }
        debug_assert_eq!(self.serial, end);
        self.pending -= n;
        Ok(out)
    }
    fn lease_record(&self, l: &Lease) -> Result<&Record<'a>, Error> {
        if !Arc::ptr_eq(&self.owner, &l.owner)
            || self.active.get(&l.serial) != Some(&(l.cycle, l.job))
        {
            return Err(Error::InvalidLease);
        }
        let r = &self.cycles.get(&l.cycle).ok_or(Error::InvalidLease)?.jobs[&l.job];
        if !matches!(r.state,State::Active(s,g)|State::Revoked(s,g) if s==l.serial&&g==l.generation)
        {
            return Err(Error::InvalidLease);
        }
        Ok(r)
    }
    pub fn complete(
        &mut self,
        p: &Population,
        g: u64,
        l: &Lease,
        contribution: Option<Contribution>,
    ) -> Result<Option<CompletedIssuer>, Error> {
        self.check(p, g)?;
        let r = self.lease_record(l)?;
        if matches!(r.state, State::Revoked(..)) {
            return Err(Error::Revoked);
        }
        if l.generation != g {
            return Err(Error::StaleGeneration);
        }
        let mut m = Meter::new(self.limits.operation_work);
        if let Some(c) = &contribution {
            budget::binding(c, l.job, r.comparison)?;
            m.contribution(c)?;
        }
        self.reserve(m.bytes)?;
        self.reserve_cleanup(m.bytes)?;
        let incarnation = self.cycles[&l.cycle].incarnation;
        let proof = contribution.as_ref().map(|_| CompletedIssuer {
            incarnation,
            owner: self.owner.clone(),
            cycle: l.cycle,
            job: l.job,
            generation: g,
        });
        let r = self
            .cycles
            .get_mut(&l.cycle)
            .expect("lease cycle")
            .jobs
            .get_mut(&l.job)
            .expect("lease job");
        r.state = State::Done;
        r.contribution = contribution.map(Box::new);
        self.cycles
            .get_mut(&l.cycle)
            .expect("lease cycle")
            .extra_bytes += m.bytes;
        self.owned += m.bytes;
        self.cleanup_reserved += m.bytes;
        self.active.remove(&l.serial);
        Ok(proof)
    }
    pub fn close_discovery(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
    ) -> Result<(), Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        let c = self.cycles.get_mut(&cycle).ok_or(Error::Identity)?;
        if c.status != CycleStatus::Open || c.sealed {
            return Err(Error::Closed);
        }
        if c.jobs.values().any(|r| {
            matches!(
                r.state,
                State::Pending | State::Active(..) | State::Revoked(..)
            )
        }) {
            return Err(Error::Incomplete);
        }
        c.discovery_closed = true;
        Ok(())
    }
    pub fn seal(&mut self, p: &Population, g: u64, handle: &CycleHandle) -> Result<(), Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        let c = self.cycles.get_mut(&cycle).ok_or(Error::Identity)?;
        if c.status != CycleStatus::Open || c.sealed {
            return Err(Error::Closed);
        }
        if !c.discovery_closed || c.jobs.values().any(|r| r.state != State::Done) {
            return Err(Error::Incomplete);
        }
        c.sealed = true;
        c.status = CycleStatus::Ready;
        Ok(())
    }
    pub fn status(
        &self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
    ) -> Result<CycleStatus, Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        Ok(self.cycles.get(&cycle).ok_or(Error::Identity)?.status)
    }
    pub fn cancel(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
        reason: CancelReason,
    ) -> Result<(), Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        self.cancel_inner(cycle, reason)
    }
    fn cancel_inner(&mut self, cycle: u128, reason: CancelReason) -> Result<(), Error> {
        let c = self.cycles.get_mut(&cycle).ok_or(Error::Identity)?;
        if matches!(c.status, CycleStatus::Composed) {
            return Err(Error::Closed);
        }
        if matches!(c.status, CycleStatus::Partial(_)) {
            return Ok(());
        }
        let (pending, quarantined, extra) = Self::cancel_records(c, reason);
        self.pending -= pending;
        self.quarantined += quarantined;
        self.foreground.retain(|(id, _)| *id != cycle);
        self.maintenance.retain(|(id, _)| *id != cycle);
        self.owned -= extra;
        self.cleanup_reserved -= extra;
        Ok(())
    }
    fn cancel_records(c: &mut Cycle<'a>, reason: CancelReason) -> (usize, usize, usize) {
        let (mut pending, mut quarantined) = (0, 0);
        for r in c.jobs.values_mut() {
            match r.state {
                State::Pending => {
                    r.state = State::Cancelled;
                    pending += 1
                }
                State::Active(s, g) => {
                    r.state = State::Revoked(s, g);
                    quarantined += 1
                }
                _ => {}
            }
            r.contribution = None;
            r.context = None;
        }
        c.visits = Vec::new();
        let extra = c.extra_bytes;
        c.extra_bytes = 0;
        c.status = CycleStatus::Partial(reason);
        c.discovery_closed = true;
        (pending, quarantined, extra)
    }
    /// Explicit host monotonic instant. Regressing supplied instants refuse
    /// without mutation; no clock is read inside this kernel.
    pub fn poll_deadline(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
        now: u64,
    ) -> Result<CycleStatus, Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        let c = self.cycles.get_mut(&cycle).ok_or(Error::Identity)?;
        if c.last_now.is_some_and(|previous| now < previous) {
            return Err(Error::Invalid);
        }
        c.last_now = Some(now);
        if c.limit.deadline.is_some_and(|d| now >= d)
            && matches!(c.status, CycleStatus::Open | CycleStatus::Ready)
        {
            self.cancel_inner(cycle, CancelReason::Deadline)?
        }
        self.status(p, g, handle)
    }
    /// Restart never frees active workers; their OLD generation acknowledgements
    /// remain required. This protocol does not establish that the host fenced them.
    pub fn restart(&mut self, p: &Population, g: u64, next: u64) -> Result<(), Error> {
        self.check(p, g)?;
        if next <= g {
            return Err(Error::StaleGeneration);
        }
        // Acceptance reserves this complete structural/drop bound, even for
        // lowered caller limits. No fallible budget operation follows mutation.
        debug_assert!(self.cleanup_reserved <= self.limits.operation_work);
        for c in self.cycles.values_mut() {
            if !matches!(c.status, CycleStatus::Composed | CycleStatus::Partial(_)) {
                let (pending, quarantined, extra) = Self::cancel_records(c, CancelReason::Restart);
                self.pending -= pending;
                self.quarantined += quarantined;
                self.owned -= extra;
                self.cleanup_reserved -= extra;
            }
        }
        // Exactly one global queue clear, never one retain per cycle.
        self.foreground.clear();
        self.maintenance.clear();
        self.foreground_quanta = 0;
        self.generation = next;
        Ok(())
    }
    /// Call only after trusted host fencing/observed termination. The supplied
    /// worker generation binds the revoked worker; the separate current owner
    /// generation refuses delayed acknowledgements from before broker restart.
    pub fn acknowledge_fence(
        &mut self,
        p: &Population,
        current_generation: u64,
        worker_generation: u64,
        l: &Lease,
    ) -> Result<(), Error> {
        self.check(p, current_generation)?;
        if worker_generation != l.generation {
            return Err(Error::StaleGeneration);
        }
        let r = self.lease_record(l)?;
        if !matches!(r.state, State::Revoked(..)) {
            return Err(Error::InvalidLease);
        }
        self.cycles
            .get_mut(&l.cycle)
            .expect("lease cycle")
            .jobs
            .get_mut(&l.job)
            .expect("lease job")
            .state = State::Cancelled;
        self.active.remove(&l.serial);
        self.quarantined -= 1;
        Ok(())
    }
    /// Drop only this completed/cancelled cycle's retained metadata. Events and
    /// returned plans are host-owned. A surviving revoked worker blocks retirement.
    pub fn retire_cycle(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
    ) -> Result<(), Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        let c = self.cycles.get(&cycle).ok_or(Error::Identity)?;
        if !matches!(c.status, CycleStatus::Partial(_) | CycleStatus::Composed) {
            return Err(Error::Incomplete);
        }
        if c.jobs
            .values()
            .any(|r| matches!(r.state, State::Revoked(..)))
        {
            return Err(Error::Revoked);
        }
        if c.jobs
            .values()
            .any(|r| matches!(r.state, State::Pending | State::Active(..)))
        {
            return Err(Error::Incomplete);
        }
        let bytes = CYCLE_BYTES + c.jobs.len() * JOB_BYTES + c.extra_bytes;
        let cleanup = CYCLE_CLEANUP + c.jobs.len() * JOB_CLEANUP + c.extra_bytes;
        self.cycles.remove(&cycle);
        self.owned = self.owned.checked_sub(bytes).expect("cycle accounting");
        self.cleanup_reserved = self
            .cleanup_reserved
            .checked_sub(cleanup)
            .expect("cleanup accounting");
        Ok(())
    }
}
