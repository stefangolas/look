# CC-004-CLEAR — P5: ball clearance (BVH distance + signed-distance clearance)

CC program Phase A (spine S7). Theory:
`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §1 P5. This is the only
Phase-A packet outside `truck-certified`: the distance substrate is a leaf
fact of `truck-base` (the BVH there has ONLY overlap queries today), and the
clearance predicate consumes `truck-evidence`'s `ImplicitField` carriers.
Consumers (through the CC-000 manifest edge): loft validity far pairs
(CC-014), offset broad phase (CC-021/022), shell bridge (CC-023), blend
admissibility (CC-030).

```yaml
id:          CC-004-CLEAR
contract:    [CC-004-CLEAR]
class:       design
crates:      [truck-base, truck-evidence]
depends_on:  []
write_allow:
  - vendor/truck/truck-base/src/bvh.rs
  - vendor/truck/truck-evidence/src/clear.rs
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/tests/clear.rs
  - vendor/truck/truck-base/tests/bvh_distance.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-base/src/bvh.rs
  - vendor/truck/truck-evidence/src/contact
budget:      {turns: 22, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn candidate_pairs(' vendor/truck/truck-base/src/bvh.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn candidate_pairs_self(' vendor/truck/truck-base/src/bvh.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn query' vendor/truck/truck-base/src/bvh.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub trait ImplicitField' vendor/truck/truck-evidence/src/contact/implicit.rs"}
  - {id: A5, expect: 6, cmd: "grep -c 'fn implicit(' vendor/truck/truck-evidence/src/contact/implicit.rs"}
tests_required:
  - distance_lower_bound_never_exceeds_true_min_distance
  - distance_lower_bound_is_infinite_for_disjoint_piece_sets
  - ball_clearance_true_with_margin_at_known_separation
  - ball_clearance_false_when_ball_overlaps_exclusion
  - ball_clearance_refuses_when_interval_straddles_the_margin
  - round_mode_requires_field_negative_on_ball
  - fillet_mode_requires_field_positive_on_ball
```

Section 1: `truck-base/src/bvh.rs` — ADDITIVE only; existing entry points
and their behavior stay bit-identical (V5 identity guard doctrine):
`pub fn distance_lower_bound(&self, other: &Bvh<P>) -> f64` and `pub fn
distance_lower_bound_self(&self) -> f64`. Both return a certified LOWER
bound on the minimum distance between the two piece sets (≤ the true
minimum; `f64::INFINITY` only when the sets are provably disjoint at the
root). Implementation: dual-traversal with axis-aligned box-to-box lower
distance `max over axes of max(0, lo_b − hi_a, lo_a − hi_b)` — pre-made:
port the arithmetic, not the code, of the private `box_distance` in
`truck-evidence/src/fid/lfs.rs` (truck-base cannot depend on truck-evidence;
reimplement the six-line formula). Prune a pair when its box distance
exceeds the best-so-far upper bound computed from leaf boxes (deterministic
order: fixed child visitation order, never hash- or parallel-dependent).
Leave `LEAF_CAP` and the build order untouched.

Section 2: `truck-evidence/src/clear.rs` (new module; `pub mod clear;` in
`lib.rs` is the designed one-line conflict) — per spine S7 (as amended: mu
is an explicit parameter because truck-evidence cannot read
`construct/config.rs`):
`pub enum BallAdmissibility { Fillet, Round }` and `pub fn
ball_clearance(field: &impl ImplicitField, exclusion: &Box3, r: Interval,
mu: f64, mode: BallAdmissibility) -> Result<bool, Refusal>`. Semantics per
theory §1 P5, pre-made: `Clear` holds iff the contact ball of radius r is
farther than mu from the excluded boundary region AND the mode's
containment side holds. Decide BOTH sides in interval arithmetic over the
input boxes: (a) separation — lower-bound the distance from the ball to the
`exclusion` box using the Section 1 formula on the box level (the ball is
enclosed by its own box built from the caller's centre box ± r; the caller
passes the centre box inside `exclusion`'s coordinate frame — the exclusion
box is the region to stay away from, NOT containing the ball); (b) side —
`Round` requires `field ≤ 0` on the ball's box (negative-inside convention
documented at `contact/implicit.rs`), `Fillet` requires `field ≥ 0`. Each
side returns True / False / Undecided on its interval test; True on both →
`Ok(true)`; False on either → `Ok(false)`; Undecided on either →
`Err(Refusal::NumericallyUnresolved { spent, witness:
UnresolvedWitness::UncertifiedContainment })` with the caller's budget
entered as spent. Never widen, never retry internally — higher precision is
the CALLER's escalation (theory §9 retry rule lives above this layer).

Section 3: tests use the landed `ImplicitField` carriers directly (A4/A5:
plane, sphere, cylinder, cone, torus are implemented) — no CC-000 fixtures
are reachable from truck-evidence and none are needed. Ground truths: a
unit sphere field with the exclusion box `z ≥ 2` and ball r = 0.5 at the
origin → clear True with mu = 0.1; the same with ball at z = 1.6 → False;
centre box width chosen so the separation interval straddles r + mu →
Undecided refusal (H-3 opt-out on comparison lines).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-base`, `cargo check -p truck-evidence`, `cargo test -p truck-base
--test bvh_distance`, `cargo test -p truck-evidence --test clear`. No
workspace builds. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) if adding a method to `Bvh` forces any change to
existing methods or to `BoundedPiece`, STOP and file QUESTION.md — the
additive contract is the spine's; (2) if `ImplicitField`'s sign convention
is not uniformly negative-inside across all five carriers (read the impls,
A5), record the actual per-carrier convention in RESULT notes and implement
the side test against the DOCUMENTED convention of each carrier — do not
normalize them in this packet; (3) if the straddle case cannot be
constructed with the plane/sphere carriers, file QUESTION.md.
