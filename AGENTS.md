# Agent guide

## Product boundary

`look` is a native GLB/STL screenshot executable optimized for time to a usable
image. Keep the hot path small. Do not add browser, GUI, plugin, conversion, or
general scene-framework dependencies unless a measured user requirement needs
them.

## Use look efficiently

For one render:

```console
look model.glb --view iso --output render.png --json
```

For multiple views, prefer one atlas. `--resolution` is per tile:

```console
look model.glb --views front,right,top,iso --atlas 2 --resolution 512x512 --output views.png --json
```

For repeated inspection, persist once and reuse the returned session ID:

```console
look persist model.glb --material-mode source --ttl 600 --json
look render --session SESSION_ID --views front,right,iso --atlas 2 --output views.png --json
look inspect --session SESSION_ID --json
look close SESSION_ID --json
```

Use `technical` material mode when source textures are irrelevant. It skips
texture decode/upload and uses the compact vertex path. Use `source` when PBR
fidelity matters. Use `--preset f3d-match` only for F3D 3.5 compatibility.

Prefer `--json` and parse fields rather than human output. Inspect geometry with
`look inspect MODEL --json`; this command does not initialize the GPU and uses a
validated metadata cache on repeated calls.

## Repository invariants

- CLI name, crate, executable, environment variables, cache paths, and docs use
  `look` consistently.
- CLI flags and YAML fields map to the same normalized configuration types.
- One-shot and session rendering use the same scene compiler and renderer.
- Bounds fitting, named views, traversal, lighting presets, and atlas tile order
  remain deterministic.
- Technical mode must not decode or upload source textures.
- Repeated views must reuse parsed scenes, pipelines, GPU buffers, and targets.
- Unknown YAML fields fail validation.
- Errors returned to agents should remain nonzero and machine-readable where a
  command exposes `--json`.

## Verification

Run before committing:

```console
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo test --locked --all-targets
```

On a machine with a native GPU, also run the ignored tests serially because the
session test mutates process cache state:

```console
cargo test --release --test gpu_smoke -- --ignored --nocapture --test-threads=1
```

Do not update golden images or performance claims solely to make a test pass.
First explain changes in framing, color, alpha, output dimensions, or adapter.

## Performance work

Benchmark release builds. Retain raw samples and the hardware fingerprint from
`look doctor --json`. Fresh-process and resident-session measurements answer
different questions and must not be combined.

When comparing F3D, include `--no-config`, make camera/resolution/background and
effects explicit, alternate launch order, and compare output fidelity. When
comparing Three.js, verify the resolved WebGL/WebGPU backend and whether adapter
identity is actually observable.

Use physical recorded hardware for published latency claims. Hosted virtual,
partitioned, and software GPUs provide correctness evidence only. See
`docs/BENCHMARKS.md` and `docs/CROSS_PLATFORM_TESTING.md`.

Optimize from timings and Amdahl's law. In the resident path PNG encoding and
file output currently dominate; GPU draw submission is already sub-millisecond
for the benchmark atlas. Validate any low-level change end to end rather than
assuming fewer API calls improve wall time.

## Releases

The release workflow builds Linux, Windows, and macOS packages on x64 and ARM64,
runs tests and packaged-binary smoke checks, and emits SHA-256 files. Do not tag
a release until the branch CI and a manual release-matrix run pass. Keep install
asset names synchronized with `install.sh` and `install.ps1`.

Do not publish benchmark numbers from GitHub-hosted GPU runners. Do not change
repository visibility, create a public release, or add signing credentials
without explicit owner approval.
