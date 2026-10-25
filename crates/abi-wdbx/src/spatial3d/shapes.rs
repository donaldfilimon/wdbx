//! Closed checked Cartesian shapes with exact represented-input decisions.
use super::{Point3, Radius, SpatialError, exact};

/// A closed axis-aligned box, including degenerate boxes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    min: Point3,
    max: Point3,
}
impl Aabb3 {
    /// Refuse reversed bounds along any axis.
    pub fn try_new(min: Point3, max: Point3) -> Result<Self, SpatialError> {
        if min
            .coordinates()
            .iter()
            .zip(max.coordinates())
            .any(|(a, b)| *a > b)
        {
            return Err(SpatialError::InvalidBounds);
        }
        Ok(Self { min, max })
    }
    /// Lower corner.
    #[must_use]
    pub fn min(self) -> Point3 {
        self.min
    }
    /// Upper corner.
    #[must_use]
    pub fn max(self) -> Point3 {
        self.max
    }
    /// Whether the closed box contains the point.
    #[must_use]
    pub fn contains_point(self, point: Point3) -> bool {
        point
            .coordinates()
            .iter()
            .zip(self.min.coordinates())
            .zip(self.max.coordinates())
            .all(|((p, a), b)| *p >= a && *p <= b)
    }
    /// Whether the closed boxes share any point.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        (0..3).all(|i| {
            self.min.coordinates()[i] <= other.max.coordinates()[i]
                && other.min.coordinates()[i] <= self.max.coordinates()[i]
        })
    }
}
/// A closed sphere with a validated center and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere3 {
    center: Point3,
    radius: Radius,
}
impl Sphere3 {
    /// Construct from validated primitives.
    #[must_use]
    pub fn new(center: Point3, radius: Radius) -> Self {
        Self { center, radius }
    }
    /// Center.
    #[must_use]
    pub fn center(self) -> Point3 {
        self.center
    }
    /// Radius.
    #[must_use]
    pub fn radius(self) -> Radius {
        self.radius
    }
    /// Whether the exact squared distance is at most the squared radius.
    #[must_use]
    pub fn contains_point(self, point: Point3) -> bool {
        exact::within(self.center, point, &[self.radius.get()])
    }
    /// Whether the closed spheres intersect, without rounding their radius sum.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        exact::within(
            self.center,
            other.center,
            &[self.radius.get(), other.radius.get()],
        )
    }
    /// Whether the sphere intersects a closed box.
    #[must_use]
    pub fn intersects_aabb(&self, bounds: &Aabb3) -> bool {
        let mut nearest = self.center.coordinates();
        for (i, value) in nearest.iter_mut().enumerate() {
            *value = value.clamp(bounds.min.coordinates()[i], bounds.max.coordinates()[i]);
        }
        // Each selected coordinate comes from an already validated primitive.
        let point =
            Point3::try_new(nearest[0], nearest[1], nearest[2]).expect("validated box projection");
        self.contains_point(point)
    }
}
