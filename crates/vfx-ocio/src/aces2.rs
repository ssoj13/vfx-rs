//! ACES 2.0 Output Transform implementation.
//!
//! Ported from OpenColorIO's ACES2/Transform.cpp.
//! Implements CAM16-based JMh perceptual color space, tonescale compression,
//! chroma compression, and gamut compression.

use std::f32::consts::PI;

// --- Types ---

pub type F3 = [f32; 3];
pub type M33 = [f32; 9];

/// Chromaticity coordinate pair.
#[derive(Debug, Clone, Copy)]
pub struct Chromaticity {
    pub x: f32,
    pub y: f32,
}

/// Set of RGB primaries + white point.
#[derive(Debug, Clone, Copy)]
pub struct Primaries {
    pub red: Chromaticity,
    pub green: Chromaticity,
    pub blue: Chromaticity,
    pub white: Chromaticity,
}

// --- Well-known primaries ---

pub const ACES_AP0: Primaries = Primaries {
    red: Chromaticity { x: 0.7347, y: 0.2653 },
    green: Chromaticity { x: 0.0, y: 1.0 },
    blue: Chromaticity { x: 0.0001, y: -0.077 },
    white: Chromaticity { x: 0.32168, y: 0.33767 },
};

pub const ACES_AP1: Primaries = Primaries {
    red: Chromaticity { x: 0.713, y: 0.293 },
    green: Chromaticity { x: 0.165, y: 0.83 },
    blue: Chromaticity { x: 0.128, y: 0.044 },
    white: Chromaticity { x: 0.32168, y: 0.33767 },
};

const CAM16_PRIMARIES: Primaries = Primaries {
    red: Chromaticity { x: 0.8336, y: 0.1735 },
    green: Chromaticity { x: 2.3854, y: -1.4659 },
    blue: Chromaticity { x: 0.087, y: -0.125 },
    white: Chromaticity { x: 0.333, y: 0.333 },
};

// --- Constants ---

const HUE_LIMIT: f32 = 360.0;
const REFERENCE_LUMINANCE: f32 = 100.0;
const L_A: f32 = 100.0;
const Y_B: f32 = 20.0;
const SURROUND: F3 = [0.9, 0.59, 0.9]; // Dim surround

const J_SCALE: f32 = 100.0;
const CAM_NL_Y_REF: f32 = 100.0;
const CAM_NL_OFFSET: f32 = 0.2713 * CAM_NL_Y_REF;
const CAM_NL_SCALE: f32 = 4.0 * CAM_NL_Y_REF;

// Chroma compression
const CHROMA_COMPRESS: f32 = 2.4;
const CHROMA_COMPRESS_FACT: f32 = 3.3;
const CHROMA_EXPAND: f32 = 1.3;
const CHROMA_EXPAND_FACT: f32 = 0.69;
const CHROMA_EXPAND_THR: f32 = 0.5;

// Gamut compression
const SMOOTH_CUSPS: f32 = 0.12;
const SMOOTH_M: f32 = 0.27;
const CUSP_MID_BLEND: f32 = 1.3;
const FOCUS_GAIN_BLEND: f32 = 0.3;
#[allow(dead_code)]
const FOCUS_ADJUST_GAIN_INV: f32 = 1.0 / 0.55;
const FOCUS_DISTANCE: f32 = 1.35;
const FOCUS_DISTANCE_SCALING: f32 = 1.75;
const COMPRESSION_THRESHOLD: f32 = 0.75;

// Table generation
const GAMMA_MINIMUM: f32 = 0.0;
const GAMMA_MAXIMUM: f32 = 5.0;
const GAMMA_SEARCH_STEP: f32 = 0.4;
const GAMMA_ACCURACY: f32 = 1e-5;

const CUSP_CORNER_COUNT: usize = 6;
const TOTAL_CORNER_COUNT: usize = CUSP_CORNER_COUNT + 2;
const MAX_SORTED_CORNERS: usize = 2 * CUSP_CORNER_COUNT;
const REACH_CUSP_TOLERANCE: f32 = 1e-3;
const DISPLAY_CUSP_TOLERANCE: f32 = 1e-7;

const GAMMA_TEST_COUNT: usize = 5;

// --- Table types ---

const TABLE_NOMINAL_SIZE: usize = 360;
const TABLE_TOTAL_SIZE: usize = TABLE_NOMINAL_SIZE + 2;
const TABLE_BASE_INDEX: usize = 1;
const TABLE_LOWER_WRAP: usize = 0;
const TABLE_UPPER_WRAP: usize = TABLE_BASE_INDEX + TABLE_NOMINAL_SIZE;
const TABLE_FIRST_NOM: usize = TABLE_BASE_INDEX;
const TABLE_LAST_NOM: usize = TABLE_UPPER_WRAP - 1;

#[derive(Debug, Clone)]
pub struct Table1D {
    pub data: [f32; TABLE_TOTAL_SIZE],
}

impl Default for Table1D {
    fn default() -> Self {
        Self { data: [0.0; TABLE_TOTAL_SIZE] }
    }
}

impl std::ops::Index<usize> for Table1D {
    type Output = f32;
    fn index(&self, i: usize) -> &f32 { &self.data[i] }
}
impl std::ops::IndexMut<usize> for Table1D {
    fn index_mut(&mut self, i: usize) -> &mut f32 { &mut self.data[i] }
}

impl Table1D {
    fn base_hue_for_pos(&self, i: usize) -> f32 { i as f32 }
    fn hue_pos_in_uniform(&self, hue: f32) -> usize { hue as usize }
    fn nominal_hue_pos(&self, hue: f32) -> usize { TABLE_FIRST_NOM + self.hue_pos_in_uniform(hue) }
}

#[derive(Debug, Clone)]
pub struct Table3D {
    pub data: [[f32; 3]; TABLE_TOTAL_SIZE],
}

impl Default for Table3D {
    fn default() -> Self {
        Self { data: [[0.0; 3]; TABLE_TOTAL_SIZE] }
    }
}

impl std::ops::Index<usize> for Table3D {
    type Output = [f32; 3];
    fn index(&self, i: usize) -> &[f32; 3] { &self.data[i] }
}
impl std::ops::IndexMut<usize> for Table3D {
    fn index_mut(&mut self, i: usize) -> &mut [f32; 3] { &mut self.data[i] }
}

// --- Parameter structs ---

#[derive(Debug, Clone)]
pub struct JMhParams {
    pub mat_rgb_to_cam16_c: M33,
    pub mat_cam16_c_to_rgb: M33,
    pub mat_cone_to_aab: M33,
    pub mat_aab_to_cone: M33,
    pub f_l_n: f32,
    pub cz: f32,
    pub inv_cz: f32,
    pub a_w_j: f32,
    pub inv_a_w_j: f32,
}

#[derive(Debug, Clone)]
pub struct ToneScaleParams {
    pub n: f32,
    pub n_r: f32,
    pub g: f32,
    pub t_1: f32,
    pub c_t: f32,
    pub s_2: f32,
    pub u_2: f32,
    pub m_2: f32,
    pub forward_limit: f32,
    pub inverse_limit: f32,
    pub log_peak: f32,
}

#[derive(Debug, Clone)]
pub struct SharedCompressionParams {
    pub limit_j_max: f32,
    pub model_gamma_inv: f32,
    pub reach_m_table: Table1D,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedSharedParams {
    pub limit_j_max: f32,
    pub model_gamma_inv: f32,
    pub reach_max_m: f32,
}

#[derive(Debug, Clone)]
pub struct ChromaCompressParams {
    pub sat: f32,
    pub sat_thr: f32,
    pub compr: f32,
    pub chroma_compress_scale: f32,
}

#[derive(Debug, Clone)]
pub struct HueDependantGamutParams {
    pub gamma_bottom_inv: f32,
    pub jm_cusp: [f32; 2],
    pub gamma_top_inv: f32,
    pub focus_j: f32,
    pub analytical_threshold: f32,
}

#[derive(Debug, Clone)]
pub struct GamutCompressParams {
    pub mid_j: f32,
    pub focus_dist: f32,
    pub lower_hull_gamma_inv: f32,
    pub hue_linearity_search_range: [i32; 2],
    pub hue_table: Table1D,
    pub gamut_cusp_table: Table3D,
}

/// Full pre-computed ACES 2.0 Output Transform state.
#[derive(Debug, Clone)]
pub struct Aces2State {
    pub p_in: JMhParams,
    pub p_out: JMhParams,
    pub ts: ToneScaleParams,
    pub shared: SharedCompressionParams,
    pub chroma: ChromaCompressParams,
    pub gamut: GamutCompressParams,
}

// --- Matrix math ---

fn mult_f3_m33(v: &F3, m: &M33) -> F3 {
    [
        v[0] * m[0] + v[1] * m[1] + v[2] * m[2],
        v[0] * m[3] + v[1] * m[4] + v[2] * m[5],
        v[0] * m[6] + v[1] * m[7] + v[2] * m[8],
    ]
}

fn mult_m33_m33(a: &M33, b: &M33) -> M33 {
    [
        a[0]*b[0]+a[1]*b[3]+a[2]*b[6], a[0]*b[1]+a[1]*b[4]+a[2]*b[7], a[0]*b[2]+a[1]*b[5]+a[2]*b[8],
        a[3]*b[0]+a[4]*b[3]+a[5]*b[6], a[3]*b[1]+a[4]*b[4]+a[5]*b[7], a[3]*b[2]+a[4]*b[5]+a[5]*b[8],
        a[6]*b[0]+a[7]*b[3]+a[8]*b[6], a[6]*b[1]+a[7]*b[4]+a[8]*b[7], a[6]*b[2]+a[7]*b[5]+a[8]*b[8],
    ]
}

fn scale_m33(m: &M33, s: &F3) -> M33 {
    [
        m[0]*s[0], m[3],      m[6],
        m[1],      m[4]*s[1], m[7],
        m[2],      m[5],      m[8]*s[2],
    ]
}

fn invert_m33(m: &M33) -> M33 {
    let det = m[0]*(m[4]*m[8]-m[5]*m[7]) - m[1]*(m[3]*m[8]-m[5]*m[6]) + m[2]*(m[3]*m[7]-m[4]*m[6]);
    if det.abs() < 1e-30 { return *m; }
    let inv_det = 1.0 / det;
    [
        (m[4]*m[8]-m[5]*m[7])*inv_det, (m[2]*m[7]-m[1]*m[8])*inv_det, (m[1]*m[5]-m[2]*m[4])*inv_det,
        (m[5]*m[6]-m[3]*m[8])*inv_det, (m[0]*m[8]-m[2]*m[6])*inv_det, (m[2]*m[3]-m[0]*m[5])*inv_det,
        (m[3]*m[7]-m[4]*m[6])*inv_det, (m[1]*m[6]-m[0]*m[7])*inv_det, (m[0]*m[4]-m[1]*m[3])*inv_det,
    ]
}

const IDENTITY_M33: M33 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

// --- Color matrix helpers ---

/// Build RGB-to-XYZ matrix for given primaries (CIE XYZ Illuminant E).
fn rgb_to_xyz_m33(p: &Primaries) -> M33 {
    // XYZ from xy chromaticities (Illuminant E, Y=1)
    let rx = p.red.x; let ry = p.red.y;
    let gx = p.green.x; let gy = p.green.y;
    let bx = p.blue.x; let by = p.blue.y;
    let wx = p.white.x; let wy = p.white.y;

    // XYZ of primaries (z = 1 - x - y, normalized so Y=1)
    let rz = 1.0 - rx - ry;
    let gz = 1.0 - gx - gy;
    let bz = 1.0 - bx - by;

    // XYZ of whitepoint
    let wz = 1.0 - wx - wy;
    let w_xyz = [wx / wy, 1.0, wz / wy];

    // Build matrix of primaries (each column is XYZ/Y of a primary)
    let m = [
        rx / ry, gx / gy, bx / by,
        1.0,     1.0,     1.0,
        rz / ry, gz / gy, bz / by,
    ];
    let m_inv = invert_m33(&m);
    let s = mult_f3_m33(&w_xyz, &m_inv);

    [
        s[0] * m[0], s[1] * m[1], s[2] * m[2],
        s[0] * m[3], s[1] * m[4], s[2] * m[5],
        s[0] * m[6], s[1] * m[7], s[2] * m[8],
    ]
}

fn xyz_to_rgb_m33(p: &Primaries) -> M33 {
    invert_m33(&rgb_to_xyz_m33(p))
}

fn rgb_to_rgb_m33(src: &Primaries, dst: &Primaries) -> M33 {
    mult_m33_m33(&xyz_to_rgb_m33(dst), &rgb_to_xyz_m33(src))
}

// --- Hue utilities ---

fn wrap_to_hue_limit(hue: f32) -> f32 {
    let y = hue % HUE_LIMIT;
    if y < 0.0 { y + HUE_LIMIT } else { y }
}

fn to_radians(v: f32) -> f32 { PI * v / 180.0 }
fn from_radians_wrapped(v: f32) -> f32 {
    let y = 180.0 * v / PI;
    if y < 0.0 { y + HUE_LIMIT } else { y }
}

fn lerpf(a: f32, b: f32, t: f32) -> f32 { a + t * (b - a) }
fn lerp_f3(a: &F3, b: &F3, t: f32) -> F3 {
    [lerpf(a[0], b[0], t), lerpf(a[1], b[1], t), lerpf(a[2], b[2], t)]
}

fn midpoint_f(a: f32, b: f32) -> f32 { (a + b) * 0.5 }

// --- Table lookups ---

fn lookup_hue_interval(h: f32, hues: &Table1D, search_range: &[i32; 2]) -> usize {
    let i_start = hues.nominal_hue_pos(h);
    let mut i_lo = (i_start as i32 + search_range[0]).max(TABLE_LOWER_WRAP as i32) as usize;
    let mut i_hi = (i_start as i32 + search_range[1]).min(TABLE_UPPER_WRAP as i32) as usize;
    let mut i = i_start;

    while i_lo + 1 < i_hi {
        if h > hues[i] {
            i_lo = i;
        } else {
            i_hi = i;
        }
        i = (i_lo + i_hi) / 2;
    }
    i_hi.max(1)
}

fn interp_weight(h: f32, h_lo: f32, h_hi: f32) -> f32 {
    (h - h_lo) / (h_hi - h_lo)
}

fn cusp_from_table(i_hi: usize, t: f32, gt: &Table3D) -> F3 {
    lerp_f3(&gt[i_hi - 1], &gt[i_hi], t)
}

fn reach_m_from_table(h: f32, rt: &Table1D) -> f32 {
    let base = rt.hue_pos_in_uniform(h);
    let t = h - base as f32;
    let i_lo = base + TABLE_FIRST_NOM;
    let i_hi = i_lo + 1;
    lerpf(rt[i_lo], rt[i_hi], t)
}

// --- CAM16 core ---

fn post_adapt_compress_fwd_inner(rc: f32) -> f32 {
    let f_l_y = rc.powf(0.42);
    f_l_y / (CAM_NL_OFFSET + f_l_y)
}

fn post_adapt_compress_inv_inner(ra: f32) -> f32 {
    let ra_lim = ra.min(0.99);
    let f_l_y = CAM_NL_OFFSET * ra_lim / (1.0 - ra_lim);
    f_l_y.powf(1.0 / 0.42)
}

fn post_adapt_compress_fwd(v: f32) -> f32 {
    let ra = post_adapt_compress_fwd_inner(v.abs());
    ra.copysign(v)
}

fn post_adapt_compress_inv(v: f32) -> f32 {
    let rc = post_adapt_compress_inv_inner(v.abs());
    rc.copysign(v)
}

fn achromatic_n_to_j(a: f32, cz: f32) -> f32 { J_SCALE * a.powf(cz) }
fn j_to_achromatic_n(j: f32, inv_cz: f32) -> f32 { (j / J_SCALE).powf(inv_cz) }

fn a_to_y(a: f32, p: &JMhParams) -> f32 {
    let ra = p.a_w_j * a;
    post_adapt_compress_inv_inner(ra) / p.f_l_n
}

fn j_to_y(j: f32, p: &JMhParams) -> f32 {
    a_to_y(j_to_achromatic_n(j, p.inv_cz), p)
}

fn y_to_j_inner(y: f32, p: &JMhParams) -> f32 {
    let ra = post_adapt_compress_fwd_inner(y * p.f_l_n);
    achromatic_n_to_j(ra * p.inv_a_w_j, p.cz)
}

pub fn y_to_j(y: f32, p: &JMhParams) -> f32 {
    let j = y_to_j_inner(y.abs(), p);
    j.copysign(y)
}

pub fn rgb_to_aab(rgb: &F3, p: &JMhParams) -> F3 {
    let rgb_m = mult_f3_m33(rgb, &p.mat_rgb_to_cam16_c);
    let rgb_a = [
        post_adapt_compress_fwd(rgb_m[0]),
        post_adapt_compress_fwd(rgb_m[1]),
        post_adapt_compress_fwd(rgb_m[2]),
    ];
    mult_f3_m33(&rgb_a, &p.mat_cone_to_aab)
}

pub fn aab_to_jmh(aab: &F3, p: &JMhParams) -> F3 {
    if aab[0] <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let j = achromatic_n_to_j(aab[0], p.cz);
    let m = (aab[1] * aab[1] + aab[2] * aab[2]).sqrt();
    let h = from_radians_wrapped(aab[2].atan2(aab[1]));
    [j, m, h]
}

pub fn rgb_to_jmh(rgb: &F3, p: &JMhParams) -> F3 {
    aab_to_jmh(&rgb_to_aab(rgb, p), p)
}

pub fn jmh_to_aab_trig(jmh: &F3, cos_hr: f32, sin_hr: f32, p: &JMhParams) -> F3 {
    let a_n = j_to_achromatic_n(jmh[0], p.inv_cz);
    [a_n, jmh[1] * cos_hr, jmh[1] * sin_hr]
}

pub fn jmh_to_aab(jmh: &F3, p: &JMhParams) -> F3 {
    let h_rad = to_radians(jmh[2]);
    jmh_to_aab_trig(jmh, h_rad.cos(), h_rad.sin(), p)
}

pub fn aab_to_rgb(aab: &F3, p: &JMhParams) -> F3 {
    let rgb_a = mult_f3_m33(aab, &p.mat_aab_to_cone);
    let rgb_m = [
        post_adapt_compress_inv(rgb_a[0]),
        post_adapt_compress_inv(rgb_a[1]),
        post_adapt_compress_inv(rgb_a[2]),
    ];
    mult_f3_m33(&rgb_m, &p.mat_cam16_c_to_rgb)
}

pub fn jmh_to_rgb(jmh: &F3, p: &JMhParams) -> F3 {
    aab_to_rgb(&jmh_to_aab(jmh, p), p)
}

// --- Tonescale ---

fn aces_tonescale_fwd(y_in: f32, pt: &ToneScaleParams) -> f32 {
    let f = pt.m_2 * (y_in / (y_in + pt.s_2)).powf(pt.g);
    (f * f / (f + pt.t_1)).max(0.0) * pt.n_r
}

fn aces_tonescale_inv(y_in: f32, pt: &ToneScaleParams) -> f32 {
    let y_ts_norm = y_in / REFERENCE_LUMINANCE;
    let z = y_ts_norm.max(0.0).min(pt.inverse_limit);
    let f = (z + (z * (4.0 * pt.t_1 + z)).sqrt()) * 0.5;
    pt.s_2 / ((pt.m_2 / f).powf(1.0 / pt.g) - 1.0)
}

fn tonescale_fwd_j(j: f32, p: &JMhParams, pt: &ToneScaleParams) -> f32 {
    let j_abs = j.abs();
    let y_in = j_to_y(j_abs, p);
    let y_out = aces_tonescale_fwd(y_in, pt);
    let j_out = y_to_j_inner(y_out, p);
    j_out.copysign(j)
}

fn tonescale_inv_j(j: f32, p: &JMhParams, pt: &ToneScaleParams) -> f32 {
    let j_abs = j.abs();
    let y_in = j_to_y(j_abs, p);
    let y_out = aces_tonescale_inv(y_in, pt);
    let j_out = y_to_j_inner(y_out, p);
    j_out.copysign(j)
}

fn tonescale_a_to_j_fwd(a: f32, p: &JMhParams, pt: &ToneScaleParams) -> f32 {
    let y_in = a_to_y(a, p);
    let y_out = aces_tonescale_fwd(y_in, pt);
    let j_out = y_to_j_inner(y_out, p);
    j_out.copysign(a)
}

// --- Chroma compress ---

pub fn chroma_compress_norm(cos_hr: f32, sin_hr: f32, scale: f32) -> f32 {
    let cos2 = 2.0 * cos_hr * cos_hr - 1.0;
    let sin2 = 2.0 * cos_hr * sin_hr;
    let cos3 = 4.0 * cos_hr * cos_hr * cos_hr - 3.0 * cos_hr;
    let sin3 = 3.0 * sin_hr - 4.0 * sin_hr * sin_hr * sin_hr;

    let m = 11.34072 * cos_hr + 16.46899 * cos2 + 7.88380 * cos3
          + 14.66441 * sin_hr + (-6.37224) * sin2 + 9.19364 * sin3
          + 77.12896;
    m * scale
}

fn toe_fwd(x: f32, limit: f32, k1_in: f32, k2_in: f32) -> f32 {
    if x > limit { return x; }
    let k2 = k2_in.max(0.001);
    let k1 = (k1_in * k1_in + k2 * k2).sqrt();
    let k3 = (limit + k1) / (limit + k2);
    let mb = k3 * x - k1;
    let mac = k2 * k3 * x;
    0.5 * (mb + (mb * mb + 4.0 * mac).sqrt())
}

fn toe_inv(x: f32, limit: f32, k1_in: f32, k2_in: f32) -> f32 {
    if x > limit { return x; }
    let k2 = k2_in.max(0.001);
    let k1 = (k1_in * k1_in + k2 * k2).sqrt();
    let k3 = (limit + k1) / (limit + k2);
    (x * x + k1 * x) / (k3 * (x + k2))
}

pub fn chroma_compress_fwd(jmh: &F3, j_ts: f32, mnorm: f32, pr: &ResolvedSharedParams, pc: &ChromaCompressParams) -> F3 {
    let (j, m, h) = (jmh[0], jmh[1], jmh[2]);
    let mut m_cp = m;

    if m != 0.0 {
        let nj = j_ts / pr.limit_j_max;
        let snj = (1.0 - nj).max(0.0);
        let limit = nj.powf(pr.model_gamma_inv) * pr.reach_max_m / mnorm;

        m_cp = m * (j_ts / j).powf(pr.model_gamma_inv);
        m_cp /= mnorm;
        m_cp = limit - toe_fwd(limit - m_cp, limit - 0.001, snj * pc.sat, (nj * nj + pc.sat_thr).sqrt());
        m_cp = toe_fwd(m_cp, limit, nj * pc.compr, snj);
        m_cp *= mnorm;
    }
    [j_ts, m_cp, h]
}

pub fn chroma_compress_inv(jmh: &F3, j: f32, mnorm: f32, pr: &ResolvedSharedParams, pc: &ChromaCompressParams) -> F3 {
    let (j_ts, m_cp, h) = (jmh[0], jmh[1], jmh[2]);
    let mut m = m_cp;

    if m_cp != 0.0 {
        let nj = j_ts / pr.limit_j_max;
        let snj = (1.0 - nj).max(0.0);
        let limit = nj.powf(pr.model_gamma_inv) * pr.reach_max_m / mnorm;

        m /= mnorm;
        m = toe_inv(m, limit, nj * pc.compr, snj);
        m = limit - toe_inv(limit - m, limit - 0.001, snj * pc.sat, (nj * nj + pc.sat_thr).sqrt());
        m *= mnorm;
        m *= (j_ts / j).powf(-pr.model_gamma_inv);
    }
    [j, m, h]
}

// --- Gamut compress ---

fn get_focus_gain(j: f32, threshold: f32, limit_j_max: f32, focus_dist: f32) -> f32 {
    let mut gain = limit_j_max * focus_dist;
    if j > threshold {
        let adj = ((limit_j_max - threshold) / (limit_j_max - j).max(0.0001)).log10();
        let adj = adj * adj + 1.0;
        gain *= adj;
    }
    gain
}

fn solve_j_intersect(j: f32, m: f32, focus_j: f32, max_j: f32, slope_gain: f32) -> f32 {
    let m_scaled = m / slope_gain;
    let a = m_scaled / focus_j;

    if j < focus_j {
        let b = 1.0 - m_scaled;
        let c = -j;
        let det = b * b - 4.0 * a * c;
        -2.0 * c / (b + det.sqrt())
    } else {
        let b = -(1.0 + m_scaled + max_j * a);
        let c = max_j * m_scaled + j;
        let det = b * b - 4.0 * a * c;
        -2.0 * c / (b - det.sqrt())
    }
}

fn smin_scaled(a: f32, b: f32, scale_ref: f32) -> f32 {
    let s_scaled = SMOOTH_CUSPS * scale_ref;
    let h = (s_scaled - (a - b).abs()).max(0.0) / s_scaled;
    a.min(b) - h * h * h * s_scaled * (1.0 / 6.0)
}

fn compute_compression_slope(intersect_j: f32, focus_j: f32, limit_j_max: f32, slope_gain: f32) -> f32 {
    let dir = if intersect_j < focus_j { intersect_j } else { limit_j_max - intersect_j };
    dir * (intersect_j - focus_j) / (focus_j * slope_gain)
}

fn estimate_line_boundary_m(j_intersect: f32, slope: f32, inv_gamma: f32, j_max: f32, m_max: f32, j_ref: f32) -> f32 {
    let nj = j_intersect / j_ref;
    let shifted = j_ref * nj.powf(inv_gamma);
    shifted * m_max / (j_max - slope * m_max)
}

fn find_gamut_boundary_m(jm_cusp: &[f32; 2], j_max: f32, gamma_top_inv: f32, gamma_bottom_inv: f32,
                          j_intersect_src: f32, slope: f32, j_intersect_cusp: f32) -> f32 {
    let m_lower = estimate_line_boundary_m(j_intersect_src, slope, gamma_bottom_inv, jm_cusp[0], jm_cusp[1], j_intersect_cusp);
    let f_j_cusp = j_max - j_intersect_cusp;
    let f_j_src = j_max - j_intersect_src;
    let f_jm_j = j_max - jm_cusp[0];
    let m_upper = estimate_line_boundary_m(f_j_src, -slope, gamma_top_inv, f_jm_j, jm_cusp[1], f_j_cusp);
    smin_scaled(m_lower, m_upper, jm_cusp[1])
}

fn reinhard_fwd(scale: f32, nd: f32) -> f32 { scale * nd / (1.0 + nd) }
fn reinhard_inv(scale: f32, nd: f32) -> f32 {
    if nd >= 1.0 { scale } else { scale * -(nd / (nd - 1.0)) }
}

fn remap_m_fwd(m: f32, gamut_m: f32, reach_m: f32) -> f32 {
    let ratio = gamut_m / reach_m;
    let proportion = ratio.max(COMPRESSION_THRESHOLD);
    let threshold = proportion * gamut_m;
    if m <= threshold || proportion >= 1.0 { return m; }
    let m_off = m - threshold;
    let gamut_off = gamut_m - threshold;
    let reach_off = reach_m - threshold;
    let scale = reach_off / (reach_off / gamut_off - 1.0);
    threshold + reinhard_fwd(scale, m_off / scale)
}

fn remap_m_inv(m: f32, gamut_m: f32, reach_m: f32) -> f32 {
    let ratio = gamut_m / reach_m;
    let proportion = ratio.max(COMPRESSION_THRESHOLD);
    let threshold = proportion * gamut_m;
    if m <= threshold || proportion >= 1.0 { return m; }
    let m_off = m - threshold;
    let gamut_off = gamut_m - threshold;
    let reach_off = reach_m - threshold;
    let scale = reach_off / (reach_off / gamut_off - 1.0);
    threshold + reinhard_inv(scale, m_off / scale)
}

fn compress_gamut_core<const INV: bool>(jmh: &F3, jx: f32, sr: &ResolvedSharedParams, p: &GamutCompressParams, hdp: &HueDependantGamutParams) -> F3 {
    let (j, m, h) = (jmh[0], jmh[1], jmh[2]);

    let slope_gain = get_focus_gain(jx, hdp.analytical_threshold, sr.limit_j_max, p.focus_dist);
    let j_intersect_src = solve_j_intersect(j, m, hdp.focus_j, sr.limit_j_max, slope_gain);
    let gamut_slope = compute_compression_slope(j_intersect_src, hdp.focus_j, sr.limit_j_max, slope_gain);
    let j_intersect_cusp = solve_j_intersect(hdp.jm_cusp[0], hdp.jm_cusp[1], hdp.focus_j, sr.limit_j_max, slope_gain);
    let gamut_m = find_gamut_boundary_m(&hdp.jm_cusp, sr.limit_j_max, hdp.gamma_top_inv, hdp.gamma_bottom_inv, j_intersect_src, gamut_slope, j_intersect_cusp);

    if gamut_m <= 0.0 {
        return [j, 0.0, h];
    }

    let reach_m = estimate_line_boundary_m(j_intersect_src, gamut_slope, sr.model_gamma_inv, sr.limit_j_max, sr.reach_max_m, sr.limit_j_max);
    let remapped = if INV { remap_m_inv(m, gamut_m, reach_m) } else { remap_m_fwd(m, gamut_m, reach_m) };

    [j_intersect_src + remapped * gamut_slope, remapped, h]
}

fn compute_focus_j(cusp_j: f32, mid_j: f32, limit_j_max: f32) -> f32 {
    lerpf(cusp_j, mid_j, (CUSP_MID_BLEND - cusp_j / limit_j_max).min(1.0))
}

fn init_hue_gamut_params(hue: f32, sr: &ResolvedSharedParams, p: &GamutCompressParams) -> HueDependantGamutParams {
    let i_hi = lookup_hue_interval(hue, &p.hue_table, &p.hue_linearity_search_range);
    let t = interp_weight(hue, p.hue_table[i_hi - 1], p.hue_table[i_hi]);
    let cusp = cusp_from_table(i_hi, t, &p.gamut_cusp_table);

    let jm_cusp = [cusp[0], cusp[1]];
    let gamma_top_inv = cusp[2];
    let focus_j = compute_focus_j(jm_cusp[0], p.mid_j, sr.limit_j_max);
    let threshold = lerpf(jm_cusp[0], sr.limit_j_max, FOCUS_GAIN_BLEND);

    HueDependantGamutParams {
        gamma_bottom_inv: p.lower_hull_gamma_inv,
        jm_cusp,
        gamma_top_inv,
        focus_j,
        analytical_threshold: threshold,
    }
}

pub fn gamut_compress_fwd(jmh: &F3, sr: &ResolvedSharedParams, p: &GamutCompressParams) -> F3 {
    let (j, m, h) = (jmh[0], jmh[1], jmh[2]);
    if j <= 0.0 { return [0.0, 0.0, h]; }
    if m <= 0.0 || j > sr.limit_j_max { return [j, 0.0, h]; }
    let hdp = init_hue_gamut_params(h, sr, p);
    compress_gamut_core::<false>(jmh, j, sr, p, &hdp)
}

pub fn gamut_compress_inv(jmh: &F3, sr: &ResolvedSharedParams, p: &GamutCompressParams) -> F3 {
    let (j, m, h) = (jmh[0], jmh[1], jmh[2]);
    if j <= 0.0 { return [0.0, 0.0, h]; }
    if m <= 0.0 || j > sr.limit_j_max { return [j, 0.0, h]; }
    let hdp = init_hue_gamut_params(h, sr, p);
    let mut jx = j;
    if jx > hdp.analytical_threshold {
        jx = compress_gamut_core::<true>(jmh, jx, sr, p, &hdp)[0];
    }
    compress_gamut_core::<true>(jmh, jx, sr, p, &hdp)
}

// --- Initialization ---

fn model_gamma() -> f32 {
    SURROUND[1] * (1.48 + (Y_B / REFERENCE_LUMINANCE).sqrt())
}

pub fn init_jmh_params(prims: &Primaries) -> JMhParams {
    let base_cone_to_aab: M33 = [
        2.0, 1.0, 1.0 / 20.0,
        1.0, -12.0 / 11.0, 1.0 / 11.0,
        1.0 / 9.0, 1.0 / 9.0, -2.0 / 9.0,
    ];

    let mat16 = xyz_to_rgb_m33(&CAM16_PRIMARIES);
    let rgb_to_xyz = rgb_to_xyz_m33(prims);
    let xyz_w = mult_f3_m33(&[REFERENCE_LUMINANCE; 3], &rgb_to_xyz);
    let y_w = xyz_w[1];
    let rgb_w = mult_f3_m33(&xyz_w, &mat16);

    let k = 1.0 / (5.0 * L_A + 1.0);
    let k4 = k * k * k * k;
    let f_l = 0.2 * k4 * (5.0 * L_A) + 0.1 * (1.0 - k4).powi(2) * (5.0 * L_A).powf(1.0 / 3.0);
    let f_l_n = f_l / REFERENCE_LUMINANCE;
    let cz = model_gamma();
    let inv_cz = 1.0 / cz;

    let d_rgb = [
        f_l_n * y_w / rgb_w[0],
        f_l_n * y_w / rgb_w[1],
        f_l_n * y_w / rgb_w[2],
    ];

    let rgb_wc = [d_rgb[0] * rgb_w[0], d_rgb[1] * rgb_w[1], d_rgb[2] * rgb_w[2]];
    let rgb_aw = [
        post_adapt_compress_fwd(rgb_wc[0]),
        post_adapt_compress_fwd(rgb_wc[1]),
        post_adapt_compress_fwd(rgb_wc[2]),
    ];

    let cam_nl_s = [CAM_NL_SCALE; 3]; // actually this is the scaling used in OCIO
    // The OCIO code: scale_f33(Identity_M33, f3_from_f(cam_nl_scale))
    // which scales rows by cam_nl_scale
    let cone_to_aab = mult_m33_m33(&scale_m33(&IDENTITY_M33, &cam_nl_s), &base_cone_to_aab);

    let a_w = cone_to_aab[0] * rgb_aw[0] + cone_to_aab[1] * rgb_aw[1] + cone_to_aab[2] * rgb_aw[2];
    let a_w_j = post_adapt_compress_fwd_inner(f_l);
    let inv_a_w_j = 1.0 / a_w_j;

    let mat_rgb_to_cam16 = mult_m33_m33(&rgb_to_rgb_m33(prims, &CAM16_PRIMARIES), &scale_m33(&IDENTITY_M33, &[REFERENCE_LUMINANCE; 3]));
    let mat_rgb_to_cam16_c = mult_m33_m33(&scale_m33(&IDENTITY_M33, &d_rgb), &mat_rgb_to_cam16);
    let mat_cam16_c_to_rgb = invert_m33(&mat_rgb_to_cam16_c);

    let mat_cone_to_aab = [
        cone_to_aab[0] / a_w,                             cone_to_aab[1] / a_w,                             cone_to_aab[2] / a_w,
        cone_to_aab[3] * 43.0 * SURROUND[2], cone_to_aab[4] * 43.0 * SURROUND[2], cone_to_aab[5] * 43.0 * SURROUND[2],
        cone_to_aab[6] * 43.0 * SURROUND[2], cone_to_aab[7] * 43.0 * SURROUND[2], cone_to_aab[8] * 43.0 * SURROUND[2],
    ];
    let mat_aab_to_cone = invert_m33(&mat_cone_to_aab);

    JMhParams {
        mat_rgb_to_cam16_c,
        mat_cam16_c_to_rgb,
        mat_cone_to_aab: mat_cone_to_aab,
        mat_aab_to_cone: mat_aab_to_cone,
        f_l_n,
        cz,
        inv_cz,
        a_w_j,
        inv_a_w_j,
    }
}

pub fn init_tonescale_params(peak_lum: f32) -> ToneScaleParams {
    let n = peak_lum;
    let n_r = 100.0;
    let g = 1.15;
    let c = 0.18;
    let c_d = 10.013;
    let w_g = 0.14;
    let t_1 = 0.04;
    let r_hit_min = 128.0;
    let r_hit_max = 896.0;

    let r_hit = r_hit_min + (r_hit_max - r_hit_min) * ((n / n_r).ln() / (10000.0_f32 / 100.0).ln());
    let m_0 = n / n_r;
    let m_1 = 0.5 * (m_0 + (m_0 * (m_0 + 4.0 * t_1)).sqrt());
    let u = ((r_hit / m_1) / (r_hit / m_1 + 1.0)).powf(g);
    let _m = m_1 / u;
    let w_i = (n / 100.0).log2();
    let c_t = c_d / n_r * (1.0 + w_i * w_g);
    let g_ip = 0.5 * (c_t + (c_t * (c_t + 4.0 * t_1)).sqrt());
    let g_ipp2 = -(m_1 * (g_ip / _m).powf(1.0 / g)) / ((g_ip / _m).powf(1.0 / g) - 1.0);
    let w_2 = c / g_ipp2;
    let s_2 = w_2 * m_1 * REFERENCE_LUMINANCE;
    let u_2 = ((r_hit / m_1) / (r_hit / m_1 + w_2)).powf(g);
    let m_2 = m_1 / u_2;
    let inverse_limit = n / (u_2 * n_r);
    let forward_limit = 8.0 * r_hit;
    let log_peak = (n / n_r).log10();

    ToneScaleParams { n, n_r, g, t_1, c_t, s_2, u_2, m_2, forward_limit, inverse_limit, log_peak }
}

pub fn init_shared_compression_params(peak_lum: f32, input_params: &JMhParams, reach_params: &JMhParams) -> SharedCompressionParams {
    let limit_j_max = y_to_j(peak_lum, input_params);
    let model_gamma_inv = 1.0 / model_gamma();
    SharedCompressionParams {
        limit_j_max,
        model_gamma_inv,
        reach_m_table: make_reach_m_table(reach_params, limit_j_max),
    }
}

pub fn resolve_compression_params(hue: f32, p: &SharedCompressionParams) -> ResolvedSharedParams {
    ResolvedSharedParams {
        limit_j_max: p.limit_j_max,
        model_gamma_inv: p.model_gamma_inv,
        reach_max_m: reach_m_from_table(hue, &p.reach_m_table),
    }
}

pub fn init_chroma_compress_params(peak_lum: f32, ts: &ToneScaleParams) -> ChromaCompressParams {
    let compr = CHROMA_COMPRESS + CHROMA_COMPRESS * CHROMA_COMPRESS_FACT * ts.log_peak;
    let sat = (CHROMA_EXPAND - CHROMA_EXPAND * CHROMA_EXPAND_FACT * ts.log_peak).max(0.2);
    let sat_thr = CHROMA_EXPAND_THR / ts.n;
    let ccs = (0.03379 * peak_lum).powf(0.30596) - 0.45135;
    ChromaCompressParams { sat, sat_thr, compr, chroma_compress_scale: ccs }
}

// --- Table generation ---

fn gen_unit_cube_corner(corner: usize) -> F3 {
    [
        if ((corner + 1) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
        if ((corner + 5) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
        if ((corner + 3) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
    ]
}

fn build_limiting_cusp_corners(params: &JMhParams, peak_lum: f32) -> ([F3; TOTAL_CORNER_COUNT], [F3; TOTAL_CORNER_COUNT]) {
    let mut tmp_rgb = [[0.0f32; 3]; CUSP_CORNER_COUNT];
    let mut tmp_jmh = [[0.0f32; 3]; CUSP_CORNER_COUNT];
    let mut min_idx = 0usize;
    let scale = peak_lum / REFERENCE_LUMINANCE;

    for i in 0..CUSP_CORNER_COUNT {
        let c = gen_unit_cube_corner(i);
        tmp_rgb[i] = [scale * c[0], scale * c[1], scale * c[2]];
        tmp_jmh[i] = rgb_to_jmh(&tmp_rgb[i], params);
        if tmp_jmh[i][2] < tmp_jmh[min_idx][2] { min_idx = i; }
    }

    let mut rgb_corners = [[0.0f32; 3]; TOTAL_CORNER_COUNT];
    let mut jmh_corners = [[0.0f32; 3]; TOTAL_CORNER_COUNT];
    for i in 0..CUSP_CORNER_COUNT {
        rgb_corners[i + 1] = tmp_rgb[(i + min_idx) % CUSP_CORNER_COUNT];
        jmh_corners[i + 1] = tmp_jmh[(i + min_idx) % CUSP_CORNER_COUNT];
    }
    rgb_corners[0] = rgb_corners[CUSP_CORNER_COUNT];
    rgb_corners[CUSP_CORNER_COUNT + 1] = rgb_corners[1];
    jmh_corners[0] = jmh_corners[CUSP_CORNER_COUNT];
    jmh_corners[CUSP_CORNER_COUNT + 1] = jmh_corners[1];
    jmh_corners[0][2] -= HUE_LIMIT;
    jmh_corners[CUSP_CORNER_COUNT + 1][2] += HUE_LIMIT;

    (rgb_corners, jmh_corners)
}

fn find_reach_corners(params: &JMhParams, limit_j: f32, max_src: f32) -> [F3; TOTAL_CORNER_COUNT] {
    let limit_a = j_to_achromatic_n(limit_j, params.inv_cz);
    let mut tmp = [[0.0f32; 3]; CUSP_CORNER_COUNT];
    let mut min_idx = 0usize;

    for i in 0..CUSP_CORNER_COUNT {
        let v = gen_unit_cube_corner(i);
        let mut lower = 0.0f32;
        let mut upper = max_src;
        while upper - lower > REACH_CUSP_TOLERANCE {
            let test = midpoint_f(lower, upper);
            let tc = [test * v[0], test * v[1], test * v[2]];
            let a = rgb_to_aab(&tc, params)[0];
            if a < limit_a { lower = test; } else { upper = test; }
            if a == limit_a { break; }
        }
        let uc = [upper * v[0], upper * v[1], upper * v[2]];
        tmp[i] = rgb_to_jmh(&uc, params);
        if tmp[i][2] < tmp[min_idx][2] { min_idx = i; }
    }

    let mut corners = [[0.0f32; 3]; TOTAL_CORNER_COUNT];
    for i in 0..CUSP_CORNER_COUNT {
        corners[i + 1] = tmp[(i + min_idx) % CUSP_CORNER_COUNT];
    }
    corners[0] = corners[CUSP_CORNER_COUNT];
    corners[CUSP_CORNER_COUNT + 1] = corners[1];
    corners[0][2] -= HUE_LIMIT;
    corners[CUSP_CORNER_COUNT + 1][2] += HUE_LIMIT;
    corners
}

fn extract_sorted_cube_hues(reach: &[F3; TOTAL_CORNER_COUNT], display: &[F3; TOTAL_CORNER_COUNT]) -> (Vec<f32>, usize) {
    let mut sorted = vec![0.0f32; MAX_SORTED_CORNERS];
    let mut idx = 0;
    let mut ri = 1;
    let mut di = 1;

    while ri < CUSP_CORNER_COUNT + 1 || di < CUSP_CORNER_COUNT + 1 {
        let rh = if ri < CUSP_CORNER_COUNT + 1 { reach[ri][2] } else { f32::MAX };
        let dh = if di < CUSP_CORNER_COUNT + 1 { display[di][2] } else { f32::MAX };

        if (rh - dh).abs() < 1e-10 {
            sorted[idx] = rh; ri += 1; di += 1;
        } else if rh < dh {
            sorted[idx] = rh; ri += 1;
        } else {
            sorted[idx] = dh; di += 1;
        }
        idx += 1;
    }
    (sorted, idx)
}

fn build_hue_table(hue_table: &mut Table1D, sorted_hues: &[f32], unique_hues: usize) {
    let ideal_spacing = TABLE_NOMINAL_SIZE as f32 / HUE_LIMIT;
    let mut samples_count = vec![0u32; 2 * CUSP_CORNER_COUNT + 2];
    let mut last_idx = u32::MAX;
    let mut min_index: u32 = if sorted_hues[0] == 0.0 { 0 } else { 1 };

    for hue_idx in 0..unique_hues {
        let mut nominal = ((sorted_hues[hue_idx] * ideal_spacing).round() as u32)
            .max(min_index)
            .min(TABLE_NOMINAL_SIZE as u32 - 1);

        if last_idx == nominal {
            if hue_idx > 1 && samples_count[hue_idx - 2] != samples_count[hue_idx - 1] - 1 {
                samples_count[hue_idx - 1] -= 1;
            } else {
                nominal += 1;
            }
        }
        samples_count[hue_idx] = nominal.min(TABLE_NOMINAL_SIZE as u32 - 1);
        last_idx = nominal;
        min_index = nominal;
    }

    let mut total = 0u32;
    // First interval
    let cnt0 = samples_count[0];
    for s in 0..cnt0 {
        hue_table[total as usize + s as usize + 1] = sorted_hues[0] * s as f32 / cnt0 as f32;
    }
    total += cnt0;

    for i in 1..unique_hues {
        let samples = samples_count[i] - samples_count[i - 1];
        let lower = sorted_hues[i - 1];
        let upper = sorted_hues[i];
        let delta = (upper - lower) / samples as f32;
        for s in 0..samples {
            hue_table[total as usize + s as usize + 1] = lower + s as f32 * delta;
        }
        total += samples;
    }

    // Last interval
    let remaining = TABLE_NOMINAL_SIZE as u32 - total;
    let lower = sorted_hues[unique_hues - 1];
    let delta = (HUE_LIMIT - lower) / remaining as f32;
    for s in 0..remaining {
        hue_table[total as usize + s as usize + 1] = lower + s as f32 * delta;
    }

    hue_table[TABLE_LOWER_WRAP] = hue_table[TABLE_LAST_NOM] - HUE_LIMIT;
    hue_table[TABLE_UPPER_WRAP] = hue_table[TABLE_FIRST_NOM] + HUE_LIMIT;
}

fn find_display_cusp_for_hue(hue: f32, rgb_corners: &[F3; TOTAL_CORNER_COUNT], jmh_corners: &[F3; TOTAL_CORNER_COUNT], params: &JMhParams, prev: &mut [f32; 2]) -> [f32; 2] {
    let mut upper_corner = 1;
    for i in 1..TOTAL_CORNER_COUNT {
        if jmh_corners[i][2] > hue { upper_corner = i; break; }
    }
    let lower_corner = upper_corner - 1;

    if jmh_corners[lower_corner][2] == hue {
        return [jmh_corners[lower_corner][0], jmh_corners[lower_corner][1]];
    }

    let cl = &rgb_corners[lower_corner];
    let cu = &rgb_corners[upper_corner];
    let mut lower_t = if upper_corner == prev[0] as usize { prev[1] } else { 0.0 };
    let mut upper_t = 1.0f32;

    while upper_t - lower_t > DISPLAY_CUSP_TOLERANCE {
        let sample_t = midpoint_f(lower_t, upper_t);
        let sample = lerp_f3(cl, cu, sample_t);
        let jmh = rgb_to_jmh(&sample, params);
        if jmh[2] < jmh_corners[lower_corner][2] {
            upper_t = sample_t;
        } else if jmh[2] >= jmh_corners[upper_corner][2] {
            lower_t = sample_t;
        } else if jmh[2] > hue {
            upper_t = sample_t;
        } else {
            lower_t = sample_t;
        }
    }

    let sample_t = midpoint_f(lower_t, upper_t);
    let sample = lerp_f3(cl, cu, sample_t);
    let jmh = rgb_to_jmh(&sample, params);
    prev[0] = upper_corner as f32;
    prev[1] = sample_t;
    [jmh[0], jmh[1]]
}

fn build_cusp_table(hue_table: &Table1D, rgb_corners: &[F3; TOTAL_CORNER_COUNT], jmh_corners: &[F3; TOTAL_CORNER_COUNT], params: &JMhParams) -> Table3D {
    let mut prev = [0.0f32; 2];
    let mut out = Table3D::default();
    for i in TABLE_FIRST_NOM..TABLE_UPPER_WRAP {
        let hue = hue_table[i];
        let jm = find_display_cusp_for_hue(hue, rgb_corners, jmh_corners, params, &mut prev);
        out[i] = [jm[0], jm[1] * (1.0 + SMOOTH_M * SMOOTH_CUSPS), hue];
    }
    out[TABLE_LOWER_WRAP] = [out[TABLE_LAST_NOM][0], out[TABLE_LAST_NOM][1], hue_table[TABLE_LOWER_WRAP]];
    out[TABLE_UPPER_WRAP] = [out[TABLE_FIRST_NOM][0], out[TABLE_FIRST_NOM][1], hue_table[TABLE_UPPER_WRAP]];
    out
}

fn any_below_zero(rgb: &F3) -> bool { rgb[0] < 0.0 || rgb[1] < 0.0 || rgb[2] < 0.0 }
fn outside_hull(rgb: &F3, max_val: f32) -> bool { rgb[0] > max_val || rgb[1] > max_val || rgb[2] > max_val }

fn make_reach_m_table(params: &JMhParams, limit_j_max: f32) -> Table1D {
    let mut table = Table1D::default();
    for i in 0..TABLE_NOMINAL_SIZE {
        let hue = table.base_hue_for_pos(i);
        let mut low = 0.0f32;
        let mut high = 50.0f32;
        let mut found = false;
        while !found && high < 1300.0 {
            let search_jmh = [limit_j_max, high, hue];
            let rgb = jmh_to_rgb(&search_jmh, params);
            found = any_below_zero(&rgb);
            if !found { low = high; high += 50.0; }
        }
        while high - low > 1e-2 {
            let mid = (high + low) * 0.5;
            let search_jmh = [limit_j_max, mid, hue];
            let rgb = jmh_to_rgb(&search_jmh, params);
            if any_below_zero(&rgb) { high = mid; } else { low = mid; }
        }
        table[i + TABLE_BASE_INDEX] = high;
    }
    table[TABLE_LOWER_WRAP] = table[TABLE_LAST_NOM];
    table[TABLE_UPPER_WRAP] = table[TABLE_FIRST_NOM];
    table
}

fn make_uniform_hue_gamut_table(reach_params: &JMhParams, limit_params: &JMhParams, peak_lum: f32, forward_limit: f32, sp: &SharedCompressionParams) -> (Table3D, Table1D) {
    let reach_corners = find_reach_corners(reach_params, sp.limit_j_max, forward_limit);
    let (rgb_corners, jmh_corners) = build_limiting_cusp_corners(limit_params, peak_lum);
    let (sorted, unique) = extract_sorted_cube_hues(&reach_corners, &jmh_corners);
    let mut hue_table = Table1D::default();
    build_hue_table(&mut hue_table, &sorted, unique);
    let cusp_table = build_cusp_table(&hue_table, &rgb_corners, &jmh_corners, limit_params);
    (cusp_table, hue_table)
}

struct TestData {
    test_jmh: F3,
    j_intersect_src: f32,
    slope: f32,
    j_intersect_cusp: f32,
}

fn gen_gamma_test_data(jm_cusp: &[f32; 2], hue: f32, limit_j_max: f32, mid_j: f32, focus_dist: f32) -> [TestData; GAMMA_TEST_COUNT] {
    let positions = [0.01f32, 0.1, 0.5, 0.8, 0.99];
    let threshold = lerpf(jm_cusp[0], limit_j_max, FOCUS_GAIN_BLEND);
    let focus_j = compute_focus_j(jm_cusp[0], mid_j, limit_j_max);

    std::array::from_fn(|i| {
        let test_j = lerpf(jm_cusp[0], limit_j_max, positions[i]);
        let sg = get_focus_gain(test_j, threshold, limit_j_max, focus_dist);
        let ji = solve_j_intersect(test_j, jm_cusp[1], focus_j, limit_j_max, sg);
        TestData {
            test_jmh: [test_j, jm_cusp[1], hue],
            j_intersect_src: ji,
            slope: compute_compression_slope(ji, focus_j, limit_j_max, sg),
            j_intersect_cusp: solve_j_intersect(jm_cusp[0], jm_cusp[1], focus_j, limit_j_max, sg),
        }
    })
}

fn evaluate_gamma_fit(jm_cusp: &[f32; 2], data: &[TestData; GAMMA_TEST_COUNT], top_gamma_inv: f32, peak_lum: f32, limit_j_max: f32, lower_gamma_inv: f32, limit_params: &JMhParams) -> bool {
    let lum_limit = peak_lum / REFERENCE_LUMINANCE;
    for td in data {
        let approx_m = find_gamut_boundary_m(jm_cusp, limit_j_max, top_gamma_inv, lower_gamma_inv, td.j_intersect_src, td.slope, td.j_intersect_cusp);
        let approx_j = td.j_intersect_src + td.slope * approx_m;
        let rgb = jmh_to_rgb(&[approx_j, approx_m, td.test_jmh[2]], limit_params);
        if !outside_hull(&rgb, lum_limit) { return false; }
    }
    true
}

fn make_upper_hull_gamma(hue_table: &Table1D, cusp_table: &mut Table3D, peak_lum: f32, limit_j_max: f32, mid_j: f32, focus_dist: f32, lower_gamma_inv: f32, limit_params: &JMhParams) {
    for i in TABLE_FIRST_NOM..TABLE_UPPER_WRAP {
        let hue = hue_table[i];
        let jm_cusp = [cusp_table[i][0], cusp_table[i][1]];
        let data = gen_gamma_test_data(&jm_cusp, hue, limit_j_max, mid_j, focus_dist);

        let mut low = GAMMA_MINIMUM;
        let mut high = low + GAMMA_SEARCH_STEP;
        let mut outside = false;
        while !outside && high < GAMMA_MAXIMUM {
            if evaluate_gamma_fit(&jm_cusp, &data, 1.0 / high, peak_lum, limit_j_max, lower_gamma_inv, limit_params) {
                outside = true;
            } else {
                low = high;
                high += GAMMA_SEARCH_STEP;
            }
        }
        while high - low > GAMMA_ACCURACY {
            let test = midpoint_f(high, low);
            if evaluate_gamma_fit(&jm_cusp, &data, 1.0 / test, peak_lum, limit_j_max, lower_gamma_inv, limit_params) {
                high = test;
            } else {
                low = test;
            }
        }
        cusp_table[i][2] = 1.0 / high;
    }
    cusp_table[TABLE_LOWER_WRAP][2] = cusp_table[TABLE_LAST_NOM][2];
    cusp_table[TABLE_UPPER_WRAP][2] = cusp_table[TABLE_FIRST_NOM][2];
}

fn determine_hue_linearity_range(cusp_table: &Table3D) -> [i32; 2] {
    let mut range = [0i32, 1i32];
    for i in TABLE_FIRST_NOM..TABLE_UPPER_WRAP {
        let pos = (cusp_table[i][2] as usize).min(TABLE_NOMINAL_SIZE - 1);
        let delta = i as i32 - pos as i32;
        range[0] = range[0].min(delta);
        range[1] = range[1].max(delta + 1);
    }
    range
}

pub fn init_gamut_compress_params(peak_lum: f32, input_params: &JMhParams, limit_params: &JMhParams, ts: &ToneScaleParams, sh: &SharedCompressionParams, reach_params: &JMhParams) -> GamutCompressParams {
    let mid_j = y_to_j(ts.c_t * REFERENCE_LUMINANCE, input_params);
    let focus_dist = FOCUS_DISTANCE + FOCUS_DISTANCE * FOCUS_DISTANCE_SCALING * ts.log_peak;
    let lower_hull_gamma_inv = 1.0 / (1.14 + 0.07 * ts.log_peak);

    let (mut cusp_table, hue_table) = make_uniform_hue_gamut_table(reach_params, limit_params, peak_lum, ts.forward_limit, sh);
    let search_range = determine_hue_linearity_range(&cusp_table);
    make_upper_hull_gamma(&hue_table, &mut cusp_table, peak_lum, sh.limit_j_max, mid_j, focus_dist, lower_hull_gamma_inv, limit_params);

    GamutCompressParams {
        mid_j,
        focus_dist,
        lower_hull_gamma_inv,
        hue_linearity_search_range: search_range,
        hue_table,
        gamut_cusp_table: cusp_table,
    }
}

// --- Public API: Full Output Transform ---

/// Build full ACES 2.0 Output Transform state.
pub fn init_output_transform(peak_lum: f32, limiting_primaries: &Primaries) -> Aces2State {
    let p_in = init_jmh_params(&ACES_AP0);
    let p_out = init_jmh_params(limiting_primaries);
    let ts = init_tonescale_params(peak_lum);
    let reach = init_jmh_params(&ACES_AP1);
    let shared = init_shared_compression_params(peak_lum, &p_in, &reach);
    let chroma = init_chroma_compress_params(peak_lum, &ts);
    let gamut = init_gamut_compress_params(peak_lum, &p_in, &p_out, &ts, &shared, &reach);

    Aces2State { p_in, p_out, ts, shared, chroma, gamut }
}

/// Apply ACES 2.0 Output Transform forward (scene-referred AP0 → display).
pub fn output_transform_fwd(rgb: &F3, st: &Aces2State) -> F3 {
    let aab = rgb_to_aab(rgb, &st.p_in);
    let jmh = aab_to_jmh(&aab, &st.p_in);

    let rp = resolve_compression_params(jmh[2], &st.shared);
    let h_rad = to_radians(jmh[2]);
    let (cos_hr, sin_hr) = (h_rad.cos(), h_rad.sin());
    let mnorm = chroma_compress_norm(cos_hr, sin_hr, st.chroma.chroma_compress_scale);

    let j_ts = tonescale_a_to_j_fwd(aab[0], &st.p_in, &st.ts);
    let tonemapped = chroma_compress_fwd(&jmh, j_ts, mnorm, &rp, &st.chroma);
    let compressed = gamut_compress_fwd(&tonemapped, &rp, &st.gamut);

    let aab_out = jmh_to_aab_trig(&compressed, cos_hr, sin_hr, &st.p_out);
    aab_to_rgb(&aab_out, &st.p_out)
}

/// Apply ACES 2.0 Output Transform inverse (display → scene-referred AP0).
pub fn output_transform_inv(rgb: &F3, st: &Aces2State) -> F3 {
    let compressed = rgb_to_jmh(rgb, &st.p_out);

    let rp = resolve_compression_params(compressed[2], &st.shared);
    let h_rad = to_radians(compressed[2]);
    let (cos_hr, sin_hr) = (h_rad.cos(), h_rad.sin());
    let mnorm = chroma_compress_norm(cos_hr, sin_hr, st.chroma.chroma_compress_scale);

    let tonemapped = gamut_compress_inv(&compressed, &rp, &st.gamut);
    let j = tonescale_inv_j(tonemapped[0], &st.p_in, &st.ts);
    let jmh = chroma_compress_inv(&tonemapped, j, mnorm, &rp, &st.chroma);

    let aab = jmh_to_aab_trig(&jmh, cos_hr, sin_hr, &st.p_in);
    aab_to_rgb(&aab, &st.p_in)
}

// --- Individual ACES 2.0 FixedFunction operations ---

/// RGB to JMh (degrees output).
pub fn rgb_to_jmh_degrees(rgb: &F3, params: &JMhParams) -> F3 {
    let jmh = rgb_to_jmh(rgb, params);
    [jmh[0], jmh[1], jmh[2]] // hue already in degrees (0..360)
}

/// JMh (degrees input) to RGB.
pub fn jmh_degrees_to_rgb(jmh: &F3, params: &JMhParams) -> F3 {
    let h = wrap_to_hue_limit(jmh[2]);
    jmh_to_rgb(&[jmh[0], jmh[1], h], params)
}

/// Tonescale + chroma compress forward (JMh in degrees).
pub fn tonescale_compress_fwd(jmh_deg: &F3, p: &JMhParams, ts: &ToneScaleParams, shared: &SharedCompressionParams, chroma: &ChromaCompressParams) -> F3 {
    let h = wrap_to_hue_limit(jmh_deg[2]);
    let h_rad = to_radians(h);
    let (cos_hr, sin_hr) = (h_rad.cos(), h_rad.sin());
    let mnorm = chroma_compress_norm(cos_hr, sin_hr, chroma.chroma_compress_scale);
    let rp = resolve_compression_params(h, shared);
    let j_ts = tonescale_fwd_j(jmh_deg[0], p, ts);
    let out = chroma_compress_fwd(&[jmh_deg[0], jmh_deg[1], h], j_ts, mnorm, &rp, chroma);
    [out[0], out[1], out[2]] // hue stays in degrees
}

/// Tonescale + chroma compress inverse (JMh in degrees).
pub fn tonescale_compress_inv(jmh_deg: &F3, p: &JMhParams, ts: &ToneScaleParams, shared: &SharedCompressionParams, chroma: &ChromaCompressParams) -> F3 {
    let h = wrap_to_hue_limit(jmh_deg[2]);
    let h_rad = to_radians(h);
    let (cos_hr, sin_hr) = (h_rad.cos(), h_rad.sin());
    let mnorm = chroma_compress_norm(cos_hr, sin_hr, chroma.chroma_compress_scale);
    let rp = resolve_compression_params(h, shared);
    let j = tonescale_inv_j(jmh_deg[0], p, ts);
    let out = chroma_compress_inv(&[jmh_deg[0], jmh_deg[1], h], j, mnorm, &rp, chroma);
    [out[0], out[1], out[2]]
}

/// Gamut compress forward (JMh in degrees).
pub fn gamut_compress_fwd_deg(jmh_deg: &F3, shared: &SharedCompressionParams, gamut: &GamutCompressParams) -> F3 {
    let h = wrap_to_hue_limit(jmh_deg[2]);
    let rp = resolve_compression_params(h, shared);
    let out = gamut_compress_fwd(&[jmh_deg[0], jmh_deg[1], h], &rp, gamut);
    [out[0], out[1], out[2]]
}

/// Gamut compress inverse (JMh in degrees).
pub fn gamut_compress_inv_deg(jmh_deg: &F3, shared: &SharedCompressionParams, gamut: &GamutCompressParams) -> F3 {
    let h = wrap_to_hue_limit(jmh_deg[2]);
    let rp = resolve_compression_params(h, shared);
    let out = gamut_compress_inv(&[jmh_deg[0], jmh_deg[1], h], &rp, gamut);
    [out[0], out[1], out[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_jmh_roundtrip() {
        let params = init_jmh_params(&ACES_AP0);
        let rgb = [0.18, 0.18, 0.18];
        let jmh = rgb_to_jmh(&rgb, &params);
        let rgb2 = jmh_to_rgb(&jmh, &params);
        for i in 0..3 {
            assert!((rgb[i] - rgb2[i]).abs() < 1e-4, "channel {} mismatch: {} vs {}", i, rgb[i], rgb2[i]);
        }
    }

    #[test]
    fn test_achromatic_j_roundtrip() {
        let params = init_jmh_params(&ACES_AP0);
        let y = 0.18;
        let j = y_to_j(y, &params);
        let y2 = j_to_y(j, &params);
        assert!((y - y2).abs() < 1e-5, "Y roundtrip: {} vs {}", y, y2);
    }

    #[test]
    fn test_tonescale_roundtrip() {
        let ts = init_tonescale_params(1000.0);
        let y = 0.18;
        let y_out = aces_tonescale_fwd(y, &ts);
        let y_back = aces_tonescale_inv(y_out, &ts);
        assert!((y - y_back).abs() < 1e-3, "tonescale roundtrip: {} vs {}", y, y_back);
    }

    #[test]
    fn test_output_transform_smoke() {
        // Rec.709 primaries for sRGB-like display
        let rec709 = Primaries {
            red: Chromaticity { x: 0.64, y: 0.33 },
            green: Chromaticity { x: 0.30, y: 0.60 },
            blue: Chromaticity { x: 0.15, y: 0.06 },
            white: Chromaticity { x: 0.3127, y: 0.3290 },
        };
        let st = init_output_transform(1000.0, &rec709);
        let rgb_in = [0.18, 0.18, 0.18];
        let rgb_out = output_transform_fwd(&rgb_in, &st);
        // Should produce a valid non-zero result
        assert!(rgb_out[0] > 0.0 && rgb_out[1] > 0.0 && rgb_out[2] > 0.0,
            "output should be positive: {:?}", rgb_out);
        // Inverse roundtrip
        let rgb_back = output_transform_inv(&rgb_out, &st);
        for i in 0..3 {
            assert!((rgb_in[i] - rgb_back[i]).abs() < 0.05,
                "channel {} roundtrip: {} vs {}", i, rgb_in[i], rgb_back[i]);
        }
    }

    #[test]
    fn test_matrix_inversion() {
        let m = rgb_to_xyz_m33(&ACES_AP0);
        let m_inv = invert_m33(&m);
        let id = mult_m33_m33(&m, &m_inv);
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((id[i*3+j] - expected).abs() < 1e-4,
                    "identity[{}][{}] = {} (expected {})", i, j, id[i*3+j], expected);
            }
        }
    }
}
