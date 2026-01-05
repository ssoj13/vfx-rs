# vfx-bench

Performance benchmarks for vfx-rs.

## Purpose

Criterion-based benchmarks measuring performance of critical operations. Used for optimization, regression detection, and comparison with OIIO/OCIO.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench -p vfx-bench

# Run specific benchmark
cargo bench -p vfx-bench -- resize

# Generate HTML report
cargo bench -p vfx-bench -- --save-baseline main
```

## Benchmark Categories

### I/O Performance

```
io/exr_read_1080p       time: [12.5 ms 12.8 ms 13.1 ms]
io/exr_write_1080p      time: [45.2 ms 46.1 ms 47.0 ms]
io/png_read_1080p       time: [8.3 ms 8.5 ms 8.7 ms]
io/dpx_read_1080p       time: [5.1 ms 5.2 ms 5.4 ms]
```

### Resize Operations

```
resize/lanczos_1080p    time: [18.3 ms 18.7 ms 19.1 ms]
resize/bilinear_1080p   time: [4.2 ms 4.3 ms 4.4 ms]
resize/nearest_1080p    time: [1.1 ms 1.2 ms 1.2 ms]
```

### Color Transforms

```
color/srgb_eotf_1mp     time: [2.1 ms 2.2 ms 2.3 ms]
color/matrix_1mp        time: [1.8 ms 1.9 ms 2.0 ms]
color/rrt_odt_1080p     time: [8.5 ms 8.7 ms 8.9 ms]
```

### LUT Application

```
lut/3d_33_1080p         time: [15.2 ms 15.5 ms 15.8 ms]
lut/3d_65_1080p         time: [16.1 ms 16.4 ms 16.7 ms]
lut/1d_1024_1080p       time: [3.2 ms 3.3 ms 3.4 ms]
```

## Benchmark Structure

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn resize_benchmark(c: &mut Criterion) {
    let sizes = [(1920, 1080), (3840, 2160), (7680, 4320)];
    
    let mut group = c.benchmark_group("resize");
    
    for (w, h) in sizes {
        let img = create_test_image(w, h, 3);
        let data = img.to_f32();
        
        group.bench_with_input(
            BenchmarkId::new("lanczos", format!("{}x{}", w, h)),
            &data,
            |b, data| {
                b.iter(|| {
                    vfx_ops::resize::resize_f32(
                        data, w, h, 3,
                        w / 2, h / 2,
                        vfx_ops::resize::Filter::Lanczos3
                    )
                })
            }
        );
    }
    
    group.finish();
}

criterion_group!(benches, resize_benchmark);
criterion_main!(benches);
```

## Comparing Baselines

```bash
# Save baseline
cargo bench -p vfx-bench -- --save-baseline before

# Make changes...

# Compare
cargo bench -p vfx-bench -- --baseline before
```

Output shows performance difference:

```
resize/lanczos_1080p    time: [18.1 ms 18.5 ms 18.9 ms]
                        change: [-5.2% -3.1% -1.0%] (p = 0.01 < 0.05)
                        Performance has improved.
```

## GPU Benchmarks

When GPU features enabled:

```bash
cargo bench -p vfx-bench --features gpu
```

```
gpu/exposure_4k         time: [2.1 ms 2.2 ms 2.3 ms]
gpu/lut3d_4k            time: [1.5 ms 1.6 ms 1.7 ms]
cpu/exposure_4k         time: [12.5 ms 12.8 ms 13.1 ms]
cpu/lut3d_4k            time: [15.2 ms 15.5 ms 15.8 ms]
```

## Memory Benchmarks

Peak memory usage (via custom harness):

```rust
#[bench]
fn memory_exr_read() {
    let before = get_memory_usage();
    let _img = vfx_io::read("large.exr").unwrap();
    let after = get_memory_usage();
    
    println!("Peak memory: {} MB", (after - before) / 1024 / 1024);
}
```

## Profiling Integration

Generate flamegraphs:

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Run with profiling
cargo flamegraph --bench resize_bench -- --bench
```

## Comparison with OIIO

Benchmark results vs OpenImageIO (example):

| Operation | vfx-rs | OIIO | Ratio |
|-----------|--------|------|-------|
| EXR read 1080p | 12.8 ms | 15.2 ms | 0.84x |
| PNG write 1080p | 45 ms | 52 ms | 0.87x |
| Lanczos resize | 18.5 ms | 22.1 ms | 0.84x |
| 3D LUT apply | 15.5 ms | 18.3 ms | 0.85x |

*Results vary by hardware and configuration.*

## Writing Benchmarks

### Best Practices

1. **Warm up** - Criterion handles this automatically
2. **Multiple iterations** - Report mean with confidence interval
3. **Realistic data** - Use production-like image sizes
4. **Isolate operations** - Benchmark one thing at a time

### Example Template

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    // Setup (not timed)
    let data = prepare_test_data();
    
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            // Timed operation
            let result = my_operation(black_box(&data));
            black_box(result)  // Prevent optimization
        })
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

## Dependencies

```toml
[dependencies]
vfx-core = { workspace = true }
vfx-io = { workspace = true }
vfx-ops = { workspace = true }
vfx-color = { workspace = true }

[dev-dependencies]
criterion = { workspace = true }

[[bench]]
name = "io_bench"
harness = false

[[bench]]
name = "resize_bench"
harness = false
```

## CI Integration

Benchmarks run on CI for regression detection:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench -p vfx-bench -- --noplot
  
- name: Check for regressions
  run: |
    # Compare against main branch baseline
    cargo bench -p vfx-bench -- --baseline main --noplot
```
