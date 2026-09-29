use crate::{Result, error};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub inputs: usize,
    pub outputs: usize,
    pub offsets: Vec<usize>,
    pub columns: Vec<usize>,
    pub weights: Vec<f32>,
    pub biases: Vec<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub version: u32,
    pub layers: Vec<Layer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synthesis {
    pub values: Vec<f32>,
    pub activations: Vec<String>,
    pub backend: String,
    pub fallback: Option<String>,
}
/// Evaluates one layer on a device; `Network::run` falls back to CPU on error.
pub trait Accelerator: Sync {
    /// Short backend name reported in `Synthesis::backend`, e.g. "wgpu".
    fn name(&self) -> &'static str;
    fn layer(&self, layer: &Layer, input: &[f32]) -> Result<Vec<f32>>;
}
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn sample(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        if self.0 == 0 {
            self.0 = 104729;
        }
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}
impl Default for Network {
    fn default() -> Self {
        let mut rng = Rng(104729);
        Self {
            version: 1,
            layers: [(128, 64), (64, 32)]
                .into_iter()
                .map(|(inputs, outputs)| {
                    let mut layer = Layer {
                        inputs,
                        outputs,
                        offsets: vec![0],
                        columns: Vec::new(),
                        weights: Vec::new(),
                        biases: vec![0.01; outputs],
                    };
                    for row in 0..outputs {
                        for k in 0..8 {
                            layer.columns.push((row * 7 + k * 13) % inputs);
                            layer.weights.push(((rng.sample() - 0.5) * 0.7) as f32);
                        }
                        layer.offsets.push(layer.weights.len());
                    }
                    layer
                })
                .collect(),
        }
    }
}
impl Network {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.layers.is_empty() || self.layers.len() > 8 {
            return Err(error("MalformedSave", "Invalid network architecture"));
        }
        let mut previous = 128;
        for l in &self.layers {
            if l.inputs != previous
                || l.outputs == 0
                || l.outputs > 1024
                || l.weights.len() > 131072
                || l.columns.len() != l.weights.len()
                || l.biases.len() != l.outputs
                || l.offsets.len() != l.outputs + 1
                || l.offsets.first() != Some(&0)
                || l.offsets.last() != Some(&l.weights.len())
                || l.offsets.windows(2).any(|w| w[0] > w[1])
                || l.columns.iter().any(|&c| c >= l.inputs)
                || l.weights.iter().chain(&l.biases).any(|n| !n.is_finite())
            {
                return Err(error(
                    "MalformedSave",
                    "Invalid sparse layer shape or weights",
                ));
            }
            previous = l.outputs;
        }
        if previous != 32 {
            return Err(error(
                "MalformedSave",
                "Composition decoder requires 32 outputs",
            ));
        }
        Ok(())
    }
    pub fn run(
        &self,
        input: &[f32],
        familiar: bool,
        seed: u64,
        accel: Option<&dyn Accelerator>,
    ) -> Result<Synthesis> {
        self.validate()?;
        if input.len() != 128 || input.iter().any(|x| !x.is_finite()) {
            return Err(error(
                "MalformedInput",
                "Neural input must contain 128 finite features",
            ));
        }
        let mut rng = Rng(seed);
        let mut values = input.to_vec();
        let mut activations = Vec::new();
        let mut fallback = None;
        let mut used = accel;
        for l in &self.layers {
            let sigmoid = rng.sample() < if familiar { 0.75 } else { 0.25 };
            let raw = if let Some(a) = used {
                match a.layer(l, &values) {
                    Ok(v) => v,
                    Err(e) => {
                        fallback = Some(e.message);
                        used = None;
                        cpu_layer(l, &values)?
                    }
                }
            } else {
                cpu_layer(l, &values)?
            };
            values = raw
                .iter()
                .map(|x| {
                    if sigmoid {
                        1.0 / (1.0 + libm::expf(-x.clamp(-30.0, 30.0)))
                    } else {
                        x.clamp(0.0, 16.0)
                    }
                })
                .collect();
            activations.push(if sigmoid { "sigmoid" } else { "relu" }.into());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(error("ObserverRejected", "Nonfinite synthesis result"));
        }
        Ok(Synthesis {
            values,
            activations,
            backend: used.map_or("cpu", |a| a.name()).into(),
            fallback,
        })
    }
}
fn cpu_layer(l: &Layer, input: &[f32]) -> Result<Vec<f32>> {
    (0..l.outputs)
        .map(|row| {
            let start = l.offsets[row];
            let end = l.offsets[row + 1];
            let gathered = l.columns[start..end]
                .iter()
                .map(|&c| input[c])
                .collect::<Vec<_>>();
            abi_compute::CpuBackend::new()
                .dot(&gathered, &l.weights[start..end])
                .map(|x| x + l.biases[row])
                .map_err(|e| error("Compute", e))
        })
        .collect()
}
pub fn encode(text: &str, support: &[String], valence: f32, intensity: f32) -> Vec<f32> {
    let mut v = vec![0.0; 128];
    let terms = crate::language::tokens(text);
    for term in &terms {
        let h = crate::digest(term.as_bytes());
        let bucket = usize::from_str_radix(&h[..8], 16).unwrap_or(0) % 96;
        v[bucket] += 1.0 / (terms.len().max(1) as f32);
    }
    for (i, s) in support.iter().take(24).enumerate() {
        v[96 + i] = (s.len().min(1000) as f32) / 1000.0;
    }
    v[120] = valence;
    v[121] = intensity;
    v[122] = (terms.len().min(100) as f32) / 100.0;
    v[123] = support.len().min(24) as f32 / 24.0;
    v
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_and_validated() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        assert_eq!(
            n.run(&input, false, 9, None).unwrap().values,
            n.run(&input, false, 9, None).unwrap().values
        );
        let mut broken = n.clone();
        broken.layers[0].columns[0] = 200;
        assert!(broken.validate().is_err());
    }
    struct Failing;
    impl Accelerator for Failing {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn layer(&self, _: &Layer, _: &[f32]) -> Result<Vec<f32>> {
            Err(crate::error("Accelerator", "unavailable"))
        }
    }
    #[test]
    fn missing_or_failing_accelerator_falls_back_to_cpu() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        let cpu = n.run(&input, false, 9, None).unwrap();
        assert_eq!(cpu.backend, "cpu");
        let fell = n.run(&input, false, 9, Some(&Failing)).unwrap();
        assert_eq!(fell.backend, "cpu");
        assert!(fell.fallback.is_some());
        assert_eq!(fell.values, cpu.values);
    }
}
