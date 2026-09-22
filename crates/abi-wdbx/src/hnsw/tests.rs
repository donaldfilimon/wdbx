//! Focused HNSW index tests.

use super::*;
use crate::ExactIndex;

fn vector(index: u16, total: u16) -> [f32; 4] {
    let angle = f32::from(index) * std::f32::consts::TAU / f32::from(total);
    [angle.cos(), angle.sin(), (angle * 0.5).cos(), 1.0]
}

#[test]
fn vector_storage_supports_sparse_ids_without_zero_filled_presence() {
    let mut storage = VectorStorage::new(3).expect("storage");
    storage.insert(5, &[1.0, 2.0, 3.0]).expect("insert");
    assert_eq!(storage.get(5), Some([1.0, 2.0, 3.0].as_slice()));
    assert_eq!(storage.get(0), None);
    assert!(!storage.contains(99));
}

#[test]
fn dimensions_are_validated_at_construction_and_use() {
    assert!(matches!(
        HnswIndex::new(0),
        Err(HnswError::InvalidDimensions { dimensions: 0 })
    ));
    let mut index = HnswIndex::new(4).expect("index");
    assert_eq!(
        index.insert(1, &[1.0, 0.0]),
        Err(HnswError::DimensionMismatch {
            expected: 4,
            found: 2,
        })
    );
    assert_eq!(
        index.search(&[1.0], 1),
        Err(HnswError::DimensionMismatch {
            expected: 4,
            found: 1,
        })
    );
}

#[test]
fn insert_get_and_duplicate_rejection_are_consistent() {
    let mut index = HnswIndex::new(2).expect("index");
    index.insert(7, &[1.0, 2.0]).expect("insert");
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(7), Some([1.0, 2.0].as_slice()));
    assert_eq!(
        index.insert(7, &[2.0, 1.0]),
        Err(HnswError::DuplicateId { id: 7 })
    );
}

#[test]
fn empty_and_zero_limit_searches_return_no_results() {
    let mut index = HnswIndex::new(2).expect("index");
    assert_eq!(index.search(&[1.0, 0.0], 3).expect("empty").len(), 0);
    index.insert(1, &[1.0, 0.0]).expect("insert");
    assert_eq!(index.search(&[1.0, 0.0], 0).expect("zero").len(), 0);
}

#[test]
fn inserted_vectors_are_found_as_their_own_nearest_neighbor() {
    let mut hnsw = HnswIndex::new(4).expect("hnsw");
    let mut exact = ExactIndex::new();
    for id in 1_u16..=96 {
        let values = vector(id - 1, 96);
        hnsw.insert(u64::from(id), &values).expect("hnsw insert");
        hnsw.commit_last_insert(u64::from(id));
        assert_eq!(
            exact.put_vector(&values).expect("exact insert"),
            u64::from(id)
        );
    }

    for id in (1_u16..=96).step_by(3) {
        let query = vector(id - 1, 96);
        let approximate = hnsw.search(&query, 1).expect("hnsw search");
        let reference = exact.search(&query, 1).expect("exact search");
        assert_eq!(approximate[0].id, reference[0].id);
        assert!((approximate[0].score - reference[0].score).abs() < 0.0001);
    }
}

#[test]
fn search_results_are_score_sorted_and_borrow_storage() {
    let mut index = HnswIndex::new(4).expect("index");
    index.insert(1, &[1.0, 0.0, 0.0, 0.0]).expect("one");
    index.insert(2, &[0.9, 0.1, 0.0, 0.0]).expect("two");
    index.insert(3, &[0.0, 1.0, 0.0, 0.0]).expect("three");
    let stored = index.get(1).expect("stored");
    let results = index.search(&[1.0, 0.0, 0.0, 0.0], 3).expect("search");
    assert_eq!(results[0].id, 1);
    assert!(
        results
            .windows(2)
            .all(|pair| pair[0].score >= pair[1].score)
    );
    assert_eq!(results[0].vector.as_ptr(), stored.as_ptr());
}

#[test]
fn graph_invariants_hold_after_neighbor_pruning() {
    let mut index = HnswIndex::with_seed(4, 42).expect("index");
    for id in 1_u16..=256 {
        index
            .insert(u64::from(id), &vector(id - 1, 256))
            .expect("insert");
        index.commit_last_insert(u64::from(id));
        index.validate_graph().expect("valid graph");
    }
}

#[test]
fn rollback_restores_the_complete_prior_graph() {
    let mut index = HnswIndex::with_seed(4, 7).expect("index");
    for id in 1_u16..=80 {
        index
            .insert(u64::from(id), &vector(id - 1, 81))
            .expect("insert");
        index.commit_last_insert(u64::from(id));
    }
    let prior = index.graph_signature();

    index.insert(81, &vector(80, 81)).expect("last insert");
    assert!(index.rollback_last_insert(81));
    assert_eq!(index.graph_signature(), prior);
    assert!(!index.storage.contains(81));
    index.validate_graph().expect("restored graph");
}

#[test]
fn rollback_is_scoped_to_the_matching_newest_insert() {
    let mut index = HnswIndex::new(2).expect("index");
    index.insert(1, &[1.0, 0.0]).expect("first");
    index.commit_last_insert(1);
    index.insert(2, &[0.0, 1.0]).expect("second");
    assert!(!index.rollback_last_insert(1));
    assert_eq!(index.len(), 2);
    assert!(index.rollback_last_insert(2));
    assert_eq!(index.len(), 1);
}

#[test]
fn identical_seed_and_insertion_order_produce_identical_graphs() {
    let mut left = HnswIndex::with_seed(4, 123).expect("left");
    let mut right = HnswIndex::with_seed(4, 123).expect("right");
    for id in 1_u16..=64 {
        let values = vector(id - 1, 64);
        left.insert(u64::from(id), &values).expect("left insert");
        right.insert(u64::from(id), &values).expect("right insert");
    }
    assert_eq!(left.graph_signature(), right.graph_signature());
}

#[test]
fn snapshot_rebuild_preserves_ids_and_vectors() {
    let mut snapshot = Snapshot::new();
    snapshot.vectors.insert(4, vec![1.0, 0.0]);
    snapshot.vectors.insert(9, vec![0.0, 1.0]);
    let index = HnswIndex::from_snapshot(&snapshot).expect("rebuild");
    assert_eq!(index.len(), 2);
    assert_eq!(index.get(4), Some([1.0, 0.0].as_slice()));
    assert_eq!(index.get(9), Some([0.0, 1.0].as_slice()));
    index.validate_graph().expect("valid graph");
}
