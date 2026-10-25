use super::broker::State;
use super::budget::Meter;
use super::*;
use crate::{
    logical_charge_v1::{
        ATTACHMENT_CONTEXT_BYTES_V1, ATTACHMENT_CONTEXT_REFERENCE_BYTES_V1,
        ATTACHMENT_VISIT_BYTES_V1, LogicalChargeV1,
    },
    scanner_v3::Rational,
    scoring_v3::DrawIdentity,
};
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Directed,
    Bidirectional,
}
#[derive(Debug, Clone, Copy)]
pub enum AttachmentMode {
    Hard,
    Soft(Rational),
}
#[derive(Debug, Clone, Copy)]
pub struct Attachment {
    pub from: NodeRef,
    pub to: NodeRef,
    pub direction: Direction,
    pub mode: AttachmentMode,
    pub group: u128,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffinityTrace {
    pub random: DrawIdentity,
    pub draw: Option<(usize, u32)>,
    pub affinity: Option<Rational>,
    pub selected: bool,
}
/// Separate explicitly labelled attachment stream. Neither scorer private cursor
/// nor ambient randomness is accessed. A failed enqueue never consumes a draw.
pub struct AffinityTape<'a> {
    identity: DrawIdentity,
    values: &'a [u32],
    position: usize,
    limit: usize,
}
impl<'a> AffinityTape<'a> {
    pub fn new(seed: u64, stream: &str, values: &'a [u32], limit: usize) -> Result<Self, Error> {
        if stream.is_empty() || stream.len() > 128 || limit > 100_000 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            identity: DrawIdentity {
                seed,
                stream: stream.into(),
            },
            values,
            position: 0,
            limit,
        })
    }
    pub fn consumed(&self) -> usize {
        self.position
    }
    fn sample(&self, mode: AttachmentMode) -> Result<AffinityTrace, Error> {
        let (affinity, draw, selected) = match mode {
            AttachmentMode::Hard => (None, None, true),
            AttachmentMode::Soft(a) => {
                if a.numerator() < 0
                    || a.numerator() > a.denominator()
                    || a.denominator() > 1_000_000
                {
                    return Err(Error::Invalid);
                }
                if a.numerator() == 0 {
                    (Some(a), None, false)
                } else if a.numerator() == a.denominator() {
                    (Some(a), None, true)
                } else {
                    if self.position >= self.limit {
                        return Err(Error::Backpressure);
                    }
                    let d = *self.values.get(self.position).ok_or(Error::Backpressure)?;
                    let l = i128::from(d)
                        .checked_mul(a.denominator())
                        .ok_or(Error::Invalid)?;
                    let r = a
                        .numerator()
                        .checked_mul(i128::from(u32::MAX))
                        .ok_or(Error::Invalid)?;
                    (Some(a), Some((self.position, d)), l < r)
                }
            }
        };
        Ok(AffinityTrace {
            random: self.identity.clone(),
            draw,
            affinity,
            selected,
        })
    }
}
/// Immutable inherited provenance. `authoritative_captures` returns only whole
/// captures inside the delegated span; other captures remain contextual here.
#[derive(Debug)]
pub struct AttachmentContext<'a> {
    pub(super) root: &'a PreparedAction,
    pub(super) span: Span,
    pub(super) text: &'a str,
    pub(super) guards: Constraints,
    pub(super) captures: Vec<RouteCapture>,
    pub(super) parent: String,
    pub(super) issuer: u128,
    pub(super) group: u128,
    pub(super) depth: u8,
    pub(super) trace: AffinityTrace,
}
impl AttachmentContext<'_> {
    pub fn remainder(&self) -> &str {
        self.text
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn guards(&self) -> &Constraints {
        &self.guards
    }
    pub fn inherited_captures(&self) -> &[RouteCapture] {
        &self.captures
    }
    pub fn authoritative_captures(&self) -> impl Iterator<Item = &RouteCapture> {
        self.captures
            .iter()
            .filter(|c| c.span.start >= self.span.start && c.span.end <= self.span.end)
    }
    pub fn source(&self) -> &PreparedAction {
        self.root
    }
    pub fn parent_lineage(&self) -> &str {
        &self.parent
    }
    pub fn depth(&self) -> u8 {
        self.depth
    }
    pub fn issuer(&self) -> u128 {
        self.issuer
    }
    pub fn group(&self) -> u128 {
        self.group
    }
    pub fn affinity(&self) -> &AffinityTrace {
        &self.trace
    }
}
pub(super) struct Visit<'a> {
    target: NodeRef,
    root: &'a PreparedAction,
    span: Span,
    text: &'a str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentDisposition {
    Scheduled(usize),
    AffinityDeclined,
    EmptyRemainder,
}
#[derive(Debug, Clone)]
pub struct AttachmentReceipt {
    pub disposition: AttachmentDisposition,
    pub trace: Option<AffinityTrace>,
}
impl<'a> Broker<'a> {
    /// Host partition correctness is unqualified; byte containment, Unicode,
    /// inherited guards, captures, depth and the enqueue barrier are enforced.
    #[allow(clippy::too_many_arguments)]
    pub fn attach(
        &mut self,
        p: &Population,
        g: u64,
        issuer: &CompletedIssuer,
        link: Attachment,
        remainder: Span,
        jobs: &[Job<'a>],
        tape: &mut AffinityTape<'_>,
    ) -> Result<AttachmentReceipt, Error> {
        self.check(p, g)?;
        if !Arc::ptr_eq(&issuer.owner, &self.owner) || issuer.generation != g {
            return Err(Error::InvalidLease);
        }
        let cycle = self.cycles.get(&issuer.cycle).ok_or(Error::Identity)?;
        if cycle.incarnation != issuer.incarnation {
            return Err(Error::InvalidLease);
        }
        if cycle.status != CycleStatus::Open || cycle.sealed || cycle.discovery_closed {
            return Err(Error::Closed);
        }
        let record = cycle.jobs.get(&issuer.job).ok_or(Error::InvalidLease)?;
        if record.state != State::Done {
            return Err(Error::InvalidLease);
        }
        let proof = record.contribution.as_ref().ok_or(Error::Binding)?;
        let node = proof.record().node;
        let target = if node == link.from {
            link.to
        } else if node == link.to && link.direction == Direction::Bidirectional {
            link.from
        } else {
            return Err(Error::Binding);
        };
        if link.group == 0 || link.from == link.to {
            return Err(Error::Invalid);
        }
        if let AttachmentMode::Soft(a) = link.mode
            && (a.numerator() < 0 || a.numerator() > a.denominator() || a.denominator() > 1_000_000)
        {
            return Err(Error::Invalid);
        }
        let depth = record.depth.checked_add(1).ok_or(Error::Depth)?;
        if depth > 4 {
            return Err(Error::Depth);
        }
        let root = record
            .context
            .as_ref()
            .map_or(record.comparison.source, |c| c.root);
        let containing = record.context.as_ref().map_or(root.span(), |c| c.span);
        if remainder.start > remainder.end
            || remainder.start < containing.start
            || remainder.end > containing.end
        {
            return Err(Error::Binding);
        }
        let start = remainder
            .start
            .checked_sub(root.span().start)
            .ok_or(Error::Binding)?;
        let end = remainder
            .end
            .checked_sub(root.span().start)
            .ok_or(Error::Binding)?;
        let text = root.original().get(start..end).ok_or(Error::Binding)?;
        if text.is_empty() {
            if !jobs.is_empty() {
                return Err(Error::Binding);
            }
            return Ok(AttachmentReceipt {
                disposition: AttachmentDisposition::EmptyRemainder,
                trace: None,
            });
        }
        let mut visit_meter = Meter::new(self.limits.operation_work);
        self.validate_descriptors(issuer.cycle, jobs, &mut visit_meter)?;
        for v in &cycle.visits {
            visit_meter.add(64)?;
            if v.target == target {
                visit_meter.text(v.root.original())?;
                visit_meter.text(root.original())?;
                visit_meter.origin(v.root.origin())?;
                visit_meter.origin(root.origin())?;
                visit_meter.text(v.text)?;
                visit_meter.text(text)?;
            }
        }
        if cycle.visits.iter().any(|v| {
            v.target == target
                && v.root.origin() == root.origin()
                && v.root.original() == root.original()
                && v.span == remainder
                && v.text == text
        }) {
            return Err(Error::Repeated);
        }
        let guards = record
            .context
            .as_ref()
            .map_or(&proof.origin().constraints, |c| &c.guards);
        let captures = record
            .context
            .as_ref()
            .map_or(proof.origin().captures.as_slice(), |c| {
                c.captures.as_slice()
            });
        // New target-local bindings cannot turn an external/partial inherited capture
        // into authority. Authorized inherited spans are rebased only after containment.
        let mut m = visit_meter;
        m.add(
            ATTACHMENT_CONTEXT_BYTES_V1
                .checked_add(ATTACHMENT_VISIT_BYTES_V1)
                .ok_or(Error::Backpressure)?,
        )?;
        m.captures(captures)?;
        m.label(proof.parent_lineage())?;
        m.label(&tape.identity.stream)?;
        if jobs.is_empty() {
            return Err(Error::Invalid);
        }
        for j in jobs {
            let c = j.comparison;
            m.add(16)?;
            m.text(c.source.original())?;
            m.text(text)?;
            m.label(c.parent_lineage)?;
            m.label(proof.parent_lineage())?;
            m.items(c.dependencies)?;
            if j.priority != Priority::Foreground
                || c.record.node != target
                || c.source.original() != text
                || &c.origin.constraints != guards
                || c.parent_lineage != proof.parent_lineage()
                || c.attachment_group != Some(link.group)
                || !c.dependencies.contains(&issuer.job)
            {
                return Err(Error::Binding);
            }
            for cap in &c.origin.captures {
                let mut matched = false;
                for prior in captures {
                    // Charge the actual variable-length equality on every
                    // attempted pair, not merely the outer vector lengths.
                    m.captures(std::slice::from_ref(cap))?;
                    m.captures(std::slice::from_ref(prior))?;
                    if prior.name == cap.name
                        && prior.value == cap.value
                        && prior.span.start >= remainder.start
                        && prior.span.end <= remainder.end
                        && cap.span.start
                            == prior.span.start - remainder.start + c.source.span().start
                        && cap.span.end == prior.span.end - remainder.start + c.source.span().start
                    {
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    return Err(Error::Binding);
                }
            }
        }
        for (i, j) in jobs.iter().enumerate() {
            for previous in &jobs[..i] {
                m.add(1)?;
                if previous.comparison.record.reference == j.comparison.record.reference {
                    m.candidate(previous.comparison.origin)?;
                    m.candidate(j.comparison.origin)?;
                    if previous.comparison.origin == j.comparison.origin {
                        return Err(Error::Identity);
                    }
                }
            }
        }
        self.ensure_new_jobs(issuer.cycle, jobs)?;
        let trace = tape.sample(link.mode)?;
        // Declines are idempotent visit receipts too, and are retained under the same
        // bounded lifecycle. Backpressure leaves visit and RNG state untouched.
        let mut extra = ATTACHMENT_CONTEXT_BYTES_V1
            .checked_add(ATTACHMENT_VISIT_BYTES_V1)
            .and_then(|n| n.checked_add(ATTACHMENT_CONTEXT_REFERENCE_BYTES_V1))
            .and_then(|n| {
                captures
                    .len()
                    .checked_mul(RouteCapture::UNIT_BYTES)
                    .and_then(|bytes| n.checked_add(bytes))
            })
            .ok_or(Error::Backpressure)?;
        for capture in captures {
            let payload = match &capture.value {
                CaptureValue::Text(s) | CaptureValue::Operator(s) => s.len(),
                CaptureValue::Bytes(b) => b.len(),
                CaptureValue::Number(_) => 0,
            };
            extra = extra
                .checked_add(capture.name.len())
                .and_then(|n| n.checked_add(payload))
                .ok_or(Error::Backpressure)?;
        }
        extra = extra
            .checked_add(proof.parent_lineage().len())
            .and_then(|n| n.checked_add(tape.identity.stream.len()))
            .ok_or(Error::Backpressure)?;
        let job_bytes = if trace.selected {
            jobs.len()
                .checked_mul(budget::JOB_BYTES)
                .ok_or(Error::Backpressure)?
        } else {
            0
        };
        self.reserve(extra.checked_add(job_bytes).ok_or(Error::Backpressure)?)?;
        let cleanup = if trace.selected {
            jobs.len()
                .checked_mul(budget::JOB_CLEANUP)
                .ok_or(Error::Backpressure)?
        } else {
            0
        };
        self.reserve_cleanup(extra.checked_add(cleanup).ok_or(Error::Backpressure)?)?;
        if trace.selected {
            self.reserve_jobs(issuer.cycle, jobs)?;
        }
        let context = if trace.selected {
            Some(Arc::new(AttachmentContext {
                root,
                span: remainder,
                text,
                guards: guards.clone(),
                captures: captures.to_vec(),
                parent: proof.parent_lineage().into(),
                issuer: issuer.job,
                group: link.group,
                depth,
                trace: trace.clone(),
            }))
        } else {
            None
        };
        if trace.selected {
            self.insert(issuer.cycle, jobs, context, depth);
        }
        let cycle = self.cycles.get_mut(&issuer.cycle).expect("validated cycle");
        cycle.visits.push(Visit {
            target,
            root,
            span: remainder,
            text,
        });
        cycle.extra_bytes += extra;
        self.owned += extra;
        self.cleanup_reserved += extra;
        if trace.draw.is_some() {
            tape.position += 1
        }
        Ok(AttachmentReceipt {
            disposition: if trace.selected {
                AttachmentDisposition::Scheduled(jobs.len())
            } else {
                AttachmentDisposition::AffinityDeclined
            },
            trace: Some(trace),
        })
    }
}
