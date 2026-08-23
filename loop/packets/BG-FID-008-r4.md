# WORK PACKET BG-FID-008-r4 — relative width floor and an ulp-widening retry at the floor

You are amending one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Your prior work is already committed on this branch** (`c6a833e`, `6534bc8`,
`b45fd6d`). The r3 run correctly stopped BLOCKED: with `WITNESS_T = 0.71` the
double-cover root at `t_x + 2π` sits at the MIDPOINT of its floor-width box,
krawczyk bisects through the floor box at exactly the root, and strict-interior
Unique is unreachable. r3's instrumentation and extended machine-check were
right, and they localize the real defect — which is in THIS engine's floor,
not in any witness parameter:

**`WIDTH_FLOOR` is absolute (`8·EPS`).** At `t ≈ 0.7` that is 16 ulps (room
for K's interval rounding to contract strictly inside); at `t ≈ 7` it is 2
ulps (none). One crossing of the double-cover witness is structurally always
at `|t| ≥ 2π`, so no witness parameter can dodge the large-magnitude regime.
The floor must scale with the parameter magnitude. And even with a relative
floor, a descending root can land 1-2 ulps from a box edge (measured: at
`WITNESS_T = 0.71` the margins are 5/7 ulps — fine — but at `0.7` they are
11/1 — not), so the floor case also needs a bounded retry on a widened box.

```json
{"id":"BG-FID-008-r4","status":"DONE","contracts":["BG-FID-008"],
 "tests_added":8,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-008-r4
covers:      [BG-FID-008, BG-FID-008-r2, BG-FID-008-r3]
contract:    [BG-FID-008]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
budget:      {turns: 20, ctx_tokens: 70000}
anchors:
  # Pinned to THIS branch's tip b45fd6d with `git show`, because the packet
  # is dispatched onto a branch carrying prior work. A count mismatch is a
  # stop condition (ANCHOR_MISMATCH).
  - {id: X1, expect: 21, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'WITNESS_T'"}
  - {id: X2, expect: 1, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'fn sup_distance'"}
  - {id: X3, expect: 0, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'DISC_DECIDE_WIDTH'"}
  - {id: X4, expect: 2, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'boundary_root_on_disc_edge'"}
  - {id: X5, expect: 1, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'const WIDTH_FLOOR'"}
  - {id: X6, expect: 1, cmd: "git show b45fd6d:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'double_cover_witness_refuses'"}
```

## Decisions already made for you

### Decision 1 — relative width floor

Replace the `const WIDTH_FLOOR` with a function:

```rust
/// At or below this width a parameter box cannot subdivide further. The
/// floor is RELATIVE to the parameter magnitude: 8 ulps at the box's own
/// scale, never below 8 ulps of a unit-width interval. An absolute floor is
/// 16 ulps near the origin but only 2 ulps at t ~ 7 — too narrow for the
/// interval K operator to contract strictly inside, which strands every
/// descending root with |t| > 2 (measured on the double-cover witness).
/// H-3: a dimensionless width in parameter units, not a model-space length.
fn width_floor(tt: &Interval) -> f64 {
    8.0 * f64::EPSILON * tt.inf().abs().max(tt.sup().abs()).max(1.0) // H-3: 8 ulps at the box magnitude
}
```

Every use of the old const (`width <= WIDTH_FLOOR` guards in the worklist
and the terminal case) reads `width <= width_floor(&tt)`. Delete the const.
Nothing else about the worklist changes.

### Decision 2 — one bounded ulp-widening retry at the floor

In the terminal case (a popped floor-width box whose `krawczyk` call returns
`Err`), retry ONCE on the widened box
`[next_after(lo, -inf); next_after(hi, inf)]` four times per side (four
`f64::next_after` steps outward per endpoint — write them out, no loop magic),
then take the disc decision / count semantics on whichever box certified.
Widening is SOUND for the same reasons the engine's other rules are:
`KrawczykProof::Unique` on the widened box still certifies exactly one root
in it (the operator's own discipline cannot certify a box holding two), the
dedupe rule absorbs the slightly wider point-box, and a root that was on the
original box's edge or within 1-2 ulps of it is now strictly interior with
multi-ulp margins (measured at the historical `t_x = 0.7`: margins 11/1
before, 15/5 after). Second `Err` → `Err(SheetCountUnresolved)` as before.
Extend the module docs' resolution-limit paragraph by one sentence covering
the retry. A widened-box `Unique` follows the same count/dedupe path as any
other certified box.

### Decision 3 — tests

The seven existing tests stand with their current expectations (the
`double_cover_witness_refuses` test now passes: the root at `t_x + 2π` has
5/7-ulp margins at the relative floor — no widening needed for it). Add ONE
regression test for the retry path, using the historical edge case verbatim:

8. `edge_coincident_root_t07_certifies_after_widening` — the double-cover
   witness with a LOCAL witness parameter const `0.7` (named, `// H-3:`,
   shadowing nothing): at `t_x = 0.7` the second in-disc root at
   `6.983185307179586` lands 1 ulp from its relative-floor box edge (margins
   11/1), the first krawczyk refuses, and the widened retry (margins 15/5)
   certifies. Expected `NotOne { count: 2 }` — the r2 run's exact measured
   failure, now passing through the retry.

### Decision 4 — machine-check, through the floor and in ulps

Before writing RESULT.json, extend the machine-check script (keep it and its
output in `notes`): for every floor-descending root of every certifying test,
simulate the bisection descent (`mid = 0.5*lo + 0.5*hi`) with the RELATIVE
floor, then report the root's margins to both terminal-box edges in ulps of
the root, and — if either margin is below 2 — the margins after the 4-ulp
widening. Expected values (verify you reproduce them):
`0.71 → 11/2`; `0.71 + 2π → 5/7`; `0.7 + 2π → 11/1` widening to `15/5`.
Also re-derive the crossing distances as in r3 (`eps/2`, `eps*cos(t/2)` at
both witness parameters, pair separations, `eps`, `3*eps`).

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. Every new
or changed float is a named const or written expression with a same-line
`// H-3:` comment. Run `bash scripts/kernel-gates.sh fc8925f` before writing
RESULT.json (the base spans this branch's whole history).

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast        # 8/8 must pass
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh fc8925f
```

Note: this branch does NOT contain BG-NUM-003-r2 (landed separately on
integration); the tests must pass without it. The tangential and
budget-exhaustion tests may run slower without that fix — that is expected
and not a failure.

## Forbidden

Editing files outside `write_allow`. Changing any test expectation except
adding test 8. Changing the disc decision, the dedupe rule, the prunes, or
the worklist's subdivision structure (Decisions 1-2 are the only engine
edits). Weakening or deleting tests. Bare float literals without `// H-3`.
`unwrap()`/`expect()` on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the relative floor + widening retry still cannot certify the double-cover
  roots → `BLOCKED` with the instrumentation
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
fix(evidence,fid): relative width floor and ulp-widening retry at the floor (BG-FID-008-r4)
```
