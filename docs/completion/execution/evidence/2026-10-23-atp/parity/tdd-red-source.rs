//! Standalone private whitebox diagnostic; never registered in the kernel.
#[cfg(feature = "atp-diagnostic")]
pub fn trace_json() -> String {
    "{\"rows\":[]}".to_owned()
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
