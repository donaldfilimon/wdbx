//! Private bounded inverse of the provisional complete envelope.
use super::*;
use crate::v3::commitment::{CanonicalDecodeError, CanonicalDecodeLimits, CanonicalValue as C};
mod containers;
mod groups;
mod references;
mod vocabulary;
use containers::*;
use groups::*;
use references::*;
use vocabulary::*;
type DResult<T> = std::result::Result<T, CompleteEpisodeDecodeError>;
use CompleteEpisodeDecodeError as E;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CompleteEpisodeDecodeError {
    #[error(transparent)]
    Canonical(#[from] CanonicalDecodeError),
    #[error("invalid complete episode shape")]
    InvalidShape,
    #[error("unsupported complete episode domain")]
    UnsupportedDomain,
    #[error("unsupported complete episode version")]
    UnsupportedVersion,
    #[error("unknown complete episode variant")]
    UnknownVariant,
    #[error(transparent)]
    InvalidValue(#[from] CompleteEpisodeError),
    #[error("noncanonical complete episode encoding")]
    NonCanonicalEncoding,
}

pub(crate) struct DecodedCompleteEpisode {
    input: CompleteEpisodeInput,
    episode: CompleteEpisode,
}
redacted!(DecodedCompleteEpisode);
impl DecodedCompleteEpisode {
    /// Decode bounded syntax and typed values without conferring admission authority.
    /// `max_parent_count` is an old-envelope-only policy and is inert here.
    /// Other caller limits, including zero, retain their generic-reader semantics.
    /// Peak memory includes the generic tree, retained input, bounded clone and
    /// canonical buffers; the input cap is not a peak-memory or OOM guarantee.
    pub(crate) fn from_canonical_bytes(
        bytes: &[u8],
        mut limits: CanonicalDecodeLimits,
    ) -> DResult<Self> {
        limits.max_input_bytes = limits.max_input_bytes.min(MAX_ENCODED_SIZE);
        let tree = C::from_canonical_bytes(bytes, limits)?;
        let C::Map(entries) = tree else {
            return Err(E::InvalidShape);
        };
        if entries.len() != 12 {
            return Err(E::InvalidShape);
        }
        let mut values = Vec::with_capacity(12);
        for (index, (key, value)) in entries.into_iter().enumerate() {
            if key != C::Unsigned(index as u64) {
                return Err(E::InvalidShape);
            }
            values.push(value);
        }
        let mut values = values.into_iter();
        if text(next(&mut values)?)? != COMPLETE_EPISODE_DOMAIN {
            return Err(E::UnsupportedDomain);
        }
        if unsigned(next(&mut values)?)? != 1 {
            return Err(E::UnsupportedVersion);
        }
        let input = CompleteEpisodeInput::new((
            identity(next(&mut values)?)?,
            time(next(&mut values)?)?,
            origin(next(&mut values)?)?,
            context(next(&mut values)?)?,
            decision(next(&mut values)?)?,
            outcome(next(&mut values)?)?,
            authority(next(&mut values)?)?,
            evidence(next(&mut values)?)?,
            lifecycle(next(&mut values)?)?,
            provenance(next(&mut values)?)?,
        ));
        let episode = CompleteEpisode::try_new(input.clone())?;
        if episode.bytes != bytes {
            return Err(E::NonCanonicalEncoding);
        }
        Ok(Self { input, episode })
    }
    pub(crate) fn input(&self) -> &CompleteEpisodeInput {
        &self.input
    }
    pub(crate) fn episode(&self) -> &CompleteEpisode {
        &self.episode
    }
    pub(crate) fn into_parts(self) -> (CompleteEpisodeInput, CompleteEpisode) {
        (self.input, self.episode)
    }
}
