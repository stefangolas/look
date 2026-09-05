# WORK PACKET CTE-004-HESSIAN — reduced Hessian, A₁ arms, five-way cascade

You are implementing the classifier of the Certified Tangency and Exact
Contact (CTE) program. Everything you need is in this document,
`docs/CTE_BUILD_SPINE.md`, and the root theory
`docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md` (§2.5, §2.8, §2.9, §2.11 are
normative). Do not read other spec files. If something you need is genuinely
missing, that is a SPEC_GAP: you stop and report, you do not research it.

```yaml
id:          CTE-004-HESSIAN
contract:    [CTE-004-HESSIAN]
class:       design
crates:      [truck-certified]
depends_on:  [CTE-002-GRAPH, CTE-003-MINORS]
write_allow:
  - vendor/truck/truck-certified/src/tangency/hessian.rs
  - vendor/truck/truck-certified/src/tangency/cascade.rs
  - vendor/truck/truck-certified/src/tangency/mod.rs
read_allow:
  - vendor/truck/truck-certified/src/tangency/{shapes,fixtures}.rs
  - vendor/truck/truck-certified/src/tangency/{graph,tsystem,exclude,minors}.rs
  - vendor/truck/truck-certified/src/tangency/witness.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-certified/src/kernel/contact.rs
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/CTE_BUILD_SPINE.md
tests_required:
  - hessian_evaluator_rejects_raw_box
  - t13_symmetry_on_fixture
  - f3_isolated_contact_verdict
  - f4_node_verdict_is_not_unresolved
  - f6_transversal_by_exclusion
  - cascade_verdicts_are_deterministic
budget:      {turns: 110, ctx_tokens: 220000}
```

## Problem

At a certified critical point of `T` (Krawczyk unique root from CTE-003's
`TSystem`), the theory's trichotomy (§2.9) decides the box: T1.6a
(transversal / Empty / Loop from critical-value sign + definiteness), T1.6b
`A1Isolated` (definite `H_h`, exact-zero witness), T1.6c `A1Node`
(indefinite, `det Ĥ_h < 0`). You build the T1.3 reduced-Hessian evaluator
over the graph enclosure, the §2.8 definiteness test with explicit μ, and
the five-way cascade that assembles the theory's stage table (§2.11).

## Scope decisions — pre-made, do not relitigate

1. **R2 is the signature**: `reduced_hessian(chart: &Rank2Chart, over:
   GraphEnclosure, ...)`. There is NO raw-box path — evaluating over `B`
   would not bound `H_h` at all (theory §2.5 correction).
2. **T1.3 needs no second derivative of φ**: build `P = −A⁻¹G_z`, `J =
   [P; I₂]`, `λ = A⁻ᵀf_yᵀ`, then `H_h = Jᵀ(H_f − λ₁H_G1 − λ₂H_G2)J` from
   CTE-003's interval second partials — all over `Ŷ×Z`. Symmetry of `H_h`
   (exact off-critical-points, theory T1.3) is the admission test.
3. **The §2.8 test carries μ** (theory R9): PD via `a_min > 0` and
   `a_min·c_min > β²` with μ = Gershgorin `min(a_min, c_min) − β`; use the
   closed form when Gershgorin fails but PD still certifies. The
   `Definiteness` constructor refuses `Definite` with `μ ≤ 0` (CTE-000
   shape). Indefiniteness via `det Ĥ_h < 0` strictly.
4. **A₁ requires the exact-zero witness** (theory §4.1, D7): interval
   methods alone CANNOT certify isolation. `A1Cert` without a
   `VerifiedWitness` is unconstructible (CTE-000 shape enforces).
5. **A₁⁻ is a first-class verdict** (theory R3): the F4 battery asserts the
   node verdict. A cascade arm that would emit `Unresolved` for the
   indefinite case is a BUG, not a scope note.
6. **T1.4 never runs at runtime** (R1): Krawczyk contraction on `T` uses the
   honest `DT(B)`; T1.4 backs the termination argument only.
7. **Cascade order is the theory's stage table** (§2.11), stages 0–7, each
   verdict carrying its certificate; `Unresolved { kappa, cell }` records
   the deepest stage reached. Deterministic by construction (fixed stage
   order, no heuristics).
8. `kernel/contact.rs` is READ-ONLY — its Morse/`TopoNode` logic is a
   different setting (canonical carriers, shared chart); CTE-008 runs the
   congruence battery where both apply.

## Contract — frozen output (spine §2; CTE-007/008 consume)

```rust
pub fn reduced_hessian(system: &SquareSystem3, chart: &Rank2Chart,
                       graph: &GraphEnclosure) -> Result<IntervalSym2, TangencyRefusal>;
pub fn definiteness(h: &IntervalSym2) -> Result<Definiteness, TangencyRefusal>;
pub fn classify_box(system: &SquareSystem3, graph: &CertifiedGraph,
                    minors: &ChartMinorGrids, tsys: &TSystem, b: &Box4,
                    budget: &mut Budget) -> Result<ContactVerdict, TangencyRefusal>;
```

## Anchors — measured this session; re-check on your branch before building

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/tangency/shapes.rs` | `pub struct GraphEnclosure` | 1 |
| A2 | `truck-certified/src/tangency/shapes.rs` | `pub enum ContactVerdict` | 1 |
| A3 | `truck-certified/src/tangency/shapes.rs` | `pub enum Definiteness` | 1 |
| A4 | `truck-certified/src/tangency/tsystem.rs` | `impl KrawczykSystem<4> for TSystem` | 1 |
| A5 | `truck-certified/src/tangency/graph.rs` | `pub struct CertifiedGraph` | 1 |
| A6 | `truck-certified/src/kernel/contact.rs` | `fn det2` | 1 |
| A7 | `truck-certified/src/tangency/witness.rs` | `pub fn verify` | 1 |

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, or out-of-range indexing
  reachable from geometry.
- **H-2** Fallible operations return `Result<T, TangencyRefusal>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`.
- **Determinism**: fixed stage order; fixed Gershgorin-then-closed-form μ
  rule; no hash ordering in output.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

1. `hessian_evaluator_rejects_raw_box` — the R2 signature has no raw-box
   path (compile-time shape assertion + a refuses-on-degenerate-enclosure
   test).
2. `t13_symmetry_on_fixture` — `H_h` symmetric within enclosure on F3/F4
   graphs.
3. `f3_isolated_contact_verdict` — F3 → `A1Isolated` with the exact-zero
   witness verified and `μ > 0`.
4. `f4_node_verdict_is_not_unresolved` — F4 → `A1Node` (the R3 gate).
5. `f6_transversal_by_exclusion` — F6 → `Transversal` via T1.5(1) where
   minor separation cannot certify.
6. `cascade_verdicts_are_deterministic` — same input twice → identical
   verdict (field-for-field).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib tangency
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`tangency/{shapes,graph,minors,tsystem,exclude}.rs` (upstream contracts),
`kernel/contact.rs`, `Cargo.lock`. A runtime T1.4 evaluation (R1). An
`A1Isolated` emission without a verified witness. Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- F4 cannot reach `A1Node` at any subdivision depth → `SPEC_GAP` with the
  enclosure widths (this would falsify the theory's genericity claim — the
  loop must see it)
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.**

```json
{"id":"CTE-004-HESSIAN","status":"DONE","contracts":["CTE-004-HESSIAN"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":1,"A7":1},
 "notes":"mu rule used per fixture; cascade stage reached on each fixture; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): reduced Hessian + A1 arms + five-way cascade (CTE-004-HESSIAN)`.
