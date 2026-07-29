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

**Scope decision, 2026-07-29: the STEP path is becoming a certified geometry
kernel.** `AGENTS.md` previously drew the boundary at "a renderer, not an
embeddable CAD framework". That boundary has been deliberately moved. See
§"Why the architecture changed" and the roadmap below. The renderer product
goal is unchanged; the ingestion layer beneath it is being rebuilt.

## Where the effort goes

STEP. `look` reads it with its own ISO 10303-21 reader (`src/step/part21.rs`),
resolves the entity graph through a fork of `truck-stepio`, tessellates through
`truck-meshalgo`, and renders with `wgpu`.

Two forks are pinned by exact revision: `stefangolas/truck` and
`stefangolas/ruststep`.

> **Fork state, unresolved.** `Cargo.toml` currently carries a temporary
> `[patch."https://github.com/stefangolas/truck"]` block pointing at
> `../truck-fork`. **Thirteen commits sit in that local clone, unpushed.**
> Nothing is pinned. Before any release: push the fork, pin the rev, delete the
> patch block. A strategic decision is also outstanding — whether these changes
> go upstream to `ricosjp/truck` (which publishes roughly every two years) or
> `stefangolas/truck` becomes a permanent architectural fork.

---

## Current state of the corpus

All 20 ABC models load and render without error. **Rendering correctly is a
different question**, and the failure is silent — nothing errors, the picture is
just wrong.

Blob shells on `00009190` (577 shells, 24,202 faces), by fix:

| state | blob shells | note |
|---|---:|---|
| session start | 70 | renders as undifferentiated lens blobs |
| after containment fix | 12 | |
| after periodic-lift fix | 4 | renders as a recognizable submarine |
| after `eidx_map` fix | 3 | |
| after disabling `surface.invert()` | 2 (diagnostic only, not a fix) | |

Face loss on `00009190`: 394 → 449, with "no surface" flat at 276 and the rest
in "meshed to nothing".

Renders: `../look-trimming-residual/renders/` (iso, front at 1400×1400).

---

## Why the architecture changed

Four distinct defects surfaced in one session. Every one is the same failure
mode: **an invalid state was representable, and the next stage consumed it
happily**, producing a smooth plausible blob instead of an error.

| defect | mechanism |
|---|---|
| Signed-area domain semantics | Material side inferred from a chart-dependent predicate; `A(φ∘γ) = −A(γ)` while the face is unchanged, so the same solid meshed differently under a reflected chart |
| Periodic lift instability | `get_mindiff` breaks a tie arbitrarily at exactly half a period, so a boundary's winding class depended on tessellation tolerance |
| `eidx_map` reserve-before-convert | Index claimed before the conversion that decides whether the edge exists; one failure desynchronised map and vector, and faces silently received a neighbouring edge's curve |
| `surface.invert()` | Breaks curve-on-surface incidence rather than only reversing parameterization |

Four in one session is a design property, not bad luck. The response is to make
each pipeline stage a fallible constructor whose output type carries evidence,
so later stages cannot consume unproven state.

**Sequencing principle: each PR stands alone.** If priorities change, the work
done so far should have shipped value rather than left a half-rewrite.

---

## Roadmap

### PR 1 — Residual gate (IN PROGRESS, NOT VERIFIED)

A nearest point is not an incidence. `search_nearest_parameter` answers whether
or not the query lies on the surface, so a boundary belonging to another face
still yields a plausible parameter and a smooth uv path.

Implemented in `PolyBoundaryPiece::try_new`: `tol` threaded through both call
sites, boundary points rejected at `residual > tol * COMPATIBILITY_FACTOR`
(currently 5.0), diagnostic behind `TRUCK_PROBE_COMPAT`.

> **Status: fires, but the factor is untuned and the cost is real.** Measured
> on `00009190` at `COMPATIBILITY_FACTOR = 5.0`:
>
> | | before gate | with gate |
> |---|---:|---:|
> | faces lost | 449 | **685** |
> | — no surface | 276 | **519** |
> | triangles | 216,129 | 195,248 |
>
> It rejects **243 additional faces**, about 1% of the model, and does not
> reduce the blob count on 160144/160014 (it fires twice on 160039). Two
> readings are possible and they have opposite consequences: either 5× is too
> tight and is discarding faces that meshed acceptably, or ~1% of this model's
> faces genuinely violate curve-on-surface incidence and the underlying import
> defect is far more widespread than the blob count suggests.
>
> **Next session: distinguish these before tuning the factor.** Sweep the
> factor (5, 10, 25, 100) and plot rejected faces against it. A sharp knee means
> a real population of incompatible faces; a smooth curve means the threshold is
> arbitrary. Also check whether the rejected faces are visible in the render at
> all — a face rejected here previously produced *something*, and whether that
> something was right is the question the blob metric cannot answer.

### PR 2 — Typed identities and arenas

Newtype `EdgeCurveId`, `EdgeIndex`, `VertexIndex`; naked `usize` does not cross
arena boundaries. `EdgeArena::try_insert` performs conversion, push, and
source-ID mapping as one transaction, so the `eidx_map` class becomes
unconstructable rather than fixed.

Regression: valid A / invalid B / valid C must yield `map[C] == 1` addressing C,
never a neighbour.

### PR 3 — Fail-whole-bound conversion

`face_bound_to_edges` uses `filter_map`, so every `?` **drops that edge from the
wire** rather than failing the face. A bound missing an edge is a broken bound,
not a shorter one. Replace with `collect::<Result<_,_>>()` and a `ClosedWire`
smart constructor; assert source-use count equals resolved-use count.

### PR 4 — Source identity through `CompressedEdge`

Carry the `EDGE_CURVE` entity ID to the point of use and assert the edge a face
receives is the one its `ORIENTED_EDGE` named. Turns a whole class of geometric
inference into a one-line equality check. (A probe form of this already exists
behind `TRUCK_PROBE_IDENTITY`; it reports clean on all three reproducers.)

### PR 5 — Explicit domain semantics

Preserve `FACE_OUTER_BOUND` vs `FACE_BOUND` through `truck-stepio` — currently
both parse into the same `FaceBound` struct, discarding the outer/hole role the
file states explicitly. Add `BaseDomain { Empty, NaturalRange, PeriodicQuotient }`
and `BoundRole { Outer, Inner }` as `Known`/`Unknown`, and classify by
`base XOR parity` rather than inferring from loop count.

Note `closed.is_empty()` (the current gate) cannot be the final rule: parity
answers "how many boundaries crossed", not "was the starting region material".

### PR 6 — Quotient topology and deck consistency

`QuotientLoop` carrying `(k_u, k_v)`; a relative deck-offset solver across the
bounds of one face. Each bound is currently lifted from `sp(surface, pt, None)`
— an arbitrary principal value — so relative offsets between bounds were never
controlled. Measured: two bounds of one face at `quot_v = −1` and `+1`.

### PR 7 — Constraint provenance and conforming CDT

`insert_to` silently skips constraints it cannot add. Flood-fill labelling is
`O(T+E)` versus the current `O(nm)` per-point ray cast, but a *missing*
constraint leaks a label across a whole region, so it requires
`requested == inserted` verification with per-face fallback.

### PR 8 — Certified face and shell meshes

Surface approximation bound, shared-edge conformance, shell incidence.

### PR 9 — Property and metamorphic harness

Chart reflection, wire reversal, seam shift, cyclic rotation, tolerance sweep.
The two existing tests in `truck-meshalgo/tests/tessellation/trimming_domain.rs`
are the template.

---

## The open defect: 2 remaining blob shells

Reproducers: `../look-trimming-residual/repro/blobs/` — `shell_160144.step`
(76 faces), `shell_160014.step` (53). Each reproduces standalone at the parent
model's tolerance and is correct at its own:

```console
cargo run --release --example find_blobs -- shell_160144.step 0.003056   # blob
cargo run --release --example find_blobs -- shell_160144.step            # clean
```

### What is established by measurement

- The **source STEP is correct**. All 15 circle/cylinder pairs in 160039 are
  exactly incident: `e_angle = 0`, `e_axis ≤ 1e-14`, `e_radius = 0`.
- **Surface conversion is correct.** Fitted cross-section radii match source
  cylinder radii to five decimals.
- **Curve conversion is correct.** Converted edge radii reproduce the source
  inventory {0.003, 0.00337, 0.0035, 0.004, 0.0045, 0.006}.
- **Edge association is correct.** `TRUCK_PROBE_IDENTITY` reports
  `mismatched=0`, `edges == mapped`, on all three.
- Yet boundary points sit **0.027 from their own surface** — nine times the
  chord tolerance — and `d_min` by brute force over the whole parameter domain
  equals the projection's residual, so **the projection is right and the input
  is wrong**.
- `surface.invert()` accounts for 160039 but **not** 160144 or 160014.

### Remaining candidates

Face→surface pairing (checked for edges, **not** for surfaces), or a transform
applied inconsistently between curve and surface. The next probe is the
transform provenance dump: for one bad face, evaluate source and converted
geometry in the same coordinates at each transform stage, and find the first
stage where `d(C(t), S) ≤ ε` stops holding.

---

## Traps — read before trusting any measurement here

- **Detectors have had the bug they were hunting, twice.** `find_untrimmed`
  measured shells from vertices only, and a circle has one vertex, so every
  circle-bounded shell measured near zero and its faces looked enormous. The
  widely-quoted "710 of 24,201 faces untrimmed" came from that and **is not a
  real number**. Two shells named as worst offenders render correctly.
- **`ASSOC` probe resolution.** Falls back to a `(−1, 1)` window on an axis with
  neither range nor period; at `GRID=60` that is coarser than the residuals it
  reports. Its absolute numbers are not trustworthy.
- **`normal_align ≈ 1` proves nothing.** It is the first-order optimality
  condition of any nearest-point projection. An earlier reading of it as
  evidence of a tilted circle was wrong.
- **A three-point circle fit on a doubly-winding bound is meaningless** as a
  radius.
- Never pass a POSIX-style path to a Windows executable. It fails silently and
  has invalidated measurement runs.
- A measurement taken while the machine is short of memory or disk is not a
  measurement.

## Corpus

**`C:\Users\stefa\look-corpus\abc`** — 20 ABC models, 4.2 GB, moved out of a
temp scratchpad 2026-07-28 because that was the only copy.

**Not in the repo and cannot all be**: 12 of 20 exceed GitHub's 100 MB per-file
limit (up to 540 MB), and 4.2 GB busts the free LFS tier while every CI clone
would pull it. The five smallest are 67–86 MB; committing them was discussed and
not done.

## Companion repositories

- `../look-untrimmed-bug` — the original complement defect: plain-English
  statement, both code paths (truck and OCCT) with real code, extraction tool.
- `../look-trimming-residual` — formal treatment of the invariance fix and the
  residual scale defect, plus the isolated blob reproducers and renders.

## Tools

- `examples/find_blobs.rs` — per-shell mesh extent versus own extent. Optional
  second argument overrides the tolerance, which is how an extracted shell is
  meshed at its parent model's tolerance.
- `examples/find_untrimmed.rs` — per-face variant. Slow (24k separate
  triangulations) and superseded by `find_blobs` for most purposes.
- `examples/face_profile.rs` — per-face timing.
- `../look-untrimmed-bug/tools/extract_shell.py` — transitive closure of one
  `CLOSED_SHELL` into a valid standalone Part 21 file.

Fork probes, all env-gated: `TRUCK_PROBE_BOUNDARY`, `TRUCK_PROBE_LIFT`,
`TRUCK_PROBE_EDGE`, `TRUCK_PROBE_ASSOC`, `TRUCK_PROBE_IDENTITY`,
`TRUCK_PROBE_COMPAT`, and `TRUCK_NO_INVERT` (disables `surface.invert()`).
**These are diagnostic scaffolding and should be culled or promoted to typed
certificates before the fork rev is bumped.**

## Deferred performance work

Measured and real, but subordinate to correctness:

- STEP index buffer is the identity permutation — 4 bytes/vertex, 22 MB on a
  1.9 M-triangle model. Needs a non-indexed draw path.
- Vertices are unwelded; welding on position *and* normal keeps creases.
- `source_attributes` stores one identical 32-byte value per vertex.
- Entity table builds ~60 typed maps regardless of need.
- `upload_scene` makes a second full CPU copy before the GPU buffer exists.
- Non-converging NURBS faces: 15 of 24,202 held 98.3% of tessellation time.
  Needs an early exit in `sub_parameter_division`, not caching.
- `docs/patches/truck-synthetic-boundary-decimation.patch` — unlanded, carries
  profiling `eprintln!`s. May now be moot; re-measure before landing.

## How results are reported

- Benchmark release builds; retain raw samples and the hardware fingerprint.
- Fresh-process and resident-session numbers answer different questions and are
  never combined.
- F3D comparisons use `--force-reader=STEP`, explicit camera, resolution and
  background, and alternating launch order.
- Say when a number is attribution rather than a benchmark.

See `docs/BENCHMARKS.md`, `docs/ARCHITECTURE.md`, and `AGENTS.md`.
