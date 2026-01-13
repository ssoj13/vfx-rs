// Debug CDL identity test
use vfx_color::cdl::Cdl;
use sha2::{Sha256, Digest};

fn main() {
    let cdl = Cdl::new();  // Identity
    
    // Simple test
    let mut rgb = [0.5f32, 0.3, 0.2];
    println!("Input: {:?}", rgb);
    cdl.apply(&mut rgb);
    println!("Output: {:?}", rgb);
    println!("Expected: [0.5, 0.3, 0.2]");
    println!();
    
    // Generate RGB cube like in golden.rs
    let size = 8;
    let mut cube: Vec<[f32; 3]> = Vec::with_capacity(size * size * size);
    
    for r in 0..size {
        for g in 0..size {
            for b in 0..size {
                cube.push([
                    r as f32 / (size - 1) as f32,
                    g as f32 / (size - 1) as f32,
                    b as f32 / (size - 1) as f32,
                ]);
            }
        }
    }
    
    println!("Cube size: {}", cube.len());
    println!("First entry: {:?}", cube[0]);
    println!("Last entry: {:?}", cube[cube.len()-1]);
    
    // Hash input (before CDL)
    let input_hash = compute_hash(&cube);
    println!("\nInput hash: {}", input_hash);
    
    // Apply identity CDL
    let mut result = cube.clone();
    for rgb in &mut result {
        cdl.apply(rgb);
    }
    
    // Hash output (after CDL)
    let output_hash = compute_hash(&result);
    println!("Output hash: {}", output_hash);
    println!("Hashes match: {}", input_hash == output_hash);
    
    // Check first few entries
    println!("\nFirst 5 entries after CDL:");
    for i in 0..5 {
        println!("  {:?} -> {:?}", cube[i], result[i]);
    }
    
    // Expected golden hash
    let expected = "cff6e617bdcb51b048e7518086a4d1f2d78f0a0259e73047be195b50010885cd";
    println!("\nExpected: {}", expected);
    println!("Input matches expected: {}", input_hash == expected);
    println!("Output matches expected: {}", output_hash == expected);
}

fn compute_hash(data: &[[f32; 3]]) -> String {
    let factor = 100000.0f64;  // 10^5
    
    let quantized: Vec<i64> = data
        .iter()
        .flat_map(|rgb| rgb.iter())
        .map(|&v| (v as f64 * factor).round() as i64)
        .collect();
    
    let bytes: Vec<u8> = quantized
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    
    hex::encode(result)
}
