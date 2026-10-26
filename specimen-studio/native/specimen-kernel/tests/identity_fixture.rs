//! Newly authored deterministic mechanism fixture, independent of held-out
//! corpora/oracles. A later native/WASM probe can include this module by path and
//! call `ordered_refs_fixture`; the result contains integer refs only.
use specimen_kernel::identity::*;
use specimen_kernel::language::Expr;

pub fn ordered_refs_fixture() -> Vec<u128> {
    let population = Population::new("fixture-tenant", "fixture-specimen", "pattern");
    let key = PatternKey::new("prepared-fixture-v1", "text", "low", [0xa5; 32]);
    let mut table = HybridTable::default();
    for i in 0..65 {
        let entry = EntryRecord {
            population: population.clone(),
            node: NodeRef(1 + i),
            reference: EntryRef(1001 + i),
            key: key.clone(),
            original: format!("fixture {i}"),
            ast: ActionAst {
                bind: vec![Expr::Text {
                    value: "prepared bind".into(),
                }],
                alternatives: vec![Alternative {
                    action: vec![Expr::Text {
                        value: "literal result".into(),
                    }],
                    inhibited: i % 2 == 0,
                    weight: 0.5,
                }],
            },
            resource: None,
        };
        assert!(matches!(
            table.insert(entry, None).unwrap(),
            InsertOutcome::Inserted { .. }
        ));
    }
    for (reference, link) in [(2001, Some("shared")), (2002, Some("shared")), (2003, None)] {
        table
            .insert_resource(ResourceRecord {
                population: population.clone(),
                reference: ResourceRef(reference),
                key: key.clone(),
                original: "same original resource".into(),
                link: link.map(|s| ResourceId(s.into())),
            })
            .unwrap();
    }
    let mut rebuilt = HybridTable::default();
    rebuilt.rebuild(table.snapshot()).unwrap();
    let mut refs = rebuilt
        .find_all(&population, &key)
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>();
    refs.extend(
        rebuilt
            .find_resources(&population, &key)
            .into_iter()
            .map(|r| r.0),
    );
    refs.extend(
        rebuilt
            .find_resource_links(&population, &ResourceId("shared".into()))
            .into_iter()
            .map(|r| r.0),
    );
    refs
}

#[test]
fn deterministic_fixture_retains_order_at_hash_crossover() {
    let expected = (1001..=1065)
        .chain([2001, 2002, 2003, 2001, 2002])
        .collect::<Vec<_>>();
    assert_eq!(ordered_refs_fixture(), expected);
}
