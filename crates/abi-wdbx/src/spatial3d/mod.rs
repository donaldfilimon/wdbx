//! Additive checked Cartesian geometry; legacy storage remains unchanged.

mod numeric;

pub use numeric::{Point3, Radius, SpatialError};

pub(super) mod legacy_checked;
pub use legacy_checked::{CheckedSpatialHit, QueryLimits};

mod octree_checked;
pub use octree_checked::*;
