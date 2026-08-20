# WORK PACKET BG-ENC-002-TORUS — enclosure for the `Torus` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-TORUS","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-002-TORUS
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/torus.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/torus.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - torus_encloses_sampled_points
  - torus_trig_extrema_inside_interval
  - torus_enclosure_converges_under_bisection
  - torus_normal_cone_over_patch_and_full_sweep
  - torus_immersion_lower_bound_vanishes_on_a_spindle
  - torus_der_enclosures_match_partials
budget:      {turns: 34, ctx_tokens: 80000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl ParametricSurface3D for Torus' vendor/truck/truck-geometry/src/specifieds/torus.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Torus' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and one reference
carrier (`Plane`, exact because affine). This packet adds the **torus** — two
periodic angles instead of one, which is what makes it worth doing separately
from the cylinder. The parameterization (read it off `specifieds/torus.rs`,
confirmed at packet time) is

    S(u, v) = center + ((R + r·cos v)·cos u, (R + r·cos v)·sin u, r·sin v)

with `u` and `v` both in `[0, 2π)` and periodic, `R = large_radius`,
`r = small_radius`. `Torus::new` panics unless both radii are `> 0`, so
**`R > 0` and `r > 0` are invariants you may rely on**.

**What it does NOT guarantee is `R > r`.** A torus with `r ≥ R` is a *spindle*
torus: the inner circle `R + r·cos v = 0` is a genuine singular circle where
`S_u` vanishes and there is no normal. The carrier admits these and so must you.
This is the same shape of obligation the cone has at its apex, and it is why the
trait's `normal_cone` returns an `Option`.

**Where the interval trig comes from.** `inari::Interval` has **no** `sin`/`cos`
in this tree: they live in `inari`'s own `elementary` module behind its `gmp`
feature, and `truck-evidence` takes `inari` with `default-features = false`.
Use the crate's own certified pair instead —

    use crate::elementary::{cos, sin};

free functions from `inari::Interval` to `inari::Interval`, already
outward-rounded and already accounting for the interior extrema at `kπ/2`.
Write `cos(uu)`, never `uu.cos()`; the method does not exist and a design that
needs it is a design that stops. **Never evaluate a trig function only at the
interval endpoints** — an interval spanning an interior extremum (e.g.
`[0.4π, 0.6π]` for `cos`) must contain the extremal value, and endpoint
evaluation is the historic under-estimation bug this item exists to prevent.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/torus.rs`. It is
   already created and already declared as `pub mod torus;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` for structure, doc tone, and the
   `interval_at` helper (copy it or reuse it via `pub(crate)` — your call, but
   one definition is better than two).

2. **Compute the tube radius interval once**: `rho = interval_at(R) +
   interval_at(r) * cos(vv)`. Every coordinate and every partial is written in
   terms of `rho`, and doing it once is both clearer and tighter than inlining
   it three times. Note `rho` may straddle zero on a spindle torus; that is
   legal and `inari` handles it. Do **not** take an absolute value here.

3. **`enclose`**:

       x = center.x + rho * cos(uu)
       y = center.y + rho * sin(uu)
       z = center.z + interval_at(r) * sin(vv)

   all in `inari` arithmetic, which rounds outward for you. This is a *box*
   over the two independent angle intervals: it is not tight (the true patch is
   curved), it is sound, and soundness is the contract.

4. **`enclose_der(m, n, uu, vv)`** — from the partials in `torus.rs`:
   - `(1,0)` → `rho * (−sin u, cos u, 0)`, `z` degenerate at `0.0`;
   - `(0,1)` → `interval_at(r) * (−sin v · cos u, −sin v · sin u, cos v)`;
   - `(2,0)` → `rho * (−cos u, −sin u, 0)`;
   - `(1,1)` → `−interval_at(r) * sin(vv) * (−sin u, cos u, 0)`, i.e. the
     `u`-derivative of `(0,1)`; derive it rather than guessing, and check it
     against `torus.rs`'s own higher partials if it has them;
   - `(0,2)` → `interval_at(r) * (−cos v · cos u, −cos v · sin u, −sin v)`.
   For any `(m, n)` you do not implement in closed form, returning a
   sound-but-loose box is acceptable **only if you say which ones in
   `deviations`**. Returning a box that is too small is a soundness bug.

5. **`normal_cone(uu, vv)` returns `Option<DirCone>`.** The unit normal (read
   it off `torus.rs`, where `normal` is already unit) is

       n(u, v) = (cos v · cos u, cos v · sin u, sin v)

   — the point of the unit sphere at longitude `u`, latitude `v`. It does not
   depend on `R` or `r` at all.
   - **`None` when the cell can touch the singular circle**: that is when
     `rho` (from decision 2) contains `0.0`. On an ordinary torus (`R > r`)
     this never happens and you always return `Some`; on a spindle it is
     exactly the inner circle. Test both.
   - Otherwise: let `wu = uu.sup() − uu.inf()` and `wv = vv.sup() − vv.inf()`
     (do NOT wrap either), axis = `n` at the two midpoints, and
     **`half_angle = (wu + wv) / 2`, clamped above at `π`**. The justification,
     which belongs in the doc comment: on the unit sphere, moving by `Δv` in
     latitude moves you by at most `Δv` of angle, and moving by `Δu` in
     longitude moves you by at most `Δu` (the factor is `|cos v| ≤ 1`), so the
     angular distance from the midpoint normal is at most `(wu + wv)/2`. A
     half-angle of `π` is the whole sphere and is the honest answer for a full
     sweep. Sound, not tight; tightness is BG-ENC-004's problem, not yours.
   Name the `π` clamp as a `const` with a word on what it is (H-3).

6. **`immersion_lower_bound(uu, vv)`**: `S_u` and `S_v` are orthogonal with
   `‖S_u‖ = |R + r·cos v|` and `‖S_v‖ = r`, so

       ‖S_u × S_v‖ = r · |R + r·cos v| = r · |rho|

   Take the **smallest** `|rho|` over the cell: `0.0` if `rho` contains zero,
   otherwise `min(|rho.inf()|, |rho.sup()|)`; multiply by `r` and **round
   down** — this is a lower bound and returning a value a rounding-unit too
   large is a soundness bug, not a tightness one. Compute in `inari` and take
   `.inf()` of the result rather than doing it in `f64`. State the
   orthogonality argument in one line of comment; it is the reason this is a
   product and not a numerical minimisation.

7. **No changes to `enclosure.rs`, `harness.rs`, or `plane.rs`.** If you find
   yourself wanting to touch the trait, that is a SPEC_GAP, not an edit.

## Tests required

All in the `#[cfg(test)]` module of `torus.rs`, using the shared harness
(`crate::harness::{assert_encloses_surface, assert_converges}`) and the
`plane.rs` test style for literals (named consts; a `// H-3` same-line opt-out
if a bare float is ever unavoidable — note rustfmt moves trailing comments off
brace-opening lines).

1. `torus_encloses_sampled_points` — several boxes on an ordinary torus
   (`R = 3, r = 1`): a small patch; a patch crossing `v = π` (where `cos v` is
   at its interior minimum); a full `2π` sweep in `u` with a small `v` range;
   a full sweep in both. `assert_encloses_surface` with ≥ 20 samples per axis.
2. `torus_trig_extrema_inside_interval` — the spec's mandated unit test: for
   `vv = [0.9π, 1.1π]` the tube radius interval must contain the interior
   minimum at `cos π = −1`, so the enclosure must reach `R − r` in the radial
   direction and be strictly wider than endpoint-only evaluation would have
   given. State the check in terms of relations, not bit-equality.
3. `torus_enclosure_converges_under_bisection` — `assert_converges` from a
   moderate box, depth ~20.
4. `torus_normal_cone_over_patch_and_full_sweep` — small patch: axis ≈ the
   midpoint normal, `half_angle ≈ (wu + wv)/2`; full sweep in both angles:
   `half_angle` is the `π` clamp. In every `Some` case, assert the cone
   **contains** the sampled unit normals over a grid, by angle. This is the
   test that would catch a `(wu + wv)/2` replaced by `max(wu, wv)/2`.
5. `torus_immersion_lower_bound_vanishes_on_a_spindle` — on an ordinary torus
   (`R = 3, r = 1`) the bound is strictly positive everywhere and is a genuine
   *lower* bound (`≤` the sampled `‖S_u × S_v‖` over a grid, for every sample).
   On a spindle torus (`R = 1, r = 2`) it is exactly `0.0` for a `vv`
   containing the singular latitude `acos(−R/r)`, and `normal_cone` is `None`
   there.
6. `torus_der_enclosures_match_partials` — `(1,0)`, `(0,1)`, `(2,0)`, `(1,1)`
   and `(0,2)` enclosures contain the analytically sampled partials over a
   grid.

`DirCone` containment by angle: `cos(angle between axis and d) >= cos(half_angle)`
— implement as a small test-local helper with a comment; a `half_angle` at the
`π` clamp needs the `>=` with float tolerance to survive rounding.

## H-3, which is what rejected the two carrier packets before yours

GATE-2 fails any **added** line carrying a bare `1e-N` literal unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does not
know your literal is an angle, and it does not care that the line is in a test.
`BG-ENC-002-LINE` was rejected for one such line and `BG-ENC-002-CIRCLE` for
six, both times on assertion epsilons in tests, both times costing a verify.

So: **every comparison epsilon you write gets a same-line `// H-3:` comment
naming the dimensionless quantity being compared.** The house form, from
`truck-base/src/evidence.rs`:

    assert!((a - b).magnitude() < 1.0e-12, ...); // H-3: float slack between two unit direction vectors, not a length
    assert!((h - expected).abs() < 1.0e-12, ...); // H-3: float slack between two half-angles in radians, not a length
    assert!(cos_angle >= limit - 1.0e-12, ...);   // H-3: float slack between two direction cosines, not a length

Directions, angles, direction cosines, parameter values and interval bounds are
all dimensionless and all legitimate — the comment is what says so. A literal
that really is a model-space *length* does not get an opt-out; it goes through
`ToleranceCtx` instead. Run `bash scripts/kernel-gates.sh` yourself before you
write `RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The plane tests must keep passing unchanged.

## Forbidden

Editing any file outside `write_allow`. Changing the `EnclosureSurface` trait,
the harness, or `plane.rs`. Endpoint-only trig evaluation anywhere. Assuming
`R > r`. Returning `Some` from `normal_cone` for a cell whose tube radius
interval contains zero. Adding `#[ignore]`. Adding `unscaled_legacy(` call
sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `inari` lacks a trig or rounding primitive this design needs → `SPEC_GAP`,
  naming it — do not hand-roll directed rounding
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureSurface for Torus (BG-ENC-002-TORUS)`.
