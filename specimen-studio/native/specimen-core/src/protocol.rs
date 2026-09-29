//! Versioned desktop operation schema. Durable mutations always carry expected revisions.
use crate::vision::{Analysis, Focus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Request {
    Capabilities,
    Snapshot,
    /// Read-only store report for the store explorer.
    StoreInfo,
    Edit {
        revision: u64,
        specimen: Value,
    },
    Run {
        revision: u64,
        job_id: String,
        input: String,
    },
    Review {
        revision: u64,
        job_id: String,
    },
    Maintenance {
        revision: u64,
        job_id: String,
        mode: String,
    },
    Export {
        path: String,
    },
    Import {
        revision: u64,
        path: String,
    },
    Asset {
        id: String,
    },
    Analyze {
        job_id: String,
        bytes: Vec<u8>,
        focus: Focus,
    },
    RunVisual {
        revision: u64,
        job_id: String,
        asset: String,
        focus: Focus,
    },
    LearnVisual {
        revision: u64,
        asset: String,
        analysis: Box<Analysis>,
        name: String,
        ocr_correction: String,
    },
    LearnText {
        revision: u64,
        artifact_id: Option<String>,
        asset: Option<String>,
        pattern: Option<String>,
        text: Option<String>,
    },
    Models,
    Jobs,
    InstallModel {
        job_id: String,
        model_id: String,
        path: Option<String>,
    },
    LoadModel {
        job_id: String,
        model_id: String,
    },
    UnloadModel,
    RemoveModel {
        model_id: String,
    },
    Generate {
        job_id: String,
        model_id: String,
        prompt: String,
        seed: u64,
    },
    Network {
        revision: u64,
        network: crate::neural::Network,
    },
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_operations_fail_before_dispatch() {
        assert!(
            serde_json::from_value::<Request>(serde_json::json!({"op":"edit","specimen":{}}))
                .is_err()
        );
        assert!(serde_json::from_value::<Request>(serde_json::json!({"op":"analyze","bytes":[999],"jobId":"j","focus":{"x":0,"y":0,"width":1,"height":1}})).is_err());
        assert!(
            serde_json::from_value::<Request>(serde_json::json!({"op":"executeArbitraryCode"}))
                .is_err()
        );
    }
}
