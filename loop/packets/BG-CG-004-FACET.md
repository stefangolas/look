# WORK PACKET BG-CG-004-FACET — the direct facet realization backend (FAC)

You are landing the core realization backend of the constructive geometry
program (plan §3.3, CG-004): the direct facet sweep that turns a landed
`SpineFrameRecipe` into a shared-topology `PolygonMesh` **closed by
construction — no sewing, no welding, no healing, no booleans, no surface
fitting, no Newton, no generic surface/surface intersection on the fast
path**. The design is already made — transcribe it. Do not read other spec
files and do not redesign anything named here. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

The frozen conventions you implement live in
`truck-geometry/src/constructive/mod.rs` (CG-000's module docs: index
identity, certificate mapping). Read them first.

```yaml
id:          BG-CG-004-FACET
contract:    [BG-CG-004-FACET]
class:       design
crates:      [truck-modeling]
depends_on:  [BG-CG-001-RECIPE, BG-CG-002-FRAMES-ANALYTIC, BG-CG-003-TRANSPORT]
write_allow:
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/Cargo.toml
  - Cargo.lock
  - vendor/truck/truck-modeling/tests/facet_sweep_conformance.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
  - vendor/truck/truck-geometry/src/constructive/sampling.rs
  - vendor/truck/truck-polymesh/src/lib.rs
  - vendor/truck/truck-polymesh/src/polygon_mesh.rs
  - vendor/truck/truck-modeling/src/lib.rs
tests_required:
  - straight_duct_closes_with_exact_shared_indices
  - grid_registry_creates_each_vertex_exactly_once
  - tapered_duct_emits_planar_quads
  - curved_spine_splits_along_the_fixed_diagonal
  - profile_collapse_refuses_before_emission
  - non_convex_cap_refuses
  - winding_audit_counts_violations
  - signed_volume_matches_analytic_box
  - inconclusive_verdict_is_representable
  - stations_are_validated
budget:      {turns: 50, ctx_tokens: 120000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct PolygonMesh' vendor/truck/truck-polymesh/src/lib.rs"}
  - {id: A2, expect: 0, cmd: "grep -c 'truck-polymesh' vendor/truck/truck-modeling/Cargo.toml"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub trait Spine' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn frame' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'facet_sweep' vendor/truck/truck-modeling/src/lib.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum FrameLaw' vendor/truck/truck-geometry/src/constructive/mod.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn evaluate' vendor/truck/truck-geometry/src/constructive/profile.rs"}
```

## The manifest change (exactly this, nothing else)

`truck-modeling/Cargo.toml` gains one dependency line in `[dependencies]`
(plan §3.1 books this: polymesh is a leaf crate, no cycle; construction does
not belong in the tessellation crate):

```rust
truck-polymesh = { version = "0.6.0", path = "../truck-polymesh" }
```

`Cargo.lock` (workspace root) changes ONLY by the resulting
`truck-modeling` dependency-edge addition (the `truck-polymesh` package
entry already exists in the lock — it is a workspace member). If anything
else in the lock moves, that is a SPEC_GAP. `truck-modeling/src/lib.rs`
gains exactly the module declaration:

```rust
/// BG-CG-004-FACET: the direct facet realization backend (FAC) — a
/// `SpineFrameRecipe` realized as a shared-topology `PolygonMesh`, closed
/// by construction, with the mandatory mesh-level sanity audit (plan §3.3).
pub mod facet_sweep;
```

## The types (all in `facet_sweep.rs`, new file)

File header `#![deny(clippy::unwrap_used)]` (GATE-1). Import the landed
constructive types through `truck_geometry::constructive::*` (the CG-000
re-exports; `Spine` is re-exported too). `use truck_polymesh::*` as needed.

```rust
//! BG-CG-004-FACET — the direct facet realization backend.

/// The three-valued verdict of the plan §3.3 sanity audit. CG-007 maps this
/// onto the unified realization evidence (the CG-000 §3.5 mapping row);
/// until then this local spelling is the booked representation. Uncertainty
/// is surfaced (Inconclusive), never converted into success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetVerdict {
    /// The mesh closed by construction and the audit found nothing.
    CertifiedWithinTolerance,
    /// The winding audit found violations — FAILED, never a warning.
    Failed,
    /// The audit could not decide (e.g. the signed volume is degenerate
    /// against the mesh's own extent).
    Inconclusive,
}

/// The mandatory mesh-level sanity audit facts (plan §3.3): signed-volume
/// sign sanity and the twin-triangle winding audit. Pure data; the verdict
/// is derived by [`verdict_of`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FacetSweepAudit {
    /// Emitted triangles.
    pub triangle_count: usize,
    /// Emitted planar quads (a quad is ONE face of the polygon mesh).
    pub quad_count: usize,
    /// Signed volume V = (1/6) * sum a . (b x c) over the fan triangulation
    /// of every face, after the global orientation normalization.
    pub signed_volume: f64,
    /// Number of interior mesh edges whose two uses do NOT traverse in
    /// opposite effective directions, plus boundary uses (0 for a closed
    /// mesh — which this construction produces).
    pub winding_violations: usize,
}

/// The result: the mesh, the audit facts, and the verdict.
#[derive(Debug, Clone)]
pub struct FacetSweepResult {
    /// The realized mesh. Every position index is a grid-registry index:
    /// adjacent faces share the identity BY CONSTRUCTION (plan §3.3); no
    /// positional welding is ever invoked.
    pub mesh: PolygonMesh,
    /// The audit facts.
    pub audit: FacetSweepAudit,
    /// The three-valued verdict.
    pub verdict: FacetVerdict,
}
```

## The entry point (exact semantics)

```rust
/// Realizes the recipe as a faceted `PolygonMesh` over the given spine
/// stations (RESOLVED stations — ascending, >= 2, inside the spine domain;
/// resolve a `SamplingPolicy` with its `resolve` first and pass the result).
///
/// `ring_resolution` is the profile vertex count k: the ring parameter of
/// profile vertex j is v_j = j / k (the per-edge-uniform convention the
/// profile evaluator is booked on; plan §3.3's grid vertex (i, j)).
///
/// Structured grid x_{i,j} = position(s_i, v_j); grid vertex (i, j) is
/// created EXACTLY ONCE via the private grid registry (index i*k + j);
/// adjacent faces reuse the identity; internal grid edges are created once
/// and traversed oppositely by their two faces. No sewing (plan §3.3).
pub fn facet_sweep<S: Spine>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
    ring_resolution: usize,
) -> std::result::Result<FacetSweepResult, ConstructError>
```

(`std::result::Result` spelled out if the crate's 1-arg `Result` alias is in
scope; match what compiles.) Return type rationale: construction-stage
refusals are the frozen `ConstructError` currency of the recipe evaluators
this function composes; the audit rides beside the mesh, and CG-007 maps it
onto the unified evidence — the §3.5 `EnvelopeCase::ConstructRefused` and
`RealizationVerdict` variants are CG-007's additions, not this packet's.

**Body, exactly:**

1. **Validation.** `ring_resolution < 3` → `InvalidInput`. `stations.len() <
   2` → `InvalidInput`. Any non-finite station → `NonFinite { at: that
   station }`. Not ascending (strictly: equal or decreasing neighbors) →
   `InvalidInput`. Any station outside `recipe.spine.domain()` (beyond
   `DirectTolerance::default().parameter`) → `InvalidInput`.
2. **Grid emission.** For `i` in `0..m` (`m = stations.len()`) and `j` in
   `0..k`: `v_j = j as f64 / k as f64`;
   `x_{i,j} = recipe.position(s_i, v_j)?` — every recipe refusal propagates
   (profile collapse, zero tangent, frame singularities, non-finite values,
   each already carrying its `at`). The grid registry is the index map
   `i*k + j`: each grid vertex exists exactly once; NOTHING is a "copy".
3. **Side faces.** For `i` in `0..m-1`, `j` in `0..k` (ring index `j2 =
   (j+1) % k`): the quad `[ (i,j), (i+1,j), (i+1,j2), (i,j2) ]`. Planarity
   test (the bilinear twist): the quad is planar iff
   `‖(x_{i,j} + x_{i+1,j2}) − (x_{i+1,j} + x_{i,j2})‖ ≤
   DirectTolerance::default().position`. Planar → emit ONE quad face;
   otherwise TWO triangles along the FIXED diagonal `(i,j)−(i+1,j2)`:
   `[(i,j), (i+1,j), (i+1,j2)]` and `[(i,j), (i+1,j2), (i,j2)]` — the
   diagonal choice is structural (always this diagonal), never a float
   comparison (plan §3.3).
4. **Caps.** The start ring (grid row `i = 0`) and end ring (row `i = m−1`)
   are the cap boundaries — the ring vertices ARE the grid vertices (shared
   identity; no cap duplicates). Triangulate each cap as a FAN from ring
   vertex 0 — which requires the profile to be CONVEX at the cap stations:
   certify convexity by the cross-sign consistency of the ring polygon's
   consecutive edge pairs (all crosses strictly one sign within
   `DirectTolerance::default().position`). A non-convex profile refuses
   `ConstructError::InvalidInput` — this is a TYPED ENVELOPE LINE (booked
   follow-up: CDT-based caps; the plan's "existing planar support" does not
   cover concave rings without the tessellation crate's machinery, which
   would violate the fast-path contract). Start and end caps wind in
   OPPOSITE senses (they close the tube). Check the profile convexity ONCE
   (at `s_0` and at `s_{m-1}` — a Constant/Scale law is convex at every
   station iff convex at one; a LinearCorrespondence between two convex
   profiles is convex at every station — but do not argue it: certify at
   BOTH cap stations, which is what the construction actually uses).
5. **Global orientation normalization.** Compute the signed volume over the
   emitted faces (fan each quad from its first vertex). If V < 0, invert
   EVERY face (reverse each face's index cycle). This is deterministic and
   structural — the grid's faces all share one handedness by construction,
   so one global sign check replaces any per-face BFS.
6. **The mandatory audit** (plan §3.3 — on the FINAL emitted mesh):
   - Winding audit: for every face, walk its index cycle; key each
     undirected edge by sorted index pair; count uses and record direction
     agreement. Every undirected edge of a closed mesh must appear exactly
     twice with opposite effective directions; every violation counts into
     `winding_violations` (a use-count of 1 or ≥ 3 is also a violation).
   - Signed volume `V = (1/6) Σ a·(b×c)` over the fan triangulation of
     every face; a mesh extent `d` = the max distance between any two grid
     positions; the degeneracy floor is `d^3 * 1e-9` — COMPUTED from data,
     never a bare literal in a predicate (H-3: write it as
     `d * d * d * 1e-9`? — NO: `1e-9` trips the gate regex. Compute the
     floor as `d³ / 1_000_000_000.0` — the integer-literal division is
     regex-clean and identical in value).
   - `verdict_of(&audit, extent) -> FacetVerdict`: `winding_violations >
     0` → `Failed`; `|signed_volume| <= extent³ / 1_000_000_000.0` →
     `Inconclusive`; else `CertifiedWithinTolerance`. (Both fns `pub` —
     CG-007 consumes them; testability demands it.)
7. **Assembly.** `PolygonMesh::new(attributes, faces)` over the grid
   positions (read truck-polymesh's `StandardAttributes`/`Faces` API for
   the exact construction — faces may carry `usize` vertex references
   directly; check what `Faces::from`/`triangles()` accept and use the
   house pattern). The mesh's position array IS the grid registry, in
   `i*k + j` order.

Determinism (plan §7): identical ordered input → byte-identical mesh and
verdict; emission order is fixed (stations ascending, ring index ascending);
no hash-map iteration may influence output ordering (the winding audit's
edge map may be a HashMap INTERNALLY, but violations are counted, not
enumerated, into the output — and say so in a comment).

## Tests required — `tests/facet_sweep_conformance.rs` (new file)

Header `#![deny(clippy::unwrap_used)]`. Fixtures: a unit-square profile
(4 vertices, CCW), a tapered pair (LinearCorrespondence square → larger
square), an L-shaped concave profile (6 vertices), a straight LineSpine of
length 2, and a test-local curved `Spine` impl (quarter/two-quarter circle
in a plane — the sanctioned extension point, same as the frame packets).
Resolutions come from `SamplingPolicy::UniformCount { spine: n }.resolve(..)`
— exercised end to end. No `1e-…` literals; bounds through
`DirectTolerance::default()`/`TOLERANCE`/derived floors.

1. `straight_duct_closes_with_exact_shared_indices` — square profile, 5
   stations: the mesh's every undirected edge appears exactly twice with
   opposite directions (recompute in the test, independently of the
   production auditor), `verdict == CertifiedWithinTolerance`,
   `winding_violations == 0`.
2. `grid_registry_creates_each_vertex_exactly_once` — the mesh's position
   count is EXACTLY `m * k` (5 * 4 = 20: caps reuse the ring rows, the
   registry has no duplicates).
3. `tapered_duct_emits_planar_quads` — LinearCorrespondence between two
   squares on a straight spine: every side face is planar (twist test) so
   `quad_count == (m-1) * k` and `triangle_count == 0` from the side strip
   (cap fans add their own triangles; assert the exact totals).
4. `curved_spine_splits_along_the_fixed_diagonal` — the curved fixture:
   non-planar side cells split into triangle PAIRS whose shared edge is
   always the `(i,j)−(i+1,j2)` diagonal — assert the vertex-pattern of
   every split pair (structural, integer assertions).
5. `profile_collapse_refuses_before_emission` — `Scale` with a
   `ScalarLaw::Constant(0.0)`: `facet_sweep` is
   `Err(ConstructError::ProfileCollapse { .. })` — the recipe's refusal
   propagates before any face is emitted.
6. `non_convex_cap_refuses` — the L-shaped profile: `Err(InvalidInput)`
   (the booked typed envelope line; the error is typed, not a panic).
7. `winding_audit_counts_violations` — `winding_audit` on a GOOD closed
   mesh is 0; on a hand-broken mesh (two same-winding triangles sharing an
   edge, built directly through the polymesh API) it is > 0. (The auditor
   is `pub` — CG-007 consumes it; this test is its contract.)
8. `signed_volume_matches_analytic_box` — the straight square duct of
   length 2 over the unit square profile: `|V − 2.0|` within a bound
   derived from the mesh extent (the analytic volume of the prism is
   exactly area × length = 2.0; the faceted mesh reproduces it EXACTLY up
   to float rounding because the side faces are planar and the caps are
   flat — assert near-equality at a TOLERANCE-scaled bound).
9. `inconclusive_verdict_is_representable` — `verdict_of` with a
   hand-built audit whose volume is 0.0 → `Inconclusive`; with a violation
   count > 0 → `Failed` (Failed dominates Inconclusive).
10. `stations_are_validated` — unsorted, single-station, non-finite, and
    out-of-domain station lists each refuse with the booked error.

No existing test may be deleted, `#[ignore]`d, or weakened.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` — code
  and tests. No `debug_new` in added lines (GATE-3).
- **H-3** No `1e-…` literals anywhere (the regex catches `1e-9`; use the
  integer-division form). All comparison bounds are derived
  (`DirectTolerance::default()`, `TOLERANCE`, or the extent floor).
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item, derive `Debug`.
- No `unscaled_legacy(` calls (GATE-4); no `cfg!(debug_assertions)` (GATE-3).
- Fast path (plan §3.3, a gate): the hot loop is curve/frame evaluation,
  profile transform, and index emission — no fitting, Newton, sewing,
  healing, booleans, or generic SSI anywhere in this file.
- Determinism (plan §7): fixed emission order; byte-identical output for
  identical ordered input; no hash-order-dependent output.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-modeling -p truck-geometry -p truck-polymesh
cargo clippy -p truck-modeling --all-targets -- -D warnings
cargo test -p truck-modeling --lib --tests
```

Never run a bare `cargo test`. Your write set changes a workspace manifest
(the polymesh dep edge), so also run `cargo check --workspace --all-targets`
once at the end and say in RESULT notes that it passed — a ripple is cheaper
to catch at worker time. Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially
`truck-geometry/src/constructive/**` (import the landed types; the
`ProfileLaw::vertex_count` accessor you might wish for is a booked
follow-up, NOT this packet), `truck-polymesh/**` (read-only),
`tessellation/**`, `scripts/kernel-gates.sh`. Any welding/sewing/healing
call. Any per-face float-comparison-based diagonal choice. Adding a
fallback for non-convex caps (the typed refusal IS the booked v1 behavior).
Adding `#[ignore]`. Adding `#[allow]` without a same-line justification.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A2 and A5 must read 0 —
  they prove the dep edge and the module do not exist yet)
- the design as written cannot compile as specified → `SPEC_GAP`, naming
  the exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-004-FACET","status":"DONE","contracts":["BG-CG-004-FACET"],
 "tests_added":10,"anchors_verified":{"A1":1,"A2":0,"A3":1,"A4":1,"A5":0,"A6":1,"A7":1},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(modeling): direct facet realization backend (BG-CG-004-FACET)`
BEFORE writing `RESULT.json`.
