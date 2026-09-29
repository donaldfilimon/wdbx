use crate::{Result, engine, error, neural::Network};
use abi_wdbx::v2::{V2Mutation, V2Store};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema: String,
    pub revision: u64,
    pub specimen: Value,
    pub network: Network,
    pub visuals: Vec<Value>,
    pub artifacts: Vec<Value>,
    pub tombstones: Vec<Value>,
    pub native_ids: BTreeMap<String, String>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            schema: "wdbx.native.v2".into(),
            revision: 0,
            specimen: Value::Null,
            network: Network::default(),
            visuals: Vec::new(),
            artifacts: Vec::new(),
            tombstones: Vec::new(),
            native_ids: BTreeMap::new(),
        }
    }
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "wdbx.native.v2" {
            return Err(error("MalformedSave", "Unsupported native save version"));
        }
        engine::validate(&self.specimen)?;
        self.network.validate()?;
        if self.visuals.len() > 10000 || self.artifacts.len() > 10000 {
            return Err(error("BudgetExceeded", "Too many visual records"));
        }
        for visual in &self.visuals {
            serde_json::from_value::<crate::vision::Analysis>(visual["analysis"].clone())?
                .validate()?;
            if !engine::rows(&self.specimen, "nodes")
                .iter()
                .any(|n| n["ref"] == visual["nodeRef"])
            {
                return Err(error(
                    "MalformedSave",
                    "Visual record refers to a missing node",
                ));
            }
        }
        for record in self.visuals.iter().chain(&self.artifacts) {
            if let Some(asset) = record["asset"].as_str() {
                check_digest(asset)?;
            }
        }
        Ok(())
    }
    fn asset_ids(&self) -> Result<std::collections::BTreeSet<String>> {
        fn collect(value: &Value, ids: &mut std::collections::BTreeSet<String>) -> Result<()> {
            match value {
                Value::Object(values) => {
                    for (key, value) in values {
                        if key == "asset"
                            && let Some(id) = value.as_str()
                        {
                            check_digest(id)?;
                            ids.insert(id.into());
                        }
                        collect(value, ids)?;
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        collect(value, ids)?;
                    }
                }
                _ => (),
            }
            Ok(())
        }
        let mut ids = std::collections::BTreeSet::new();
        collect(&serde_json::to_value(self)?, &mut ids)?;
        Ok(ids)
    }
    fn reindex(&mut self) {
        self.native_ids.clear();
        for n in engine::rows(&self.specimen, "nodes") {
            for e in engine::rows(n, "entries") {
                self.native_ids.insert(
                    engine::text(e, "id").into(),
                    crate::language::identify(engine::text(e, "pattern")).id,
                );
            }
        }
        for visual in &self.visuals {
            if let Some(node) = engine::rows(&self.specimen, "nodes")
                .iter()
                .find(|node| node["ref"] == visual["nodeRef"])
            {
                for entry in engine::rows(node, "entries") {
                    self.native_ids.insert(
                        engine::text(entry, "id").into(),
                        engine::text(&visual["analysis"], "patternId").into(),
                    );
                }
            }
        }
        for r in engine::rows(&self.specimen, "resources") {
            self.native_ids.insert(
                engine::text(r, "ref").into(),
                crate::language::identify(engine::text(r, "text")).id,
            );
        }
    }
}
pub struct Store {
    root: PathBuf,
    store: V2Store,
    pub snapshot: Snapshot,
}
impl Store {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("assets"))?;
        let store = V2Store::open(root.join("store")).map_err(|e| error("Storage", e))?;
        let snapshot = store
            .snapshot()
            .preferred_value("studio/snapshot")
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default();
        let instance = Self {
            root,
            store,
            snapshot,
        };
        if instance.snapshot.revision > 0 {
            instance.snapshot.validate()?;
        }
        Ok(instance)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn commit(&mut self, expected: u64, mut next: Snapshot) -> Result<Snapshot> {
        if expected != self.snapshot.revision {
            return Err(error(
                "StaleRevision",
                "Workspace changed; refresh before retrying",
            ));
        }
        next.validate()?;
        next.reindex();
        next.revision = expected
            .checked_add(1)
            .ok_or_else(|| error("BudgetExceeded", "Revision counter exhausted"))?;
        let data = serde_json::to_string(&next)?;
        if data.len() > 64 * 1024 * 1024 {
            return Err(error("BudgetExceeded", "Snapshot exceeds 64 MiB"));
        }
        self.store
            .commit(vec![V2Mutation::PutKv {
                key: "studio/snapshot".into(),
                value: data,
            }])
            .map_err(|e| error("Storage", e))?;
        self.snapshot = next;
        if self.snapshot.revision.is_multiple_of(50) {
            let _ = self.store.compact();
        }
        Ok(self.snapshot.clone())
    }
    pub fn edit(&mut self, expected: u64, specimen: Value) -> Result<Snapshot> {
        let mut next = self.snapshot.clone();
        engine::validate(&specimen)?;
        // Lifetime mutation records cannot be erased by ordinary edits or deletion.
        for m in engine::rows(&next.specimen, "mutations") {
            if !engine::rows(&specimen, "mutations").iter().any(|x| x == m) {
                return Err(error(
                    "InvalidEdit",
                    "An edit cannot erase mutation provenance",
                ));
            }
        }
        for n in engine::rows(&next.specimen, "nodes") {
            if !engine::rows(&specimen, "nodes")
                .iter()
                .any(|v| v["ref"] == n["ref"])
            {
                next.tombstones
                    .push(json!({"node":n,"deletedAt":crate::now()}));
            }
        }
        for m in engine::rows(&specimen, "mutations") {
            for n in engine::rows(&specimen, "nodes") {
                for e in engine::rows(n, "entries") {
                    for a in engine::rows(e, "alternatives") {
                        if (a["id"] == m["slot"] && a["remixed"] != true)
                            || (e["pattern"] == m["originalPattern"]
                                && a["action"] == m["originalAction"])
                        {
                            return Err(error(
                                "InvalidEdit",
                                "An edit cannot restore an inhibited original pairing or clear a remix marker",
                            ));
                        }
                    }
                }
            }
        }
        next.visuals.retain(|v| {
            engine::rows(&specimen, "nodes")
                .iter()
                .any(|n| n["ref"] == v["nodeRef"])
        });
        next.specimen = specimen;
        self.commit(expected, next)
    }
    /// Reclaim expired unreferenced assets only when no recovery snapshot needs them.
    pub fn cleanup(&self, cancel: &std::sync::atomic::AtomicBool) -> Result<usize> {
        use std::sync::atomic::Ordering;
        fn assets(value: &Value, keep: &mut std::collections::BTreeSet<String>) {
            match value {
                Value::Object(map) => {
                    for (key, value) in map {
                        if key == "asset"
                            && let Some(id) = value.as_str()
                        {
                            keep.insert(id.to_owned());
                        }
                        assets(value, keep);
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        assets(value, keep);
                    }
                }
                _ => (),
            }
        }
        let mut keep = std::collections::BTreeSet::new();
        assets(&serde_json::to_value(&self.snapshot)?, &mut keep);
        // Recovery imports may refer to assets not embedded in a JSON file. Keep asset files
        // conservatively while recovery copies exist; partial scratch files remain eligible.
        let recovery = self.root.join("recovery");
        let recovery_exists = recovery.is_dir() && fs::read_dir(recovery)?.next().is_some();
        let retention = engine::number(
            &self.snapshot.specimen["settings"],
            "scratchRetentionSeconds",
            86400.,
        )
        .clamp(3600., 31536000.);
        let mut removed = 0;
        for directory in [self.root.join("assets"), self.root.join("models")] {
            if !directory.is_dir() {
                continue;
            }
            for entry in fs::read_dir(directory)? {
                if cancel.load(Ordering::Relaxed) {
                    return Err(error("Cancelled", "Cleanup cancelled"));
                }
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let kind = entry.file_type()?;
                if !kind.is_file() || kind.is_symlink() {
                    continue;
                }
                let expired = entry
                    .metadata()?
                    .modified()?
                    .elapsed()
                    .is_ok_and(|age| age.as_secs_f64() > retention);
                let orphan =
                    check_digest(&name).is_ok() && !keep.contains(&name) && !recovery_exists;
                let scratch = name.starts_with('.') || name.ends_with(".partial");
                if expired && (orphan || scratch) {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }
    pub fn put_asset(&self, bytes: &[u8]) -> Result<String> {
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(error("BudgetExceeded", "Asset exceeds 32 MiB"));
        }
        let digest = crate::digest(bytes);
        let path = self.root.join("assets").join(&digest);
        if !path.exists() {
            let temporary = self.root.join("assets").join(format!(".{}", crate::uid()));
            let mut file = File::create(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(temporary, path)?;
        }
        Ok(digest)
    }
    pub fn asset(&self, id: &str) -> Result<Vec<u8>> {
        check_digest(id)?;
        let bytes = fs::read(self.root.join("assets").join(id))?;
        if bytes.len() > 32 * 1024 * 1024 || crate::digest(&bytes) != id {
            return Err(error("MalformedSave", "Asset integrity check failed"));
        }
        Ok(bytes)
    }
    pub fn export(&self, path: &Path) -> Result<()> {
        self.snapshot.validate()?;
        let tmp = path.with_extension(format!("{}.partial", crate::uid()));
        let result = (|| -> Result<()> {
            let file = File::create(&tmp)?;
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options)
                .map_err(|e| error("Storage", e))?;
            zip.write_all(&serde_json::to_vec(&self.snapshot)?)?;
            for id in self.snapshot.asset_ids()? {
                zip.start_file(format!("assets/{id}"), options)
                    .map_err(|e| error("Storage", e))?;
                zip.write_all(&self.asset(&id)?)?;
            }
            zip.finish().map_err(|e| error("Storage", e))?.sync_all()?;
            fs::rename(&tmp, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result
    }
    pub fn import(&mut self, path: &Path, expected: u64) -> Result<Snapshot> {
        if expected != self.snapshot.revision {
            return Err(error("StaleRevision", "Workspace changed during import"));
        }
        if fs::metadata(path)?.len() > 512 * 1024 * 1024 {
            return Err(error("BudgetExceeded", "Save file exceeds 512 MiB"));
        }
        let bytes = fs::read(path)?;
        let mut assets = BTreeMap::new();
        let mut next = if bytes.starts_with(b"PK") {
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes))
                .map_err(|e| error("MalformedSave", e))?;
            if zip.len() > 10001 {
                return Err(error("BudgetExceeded", "Archive contains too many entries"));
            }
            let mut manifest = None;
            let mut total = 0u64;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| error("MalformedSave", e))?;
                let name = file.name().to_owned();
                let max = if name == "manifest.json" {
                    64 * 1024 * 1024
                } else {
                    32 * 1024 * 1024
                };
                if file.size() > max {
                    return Err(error("BudgetExceeded", "Archive entry exceeds limit"));
                }
                total = total.saturating_add(file.size());
                if total > 512 * 1024 * 1024 {
                    return Err(error("BudgetExceeded", "Expanded archive exceeds 512 MiB"));
                }
                let mut content = Vec::new();
                file.by_ref().take(max + 1).read_to_end(&mut content)?;
                if content.len() as u64 > max {
                    return Err(error("BudgetExceeded", "Expanded entry exceeds limit"));
                }
                if name == "manifest.json" {
                    if manifest.is_some() {
                        return Err(error("MalformedSave", "Duplicate manifest"));
                    }
                    manifest = Some(serde_json::from_slice::<Snapshot>(&content)?);
                } else if let Some(id) = name.strip_prefix("assets/") {
                    check_digest(id)?;
                    if crate::digest(&content) != id
                        || assets.insert(id.to_owned(), content).is_some()
                    {
                        return Err(error("MalformedSave", "Invalid or duplicate asset"));
                    }
                } else {
                    return Err(error("MalformedSave", "Unknown archive member"));
                }
            }
            manifest.ok_or_else(|| error("MalformedSave", "Missing manifest"))?
        } else {
            let specimen: Value = serde_json::from_slice(&bytes)?;
            engine::validate(&specimen)?;
            Snapshot {
                specimen,
                ..Default::default()
            }
        };
        next.validate()?;
        for id in next.asset_ids()? {
            if !assets.contains_key(&id) {
                return Err(error(
                    "MalformedSave",
                    "Archive is missing a referenced asset",
                ));
            }
        }
        fs::create_dir_all(self.root.join("recovery"))?;
        fs::write(
            self.root
                .join("recovery")
                .join(format!("{}-import.bin", crate::uid())),
            &bytes,
        )?;
        if self.snapshot.revision > 0 {
            fs::write(
                self.root
                    .join("recovery")
                    .join(format!("{}-previous.json", crate::uid())),
                serde_json::to_vec(&self.snapshot)?,
            )?;
        }
        for data in assets.values() {
            self.put_asset(data)?;
        }
        next.reindex();
        self.commit(expected, next)
    }
}
pub fn check_digest(id: &str) -> Result<()> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(error("MalformedSave", "Invalid asset digest"));
    }
    Ok(())
}
