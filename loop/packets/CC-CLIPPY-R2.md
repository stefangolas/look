# CC-CLIPPY-R2 — the two nurbs clippy findings CC-CLIPPY-R1's scope missed

The 09-05 02:12 battery attempt failed clippy on exactly two findings,
both introduced by CC-DEF-INTERPOLE (whose files were outside CC-CLIPPY-R1's
construct-only scope). Everything else is green: tests PASS (environmental
canary excluded with base verification), kernel-gates PASS. This packet is
the last blocker before the battery greens.

```yaml
id:          CC-CLIPPY-R2
contract:    [CC-CLIPPY-R2]
class:       mechanical
crates:      [truck-geometry]
depends_on:  [CC-000-CONTRACT]
write_allow:
  - vendor/truck/truck-geometry/src/nurbs/bspcurve.rs
  - vendor/truck/truck-geometry/src/nurbs/knot_vec.rs
read_allow:
  - loop/battery_clippy.log
  - docs/defects/NUM-INTERPOLE-OVERSHOOT-001.md
budget:      {turns: 10, ctx_tokens: 50000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c '!(data_extent > 0.0)' vendor/truck/truck-geometry/src/nurbs/bspcurve.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'for r in j..(j + p)' vendor/truck/truck-geometry/src/nurbs/knot_vec.rs"}
tests_required:
  - construct_interpole_bounds_still_green
```

Section 1: `bspcurve.rs:299` — `if !(data_extent > 0.0)`. The negated
comparison on f64 means `data_extent <= 0.0 || data_extent.is_nan()`. The
rewrite MUST preserve NaN-refusing semantics: `if data_extent <= 0.0 ||
data_extent.is_nan()` is the sanctioned form — do NOT write `data_extent <=
0.0` alone (that changes behavior for NaN extents: the current code refuses
them, the naive rewrite would admit them).

Section 2: `knot_vec.rs:826` — `for r in j..(j + p)` indexing `stations`.
This is the de Boor averaging inner sum: the accumulation ORDER is the
determinism contract. If an iterator rewrite is provably order-identical
(`stations[j..j+p].iter().enumerate()`-style, same visit order), take it;
otherwise a TARGETED same-line
`#[allow(clippy::needless_range_loop)] // de Boor averaging: fixed-index
accumulation order is the determinism contract` — module-level allows are
forbidden (H-1).

Section 3: verification — `cargo clippy -p truck-geometry --all-targets
--no-deps` reports ZERO findings in the two files; `cargo test -p
truck-geometry --test constructive_interpole_bounds` stays green; no other
file changes.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only. COMMIT BEFORE writing
RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) NO behavior change — NaN semantics and accumulation
order are the contract; targeted allows are the sanctioned escape, taken
with justification; (2) if clippy reports ADDITIONAL findings in these two
files beyond the battery's list, fix them too under the same rules and list
them in RESULT notes; (3) do not touch any other file — the battery is
green everywhere else.
