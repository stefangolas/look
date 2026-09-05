# WORK PACKET CL-003-SWEEP-ENCLOSURE — certified derivative enclosures for SpineFrameSweep

You are implementing the sweep-side enclosure layer of the Carrier Lift
(CL) program. Everything you need is in this document and
`docs/CARRIER_LIFT_BUILD_SPEC.md`. Do not read other spec files. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          CL-003-SWEEP-ENCLOSURE
contract:    [CL-003-SWEEP-ENCLOSURE]
class:       mechanical
crates:      [truck-evidence]
depends_on:  []
write_allow:
  - vendor/truck/truck-evidence/src/enclosure_sweep.rs
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/num/sweep_sigma.rs
read_allow:
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-certified/src/formal/exact.rs
  - docs/CARRIER_LIFT_BUILD_SPEC.md
tests_required:
  - sweep_enclosure_brackets_brute_sample
  - sweep_sigma_g_bounds_brute_metric
  - sweep_enclosure_is_additive_over_window_split
budget:      {turns: 50, ctx_tokens: 120000}
```

**New files** (`enclosure_sweep.rs`, `num/sweep_sigma.rs`): H-1 applies.
The impl deliberately lives in a NEW file so this packet and
CL-000-SPLINE-ADMIT stay write-disjoint (they dispatch in parallel).

## Problem

The landed `EnclosureSurface` has impls for Cone, Cylinder, Plane, Sphere,
Torus (+ decorators) — none for `SpineFrameSweep`. The restricted solver
(BIE-002, landed) certifies sweep×canonical pairs, but certified gradient
bounds over the sweep side do not exist: every real sweep pair's
certification depends on this packet. Sweep-side σ_G (first fundamental
form bounds) is likewise absent.

## Scope decisions — pre-made, do not relitigate

1. **The sweep's derivative structure is compositional.** A
   `SpineFrameSweep` maps `(s, v)` through the spine curve's frame at s
   and the profile law at v — both landed analytic pieces with landed
   derivative machinery. The sweep-side grad enclosure COMPOSES the two:
   chain rule through the landed frame evaluator, bounds per piece from
   the landed enclosures/interval ops, outward by construction. Do NOT
   sample-and-hull the sweep directly; the composition IS the certificate.
2. **Windowed domain only.** The sweep's closed value carries the windowed
   domain fields (spec 5.10 posture, `sweep_surface.rs:52`); the enclosure
   is over `(s0,s1)×(v0,v1)`. Outside the window refuses typed.
3. **σ_G (sweep-side)**: the first fundamental form's four entries are
   expressible through the same composed derivatives; bound each entry
   with the landed interval ops (outward-rounded). Sweeps are pole-free —
   the booking records it; do not re-derive, but assert the boundedness
   (the enclosure refuses a non-finite bound rather than emitting one).
4. **Additivity**: splitting the window must give sub-enclosures whose
   union is contained in the parent's — asserted as a test, used by the
   solver's subdivision.
5. `enclosure.rs`, `sweep_surface.rs`, `krawczyk.rs`, `exact.rs` are NOT
   edited (V5 guard).

## Anchors — measured 2026-09-05, counts are exact

Locate by pattern, never by line number. If a count differs, STOP and
report `ANCHOR_MISMATCH`.

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `pub struct SpineFrameSweep` | 1 |
| A2 | `vendor/truck/truck-geometry/src/constructive/sweep_surface.rs` | `impl ParametricSurface for SpineFrameSweep` | 1 |
| A3 | `vendor/truck/truck-evidence/src/enclosure.rs` | `pub trait EnclosureSurface` | 1 |
| A4 | `vendor/truck/truck-evidence/src/num/mod.rs` | `^pub mod` | 3 |

A4 becomes 4 when you add `pub mod sweep_sigma;`.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!` reachable from geometry.
- **H-2** Fallible operations return `Outcome<T>`.
- **H-3** No absolute constants in predicates; test epsilons carry `// H-3`
  on the SAME line.
- **H-6** Bounds are certified (outward composition), never
  `Method::Exact`.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).** Never run a bare `cargo test` — scoped commands only.

## Tests required

1. `sweep_enclosure_brackets_brute_sample` — for ≥3 sweeps (straight spine,
   curved spine, scaled profile), the grad enclosure brackets ≥1000 brute
   `der` samples per component over the window.
2. `sweep_sigma_g_bounds_brute_metric` — the σ_G bounds bracket the brute
   first-fundamental-form entries on the same sweeps.
3. `sweep_enclosure_is_additive_over_window_split` — bisecting the window
   and re-enclosing yields bounds contained in (or equal to) the parent's.

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets -- -D warnings
cargo test -p truck-evidence --lib
cargo check -p truck-certified -p truck-shapeops
```

Send cargo output to a file and read the tail.

## Forbidden

Editing anything outside `write_allow` — especially `enclosure.rs`,
`sweep_surface.rs`, `krawczyk.rs`, `exact.rs`, anything under
`contact/` or `truck-shapeops/`, `scripts/kernel-gates.sh`, `Cargo.lock`.
Adding `#[ignore]`. Adding `#[allow]` without a same-line justification.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- the frame-law composition cannot be bounded without new machinery in
  `sweep_surface.rs` → `SPEC_GAP`, naming the missing accessor
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the
root of your worktree.

```json
{"id":"CL-003-SWEEP-ENCLOSURE","status":"DONE","contracts":["CL-003-SWEEP-ENCLOSURE"],
 "tests_added":3,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":4},
 "notes":"the composition chain you certified, and observed tightness vs brute samples"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): SpineFrameSweep derivative enclosures + sweep-side sigma_G (CL-003-SWEEP-ENCLOSURE)`.
