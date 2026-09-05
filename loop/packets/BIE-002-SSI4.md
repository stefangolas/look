# WORK PACKET BIE-002-SSI4 — the restricted-pair interaction solver

You are implementing the solver at the heart of the Certified Interaction
Engine (BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-002-SSI4
contract:    [BIE-002-SSI4]
class:       design
crates:      [truck-certified, truck-evidence]
depends_on:  [BIE-001-ARITHMETIC]
write_allow:
  - vendor/truck/truck-certified/src/construct/bie/mod.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/num/parallelotope.rs
read_allow:
  - vendor/truck/truck-certified/src/interval/mod.rs
  - vendor/truck/truck-certified/src/interval/box4.rs
  - vendor/truck/truck-certified/src/interval/bounds.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-certified/src/kernel/engine.rs
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-certified/src/construct/bie/fixtures.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - column_choice_finds_transversal_subset
  - minor_sign_predicate_matches_expansion
  - boundary_seed_resolves_exf_and_fxe
  - continuation_tracks_known_curve
  - unresolved_elsewhere_is_typed
budget:      {turns: 100, ctx_tokens: 220000}
```

**New files** (`construct/bie/ssi4.rs`, `num/parallelotope.rs`): H-1 applies —
no `unwrap_used` without a justified same-line opt-out.

## Problem

For a restricted pair (a `SpineFrameSweep` × a canonical surface, or
canonical × canonical falling through the analytic funnel), the interaction
form $F = R_A - R_B$ (theory §1.1) is **directly evaluable** through landed
`subs`/`der` — `impl ParametricSurface for SpineFrameSweep` is landed and
canonical surfaces carry the same traits. The solver certifies the zero set
of F over 4-D parameter boxes: metric normalization σ, transversal column
choice, the minor-sign predicate (R′), boundary seeding, and parallelotope
continuation. Everything it cannot certify is a typed `Unresolved` — never a
guess.

## Scope decisions — pre-made, do not relitigate

1. **F is evaluated directly.** No cross-multiplication, no polynomialization.
   The landed `truck-certified/src/ssi.rs` (the cross-multiplied
   square-system engine) is the GENERAL-pair tail; this module is the
   restricted normal path and shares no code with it. Do not import `ssi.rs`.
2. **Krawczyk is instantiated, not extended.** The landed entry is
   `pub fn krawczyk<const N: usize>(system: &impl KrawczykSystem<N>, start:
   &[Interval; N], budget)` over `pub trait KrawczykSystem<const N: usize>`
   (`truck-evidence/src/num/krawczyk.rs:86/:62`). You implement
   `KrawczykSystem<4>` and `KrawczykSystem<3>` over your F-form system.
   `krawczyk.rs` is NOT edited. The landed precedents to copy style from are
   `krawczyk_c1_n3` / `krawczyk_c1_n4` in `truck-certified/src/kernel/engine.rs`.
3. **The parallelotope tracker is new** — `num/parallelotope.rs`: the
   continuation step (theory §3.3 θρ step) tracks the solution curve's
   tangent frame as a parallelotope over the box, feeding the next seed.
   Additive to `num/mod.rs` (one `pub mod` line).
4. **σ/σ_G**: first-fundamental form from the landed `derivative` family;
   σ_G is needed canonical-side only (sweeps are pole-free — booking §1 row
   §2.3). Canonical-side σ_G reduces to the recognized-carrier list; pairs
   outside it keep the landed analytic path.
5. **Column choice is closed form**: try all four 3-of-4 subsets (~100 LOC);
   no RRQR. The transversal subset is the one whose 3×3 minor sign is
   certified by (R′).
6. **(R′) minor-sign** uses the landed `Expansion` (`formal/exact.rs:63`)
   for the certified 3×3 determinant sign over a box.
7. **Boundary seeding**: N=3 square systems on the E×F and F×E product
   strata (`BoundedStratum::Face/Edge` enumeration; interval exclusion via
   BIE-001 boxes disposes of empties).
8. **Unresolved elsewhere** — `InteractionOutcome::Unresolved { kappa, cell,
   slope }` from the BIE-000 shim, mapping onto the landed
   `Refusal::NumericallyUnresolved` witness. Zero new refusal arms.

## Contract — frozen output type (spine §3; BIE-004 escalates, BIE-005 consumes)

```rust
/// A certified interaction-curve branch in the 4-D product chart.
pub struct CertifiedChartCurve {
    /// Ordered samples along the branch (parameter cells, certified).
    pub samples: Vec<ChartSample>,
    /// Per-sample tangent frames (the parallelotope output).
    pub tangent_frames: Vec<ParallelotopeFrame>,
    /// The unresolved witness slot (κ/cell/slope) for escalation.
    pub witness: Option<InteractionOutcome>,
}
```

Where the landed API differs in detail, use what is actually there and note
it in RESULT notes — the frozen contract is the API SHAPE.

## Anchors — measured 2026-09-05 (A1–A3), A4 pre-shim: re-check after BIE-000 lands

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-evidence/src/num/krawczyk.rs` | `pub trait KrawczykSystem<const N: usize>` | 1 |
| A2 | `vendor/truck/truck-evidence/src/num/krawczyk.rs` | `pub fn krawczyk<const N: usize>` | 1 |
| A3 | `vendor/truck/truck-certified/src/kernel/engine.rs` | `pub fn krawczyk_c1_n4` | 1 |
| A4 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `impl ParametricSurface for SpineFrameSweep` | 1 |
| A5 | `vendor/truck/truck-certified/src/formal/exact.rs` | `pub struct Expansion` | 1 |
| A6 | `vendor/truck/truck-evidence/src/num/mod.rs` | `^pub mod` | 3 |

A6 becomes 4 when you add `pub mod parallelotope;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed values are never recorded as `Method::Exact`; every
  certified answer carries its certificate.
- **Determinism** (spine §8): identical ordered input → identical verdicts;
  no output ordering from hash iteration. Bisection order is by axis, then
  low-before-high, always.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns (in-module test sections) — the verifier checks the
names appear in your diff. Build against the BIE-000 fixture kit
(`construct/bie/fixtures.rs`) for ground truths.

1. `column_choice_finds_transversal_subset` — on a transverse fixture pair,
   the 3-of-4 search finds the subset whose minor sign certifies.
2. `minor_sign_predicate_matches_expansion` — the (R′) predicate agrees with
   the landed `Expansion` exact sign on constructed 3×3 systems.
3. `boundary_seed_resolves_exf_and_fxe` — both boundary strata classes seed
   and certify on the fixture kit.
4. `continuation_tracks_known_curve` — the parallelotope continuation tracks
   a known intersection branch (plane×sphere circle from the fixture kit)
   end to end within `// H-3` tolerance.
5. `unresolved_elsewhere_is_typed` — a degenerate/tangent fixture returns
   `InteractionOutcome::Unresolved` (typed), never a guess and never a panic.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified -p truck-evidence
cargo clippy -p truck-certified -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo test -p truck-evidence --lib num
cargo check -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `num/krawczyk.rs`,
`kernel/engine.rs`, `formal/exact.rs`, `src/ssi.rs`, anything under
`truck-geometry/` or `truck-shapeops/`, `scripts/kernel-gates.sh`,
`Cargo.lock`. Importing the landed `ssi.rs` cross-multiplied engine.
Adding `#[ignore]`. Adding `#[allow]` without a justification comment on the
same line. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- a fixture ground truth cannot be certified without fabricating the
  certificate → stop, record the box and the F values, status `SPEC_GAP`
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-002-SSI4","status":"DONE","contracts":["BIE-002-SSI4"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":3},
 "notes":"which fixture pairs you certified, the unresolved rate you observed, and any API deviations from the frozen contract"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): restricted-pair SSI4 solver — metric normalization, seeding, parallelotope continuation (BIE-002-SSI4)`.
