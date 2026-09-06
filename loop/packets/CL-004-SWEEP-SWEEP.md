# WORK PACKET CL-004-SWEEP-SWEEP — sweep×sweep dispatch through the restricted solver

You are opening the sweep×sweep dispatch arm. Everything you need is in
this document and `docs/CARRIER_LIFT_BUILD_SPEC.md` (row CL-004, §1
substrate facts, §4 gates). Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          CL-004-SWEEP-SWEEP
contract:    [CL-004-SWEEP-SWEEP]
class:       mechanical
crates:      [truck-evidence, truck-certified]
depends_on:  [CL-003-SWEEP-ENCLOSURE, BIE-006-CLASSIFY]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure_sweep.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/num/parallelotope.rs
  - vendor/truck/truck-certified/src/construct/bie/fixtures.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - sweep_sweep_pair_dispatches
  - sigma_g_composed_both_sides
  - unresolved_elsewhere_is_typed
budget:      {turns: 80, ctx_tokens: 160000}
```

## Problem

Zero `SpineFrameSurface` arms exist in the contact dispatch — sweep×sweep
(monocoque×sidepod class) has no path. CL-003 landed the sweep-side
enclosures (`impl EnclosureSurface for SpineFrameSweep` + σ_G bounds);
BIE-002's restricted solver is landed. This packet adds the dispatch arm:
both sides sweep → the restricted solver (BIE-002's machinery) with CL-003
enclosures and σ_G composed on both sides.

## Scope decisions — pre-made, do not relitigate

1. **The solver is instantiated, never extended**: `ssi4.rs` gains the
   sweep-carrier arm ADDITIVELY (the carrier list grows); the landed
   scheduler/continuation algebra is untouched (V5).
2. **`contact/mod.rs` is yours this cycle** (hot-file rule — CL-001/006
   have landed; no other packet writes it now). The dispatch arm mirrors
   CL-001's: recognize both sides as sweeps, compose σ_G bounds from CL-003's
   landed enclosures, hand the pair to the restricted solver.
3. **Typed outcomes only**: pairs the solver cannot certify emit
   `Unresolved` with κ/cell/slope — zero new `Refusal` arms.
4. **V5, absolute**: landed canonical×canonical and sweep×analytic answers
   stay bit-identical; the only new behavior is the sweep×sweep arm.

## Anchors — measured 2026-09-06 morning, re-check on your branch

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-evidence/src/enclosure_sweep.rs` | `impl EnclosureSurface for SpineFrameSweep` | 1 |
| A2 | `truck-evidence/src/contact/mod.rs` | `pub fn contact` | 1 |
| A3 | `truck-evidence/src/contact/mod.rs` | `set_spline_ssi_entry` | 1 |
| A4 | `truck-certified/src/construct/bie/ssi4.rs` | `impl KrawczykSystem<3>` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: fixed dispatch order; no hash ordering in output.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `sweep_sweep_pair_dispatches` — two `SpineFrameSweep` carriers route to
   the restricted solver and produce a certified verdict on a dyadic
   fixture (the BIE-000 fixture kit's sweep pairs).
2. `sigma_g_composed_both_sides` — the CL-003 σ_G bounds compose on both
   sides and the composed box is finite/isotropic in model units.
3. `unresolved_elsewhere_is_typed` — a pair outside the solver's
   admissible set yields the typed witness, never a guess.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib contact
cargo test -p truck-certified --lib bie
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `enclosure_sweep.rs`,
`num/{krawczyk,parallelotope}.rs`, `construct/bie/{closure,mod}.rs`, any
landed test file, `Cargo.lock`. Adding `#[ignore]`. Unjustified `#[allow]`.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- CL-003's enclosures cannot bound a real sweep×sweep fixture pair →
  `SPEC_GAP`, naming the unbounded quantity
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CL-004-SWEEP-SWEEP","status":"DONE","contracts":["CL-004-SWEEP-SWEEP"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"the dispatch arm's recognition path; sigma_G composition results; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(evidence): sweep×sweep dispatch through the restricted solver (CL-004-SWEEP-SWEEP)`.
