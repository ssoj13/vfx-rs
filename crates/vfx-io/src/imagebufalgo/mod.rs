//! OIIO-compatible ImageBufAlgo implementation.
//!
//! ImageBufAlgo provides a comprehensive set of image processing operations
//! compatible with OpenImageIO's ImageBufAlgo namespace.
//!
//! # Modules
//!
//! - [`patterns`] - Pattern generation (fill, checker, noise)
//! - [`channels`] - Channel operations (shuffle, append, extract)
//! - [`geometry`] - Geometric operations (crop, flip, rotate, resize)
//! - [`arithmetic`] - Arithmetic operations (add, sub, mul, div, over)
//! - [`color`] - Color operations (saturate, contrast, color maps, gamma)
//! - [`composite`] - Compositing operations (Porter-Duff, blend modes)
//! - [`stats`] - Statistics and analysis (histogram, compare, min/max)
//! - [`ocio`] - OCIO color conversion (colorconvert, ociodisplay, ociolook)
//! - [`fft`] - Fast Fourier Transform operations
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebuf::{ImageBuf, InitializePixels};
//! use vfx_io::imagebufalgo::{fill, add, flip, saturate};
//! use vfx_core::ImageSpec;
//!
//! // Create and fill an image
//! let spec = ImageSpec::rgba(100, 100);
//! let mut buf = ImageBuf::new(spec, InitializePixels::No);
//! fill(&mut buf, &[1.0, 0.0, 0.0, 1.0], None);
//!
//! // Apply operations
//! let flipped = flip(&buf, None);
//! let desaturated = saturate(&flipped, 0.5, 0, None);
//! ```

pub mod patterns;
pub mod channels;
pub mod geometry;
pub mod arithmetic;
pub mod color;
pub mod composite;
pub mod stats;
pub mod ocio;
pub mod deep;
pub mod filters;
pub mod fft;
pub mod drawing;
pub mod warp;
pub mod demosaic;
pub mod texture;
pub mod fillholes;

#[cfg(feature = "text")]
pub mod text;

// Re-export commonly used functions
pub use patterns::{zero, fill, checker, noise};
pub use channels::{channels, channel_append, channel_sum, extract_channel, flatten, get_alpha};
pub use geometry::{
    crop, cut, flip, flop, transpose,
    rotate90, rotate180, rotate270,
    resize, resample, fit,
    rotate, circular_shift, paste,
    reorient, reorient_auto,
    ResizeFilter,
};
pub use arithmetic::{add, sub, mul, div, abs, absdiff, pow, clamp, invert, over, max, min, mad, normalize, normalize_into};

// Color operations
pub use color::{
    premult, unpremult, repremult,
    saturate, contrast_remap,
    color_map, ColorMapName,
    colormatrixtransform,
    rangecompress, rangeexpand,
    srgb_to_linear, linear_to_srgb,
};

// Compositing operations
pub use composite::{
    // Porter-Duff
    under, in_op, out, atop, xor,
    // Blend modes
    screen, multiply, overlay, hardlight, softlight,
    difference, exclusion, colordodge, colorburn, add_blend,
    // Z-depth compositing
    zover, zover_into,
};

// Statistics operations
pub use stats::{
    compute_pixel_stats, PixelStats,
    compare, compare_relative, CompareResults,
    is_constant_color, is_constant_channel, is_monochrome,
    histogram, Histogram,
    maxchan, minchan,
    color_range_check, RangeCheckResult,
    color_count, unique_color_count,
};

// OCIO color conversion operations
pub use ocio::{
    colorconvert, colorconvert_into, colorconvert_inplace,
    ociodisplay, ociodisplay_into,
    ociolook, ociolook_into,
    ociofiletransform, ociofiletransform_into,
    ocionamedtransform, ocionamedtransform_into,
    equivalent_colorspace,
};

// Deep image operations
pub use deep::{
    flatten_deep, flatten_deep_into,
    deepen, deepen_with_z,
    deep_merge, deep_merge_into,
    deep_holdout, deep_holdout_matte,
    deep_tidy, deep_stats, DeepStats,
};

// Filter operations
pub use filters::{
    median, median_into,
    blur, blur_into,
    unsharp_mask, unsharp_mask_into,
    dilate, dilate_into,
    erode, erode_into,
    morph_open, morph_close,
    laplacian, sharpen, sobel,
    convolve, convolve_into,
    box_blur, box_blur_into,
    // New OIIO-parity functions from review branch
    KernelType, make_kernel, make_kernel_from_name,
    dilate_n, erode_n,
    morph_gradient, top_hat, black_hat,
    convolve_with_border, convolve_with_border_into,
    bilateral, bilateral_into,
};

// FFT operations
pub use fft::{
    fft, fft_into,
    ifft, ifft_into,
    complex_to_polar, complex_to_polar_into,
    polar_to_complex, polar_to_complex_into,
};

// Drawing operations
pub use drawing::{
    render_point, render_line, render_box,
    render_circle, render_ellipse, render_polygon,
};

// Demosaic operations
pub use demosaic::{
    demosaic,
    BayerPattern, DemosaicAlgorithm,
};

// Texture/mipmap operations
pub use texture::{
    make_texture, make_mip_level,
    mip_level_count, mip_dimensions,
    MipmapFilter, MipmapOptions,
};

// Hole filling operations
pub use fillholes::{
    fillholes_pushpull, has_holes, count_holes,
    FillHolesOptions,
};

// Text rendering (optional)
#[cfg(feature = "text")]
pub use text::{
    render_text, render_text_into,
    TextStyle, TextAlign,
};

// Warp operations
pub use warp::{
    warp, warp_into,
    st_warp, st_warp_into,
    WarpWrap,
    matrix_identity, matrix_translate, matrix_scale, matrix_rotate, matrix_shear,
    matrix_multiply, matrix_invert,
};
