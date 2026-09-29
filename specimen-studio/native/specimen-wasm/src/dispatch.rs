//! JSON command surface over the kernel, shared by the WASM export and tests.
//! A request is `{"op": …, …}`; the reply is `{"ok": …}` or `{"error": {…}}`.
use serde_json::{Value, json};
use specimen_kernel::{
    Result, engine, error,
    host::Host,
    mutate::{self, NodeInput, ResourceInput},
    neural::Network,
};
use std::sync::atomic::AtomicBool;

fn field<'a>(req: &'a Value, key: &str) -> Result<&'a Value> {
    match &req[key] {
        Value::Null => Err(error("Invalid", format!("Missing {key}"))),
        v => Ok(v),
    }
}

fn string<'a>(req: &'a Value, key: &str) -> Result<&'a str> {
    field(req, key)?
        .as_str()
        .ok_or_else(|| error("Invalid", format!("{key} must be text")))
}

/// The request's `network` (validated), or the default when absent: cycles
/// and reviews compose with the caller's network like the desktop does.
fn network_or_default(req: &Value) -> Result<Network> {
    match req.get("network") {
        None | Some(Value::Null) => Ok(Network::default()),
        Some(value) => {
            let network: Network =
                serde_json::from_value(value.clone()).map_err(|e| error("InvalidNetwork", e))?;
            network
                .validate()
                .map_err(|e| error("InvalidNetwork", e.message))?;
            Ok(network)
        }
    }
}

fn optional<'a>(req: &'a Value, key: &str) -> Option<&'a str> {
    req[key].as_str().filter(|s| !s.is_empty())
}

/// Runs one command. `progress` receives each trace step of a cycle.
pub fn dispatch(h: &Host, req: &Value, progress: &dyn Fn(Value)) -> Result<Value> {
    let state = || field(req, "state");
    let op = string(req, "op")?;
    match op {
        "seed" => Ok(mutate::seed(h)),
        "validate" => {
            engine::validate(state()?)?;
            mutate::validate_settings(&state()?["settings"])?;
            mutate::validate_records(state()?)?;
            Ok(Value::Null)
        }
        "validateSettings" => {
            mutate::validate_settings(field(req, "settings")?).map(|()| Value::Null)
        }
        "migrateIds" => Ok(mutate::migrate_ids(h, state()?)),
        "networkDefault" => Ok(serde_json::to_value(Network::default())?),
        "networkValidate" => {
            let network: Network = serde_json::from_value(field(req, "network")?.clone())
                .map_err(|e| error("InvalidNetwork", e))?;
            network
                .validate()
                .map_err(|e| error("InvalidNetwork", e.message))?;
            Ok(Value::Null)
        }
        "networkEdit" => {
            let network: Network = serde_json::from_value(field(req, "network")?.clone())?;
            let command = serde_json::from_value(field(req, "command")?.clone())?;
            let next = specimen_kernel::network_edit::apply(&network, command)?;
            Ok(serde_json::to_value(next)?)
        }
        "networkTrace" => {
            let network: Network = serde_json::from_value(field(req, "network")?.clone())?;
            let input = specimen_kernel::neural::encode(string(req, "text")?, &[], 0.0, 0.2);
            let seed = field(req, "seed")?
                .as_u64()
                .ok_or_else(|| error("InvalidInput", "seed must be a whole number"))?;
            Ok(serde_json::to_value(network.trace(&input, false, seed)?)?)
        }
        "raftPlan" => {
            let n = |k: &str| -> Result<usize> {
                field(req, k)?
                    .as_u64()
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| error("InvalidInput", format!("{k} must be a whole number")))
            };
            let plan = specimen_kernel::search::raft_plan(
                n("len")?,
                n("chunk")?,
                n("workers")?,
                n("limit")?,
            )?;
            Ok(serde_json::to_value(plan)?)
        }
        "addNode" => {
            let input: NodeInput = serde_json::from_value(field(req, "input")?.clone())?;
            mutate::add_node(h, state()?, &input, optional(req, "ref"))
        }
        "addEntry" => mutate::add_entry(
            h,
            state()?,
            string(req, "nodeRef")?,
            string(req, "pattern")?,
            string(req, "action")?,
        ),
        "removeNode" => Ok(mutate::remove_node(h, state()?, string(req, "ref")?)),
        "saveResource" => {
            let input: ResourceInput = serde_json::from_value(field(req, "input")?.clone())?;
            mutate::save_resource(h, state()?, &input, optional(req, "ref"))
        }
        "feedback" => {
            let right = field(req, "right")?
                .as_bool()
                .ok_or_else(|| error("Invalid", "right must be true or false"))?;
            let (s, message) = mutate::feedback(
                h,
                state()?,
                string(req, "cycleId")?,
                right,
                optional(req, "color"),
            )?;
            Ok(json!({"state": s, "message": message}))
        }
        "togglePin" => mutate::toggle_pin(h, state()?, string(req, "id")?),
        "log" => {
            let mut s = state()?.clone();
            mutate::log(
                h.env,
                &mut s,
                string(req, "type")?,
                string(req, "title")?,
                string(req, "detail")?,
            );
            Ok(s)
        }
        "cycle" => {
            let network = network_or_default(req)?;
            let (s, cycle) = engine::cycle(
                h,
                state()?,
                string(req, "input")?,
                &network,
                &AtomicBool::new(false),
                progress,
            )?;
            Ok(json!({"state": s, "cycle": cycle}))
        }
        "review" => engine::review(
            h,
            state()?,
            &network_or_default(req)?,
            &AtomicBool::new(false),
        ),
        "maintain" => engine::maintain(h, state()?, string(req, "mode")?, &AtomicBool::new(false)),
        other => Err(error(
            "UnknownCommand",
            format!("Unknown operation {other}"),
        )),
    }
}

/// Wraps `dispatch` in the reply envelope.
pub fn reply(h: &Host, input: &str, progress: &dyn Fn(Value)) -> String {
    let result = serde_json::from_str::<Value>(input)
        .map_err(Into::into)
        .and_then(|req| dispatch(h, &req, progress));
    match result {
        Ok(v) => json!({"ok": v}).to_string(),
        Err(e) => json!({"error": {"code": e.code, "message": e.message}}).to_string(),
    }
}

/// Milliseconds since the Unix epoch as RFC 3339 UTC with milliseconds.
pub fn rfc3339(ms: i64) -> String {
    let (secs, millis) = (ms.div_euclid(1000), ms.rem_euclid(1000));
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant), proleptic Gregorian.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// A v4 UUID from 128 random bits.
pub fn uuid_v4(words: [u32; 4]) -> String {
    let mut b = [0u8; 16];
    for (i, w) in words.iter().enumerate() {
        b[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use specimen_kernel::host::FixedHost;
    use std::cell::RefCell;

    fn call(req: Value) -> Value {
        let fixed = FixedHost::default();
        serde_json::from_str(&reply(&fixed.host(), &req.to_string(), &|_| {})).unwrap()
    }

    #[test]
    fn seed_then_mutate_through_the_envelope() {
        let s = call(json!({"op": "seed"}))["ok"].clone();
        assert_eq!(s["nodes"].as_array().unwrap().len(), 8);
        let next = call(
            json!({"op": "addNode", "state": s, "input": {"name": "Color", "pattern": "favorite color", "action": "Teal."}}),
        );
        assert_eq!(next["ok"]["nodes"].as_array().unwrap().len(), 9);
        assert_eq!(
            call(json!({"op": "validate", "state": next["ok"]}))["ok"],
            Value::Null
        );
    }

    #[test]
    fn errors_come_back_as_codes_and_messages() {
        let s = call(json!({"op": "seed"}))["ok"].clone();
        let r = call(
            json!({"op": "addNode", "state": s, "input": {"name": "", "pattern": "x", "action": "y"}}),
        );
        assert_eq!(r["error"]["message"], "Enter a name, pattern, and action.");
        assert_eq!(
            call(json!({"op": "nope"}))["error"]["code"],
            "UnknownCommand"
        );
        assert_eq!(
            call(json!({"op": "addEntry"}))["error"]["message"],
            "Missing state"
        );
        let fixed = FixedHost::default();
        let bad: Value = serde_json::from_str(&reply(&fixed.host(), "{not json", &|_| {})).unwrap();
        assert_eq!(bad["error"]["code"], "MalformedSave");
    }

    #[test]
    fn cycle_streams_trace_steps_and_returns_the_answer() {
        let s = call(json!({"op": "seed"}))["ok"].clone();
        let fixed = FixedHost::default();
        let steps = RefCell::new(vec![]);
        let out: Value = serde_json::from_str(&reply(
            &fixed.host(),
            &json!({"op": "cycle", "state": s, "input": "What is 2 + 2?"}).to_string(),
            &|step| steps.borrow_mut().push(step),
        ))
        .unwrap();
        assert_eq!(out["ok"]["cycle"]["segments"][0]["text"], "4");
        let phases: Vec<String> = steps
            .borrow()
            .iter()
            .map(|s| s["phase"].as_str().unwrap().to_owned())
            .collect();
        assert!(phases.starts_with(&["Prepare".to_owned()]), "{phases:?}");
        assert!(phases.iter().any(|p| p == "Compose"), "{phases:?}");
    }

    #[test]
    fn feedback_returns_state_and_message() {
        let s = call(json!({"op": "seed"}))["ok"].clone();
        let cycled =
            call(json!({"op": "cycle", "state": s, "input": "What is 2 + 2?"}))["ok"].clone();
        let id = cycled["cycle"]["id"].clone();
        let r =
            call(json!({"op": "feedback", "state": cycled["state"], "cycleId": id, "right": true}));
        assert!(
            r["ok"]["message"]
                .as_str()
                .unwrap()
                .contains("contributors changed strength")
        );
    }

    #[test]
    fn rfc3339_formats_known_instants() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(1_767_225_600_000), "2026-01-01T00:00:00.000Z");
        assert_eq!(rfc3339(951_782_400_123), "2000-02-29T00:00:00.123Z");
        assert_eq!(rfc3339(1_790_664_600_999), "2026-09-29T06:50:00.999Z");
    }

    #[test]
    fn uuid_v4_sets_version_and_variant_bits() {
        let u = uuid_v4([0xffff_ffff, 0xffff_ffff, 0, 0x1234_5678]);
        assert_eq!(u, "ffffffff-ffff-4fff-8000-000012345678");
        assert_eq!(u.len(), 36);
    }
}
