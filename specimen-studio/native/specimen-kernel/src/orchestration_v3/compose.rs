use super::budget::Meter;
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Debug, Clone)]
pub struct AttachmentGroup {
    pub id: u128,
    pub issuers: Vec<u128>,
    pub members: Vec<u128>,
}
#[derive(Debug)]
pub struct PlanNode<'a> {
    pub contribution: Contribution,
    pub attachment: Option<Arc<AttachmentContext<'a>>>,
}
/// Inert completed graph only: no support retrieval, transient computation,
/// presentation/tone, aftercare, durable ATP or engine/save migration is implied.
#[derive(Debug)]
pub struct CompletePlan<'a> {
    pub cycle: u128,
    pub generation: u64,
    pub nodes: Vec<PlanNode<'a>>,
    pub groups: Vec<AttachmentGroup>,
    pub arbitration: Vec<ArbitrationTrace>,
}
fn order(contributions: &[&Contribution], m: &mut Meter) -> Result<Vec<u128>, Error> {
    m.items(contributions)?;
    for c in contributions {
        m.items(c.dependencies())?;
        m.add(64)?;
    }
    let mut index = BTreeMap::new();
    for (i, c) in contributions.iter().enumerate() {
        if index.insert(c.id(), i).is_some() {
            return Err(Error::Graph);
        }
    }
    let mut incoming = vec![0usize; contributions.len()];
    let mut edges = vec![vec![]; contributions.len()];
    for (i, c) in contributions.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for d in c.dependencies() {
            let parent = *index.get(d).ok_or(Error::Graph)?;
            if parent == i || !seen.insert(*d) {
                return Err(Error::Graph);
            }
            incoming[i] += 1;
            edges[parent].push(i)
        }
    }
    let mut ready = BTreeSet::new();
    for (i, n) in incoming.iter().enumerate() {
        if *n == 0 {
            ready.insert(i);
        }
    }
    let mut out = Vec::with_capacity(contributions.len());
    while let Some(i) = ready.pop_first() {
        out.push(contributions[i].id());
        for child in &edges[i] {
            incoming[*child] -= 1;
            if incoming[*child] == 0 {
                ready.insert(*child);
            }
        }
    }
    if out.len() != contributions.len() {
        return Err(Error::Graph);
    }
    Ok(out)
}
impl<'a> Broker<'a> {
    /// Arbitrates once, only after explicit admission/discovery closure. Graph
    /// errors precede draws and leave this cycle ready for an explicit diagnostic.
    pub fn compose(
        &mut self,
        p: &Population,
        g: u64,
        handle: &CycleHandle,
        tape: &mut DrawTape<'_>,
        scoring: ScoringBudget,
    ) -> Result<CompletePlan<'a>, Error> {
        self.check(p, g)?;
        let cycle = self.resolve_cycle(handle, g)?;
        if scoring.contributions > 100_000
            || scoring.work == 0
            || scoring.work > 1_000_000
            || scoring.payload_bytes == 0
            || scoring.payload_bytes > 16 * 1024 * 1024
            || scoring.draws > 100_000
        {
            return Err(Error::Scoring(ScoringError::InvalidBudget));
        }
        let c = self.cycles.get(&cycle).ok_or(Error::Identity)?;
        if c.status != CycleStatus::Ready || !c.sealed {
            return Err(Error::Incomplete);
        }
        let mut meter = Meter::new(self.limits.operation_work);
        meter.add(c.jobs.len().checked_mul(32).ok_or(Error::Backpressure)?)?;
        if c.jobs.len() > self.limits.trace {
            return Err(Error::Backpressure);
        }
        let mut records = c
            .jobs
            .values()
            .filter(|r| r.contribution.is_some())
            .collect::<Vec<_>>();
        records.sort_by_key(|r| r.ordinal);
        let refs = records
            .iter()
            .filter_map(|r| r.contribution.as_deref())
            .collect::<Vec<_>>();
        let ids = order(&refs, &mut meter)?;
        // Preflight all graph/context output bytes before copies. Scorer independently
        // charges its exact arbitration keys, tree copies and draw trace.
        meter.items(&refs)?;
        for r in &refs {
            meter.contribution(r)?;
        }
        self.reserve(meter.bytes.checked_mul(4).ok_or(Error::Backpressure)?)?;
        let scoring = ScoringBudget {
            work: scoring.work.min(meter.remaining()),
            payload_bytes: scoring.payload_bytes.min(
                self.limits
                    .owned_bytes
                    .saturating_sub(self.owned + meter.bytes),
            ),
            draws: scoring.draws.min(self.limits.trace),
            ..scoring
        };
        let contributions = refs.into_iter().cloned().collect::<Vec<_>>();
        let result = arbitrate(&contributions, tape, scoring)?;
        // Removing redundant leaves cannot create cycles; referenced parents are exempt
        // in the reviewed scorer. All graph validity was established before drawing.

        let mut survivors = result
            .contributions
            .into_iter()
            .map(|c| (c.id(), c))
            .collect::<BTreeMap<_, _>>();
        let mut groups: Vec<(AttachmentGroup, BTreeSet<u128>, BTreeSet<u128>)> = vec![];
        let mut group_index = BTreeMap::new();
        let mut nodes = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(contribution) = survivors.remove(&id) else {
                continue;
            };
            let attachment = c.jobs[&id].context.clone();
            if let Some(a) = &attachment {
                let index = *group_index.entry(a.group).or_insert_with(|| {
                    groups.push((
                        AttachmentGroup {
                            id: a.group,
                            issuers: vec![],
                            members: vec![],
                        },
                        BTreeSet::new(),
                        BTreeSet::new(),
                    ));
                    groups.len() - 1
                });
                let (group, members, issuers) = &mut groups[index];
                if issuers.insert(a.issuer) {
                    group.issuers.push(a.issuer);
                }
                if members.insert(a.issuer) {
                    group.members.push(a.issuer);
                }
                if members.insert(id) {
                    group.members.push(id);
                }
            }
            nodes.push(PlanNode {
                contribution,
                attachment,
            });
        }
        self.cycles.get_mut(&cycle).expect("ready cycle").status = CycleStatus::Composed;
        Ok(CompletePlan {
            cycle,
            generation: g,
            nodes,
            groups: groups.into_iter().map(|(group, _, _)| group).collect(),
            arbitration: result.groups,
        })
    }
}
