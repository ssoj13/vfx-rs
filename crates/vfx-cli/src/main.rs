//! vfx - Unified VFX image processing CLI
//!
//! Combines functionality of oiiotool, iconvert, iinfo, idiff, maketx

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(name = "vfx")]
#[command(author, version, about = "Unified VFX image processing CLI")]
#[command(long_about = "
A comprehensive image processing tool for VFX workflows.
Combines functionality of oiiotool, iconvert, iinfo, idiff, and maketx.

Examples:
  vfx info image.exr                    # Show image info
  vfx convert input.exr output.png      # Convert formats
  vfx resize input.exr -w 1920 -h 1080 -o output.exr
  vfx diff a.exr b.exr                  # Compare images
  vfx composite fg.exr bg.exr -o out.exr
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Number of threads (0 = auto)
    #[arg(short = 'j', long, global = true, default_value = "0")]
    threads: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Display image information (like iinfo)
    #[command(visible_alias = "i")]
    Info(InfoArgs),

    /// Convert image format (like iconvert)
    #[command(visible_alias = "c")]
    Convert(ConvertArgs),

    /// Resize/scale image
    #[command(visible_alias = "r")]
    Resize(ResizeArgs),

    /// Crop image
    Crop(CropArgs),

    /// Compare images (like idiff)
    #[command(visible_alias = "d")]
    Diff(DiffArgs),

    /// Composite images (over, add, multiply, etc.)
    #[command(visible_alias = "comp")]
    Composite(CompositeArgs),

    /// Apply blur filter
    Blur(BlurArgs),

    /// Apply sharpening
    Sharpen(SharpenArgs),

    /// Apply color transform
    Color(ColorArgs),

    /// Apply LUT
    Lut(LutArgs),

    /// Flip/rotate image
    Transform(TransformArgs),

    /// Create tiled/mipmapped texture (like maketx)
    #[command(visible_alias = "tx")]
    Maketx(MaketxArgs),

    /// Search for pattern in image metadata (like igrep)
    Grep(GrepArgs),

    /// Batch process multiple images
    Batch(BatchArgs),
}

#[derive(Args)]
struct InfoArgs {
    /// Input image(s)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Show detailed stats
    #[arg(short, long)]
    stats: bool,

    /// Show all metadata
    #[arg(short, long)]
    all: bool,

    /// Machine-readable output (JSON)
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ConvertArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    output: PathBuf,

    /// Bit depth: 8, 16, 32, half
    #[arg(short = 'd', long)]
    depth: Option<String>,

    /// Compression (format-specific)
    #[arg(short = 'c', long)]
    compression: Option<String>,

    /// Quality (0-100, for JPEG)
    #[arg(short = 'q', long)]
    quality: Option<u8>,
}

#[derive(Args)]
struct ResizeArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Target width
    #[arg(short, long)]
    width: Option<usize>,

    /// Target height
    #[arg(short = 'H', long)]
    height: Option<usize>,

    /// Scale factor (e.g., 0.5, 2.0)
    #[arg(short, long)]
    scale: Option<f32>,

    /// Filter: box, bilinear, lanczos, mitchell
    #[arg(short, long, default_value = "lanczos")]
    filter: String,

    /// Fit mode: exact, fit, fill
    #[arg(long, default_value = "exact")]
    fit: String,
}

#[derive(Args)]
struct CropArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// X offset
    #[arg(short)]
    x: usize,

    /// Y offset
    #[arg(short)]
    y: usize,

    /// Width
    #[arg(short)]
    w: usize,

    /// Height
    #[arg(short = 'H')]
    h: usize,
}

#[derive(Args)]
struct DiffArgs {
    /// First image
    a: PathBuf,

    /// Second image
    b: PathBuf,

    /// Output difference image
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Fail threshold (max allowed difference)
    #[arg(short, long, default_value = "0.0")]
    threshold: f32,

    /// Per-pixel warning threshold
    #[arg(short, long)]
    warn: Option<f32>,
}

#[derive(Args)]
struct CompositeArgs {
    /// Foreground image
    fg: PathBuf,

    /// Background image
    bg: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Blend mode: over, add, multiply, screen
    #[arg(short, long, default_value = "over")]
    mode: String,

    /// Opacity (0.0-1.0)
    #[arg(long, default_value = "1.0")]
    opacity: f32,
}

#[derive(Args)]
struct BlurArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Blur radius in pixels
    #[arg(short, long, default_value = "3")]
    radius: usize,

    /// Blur type: box, gaussian
    #[arg(short = 't', long, default_value = "gaussian")]
    blur_type: String,
}

#[derive(Args)]
struct SharpenArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Sharpen amount (0.0-10.0)
    #[arg(short, long, default_value = "1.0")]
    amount: f32,
}

#[derive(Args)]
struct ColorArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Source color space
    #[arg(long)]
    from: Option<String>,

    /// Target color space
    #[arg(long)]
    to: Option<String>,

    /// Apply transfer function: srgb, rec709, log, pq
    #[arg(long)]
    transfer: Option<String>,

    /// Exposure adjustment (stops)
    #[arg(long)]
    exposure: Option<f32>,

    /// Gamma correction
    #[arg(long)]
    gamma: Option<f32>,

    /// Saturation multiplier
    #[arg(long)]
    saturation: Option<f32>,
}

#[derive(Args)]
struct LutArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// LUT file (.cube, .clf)
    #[arg(short, long)]
    lut: PathBuf,

    /// Invert LUT
    #[arg(long)]
    invert: bool,
}

#[derive(Args)]
struct TransformArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Flip horizontal
    #[arg(long)]
    flip_h: bool,

    /// Flip vertical
    #[arg(long)]
    flip_v: bool,

    /// Rotate: 90, 180, 270
    #[arg(short, long)]
    rotate: Option<i32>,

    /// Transpose (swap X/Y)
    #[arg(long)]
    transpose: bool,
}

#[derive(Args)]
struct MaketxArgs {
    /// Input image
    input: PathBuf,

    /// Output texture
    #[arg(short, long)]
    output: PathBuf,

    /// Generate mipmaps
    #[arg(short, long)]
    mipmap: bool,

    /// Tile size
    #[arg(short, long, default_value = "64")]
    tile: usize,

    /// Filter for mipmaps
    #[arg(short, long, default_value = "lanczos")]
    filter: String,

    /// Wrap mode: black, clamp, periodic
    #[arg(short, long, default_value = "black")]
    wrap: String,
}

#[derive(Args)]
struct GrepArgs {
    /// Pattern to search
    pattern: String,

    /// Images to search
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Case insensitive
    #[arg(short, long)]
    ignore_case: bool,
}

#[derive(Args)]
struct BatchArgs {
    /// Input pattern (glob)
    #[arg(short, long)]
    input: String,

    /// Output directory
    #[arg(short, long)]
    output_dir: PathBuf,

    /// Operation to apply
    #[arg(short, long)]
    op: String,

    /// Operation arguments (key=value)
    #[arg(short, long)]
    args: Vec<String>,

    /// Output format extension
    #[arg(short, long)]
    format: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configure thread pool
    if cli.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()
            .context("Failed to configure thread pool")?;
    }

    match cli.command {
        Commands::Info(args) => commands::info::run(args, cli.verbose),
        Commands::Convert(args) => commands::convert::run(args, cli.verbose),
        Commands::Resize(args) => commands::resize::run(args, cli.verbose),
        Commands::Crop(args) => commands::crop::run(args, cli.verbose),
        Commands::Diff(args) => commands::diff::run(args, cli.verbose),
        Commands::Composite(args) => commands::composite::run(args, cli.verbose),
        Commands::Blur(args) => commands::blur::run(args, cli.verbose),
        Commands::Sharpen(args) => commands::sharpen::run(args, cli.verbose),
        Commands::Color(args) => commands::color::run(args, cli.verbose),
        Commands::Lut(args) => commands::lut::run(args, cli.verbose),
        Commands::Transform(args) => commands::transform::run(args, cli.verbose),
        Commands::Maketx(args) => commands::maketx::run(args, cli.verbose),
        Commands::Grep(args) => commands::grep::run(args, cli.verbose),
        Commands::Batch(args) => commands::batch::run(args, cli.verbose),
    }
}
