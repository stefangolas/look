# Truck audit against the B-rep generation formal system

> Audits `truck` at the pinned rev `c5f4b6e9778e0721a1d446f10568eb5e5594e8ed` against
> [`FORMAL_SYSTEM_BREP_GENERATION.md`](FORMAL_SYSTEM_BREP_GENERATION.md).
>
> **Read the framing in §0 before the percentages.** Truck was not written against
> this or any formal system, so a naive score is ~0% and tells you nothing. The
> useful question — and the one this document answers — is *which* gaps are
> structural (the data model forbids the fix), which are missing work (the shape
> is right, the certificate is absent), and which are **active correctness
> faults** (truck returns a wrong answer, or panics, on inputs it accepts).
>
> **Decision recorded 2026-08-15: truck is being vendored** as the basis of a
> B-rep generation kernel, on the same pattern as the STEP ingestion path. §6–§8
> are the working documents for that: §6 the design faults that make truck a
> weak foundation *independent of the spec*, §7 the dependency-ordered build
> plan, §8 the component-by-component keep/replace map. §1–§5 are the evidence
> those rest on.
>
> **Implementation orders are in
> [`GENERATION_KERNEL_BUILD_SPEC.md`](GENERATION_KERNEL_BUILD_SPEC.md)** —
> per-item algorithms, `BG-` contracts, invariants and tests for Stages 0–3.
> This document says *what is wrong and in what order to fix it*; that one says
> *how*.
>
> Companion to [`FORMAL_SYSTEM_STEP_INGESTION.md`](FORMAL_SYSTEM_STEP_INGESTION.md),
> which governs the *import* path. This document is about the *generation* path,
> which `look` does not currently use — see §9.

---

## 0. Method and framing

Every claim below is anchored to a file and line in the pinned checkout. Paths
are relative to the truck workspace root.

**These line numbers are evidence, not instructions.** They record where each
finding was observed at rev `c5f4b6e` and will drift as soon as work begins. Do
not drive edits from them — the actionable, drift-resistant form (file + symbol +
`rg` pattern + expected hit count) is in
[`GENERATION_KERNEL_BUILD_SPEC.md`](GENERATION_KERNEL_BUILD_SPEC.md), per its
H-8. That convention has already paid for itself once: verifying the counts
caught that this document's D-2 undercounted truck's deadlock warnings as 10
across 5 files when it is 12 across 6 — `wire.rs` was missed. Corrected below.

Three distinct verdicts are used, and they are not interchangeable:

| verdict | meaning |
|---|---|
| **STRUCTURAL** | The data model or architecture forbids the specified behaviour. Cannot be closed by adding code; requires changing a type that everything else depends on. |
| **ABSENT** | The specified thing simply is not implemented. The existing architecture would accommodate it. |
| **FAULT** | Truck accepts an input and returns a wrong answer, or aborts the process, where the spec requires a typed refusal. These are bugs *by truck's own standards*, not only by the spec's. |

A fourth category, **PARTIAL**, marks things that exist in recognisable but
uncertified form — the algorithm is there, the certificate is not.

Fault IDs use three prefixes, and the distinction is load-bearing for the build
plan: **`S-`** (§2) are places truck cannot express the specification; **`F-`**
(§3) are places truck is actively wrong on inputs it accepts; **`D-`** (§6) are
places truck's design is weak *as a geometry kernel*, whether or not the
specification exists. The `D-` list was found last and matters most for
vendoring, because those are the faults that would still bite a kernel with no
formal system at all.

Two scope notes. First, truck's own documentation is candid about its limits
(`truck-shapeops/src/lib.rs:1-14`: Booleans "supported only for shapes where
faces intersect transversally"; fillet only for "a single edge whose end
vertices are each adjacent to exactly three faces"). Where truck says it does
not do something, this document does not treat that as a hidden defect — but it
does record it, because the spec needs the coverage number either way. Second,
`truck-meshalgo/src/tessellation/formal/*` is **our fork's** work against the
*ingestion* formal system, not upstream truck and not part of the generation
path. It is not credited to truck below; see §9.

---

## 1. Headline verdict

Truck is a **competent, compact, unguarded modelling kernel**. It implements the
combinatorial skeleton of sweeping and a mesh-seeded Boolean, and it does so in
about a tenth of the code a production kernel would use. What it does not have —
anywhere, at any level — is the notion that an operation might have a
*precondition* or produce a *certificate*.

Three sentences capture the whole audit:

1. **There is no evidence layer.** Every operation in `truck-shapeops` returns
   `Option<T>`. `None` is the sole failure value, and it conflates degenerate
   input, tangency, budget exhaustion, approximation failure, and internal
   inconsistency. The spec's nine terminal outcomes (§4) collapse to one bit,
   and that bit carries no diagnosis.

2. **The exact geometry is not what decides the topology.** Boolean face
   classification and intersection-curve connectivity are both decided on a
   *triangle mesh* at tolerance `tol`, then the geometry is relaxed onto the
   true surfaces afterwards. The spec's entire §6/§9/§11 apparatus exists to
   guarantee that the combinatorics computed on an approximant equal the exact
   combinatorics; truck computes the combinatorics on the approximant and never
   asks the question.

3. **Failure is frequently a panic, not a refusal.** Every one of the six §17
   fillet gates surfaces as an `.unwrap()` on a Newton solve. Violating them
   aborts the process.

The third is the most consequential for `look` specifically: a library that
panics on bad geometry cannot be a renderer's ingestion path without a
catch-unwind wrapper, and cannot be a kernel at all.

---

## 2. The four structural faults

These are the ones that cannot be fixed by adding cells. Each blocks a large
fraction of the spec downstream.

### S-1 — No coedge. The pcurve has nowhere to live. (§1, invariant 4)

`truck-topology/src/lib.rs:126-141`:

```rust
pub struct Edge<P, C> { vertices: (Vertex<P>, Vertex<P>), orientation: bool, curve: Arc<Mutex<C>> }
pub struct Wire<P, C> { edge_list: VecDeque<Edge<P, C>> }
pub struct Face<P, C, S> { boundaries: Vec<Wire<P, C>>, orientation: bool, surface: Arc<Mutex<S>> }
```

A wire is a sequence of *oriented edge handles*. Two handles to the same edge
share one `Arc<Mutex<C>>` and one `EdgeID`. That is a coedge in the weak sense
of carrying a sense bit — but it has **no per-use payload**, so:

- **There is no pcurve anywhere in the model.** A face is a surface plus 3D
  boundary curves; the parametric position of a boundary is recovered on demand
  by numerical surface inversion (`search_parameter`, used at
  `truck-meshalgo/src/tessellation/mod.rs:268,275,335`). Invariant 4
  (same-parameter/same-range) is therefore not merely unchecked — it is **not
  expressible**. There is no second representation to agree with the first.
- **A seam edge cannot be represented faithfully.** The spec's motivating case —
  one 3D edge appearing twice on one periodic face with two *different* pcurves
  — requires two uses with distinct parametric data. Truck's two uses share one
  curve. The workaround is to make two distinct `Edge` instances, which then
  breaks the coedge-pairing invariant in the other direction.
- Non-manifold declared incidence ($2k$ coedges) has no representation.

Everything in §6.3, §8, §9.3 (seam incidence), and §19's `REP-PCV-001` is
downstream of this. This is the single highest-leverage structural gap.

### S-2 — No tolerance on any entity; one global absolute constant (§0.1, §2, §5)

`truck-base/src/tolerance.rs:6`:

```rust
pub const TOLERANCE: f64 = 1.0e-6;
```

`Vertex` carries a point and nothing else; `Edge` has no $\tau_e$; `Face` has no
$\tau_f$. So:

- The three-budget semantics of §0.1 ($\tau_{\text{in}}, \tau_{\text{rep}},
  \tau_{\text{col}}$) has nowhere to attach. Where truck takes a `tol`
  parameter it is a single scalar meaning "mesh fineness", used
  simultaneously as approximation target and as membership-decision scale.
- Invariant 7 (tolerance monotonicity) and §5's admissibility bound
  $R_C \le \theta\cdot\mathrm{lfs}_\sigma$ are unstatable.
- The constant is **absolute**, in model units. §2 names this exactly: "Absolute
  constants in predicates are a defect." A part modelled in metres and the same
  part in millimetres take different code paths. This is not theoretical — it
  reaches user-visible behaviour, because `nonpositive_tolerance!`
  (`truck-geotrait/src/lib.rs:19-30`) is an `assert!`, so calling
  `shapeops::and(a, b, 1e-8)` on a small part **panics**.

### S-3 — Topology is decided on a triangle mesh (§6.3, §9, §11, §12)

`truck-shapeops/src/transversal/integrate/mod.rs:85-86`:

```rust
let poly_shell0 = shell0.triangulation(tol);
let poly_shell1 = shell1.triangulation(tol);
```

Those meshes then drive:

- **intersection curve extraction** — mesh–mesh intersection segments, assembled
  into polylines, and only then relaxed onto the exact surfaces by Newton
  (`IntersectionCurveWithParameters::try_new`,
  `truck-shapeops/src/transversal/intersection_curve/mod.rs:28-52`, calling
  `search_triple` → `double_projection`). The *number of components*, their
  *connectivity*, and *which branch connects to which* are all fixed by the mesh
  before any exact geometry is consulted;
- **face membership classification** — ray casting against the mesh
  (`integrate/mod.rs:100-127`).

The spec's §6.3 exists precisely to license this move, and it requires two
conditions: Hausdorff closeness $\varepsilon < \sigma_{\text{cl}}/3$ **and** the
tangent-angle condition of §6.2(ii). Neither is computed. There is no
$\sigma_{\text{cl}}$, no transversality margin $\delta$, no reach $\rho$. Two
intersection branches closer than `tol` merge silently; a branch thinner than
`tol` vanishes silently.

This is also the mechanical reason tangency is unsupported: near a tangential
contact the mesh intersection is unstable in *combinatorics*, not just in
position, so no amount of Newton polish afterwards recovers it.

### S-4 — No arrangement engine, no membership propagation (§11, §12)

§12's theorem — one certified seed per face, propagate over the dual adjacency
graph, verify every non-tree edge — is the spec's mechanism for getting
membership right with a single risky decision per face. Truck does the opposite:
it takes an **independent, uncertified ray cast per unclassified face**
(`integrate/mod.rs:100-127`). There is no propagation, no spanning tree, no
cycle-consistency check, and therefore no contradiction witness when two faces
disagree — the disagreement simply becomes a malformed shell later.

There is likewise no DCEL and no atomisation at certified contact clusters.
Curve-graph assembly is a greedy walk over a hash grid (see F-2).

---

## 3. Serious correctness faults

Ranked by how likely they are to produce a *silently wrong* result, which is
worse than a crash.

### F-1 — Disjoint Boolean results become one solid with fake voids

`truck-shapeops/src/transversal/integrate/mod.rs:156-158` and `179-181`:

```rust
let boundaries = or_shell.connected_components();
Some(Solid::new(boundaries))
```

`Solid`'s field is `boundaries: Vec<Shell>` where the documented semantics
(§1, and truck's own `Solid::try_new`) is *one outer shell plus inner shells*.
Feeding it the connected components of a result means: **the union of two
disjoint solids is returned as a single solid whose second component is
interpreted as a cavity.** No containment test is performed anywhere —
`Solid::try_new` (`truck-topology/src/solid.rs:18-31`) checks non-empty,
connected, closed, and non-singular per shell, and never checks nesting.
Invariant 8 is absent.

This is a wrong answer on a completely ordinary input (union of two separated
lumps), and it propagates: downstream volume, in/out, and tessellation all read
the second shell as a void.

### F-2 — Intersection-curve connectivity is decided by a hash-grid snap

`truck-shapeops/src/transversal/polyline_construction/mod.rs:30-34`:

```rust
impl From<Point3> for PointIndex {
    fn from(pt: Point3) -> PointIndex {
        let idx = pt.add_element_wise(TOLERANCE) / (2.0 * TOLERANCE);
        PointIndex(idx.cast::<i64>().unwrap().into())
    }
}
```

Intersection polyline endpoints are identified by quantising to a grid of pitch
$2\times10^{-6}$ **absolute**. This fails in both directions, which is the
signature of a snapping scheme rather than a clustering scheme:

- two points $10^{-9}$ apart that straddle a cell boundary are treated as
  **distinct** — the curve is split where it should be continuous;
- two points $3\times10^{-6}$ apart inside one cell are treated as **identical** —
  distinct branches are welded.

§5 forbids exactly this ("$p\sim_\tau q$ is not transitive and is not used") and
prescribes certified ball clustering with an admissibility bound. Compounding
it, the graph walk picks an arbitrary neighbour at a branch node
(`pop_one_adjacency`, `polyline_construction/mod.rs:48-52`, taking
`self.adjacency.iter().next().unwrap()`), so at any point where three or more
segments meet — a self-intersection or a tangential contact of the intersection
curve — the traversal chooses arbitrarily. That is §11's atomisation step
replaced by a coin flip.

### F-3 — Ray-cast membership with no admissible-direction certificate

`integrate/mod.rs:103` and `118`:

```rust
let dir = hash::take_one_unit(pt);
```

The ray direction is derived by hashing the seed point. It is deterministic and
therefore reproducible, but it is never **certified admissible**: there is no
check that the ray misses vertices, edges, or tangential loci, and there is no
resampling path when it does not. §12's `CLS-SEED-001` requires interval
separation certification and a `NoAdmissibleRay` fallback to certified winding
number; neither exists.

The counting predicate is `count >= 1` (`integrate/mod.rs:108,123`, and
`IncludingPointInDomain::inside`, `truck-meshalgo/src/analyzers/in_out_judge.rs:115`)
over a *signed* crossing sum. For a correctly oriented, non-nested mesh this
agrees with `!= 0`, but it silently returns "outside" everywhere for an inverted
shell, and it is not the winding-number test the nesting case needs.

Underneath, `signed_crossing_faces` (`in_out_judge.rs:94-112`) accumulates
`tri.is_crossing(ray)` with no handling of the ray passing through a shared
triangle edge or vertex — the classic double-count/miss, unmitigated by any
symbolic perturbation.

### F-4 — Every fillet gate is a panic

The spec's §17 lists six gates. In `truck-shapeops/src/fillet/mod.rs:181-240`,
each is reached as an `.unwrap()` on an `Option`-returning Newton solve:

```rust
let (_, _, v00, _) = strict_surface
    .search_contact_curve0_cross_point_with_adjacent_edge(t0, &curve00, s00_hint, 100)
    .unwrap();                      // :214, and again :220, :228, :234
```

`NoSpine` (gate 2), `FilletSpillover` (gate 5), and a radius exceeding curvature
(gate 1) all manifest as that `unwrap` failing — i.e. as a process abort. Two
more at `:153` and `:160` (`Matrix3::from_cols(...).invert().unwrap()`) abort on
chart degeneracy. The refinement loop in
`truck-geometry/src/decorators/af_surface.rs:463` re-enters the same solve as
`fillet_surface.contact_circle(v).unwrap()` — inconsistently, since the
identical call three lines earlier at `:392-394` is `?`-handled.

Note the asymmetry this creates: a fillet radius slightly too large for the
supporting face's curvature is a completely routine user error, and it takes
down the process.

### F-5 — The fillet's own boundary curves are guessed

`fillet/mod.rs:133-166`, `create_pcurve_edge`. The two end edges of the fillet
face are built as a **cubic Bézier in $(u,v)$** whose interior control points are
placed at $\mathrm{dist}/3$ along the endpoint tangents:

```rust
let cp1 = uv0 + dist / 3.0 * uvder0.truncate().normalize();
let cp2 = uv1 - dist / 3.0 * uvder1.truncate().normalize();
```

This is a shape heuristic with no fidelity bound — nothing relates it to the
true boundary of the rolling-ball envelope. It is a direct OB-4 violation: the
cell emits topology (an edge, with pairing consequences for the shell) whose
geometry it has not certified. The `debug_assert!` that would have checked the
tangency assumption is commented out at `:154` and `:160`.

### F-6 — The fillet surface's error is sampled at three points, and only in position

`af_surface.rs:459-470`:

```rust
let is_far = |t: f64| approx.subs(t, v).distance2(cc.subs(t)) < tol * tol;
match [0.0, 0.5, 1.0].into_iter().all(is_far) { true => None, false => Some((v, cc)) }
```

The approximation is accepted when three sample points per candidate span fall
within `tol`. This is point sampling, not a certified bound (§6.2(i) needs a
two-sided Hausdorff bound over the span), and the tangent-angle condition
§6.2(ii) is never evaluated at all — which is the condition that actually
carries the isotopy claim. The loop budget is a hard-coded `for _i in 0..16`
(`:396`) falling through to `None`, so budget exhaustion is indistinguishable
from geometric failure.

(The closure name is inverted — `is_far` returns true when the point is *near*.
The logic that consumes it is correct; the name invites a wrong edit.)

### F-7 — Release builds skip the validity checks that debug builds run

`truck-topology/src/face.rs:73-78`:

```rust
pub fn debug_new(boundaries: Vec<Wire<P, C>>, surface: S) -> Face<P, C, S> {
    match cfg!(debug_assertions) {
        true => Face::new(boundaries, surface),
        false => Face::new_unchecked(boundaries, surface),
    }
}
```

`builder::cone` and the sweep paths construct faces and edges via `debug_new`
(`truck-modeling/src/builder.rs:737,738,754,757,773,777`). So the wire-closure
and simplicity checks that run in tests do **not** run in the shipped
configuration. Correctness properties that hold under `cargo test` are not the
properties of the shipped artifact — which also means the test suite cannot
witness a regression in them.

### F-8 — Boolean multi-shell iteration is not the Boolean

`integrate/mod.rs:140-155`. For solids with more than one boundary shell, `and`
folds: `and_shell = process(and_shell, next_shell)` over the remaining shells of
*both* operands. Intersecting the running result against a **cavity** shell is
not the same operation as subtracting that cavity. For solids with voids the
result is not the regularized intersection. Combined with F-1 — which
manufactures spurious multi-shell solids out of disjoint unions — this is
reachable from ordinary inputs.

Also `iter0.next().unwrap()` at `:143` panics on a solid with no boundaries.

### F-9 — Composition aborts: seven `unimplemented!()` on `IntersectionCurve`

`truck-modeling/src/geometry.rs` panics on `Curve::IntersectionCurve` in seven
places — `:189, :195, :201, :217, :230, :233` (all inside `IncludeCurve<Curve>
for Surface`) and `:315` (inside `ToSameGeometry<Surface> for
ExtrudedCurve<Curve, Vector3>`).

`IntersectionCurve` is precisely the curve variant that **Boolean operations
produce**. So the results of a Boolean cannot be fed back into the modelling
operations:

- `Surface::include(curve)` — the "does this surface contain this curve"
  predicate — aborts on any Boolean-derived edge. This is not an obscure entry
  point: `builder::try_attach_plane` is bounded on `Plane: IncludeCurve<C>`
  (`builder.rs:379-382`), so **capping a wire whose edges came from a Boolean
  panics**;
- extruding such an edge aborts (`:315`).

§18's entire subject is composition, and truck's answer to the most common
composition — Boolean, then cap or extrude the result — is a process abort. Note
that the failure is by *curve variant*, not by geometry: it fires regardless of
how well-conditioned the actual input is.

---

## 4. Coverage against the specification

Percentages are *implementation* coverage, i.e. how much of each section's cell
inventory exists in certified form. The `verdict` column is the more useful
number.

| § | Layer | verdict | cov. | Evidence / note |
|---|---|---|---|---|
| 0.1 | Backward-error semantics, 3 budgets | STRUCTURAL | 0% | One scalar `tol`; no certificate returned by any operation. |
| 0.2 | Epistemic vs constructive closure | ABSENT | 0% | No envelope declared, no membership classifier. |
| 1 | Data model: coedge, pcurve, degenerate edge, τ | STRUCTURAL | 25% | S-1, S-2. Vertices/edges/faces/shells/solids present and sound; coedge, pcurve, tolerance, $\Lambda$, provenance all absent. |
| 1.1 | Invariants 1–9 | PARTIAL | 30% | **1** (pairing) ✅ via `ShellCondition::Closed`. **2** (link) ✅ and correct — `singular_vertices` (`shell.rs:536`) tests link connectivity, which *given* Closed implies a single cycle, since every link node then has degree 2. **3** (Euler) ✅ available. **4** unstatable (S-1). **5, 6, 7, 9** absent. **8** (nesting) absent → F-1. |
| 2 | Carriers, `rep`, scale invariance | PARTIAL | 35% | $\mathcal{G}$ = {Plane, BSpline, NURBS, RevolutedCurve} for surfaces; {Line, BSpline, NURBS, IntersectionCurve} for curves (`truck-modeling/src/geometry.rs:28,132`). Geometrically adequate — a rational NURBS represents circles/cylinders exactly — but **analytic identity is discarded**: extruding a circle yields a NURBS, not a cylinder (`geometry.rs:296-320`). That loses the exact carriers §16.1 depends on. No `rep` operator, no scale invariance (S-2). |
| 3 | OB-1…OB-7 | ABSENT | 0% | No dispatch structure to state them over. |
| 4 | Evidence algebra, 9 terminal outcomes | ABSENT | 0% | `Option<T>`. This is the single most pervasive gap. |
| 5 | Certified clustering, collapse calculus | FAULT | 5% | Replaced by hash-grid snapping (F-2). No `lfs`, no admissibility bound, none of the seven collapse gates. |
| 6 | Stratified reach, isotopy, label preservation | ABSENT | 0% | No reach, no `lfs_σ`, no tangent-angle condition, no stratification anywhere in the tree. This is the spec's **root** node. |
| 7 | Numerical substrate | PARTIAL | 20% | Newton (`truck-base/src/newton.rs`) and `double_projection` exist and are reasonable. **No interval arithmetic, no Bernstein/Descartes subdivision, no Krawczyk, no exact/rational tier.** Budgets exist as bare trial counts (`100`, `0..16`) whose exhaustion is untyped. |
| 8 | Periodicity contract | ABSENT | 10% | `u_period`/`v_period` exist on surfaces. No $\Lambda$, no fundamental domain, no seam coedges (S-1), no `Range`/`Var` lift bounds. |
| 9 | Intersection atlas | PARTIAL | 15% | SS transverse only, mesh-seeded (S-3). **No** rank/transversality predicate $\sin\theta\ge\delta$, **no** contact classification, **no** tangential cells, **no** coincidence detection, **no** Milnor number, **no** flip parity. Truck documents the tangency limit itself. CC/CS atlases absent. |
| 10 | Self-intersection engine | ABSENT | 0% | No diagonal deflation, no blow-up, no `SI-*` cell. Nothing checks self-intersection of any generated surface. |
| 11 | Arrangement engine | FAULT | 5% | S-4, F-2. `loops_store` + `divide_face` do a face-splitting job, but with no DCEL, no certified clusters, and no contradiction localisation. |
| 12 | Membership by propagation | FAULT | 10% | S-4, F-3. Per-face uncertified ray cast; no propagation, no seed certificate, no flip-parity rule. |
| 13 | Boolean reconstruction | PARTIAL | 30% | `and`/`or` only — **no difference operator at all**. No regularization semantics stated. No coincident-face handling, so §13.1's material-state formulation has no counterpart; coincident faces fall in the untested tangency hole. Shell assembly does run link + Euler via `Solid::try_new`, which is genuinely good. Nesting absent (F-1). |
| 14 | Envelopes and discriminants | ABSENT | 5% | `af_surface.rs` computes *one* envelope (the rolling-ball fillet) numerically. No discriminant stratification, no $A_2$ regression edge, no $A_3$ swallowtail detection. |
| 15.1 | Extrude | PARTIAL | 20% | `builder::tsweep` (`builder.rs:497`) is purely combinatorial: **zero preconditions**. No profile-simplicity check, no $\lvert\langle\hat d,n_P\rangle\rvert\ge\delta$ gate. Extruding a profile along a direction in its own plane yields a zero-volume "solid" with no error. No draft, no extrude-to-target, no vertex suppression. |
| 15.2 | Revolve | PARTIAL | 30% | `rsweep` + `cone` (`builder.rs:611,711`). Full-turn seam closure is handled, and the pole case is handled by *omitting* the degenerate edge (`builder.rs:731-780`) — a legitimate alternative to §1's first-class degenerate edge, but it means no $\beta$-angle sub-classification, no $G^1$ certificate at a smooth pole, no cusp refusal. `pt1_on_axis` is an absolute `so_small()` test. **`REV-AXIS-TOUCH-001` and `REV-AXIS-CROSS-001` are undetected**: a profile that touches or crosses the axis produces a silently pinched or double-covered solid. |
| 15.3 | Sweep | ABSENT | 10% | `tsweep`/`rsweep` are rigid-motion sweeps only. **No general spine sweep, no frame machinery at all** — no Frenet, no RMF, no closed-spine holonomy defect. `SWP-REG-001`/`SWP-EMB-001` have nothing to gate. |
| 15.4 | Loft | ABSENT | 15% | `homotopy`/`try_wire_homotopy` (`builder.rs:202,301`) give the two-section ruled case (LFT-RULED-001) with no injectivity gate. No correspondence, no $n$-section loft, no pole, no tangent lofting. |
| 16 | Offset and shell | ABSENT | 5% | `Offset`/`NormalField` (`truck-geometry/src/decorators/offset/`) are **lazy pointwise evaluators**, not an operation: a surface plus a vector field, summed at `subs`. There is no offset operation on a shell or solid, no shell/hollow operation, no edge or vertex treatment, no topology events, no $1-d\kappa$ check, no reach check. `grep` for `fn hollow|fn thicken|fn shell_solid` returns nothing. |
| 17 | Fillet | PARTIAL | 25% | The most mathematically serious code in the tree: `RbfSurface` genuinely computes rolling-ball contact circles and `ApproxFilletSurface` fits a rational patch to them. But **all six gates are panics** (F-4), the end edges are guessed (F-5), the error bound is 3-point sampling (F-6), and scope is one edge with 3-face endpoints. No `FIL-CNR-001` (vertex blend), no `FIL-FIL-001`, no variable-radius closure. **Not reachable from `truck_modeling::{Curve, Surface}`** — the required `ToSameGeometry` impls for `PCurve` and `ApproxFilletSurface` exist only inside test files (`fillet/tests.rs:221,248`; `tests/fillet.rs:36,58`). |
| 18 | Composition, conditioning, margins | ABSENT | 0% | No modulus of continuity anywhere, no topological stability margin, no composition bookkeeping. F-9 is what composition currently does. |
| 19 | Input validation and repair | PARTIAL | 25% | `truck-shapeops/src/healing/` provides the `SplitClosedEdgesAndFaces` and `RobustSplitClosedEdgesAndFaces` traits (`healing/mod.rs:35,83`) — real, useful repairs, roughly `REP-SEAM-001`'s neighbourhood. No validation atlas, no backward-error accounting, no `REP-PCV-001` (impossible — S-1), no `REP-TOL-001`, no `REP-ORI-001`. |
| 20 | Identity and regeneration | ABSENT | 5% | `VertexID`/`EdgeID`/`FaceID` are pointer identities (`ID<Mutex<T>>`) — stable within a session, meaningless across a regeneration. No construction-derived selector, no `Preserved/Split/Merged/Vanished/Ambiguous` map, no policy recording. |
| 21 | Verification | PARTIAL | 20% | Good property-based testing with `proptest` in places (`geom_impls.rs`, `af_surface.rs`). No OCCT differential harness, no metamorphic invariants ($A\cup^*A=A$ etc.), no margin sweeps. |

**Aggregate: ≈14% of the specification's implementation surface**, and that
number is charitable — it credits recognisable-but-uncertified machinery at
partial weight. Against the spec's own bar (a cell may only claim
`ProvenConstruction` on **[DERIVED]** gates with a published modulus), truck
implements **zero** constructive cells, because no cell in truck states a
precondition or emits a certificate.

---

## 5. Where the biggest gaps are

Ordered by leverage, which is not the same as by size.

> **Sequencing note.** This ranking answers "which gaps hurt most", and it is
> still accurate as a ranking. It is **not** the build order — see §7. The
> difference matters: several items high on this list (F-1, F-2, and the
> propagation work) live inside the mesh-seeded Boolean that §7 Stage 4
> replaces, so ranking by pain and building by dependency give different
> sequences. Read this section for *what is wrong*, §7 for *what to do first*.

1. **The evidence layer (§4) — biggest ratio of value to effort.** Replacing
   `Option<T>` with a typed outcome enum, and turning the ~15 `.unwrap()` calls
   on the fillet/Boolean solve paths (F-4) plus the seven `unimplemented!()`
   composition aborts (F-9) into typed refusals, would move truck from
   "unguarded" to "epistemically honest" without touching a single geometric
   algorithm. It is also the prerequisite for every other item: you cannot
   report a gate failure until there is a channel to report it on. F-9 in
   particular is close to free — the panics are exhaustive-match arms, not
   missing mathematics.

2. **The coedge (S-1) — biggest structural blocker.** Adding a per-use payload
   to `Wire` unlocks pcurves, invariant 4, seams, §8, and `REP-PCV-001`. It is
   a breaking change to the central type of `truck-topology` and everything
   downstream, which is why it should be decided early rather than late.

3. **Certified clustering to replace hash-grid snapping (§5, F-2).** Localised
   to one file, fixes a fault that silently corrupts Boolean results, and is a
   direct precondition for §11.

4. **Nesting determination (F-1).** Small, self-contained, fixes a wrong answer
   on an ordinary input.

5. **Preconditions on the sweep operations (§15.1–15.2).** `tsweep` and
   `rsweep` currently accept anything. Adding the direction gate, profile
   simplicity, and axis-crossing detection is cheap and turns the two most-used
   operations from "usually fine" into "gated".

6. **The §6 stratified fidelity engine.** The spec's root, and genuinely
   absent — there is no reach, no `lfs`, no isotopy condition anywhere in
   truck. This is the largest single body of new work, and every certificate in
   the system is downstream of it. It cannot be retrofitted cheaply, and it is
   the reason the honest overall number is 14% rather than 40%.

7. **§9.2 tangential and coincident cells.** Truck's documented headline
   limitation, and — per the spec's own scoping note — the one that excludes
   most real mechanical parts, since every fillet is tangent to its supports.
   Depends on 6.

8. **§10 self-intersection.** Nothing in truck checks it. Any offset, variable
   sweep, or draft work is untrustworthy until it exists.

---

## 6. Design faults: why truck is a poor *foundation*, independent of the spec

§2's `S-` faults are places truck cannot express the specification. This section
is different: these are places where truck's design is weak **as a geometry
kernel**, on its own terms, whether or not the spec exists. They were found
after the first pass, and they matter more for the vendoring decision than the
`S-` list does, because they are the ones that would still bite a kernel with no
formal system at all.

### D-1 — The evaluation interface is pointwise `f64`, so nothing can be enclosed

`truck-geotrait/src/traits/curve.rs:10-14`, and identically for surfaces
(`traits/surface.rs:5-25`):

```rust
pub trait ParametricCurve: Clone {
    type Point;  type Vector;
    fn subs(&self, t: f64) -> Self::Point;   // parameter is f64
}
```

`Point` and `Vector` are associated types, so the *codomain* is negotiable. The
**parameter is not**. No truck curve or surface can be evaluated over an
interval or a parameter box.

This is the single most consequential design fault, because **every certified
quantity in the specification is an enclosure over a box**: §7's
Bernstein/Descartes subdivision and Krawczyk operator, §6's two-sided Hausdorff
bound, §6.2(ii)'s tangent-angle bound, §9's transversality margin $\delta$, §5's
certified cluster radii, §10's deflated system $H$ over $D\times S^1\times[0,h]$.
Not one of them is computable through this interface.

It is not a missing module — it is the wrong interface at the bottom of the
stack, and everything above inherits the limitation.

**Tractable, though.** The fix is a second, parallel interface
(`EnclosureCurve`/`EnclosureSurface`) rather than a rewrite of the first:

- for B-spline and NURBS carriers the convex-hull property over a subdivided
  span is both easier *and tighter* than generic interval arithmetic, and truck
  already has the pieces — control points, subdivision, and a
  `roughly_bounding_box` on `BSplineCurve` (`nurbs/bspcurve.rs:1219`) that is
  exactly a control-point hull enclosure;
- for `specifieds/` carriers, enclosure is closed-form;
- for decorators (`Processor`, `RevolutedCurve`, `PCurve`, `Offset`,
  `IntersectionCurve`) it is compositional, and it is the real work.

Estimate 3,000–6,000 LOC. The existing `f64` interface survives as the fast
path, which is what production kernels do anyway. But it means `truck-geotrait`
is *one* of two evaluation interfaces, not the foundation.

### D-2 — Geometry is shared mutable state; identity is allocation, not construction

Every topological entity holds its geometry in `Arc<Mutex<_>>`
(`truck-topology/src/lib.rs:126-141`). Two consequences, both bad for a kernel:

**Deadlock hazard is systemic, not incidental.** Twelve separate warnings — two
in each of `vertex.rs`, `edge.rs`, `wire.rs`, `face.rs`, `shell.rs`, `solid.rs`,
i.e. *every* mapping API in the crate (`rg 'will result in a deadlock'
truck-topology/src`) — all reading "Accessing geometry elements directly in the
closure will result in a deadlock."
For a library that wants `rayon` parallelism (as `look` uses elsewhere), a
mutex-per-entity model with documented reentrancy traps is a poor substrate.

**Identity is wrong for regeneration.** `VertexID<P> = ID<Mutex<P>>` is the
*pointer identity of a mutable cell*, and truck documents that mutating the
geometry preserves the ID (`lib.rs:206-217`: "The id does not changed even if
the value of point changes", demonstrated with `v.set_point(1)`). §20 requires
the opposite: identity derived from the *construction*
($\mathrm{op}_j(\mathrm{id}_1,\dots)$), over immutable values. Allocation
identity over a mutable cell cannot survive a regeneration, cannot be
serialised, and cannot express `Split`/`Merged`.

Since the same three structs are already being reworked for the coedge (S-1),
doing identity and immutability in that pass is close to free. Doing it later
means touching everything twice.

### D-3 — There is no Cylinder and no Cone carrier anywhere in truck

`truck-geometry/src/specifieds/` contains exactly: `Line`, `UnitCircle`,
`UnitHyperbola`, `UnitParabola`, `Plane`, `Sphere`, `Torus`. **Cylinder and cone
are absent** — the two most common mechanical surfaces after the plane. They
exist only as `RevolutedCurve`/`ExtrudedCurve` decorators, or, once through
`truck-modeling`'s conversions, as NURBS
(`truck-modeling/src/geometry.rs:296-320`).

This is geometrically exact — a rational NURBS represents a cylinder exactly —
but **analytically anonymous**, and the spec needs the analytic identity, not
just the point set:

- §16.1 `OFF-CARRIER-001` requires cylinder→cylinder ($r\pm d$) and cone→cone
  (shifted apex, with apex vanishing as a *topology event*) in closed form. A
  NURBS cylinder falls through to `OFF-CARRIER-NURBS-001`, whose error degrades
  as $1/|1-d\kappa|$ and whose refusal is `OffsetApproximationBudgetExhausted` —
  for a case that should be exact and never fail;
- §17's fillet gate 1 needs $\kappa^+_{\max}$, trivial on an analytic cylinder
  and a numerical estimate on a NURBS;
- STEP round-trip loses the `CYLINDRICAL_SURFACE`/`CONICAL_SURFACE` entity.

`look`'s ingestion side already knows this cost — `cylinder_band.rs`,
`cone_band.rs`, `cylinder_lift.rs`, `torus_deck.rs` exist precisely because
these surfaces need first-class analytic treatment. Adding `Cylinder` and `Cone`
to `specifieds/` is cheap **now** and expensive once code depends on the current
carrier set.

### D-4 — Genericity without a canonical geometry model

`truck-topology` is generic over `<P, C, S>`, and truck never commits to a
canonical carrier set. That is flexible for a *framework* and wrong for a
*library*, and the audit already caught the consequence twice:

- `simple_fillet` is uninstantiable from `truck_modeling::{Curve, Surface}` —
  the required `ToSameGeometry` impls exist only inside test files
  (`fillet/tests.rs:221,248`);
- `truck-modeling`'s own enum (`Plane | BSpline | NURBS | RevolutedCurve`) is a
  *different, smaller* carrier set than `truck-geometry` actually provides,
  silently discarding `Sphere` and `Torus` (D-3).

§2 specifies one carrier set $\mathcal{G}$. A vendored kernel should commit to
it once, in one place, and let the genericity serve testing rather than the
public API.

---

## 7. Build plan for a vendored generation kernel

**This supersedes an earlier remediation plan in this document that sequenced
the work by fault severity.** That ordering answers "make the current truck
honest". It is the wrong ordering for "build a B-rep generation library",
because Phases A and B of it substantially optimise the mesh-seeded Boolean —
the exact component S-3 says must be replaced. The stages below are ordered by
*dependency*, which is what a vendored rewrite should follow.

Anchor for every estimate: our own ingestion atlas is ~44,000 lines
(`tessellation/formal/` + `domain/`) for five analytic surface types, of which
the evidence + outcome layer alone is 2,782.

### Stage 0 — Free wins to land immediately (~200–300 LOC)

Independent of everything, worth doing on day one so the vendored tree is usable
while the real work proceeds:

- **F-9**, the seven composition aborts. Six are `IncludeCurve` match arms
  answerable *structurally and exactly* — an `IntersectionCurve` carries its own
  two surfaces, so `surface.include(curve)` is decidable when `self` is one of
  them, which is the case that arises.
- **F-4a**, converting the six fillet `.unwrap()`s to `?`.

Nothing else from the old Phase A/B carries: F-1, F-2 and the S-4 propagation
work all live inside code Stage 4 replaces.

### Stage 1 — Data model (6,000–10,000 LOC)

Everything's shape depends on this, so it goes first, while nothing depends on
the current shape.

- **S-1, the coedge.** Cheaper than it looks: truck's `Edge` is *already* a
  coedge — `curve` is shared through the `Arc`, `orientation` is per-handle. Two
  handles to one edge already are two coedges over one curve. The entity/use
  split exists; it has exactly one per-use field. Add a second
  (`pcurve: Option<PC>`), defaulted so `None` reproduces today's behaviour.
  Seams become representable immediately; invariant 4 becomes statable.
- **S-2**, per-entity tolerance on the same three structs, plus scale-relative
  predicates — 184 sites, 128 in `truck-geometry`, and the tedious part, because
  `.near()` on a knot parameter and `.near()` on a model distance want different
  treatments.
- **D-2**, immutable geometry and construction-derived identity, in the same
  pass as S-1 since it touches the same structs.
- **D-3/D-4**, commit to the §2 carrier set: add `Cylinder` and `Cone` to
  `specifieds/`, and define *one* canonical `Curve`/`Surface` model.

### Stage 2 — Certified evaluation interface (3,000–6,000 LOC)

**D-1.** The parallel enclosure traits, and enclosure impls for every carrier in
the Stage 1 set. This is the bottom of the certified stack and it appears in no
earlier version of this plan; without it Stages 3–5 are unimplementable, not
merely uncertified.

### Stage 3 — Fidelity and solvers (8,000–14,000 LOC)

§6 stratified reach, `lfs_σ`, the isotopy conditions and their gluing (OB-7);
§7's Bernstein/Descartes subdivision, Krawczyk, and the budget ledger. §6 is the
spec's declared root and the largest single body of genuinely new mathematics.

### Stage 4 — Intersection, arrangement, Boolean (13,000–22,000 LOC)

§9 atlas — and **budget for the tangential cells here, not later**. Transverse-only
is not a shippable envelope for mechanical parts, since every fillet is tangent
to its supports and every counterbore is coaxial. Then §10 self-intersection
(diagonal deflation), §11 DCEL arrangement on certified clusters, §12 seeded
propagation with non-tree-edge verification, §13 Boolean with the material-state
formulation of §13.1 — which is also where the missing difference operator
arrives.

One salvage note: `FacesClassification::integrate_by_component`
(`faces_classification/mod.rs:41-64`) is §12's propagation idea in embryo, and
its *logic* transfers even though its call site does not.

### Stage 5 — Generative operations (6,000–10,000 LOC)

§15 with real gates. `truck-modeling`'s `Sweep`/`Connector` abstraction is the
right shape and survives; it needs preconditions, not restructuring. Extrude and
revolve admit outright global-embedding proofs (§15.1, §15.2) and should land
first; sweep and loft need §10 underneath them.

### Total

**36,000–62,000 LOC**, consistent with the 44k ingestion anchor for a problem
that has no composition, no self-intersection and no envelope formation. Note
that Stages 1 and 2 — ~10,000–16,000, and the part that unblocks everything —
produce **no user-visible capability at all**. That is the schedule risk worth
naming up front.

### Corpus invariance

How much of this is fixed regardless of what real parts turn out to look like.
The formal system's own gate labels are most of the answer: **[DERIVED]** gates
are theorems a corpus cannot falsify; **[POLICY]** items are contingent *by
definition*, and all of them sit in Stage 5; **[PROVISIONAL SUFFICIENT GATE]**
is a third thing — research debt, not corpus debt.

| Stage | LOC (mid) | Invariant | Contingent part |
|---|---|---|---|
| 0 Free wins | 0.25k | **100%** | — a panic is wrong on any input |
| 1 Data model | 8k | **~85%** | tail of $\mathcal{G}$: hyperbola/parabola, NURBS degree and span caps |
| 2 Enclosure | 4.5k | **~95%** | only *which* carriers, which follows from Stage 1 |
| 3 Fidelity + solvers | 11k | **~100%** | none — §6 and §7 are pure mathematics |
| 4 Intersection/Boolean | 17.5k | **~75%** | tangential cell density: $A_2$ only, or $A_3$? |
| 5 Generative ops | 8k | **~40%** | *which* operations, and every [POLICY] item |

**≈75–80% is corpus-invariant**, and it is **front-loaded**: Stages 0–3 are ~24k
and ~94% invariant. That resolves the schedule risk above rather than compounding
it — the block that produces no demo is the same block that needs no corpus to
begin, and is the least likely to be rebuilt.

Caveat: this is invariance of *necessity*, not of final code shape. A corpus will
tune constants, budget sizes and refinement paths throughout. Read 75–80% as
"won't be thrown away", not "won't be touched".

**Gate on a corpus, don't guess:** the tail of $\mathcal{G}$ (§2); envelope
bounds $N_{\text{copies}}$/$N_{\text{crossings}}$ (§8) and budget ledger sizes
(§7); tangential cell density (§9.2 — the *need* is invariant for any mechanical
corpus, the depth is not); operation mix (§15, and
[`TEXT_TO_CAD_INTEGRATION.md`](TEXT_TO_CAD_INTEGRATION.md) is a live input
here); prevalence weights for §21.

---

## 8. What truck gets right and wrong: the vendoring map

The keep/rework/replace call, component by component. Salvage is by LOC of the
component, judged as "code that survives into a certified kernel".

| Component | LOC | Verdict | Salvage |
|---|---|---|---|
| `truck-topology` | 3.4k | **Rework, keep the core** | ~80% |
| `truck-geometry` NURBS + `specifieds/` | 11k | **Keep as fast path, extend** | ~70% |
| `truck-geotrait` | 1.9k | **Keep, add a second interface** | ~40% |
| `truck-modeling` | 2k | **Keep the abstraction, replace the carriers** | ~50% |
| `truck-shapeops` Boolean | ~3.5k | **Replace** | ~10% |
| `truck-shapeops` fillet | ~1k | **Keep the mathematics, replace the scaffolding** | ~40% |
| `truck-shapeops` healing | ~0.2k | **Keep** | ~90% |
| `truck-base` | 1.9k | **Keep** | ~85% |

Overall **≈55–65% by LOC** — comfortably past the 20% bar.

### What truck gets right

These are the reasons vendoring is the correct call, and they should be
protected during the rewrite rather than casually refactored away.

1. **The topological core is correct, not merely present.** `Solid::try_new`
   (`solid.rs:18-31`) checks non-empty, connected, closed-by-coedge-pairing, and
   non-singular-vertex. The link test is genuinely sound: `singular_vertices`
   (`shell.rs:536`) tests link *connectivity*, which — **given** the `Closed`
   check that runs first — implies a single cycle, because every link node then
   has degree exactly 2. Most kernels this size do less, and do it wrong.
2. **The entity/use split already exists.** See Stage 1. This is the single
   piece of design that makes S-1 tractable rather than a rewrite, and it was
   not obvious from the type names.
3. **The generic sweep abstraction is well factored.** `Sweep`/`ClosedSweep`
   over `GeometricMapping` + `Connector` (`truck-modeling/src/sweep.rs`,
   `geom_impls.rs`) cleanly separates the combinatorial sweep from carrier
   generation. It is the right *shape* for §15 and needs gates, not surgery.
4. **The NURBS layer is real, substantial work.** 11k lines of B-splines, knot
   vectors, degree elevation, interpolation, subdivision — competent, and the
   convex-hull property it already exposes is the natural basis for D-1's
   enclosure interface.
5. **`RbfSurface` is real mathematics.** Contact circles and a rational fit to
   the rolling-ball envelope. Its faults are ungated entry and sampled error
   control (F-4, F-6), not conception.
6. **Healing** is a genuine §19 in miniature, and the one place in truck that
   treats a defect as something to repair rather than assert away.
7. **Property-based testing** with `proptest` is used in the right places.

### What truck gets wrong

Ordered by how much they constrain a vendored rewrite:

1. **D-1** — pointwise `f64` evaluation. Forecloses every certificate in the
   spec. The deepest fault in the tree.
2. **S-3** — mesh-seeded topology. Not a defect *in* the Boolean; it *is* the
   Boolean. Unpatchable, hence Stage 4 is a rewrite.
3. **D-2** — shared mutable geometry, allocation identity, systemic deadlock
   hazard.
4. **S-2 / D-3 / D-4** — one absolute global tolerance; no cylinder or cone; no
   canonical carrier set. Individually cheap, and all three get much more
   expensive after code depends on them, which is why they are Stage 1.
5. **§4, the missing evidence layer** — `Option<T>` everywhere, 73 signatures in
   shapeops alone. Pervasive but shallow: it is a mechanical change, not a
   design problem, and it should be designed once in Stage 1 and threaded
   outward rather than retrofitted per stage.
6. **The nine `F-` faults** — real, but note that F-1, F-2, F-3 and F-8 all live
   inside code Stage 4 deletes. Only F-9 and F-4a are worth fixing ahead of the
   rewrite (Stage 0).

### The one thing to decide before starting

Stages 1 and 2 are ~10,000–16,000 lines that produce no user-visible capability.
Everything downstream depends on them, and doing them *after* any of Stages 3–5
means doing those stages twice. If that run-up is not affordable, the honest
alternative is not "a cheaper version of this plan" — it is Stage 0 plus the old
fault-fixing sequence, accepting an uncertified kernel indefinitely. The two are
different projects and the middle ground is the expensive place to stand.

## 9. Relationship to `look` as it stands today

Two clarifications, so this audit is not read as a status report on `look`:

- **`look` does not use the generation path.** The dependency set in
  `Cargo.toml` is `truck-{assembly,meshalgo,polymesh,stepio,topology}`.
  `truck-shapeops` is not a dependency: no Boolean, no fillet from this audit
  runs in `look` today. `truck-modeling` and `truck-geometry` arrive only
  through the `[patch.crates-io]` table.
- **The formal work already in `look` is on the ingestion side and is not
  credited to truck above.** `truck-meshalgo/src/tessellation/formal/*` and
  `tessellation/domain/*` in our fork are the STEP-import atlas built against
  [`FORMAL_SYSTEM_STEP_INGESTION.md`](FORMAL_SYSTEM_STEP_INGESTION.md), with its
  own evidence types (`formal/evidence.rs`, `formal/outcome.rs`) and quotient
  machinery. Notably, **that work already has the two things this audit finds
  most conspicuously missing from truck proper** — a typed outcome classifier
  and an explicit evidence ledger. If the generation path is ever built here,
  those are the models to extend, not `Option<T>`.

So the practical reading: this document scopes what adopting truck as a
*generation* kernel would cost, and the answer is that the ingestion-side
investment already made is the more transferable asset — the outcome/evidence
architecture generalises, while truck's generation code would need gates,
certificates, and a coedge before any of it could claim a construction.
