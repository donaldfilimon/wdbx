//! Provisional private complete-episode prototype; subject to ABI ratification.
//! No compatibility promise, admission, persistence, or signature verification.
//! Checked sizing precedes canonical-tree allocation. Temporary encoder buffers
//! mean the output cap is not a claim about peak memory. Caller allocations are
//! outside this guarantee. Explicit byte access intentionally discloses input.
// Consumer access is deliberately gated pending ABI task-02 review.
#![allow(dead_code)]

macro_rules! redacted {
    ($($ty:ty),* $(,)?) => {$(impl std::fmt::Debug for $ty {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(concat!(stringify!($ty), " { .. }"))
        }
    })*};
}
mod canonical;
mod primitives;
mod types;
mod validate;
use canonical::{Wire, counted_size};
pub(crate) use primitives::*;
use sha2::{Digest, Sha256};
pub(crate) use types::*;

pub(crate) const COMPLETE_EPISODE_DOMAIN: &str = "wdbx-complete-episode-v1";
pub(crate) const MAX_ENCODED_SIZE: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CompleteEpisodeError {
    #[error("invalid identifier")]
    InvalidIdentifier,
    #[error("invalid UUID")]
    InvalidUuid,
    #[error("invalid range")]
    InvalidRange,
    #[error("duplicate reference")]
    DuplicateReference,
    #[error("scope mismatch")]
    ScopeMismatch,
    #[error("collection limit")]
    CollectionLimit,
    #[error("encoded size limit")]
    EncodedSizeLimit,
    #[error("invalid posterior")]
    InvalidPosterior,
    #[error("self reference")]
    SelfReference,
    #[error("invalid artifact kind")]
    InvalidArtifactKind,
    #[error("canonical encoding failure")]
    Encoding,
}
type Result<T> = std::result::Result<T, CompleteEpisodeError>;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CompleteEpisode {
    bytes: Vec<u8>,
    digest: [u8; 32],
}
redacted!(CompleteEpisode);
impl CompleteEpisode {
    pub(crate) fn try_new(input: CompleteEpisodeInput) -> Result<Self> {
        validate::validate(&input)?;
        counted_size(&input, MAX_ENCODED_SIZE)?;
        let bytes = input
            .wire::<canonical::Tree>()?
            .canonical_bytes()
            .map_err(|_| CompleteEpisodeError::Encoding)?;
        let digest = Sha256::digest(&bytes).into();
        validate::reject_self_reference(&input, digest)?;
        drop(input);
        Ok(Self { bytes, digest })
    }
    pub(crate) fn canonical_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }
    pub(crate) fn digest(&self) -> Result<[u8; 32]> {
        Ok(self.digest)
    }
}
#[cfg(test)]
fn encode_test(value: &impl Wire) -> Vec<u8> {
    counted_size(value, MAX_ENCODED_SIZE).unwrap();
    value
        .wire::<canonical::Tree>()
        .unwrap()
        .canonical_bytes()
        .unwrap()
}
#[cfg(test)]
mod tests;
