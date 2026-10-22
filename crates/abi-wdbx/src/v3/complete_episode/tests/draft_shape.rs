//! Strict test-fixture shape and duplicate-member parsing, separate from model validation.
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::fmt;
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("synthetic fixture JSON")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
        Ok(Unique(Value::Bool(v)))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
        Ok(Unique(Value::from(v)))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
        Ok(Unique(Value::from(v)))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
        Ok(Unique(Value::from(v)))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
        Ok(Unique(Value::from(v)))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
        let mut result = vec![];
        while let Some(Unique(v)) = a.next_element()? {
            result.push(v);
        }
        Ok(Unique(Value::Array(result)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
        let mut result = serde_json::Map::new();
        while let Some((key, Unique(value))) = a.next_entry::<String, Unique>()? {
            if result.insert(key, value).is_some() {
                return Err(de::Error::custom("duplicate fixture member"));
            }
        }
        Ok(Unique(Value::Object(result)))
    }
}
pub(super) fn parse(s: &str) -> Result<Value, &'static str> {
    serde_json::from_str::<Unique>(s)
        .map(|v| v.0)
        .map_err(|_| "fixture JSON")
}
type R = Result<(), &'static str>;
fn fields(v: &Value, required: &[&str], optional: &[&str]) -> R {
    let m = v.as_object().ok_or("fixture object")?;
    if required.iter().any(|k| !m.contains_key(*k))
        || m.keys()
            .any(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err("fixture fields");
    }
    Ok(())
}
fn list(v: &Value, f: fn(&Value) -> R) -> R {
    for x in v.as_array().ok_or("fixture array")? {
        f(x)?;
    }
    Ok(())
}
fn estimate(v: &Value, f: fn(&Value) -> R) -> R {
    if v.get("unknown") == Some(&Value::Bool(true)) {
        fields(v, &["unknown"], &[])
    } else {
        fields(v, &["known"], &[])?;
        f(&v["known"])
    }
}
fn scalar(v: &Value) -> R {
    if v.is_null() {
        Err("null fixture")
    } else {
        Ok(())
    }
}
fn binding(v: &Value) -> R {
    fields(v, &["tenant", "scope"], &[])?;
    let s = &v["scope"];
    let members = match s["variant"].as_str() {
        Some("Guild") => vec!["variant", "guild"],
        Some("GuildMember") => vec!["variant", "guild", "member"],
        Some("DirectMessage") => vec!["variant", "user"],
        Some("Organization") => vec!["variant", "organization"],
        _ => return Err("scope"),
    };
    fields(s, &members, &[])
}
fn version(v: &Value) -> R {
    fields(v, &["component", "revision", "digest"], &[])
}
fn local(v: &Value) -> R {
    fields(v, &["binding", "domain", "digest"], &[])?;
    binding(&v["binding"])
}
fn artifact(v: &Value) -> R {
    fields(
        v,
        &["binding", "digest", "byte_length", "kind", "format"],
        &[],
    )?;
    binding(&v["binding"])?;
    version(&v["format"])
}
fn consulted(v: &Value) -> R {
    fields(v, &["variant", "reference"], &[])?;
    let r = &v["reference"];
    match v["variant"].as_str() {
        Some("Local") => local(r),
        Some("Foreign") => {
            fields(
                r,
                &[
                    "origin_organization",
                    "origin_binding",
                    "domain",
                    "digest",
                    "import_receipt",
                ],
                &[],
            )?;
            binding(&r["origin_binding"])?;
            local(&r["import_receipt"])
        }
        _ => Err("consulted"),
    }
}
fn reading(v: &Value) -> R {
    fields(v, &["resource", "amount"], &[])
}
fn action(v: &Value) -> R {
    fields(
        v,
        &["capability", "action", "parameter_classes", "phase"],
        &[],
    )?;
    version(&v["capability"])
}
fn measurement(v: &Value) -> R {
    fields(v, &["unit", "value"], &[])
}
fn consent(v: &Value) -> R {
    fields(v, &["binding", "epoch", "policy", "attestation"], &[])?;
    binding(&v["binding"])?;
    version(&v["policy"])?;
    local(&v["attestation"])
}
fn retention(v: &Value) -> R {
    let keys = match v["variant"].as_str() {
        Some("Ephemeral") => vec!["variant"],
        Some("Session") => vec!["variant", "session", "expires_at"],
        Some("Operational") => vec!["variant", "expires_at"],
        Some("Durable") => vec!["variant", "privacy_basis"],
        Some("MandatoryIncident") => vec!["variant", "obligation"],
        _ => return Err("retention"),
    };
    fields(v, &keys, &[])?;
    for k in ["privacy_basis", "obligation"] {
        if let Some(x) = v.get(k) {
            local(x)?;
        }
    }
    Ok(())
}
fn edge(v: &Value) -> R {
    fields(v, &["relation", "target", "basis"], &[])?;
    local(&v["target"])?;
    local(&v["basis"])
}
pub(super) fn validate(fixture: &Value) -> R {
    fields(
        fixture,
        &[
            "identity",
            "time",
            "origin",
            "context",
            "decision",
            "outcome",
            "authority",
            "evidence",
            "lifecycle",
            "provenance",
        ],
        &[],
    )?;
    let identity = &fixture["identity"];
    fields(
        identity,
        &["episode_id", "request_id", "binding", "contract"],
        &[],
    )?;
    binding(&identity["binding"])?;
    version(&identity["contract"])?;
    let time = &fixture["time"];
    fields(time, &["acquired_at", "received_at"], &["monotonic"])?;
    if let Some(m) = time.get("monotonic") {
        fields(m, &["clock", "start", "end"], &[])?;
    }
    let origin = &fixture["origin"];
    fields(origin, &["source_node", "actor", "signer"], &[])?;
    fields(&origin["actor"], &["principal", "kind"], &[])?;
    let context = &fixture["context"];
    fields(
        context,
        &[
            "task_id",
            "regime",
            "regime_posterior",
            "habituation",
            "resources",
        ],
        &["summary", "reservoir"],
    )?;
    estimate(&context["regime_posterior"], scalar)?;
    estimate(&context["habituation"], scalar)?;
    list(&context["resources"], reading)?;
    for k in ["summary", "reservoir"] {
        if let Some(x) = context.get(k) {
            artifact(x)?;
        }
    }
    let decision = &fixture["decision"];
    fields(
        decision,
        &[
            "route",
            "prediction",
            "actions",
            "uncertainty",
            "risk",
            "novelty",
        ],
        &[],
    )?;
    estimate(&decision["prediction"], artifact)?;
    list(&decision["actions"], action)?;
    for k in ["uncertainty", "risk", "novelty"] {
        estimate(&decision[k], scalar)?;
    }
    let outcome = &fixture["outcome"];
    fields(
        outcome,
        &["observed", "utility", "safety_effect", "costs", "rollback"],
        &[],
    )?;
    estimate(&outcome["observed"], artifact)?;
    estimate(&outcome["utility"], measurement)?;
    list(&outcome["costs"], reading)?;
    let authority = &fixture["authority"];
    fields(
        authority,
        &[
            "policy",
            "safety_policy",
            "capabilities",
            "models",
            "calibrations",
            "approvals",
        ],
        &["consent"],
    )?;
    version(&authority["policy"])?;
    version(&authority["safety_policy"])?;
    for k in ["capabilities", "models", "calibrations"] {
        list(&authority[k], version)?;
    }
    list(&authority["approvals"], local)?;
    if let Some(x) = authority.get("consent") {
        consent(x)?;
    }
    let evidence = &fixture["evidence"];
    fields(
        evidence,
        &["maturity", "source_refs", "observation_refs", "dimensions"],
        &[],
    )?;
    list(&evidence["source_refs"], artifact)?;
    list(&evidence["observation_refs"], artifact)?;
    let keys = [
        "cryptographic",
        "source",
        "calibration",
        "outcome",
        "compatibility",
        "regime",
        "lifecycle",
        "confidence",
    ];
    fields(&evidence["dimensions"], &keys, &[])?;
    for k in keys {
        estimate(&evidence["dimensions"][k], artifact)?;
    }
    let lifecycle = &fixture["lifecycle"];
    fields(
        lifecycle,
        &["retention", "holds", "edges"],
        &["deletion_key"],
    )?;
    retention(&lifecycle["retention"])?;
    list(&lifecycle["holds"], local)?;
    list(&lifecycle["edges"], edge)?;
    if let Some(x) = lifecycle.get("deletion_key") {
        fields(x, &["binding", "key_id"], &[])?;
        binding(&x["binding"])?;
    }
    let provenance = &fixture["provenance"];
    fields(
        provenance,
        &["causal_parents", "consulted_parents", "records"],
        &[],
    )?;
    list(&provenance["causal_parents"], local)?;
    list(&provenance["consulted_parents"], consulted)?;
    list(&provenance["records"], artifact)
}
