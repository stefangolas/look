# WORK PACKET BG-SOL-S7-OVERLAP - the 2-D overlap screen

Both coincident paths in the Contact Layer overclaim: the struct-equal
identity arms and the analytic `Coincident` cells emit Region2/Arc1 records
without screening the parameter boxes, so two DISJOINT patches of the same
canonical carrier report contact today. Land the screen: parameter-box
interior overlap decides Coincident-vs-empty. If live code contradicts this
packet, report it in `disagreements`.

```json
{"id":"BG-SOL-S7-OVERLAP","status":"DONE","contracts":["BG-SOL-S7-OVERLAP"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S7-OVERLAP
contract:    [BG-SOL-S7-OVERLAP]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/overlap.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/fe_ee.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
tests_required:
  - overlap_screen_identity_face_disjoint_boxes_certify_empty
  - overlap_screen_identity_face_periodic_wrap_decides
  - overlap_screen_same_axis_cylinder_shift_decides
  - overlap_screen_parallel_frame_planes_decide
  - overlap_screen_edge_disjoint_ranges_certify_empty
  - overlap_screen_is_order_insensitive
budget:      {turns: 30, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 0, cmd: "ls vendor/truck/truck-evidence/src/contact/overlap.rs 2>/dev/null | wc -l"}
  - {id: A2, expect: 3, cmd: "grep -c 'ContactLocus::Coincident' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A3, expect: 5, cmd: "grep -c 'IdenticalCarrier' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn analytic_ff' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A5, expect: 2, cmd: "grep -c 'analytic_records' vendor/truck/truck-evidence/src/contact/mod.rs"}
```

A2 grows (empty-complex paths may reuse the arm); A3 stays 5; A4 stays 1;
A5 grows; A1 becomes 1.

## Problem

The identity arms match `(Face { surface: l, .. }, Face { surface: r, .. })
if l == r` and emit `Region2/IdenticalCarrier/Coincident` without looking at
the `(u, v)` boxes. Two disjoint patches of the same canonical plane — the
two sides of a shared wall — are reported as coincident TODAY. The analytic
path has the same hole: `plane_plane` returns `Coincident` for any coplanar
pair, `coaxial` CylCyl returns `Coincident` for equal radii, and
`analytic_records` maps both to Region2 records, again ignoring the boxes.
For the Boundary Rewrite a false Region2 record means splicing material
states between faces that never touch.

The screen: emit the Coincident record only when the two patches' parameter
boxes overlap with NON-EMPTY INTERIOR; otherwise return a certified empty
complex (`Method::Exact`, no budget spend, empty props). Boundary-only
contact (boxes touching at an edge or corner, interiors disjoint) is
intentionally EMPTY here: shared-boundary contact is owned by the FE/EE
stages over their own strata pairs.

All decisions are exact-f64 arithmetic on stored analytic data — the
BG-ANA-002 5.1 decision class already used by `parallel_cylinders`' exact
radius equality: sub-ulp boundary configurations may decide either way, and
test witnesses are dyadic. Say this in the module doc.

## Decisions already made

### 1. New module `contact/overlap.rs` (H-1 deny header, like `gff.rs`)

```rust
/// Strict interior overlap of two aperiodic intervals.
pub(crate) fn interior_overlap(a: (f64, f64), b: (f64, f64)) -> bool;
/// Strict interior overlap of two intervals on a circle of the given
/// period: each interval wraps into [0, period) as at most two arcs, and
/// any pair of arcs with strict interior overlap decides true. An interval
/// whose width is >= period covers the whole circle.
pub(crate) fn periodic_interior_overlap(
    a: (f64, f64), b: (f64, f64), period: f64,
) -> bool;
```

`interior_overlap(a, b) = a.0 < b.1 && b.0 < a.1` (degenerate intervals
have empty interior and never overlap). Mirror the wrap conventions of
`fe_ee.rs` (it wraps circle angles into `[0, TAU)`); the period is
`std::f64::consts::TAU` everywhere it appears.

### 2. Per-carrier periodicity table (the identity Face arm)

The same carrier means the same parameterization, so the screen is the box
test with the right periodicity, read off the carriers' own
`parameter_range`/`u_period` conventions:

- `Plane`: `interior_overlap` on u AND v (neither periodic).
- `Cylinder`: `periodic_interior_overlap(u, TAU)` AND `interior_overlap(v)`
  (u is the azimuth; v is z relative to the center).
- `Cone`: same as Cylinder (u azimuth, v axial from the apex).
- `Sphere`: `interior_overlap(u)` AND `periodic_interior_overlap(v, TAU)`
  (u is the POLAR angle on [0, PI], v is the azimuth - note the swap
  relative to cylinder/cone).
- `Torus`: periodic on BOTH u and v.
- `Placed`: struct-equal placements carry the same parameter map; screen
  the inner carrier (`Processor`'s `entity()` accessor) with its row of the
  table.

Apply the table inside BOTH identity arms' guards: when the screen says
overlap, keep the existing record and certificate byte-for-byte; when it
says no overlap, return the certified empty complex. The record shape, the
`Method::Exact`, the `Prop::AnalyticCarrier` stamp, and the untouched
budget are unchanged.

### 3. The Edge identity arm

Same curve, `t_range` in the curve's own parameterization:
`CanonicalCurve::Line` -> `interior_overlap`; `CanonicalCurve::Circle` ->
`periodic_interior_overlap(t1, t2, TAU)`. Equal placed-circle structs share
one parameterization, so the wrap test is exact.

### 4. The analytic `Coincident` screen (inside `analytic_ff`)

When the cell returns `AnalyticIntersection::Coincident`, screen before
`analytic_records`:

- **(Cylinder, Cylinder)**: the coaxial cell fired, so `(cx, cy, r)` are
  equal and the structs differ only in `cz`. u is identical; v differs by
  the center shift: patch 1's absolute z-extent is
  `[cz1 + v1.0, cz1 + v1.1]`, patch 2's is `[cz2 + v2.0, cz2 + v2.1]`
  (each endpoint ONE exactly-rounded f64 addition). Overlap = strict
  interior intersection of the absolute z-intervals AND
  `periodic_interior_overlap(u1, u2, TAU)`. Screen empty -> certified
  empty complex (`Method::Exact`); overlap -> the existing
  Region2/`CoincidentInterval` record.
- **(Plane, Plane)**: struct-unequal coplanar planes (the cell proved
  coincidence; the identity arm did not fire). Solve the parameter
  correspondence by Cramer in plane 1's frame:
  `subs1(u1, v1) = o1 + u1*U1 + v1*V1` with `U1 = p1 - o1`, `V1 = q1 - o1`
  (same for plane 2); with `n = plane1.normal()` and
  `det = (U1 x V1) . n`, the affine map
  `(u1, v1) = M (u2, v2) + c` has entries
  `M[0][0] = ((U2 x V1) . n) / det`, `M[0][1] = ((V2 x V1) . n) / det`,
  `M[1][0] = ((U1 x U2) . n) / det`, `M[1][1] = ((U1 x V2) . n) / det`,
  `c[0] = ((o2 - o1) x V1) . n / det`,
  `c[1] = (U1 x (o2 - o1)) . n / det`.
  If `M[0][1] == 0.0 && M[1][0] == 0.0` (the PARALLEL-frame signature -
  exactly zero for construction data whose frames are exact multiples), the
  image of box 2 is the axis-aligned rectangle
  `u1 in [c0 + M00*u2.0, c0 + M00*u2.1]` (min/max ordered by M00's sign)
  and likewise for v1; overlap = `interior_overlap` on both image
  intervals. If the off-diagonals are NOT exactly zero (rotated frames),
  do NOT screen: keep today's emission and leave the decision to the
  booked `BG-SOL-S7-OVERLAP-PLANE` follow-up (3-D SAT). Document this
  deferral in the module doc.
- **(Cone, Cone) / (Sphere, Sphere) / (Torus, Torus)**: same-type analytic
  Coincident implies the same surface and the same parameterization (the
  struct-unequal sphere/torus cases are unreachable - equal carriers hit
  the identity arm first); apply the carrier's identity-arm table as a
  defensive screen.

`analytic_ff`'s non-Coincident arms are untouched. The `Parallel`/`Empty`
arms already return empty complexes.

### 5. Order-insensitivity

Every screen is symmetric (interior tests and the cylinder z-test are
symmetric in their arguments; the plane Cramer map inverts under swapping -
the SAME overlap decision, not necessarily the same intermediate map). The
metamorphic property `C(A, B) = C(B, A)` must hold for every screened
path; the required test asserts it.

### 6. What does NOT change

`cover_branch`, `gff.rs`, `implicit.rs`, `fe_ee.rs`, the analytic cells,
`BoundedStratum`, `ContactLocus`, and the dispatch order. The existing
`contact_ff_coincident_planes_returns_coincident` test (unit boxes) keeps
passing unchanged - its boxes overlap. No existing test flips; if you find
one that does, STOP and report it in `disagreements` instead of editing the
test.

## Tests required

All dispatcher-level (`contact(...)`) with dyadic witnesses, in `mod.rs`'s
test module (or `overlap.rs`'s own test module for the helpers - your call,
but the six names below must exist as test functions):

1. `overlap_screen_identity_face_disjoint_boxes_certify_empty`: the same
   `Plane::xy()` carrier, boxes `(0,1)x(0,1)` vs `(2,3)x(2,3)` -> empty
   complex, `Method::Exact`, budget untouched; the same unit cylinder with
   v ranges `(0,1)` vs `(5,6)` -> empty.
2. `overlap_screen_identity_face_periodic_wrap_decides`: the same unit
   cylinder, u `(0.1, 0.2)` vs `(TAU - 0.1, TAU + 0.1)` (wraps past the
   seam onto `(0, 0.1) u (TAU-0.1, TAU)`) -> Coincident; u `(3.0, 3.1)` vs
   `(TAU - 0.1, TAU + 0.1)` -> empty. One sphere case exercising the v
   azimuth wrap.
3. `overlap_screen_same_axis_cylinder_shift_decides`: `Cylinder` center
   `(0,0,0)` r=1 and center `(0,0,5)` r=1 (struct-unequal, same wall).
   Disjoint: v `(0, 1)` on both -> absolute z `[0,1]` vs `[5,6]` -> empty
   complex. Overlapping: v `(4, 6)` on the first, v `(0, 1)` on the second
   -> absolute z `[4,6]` vs `[5,6]` -> Region2 `CoincidentInterval` record
   via the analytic path.
4. `overlap_screen_parallel_frame_planes_decide`: two struct-unequal
   coplanar `Plane::new` triples with parallel frames (e.g. one at origin
   with axes `(1,0,0),(0,1,0)`, the other with origin `(0,0,0)` and axes
   `(2,0,0),(0,2,0)`), parameter boxes chosen disjoint
   (`(0,1)x(0,1)` vs `(3,4)x(3,4)` in the second's units, mapping away
   from the first) -> empty; and a second pair of boxes that map into the
   first's interior -> Region2 record.
5. `overlap_screen_edge_disjoint_ranges_certify_empty`: the same `Line`
   edge with t `(0, 0.5)` vs `(0.5, 1.0)` (touching at the endpoint,
   interiors disjoint) -> empty; the same circle with disjoint arcs ->
   empty; overlapping arcs -> the existing Arc1/IdenticalCarrier record.
6. `overlap_screen_is_order_insensitive`: the shift and parallel-frame
   witnesses with the strata swapped produce the same outcome (empty stays
   empty, Coincident stays Coincident with the same dimension/kind).

Preserve every pre-existing test function name. H-3 rejects an added bare
`1e-N` unless the same line has a `// H-3` comment.

## Done when

```console
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --locked -p truck-evidence --all-targets
cargo test -p truck-evidence --lib contact --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`contact: parameter-overlap screen for coincident paths (BG-SOL-S7-OVERLAP)`)
**before** writing `RESULT.json`: the verifier measures the committed diff,
and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing outside `write_allow` (the analytic cells, `gff.rs`, `implicit.rs`,
`fe_ee.rs`, and `recognize.rs` are read-only); changing the Coincident
record shapes, certificates, or dispatch order; screening non-Coincident
analytic arms; adding interval/tolerance arithmetic (exact f64 only);
claiming rotated-frame coplanar planes are screened; adding `#[ignore]`;
loosening a gate; changing the GATE-4 ceiling; renaming or deleting a
pre-existing test.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- an existing test's assertion flips under the screen -> `disagreements`
  with the test name and both outcomes (do NOT edit the test);
- the parallel-frame Cramer map disagrees with a hand-checked dyadic
  witness -> `SPEC_GAP` with the computed entries;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes` the machine-checked Cramer entries for test 4's witness
(both M, c vectors) and the absolute-z arithmetic for test 3.
