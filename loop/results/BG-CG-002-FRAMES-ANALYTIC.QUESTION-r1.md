# QUESTION.md — BG-CG-002-FRAMES-ANALYTIC (status SPEC_GAP)

## What I attempted

1. Verified all seven anchors at dispatch time (A1=1, A2=1, A3=1, A4=0, A5=1,
   A6=1, A7=1) — all matched, so this is not an ANCHOR_MISMATCH.
2. Transcribed the packet exactly as specified: created
   `constructive/frame_fixed.rs`, `constructive/frame_up.rs`,
   `constructive/frame_radial.rs` from the quoted signatures/formulas; added
   the three `mod` lines to `constructive/mod.rs`; replaced the `frame()`
   stub body with the quoted dispatcher and the STUB doc note with the landed
   note in `constructive/recipe.rs`.
3. Ran `cargo check -p truck-geometry`. **The crate does not compile** (5
   errors: E0425 x2, E0433 x3). `cargo fmt --check`, `cargo clippy -- -D
   warnings`, and `cargo test` are therefore unreachable, so the packet's
   "Done when" cannot be met as written.
4. Reverted the transcription so the tree is clean. Nothing was committed:
   there is no code that compiles to commit under the packet's subject.

Four independent design conflicts block the landing. Each is a quoted,
normative element of the packet, so per the packet's rules I stopped and
report rather than choosing a fix.

---

## Conflict 1 — the quoted dispatcher cannot name the sibling modules (E0433)

The dispatcher body calls `frame_fixed::fixed_plane(...)`,
`frame_up::architectural_up(...)`, `frame_radial::radial_about_axis(...)` by
bare module name from inside `constructive::recipe` (edition 2021). Rust 2021
path resolution for a bare identifier in an expression resolves against the
current module, then the crate root / extern prelude — it does **not** search
the parent module's other children. Verified in-tree (3 x E0433) and with a
minimal edition-2021 reproduction. The existing code never does this:
`recipe.rs:5` reaches its sibling through `use super::errors::ConstructError;`.

Readings I could not choose between (each violates one explicit packet
constraint):

- **(a)** Add `use super::{frame_fixed, frame_up, frame_radial};` to
  `recipe.rs`. Keeps the quoted body verbatim, but adds a third change to a
  file the packet restricts to "EXACTLY two places ... every other line stays
  byte-identical".
- **(b)** Qualify the calls as `super::frame_fixed::fixed_plane(...)` etc.
  Keeps "two changes", but deviates from the quoted "exact" dispatcher body.

## Conflict 2 — `radial_about_axis` cannot produce the tangent it must return (E0425)

Quoted signature:
`radial_about_axis(origin: Point3, axis: Vector3, spine_point: Point3, at: f64)`
and the quoted dispatcher calls it as `(origin, axis, c, s)` — **no tangent**.
But the quoted body forms `b = t × n` ("t is the unit spine tangent from the
dispatcher") and returns `Frame3 { tangent: t, normal: n, binormal: b }`. The
law has no spine reference, no derivative, and no tangent argument, so `t` is
undefined in that scope (E0425 in-tree at the `b = t × n` and `tangent`
return sites).

Readings:

- **(a)** Both the signature and the dispatcher call must gain a tangent
  argument (changes two quoted artifacts).
- **(b)** `t` must be recomputed inside the law — not actually available, the
  law only receives a point, not the spine.

## Conflict 3 — test 6 pins a normal that contradicts the law's own formula for the quoted fixture

Quoted fixture: unit-circle arc in the XZ plane, `C(s) = (cos θ, 0, sin θ)`,
"about the Z axis through the origin" (`origin = (0,0,0)`, `â = (0,0,1)`).

- Quoted law: `n = radial̂`, `radial = d − (d·â)â`. For this fixture
  `d = (cos θ, 0, sin θ)`, so `radial = (cos θ, 0, 0)` and `n = (1, 0, 0)`
  for `cos θ > 0`.
- Quoted test 6: "`n` is the outward radial direction `(cos θ, 0, sin θ)`".

They agree only if the fixture is an **XY-plane** circle
(`C(s) = (cos θ, sin θ, 0)`), where `radial = d` and
`radial̂ = (cos θ, sin θ, 0)`. With the quoted XZ fixture the law's frame is
not even orthonormal: `t = (−sin θ, 0, cos θ)`, `t·n = −sin θ ≠ 0`, and
`b = t × n = (0, cos θ, 0)` has magnitude `|cos θ|`.

Readings:

- **(a)** Law formula normative → the fixture and the test-6 expectation must
  change (e.g. an XY-plane circle).
- **(b)** Fixture + test-6 expectation normative (`n = d̂`) → the law must be
  `radial = d` with no axis-component subtraction, contradicting the quoted
  formula.

## Conflict 4 — a landed test outside write_allow pins the stub behavior

`tests/constructive_contract.rs::recipe_evaluators_refuse_while_stub` asserts
`recipe.frame(0.5)` and `recipe.position(0.5, 0.25)` return
`Err(ConstructError::InvalidInput)` for a `LineSpine` + `FixedPlane{normal: +Z}`
recipe. Landing the frames makes both return `Ok`. That file is **not** in
`write_allow`, and the packet books only ONE in-place test amendment
(`recipe_position_refuses_until_frames_land`), so under the packet's own rules
I may not amend it, and it cannot keep passing.

Readings:

- **(a)** Amend `recipe_evaluators_refuse_while_stub` in place, following the
  CG-001 precedent the file's own comment documents (extends write_allow).
- **(b)** Leave it and accept a failing `cargo test` — contradicts the packet's
  "Done when".

---

## Bottom line

The packet cannot be landed as written: `cargo check -p truck-geometry` fails
before any test can run, and the quoted test-6 formula and the landed contract
test are each internally incompatible with the landed design. Status is
`SPEC_GAP`; I did not commit code (none compiles), and the tree is left as it
was at dispatch.
