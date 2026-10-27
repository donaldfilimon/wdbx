#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::{orchestration_v3::*, scoring_v3::*};
use support::*;
fn seal(b: &mut Broker<'_>, h1: &CycleHandle) {
    b.close_discovery(&scope(), 1, h1).unwrap();
    b.seal(&scope(), 1, h1).unwrap();
}
#[test]
fn complete_complementary_actions_fact_and_dependency_graph_keeps_all_origins() {
    let a = Fixture::new("2 + 3", "2 + 3", "fact", 1);
    let z = Fixture::new("2 + 3", "2 + 3", "result", 2);
    let d = Fixture::new("2 + 3", "2 + 3", "other", 3);
    let mut ca = a.comparison();
    ca.kind = ContributionKind::IntermediateFact;
    let mut cz = z.comparison();
    cz.dependencies = &[8];
    let cd = d.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &[
            job(2, &cz, Priority::Foreground),
            job(8, &ca, Priority::Foreground),
            job(3, &cd, Priority::Foreground),
        ],
    )
    .unwrap();
    for l in b.dispatch(&p, 1, 3).unwrap() {
        let c = match l.job() {
            2 => &cz,
            8 => &ca,
            _ => &cd,
        };
        b.complete(&p, 1, &l, evaluate(l.job(), c)).unwrap();
    }
    seal(&mut b, &h1);
    let plan = b
        .compose(
            &p,
            1,
            &h1,
            &mut DrawTape::new(1, "compose", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    assert_eq!(
        plan.nodes
            .iter()
            .map(|n| n.contribution.id())
            .collect::<Vec<_>>(),
        vec![8, 2, 3]
    );
    assert_eq!(
        plan.nodes[0].contribution.kind(),
        ContributionKind::IntermediateFact
    );
    assert_eq!(plan.nodes[1].contribution.dependencies(), &[8]);
    assert_eq!(plan.nodes[0].contribution.origin(), &a.origin);
    assert_eq!(
        b.compose(
            &p,
            1,
            &h1,
            &mut DrawTape::new(1, "compose", &[], 0).unwrap(),
            ScoringBudget::default()
        )
        .unwrap_err(),
        Error::Incomplete
    );
}
#[test]
fn dangling_dependency_and_cycle_refuse_before_arbitration_draws() {
    for cyclic in [false, true] {
        let a = Fixture::new("2 + 3", "2 + 3", "same", 1);
        let z = Fixture::new("2 + 3", "2 + 3", "same", 2);
        let mut ca = a.comparison();
        ca.dependencies = if cyclic { &[2] } else { &[99] };
        let mut cz = z.comparison();
        if cyclic {
            cz.dependencies = &[1]
        };
        let mut b = Broker::new(scope(), 1, Limits::default()).unwrap();
        let p = scope();
        let h2 = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
        b.admit(
            &p,
            1,
            &h2,
            &[
                job(1, &ca, Priority::Foreground),
                job(2, &cz, Priority::Foreground),
            ],
        )
        .unwrap();
        for l in b.dispatch(&p, 1, 2).unwrap() {
            let c = if l.job() == 1 { &ca } else { &cz };
            b.complete(&p, 1, &l, evaluate(l.job(), c)).unwrap();
        }
        b.close_discovery(&p, 1, &h2).unwrap();
        b.seal(&p, 1, &h2).unwrap();
        let mut tape = DrawTape::new(1, "compose", &[0], 1).unwrap();
        assert_eq!(
            b.compose(&p, 1, &h2, &mut tape, ScoringBudget::default())
                .unwrap_err(),
            Error::Graph
        );
        assert_eq!(tape.consumed(), 0);
        assert_eq!(b.status(&p, 1, &h2).unwrap(), CycleStatus::Ready);
    }
}
#[test]
fn exact_redundancy_runs_once_with_original_election_lineage() {
    let a = Fixture::new("2 + 3", "2 + 3", "same", 1);
    let z = Fixture::new("2 + 3", "2 + 3", "same", 2);
    let ca = a.comparison();
    let cz = z.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &[
            job(7, &ca, Priority::Foreground),
            job(1, &cz, Priority::Foreground),
        ],
    )
    .unwrap();
    for l in b.dispatch(&p, 1, 2).unwrap() {
        b.complete(
            &p,
            1,
            &l,
            if l.job() == 7 {
                evaluate(7, &ca)
            } else {
                evaluate(1, &cz)
            },
        )
        .unwrap();
    }
    seal(&mut b, &h1);
    let mut tape = DrawTape::new(9, "arbitrate", &[u32::MAX], 1).unwrap();
    let plan = b
        .compose(&p, 1, &h1, &mut tape, ScoringBudget::default())
        .unwrap();
    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].contribution.id(), 1);
    assert_eq!(plan.arbitration[0].candidates, vec![7, 1]);
    assert_eq!(plan.arbitration[0].winner, Some(1));
    assert_eq!(tape.consumed(), 1);
}
