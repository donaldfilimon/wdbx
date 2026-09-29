//! The specimen kernel for the browser: a JSON command ABI (`call`) plus the
//! P0 fixed-host probe that the conformance goldens pin.
pub mod dispatch;
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

#[cfg(target_arch = "wasm32")]
mod exports {
    //! The WebAssembly ABI. Buffers cross as `(pointer << 32) | length`
    //! (wasm32 pointers are 32-bit); every buffer handed to JS is released by
    //! JS through `free`.
    use crate::dispatch::{reply, rfc3339, uuid_v4};
    use specimen_kernel::{
        host::{Env, Host},
        search::Sequential,
    };

    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn now_ms() -> f64;
        fn random_u32() -> u32;
        fn progress(ptr: *const u8, len: usize);
    }

    struct WasmEnv {
        start: f64,
    }

    impl Env for WasmEnv {
        fn now_rfc3339(&self) -> String {
            rfc3339(self.now_millis())
        }
        fn now_millis(&self) -> i64 {
            unsafe { now_ms() as i64 }
        }
        fn uid(&self) -> String {
            unsafe { uuid_v4([random_u32(), random_u32(), random_u32(), random_u32()]) }
        }
        fn monotonic_ms(&self) -> f64 {
            unsafe { now_ms() - self.start }
        }
    }

    fn hand_out(bytes: Vec<u8>) -> u64 {
        let out = bytes.into_boxed_slice();
        let (p, n) = (out.as_ptr() as u64, out.len() as u64);
        std::mem::forget(out);
        (p << 32) | n
    }

    /// Reserves exactly `len` bytes for the caller to fill.
    #[unsafe(no_mangle)]
    pub extern "C" fn alloc(len: usize) -> *mut u8 {
        Box::into_raw(vec![0u8; len].into_boxed_slice()).cast()
    }

    /// Releases a buffer from `alloc` or one returned by `call`/`probe_raw`.
    ///
    /// # Safety
    /// `ptr`/`len` must describe a buffer this module handed out, once.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn free(ptr: *mut u8, len: usize) {
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
    }

    fn take(ptr: *mut u8, len: usize) -> String {
        let bytes = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) };
        String::from_utf8(bytes.into_vec()).unwrap_or_default()
    }

    /// Runs one JSON command (see `dispatch`); trace steps stream through the
    /// imported `progress`. Takes ownership of the input buffer.
    ///
    /// # Safety
    /// `ptr` must come from `alloc(len)` and hold UTF-8 JSON.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn call(ptr: *mut u8, len: usize) -> u64 {
        let input = take(ptr, len);
        let env = WasmEnv {
            start: unsafe { now_ms() },
        };
        let host = Host {
            env: &env,
            search: &Sequential,
            accel: None,
        };
        let out = reply(&host, &input, &|step| {
            let text = step.to_string();
            unsafe { progress(text.as_ptr(), text.len()) };
        });
        hand_out(out.into_bytes())
    }

    /// P0 probe on a fixed host (conformance goldens).
    ///
    /// # Safety
    /// `ptr` must come from `alloc(len)` and hold UTF-8 JSON.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn probe_raw(ptr: *mut u8, len: usize) -> u64 {
        hand_out(crate::probe(&take(ptr, len)).into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn starter() -> Value {
        serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
    }
    /// Arithmetic only: no neural run, no visual trigonometry, `exp` of a huge
    /// negative number (`lastUpdate` 0) that underflows to exactly zero.
    fn calc_request() -> String {
        json!({ "specimen": starter(), "input": "What is 2 + 2?" }).to_string()
    }
    /// Reaches `f64::sin`/`cos` (visual arrays), `f32::exp` (sigmoid layer) and
    /// a finite `f64::exp` decay (`lastUpdate` 61.234 s before `FixedEnv` now).
    fn imagine_request() -> String {
        let mut s = starter();
        s["atp"]["lastUpdate"] = json!(1_767_225_538_766_i64);
        json!({ "specimen": s, "input": "imagine a happy blue square" }).to_string()
    }
    /// The `^` operator (`powf`) plus the finite `exp` decay.
    fn pow_request() -> String {
        let mut s = starter();
        s["atp"]["lastUpdate"] = json!(1_767_225_538_766_i64);
        json!({ "specimen": s, "input": "2 ^ 0.5" }).to_string()
    }
    fn check_golden(request: &str, file: &str) {
        let got = specimen_kernel::digest(probe(request).as_bytes());
        let path = format!("{}/../../conformance/{file}", env!("CARGO_MANIFEST_DIR"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, format!("{got}\n")).unwrap();
        }
        assert_eq!(got, std::fs::read_to_string(&path).unwrap().trim());
    }
    #[test]
    fn native_probe_matches_golden() {
        check_golden(&calc_request(), "p0-probe.sha256");
    }
    #[test]
    fn native_imagine_probe_matches_golden() {
        check_golden(&imagine_request(), "p0-probe-imagine.sha256");
    }
    #[test]
    fn native_pow_probe_matches_golden() {
        check_golden(&pow_request(), "p0-probe-pow.sha256");
    }
    #[test]
    fn pow_case_evaluates_a_fractional_power() {
        let out: Value = serde_json::from_str(&probe(&pow_request())).unwrap();
        let text = out["cycle"]["segments"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("1.41421356"), "got {text}");
    }
    #[test]
    fn imagine_case_exercises_transcendentals() {
        let out: Value = serde_json::from_str(&probe(&imagine_request())).unwrap();
        let visual = &out["cycle"]["visual"];
        assert!(visual["yArray"].as_array().is_some_and(|a| !a.is_empty()));
        assert!(
            visual["synthesis"]["activationFunctions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "sigmoid")
        );
        let valence = out["specimen"]["atp"]["valence"].as_f64().unwrap();
        assert!(
            valence > 0.0 && valence != 0.2,
            "decay must be finite and applied"
        );
    }
    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        let out: Value = serde_json::from_str(&probe("{not json")).unwrap();
        assert_eq!(out["error"]["code"], "MalformedSave");
    }
}
