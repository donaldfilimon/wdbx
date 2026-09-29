use specimen_kernel::search::{MAX_PLAN_CANDIDATES, MAX_RAFT_CHUNK, RaftChunk, raft_plan};

type Range = (usize, usize);

fn ranges(plan: &[RaftChunk]) -> Vec<(Range, Vec<Range>)> {
    plan.iter()
        .map(|c| ((c.start, c.end), c.rafts.clone()))
        .collect()
}

#[test]
fn empty_candidate_list_has_no_chunks() {
    let plan = raft_plan(0, 256, 8, usize::MAX).unwrap();
    assert!(plan.chunks.is_empty());
    assert_eq!((plan.checkpoints, plan.max_rafts), (0, 0));
}

#[test]
fn chunks_split_across_rafts_with_ceiling_size() {
    // 10 candidates, chunks of 4, 3 rafts: [0,4) -> 2,2 ; [4,8) -> 2,2 ; [8,10) -> 1,1
    let plan = raft_plan(10, 4, 3, usize::MAX).unwrap();
    assert_eq!(
        ranges(&plan.chunks),
        vec![
            ((0, 4), vec![(0, 2), (2, 4)]),
            ((4, 8), vec![(4, 6), (6, 8)]),
            ((8, 10), vec![(8, 9), (9, 10)]),
        ]
    );
    assert_eq!(
        (plan.chunk_size, plan.checkpoints, plan.max_rafts),
        (4, 3, 2)
    );
}

#[test]
fn more_workers_than_candidates_gives_one_per_raft() {
    assert_eq!(
        ranges(&raft_plan(3, 256, 8, usize::MAX).unwrap().chunks),
        vec![((0, 3), vec![(0, 1), (1, 2), (2, 3)])]
    );
}

#[test]
fn chunk_is_capped_at_the_shared_maximum() {
    let plan = raft_plan(5000, 10_000, 1, usize::MAX).unwrap();
    assert_eq!(MAX_RAFT_CHUNK, 4096);
    assert_eq!(plan.chunk_size, 4096);
    assert_eq!(
        ranges(&plan.chunks),
        vec![
            ((0, 4096), vec![(0, 4096)]),
            ((4096, 5000), vec![(4096, 5000)])
        ]
    );
}

#[test]
fn only_a_bounded_prefix_is_materialized() {
    let plan = raft_plan(100_000, 1, 1, 12).unwrap();
    assert_eq!(plan.checkpoints, 100_000);
    assert_eq!(plan.chunks.len(), 12);
    assert_eq!(plan.chunks[11].start, 11);
}

#[test]
fn invalid_budgets_are_rejected_like_the_search() {
    assert_eq!(raft_plan(10, 0, 1, 1).unwrap_err().code, "BudgetExceeded");
    assert_eq!(raft_plan(10, 4, 0, 1).unwrap_err().code, "BudgetExceeded");
    assert_eq!(raft_plan(10, 4, 65, 1).unwrap_err().code, "BudgetExceeded");
    assert_eq!(
        raft_plan(MAX_PLAN_CANDIDATES + 1, 4, 1, 1)
            .unwrap_err()
            .code,
        "BudgetExceeded"
    );
}
