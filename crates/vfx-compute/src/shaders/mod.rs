//! WGSL shader sources for GPU compute pipelines.

#![cfg_attr(not(feature = "wgpu"), allow(dead_code))]

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

    // Saturation (Rec.709 luma coefficients - see vfx_core::pixel::REC709_LUMA_*)
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

/// 3D LUT tetrahedral interpolation (higher quality).
pub const LUT3D_TETRAHEDRAL: &str = r#"
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

    var r0 = min(u32(r), s - 2);
    var g0 = min(u32(g), s - 2);
    var b0 = min(u32(b), s - 2);
    let r1 = r0 + 1;
    let g1 = g0 + 1;
    let b1 = b0 + 1;

    let fr = r - f32(r0);
    let fg = g - f32(g0);
    let fb = b - f32(b0);

    for (var ch = 0u; ch < 3u; ch = ch + 1) {
        // Fetch all 8 corners
        let c000 = lut_idx(r0, g0, b0, ch, s);
        let c100 = lut_idx(r1, g0, b0, ch, s);
        let c010 = lut_idx(r0, g1, b0, ch, s);
        let c110 = lut_idx(r1, g1, b0, ch, s);
        let c001 = lut_idx(r0, g0, b1, ch, s);
        let c101 = lut_idx(r1, g0, b1, ch, s);
        let c011 = lut_idx(r0, g1, b1, ch, s);
        let c111 = lut_idx(r1, g1, b1, ch, s);

        // Tetrahedral interpolation: 6 tetrahedra based on fr,fg,fb ordering
        var result: f32;
        if fr > fg {
            if fg > fb {
                // fr > fg > fb: tetrahedron (0,0,0)-(1,0,0)-(1,1,0)-(1,1,1)
                result = c000 + fr*(c100-c000) + fg*(c110-c100) + fb*(c111-c110);
            } else if fr > fb {
                // fr > fb > fg: tetrahedron (0,0,0)-(1,0,0)-(1,0,1)-(1,1,1)
                result = c000 + fr*(c100-c000) + fb*(c101-c100) + fg*(c111-c101);
            } else {
                // fb > fr > fg: tetrahedron (0,0,0)-(0,0,1)-(1,0,1)-(1,1,1)
                result = c000 + fb*(c001-c000) + fr*(c101-c001) + fg*(c111-c101);
            }
        } else {
            if fr > fb {
                // fg > fr > fb: tetrahedron (0,0,0)-(0,1,0)-(1,1,0)-(1,1,1)
                result = c000 + fg*(c010-c000) + fr*(c110-c010) + fb*(c111-c110);
            } else if fg > fb {
                // fg > fb > fr: tetrahedron (0,0,0)-(0,1,0)-(0,1,1)-(1,1,1)
                result = c000 + fg*(c010-c000) + fb*(c011-c010) + fr*(c111-c011);
            } else {
                // fb > fg > fr: tetrahedron (0,0,0)-(0,0,1)-(0,1,1)-(1,1,1)
                result = c000 + fb*(c001-c000) + fg*(c011-c001) + fr*(c111-c011);
            }
        }
        dst[base + ch] = result;
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

/// Hue curves (Hue vs Hue/Sat/Lum) shader.
pub const HUE_CURVES: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, lut_size
@group(0) @binding(3) var<storage, read> hue_vs_hue: array<f32>;
@group(0) @binding(4) var<storage, read> hue_vs_sat: array<f32>;
@group(0) @binding(5) var<storage, read> hue_vs_lum: array<f32>;

// RGB to HSL
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> vec3<f32> {
    let mx = max(max(r, g), b);
    let mn = min(min(r, g), b);
    let l = (mx + mn) * 0.5;

    if mx - mn < 1e-6 {
        return vec3<f32>(0.0, 0.0, l);
    }

    let d = mx - mn;
    let s = select(d / (mx + mn), d / (2.0 - mx - mn), l > 0.5);

    var h: f32;
    if mx == r {
        h = (g - b) / d;
        if g < b { h = h + 6.0; }
    } else if mx == g {
        h = (b - r) / d + 2.0;
    } else {
        h = (r - g) / d + 4.0;
    }
    h = h / 6.0;

    return vec3<f32>(h, s, l);
}

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if t < 0.0 { t = t + 1.0; }
    if t > 1.0 { t = t - 1.0; }
    if t < 1.0/6.0 { return p + (q - p) * 6.0 * t; }
    if t < 0.5 { return q; }
    if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
    return p;
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> vec3<f32> {
    if s < 1e-6 {
        return vec3<f32>(l, l, l);
    }
    let q = select(l * (1.0 + s), l + s - l * s, l >= 0.5);
    let p = 2.0 * l - q;
    return vec3<f32>(
        hue_to_rgb(p, q, h + 1.0/3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0/3.0)
    );
}

// Sample LUT with wrap-around interpolation
fn sample_lut(lut_base: u32, lut_size: u32, hue: f32) -> f32 {
    let h = hue - floor(hue);  // wrap to 0-1
    let pos = h * f32(lut_size - 1);
    let i0 = u32(pos);
    let i1 = (i0 + 1) % lut_size;
    let f = pos - f32(i0);
    // Note: we use different array accessors based on which LUT
    return 0.0; // placeholder - actual impl uses binding index
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let lut_size = dims.w;
    let base = px * c;

    let r = src[base];
    let g = src[base + 1];
    let b = src[base + 2];

    // RGB to HSL
    let hsl = rgb_to_hsl(r, g, b);
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;

    // Sample LUTs
    let h_norm = h - floor(h);
    let pos = h_norm * f32(lut_size - 1);
    let i0 = u32(pos);
    let i1 = min(i0 + 1, lut_size - 1);
    let f = pos - f32(i0);

    let hue_shift = hue_vs_hue[i0] + f * (hue_vs_hue[i1] - hue_vs_hue[i0]);
    let sat_mult = hue_vs_sat[i0] + f * (hue_vs_sat[i1] - hue_vs_sat[i0]);
    let lum_offset = hue_vs_lum[i0] + f * (hue_vs_lum[i1] - hue_vs_lum[i0]);

    // Apply adjustments
    var new_h = h + hue_shift;
    new_h = new_h - floor(new_h);  // wrap
    let new_s = clamp(s * sat_mult, 0.0, 1.0);
    let new_l = clamp(l + lum_offset, 0.0, 1.0);

    // HSL to RGB
    let rgb = hsl_to_rgb(new_h, new_s, new_l);

    dst[base] = rgb.x;
    dst[base + 1] = rgb.y;
    dst[base + 2] = rgb.z;
    if c >= 4 { dst[base + 3] = src[base + 3]; }
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

/// Porter-Duff Over compositing.
pub const COMPOSITE_OVER: &str = r#"
@group(0) @binding(0) var<storage, read> fg: array<f32>;    // foreground RGBA
@group(0) @binding(1) var<storage, read_write> bg: array<f32>; // background RGBA (in-place)
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, 0

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let base = px * c;

    let fg_r = fg[base];
    let fg_g = fg[base + 1];
    let fg_b = fg[base + 2];
    let fg_a = select(1.0, fg[base + 3], c >= 4);

    let bg_r = bg[base];
    let bg_g = bg[base + 1];
    let bg_b = bg[base + 2];
    let bg_a = select(1.0, bg[base + 3], c >= 4);

    // Porter-Duff Over: Fg + Bg * (1 - Fg.a)
    let inv_fg_a = 1.0 - fg_a;
    bg[base] = fg_r * fg_a + bg_r * bg_a * inv_fg_a;
    bg[base + 1] = fg_g * fg_a + bg_g * bg_a * inv_fg_a;
    bg[base + 2] = fg_b * fg_a + bg_b * bg_a * inv_fg_a;
    if c >= 4 {
        bg[base + 3] = fg_a + bg_a * inv_fg_a;
    }
}
"#;

/// Blend modes compositing.
pub const BLEND: &str = r#"
struct BlendParams {
    mode: u32,      // 0=Normal,1=Multiply,2=Screen,3=Add,4=Subtract,5=Overlay,6=SoftLight,7=HardLight,8=Difference
    opacity: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> fg: array<f32>;
@group(0) @binding(1) var<storage, read_write> bg: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;
@group(0) @binding(3) var<uniform> params: BlendParams;

fn blend_channel(a: f32, b: f32, mode: u32) -> f32 {
    switch mode {
        case 0u: { return a; }                                           // Normal
        case 1u: { return a * b; }                                       // Multiply
        case 2u: { return 1.0 - (1.0 - a) * (1.0 - b); }                 // Screen
        case 3u: { return min(a + b, 1.0); }                             // Add
        case 4u: { return max(b - a, 0.0); }                             // Subtract
        case 5u: {                                                        // Overlay
            if b < 0.5 { return 2.0 * a * b; }
            else { return 1.0 - 2.0 * (1.0 - a) * (1.0 - b); }
        }
        case 6u: {                                                        // SoftLight
            if a < 0.5 { return b - (1.0 - 2.0 * a) * b * (1.0 - b); }
            else { return b + (2.0 * a - 1.0) * (sqrt(b) - b); }
        }
        case 7u: {                                                        // HardLight
            if a < 0.5 { return 2.0 * a * b; }
            else { return 1.0 - 2.0 * (1.0 - a) * (1.0 - b); }
        }
        case 8u: { return abs(a - b); }                                  // Difference
        default: { return a; }
    }
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let px = id.x;
    let total = dims.x * dims.y;
    if px >= total { return; }

    let c = dims.z;
    let base = px * c;

    for (var ch = 0u; ch < min(c, 3u); ch = ch + 1) {
        let a = fg[base + ch];
        let b = bg[base + ch];
        let blended = blend_channel(a, b, params.mode);
        bg[base + ch] = b + params.opacity * (blended - b);
    }
}
"#;

/// Crop region from image.
pub const CROP: &str = r#"
struct CropParams {
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    x: u32,
    y: u32,
    c: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> params: CropParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dx = id.x;
    let dy = id.y;
    if dx >= params.dst_w || dy >= params.dst_h { return; }

    let sx = params.x + dx;
    let sy = params.y + dy;
    let src_idx = (sy * params.src_w + sx) * params.c;
    let dst_idx = (dy * params.dst_w + dx) * params.c;

    for (var ch = 0u; ch < params.c; ch = ch + 1) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}
"#;

/// Flip horizontal.
pub const FLIP_H: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, 0

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    let w = dims.x;
    let h = dims.y;
    let c = dims.z;
    if x >= w || y >= h { return; }

    let src_idx = (y * w + (w - 1 - x)) * c;
    let dst_idx = (y * w + x) * c;

    for (var ch = 0u; ch < c; ch = ch + 1) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}
"#;

/// Flip vertical.
pub const FLIP_V: &str = r#"
@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> dims: vec4<u32>;  // w, h, c, 0

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    let w = dims.x;
    let h = dims.y;
    let c = dims.z;
    if x >= w || y >= h { return; }

    let src_idx = ((h - 1 - y) * w + x) * c;
    let dst_idx = (y * w + x) * c;

    for (var ch = 0u; ch < c; ch = ch + 1) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}
"#;

/// Rotate 90 degrees clockwise.
pub const ROTATE_90: &str = r#"
struct RotateParams {
    src_w: u32,
    src_h: u32,
    dst_w: u32,  // = src_h
    dst_h: u32,  // = src_w
    c: u32,
    n: u32,      // rotation count (1=90, 2=180, 3=270)
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst: array<f32>;
@group(0) @binding(2) var<uniform> params: RotateParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dx = id.x;
    let dy = id.y;
    if dx >= params.dst_w || dy >= params.dst_h { return; }

    var sx: u32;
    var sy: u32;
    
    switch params.n {
        case 1u: {  // 90° CW
            sx = dy;
            sy = params.src_h - 1 - dx;
        }
        case 2u: {  // 180°
            sx = params.src_w - 1 - dx;
            sy = params.src_h - 1 - dy;
        }
        case 3u: {  // 270° CW
            sx = params.src_w - 1 - dy;
            sy = dx;
        }
        default: {  // 0° (copy)
            sx = dx;
            sy = dy;
        }
    }

    let src_idx = (sy * params.src_w + sx) * params.c;
    let dst_idx = (dy * params.dst_w + dx) * params.c;

    for (var ch = 0u; ch < params.c; ch = ch + 1) {
        dst[dst_idx + ch] = src[src_idx + ch];
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
