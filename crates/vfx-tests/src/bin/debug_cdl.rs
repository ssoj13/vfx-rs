use vfx_color::cdl::Cdl;

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
    let cube = rgb_cube_8();
    
    println!("Rust RGB cube first 5:");
    for (i, rgb) in cube.iter().enumerate().take(5) {
        println!("  [{:}] [{:.8}, {:.8}, {:.8}]", i, rgb[0], rgb[1], rgb[2]);
    }
    
    // Test CDL contrast
    let cdl = Cdl::new()
        .with_offset([-0.1, -0.1, -0.1])
        .with_power([1.2, 1.2, 1.2]);
    
    println!("\nCDL contrast (offset=-0.1, power=1.2):");
    for (i, rgb) in cube.iter().enumerate().take(5) {
        let mut result = *rgb;
        cdl.apply(&mut result);
        println!("  [{:}] [{:.8}, {:.8}, {:.8}] -> [{:.8}, {:.8}, {:.8}]",
            i, rgb[0], rgb[1], rgb[2], result[0], result[1], result[2]);
    }
    
    // Check pixel 0
    let mut p0 = [0.0, 0.0, 0.0];
    cdl.apply(&mut p0);
    println!("\nPixel 0 (0,0,0) -> [{:.8}, {:.8}, {:.8}]", p0[0], p0[1], p0[2]);
    
    // Check pixel 256
    let idx = 256;
    let mut p256 = cube[idx];
    let orig = p256;
    cdl.apply(&mut p256);
    println!("Pixel {} ({:.8},{:.8},{:.8}) -> [{:.8}, {:.8}, {:.8}]", 
        idx, orig[0], orig[1], orig[2], p256[0], p256[1], p256[2]);
    
    // Manual calculation for pixel 1 (0, 0, 0.14285715)
    println!("\nManual calculation for pixel 1:");
    let b = 1.0 / 7.0_f32;  // 0.14285715
    println!("  b = {:.10}", b);
    let v = b * 1.0 + (-0.1);
    println!("  After offset: {:.10}", v);
    let v_clamped = v.clamp(0.0, 1.0);
    println!("  After clamp [0,1]: {:.10}", v_clamped);
    let v_power = v_clamped.powf(1.2);
    println!("  After power 1.2: {:.10}", v_power);
}
