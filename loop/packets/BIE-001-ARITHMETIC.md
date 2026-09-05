# WORK PACKET BIE-001-ARITHMETIC — outward-rounded 4-D interval boxes + range bounds

You are implementing the arithmetic substrate of the Certified Interaction
Engine (BIE) program. Everything you need is in this document and
`docs/BIE_BUILD_SPINE.md`. Do not read other spec files. If something you
need is genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you
stop and report, you do not research it.

```yaml
id:          BIE-001-ARITHMETIC
contract:    [BIE-001-ARITHMETIC]
class:       design
crates:      [truck-certified]
depends_on:  [BIE-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/lib.rs
  - vendor/truck/truck-certified/src/interval/mod.rs
  - vendor/truck/truck-certified/src/interval/box4.rs
  - vendor/truck/truck-certified/src/interval/bounds.rs
read_allow:
  - vendor/truck/truck-certified/src/formal/exact.rs
  - vendor/truck/truck-certified/src/hull.rs
  - vendor/truck/truck-certified/src/contract.rs
  - vendor/truck/truck-certified/src/construct/bie/fixtures.rs
  - vendor/truck/truck-certified/src/construct/bie/mod.rs
  - docs/BIE_BUILD_SPINE.md
tests_required:
  - interval_box4_refuses_inverted_and_nonfinite
  - interval_box_ops_are_outward_rounded
  - mean_value_bound_brackets_brute_sample
  - bernstein_box4_known_polynomial
  - bernstein_box4_brackets_brute_sample
budget:      {turns: 50, ctx_tokens: 120000}
```

**New files** (`interval/mod.rs`, `interval/box4.rs`, `interval/bounds.rs`):
H-1 applies — no `unwrap_used` without a justified same-line opt-out; the
kernel gates count new files.

## Problem

The BIE-002 solver must certify range bounds of the interaction form
$F(u,v,s,t)$ over 4-D parameter boxes. The scalar outward-rounded interval
type exists (`CertifiedInterval`, landed) but there is no box type, no
mean-value/Taylor range bound, and no Bernstein evaluation over a 4-D box.
This packet is that substrate — consumed by BIE-002 and BIE-004 through the
contract frozen below.

## Scope reduction — read before you plan

The booking assumed a new interval library was needed. It is not:
`CertifiedInterval` (`truck-certified/src/formal/exact.rs:227`) already
provides outward-rounded `add/sub/neg/mul/div/sqrt/width/contains`. **Reuse
it for every scalar operation.** Do not reimplement outward rounding; do not
edit `formal/exact.rs` (landed, V5 guard). Your module is boxes and bounds,
composed from the landed scalar ops.

## Contract — frozen signatures (spine §3; BIE-002 types against these)

```rust
/// A 4-D parameter box over (u, v, s, t). Refusing constructor (H-2):
/// inverted bounds and non-finite endpoints refuse.
pub struct IntervalBox4 { /* four CertifiedInterval components */ }

impl IntervalBox4 {
    pub fn new(bounds: [(f64, f64); 4]) -> Outcome<Self>;
    pub fn bisect(&self, axis: usize) -> Option<(Self, Self)>; // axis 0..=3
}

/// Mean-value (first-order Taylor) range bound of f over the box, given a
/// certified derivative-enclosure provider (consumes the landed
/// `EnclosureSurface::enclose_der`-style bounds; see read_allow).
pub fn mean_value_bound(
    f: &impl Fn(&IntervalBox4) -> (f64, f64),      // cheap center eval
    grad_enclosure: &impl Fn(&IntervalBox4) -> [(f64, f64); 4],
    box_: &IntervalBox4,
) -> (f64, f64);

/// Bernstein (n=3 or n=4 tensor grid) range evaluation over the box,
/// reusing the landed `hull_bernstein_2d` machinery compositionally —
/// `hull.rs` itself is NOT edited.
pub fn bernstein_box4(
    grid: &TensorGrid4, box_: &IntervalBox4,
) -> Outcome<(f64, f64)>;
```

`TensorGrid4` is your minimal value type (flat coefficient array with
documented layout). Where the landed API differs from this sketch in detail
(constructors, naming), **use what is actually there** and note the
difference in RESULT notes — the frozen contract is the API SHAPE above.

## Anchors — verified 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and report
`ANCHOR_MISMATCH` with what you saw.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-certified/src/formal/exact.rs` | `pub struct CertifiedInterval` | 1 |
| A2 | `vendor/truck/truck-certified/src/hull.rs` | `pub fn hull_bernstein_2d` | 1 |
| A3 | `vendor/truck/truck-certified/src/lib.rs` | `^pub mod` | 14 |
| A4 | `vendor/truck/truck-certified/src/contract.rs` | `pub struct IntervalEnclosure` | 1 |

A3 becomes 15 when you add `pub mod interval;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!`, or
  out-of-range indexing reachable from geometry. `bisect` on axis 4+ returns
  `None`, it does not index out of range.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line as the literal.
- **H-6** Float-computed bounds are never recorded as `Method::Exact`; the
  bounds are certified by construction (outward rounding), certify them
  accordingly.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim). Do not invoke cargo by absolute path; do not unset the shim.**
- Never run a bare `cargo test` — use the scoped commands below.

## Tests required

Named `#[test]` fns in the module's test sections — the verifier checks the
names appear in your diff.

1. `interval_box4_refuses_inverted_and_nonfinite` — `new` refuses lo>hi,
   NaN, and ±inf (H-2 refusals, not panics).
2. `interval_box_ops_are_outward_rounded` — box add/sub/mul composed from
   `CertifiedInterval` contain the exact result on known cases; width is
   monotone under bisection re-join.
3. `mean_value_bound_brackets_brute_sample` — for ≥3 test functions with
   known derivative enclosures, the bound brackets ≥1000 brute samples of f
   over the box.
4. `bernstein_box4_known_polynomial` — a tensor polynomial with known range
   (e.g. coordinate functions) evaluates to it exactly within `// H-3`
   tolerance.
5. `bernstein_box4_brackets_brute_sample` — same discipline as test 3.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib
cargo check -p truck-evidence
```

The last one proves you did not disturb the downstream crate. Send cargo
output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `formal/exact.rs`,
`hull.rs`, `contract.rs`, anything under `truck-evidence/` or
`truck-geometry/`, `scripts/kernel-gates.sh`, `Cargo.lock`. Reimplementing
outward rounding. Solver bodies (BIE-002's job). Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the mean-value bound cannot be certified without editing `formal/exact.rs`
  → `SPEC_GAP`, naming the missing scalar op
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree (not `loop/results/` — the orchestrator files it there).

```json
{"id":"BIE-001-ARITHMETIC","status":"DONE","contracts":["BIE-001-ARITHMETIC"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":1,"A3":14,"A4":1},
 "notes":"API differences from the frozen contract shape, and why"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(certified): outward-rounded 4-D interval boxes + mean-value/Bernstein bounds (BIE-001-ARITHMETIC)`.
