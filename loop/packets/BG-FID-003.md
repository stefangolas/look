# WORK PACKET BG-FID-003 — isotopy conditions (i)-(iv), CURVE components

You are implementing one item from a formal kernel specification. Everything
you need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-FID-003","status":"DONE","contracts":["BG-FID-003"],
 "tests_added":8,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-003
contract:    [BG-FID-003]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/isotopy.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 40, ctx_tokens: 120000}
anchors:
  # Measured under Git Bash on integration HEAD 1c6bf97 at dispatch-ready
  # time (after BG-FID-008-r4 landed). A count mismatch is a stop condition
  # (ANCHOR_MISMATCH), not a nuisance.
  - {id: T1, expect: 2, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: T2, expect: 0, cmd: "grep -c 'pub mod isotopy' vendor/truck/truck-evidence/src/fid/mod.rs"}
  - {id: T3, expect: 1, cmd: "grep -c 'pub fn fibre_degree_one' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T4, expect: 1, cmd: "grep -c 'fn sup_distance' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T5, expect: 1, cmd: "grep -c 'fn width_floor' vendor/truck/truck-evidence/src/fid/one_sheet.rs"}
  - {id: T6, expect: 2, cmd: "grep -c 'fn cross_box' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: T7, expect: 1, cmd: "grep -c 'fn immersion_lower_bound_box' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: T8, expect: 1, cmd: "grep -c 'pub fn face_scale_components' vendor/truck/truck-evidence/src/fid/lfs.rs"}
  - {id: T9, expect: 1, cmd: "grep -c 'pub enum KrawczykProof' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
```

## Problem

Conditions (i)-(iii) of §6.2 make the normal projection restricted to an
approximant a proper local homeomorphism — a covering of SOME constant finite
degree. Condition (iv) certifies the degree is one. Together they are DESIGNED
to discharge the hypotheses of [CCS05] Thms 2.1/2.2 — but that discharge is
CONDITIONAL on three open bridge lemmas (see the structured-comment block
below); this module certifies the CONDITIONS, never isotopy itself.

Nothing landed today checks (i)-(iv) as a whole. BG-FID-008 (one_sheet.rs)
certifies (iv-a) for one witnessed disc on a curve; BG-FID-001 (lfs.rs) ships
FACE-scale components. This packet is the consumer: the whole-span conditions
checker for one curve component pair, discharging (i) two-sided closeness,
(ii) the tangent-angle bound (MANDATORY — Hausdorff closeness alone does not
imply isotopy; an approximant can oscillate inside the tube and be
topologically wrong), (iii) endpoint correspondence, and (iv) by calling the
landed `fibre_degree_one`.

Scope, decided for you: CURVE components only, one (exact, approx) pair per
call. The surface case and discharge (iv-b) land with BG-FID-005, where the
emitter's cell partition makes them free; document both deferrals in the
module docs, do not stub either.

## Decisions already made for you

### Decision 0 — API and types

```rust
/// The certified inputs and achieved margins of one whole-span
/// conditions check on one curve component pair.
pub struct IsotopyConditionsReport {
    /// The eps every condition was certified against (the input, echoed).
    pub eps: f64,
    /// The certified lower bound on the exact curve's minimum curvature
    /// radius over its span (extended real: +inf for a straight line).
    pub rho_lower: f64,
}

/// Typed failures. Every `*Unresolved` arm is EPISTEMIC: a claim about the
/// run, never about the geometry. The `*Violation`/`MultiSheet` arms are
/// POSITIVE certified claims that the condition fails.
pub enum IsotopyConditionsError {
    /// eps <= 0, non-finite eps, or a parameter span that is not finitely
    /// bounded on either curve.
    InvalidMargin,
    /// 2*eps >= rho_lower: the tube budget exceeds the certified curvature
    /// radius (the BOUND was too small; says nothing about the geometry —
    /// refine rho_lower or shrink eps).
    ReachLowerBoundTooSmall,
    /// (i) certified failed: a floor-width approx (resp. exact) cell box has
    /// certified distance > eps to EVERY cell of the other curve.
    ClosenessViolation { witness_cell: Interval },
    /// (ii) certified failed: a paired cell box exhibits a tangent pair
    /// whose angle reaches the bound (see Decision 2's two-sided test).
    AngleViolation { approx_cell: Interval, exact_cell: Interval },
    /// (iii) certified failed: an endpoint of one curve is > eps from every
    /// endpoint of the other, or the curves disagree on closure.
    BoundaryMismatch,
    /// (iv): the witnessed disc met the approximant a certified count != 1
    /// times (`count == 0` is the coverage-violation arm).
    MultiSheet { count: usize },
    /// (i) could not decide within budget / width floor.
    ClosenessUnresolved,
    /// (ii) could not decide within budget / width floor.
    AngleUnresolved,
    /// (iv) propagated from BG-FID-008: root isolation unresolved.
    DegreeOneUnresolved,
    /// (iv) propagated from BG-FID-008: bad witness parameter.
    InvalidWitness,
}

pub fn curve_isotopy_conditions(
    exact: &impl EnclosureCurve,
    approx: &impl EnclosureCurve,
    eps: f64,
    budget: &mut Budget,
) -> Result<IsotopyConditionsReport, IsotopyConditionsError>
```

Naming discipline: the module is `isotopy.rs` (the registry's name), but
NOTHING in the API claims isotopy — `curve_isotopy_conditions` certifies the
CONDITIONS; the conditions-to-isotopy step is the open lemma chain. Every
public item carries the annotation block from Decision 4.

### Decision 1 — rho_lower, computed once, exposed

```rust
/// Certified lower bound on the exact curve's minimum curvature radius over
/// its whole span: `rho_lower = 1 / kappa_upper` with
/// `kappa_upper = sup_t |X' x X''| / (inf_t |X'|)^3` (the parametrization-
/// honest curvature kappa = |X' x X''| / |X'|^3). +inf when the numerator
/// bracket is 0 (a straight line). Duplicate `cross_box` usage via
/// `crate::enclosure::cross_box` and `immersion_lower_bound_box` (both
/// `pub(crate)`, same crate — do NOT duplicate them locally).
pub fn curvature_radius_lower_span(
    exact: &impl EnclosureCurve,
    budget: &mut Budget,
) -> Result<f64, IsotopyConditionsError>
```

Evaluate over a bisection partition of the span: per cell, the numerator is
`norm(cross_box(d1, d2)).sup()` (interval norm, read from the UPPER endpoint —
an over-estimate is sound here, this bounds curvature FROM ABOVE), the
denominator is `immersion_lower_bound_box(d1)` **cubed, once globally** (the
inf over cells — refine until the box mignitudes are positive, refuse
`ClosenessUnresolved`-style at the floor with `AngleUnresolved`... no: use
`InvalidMargin` only for inputs; a span whose tangent enclosure contains zero
at every refinement refuses `ReachLowerBoundTooSmall` — the epistemic reading:
the radius bound could not be certified). `kappa_upper` = max over cells;
`rho_lower = 1/kappa_upper` (`+inf` when `kappa_upper == 0`). The same value
feeds BOTH `2*eps < rho_lower` and Decision 2's angle bound — one source of
truth, no drift between the two uses.

### Decision 2 — the three whole-span conditions, all by interval evaluation

**(i) two-sided eps-closeness, by cell pairing.** Partition each span by
bisection (shared `Budget`). For a cell box `B'` of one curve and a cell box
`B` of the other:

- **sup-distance** `sup_distance(A, B) = sqrt(sum_i max((a_lo−b_i)^2, (a_hi−b_i)^2))`
  (farthest corner pair; duplicate locally exactly as one_sheet.rs does);
- **inf-distance** `box_distance(A, B)` (nearest pair; duplicate locally).

Sound pairing rule, both directions (approx→exact and exact→approx):
every cell must find a partner cell of the other curve with
`sup_distance <= eps` — this certifies `sup_t d(X(t), X') <= eps` because for
ANY point of the cell and ANY point of the partner, the distance is `<= eps`.
A cell with no partner subdivides; at the width floor, if its POINT box has
`box_distance > eps` to EVERY cell box of the other curve, that is a
certified `ClosenessViolation` (every curve point of the other side is
strictly farther than eps from the witness); still undecided at the floor
(`ClosenessUnresolved`) is epistemic. Run the machinery BOTH directions;
either side may fire the violation.

**(ii) the angle condition, MANDATORY, on paired cells.** For every pairing
found in (i), over first-derivative boxes `D' = approx.enclose_der(1, cell)`,
`D = exact.enclose_der(1, partner)`: the condition is
`max_angle(a, e) < pi/2 − asin(eps/rho_lower)` for every tangent pair
`(a, e)` with `a ∈ D'`, `e ∈ D` — checked in cosine form, both sides sound:

- pass: `dot_box(D', D).inf() / (norm(D').sup() * norm(D).sup()) > s` where
  `s = eps / rho_lower` proves `min cos > s` (denominators only shrink);
- violation: `dot_box(D', D).sup() / (norm(D').inf() * norm(D).inf()) <= s`
  proves `min cos <= s` (the norm infima are `immersion_lower_bound_box`,
  the numerators' sup is the interval dot's upper endpoint) — a certified
  `AngleViolation`;
- strictly between: subdivide the pair; floor → `AngleUnresolved`.

Note `arccos(c) = pi/2 − asin(c)` for `c ∈ [0,1]` is what makes the cosine
form identical to the spec's angle form; do NOT take any `acos` in code.
`s > 0` and both norms strictly positive are preconditions of the arithmetic
(tangent boxes containing zero already refused in Decision 1 / pairing).

**(iii) endpoint correspondence, at parameter-endpoint granularity.** Let
`E_lo = enclose(degenerate(lo))`, `E_hi = enclose(degenerate(hi))` for each
curve (degenerate point boxes: sup-distance and inf-distance coincide, no
ambiguity). Certify: every endpoint point-box of either curve has
`sup_distance <= eps` to SOME endpoint point-box of the other. Two
correspondences where an endpoint maps to the OTHER curve's opposite
endpoint are fine (orientation is combinatorial, not certified here);
`BoundaryMismatch` names the failed endpoint. Closure of either curve is NOT
claimed as a topological fact — f64 endpoint enclosures never coincide
exactly; the doc comment says so and says the carrier owns its own topology.

**(iv) one sheet, by the landed evidence.** Call
`fibre_degree_one(exact, approx, t_x, eps, budget)` with `t_x` the exact
curve's span midpoint, recentered off the nearest dyadic bisection midpoint
AND machine-checked against float bisection edges (the BG-FID-008-r3/r4
lesson, which cost two round trips: a witness root that coincides with, or
sits within 2 ulps of, a bisection box edge is outside the operator's
strict-interior reach — simulate the descent `mid = 0.5*lo + 0.5*hi` to the
relative floor and require ≥ 2-ulp margins on both sides, widening once if
not; the landed engine retries on a widened box, but do not pick a witness
that needs it). Map `Ok(ExactlyOne)` → continue; `Ok(NotOne { count })` →
`MultiSheet { count }`; `Err(SheetCountUnresolved)` →
`DegreeOneUnresolved`; `Err(InvalidWitness)` → `InvalidWitness`. Do NOT
reimplement any part of the fibre machinery.

Order of evaluation: `InvalidMargin` checks → rho_lower (Decision 1) →
`2*eps < rho_lower` else `ReachLowerBoundTooSmall` → (i) → (iii) → (ii) →
(iv). All four must hold for `Ok`; the report carries `eps` and `rho_lower`.

### Decision 3 — the bridge lemmas, as a structured comment (NOT code)

Copy this block verbatim into the module docs, above the public function,
marking each lemma with its status. It is the certificate site the spec
amendment requires; a future packet discharges or refutes each lemma:

```text
L-TUBE       eps < reach(X) => the closed eps-tube of a compact C²
             surface-with-boundary is a topological thickening whose sides
             are the offset sheets. STATUS: OPEN (closed case = classical
             tubular neighborhood theorem; the with-boundary restriction is
             ours).
L-COVERING   transversality/local-inverse (ii) + properness + certified
             fibre multiplicity one (iv) => the fibre projection is a
             ONE-SHEETED COVERING => homeomorphism. STATUS: OPEN.
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
lfs.rs):

```rust
/// @feeds [CCS05, Thm 2.1:H1-H3]          # would discharge, conditional on lemmas
/// @via-open-lemma FID-L-TUBE | FID-L-COVERING | FID-L-SEPARATES
/// @establishes
///   conditions (i)-(iv) of §6.2 on ONE curve component pair
/// @does-not-establish
///   isotopy | homeomorphism | side separation | surface case | (iv-b)
```

A definition citation is not a theorem instance. Wrong tags are
`disagreements` findings.

### Decision 5 — module layout

`fid/mod.rs` gains exactly one line `pub mod isotopy;` (alphabetical) and its
doc line notes the surface case and (iv-b) wait on BG-FID-005. isotopy.rs
carries `#![deny(clippy::unwrap_used)]` INCLUDING the test module (GATE-1).
`sup_distance`/`box_distance`/`dot_box` are duplicated locally exactly as
one_sheet.rs does — enclosure.rs visibility stays untouched. Test-only curve
structs live IN the test module following one_sheet.rs's local-curve pattern
(implement ParametricCurve + EnclosureCurve with hand-written interval
enclosures built on crate::elementary's outward-rounded cos/sin).

### Decision 6 — tests (all in isotopy.rs's test module)

All floats named consts with same-line `// H-3:` comments. Reuse the landed
witness conventions: exact circle radius `R = 2`, `eps = 0.05`.

1. `single_sheet_circle_conditions_hold` — exact circle over `[0, 2pi]`,
   approx circle radius `R + eps/2` (decidably in-disc per BG-FID-008-r2's
   witness table): `Ok(_)` with `rho_lower` near (slightly below) `R`.
2. `radial_sinusoid_fails_angle_condition` — THE motivating (ii) case:
   approx `(R + a*sin(omega*t))*e(t)` over `[0, 2pi]` with `a = 0.04 <= eps`
   and `omega = 4000`. (i) passes (two-sided distance = a); (ii) must fail:
   the extreme tangent deviation `atan(a*omega/R) = atan(80) ≈ 1.5583` rad
   exceeds `pi/2 − asin(eps/R) ≈ 1.5458` rad. Expected: `AngleViolation`.
   If this returns `Ok` or `ClosenessViolation`, the checker is wrong. NOTE:
   the spec's prose says "tangent angle > pi/2", which is UNREACHABLE between
   unoriented tangents (graph-over-line angles are `< pi/2`); the executable
   witness fails the asin-tightened bound, which is the actual condition.
3. `double_cover_is_multisheet` — the landed BG-FID-008 witness verbatim:
   `(R + eps*cos(t/2))*e(t)` over `[0, 4pi]`. (i), (iii) hold, (ii) holds
   (tangent deviation `O(eps/R)`); expected `MultiSheet { count: 2 }`.
4. `trimmed_approx_boundary_mismatch` — two open segments: exact line
   segment over `[0, 1]`, approx the same over `[0.1, 1]` (endpoint displaced
   by `0.1*|X'| > eps`): expected `BoundaryMismatch`.
5. `coarse_radius_refuses` — exact circle of radius `0.08`, `eps = 0.05`:
   `2*eps >= rho_lower` → `ReachLowerBoundTooSmall` (the epistemic refusal).
6. `invalid_margin_refuses` — `eps = 0`, negative eps, non-finite eps.
7. `zero_budget_refuses_unresolved` — empty budget → an `*Unresolved` arm
   (which one depends on evaluation order; assert it is one of
   `ClosenessUnresolved` / `AngleUnresolved` / `DegreeOneUnresolved`).
8. `line_pair_conditions_hold` — two straight parallel segments offset by
   `eps/2`: `rho_lower = +inf` exercises the extended-real path; `Ok(_)`.

Machine-check every witness number with a script BEFORE writing RESULT.json
(the session-18 lesson): `atan(80)`, `asin(0.025)`, the two-sided distance
`a` of the sinusoid, `rho_lower` brackets — through THIS module's formulas,
not a scratch variant.

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
cargo test -p truck-evidence --lib fid::isotopy --no-fail-fast
cargo check -p truck-evidence
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-evidence is green at baseline. Any baseline failure you did not cause is
a stop condition. Send cargo output to a file and read the tail. Never run a
bare `cargo test`.

## Forbidden

Editing files outside `write_allow`. Implementing the surface case or (iv-b)
here. Claiming isotopy, homeomorphism, or any bridge lemma as proved, or
tagging anything "Thm instance". Sampling-based (point) checks of (i) or (ii)
— the spec's own rule: the bound is over the WHOLE SPAN by interval
evaluation, and sampling passes on precisely the inputs that matter. Taking
`acos`/`asin` in the (ii) comparison (the cosine form is exact and cheaper).
Bare float literals without `// H-3`. `unwrap()`/`expect()` on fallible
production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the landed `fibre_degree_one` signature or semantics do not match this
  packet's Decision 2(iv) description → `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,fid): whole-span isotopy conditions (i)-(iv) for curve components (BG-FID-003)
```
