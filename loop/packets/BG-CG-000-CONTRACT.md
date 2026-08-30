# WORK PACKET BG-CG-000-CONTRACT — the constructive geometry contract skeleton

You are landing the first packet of the **constructive geometry kernel** program.
Everything you need is in this document — the design is already made; your job
is to transcribe it into compiling Rust with stub bodies and to pin it with
tests. Do not read other spec files and do not redesign anything named here. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

Every other CG packet types against what you land here. Spelling matters.

```yaml
id:          BG-CG-000-CONTRACT
contract:    [BG-CG-000-CONTRACT]
class:       design
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-geometry/src/constructive/sampling.rs
  - vendor/truck/truck-geometry/tests/constructive_contract.rs
  - vendor/truck/truck-geometry/src/lib.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/lib.rs
  - vendor/truck/truck-geometry/src/errors.rs
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-meshalgo/src/tessellation/validity.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/outcome.rs
  - vendor/truck/truck-topology/src/lib.rs
tests_required:
  - frame3_try_new_accepts_right_handed_basis
  - frame3_try_new_rejects_left_handed_basis
  - frame3_try_new_rejects_non_orthonormal_basis
  - frame3_law_names_are_stable
  - profile2d_try_closed_rejects_structurally_invalid
  - profile_law_linear_correspondence_rejects_count_mismatch
  - scalar_law_linear_interpolates
  - direct_tolerance_defaults_derive_from_truck_base
  - construct_error_display_names_law_and_parameter
  - recipe_evaluators_refuse_while_stub
  - sampling_policy_resolve_refuses_while_stub
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct MeshedShellOutcome' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct FaceValidityCertificate' vendor/truck/truck-meshalgo/src/tessellation/validity.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum ProvenanceRecord' vendor/truck/truck-meshalgo/src/tessellation/formal/outcome.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub const TOLERANCE: f64' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub type EdgeID' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A6, expect: 0, cmd: "grep -c 'constructive' vendor/truck/truck-geometry/src/lib.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub struct PolygonMesh' vendor/truck/truck-polymesh/src/polygon_mesh.rs"}
```

## The one existing file you may touch

`vendor/truck/truck-geometry/src/lib.rs` is in `write_allow` for EXACTLY this
change and nothing else: a doc-comment line plus the module declaration,
inserted after the `arrange` module block (line order relative to the other
modules is free; nothing existing moves):

```rust
/// BG-CG-000-CONTRACT: the constructive geometry contract skeleton
/// (`SpineFrameRecipe`, frame/profile laws, sampling policy, errors).
/// Scaffolded with stub bodies; later CG packets fill them.
pub mod constructive;
```

Do not touch the `prelude` module, the lint attributes, or any other line of
that file. Every other file in your write set is NEW.

## Context — what this program is (one paragraph)

The constructive geometry kernel preserves authored BREP incidence through
realization instead of recovering it by sewing or booleans. A client authors a
`SpineFrameRecipe` (spine curve + frame law + profile law), a direct facet
backend realizes it as a shared-topology `PolygonMesh`, and certification
composes with the existing evidence types. CG-000 lands ONLY the contract
skeleton: the types, their signatures, doc-comments that freeze two conventions
the rest of the program consumes, and tests that pin the shapes. No geometry is
computed in this packet — every evaluator is a typed stub.

## File 1: `constructive/errors.rs` (new)

Follow the crate's existing error style (`src/errors.rs`): `thiserror::Error`,
doc comment per variant, `#[error("...")]` display strings. This error is the
constructive module's own currency; it does NOT replace the crate `Error`.

```rust
//! BG-CG-000-CONTRACT — typed refusals of constructive evaluation.

use thiserror::Error;

/// Typed refusal of a constructive evaluation or construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConstructError {
    /// The spine derivative vanished at `at`; the frame is undefined there.
    /// Refused, never clamped (normative: plan §3.2, "Spine smoothness
    /// contract").
    #[error("zero tangent at s = {at}")]
    ZeroTangent {
        /// The spine parameter where the tangent vanished.
        at: f64,
    },
    /// The named frame law is singular at `at` (e.g. `ArchitecturalUp` with
    /// `up ∥ t`). `law` names the law (see `FrameLaw::law_name`); the recipe
    /// refuses, it never rotates the frame silently.
    #[error("frame law `{law}` is singular at s = {at}")]
    FrameSingular {
        /// The spine parameter where the frame law is singular.
        at: f64,
        /// Which law refused; matches `FrameLaw::law_name`.
        law: &'static str,
    },
    /// The spine is not C¹ on the evaluated interval (tangent discontinuity
    /// beyond `DirectTolerance::parameter`, or a declaration-based detection).
    /// Non-C¹ spines are typed-refused, never clamped or silently smoothed.
    #[error("spine is not C1 at s = {at}")]
    SpineNotC1 {
        /// The spine parameter where C¹ fails.
        at: f64,
    },
    /// `ProfileLaw::LinearCorrespondence` was asked to pair profiles whose
    /// vertex counts differ. Correspondence is explicit, never inferred.
    #[error("profile correspondence mismatch")]
    ProfileCorrespondenceMismatch,
    /// The profile degenerated at `at` (e.g. a `Scale` law through zero).
    #[error("profile collapses at s = {at}")]
    ProfileCollapse {
        /// The spine parameter where the profile collapsed.
        at: f64,
    },
    /// A computed value was non-finite at `at`.
    #[error("non-finite value at s = {at}")]
    NonFinite {
        /// The spine parameter where a non-finite value appeared.
        at: f64,
    },
    /// Structurally invalid input to a constructor (wrong arity, non-finite
    /// fixture data, a non-orthonormal frame). Constructor validation only;
    /// evaluation-time failures use the parameter-bearing variants above.
    #[error("invalid input")]
    InvalidInput,
}
```

`errors.rs` is a new kernel `.rs` file, so GATE-1 requires it to declare the
H-1 lint denial. Start the file with this inner attribute (applies to the
module; it coexists with the crate-level denies):

```rust
#![deny(clippy::unwrap_used)]
```

Put the `use thiserror::Error;` after it. Do the same for all four new module
files and the test file.

## File 2: `constructive/sampling.rs` (new)

```rust
//! BG-CG-000-CONTRACT — the spine sampling policy.

use super::errors::ConstructError;

/// How the spine parameter axis is sampled for realization.
///
/// Determinism is normative (plan §7): identical ordered input + tolerance
/// produces byte-identical sample lists, repeated runs. Resolved sample lists
/// are sorted ascending and contain no duplicates; nothing about the output
/// may derive from hash-map iteration order.
#[derive(Debug, Clone, PartialEq)]
pub enum SamplingPolicy {
    /// `spine` uniformly spaced stations over the recipe's spine parameter
    /// domain (inclusive of both endpoints).
    UniformCount {
        /// The number of spine stations, >= 2.
        spine: usize,
    },
    /// The exact station list, caller-owned and used verbatim (sorted).
    CustomParameters(Vec<f64>),
    /// Refine until the chordal deviation of the spine polyline is within the
    /// given bound (a length; compared through `DirectTolerance::position`
    /// semantics, never a bare literal).
    ChordTolerance(f64),
    /// Refine until the tangent-direction change between adjacent stations is
    /// within the given bound (radians).
    AngularTolerance(f64),
}

impl SamplingPolicy {
    /// Resolves the policy over the spine parameter window `[s0, s1]` into a
    /// sorted, duplicate-free station list.
    ///
    /// STUB (BG-CG-000-CONTRACT): the resolution fills in with the recipe
    /// work (CG-001+). Until then every call refuses; it never panics (H-1)
    /// and never returns a fabricated sample list.
    pub fn resolve(&self, _s0: f64, _s1: f64) -> Result<Vec<f64>, ConstructError> {
        Err(ConstructError::InvalidInput)
    }
}
```

## File 3: `constructive/recipe.rs` (new)

```rust
//! BG-CG-000-CONTRACT — the core evaluator: X(s, v) = C(s) + T(s)·P(s, v).

use super::errors::ConstructError;
use super::{Frame3, FrameLaw, Profile2D, ProfileLaw};

/// The core recipe: a spine curve, a profile law transported along it, and the
/// frame law that orients the profile. `S` is the spine; CG-000 freezes the
/// struct and the evaluator signatures — the spine trait surface and the
/// evaluation bodies land with CG-001, so `S` carries no bound here yet.
#[derive(Debug, Clone, PartialEq)]
pub struct SpineFrameRecipe<S, P, F> {
    /// The spine curve C(s). Unbounded until CG-001 books the spine trait.
    pub spine: S,
    /// The profile law P(s, v).
    pub profile_law: P,
    /// The frame law producing T(s).
    pub frame_law: F,
}

impl<S, P, F> SpineFrameRecipe<S, P, F> {
    /// Assembles a recipe. No validation yet: construction is structural;
    /// refusal happens at evaluation, with a spine parameter attached.
    pub const fn new(spine: S, profile_law: P, frame_law: F) -> Self {
        Self { spine, profile_law, frame_law }
    }

    /// The realized point `X(s, v) = C(s) + T(s)·P(s, v)`.
    ///
    /// DEVIATION NOTE (frozen here, do not relitigate): the program plan
    /// spelled this `fn position(&self, s, v) -> Point3`. CG-000 freezes it
    /// fallible — a stub body must be total without lying (H-1 forbids
    /// panics; a fabricated zero point is a lie), and `profile` is fallible in
    /// the plan's own signature, so the composition cannot be less fallible
    /// than its parts. Semantics on the success path are unchanged.
    ///
    /// STUB (BG-CG-000-CONTRACT): CG-001 fills the evaluation.
    pub fn position(&self, _s: f64, _v: f64) -> Result<cgmath::Point3<f64>, ConstructError> {
        Err(ConstructError::InvalidInput)
    }

    /// The frame at `s` (see `Frame3` for the axis convention).
    ///
    /// STUB (BG-CG-000-CONTRACT): the frame laws land with CG-002 (analytic)
    /// and CG-003 (transport).
    pub fn frame(&self, _s: f64) -> Result<Frame3, ConstructError> {
        Err(ConstructError::InvalidInput)
    }

    /// The transported profile point `P(s, v)` in the frame plane.
    ///
    /// STUB (BG-CG-000-CONTRACT): CG-001 fills the evaluation.
    pub fn profile(&self, _s: f64, _v: f64) -> Result<cgmath::Point2<f64>, ConstructError> {
        Err(ConstructError::InvalidInput)
    }
}
```

Note the `Frame3, FrameLaw, Profile2D, ProfileLaw` imports: `FrameLaw` and
`ProfileLaw` are imported for the doc links to resolve (`[`FrameLaw`]` in a
doc comment) — if you do not reference them in docs, drop the unused imports
rather than carrying them (clippy `-D warnings` is a gate). `Profile2D` is
needed if you link it; same rule.

## File 4: `constructive/mod.rs` (new)

This file carries the two frozen conventions as module documentation. The two
blocks below are QUOTED CONTRACT — copy them into the module docs verbatim
(fixing only doc-link syntax so `cargo doc` stays clean), then define the
types.

### Module doc, part 1 — the index-identity convention (§3.4, frozen)

```text
Index identity (frozen at BG-CG-000-CONTRACT; two consumers: the direct facet
backend's grid registry and the meshalgo edge-sample ledger):

A mesh position index is a pure function of (entity identity, sample ordinal)
— never of coordinates.

- Each unique `EdgeID<Curve>` is sampled once; a reversed edge consumes the
  same integer sequence, reversed.
- Watertightness invariant: for incident faces A, B sharing edge E,
  I(A, E) == reverse(I(B, E)) **as integer sequences**.
- If the shell is combinatorially closed and every boundary mesh vertex's
  index derives from (EdgeID, ordinal), the emitted mesh is closed by
  construction; positional welding (`put_together_same_attrs`) is never
  invoked.
- The ledger carrier itself — `EdgeSampleLedger { edge_id: EdgeID<Curve>,
  parameters: Vec<f64>, position_indices: Vec<usize> }` — lands in
  truck-meshalgo (CG-005), not here.
- Implementation shape: a NEW parallel entry point
  (`triangulation_with_ledger`-style) reusing the existing unique-edge
  sampling and per-face CDT internals; the existing entry points remain
  bit-identical.
- FAC (CG-004): grid vertex (i, j) is created exactly once via a private grid
  registry keyed by (entity identity, sample ordinal); adjacent faces reuse
  the identity; internal grid edges are created once and traversed oppositely
  by their two faces.
```

### Module doc, part 2 — the certificate mapping (§3.5, frozen)

```text
Certificate mapping (frozen at BG-CG-000-CONTRACT; CG-007 implements it and
cannot be dispatched against an unfrozen mapping). New evidence composes with
the existing vocabulary — `MeshedShellOutcome`, `FaceValidityCertificate`,
`ProvenanceRecord` — never a parallel validation universe.

| Evidence kind | Carrier | Where the variant lands |
|---|---|---|
| Recipe construct refusals — every `ConstructError` variant (spine/frame validity, profile collapse, correspondence mismatch) | `Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)` at the realization entry; the detailed `ConstructError` rides the realization evidence record | NEW unit variant `EnvelopeCase::ConstructRefused` in `truck-base/src/evidence.rs`; NEW `RealizationEvidence` type in truck-meshalgo (CG-007) |
| Jacobian bounds (frame conditioning during realization) | per-face, positionally aligned with `shell.faces` exactly as `MeshedShellOutcome::face_failures` is | NEW `RealizationCertificate` struct + NEW field on the CG-004 realization outcome (CG-007 fills it); deliberately NOT a widening of `FaceValidityCertificate` — different vocabulary, the same separation doctrine as `band_attempts` vs `cone_band_attempts` |
| Shared-edge pair errors (`EdgeID` + FaceID A + FaceID B + error_a + error_b) | NEW field `shared_edge_pairs: Vec<SharedEdgePairEvidence>` on the realization outcome | NEW `SharedEdgePairEvidence` struct (CG-007); never a `ProvenanceRecord` variant (that type is `Copy + Eq`; the payload carries f64s) |
| Winding audit (twin-triangle) | a three-valued verdict carried beside the emitted `PolygonMesh` | NEW `RealizationVerdict { CertifiedWithinTolerance, Failed, Inconclusive }` (CG-007); winding-audit failure is `FAILED`, never a warning; uncertainty is `INCONCLUSIVE`, never converted into success |
| Any other realization-stage per-face evidence | the existing `MeshedShellOutcome` positional-vector doctrine | new vocabulary = a new `Vec<Option<...>>` field aligned with `shell.faces`; never a widening of an existing vector |

Standing notes: construct-stage failures predate meshing, so they never enter
`MeshedShellOutcome` (there is no shell to annotate). Every value computed in
floats certifies `Method::Float` (H-6), never `Method::Exact`. Verdicts are
three-valued throughout: `CERTIFIED_WITHIN_TOLERANCE | FAILED | INCONCLUSIVE`.
```

### The types (mod.rs)

Vector/point types come from the crate's re-exported base
(`use truck_base::cgmath64::*;` — match the crate's existing import style).

```rust
/// The orthonormal right-handed frame at one spine station.
///
/// Convention (normative): `tangent` is the spine direction, and the triple
/// (tangent, normal, binormal) satisfies `tangent × normal == binormal` and
/// unit lengths — i.e. `n = b × t`, `b = t × n`, matching the plan's
/// `FixedPlane` semantics (`t = C'/‖C'‖`, `b` = the plane normal, `n = b × t`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    /// The unit tangent — the spine direction.
    pub tangent: Vector3,
    /// The unit normal.
    pub normal: Vector3,
    /// The unit binormal.
    pub binormal: Vector3,
}

impl Frame3 {
    /// Validates and builds a frame: every component finite, every vector unit
    /// length, all three pairwise orthogonal, and the triple right-handed
    /// (`tangent × normal` equals `binormal`) — all compared at
    /// `DirectTolerance::default()`'s `position` bound. Constructor validation
    /// has no spine parameter, so every failure is `ConstructError::InvalidInput`.
    pub fn try_new(
        tangent: Vector3,
        normal: Vector3,
        binormal: Vector3,
    ) -> Result<Frame3, ConstructError> { /* real body: see below */ }
}

/// Which frame law a recipe carries, and its normative semantics.
///
/// - `FixedPlane`: `t = C'/‖C'‖`, `b = normal`, `n = b × t`; refuse
///   `‖C'‖ < tolerance`. Preferred for planar spines.
/// - `ArchitecturalUp`: `b = normalize(up × t)`, `n = t × b`; refuse `up ∥ t`
///   unless an explicit fallback policy is supplied. No silent frame rotation.
/// - `ParallelTransport`: Bishop rotation-minimizing frame via the
///   double-reflection method; stable at zero curvature and inflections;
///   deterministic from `initial_normal`. Frenet framing is never the default.
/// - `RadialAboutAxis`: analytic from the axis; rotated copies equivariant
///   modulo floating-point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameLaw {
    /// Pin the binormal to a fixed plane normal (planar spines).
    FixedPlane {
        /// The plane's unit normal; becomes the binormal.
        normal: Vector3,
    },
    /// The architectural up vector `up` (e.g. world +Z); refuses when
    /// `up ∥ tangent`.
    ArchitecturalUp {
        /// The preferred up direction.
        up: Vector3,
    },
    /// Rotation-minimizing (Bishop) frame, double-reflection method,
    /// deterministic from the initial normal.
    ParallelTransport {
        /// The normal at the spine's start station.
        initial_normal: Vector3,
    },
    /// Frames derived analytically from a fixed axis (revolved shapes).
    RadialAboutAxis {
        /// A point on the axis.
        origin: Point3,
        /// The axis direction.
        axis: Vector3,
    },
}

impl FrameLaw {
    /// The stable law name carried by `ConstructError::FrameSingular`'s `law`
    /// field: exactly `"FixedPlane"`, `"ArchitecturalUp"`,
    /// `"ParallelTransport"`, or `"RadialAboutAxis"`.
    pub fn law_name(&self) -> &'static str { /* real body, see tests */ }
}

/// A closed polygonal profile in the frame plane.
///
/// Semantics (normative for CG-001): vertices are ordered CCW about the
/// profile normal; edge `i` connects vertex `i` to vertex `(i + 1) mod k`;
/// the closing edge is implicit and never stored; no self-intersection.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile2D {
    /// The distinct vertices, in CCW order.
    pub vertices: Vec<Point2>,
}

impl Profile2D {
    /// Structural validation: at least three vertices, every coordinate
    /// finite. (Per-station collapse is an evaluation-time
    /// `ConstructError::ProfileCollapse`, CG-001's business.)
    pub fn try_closed(vertices: Vec<Point2>) -> Result<Profile2D, ConstructError> { /* real body */ }
}

/// A scalar function of the normalized spine parameter `s ∈ [0, 1]`.
///
/// Pre-decided here (the plan's `Scale` variant names this type without
/// defining it; CG-001 may add variants additively). `Linear` interpolates
/// `start + (end - start) * s` — total, no clamping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarLaw {
    /// A constant scalar.
    Constant(f64),
    /// Linear interpolation from `start` at s=0 to `end` at s=1 (linear
    /// extrapolation outside). A `Scale` profile law whose scalar reaches
    /// zero collapses the profile — refused as `ProfileCollapse` at
    /// evaluation time (CG-001), never silently.
    Linear {
        /// The value at s = 0.
        start: f64,
        /// The value at s = 1.
        end: f64,
    },
}

impl ScalarLaw {
    /// The scalar at `s`. Total arithmetic; non-finite inputs propagate
    /// (detection is the evaluator's job, CG-001).
    pub fn at(&self, s: f64) -> f64 { /* real body: Constant(c) => c; Linear{..} => start + (end - start) * s */ }
}

/// How the profile evolves along the spine.
///
/// `LinearCorrespondence` requires an EXPLICIT declared vertex/edge
/// correspondence between start and end; correspondence is never inferred.
/// Here the declaration is positional: vertex `i` of `start` corresponds to
/// vertex `i` of `end`. Arbitrary split/merge profile topology is out of
/// scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileLaw {
    /// The same profile at every station.
    Constant(Profile2D),
    /// One profile, uniformly scaled by a scalar law.
    Scale {
        /// The profile being scaled.
        profile: Profile2D,
        /// The scalar law over normalized s.
        scale: ScalarLaw,
    },
    /// Start and end profiles with declared positional correspondence;
    /// intermediate stations interpolate vertex-wise.
    LinearCorrespondence {
        /// The profile at s = 0.
        start: Profile2D,
        /// The profile at s = 1 (same vertex count as `start`).
        end: Profile2D,
    },
}

impl ProfileLaw {
    /// The validated `LinearCorrespondence` constructor: equal vertex counts
    /// or `ConstructError::ProfileCorrespondenceMismatch`; finite fixture
    /// data or `ConstructError::InvalidInput`.
    pub fn try_linear_correspondence(
        start: Profile2D,
        end: Profile2D,
    ) -> Result<ProfileLaw, ConstructError> { /* real body */ }
}

/// The tolerance bundle of the direct realization path.
///
/// Placement decision (booked): lives here in truck-geometry, not truck-base,
/// so CG-000 stays additive over the existing tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectTolerance {
    /// World-space distance comparisons (realization output).
    pub position: f64,
    /// Spine/profile parameter-space comparisons, including the C¹
    /// tangent-discontinuity detection threshold.
    pub parameter: f64,
    /// The bound on frame-Jacobian conditioning deviation.
    pub jacobian: f64,
    /// Shared-edge pair error comparison bounds.
    pub intersection: f64,
}

impl Default for DirectTolerance {
    /// Every field defaults to `truck_base::tolerance::TOLERANCE` (the plan:
    /// "defaults derive from truck_base::tolerance").
    fn default() -> Self {
        let t = truck_base::tolerance::TOLERANCE;
        Self { position: t, parameter: t, jacobian: t, intersection: t }
    }
}
```

`Frame3::try_new`, `FrameLaw::law_name`, `Profile2D::try_closed`,
`ProfileLaw::try_linear_correspondence`, and `ScalarLaw::at` get REAL bodies
in this packet (they are pure arithmetic/validation, not design): transcribe
the doc-commented semantics exactly. `position`, `frame`, `profile`,
`SamplingPolicy::resolve` are the four STUBs — typed refusals, per above.

mod.rs re-exports for consumers (`use truck_geometry::constructive::*`):

```rust
pub use errors::ConstructError;
pub use recipe::SpineFrameRecipe;
pub use sampling::SamplingPolicy;
```

Do NOT add the module to the crate `prelude` (that is a later packet's
decision if one is ever needed).

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing — in production code AND in these tests (the module
  denies `clippy::unwrap_used`; write tests with `matches!` /
  `assert!(matches!(..))` / `is_err()` / `assert_eq!` on fields).
- **H-2** Fallible operations return `Result<_, ConstructError>` (this
  module's frozen currency) — never `Option`, never a bare panic.
- **H-3** No bare absolute length literals in predicates. Anything like
  `1e-6` in an added line trips GATE-2; route every tolerance through the
  `truck_base::tolerance::TOLERANCE` const or `DirectTolerance`. The GATE-2
  regex also scans test files — never write a `1e-…` literal anywhere; if one
  is truly unavoidable it needs a same-line `// H-3` marker (same line, not
  the line above).
- **H-6** A value computed in floats is never recorded as `Method::Exact`
  (nothing here certifies yet, but any doc example must respect this).
- The crate carries `#![warn(missing_docs, missing_debug_implementations)]`
  and `deny(warnings)` in release: EVERY public item gets a doc comment and
  every public type derives `Debug`.
- No `unscaled_legacy(` calls anywhere (GATE-4 ratchet).
- No `debug_new`, no `cfg!(debug_assertions)` semantics (GATE-3).

## Tests required — `tests/constructive_contract.rs` (new file)

Header: `#![deny(clippy::unwrap_used)]` (GATE-1 — the file lives under
vendor/truck). These tests PIN the contract; they must pass exactly as the
types land, and later CG packets must not be able to change the shapes without
these tests failing.

1. `frame3_try_new_accepts_right_handed_basis` — `(t = (1,0,0), n = (0,1,0),
   b = (0,0,1))` is `Ok`; the returned fields equal the inputs.
2. `frame3_try_new_rejects_left_handed_basis` — same but `b = (0,0,-1)`:
   `Err(ConstructError::InvalidInput)` (`t × n = +z ≠ b`).
3. `frame3_try_new_rejects_non_orthonormal_basis` — a non-orthogonal pair AND
   a non-unit vector each give `Err(ConstructError::InvalidInput)`.
4. `frame3_law_names_are_stable` — the four `law_name()` outputs are exactly
   the four strings named in `FrameLaw::law_name`'s doc.
5. `profile2d_try_closed_rejects_structurally_invalid` — fewer than three
   vertices, and a profile containing a non-finite vertex, each give
   `Err(ConstructError::InvalidInput)`.
6. `profile_law_linear_correspondence_rejects_count_mismatch` — a triangle
   and a quad give `Err(ConstructError::ProfileCorrespondenceMismatch)`.
7. `scalar_law_linear_interpolates` — `Linear { start: 1.0, end: 3.0 }` at
   `0.5` is `2.0`, at `0.0` is `1.0`, at `1.0` is `3.0` (plain decimal
   literals are fine; the H-3 regex only bans `1e-…` forms).
8. `direct_tolerance_defaults_derive_from_truck_base` — all four fields
   `assert_eq!` against the imported `truck_base::tolerance::TOLERANCE` const
   (no literals).
9. `construct_error_display_names_law_and_parameter` — the `Display` of
   `FrameSingular { at: 0.5, law: "ArchitecturalUp" }` contains both
   `"ArchitecturalUp"` and `"0.5"`.
10. `recipe_evaluators_refuse_while_stub` — a
    `SpineFrameRecipe { spine: (), profile_law: Constant(profile), frame_law:
    FixedPlane { normal } }`: `position`, `frame`, `profile` each return
    `Err(ConstructError::InvalidInput)`, and nothing panics. (This test
    freezes that the stubs are total; CG-001 will amend it in place to
    positive-value assertions — that amendment is expected and booked.)
11. `sampling_policy_resolve_refuses_while_stub` — one representative of each
    `SamplingPolicy` variant returns `Err(ConstructError::InvalidInput)` from
    `resolve(0.0, 1.0)`.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets -- -D warnings
cargo test -p truck-geometry --lib --tests
```

Never run a bare `cargo test` (it builds 56 examples). No workspace check is
needed — the module is purely additive and nothing existing consumes it; the
verifier runs the workspace gates authoritatively. Send cargo output to a file
and read the tail.

## Forbidden

Editing any file outside `write_allow` (in particular: `truck-base/src/*`,
`truck-meshalgo/src/*`, the crate `prelude`, `Cargo.toml`, `Cargo.lock`,
`scripts/kernel-gates.sh`). Implementing any evaluator beyond the five real
bodies named above. Adding `#[ignore]`. Adding `#[allow]` without a
justification comment on the same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A6 must read 0 at your fork
  point — it proves the module does not exist yet)
- the contract as written cannot compile as specified → `SPEC_GAP`, naming
  the exact conflict (do not silently adjust a signature)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-000-CONTRACT","status":"DONE","contracts":["BG-CG-000-CONTRACT"],
 "tests_added":11,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":0,"A7":1},
 "notes":"any deviation from the quoted contract, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(geometry): constructive geometry contract skeleton (BG-CG-000-CONTRACT)`
BEFORE writing `RESULT.json`.
