# WORK PACKET BG-SOL-P0-SPAN — the lazy rational-Bézier span cache

You are implementing the solver family's span extraction: per-carrier,
per-knot-span `SpanRecord`s (conservative bounding box, derivative hull,
parameter window) that the broad phase and the certified solvers both consume.
Everything you need is in this document. **Do not read any other spec file** —
this packet is self-contained. It implements the approved design in
`docs/SOLVER_FAMILY_PLAN.md` §2 and §4 (Phase 0, `truck-geometry` module
`span`), sharing the `BoundedPiece` vocabulary with `truck-base/src/bvh.rs`
(packet BG-SOL-P0-BVH, already scaffolded).

```json
{"id":"BG-SOL-P0-SPAN","status":"DONE","contracts":["BG-SOL-P0-SPAN"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-P0-SPAN
class:       design
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/span.rs
read_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-base/src/bvh.rs
tests_required:
  - span_bspline_surface_produces_per_span_records
  - span_plane_is_exact_corner_hull
  - span_processor_transforms_the_box
  - span_cache_reuses_keyed_extraction
  - span_unbounded_cylinder_has_no_spans
budget:      {turns: 65, ctx_tokens: 150000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod span' vendor/truck/truck-geometry/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum Surface' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'clippy::unwrap_used' vendor/truck/truck-geometry/src/span.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn to_single_multi' vendor/truck/truck-geometry/src/nurbs/knot_vec.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn bezier_decomposition' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
```

## Problem

The certified solver family narrows every surface-surface question to candidate
span pairs: each face's carrier is split into **span records** that carry a
conservative bounding box (for BVH culling) and derivative bounds (for the
later certified tests), then the BVH finds overlapping pairs. The span cache's
boxes must be **conservative** — every point of the span lies inside — because
an under-estimating box lets the broad phase cull a real intersection. The
trap is the same one the evidence substrate records: sampling a grid is NOT a
certification, so every box here comes from a structural argument (convex hull,
constant-partial, or closed-form image bound), and the sampling tests are
regression witnesses only.

The plan's §4 target reads `pub struct SpanRecord { pub bbox: Box3, ... }`
and `pub fn spans(&self, s: &Surface) -> Vec<SpanRecord>`. Two resolved
readings, record both in `disagreements`:

1. **`bbox` is `BoundingBox<Point3>`** (truck-base's `f64` broad-phase box, the
   same type the BVH packet uses), NOT `truck-evidence`'s certified interval
   `Box3` — `truck-geometry` has no `inari`. Certified enclosures convert
   outward at the solver boundary.
2. **`spans` takes a caller-owned cache key.** A value-keyed cache
   (`HashMap<Surface, ...>`) needs `Surface: Hash + Eq`, which it neither is
   nor should become, and a structural-hash key can collide silently, returning
   the WRONG spans for a certified pipeline. The cache is keyed by a
   caller-owned `u64` (e.g. the B-rep face index); the caller guarantees
   uniqueness per distinct `Surface` instance. Same surface under two keys is
   just two cached copies.

## The design — decide nothing, implement it

The scaffold has already declared the module (`pub mod span;` in
`truck-geometry/src/lib.rs`) and this file carries the H-1 deny header. You
fill the file. Do not edit `lib.rs`.

### 1. Types

```rust
/// One extracted span of a carrier surface: a conservative box over the
/// span's image, derivative bounds, and the span's parameter window.
#[derive(Clone, Debug, PartialEq)]
pub struct SpanRecord {
    /// Conservative box containing the span's image. MUST contain every
    /// surface point over `u_range × v_range`.
    pub bbox: BoundingBox<Point3>,
    /// Conservative bounds on the span's partials; empty boxes mean unknown.
    pub derivative_hull: DerivativeBounds,
    /// The span's parameter window in u.
    pub u_range: (f64, f64),
    /// The span's parameter window in v.
    pub v_range: (f64, f64),
}

/// Per-carrier lazy span extraction, cached by a caller-owned key.
#[derive(Default)]
pub struct SpanCache {
    inner: HashMap<u64, Vec<SpanRecord>>,
}

impl SpanCache {
    /// An empty cache.
    pub fn new() -> Self;

    /// The spans of `s` under `key`, extracting (once) and caching them.
    pub fn spans(&mut self, key: u64, s: &Surface) -> &[SpanRecord];
}
```

`BoundingBox` / `DerivativeBounds` / `BoundedPiece` come from
`truck_base::bvh` (and `truck_base::bounding_box::BoundingBox`). The module
denies `clippy::indexing_slicing`: read slices with `.get()` and skip the
record rather than indexing.

### 2. Extraction rules — per `Surface` arm, exactly these

Let `hull(pts)` mean `BoundingBox::new()` then `push` each point.

- **`Surface::Plane(p)`** — ONE record over `u_range = v_range = (0.0, 1.0)`.
  `bbox = hull([p.subs(0,0), p.subs(0,1), p.subs(1,0), p.subs(1,1)])` — the
  plane is bilinear (`o + u(p−o) + v(q−o)`), so the corner hull is **exact**.
  `derivative_hull.first` = hull of the two constant partials `p.uder(0,0)`,
  `p.vder(0,0)` treated as points; `.second` = the point `(0,0,0)`.
- **`Surface::Sphere(s)`** — ONE record over `s.try_range_tuple()` (both
  bounded: `u ∈ [0, π]`, `v ∈ [0, 2π)`). `bbox` = `center ± r` on every axis
  (the full bounding box; the arc patch always lies inside — loose but
  sound). `derivative_hull` = `DerivativeBounds::new()` (unknown).
- **`Surface::Torus(t)`** — ONE record over `t.try_range_tuple()`.
  `bbox` = `center ± (R+r, R+r, r)` (the full bounding box; sound by the same
  argument). `derivative_hull` = unknown.
- **`Surface::Cylinder(_)` and `Surface::Cone(_)`** — EMPTY list. Their `v`
  range is unbounded (`(Unbounded, Unbounded)`), so no finite span exists and
  no box can bound them; the trim lives in the B-rep face, not the carrier,
  and arrives with the Contact Layer (documented Phase-0 scope boundary).
- **`Surface::BSplineSurface(p)`** — the per-knot-span extraction:
  1. Clone `p`. For every distinct interior knot value `x` of
     `p.uknot_vec().to_single_multi()` with multiplicity `m < udegree()`,
     call `add_uknot(x)` `udegree() - m` times. Same for v. (Exact counting —
     `KnotVec::multiplicity` matches by tolerance; use exact equality, the
     deviation.rs `exact_count` pattern.) A knot already at multiplicity
     `≥ degree` is left alone.
  2. After insertion, the distinct knot values in each direction partition
     the domain. For span `k` (between distinct values `u_k` and `u_{k+1}`),
     the Bézier control rows are `control_points()[k*udegree .. k*udegree +
     udegree + 1]`, and symmetrically for v. Skip zero-width spans
     (`u_{k+1} == u_k`).
  3. `bbox` = hull of the span's `(udegree+1) × (vdegree+1)` control sub-grid
     (convex-hull property — CERTIFIED).
  4. `derivative_hull.first` = union of the two first-derivative hulls from the
     sub-grid (Bézier derivative control points in GLOBAL units, `w_u =
     u_{k+1} − u_k`, `w_v = v_{l+1} − v_l`):
     - u: `hull({ udegree * (P[i+1][j] − P[i][j]) / w_u })`
     - v: `hull({ vdegree * (P[i][j+1] − P[i][j]) / w_v })`
     `.second` similarly from the second differences:
     - uu: `udegree*(udegree−1) * (P[i+2][j] − 2 P[i+1][j] + P[i][j]) / w_u²`
     - vv: `vdegree*(vdegree−1) * (P[i][j+2] − 2 P[i][j+1] + P[i][j]) / w_v²`
     - uv: `udegree*vdegree * (P[i+1][j+1] − P[i+1][j] − P[i][j+1] + P[i][j]) / (w_u * w_v)`
     (`P` is the span's local sub-grid; out-of-range indices contribute
     nothing.)
  5. `u_range`/`v_range` = `(u_k, u_{k+1})`, `(v_l, v_{l+1})`.
- **`Surface::NurbsSurface(p)`** — same insertion on the homogeneous clone
  (`add_uknot`/`add_vknot` exist on `NurbsSurface`). Per-span sub-grid of
  homogeneous `Vector4` control points; **if any weight in the whole surface
  is `<= 0`**, return EMPTY for that surface (the projected hull property
  needs positive weights; refusing is the honest answer). Otherwise `bbox` =
  hull of the projected points `cp.to_point()` (the rational patch lies in the
  convex hull of the projected control points for positive weights —
  CERTIFIED). `derivative_hull` = `DerivativeBounds::new()` (the rational
  derivative's control points are not a simple hull — documented). Ranges as
  above.
- **`Surface::Processor(pr)`** — recurse on `pr.entity()`; transform each
  returned record's `bbox` by `pr.transform()` (affine: transform the 8
  corners via `Matrix4::transform_point` and hull — exact under affine), and
  each `derivative_hull` box by `transform_vector` (the derivative of `M∘S` is
  `M·S'`) on its 8 corners. Ranges unchanged. If the inner carrier yields
  nothing, nothing.
- **`Surface::RevolutedCurve(_)` and `Surface::ExtrudedCurve(_)`** — EMPTY.
  Not canonical; `recognize_surface` (packet BG-SOL-P0-REC) canonicalizes them
  to analytic carriers before spanning, which is the whole point of the
  recognizer. Documented Phase-0 scope boundary.

### 3. Determinism

Every rule above is a pure function of the `Surface` value: no iteration over
an unordered collection, insertion in ascending knot order, control grids read
in index order. Two identical surfaces produce identical records.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's code divides by span widths (`w_u`, `w_v`) and degrees — no small
literals. The sampling tests compare points against boxes with `contains` —
exact, no tolerance. If any comparison needs a slack, use the named-const
form on the same line:

```rust
const SLACK: f64 = 1.0e-9; // H-3: <why this slack, dimensionally>
```

Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

Put the tests in a `#[cfg(test)] mod tests` inside `span.rs` with
`#[allow(clippy::unwrap_used, clippy::expect_used)]`. Build surfaces with the
house constructors (`BSplineSurface::new(KnotVec::uniform_knot(deg, div),
control_points)`; `KnotVec` comes from `crate::nurbs::*`).

1. `span_bspline_surface_produces_per_span_records` — a degree-2×2
   `BSplineSurface` over `uniform_knot(2, 2)` (three distinct knots per axis →
   two spans per axis → 4 records). Assert: record count 4; each record's
   `u_range`/`v_range` partition the domain; each record's `bbox` contains
   every sample of the surface on a 9×9 grid restricted to that span; the
   union of the records' boxes contains the full-domain grid.
2. `span_plane_is_exact_corner_hull` — `Plane::new((0,0,0), (2,0,0), (0,1,0))`;
   one record; `bbox` contains the four corners and the two midpoints; the
   box's `min()` is `(0,0,0)` and `max()` is `(2,1,0)` exactly; the
   `derivative_hull.first` box contains both `(2,0,0)` and `(0,1,0)`.
3. `span_processor_transforms_the_box` — a `BSplineSurface` placed through
   `Processor::with_transform(entity, Matrix4::from_translation(v))` for
   `v = (1, 2, 3)`; the records' `bbox`es equal the inner records' `bbox`es
   translated by `v` (assert on `min()`/`max()`).
4. `span_cache_reuses_keyed_extraction` — a cache; call `spans(7, &surface)`
   twice with the same key; assert the two returned slices are equal; call
   `spans(8, &surface)` with a different key; assert it equals the first
   (extraction is a pure function), and that `spans(7, …)` is unchanged.
5. `span_unbounded_cylinder_has_no_spans` —
   `Cylinder::new(Point3::origin(), 1.0).expect("valid")`; assert `spans`
   returns an empty slice. Same for a `Cone::new(Point3::origin(), PI/4)`
   if the constructor accepts it.

Every other existing truck-geometry test must stay green.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo check --locked -p truck-geometry --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Returning a box that does not contain
the span's image — under-estimation is a silent wrong answer; if a carrier's
image bound cannot be certified, return EMPTY for that surface (honest), never
a sampled guess. Sampling to manufacture a box. Hashing a `Surface` value to
key the cache (the caller-owned key is the contract). Adding `#[ignore]`.
Changing the GATE-4 ceiling. Running cargo check --workspace / cargo build --workspace / a bare cargo check (the crate-scoped -p <crate> checks in Done-when are the contract; a workspace-wide build on a shared machine with concurrent workers exhausts disk).

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
two resolved readings (`BoundingBox<Point3>` for `bbox`; the caller-owned key
for the cache) and the span counts you observed on the test surface.

Commit on the current branch with subject
`feat(geometry): lazy rational-Bezier span cache (BG-SOL-P0-SPAN)`.
