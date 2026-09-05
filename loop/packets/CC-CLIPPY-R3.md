# CC-CLIPPY-R3 — comprehensive construct clippy sweep (battery blocker, round 3)

The 09-05 battery attempt's clippy stage fails on ~45 findings across seven
construct files (the workers whose scoped checks never run clippy; same
class as CC-CLIPPY-R1/R2, now enumerated COMPLETELY from a full
`cargo clippy --locked --workspace --all-targets --no-deps` run so this is
the last round):

- `loft_weights.rs:198-199` needless_range_loop (iu/iv over du_gen/dv_gen)
- `offset_strata.rs:545,574,580,585` neg_cmp_op_on_partial_ord
- `blend.rs:482,869` (both classes)
- `blend_varradius.rs:491,694,728,787,1471,1891` (both classes)
- `face_consumption.rs:455`
- `setback.rs:234,318,339,361,363,406,407,672,676,682,712,714,790,791,817,828,833,841,1070,1072,1098,1177,1178,1240` (both classes)

```yaml
id:          CC-CLIPPY-R3
contract:    [CC-CLIPPY-R3]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/loft_weights.rs
  - vendor/truck/truck-certified/src/construct/offset_strata.rs
  - vendor/truck/truck-certified/src/construct/blend.rs
  - vendor/truck/truck-certified/src/construct/blend_varradius.rs
  - vendor/truck/truck-certified/src/construct/face_consumption.rs
  - vendor/truck/truck-certified/src/construct/setback.rs
read_allow:
  - loop/battery_clippy.log
budget:      {turns: 20, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'for iu in' vendor/truck/truck-certified/src/construct/loft_weights.rs"}
tests_required:
  - all_construct_scoped_tests_stay_green
```

Section 1: the rule set, identical to CC-CLIPPY-R1/R2 (pre-made):
(a) `needless_range_loop` — rewrite with iterators ONLY when the visit order
and index arithmetic are provably identical (determinism contract: fixed
accumulation order); otherwise a TARGETED same-line
`#[allow(clippy::needless_range_loop)] // fixed-index accumulation order is
the determinism contract` — module-level allows forbidden (H-1).
(b) `neg_cmp_op_on_partial_ord` — `!(x > 0.0)` on f64 is
`x <= 0.0 || x.is_nan()`; the rewrite MUST preserve NaN-refusing semantics —
`x <= 0.0` alone changes behavior. Where the nan-aware form reads worse, a
targeted same-line allow with justification is sanctioned.
(c) NO behavior change of any kind — every existing test must pass
unmodified; if a fix cannot be made without touching accumulation order or
NaN semantics, take the targeted allow.

Section 2: verification — `cargo clippy -p truck-certified --all-targets
--no-deps` reports ZERO findings in the six files (pre-existing formal/
findings are out of scope and expected to remain); the scoped tests of all
six modules stay green: construct_loft_weights, construct_offset_strata,
construct_blend, construct_blend_varradius, construct_face_consumption,
construct_setback. If clippy reports NEW findings in these files beyond the
battery's list, fix them too under the same rules and enumerate them in
RESULT notes.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow` (targeted same-line allows with justification are the
sanctioned escape).** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) zero behavior change — this packet edits lint shape,
never semantics; (2) pre-existing formal/ findings (common_arc.rs etc.) are
out of scope — the battery's baseline logic excludes them; (3) if a finding
cannot be resolved by rewrite-or-allow under (a)/(b), STOP and QUESTION.md.
