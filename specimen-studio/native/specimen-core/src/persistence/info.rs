//! Read-only inspection of the desktop store for the store explorer.
use super::Store;
use crate::{Result, engine};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

const SNAPSHOT_KEY: &str = "studio/snapshot";
const TOMBSTONES_SHOWN: usize = 50;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreInfo {
    pub root: String,
    pub writer_id: String,
    /// Causal frontier: writer id -> committed sequence.
    pub heads: BTreeMap<String, u64>,
    pub committed_transactions: usize,
    pub kv_count: usize,
    pub vector_count: usize,
    pub spatial_count: usize,
    pub audit_count: usize,
    pub audit_heads: Vec<String>,
    pub audit_dag: AuditCheck,
    /// The current version of `studio/snapshot`, or null before the first save.
    pub snapshot_key: Option<KeyVersion>,
    pub schema: String,
    pub revision: u64,
    pub records: BTreeMap<&'static str, usize>,
    /// Most recent deletions first.
    pub tombstones: Vec<Tombstone>,
    pub disk: BTreeMap<&'static str, DiskUsage>,
}

#[derive(Debug, Serialize)]
pub struct AuditCheck {
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyVersion {
    pub key: &'static str,
    pub version_id: String,
    pub writer_id: String,
    pub sequence: u64,
    /// Unresolved concurrent versions besides the preferred one.
    pub conflicts: usize,
    pub bytes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    pub name: Value,
    #[serde(rename = "ref")]
    pub node_ref: Value,
    pub deleted_at: Value,
}

#[derive(Debug, Default, Serialize)]
pub struct DiskUsage {
    pub files: u64,
    pub bytes: u64,
}

fn usage(dir: &Path) -> Result<DiskUsage> {
    let mut total = DiskUsage::default();
    if !dir.is_dir() {
        return Ok(total);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            let inner = usage(&entry.path())?;
            total.files += inner.files;
            total.bytes += inner.bytes;
        } else if kind.is_file() {
            total.files += 1;
            total.bytes += entry.metadata()?.len();
        }
    }
    Ok(total)
}

impl Store {
    /// A read-only report of the store: heads, counts, audit DAG verification,
    /// the snapshot key's current version, tombstones and on-disk usage.
    pub fn info(&self) -> Result<StoreInfo> {
        let v2 = self.store.snapshot();
        let snapshot_key = v2.get(SNAPSHOT_KEY).map(|set| KeyVersion {
            key: SNAPSHOT_KEY,
            version_id: set.preferred.version_id.to_string(),
            writer_id: set.preferred.writer_id.to_string(),
            sequence: set.preferred.sequence,
            conflicts: set.conflicts.len(),
            bytes: set.preferred.value.len(),
        });
        let audit_dag = match v2.verify_audit_dag() {
            Ok(()) => AuditCheck {
                ok: true,
                error: None,
            },
            Err(e) => AuditCheck {
                ok: false,
                error: Some(e),
            },
        };
        let s = &self.snapshot;
        let rows = |key| engine::rows(&s.specimen, key).len();
        let entries = engine::rows(&s.specimen, "nodes")
            .iter()
            .map(|n| engine::rows(n, "entries").len())
            .sum();
        let records = BTreeMap::from([
            ("nodes", rows("nodes")),
            ("entries", entries),
            ("resources", rows("resources")),
            ("attachments", rows("attachments")),
            ("history", rows("history")),
            ("events", rows("events")),
            ("mutations", rows("mutations")),
            ("proposals", rows("proposals")),
            ("visuals", s.visuals.len()),
            ("artifacts", s.artifacts.len()),
            ("tombstones", s.tombstones.len()),
            ("nativeIds", s.native_ids.len()),
        ]);
        let tombstones = s
            .tombstones
            .iter()
            .rev()
            .take(TOMBSTONES_SHOWN)
            .map(|t| Tombstone {
                name: t["node"]["name"].clone(),
                node_ref: t["node"]["ref"].clone(),
                deleted_at: t["deletedAt"].clone(),
            })
            .collect();
        Ok(StoreInfo {
            root: self.root.display().to_string(),
            writer_id: self.store.writer_id().to_string(),
            heads: v2
                .heads()
                .iter()
                .map(|(writer, seq)| (writer.to_string(), *seq))
                .collect(),
            committed_transactions: v2.committed_transactions(),
            kv_count: v2.kv_count(),
            vector_count: v2.vector_count(),
            spatial_count: v2.spatial_count(),
            audit_count: v2.audit_count(),
            audit_heads: v2.audit_heads(),
            audit_dag,
            snapshot_key,
            schema: s.schema.clone(),
            revision: s.revision,
            records,
            tombstones,
            disk: BTreeMap::from([
                ("store", usage(&self.root.join("store"))?),
                ("assets", usage(&self.root.join("assets"))?),
            ]),
        })
    }
}
