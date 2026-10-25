use specimen_kernel::{
    identity::*,
    language::Expr,
    language_v3::*,
    scanner_v3::{self, routes::*},
    scoring_v3::*,
    text_v3,
};
fn prepared(text: &str) -> Prepared {
    let tables = UtilityTables::new(
        "p",
        "v",
        UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "p".into(),
                version: "v".into(),
                kind,
                rows: vec![],
            })
            .collect(),
    )
    .unwrap();
    prepare(
        Input {
            id: "input".into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "p".into(),
            utility_version: "v".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &tables,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap()
}
struct Fixture {
    source: Prepared,
    target: Prepared,
    record: EntryRecord,
    origin: CandidateOrigin,
    binding: TargetBinding,
}
impl Fixture {
    fn new(source: &str, target: &str, action: &str) -> Self {
        let source = prepared(source);
        let target = prepared(target);
        let s = &source.clauses()[0].variants()[0];
        let t = &target.clauses()[0].variants()[0];
        let record = EntryRecord {
            population: Population::new("tenant", "specimen", "text"),
            node: NodeRef(1),
            reference: EntryRef(
                2 + action.bytes().fold(0u128, |n, b| {
                    n.wrapping_mul(257).wrapping_add(u128::from(b))
                }),
            ),
            key: text_v3::encode(t).unwrap().pattern_key().clone(),
            original: t.original().into(),
            ast: ActionAst {
                bind: vec![],
                alternatives: vec![Alternative {
                    action: vec![Expr::Text {
                        value: action.into(),
                    }],
                    inhibited: false,
                    weight: 1.0,
                }],
            },
            resource: None,
        };
        let origin = CandidateOrigin {
            origin: s.origin().clone(),
            span: s.span(),
            token_spans: s.tokens().iter().map(Token::span).collect(),
            constraints: s.into(),
            path: CandidatePath::Wildcard("synthetic".into()),
            captures: vec![],
            bypass: SimilarityBypass::RequireFullScan,
        };
        let binding = TargetBinding {
            node: record.node,
            entry: record.reference,
            origin: t.origin().clone(),
        };
        Self {
            source,
            target,
            record,
            origin,
            binding,
        }
    }
    fn request(&self, id: u128) -> EntryRequest<'_> {
        EntryRequest {
            id,
            source: &self.source.clauses()[0].variants()[0],
            target: &self.target.clauses()[0].variants()[0],
            record: &self.record,
            origin: &self.origin,
            target_binding: &self.binding,
            parent_lineage: "parent",
            inhibitions: &[],
            dependencies: &[],
            attachment_group: None,
            kind: ContributionKind::Action,
            policy: Policy {
                jitter: false,
                ..Policy::default()
            },
        }
    }
    fn contribution(&self, id: u128) -> Contribution {
        evaluate_entry(
            self.request(id),
            &mut DrawTape::new(5, "fixture", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap()
        .contribution
        .unwrap()
    }
}
#[test]
fn s09_every_qualified_entry_in_a_node_can_contribute() {
    let a = Fixture::new("2 + 3", "2 + 3", "first");
    let mut b = Fixture::new("2 + 3", "2 + 3", "second");
    b.record.reference = EntryRef(3);
    b.binding.entry = EntryRef(3);
    let a = a.contribution(1);
    let b = b.contribution(2);
    assert_eq!(a.record().node, b.record().node);
    assert_ne!(a.record().reference, b.record().reference);
    assert_eq!(
        a.score().effective,
        scanner_v3::Rational::new(60, 1).unwrap()
    );
}
#[test]
fn stale_explicit_binding_refuses_before_jitter() {
    let mut f = Fixture::new("2 + 3", "2 + 3", "a");
    f.binding.entry = EntryRef(9);
    let mut tape = DrawTape::new(0, "stale", &[u32::MAX, 1], 2).unwrap();
    assert!(matches!(
        evaluate_entry(f.request(1), &mut tape, ScoringBudget::default()),
        Err(ScoringError::BindingMismatch)
    ));
    assert_eq!(tape.consumed(), 0);
}
#[test]
fn s11_actual_redundancy_elects_only_within_its_group() {
    let a = Fixture::new("2 + 3", "2 + 3", "same");
    let b = Fixture::new("2 + 3", "2 + 3", "same");
    let c = Fixture::new("2 + 3", "2 + 3", "complement");
    let values = [u32::MAX];
    let mut t = DrawTape::new(9, "group", &values, 1).unwrap();
    let out = arbitrate(
        &[a.contribution(1), b.contribution(2), c.contribution(3)],
        &mut t,
        ScoringBudget::default(),
    )
    .unwrap();
    assert_eq!(
        out.contributions
            .iter()
            .map(Contribution::id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(out.groups[0].candidates, vec![1, 2]);
    assert_eq!(out.groups[0].winner, Some(2));
}
#[test]
fn s10_complementary_entries_survive_without_election() {
    let a = Fixture::new("2 + 3", "2 + 3", "a");
    let b = Fixture::new("2 + 3", "2 + 3", "b");
    let mut t = DrawTape::new(0, "complement", &[], 0).unwrap();
    let out = arbitrate(
        &[a.contribution(1), b.contribution(2)],
        &mut t,
        ScoringBudget::default(),
    )
    .unwrap();
    assert_eq!(out.contributions.len(), 2);
    assert!(out.groups.is_empty());
}
#[test]
fn different_parent_captures_inhibition_and_tree_are_complementary() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let base = f.contribution(1);
    let mut parent = f.request(2);
    parent.parent_lineage = "independent";
    let parent = evaluate_entry(
        parent,
        &mut DrawTape::new(0, "parent", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .contribution
    .unwrap();
    let mut capture = Fixture::new("2 + 3", "2 + 3", "a");
    capture.origin.captures.push(RouteCapture {
        name: "operand".into(),
        span: Span { start: 0, end: 1 },
        value: CaptureValue::Number(CanonicalDecimal::parse("2").unwrap()),
    });
    let capture = capture.contribution(3);
    let mut inhibition = f.request(4);
    let facts = [InhibitionFact {
        domain: InhibitionDomain::Target,
        code: "retain".into(),
    }];
    inhibition.inhibitions = &facts;
    let inhibition = evaluate_entry(
        inhibition,
        &mut DrawTape::new(0, "facts", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .contribution
    .unwrap();
    let mut tree = Fixture::new("2 + 3", "2 + 3", "a");
    tree.record.reference = EntryRef(808);
    tree.binding.entry = EntryRef(808);
    tree.record.ast.alternatives[0].action = vec![Expr::Call {
        name: "a".into(),
        args: vec![],
    }];
    let tree = tree.contribution(5);
    let mut t = DrawTape::new(0, "complement", &[], 0).unwrap();
    let out = arbitrate(
        &[base, parent, capture, inhibition, tree],
        &mut t,
        ScoringBudget::default(),
    )
    .unwrap();
    assert_eq!(out.contributions.len(), 5);
    assert!(out.groups.is_empty());
}
#[test]
fn attached_facts_and_referenced_dependencies_are_exempt() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let mut requests = [
        f.request(1),
        f.request(2),
        f.request(3),
        f.request(4),
        f.request(5),
    ];
    requests[1].attachment_group = Some(9);
    requests[2].kind = ContributionKind::IntermediateFact;
    let deps = [1];
    requests[3].dependencies = &deps;
    let mut tape = DrawTape::new(0, "exempt", &[], 0).unwrap();
    let values = evaluate_entries(&requests, &mut tape, ScoringBudget::default())
        .unwrap()
        .into_iter()
        .map(|o| o.contribution.unwrap())
        .collect::<Vec<_>>();
    let out = arbitrate(&values, &mut tape, ScoringBudget::default()).unwrap();
    assert_eq!(out.contributions.len(), 5);
    assert!(out.groups.is_empty());
}
#[test]
fn zero_confidence_redundancy_abstains_without_forging_bypass_confidence() {
    let mut f = Fixture::new("alpha", "beta", "a");
    f.origin.bypass = SimilarityBypass::Declared;
    let a = f.contribution(1);
    let b = f.contribution(2);
    assert!(a.score().effective.numerator() < 0);
    assert!(a.score().qualified);
    let mut t = DrawTape::new(0, "abstain", &[], 0).unwrap();
    let out = arbitrate(&[a, b], &mut t, ScoringBudget::default()).unwrap();
    assert!(out.contributions.is_empty());
    assert_eq!(out.groups[0].election.weights, vec![0.0, 0.0]);
    assert_eq!(out.groups[0].winner, None);
    assert_eq!(t.consumed(), 0);
}
#[test]
fn guards_and_blocked_automaton_never_become_successful_bypasses() {
    for (source, target, reason) in [
        ("not 2 + 3", "2 + 3", Refusal::InputInhibited),
        ("2 + 3", "not 2 + 3", Refusal::TargetInhibited),
        ("if rain then 2 + 3", "2 + 3", Refusal::Unresolved),
    ] {
        let mut f = Fixture::new(source, target, "a");
        f.origin.bypass = SimilarityBypass::Declared;
        let mut t = DrawTape::new(0, "guard", &[], 0).unwrap();
        let out = evaluate_entry(f.request(1), &mut t, ScoringBudget::default()).unwrap();
        assert!(out.contribution.is_none());
        assert!(out.score.refusals.contains(&reason));
    }
    let mut f = Fixture::new("2 + 3", "2 + 3", "a");
    f.origin.path = CandidatePath::Automaton {
        rule: "blocked".into(),
        status: AutomatonStatus::Blocked,
    };
    let mut t = DrawTape::new(0, "blocked", &[], 0).unwrap();
    let out = evaluate_entry(f.request(1), &mut t, ScoringBudget::default()).unwrap();
    assert!(out.contribution.is_none());
    assert!(out.score.refusals.contains(&Refusal::BlockedAutomaton));
    f.origin.bypass = SimilarityBypass::Declared;
    assert!(matches!(
        evaluate_entry(f.request(1), &mut t, ScoringBudget::default()),
        Err(ScoringError::BindingMismatch)
    ));
}
#[test]
fn late_invalid_batch_and_arbitration_references_leave_draws_untouched() {
    let good = Fixture::new("2 + 3", "2 + 3", "a");
    let mut bad = Fixture::new("2 + 3", "2 + 3", "a");
    bad.record.ast.alternatives.push(Alternative {
        action: vec![],
        inhibited: false,
        weight: f64::NAN,
    });
    let mut requests = [good.request(1), bad.request(2)];
    requests[0].policy.jitter = true;
    let mut t = DrawTape::new(0, "atomic", &[u32::MAX, 1], 2).unwrap();
    assert!(matches!(
        evaluate_entries(&requests, &mut t, ScoringBudget::default()),
        Err(ScoringError::InvalidWeight)
    ));
    assert_eq!(t.consumed(), 0);
    let a = good.contribution(1);
    assert!(matches!(
        arbitrate(&[a.clone(), a], &mut t, ScoringBudget::default()),
        Err(ScoringError::InvalidContribution)
    ));
    assert_eq!(t.consumed(), 0);
    let mut request = good.request(3);
    let deps = [999];
    request.dependencies = &deps;
    let value = evaluate_entry(request, &mut t, ScoringBudget::default())
        .unwrap()
        .contribution
        .unwrap();
    assert!(matches!(
        arbitrate(
            &[good.contribution(1), value],
            &mut t,
            ScoringBudget::default()
        ),
        Err(ScoringError::InvalidContribution)
    ));
    assert_eq!(t.consumed(), 0);
}
#[test]
fn oversized_action_vectors_and_payloads_refuse_before_copy_or_draw() {
    let mut f = Fixture::new("2 + 3", "2 + 3", "a");
    f.record.ast.alternatives[0].action = vec![
        Expr::Text {
            value: String::new()
        };
        100_001
    ];
    let mut t = DrawTape::new(0, "budget", &[u32::MAX, 1], 2).unwrap();
    assert!(matches!(
        evaluate_entry(f.request(1), &mut t, ScoringBudget::default()),
        Err(ScoringError::BudgetExceeded)
    ));
    assert_eq!(t.consumed(), 0);
    f.record.ast.alternatives[0].action = vec![Expr::Text {
        value: "x".repeat(8001),
    }];
    assert!(matches!(
        evaluate_entry(f.request(1), &mut t, ScoringBudget::default()),
        Err(ScoringError::BudgetExceeded)
    ));
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    for budget in [
        ScoringBudget {
            work: 1,
            ..ScoringBudget::default()
        },
        ScoringBudget {
            payload_bytes: 1,
            ..ScoringBudget::default()
        },
        ScoringBudget {
            contributions: 0,
            ..ScoringBudget::default()
        },
    ] {
        assert!(matches!(
            evaluate_entry(f.request(1), &mut t, budget),
            Err(ScoringError::BudgetExceeded)
        ));
    }
    assert_eq!(t.consumed(), 0);
}
#[test]
fn supplied_evidence_is_checked_but_cannot_construct_contributions() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let original = scanner_v3::scan(f.request(1).source, f.request(1).target).unwrap();
    assert!(validate_evidence(&original).is_ok());
    let mut versions = original.clone();
    versions.version = "old";
    assert_eq!(
        validate_evidence(&versions),
        Err(ScoringError::InvalidEvidence)
    );
    let mut totals = original.clone();
    totals.totals.confidence = scanner_v3::Rational::new(100, 1).unwrap();
    assert_eq!(
        validate_evidence(&totals),
        Err(ScoringError::InvalidEvidence)
    );
    let mut feature = original.clone();
    feature.components[0].weight = 100;
    assert_eq!(
        validate_evidence(&feature),
        Err(ScoringError::InvalidEvidence)
    );
    let mut guard = original.clone();
    guard.left_constraints.inhibited = true;
    assert_eq!(
        validate_evidence(&guard),
        Err(ScoringError::InvalidEvidence)
    );
    let mut fraction = original;
    fraction.components[0].similarity = scanner_v3::Rational::new(-1, 2).unwrap();
    assert_eq!(
        validate_evidence(&fraction),
        Err(ScoringError::InvalidEvidence)
    );
}
#[test]
fn source_guard_mismatch_refuses_and_matching_inhibited_alternatives_cannot_emit() {
    let mut f = Fixture::new("2 + 3", "2 + 3", "a");
    f.origin.constraints.executable = false;
    let mut t = DrawTape::new(0, "mismatch", &[], 0).unwrap();
    assert!(matches!(
        evaluate_entry(f.request(1), &mut t, ScoringBudget::default()),
        Err(ScoringError::InvalidEvidence)
    ));
    f.origin.constraints.executable = true;
    f.record.ast.alternatives[0].inhibited = true;
    let out = evaluate_entry(f.request(1), &mut t, ScoringBudget::default()).unwrap();
    assert!(out.contribution.is_none());
    assert_eq!(out.score.refusals, vec![Refusal::NoAlternative]);
    assert_eq!(out.election.unwrap().refused, vec![0]);
}
#[test]
fn sorted_inhibition_sets_ignore_order_but_keep_input_target_codes_distinct() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let input = InhibitionFact {
        domain: InhibitionDomain::Input,
        code: "same-code".into(),
    };
    let target = InhibitionFact {
        domain: InhibitionDomain::Target,
        code: "same-code".into(),
    };
    let first = [input.clone(), target.clone()];
    let second = [target, input.clone()];
    let third = [input];
    let mut requests = [f.request(1), f.request(2), f.request(3)];
    requests[0].inhibitions = &first;
    requests[1].inhibitions = &second;
    requests[2].inhibitions = &third;
    let values = evaluate_entries(
        &requests,
        &mut DrawTape::new(0, "metadata", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .into_iter()
    .map(|o| o.contribution.unwrap())
    .collect::<Vec<_>>();
    assert_eq!(values[1].inhibitions(), &second);
    let mut t = DrawTape::new(0, "groups", &[u32::MAX], 1).unwrap();
    let out = arbitrate(&values, &mut t, ScoringBudget::default()).unwrap();
    assert_eq!(
        out.contributions
            .iter()
            .map(Contribution::id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}
#[test]
fn named_typed_capture_values_are_ordered_and_spans_are_only_provenance() {
    let mut a = Fixture::new("2 + 3", "2 + 3", "a");
    let capture = |name: &str, span: Span, value: CaptureValue| RouteCapture {
        name: name.into(),
        span,
        value,
    };
    a.origin.captures = vec![capture(
        "x",
        Span { start: 0, end: 1 },
        CaptureValue::Text("2".into()),
    )];
    let mut b = Fixture::new("2 + 3", "2 + 3", "a");
    b.origin.captures = vec![capture(
        "x",
        Span { start: 4, end: 5 },
        CaptureValue::Text("2".into()),
    )];
    let mut c = Fixture::new("2 + 3", "2 + 3", "a");
    c.origin.captures = vec![capture(
        "x",
        Span { start: 0, end: 1 },
        CaptureValue::Number(CanonicalDecimal::parse("2").unwrap()),
    )];
    let values = [a.contribution(1), b.contribution(2), c.contribution(3)];
    let mut t = DrawTape::new(0, "capture", &[u32::MAX], 1).unwrap();
    let out = arbitrate(&values, &mut t, ScoringBudget::default()).unwrap();
    assert_eq!(
        out.contributions
            .iter()
            .map(Contribution::id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(
        out.contributions[0].origin().captures[0].span,
        Span { start: 4, end: 5 }
    );
}
#[test]
fn distinct_dependency_sets_remain_complementary() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let mut requests = [f.request(1), f.request(2), f.request(3), f.request(4)];
    let d1 = [1];
    let d2 = [2];
    requests[2].dependencies = &d1;
    requests[3].dependencies = &d2;
    let values = evaluate_entries(
        &requests,
        &mut DrawTape::new(0, "metadata", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .into_iter()
    .map(|o| o.contribution.unwrap())
    .collect::<Vec<_>>();
    let mut t = DrawTape::new(0, "deps", &[], 0).unwrap();
    let out = arbitrate(&values, &mut t, ScoringBudget::default()).unwrap();
    assert_eq!(out.contributions.len(), 4);
    assert!(out.groups.is_empty());
}
#[test]
fn lowering_arbitration_budget_refuses_without_partial_draws() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let values = [f.contribution(1), f.contribution(2)];
    let mut t = DrawTape::new(0, "limited", &[0], 1).unwrap();
    assert!(matches!(
        arbitrate(
            &values,
            &mut t,
            ScoringBudget {
                draws: 0,
                ..ScoringBudget::default()
            }
        ),
        Err(ScoringError::BudgetExceeded)
    ));
    assert_eq!(t.consumed(), 0);
    assert!(matches!(
        arbitrate(
            &values,
            &mut t,
            ScoringBudget {
                payload_bytes: 1,
                ..ScoringBudget::default()
            }
        ),
        Err(ScoringError::BudgetExceeded)
    ));
    assert_eq!(t.consumed(), 0);
}
#[test]
fn fanout_siblings_share_root_but_keep_distinct_origin_provenance() {
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "p".into(),
            version: "v".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    snapshots
        .iter_mut()
        .find(|s| s.kind == UtilityKind::Thesaurus)
        .unwrap()
        .rows
        .push(UtilityRow {
            reference: "fanout".into(),
            phrase: "bright".into(),
            effect: UtilityEffect::Equivalent {
                replacement: "radiant".into(),
                weight: 1.0,
            },
        });
    let tables = UtilityTables::new("p", "v", snapshots).unwrap();
    let source = prepare(
        Input {
            id: "input".into(),
            text: "calculate bright 2 + 3".into(),
        },
        &ContextSnapshot {
            population: "p".into(),
            utility_version: "v".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &tables,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert!(source.clauses()[0].variants().len() > 1);
    let f = Fixture::new("calculate bright 2 + 3", "calculate bright 2 + 3", "same");
    let mut origins = vec![];
    for s in source.clauses()[0].variants().iter().take(2) {
        origins.push(CandidateOrigin {
            origin: s.origin().clone(),
            span: s.span(),
            token_spans: s.tokens().iter().map(Token::span).collect(),
            constraints: s.into(),
            path: CandidatePath::Wildcard("fanout-route".into()),
            captures: vec![],
            bypass: SimilarityBypass::Declared,
        });
    }
    let mut requests = [f.request(1), f.request(2)];
    for (i, r) in requests.iter_mut().enumerate() {
        r.source = &source.clauses()[0].variants()[i];
        r.origin = &origins[i];
    }
    let values = evaluate_entries(
        &requests,
        &mut DrawTape::new(0, "fanout-eval", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .into_iter()
    .map(|o| o.contribution.unwrap())
    .collect::<Vec<_>>();
    assert_ne!(
        values[0].origin().origin.fanout_id(),
        values[1].origin().origin.fanout_id()
    );
    let mut t = DrawTape::new(0, "fanout", &[0], 1).unwrap();
    let out = arbitrate(&values, &mut t, ScoringBudget::default()).unwrap();
    assert_eq!(out.groups.len(), 1);
    assert_eq!(out.groups[0].candidates, vec![1, 2]);
    assert_eq!(out.contributions.len(), 1);
}
#[test]
fn external_evidence_spans_are_bounded() {
    let f = Fixture::new("2 + 3", "2 + 3", "a");
    let mut evidence = scanner_v3::scan(f.request(1).source, f.request(1).target).unwrap();
    evidence.components[0].left_spans = vec![Span { start: 0, end: 1 }; 1_000_001];
    assert_eq!(
        validate_evidence(&evidence),
        Err(ScoringError::BudgetExceeded)
    );
}
#[test]
fn canonical_actions_exclude_node_identity_and_election_weight() {
    let a = Fixture::new("2 + 3", "2 + 3", "same");
    let mut b = Fixture::new("2 + 3", "2 + 3", "same");
    b.record.node = NodeRef(99);
    b.binding.node = NodeRef(99);
    b.record.reference = EntryRef(99);
    b.binding.entry = EntryRef(99);
    b.record.ast.alternatives[0].weight = 999.0;
    let values = [a.contribution(1), b.contribution(2)];
    let mut tape = DrawTape::new(0, "canonical", &[u32::MAX], 1).unwrap();
    let out = arbitrate(&values, &mut tape, ScoringBudget::default()).unwrap();
    assert_eq!(out.groups.len(), 1);
    assert_eq!(out.contributions[0].id(), 2);
    assert_eq!(out.contributions[0].record().node, NodeRef(99));
    assert_eq!(out.groups[0].election.weights, vec![60.0, 60.0]);
}
#[test]
fn threshold_bypass_does_not_bypass_zero_strength_or_action_inhibition() {
    let mut f = Fixture::new("alpha", "beta", "a");
    f.origin.bypass = SimilarityBypass::Declared;
    let mut request = f.request(1);
    request.policy.strength = 0;
    let mut tape = DrawTape::new(0, "zero-strength", &[], 0).unwrap();
    let out = evaluate_entry(request, &mut tape, ScoringBudget::default()).unwrap();
    assert!(out.contribution.is_none());
    assert_eq!(out.score.refusals, vec![Refusal::ZeroStrength]);
    f.record.ast.alternatives[0].inhibited = true;
    let out = evaluate_entry(f.request(1), &mut tape, ScoringBudget::default()).unwrap();
    assert!(out.contribution.is_none());
    assert_eq!(out.score.refusals, vec![Refusal::NoAlternative]);
}
#[test]
fn deep_and_many_argument_trees_fail_before_cloning() {
    let mut f = Fixture::new("2 + 3", "2 + 3", "a");
    let mut tree = vec![Expr::Text { value: "a".into() }];
    for _ in 0..16 {
        tree = vec![Expr::Call {
            name: "call".into(),
            args: vec![tree],
        }];
    }
    f.record.ast.alternatives[0].action = tree;
    let mut tape = DrawTape::new(0, "deep", &[], 0).unwrap();
    assert!(matches!(
        evaluate_entry(f.request(1), &mut tape, ScoringBudget::default()),
        Err(ScoringError::BudgetExceeded)
    ));
    f.record.ast.alternatives[0].action = vec![Expr::Call {
        name: "wide".into(),
        args: vec![vec![]; 100_001],
    }];
    assert!(matches!(
        evaluate_entry(f.request(1), &mut tape, ScoringBudget::default()),
        Err(ScoringError::BudgetExceeded)
    ));
}

#[test]
fn conflicting_concrete_entry_snapshots_refuse_atomically() {
    let a = Fixture::new("2 + 3", "2 + 3", "a");
    let mut b = Fixture::new("2 + 3", "2 + 3", "b");
    b.record.reference = a.record.reference;
    b.binding.entry = a.record.reference;
    let values = [a.contribution(1), b.contribution(2)];
    let mut tape = DrawTape::new(0, "conflicting-records", &[0], 1).unwrap();
    assert!(matches!(
        arbitrate(&values, &mut tape, ScoringBudget::default()),
        Err(ScoringError::InvalidContribution)
    ));
    assert_eq!(tape.consumed(), 0);
}
