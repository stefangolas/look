# BG-CK-P1-DISPATCH — class 2 fast path: certified analytic pair dispatch (framework + exact arms)

Certified-kernel Phase 1, fourth packet — plan §2 class 2
(`docs/CERTIFIED_PHASE1_BOOKING.md`). The dispatcher routes certified
surface-pair classes to closed-form certified contact constructions, with
exact predicates deciding admission and directed rounding at the evaluation
leaves. The landed `formal/intersection.rs` 2D pipeline is the
implementation model; the landed 2D result shape (`PairIntersectionResult`
→ `PairContactResult`, `formal/contact.rs`) is the shape this module's
result mirrors.

**Admission is mass-driven and measured** (the plan's own doctrine —
prevalence decides). Corpus pair masses (`docs/CERTIFIED_PREVALENCE.md`):
cylinder~plane 37,361 + plane~plane 26,274 + cylinder~cylinder 5,354 +
plane~sphere 281 + sphere~sphere 126 — the arms THIS packet lands carry
**64,042 pairs before the coaxial/parallel subset of cylinder~cylinder
counts, ~62% of the analytic mass**. The cone and torus arms
(plane~cone 8,379; plane~torus 5,385; cylinder~sphere 3,249) are
certifiable only in special geometric positions and book as
BG-CK-P1-DISPATCH-2 after FLOOR's first measurement
(velocity-recalibration doctrine). Out-of-admitted-set classes refuse
typed — never swallowed, never downgraded (the no-silent-downgrade
doctrine). Zero mesh-derived intersection polylines in the certified path
(F1: certified loci, never approximations).

```yaml
id:          BG-CK-P1-DISPATCH
contract:    [BG-CK-P1-DISPATCH]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P0-FREEZE, BG-CK-P1-HULL, BG-CK-P1-SPHERE]
write_allow:
  - vendor/truck/truck-certified/src/pair_dispatch.rs
  - vendor/truck/truck-certified/src/formal/intersection.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/pair_dispatch_conformance.rs
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE1_BOOKING.md
  - docs/CERTIFIED_PREVALENCE.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/sphere.rs
  - vendor/truck/truck-certified/src/formal/support.rs
  - vendor/truck/truck-certified/src/formal/cylinder.rs
  - vendor/truck/truck-certified/src/formal/cone.rs
  - vendor/truck/truck-certified/src/formal/torus.rs
  - vendor/truck/truck-certified/src/formal/intersection.rs
  - vendor/truck/truck-certified/src/formal/contact.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum PairUnsupported' vendor/truck/truck-certified/src/formal/intersection.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum PairContactResult' vendor/truck/truck-certified/src/formal/contact.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct CertifiedEmbeddedSphere' vendor/truck/truck-certified/src/formal/sphere.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub struct PlaneSchema' vendor/truck/truck-certified/src/formal/support.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct CertifiedEmbeddedCylinder' vendor/truck/truck-certified/src/formal/cylinder.rs"}
  - {id: A6, expect: 0, cmd: "grep -rnw 'UnsupportedPairClass' vendor/truck/truck-certified/src | wc -l"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn exact_sq_dist' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A8, expect: 0, cmd: "grep -c 'pub mod pair_dispatch;' vendor/truck/truck-certified/src/lib.rs"}
tests_required:
  - transverse_planes_emit_certified_line
  - distinct_parallel_planes_are_disjoint_and_coincident_refuses_overlap
  - plane_cylinder_transverse_emits_circle
  - plane_cylinder_tangent_emits_generatrix_line_and_offset_is_disjoint
  - plane_sphere_transverse_emits_circle_with_enclosing_radius
  - tangent_sphere_plane_emits_point
  - sphere_sphere_transverse_emits_radical_circle_and_tangent_emits_point
  - coaxial_cylinders_emit_circle_and_equal_radius_refuses_overlap
  - unsupported_pair_class_refuses_named_case
  - operand_swap_yields_the_sorted_canonical_answer
  - dispatch_never_panics_and_admission_is_exact_predicate_decided
```

## Pre-made decisions (do not relitigate; quote the tags into the module doc)

**H-1.** Crate-level `#![deny(clippy::unwrap_used)]` covers the new module.
NO `unwrap`/`expect`/`panic!`, NO module-level `allow`. `Option`s unpack
through `ok_or(...)?` into named refusals.

**D-reuse — the refusal class is the LANDED enum, widened by exactly one
named variant.** `formal::PairUnsupported` (Overlap / UnrelatedTangency /
CoincidentCircles) is the shared pair-refusal witness across the 2D and
generic pipelines. This packet adds ONE variant, pre-named:

```rust
/// The pair class (or configuration) is outside the dispatcher's admitted
/// set. The typed no-silent-downgrade boundary: a refusal here is a class
/// line, not a failure — Phase 2's generic path owns what this refuses.
UnsupportedPairClass,
```

with tag `"pair_unsupported_class"`. This is a certified-layer-local
widening booked per `docs/CERTIFICATE_MAPPING.md` section C row 1 (failure
witnesses live in `truck-certified`; no top-level `Refusal` variant, base
evidence untouched). `contract::Refusal` stays frozen. Existing match
sites on `PairUnsupported` (intersection.rs, contact.rs) gain the new arm
— those files' diffs are the minimal match-arm additions, nothing else.

**D-result — a row-3 result type, not a witness-edge.** Mapping section C
row 2 (witness-edge) is for certified shell EDGES; a derived pair contact
is row-3 branch geometry "carried as a result, not annotated onto shell
evidence". The module defines `CertifiedPairContact`/`CertifiedPairResult`
shaped like the landed `PairContactResult`
(Disjoint / Contact / Unsupported(PairUnsupported) / Unresolved — reuse
`PairUnsupported` verbatim; Unresolved uses `GenericUnresolved` already in
`formal/contact.rs`). The contact locus is family-tagged, WORLD-space,
representation-derived certified geometry:

```rust
/// The certified contact locus of an admitted pair. Raw-frame doctrine:
/// directions are the surfaces' OWN axes (never orthogonalised, never
/// normalised downstream — the identify_plane retained-basis rule).
pub enum ContactLocus {
    /// A shared line: point on the line + direction (raw magnitude).
    Line { point: Point3, direction: Vector3 },
    /// A shared circle: center, axis direction (raw), and a certified
    /// enclosure of the radius (the sqrt path is enclosure-valued, not
    /// exact).
    Circle { center: Point3, axis: Vector3, radius: CertifiedInterval },
    /// A single tangent point.
    Point { point: Point3 },
}
```

Chart (pcurve) emission is NOT in this packet — no Phase-1 consumer needs
it (FLOOR measures certify/refuse, it does not consume pcurves); it books
with Phase 3's boolean core as a follow-up. Say so in the module doc.

**D-sorted — operand order is canonical.** Mirror the landed
`canonical_sides` discipline (`formal/contact.rs`): the pair is sorted by
participant identity and `dispatch(a, b) == dispatch(b, a)` (a required
test). Identity: the participant enum's discriminant order plus, within a
class, the witness's representation-derived geometry ordering (state the
exact comparator — deterministic, no hash order; coordinates break ties
lexicographically).

**D-exact — admission is exact-predicate-decided.** Every admission screen
(the geometric configuration test that decides which closed form applies)
is decided through `formal/exact.rs` exact arithmetic on the witnesses'
representation-derived `f64` coordinates (`Expansion` sign decisions via
`exact_sq_dist` / `exact_dot2` / `cross_exp` and their obvious
extensions built from the same primitives) — never a floating-point
epsilon comparison, never an interval straddle at ADMISSION time. The
VALUES of the emitted locus may be enclosure-valued (`Circle`'s radius
through `CertifiedInterval::sqrt`); the DECISIONS are exact. A
configuration the screens cannot name refuses `UnsupportedPairClass`.

**D-routing — one participant enum, built from the landed witnesses.**

```rust
/// One side of a dispatched pair: the certified witness of an identified
/// analytic surface. Constructed from the landed identification enums.
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedPairParticipant {
    Plane(PlaneSchema),
    Cylinder(CertifiedEmbeddedCylinder),
    Sphere(CertifiedEmbeddedSphere),
}
```

Cone and torus witnesses are KNOWN to the routing (the enum gains the
variant in DISPATCH-2); in this packet a cone/torus side refuses
`UnsupportedPairClass` — the enum does not carry variants it cannot
dispatch. The from-identification constructors (e.g.
`from_support_schema`, `from_cylinder_identification`,
`from_sphere_identification`) map the landed `NotA*` arms to `None` and
the certified arm to `Some(...)`.

## The admitted arms (per-arm specification)

Each arm: the admission screen (exact), the emitted locus, and the
refusal mapping. Angles/parallelism are decided by exact expansion sign
tests on cross/dot products of the raw representation vectors (the
cross/dot EXPANSIONS, not their f64 approximations — `cross_exp` and
`exact_dot2` are the primitives; build the 3-D dot/cross expansions the
same way `exact_dot2` builds the 2-D one).

1. **plane~plane (26,274).** Normals cross to zero (parallel) vs not
   (transverse), then offset test. Transverse → `Line` (the classic
   point+direction construction, evaluated with directed rounding).
   Parallel distinct → `Disjoint`. Parallel coincident (same plane:
   point-on-plane exact test) → `Unsupported(Overlap)` (the landed
   variant — a positive-length shared region, the 2D pipeline's own
   meaning).
2. **plane~cylinder (37,361).** Axis vs plane normal exact test.
   Transverse → `Circle` (center = axis∩plane, radius = cylinder radius
   exactly — a transverse plane cuts a cylinder in its own radius
   circle... NO: only if the plane is ⊥ the axis. General transverse
   plane cuts an ELLIPSE). **Ellipse is NOT a `ContactLocus` variant and
   is not certifiable closed-form here — the admitted configuration is
   the axis-normal plane only** (exact test: axis × plane-normal cross
   expansion is zero AND axis dot plane-normal nonzero). Perpendicular
   → `Circle` with the cylinder's exact radius. Tangent-parallel (axis
   in the plane direction... the plane parallel to the axis) → one
   generatrix `Line` if the plane touches (distance-to-axis exact test
   equals radius), `Disjoint` if it misses. All other configurations →
   `UnsupportedPairClass`. (The general ellipse cut books with
   DISPATCH-2 alongside cone~plane — one rational-conic machinery
   serves both.)
3. **plane~sphere (281).** Exact squared distance from center to plane
   (`exact_sq_dist`-class expansion) vs r² exact. Less → `Circle`
   (center = foot of perpendicular; radius enclosure through
   `CertifiedInterval::sqrt` of the exact difference's interval image —
   the enclosure must CONTAIN the true radius; the packet's test
   asserts containment against a brute-force ulp bracket). Equal →
   `Point`. Greater → `Disjoint`.
4. **sphere~sphere (126).** Exact `|c1−c2|²` vs `(r1±r2)²` all-exact
   (`Expansion` on the representation coordinates; radii are
   `PositiveFinite` f64). Strictly between → `Circle` (radical-plane
   circle; radius enclosure via `sqrt` as in arm 3). Equal to the sum →
   `Point` (external tangency); equal to the difference → `Point`
   (internal tangency); beyond either → `Disjoint`; same center and
   same radius → `Unsupported(CoincidentCircles)` is the 2D enum's
   closest cause but wrong domain — same-center-same-radius spheres
   refuse `UnsupportedPairClass` (a coincident-sphere pair is not a
   curve contact; the boolean layer's coincidence handling owns it).
   Say this asymmetry explicitly in the module doc.
5. **cylinder~cylinder (5,354; admitted subset).** Axes parallel exact
   test. Parallel + collinear (coaxial): radii equal →
   `UnsupportedPairClass` (coincident cylinder faces — same doctrine as
   arm 4); radii differ → `Disjoint` if the infinite-cylinder band
   argument says the surfaces cannot meet as a curve... CAREFUL:
   coaxial cylinders of different radii NEVER meet (annulus gap) →
   `Disjoint` is exact. Non-parallel axes (general) →
   `UnsupportedPairClass` this packet (the general skew-cylinder
   intersection is a quartic; DISPATCH-2 or Phase 2). Parallel
   non-collinear: exact axis-distance vs r1+r2 — tangent (equal) →
   `Line` (the shared generatrix through the closest points), greater →
   `Disjoint`. The landed `cylinder_arrangement.rs` is prior art for the
   cylinder-geometry discipline, not a dependency of the arm.

Unroutable inputs (any side cone/torus/spline/other, any class outside
arms 1–5) → `Unsupported(PairUnsupported::UnsupportedPairClass)`.

## Section 1 — `truck-certified/src/pair_dispatch.rs` (NEW)

Header: crate lint style. Module doc: the decisions above, each tagged,
plus the mass table (this packet's arms and their corpus counts, with the
prevalence doc as provenance) and the DISPATCH-2 deferral line.

Public surface (signatures exact; the arm bodies are the closed forms
above):

```rust
pub enum CertifiedPairParticipant { /* D-routing */ }

pub enum ContactLocus { /* D-result */ }

/// The result of dispatching one admitted-or-refused pair. Shape mirrors
/// the landed PairContactResult (formal/contact.rs).
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedPairResult {
    Disjoint,
    Contact(CertifiedPairContact),
    Unsupported(PairUnsupported),
    Unresolved(GenericUnresolved),
}

/// The certified contact: the sorted participants and the shared locus.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedPairContact {
    pub first: CertifiedPairParticipant,
    pub second: CertifiedPairParticipant,
    pub locus: ContactLocus,
}

/// Dispatch one analytic surface pair. Operand order is canonical (D-sorted).
pub fn dispatch_pair(a: &CertifiedPairParticipant, b: &CertifiedPairParticipant)
    -> CertifiedPairResult;
```

(`Unresolved(GenericUnresolved)` is carried for shape-parity with the
landed result; the exact-decision doctrine means the exact arms never
produce it — say so in the doc, keep the variant for the family shape.)

## Section 2 — `formal/intersection.rs`: the enum variant

Add `UnsupportedPairClass` to `PairUnsupported` with tag
`"pair_unsupported_class"`, and add the match arm to its `tag()` method.
Any OTHER match site on `PairUnsupported` in the crate that the compiler
flags gains a minimal arm (route to the same behavior as the other
unsupported causes where the site is a catch-all; the compiler drives the
census — `cargo check --workspace` finds them all). Nothing else in
intersection.rs changes.

## Section 3 — lib.rs: one line

`pub mod pair_dispatch;` beside `pub mod hull;`. Nothing else changes.

## Section 4 — tests (`truck-certified/tests/pair_dispatch_conformance.rs`, NEW)

All types are `pub`; fixtures are witnesses built from the landed
identifiers (`identify_sphere_world`, `identify_cylinder` on constructed
`RevolutedCurve<Line<Point3>>`, `identify_plane`'s schema path — build
planes through the same public entry the census uses; if no public
path constructs a `PlaneSchema` outside the crate, state that in RESULT
notes and use the public `identify_plane` route on a truck-geometry
`Plane`). Required tests are the eleven `tests_required` names; the
load-bearing shapes:

- Every emitted `Circle`'s radius enclosure CONTAINS a brute-force
  ulp-bracketed true radius (the sqrt-enclosure discipline).
- Every emitted `Line`/`Point` locus satisfies both surfaces' equations
  at its construction values to ulp tolerance (`// H-3` same-line opt-out
  where an epsilon appears).
- `distinct_parallel_planes_are_disjoint_and_coincident_refuses_overlap`
  and the coincident-sphere/coincident-cylinder asymmetry above hit the
  NAMED variants exactly.
- `operand_swap_yields_the_sorted_canonical_answer` — for a battery of
  pairs spanning all arms, `dispatch_pair(a, b) == dispatch_pair(b, a)`.
- `unsupported_pair_class_refuses_named_case` — sphere~torus-shaped
  (routed as unroutable: build the participant set the enum can express
  and assert the refusal for classes the enum cannot carry — e.g. via a
  raw `dispatch_pair` on two plane participants is routable, so this
  test exercises the enum-absence route: the from-identification
  constructors return `None` for cone/torus identifications, and the
  doc states the unroutable mapping; assert the constructor `None`s and
  one `UnsupportedPairClass` from a geometry case the screens reject,
  e.g. the general plane~cylinder oblique cut).
- `dispatch_never_panics_and_admission_is_exact_predicate_decided` —
  battery of degenerate inputs (zero-ish radii are impossible through
  the witnesses' `PositiveFinite`; NaN coordinates are impossible
  through the identifying constructors — assert the constructors' own
  refusals rather than panic paths) and a source-scan comment for
  no-unwrap.

House rules: H-3 opt-outs same-line; clippy zero findings on the new
files (pre-existing baseline findings out of scope); no manifest change.

## Done-when

- `cargo fmt` clean on the NEW files (workspace `--all` pre-existing
  violations out of scope — do not fix, do not claim).
- `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps` — zero findings attributable to this packet's files.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  landed suites unchanged PLUS the new dispatch tests.
- `cargo check --workspace --all-targets` green (the
  `PairUnsupported` widening compiles everywhere).

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. The substrate moved under you relative to the anchors — e.g. the
   landed sphere/cylinder/cone witnesses differ from the read, or
   `PairUnsupported`'s variants changed. Stop, do not adapt silently.
2. An arm's admission screen cannot be decided by exact expansion
   arithmetic (you find yourself reaching for an f64 epsilon or an
   interval straddle at ADMISSION time). The screen is exact or the arm
   refuses — say which configuration forced the gap and book it for
   DISPATCH-2 instead of widening a tolerance.
3. No public construction path reaches a witness type the routing enum
   needs (e.g. `PlaneSchema` is unreachable from the integration test
   crate) — record the exact reachability gap; the fix is an
   orchestrator decision about which constructor goes public, never a
   test-side reimplementation of an identifier.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): Phase-1
class-2 analytic pair dispatch, exact arms (BG-CK-P1-DISPATCH)`) BEFORE
writing `RESULT.json`.
