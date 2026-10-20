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
