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
  vfx info image.exr --stats --all      # Full stats and metadata
  vfx convert input.exr output.png      # Convert formats
  vfx convert input.exr output.exr -d half -c piz
  vfx resize input.exr -w 1920 -h 1080 -o output.exr
  vfx diff a.exr b.exr                  # Compare images
  vfx composite fg.exr bg.exr -o out.exr
  vfx color input.exr -o out.exr --from ACEScg --to sRGB
  vfx lut input.exr -o out.exr -l look.cube
  vfx maketx input.exr -o tex.tx -m -t 64
  vfx --allow-non-color blur id.exr -o id_blur.exr
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

    /// Allow processing non-color channels (ID/Mask/Generic) by casting to float
    #[arg(long = "allow-non-color", alias = "force-processing", alias = "force", global = true)]
    allow_non_color: bool,
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

    /// List layers and channels in multi-layer EXR files
    #[command(visible_alias = "l")]
    Layers(LayersArgs),

    /// Extract a single layer from multi-layer EXR
    #[command(name = "extract-layer", visible_alias = "xl")]
    ExtractLayer(ExtractLayerArgs),

    /// Merge layers from multiple files into one EXR
    #[command(name = "merge-layers", visible_alias = "ml")]
    MergeLayers(MergeLayersArgs),

    /// Shuffle/rearrange image channels
    #[command(name = "channel-shuffle", visible_alias = "cs")]
    ChannelShuffle(ChannelShuffleArgs),

    /// Extract specific channels to new image
    #[command(name = "channel-extract", visible_alias = "cx")]
    ChannelExtract(ChannelExtractArgs),

    /// Paste/overlay one image onto another
    Paste(PasteArgs),

    /// Rotate image by arbitrary angle
    Rotate(RotateArgs),

    /// Apply warp/distortion effect
    Warp(WarpArgs),

    /// Apply ACES color transforms (IDT/RRT/ODT)
    Aces(AcesArgs),
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

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    layer: Option<String>,
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

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    layer: Option<String>,
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

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    layer: Option<String>,
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

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    layer: Option<String>,
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

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    layer: Option<String>,
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

/// Arguments for the `layers` command.
#[derive(Args)]
struct LayersArgs {
    /// Input EXR file(s)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Machine-readable output (JSON)
    #[arg(long)]
    json: bool,
}

/// Arguments for the `extract-layer` command.
#[derive(Args)]
struct ExtractLayerArgs {
    /// Input EXR file
    input: PathBuf,

    /// Output file
    #[arg(short, long)]
    output: PathBuf,

    /// Layer name or index to extract
    #[arg(short, long)]
    layer: Option<String>,
}

/// Arguments for the `merge-layers` command.
#[derive(Args)]
struct MergeLayersArgs {
    /// Input files (each becomes a layer)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output EXR file
    #[arg(short, long)]
    output: PathBuf,

    /// Custom layer names (one per input)
    #[arg(short, long)]
    names: Vec<String>,
}

/// Arguments for the `channel-shuffle` command.
#[derive(Args)]
struct ChannelShuffleArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Channel shuffle pattern (e.g., BGR, RGBA, RRR, RGB1)
    /// R/G/B/A = copy that channel, 0 = black, 1 = white
    #[arg(short, long)]
    pattern: String,
}

/// Arguments for the `channel-extract` command.
#[derive(Args)]
struct ChannelExtractArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Channels to extract (by name R/G/B/A or index 0/1/2)
    #[arg(short, long, required = true)]
    channels: Vec<String>,
}

/// Arguments for the `paste` command.
#[derive(Args)]
struct PasteArgs {
    /// Background image
    background: PathBuf,

    /// Foreground image to paste
    foreground: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// X offset (can be negative)
    #[arg(short, long, default_value = "0")]
    x: i32,

    /// Y offset (can be negative)
    #[arg(short, long, default_value = "0")]
    y: i32,

    /// Use alpha blending (if foreground has alpha)
    #[arg(short, long)]
    blend: bool,
}

/// Arguments for the `rotate` command.
#[derive(Args)]
struct RotateArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Rotation angle in degrees (counter-clockwise)
    #[arg(short, long)]
    angle: f32,

    /// Background color R,G,B or R,G,B,A (default: 0,0,0)
    #[arg(long, default_value = "0,0,0")]
    bg_color: String,
}

/// Arguments for the `warp` command.
#[derive(Args)]
struct WarpArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Warp type: barrel, pincushion, fisheye, twist, wave, spherize, ripple
    #[arg(short = 't', long = "type")]
    warp_type: String,

    /// Primary parameter (k1 for lens, strength for effects)
    #[arg(short = 'k', long, default_value = "0.2")]
    k1: f32,

    /// Secondary parameter (k2 for lens, frequency for wave/ripple)
    #[arg(long, default_value = "0.0")]
    k2: f32,

    /// Radius for effects (twist, spherize)
    #[arg(short, long, default_value = "0.5")]
    radius: f32,
}

/// Arguments for the `aces` command.
#[derive(Args)]
struct AcesArgs {
    /// Input image
    input: PathBuf,

    /// Output image
    #[arg(short, long)]
    output: PathBuf,

    /// Transform type: idt (sRGB->ACEScg), rrt (tonemap), odt (ACEScg->sRGB), rrt-odt (full output)
    #[arg(short = 't', long = "transform", default_value = "rrt-odt")]
    transform: String,

    /// RRT variant: default, high-contrast
    #[arg(long, default_value = "default")]
    rrt_variant: String,
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
        Commands::Resize(args) => commands::resize::run(args, cli.verbose, cli.allow_non_color),
        Commands::Crop(args) => commands::crop::run(args, cli.verbose, cli.allow_non_color),
        Commands::Diff(args) => commands::diff::run(args, cli.verbose, cli.allow_non_color),
        Commands::Composite(args) => {
            commands::composite::run(args, cli.verbose, cli.allow_non_color)
        }
        Commands::Blur(args) => commands::blur::run(args, cli.verbose, cli.allow_non_color),
        Commands::Sharpen(args) => commands::sharpen::run(args, cli.verbose, cli.allow_non_color),
        Commands::Color(args) => commands::color::run(args, cli.verbose, cli.allow_non_color),
        Commands::Lut(args) => commands::lut::run(args, cli.verbose, cli.allow_non_color),
        Commands::Transform(args) => {
            commands::transform::run(args, cli.verbose, cli.allow_non_color)
        }
        Commands::Maketx(args) => commands::maketx::run(args, cli.verbose, cli.allow_non_color),
        Commands::Grep(args) => commands::grep::run(args, cli.verbose),
        Commands::Batch(args) => commands::batch::run(args, cli.verbose, cli.allow_non_color),
        Commands::Layers(args) => commands::layers::run_layers(args, cli.verbose),
        Commands::ExtractLayer(args) => commands::layers::run_extract_layer(args, cli.verbose),
        Commands::MergeLayers(args) => commands::layers::run_merge_layers(args, cli.verbose),
        Commands::ChannelShuffle(args) => commands::channels::run_shuffle(args, cli.verbose),
        Commands::ChannelExtract(args) => commands::channels::run_extract(args, cli.verbose),
        Commands::Paste(args) => commands::paste::run(args, cli.verbose),
        Commands::Rotate(args) => commands::rotate::run(args, cli.verbose),
        Commands::Warp(args) => commands::warp::run(args, cli.verbose),
        Commands::Aces(args) => commands::aces::run(args, cli.verbose),
    }
}
