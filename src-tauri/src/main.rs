#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{Value, json};
use specimen_core::{
    Result, engine, error,
    models::Models,
    persistence::Store,
    scheduler::{Priority, Scheduler},
    vision::{self, Focus},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{Manager, ipc::Channel};
struct AppState {
    store: Mutex<Store>,
    models: Mutex<Models>,
    scheduler: Scheduler,
}
fn value<'a>(r: &'a Value, key: &str) -> Result<&'a str> {
    r[key]
        .as_str()
        .ok_or_else(|| error("MalformedInput", format!("Missing {key}")))
}
fn revision(r: &Value) -> Result<u64> {
    r["revision"]
        .as_u64()
        .ok_or_else(|| error("MalformedInput", "Missing revision"))
}
fn invoke(state: Arc<AppState>, r: Value, progress: Channel<Value>) -> Result<Value> {
    let op = value(&r, "op")?;
    let notify = |v| {
        let _ = progress.send(v);
    };
    match op {
        "capabilities" => Ok(
            json!({"runtime":"desktop","storage":"WDBX v2","cpu":true,"gpu":"optional wgpu","ocr":true,"textGeneration":true,"imageGeneration":true,"nativeVersion":"0.2.0","contactLedger":specimen_core::language::ledger()}),
        ),
        "jobs" => Ok(json!(state.scheduler.active())),
        "snapshot" => Ok(serde_json::to_value(&state.store.lock().unwrap().snapshot)?),
        "edit" => Ok(serde_json::to_value(
            state
                .store
                .lock()
                .unwrap()
                .edit(revision(&r)?, r["specimen"].clone())?,
        )?),
        "run" | "review" | "maintenance" => {
            let lease = if op == "maintenance" {
                Some("maintenance".into())
            } else {
                None
            };
            let job = state.scheduler.begin_state(
                value(&r, "jobId")?.into(),
                lease,
                120,
                match op {
                    "run" => Priority::Foreground,
                    "review" => Priority::Residual,
                    _ => Priority::Idle,
                },
            )?;
            let mut next = state.store.lock().unwrap().snapshot.clone();
            if next.revision != revision(&r)? {
                return Err(error(
                    "StaleRevision",
                    "Workspace changed before the job started",
                ));
            }
            let mut cycle = Value::Null;
            next.specimen = match op {
                "run" => {
                    let (s, c) = engine::cycle(
                        &next.specimen,
                        value(&r, "input")?,
                        &next.network,
                        &job.cancel,
                        &notify,
                    )?;
                    cycle = c;
                    s
                }
                "review" => engine::review(&next.specimen, &next.network, &job.cancel)?,
                _ => {
                    let maintained = engine::maintain(
                        &next.specimen,
                        r["mode"].as_str().unwrap_or("phagy"),
                        &job.cancel,
                    )?;
                    engine::review(&maintained, &next.network, &job.cancel)?
                }
            };
            job.check()?;
            let mut store = state.store.lock().unwrap();
            let snapshot = store.commit(revision(&r)?, next)?;
            if op == "maintenance" && r["mode"] == "phagy" {
                let removed = store.cleanup(&job.cancel)?;
                notify(json!({"phase":"PHAGY","removed":removed}));
            }
            Ok(json!({"snapshot":snapshot,"cycle":cycle}))
        }
        "export" => {
            state
                .store
                .lock()
                .unwrap()
                .export(Path::new(value(&r, "path")?))?;
            Ok(json!(true))
        }
        "import" => Ok(serde_json::to_value(
            state
                .store
                .lock()
                .unwrap()
                .import(Path::new(value(&r, "path")?), revision(&r)?)?,
        )?),
        "asset" => {
            let bytes = state.store.lock().unwrap().asset(value(&r, "id")?)?;
            Ok(json!(bytes))
        }
        "analyze" => {
            let job = state
                .scheduler
                .begin(value(&r, "jobId")?.into(), None, 120)?;
            let bytes: Vec<u8> = serde_json::from_value(r["bytes"].clone())?;
            let focus: Focus = serde_json::from_value(r["focus"].clone()).unwrap_or_default();
            let paths = {
                let models = state.models.lock().unwrap();
                (
                    models.path("text-detection").ok(),
                    models.path("text-recognition").ok(),
                )
            };
            let analysis = vision::analyze(
                &bytes,
                focus,
                paths.0.as_deref().zip(paths.1.as_deref()),
                &job.cancel,
            )?;
            job.check()?;
            let store = state.store.lock().unwrap();
            let asset = store.put_asset(&bytes)?;
            let mut matches = Vec::new();
            for visual in &store.snapshot.visuals {
                if let Ok(other) =
                    serde_json::from_value::<vision::Analysis>(visual["analysis"].clone())
                    && other.pattern_id == analysis.pattern_id
                {
                    matches.push(json!({"nodeRef":visual["nodeRef"],"name":visual["name"],"score":vision::compare(&analysis,&other)?}));
                }
            }
            Ok(json!({"analysis":analysis,"asset":asset,"matches":matches}))
        }
        "runVisual" => {
            let job = state.scheduler.begin_state(
                value(&r, "jobId")?.into(),
                None,
                120,
                Priority::Foreground,
            )?;
            let (mut next, bytes) = {
                let store = state.store.lock().unwrap();
                (store.snapshot.clone(), store.asset(value(&r, "asset")?)?)
            };
            if next.revision != revision(&r)? {
                return Err(error("StaleRevision", "Workspace changed"));
            }
            let focus: Focus = serde_json::from_value(r["focus"].clone())?;
            let analysis = vision::analyze(&bytes, focus, None, &job.cancel)?;
            let mut scores = std::collections::BTreeMap::new();
            for visual in &next.visuals {
                let original: vision::Analysis =
                    serde_json::from_value(visual["analysis"].clone())?;
                if analysis.pattern_id != original.pattern_id {
                    continue;
                }
                let comparison = vision::compare(&analysis, &original)?;
                let score = specimen_core::language::Score {
                    similarity: engine::number(&comparison, "similarity", 0.),
                    dissimilarity: engine::number(&comparison, "dissimilarity", 100.),
                    modulation: 0.,
                    jitter: 0.,
                    confidence: engine::number(&comparison, "confidence", -100.),
                };
                if let Some(node) = engine::rows(&next.specimen, "nodes")
                    .iter()
                    .find(|n| n["ref"] == visual["nodeRef"])
                {
                    for entry in engine::rows(node, "entries") {
                        scores.insert(engine::text(entry, "id").to_owned(), score.clone());
                    }
                }
            }
            let input = format!("[image:{}]", value(&r, "asset")?);
            let (specimen, mut cycle) = engine::cycle_with_visual(
                &next.specimen,
                &input,
                &next.network,
                &job.cancel,
                &notify,
                Some(&scores),
            )?;
            cycle["imageEvidence"] = json!({"asset":r["asset"],"focus":focus,"patternId":analysis.pattern_id,"candidateEntries":scores.len()});
            next.specimen = specimen;
            if let Some(last) = next.specimen["history"]
                .as_array_mut()
                .and_then(|h| h.last_mut())
            {
                *last = cycle.clone();
            }
            job.check()?;
            let snapshot = state.store.lock().unwrap().commit(revision(&r)?, next)?;
            Ok(json!({"snapshot":snapshot,"cycle":cycle}))
        }
        "learnVisual" => {
            let mut store = state.store.lock().unwrap();
            let mut next = store.snapshot.clone();
            let asset = value(&r, "asset")?;
            let bytes = store.asset(asset)?;
            let supplied: vision::Analysis = serde_json::from_value(r["analysis"].clone())?;
            supplied.validate()?;
            let mut analysis = vision::analyze(
                &bytes,
                supplied.focus,
                None,
                &std::sync::atomic::AtomicBool::new(false),
            )?;
            analysis.ocr = supplied.ocr;
            analysis.ocr_status = supplied.ocr_status;
            let name = value(&r, "name")?;
            if name.is_empty() || name.len() > 100 {
                return Err(error(
                    "MalformedInput",
                    "Visual name must be 1 to 100 bytes",
                ));
            }
            if next.visuals.iter().any(|v| v["asset"] == asset) {
                return Err(error(
                    "Duplicate",
                    "This original image has already been learned",
                ));
            }
            let ref_id = specimen_core::uid();
            let entry = specimen_core::uid();
            let pattern = format!("[image:{asset}]");
            next.specimen["nodes"].as_array_mut().ok_or_else(||error("MalformedSave","Initialize the specimen first"))?.push(json!({"ref":ref_id,"name":name,"type":"pattern","contextId":"","patternId":format!("visual:{}",analysis.pattern_id),"resolution":analysis.resolution,"strength":5,"jitter":true,"tone":"neutral","entries":[{"id":entry,"pattern":pattern,"alternatives":[{"id":specimen_core::uid(),"action":format!("Recognized image: {name}"),"inhibition":"","weight":1,"remixed":false}]}],"createdAt":specimen_core::now(),"modality":"image"}));
            for artifact in &mut next.artifacts {
                if artifact["asset"] == asset {
                    artifact["learnedNode"] = json!(ref_id);
                }
            }
            next.specimen["resources"].as_array_mut().unwrap().push(json!({"ref":specimen_core::uid(),"subsystem":"imaginarium","text":name,"value":format!("Visual ingredient: {name}"),"patternId":analysis.pattern_id,"resourceId":"","valence":0,"intensity":0,"visualFeatures":analysis.features,"asset":asset,"nodeRef":ref_id}));
            next.visuals.push(json!({"asset":asset,"analysis":analysis,"nodeRef":ref_id,"name":name,"ocrCorrection":r["ocrCorrection"],"createdAt":specimen_core::now()}));
            Ok(serde_json::to_value(store.commit(revision(&r)?, next)?)?)
        }
        "learnText" => {
            let mut store = state.store.lock().unwrap();
            let mut next = store.snapshot.clone();
            let (pattern, response, provenance) = if let Some(id) = r["artifactId"].as_str() {
                let artifact = next
                    .artifacts
                    .iter()
                    .find(|a| a["id"] == id)
                    .ok_or_else(|| error("MalformedInput", "Unknown artifact"))?;
                (
                    value(artifact, "prompt")?.to_owned(),
                    value(artifact, "text")?.to_owned(),
                    json!({"artifactId":id,"model":artifact["model"],"modelDigest":artifact["modelDigest"]}),
                )
            } else {
                let asset = value(&r, "asset")?;
                store.asset(asset)?;
                (
                    value(&r, "pattern")?.to_owned(),
                    value(&r, "text")?.to_owned(),
                    json!({"asset":asset,"source":"reviewed OCR correction"}),
                )
            };
            if pattern.is_empty()
                || pattern.len() > 2000
                || response.is_empty()
                || response.len() > 7000
            {
                return Err(error(
                    "BudgetExceeded",
                    "Learning text exceeds pattern or action bounds",
                ));
            }
            let reference = specimen_core::uid();
            let descriptor = specimen_core::language::identify(&pattern);
            let node = json!({"ref":reference,"name":pattern.chars().take(80).collect::<String>(),"type":"pattern","contextId":"","patternId":descriptor.id,"resolution":descriptor.resolution,"strength":engine::number(&next.specimen["settings"],"initialStrength",5.),"jitter":true,"tone":"neutral","entries":[{"id":specimen_core::uid(),"pattern":pattern,"alternatives":[{"id":specimen_core::uid(),"action":format!("&literal({})",serde_json::to_string(&response)?),"inhibition":"","weight":1,"remixed":false}]}],"provenance":provenance,"createdAt":specimen_core::now()});
            next.specimen["nodes"]
                .as_array_mut()
                .ok_or_else(|| error("MalformedSave", "Initialize specimen first"))?
                .push(node);
            if let Some(id) = r["artifactId"].as_str() {
                for artifact in &mut next.artifacts {
                    if artifact["id"] == id {
                        artifact["learnedNode"] = json!(reference);
                    }
                }
            }
            Ok(serde_json::to_value(store.commit(revision(&r)?, next)?)?)
        }
        "loadModel" => {
            let job = state.scheduler.begin(
                value(&r, "jobId")?.into(),
                Some("generation".into()),
                180,
            )?;
            state
                .models
                .lock()
                .unwrap()
                .load(value(&r, "modelId")?, &job.cancel, &notify)?;
            Ok(json!(true))
        }
        "models" => match state.models.try_lock() {
            Ok(mut models) => Ok(models.status()),
            Err(_) => Ok(json!({"busy":true})),
        },
        "installModel" => {
            let job = state.scheduler.begin(
                value(&r, "jobId")?.into(),
                Some("model-install".into()),
                7200,
            )?;
            let (root, binaries) = {
                let models = state.models.lock().unwrap();
                (models.root.clone(), models.binaries.clone())
            };
            let models = Models::new(root, binaries)?;
            models.install(
                value(&r, "modelId")?,
                r["path"].as_str().map(Path::new),
                &job.cancel,
                &notify,
            )
        }
        "unloadModel" => {
            state.models.lock().unwrap().unload();
            Ok(json!(true))
        }
        "removeModel" => {
            state.models.lock().unwrap().remove(value(&r, "modelId")?)?;
            Ok(json!(true))
        }
        "generate" => {
            let job = state.scheduler.begin(
                value(&r, "jobId")?.into(),
                Some("generation".into()),
                1800,
            )?;
            let (mut artifact, bytes) = state.models.lock().unwrap().generate(
                value(&r, "modelId")?,
                value(&r, "prompt")?,
                r["seed"].as_u64().unwrap_or(104729),
                job.cancel.clone(),
                &notify,
            )?;
            job.check()?;
            let mut store = state.store.lock().unwrap();
            if let Some(bytes) = bytes {
                artifact["asset"] = json!(store.put_asset(&bytes)?);
            }
            let mut next = store.snapshot.clone();
            next.artifacts.push(artifact.clone());
            let revision = next.revision;
            let snapshot = store.commit(revision, next)?;
            Ok(json!({"snapshot":snapshot,"artifact":artifact}))
        }
        "network" => {
            let mut store = state.store.lock().unwrap();
            let mut next = store.snapshot.clone();
            next.network = serde_json::from_value(r["network"].clone())?;
            Ok(serde_json::to_value(store.commit(revision(&r)?, next)?)?)
        }
        _ => Err(error("InvalidScope", "Unknown native operation")),
    }
}
#[tauri::command]
async fn native_call(
    state: tauri::State<'_, Arc<AppState>>,
    request: Value,
    progress: Channel<Value>,
) -> Result<Value> {
    let request: specimen_core::protocol::Request =
        serde_json::from_value(request).map_err(|e| error("MalformedInput", e))?;
    let request = serde_json::to_value(request)?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || invoke(state, request, progress))
        .await
        .map_err(|e| error("Runtime", e))?
}
#[tauri::command]
fn cancel_job(state: tauri::State<'_, Arc<AppState>>, job_id: String) {
    state.scheduler.cancel(&job_id);
}
fn main() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .setup(|app| {
            let root = if cfg!(debug_assertions) {
                std::env::var_os("WDBX_STUDIO_TEST_DATA")
                    .map(PathBuf::from)
                    .unwrap_or(app.path().app_data_dir()?)
            } else {
                app.path().app_data_dir()?
            };
            let resources = app.path().resource_dir()?.join("binaries");
            let models = Models::new(root.join("models"), resources)?;
            let state = Arc::new(AppState {
                store: Mutex::new(Store::open(&root)?),
                models: Mutex::new(models),
                scheduler: Scheduler::default(),
            });
            let weak = Arc::downgrade(&state);
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let Some(state) = weak.upgrade() else {
                        break;
                    };
                    if let Ok(mut models) = state.models.try_lock() {
                        models.expire();
                    }
                }
            });
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![native_call, cancel_job])
        .build(tauri::generate_context!())
        .expect("WDBX desktop initialization failed")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let state = app.state::<Arc<AppState>>();
                state.scheduler.cancel_all();
                if let Ok(mut models) = state.models.try_lock() {
                    models.unload();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::ipc::InvokeResponseBody;

    #[test]
    fn invoke_cancellation_before_commit_preserves_revision_and_releases_job() {
        let root =
            std::env::temp_dir().join(format!("wdbx-invoke-cancel-{}", uuid::Uuid::new_v4()));
        let mut store = Store::open(&root).unwrap();
        let specimen = serde_json::from_str(include_str!(
            "../../native/specimen-core/tests/fixtures/starter.json"
        ))
        .unwrap();
        store.edit(0, specimen).unwrap();
        let before = serde_json::to_value(&store.snapshot).unwrap();
        let state = Arc::new(AppState {
            store: Mutex::new(store),
            models: Mutex::new(Models::new(root.join("models"), root.join("binaries")).unwrap()),
            scheduler: Scheduler::default(),
        });
        let callback_state = state.clone();
        let reached_compose = Arc::new(AtomicBool::new(false));
        let callback_reached_compose = reached_compose.clone();
        let channel = Channel::new(move |body| {
            let InvokeResponseBody::Json(body) = body else {
                panic!("Cycle progress must be JSON");
            };
            let event: Value = serde_json::from_str(&body).unwrap();
            if event["phase"] == "Compose" {
                callback_reached_compose.store(true, Ordering::Relaxed);
                callback_state.scheduler.cancel("cancel-at-compose");
            }
            Ok(())
        });
        let result = invoke(
            state.clone(),
            json!({"op":"run","revision":1,"jobId":"cancel-at-compose","input":"2+2"}),
            channel,
        );
        assert!(reached_compose.load(Ordering::Relaxed));
        assert_eq!(result.unwrap_err().code, "Cancelled");
        assert!(state.scheduler.active().is_empty());
        assert_eq!(
            serde_json::to_value(&state.store.lock().unwrap().snapshot).unwrap(),
            before
        );
        drop(state);
        let reopened = Store::open(&root).unwrap();
        assert_eq!(serde_json::to_value(&reopened.snapshot).unwrap(), before);
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }
}
