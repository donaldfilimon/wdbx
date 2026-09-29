use crate::{Result, error};
pub use specimen_kernel::neural::*;

/// GPU layer evaluation through wgpu; the kernel falls back to CPU on error.
pub struct WgpuAccelerator;

impl Accelerator for WgpuAccelerator {
    fn name(&self) -> &'static str {
        "wgpu"
    }
    fn layer(&self, l: &Layer, input: &[f32]) -> Result<Vec<f32>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires qualified GPU"]
    fn cpu_gpu_parity() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        let cpu = n.run(&input, false, 9, None).unwrap();
        let gpu = n.run(&input, false, 9, Some(&WgpuAccelerator)).unwrap();
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
