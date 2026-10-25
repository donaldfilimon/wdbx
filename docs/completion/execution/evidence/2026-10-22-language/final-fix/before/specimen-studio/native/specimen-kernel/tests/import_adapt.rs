use specimen_kernel::{host::FixedHost, import_adapt, mutate};

fn host() -> FixedHost {
    FixedHost::default()
}

#[test]
fn network_for_import_is_a_valid_decoder_and_repeats_for_the_same_seed() {
    for nodes in [1, 9, 24] {
        let net = import_adapt::network_for_import(nodes, 7).unwrap();
        net.validate().unwrap();
        assert_eq!(net.layers[0].inputs, 128);
        assert_eq!(net.layers.last().unwrap().outputs, 32);
        assert_eq!(net, import_adapt::network_for_import(nodes, 7).unwrap());
    }
    assert_ne!(
        import_adapt::network_for_import(4, 7).unwrap(),
        import_adapt::network_for_import(4, 8).unwrap()
    );
}

#[test]
fn markdown_becomes_a_node_and_a_valid_network() {
    let fixed = host();
    let adapted =
        import_adapt::adapt_import(&fixed.host(), "# Color\n\nTeal.\n", "notes.md").unwrap();
    assert!(!adapted.specimen["nodes"].as_array().unwrap().is_empty());
    let network = adapted.network.expect("prose import derives a network");
    network.validate().unwrap();
    assert_eq!(network.layers[0].inputs, 128);
    assert_eq!(network.layers.last().unwrap().outputs, 32);
}

#[test]
fn a_wrong_schema_is_not_a_supported_specimen() {
    let fixed = host();
    let err =
        import_adapt::adapt_import(&fixed.host(), r#"{"schema":"broken"}"#, "x.json").unwrap_err();
    assert!(err.message.contains("not a supported"), "{}", err.message);
}

#[test]
fn a_valid_seed_round_trips_as_a_specimen_and_keeps_its_name() {
    let fixed = host();
    let seeded = mutate::seed(&fixed.host());
    let text = serde_json::to_string(&seeded).unwrap();
    let adapted = import_adapt::adapt_import(&fixed.host(), &text, "ignored.json").unwrap();
    assert_eq!(adapted.kind, "specimen");
    assert_eq!(adapted.specimen["name"], "Specimen 001");
    assert_eq!(adapted.specimen["name"], seeded["name"]);
    assert!(adapted.network.is_none());
}

#[test]
fn an_empty_file_errors() {
    let fixed = host();
    let err = import_adapt::adapt_import(&fixed.host(), "   \n", "empty.txt").unwrap_err();
    assert_eq!(err.message, "This file is empty.");
}
