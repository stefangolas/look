# WORK PACKET BG-FID-003-r2 — isotopy conditions (i)-(iv-a), CURVE components

You are implementing one item from a formal kernel specification. Everything
you need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**If your session already read a packet called BG-FID-003 (attempt 1): that
packet was killed by the orchestrator for five design defects before any edit
landed, and this file supersedes it ENTIRELY. Where your memory of that packet
disagrees with this file, this file wins. The five fixes, so you can unlearn
the right things: (1) the tube bound is a COMPOSED scale (curvature AND
self-separation), never curvature alone; (2) the angle condition is between
tangent SPACES (unoriented, absolute dot); (3) the boundary kind is an explicit
input; (4) condition (iv) is established as witnessed (iv-a) only — the
promotion to whole-span is an open lemma, never claimed; (5) cell distances are
box-to-box and partner search is pruned.**

```json
{"id":"BG-FID-003-r2","status":"DONE","contracts":["BG-FID-003"],
 "tests_added":14,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-003-r2
contract:    [BG-FID-003]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/isotopy.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs   # scoped: ONLY Decision 7's addition
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 40, ctx_tokens: 120000}
anchors:
  # Measured under Git Bash on the integration tree at 1c6bf97 (the vendored
  # tree is unchanged since; loop/docs commits do not touch these files).
  # A count mismatch is a stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: T1, expect: 2, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: T2, expect: 0, cmd: "grep -c 'pub mod isotopy' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: T3, expect: 1, cmd: "grep -c 'pub fn fibre_degree_one' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T4, expect: 1, cmd: "grep -c 'fn sup_distance' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T5, expect: 1, cmd: "grep -c 'fn width_floor' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T6, expect: 2, cmd: "grep -c 'fn cross_box' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: T7, expect: 1, cmd: "grep -c 'fn immersion_lower_bound_box' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: T8, expect: 1, cmd: "grep -c 'pub fn face_scale_components' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: T9, expect: 1, cmd: "grep -c 'pub enum KrawczykProof' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: T10, expect: 0, cmd: "grep -c 'pub fn fibre_degree_one_auto' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T11, expect: 1, cmd: "grep -c 'pub struct FaceScaleComponents' vendor/truck/truck-evidence/src/fid/lfs.rs"}
```

Note T3/T4: `fibre_degree_one` exists, `fibre_degree_one_auto` must NOT exist
yet (you add it). T4's `sup_distance` is box-to-POINT (its second operand is a
`Point3`) — see Decision 2(i) for why you must NOT reuse it for boxes.

## Problem

Conditions (i)-(iii) of ??6.2 make the normal projection restricted to an
approximant a proper local homeomorphism — a covering of SOME constant finite
degree. Condition (iv-a) certifies, at ONE witnessed normal disc, that the
fibre multiplicity is one. Together they are DESIGNED to discharge the
hypotheses of [CCS05] Thms 2.1/2.2 — but that discharge is CONDITIONAL on the
open bridge lemmas (Decision 3); this module certifies the CONDITIONS, never
isotopy itself, and never claims more than one witnessed disc for (iv-a): the
promotion of one witnessed fibre to whole-span one-sheetness is the open
L-COVERING lemma's consequence of (i)-(iii), not something this module proves.

Nothing landed today checks (i)-(iii) as a whole. BG-FID-008 (one_sheet.rs)
certifies fibre cardinality on one witnessed disc; BG-FID-001 (lfs.rs) ships
FACE-scale components and the naming discipline this packet now mirrors. This
packet is the consumer: the whole-span conditions checker for one curve
component pair — (i) two-sided closeness, (ii) the tangent-space angle bound
(MANDATORY — Hausdorff closeness alone does not imply isotopy; an approximant
can oscillate inside the tube and be topologically wrong), (iii) endpoint
correspondence at an explicit boundary kind, and (iv-a) via the landed fibre
machinery behind a witness-choosing wrapper (Decision 6).

Scope, decided for you: CURVE components only, one (exact, approx) pair per
call. The surface case and discharge (iv-b) land with BG-FID-005, where the
emitter's cell partition makes them free; document both deferrals in the
module docs, do not stub either.

## Decisions already made for you

### Decision 0 — API and types

```rust
/// The boundary kind of ONE curve component, vouched for by the CALLER.
/// `EnclosureCurve` carries no topology: whether a component's parameter
/// endpoints are identified (Closed) or are genuine boundary (Open) is a
/// claim about the carrier's topology, supplied here as input. This type
/// makes no claim of its own; a wrong claim from the caller is outside
/// this module's certificate (the both-Closed seam gate below detects
/// gross inconsistency, it does not establish closedness).
pub enum CurveBoundary {
    /// Parameter endpoints are the same geometric point (periodic).
    Closed,
    /// Parameter endpoints are genuine boundary points.
    Open,
}

/// Certified scale components for ONE curve component's whole span, named
/// under the BG-FID-001 amendment's rules (see lfs.rs's FaceScaleComponents,
/// the pattern to mirror): each field certifies exactly ONE direction and
/// composes into nothing. `+inf` values are intentional (straight line;
/// empty separation slice).
pub struct CurveScaleComponents {
    /// From `curvature_radius_lower_span`; `+inf` for a straight line.
    pub curvature_radius_lower: f64,
    /// From `self_separation_lower_span`; `+inf` when no cell pair
    /// qualifies at the requested parameter gap.
    pub self_separation_lower: f64,
}

impl CurveScaleComponents {
    /// Plain component-wise minimum (the FaceScaleComponents mirror).
    /// Extended-real: `+inf` components are ignored by `f64::min`.
    pub fn conservative_min(&self) -> f64;

    /// `min(curvature_radius_lower, self_separation_lower / 2)` — the
    /// Federer-motivation composition `reach = min(1/kappa_max, half the
    /// bottleneck)` for a CLOSED curve, used ONLY as the gate bound in the
    /// inequality form (BG-FID-007: substituting a lower bound can only
    /// refuse more). This method claims NO reach semantics: the promotion
    /// of this composition to a tube/reach statement is L-FEDERER-PATCH,
    /// open. The `1/2` is the motivation shape, not a proved equality.
    pub fn tube_scale_lower(&self) -> f64;
}

/// The certified inputs and achieved margins of one whole-span conditions
/// check on one curve component pair.
pub struct IsotopyConditionsReport {
    /// The eps every condition was certified against (the input, echoed).
    pub eps: f64,
    /// The scale components every gate was evaluated against (the input,
    /// echoed). There is deliberately NO bare `rho_lower` field: the
    /// achieved gate bound is `scale.tube_scale_lower()`, and echoing it
    /// as a value would re-claim what the components only motivate.
    pub scale: CurveScaleComponents,
}

/// Typed failures. Every `*Unresolved` arm is EPISTEMIC: a claim about the
/// run, never about the geometry. The `*Violation`/`MultiSheet` arms are
/// POSITIVE certified claims that the condition fails.
pub enum IsotopyConditionsError {
    /// eps <= 0, non-finite eps, a parameter span not finitely bounded on
    /// either curve, or (on the separation helper) arc_gap <= 0 / non-finite.
    InvalidMargin,
    /// `2*eps >= scale.tube_scale_lower()`: the tube budget exceeds the
    /// composed certified bound. EPISTEMIC per spec: the BOUND could not be
    /// certified large enough — it says nothing about the geometry.
    ReachLowerBoundTooSmall,
    /// (i) certified failed: a floor-width cell box has certified distance
    /// > eps to EVERY cell of the other curve.
    ClosenessViolation { witness_cell: Interval },
    /// (ii) certified failed: a paired cell box exhibits a tangent pair
    /// whose SPACE angle reaches the bound (Decision 2's two-sided test).
    AngleViolation { approx_cell: Interval, exact_cell: Interval },
    /// (iii) certified failed: boundary kinds disagree (one Closed, one
    /// Open — circle-vs-interval is not isotopy and no geometric endpoint
    /// check can catch it), an endpoint of one curve is > eps from every
    /// endpoint of the other, or a both-Closed input fails the seam
    /// consistency gate.
    BoundaryMismatch,
    /// (iv-a): the witnessed disc met the approximant a certified count
    /// != 1 times (`count == 0` is the coverage-violation arm).
    MultiSheet { count: usize },
    /// (i) could not decide within budget / width floor.
    ClosenessUnresolved,
    /// (ii) could not decide within budget / width floor.
    AngleUnresolved,
    /// (iv-a) propagated from BG-FID-008: root isolation unresolved.
    DegreeOneUnresolved,
    /// (iv-a) propagated from BG-FID-008: bad witness (all ladder points
    /// refused).
    InvalidWitness,
    /// `curvature_radius_lower_span` could not certify a positive immersion
    /// bound at any refinement (epistemic; returning `+inf` here would be
    /// the over-estimate this crate must never produce).
    CurvatureUnresolved,
    /// `self_separation_lower_span` could not complete within budget
    /// (epistemic).
    SeparationUnresolved,
}

pub fn curve_isotopy_conditions(
    exact: &impl EnclosureCurve,
    exact_boundary: CurveBoundary,
    approx: &impl EnclosureCurve,
    approx_boundary: CurveBoundary,
    eps: f64,
    scale: &CurveScaleComponents,
    budget: &mut Budget,
) -> Result<IsotopyConditionsReport, IsotopyConditionsError>
```

Naming discipline: the module is `isotopy.rs`, but NOTHING in the API claims
isotopy or reach — `curve_isotopy_conditions` certifies the CONDITIONS; the
conditions-to-isotopy step is the open lemma chain; `tube_scale_lower` is a
gate bound, not a reach. Every public item carries the annotation block from
Decision 4.

The two span helpers are published (the certificate producers), so callers —
including BG-FID-005 later — compose their own inputs:

```rust
/// Certified lower bound on the exact curve's minimum curvature radius over
/// its whole span: `1 / kappa_upper` with
/// `kappa_upper = sup_t |X' x X''| / (inf_t |X'|)^3`. `+inf` when the
/// numerator bracket is 0 (a straight line). Uses
/// `crate::enclosure::cross_box` and `immersion_lower_bound_box` (both
/// already `pub(crate)`) — do NOT duplicate them locally. A span whose
/// tangent enclosure contains zero at every refinement refuses
/// `CurvatureUnresolved` (never `+inf`: that would claim straightness).
pub fn curvature_radius_lower_span(
    exact: &impl EnclosureCurve,
    budget: &mut Budget,
) -> Result<f64, IsotopyConditionsError>

/// Certified lower bound on `min |X(s) - X(t)|` over parameter pairs at
/// certified PARAMETER gap >= arc_gap: partition the span by bisection;
/// for every pair of cells (I, J) whose parameter gap qualifies, the
/// box-to-box distance of the position enclosures is a certified lower
/// bound on the arc-to-arc distance; the minimum over qualifying pairs is
/// the certificate. PARAMETER-gap semantics, stated in the doc: with a
/// derivative lower bound m, parameter gap G implies arc gap >= m*G, and a
/// consumer wanting arc gap A passes G = A/m. For `CurveBoundary::Closed`
/// the parameter gap is the WRAPPED distance `min(|s-t|, span-|s-t|)`
/// (a closed loop's two sides both count); for Open it is `|s-t|`.
/// `+inf` when no pair qualifies (the empty-set identity, e.g. any curve
/// with arc_gap >= span). arc_gap <= 0 / non-finite -> InvalidMargin.
/// Budget exhaustion -> SeparationUnresolved.
pub fn self_separation_lower_span(
    exact: &impl EnclosureCurve,
    boundary: CurveBoundary,
    arc_gap: f64,
    budget: &mut Budget,
) -> Result<f64, IsotopyConditionsError>
```

Both helpers carry the same annotation block as the components struct.

### Decision 2 — the whole-span conditions, all by interval evaluation

**(i) two-sided eps-closeness, by cell pairing with BOX-BOX distances.**
Partition each span by bisection (shared `Budget`). For a cell box `A` of one
curve and a cell box `B` of the other, both distances are box-to-box:

- **sup-distance (farthest corner pair)**, per coordinate:
  `max(|a_lo - b_hi|, |a_hi - b_lo|)`, then `sqrt(sum_i of squares)` —
  `a_lo/a_hi` are the interval's endpoints per axis. Do NOT reuse
  `one_sheet::sup_distance`: that helper's second operand is a `Point3`
  (box-to-point); a box operand needs the form above (a point box is the
  degenerate case `b_lo == b_hi`). Duplicate the box-box form locally.
- **inf-distance (nearest pair)**, per coordinate:
  `gap_i = max(0, a_lo - b_hi, b_lo - a_hi)`, then `sqrt(sum_i gap_i^2)`.
  (`one_sheet::box_distance` already has exactly this box-box form —
  duplicate it locally as the sibling of the sup form.)

Sound pairing rule, BOTH directions (approx->exact and exact->approx): every
cell must find a partner cell of the other curve with `sup_distance <= eps` —
this certifies `sup_t d(X(t), X') <= eps` because for ANY point of the cell
and ANY point of the partner, the distance is `<= eps`. A cell with no partner
subdivides; at the width floor, if its box has `box_distance > eps` to EVERY
cell box of the other curve, that is a certified `ClosenessViolation`; still
undecided at the floor is `ClosenessUnresolved` (epistemic). Either direction
may fire the violation.

**Search structure is mandated, not stylistic.** Build per curve a balanced
binary tree over the cell boxes (median split on the widest-interval axis).
A partner query for cell A descends the other curve's tree pruning any node
whose own box has `box_distance(A, node_box) > eps` — sound because
`box_distance > eps` implies `sup_distance > eps`, so no partner hides
inside. Leaf candidates are tested with `sup_distance <= eps`. The same tree
serves the separation helper (best-so-far pruning: skip node pairs whose
`box_distance >= current best`, which cannot lower the minimum, and skip
pairs failing the parameter-gap precondition). **Any O(N*M) whole-array
double loop over cells — in (i), (ii) or the separation helper — is a review
reject**: the witnessed sinusoid refines to ~1.6e4 cells (machine-checked),
and the ??=4000 test must run in seconds, not minutes.

**(ii) the tangent-SPACE angle condition, MANDATORY, on paired cells.** The
angle is between unoriented tangent SPACES (`cos angle = |a??e| / (|a||e|)`,
range [0, pi/2]) — the same geometry reversed must behave identically. For
every pairing certified in (i), over first-derivative boxes
`D' = approx.enclose_der(1, cell)`, `D = exact.enclose_der(1, partner)`, with
`s = eps / scale.tube_scale_lower()`, checked in cosine form, both sides
sound:

- pass: `abs_lower(dot_box(D', D)) / (norm(D').sup() * norm(D).sup()) > s`
  proves every tangent pair in the boxes has `|cos| > s` (denominators only
  shrink);
- violation: `abs_upper(dot_box(D', D)) / (norm(D').inf() * norm(D).inf()) <= s`
  proves every pair in the boxes has `|cos| <= s` — a certified
  `AngleViolation` (stronger than "some pair", and that is fine);
- strictly between: subdivide the pair; floor — `AngleUnresolved`.

where for an interval `I = [lo, hi]` (the dot box is 1-dimensional):

```rust
fn abs_lower(I) -> f64 { if I.contains(0.0) { 0.0 } else { I.inf().abs().min(I.sup().abs()) } }
fn abs_upper(I) -> f64 { I.inf().abs().max(I.sup().abs()) }
```

A derivative box whose norm infimum is 0 (contains the zero vector) cannot be
tested: subdivide it; at the floor — `AngleUnresolved` (epistemic — the
tangent direction is not certifiable there). `arccos(c) = pi/2 - asin(c)` for
`c — [0,1]` is what makes the cosine form identical to the spec's angle form;
do NOT take any `acos`/`asin` in code. Condition (ii) consumes EXACTLY the
pairs (i) certified — it never scans cells on its own.

**(iii) endpoint correspondence, at an explicit boundary kind.** With
`E_lo = enclose(degenerate(lo))`, `E_hi = enclose(degenerate(hi))` for each
curve (degenerate point boxes: sup- and inf-distance coincide):

- Kinds must AGREE: `exact_boundary != approx_boundary` — `BoundaryMismatch`
  (a closed exact with an open approx is circle-vs-interval: not isotopic,
  and a purely geometric endpoint check CANNOT catch it — a near-full-circle
  open approx passes every endpoint-distance test at sub-eps seam gap).
- Both Open: every endpoint point-box of either curve has
  `sup_distance <= eps` to SOME endpoint point-box of the other. Crossed
  correspondences (start <-> end) are fine — orientation is combinatorial,
  not certified here.
- Both Closed: the same endpoint correspondence, PLUS the seam consistency
  gate per curve: `box_distance(E_lo, E_hi) <= 2*eps` for each curve of the
  pair. This is a consistency gate on the caller's Closed claim (a truly
  closed curve's two endpoint enclosures contain the same geometric point,
  so they are ~0 apart), NOT a closedness certificate — the doc comment
  says both sentences verbatim.
- Failure of any gate — `BoundaryMismatch`.

Closure of either curve is NEVER claimed as a topological fact; the carrier
owns its own topology (the doc comment says so).

**(iv-a) one sheet, by the landed evidence, behind the auto wrapper.** Call
`fibre_degree_one_auto(exact, approx, eps, budget)` (Decision 6) — the
witness choice is one_sheet-internal; this module never picks `t_x` and
carries no bisection-edge folklore. Map `Ok(ExactlyOne)` — continue;
`Ok(NotOne { count })` — `MultiSheet { count }`;
`Err(SheetCountUnresolved)` — `DegreeOneUnresolved`;
`Err(InvalidWitness)` — `InvalidWitness`. Do NOT reimplement any part of the
fibre machinery.

Order of evaluation: `InvalidMargin` checks (eps, finite spans) — the tube
gate `2*eps >= scale.tube_scale_lower()` — `ReachLowerBoundTooSmall` —
(i) — (iii) — (ii) — (iv-a). (iv-a) is LAST because its decisiveness hangs
on (i)-(iii) holding (the L-COVERING dependency, Decision 3) — the landed
`fibre_degree_one` documents that precondition and this order honours it.
All four gates must hold for `Ok`; the report echoes `eps` and `scale`.

### Decision 3 — the bridge lemmas, as a structured comment (NOT code)

Copy this block verbatim into the module docs, above the public function,
marking each lemma with its status. It is the certificate site the spec
amendment requires; a future packet discharges or refutes each lemma:

```text
L-TUBE       eps < reach(X) => the closed eps-tube of a compact C??
             surface-with-boundary is a topological thickening whose sides
             are the offset sheets. STATUS: OPEN (closed case = classical
             tubular neighborhood theorem; the with-boundary restriction is
             ours).
L-FEDERER-PATCH  a cell at certified distance h from its trimmed boundary,
             curvature bounded above by K, and certified exclusion of
             non-incident sheets within radius r has a single-valued normal
             tube of radius min(1/K, r, h). STATUS: OPEN — until it lands,
             CurveScaleComponents and tube_scale_lower() are certified
             COMPONENTS and a gate bound, never reach.
L-COVERING   transversality/local-inverse (ii) + properness + certified
             fibre multiplicity one (iv) => the fibre projection is a
             ONE-SHEETED COVERING => homeomorphism. STATUS: OPEN. The
             promotion of ONE witnessed fibre to whole-span one-sheetness
             is exactly this lemma's consequence of (i)-(iii): NOT proved
             here, NOT claimed here.
L-SEPARATES  a continuous one-sheet SECTION of the product thickening
             separates the thickening's sides; the section property comes
             from L-COVERING's homeomorphism inverse. STATUS: OPEN.
Chain: (i)-(iii) + (iv) => local homeomorphism => covering => homeomorphism
        => continuous section => side separation => CCS05 Thm 2.1 isotopy.
THIS MODULE ESTABLISHES THE CONDITIONS. THE CHAIN IS NOT A PROOF UNTIL THE
LEMMAS LAND.
```

### Decision 4 — annotations (@feeds form, mandatory)

Every public item carries immediately above it (the landed house form — see
lfs.rs; one_sheet.rs's items are the other live examples):

```rust
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///   conditions (i)-(iii) of ??6.2 on ONE curve component pair
///   + (iv-a): certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet (iv) |
///   surface case | (iv-b) | reach semantics for the scale components
```

The scale components struct and both span helpers use the narrower block:
`@via-open-lemma FID-L-FEDERER-PATCH`,
`@establishes component-wise certified directions`,
`@does-not-establish reach | tube width | lfs` (lfs.rs's
FaceScaleComponents doc is the wording to mirror). A definition citation is
not a theorem instance. Wrong tags are `disagreements` findings.

### Decision 5 — module layout

`fid/mod.rs` gains exactly one line `pub mod isotopy;` (alphabetical) and its
doc line notes the surface case and (iv-b) wait on BG-FID-005. isotopy.rs
carries `#![deny(clippy::unwrap_used)]` INCLUDING the test module (GATE-1).
`sup_distance_box`/`box_distance`/`dot_box`/`abs_lower`/`abs_upper` are
duplicated locally exactly as one_sheet.rs duplicates its helpers —
enclosure.rs visibility stays untouched. Test-only curve structs live IN the
test module following one_sheet.rs's local-curve pattern (implement
ParametricCurve + EnclosureCurve with hand-written interval enclosures built
on crate::elementary's outward-rounded cos/sin).

### Decision 6 — `fibre_degree_one_auto` (the ONLY permitted edit to one_sheet.rs)

Witness selection is one_sheet-internal knowledge; this packet adds the
wrapper so callers stop choosing `t_x`:

```rust
/// fibre_degree_one with the witness chosen for you: a deterministic
/// ladder of exact-span points (midpoint first), stopping at the first
/// ladder point whose call RETURNS (Ok or a non-witness error).
///
/// @feeds-open-lemma FID-L-COVERING      # degree-one fibre evidence, per component
/// @establishes certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet
///
/// Ladder (fractions of the exact span (lo, hi)): 1/2, 1/4, 3/4, 1/8, 7/8,
/// 1/3, 2/3, 1/6, 5/6 — computed as lo + f*(hi - lo), no RNG, stable
/// order. Retry on `SheetCountUnresolved` AND on `InvalidWitness` (a
/// midpoint whose tangent enclosure contains zero — e.g. a cusp at
/// midspan — is a bad WITNESS, not bad input; eps validity has already
/// been checked by then). If every rung refuses: return
/// `SheetCountUnresolved` if any rung produced it, else `InvalidWitness`.
/// Every rung spends from the SAME budget — a caller wanting per-rung
/// isolation pre-reserves.
pub fn fibre_degree_one_auto(
    exact: &impl EnclosureCurve,
    approx: &impl EnclosureCurve,
    eps: f64,
    budget: &mut Budget,
) -> Result<FibreMultiplicity, OneSheetError>
```

Scope fence: in one_sheet.rs you may add ONLY this function, its doc
comment, and its tests. No edit to `fibre_degree_one`, the enums, the
helpers, or any existing test. (isotopy.rs and fid/mod.rs are otherwise
yours per write_allow.)

### Decision 7 — tests (isotopy.rs test module unless named otherwise)

All floats named consts with same-line `// H-3:` comments. Reuse the landed
witness conventions: exact circle radius `R = 2`, `eps = 0.05`. Every
number below is machine-checked (the orchestrator's script); if your
implementation disagrees with one, YOUR code or YOUR fixture is wrong —
or say so in `disagreements` with the arithmetic.

1. `single_sheet_circle_conditions_hold` — exact circle over `[0, 2pi]`,
   approx circle radius `R + eps/2`, both `Closed`; scale built from the
   two helpers with `arc_gap = pi` (parameter): expect
   `curvature_radius_lower` near 2, `self_separation_lower` near 4
   (`2R sin(pi/2)`), `Ok(_)` with the report echoing eps and scale.
2. `radial_sinusoid_fails_angle_condition` — THE motivating (ii) case:
   approx `(R + a*sin(omega*t))*e(t)` over `[0, 2pi]`, both `Closed`,
   `a = 0.04 <= eps`, `omega = 4000`. (i) passes (two-sided distance = a);
   (ii) must fail: extreme deviation `atan(80) = 1.5582969778` rad exceeds
   `pi/2 - asin(eps/R) = 1.5457937219` rad; in cosine form
   `|cos| = 1/sqrt(6401) = 0.0124990236 <= s = eps/2 = 0.025` with margin
   `0.0125` (decisive at cell width ~1e-4; (i)'s own cells at ~3.95e-4
   straddle, so expect at most ~2 extra (ii) subdivision levels — if this
   test takes minutes, your pairing is unpruned: bug, not slow machine).
   Expected: `AngleViolation`.
3. `double_cover_is_multisheet` — the landed BG-FID-008 witness verbatim:
   `(R + eps*cos(t/2))*e(t)` over `[0, 4pi]`, both `Closed`. (i), (iii)
   hold, (ii) holds (tangent deviation `O(eps/R)`); expected
   `MultiSheet { count: 2 }`.
4. `trimmed_approx_boundary_mismatch` — two open segments: exact line
   segment over `[0, 1]`, approx the same over `[0.1, 1]` (endpoint
   displaced by `0.1*|X'| > eps`): expected `BoundaryMismatch`.
5. `coarse_radius_refuses` — exact circle of radius `0.08`, `eps = 0.05`,
   helper-built scale: `tube_scale_lower = 0.08`, `2*eps = 0.1 >= 0.08` —
   `ReachLowerBoundTooSmall` (the epistemic refusal).
6. `invalid_margin_refuses` — `eps = 0`, negative eps, non-finite eps;
   separation helper with `arc_gap = 0`, negative, non-finite.
7. `zero_budget_refuses_unresolved` — empty budget — an `*Unresolved` arm
   (assert it is one of `ClosenessUnresolved` / `AngleUnresolved` /
   `DegreeOneUnresolved`).
8. `line_pair_conditions_hold` — two straight parallel segments offset by
   `eps/2`, both `Open`: `curvature_radius_lower = +inf` AND
   `self_separation_lower = +inf` (no pair at any arc_gap < span) exercise
   the extended-real path; `s = eps/+inf = 0`; `Ok(_)`.
9. `reversed_parameterization_matches_forward` — the (ii) unoriented
   regression: test 1's pair with the approx traversed BACKWARDS
   (`approx_rev(t) = approx(2pi - t)`); must `Ok(_)` with the same report
   fields. A signed-dot angle test fails this — that is the bug it exists
   to catch. Crossed endpoint correspondence is expected and fine.
10. `closed_exact_open_approx_mismatches` — the (iii) kind gate's flagship:
    exact circle over `[0, 2pi]` `Closed`; approx the same circle over
    `[0, 2pi - 0.001]` `Open`. Endpoint gap `2R sin(0.0005) = 0.002 < eps`,
    so every GEOMETRIC endpoint check passes and only the kind gate fires:
    expected `BoundaryMismatch`.
11. `hairpin_scale_refuses_on_separation` — the composed-scale gate: hand
    -construct `CurveScaleComponents { curvature_radius_lower: 10.0,
    self_separation_lower: 0.12 }` (a hairpin with a gentle far-away
    turnaround: curvature radius ~10, strand gap 0.12) and run test 1's
    circle pair with it: `tube_scale_lower = min(10, 0.06) = 0.06 < 2*eps`
    while `2*eps = 0.1 < 10` — the refusal is attributable to SEPARATION
    alone; expected `ReachLowerBoundTooSmall`. (This is the case a
    curvature-only bound silently admits.)
12. `ellipse_separation_soundness` — `self_separation_lower_span` on the
    ellipse `(2 cos t, 0.5 sin t, 0)` over `[0, 2pi]`, `Closed`,
    `arc_gap = 2.0`: the certified value must be `<= 0.84179354` (brute
    -force reference over a 4000x4000 wrapped grid, min at parameter gap
    exactly 2 through the wrap side) and `>= 0.75` (usefulness floor,
    strictly below the true value — a helper returning 0 passes soundness
    vacuously and is its own bug).
13. `auto_witness_certifies_single_sheet` (one_sheet.rs test module) —
    `fibre_degree_one_auto` on the landed single-sheet circle fixture —
    `ExactlyOne`.
14. `auto_witness_double_cover_not_one` (one_sheet.rs) — auto on the
    landed double-cover fixture — `NotOne { count: 2 }`.

Machine-check every witness number with a script BEFORE writing RESULT.json
(the session-18 lesson) — through THIS module's formulas, not a scratch
variant (the FID-001 lesson: check WHICH code path produced the numbers).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. EVERY
epsilon, radius, angle and slack above is a named const whose defining line
carries a same-line `// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh <base>` before writing RESULT.json.

## Done when — run these, all must pass

The division of labour is worker-fast / verifier-authoritative: your checks
exist to keep YOU honest while iterating; `verify.py` re-establishes every
property authoritatively (V2 build, V3 lint, V5 whole-crate tests, V8
downstream, V9 geometry). Do NOT run workspace-wide commands per iteration —

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib fid --no-fail-fast
cargo check -p truck-evidence
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-evidence is green at baseline. Any baseline failure you did not cause is
a stop condition. Send cargo output to a file and read the tail. Never run a
bare `cargo test`.

## Forbidden

Editing one_sheet.rs beyond Decision 6's fenced addition. Editing files
outside `write_allow`. Implementing the surface case or (iv-b) here. Claiming
isotopy, homeomorphism, reach, tube width, lfs, or any bridge lemma as proved,
or tagging anything "Thm instance". A signed (oriented) angle test. Computing
a curvature-only tube bound. Picking `t_x` or simulating bisection edges in
isotopy.rs. Sampling-based (point) checks of (i), (ii) or the separation —
the spec's own rule: the bound is over the WHOLE SPAN by interval evaluation,
and sampling passes on precisely the inputs that matter. Taking `acos`/`asin`
in the (ii) comparison (the cosine form is exact and cheaper). Bare float
literals without `// H-3`. `unwrap()`/`expect()` on fallible production
paths. O(N*M) whole-array cell scans. Committing to `main`.

## Stop conditions

- an anchor count differs — `ANCHOR_MISMATCH`, naming the anchor
- the landed `fibre_degree_one` signature or `FibreMultiplicity`/
  `OneSheetError` semantics do not match Decision 2(iv-a)'s mapping —
  `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error — `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,fid): whole-span isotopy conditions (i)-(iv-a) for curve components (BG-FID-003-r2)
```
