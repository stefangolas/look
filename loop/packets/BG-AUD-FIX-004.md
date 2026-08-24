# WORK PACKET BG-AUD-FIX-004 — parametric IntersectionCurve Krawczyk (AUD-004)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-004), in
`truck-evidence/src/decorators/intersection_curve.rs`. Everything you need is
in this document. **Do not read any other spec file** — this packet is
self-contained.

```json
{"id":"BG-AUD-FIX-004","status":"DONE","contracts":["AUD-004"],
 "tests_added":0,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-004
contract:    [AUD-004]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/decorators/intersection_curve.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
tests_required: []
budget:      {turns: 45, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 2, cmd: "grep -c 'fn certify_cell' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn f_iv' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn j_iv' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn enclose_span' vendor/truck/truck-evidence/src/decorators/intersection_curve.rs"}
```

## Problem

`certify_cell` (intersection_curve.rs:382-474) certifies existence AND
uniqueness of the double-projection system's solution inside the returned box
for EVERY `t` in the parameter cell. The parametric Krawczyk operator evaluates
its center term at the single point `t_mid`:

```rust
let f = sys.f_iv(
    interval_at(m0), interval_at(m1), interval_at(m2), interval_at(m3),
    interval_at(t_mid),   // <-- point evaluation of t
);
```

The Krawczyk theorem requires the center term to enclose `F` over the whole
parameter cell: `F(cell, m)`. `F(t_mid, m)` is a single point and
`K_point ⊆ K_cell`, so `K_point ⊂ int(Q)` does NOT imply `K_cell ⊂ int(Q)` —
the certificate proves uniqueness at `t_mid` only, and the unexamined
variation `F(t, m) − F(t_mid, m) ≈ ∂F/∂t·(t − t_mid)` is absorbed only
empirically by `pad` inflation and bisection, never bounded. A non-monotone
intersection path is the residual risk class.

The audit's 1-D counterexample: `F(t, q) = q − (2t − 0.6)`, cell `[0,1]`,
`m = 0.5`, `Q = [0.2, 0.8]`, `Y = J = 1` — the code computes
`K = {0.4} ⊂ int(Q)` and certifies, but the true solution `q*(t) = 2t − 0.6`
leaves `Q` for `t < 0.4`.

This is the audit's ONLY finding with a complete code-level argument but no
failing real-crate reproducer yet. **This packet is two-phase and stops at
phase A if the defect is not demonstrable in the real crate.**

## Phase A — construct the smallest real failing witness (do this FIRST)

Build a real `IntersectionCurve` test (in the existing test module of
`intersection_curve.rs`) whose certified cell under-encloses: a non-monotone
intersection branch where the midpoint center term passes but the true solution
leaves the candidate `Q` box within a `t`-cell.

- **Preferred construction: a synthetic analytic surface pair.** A non-monotone
  intersection path arises when two surfaces cross twice near each other along
  the leader. Use the crate's available carriers (`Sphere`/`Plane`/`Cylinder`/
  `Torus`/`Line` — see the existing tests in this module for the witness
  pattern). The leader is a `ParametricCurve` whose parameter `t` maps onto the
  intersection. The goal is a cell where the seed hull + pad + bisection still
  certify while the true curve escapes `Q`.
- A deterministic 1-D reduction through the real `Sys` is not required; the
  witness must exercise the REAL `certify_cell`/`enclose_span` path with real
  `S0`/`S1`/leader types and show a certified `enclose` box that misses a
  sampled point of the true intersection curve.
- **Do not spend hours searching random geometries.** A handful of designed
  candidates is the budget. If, after your best designed candidates, no failing
  witness exists, go to the Phase-A-fails branch below — do NOT keep hunting.

### If Phase A produces no failing witness (explicit branch)

The seed hull (Q contains the path while it is monotone in each coordinate) plus
`pad` inflation plus bisection may make the audit's counterexample
unrepresentable in the real 4-D system — a stronger invariant than the report
considered. If so:

- write a concise owner-adjudication note in `RESULT.json.disagreements` (or a
  `QUESTION.md`) that states the invariant you found, with the evidence
  (the candidates you tried and why each cannot certify unsoundly);
- return `status: OWNER_BLOCKED` — do NOT force the center-term rewrite "to be
  safe". An unforced rewrite of the center term to `F(cell, m)` can break the
  packet's measured margins and is not justified without a failing witness.
- `tests_added: 0` and the notes carry the Phase-A evidence.

## Phase B — implement the parametric fix (only if Phase A produced a failing witness)

Decided repair, in `certify_cell`: bound the t-variation explicitly instead of
evaluating the center at `t_mid` alone. Two acceptable implementations:

- **Option 1 (preferred):** evaluate the center as `F(m, cell)` — pass `cell`
  for the `tt` argument of `sys.f_iv` while keeping the four `q` arguments as
  the degenerate midpoint intervals (this bounds `F(t, m) − F(t_mid, m)` over
  the cell WITHOUT dragging the q-decorrelation into the center, which is the
  reason the current comment gives for the point evaluation).
- **Option 2:** keep `F(t_mid, m)` and add an explicit certified
  `∂F/∂t · cell_radius` term to the K operator, where `∂F/∂t` is bounded
  over `(Q, cell)` (e.g. from `leader.enclose_der(1, cell)` combined with the
  existing `j_iv` structure) and `cell_radius = (cell.sup() − cell.inf())/2`.

Do NOT merely increase `pad`. Empirical widening is not a proof.

The regression (Phase A's witness) must FAIL before the fix and PASS after.
The pre-existing ISC tests (plane/plane, plane/cylinder, sphere/sphere) must
stay green — a non-monotone cell now refusing (`None` → bisect) instead of
certifying is the expected behavior change, and the existing witnesses are
monotone, so they must not regress.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. Any small
float in test code needs the same-line `// H-3` comment. Run `bash
scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Increasing `pad`, `GROWTH`, or
`INITIAL_PAD` as the "fix". Returning `OWNER_BLOCKED` without the Phase-A
evidence note. Weakening an existing test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the Phase-A witness construction cannot be expressed against the real
  `IntersectionCurve`/carrier API → `SPEC_GAP`, with the exact mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`,
`OWNER_BLOCKED`. On any non-`DONE` status also write `QUESTION.md` beside it.
For `OWNER_BLOCKED`, the notes/disagreements carry the invariant argument.

Commit on the current branch with subject
`fix(evidence): parametric Krawczyk center term over the cell (BG-AUD-FIX-004)`
(or `wip(evidence): AUD-004 phase-A no-witness adjudication (BG-AUD-FIX-004)`
for the OWNER_BLOCKED path).
