//! Finite independent mapping coverage; Rust remains primary admission validation.
use super::draft_witness::{compare, fixture};
use serde_json::{Value, json};
fn case_at(base: &Value, path: &str, value: Value) -> Value {
    let mut v = base.clone();
    *v.pointer_mut(path).unwrap() = value;
    v
}
fn local(base: &Value) -> Value {
    base["provenance"]["causal_parents"][0].clone()
}
fn artifact(base: &Value) -> Value {
    base["evidence"]["source_refs"][0].clone()
}
fn rebind(v: &mut Value, old: &Value, new: &Value) {
    if v == old {
        *v = new.clone();
        return;
    }
    match v {
        Value::Object(m) => {
            for x in m.values_mut() {
                rebind(x, old, new);
            }
        }
        Value::Array(a) => {
            for x in a {
                rebind(x, old, new);
            }
        }
        _ => {}
    }
}
pub(super) fn vocabulary_cases() -> Vec<Value> {
    let base = fixture(true);
    let minimal = fixture(false);
    let mut cases = vec![];
    for (path, names) in [
        (
            "/decision/route",
            vec![
                "NoModel",
                "LocalModel",
                "CloudModel",
                "Specialist",
                "Deterministic",
            ],
        ),
        ("/decision/actions/0/phase", vec!["Proposed", "Performed"]),
        (
            "/outcome/safety_effect",
            vec![
                "Unknown",
                "NoViolationObserved",
                "InterlockTriggered",
                "ViolationObserved",
            ],
        ),
        (
            "/outcome/rollback",
            vec![
                "NotApplicable",
                "NotAttempted",
                "Attempted",
                "ExactRestoreObserved",
                "Failed",
                "Unknown",
            ],
        ),
        (
            "/origin/actor/kind",
            vec![
                "HumanSubject",
                "OrganizationOwner",
                "GuildOwner",
                "GuildAdministrator",
                "GuildManager",
                "Service",
            ],
        ),
        (
            "/evidence/maturity",
            vec!["C0", "C1", "C2", "C3", "C4", "C5", "C6", "C7"],
        ),
        (
            "/lifecycle/edges/0/relation",
            vec![
                "Supersedes",
                "Contradicts",
                "Revokes",
                "Quarantines",
                "Resolves",
                "Redacts",
                "Forgets",
            ],
        ),
        (
            "/provenance/causal_parents/0/domain",
            vec!["CompleteV1", "LegacyEpisodeV1"],
        ),
        (
            "/provenance/consulted_parents/0/reference/domain",
            vec!["CompleteV1", "LegacyEpisodeV1"],
        ),
        (
            "/provenance/consulted_parents/1/reference/domain",
            vec!["CompleteV1", "LegacyEpisodeV1"],
        ),
    ] {
        for name in names {
            let mut v = case_at(&base, path, json!(name));
            // The other edge otherwise duplicates Resolves when that variant is visited.
            v["lifecycle"]["edges"][1]["basis"]["digest"] = json!("44".repeat(32));
            cases.push(v);
        }
    }
    for unit in ["Unitless", "Count", "Milliseconds", "Bytes", "Microjoules"] {
        cases.push(case_at(
            &base,
            "/outcome/utility",
            json!({"known":{"unit":unit,"value":1_000_000}}),
        ));
    }
    for kind in [
        "StateSummary",
        "Prediction",
        "Outcome",
        "Observation",
        "Model",
        "Calibration",
        "Provenance",
        "ReservoirEmbedding",
        "Evidence",
        "Authority",
    ] {
        let mut a = artifact(&base);
        a["kind"] = json!(kind);
        cases.push(case_at(&base, "/evidence/source_refs", json!([a])));
    }
    for kind in [
        "Tokens",
        "CpuMicros",
        "MemoryBytes",
        "NetworkBytes",
        "EnergyMicrojoules",
    ] {
        cases.push(case_at(
            &base,
            "/context/resources",
            json!([{"resource":kind,"amount":u64::MAX}]),
        ));
        cases.push(case_at(
            &base,
            "/outcome/costs",
            json!([{"resource":kind,"amount":0}]),
        ));
    }
    for scope in [
        json!({"variant":"Guild","guild":"new-guild"}),
        json!({"variant":"GuildMember","guild":"new-guild","member":"new-member"}),
        json!({"variant":"DirectMessage","user":"new-user"}),
        json!({"variant":"Organization","organization":"new-org"}),
    ] {
        let mut v = base.clone();
        let old = v["identity"]["binding"].clone();
        let new = json!({"tenant":"rebound-tenant","scope":scope});
        rebind(&mut v, &old, &new);
        cases.push(v);
    }
    for retention in [
        json!({"variant":"Ephemeral"}),
        json!({"variant":"Session","session":"synthetic-session","expires_at":100}),
        json!({"variant":"Operational","expires_at":100}),
        json!({"variant":"Durable","privacy_basis":local(&base)}),
        json!({"variant":"MandatoryIncident","obligation":local(&base)}),
    ] {
        cases.push(case_at(&base, "/lifecycle/retention", retention));
    }
    let mut foreign = base.clone();
    foreign["provenance"]["consulted_parents"][1]["reference"]["origin_binding"]["scope"] =
        json!({"variant":"DirectMessage","user":"foreign-user"});
    cases.push(foreign);
    for n in [
        0,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        u64::from(u32::MAX),
        u64::from(u32::MAX) + 1,
        u64::MAX,
    ] {
        cases.push(case_at(&base, "/context/resources/0/amount", json!(n)));
        cases.push(case_at(&base, "/time/acquired_at", json!(n)));
        cases.push(case_at(&base, "/authority/consent/epoch", json!(n)));
    }
    for n in [
        i64::MIN,
        -4_294_967_297,
        -4_294_967_296,
        -65_537,
        -65_536,
        -257,
        -256,
        -25,
        -24,
        -1,
        0,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        4_294_967_296,
        i64::MAX,
    ] {
        cases.push(case_at(&base, "/outcome/utility/known/value", json!(n)));
    }
    cases.push(case_at(
        &base,
        "/identity/contract/digest",
        json!("00".repeat(32)),
    ));
    cases.push(case_at(
        &base,
        "/provenance/causal_parents/0/digest",
        json!("00".repeat(32)),
    ));
    let mut v = minimal.clone();
    v["context"]["habituation"] = json!({"known":0});
    cases.push(v);
    let mut v = minimal.clone();
    v["context"]["summary"] = artifact(&base);
    v["context"]["summary"]["kind"] = json!("StateSummary");
    cases.push(v);
    // All evidence fields are independently mapped, rather than relying only on equal populated values.
    for key in [
        "cryptographic",
        "source",
        "calibration",
        "outcome",
        "compatibility",
        "regime",
        "lifecycle",
        "confidence",
    ] {
        let mut v = minimal.clone();
        let mut a = artifact(&base);
        a["kind"] = json!("Evidence");
        v["evidence"]["dimensions"][key] = json!({"known":a});
        cases.push(v);
    }
    cases
}
fn bytes(v: &Value) -> Vec<u8> {
    super::super::CompleteEpisode::try_new(super::draft_adapter::input(v).unwrap())
        .unwrap()
        .canonical_bytes()
        .unwrap()
}
pub(super) fn set_reversal_cases() -> Vec<Value> {
    let mut base = fixture(true);
    // Give every set at least two distinct members so every reversal is meaningful.
    for path in ["/authority/models", "/authority/calibrations"] {
        let a = base.pointer_mut(path).unwrap().as_array_mut().unwrap();
        let mut v = a[0].clone();
        v["component"] = json!("aaa");
        a.push(v);
    }
    {
        let path = "/lifecycle/holds";
        let a = base.pointer_mut(path).unwrap().as_array_mut().unwrap();
        let mut v = a[0].clone();
        v["digest"] = json!("55".repeat(32));
        a.push(v);
    }
    for path in ["/evidence/observation_refs", "/provenance/records"] {
        let a = base.pointer_mut(path).unwrap().as_array_mut().unwrap();
        let mut v = a[0].clone();
        v["digest"] = json!("56".repeat(32));
        a.push(v);
    }
    let original = bytes(&base);
    let mut cases = vec![base.clone()];
    for path in [
        "/context/resources",
        "/outcome/costs",
        "/authority/capabilities",
        "/authority/models",
        "/authority/calibrations",
        "/authority/approvals",
        "/evidence/source_refs",
        "/evidence/observation_refs",
        "/lifecycle/holds",
        "/lifecycle/edges",
        "/provenance/causal_parents",
        "/provenance/consulted_parents",
        "/provenance/records",
        "/decision/actions/0/parameter_classes",
        "/context/regime_posterior/known",
    ] {
        let mut v = base.clone();
        v.pointer_mut(path)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .reverse();
        assert_eq!(bytes(&v), original, "{path}");
        cases.push(v);
    }
    let mut sequence = base.clone();
    let mut second = sequence["decision"]["actions"][0].clone();
    second["action"] = json!("second");
    sequence["decision"]["actions"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let forward = bytes(&sequence);
    let mut reverse = sequence.clone();
    reverse["decision"]["actions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_ne!(bytes(&reverse), forward);
    let mut repeated = base.clone();
    let action = repeated["decision"]["actions"][0].clone();
    repeated["decision"]["actions"]
        .as_array_mut()
        .unwrap()
        .push(action);
    cases.extend([sequence, reverse, repeated]);
    cases
}
#[test]
fn every_vocabulary_scope_and_integer_mapping() {
    compare(&vocabulary_cases());
}
#[test]
fn independent_set_reversals_and_action_sequence() {
    compare(&set_reversal_cases());
}
