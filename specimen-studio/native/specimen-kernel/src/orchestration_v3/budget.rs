use super::*;
use crate::{
    language::Expr,
    logical_charge_v1::{CONTRIBUTION_BYTES_V1, LogicalChargeV1},
};
pub(super) const JOB_BYTES: usize = 160;
pub(super) const CYCLE_BYTES: usize = 256;
// Structural cleanup units: cycle traversal/status and job/state/queue/drop visits.
pub(super) const CYCLE_CLEANUP: usize = 4;
pub(super) const JOB_CLEANUP: usize = 4;
pub(super) struct Meter {
    left: usize,
    pub bytes: usize,
}
impl Meter {
    pub fn new(work: usize) -> Self {
        Self {
            left: work,
            bytes: 0,
        }
    }
    pub fn remaining(&self) -> usize {
        self.left
    }
    pub fn add(&mut self, n: usize) -> Result<(), Error> {
        self.left = self.left.checked_sub(n).ok_or(Error::Backpressure)?;
        self.bytes = self.bytes.checked_add(n).ok_or(Error::Backpressure)?;
        Ok(())
    }
    pub fn items<T: LogicalChargeV1>(&mut self, v: &[T]) -> Result<(), Error> {
        if v.len() > 100_000 {
            return Err(Error::Backpressure);
        }
        self.add(
            v.len()
                .checked_mul(T::UNIT_BYTES)
                .ok_or(Error::Backpressure)?,
        )
    }
    pub fn text(&mut self, s: &str) -> Result<(), Error> {
        if s.len() > 8000 {
            return Err(Error::Backpressure);
        }
        self.add(s.len())
    }
    pub fn label(&mut self, s: &str) -> Result<(), Error> {
        if s.is_empty() {
            return Err(Error::Invalid);
        }
        self.text(s)
    }
    pub fn population(&mut self, p: &Population) -> Result<(), Error> {
        self.label(&p.tenant)?;
        self.label(&p.specimen)?;
        self.label(&p.node_type)
    }
    pub fn origin(&mut self, o: &Origin) -> Result<(), Error> {
        self.label(o.input_id())?;
        self.items(o.rule_refs())?;
        for s in o.rule_refs() {
            self.label(s)?
        }
        Ok(())
    }
    pub fn captures(&mut self, v: &[RouteCapture]) -> Result<(), Error> {
        self.items(v)?;
        for c in v {
            self.label(&c.name)?;
            if c.span.start > c.span.end {
                return Err(Error::Binding);
            }
            match &c.value {
                CaptureValue::Text(s) | CaptureValue::Operator(s) => self.text(s)?,
                CaptureValue::Bytes(b) => {
                    if b.len() > 8000 {
                        return Err(Error::Backpressure);
                    }
                    self.items(b)?
                }
                CaptureValue::Number(_) => {}
            }
        }
        Ok(())
    }
    fn tree(&mut self, t: &[Expr], depth: usize, n: &mut usize) -> Result<(), Error> {
        if depth > 16 || t.len() > 4096usize.saturating_sub(*n) {
            return Err(Error::Backpressure);
        }
        self.items(t)?;
        for e in t {
            *n += 1;
            match e {
                Expr::Text { value } => self.text(value)?,
                Expr::Call { name, args } => {
                    if args.len() > 4096usize.saturating_sub(*n) {
                        return Err(Error::Backpressure);
                    }
                    *n += args.len();
                    self.label(name)?;
                    self.items(args)?;
                    for a in args {
                        self.tree(a, depth + 1, n)?
                    }
                }
            }
        }
        Ok(())
    }
    pub fn record(&mut self, r: &EntryRecord) -> Result<(), Error> {
        self.population(&r.population)?;
        self.text(&r.original)?;
        self.label(&r.key.encoder_version)?;
        self.label(&r.key.modality)?;
        self.label(&r.key.resolution)?;
        if let Some(v) = &r.resource {
            self.label(&v.0)?
        }
        self.tree(&r.ast.bind, 1, &mut 0)?;
        self.items(&r.ast.alternatives)?;
        for a in &r.ast.alternatives {
            if !a.weight.is_finite() || a.weight < 0.0 {
                return Err(Error::Invalid);
            }
            self.tree(&a.action, 1, &mut 0)?
        }
        Ok(())
    }
    pub fn comparison(&mut self, c: &Comparison<'_>) -> Result<(), Error> {
        self.record(c.record)?;
        self.text(c.source.original())?;
        self.text(c.target.original())?;
        self.origin(c.source.origin())?;
        self.origin(c.target.origin())?;
        self.origin(&c.origin.origin)?;
        self.captures(&c.origin.captures)?;
        self.items(&c.origin.token_spans)?;
        self.label(c.parent_lineage)?;
        self.items(c.dependencies)?;
        match &c.origin.path {
            CandidatePath::Structural(k) => {
                self.label(&k.encoder_version)?;
                self.label(&k.modality)?;
                self.label(&k.resolution)?
            }
            CandidatePath::Grammar(s)
            | CandidatePath::Wildcard(s)
            | CandidatePath::Automaton { rule: s, .. } => self.label(s)?,
        }
        Ok(())
    }
    pub fn comparison_binding(&mut self, c: &Comparison<'_>) -> Result<(), Error> {
        // Charge variable-length equality independently of the metadata pass.
        self.origin(c.source.origin())?;
        self.origin(&c.origin.origin)?;
        self.text(&c.record.original)?;
        self.text(c.target.original())?;
        self.add(8)?;
        if c.source.origin() != &c.origin.origin
            || c.source.span() != c.origin.span
            || c.origin.constraints != Constraints::from(c.source)
            || c.record.original != c.target.original()
        {
            return Err(Error::Binding);
        }
        Ok(())
    }
    pub fn candidate(&mut self, c: &CandidateOrigin) -> Result<(), Error> {
        self.origin(&c.origin)?;
        self.items(&c.token_spans)?;
        self.captures(&c.captures)?;
        self.add(64)?;
        match &c.path {
            CandidatePath::Structural(k) => {
                self.label(&k.encoder_version)?;
                self.label(&k.modality)?;
                self.label(&k.resolution)?
            }
            CandidatePath::Grammar(s)
            | CandidatePath::Wildcard(s)
            | CandidatePath::Automaton { rule: s, .. } => self.label(s)?,
        }
        Ok(())
    }
    pub fn contribution(&mut self, c: &Contribution) -> Result<(), Error> {
        self.add(CONTRIBUTION_BYTES_V1)?;
        self.record(c.record())?;
        self.origin(&c.origin().origin)?;
        self.origin(c.target_origin())?;
        self.text(c.source_original())?;
        self.captures(&c.origin().captures)?;
        self.items(&c.origin().token_spans)?;
        self.label(c.parent_lineage())?;
        self.items(c.dependencies())?;
        self.items(c.inhibitions())?;
        for f in c.inhibitions() {
            self.label(&f.code)?
        }
        self.items(&c.election().weights)?;
        self.items(&c.election().refused)?;
        self.items(&c.score().draws)?;
        self.label(&c.score().random.stream)?;
        self.label(&c.election().random.stream)?;
        for d in c.score().draws.iter().chain(c.election().draw.iter()) {
            self.label(&d.stream)?
        }
        for e in &c.evidence().components {
            self.items(&e.left_spans)?;
            self.items(&e.right_spans)?
        }
        match &c.origin().path {
            CandidatePath::Structural(k) => {
                self.label(&k.encoder_version)?;
                self.label(&k.modality)?;
                self.label(&k.resolution)?
            }
            CandidatePath::Grammar(s)
            | CandidatePath::Wildcard(s)
            | CandidatePath::Automaton { rule: s, .. } => self.label(s)?,
        }
        Ok(())
    }
}
pub(super) fn scope(p: &Population) -> Result<(), Error> {
    Meter::new(1_000_000).population(p)
}
pub(super) fn binding(c: &Contribution, id: u128, b: &Comparison<'_>) -> Result<(), Error> {
    if c.id() != id
        || c.record() != b.record
        || c.origin() != b.origin
        || c.source_original() != b.source.original()
        || c.target_origin() != b.target.origin()
        || c.parent_lineage() != b.parent_lineage
        || c.dependencies() != b.dependencies
        || c.attachment_group() != b.attachment_group
        || c.kind() != b.kind
        || c.evidence().version != "text-scanner-v3"
        || c.evidence()
            != &crate::scanner_v3::scan(b.source, b.target).map_err(ScoringError::from)?
        || !c.score().qualified
    {
        return Err(Error::Binding);
    }
    Ok(())
}
