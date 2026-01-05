# Logging & Debugging

vfx-rs uses structured logging via the `tracing` crate for observability.

## Verbosity Levels

| Flag | Level | Shows |
|------|-------|-------|
| (none) | warn | Errors and warnings only |
| `-v` | info | High-level progress |
| `-vv` | debug | Detailed operations |
| `-vvv` | trace | Function entry/exit, all details |

## Examples

```bash
# Normal operation (quiet)
vfx convert input.exr output.png

# Info level - see what's happening
vfx convert -v input.exr output.png
# Converting input.exr (Exr) -> output.png (Png)
# Done.

# Debug level - operation details
vfx resize -vv input.exr -s 0.5 -o out.exr
# 0.001s DEBUG vfx_io: Reading image path=input.exr format=Exr
# 0.085s DEBUG vfx_ops::resize: Attempting GPU resize
# 0.086s DEBUG vfx_ops::resize: Using GPU backend: Vulkan
# 0.102s DEBUG vfx_io: Writing image path=out.exr format=Exr

# Trace level - everything
vfx info -vvv image.exr
# 0.000s TRACE vfx::commands::info: info::run files=1
# 0.000s DEBUG vfx::commands::info: Reading image info path=image.exr
# 0.081s TRACE vfx_io: vfx_io::read path=image.exr
# ...
```

## File Logging

Write logs to file with `-l` / `--log`:

```bash
# Default log file (vfx.log next to binary)
vfx batch -vv -l "*.exr" -o out/ --op resize

# Custom log path
vfx batch -vv --log=batch_2024.log "*.exr" -o out/ --op resize

# Both console and file
vfx convert -vv --log=debug.log input.exr output.png
```

## Log Format

```
0.001234567s DEBUG vfx_io: Reading image path=input.exr format=Exr
│            │     │       │
│            │     │       └─ Structured fields
│            │     └─ Message
│            └─ Module path
└─ Uptime (seconds)
```

## Environment Variable

Override log level via `RUST_LOG`:

```bash
# Set specific module levels
RUST_LOG=vfx_io=trace,vfx_ops=debug vfx convert input.exr out.png

# All trace
RUST_LOG=trace vfx info image.exr
```

## Debugging Tips

1. **Format detection issues**: Use `-vvv` to see magic byte detection
2. **GPU fallback**: `-vv` shows if GPU or CPU path is used
3. **Color accuracy**: Trace shows transfer function applications
4. **Performance**: Compare timings in debug output
