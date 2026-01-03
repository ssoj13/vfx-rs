//! Benchmarks for VFX-RS operations.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Import crates for benchmarking
use vfx_lut::{Lut1D, Lut3D, Interpolation};
use vfx_math::simd;
use vfx_transfer::{srgb, pq, gamma};
use vfx_color::cdl::Cdl;

/// Benchmark transfer function EOTF/OETF operations.
fn bench_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer");

    // Test different input sizes
    for size in [1000, 10000, 100000].iter() {
        let values: Vec<f32> = (0..*size).map(|i| i as f32 / *size as f32).collect();

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("srgb_eotf", size), &values, |b, v| {
            b.iter(|| {
                v.iter().map(|&x| srgb::eotf(black_box(x))).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("srgb_oetf", size), &values, |b, v| {
            b.iter(|| {
                v.iter().map(|&x| srgb::oetf(black_box(x))).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("pq_eotf", size), &values, |b, v| {
            b.iter(|| {
                v.iter().map(|&x| pq::eotf(black_box(x))).collect::<Vec<_>>()
            })
        });

        group.bench_with_input(BenchmarkId::new("gamma_2.2", size), &values, |b, v| {
            b.iter(|| {
                v.iter().map(|&x| gamma::gamma_eotf(black_box(x), 2.2)).collect::<Vec<_>>()
            })
        });
    }

    group.finish();
}

/// Benchmark 1D LUT operations.
fn bench_lut1d(c: &mut Criterion) {
    let mut group = c.benchmark_group("lut1d");

    let lut_256 = Lut1D::gamma(256, 2.2);
    let lut_1024 = Lut1D::gamma(1024, 2.2);
    let lut_4096 = Lut1D::gamma(4096, 2.2);

    let values: Vec<f32> = (0..10000).map(|i| i as f32 / 10000.0).collect();
    group.throughput(Throughput::Elements(10000));

    group.bench_function("apply_256", |b| {
        b.iter(|| {
            values.iter().map(|&v| lut_256.apply(black_box(v))).collect::<Vec<_>>()
        })
    });

    group.bench_function("apply_1024", |b| {
        b.iter(|| {
            values.iter().map(|&v| lut_1024.apply(black_box(v))).collect::<Vec<_>>()
        })
    });

    group.bench_function("apply_4096", |b| {
        b.iter(|| {
            values.iter().map(|&v| lut_4096.apply(black_box(v))).collect::<Vec<_>>()
        })
    });

    group.finish();
}

/// Benchmark 3D LUT operations.
fn bench_lut3d(c: &mut Criterion) {
    let mut group = c.benchmark_group("lut3d");

    let lut_17 = Lut3D::identity(17);
    let lut_33 = Lut3D::identity(33);
    let lut_65 = Lut3D::identity(65);
    let lut_tetra = Lut3D::identity(33).with_interpolation(Interpolation::Tetrahedral);

    let pixels: Vec<[f32; 3]> = (0..10000)
        .map(|i| {
            let t = i as f32 / 10000.0;
            [t, t * 0.8, t * 0.6]
        })
        .collect();

    group.throughput(Throughput::Elements(10000));

    group.bench_function("trilinear_17", |b| {
        b.iter(|| {
            pixels.iter().map(|&p| lut_17.apply(black_box(p))).collect::<Vec<_>>()
        })
    });

    group.bench_function("trilinear_33", |b| {
        b.iter(|| {
            pixels.iter().map(|&p| lut_33.apply(black_box(p))).collect::<Vec<_>>()
        })
    });

    group.bench_function("trilinear_65", |b| {
        b.iter(|| {
            pixels.iter().map(|&p| lut_65.apply(black_box(p))).collect::<Vec<_>>()
        })
    });

    group.bench_function("tetrahedral_33", |b| {
        b.iter(|| {
            pixels.iter().map(|&p| lut_tetra.apply(black_box(p))).collect::<Vec<_>>()
        })
    });

    group.finish();
}

/// Benchmark CDL operations.
fn bench_cdl(c: &mut Criterion) {
    let mut group = c.benchmark_group("cdl");

    let cdl = Cdl::new()
        .with_slope([1.1, 1.0, 0.9])
        .with_offset([0.01, 0.0, -0.01])
        .with_power([1.0, 1.0, 1.0])
        .with_saturation(1.1);

    let mut pixels: Vec<[f32; 3]> = (0..10000)
        .map(|i| {
            let t = i as f32 / 10000.0;
            [t, t * 0.8, t * 0.6]
        })
        .collect();

    group.throughput(Throughput::Elements(10000));

    group.bench_function("apply", |b| {
        b.iter(|| {
            for pixel in &mut pixels {
                cdl.apply(black_box(pixel));
            }
        })
    });

    group.bench_function("apply_buffer", |b| {
        b.iter(|| {
            cdl.apply_buffer(black_box(&mut pixels));
        })
    });

    group.finish();
}

/// Benchmark SIMD operations.
fn bench_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd");

    let values: Vec<f32> = (0..10000).map(|i| i as f32 / 10000.0).collect();
    group.throughput(Throughput::Elements(10000));

    // Compare scalar vs SIMD mul_add
    group.bench_function("scalar_mul_add", |b| {
        b.iter(|| {
            values.iter().map(|&v| black_box(v) * 2.0 + 0.1).collect::<Vec<_>>()
        })
    });

    group.bench_function("simd_batch_mul_add", |b| {
        b.iter(|| simd::batch_mul_add(black_box(&values), 2.0, 0.1))
    });

    // In-place vs allocating
    let mut values_mut = values.clone();
    group.bench_function("simd_batch_mul_add_inplace", |b| {
        b.iter(|| {
            simd::batch_mul_add_inplace(black_box(&mut values_mut), 2.0, 0.1);
        })
    });

    // Clamp operations
    group.bench_function("scalar_clamp", |b| {
        b.iter(|| {
            values.iter().map(|&v| black_box(v).clamp(0.0, 1.0)).collect::<Vec<_>>()
        })
    });

    group.bench_function("simd_batch_clamp", |b| {
        b.iter(|| simd::batch_clamp01(black_box(&values)))
    });

    // Power operations
    group.bench_function("scalar_pow_2", |b| {
        b.iter(|| {
            values.iter().map(|&v| black_box(v).powf(2.0)).collect::<Vec<_>>()
        })
    });

    group.bench_function("simd_batch_pow_2", |b| {
        b.iter(|| simd::batch_pow(black_box(&values), 2.0))
    });

    group.finish();
}

/// Benchmark batch pixel operations.
fn bench_pixels(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixels");

    // Create test pixel buffers of various sizes
    for &pixel_count in &[256 * 256, 1024 * 1024, 1920 * 1080] {
        let pixels: Vec<[f32; 3]> = (0..pixel_count)
            .map(|i| {
                let t = i as f32 / pixel_count as f32;
                [t, t * 0.8, t * 0.6]
            })
            .collect();

        group.throughput(Throughput::Elements(pixel_count as u64));

        group.bench_with_input(
            BenchmarkId::new("sum_rgb", pixel_count),
            &pixels,
            |b, pixels| {
                b.iter(|| {
                    let mut sum = 0.0f32;
                    for p in pixels.iter() {
                        sum += p[0] + p[1] + p[2];
                    }
                    black_box(sum)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("transform_rgb", pixel_count),
            &pixels,
            |b, pixels| {
                b.iter(|| {
                    pixels.iter()
                        .map(|p| [
                            p[0] * 1.1 + 0.01,
                            p[1] * 1.0 + 0.0,
                            p[2] * 0.9 - 0.01,
                        ])
                        .collect::<Vec<_>>()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_transfer,
    bench_lut1d,
    bench_lut3d,
    bench_cdl,
    bench_simd,
    bench_pixels,
);

criterion_main!(benches);
