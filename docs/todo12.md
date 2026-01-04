# TODO 12: LayeredImage Integration

## Phase 1: CLI Layer Commands
- [x] `vfx layers <file>` - list layers and channels
- [x] `vfx extract-layer <file> --layer <name> -o <out>` - extract single layer
- [x] `vfx merge-layers <files...> -o <out>` - merge layers from multiple files

## Phase 2: CLI Integration
- [x] `convert` preserves layers for EXR->EXR
- [x] `info` shows layer info for multi-layer files
- [x] All ops work on specified layer (--layer flag)

## Phase 3: vfx-ops API Guard
- [x] Move ensure_color_processing to vfx-ops
- [x] Ops accept LayeredImage or ImageLayer (layer_ops.rs)

## Phase 4: Channel Operations
- [x] `channel-shuffle` - reorder/rename channels
- [x] `channel-extract` - extract specific channels

## Phase 5: Continue todo11.md
- [x] paste op
- [x] rotate arbitrary
- [x] warp/distortion (barrel, pincushion, fisheye, twist, wave, spherize, ripple)
- [x] ACES IDT/RRT/ODT
