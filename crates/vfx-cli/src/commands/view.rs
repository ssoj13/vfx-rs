//! View command - image viewer with OCIO color management.

use anyhow::Result;

use crate::ViewArgs;

/// Run the view command.
pub fn run(args: ViewArgs, verbose: u8) -> Result<()> {
    let config = vfx_view::ViewerConfig {
        ocio: args.ocio,
        display: args.display,
        view: args.view,
        colorspace: args.colorspace,
        verbose: if verbose > 0 { 1 } else { 0 },
    };

    let exit_code = vfx_view::run_opt(args.input, config);
    
    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
