use specimen_core::{
    models::Models,
    vision::{self, Focus},
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let root = PathBuf::from(args.get(1).ok_or("test data directory required")?);
    let binaries = PathBuf::from(args.get(2).ok_or("runtime directory required")?);
    let mode = args.get(3).map(String::as_str).unwrap_or("ocr");
    let mut models = Models::new(root.join("models"), binaries)?;
    std::fs::create_dir_all(root.join("results"))?;
    if mode == "ocr" {
        let bytes = std::fs::read(root.join("ocr-fixture.png"))?;
        let detection = models.path("text-detection")?;
        let recognition = models.path("text-recognition")?;
        let result = vision::analyze(
            &bytes,
            Focus::default(),
            Some((&detection, &recognition)),
            &AtomicBool::new(false),
        )?;
        let text = result
            .ocr
            .iter()
            .filter_map(|line| line["text"].as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if !text.contains("HELLO") || !text.contains("WORLD") {
            return Err(format!("OCR mismatch: {text}; status {}", result.ocr_status).into());
        }
        std::fs::write(
            root.join("results/ocr.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
        println!("OCR verified: {text}");
    } else {
        let id = if mode == "text" {
            "qwen3-4b"
        } else {
            "sdxl-turbo"
        };
        let prompt = if mode == "text" {
            "Write one sentence about a teal circle. /no_think"
        } else {
            "A teal ceramic sphere on a warm ivory background, soft studio light"
        };
        let last = std::sync::Mutex::new(std::time::Instant::now());
        let (result, bytes) = models.generate(
            id,
            prompt,
            104729,
            Arc::new(AtomicBool::new(false)),
            &|event| {
                let mut last = last.lock().unwrap();
                if last.elapsed().as_secs() >= 10 {
                    println!("{}", event);
                    *last = std::time::Instant::now();
                }
            },
        )?;
        if mode == "text" && result["text"].as_str().is_none_or(|s| s.trim().is_empty()) {
            return Err("Empty generated text".into());
        }
        if mode == "image" {
            let bytes = bytes.ok_or("Image inference returned no image artifact")?;
            if bytes.is_empty() {
                return Err("Image inference returned an empty image artifact".into());
            }
            std::fs::write(root.join("results/generated-image.png"), bytes)?;
        }
        std::fs::write(
            root.join("results").join(format!("{mode}.json")),
            serde_json::to_vec_pretty(&result)?,
        )?;
        println!(
            "Generation verified: {}",
            result["text"].as_str().unwrap_or("image")
        );
        // Cancel only after the real provider reports its spawned inference job.
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled_at = std::sync::Mutex::new(None);
        let failure = models
            .generate(id, prompt, 104730, cancel.clone(), &|event| {
                if event["phase"] == "generating" {
                    let mut at = cancelled_at.lock().unwrap();
                    if at.is_none() {
                        *at = Some(std::time::Instant::now());
                        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            })
            .expect_err("A cancelled real inference job must not produce an artifact");
        let cancellation_ms = cancelled_at
            .lock()
            .unwrap()
            .ok_or("Job never entered generation")?
            .elapsed()
            .as_millis();
        if failure.code != "Cancelled" || cancellation_ms > 5000 {
            return Err(format!("Unexpected cancellation: {failure}; {cancellation_ms} ms").into());
        }
        let stray_images = std::fs::read_dir(root.join("models"))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
            .count();
        if stray_images != 0 {
            return Err("Cancelled image left a scratch artifact".into());
        }
        std::fs::write(
            root.join("results")
                .join(format!("{mode}-cancellation.json")),
            serde_json::to_vec_pretty(
                &serde_json::json!({"model":id,"code":failure.code,"cancelledAfterGenerationStarted":true,"cancellationMilliseconds":cancellation_ms,"scratchImages":stray_images,"previousArtifactDigest":result["artifactDigest"]}),
            )?,
        )?;
        println!("Real inference cancellation verified in {cancellation_ms} ms");
    }
    Ok(())
}
