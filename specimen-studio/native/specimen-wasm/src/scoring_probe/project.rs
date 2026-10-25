use serde_json::{Value, json};
use specimen_kernel::{
    language_v3::*,
    scanner_v3::{Evidence, Rational, routes::*},
    scoring_v3::*,
};
pub fn fraction(r: Rational) -> Value {
    json!({"n":r.numerator().to_string(),"d":r.denominator().to_string()})
}
pub fn policy(p: Policy) -> Value {
    json!({"strength":p.strength,"threshold":p.threshold,"jitter":p.jitter,"cap":fraction(p.cap),"valence":fraction(p.atp.valence),"intensity":fraction(p.atp.intensity)})
}
pub fn draw(d: &DrawRecord) -> Value {
    json!({"seed":d.seed.to_string(),"stream":d.stream,"index":d.index,"value":d.value,"purpose":format!("{:?}",d.purpose)})
}
pub fn score(s: &ScoreTrace) -> Value {
    json!({"seed":s.random.seed.to_string(),"stream":s.random.stream,"scan":fraction(s.scan),"baseline":fraction(s.baseline),"delta":fraction(s.delta),"effective":fraction(s.effective),"baseline_qualified":s.baseline_qualified,"qualified":s.qualified,"reason":format!("{:?}",s.jitter_reason),"refusals":s.refusals.iter().map(|r|format!("{r:?}")).collect::<Vec<_>>(),"draws":s.draws.iter().map(draw).collect::<Vec<_>>()})
}
pub fn election(e: &ElectionTrace) -> Value {
    json!({"seed":e.random.seed.to_string(),"stream":e.random.stream,"weight_bits":e.weights.iter().map(|w|format!("{:016x}",w.to_bits())).collect::<Vec<_>>(),"refused":e.refused,"draw":e.draw.as_ref().map(draw),"winner":e.winner})
}
pub fn span(s: Span) -> Value {
    json!([s.start, s.end])
}
pub fn constraints(c: &Constraints) -> Value {
    json!({"inhibited":c.inhibited,"uncertain":c.uncertain_negation,"unresolved":c.unresolved_frame,"conditional":format!("{:?}",c.conditional),"executable":c.executable})
}
pub fn origin(o: &Origin) -> Value {
    json!({"input":o.input_id(),"clause":o.clause_id(),"action":o.action_id(),"fanout":o.fanout_id(),"rules":o.rule_refs()})
}
pub fn captures(cs: &[RouteCapture]) -> Value {
    json!(cs.iter().map(|c|json!({"name":c.name,"span":span(c.span),"value":match &c.value {CaptureValue::Text(s)=>json!({"tag":"Text","value":s}),CaptureValue::Operator(s)=>json!({"tag":"Operator","value":s}),CaptureValue::Bytes(b)=>json!({"tag":"Bytes","value":b}),CaptureValue::Number(n)=>json!({"tag":"Number","coefficient":n.coefficient().to_string(),"scale":n.scale()})}})).collect::<Vec<_>>())
}
pub fn prepared(a: &PreparedAction) -> Value {
    let key = specimen_kernel::text_v3::encode(a).unwrap();
    json!({"original":a.original(),"origin":origin(a.origin()),"span":span(a.span()),"token_spans":a.tokens().iter().map(|t|span(t.span())).collect::<Vec<_>>(),"constraints":constraints(&a.into()),"key":format!("{:?}",key.pattern_key())})
}
pub fn evidence(e: &Evidence) -> Value {
    json!({"version":e.version,"components":e.components.iter().map(|c|json!({"name":c.name,"weight":c.weight.to_string(),"similarity":fraction(c.similarity),"dissimilarity":fraction(c.dissimilarity),"reason":format!("{:?}",c.reason),"left":c.left_spans.iter().copied().map(span).collect::<Vec<_>>(),"right":c.right_spans.iter().copied().map(span).collect::<Vec<_>>()})).collect::<Vec<_>>(),"similarity":fraction(e.totals.similarity),"dissimilarity":fraction(e.totals.dissimilarity),"difference":fraction(e.totals.difference),"confidence":fraction(e.totals.confidence),"left_constraints":constraints(&e.left_constraints),"right_constraints":constraints(&e.right_constraints)})
}
pub fn contribution(c: &Contribution) -> Value {
    json!({"id":c.id().to_string(),"node":c.record().node.0.to_string(),"entry":c.record().reference.0.to_string(),"population":{"tenant":c.record().population.tenant,"specimen":c.record().population.specimen,"node_type":c.record().population.node_type},"root":origin(&c.origin().origin),"source_original":c.source_original(),"target_origin":origin(c.target_origin()),"parent":c.parent_lineage(),"inhibitions":c.inhibitions().iter().map(|f|json!({"domain":format!("{:?}",f.domain),"code":f.code})).collect::<Vec<_>>(),"captures":captures(&c.origin().captures),"dependencies":c.dependencies().iter().map(ToString::to_string).collect::<Vec<_>>(),"group":c.attachment_group().map(|v|v.to_string()),"kind":format!("{:?}",c.kind()),"slot":c.slot(),"strength":c.strength(),"action":c.action(),"score":score(c.score()),"election":election(c.election()),"evidence":evidence(c.evidence())})
}
pub fn arbitration(a: &ArbitrationTrace) -> Value {
    json!({"candidates":a.candidates.iter().map(ToString::to_string).collect::<Vec<_>>(),"winner":a.winner.map(|v|v.to_string()),"election":election(&a.election)})
}
