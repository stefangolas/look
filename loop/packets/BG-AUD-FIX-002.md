# WORK PACKET BG-AUD-FIX-002 — whole-span deviation endpoint soundness (AUD-002)

You are repairing one defect found by the formal-kernel correctness audit
`loop/audits/BG-AUDIT-001.md` (finding AUD-002). Everything you need is in this
document. **Do not read any other spec file** — this packet is self-contained.
If something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```json
{"id":"BG-AUD-FIX-002","status":"DONE","contracts":["AUD-002"],
 "tests_added":2,"deviations":[],"disagreements":[],
 "baseline_failures":[],"notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-AUD-FIX-002
contract:    [AUD-002]
class:       mechanical
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/deviation.rs
read_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
  - vendor/truck/truck-geometry/src/nurbs/bspline.rs
  - vendor/truck/truck-topology/src/invariants/same_parameter.rs
tests_required:
  - route1_degree0_half_span_endpoint_deviation_refuses
  - route1_degree0_exact_pair_still_certifies
budget:      {turns: 30, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 5, cmd: "grep -c 'fn route1' vendor/truck/truck-evidence/src/deviation.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn control_point_box' vendor/truck/truck-evidence/src/deviation.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'fn certify_deviation' vendor/truck/truck-evidence/src/deviation.rs"}
```

## Problem

`certify_deviation`'s route 1 certifies
`|| carrier(t) − leader(phi(t)) || ≤ tau` for ALL `t` in the span by hulling
the coefficientwise difference spline. For a **degree-0** spline the certificate
is false at the span's right endpoint: truck's B-spline convention is
right-open (`knot_vec.rs`: "the B-spline basis function is based on the
characteristic function of the right-open intervals [s, t)"), so `subs(t)` at
an interior knot returns the NEXT span's value, and the sub-piece `[lo, hi)`
extracted by `cut` carries only the value on `[lo, hi)` — the value the source
curve attains at `t = hi` is cut away. The control-point hull then omits it and
the certificate can be false by an arbitrary amount.

**The exact audit witness** (already probed and confirmed on this tree):

- carrier: degree-0 spline, knots `[0, 0.5, 1]`, control points
  `[(0,0,0), (0,0,1)]`;
- leader: degree-0 spline, same knots, control points `[(0,0,0), (0,0,0)]`;
- `phi = ParamMap::IDENTITY`; span `[0, 0.5]`; `tau = 0.5`.

Current behavior: the half-span certifies `bound ≈ 2.46e-14 ≤ 0.5` while the
true deviation at `t = 0.5` is `1.0` (`carrier.subs(0.5) = (0,0,1)`,
`leader.subs(0.5) = (0,0,0)`). The full span `[0,1]` correctly refuses
(`ForwardToleranceExceeded { bound: 0.99999..., allowed: 0.5 }`), proving the
mechanism is the cut-away hull.

**Your first obligation — observe the regression fail on the buggy code:**
before making any edit, add the required regression test below to the crate's
test module and run it; it must FAIL (the half-span certifies a hold when it
must refuse). Record that pre-fix observation in `RESULT.json.notes`. Then
implement the repair and watch the same test PASS. Never land a fix whose new
regression has only ever been seen passing.

## Repair

In `route1` (`vendor/truck/truck-evidence/src/deviation.rs`), the hull of every
certified piece must contain the values the source difference spline attains at
the piece's **right-open endpoint semantics**. The decided repair:

- keep the coefficientwise difference spline `diff` as it is today;
- in the worklist loop, for each piece compute the piece's hull as today, then
  ALSO union in the two point values `diff.subs(a)` and `diff.subs(b)` where
  `[a, b]` is the piece's knot-range span (`a`/`b` are parameter values in the
  ORIGINAL difference spline's space — read them off the piece's knot vector as
  the loop already does). Concretely, extend the per-axis min/max used by
  `control_point_box` with these two points.

This is exactly the pattern `bspline.rs`'s `hull_sub_curve` uses (union the
endpoint `subs` values into the hull). For degree >= 1 the endpoint values are
already inside the hull up to rounding (the curve is continuous at interior
knots), so the existing degree-1/2 tests must remain green unchanged. For
degree 0 the union supplies the right-open endpoint value that `cut` omits, so
the witness above must now refuse with `ForwardToleranceExceeded`.

Do NOT:
- special-case degree 0 by refusing all degree-0 inputs (the contract
  deliberately does not refuse them; the union fix handles them);
- relax `tau` or pad the hull with an arbitrary slack;
- weaken the certificate fields.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. The test
uses `tau = 0.5` (no match) and control-point coordinates `0.0`/`1.0` (no
match). If you must write any small float, add the same-line `// H-3` comment.
Run `bash scripts/kernel-gates.sh <your base commit>` yourself before writing
`RESULT.json`. The test module already carries `#![allow(clippy::unwrap_used,
clippy::expect_used)]` with `#![deny(clippy::unwrap_used)]` — keep that shape;
do not use `unwrap`/`expect` in the test bodies (use `match`/`matches!` on the
`Outcome`).

## Tests required (exact names)

Both in the existing `mod tests` of `deviation.rs`.

1. `route1_degree0_half_span_endpoint_deviation_refuses`

   Build the audit witness exactly:
   ```rust
   let knots = KnotVec::try_from(vec![0.0, 0.5, 1.0]).expect("sorted");
   let carrier = BSplineCurve::new(
       knots.clone(),
       vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 1.0)],
   );
   let leader = BSplineCurve::new(
       knots,
       vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)],
   );
   let mut budget = Budget::new(1 << 16, 0, 0);
   let err = certify_deviation(&leader, &carrier, ParamMap::IDENTITY, iv(0.0, 0.5), 0.5, &mut budget)
       .expect_err("the degree-0 half span must refuse: the true endpoint deviation is 1");
   assert!(matches!(err, Refusal::ForwardToleranceExceeded { .. }));
   ```
   (`.expect_err` inside a test body is acceptable per the module's test-only
   allow; if the house lint objects, restructure with `match`.) This test must
   FAIL before the repair and PASS after.

2. `route1_degree0_exact_pair_still_certifies`

   The same construction with the leader equal to the carrier (control points
   `[(0,0,0), (0,0,1)]`): the half span `[0, 0.5]` at `tau = 0.5` must still
   certify `Ok` with `out.value <= tau` — the union adds the endpoint value
   `(0,0,1)` which the hull already contains, so the fix must not turn exact
   degree-0 pairs into refusals.

After implementing, run the full crate suite; every pre-existing test in
`deviation.rs` must stay green.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Also run, and report in `RESULT.json.notes`:
`cargo test -p truck-topology --lib invariants::same_parameter` — the
same-parameter checker consumes this certificate; the exact-pair and flip
cases must stay green (they are degree >= 1 and are unaffected, but verify).

Never run a bare `cargo test`.

## Forbidden

Editing any file outside `write_allow`. Relaxing `tau`, widening the hull pad,
or adding an `#[ignore]`. Weakening the certificate's `method`/`props`. Refusing
all degree-0 inputs as a shortcut for the union fix. Deleting or weakening a
negative test.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the degree-0 construction does not compile against the real `KnotVec` /
  `BSplineCurve` APIs → `SPEC_GAP`, with the exact mismatch and the corrected
  construction
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): union right-open endpoint values into the route-1 deviation hull (BG-AUD-FIX-002)`.
