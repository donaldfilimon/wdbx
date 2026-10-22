//! Complete-or-error Euclidean queries over the legacy point population.
use super::{Point3, Radius, SpatialError};
use crate::spatial::SpatialRecord3D;
use crate::spatial::{Point3D, SpatialIndex3D};
use std::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
/// Explicit local query resource ceilings.
#[derive(Clone, Copy, Debug)]
pub struct QueryLimits {
    /// Maximum population observation examinations (validation plus execution).
    pub max_work: usize,
    /// Maximum returned observations.
    pub max_results: usize,
    /// Hit storage plus borrowed payload logical bytes; excludes allocator metadata.
    pub max_output_bytes: usize,
}
/// One borrowed hit; identity is local ID plus insertion ordinal only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckedSpatialHit<'a> {
    /// Legacy numeric identifier.
    pub id: u32,
    /// Zero-based index-local insertion ordinal.
    pub insertion_ordinal: usize,
    /// Unmodified original point.
    pub point: Point3D,
    /// Rounded display distance, not the ordering key.
    pub distance_f64: f64,
    /// Borrowed original payload.
    pub payload: &'a [u8],
    squared_distance: f64,
}
impl SpatialIndex3D {
    /// Closed-radius complete scan under the defined-float profile.
    pub fn try_radius_search_checked(
        &self,
        center: Point3D,
        radius: f32,
        limits: QueryLimits,
    ) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError> {
        self.try_radius_search_checked_with_cancel(center, radius, limits, &AtomicBool::new(false))
    }
    /// Closed-radius scan with cooperative cancellation, polled per examination.
    /// Work is exactly twice the population size, including validation. Errors
    /// return no partial vector. Sorting costs are bounded by the result cap.
    pub fn try_radius_search_checked_with_cancel(
        &self,
        center: Point3D,
        radius: f32,
        limits: QueryLimits,
        cancel: &AtomicBool,
    ) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError> {
        let center = Point3::from_legacy(center)?;
        let radius = Radius::try_new(f64::from(radius))?;
        poll(cancel)?;
        let records = self.checked_records();
        admission(records.len(), limits)?;
        let radius_squared = radius.get() * radius.get();
        let mut count = 0usize;
        let mut bytes = 0usize;
        for record in records {
            poll(cancel)?;
            let point = Point3::from_legacy(record.point)?;
            if squared(center, point) <= radius_squared {
                count = count.checked_add(1).ok_or(SpatialError::BudgetExceeded)?;
                bytes = charge(bytes, record.payload.len())?;
                if count > limits.max_results || bytes > limits.max_output_bytes {
                    return Err(SpatialError::BudgetExceeded);
                }
            }
        }
        let mut hits = Vec::new();
        hits.try_reserve_exact(count)
            .map_err(|_| SpatialError::BudgetExceeded)?;
        for (ordinal, record) in records.iter().enumerate() {
            poll(cancel)?;
            let key = squared(center, Point3::from_legacy(record.point)?);
            if key <= radius_squared {
                hits.push(hit(record, ordinal, key));
            }
        }
        hits.sort_unstable_by(order);
        poll(cancel)?;
        Ok(hits)
    }
    /// Complete nearest selection ordered by squared distance, ID, then ordinal.
    pub fn try_nearest_neighbors_checked(
        &self,
        center: Point3D,
        k: usize,
        limits: QueryLimits,
    ) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError> {
        self.try_nearest_neighbors_checked_with_cancel(center, k, limits, &AtomicBool::new(false))
    }
    /// Complete nearest selection with cooperative cancellation.
    /// Query validation precedes the zero-k fast path (no population access or
    /// allocation). Otherwise work is twice the population size; heap selection
    /// costs O(n log min(k,n)), with at most min(k,n) allocated hit slots. Byte
    /// admission charges those slots before allocation and final payload lengths
    /// after selection, never the lengths of discarded candidates.
    pub fn try_nearest_neighbors_checked_with_cancel(
        &self,
        center: Point3D,
        k: usize,
        limits: QueryLimits,
        cancel: &AtomicBool,
    ) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError> {
        let center = Point3::from_legacy(center)?;
        poll(cancel)?;
        if k == 0 {
            return Ok(Vec::new());
        }
        let records = self.checked_records();
        admission(records.len(), limits)?;
        for record in records {
            poll(cancel)?;
            Point3::from_legacy(record.point)?;
        }
        let count = k.min(records.len());
        let slots = count
            .checked_mul(std::mem::size_of::<CheckedSpatialHit<'_>>())
            .ok_or(SpatialError::BudgetExceeded)?;
        if count > limits.max_results || slots > limits.max_output_bytes {
            return Err(SpatialError::BudgetExceeded);
        }
        let mut hits = Vec::new();
        hits.try_reserve_exact(count)
            .map_err(|_| SpatialError::BudgetExceeded)?;
        for (ordinal, record) in records.iter().enumerate() {
            poll(cancel)?;
            let candidate = hit(
                record,
                ordinal,
                squared(center, Point3::from_legacy(record.point)?),
            );
            if hits.len() < count {
                hits.push(candidate);
                let mut child = hits.len() - 1;
                while child > 0 {
                    let parent = (child - 1) / 2;
                    if order(&hits[child], &hits[parent]) != Ordering::Greater {
                        break;
                    }
                    hits.swap(child, parent);
                    child = parent;
                }
            } else if count > 0 && order(&candidate, &hits[0]) == Ordering::Less {
                hits[0] = candidate;
                sift_down(&mut hits);
            }
        }
        let bytes = hits
            .iter()
            .try_fold(0usize, |sum, h| charge(sum, h.payload.len()))?;
        if bytes > limits.max_output_bytes {
            return Err(SpatialError::BudgetExceeded);
        }
        hits.sort_unstable_by(order);
        poll(cancel)?;
        Ok(hits)
    }
}
fn admission(population: usize, limits: QueryLimits) -> Result<(), SpatialError> {
    if population
        .checked_mul(2)
        .ok_or(SpatialError::BudgetExceeded)?
        > limits.max_work
    {
        return Err(SpatialError::BudgetExceeded);
    }
    Ok(())
}
fn poll(cancel: &AtomicBool) -> Result<(), SpatialError> {
    if cancel.load(AtomicOrdering::Relaxed) {
        Err(SpatialError::Cancelled)
    } else {
        Ok(())
    }
}
fn charge(sum: usize, payload: usize) -> Result<usize, SpatialError> {
    sum.checked_add(std::mem::size_of::<CheckedSpatialHit<'_>>())
        .and_then(|v| v.checked_add(payload))
        .ok_or(SpatialError::BudgetExceeded)
}
// Defined float profile: widen before subtraction, ordinary multiply/add in
// exactly this reduction order, no FMA; preserve IEEE subnormals. All finite
// legacy f32 differences and squared sums are representable as finite f64.
pub(super) fn squared(left: Point3, right: Point3) -> f64 {
    let dx = left.x() - right.x();
    let dy = left.y() - right.y();
    let dz = left.z() - right.z();
    (dx * dx + dy * dy) + dz * dz
}
pub(super) fn hit(
    record: &SpatialRecord3D,
    insertion_ordinal: usize,
    squared_distance: f64,
) -> CheckedSpatialHit<'_> {
    hit_with_distance(
        record,
        insertion_ordinal,
        squared_distance,
        squared_distance.sqrt(),
    )
}
// Shared internal constructor: T05 supplies squared Euclidean keys. Future
// metric-qualified callers must supply their authoritative key and display
// separately; the private field name does not define their metric contract.
pub(super) fn hit_with_distance(
    record: &SpatialRecord3D,
    insertion_ordinal: usize,
    key: f64,
    distance_f64: f64,
) -> CheckedSpatialHit<'_> {
    CheckedSpatialHit {
        id: record.id,
        insertion_ordinal,
        point: record.point,
        distance_f64,
        payload: &record.payload,
        squared_distance: key,
    }
}
pub(super) fn order(a: &CheckedSpatialHit<'_>, b: &CheckedSpatialHit<'_>) -> Ordering {
    a.squared_distance
        .total_cmp(&b.squared_distance)
        .then(a.id.cmp(&b.id))
        .then(a.insertion_ordinal.cmp(&b.insertion_ordinal))
}
// In-place bounded max heap: the root is the worst retained observation.
pub(super) fn sift_down(hits: &mut [CheckedSpatialHit<'_>]) {
    let mut parent = 0usize;
    while parent < hits.len() / 2 {
        let left = parent * 2 + 1;
        let right = left + 1;
        let child = if right < hits.len() && order(&hits[right], &hits[left]) == Ordering::Greater {
            right
        } else {
            left
        };
        if order(&hits[parent], &hits[child]) != Ordering::Less {
            break;
        }
        hits.swap(parent, child);
        parent = child;
    }
}

// Authoritative metric key, distinct from rounded display distance.
pub(super) fn metric_key(hit: &CheckedSpatialHit<'_>) -> f64 {
    hit.squared_distance
}
