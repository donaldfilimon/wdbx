use specimen_kernel::identity::*;
use specimen_kernel::language::Expr;

fn population() -> Population {
    Population::new("tenant", "specimen", "pattern")
}
fn key() -> PatternKey {
    PatternKey::new("prepared-test-v1", "text", "low", [7; 32])
}
fn text(value: &str) -> Vec<Expr> {
    vec![Expr::Text {
        value: value.into(),
    }]
}
fn entry(n: u128, e: u128, original: &str) -> EntryRecord {
    EntryRecord {
        population: population(),
        node: NodeRef(n),
        reference: EntryRef(e),
        key: key(),
        original: original.into(),
        ast: ActionAst {
            bind: text("bind"),
            alternatives: vec![Alternative {
                action: text("answer"),
                inhibited: false,
                weight: 1.0,
            }],
        },
        resource: None,
    }
}
fn resource(reference: u128, link: Option<&str>, original: &str) -> ResourceRecord {
    ResourceRecord {
        population: population(),
        reference: ResourceRef(reference),
        key: key(),
        original: original.into(),
        link: link.map(|s| ResourceId(s.into())),
    }
}
fn inserted(table: &mut HybridTable, value: EntryRecord, spill: Option<NodeRef>) -> NodeRef {
    match table.insert(value, spill).unwrap() {
        InsertOutcome::Inserted { node, .. } => node,
        other => panic!("expected insertion, got {other:?}"),
    }
}

#[test]
fn s01_s02_collisions_retain_originals_and_concrete_nodes() {
    let mut table = HybridTable::default();
    inserted(&mut table, entry(1, 101, "hello"), None);
    inserted(&mut table, entry(2, 102, "goodbye"), None);
    assert_eq!(
        table.find_all(&population(), &key()),
        vec![EntryRef(101), EntryRef(102)]
    );
    assert_eq!(
        table
            .get(&population(), NodeRef(2), EntryRef(102))
            .unwrap()
            .original
            .as_bytes(),
        b"goodbye"
    );
    let before = table.snapshot();
    assert_eq!(
        table.remove(&population(), NodeRef(1), EntryRef(102)),
        Err(IdentityError::NodeMismatch)
    );
    assert_eq!(table.snapshot(), before);
    assert!(
        table
            .remove(&population(), NodeRef(1), EntryRef(101))
            .unwrap()
    );
    assert_eq!(table.find_all(&population(), &key()), vec![EntryRef(102)]);
}

#[test]
fn s03_capacity_plus_one_spills_atomically_with_host_reference() {
    let mut table = HybridTable::default();
    for i in 0..20 {
        inserted(&mut table, entry(1, 100 + i, &format!("p{i}")), None);
    }
    let before = table.snapshot();
    assert_eq!(
        table.insert(entry(1, 200, "next"), None).unwrap(),
        InsertOutcome::Full
    );
    assert_eq!(table.snapshot(), before);
    for collision in [NodeRef(1), NodeRef(100), NodeRef(200)] {
        assert_eq!(
            table.insert(entry(1, 200, "next"), Some(collision)),
            Err(IdentityError::DuplicateRef)
        );
        assert_eq!(table.snapshot(), before);
    }
    assert_eq!(
        inserted(&mut table, entry(1, 200, "next"), Some(NodeRef(2))),
        NodeRef(2)
    );
    assert_eq!(
        table
            .get(&population(), NodeRef(2), EntryRef(200))
            .unwrap()
            .node,
        NodeRef(2)
    );
    assert_eq!(table.snapshot().nodes.len(), 2);
    assert_eq!(table.find_all(&population(), &key()).len(), 21);
}

#[test]
fn repeats_compare_complete_structures_across_keys_and_population() {
    let mut table = HybridTable::default();
    let original = entry(1, 101, "  HéLLo\n");
    inserted(&mut table, original.clone(), None);
    let mut repeat = original.clone();
    repeat.node = NodeRef(2);
    repeat.reference = EntryRef(102);
    repeat.key.encoder_version = "other".into();
    repeat.resource = Some(ResourceId("different".into()));
    assert_eq!(
        table.insert(repeat, None).unwrap(),
        InsertOutcome::Duplicate {
            node: NodeRef(1),
            entry: EntryRef(101)
        }
    );
    assert_eq!(table.snapshot().entries.len(), 1);
    for (i, field) in ["bind", "action", "inhibition", "weight", "order", "zero"]
        .iter()
        .enumerate()
    {
        let mut changed = original.clone();
        changed.node = NodeRef(10 + i as u128);
        changed.reference = EntryRef(200 + i as u128);
        match *field {
            "bind" => changed.ast.bind = text("other bind"),
            "action" => changed.ast.alternatives[0].action = text("other action"),
            "inhibition" => changed.ast.alternatives[0].inhibited = true,
            "weight" => changed.ast.alternatives[0].weight = 0.5,
            "order" => {
                changed.ast.alternatives.push(Alternative {
                    action: text("second"),
                    inhibited: false,
                    weight: 1.0,
                });
                changed.ast.alternatives.reverse();
            }
            _ => changed.ast.alternatives[0].weight = -0.0,
        }
        inserted(&mut table, changed, None);
    }
    let mut positive_zero = original.clone();
    positive_zero.node = NodeRef(30);
    positive_zero.reference = EntryRef(230);
    positive_zero.ast.alternatives[0].weight = 0.0;
    inserted(&mut table, positive_zero, None);
    for (i, p) in [
        Population::new("other", "specimen", "pattern"),
        Population::new("tenant", "other", "pattern"),
        Population::new("tenant", "specimen", "other"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut separate = original.clone();
        separate.population = p.clone();
        separate.node = NodeRef(40 + i as u128);
        separate.reference = EntryRef(240 + i as u128);
        inserted(&mut table, separate, None);
        assert_eq!(table.find_all(&p, &key()), vec![EntryRef(240 + i as u128)]);
    }
}

#[test]
fn alternative_tree_and_slot_order_are_identity() {
    let mut table = HybridTable::default();
    let mut a = entry(1, 101, "pattern");
    a.ast.alternatives.push(Alternative {
        action: text("second"),
        inhibited: true,
        weight: 0.5,
    });
    inserted(&mut table, a.clone(), None);
    let mut b = a.clone();
    b.node = NodeRef(2);
    b.reference = EntryRef(102);
    b.ast.alternatives.reverse();
    inserted(&mut table, b, None);
    let mut c = a;
    c.node = NodeRef(3);
    c.reference = EntryRef(103);
    c.ast.bind = vec![Expr::Call {
        name: "n".into(),
        args: vec![text("x"), text("y")],
    }];
    inserted(&mut table, c.clone(), None);
    c.node = NodeRef(4);
    c.reference = EntryRef(104);
    if let Expr::Call { args, .. } = &mut c.ast.bind[0] {
        args.reverse();
    }
    inserted(&mut table, c, None);
    assert_eq!(table.find_all(&population(), &key()).len(), 4);
}

#[test]
fn s04_every_complete_key_dimension_is_used() {
    let mut table = HybridTable::default();
    let mut keys = vec![key(); 5];
    keys[1].encoder_version = "other".into();
    keys[2].modality = "visual".into();
    keys[3].resolution = "high".into();
    keys[4].digest[31] ^= 1;
    for (i, k) in keys.iter().enumerate() {
        let mut e = entry(1 + i as u128, 101 + i as u128, &format!("p{i}"));
        e.key = k.clone();
        inserted(&mut table, e, None);
    }
    for (i, k) in keys.iter().enumerate() {
        assert_eq!(
            table.find_all(&population(), k),
            vec![EntryRef(101 + i as u128)]
        );
    }
    let before = table.snapshot();
    let mut wrong = entry(1, 300, "new");
    wrong.key = keys[1].clone();
    assert_eq!(table.insert(wrong, None), Err(IdentityError::KeyMismatch));
    assert_eq!(table.snapshot(), before);
}

#[test]
fn s17_resource_links_are_nonunique_and_optional() {
    let mut table = HybridTable::default();
    for (i, link) in [Some("shared"), Some("shared"), None]
        .into_iter()
        .enumerate()
    {
        table
            .insert_resource(resource(501 + i as u128, link, &format!("r{i}")))
            .unwrap();
    }
    assert_eq!(
        table.find_resources(&population(), &key()),
        vec![ResourceRef(501), ResourceRef(502), ResourceRef(503)]
    );
    assert_eq!(
        table.find_resource_links(&population(), &ResourceId("shared".into())),
        vec![ResourceRef(501), ResourceRef(502)]
    );
    assert!(
        table
            .find_resource_links(
                &Population::new("different", "specimen", "pattern"),
                &ResourceId("shared".into())
            )
            .is_empty()
    );
    let mut e = entry(1, 101, "p");
    e.resource = Some(ResourceId("shared".into()));
    inserted(&mut table, e, None);
    assert_eq!(
        table
            .get_resource(&population(), ResourceRef(503))
            .unwrap()
            .original,
        "r2"
    );
}

#[test]
fn invalid_mutations_are_transactional_and_validate_nested_trees() {
    let mut table = HybridTable::default();
    inserted(&mut table, entry(1, 101, "existing"), None);
    let before = table.snapshot();
    for weight in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut e = entry(2, 102, "bad");
        e.ast.alternatives[0].weight = weight;
        assert_eq!(table.insert(e, None), Err(IdentityError::InvalidWeight));
        assert_eq!(table.snapshot(), before);
    }
    let mut invalid = entry(2, 102, "bad");
    invalid.ast.alternatives[0].action = vec![Expr::Call {
        name: "repeat".into(),
        args: vec![vec![Expr::Call {
            name: "unknown".into(),
            args: vec![],
        }]],
    }];
    assert_eq!(table.insert(invalid, None), Err(IdentityError::InvalidAst));
    let mut invalid = entry(2, 102, "bad");
    invalid.ast.bind = vec![Expr::Call {
        name: "calc".into(),
        args: vec![],
    }];
    assert_eq!(table.insert(invalid, None), Err(IdentityError::InvalidAst));
    let mut invalid = entry(2, 102, "bad");
    let mut nested = text("leaf");
    for _ in 0..18 {
        nested = vec![Expr::Call {
            name: "repeat".into(),
            args: vec![nested],
        }];
    }
    invalid.ast.alternatives[0].action = nested;
    assert_eq!(table.insert(invalid, None), Err(IdentityError::InvalidAst));
    let mut invalid = entry(2, 102, "bad");
    invalid.ast.alternatives[0].action = vec![Expr::Call {
        name: "literal".into(),
        args: vec![],
    }];
    assert_eq!(table.insert(invalid, None), Err(IdentityError::InvalidAst));
    let mut invalid = entry(1, 102, "bad");
    invalid.population.tenant = "other".into();
    assert_eq!(
        table.insert(invalid, None),
        Err(IdentityError::PopulationMismatch)
    );
    for (n, e) in [(2, 101), (101, 102), (2, 1), (102, 102)] {
        assert_eq!(
            table.insert(entry(n, e, "bad"), None),
            Err(IdentityError::DuplicateRef)
        );
    }
    assert_eq!(table.snapshot(), before);
}

#[test]
fn pattern_character_limit_preserves_multibyte_original_bytes() {
    let mut table = HybridTable::default();
    let original = "é".repeat(2000);
    inserted(&mut table, entry(1, 101, &original), None);
    assert_eq!(
        table
            .get(&population(), NodeRef(1), EntryRef(101))
            .unwrap()
            .original
            .as_bytes(),
        original.as_bytes()
    );
    let before = table.snapshot();
    assert_eq!(
        table.insert(entry(2, 102, &"é".repeat(2001)), None),
        Err(IdentityError::PatternTooLong)
    );
    assert_eq!(
        table.insert_resource(resource(501, None, &"é".repeat(2001))),
        Err(IdentityError::PatternTooLong)
    );
    assert_eq!(table.snapshot(), before);
}

#[test]
fn rebuild_rejects_corruption_without_replacing_current_state() {
    let mut table = HybridTable::default();
    inserted(&mut table, entry(1, 101, "existing"), None);
    table
        .insert_resource(resource(501, None, "resource"))
        .unwrap();
    let before = table.snapshot();
    let mut corruptions = vec![];
    let mut s = before.clone();
    s.entries.push(s.entries[0].clone());
    corruptions.push(s);
    let mut s = before.clone();
    s.nodes.push(s.nodes[0].clone());
    corruptions.push(s);
    let mut s = before.clone();
    s.resources.push(s.resources[0].clone());
    corruptions.push(s);
    let mut s = before.clone();
    s.entries[0].node = NodeRef(999);
    corruptions.push(s);
    let mut s = before.clone();
    s.entries[0].key.modality = "other".into();
    corruptions.push(s);
    let mut s = before.clone();
    s.entries[0].population.tenant = "other".into();
    corruptions.push(s);
    let mut s = before.clone();
    s.entries[0].ast.alternatives[0].weight = f64::NAN;
    corruptions.push(s);
    let mut s = before.clone();
    s.resources[0].reference = ResourceRef(101);
    corruptions.push(s);
    let mut s = before.clone();
    let mut duplicate = s.entries[0].clone();
    duplicate.reference = EntryRef(102);
    s.entries.push(duplicate);
    corruptions.push(s);
    let mut s = before.clone();
    for i in 0..20 {
        s.entries.push(entry(1, 200 + i, &format!("p{i}")));
    }
    corruptions.push(s);
    for s in corruptions {
        assert!(table.rebuild(s).is_err());
        assert_eq!(table.snapshot(), before);
    }
    table.rebuild(before.clone()).unwrap();
    assert_eq!(table.snapshot(), before);
}

#[test]
fn s40_array_hash_crossover_and_multiple_chunks_preserve_all_matches() {
    let mut table = HybridTable::default();
    for i in 0..2065 {
        // Distinct same-key nodes ensure later candidate chunks remain visible.
        inserted(&mut table, entry(i + 1, 10000 + i, &format!("p{i}")), None);
        if i == 63 || i == 64 || i == 2064 {
            let expected = (0..=i).map(|n| EntryRef(10000 + n)).collect::<Vec<_>>();
            assert_eq!(table.find_all(&population(), &key()), expected);
            let snapshot = table.snapshot();
            let mut rebuilt = HybridTable::default();
            rebuilt.rebuild(snapshot.clone()).unwrap();
            assert_eq!(rebuilt.find_all(&population(), &key()), expected);
            assert_eq!(rebuilt.snapshot(), snapshot);
        }
    }
    assert!(
        table
            .remove(&population(), NodeRef(65), EntryRef(10064))
            .unwrap()
    );
    assert_eq!(table.find_all(&population(), &key()).len(), 2064);
}

#[test]
fn resources_enforce_cap_and_rebuild_array_hash_equivalence() {
    let mut snapshot = TableSnapshot::default();
    for i in 0..10000 {
        snapshot.resources.push(resource(
            50000 + i,
            if i % 2 == 0 { Some("shared") } else { None },
            &format!("r{i}"),
        ));
    }
    let mut table = HybridTable::default();
    for size in [64usize, 65, 10000] {
        let s = TableSnapshot {
            resources: snapshot.resources[..size].to_vec(),
            ..Default::default()
        };
        table.rebuild(s).unwrap();
        assert_eq!(
            table.find_resources(&population(), &key()),
            (0..size)
                .map(|i| ResourceRef(50000 + i as u128))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            table
                .find_resource_links(&population(), &ResourceId("shared".into()))
                .len(),
            size.div_ceil(2)
        );
    }
    let before = table.snapshot();
    assert_eq!(
        table.insert_resource(resource(99999, None, "overflow")),
        Err(IdentityError::ResourceLimit)
    );
    assert_eq!(table.snapshot(), before);
    snapshot.resources.push(resource(99999, None, "overflow"));
    assert_eq!(table.rebuild(snapshot), Err(IdentityError::ResourceLimit));
    assert_eq!(table.snapshot(), before);
}

#[test]
fn exact_repeat_after_spill_returns_existing_ref_even_with_new_key_projection() {
    let mut table = HybridTable::default();
    for i in 0..20 {
        inserted(&mut table, entry(1, 100 + i, &format!("p{i}")), None);
    }
    let original_request = entry(1, 200, "spilled");
    inserted(&mut table, original_request.clone(), Some(NodeRef(2)));
    let before = table.snapshot();
    assert_eq!(
        table.insert(original_request, Some(NodeRef(2))).unwrap(),
        InsertOutcome::Duplicate {
            node: NodeRef(2),
            entry: EntryRef(200)
        }
    );
    let mut retry = entry(3, 200, "spilled");
    retry.key.encoder_version = "projection-v2".into();
    retry.resource = Some(ResourceId("projection".into()));
    assert_eq!(
        table.insert(retry, None).unwrap(),
        InsertOutcome::Duplicate {
            node: NodeRef(2),
            entry: EntryRef(200)
        }
    );
    assert_eq!(table.snapshot(), before);
}
