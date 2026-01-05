# Benchmarks

The `vfx-bench` crate contains performance benchmarks using Criterion.

## Running Benchmarks

```bash
# All benchmarks
cargo bench -p vfx-bench

# Specific benchmark
cargo bench -p vfx-bench -- resize

# With baseline comparison
cargo bench -p vfx-bench -- --baseline main

# Generate plots (requires gnuplot)
cargo bench -p vfx-bench -- --plotting-backend gnuplot
```

## Benchmark Categories

### IO Benchmarks

Image loading/saving performance:

```rust
fn bench_exr_load(c: &mut Criterion) {
    let path = "test/images/4k.exr";
    
    c.bench_function("exr_load_4k", |b| {
        b.iter(|| load_image(black_box(path)))
    });
}
```

Results vary by:
- Image dimensions
- Pixel format (half vs float)
- Compression type
- Channel count

### Resize Benchmarks

```rust
fn bench_resize(c: &mut Criterion) {
    let data = vec![0.5f32; 1920 * 1080 * 3];
    
    let mut group = c.benchmark_group("resize");
    
    for filter in [Filter::Nearest, Filter::Bilinear, Filter::Lanczos3] {
        group.bench_function(format!("{:?}", filter), |b| {
            b.iter(|| resize_f32(&data, 1920, 1080, 3, 960, 540, filter))
        });
    }
    
    group.finish();
}
```

Typical results (1080p → 540p, 3-channel):
- Nearest: ~2ms
- Bilinear: ~8ms
- Lanczos3: ~25ms

### Color Transform Benchmarks

```rust
fn bench_color_pipeline(c: &mut Criterion) {
    let data = vec![0.18f32; 1920 * 1080 * 3];
    
    c.bench_function("srgb_to_linear_1080p", |b| {
        b.iter(|| {
            let mut d = data.clone();
            apply_srgb_to_linear(&mut d);
        })
    });
}
```

### LUT Application

```rust
fn bench_lut3d(c: &mut Criterion) {
    let lut = Lut3D::load("test/luts/film.cube").unwrap();
    let data = vec![0.5f32; 1920 * 1080 * 3];
    
    let mut group = c.benchmark_group("lut3d");
    
    group.bench_function("trilinear", |b| {
        b.iter(|| lut.apply_trilinear(&data))
    });
    
    group.bench_function("tetrahedral", |b| {
        b.iter(|| lut.apply_tetrahedral(&data))
    });
    
    group.finish();
}
```

## Writing Benchmarks

### Basic Benchmark

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("operation_name", |b| {
        b.iter(|| {
            // Code to benchmark
            // Use black_box() to prevent optimization
            black_box(expensive_operation())
        })
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

### Parameterized Benchmarks

```rust
fn bench_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_by_size");
    
    for size in [256, 512, 1024, 2048] {
        let data = vec![0.5f32; size * size * 3];
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &data,
            |b, data| {
                b.iter(|| resize_f32(data, size, size, 3, size/2, size/2, Filter::Bilinear))
            }
        );
    }
    
    group.finish();
}
```

### Throughput Measurement

```rust
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_throughput");
    
    let data = vec![0.5f32; 4096 * 4096 * 4];
    let bytes = data.len() * 4;
    
    group.throughput(Throughput::Bytes(bytes as u64));
    
    group.bench_function("exr_save_4k", |b| {
        b.iter(|| save_exr("bench_out.exr", &data, 4096, 4096, 4))
    });
    
    group.finish();
}
```

## Interpreting Results

Criterion outputs:

```
resize/Lanczos3     time:   [24.532 ms 24.891 ms 25.287 ms]
                    change: [-2.1234% -0.5678% +1.0123%] (p = 0.12 > 0.05)
                    No change in performance detected.
```

- **time**: [lower bound, estimate, upper bound]
- **change**: comparison to baseline
- **p-value**: statistical significance

## Performance Targets

Rough targets for common operations:

| Operation | Resolution | Target |
|-----------|-----------|--------|
| EXR load | 4K RGBA | < 100ms |
| EXR save | 4K RGBA | < 200ms |
| Resize (Lanczos) | 4K → 2K | < 50ms |
| LUT 3D apply | 4K | < 30ms |
| ACES RRT+ODT | 4K | < 20ms |

## Profiling

For detailed performance analysis:

```bash
# CPU profiling with perf (Linux)
perf record --call-graph dwarf cargo bench -p vfx-bench -- --profile-time 10
perf report

# With flamegraph
cargo flamegraph --bench vfx-bench -- resize

# Memory profiling
valgrind --tool=massif cargo bench -p vfx-bench
```

## GPU Benchmarks

When `gpu` feature is enabled:

```rust
#[cfg(feature = "gpu")]
fn bench_gpu_resize(c: &mut Criterion) {
    let processor = Processor::new(Backend::Gpu).unwrap();
    let data = vec![0.5f32; 4096 * 4096 * 4];
    
    c.bench_function("gpu_resize_4k", |b| {
        b.iter(|| processor.resize(&data, 4096, 4096, 2048, 2048))
    });
}
```

GPU benchmarks include transfer overhead, making them realistic for actual usage.

## Continuous Benchmarking

For tracking performance over time:

```bash
# Save baseline
cargo bench -p vfx-bench -- --save-baseline v0.1.0

# Compare against baseline
cargo bench -p vfx-bench -- --baseline v0.1.0
```

CI can catch regressions:

```yaml
- name: Benchmark
  run: |
    cargo bench -p vfx-bench -- --baseline main
    # Fail if >10% regression
```
