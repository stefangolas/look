# WORK PACKET BG-ENC-002-LINE — enclosure for the `Line<Point3>` carrier

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ENC-002-LINE","status":"DONE","contracts":["BG-ENC-001","BG-ENC-002"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ENC-002-LINE
contract:    [BG-ENC-001, BG-ENC-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/line.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-evidence/src/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/line.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
tests_required:
  - line_encloses_sampled_points
  - line_enclosure_is_exact_at_the_endpoints
  - line_enclosure_converges_under_bisection
  - line_tangent_cone_is_the_single_direction
  - line_der_enclosures_are_constant_then_zero
budget:      {turns: 26, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'impl<P: ControlPoint<f64>> ParametricCurve for Line<P>' vendor/truck/truck-geometry/src/specifieds/line.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Line' vendor/truck/truck-geometry/src/specifieds/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub mod plane' vendor/truck/truck-evidence/src/lib.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'impl EnclosureSurface for Plane' vendor/truck/truck-evidence/src/plane.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'pub fn assert_' vendor/truck/truck-evidence/src/harness.rs"}
```

## Problem

`truck-evidence` has the enclosure interface (BG-ENC-001) and one reference
carrier — `Plane`, exact because affine. `EnclosureSurface` therefore has a
reference impl and `EnclosureCurve` has **none**. This packet gives it one, and
picks the carrier where the same "affine, so exact" argument applies, so that
the curve side of the interface is established before any carrier that needs
transcendental functions.

The carrier (read it off `specifieds/line.rs`, confirmed at packet time) is the
tuple struct `Line<P>(pub P, pub P)` with

    C(t) = self.0 + t·(self.1 − self.0)

`der(t) = self.1 − self.0` constant, `der2 = 0`, and `der_n(n, t) = 0` for all
`n ≥ 2`. Note the domain is **not** restricted to `[0, 1]`: `ParametricCurve`
evaluates it for any `t`, and your enclosure must be correct for a `tt` that
lies outside `[0, 1]` or straddles it.

You implement `EnclosureCurve for Line<Point3>` — the concrete `Point3`
instantiation, because the trait requires `ParametricCurve<Point = Point3>`.
Do not try to make the impl generic over `P`.

**There is no interval trigonometry in this crate.** `inari` is taken with
`default-features = false`, which excludes its `elementary` module, so
`Interval::sin`/`cos` do not exist. You do not need them: every quantity here is
affine in `t` and `inari`'s ordinary arithmetic rounds outward for you. If you
find yourself reaching for a transcendental function, stop and re-read — it is
not needed and it is not available.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/line.rs`. It is
   already created and already declared as `pub mod line;` in `lib.rs`, and
   `lib.rs` is **read-only for you** — it is not on your `write_allow` and
   editing it is a scope violation. The declaration was made up front so the
   six sibling carrier packets have disjoint write sets and can run in
   parallel; the file currently holds only a scaffolding doc comment, which
   you replace. Crate-level `#![deny(...)]` in `lib.rs` covers your module; do
   not add a second header. Follow `plane.rs` for structure, doc tone, and the
   `interval_at` helper (copy it or reuse it via `pub(crate)` — your call, but
   one definition is better than two).

2. **`enclose(tt)`**: componentwise
   `p0 + tt * (p1 − p0)` in `inari` arithmetic, where `p0`, `p1` components go
   through `interval_at`. `tt` may be negative, straddle zero, or lie outside
   `[0, 1]`; `inari` multiplication handles mixed signs correctly, so do **not**
   hand-roll a sign case analysis. The result is the tightest possible box up to
   outward rounding, because the map is affine in `t`.

3. **`enclose_der(n, tt)`**:
   - `n == 0` → the same box as `enclose(tt)`. `ParametricCurve::der_n` returns
     `self.subs(t).to_vec()` for `n = 0`, so this is a *vector* whose components
     equal the point's coordinates. Match the carrier, do not "fix" it.
   - `n == 1` → the degenerate-per-component box at `p1 − p0`, computed as the
     `inari` difference of the endpoint intervals so the subtraction's rounding
     is captured rather than assumed away.
   - `n >= 2` → the zero box.

4. **`tangent_cone(tt)`**: the direction is the constant `d = p1 − p0`,
   independent of `tt`.
   - If the `n == 1` enclosure **contains the zero vector** — which for a
     degenerate `Line(p, p)` it does — return `None`. That is the trait's
     stated contract ("`None` when the derivative enclosure contains 0") and a
     degenerate line is exactly the case it exists for. Test it.
   - Otherwise return `Some(DirCone { axis: d.normalize(), half_angle: 0.0 })`.
     A constant direction has zero spread, and `0.0` is the honest half-angle;
     do not pad it to make containment tests easier. If you find that a
     containment helper needs a tolerance to pass against `half_angle = 0.0`,
     put the tolerance in the *test helper*, not in the returned cone, and say
     why in a comment.
   Deciding "contains zero" on a box: all three component intervals contain
   `0.0`. Write it as a small named helper, not three inline conditions.

5. **No changes to `enclosure.rs`, `harness.rs`, or `plane.rs`.** If you find
   yourself wanting to touch the trait, that is a SPEC_GAP, not an edit.

## Tests required

All in the `#[cfg(test)]` module of `line.rs`, using the shared harness
(`crate::harness::assert_encloses_curve`) and the `plane.rs` test style for
literals (named consts; a `// H-3` same-line opt-out if a bare float is ever
unavoidable — note rustfmt moves trailing comments off brace-opening lines).

1. `line_encloses_sampled_points` — several `tt`, including `[0, 1]`, a
   sub-interval, one entirely negative, one straddling zero, and one beyond
   `t = 1`; for a line that is axis-aligned and one that is not.
   `assert_encloses_curve` with ≥ 20 samples.
2. `line_enclosure_is_exact_at_the_endpoints` — for `tt = [0, 1]`, each
   component interval's bounds are within one rounding step of
   `min/max(p0_i, p1_i)`. This is the property that distinguishes an affine
   carrier from a subdivided one; assert it as a relation on widths, not as
   bit-equality.
3. `line_enclosure_converges_under_bisection` — halving `tt` at least halves
   each component width (up to rounding), down to depth ~20. The harness's
   `assert_converges` is written against `EnclosureSurface`; if it does not
   apply to a curve, write the loop locally rather than changing the harness,
   and note that in `notes`.
4. `line_tangent_cone_is_the_single_direction` — `Some` with
   `axis ≈ (p1 − p0).normalize()` and `half_angle == 0.0` for an ordinary line
   over several `tt`; **`None`** for the degenerate `Line(p, p)`.
5. `line_der_enclosures_are_constant_then_zero` — `n = 1` is the same box for
   every `tt` and contains the sampled `der(t)`; `n = 2` and `n = 5` are the
   zero box; `n = 0` agrees with `enclose`.

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

Editing any file outside `write_allow`. Changing the `EnclosureCurve` trait, the
harness, or `plane.rs`. Enabling a cargo feature or touching any `Cargo.toml`.
Adding `#[ignore]`. Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the trait cannot be implemented for `Line<Point3>` without changing it →
  `SPEC_GAP`, naming the obstruction exactly
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): EnclosureCurve for Line (BG-ENC-002-LINE)`.
