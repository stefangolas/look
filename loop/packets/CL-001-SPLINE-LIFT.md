# WORK PACKET CL-001-SPLINE-LIFT — the spline×analytic contact dispatch arm

You are lifting the landed general-pair SSI engine into the contact funnel's
dispatch. Everything you need is in this document,
`docs/CARRIER_LIFT_BUILD_SPEC.md` (rows CL-001/CL-000, §1 substrate facts,
§4 gates), and `docs/TRUCK123D_PY_BRIDGE_SPEC.md` §8 for the corpus motive.
Do not read other spec files. If something you need is genuinely missing,
that is a SPEC_GAP (see "Stop conditions"): you stop and report, you do not
research it.

```yaml
id:          CL-001-SPLINE-LIFT
contract:    [CL-001-SPLINE-LIFT]
class:       mechanical
crates:      [truck-evidence, truck-certified]
depends_on:  [CL-000-SPLINE-ADMIT, BIE-006-CLASSIFY]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-certified/src/ssi_admit.rs
  - vendor/truck/truck-certified/src/lib.rs
read_allow:
  - vendor/truck/truck-certified/src/patch_admit.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - contact_ff_spline_surface_refuses
  - spline_analytic_pair_dispatches_through_ssi
  - unresolved_carries_kappa_cell_slope
budget:      {turns: 90, ctx_tokens: 180000}
```

**New file** (`ssi_admit.rs`): H-1 applies — no `unwrap_used` without a
justified same-line opt-out.

## Problem

The general-pair engine (`truck-certified/src/ssi.rs`) is landed and
stranded: zero funnel callers. Spline carriers are structurally
`Unrecognized` in the contact layer — the pinned test
`contact_ff_spline_surface_refuses` (`contact/mod.rs`) asserts the refusal.
CL-000 landed the admission layer (`patch_admit.rs`: BSplineSurface →
per-knot-span `RationalBipatch` extraction). This packet wires the dispatch
arm: spline×analytic pairs route through `construct_square_system` +
Krawczyk, with the analytic side carried by its landed `EnclosureSurface`.

## Scope decisions — pre-made, do not relitigate

1. **The pinned refusal test is UPDATED, never deleted** (booking §1): its
   expectation changes from "spline refuses" to "spline dispatches through
   the certified path" — the name stays, the assertion moves. This is the
   booked envelope change, owner-noted.
2. **Admission first**: `ssi_admit.rs` is the bridge — take the contact
   layer's recognized carrier pair, extract the spline side's patches via
   CL-000's landed `patch_admit` entry points, build the
   `SsiParticipant::RationalBipatch` sides, and call the landed
   `construct_square_system`. Any admission refusal (bidegree budget,
   ragged, non-finite) propagates as a typed `Unresolved`, never a panic.
3. **Krawczyk is instantiated, not extended**: the landed
   `krawczyk3_certificate` / generic `KrawczykSystem` drive the certified
   path; `ssi.rs`, `ssi_types.rs`, `krawczyk.rs` are NOT edited.
4. **Typed outcomes only**: where the engine cannot certify
   (conditioning, inclusion, determinant), emit `Unresolved` with the
   κ/cell/slope witness slot — zero new `Refusal` arms (booking §4).
5. **V5, absolute**: every landed canonical×canonical answer stays
   bit-identical; the ONLY behavior change is the spline×analytic arm the
   pinned test books.
6. `contact/mod.rs` is the program's hot file — you are its sole writer
   this cycle (BIE-006 and CL-004/005/006 have landed or are gated on you).

## Anchors — measured 2026-09-05 evening, re-check on your branch

Locate by pattern, never by line number. If a count differs, STOP and
report `ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-evidence/src/contact/mod.rs` | `contact_ff_spline_surface_refuses` | 1 |
| A2 | `truck-certified/src/ssi.rs` | `pub struct RationalBipatch` | 1 |
| A3 | `truck-certified/src/patch_admit.rs` | `pub fn` | 5 |
| A4 | `truck-certified/src/ssi.rs` | `pub fn construct_square_system` | 1 |
| A5 | `truck-base/src/evidence.rs` | `ContactReductionDeferred` | 1 |

## House rules

- **H-1** no unwrap/expect/panic reachable from geometry; **H-3** same-line
  `// H-3`; **H-6** never record `Float` as `Exact`.
- **Determinism**: fixed dispatch order; no hash ordering in output.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `contact_ff_spline_surface_refuses` — UPDATED expectation: the spline
   pair now dispatches through the certified path (name kept, assertion
   moved; the old refusal is asserted nowhere).
2. `spline_analytic_pair_dispatches_through_ssi` — a spline×analytic pair
   (CL-000's admitted extraction on the spline side, landed enclosure on
   the analytic side) produces a `SquareSystem3` and a Krawczyk verdict on
   a dyadic fixture.
3. `unresolved_carries_kappa_cell_slope` — a conditioning-refused pair
   yields the typed `Unresolved` witness, never a bare refusal and never a
   guess.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-certified
cargo clippy -p truck-evidence -p truck-certified --all-targets -- -D warnings
cargo test -p truck-evidence --lib contact
cargo test -p truck-certified --lib
cargo check -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Anything outside `write_allow` — especially `ssi.rs`, `ssi_types.rs`,
`patch_admit.rs`, `num/krawczyk.rs`, `construct/bie/**`, any landed test
file, `Cargo.lock`. Adding `#[ignore]`. Unjustified `#[allow]`. Committing
to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- CL-000's extraction cannot admit any real spline carrier in the funnel's
  fixtures → `SPEC_GAP`, naming the admission refusal
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"CL-001-SPLINE-LIFT","status":"DONE","contracts":["CL-001-SPLINE-LIFT"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":5,"A4":1,"A5":1},
 "notes":"the dispatch arm's admission path; the pinned test's new assertion; any API deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit subject: `feat(evidence): spline×analytic contact dispatch through the landed SSI engine (CL-001-SPLINE-LIFT)`.
