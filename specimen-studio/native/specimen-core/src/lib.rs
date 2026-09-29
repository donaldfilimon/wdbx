pub mod engine;
pub mod language;
pub mod models;
pub mod neural;
pub mod persistence;
pub mod scheduler;
pub mod vision;
pub use specimen_kernel::{Error, Result, digest, error};
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub mod protocol;
