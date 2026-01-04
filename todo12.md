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
- [ ] Ops accept LayeredImage or ImageLayer

## Phase 4: Channel Operations
- [ ] `channel-shuffle` - reorder/rename channels
- [ ] `channel-extract` - extract specific channels

## Phase 5: Continue todo11.md
- [ ] paste op
- [ ] rotate arbitrary
- [ ] warp/distortion
- [ ] ACES IDT/RRT/ODT
