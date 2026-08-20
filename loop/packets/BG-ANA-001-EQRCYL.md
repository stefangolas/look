# WORK PACKET BG-ANA-001-EQRCYL — exactly solvable pair: equal-radius cylinders, intersecting axes

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-ANA-001-EQRCYL","status":"DONE","contracts":["BG-ANA-001","BG-ANA-002"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: if a decision below is wrong,
say so rather than working around it.

```yaml
id:          BG-ANA-001-EQRCYL
contract:    [BG-ANA-001, BG-ANA-002]
class:       mechanical
crates:      [truck-evidence]
depends_on:  [BG-CE-006-ENUM]
write_allow:
  - vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs
read_allow:
  - vendor/truck/truck-evidence/src/lib.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-evidence/src/harness.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-geometry/src/specifieds/circle.rs
  - vendor/truck/truck-geometry/src/decorators/processor.rs
  - vendor/truck/truck-geometry/src/decorators/trimmied_curve.rs
tests_required:
  - eqrcyl_steinmetz_perpendicular_two_ellipses
  - eqrcyl_oblique_angle_two_ellipses
  - eqrcyl_parallel_axes_refused
  - eqrcyl_skew_axes_refused
  - eqrcyl_certificate_is_exact
budget:      {turns: 36, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum AnalyticIntersection' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum ExactCurve' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub type PlacedCircle' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub mod equal_radius_cylinders' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A5, expect: 4, cmd: "grep -c 'Ellipse' vendor/truck/truck-evidence/src/analytic/mod.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'impl ParametricCurve for UnitCircle<Point3>' vendor/truck/truck-geometry/src/specifieds/circle.rs"}
```

## Problem

**The classic exact case.** Two cylinders of **equal radius** whose axes
**intersect** (are coplanar and meet at a point, at a nonzero angle) meet in
**two ellipses** — with a rational parameterization, no iteration, no
approximation. The two ellipses lie in the two planes bisecting the angle
between the axes, each centred at the axes' intersection point, with
semi-minor axis `r` and semi-major axis `r / cos(θ/2)` where θ is the angle
between the axes. This packet classifies and emits them.

Why the equal radius matters — and is therefore a **precondition, not a
predicate**: for unequal radii the intersection is a quartic space curve
belonging to the general solver. Likewise skew (non-coplanar) or parallel axes
are outside this cell. Those placements are **refused**, never approximated.

Read `analytic/mod.rs`'s module docs and `#[cfg(test)]` module first —
`TrimmedCurve` does **not** remap its parameter; `subs(t)` takes the angle
directly, and the module's own tests assert that convention.

## Decisions already made for you

1. **One existing file**, `vendor/truck/truck-evidence/src/analytic/equal_radius_cylinders.rs`.
   It is already created and already declared as `pub mod
   equal_radius_cylinders;` in `analytic/mod.rs`, itself declared in
   `lib.rs`. **Both `lib.rs` and `analytic/mod.rs` are read-only for you** —
   editing either is a scope violation that will get this packet rejected.
   The declarations and the shared result type were landed up front by the
   orchestrator so the eight sibling packets have disjoint write sets and can
   run in parallel; your file currently holds only a scaffolding doc comment,
   which you replace. The crate-level `#![deny(...)]` covers your module; do
   not add a second header.

2. **The shared result type is `crate::analytic::AnalyticIntersection` with
   `crate::analytic::{AnalyticOutcome, ExactCurve, PlacedCircle}` — read
   `analytic/mod.rs` first.** You do NOT define any result type of your own.
   Because both canonical cylinders share the z axis, this cell takes the two
   axes **explicitly**, sharing one radius:

   ```rust
   pub fn equal_radius_cylinders(
       radius: f64,
       axis0: &(Point3, Vector3),
       axis1: &(Point3, Vector3),
   ) -> AnalyticOutcome
   ```

   (`axis.0` a point on the line, `axis.1` its direction; the direction need
   not be unit — normalize internally.)

3. **The exact predicates: interval computation, three-way comparison,
   refusal.** Compute predicate quantities as `inari::Interval` (inari rounds
   outward), with named private helpers written exactly this way:

   - `decisively_zero(i) == (i.inf() == 0.0 && i.sup() == 0.0)`
   - `excludes_zero(i) == (i.inf() > 0.0 || i.sup() < 0.0)`
   - `three_way(a, b) -> Option<std::cmp::Ordering>`:
     `Some(Less)` iff `a.sup() < b.inf()`; `Some(Greater)` iff `b.sup() <
     a.inf()`; `Some(Equal)` iff both intervals are degenerate and identical;
     `None` otherwise.

   **Undecidable is a stop, not a guess:** return
   `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0, 0, 0), witness:
   UnresolvedWitness::RootNotIsolated })` for undecidable *predicate*
   straddles. For **structurally out-of-cell placements** (below) the refusal
   is the typed envelope refusal instead — the two are different and must not
   be confused.

4. **Every `Ok` carries the exact certificate, field-by-field at every return
   site** — deliberately no helper (BG-EVD-002):

   ```rust
   let mut props = PropMap::new();
   props.set(Prop::AnalyticCarrier, Truth::True);
   Certified::new(
       value,
       Certificate {
           props,
           method: Method::Exact,
           budget_left: Budget::new(0, 0, 0),
           margin: Margin::UNBOUNDED,
           modulus: Modulus::Unbounded,
       },
   )
   ```

   Doc-comment what `Method::Exact` means here: the classification is exact
   (decisive interval predicates on the f64 carrier parameters), and the
   emitted ellipses are the closed-form intersections. Coordinates are
   computed in f64; the spec's obligation is "lies on both carriers to
   machine precision", asserted with an H-3-commented slack. No `τ_rep`
   anywhere.

5. **The classification algorithm, pre-decided.** Normalize `a0`, `a1` (f64;
   if `|a|²` is `decisively_zero` in inari →
   `Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate)`). Then:

   1. **Parallel axes**: cross product `a0 × a1` per component in inari; all
      three `decisively_zero` → parallel →
      `UnsupportedEnvelope(NonCanonicalCarrier)` (that placement belongs to
      the parallel-axis cell / coaxial cells); any component
      `excludes_zero` → not parallel, continue; otherwise → refuse
      (`NumericallyUnresolved`).
   2. **Coplanar (intersecting) axes**: for non-parallel lines, coplanarity
      is the scalar triple product `τ = (a0 × a1) · (p1 − p0)` **in inari**;
      `decisively_zero` → intersecting, continue; `excludes_zero` → skew →
      `UnsupportedEnvelope(NonCanonicalCarrier)`; otherwise → refuse.
   3. The intersection point `q` (f64 closed form: solve the two line
      equations — `q = p0 + ((p1 − p0) × a1) · (a0 × a1) / |a0 × a1|² · a0`
      written as the standard closest-point/intersection formula; verify
      numerically in the tests).
   4. `cosθ = a0 · a1` (unit vectors, f64), `c = √((1 + cosθ)/2) = cos(θ/2)`
      (f64). Emit `Ok(TwoCurves([ExactCurve::Ellipse(e0),
      ExactCurve::Ellipse(e1)]))` where, with `û = normalize(a0 × a1)`,
      `b̂+ = normalize(a0 + a1)`, `b̂− = normalize(a0 − a1)`:
      - `e0` lies in the bisector plane spanned by `(b̂+, û)`: centre `q`,
        semi-major `r / c` along `b̂+`, semi-minor `r` along `û`.
      - `e1` lies in the plane spanned by `(b̂−, û)`: centre `q`, semi-major
        `r / c` along `b̂−`, semi-minor `r` along `û`.
      **Verify the semi-axis orientation claim numerically before committing
      it** (the perpendicular Steinmetz test does exactly this); record any
      correction in `deviations`.

6. **Placing ellipses** — write this private helper in your own file:

   ```text
   fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4
   ```

   = `Matrix4::from_cols(Vector4::new(u.x, u.y, u.z, 0.0), Vector4::new(v.x,
   v.y, v.z, 0.0), Vector4::new(n.x, n.y, n.z, 0.0), Vector4::new(o.x, o.y,
   o.z, 1.0)) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)`.
   An ellipse with semi-axes `ru` along `u` and `rv` along `v`, centred at
   `o` (`n = u × v`), is `Processor::with_transform(TrimmedCurve::new(
   UnitCircle::<Point3>::new(), (0.0, TAU)), frame(u, v, n, o, ru, rv))`.
   Sibling shards write their own copy; that duplication is deliberate and
   explicitly not a deviation — do not share it and do not report it.

## Tests required

All in the `#[cfg(test)]` module of `equal_radius_cylinders.rs`: named consts,
and a same-line `// H-3:` comment wherever a bare float slack literal appears.

1. `eqrcyl_steinmetz_perpendicular_two_ellipses` — r = 1, axis0 = x̂ through
   the origin, axis1 = ŷ through the origin. Expect `TwoCurves` of two
   ellipses, each with semi-minor 1 and semi-major √2, in the planes y = x
   and y = −x. Sample both (≥ 30 points each); every point satisfies
   `y² + z² == 1` and `x² + z² == 1` to machine precision (H-3-commented
   slacks) — the Steinmetz conditions. Also assert each ellipse's centre is
   the origin.
2. `eqrcyl_oblique_angle_two_ellipses` — axis1 = normalize((1, 0, 1)) (45°);
   assert `TwoCurves`; sample both ellipses and check distance to each axis
   equals r (the distance-to-line test, written once as a test-local helper)
   to machine precision. Assert the semi-major/minor ratio is
   `1/cos(θ/2)` within an H-3-commented slack.
3. `eqrcyl_parallel_axes_refused` — axis1 = axis0 →
   `Err(UnsupportedEnvelope(NonCanonicalCarrier))`; axis1 = −axis0 → same.
4. `eqrcyl_skew_axes_refused` — axis0 = x̂ at the origin, axis1 = ŷ at
   (0, 0, 1): the triple product is decisively 1 →
   `Err(UnsupportedEnvelope(NonCanonicalCarrier))`.
5. `eqrcyl_certificate_is_exact` — for the Steinmetz outcome: the `Ok`
   carries `method == Method::Exact` and the `AnalyticCarrier` prop set to
   `Truth::True`.

## H-3, which is what rejected three packets in this family

GATE-2 fails any **added** line carrying a bare `1e-N` literal unless that same
line ends with an `// H-3` comment. It is a text gate on the diff: it does not
know your literal is an angle, and it does not care that the line is in a test.
`BG-ENC-002-LINE` was rejected for one such line and `BG-ENC-002-CIRCLE` for
six, both times on assertion epsilons in tests, both times costing a verify.

So: **every comparison epsilon you write gets a same-line `// H-3:` comment
naming the dimensionless quantity being compared.** The house form:

    assert!((a - b).magnitude() < 1.0e-12, ...); // H-3: float slack between two unit direction vectors, not a length
    assert!((h - expected).abs() < 1.0e-12, ...); // H-3: float slack between two half-angles in radians, not a length
    assert!(cos_angle >= limit - 1.0e-12, ...);   // H-3: float slack between two direction cosines, not a length

Directions, angles, direction cosines, parameter values, residuals of
unit-scale witnesses and interval bounds are all dimensionless and all
legitimate — the comment is what says so. A literal that really is a
model-space *length* does not get an opt-out; it goes through `ToleranceCtx`
instead. Run `bash scripts/kernel-gates.sh` yourself before you write
`RESULT.json`; it is the same script V4 runs.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps -- -D warnings
cargo test -p truck-evidence --lib --tests --no-fail-fast
cargo check --workspace --all-targets
```

Never run a bare `cargo test` — it builds 56 examples. Send cargo output to a
file and read the tail. The existing 115 lib tests + 3 integration tests must
keep passing unchanged.

## Forbidden

Editing any file outside `write_allow` — `lib.rs` and `analytic/mod.rs`
especially. Defining a private result enum. Changing the shared types, the
harness, or any carrier. Deciding a predicate by sampling the surfaces.
Returning an `Ok` arm chosen by an undecidable predicate. Handling unequal
radii, parallel or skew axes (they are refused, not solved). Adding
`#[ignore]`. Adding `unscaled_legacy(` call sites. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the closed forms of decision 5 cannot be made to pass the
   on-both-carriers tests and you cannot correct them → `SPEC_GAP`, with the
   witness and the failing sample
- `inari` lacks a primitive this design needs → `SPEC_GAP`, naming it
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): exact equal-radius cylinders (BG-ANA-001-EQRCYL)`.
