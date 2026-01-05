//! UDIM texture set operations

use anyhow::{Context, Result};
use std::path::Path;
use vfx_io::udim::{UdimResolver, UdimTile};

use crate::{UdimArgs, UdimCommand};
use super::{load_image, save_image};

/// Run UDIM subcommand
pub fn run(args: UdimArgs, verbose: u8) -> Result<()> {
    match args.command {
        UdimCommand::Info { pattern } => run_info(&pattern, verbose),
        UdimCommand::Convert { input, output, compression } => {
            run_convert(&input, &output, compression.as_deref(), verbose)
        }
        UdimCommand::Atlas { input, output, tile_size } => {
            run_atlas(&input, &output, tile_size, verbose)
        }
        UdimCommand::Split { input, output, tile_size } => {
            run_split(&input, &output, tile_size, verbose)
        }
    }
}

/// Show UDIM texture set information
fn run_info(pattern: &Path, verbose: u8) -> Result<()> {
    let resolver = UdimResolver::new(pattern)
        .with_context(|| format!("Failed to resolve UDIM pattern: {}", pattern.display()))?;

    let count = resolver.tile_count();
    if count == 0 {
        println!("No UDIM tiles found for pattern: {}", pattern.display());
        return Ok(());
    }

    println!("UDIM Texture Set: {}", pattern.display());
    println!("Tiles: {}", count);

    if let Some((min, max)) = resolver.bounds() {
        println!("Bounds: U[{}..{}] V[{}..{}]", min.u, max.u, min.v, max.v);
    }

    println!();

    // Collect and sort tiles
    let mut tiles: Vec<_> = resolver.tiles().collect();
    tiles.sort_by_key(|(tile, _)| tile.udim());

    for (tile, path) in &tiles {
        print!("  {} (U={}, V={})", tile.udim(), tile.u, tile.v);
        if verbose {
            // Get image info
            if let Ok(img) = vfx_io::read(path) {
                print!(" - {}x{} {}ch", img.width, img.height, img.channels);
            }
            print!(" -> {}", path.display());
        }
        println!();
    }

    Ok(())
}

/// Convert all tiles to another format
fn run_convert(input: &Path, output: &Path, compression: Option<&str>, verbose: u8) -> Result<()> {
    let resolver = UdimResolver::new(input)
        .with_context(|| format!("Failed to resolve input: {}", input.display()))?;

    if resolver.tile_count() == 0 {
        anyhow::bail!("No UDIM tiles found for: {}", input.display());
    }

    // Create output resolver to build paths
    let out_resolver = UdimResolver::new(output)
        .with_context(|| format!("Failed to parse output pattern: {}", output.display()))?;

    let tiles: Vec<_> = resolver.tiles().collect();
    
    for (tile, src_path) in &tiles {
        let dst_path = out_resolver.build_path(tile.udim());
        
        if verbose {
            println!("Converting {} -> {}", src_path.display(), dst_path.display());
        }

        let mut image = load_image(src_path)?;
        
        // Apply compression if specified and output is EXR
        if let Some(comp) = compression {
            if dst_path.extension().map(|e| e.eq_ignore_ascii_case("exr")).unwrap_or(false) {
                image.metadata.attrs.set("compression", comp.into());
            }
        }

        save_image(&dst_path, &image)?;
    }

    println!("Converted {} tiles", tiles.len());
    Ok(())
}

/// Create atlas from UDIM tiles
fn run_atlas(input: &Path, output: &Path, tile_size: u32, verbose: u8) -> Result<()> {
    let resolver = UdimResolver::new(input)
        .with_context(|| format!("Failed to resolve: {}", input.display()))?;

    if resolver.tile_count() == 0 {
        anyhow::bail!("No UDIM tiles found for: {}", input.display());
    }

    let (min_tile, max_tile) = resolver.bounds()
        .context("Failed to get tile bounds")?;

    // Atlas dimensions in tiles
    let cols = (max_tile.u - min_tile.u + 1) as usize;
    let rows = (max_tile.v - min_tile.v + 1) as usize;
    let atlas_w = cols * tile_size as usize;
    let atlas_h = rows * tile_size as usize;

    if verbose {
        println!("Creating {}x{} atlas ({}x{} tiles @ {}px)", 
                 atlas_w, atlas_h, cols, rows, tile_size);
    }

    // Determine channel count from first tile
    let first_tile = resolver.tiles().next()
        .context("No tiles available")?;
    let first_img = load_image(first_tile.1)?;
    let channels = first_img.channels as usize;

    // Create atlas buffer
    let mut atlas_data = vec![0.0f32; atlas_w * atlas_h * channels];

    // Process each tile
    for (tile, path) in resolver.tiles() {
        if verbose {
            println!("  Processing tile {} from {}", tile.udim(), path.display());
        }

        let img = load_image(path)?;
        let img_data = img.to_f32();

        // Position in atlas (Y is flipped: V=0 at bottom)
        let ax = ((tile.u - min_tile.u) as usize) * tile_size as usize;
        let ay = ((max_tile.v - tile.v) as usize) * tile_size as usize;

        // Resize tile if needed
        let tile_data = if img.width != tile_size || img.height != tile_size {
            resize_tile(&img_data, img.width as usize, img.height as usize, 
                       tile_size as usize, channels)
        } else {
            img_data
        };

        // Copy to atlas
        for ty in 0..tile_size as usize {
            for tx in 0..tile_size as usize {
                let src_idx = (ty * tile_size as usize + tx) * channels;
                let dst_idx = ((ay + ty) * atlas_w + ax + tx) * channels;
                for c in 0..channels {
                    if src_idx + c < tile_data.len() && dst_idx + c < atlas_data.len() {
                        atlas_data[dst_idx + c] = tile_data[src_idx + c];
                    }
                }
            }
        }
    }

    // Save atlas
    let atlas = vfx_io::ImageData::from_f32(
        atlas_w as u32, 
        atlas_h as u32, 
        channels as u32,
        atlas_data
    );
    save_image(output, &atlas)?;

    println!("Created atlas: {} ({}x{})", output.display(), atlas_w, atlas_h);
    Ok(())
}

/// Split single image into UDIM tiles
fn run_split(input: &Path, output: &Path, tile_size: u32, verbose: u8) -> Result<()> {
    let image = load_image(input)?;
    let data = image.to_f32();
    let w = image.width as usize;
    let h = image.height as usize;
    let ch = image.channels as usize;
    let ts = tile_size as usize;

    // Calculate grid
    let cols = (w + ts - 1) / ts;
    let rows = (h + ts - 1) / ts;

    if verbose {
        println!("Splitting {}x{} into {}x{} tiles @ {}px", w, h, cols, rows, ts);
    }

    // Validate grid fits UDIM limits
    if cols > 10 {
        anyhow::bail!("Image too wide: {} tiles (max 10 for UDIM)", cols);
    }
    if rows > 100 {
        anyhow::bail!("Image too tall: {} tiles (max 100 for UDIM)", rows);
    }

    // Create output resolver
    let out_resolver = UdimResolver::new(output)
        .with_context(|| format!("Failed to parse output pattern: {}", output.display()))?;

    let mut count = 0;
    for row in 0..rows {
        for col in 0..cols {
            let tile = UdimTile::new(col as u32, (rows - 1 - row) as u32);
            let dst_path = out_resolver.build_path(tile.udim());

            // Extract tile data
            let x0 = col * ts;
            let y0 = row * ts;
            let tw = ts.min(w - x0);
            let th = ts.min(h - y0);

            let mut tile_data = vec![0.0f32; ts * ts * ch];
            for ty in 0..th {
                for tx in 0..tw {
                    let src_idx = ((y0 + ty) * w + x0 + tx) * ch;
                    let dst_idx = (ty * ts + tx) * ch;
                    for c in 0..ch {
                        tile_data[dst_idx + c] = data[src_idx + c];
                    }
                }
            }

            let tile_img = vfx_io::ImageData::from_f32(
                ts as u32,
                ts as u32,
                ch as u32,
                tile_data
            );

            if verbose {
                println!("  Tile {} -> {}", tile.udim(), dst_path.display());
            }

            save_image(&dst_path, &tile_img)?;
            count += 1;
        }
    }

    println!("Created {} UDIM tiles", count);
    Ok(())
}

/// Simple bilinear resize for tile
fn resize_tile(src: &[f32], sw: usize, sh: usize, size: usize, ch: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; size * size * ch];
    let sx = sw as f32 / size as f32;
    let sy = sh as f32 / size as f32;

    for dy in 0..size {
        for dx in 0..size {
            let fx = dx as f32 * sx;
            let fy = dy as f32 * sy;
            let x0 = (fx as usize).min(sw - 1);
            let y0 = (fy as usize).min(sh - 1);
            let x1 = (x0 + 1).min(sw - 1);
            let y1 = (y0 + 1).min(sh - 1);
            let fx = fx - x0 as f32;
            let fy = fy - y0 as f32;

            let dst_idx = (dy * size + dx) * ch;
            for c in 0..ch {
                let c00 = src[(y0 * sw + x0) * ch + c];
                let c10 = src[(y0 * sw + x1) * ch + c];
                let c01 = src[(y1 * sw + x0) * ch + c];
                let c11 = src[(y1 * sw + x1) * ch + c];
                let top = c00 * (1.0 - fx) + c10 * fx;
                let bot = c01 * (1.0 - fx) + c11 * fx;
                dst[dst_idx + c] = top * (1.0 - fy) + bot * fy;
            }
        }
    }
    dst
}
