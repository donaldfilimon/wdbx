//! One reportable failure: a stable name, a detail without secrets, an exit code.

#[derive(Debug)]
pub(crate) struct TypedError {
    pub(crate) name: &'static str,
    pub(crate) detail: String,
    pub(crate) code: u8,
}

impl TypedError {
    pub(crate) fn new(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            detail: detail.into(),
            code: 1,
        }
    }

    pub(crate) fn usage(detail: &str) -> Self {
        Self {
            code: 2,
            ..Self::new("Usage", detail)
        }
    }

    /// Optimistic-concurrency refusal: the caller re-inspects and retries.
    pub(crate) fn stale(detail: impl Into<String>) -> Self {
        Self {
            code: 3,
            ..Self::new("StaleFrontier", detail)
        }
    }

    /// Another typed writer holds the store lock (`EX_TEMPFAIL`).
    pub(crate) fn writer_busy() -> Self {
        Self {
            code: 75,
            ..Self::new("WriterBusy", "another typed writer holds this store")
        }
    }
}
