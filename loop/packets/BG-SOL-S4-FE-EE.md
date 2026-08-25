# WORK PACKET BG-SOL-S4-FE-EE — Contact Layer strata reductions: FE (Edge×Face) and EE (Edge×Edge)

You are implementing the **strata-reduction stage** of the Contact Layer funnel
(`contact` module in `truck-evidence`, plan §4 Phase 3). The flagship
differential test `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)` (M2) drives the
Boundary Rewrite (Phase 4) from `contact()`, and the Boundary Rewrite must
split edges and faces at **edge-on-face** and **edge-on-edge** contacts. This
packet lands that stage: the bounded locus vocabulary, the FE and EE dispatch,
and the first analytic families. Everything you need is in this document.
**Do not read any other spec file** — this packet is self-contained.

```json
{"id":"BG-SOL-S4-FE-EE","status":"DONE","contracts":["BG-SOL-S4-FE-EE"],
 "tests_added":8,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S4-FE-EE
class:       design
crates:      [truck-evidence, truck-geometry, truck-base]
write_allow:
  - vendor/truck/truck-evidence/src/contact.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/fe_ee.rs
read_allow:
  - vendor/truck/truck-evidence/src/analytic/plane_plane.rs
  - vendor/truck/truck-evidence/src/analytic/plane_cylinder.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/cylinder.rs
  - vendor/truck/truck-base/src/contact.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - contact_fe_line_punctures_cylinder_wall_returns_point
  - contact_fe_line_in_plane_returns_coincident_arc
  - contact_fe_circle_on_plane_returns_coincident_arc
  - contact_fe_puncture_outside_bounds_returns_empty
  - contact_ee_line_circle_returns_point
  - contact_ee_coincident_lines_return_arc
  - contact_fe_ee_commutes
  - contact_fe_circle_latitudinal_on_cylinder_returns_coincident
budget:      {turns: 110, ctx_tokens: 260000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum ContactLocus' vendor/truck/truck-evidence/src/contact.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn contact' vendor/truck/truck-evidence/src/contact.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum BoundedStratum' vendor/truck/truck-evidence/src/contact.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn face_stratum' vendor/truck/truck-evidence/src/contact.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub enum CanonicalCurve' vendor/truck/truck-geometry/src/recognize.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'ContactReductionDeferred' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub enum ContactDimension' vendor/truck/truck-base/src/contact.rs"}
```

## Problem

The flagship differential test needs a 3-D Boolean on its RHS, and the Boolean
is the Boundary Rewrite, which iterates every boundary stratum pair. The
skeleton (BG-SOL-S3-CONTACT) landed the vocabulary and the two cheapest
dispatch stages (C0-C2 identity and the analytic FF table); every FE/EE pair
refuses with `ContactReductionDeferred`. This packet fills the next funnel
stage: for an edge and a face (FE), and two edges (EE), answer "where do they
meet and how", certified, bounded to both strata.

## Design decisions already made for you

### 1. Module shape: `contact.rs` becomes a directory module

The S3 skeleton lives in `vendor/truck/truck-evidence/src/contact.rs` (single
file, `pub mod contact;` in `lib.rs` — **do not touch `lib.rs`**). Convert it
to a directory module so the later funnel packets (cylinder×cylinder, general
validated FF, 2-D overlap) extend the Contact Layer without colliding on the
dispatcher file (plan §6: one module file per family):

- `git mv vendor/truck/truck-evidence/src/contact.rs
  vendor/truck/truck-evidence/src/contact/mod.rs` (or the equivalent: create
  `contact/` with `mod.rs` holding the existing file's content unchanged).
- `mod.rs` carries the existing vocabulary (`BoundedStratum`,
  `ContactComplex`, `ContactRecord`, `ContactLocus`), the `contact()`
  dispatcher, `face_stratum`, `analytic_ff`, `analytic_records`, and the
  existing tests, **all verbatim** (they are landed, verified, accepted code —
  do not rewrite them).
- Add `pub mod fe_ee;` to `mod.rs`.
- Put ALL new FE/EE machinery in the new
  `vendor/truck/truck-evidence/src/contact/fe_ee.rs`.
- Keep the H-1 deny header (`#![deny(clippy::unwrap_used, ...)]`) on `mod.rs`
  (it already carries it) and put the same deny header on `fe_ee.rs`.

The new `ContactLocus` arms below are added in `mod.rs`, next to the existing
arms.

### 2. The bounded locus forms (the vocabulary extension)

The landed `ContactLocus` is `{ Coincident, Analytic(AnalyticIntersection) }`
— C1/C2 identity and the *unbounded* analytic FF result. The FE/EE stage
needs the bounded forms. Add exactly these two arms:

```rust
pub enum ContactLocus {
    Coincident,                                        // unchanged
    Analytic(AnalyticIntersection),                    // unchanged
    /// An isolated contact point (FE punctures, EE crossings).
    Point(Point3),
    /// An exact curve clipped to a parameter range in the curve's own
    /// parameterization: an Arc1 coincident sub-arc (an edge lying on a face,
    /// overlapping collinear edges). `t_range` is on the curve's own
    /// parameter, so a `Line` sub-segment is `t_range ⊂ [0, 1]` on `subs(t) =
    /// a + t(b−a)` and a circle sub-arc is an angular interval on `[0, TAU)`.
    BoundedCurve { curve: ExactCurve, t_range: (f64, f64) },
}
```

`ExactCurve` is `truck_evidence::analytic::ExactCurve` (already imported in
`mod.rs`). **Do not change `ContactRecord { dimension, kind, locus }`** — the
parameter bookkeeping (t on the edge, `(u, v)` on the face, needed to split
edges and faces) is the Boundary Rewrite's (Phase 4), not this stage's. Record
that decision in your RESULT notes.

### 3. The dispatcher — extend `contact()`, in this order

The landed `contact()` stops at the first decided stage. Insert the strata
reductions between stage 2 (FF analytic) and stage 3 (the deferred funnel):

1. **C0-C2 identity** — unchanged (`Face/Face` equal carriers,
   `Edge/Edge` equal carriers).
2. **FF analytic** — unchanged.
3. **Strata reductions (NEW).**
   - `(BoundedStratum::Edge { .. }, BoundedStratum::Face { .. })` →
     `fe_ee::fe_contact(edge, face, budget)`.
   - `(BoundedStratum::Face { .. }, BoundedStratum::Edge { .. })` →
     `fe_ee::fe_contact(edge, face, budget)` (the same solver, arguments
     swapped so the solver always sees `(edge, face)`).
   - `(BoundedStratum::Edge { .. }, BoundedStratum::Edge { .. })` →
     `fe_ee::ee_contact(lhs, rhs, budget)`.
4. **Everything else** — unchanged: the deferred funnel
   (`ContactReductionDeferred`) for any pair involving a `Vertex`, any FF pair
   that is `Torus`/`Placed` or outside the §3.3 table, and unrecognized
   carriers at the `face_stratum` lift boundary.

The FE and EE solvers are commutative: `contact(A, B)` and `contact(B, A)`
must produce the same `ContactComplex` (the metamorphic property, plan §2).
The records carry no lhs/rhs parameter fields, so the symmetry is structural;
`contact_fe_ee_commutes` proves it.

### 4. The FE analytic table — what this packet solves, and what it defers

The FE solver `fe_contact(edge, face, budget)` dispatches on the carrier pair.
Implement **exactly** this table; anything not listed returns the
`ContactReductionDeferred` refusal.

| edge carrier | face carrier | implementation |
|---|---|---|
| `CanonicalCurve::Line` | `CanonicalSurface::Plane` | linear solve (§5.1) |
| `CanonicalCurve::Line` | `CanonicalSurface::Cylinder` | quadratic solve + generator coincident (§5.2) |
| `CanonicalCurve::Circle` | `CanonicalSurface::Plane` | chord solve + coincident arc clip (§5.3) |
| `CanonicalCurve::Circle` | `CanonicalSurface::Cylinder` | latitudinal coincident only (§5.4) |

Deferred (return `Err(Refusal::UnsupportedEnvelope(
EnvelopeCase::ContactReductionDeferred))`, note each in RESULT notes as a
documented follow-up): `Line`×`Cone`, `Line`×`Sphere` (the quadratic pattern
of §5.2 generalizes; a follow-up fills them), `Circle`×`Cone`,
`Circle`×`Sphere` (circle×sphere reduces to a 2-D circle-circle after
restricting the sphere to the circle's plane — the follow-up), `Circle`×
`Cylinder` **transverse** (a circle puncturing the wall is conic×conic, the
general validated solver's job), and any face that is `Torus`/`Placed`.

The edge `t_range` is the interval on the edge's own canonical parameter
(Line: `[0, 1]` in `subs(t) = a + t(b−a)`; Circle: an angular interval on
`[0, TAU)`). The face `(u, v)` box is `u_range`/`v_range` on the canonical
surface's parameterization. **Every reported point or arc must lie within BOTH
strata's bounds** (the edge's `t_range` and the face's box). A contact point
at a stratum boundary is `EndpointTouch`; strictly inside both is `Transverse`;
the degenerate quadratic discriminant (`== 0`) is `Tangency`.

### 5. The FE solvers — the algorithms

House exactness discipline (BG-ANA-002, copy it verbatim): every classification
predicate is decided by **decisive interval enclosures** of quantities derived
from the f64 carrier parameters (`decisively_zero`: the interval is degenerate
`[0, 0]`; `excludes_zero`: the interval lies strictly away from 0; a
straddling enclosure is `Err(Refusal::NumericallyUnresolved { spent:
Budget::new(0, 0, 0), witness: UnresolvedWitness::RootNotIsolated })`, never a
guess). Copy the `interval_at` / `decisively_zero` / `excludes_zero` helpers
from `plane_plane.rs` (they are in `analytic/plane_plane.rs`, read-only).
Emitted coordinates are f64 closed forms; the certificate's obligation is
"lies on both carriers to machine precision", asserted in tests with an
H-3-commented slack — never dyadic-exact claims about root coordinates.

#### 5.1 Line × Plane

Edge line `a + t·(b − a)` over `t ∈ [0, 1]`, direction `d = b − a`. Face plane
with normal `n`, origin `o`. Let `denom = d·n`, `num = (o − a)·n`.

- `denom` decisively nonzero: the line meets the plane at `t0 = num / denom`.
  If `t0` is decisively outside `edge.t_range` → empty. Else compute
  `q = a + t0·d`, and the face-box check: `plane.get_parameter(q)` gives
  `(u, v, w)` (`w` is the normal offset, exactly 0 here); `q` is on the face
  iff `u ∈ u_range && v ∈ v_range`. Inside → one `Point(q)` record
  (`Transverse`, or `EndpointTouch` if `t0` equals a `t_range` endpoint or the
  point is on a face-box boundary). Outside → empty.
- `denom` decisively zero: the line is parallel to the plane.
  - `num` decisively zero: the line lies IN the plane → the coincident Arc1.
    Clip: map the line into the face's `(u, v)` coordinates
    (`get_parameter` on two points), giving affine `(u(t), v(t))`; the face
    box is `u(t) ∈ u_range && v(t) ∈ v_range`, each an interval in `t`;
    intersect both with `edge.t_range`. If the result `[t_lo, t_hi]` is
    decisively nonempty (allow inclusive endpoints) → one `BoundedCurve {
    curve: ExactCurve::Line(line), t_range: (t_lo, t_hi) }` record, Arc1,
    `CoincidentInterval`. Empty → empty `ContactComplex`.
  - `num` decisively nonzero: parallel, no contact → empty.
- Straddling `denom` or `num` → `NumericallyUnresolved`.

#### 5.2 Line × Cylinder

Canonical cylinder: `center c`, `radius r`, axis ẑ. Edge line `a + t·d` over
`t ∈ [0, 1]`. The on-surface equation is
`(ax + t·dx − cx)² + (ay + t·dy − cy)² = r²`.

- `dx` and `dy` both decisively zero (the line is parallel to the axis): the
  radial distance test. `rho = (ax − cx)² + (ay − cy)²` vs `r²` decisive:
  - `rho == r²` (decisively): the line is a **generator**, coincident Arc1.
    Clip by `v = az + t·dz − cz ∈ v_range` (an interval in `t`), intersect
    with `edge.t_range`; nonempty → `BoundedCurve { curve: Line, t_range }`,
    Arc1, `CoincidentInterval`.
  - `rho ≠ r²` (decisively): no contact → empty.
- Otherwise the quadratic in `t`: `a_q = dx² + dy²` (decisively positive),
  `b_q = 2((ax−cx)dx + (ay−cy)dy)`, `c_q = (ax−cx)² + (ay−cy)² − r²`.
  Discriminant `D = b_q² − 4·a_q·c_q`, decided by decisive enclosure:
  - `D` decisively < 0 → empty.
  - `D` decisively == 0 → one tangent root `t0 = −b_q / (2·a_q)`: point
    `q`, `Point(q)`, `Tangency`, subject to the bounds below.
  - `D` decisively > 0 → two roots `t = (−b_q ± √D) / (2·a_q)`: each point
    `Point(q)`, `Transverse`, subject to the bounds below.
  - `D` straddling zero → `NumericallyUnresolved`.
  For every candidate root: `t ∈ edge.t_range` (else drop), and the face-box
  check for the cylinder: `u = atan2(qy − cy, qx − cx)` wrapped into `[0, 2π)`
  (`if u < 0.0 { u += TAU }`), `v = qz − cz`; the point is on the face iff
  `u ∈ u_range && v ∈ v_range`. Dropped-by-bounds candidates contribute
  nothing; if all are dropped → empty. (For the seam: a wrapped `u` in
  `[0, 2π)` against a `u_range` that spans `[0, TAU)` is fine; a partial
  `u_range` crossing the seam is out of scope this packet — note it.)

#### 5.3 Circle × Plane

The circle edge is `CanonicalCurve::Circle(placed)`, a
`Processor<TrimmedCurve<UnitCircle>, Matrix4>` whose transform columns are the
in-plane axes `x`, `y` (both of length `r` for a circle), the plane normal `z`
and the center `w`. Build the circle's carrier plane
`Plane::new(center, center + x, center + y)` (normal `n_c`, aligned
parameterization) and call the landed `plane_plane(circle_plane, face_plane)`:

- **`Coincident`**: the circle lies in the face plane → the coincident Arc1.
  Clip to the face box: in the circle's parameterization `θ ∈ [0, TAU)`, the
  face box's four boundary lines (the `(u, v)` box of the face plane, mapped
  into the circle's plane) cut the circle in up to 8 crossing angles; the
  contained angular intervals are the maximal `θ`-intervals whose midpoint is
  inside the box. Each nonempty contained interval, intersected with the
  edge's `t_range`, becomes a `BoundedCurve { curve: ExactCurve::Circle(..),
  t_range }` record, Arc1, `CoincidentInterval`. (A whole circle inside the box
  is the single interval `[0, TAU)`.)
- **`Curve(ExactCurve::Line(line))`**: the two planes meet in a line; the
  circle ∩ line is the chord solve: project the line into the circle's plane
  (in-plane coordinates), solve the quadratic `|q(σ) − center|² = r²` over the
  line parameter `σ`, decisive discriminant → 0/1/2 points. Each point must be
  in the face box (`plane.get_parameter` on the face plane) AND on the circle's
  full edge (a circle edge is the whole `[0, TAU)`) → `Point(q)` records
  (`Transverse`, `Tangency` on the degenerate discriminant). Outside the box →
  drop.
- **`Parallel`** → empty.
- **`Empty`** → empty.
- Any other arm (e.g. `NumericallyUnresolved` propagated from `plane_plane`) →
  propagate it (a stop, not a guess).

#### 5.4 Circle × Cylinder — latitudinal coincident only

The circle is latitudinal on the cylinder iff (all decisive predicates):
- the circle's plane normal is parallel to the axis: `|n_c × ẑ|` decisive
  zero;
- the circle's center is on the axis: `(center − c).x` and `(center − c).y`
  decisive zero;
- the radii agree: `|r_c − r|` decisive zero;
- the circle's height `zc = center.z` is within the face's `v_range`.

Then the coincident locus is the sub-arc of the circle within the face's
`u_range` (the wall's `u` IS the circle's angle): `t_range =
[0, TAU) ∩ u_range` (a partial `u_range` gives the sub-arc `[u0, u1]`; a full
`[0, TAU)` gives the whole circle), reported as `BoundedCurve { curve:
ExactCurve::Circle(..), t_range }`, Arc1, `CoincidentInterval`. Any
non-latitudinal circle × cylinder → `ContactReductionDeferred` (transverse is
conic×conic).

### 6. The EE analytic table

`ee_contact(lhs, rhs, budget)` dispatches on the two edge carriers.

| lhs | rhs | implementation |
|---|---|---|
| `Line` | `Line` | §6.1 |
| `Line` | `Circle` | §6.2 |
| `Circle` | `Line` | §6.2 (same solver, order-insensitive) |
| `Circle` | `Circle` | deferred: `ContactReductionDeferred` (3-D two-circle; coplanar + non-coplanar both documented follow-ups) |

#### 6.1 Line × Line

Edge lines `a0 + t0·d0` over `t0 ∈ [0, 1]` and `a1 + t1·d1` over `t1 ∈ [0, 1]`.
Let `c = d0 × d1`.

- `c` decisively nonzero (not parallel). Coplanarity `T = c·(a1 − a0)`:
  - decisively `≠ 0`: skew → empty.
  - decisively `== 0`: coplanar. Solve `s·d0 − t·d1 = a1 − a0` by the 2×2
    system in the two dot products `d0·d0`, `d0·d1`, `d1·d1` (determinant
    `|d0|²|d1|² − (d0·d1)² = |c|²` decisive nonzero). The intersection point
    `q`; if `s ∈ lhs.t_range && t ∈ rhs.t_range` → `Point(q)`, `Transverse`
    (or `EndpointTouch` at a boundary). Else empty.
  - straddling `T` → `NumericallyUnresolved`.
- `c` decisively zero (parallel). `(a1 − a0) × d0` decisive:
  - zero: collinear → coincident Arc1. Express the overlap in the lhs line's
    parameter: `t_base = (a1 − a0)·d0 / |d0|²`, and the rhs segment spans
    `[t_base, t_base + |d1|/|d0|]` (direction sign from `d1·d0`). The overlap
    `[max(0, lo), min(1, hi)]` (lhs is `[0, 1]`); nonempty → `BoundedCurve {
    curve: ExactCurve::Line(lhs_line), t_range: overlap }`, Arc1,
    `CoincidentInterval`. Empty → empty.
  - nonzero: parallel, no contact → empty.
  - straddling → `NumericallyUnresolved`.
- `c` straddling zero → `NumericallyUnresolved`.

#### 6.2 Line × Circle

Edge line `a + t·d` over `t ∈ [0, 1]`; circle with center `m`, plane normal
`n` (from `z` column), in-plane unit axes `u`, `v` (from `x`/`y` columns
normalized), radius `r`.

- `d·n` decisively nonzero (line meets the circle's plane once): `t0 =
  ((m − a)·n)/(d·n)`; if `t0` outside `lhs.t_range` → empty. Point
  `q = a + t0·d` lies in the plane; the on-circle test is the in-plane radius
  check `(q−m)·u` and `(q−m)·v` (since `q − m` has no normal component,
  `|q−m|² = ((q−m)·u)² + ((q−m)·v)²`): decisive `== r²` → `Point(q)`,
  `Transverse`; decisive `≠ r²` → empty.
- `d·n` decisively zero (line parallel to the circle's plane). In-plane offset
  `h = (a − m)·n`:
  - decisive `== 0`: the line is in the plane → 2-D chord. Quadratic in `t`
    from `|a + t·d − m|² = r²`; decisive discriminant → 0/1/2 roots; each
    `t ∈ lhs.t_range` → `Point(q)` (`Transverse`, `Tangency` on the
    degenerate discriminant).
  - decisive `≠ 0`: parallel plane, no contact → empty.
  - straddling → `NumericallyUnresolved`.
- `d·n` straddling zero → `NumericallyUnresolved`.

### 7. Certificate construction

Identical to the landed stage 1/2 pattern in `mod.rs`: every `Ok` carries an
explicit field-by-field `Certificate { props: {Prop::AnalyticCarrier: True},
method: Method::Exact, budget_left: *budget, margin: Margin::UNBOUNDED,
modulus: Modulus::Unbounded }`; nothing is spent from `budget` (no subdivision
happens anywhere in this packet). `Empty` outcomes are `Ok(Certified::new(
ContactComplex { contacts: vec![] }, <same certificate>))` — a decided "no
contact" is a certified answer, not a refusal. The only `Err` arms are the
deferred-funnel `ContactReductionDeferred` and `NumericallyUnresolved` for
straddling predicates.

### 8. Tests (in `contact/mod.rs` tests, plus what belongs to the solvers in `fe_ee.rs`)

House rule: GATE-1 requires `#![deny(clippy::unwrap_used)]` on every new test
file — `fe_ee.rs` carries the module header, and the test modules carry
`#[allow(clippy::unwrap_used, clippy::expect_used)]` exactly as the analytic
modules do. Build canonical carriers via `recognize_curve`/`recognize_surface`
or directly. Use dyadic witnesses (integer/half coordinates) everywhere; assert
with H-3-commented slacks, never bare `1e-N` literals without `// H-3` on the
same line. The eight required tests:

1. `contact_fe_line_punctures_cylinder_wall_returns_point` — a vertical line
   edge crossing a cylinder wall (e.g. edge `(2, 0, −1)→(2, 0, 2)` against the
   unit cylinder centered at the origin, `t_range = [0, 1]`) → exactly one
   record, `Point0`, `Transverse`, `ContactLocus::Point` inside the cylinder's
   `v_range`.
2. `contact_fe_line_in_plane_returns_coincident_arc` — a line edge lying in a
   plane face, clipped to the face box → one record, `Arc1`,
   `CoincidentInterval`, `ContactLocus::BoundedCurve` with a `t_range` strictly
   inside the face box (and inside the edge's own `t_range`).
3. `contact_fe_circle_on_plane_returns_coincident_arc` — a cap circle (unit
   circle at `z = 0`) lying in the `xy`-plane face → one record, `Arc1`,
   `CoincidentInterval`, the whole circle `[0, TAU)` when the face box contains
   it.
4. `contact_fe_puncture_outside_bounds_returns_empty` — the same
   line×cylinder geometry with the edge's `t_range` cut so the puncture is
   outside it → `Ok` with an empty `contacts` vec.
5. `contact_ee_line_circle_returns_point` — a vertical line edge crossing a
   cap circle (e.g. the vertical line `x = 2, y = 0` vs the unit circle in
   `z = 0`) → one record, `Point0`, `Transverse`.
6. `contact_ee_coincident_lines_return_arc` — two collinear overlapping edge
   segments on the same line → one record, `Arc1`, `CoincidentInterval`, with
   the overlap's `t_range`.
7. `contact_fe_ee_commutes` — for one FE pair (line×cylinder) and one EE pair
   (line×line coplanar), assert `contact(A, B)` and `contact(B, A)` produce
   structurally equal `ContactComplex` values (compare the `(dimension, kind,
   locus)` records).
8. `contact_fe_circle_latitudinal_on_cylinder_returns_coincident` — a unit
   circle at `z = 1` against the unit cylinder wall face with `v_range`
   containing `1` → one record, `Arc1`, `CoincidentInterval`.

Also keep the landed S3 tests green (they are untouched), and add at least one
negative test that the still-deferred families still refuse with
`ContactReductionDeferred` (e.g. a Circle×Circle EE pair, or a Circle×Cone
FE pair — your choice; name it `contact_deferred_families_still_refuse`).

## Done-when gates

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --locked -p truck-evidence --all-targets
cargo check --locked -p truck-base --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Never run `cargo check --workspace` — it
exhausts disk on a shared machine.

## H-3 / GATE-4

GATE-2 rejects added lines carrying bare `1e-N` literals unless the line ends
with `// H-3`. Float tolerances in test assertions must carry `// H-3` on the
same line. This packet adds NO `unscaled_legacy()` calls; do not touch
`scripts/unscaled_legacy_ceiling.txt` (GATE-4 stays at 111).

## Forbidden

Editing any file outside `write_allow`. Editing `truck-evidence/src/lib.rs`
(the `pub mod contact;` declaration must stay), any analytic pair module, any
topology/modelling file. Implementing the deferred funnel stages (general
validated FF, singular event cells, 2-D overlap), the deferred FE/EE families
(`Line`×`Cone`/`Sphere`, `Circle`×`Cone`/`Sphere`, `Circle`×`Cylinder`
transverse, `Circle`×`Circle`), or vertex-strata contact. Changing the `(
dimension, kind, locus)` shape of `ContactRecord`. Adding `#[ignore]`. Changing
the GATE-4 ceiling. Running `cargo check --workspace` / `cargo build
--workspace`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Record in
`notes`: the module move (`contact.rs` → `contact/mod.rs`, `lib.rs` untouched),
the two new `ContactLocus` arms, the FE/EE table actually landed vs deferred
(with the deferral list), the bounded-bounds checks per surface type (including
the cylinder `u`-wrap into `[0, 2π)`), the certificate shape, and your read of
whether any in-scope representation was infeasible.

Commit on the current branch with subject
`feat(evidence): Contact Layer strata reductions — FE (Edge×Face) and EE (Edge×Edge) with bounded loci (BG-SOL-S4-FE-EE)`.
