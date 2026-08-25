# Agent guide

## Product boundary

`look` is a native GLB/STL screenshot executable optimized for time to a usable
image. Keep the hot path small. Do not add browser, GUI, plugin, conversion, or
general scene-framework dependencies unless a measured user requirement needs
them.

## Kernel solver program

The base kernel loop is FINISHED (76/76) and BG-AUDIT-001 closed (17/17). The
next program is the certified solver family: `docs/SOLVER_FAMILY_PLAN.md` is
the approved design and books the existing `truck-evidence`/`truck-shapeops`/
`truck-geometry`/`truck-topology` API surface with real signatures so fresh
packets do not re-explore it. Kernel code changes only through the packet /
worker / `verify.py` loop (see `loop/ORCHESTRATOR.md`); `vendor/truck/**` is
off-limits to direct editing. Solver-family packets are written against the
plan doc's §3 booked API and §5 parallel graph.

## Use look efficiently

For one render:

```console
look model.glb --view iso --output render.png --json
```

For one STEP render, use the same flags; the STEP boundary representation is
tessellated on load inside the binary:

```console
look core_xy.step --view iso --output render.png --json
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

## Build modes: fast inner loop vs authoritative release

There are two build modes and they answer different questions.

**Fast inner loop** for edit-rebuild-run cycles. Uses the `quick` profile, which
keeps dev semantics/assertions, optimizes at `opt-level = 2`, disables LTO, uses
`codegen-units = 256`, and turns on incremental compilation. Artifacts land in
`target/quick/`, separate from release.

```console
cargo qcheck                      # check --profile quick --locked
cargo qbuild                      # build --profile quick --locked
cargo qtest                       # test --profile quick --tests --locked
cargo test --profile quick --lib <filter> --locked   # narrow kernel test
```

`qtest` uses `--tests`, so it does not compile the `examples/` probe collection
on every dev-test run.

**Authoritative release** for measured performance and regression work. Its
profile (`lto="thin"`, `codegen-units=1`) is fixed; never weaken it.

```console
cargo build --release --locked
```

`target/quick/look.exe` must never be used for recorded performance numbers.
Quick runtime is not comparable to release runtime; the two artifact paths make
a mix-up visible, but only if you keep pointing scripts at `target/release/`.

On this machine the active host is `x86_64-pc-windows-gnullvm`. Ordinary native
builds must not pass `--target x86_64-pc-windows-gnullvm`: it is the host, and
specifying it creates a separate Cargo target-artifact tree and doubles
proc-macro compilation.

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

### STEP vs OCCT

The `core_xy.step` assembly in the home directory is a local reference model
(9.1 MB, 5,670 faces). On this machine `look` renders it fresh in about 2.0 s at
512x512 versus about 6.4 s for OCCT through F3D 3.5 (`--force-reader=STEP`), a
3.2x gap, because the Part 21 parser and tessellation complete well inside the
process and adapter startup floor. Do not quote that ratio as a universal STEP
claim: it is one assembly on one GPU.

To re-measure against OCCT, use the exact commands in the README's
"Reproducing the core_xy STEP comparison" section rather than improvising: one
unmeasured conditioning launch per tool, alternating launch order, median of
five measured launches, `look render --preset f3d-match` versus
`f3d-console --no-config --force-reader=STEP`, 512x512 front orthographic,
no AA/AO/tone-mapping, `#252525` background, and success judged by whether a PNG
was produced. `benchmarks/step-vs-f3d.ps1` exists for the NIST corpus and is
not wired to `core_xy.step`. F3D reads STEP through its bundled OpenCASCADE
plugin, so this compares whole pipelines (parse, tessellate, render). Confirm
the F3D PNG contains the model — the F3D GUI start page can make a mis-served
file look like a successful render.

Verify output fidelity alongside speed, not just timing: compare foreground
bounding boxes and luminance coverage on the produced PNGs so a blank or
mis-framed OCCT render cannot be reported as a win.

## Releases

The release workflow builds Linux, Windows, and macOS packages on x64 and ARM64,
runs tests and packaged-binary smoke checks, and emits SHA-256 files. Do not tag
a release until the branch CI and a manual release-matrix run pass. Keep install
asset names synchronized with `install.sh` and `install.ps1`.

Do not publish benchmark numbers from GitHub-hosted GPU runners. Do not change
repository visibility, create a public release, or add signing credentials
without explicit owner approval.
