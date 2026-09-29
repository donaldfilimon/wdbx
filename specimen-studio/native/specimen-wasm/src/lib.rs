//! P0 feasibility probe: one full specimen cycle over a raw ABI.
//! Replaced by a wasm-bindgen command surface in P1.
use serde_json::{Value, json};
use specimen_kernel::{engine, host::FixedHost, neural::Network};
use std::sync::atomic::AtomicBool;

/// Runs `{"specimen": ..., "input": "..."}` through one cycle on a fixed host.
/// Returns `{"specimen", "cycle"}` or `{"error": {"code", "message"}}`.
pub fn probe(input: &str) -> String {
    let run = || -> specimen_kernel::Result<Value> {
        let request: Value = serde_json::from_str(input)?;
        let fixed = FixedHost::default();
        let (specimen, cycle) = engine::cycle(
            &fixed.host(),
            &request["specimen"],
            request["input"].as_str().unwrap_or_default(),
            &Network::default(),
            &AtomicBool::new(false),
            &|_| {},
        )?;
        Ok(json!({ "specimen": specimen, "cycle": cycle }))
    };
    match run() {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "error": { "code": e.code, "message": e.message } }).to_string(),
    }
}

/// Reserves `len` bytes for the caller to fill before `probe_raw`.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Takes ownership of the input buffer and returns the output as
/// `(pointer << 32) | length`. The output buffer is intentionally leaked.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` initialized bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_raw(ptr: *mut u8, len: usize) -> u64 {
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    let out = probe(&String::from_utf8_lossy(&bytes))
        .into_bytes()
        .into_boxed_slice();
    let (p, n) = (out.as_ptr() as u64, out.len() as u64);
    std::mem::forget(out);
    (p << 32) | n
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> String {
        let starter: Value =
            serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap();
        json!({ "specimen": starter, "input": "What is 2 + 2?" }).to_string()
    }
    #[test]
    fn native_probe_matches_golden() {
        let got = specimen_kernel::digest(probe(&request()).as_bytes());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/p0-probe.sha256");
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(path, format!("{got}\n")).unwrap();
        }
        assert_eq!(got, std::fs::read_to_string(path).unwrap().trim());
    }
    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        let out: Value = serde_json::from_str(&probe("{not json")).unwrap();
        assert_eq!(out["error"]["code"], "MalformedSave");
    }
}
