# look

`look` is a native command-line utility that turns GLB, STL, and STEP models
into PNG images. Its basic purpose is to let a person, script, or software agent
inspect a 3D model without opening a full CAD application or browser-based
viewer.

STEP files are CAD boundary representations rather than meshes, so `look`
tessellates them on load. That happens inside the binary — there is nothing to
install and no converter to configure:

```console
look part.step --output part.png
```

<img src="docs/images/damaged-helmet-look-vs-f3d.png" width="640" alt="Khronos Damaged Helmet rendered side by side by look and F3D 3.5">

`look` is faster than F3D 3.5 across every scene measured, by 1.4x on typical
glTF samples and up to 5.9x on large ones. See [benchmarks](docs/BENCHMARKS.md)
for the numbers, raw samples, image licenses, and methodology.

## Quick example

Render an automatically framed isometric view:

```console
look model.glb --output render.png
```

That command loads `model.glb`, selects a technical material and lighting setup,
fits the camera to the model, renders an isometric view, and writes `render.png`.
No configuration file is required.

Render four camera views directly into one GPU-generated PNG:

```console
look model.glb --material-mode source \
  --views front,right,top,iso --camera orthographic \
  --resolution 384x384 --atlas 2 --output views.png --json
```

The atlas path draws every view into one render target in a single GPU pass,
then does one readback and one PNG, rather than paying per-view capture
overhead.

`look` can also:

- render named camera views with controllable lighting and materials;
- combine several views into one atlas image for efficient agent inspection;
- report mesh, triangle, material, texture, and bounds metadata as JSON;
- execute repeatable YAML render jobs; and
- keep a scene GPU-resident for multiple inspection passes.

It uses Rust and `wgpu` directly and emits machine-readable results for agent
workflows. There is no browser, Electron shell, VTK scene layer, or import
framework on the hot path.

## Command help

Yes, every command has built-in help:

```console
look --help
look render --help
look persist --help
look --version
```

`look help render` is equivalent to `look render --help`.

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

## Included test model

The repository includes `tests/fixtures/triangle.stl`, a 162-byte ASCII STL
used for CLI, package, parser, and render smoke tests:

```console
look tests/fixtures/triangle.stl --output triangle.png --json
```

Larger Khronos models are benchmark inputs rather than fixtures, and are kept
out of the repository to avoid bloating clones and redistributing third-party
assets without their license records.

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

Fresh-process, against F3D 3.5: 1.40x geometric mean on six glTF samples, 2.00x
on New York Boulevard at 4K, and 5.94x on the 10.8M-triangle Sponza foliage
composite. On STEP, 1.26x against F3D's OpenCASCADE reader.

Resident, against Three.js at 512 px tiles: a four-view atlas takes 8.94 ms in
`look` versus 22.50 ms in WebGL2 and 27.30 ms in WebGPU.

These are local results on one NVIDIA RTX 5050 Laptop GPU, not universal
guarantees. Commands, raw samples, fidelity metrics, hardware fingerprints, and
methodological limits are in [benchmarks](docs/BENCHMARKS.md).

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
