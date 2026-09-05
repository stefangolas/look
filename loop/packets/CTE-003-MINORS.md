# WORK PACKET CTE-003-MINORS — chart-minor polynomials, the T system, T1.5 exclusion

You are implementing the deflation substrate of the Certified Tangency and
Exact Contact (CTE) program. Everything you need is in this document,
`docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§2.3–§2.4 and §2.7 are
normative). Do not read other spec files. If something you need is genuinely
missing, that is a SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-003-MINORS
contract:    [CTE-003-MINORS]
class:       design
crates:      [truck-certified]
depends_on:  [CTE-000-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/tangency/minors.rs
  - vendor/truck/truck-certified/src/tangency/tsystem.rs
  - vendor/truck/truck-certified/src/tangency/exclude.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
  - vendor/truck/truck-certified/src/ssi.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/shapes.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-certified/src/kernel/minor_algebra.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - second_partials_match_finite_difference_on_fixture
  - chart_minors_match_t11_identity_on_f2
  - tsystem_admits_krawczyk4_instantiation
  - t14_identity_holds_on_f1_admission
  - t15_excludes_transversal_fixture_f6
  - t15_refusal_is_named
budget:      {turns: 110, ctx_tokens: 220000}
```

## Problem

The deflated system `T = (G1, G2, M1, M2)` (theory §2.4) needs the chart
minors as POLYNOMIALS with an interval Jacobian — the landed
`kernel/minor_algebra.rs` gives per-box interval minor enclosures, which are
not a system. You build: additive second-partial Bernstein nets on the SSI
`Tensor4`; the polynomial chart-minor grids `M1, M2`; the `TSystem` as a
`KrawczykSystem<4>` instantiation; and the T1.5(1) five-equation exclusion
driver. This packet is the A₁/A₂ substrate; CTE-004 and CTE-005 consume it.

## Scope decisions — pre-made, do not relitigate

1. **Second partials are additive and pattern-copied.** `ssi.rs` gains
   `Tensor4::partial2_axis(j, l)` — the Bernstein second-derivative
   coefficient rule `d·(d−1)·(c[k+2] − 2c[k+1] + c[k])` per axis pair,
   exactly the pattern of `kernel/engine.rs`'s private `grid_second_partial`
   (engine.rs:1128). **`kernel/engine.rs` is NOT edited** — the pattern is
   adapted, not imported (V5 guard). You are `ssi.rs`'s sole writer this
   program; the additive method is the ONLY change there.
2. **Chart minors are composed polynomial grids.** `M_j = det
   D_{(y1,y2,zj)}(G1,G2,f)` is a 3×3 determinant of partial-derivative
   Tensor4 grids (one first-partial along `zj`, two along the pivot `y`
   axes) — compose the grids polynomially ( Bernstein coefficient sums and
   products per the landed de-Casteljau-compatible layout), then hull per
   box. Admission test: the T1.1 identity (theory §2.3) residual is exactly
   0 on the F2 fixture, evaluated in exact/rationally-representable
   arithmetic in-test.
3. **TSystem instantiates the landed generic Krawczyk.**
   `impl KrawczykSystem<4> for TSystem` with `f_point`, ROW-MAJOR interval
   Jacobian (rows `G1,G2,M1,M2`; the M rows' Jacobian entries need SECOND
   partials of F — that is why this packet owns `partial2_axis`), and a
   float preconditioner. `krawczyk.rs` is NOT edited. The N=4 precedent is
   `construct/bie/ssi4.rs`'s `Ssi4System`.
4. **The R1 rule is absolute**: `det DT = det A³·det H_h` (theory T1.4)
   appears ONLY as the F1 fixture identity test and in the termination
   argument's doc comment. No runtime function evaluates it. Krawczyk uses
   the honest interval `DT(B)`.
5. **T1.5(1) exclusion driver** (`exclude.rs`): decide "no root of
   `(F, M1, M2)` in B" by subdivision: per box, FIRST try single-equation
   interval separation (any of the five equations separated from 0 over the
   box excludes — cheap pruning), then a square-subsystem Krawczyk exclusion
   (`K ∩ Q = ∅`) on any 4-of-5 subsystem, else bisect (widest axis,
   low-before-high) under Budget. Exhausted budget at a non-excluded,
   non-degenerate box is `Inconclusive { cell }` — never a guess.
6. **Refusals are named**, wrapping landed causes. Zero new top-level
   evidence kinds.

## Contract — frozen output (spine §2; CTE-004/005 consume)

```rust
pub struct ChartMinorGrids { pub m1: Tensor4, pub m2: Tensor4 }  // shapes.rs type, built here
pub struct TSystem { /* G1, G2, M1, M2 grids */ }
impl KrawczykSystem<4> for TSystem { /* f_point / jacobian / preconditioner */ }
pub enum ExclusionEvidence { NoRootFiveEq { spend: Budget }, CertifiedCriticalPoint(Box4), Inconclusive { cell: Box4 } }
pub fn build_chart_minors(system: &SquareSystem3, chart: &Rank2Chart) -> Result<ChartMinorGrids, TangencyRefusal>;
pub fn exclude_five(system: &SquareSystem3, minors: &ChartMinorGrids, chart: &Rank2Chart, b: &Box4, budget: &mut Budget)
    -> Result<ExclusionEvidence, TangencyRefusal>;
```

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/ssi.rs` | `struct Tensor4` | 1 |
| A2 | `truck-certified/src/ssi.rs` | `fn partial_axis` | 1 |
| A3 | `truck-certified/src/kernel/engine.rs` | `fn grid_second_partial` | 1 |
| A4 | `truck-certified/src/kernel/engine.rs` | `pub(crate) fn system_hessian` | 1 |
| A5 | `truck-certified/src/kernel/minor_algebra.rs` | `pub fn minor_vector_encl` | 1 |
| A6 | `truck-evidence/src/num/krawczyk.rs` | `pub trait KrawczykSystem` | 1 |
| A7 | `truck-certified/src/tangency/shapes.rs` | `pub struct ChartMinorGrids` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing
  reachable from geometry.
- **H-2** Fallible operations return `Result<T, TangencyRefusal>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`.
- **Determinism**: fixed subsystem attempt order (indices ascending,
  skip-one in ascending index order); bisection widest-axis,
  low-before-high, always.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

1. `second_partials_match_finite_difference_on_fixture` — `partial2_axis`
   agrees with centered finite differences of `partial_axis` on F5's grid
   (H-3 tolerance; the CERTIFICATE is the Bernstein rule, this pins the
   axis-pair bookkeeping).
2. `chart_minors_match_t11_identity_on_f2` — T1.1 residual exactly 0 on the
   F2 fixture for both j.
3. `tsystem_admits_krawczyk4_instantiation` — the trait impl compiles and
   `f_point` at the F1 critical point returns the expected zero vector.
4. `t14_identity_holds_on_f1_admission` — the F1 ground truth (−60) as an
   EXACT test-side identity; asserts this is test-only (the R1 gate).
5. `t15_excludes_transversal_fixture_f6` — the driver returns
   `NoRootFiveEq` on F6 (the minor-separation-sharpening case).
6. `t15_refusal_is_named` — budget exhaustion yields `Inconclusive { cell }`
   with the landed cause, never a string or a guess.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
cargo test -p truck-certified --lib ssi
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `kernel/engine.rs`
(pattern source, read-only), `kernel/minor_algebra.rs`,
`tangency/{shapes,fixtures}.rs`, `truck-evidence/src/num/krawczyk.rs`,
`Cargo.lock`. Evaluating the T1.4 identity at runtime. Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the T1.1 identity cannot be certified exactly on F2 → `SPEC_GAP` (record
  the residual form you computed — a wrong identity reading is a theory
  defect the loop needs to see)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-003-MINORS","status":"DONE","contracts":["CTE-003-MINORS"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":1,"A7":1},
 "notes":"partial2_axis axis-pair convention; minor-grid composition cost; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): chart-minor polynomials + deflated T system + T1.5 exclusion (CTE-003-MINORS)`.
