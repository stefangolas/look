# Constructive Geometry Kernel — Program Plan (CG program)

**Status:** approved program design, session 43. This is the loop-side
execution plan: it books the packet list, write sets, dependency graph, and
gates. The full kernel-side specification (design rationale, formulas, formal
properties) lives in the truck-fork repo at
`truck-fork/CONSTRUCTIVE-GEOMETRY-BUILD-SPEC.md` (committed on truck
`master`); the load-bearing contract content is **quoted here** so packets and
workers never need that repo.

## 1. Thesis

```text
authored topology
→ shared boundary geometry
→ direct realization
→ separated topological/geometric certification
→ topology-preserving tessellation
```

A procedural client knows BREP incidence before realization. The kernel
preserves that knowledge instead of recovering adjacency by proximity, sewing,
or booleans. The Exeter benchmark proved the fast end of this (explicit shared
vertices/edges, planar facets, combinatorial closure, no sewing/healing) works
today on existing primitives. This program generalizes only the pieces that
are genuinely kernel-level.

## 2. Scope and disposition

| Kernel packet | Disposition |
| --- | --- |
| TR-SWP-001 spine/frame recipe + frame laws | **Core, first** |
| TR-SWP-002-FAC direct facet realization | **Core, first (mandatory backend)** |
| TR-MESH-001 topology-preserving mesh assembly | **Core** |
| TR-TOP-001 manifold diagnostics | **Reduced: aggregation + vertex link + orientation diagnostics only** |
| TR-VAL-001 realization certification | **Integration into existing certificate/evidence types only** |
| TR-GEO-001 Coons4 | **Kept, parallel-eligible, not on the rendering critical path** |
| TR-SWP-002-BREP parametric `SpineFrameSurface` | **Second stage** |
| TR-GEO-002 triangular transfinite | **Deferred** (facets cover it; rectangular-domain trait risk) |
| TR-DIR-001 public `DirectBrepBuilder` | **Deferred** (handles already author topology; FAC uses a *private* grid registry) |
| Global BVH / self-intersection subsystem | **Deferred** (mesh-level audit covers FAC; §7) |
| TR-NRB-001 recipe→NURBS | **Follow-on**, estimated after the enum/STEP contract is frozen |

Promotion doctrine for anything deferred: independent reinvention by two
clients, lost kernel information, representation unlock, or
client-becomes-mini-kernel. Repeated use inside one client is insufficient.

## 3. Booked contract (to be landed as real, compiling signatures by CG-000)

Everything below is **normative for the program**; CG-000 turns it into
`vendor/truck` code with stub bodies. Exact Rust spelling is adjustable at
CG-000 review; semantics are not.

### 3.1 Placement

- `truck-geometry/src/constructive/` — new module: recipe, frame laws,
  profile laws, errors, sampling policy. Additive; nothing existing moves.
- `truck-modeling` — facet backend + (stage 2) the parametric surface and
  sweep constructor. **Placement decision booked at CG-000:** emitting
  `PolygonMesh` from `truck-modeling` requires adding `truck-polymesh` as a
  dependency (leaf crate, no cycle) — default is to add it; alternative is
  truck-meshalgo, rejected by default because construction does not belong in
  the tessellation crate.
- `truck-topology/src/manifold.rs` — new diagnostics module.
- `truck-meshalgo` — ledger entry point + certificate integration.
- `truck-modeling/src/geometry.rs` — enum ripple (CG-009 only; see §4).

### 3.2 Types

```rust
// truck-geometry/src/constructive/mod.rs
pub struct Frame3 { pub tangent: Vector3, pub normal: Vector3, pub binormal: Vector3 }
    // orthonormal, right-handed; tangent = spine direction.

pub enum FrameLaw {
    FixedPlane { normal: Vector3 },
    ArchitecturalUp { up: Vector3 },
    ParallelTransport { initial_normal: Vector3 },
    RadialAboutAxis { origin: Point3, axis: Vector3 },
}

pub enum ProfileLaw {
    Constant(Profile2D),
    Scale { profile: Profile2D, scale: ScalarLaw },
    LinearCorrespondence { start: Profile2D, end: Profile2D },
    // LinearCorrespondence requires explicit declared vertex/edge
    // correspondence between start and end; correspondence is never inferred.

pub struct SpineFrameRecipe<S, P, F> { pub spine: S, pub profile_law: P, pub frame_law: F }
    // Core evaluator: X(s, v) = C(s) + T(s) * P(s, v)
    impl: fn position(&self, s: f64, v: f64) -> Point3
          fn frame(&self, s: f64) -> Result<Frame3, ConstructError>
          fn profile(&self, s: f64, v: f64) -> Result<Point2, ConstructError>

pub enum ConstructError {
    ZeroTangent { at: f64 },
    FrameSingular { at: f64, law: &'static str },   // e.g. up ∥ tangent
    SpineNotC1 { at: f64 },
    ProfileCorrespondenceMismatch,
    ProfileCollapse { at: f64 },
    NonFinite { at: f64 },
    InvalidInput,
}

pub enum SamplingPolicy {
    UniformCount { spine: usize },
    CustomParameters(Vec<f64>),
    ChordTolerance(f64),
    AngularTolerance(f64),
}

pub struct DirectTolerance { pub position: f64, pub parameter: f64, pub jacobian: f64, pub intersection: f64 }
    // truck-geometry or truck-base; defaults derive from truck_base::tolerance.
```

**Spine smoothness contract (normative).** MVP spines MUST be C¹ on the
evaluated interval. Non-C¹ spines are typed-refused (`SpineNotC1`), never
clamped or silently smoothed. Detection is declaration-based or
tangent-discontinuity sampling beyond `DirectTolerance::parameter`; the
mechanism must be deterministic. An explicit Miter corner policy MAY be added
later; Bevel/Round MAY follow; corner policy is always explicit caller input.

**Frame laws (normative semantics).**

- `FixedPlane`: `t = C'/‖C'‖`, `b = normal`, `n = b × t`; refuse `‖C'‖ < τ`.
  Preferred for planar spines.
- `ArchitecturalUp`: `b = normalize(up × t)`, `n = t × b`; refuse `up ∥ t`
  unless an explicit fallback policy is supplied. No silent frame rotation.
- `ParallelTransport`: Bishop rotation-minimizing frame via the
  **double-reflection method**; stable at zero curvature and inflections;
  deterministic from `initial_normal`. Frenet framing is never the default.
- `RadialAboutAxis`: analytic from the axis; rotated copies equivariant
  modulo floating-point.

### 3.3 Facet backend doctrine (CG-004)

- **Primary contractual output: `PolygonMesh` with exact shared indices.**
  Faceted `Shell`/`Solid` emission is an explicit opt-in secondary target
  (an m×k grid becomes m·k planar faces — document the consequence at the
  opt-in API). Faceted BREP is NOT built in CG-004; it arrives with CG-009
  material only if still wanted.
- Structured grid `x_ij = X(s_i, p_j)`; grid vertex `(i,j)` is created
  **exactly once** via a private grid registry (keyed entity cache, internal
  only — never public builder API). Adjacent faces reuse the identity;
  internal grid edges are created once, traversed oppositely by their two
  faces. No sewing.
- Cell triangulation: quad if planarity is explicitly certified, else two
  triangles; **diagonal choice is deterministic**, never an unstable
  float-comparison.
- Caps: closed planar start/end rings via existing planar support. No
  nonplanar cap solving.
- Performance contract: no surface fitting, Newton, sewing, healing,
  booleans, or generic surface/surface intersection on the fast path.
- **Mandatory mesh-level sanity audit on output** (BVH stays deferred):
  signed-volume sign sanity; twin-triangle winding audit (every interior mesh
  edge referenced by exactly two opposite-winding triangles); optional deeper
  pass via the existing collision analyzers. Failed winding audit = FAILED,
  not a warning. Verdicts are three-valued:
  `CERTIFIED_WITHIN_TOLERANCE | FAILED | INCONCLUSIVE` — uncertainty is
  surfaced, never converted into success.

### 3.4 Index-identity convention (single, frozen; two consumers)

One convention, defined in CG-000, shared by FAC's grid registry and the
meshalgo ledger: **a mesh position index is a pure function of (entity
identity, sample ordinal)** — never of coordinates.

```rust
// truck-meshalgo (ledger entry point returns it; not a rewrite of triangulation.rs)
pub struct EdgeSampleLedger { pub edge_id: EdgeID<Curve>, pub parameters: Vec<f64>, pub position_indices: Vec<usize> }
```

- Each unique `EdgeID` is sampled once; a reversed edge consumes the same
  integer sequence reversed.
- Watertightness invariant: for incident faces A, B sharing edge E,
  `I(A,E) == reverse(I(B,E))` **as integer sequences**. If the shell is
  combinatorially closed and every boundary mesh vertex's index derives from
  `(EdgeID, ordinal)`, the emitted mesh is closed **by construction** —
  `put_together_same_attrs` (positional welding) is never invoked.
- Implementation shape: a **new parallel entry point**
  (`triangulation_with_ledger`-style) that reuses the existing unique-edge
  sampling and per-face CDT internals and returns
  `(ledger, per-face local-index triangulations)`; global assembly happens
  outside. **Existing entry points remain bit-identical** (the V5 identity
  guard is law here).

### 3.5 Certificate integration (CG-000 freezes the mapping)

New evidence composes with the existing `MeshedShellOutcome` /
`FaceValidityCertificate` / provenance vocabulary — no parallel validation
universe. CG-000 delivers the **field-level mapping table**: which existing
type carries spine/frame validity, profile collapse, Jacobian bounds,
shared-edge pair errors (EdgeID + FaceID A + FaceID B + error_a + error_b),
and the winding audit, and where a new evidence variant must be added.
CG-007 cannot be dispatched against an unfrozen mapping.

### 3.6 Manifold diagnostics (CG-006)

Aggregate, do not duplicate: `shell_condition()`, `connected_components()`,
`extract_boundaries()`, `singular_vertices()`, `face_adjacency()` are the
substrate. Deliverable is actionable explanation:

```rust
pub struct ManifoldDiagnostics {
    pub shell_condition: ShellCondition,
    pub connected_components: usize,
    pub boundary_edges: Vec<EdgeID<Curve>>,
    pub irregular_edges: Vec<EdgeDiagnostic>,
    pub singular_vertices: Vec<VertexDiagnostic>,
    pub orientation_conflicts: Vec<OrientationDiagnostic>,
}
```

Plus: vertex-link classification (closed 2-manifold ⇒ link is one cycle;
with boundary ⇒ one path; two sheets touching at a vertex ⇒ nonmanifold);
orientation diagnostics returning a consistent parity assignment or the
conflicting edges/faces (analysis only; a separate explicit op MAY apply it —
no silent repair); outward sign via signed volume
`V = ⅙ Σ a·(b×c)` (reuse `CalcVolume`), never a centroid-normal test.

### 3.7 Coons4 (CG-008)

Bilinearly blended Coons patch; boundary correctness is by exact pairwise
cancellation against the corner term and is asserted numerically. Full fork
trait checklist: `ParametricSurface, ParametricSurface3D, BoundedSurface,
ParameterDivision2D, SearchParameter<D2>, Invertible, Transformed<Matrix4>,
IncludeCurve`. First derivatives analytic; constructor validates corner
consistency to `DirectTolerance::position` and never guesses orientation (a
convenience constructor MAY try finite legal reversals and return the chosen
one). Regularity is certified, not assumed: expose `J = S_u × S_v`; folded
patches are construction-valid, geometry-invalid.

## 4. Packet list, write sets, classes

| Packet | Class | Write set (additive unless noted) | Depends on |
| --- | --- | --- | --- |
| `BG-CG-000-CONTRACT` | design | `truck-geometry/src/constructive/{mod,recipe,errors,sampling}.rs` (new, stub bodies), `DirectTolerance`, the §3.4 convention doc-comment, the §3.5 mapping table (as doc or `docs/`), unit-shape tests marked as contract | — |
| `BG-CG-001-RECIPE` | design | `constructive/recipe.rs`, `constructive/profile.rs` fill-in + tests | 000 |
| `BG-CG-002-FRAMES-ANALYTIC` | mechanical | `constructive/frame_fixed.rs`, `frame_up.rs`, `frame_radial.rs` (new files) + tests | 001 |
| `BG-CG-003-TRANSPORT` | design | `constructive/frame_transport.rs` (new file) + tests. **Never split.** | 001 |
| `BG-CG-004-FACET` | design | `truck-modeling/src/facet_sweep.rs` (new) + `Cargo.toml` (polymesh dep, per §3.1) + tests | 001–003 |
| `BG-CG-005-LEDGER` | mechanical+ | `truck-meshalgo/src/tessellation/triangulation_with_ledger.rs` (new) + minimal `mod.rs` wiring + tests | 000 |
| `BG-CG-006-DIAG` | mechanical | `truck-topology/src/manifold.rs` (new) + tests | 000 |
| `BG-CG-007-CERT` | design | `truck-meshalgo` validity/evidence integration + new evidence variant + tests | 000, 004, 006 |
| `BG-CG-008-COONS` | design | `truck-geometry/src/decorators/coons.rs` (new) + tests | 000 |
| `BG-CG-009-BREP` | design | **`truck-modeling/src/geometry.rs` enum ripple (integrator-owned)** + `SpineFrameSurface` + `spine_sweep` topology constructor (per-profile-edge side faces, shared longitudinal trajectory edges, `try_attach_plane` caps) | 004 |

Elastic pool (dispatch whenever a slot is idle; lowest review-judgment):
corpus fixtures (cube, holed prism, multi-hole plate, straight/tapered/90°
ducts, S-rail, annular sweep, variable-radius passage, ribbed panel, Coons
warped quad shell, repeated sweep assembly), mutation batteries (§5 of the
truck-fork spec), kernel microbenchmarks (24×32, 24×128, 100/1,000 sweeps,
1,000 panels, 10,000 faces). Corpus and mutation work is the parallelism
elasticity of this program.

## 5. Velocity recalibration and the parallelism strategy

Measured context: the coverage program landed ~69k insertions across sessions
41–42 at a cadence of several packets per session. Against that demonstrated
throughput, this program's core (~2,000–3,900 LOC production) is a **3–7
working-day serial effort**, not the multi-week effort a generic solo-dev
baseline suggests. Consequences, deliberately decided now:

1. **Full-wave orchestration is NOT planned.** Coordination overhead (unit
   specs, review, merge friction) is ~1–1.5 days fixed; perfect 2×
   compression of a 5-day critical path saves ~2.5 days — a wash at best
   once merge-point debugging is counted.
2. **Concurrency is capped at ≤3 live packets**, and only over the
   write-set-disjoint set (§6). Prefer serial for design-class packets;
   the loop's write-set-disjointness rule (`schedule.py`) is the authority.
3. **The elastic pool is where parallelism pays**: corpus/mutation/benchmark
   packets fill idle slots. They are per-fixture mechanical, cheap to review,
   and the most likely work to be skipped under time pressure.
4. **Escalation trigger:** if CG-009's enum ripple has not converged within
   ~2 days of starting, split and parallelize then (that is where the phase
   can balloon).

## 6. Dependency graph

```text
CG-000 (serial, everything types against it)
   ├─→ CG-001 ─┬→ CG-002 ─┐
   │           └→ CG-003 ─┴→ CG-004 ─→ CG-009 (enum ripple, integrator)
   ├─→ CG-005                      │
   ├─→ CG-006 ─────────────────────┴→ CG-007
   └─→ CG-008 (independent; parallel-eligible from the start)

Elastic pool: runs whenever slots are idle, after CG-000.
```

Disjointness notes: CG-002/003 own separate new files and share only CG-001's
landed types; CG-005/006/008 are mutually disjoint and disjoint from the
geometry chain. CG-009 touches shared files and runs effectively alone.

## 7. Gates and acceptance

Standard V0–V10 apply. Program-specific invariants (packets book these in
their done-when):

- **No welding:** every closed test shell tessellates → `PolygonMesh` reports
  Closed with no `put_together_same_attrs` (or any positional welding).
- **Integer identity:** `I(A,E) == reverse(I(B,E))` asserted as integers, not
  coordinates, in CG-005's tests.
- **Determinism:** identical ordered input + tolerance → byte-identical mesh
  position indices and identical verdicts, repeated runs. Parallel writes go
  to index-stable slots; float reductions combine in fixed order (never
  `par.sum()`); no observable output ordering derives from hash-map iteration.
- **C¹ refusals:** polyline-spine fixtures produce `SpineNotC1` (typed), in
  CG-001's tests.
- **Sanity audit:** winding-audit failure ⇒ FAILED in CG-004's tests; an
  INCONCLUSIVE verdict is representable and surfaced.
- **Existing entry points bit-identical:** CG-005 must not change
  `triangulation`'s existing behavior (V5 identity guard).
- Fixture discipline carried from STATE traps: fixtures that tessellate or
  transform circle-carrying solids are release-only or `#[cfg(not(debug_
  assertions))]` (the debug self-loop panic) until the booked topology fold
  fix lands; `cargo` commands grade with `-p <crate> --lib --tests`, never a
  bare `cargo test`.

## 8. The Exeter regression gate

After CG-004 lands (before CG-007 ideally, no later than CG-009): migrate one
Exeter rib from the client's local spine/profile transport to the kernel
`SpineFrameRecipe` + FAC, and measure construction wall time, allocations if
available, vertex/face counts, closure, signed volume, and geometry deviation
against the local implementation. Parity or improvement is the bar; a
substantial regression requires written justification before CG-009 proceeds.
This is a client-side milestone, not a kernel packet — no cathedral-specific
logic moves into `vendor/truck`. It also validates §1's cross-domain rule:
the same kernel API must serve the Exeter rib, a curved rectangular duct, and
a coolant-passage fixture without domain flags.

## 9. Definition of completion

- One generic API constructs a curved duct, coolant passage, molded member,
  and the Exeter rib.
- FAC emits exact shared-topology `PolygonMesh` without sewing/healing/
  fitting/booleans, passing the §3.3 audit.
- Smooth Truck BREP tessellates with `EdgeID`-derived shared boundary indices,
  no welding (CG-005).
- Constructive geometry certifies inside the existing evidence framework per
  the frozen §3.5 mapping (CG-007).
- Optional: the same recipe becomes resolution-independent parametric BREP
  without client rewrite (CG-009).
- No public kernel API knows what an Exeter vault, rocket nozzle, turbine
  blade, or muqarnas cell is.
- Deferred items (§2) remain deferred absent a §2 promotion trigger.
