# BG-CK-P1-HULL — the D2 enclosure primitive as public API: control-point hull over subboxes

Certified-kernel Phase 1, first packet. Plan D2: "one enclosure primitive,
composed where the quantity is not polynomial — Bézier form + control-point
hull + directed rounding at the leaves." The substrate pieces all exist
(`formal/exact.rs` `CertifiedInterval` with outward rounding;
`formal/bezier.rs` `RationalBezierSpan2` fully landed, fields `pub(crate)`;
`formal/bezier_isect.rs` carries PRIVATE twin kernels documented
solver-private). What is missing is the public, typed primitive
`docs/CERTIFIED_PHASE1_BOOKING.md` books as BG-CK-P1-HULL: hull bounds of a
Bézier span — curve and surface (tensor) form — over any compact rectangular
subbox, with derivative patches to order 2. MAP (class 1) composes this
module; nothing else changes.

This module is NOT a general interval evaluator (the parsimony hinge; the
frozen F2 table in `src/contract.rs` names the compositions, this module is
what they compose). Hulls are enclosures for POLYNOMIAL quantities only:
this module never divides. The rational curve's dehomogenized value is
composed by the consumer from the homogeneous `X`/`Y`/`W` hulls (F2 rows
RationalNumerator/RationalDenominator/RationalQuotient) — hull returns
homogeneous enclosures for the curve form and leaves division to callers.

```yaml
id:          BG-CK-P1-HULL
contract:    [BG-CK-P1-HULL]
class:       design
crates:      [truck-certified]
depends_on:  [BG-CK-P0-FREEZE]
write_allow:
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/tests/hull_conformance.rs
read_allow:
  - CERTIFIED-KERNEL-PLAN.md
  - docs/CERTIFIED_PHASE1_BOOKING.md
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/formal/bezier.rs
  - vendor/truck/truck-certified/src/formal/bezier_isect.rs
  - vendor/truck/truck-certified/src/formal/span.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'pub mod hull;' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct RationalBezierSpan2' vendor/truck/truck-certified/src/formal/bezier.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub(crate) fn evaluate_enclosure' vendor/truck/truck-certified/src/formal/bezier.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn one_d_range' vendor/truck/truck-certified/src/formal/bezier_isect.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub enum Refusal' vendor/truck/truck-certified/src/contract.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'deny(clippy::unwrap_used)' vendor/truck/truck-certified/src/lib.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn is_finite' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A8, expect: 0, cmd: "grep -rnw 'EnclosureUnavailable' vendor/truck/truck-certified/src | wc -l"}
tests_required:
  - hull_contains_point_evaluation_enclosures_on_degenerate_subinterval
  - hull_contains_brute_force_samples_of_the_polynomial
  - linear_span_hull_is_the_exact_range
  - non_compact_subinterval_refuses_domain_not_compact
  - non_finite_hull_refuses_enclosure_unavailable
  - derivative_coefficients_match_analytic_quadratic
  - hull_over_subinterval_is_contained_in_hull_over_whole
  - curve_homogeneous_jet_order_two_bounds_finite_difference_slopes
  - tensor_patch_hull_contains_corner_and_midpoint_samples
  - hull_never_panics_and_never_divides
```

## Pre-made decisions (do not relitigate; quote the tags into the module doc)

**H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
`hull.rs`. The new module carries NO `unwrap`/`expect`, NO `panic!`, and NO
module-level `allow` — it is authored certified code, not moved baseline.
(No `unwrap_used` grandfathering applies here; that doctrine is for the 19
moved modules only.)

**D2-primitive.** One enclosure primitive, composed. No external interval
crate, no second root engine. All arithmetic goes through
`formal/exact.rs`'s `CertifiedInterval` (outward-rounded, untouched). The
module adds zero interval algebra of its own.

**Polynomial-only.** Hull bounds are enclosures for polynomial quantities
(plan D2 scope statement). The module performs NO division — not even the
rational curve's own dehomogenization. The curve form returns homogeneous
`X`/`Y`/`W` enclosures; division into a dehomogenized bound is the
consumer's named F2 composition.

**Solver-private twins stay private.** `formal/bezier_isect.rs` carries
private `one_d_range` / `bivariate_range` / `tensor_derivative_axis` —
documented solver-private. Do NOT widen their visibility and consume them:
the public primitive must not depend on solver internals (a solver rewrite
must never ripple into public API substrate). `hull.rs` implements its own
kernels with the same de Casteljau-over-`CertifiedInterval` discipline;
note the twins by name in the module doc as prior art, not dependencies.

**Refusal vocabulary is hull-local.** `contract::Refusal` is FROZEN
(Unfrozen/InvalidInput/ConditioningBelowThreshold) — do not add variants to
it, and do not touch the base `truck_base::evidence::Refusal` (mapping
section C row 1). Define `HullRefusal` in `hull.rs` with exactly two named
cases (outcome.rs shape: named cases only, no catch-all, a `tag()` method):

```rust
/// Why a hull enclosure could not be certified.
pub enum HullRefusal {
    /// The directed-rounded hull is not finite (`CertifiedInterval::is_finite()`
    /// is false): the quantity overflows the enclosure at this policy. Never
    /// retried with a wider representation at this level.
    EnclosureUnavailable,
    /// The requested subbox is not a compact subset of the domain: non-finite
    /// bounds, misordered bounds, or bounds outside the span's (canonical)
    /// source domain. Compactness is INCLUSIVE: the closed subinterval and the
    /// full domain boundary are admissible.
    DomainNotCompact,
}
```

## Section 1 — `truck-certified/src/hull.rs` (NEW)

Header: match the crate's lint style (no new lint attributes needed — lib.rs
governs). Module doc: the four pre-made decisions above, each tagged.

### The 1-D kernel

```rust
/// Certified range enclosure of the Bernstein polynomial with coefficients
/// `coeffs` (rising Bernstein basis, degree `coeffs.len() - 1`) over the unit
/// subinterval `sub = (lo, hi)` with `0 <= lo <= hi <= 1`.
///
/// Discipline (the `bezier_isect::one_d_range` twin, re-derived public):
/// de Casteljau evaluation with the subinterval as an OUTWARD-ROUNDED
/// `CertifiedInterval` parameter and every coefficient widened through
/// `CertifiedInterval::point`. Interval arithmetic at each node encloses the
/// exact expression's range over the input box (the dependency problem only
/// widens), so the result provably contains the polynomial's range. An empty
/// coefficient list or any non-finite coefficient refuses `DomainNotCompact`.
pub fn hull_bernstein_1d(coeffs: &[f64], sub: (f64, f64))
    -> Result<CertifiedInterval, HullRefusal>;
```

### The tensor (surface-form) kernel

```rust
/// Certified range enclosure of the bivariate tensor-Bernstein polynomial
/// `c[i][j] * B^i_m(s) B^j_n(t)` over the unit rectangle `s x t` (each axis a
/// compact subinterval of [0, 1]). `grid[i][j]` is the coefficient of
/// `B^i_m(s) B^j_n(t)`; a ragged or empty grid refuses `DomainNotCompact`.
///
/// Discipline: per-column 1-D hull in `s`, then one 1-D hull in `t` (the
/// `bezier_isect::bivariate_range` discipline, re-derived public).
pub fn hull_bernstein_2d(grid: &[Vec<f64>], s: (f64, f64), t: (f64, f64))
    -> Result<CertifiedInterval, HullRefusal>;
```

### Derivative patches (pure f64 coefficient transforms)

```rust
/// Bernstein coefficients of the first derivative: degree `d - 1`, coefficient
/// `i` is `d * (coeffs[i + 1] - coeffs[i])` computed in `f64`. The derivative
/// POLYNOMIAL IS DEFINED by these computed coefficients — the enclosure claim
/// of any hull over them certifies that polynomial (the same definition
/// `bezier_isect::tensor_derivative_axis` uses). A degree-0 input yields the
/// zero polynomial.
pub fn bernstein_derivative_1d(coeffs: &[f64]) -> Vec<f64>;

/// Axis-wise first-derivative coefficients of a tensor grid (`axis == 0` in
/// `s`, `axis == 1` in `t`), same definition and degree bookkeeping as
/// `bernstein_derivative_1d` along the chosen axis.
pub fn bernstein_derivative_2d(grid: &[Vec<f64>], axis: usize) -> Vec<Vec<f64>>;
```

Order 2 is order 1 applied twice (the packet does not add a separate
order-2 function). The `d * (c[i+1] - c[i])` products are `f64` input
transformations, NOT certified quantities — say so in the doc so no reader
mistakes them for directed-rounded work.

### The curve form over `RationalBezierSpan2`

`hull.rs` is in the same crate, so it reads `span.control` / `span.domain`
directly (`pub(crate)` fields — this access is deliberate; widening the
fields' visibility is NOT wanted).

```rust
/// How many derivatives the jet carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JetOrder {
    /// The value itself.
    Value,
    /// First derivative patch.
    First,
    /// Second derivative patch (first applied twice).
    Second,
}

/// Certified homogeneous enclosures `(X, Y, W)` of a rational Bézier span
/// (or of its order-`n` homogeneous derivative patch) over the subinterval
/// `sub = (lo, hi)` in SOURCE parameters.
///
/// `sub` must be a compact subset of the span's canonical source domain
/// (`min(d0, d1) <= lo <= hi <= max(d0, d1)`, inclusive) — anything else
/// refuses `DomainNotCompact`. The source-to-unit map of the subinterval
/// endpoints is computed in `CertifiedInterval` arithmetic (widened — the
/// affine map rounds in `f64`), so the kernel consumes an enclosure of the
/// exact image, never a naked rounded point.
pub fn hull_curve_homogeneous(
    span: &RationalBezierSpan2,
    sub: (f64, f64),
    order: JetOrder,
) -> Result<[CertifiedInterval; 3], HullRefusal>;
```

Implementation: lift `span.control` to per-coordinate `f64` coefficient
vectors, apply `bernstein_derivative_1d` `order` times, map `sub` to the
unit interval as above, and run `hull_bernstein_1d` per coordinate. A
reversed domain (`d1 < d0`, admitted by `RationalBezierSpan2::new`) works
unchanged: the canonical domain is the sorted pair and the coefficient
vector runs with it (the `bezier_isect::canonicalize` reading — the
reversed span's coefficients are its own; do NOT reverse them here).

Overflow path: `CertifiedInterval::mul` collapses to `(-inf, +inf)` on
non-finite intermediates and every kernel finishes with
`is_finite()` — false refuses `EnclosureUnavailable`. Nothing panics.

## Section 2 — lib.rs: one line

`pub mod hull;` added to the five existing module declarations. Nothing else
changes.

## Section 3 — tests (`truck-certified/tests/hull_conformance.rs`, NEW)

Names are contract (`tests_required`). A `RationalBezierSpan2` fixture is
built via `RationalBezierSpan2::new` — `pub(crate)`, so the integration test
CANNOT call it; build spans through `CurveSpan2`'s analytic variants for
non-Bézier cases and construct the Bézier case through the crate's own test
surface instead: put the span-construction tests that need `::new` in a
`#[cfg(test)] mod tests` INSIDE `hull.rs` (same-crate access, the crate's
existing pattern) and keep `tests/hull_conformance.rs` to the `pub` kernels
(`hull_bernstein_1d`, `hull_bernstein_2d`, the derivative transforms,
`HullRefusal`) plus `hull_curve_homogeneous` exercised via any span the crate
exposes publicly. If no public path to a `RationalBezierSpan2` exists, the
curve-form conformance lives in the in-module tests — say which split you
used in `RESULT.json` notes. The load-bearing assertions:

1. `hull_contains_point_evaluation_enclosures_on_degenerate_subinterval` —
   for a degree-3 1-D polynomial, the degenerate subinterval `(u, u)` hull
   contains the de Casteljau point evaluation at `u` (a slightly-widened
   match is expected and fine; containment is the claim).
2. `hull_contains_brute_force_samples_of_the_polynomial` — 1000 uniform
   `f64` samples of a degree-4 polynomial all lie inside the hull over the
   sample's subbox (1-D and 2-D).
3. `linear_span_hull_is_the_exact_range` — a linear Bernstein polynomial's
   hull over any subinterval equals the exact endpoint range up to a few
   ulps (assert containment both ways with an ulp-scale slack; H-3 opt-out
   `// H-3` ON THE SAME LINE as each float epsilon).
4. `non_compact_subinterval_refuses_domain_not_compact` — misordered,
   non-finite, outside-[0,1], and (for the curve form) outside the source
   domain each refuse the named case; the closed domain boundary itself is
   ACCEPTED (inclusive compactness).
5. `non_finite_hull_refuses_enclosure_unavailable` — coefficients near
   `f64::MAX` whose de Casteljau intermediates overflow refuse the named
   case (e.g. `[f64::MAX, f64::MAX]` over a subinterval strictly inside
   (0,1) sums past overflow in interval arithmetic).
6. `derivative_coefficients_match_analytic_quadratic` — the derivative
   coefficients of the quadratic Bernstein form of `p(u) = a u^2 + b u + c`
   match the Bernstein coefficients of `2 a u + b` (exact `f64` equality is
   achievable; if a case needs rounding, H-3 opt-out same-line).
7. `hull_over_subinterval_is_contained_in_hull_over_whole` — monotonicity
   of the enclosure under subbox inclusion (containment, not width).
8. `curve_homogeneous_jet_order_two_bounds_finite_difference_slopes` — for
   an in-module span fixture, the `First` jet hull over a subinterval
   contains the divided differences `(C(u2) - C(u1)) / (u2 - u1)` computed
   in `f64` for samples inside the interval (mean-value theorem, f64-sloped;
   containment with a small slack), and `Second` refuses nothing but
   contains the `First` jet's finite-difference slopes' own bracket.
9. `tensor_patch_hull_contains_corner_and_midpoint_samples` — a bilinear
   tensor patch's hull over a rectangle contains the four corner values and
   the center value exactly-ish (containment).
10. `hull_never_panics_and_never_divides` — every public entry returns
    `Result` and the module text contains no `/` on `CertifiedInterval`
    values and no `unwrap`/`expect` (assert by a source-scan comment and by
    construction; the crate-level deny already enforces the unwrap half).

House rules: H-3 float-comparison opt-outs go ON THE SAME LINE as the
comparison. Clippy zero findings on the new files (`cargo clippy -p
truck-certified --all-targets --message-format=short --no-deps`). No new
dependency edges: `truck-certified`'s manifest is untouched.

## Done-when

- `cargo fmt --all -- --check` clean.
- `cargo clippy -p truck-certified --all-targets --message-format=short
  --no-deps` — zero findings.
- `cargo test -p truck-certified --lib --tests --no-fail-fast` green —
  all landed suites unchanged PLUS the new hull tests.
- `cargo check --workspace --all-targets` green.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. The substrate moved under you relative to the anchors — e.g.
   `RationalBezierSpan2`'s fields changed visibility, `bezier_isect`'s
   kernels became public, or `exact.rs`'s `CertifiedInterval` grew a
   competing algebra. Stop, do not adapt silently.
2. A hull over a subbox cannot be certified with the de
   Casteljau-over-`CertifiedInterval` discipline alone — e.g. you find
   yourself reaching for sampling, Lipschitz bounds, or a second engine.
   The honest answer is that the box needs subdivision (MAP's job, not
   HULL's); say so instead of inventing a mechanism.
3. The curve form needs the dehomogenized value to be useful in a test and
   you are tempted to divide inside `hull.rs`. Division is a consumer-side
   F2 composition; a test that cannot live without it moves to the
   in-module fixture using `CertifiedInterval::div` EXPLICITLY and the
   module doc says the test composes what the primitive deliberately does
   not.

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(certified): Phase-1
D2 hull primitive as public API (BG-CK-P1-HULL)`) BEFORE writing
`RESULT.json`.
