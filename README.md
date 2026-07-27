# v3

`v3` is a small native renderer for producing deterministic views of GLB and STL
scenes. Technical mode skips source-material work completely; source mode uses
glTF metallic-roughness materials, textures, normal maps, emissive, occlusion,
alpha modes, and vertex colors. Equal geometry payloads are deduplicated and a
renderer session reuses uploaded scenes, pipelines, and render resources.

## Commands

```powershell
v3 render model.glb --view iso --output render.png
v3 render model.glb --material-mode source --output source.png
v3 render bracket.stl --view front --camera orthographic --output bracket.png
v3 render model.glb --views front,right,top,iso --output-dir renders
v3 run examples/technical.yaml
v3 inspect model.glb --json
v3 doctor --json
```

If the first argument is a model path, `render` is implied:

```powershell
v3 model.glb --view iso --output render.png
```

## Current boundary

- GLB with embedded geometry and texture buffers
- Binary and ASCII STL through a direct dependency-free parser
- Technical and glTF source-material modes
- Static triangle primitives, instancing, and equal-geometry deduplication
- Position, normal, UV0/UV1, and vertex-color attributes
- Perspective and orthographic cameras
- Automatic bounds fitting
- Ambient plus one directional light
- One or more PNG outputs
- Hardware fingerprint and per-stage timings

The renderer interface is deliberately backend-neutral. A native D3D12,
Vulkan, or Metal implementation can be added without changing scene loading,
configuration, output, or cache formats.

## Benchmarks and platform testing

`benchmarks/compare.ps1` compares fresh-process `v3` and F3D renders with equal
resolution, camera class, antialiasing, and background settings. Results and
images are written beneath `target/bench/outputs`.

Cross-platform unit CI and an on-demand/weekly Apple-silicon Metal correctness
job live in `.github/workflows`. See `docs/CROSS_PLATFORM_TESTING.md` for the
bare-metal performance, virtual GPU correctness, software rendering, and driver
matrix policy.
