# GPU Compute

wgpu compute shader architecture for GPU-accelerated processing.

## Overview

vfx-compute uses wgpu for cross-platform GPU compute:

- **Vulkan** (Linux, Windows)
- **Metal** (macOS, iOS)
- **DX12** (Windows)
- **WebGPU** (Browser, future)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Processor (public API)                   │
├─────────────────────────────────────────────────────────────┤
│                        Backend                               │
│    ┌──────────────────┐     ┌──────────────────────┐        │
│    │   CpuBackend     │     │     WgpuBackend      │        │
│    │   (rayon)        │     │  (compute shaders)   │        │
│    └──────────────────┘     └──────────────────────┘        │
├─────────────────────────────────────────────────────────────┤
│                      GpuPrimitives trait                     │
│    apply_exposure, apply_matrix, apply_lut, ...             │
└─────────────────────────────────────────────────────────────┘
```

## Device Initialization

```rust
pub async fn create_device() -> ComputeResult<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .ok_or(ComputeError::NoAdapter)?;
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        )
        .await?;
    
    Ok((device, queue))
}
```

## Buffer Management

### Image Upload

```rust
pub fn upload_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[f32],
) -> wgpu::Buffer {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Image Buffer"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE 
             | wgpu::BufferUsages::COPY_SRC 
             | wgpu::BufferUsages::COPY_DST,
    });
    
    buffer
}
```

### Image Download

```rust
pub async fn download_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    size: usize,
) -> Vec<f32> {
    // Create staging buffer for readback
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: (size * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    // Copy GPU buffer → staging
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (size * 4) as u64);
    queue.submit([encoder.finish()]);
    
    // Map and read
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    
    let data = slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    
    drop(data);
    staging.unmap();
    
    result
}
```

## Compute Shaders

### Shader Structure

```wgsl
// exposure.wgsl
@group(0) @binding(0)
var<storage, read_write> pixels: array<f32>;

struct Params {
    factor: f32,
    width: u32,
    height: u32,
    channels: u32,
}

@group(0) @binding(1)
var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total = params.width * params.height * params.channels;
    
    if (idx >= total) {
        return;
    }
    
    pixels[idx] = pixels[idx] * params.factor;
}
```

### Pipeline Creation

```rust
fn create_exposure_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Exposure Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/exposure.wgsl").into()),
    });
    
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Exposure Layout"),
        entries: &[
            // Pixel buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Params uniform
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Exposure Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Exposure Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}
```

### Dispatch

```rust
fn dispatch_exposure(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    image_buffer: &wgpu::Buffer,
    params: &ExposureParams,
    pixel_count: u32,
) {
    // Create params uniform
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    
    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Exposure Bind Group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: image_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });
    
    // Dispatch
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        
        let workgroups = (pixel_count + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    
    queue.submit([encoder.finish()]);
}
```

## 3D LUT Application

### Texture-Based LUT

```rust
fn upload_lut_3d(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    lut: &Lut3D,
) -> wgpu::Texture {
    let size = lut.size as u32;
    
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("3D LUT"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: size,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&lut.data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size * 16),  // 4 floats * 4 bytes
            rows_per_image: Some(size),
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: size,
        },
    );
    
    texture
}
```

### LUT Shader

```wgsl
@group(0) @binding(0) var<storage, read_write> pixels: array<vec4<f32>>;
@group(0) @binding(1) var lut_texture: texture_3d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;

@compute @workgroup_size(256)
fn apply_lut(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    var color = pixels[idx];
    
    // Sample 3D texture with trilinear interpolation
    let coords = clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let result = textureSampleLevel(lut_texture, lut_sampler, coords, 0.0);
    
    pixels[idx] = vec4<f32>(result.rgb, color.a);
}
```

## Tile Processing

For images larger than GPU memory:

```rust
pub fn process_tiled<F>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &mut [f32],
    width: usize,
    height: usize,
    channels: usize,
    tile_size: usize,
    process: F,
) -> ComputeResult<()>
where
    F: Fn(&wgpu::Device, &wgpu::Queue, &wgpu::Buffer, u32, u32) -> ComputeResult<()>,
{
    for ty in (0..height).step_by(tile_size) {
        for tx in (0..width).step_by(tile_size) {
            let tw = tile_size.min(width - tx);
            let th = tile_size.min(height - ty);
            
            // Extract tile
            let tile_data = extract_tile(image, width, height, channels, tx, ty, tw, th);
            
            // Upload to GPU
            let buffer = upload_image(device, queue, &tile_data);
            
            // Process
            process(device, queue, &buffer, tw as u32, th as u32)?;
            
            // Download and insert
            let result = download_image(device, queue, &buffer, tw * th * channels).await;
            insert_tile(image, width, height, channels, tx, ty, tw, th, &result);
        }
    }
    
    Ok(())
}
```

## Performance Considerations

### Workgroup Size

```wgsl
// Good: Power of 2, typically 64-256
@compute @workgroup_size(256)

// Bad: Non-power-of-2
@compute @workgroup_size(100)
```

### Memory Coalescing

```wgsl
// Good: Sequential access
let idx = global_id.x;
pixels[idx] = process(pixels[idx]);

// Bad: Strided access
let idx = global_id.x * stride;  // Cache misses
```

### Minimize Transfers

```rust
// Bad: Upload, process, download for each operation
for op in operations {
    upload();
    dispatch(op);
    download();
}

// Good: Batch operations on GPU
upload();
for op in operations {
    dispatch(op);
}
download();
```
