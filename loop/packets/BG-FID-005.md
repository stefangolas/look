# WORK PACKET BG-FID-005 — the `rep` operator: refine loop, (iv-b) on the emitter partition (CURVE)

You are implementing one item from a formal kernel specification. Everything
you need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-FID-005","status":"DONE","contracts":["BG-FID-005"],
 "tests_added":10,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it — especially the landed
BG-FID-003-r2 API you consume — say so in `disagreements` rather than making
the code match the packet.**

```yaml
id:          BG-FID-005
contract:    [BG-FID-005]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/rep.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
  - vendor/truck/truck-evidence/src/fid/isotopy.rs   # scoped: ONLY the pub(crate) exposure of Decision 5
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 40, ctx_tokens: 140000}
anchors:
  # Expected counts assume BG-FID-003-r2 landed as specified. The dispatch
  # re-measures every count against the integration tip before launch (H-8).
  - {id: U1, expect: 1, cmd: "grep -c 'pub fn curve_isotopy_conditions' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: U2, expect: 1, cmd: "grep -c 'pub enum CurveBoundary' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: U3, expect: 1, cmd: "grep -c 'pub struct CurveScaleComponents' vendor/truck/truck-evidence/src/fid/isotopy.rs"}
  - {id: U4, expect: 1, cmd: "grep -c 'pub fn fibre_degree_one_auto' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: U5, expect: 3, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: U6, expect: 0, cmd: "grep -c 'pub mod rep' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: U7, expect: 1, cmd: "grep -c 'pub struct FaceScaleComponents' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: U8, expect: 1, cmd: "grep -c 'pub enum KrawczykProof' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
```

## Problem

`rep` is the ONLY sanctioned path from an exact result into the emitted
geometry class: it approximates one exact CURVE component over a certified
partition and returns the achieved error, the achieved tangent margin AND the
degree-one certificate TOGETHER — never a bare curve, and never (eps, theta)
alone, since (eps, theta) without (iv) is precisely the unsound pairing
(condition (i)-(iii) pass on a double cover; nothing above the certificate is
sound if (iv) is missing).

The design point this packet exists to honour: `rep` already subdivides to
hit (eps, theta), so its cell decomposition IS the partition that the (iv-b)
form of the one-sheet condition requires — per-cell fibre-block containment,
per-cell injectivity and non-adjacent separation cost no new subdivision
structure, only new assertions on boxes the loop already computes.
Implementing (iv) as a separate post-pass over an opaque emitted curve is the
expensive way to get the same certificate and is a review reject.

Scope, decided for you: CURVE components (REP-CRV-001). The surface rep
(REP-SRF-001), the surface (iv-b) discharge and the surface double-sheet
negative test are BG-FID-005-SRF, a separate packet; document the deferral in
the module docs, do not stub it.

## Decisions already made for you

### Decision 0 — API and types

```rust
/// Typed refusal. Mirrors the spec's refusal names; converts into the
/// landed §4 `Refusal` (whose `EnvelopeCase::ReachTooSmall` arm is
/// documented for exactly this packet). `Refusal` has no invalid-input
/// arm and is not stretched: garbage input is `InvalidMargin` here.
pub enum RepError {
    /// tau_rep <= 0 / non-finite, arc_gap <= 0 / non-finite, or a
    /// non-finitely-bounded exact span.
    InvalidMargin,
    /// The scale components could not be certified at all (collapsing
    /// geometry: a corner's tangent enclosure contains both branch
    /// directions at every refinement). Routes to §5 collapse via
    /// `into_refusal()`. NEVER fired merely because tube_scale is small:
    /// small-but-positive refines (Decision 3).
    ReachTooSmall,
    /// Refinement did not reach target within budget, or eps stalled above
    /// target at the enclosure width floor. Carries the spend; never a
    /// best-effort curve.
    Unresolved { subdivisions: u32 },
}

impl RepError {
    /// The §4-level view of this refusal.
    pub fn into_refusal(self) -> Refusal; // InvalidMargin has no §4 arm: debug_assert! it never converts, return the nearest arm documenting why
}

/// The emitted approximant: piecewise cubic Hermite in Bezier form over a
/// certified partition (Decision 2). Implements ParametricCurve +
/// EnclosureCurve via the Bernstein hull property, so every downstream
/// consumer (including curve_isotopy_conditions itself) consumes it through
/// the same trait as any other curve.
pub struct HermiteCurve { /* partition knots, per-cell control points, span */ }

/// What rep proved, and what it achieved. This IS the certificate — rep
/// never returns the curve without it.
pub struct RepCertificate {
    /// Certified achieved two-sided sup-distance exact-vs-emitted.
    pub eps_achieved: f64,
    /// Certified min |cos| over all paired tangent boxes (the (ii) margin).
    pub angle_cos_lower: f64,
    /// Final uniform partition depth (2^depth cells).
    pub depth: u32,
    /// The knots, ascending, echo of the certified partition.
    pub partition: Vec<f64>,
    /// Refinement levels spent from the first attempt to the certificate.
    pub subdivisions_spent: u32,
    /// The scale components every gate was evaluated against (echo).
    pub scale: CurveScaleComponents,
}

/// rep_curve's success: the curve AND the certificate, together.
pub struct RepCurveOutput {
    pub curve: HermiteCurve,
    pub certificate: RepCertificate,
}

/// Approximate one exact curve component to tau_rep, certifying (i)-(iii)
/// and discharging (iv-b) on the same partition.
///
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-FEDERER-PATCH | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///   (i)-(iii) of §6.2 between exact and emitted curve
///   + (iv-b) per-cell fibre-block degree-one on the emitted partition
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet as a
///   topological claim | surface case (BG-FID-005-SRF) | reach semantics
pub fn rep_curve(
    exact: &impl EnclosureCurve,
    boundary: CurveBoundary,
    tau_rep: f64,
    arc_gap: f64,
    initial_depth: u32,
    budget: &mut Budget,
) -> Result<RepCurveOutput, RepError>
```

`sigma_cl` is NOT gated here: standalone rep has no arrangement context;
BG-FID-006's consumer adds its condition where it exists.

### Decision 1 — scale components, computed once

Call `curvature_radius_lower_span(exact, budget)` and
`self_separation_lower_span(exact, boundary, arc_gap, budget)` (the landed
BG-FID-003-r2 helpers). Their epistemic refusals (`CurvatureUnresolved`,
`SeparationUnresolved`) propagate as `RepError::ReachTooSmall` — that is the
collapsing-geometry route (a corner refuses here; a small-but-positive bound
does NOT, see Decision 3). `target_eps = min(tau_rep, tube_scale_lower / 2)`
where `tube_scale_lower = scale.tube_scale_lower()`.

### Decision 2 — the emitted approximant

Per cell `[a, b]` of the current uniform partition (h = b − a), with positions
and tangents taken as the MIDPOINTS of the exact curve's degenerate endpoint
enclosures (deterministic; a wrong-but-deterministic choice is correctable,
an unstable one is not):

```text
p0 = X(a),  p3 = X(b)
p1 = p0 + (h/3) * T(a),  p2 = p3 - (h/3) * T(b)      # T = tangent midpoint
```

This is the standard cubic Hermite in Bezier form. Enclosures use the
Bernstein hull property — the curve lies in the hull of its control points:

- `enclose(cell)`: the axis-aligned hull of {p0, p1, p2, p3} over that cell,
  each coordinate padded outward by `64 * EPS * (1 + |coord|)` (the house
  padding from the BG-ENC-003 bspline carrier);
- `enclose_der(1, cell)`: hull of {3(p1-p0), 3(p2-p1), 3(p3-p2)} divided by
  h, same padding;
- `enclose_der(n > 1, cell)`: the corresponding degree-(3-n) difference
  hull, or the zero-width hull of exact differences when 3-n = 0 (a cubic's
  third derivative is constant per cell, its fourth is zero).

Machine-verified error behaviour you may rely on (dense sampling, exact
circle R = 2, tau = 0.05): max radial error per depth 0.336512 (d=0),
0.429204 (d=1 — WORSE than d=0: the long-cell tangent overshoot; expect it),
0.030426 (d=2), 0.001962 (d=3); fourth order from there (ratio ~16). The
tangent margin at d=2: min |cos| = 0.999614 against s = eps/tube = 0.015213.

### Decision 3 — the refine loop

```text
scale components (Decision 1); target_eps = min(tau_rep, tube_scale_lower/2)
depth = initial_depth
loop:
    spend_subdiv(1)?            # Budget's own exhaustion; -> Unresolved{subdivisions}
    build uniform partition (2^depth cells) and the HermiteCurve
    measure eps_now:  max over cells of sup_distance(emitted hull, exact cell box),
                      BOTH directions paired by the identity pairing (Decision 4)
    measure theta_now: min over paired cells of abs_lower(dot)/(sup*sup)  [the (ii) pass form]
    if eps_now > target_eps: depth += 1; continue
    if theta_now <= target_eps / tube_scale_lower: depth += 1; continue     # (ii) gate at achieved eps
    discharge (iv-b) per cell (Decision 4):
        a cell fails -> depth += 1; continue          # the refine arm
    return RepCurveOutput { curve, certificate }
```

- A small-but-positive `tube_scale_lower` NEVER refuses: refinement drives
  eps_now under it (an R=0.08 circle at tau=0.05 EMITS at target 0.04).
- eps stalls above target at the enclosure width floor (two consecutive
  depths barely improve eps_now) -> `Unresolved`.
- Budget exhaustion -> `Unresolved { subdivisions }` — NEVER a best-effort
  curve; returning the best effort here is the tempting bug.
- Every level's eps_now and theta_now are computed by INTERVAL evaluation on
  the cell boxes (the same idioms as BG-FID-003-r2), never by sampling.

### Decision 4 — (iv-b) on the emitter partition (the SAME partition)

The emitter shares the exact curve's parameter space, so cell D_j of the
emitted curve and cell I_j of the exact curve are the SAME interval: the
pairing is the identity and no search is needed. Per cell j, with
`H_j` the emitted hull box, `E_j` the exact enclosure box,
`D'_j`/`X'_j` the first-derivative boxes, `X''_j` the second-derivative box:

1. **fibre-block containment (a).** `sup_distance(H_j, E_j) <= eps_now`
   (already guaranteed by the eps measurement) AND item 3 below. Do NOT
   re-implement (a) as a radial tube test — `||phi(D_j) - X_j|| <= eps`
   re-implements (i) and certifies nothing new; the containment claim is the
   conjunction: within eps of the own cell, beyond eps of every non-adjacent
   one, adjacent cells share only the boundary fibre.
2. **per-cell injectivity (b).** With `s(t)` the projected exact parameter,
   defined implicitly by `<phi(t) - X(s), X'(s)> = 0`:
   `s'(t) = <phi'(t), X'(s)> / (<X'(s), X'(s)> - <phi(t)-X(s), X''(s)>)`.
   You do NOT evaluate this formula. Given (ii) and the tube gate, the
   numerator is sign-definite (|cos| > s > 0 excludes zero) and the
   denominator is positive (`m^2 - eps*K > 0` rearranges to `eps < rho`,
   which the tube gate gives). What you CHECK per cell is the consequence:
   the knot-projection correspondence — every INTERIOR knot t* of the
   partition has its projected parameter within the shared closure of its
   two cells: `s(t*) ∈ [t* - w_j, t* + w_j]` with w_j the certified enclosure
   slack from the implicit-function box (evaluate `<phi(t*) - X(s), X'(s)>`
   over a small s-interval around t* and require the unique zero box to
   touch t*). A knot whose zero box misses its own neighbourhood certifies a
   fold: refuse-and-refine.
3. **non-adjacent separation (c).** For every pair (j, k) with k non-adjacent
   to j (adjacent = |j-k| = 1, PLUS wrap adjacency 0 and n-1 when
   `boundary == Closed`): `box_distance(H_j, E_k) > eps_now`. Use the
   close-pairs BVH query exposed pub(crate) from isotopy.rs (Decision 5) —
   no O(N^2) scan.
4. Any failing cell returns the refine arm (Decision 3). The seam test
   (test 8) exercises this arm directly on a hand-failing input.

### Decision 5 — reuse: the pub(crate) exposure from isotopy.rs

The ONLY edit permitted in isotopy.rs is exposing existing internals as
`pub(crate)` items for rep's use, with zero semantic change: cell-list
construction over a span, the balanced BVH build over cell boxes, the
close-pairs/partner query with a distance threshold, and the per-pair
tangent-box evaluation (the (ii) pass form). Refactoring internals into
these functions is allowed; changing any count, formula, floor or refusal is
not. If the API you need is not factored that way, factor it — do not
duplicate 300 lines of pairing code into rep.rs.

### Decision 6 — module layout

`fid/mod.rs` gains exactly one line `pub mod rep;` (alphabetical: isotopy,
lfs, one_sheet, rep) and its doc line notes the surface case waits on
BG-FID-005-SRF. rep.rs carries `#![deny(clippy::unwrap_used)]` INCLUDING the
test module (GATE-1). `Refusal`/`EnvelopeCase` come from
`truck_base::evidence` (already a dependency — one_sheet.rs's `Budget`
import is the precedent to copy). Test-only exact curve fixtures live IN the
test module following one_sheet.rs's local-curve pattern (hand-written
interval enclosures on crate::elementary's outward-rounded cos/sin; the
V-corner fixture is two line segments).

### Decision 7 — tests (all in rep.rs's test module)

All floats named consts with same-line `// H-3:` comments. House witness
conventions: circle R = 2, tau = 0.05, arc_gap = pi. Every number below is
machine-checked; if your code disagrees, your code or fixture is wrong — or
say so in `disagreements` with the arithmetic.

1. `rep_circle_from_coarse_certifies` — R=2 circle, `Closed`, tau=0.05,
   initial_depth=0: `Ok`, `subdivisions_spent >= 2` (d=0 error 0.336512 and
   d=1 error 0.429204 both exceed target 0.05; d=2 achieves 0.030426 with
   min|cos| 0.999614 > s 0.015213), `eps_achieved <= 0.05`,
   `partition.len() >= 4`. THEN the independent cross-check:
   `curve_isotopy_conditions(exact, Closed, &curve, Closed, eps_achieved +
   slack, &scale, ...)` returns `Ok` — (iv-a) through the landed checker
   AGREES with (iv-b) on the emitted partition.
2. `rep_does_not_emit_at_coarse_depth` — same input, initial_depth=1: the
   certificate's `partition.len() > 2` (it refined past its start).
3. `coarse_circle_refines_and_emits` — R=0.08 circle, tau=0.05: `Ok` with
   `eps_achieved <= 0.04` (target = min(0.05, 0.08/2)): the over-refusal
   guard — small-but-positive tube_scale EMITS, never refuses.
4. `v_corner_routes_to_collapse` — exact = two segments meeting at 60
   degrees (a corner: the tangent enclosure at the corner cell contains
   both branch directions at every refinement): `Err(ReachTooSmall)`, and
   `into_refusal()` yields `UnsupportedEnvelope(ReachTooSmall)`. No value
   produced.
5. `budget_exhaustion_refuses` — a budget with ~2 subdivisions left on the
   R=2 circle from depth 0: `Err(Unresolved { .. })` carrying the spend;
   assert no `Ok` value could be produced.
6. `rep_idempotent_at_same_tolerance` — rep the R=2 circle twice at
   tau=0.05; then `curve_isotopy_conditions(&emit1, Closed, &emit2, Closed,
   tau, &scale1, ...)` returns `Ok`: the two emissions are mutually
   tau-close (idempotence up to tau_rep).
7. `reversed_exact_emits_reversed` — rep on the reversed parameterization:
   the emission is mutually tau-close to the forward emission (isotopy
   conditions Ok, crossed endpoint correspondence expected) — orientation
   robustness, the same regression class BG-FID-003-r2's test 9 guards.
8. `ivb_separation_failure_refines` — the seam test: build a depth-2
   circle's cells, hand-widen ONE exact cell box so a non-adjacent pair
   comes within eps, call the per-cell (iv-b) check directly: it reports the
   separation failure; the loop's mapping spends one subdivision. This
   exercises the refine arm without contrived geometry.
9. `invalid_inputs_refuse` — tau = 0 / negative / non-finite; arc_gap = 0 /
   negative / non-finite: `Err(InvalidMargin)` before any budget spend.
10. `rep_family_conditions_hold` — deterministic family (R=2 circle; ellipse
    (2 cos t, 0.5 sin t, 0); the radial sinusoid exact `(R +
    0.04*sin(8t))*e(t)` — all `Closed`, tau=0.05): each rep returns `Ok` and
    each emission passes `curve_isotopy_conditions` at its achieved eps.

Machine-check every witness number with a script BEFORE writing RESULT.json
(the session-18 lesson), through THIS module's formulas, not a scratch
variant.

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
bare `cargo test`.

## Forbidden

Editing isotopy.rs beyond Decision 5's pub(crate) exposure. Editing files
outside `write_allow`. Implementing the surface case, REP-SRF-001, or any 2D
(iv-b) here. Returning a curve without its certificate, or (eps, theta)
without (iv). Re-implementing (iv-a)'s root machinery (call the landed
checkers). A radial-tube misreading of (iv-b)(a). Sampling-based measurement
of eps_now / theta_now. Immediate refusal on `2*tau >= tube_scale` (that
over-refuses refinable geometry — Decision 3). Duplicating the pairing/BVH
code instead of exposing it. O(N^2) cell scans. Claiming isotopy, reach, or
any bridge lemma. Bare float literals without `// H-3`. `unwrap()`/`expect()`
on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs -> `ANCHOR_MISMATCH`, naming the anchor (the
  BG-FID-003-r2 landing may have named things differently than this packet
  predicts — that is exactly what U1-U5 detect)
- the landed BG-FID-003-r2 API (curve_isotopy_conditions, the span helpers,
  CurveScaleComponents, CurveBoundary) or `fibre_degree_one`'s semantics do
  not match this packet's usage -> `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error -> `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,fid): rep_curve with (iv-b) on the emitter partition (BG-FID-005)
```
