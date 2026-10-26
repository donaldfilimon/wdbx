#![cfg(feature = "scoring-probe")]
#[test]
fn finite_trace_is_deterministic_bounded_and_calls_actual_apis() {
    let a = specimen_wasm::scoring_probe::run();
    assert_eq!(a, specimen_wasm::scoring_probe::run());
    assert!(a.len() < 512 * 1024);
    let raw: serde_json::Value = serde_json::from_str(&a).unwrap();
    let v = &raw["semantic"];
    assert_eq!(
        raw["diagnostics"]["observations"].as_array().unwrap().len(),
        68
    );
    assert_eq!(v["numeric"].as_array().unwrap().len(), 18);
    assert_eq!(v["entries"]["evaluated"].as_array().unwrap().len(), 12);
    assert_eq!(
        v["entries"]["refusals"][4]["output"]["error"],
        "BindingMismatch"
    );
    assert_eq!(
        v["orchestration"]["attachment"]["early_close"],
        "Incomplete"
    );
    assert_eq!(
        v["orchestration"]["attachment"]["plan"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn fixed_same_limit_decisions_preserve_refused_rng_and_private_lease_retry() {
    let raw: serde_json::Value =
        serde_json::from_str(&specimen_wasm::scoring_probe::run()).unwrap();
    let b = &raw["semantic"]["logical_budget"];
    assert_eq!(b["profile"], "logical-charge-v1");
    assert_eq!(
        b["fixture"]["alternatives"][0]["action"][0]["value"]
            .as_str()
            .unwrap()
            .len(),
        2
    );
    let rows = b["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    let expected = [
        2970, 2971, 2157, 2158, 3354, 3355, 2791, 2792, 465, 466, 2985, 2986, 2303, 2304,
    ];
    for (i, row) in rows.iter().take(14).enumerate() {
        assert_eq!(row["limit"], expected[i]);
        if i % 2 == 0 {
            assert_eq!(
                row["output"]["error"],
                if i < 8 {
                    "BudgetExceeded"
                } else {
                    "Backpressure"
                }
            );
        } else {
            assert!(row["output"]["error"].is_null());
        }
        if i < 8 {
            assert_eq!(row["draws_before"], row["draws_after"]);
        } else if i % 2 == 0 {
            assert_eq!(row["before"], row["after"]);
            assert_eq!(row["status_before"], row["status_after"]);
        }
        if i >= 10 && i % 2 == 0 {
            assert_eq!(row["fallback"]["result"], "Ok");
            assert_eq!(row["fallback"]["counts"]["active"], 0);
        }
    }
    assert_eq!(rows[13]["after"]["owned_bytes"], 2986);
    assert_eq!(rows[13]["extra_cycle"]["error"], "Backpressure");
    assert_eq!(
        rows[13]["extra_cycle"]["before"],
        rows[13]["extra_cycle"]["after"]
    );
    assert_eq!(rows[13]["after_restart_retire"]["owned_bytes"], 274);
}

#[test]
fn attachment_fixed_limits_keep_retry_visits_and_unrelated_lease_atomic() {
    let raw: serde_json::Value =
        serde_json::from_str(&specimen_wasm::scoring_probe::run()).unwrap();
    let rows = raw["semantic"]["logical_budget"]["rows"]
        .as_array()
        .unwrap();
    let expected = [2936, 2937, 4023, 4024, 3863, 3864, 2933, 2934, 2929, 2930];
    for (i, row) in rows.iter().skip(14).enumerate() {
        assert_eq!(row["limit"], expected[i]);
        if i % 2 == 0 {
            assert_eq!(row["output"]["error"], "Backpressure");
            assert_eq!(row["before"], row["after"]);
            assert_eq!(row["draws_after"], 0);
            assert!(row["retry"]["output"]["error"].is_null());
        } else {
            assert!(row["output"]["error"].is_null());
            assert_eq!(row["draws_after"], 1);
        }
        assert_eq!(row["repeated"]["output"]["error"], "Repeated");
        assert_eq!(row["repeated"]["draws_after"], 1);
        assert_eq!(row["drained"]["active"], 0);
        assert_eq!(row["after_restart_retire"]["owned_bytes"], 274);
    }
}
