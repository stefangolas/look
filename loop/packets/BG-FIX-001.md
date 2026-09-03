# WORK PACKET BG-FIX-001 — relative-tolerance derivative assertions in the geotrait positive-test helpers

You are fixing one documented latent test defect in the vendored kernel.
Everything you need is in this document. **Do not read
`docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other spec file** — they are not
on your allowlist and this packet is self-contained. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```json
{"id":"BG-FIX-001","status":"DONE","contracts":["BG-FIX-001"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FIX-001
contract:    [BG-FIX-001]
class:       mechanical
crates:      [truck-geotrait]
write_allow:
  - vendor/truck/truck-geotrait/src/traits/curve.rs
read_allow:
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-geometry/tests/nurbscurve.rs
  - vendor/truck/truck-geometry/tests/bspcurve.rs
  - vendor/truck/truck-topology/tests/euler_operators.rs
tests_required: []
# (intentionally EMPTY: this packet adds no test fns — it repairs two existing
# ones. V6 matches tests_required against #[test] fns ADDED IN THE DIFF, so a
# fix packet listing pre-existing tests there fails the gate. The stabilized
# tests are pinned by prose below and verified by V5's baseline comparison.)
budget:      {turns: 20, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn exec_concat_random_test' vendor/truck/truck-geotrait/src/traits/curve.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'assert_near!(concatted.der2' vendor/truck/truck-geotrait/src/traits/curve.rs"}
  - {id: A3, expect: 2, cmd: "grep -cE 'pub fn (concat|parameter_transform)_random_test' vendor/truck/truck-geotrait/src/traits/curve.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'InnerSpace' vendor/truck/truck-geotrait/src/traits/curve.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'tolerance::Tolerance' vendor/truck/truck-geotrait/src/traits/curve.rs"}
```

(A4 pins that `InnerSpace` is NOT yet imported; `git grep -c` exits 1 on zero
matches, which IS the expected count.)

## Problem

`truck-geotrait`'s positive-test helpers assert derivatives with an ABSOLUTE
tolerance (`assert_near!` → `Tolerance::near` → `abs_diff_eq(.., TOLERANCE)`,
`TOLERANCE = 1.0e-6`). Derivative magnitude is unbounded in the test inputs:
`concat_positive_test` (truck-geometry/tests/nurbscurve.rs:80) scales control
points by `w ∈ [-5, 5]` with rational weights up to 2, producing second
derivatives of magnitude ~2.7e4. The floating-point noise of two evaluation
paths scales with the magnitude (observed relative error ~2e-10), so the
absolute difference reaches several times 1e-6 and the assertion fails on
mathematically identical values:

```
left: Vector3 [26695.240734762312, ...], right: Vector3 [26695.240739834226, ...]
```

This is a pre-existing latent defect INDEPENDENT of any packet: it has blocked
three verifies of BG-SOL-S5-CYLPAIR (a packet that changed only
truck-evidence, which truck-geometry does not depend on), flaked
`bspcurve.rs::parameter_random_tests` before that, and is documented in
BG-ENC-004-OFFSET's RESULT and loop/STATE.md's traps. It went rare→persistent
and back; a flaky gate is worse than a failing gate.

**The fix is the same disease family as BG-TOL-001's core lesson: an absolute
epsilon compared against a quantity whose magnitude is unbounded is the wrong
predicate.** The derivative comparisons must be relative to their own
magnitude. Point/range assertions stay absolute — point coordinates in these
tests are bounded (~50) and have never flaked.

## Decisions already made for you

**Only `vendor/truck/truck-geotrait/src/traits/curve.rs` changes.** All three
callers of the affected helpers pass concrete curves whose vector types
already satisfy every bound you add — verified at dispatch time:

- `nurbscurve.rs` / `bspcurve.rs`: `NurbsCurve`/`BSplineCurve` over `Vector4`
- `euler_operators.rs`: a local `Segment` over `Vector3`

No caller file needs edits. Do not touch them.

### 1. Imports (top of curve.rs, lines 4-8), verbatim replacement:

```rust
use truck_base::{
    assert_near,
    cgmath64::{InnerSpace, Point2, Point3, Vector2, Vector3},
    tolerance::{Tolerance, TOLERANCE},
};
```

(`cgmath64` re-exports `cgmath::prelude::*`, which carries `InnerSpace`.
truck-geotrait does not depend on cgmath directly; import it only through
`truck_base::cgmath64`.)

### 2. One new private helper, placed immediately above `pub fn parameter_transform_random_test`, verbatim:

The predicate is `|a - b| <= TOLERANCE * max(1, |a|, |b|)`: exactly the legacy
absolute epsilon at or below unit magnitude, proportional above it. **Do not
floor `scale` by anything other than `1.0`** — a floor of `TOLERANCE` would
make near-zero comparisons an effective `TOLERANCE² = 1e-12`, a million times
stricter than the behavior being preserved.

```rust
/// Relative-tolerance assertion for derivative vectors.
///
/// An absolute `assert_near!` on a derivative is the wrong predicate whenever
/// the input data has unbounded magnitude: both sides' evaluation noise grows
/// with the magnitude while [`TOLERANCE`] stays fixed, so mathematically equal
/// derivatives fail once they are large enough. Compare the difference against
/// the magnitudes instead: `|a - b| <= TOLERANCE * max(1, |a|, |b|)`. Below
/// unit magnitude this is exactly the legacy absolute epsilon; above it, the
/// margin grows proportionally, so the predicate is scale-invariant where the
/// absolute one was not.
fn assert_derivative_near<V>(left: V, right: V)
where
    V: Debug + Tolerance + std::ops::Sub<Output = V> + InnerSpace<Scalar = f64>,
{
    let diff = (left - right).magnitude();
    let scale = left.magnitude().max(right.magnitude());
    assert!(
        diff <= TOLERANCE * scale.max(1.0),
        "derivatives differ beyond combined absolute/relative tolerance\nleft: {left:?},\nright: {right:?}\n|diff| = {diff}, scale = {scale}",
    );
}
```

**Why the relative coefficient is [`TOLERANCE`] itself and not a tighter
test-only constant:** the observed noise is ~2e-10 relative, so `TOLERANCE`
gives a ~5000x margin — loose-looking but deliberate. These helpers are the
kernel's only shared positive tests for `Concat`/`ParameterTransform`
identities; introducing a second, private epsilon would create a second
source of truth outside truck-base's tolerance module, which BG-TOL-001
exists to eliminate. Real concat/transform defects displace values by
parameter-window-scale or O(1) amounts, not parts in 1e8; a gate tightened to
1e-9-relative would re-acquire the very flakiness this packet removes the
next time the input distributions move. One managed epsilon, used as both the
absolute floor and the relative margin, is the coherent policy here.

### 3. Where-clause additions (the ONLY signature changes):

- `pub fn parameter_transform_random_test` and its private
  `exec_parameter_transform_random_test`: add
  `+ InnerSpace<Scalar = f64>` to the existing
  `C::Vector: Debug + Tolerance + std::ops::Mul<f64, Output = C::Vector>` line.
- `pub fn concat_random_test` and its private `exec_concat_random_test`: change
  `C0::Vector: Debug + Tolerance,` to
  `C0::Vector: Debug + Tolerance + std::ops::Sub<Output = C0::Vector> + InnerSpace<Scalar = f64>,`.

These are additive bounds on already-published helpers; every in-tree caller
satisfies them (verified above). Do not touch any other where-clause.

### 4. Assertion swaps — exactly six lines, nothing else:

In `exec_parameter_transform_random_test`:

- `assert_near!(transformed.der(t * a + b) * a, curve.der(t));`
  → `assert_derivative_near(transformed.der(t * a + b) * a, curve.der(t));`
- `assert_near!(transformed.der2(t * a + b) * a * a, curve.der2(t));`
  → `assert_derivative_near(transformed.der2(t * a + b) * a * a, curve.der2(t));`

In `exec_concat_random_test`:

- `assert_near!(concatted.der(t), curve0.der(t));`
  → `assert_derivative_near(concatted.der(t), curve0.der(t));`
- `assert_near!(concatted.der2(t), curve0.der2(t));`
  → `assert_derivative_near(concatted.der2(t), curve0.der2(t));`
- `assert_near!(concatted.der(t), curve1.der(t));`
  → `assert_derivative_near(concatted.der(t), curve1.der(t));`
- `assert_near!(concatted.der2(t), curve1.der2(t));`
  → `assert_derivative_near(concatted.der2(t), curve1.der2(t));`

Leave EVERY other `assert_near!` alone: range tuples, `subs`, `front`,
`back`, and all of `exec_cut_random_test`. They compare bounded quantities and
their absolute semantics are correct.

`cut_random_test` is NOT in scope even though it sits between the two edited
helpers.

## H-3, the house rule about float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless the line ends with an `// H-3` comment (its pattern is
`[^a-zA-Z0-9_]1(\.0+)?e-0?[0-9]+`; a bare `1.0` does not match). This packet
introduces exactly one float literal, the floor constant `1.0` in
`assert_derivative_near`, which is outside that pattern class; `TOLERANCE` is
imported by name. Run `bash scripts/kernel-gates.sh` yourself before writing
`RESULT.json`; V3 runs clippy with `-D warnings` on your added lines and fmt
on your changed file, so keep both clean as you go.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-geotrait
cargo clippy -p truck-geotrait --all-targets --no-deps
cargo check --workspace --all-targets
cargo test -p truck-geotrait --lib --tests --no-fail-fast
cargo test -p truck-geometry --test nurbscurve --test bspcurve
cargo test -p truck-topology --test euler_operators
bash scripts/kernel-gates.sh <your base commit>
```

The workspace check stays in because this packet changes signatures other
crates' tests consume. Never run a bare `cargo test`.

Then run the stabilization check — the whole point of this packet — **five
times**, in the host shell (PowerShell):

```powershell
1..5 | ForEach-Object {
    cargo test -p truck-geometry --test nurbscurve concat_positive_test
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

(5 consecutive passes is the done-bar; report the count in RESULT.json notes.
If ANY of the five fails, do NOT write `DONE`: record the observed failure in
`notes`/`disagreements` and return `BLOCKED`, unless the failure is
demonstrably an unrelated pre-existing baseline defect — in that case it is a
`baseline_failures` entry naming the unrelated test. A failure of
`concat_positive_test` ITSELF after this patch is this packet's own acceptance
criterion failing, never a baseline entry.)

## Forbidden

Editing any file outside `write_allow`. Changing `TOLERANCE`, `Tolerance`, or
anything in truck-base. Touching `exec_cut_random_test` or any non-derivative
assertion. Adding `#[ignore]`. Loosening any other tolerance. Renaming the
public helpers.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- a caller fails to compile under the new bounds → `SPEC_GAP` (that would mean
  this packet's caller survey is wrong; name the type that failed)
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(geotrait): relative-tolerance derivative assertions in positive-test helpers (BG-FIX-001)`.
