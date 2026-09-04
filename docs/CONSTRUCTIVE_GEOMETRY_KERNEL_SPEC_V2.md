# Constructive Geometry and Certified Intersection
# A Complete Kernel Specification, v2

Status: normative. Sections marked (informative) are rationale and may be skipped by implementers. Every section that defines a predicate, certificate, type, or invariant is a contract. Every theorem load-bearing for the kernel is proved here; §23 lists external work and is explicitly non-load-bearing, so no module depends on an unverified external result.

Audience. An orchestrator agent decomposing work across implementation agents. This document is self-contained: the certificate mapping table (§22) and the tolerance defaults (§0.4) are inlined, and no worker needs another specification.

Changes from v1. The zero-dimensional certificate is restated as a contraction result with its proof (§8.2). Tangential-curve tracing is refused rather than mis-typed (§7 R2, §10.4). Exact tangency is identified as uncertifiable from floating-point data and quarantined into a tolerance-tagged claim (§10.3). Node identity is extended across charts, leaves, and the residual implication order (§4.2). Covering-space assembly gains deck identification and a traversal bound (§0.4, §14.2). Curve–surface, curve–curve, and trim-clip contracts are written (§7 R8/R9, §9.4). Rational leaves carry a certified weight bound as a type-level precondition (§7.1). Analytic carriers are restricted to rational reparameterizations for reproducibility (§3.2). The PH implication is corrected to a subclass statement (§5.2). The canal orthogonality invariant is proved rather than certified separately (§12.2). Types are corrected for arity (§16).

## Part 0 — Scope and doctrine

### 0.1 What this specifies

A geometry kernel with two halves that share one substrate.

**Constructive realization.** Authored topology → shared boundary geometry → direct realization → separated topological and geometric certification → topology-preserving tessellation. A procedural client knows B-rep incidence before realization; the kernel preserves that knowledge instead of recovering adjacency by proximity, sewing, or Booleans.

**Certified intersection.** Intersection of two trimmed, possibly periodic, rational or procedural parametric surfaces, producing a certified 1-complex promotable to B-rep edges; plus the reuse of that machinery for constant-radius rolling-ball fillets and single-surface self-intersection.

The two halves are not alternatives. Constructive realization is what you do when topology is known. Certified intersection is what you do when it is not. §15 adds the third mode: verifying topology that somebody else authored.

### 0.2 Non-goals for v1

Variable-radius and setback blend networks; general near-coincident (ε-overlap) face handling; dirty-input healing; general tangential-curve tracing (§10.4); singularities beyond the ordinary contact tier; non-manifold blend corners of valence > 3; general Whitney stratification; triangular transfinite patches; a public direct-B-rep builder; validated ODE integration; transcendental surface carriers.

### 0.3 Doctrine

**D1 — Fail closed.** Every construction returns an accepted object or a typed refusal carrying evidence. No operation returns a partial or approximate object as if it were accepted. Uncertainty is represented, never converted into success.

**D2 — Identity is never coordinates.** No comparison of the form dist < eps establishes the identity of any entity, at any tolerance, anywhere in this kernel. This is an audit obligation, not a style preference.

**D3 — Preserve authored knowledge; recover only what was not authored.**

**D4 — Float proposes, intervals dispose.** Exploratory work — marching, seeding, prediction, tolerance-driven sampling — runs in floating point at full speed. Interval arithmetic appears only on the accept/reject path, as an a posteriori validator of a candidate object.

Carve-out, exhaustive. Interval subdivision is used as a search mechanism at exactly three sites, and nowhere else: Tier-2 start-set isolation (§9.2), fillet-branch isolation (§12), and self-intersection branch isolation (§13). A doctrine that pretended otherwise would be violated in week one and then ignored.

**D5 — Segmentation is not topology.** Chart switches, frame switches, leaf boundaries, and sampling refinements do not create vertices, do not increase graph valence, and do not appear in any topological enumeration.

**D6 — Provenance is not proof.** An authored claim, an import annotation, or a client assertion is evidence about where an object came from. It is never a certificate. The two are separately typed and never unify.

**D7 — One witness.** An intersection branch is a single curve in the product of parameter domains. The model-space curve and both pcurves are projections of it, never independently computed geometries reconciled afterwards.

**D8 — One residual family, one solver.** All zero-finding in this kernel is Krawczyk on a member of the residual family of §7. That family is closed: R1–R9 are all of it, and curve–surface, curve–curve, projection, corner, and multiplier systems are members, not exceptions.

**D9 — No welding, no healing, no fitting on the fast path.** No surface fitting, Newton iteration, sewing, healing, Boolean, or generic surface–surface intersection appears in the constructive realization path.

### 0.4 Kernel configuration constants

Fixed at kernel initialization. Defaults are normative; a deployment may override them and must record the override in every certificate it issues.

| Constant | Default | Meaning |
| --- | --- | --- |
| ε_rep | 1e-9 (model units) | model-space representation gap allowed for stored approximants |
| ρ_max | 0.5 | Krawczyk contraction acceptance threshold |
| κ_max | 1e6 | conditioning bound above which a frame is rebuilt |
| depth_max | 40 | subdivision depth cap at the three D4 carve-out sites |
| k_a | 4 | Tier-2 direction retries before IncompleteStartSet |
| deck_max | 8 | maximum deck traversals of any periodic direction on one edge (§14.2) |
| tol.position | 1e-9 | DirectTolerance::position — model-space agreement |
| tol.parameter | 1e-11 | DirectTolerance::parameter — parameter-space agreement, C¹ detection |
| tol.jacobian | 1e-12 | DirectTolerance::jacobian — regularity floor for EG − F² |
| tol.intersection | = ε_rep | tolerance at which a tangency claim is tagged (§10.3) |

Rounding mode and evaluation-order policy per §1.

### 0.5 Module map

| Module | Content | Spec sections |
| --- | --- | --- |
| K0 | numerics, interval core, determinism | §1 |
| K1 | evidence algebra | §2 |
| K2 | CertifiedPatch, leaves, atlas, carriers | §3 |
| K3 | identity doctrine, ledger types | §4 |
| C1 | recipe, spine, frame laws, profile laws | §5.1–§5.5 |
| C2 | facet realization | §5.6 |
| C3 | edge sample ledger | §5.7 |
| C4 | manifold diagnostics | §5.8 |
| C5 | Coons4 | §5.9 |
| C6 | parametric sweep surface + B-rep constructor | §5.10 |
| S0 | product-space identity, rank theory | §6 |
| S1 | residual family | §7 |
| S2 | certificate calculus | §8 |
| S3 | completeness protocol, trim clipping | §9 |
| S4 | tracer and escalation | §10.1–§10.2 |
| S5 | contact engine | §10.3–§10.4 |
| S6 | overlap | §11 |
| S7 | fillets and canals | §12 |
| S8 | self-intersection | §13 |
| S9 | graph assembly and promotion | §14 |
| S10 | authored-topology verification | §15 |

## Part 1 — Shared substrate

### 1. Numerics contract (K0)

**N1 — Rounding.** A single documented rounding mode fixed at kernel init. Directed rounding everywhere in the interval layer.

**N2 — Evaluation order is pinned.** All reductions — interval Bernstein evaluation, signed-volume sums, dot products inside certificates — use a fixed, documented order. Compiler reassociation is forbidden: no fast-math, explicit FMA policy. Never par.sum() or any order-nondeterministic reduction.

**N3 — No observable output derives from hash-map iteration order.** Parallel writes go to index-stable slots.

**N4 — Bit reproducibility.** For a given input, kernel version, and configuration: byte-identical mesh position indices, byte-identical certificate outcomes, and identical verdicts, across repeated runs and across at least two architectures. A certification that succeeds on one platform and fails on another is "certified here," which is unusable when differencing against an external oracle. This is a CI gate, not an aspiration.

N4 has a hard consequence: no transcendental function may appear on any certificate path. sin, cos, atan2, log, and exp are not bit-reproducible across platforms without pinning a specific libm, which this kernel does not do. All carriers are therefore restricted to rational reparameterizations (§3.2). Transcendental evaluation is permitted only in the float predictor (D4), where its result is a proposal that intervals later dispose of.

**N5 — Homogeneous evaluation for rationals.** Carry (P, w); never divide by weights inside an interval evaluation; rationalize once at the end against a certified w > 0. See §7.1 for the type-level obligation this induces.

**N6 — Never normalize inside an enclosure without a certified positive lower bound on the norm in hand.**

**N7 — Degree management.** Where a predicate admits both a Bernstein-net form and an interval-evaluated form, the two differ in cost and tightness in opposite directions: interval evaluation from cached derivative enclosures is cheap and loose; a Bernstein net of the same polynomial is expensive and tight. Neither dominates. Every such predicate is implemented as a two-stage test: interval form first, Bernstein net only where the cheap form is inconclusive.

### 2. Evidence algebra (K1)

Two shapes, for two different questions. Neither is forced into the other.

```rust
/// A proposition about an object that already exists.
pub enum ClaimVerdict<T, E, R> {
    Proven(T),        // the predicate holds, with certificate T
    Disproven(E),     // the predicate provably fails, with witness E
    Inconclusive(R),  // insufficient evidence, with reason R
}

/// The outcome of an attempt to construct an object.
pub type Construction<T> = Result<T, Refusal>;

pub struct Refusal {
    pub kind: RefusalKind,
    pub backing: VerdictClass,       // Disproven | Inconclusive
    pub evidence: Evidence,          // residual, box, failing predicate
    pub partial: Option<PartialGraph>,
}
```

Rules.

1. Predicates, audits, and certificates return ClaimVerdict. Constructions return Construction.
2. Inconclusive is not Disproven. Most refusals in this kernel are inconclusive; a caller needs that distinction to decide whether retrying with a larger budget is meaningful.
3. Failure of a certificate is never evidence of nonexistence. It licenses exactly: shrink, re-frame, escalate, or refuse.
4. A certificate names its residual. Certificates on residuals unrelated by the implication order of §4.2 are never compared or merged.
5. No module weakens a certificate by re-deriving it at a coarser tolerance.
6. An accepted object contains no refusal. Diagnostics travel in Refusal { partial: Some(..) }.
7. A certificate whose truth is relative to a tolerance carries that tolerance in its type (§10.3). It never unifies with an exact certificate.

There is one evidence vocabulary and one mapping table (§22).

### 3. Geometric substrate (K2)

#### 3.1 CertifiedPatch

Every surface this kernel intersects, tessellates, or certifies is consumed through one trait. Subdivision is a domain operation: the caller splits the parameter box and the implementor tightens internally.

```rust
pub trait CertifiedPatch {
    fn enclose(&self, d: IBox2) -> IBox3;
    fn derivs(&self, d: IBox2) -> DerivativeEnclosure;          // S_u, S_v
    fn normal_cone(&self, d: IBox2) -> Cone;                     // bounds N = S_u x S_v
    fn regularity(&self, d: IBox2)
        -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason>;  // lower bound on EG - F^2
    /// Some(_) only for rational carriers; §7.1 makes this a precondition, not an option.
    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>>;
}

/// Required by R2, R7, and the contact classifier. NOT required by R1 tracing,
/// completeness, or overlap.
pub trait CertifiedPatchC2: CertifiedPatch {
    fn second_derivs(&self, d: IBox2) -> SecondDerivativeEnclosure;
}

/// Required only by the A2 cusp classifier. Takes a BOX, not a point:
/// a pointwise jet certifies nothing.
pub trait CertifiedPatchC3: CertifiedPatchC2 {
    fn third_jet(&self, d: IBox2) -> ThirdJetEnclosure;
}
```

The capability split is exactly the boundary between the ordinary path (R1 tracing, Tier 1, Tier 2, exact overlap) and the contact/fillet path.

The regularity predicate is ‖S_u × S_v‖ > 0, equivalently EG − F² > 0. There is no determinant det(S_u, S_v) for a map into R³; that expression is an error wherever it appears.

Implementors: BezierLeaf (§3.2), RationalCarrier (§3.2), Coons4 (§5.9), SpineFrameSurface (§5.10), OffsetPatch (§12.1).

#### 3.2 Leaves and carriers

NURBS remains the public representation. Polynomial computation happens on Bézier leaves obtained by knot-insertion extraction. Each leaf caches: control net and derivative control nets (second-derivative nets where C² is required); AABB and OBB; normal cone; optional curvature bounds; the certified regularity bound; and, for rational leaves, the certified weight bound.

Carriers are rational, never transcendental. Sphere, cylinder, cone, torus, and plane are represented by their rational (half-angle) parameterizations, so that all enclosures are Bernstein or interval-rational and N4 holds. A carrier whose only available parameterization is transcendental is Refuse(TranscendentalCarrier); it may be converted by the client to a rational approximant with a declared representation bound, which is then an ordinary rational carrier.

Rational patches are evaluated in homogeneous form per N5.

#### 3.3 Atlas and lifting

A parameter is `Param = (chart_id, deck: i32, ũ, ṽ)` living in a lifted covering chart. For a periodic direction, ũ ∈ R with ũ ~ ũ + P, so a pcurve may run 5.9 → 6.4 rather than jumping to 0.116. Deck transformations are carried as explicit integers, and |deck| on any single edge is bounded by deck_max (§0.4); exceeding it is Refuse(DeckExhausted), which is the termination bound for helical and wrapping branches.

Seams are not events. Wrapping into a canonical domain is an export operation. Chart and frame changes are segmentation metadata (D5). Deck identification at assembly is §14.2.

Poles need genuine alternate charts, not covering lifts. Rational carriers ship with a finite atlas of regular charts.

#### 3.4 Parametric degeneracy versus geometric singularity

```text
rank-deficient parameterization ⇒
    switch chart                        if regularity of the image is certified elsewhere
    carrier singularity (refuse/trim)   otherwise
```

A sphere pole is the first case; a cone apex is the second. A chart switch continues the same arc and never enters the contact classifier.

### 4. Identity doctrine (K3)

#### 4.1 Entity identity

A mesh position index is a pure function of (entity identity, sample ordinal) — never of coordinates.

#### 4.2 Node identity

Three rules. Rule A alone is insufficient and will refuse legal geometry.

**Rule A — same residual, same chart.** Let B* = □hull(B₁ ∪ B₂). If both certificates name the same residual R and §8.2 establishes a unique root of R in B*, then root(B₁) and root(B₂) are equal. Nesting is the special case and is not required, since two valid certified neighbourhoods of the same root need not nest. Never use B₁ ∩ B₂: two boxes each containing a certified root can intersect in a region containing neither.

**Rule B — across charts and leaves.** Chart transitions are integer deck translations; leaf restrictions are affine reparameterizations with floating-point coefficients. Transport both boxes into a common chart by outward-rounded interval evaluation of the exact map, then apply Rule A. Outward rounding preserves containment, which is all Rule A needs; exactness is not required.

**Rule C — across residuals, by implication.** Define R' ⊒ R ("R' is stronger") when R'(x) = 0 ⟹ R(x) = 0 on the common domain. Two certificates on R' and R'' with R' ⊒ R and R'' ⊒ R identify by applying Rule A to R. The admissible implications in this kernel are exactly:

```text
R2 ⊒ R1                     (G = (F, contact rows))
Contact system ⊒ R1         (§10.3; a certified contact point is a point of Z)
R6_A ↔ R6_B                 (via the transition of Theorem 13.3; same witness)
R8, R9 ⊒ nothing            (different domains; never merged with R1)
R7 ⊒ nothing                (different variables; corner systems relate to R7 only)
```

Without Rule C, §14.3 refuses a Morse saddle's node against its own four half-arc endpoints, which is legal geometry. Without Rule B it refuses every seam crossing and every leaf boundary.

If no rule applies, the nodes are not certified equal and the caller refuses rather than snapping. Identity never depends on subdivision history.

#### 4.3 Sample identity: the dyadic join

Sampling requirements are dyadic refinement requests on an edge's canonical parameter interval, expressed as a finite prefix-closed set of binary node addresses. An edge's sample set is the join of all incident requirements:

```text
EdgeSamples(E) = join( R_E, R_{F₁,E}, R_{F₂,E}, ... )
```

This is required because a face legitimately needs tighter boundary sampling than the edge curve alone would request — face tessellation error depends on the adjacent surface's curvature — while the merge must not depend on the order faces are visited.

**Theorem 4.1 (deterministic join).** The join is associative, commutative, and idempotent, is computed by integer operations on node addresses, and is therefore independent of gather order and free of floating-point comparison.

*Proof.* A dyadic refinement tree on [a,b] is a finite prefix-closed set of addresses; its parameters are the dyadic rationals a + (b−a)·k/2ᵈ at its leaves. Union of prefix-closed address sets is a set union — associative, commutative, idempotent, integer. Parameters are generated from addresses by a fixed formula in a fixed order. ∎

Float comparison occurs only inside each requester, converting a tolerance into a depth; that is deterministic under N2. The join itself, the only place order could leak, is integer.

Non-dyadic requests are not admissible on shared entities. SamplingPolicy::CustomParameters is legal only on a surface interior and on non-shared boundaries. Requesting it on an edge incident to more than one face is Refuse(NonDyadicSharedRequest). There is no embedding of an arbitrary float list into the address lattice that preserves Theorem 4.1.

#### 4.4 Positions come from the edge

```text
EdgeID  ⟶  { C(tᵢ) }   computed once, from the edge's model-space approximant
```

Both incident faces consume those positions. Pcurves are used only as interior constraints on their own face, never to produce a boundary position.

This is the actual watertightness guarantee, and it is strictly stronger than sharing integer indices. An intersection edge carries two pcurves and one model curve; if face A evaluates S₁(p₁(t)) and face B evaluates S₂(p₂(t)) at the same t, the results differ by up to ε_rep while the integer indices still match. Index identity alone passes and the mesh is still not closed.

#### 4.5 Watertightness invariant

For incident faces A, B sharing edge E: `I(A,E) == reverse(I(B,E))` as integer sequences. If the shell is combinatorially closed and every boundary mesh vertex's index derives from (EdgeID, ordinal) with positions from §4.4, the emitted mesh is closed by construction. Positional welding is never invoked.

## Part 2 — Constructive realization

### 5. Recipes, realization, and tessellation

#### 5.1 Core types

```rust
pub struct Frame3 { pub tangent: Vector3, pub normal: Vector3, pub binormal: Vector3 }
// orthonormal, right-handed; tangent = spine direction

pub struct SpineFrameRecipe<S, P, F> { pub spine: S, pub profile_law: P, pub frame_law: F }

// Core evaluator:  X(s, v) = C(s) + T(s) · P(s, v)
```

Generics live inside the constructive module only; see §5.10 for the enum boundary.

#### 5.2 Spine

```rust
pub enum Spine {
    Ph(PhSpine),          // RrmfQuintic | RmErfSeptic — exact fast path
    General(C1Curve),     // procedural, non-rational, first-class
}
```

Both variants are first-class. PH is a fast path, never an admission criterion. A user who sweeps an arbitrary B-spline spine does not have their geometry replaced by an approximation because the kernel would prefer a rational frame. If NURBS or STEP export is required for a general spine, it is a certified approximation with a declared representation bound, recorded as such.

Correct statement of what the fast path rests on. Only Pythagorean-hodograph curves can have rational rotation-minimizing frames, because only PH curves have rational unit tangents — that is a necessary condition, not a sufficient one. A general PH curve's RMF involves logarithmic terms. Rationality holds for specific characterized subclasses: quintic PH curves satisfying the RRMF condition, and degree-7 PH curves whose Euler–Rodrigues frame is rotation-minimizing. The enum names exactly those two.

```text
Spine::Ph(RrmfQuintic | RmErfSeptic)
   ⇒ rational RMF ⇒ rational sweep ⇒ exact NURBS conversion
   ⇒ polynomial parametric speed ⇒ exact arc length for chord sampling and §14.2
   ⇒ BezierLeaf is the CertifiedPatch implementor, so intersection needs no new code
```

Spine smoothness contract. Spines MUST be C¹ on the evaluated interval. Non-C¹ spines are typed-refused (SpineNotC1), never clamped or silently smoothed. Detection is declaration-based or by tangent-discontinuity sampling beyond tol.parameter; the mechanism must be deterministic.

#### 5.3 Frame laws

```rust
pub enum FrameLaw {
    FixedPlane        { normal: Vector3 },
    ArchitecturalUp   { up: Vector3 },
    ParallelTransport { initial_normal: Vector3 },
    RadialAboutAxis   { origin: Point3, axis: Vector3 },
}
```

**FixedPlane.** t = C'/‖C'‖, b = normal, n = b × t. Refuse ‖C'‖ < τ.

**ArchitecturalUp.** b = normalize(up × t), n = t × b. Refuse up ∥ t unless an explicit fallback policy is supplied. No silent frame rotation.

**RadialAboutAxis.** Analytic from the axis; rotated copies equivariant modulo floating point.

**ParallelTransport.** Rotation-minimizing, deterministic from initial_normal, stable at zero curvature and inflections. Frenet framing is never the default.

What ParallelTransport denotes. Double reflection approximates the Bishop ODE; a certified enclosure of an ODE solution requires a validated integrator, which this kernel does not build. So:

- For Spine::Ph: the exact rational rotation-minimizing frame. No ODE, no approximation.
- For Spine::General: the double-reflection frame at a declared refinement level, stored in the recipe as data (FrameData). The surface is resolution-independent once frozen and enclosable by the general implementor. Changing the recorded level changes the surface; that is by design and is recorded.

#### 5.4 Profile laws

```rust
pub enum ProfileLaw {
    Constant(Profile2D),
    Scale { profile: Profile2D, scale: ScalarLaw },
    LinearCorrespondence { start: Profile2D, end: Profile2D },
}
```

LinearCorrespondence requires explicit declared vertex and edge correspondence. Correspondence is never inferred.

#### 5.5 Sampling policy

```rust
pub enum SamplingPolicy {
    UniformCount { spine: usize },     // dyadic-embeddable when spine is a power of two
    ChordTolerance(f64),               // dyadic requester
    AngularTolerance(f64),             // dyadic requester
    CustomParameters(Vec<f64>),        // interior and non-shared boundaries ONLY, §4.3
}
```

A policy is a requester: it produces a dyadic depth request per §4.3. It does not own the final sample set for any shared entity.

#### 5.6 Facet realization (C2)

Primary contractual output: PolygonMesh with exact shared indices. Faceted Shell/Solid emission is an explicit opt-in secondary target. Faceted B-rep is not built here.

Structured grid x_ij = X(sᵢ, pⱼ). Grid vertex (i,j) is created exactly once via a private grid registry — internal only, never a public builder API. Adjacent faces reuse the identity; internal grid edges are created once and traversed oppositely. No sewing.

Cell triangulation: a quad if planarity is explicitly certified, else two triangles. Diagonal choice is deterministic, never an unstable float comparison.

Caps: closed planar start and end rings via existing planar support.

**Lemma 5.1 (caps are planar).** For X(s,v) = C(s) + T(s)P(s,v) with T(s) = [n(s)  b(s)] and 2-dimensional profile P, the end profile at s = s₀ lies in the affine plane through C(s₀) spanned by n(s₀), b(s₀).

*Proof.* The image is {C(s₀) + P₁n(s₀) + P₂b(s₀)}. ∎

Holds for all four frame laws, so "no nonplanar cap solving" is non-binding for v1. Say so in the doc comment so it is not mistaken for a capability gap.

Performance contract: D9 applies without exception.

Mandatory mesh audit on output.

- Twin-triangle winding audit — every interior mesh edge referenced by exactly two opposite-winding triangles. Applies to all outputs. Failure is Disproven, not a warning.
- Signed-volume sign sanity via V = ⅙ Σ a·(b×c) under N2's fixed summation order. Applies only to outputs declared closed. Open sweeps, sheets, single faces, and caps-only assemblies are exempt by declaration; running it on them produces a false Disproven. The declaration is caller-supplied and is checked against the boundary-edge count from §5.8: a declared-closed output with nonempty boundary_edges is itself Disproven.

#### 5.7 Edge sample ledger (C3)

```rust
pub struct EdgeSampleLedger {
    pub edge_id: EdgeID<Curve>,
    pub parameters: Vec<f64>,       // arclength domain, §14.2
    pub position_indices: Vec<usize>,
    pub positions: Vec<Point3>,     // computed once by the edge, §4.4
}
```

Each unique EdgeID is sampled once via §4.3's join; a reversed edge consumes the same integer sequence reversed. Implementation shape: a new parallel entry point reusing existing unique-edge sampling and per-face CDT internals, returning (ledger, per-face local-index triangulations), with global assembly outside. Existing tessellation entry points remain bit-identical.

#### 5.8 Manifold diagnostics (C4)

Aggregate; do not duplicate.

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

Plus vertex-link classification (closed 2-manifold ⇒ link is one cycle; with boundary ⇒ one path; two sheets at a vertex ⇒ non-manifold), and orientation diagnostics returning a consistent parity assignment or the conflicting edges and faces. Analysis only — no silent repair.

#### 5.9 Coons4 (C5)

Bilinearly blended Coons patch; boundary correctness by exact pairwise cancellation against the corner term, asserted numerically. Full trait checklist plus CertifiedPatch. First derivatives analytic. The constructor validates corner consistency to tol.position and never guesses orientation.

Coons4's exposed Jacobian J = S_u × S_v is its CertifiedPatch::regularity implementation — one call, not two under different names. Folded patches are construction-valid and geometry-invalid.

#### 5.10 Parametric sweep surface and the enum boundary (C6)

The geometry enum gains exactly one variant:

```rust
pub struct SpineFrameSurface {
    spine: Spine, profile_law: ProfileLaw, frame_law: FrameLaw, frame_data: FrameData,
}
```

Generic composition stays inside the constructive module. This is the difference between a one-variant ripple and a combinatorial one, and it is the highest-risk integration point in the constructive half.

**Owner amendment (session 51, BG-KV2-501-C6 stop 1 — recorded deviation).**
Two constraints force a refinement of the struct as literally written:
(1) the canonical `Curve`/`Surface` enums require `Clone + Serialize +
Deserialize`, and the constructive `Spine` enum (§5.2) implements neither —
so the stored `spine` field is the closed canonical `Box<Curve>` carrier
(the §5.2 decorator precedent), with the `Spine` recipe type living at the
constructive layer; (2) the B-rep constructor stores one per-face WINDOWED
realization as the enum variant's value (a side face's carrier must know
its own `s0/s1/v0/v1` window, pinned by landed volume and window
conformance tests). The variant therefore carries the four §5.10 fields —
whole sweep, no per-window combinatorial family — PLUS the window domain
as part of its closed value, and per-face windows are NOT derived from
face wires at realization time. The §5.10 rationale ("one variant, not a
combinatorial family") is unaffected.

## Part 3 — Certified intersection

### 6. The product-space identity and its rank structure (S0)

#### 6.1 Identity

```text
F(x) = S₁(u,v) − S₂(s,t) ∈ R³,     Z = { x ∈ D̃₁ × D̃₂ : F(x) = 0 },   x = (u,v,s,t)
```

Z is the intersection. The model-space curve and both pcurves are its projections (D7).

#### 6.2 Rank dichotomy

```text
DF(x) = [ S¹_u   S¹_v   −S²_s   −S²_t ] ∈ R^{3×4}
```

**Theorem 6.1 (rank dichotomy).** If both patches are immersed at x, then rank DF(x) ∈ {2,3}, and rank DF(x) = 2 ⟺ T₁(x) = T₂(x).

*Proof.* The column space of DF is T₁ + T₂, the sum of two 2-planes in R³; its dimension is at least 2 by immersion and at most 3, and equals 2 exactly when the planes coincide. ∎

**Corollary 6.2 (the singular locus is the tangency locus).** Sing(Z) = { x : F(x) = 0, N₁·S²_s = 0, N₁·S²_t = 0 }, N₁ = S¹_u × S¹_v.

*Proof.* T₁ = T₂ iff N₁ ⊥ T₂ iff N₁ annihilates a basis of T₂. ∎

**Corollary 6.3 (no transversal crossings).** Where rank DF = 3, Z is a smooth embedded 1-manifold. No node of valence > 2 occurs at a transversal point; any four-half-edge node is tangential.

This deletes an entire node kind. There is no TransversalCrossing; only MorseSaddle. Two intersection curves crossing at the same model-space point but different parameter pairs are distinct product-space witnesses and must not be identified.

#### 6.3 Maximal-minor algebra

Define m(x) ∈ R⁴ by mⱼ = (−1)ʲ det( DF with column j deleted ).

**Theorem 6.4.** (i) DF m = 0. (ii) m ≠ 0 ⟺ rank DF = 3, and then ker DF = span m. (iii) For any a ∈ R⁴, det[DF; aᵀ] = a·m.

*Proof.* (iii) Cofactor-expand [DF; aᵀ] along its last row: det = Σⱼ(−1)^{4+j}aⱼ det(DF^{(j)}) = Σⱼ(−1)ʲaⱼ det(DF^{(j)}) = a·m. (i) Apply (iii) with a any row of DF; the matrix has two equal rows, so rᵢ·m = 0. (ii) rank = 3 iff some 3×3 minor is nonzero iff m ≠ 0; with DF m = 0 and rank 3, the kernel is one-dimensional and contains m. ∎

Rank testing, kernel extraction for frame construction, and both completeness tiers read off one enclosure of m. m is not low degree — each component roughly triples the parametric degree — so N7 governs.

**Theorem 6.5 (structure of the kernel vector).** With m = (α,β,γ,δ), set w = αS¹_u + βS¹_v. Then w ∈ T₁ ∩ T₂, w ≠ 0 wherever rank DF = 3, and where the normals are non-parallel w = c(n₁ × n₂) with c continuous and nowhere zero. For a(x) = (d·S¹_u, d·S¹_v, 0, 0), a·m = d·w.

*Proof.* DF m = 0 reads αS¹_u + βS¹_v = γS²_s + δS²_t, so w ∈ T₁ ∩ T₂. If (α,β) = 0 then γS²_s + δS²_t = 0, forcing (γ,δ) = 0 by immersion of S₂, hence m = 0. Where normals are non-parallel T₁ ∩ T₂ = span(n₁ × n₂), so c = ⟨w, n₁×n₂⟩/‖n₁×n₂‖² is defined, continuous and nonvanishing. Finally a·m = α(d·S¹_u) + β(d·S¹_v) = d·w. ∎

#### 6.4 Genericity of tangency, and what it implies for scheduling

F = 0 is 3 conditions in 4 unknowns, so dim Z = 1. Tangency imposes 2 further conditions: 5 conditions in 4 unknowns, empty for generic pairs. Tangency is codimension 1 in the space of surface pairs for isolated contact, and codimension ≥ 2 for tangential curves.

The operative consequence is not "tangency is rare" — it is that tangency is never accidental. A tangency in a real model is there because somebody built it: a fillet meeting its parent face, a G¹ blend, a coaxial pair. Such tangency is authored, and authored tangency is recognized, not discovered:

- by carrier recognition, where the contact locus is exact and known in closed form (§11, §12.2);
- by certify_claimed, where the client supplies it (§15).

Numerical discovery of tangency is therefore the fallback, not the workhorse, and §10.4 refuses its hardest case outright. This is the honest form of the constructive/intersection split: constructive output does not make the singular tier the common path; it makes the singular tier mostly unnecessary.

### 7. The residual family (S1)

All zero-finding uses one of the following. The family is closed (D8).

**R1 — Ordinary (transversal).** F = S₁ − S₂ : R⁴ → R³. Regularity: rank DF = 3, certified via σ_min > 0 on the block selected by the frame.

**R2 — Contact (tangency), detection only.** G(x) = (F(x), N₁·S²_s, N₁·S²_t) : R⁴ → R⁵. At parallel normals n₁ × n₂ = 0 supplies two locally independent scalar conditions, not one; discarding a component to force a square system is unsound. R2 is overdetermined and is never passed to §8.3. Its only uses are the rank test on DG that detects a positive-dimensional contact locus (§10.4) and, via §10.3, isolated-contact classification.

**R3 — Critical point (completeness).** For a ∈ R⁴, Ψ_a(x) = (F(x), det[DF; aᵀ]) : R⁴ → R⁴, evaluated as (F(x), a·m(x)) by Theorem 6.4(iii). The multiplier form Ψ^L_a(x,μ) = (F(x), DF(x)ᵀμ − a) : R⁷ → R⁷ is a member of this family and is used for certification; the minor form is used for exclusion.

**R4 — Graph projection.** For a unit n₀ and Π = n₀^⊥, solve Π-proj(S(u,v)) = q for (u,v) — a square 2×2 system per surface, independent of the other. Certified by §8.5.

**R4′ — Normal projection (fallback).** For fixed (u,v), P(u,v;s,t) = (S¹_u·(S₂ − S₁), S¹_v·(S₂ − S₁)) : R² → R². Square. Retained for boxes where no feasible n₀ exists and subdivision is capped.

**R5 — Difference residual.** g(q) = f₁(q) − f₂(q) on Π. g is analytic but not polynomial. Its enclosure contract is §8.6; without that contract it may not be evaluated.

**R6 — Self-intersection (deflated).** §13.

**R7 — Ball-center.** §12.

**R8 — Curve–surface.** H(t,u,v) = C(t) − S(u,v) : R³ → R³. Square, C1. Used for every boundary-stratum seed (§9.3) and for the compositional three-face corner (§12.3). Regularity: det DH ≠ 0, equivalently C'(t) ∉ T(u,v).

**R9 — Curve–curve in one chart.** J(t,r) = C₁(t) − C₂(r) : R² → R², both curves in the same lifted chart. Square, C1. Used for trim–arc events (§9.4) and trim–trim events. Regularity: det[C₁′ , −C₂′] ≠ 0, i.e. the two chart tangents are independent.

#### 7.1 Rational leaves: a type-level precondition

N5 is an evaluation rule; on its own it is not a statement about zero sets. For S = P/w, the systems S₁ − S₂ = 0 and the homogenized numerator system agree only where w > 0 is certified. Where a weight enclosure straddles zero, a certificate on the numerator may certify a pole of the weight or miss a genuine zero.

Contract. Every C1, C2, and C4 entry point taking a rational carrier requires a CertifiedPositive weight bound for the box, obtained from CertifiedPatch::weight_bound, as a value argument, not as a checked assumption:

```rust
fn c1_certify(r: &Residual, b: IBox, w: &[CertifiedPositive]) -> ClaimVerdict<PointCert, _, _>;
```

Failure to obtain the bound is Refuse(WeightDegenerate), backed Disproven if the enclosure of w provably contains zero, Inconclusive if it merely fails to separate.

#### 7.2 Why two engines, and why the split is a design invariant

**Theorem 7.1 (conditioning complementarity).** Near tangency T₁ ≈ T₂, every 3×3 block of DF is near-singular by Theorem 6.1, so R1 degenerates. In the same regime both normals are nearly parallel, so a single n₀ makes both patches graphs over Π with well-conditioned projections, and g has small but well-scaled gradient. Conversely, where T₁ and T₂ are far apart no single n₀ serves both, and R1 is well conditioned.

*Proof.* By Theorem 6.1, rank DF → 2 as T₁ → T₂, so σ_min(DF Q_⊥) → 0 for every Q_⊥. For the second clause, det D(Π-proj ∘ Sᵢ) = n₀·Nᵢ by Theorem 8.3, bounded away from zero when both normals lie in a cone about n₀. ∎

The 2D engine is well conditioned exactly where the 4D engine degenerates, and vice versa. Any refactor breaking the complementarity breaks the kernel.

The 2D engine works by elimination: reducing to a scalar difference removes the artificial ill-conditioning of the 4→3 rank drop, leaving only intrinsic sensitivity. §12 uses the opposite move, lifting, for the opposite reason: elimination there introduces transcendence. There is no single principle; both moves appear, for different causes.

### 8. Certificate calculus (S2)

#### 8.1 Frames

```text
Q = [ q_τ   Q_⊥ ] ∈ O(n),    A ≈ [ DF(ẑ) Q_⊥ ]⁻¹ ∈ R^{(n−1)×(n−1)}
Frame<N> = (ẑ, Q, q_τ, Q_⊥, A)
```

A bare ê_τ ∈ Rⁿ is insufficient: it fixes the complement as a subspace but not as a basis, and A is expressed in that basis. Construct Q by SVD of DF(ẑ); q_τ may equivalently be taken as the normalized enclosure of m (Theorem 6.4).

#### 8.2 The contraction lemma, and C1

Both C1 and C2 rest on one fact, which v1 asserted without proof.

**Lemma 8.0 (interior inclusion gives contraction).** Let B be a box with radius vector r = rad(B) > 0 componentwise, R a C¹ residual, A a preconditioner, and

```text
K(B) = ẑ − A R(ẑ) + (I − A □DR(B))(B − ẑ).
```

Let M = mag(I − A □DR(B)) be the componentwise magnitude matrix. If K(B) ⊆ int B then M r < r componentwise, and hence the weighted norm ‖I − A DR(x)‖_{∞,r} ≤ ρ < 1 for every x ∈ B, with ρ = maxᵢ (M r)ᵢ / rᵢ.

*Proof.* rad(K(B)) = M r. Strict inclusion K(B) ⊆ int B requires in particular that the radius of K(B) plus the offset of its midpoint from mid(B) be strictly below r componentwise, so M r < r. For the weighted max-norm ‖y‖_{∞,r} = maxᵢ |yᵢ|/rᵢ, the induced matrix norm of any N with mag(N) ≤ M satisfies ‖N‖_{∞,r} ≤ maxᵢ (M r)ᵢ / rᵢ = ρ < 1. Every DR(x) for x ∈ B satisfies I − A DR(x) ∈ I − A □DR(B). ∎

**C1 — Zero-dimensional certificate.** Square residual R : Rⁿ → Rⁿ, box B, point ẑ ∈ B, preconditioner A, plus (for rational carriers) certified weight bounds per §7.1. If K(B) ⊆ int B, then:

1. R has a root in B (Brouwer, from K(B) ⊆ B and continuity);
2. that root is unique in B, by Lemma 8.0 and Banach;
3. the preconditioned chord iteration x ← x − A R(x) with A held fixed converges to it from any starting point in B, with rate ρ ≤ ρ_max.

Claim (3) is deliberately narrower than the folklore statement. Full Newton with a re-evaluated Jacobian at each step is not covered by this argument and is not certified by this certificate. The tracer may use it as a predictor (D4); it may not cite C1 as a convergence guarantee for it.

ρ is stored in the certificate and must satisfy ρ ≤ ρ_max.

#### 8.3 C2 — One-dimensional (tube) certificate, in Rⁿ

**Theorem 8.1 (graph certificate).** Let F : Rⁿ → R^{n−1} be C¹, Q = [q_τ  Q_⊥] ∈ O(n), A ∈ R^{(n−1)×(n−1)}, ẑ ∈ Rⁿ. Write x = ẑ + Q(τe₁ + y), τ ∈ I_τ ⊂ R, y ∈ B_⊥ ⊂ R^{n−1}. Define, with directed rounding,

```text
K(I_τ, B_⊥) = ŷ − A F(ẑ + Q(□I_τ, ŷ)) + (I − A □D_yF(I_τ, B_⊥))(B_⊥ − ŷ).
```

If K(I_τ, B_⊥) ⊆ int B_⊥ then for all τ ∈ I_τ there exists a unique y*(τ) ∈ B_⊥ with F(ẑ + Q(τ, y*(τ))) = 0, and y* is C¹ in τ with ‖Dy*‖ bounded by the enclosure.

*Proof.* Fix τ. Lemma 8.0 applied to the slice gives ‖I − A D_yF‖_{∞,rad(B_⊥)} ≤ ρ < 1, so the Krawczyk operator contracts B_⊥ into itself; Banach gives existence and uniqueness, and invertibility of A D_yF gives C¹ dependence by the implicit function theorem. Uniformity over I_τ follows because the enclosure was taken over all of I_τ at once. Nothing in the argument used a particular n. ∎

Domain of applicability, normative. §8.3 applies only to F : Rⁿ → R^{n−1}. Instances: R1 and R6 at n = 4, R5 at n = 2, R7 at n = 7. R2 (R⁴ → R⁵) is not an instance and must never be passed to it — forming A ≈ [DG Q_⊥]⁻¹ for a 5×3 matrix is a type error, and squaring R2 by dropping a row is the unsound move R2 exists to avoid.

Do not adopt any global box-cover construction. Certifying a unit sphere as thousands of 4D boxes is the wrong representation; the tube over a long τ-interval is the right one.

#### 8.4 C3 — Sheet certificate

See §11.

#### 8.5 C4 — Graph certificate for projections

**Theorem 8.3.** Let S be a patch on a box D, n₀ a unit vector, and q : D → Π = n₀^⊥ the composition of S with orthogonal projection. Then det Dq = n₀·N where N = S_u × S_v. If 0 ∉ □(n₀·N)(D) then q is injective on D and a diffeomorphism onto its image.

*Proof.* In an orthonormal basis (e₁,e₂) of Π with e₁ × e₂ = n₀, det Dq = (e₁·S_u)(e₂·S_v) − (e₂·S_u)(e₁·S_v) = (S_u × S_v)·(e₁ × e₂) = N·n₀. For injectivity: D is convex, so for x ≠ y ∈ D, q(x) − q(y) = M(x−y) with row i of M equal to ∇qᵢ(ξᵢ) for some ξᵢ on the segment, hence M ∈ □Dq(D). Interval evaluation gives det M ∈ □(n₀·N)(D), which excludes 0. ∎

GraphCert(patch, box, n₀) := 0 ∉ □(n₀·N). Feasibility of n₀ for a leaf pair is a linear-programming problem over the two cached normal cones — the same LP as §9.1. Where no feasible n₀ exists, subdivide; where subdivision is capped, fall back to R4′.

#### 8.6 C4b — R5 enclosure contract

GraphCert certifies that the projection is invertible. It does not by itself let you evaluate g. Enclosing g over a target box Q ⊂ Π requires a certified preimage, and that requires a solve. The contract:

1. **Preimage.** For each i, run C1 on R4 over q ∈ Q to obtain a box Dᵢ′ ⊆ Dᵢ with σᵢ(Q) ⊆ Dᵢ′ certified. If Krawczyk fails to contract at depth_max, Refuse(R5EnclosureFailed) (Inconclusive).
2. **Value.** fᵢ(Q) ⊆ n₀ · □Sᵢ(Dᵢ′).
3. **Gradient.** ∇fᵢ = (Dσᵢ)ᵀ (n₀·S_u, n₀·S_v)ᵀ with Dσᵢ = (Dq)⁻¹, enclosed by interval inversion of □Dq(Dᵢ′), which is nonsingular by GraphCert.
4. **Hessian** (C² carriers only) by one further differentiation of (3).
5. g = f₁ − f₂ and its derivatives follow by subtraction.

An R5 tube is then C2 at n = 2 (g : R² → R¹). No module may evaluate R5 without a R5Enclosure value in scope, enforced by making the entry point take &R5Enclosure. Bernstein does not apply to g; asserting otherwise in code or comment is an audit failure.

#### 8.7 C5 — Offset regularity

```text
Δ_off = (EG − F²) − σr(EN − 2FM + GL) + (σr)²(LN − M²)
certify:   0 ∉ □(EG − F²)   and   0 ∉ □Δ_off   on the box.
```

The correct statement is radius versus radius of curvature. This certifies local immersion only; global offset self-intersection is §13's obligation. §12.1 shows this predicate is subsumed by the R7 regularity certificate and survives as a named diagnostic and a global validity check.

### 9. Completeness protocol and trimming (S3)

Krawczyk locks a branch you already hold; it never finds one. Completeness is a separate finite start-set argument. Two tempting formulations are unsound and excluded by name:

1. "Poincaré index 0 ⇒ discard the cell" is unsound. An extremum (+1) and a saddle (−1) in the same cell cancel.
2. "A component regular for both projections and missing every patch boundary cannot exist" is false. x² + y² − r² = 0 is a loop with nonvanishing gradient everywhere on it.

#### 9.1 Tier 1 — loop-free certificate

**Theorem 9.1.** Suppose on a leaf pair (P,Q) there exists d ∈ R³ with d·(n₁ × n₂) ≠ 0 for all n₁ ∈ N_P, n₂ ∈ N_Q. Then on that pair: (i) no tangency occurs, so Z is a smooth 1-manifold; (ii) the model-space image of every component is strictly monotone in d; (iii) no component is closed; (iv) every component meets the boundary of the lifted product domain.

*Proof.* (i) forces n₁ × n₂ ≠ 0, so T₁ ≠ T₂ and Theorem 6.1 gives rank 3. (ii) At a point of Z, ker DF is one-dimensional with image under DS₁ spanned by n₁ × n₂ (Theorem 6.5), so d/dτ (d·S₁(γ(τ))) ≠ 0. (iii) A closed component would give a critical point of that function. (iv) The lifted product domain of a leaf pair is compact and Z is closed in it, so every component is compact; by (iii) it is an arc with endpoints on the boundary. ∎

When Tier 1 passes, boundary seeds alone are provably complete. The test is LP feasibility over two cached cones.

What normal cones do and do not do. They certify transversality, rule out contact, drive frame and n₀ selection, and give Theorem 9.1. They do not establish that two patches are disjoint.

#### 9.2 Tier 2 — critical-point start set

**Theorem 9.2.** Let B be the compact lifted domain of one leaf pair, a ∈ R⁴, Ψ_a as in R3. Then every connected component C of Z ∩ B either meets ∂B or contains a zero of Ψ_a. Moreover Sing(Z) ⊆ Ψ_a⁻¹(0) for every a.

*Proof.* C is compact. If C ∩ ∂B = ∅, then λ(x) = a·x attains its maximum on C at an interior x*. If x* is smooth, T_{x*}Z = ker DF(x*) ⊆ ker aᵀ, so a ∈ row(DF) and rank[DF; aᵀ] = 3 < 4, giving det = 0. If x* is singular, rank DF = 2 by Theorem 6.1, so the stacked matrix has rank ≤ 3 and again det = 0. The last claim is the same rank argument, independent of a. ∎

**Corollary 9.3 (complete start set).** Boundary seeds together with the certified zeros of Ψ_a form a complete start set on a compact lifted leaf pair.

The compactness hypothesis is discharged per leaf pair, and only there. A Bézier leaf lives on a bounded parameter box; its lift is one bounded period-window; the product of two such is compact. Theorem 9.2 says nothing about global assembly across decks. A branch that leaves one leaf's window and re-enters under a deck map is an assembly obligation, discharged in §14.2 by deck identification and bounded by deck_max. Conflating the two is the classic way a cylinder wrap fails to close.

Evaluation. Exclusion on a cell is 0 ∉ □(a·m) from the cached enclosure of m, under N7's two-stage rule.

Relation between the tiers. They share this algebra and nothing else. Theorem 9.1 uses the x-dependent covector a(x) = (d·S¹_u, d·S¹_v, 0, 0) — by Theorem 6.5, a·m = d·w ∝ d·(n₁ × n₂) — and concludes monotonicity and no loops; its proof does not transfer to Theorem 9.2, which needs a constant linear functional. Conversely, when 0 ∈ □(a·m), Tier 2 must still isolate the zeros. The 3D enclosure of n₁ × n₂ is tighter than the 4D enclosure of a·m, so Tier 1 is always tried first.

Genericity. a must be chosen so λ|_Z has isolated critical points on the smooth part; this cannot be certified in advance, so verify a posteriori. If square Krawczyk isolates every zero of Ψ_a and exclusion clears the remainder, the start set is complete. If subdivision stalls at depth_max without isolation, perturb a and retry up to k_a times. A persistent positive-dimensional Ψ_a zero set is the signature of a tangential curve and routes to §10.4, not to IncompleteStartSet.

#### 9.3 Boundary strata

Every edge of P against Q, and every edge of Q against P, is an R8 problem: 3 equations in 3 unknowns, square, C1. Every hit produces a Boundary node and a seed.

#### 9.4 Trim clipping

Completeness is proved on the leaf product; faces are trimmed. The clip step between them:

1. Certify the 1-complex on the leaf product per §9.1–§9.3.
2. Each face carries its trim loops as certified curves in lifted charts (Param, §3.3).
3. For each arc's pcurve and each trim curve in the same chart, compute certified crossings by R9. Each is a TopoNode of kind TrimCrossing, certified by C1 and identified by §4.2 Rule A.
4. Split arcs at trim crossings.
5. Classify each resulting sub-arc as inside or outside the trim interior by evaluating the winding number of the closed trim loop about one interior sample point of the sub-arc, where the sample is certified off the loop by the crossing certificates of (3).
6. Discard outside sub-arcs; retain inside ones. Trim boundary endpoints become TopoNode::TrimCrossing, which increases valence and is not a SegmentBreak.

An interior loop that meets no leaf boundary but crosses a trim is handled by (3)–(6) with no special case.

This use of winding number is sound and is not the one rejected in §9. Here it is the winding number of a closed plane curve about a point certified to lie off it — a classical, exactly computable index with no cancellation ambiguity. The rejected use was the index of a vector field over a cell, as an emptiness certificate for an unknown number of zeros. The two share a name and nothing else.

Failure to isolate a crossing at depth_max is Refuse(TrimClipFailed) (Inconclusive).

### 10. Algorithms (S4, S5)

#### 10.1 Fast path

```text
1. extract leaves                              -> leaf forest of CertifiedPatch
2. BVH pair rejection                          (AABB, then OBB)
3. Bernstein hull rejection on F               (exclusion only)
4. Tier-1 loop-free LP (Thm 9.1)               -> COMPLETENESS_MODE
5. seeds: R8 boundary events (always)
        + Tier-2 zeros of Psi_a (only if step 4 failed)
6. float predictor-corrector in current Frame, adaptive dtau
7. attempt C2 on [tau, tau + dtau]
     ok   -> extend, then try to GROW dtau
     fail -> halve dtau; after k halvings rebuild Frame; after m rebuilds escalate
8. trim clip (§9.4)
9. glue arcs at certified Nodes / Breaks; deck-identify (§14.2)
```

Four policies carry the performance:

- **Long arcs.** Accept the largest I_τ that passes C2. Grow aggressively on success. The stored curve is a quintic Hermite plus the tube. Never a point cloud.
- **Batch interval work.** Compute enclosures of S, S_u, S_v once per leaf and reuse for hull tests, DF, m, and Krawczyk. Directed rounding lives here, not in the predictor.
- **Monotone in τ only.** When the frame tilts past the slope bound, SVD-rotate and open a new arc at a FrameSwitch. Strong monotonicity of the model-space image is not required and must not be imposed; requiring it fragments helices, cylinder wraps, and any branch that folds in R³ while remaining a graph in its local frame.
- **Cheap predictor.** One Gauss–Newton step reusing the last factorization; re-factor only when κ(DF Q_⊥) > κ_max. Per §8.2, this is a predictor, not a certified iteration.

#### 10.2 Escalation ladder

```text
if sigma_min(DF) > 0 certified on the box:
        rebuild Frame, retry C2                       // conditioning, not geometry
elif parametric regularity fails:
        chart switch (§3.4) or carrier refusal         // NOT the contact classifier
elif rank test on DG shows R2 zero set is 1-dimensional:
        §10.4                                          // refuse or recognize; do NOT trace
elif R2 zero set is isolated:
        §10.3                                          // tolerance-tagged contact
else:
        Refuse(HighOrderSingularity)
```

#### 10.3 Isolated contact: classification, and what cannot be certified

**Theorem 10.1.** Let x* be a contact point: F(x*) = 0, T₁ = T₂. Let n₀ be a unit normal common to both surfaces at x* and Π = n₀^⊥. Near x* both surfaces are graphs z = fᵢ(q) over Π with ∇fᵢ(0) = 0. With g = f₁ − f₂:

```text
g(0) = 0,     ∇g(0) = 0,     Hess g(0) = II₁ − II₂,
```

both forms taken with respect to n₀ — so II₂ is sign-flipped when n₂ = −n₀.

*Proof.* Tᵢ = Π at x* gives ∇fᵢ(0) = 0. For a graph with vanishing gradient at a point, the second fundamental form with respect to the upward normal equals Hess f / √(1+|∇f|²) = Hess f there. Subtract. ∎

**Corollary 10.2 (classification).** With H = II₁ − II₂ in a common orthonormal basis of Π:

| certified condition | classification |
| --- | --- |
| det H < 0 (indefinite) | MorseSaddle — two real tangents, crossing node, four half-arcs |
| det H > 0 (definite) | MorseExtremum — isolated contact |
| rank H = 1, certified nonzero cubic in the null direction | A2Cusp |
| otherwise | Refuse(HighOrderJet) |

*Proof.* Morse lemma applied to g. ∎

**Proposition 10.3 (exact tangency is not certifiable from floating-point surface data).** An isolated contact requires g = 0 and ∇g = 0: three conditions on two unknowns. The set of surface pairs admitting one is codimension 1 in coefficient space, so a perturbation of the input coefficients by one unit in the last place generically destroys it, replacing the contact by a small loop or by nothing. No interval method can distinguish these three cases from finite-precision coefficients.

Contract. The kernel therefore certifies a tolerance-tagged claim, never an exact one:

```rust
pub struct ContactCert {
    pub critical_point: PointCert,   // EXACT: unique zero of ∇g = 0 in B, square C1
    pub gap: Interval,               // □g at that point
    pub tolerance: f64,              // = tol.intersection
    pub hessian_sign: SignCert,      // EXACT: sign of det(II₁ − II₂)
}
```

with the three-valued outcome:

- 0 ∉ gap → Disproven: there is a certified separation or a certified crossing at this critical point. This is a good outcome — the pair is transversal or disjoint here and the ordinary path resumes.
- 0 ∈ gap and width(gap) > tolerance → Inconclusive: shrink and retry.
- 0 ∈ gap and width(gap) ≤ tolerance → Proven, tagged TangencyAtTolerance(tolerance).

The classification is exact even though the contact is not. critical_point and hessian_sign are ordinary certificates; only the existence of contact is tolerance-relative. Per §2 rule 7, a TangencyAtTolerance claim never unifies with an exact certificate, and a Boolean requiring exact topology must reject a graph containing one.

At a saddle, emit one node and four half-arcs, each with its own frame along its own certified tangent. Never carry one parameterization through the node.

Requires CertifiedPatchC2; the A₂ branch requires CertifiedPatchC3, since the cubic needs each fᵢ's 3-jet — Sᵢ's 3-jet composed with the inverse 2-jet of the projection, closed-form but real work.

#### 10.4 Tangential curves: refused, or recognized

When the rank test on DG shows R2's zero set is positive-dimensional, Z contains a curve of tangency. v1 does not trace it.

There is no sound cheap route. C2 does not apply to R2 (§8.3). Dropping a contact row to square the system yields a residual whose branch is a superset of the true tangential curve, and showing the dropped component vanishes identically on that branch is a statement about the germ — interval arithmetic can bound it small but can never certify it zero. Tracing R1 instead is impossible: on a tangential curve rank DF = 2 by Theorem 6.1, so every 3×3 block is singular and C2 fails by construction.

Disposition, exhaustive:

1. **Recognized carriers.** Coaxial cylinders, concentric spheres, a torus and its axis plane, a canal and its parent face — the contact locus is exact and closed-form. Emit it directly as certified arcs with residual: Carrier. This covers the overwhelming majority of real tangential curves, because they were built that way (§6.4).
2. **Canal-to-face contact.** Not computed at all; carried. The contact direction fields d₁, d₂ are outputs of R7 and are already certified (§12.2).
3. **Client-authored.** Routes to certify_claimed (§15), which certifies the claimed arcs against ContactCert at each sampled point rather than tracing.
4. **Everything else.** Refuse(TangentialCurve), backed Inconclusive, with the R2 rank witness and the box as evidence.

Deferred with a named entry point (§21): certified deflation of R2 along the contact locus, which is a research task, not a milestone.

### 11. Overlap (S6)

v1 supports ExactSheet only. A Sheet is legal on a box D ⊂ D̃₁ iff:

1. a certified ψ : D → D̃₂ exists, from either the same recognized rational carrier with closed-form ψ, or two Bézier leaves with a certified affine or bilinear parameter map;
2. S₁(u,v) = S₂(ψ(u,v)) by certified representational equality, or certified exact correspondence for the recognized carrier;
3. n₁·(n₂ ∘ ψ) certified of constant sign;
4. det Dψ certified nonzero.

Sheet boundaries come from trims and patch boundaries and are ordinary certified arcs.

‖S₁ − S₂∘ψ‖ ≤ τ is not a sheet condition. Near-coincident faces with no exact ψ are Refuse(NearOverlap). A ToleranceSheet with explicit model-tolerance semantics is deferred (§21); admitting it implicitly turns overlap into geometric healing.

Honest scope note. "Clean B-reps" does real work here. If Booleans on imported or dirty data enter scope, a tolerance-sheet object is inevitable and must be introduced deliberately, not smuggled in through a comparison operator.

### 12. Fillets and canals (S7)

#### 12.1 The ball-center residual R7

Unknowns (c, u, v, s, t) ∈ R⁷; for i = 1,2:

```text
(c − Sᵢ)·Sⁱ_u = 0,     (c − Sᵢ)·Sⁱ_v = 0,     ‖c − Sᵢ‖² − r² = 0
```

Six polynomial equations, seven unknowns; the zero set is one-dimensional. Side selection by the certified sign of Nᵢ·(c − Sᵢ) — an inequality, not an equation.

**Theorem 12.1 (equivalence to offset intersection).** Given ‖Nᵢ‖ > 0 certified, (c,u,v,s,t) solves R7 with sign(Nᵢ·(c − Sᵢ)) = σᵢ iff c = S₁ + σ₁r n₁(u,v) = S₂ + σ₂r n₂(s,t).

*Proof.* The first two equations give c − S₁ ⊥ T₁, hence c − S₁ = μn₁; the third gives |μ| = r; the sign condition gives μ = σ₁r. Symmetrically for i = 2. ∎

**Theorem 12.2 (rank structure; offset regularity is subsumed).** At a solution, in the ordering (c ; (u,v) ; (s,t)),

```text
DR7 = [ B₁ | M₁ |  0 ]
      [ B₂ |  0 | M₂ ]
```

with Bᵢ nonsingular 3×3, and Mᵢ having zero bottom row and top 2×2 block −(Iᵢ − σᵢr IIᵢ), whose determinant is exactly Δ_off,i of §8.7. Hence rank DR7 = 6 at a solution iff both offsets are immersed at c and their tangent planes at c are distinct.

*Proof.* The ∂/∂c rows for i = 1 are S¹_u, S¹_v, 2σ₁r n₁ᵀ, a basis of R³ by immersion, so B₁ is nonsingular; likewise B₂. For the (u,v) block, ∂_u[(c−S₁)·S¹_u] = −E + σ₁rL, ∂_v[(c−S₁)·S¹_u] = −F + σ₁rM, ∂_u[(c−S₁)·S¹_v] = −F + σ₁rM, ∂_v[(c−S₁)·S¹_v] = −G + σ₁rN, giving −(I₁ − σ₁r II₁); and ∂_{u,v}[‖c−S₁‖² − r²] = −2(c−S₁)·S¹_{u,v} = 0 at a solution. Expanding det(I − σr II) yields Δ_off. A kernel vector satisfies δc = −B₁⁻¹M₁δp₁ = −B₂⁻¹M₂δp₂; the image of Bᵢ⁻¹Mᵢ is the tangent plane of Oᵢ at c, of dimension 2 iff Δ_off,i ≠ 0; two 2-planes in R³ meet in dimension 1 iff distinct. ∎

R7 is polynomial, so Bernstein applies directly and no normalized-normal enclosure is formed (N6 satisfied vacuously). It reuses C2 at n = 7. Offset regularity is not a separate precondition: its content is recovered from σ_min(DR7 Q_⊥) > 0. The cost is an O(7) frame, a 6×6 preconditioner, and a looser exclusion cone.

OffsetPatch is retained as an alternative implementor and as the geometric definition; Δ_off survives as a named diagnostic and as the global validity certificate on the resulting canal. An A/B benchmark of R7 against offset-intersection is a release gate (§20).

#### 12.2 Canal representation

```rust
pub struct Canal {
    spine: ArcId,             // residual: R7
    r: f64,
    sigma: (i8, i8),
    contact: (DirField, DirField),
}
```

**Proposition 12.3 (the normal-plane invariant is a theorem, not an obligation).** Along an R7 branch, with dᵢ = (c − Sᵢ)/r, we have dᵢ(τ)·c′(τ) = 0 identically.

*Proof.* Differentiate ‖c − Sᵢ‖² = r² along the branch: (c − Sᵢ)·(c′ − Ṡᵢ) = 0. The foot point moves within the surface, so Ṡᵢ ∈ Tᵢ, and c − Sᵢ ⊥ Tᵢ by the first two R7 equations; hence (c − Sᵢ)·Ṡᵢ = 0 and therefore (c − Sᵢ)·c′ = 0. ∎

So CertifiedOrthogonality is deleted from the type: the invariant follows from the residual and needs no separate certificate. The blend section at each τ is the spherical arc joining d₁ and d₂ in the normal plane; evaluation is a circle; no NURBS surface is needed. Conversion to NURBS is an export operation only.

The canal's contact curves with its parent faces are d₁ and d₂ themselves — certified outputs of R7, not results of an intersection. This is §10.4's case 2 and is why the most common tangential curve in real models is never traced.

#### 12.3 Three-face corner

Compositional (preferred when the arc exists): solve c₁₂(τ) = O₃(u,v) — R8, square, C1. Direct: c ∈ R³ and three parameter pairs (9 unknowns); three R7 equations per face (9 equations); square, C1.

On failure: Refuse(CornerUnsolved). Do not invent a blend network.

#### 12.4 Scope

v1: constant-radius, rolling-ball, manifold networks of valence ≤ 3.

### 13. Self-intersection (S8)

The naive residual S(u,v) − S(s,t) vanishes on the diagonal. Deflate by blowing it up. With (s,t) = (u+h, v+k),

```text
S(u+h,v+k) − S(u,v) = h·D₁ + k·D₂
D₁ = [S(u+h,v+k) − S(u,v+k)]/h,     D₂ = [S(u,v+k) − S(u,v)]/k
```

both polynomial and computable on the Bézier net. For rational S = P/w, form the residual on the numerator P(u+h,v+k)w(u,v) − P(u,v)w(u+h,v+k), which vanishes on the diagonal and admits polynomial divided differences; §7.1 governs.

```text
Chart A:  δ = λ(1,m), λ > 0, |m| ≤ 1
          R6_A(u,v,λ,m) = D₁(u,v,λ,λm) + m·D₂(u,v,λ,λm) ∈ R³
Chart B:  δ = λ(m,1), λ > 0, |m| < 1
          R6_B(u,v,λ,m) = m·D₁(u,v,λm,λ) + D₂(u,v,λm,λ) ∈ R³
```

Each is 3 equations in 4 unknowns, structurally identical to R1, served by C2 at n = 4. A unit-circle constraint is not used: it yields 4 equations in 5 unknowns and a sign symmetry under which two uniqueness certificates contend for one curve.

**Theorem 13.1 (exact cover, no double count).** For every nonzero δ = (h,k), exactly one of δ, −δ is admissible, in exactly one chart.

*Proof.* If |h| ≥ |k| then h ≠ 0; exactly one of ±δ has h > 0, giving λ = h > 0, m = k/h, |m| ≤ 1 — chart A, uniquely, and excluded from B which requires |h| < |k|. If |k| > |h|, symmetrically chart B with |m| < 1, excluded from A which requires |k| ≤ |h|. Exhaustive and mutually exclusive. ∎

**Corollary 13.2.** Each unordered pair {P, P+δ} has exactly one witness. ∎

**Theorem 13.3 (transitions preserve the witness).** Transitions are of exactly two kinds, both SegmentBreak: Type I at m_A = +1, same base point, m_B = 1/m_A, λ_B = λ_A m_A; Type II at m_A = −1, base-point swap to −δ at P + δ.

*Proof.* Equating λ_A(1,m_A) = λ_B(m_B,1) gives m_B = 1/m_A, λ_B = λ_A m_A, admissible iff m_A > 0. At m_A = −1 this fails and Theorem 13.1 selects −δ, the same unordered pair by Corollary 13.2. ∎

**Theorem 13.4 (no branch lost).** Zeros with λ > 0 are genuine self-intersections. On λ = 0, R6_A reduces to S_u + m S_v = 0, solvable only where S_u, S_v are dependent.

*Proof.* D₁(u,v,0,0) = S_u, D₂(u,v,0,0) = S_v. ∎

The λ = 0 stratum is parametric degeneracy, routed to §3.4. It never reaches the contact classifier.

### 14. Graph assembly and B-rep promotion (S9)

#### 14.1 Node identity

Per §4.2, all three rules. Rule A alone will refuse legal geometry.

#### 14.2 Segment gluing and deck identification

Adjacent arcs meeting at a SegmentBreak must satisfy:

1. their tubes overlap in a region containing a common certified point (TubeOverlapCert);
2. their stored Hermite approximants agree to C¹ at the break within ε_rep;
3. the exported pcurve is the concatenation reparameterized to a single monotone parameter, taken as arclength of the model-space approximant. This is the ledger's parameter domain (§5.7).

Deck identification. An arc ending at (chart, deck = k, ũ) and one beginning at (chart, deck = k+1, ũ − P) denote the same point of the quotient. At assembly:

1. compute the total deck displacement along each closed chain;
2. a chain whose endpoints differ by an exact integer deck translation and whose nodes identify by §4.2 Rule B closes as a loop; the deck displacement is recorded on the edge as its winding, and is exported;
3. |deck| exceeding deck_max on one edge is Refuse(DeckExhausted) — this is the termination bound for helices and wraps;
4. deck identification is a SegmentBreak-level operation and creates no vertex (D5).

Without (1)–(3), a cylinder wrap or torus seam produces an open chain whose two ends are the same geometry and never closes as topology. Theorem 9.2 does not and cannot supply this; it is per-leaf-pair.

#### 14.3 Promotion to a model edge

An Arc becomes a B-rep edge only after:

1. both pcurves lie in lifted charts of the owning faces;
2. every endpoint is a shared TopoNode with a C1 certificate, identified by §4.2;
3. trim events are certified R9 events in one chart (§9.4);
4. ‖S₁(π_uv γ̂) − S₂(π_st γ̂)‖_∞ ≤ ε_rep;
5. §14.2 holds across every internal break, including deck identification;
6. knot multiplicity is set at crossings and cusps;
7. the edge publishes an arclength parameterization and a position table per §4.3–§4.4 before any face is tessellated against it;
8. no endpoint carries a TangencyAtTolerance tag, unless the caller has explicitly opted into tolerance-tagged topology.

If tubes overlap and endpoints do not match under any rule of §4.2: Refuse(SliverOrNearOverlap). Never snap.

### 15. Authored-topology verification (S10)

```rust
pub struct TopologyClaim {
    pub components: Vec<ClaimedComponent>,   // seed Point4 + expected kind
    pub exhaustive: bool,
    pub provenance: Provenance,
}

pub fn certify_claimed(pair: &LeafPair, claim: &TopologyClaim)
    -> ClaimVerdict<CertifiedGraph, ClaimRefutation, Refusal>;
```

1. Each claimed component is certified independently: tube chain via C2, endpoints via C1, nodes via §4.2.
2. If any is refuted, return Disproven(ClaimRefutation) naming the component and the failing predicate. Never silently repair.
3. If exhaustive, completeness must still be discharged — but the claim narrows it: Tier-1 and Tier-2 exclusion run on the complement of the certified tubes, where exclusion is most effective. Targeted completeness, not skipped completeness.
4. Without exhaustive, the result is a ClaimedGraph (§16), which downstream Booleans requiring closure must reject.

D6 applies without exception. provenance is not a certificate and does not discharge (3). A trusted-provenance mode may skip (3); its output is a ClaimedGraph, a distinct type from CertifiedGraph, so a Boolean signature cannot accept the wrong one by accident.

This converts §9's completeness protocol from search into verification, and combined with §10.4's cases 1–3 it is how authored tangency reaches the kernel without being traced.

## Part 4 — Types, taxonomy, and execution

### 16. Consolidated types

```rust
// ---------- substrate ----------
struct ChartId(u32);
struct Param  { chart: ChartId, deck: i32, u: f64, v: f64 }   // lifted, never wrapped
struct Point4 { p1: Param, p2: Param }

// ---------- certificates ----------
struct Frame<const N: usize> { z_hat: [f64; N], q: Ortho<N>, q_tau: Vector<N>,
                               q_perp: Matrix<N, {N-1}>, a: Matrix<{N-1}, {N-1}> }

struct ArcCert<const N: usize> {
    residual: ResidualId,          // R1 | R5 | R6 | R7 | Carrier
    frame: Frame<N>,
    i_tau: Interval,
    b_perp: IBox<{N-1}>,
    rho: f64,                      // <= RHO_MAX, from Lemma 8.0
    jac_encl: IMatrix,
    weights: Option<Vec<CertifiedPositive>>,   // §7.1, rational carriers
}

struct PointCert  { residual: ResidualId, box_: IBox, rho: f64 }
struct ContactCert { critical_point: PointCert, gap: Interval,
                     tolerance: f64, hessian_sign: SignCert }     // §10.3
struct GraphCert  { domain: IBox2, n0: Vector3, det_bound: CertifiedNonzero }
struct R5Enclosure { q: IBox2, preimage: [IBox2; 2], cert: [PointCert; 2] }  // §8.6
struct SheetCert  { domain: IBox2, psi: PsiMap, det_dpsi: CertifiedNonzero }

// ---------- geometry ----------
struct Approx { gamma: HermiteSpline }         // THE single witness approximant
struct Arc<const N: usize> { id: ArcId, approx: Approx,
                             cert: ArcCert<N>, ends: (ArcEnd, ArcEnd) }

enum AnyArc { Ordinary(Arc<4>), Difference(Arc<2>),
              SelfInt(Arc<4>), Spine(Arc<7>), Carrier(CarrierArc) }

// ---------- topology vs. segmentation ----------
enum TopoNode {                       // increases graph valence
    Boundary, TrimCrossing, MorseSaddle, MorseExtremum, A2Cusp,
    OverlapBoundary, FilletEnd,
}
enum SegmentBreak {                   // does NOT increase graph valence
    ChartSwitch, FrameSwitch, LeafBoundary, DeckStep, R6ChartSwitch, R6BaseSwap,
}
enum ArcEnd { Topo(NodeId), Seg(BreakId) }

struct Node  { id: NodeId, at: Point4, kind: TopoNode, cert: NodeCert }
enum  NodeCert { Exact(PointCert), AtTolerance(ContactCert) }   // §2 rule 7
struct Break { id: BreakId, at: Point4, kind: SegmentBreak, overlap: TubeOverlapCert }

struct Sheet { domain: IBox2, psi: PsiMap, cert: SheetCert, boundary: Vec<ArcId> }
struct Canal { spine: ArcId, r: f64, sigma: (i8, i8),
               contact: (DirField, DirField) }   // orthogonality is Prop. 12.3

// ---------- graphs: two types, never unified ----------
struct CertifiedGraph { nodes: Vec<Node>, breaks: Vec<Break>,
                        arcs: Vec<AnyArc>, sheets: Vec<Sheet>, exhaustive: Exhaustive }
struct ClaimedGraph   { graph: CertifiedGraph, provenance: Provenance }  // §15
struct PartialGraph   { graph: CertifiedGraph, frontier: Vec<Point4> }   // evidence only

// ---------- constructive ----------
struct SpineFrameSurface { spine: Spine, profile_law: ProfileLaw,
                           frame_law: FrameLaw, frame_data: FrameData }
struct EdgeSampleLedger { edge_id: EdgeID<Curve>, parameters: Vec<f64>,
                          position_indices: Vec<usize>, positions: Vec<Point3> }
```

Refuse must not appear in TopoNode. An accepted graph contains no refusal.

### 17. Refusal taxonomy

```rust
enum RefusalKind {
    // --- constructive ---
    SpineNotC1,                   // Disproven
    FrameSingular,                // Disproven
    ProfileCollapse,              // Disproven
    ProfileCorrespondenceMismatch,// Disproven
    NonFinite,                    // Disproven
    WindingAuditFailed,           // Disproven — §5.6
    NonDyadicSharedRequest,       // Disproven — §4.3

    // --- carrier, charts, numerics ---
    CarrierSingularity,           // Disproven — §3.4
    ChartExhausted,               // Disproven
    TranscendentalCarrier,        // Disproven — §3.2 / N4
    WeightDegenerate,             // Disproven or Inconclusive — §7.1
    DeckExhausted,                // Inconclusive — §14.2

    // --- intersection ---
    Conditioning,                 // Inconclusive
    TangentialCurve,              // Inconclusive — §10.4, v1 scope
    HighOrderJet,                 // Inconclusive — §10.3
    IncompleteStartSet,           // Inconclusive — §9.2
    R5EnclosureFailed,            // Inconclusive — §8.6
    TrimClipFailed,               // Inconclusive — §9.4
    NearOverlap,                  // Disproven *of ExactSheet* — §11
    OffsetDegenerate,             // Disproven
    OffsetSwallowtail,            // Disproven — §8.7
    CornerUnsolved,               // Inconclusive — §12.3
    SliverOrNearOverlap,          // Inconclusive — §14.3
    ClaimRefuted,                 // Disproven — §15
    Budget,                       // Inconclusive
}
```

Refuse is an outcome, not topology.   Inconclusive is not False.

### 18. Complexity budget

**Transversal profile.** O(L) float predictor steps for τ-length L, plus O(1) C2 validations per successful extension. Tier-1 completeness is O(1) — one LP over two cached cones. Nearly all CPU in patch evaluation and 3×3 solves.

**Designed-tangency profile.** Both tiers fail: Tier 1 because normals are parallel somewhere, Tier 2 because Ψ_a has positive-dimensional components. What happens next depends on §6.4's routing, and this is where the budget actually lives:

| route | cost |
| --- | --- |
| recognized carrier (§10.4 case 1) | O(1) — closed form, no subdivision |
| canal-to-face (§10.4 case 2) | zero — carried, not computed |
| client-authored (§15) | O(k) ContactCert evaluations, no completeness search |
| isolated contact (§10.3) | a handful of interval 2- and 3-jets |
| general tangential curve | refused |

So the honest headline is not "the singular tier dominates on constructive output" — it is that constructive output routes around the singular tier, and the residue that does not is refused rather than paid for. The expensive case that remains is Tier-2 subdivision on imported geometry with undocumented tangency, which is exactly the corpus where refusal is also the most defensible answer.

Profile transversal and designed-tangency corpora separately; a single aggregate benchmark hides the split.

### 19. Build order and dependency graph

| # | Module | Deliverable |
| --- | --- | --- |
| 1 | K0 | interval core, directed rounding, pinned order, N1–N7 |
| 2 | K1 | ClaimVerdict, Construction, Refusal, mapping table (§22) |
| 3 | K2 | CertifiedPatch + subtraits; BezierLeaf, RationalCarrier; BVH; Bernstein exclusion; atlas with deck integers |
| 4 | K3 | identity rules A/B/C, dyadic join, ledger types |
| 5 | C1 | recipe, Spine, frame laws, profile laws, sampling |
| 6 | C2 | facet realization + mesh audit with closed/open declaration |
| 7 | C3 | edge sample ledger |
| 8 | S4a | float tracer, uncertified |
| 9 | S2a | Lemma 8.0, C1, C2 (ArcCert<N>), frame construction |
| 10 | S1a | R8, R9 (curve–surface, curve–curve) |
| 11 | S0/S3a | maximal-minor algebra, boundary seeds via R8, Tier-1 LP |
| 12 | S5a | II₁ − II₂ classifier + ContactCert (pulled forward; jets only) |
| 13 | S9a | node identity, graph assembly, deck identification |
| 14 | S3b | Tier-2 Ψ_a start set |
| 15 | S3c | trim clipping (§9.4) |
| 16 | S2b | GraphCert, R5 enclosure contract, R4/R4′ |
| 17 | C4 | manifold diagnostics |
| 18 | C5 | Coons4 (parallel-eligible from step 3) |
| 19 | K2b | full lifted atlas, pole charts, rational carrier family |
| 20 | S7 | R7, canal, Δ_off validity, corner |
| 21 | S6 | ExactSheet + recognized-carrier contact loci (§10.4 case 1) |
| 22 | S8 | R6 projective charts |
| 23 | C6 | SpineFrameSurface + B-rep constructor (integrator-owned, runs alone) |
| 24 | S9b | B-rep promotion |
| 25 | S10 | certify_claimed, ClaimedGraph |

```text
K0 → K1 → K2 ─┬─ K3 ─┬─ C1 → C2 → C3 ─────────────┬─ C6 → S9b
              │      ├─ C5 (parallel)             │
              │      └─ C4 (parallel)             │
              └─ S4a → S2a → S1a → S0/S3a ─┬─ S5a ┤
                                           ├─ S9a ┤
                                           ├─ S3b → S3c
                                           ├─ S2b        ├─ S10
                                           ├─ S7         │
                                           ├─ S6 ────────┤
                                           └─ S8 ────────┘
```

Elastic pool — dispatch whenever a slot is idle: corpus fixtures (cube, holed prism, multi-hole plate, straight/tapered/90° ducts, S-rail, annular sweep, ribbed panel, warped Coons shell, repeated sweep assembly); the transversal battery (sphere/cylinder/plane pairs, small loops, seam crossings, helices); the designed-tangency battery (coaxial cylinders, sphere–plane contact, fillet chains, G¹ blend runouts, canal-to-face); mutation batteries; microbenchmarks. Concurrency capped at ≤ 3 live packets over the write-set-disjoint set.

### 20. Gates and acceptance tests

Split every intersection test across two corpora. The transversal battery exercises §9.1 and §10.1; the designed-tangency battery exercises §6.4's routing, §10.3, and §10.4. A single aggregate number is not an acceptance signal.

| Module | Acceptance test |
| --- | --- |
| K0 | bit-reproducible enclosures on two architectures; audit: no transcendental call on any certificate path; no par.sum() |
| K1 | every certificate and refusal in this document has a §22 row; Inconclusive representable and surfaced everywhere |
| K2 | zero false exclusions on a random-pair corpus; no module names BezierLeaf outside K2; Coons4::regularity and its exposed J are one call; torus/sphere carriers are rational and reproduce bit-identically |
| K3 | join order-independent under randomized gather (property test); boundary positions byte-identical across incident faces; CustomParameters on a shared edge refuses |
| C1 | polyline spine → SpineNotC1; RRMF quintic → rational sweep with exact NURBS conversion; general B-spline spine → working CertifiedPatch, not refused for promotion |
| C2 | closed shells tessellate to Closed with no positional welding; winding failure → Disproven; open sweep does not trigger a signed-volume Disproven |
| C3 | I(A,E) == reverse(I(B,E)) as integers; existing entry points bit-identical |
| S2a | Lemma 8.0's ρ recorded and ≤ ρ_max on every accepted certificate; audit: no claim of full-Newton convergence anywhere; audit: R2 is never passed to C2 |
| S1a | R8 seeds a patch-boundary crossing; R9 seeds a trim–arc crossing; both square, both C1 |
| S0/S3a | Tier 1 passes on the whole transversal battery; boundary-seed completeness matches oracle component counts; two-stage minor test escalates to Bernstein on < 20% of transversal cells |
| S5a | sphere–plane and equal-radius cylinder–cylinder classify correctly with no R5Enclosure in scope (audit); a deliberately perturbed near-tangency returns Disproven with a certified gap, not a false contact; a genuine contact returns TangencyAtTolerance |
| S9a | no identity by proximity anywhere (audit); a full cylinder wrap closes as a loop via deck identification; a helix exceeding deck_max refuses rather than looping forever; a Morse saddle's node identifies against its four half-arc endpoints (Rule C) |
| S3b | finds the interior loop on a torus/plane tangential-adjacent case and on the small-loop battery |
| S3c | an interior loop crossing a trim but missing every leaf boundary is clipped correctly; the point-in-region sample is certified off the loop |
| S2b | GraphCert is a cone test with no solve; R5 tube certified at n = 2 through the §8.6 contract; R4′ fallback exercised on a no-feasible-n₀ fixture |
| S6 | coplanar faces, coaxial cylinders; coaxial-cylinder contact locus emitted in closed form, not traced |
| S7 | fillet spine on two-plane and plane–cylinder edges; valence-3 box corner; Δ_off = 0 refuses via the R7 regularity certificate with no separate precondition; R7 and offset-intersection agree on all fixtures, benchmark recorded; audit: Canal has no orthogonality certificate field |
| S8 | detects a self-overlapping sweep; exactly one witness per self-intersection (double-count regression); none on valid patches; λ = 0 routes to chart/carrier |
| S9b | watertight trims on the Boolean regression corpus; a graph containing TangencyAtTolerance is rejected by a promotion that has not opted in |
| S10 | a deliberately wrong component count returns Disproven, never a repaired graph; a ClaimedGraph is rejected by a CertifiedGraph signature at compile time |

**Realization agreement gate.** For every recipe and policy in the corpus: (1) I_FAC(E) == I_BREP(E) as integer sequences for every shared edge; (2) boundary positions identical; (3) topology and incidence identical wherever both realize the same authored topology; (4) interior geometry within a certified mutual deviation ≤ tol.position; (5) verdicts identical. Not whole-mesh byte identity.

**Cross-cutting audits.** No dist < eps establishes identity. No TopoNode named Refuse, ChartSwitch, FrameSwitch, or DeckStep. No path requires 3D strong monotonicity. Poles never reach the contact classifier. No face produces a boundary position by evaluating its own surface. The sampling join is integer-only. Provenance never appears where a certificate is required. No comment asserts Bernstein applies to g. R2 never reaches C2. No transcendental on a certificate path. No ClaimedGraph reaches a CertifiedGraph consumer.

### 21. Deferred, with the reason

| Deferred | Reason | Trigger to revisit |
| --- | --- | --- |
| General tangential-curve tracing | no sound cheap route (§10.4); squaring R2 is unsound, R1 degenerates by Theorem 6.1 | certified deflation of R2 along the contact locus is worked out, or corpus prevalence outside cases 1–3 justifies it |
| ToleranceSheet | conflating with ExactSheet turns overlap into healing | Booleans on imported or dirty data enter scope |
| Validated ODE integration | §5.3 removes the need | a frame law genuinely only definable as an ODE solution |
| Transcendental carriers | N4 forbids them on certificate paths | a pinned, reproducible libm is adopted kernel-wide |
| Variable-radius and setback blends | full blend-network optimizer | valence-3 constant-radius solid |
| Exact (non-tolerance) tangency certification | Proposition 10.3: impossible from float coefficients | exact-arithmetic input path exists |
| Triangular transfinite patches | facets cover it | measured gap |
| Public direct-B-rep builder | handles already author topology | a second independent client reinvents it |
| Model-space silhouette starts | Theorem 9.2 gives completeness | a test case forces it |
| Non-Morse singularities beyond A₂ | research, not a milestone | corpus prevalence justifies |

Promotion doctrine. Independent reinvention by two clients, lost kernel information, representation unlock, or client-becomes-mini-kernel. Repeated use inside one client is insufficient.

### 22. Certificate mapping table

The single booking surface. A packet producing a new certificate or refusal adds its row before dispatch.

| Certificate / verdict | Type | Produced by | Consumed by | Evidence on failure |
| --- | --- | --- | --- | --- |
| regularity | CertifiedPositive | §3.1 | R1, R7, R8, all tracing | Degeneracy { box, EG−F² enclosure } |
| weight bound | CertifiedPositive | §3.1, §7.1 | every C1/C2 on rational leaves | Pole { box, w enclosure } |
| normal cone | Cone | §3.1 | Tier 1, n₀ LP, frame selection | — (total) |
| PointCert | exact | C1 §8.2 | node identity, seeds, corners | { residual, box, ρ } |
| ArcCert<N> | exact | C2 §8.3 | arcs, tubes, promotion | { residual, frame, I_τ, ρ } |
| GraphCert | exact | C4 §8.5 | R4, R5 enclosure | { n₀, det enclosure } |
| R5Enclosure | exact | §8.6 | R5 tracing only | R5EnclosureFailed |
| ContactCert | tolerance-tagged | §10.3 | contact nodes, classification | { gap, tolerance }; three-valued |
| SheetCert | exact | §11 | ExactSheet | NearOverlap |
| Δ_off | exact | §8.7 | canal global validity | OffsetSwallowtail |
| TubeOverlapCert | exact | §14.2 | segment gluing | SliverOrNearOverlap |
| winding audit | ClaimVerdict | §5.6 | mesh acceptance | failing edge list |
| signed volume | ClaimVerdict | §5.6 | closed shells only | declaration mismatch |
| ManifoldDiagnostics | ClaimVerdict | §5.8 | shell acceptance | per-entity diagnostics |
| ClaimRefutation | exact | §15 | authored verification | component + failing predicate |
| Provenance | not a certificate | client | ClaimedGraph only | — |

### 23. Provenance of results (informative)

**Proved here, load-bearing:** Theorems 4.1, 6.1, 6.4, 6.5, 7.1, 8.1, 9.1, 9.2, 10.1, 12.1, 12.2, 13.1, 13.3, 13.4; Lemma 8.0; Propositions 10.3, 12.3; Corollaries 6.2, 6.3, 9.3, 10.2, 13.2; Lemma 5.1. No module may add a dependency on an unproved external theorem without adding a proof or a fallback here.

**Classical, no citation risk:** the Krawczyk operator and Moore's uniqueness theorem — used only in the form proved as Lemma 8.0 and Theorem 8.1, not as folklore; loop detection via a common transversal direction on normal cones (Sederberg–Meyers lineage) and its LP formulation over bounded Gauss maps (Hohmeyer); jet classification of ordinary intersection singularities (Kriezis–Patrikalakis–Wolter lineage); topology-before-tracing as an architectural stance (Grandine–Klein lineage); rolling-ball fillets as offset–offset intersection and canal surfaces as sphere envelopes; that only Pythagorean-hodograph curves admit rational unit tangents, and the characterizations of quintic PH curves with rational rotation-minimizing frames and of degree-7 PH curves with rotation-minimizing Euler–Rodrigues frames (Farouki and co-authors — cite by author; an earlier draft carried a wrong DOI and that attribution is void).

**Deliberately not load-bearing:**

- Generalized interval Krawczyk work is a source of ideas for SVD-aligned frame selection only. Theorem 8.1 is proved independently and the associated global box-cover construction is rejected outright.
- Winding-number intersection methods are not used for completeness. §9 replaces them. The 2026 line combines winding number with subdivision of a vector field on one parametric domain; the subdivision is what would break index cancellation, relocating the burden onto a termination argument Theorem 9.2 supplies directly. An erratum is associated with at least one paper in this line. (§9.4's use of winding number is a different and sound object; see the note there.)
- Claims that topology-guaranteed tracing requires dual 4D/3D strong monotonicity could not be verified against a specific source, and the dual requirement is rejected on its merits (§10.1).

**Reading list, content unverified.** Title and venue confirmed; nothing below has been read, and no characterization of its content should be trusted until it has been. Four of the six originate from one group, so this is one line of work, not a survey.

- Li, Yang & Jia, Advances and challenges in surface–surface intersection computation — An overview, CAD 193, 104039 (2026). Date inconsistent across sources.
- Yang & Jia, A Robust and Efficient Intersection Algorithm for NURBS Surfaces: Handling Small Loops and Tangent Intersections, TOG 45:5 (2026). Abstract read; winding-number based, hence covered by the rejection above. Its tangent-intersection handling is the nearest published alternative to §10.4's refusal and is the first thing to read if that scope decision is revisited.
- Wang, Jia, Yang, Wang, Bo & Liu, Improving the Watertightness of Parametric Surface/Surface Intersection, CGF (Dec 2025). Nearest prior art to §4.4 and §14.
- Overlap Region Extraction of Two NURBS Surfaces, TOG (Dec 2025). Sits on §11's boundary.
- He, Wang & Zhao, Self-intersection detection algorithm of sweep surfaces based on geometric features of spine curves, CAD 192, 104024 (2026). Reported scope is planar profiles along planar spines — narrower than §13.
- Hass, Farouki, Chang, Song & Sederberg, Guaranteed consistency of surface intersections and trimmed surfaces using a coupled topology resolution and domain decomposition scheme, Adv. Comput. Math. 27 (2007). Nearest classical prior art to §14 and §9.4.

Any result promoted out of this section requires a proof or a fallback added to Part 3.
