# WORK PACKET BG-FIX-002 — relative parameter assertions in the circle search-parameter property tests

You are fixing one documented latent test defect in the vendored kernel.
Everything you need is in this document. **Do not read
`docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other spec file** — they are not
on your allowlist and this packet is self-contained. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```json
{"id":"BG-FIX-002","status":"DONE","contracts":["BG-FIX-002"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FIX-002
contract:    [BG-FIX-002]
class:       mechanical
crates:      [truck-geometry]
write_allow:
  - vendor/truck/truck-geometry/tests/circle.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-geometry/src/lib.rs
tests_required: []
# (intentionally EMPTY: no new test fns are added. V6 matches tests_required
# against #[test] fns ADDED IN THE DIFF; a fix packet listing pre-existing
# tests there fails the gate.)
budget:      {turns: 15, ctx_tokens: 40000}
anchors:
  - {id: A1, expect: 6, cmd: "grep -c 'prop_assert_near2!(s, t)' vendor/truck/truck-geometry/tests/circle.rs"}
  - {id: A2, expect: 7, cmd: "grep -c '#\\[property_test\\]' vendor/truck/truck-geometry/tests/circle.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'TOLERANCE' vendor/truck/truck-geometry/tests/circle.rs"}
```

(A3 pins that `TOLERANCE` is not yet imported into the file; `grep -c` exits 1
on zero matches, which IS the expected count.)

## Problem

The six `search_*` property tests in `vendor/truck/truck-geometry/tests/circle.rs`
assert the recovered parameter with `prop_assert_near2!`, whose epsilon is
`TOLERANCE2 = TOLERANCE * TOLERANCE = 1e-12` — an ABSOLUTE bound. The tested
parameter ranges reach `|t| ≈ 100` (`search_parameter_with_parameter_hint`
uses `-100.0..=100.0`; the range variants use starts in `-100.0..=100.0`), so a
few ulps of floating-point noise in the search — amplified by argument
reduction of large angles inside `subs`/`search_nearest_parameter` — exceeds
the fixed 1e-12 window.

Observed failure (BG-SOL-S5-CYLPAIR verify, 2026-08-26): minimal shrinking to

```
t: 12.566237776623453 (= 4*pi), d: 0.0
left: 12.566237776624755, right: 12.566237776623453   // |diff| ~ 1.3e-12 > TOLERANCE2
```

— a RELATIVE error of ~1e-13, i.e. pure representation noise, asserted against
an absolute squared epsilon. This is the same disease family as BG-FIX-001
(absolute tolerance on an unbounded-magnitude quantity) and the documented
flaky-proptest family in loop/STATE.md's traps; it has now blocked verifies
and poisons worktrees via proptest's SourceParallel persistence when it fires.

**Keep every strategy range exactly as it is.** The wide ranges are valuable
coverage — they exercise the hint/range machinery far outside the canonical
`[0, tau]` domain. Only the comparison predicate changes.

## Decisions already made for you

**Only `vendor/truck/truck-geometry/tests/circle.rs` changes.**

### 1. Add one import beside the existing uses:

```rust
use truck_base::tolerance::TOLERANCE;
```

(truck-base is a direct dependency of truck-geometry; do not route it through
the prelude.)

### 2. Replace ALL SIX assertion lines, verbatim:

each occurrence of

```rust
prop_assert_near2!(s, t);
```

becomes

```rust
prop_assert!(
    (s - t).abs() <= TOLERANCE * t.abs().max(1.0),
    "parameter drift beyond combined absolute/relative tolerance: s = {s}, t = {t}",
);
```

The predicate is `|s - t| <= TOLERANCE * max(1, |t|)`: identical to the old
absolute behavior for `|t| <= 1` and proportional above it — the same combined
form BG-FIX-001 landed in truck-geotrait's `assert_derivative_near`, chosen so
both fixes state ONE policy. Do not floor by anything other than `1.0`.

### 3. Nothing else changes.

All seven `#[property_test]` functions stay; their strategies stay; the two
plain `#[test]` fns at the bottom stay; `to_nurbs`' assertions stay (they
compare unit-circle points, bounded quantities).

Out of scope, do not touch: `truck-base/tests/newton.rs::test_newton1` (a
KNOWN separate latent flake with a different disease — a Newton basin issue,
not a tolerance swap — recorded in STATE; it stays open). Any other test file.
Any production file. The tracked
`truck-evidence/tests/plane_properties.proptest-regressions` seeds file.

## H-3, the house rule about float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless the line ends with an `// H-3` comment. This packet adds
no `1e-N` literals — `TOLERANCE` is imported by name, and the floor constant
`1.0` is outside GATE-2's pattern class. Run `bash scripts/kernel-gates.sh`
yourself before writing `RESULT.json`; V3 runs clippy with `-D warnings` on
your added lines and fmt on your changed file.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geometry
cargo clippy -p truck-geometry --all-targets --no-deps
cargo check --workspace --all-targets
cargo test -p truck-geometry --test circle --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. Then run the stabilization check — the whole
point of this packet — **five times**, in the host shell (PowerShell):

```powershell
1..5 | ForEach-Object {
    cargo test -p truck-geometry --test circle search_parameter_with_parameter_hint
    cargo test -p truck-geometry --test circle search_nearest_parameter_with_parameter_hint
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

Also delete any `*.proptest-regressions` file that appears under
`vendor/truck/truck-geometry/tests/` while you work — a saved seed replays
deterministically on every later run and will fake a persistent failure. Never
commit such a file.

If ANY stabilization run fails, do NOT write `DONE`: record the observed
failure in `notes`/`disagreements` and return `BLOCKED`, unless the failure is
demonstrably an unrelated pre-existing baseline defect — in that case it is a
`baseline_failures` entry naming the unrelated test. A failure of the circle
search tests THEMSELVES after this patch is this packet's own acceptance
criterion failing, never a baseline entry.

## Forbidden

Editing any file outside `write_allow`. Narrowing any strategy range. Adding
`#[ignore]`. Touching `to_nurbs`, `parameter_division`, or newton.rs. Committing
any `.proptest-regressions` file.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a stabilization run of the patched tests fails → `BLOCKED` (see above)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geometry): relative parameter assertions in circle search property tests (BG-FIX-002)`.
