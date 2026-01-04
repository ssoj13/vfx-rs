//! WGSL shader sources for GPU compute pipelines.
//! These are used by the wgpu backend when the `wgpu` feature is enabled.

#![allow(dead_code)] // Shaders used by wgpu backend

/// Color matrix 4x4 transform.
pub const COLOR_MATRIX: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, 0
@group(0) @binding(3) var<uniform> matrix: mat4x4<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let base = px * c;

    let r = src[base];
    let g = src[base + 1];
    let b = src[base + 2];
    let a = select(1.0, src[base + 3], c >= 4);

    let inp = vec4<f32>(r, g, b, a);
    let out = matrix * inp;

    dst[base] = out.x;
    dst[base + 1] = out.y;
    dst[base + 2] = out.z;
    if c >= 4 { dst[base + 3] = out.w; }
}
"#;

/// CDL (Color Decision List) transform.
pub const CDL: &str = r#"
struct CdlParams {
    slope: vec3<f32>,
    _pad0: f32,
    offset: vec3<f32>,
    _pad1: f32,
    power: vec3<f32>,
    saturation: f32,
}

@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;
@group(0) @binding(3) var<uniform> cdl: CdlParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let base = px * c;

    // CDL: out = (in * slope + offset) ^ power
    var r = pow(max(src[base] * cdl.slope.x + cdl.offset.x, 0.0), cdl.power.x);
    var g = pow(max(src[base + 1] * cdl.slope.y + cdl.offset.y, 0.0), cdl.power.y);
    var b = pow(max(src[base + 2] * cdl.slope.z + cdl.offset.z, 0.0), cdl.power.z);

    // Saturation
    if cdl.saturation != 1.0 {
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r = luma + cdl.saturation * (r - luma);
        g = luma + cdl.saturation * (g - luma);
        b = luma + cdl.saturation * (b - luma);
    }

    dst[base] = r;
    dst[base + 1] = g;
    dst[base + 2] = b;
    if c >= 4 { dst[base + 3] = src[base + 3]; }
}
"#;

/// 1D LUT interpolation.
pub const LUT1D: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, lut_size
@group(0) @binding(3) var<storage, read> lut: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let lut_size = dims.w;
    let scale = f32(lut_size - 1);
    let base = px * c;

    for (var ch = 0u; ch < min(c, 3u); ch = ch + 1) {
        let v = clamp(src[base + ch], 0.0, 1.0) * scale;
        let i0 = u32(v);
        let i1 = min(i0 + 1, lut_size - 1);
        let f = v - f32(i0);

        let v0 = lut[i0 * 3 + ch];
        let v1 = lut[i1 * 3 + ch];
        dst[base + ch] = v0 + f * (v1 - v0);
    }
    if c >= 4 { dst[base + 3] = src[base + 3]; }
}
"#;

/// 3D LUT trilinear interpolation.
pub const LUT3D: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, lut_size
@group(0) @binding(3) var<storage, read> lut: array<f32>;

fn lut_idx(ri: u32, gi: u32, bi: u32, ch: u32, s: u32) -> f32 {
    return lut[(bi * s * s + gi * s + ri) * 3 + ch];
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let s = dims.w;
    let scale = f32(s - 1);
    let base = px * c;

    let r = clamp(src[base], 0.0, 1.0) * scale;
    let g = clamp(src[base + 1], 0.0, 1.0) * scale;
    let b = clamp(src[base + 2], 0.0, 1.0) * scale;

    let r0 = min(u32(r), s - 1);
    let g0 = min(u32(g), s - 1);
    let b0 = min(u32(b), s - 1);
    let r1 = min(r0 + 1, s - 1);
    let g1 = min(g0 + 1, s - 1);
    let b1 = min(b0 + 1, s - 1);

    let fr = r - f32(r0);
    let fg = g - f32(g0);
    let fb = b - f32(b0);

    for (var ch = 0u; ch < 3u; ch = ch + 1) {
        let c000 = lut_idx(r0, g0, b0, ch, s);
        let c100 = lut_idx(r1, g0, b0, ch, s);
        let c010 = lut_idx(r0, g1, b0, ch, s);
        let c110 = lut_idx(r1, g1, b0, ch, s);
        let c001 = lut_idx(r0, g0, b1, ch, s);
        let c101 = lut_idx(r1, g0, b1, ch, s);
        let c011 = lut_idx(r0, g1, b1, ch, s);
        let c111 = lut_idx(r1, g1, b1, ch, s);

        let c00 = c000 + fr * (c100 - c000);
        let c10 = c010 + fr * (c110 - c010);
        let c01 = c001 + fr * (c101 - c001);
        let c11 = c011 + fr * (c111 - c011);

        let c0 = c00 + fg * (c10 - c00);
        let c1 = c01 + fg * (c11 - c01);

        dst[base + ch] = c0 + fb * (c1 - c0);
    }
    if c >= 4 { dst[base + 3] = src[base + 3]; }
}
"#;

/// Bilinear resize.
pub const RESIZE: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> src_dims: vec4<u32>;  // sw, sh, c, 0
@group(0) @binding(3) var<uniform> dst_dims: vec4<u32>;  // dw, dh, 0, 0

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dx = id.x;
    let dy = id.y;
    let dw = dst_dims.x;
    let dh = dst_dims.y;
    if dx >= dw || dy >= dh { return; }

    let sw = src_dims.x;
    let sh = src_dims.y;
    let c = src_dims.z;

    let sx = f32(sw) / f32(dw);
    let sy = f32(sh) / f32(dh);

    let fx = f32(dx) * sx;
    let fy = f32(dy) * sy;

    let x0 = min(u32(fx), sw - 1);
    let y0 = min(u32(fy), sh - 1);
    let x1 = min(x0 + 1, sw - 1);
    let y1 = min(y0 + 1, sh - 1);

    let ffx = fx - f32(x0);
    let ffy = fy - f32(y0);

    let dst_base = (dy * dw + dx) * c;

    for (var ch = 0u; ch < c; ch = ch + 1) {
        let c00 = src[(y0 * sw + x0) * c + ch];
        let c10 = src[(y0 * sw + x1) * c + ch];
        let c01 = src[(y1 * sw + x0) * c + ch];
        let c11 = src[(y1 * sw + x1) * c + ch];

        let top = c00 + ffx * (c10 - c00);
        let bot = c01 + ffx * (c11 - c01);
        dst[dst_base + ch] = top + ffy * (bot - top);
    }
}
"#;

/// Gaussian blur (horizontal pass).
pub const BLUR_H: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, radius
@group(0) @binding(3) var<storage, read> kernel: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let w = dims.x;
    let h = dims.y;
    let c = dims.z;
    let r = i32(dims.w);

    let y = px / w;
    let x = px % w;
    if y >= h { return; }

    let k_size = u32(r * 2 + 1);
    let base = (y * w + x) * c;

    for (var ch = 0u; ch < c; ch = ch + 1) {
        var acc = 0.0;
        for (var ki = 0u; ki < k_size; ki = ki + 1) {
            let sx = clamp(i32(x) + i32(ki) - r, 0, i32(w) - 1);
            acc = acc + src[(y * w + u32(sx)) * c + ch] * kernel[ki];
        }
        dst[base + ch] = acc;
    }
}
"#;

/// Gaussian blur (vertical pass).
pub const BLUR_V: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, radius
@group(0) @binding(3) var<storage, read> kernel: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let w = dims.x;
    let h = dims.y;
    let c = dims.z;
    let r = i32(dims.w);

    let y = px / w;
    let x = px % w;
    if y >= h { return; }

    let k_size = u32(r * 2 + 1);
    let base = (y * w + x) * c;

    for (var ch = 0u; ch < c; ch = ch + 1) {
        var acc = 0.0;
        for (var ki = 0u; ki < k_size; ki = ki + 1) {
            let sy = clamp(i32(y) + i32(ki) - r, 0, i32(h) - 1);
            acc = acc + src[(u32(sy) * w + x) * c + ch] * kernel[ki];
        }
        dst[base + ch] = acc;
    }
}
"#;
