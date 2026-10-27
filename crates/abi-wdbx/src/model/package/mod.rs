//! Bounded native package loading. Validation never grants execution authority.
mod components;
mod directory;
mod header;
mod read;
mod stage;
mod write;
use super::error::{ErrorKind, ModelError};
pub use components::{check_resume, validate_model};
pub use directory::validate_framing;
pub use read::{read_bounded_snapshot, read_bounded_snapshot_with_cancel};
use sha2::{Digest as _, Sha256};
pub use stage::{StagedObject, import_staged};
pub use write::{Annotation, build_package, export_exact};
pub(crate) type Result<T> = std::result::Result<T, ModelError>;
pub(crate) fn err(kind: ErrorKind, field: &'static str) -> ModelError {
    ModelError::new(kind, field, None, None)
}
pub(crate) fn require(ok: bool, kind: ErrorKind, field: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(err(kind, field)) }
}
pub(crate) fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub(crate) fn cancelled(cancel: &mut impl FnMut() -> bool) -> Result<()> {
    require(!cancel(), ErrorKind::Cancelled, "cancelled")
}

pub(crate) fn size(n: u64) -> Result<usize> {
    usize::try_from(n).map_err(|_| err(ErrorKind::CapacityExceeded, "platform-size"))
}
