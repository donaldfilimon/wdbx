//! Additive checked Cartesian geometry; legacy storage remains unchanged.

mod numeric;

pub use numeric::{Point3, Radius, SpatialError};

pub(super) mod legacy_checked;
pub use legacy_checked::{CheckedSpatialHit, QueryLimits};

mod octree_checked;
pub use octree_checked::*;

mod exact;
mod shapes;
pub use exact::compare_squared_distances;
pub use shapes::{Aabb3, Sphere3};

mod frames;
pub use frames::{CartesianFrameRef, CartesianPoint3, FrameConversion, FrameError};
