//! Validated numeric primitives for the `cartesian-f64-v1` profile.
use crate::spatial::Point3D;

/// Validation or execution failure in the checked spatial API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpatialError {
    /// An input is NaN or infinite.
    NonFinite,
    /// A coordinate or radius exceeds the profile magnitude limit.
    CoordinateOutOfRange,
    /// A radius is strictly negative.
    NegativeRadius,
    /// A bound minimum exceeds its maximum.
    InvalidBounds,
    /// A work or memory budget would be exceeded.
    BudgetExceeded,
    /// The requested metric is unsupported.
    UnsupportedMetric,
    /// Cooperative cancellation was requested.
    Cancelled,
}

/// A finite Cartesian point with coordinate magnitudes at most `1e150`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3 {
    coordinates: [f64; 3],
}
impl Point3 {
    /// Validate coordinates, normalizing geometric signed zeros to positive zero.
    pub fn try_new(x: f64, y: f64, z: f64) -> Result<Self, SpatialError> {
        let coordinates = [x, y, z];
        if coordinates.iter().any(|value| !value.is_finite()) {
            return Err(SpatialError::NonFinite);
        }
        if coordinates.iter().any(|value| value.abs() > 1e150) {
            return Err(SpatialError::CoordinateOutOfRange);
        }
        Ok(Self {
            coordinates: coordinates.map(normalize_zero),
        })
    }
    /// Explicitly widen a legacy point without modifying its original bits.
    pub fn from_legacy(point: Point3D) -> Result<Self, SpatialError> {
        Self::try_new(f64::from(point.x), f64::from(point.y), f64::from(point.z))
    }
    /// X coordinate.
    #[must_use]
    pub fn x(self) -> f64 {
        self.coordinates[0]
    }
    /// Y coordinate.
    #[must_use]
    pub fn y(self) -> f64 {
        self.coordinates[1]
    }
    /// Z coordinate.
    #[must_use]
    pub fn z(self) -> f64 {
        self.coordinates[2]
    }
    /// Coordinates in X, Y, Z order, returned by value.
    #[must_use]
    pub fn coordinates(self) -> [f64; 3] {
        self.coordinates
    }
}

/// A finite radius in the closed interval `[0, 1e150]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radius(f64);
impl Radius {
    /// Validate a radius, normalizing negative zero to positive zero.
    pub fn try_new(radius: f64) -> Result<Self, SpatialError> {
        if !radius.is_finite() {
            return Err(SpatialError::NonFinite);
        }
        if radius < 0.0 {
            return Err(SpatialError::NegativeRadius);
        }
        if radius > 1e150 {
            return Err(SpatialError::CoordinateOutOfRange);
        }
        Ok(Self(normalize_zero(radius)))
    }
    /// Return the validated radius value.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
