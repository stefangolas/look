# WORK PACKET BG-SOL-S6-IMPLICIT — certified implicit-field evaluation over canonical surface carriers

You are implementing one stage of the solver family's Contact Layer funnel.
Everything you need is in this document. **Do not read
`docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other spec file** — they are not
on your allowlist and this packet is self-contained. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```json
{"id":"BG-SOL-S6-IMPLICIT","status":"DONE","contracts":["BG-SOL-S6-IMPLICIT"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-SOL-S6-IMPLICIT
contract:    [BG-SOL-S6-IMPLICIT]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/implicit.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/contact/fe_ee.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-geometry/src/specifieds/mod.rs
  - vendor/truck/truck-evidence/src/lib.rs
tests_required:
  - implicit_zero_on_surface_witnesses
  - implicit_sign_away_from_surface
  - grad_matches_finite_difference
  - regular_on_detects_cone_apex
  - implicit_soundness_on_boxes
budget:      {turns: 25, ctx_tokens: 70000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod fe_ee' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct Box3' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub use inari::Interval' vendor/truck/truck-evidence/src/enclosure.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'implicit' vendor/truck/truck-evidence/src/contact/mod.rs"}
```

(A4 pins that no `implicit` machinery exists yet in the dispatcher file;
`grep -c` exits 1 on zero matches, which IS the expected count.)

## Problem

The Contact Layer's next funnel stage is **general validated FF**: face×face
carrier pairs without a closed-form cell in the §3.3 analytic table (the
offset mixed quadrics, later Torus and Placed). Every certified formulation of
that stage — event finding, Krawczyk arc continuation, singular-cell
detection — needs the same primitive first: **interval evaluation of each
canonical carrier's implicit function f(p) and its gradient ∇f on a Box3**,
with a documented sign convention and a regularity predicate. This packet
builds exactly that primitive and nothing else. It writes NO solver logic, NO
dispatch changes beyond one module declaration, and touches no existing
behavior.

This is substrate for later stages the way `num/krawczyk.rs` is substrate for
the numeric solvers: independently testable, deliberately minimal.

## Decisions already made for you

**New file `vendor/truck/truck-evidence/src/contact/implicit.rs`, declared in
`contact/mod.rs` beside `pub mod fe_ee;` as `pub mod implicit;` plus whatever
imports that declaration requires. That is the ONLY edit to mod.rs — do not
touch the dispatcher logic, `analytic_ff`, or anything else in it.**

### 1. The trait, verbatim:

```rust
/// Certified interval evaluation of a canonical carrier's implicit function.
///
/// The contact set of a carrier is `{ p : f(p) = 0 }`; the sign convention is
/// documented per implementing arm. Evaluations are sound interval enclosures:
/// the true f value of EVERY point in the box lies inside the returned
/// interval. This trait is substrate for the general validated FF stage
/// (event finding, Krawczyk continuation); it decides nothing about contact
/// by itself.
pub trait ImplicitField {
    /// Sound interval enclosure of f over the box.
    fn implicit(&self, p: &Box3) -> Interval;
    /// Sound interval enclosure of ∇f over the box, component order (x, y, z).
    fn grad(&self, p: &Box3) -> [Interval; 3];
    /// Proves ∇f ≠ 0 somewhere in every direction test: true iff SOME
    /// component's gradient enclosure excludes zero. `false` means "not
    /// PROVEN regular here", never "proven singular".
    fn regular_on(&self, p: &Box3) -> bool;
}
```

(`Box3` and `Interval` come from `crate::enclosure`; `Interval` is the re-
exported `inari` type. Follow enclosure.rs's own patterns for constructing
intervals from scalars — inari 2.0 has no `Interval * f64`; wrap scalar
coefficients as degenerate intervals.)

### 2. The five bare-carrier impls, with these forms and sign conventions:

Accessors are confirmed against the tree: `Plane::{normal, origin}`,
`Sphere/Cylinder/Torus::center`, `Sphere/Cylinder/Torus::radius` /
`large_radius` / `small_radius`, `Cone::{apex, half_angle}`. Write `(x', y',
z')` for `p − c` below.

- **Plane** (`o`, unit normal `n = normal()`): `f = n · (p − o)`.
  ∇f = n. Regular everywhere.
- **Sphere** (center `c`, radius `r`): `f = |p−c|² − r²`. ∇f = 2·(p−c).
  Negative inside.
- **Cylinder** (z-axis line through `center = (cx, cy, cz)`, radius `r`):
  `f = (x−cx)² + (y−cy)² − r²`. ∇f = (2(x−cx), 2(y−cy), 0).
  Note `cz` does NOT enter the form — document that.
- **Cone** (apex `a`, half angle `θ`, opening along +z, `t = θ.tan()`):
  `f = x'² + y'² − (z'·t)²`. ∇f = (2x', 2y', −2z'·t²).
  `f(a) = 0`: the apex IS on the zero set and ∇f(a) = 0 there — the cone arm's
  `regular_on` must be able to return false near it (see tests).
- **Torus** (center `c`, large `R`, small `r`), via the sqrt-free quartic
  form with `g = |p−c|² + R² − r²`, `h = x'² + y'²`:
  `f = g² − 4R²h`. ∇f = 2g·∇g − 4R²·∇h with ∇g = 2(x', y', z'),
  ∇h = (2x', 2y', 0).

Do NOT implement `ImplicitField` for `CanonicalSurface` or for `Placed`
carriers — the dispatcher refuses `Placed` upstream and the GFF stage will
match the enum itself. A doc comment on the trait says exactly that, so the
omission reads as scoped, not forgotten.

All evaluation is plain sound interval arithmetic. No budget interaction, no
certificates, no `Method` — those belong to the consumers. H-1 applies as
everywhere (no unwrap/expect outside tests).

## Tests (all witnesses machine-checked at packet-writing time)

Required test names (in `#[cfg(test)] mod tests` inside implicit.rs, opening
with `#![deny(clippy::unwrap_used)]`... construct values explicitly so unwrap
is never needed):

- `implicit_zero_on_surface_witnesses` — for each carrier, a box collapsed to
  a known on-surface point returns an enclosure containing 0:
  sphere r=1 center origin at (0,0,1); cylinder r=1 z-axis at (1,0,5);
  cone apex origin half-angle π/4 at (1/√2, 1/√2, 1) — note 0.5+0.5−1 = 0
  exactly, but 1/√2 is irrational, so assert the enclosure CONTAINS 0 rather
  than equals it (also use the rational witness (0,1,1), which is exact);
  torus R=2 r=0.5 at (2.5,0,0), (2,0,0.5), (1.5,0,0) — all exact zeros;
  plane through origin with normal +z at any (x,y,z=0).
- `implicit_sign_away_from_surface` — sphere: origin → strictly negative
  enclosure, (2,0,0) → positive containing 3; cylinder: origin → negative,
  (2,0,0) → positive containing 3; cone: (0,0,1) → negative (inside the
  solid angle), apex box → contains 0; torus R=2 r=0.5: center → positive
  containing 14.0625.
- `grad_matches_finite_difference` — for each carrier, compare the gradient
  enclosure at a nondegenerate point against central differences of the
  scalar enclosure (step 1e-3, generous interval containment assertion), AND
  exact component checks where trivial: sphere grad at (1,0,0) ⊇/≈ (2,0,0);
  cylinder grad at (2,0,3) ≈ (4,0,0) with z-component exactly 0.
- `regular_on_detects_cone_apex` — cone: `regular_on` false for the box
  collapsed to the apex; true for a box away from the apex. Cylinder: true
  off-axis (x-component excludes zero), false ON the axis box ((cx,cy) slice)
  — both gradient components straddle zero there.
- `implicit_soundness_on_boxes` — for a NON-degenerate box, sample interior
  points and assert each point's exact f value lies inside the returned
  enclosure (soundness of interval evaluation over the whole box, not just at
  collapsed boxes): sphere unit-at-origin over the box
  x∈[0.9,1.1], y∈[−0.05,0.05], z∈[0.95,1.05] with sampled points including
  (1.0, 0.0, 1.0) and corners; cylinder likewise off-axis.

Note on constructors (verified in the tree): `Sphere::new(center, radius)`
returns the bare struct, while `Cylinder::new` and `Cone::new` return an
`Outcome<Certified<...>>` — see contact/mod.rs's own tests for the idiomatic
construction. Float literals in tests: H-3 forbids added `1e-N` literals
without a same-line `// H-3` opt-out; use rational decimals or mark lines.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --workspace --all-targets
cargo test -p truck-evidence --lib contact::implicit --no-fail-fast
cargo test -p truck-evidence --lib --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Editing the dispatcher logic, `analytic_ff`, `fe_ee.rs`, or any file outside
the write set. Implementing the trait for `CanonicalSurface`/`Placed`. Adding
solver, event-finding, or Krawczyk logic — this packet is the field layer
ONLY. Adding dependencies.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- an accessor named above does not exist → `SPEC_GAP`, naming the type
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(evidence): certified implicit-field evaluation over canonical carriers (BG-SOL-S6-IMPLICIT)`.
