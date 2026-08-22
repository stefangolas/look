# WORK PACKET BG-ENC-004-ISC — `EnclosureCurve` for `IntersectionCurve`

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-004-ISC","status":"DONE","contracts":["BG-ENC-004"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were derived
by command against the tree and validated by compiling and **RUNNING** the whole
certification in a scratch crate against the real carriers (sphere-sphere,
plane-sphere, and a degenerate negative; every formula in this packet ran and
certified before it was written down), but they are exactly the kind of claim
that can be confidently wrong. **If anything below contradicts what you find in
the code, say so in `disagreements` rather than making the code match the
packet.**

```yaml
id:          BG-ENC-004-ISC
contract:    [BG-ENC-004]
class:       mechanical
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/intersection_curve.rs
read_allow:
  - vendor/truck/truck-geometry/src/decorators/intersection_curve.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/decorators/pcurve.rs
  - vendor/truck/truck-evidence/src/decorators/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/bspline.rs
  - vendor/truck/truck-evidence/Cargo.toml
tests_required:
  - isc_encloses_sampled_sphere_sphere
  - isc_plane_sphere_slice_is_tight
  - isc_identical_surfaces_refuse_whole
  - isc_out_of_range_span_is_unbounded
  - isc_der1_contains_finite_differences
  - isc_der_above_one_is_unbounded
  - isc_tangent_cone_contains_exact_circle
budget:      {turns: 36, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'BG-ENC-004-ISC' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn search_triple' vendor/truck/truck-geometry/src/decorators/intersection_curve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn double_projection' vendor/truck/truck-geometry/src/decorators/intersection_curve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub trait EnclosureCurve' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn exact_spline' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub mod intersection_curve' vendor/truck/truck-evidence/src/decorators/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'impl<S: EnclosureSurface<Vector = Vector3>> EnclosureCurve for PCurve' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A8, expect: 0, cmd: "grep -c 'unscaled_legacy(' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
  - {id: A9, expect: 9, cmd: "grep -c 'Interval::ENTIRE' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'const HULL_PAD' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A11, expect: 1, cmd: "grep -c 'fn assert_encloses_curve' vendor/truck/truck-evidence/src/harness.rs"}
  - {id: A12, expect: 2, cmd: "grep -c 'MAX_HALF_ANGLE' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
```

(`grep -c` exits 1 on zero matches — a count of 0 IS the expected answer for
A8, not a command failure.)

## Problem

`IntersectionCurve<C, S0, S1>` (truck-geometry, read-only to you) is the curve a
transversal surface-surface intersection produces. `subs(t)` runs
`search_triple`: a 4-variable Newton solve on the unknowns
`q = (x, y, z, w) = ((x,y) on S0, (z,w) on S1)` for the system

```
F(t; q) = [ S0(x,y) − S1(z,w) ;  L'(t) · ( (S0(x,y) + S1(z,w))/2 − L(t) ) ]
```

(a 3-vector equation plus one scalar plane equation; `L` is the leader curve,
`L'` its derivative), returning `midpoint(S0(x,y), S1(z,w))`. The leader is an
approximation, so the leader hull alone under-estimates — BG-ENC-001's silent
wrong answer. The sound enclosure needs the **certified parameter images**: a
box Q₀ × Q₁ (parameter boxes on each surface) that provably contains the
system's solution for **every** t in the span. The 3D enclosure is then pure
composition: `midpoint(S0.enclose(Q₀), S1.enclose(Q₁))`.

The certificate that produces Q is a **parametric Krawczyk operator** — for
every t in a t-cell, existence AND uniqueness of the solution in Q — evaluated
in interval arithmetic over the landed `EnclosureSurface`/`EnclosureCurve`
impls. Everything below was measured against the real carriers before this
packet was written:

- sphere-sphere (two √2-spheres at (0,0,±1), unit-circle intersection, 8- and
  16-segment chord leaders): 6–12 cells per span, **0.3–2.6 ms per `enclose`**,
  0 containment escapes of `subs` on 100-point grids;
- plane-sphere (plane z = 0.3 cutting the unit sphere): the slice's z-width
  certifies to ±1.1e-6;
- the degenerate negative (identical spheres — the system rank-deficient
  everywhere): certification honestly fails and `enclose` returns the unbounded
  box;
- float `search_triple` results over 200-point grids: 0 parameter escapes from
  the certified boxes.

The module `vendor/truck/truck-evidence/src/decorators/intersection_curve.rs`
is scaffolded with a doc comment recording this blocker; you replace its
content (keep a module doc comment in the house style — see decision 9).

## Decisions already made for you

### 0. The impl block, verbatim bounds

Replace the scaffold with the impl. Exact bounds (the carrier's own
`ParametricCurve` impl uses the same supertraits — mirror them):

```rust
use crate::enclosure::{Box3, DirCone, EnclosureCurve, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::{Matrix4, Point3, Vector3, Vector4};
use truck_geometry::decorators::IntersectionCurve;
use truck_geotrait::traits::ParametricCurve3D; // adjust to the crate's actual
use truck_geotrait::traits::ParametricSurface3D; // import path if it differs
use truck_geotrait::{D2, ParametricCurve, ParametricSurface, SearchNearestParameter};

impl<C, S0, S1> EnclosureCurve for IntersectionCurve<C, S0, S1>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3>
        + ParametricCurve3D
        + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    // decisions 3–6
}
```

If the `ParametricCurve3D`/`ParametricSurface3D`/`D2` imports resolve
differently in-crate (they are re-exported at the crate root of
truck-geotrait), adapt the `use` lines, not the bounds. `InnerSpace` and
`SquareMatrix` (for `.magnitude()` and `Matrix4::invert()`) come from
`truck_base::cgmath64` when the tests need them. Do NOT override
`exact_spline` — the default `None` is correct (an intersection curve is not
exactly a spline).

### 1. The system, in intervals — and the sign convention that costs a day

A private helper struct holds the three references:

```rust
struct Sys<'a, C, S0, S1> { leader: &'a C, s0: &'a S0, s1: &'a S1 }
```

with the same bounds as the impl (freestanding `where` clause). Three methods.
`f_iv` — the interval F over parameter boxes and the t-cell:

```rust
fn f_iv(&self, xb: Interval, yb: Interval, zb: Interval, wb: Interval, tt: Interval)
    -> [Interval; 4]
{
    let p0 = self.s0.enclose(xb, yb);
    let p1 = self.s1.enclose(zb, wb);
    let d = Box3 { x: p0.x - p1.x, y: p0.y - p1.y, z: p0.z - p1.z };
    let l = self.leader.enclose(tt);
    let n = self.leader.enclose_der(1, tt);
    let half = interval_at(0.5);
    let m = Box3 {
        x: (p0.x + p1.x) * half - l.x,
        y: (p0.y + p1.y) * half - l.y,
        z: (p0.z + p1.z) * half - l.z,
    };
    let f4 = dot3(&n, &m);
    [d.x, d.y, d.z, f4]
}
```

`j_iv` — the interval Jacobian. **Storage convention: the returned array is
`j[param][equation]` (column-major), because the natural construction builds
one array per parameter column.** The algebra in decision 2 knows this.

**The sign convention (copy the carrier exactly, `double_projection`'s
`from_cols` call): the S1 columns negate the 3-D part ONLY; the fourth
component keeps `+n·d/2`.** The carrier writes
`(-uder1).extend(plane_normal.dot(uder1) / 2.0)` — negating the whole Vector4
here makes the two azimuthal columns exactly parallel on symmetric witnesses,
the float Jacobian singular (measured det ≈ 5.5e-17), and nothing certifies:

```rust
fn j_iv(&self, xb: Interval, yb: Interval, zb: Interval, wb: Interval, tt: Interval)
    -> [[Interval; 4]; 4]
{
    let n = self.leader.enclose_der(1, tt);
    let half = interval_at(0.5);
    let u0 = self.s0.enclose_der(1, 0, xb, yb);
    let v0 = self.s0.enclose_der(0, 1, xb, yb);
    let u1 = self.s1.enclose_der(1, 0, zb, wb);
    let v1 = self.s1.enclose_der(0, 1, zb, wb);
    let col = |d: &Box3| {
        [d.x, d.y, d.z, (n.x * d.x + n.y * d.y + n.z * d.z) * half]
    };
    let col_neg = |d: &Box3| {
        [-d.x, -d.y, -d.z, (n.x * d.x + n.y * d.y + n.z * d.z) * half]
    };
    [col(&u0), col(&v0), col_neg(&u1), col_neg(&v1)]
}
```

`j_fl` — the float Jacobian at a point, mirroring the same convention:

```rust
fn j_fl(&self, q: [f64; 4], t: f64) -> Option<Matrix4> {
    let n = self.leader.der(t);
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let u0 = self.s0.der_mn(1, 0, x, y);
    let v0 = self.s0.der_mn(0, 1, x, y);
    let u1 = self.s1.der_mn(1, 0, z, w);
    let v1 = self.s1.der_mn(0, 1, z, w);
    let colf = |d: Vector3| Vector4::new(d.x, d.y, d.z, n.dot(d) / 2.0);
    let colf_neg = |d: Vector3| Vector4::new(-d.x, -d.y, -d.z, n.dot(d) / 2.0);
    Some(Matrix4::from_cols(colf(u0), colf(v0), colf_neg(u1), colf_neg(v1)))
}
```

(`der_mn(m, n, u, v)` is u-order m, v-order n; `n.dot(d)` needs
`InnerSpace` in scope — `use truck_base::cgmath64::InnerSpace;`. The `Option`
is for total-behaviour symmetry with `invert`; this body cannot fail — return
`Some` directly.)

Private interval helpers the module needs (duplicate them here; the sibling
decorators duplicate theirs for disjoint write sets): `interval_at` (the
crate's standard `Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)`),
`dot3(a: &Box3, b: &Box3) -> Interval` (`a.x*b.x + a.y*b.y + a.z*b.z`),
`cross3(a: &Box3, b: &Box3) -> Box3` (componentwise interval cross), and
`unbounded_box() -> Box3` (an `Interval::ENTIRE` per axis, pcurve.rs's
`unbounded_box` copied with its doc comment).

### 2. The parametric Krawczyk operator

Interval 4-vector/4-matrix algebra, private to the module: `type IVec4 =
[Interval; 4]; type IMat4 = [[Interval; 4]; 4];` plus

- `row4(y: &Matrix4, r: usize) -> [f64; 4]` — row r of a cgmath Matrix4 by
  explicit match on `(r, c)` (H-1 bans indexing; a `match` is total).
- `y_times_fvec(y, f: &IVec4) -> IVec4` — `Σ_k row4(y,r)[k]·f[k]`.
- `y_times_imat(y, j: &IMat4) -> IMat4` — **`out[r][c] = Σ_k row4(y,r)[k] *
  j[c][k]`** — note `j[c][k]`, NOT `j[k][c]`: the storage is column-major
  (`j[param][equation]`), the output is row-major. Getting this transposed
  computes `Y·Jᵀ`; its widths look healthy and its centers are O(1) off — the
  failure is silent and total.
- `identity_minus(a: &IMat4) -> IMat4`, `imat_times_ivec(a, v) -> IVec4` —
  row-major, straightforward.

The certification for ONE t-cell:

```rust
fn certify_cell(sys, cell: Interval, q_lo: [f64; 4], q_hi: [f64; 4], t_mid: f64)
    -> Option<[Interval; 4]>
```

(freestanding, same bounds as the impl). Loop `for step in 0..MAX_INFLATIONS`
(24), growing `pad` from `INITIAL_PAD` (1.0e-6, a named const with its H-3
comment) by `GROWTH = 4.0` each step:

1. `Q`: per axis, hull of `q_lo[a]`, `q_hi[a]` widened by
   `pad * (1.0 + max(|lo|, |hi|))` on each side. `m` = float midpoints.
2. `y = sys.j_fl(m, t_mid)?.invert()?` — any `None` fails the cell (return
   `None`; the caller bisects).
3. **The center term is a POINT evaluation** — `f = sys.f_iv` at the four
   degenerate intervals `interval_at(m[a])` and `interval_at(t_mid)`, NEVER at
   the boxes `Q`/`cell`: the interval F over Q drags the `p0−p1` decorrelation
   (two copies of the solution arc's width) into the center and doubles the
   linear part against the contraction term; with it, no box ever certifies
   (measured: K ≥ 5·width(Q) at every scale, on every witness).
4. `j = sys.j_iv` at the boxes `Q`, `cell` (the t-dependence enters here,
   soundly, as the interval over the cell).
5. `K = m − y_times_fvec(y, &f) + imat_times_ivec(identity_minus(&y_times_imat(y, &j)), &qminusm)`
   elementwise, with `qminusm[a] = q[a] − interval_at(m[a])`.
6. Certify iff **strict** interior containment on all four axes:
   `k[a].inf() > q[a].inf() && k[a].sup() < q[a].sup()` (strict — non-strict
   proves existence, not uniqueness). Return `Some(q)`.

### 3. `enclose` — cells, seeding, refusal

Worklist over t-cells (`Vec<Interval>` stack):

1. **Knot-aligned initial cells.** If `self.leader().exact_spline()` is
   `Some(bsp)`, split `tt` at every interior knot value strictly inside
   `(tt.inf(), tt.sup())` (iterate `bsp.knot_vec().iter()`, sort the cut list,
   take `windows(2)`). A cell straddling a leader knot sees the kink's
   derivative fan in `leader.enclose_der(1, ·)` and cannot certify. A leader
   without an exact spline gets one cell covering `tt` and pays bisection
   instead.
2. Per cell, `t_mid = (inf+sup)/2`; seed with
   `self.search_triple(t, 100)` (pub method on the carrier; returns
   `Option<(Point3, Point2, Point2)>` — map to `[uv0.x, uv0.y, uv1.x, uv1.y]`)
   at **both endpoints** of the cell; if either fails, fall back to the
   midpoint seed; if that fails too, bisect the cell while its half-width
   exceeds `f64::EPSILON`, else return the unbounded box for the whole call.
3. `certify_cell(...)`. On `Some(q)`: compose the cell's 3-D box

   ```rust
   let p0 = self.surface0().enclose(q[0], q[1]);
   let p1 = self.surface1().enclose(q[2], q[3]);
   let half = interval_at(0.5);
   let b = Box3 {
       x: (p0.x + p1.x) * half, y: (p0.y + p1.y) * half, z: (p0.z + p1.z) * half,
   };
   ```

   then widen per axis by `NEWTON_PAD * (1.0 + |mid|)` with
   `const NEWTON_PAD: f64 = 64.0 * f64::EPSILON;` — the float-evaluation guard
   for `subs`'s float `S0.subs/S1.subs` at parameters the certificate proved
   inside `Q` (measured: the float Newton results sit inside the certified
   boxes on every grid run; this pad covers the ulp-class float-image drift,
   exactly HULL_PAD's epistemic status — name it in the const's doc comment).
   Hull the cell boxes together per axis (`Interval::convex_hull`).
4. On `None`: bisect the cell while its half-width exceeds `CELL_FLOOR`
   (1.0e-12, named const, H-3 comment); at the floor, return the unbounded
   box. Cap the worklist at 512 processed cells → unbounded box. Every
   refusal is the unbounded box — sound (over-estimation is always
   acceptable), honest, panic-free (H-1).

Measured on the witnesses: certification succeeds at inflation steps 5–7
(pad 1e-3 … 1.6e-2), 6–12 cells per span, 0.3–2.6 ms per call (dev profile).

### 4. `enclose_der` — n = 0 and n = 1 composed, n ≥ 2 refused

`n == 0` → `self.enclose(tt)`. `n >= 2` → `unbounded_box()`: the carrier's
`ders` recursion differentiates the 4×4 system implicitly per order and
composing that in intervals is not derived — a sound widest box is the honest
answer (the module docs say so, citing the PCURVE fourth-order precedent).

`n == 1` — re-derive the certification per cell (deterministic; same cost as
`enclose`), then compose the carrier's own `der` formula in intervals. The
carrier computes `der(t) = n̂ · k` with `n̂ = n0 × n1` (surface normals' cross)
and `k = (|L'|² − (c − L)·L'') / (n̂ · L')`. In intervals over the certified
`q`, `cell`:

```rust
let u0 = self.surface0().enclose_der(1, 0, q[0], q[1]);
let v0 = self.surface0().enclose_der(0, 1, q[0], q[1]);
let u1 = self.surface1().enclose_der(1, 0, q[2], q[3]);
let v1 = self.surface1().enclose_der(0, 1, q[2], q[3]);
let nbox = cross3(&cross3(&u0, &v0), &cross3(&u1, &v1));
let c = /* the cell's 3-D box from decision 3, step 3 */;
let l  = self.leader().enclose(cell);
let l1 = self.leader().enclose_der(1, cell);
let l2 = self.leader().enclose_der(2, cell);
let num = dot3(&l1, &l1) - dot3(&sub3(&c, &l), &l2);
let den = dot3(&nbox, &l1);
```

`sub3` is a per-axis interval subtraction helper (add it to decision 1's
helpers). If `den` contains `0.0` (`den.inf() <= 0.0 && den.sup() >= 0.0`)
the leader's tangent lies in the constraint plane and the parametrization
degenerates — the whole `der1` result is the unbounded box (the family's
`None` condition arriving numerically; inari's division would return ENTIRE
anyway, but check explicitly). Otherwise `k = num / den`, the cell's box is
`Box3 { x: nbox.x * k, y: nbox.y * k, z: nbox.z * k }`, hull across cells.
Cells that fail certification contribute nothing different — if ANY cell
fails, return the unbounded box for the whole call (a partial derivative
enclosure is unsound). This needs the leader's `enclose_der(2, ·)` — the
`C: EnclosureCurve` bound supplies it.

### 5. `tangent_cone` — the family construction, verbatim

The 2026-08-20 family amendment's ball-around-midpoint construction off the
`n = 1` enclosure's per-axis hull: axis `mid(hull).normalize()`, half-angle
`asin(rho / ‖mid‖)` when `rho < ‖mid‖` with `rho` the hull's halfwidth norm —
`None` otherwise (the derivative enclosure contains 0: a cusp, a transversal
failure, or the k-degeneracy of decision 4). Round `rho` up and `‖mid‖` down;
clamp the half-angle at `MAX_HALF_ANGLE = PI` (copy pcurve.rs's const and its
doc comment — the ulp nudge past π/2 case). A non-finite or zero axis →
`None`. `enclose_der(1, ·)` returning the unbounded box makes the hull contain
0 → `None` — correct by construction.

### 6. Module docs and house style

The module doc comment states: what the carrier is (the double-projection
system), the leader-hull under-estimation hazard, the certified-parameter-
image design, the Krawczyk operator and its point-center term, the knot-
aligned cells, the measured costs (quote the numbers above), the n ≥ 2
refusal, and that over-estimation refusals are always the unbounded box.
The H-1 deny block is NOT needed in the module file (the crate root denies
`unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented`/`indexing_slicing`
crate-wide); do not introduce any of those anyway — `search_triple` returns
`Option` and every path is a value or the unbounded box.

### 7. Witnesses (copy these exactly — they are the measured ones)

```rust
/// Two sqrt(2)-spheres at (0, 0, ±1): the intersection is the unit circle
/// in the z = 0 plane.
fn sphere_pair() -> (Sphere, Sphere) { /* Sphere::new((0,0,1), sqrt2), Sphere::new((0,0,-1), sqrt2) */ }

/// A chord-polyline leader on the unit circle, `theta in [0.3, 1.0]`,
/// `segs` equal segments, as a clamped degree-1 BSplineCurve<Point3> with
/// control points ON the circle (chord sag = the leader's coarseness).
fn chord_leader(segs: usize) -> BSplineCurve<Point3> {
    let mut knots = vec![0.0, 0.0];
    for i in 1..segs { knots.push(i as f64 / segs as f64); }
    knots.push(1.0); knots.push(1.0);
    let ctrl = (0..=segs).map(|i| {
        let th = 0.3 + 0.7 * (i as f64) / (segs as f64);
        Point3::new(th.cos(), th.sin(), 0.0)
    }).collect();
    BSplineCurve::new(KnotVec::from(knots), ctrl)
}
```

with `use truck_geometry::nurbs::{BSplineCurve, KnotVec};` and
`use truck_geometry::specifieds::{Plane, Sphere};`. The plane-sphere witness:
`Plane::new((0,0,0.3), (1,0,0.3), (0,1,0.3))`, `Sphere::new(origin, 1.0)`,
leader `chord_leader(12)` with θ ∈ [0.2, 1.2] (adapt the builder or build
that leader inline; the slice circle has radius sqrt(1 − 0.09) at z = 0.3).
The negative witness: `Sphere::new(origin, 1.0)` twice.

### 8. Tests (all in the module's `#[cfg(test)]`, opening with the standard
`#[allow(clippy::unwrap_used, clippy::expect_used)]` + H-1 justification
comment as pcurve.rs's test module does)

Use `crate::harness::assert_encloses_curve` for the sampling guards, and copy
pcurve.rs's `cone_contains` helper (with its H-3 comment) for the cone test.
`iv(a, b)` = the module's `Interval::try_from((a, b))` test helper (test-side
unwrap allowed by the allow block). By name:

- `isc_encloses_sampled_sphere_sphere` — both `chord_leader(8)` and
  `chord_leader(16)`, span `[0.15, 0.85]`: `assert_encloses_curve(&isc, iv(0.15, 0.85), 40)`;
  also an interior span `[0.3, 0.7]`; also assert the box is FINITE (each
  axis width < 1.0 — a certification collapse to the unbounded box must fail
  this test, not silently pass soundness).
- `isc_plane_sphere_slice_is_tight` — the plane-sphere witness, span `[0.1, 0.9]`:
  soundness by harness, then assert the z-axis width `<= SLICE_SLACK`
  (`const SLICE_SLACK: f64 = 1.0e-4;` with an H-3 comment naming it the
  certified z-slice width of the witness, measured 2.2e-6; the const leaves
  room for the certification's pad choice without blessing a collapse).
- `isc_identical_surfaces_refuse_whole` — the negative witness:
  `enclose(iv(0.1, 0.9))` is `Interval::ENTIRE` on every axis, and
  `tangent_cone` is `None` (assert both — a whole-box `enclose` whose cone
  still returned `Some` would mean decision 5 is not wired).
- `isc_out_of_range_span_is_unbounded` — the sphere-sphere witness with
  `iv(-1.0, 0.5)`, `iv(0.5, 2.0)`, `iv(-10.0, 10.0)`: every axis ENTIRE
  (outside the leader's [0, 1] range the leader hull is unbounded and no
  certification can hold; the sound answer propagates).
- `isc_der1_contains_finite_differences` — the plane-sphere witness, span
  `[0.2, 0.8]`: `enclose_der(1, ·)` is finite (not ENTIRE), and for 5 grid t's
  the central difference `(subs(t+h) − subs(t−h)) / (2h)` with
  `h = 1.0e-5` (named const, H-3 comment: finite-difference step) is
  contained per axis.
- `isc_der_above_one_is_unbounded` — n = 2 and n = 3 on the sphere-sphere
  witness: ENTIRE per axis.
- `isc_tangent_cone_contains_exact_circle` — the sphere-sphere witness, span
  `[0.2, 0.8]`: the cone is `Some`, and the exact unit-circle tangents
  `(−sin θ, cos θ, 0)` at 9 grid θ values interpolated along the span lie in
  the cone by `cone_contains`. (The ISC's tangent direction IS the circle
  tangent; the leader's k-factor only scales it.)

One doctest, in the module docs: build the sphere-sphere 8-segment witness,
`enclose` over `[0.15, 0.85]`, assert a sampled `subs(t)` (three t's) lies in
the box. Keep it dependency-light (truck-geometry witnesses, no harness).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal (the regex catches `1e-6`, `1.0e-6`, `1.0e-06`, ...) unless
that same line ends with an `// H-3` comment. It is a text gate on the diff:
it does not know your literal is a tolerance, and it does not care that the
line is in a test. This packet's constants are pre-named for you —
`INITIAL_PAD`, `GROWTH`, `MAX_INFLATIONS`, `CELL_FLOOR`, `NEWTON_PAD`,
`SLICE_SLACK`, the finite-difference step, `MAX_HALF_ANGLE` — define each as a
named const whose defining line carries a same-line `// H-3:` comment naming
the dimensionless quantity it is. Witness coordinates are `0.3`/`0.7`/`1.0`/
`0.5`-class decimals. Run `bash scripts/kernel-gates.sh` yourself before you
write `RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo test -p truck-evidence --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**The crate is clean at baseline** — measured at the tree this packet was
written against (HEAD b1519e0): 156 lib tests + 3 doc/test-binary results
pass, zero clippy findings (the crate denies `clippy::all` at its root and is
clean). Your bar: everything above stays green plus your seven new tests and
one doctest. Any baseline failure you did not cause is a stop condition; any
failure you did cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — the other decorators
(`pcurve.rs`, `processor.rs`, `revolved.rs`, `extruded.rs`, `offset.rs`), the
carriers (`bspline.rs`, `sphere.rs`, `plane.rs`, `circle.rs`, `cone.rs`,
`cylinder.rs`, `torus.rs`, `nurbs.rs`, `elementary.rs`, `harness.rs`,
`enclosure.rs`, `deviation.rs`, `analytic/**`), truck-geometry and every other
crate (the carrier, `search_triple`, `der_mn`, `cut`, `KnotVec` are read-only
dependencies). Overriding `exact_spline` on the ISC. Adding `#[ignore]`.
Adding `unwrap()`/`expect()`/`panic!` on fallible paths in production code
(the test module's allow block is the house exception). Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `search_triple`'s signature or return shape differs from
  `Option<(Point3, Point2, Point2)>`, or `der_mn`'s argument order differs
  from `(m, n, u, v)` (u-order first) → `SPEC_GAP`, with the exact signature
  you found
- certification never succeeds on the sphere-sphere 8-segment witness even
  with the formulas verbatim (bisection to the floor, unbounded box) →
  `SPEC_GAP` — the design was validated by running it, but exactly the kind
  of claim that can be confidently wrong; report what K's widths look like
  relative to Q's at the first inflation steps
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): certified enclosure for IntersectionCurve (BG-ENC-004-ISC)`.
