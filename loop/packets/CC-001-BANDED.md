# CC-001-BANDED — P1: certified banded solve (interval no-pivot GE + Rump fallback)

CC program Phase A (spine S3). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §1 P1. Consumers: the loft
collocation solve (CC-010/012/015) and Hermite ribbons (CC-033). The fast
path's stability is CLASS-SPECIFIC: banded totally-positive matrices (all
Schoenberg–Whitney collocation matrices) have growth factor exactly 1 under
no-pivot Gaussian elimination (de Boor–Pinkus), which is why interval
elimination without row exchanges is safe here and would not be in general —
this justification goes in the module doc verbatim in substance.

```yaml
id:          CC-001-BANDED
contract:    [CC-001-BANDED]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/banded.rs
  - vendor/truck/truck-certified/src/construct/residual_solve.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_banded.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-certified/src/formal/exact.rs
budget:      {turns: 24, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub type Interval' vendor/truck/truck-certified/src/kernel/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn sqrt' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn from_product' vendor/truck/truck-certified/src/formal/exact.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'banded_cubic_uniform' vendor/truck/truck-certified/src/construct/fixtures.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'banded_pivot_spans_zero' vendor/truck/truck-certified/src/construct/fixtures.rs"}
tests_required:
  - banded_uniform_cubic_recovers_known_rational_solution
  - pivot_containing_zero_refuses_singular_interpolation_system
  - ill_conditioned_non_tp_fixture_refuses_never_pivots
  - enclosure_width_shrinks_with_input_width
  - rump_residual_certifies_when_eta_below_one
  - rump_refuses_conditioning_below_threshold_when_eta_at_or_above_one
```

Section 1: `construct/banded.rs` — `pub struct BandedFactor` (private band
storage, order `n`, half-bandwidth `q`) and
`pub fn factor_banded_tp(bands: &[Interval]) -> Result<BandedFactor,
ConstructRefusal>` per spine S3: the input is the row-major band storage of
a banded totally-positive collocation matrix (the caller builds bands from
B-spline basis values; the factorization never sees geometry). Interval
Gaussian elimination WITHOUT pivoting: any pivot interval containing 0 →
`Err(ConstructRefusal::SingularInterpolationSystem)` — never swap, never
retry with a different order, never widen. Back-substitution delivers
enclosure rows. Deterministic evaluation order everywhere (row-major, fixed
accumulation order); no parallel reductions.

Section 2: `impl BandedFactor` — `pub fn solve_homogeneous(&self, rhs:
&[[Interval; 4]]) -> Result<Vec<[Interval; 4]>, ConstructRefusal>` (all
homogeneous control rows of a loft in one call, one factorization shared
across strips) and `pub fn max_control_error(&self) -> f64` — the L2
enclosure width ε: max over delivered control entries of enclosure width.
Both are pure functions of the factorization + input.

Section 3: `construct/residual_solve.rs` — the Rump/Ogita/Oishi fallback for
systems OUTSIDE the banded-TP class (theory §1 P1 fallback; consumers:
Hermite ribbons, radius-law splines):
`pub fn residual_solve_dense<const N: usize>(a: &[[Interval; N]; N], r_inv:
&[[f64; N]; N], x_hat: &[f64; N], b: &[Interval; N]) -> Result<[Interval;
N], ConstructRefusal>`. Compute η = ‖I − R·A‖_∞ in interval arithmetic;
η ≥ 1 → `Err(ConstructRefusal::ConditioningBelowThreshold)`. Otherwise
compute the residual enclosure r = b − A·x̂, form the bound ‖x − x̂‖_∞ ≤
‖R·r‖_∞/(1 − η) and return x̂ ± that bound as an enclosure. The proof
identity (RA = I − E) and the bound derivation go in the module doc. The
float preconditioner R is CALLER-supplied (a plain 2×2/3×3 adjugate inverse
is fine); this packet does not build preconditioners.

Section 4: the exact rational path (P1 exact path, theory §1) is PRE-DECIDED
OUT OF v1: `num-rational` stays out of the manifest. If the loft corpus
needs it, a later amendment adds it. There is no rational-path code and no
rational-path test in this packet; the module doc records the decision and
the `CC_N_EXACT` constant remains reserved.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_banded`. No workspace builds. The `pub mod banded;` / `pub mod
residual_solve;` lines in `construct/mod.rs` are the DESIGNED one-line
conflicts. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) read `formal/exact.rs` FIRST and use its
`CertifiedInterval` ops — do not reimplement directed rounding (A2/A3 pin
the ops you need exist); (2) if `Interval` arithmetic cannot express
something the elimination needs (e.g. division by an interval containing 0
in a NON-pivot position), stop and file QUESTION.md rather than widening —
that is a spine seam defect; (3) the two fixtures from CC-000
(`banded_cubic_uniform`, `banded_pivot_spans_zero`) are the required
test inputs — if their ground truths do not hold, that is a CC-000 defect:
file QUESTION.md, do not bend the fixture.
