//! Independent complete scan for defined_float_profile, not exact-real predicates.
use abi_wdbx::spatial3d::{Point3, Radius};
#[derive(Clone, Copy, Debug)]
pub struct OraclePoint {
    pub point: Point3,
    pub id: u32,
    pub ordinal: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleHit {
    pub id: u32,
    pub ordinal: usize,
    pub squared_distance: f64,
    pub distance: f64,
}
// Inputs are already f64 (legacy points widen before any subtraction). IEEE
// subnormals are preserved; ordinary multiply/add only, with left-to-right
// x*x + y*y + z*z reduction and no mul_add/FMA. Geometric zero is canonical.
fn measure(row: &OraclePoint, center: Point3) -> OracleHit {
    let a = row.point.coordinates();
    let b = center.coordinates();
    let x = a[0] - b[0];
    let y = a[1] - b[1];
    let z = a[2] - b[2];
    let squared_distance = (x * x + y * y) + z * z;
    OracleHit {
        id: row.id,
        ordinal: row.ordinal,
        squared_distance,
        distance: squared_distance.sqrt(),
    }
}
fn ordered(points: &[OraclePoint], center: Point3) -> Vec<OracleHit> {
    let mut hits: Vec<_> = points.iter().map(|row| measure(row, center)).collect();
    hits.sort_by(|a, b| {
        a.squared_distance
            .total_cmp(&b.squared_distance)
            .then(a.id.cmp(&b.id))
            .then(a.ordinal.cmp(&b.ordinal))
    });
    hits
}
pub fn reference_radius(points: &[OraclePoint], center: Point3, radius: Radius) -> Vec<OracleHit> {
    let limit = radius.get() * radius.get();
    ordered(points, center)
        .into_iter()
        .filter(|hit| hit.squared_distance <= limit)
        .collect()
}
pub fn reference_knn(points: &[OraclePoint], center: Point3, k: usize) -> Vec<OracleHit> {
    if k == 0 {
        return Vec::new();
    }
    let mut hits = ordered(points, center);
    hits.truncate(k);
    hits
}
