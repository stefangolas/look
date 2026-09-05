# WORK PACKET CTE-002-GRAPH — chart search + parametric graph enclosure

You are implementing the critical-path numeric packet of the Certified
Tangency and Exact Contact (CTE) program. Everything you need is in this
document, `docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§2.1–§2.2 are normative).
Do not read other spec files. If something you need is genuinely missing,
that is a SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-002-GRAPH
contract:    [CTE-002-GRAPH]
class:       design
crates:      [truck-certified]
depends_on:  [CTE-000-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/tangency/chart.rs
  - vendor/truck/truck-certified/src/tangency/graph.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/shapes.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - chart_search_finds_pivot_on_rank2_fixture
  - chart_search_refuses_on_transversal_fixture
  - parametric_newton_certifies_graph_on_f3
  - graph_enclosure_contains_phi_on_probe_grid
  - no_chart_refusal_is_named
budget:      {turns: 120, ctx_tokens: 220000}
```

## Problem

The theory reduces singular SSI to a scalar function `h(z) = f(φ(z), z)` on a
certified graph (§2.2, hypothesis H-graph). Two certified predicates are
needed first: Lemma T1.0's bounded 18-chart search (a component choice × a
coordinate-pair choice with `0 ∉ det Â(B)`), and the parametric interval
Newton/Krawczyk that certifies, for every `z ∈ Z`, a unique `y = φ(z) ∈ Y`
with `G(φ(z), z) = 0`, plus the graph enclosure `Ŷ ⊇ φ(Z)`, `Ŷ ⊆ Y`. This is
the program's critical path: CTE-004 and CTE-005 gate on it.

## Scope decisions — pre-made, do not relitigate

1. **18 charts = 3 components × C(4,2) coordinate pairs.** Enumerate in a
   FIXED order (component x,y,z; coordinate pairs lexicographic); return the
   FIRST chart whose `det Â(B)` enclosure excludes 0 — determinism gate. The
   search's success predicate is exactly `0 ∉ det Â(B)` (theory §2.1); no
   conditioning score, no "best pivot" heuristic.
2. **Parametric Newton is new code but instantiated discipline.** The landed
   `KrawczykSystem`/`krawczyk` (`truck-evidence/src/num/krawczyk.rs:62/:86`)
   is point-box. Your parametric variant follows the SAME contract shape with
   `z` as the parameter: `f_point(y, z_mid)`, interval Jacobian
   `D_yG(Y, Z)`, preconditioner at the midpoint pair, and STRICT inclusion
   `K(Y) ⊆ int(Y)` proving a unique `y` for EVERY `z ∈ Z` (the parameterized
   rule stated in `krawczyk.rs`'s doc: `F(m, t_mid)`, `J(Q, T)`). Do not edit
   `krawczyk.rs`.
3. **The graph enclosure `Ŷ` is the hull of the Krawczyk images** over `Z`,
   outward-rounded, clamped into `Y`; refuse `Ŷ ⊄ Y` (the H-graph
   hypothesis fails — named refusal, never a retry on the same box).
4. **Chart minors are NOT built here** (CTE-003 owns polynomial `M1, M2`).
   You build `det Â`'s enclosure only, from the landed
   `Tensor4::partial_axis` first partials composed over the pivot's rows and
   columns — a 2×2 interval determinant of certified partial enclosures.
5. **Refusals are named**: `NoChart` (all 18 pivots contain 0 — the box is
   not rank-2-admissible), `GraphFailure` (parametric contraction failed at
   the budgeted depth), wrapping the landed `SsiRefusal` vocabulary. Zero new
   top-level evidence kinds.
6. **Bisection on graph failure** follows the landed operator's discipline
   (widest z-axis, ties toward lowest index, low-before-high), under Budget;
   a degenerate point that still fails refuses — never an infinite split.

## Contract — frozen output (spine §2; CTE-003/004/005 consume)

```rust
/// The certified (H-graph) predicate over one box (theory §2.2).
pub struct CertifiedGraph {
    pub chart: Rank2Chart,          // pivot + det_a enclosure (0 excluded)
    pub graph: GraphEnclosure,      // Y_hat ⊇ φ(Z), Y_hat ⊆ Y
}
pub fn find_chart(system: &SquareSystem3, b: &Box4) -> Result<Rank2Chart, TangencyRefusal>;
pub fn certify_graph(system: &SquareSystem3, chart: &Rank2Chart, b: &Box4)
    -> Result<CertifiedGraph, TangencyRefusal>;
```

Where the landed API differs in detail, use what is actually there and note
it in RESULT notes — the frozen contract is the API SHAPE.

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/ssi.rs` | `struct Tensor4` | 1 |
| A2 | `truck-certified/src/ssi.rs` | `fn partial_axis` | 1 |
| A3 | `truck-certified/src/ssi.rs` | `fn hull_tensor4` | 1 |
| A4 | `truck-certified/src/ssi_types.rs` | `pub struct SquareSystem3` | 1 |
| A5 | `truck-evidence/src/num/krawczyk.rs` | `pub trait KrawczykSystem` | 1 |
| A6 | `truck-certified/src/tangency/shapes.rs` | `pub struct Rank2Chart` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing
  reachable from geometry.
- **H-2** Fallible operations return `Result<T, TangencyRefusal>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`; every
  certified answer carries its certificate.
- **Determinism**: fixed enumeration order everywhere; bisection widest-axis,
  low-before-high, always.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Build against the CTE-000 fixture kit (`tangency/fixtures.rs`):

1. `chart_search_finds_pivot_on_rank2_fixture` — on F3 (A₁⁺) and F4 (A₁⁻)
   the search returns a chart with `0 ∉ det Â(B)`.
2. `chart_search_refuses_on_transversal_fixture` — F6 returns `NoChart`
   (a transversal box admits no rank-2 chart; that is CORRECT, not an error).
3. `parametric_newton_certifies_graph_on_f3` — strict inclusion holds on the
   F3 box at a workable subdivision depth.
4. `graph_enclosure_contains_phi_on_probe_grid` — for a dyadic probe grid of
   `z` values, the float Newton solution `φ(z)` lies inside `Ŷ` for every
   probe (consistency, H-3 tolerance; the CERTIFICATE is the Krawczyk
   inclusion, this test is a regression net).
5. `no_chart_refusal_is_named` — the refusal carries the landed cause, never
   a string.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `ssi.rs` (CTE-003 is its
sole writer), `tangency/shapes.rs`, `tangency/fixtures.rs`,
`truck-evidence/src/num/krawczyk.rs`, `Cargo.lock`. Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the parametric inclusion cannot be certified on ANY fixture at ANY budget
  → stop, record the boxes and widths, status `SPEC_GAP` (this is the
  contract-semantics defect the loop needs to see, not something to bury)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-002-GRAPH","status":"DONE","contracts":["CTE-002-GRAPH"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":1},
 "notes":"which fixtures certified; enclosure widths at admission depth; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): 18-chart search + parametric graph enclosure (CTE-002-GRAPH)`.
