use super::*;
use crate::v3::commitment::CanonicalDecodeLimits;

#[test]
fn typed_decoder_retains_minimal_input() {
    let input = minimal();
    let encoded = CompleteEpisode::try_new(input.clone()).unwrap();
    let bytes = encoded.canonical_bytes().unwrap();
    let decoded =
        DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
            .unwrap();
    assert_eq!(decoded.input(), &input);
    assert_eq!(decoded.episode(), &encoded);
    let (retained, episode) = decoded.into_parts();
    assert_eq!(retained, input);
    assert_eq!(episode, encoded);
}

pub(super) fn check(input: CompleteEpisodeInput) -> CompleteEpisodeInput {
    let original = CompleteEpisode::try_new(input.clone()).unwrap();
    let decoded = DecodedCompleteEpisode::from_canonical_bytes(
        &original.canonical_bytes().unwrap(),
        CanonicalDecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(decoded.input(), &super::decode_normalize::normalize(input));
    assert_eq!(decoded.episode(), &original);
    decoded.into_parts().0
}
#[test]
fn populated_and_minimal_pins_retain_every_leaf() {
    for (input, pin) in [
        (minimal(), include_str!("provisional-minimal.hex")),
        (populated(), include_str!("provisional-populated.hex")),
    ] {
        let encoded = bytes(input.clone());
        assert_eq!(super::draft_witness::hex(&encoded), pin.trim());
        check(input);
    }
}
#[test]
fn set_permutations_and_action_sequences() {
    let mut a = populated();
    a.authority.models = vec![version("z"), version("aa")];
    a.decision.actions[0].parameter_classes = vec![
        ParameterClassRef::new("z").unwrap(),
        ParameterClassRef::new("aa").unwrap(),
    ];
    a.context.regime_posterior = Estimate::Known(
        RegimeDistribution::new(vec![
            (RegimeRef::new("z").unwrap(), UnitPpm::new(0).unwrap()),
            (
                RegimeRef::new("aa").unwrap(),
                UnitPpm::new(1_000_000).unwrap(),
            ),
        ])
        .unwrap(),
    );
    let mut b = a.clone();
    b.authority.models.reverse();
    b.decision.actions[0].parameter_classes.reverse();
    known_mut(&mut b.context.regime_posterior).0.reverse();
    b.provenance.causal_parents.reverse();
    assert_eq!(bytes(a.clone()), bytes(b.clone()));
    assert_eq!(check(a.clone()), check(b));
    let mut different = a.decision.actions[0].clone();
    different.action = ActionRef::new("other-action").unwrap();
    a.decision.actions.push(different);
    let mut b = a.clone();
    b.decision.actions.reverse();
    assert_ne!(bytes(a.clone()), bytes(b.clone()));
    assert_ne!(check(a), check(b));
}
#[test]
fn self_reference_validation_propagates() {
    let encoded = CompleteEpisode::try_new(minimal()).unwrap();
    let bytes = encoded.canonical_bytes().unwrap();
    let (result, calls) =
        super::super::validate::self_reference_probe::run(encoded.digest().unwrap(), true, || {
            DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
        });
    assert_eq!(calls, 1);
    assert_eq!(
        result.unwrap_err(),
        CompleteEpisodeDecodeError::InvalidValue(CompleteEpisodeError::SelfReference)
    );
}
