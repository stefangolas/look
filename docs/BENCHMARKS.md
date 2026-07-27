# Benchmarks

Benchmark claims are separated by lifecycle. F3D is measured as a fresh
executable because that is its normal screenshot path. Three.js and `look`
sessions are measured after model parsing, shader creation, and GPU upload
because they represent repeated inspection.

These measurements were collected on one Windows laptop with an NVIDIA GeForce
RTX 5050 Laptop GPU. They establish local wins and identify bottlenecks; they do
not predict every machine.

## Fresh process: look and F3D 3.5

`benchmarks/compare-pbr.ps1` launches a new process for every sample. It performs
one unmeasured conditioning launch per tool, alternates launch order, disables
F3D user configuration, and measures process start through a completed PNG.

Shared settings:

- 512x512 output
- front orthographic automatic fit
- glTF source PBR materials
- antialiasing, ambient occlusion, and tone mapping disabled
- `#252525` background
- seven measured launches per model
- `look --preset f3d-match`

| Khronos glTF sample | look median | F3D median | F3D / look |
|---|---:|---:|---:|
| Box | 531.751 ms | 588.282 ms | 1.106x |
| Damaged Helmet | 602.276 ms | 939.075 ms | 1.559x |
| Avocado | 538.785 ms | 846.856 ms | 1.572x |
| BoomBox | 564.027 ms | 876.118 ms | 1.553x |
| Metal-Rough Spheres | 560.613 ms | 716.991 ms | 1.279x |
| Normal/Tangent Test | 567.921 ms | 795.104 ms | 1.400x |

The geometric-mean speedup is 1.40x. The cold path is dominated by operating
system process launch and GPU adapter/device initialization. Actual GPU draw
time is a small fraction, so low-level shader improvements cannot remove most
of this latency.

The raw report is generated at `target/bench/pbr/benchmark.json`. Model SHA-256
prefixes for this run were `ed52f719` (Box), `a1e3b04d` (Helmet), `ccc9c3ce`
(Avocado), `f8b91844` (BoomBox), `450c0555` (Spheres), and `5ac09323`
(Normal/Tangent).

Run it with:

```powershell
cargo build --release
./benchmarks/compare-pbr.ps1
```

## F3D visual compatibility

The F3D match profile is pinned to F3D 3.5's VTK camera framing and default
five-light `vtkLightKit`, including its camera-relative key, fill, head, and two
back lights. With clean F3D configuration, the measured object bounding-box
difference was 0.00% to 0.45% across all six fixtures. Alpha matched for these
opaque models.

Linear-RGB RMSE after common framing was:

| Model | RMSE |
|---|---:|
| Box | 0.0099 |
| Damaged Helmet | 0.0430 |
| Avocado | 0.0063 |
| BoomBox | 0.0271 |
| Metal-Rough Spheres | 0.0289 |
| Normal/Tangent Test | 0.0239 |

Box and Avocado are near a 1% pixel-space error. The textured and strongly PBR
fixtures differ more because F3D/VTK and `wgpu` do not use identical material,
sampling, and color pipelines. Use a perceptual metric and an explicit
tolerance for regression testing; do not require byte-identical PNGs.

## Resident scene: look and Three.js

`examples/session_benchmark.rs` loads Damaged Helmet once, decodes textures,
creates the device and pipelines, uploads the model, and performs a 1x1 warm
render. Eleven measured iterations then include render submission, readback,
PNG encoding, and output-file replacement.

The Three.js harness uses Three.js 0.185.1, persistent headless Chrome 150,
the same model, cameras, bounds fit, background, source PBR materials, no AA,
an explicit GPU finish, and canvas PNG encoding. Browser launch is excluded.
The browser results do not include writing the encoded PNG to disk, making the
comparison conservative for `look`.

| Workload | look | Three.js WebGL2 | Three.js WebGPU | look vs WebGL2 | look vs WebGPU |
|---|---:|---:|---:|---:|---:|
| One 512x512 view | 3.459 ms | 7.600 ms | 13.500 ms | 2.20x | 3.90x |
| Four-view 2x2 atlas | 8.940 ms | 22.500 ms | 27.300 ms | 2.52x | 3.05x |

`look` setup was 717.687 ms. Its last atlas sample spent 0.213 ms submitting GPU
work and 1.915 ms reading back; PNG encoding and file output dominate the
resident path. This is the current Amdahl limit and the best optimization target
for repeated screenshots.

Chrome WebGL2 reported ANGLE on the same RTX 5050. Chrome's WebGPU adapter-info
object was empty because of browser privacy controls. The harness requested
WebGPU, verified that Three.js resolved WebGPU rather than falling back, and
launched Chrome with its high-performance-GPU preference, but the WebGPU result
must not be described as a proven same-adapter comparison.

Run the native resident benchmark:

```console
cargo run --release --example session_benchmark -- target/bench/models/DamagedHelmet.glb 11
```

Run the browser harness:

```console
cd benchmarks/threejs
npm install
npm run bench:webgl -- ../../target/bench/models/DamagedHelmet.glb 11 webgl2
npm run bench:webgpu
```

Reports are written beneath `target/bench/threejs`.

## Rules for publishable results

- Record the `look` commit, model hash, OS, GPU, driver, backend, and power mode.
- Benchmark a release build and retain all raw samples, not only the median.
- Alternate competitors when shared machine state can bias launch order.
- Do not run other GPU-heavy workloads during a performance measurement.
- Hosted, virtual, partitioned, or software GPUs are correctness lanes only.
- Publish timing claims only from a physical machine with a recorded adapter.
- Compare outputs as well as speed. A faster render with materially different
  framing, alpha, or color is not a valid win.
