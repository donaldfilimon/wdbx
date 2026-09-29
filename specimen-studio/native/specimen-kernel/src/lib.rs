//! Pure specimen rules. Every impure service arrives through `host`.
pub mod host;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(code: &str, message: impl ToString) -> Error {
    Error {
        code: code.into(),
        message: message.to_string(),
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        error("Storage", e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        error("MalformedSave", e)
    }
}
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
