use crate::{Result, error};
use image::{GenericImageView, ImageReader};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::Cursor,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Focus {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Default for Focus {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub width: u32,
    pub height: u32,
    pub focus: Focus,
    pub pattern_id: String,
    pub resolution: String,
    pub features: Vec<f32>,
    pub contours: Vec<[f32; 2]>,
    pub holes: usize,
    pub convex: usize,
    pub concave: usize,
    pub sdf: DistanceField,
    pub ocr: Vec<Value>,
    pub ocr_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceField {
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
}
impl Analysis {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 8192
            || self.height > 8192
            || self.features.len() != 80
            || self
                .features
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.sdf.width != 64
            || self.sdf.height != 64
            || self.sdf.values.len() != 4096
            || self
                .sdf
                .values
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 100.)
            || self.contours.len() > 4096
            || self
                .contours
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.ocr.len() > 10000
            || !self.pattern_id.starts_with("visual-v2:")
        {
            return Err(error(
                "MalformedSave",
                "Invalid visual descriptor or signed distance field",
            ));
        }
        Ok(())
    }
}
pub fn analyze(
    bytes: &[u8],
    focus: Focus,
    ocr_models: Option<(&Path, &Path)>,
    cancel: &AtomicBool,
) -> Result<Analysis> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(error("BudgetExceeded", "Image file exceeds 32 MiB"));
    }
    if [focus.x, focus.y, focus.width, focus.height]
        .iter()
        .any(|n| !n.is_finite())
        || focus.x < 0.
        || focus.y < 0.
        || focus.width <= 0.
        || focus.height <= 0.
        || focus.x + focus.width > 1.001
        || focus.y + focus.height > 1.001
    {
        return Err(error("MalformedInput", "Focus must fit within the image"));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| error("MalformedInput", e))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let original = reader.decode().map_err(|e| error("MalformedInput", e))?;
    let (w, h) = original.dimensions();
    let x = (focus.x * w as f32) as u32;
    let y = (focus.y * h as f32) as u32;
    let fw = ((focus.width * w as f32) as u32).max(1).min(w - x);
    let fh = ((focus.height * h as f32) as u32).max(1).min(h - y);
    let cropped = original.crop_imm(x, y, fw, fh);
    let grid = cropped
        .resize_exact(64, 64, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let mut histogram = vec![0f32; 80];
    let mut mask = vec![false; 4096];
    let background = grid.get_pixel(0, 0).0;
    for (px, py, p) in grid.enumerate_pixels() {
        let index = py as usize * 64 + px as usize;
        let distance =
            p.0.iter()
                .zip(background)
                .map(|(a, b)| (*a as i32 - b as i32).unsigned_abs())
                .sum::<u32>();
        mask[index] = distance > 90;
        let bucket = (p[0] as usize / 64) * 16 + (p[1] as usize / 64) * 4 + p[2] as usize / 64;
        histogram[bucket] += 1. / 4096.;
        histogram[64 + ((py / 16) * 4 + px / 16) as usize] += f32::from(mask[index]) / 256.;
    }
    let neighbors = |i: usize| {
        let x = i % 64;
        let y = i / 64;
        [
            if x > 0 { Some(i - 1) } else { None },
            if x < 63 { Some(i + 1) } else { None },
            if y > 0 { Some(i - 64) } else { None },
            if y < 63 { Some(i + 64) } else { None },
        ]
    };
    let mut boundaries = Vec::new();
    let (mut convex, mut concave) = (0, 0);
    for i in 0..4096 {
        if mask[i] {
            let count = neighbors(i)
                .into_iter()
                .flatten()
                .filter(|&j| mask[j])
                .count();
            if count < 4 {
                boundaries.push(i);
                if count <= 2 {
                    convex += 1;
                } else {
                    concave += 1;
                }
            }
        }
    }
    let mut visited = vec![false; 4096];
    let mut holes = 0;
    for i in 0..4096 {
        if !mask[i] && !visited[i] {
            let mut queue = VecDeque::from([i]);
            visited[i] = true;
            let mut edge = false;
            while let Some(n) = queue.pop_front() {
                edge |= n % 64 == 0 || n % 64 == 63 || n / 64 == 0 || n / 64 == 63;
                for j in neighbors(n).into_iter().flatten() {
                    if !mask[j] && !visited[j] {
                        visited[j] = true;
                        queue.push_back(j);
                    }
                }
            }
            if !edge {
                holes += 1;
            }
        }
    }
    let mut sdf = Vec::with_capacity(4096);
    for (i, inside) in mask.iter().enumerate() {
        if i % 64 == 0 && cancel.load(Ordering::Relaxed) {
            return Err(error("Cancelled", "Image analysis cancelled"));
        }
        let d = boundaries
            .iter()
            .map(|&b| {
                let dx = (b % 64) as f32 - (i % 64) as f32;
                let dy = (b / 64) as f32 - (i / 64) as f32;
                dx * dx + dy * dy
            })
            .fold(4096., f32::min)
            .sqrt()
            / 64.;
        sdf.push(if *inside { -d } else { d });
    }
    let resolution = if boundaries.len() < 80 {
        "low"
    } else if boundaries.len() < 300 {
        "medium"
    } else {
        "high"
    };
    let quantized = histogram
        .iter()
        .map(|x| (x * 4.).round() as u8)
        .chain([holes.min(15) as u8])
        .collect::<Vec<_>>();
    let pattern_id = format!(
        "visual-v2:{resolution}:{}",
        &crate::digest(&quantized)[..24]
    );
    let (ocr, ocr_status) = if let Some((d, r)) = ocr_models {
        match recognize(&cropped.to_rgb8(), d, r) {
            Ok(mut text) => {
                for item in &mut text {
                    item["focusOrigin"] = json!({"x":x,"y":y});
                }
                (text, "complete".into())
            }
            Err(e) => (vec![], e.message),
        }
    } else {
        (
            vec![],
            "Install the OCR model pair to recognize text locally.".into(),
        )
    };
    Ok(Analysis {
        width: w,
        height: h,
        focus,
        pattern_id,
        resolution: resolution.into(),
        features: histogram,
        contours: boundaries
            .iter()
            .map(|&i| [(i % 64) as f32 / 64., (i / 64) as f32 / 64.])
            .collect(),
        holes,
        convex,
        concave,
        sdf: DistanceField {
            width: 64,
            height: 64,
            values: sdf,
        },
        ocr,
        ocr_status,
    })
}
fn recognize(image: &image::RgbImage, detection: &Path, recognition: &Path) -> Result<Vec<Value>> {
    use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
    let detection_model =
        rten::Model::load_file(detection).map_err(|e| error("ModelInvalid", e))?;
    let recognition_model =
        rten::Model::load_file(recognition).map_err(|e| error("ModelInvalid", e))?;
    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .map_err(|e| error("ModelInvalid", e))?;
    let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
        .map_err(|e| error("MalformedInput", e))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| error("Compute", e))?;
    let words = engine
        .detect_words(&input)
        .map_err(|e| error("Compute", e))?;
    let lines = engine.find_text_lines(&input, &words);
    let recognized = engine
        .recognize_text(&input, &lines)
        .map_err(|e| error("Compute", e))?;
    Ok(recognized.into_iter().zip(lines).filter_map(|(line,regions)|line.map(|line|json!({"text":line.to_string(),"regions":regions.iter().map(|r|{let c=r.center();json!({"cx":c.x,"cy":c.y,"width":r.width(),"height":r.height()})}).collect::<Vec<_>>()}))).collect())
}
pub fn compare(a: &Analysis, b: &Analysis) -> Result<Value> {
    if a.features.len() != b.features.len() {
        return Err(error("MalformedInput", "Incompatible visual encoder"));
    }
    if a.features.len() != 80 || a.sdf.values.len() != 4096 || b.sdf.values.len() != 4096 {
        return Err(error("MalformedInput", "Visual descriptor shape mismatch"));
    }
    let color = a.features[..64]
        .iter()
        .zip(&b.features[..64])
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / 2.;
    let spatial = a.features[64..]
        .iter()
        .zip(&b.features[64..])
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / 16.;
    let shape = (a
        .sdf
        .values
        .iter()
        .zip(&b.sdf.values)
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / 1024.)
        .min(1.);
    let mismatch = (color * 0.35
        + spatial * 0.35
        + shape * 0.3
        + a.holes.abs_diff(b.holes).min(3) as f32 * 0.1)
        .clamp(0., 1.);
    let similarity = 100. * (1. - mismatch);
    Ok(
        json!({"similarity":similarity,"dissimilarity":100.-similarity,"confidence":(similarity*2.-100.).clamp(-100.,100.),"samePatternId":a.pattern_id==b.pattern_id}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_field_and_focus() {
        let mut img = image::RgbImage::from_pixel(64, 64, image::Rgb([255, 255, 255]));
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, image::Rgb([0, 0, 0]));
            }
        }
        let mut data = Cursor::new(Vec::new());
        img.write_to(&mut data, image::ImageFormat::Png).unwrap();
        let a = analyze(
            data.get_ref(),
            Focus::default(),
            None,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(a.sdf.values[32 * 64 + 32] < 0.);
        assert!(a.sdf.values[0] > 0.);
        assert_eq!(compare(&a, &a).unwrap()["confidence"], 100.);
    }
}
