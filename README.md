# look

`look` is a small native executable for rendering GLB and STL files quickly. It
uses Rust and `wgpu` directly, supports deterministic camera and lighting
configuration, and emits machine-readable results for agent workflows.

```console
look model.glb --view iso --output render.png
```

There is no browser, Electron shell, VTK scene layer, or import framework on the
hot path. GLB and STL parsing, scene compilation, GPU upload, rendering, and PNG
output live in one executable.

## Install

Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/stefangolas/look/main/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/stefangolas/look/main/install.ps1 | iex
```

Release archives and Debian packages are also attached to each GitHub release.
See [installation](docs/INSTALLATION.md) for version pinning and source builds.

## Render

The `render` subcommand is implied when the first argument is a model path:

```console
look model.glb --view iso --resolution 1024x1024 --output render.png
look model.glb --material-mode source --antialias --output pbr.png
look bracket.stl --view front --camera orthographic --output bracket.png
look model.glb --views front,right,top,iso --output-dir renders --json
```

Pack multiple views into one GPU-rendered PNG atlas. The resolution is the size
of each tile, not the whole atlas:

```console
look model.glb --views front,right,top,iso --atlas 2 --output views.png --json
```

Use the F3D 3.5/VTK compatibility profile when comparing the two renderers:

```console
look model.glb --preset f3d-match --view front --camera orthographic --output matched.png
```

The profile matches F3D's default camera framing, background, and five-light
kit. It is a compatibility preset, not a promise of pixel identity for every
PBR material.

## Reuse a loaded scene

Repeated inspection should avoid parsing, texture decoding, shader creation,
and GPU upload. `persist` starts a private loopback server, warms the scene, and
returns a session ID:

```console
look persist assembly.glb --material-mode source --ttl 600 --json
look render --session SESSION_ID --views front,right,top,iso --atlas 2 --output views.png --json
look inspect --session SESSION_ID --json
look close SESSION_ID --json
```

Use `look sessions --json`, `look server status --json`, and
`look server stop --json` to manage the local server. Sessions expire after the
configured idle lifetime. The server binds only to loopback and authenticates
requests with a per-process token stored in the user's cache directory.

## YAML jobs

```console
look run examples/technical.yaml --json
```

```yaml
version: 1
scene:
  source: assembly.glb
  up_axis: y
render:
  resolution: [1024, 1024]
  material_mode: technical
  background: "#252525"
  antialias: false
  atlas_columns: 2
lighting:
  preset: technical
  ambient: 0.35
  direction: [-1, -2, -3]
  intensity: 0.85
  color: "#ffffff"
views:
  - id: front
    type: orthographic
    direction: [0, 0, 1]
  - id: iso
    type: perspective
    direction: [1, 1, 1]
output:
  directory: renders
  naming: "{view}.png"
```

Unknown YAML fields are rejected instead of silently ignored.

## Current scope

- Embedded-buffer GLB/glTF 2.0 and binary or ASCII STL
- Technical material mode and glTF metallic-roughness source materials
- Base-color, metallic-roughness, normal, emissive, and occlusion textures
- Alpha modes, UV0/UV1, vertex colors, instancing, and geometry deduplication
- Perspective and orthographic bounds-fit cameras with seven named views
- Configurable technical lighting and an F3D 3.5 compatibility light kit
- Multi-view PNGs and single-pass GPU atlas rendering
- Content-validated scene metadata cache and bounded GPU-resident session cache
- DirectX 12, Vulkan, Metal, and browser WebGPU-class portability through `wgpu`

This is deliberately a renderer, not an embeddable CAD framework or general
scene editor.

## Measured performance

Local measurements used an NVIDIA RTX 5050 Laptop GPU, 512 px tiles, source PBR
materials, and 11 resident samples. Lower is better.

| Resident workload | look | Three.js WebGL2 | Three.js WebGPU |
|---|---:|---:|---:|
| One view, render + PNG | 3.46 ms | 7.60 ms | 13.50 ms |
| Four-view 2x2 atlas, render + PNG | 8.94 ms | 22.50 ms | 27.30 ms |

The `look` measurements include replacing the output file; the browser numbers
stop after PNG encoding. Chrome WebGL2 and `look` reported the RTX adapter.
Chrome withheld WebGPU adapter identity, so that row is informative rather than
a same-adapter performance claim.

Against clean-config F3D 3.5 fresh processes on six glTF sample models, `look`
was 1.11x to 1.57x faster with a 1.40x geometric-mean speedup. These are local
results, not universal guarantees. Full commands, model hashes, fidelity
metrics, and methodological limits are in [benchmarks](docs/BENCHMARKS.md).

## Development

```console
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo test --release --test gpu_smoke -- --ignored --nocapture --test-threads=1
```

See [architecture](docs/ARCHITECTURE.md), [cross-platform testing](docs/CROSS_PLATFORM_TESTING.md),
and [AGENTS.md](AGENTS.md). GitHub Actions builds and smoke-tests release
binaries for Linux, Windows, and macOS on x64 and ARM64.

Licensed under MIT or Apache-2.0.
