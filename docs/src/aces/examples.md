# ACES Practical Examples

Real-world examples of ACES workflows using vfx-rs.

## Example 1: Simple Web Delivery

Convert sRGB images to web-ready output with ACES tonemapping.

```bash
# Input: sRGB JPEG from graphics
# Output: sRGB JPEG with filmic look

# 1. Convert to ACEScg (IDT)
vfx aces hero_graphic.jpg -o working.exr -t idt

# 2. Apply RRT + ODT for web
vfx aces working.exr -o web_final.jpg -t rrt-odt

# Or in one step (temporary file created internally)
vfx aces hero_graphic.jpg -o web_final.jpg -t idt
vfx aces web_final.jpg -o web_final.jpg -t rrt-odt
```

## Example 2: CG Composite

Composite CG render over live-action plate.

```bash
# Input files:
# - bg_plate.exr (already ACEScg from grade)
# - cg_render.exr (linear sRGB primaries from renderer)

# 1. Convert CG to ACEScg
vfx color cg_render.exr -o cg_acescg.exr \
    --from "sRGB Linear" --to ACEScg

# 2. Composite
vfx composite cg_acescg.exr bg_plate.exr \
    -o comp.exr --mode over

# 3. Color correct
vfx color comp.exr -o graded.exr \
    --exposure -0.3 --saturation 1.05

# 4. Output for review (sRGB)
vfx aces graded.exr -o review.jpg -t rrt-odt
```

## Example 3: Multi-Camera Project

Matching footage from different cameras.

```bash
# ARRI Alexa footage (LogC3)
export OCIO=/path/to/aces_1.2_config.ocio
vfx color arri_shot.dpx -o shot_a.exr \
    --from "ARRI LogC3" --to ACEScg

# Sony Venice footage (S-Log3)
vfx color sony_shot.dpx -o shot_b.exr \
    --from "Sony S-Log3 S-Gamut3" --to ACEScg

# RED footage (Log3G10)
vfx color red_shot.dpx -o shot_c.exr \
    --from "RED Log3G10 RWG" --to ACEScg

# All shots now in ACEScg - can be graded consistently
vfx aces shot_a.exr -o shot_a_preview.jpg -t rrt-odt
vfx aces shot_b.exr -o shot_b_preview.jpg -t rrt-odt
vfx aces shot_c.exr -o shot_c_preview.jpg -t rrt-odt
```

## Example 4: HDR and SDR Delivery

Create both HDR and SDR deliverables from ACEScg master.

```bash
# Master file: final_grade.exr (ACEScg)

# SDR delivery (sRGB for web)
vfx aces final_grade.exr -o sdr_web.jpg -t rrt-odt

# SDR delivery (Rec.709 for broadcast)
export OCIO=/path/to/aces_config.ocio
vfx color final_grade.exr -o sdr_broadcast.dpx \
    --from ACEScg --to "Rec.709 - Display"

# HDR delivery (Rec.2020 PQ 1000 nits)
vfx color final_grade.exr -o hdr_delivery.exr \
    --from ACEScg --to "Rec.2020 ST2084 - 1000 nits"

# HDR delivery (Dolby Vision P3)
vfx color final_grade.exr -o dolby_delivery.exr \
    --from ACEScg --to "P3-D65 ST2084 - 1000 nits"
```

## Example 5: VFX Shot Pipeline

Complete VFX shot from plate to delivery.

```bash
#!/bin/bash
# Full VFX shot pipeline

SHOT="sh010"
PLATE="plate_${SHOT}.dpx"

# 1. Ingest plate to ACEScg
echo "Converting plate to ACEScg..."
vfx color "${PLATE}" -o "${SHOT}_plate.exr" \
    --from "ARRI LogC3" --to ACEScg

# 2. Process VFX elements (already in ACEScg from Nuke)
# Composite main elements
vfx composite "${SHOT}_fg.exr" "${SHOT}_plate.exr" \
    -o "${SHOT}_comp_v001.exr" --mode over

# Add atmosphere layer
vfx composite "${SHOT}_atmos.exr" "${SHOT}_comp_v001.exr" \
    -o "${SHOT}_comp_v002.exr" --mode add --opacity 0.3

# 3. Apply show LUT (if any)
if [ -f "show_lmt.cube" ]; then
    vfx lut "${SHOT}_comp_v002.exr" \
        -o "${SHOT}_styled.exr" -l show_lmt.cube
else
    cp "${SHOT}_comp_v002.exr" "${SHOT}_styled.exr"
fi

# 4. Generate outputs
# Client review (sRGB)
vfx aces "${SHOT}_styled.exr" \
    -o "review/${SHOT}_review.jpg" -t rrt-odt

# DI delivery (preserve ACEScg)
cp "${SHOT}_styled.exr" "delivery/${SHOT}_acescg.exr"

# Archive (ACES2065-1)
vfx color "${SHOT}_styled.exr" \
    -o "archive/${SHOT}_aces.exr" \
    --from ACEScg --to ACES2065-1

echo "Shot ${SHOT} complete!"
```

## Example 6: Batch Processing

Process multiple images in a directory.

```bash
# Note: `batch` command supports convert, resize, blur, flip_h, flip_v
# ACES transforms must be done with a shell loop:

# Convert all JPEG files to ACEScg EXR
for f in input/*.jpg; do
    vfx aces "$f" -o "acescg_exr/$(basename "$f" .jpg).exr" -t idt
done

# Apply RRT+ODT to all ACEScg files
for f in acescg_exr/*.exr; do
    vfx aces "$f" -o "output_srgb/$(basename "$f" .exr).png" -t rrt-odt
done

# Resize all outputs (batch supports resize)
vfx batch -i "output_srgb/*.png" \
    -o ./thumbs \
    --op resize \
    --args scale=0.25 \
    -f jpg
```

## Example 7: EXR Layer Workflow

Working with multi-layer EXR files.

```bash
# List layers in VFX output
vfx layers vfx_shot.exr

# Output:
#   beauty (RGBA)
#   diffuse (RGB)
#   specular (RGB)
#   emission (RGB)
#   depth (Z)

# Extract beauty layer for comp
vfx extract-layer vfx_shot.exr \
    -o beauty.exr --layer beauty

# Merge separate passes into layers
# Note: --names uses repeated flags, not comma-separated
vfx merge-layers \
    beauty.exr diffuse.exr specular.exr depth.exr \
    -o combined.exr \
    --names beauty --names diffuse --names specular --names depth

# Apply ACES to beauty layer (extract first, then transform)
vfx extract-layer vfx_shot.exr -o beauty_only.exr --layer beauty
vfx aces beauty_only.exr -o preview.jpg -t rrt-odt
```

## Example 8: UDIM Texture Pipeline

Process UDIM texture sets.

```bash
# View UDIM set info
vfx udim info textures/diffuse.<UDIM>.exr

# Convert all tiles to ACEScg
for tile in textures/diffuse.*.exr; do
    base=$(basename "$tile" .exr)
    vfx aces "$tile" -o "acescg/${base}.exr" -t idt
done

# Or batch convert
vfx udim convert \
    textures/diffuse.<UDIM>.exr \
    acescg/diffuse.<UDIM>.exr

# Create atlas from UDIM tiles
vfx udim atlas \
    textures/diffuse.<UDIM>.exr \
    atlas_diffuse.exr \
    --tile-size 1024
```

## Verification Commands

```bash
# Check image is in expected color space
vfx info output.exr --all | grep chromaticities

# Compare before/after
vfx diff original.exr processed.exr -o diff.exr

# View with correct transform
vfx view acescg_file.exr --colorspace ACEScg
```
