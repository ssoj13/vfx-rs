# Glossary

VFX and color science terminology used in vfx-rs.

## A

**ACES** (Academy Color Encoding System)
: Industry-standard color management framework. Defines working colorspaces (ACEScg), transforms (IDT, RRT, ODT), and interchange formats.

**ACEScg**
: ACES working colorspace optimized for CGI rendering. Linear, AP1 primaries, D60 white point.

**Alpha**
: Transparency channel. 0 = transparent, 1 = opaque.

## B

**Bit Depth**
: Number of bits per channel. Higher = more precision. Common: 8-bit (256 levels), 16-bit (65536), 32-bit float.

## C

**CDL** (Color Decision List)
: ASC standard for basic color correction. Defines slope, offset, power, saturation.

**Chromaticity**
: Color defined by x,y coordinates (ignoring brightness). Used to specify primaries and white points.

**CLF** (Common LUT Format)
: Academy XML format for LUT interchange. Supports multiple transforms in sequence.

**Colorspace**
: Definition of how RGB values map to real colors. Includes primaries, white point, and transfer function.

**Composite**
: Combine multiple images. Over operation places foreground on background using alpha.

## D

**D65**
: CIE standard illuminant. 6500K daylight white point. Used by sRGB, Rec.709.

**DPX**
: Digital Picture Exchange format. Common in film scanning/recording. Often 10-bit log.

## E

**EOTF** (Electro-Optical Transfer Function)
: Converts encoded values to linear light for display. Inverse of OETF.

**EXR**
: OpenEXR format. Industry standard for VFX. Supports float, multi-layer, HDR.

**Exposure**
: Light level adjustment in stops. +1 stop = 2x brightness.

## F

**f16** (Half Float)
: 16-bit floating point. Range ±65504, precision varies. Good balance of range/size.

**f32** (Full Float)
: 32-bit floating point. IEEE 754 single precision. Maximum quality for processing.

## G

**Gamma**
: Power function for encoding/decoding. sRGB ≈ 2.2 gamma.

**Gamut**
: Range of colors a colorspace can represent. Larger gamuts contain more saturated colors.

## H

**HDR** (High Dynamic Range)
: Images with brightness range exceeding standard displays. Values can exceed 1.0.

**HLG** (Hybrid Log-Gamma)
: HDR transfer function. Backwards-compatible with SDR displays.

## I

**ICC**
: International Color Consortium. Defines color profiles for devices and colorspaces.

**IDT** (Input Device Transform)
: ACES transform from camera/input colorspace to ACES.

## L

**Linear**
: Light values directly proportional to physical light. Required for correct math operations.

**LUT** (Look-Up Table)
: Precomputed transform. 1D LUT transforms single values, 3D LUT transforms RGB together.

## M

**Matte**
: Alpha channel or mask defining regions.

## N

**Nits**
: Unit of luminance (cd/m²). SDR displays: ~100 nits, HDR: 1000+ nits.

## O

**OCIO** (OpenColorIO)
: Open-source color management library. Industry standard for VFX pipelines.

**ODT** (Output Device Transform)
: ACES transform from ACES to display colorspace.

**OETF** (Opto-Electronic Transfer Function)
: Converts linear light to encoded values for storage/transmission.

**Over**
: Alpha compositing operation. Result = FG × α + BG × (1 - α).

## P

**PQ** (Perceptual Quantizer)
: HDR transfer function (SMPTE ST 2084). Used in HDR10, Dolby Vision.

**Premultiplied Alpha**
: RGB values already multiplied by alpha. Required for correct compositing.

**Primaries**
: RGB chromaticity coordinates defining a colorspace's color triangle.

## R

**Rec.709**
: ITU standard for HDTV. Same primaries as sRGB, different transfer function.

**Rec.2020**
: ITU standard for UHD/4K. Wider gamut than Rec.709.

**RRT** (Reference Rendering Transform)
: ACES tonemap. Compresses HDR to displayable range while preserving appearance.

## S

**sRGB**
: Standard RGB colorspace. Most common for web/consumer displays.

**Scene-referred**
: Values represent actual light levels from the scene. Can exceed 0-1 range.

**Display-referred**
: Values represent display output. Typically 0-1 range.

## T

**Tetrahedral Interpolation**
: High-quality 3D LUT interpolation. Better than trilinear at color boundaries.

**Tonemap**
: Compress HDR range to displayable range. Maps bright values, preserves shadow detail.

**Transfer Function**
: Mathematical function converting between linear and encoded values.

**Trilinear Interpolation**
: 3D LUT interpolation using 8 surrounding cube vertices. Fast but can show artifacts.

## W

**White Point**
: Reference white color. D65 (6500K), D60, D50 are common standards.

**Working Space**
: Colorspace used during image processing. Should be linear for math operations.

## X

**XYZ**
: CIE 1931 colorspace. Device-independent, covers all visible colors. Interchange standard.
