#![cfg(feature = "language-probe")]
use serde_json::Value;
#[test]
fn pure_language_fixture_substantively_exercises_mechanisms() {
    let trace: Value = serde_json::from_str(&specimen_wasm::language_probe::run()).unwrap();
    assert_eq!(trace["schema"], "task8-language-mechanism-probe-v1");
    assert_eq!(trace["numeric"][0]["actions"][0]["value"], 4.0);
    assert_eq!(trace["numeric"][1]["actions"][0]["value"], 4.0);
    assert_eq!(trace["errors"]["division"], "DivisionByZero");
    assert_eq!(trace["scripts"]["nested_denial"], "UndeclaredContact");
    assert_eq!(trace["scripts"]["stale"], "StaleRegistry");
    assert_eq!(
        trace["scans"]["uncertain_body"]["components"][2]["reason"],
        "UncertainNegation"
    );
    assert_eq!(trace["shared_low"]["refs"], serde_json::json!(["1", "2"]));
    assert_ne!(
        trace["shared_low"]["scans"][0]["components"][3],
        trace["shared_low"]["scans"][1]["components"][3]
    );
    assert_eq!(
        trace["routes"]["hits"][0]["origins"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(trace["huge"]["difference"], serde_json::json!(["1", "3"]));
}

#[test]
fn witness_preserves_source_guards_variants_and_class_boundaries() {
    let trace: Value = serde_json::from_str(&specimen_wasm::language_probe::run()).unwrap();
    assert_eq!(
        trace["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| (k["count"].as_u64().unwrap(), k["class"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![(6, "low"), (7, "medium"), (16, "medium"), (17, "high")]
    );
    assert_eq!(
        trace["weighted"][0]["actions"][1]["tokens"][0]["lemma"],
        "radiant"
    );
    assert_eq!(
        trace["weighted"][1]["actions"][1]["tokens"][0]["lemma"],
        "shining"
    );
    assert_eq!(
        trace["utility"]["actions"][1]["origin"]["rules"],
        serde_json::json!(["sense"])
    );
    assert_eq!(
        trace["utility"]["actions"][0]["definitions"][0]["definition"],
        "luminous"
    );
    assert_eq!(
        trace["utility"]["actions"][0]["charges"][0]["intensity"],
        0.75
    );
    assert_eq!(trace["quoted"]["actions"][0]["tokens"][0]["quoted"], true);
    assert_eq!(
        trace["quoted"]["actions"][0]["constraints"]["inhibited"],
        false
    );
    assert_eq!(
        trace["quoted"]["actions"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["constraints"]["unresolved_frame"],
        true
    );
    assert_eq!(
        trace["decimal"]["actions"][0]["captures"][0]["decimal"],
        serde_json::json!({"coefficient":"12","scale":1})
    );
    assert!(
        trace["unframed"]["actions"][0]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["kind"] == "Word" && t["decimal"].is_null())
    );
    assert_eq!(
        trace["scans"]["antecedent_only"]["components"][2]["s"],
        serde_json::json!(["20", "1"])
    );
    assert_eq!(
        trace["scans"]["uncertain_antecedent"]["components"][2]["s"],
        serde_json::json!(["20", "1"])
    );
    assert_eq!(
        trace["scans"]["antecedent_body_conflict"]["components"][2]["reason"],
        "NegationConflict"
    );
    assert_eq!(
        trace["scans"]["repeated"]["components"][1]["s"],
        serde_json::json!(["20", "3"])
    );
    assert_eq!(
        trace["scans"]["partial_roles"]["components"][2]["s"],
        serde_json::json!(["40", "3"])
    );
    assert_eq!(
        trace["routes"]["repeated"][0]["origins"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        trace["routes"]["blocked"][0]["origins"][0]["path"]["status"],
        "Blocked"
    );
    assert_eq!(trace["scripts"]["runaway"], "StepBudget");
}

#[test]
fn final_fix_witness_exercises_guards_and_borrowed_budget_refusal() {
    let trace: Value = serde_json::from_str(&specimen_wasm::language_probe::run()).unwrap();
    let f = &trace["final_fix"];
    assert_eq!(f["conditions"][0]["guard"], "Err(ActionBlocked)");
    assert_eq!(f["conditions"][2]["guard"], "Ok(\"executed\")");
    assert_eq!(
        f["invalid_controls"],
        serde_json::json!(["InvalidTables", "InvalidTables", "InvalidTables"])
    );
    assert_eq!(f["empty_query_error"], "BudgetExceeded");
    assert_eq!(f["selected_alternatives_error"], "BudgetExceeded");
    assert_eq!(f["complete_alternative_count"], 12000);
    assert_eq!(f["complexity"]["getter"], 6);
}
