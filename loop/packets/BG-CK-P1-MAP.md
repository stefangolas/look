# BG-CK-P1-MAP — class 1 CertifiedMap: admission, enclosure oracle, rank margin

Certified-kernel Phase 1, third packet — plan §2 class 1
(`docs/CERTIFIED_PHASE1_BOOKING.md`). Admission of a compact rectangular
parameter domain of a B-spline curve/surface decomposed to Bézier pieces
(D2), the enclosure oracle over subboxes (hull of f and ∂f, via the landed
BG-CK-P1-HULL primitive), and the rank margin: interval evaluation of
Jacobian minors against a declared τ. **Admission lives here, not in
truck-geometry** (D1) — geometry types gain no knowledge of certification;
callers admit. Correspondence-is-input: loft/screw/developable/section-law
maps are CLIENTS. First consumer: the SpineFrameRecipe sweep core
(certified Jacobian evidence for TR-VAL-001).

The F2 frozen table anticipated exactly the compositions this module makes:
NormalAdmissibility is "hull-bounded first-derivative patches, interval
cross product, directed rounding at the leaves". These are the sanctioned
fixed, named compositions — not free-form interval arithmetic.

```yaml
id:          BG-CK-P1-MAP
contract:    [BG-CK-P1-MAP]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P0-FREEZE, BG-CK-P1-HULL]
write_allow:
  - vendor/truck/truck-certified/src/certified_map.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/certified_map_conformance.rs
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE1_BOOKING.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/formal/numeric.rs
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/bspsurface.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn hull_bernstein_1d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn hull_bernstein_2d' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub enum HullRefusal' vendor/truck/truck-certified/src/hull.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn bezier_decomposition' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'pub mod certified_map;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub struct PositiveFinite' vendor/truck/truck-certified/src/formal/numeric.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'bezier_decomposition' vendor/truck/truck-geometry/src/nurbs/bspsurface.rs"}
tests_required:
  - curve_map_admission_certifies_rank_above_tau
  - surface_map_admission_certifies_rank_above_tau
  - degenerate_parameterization_refuses_named_case
  - non_compact_region_refuses_named_case
  - enclosure_contains_brute_force_samples
  - region_enclosure_contained_in_whole_domain_enclosure
  - bspline_curve_admission_matches_direct_bezier_admission
  - bspline_surface_decomposition_covers_the_declared_domain
  - rank_margin_lower_bound_bounded_by_brute_force_min
  - map_never_panics_and_tau_is_declared_not_inferred
```

## Pre-made decisions (do not relitigate; quote the tags into the module doc)

**H-1.** Crate-level `#![deny(clippy::unwrap_used)]` covers the new module.
NO `unwrap`/`expect`/`panic!`, NO module-level `allow` — authored certified
code.

**D1 — admission lives here.** No truck-geometry change; its manifest and
sources are read-only. The surface Bézier decomposition is built INSIDE
this module from landed per-axis curve machinery (see D-map below); it is
NOT contributed back to truck-geometry.

**D2 — one primitive, named compositions.** Every enclosure goes through
the landed `hull.rs` kernels (`hull_bernstein_1d`, `hull_bernstein_2d`,
`bernstein_derivative_1d`, `bernstein_derivative_2d`). The rank margin is
exactly two named compositions, pre-decided:

- Curve rank margin: hulls of the three first-derivative coefficient
  vectors (per coordinate) give interval components `C'`; the certified
  lower bound of `|C'|²` is the sum over coordinates of `d_i²` where
  `d_i = 0` if the component's enclosure contains 0, else the distance
  from 0 to the nearer endpoint (each square and sum through
  `CertifiedInterval::mul`/`add` — outward-rounded).
- Surface rank margin: hulls of the six first-derivative patches
  (`Sᵤ`, `Sᵥ` per coordinate) give interval vectors; the interval cross
  product (three fixed coordinate expressions through
  `CertifiedInterval::mul`/`sub`) gives the interval normal `Sᵤ × Sᵥ`;
  its norm lower bound by the same component rule as above.

The margin DECISION compares the certified lower bound against the
declared τ in `f64` (the F3 pattern: a certified bound against a declared
threshold — never a naked f64 comparison of raw geometry).

**D-tau — declared, never inferred.** τ arrives as `PositiveFinite`
(`formal/numeric.rs`) on every admission call. No default, no module
constant, no auto-tuning. A region whose certified margin lower bound is
≤ τ refuses `ParameterizationDegenerate` — this covers both the truly
degenerate case and the cannot-decide case (the enclosure straddles τ).
The refusal is PER REGION: the caller's remedy is a smaller region (a new
admission attempt over a subbox), never a weakened τ and never a retry
with the same box. Say this in the module doc — it is the honest
discipline and matches F3's "never retried with a weaker test".

**D-map — the piece table is the module's spine.** The declared domain is
a B-spline's clamped knot range. Curve: the landed
`BSplineCurve::bezier_decomposition()` gives the pieces; each piece's
coefficient vectors and its subinterval `[t_i, t_{i+1}]` are recorded in
the map's piece table. Surface: truck-geometry has NO surface
decomposition (anchor A7), so this module builds one mechanically from
landed curve machinery: decompose every row of control points along `u`
with `BSplineCurve::bezier_decomposition` (each row is a BSplineCurve in
the u parameter), then for each u-piece decompose every column along `v`
the same way. Tensor cut operations commute across axes, so the result is
exactly the Bézier patch grid; the worker asserts coverage — the patches'
subintervals tile the declared domain exactly (adjacent shared endpoints,
no gaps) — as a required test. Rational (weighted) B-splines are OUT OF
SCOPE for this packet: admission takes ordinary `BSplineCurve<Point3>` /
`BSplineSurface<Point3>`; the homogeneous path composes later per the F2
rational rows when a consumer needs it.

**D-region — queries are per-piece, combined conservatively.** A subbox
of the declared domain may span piece boundaries. The enclosure is the
component-wise union (min lower, max upper) of the per-piece hulls over
the pieces the subbox touches; the rank margin is the MINIMUM of the
per-piece margins over those pieces. Sound because the clamped pieces
cover the domain exactly. EnclosureUnavailable propagates from any piece
whose directed-rounded hull overflows (`HullRefusal::EnclosureUnavailable`
maps 1:1 onto `MapRefusal::EnclosureUnavailable`); DomainNotCompact maps
the same way and additionally fires for a region outside the declared
domain.

**Refusal vocabulary is map-local.** `contract::Refusal` stays frozen; the
base Refusal is untouched (mapping section C row 1). `MapRefusal` in this
module, exactly three named cases (plan §2 class 1), `tag()` method,
no catch-all:

```rust
pub enum MapRefusal {
    /// The certified rank margin is not above the declared tau on this
    /// region (covers both true degeneracy and cannot-decide). Remedy: a
    /// smaller region, never a weaker tau.
    ParameterizationDegenerate,
    /// A directed-rounded hull overflowed on this region.
    EnclosureUnavailable,
    /// The region is not a compact subset of the declared domain
    /// (non-finite, misordered, or outside bounds; inclusive edges).
    DomainNotCompact,
}
```

## Section 1 — `truck-certified/src/certified_map.rs` (NEW)

Header: match the crate's lint style. Module doc: the decisions above,
each tagged, plus the class-1 provenance (plan §2, booking doc).

### The two map types

```rust
/// A certified curve map C: [t0, t1] -> R^3, admitted over its declared
/// domain. Constructed only through [`admit_curve`].
#[derive(Debug, Clone)]
pub struct CertifiedCurveMap { /* piece table: subintervals + coefficient vectors, tau */ }

/// A certified surface map S: [u0, u1] x [v0, v1] -> R^3, admitted over
/// its declared domain. Constructed only through [`admit_surface`].
#[derive(Debug, Clone)]
pub struct CertifiedSurfaceMap { /* piece table: subboxes + coefficient grids, tau */ }
```

### Admission (the constructors)

```rust
/// Admit a curve map over the B-spline's clamped knot range. Decomposes to
/// Bézier pieces (landed `bezier_decomposition`), then certifies the rank
/// margin over the WHOLE domain against `tau`. Refuses
/// ParameterizationDegenerate if the whole-domain margin is not above tau —
/// admit a sub-region for locally-degenerate maps.
pub fn admit_curve(curve: &BSplineCurve<Point3>, tau: PositiveFinite)
    -> Result<CertifiedCurveMap, MapRefusal>;

/// Admit a curve map over a compact subinterval of an already-decomposed
/// domain (the per-region remedy). The map carries its piece table; the
/// region may span pieces (D-region).
pub fn admit_curve_region(map: &CertifiedCurveMap, sub: (f64, f64))
    -> Result<CertifiedRegionRank, MapRefusal>;

/// Admit a surface map over the B-spline's clamped knot ranges. Builds the
/// Bézier patch grid in-module (D-map), then certifies the rank margin over
/// the WHOLE domain.
pub fn admit_surface(surface: &BSplineSurface<Point3>, tau: PositiveFinite)
    -> Result<CertifiedSurfaceMap, MapRefusal>;

/// The surface per-region remedy, mirroring `admit_curve_region`.
pub fn admit_surface_region(map: &CertifiedSurfaceMap, sub: ((f64, f64), (f64, f64)))
    -> Result<CertifiedRegionRank, MapRefusal>;
```

`CertifiedRegionRank` is the admission answer for a region: the certified
margin lower bound (a `CertifiedInterval`) and the region box, with
accessors only.

### The oracle (methods on the maps)

```rust
impl CertifiedCurveMap {
    /// Certified enclosure of C(t) over a compact subinterval: per-piece
    /// hulls of the value patches, combined conservatively (D-region).
    pub fn enclosure(&self, sub: (f64, f64)) -> Result<[CertifiedInterval; 3], MapRefusal>;
    /// Certified LOWER bound of |C'(t)| over the subinterval (D2 named
    /// composition). Above-tau certification is the caller's comparison.
    pub fn rank_margin(&self, sub: (f64, f64)) -> Result<CertifiedInterval, MapRefusal>;
}

impl CertifiedSurfaceMap {
    /// Certified enclosure of S(u, v) over a compact rectangle.
    pub fn enclosure(&self, sub: ((f64, f64), (f64, f64)))
        -> Result<[CertifiedInterval; 3], MapRefusal>;
    /// Certified LOWER bound of |Sᵤ × Sᵥ| over the rectangle.
    pub fn rank_margin(&self, sub: ((f64, f64), (f64, f64)))
        -> Result<CertifiedInterval, MapRefusal>;
}
```

Value-patch hulls: for a piece's per-coordinate coefficient vector
(curve) or grid (surface), `hull_bernstein_1d` / `hull_bernstein_2d` over
the piece-mapped subbox; first-derivative patches via
`bernstein_derivative_1d` / `bernstein_derivative_2d` applied once (the
margin never needs order 2 — curvature is class 3, not this packet).

## Section 2 — lib.rs: one line

`pub mod certified_map;` beside `pub mod hull;`. Nothing else changes.

## Section 3 — tests (`truck-certified/tests/certified_map_conformance.rs`, NEW)

All entry points are `pub` and truck-geometry types are public, so the
integration file needs no in-module split. Fixtures: a degree-3 BSpline
circle-like closed curve is NOT wanted (periodic knots complicate
clamping); use ordinary clamped B-splines — e.g. the doc-test curve of
`bezier_decomposition` (2 pieces over [0, 2]) and a bicubic-ish bilinear/
quadratic surface built from `KnotVec::bezier_knot` (single Bézier piece)
plus a 2-piece u-knot surface for the piece-spanning case. Load-bearing
assertions:

1. `curve_map_admission_certifies_rank_above_tau` — a straight-ish line
   curve with constant nonzero derivative admits; `rank_margin` over the
   whole domain is above tau and its lower bound is within a few ulps of
   the analytic `|C'|` (H-3 opt-out same-line).
2. `surface_map_admission_certifies_rank_above_tau` — a plane patch
   admits; the rank margin lower bound is within a few ulps of the
   analytic `|Sᵤ × Sᵥ|` (constant for a plane).
3. `degenerate_parameterization_refuses_named_case` — a curve whose
   derivative vanishes on a subinterval (e.g. a cubic Bézier with two
   coincident control points mid-curve) refuses admission over a region
   containing the vanish with `ParameterizationDegenerate`, while
   sub-regions excluding it ADMIT (the per-region remedy, exercised).
4. `non_compact_region_refuses_named_case` — misordered, non-finite, and
   outside-domain regions refuse `DomainNotCompact`; the closed domain
   boundary admits (inclusive edges).
5. `enclosure_contains_brute_force_samples` — 1000 samples of the map
   over the region lie inside the enclosure (curve and surface).
6. `region_enclosure_contained_in_whole_domain_enclosure` — monotone
   under region inclusion.
7. `bspline_curve_admission_matches_direct_bezier_admission` — a
   B-spline with Bézier knots admits to a piece table identical (up to
   the piece bookkeeping) to admitting each `bezier_decomposition` piece
   directly; their enclosures agree on overlapping regions.
8. `bspline_surface_decomposition_covers_the_declared_domain` — the
   patch grid's sub-boxes tile the declared domain exactly (adjacent
   shared edges, no gap, no overlap beyond measure zero — assert sorted
   adjacency equality in f64, exact).
9. `rank_margin_lower_bound_bounded_by_brute_force_min` — the certified
   margin lower bound is ≤ the minimum finite-difference `|C'|` (resp.
   `|Sᵤ × Sᵥ|`) over a sample grid (containment discipline: the
   certified bound never overclaims).
10. `map_never_panics_and_tau_is_declared_not_inferred` — every entry
    returns `Result`; admission signatures take `PositiveFinite` (the
    type IS the test that τ is declared); no unwrap/expect in the module
    text (source-scan assertion like hull's).

House rules: H-3 opt-outs same-line. Clippy zero findings on the new
files; pre-existing findings in untouched modules are out of scope. No
new dependency edges; `truck-certified`'s manifest untouched.

## Done-when

- `cargo fmt` clean on the NEW files (workspace `--all` has pre-existing
  violations outside this write set — do not fix, do not claim).
- `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps` — zero findings attributable to the new files.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  landed suites unchanged PLUS the new map tests.
- `cargo check --workspace --all-targets` green.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE
WORKTREE ROOT) with the finding verbatim if:

1. The substrate moved under you relative to the anchors — e.g. hull.rs's
   kernels changed shape, `bezier_decomposition`'s clamping semantics
   differ from the read, or `PositiveFinite`'s API differs. Stop, do not
   adapt silently.
2. The row-wise/column-wise surface decomposition does NOT commute — a
   patch grid whose evaluations disagree with the surface's own `subs`
   beyond ulp noise. Record the disagreement (piece, parameter, both
   values) instead of papering over it; the tensor-commutation claim is a
   pre-made decision and its failure is a SPEC_GAP.
3. The rank margin's component rule cannot be expressed without free-form
   interval arithmetic (you find yourself composing more than the two
   named compositions). The compositions are frozen; say what you needed
   instead.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): Phase-1
class-1 CertifiedMap admission + oracle (BG-CK-P1-MAP)`) BEFORE writing
`RESULT.json`.
