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

<img src="images/damaged-helmet-look-vs-f3d.png" width="900" alt="Khronos Damaged Helmet rendered side by side by look and F3D 3.5">

Both panels above come from the runs tabulated here. Shading differs slightly
because F3D/VTK and `wgpu` do not share a material, sampling, and color
pipeline; the pixel-space error for this model is tabulated further down.

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

### Larger scenes

The same fresh-process harness was also run against two environment models.
These are separate workloads and are not folded into the six-model Khronos
geometric mean.

New York Boulevard contains 747,897 triangles, 650,951 compiled vertices, 11
draw instances, and two source textures. At 4096x4096, seven measured launches
produced:

| Renderer | Samples (ms) | Median |
|---|---|---:|
| look | 697.395, 887.887, 1029.659, 709.606, 1138.825, 684.565, 1146.297 | 887.887 ms |
| F3D 3.5 | 2321.472, 1769.127, 2391.241, 1770.464, 1765.476, 1785.720, 1776.994 | 1776.994 ms |

F3D / look was 2.001x. Foreground bounds were identical at
`[213, 1655, 3878, 2438]`. The asset declares `KHR_materials_unlit`; `look`
honors that extension while the F3D-compatible lighting path produces visibly
different shading, so this is a matched framing and output-work comparison,
not a claim of pixel identity. Linear-RGB RMSE over the foreground union was
0.0311 and alpha was identical.

Intel's Sponza Base Scene contains 3,747,018 triangles, 1,945,350 compiled
vertices, 405 draw instances, 29 materials, and 72 textures. The official
textures are 4096x4096. The comparison used a geometry-identical derivative
with each texture resized to 2048x2048 so both renderers fit reliably on this
laptop:

| Renderer | Samples (ms) | Median |
|---|---|---:|
| look | 2832.891, 3262.294, 2770.226 | 2832.891 ms |
| F3D 3.5 | 18220.546, 18233.087, 16625.062 | 18220.546 ms |

F3D / look was 6.432x at 512x512. This is mainly a scene import, texture, and
renderer initialization result: `look`'s measured GPU draw was about 1.1 ms.
The full 4K-texture package did not complete reliably in source-material mode
on this GPU, while technical mode succeeded because it intentionally skips
source texture decode and upload. The reduced derivative SHA-256 is
`b283303b7133df7ab6939229ac6492dcee3ee84ab9722041b2af5cf6bad79564`.

The New York asset hash reported by `look` was
`2238e481971910f248d66ea6796191d1dc4cb5f819940680b58760c270dd5235`.
The Sponza derivative source hash was
`e3e8fb573e1718cd5ea51b5ce948b040ca13fdcbcf735a6acb8dd20d1b7cb3f8`.

### STEP

STEP files are usually CAD boundary representations, so `look` evaluates and
tessellates them on load rather than reading triangles. These runs come from
the NIST MBE PMI validation set, which is freely redistributable and exported
by several commercial CAD systems, so it exercises real exporter behaviour
rather than synthetic geometry.

Fresh process, 512x512, median of five measured launches after one unmeasured
warm-up:

| Model | Size | Median | Triangles |
|---|---:|---:|---:|
| ctc_01 (AP203) | 0.22 MB | 686.0 ms | 1,200 |
| ftc_06 (AP203) | 0.21 MB | 659.8 ms | 2,537 |
| ctc_03 (AP242) | 0.64 MB | 663.2 ms | 1,164 |
| ctc_02 (AP242) | 1.89 MB | 687.7 ms | 6,122 |
| ftc_09 (AP242) | 5.83 MB | 643.5 ms | 1,766 |
| stc_09 (AP242) | 5.04 MB | 667.7 ms | 1,708 |

Runtime no longer tracks either file size or triangle count. It is flat within
noise from 0.21 MB to 5.83 MB, a 28x range of input, because every model now
finishes its CPU work before the GPU is ready and the render waits on adapter
and device creation. The phase breakdown shows the remaining CPU cost.

| Model | Parse | Table | Tessellate |
|---|---:|---:|---:|
| ftc_06 | 2.6 ms | 3.8 ms | 4.8 ms |
| ctc_02 | 20.6 ms | 20.3 ms | 26.6 ms |
| ftc_09 | 41.4 ms | 51.0 ms | 8.9 ms |
| stc_09 | 54.3 ms | 67.2 ms | 10.0 ms |

These three phases are reported separately from the totals above because a loop
that relaunches the same model back to back contends on the GPU adapter and
inflates the wait; the CPU phases are unaffected by that and are stable across
runs, while the totals come from `step-bench.ps1`, which warms up first.

Parsing used to be the whole story, reaching 88% of runtime on the largest file
at 729.9 ms for `stc_09`. It is now 54.3 ms. `look` reads the exchange
structure with its own Part 21 reader (`src/step/part21.rs`) rather than
ruststep's nom grammar: every token in this grammar is identifiable from its
first byte, so the reader dispatches on that byte and never backtracks, where
the nom `alt` chains reached a number or a `#` reference only after trying and
discarding the earlier alternatives, each of which allocated an error value on
the way down. Throughput on `stc_09` went from about 6.4 MB/s to about 80 MB/s.

The reader is held to producing exactly the syntax tree ruststep produces. The
`part21_agrees_with_ruststep_across_a_corpus` test compares both trees outright
over a corpus directory given in `LOOK_STEP_CORPUS`, and all 33 NIST files
match. Anything the reader turns down falls back to ruststep, so it can only
make parsing faster and never narrows what `look` accepts.

Tessellation was never the cost and still is not, staying between 5 and 27 ms.
Resolving the entity graph into a `Table` is now the same order of magnitude as
parsing, which makes it the next candidate rather than the parser.

The floor of roughly 650 ms is process launch plus adapter and device creation.
It is the whole runtime for every model in this set, so no further STEP-side
work will move these numbers; only cutting GPU initialization would.

#### Reading a model much larger than NIST

NIST tops out at 6 MB. The ABC dataset's Onshape exports reach 540 MB, which is
a different regime: there the binding constraint is memory rather than time.
`examples/step_table_scaling.rs` generates inputs differing only in entity count
and reports both, because on real files neither question can be answered
cleanly — file size is a poor proxy for entity count, the entity type mix varies
between files, and a model that does not fit produces the same curve as a
superlinear algorithm.

It shows that reading is linear. Across a 16x range of entity count, parsing
holds at 0.25 to 0.28 us per entity and table construction at 0.25 to 0.42 us.
That matches the code: `Table::from_data_section` is `from_iter` calling
`push_instance` once per entity, and `push_instance` is a flat match on the
record name doing one map insert. There is no superlinear term to find, and a
reading that appears to show one is measuring the machine.

Memory is the real limit. For a 104 MB input the syntax tree is 795 MB, about
eight times the file, and the table another 348 MB. Those used to be resident
simultaneously, because building a table from a borrowed data section keeps the
whole tree alive until it finishes. `look` now hands the tree over instead, so
each entity's storage is reclaimed as it is converted and the table is built
out of memory the tree has already released. Peak for that input fell from
1,334 MB to 1,010 MB, a 24% reduction, at the cost of about 9% on table
construction from the incremental drops.

One consequence is deliberate: an AP242 file that ships its mesh directly needs
the syntax tree after the table has been built and no longer has it, so those
files are read a second time. They are rare and small, and parsing is no longer
the expensive part of loading one — the re-read costs about 14 ms on the 2.1 MB
NIST tessellated model.

#### Running a corpus

`benchmarks/step_corpus.py` renders a directory of STEP files with either
`look` or F3D and reports outcomes, phase timings, peak memory, and any
warnings about incomplete geometry, then clusters failures by root cause. The
value of a corpus run is the set of distinct failure modes, not the count of
failing files.

```console
python benchmarks/step_corpus.py --tool look \
    --exe target/release/look.exe --dir <corpus> --out look.json
python benchmarks/step_corpus.py --tool f3d \
    --exe "C:/Program Files/F3D/bin/f3d-console.exe" --dir <corpus> --out f3d.json
python benchmarks/step_corpus.py --tool look --exe target/release/look.exe \
    --dir <corpus> --baseline look.json
```

It refuses to run when free physical memory is below `--min-free-gb`. That is
not fussiness: a short machine turns every timing into a measurement of the page
file, and Windows trims peak working set under pressure, so peaks recorded in
different machine states cannot be differenced. `--allow-low-memory` records
anyway and marks every sample untrusted, and a baseline comparison then declines
to compare timings at all.

Two further guards exist because their absence already produced wrong results.
Success is judged by whether an image was produced rather than by whether the
timeout fired, because under paging a wait can overshoot its budget by minutes
while the process still completes. And a POSIX-style path given to a Windows
executable is rejected rather than silently failing.

Run it with:

```powershell
cargo build --release
./benchmarks/step-bench.ps1 -Look target/release/look.exe -Nist <path-to-nist-files>
```

Coverage on that corpus is 33 of 33 files, spanning AP203, AP242 editions 1
through 3, and one model that ships an AP242 tessellated solid instead of a
boundary representation.

#### The regression corpus

Larger real models live in a separate repository,
[`look-corpus`](https://github.com/stefangolas/look-corpus), so they are not
cloned with every checkout of `look`:

| family      | file                        | size  | notes                              |
|-------------|-----------------------------|-------|------------------------------------|
| `ur10`      | `ur10/ur10.step`            | 28 MB | Universal Robots UR10 arm          |
| `formula1`  | `formula1/formula1.step`    | 44 MB | CFD step geometry, renamed Formula 1 |
| `core_xy`   | `core_xy/core_xy.step`      | 8.7 MB| CoreXY printer-style assembly      |
| `jackhammer`| `jackhammer/jackhammer.step`| 43 MB | Hydraulic jackhammer assembly      |
| `quadruped` | `quadruped/quadruped.step`  | 150 MB| Quadruped robot assembly           |
| `nist`      | `nist/NIST-PMI-STEP-Files/` | 54 MB | NIST MBE PMI set (33 files)        |

Set `LOOK_CORPUS` to a checkout of that repository when running the
external-corpus regression gate, and run the corpus like any other model:

```console
git clone https://github.com/stefangolas/look-corpus
$env:LOOK_CORPUS = "C:\path\to\look-corpus"
python benchmarks/step_corpus.py --tool look \
    --exe target/release/look.exe --dir $env:LOOK_CORPUS \
    --baseline benchmarks/corpus_regression_baseline.json
```

`benchmarks/corpus_regression_baseline.json` is the current build's recording
over the corpus (38 files): core_xy renders 668k triangles, formula1 renders
366k, jackhammer renders 1.29M, quadruped renders 2.16M, ur10 renders 502k, all
33 NIST files render, and every file completes within the timeout budget. The
baseline was
taken below the free-memory threshold, so its timings are untrusted; the
regression signals are the per-file `outcome` and `triangles`, which
`step_corpus.py --baseline` diffs against. Re-record the baseline deliberately
after a change you intend to keep, never to make a diff go away.

#### Against F3D

F3D reads STEP through its bundled OpenCASCADE plugin. It does not select that
reader automatically for a `.stp` file, so `--force-reader=STEP` is required.
Same settings as the fresh-process comparisons above, median of five measured
launches:

| Model | Size | look | F3D 3.5 | F3D / look |
|---|---:|---:|---:|---:|
| ctc_01 (AP203) | 0.22 MB | 696.1 ms | 693.0 ms | 1.00x |
| ftc_06 (AP203) | 0.21 MB | 667.4 ms | 695.3 ms | 1.04x |
| ctc_03 (AP242) | 0.64 MB | 681.3 ms | 694.0 ms | 1.02x |
| ctc_02 (AP242) | 1.89 MB | 679.4 ms | 1,412.1 ms | 2.08x |
| ftc_09 (AP242) | 5.83 MB | 730.3 ms | 957.7 ms | 1.31x |
| stc_09 (AP242) | 5.04 MB | 706.9 ms | 989.6 ms | 1.40x |

The geometric mean is 1.26x. The previous measurement of this table was 1.08x,
parity, taken when parsing ISO 10303-21 text dominated STEP runtime and left a
rendering advantage no room to show. With the Part 21 reader above, the two
files where F3D was level or ahead have moved: `ftc_09` from 1.00x to 1.31x and
`stc_09` from 0.87x to 1.40x.

The three small models sit at parity and cannot leave it. Both tools are pinned
to their process and GPU initialization floor there, and on `ctc_01` that floor
is 693 ms of F3D's 693 ms. The comparison only has room to separate on files
large enough to do measurable work, which in this corpus means `ctc_02` and up.

Both tools measured about 10% slower in this run than in the previous one on
the same machine — F3D's `ctc_01` went from 631.6 ms to 693.0 ms — so the
ratios carry the result and the absolute milliseconds should not be compared
across the two runs. The whole STEP section was re-measured together for that
reason rather than having the `look` column updated in place.

Run it with:

```powershell
./benchmarks/step-vs-f3d.ps1 -Look target/release/look.exe -Nist <path-to-nist-files>
```

#### Foliage stress scene

The official Sponza Base, Ivy, and Trees packages were merged into one GLB by
`benchmarks/merge-gltf-scenes.py`. The composite contains 10,836,323 triangles,
6,928,589 compiled vertices, 411 draw instances, 348 geometries, 34 materials,
and 79 textures. Base textures were resized to 2048x2048; the foliage add-on
textures remain at their official resolution.

At 512x512, after one conditioning launch and with alternating launch order:

| Renderer | Samples (ms) | Median |
|---|---|---:|
| look | 36244.279, 8491.023, 7533.588 | 8491.023 ms |
| F3D 3.5 | 52770.050, 46137.207, 50466.004 | 50466.004 ms |

F3D / look was 5.943x. The first `look` sample was a large outlier and is
retained above. Foreground bounds matched at `[26, 130, 486, 382]`;
linear-RGB RMSE over the foreground union was 0.0166 and alpha was identical.
The combined source hash reported by `look` was
`1c4c0098e9dbf5b21955a81d21b0e0bf1bfd4df9e0801c908b2a4183045c47f5`.

This scene also exposed and fixed a real scale limit: `look` had requested
wgpu's portable 256 MiB maximum buffer size even when the physical adapter
supported larger buffers. It now requests the adapter's native buffer-size
limit and reports an ordinary error if a packed scene buffer still exceeds it,
instead of allowing a GPU validation panic.

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

## Resident camera path

`examples/camera_path_benchmark.rs` moves a perspective camera through the
interior of a resident scene, reads every frame back, and writes four sampled
keyframes as an atlas. Set `LOOK_GPU_TIMESTAMPS=1` to separate GPU execution
from submission and readback:

```console
LOOK_GPU_TIMESTAMPS=1 cargo run --release --example camera_path_benchmark -- \
  target/bench/models/Sponza-2k.glb 120 1024x1024
```

On base Sponza, the retained 120-frame 1024x1024 path measured 1.587 ms median
GPU time (1.724 ms p95) and 4.999 ms median wall time including readback. The
retained 30-frame 4096x4096 path with `--antialias` measured 4.661 ms median GPU
time (6.337 ms p95) and 26.767 ms median wall time.

Adding Ivy and Trees made the same 30-frame 4K, 4x-MSAA workload materially more
demanding: 7.580 ms median GPU time, 10.039 ms p95, and 37.330 ms median wall
time. A resident four-view 512px atlas measured 12.232 ms median GPU time.
Three.js WebGL2 measured 9.9 ms for its GPU-finish interval on the same atlas
and confirmed ANGLE D3D11 on the same RTX 5050; `look` therefore does not claim
a pure-GPU win for that workload. End-to-end atlas medians were 38.05 ms for
`look` and 42.5 ms for Three.js. Three.js WebGPU measured 17.0 ms, but Chrome
withheld adapter identity, so it is not a proven same-adapter comparison.

Atlas scaling provides a separate saturation lane. With 512px tiles and 4x
MSAA, median GPU time was 9.525 ms for 8 views, 19.108 ms for 16, 38.850 ms for
32, and 80.837 ms for 64. The 64-view pass submits 25,920 draws and transforms
about 239.8 million triangles.

## Rules for publishable results

- Record the `look` commit, model hash, OS, GPU, driver, backend, and power mode.
- Benchmark a release build and retain all raw samples, not only the median.
- Alternate competitors when shared machine state can bias launch order.
- Do not run other GPU-heavy workloads during a performance measurement.
- Hosted, virtual, partitioned, or software GPUs are correctness lanes only.
- Publish timing claims only from a physical machine with a recorded adapter.
- Compare outputs as well as speed. A faster render with materially different
  framing, alpha, or color is not a valid win.
