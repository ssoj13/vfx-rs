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
use vfx_transfer::{linear_to_acescct, acescct_to_linear};

let cct = linear_to_acescct(0.18);  // ~0.4135
let linear = acescct_to_linear(0.4135);  // ~0.18
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
use vfx_transfer::{linear_to_acescc, acescc_to_linear};

let cc = linear_to_acescc(0.18);  // ~0.4135
let linear = acescc_to_linear(0.4135);  // ~0.18
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
use vfx_transfer::{linear_to_logc3, logc3_to_linear};
use vfx_transfer::{linear_to_logc4, logc4_to_linear};

// LogC3 (legacy Alexa)
let logc3 = linear_to_logc3(0.18);  // ~0.391

// LogC4 (Alexa 35)
let logc4 = linear_to_logc4(0.18);  // ~0.278
```

### Sony S-Log

```rust
use vfx_transfer::{linear_to_slog2, slog2_to_linear};
use vfx_transfer::{linear_to_slog3, slog3_to_linear};

// S-Log2
let slog2 = linear_to_slog2(0.18);  // ~0.339

// S-Log3
let slog3 = linear_to_slog3(0.18);  // ~0.406
```

### Panasonic V-Log

```rust
use vfx_transfer::{linear_to_vlog, vlog_to_linear};

let vlog = linear_to_vlog(0.18);  // ~0.423
```

### RED Log3G10

```rust
use vfx_transfer::{linear_to_redlog, redlog_to_linear};

let redlog = linear_to_redlog(0.18);  // ~0.333
```

### Blackmagic Film

```rust
use vfx_transfer::{linear_to_bmdfilm, bmdfilm_to_linear};

let bmdfilm = linear_to_bmdfilm(0.18);  // ~0.38
```

## Display Transfer Functions

### sRGB

The standard for computer monitors:

```rust
use vfx_transfer::{linear_to_srgb, srgb_to_linear};

let srgb = linear_to_srgb(0.18);  // ~0.46
let linear = srgb_to_linear(0.46);  // ~0.18
```

**Formula:**
```
sRGB = 12.92 * L                    for L ≤ 0.0031308
     = 1.055 * L^(1/2.4) - 0.055    for L > 0.0031308
```

### Rec.709

Similar to sRGB but different constants:

```rust
use vfx_transfer::{linear_to_rec709, rec709_to_linear};

let rec709 = linear_to_rec709(0.18);  // ~0.409
```

### PQ (ST.2084)

HDR10 perceptual quantizer:

```rust
use vfx_transfer::{linear_to_pq, pq_to_linear};

// Input in nits (cd/m²)
let pq = linear_to_pq(100.0);  // ~0.508 (100 nits)
let nits = pq_to_linear(0.508);  // ~100 nits
```

**Range:** 0 to 10,000 nits

### HLG

Hybrid Log-Gamma for broadcast HDR:

```rust
use vfx_transfer::{linear_to_hlg, hlg_to_linear};

let hlg = linear_to_hlg(0.18);
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
