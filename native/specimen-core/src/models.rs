use crate::{Result, error};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub repo: String,
    pub file: String,
    pub license: String,
    pub memory_bytes: u64,
    pub revision: String,
    pub size: u64,
    pub sha256: String,
    pub url: String,
    pub source: String,
}
pub fn catalog() -> Vec<Model> {
    serde_json::from_str(include_str!("../../model-catalog.json")).expect("checked model catalog")
}
struct Loaded {
    child: Child,
    id: String,
    port: u16,
    token: String,
    last_use: Instant,
}
pub struct Models {
    pub root: PathBuf,
    pub binaries: PathBuf,
    loaded: Option<Loaded>,
}
impl Drop for Models {
    fn drop(&mut self) {
        self.unload();
    }
}
fn check(cancel: &AtomicBool, deadline: Instant) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(error("Cancelled", "Model job cancelled"))
    } else if Instant::now() > deadline {
        Err(error("BudgetExceeded", "Model job timed out"))
    } else {
        Ok(())
    }
}
impl Models {
    pub fn new(root: PathBuf, binaries: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            binaries,
            loaded: None,
        })
    }
    pub fn model(&self, id: &str) -> Result<Model> {
        catalog()
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| error("ModelMissing", "Unknown model"))
    }
    pub fn status(&mut self) -> Value {
        self.expire();
        json!(catalog().iter().map(|m|json!({"model":m,"installed":self.root.join(&m.file).is_file(),"loaded":self.loaded.as_ref().is_some_and(|l|l.id==m.id)})).collect::<Vec<_>>())
    }
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        let model = self.model(id)?;
        let p = self.root.join(&model.file);
        if !p.is_file() {
            return Err(error(
                "ModelMissing",
                format!("Install {} first", model.name),
            ));
        }
        Ok(p)
    }
    pub fn install(
        &self,
        id: &str,
        source: Option<&Path>,
        cancel: &AtomicBool,
        progress: &dyn Fn(Value),
    ) -> Result<Value> {
        let model = self.model(id)?;
        let target = self.root.join(&model.file);
        let tmp = target.with_extension(format!("{}.partial", crate::uid()));
        let deadline = Instant::now() + Duration::from_secs(7200);
        let result = (|| -> Result<Value> {
            let mut input: Box<dyn Read> = if let Some(path) = source {
                Box::new(File::open(path)?)
            } else {
                let response = reqwest::blocking::Client::builder()
                    .connect_timeout(Duration::from_secs(20))
                    .timeout(Duration::from_secs(7200))
                    .build()
                    .map_err(|e| error("Download", e))?
                    .get(&model.url)
                    .send()
                    .map_err(|e| error("Download", e))?
                    .error_for_status()
                    .map_err(|e| error("Download", e))?;
                Box::new(response)
            };
            let mut file = File::create(&tmp)?;
            let mut hasher = Sha256::new();
            let mut bytes = 0u64;
            let mut buffer = [0u8; 1024 * 256];
            let mut last = Instant::now();
            loop {
                check(cancel, deadline)?;
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                bytes += n as u64;
                if bytes > model.size {
                    return Err(error(
                        "ModelInvalid",
                        "Model is larger than its pinned manifest",
                    ));
                }
                file.write_all(&buffer[..n])?;
                hasher.update(&buffer[..n]);
                if last.elapsed() > Duration::from_millis(250) {
                    progress(json!({"phase":"download","received":bytes,"total":model.size}));
                    last = Instant::now();
                }
            }
            if bytes != model.size || format!("{:x}", hasher.finalize()) != model.sha256 {
                return Err(error(
                    "ModelInvalid",
                    "Model size or SHA-256 verification failed",
                ));
            }
            file.sync_all()?;
            fs::rename(&tmp, &target)?;
            progress(json!({"phase":"installed","received":bytes,"total":bytes}));
            Ok(json!({"id":model.id,"sha256":model.sha256,"revision":model.revision}))
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result
    }
    pub fn unload(&mut self) {
        if let Some(mut loaded) = self.loaded.take() {
            let _ = loaded.child.kill();
            let _ = loaded.child.wait();
        }
    }
    pub fn expire(&mut self) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|l| l.last_use.elapsed() > Duration::from_secs(120))
        {
            self.unload();
        }
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        if self.loaded.as_ref().is_some_and(|l| l.id == id) {
            self.unload();
        }
        let path = self.path(id)?;
        fs::remove_file(path)?;
        Ok(())
    }
    fn binary(&self, name: &str) -> Result<PathBuf> {
        let name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        };
        let p = self.binaries.join(name);
        if !p.is_file() {
            return Err(error(
                "BackendUnavailable",
                "Bundled model runtime is missing",
            ));
        }
        Ok(p)
    }
    fn verify(&self, m: &Model) -> Result<()> {
        let mut file = File::open(self.path(&m.id)?)?;
        if file.metadata()?.len() != m.size {
            return Err(error("ModelInvalid", "Installed model size changed"));
        }
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 262144];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        if format!("{:x}", hasher.finalize()) != m.sha256 {
            return Err(error("ModelInvalid", "Installed model digest changed"));
        }
        Ok(())
    }
    fn admission(m: &Model) -> Result<()> {
        let system = sysinfo::System::new_all();
        if system.available_memory() < m.memory_bytes {
            return Err(error(
                "BudgetExceeded",
                format!(
                    "{} needs approximately {} GB of available memory; {:.1} GB is available",
                    m.name,
                    m.memory_bytes / 1_000_000_000,
                    system.available_memory() as f64 / 1e9
                ),
            ));
        }
        Ok(())
    }
    pub fn load(&mut self, id: &str, cancel: &AtomicBool, progress: &dyn Fn(Value)) -> Result<()> {
        self.expire();
        if self.loaded.as_ref().is_some_and(|l| l.id == id) {
            return Ok(());
        }
        self.unload();
        let m = self.model(id)?;
        if m.kind != "text" {
            return Err(error(
                "ModelInvalid",
                "Image and OCR models load for each job",
            ));
        }
        self.verify(&m)?;
        Self::admission(&m)?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let token = crate::uid();
        let log = File::create(self.root.join("text-runtime.log"))?;
        let child = Command::new(self.binary("llama-server")?)
            .args([
                "--model",
                self.path(id)?
                    .to_str()
                    .ok_or_else(|| error("Storage", "Model path is not UTF-8"))?,
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--api-key",
                &token,
                "--ctx-size",
                "4096",
                "--threads",
                "4",
                "--n-gpu-layers",
                "0",
                "--jinja",
            ])
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()?;
        self.loaded = Some(Loaded {
            child,
            id: id.into(),
            port,
            token,
            last_use: Instant::now(),
        });
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .map_err(|e| error("BackendUnavailable", e))?;
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if let Err(e) = check(cancel, deadline) {
                self.unload();
                return Err(e);
            }
            let loaded = self.loaded.as_mut().unwrap();
            if loaded.child.try_wait()?.is_some() {
                self.unload();
                return Err(error(
                    "BackendUnavailable",
                    "Text runtime exited while loading; inspect its local log",
                ));
            }
            if client
                .get(format!("http://127.0.0.1:{port}/health"))
                .bearer_auth(&loaded.token)
                .send()
                .is_ok_and(|r| r.status().is_success())
            {
                break;
            }
            progress(json!({"phase":"loading","model":id}));
            std::thread::sleep(Duration::from_millis(200));
        }
        Ok(())
    }
    pub fn generate(
        &mut self,
        id: &str,
        prompt: &str,
        seed: u64,
        cancel: Arc<AtomicBool>,
        progress: &dyn Fn(Value),
    ) -> Result<(Value, Option<Vec<u8>>)> {
        if prompt.trim().is_empty() || prompt.len() > 8000 {
            return Err(error(
                "BudgetExceeded",
                "Generation prompt must contain 1 to 8,000 bytes",
            ));
        }
        let m = self.model(id)?;
        let deadline = Instant::now() + Duration::from_secs(1800);
        let mut provenance = json!({"id":crate::uid(),"model":m.id,"revision":m.revision,"modelDigest":m.sha256,"seed":seed,"prompt":prompt,"createdAt":crate::now(),"contributors":[],"learned":false});
        if m.kind == "text" {
            self.load(id, &cancel, progress)?;
            let loaded = self.loaded.as_ref().unwrap();
            let url = format!("http://127.0.0.1:{}/v1/chat/completions", loaded.port);
            let token = loaded.token.clone();
            let prompt = prompt.to_owned();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = (|| -> Result<Value> {
                    reqwest::blocking::Client::builder().timeout(Duration::from_secs(1800)).build().map_err(|e|error("Generation",e))?.post(url).bearer_auth(token).json(&json!({"messages":[{"role":"user","content":prompt}],"max_tokens":512,"temperature":0.7,"top_p":0.8,"seed":seed,"chat_template_kwargs":{"enable_thinking":false},"stream":false})).send().map_err(|e|error("Generation",e))?.error_for_status().map_err(|e|error("Generation",e))?.json().map_err(|e|error("Generation",e))
                })();
                let _ = tx.send(result);
            });
            loop {
                if let Err(e) = check(&cancel, deadline) {
                    self.unload();
                    return Err(e);
                }
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(result) => {
                        let response = result?;
                        let output = response["choices"][0]["message"]["content"]
                            .as_str()
                            .ok_or_else(|| error("Generation", "Model returned no text"))?;
                        provenance["text"] = json!(output);
                        provenance["artifactDigest"] = json!(crate::digest(output.as_bytes()));
                        provenance["parameters"] =
                            json!({"context":4096,"maxTokens":512,"thinking":false});
                        if let Some(l) = &mut self.loaded {
                            l.last_use = Instant::now();
                        }
                        return Ok((provenance, None));
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        progress(json!({"phase":"generating","model":id}))
                    }
                    Err(_) => return Err(error("Generation", "Inference worker stopped")),
                }
            }
        } else if m.kind == "image" {
            self.unload();
            self.verify(&m)?;
            Self::admission(&m)?;
            let output = self.root.join(format!("{}.png", crate::uid()));
            let log = File::create(self.root.join("image-runtime.log"))?;
            let mut child = Command::new(self.binary("sd-cli")?)
                .args([
                    "-m",
                    self.path(id)?
                        .to_str()
                        .ok_or_else(|| error("Storage", "Invalid model path"))?,
                    "-p",
                    prompt,
                    "-W",
                    "512",
                    "-H",
                    "512",
                    "--steps",
                    "1",
                    "--cfg-scale",
                    "1",
                    "--sampling-method",
                    "euler",
                    "--seed",
                    &seed.to_string(),
                    "-o",
                    output.to_str().unwrap(),
                    "-t",
                    "4",
                ])
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()?;
            let result = (|| -> Result<Vec<u8>> {
                loop {
                    if let Err(e) = check(&cancel, deadline) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(e);
                    }
                    if let Some(status) = child.try_wait()? {
                        if !status.success() {
                            return Err(error(
                                "Generation",
                                "Image runtime exited; inspect its local log",
                            ));
                        }
                        break;
                    }
                    progress(json!({"phase":"generating","model":id}));
                    std::thread::sleep(Duration::from_millis(100));
                }
                let bytes = fs::read(&output)?;
                image::load_from_memory(&bytes).map_err(|e| error("Generation", e))?;
                Ok(bytes)
            })();
            let _ = fs::remove_file(&output);
            let bytes = result?;
            provenance["artifactDigest"] = json!(crate::digest(&bytes));
            provenance["parameters"] = json!({"width":512,"height":512,"steps":1});
            Ok((provenance, Some(bytes)))
        } else {
            Err(error(
                "ModelInvalid",
                "Choose a text or image generation model",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_and_runtime_are_structured_failures() {
        let temp = tempfile::tempdir().unwrap();
        let models = Models::new(temp.path().join("models"), temp.path().join("binaries")).unwrap();
        assert_eq!(models.path("qwen3-4b").unwrap_err().code, "ModelMissing");
        assert_eq!(
            models.binary("llama-server").unwrap_err().code,
            "BackendUnavailable"
        );
        assert_eq!(
            models.binary("sd-cli").unwrap_err().code,
            "BackendUnavailable"
        );
    }

    #[test]
    fn failed_and_cancelled_imports_preserve_installed_model_and_clean_scratch() {
        let temp = tempfile::tempdir().unwrap();
        let models = Models::new(temp.path().join("models"), temp.path().join("binaries")).unwrap();
        let target = models
            .root
            .join(models.model("text-detection").unwrap().file);
        // Distinct existing bytes prove rejection preserves the target, even when it exists.
        fs::write(&target, b"existing model").unwrap();
        let source = temp.path().join("invalid-model");
        fs::write(&source, b"invalid replacement").unwrap();
        for (cancelled, code) in [(false, "ModelInvalid"), (true, "Cancelled")] {
            let result = models.install(
                "text-detection",
                Some(&source),
                &AtomicBool::new(cancelled),
                &|_| {},
            );
            assert_eq!(result.unwrap_err().code, code);
            assert_eq!(fs::read(&target).unwrap(), b"existing model");
            assert_eq!(fs::read_dir(&models.root).unwrap().count(), 1);
        }
    }
}
