# WORK PACKET BG-CE-002 — the whole-span leader-vs-carrier deviation certificate

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-CE-002","status":"DONE","contracts":["BG-CE-002"],
 "tests_added":12,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: the claims below were derived
by command against the tree and validated by compiling and RUNNING the whole
design in a scratch crate against the real carriers, but they are exactly the
kind of claim that can be confidently wrong. **If anything below contradicts
what you find in the code, say so in `disagreements` rather than making the
code match the packet.**

```yaml
id:          BG-CE-002
contract:    [BG-CE-002]
class:       design
crates:      [truck-base, truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/deviation.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/bspline.rs
  - vendor/truck/truck-evidence/src/nurbs.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-evidence/src/decorators/pcurve.rs
  - vendor/truck/truck-base/src/evidence.rs
read_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-geometry/src/decorators/af_surface.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - knot_multiplicity_counts_exactly
  - bspline_hull_converges_into_the_terminal_strip
  - pcurve_hull_converges_into_the_terminal_strip
  - nurbs_hull_converges_into_the_terminal_strip
  - exact_spline_exposes_the_plane_composition
  - route1_exact_pair_certifies_one_shot
  - route1_offset_pcurve_fails_one_shot
  - route1_flip_correspondence_certifies
  - route1_degree_mismatch_elevates_and_certifies
  - route2_line_pair_with_rescaled_range_certifies
  - route2_budget_exhaustion_refuses
  - deviation_empty_span_refuses_empty
  - deviation_bound_dominates_sampled_deviations
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn knot_multiplicity' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn knot_multiplicity' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn knot_multiplicity' vendor/truck/truck-evidence/src/nurbs.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'knot_vec().multiplicity(idx)' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'knot_vec().multiplicity(idx)' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'knot_vec().multiplicity(idx)' vendor/truck/truck-evidence/src/nurbs.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'impl EnclosureCurve for BSplineCurve<Point3>' vendor/truck/truck-evidence/src/bspline.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'impl<S: EnclosureSurface<Vector = Vector3>> EnclosureCurve for PCurve' vendor/truck/truck-evidence/src/decorators/pcurve.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A10, expect: 1, cmd: "grep -c 'pub trait EnclosureCurve' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A11, expect: 1, cmd: "grep -c 'pub trait EnclosureSurface' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A12, expect: 1, cmd: "grep -c 'pub enum UnresolvedWitness' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A13, expect: 0, cmd: "grep -c 'DeviationUncertified' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A14, expect: 0, cmd: "grep -r 'certify_deviation' vendor/truck/truck-evidence/src | wc -l"}
  - {id: A15, expect: 1, cmd: "grep -c 'pub fn spend_subdiv' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A16, expect: 1, cmd: "grep -c 'pub fn elevate_degree' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A17, expect: 1, cmd: "grep -c 'pub fn entity_tau' vendor/truck/truck-base/src/tolerance.rs"}
```

(A `grep -c` on a directory needs `-r`; A14's `grep -r ... | wc -l` form
counts matching LINES — zero when the name does not exist yet. `grep -c` and
`grep -r` exit 1 on zero matches — that IS the expected count, not a command
failure.)

## Problem

BG-CE-001 landed the per-use pcurve payload on the edge handle. The certificate
that payload exists for — **BG-CE-002**: for an edge use with parametric trace
`pc_u` on face `f` and leader curve `c_e` with parameter correspondence `phi_u`,

```
|| Γ_f(pc_u(t)) − c_e(phi_u(t)) || ≤ τ_e   for ALL t in the span
```

— does not exist yet. It must be certified by **interval evaluation over the
whole span** (BG-ENC-001), not by sampling: sampling is the classic false pass
here. This packet lands the certificate as a reusable function in
truck-evidence, and it fixes one real defect in the landed carriers that this
certificate is the first consumer to walk into (decision 0).

Two facts about the cost, both measured against the real carriers in a scratch
crate before this packet was written:

1. **Box-minus-box bisection does not scale to small τ.** The residual box of
   two independently-enclosed curves over-estimates by ~`(‖c'‖+‖l'‖)·width`
   per cell (the interval dependency problem), so certifying τ = 1e-6 on a
   unit span needs millions of cells — measured ~130 µs per cell for
   spline-backed carriers, i.e. minutes per edge. That route is kept as the
   generic fallback (decision 4) and is budgeted and honest, but it cannot be
   the main path.
2. **When both sides are exactly B-splines, the certificate is one-shot.**
   Subtracting the two curves *as splines* (coefficientwise, after knot merge)
   and hulling the difference kills the dependency problem: an exact-agreement
   pair has a difference spline whose control points are all ~0, so the
   whole-span hull certifies at any τ with zero subdivisions. Measured: bound
   2.5e-14 at τ = 1e-6, one cell, for the exact pair; one-shot decisive
   violation (bound 2.0e-6 > τ = 1e-6) for a pair offset by 2τ. This is route
   1 (decision 3).

A `PCurve<BSplineCurve<Point2>, Plane>` composes exactly into a
`BSplineCurve<Point3>` (decision 2), which is precisely the pcurve-on-planar-
face case, so route 1 covers the commonest real consumer.

**The defect (decision 0).** `knot_multiplicity` in `bspline.rs`,
`decorators/pcurve.rs` and `nurbs.rs` (three copies, byte-identical logic)
delegates to `KnotVec::multiplicity(idx)`, which counts knots by TOLERANCE
(`is_small_ratio`, i.e. within 1e-6 at the legacy scale) — even though the
helper's own doc comment claims the comparison is tolerance-free. When the
sub-curve extraction cuts at a parameter `x` within 1e-6 of a *different* knot
value (e.g. any cell in the last 1e-6 of the knot range, next to the terminal
knot), the neighbor's copies inflate the count, `raise_to_full_multiplicity`
under-inserts, and the extracted "sub-curve" spans a much larger piece — its
hull stays sound (over-estimation) but STOPS CONVERGING: measured, the
enclosure of `[1−w, 1]` plateaus at the whole-tail hull for every `w < 1e-6`.
That violates BG-ENC-002 (convergence) in the terminal strip of every knot,
and it starves both routes of their subdivision fallback there. No landed test
catches it because none bisects into the strip. The fix is one function per
file: count EXACT matches only. Verified in the scratch crate: with the exact
count, the sub-curve over `[1−w, 1]` has knot range exactly `[1−w, 1]` and its
hull converges to the endpoint for w down to 1e-12.

## Decisions already made for you

### 0. The `knot_multiplicity` fix, in all three files, verbatim

In `bspline.rs`, `decorators/pcurve.rs` and `nurbs.rs`, replace the body of
`knot_multiplicity` (keep each file's doc comment, correcting its last
sentence as below) with an exact count:

```rust
fn knot_multiplicity<P: ControlPoint<f64>>(bsp: &BSplineCurve<P>, x: f64) -> usize {
    bsp.knot_vec().iter().filter(|&&k| k == x).count()
}
```

Adjust each doc comment to say: the count is over **exact** knot equality —
`KnotVec::multiplicity` matches by tolerance and would count a *different*
knot value within 1e-6 of `x`, which under-inserts in the raising loop and
extracts an over-wide sub-curve whenever `x` sits within tolerance of another
knot (the terminal strip of every knot range). Do not change
`raise_to_full_multiplicity`, `sub_curve`, or anything else in those files'
extraction chains — with the exact count, the existing chain is correct (this
was traced through `cut`'s snap branch: pre-raised exact knots make the snap
a no-op and the tolerance-inflated `s` in `cut` only skips insertion that is
already done).

Each fixed file gains ONE regression test in its own `#[cfg(test)]` module
(see "Tests required"): bisect a witness curve's enclosure into the terminal
strip and assert the box keeps shrinking there.

### 1. `ParamMap` — the parameter correspondence, in `deviation.rs`

```rust
/// The parameter correspondence phi between the carrier trace's parameter t
/// and the leader curve's parameter: phi(t) = scale * t + offset, evaluated
/// in outward-rounded interval arithmetic on every cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamMap {
    /// The scale factor.
    pub scale: f64,
    /// The offset.
    pub offset: f64,
}

impl ParamMap {
    /// phi(t) = t.
    pub const IDENTITY: Self = Self { scale: 1.0, offset: 0.0 };

    /// phi(t) = t0 + t1 - t, the orientation flip over [t0, t1].
    pub const fn flip(t0: f64, t1: f64) -> Self {
        Self { scale: -1.0, offset: t0 + t1 }
    }

    /// The affine map sending [a0, a1] onto [b0, b1]; `None` when a0 == a1.
    pub fn from_ranges(a0: f64, a1: f64, b0: f64, b1: f64) -> Option<Self> {
        if a0 == a1 {
            None
        } else {
            let scale = (b1 - b0) / (a1 - a0);
            Some(Self { scale, offset: b0 - a0 * scale })
        }
    }

    /// phi(t) in f64 (for sampling guards and tests, never for certification).
    pub fn apply_f64(&self, t: f64) -> f64 {
        self.scale * t + self.offset
    }

    /// phi(tt), outward-rounded.
    pub fn apply(&self, tt: Interval) -> Interval {
        interval_at(self.scale) * tt + interval_at(self.offset)
    }
}
```

`interval_at` is this crate's standard duplicated helper (`Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)`); define
it in `deviation.rs` the same way `pcurve.rs` and `line.rs` carry their own
copies. Negation (flip's scale) is exact in interval arithmetic; the general
scale rounds outward. Nothing here needs a guard.

### 2. The trait plumbing: `exact_spline` and `as_plane`

In `enclosure.rs`, add to `EnclosureCurve`:

```rust
/// This curve exactly represented as a `BSplineCurve<Point3>`, when it is one
/// — including by exact affine composition of a planar pcurve. `None` for any
/// curve whose exact representation is not a plain B-spline (circles, NURBS,
/// lines, general pcurves). Route 1 of BG-CE-002's deviation certificate
/// builds on this; the default keeps every other carrier on the generic
/// bisection route.
fn exact_spline(&self) -> Option<BSplineCurve<Point3>> {
    None
}
```

and to `EnclosureSurface`:

```rust
/// This surface exactly, when it is a `Plane` (the exact affine carrier).
/// `None` otherwise. Used by `PCurve`'s `exact_spline` to compose a planar
/// pcurve into a spline exactly.
fn as_plane(&self) -> Option<&Plane> {
    None
}
```

with `use truck_geometry::specifieds::Plane;` and
`use truck_geometry::nurbs::BSplineCurve;` added to `enclosure.rs`'s imports
(truck-evidence already depends on truck-geometry). Override them in exactly
three places:

- `bspline.rs`, inside `impl EnclosureCurve for BSplineCurve<Point3>`:
  `{ Some(self.clone()) }`.
- `plane.rs`, inside `impl EnclosureSurface for Plane`:
  `as_plane` → `{ Some(self) }` (Plane is Copy).
- `decorators/pcurve.rs`, inside the existing
  `impl<S: EnclosureSurface<Vector = Vector3>> EnclosureCurve for
  PCurve<BSplineCurve<Point2>, S>` — add `exact_spline`:

```rust
fn exact_spline(&self) -> Option<BSplineCurve<Point3>> {
    let plane = self.surface().as_plane()?;
    // S(p) = o + p.x * a + p.y * b is affine, and B-spline evaluation is
    // linear in the control points, so the composed curve is the B-spline
    // with the same knots and control points o + cx_i * a + cy_i * b —
    // exact, not an approximation.
    let o = plane.origin();
    let a = plane.u_axis();
    let b = plane.v_axis();
    let cps: Vec<Point3> = self
        .curve()
        .control_points()
        .iter()
        .map(|p| o + a * p.x + b * p.y)
        .collect();
    Some(BSplineCurve::new(self.curve().knot_vec().clone(), cps))
}
```

  (`Point3 + Vector3 -> Point3` is cgmath's; `o + a * p.x + b * p.y`
  type-checks as written.) Do NOT add `exact_spline` overrides anywhere else —
  every other carrier stays on the default `None`.

### 3. Route 1 — the difference spline (the main path)

All in `deviation.rs`. The entry point first tries route 1 and falls back to
route 2 (decision 4) when any precondition fails. Route 1's steps, each one
validated by compiling and running it against the real carriers:

**(a) Obtain both splines.** `leader.exact_spline()` and
`carrier.exact_spline()`; either `None` → route 2.

**(b) Apply phi to the leader — identity or flip only.** phi ==
`ParamMap::IDENTITY` → use the leader spline as-is. phi with `scale == -1.0` →
construct the reversed spline:

```rust
/// The leader under the flip correspondence: knots k -> offset - k (the
/// mapped list reversed back to ascending), control points reversed. Valid
/// only when both endpoint knots are at full multiplicity `degree + 1`;
/// `None` otherwise (the caller falls back to route 2).
fn flipped_spline(leader: &BSplineCurve<Point3>, offset: f64) -> Option<BSplineCurve<Point3>>
```

Any other phi (including `from_ranges` maps) → route 2. The reversal's knot
arithmetic (`offset - k`) has one rounding per knot; that ulp-class
perturbation is covered by the hull pad (see (e)) — the pad exists for exactly
this class of f64 recomputation, with ten orders of margin at tau >= 1e-6
scales.

**(c) Equalize degrees, then merge knot vectors.**

```rust
/// Raises each curve at the other's distinct knot values until both share
/// one knot vector. Returns false when the degrees differ before elevation.
fn merge_knots(a: &mut BSplineCurve<Point3>, b: &mut BSplineCurve<Point3>) -> bool
```

If `a.degree() != b.degree()`, call `elevate_degree()` (a public, general
method of `BSplineCurve`, anchor A16) on the lower-degree curve until the
degrees match — elevation recomputes control points in f64, again ulp-class,
again covered by the pad. Merge by raising each curve at every distinct knot
value of the other up to that value's multiplicity in the other (count with
the exact `iter().filter(|&&k| k == x).count()` form, NOT
`KnotVec::multiplicity` — decision 0's defect class). Finish only if the two
knot vectors end up equal element-by-element; otherwise return the fallback.

**(d) The difference spline.** Same knots, same degree, same control-point
count: subtract coefficientwise. NOTE: cgmath `Point3 - Point3` yields a
`Vector3` — build the difference points explicitly:

```rust
.map(|(a, b)| Point3::new(a.x - b.x, a.y - b.y, a.z - b.z))
```

Build with `BSplineCurve::new_unchecked` (the merged vectors are valid).

**(e) Cut to the certification span, hull, subdivide.** The span
`tt = [lo, hi]` must lie inside the difference spline's knot range
`[first, last]` (else fallback to route 2). Raise `lo` and `hi` to exact full
multiplicity with decision 0's fixed counting, then extract the sub-piece the
same way `sub_curve` does — **`cut` mutates self to the FRONT piece and
returns the TAIL**: `let _tail = d.cut(hi); let mut piece = d.cut(lo);` —
`piece` is `[lo, hi]`; getting this backwards produces the complement piece
and the certificate reads garbage.

The whole-span hull bound off a piece's control points:

```rust
/// The per-axis control-point hull box, padded HULL_PAD (1 + |.|) outward
/// per endpoint. The pad covers the ulp-class recomputations along the way
/// (merge insertion, degree elevation, reversal, extraction).
fn control_point_box(bsp: &BSplineCurve<Point3>) -> Box3
```

with `const HULL_PAD: f64 = 64.0 * f64::EPSILON;` (the landed carriers'
constant and rationale, copied as usual). Norm bounds off a `Box3` — reuse one
helper for both routes:

```rust
/// (sup, inf) of ||v|| for v in the box, by interval arithmetic.
fn norm_bounds(b: &Box3) -> (f64, f64) {
    let norm = (b.x * b.x + b.y * b.y + b.z * b.z).sqrt();
    (norm.sup(), norm.inf())
}
```

Worklist loop over pieces (a `Vec` stack): per piece, `(upper, lower) =
norm_bounds(&control_point_box(&piece))`; `upper <= tau` → accumulate
`sup_bound = sup_bound.max(upper)`; `lower > tau` (finite) → **decisive
violation**, `Err(Refusal::ForwardToleranceExceeded { bound: lower, allowed:
tau })`; else bisect the piece at its parameter midpoint (after the pre-raised
cuts the piece's knot range IS its span — read the endpoints off
`knot_vec().first()/last()`), spending one subdivision per bisection, with the
same midpoint-representability floor as route 2. Because the difference
spline's control points are tight around the true residual, this loop
terminates in a handful of levels for near-agreement pairs and proves
violations almost immediately for bad ones; the budget still bounds the
pathological middle.

**(f) Certificate.** On success return `Ok(Certified::new(sup_bound,
Certificate { props, method: Method::Interval, budget_left: *budget, margin:
Margin::UNBOUNDED, modulus: Modulus::Unbounded }))` with
`props.set(Prop::SoundEnclosure, Truth::True)` — the certificate's whole
claim is the sound enclosure of the deviation. `margin`/`modulus` follow the
house pattern every current `Outcome` producer uses (af_surface.rs:531-534,
the analytic modules); the module docs note that no stability claim is made.

### 4. Route 2 — the generic bisection fallback

The loop the cost analysis measured; kept exactly as designed:

```rust
pub fn certify_deviation<L, C>(
    leader: &L,
    carrier: &C,
    phi: ParamMap,
    tt: Interval,
    tau: f64,
    budget: &mut Budget,
) -> Outcome<f64>
where
    L: EnclosureCurve,
    C: EnclosureCurve,
```

Semantics, in order:

1. `tt` empty or non-finite → `Err(Refusal::Empty)` — nothing to certify.
2. Try route 1 (both `exact_spline`s, phi identity/flip, span in range,
   degree/knot merge successful). Route 1's result is returned as-is when it
   applies.
3. Route 2 worklist: per cell `tt_i`, the residual box is
   `carrier.enclose(tt_i)` minus `leader.enclose(phi.apply(tt_i))` per axis
   (interval subtraction), norm by `norm_bounds`. `upper <= tau` (finite) →
   accumulate. `lower > tau` (finite) → decisive violation as in route 1.
   Otherwise bisect at the midpoint: if `!(lo < mid && mid < hi)` the cell
   cannot be bisected at representable width — unresolved; if
   `budget.spend_subdiv(1)` fails — unresolved. Either way:
   `Err(Refusal::NumericallyUnresolved { spent: budget_spent(initial,
   *budget), witness: UnresolvedWitness::DeviationUncertified })`.
4. All cells certified → the certificate of (f), value = the max per-cell
   upper bound.

`budget_spent` is af_surface.rs:380's helper, duplicated here with its doc
comment (`initial` snapshotted at entry; `Budget` fields are remaining counts,
so spent = entry − remaining). `interval_at` likewise. `tau` is taken as a
bare `f64` with NO validity guard: a nonpositive or NaN tau is handled by the
loop's own logic (upper <= tau is false for every cell, no lower bound can
prove violation, the budget or the width floor eventually refuses) — honest
refusals, no panics, no special case; document this in the function's doc
comment. Callers derive tau from `ToleranceCtx::entity_tau` (anchor A17) in
real use.

### 5. The truck-base amendment: one enum variant

In `vendor/truck/truck-base/src/evidence.rs`, add to `UnresolvedWitness` (a
plain additive arm with a doc comment; no exhaustive `match` on this enum
exists anywhere in the tree — verified by grep before this packet was
written):

```rust
/// A whole-span deviation bound (BG-CE-002) could not be certified within
/// the subdivision budget: interval evaluation left at least one cell whose
/// upper bound exceeds the tolerance and whose lower bound does not prove
/// violation.
DeviationUncertified,
```

### 6. Module wiring

`deviation.rs` opens with the H-1 deny block (GATE-1 gates new kernel files
on it — copy the five-attribute block from `lib.rs:27-34`). `lib.rs` gains
`pub mod deviation;` in the module list (alphabetical position, beside
`decorators`) and the re-export
`pub use deviation::{certify_deviation, ParamMap};` beside the existing
`pub use enclosure::{...}` line. The module's doc comment states the cost
model plainly: route 2's per-cell over-estimation is
~`(‖c'‖+‖l'‖)·width`, so certifying tau on a span costs
`O((‖c'‖+‖l'‖)·span/tau)` subdivisions — callers budget accordingly; route 1
is one-shot for exact-spline pairs.

### 7. Tests

All in `deviation.rs`'s `#[cfg(test)]` module (opening
`#[allow(clippy::unwrap_used, clippy::expect_used)]` with the standard H-1
justification comment, as `pcurve.rs:470-473` does), except the three
convergence regressions which live in their own files' test modules. Reuse
`pcurve.rs`'s witnesses — copy the builders:

- `plane()`: `Plane::new(origin, (1,0,1), (0,1,1))` — S(u,v) = (u, v, u+v).
- `parabola2()`: the degree-2 Bezier (t, t²) with control points
  (0,0), (1/2,0), (1,1).
- `carrier_witness()`: `PCurve::new(parabola2(), plane())` — composes to
  (t, t², t+t²).
- `leader_witness()`: the SAME curve as a `BSplineCurve<Point3>` with control
  points (0,0,0), (1/2,0,1/2), (1,1,2) on `KnotVec::bezier_knot(2)` —
  Bernstein: x = t, y = t², z = t + t². Bit-exact agreement with the
  flattened carrier.

The required tests, by name:

- `knot_multiplicity_counts_exactly` — unit test (place it in `bspline.rs`'s
  test module): a curve with knots `[0,0,0,1,1,1]`; the exact count of 1.0 is
  3 and of `1.0 - 1.0e-6` is 0; then `raise_to_full_multiplicity` at
  `1.0 - 1.0e-6` reaches multiplicity 3 (three insertions), which the old
  tolerance-based count refused to do.
- `bspline_hull_converges_into_the_terminal_strip` — for w in a descending
  list ending at 1e-12-ish: `enclose(iv(1.0 - w, 1.0))` has x-width `<= 4.0 *
  w + hull-slack` (the true x-span is w; allow the pad and two hull
  endpoints) and strictly smaller than at the previous, larger w. Use named
  consts; no bare `1e-N` literals (H-3).
- `pcurve_hull_converges_into_the_terminal_strip` — same assertion on
  `carrier_witness()`'s enclosure (the composed z-coordinate is the widest).
- `nurbs_hull_converges_into_the_terminal_strip` — same on a
  `NurbsCurve<Vector4>` with unit weights over the same parabola (build with
  `NurbsCurve::new` from truck-geometry; homogeneous control points
  (x, y, z, 1)). If NurbsCurve's constructor needs a different shape than
  this suggests, adapt the witness, not the assertion.
- `exact_spline_exposes_the_plane_composition` —
  `carrier_witness().exact_spline()` is `Some` and equals `leader_witness()`
  (compare control points and knot vectors); `Plane`'s `as_plane()` is
  `Some`; a `Line<Point3>`'s `exact_spline()` is `None` (the default).
- `route1_exact_pair_certifies_one_shot` — certify
  (`leader_witness`, `carrier_witness`, `IDENTITY`, full span, tau =
  `ToleranceCtx::unscaled_legacy().entity_tau(TOLERANCE)`): `Ok` with value
  `<= tau` (measured 2.5e-14) AND the budget's subdiv counter untouched
  (zero subdivisions — this is the route-1 claim).
- `route1_offset_pcurve_fails_one_shot` — **the spec's named negative
  test**: the leader translated by `2.0 * tau` in z (add it to every control
  point). Must return `Err(Refusal::ForwardToleranceExceeded { bound, allowed
  })` with `bound > tau` (measured ~2·tau) and `allowed == tau`, again with
  zero subdivisions. A checker that passes everything is the failure mode
  here; this test is why the lower-bound branch exists.
- `route1_flip_correspondence_certifies` — the reversed leader (control
  points in reverse order) with `ParamMap::flip(0.0, 1.0)`: one-shot `Ok`,
  value `<= tau`.
- `route1_degree_mismatch_elevates_and_certifies` — elevate the carrier
  witness' flattened spline once with `elevate_degree()` (degree 2 → 3, same
  curve), certify against the degree-2 leader: still one-shot `Ok`.
- `route2_line_pair_with_rescaled_range_certifies` — the fallback:
  `carrier = Line((0,0,0), (2,0,0))` on [0,1] (subs(t) = (2t,0,0)),
  `leader = Line((0,0,0), (2,0,0))` interpreted on [0,2], phi =
  `from_ranges(0.0, 1.0, 0.0, 2.0)`. Lines never expose `exact_spline`, so
  this exercises route 2 end to end with an exact-agreement pair. Use tau =
  `1.0e-4` and budget 1<<16 (measured: ~30k subdivisions, well under a
  second — lines enclose in closed form). Assert `Ok`, value `<= tau`, and
  that the budget actually spent subdivisions (`budget_left.subdiv <
  initial.subdiv`) — route 2's loop is load-bearing here.
- `route2_budget_exhaustion_refuses` — the same line pair with
  `Budget::new(0, 0, 0)`: `Err(Refusal::NumericallyUnresolved { spent,
  witness: UnresolvedWitness::DeviationUncertified })` with
  `spent.subdiv == 0` (nothing was spendable). This is also the
  DeviationUncertified construction test.
- `deviation_empty_span_refuses_empty` — `Interval::EMPTY` (and a NaN-bound
  box) → `Err(Refusal::Empty)` on both routes' shared preface.
- `deviation_bound_dominates_sampled_deviations` — the falsification guard:
  certify the exact pair on an interior span `[0.2, 0.8]` (route 1), then
  sample ~200 t's, computing `carrier.subs(t) - leader.subs(phi.apply_f64(t))`
  in f64 (needs `use truck_geotrait::ParametricCurve;` and
  `truck_base::cgmath64::InnerSpace` for `.magnitude()`); every sampled
  deviation must be `<= certified bound + 1.0e-12` (H-3 comment on that
  line: sampling slack, not a length tolerance). Sampling may only falsify,
  never establish.

Doctests: give `ParamMap` and `certify_deviation` doc examples in the house
style (two total is enough; keep them dependency-light — `ParamMap`'s can be
pure arithmetic, `certify_deviation`'s can build the plane witnesses).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal (the regex catches `1e-6`, `1.0e-6`, `1.0e-06`, ...) unless
that same line ends with an `// H-3` comment. It is a text gate on the diff:
it does not know your literal is a tolerance, and it does not care that the
line is in a test. This packet's tests avoid it structurally — tolerances
come from `ToleranceCtx::unscaled_legacy().entity_tau(TOLERANCE)` (named
imports, no literals), witness coordinates are `0.5`/`1.0`/`2.0`-class
decimals, and the one deliberate `1.0e-4` (route-2 tau) and `1.0e-12`
(sampling slack) must be written as named consts whose defining line carries
a same-line `// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh` yourself before you write `RESULT.json`; it is
the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-base
cargo clippy -p truck-evidence -p truck-base --all-targets --no-deps
cargo test -p truck-evidence -p truck-base --lib --tests --no-fail-fast
cargo test -p truck-evidence --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail.

**Both crates are clean at baseline** — measured at the tree this packet was
written against (HEAD 2cf9094): 201 lib/integration tests pass across the two
crates, zero clippy findings on truck-evidence (the crate denies
`clippy::all` at its root and is clean), and the `--doc` suite passes. Your
bar: everything above stays green plus your thirteen new tests and two
doctests. Any baseline failure you did not cause is a stop condition; any
failure you did cause is yours to fix.

## Forbidden

Editing any file outside `write_allow` — the other carriers (`circle.rs`,
`cone.rs`, `cylinder.rs`, `sphere.rs`, `torus.rs`, `elementary.rs`,
`harness.rs`, `analytic/**`, the other decorators), `truck-geometry`
(`elevate_degree`, `cut`, `add_knot` are read-only dependencies), and every
other crate. Changing the semantics of any existing enclosure (the fix in
decision 0 tightens extraction; soundness and every existing test must hold
unchanged). Adding `#[ignore]`. Adding `unwrap()`/`expect()` on fallible
paths in production code (the test module's allow block is the house
exception). Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `elevate_degree` or `cut` does not behave as this packet describes (mutates
  self to the front, returns the tail; general elevation via bezier
  decomposition) → `SPEC_GAP`, with the exact behavior you observed
- the plane-pcurve composition in decision 2 does not reproduce the carrier
  bit-exactly on the witnesses → `SPEC_GAP` — that claim was validated by
  running it, but bit-exactness across all inputs is exactly the kind of
  claim that can be confidently wrong
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): whole-span leader-vs-carrier deviation certificate (BG-CE-002)`.
