//! Independent, deliberately simple ZIP (stored method) byte writer.
//! No production ZIP API is used to construct adversarial metadata.
use std::{collections::BTreeMap, fs, path::Path};

pub const MANIFEST: &[u8] = include_bytes!("../fixtures/portable-archive/manifest.json");
pub const ASSET: &[u8] = include_bytes!("../fixtures/portable-archive/asset.bin");
pub const LEGACY: &[u8] = include_bytes!("../fixtures/portable-archive/legacy-zip4.wdbxspecimen");

#[derive(Clone)]
pub struct Entry {
    pub name: Vec<u8>,
    pub data: Vec<u8>,
    pub size: u32,
    pub mode: u32,
}
impl Entry {
    pub fn new(name: impl AsRef<[u8]>, data: impl AsRef<[u8]>) -> Self {
        Self {
            name: name.as_ref().to_vec(),
            data: data.as_ref().to_vec(),
            size: data.as_ref().len() as u32,
            mode: 0o100600,
        }
    }
}
pub struct Archive {
    pub bytes: Vec<u8>,
    pub local: Vec<usize>,
    pub central: Vec<usize>,
    pub data: Vec<usize>,
}
fn word(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_le_bytes());
}
fn dword(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_le_bytes());
}
pub fn get32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
pub fn set32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}
pub fn archive(entries: &[Entry]) -> Archive {
    let mut bytes = Vec::new();
    let mut local = Vec::new();
    let mut data = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        local.push(bytes.len());
        dword(&mut bytes, 0x04034b50);
        word(&mut bytes, 20);
        word(&mut bytes, 0x800);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        dword(&mut bytes, crc32(&entry.data));
        dword(&mut bytes, entry.data.len() as u32);
        dword(&mut bytes, entry.size);
        word(&mut bytes, entry.name.len() as u16);
        word(&mut bytes, 0);
        bytes.extend(&entry.name);
        data.push(bytes.len());
        bytes.extend(&entry.data);
    }
    let directory = bytes.len();
    for (index, entry) in entries.iter().enumerate() {
        central.push(bytes.len());
        dword(&mut bytes, 0x02014b50);
        word(&mut bytes, 0x0314);
        word(&mut bytes, 20);
        word(&mut bytes, 0x800);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        dword(&mut bytes, crc32(&entry.data));
        dword(&mut bytes, entry.data.len() as u32);
        dword(&mut bytes, entry.size);
        word(&mut bytes, entry.name.len() as u16);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        word(&mut bytes, 0);
        dword(&mut bytes, entry.mode << 16);
        dword(&mut bytes, local[index] as u32);
        bytes.extend(&entry.name);
    }
    let directory_size = bytes.len() - directory;
    dword(&mut bytes, 0x06054b50);
    word(&mut bytes, 0);
    word(&mut bytes, 0);
    word(&mut bytes, entries.len() as u16);
    word(&mut bytes, entries.len() as u16);
    dword(&mut bytes, directory_size as u32);
    dword(&mut bytes, directory as u32);
    word(&mut bytes, 0);
    Archive {
        bytes,
        local,
        central,
        data,
    }
}
/// Decode this fixture's stored members independently of the production ZIP reader.
/// Check local/central metadata, names, sizes and CRCs, while allowing overlaps.
pub fn stored_members(zip: &Archive) -> BTreeMap<String, Vec<u8>> {
    fn get16(bytes: &[u8], offset: usize) -> usize {
        u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize
    }
    let mut members = BTreeMap::new();
    for &central in &zip.central {
        assert_eq!(get32(&zip.bytes, central), 0x02014b50);
        let local = get32(&zip.bytes, central + 42) as usize;
        assert_eq!(get32(&zip.bytes, local), 0x04034b50);
        // Version, flags, method, time/date, CRC and both sizes must agree.
        assert_eq!(
            &zip.bytes[local + 4..local + 26],
            &zip.bytes[central + 6..central + 28]
        );
        assert_eq!(get16(&zip.bytes, local + 8), 0, "stored method only");
        let name_len = get16(&zip.bytes, central + 28);
        assert_eq!(get16(&zip.bytes, local + 26), name_len);
        let name = &zip.bytes[central + 46..central + 46 + name_len];
        assert_eq!(&zip.bytes[local + 30..local + 30 + name_len], name);
        let start = local + 30 + name_len + get16(&zip.bytes, local + 28);
        let size = get32(&zip.bytes, central + 20) as usize;
        assert_eq!(size, get32(&zip.bytes, central + 24) as usize);
        assert!(start + size <= zip.central[0]);
        let data = zip.bytes[start..start + size].to_vec();
        assert_eq!(crc32(&data), get32(&zip.bytes, central + 16));
        assert!(
            members
                .insert(String::from_utf8(name.to_vec()).unwrap(), data)
                .is_none()
        );
    }
    members
}
pub fn valid_entries() -> Vec<Entry> {
    vec![
        Entry::new("manifest.json", MANIFEST),
        Entry::new(format!("assets/{}", specimen_core::digest(ASSET)), ASSET),
    ]
}
/// Pin files, not just directory counts: recovery and content bytes must not move.
pub fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.strip_prefix(root).unwrap().starts_with("store/leases") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
