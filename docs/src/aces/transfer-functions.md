# ACES Transfer Functions

Transfer functions encode and decode light values for storage and display.

## Linear vs Encoded

### Linear (Scene-Referred)

```
Scene Light    Linear Value
    0.18   →      0.18      (18% gray)
    0.36   →      0.36      (1 stop brighter)
    0.09   →      0.09      (1 stop darker)
```

**Properties:**
- Direct relationship to light
- Correct for compositing math
- Wide dynamic range needed (half-float minimum)

### Encoded (Log/Gamma)

```
Scene Light    Log Value     Gamma Value
    0.18   →    ~0.40    →     ~0.46
    0.36   →    ~0.45    →     ~0.61
    0.09   →    ~0.35    →     ~0.32
```

**Properties:**
- Perceptually uniform steps
- Efficient storage (8-16 bit)
- Not suitable for compositing

## ACES-Specific Functions

### ACEScct

Logarithmic encoding with toe for color grading:

```
                1.0 ┤              ___
                    │          ___/
                    │      ___/
                    │   __/
                0.5 ┤  /
                    │ /  ← Log section
                    │/
             Linear │    ← Linear toe
                0.0 ┼───────────────────
                    0    0.18    1.0   10
                         Scene Linear
```

**Formula:**
```
ACEScct = 10.5402377 * x + 0.0729055    for x ≤ 0.0078125
        = (log2(x) + 9.72) / 17.52      for x > 0.0078125
```

**vfx-rs usage:**
```rust
use vfx_transfer::{acescct_encode, acescct_decode};

let cct = acescct_encode(0.18);    // ~0.4135 (linear → ACEScct)
let linear = acescct_decode(0.4135);  // ~0.18 (ACEScct → linear)
```

### ACEScc

Pure logarithmic encoding (no toe):

```
                1.0 ┤              ___
                    │          ___/
                    │      ___/
                    │   __/
                0.5 ┤  /
                    │ /
                    │/
                    ├ (goes to -∞ at 0)
               -∞  ┼───────────────────
                    0    0.18    1.0   10
```

**Formula:**
```
ACEScc = (log2(x) + 9.72) / 17.52    for x ≥ 2^-15
       = (log2(2^-16 + x*0.5) + 9.72) / 17.52    for x < 2^-15
```

**vfx-rs usage:**
```rust
use vfx_transfer::{acescc_encode, acescc_decode};

let cc = acescc_encode(0.18);      // ~0.4135 (linear → ACEScc)
let linear = acescc_decode(0.4135);  // ~0.18 (ACEScc → linear)
```

### ACEScct vs ACEScc

| Property | ACEScct | ACEScc |
|----------|---------|--------|
| Shadow handling | Linear toe | Pure log (→ -∞) |
| Minimum value | 0.0 | Approaches -∞ |
| Colorist preference | More common | Original ACES |
| Similar to | Cineon/DPX log | Pure mathematical log |

## Camera Transfer Functions

vfx-rs supports common camera log encodings:

### ARRI LogC

```rust
use vfx_transfer::{log_c_encode, log_c_decode};
use vfx_transfer::{log_c4_encode, log_c4_decode};

// LogC3 (legacy Alexa)
let logc3 = log_c_encode(0.18);  // ~0.391
let linear = log_c_decode(0.391);

// LogC4 (Alexa 35)
let logc4 = log_c4_encode(0.18);  // ~0.278
```

### Sony S-Log

```rust
use vfx_transfer::{s_log2_encode, s_log2_decode};
use vfx_transfer::{s_log3_encode, s_log3_decode};

// S-Log2
let slog2 = s_log2_encode(0.18);  // ~0.339

// S-Log3
let slog3 = s_log3_encode(0.18);  // ~0.406
```

### Panasonic V-Log

```rust
use vfx_transfer::{v_log_encode, v_log_decode};

let vlog = v_log_encode(0.18);  // ~0.423
```

### RED Log3G10

```rust
use vfx_transfer::{log3g10_encode, log3g10_decode};

let redlog = log3g10_encode(0.18);  // ~0.333
```

### Blackmagic Film

```rust
use vfx_transfer::{bmd_film_gen5_encode, bmd_film_gen5_decode};

let bmdfilm = bmd_film_gen5_encode(0.18);  // ~0.38
```

## Display Transfer Functions

### sRGB

The standard for computer monitors:

```rust
use vfx_transfer::{srgb_oetf, srgb_eotf};
// Or via module: use vfx_transfer::srgb::{oetf, eotf};

let srgb = srgb_oetf(0.18);    // ~0.46 (linear → sRGB)
let linear = srgb_eotf(0.46);  // ~0.18 (sRGB → linear)
```

**Formula:**
```
sRGB = 12.92 * L                    for L ≤ 0.0031308
     = 1.055 * L^(1/2.4) - 0.055    for L > 0.0031308
```

### Rec.709

Similar to sRGB but different constants:

```rust
use vfx_transfer::rec709;

let encoded = rec709::oetf(0.18);  // ~0.409
let linear = rec709::eotf(0.409);  // ~0.18
```

### PQ (ST.2084)

HDR10 perceptual quantizer:

```rust
use vfx_transfer::pq;

// Input in nits (cd/m²)
let pq_val = pq::oetf(100.0);  // ~0.508 (100 nits)
let nits = pq::eotf(0.508);  // ~100 nits
```

**Range:** 0 to 10,000 nits

### HLG

Hybrid Log-Gamma for broadcast HDR:

```rust
use vfx_transfer::hlg;

let encoded = hlg::oetf(0.18);
let linear = hlg::eotf(0.5);
```

## Conversion Chart

| Transfer | Mid-Gray Code Value | Stops Above Mid |
|----------|--------------------:|----------------:|
| Linear | 0.180 | - |
| sRGB | 0.461 | 2.9 |
| Rec.709 | 0.409 | 2.4 |
| ACEScct | 0.414 | 6.5 |
| ACEScc | 0.414 | 6.5 |
| LogC3 | 0.391 | 8.0 |
| LogC4 | 0.278 | 17.0 |
| S-Log3 | 0.406 | 6.0 |
| V-Log | 0.423 | 8.0 |
| PQ | 0.508 (100 nits) | 40 |

## Using in Pipeline

```bash
# Decode camera log
vfx color input.dpx -o linear.exr --transfer logc3

# Encode to log for grading
vfx color linear.exr -o grading.exr --transfer acescct

# Encode for display
vfx color linear.exr -o display.png --transfer srgb
```
