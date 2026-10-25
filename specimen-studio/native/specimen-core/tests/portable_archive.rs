#[path = "support/portable_archive.rs"]
mod support;
use specimen_core::persistence::Store;
use std::{fs, process::Command};
use support::*;
// The independent Python witness must not inherit another test's live store lock.
static STORE_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn refuses_without_writes(bytes: &[u8], code: Option<&str>) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let sentinel = temp.path().join("sentinel");
    fs::write(&sentinel, b"outside workspace").unwrap();
    let mut store = Store::open(&root).unwrap();
    let starter = serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap();
    store.edit(0, starter).unwrap();
    store.put_asset(b"existing retained asset").unwrap();
    fs::create_dir_all(root.join("recovery")).unwrap();
    fs::write(root.join("recovery/retained.bin"), b"previous recovery").unwrap();
    let before = serde_json::to_value(&store.snapshot).unwrap();
    let disk = files(&root);
    let path = temp.path().join("hostile.wdbxspecimen");
    fs::write(&path, bytes).unwrap();
    let result = store.import(&path, 1);
    assert!(result.is_err(), "malformed archive unexpectedly imported");
    if let Some(code) = code {
        assert_eq!(result.unwrap_err().code, code);
    }
    assert_eq!(serde_json::to_value(&store.snapshot).unwrap(), before);
    assert_eq!(files(&root), disk);
    drop(store);
    let reopened = Store::open(&root).unwrap();
    assert_eq!(serde_json::to_value(&reopened.snapshot).unwrap(), before);
    assert_eq!(files(&root), disk);
    assert_eq!(fs::read(sentinel).unwrap(), b"outside workspace");
}
#[test]
fn actual_zip4_fixture_and_zip9_export_preserve_semantics() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path().join("workspace")).unwrap();
    let path = temp.path().join("legacy.wdbxspecimen");
    fs::write(&path, LEGACY).unwrap();
    let restored = store.import(&path, 0).unwrap();
    let expected_snapshot: specimen_core::persistence::Snapshot =
        serde_json::from_slice(MANIFEST).unwrap();
    let expected = serde_json::to_value(expected_snapshot).unwrap();
    for (key, value) in expected.as_object().unwrap() {
        assert!(
            serde_json::to_value(&restored).unwrap()[key] == *value,
            "legacy snapshot field differs: {key}"
        );
    }
    assert_eq!(restored.schema, "wdbx.native.v2");
    assert_eq!(restored.specimen["name"], "Café 東京 — portable legacy");
    assert_eq!(
        restored.specimen["settings"]["jitter"]
            .as_f64()
            .unwrap()
            .to_bits(),
        1.8688716782771921_f64.to_bits()
    );
    let id = specimen_core::digest(ASSET);
    assert_eq!(store.asset(&id).unwrap(), ASSET);
    let output = temp.path().join("zip9.wdbxspecimen");
    store.export(&output).unwrap();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/portable-archive");
    // Independent standard-library reader compares decompressed bytes, not ZIP encoding.
    let status = Command::new("python3")
        .arg("-c")
        .arg(r#"
import sys,zipfile,pathlib,json,hashlib
p=pathlib.Path(sys.argv[2]); asset=(p/'asset.bin').read_bytes()
with zipfile.ZipFile(sys.argv[1]) as z:
 assert z.testzip() is None
 assert sorted(z.namelist()) == sorted(['manifest.json','assets/'+hashlib.sha256(asset).hexdigest()])
 assert z.read('manifest.json') == (p/'manifest.json').read_bytes()
 assert json.loads(z.read('manifest.json')) == json.loads((p/'manifest.json').read_bytes())
 assert z.read('assets/'+hashlib.sha256(asset).hexdigest()) == asset
"#)
        .arg(output)
        .arg(fixture)
        .status()
        .unwrap();
    assert!(status.success());
    drop(store);
    let reopened = Store::open(temp.path().join("workspace")).unwrap();
    assert_eq!(serde_json::to_value(&reopened.snapshot).unwrap(), expected);
}
#[test]
fn duplicate_names_fail_even_when_payloads_match() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for index in [0, 1] {
        for differing in [false, true] {
            let mut entries = valid_entries();
            let mut duplicate = entries[index].clone();
            if differing {
                duplicate.data = b"different payload".to_vec();
                duplicate.size = duplicate.data.len() as u32;
            }
            entries.push(duplicate);
            let zip = archive(&entries);
            assert_eq!(entries[index].name, entries[2].name);
            assert_ne!(zip.local[index], zip.local[2]);
            assert_eq!(get32(&zip.bytes, zip.central[2]), 0x02014b50);
            refuses_without_writes(&zip.bytes, Some("MalformedSave"));
        }
    }
}
#[test]
fn missing_manifest_asset_corrupt_hash_and_unknown_member_refuse() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut cases = vec![
        vec![valid_entries()[1].clone()],
        vec![valid_entries()[0].clone()],
    ];
    let mut corrupt = valid_entries();
    corrupt[1].data[0] ^= 1;
    cases.push(corrupt);
    let mut unknown = valid_entries();
    unknown.push(Entry::new("unknown", b"payload"));
    cases.push(unknown);
    for entries in cases {
        refuses_without_writes(&archive(&entries).bytes, Some("MalformedSave"));
    }
}
#[test]
fn local_central_name_and_size_mismatches_refuse() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut name = archive(&valid_entries());
    name.bytes[name.local[0] + 30] = b'M';
    assert_ne!(
        name.bytes[name.local[0] + 30],
        name.bytes[name.central[0] + 46]
    );
    refuses_without_writes(&name.bytes, Some("MalformedSave"));
    let mut size = archive(&valid_entries());
    let old = get32(&size.bytes, size.local[0] + 22);
    set32(&mut size.bytes, size.local[0] + 22, old + 1);
    assert_ne!(
        get32(&size.bytes, size.local[0] + 22),
        get32(&size.bytes, size.central[0] + 24)
    );
    refuses_without_writes(&size.bytes, Some("MalformedSave"));
}
#[test]
fn overlapping_members_with_matching_headers_refuse() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let nested = archive(&[Entry::new("manifest.json", MANIFEST)]);
    let nested_local = &nested.bytes[..nested.central[0]];
    let mut payload = b"prefix".to_vec();
    payload.extend(nested_local);
    payload.extend(b"suffix");
    let embedded_asset = format!("assets/{}", specimen_core::digest(&payload));
    let referenced_asset = format!("assets/{}", specimen_core::digest(ASSET));
    let entries = vec![
        Entry::new(&embedded_asset, &payload),
        Entry::new("manifest.json", MANIFEST),
        Entry::new(&referenced_asset, ASSET),
    ];
    let mut zip = archive(&entries);
    let control_bytes = zip.bytes.clone();
    let control_members = stored_members(&zip);
    assert_eq!(control_members.len(), 3);
    assert_eq!(control_members["manifest.json"], MANIFEST);
    assert_eq!(control_members[&referenced_asset], ASSET);
    assert_eq!(control_members[&embedded_asset], payload);
    let manifest: serde_json::Value =
        serde_json::from_slice(&control_members["manifest.json"]).unwrap();
    assert_eq!(
        manifest["specimen"]["nodes"][0]["provenance"]["asset"],
        specimen_core::digest(ASSET)
    );
    // All original local records are disjoint, including the referenced asset.
    for (i, entry) in entries.iter().enumerate().take(entries.len() - 1) {
        assert_eq!(zip.data[i] + entry.data.len(), zip.local[i + 1]);
    }
    // The non-overlapping form must be semantically importable first.
    let control = tempfile::tempdir().unwrap();
    let control_path = control.path().join("control.wdbxspecimen");
    fs::write(&control_path, &zip.bytes).unwrap();
    let mut control_store = Store::open(control.path().join("workspace")).unwrap();
    let imported = control_store
        .import(&control_path, 0)
        .expect("non-overlapping control must import before redirecting the manifest offset");
    assert_eq!(imported.specimen, manifest["specimen"]);
    assert_eq!(
        control_store.asset(&specimen_core::digest(ASSET)).unwrap(),
        ASSET
    );
    assert_eq!(
        control_store
            .asset(&specimen_core::digest(&payload))
            .unwrap(),
        payload
    );
    drop(control_store);

    let nested_offset = zip.data[0] + 6;
    let offset_field = zip.central[1] + 42;
    assert_eq!(get32(&zip.bytes, offset_field), zip.local[1] as u32);
    set32(&mut zip.bytes, offset_field, nested_offset as u32);
    // Only this central offset changes; every independently checked name,
    // header field, CRC and semantic member payload remains identical.
    assert_eq!(&zip.bytes[..offset_field], &control_bytes[..offset_field]);
    assert_eq!(
        &zip.bytes[offset_field + 4..],
        &control_bytes[offset_field + 4..]
    );
    assert_eq!(stored_members(&zip), control_members);
    assert!(nested_offset > zip.data[0]);
    assert!(nested_offset + nested_local.len() < zip.data[0] + payload.len());
    refuses_without_writes(&zip.bytes, Some("MalformedSave"));
}
#[test]
fn invalid_utf8_truncation_crc_and_streamed_overrun_refuse() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut invalid = archive(&valid_entries());
    invalid.bytes[invalid.local[0] + 30] = 0xff;
    invalid.bytes[invalid.central[0] + 46] = 0xff;
    assert!(
        std::str::from_utf8(&invalid.bytes[invalid.local[0] + 30..invalid.local[0] + 43]).is_err()
    );
    assert_eq!(
        &invalid.bytes[invalid.central[0] + 8..invalid.central[0] + 10],
        &0x800u16.to_le_bytes()
    );
    refuses_without_writes(&invalid.bytes, Some("MalformedSave"));
    let valid = archive(&valid_entries());
    refuses_without_writes(
        &valid.bytes[..valid.bytes.len() - 12],
        Some("MalformedSave"),
    );
    let mut truncated = archive(&valid_entries());
    let central_start = truncated.central[0];
    truncated.bytes.drain(central_start - 3..central_start);
    let end = truncated.bytes.len() - 22;
    set32(&mut truncated.bytes, end + 16, (central_start - 3) as u32);
    assert_eq!(get32(&truncated.bytes, central_start - 3), 0x02014b50);
    assert!(truncated.data[1] + ASSET.len() > central_start - 3);
    refuses_without_writes(&truncated.bytes, Some("MalformedSave"));
    let mut crc = archive(&valid_entries());
    crc.bytes[crc.data[1]] ^= 1;
    assert_ne!(
        crc32(&crc.bytes[crc.data[1]..crc.data[1] + ASSET.len()]),
        get32(&crc.bytes, crc.central[1] + 16)
    );
    refuses_without_writes(&crc.bytes, None);
    let overrun = include_bytes!("fixtures/portable-archive/streamed-overrun.zip");
    assert_eq!(get32(overrun, 22), 1);
    assert_eq!(get32(overrun, 0), 0x04034b50);
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/portable-archive/streamed-overrun.zip");
    assert!(
        Command::new("python3")
            .arg("-c")
            .arg(
                r#"
import sys,struct,zlib,pathlib
b=pathlib.Path(sys.argv[1]).read_bytes()
n=struct.unpack_from('<H',b,26)[0]; c=struct.unpack_from('<I',b,18)[0]
d=zlib.decompress(b[30+n:30+n+c],-15)
assert len(d)==32*1024*1024+1 and d==b'Z'*len(d)
"#
            )
            .arg(fixture)
            .status()
            .unwrap()
            .success()
    );
    refuses_without_writes(overrun, None);
}
#[test]
fn unsafe_names_cannot_touch_external_sentinel() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let outside = tempfile::tempdir().unwrap();
    let sentinel = outside.path().join("sentinel");
    fs::write(&sentinel, b"untouched").unwrap();
    let names = vec![
        b"../sentinel".to_vec(),
        sentinel.to_string_lossy().as_bytes().to_vec(),
        b"/absolute".to_vec(),
        b"C:\\sentinel".to_vec(),
        b"assets\\bad".to_vec(),
        b"manifest.json\0suffix".to_vec(),
        format!("assets/{}", "G".repeat(64)).into_bytes(),
        format!("assets/{}", "A".repeat(64)).into_bytes(),
        b"assets/abc".to_vec(),
    ];
    for name in names {
        let mut entries = valid_entries();
        entries.push(Entry::new(name, b"overwrite"));
        refuses_without_writes(&archive(&entries).bytes, Some("MalformedSave"));
        assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 1);
}
#[test]
fn symlink_and_special_modes_are_inert_bytes() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for mode in [0o120777, 0o010777] {
        let temp = tempfile::tempdir().unwrap();
        let sentinel = temp.path().join("outside");
        fs::write(&sentinel, b"external content").unwrap();
        let payload = sentinel.to_string_lossy().as_bytes().to_vec();
        let id = specimen_core::digest(&payload);
        let mut manifest: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
        manifest["specimen"]["nodes"][0]["provenance"]["asset"] = serde_json::json!(id);
        let mut entry = Entry::new(format!("assets/{id}"), &payload);
        entry.mode = mode;
        let zip = archive(&[
            Entry::new("manifest.json", serde_json::to_vec(&manifest).unwrap()),
            entry,
        ]);
        assert_eq!(get32(&zip.bytes, zip.central[1] + 38) >> 16, mode);
        let path = temp.path().join("mode.wdbxspecimen");
        fs::write(&path, zip.bytes).unwrap();
        let mut store = Store::open(temp.path().join("workspace")).unwrap();
        store.import(&path, 0).unwrap();
        assert_eq!(store.asset(&id).unwrap(), payload);
        let metadata = fs::symlink_metadata(store.root().join("assets").join(id)).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(metadata.permissions().mode() & 0o111, 0);
        }
        assert_eq!(fs::read(sentinel).unwrap(), b"external content");
    }
}
#[test]
fn declared_limits_and_entry_count_refuse_before_payload_writes() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (index, size) in [(0, 64 * 1024 * 1024 + 1), (1, 32 * 1024 * 1024 + 1)] {
        let mut entries = valid_entries();
        entries[index].size = size;
        let zip = archive(&entries);
        assert_eq!(get32(&zip.bytes, zip.central[index] + 24), size);
        assert_eq!(get32(&zip.bytes, zip.local[index] + 22), size);
        refuses_without_writes(&zip.bytes, Some("BudgetExceeded"));
    }
    let entries: Vec<_> = (0..10002)
        .map(|i| Entry::new(format!("assets/{i:064x}"), b""))
        .collect();
    let zip = archive(&entries);
    assert_eq!(zip.central.len(), 10002);
    refuses_without_writes(&zip.bytes, Some("BudgetExceeded"));
}
#[test]
fn stale_import_and_failed_export_preserve_existing_archive() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let mut store = Store::open(&root).unwrap();
    let path = temp.path().join("save.wdbxspecimen");
    fs::write(&path, LEGACY).unwrap();
    store.import(&path, 0).unwrap();
    let before = files(&root);
    let snapshot = serde_json::to_value(&store.snapshot).unwrap();
    assert_eq!(store.import(&path, 0).unwrap_err().code, "StaleRevision");
    assert_eq!(files(&root), before);
    store.export(&path).unwrap();
    let successful = fs::read(&path).unwrap();
    fs::remove_file(root.join("assets").join(specimen_core::digest(ASSET))).unwrap();
    assert!(store.export(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), successful);
    assert!(!fs::read_dir(temp.path()).unwrap().any(|e| {
        e.unwrap()
            .path()
            .extension()
            .is_some_and(|e| e == "partial")
    }));
    drop(store);
    let reopened = Store::open(root).unwrap();
    assert_eq!(serde_json::to_value(&reopened.snapshot).unwrap(), snapshot);
}

#[test]
fn oversized_input_is_refused_without_reading_sparse_payload() {
    let _guard = STORE_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let mut store = Store::open(&root).unwrap();
    let specimen = serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap();
    store.edit(0, specimen).unwrap();
    let before = files(&root);
    let snapshot = serde_json::to_value(&store.snapshot).unwrap();
    let path = temp.path().join("sparse.wdbxspecimen");
    fs::File::create(&path)
        .unwrap()
        .set_len(512 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(store.import(&path, 1).unwrap_err().code, "BudgetExceeded");
    assert_eq!(files(&root), before);
    drop(store);
    let reopened = Store::open(root).unwrap();
    assert_eq!(serde_json::to_value(&reopened.snapshot).unwrap(), snapshot);
}

#[test]
fn captured_fixture_bytes_match_the_pinned_provenance() {
    assert_eq!(crc32(b"123456789"), 0xcbf43926);
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/portable-archive/provenance.json")).unwrap();
    for (name, bytes) in [
        ("legacy-zip4.wdbxspecimen", LEGACY),
        ("manifest.json", MANIFEST),
        ("asset.bin", ASSET),
        (
            "streamed-overrun.zip",
            include_bytes!("fixtures/portable-archive/streamed-overrun.zip").as_slice(),
        ),
    ] {
        assert_eq!(
            specimen_core::digest(bytes),
            provenance["fixtures_sha256"][name]
        );
    }
    assert_eq!(
        specimen_core::digest(include_bytes!("fixtures/portable-archive/producer.rs")),
        provenance["producer_sha256"]
    );
}
