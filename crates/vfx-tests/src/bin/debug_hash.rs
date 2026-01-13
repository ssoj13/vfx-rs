use sha2::{Sha256, Digest};

const HASH_PRECISION: i32 = 5;

fn compute_hash_f32(data: &[f32]) -> String {
    let factor = 10f64.powi(HASH_PRECISION);
    
    // Quantize to i64 (matching Python implementation)
    let quantized: Vec<i64> = data
        .iter()
        .map(|&v| (v as f64 * factor).round() as i64)
        .collect();
    
    // Hash the bytes
    let bytes: Vec<u8> = quantized
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    
    hex::encode(result)
}

mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}

fn rgb_cube_8() -> Vec<[f32; 3]> {
    let size = 8;
    let mut cube = Vec::with_capacity(size * size * size);
    
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
    
    cube
}

fn main() {
    println!("=== Full RGB Cube Hash Test ===");
    
    let cube = rgb_cube_8();
    println!("Cube length: {} (expected 512)", cube.len());
    println!("First: {:?}", cube[0]);
    println!("Last: {:?}", cube[cube.len()-1]);
    
    // Flatten
    let flat: Vec<f32> = cube.iter().flat_map(|rgb| rgb.iter().copied()).collect();
    println!("Flat length: {} (expected 1536)", flat.len());
    println!("First 9 flat: {:?}", &flat[..9]);
    
    // Quantize
    let factor = 10f64.powi(HASH_PRECISION);
    let quantized: Vec<i64> = flat.iter().map(|&v| (v as f64 * factor).round() as i64).collect();
    println!("First 9 quantized: {:?}", &quantized[..9]);
    
    // Hash input
    let hash = compute_hash_f32(&flat);
    println!("\nInput hash: {}", hash);
    
    let expected = "cff6e617bdcb51b048e7518086a4d1f2d78f0a0259e73047be195b50010885cd";
    println!("Expected hash: {}", expected);
    println!("Input match: {}", hash == expected);
    
    // Now test with CDL identity
    println!("\n=== CDL Identity Test ===");
    use vfx_color::cdl::Cdl;
    
    let cdl = Cdl::new();
    println!("CDL: slope={:?} offset={:?} power={:?} sat={}",
        cdl.slope, cdl.offset, cdl.power, cdl.saturation);
    
    let mut result: Vec<[f32; 3]> = cube.clone();
    for rgb in &mut result {
        cdl.apply(rgb);
    }
    
    println!("First 3 after CDL:");
    for i in 0..3 {
        println!("  {:?} -> {:?}", cube[i], result[i]);
    }
    
    let flat_out: Vec<f32> = result.iter().flat_map(|rgb| rgb.iter().copied()).collect();
    let hash_out = compute_hash_f32(&flat_out);
    println!("\nOutput hash: {}", hash_out);
    println!("Output match expected: {}", hash_out == expected);
}
