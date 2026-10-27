use super::*;
use crate::{
    identity::*,
    language::Expr,
    language_v3::*,
    scanner_v3::{Evidence, routes::*},
};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InhibitionDomain {
    Input,
    Target,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InhibitionFact {
    pub domain: InhibitionDomain,
    pub code: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionKind {
    Action,
    IntermediateFact,
}
/// Explicit host record/origin association. Not an authenticated permit.
#[derive(Debug, Clone)]
pub struct TargetBinding {
    pub node: NodeRef,
    pub entry: EntryRef,
    pub origin: Origin,
}
pub struct EntryRequest<'a> {
    pub id: u128,
    pub source: &'a PreparedAction,
    pub target: &'a PreparedAction,
    pub record: &'a EntryRecord,
    pub origin: &'a CandidateOrigin,
    pub target_binding: &'a TargetBinding,
    pub parent_lineage: &'a str,
    pub inhibitions: &'a [InhibitionFact],
    pub dependencies: &'a [u128],
    pub attachment_group: Option<u128>,
    pub kind: ContributionKind,
    pub policy: Policy,
}
#[derive(Debug, Clone)]
pub struct Contribution {
    pub(super) id: u128,
    pub(super) record: EntryRecord,
    pub(super) origin: CandidateOrigin,
    pub(super) source_original: String,
    pub(super) target_origin: Origin,
    pub(super) parent_lineage: String,
    pub(super) inhibitions: Vec<InhibitionFact>,
    pub(super) dependencies: Vec<u128>,
    pub(super) attachment_group: Option<u128>,
    pub(super) kind: ContributionKind,
    pub(super) slot: usize,
    pub(super) score: ScoreTrace,
    pub(super) election: ElectionTrace,
    pub(super) evidence: Evidence,
    pub(super) strength: u8,
}
impl Contribution {
    pub fn id(&self) -> u128 {
        self.id
    }
    pub fn record(&self) -> &EntryRecord {
        &self.record
    }
    pub fn origin(&self) -> &CandidateOrigin {
        &self.origin
    }
    pub fn source_original(&self) -> &str {
        &self.source_original
    }
    pub fn target_origin(&self) -> &Origin {
        &self.target_origin
    }
    pub fn parent_lineage(&self) -> &str {
        &self.parent_lineage
    }
    pub fn inhibitions(&self) -> &[InhibitionFact] {
        &self.inhibitions
    }
    pub fn dependencies(&self) -> &[u128] {
        &self.dependencies
    }
    pub fn attachment_group(&self) -> Option<u128> {
        self.attachment_group
    }
    pub fn kind(&self) -> ContributionKind {
        self.kind
    }
    pub fn slot(&self) -> usize {
        self.slot
    }
    pub fn score(&self) -> &ScoreTrace {
        &self.score
    }
    pub fn election(&self) -> &ElectionTrace {
        &self.election
    }
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
    pub fn strength(&self) -> u8 {
        self.strength
    }
    /// Inert legacy expression tree only. This API does not compile v3 scripts.
    pub fn action(&self) -> &[Expr] {
        &self.record.ast.alternatives[self.slot].action
    }
}
#[derive(Debug, Clone)]
pub struct EntryOutcome {
    pub score: ScoreTrace,
    pub election: Option<ElectionTrace>,
    pub contribution: Option<Contribution>,
}
pub fn evaluate_entry(
    request: EntryRequest<'_>,
    tape: &mut DrawTape<'_>,
    budget: ScoringBudget,
) -> Result<EntryOutcome, ScoringError> {
    let mut work = super::budget::Work::new(budget)?;
    if budget.contributions == 0 {
        return Err(ScoringError::BudgetExceeded);
    }
    preflight(&request, &mut work)?;
    let evidence = crate::scanner_v3::scan(request.source, request.target)?;
    validate_evidence(&evidence)?;
    let mut position = tape.position;
    let mut score = super::policy::score_at(
        evidence.totals.confidence,
        request.policy,
        tape,
        &mut position,
    )?;
    // A declared route skips just the numeric gate; guards and strength remain.
    if request.origin.bypass == SimilarityBypass::Declared {
        score.refusals.retain(|r| *r != Refusal::BelowThreshold)
    }
    add_guard_refusals(&request.origin.constraints, true, &mut score.refusals);
    add_guard_refusals(&evidence.right_constraints, false, &mut score.refusals);
    if matches!(
        request.origin.path,
        CandidatePath::Automaton {
            status: AutomatonStatus::Blocked,
            ..
        }
    ) {
        score.refusals.push(Refusal::BlockedAutomaton)
    }
    score.qualified = score.refusals.is_empty();
    let election = if score.qualified {
        let weights = request
            .record
            .ast
            .alternatives
            .iter()
            .map(|a| a.weight)
            .collect::<Vec<_>>();
        let eligible = request
            .record
            .ast
            .alternatives
            .iter()
            .map(|a| !a.inhibited)
            .collect::<Vec<_>>();
        Some(super::election::elect_at(
            &weights,
            &eligible,
            tape,
            &mut position,
            DrawPurpose::Alternative,
        )?)
    } else {
        None
    };
    if position - tape.position > budget.draws {
        return Err(ScoringError::BudgetExceeded);
    }
    if score.qualified && election.as_ref().is_none_or(|e| e.winner.is_none()) {
        score.refusals.push(Refusal::NoAlternative);
        score.qualified = false;
    }
    let contribution = election
        .as_ref()
        .and_then(|e| e.winner)
        .map(|slot| Contribution {
            id: request.id,
            record: request.record.clone(),
            origin: request.origin.clone(),
            source_original: request.source.original().into(),
            target_origin: request.target.origin().clone(),
            parent_lineage: request.parent_lineage.into(),
            inhibitions: request.inhibitions.to_vec(),
            dependencies: request.dependencies.to_vec(),
            attachment_group: request.attachment_group,
            kind: request.kind,
            slot,
            score: score.clone(),
            election: election.as_ref().expect("winner has election").clone(),
            evidence,
            strength: request.policy.strength,
        });
    tape.position = position;
    Ok(EntryOutcome {
        score,
        election,
        contribution,
    })
}
fn add_guard_refusals(c: &Constraints, input: bool, out: &mut Vec<Refusal>) {
    if c.inhibited {
        out.push(if input {
            Refusal::InputInhibited
        } else {
            Refusal::TargetInhibited
        })
    }
    if c.uncertain_negation {
        out.push(Refusal::UncertainNegation)
    }
    if c.unresolved_frame || c.conditional == ConditionalDisposition::Unresolved {
        out.push(Refusal::Unresolved)
    }
    if c.conditional == ConditionalDisposition::Refuted {
        out.push(Refusal::Refuted)
    }
}
fn preflight(r: &EntryRequest<'_>, w: &mut super::budget::Work) -> Result<(), ScoringError> {
    use std::collections::BTreeSet;
    super::policy::validate_policy(r.policy)?;
    w.bytes(2048)?;
    w.prepared(r.source)?;
    w.prepared(r.target)?;
    w.label(&r.record.population.tenant)?;
    w.label(&r.record.population.specimen)?;
    w.label(&r.record.population.node_type)?;
    w.key(&r.record.key)?;
    w.text(&r.record.original)?;
    w.ast(&r.record.ast)?;
    // Reserve the weight/eligibility/index buffers and the two retained election
    // traces in addition to the original alternative records.
    w.items(&r.record.ast.alternatives)?;
    w.items(&r.record.ast.alternatives)?;
    if let Some(id) = &r.record.resource {
        w.label(&id.0)?
    }
    w.origin(&r.origin.origin)?;
    w.origin(&r.target_binding.origin)?;
    w.items(&r.origin.token_spans)?;
    w.path(&r.origin.path)?;
    w.captures(&r.origin.captures)?;
    w.label(r.parent_lineage)?;
    w.items(r.dependencies)?;
    w.items(r.inhibitions)?;
    let mut facts = BTreeSet::new();
    for fact in r.inhibitions {
        w.charge(
            (1 + fact.code.len())
                .checked_mul((usize::BITS - r.inhibitions.len().leading_zeros()) as usize)
                .ok_or(ScoringError::BudgetExceeded)?,
        )?;
        w.label(&fact.code)?;
        if !facts.insert(fact) {
            return Err(ScoringError::InvalidContribution);
        }
    }
    if r.dependencies.contains(&r.id)
        || r.dependencies.iter().collect::<BTreeSet<_>>().len() != r.dependencies.len()
    {
        return Err(ScoringError::InvalidContribution);
    }
    super::policy::validate_constraints(&r.origin.constraints)?;
    let source = r.source;
    let target = r.target;
    if r.target_binding.node != r.record.node
        || r.target_binding.entry != r.record.reference
        || &r.target_binding.origin != target.origin()
        || r.record.original != target.original()
        || &r.origin.origin != source.origin()
        || r.origin.span != source.span()
        || r.origin.constraints != Constraints::from(source)
        || r.origin.token_spans.len() != source.tokens().len()
        || !r
            .origin
            .token_spans
            .iter()
            .zip(source.tokens())
            .all(|(a, b)| *a == b.span())
    {
        return Err(ScoringError::BindingMismatch);
    }
    let target_key = crate::text_v3::encode(target).map_err(|_| ScoringError::BindingMismatch)?;
    if target_key.pattern_key() != &r.record.key {
        return Err(ScoringError::BindingMismatch);
    }
    if let CandidatePath::Structural(k) = &r.origin.path
        && (k != &r.record.key
            || crate::text_v3::encode(source)
                .map_err(|_| ScoringError::BindingMismatch)?
                .pattern_key()
                != k)
    {
        return Err(ScoringError::BindingMismatch);
    }
    if r.origin.bypass == SimilarityBypass::Declared
        && !matches!(r.origin.path, CandidatePath::Wildcard(_))
    {
        return Err(ScoringError::BindingMismatch);
    }
    let mut names = BTreeSet::new();
    for c in &r.origin.captures {
        w.charge(
            (1 + c.name.len())
                .checked_mul((usize::BITS - r.origin.captures.len().leading_zeros()) as usize)
                .ok_or(ScoringError::BudgetExceeded)?,
        )?;
        if !names.insert(&c.name)
            || c.span.start < source.span().start
            || c.span.end > source.span().end
            || !source
                .original()
                .is_char_boundary(c.span.start - source.span().start)
            || !source
                .original()
                .is_char_boundary(c.span.end - source.span().start)
        {
            return Err(ScoringError::BindingMismatch);
        }
    }
    Ok(())
}
/// Atomic batch entry evaluation. Admission validates all bounded metadata and
/// duplicate IDs before creating any contribution; errors roll back the tape.
pub fn evaluate_entries(
    requests: &[EntryRequest<'_>],
    tape: &mut DrawTape<'_>,
    budget: ScoringBudget,
) -> Result<Vec<EntryOutcome>, ScoringError> {
    let mut work = super::budget::Work::new(budget)?;
    if requests.len() > budget.contributions {
        return Err(ScoringError::BudgetExceeded);
    }
    work.items(requests)?;
    let mut ids = std::collections::BTreeSet::new();
    for r in requests {
        work.charge(1 + (usize::BITS - requests.len().leading_zeros()) as usize)?;
        if !ids.insert(r.id) {
            return Err(ScoringError::InvalidContribution);
        }
        preflight(r, &mut work)?;
    }
    let initial = tape.position;
    let result = (|| {
        let mut out = Vec::with_capacity(requests.len());
        for r in requests {
            let request = EntryRequest {
                id: r.id,
                source: r.source,
                target: r.target,
                record: r.record,
                origin: r.origin,
                target_binding: r.target_binding,
                parent_lineage: r.parent_lineage,
                inhibitions: r.inhibitions,
                dependencies: r.dependencies,
                attachment_group: r.attachment_group,
                kind: r.kind,
                policy: r.policy,
            };
            out.push(evaluate_entry(request, tape, budget)?);
            if tape.position - initial > budget.draws {
                return Err(ScoringError::BudgetExceeded);
            }
        }
        Ok(out)
    })();
    if result.is_err() {
        tape.position = initial
    }
    result
}
