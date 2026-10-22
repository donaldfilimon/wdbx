//! Independent expected typed ordering; never decode to construct expectations.
use super::*;
pub(super) fn normalize(mut input: CompleteEpisodeInput) -> CompleteEpisodeInput {
    input.context.resources.sort_by_key(encode_test);
    input.outcome.costs.sort_by_key(encode_test);
    input
        .authority
        .capabilities
        .sort_by(|a, b| a.component.cmp(&b.component));
    input
        .authority
        .models
        .sort_by(|a, b| a.component.cmp(&b.component));
    input
        .authority
        .calibrations
        .sort_by(|a, b| a.component.cmp(&b.component));
    input.authority.approvals.sort_by_key(encode_test);
    input.evidence.source_refs.sort_by_key(encode_test);
    input.evidence.observation_refs.sort_by_key(encode_test);
    input.lifecycle.holds.sort_by_key(encode_test);
    input.lifecycle.edges.sort_by_key(encode_test);
    input.provenance.causal_parents.sort_by_key(encode_test);
    input.provenance.consulted_parents.sort_by_key(encode_test);
    input.provenance.records.sort_by_key(encode_test);
    for action in &mut input.decision.actions {
        action.parameter_classes.sort();
    }
    if let Estimate::Known(posterior) = &mut input.context.regime_posterior {
        posterior.0.sort_by(|a, b| a.0.cmp(&b.0));
    }
    input
}
