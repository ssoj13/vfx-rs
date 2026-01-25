# vfx-bench

Performance benchmarks for vfx-rs.

## Purpose

Criterion-based benchmarks measuring performance of critical operations. Used for optimization, regression detection, and comparison with OIIO/OCIO.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench -p vfx-bench

# Run specific benchmark group
cargo bench -p vfx-bench -- transfer
cargo bench -p vfx-bench -- lut3d

# Generate HTML report
cargo bench -p vfx-bench -- --save-baseline main
```

## Benchmark Categories

The single `vfx_bench` target includes these groups:

### Transfer Functions

```
transfer/srgb_eotf/1000      time: [x.xx µs ...]
transfer/srgb_oetf/10000     time: [x.xx µs ...]
transfer/pq_eotf/100000      time: [x.xx ms ...]
transfer/gamma_2.2/10000     time: [x.xx µs ...]
```

### 1D LUT Operations

```
lut1d/apply_256              time: [x.xx µs ...]
lut1d/apply_1024             time: [x.xx µs ...]
lut1d/apply_4096             time: [x.xx µs ...]
```

### 3D LUT Operations

```
lut3d/trilinear_17           time: [x.xx ms ...]
lut3d/trilinear_33           time: [x.xx ms ...]
lut3d/trilinear_65           time: [x.xx ms ...]
lut3d/tetrahedral_33         time: [x.xx ms ...]
```

### CDL Operations

```
cdl/apply                    time: [x.xx µs ...]
cdl/apply_buffer             time: [x.xx µs ...]
```

### SIMD Operations

```
simd/scalar_mul_add          time: [x.xx µs ...]
simd/simd_batch_mul_add      time: [x.xx µs ...]
simd/simd_batch_clamp        time: [x.xx µs ...]
simd/simd_batch_pow_2        time: [x.xx µs ...]
```

### Pixel Batch Operations

```
pixels/sum_rgb/65536         time: [x.xx µs ...]
pixels/transform_rgb/1048576 time: [x.xx ms ...]
pixels/transform_rgb/2073600 time: [x.xx ms ...] (1920x1080)
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

## Notes

**GPU benchmarks:** The vfx-bench crate currently does not include GPU benchmarks or a `gpu` feature.

**I/O benchmarks:** I/O performance benchmarks (EXR read/write, PNG, etc.) are not currently included in vfx-bench.

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
vfx-math = { workspace = true }
vfx-lut = { workspace = true }
vfx-transfer = { workspace = true }
vfx-color = { workspace = true }

[dev-dependencies]
criterion = { workspace = true }

[[bench]]
name = "vfx_bench"
harness = false
```

**Note:** Only one benchmark target (`vfx_bench`) is defined, which contains all benchmark groups.

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
