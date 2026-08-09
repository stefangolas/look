# Accuracy findings: where the world-space error bound is lost

**Scope:** the executed tessellation path from `parse_step` to `PolygonMesh`,
traced backward from final face triangulation through edge subdivision,
boundary projection, surface `parameter_division`, interior Steiner-point
generation, constrained triangulation, the analytic recovery routes, and shell
compression.

**Method:** static read of the executed source. No probe was run and no new
instrumentation was added; every number quoted below is either a constant in the
source or a measurement already recorded in a source comment, and is attributed
as such.

**Audited artifact:** `look` `integration/formal-atlas-wave-2`, truck fork
pinned `3a81a169`. Provenance discharged: `cargo tree -p truck-meshalgo`
resolves to the git rev, and the `paths` override in `.cargo/config.toml` is
fully commented out — unlike the state `look-build-quirks` warns about, which
invalidated manifest v1.0.

**Line references** are into
`truck-meshalgo/src/tessellation/triangulation.rs` unless otherwise qualified.

---

## 0. The question, answered

> Does Truck guarantee that every final triangle lies within the requested
> world-space tolerance of the source B-rep surface and trim region?

**No.** An error-controlled mesher needs

```
source surface / trim
  → locally certified approximation regions
  → triangulation constrained to those certified regions
  → final triangles individually inherit the error bound
```

Truck maintains links 1 and 2, subject to the heuristic caveats in §1 and §3,
and **breaks link 3 in `insert_surface`**. Certified regions are produced —
they are the cells of the `parameter_division` grid — but the triangulation is
not constrained to them wherever a grid point is declined, and even where it is
constrained, the surface that was certified is not the surface that is realized.

Three breaks, independent of one another, are established below. Two further
findings (§6) change how the ranking reads.

---

## 1. Stage audit

| Stage | Criterion actually enforced | Space measured in | Bound or heuristic |
|---|---|---|---|
| Edge subdivision (`algo::curve::parameter_division`) | one jittered sample vs the chord, `p ∈ [0.4, 0.6]` | world 3D | **heuristic** |
| pcurve / trim subdivision | *none — no pcurves are read* | — | n/a |
| Boundary → UV projection (`sp`) | residual vs `tol × COMPATIBILITY_FACTOR` | world 3D | **gate disabled** (`= f64::INFINITY`) |
| `parameter_division`, `Plane` | `2 × 2` | — | **exact (zero)** |
| …`Sphere` / `Torus` | closed-form sagitta inversion from the radii | world 3D | **true bound** |
| …`RevolutedCurve` | generatrix division + `acos(1 − tol/max)` | world 3D | **true bound, conservative** |
| …`BSplineSurface` / `NurbsSurface` | one jittered sample vs the *bilinear interpolant* | world 3D | **heuristic** |
| `insert_surface` interior grid | inherits the above; points gated on `Inside` | UV for gating | **certification does not transfer** |
| Constrained triangulation (Spade) | Delaunay empty-circumcircle | **UV, not 3D** | no 3D quality criterion at all |
| Analytic plane / cylinder / cone | boundary polygon only, **no interior points** | — | exact *given* the boundary |
| Analytic torus | explicit `nu × nv` from tol and both radii, clamped `[8,256] × [4,128]` | world 3D | bound, clamp aside |
| `wrap_shell` compression | topology and vertices preserved, edges sampled once by id | — | **not a contributor** |

`RevolutedCurve`'s `max` is the largest revolved radius over the whole
generatrix division, so the circumferential count is sized by the widest point
and is conservative for narrower ones. That is the right direction.

---

## 2. Break 1 — both adaptive kernels are single-sample tests

`truck-geotrait/src/algo/curve.rs:117`, `algo/surface.rs:258`.

The curve kernel tests **one** point per interval and, on success, never
examines any sub-interval:

```rust
let dist2 = curve.subs(t).distance2(mid);
if dist2 < tol * tol || trials == 0 || *budget == 0 {
    (vec![range.0, range.1], vec![ends.0, ends.1])   // accepted, never revisited
}
```

A curve symmetric about its midpoint — an S-shaped inflection, ubiquitous in
B-spline trim curves — has near-zero mid-chord deviation and is accepted at the
top level with unbounded true error. The `p ∈ [0.4, 0.6]` jitter narrows the
blind spot; it does not remove it. The surface kernel has the same hole in two
dimensions.

The surface kernel carries an additional structural limit. `divide_flag0` is
indexed by u-interval and `divide_flag1` by v-interval, so split decisions are
**per row and column, not per cell**. It is a pure tensor grid: one failing cell
splits an entire row *and* an entire column, and a locally detailed region
forces globally dense sampling.

All three budgets terminate silently with the tolerance unmet:

- `MAX_CURVE_POINTS = 1 << 16` (`algo/curve.rs:91`)
- `MAX_DIVISION_CELLS = 1 << 16` (`algo/surface.rs:256`)
- `trials = 100` on both

On a large trimmed spline face the cell cap is reachable, and
`sub_parameter_division` simply returns.

**Verdict:** a sampling heuristic, not a bound. Necessary but not sufficient.

---

## 3. Break 2 — the boundary-compatibility gate is deliberately off

`:3369` and `:3738`.

`sp()` computes the residual between the boundary point and the surface it is
supposed to trim, then compares it to `tol × compatibility_factor()`:

```rust
let residual = surface.subs(u, v).distance(pt);
if residual > tol * compatibility_factor() { /* refuse */ }
```

with

```rust
const COMPATIBILITY_FACTOR: f64 = f64::INFINITY;
```

Boundary vertices are stored as their **original 3D curve points**
(`boundary_map.insert(idx, pt.point)`, `:4902`/`:4907`), and
`triangulation_into_polymesh_outcome` prefers `boundary_map` over
`surface.subs(u, v)` (`:5912`). So a vertex known not to lie on the surface is
spliced into a triangle fan whose other vertices do lie on it. The discrepancy
becomes mesh geometry.

The measured population, recorded in the source comment at `:3729` for ABC
`00009190`: **315 rejected points at a median of 191× tolerance and a maximum of
617×**, with loosening the factor twentyfold removing only 62 of them.

This is a documented, conscious accuracy-for-coverage trade — the comment notes
that enabling the gate costs 292 faces and 21,131 triangles while changing no
visible blob shell. It is nonetheless real, localized, spectacular error, and it
is the largest per-vertex deviation anywhere in the pipeline.

---

## 4. Break 3 — `insert_surface` does not triangulate the certified regions

`:5651`. **This is the architectural break.** Two distinct failures.

### 4.1 The certified surface is not the realized surface

The accept test in `sub_parameter_division` compares the true surface against
the **bilinear interpolant** of a cell's four corners. What `insert_surface`
emits is **two flat triangles** across the cell, with the diagonal chosen by
UV-Delaunay. For a warped quad these are different surfaces. At the cell centre
they differ by

```
| P01 + P10 − P00 − P11 | / 4
```

which is the same order as the surface's own second-order variation across the
cell — that is, comparable to `tol` itself. The realized error is therefore
roughly twice the certified figure. The bound is off by a constant factor rather
than broken.

### 4.2 The certified regions are not what gets triangulated

Grid points are inserted only on `PointLocation::Inside`; `Outside`, `Boundary`
and `Indeterminate` all yield `None`. Critically, the `constrain` closure fires
only when **both** endpoints are `Some`:

```rust
if let Some(x) = a[0] {
    if let Some(y) = a[1] { constrain(triangulation, x, y); }
    if let Some(z) = z    { constrain(triangulation, x, *z); }
}
```

So each declined point deletes up to four grid-line constraints. In the band
between the last fully-interior grid line and the trim polyline there are **no
constraints at all**, and Spade fills it by unconstrained Delaunay — producing
triangles that span multiple certified cells and whose interiors were never
tested against anything.

This is the direct answer to the question of whether the CDT can invalidate the
local error assumptions used when the points were generated. It can, it does,
and it does so precisely in the boundary-adjacent band — which on a trimmed
mechanical face is most of the face.

Two aggravators:

- The grid `range` is the **bounding box of the lifted boundary**, not the trim
  region (`:5687`). On an L-shaped or annular face a large fraction of grid
  points fall outside the material region and are declined, so effective
  interior sampling is far sparser than the certified grid.
- `insert_res.windows(2)` means `vec[0]` only ranges over columns `0 .. N-2`, so
  the **last u-column's** v-direction constraints are never added at all.

### 4.3 Correction to an earlier reading

An earlier pass over this code called the `Inside`-gating "bounded, roughly
cell-scale," on the reasoning that a triangle spanning one cell deviates by
about as much as the cell was certified for. That is wrong, and the error was
not tracing that the **constraints** vanish along with the points. Triangles in
the boundary band are not confined to one cell and are not cell-scale bounded.
This moves the finding from a quality wart to an architectural break, and it is
why §7 recommends what it does.

---

## 5. Stages that are sound

**Analytic plane, cylinder and cone routes.** These bypass `insert_surface`
entirely and triangulate the boundary polygon with no interior Steiner points.
`tolerance` in `formal/cylinder_mesh.rs` and `formal/planar_developed.rs` is
used for *certification* — is this point on the plane or cylinder? — never for
subdivision. That is geometrically correct: a plane is exact, and cylinders and
cones are developable and ruled, so a triangle whose vertices lie on the surface
deviates only circumferentially.

**Analytic torus.** Not developable, so it grids explicitly, with `nu`/`nv`
derived from `tol` and both radii (`:2890`) and clamped to `[8, 256] × [4, 128]`.
The clamp is the only break, and only on very large or very finely toleranced
tori.

**Shell compression.** `wrap_shell` (`src/step/policy_geometry.rs:435`) carries
vertices, topology, orientation and provenance through unchanged and wraps only
geometry. A compressed shell samples each edge once by id, so adjacent faces
share one boundary polyline: no cracks, no geometry movement.

**Vertex welding.** `distance_2 < 1e-12` in UV (`:4954`) is absolute in
parameter space, which would be a real hazard under a badly scaled
parameterization. For the parameterizations actually in play — world-scaled
planes, radians on revolved surfaces, unit knot spans on splines — it
corresponds to ≤ 1e-4 mm. Not a contributor.

**UV-space Delaunay** is worth naming even though it is not a bound violation:
the empty-circumcircle criterion is applied in UV, so for an anisotropic
parameterization (a cone near its apex, non-uniform spline knots, a cylinder
whose `u` is generatrix length and `v` is radians) the result contains triangles
that are slivers in 3D. No 3D quality criterion is enforced anywhere in the
pipeline.

---

## 6. Two findings that change how the results read

### 6.1 Cylinder and cone error is entirely boundary-determined

Because those routes add no interior points (§5), the interior error of every
cylinder and cone in the model is fixed by the discretization of its boundary
circles and by nothing else. The "adequate interior subdivision, boundary
dominates" case is not hypothetical here — it is the normal operating mode for
the most common curved surfaces in mechanical CAD.

### 6.2 Every straight edge carries 17 vertices instead of 2

`Line::parameter_division` (`truck-geometry/src/specifieds/line.rs:141`) returns
exactly two points, which is correct — a line is exact. `tessellate_edge`
(`:867`) then hits:

```rust
let mut poly = PolylineCurve::from_curve(curve, range, tol);
if poly.len() <= 2 && range.1 - range.0 > 1e-4 {
    const STEPS: usize = 16;
    // ... 17 uniform samples, no error test
}
```

For any real line the span exceeds `1e-4`, so this **always fires**. A
four-sided planar face receives 68 boundary vertices instead of 8, which the CDT
turns into roughly 66 triangles instead of 2.

There is no accuracy gain. This is a triangle-budget misallocation, and it
explains how the mesh can be simultaneously heavy and visibly coarse: the budget
is spent on straight edges while curved geometry runs on the bare relative
tolerance.

---

## 7. Ranking, and the smallest sufficient change

### Likely dominant on ordinary mechanical STEP geometry

**The topology-safe meshing-policy gate**, and §6.1 is what makes it decisive.

Ordinary prismatic parts *are* mostly plane, cylinder and cone, so the
24-segment angular floor applies and holes come out round. But
`src/step/policy_geometry.rs` requires a certified circle **and** every incident
face in `{plane, cylinder, cone}`. Therefore:

- any circle shared with a sphere, torus, spline or extrusion gets no floor;
- **every non-circular curved edge** — true ellipses, spline trim curves — never
  gets one;
- `PolicySurface`'s interior floor is gated identically, so sphere, torus and
  spline interiors never see it either.

All of those fall back to `0.001 × model bbox diagonal`, which on an assembly is
set by the assembly rather than the feature — precisely the failure
`meshing_policy.rs` opens by describing, still in force for everything outside
that set. Combined with §6.1, a coarse boundary becomes a coarse surface with
nothing downstream able to recover it.

**Full ranking**

1. The policy gate (§7 above) — widest reach on ordinary parts.
2. The boundary-adjacent unconstrained band (§4.2) — every trimmed curved face
   not on an analytic route.
3. The disabled compatibility gate (§3) — extreme magnitude, small population.
4. The single-sample heuristics (§2) — splines specifically, not prismatic parts.

### Smallest architectural change

Not a new refinement layer; three adaptive mechanisms already exist and a fourth
would duplicate them. The minimal change that turns the sampling criterion into
a genuine final-triangle bound is to **triangulate the certified regions rather
than discarding their corners**, contained to `insert_surface`:

> When a grid point is not `Inside`, insert the intersection(s) of its grid
> lines with the trim polyline instead of declining it, and constrain those.

That restores a constrained decomposition right up to the boundary, so every
emitted triangle lies inside exactly one certified cell — which is the missing
implication of §0. One function, no new types, no contact with the lift,
arrangement, or material-selection work.

Two cheap riders belong with it:

- change the accept test to compare against the **two triangles actually
  emitted** rather than the bilinear interpolant, so the certified surface is
  the realized one (§4.1);
- close the `windows(2)` gap so the last u-column is constrained (§4.2).

The policy gate must be widened too, but separately: `policy_geometry.rs`
records bidirectional sphere and B-spline face-recovery flips in the NIST corpus
when shared circular edges were densified, so that change re-opens a known
density sensitivity and has to be moved against the face census rather than
blind.

---

## 8. What this document does not establish

Findings §2, §4 and §6.2 are structural and follow from the source alone.
The *ranking* in §7 is an argument from reach, not a measurement: nothing here
quantifies how much geometric error each mechanism contributes on a real corpus.

`examples/tolerance_probe.rs` exists for exactly that and is built at
`target/x86_64-pc-windows-gnullvm/release/examples/tolerance_probe.exe`. It
sweeps tolerance factors and reports faces, triangles and edge-sample counts per
surface kind, which would size the policy gate directly. It was not run for this
document.

Two claims specifically remain unmeasured: that the policy gate outranks the
boundary band on ordinary parts, and that §6.2's vertex inflation is
quantitatively significant against total triangle count.
