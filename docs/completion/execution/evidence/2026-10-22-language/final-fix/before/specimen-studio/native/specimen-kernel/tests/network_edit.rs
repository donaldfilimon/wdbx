use specimen_kernel::network_edit::{NetworkCommand, apply, fan_in};
use specimen_kernel::neural::{Network, encode};

fn cmd(json: &str) -> NetworkCommand {
    serde_json::from_str(json).unwrap()
}

#[test]
fn add_layer_inserts_a_hidden_layer_and_rewires_the_displaced_one() {
    let base = Network::default(); // 128 -> 64 -> 32
    let next = apply(
        &base,
        cmd(r#"{"command":"addLayer","at":1,"outputs":48,"fanIn":6,"seed":7}"#),
    )
    .unwrap();
    let widths: Vec<_> = next.layers.iter().map(|l| (l.inputs, l.outputs)).collect();
    assert_eq!(widths, vec![(128, 64), (64, 48), (48, 32)]);
    assert_eq!(fan_in(&next.layers[1]), 6);
    // The displaced decoder keeps its fan-in (8) but now reads 48 inputs.
    assert_eq!(fan_in(&next.layers[2]), 8);
    assert!(next.layers[2].columns.iter().all(|&c| c < 48));
    next.validate().unwrap();
    // Deterministic for a seed.
    let again = apply(
        &base,
        cmd(r#"{"command":"addLayer","at":1,"outputs":48,"fanIn":6,"seed":7}"#),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&next).unwrap(),
        serde_json::to_value(&again).unwrap()
    );
}

#[test]
fn each_neuron_reads_distinct_sorted_inputs() {
    let next = apply(
        &Network::default(),
        cmd(r#"{"command":"setConnectivity","index":0,"fanIn":20,"seed":3}"#),
    )
    .unwrap();
    let l = &next.layers[0];
    for row in 0..l.outputs {
        let cols = &l.columns[l.offsets[row]..l.offsets[row + 1]];
        assert_eq!(cols.len(), 20);
        assert!(cols.windows(2).all(|w| w[0] < w[1]), "row {row}: {cols:?}");
    }
}

#[test]
fn fan_in_is_capped_at_the_input_width() {
    let next = apply(
        &Network::default(),
        cmd(r#"{"command":"setConnectivity","index":1,"fanIn":500,"seed":3}"#),
    )
    .unwrap();
    assert_eq!(fan_in(&next.layers[1]), 64);
}

#[test]
fn the_decoder_cannot_be_resized_removed_or_followed() {
    let base = Network::default();
    for bad in [
        r#"{"command":"resizeLayer","index":1,"outputs":40,"seed":1}"#,
        r#"{"command":"removeLayer","index":1}"#,
        r#"{"command":"addLayer","at":2,"outputs":40,"fanIn":4,"seed":1}"#,
    ] {
        let err = apply(&base, cmd(bad)).unwrap_err();
        assert_eq!(err.code, "InvalidNetwork", "{bad}");
    }
}

#[test]
fn remove_and_resize_rewire_the_next_layer() {
    let base = Network::default();
    let removed = apply(&base, cmd(r#"{"command":"removeLayer","index":0}"#)).unwrap();
    assert_eq!(removed.layers.len(), 1);
    assert_eq!(
        (removed.layers[0].inputs, removed.layers[0].outputs),
        (128, 32)
    );
    removed.validate().unwrap();
    let resized = apply(
        &base,
        cmd(r#"{"command":"resizeLayer","index":0,"outputs":100,"seed":2}"#),
    )
    .unwrap();
    assert_eq!(resized.layers[0].outputs, 100);
    assert_eq!(resized.layers[1].inputs, 100);
    resized.validate().unwrap();
}

#[test]
fn limits_are_reported_not_silently_clamped() {
    let base = Network::default();
    let too_wide = apply(
        &base,
        cmd(r#"{"command":"resizeLayer","index":0,"outputs":2000,"seed":2}"#),
    );
    assert_eq!(too_wide.unwrap_err().code, "InvalidNetwork");
    // Exactly at the weight budget is allowed (1024 x 128 = 131072)...
    let wide = apply(
        &base,
        cmd(r#"{"command":"resizeLayer","index":0,"outputs":1024,"seed":2}"#),
    )
    .unwrap();
    apply(
        &wide,
        cmd(r#"{"command":"setConnectivity","index":0,"fanIn":128,"seed":2}"#),
    )
    .unwrap();
    // ...one input more per neuron over 1024 neurons is not.
    let over_budget = apply(
        &wide,
        cmd(r#"{"command":"addLayer","at":1,"outputs":1024,"fanIn":129,"seed":2}"#),
    );
    assert_eq!(over_budget.unwrap_err().code, "InvalidNetwork");
    let zero = apply(
        &base,
        cmd(r#"{"command":"setConnectivity","index":0,"fanIn":0,"seed":2}"#),
    );
    assert_eq!(zero.unwrap_err().code, "InvalidNetwork");
    let mut full = base.clone();
    for _ in 0..6 {
        full = apply(
            &full,
            cmd(r#"{"command":"addLayer","at":0,"outputs":16,"fanIn":4,"seed":1}"#),
        )
        .unwrap();
    }
    assert_eq!(full.layers.len(), 8);
    let ninth = apply(
        &full,
        cmd(r#"{"command":"addLayer","at":0,"outputs":16,"fanIn":4,"seed":1}"#),
    );
    assert_eq!(ninth.unwrap_err().code, "InvalidNetwork");
}

#[test]
fn init_weights_keeps_connectivity_and_changes_weights() {
    let base = Network::default();
    let next = apply(&base, cmd(r#"{"command":"initWeights","seed":99}"#)).unwrap();
    for (a, b) in base.layers.iter().zip(&next.layers) {
        assert_eq!(a.columns, b.columns);
        assert_eq!(a.offsets, b.offsets);
        assert_ne!(a.weights, b.weights);
        assert!(b.biases.iter().all(|&x| x == 0.01));
    }
}

#[test]
fn trace_ends_with_the_run_result() {
    let n = Network::default();
    let input = encode("red circle", &[], 0.0, 0.2);
    let layers = n.trace(&input, false, 9).unwrap();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].len(), 64);
    assert_eq!(layers[1], n.run(&input, false, 9, None).unwrap().values);
}

#[test]
fn a_malformed_input_network_is_rejected_before_any_work() {
    let mut bad = Network::default();
    // A previous layer claiming a huge width must not size the input pool
    // (billions of entries) before the network is validated.
    bad.layers[0].outputs = 2_000_000_000;
    let started = std::time::Instant::now();
    let err = apply(
        &bad,
        cmd(r#"{"command":"setConnectivity","index":1,"fanIn":4,"seed":1}"#),
    )
    .unwrap_err();
    assert_eq!(err.code, "InvalidNetwork");
    assert!(
        started.elapsed() < std::time::Duration::from_millis(200),
        "rejected only after {:?} of work",
        started.elapsed()
    );
}
