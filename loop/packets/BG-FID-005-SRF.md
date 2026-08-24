# WORK PACKET BG-FID-005-SRF — the `rep` operator, SURFACE case: REP-SRF-001, the surface (iv-b) discharge, the double-sheet negative test

You are implementing one item from a formal kernel specification. Everything
you need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-FID-005-SRF","status":"DONE","contracts":["BG-FID-005-SRF"],
 "tests_added":12,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it — especially the landed
`rep_curve`/`HermiteCurve` pattern you are generalizing, or the landed
`lfs::curvature_radius_lower` you are consuming — say so in `disagreements`
rather than making the code match the packet.**

```yaml
id:          BG-FID-005-SRF
contract:    [BG-FID-005-SRF]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/rep.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs   # scoped: ONLY the module doc line (Decision 8)
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-evidence/src/fid/isotopy.rs
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 50, ctx_tokens: 160000}
anchors:
  # Expected counts assume the tree at the integration tip (verified 2026-08-23
  # through scratch/fid005srf-anchors.sh). The dispatch re-measures every count
  # before launch (H-8).
  - {id: S1, expect: 1, cmd: "grep -c 'pub fn rep_curve' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S2, expect: 1, cmd: "grep -c 'pub struct HermiteCurve' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S3, expect: 1, cmd: "grep -c 'pub enum RepError' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S4, expect: 1, cmd: "grep -c 'pub fn curvature_radius_lower' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: S5, expect: 4, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: S6, expect: 1, cmd: "grep -c 'pub mod rep' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: S7, expect: 1, cmd: "grep -c 'pub fn krawczyk' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: S8, expect: 1, cmd: "grep -c 'pub enum KrawczykProof' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: S9, expect: 1, cmd: "grep -c 'fn uniform_cells' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: S10, expect: 1, cmd: "grep -c 'pub(crate) fn box_distance' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: S11, expect: 1, cmd: "grep -c 'pub(crate) fn sup_distance_box' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: S12, expect: 1, cmd: "grep -c 'pub(crate) fn angle_pass_form' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: S13, expect: 1, cmd: "grep -c 'pub(crate) fn dot_box' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: S14, expect: 1, cmd: "grep -c 'pub fn face_scale_components' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: S15, expect: 1, cmd: "grep -c 'pub fn fibre_degree_one_auto' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: S16, expect: 0, cmd: "grep -c 'pub fn rep_surface' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S17, expect: 0, cmd: "grep -c 'pub struct HermiteSurface' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S18, expect: 0, cmd: "grep -c 'SurfaceBoundary' vendor/truck/truck-evidence/src/fid/rep.rs"}
  - {id: S19, expect: 0, cmd: "grep -c 'pub enum RepSurfaceError' vendor/truck/truck-evidence/src/fid/rep.rs"}
```

## Problem

`rep` is the ONLY sanctioned path from an exact result into the emitted
geometry class. `rep_curve` (BG-FID-005, landed in this same file) implements
the CURVE case; this packet lands the SURFACE case: `rep_surface` approximates
one exact SURFACE patch over a certified tensor-product partition and returns
the achieved error, the achieved normal-angle margin AND the per-cell (iv-b)
discharge TOGETHER — never a bare surface, and never (eps, theta) alone, since
(eps, theta) without the (iv) discharge is precisely the unsound pairing
(conditions (i)-(iii) pass on a double sheet; nothing above the certificate is
sound if (iv) is missing). The surface double-sheet negative test — "a double
sheet inside one normal tube, with correct tangent planes on BOTH sheets" — is
this packet's flagship negative test: it is where (iv) is least intuitive.

The design point (same as the curve packet): `rep` already subdivides to hit
(eps, theta), so its cell decomposition IS the partition the (iv-b) form
requires — per-cell fibre-block containment, per-cell injectivity and
non-adjacent separation cost no new subdivision structure, only new assertions
on boxes the loop already computes.

Every number this packet quotes is machine-checked through the exact formulas
below (outward-rounded interval arithmetic reproducing the gates' box
semantics — checked orchestrator-side before dispatch). If your module
disagrees with a quoted number by more than float slack, your code or fixture
is wrong — or say so in `disagreements` with the arithmetic.

## Decisions already made for you

### Decision 0 — API and types

```rust
/// The boundary kind of ONE surface patch, per direction, vouched for by the
/// CALLER (the BG-FID-003-r2 CurveBoundary decision lifted to 2D). Drives
/// wrap adjacency in the (iv-b)(c) separation and wrapped gaps in
/// self-separation ONLY; rep_surface runs NO boundary-correspondence gate
/// (the curve rep ran none; that condition belongs to the isotopy checker,
/// which has no surface form yet).
///
/// @establishes the caller's boundary-kind input for ONE surface patch
/// @does-not-establish closedness | openness | any topology claim
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceBoundary {
    /// Both parameter directions are genuine boundary.
    Open,
    /// The u endpoints are identified (periodic in u).
    ClosedU,
    /// The v endpoints are identified (periodic in v).
    ClosedV,
    /// Both directions identified (a torus-like patch).
    ClosedUV,
}

/// Typed refusal. Mirrors RepError's arms and refusal mapping exactly
/// (the fid/ house pattern: one typed enum per operator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepSurfaceError {
    /// tau_rep <= 0 / non-finite, gap <= 0 / non-finite, or a
    /// non-finitely-bounded exact span on either axis.
    InvalidMargin,
    /// The scale components could not be certified at all (collapsing
    /// geometry, or the scale-stage budget exhausted — BOTH are the
    /// certification-failure route). Routes to §5 collapse via
    /// `into_refusal()`. NEVER fired merely because tube_scale is small:
    /// small-but-positive refines (Decision 4).
    ReachTooSmall,
    /// Refinement did not reach target within budget, or eps stalled above
    /// target at the enclosure width floor. Carries the spend; never a
    /// best-effort surface.
    Unresolved { subdivisions: u32 },
}

impl RepSurfaceError {
    /// The §4-level view: ReachTooSmall -> UnsupportedEnvelope(ReachTooSmall),
    /// Unresolved -> NumericallyUnresolved carrying spend, InvalidMargin has
    /// no §4 arm (debug_assert! never; return the nearest arm documenting why)
    /// — copy RepError::into_refusal verbatim, substituting the type.
    pub fn into_refusal(self) -> Refusal;
}

/// Certified scale components for ONE surface patch, named under the
/// BG-FID-001 amendment's rules (the CurveScaleComponents mirror): no field
/// claims tube/reach/lfs semantics; promotion is L-FEDERER-PATCH (open).
/// `+inf` values are intentional (flat; empty separation slice).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceScaleComponents {
    /// From [`surface_curvature_radius_lower_span`].
    pub curvature_radius_lower: f64,
    /// From [`surface_self_separation_lower_span`]; `+inf` when no pair
    /// qualifies (the empty-set identity).
    pub self_separation_lower: f64,
}

impl SurfaceScaleComponents {
    /// min(curvature_radius_lower, self_separation_lower / 2) — the
    /// Federer-motivation composition, a gate bound ONLY, never reach.
    pub fn tube_scale_lower(&self) -> f64;
}

/// The emitted approximant: tensor-product bicubic Hermite in Bezier form
/// over a certified uniform-per-axis grid (Decision 2). Implements
/// ParametricSurface + EnclosureSurface so every downstream consumer
/// consumes it through the same traits as any other surface.
pub struct HermiteSurface { /* u-knots, v-knots, per-cell nets, spans */ }

/// What rep_surface proved, and what it achieved. This IS the certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct RepSurfaceCertificate {
    /// Certified achieved two-sided sup-distance exact-vs-emitted.
    pub eps_achieved: f64,
    /// Certified min |cos| over all paired normal boxes (the (ii) margin).
    pub angle_cos_lower: f64,
    /// Final per-axis partition depths (2^depth cells per axis).
    pub depth_u: u32,
    pub depth_v: u32,
    /// The knots, ascending, echo of the certified partition.
    pub partition_u: Vec<f64>,
    pub partition_v: Vec<f64>,
    /// Refinement levels spent from the first attempt to the certificate.
    pub subdivisions_spent: u32,
    /// The scale components every gate was evaluated against (echo).
    pub scale: SurfaceScaleComponents,
}

/// rep_surface's success: the surface AND the certificate, together.
#[derive(Debug, Clone)]
pub struct RepSurfaceOutput {
    pub surface: HermiteSurface,
    pub certificate: RepSurfaceCertificate,
}

/// The outcome of the per-cell surface (iv-b) discharge on one partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceIvbOutcome {
    /// Every interior grid vertex certified and no non-adjacent overlap.
    Pass,
    /// A grid-vertex projection could not be certified: refine.
    ProjectionFailure,
    /// Certified non-adjacent overlap (row-major cell indices): two sheets in
    /// one normal-tube region. Either a partition too coarse (refinement
    /// fixes it) or a genuine self-overlap (route to the self-intersection
    /// engine) — the caller decides. Positive certified claim, not epistemic.
    MultiSheet { cells: (usize, usize) },
}

/// Discharge (iv-b) per cell on a SHARED parameter grid: (b) at every
/// INTERIOR grid vertex and (c) over whole-cell boxes. `cell_eps[j]` is the
/// per-cell certified deviation (row-major), from the same measurement the
/// loop uses (Decision 4). The failing pair is reported row-major.
pub fn surface_ivb_discharge(
    exact: &impl EnclosureSurface,
    approx: &impl EnclosureSurface,
    grid: (&[f64], &[f64]),
    boundary: SurfaceBoundary,
    cell_eps: &[f64],
    budget: &mut Budget,
) -> SurfaceIvbOutcome;

/// Approximate one exact surface patch to tau_rep, certifying the eps/theta
/// gates and discharging (iv-b) on the same partition.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///     (i)-(ii) between exact and emitted surface at the achieved (eps, theta)
///     + (iv-b) per-cell fibre-block degree-one on the emitted partition
/// @does-not-establish
///     isotopy | homeomorphism | side separation | whole-span one-sheet as a
///     topological claim | surface isotopy conditions (iii) | reach semantics
pub fn rep_surface(
    exact: &impl EnclosureSurface,
    boundary: SurfaceBoundary,
    tau_rep: f64,
    gap: f64,
    initial_depth: u32,
    budget: &mut Budget,
) -> Result<RepSurfaceOutput, RepSurfaceError>;

/// Typed scale-stage refusal (the isotopy helpers' house pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceScaleError {
    /// Bad span or gap input.
    InvalidMargin,
    /// The curvature span helper could not certify (immersion collapse at the
    /// floor, or scale-stage budget exhaustion).
    CurvatureUnresolved,
    /// The separation span helper could not complete.
    SeparationUnresolved,
}

/// Certified lower bound on the exact surface's minimum curvature radius over
/// its whole span: uniform quad refinement, per cell `lfs::curvature_radius_lower`
/// (landed, pub — DO NOT re-implement it), min over certifiable cells;
/// relative convergence (level change < 5% of the certificate) or the level
/// cap 7 (Decision 3). `+inf` when every cell is flat.
pub fn surface_curvature_radius_lower_span(
    exact: &impl EnclosureSurface,
    budget: &mut Budget,
) -> Result<f64, SurfaceScaleError>;

/// Certified lower bound on min |S(p) - S(q)| over parameter pairs at
/// Chebyshev parameter gap >= gap (Decision 3's qualifying rule). `+inf`
/// when no pair qualifies.
pub fn surface_self_separation_lower_span(
    exact: &impl EnclosureSurface,
    boundary: SurfaceBoundary,
    gap: f64,
    budget: &mut Budget,
) -> Result<f64, SurfaceScaleError>;
```

`sigma_cl` is NOT gated here (the curve packet's Decision 6, unchanged).
rep_surface maps the scale-stage errors: `InvalidMargin -> InvalidMargin`,
`CurvatureUnresolved | SeparationUnresolved -> ReachTooSmall` (the
certification-failure route — the collapsing-geometry analogue of the curve
packet's V-corner; see test 4's pole patch).

### Decision 1 — scale components, computed once

Call `surface_curvature_radius_lower_span` then
`surface_self_separation_lower_span` (Decision 3 spells their internals).
`target_eps = tau_rep.min(tube_scale_lower / 2)`. A small-but-positive
`tube_scale_lower` NEVER refuses (the small-R belt EMITS at target
`min(tau, tube/2)` — test 3). The level cap makes the helpers' values
conservative-certified (BG-FID-007), which only deepens the emit, never
unsound.

**Machine-checked values** (belt = sphere R=2, u in [pi/4, 3pi/4], v in
[0, 2pi], ClosedV, gap = pi): curvature 0.506718 (CAP at level 7, spend
5461), separation 2.757331 (CONV, spend 5461), tube 0.506718, target
0.253359. Open patch [pi/4,3pi/4]^2: curvature 0.593346, separation +inf
(no pair qualifies — the empty-set identity), target 0.296673. Small belt
R=0.3 (same spans): curvature 0.076008, separation 0.413600, target
0.038004. Graph fixture (Decision 9): curvature 0.985244 (CONV, spend 85),
separation +inf, target 0.3. Double cover (Decision 7): curvature 0.399506,
separation 0.000000 (the two sheets coincide — CORRECT), tube 0, target 0.

### Decision 2 — the emitted approximant

Per cell `[a,b] × [c,d]` of the current grid (hu = b−a, hv = d−c), the 4×4
control net `Q[i][j]` (i = u-index 0..3, j = v-index 0..3) from the exact
surface's corner data — positions via `exact.subs`, tangents and twists as
the MIDPOINTS of degenerate enclosures
`exact.enclose_der(m, n, interval_at(u), interval_at(v))` (deterministic; the
curve packet's convention):

```text
P00 = S(a,c)   P30 = S(b,c)   P03 = S(a,d)   P33 = S(b,d)
U** = S_u at the corner (enclosure midpoint)   V** = S_v   W** = S_uv
hu3 = hu/3   hv3 = hv/3   wh = hu*hv/9

Q[0][0]=P00  Q[3][0]=P30  Q[0][3]=P03  Q[3][3]=P33
Q[1][0]=P00+hu3*U00      Q[2][0]=P30-hu3*U30
Q[1][3]=P03+hu3*U03      Q[2][3]=P33-hu3*U33
Q[0][1]=P00+hv3*V00      Q[0][2]=P03-hv3*V03
Q[3][1]=P30+hv3*V30      Q[3][2]=P33-hv3*V33
Q[1][1]=P00+hu3*U00+hv3*V00+wh*W00      # twist + at (a,c)
Q[2][1]=P30-hu3*U30+hv3*V30-wh*W30      # twist - at (b,c)
Q[1][2]=P03+hu3*U03-hv3*V03-wh*W03      # twist - at (a,d)
Q[2][2]=P33-hu3*U33-hv3*V33+wh*W33      # twist + at (b,d)
```

The twist signs come from the mixed second-difference relation (e.g.
`9(Q[1][1]-Q[1][0]-Q[0][1]+Q[0][0])/(hu*hv) = S_uv(a,c)`); the signs
alternate between the corners. Machine-checked: this net reproduces all 16
corner data (4 positions, 4 u-tangents, 4 v-tangents, 4 twists) to 1.6e-15.

Enclosures (all Bernstein hulls of the de Casteljau-RESTRICTED net, padded
outward by the house `64 * EPS * (1 + |coord|)` per endpoint — `HULL_PAD`,
the curve module's constant):

- `enclose(uu, vv)`: for every grid cell whose parameter box contributes
  (the curve module's `cellOverlaps` rule, PER AXIS: interior overlap, or
  the query is a degenerate point on the cell boundary lying inside the
  cell), restrict the net to the intersection (de Casteljau splits per
  axis — the curve `restrict` logic applied per axis; degenerate
  intersection in an axis gives the point column) and join the hulls of
  the 16 restricted points.
- `enclose_der(m, n, uu, vv)`: the m-th u-differences and n-th v-differences
  of the restricted net, scaled by the Bernstein factor
  (m-th difference control points scaled by `3!/(3-m)! / hu^m`, likewise v).
- **Sliver routing (NEW, machine-check-mandated):** an intersection whose
  width in an axis is at or below the width floor `8 * EPS * max(|lo|,
  |hi|, 1)` routes through DIRECT evaluation — the u-derivative column at
  the intersection midpoint (the degree-(3-m) Bernstein evaluation of the
  u-difference net), then the 1D v-curve machinery (`restrict` + derivative
  control points) over the other axis's intersection — because the
  restricted-net derivative scaling DIVIDES by the intersection width and
  explodes on ulp-wide slivers (a query edge landing within ulps of a grid
  knot; measured: without this rule, re-`rep` of an emission collapses its
  curvature certificate to ~0). The house hull pad absorbs the O(sliver)
  variation of the point evaluation. A degenerate query axis is the
  width-0 case of the same rule.
- `immersion_lower_bound`: `immersion_lower_bound_box(&cross_box(du_box,
  dv_box))` (enclosure.rs, pub(crate) — the mignitude-norm form).
- `normal_cone`: `None` always (documented: the emitter provides no cones;
  consumers use `enclose_der` — the HermiteCurve `tangent_cone` precedent).
- `ParametricSurface`: subs/uder/vder/uuder/uvder/vvder/der_mn evaluate the
  containing cell directly (the net's Bernstein evaluation; der_mn via the
  u-derivative column then the v-curve — same construction the sliver rule
  uses); orders above 3 in either axis are zero; `parameter_range` is the
  span.

### Decision 3 — the two span helpers

Both refine UNIFORMLY (quad-split: both axes halve per level), spend one
`spend_subdiv(1)` per split cell, and stop at **relative convergence**
(level-to-level certificate change < 0.05 * certificate) **OR a level cap of
7** — uniform quad refinement is 4^level cells (level 7 = 16384 cells) and
the lfs bound's deficit is LINEAR in cell width, so absolute convergence
would need level ~11 = 4M cells; and an absolute threshold FALSELY converges
on the garbage-small coarse-level certificates (measured: the R=0.3 belt's
level-3 certificate is ~0.001 against a converged ~0.076 — stopping there
would drive the loop to an infeasible target). The capped value is a
certified, more conservative lower bound (BG-FID-007: substituting lower
bounds can only refuse more).

- `surface_curvature_radius_lower_span`: per cell call the LANDED
  `lfs::curvature_radius_lower(surface, (uu, vv))` (pub; do not duplicate
  it). `Err` cells (ImmersionUnresolved / MetricLowerBoundUnresolved) are
  refine-worthy at coarse widths; a refusal AT THE WIDTH FLOOR (both axes
  unsubdividable) is `CurvatureUnresolved` (the pole route — test 4). `+inf`
  when every certifiable cell is flat and none refuse.
- `surface_self_separation_lower_span`: the certificate is the min over
  QUALIFYING cell pairs of `box_distance(exact.enclose(cell_j),
  exact.enclose(cell_k))`. Qualification (the BG-FID-003-r2 max-gap soundness
  argument lifted to 2D): the pair's Chebyshev point-gap qualifies when
  `max(gap_u, gap_v) >= gap`, where each axis's FARTHEST point gap between
  the two intervals is `d_max` (open axis) or, on a closed axis of period P,
  the closed form `d_max if d_max <= P/2 else (P - d_min if d_min >= P/2
  else P/2)` with `d_min = max(0, a_lo - b_hi, b_lo - a_hi)` and `d_max` the
  farthest endpoint distance. The cell pair containing a qualifying point
  pair always qualifies (sound lower bound); extra qualifying pairs only
  lower the certificate (conservative). Partner search: enumerate candidate
  index distances per axis (the qualifying m form a small window around P/2
  on a closed axis; on an open axis m >= gap/w - 1), plus a 2D BVH over
  position boxes with union-box pruning (Decision 6) — an O(N^2) whole-array
  double loop is a review reject at the witnessed cell counts (16384 cells
  at the cap). Budget exhaustion -> `SeparationUnresolved`.

### Decision 4 — the refine loop

```text
scale components (Decision 1); target_eps = min(tau_rep, tube_scale_lower/2)
(du, dv) = (initial_depth, initial_depth)
loop:
    spend_subdiv(1)?            # Budget's own exhaustion; -> Unresolved{subdivisions}
    build the grid (2^du x 2^dv cells) and the HermiteSurface
    measure, per cell, per SURF_MEASURE_SUB=4-per-axis sub-box:
        eps_now  = max over sub-boxes of sup_distance(emitted hull, exact box)
        cell_eps[j] = the per-cell max (the (iv-b)(c) gate is per-cell)
        theta_now = min over sub-boxes of angle_pass_form(
                        cross_box(emitted S_u box, emitted S_v box),
                        cross_box(exact S_u box, exact S_v box))    # normal boxes
        ext_u = max over sub-boxes of (sub-u-width * norm_sup(exact S_u box))
        ext_v likewise
    if eps_now > target_eps:
        stall detection (two consecutive levels improving eps_now < 1%
            relative) -> Unresolved{subdivisions}
        refine the axis with larger (ext_u, ext_v); tie -> u
        continue
    if theta_now <= target_eps / tube_scale_lower:
        refine the larger-extent axis (tie -> u); continue
    discharge (iv-b) (Decision 6) on this grid:
        Pass -> return RepSurfaceOutput { surface, certificate }
        ProjectionFailure -> refine the larger-extent axis (tie -> u); continue
        MultiSheet { cells } -> the refine arm: refine the axis in which the
            failing pair's index distance is ZERO (the non-separating axis:
            its extent inflates cell_eps without widening that pair's gap;
            both nonzero -> larger extent, tie -> u); continue
```

Per-axis refinement is the point: a 2D uniform grid squares the cell count.
`MultiSheet -> refine` is the SPEC's loop mapping ("MultiSheetInTube ->
refine and continue"); a GENUINE double sheet therefore exhausts the budget
or stalls to `Unresolved`, never `Ok` (test 10) — the typed MultiSheet claim
lives on the discharge, which the negative test calls directly (test 9).

**Machine-checked walks** (tau = 0.3, gap = pi, initial depth 0):
belt — eps 4.51 / 1.85 / 1.24 / 0.95 / 0.63 / 0.47 / 0.32 over depths
(0,0)..(2,4), ProjectionFailure at (3,4) and (3,5), **EMIT at (4,5)** (16x32
grid): eps_achieved 0.118463 (2.14x under target), angle_cos_lower 0.784307
(s = 0.5, 1.57x), min non-adjacent separation margin 0.039880,
subdivisions_spent 10. Open patch — EMIT at (4,3): eps 0.118463, spent 8.
Small belt — EMIT at (4,5): eps 0.017769 (target 0.038004, 2.14x), spent
10. Graph — EMIT at (1,1): eps 0.293699 (target 0.3), theta 0.918101
(s 0.304493, 3.02x), spent 3. The ProjectionFailure at intermediate depths
is EXPECTED (Decision 6's first-box requirement: the vertex box must
contract; coarse wu/wv honestly refuse). The exact emitting depths may
shift by the tie rule — assert the RANGES in tests, not exact depths.

### Decision 5 — reuse: what is consumed, what is new

CONSUMED as-is (no edits anywhere outside write_allow):
- `isotopy.rs` pub(crate) items: `interval`, `uniform_cells` (per axis — the
  grid is the product of two 1D uniform cell lists), `box_distance`,
  `sup_distance_box`, `dot_box`, `angle_pass_form`, `norm_sup`.
- `enclosure.rs` pub(crate) items: `cross_box`, `immersion_lower_bound_box`,
  `interval_at`, and the `EnclosureSurface` trait you implement.
- `lfs.rs` pub items: `curvature_radius_lower` (per cell, Decision 3).
- `num/krawczyk.rs`: `krawczyk::<2>` (the generic-N operator — it takes
  `&[Interval; 2]` and your `KrawczykSystem<2>` impl), `KrawczykProof`.
- The curve half of rep.rs: `HULL_PAD`, the pad/hull helpers, `bezier_split`
  style lerp/split helpers, `RepError::into_refusal`'s mapping (copy for the
  new enum).

NEW and local to rep.rs: the 2D grid machinery, the tensor-product net
builders, the 2D BVH (Decision 6), the bivariate Krawczyk system, the
surface scale helpers. Do NOT edit isotopy.rs, lfs.rs, one_sheet.rs, or
enclosure.rs — if you believe you must, that is a SPEC_GAP.

### Decision 6 — the surface (iv-b) discharge

The emitter shares the exact surface's parameter space, so the pairing is
the identity grid. Per cell j (row-major), with H_j the emitted whole-cell
hull box, E_j the exact whole-cell enclosure box:

**(b) grid-vertex projection correspondence.** At every INTERIOR grid vertex
(u*, v*) (1 <= iu < n_u, 1 <= iv < n_v — seam lines are NOT checked on
closed directions; the curve packet checked only interior knots, same
precedent), the emitter interpolates the exact surface exactly
(corner positions are `exact.subs` values), so (u*, v*) IS a root of the
bivariate normal-projection system

```text
F(s, t) = [ <phi(u*,v*) - S(s,t), S_u(s,t)>,
            <phi(u*,v*) - S(s,t), S_v(s,t)> ]
J(s, t) = [ <d, S_uu> - <S_u, S_u>,  <d, S_uv> - <S_u, S_v> ]
           [ <d, S_uv> - <S_v, S_u>,  <d, S_vv> - <S_v, S_v> ]    d = phi - S
```

Certify `KrawczykProof::Unique` via `krawczyk::<2>` over the box
`(u* - wu, u* + wu) x (v* - wv, v* + wv)` with wu, wv the larger of the two
adjacent cell widths in each axis (per-axis; the union closure of the four
adjacent cells — the 2D form of the curve packet's knot neighbourhood).
The preconditioner is the float 2x2 inverse of J at the midpoint
(determinant formula; `None` on zero/non-finite determinant — the operator
bisects on None). `f_point` evaluates the enclosures at DEGENERATE
parameter boxes (the curve KnotProjection precedent). Any verdict other
than `Unique` (Err included) -> `ProjectionFailure`.

**THE FIRST-BOX REQUIREMENT (the BG-FID-008-r3 bisection-edge trap, 2D
edition): once the operator bisects, the root sits exactly ON the children's
shared edge (the split midpoint IS the vertex) and strict-interior
uniqueness is unreachable — the worklist descends forever and refuses at
the floor.** A coarse grid's box (wu or wv too large for the interval
Jacobian to contract) therefore refuses — which is the honest refine
signal; the witnesses' emitting depths have wu, wv small enough that every
interior vertex certifies on its FIRST box (machine-checked: the belt at
(4,5), wu = 0.098, wv = 0.196 — every one of 15*31 = 465 vertices Unique on
box 1; coarser (3,4) refuses). Do NOT add a retry loop; do NOT widen boxes.

**(c) non-adjacent separation.** For every pair (j, k), k non-adjacent to j:
`box_distance(H_j, E_k) <= cell_eps[j]` -> `MultiSheet { cells: (j, k) }`.
Adjacency is CHEBYSHEV-1 in the grid indices — `max(|du_idx|, |dv_idx|) <= 1`
— PLUS wrap per closed direction: `(0, jv)` ~ `(n_u - 1, jv)` when ClosedU,
likewise v (corner-sharing cells share a fibre and MUST be exempt — an
edge-only adjacency rule would fire on every diagonal neighbour). The scan
runs over a 2D BVH local to rep.rs: leaves carry (uu, vv, bb, index);
internal nodes carry the union position box; median split on the widest
position-box axis (the isotopy `build_tree` shape with 2D parameter
leaves); prune a subtree when `box_distance(query, node_union) > cell_eps[j]`
(box-distance to a union is a lower bound for every leaf inside). An
O(N^2) whole-array double loop is a review reject (16384 leaves at the
cap). Expose the scan as a `pub(crate)` function taking the box lists so
the seam test (test 8) can call it directly — the curve module's
`separation_violation` pattern.

(a) own-cell containment is the per-cell measurement itself (cell_eps[j],
from the sub-cell sup distances) — do NOT re-implement it as a radial tube
test.

### Decision 7 — the double-sheet witness (the negative test's fixture)

```text
D(u,v) = (R + a*cos(u/2)) * (sin v * cos u, sin v * sin u, cos v)
domain u in [0, 4*pi] (ClosedU — the azimuth is covered TWICE),
        v in [pi/4, 3*pi/4], R = 2, a = 0.025
```

As u sweeps [0, 4pi] the azimuth wraps twice: sheet 1 (u in [0, 2pi)) and
sheet 2 (u in [2pi, 4pi)) cover the SAME sphere band, at radii
`R + a*cos(azimuth/2)` and `R - a*cos(azimuth/2)` — both within a = 0.025
of R, i.e. STRICTLY inside eps = 0.05 with a 2.0x margin (the BG-FID-003-r2
test-3 trap: a witness whose deviation EQUALS the tolerance is uncertifiable
by design — amplitude eps/2, never eps). The tangent planes deviate from the
sphere's by O(a/R): machine-checked min |cos| between each sheet's normal
and the sphere normal = 0.999961 — "correct tangent planes on BOTH sheets".
The fixture implements `EnclosureSurface` with the interval Leibniz table
(derive_mn by Leibniz over the u-factors; every (m, n) up to (2, 2) is
exercised by the gates). The scale components CORRECTLY certify separation
~0 (the sheets coincide geometrically: machine-checked 0.000000), so
`rep_surface` refuses (Unresolved, test 10) — and the DIRECT discharge call
at the FIXED grid (du, dv) = (7, 5) (128x32 cells) returns
`MultiSheet { cells }` with the failing pair's u-index distance within 2 of
n_u/2 = 64 (machine-checked: cells (0,0) and (63,0) — sheet 1's start and
sheet 2's start, the same sphere region; at (6,5) the first-found pair is a
coarse-cell eps artifact with du_idx 1 — use (7,5), where only sheet pairs
fire: cell_eps 0.0600, non-sheet non-adjacent gaps > 0.2).

### Decision 8 — module layout

ALL of the surface work lands in `rep.rs` (the module's docs extend: the
surface case has LANDED; the curve docs' deferral note updates). `fid/mod.rs`
changes ONLY its module doc line (the `pub mod rep;` line already exists —
the file's `^pub mod` count stays 4; anchors S5/S6 verify). rep.rs keeps
`#![deny(clippy::unwrap_used)]` INCLUDING the test module (GATE-1).
`Refusal`/`EnvelopeCase` from `truck_base::evidence` (the existing import).
Test-only exact surface fixtures live IN the test module (the curve tests'
local-fixture pattern: hand-written interval enclosures on
`crate::elementary`'s outward-rounded cos/sin, the sphere.rs carrier's
per-coordinate product style; `immersion_lower_bound` via the mignitude
form — the fixtures do not need closed forms).

### Decision 9 — tests (all in rep.rs's test module)

All floats named consts with same-line `// H-3:` comments. House witness
conventions: R = 2, tau = 0.3, gap = pi; the belt/patch spans are
[pi/4, 3pi/4] in each direction. Every number below is machine-checked
through THIS packet's formulas (scratch/fid005srf-check.py); the exact
emitting depths may shift by the tie rule — assert ranges where marked.

1. `rep_surface_belt_from_coarse_certifies` — belt (ClosedV), initial_depth
   0: `Ok`; `eps_achieved <= 0.26` (machine value 0.118463);
   `angle_cos_lower >= 0.6` (machine 0.784307 against s = 0.5);
   `depth_u >= 3 && depth_v >= 4` (machine (4,5)); `partition_u.len() >= 9`,
   `partition_v.len() >= 17`; `subdivisions_spent >= 2` (machine 10).
2. `rep_surface_open_patch_certifies` — open patch, Open: `Ok`;
   `eps_achieved <= 0.297` (machine 0.118463); the certificate's scale
   echoes separation = +inf (the empty-set identity).
3. `coarse_belt_refines_and_emits` — small belt R=0.3 (same spans), tau
   0.3: `Ok` with `eps_achieved <= 0.0381` (machine 0.017769; target =
   min(0.3, 0.076008/2) = 0.038004): the over-refusal guard —
   small-but-positive tube_scale EMITS at its own target, never refuses.
4. `pole_patch_routes_to_collapse` — sphere patch u in [0, pi/3], v in
   [pi/4, 3pi/4] (touches the pole: `sin(uu).inf() = 0` at the pole cell at
   EVERY refinement level — immersion lower bound 0), budget 2^12:
   `Err(ReachTooSmall)` and `into_refusal()` yields
   `UnsupportedEnvelope(EnvelopeCase::ReachTooSmall)`. The refusal is the
   scale-stage budget exhaustion (immersion never certifies at the pole) —
   the surface V-corner analogue.
5. `budget_exhaustion_refuses` — measure the belt's scale spend
   (`surface_curvature_radius_lower_span` + `surface_self_separation_lower_span`
   spends; machine value 5461 + 5461 = 10922), hand the loop exactly that
   plus ~2: `Err(Unresolved { subdivisions })` with `subdivisions >= 2` —
   never a best-effort surface.
6. `rep_surface_is_idempotent` — (a) rep the belt twice at tau 0.3: the two
   certificates are EQUAL (determinism: same eps, depths, partitions,
   spend); (b) the metamorphic re-`rep`: rep the EMISSION (it implements
   `EnclosureSurface`): `Ok` with `eps_achieved <= 0.26` (machine 0.118463
   at (4,5) — this is the test that exercises Decision 2's sliver routing:
   the new grid's queries straddle the emission's knots; without the
   routing the emission's own curvature certificate collapses to ~0 and
   the re-rep can never emit).
7. `transposed_parameterization_also_emits` — the open patch and its
   transpose (a fixture wrapper swapping (u, v)): both `Ok`, both
   `eps_achieved <= 0.297` (machine: (4,3) and (3,4) respectively) —
   parameterization robustness, the curve packet's test 7 class.
8. `ivb_separation_failure_reports_multisheet` — the seam test: build the
   belt emission at the machine-emitting grid (du, dv) = (4, 5); compute
   the cell boxes and cell_eps; assert the pub(crate) separation scan finds
   NO violation; then hand-widen ONE exact cell box to a 3-unit cube and
   assert the scan reports a violation whose reported pair includes that
   cell — the direct-call pattern of the curve packet's test 8, exercising
   the (iv-b)(c) arm without contrived geometry.
9. `double_sheet_is_multisheet` — THE negative test: the Decision-7
   fixture; build its emission at the FIXED grid (du, dv) = (7, 5), measure
   cell_eps, call `surface_ivb_discharge` directly: the outcome is
   `MultiSheet { cells }` (NOT Pass, NOT ProjectionFailure) with the pair's
   u-index distance within 2 of n_u/2 (machine: (0,0) x (63,0), distance
   63, n_u/2 = 64). Also assert the witness's own geometry: `a = 0.025 <
   0.05` (the deviation strictly inside eps — the amplitude-halving trap)
   and the fixture's min |cos| to the sphere normal over a dense sample
   `>= 0.999` (machine 0.999961 — correct tangent planes on BOTH sheets).
10. `double_cover_rep_never_emits` — `rep_surface` on the Decision-7
    fixture (ClosedU, tau 0.3, gap pi, budget 2^14): NOT `Ok` (machine:
    separation certificate 0 -> tube 0 -> target 0 -> the stall guard
    returns `Unresolved`) — rep never certifies a double sheet.
11. `invalid_inputs_refuse` — tau = 0 / negative / non-finite; gap = 0 /
    negative / non-finite: `Err(InvalidMargin)` before any budget spend
    (assert `budget.subdiv` unchanged).
12. `rep_surface_family_conditions_hold` — belt, open patch, and the graph
    fixture `(u, v, 0.5 + 0.5*sin(u)*sin(v))` over [pi/4,3pi/4]^2 (Open):
    each `Ok` with `eps_achieved <= 0.3` (machine: graph emits at (1,1),
    eps 0.293699, theta 0.918101 against s 0.304493).

Machine-check every witness number with a script BEFORE writing
RESULT.json (the session-18 lesson), through THIS module's formulas, not a
surrogate: every number quoted above was machine-checked orchestrator-side
through the exact interval formulas this packet mandates; your obligation is
the same check through the module's own code paths (a script in your
worktree, not part of the commit, is fine).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. EVERY
epsilon, radius, angle and slack above is a named const whose defining line
carries a same-line `// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh <base>` before writing RESULT.json.

## Done when — run these, all must pass

The division of labour is worker-fast / verifier-authoritative: your checks
exist to keep YOU honest while iterating; `verify.py` re-establishes every
property authoritatively. Do NOT run workspace-wide commands per iteration —

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib fid --no-fail-fast
cargo check -p truck-evidence
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-evidence is green at baseline. Any baseline failure you did not cause is
a stop condition. Send cargo output to a file and read the tail. Never run a
bare `cargo test`. The full fid test suite gains this packet's 12 tests; the
whole-crate suite must stay green (216+ tests at baseline).

## Forbidden

Editing `fid/mod.rs` beyond the module doc line. Editing isotopy.rs, lfs.rs,
one_sheet.rs, enclosure.rs, num/krawczyk.rs, or any file outside
`write_allow`. Re-implementing `lfs::curvature_radius_lower` (call it).
Re-implementing the 1D pairing/BVH machinery from isotopy.rs (consume the
pub(crate) items). A radial-tube misreading of (iv-b)(a). Sampling-based
measurement of eps_now / theta_now. O(N^2) cell scans in production paths.
Immediate refusal on `2*tau >= tube_scale` (over-refuses refinable geometry —
Decision 1). Returning a surface without its certificate, or (eps, theta)
without (iv). A Krawczyk retry/widening loop at the vertex check (Decision
6's first-box requirement — coarse grids refuse; that is the design).
Denying corner-adjacency (Chebyshev-1) in the separation check. Claiming
isotopy, reach, or any bridge lemma. Bare float literals without `// H-3`.
`unwrap()`/`expect()` on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs -> `ANCHOR_MISMATCH`, naming the anchor (the
  BG-FID-005 landing may have named things differently than this packet
  predicts — that is exactly what S1-S19 detect)
- the landed APIs you consume (`rep_curve`'s helpers, `lfs::curvature_radius_lower`,
  `krawczyk::<2>`, isotopy's pub(crate) items, `EnclosureSurface`) do not
  match this packet's usage -> `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error -> `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,fid): rep_surface with the surface (iv-b) discharge (BG-FID-005-SRF)
```
