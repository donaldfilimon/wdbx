//! Transient checked adapter of the legacy octree; no durable generation or frame.
use super::{CheckedSpatialHit, Point3, QueryLimits, Radius, SpatialError};
use crate::spatial::{DistanceMetric, SpatialIndex3D};
use std::sync::atomic::AtomicBool;
/// Complete point selection.
#[derive(Clone, Copy, Debug)]
pub enum PointSelection {
    /// Closed radius selection.
    Radius(Radius),
    /// Nearest k observations, including a complete empty k=0 selection.
    Knn(usize),
}
/// Scoped transient index ceilings, separate from returned output bytes.
#[derive(Clone, Copy, Debug)]
pub struct OctreeBuildLimits {
    /// Maximum transient topology nodes, including empty children.
    pub max_nodes: usize,
    /// Maximum cumulative requested logical tree and partition heap bytes.
    pub max_scratch_bytes: usize,
}
/// Checked query over explicitly unitless legacy Cartesian coordinates.
#[derive(Debug)]
pub struct CheckedPointQuery<'a> {
    /// Validated center in the explicitly unitless legacy coordinate space.
    pub center: Point3,
    /// Radius enumeration or nearest selection.
    pub selection: PointSelection,
    /// Euclidean, Manhattan, or complete-scan cosine.
    pub metric: DistanceMetric,
    /// Cumulative observation work and returned result ceilings.
    pub limits: QueryLimits,
    /// Transient index ceilings independent of output bytes.
    pub build_limits: OctreeBuildLimits,
    /// Optional cooperative cancellation flag.
    pub cancellation: Option<&'a AtomicBool>,
}
/// Why a full scan was selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanReason {
    /// Population is smaller than the original 64-point threshold.
    BelowThreshold,
    /// Cosine pruning has no qualified bound.
    UnsupportedCosineBound,
    /// Transient build exceeded its scoped allocation or work ceiling.
    IndexBudget,
}
/// Actual candidate execution path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointQueryPath {
    /// Original octant partition and topology with conservative checked bounds.
    Octree,
    /// Exhaustive population scan with explicit reason.
    CompleteScan(ScanReason),
    /// Zero-k selection without population access.
    EmptySelection,
}
/// Local complete outcome; metadata makes no durable snapshot/generation claim.
#[derive(Debug)]
pub struct CheckedPointOutcome<'a> {
    /// Borrowed original observations in authoritative metric-key order.
    pub hits: Vec<CheckedSpatialHit<'a>>,
    /// Pinned arithmetic kernel identifier.
    pub numeric_profile: &'static str,
    /// Defined floating-point accuracy, without exact-real predicate claims.
    pub numeric_accuracy: &'static str,
    /// Actual execution path.
    pub path_used: PointQueryPath,
    /// Exhaustive coverage on every successful query.
    pub candidate_coverage: &'static str,
    /// Complete on every successful query; incomplete execution returns an error.
    pub completion: &'static str,
    /// Exclusive local borrow only, without durable snapshot semantics.
    pub consistency: &'static str,
    /// Metric key, numeric ID, and insertion ordinal ordering.
    pub ranking_policy_id: &'static str,
    /// Empty: this transient index has no durable generations.
    pub observed_generations: [(); 0],
    /// Cumulative admission, partition, bounds, and leaf record inspections.
    pub work_observations: usize,
    /// Absent: partial output is unsupported.
    pub partial_reason: Option<&'static str>,
    /// Absent: this adapter does not issue cursors.
    pub continuation: Option<()>,
    /// Scope and resource explanation.
    pub explanation_summary: &'static str,
    /// Nodes retained in a successful transient tree; zero on scan fallback.
    pub built_nodes: usize,
    /// Conservative cumulative logical build reservations, excluding allocator metadata.
    pub reserved_scratch_bytes: usize,
}
use super::legacy_checked::{hit_with_distance, metric_key, order, sift_down, squared};
use crate::spatial::{
    BOUNDS_EPSILON, BoundingBox, OCTREE_LEAF_CAPACITY, OCTREE_MAX_DEPTH, OCTREE_MIN_POINTS,
    SpatialRecord3D, child_bounds, octant_of,
};
use std::cmp::Ordering;
use std::sync::atomic::Ordering as AtomicOrdering;
struct Work<'a> {
    used: usize,
    max: usize,
    cancel: Option<&'a AtomicBool>,
}
impl Work<'_> {
    fn poll(&self) -> Result<(), SpatialError> {
        if self.cancel.is_some_and(|v| v.load(AtomicOrdering::Relaxed)) {
            Err(SpatialError::Cancelled)
        } else {
            Ok(())
        }
    }
    fn visit(&mut self) -> Result<(), SpatialError> {
        self.poll()?;
        if self.used >= self.max {
            return Err(SpatialError::BudgetExceeded);
        }
        self.used += 1;
        Ok(())
    }
}
struct Node {
    bounds: BoundingBox,
    children: Option<[usize; 8]>,
    indices: Vec<usize>,
}
struct Tree {
    nodes: Vec<Node>,
    bytes: usize,
    limits: OctreeBuildLimits,
}
impl Tree {
    fn charge(&mut self, bytes: usize) -> Result<(), SpatialError> {
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or(SpatialError::BudgetExceeded)?;
        if total > self.limits.max_scratch_bytes {
            Err(SpatialError::BudgetExceeded)
        } else {
            self.bytes = total;
            Ok(())
        }
    }
    fn node(
        &mut self,
        records: &[SpatialRecord3D],
        indices: Vec<usize>,
        bounds: BoundingBox,
        depth: u32,
        work: &mut Work<'_>,
    ) -> Result<usize, SpatialError> {
        work.poll()?;
        if self.nodes.len() >= self.limits.max_nodes {
            return Err(SpatialError::BudgetExceeded);
        }
        self.charge(std::mem::size_of::<Node>())?;
        self.nodes
            .try_reserve_exact(1)
            .map_err(|_| SpatialError::BudgetExceeded)?;
        let id = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            children: None,
            indices: Vec::new(),
        });
        if indices.len() <= OCTREE_LEAF_CAPACITY || depth >= OCTREE_MAX_DEPTH {
            self.nodes[id].indices = indices;
            return Ok(id);
        }
        let first = records[indices[0]].point;
        let mut coincident = true;
        let center = bounds.center();
        let mut counts = [0usize; 8];
        for &i in &indices {
            work.visit()?;
            let p = records[i].point;
            coincident &= p == first;
            counts[octant_of(p, center)] += 1;
        }
        if coincident {
            self.nodes[id].indices = indices;
            return Ok(id);
        }
        self.charge(
            indices
                .len()
                .checked_mul(std::mem::size_of::<usize>())
                .ok_or(SpatialError::BudgetExceeded)?,
        )?;
        let mut buckets: [Vec<usize>; 8] = std::array::from_fn(|_| Vec::new());
        for (bucket, count) in buckets.iter_mut().zip(counts) {
            bucket
                .try_reserve_exact(count)
                .map_err(|_| SpatialError::BudgetExceeded)?;
        }
        for &i in &indices {
            work.visit()?;
            buckets[octant_of(records[i].point, center)].push(i);
        }
        let mut children = [0; 8];
        for (octant, bucket) in buckets.into_iter().enumerate() {
            children[octant] = self.node(
                records,
                bucket,
                child_bounds(bounds, center, octant),
                depth + 1,
                work,
            )?;
        }
        self.nodes[id].children = Some(children);
        Ok(id)
    }
}
// Each box encloses its assigned represented coordinates (the legacy midpoint
// split partitions with >=). Widening f32 endpoints is exact. For an external
// query coordinate, the nearest endpoint difference has magnitude <= every
// descendant difference. Correctly rounded subtraction, nonnegative square,
// and left-to-right addition are monotone. Thus this uses exactly the point
// kernel's reduction order and is a conservative defined-float lower bound.
// Strict > pruning retains boundary hits and all ID/ordinal tie contenders.
fn bound(bounds: BoundingBox, center: Point3, metric: DistanceMetric) -> f64 {
    let lo = [bounds.min.x, bounds.min.y, bounds.min.z].map(f64::from);
    let hi = [bounds.max.x, bounds.max.y, bounds.max.z].map(f64::from);
    let q = center.coordinates();
    let d: [f64; 3] = std::array::from_fn(|i| {
        if q[i] < lo[i] {
            lo[i] - q[i]
        } else if q[i] > hi[i] {
            q[i] - hi[i]
        } else {
            0.
        }
    });
    if metric == DistanceMetric::Euclidean {
        (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]
    } else {
        (d[0] + d[1]) + d[2]
    }
}
fn key(center: Point3, point: Point3, metric: DistanceMetric) -> f64 {
    match metric {
        DistanceMetric::Euclidean => squared(center, point),
        DistanceMetric::Manhattan => {
            let a = center.coordinates();
            let b = point.coordinates();
            ((a[0] - b[0]).abs() + (a[1] - b[1]).abs()) + (a[2] - b[2]).abs()
        }
        DistanceMetric::Cosine => {
            let a = center.coordinates();
            let b = point.coordinates();
            let dot = (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2];
            let na = ((a[0] * a[0] + a[1] * a[1]) + a[2] * a[2]).sqrt();
            let nb = ((b[0] * b[0] + b[1] * b[1]) + b[2] * b[2]).sqrt();
            if na == 0. || nb == 0. {
                1.
            } else {
                1. - dot / (na * nb)
            }
        }
    }
}
struct Selection<'a> {
    hits: Vec<CheckedSpatialHit<'a>>,
    radius_output_bytes: usize,
    count: usize,
    ceiling: f64,
    radius: bool,
    metric: DistanceMetric,
    limits: QueryLimits,
}
impl<'a> Selection<'a> {
    fn consider(
        &mut self,
        record: &'a SpatialRecord3D,
        ordinal: usize,
        center: Point3,
    ) -> Result<(), SpatialError> {
        let value = key(center, Point3::from_legacy(record.point)?, self.metric);
        if self.radius && value > self.ceiling {
            return Ok(());
        }
        let hit = hit_with_distance(
            record,
            ordinal,
            value,
            if self.metric == DistanceMetric::Euclidean {
                value.sqrt()
            } else {
                value
            },
        );
        if self.radius {
            if self.hits.len() >= self.limits.max_results {
                return Err(SpatialError::BudgetExceeded);
            }
            // Radius hits cannot be discarded later: admit both their slot and
            // borrowed payload bytes before reserving or retaining the hit.
            let bytes = self
                .radius_output_bytes
                .checked_add(std::mem::size_of::<CheckedSpatialHit<'_>>())
                .and_then(|sum| sum.checked_add(record.payload.len()))
                .ok_or(SpatialError::BudgetExceeded)?;
            if bytes > self.limits.max_output_bytes {
                return Err(SpatialError::BudgetExceeded);
            }
            self.hits
                .try_reserve_exact(1)
                .map_err(|_| SpatialError::BudgetExceeded)?;
            self.hits.push(hit);
            self.radius_output_bytes = bytes;
        } else if self.hits.len() < self.count {
            self.hits.push(hit);
            let mut child = self.hits.len() - 1;
            while child > 0 {
                let parent = (child - 1) / 2;
                if order(&self.hits[child], &self.hits[parent]) != Ordering::Greater {
                    break;
                }
                self.hits.swap(child, parent);
                child = parent;
            }
        } else if self.count > 0 && order(&hit, &self.hits[0]) == Ordering::Less {
            self.hits[0] = hit;
            sift_down(&mut self.hits);
        }
        Ok(())
    }
}
fn walk<'a>(
    tree: &Tree,
    id: usize,
    records: &'a [SpatialRecord3D],
    query: &CheckedPointQuery<'_>,
    selected: &mut Selection<'a>,
    work: &mut Work<'_>,
) -> Result<(), SpatialError> {
    work.poll()?;
    let node = &tree.nodes[id];
    // Use the retained authoritative metric key, never rounded display sqrt.
    let threshold = if !selected.radius && selected.hits.len() == selected.count {
        selected.hits.first().map_or(f64::INFINITY, metric_key)
    } else if selected.radius {
        selected.ceiling
    } else {
        f64::INFINITY
    };
    if bound(node.bounds, query.center, query.metric) > threshold {
        return Ok(());
    }
    if let Some(children) = node.children {
        for child in children {
            walk(tree, child, records, query, selected, work)?;
        }
    } else {
        for &i in &node.indices {
            work.visit()?;
            selected.consider(&records[i], i, query.center)?;
        }
    }
    Ok(())
}
/// Execute with cumulative observation work (admission, build, execution),
/// cooperative per-node/per-observation cancellation and complete-or-error output.
/// The transient topology is rebuilt under the exclusive local borrow each call.
/// Scratch accounting is conservative cumulative requested bytes, including freed
/// partitions; allocator metadata is excluded. No acceleration is presumed.
pub fn checked_octree_query<'a>(
    index: &'a mut SpatialIndex3D,
    query: &CheckedPointQuery<'_>,
) -> Result<CheckedPointOutcome<'a>, SpatialError> {
    let mut work = Work {
        used: 0,
        max: query.limits.max_work,
        cancel: query.cancellation,
    };
    work.poll()?;
    let records = index.checked_records();
    let empty = matches!(query.selection, PointSelection::Knn(0));
    let mut path = PointQueryPath::EmptySelection;
    let mut tree = Tree {
        nodes: Vec::new(),
        bytes: 0,
        limits: query.build_limits,
    };
    if !empty {
        for record in records {
            work.visit()?;
            Point3::from_legacy(record.point)?;
        }
        path = if query.metric == DistanceMetric::Cosine {
            PointQueryPath::CompleteScan(ScanReason::UnsupportedCosineBound)
        } else if records.len() < OCTREE_MIN_POINTS {
            PointQueryPath::CompleteScan(ScanReason::BelowThreshold)
        } else {
            PointQueryPath::Octree
        };
        if path == PointQueryPath::Octree {
            let attempt = (|| {
                tree.charge(
                    records
                        .len()
                        .checked_mul(std::mem::size_of::<usize>())
                        .ok_or(SpatialError::BudgetExceeded)?,
                )?;
                let mut indices = Vec::new();
                indices
                    .try_reserve_exact(records.len())
                    .map_err(|_| SpatialError::BudgetExceeded)?;
                // Bounds are the existing legacy outward f32 bounds; account its inspections.
                let mut min = records[0].point;
                let mut max = min;
                for (i, r) in records.iter().enumerate() {
                    work.visit()?;
                    indices.push(i);
                    min.x = min.x.min(r.point.x);
                    min.y = min.y.min(r.point.y);
                    min.z = min.z.min(r.point.z);
                    max.x = max.x.max(r.point.x);
                    max.y = max.y.max(r.point.y);
                    max.z = max.z.max(r.point.z);
                }
                let bounds = BoundingBox {
                    min: crate::spatial::Point3D {
                        x: min.x - BOUNDS_EPSILON,
                        y: min.y - BOUNDS_EPSILON,
                        z: min.z - BOUNDS_EPSILON,
                    },
                    max: crate::spatial::Point3D {
                        x: max.x + BOUNDS_EPSILON,
                        y: max.y + BOUNDS_EPSILON,
                        z: max.z + BOUNDS_EPSILON,
                    },
                };
                tree.node(records, indices, bounds, 0, &mut work)?;
                Ok(())
            })();
            match attempt {
                Ok(()) => {}
                Err(SpatialError::BudgetExceeded) => {
                    path = PointQueryPath::CompleteScan(ScanReason::IndexBudget);
                    tree.nodes.clear();
                }
                Err(e) => return Err(e),
            }
        }
    }
    let (radius, count, ceiling) = match query.selection {
        PointSelection::Radius(r) => (
            true,
            records.len(),
            if query.metric == DistanceMetric::Euclidean {
                r.get() * r.get()
            } else {
                r.get()
            },
        ),
        PointSelection::Knn(k) => (false, k.min(records.len()), f64::INFINITY),
    };
    let mut selected = Selection {
        hits: Vec::new(),
        radius_output_bytes: 0,
        count,
        ceiling,
        radius,
        metric: query.metric,
        limits: query.limits,
    };
    if !empty {
        if !radius {
            let bytes = count
                .checked_mul(std::mem::size_of::<CheckedSpatialHit<'_>>())
                .ok_or(SpatialError::BudgetExceeded)?;
            if count > query.limits.max_results || bytes > query.limits.max_output_bytes {
                return Err(SpatialError::BudgetExceeded);
            }
            selected
                .hits
                .try_reserve_exact(count)
                .map_err(|_| SpatialError::BudgetExceeded)?;
        }
        if path == PointQueryPath::Octree {
            walk(&tree, 0, records, query, &mut selected, &mut work)?;
        } else {
            for (i, record) in records.iter().enumerate() {
                work.visit()?;
                selected.consider(record, i, query.center)?;
            }
        }
    }
    let bytes = selected.hits.iter().try_fold(0usize, |sum, h| {
        sum.checked_add(std::mem::size_of::<CheckedSpatialHit<'_>>())
            .and_then(|v| v.checked_add(h.payload.len()))
            .ok_or(SpatialError::BudgetExceeded)
    })?;
    if bytes > query.limits.max_output_bytes {
        return Err(SpatialError::BudgetExceeded);
    }
    selected.hits.sort_unstable_by(order);
    work.poll()?;
    Ok(CheckedPointOutcome {
        hits: selected.hits,
        numeric_profile: "cartesian-f64-v1/nonfma-xyz-v1",
        numeric_accuracy: "defined_float_profile",
        path_used: path,
        candidate_coverage: "exhaustive",
        completion: "complete",
        consistency: "exclusive_local_borrow",
        ranking_policy_id: "metric-key-id-insertion-ordinal-v1",
        observed_generations: [],
        work_observations: work.used,
        partial_reason: None,
        continuation: None,
        explanation_summary: "Unitless legacy population under exclusive local borrow; transient legacy-partition tree or reported complete fallback, bounded cumulative observation work and requested heap bytes.",
        built_nodes: tree.nodes.len(),
        reserved_scratch_bytes: tree.bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radius_payload_budget_rejects_before_reservation() {
        let record = SpatialRecord3D {
            id: 7,
            point: crate::spatial::Point3D::default(),
            payload: vec![3; 17],
        };
        let mut selected = Selection {
            hits: Vec::new(),
            radius_output_bytes: 0,
            count: 1,
            ceiling: 0.,
            radius: true,
            metric: DistanceMetric::Euclidean,
            limits: QueryLimits {
                max_work: 10,
                max_results: 1,
                max_output_bytes: std::mem::size_of::<CheckedSpatialHit<'_>>() + 16,
            },
        };
        assert_eq!(
            selected.consider(&record, 0, Point3::try_new(0., 0., 0.).unwrap()),
            Err(SpatialError::BudgetExceeded)
        );
        assert_eq!(selected.hits.capacity(), 0);
        assert!(selected.hits.is_empty());
    }
}
