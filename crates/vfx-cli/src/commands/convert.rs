//! Format conversion command (like iconvert)

use crate::ConvertArgs;
use anyhow::Result;

pub fn run(args: ConvertArgs, verbose: bool) -> Result<()> {
    if verbose {
        println!("Converting {} -> {}", args.input.display(), args.output.display());
    }

    let image = super::load_image(&args.input)?;

    // TODO: Apply bit depth conversion if specified
    if let Some(ref depth) = args.depth {
        if verbose {
            println!("  Target depth: {}", depth);
        }
    }

    super::save_image(&args.output, &image)?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}
