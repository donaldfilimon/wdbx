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
    pub fn run(&self, input: &[f32], familiar: bool, seed: u64, gpu: bool) -> Result<Synthesis> {
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
        let mut used_gpu = gpu;
        for l in &self.layers {
            let sigmoid = rng.sample() < if familiar { 0.75 } else { 0.25 };
            let raw = if used_gpu {
                match gpu_layer(l, &values) {
                    Ok(v) => v,
                    Err(e) => {
                        fallback = Some(e.message);
                        used_gpu = false;
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
                        1.0 / (1.0 + (-x.clamp(-30.0, 30.0)).exp())
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
            backend: if used_gpu { "wgpu" } else { "cpu" }.into(),
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
fn gpu_layer(l: &Layer, input: &[f32]) -> Result<Vec<f32>> {
    pollster::block_on(async {
        use wgpu::util::DeviceExt;
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or_else(|| {
                error(
                    "BackendUnavailable",
                    "No compatible GPU adapter; CPU reference used",
                )
            })?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WDBX transient"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .map_err(|e| error("BackendUnavailable", e))?;
        let mut packed = vec![0.0f32; l.outputs * l.inputs];
        for row in 0..l.outputs {
            for i in l.offsets[row]..l.offsets[row + 1] {
                packed[row * l.inputs + l.columns[i]] += l.weights[i];
            }
        }
        let shader = format!(
            "@group(0) @binding(0) var<storage,read> x:array<f32>; @group(0) @binding(1) var<storage,read> w:array<f32>; @group(0) @binding(2) var<storage,read_write> y:array<f32>; @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){{let row=id.x;if(row>={outputs}u){{return;}}var sum=0.0;for(var c=0u;c<{inputs}u;c=c+1u){{sum=sum+x[c]*w[row*{inputs}u+c];}}y[row]=sum;}}",
            outputs = l.outputs,
            inputs = l.inputs
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: "main",
        });
        let x = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let w = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let size = (l.outputs * 4) as u64;
        let y = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: w.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: y.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((l.outputs as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&y, 0, &read, 0, size);
        queue.submit(Some(encoder.finish()));
        let slice = read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| error("Compute", e))?
            .map_err(|e| error("Compute", e))?;
        let data = slice.get_mapped_range();
        let out = bytemuck::cast_slice::<u8, f32>(&data)
            .iter()
            .zip(&l.biases)
            .map(|(a, b)| a + b)
            .collect();
        drop(data);
        read.unmap();
        Ok(out)
    })
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
            n.run(&input, false, 9, false).unwrap().values,
            n.run(&input, false, 9, false).unwrap().values
        );
        let mut broken = n.clone();
        broken.layers[0].columns[0] = 200;
        assert!(broken.validate().is_err());
    }
    #[test]
    #[ignore = "requires qualified GPU"]
    fn cpu_gpu_parity() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        let cpu = n.run(&input, false, 9, false).unwrap();
        let gpu = n.run(&input, false, 9, true).unwrap();
        assert_eq!(gpu.backend, "wgpu");
        let mut max_delta = 0.0_f32;
        for (index, (a, b)) in cpu.values.iter().zip(&gpu.values).enumerate() {
            let delta = (a - b).abs();
            println!("output[{index}] delta={delta:.8}");
            assert!(delta < 1e-4, "output[{index}] delta {delta} exceeded 1e-4");
            max_delta = max_delta.max(delta);
        }
        println!(
            "backend={} max_delta={max_delta:.8} threshold=0.0001",
            gpu.backend
        );
    }
}
