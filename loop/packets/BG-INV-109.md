# WORK PACKET BG-INV-109 — invariant checker 9: wedge non-degeneracy

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-109","status":"DONE","contracts":["BG-INV-109"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-109
contract:    [BG-INV-001]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/wedge.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/edge.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/tolerance.rs
  - vendor/truck/truck-geotrait/src/lib.rs
  - vendor/truck/truck-geometry/src/specifieds/plane.rs
  - vendor/truck/truck-geometry/src/specifieds/line.rs
tests_required:
  - wedge_right_angle_tent_holds
  - wedge_folded_coplanar_faces_violate
  - wedge_doubled_back_faces_violate
  - wedge_boundary_edge_is_skipped
  - wedge_projection_failure_is_unresolved
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod wedge' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod invariants' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn edge_iter' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn surface' vendor/truck/truck-topology/src/face.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'WedgeNonDegeneracy' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn sin_margin' vendor/truck/truck-base/src/tolerance.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'pub fn' vendor/truck/truck-topology/src/invariants/wedge.rs"}
```

(A7 pins the scaffold as EMPTY; `grep -c` exits 1 on zero matches, which IS
the expected count.)

## Problem

§1.1 invariant 9: the dihedral angle at every INTERIOR edge of a valid solid
boundary is bounded away from 0 and 2π — no folded (knife-edge) or
doubled-back (crack) wedges. This is the condition local feature size needs
to be positive (BG-FID-001).

Two honest scope statements, recorded in the checker's doc comment and
pinned by the spec at landing:

- **This v1 samples the edge's midpoint.** The whole-span version needs the
  edge's parameter image on each face — the pcurve (BG-CE-001's payload,
  unwired) — to feed the surfaces' `normal_cone`s. What v1 certifies: the
  wedge is non-degenerate AT the sampled point. The sample is the parameter
  midpoint of the edge's curve, and the projection of that point onto each
  adjacent face's surface.
- **Interior edges only.** An edge used by one face (an open boundary) has
  no wedge; an edge used by more than two faces is BG-INV-101's violation,
  not this checker's — both are skipped here.

The checkers module tree is already scaffolded and declared — read
`invariants/mod.rs` first. **Only `wedge.rs` is yours.**

## Decisions already made for you

1. **The public API, verbatim:**

   ```rust
   use crate::Shell;   // the struct lives at the crate root, NOT in ::shell
   use truck_base::cgmath64::{Point3, Vector3};
   use truck_base::evidence::{
       Budget, Certificate, Certified, ContradictionWitness, Margin, Method,
       Modulus, Outcome, Prop, PropMap, Refusal, Truth, UnresolvedWitness,
   };
   use truck_geotrait::{ParametricCurve, SearchParameter};
   use std::collections::HashMap;

   /// BG-INV-109: wedge non-degeneracy (§1.1 invariant 9) at every interior
   /// edge, v1: sampled at each edge's parameter midpoint.
   ///
   /// For each edge used by exactly two faces: take the curve's midpoint
   /// `p`, project `p` onto both faces' surfaces (`search_parameter`),
   /// take both unit normals there, and require
   /// `|n_A × n_B| >= sin_margin` — the sine of the normals' angle, zero
   /// exactly for the folded (normals parallel) and doubled-back (normals
   /// antiparallel) degenerate wedges. `sin_margin` is dimensionless; pass
   /// `ToleranceCtx::sin_margin()` for the house default.
   ///
   /// **v1 samples the midpoint only** — the whole-span certificate needs
   /// the pcurve parameter images (BG-CE-001's payload, unwired) feeding
   /// `normal_cone`. Edges used by one face (open boundary) are skipped;
   /// edges used by more than two faces are BG-INV-101's to catch, skipped
   /// here. A failed projection is `NumericallyUnresolved` (the point's
   /// containment in the surface could not be certified), never a
   /// violation. Localise: the refusal's `prop` names the invariant; the
   /// offending edge is the first in `edge_iter` order whose check fails.
   pub fn check<P, C, S>(
       shell: &Shell<P, C, S>,
       sin_margin: f64,
   ) -> Outcome<()>
   where
       C: ParametricCurve<Point = Point3> + Clone,
       S: truck_geotrait::ParametricSurface<Point = Point3>
           + SearchParameter<D2, Point = Point3>
           + Clone,
   {
   ```

   (Read `truck-geotrait`'s traits before writing: if `normal` is not a
   method on `ParametricSurface` itself, the bound that supplies it is
   `truck_geotrait::ParametricSurface3D` — use whichever provides
   `normal(u, v) -> Vector3`; `Plane` implements it. Do not import traits
   you do not call.)

2. **The body, in order:**

   - Build the edge→uses map: iterate `shell.face_iter()`, and within each
     face `face.boundary_iters()` (or `face.boundaries()` flattened — read
     `face.rs` and use what exists) collecting
     `(EdgeID<C>, face_index)` pairs into a
     `HashMap<EdgeID<C>, Vec<usize>>` (edge id → the indices of the faces
     using it). `EdgeID` is `Hash + Eq` by construction and lives at the
     crate root (`use crate::EdgeID;` — it is a type alias there, not in
     `edge.rs`).
   - For each edge id with exactly TWO using faces: run the wedge test of
     decision 3. One face or more than two: skip (documented).
   - First failure returns; all pass → the certificate of decision 4.

3. **The wedge test of one interior edge:**

   - `let (t0, t1) = edge_curve.parameter_range()` bounds; the midpoint
     parameter `t_mid = (t0 + t1) / 2.0` (extract the f64s from the
     `Bound`s; `Unbounded` → `NumericallyUnresolved` — no sampled point
     exists);
   - `let p: Point3 = edge.curve().subs(t_mid);` (`edge.curve()` needs
     `C: Clone` — the bound of decision 1 supplies it);
   - for each of the two faces: `let (u, v) =
     face.surface().search_parameter(p, None, SEARCH_TRIALS)?` — `None` →
     `Err(Refusal::NumericallyUnresolved { spent: Budget::new(0, 0, 0),
     witness: UnresolvedWitness::UncertifiedContainment })`
     (containment of the point in the surface could not be certified — the
     exactly-fitting variant);
   - `let n = face.surface().normal(u, v).normalize();` (normalize
     defensively — do not trust the trait to have done it);
   - `let sin_angle = n_a.cross(n_b).magnitude();` — if
     `sin_angle < sin_margin` → the violation of decision 5.
   - `SEARCH_TRIALS`: a named `const SEARCH_TRIALS: usize = 100;` (the
     crate's own `SEARCH_PARAMETER_TRIALS` in lib.rs is private; a local
     const is the H-4-clean equivalent).

4. **The holds certificate** — the house structural pattern:
   `props.set(Prop::WedgeNonDegeneracy, Truth::True)`, `method:
   Method::Float` (v1 samples and computes in f64 — recording `None` here
   would be a lie; `Float` is the honest method and the doc comment says
   so), `budget_left: Budget::new(0, 0, 0)`, `margin: Margin::UNBOUNDED`,
   `modulus: Modulus::Unbounded`.

5. **The violation refusal, verbatim:**

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::WedgeNonDegeneracy,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

6. **Tests** — one `#[cfg(test)]` module opening with
   `#![deny(clippy::unwrap_used, clippy::expect_used)]` (H-1 justification
   comment), `use super::*;` and the witness builders. Witnesses are
   `C = Line`, `S = Plane` (both implement every bound of decision 1 —
   `Line: ParametricCurve<Point = Point3>`, `Plane: ParametricSurface +
   SearchParameter<D2>`), `P = usize`. The tent: two triangular faces
   sharing one edge, their planes at a known angle — e.g. the shared edge
   along the x-axis from `(0,0,0)` to `(1,0,0)`, face A in the z = 0 plane,
   face B in the y = 0 plane (a 90° wedge). Each face's wire: the shared
   edge plus two edges to an apex (`(0.5, 0, 1)` for A's triangle,
   `(0.5, 1, 0)` for B's) — the second face's wire traverses the shared
   edge INVERTED (`edge.inverse()`) so the uses pair. Build vertices at the
   four points, edges with `Line` curves between them
   (`Edge::new(&v0, &v1, Line(v0_pt, v1_pt))`), `Wire::from(vec![…])`,
   `Face::new(vec![wire], plane)`.

   - `wedge_right_angle_tent_holds` — the 90° tent:
     `check(&shell, margin)` is `Ok` with
     `props.get(Prop::WedgeNonDegeneracy) == Truth::True`, using a named
     `sin` margin const (`0.5` — comfortable below sin(90°)=1, with an
     `// H-3:` comment naming it the test wedge margin).
   - `wedge_folded_coplanar_faces_violate` — both faces in the SAME plane
     (B's apex moved to `(0.5, 0, 1)` — same plane as A): the violation
     refusal (normals parallel).
   - `wedge_doubled_back_faces_violate` — B in the same plane but its wire
     traversed with the same orientation as A's (both faces "up" — normals
     antiparallel across the shared edge): the violation refusal.
   - `wedge_boundary_edge_is_skipped` — a shell with one triangular face
     only (every edge used once): `Ok` (all edges are boundary edges; the
     checker skips them — the doc comment of decision 1 says so).
   - `wedge_projection_failure_is_unresolved` — a face whose surface does
     not contain the shared edge's midpoint: e.g. the tent with face B's
     plane TRANSLATED off the shared edge (`Plane::new((0.5, 0.0, 0.1),
     …)` — a plane not through the edge): `Err(Refusal::
     NumericallyUnresolved { witness: UncertifiedContainment, .. })`.

   For the `Plane`/`Line` constructors read their source
   (`specifieds/plane.rs`, `specifieds/line.rs`): `Plane::new(origin,
   u_point, v_point)`, `Line(Point3, Point3)` — and note `Plane`'s
   parameterization `S(u, v) = o + u·(p−o) + v·(q−o)` with range `[0,1]²`;
   the shared-edge midpoint `(0.5, 0, 0)` must lie IN each test plane and
   within its parameter range for `search_parameter` to find it — place the
   witness planes so it does (the z=0 plane through the edge: `Plane::new(
   (0,0,0), (1,0,0), (0.5,0,1))` contains the midpoint at (u,v) = (0.5, 0)
   ✓).

7. One doctest on `check`: the right-angle tent, `is_ok()`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that line ends with an `// H-3` comment naming the
dimensionless quantity. This packet's floats are witness coordinates
(`0.5`, `1.0` — safe) and the sin margin (named const, `// H-3:` comment
naming it the test wedge margin). Run `bash scripts/kernel-gates.sh`
yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. The crate is clean at baseline (all tests,
116 doctests, zero clippy findings, measured at HEAD 49997d3); your bar is
everything above stays green plus your five tests and one doctest.

## Forbidden

Editing any file outside `write_allow`. Changing the refusal shapes or the
certificate fields of decisions 4-5 (the wave's seven checkers share them).
Recording `Method::None` on the holds certificate (the sampling is float
arithmetic — `Method::Float` is the honest label). Whole-span claims (v1
samples the midpoint; the doc comment must say so). Touching `shell.rs`,
`face.rs`, `edge.rs` or any geotrait file. Adding `#[ignore]`. Adding
`unwrap()`/`expect()` outside the test module.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- `ParametricSurface3D`/`SearchParameter` bounds do not fit `Plane` and
  `Line` as this packet assumes (read the traits first) → adapt the bounds
  and note it in `deviations`; only stop if no bound set compiles
- `Plane::search_parameter` cannot recover the midpoint of a point lying in
  the plane's parameter range → `SPEC_GAP`, with the exact witness and
  behavior
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): wedge non-degeneracy invariant checker (BG-INV-109)`.
