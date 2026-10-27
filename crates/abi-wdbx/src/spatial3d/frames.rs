//! Local frame conversion primitives; no registry, persistence or authority.

use super::{Point3, SpatialError};

/// Caller-supplied content digest of an immutable Cartesian frame definition.
///
/// Equality compares the entire reference. Construction does not establish
/// registration, units, provenance, authorization or digest authenticity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CartesianFrameRef([u8; 32]);

impl CartesianFrameRef {
    /// Refer to a definition already validated by the caller's frame authority.
    #[must_use]
    pub const fn from_definition_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// Return the complete definition reference.
    #[must_use]
    pub const fn definition_digest(self) -> [u8; 32] {
        self.0
    }
}

/// A point with an explicit Cartesian frame, or an explicitly unbound legacy view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CartesianPoint3 {
    point: Point3,
    frame: Option<CartesianFrameRef>,
}

impl CartesianPoint3 {
    /// Bind an already validated point to a caller-validated frame definition.
    #[must_use]
    pub const fn new(point: Point3, frame: CartesianFrameRef) -> Self {
        Self {
            point,
            frame: Some(frame),
        }
    }

    /// Widen legacy coordinates without inferring a physical frame or units.
    pub fn from_legacy(point: crate::spatial::Point3D) -> Result<Self, SpatialError> {
        Ok(Self {
            point: Point3::from_legacy(point)?,
            frame: None,
        })
    }

    /// Coordinates in the stated frame, or unitless when unbound.
    #[must_use]
    pub const fn point(self) -> Point3 {
        self.point
    }

    /// Exact frame reference; `None` never means a default world frame.
    #[must_use]
    pub const fn frame(self) -> Option<CartesianFrameRef> {
        self.frame
    }
}

/// Refusal from local frame conversion, separate from persisted error contracts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    /// Legacy coordinates have no explicit physical binding.
    Unbound,
    /// The complete source reference differs, including definition version.
    MismatchedSource,
    /// Only proper signed permutation rotations are implemented.
    UnsupportedRotation,
    /// Unit scale must be finite and strictly positive.
    InvalidScale,
    /// Converted coordinates violate the checked Cartesian numeric domain.
    Numeric(SpatialError),
}

/// Explicit conversion between caller-validated Cartesian frame definitions.
///
/// Supports the 24 proper signed permutation rotations only (zero tolerance).
/// This excludes arbitrary rotations, reflections, shears and geodetic inputs.
/// A positive scale converts source units to target units; translation is in
/// target units. No conversion path, inverse or registry lookup is inferred.
#[derive(Clone, Copy, Debug)]
pub struct FrameConversion {
    source: CartesianFrameRef,
    target: CartesianFrameRef,
    axes: [(usize, f64); 3],
    scale: f64,
    translation: Point3,
}

impl FrameConversion {
    /// Validate an exact row-major signed permutation and positive unit scale.
    pub fn try_new(
        source: CartesianFrameRef,
        target: CartesianFrameRef,
        rotation: [[i8; 3]; 3],
        scale: f64,
        translation: Point3,
    ) -> Result<Self, FrameError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(FrameError::InvalidScale);
        }
        let mut axes = [(0, 0.0); 3];
        let mut used = [false; 3];
        for (i, row) in rotation.iter().enumerate() {
            let mut found = None;
            for (j, &value) in row.iter().enumerate() {
                if value == 0 {
                    continue;
                }
                if !matches!(value, -1 | 1) || found.is_some() || used[j] {
                    return Err(FrameError::UnsupportedRotation);
                }
                found = Some((j, f64::from(value)));
            }
            axes[i] = found.ok_or(FrameError::UnsupportedRotation)?;
            used[axes[i].0] = true;
        }
        // Every entry is now -1, 0 or 1: determinant arithmetic cannot overflow.
        let m = rotation.map(|row| row.map(i32::from));
        let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if determinant != 1 {
            return Err(FrameError::UnsupportedRotation);
        }
        Ok(Self {
            source,
            target,
            axes,
            scale,
            translation,
        })
    }

    /// Convert with separate IEEE f64 multiplication and addition, then validate.
    ///
    /// Rounding and underflow follow f64 semantics; physical uncertainty and
    /// exact inverse round trips are not guaranteed. No shape bounds are inferred.
    pub fn convert(self, point: CartesianPoint3) -> Result<CartesianPoint3, FrameError> {
        let frame = point.frame.ok_or(FrameError::Unbound)?;
        if frame != self.source {
            return Err(FrameError::MismatchedSource);
        }
        let input = point.point.coordinates();
        let translation = self.translation.coordinates();
        let output: [f64; 3] = std::array::from_fn(|i| {
            let (axis, sign) = self.axes[i];
            (sign * input[axis]) * self.scale + translation[i]
        });
        let converted =
            Point3::try_new(output[0], output[1], output[2]).map_err(FrameError::Numeric)?;
        Ok(CartesianPoint3::new(converted, self.target))
    }
}
