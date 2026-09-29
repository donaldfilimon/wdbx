use crate::{
    neural::Accelerator,
    search::{Search, Sequential},
};
use std::sync::atomic::{AtomicU64, Ordering};

/// Every impure value the kernel needs comes from here.
pub trait Env: Sync {
    fn now_rfc3339(&self) -> String;
    fn now_millis(&self) -> i64;
    fn uid(&self) -> String;
    /// Milliseconds on a monotonic clock; only differences are meaningful.
    fn monotonic_ms(&self) -> f64;
}

/// Deterministic environment for tests and conformance.
#[derive(Debug, Default)]
pub struct FixedEnv {
    next: AtomicU64,
}

impl FixedEnv {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Env for FixedEnv {
    fn now_rfc3339(&self) -> String {
        "2026-01-01T00:00:00.000Z".into()
    }
    fn now_millis(&self) -> i64 {
        1_767_225_600_000
    }
    fn uid(&self) -> String {
        let n = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        format!("00000000-0000-4000-8000-{n:012x}")
    }
    fn monotonic_ms(&self) -> f64 {
        0.0
    }
}

/// Everything impure the engine needs, supplied by the embedding runtime.
pub struct Host<'a> {
    pub env: &'a dyn Env,
    pub search: &'a dyn Search,
    pub accel: Option<&'a dyn Accelerator>,
}

/// Owned deterministic host: `FixedEnv`, sequential search, no accelerator.
#[derive(Default)]
pub struct FixedHost {
    pub env: FixedEnv,
}

impl FixedHost {
    pub fn host(&self) -> Host<'_> {
        Host {
            env: &self.env,
            search: &Sequential,
            accel: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_env_is_deterministic() {
        let a = FixedEnv::new();
        let b = FixedEnv::new();
        assert_eq!(a.now_millis(), 1_767_225_600_000);
        assert_eq!(a.now_rfc3339(), "2026-01-01T00:00:00.000Z");
        assert_eq!(a.monotonic_ms(), 0.0);
        assert_eq!(a.uid(), "00000000-0000-4000-8000-000000000001");
        assert_eq!(a.uid(), "00000000-0000-4000-8000-000000000002");
        assert_eq!(b.uid(), "00000000-0000-4000-8000-000000000001");
    }
}
