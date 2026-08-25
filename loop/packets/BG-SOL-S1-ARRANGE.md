# WORK PACKET BG-SOL-S1-ARRANGE — the 2-D planar arrangement over analytic profiles

You are implementing S1 of the solver family: the planar arrangement that turns
a closed analytic profile (`Line`/`Circle` curves in the plane) into a certified
2-D subdivision — vertices, half-edges and regions with winding numbers. It is
the critical path to M1 (certified planar construction: rectangle − circle →
arrangement → profile with hole → direct extrude). Everything you need is in
this document. **Do not read any other spec file** — this packet is
self-contained. It implements the approved design in
`docs/SOLVER_FAMILY_PLAN.md` §4 Phase 1 and §7 M1, on top of the LANDED Phase-0
API (orient2d, CurveContact, BoundingBox<Point2>).

```json
{"id":"BG-SOL-S1-ARRANGE","status":"DONE","contracts":["BG-SOL-S1-ARRANGE"],
 "tests_added":5,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S1-ARRANGE
class:       design
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/src/arrange.rs
read_allow:
  - vendor/truck/truck-base/src/pred.rs
  - vendor/truck/truck-base/src/contact.rs
  - vendor/truck/truck-geometry/src/recognize.rs
tests_required:
  - arrange_rectangle_with_hole_has_three_regions
  - arrange_crossing_lines_split_at_the_intersection
  - arrange_line_circle_crossing_is_dyadic_exact
  - arrange_self_intersecting_profile_is_refused
  - arrange_circle_winding_is_one
budget:      {turns: 90, ctx_tokens: 220000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod arrange' vendor/truck/truck-geometry/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn orient2d' vendor/truck/truck-base/src/pred.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum ContactEventKind' vendor/truck/truck-base/src/contact.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub enum Surface' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A5, expect: 1, cmd: "grep -c '^#!\\[deny(' vendor/truck/truck-geometry/src/arrange.rs"}
```

## Problem

M1 (plan §7) constructs a plate with a cylindrical hole by 2-D means: the
profile `rectangle − circle` is a set of closed analytic loops, and the first
step is to certify the profile's planar structure — which loops exist, which
vertices bound which edges, and which regions the loops enclose — as a planar
arrangement. S1 produces that `Arrangement`. It is the reference 2-D structure
the whole solver family reuses: S2 extrudes it into a B-rep, the Contact Layer
reuses its `CurveContact` event vocabulary, and S5's coincidence handling is a
lookup on the recognizer + arrangement rather than a re-solve.

S1 must be **certified at the topology level**: which edge crosses which is a
predicate decision (`orient2d`, exact), never a float guess, and the vertices
of M1's profiles are exactly representable. The general algebraic intersection
point (a circle crossing a line at a square-root coordinate) is the documented
extension point; v1 computes only exactly-representable vertices and refuses
the rest honestly.

## Design decisions already made for you

### 0. Module and scaffolding

The module is ALREADY declared: the scaffold commit added `pub mod arrange;` to
`truck-geometry/src/lib.rs` and created this file with the H-1 deny header and
this contract doc. You fill `arrange.rs` and do NOT touch `lib.rs`. The file
keeps the deny header:

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]
```

### 1. The 2-D setting

The profile is a `&[Curve]` of **`Curve::Line` and `Curve::Circle`** variants
only. Everything else (`Curve::BSplineCurve`, `NurbsCurve`,
`IntersectionCurve`) → `Err(Refusal::UnsupportedEnvelope(
EnvelopeCase::NonCanonicalCarrier))`. The profile is assumed to lie in the
plane **z = 0** (M1's setting); a curve whose control data leaves z = 0 by more
than the representation tolerance is refused with `UnsupportedEnvelope(
ChartDegenerate)` — a general plane basis is a documented later extension.

The 2-D coordinates of a `Curve::Line(Line(a, b))` are `(a.x, a.y)` / `(b.x,
b.y)`; of a `Curve::Circle(p)` the placed unit circle: its position is the
translation column `p.transform().w.to_point()` and its radius is
`p.transform().x.magnitude()`, and its parameter is the angle `t ∈ [t0, t1]`
from the trimmed range. **Re-derive these from `recognize.rs` and the
`canonical.rs` cylinder test before coding** — the packet reuses the same
exact-comparison conventions.

### 2. The types — decide nothing, type them exactly

```rust
/// A vertex of the arrangement.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrVertex {
    /// The vertex's 3-D position (z = 0 for the planar profile).
    pub point: Point3,
    /// Indices into `Arrangement::half_edges` of the edges originating here.
    pub incident: Vec<usize>,
}

/// A directed edge of the arrangement (a half-edge).
#[derive(Clone, Debug, PartialEq)]
pub struct ArrHalfEdge {
    /// The origin vertex (index into `vertices`).
    pub origin: usize,
    /// The twin half-edge (index into `half_edges`).
    pub twin: usize,
    /// The next half-edge around this edge's face, CCW.
    pub next: usize,
    /// The previous half-edge around this edge's face.
    pub prev: usize,
    /// Index into the input `profile` slice this edge lies on.
    pub curve: usize,
    /// Parameter window on that curve (in the curve's own parameter).
    pub u_range: (f64, f64),
}

/// A face of the planar subdivision.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrRegion {
    /// The region's boundary half-edge cycles, in order. A region with a
    /// hole has MORE THAN ONE cycle: the first is the outer boundary (CCW),
    /// the rest are the holes (CW). M1's plate is the canonical case:
    /// `boundaries = [[outer rectangle cycle], [inner circle cycle]]`.
    /// A region's total boundary is the union of its cycles.
    pub boundaries: Vec<Vec<usize>>,
    /// The winding number of the region around any interior point.
    pub winding: i32,
    /// Whether the region is bounded (M1: the plate and the hole are
    /// bounded; the exterior is not).
    pub bounded: bool,
}

/// The planar subdivision of a closed analytic profile.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Arrangement {
    pub vertices: Vec<ArrVertex>,
    pub half_edges: Vec<ArrHalfEdge>,
    pub regions: Vec<ArrRegion>,
}

/// Builds the arrangement of a closed analytic profile. The profile's loops
/// must be closed (each curve's end meets the next start within the
/// representation tolerance) and pairwise disjoint in the M1 contract;
/// interior crossings are supported by the machinery and reported as split
/// vertices (tests below prove it), but a self-intersecting single loop is
/// refused.
pub fn arrange(profile: &[Curve], domain: Option<BoundingBox<Point2>>)
    -> Outcome<Arrangement>;
```

`BoundingBox<Point2>` is `truck_base::bounding_box::BoundingBox`; `Point2`,
`Point3`, `Vector2`, `Vector3` from `crate::prelude::*`. House rule H-1
(indexing_slicing) applies: read the vecs with `.get()`.

### 3. The pipeline — decide nothing, implement it

The v1 pipeline is the M1-sufficient general shape:

**Stage 1 — closed-loop validation.** Walk the profile in order; assert each
`Curve`'s end point equals the next curve's start point within
`64.0 * TOLERANCE` (`TOLERANCE` is `truck_base::tolerance::TOLERANCE`, a
name — H-3-clean). The profile is a sequence of closed loops. A gap beyond
tolerance → `Err(Refusal::Contradictory(ContradictionWitness { prop: Prop::
DomainBoundary, left: Truth::False, right: Truth::True }))` — a broken loop is a
contradiction between the declared boundary and the actual geometry.

**Stage 2 — pairwise intersections (exact where the vertices are exact).**
For every ordered pair of distinct curves in the profile, compute their
intersections:
- **Line/Line** (`Line<Point2>` from `recognize` of the two curves): the exact
  crossing decision from `orient2d` (the four endpoint configurations — a pair
  of segments cross iff the endpoints of each straddle the other, decided
  exactly). The intersection parameters and point are computed EXACTLY by
  Cramer's rule in scaled integer arithmetic (`i128`) when the coordinates fit
  — NOT from `Line::intersection`'s f64 result, which is a reference for the
  algebra only. The vertex is exactly representable when the endpoints are
  commensurate dyadic/rational values; refuse
  `NumericallyUnresolved(RootNotIsolated)` when the scaled integer arithmetic
  overflows (coordinates beyond ~1e18 or incommensurate scale). For v1 the
  exactly-representable case is the contract.
- **Line/Circle and Circle/Circle**: solve the quadratic exactly (the radical
  axis for Circle/Circle, the `(d·d)t² + 2(f·d)t + (f·f − r²)` for Line/Circle,
  both machine-checked before dispatch). When the discriminant is a perfect
  square of a dyadic rational, the vertices are exactly representable — compute
  them exactly (dyadic rational arithmetic in i128 with scaling). Otherwise the
  vertex is algebraic → v1 refuses
  `Err(Refusal::NumericallyUnresolved { witness: UnresolvedWitness::RootNotIsolated,
  spent: Budget::new(0, 0, 0) })` — the interval-certified vertex substrate
  (via `num::roots`/`num::krawczyk`) is the documented extension.
- A pair of curves whose pieces overlap on an interval (e.g. two collinear
  overlapping lines, or two coincident circles) → `Err(Refusal::Empty)` — the
  arrangement's domain (non-degenerate pairwise intersections) is violated; the
  2-D overlap case is S5.3 territory and out of v1 scope. Record in
  `disagreements` if the evidence algebra makes a different refusal read more
  honestly.

**Stage 3 — vertex and edge construction.** Collect every vertex: curve
endpoints and interior intersection points. For each curve, sort its vertex
parameters; split the curve at every interior parameter. Each resulting
segment is a half-edge; add its twin. Connect half-edges into a planar graph
by origin/destination vertex identity (exact `Point3` equality — the vertices
are exactly representable).

**Stage 4 — DCEL wiring (the `next`/`prev` links).** For each vertex, order
its outgoing half-edges by angle around the vertex (use `orient2d` of the
edges' first interior points, or the exact `atan2` of the segment direction —
prefer the predicate, document the choice). Wire each half-edge's `next` to
the first outgoing half-edge of its destination vertex that is CCW of the twin
(the standard "turn left at the vertex" traversal). This produces the face
cycles.

**Stage 5 — region tracing, grouping and winding.** Walk every half-edge
cycle once per face (the "turn left at the vertex" traversal of Stage 4 yields
each face cycle). Then GROUP the face cycles into regions by containment: a
cycle C is a hole of region R when C lies strictly inside R's outer cycle and
no other cycle lies between them. The containment test is a point-in-loop
predicate (a point strictly inside C's cycle by the ray-casting winding test on
a polygonization of the cycle) — exact for the dyadic witnesses. M1's
rectangle + circle yields exactly the cycles `[rect loop, circle loop]` and the
grouping gives three regions: the exterior (`boundaries = [[rect loop]]`,
winding 0, unbounded), the plate (`boundaries = [[rect loop], [circle loop]]`,
winding 1, bounded — the circle cycle is its HOLE), and the hole interior
(`boundaries = [[circle loop]]`, winding 1, bounded). Winding is computed per
region from its representative interior point over the profile loops (the
standard ray-casting winding, driven by `orient2d` for the crossing decisions).
Two bounded regions can share a winding number (the plate and the hole are
both ±1) — they are distinguished by nesting, not by winding; the test asserts
the nesting explicitly.

**Stage 6 — the domain.** If `domain` is `Some`, clip the arrangement's region
classification to it (a region wholly outside the domain is not reported); if
`None`, the exterior is unbounded and is reported as the single winding-0
region. M1 passes `None`.

### 4. The certified-relationship contract

Every vertex position, every crossing decision and every winding number is
either exact (an exactly representable rational — dyadic for the packet's
witnesses) or an honest refusal. No vertex is a float approximation of an
algebraic point in v1. The tests below verify exactness by construction (the
witness vertices are dyadic) and by value assertion.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. All
tolerances in this packet are `64.0 * TOLERANCE` (a name) or `TOLERANCE`; the
curve-joining check uses `64.0 * TOLERANCE` — a length through the named
representation tolerance, H-3-clean. If a comparison ever needs a small
literal, use the same-line form:

```rust
const EPS: f64 = 1.0e-9; // H-3: <why this slack, dimensionally>
```

Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## GATE-4 / `unscaled_legacy` (the ratchet)

This packet adds NO `unscaled_legacy()` calls. Do not touch
`scripts/unscaled_legacy_ceiling.txt` — the orchestrator owns the ratchet.

## Regression tests (exact names)

Put the tests in a `#[cfg(test)] mod tests` inside `arrange.rs` with
`#[allow(clippy::unwrap_used, clippy::expect_used)]` (the module-level H-1 deny
does not apply to test assertions). All witnesses are machine-checked and use
dyadic coordinates.

1. `arrange_rectangle_with_hole_has_three_regions` — the M1 profile: a 4×4
   rectangle as four `Curve::Line`s `(0,0)→(4,0)→(4,4)→(0,4)→(0,0)` and a
   `Curve::Circle` of radius 1 centered at `(2,2)` (full circle, trimmed range
   `(0, 2π)`). Assert: `arrange` returns `Ok`; there are exactly **three**
   regions; exactly one is the exterior (winding 0, unbounded) with
   `boundaries == [[the rectangle cycle]]`; the plate region is bounded,
   winding ±1, with `boundaries` containing TWO cycles — the rectangle cycle
   and the circle cycle (the circle is the plate's HOLE); the hole region is
   bounded, winding ±1, with `boundaries == [[the circle cycle]]`. Assert the
   vertex count is 5 in the canonical no-split case (4 rectangle corners + the
   circle's single seam vertex — the full circle is one closed edge); if your
   implementation splits the circle for x-monotonicity, assert the split
   vertices' coordinates instead and say so in `notes`.
2. `arrange_crossing_lines_split_at_the_intersection` — two `Curve::Line`s
   crossing at a dyadic point: `(0,0)→(2,2)` and `(0,2)→(2,0)`. Assert: the
   arrangement has a vertex at `(1,1)` exactly; the crossing vertex has four
   incident half-edges; and the arrangement has exactly **four** regions — the
   four wedges of the crossing, each unbounded and winding 0 (the finite
   segments form an X whose complement is four unbounded regions; there is NO
   separate bounded "quadrant" face).
3. `arrange_line_circle_crossing_is_dyadic_exact` — the machine-checked
   witness: line `(−1,0)→(3,0)` and circle center `(1,0)` radius 1. Assert the
   two intersection vertices at `(0,0)` and `(2,0)` exactly (dyadic — the
   packet's formula gives `t = 0.25` and `t = 0.75` on the line), and the
   circle is split into two arcs between them.
4. `arrange_self_intersecting_profile_is_refused` — a single `Curve::Line`
   bowtie profile that is not a valid closed loop (e.g. the four-line bowtie
   `(0,0)→(2,2)→(0,2)→(2,0)→(0,0)`) → `Err`, never a panic. (The M1 contract
   is pairwise-disjoint loops; a self-crossing single loop is the documented
   refusal case. If your implementation computes the bowtie's crossing as a
   valid split vertex and then refuses at the region stage, record that in
   `disagreements` — either way the result is `Err`, not a graph.)
5. `arrange_circle_winding_is_one` — a single full `Curve::Circle` (center
   `(0,0)`, radius 1). Assert two regions: the bounded interior (winding 1)
   and the unbounded exterior (winding 0), and that the winding of the
   interior point `(0,0)` over the circle loop is exactly 1 (i.e. +1 for the
   CCW parameterization).

Every other existing truck-geometry test must stay green — in particular the
Phase-0 `recognize`/`span` suites and the `canonical`/`decorators` suites.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo test -p truck-geometry --lib --tests --no-fail-fast
cargo check --locked -p truck-geometry --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine with concurrent workers (this packet's
Forbidden list includes it explicitly for that reason).

## Forbidden

Editing any file outside `write_allow`. Approximating an algebraic
intersection vertex in f64 and calling it certified — v1 refuses those with
`NumericallyUnresolved`; the interval-certified vertex substrate is a later
packet. Guessing a crossing decision or winding number from float comparisons
— the topology decisions use `orient2d`/exact predicates. Treating a
self-intersecting loop as a valid M1 profile. Running `cargo check --workspace`
/ `cargo build --workspace` (disk). Adding `#[ignore]`. Changing the GATE-4
ceiling.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a pre-existing test you did not expect to touch is broken → do NOT weaken the
  gate; report it in `disagreements` with the failing test name and the exact
  reason
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it. In `notes`, record the
region counts you observed on test 1 (must be exactly 3), the vertex count,
the `next`-wiring choice you made (predicate vs atan2), and the refusal path
you used for the overlapping/coincident pair case (`Collapsed` vs `Empty`).

Commit on the current branch with subject
`feat(geometry): certified planar arrangement over analytic profiles (BG-SOL-S1-ARRANGE)`.
