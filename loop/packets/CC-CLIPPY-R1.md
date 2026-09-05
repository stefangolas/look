# CC-CLIPPY-R1 — clippy debt in the landed construct modules (battery blocker)

The CC program's first battery attempt failed clippy on findings introduced
by landed construct packets (banded.rs, residual_solve.rs, loft.rs,
canal.rs). Pre-existing formal/ findings (cone.rs, common_arc.rs,
projection.rs) are baseline-excluded by the battery's modified-files logic
and are NOT this packet's business. Fix the construct findings so the
battery's clippy stage goes green on modified files.

```yaml
id:          CC-CLIPPY-R1
contract:    [CC-CLIPPY-R1]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-certified/src/construct/banded.rs
  - vendor/truck/truck-certified/src/construct/residual_solve.rs
  - vendor/truck/truck-certified/src/construct/loft.rs
  - vendor/truck/truck-certified/src/construct/canal.rs
  - vendor/truck/truck-certified/tests/construct_clippy_r1.rs
read_allow:
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 14, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'for i in 0..self.order' vendor/truck/truck-certified/src/construct/banded.rs"}
  - {id: A2, expect: 1, cmd: "grep -c '!(r.lo > 0.0)' vendor/truck/truck-certified/src/construct/canal.rs"}
tests_required:
  - scoped_tests_of_all_four_files_still_green
```

Section 1: the findings, verbatim from the battery log
(`loop/battery_clippy.log`): banded.rs:117/120/128 needless_range_loop;
residual_solve.rs:76/78/114 needless_range_loop; loft.rs:224
needless_range_loop; canal.rs:428/433 negated comparisons
(`!(x > 0.0)`). Fix each, with THE PRE-MADE RULE: these loops carry the
determinism contract (fixed accumulation order). Where an iterator/enum-
erate rewrite is provably order-identical (it usually is — the order of
visited indices must not change), rewrite; where a rewrite would obscure
the band arithmetic, use a TARGETED same-line `#[allow(clippy::
needless_range_loop)] // fixed-order band recurrence; index math is the
contract` with that justification — module-level allows are forbidden
(H-1). canal.rs: `!(x > 0.0)` on f64 means `x <= 0.0 || x.is_nan()` —
preserve the NaN-refusing semantics exactly; `x <= 0.0` alone changes
behavior. If a rewrite risks any semantic change, take the targeted allow
instead and say so in RESULT notes.

Section 2: verification — all four files' existing scoped tests stay green
(`cargo test -p truck-certified --test construct_banded`,
`construct_loft`, `construct_canal`, plus `construct_contract`); the
required test file constructs nothing new — it exists so the packet has a
test artifact: one smoke test asserting the modules still expose their
seam functions (the A1/A2 anchors' symbols). `cargo clippy -p
truck-certified --all-targets --no-deps` reports ZERO findings in the four
construct files.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow` (targeted same-line allows with justification are
the sanctioned escape).** **H-3: float comparisons in tests take the
`// H-3` opt-out ON THE SAME LINE.** **All cargo invocations go through
the queue (the `cargo` on PATH IS the queue shim). Do not invoke cargo by
absolute path; do not unset the shim.** Scoped checks only. COMMIT BEFORE
writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) NO behavior change is permitted — if a lint fix
cannot be made without touching accumulation order or NaN semantics,
targeted allow + RESULT note; (2) pre-existing formal/ findings are out of
scope — do not touch cone.rs/common_arc.rs/projection.rs; (3) if clippy
reports NEW findings in the four files beyond the battery's list, fix them
too (same rules) and list them in RESULT notes.
