use serde_json::{Value, json};
use specimen_core::persistence::Store;
use std::{fs, io::Read, path::PathBuf};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let output = PathBuf::from(&args[1]);
    let scratch = PathBuf::from(&args[2]);
    let mut store = Store::open(scratch).unwrap();
    let mut specimen: Value = serde_json::from_str(&fs::read_to_string(&args[3]).unwrap()).unwrap();
    specimen["name"] = json!("Café 東京 — portable legacy");
    specimen["settings"]["jitter"] = json!(1.8688716782771921_f64);
    let asset = b"ZIP4 baseline asset: caf\xc3\xa9 \xe6\x9d\xb1\xe4\xba\xac\n";
    let id = store.put_asset(asset).unwrap();
    specimen["nodes"][0]["provenance"] =
        json!({"asset":id,"source":"isolated ZIP4 compatibility fixture"});
    store.edit(0, specimen).unwrap();
    store
        .export(&output.join("legacy-zip4.wdbxspecimen"))
        .unwrap();
    let mut archive =
        zip::ZipArchive::new(fs::File::open(output.join("legacy-zip4.wdbxspecimen")).unwrap())
            .unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    fs::write(output.join("manifest.json"), manifest).unwrap();
    fs::write(output.join("asset.bin"), asset).unwrap();
    println!("asset={id}; ZIP4 entries={}", archive.len());
}
