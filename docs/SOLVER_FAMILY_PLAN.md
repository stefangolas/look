# Solver family program plan

Status: **approved design, not yet dispatched.** Recorded at the close of
session 26, after the base kernel loop finished 76/76 and BG-AUDIT-001 closed
17/17. This is the plan for the next program: a family of certified CAD
solvers (S1..S8) built on the existing evidence substrate.

The plan is a program-level artifact. Per-packet detail belongs in
`docs/GENERATION_KERNEL_BUILD_SPEC.md` (as Stage-5 / §9 extension packets) and
in `loop/packets/`. Nothing here is a packet yet.

## 1. The one-paragraph architecture

Recognize structure aggressively, solve in the lowest-dimensional/simplest
representation, and use validated generic machinery only when necessary.

```
public CAD operation
        ↓
structural recognition
        ↓
L0: provenance / trivial result
        ↓
L1: analytic / dimension-reduced solver
        ↓
L2: regular validated numerical solver
        ↓
L3: topology-degenerate general solver
        ↓
Certified result | ordinary CAD failure
```

Those are internal execution strategies of the same public API. `fillet()`,
`boolean()`, `extrude()` stay the same functions; the ladder is how they
dispatch.

## 2. Locked design decisions (do not relitigate)

- **Keep Arc-handle `truck-topology` at the boundary. Do NOT flatten it.**
  Persistent B-rep stays Arc handles. During an operation, the solver works on
  dense local scratch (`FaceWorkset { faces, edges, vertices }` +
  `HashMap<FaceID, usize>`), then commits. This gives the transaction boundary
  `B_in → W_scratch → B_out`: if the solver returns Unresolved, no malformed
  partial B-rep is ever committed. That is the natural place for BG-INV-001
  checkers to run (BG-TEST-002).
- **The structural recognizer produces a witness, not a type.**
  `CanonicalCarrierWitness { ExactCanonical { carrier, map } |
  Derived { carrier, provenance, map } | Unrecognized }`, where `S_stored =
  S_canonical ∘ φ` is a certified relationship. Coincidence (S5.0) is then a
  lookup on the witness, not a re-solve.
- **The BVH and the Bézier-span cache share one abstraction:** `BoundedPiece`
  (`bbox`, `derivative_bounds`, `subdivide`). Analytic faces, Bézier surface
  spans, Bézier curve spans, trim segments and intersection candidates all
  enter the same broad phase: BREP → faces → carrier spans → BVH nodes →
  candidate span pairs → certified solver.
- **`CurveContact` ontology is defined once, in 2-D, and reused by 3-D.**
  Contact has `dimension` (0D point / 1D arc / 2D region) and event kinds
  (transverse, tangency, endpoint touch, coincident interval, identical
  carrier). Getting it right in S1 makes S5 conceptually familiar rather than
  a second paradigm.
- **Adaptive-exact predicates move into Phase 1, not deferred to SSI.**
  Topology-changing predicates (`orient2d`, event ordering, exact tangency,
  endpoint membership) are never naked f64 comparisons. Escalation discipline:
  `Certified<T> = Proven(T) | Unresolved(reason)`. S5 inherits it.
- **S2 direct B-rep is the reference implementation of the Boundary Rewrite
  Atlas output side.** `extrude(profile)` builds cap/side/shared-edge topology
  combinatorially (n side faces, 1 top, 1 bottom) with no tool-body Boolean.
  Pcurves on the edges are NOT part of S2 — the topology erases `PC` at the
  `Wire` boundary (see §4 Phase 2, AMENDED session 30). If this is rock-solid
  before S6, a failed Boolean is provably about contact, not assembly.
- **S5.0/S5.2/S5.3 are replaced by a Contact Layer:** `contact(lhs, rhs) ->
  Certified<ContactComplex>` over boundary strata — FF 0D/1D/2D, FE 0D/1D,
  EE 0D/1D. Boolean depends on ContactSolve being exhaustive over strata, not
  specifically on SSI. Dispatch inside: identity/overlap → analytic → general
  validated → singular. (The 2D-overlap/coincident-patch case is the hard one
  and the last to build.)
- **Coincidence levels:** C0 provenance identity (same Arc / originating
  carrier) → C1 canonical analytic equivalence (`Plane(n,d)≡Plane(−n,−d)`,
  parameter-orientation changes) → C2 certified carrier equivalence → C3
  discovered local coincidence (2-D solution locus found by the solver).
  C0+C1 get most CAD-generated overlaps for free.
- **Regular SSI output is certified arcs between event boxes, not polylines.**
  `RegularContactArc { lhs_span, rhs_span, start, end,
  continuation: CertifiedCurveEvaluator, lhs_pcurve, rhs_pcurve,
  certificate }`. The claims are: exactly one branch exists here, no branch is
  missing, it connects these boundary/event cells. The sampled polyline is
  optional debug data.
- **S6 is rewritten around material state, not extended procedurally.**
  `material_transition(region) -> Certified<(State, State)>` with
  `State { in_a: bool, in_b: bool }`; retain iff
  `op.eval(minus) != op.eval(plus)`. union/intersection/difference/xor are
  different truth functions, and coincident faces stop being a special-case
  swamp.
- **Metamorphic tests live at every layer, not just S6.** Arrangement:
  `A(Γ) = A(permute Γ)`; contact: `C(A,B) ≅ C(B,A)`; Boolean: `A∪B=B∪A`,
  `A−A=∅`, `A∪A=A`; geometry: `T(A⋆B) = T(A)⋆T(B)` for rigid T.
  `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)` is the flagship cross-layer test.
- **S8 keeps `LocalOffsetRegular` and `GloballySelfIntersectionFree` as
  separate certificates.** Curvature only protects the local offset
  singularity (`|d| < 1/|κmax|`); global self-contact needs the
  non-incident/boundary separation terms. `fid::lfs::FaceScaleComponents`
  already carries `curvature_radius_lower`, `nonincident_separation_lower`,
  `boundary_distance_lower` as separate fields — the composition that turns
  them into a global certificate is the work.

## 3. Booked API surface — what exists today, with real names

Every signature below was read off the tree on 2026-08-24. **Re-derive by
`git grep`/`Select-String` before quoting one in a packet** — the spec goes
stale invisibly. Where a packet names one of these, copy the spelling exactly.

### 3.1 The evidence algebra (`truck-base/src/evidence.rs`, re-exported by truck-evidence as `outcome`)

```rust
pub type Outcome<T> = Result<Certified<T>, Refusal>;

pub struct Certified<T> {
    pub value: T,
    pub cert: Certificate,
}
impl<T> Certified<T> { pub const fn new(value: T, cert: Certificate) -> Self; }

pub enum Refusal {
    Empty,
    UnsupportedEnvelope(EnvelopeCase),
    NumericallyUnresolved { spent: Budget, witness: UnresolvedWitness },
    CompositionMarginExhausted(MarginWitness),
    InputOutsideBackwardBudget(RepairWitness),
    Contradictory(ContradictionWitness),
    Collapsed(Collapse, Certificate),
    ForwardToleranceExceeded { bound: f64, allowed: f64 },
}
pub enum EnvelopeCase { ChartDegenerate, ReachTooSmall, NonCanonicalCarrier, NonPositiveNurbsWeight }
pub enum UnresolvedWitness { UncertifiedContainment, RootNotIsolated, KrawczykIndeterminate, ContactCurveNotFound, DeviationUncertified }

pub struct Certificate {
    pub props: PropMap,      // π: Prop -> Truth
    pub method: Method,      // Exact | Interval | Float | None
    pub budget_left: Budget,
    pub margin: Margin,      // log2-scaled stability margin
    pub modulus: Modulus,
}
pub enum Method { Exact, Interval, Float, None }
pub enum Truth { Unknown, True, False, Both }
pub enum Prop { AnalyticCarrier, SoundEnclosure, Provisional, AnalyticPreserved,
    CoedgePairing, VertexLink, EulerPoincare, SameParameter, DomainBoundary,
    Representation, ToleranceMonotonicity, ShellNesting, WedgeNonDegeneracy }
pub struct PropMap { /* set/get/join */ }
pub struct Budget { pub subdiv: u32, pub newton: u32, pub depth: u32 }
impl Budget { pub fn new(subdiv, newton, depth) -> Self;
    pub fn spend_subdiv(&mut self, n: u32) -> Result<(), Exhausted>;
    pub fn spend_newton(&mut self, n: u32) -> Result<(), Exhausted>;
    pub fn spend_depth(&mut self) -> Result<(), Exhausted>; }
pub enum ModulusShape { Lipschitz(f64), Holder { k: f64, exponent: f64 }, Pole { k: f64 }, Unbounded }
```

### 3.2 The enclosure interface (`truck-evidence/src/enclosure.rs`)

```rust
pub use inari::Interval;                       // .inf()/.sup()/.mid()/.wid()/.mig()
pub struct Box3 { pub x: Interval, pub y: Interval, pub z: Interval }
impl Box3 { pub fn empty() -> Self; pub fn point(p: Point3) -> Self;
    pub fn contains(&self, p: Point3) -> bool; pub fn width(&self) -> f64; }
pub struct DirCone { pub axis: Vector3, pub half_angle: f64 }

pub trait EnclosureCurve: ParametricCurve<Point = Point3> {
    fn enclose(&self, tt: Interval) -> Box3;
    fn enclose_der(&self, n: usize, tt: Interval) -> Box3;
    fn tangent_cone(&self, tt: Interval) -> Option<DirCone>;
    fn exact_spline(&self) -> Option<BSplineCurve<Point3>> { None }  // default
}
pub trait EnclosureSurface: ParametricSurface<Point = Point3> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;
    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone>;
    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64;
    fn as_plane(&self) -> Option<&Plane> { None }   // default
}
// BG-ENC-004-OFFSET additions (landed 948a513):
pub trait EnclosureVectorField: ParametricSurface<Point = Vector3, Vector = Vector3> {
    fn enclose(&self, uu: Interval, vv: Interval) -> Box3;
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Box3;
}
pub trait EnclosureScalarField2 {
    fn enclose(&self, uu: Interval, vv: Interval) -> Interval;
    fn enclose_der(&self, m: usize, n: usize, uu: Interval, vv: Interval) -> Interval;
}
// pub(crate) helpers in this module: interval_at, cross_box, midpoint_ball_cone, immersion_lower_bound_box
```

### 3.3 Analytic pairs (`truck-evidence/src/analytic/`)

```rust
pub type PlacedCircle = Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>;
pub type PlacedParabola = Processor<TrimmedCurve<UnitParabola<Point3>>, Matrix4>;
pub type PlacedHyperbola = Processor<TrimmedCurve<UnitHyperbola<Point3>>, Matrix4>;

pub enum ExactCurve { Line(Line<Point3>), Circle(PlacedCircle), Ellipse(PlacedCircle),
    Parabola(PlacedParabola), Hyperbola(PlacedHyperbola) }

pub enum AnalyticIntersection {
    Curve(ExactCurve),              // transverse 1-D
    TwoCurves([ExactCurve; 2]),     // plane×cyl line pair, eq-radius cylinders, coaxial pair
    TangentPoint(Point3),           // tangent at one point
    TangentLine(Line<Point3>),      // tangent along a generator
    TangentCircle(PlacedCircle),    // counterbore / fillet families
    Parallel,                       // exact parallelism classification
    Coincident,                     // 2-D overlap — outside this track's contract
    Empty,                          // transverse placement, no intersection
}
pub type AnalyticOutcome = Outcome<AnalyticIntersection>;

// One entry per family (all present, all returning AnalyticOutcome):
pub fn plane_plane(p0: &Plane, p1: &Plane) -> AnalyticOutcome;
pub fn plane_sphere(p: &Plane, s: &Sphere) -> AnalyticOutcome;
pub fn sphere_sphere(s0: &Sphere, s1: &Sphere) -> AnalyticOutcome;
pub fn plane_cylinder(p: &Plane, c: &Cylinder) -> AnalyticOutcome;
pub fn plane_cone(p: &Plane, c: &Cone) -> AnalyticOutcome;
pub fn parallel_cylinders(c0: &Cylinder, c1: &Cylinder) -> AnalyticOutcome;
pub fn equal_radius_cylinders(/*...*/) -> AnalyticOutcome;
pub mod coaxial { pub enum CoaxialPair<'a> { /*...*/ }; pub fn coaxial(pair: &CoaxialPair) -> AnalyticOutcome; }
```

### 3.4 The certified numeric substrate (`truck-evidence/src/num/`)

```rust
// krawczyk.rs
pub trait KrawczykSystem<const N: usize> {
    fn f_point(&self, x: &[f64; N]) -> [Interval; N];
    fn jacobian(&self, b: &[Interval; N]) -> [[Interval; N]; N];   // ROW-MAJOR
    fn preconditioner(&self, x: &[f64; N]) -> Option<[[f64; N]; N]>;  // None => BISECT, never refuse
}
pub enum KrawczykProof { Unique, NoRoot }
pub fn krawczyk<const N: usize>(system: &impl KrawczykSystem<N>,
    start: &[Interval; N], budget: &mut Budget) -> Outcome<KrawczykProof>;
// AMENDED (session 32, BG-NUM-003-CONTRACT): the operator's contraction is
// the true matrix product (I − Y·J)[r][c] = δ(r,c) − Σ_k y[r][k]·j[k][c],
// not the Hadamard form δ(r,c) − y[r][c]·j[r][c] the original BG-NUM-003
// spec encoded. The two agree on diagonal Jacobians (all early users); the
// coupled slab systems of the general FF stage expose the difference.

// roots.rs
pub fn isolate_roots(coeffs: &[f64], domain: (f64, f64), tau: f64,
    budget: &mut Budget) -> Outcome<Vec<Interval>>;   // Bernstein form; one isolating interval per simple root

// cluster.rs (BG-NUM-004)
pub struct Cluster { pub members: Vec<usize>, pub center: Point3, pub enclosing_radius: f64 }
pub struct ClusterPolicy { pub eps: f64, pub tau_col: f64, pub scale_lower: Option<f64> }
pub fn cluster(points: &[Point3], radii: &[f64], policy: &ClusterPolicy,
    budget: &mut Budget) -> Outcome<Vec<Cluster>>;
```

### 3.5 Feature size / reach / isotopy (`truck-evidence/src/fid/`)

```rust
// lfs.rs — the S8 safe gate
pub struct FaceScaleComponents {
    pub curvature_radius_lower: f64,      // +inf allowed (flat cell)
    pub nonincident_separation_lower: f64, // d(cell image, exclusion boxes)
    pub boundary_distance_lower: f64,      // d(cell image, boundary boxes)
}
impl FaceScaleComponents { pub fn conservative_min(&self) -> f64; }
pub fn face_scale_components(surface: &impl EnclosureSurface, cell: (Interval, Interval),
    nonincident_boxes: &[Box3], boundary_boxes: &[Box3]) -> Result<FaceScaleComponents, FidRefusal>;
pub fn curvature_radius_lower(surface: &impl EnclosureSurface, cell: (Interval, Interval)) -> Result<f64, FidRefusal>;
pub struct WedgeSlopeLowerBound { /*...*/ }
pub fn wedge_slope_lower_from_sin_margin(/*...*/) -> ...;

// one_sheet.rs
pub enum FibreMultiplicity { /* one / multi / ... */ }
pub enum OneSheetError { SheetCountUnresolved, InvalidWitness /* ... */ }
pub fn fibre_degree_one(/*...*/) -> Result<FibreMultiplicity, OneSheetError>;
pub fn fibre_degree_one_auto(/*...*/) -> Result<FibreMultiplicity, OneSheetError>;

// isotopy.rs
pub enum CurveBoundary { Closed, Open }
pub struct CurveScaleComponents { /* conservative_min(), tube_scale_lower() */ }
pub struct IsotopyConditionsReport { /*...*/ }
pub enum IsotopyConditionsError { /*...*/ }
pub fn curve_isotopy_conditions(/*...*/) -> ...;
pub fn curvature_radius_lower_span(/*...*/) -> ...;
pub fn self_separation_lower_span(/*...*/) -> ...;

// rep.rs — the ONLY sanctioned exact→emitted path
pub enum RepError { InvalidMargin, ReachTooSmall, Unresolved { subdivisions: u32 } }
impl RepError { pub fn into_refusal(self) -> Refusal; }
pub struct HermiteCurve { /*...*/ }
pub struct RepCertificate { /*...*/ }
pub struct RepCurveOutput { pub curve: HermiteCurve, pub certificate: RepCertificate }
pub fn rep_curve(exact: &impl EnclosureCurve, boundary: CurveBoundary, tau_rep: f64,
    arc_gap: f64, initial_depth: u32, budget: &mut Budget) -> Result<RepCurveOutput, RepError>;

pub enum SurfaceBoundary { /*...*/ }
pub enum RepSurfaceError { /*...*/ }
pub struct SurfaceScaleComponents { /* tube_scale_lower() */ }
pub struct HermiteSurface { /*...*/ }
pub struct RepSurfaceCertificate { /*...*/ }
pub struct RepSurfaceOutput { /*...*/ }
pub fn rep_surface(exact: &impl EnclosureSurface, boundary: SurfaceBoundary, tau_rep: f64,
    gap: f64, initial_depth: u32, budget: &mut Budget) -> Result<RepSurfaceOutput, RepSurfaceError>;
```

### 3.6 Deviation certificate (`truck-evidence/src/deviation.rs`)

```rust
pub struct ParamMap { pub scale: f64, pub offset: f64 }   // phi(t) = scale*t + offset
impl ParamMap { pub const IDENTITY: Self; pub const fn flip(t0, t1) -> Self;
    pub fn from_ranges(a0, a1, b0, b1) -> Option<Self>; pub fn apply_f64(&self, t) -> f64;
    pub fn apply(&self, tt: Interval) -> Interval; }
pub fn certify_deviation<L: EnclosureCurve, C: EnclosureCurve>(
    leader: &L, carrier: &C, phi: ParamMap, tt: Interval, tau: f64,
    budget: &mut Budget) -> Outcome<f64>;   // whole-span; routes 1 (exact spline) + 2 (bisection)
```

### 3.7 Existing S6 machinery (`truck-shapeops/src/transversal/`)

```rust
// integrate.rs — current Boolean, transversal-only, returns Option (not Outcome)
pub trait ShapeOpsSurface: ParametricSurface3D + ParameterDivision2D
    + SearchParameter<D2, Point = Point3> + SearchNearestParameter<D2, Point = Point3>
    + Invertible + Send + Sync {}
pub trait ShapeOpsCurve<S: ShapeOpsSurface>: ParametricCurve3D
    + ParameterDivision1D<Point = Point3> + Cut + Invertible
    + From<IntersectionCurve<BSplineCurve<Point3>, S, S>>
    + SearchParameter<D1, Point = Point3> + SearchNearestParameter<D1, Point = Point3>
    + Send + Sync {}
pub fn and<C, S>(solid0: &Solid<Point3, C, S>, solid1: &Solid<Point3, C, S>, tol: f64)
    -> Option<Solid<Point3, C, S>>;
pub fn or<C, S>(solid0: &Solid<Point3, C, S>, solid1: &Solid<Point3, C, S>, tol: f64)
    -> Option<Solid<Point3, C, S>>;

// intersection_curve.rs — the current marcher (polyline-based; to be replaced by Contact)
pub struct IntersectionCurveWithParameters<S0, S1> { /* ic + params0 + params1 */ }
pub fn intersection_curves<S0, S1>(surface0: S0, polygon0: &PolygonMesh,
    surface1: S1, polygon1: &PolygonMesh) -> Option<Vec<IntersectionTuple<S0, S1>>>;

// divide_face.rs
pub fn divide_faces<C, S>(shell: &Shell<Point3, C, S>, loops_store: &LoopsStore<Point3, C>,
    tol: f64) -> Option<FacesClassification<Point3, C, S>>;

// faces_classification.rs — §12 propagation in embryo (the spec names it)
pub struct FacesClassification<P, C, S> { /*...*/ }
impl { pub fn push(&mut self, face, status: ShapesOpStatus);
    pub fn and_or_unknown(&self) -> [Shell<P, C, S>; 3];
    pub fn integrate_by_component(&mut self); }
```

### 3.8 Canonical carriers and topology (`truck-geometry/src/canonical.rs`, `truck-topology/src/lib.rs`)

```rust
pub enum Curve {
    Line(Line<Point3>),
    Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>),   // placed analytic circle
    BSplineCurve(BSplineCurve<Point3>),
    NurbsCurve(NurbsCurve<Vector4>),
    IntersectionCurve(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>),
}
pub enum Surface {
    Plane(Plane), Cylinder(Cylinder), Cone(Cone), Sphere(Sphere), Torus(Torus),
    RevolutedCurve(RevolutedCurve<Curve>),      // NOTE: variant exists; BG-CE-007 emits it
    ExtrudedCurve(ExtrudedCurve<Curve, Vector3>), // RESERVED, no conversion emits it yet
    BSplineSurface(BSplineSurface<Point3>),
    NurbsSurface(NurbsSurface<Vector4>),
    Processor(Processor<Box<Surface>, Matrix4>), // placed carrier; exact under affine
}

// truck-topology — Arc-handle persistence (do not flatten)
pub struct Vertex<P> { /* point: Arc<P> */ }
pub struct Edge<P, C, PC = ()> { /* vertices, orientation: bool, pcurve: Option<PC>, curve: Arc<C> */ }
pub struct Wire<P, C> { /* edge_list: VecDeque<Edge> */ }
pub struct Face<P, C, S> { /* boundaries: Vec<Wire>, orientation: bool, surface: Arc<S> */ }
pub struct Shell<P, C, S> { /* face_list: Vec<Face> */ }
pub struct Solid<P, C, S> { /* boundaries: Vec<Shell> */ }
// EntityId / Op / OpKind / Selector in truck-topology/src/entity_id.rs — BG-CE-003 identity
```

### 3.9 Decorators the plan builds on (`truck-geometry/src/decorators/`)

```rust
// Offset + NormalField (geometry carrier + evidence enclosure both landed)
pub struct Offset<T, N> { /* entity: T, offset: N */ }          // mod.rs:359
pub struct NormalField<T, F> { /* entity: T, scalar: F */ }     // mod.rs:366
impl<T, N> Offset<T, N> { pub const fn new(entity: T, offset: N) -> Self;
    pub const fn entity(&self) -> &T; pub const fn offset(&self) -> &N; }
impl<T, F> NormalField<T, F> { pub fn new(entity: T, scalar: F) -> Self;
    pub const fn entity(&self) -> &T; pub const fn scalar(&self) -> &F; }
// evidence impls: impl EnclosureScalarField2 for f64; impl<S,F> EnclosureVectorField for NormalField<S,F>;
//                 impl<S,N> EnclosureSurface for Offset<S,N>   (decorators/offset.rs, BG-ENC-004-OFFSET)

// Processor — the placed-carrier wrapper: Processor<E, T> with E the entity, T the transform
impl<E, T: One> Processor<E, T> { pub fn new(entity: E) -> Self; }
// TrimmedCurve<C> with `pub const fn new(curve: C, range: (f64, f64))`, `.curve()`, `.range()`
```

## 4. What does not exist yet (build order, with target signatures)

**Phase 0 — shared substrate (4-wide parallel packet wave).** All new modules;
no two packets share a write set.

- `truck-geometry` new module (recognizer): produce a witness, not a type.
  ```rust
  pub enum CanonicalCarrierWitness {
      ExactCanonical { carrier: CanonicalCarrier, map: ParamMap },
      Derived { carrier: CanonicalCarrier, provenance: ConstructionWitness, map: ParamMap },
      Unrecognized,
  }
  pub fn recognize_curve(c: &Curve) -> CanonicalCarrierWitness;    // tbd module path
  pub fn recognize_surface(s: &Surface) -> CanonicalCarrierWitness;
  ```
  `CanonicalCarrier` reuses the `Surface`/`Curve` enums' analytic arms. The
  `map: ParamMap` is the certified φ with `S_stored = S_canonical ∘ φ`.

- `truck-base` new module (`bvh`): wide flat-array BVH over `Box3`.
  ```rust
  pub trait BoundedPiece { fn bbox(&self) -> Box3;
      fn derivative_bounds(&self) -> DerivativeBounds;    // tbd small struct
      fn subdivide(&self) -> Vec<Self> where Self: Sized; }
  pub struct Bvh<P: BoundedPiece> { /* flat node array, contiguous leaves */ }
  impl<P: BoundedPiece> Bvh<P> {
      pub fn build(pieces: &[P]) -> Self;
      pub fn candidate_pairs(&self, other: &Self) -> Vec<(usize, usize)>;  // BVH box overlap
  }
  ```

- `truck-geometry` new module (`span`): lazy rational-Bézier span extraction.
  ```rust
  pub struct SpanCache { /* per-carrier lazy extraction */ }
  impl SpanCache {
      pub fn spans(&self, s: &Surface) -> Vec<SpanRecord>;    // tbd: cached
  }
  pub struct SpanRecord { pub bbox: Box3, pub derivative_hull: DerivativeBounds, /* knot range */ }
  ```

- `truck-base` new module (`pred`): certified predicates with escalation.
  ```rust
  pub enum CertifiedPred { Proven, Unresolved(UnresolvedWitness) }
  pub fn orient2d(a: Point2, b: Point2, c: Point2) -> CertifiedPred;   // exact; adaptive escalation
  ```

**Phase 1 — S1 planar arrangement.** New `truck-geometry` (or `truck-base`)
module `arrange`. The `CurveContact` ontology lands here (types in Phase 0).
```rust
pub enum ContactDimension { Point0, Arc1, Region2 }
pub enum ContactEventKind { Transverse, Tangency, EndpointTouch, CoincidentInterval, IdenticalCarrier }
pub struct CurveContact { pub dimension: ContactDimension, pub kind: ContactEventKind, /* params */ }

pub struct Arrangement {
    pub vertices: Vec<ArrVertex>,
    pub half_edges: Vec<ArrHalfEdge>,
    pub regions: Vec<ArrRegion>,
    // quotient: Option<QuotientWitness>   — Option on M1; do not over-promise
}
pub fn arrange(profile: &[Curve], domain: Option<Box2>) -> Outcome<Arrangement>;
```
Internal: x-monotone split → sweep → intersections → half-edge/DCEL → faces
→ winding. Analytic curve/curve first (`orient2d`-based), spline pairs via
`num::roots`/`num::krawczyk`.

**Phase 2 — S2 direct B-rep constructors.** `truck-modeling` additions.
```rust
// AMENDED (session 28, SPEC_GAP): the landed S1 `Arrangement` carries no
// carrier geometry — ArrHalfEdge.curve is an index into the profile slice, and
// a full circle is not determined by its seam vertex + 2pi window — so the
// profile is a second argument. The §4 signature below is superseded.
pub fn extrude_profile(profile: &[Curve], arrangement: &Arrangement, height: f64) -> Outcome<Solid<Point3, Curve, Surface>>;
pub fn revolve_profile(profile: &[Curve], arrangement: &Arrangement, axis: Line<Point3>, angle: f64) -> Outcome<Solid<Point3, Curve, Surface>>;
// canonicalization: recognize (circle × straight path) => Cylinder etc.
```
No tool-body Boolean. n side faces + 1 top + 1 bottom + shared edges.

**AMENDED (session 30, SPEC_GAP, BG-SOL-S2-PCURVE):** pcurves on the returned
`Solid`'s edges are NOT deliverable by S2. The landed topology erases the
pcurve payload at the Wire boundary: `Wire<P,C>` holds `VecDeque<Edge<P,C>>`
(`PC = ()` by default), and `with_pcurve<Q>` returns `Edge<P,C,Q>`, so a real
`PCurve` edge cannot enter a Wire (compile probe E0277). Delivering pcurves
requires threading `PC` through `Wire`/`Face`/`Shell`/`Solid` — a cross-crate
topology-wide type change touching every `Wire` use across meshalgo/shapeops/
modeling/stepio, which the spec's BG-CE-001 record anticipated ("the packet
that wires real pcurves owns trace splitting"). Deferred to that topology-PC
program; it is not an S2 follow-up.

**Phase 3 — Contact Layer.** New `truck-evidence` module `contact`.
```rust
// AMENDED (session 30, BG-SOL-S3-CONTACT): the landed strata are geometry-side
// and take references — `contact` inspects both strata before constructing and
// the Boundary Rewrite iterates every pair, so `&BoundedStratum` (the plan's
// by-value booking was infeasible: CanonicalSurface is not Copy).
// `BoundedStratum::Face` carries a `CanonicalSurface` (recognizer), which has
// NO Unrecognized arm — an unrecognized/spline carrier is refused at the
// `face_stratum` lift boundary, not in `contact()`. The deferral funnel uses a
// new `EnvelopeCase::ContactReductionDeferred` arm.
pub enum BoundedStratum { Face { surface: CanonicalSurface, u_range, v_range },
    Edge { curve: CanonicalCurve, t_range }, Vertex { point: Point3 } }
pub struct ContactComplex { pub contacts: Vec<ContactRecord> }   // ContactRecord { dimension, kind, locus }
pub enum ContactLocus { Coincident, Analytic(AnalyticIntersection) }
pub fn contact(lhs: &BoundedStratum, rhs: &BoundedStratum, budget: &mut Budget)
    -> Outcome<ContactComplex>;
```
Dispatch: identity/overlap (C0-C2) → analytic pairs (exists, §3.3) → strata
reductions (FE, EE via curve machinery) → general validated FF → singular
event cells → 2-D overlap (C3/C4, last). Landed stages (session 30): C0-C2
identity (struct-equal canonical carriers) + the FF analytic table
(plane_plane/plane_sphere/sphere_sphere/plane_cylinder/plane_cone, both
orientations). FE/EE, cylinder×cylinder and other analytic-pair families
outside the table, Torus/Placed, and 2-D overlap all refuse with
`ContactReductionDeferred`; the next packets fill the stages they own.

**AMENDED (session 31, BG-SOL-S4-FE-EE, the strata-reduction stage):** the FE
(Edge×Face) and EE (Edge×Edge) stage dispatches from `contact()` and builds the
**bounded locus forms** the skeleton deferred. Two new `ContactLocus` arms:
`Point(Point3)` for isolated Point0 contacts (FE punctures, EE crossings) and
`BoundedCurve { curve: ExactCurve, t_range: (f64, f64) }` for the Arc1
coincident sub-arcs (an edge lying on a face, overlapping collinear edges),
`t_range` in the curve's own parameterization. `ContactRecord { dimension,
kind, locus }` is unchanged; parameter bookkeeping for edge/face splitting is
the Boundary Rewrite's (Phase 4), not this stage's. Module shape: `contact.rs`
becomes the directory module `contact/mod.rs` (vocabulary + dispatcher) with a
new sibling `contact/fe_ee.rs` holding the FE/EE solvers, so later funnel
packets extend the Contact Layer without colliding on the dispatcher file.
The stage's analytic table (both orientations through one solver, satisfying
the metamorphic `C(A,B) ≅ C(B,A)` test): FE `Line` edge × `Plane`/`Cylinder`
face (linear / quadratic solves, decisive-interval predicates), FE `Circle`
edge × `Plane` face (chord solve on the two planes' meet-line, or coincident
Arc1 when the planes coincide, clipped to the face box), FE `Circle` edge ×
`Cylinder` face **latitudinal-coincident only** (same-axis detection →
Arc1), EE `Line` × `Line` (skew/parallel-empty/coplanar-point/coincident-arc),
EE `Line` × `Circle` (transverse on-circle test or in-plane chord). Every
reported point or arc is checked against BOTH strata's bounds (edge `t_range`
and the face `(u, v)` box; cylinder u wraps into `[0, 2π)`). Everything else —
FE `Line`×{Cone,Sphere}, `Circle`×{Cone,Sphere}, Circle×Cylinder transverse,
EE Circle×Circle (3-D two-circle), Torus/Placed, vertex strata — keeps the
`ContactReductionDeferred` refusal with its documented follow-up.

**AMENDED (session 31, BG-SOL-S5-CYLPAIR, the cylinder-family FF cells):** the
curved × curved FF cells that canonical carriers make reachable dispatch
through the §3.3 `parallel_cylinders` / `coaxial` families inside
`analytic_ff`. Because every canonical `Cylinder`/`Cone`/`Sphere` is z-axis-
aligned, any two of them have **parallel** axes, so the pair is either coaxial
(exact `(x, y)` axis-position equality, matching `CoaxialPair::validate`) or
parallel-but-offset. Dispatch: `(Cylinder, Cylinder)` → coaxial `CylCyl` on
same-axis, `parallel_cylinders` otherwise; `(Cylinder, Cone)`,
`(Cylinder, Sphere)`, `(Cone, Cone)`, `(Cone, Sphere)` (both orientations) →
the corresponding `CoaxialPair` cell on same-axis, `ContactReductionDeferred`
otherwise. **`equal_radius_cylinders` is NOT wired**: its intersecting-axes
cell is unreachable from canonical carriers (parallel axes by construction) and
needs `Placed` cylinders, which the funnel defers; the family stays the
analytic-cell oracle for BG-NUM-003. `Torus` pairs stay deferred. All returned
arms flow through the existing `analytic_records` mapping (no new locus forms).

**AMENDED (session 32, BG-SOL-S6-IMPLICIT, the general-validated-FF substrate
stage):** the general validated FF stage is split. Its first packet lands ONLY
the shared primitive every later formulation needs:
`contact/implicit.rs` in truck-evidence — trait `ImplicitField { implicit(&Box3)
-> Interval, grad(&Box3) -> [Interval; 3], regular_on(&Box3) -> bool }` over
the FIVE BARE carriers (Plane, Cylinder, Cone, Sphere, Torus) with documented
sign conventions (plane: signed distance; quadrics: negative inside; cone via
`x'²+y'²−(z't)²`, apex on the zero set with ∇f=0 there; torus via the
sqrt-free quartic `(g)²−4R²h`). `regular_on` is deliberately one-sided
(true = proven regular; false = not proven). No `CanonicalSurface`/`Placed`
impl: the dispatcher refuses `Placed` upstream and the GFF solver will match
the enum itself. The solver stages that consume it (offset mixed quadrics,
Torus pairs, event finding + Krawczyk arc continuation, singular cells) are
separate packets.

**AMENDED (session 32, BG-SOL-S7-GFF-COVER, the branch-cover engine):** the
second GFF substrate packet lands `contact/gff.rs`:
`cover_branch(f1, f2, domain, tau, budget) -> Outcome<BranchCover>` decomposes
a 3-D search box for two implicit fields into proven crossings, proven-singular
boxes, and an honestly-typed unresolved remainder under `tau`/budget.
Deterministic widest-axis bisection. No dispatcher wiring, no new locus arms —
the wiring packet (locus representation for non-exact arcs) follows once the
cover is proven on the offset mixed quadric pairs.

**AMENDMENT r2 (session 32):** the r1 probe — a 3×3 augmented Krawczyk system
`[f1, f2, g·(p−m)]` with a full 3×3 inverse — returned SPEC_GAP at 836b704:
it could not certify ANY crossing of the transversal sphere/cylinder pair
(`NumericallyUnresolved` after 4096 subdivisions). Reading `k_image` showed
the operator IS the correct full-matrix Krawczyk (not diagonal-only, as the
worker first claimed); the defect was formulation conditioning. r2 replaces
the probe with the **2×2 z-slab system** `F(x,y) = [f1(x,y,z0), f2(x,y,z0)]`
per z-leaf, exact 2×2 closed-form inverse preconditioner, singular screen on
the slab-Jacobian determinant (`det = 4(y·cx − x·cy)` for cylinder×sphere —
non-singular exactly off the singular locus). This is the certified branch-
cover formulation that wiring will consume.

**AMENDED (session 34, BG-SOL-S7-GFF-WIRE, the first dispatcher wiring):**
the branch cover is wired into the four offset mixed-quadric FF families
(`Cylinder×Cone`, `Cylinder×Sphere`, `Cone×Cone`, `Cone×Sphere`, both
orientations). The intermediate locus is deliberately
`ContactLocus::ValidatedBranchCover(BranchCover)`, not a point vector and not
an `ExactCurve`: `BranchCover` proves a deterministic set of regular
cross-sections but does not yet prove connectivity or component ordering. A
cover is returned as an `Arc1`/`Transverse` record only when both its singular
and unresolved boxes are empty. Singular boxes stay in the deferred funnel for
the singular-event stage; unresolved boxes produce a typed
`NumericallyUnresolved` refusal. The finite world-space search domain is the
intersection of the two carriers' certified `EnclosureSurface` boxes over the
bounded face parameter ranges. As with the landed analytic FF stage, exact
trimming/component splitting remains Phase 4 Boundary Rewrite work. `Placed`
and `Torus` stay deferred. Resolution is scale-relative to the finite search
box; budget spend remains caller-controlled.

**AMENDED (session 35, BG-SOL-S7-GFF-CHART, chart-artifact recovery before
true singular classification):** `BranchCover::singular_boxes` currently means
the fixed z-slab Jacobian minor contains zero, not that the full surface
gradients are rank-deficient. The xy minor is the z component of
`grad(f1) cross grad(f2)`; a regular intersection whose tangent turns
horizontal can therefore be mislabeled singular even when the xz or yz minor
decisively excludes zero. Before classifying singular contact loci, generalize
the slab probe to select a deterministic regular coordinate chart from all
three 2x2 minors over the finite domain. Choose the minor with the largest
certified distance from zero (stable axis tie-break); slice along its omitted
coordinate and solve in the other two. Only a box for which no minor excludes
zero remains a singular candidate. This packet does not claim those candidates
are truly rank-deficient, connect arcs, or classify isolated/curve/region
loci; those are the following singular-event packets.

**AMENDED (session 35, BG-SOL-S7-SING-*, the singular-event stage):** post-
CHART, a `singular_boxes` entry means all three cross-gradient minors merely
contain zero over the unsubdivided domain — provable-or-suspected, never
proven rank deficiency. The stage is two packets. **SING-SUBSTRATE** extends
`ImplicitField` with sound `hess(&Box3) -> [[Interval; 3]; 3]` enclosures
(constant for the quadrics — plane 0, sphere 2I, cylinder diag(2,2,0), cone
diag(2,2,−2t²); torus quartic `2∇g∇gᵀ + 4gI − 8R²diag(1,1,0)`) and
`degenerate_points() -> Vec<Point3>` (exact isolated on-surface ∇f=0 points:
cone apex only; the torus r=R/2 inner-equator circle is a positive-dimensional
degenerate locus the method deliberately does NOT enumerate — documented in
its contract). **SING-CLASSIFY** lands `contact/singular.rs`:
`singular_events(f1, f2, cells, tau, budget)` refines each singular cell
(children that field-exclude drop; children that chart-certify go through
`cover_branch`, which re-selects a chart per child — the per-child chart use
CHART deliberately deferred from its own scope; connectivity stays unclaimed);
each resolution-floor residue leaf then classifies by (a) exact degenerate
points tested against the other carrier, (b) the 4-D Lagrange system
`[f1, ∇f2 + λ∇f1]` with the sound λ-envelope `±sup|∇f2| / min|∇f1|` — the
direct 3×3 `[f1, f2, T]` formulation is unusable because its Jacobian is
singular at every tangency and `krawczyk` correctly refuses double roots; the
Lagrange system is the standard regularization and is nonsingular at
nondegenerate tangencies — then (c) restricted-Hessian inertia of
`Hess(f2) + λ*·Hess(f1)` on the certified root's tangent plane: definite →
isolated tangency → `Point0`/`Tangency` record; indefinite → **tangential
crossing** (the contact locus self-crosses at a gradient-parallel saddle —
real in-family: cyl×sphere internal tangency at axis distance R−1 pinches the
exit curve through itself) → deferred with the certified point recorded.
Degenerate contacts and tangential crossings stay deferred: their local
branch topology is Boundary-Rewrite vocabulary, not this stage's. Residue
that certifies nothing stays deferred: one- and two-dimensional singular loci
(tangency along a curve, coincident patches) are NOT claimed — for the wired
offset mixed-quadric families Region2 coincidence is structurally unreachable
(distinct quadric types cannot share a 2-D patch) and Arc1 tangency is
coaxial-only (upstream), but the stage defers rather than hardcoding that
belief. The dimension split lives in the report type: `tangencies` (proven
isolated Point0), `tangential_crossings` (proven saddle points, locus not
isolated), `degenerate` (proven carrier-degenerate contact points),
`residue` (dimension unknown). Singular handling is not a tangent-point
special case: only the isolated arm becomes a record, and every other arm
defers with a named reason.

**AMENDED (session 35, BG-SOL-S7-OVERLAP, the 2-D overlap screen):** the
landed coincident paths overclaim: BOTH the struct-equal identity arms and
the analytic `Coincident` cells emit Region2/Arc1 records without screening
the parameter boxes, so two DISJOINT patches of the same canonical carrier
(a shared wall's two sides, two same-wall cylinder patches at different
heights, two disjoint arcs of one circle) report contact today. The overlap
screen decides, per coincident path, whether the patches' parameter boxes
overlap with non-empty interior: only then is the Coincident record emitted;
otherwise the stage returns a certified empty complex (exact method, no
budget). Boundary-only contact (boxes touching at an edge/corner) is
intentionally empty here — shared-boundary contact is owned by the FE/EE
stages' own strata pairs. The screen is exact-f64 arithmetic on stored
analytic data (the BG-ANA-002 5.1 decision class: sub-ulp boundary
configurations may decide either way; test witnesses are dyadic). Covered:
the identity arms for all five carriers with per-carrier periodicity
(cylinder/cone: u wraps 2π, v aperiodic; sphere: u polar aperiodic, v wraps
2π; torus: both wrap; plane: neither), the Edge×Edge identity arm (line
plain, circle wrap on TAU), the coaxial same-radius Cylinder×Cylinder
C1 case (same (cx, cy, r), any cz: u identical, v shifted by cz2 − cz1),
and struct-unequal coplanar Plane×Plane with PARALLEL frames (the
construction-data case: exact 2×2 solve per endpoint). Rotated-frame
coplanar planes (general affine map → 3-D SAT with sound interval
projections) are the named follow-up `BG-SOL-S7-OVERLAP-PLANE` packet and
keep today's unscreened emission until it lands.

**Phase 4 — Boundary Rewrite.** New `truck-shapeops` module; material-state
heart (spec §13.1).
```rust
pub struct State { pub in_a: bool, pub in_b: bool }
pub enum BoolOp { Union, Intersection, Difference, Xor }
impl BoolOp { pub fn eval(&self, s: State) -> bool; }
pub fn material_transition(region: &AtomicFaceRegion) -> Outcome<(State, State)>;
pub fn boolean(a: &Solid<Point3, Curve, Surface>, op: BoolOp, b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget) -> Outcome<Solid<Point3, Curve, Surface>>;
```

**Phase 5 — S8 safe shell.** `truck-shapeops`/`truck-modeling`.
```rust
pub struct OffsetCertificates { pub local: LocalOffsetRegular, pub global: GloballySelfIntersectionFree }
pub fn shell(body: &Solid<Point3, Curve, Surface>, d: f64, budget: &mut Budget)
    -> Outcome<Solid<Point3, Curve, Surface>>;   // safe gate: |d| < FaceScaleComponents::conservative_min()
```

**Phase 6 — S7/S3/S4.** Local `N(e,r)` fillet scoping; RMF sweep with
curvature/clearance gates; minimal-knot loft (`AX=B`, factor A once, solve
many RHS with a banded factorization).

**Production total ~28k new LOC; tests/witnesses ~13k; ~41k overall.**

## 5. Dependency graph — the parallelizable form

```
                 Phase 0  (4-wide packet wave: witness │ span-cache │ BVH │ predicates │ CurveContact types)
                     │            │           │         │
       ┌─────────────┼────────────┼─────────┬─┴─────────┐
       ▼             ▼            ▼         ▼           ▼
     S1 Arrange    S2 BREP     S8-safe    S4 Loft    Contact-analytic+strata
     (2D)          (direct,    (shell,    (standalone (identity, analytic pairs,
       │           parallel     no-3D      solver)    curve/face, curve/curve)
       │            to S1)      Boolean)
       │             │           │            │            │
       │             └───────────┼────────────┼─────── Contact general FF + overlap
       │                         │            │              (the funnel)
       └─────────── M1 ◀─────────┘            │              │
           (2D plate-with-hole)               │              ▼
                                              │       Boundary Rewrite (S6)
                                              │              │
                                              └──────── S7/S3 fallback + S8 self-intersection
```

Critical path: Phase 0 → S1 → Contact-general → Boundary Rewrite. Everything
else fills the sides and feeds M1 without lengthening it. Strategy: delay the
funnel (S5.3 general FF + overlap) as long as possible.

## 6. Loop-level execution rules

- **The scheduler's real law is write-set disjointness** (`loop/schedule.py`),
  not logical `needs`. Two packets that collide on a file are discovered only
  at merge, after both workers are paid.
- **Each solver family / substrate service gets its own module file.** No two
  packets in one wave may touch `canonical.rs` or `enclosure.rs`. This is the
  single design rule that keeps the graph parallel.
- **Slots are a disk budget, not a cap of 3.** The scripts discover slots by
  scanning `loop/slots/`; `new_slot.py --slot N` takes any integer. Each warm
  slot is ~2–3 GB and each verify baseline ~1.3 GB transiently. Fork as many
  as disk allows; a 4-wide Phase 0 wave needs slot 3+.
- **Same-base verifies run sequentially** (they race the baseline cache).
- **Slots = desired worker concurrency, reused per packet via `new_slot.py`.**
  Not one slot per packet.

## 7. Milestones

**M1 — Certified planar construction.** `rectangle − circle` → 2-D arrangement
→ profile with hole → direct extrude → valid B-rep (plate with cylindrical
hole), with no 3-D Boolean at all. Exercises recognizer, predicates, analytic
intersections, arrangement, material state, topology construction, canonicalization,
and — since BG-SOL-S2-PCURVE (session 30) proved pcurves cannot ride on the
returned `Solid`'s edges — the pcurve **carrier** and its invariant layer:
`PCurve<C, S>` exists in truck-geometry and truck-topology's BG-INV-001
same-parameter checker certifies pcurve-carrying edges on standalone edges.
M1's "exercises pcurves" is recorded as satisfied by the S2 construction plus
the pcurve carrier / same-parameter invariant existing; attaching real pcurves
to a `Solid`'s edges is re-scoped to the future topology-PC program (see §4
Phase 2). Establishes `Extrude(P−Q) ≅ Extrude(P)−Extrude(Q)` as the
flagship differential test. ~8–9k LOC.

**M2 — 3-D contact/Boolean.** `extruded_solid − cylinder` produces the same
canonical answer via the 3-D contact path, and the M1 relation is checked
cross-layer. Requires the Contact Layer + Boundary Rewrite.

## 8. Open items carried from the base loop

- The recommended (not dispatched) follow-up audit of
  `truck-evidence/src/fid/rep.rs` (4362 lines; the sole sanctioned exact→
  emitted path). Do it as its own program, not inside solver-phase packets.
- Corpus measurement (spec P-7) gates the *density* parameters of the
  tangency/contact cells, not the L0/L1 recognition. The parallel tracks above
  do not wait on it.
