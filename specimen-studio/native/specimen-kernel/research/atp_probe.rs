//! Standalone private whitebox diagnostic; never registered in the kernel.
#[cfg(feature = "atp-diagnostic")]
#[path = "../src/atp_v3/mod.rs"]
mod atp_v3;
#[cfg(feature = "atp-diagnostic")]
#[path = "atp_fixtures.rs"]
mod fixtures;

/// Bounded authored synthetic trace. Local unratified JSON, no consumer interface.
#[cfg(feature = "atp-diagnostic")]
pub fn trace_json() -> String {
    use atp_v3::{State, Stimulus, advance};
    use serde_json::json;
    let mut rows = Vec::new();
    let mut states = std::collections::BTreeMap::new();
    for mut f in fixtures::fixtures() {
        if let Some(previous) = &f.previous { f.state = states[previous]; }
        let input = f.input();
        let state = State::new(f32::from_bits(f.state[0]), f32::from_bits(f.state[1]), f32::from_bits(f.state[2]));
        let stimulus = Stimulus::new(f32::from_bits(f.stimulus[0]), f32::from_bits(f.stimulus[1]), f.urgency);
        let output = match (state,stimulus) {
            (Err(e),_) => json!({"error":format!("State::{e:?}")}),
            (_,Err(e)) => json!({"error":format!("Stimulus::{e:?}")}),
            (Ok(state),Ok(stimulus)) => match advance(state,stimulus,f32::from_bits(f.elapsed)) {
                Err(e) => json!({"error":format!("Elapsed::{e:?}")}),
                Ok(t) => {
                    let s=t.state();
                    let bits=[s.v().to_bits(),s.a().to_bits(),s.cooldown_seconds().to_bits()];
                    states.insert(f.id.clone(),bits);
                    json!({"state":bits.map(fixtures::hex),"applied_elapsed":fixtures::hex(t.applied_elapsed().to_bits()),
                        "released":t.released(),"cooldown_at_entry":t.cooldown_at_entry()})
                }
            }
        };
        rows.push(json!({"id":f.id,"previous":f.previous,"input":input,"output":output}));
    }
    json!({"schema":"private-atp-diagnostic-v1","profile":"logicalf32-v1","rows":rows}).to_string()
}

/// One call per fresh diagnostic instance; trace storage is deliberately leaked.
/// The bound is below 64KiB. There is no allocation/free ABI or installed runtime.
#[cfg(all(feature = "atp-diagnostic", target_arch = "wasm32"))]
#[unsafe(no_mangle)]
pub extern "C" fn atp_diagnostic_raw() -> u64 {
    let bytes = trace_json().into_bytes().into_boxed_slice();
    let len = bytes.len();
    let ptr = Box::leak(bytes).as_ptr() as usize;
    ((ptr as u64) << 32) | len as u64
}

#[cfg(all(test, feature = "atp-diagnostic"))]
mod tests {
    fn row(id: &str) -> serde_json::Value {
        let trace: serde_json::Value = serde_json::from_str(&super::trace_json()).unwrap();
        trace["rows"].as_array().unwrap().iter().find(|r| r["id"] == id).cloned().unwrap_or(serde_json::Value::Null)
    }
    #[test]
    fn m1_order_sensitive_charge_uses_original_private_implementation() {
        assert_eq!(row("roundorder")["output"]["state"][0], "3c9374bd");
    }
    #[test]
    fn inclusive_release_is_observed_in_trace() {
        assert_eq!(row("release-exact")["output"]["released"], true);
        assert_eq!(row("release-below")["output"]["released"], false);
    }
    #[test]
    fn cooldown_crossing_cannot_charge_same_step() {
        assert_eq!(row("cooldown-cross")["output"]["state"][0], "00000000");
        assert_eq!(row("cooldown-next")["output"]["state"][0], "3e4ccccd");
    }
    #[test]
    fn malformed_elapsed_has_exact_refusal() {
        assert_eq!(row("elapsed-nan")["output"]["error"], "Elapsed::NotFinite");
        assert_eq!(row("elapsed-negative")["output"]["error"], "Elapsed::Negative");
    }
    #[test]
    fn trace_is_repeatable_and_bounded() {
        let a = super::trace_json();
        assert_eq!(a, super::trace_json());
        assert!(a.len() > 1000 && a.len() < 65536);
    }
}
