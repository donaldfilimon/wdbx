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
        if let Some(bytes) = bytes {
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
    }
    Ok(())
}
