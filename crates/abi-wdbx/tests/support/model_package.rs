#![allow(dead_code)]
use abi_wdbx::model::{ValidatedModelState, limits::LoadLimits, package::*};
use sha2::{Digest as _, Sha256};
pub fn fixture(name: &str) -> Vec<u8> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/model-v1")
        .join(format!("{name}.hex"));
    let s = std::fs::read_to_string(p).unwrap();
    s.trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
pub fn load(raw: &[u8]) -> Result<ValidatedModelState, abi_wdbx::model::error::ModelError> {
    let l = LoadLimits::default();
    validate_model(validate_framing(read_bounded_snapshot(raw, l)?, l)?, l)
}
pub fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
pub fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
pub fn component(raw: &[u8], kind: usize) -> &[u8] {
    let p = 96 + (kind - 1) * 80;
    let start = u64_at(raw, p + 16) as usize;
    let len = u64_at(raw, p + 24) as usize;
    &raw[start..start + len]
}
// An independent wire worksheet for adversarial semantic changes. Rehashes bytes,
// never calls the production serializer, so semantic tests do not stop at SHA.
pub fn replace(raw: &[u8], kind: usize, payload: &[u8]) -> Vec<u8> {
    let count = u32_at(raw, 24) as usize;
    let end = 96 + 80 * count;
    let mut out = raw[..end].to_vec();
    for i in 1..=count {
        let bytes = if i == kind {
            payload
        } else {
            component(raw, i)
        };
        let start = out.len();
        out.extend_from_slice(bytes);
        let p = 96 + (i - 1) * 80;
        out[p + 16..p + 24].copy_from_slice(&(start as u64).to_le_bytes());
        out[p + 24..p + 32].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        out[p + 32..p + 64].copy_from_slice(&Sha256::digest(bytes));
    }
    let n = out.len() as u64;
    out[16..24].copy_from_slice(&n.to_le_bytes());
    let h = Sha256::digest(&out[96..end]);
    out[32..64].copy_from_slice(&h);
    out
}
pub fn directory_hash(raw: &mut [u8]) {
    let end = 96 + 80 * u32_at(raw, 24) as usize;
    let hash = Sha256::digest(&raw[96..end]);
    raw[32..64].copy_from_slice(&hash);
}
pub fn temporary() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "wdbx-n02-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir(&p).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    p
}
