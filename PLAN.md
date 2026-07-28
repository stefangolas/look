# Plan

## Design intent

`look` is a native command-line utility that turns GLB, STL, and STEP models
into PNG images, optimized for time to a usable image. It exists so a person, a
script, or an agent can inspect a 3D model without a CAD application or a
browser.

The working method is autonomous improvement against measurement: find where
the tool is slow or wrong on real inputs, fix the largest thing, prove the fix
with numbers, and keep the hot path small. A change is only finished when it is
measured, tested, and either published or explicitly recorded as unverified.

The product boundary in `AGENTS.md` still governs. This is a renderer, not an
embeddable CAD framework.

## Where the effort currently goes

STEP, because it is where the tool was weakest and where real files are least
like the curated test corpus.

`look` reads STEP with its own ISO 10303-21 reader (`src/step/part21.rs`),
resolves the entity graph through a fork of `truck-stepio`, tessellates through
`truck-meshalgo`, and renders with `wgpu`. Two forks are pinned by exact
revision: `stefangolas/truck` and `stefangolas/ruststep`.

## What is done

- **Part 21 reader.** Dispatches on each token's first byte and never
  backtracks. Parse of the largest NIST model went 729.9 ms to 54.3 ms; on
  300 MB files it sustains 140–250 MB/s. Falls back to ruststep on anything it
  declines, so it cannot narrow coverage. Equality with ruststep is tested, not
  assumed.
- **Memory handoff.** The syntax tree is given to the table rather than lent to
  it, so both are not resident at once. Peak fell 24% on a controlled
  measurement.
- **Six correctness fixes** (see the failure taxonomy below), taking the ten
  largest ABC models from three rendering to eight.
- **`OFFSET_SURFACE` is read.** It had no arm in the entity table, so a quarter
  of real Onshape files silently lost every face that used one. Two ABC models
  went from 246 and 205 unresolved references to none.
- **Tessellation cannot run away.** Curve and surface division were bounded only
  by recursion depth while each level doubles. A 102 MB model that died at
  5.16 GB now renders at 990 MB.
- **Face loss is now reported** instead of silent.
- **A corpus harness that refuses to produce bad numbers**, in
  `benchmarks/step_corpus.py`.

## The failure taxonomy

Every failure found on real files has been downstream of the parser.

| pipeline stage | failures |
|---|---:|
| text to syntax tree | 0 |
| syntax tree to entity table | 0 |
| entity table to geometry | 2 |
| geometry to triangles | 3 |
| triangles to PNG | 0 |

Four of the five were the *same defect*: a fallible operation panicking instead
of returning the error its own signature promises — `unwrap()` inside a
`TryFrom` returning `Result`, indexing inside a `try_new` returning `Option`,
an `assert!` where coarse meshing would do. Each was made fatal rather than
local by `panic = "abort"` in the release profile combined with rayon workers,
so one bad face ended a whole model.

The corpus matters more than the count. NIST is 33 curated files, uniformly
millimetre-scale AP203/AP242. ABC is real Onshape output: metre-scale AP214, up
to 540 MB. Every bug above was invisible to NIST by construction.

## Method: audit the path before measuring it

Read the whole relevant code path end to end and list the runtime complexity,
storage complexity, and outright bugs, *before* reaching for a benchmark. This
is not a nicety. Nearly every real finding so far came from reading:

- `push_instance` is a flat 63-arm match doing one map insert each, which ruled
  out the superlinear table cost that two rounds of timing had suggested.
- Curve and surface subdivision were bounded only by recursion depth, and both
  double per level, so the worst case is exponential. That was the
  out-of-memory, and it was visible in twenty lines of code.
- `OFFSET_SURFACE` simply has no arm, which no amount of timing would reveal.

The timing runs, by contrast, mostly produced numbers that had to be caveated
because the machine was short of memory. Measure to confirm a hypothesis or to
size a win, not to find one.

The audit covers: `src/step.rs`, `src/step/part21.rs`, `src/scene.rs`, the
`truck-stepio` entity path, and the `truck-geotrait` tessellation algorithms.

### Audit findings

Fixed:

- `OFFSET_SURFACE` was not read at all.
- Curve and surface division had no bound on output size, only on depth.
- `append_polygon` grew two multi-megabyte vectors from empty when the final
  size is known, holding both allocations live at every doubling.
- `generate_normals` collected into a second full-size buffer to normalize.
- `compile_triangle_mesh` copied every position into a temporary purely to
  measure the bounding box.

Open, in rough order of size:

- **The STEP index buffer is the identity permutation.** `append_polygon`
  emits `indices[i] == i` for every vertex, so it is four bytes per vertex of
  pure redundancy — 22 MB on a 1.9 M-triangle model. Eliding it needs a
  non-indexed draw path in the renderer.
- **Vertices are unwelded**, three per triangle, so a shared corner is stored
  as many times as it is used. That is deliberate, to keep flat per-face
  normals, but welding on the pair of position *and* normal would preserve
  creases and still collapse the flat regions.
- **`source_attributes` allocates a constant.** When source materials are on,
  it fills one vector entry per vertex with the same default value, roughly
  40 bytes per vertex, for triangle-soup sources that have no such attributes.
- **The entity table builds around sixty typed maps** whether or not the model
  needs them; presentation and styling entities are a measurable share of a
  real file and are never read by the render path.

## Near-term goals, in order

1. **Finish the audit.** It has paid for itself every time it has been run and
   has not yet covered `src/step/part21.rs`, `src/renderer/`, or the session
   path. The open storage items below came out of a partial pass.
2. **Elide the STEP index buffer.** It is the identity permutation, four bytes
   per vertex of pure redundancy, 22 MB on a 1.9 M-triangle model. Needs a
   non-indexed draw path in the renderer.
3. **Weld vertices on position and normal.** Three vertices per triangle today;
   welding on the pair keeps flat faces flat and creases sharp while collapsing
   the interior of every planar region.
4. **Find why some faces still produce no geometry.** `OFFSET_SURFACE` removed
   every unresolved reference on the files that had them, yet 135 of 7,605 and
   753 of 12,027 faces still mesh to nothing. That is now the largest known
   correctness gap and its cause is unknown.
5. **Two models still exceed five minutes**, `00000414` and `00005641`, and
   `00009190` renders in 230 s. The subdivision bounds stopped the crash but
   not the cost. Suspect the same pathological geometry.
6. **Decide on `panic = "abort"`.** `parse_step` collects per-shell failures and
   warns, and that code cannot run today. This is a judgement call for the
   owner.
7. **Shrink the syntax tree**, roughly eight times the file and the largest
   remaining memory consumer. Means changing ruststep's `Parameter`
   representation, so a deliberate project.

## How results are reported

- Benchmark release builds, retain raw samples and the hardware fingerprint.
- Fresh-process and resident-session numbers answer different questions and are
  never combined.
- Comparisons against F3D use `--force-reader=STEP` for STEP, explicit camera,
  resolution and background, and alternating launch order.
- A measurement taken while the machine is short of memory or disk is not a
  measurement. Peak memory and wall time are not comparable across runs whose
  machine state differs; say so rather than quoting the delta.
- Never pass a POSIX-style path to a Windows executable. It fails silently and
  has already invalidated two measurement runs.

See `docs/BENCHMARKS.md` for numbers and methodology, `docs/ARCHITECTURE.md` for
structure, and `AGENTS.md` for repository invariants.
