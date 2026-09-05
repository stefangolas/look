# WORK PACKET BIE-007-GATES — χ valuation + mod-2 homology validity gate

You are implementing the final gate layer of the Certified Interaction
Engine (BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-007-GATES
contract:    [BIE-007-GATES]
class:       mechanical
crates:      [truck-shapeops]
depends_on:  [BIE-006-CLASSIFY]
write_allow:
  - vendor/truck/truck-shapeops/src/lib.rs
  - vendor/truck/truck-shapeops/src/gates/mod.rs
  - vendor/truck/truck-shapeops/src/gates/homology.rs
  - vendor/truck/truck-shapeops/tests/bie_gates.rs
read_allow:
  - vendor/truck/truck-topology/src/manifold.rs
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-shapeops/tests/boolean_m2.rs
  - vendor/truck/truck-shapeops/src/boolean/assemble.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - chi_valuation_matches_known_complexes
  - mod2_homology_detects_defect
  - gate_fails_not_warns_on_mismatch
  - differential_congruent_with_boolean_m2
budget:      {turns: 60, ctx_tokens: 150000}
```

**New files** (`src/gates/mod.rs`, `src/gates/homology.rs`,
`tests/bie_gates.rs`): H-1 applies — no `unwrap_used` without a justified
same-line opt-out. `tests/bie_gates.rs` is a NEW test file (Test-Path
confirmed absent at base): do not reuse any landed test file's path.

## Problem

Theory §9: the output complex needs a validity gate beyond the landed
manifold diagnostics — an Euler-characteristic valuation and a mod-2
(Z₂) homology check over the finite output complex. This packet is the
program's mechanical tail: cheap, finite, and graded by a differential
battery against the landed canonical booleans.

## Scope decisions — pre-made, do not relitigate

1. **The gate entry is the frozen contract** (spine §3):
   `pub fn chi_homology_gate(complex) -> Outcome<GateReport>` over the
   output complex — `GateReport` carries χ, the Z₂ Betti numbers, and the
   verdict. A homology mismatch is **FAILED, never a warning** (booking
   §5): the typed outcome refuses, it does not annotate.
2. **χ valuation + mod-2 homology are cheap finite-complex linear algebra**
   over Z₂ (bitmask rows; Gaussian elimination mod 2). No homology library
   exists in-tree and none is added — the implementation is ~100 lines of
   dense Z₂ linear algebra.
3. **The differential battery reuses the landed `boolean_m2` recipe
   fixtures READ-ONLY** (`tests/boolean_m2.rs` is NOT edited — V5 identity
   guard). `bie_gates.rs` builds the same fixture shapes and asserts the
   gate's χ/homology answers are congruent with the landed
   `boolean_m2` results on the canonical pairs (7/256-face agreement
   preserved bit-for-bit per the booking's §5).
4. **The landed manifold diagnostics** (`pub fn diagnose`,
   `truck-topology/src/manifold.rs:110`) stay as the first gate stage; this
   layer runs BESIDES them (the pipeline: diagnose → χ/homology → verdict).
5. **Mutation battery**: the test module plants defects (an extra face, a
   dropped face, a flipped orientation parity) and the gate must FAIL on
   each — a gate that has only ever passed is indistinguishable from a
   gate that cannot fail (ORCHESTRATOR rule, applied to your own work).

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-topology/src/manifold.rs` | `pub fn diagnose` | 1 |
| A2 | `vendor/truck/truck-topology/src/entity_id.rs` | `pub enum EntityId` | 1 |
| A3 | `vendor/truck/truck-shapeops/src/lib.rs` | `^pub mod` | 5 |
| A4 | `vendor/truck/truck-shapeops/src/boolean/assemble.rs` | `pub fn boolean\(` | 1 |

A3 becomes 6 when you add `pub mod gates;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **Determinism** (spine §8): identical ordered input → identical verdicts;
  complex iteration order is topological (by face/edge index), never
  hash-ordered.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns in `tests/bie_gates.rs` — the verifier checks the names
appear in your diff.

1. `chi_valuation_matches_known_complexes` — χ = V − E + F on ≥3 hand-built
   complexes with known χ (cube shell 2, torus 0, sphere 2).
2. `mod2_homology_detects_defect` — the Z₂ rank computation distinguishes a
   closed shell from the planted-defect variants.
3. `gate_fails_not_warns_on_mismatch` — a mismatching complex returns a
   typed refusal outcome, not an annotated pass.
4. `differential_congruent_with_boolean_m2` — the boolean_m2 recipe
   fixtures through the gate agree with the landed results, bit-for-bit on
   the canonical pairs.

No existing test may be deleted, `#[ignore]`d, or weakened.
`tests/boolean_m2.rs` must be byte-identical after your run.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets -- -D warnings
cargo test -p truck-shapeops --lib --tests
cargo check -p truck-certified -p truck-evidence
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`tests/boolean_m2.rs` or ANY landed test file, `boolean/*` (BIE-006's
files), anything under `truck-geometry/`, `truck-certified/`,
`truck-evidence/`, `truck-topology/`, `scripts/kernel-gates.sh`,
`Cargo.lock`. Adding a homology dependency. Adding `#[ignore]`. Adding
`#[allow]` without a justification comment on the same line. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the gate cannot consume the landed output complex type without editing
  `boolean/*` → `SPEC_GAP`, naming the type mismatch
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-007-GATES","status":"DONE","contracts":["BIE-007-GATES"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":5,"A4":1},
 "notes":"the mutation-battery results (which planted defects the gate caught), and the differential congruence count"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(shapeops): chi valuation + mod-2 homology validity gate (BIE-007-GATES)`.
