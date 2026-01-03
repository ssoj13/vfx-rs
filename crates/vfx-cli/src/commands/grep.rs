//! Metadata grep command (like igrep)

use crate::GrepArgs;
use anyhow::Result;
use vfx_io::Format;

pub fn run(args: GrepArgs, _verbose: bool) -> Result<()> {
    let pattern = if args.ignore_case {
        args.pattern.to_lowercase()
    } else {
        args.pattern.clone()
    };

    for path in &args.input {
        // Get file metadata
        let _metadata = std::fs::metadata(path)?;
        let filename = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Check filename
        let name_to_check = if args.ignore_case {
            filename.to_lowercase()
        } else {
            filename.to_string()
        };

        if name_to_check.contains(&pattern) {
            println!("{}: filename matches", path.display());
        }

        // Load and check image properties
        if let Ok(image) = super::load_image(path) {
            let props = format!(
                "{}x{} {}ch",
                image.width, image.height, image.channels
            );

            let props_to_check = if args.ignore_case {
                props.to_lowercase()
            } else {
                props.clone()
            };

            if props_to_check.contains(&pattern) {
                println!("{}: {} matches", path.display(), props);
            }

            // Check format
            let format = Format::detect(path).unwrap_or(Format::Unknown);
            let format_str = format!("{:?}", format);
            let format_to_check = if args.ignore_case {
                format_str.to_lowercase()
            } else {
                format_str.clone()
            };

            if format_to_check.contains(&pattern) {
                println!("{}: format {:?} matches", path.display(), format);
            }
        }
    }

    Ok(())
}
