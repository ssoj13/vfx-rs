# Parity Check: vfx-rs vs OCIO/OIIO

## OpenColorIO (OCIO) Parity

### Config API
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Load from file | `Config::CreateFromFile()` | `Config::from_file()` | `ColorConfig.from_file()` | ✅ |
| Load from string | `Config::CreateFromStream()` | `Config::from_string()` | `ColorConfig.from_string()` | ✅ |
| Built-in configs | `Config::CreateFromBuiltinConfig()` | `builtin::aces_1_3()` | `ColorConfig.aces_1_3()` | ✅ |
| Serialize to YAML | `Config::serialize()` | `Config::serialize()` | ❌ | ⚠️ Rust only |
| Write to file | N/A | `Config::write_to_file()` | ❌ | ⚠️ Rust only |
| Config validation | `Config::validate()` | `validate::check()` | ❌ | ⚠️ Rust only |

### Color Spaces
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Get by name | `getColorSpace()` | `colorspace()` | `has_colorspace()` | ✅ |
| Get all names | `getColorSpaceNames()` | `colorspaces()` | `colorspace_names()` | ✅ |
| Family | `getFamily()` | `family()` | `colorspace_family()` | ✅ |
| Encoding | `getEncoding()` | `encoding()` | `colorspace_encoding()` | ✅ |
| Is data | `isData()` | `is_data()` | `is_colorspace_data()` | ✅ |
| Description | `getDescription()` | `description()` | `colorspace_description()` | ✅ |
| Aliases | `getAliases()` | ❌ | ❌ | ❌ Missing |
| Categories | `getCategories()` | ❌ | ❌ | ❌ Missing |

### Roles
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Get role | `getColorSpaceFromRole()` | `roles().get()` | `role_colorspace()` | ✅ |
| Has role | N/A | `roles().has()` | `has_role()` | ✅ |
| scene_linear | `ROLE_SCENE_LINEAR` | `scene_linear` role | `scene_linear()` | ✅ |
| reference | `ROLE_REFERENCE` | `reference` role | ❌ | ⚠️ |
| compositing_log | `ROLE_COMPOSITING_LOG` | `compositing_log` role | ❌ | ⚠️ |
| color_timing | `ROLE_COLOR_TIMING` | `color_timing` role | ❌ | ⚠️ |
| data | `ROLE_DATA` | `data` role | ❌ | ⚠️ |

### Displays & Views
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Get displays | `getDisplays()` | `displays()` | `display_names()` | ✅ |
| Get views | `getViews()` | `display.views()` | `view_name_by_index()` | ✅ |
| Default display | `getDefaultDisplay()` | `default_display()` | `default_display()` | ✅ |
| Default view | `getDefaultView()` | `default_view()` | `default_view()` | ✅ |
| Active displays | `getActiveDisplays()` | `active_displays()` | ❌ | ⚠️ |
| Active views | `getActiveViews()` | `active_views()` | ❌ | ⚠️ |
| Shared views | `getSharedViews()` | `shared_views()` | `shared_view_names()` | ✅ |
| Viewing rules | `getViewingRules()` | `viewing_rules()` | ❌ | ⚠️ |

### Looks
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Get look | `getLook()` | `look()` | `has_look()` | ✅ |
| Get all looks | `getLookNames()` | `looks()` | `look_name_by_index()` | ✅ |
| Process string | `getLookProcess()` | `parse_looks()` | ❌ | ⚠️ |

### Processor
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Create processor | `getProcessor()` | `processor()` | via `colorconvert()` | ✅ |
| With context | `getProcessor(context)` | `processor_with_context()` | ❌ | ⚠️ |
| With looks | N/A | `processor_with_looks()` | via `ociolook()` | ✅ |
| Display processor | `getDisplayViewProcessor()` | `display_processor()` | via `ociodisplay()` | ✅ |
| Optimization | `OptimizationFlags` | `OptimizationLevel` | ❌ | ⚠️ |
| Apply RGB | `apply()` | `apply_rgb()` | via functions | ✅ |
| Apply RGBA | `apply()` | `apply_rgba()` | via functions | ✅ |
| CPU processor | `CPUProcessor` | `Processor` | `Processor` | ✅ |
| GPU processor | `GPUProcessor` | `GpuProcessor` | ❌ | ⚠️ Rust only |

### Transforms (Supported)
| Transform | OCIO C++ | vfx-ocio | GPU | Status |
|-----------|----------|----------|-----|--------|
| Matrix | ✅ | ✅ | ✅ | ✅ |
| CDL | ✅ | ✅ | ✅ | ✅ |
| Exponent | ✅ | ✅ | ✅ | ✅ |
| ExponentWithLinear | ✅ | ✅ | ✅ | ✅ |
| Log | ✅ | ✅ | ✅ | ✅ |
| LogAffine | ✅ | ✅ | ✅ | ✅ |
| LogCamera | ✅ | ✅ | ✅ | ✅ |
| Range | ✅ | ✅ | ✅ | ✅ |
| Lut1D | ✅ | ✅ | ✅ | ✅ |
| Lut3D | ✅ | ✅ | ✅ | ✅ |
| FileTransform | ✅ | ✅ | ⚠️ | ✅ |
| GroupTransform | ✅ | ✅ | ✅ | ✅ |
| ColorSpace | ✅ | ✅ | ✅ | ✅ |
| Display | ✅ | ✅ | ✅ | ✅ |
| Look | ✅ | ✅ | ✅ | ✅ |
| ExposureContrast | ✅ | ✅ | ✅ | ✅ |
| FixedFunction | ✅ | ✅ | ⚠️ | ✅ |
| GradingPrimary | ✅ | ✅ | ✅ | ✅ |
| GradingTone | ✅ | ✅ | ✅ | ✅ |
| GradingRGBCurve | ✅ | ✅ | ⚠️ | ✅ |
| Allocation | ✅ | ✅ | ✅ | ✅ |
| BuiltinTransform | ✅ | ❌ | ❌ | ❌ Missing |

### Advanced Features
| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Context variables | ✅ | ✅ | ❌ | ⚠️ |
| Environment vars | ✅ | ✅ | ❌ | ⚠️ |
| Search paths | ✅ | ✅ | ❌ | ⚠️ |
| File rules | ✅ | ✅ | `colorspace_from_filepath()` | ✅ |
| Named transforms | ✅ | ✅ | `named_transform_names()` | ✅ |
| Config builder | ✅ | ✅ (ConfigBuilder) | ❌ | ⚠️ |
| Baker (LUT export) | ✅ | ✅ (Baker) | ❌ | ⚠️ |
| Dynamic properties | ✅ | ✅ (DynamicProcessor) | ❌ | ⚠️ |
| Processor cache | ✅ | ✅ (ProcessorCache) | ❌ | ⚠️ |

---

## OpenImageIO (OIIO) Parity

### ImageBuf Operations
| Feature | OIIO C++ | vfx-io | Python | Status |
|---------|----------|--------|--------|--------|
| colorconvert | ✅ | ✅ | ✅ | ✅ |
| ociodisplay | ✅ | ✅ | ✅ | ✅ |
| ociolook | ✅ | ✅ | ✅ | ✅ |
| ociofiletransform | ✅ | ✅ | ✅ | ✅ |
| unpremult | ✅ | ✅ | ✅ | ✅ |
| premult | ✅ | ✅ | ✅ | ✅ |
| repremult | ✅ | ✅ | ❌ | ⚠️ |

### ColorConfig (OIIO wrapper)
| Feature | OIIO C++ | vfx-io | Python | Status |
|---------|----------|--------|--------|--------|
| ColorConfig class | ✅ | ✅ | ✅ | ✅ |
| getNumColorSpaces | ✅ | ✅ | ✅ | ✅ |
| getColorSpaceNameByIndex | ✅ | ✅ | ✅ | ✅ |
| getNumDisplays | ✅ | ✅ | ✅ | ✅ |
| getNumViews | ✅ | ✅ | ✅ | ✅ |
| getNumLooks | ✅ | ✅ | ✅ | ✅ |
| equivalent | ✅ | ✅ | ✅ | ✅ |
| parseColorSpaceFromString | ✅ | ✅ | ✅ | ✅ |

---

## Summary

### OCIO Parity: ~85%
- ✅ Core transforms: 100%
- ✅ Config loading: 100%
- ✅ Color spaces, roles, displays: 95%
- ⚠️ Python bindings: 70% (missing advanced features)
- ❌ Missing: BuiltinTransform, aliases, categories

### OIIO Parity: ~90%
- ✅ ColorConfig wrapper: 100%
- ✅ ImageBufAlgo OCIO functions: 95%
- ⚠️ Minor gaps in Python

### Priority Gaps to Close:
1. **Python**: Expose ConfigBuilder, Baker, DynamicProcessor, GpuProcessor
2. **OCIO**: Add BuiltinTransform support
3. **OCIO**: Add colorspace aliases and categories

---

Last updated: 2026-01-09
