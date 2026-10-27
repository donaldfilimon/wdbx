use super::*;
use crate::{language::Expr, scanner_v3::routes::CaptureValue};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone)]
pub struct ArbitrationTrace {
    pub candidates: Vec<u128>,
    pub election: ElectionTrace,
    pub winner: Option<u128>,
}
#[derive(Debug, Clone)]
pub struct Arbitration {
    pub contributions: Vec<Contribution>,
    pub groups: Vec<ArbitrationTrace>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tree {
    Text(String),
    Call(String, Vec<Vec<Tree>>),
}
fn tree(v: &[Expr]) -> Vec<Tree> {
    v.iter()
        .map(|e| match e {
            Expr::Text { value } => Tree::Text(value.clone()),
            Expr::Call { name, args } => {
                Tree::Call(name.clone(), args.iter().map(|a| tree(a)).collect())
            }
        })
        .collect()
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Value {
    Number(crate::language_v3::CanonicalDecimal),
    Operator(String),
    Text(String),
    Bytes(Vec<u8>),
}
fn value(v: &CaptureValue) -> Value {
    match v {
        CaptureValue::Number(v) => Value::Number(v.clone()),
        CaptureValue::Operator(s) => Value::Operator(s.clone()),
        CaptureValue::Text(s) => Value::Text(s.clone()),
        CaptureValue::Bytes(v) => Value::Bytes(v.clone()),
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    input: String,
    clause: usize,
    action: usize,
    parent: String,
    tree: Vec<Tree>,
    inhibitions: Vec<InhibitionFact>,
    captures: Vec<(String, Value)>,
    dependencies: Vec<u128>,
}
fn key(c: &Contribution) -> Key {
    let mut inhibitions = c.inhibitions.clone();
    inhibitions.sort();
    let mut dependencies = c.dependencies.clone();
    dependencies.sort();
    Key {
        input: c.origin.origin.input_id().into(),
        clause: c.origin.origin.clause_id(),
        action: c.origin.origin.action_id(),
        parent: c.parent_lineage.clone(),
        tree: tree(c.action()),
        inhibitions,
        captures: c
            .origin
            .captures
            .iter()
            .map(|v| (v.name.clone(), value(&v.value)))
            .collect(),
        dependencies,
    }
}
/// Scoped exact redundancy only. Input order controls output order. References
/// must resolve within this complete collection; graph cycles are the composer's
/// later responsibility. Mixed population batches are refused explicitly.
pub fn arbitrate(
    contributions: &[Contribution],
    tape: &mut DrawTape<'_>,
    budget: ScoringBudget,
) -> Result<Arbitration, ScoringError> {
    let mut work = super::budget::Work::new(budget)?;
    if contributions.len() > budget.contributions {
        return Err(ScoringError::BudgetExceeded);
    }
    work.items(contributions)?;
    let mut ids = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    let mut entries = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    for c in contributions {
        work.charge(1 + (usize::BITS - contributions.len().leading_zeros()) as usize)?;
        if !ids.insert(c.id) {
            return Err(ScoringError::InvalidContribution);
        }
        if contributions
            .first()
            .is_some_and(|first| first.record.population != c.record.population)
        {
            return Err(ScoringError::InvalidContribution);
        }
        let before = work.remaining;
        charge_contribution(c, &mut work)?;
        let size = before - work.remaining;
        work.charge(
            size.checked_mul((usize::BITS - contributions.len().leading_zeros()) as usize)
                .ok_or(ScoringError::BudgetExceeded)?,
        )?;
        if entries
            .insert(c.record.reference.0, &c.record)
            .is_some_and(|previous| previous != &c.record)
        {
            return Err(ScoringError::InvalidContribution);
        }
        let node_binding = (&c.record.population, &c.record.key);
        if nodes
            .insert(c.record.node.0, node_binding)
            .is_some_and(|previous| previous != node_binding)
        {
            return Err(ScoringError::InvalidContribution);
        }
        referenced.extend(c.dependencies.iter().copied());
    }
    if referenced.iter().any(|id| !ids.contains(id)) {
        return Err(ScoringError::InvalidContribution);
    }
    let mut grouped: BTreeMap<Key, usize> = BTreeMap::new();
    let mut groups: Vec<Vec<usize>> = vec![];
    for (i, c) in contributions.iter().enumerate() {
        if c.attachment_group.is_some()
            || c.kind == ContributionKind::IntermediateFact
            || referenced.contains(&c.id)
        {
            continue;
        }
        work.charge(1 + (usize::BITS - contributions.len().leading_zeros()) as usize)?;
        let k = key(c);
        if let Some(group) = grouped.get(&k) {
            groups[*group].push(i)
        } else {
            let group = groups.len();
            grouped.insert(k, group);
            groups.push(vec![i]);
        }
    }
    // All groups and all bounds are known before drawing. Singleton groups are
    // complementary survivors, not elections (even if bypass confidence <=0).
    let required = groups
        .iter()
        .filter(|g| g.len() > 1)
        .filter(|g| {
            g.iter()
                .filter(|i| contributions[**i].score.effective.numerator() > 0)
                .count()
                > 1
        })
        .count();
    if required > budget.draws {
        return Err(ScoringError::BudgetExceeded);
    }
    tape.ensure(tape.position, required)?;
    let mut position = tape.position;
    let mut keep = vec![true; contributions.len()];
    let mut traces = vec![];
    for group in groups.iter().filter(|g| g.len() > 1) {
        let weights = group
            .iter()
            .map(|i| {
                let c = &contributions[*i];
                let v = c.score.effective;
                if v.numerator() <= 0 {
                    0.0
                } else {
                    (v.numerator() as f64 / v.denominator() as f64) * f64::from(c.strength)
                }
            })
            .collect::<Vec<_>>();
        let election = super::election::elect_at(
            &weights,
            &vec![true; group.len()],
            tape,
            &mut position,
            DrawPurpose::Redundancy,
        )?;
        let winner = election.winner.map(|i| contributions[group[i]].id);
        for i in group {
            keep[*i] = Some(contributions[*i].id) == winner;
        }
        traces.push(ArbitrationTrace {
            candidates: group.iter().map(|i| contributions[*i].id).collect(),
            election,
            winner,
        });
    }
    let result = Arbitration {
        contributions: contributions
            .iter()
            .zip(keep)
            .filter(|(_, keep)| *keep)
            .map(|(c, _)| c.clone())
            .collect(),
        groups: traces,
    };
    tape.position = position;
    Ok(result)
}
fn charge_contribution(c: &Contribution, w: &mut super::budget::Work) -> Result<(), ScoringError> {
    // Conservative repeated charges reserve key material, output copies and
    // trace metadata before building any key or cloning an input contribution.
    for _ in 0..3 {
        w.text(&c.source_original)?;
        w.text(&c.record.original)?;
        w.ast(&c.record.ast)?;
        w.origin(&c.origin.origin)?;
        w.origin(&c.target_origin)?;
        w.path(&c.origin.path)?;
        w.captures(&c.origin.captures)?;
        w.label(&c.parent_lineage)?;
        w.items(&c.dependencies)?;
        w.items(&c.inhibitions)?;
        for f in &c.inhibitions {
            w.label(&f.code)?
        }
        w.label(&c.record.population.tenant)?;
        w.label(&c.record.population.specimen)?;
        w.label(&c.record.population.node_type)?;
        w.key(&c.record.key)?;
        if let Some(v) = &c.record.resource {
            w.label(&v.0)?
        }
        w.items(&c.origin.token_spans)?;
        w.items(&c.election.weights)?;
        w.items(&c.election.refused)?;
        w.items(&c.score.draws)?;
        w.label(&c.score.random.stream)?;
        w.label(&c.election.random.stream)?;
        for d in c.score.draws.iter().chain(c.election.draw.iter()) {
            w.label(&d.stream)?
        }
        for component in &c.evidence.components {
            w.items(&component.left_spans)?;
            w.items(&component.right_spans)?
        }
    }
    Ok(())
}
