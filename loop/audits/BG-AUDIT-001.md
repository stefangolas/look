# BG-AUDIT-001 — whole generation-kernel correctness review

Independent correctness audit of the landed BREP generation formal-system
build on `integration/kernel-bg`. This is a **find-defects** packet, not an
implementation packet.

## 1. Executive result

```
Audited HEAD:   f919228aff9623b03f0602c3f260c66798829567  (integration/kernel-bg)
AUDIT_BASE:     d1f9c5bd7dccaeca13400c15fbf8e38fc7fb006c  (first-parent parent of the first BG merge, c8acab6)
Changed kernel files:  131 under vendor/truck (110 Rust sources + Cargo.toml + tests/regressions)
Changed kernel LOC:    34,146 added, 1,490 deleted

P0: 0
P1: 6
P2: 7
P3: 4
P4: 0
```

Highest-risk subsystem: **`truck-evidence` — the enclosure/evidence kernel**
(sphere normal-cone under-enclosure, degree-0 deviation false certificate,
parametric-Krawczyk center-term unsoundness, wedge NaN silent-pass), followed
by **`truck-geometry/src/canonical.rs`** (non-conformal transforms of revolved
surfaces silently return the wrong surface) and the **topology invariant
checkers** (degenerate closed-shell certification).

The audited HEAD is the integration tip of `integration/kernel-bg`; the
`loop/` bookkeeping (STATE, PACKETS, results) claims 74/76 packets and a green
verification loop. The verification loop's own tests do pass on this tree
(confirmed by running `cargo test -p truck-evidence --lib` = 238 passed,
`cargo test -p truck-topology --lib` = 59 passed). **This report's findings
are not contradicted by those green tests**: for every finding below, the
existing tests do not exercise the failing input class.

"No defect found in reviewed scope" is NOT claimed: the reviewed scope is a
large fraction of the new evidence kernel and all of the new topology/identity
layer, but not every changed file was audited to the same depth (see §8).

## 2. Top findings

All six P1 findings, then the P2 findings.

| ID | Priority | Confidence | Subsystem | One-line summary |
|----|----------|------------|-----------|------------------|
| AUD-001 | P1 | C3 | truck-evidence sphere | `normal_cone` under-encloses normals on azimuth-wide patches (probe: 31,702/40,000 sampled normals escape) |
| AUD-002 | P1 | C3 | truck-evidence deviation | route-1 degree-0 spline certifies a false whole-span deviation (probe: bound 2.4e-14, true 1.0) |
| AUD-003 | P1 | C3 | truck-topology wedge | wedge checker certifies non-degeneracy on a NaN normal (apex sample) |
| AUD-004 | P1 | C2 | truck-evidence ISC | parametric Krawczyk center term at `t_mid` does not prove "unique in Q for every t in the cell" |
| AUD-005 | P1 | C2 | truck-geometry canonical | `RevolutedCurve::transformed` silently returns the wrong surface under non-conformal maps |
| AUD-006 | P1 | C2 | truck-geometry bspsurface | `sectional_curve` v-cut misindexing: wrong section geometry + reachable panic (pre-existing) |
| AUD-007 | P2 | C3 | truck-evidence lfs | `wedge_slope_lower_from_sin_margin` over-reports its lower bound at small margins (cancellation) |
| AUD-008 | P2 | C3 | truck-topology invariants | single-face `[e, e.inverse()]` shell certified `Closed` + CoedgePairing + VertexLink |
| AUD-009 | P2 | C2 | truck-geometry canonical | full-circle → NURBS with negative/zero weights: `include` false negatives + NaN `subs` |
| AUD-010 | P2 | C2 | truck-geometry cone | lower nappe of the declared double-cone domain unrecognized by include/search/nearest |
| AUD-011 | P2 | C3 | truck-geometry torus | `normal_uder` wrong z-component (pre-existing; feeds fillet Newton) |
| AUD-012 | P2 | C3 | truck-geometry af_surface | reachable panics in the fillet path (H-1 violations) |
| AUD-013 | P2 | C2 | truck-geometry sphere | `search_nearest_parameter(center)` returns `Some((NaN, NaN))` |

## 3. Complete findings

Every finding carries the required shape. Commit provenance is given where it
could be identified from `git log d1f9c5bd..integration/kernel-bg`.

---

### AUD-001 — sphere `normal_cone` under-encloses normals

```
Priority:   P1
Confidence: C3
Subsystem:  truck-evidence — EnclosureSurface for Sphere
File(s):    vendor/truck/truck-evidence/src/sphere.rs
Symbol(s):  EnclosureSurface::normal_cone (impl for Sphere), lines 116-152
Introduced: BG-ENC-002-SPHERE (feat(evidence): EnclosureSurface for Sphere, f8fe35a)
```

**Formal obligation:** `enclosure.rs:184` — "A cone of normal directions over
the box, `None` when the immersion is singular somewhere inside it. Drives
§9.1's transversality predicate." BG-ENC-001 (enclosure.rs:7-10): over- or
under-estimation rules; under-estimation is a silent-wrong-answer bug.

**Current code claims:** for a parameter box `(uu, vv)`, the returned
`DirCone { axis, half_angle }` contains every unit normal `n(u,v)`,
`(u,v) ∈ uu×vv`. The half-angle is the max corner deviation from the
corner-average axis, and the cone is returned whenever that corner half-angle
is `< π/2` (sphere.rs:144-146).

**Why that claim is wrong:** the "patch is the geodesic hull of its corners"
justification (sphere.rs:120-124) is false for this parameterization: the
u-edges map to parallels (small circles), not geodesics. When the azimuth
span `wv = v1 − v0` exceeds ≈ π, the interior of a parallel bulges
arbitrarily far from the corner-average axis, and the corner-only half-angle
misses it.

**Minimal witness:** unit sphere at origin, `uu = [0.5, 0.6]`, `vv = [0, 3.6]`.
Corner-average axis = normalize(n(0.5,0)+n(0.5,3.6)+n(0.6,0)+n(0.6,3.6)),
corner half-angle = 33.37°. The interior normal `n(0.6, 1.80)` deviates
42.31° from the axis — outside the returned cone.

**Reproduction:** temporary probe against the crate (removed after the audit):

```
PROBE2 sphere normal_cone uu=[0.5,0.6] vv=[0,3.6]:
  half_angle=33.369... deg, escaped=31702/40000, max_deviation=42.307... deg
```

**Observed result:** the cone does not contain 79% of the normals on the box.

**Correct behavior:** the cone must contain all normals. Either bound the
half-angle by the true geodesic diameter of the patch, or emit the
everything-cone whenever the azimuth span exceeds π.

**Consequence:** any downstream §9.1-style transversality predicate built on
`normal_cone` certifies a minimal inter-surface angle that is too large — a
false-positive regularity certificate. No landed consumer calls this yet, but
the contract is public and the failure is in the forbidden direction.

**Existing tests:** `sphere_normal_cone_over_patch` tests only small patches
(corner-hull ≈ patch) and wide patches that already trip the everything-cone;
no test covers a wide-azimuth patch whose corner half-angle is < π/2 while the
interior bulges beyond it. The test suite passing is consistent with the
defect.

---

### AUD-002 — route-1 deviation certificate silently under-reports for degree-0 splines

```
Priority:   P1
Confidence: C3
Subsystem:  truck-evidence — BG-CE-002 whole-span deviation
File(s):    vendor/truck/truck-evidence/src/deviation.rs
Symbol(s):  route1 (368-463), control_point_box (223-244), certify_deviation
Introduced: BG-CE-002 (feat(evidence): whole-span leader-vs-carrier deviation certificate, ae7f3a9)
```

**Formal obligation:** deviation.rs:6-11 / 252-255 — certifies
`|| carrier(t) − leader(phi(t)) || ≤ tau` for **ALL** t in the span, "by
interval evaluation over the whole span … never by sampling". This feeds
§1.1 invariant 4 (same-parameter) via `same_parameter.rs:173`.

**Current code claims:** route 1 subtracts the two splines coefficientwise
after knot merge, extracts the sub-piece `[lo, hi]` and hulls the piece's
control points. A hull whose `upper ≤ tau` certifies the deviation bound over
`[lo, hi]`.

**Why that claim is wrong:** truck's B-spline convention is right-open
(knot_vec.rs:184-185: "the B-spline basis function is based on the
characteristic function of the right-open intervals [s, t)"). For a
**degree-0** spline, `subs(t)` at an interior knot returns the *next* span's
value. The sub-piece `[lo, hi]` extracted by `cut` carries only the value on
`[lo, hi)`, so `control_point_box` hulls the value on `[lo, hi)` but **omits
the value the source curve attains at `t = hi`**. The convex-hull argument
fails at exactly the span's right endpoint.

**Minimal witness:** carrier = degree-0 spline, knots `[0, 0.5, 1]`, control
points `[(0,0,0), (0,0,1)]`; leader = degree-0, same knots, control points
`[(0,0,0), (0,0,0)]`; `phi = IDENTITY`; span `[0, 0.5]`; `tau = 0.5`.

**Reproduction:** temporary probe against the crate (removed after the audit):

```
PROBE1 half-span [0,0.5]: CERTIFIED bound=2.46e-14 <= tau=0.5; true deviation at t=0.5 = 1
PROBE1 carrier.subs(0.5)=(0,0,1) leader.subs(0.5)=(0,0,0)
PROBE1 full span [0,1]: REFUSED ForwardToleranceExceeded { bound: 0.99999..., allowed: 0.5 }
```

**Observed result:** the half-span certifies a bound of 2.46e-14 while the
true deviation at `t = 0.5` is 1.0. The full-span control correctly refuses,
proving the mechanism is the cut-away hull.

**Correct behavior:** union the endpoint values `subs(lo)`, `subs(hi)` of the
*original* curve into the piece's hull (as `bspline.rs`'s `hull_sub_curve`
does), or refuse degree-0 difference splines.

**Consequence:** `certify_deviation` — and therefore the same-parameter /
pcurve-trace invariants that consume it — can emit a certified "within tau"
claim that is false by an arbitrary amount, silently.

**Existing tests:** deviation tests use degree ≥ 1 splines (parabola,
circle/half-circle, offset pairs); no degree-0 case. The proptest regressions
do not cover degree 0.

---

### AUD-003 — wedge checker certifies non-degeneracy on a NaN normal

```
Priority:   P1
Confidence: C3
Subsystem:  truck-topology — BG-INV-109 wedge non-degeneracy
File(s):    vendor/truck/truck-topology/src/invariants/wedge.rs
Symbol(s):  check (251-302), test_edge (307-340), surface_normal (345-357)
Introduced: BG-INV-109 (packet/BG-INV-109 branch, merged at c995935)
```

**Formal obligation:** `Prop::WedgeNonDegeneracy` (§1.1 invariant 9) — the
dihedral angle is certified bounded away from 0 and 2π at every interior edge;
a wedge whose lfs collapses to zero must be declared singular, never silently
certified.

**Current code claims:** `test_edge` samples the edge at `t_mid`, projects onto
both faces, and refuses only when `|n_A × n_B| < sin_margin` (wedge.rs:332).
The certificate sets `Prop::WedgeNonDegeneracy = True` over the **whole edge**.

**Why that claim is wrong:** (1) `surface_normal` returns
`surface.normal(u, v).normalize()` (wedge.rs:356). At a singular surface point
(cone apex `v == 0` → `Cone::normal = Vector3::zero()`, cone.rs:133-136;
sphere pole; any vanishing-partial point), `normalize()` of the zero vector is
`NaN`. `NaN < sin_margin` is **false** in IEEE, so the refusal arm never
fires and the edge is certified non-degenerate. (2) Even with a finite normal,
the single midpoint sample cannot see a fold/crack elsewhere on the edge.

**Minimal witness:** an edge whose midpoint parameter maps to the apex of a
cone used as a face carrier. `search_parameter(apex)` returns `(u, 0)`
(cone.rs:265); `normal(u, 0) = 0` → `normalize()` → NaN → `sin_angle = NaN` →
no refusal → certificate.

**Reproduction:** temporary probe against the crate (removed after the audit):

```
PROBE3 wedge::check on cone-apex-midpoint edge => OK (certified non-degenerate)
```

**Observed result:** the NaN normal is silently converted into a positive
WedgeNonDegeneracy certificate.

**Correct behavior:** treat a non-finite normal as `NumericallyUnresolved`
(`!sin_angle.is_finite() → refuse`), and certify the invariant over the whole
edge only with an interval certificate over the span, not one sample.

**Consequence:** a knife edge / crack at a singular point is certified
non-degenerate, voiding §6.1's wedge term and everything that relies on
invariant 9 being honest.

**Existing tests:** `wedge.rs` tests use two *planes* (finite, well-defined
normals) and two *spheres*; no singular-surface sample, no NaN path.

---

### AUD-004 — parametric Krawczyk center term at `t_mid` does not certify the claimed property

```
Priority:   P1
Confidence: C2
Subsystem:  truck-evidence — BG-ENC-004-ISC certified enclosure for IntersectionCurve
File(s):    vendor/truck/truck-evidence/src/decorators/intersection_curve.rs
Symbol(s):  certify_cell (382-474), enclose_span (741-776), Sys::f_iv / j_iv
Introduced: BG-ENC-004-ISC (feat(evidence): certified enclosure for IntersectionCurve, bf2fd29)
```

**Formal obligation:** `certify_cell`'s doc (intersection_curve.rs:378-381)
and module doc (24-25): existence **and uniqueness** of the solution of the
double-projection system inside the returned box **for every `t` in the
cell**; `enclose(tt)` therefore contains the true intersection curve on the
whole span.

**Current code claims:** `K = m − Y·F(t_mid, m) + (I − Y·J(Q, cell))·(Q − m)`
with the center term evaluated at the single point `t_mid`
(intersection_curve.rs:428-434); strict interior containment of `K` certifies
the cell.

**Why that claim is wrong:** the parametric Krawczyk theorem requires the
center term to be an enclosure over the **parameter cell**: `F(cell, m)`.
`F(t_mid, m)` is a single point and `K_point ⊆ K_cell` (the true parametric
image), so `K_point ⊂ int(Q)` does **not** imply `K_cell ⊂ int(Q)`. The
certificate proves existence/uniqueness at `t_mid` only; the unexamined
variation `F(t, m) − F(t_mid, m) ≈ ∂F/∂t · (t − t_mid)` is absorbed only
empirically by the box widening loop (`pad` inflation) and by bisection, never
bounded.

**Minimal witness (1-D analogue):** `F(t, q) = q − (2t − 0.6)`, cell `[0,1]`,
`m = 0.5`, `Q = [0.2, 0.8]`, `Y = J = 1`. The code computes
`K = 0.5 − F(0.5, 0.5) + 0 = {0.4} ⊂ int(Q)` and certifies "unique in Q for
all t", but the true solution `q*(t) = 2t − 0.6` leaves `Q` for `t < 0.4`.

**Reproduction:** not yet reduced to a failing run against the real crate; the
code-level argument is complete. The practical reach is narrowed by the seed
hull (Q is the hull of the endpoint seeds, which contains the path while it is
monotone in each coordinate) and the pad growth — a non-monotone intersection
path is the residual risk class.

**Observed result:** a certified cell whose box can miss the curve.

**Correct behavior:** evaluate the center as `F(cell, m)` (an interval over
the t-cell), or bound `∂F/∂t` and include the cell-width term explicitly.

**Consequence:** `IntersectionCurve::enclose` — an evidence-bearing enclosure
consumed by the deviation and fidelity machinery — can under-enclose on
non-monotone intersection paths.

**Existing tests:** ISC tests use plane/plane, plane/cylinder, sphere/sphere
with well-behaved (monotone) intersection branches; none exercise a
non-monotone parameter path through a cell.

---

### AUD-005 — `RevolutedCurve::transformed` silently returns the wrong surface under non-conformal maps

```
Priority:   P1
Confidence: C2
Subsystem:  truck-geometry — canonical representation
File(s):    vendor/truck/truck-geometry/src/canonical.rs
Symbol(s):  Surface::transformed arm for RevolutedCurve (344), Transformed<Matrix4> for RevolutedCurve (468-481), transform_revolution_axis (485-493)
Introduced: BG-CE-006-r2 (new code in the build; canonical.rs is a new file)
```

**Formal obligation:** the representation operator and the transform layer
must not change geometry silently. `Surface::transformed(trans)` must return
the transformed image of the surface. The analytic carriers are guarded
(canonical.rs:352-393: any non-identity linear part routes through
`placed_surface`, where every evaluation composes the map exactly).

**Current code claims:** `Surface::RevolutedCurve(entity).transformed(trans)`
rebuilds a `RevolutedCurve` from the transformed profile, origin, and a
normalized image of the axis, for **any** affine matrix (canonical.rs:344 +
468-481).

**Why that claim is wrong:** the image of a surface of revolution under a
non-uniform scale or shear is generally **not** a surface of revolution. E.g.
scaling a circular cylinder by `diag(1, 2, 1)` yields an elliptic cylinder,
but the code returns the original circular cylinder rebuilt on the scaled
origin (the axis stays the z-axis, the profile is scaled in x only and rotated
about the same axis). Additionally, `transform_revolution_axis` substitutes
`unit_z()` when the axis image is zero/NaN (canonical.rs:485-493), silently
choosing a different axis instead of refusing.

**Minimal witness:** a revolved surface with profile `(1,0,0) → (1,0,1)` about
the z-axis, transformed by `diag(1, 2, 1)`. True image: elliptic cylinder.
Returned: original circular cylinder (max point error 1.0 for a unit radius).

**Reproduction:** subagent probe against the vendored crates (verified in
`target/debug`); the code path is direct at canonical.rs:344.

**Observed result:** a wrong surface silently returned as the transform.

**Correct behavior:** route non-conformal matrices through `Processor`/placed
composition exactly as the analytic carriers do, and refuse (or place) rather
than substitute a default axis.

**Consequence:** scaled/ sheared instances (STEP-scaled, per-instance
transforms) receive geometry that is not the transformed image, silently.

**Existing tests:** no transform test applies a non-conformal matrix to a
`RevolutedCurve`.

---

### AUD-006 — `sectional_curve` v-cut misindexing: wrong section + reachable panic

```
Priority:   P1
Confidence: C2
Subsystem:  truck-geometry — NURBS surface section extraction
File(s):    vendor/truck/truck-geometry/src/nurbs/bspsurface.rs
Symbol(s):  sectional_curve (1286-1340)
Introduced: PRE-EXISTING; the two changed lines (1299, 1303) are the BG-TOL-001
            tolerance migration in this build. In a changed file; flagged for
            completeness of the audit surface.
```

**Formal obligation:** `sectional_curve(bnd_box)` returns the section of the
surface over the parameter box `bnd_box` — the v-cut decisions must test the
**v** coordinate against the v-knots.

**Current code claims:** the u-cuts test `p[0]`/`q[0]` against the u-knots
(correct, bspsurface.rs:1291-1297); the v-cuts also test `p[0]`/`q[0]`
against the **v**-knots (bspsurface.rs:1299, 1303) and then call `vcut(p[1])`
and `vcut(q[1])`.

**Why that claim is wrong:** the v-cut decision reads the **u** coordinate of
the box. For a box `u∈[0,1], v∈[0,0.5]` on a `[0,1]×[0,1]` surface, `q[0]=1`
equals the last u-knot *and* the last v-knot, so the test does not fire
correctly and the section spans the full `v∈[0,1]` (wrong geometry). For a box
`u∈[0,0.5], v∈[0,1]`, `q[0]=0.5` is tested against the last **v**-knot
`1.0`, firing `vcut(q[1]=1.0)` at the back knot, producing a degenerate
surface that panics in `BSplineSurface::new` (unwrap).

**Minimal witness:** unit surface, box `u∈[0,0.5], v∈[0,1]` → panic; box
`u∈[0,1], v∈[0,0.5]` → wrong section (returns the full-v diagonal).

**Reproduction:** subagent probe against the vendored crates (verified).

**Observed result:** wrong section geometry and a reachable panic on valid
in-contract input.

**Correct behavior:** test `p[1]`/`q[1]` against the v-knots.

**Consequence:** the section-extraction consumer (surface-on-surface
intersection / arrangement scaffolding) gets wrong geometry or a process
abort.

**Existing tests:** no `sectional_curve` test with an asymmetric box.

---

### AUD-007 — `wedge_slope_lower_from_sin_margin` over-reports its certified lower bound

```
Priority:   P2
Confidence: C3
Subsystem:  truck-evidence — BG-FID-001 face-scale components
File(s):    vendor/truck/truck-evidence/src/fid/lfs.rs
Symbol(s):  wedge_slope_lower_from_sin_margin (245-256)
Introduced: BG-FID-001 (feat(evidence): fid face-scale components and wedge-slope evidence, 7fb5377)
```

**Formal obligation:** lfs.rs:4-10 and 222-238 — a certified **lower bound**
on `d(0, conv{n_A,n_B}) = cos(phi/2)` given `sin phi ≥ s`. A lower bound must
be conservative: it may refuse what the truth admits, never admit what the
truth refuses.

**Current code claims:** `value = sqrt((1 − sqrt(1 − s²))/2)`.

**Why that claim is wrong:** the expression cancels catastrophically: in
`1 − sqrt(1 − s²)` the subtraction of two nearby quantities turns an absolute
rounding error ~`eps` into a **relative** error ~`eps·s⁻²`. Round-to-nearest
lands *above* the true worst case `sin(asin(s)/2)`, so the certified "lower
bound" is sometimes not a lower bound.

**Minimal witness (machine-checked):**

| s (sin_margin) | computed | true `sin(asin(s)/2)` | relative over-report |
|---|---|---|---|
| 1e-6 (≈ TOLERANCE) | 5.0002222465e-7 | 5.0000000000e-7 | +4.4e-5 |
| 1e-8 | 7.45e-9 | 5.00e-9 | +49% |
| 1e-9 | 0.0 | 5.0e-10 | −100% (collapse, safe direction) |

**Reproduction:** `python` evaluation of the formula vs the series
`sin(asin(s)/2) = s/2 + s³/16 + 7s⁵/512 + …`.

**Observed result:** a "lower bound" larger than the true infimum over the
feasible wedge set at the realistic `tau_rep ≈ 1e-6` scale.

**Correct behavior:** evaluate `sin(0.5 * asin(s))` (or the series) with the
result rounded down.

**Consequence:** the wedge-slope gate can admit a wedge whose certified slope
bound is unattainable — in the unsound direction for a gate bound, though the
absolute magnitude (~1e-11) is small.

**Existing tests:** `wedge_slope_monotone_and_knife_limit` and
`wedge_formula_matches_geometry` compare against geometry at moderate angles;
no tiny-`sin_margin` comparison against the true worst case, so the
cancellation is unexercised.

---

### AUD-008 — degenerate single-face `[e, e.inverse()]` shell certified `Closed`/CoedgePairing/VertexLink

```
Priority:   P2
Confidence: C3
Subsystem:  truck-topology — invariant checkers
File(s):    vendor/truck/truck-topology/src/invariants/coedge_pairing.rs (50-72),
            src/invariants/vertex_link.rs (48-69), src/shell.rs (1173-1179),
            src/wire.rs (is_simple), src/solid.rs (try_new)
Introduced: BG-INV-101/102/103 (new invariants module in this build)
```

**Formal obligation:** §1.1 invariants 1-2 and the `Closed` shell condition
feed `Solid` construction and downstream arrangement; a degenerate zero-volume
"solid" must not be certified as a valid closed manifold boundary.

**Current code claims:** a single face whose boundary wire is `[e, e.inverse()]`
(one edge used twice, opposite sense) is `ShellCondition::Closed`
(shell.rs:1173-1179: `Oriented` with empty boundary set), so
`coedge_pairing::check` sets `CoedgePairing = True` and `vertex_link::check`
sets `VertexLink = True`, and `Solid::try_new` accepts it.

**Why that claim is wrong:** the wire is accepted as simple because `is_simple`
only checks vertex-stream distinctness (`[e, e.inverse()]` visits each vertex
once). The shell is a single face — zero volume, no boundary complex — yet the
whole checker suite certifies it as a closed manifold.

**Minimal witness:**

```
PROBE3 single-face [e,e.inverse()] shell_condition=Closed
PROBE3 coedge_pairing::check => OK (certifies CoedgePairing on a single-face degenerate shell)
PROBE3 vertex_link::check => OK
```

**Reproduction:** temporary probe against the crate (removed after the audit);
also `Solid::new(vec![slit_shell])` returns `Ok`.

**Observed result:** a degenerate shell is certified `Closed` with all
invariants true.

**Correct behavior:** `is_simple` must reject a wire that reuses an edge id
(any edge appearing more than once), and/or the closedness gate must require
each face to border two distinct faces.

**Consequence:** downstream consumers treating `CoedgePairing=True` /
`Closed` as a valid 2-manifold boundary proceed on a zero-volume degenerate
solid. The certificate is consistent with the *letter* of invariant 1 (two
coedges of opposite sense) but the module doc for `coedge_pairing` claims
"shared by exactly two **faces**", which is false here.

**Existing tests:** `coedge_pairing` and `vertex_link` tests build proper
2-face cubes / Möbius bundles; the degenerate single-face `[e, e.inverse()]`
case is absent as a negative.

---

### AUD-009 — full-circle → NURBS conversion degrades to negative/zero weights

```
Priority:   P2
Confidence: C2
Subsystem:  truck-geometry — canonical include path
File(s):    vendor/truck/truck-geometry/src/canonical.rs (676-679), src/specifieds/circle.rs (219-237)
Introduced: BG-CE-006 canonical routing (new code in this build)
```

**Formal obligation:** `Surface::include(&Curve::Circle(...))` must answer the
containment question for a circle on the surface. Circles (including closed
full-circle boundaries) are the most common boundary-curve shape.

**Current code claims:** every `Curve::Circle` is routed through
`ToSameGeometry::<NurbsCurve<Vector4>>` and the NURBS include path
(canonical.rs:676-679).

**Why that claim is wrong:** for a full circle (`angle = 2π`) the rational
quadratic conversion (circle.rs:219-237) produces a middle control point with
weight `cos(angle/2) = cos(π) = −1`. After `non_rationalized()` the 4D control
points carry `w = −1`; at the antipode the evaluated weight is exactly `0`, so
`subs(π)` is NaN (division by zero) and `search_parameter` for antipodal
points returns `None`. `include` therefore returns `false` for a full circle
that genuinely lies on the surface (verified: NURBS-cylinder include of a full
circle returns `false`; the same half-circle returns `true`; the plane path
disagrees between the enum and direct `Plane::include`).

**Minimal witness (machine-checked):** full-circle 3-point rational arc:
`(1,0,1), (−1,0,−1), (1,0,1)`; non-rationalized `(1,0,0,1), (1,0,−1,−1),
(1,0,0,1)`; 4D de Casteljau at `t = π` gives `w = 0` → NaN.

**Reproduction:** Python evaluation of the conversion (above); subagent probe
of the crate include paths.

**Observed result:** silent false-negative containment and NaN geometry for
full circles.

**Correct behavior:** special-case full circles (or arcs spanning ≥ 2π) in the
conversion, or route them through the analytic carrier's own certified
containment.

**Consequence:** closed-loop boundary curves (holes, boss outlines) are falsely
reported as not on their carrier; the same geometric question is answered
differently through different paths.

**Existing tests:** circle conversion tests use arcs < 2π; no full-circle
include test.

---

### AUD-010 — cone lower nappe inconsistent with the declared unbounded v domain

```
Priority:   P2
Confidence: C2
Subsystem:  truck-geometry — canonical Cone carrier
File(s):    vendor/truck/truck-geometry/src/specifieds/cone.rs
Symbol(s):  subs (93-98), parameter_range (121-124), include (147-165), search_parameter (233-267), search_nearest_parameter (269-297)
Introduced: new Cone carrier in this build (BG-TOL-001 / BG-ENC-002-CONE)
```

**Formal obligation:** `parameter_range` declares `v` unbounded
(cone.rs:121-124), so the lower nappe (`v < 0`) is part of the surface's own
declared domain.

**Current code claims:** `subs(u, v<0)` generates the lower nappe; but
`include` tests `radial ≈ z·tan` (cone.rs:155), which holds only for `v ≥ 0`,
so `include` returns `false` for points the surface itself generated; and
`search_nearest_parameter` returns the far-side point (`u` off by π, wrong
`v`) for lower-nappe queries.

**Minimal witness:** `cone.include(cone.subs(0.7, −3.0))` returns `false`
(verified); `search_nearest_parameter(Point3(0.5, 0, −3))` returns the far
side at distance ≈ 1.88 instead of the near side at ≈ 0.89.

**Reproduction:** subagent probe against the vendored crates (verified); the
`v` relation `radial = |v|·slope` vs `include`'s `radial = v·slope` is the
mechanism.

**Observed result:** inverse/containment inconsistency on half the declared
domain.

**Correct behavior:** either declare `v ∈ [0, ∞)` (single nappe, matching
STEP's CONICAL_SURFACE) or fix the predicates for `v < 0`.

**Consequence:** a point the cone generates cannot be certified as on the cone;
nearest-parameter queries on the lower nappe return wrong points, which feeds
vertex-weld / edge-projection consumers.

**Existing tests:** cone tests use `v ≥ 0`; the lower nappe is untested.

---

### AUD-011 — torus `normal_uder` returns a wrong z-component

```
Priority:   P2
Confidence: C3
Subsystem:  truck-geometry — Torus carrier
File(s):    vendor/truck/truck-geometry/src/specifieds/torus.rs (121-124)
Introduced: PRE-EXISTING; the file is changed in this build (tolerance migration
            only on search_parameter). Exercises the fillet Newton path through
            contact_circle.rs's use of `normal_uder`.
```

**Formal obligation:** `normal_uder = ∂/∂u normal(u,v)`.

**Current code claims:** `(−cos v·sin u, cos v·cos u, sin v)`.

**Why that claim is wrong:** `normal = (cos v·cos u, cos v·sin u, sin v)`;
differentiating in `u` gives `(−cos v·sin u, cos v·cos u, 0)` — the returned
z-component `sin v` is wrong (should be 0). Finite-difference verified.

**Reproduction:** `d/du (cos v cos u, cos v sin u, sin v)` at `v = 1`: analytic
z is 0.841, finite-difference z is 0.0.

**Observed result:** `normal_uder` is not the derivative of `normal`.

**Correct behavior:** return `(−cos v·sin u, cos v·cos u, 0)`.

**Consequence:** `contact_circle.rs:164` (`next_point`) uses `normal_uder` in
the rolling-ball fillet Newton solve; torus fillets solve with a wrong
derivative.

**Existing tests:** no test asserts the torus normal derivative against a
finite difference.

---

### AUD-012 — reachable panics in the fillet approximation path (H-1)

```
Priority:   P2
Confidence: C3
Subsystem:  truck-geometry — af_surface / contact_circle
File(s):    vendor/truck/truck-geometry/src/decorators/af_surface.rs (442, 468, 477, 539),
            src/decorators/rbf_surface/contact_circle.rs (149, 168, 176)
Introduced: PRE-EXISTING; the files are in the audit surface (BG-S0-002 /
            BG-NUM-001-FILLET changed them), and the build's own house rule
            H-1 (no panics on data) is violated.
```

**Formal obligation:** H-1 — no `unwrap`, `expect`, `panic!` on any path
reachable from untrusted geometry.

**Current code claims:** the rolling-ball fillet refinement path is total.

**Why that claim is wrong:** `KnotVec::try_from(vec).unwrap()` panics on a
reversed edge parameter range; `mat.invert().unwrap()` panics on a singular
`[uder, vder, n]` frame (degenerate contact); `ccs.sort_by(|x,y|
x.partial_cmp(y).unwrap())` panics on a NaN parameter; `contact_circle.rs:176`
`debug_assert!(del.z.so_small())` fires in debug builds on scaled models.

**Reproduction:** subagent probe (panics observed). Reachability is via
degenerate/tangent fillet inputs — the exact inputs the packets' refusals were
supposed to convert into typed `Outcome`s.

**Observed result:** process abort on valid-in-contract geometry.

**Correct behavior:** thread `Outcome`/`Option` through these sites, converting
each panic into a typed refusal.

**Consequence:** a fillet on degenerate geometry aborts the process instead of
returning `NumericallyUnresolved`/`UnsupportedEnvelope` — the opposite of the
packets' stated contracts.

**Existing tests:** fillet tests use well-conditioned witnesses; the degenerate
frame cases are not exercised.

---

### AUD-013 — `Sphere::search_nearest_parameter(center)` returns `Some((NaN, NaN))`

```
Priority:   P2
Confidence: C2
Subsystem:  truck-geometry — Sphere carrier
File(s):    vendor/truck/truck-geometry/src/specifieds/sphere.rs (238-257)
Introduced: PRE-EXISTING; file changed in this build (tolerance migration).
```

**Formal obligation:** `search_nearest_parameter` must return `None` where no
parameter exists; a query at the sphere's own center has no nearest parameter.

**Current code claims:** normalizes `point − center` with no zero guard; at the
center the vector is zero and the returned `(u, v)` is `(NaN, NaN)` wrapped in
`Some`.

**Reproduction:** `search_parameter(center)` correctly returns `None` (the
radius guard); `search_nearest_parameter(center)` returns `Some((NaN, NaN))`.

**Observed result:** `Some((NaN, NaN))` for a point with no nearest parameter.

**Correct behavior:** return `None` when `‖point − center‖` is below tolerance.

**Consequence:** consumers (edge projection, nearest-point welding) that trust
`Some` can feed NaN parameters into curve/surface evaluation.

**Existing tests:** no center-query test for `search_nearest_parameter`.

---

### AUD-014 — same-parameter certifies vacuously after `pre_cut` discards pcurves

```
Priority:   P3
Confidence: C2
Subsystem:  truck-topology — same-parameter invariant
File(s):    vendor/truck/truck-topology/src/invariants/same_parameter.rs (84-110)
Introduced: BG-INV-104 (new invariants module in this build)
```

**Formal obligation:** `Prop::SameParameter` = "every edge use's parametric
trace agrees with the edge's leader curve over the whole span".

**Current code claims:** when `edge.pcurve()` is `None`, the checker returns
`SameParameter = True` (vacuous-holds, same_parameter.rs:97-110).

**Why that claim is wrong:** `Edge::pre_cut` drops the pcurve on **both**
halves (edge.rs:626-651), so after `Shell::cut_edge` every cut edge
re-certifies `SameParameter = True` with no trace to check. The certificate
says the invariant holds; the evidence for it is the absence of a trace, which
is a weaker statement than the prop's text. Documented as vacuous in the
module doc, but the emitted prop is the full invariant's prop.

**Consequence:** a consumer reading `SameParameter=True` as "the trace agrees"
is silently told so about edges whose trace was discarded by cutting.

**Existing tests:** `same_parameter.rs` tests certify real pcurves; the vacuous
arm is tested only as "certifies a hold" for the `()` default.

---

### AUD-015 — vertex-link certifies a "single cycle" where only connectivity holds

```
Priority:   P3
Confidence: C3
Subsystem:  truck-topology — vertex-link invariant
File(s):    vendor/truck/truck-topology/src/invariants/vertex_link.rs (48-69)
Introduced: BG-INV-102 (new in this build)
```

**Formal obligation:** `Prop::VertexLink` = vertex link is a single cycle
(§1.1 invariant 2).

**Current code claims:** `singular_vertices().is_empty()` ⇒
`VertexLink = True`.

**Why that claim is wrong:** `singular_vertices` tests link **connectivity**
only; "connected = single cycle" holds only on a closed shell. The checker
neither requires nor verifies closure; its own doc says the closure pre-check
"belongs to BG-INV-101's checker", which nothing enforces. Probe: an open
single-triangle shell (every link a path) certifies.

**Consequence:** on open shells the prop over-states what was checked. The
code's doc is honest about this, so it is a documented over-claim rather than a
silent one.

**Existing tests:** `vertex_link_documented_dependency_on_closed` explicitly
asserts the open-shell hold — the limitation is intentional.

---

### AUD-016 — sphere `immersion_lower_bound` uses non-directed arithmetic for a lower-bound certificate

```
Priority:   P3
Confidence: C1
Subsystem:  truck-evidence — Sphere enclosure
File(s):    vendor/truck/truck-evidence/src/sphere.rs (160-161)
Introduced: BG-ENC-002-SPHERE (new in this build)
```

**Formal obligation:** `immersion_lower_bound` returns a lower bound on
`‖S_u × S_v‖` (BG-ENC-003 outward rounding: a lower bound must round down).

**Current code claims:** `(r·r·sin(uu).inf()).max(0.0)` computed in plain
round-to-nearest f64.

**Why that claim is wrong:** round-to-nearest can round **up**, making the
"lower bound" exceed the true minimum by an ulp. The sibling implementations
(cone.rs:146-162, `immersion_lower_bound_box`) use directed rounding via
`inari` and read `.inf()`. Severity is ulp-scale and was not demonstrated to
bite, but the direction is the one the crate's own rules forbid.

**Consequence:** a consumer certifying immersion from `ι > 0` has no formal
guarantee.

**Existing tests:** compare against `r²·sin(u_min)` with a float slack that
masks the direction.

---

### AUD-017 — hyperbola/parabola containment predicates compare lengths against a dimensionless ratio margin

```
Priority:   P3
Confidence: C3
Subsystem:  truck-geometry — specifieds
File(s):    vendor/truck/truck-geometry/src/specifieds/hyperbola.rs (118-135),
            src/specifieds/parabola.rs (122-132)
Introduced: BG-TOL-001 migration in this build.
```

**Formal obligation:** a model-space length must be compared via
`is_small_len` (scaled by model scale), not `is_small_ratio` (dimensionless).

**Current code claims:** `(p − subs(t)).magnitude()` is compared with
`is_small_ratio`, a dimensionless margin.

**Why that claim is wrong:** a length is compared against a dimensionless
tolerance. Inert at Stage A (`model_scale = 1.0` makes them equal), but under a
real `model_scale` the predicate loosens/tightens wrongly.

**Consequence:** wrong containment answers on models whose scale differs from
1.

**Existing tests:** none with a non-unit model scale.

---

## 4. Coverage matrix

Every changed kernel source file under `vendor/truck` appears below. Review
depth: **DEEP** = read in full + adversarial reasoning; **REVIEW** = read in
full; **SPOT** = read key paths / diff; **COVERAGE GAP** = not reviewed to the
depth the file warrants.

| file | subsystem | claims reviewed | tests inspected | adversarial probe | findings |
|------|-----------|-----------------|-----------------|-------------------|----------|
| truck-base/src/evidence.rs | evidence algebra | Outcome/Certificate/Truth/Modulus/Budget semantics | lib tests | read | AUD-017 (none); Modulus compose domain edge noted in §8 |
| truck-base/src/tolerance.rs | tolerance ctx | scale-relative predicates, one-sided margins | lib tests | read | none |
| truck-evidence/src/enclosure.rs | enclosure interface | Box3/DirCone, cross_box, midpoint_ball_cone, immersion_lower_bound_box | lib tests | read | none (sound) |
| truck-evidence/src/elementary.rs | certified sin/cos | series, reduction, interior extrema | sampling + sweep tests | read | none (sound) |
| truck-evidence/src/plane.rs | plane enclosure | affine | tests | read | none |
| truck-evidence/src/line.rs | line enclosure | affine | tests | read | none |
| truck-evidence/src/circle.rs | unit-circle enclosure | trig, cones | tests | read | none |
| truck-evidence/src/cone.rs | cone enclosure | enclose/der/normal_cone/immersion | sweep | read | none |
| truck-evidence/src/cylinder.rs | cylinder enclosure | enclose/der/normal_cone | sweep | read | none |
| truck-evidence/src/sphere.rs | sphere enclosure | enclose/der/normal_cone/immersion | tests | **probe run** | AUD-001, AUD-016 |
| truck-evidence/src/torus.rs | torus enclosure | enclose/der/normal_cone/immersion | sweep | read | none |
| truck-evidence/src/nurbs.rs | NURBS enclosure | hull property, weights | tests | read | none found |
| truck-evidence/src/bspline.rs | B-spline enclosure | hull property, Boehm insertion | tests | read | none found |
| truck-evidence/src/analytic/* (9 files) | analytic intersections | classification predicates, exactness | packet tests | read | none found (pure classification; no interval image certificates) |
| truck-evidence/src/decorators/extruded.rs | extruded enclosure | cross_box/cones/chain rule | tests | read | none |
| truck-evidence/src/decorators/revolved.rs | revolved enclosure | rotation derivatives | tests | read | none |
| truck-evidence/src/decorators/processor.rs | placed enclosure | homogeneous image, matrix comp | tests | read | none |
| truck-evidence/src/decorators/pcurve.rs | pcurve enclosure | 3D composition chain rules | tests | read | HULL_PAD empirical bound (C1, §8) |
| truck-evidence/src/decorators/intersection_curve.rs | ISC enclosure | parametric Krawczyk, seed/bisect | tests | read | **AUD-004** |
| truck-evidence/src/decorators/offset.rs | offset stub | — | — | read | none |
| truck-evidence/src/deviation.rs | BG-CE-002 deviation | route1/route2 soundness | tests | **probe run** | **AUD-002** |
| truck-evidence/src/num/krawczyk.rs | Krawczyk operator | strict-interior rule, bisection, widening | tests | read | none (sound) |
| truck-evidence/src/num/roots.rs | Bernstein isolation | sign changes, width floor | tests | read | none (sound; endpoint/dyadic refusal documented) |
| truck-evidence/src/num/cluster.rs | certified clustering | overlap predicate, enclosing ball, admissibility | tests | read | none (enclosing-radius rounding note in §8) |
| truck-evidence/src/fid/isotopy.rs | isotopy conditions (i)-(iv-a) | closeness/angle/endpoint/fibre gates | tests | read | none found |
| truck-evidence/src/fid/one_sheet.rs | fibre degree-one | Krawczyk counting, disc membership | double-cover tests | read | resolution-honest; documented |
| truck-evidence/src/fid/rep.rs | rep operator curve+surface | Hermite emitter, (iv-b), surface grid | double-sheet tests | read | none found (certificates honestly scoped) |
| truck-evidence/src/fid/lfs.rs | face-scale / wedge slope | curvature radius, wedge slope | tests | Python-checked | **AUD-007** |
| truck-evidence/src/harness.rs, lib.rs, num/mod.rs, fid/mod.rs, decorators/mod.rs | scaffolding | — | — | read | none |
| truck-geometry/src/canonical.rs | canonical repr/transform/include | transformed, lift_up, include routing | tests | read | **AUD-005, AUD-009** |
| truck-geometry/src/decorators/af_surface.rs | fillet approximation | budget, panics, sampling heuristic | tests | read | **AUD-012**; D8 sampling heuristic (§8) |
| truck-geometry/src/decorators/rbf_surface/algo.rs, contact_circle.rs | fillet contact | invert unwraps, normal_uder use | tests | read | AUD-011 (feeds), AUD-012 |
| truck-geometry/src/decorators/intersection_curve.rs | geometry ISC | — | — | read | pre-existing `subs` unwrap (C2, §8) |
| truck-geometry/src/decorators/offset/{curve,surface}.rs | offset | tolerance migration | — | spot | none |
| truck-geometry/src/decorators/revolved_curve.rs | revolved carrier | search/proj branch, seam | tests | read | D10 (−π,π] seam range (P3-class, §8) |
| truck-geometry/src/nurbs/knot_vec.rs | knot vector | basis, multiplicity, tolerance | tests | read | none found |
| truck-geometry/src/nurbs/bspcurve.rs | B-spline curve | subs/cut/derivatives | tests | read | none |
| truck-geometry/src/nurbs/bspsurface.rs | B-spline surface | sectional_curve, cuts | tests | read | **AUD-006** |
| truck-geometry/src/nurbs/nurbscurve.rs, nurbssurface.rs, mod.rs | NURBS | tolerance migration | tests | spot | none |
| truck-geometry/src/specifieds/{circle,line,plane,sphere,torus}.rs | carriers | conversions, inverses, tolerance | tests | read | AUD-009, AUD-013, AUD-011 |
| truck-geometry/src/specifieds/cone.rs | cone carrier | subs/include/search/nearest | tests | read | **AUD-010** |
| truck-geometry/src/specifieds/cylinder.rs | cylinder carrier | parameterization | tests | read | none |
| truck-geometry/src/specifieds/{hyperbola,parabola}.rs | conics | tolerance dimension | tests | read | **AUD-017** |
| truck-geotrait/src/algo/curve.rs, surface.rs, traits/mod.rs | traits | Transformed, ToSameGeometry, Outcome | tests | read | none |
| truck-meshalgo (7 changed src files) | tessellation/analyzers | tolerance migration | — | spot | none (migration-only) |
| truck-modeling/src/geometry.rs, builder.rs, geom_impls.rs | modeling | include routing, Outcome | tests | spot | D9 unwrap noted §8 |
| truck-polymesh/src/polyline_curve.rs | polyline | tolerance | — | spot | none |
| truck-shapeops fillet/{experiment,mod}.rs | fillet | budget spending, refusals | tests | spot | none found beyond AUD-012's file deps |
| truck-shapeops healing/split_closed_faces.rs | healing | tolerance | — | spot | none |
| truck-shapeops transversal/{divide_face,loops_store,polyline_construction,intersection_curve} | arrangement | discovery/commit split, tolerance | tests | spot | none found |
| truck-stepio/src/in/mod.rs, out/geometry.rs, step_geometry/* | STEP IO | tolerance migration, new DisplayByStep | tests | spot | none found |
| truck-topology/src/edge.rs | edge | identity, pre_cut pcurve drop | tests | read | AUD-014 (pre_cut) |
| truck-topology/src/entity_id.rs | identity | OpId, selectors | tests | read | §8 (hash/selector concerns) |
| truck-topology/src/face.rs, wire.rs | face/wire | is_simple, boundaries | tests | read | AUD-008 (is_simple) |
| truck-topology/src/shell.rs, solid.rs | shell/solid | shell_condition, Boundaries, Solid::try_new | tests | **probe run** | AUD-008 |
| truck-topology/src/vertex.rs, lib.rs | vertex/lib | — | tests | read | none |
| truck-topology/src/invariants/{mod,coedge_pairing,domain_boundary,euler_poincare,representation,same_parameter,shell_nesting,vertex_link,wedge}.rs | invariants | all nine invariants | tests | **probe run** | AUD-003, AUD-008, AUD-014, AUD-015; domain_boundary/euler/nesting documented-limitation notes §8 |

No changed source file silently disappears from this table. Files marked
`spot`/`COVERAGE GAP` below were reviewed at the depth indicated; the
migration-only meshalgo/stepio/modeling/shapeops sites were not deep-audited
for every tolerance-equivalence (they were classified by the BG-TOL-001 packets
and are outside the evidence-bearing hot path).

## 5. Cross-module claim map

Producer → consumer edges, and whether the semantic strength matches:

| # | Producer claim | Consumer assumption | Match? |
|---|---|---|---|
| 1 | `Sphere::normal_cone` = cone ⊇ normals | §9.1 transversality predicate: min angle over two cones | **NO** — producer's cone can be too small (AUD-001); consumer would certify a too-large minimal angle. |
| 2 | `certify_deviation` = whole-span bound | same-parameter invariant, pcurve trace | **NO** on degree-0 splines (AUD-002); certified bound can be false. |
| 3 | `IntersectionCurve::enclose` ⊇ curve | deviation / fidelity consumers | **NO** (AUD-004): cell certificate covers `t_mid` only; enclosure can under-estimate on non-monotone paths. |
| 4 | `wedge_slope_lower_from_sin_margin` = lower bound | fid scale gates (`ReachLowerBoundTooSmall` refusals) | **NO** at small margins (AUD-007): bound can exceed the true infimum, loosening gates. |
| 5 | `WedgeNonDegeneracy = True` | §6.1 wedge term in lfs; invariant 9 | **NO** on NaN normal (AUD-003) and single-sample (AUD-003). |
| 6 | `ShellCondition::Closed` + invariant props | `Solid::try_new`, downstream arrangement | **NO** on degenerate `[e, e.inverse()]` shells (AUD-008). |
| 7 | `Surface::transformed(RevolutedCurve)` | placed/scaled instance geometry | **NO** under non-conformal maps (AUD-005). |
| 8 | rep_curve/rep_surface certificate | (iv-b) → downstream arrangement (§6.3) | **YES** — certificates are honestly scoped: "(i)-(iii) + (iv-b) on this partition", explicitly NOT isotopy. The open-lemma bridge is never silently crossed. |
| 9 | `CurveScaleComponents` / `SurfaceScaleComponents` | tube/rep gates | **YES** — named lower bounds; `tube_scale_lower()` is documented as a gate bound only. |
| 10 | `cluster` enclosing ball + admissibility | §5 collapse quotient | **YES** (sound); enclosing-radius rounding is the only C1 note (§8). |
| 11 | `isolate_roots` / `krawczyk` | fibre isolation, root counts | **YES** (sound); endpoint/dyadic refusals are typed. |
| 12 | `torus::normal_uder` | fillet Newton (`contact_circle`) | **NO** (AUD-011): wrong derivative feeds the fillet solve. |

The two structural compositions that can strengthen a claim (rows 1 and 3)
are exactly the ones the packet-local tests did not exercise.

## 6. Test-reality findings

- **Sphere normal-cone gap:** `sphere_normal_cone_over_patch` never queries a
  wide-azimuth patch whose corner half-angle stays < π/2 while the interior
  bulges past it. Vacuous-pass possibility: none for the existing test, but the
  defect is invisible to it.
- **Deviation degree-0 gap:** no test feeds degree-0 splines. The existing
  "exact pair certifies one-shot" tests use degree ≥ 1 and therefore cannot
  fail under AUD-002.
- **Wedge NaN gap:** no test samples an edge at a singular surface point; the
  `NaN < sin_margin` silent-pass is untested.
- **Krawczyk center-term gap:** ISC tests use monotone intersection branches,
  so the point-center certificate coincides with the truth there.
- **Degenerate-shell gap:** the invariant test suites only build proper
  2-face witnesses; the `[e, e.inverse()]` single-face shell is absent as a
  negative.
- **Positive tests that DO bite:** `double_cover_rep_never_emits`,
  `double_sheet_is_multisheet`, `boundary_root_on_disc_edge_is_unresolved`,
  `tangential_contact_is_unresolved_not_degree_one`, the margin-sweep tests,
  and the `tangential_double_root_refuses_indeterminate` Krawczyk test all
  genuinely exercise their negative branches. These are the strongest tests in
  the build and none of them is vacuous.
- The `plane_properties` proptest regression file and the tolerance migration
  tests are equivalence-preserving at Stage A and do not probe soundness.

## 7. Open lemmas / deliberately unproved claims

Explicitly open theorem bridges are **not** defects. The build is disciplined
about them:

- **FID-L-TUBE, FID-L-FEDERER-PATCH, FID-L-COVERING, FID-L-SEPARATES** (isotopy
  chain): documented OPEN in isotopy.rs/rep.rs; rep certificates say "(i)-(iii)
  + (iv-b) on this partition", never isotopy. No executable code crosses these.
- **L-WEDGE-SLOPE, L-COVERAGE** (fid/lfs): cited as fed/open; the 
  `WedgeSlopeLowerBound` type claims only "local normalized-slope lower bound",
  not global χ_K.
- **BG-FID-008 (iv-a)**: one witnessed fibre per component is certified; the
  promotion to whole-span one-sheetness is explicitly attributed to L-COVERING.
- **BLD-CNR-SETBACK-001, FIL-VAR-001, TAN-SNAP-001, CHM-VAR-001**: the
  underived coupled discriminants are out of scope (Stage 4/5); no code claims
  them.

Nothing in the audited code behaves as though an OPEN lemma were proved. The
defects in this report are NOT open-lemma promotions; they are failures of
certificates the code *does* emit.

## 8. Audit limitations

- **Tree advanced during the audit.** The audited snapshot is `f919228`. The
  branch tip moved to `bec92b6` while this report was being written, landing
  BG-INV-107 (`truck-topology/src/tolerance_store.rs`,
  `truck-topology/src/invariants/tolerance_monotonicity.rs`, +449 LOC) which is
  **outside** the audited snapshot and is not covered by the coverage matrix.
  Re-verify the findings in this report against any later tip before acting on
  them; a fix packet must be built on a tree that includes INV-107's checks.
- The build was reviewed primarily by static reading plus targeted executable
  probes (sphere cone, degree-0 deviation, degenerate shell, wedge NaN) and
  machine-checked arithmetic. The full workspace test suite was not re-run
  end-to-end (238+59 lib tests confirmed green on the two core crates).
- **COVERAGE GAP:** `truck-evidence/src/fid/rep.rs` (4,652 lines) was read in
  full but its most intricate surface algebra (sliver routing, per-axis
  refinement, wrapped Chebyshev adjacency) was not exhaustively re-derived;
  no defect was found beyond the honest-scoping notes, but this is the file
  most likely to hide a subtle unsoundness.
- The migration-only meshalgo/stepio/modeling/shapeops tolerance sites were
  spot-reviewed; a strict per-site equivalence proof was not done.
- `Modulus::compose`'s domain handling: for Lipschitz `k>1` moduli on finite
  domains the composed domain `min(d1,d2)` can exceed the true validity domain
  `d2/k1`. Unreachable in the current moduli table (all infinite domain except
  `Pole`, which refuses composition) — C1, not raised to a finding.
- `HULL_PAD = 64ε(1+|·|)` (bspline/pcurve/deviation) is an empirical pad for
  Boehm-insertion rounding, not a proven bound; no escape was measured. C1.
- `cluster`'s enclosing radius uses round-to-nearest `sqrt`; a one-ulp
  underestimate of an "upper bound" is possible. C1, not raised.
- `af_surface`'s `tol` success criterion samples 3 parameters per interval
  (D8): the `Method::Float` certificate claims nothing stronger, so it is a
  documented heuristic, not an over-claim.
- Pre-existing defects in changed files (AUD-006, 011, 012, 013) are included
  because the changed-file diff is the audit surface and the build's own code
  depends on the affected paths (fillet, section extraction); their provenance
  is marked so the owner can decide scope.
- `entity_id` OpId is a 64-bit hash (collision risk ~2⁻⁶⁴ per pair, documented
  "content identity") and selector indices are structural (stale after
  mutation); both are documented design choices, recorded here as risks, not
  raised to findings.
- `domain_boundary` certifies the full invariant's prop while checking only
  the topological half (pcurve correspondence deferred); `shell_nesting`
  certifies a "forest" after checking acyclicity; `euler_poincare` is honest
  "necessary only". All three are documented-limitation P3-class items noted in
  §3/§4 rather than raised as separate findings.

## 9. Recommended next packets

For each C2/C3 P0–P2 defect. No implementation detail beyond the correction
objective and a regression witness.

| proposed packet id | affected files | correction objective | suggested regression witness |
|---|---|---|---|
| BG-AUD-FIX-001 | truck-evidence/src/sphere.rs | make `normal_cone` a sound cone for azimuth-wide patches (emit everything-cone when the azimuth span exceeds π, or bound the true geodesic diameter) | probe: `uu=[0.5,0.6]`, `vv=[0,3.6]` must not return the 33.4° corner cone |
| BG-AUD-FIX-002 | truck-evidence/src/deviation.rs | union `subs(lo)`/`subs(hi)` of the source curves into the route-1 piece hull (or refuse degree-0 difference splines) | probe: degree-0 carrier/leader half-span `[0,0.5]` with a step at 0.5 must refuse |
| BG-AUD-FIX-003 | truck-topology/src/invariants/wedge.rs | refuse on non-finite normals; replace the single midpoint sample with a whole-span interval certificate | witness: edge midpoint at a cone apex; edge with a fold off-midpoint |
| BG-AUD-FIX-004 | truck-evidence/src/decorators/intersection_curve.rs | evaluate the Krawczyk center as `F(cell, m)` or add an explicit `∂F/∂t` width term | witness: non-monotone 1-D parametric system where the true root leaves Q inside a certified cell |
| BG-AUD-FIX-005 | truck-geometry/src/canonical.rs | route non-conformal matrices through placed composition for `RevolutedCurve`; refuse (not substitute) a degenerate axis image | witness: `diag(1,2,1)` of a revolved cylinder must equal the elliptic cylinder or refuse |
| BG-AUD-FIX-006 | truck-geometry/src/nurbs/bspsurface.rs | test `p[1]`/`q[1]` against the v-knots in `sectional_curve` | witness: box `u∈[0,0.5], v∈[0,1]` must not panic and must return the correct section |
| BG-AUD-FIX-007 | truck-evidence/src/fid/lfs.rs | evaluate the wedge slope as `sin(0.5·asin(s))` with downward rounding | witness: `s=1e-8` must not exceed the true `sin(asin(s)/2)` |
| BG-AUD-FIX-008 | truck-topology/src/wire.rs, invariants/* | reject a wire that reuses an edge id in `is_simple`; make closedness require ≥ 2 distinct faces per edge | witness: single-face `[e, e.inverse()]` shell must not certify `Closed` |
| BG-AUD-FIX-009 | truck-geometry/src/canonical.rs, specifieds/circle.rs | special-case full-circle conversion (or route analytic containment) so `include` is correct for closed loops | witness: full circle on a plane/cylinder must include as true |
| BG-AUD-FIX-010 | truck-geometry/src/specifieds/cone.rs | declare the single nappe (`v ≥ 0`) or fix predicates for `v < 0` | witness: `search_nearest_parameter` on a lower-nappe query must return the near point |
| BG-AUD-FIX-011 | truck-geometry/src/specifieds/torus.rs | return the true `∂/∂u` normal | witness: finite-difference of `normal` at `v=1` vs `normal_uder` |

The owner/orchestrator decides whether and in what order to spec and dispatch
these; this audit does not write implementation packets.

---

*Sources consulted:* the audited tree at `f919228`; the formal system
`docs/FORMAL_SYSTEM_BREP_GENERATION.md` (revision 4) and `docs/
GENERATION_KERNEL_BUILD_SPEC.md` (the build specification, revision-4 synced);
the full first-parent Git history of `integration/kernel-bg` from
`d1f9c5bd` to `f919228`; executable probes and machine-checked arithmetic as
cited per finding. Executable reality is treated as authoritative wherever it
disagrees with prose.
