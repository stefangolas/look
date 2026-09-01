# BG-CK-P1-SPHERE — the certified sphere constructor (booked prevalence gap)

Certified-kernel Phase 1, second packet — the booked gap
(`docs/CERTIFIED_PHASE1_BOOKING.md`): the prevalence census found 2.56% of
corpus faces (1,831) are sphere-carried with representation-named evidence
only, because no certified constructor exists. This packet lands
`identify_sphere` in the refusing-constructor discipline of
`identify_plane`/`identify_cylinder`/`identify_torus_world`, unblocking
sphere PAIRS in BG-CK-P1-DISPATCH (corpus pair mass: cylinder~sphere 3,249;
sphere~spline 1,202; torus~sphere 539; plane~sphere 281; sphere~sphere 126).
Zero behavior change elsewhere: no existing module changes except
`formal/mod.rs`'s two declaration lines.

The 284 degenerate-torus faces (the census's honest-refusal residual) are
OUT OF SCOPE — they remain the named residual; `identify_torus_world`'s
spindle/horn refusal stands unchanged.

```yaml
id:          BG-CK-P1-SPHERE
contract:    [BG-CK-P1-SPHERE]
class:       mechanical
crates:      [truck-certified]
depends_on:  [BG-CK-P0-FREEZE]
write_allow:
  - vendor/truck/truck-certified/src/formal/sphere.rs
  - vendor/truck/truck-certified/src/formal/mod.rs
  - vendor/truck/truck-certified/tests/sphere_conformance.rs
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE1_BOOKING.md
  - docs/CERTIFIED_PREVALENCE.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
  - vendor/truck/truck-certified/src/formal/torus.rs
  - vendor/truck/truck-certified/src/formal/cylinder.rs
  - vendor/truck/truck-certified/src/formal/support.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/mod.rs
budget:      {turns: 25, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn identify_torus_world' vendor/truck/truck-certified/src/formal/torus.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum CylinderIdentification {' vendor/truck/truck-certified/src/formal/cylinder.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'pub mod sphere;' vendor/truck/truck-certified/src/formal/mod.rs"}
  - {id: A4, expect: 0, cmd: "grep -rnw 'SphereIdentification' vendor/truck/truck-certified/src | wc -l"}
  - {id: A5, expect: 2, cmd: "grep -c 'PositiveFinite::new' vendor/truck/truck-certified/src/formal/torus.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct Sphere' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub type SphericalSurface' vendor/truck/truck-stepio/src/in/step_geometry/mod.rs"}
  - {id: A8, expect: 4, cmd: "grep -c 'SpindleOrHornTorus' vendor/truck/truck-certified/src/formal/torus.rs"}
tests_required:
  - witness_carries_representation_derived_center_and_radius
  - witness_fields_are_private_with_accessors_only
  - non_finite_coordinate_refuses_named_case
  - non_positive_radius_refuses_named_case
  - non_similar_placement_refuses_named_case
  - longitude_period_verified_by_evaluation
  - placement_typed_and_world_entries_agree
  - identify_never_panics_and_refusals_are_named_cases
```

## Pre-made decisions (do not relitigate; quote the tags into the module doc)

**H-1.** The crate-level `#![deny(clippy::unwrap_used)]` covers the new
module. NO `unwrap`/`expect`/`panic!` anywhere in `sphere.rs`, including
tests (match/ok patterns only), and NO module-level `allow`. This is
authored certified code — the grandfathered-allow doctrine does not apply.

**Representation-derived, never re-derived (booking decision 3).** The
witness carries the center and radius EXACTLY as the representation states
them (the identify_plane retained-basis doctrine: never orthogonalised,
never normalised downstream). The constructor certifies ADMISSIBILITY
(finiteness, positivity, similarity); it does not "improve" the numbers.
No least-squares fitting, no averaging, no epsilon-tolerant snapping.

**World-params is the single introduction rule.** Mirror the torus exactly:
`identify_sphere_world(center: Point3, radius: f64) -> SphereIdentification`
is the one place a `CertifiedEmbeddedSphere` can be born; the typed and
placement entries delegate to it. The torus's shape
(`identify_torus`/`identify_torus_world`, `TorusIdentification`,
`TorusIdentificationFailure`) is the template — copy its discipline, not
its geometry.

**Placement entry, one named similarity rule.** STEP spheres arrive as
`Processor<Sphere, Matrix4>` (`SphericalSurface` in truck-stepio). The
placement entry extracts world parameters with this pre-decided rule:

- The placement matrix's three direction columns have magnitudes
  `s_x, s_y, s_z` computed in `f64`. If they are not ALL EQUAL as `f64`
  (exact comparison — no epsilon), the placement is not a similarity: it
  deforms the sphere into an ellipsoid, and the entry refuses
  `NonSimilarityPlacement`. (A similarity placement's columns are equal
  by construction; STEP does not carry anisotropic sphere placements.)
- The common column magnitude IS the radius scale: `radius_world =
  radius_local * s_x` (one `f64` product — this is the representation's
  own claim, read out, not a re-derivation).
- The center maps through the placement in `f64` (translation column +
  the scaled center). Same doctrine: read the representation's world
  center out, never re-fit it.
- A `Processor` with an identity-ish rotation still goes through this
  rule; there is no fast path that skips the column check.

**Refusal vocabulary is sphere-local and named.** `contract::Refusal` is
FROZEN; the base `truck_base::evidence::Refusal` is untouched (mapping
section C row 1). Define in `sphere.rs`:

```rust
/// Why a surface could not be certified as an embedded sphere.
pub enum SphereIdentificationFailure {
    /// A coordinate or the radius was not finite.
    NonFiniteCoordinate { cause: NumericDomainError },
    /// The radius was not strictly positive.
    DegenerateRadius,
    /// The placement's direction columns do not share one magnitude, so
    /// the surface is an ellipsoid, not a sphere.
    NonSimilarityPlacement,
    /// The longitude period could not be verified by evaluation.
    UnverifiedPeriod,
}

/// The outcome of sphere identification: a certified witness or a named
/// refusal. A refusal is the classifier saying "not this class" — the
/// dispatch order the Phase-1 fast path runs.
pub enum SphereIdentification {
    Sphere(CertifiedEmbeddedSphere),
    NotASphere(SphereIdentificationFailure),
}
```

`NumericDomainError` is `formal/numeric.rs`'s existing failure type (the
torus's `NonFiniteCoordinate { cause }` shape — reuse that exact name; do
not invent a parallel one). If `SphereIdentificationFailure` would
collide with an existing name in `formal/`, prefix-check the crate first
(anchor A4 says none exists today).

**Period verification by evaluation (the torus pattern, mechanical).**
Periods are placement-independent: build the canonical evaluation sphere
`Sphere::new(origin, radius)`, evaluate at a fixed interior `(u0, v0)`,
and verify `subs(u0 + TAU, v0)` agrees within
`MINIMUM_TORUS_PERIOD_RESIDUAL * radius` (reuse the torus's residual
constant by its existing name — do not add a second constant with the
same meaning; if the constant is private to `torus.rs`, promote it to
`pub(crate)` IN PLACE, which is a one-token visibility change, and say so
in RESULT notes). The sphere's latitude axis is NOT periodic; only the
longitude `2π` period is verified.

## Section 1 — `truck-certified/src/formal/sphere.rs` (NEW)

Header: match the crate's lint style (no new attributes — lib.rs governs).
Module doc: the pre-made decisions above, each tagged, plus the prevalence
provenance sentence (the 1,831-face gap; cite `docs/CERTIFIED_PREVALENCE.md`
section "Sphere").

### The witness

```rust
/// A certified embedded sphere: representation-derived center and radius,
/// admissibility certified by exact predicates at construction.
///
/// Constructed only through [`identify_sphere_world`] (the single
/// introduction rule). Fields are private; accessors return the
/// representation-derived values verbatim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedEmbeddedSphere { /* center: Point3, radius: PositiveFinite */ }

impl CertifiedEmbeddedSphere {
    /// The representation-derived center, verbatim.
    pub fn center(&self) -> Point3;
    /// The certified strictly-positive radius.
    pub fn radius(&self) -> PositiveFinite;
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str;  // "certified_embedded_sphere"
}
```

### The three entries

```rust
/// Read a constructed `Sphere` and certify an embedded sphere.
pub fn identify_sphere(sphere: &Sphere) -> SphereIdentification;

/// Read a STEP placement (`SphericalSurface` shape) and certify an
/// embedded sphere under the similarity rule above.
pub fn identify_sphere_placement(sphere: &Processor<Sphere, Matrix4>) -> SphereIdentification;

/// Read world-space parameters and certify an embedded sphere. The single
/// introduction rule; the other two entries delegate here.
pub fn identify_sphere_world(center: Point3, radius: f64) -> SphereIdentification;
```

`identify_sphere_world` order of refusals (each is an early return — the
torus's order): finiteness of all coordinates → `DegenerateRadius`
(`PositiveFinite::new`) → longitude period verification
(`UnverifiedPeriod`). No other checks; a sphere has no axis to degenerate
and no radius ratio to bound.

`Processor<Sphere, Matrix4>` and `Sphere` come from `truck-geometry`
prelude (the crate already depends on it — no manifest change; anchor the
existing import style against `formal/torus.rs`).

## Section 2 — `formal/mod.rs`: two lines

`pub mod sphere;` in the module list, and the `pub use` re-export line
(`SphereIdentification`, `SphereIdentificationFailure`,
`CertifiedEmbeddedSphere`) beside the torus re-exports. Nothing else in
the file changes.

## Section 3 — tests (`truck-certified/tests/sphere_conformance.rs`, NEW)

All entries are `pub`, so the integration test file constructs everything
directly (no in-module test split is needed — state that in RESULT notes
if you add one anyway). Load-bearing assertions:

1. `witness_carries_representation_derived_center_and_radius` — a witness
   from `identify_sphere_world(p, r)` has `center() == p` exactly and
   `radius().get() == r` exactly (representation-derived means bit-equal
   round-trip).
2. `witness_fields_are_private_with_accessors_only` — by construction:
   the integration test can only reach center/radius through accessors;
   no mutation path exists (assert via a compile-shaped comment and the
   accessor signatures).
3. `non_finite_coordinate_refuses_named_case` — NaN center coordinate and
   infinite radius each refuse `NonFiniteCoordinate`.
4. `non_positive_radius_refuses_named_case` — zero and negative radii
   refuse `DegenerateRadius`.
5. `non_similar_placement_refuses_named_case` — a `Processor` placement
   with unequal direction-column magnitudes (build one matrix with a
   2x/1x anisotropic scale) refuses `NonSimilarityPlacement`; a uniform
   2x-scaled placement ACCEPTS with `radius() == 2 * r_local` exactly and
   the placed center bit-equal (document the one-product rounding if a
   case is off by an ulp — H-3 opt-out `// H-3` ON THE SAME LINE).
6. `longitude_period_verified_by_evaluation` — a well-formed sphere
   certifies; the period check's residual constant path is exercised (a
   sphere is always periodic, so this is a green-path test; the refusal
   arm stays covered by test 8's shape).
7. `placement_typed_and_world_entries_agree` — `identify_sphere`,
   `identify_sphere_placement`, and `identify_sphere_world` on the same
   underlying geometry produce identical witnesses (`PartialEq`).
8. `identify_never_panics_and_refusals_are_named_cases` — every refusal
   arm above matches its named case exactly (no catch-all), and no entry
   panics on any input in the battery.

House rules: H-3 float-comparison opt-outs go ON THE SAME LINE as the
comparison. Clippy zero findings on the new files (`cargo clippy -p
truck-certified --all-targets --message-format=short --no-deps` — the
baseline carries pre-existing findings in untouched modules; findings on
`formal/` files NOT in this packet's write set are out of scope, say so
in RESULT notes rather than fixing them). No new dependency edges:
`truck-certified`'s manifest is untouched.

## Done-when

- `cargo fmt` clean on the NEW files (the workspace `--all` check has
  pre-existing violations outside this write set — do not fix them, do
  not claim them).
- `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps` — zero findings attributable to the new files.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  all landed suites unchanged PLUS the new sphere tests.
- `cargo check --workspace --all-targets` green.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. The substrate moved under you relative to the anchors — e.g. the torus
   residual constant's name changed, `PositiveFinite`'s API differs from
   the read, or `SphericalSurface`'s type shape moved. Stop, do not adapt
   silently.
2. The similarity rule is unsatisfiable on real STEP placements — e.g.
   you find the corpus placements' column magnitudes differ at the last
   ulp, making exact equality refuse valid spheres. Do NOT widen the
   rule with an epsilon yourself; the tolerance's value and its
   justification are an orchestrator decision. Record three concrete
   placements (their column magnitudes, verbatim) as evidence.
3. `identify_sphere_world` cannot verify the longitude period through the
   canonical-evaluation pattern because `Sphere::subs` semantics differ
   from the read (e.g. parameter ranges are not `u: 2π longitude, v:
   latitude`). Record the actual semantics from the source instead of
   guessing an interval.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): Phase-1
certified sphere constructor (BG-CK-P1-SPHERE)`) BEFORE writing
`RESULT.json`.
