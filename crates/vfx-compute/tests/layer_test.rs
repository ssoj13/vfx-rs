//! Integration tests for LayerProcessor with real EXR files.

use std::path::Path;
use vfx_compute::{LayerProcessor, ComputeOp, ChannelClassification};

/// Path to test files
fn test_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("test")
        .join(name)
}

#[test]
fn test_layer_processor_with_exr() {
    let path = test_path("owl.exr");
    if !path.exists() {
        eprintln!("Skipping test: {} not found", path.display());
        return;
    }

    // Read EXR file as layers
    let layered = vfx_io::exr::read_layers(&path).expect("Failed to read owl.exr");
    let layer = &layered.layers[0];

    println!("Layer '{}': {}x{}, {} channels", 
        layer.name, layer.width, layer.height, layer.channels.len());
    for ch in &layer.channels {
        println!("  - {} ({:?})", ch.name, ch.sample_type);
    }

    // Classify channels
    let class = ChannelClassification::from_layer(layer);
    println!("Classification:");
    println!("  color: {:?}", class.color.as_ref().map(|g| &g.names));
    println!("  alpha: {:?}", class.alpha.as_ref().map(|g| &g.names));
    println!("  other_f32: {}", class.other_f32.len());
    println!("  passthrough: {:?}", class.passthrough);

    // Create processor and apply ops
    let mut proc = LayerProcessor::auto().expect("Failed to create LayerProcessor");
    
    let output = proc.process_layer(layer, &[
        ComputeOp::Exposure(0.5),
    ]).expect("Failed to process layer");

    assert_eq!(output.width, layer.width);
    assert_eq!(output.height, layer.height);
    assert_eq!(output.channels.len(), layer.channels.len());
    
    println!("Processing successful!");
}

#[test]
fn test_layer_processor_color_ops() {
    // Create a synthetic layer for testing
    use vfx_io::{ImageLayer, ImageChannel, ChannelKind, ChannelSampleType, ChannelSamples};
    
    let pixel_count = 16; // 4x4
    
    let layer = ImageLayer {
        name: "test".to_string(),
        width: 4,
        height: 4,
        channels: vec![
            ImageChannel {
                name: "R".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.5; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "G".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.3; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "B".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.2; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "A".to_string(),
                kind: ChannelKind::Alpha,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![1.0; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
        ],
    };

    let mut proc = LayerProcessor::auto().expect("Failed to create LayerProcessor");
    
    // Apply +1 stop exposure (2x brightness)
    let output = proc.process_layer(&layer, &[
        ComputeOp::Exposure(1.0),
    ]).expect("Failed to process layer");

    // Check R channel doubled
    let r_samples = output.channels[0].samples.to_f32();
    assert!((r_samples[0] - 1.0).abs() < 0.01, "Expected 1.0, got {}", r_samples[0]);
    
    // Check G channel doubled
    let g_samples = output.channels[1].samples.to_f32();
    assert!((g_samples[0] - 0.6).abs() < 0.01, "Expected 0.6, got {}", g_samples[0]);
    
    // Alpha should be unchanged (not a color op target)
    let a_samples = output.channels[3].samples.to_f32();
    assert!((a_samples[0] - 1.0).abs() < 0.01, "Alpha should be unchanged");
    
    println!("Color ops test passed!");
}

#[test]
fn test_u32_passthrough() {
    use vfx_io::{ImageLayer, ImageChannel, ChannelKind, ChannelSampleType, ChannelSamples};
    
    let pixel_count = 4; // 2x2
    
    let layer = ImageLayer {
        name: "test".to_string(),
        width: 2,
        height: 2,
        channels: vec![
            ImageChannel {
                name: "R".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.5; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "G".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.3; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "B".to_string(),
                kind: ChannelKind::Color,
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(vec![0.2; pixel_count]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
            ImageChannel {
                name: "ObjectID".to_string(),
                kind: ChannelKind::Generic,
                sample_type: ChannelSampleType::U32,
                samples: ChannelSamples::U32(vec![100, 200, 300, 400]),
                sampling: (1, 1),
                quantize_linearly: false,
            },
        ],
    };

    let class = ChannelClassification::from_layer(&layer);
    assert_eq!(class.passthrough.len(), 1, "Should have 1 passthrough channel");
    assert_eq!(class.passthrough[0], 3, "ObjectID should be at index 3");

    let mut proc = LayerProcessor::auto().expect("Failed to create LayerProcessor");
    
    let output = proc.process_layer(&layer, &[
        ComputeOp::Exposure(1.0),
    ]).expect("Failed to process layer");

    // U32 channel should be unchanged
    let id_samples = match &output.channels[3].samples {
        ChannelSamples::U32(v) => v.clone(),
        _ => panic!("Expected U32 samples"),
    };
    
    assert_eq!(id_samples, vec![100, 200, 300, 400], "U32 data should be unchanged");
    println!("U32 passthrough test passed!");
}
