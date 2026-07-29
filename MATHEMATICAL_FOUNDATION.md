# Certified Geometry Ingestion for STEP-to-GLB

## Mathematical Bedrock, Contract Registry, Rust Enforcement Architecture, and Implementation Roadmap

**Status:** Working architectural specification — authority for the STEP ingestion layer  
**Kernel strategy:** owned architectural fork of `truck` (decided 2026-07-29, §31a)  
**Primary audience:** Future agents and maintainers working on `look`, `truck-stepio`, and `truck-meshalgo`  
**Snapshot date:** 2026-07-29  
**Purpose:** Establish a durable mathematical and software foundation for converting STEP B-reps into trustworthy meshes without silently turning invalid intermediate states into plausible geometry.

---

## 0. Read This First

This document defines the geometry pipeline in four layers:

1. **Exact mathematical semantics** — what the B-rep, trimming domain, quotient topology, triangulation, and mesh are supposed to mean over ideal real geometry.
2. **Numerical certification semantics** — what the implementation can actually establish with finite-precision arithmetic, tolerances, adaptive sampling, and retained witnesses.
3. **Rust enforcement semantics** — how the API makes it impossible, or at least difficult, for later stages to consume a state whose required obligations have not been checked.
4. **Resource and cost semantics** — what each obligation costs to establish, which runtime tier it belongs to, and how imported values are prevented from inducing unbounded work. A correct kernel that aborts on a real file, or that doubles peak memory to carry its proofs, has not succeeded. See §20a and §29a.

The central design rule is:

> Every pipeline stage is a fallible constructor. Its output type represents a stronger state and carries evidence for the obligations that were discharged.

The project should not attempt to infer correctness from the final render. A smooth blob can be the result of invalid topology, wrong entity identity, a bad curve-surface association, a periodic lift error, a missing triangulation constraint, or wrong material-domain semantics. Many of those failures produce numerically smooth, internally plausible meshes.

Instead, correctness is established incrementally:

```text
STEP entities
→ resolved source topology
→ converted geometry with retained identity
→ sampled edges with provenance
→ certified curve-on-surface representations
→ quotient-resolved trim loops
→ resolved material domain
→ valid trim arrangement
→ conforming constrained triangulation
→ certified face mesh
→ certified shell mesh
```

The design is not dependent on any one bug. It is a general method for building an auditable geometry ingestion kernel.

---

# Part I — Why This Approach Works

## 1. The core problem

A traditional geometry pipeline often uses permissive types:

```rust
Vec<Edge>
Option<Point2>
PolylineCurve
Vec<Triangle>
```

Those types say almost nothing about what is true.

A `PolylineCurve` might be:

- sampled from the correct source edge;
- sampled from a neighboring edge due to an index bug;
- missing one source edge because `filter_map` dropped it;
- geometrically incompatible with the face surface;
- projected onto a distant nearest point;
- lifted into an inconsistent periodic deck copy;
- open in Euclidean UV but closed on the quotient;
- or not a valid boundary at all.

If all of those states share the same type, every downstream algorithm must rediscover the truth—or, more commonly, assumes the data are valid and creates a plausible wrong result.

The certified architecture changes the representation so that later stages receive more specific types:

```rust
ResolvedEdgeUse
TopologicallyClosedWire
SampledEdge
CurveOnSurface
QuotientClosedLoop
ResolvedFaceDomain
ConformingCdt
CertifiedFaceMesh
```

Each type is constructible only through a check that establishes its meaning.

## 2. Proof workflow versus proof of floating-point geometry

Rust will not prove arbitrary NURBS, inverse projection, or surface approximation correct over IEEE-754 arithmetic.

What Rust can enforce is:

> An operation requiring a certified state cannot be called unless the corresponding constructor has run successfully.

For example:

```rust
fn lift_periodic_bounds(
    face: CurveSurfaceCompatibleFace,
) -> Result<QuotientResolvedFace, LiftError>;
```

Safe code cannot pass an arbitrary `ResolvedFace` into this function. It must first obtain a `CurveSurfaceCompatibleFace` through the compatibility checker.

This divides proof claims into three classes.

### 2.1 Exact structural proof

Examples:

- the source entity ID requested by a face equals the source ID stored at the resolved arena index;
- every source edge use is preserved during wire conversion;
- a compact index is in bounds;
- a graph potential satisfies every integer deck constraint;
- every requested CDT constraint has a retained constrained-edge chain.

These are exact discrete properties. They can often be enforced by types and formally verified for critical algorithms.

### 2.2 Numerically certified property

Examples:

- a sampled edge lies on a surface within tolerance;
- evaluated endpoints close within tolerance;
- a mesh triangle approximates the surface within a measured bound;
- a shared edge agrees between two face meshes within tolerance.

These require a measured witness:

```rust
struct WithinTolerance {
    measured: f64,
    permitted: f64,
}
```

The certificate must retain the method used to establish the bound.

### 2.3 Heuristic diagnostic

Examples:

- a mesh bounding box is suspiciously large;
- residual vectors resemble a placement error;
- a face has an abnormal triangle-area ratio;
- one surface appears to fit an edge better than another under a coarse grid search.

Diagnostics help prioritize investigation, but they do not construct proof-bearing types.

A recurring lesson from the current debugging work is that a detector can itself be wrong. Diagnostics must be audited before their causal interpretation is trusted.

---

# Part II — Mathematical Primitives

## 3. Ambient spaces and notation

Let:

- \(\mathbb R^2\) be parameter space;
- \(\mathbb R^3\) be physical Euclidean space;
- \(\|x\|\) be Euclidean norm;
- \(d(x,A)=\inf_{a\in A}\|x-a\|\) be distance to a set;
- \(\varepsilon\) denote a declared tolerance with explicit units and provenance.

A tolerance is never an anonymous global epsilon. It must state what it bounds:

```rust
enum ToleranceKind {
    SourceGeometry,
    CurveEvaluation,
    CurveSurfaceCompatibility,
    WireClosure,
    Welding,
    SurfaceApproximation,
    ShellSewing,
}
```

## 4. B-rep topology

Define a source B-rep incidence structure:

\[
\mathcal B=(V,E,W,F,H),
\]

where:

- \(V\) is the set of vertices;
- \(E\) is the set of topological edges;
- \(W\) is the set of oriented wires;
- \(F\) is the set of faces;
- \(H\) is the set of shells.

The topology carries incidence maps:

\[
\partial_E:E\to V\times V,
\]

\[
\partial_W:W\to E^*,
\]

\[
\partial_F:F\to W^*,
\]

where \(E^*\) and \(W^*\) are oriented finite sequences.

Topology and geometry are separate. Two topological edges may share geometric support. One topological seam edge may have multiple face-side p-curves. A zero-length physical edge may remain topologically meaningful.

## 5. Curves

A 3D edge curve is:

\[
C_e:[a_e,b_e]\to\mathbb R^3.
\]

It has:

- source identity;
- parameter range;
- effective orientation;
- endpoint vertices;
- geometric representation;
- optional periodic metadata.

A sampled edge is a finite parameter-point sequence:

\[
\{(t_j,P_j)\}_{j=0}^{m},
\qquad P_j\approx C_e(t_j).
\]

Sampling fidelity is a separate obligation from source identity.

## 6. Surfaces

A supporting surface is:

\[
S_f:\Omega_f\to\mathbb R^3,
\]

where \(\Omega_f\subseteq\mathbb R^2\) is a local parameter domain.

At regular points, define the Jacobian:

\[
J=[S_u\;S_v].
\]

The surface is regular where:

\[
\operatorname{rank}(J)=2.
\]

The first fundamental form is:

\[
G=J^\mathsf TJ.
\]

It converts small parameter displacement into approximate physical distance:

\[
\|\Delta x\|^2\approx\Delta q^\mathsf TG\Delta q.
\]

Singular points—sphere poles, cone apexes, collapsed parameter boundaries—must be represented explicitly rather than treated as ordinary regular UV points.

## 7. Periodic parameter domains and quotient spaces

A periodic surface has a period lattice:

\[
\Lambda=
\{(k_uP_u,k_vP_v):k_u,k_v\in\mathbb Z\}.
\]

Coordinates are equivalent when:

\[
q\sim q+\lambda,
\qquad \lambda\in\Lambda.
\]

The physical parameter space is the quotient:

\[
Q=\Omega/\Lambda.
\]

Examples:

- cylinder: one periodic coordinate;
- torus: two periodic coordinates;
- sphere: one periodic coordinate plus singular poles;
- cone: one periodic coordinate plus an apex singularity.

These are not all equivalent topological cases.

## 8. Curves on surfaces

For edge \(e\) used by face \(f\), a p-curve or reconstructed UV path is:

\[
q_{e,f}:[a_e,b_e]\to Q_f.
\]

The exact compatibility condition is:

\[
C_e(t)=S_f(q_{e,f}(t))
\qquad\forall t\in[a_e,b_e].
\]

The numerical implementation generally establishes:

\[
\sup_t\|C_e(t)-S_f(q_{e,f}(t))\|
\le\varepsilon_{e,f}.
\]

A nearest-point projection is not automatically an inverse. A projection result must carry its residual.

## 9. Trim loops and face domains

A trimmed face is modeled as:

\[
F=(S,Q,\Gamma,\beta,o),
\]

where:

- \(S\) is the supporting surface;
- \(Q\) is its regularized parameter domain or quotient;
- \(\Gamma=\{\gamma_1,\dots,\gamma_n\}\) is the set of trim loops;
- \(\beta\) describes base material occupancy;
- \(o\) is face orientation.

For parity-based classification, material membership is:

\[
\chi_M(p)=\chi_{\mathrm{base}}(p)
\oplus
\operatorname{crossingParity}_{\Gamma}(p).
\]

The base domain is not derivable from signed loop area alone.

The formal model preserves two different facts:

1. source STEP syntax, such as `FACE_OUTER_BOUND` versus `FACE_BOUND`;
2. resolved material semantics, such as outer material boundary versus inner hole.

They must not be collapsed prematurely.

## 10. Boundary arrangements

The discretized trim segments define an arrangement \(A(\Sigma)\) in parameter space or on the quotient.

The arrangement consists of:

- vertices at endpoints and proper intersections;
- edges between arrangement vertices;
- cells of the complement;
- adjacency relations between cells.

The arrangement is valid only if intersections and overlaps are explicitly represented or rejected.

## 11. Constrained triangulations

Let \(K\) be a triangulation of the parameter region and \(\Sigma\) the requested trim constraints.

A conforming triangulation must satisfy:

1. every requested constraint is represented by a complete constrained-edge chain;
2. no triangle interior crosses a trim constraint;
3. every retained triangle belongs to a correctly labeled material cell.

## 12. Face and shell meshes

A face mesh is a discrete approximation:

\[
X_f\approx S_f(M_f).
\]

A shell mesh is assembled from face meshes and must preserve source adjacency and expected shell topology.

The mesh is derived and disposable. The B-rep and certificates remain authoritative.

---

# Part III — Contract Registry

Every obligation receives a stable identifier. Code, tests, diagnostics, PRs, and failure reports should reference these IDs.

## 13. Topology and identity contracts

### TOP-001 — Source reference integrity

For every source entity ID \(i\):

\[
\operatorname{sourceId}
\left(
\operatorname{arena}
[
\operatorname{lookup}(i)
]
\right)=i.
\]

**Meaning:** a source reference resolves to the exact converted entity it names.

Equivalently, writing \(M\) for the source map and \(A\) for the arena:

\[
M(i)=k
\implies
A[k].\mathrm{sourceId}=i.
\]

**Failure class:** wrong neighboring edge or surface due to map/vector desynchronization.

**Rust enforcement:** distinct source-ID and arena-index types; private arena indices; transactional insertion; **`source_id` retained in every arena item**, so the implication above is independently checkable rather than merely true.

**Cost:** structural tier, always on. One `u64` per entity of persistent memory; one integer comparison per lookup.

---

### TOP-002 — Transactional conversion insertion

A failed conversion creates neither:

- an arena object;
- nor a source-to-index mapping.

A successful conversion creates both atomically.

**Failure class:** reserve-before-convert bugs such as the `eidx_map` defect and its unrepaired twin in vertex conversion (§51a).

**Rust enforcement:** one generic arena used by every entity kind, so this has a single implementation to audit.

**Cost:** free — it is an ordering constraint, not a check.

---

### TOP-007 — Canonical entity identity

One source identity denotes one converted object. For every source ID \(i\):

- repeated resolution of \(i\) returns the same index;
- a first-time conversion that fails creates no arena item and no mapping;
- no second item is ever created for \(i\).

**Meaning:** a repeated reference is ordinary — a shell names the same
`EDGE_CURVE` from both faces sharing it — and must resolve, not error. Duplicate
*objects* for one identity are the failure; duplicate *references* are not.

**Rust enforcement:** `get_or_try_insert` with entry-based canonical dedup
(§22.1). `DuplicateSource` is reserved for an internal attempt to create a
second canonical object.

**Cost:** structural tier, always on. One hash lookup per reference, which the
conversion already performed.

---

### TOP-003 — Wire edge-use cardinality preservation

\[
|\partial_W^{\mathrm{source}}(w)|
=
|\partial_W^{\mathrm{resolved}}(w)|.
\]

**Meaning:** topology construction may not silently drop unresolved edge uses.

**Rust enforcement:** `collect::<Result<Vec<_>, _>>()`, never `filter_map`.

---

### TOP-004 — Topological wire closure

For oriented uses \(e_1,\dots,e_n\):

\[
\operatorname{end}(e_i)=
\operatorname{start}(e_{i+1}),
\]

cyclically.

**Rust type:** `TopologicallyClosedWire`.

---

### TOP-005 — Effective orientation consistency

The effective traversal is the composition of face, bound, oriented-edge, and edge-curve orientation.

The composed sequence must agree with source incidence.

---

### TOP-006 — Edge-use multiplicity agreement

For manifold closed source shells, each ordinary edge normally has two incident face uses with opposite effective traversal. Open or nonmanifold source topology must be represented explicitly rather than rejected by assumption.

---

## 14. Geometry conversion and provenance contracts

### GEO-001 — Converted geometry equivalence

For source geometry \(G^{\mathrm{STEP}}\), converted geometry \(G^{\mathrm{int}}\), and the declared coordinate transform \(T\):

\[
G^{\mathrm{int}}=T(G^{\mathrm{STEP}}).
\]

For analytic primitives, compare structural invariants:

- circle center, frame, radius;
- cylinder axis, frame, radius;
- sphere center and radius;
- torus frame and radii;
- cone apex, axis, and angle.

---

### GEO-002 — Sample provenance

Every sampled point retains:

- source edge ID;
- arena edge index;
- curve parameter;
- generated 3D point.

---

### GEO-003 — Sampling fidelity

For every sample:

\[
\|P_j-C_e(t_j)\|
\le\varepsilon_{\mathrm{eval}}.
\]

**Failure class:** correct edge identity but wrong or mixed points.

---

### GEO-004 — Shared physical edge consistency

For one source edge used by faces \(f_1\) and \(f_2\):

\[
C_{e,f_1}(t)=C_{e,f_2}(t)=C_e(t).
\]

Face-side p-curves may differ; physical edge geometry may not.

---

### GEO-005 — Curve-surface compatibility

For each trimming edge on a face:

\[
\sup_t
\|C_e(t)-S_f(q_{e,f}(t))\|
\le\varepsilon_{e,f}.
\]

A sampled-only detector must identify itself as sampled, not continuous.

**Rust type:** `CurveOnSurface` or `CurveSurfaceCompatibleEdgeUse`.

---

### GEO-006 — Projection validity

A surface projection returns:

```rust
struct Projection {
    uv: Point2,
    projected: Point3,
    residual: f64,
    stationarity_error: f64,
}
```

The result is an acceptable inverse only if its residual satisfies the declared tolerance policy.

A distant nearest point is not a valid inverse.

---

### GEO-007 — Transform provenance

Curves and surfaces retain the sequence of source placements and internal transforms applied to them.

The first stage at which an incidence relation stops holding must be observable.

---

## 15. Periodic and quotient-topology contracts

### QUO-001 — Reported period validity

For reported period \(P_u\):

\[
S(u+P_u,v)=S(u,v).
\]

Likewise for \(P_v\).

A reported period may be valid but nonfundamental; the distinction must be retained when relevant.

---

### QUO-002 — Quotient closure

Let \(\Delta q=\tilde\gamma(1)-\tilde\gamma(0)\) for lifted loop \(\tilde\gamma\).
The loop closes on the quotient when some lattice element accounts for the gap
up to a residual whose *physical* size is within tolerance:

\[
\exists\lambda\in\Lambda
\ \text{such that}\
\Delta q-\lambda=r,
\qquad
\|r\|_{G}=\sqrt{r^{\mathsf T}G(q_0)\,r}\le\varepsilon_{\mathrm{closure}}.
\]

Equivalently, as a minimisation over the lattice:

\[
\min_{\lambda\in\Lambda}
\sqrt{(\Delta q-\lambda)^{\mathsf T}G(q_0)(\Delta q-\lambda)}
\le\varepsilon_{\mathrm{closure}}.
\]

The metric \(G\) of §6 is required: a parameter-space gap is not a physical gap,
and the same \(\Delta q\) means different distances at different points of an
anisotropic chart.

The winding is the selected lattice element:

\[
\lambda=(k_uP_u,k_vP_v),
\qquad
h(\gamma)=(k_u,k_v)\in\mathbb Z^2,
\]

and is retained. A Boolean `closed` is insufficient.

**Singular charts.** Where \(G\) is rank-deficient — poles, apexes, collapsed
parameter boundaries — this certificate is unavailable, because a nonzero
\(r\) can have zero measured length. Closure must then be established in
physical space or by singular-case-specific semantics, and the certificate must
record which was used. See QUO-005.

**Cost:** summary numerical tier. \(O(1)\) per loop given \(\Lambda\); retain
\((k_u,k_v)\) and the achieved residual.

---

*Earlier drafts wrote this as \(\tilde\gamma(1)-\tilde\gamma(0)\in\Lambda+r\),
which is not well formed: \(\Lambda+r\) is a translate of the lattice, not a
tolerance neighbourhood, and it named no metric.*

---

### QUO-003 — Winding stability under refinement

Changing tessellation tolerance may change sample count, but it must not change:

- winding;
- contractibility;
- loop count;
- material component count;
- empty/nonempty status.

---

### QUO-004 — Relative deck consistency

Independently lifted bounds receive integer deck offsets \(n_i\). Shared topology produces equations:

\[
n_j-n_i=c_{ij}.
\]

Every constraint cycle must satisfy:

\[
\sum_{\mathrm{cycle}}c_{ij}=0.
\]

The solver returns one of:

```rust
enum DeckSolution {
    Unique(RelativeDeckAssignment),
    Multiple(Vec<RelativeDeckAssignment>),
    Underdetermined,
    Contradictory(DeckContradiction),
}
```

---

### QUO-005 — Singular-chart handling

Rank-deficient surface points are not processed as ordinary regular UV points.

The implementation must distinguish:

- periodic seams;
- collapsed parameter boundaries;
- poles;
- cone apexes;
- regular quotient loops.

---

## 16. Domain-semantics contracts

### DOM-001 — Preserve STEP bound syntax

Retain the exact source distinction:

```rust
enum StepBoundKind {
    DesignatedOuter,
    Bound,
}
```

Do not erase it during import.

---

### DOM-002 — Explicit resolved material roles

Resolved roles are distinct from source syntax:

```rust
enum MaterialBoundRole {
    Outer,
    Inner,
}

enum RoleKnowledge {
    Known(MaterialBoundRole),
    Unknown(UnknownRoleReason),
}
```

---

### DOM-003 — Explicit base domain

```rust
enum BaseDomain {
    Empty,
    NaturalParameterDomain,
    PeriodicQuotient,
}

enum BaseDomainKnowledge {
    Known(BaseDomain),
    Unknown(UnknownDomainReason),
}
```

Unknown semantics must not be silently replaced by a signed-area or loop-count guess.

---

### DOM-004 — Orientation-invariant classification

Under a chart reflection or any orientation-reversing reparameterization \(\phi\), the physical material region remains invariant:

\[
S(M)=
(S\circ\phi^{-1})(\phi(M)).
\]

Signed area may be retained as a diagnostic or normalization quantity, but not as an absolute material-side predicate unless a complete handedness convention is established upstream.

---

### DOM-005 — Base-plus-parity semantics

When parity semantics apply:

\[
\chi_M(p)=
\chi_{\mathrm{base}}(p)
\oplus
\operatorname{crossingParity}_{\Gamma}(p).
\]

The classifier requires known base occupancy.

---

## 17. Boundary-arrangement contracts

### ARR-001 — Geometric loop closure

After periodic reduction and local metric evaluation, loop endpoints close within declared tolerance.

This is distinct from topological and quotient closure.

---

### ARR-002 — Intersection completeness

Every proper intersection of trim segments becomes an arrangement vertex.

---

### ARR-003 — No unresolved overlap

Collinear overlap, duplicate segments, or coincident boundaries are either normalized explicitly or rejected.

---

### ARR-004 — Cell-label consistency

For adjacent cells \(c_i,c_j\):

\[
L(c_j)=L(c_i)\oplus b_e,
\]

where \(b_e=1\) when crossing a trim boundary and \(0\) otherwise.

Every cycle in the dual graph must produce consistent labels.

---

### ARR-005 — Containment consistency

For simple contractible loops, the containment relation must be acyclic and agree with resolved domain semantics.

---

## 18. Constrained-triangulation contracts

### CDT-001 — Constraint provenance

Every requested trim segment receives a stable `ConstraintId`.

---

### CDT-002 — Complete constrained representation

For every requested constraint \(\sigma_i\), the final triangulation contains a complete constrained-edge chain \(\widehat\sigma_i\) whose union equals the input segment within tolerance:

\[
\sigma_i=
\bigcup_{e\in\widehat\sigma_i}e.
\]

A successful insertion API call is not sufficient evidence.

---

### CDT-003 — No trim crossing

For each triangle interior \(\tau^\circ\):

\[
\tau^\circ\cap\Sigma=\varnothing.
\]

---

### CDT-004 — Valid region labeling

Flood-fill or dual-graph labeling is permitted only after `CDT-002` is certified.

---

### CDT-005 — Domain coverage

The exact material region is defined by the **continuous** trim semantics, independently of anything the triangulator computes:

\[
M=\{q\in Q:\ \chi_{\mathrm{base}}(q)\oplus\operatorname{parity}_\Gamma(q)=1\}.
\]

The discretised arrangement defines \(M_h\), and the retained triangles define \(T_h\). The coverage error then decomposes by the triangle inequality for symmetric difference:

\[
\mu(M\triangle T_h)
\le
\mu(M\triangle M_h)
+
\mu(M_h\triangle T_h).
\]

The two terms are **different obligations belonging to different stages**, and conflating them is what made an earlier draft circular — it measured the triangulation against a region the triangulation itself had defined:

- \(\mu(M\triangle M_h)\) — **trim-curve approximation error.** How well the discretised boundary represents the true trim curves. Owned by sampling and arrangement (ARR-001, GEO-005).
- \(\mu(M_h\triangle T_h)\) — **arrangement and triangulation conformance.** For a conforming triangulation with correct labelling this is \(0\), up to exact-predicate or floating-point representation assumptions. Owned by CDT-002 through CDT-004.

A certificate must say which term it bounds. A bound on the second alone says nothing about fidelity to the source geometry.

**Cost:** the second term is structural given CDT-002; the first requires curve-region integration and is a CI or strict-mode certificate.

---

## 19. Face-mesh contracts

### MSH-001 — Vertex adherence

For every mesh vertex carrying UV coordinates:

\[
\|X_i-S(u_i,v_i)\|
\le\varepsilon_{\mathrm{eval}}.
\]

---

### MSH-002 — Declared approximation level

A mesh certificate must state whether it establishes:

- sampled error only;
- derivative-based local bound;
- adaptive continuous bound;
- heuristic quality status.

Do not call a sampled centroid check a continuous proof.

---

### MSH-003 — Orientation agreement

At regular triangle centroid \(q_c\):

\[
\operatorname{sign}
\left[
 n_\tau\cdot(S_u(q_c)\times S_v(q_c))
\right]
\]

must agree with effective face orientation.

---

### MSH-004 — No unintended repeated physical coverage

Distinct UV triangles must not map repeatedly onto the same ordinary physical patch except where seams or declared covering maps require it.

---

### MSH-005 — Diagnostic anomaly metrics

Bounding-box ratios, extreme edge lengths, triangle-count spikes, and area ratios are diagnostics only. They rank suspicious faces but do not certify correctness.

---

## 20. Shell contracts

### SHL-001 — Shared-edge mesh agreement

For adjacent faces using source edge \(e\), their mesh boundary traces coincide geometrically and, where required, combinatorially.

---

### SHL-002 — Mesh incidence agreement

The output mesh incidence must agree with source shell expectations:

```rust
enum ShellTopologyExpectation {
    Closed,
    Open,
    NonManifoldAllowed,
}
```

A source open shell must not be rejected merely for having boundary mesh edges.

---

### SHL-003 — Orientation consistency

Adjacent face orientations induce opposite traversal along shared physical boundaries.

---

### SHL-004 — Topological invariants

Where source topology is trustworthy, compare:

- connected components;
- boundary components;
- orientability;
- Euler characteristic;
- expected closed/open status.

---

### SHL-005 — Optional strict self-intersection check

Triangle-triangle self-intersection is an expensive strict-mode or CI certificate, not necessarily an always-on preview check.

---

## 20a. Resource contracts

A geometry kernel can be mathematically correct on ideal inputs and still be
unusable or unsafe if imported values induce unbounded allocation or work. This
family sits beside topology and geometry, not in an appendix.

The motivating incident: `RevolutedCurve::parameter_division` derived an angular
sample count as `1 + ((vrange.1 - vrange.0) / acos(1.0 - tol / max)).floor() as
usize` with no ceiling. `acos` collapses toward zero as the revolved radius grows
against the tolerance, and `vrange` is the bounding box of a lifted boundary,
which a bad lift can make span many periods. On ABC `00000730` this requested
6,638,692,106,004,871,184 bytes and aborted the process — losing the whole model
rather than one face. No `TOP`, `GEO`, `QUO`, or `DOM` contract was violated.

### RES-001 — Bounded derived counts

Every count derived from imported geometry passes through a checked constructor:

```rust
pub struct SampleCount(NonZeroUsize);

pub enum SampleCountOutcome {
    WithinBudget(SampleCount),
    BudgetExceeded { requested: EstimatedCount, maximum: usize },
    InvalidInput { reason: InvalidCountReason },
}
```

Forbidden:

```rust
let count = expression_from_geometry as usize;
Vec::with_capacity(count);
```

**Audit signature:** a float-to-integer cast whose operand depends on imported
values, followed by an allocation.

---

### RES-002 — Face-local failure containment

One face exhausting a time or memory budget may not abort the model:

\[
\mathrm{failure}(f_i)\nRightarrow\mathrm{failure}(H).
\]

It yields `FaceMeshingOutcome::Uncertified { reason: BudgetExceeded { .. } }`.

---

### RES-003 — Budget exhaustion is not certification

If an algorithm requests \(N\) subdivisions to satisfy a tolerance but is capped
at \(B<N\), the result **may not claim the requested tolerance**:

```rust
enum SubdivisionCertificate {
    WithinTolerance { divisions: usize, error_bound: f64 },
    ResourceCapped {
        requested: EstimatedCount,
        used: usize,
        achieved_error: Option<f64>,
    },
}
```

This applies directly to the `MAX_CIRCLE_DIVISION` and `MAX_DIVISION_CELLS`
caps. **A cap makes the program safe; it does not make the returned
approximation correct.** Both currently return capped results as ordinary
success, which is a live violation of this contract.

---

### RES-004 — Finite arithmetic before resource derivation

Every geometry-derived count requires, in order: finite inputs; finite
intermediate values; a nonnegative span; checked conversion to integer; checked
multiplication and addition; and an explicit upper bound.

Degenerate inputs are part of the contract, not an edge case — a zero radius
makes `tol / max` infinite and `acos` NaN, and an unchecked cast turns that into
a division by zero.

---

### RES-005 — Termination or budgeted interruption

Adaptive algorithms must either terminate under stated assumptions, or carry a
maximum depth, work count, or time budget and return an explicit incomplete
result. Silent non-termination and silent truncation are both prohibited.

---

### RES-006 — Complexity witness

Each substantial stage records enough to explain its resource use:

```rust
struct WorkCertificate {
    samples: usize,
    subdivisions: usize,
    intersections: usize,
    triangles: usize,
    capped: bool,
}
```

`capped` is the field that connects a resource event to the certificate that
must be downgraded under RES-003.

---

# Part IV — Rust Enforcement Architecture

## 21. Typed identities

Use different newtypes for source identities and arena positions:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepEdgeCurveId(u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepFaceId(u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StepVertexId(u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeIndex(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceIndex(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexIndex(usize);
```

Tuple fields should be private outside their modules. Do not expose conversions from arbitrary `usize` except through checked arena APIs.

## 22. Transactional arenas

**One generic arena, used for every compacted entity type.** Vertices, edges,
surfaces, and faces get the same implementation parameterised by kind — not four
similar loops that each require auditing. See §51a: the reserve-before-convert
defect was repaired in the edge path and survived untouched in the vertex path
for exactly as long as the repair was site-local. A contract is discharged when
the invalid transition has *one* implementation and that implementation cannot
express the bad state.

Every arena item stores its own source identity:

```rust
pub struct StoredEdge {
    source_id: StepEdgeCurveId,
    geometry: StoredEdgeGeometry,   // vertices + curve
}

pub struct EdgeArena {
    items: Vec<StoredEdge>,
    by_source: HashMap<StepEdgeCurveId, EdgeIndex>,
}
```

Storing `source_id` is **not** mathematically necessary to maintain the lookup
invariant if the arena is correct. It is necessary for everything around that:
independently checking the invariant rather than trusting it, producing a
failure report that names the entity, retaining provenance once the source map
is out of scope, detecting corruption at the point of use, and supporting PR 4.
Structural correctness and retained evidence are separate requirements, and only
the first is free.

### 22.1 Canonical resolution, not unconditional insertion

A repeated reference to the same STEP entity is **normal** — a shell names the
same `EDGE_CURVE` from both faces that share it. Treating the second reference
as a duplicate-insertion error is a specification bug, and an earlier draft of
this document had it. The public operation resolves rather than inserts:

```rust
pub fn get_or_try_insert(
    &mut self,
    source_id: StepEdgeCurveId,
    convert: impl FnOnce() -> Result<StoredEdgeGeometry, ConversionError>,
) -> Result<EdgeIndex, ArenaError>
```

```rust
match self.by_source.entry(source_id) {
    Occupied(entry) => Ok(*entry.get()),
    Vacant(entry) => {
        let converted = convert()?;
        let index = EdgeIndex(self.items.len());
        self.items.push(StoredEdge { source_id, geometry: converted });
        entry.insert(index);
        Ok(index)
    }
}
```

`DuplicateSource` means an actual internal attempt to create a second canonical
object for one identity. It never means an ordinary repeated reference.

### 22.2 Checked resolution at the point of use

Because the identity is retained, every lookup can assert it cheaply:

```rust
let stored = arena.get(index)?;
if stored.source_id != requested_id {
    return Err(IdentityError::Mismatch {
        requested: requested_id,
        stored: stored.source_id,
        index,
    });
}
```

This is the always-on structural tier of §45, and it is what makes the failure
report in §61 printable.

The reserve-before-convert defect becomes structurally unavailable.

## 23. Fail-whole-bound conversion

Never use `filter_map` where missing input changes topology.

```rust
let uses: Vec<ResolvedEdgeUse> = source_wire
    .edge_uses()
    .map(|edge_use| resolve_edge_use(edge_use, context))
    .collect::<Result<_, _>>()?;
```

A failed use invalidates the bound or enters an explicit healing path.

## 24. Separate closure types

Three different propositions, three different types. They are not
interchangeable and no one of them implies another.

```rust
/// TOP-004. Proves only endVertex(e_i) == startVertex(e_{i+1}), cyclically:
/// closure by vertex *identity*. It does NOT prove
/// ‖C_{e_i}(b_i) − C_{e_{i+1}}(a_{i+1})‖ ≤ ε, and it does not prove closure
/// modulo a period lattice.
pub struct TopologicallyClosedWire {
    uses: NonEmptyVec<CompressedEdgeIndex>,
}

/// ARR-001. Endpoints meet within tolerance under the surface metric.
pub struct GeometricallyClosedWire {
    wire: TopologicallyClosedWire,
    closure: WithinTolerance,
}

/// QUO-002. Closure modulo Λ, with the winding retained.
pub struct QuotientClosedLoop {
    lifted: NonEmptyVec<Point2>,
    winding: Winding2,
    closure: WithinTolerance,
}
```

**Never name a type `ClosedWire`.** The unqualified name asserts all three
propositions while a constructor can only establish one, and it invites exactly
the inference this architecture exists to prevent: reading vertex-identity
closure as metric closure, or as quotient closure, at a stage that needs the
stronger fact. The name must say which closure it proves.

> **Implementation status, 2026-07-29:** done. `truck-stepio/src/in/wire.rs`
> defines `TopologicallyClosedWire`, which establishes the topological predicate
> only and says so in its own doc comment. `GeometricallyClosedWire` and
> `QuotientClosedLoop` do not exist yet; nothing currently claims either.

## 25. Provenance through sampling

Anonymous polylines are not sufficient at certification boundaries. But
"retain provenance" does **not** mean "copy the identity into every point", and
an earlier draft of this section implied that it did:

```rust
// Wrong. Repeated for millions of samples on a real model.
pub struct EdgeSample {
    source_edge_id: StepEdgeCurveId,
    edge_index: EdgeIndex,
    parameter: f64,
    point: Point3,
}
```

Identity is a property of the *edge*, so it is stored once per sampled edge:

```rust
pub struct SampledEdge {
    source_edge_id: StepEdgeCurveId,
    edge_index: EdgeIndex,
    parameters: ParameterSamples,
    points: Vec<Point3>,
    certificate: SamplingFidelityCertificate,
}

pub enum ParameterSamples {
    Explicit(Vec<f64>),
    Uniform { start: f64, step: f64, count: usize },
    Adaptive(Vec<f64>),
}
```

Uniform sampling — the common case — stores three numbers instead of a vector.

Boundary provenance is likewise stored per *span*, not per vertex:

```rust
pub struct BoundarySpan {
    sampled_edge: SampledEdgeIndex,
    sample_range: Range<u32>,
    orientation: Orientation,
}
```

That gives exact provenance at roughly 16–32 bytes per edge span, against 32
extra bytes per vertex for the naive encoding. The semantics are identical; only
the representation differs.

**The architecture must optimise proof representation, not merely add
metadata.** A certificate scheme that doubles peak memory on a 1.9M-triangle
model has failed a product whose reason for existing is time to a usable image.

## 26. Curve-on-surface type

```rust
pub struct CurveSurfaceCertificate {
    source_edge_id: StepEdgeCurveId,
    source_face_id: StepFaceId,
    max_residual: f64,
    permitted_residual: f64,
    worst_sample: usize,
    method: CompatibilityCertificationMethod,
}

pub struct CurveOnSurface {
    edge: SampledEdge,
    face: ResolvedFaceRef,
    certificate: CurveSurfaceCertificate,
}
```

Only this type may enter UV lifting.

## 27. Knowledge and ambiguity types

Unknown is a first-class result:

```rust
pub enum Knowledge<T, R> {
    Known(T),
    Unknown(R),
}
```

Do not provide convenience conversions that guess a known value.

### 27.1 What `Unknown` means downstream

`Unknown` cannot simply halt the kernel, because `look` is still a renderer and
a blank image is not a better answer than an imperfect one. The resolution is
**not** to convert `Unknown` into a guessed `Known`. It is to make the fallback
itself a named, observable operation that travels with the output:

```rust
pub enum FaceResolution {
    Certified(DomainResolvedFace),
    Unknown {
        face: QuotientResolvedFace,
        reason: UnknownDomainReason,
    },
}

pub enum FaceMeshingOutcome {
    Certified(CertifiedFaceMesh),
    Uncertified {
        mesh: RawFaceMesh,
        assumptions: Vec<ExplicitAssumption>,
        failures: Vec<ContractFailure>,
    },
    Rejected(FaceRejection),
}
```

An assumption is recorded in the form:

```text
Uncertified render assumption:
DOM-003 unresolved.
Fallback used: odd-even parity with empty base.
```

The renderer may draw such a face, tint it, warn about it, or include it
silently in ordinary PNG output — that is product policy per §31. What the
kernel may **not** do is present it as semantically resolved. The guess becomes
a recorded assumption attached to the mesh rather than an invisible default
buried in a classifier, and inspection output can always enumerate them.

This is what keeps preview rendering useful without letting `Unknown` decay back
into a silent guess.

## 28. Proof-bearing triangulation

```rust
pub struct ConstraintCertificate {
    chains: HashMap<ConstraintId, NonEmptyVec<TriangulationEdgeId>>,
}

pub struct ConformingCdt {
    triangulation: Cdt,
    constraints: ConstraintCertificate,
    labels: CellLabelCertificate,
}
```

Fields are private. The only constructor verifies all required obligations.

## 29. Certified and diagnostic mesh outcomes

```rust
pub struct RawFaceMesh {
    // Available for diagnostics and explicit fallback policies.
}

pub struct CertifiedFaceMesh {
    mesh: RawFaceMesh,
    certificate: FaceMeshCertificate,
}

pub enum FaceMeshingOutcome {
    Certified(CertifiedFaceMesh),
    Rejected(FaceRejection),
    Uncertified {
        mesh: RawFaceMesh,
        failures: Vec<CertificateFailure>,
    },
}
```

The renderer may choose fallback behavior. CAD-kernel operations may require only certified outputs.

## 29a. Cost is part of every contract

`look` exists for time to a usable image. An obligation with no stated cost is
not specified, and "generally cheap" is an assertion, not a measurement.

**Every contract entry in Part III must state:**

| Field | Meaning |
|---|---|
| Expected time complexity | in the size of the face, shell, or model |
| Expected auxiliary memory | transient working set |
| Persistent memory overhead | what survives into the retained certificate |
| Runtime tier | see the table below |
| Escalation policy | what triggers a more expensive check |
| Release-mode witnesses | whether full witnesses are retained, or only summaries |

### Runtime tiers

| Tier | Examples | Deployment |
|---|---|---|
| Structural | identity, cardinality, incidence, canonical dedup | always |
| Summary numerical | max residual, worst parameter, winding, achieved error | always, or preview |
| Full provenance | all sampled parameters and residuals | debug / strict |
| Global diagnostics | nearest-point searches, self-intersection, transform fitting | on failure |
| Metamorphic | seam shifts, chart reflections, tolerance sweeps | CI |

A detector that is correct but too expensive for its tier is not shippable in
that tier, and the honest response is to move it, not to weaken it. Summary
witnesses are the default in release: a max residual and the parameter at which
it occurred localise a defect nearly as well as the full sample vector, at
constant size.

## 30. Prevent stale certificates

Preferred strategies:

1. immutable geometry;
2. consuming typestate transitions;
3. revision-bound witnesses when mutation is unavoidable.

```rust
pub struct GeometryRevision(u64);
```

A certificate must identify the geometry revision it certifies.

## 31. Policy is separate from detection

A detector establishes a fact. Product policy decides what to do.

```rust
pub enum InvalidGeometryPolicy {
    ReportOnly,
    RejectFace,
    AttemptRepair,
    EmitUncertifiedDiagnosticMesh,
}
```

This prevents a correct residual detector from automatically becoming a destructive renderer policy.

---

## 31a. Upstream boundary: where these types are allowed to live

`truck-stepio`, `truck-geometry`, and `truck-meshalgo` are forks of an upstream
that publishes roughly every two years. Which architecture is being built
changes the design, so it is recorded here rather than assumed. Two options were
available; the second was chosen.

### Upstream-oriented architecture

Keep truck's public structures broadly stable and add checked wrappers, optional
provenance sidecars, fallible constructors, narrow newtypes, and minimal
API-breaking change:

```rust
struct CertifiedCompressedFace {
    face: CompressedFace,
    certificates: FaceCertificates,
}
```

Easier to upstream. But every hand-back into an existing truck type **erases the
proof**, so guarantees hold only inside the wrapper layer.

### Owned-fork architecture

Change the core model so the guarantees are load-bearing everywhere:

```rust
struct CompressedFace {
    surface: SurfaceIndex,
    boundaries: Vec<TopologicallyClosedWire>,
    source_id: StepFaceId,
    semantics: DomainSemantics,
}
```

Strictly stronger, and effectively a new kernel architecture — a permanent fork.
**This is the chosen architecture.**

### DECIDED 2026-07-29: owned fork

**`stefangolas/truck` is a permanent architectural fork.** The core model
changes; guarantees are load-bearing across the whole pipeline; upstream
compatibility is not a design constraint.

What this settles:

- **Proofs may not be erased at truck boundaries.** The recurring problem with
  the wrapper approach — that handing data back into a bare truck type discards
  the certificate — does not apply. `CompressedFace::boundaries` becomes
  `Vec<TopologicallyClosedWire>`, and the `position()` escape hatch in the arena
  exists to satisfy a signature that is now ours to change. Every remaining use
  of it is a defect to close, not a boundary to respect.
- **The typestate of Part V is reachable.** It requires changing types truck
  owns; that is now permitted.
- **Divergence is not a cost to be minimised.** Do not preserve an upstream
  signature, field layout, or naming convention for its own sake. Rebase burden
  against `ricosjp/truck` is accepted and is not a reason to weaken a contract.
- **Merges from upstream become selective and manual.** The fork should pull
  genuine upstream fixes deliberately, not track a branch.

What it does **not** settle: individually valuable, self-contained fixes may
still be offered upstream as a courtesy — the resource bounds of §20a are the
obvious candidates, since they are plain safety fixes with no architectural
commitment. That is contribution, not a compatibility obligation, and it must
never shape a design decision here.

**Immediate consequence:** the release blocker in `PLAN.md` — thirteen-plus
unpushed commits, a temporary `[patch]` block pointing at a local checkout,
nothing pinned — is now unambiguously "push the fork and pin the rev", with no
strategic question remaining behind it.

---

# Part V — Pipeline Typestate

## 32. Proposed state sequence

```rust
pub struct ImportedFace;
pub struct ResolvedFace;
pub struct TopologicallyValidFace;
pub struct CurveSurfaceCompatibleFace;
pub struct QuotientResolvedFace;
pub struct DomainResolvedFace;
pub struct ArrangedFace;
pub struct TriangulatedFace;
pub struct CertifiedFaceMesh;
```

Representative transitions:

```rust
fn resolve_face(
    face: ImportedFace,
    context: &ImportContext,
) -> Result<ResolvedFace, ResolveError>;

fn verify_topology(
    face: ResolvedFace,
) -> Result<TopologicallyValidFace, TopologyError>;

fn certify_curve_surface_compatibility(
    face: TopologicallyValidFace,
    tolerance: CompatibilityTolerance,
) -> Result<CurveSurfaceCompatibleFace, CompatibilityError>;

fn resolve_quotient_topology(
    face: CurveSurfaceCompatibleFace,
) -> Result<QuotientResolvedFace, QuotientError>;

fn resolve_domain(
    face: QuotientResolvedFace,
    semantics: ResolvedDomainSemantics,
) -> Result<DomainResolvedFace, DomainError>;

fn build_arrangement(
    face: DomainResolvedFace,
) -> Result<ArrangedFace, ArrangementError>;

fn triangulate_conformingly(
    face: ArrangedFace,
) -> Result<TriangulatedFace, TriangulationError>;

fn certify_face_mesh(
    face: TriangulatedFace,
    tolerance: MeshTolerance,
) -> Result<CertifiedFaceMesh, MeshCertificationError>;
```

Private fields ensure safe code cannot forge a later state.

---

# Part VI — Implementation Roadmap

## 33. PR 0 — Mathematical specification and contract registry

Deliver this specification in the repository and establish:

- definitions;
- obligation IDs;
- exact versus numerical versus heuristic claims;
- mapping from obligations to Rust types;
- stage preconditions and postconditions;
- explicit ambiguity states;
- failure-report format.

Every later PR references the obligations it discharges.

## 33a. Immediate sequence

Ordered. Items 1–4 close the gap between what landed and what this document
requires; the rest close the gaps this document had.

1. ~~Rename `ClosedWire` to `TopologicallyClosedWire` (§24).~~ **Done
   2026-07-29.**
2. ~~Store source identity in every arena item (TOP-001, §22).~~ **Done
   2026-07-29**, with `Arena::get_checked` and an `IdentityMismatch` that prints
   in the §61 form.
3. ~~Change arena semantics to `get_or_try_insert` with canonical dedup
   (TOP-007, §22.1).~~ **Done 2026-07-29.** The behaviour was already right;
   the name and the doc now say so.
4. Generalise the arena across **all** entity types so the
   reserve-before-convert class is eliminated rather than repaired (§51a).
   **Partly done 2026-07-29: surfaces resolve through the same arena; faces do
   not and cannot yet.** A face has no compacted identity to resolve — there is
   no face id → index map, because `CompressedFace` is built inline and owns its
   surface by value. So the reserve-before-convert *class* is now empty in the
   sense that every map/vector pair in the converter is an `Arena`, but the
   stronger claim in §51a — one implementation for every entity kind — is not
   reachable for faces until item 11 gives `CompressedFace` a `source_id` and a
   `SurfaceIndex`. Recording that distinction rather than ticking the item:
   repair count is not coverage, and neither is arena count.
5. Add RES-001 through RES-006 to the implementation.
6. Change capped subdivision from ordinary success to `ResourceCapped`
   (RES-003) — `MAX_CIRCLE_DIVISION` and `MAX_DIVISION_CELLS` both violate this
   today.
7. Add cost fields to every contract entry (§29a).
8. Correct QUO-002 and decompose CDT-005 in the implementation, not only here.
9. Add explicit downstream semantics for `Unknown` (§27.1).
10. Add corpus, performance, and diagnostic acceptance criteria (§60a–§60c).
11. Act on the owned-fork decision (§31a): change `CompressedFace::boundaries`
    to `Vec<TopologicallyClosedWire>` and close the `position()` escape hatches
    that only existed to satisfy upstream signatures.
12. **Start citing contract IDs** in code comments, error types, tests, and PR
    descriptions. Nothing landed so far does this.

## 34. PR 1 — Compatibility measurement and policy separation

Current direction:

- retain curve-surface residual measurement;
- default to report-only or off in production rendering;
- separate detector result from rejection policy;
- retain residual ratio and worst sample;
- do not market it as a blob fix.

This detector found a real population of incompatible faces but did not repair the measured blobs. That is evidence that detection and rendering policy must remain separate.

**Contracts:** GEO-005, GEO-006 partially.

## 35. PR 2 — Typed identities and transactional arenas

Add:

- typed source IDs;
- typed arena indices;
- private constructors;
- source identity stored in every arena entry;
- atomic convert-push-map insertion;
- canonical resolution with dedup (`get_or_try_insert`);
- one generic arena shared by every entity kind;
- invariant tests under arbitrary success/failure sequences.

**Contracts:** TOP-001, TOP-002, TOP-007.

> **Landed 2026-07-29.** `truck-stepio/src/in/arena.rs` has typed IDs, private
> index constructors, and transactional insertion; the vertex path was ported to
> it, which is how the surviving reserve-before-convert defect was found.
>
> Every item is now a `Stored { source_id, value }`, so TOP-001 is checkable
> rather than merely true: `Arena::get_checked` compares the stored identity
> against the one the caller named and returns an `IdentityMismatch` that prints
> in the §61 form. It is called on every edge reference a face bound resolves.
> `try_insert` is renamed `get_or_try_insert` and its doc states the canonical
> resolution semantics of §22.1. Surfaces are the third kind to use the arena.
>
> **Outstanding:** faces are still not compacted by identity at all — they are
> built inline in `shell_faces` and `CompressedFace` carries no `source_id` — so
> TOP-001 for faces is not merely unchecked, it is unaskable. That waits on
> §33a item 11 and PR 4. `into_items` remains a documented loss of provenance at
> the `CompressedShell` boundary.

## 36. PR 3 — Fail-whole-bound conversion

Replace topology-changing `filter_map` paths with fallible collection.

Add topological wire cardinality and closure checks.

**Contracts:** TOP-003, TOP-004, TOP-005.

> **Landed**, and renamed to `TopologicallyClosedWire` on 2026-07-29 (§24).
> Bound conversion and face-bound collection are both all-or-nothing;
> cardinality is discharged by construction. TOP-005 is **not** addressed —
> effective orientation is composed but never checked against source incidence.

## 37. PR 4 — Provenance through edge use, sampling, and compatibility

Carry source identity through:

```text
ORIENTED_EDGE
→ resolved edge use
→ compressed edge
→ sampled edge
→ boundary piece
→ curve-on-surface certificate
```

Check:

- expected source ID equals stored source ID;
- sampled points evaluate from the stored curve;
- boundary pieces contain only declared contributors;
- edge lies on the supporting surface within the declared certification level.

**Contracts:** GEO-002 through GEO-007.

This PR should sharply localize the current open defect.

## 38. PR 5 — Explicit domain semantics

Preserve source STEP bound syntax.

Represent resolved material role and base domain separately, each with known/unknown state.

Remove signed-area-based absolute material semantics.

Add chart-reflection and equivalent wire-reversal invariance tests.

**Contracts:** DOM-001 through DOM-005.

## 39. PR 6 — Quotient topology, winding, deck consistency, singular cases

Recommended split:

- **PR 6a:** period metadata, period validity, quotient closure, winding;
- **PR 6b:** relative deck solver with exact integer cycle checks;
- **PR 6c:** poles, apexes, collapsed boundaries, and seam-specific topology.

**Contracts:** QUO-001 through QUO-005.

## 40. PR 7 — Valid arrangements and conforming CDT

Recommended internal order:

```text
validated boundary arrangement
→ split intersections and reject unresolved overlaps
→ insert constraints with provenance
→ verify complete constrained chains
→ label cells in the dual graph
→ retain material triangles
```

Flood fill is not sound before constraint completeness is certified.

**Contracts:** ARR-001 through ARR-005 and CDT-001 through CDT-005.

## 41. PR 8 — Certified face and shell outcomes

Add:

- raw diagnostic meshes;
- certified face meshes;
- explicit rejection outcomes;
- source-topology-aware shell certification;
- shared-edge agreement;
- orientation and incidence checks;
- declared approximation-certificate levels.

**Contracts:** MSH-001 through MSH-005 and SHL-001 through SHL-005.

## 42. PR 9 — Property, metamorphic, numerical round-trip, and corpus harness

Four test families:

### Structural property tests

- arbitrary sequences of successful and failed arena insertions;
- map/arena consistency;
- no dropped wire uses;
- integer deck-cycle consistency;
- constraint-chain accounting.

### Numerical round-trip properties

- generate \(P=S(u,v)\), invert, and verify reconstruction;
- sample \(P=C(t)\), retain parameter, and verify evaluation;
- verify analytic primitive conversion invariants.

### Metamorphic geometry tests

- reflect the UV chart;
- shift a periodic seam;
- reverse wire and orientation flags together;
- rotate loop starting position;
- apply rigid transforms;
- uniformly scale geometry and tolerances;
- refine tessellation tolerance.

### Corpus regression tests

- isolated reproducers;
- ABC and NIST STEP corpora;
- differential comparison against another mature kernel where useful;
- explicit expected failure certificates, not only golden images.

---

# Part VII — Code Review Program

## 43. Audit targets

Search the codebase for:

```text
bare usize used as entity/index identity
parallel Vec + HashMap storage
map.len() used as pushed-vector index
mapping inserted before fallible conversion
filter_map in topology construction
? inside filter closures
Option-returning conversion that erases error cause
anonymous PolylineCurve across subsystem boundaries
source identity discarded after lookup
nearest-point APIs without residuals
duplicated AXIS2_PLACEMENT_3D conversion logic
signed area used as outer/hole semantics
loop count used as base-domain semantics
unchecked CDT constraint insertion
global epsilon constants
surface/curve transforms with no provenance
silent fallback from exact p-curve to arbitrary nearest projection
```

## 44. Stage-boundary review table

Every stage must document:

| Stage | Input | Required precondition | Output | Guaranteed postcondition | Failure type |
|---|---|---|---|---|---|
| Edge conversion | STEP edge | references resolve | stored edge | source identity retained | `EdgeConversionError` |
| Bound resolution | source wire | every use resolves | topological wire | cardinality and incidence preserved | `BoundResolutionError` |
| Sampling | stored edge | valid parameter range | sampled edge | samples come from same edge | `SamplingError` |
| Compatibility | sampled edge + face | geometry valid | curve-on-surface | residual certificate | `CompatibilityError` |
| UV lifting | curve-on-surface | periods valid | quotient loop | winding and closure retained | `LiftError` |
| Domain construction | loops + semantics | base known | material domain | chart-invariant labels | `DomainError` |
| Arrangement | material domain | valid loops | arrangement | intersections represented | `ArrangementError` |
| CDT | arrangement | constraints valid | conforming CDT | every constraint represented | `CdtError` |
| Meshing | conforming CDT | surface evaluable | face mesh | approximation certificate | `MeshError` |
| Shell assembly | certified faces | source adjacency known | shell outcome | incidence agreement | `ShellError` |

---

# Part VIII — Testing and Runtime Strategy

## 45. Always-on checks

Generally cheap:

- transactional arena invariants;
- source identity equality;
- wire cardinality;
- topological closure;
- sample fidelity;
- projection residual retention;
- quotient winding and closure;
- constraint insertion accounting;
- degenerate triangle checks;
- shared canonical edge identity;
- subdivision budget status.

## 46. Suspicious-face diagnostics

Triggered after a certificate fails:

- global nearest-point verification;
- residual-vector analysis;
- rigid or similarity transform fitting;
- alternate edge-surface association matrix;
- exact analytic incidence comparison;
- minimum boundary separation;
- detailed arrangement overlay;
- mesh self-intersection.

These diagnose cause. They should not be conflated with the first-line detector.

## 47. CI-only or strict-mode checks

Potentially expensive:

- multiple tolerance runs;
- seam shifts;
- chart reflections;
- full shell self-intersection;
- accurate physical-area integration;
- differential kernel comparison;
- model checking of finite structural cores.

## 48. Formal verification candidates

Good targets for Kani, Verus, Creusot, or equivalent tools:

- arena insertion consistency;
- no out-of-range compact references;
- no mapping for failed conversion;
- orientation algebra;
- wire-use cardinality preservation;
- integer deck-potential solver;
- dual-graph parity consistency;
- constraint provenance accounting;
- mesh incidence counting.

Do not begin by attempting end-to-end verification of NURBS evaluation or nonlinear inverse projection.

---

# Part IX — Current Bug Lessons and Contract Mapping

## 49. Signed-area domain semantics

**Observed failure:** material complement or empty region depending on chart handedness.

**Root property:** signed area changes under orientation-reversing chart transformations.

**Contracts:** DOM-001 through DOM-005.

**Preventive architecture:** preserve source bound facts; require explicit base domain; use chart-invariant classification.

## 50. Periodic lifting and deck copies

**Observed failure:** smooth blobs and large sheets on valid periodic support surfaces.

**Root property:** local UV continuity does not establish globally coherent quotient topology.

**Contracts:** QUO-001 through QUO-005.

**Preventive architecture:** retain winding; solve relative integer deck offsets; separate singular cases.

## 51. Reserve-before-convert `eidx_map` defect

**Observed failure:** after one failed conversion, every later face silently received a neighboring edge curve.

**Root property:** source map and pushed vector could diverge.

**Contracts:** TOP-001 and TOP-002.

**Preventive architecture:** typed IDs and transactional arena insertion.

## 51a. The same defect survived in a second call site

The `eidx_map` defect was repaired in edge conversion. The **identical**
reserve-before-convert pattern sat untouched in `shell_vertices` — insert the
position into the map, then call a fallible `get_owned` — and was found only
when the generic arena was written and the vertex path had to be ported to it.

**Root property:** the original fix was local, not architectural.

A contract is not discharged because every currently known call site has been
manually repaired. It is discharged when the invalid transition has **one**
implementation and that implementation **cannot express the bad state**.

**Preventive architecture:** one generic arena used for vertices, edges,
surfaces, and every other compacted entity — not four similar loops that each
require auditing. See §22.

This is also the argument against treating any per-site fix as evidence that a
contract family is satisfied. Repair count is not coverage.

## 51b. Unbounded derived sample count

**Observed failure:** process abort on a 6.6-exabyte allocation, losing an
entire model rather than one face.

**Root property:** a count derived from imported geometry with no ceiling,
reached through a float-to-integer cast, in a specialised code path that
bypassed the cap its generic sibling already had.

**Contracts:** RES-001, RES-003, RES-004.

**Preventive architecture:** checked count constructors; face-local containment;
capped results downgraded to `ResourceCapped` rather than reported as success.

**Second lesson, on validation layers.** This abort was invisible while the
curve-surface residual gate was enabled, because the gate happened to reject the
offending faces before they reached tessellation. Turning the gate off — the
correct decision on its own merits — looked like introducing a crash. A
validation layer that silently prevents a downstream failure makes the system
appear sounder than it is and makes its removal appear to be a regression. This
is an argument for fixing causes rather than accumulating gates, and for
recording what each gate is actually load-bearing for.

## 52. Distant nearest-point projection treated as inverse

**Observed failure:** invalid edge-surface pairing produced a smooth UV path and plausible blob rather than an error.

**Root property:** projection API returned a value without required validity evidence.

**Contracts:** GEO-005 and GEO-006.

**Preventive architecture:** projection residual is mandatory; only certified projections enter lifting.

## 53. Current open blob class

At this snapshot, remaining blob reproducers are not fully explained. The next useful instrumentation is transform and provenance tracing that identifies the first stage where:

\[
d(C(t),S)\le\varepsilon
\]

stops holding, or where the blob is created despite that relation continuing to hold.

Future agents must not assume all blobs share one cause. Existing measurements already separated multiple defect populations.

---

# Part X — Agent Operating Protocol

## 54. Before changing code

1. Identify the first contract whose failure could explain the symptom.
2. Instrument that contract directly rather than inferring from final mesh shape.
3. Audit the detector’s mathematics and implementation.
4. State whether the output is an exact proof, numerical certificate, or heuristic diagnostic.
5. Preserve source identity and tolerance provenance in logs.

## 55. During investigation

Do not interpret downstream topology when an upstream obligation has failed.

Examples:

- winding is not meaningful when curve-surface compatibility fails;
- flood-fill labels are not meaningful when constraints are missing;
- outer/hole classification is not meaningful when base-domain semantics are unknown;
- shell watertightness is not meaningful when shared-edge identity is unresolved.

## 56. When a hypothesis is falsified

Record:

- hypothesis;
- prediction;
- measurement;
- verdict;
- contract boundary moved;
- remaining candidates.

Do not quietly drop a failed hypothesis.

## 57. When adding a detector

Every detector must document:

```text
Predicate being tested
Exact or approximate status
Numerical method
Tolerance source
Potential false positives
Potential false negatives
Runtime tier
Witness retained
Which proof-bearing type, if any, it may construct
```

## 58. When adding a repair

Repairs are separate from detectors.

A repair must state:

- failed contract;
- original evidence;
- repair transformation;
- obligations rechecked afterward;
- whether source semantics were preserved or inferred;
- whether ambiguity remains.

## 59. When updating this specification

A code change that alters geometric meaning must update:

- mathematical definition if needed;
- relevant contract IDs;
- Rust proof-bearing type;
- constructor preconditions/postconditions;
- tests;
- failure diagnostics.

---

# Part XI — Acceptance Criteria

Completion has **four axes**. "The types exist" does not establish that the
renderer became better, and a specification whose project method is measurement
cannot define success purely by code shape.

## 60. Axis 1 — Structural

The foundation is structurally complete when:

- no source entity identity is represented by an untyped integer outside its arena module;
- failed conversions cannot create usable mappings;
- topology conversion cannot silently drop edge uses;
- source identity survives through sampling and face use;
- projections cannot be consumed as incidence without residual certification;
- source bound syntax and resolved material semantics are distinct;
- unknown semantics remain explicit;
- periodic loops retain winding and quotient closure;
- relative deck contradictions are detected;
- every CDT constraint is provably represented by a retained chain;
- certified face meshes state their actual approximation level;
- shell certificates respect source open/closed expectations;
- diagnostics cannot forge proof-bearing states;
- no unbounded geometry-derived allocation exists (RES-001);
- no proof-bearing state can be forged through safe public APIs.

## 60a. Axis 2 — Corpus correctness

- every selected corpus model terminates;
- **no unexplained process aborts**;
- known blob reproducers either no longer blob, or fail at a specific named
  upstream contract;
- no previously correct reproducer regresses;
- every missing face carries a categorised failure rather than vanishing.

These are measured over the whole corpus, not one model. `00009190` was clean on
a build that aborted outright on `00000730`.

## 60b. Axis 3 — Performance

Measured against a pinned baseline:

\[
\frac{T_{\mathrm{new}}}{T_{\mathrm{baseline}}},
\qquad
\frac{M_{\mathrm{new}}}{M_{\mathrm{baseline}}},
\qquad
\Delta(\text{triangle count}).
\]

Budgets are set and held for wall time, peak memory, persistent certificate
memory, binary size where relevant, and time to first usable image. A
certification scheme that fails these budgets is not complete, however sound.

## 60c. Axis 4 — Diagnostic quality

For every known reproducer, the **first reported failed contract localises the
defect to the correct stage**. This is the axis that says the architecture is
doing its job, and it is empirical rather than structural: it is a statement
about what the tool prints on inputs whose defects are already understood.

## 61. Operational success

The architecture is succeeding when a malformed or internally corrupted face produces a localized error such as:

```text
TOP-001 failed:
face 48794 requested EDGE_CURVE 714381,
but arena index 62 stores EDGE_CURVE 714442.
```

or:

```text
GEO-005 failed:
edge 714381 on face 48794,
max sampled residual 2.70e-2,
permitted 3.06e-3,
worst parameter 0.417.
```

rather than a smooth unexplained blob.

---

# Appendix A — Compact Contract Map

| Family | Meaning | Primary Rust enforcement |
|---|---|---|
| TOP | identity, incidence, orientation | typed IDs, arenas, fallible wire construction |
| RES | bounded work and allocation, containment | checked count constructors, budgets, capped-result certificates |
| GEO | conversion, provenance, incidence | retained source IDs, sampled witnesses, `CurveOnSurface` |
| QUO | periodic topology and singular charts | winding-bearing loops, deck solver, singular-case types |
| DOM | material-side semantics | preserved STEP facts, known/unknown domain states |
| ARR | valid trim arrangement | checked arrangement constructors |
| CDT | complete constraint representation | constraint provenance and conforming CDT |
| MSH | face approximation and orientation | certified face mesh outcomes |
| SHL | shared boundaries and shell topology | certified shell outcomes |

---

# Appendix B — Glossary

**Arena:** Storage that owns converted entities and issues typed indices.

**Base domain:** Material occupancy before trim-loop parity toggles are applied.

**Certificate:** Retained evidence that a named obligation was checked successfully.

**Curve on surface:** A 3D edge curve paired with a face-specific parameter-space path and a compatibility certificate.

**Deck copy:** One lift of a periodic quotient coordinate into the universal covering parameter plane.

**Diagnostic:** Evidence useful for investigation but insufficient to construct a proof-bearing state.

**Effective orientation:** Composition of all source orientation flags affecting traversal.

**P-curve:** A parameter-space representation of a 3D edge on a particular supporting surface.

**Proof-bearing type:** A Rust type whose private constructor checks and retains evidence for a contract.

**Quotient closure:** Closure modulo the period lattice rather than ordinary Euclidean UV closure.

**Source identity:** Original STEP entity identity retained through conversion and derived representations.

**Typestate:** Encoding pipeline state in distinct Rust types so invalid stage transitions cannot be called.

**Witness:** Numerical or structural data supporting a certificate, such as maximum residual and permitted tolerance.

---

# Appendix C — One-Sentence Project Thesis

> Build the STEP ingestion layer as a sequence of mathematically specified, fallible refinement steps whose Rust output types carry the structural or numerical evidence required by every downstream consumer.
