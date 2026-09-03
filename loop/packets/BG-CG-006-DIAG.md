# WORK PACKET BG-CG-006-DIAG — manifold diagnostics: aggregate, do not duplicate

You are landing the manifold-diagnostics deliverable of the constructive
geometry program (plan §3.6): one actionable aggregate over the substrate that
already exists in `truck-topology`. The design is already made — transcribe
it. Do not read other spec files and do not redesign anything named here. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          BG-CG-006-DIAG
contract:    [BG-CG-006-DIAG]
class:       mechanical
crates:      [truck-topology]
depends_on:  [BG-CG-000-CONTRACT]
write_allow:
  - vendor/truck/truck-topology/src/manifold.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/tests/manifold_diag.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-topology/src/shell.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/invariants/vertex_link.rs
  - vendor/truck/truck-base/src/id.rs
tests_required:
  - closed_cube_is_closed_manifold
  - open_box_has_boundary_path_links
  - two_sheets_pinch_is_nonmanifold_at_vertex
  - irregular_shell_lists_over_shared_edge
  - inverted_face_produces_orientation_conflicts
  - parity_assignment_is_some_for_oriented_shell
  - diagnostics_output_order_is_deterministic
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn shell_condition' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn connected_components' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn singular_vertices' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn face_adjacency' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub const fn absolute_boundaries' vendor/truck/truck-topology/src/face.rs"}
  - {id: A6, expect: 0, cmd: "grep -c 'manifold' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub mod vertex_link' vendor/truck/truck-topology/src/invariants/mod.rs"}
```

## The one existing file you may touch

`vendor/truck/truck-topology/src/lib.rs` gains exactly the module declaration
(after the `invariants` block is a fine place; nothing existing moves):

```rust
/// BG-CG-006-DIAG: the actionable manifold-diagnostics aggregate over the
/// shell substrate (`shell_condition`, `connected_components`,
/// `extract_boundaries`, `singular_vertices`, `face_adjacency`). Analysis
/// only; it never repairs.
pub mod manifold;
```

Do not touch the prelude, the lint attributes, or any other line.

## Context — what this is (one paragraph)

The constructive geometry program certifies realization output; a FAILED or
INCONCLUSIVE verdict is useless unless the next consumer can see WHY. The
substrate already answers the primitive questions (`shell_condition()`,
`connected_components()`, `extract_boundaries()`, `singular_vertices()`,
`face_adjacency()`, and the binary `invariants::vertex_link::check`), but a
caller must know which to call and must reconcile their outputs. This packet
lands ONE aggregate that answers them together, with per-entity diagnostics
an agent can act on. **Analysis only — no repair, no mutation.** (The
outward-sign/signed-volume check is deliberately NOT here: `CalcVolume` lives
in truck-meshalgo, which depends on this crate — putting it here would be a
dependency cycle, and the plan books signed-volume sign sanity in the FAC
audit (§3.3) anyway. Do not add it.)

## The landed substrate (verified — do not re-derive)

In `truck-topology/src/shell.rs`, all on `Shell<P, C, S>`:

- `shell_condition(&self) -> ShellCondition` — `Irregular | Regular |
  Oriented | Closed` (half-edge model; `Closed` = every edge shared by exactly
  two compatibly-oriented faces).
- `connected_components(&self) -> Vec<Shell<P, C, S>>`.
- `extract_boundaries(&self) -> Vec<Wire<P, C>>`.
- `singular_vertices(&self) -> Vec<Vertex<P>>` — vertices whose edge-wise
  adjacency (computed over ABSOLUTE boundaries) is disconnected.
- `face_adjacency(&self) -> FaceAdjacencyMap` — `type FaceAdjacencyMap<'a,
  P, C, S> = HashMap<&'a Face<P, C, S>, Vec<AdjacentFace<'a, P, C, S>>>`.
- `edge_iter`, `vertex_iter`, `face_iter` — deterministic, Vec-backed.

In `truck-topology/src/face.rs`, on `Face<P, C, S>`:

- `boundaries(&self) -> Vec<Wire<P, C>>` — **orientation-ADJUSTED**: when
  `orientation == false` every wire is returned inverted.
- `absolute_boundaries(&self) -> &Vec<Wire<P, C>>` — **STORED verbatim**
  (`pub const fn`). The session-38 naming trap is live here: always check
  WHICH accessor a wire came from before reasoning about direction.
- `orientation(&self) -> bool`, `id(&self) -> FaceID<S>`,
  `invert(&mut self)` (NOT for diagnostics use — analysis only).

In `truck-base/src/id.rs`: `pub struct ID<T>(usize, PhantomData<T>)` —
`Copy + Hash + Eq`, **NO `Ord`**. All ID types (`VertexID<P> = ID<P>`,
`EdgeID<C> = ID<C>`, `FaceID<S> = ID<S>`) are pointer-derived; you cannot
sort by them directly. Determinism rule below handles this.

## The types (all in `manifold.rs`, new file)

```rust
//! BG-CG-006-DIAG — the actionable manifold-diagnostics aggregate.
//!
//! Aggregates the shell substrate into one answer with per-entity detail:
//! what is wrong, where, and in which classification. Analysis only —
//! nothing here mutates its input, and no repair is offered (a separate
//! explicit op may apply a parity assignment later; the plan books it).

use crate::shell::{Shell, ShellCondition};
use crate::{EdgeID, FaceID, VertexID};
use std::collections::HashMap;

/// How one vertex's link is shaped (plan §3.6, normative):
/// closed 2-manifold ⇒ the link is one cycle; manifold-with-boundary ⇒ one
/// path; two sheets touching at the vertex (or any branching) ⇒ nonmanifold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexLinkClass {
    /// The link is exactly one cycle — a closed 2-manifold at this vertex.
    ClosedCycle,
    /// The link is exactly one path — a manifold boundary at this vertex.
    BoundaryPath,
    /// The link is disconnected or has a vertex of degree ≠ 2 — sheets
    /// touch or branch here.
    NonManifold,
    /// No edge uses this vertex (degenerate).
    Isolated,
}

/// One vertex's diagnosis.
#[derive(Debug, Clone, PartialEq)]
pub struct VertexDiagnostic<P> {
    /// Which vertex.
    pub vertex: VertexID<P>,
    /// How its link is shaped.
    pub classification: VertexLinkClass,
}

/// How one edge is irregular. (An edge used by exactly two faces with
/// opposite effective directions is regular and gets NO entry; boundary
/// edges go to `ManifoldDiagnostics::boundary_edges`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeIrregularity {
    /// More than two face-uses traverse the edge; `use_count` is the total.
    OverShared {
        /// The number of face-uses.
        use_count: usize,
    },
    /// The same face uses the edge twice (a fin).
    DoublyUsedByOneFace,
    /// Exactly two faces share the edge but traverse it in the SAME
    /// effective direction — an orientation conflict on this edge.
    SameDirectionUses,
}

/// One edge's diagnosis.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeDiagnostic<P, C> {
    /// Which edge.
    pub edge: EdgeID<C>,
    /// How it is irregular.
    pub classification: EdgeIrregularity,
}

/// One conflicting edge use pair, named by entity (plan §3.6: "the
/// conflicting edges/faces").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationConflict<P, C, S> {
    /// The shared edge.
    pub edge: EdgeID<C>,
    /// One incident face.
    pub face_a: FaceID<S>,
    /// The other incident face.
    pub face_b: FaceID<S>,
}

/// The whole answer for one shell. Every field is derived from the substrate;
/// nothing here re-derives what `Shell` already knows.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldDiagnostics<P, C, S> {
    /// The substrate's own half-edge verdict, verbatim.
    pub shell_condition: ShellCondition,
    /// How many pieces the shell is in (`connected_components().len()`).
    pub connected_components: usize,
    /// Edges used by exactly one face, in deterministic order.
    pub boundary_edges: Vec<EdgeID<C>>,
    /// Per-edge irregularities, in deterministic order.
    pub irregular_edges: Vec<EdgeDiagnostic<P, C>>,
    /// Per-vertex link classifications, in deterministic order. Every vertex
    /// of the shell appears exactly once (not only singular ones) — a caller
    /// filtering for trouble filters on the classification.
    pub singular_vertices: Vec<VertexDiagnostic<P>>,
    /// Every orientation conflict found by the parity walk, in deterministic
    /// order. Empty iff the shell's face orientations are mutually
    /// consistent.
    pub orientation_conflicts: Vec<OrientationConflict<P, C, S>>,
}
```

## The entry point and the algorithms (exact, pre-decided)

```rust
/// Diagnoses a shell: the aggregate of the substrate with per-entity detail.
/// Never panics, never mutates, never repairs.
pub fn diagnose<P, C, S>(shell: &Shell<P, C, S>) -> ManifoldDiagnostics<P, C, S>
```

**Determinism rule (plan §7 is a gate):** `ID<T>` carries no `Ord`, and no
observable output ordering may derive from hash-map iteration. Build one
`HashMap<VertexID<P>, usize>` of vertex ordinals from
`shell.vertex_iter()` and one `HashMap<EdgeID<C>, usize>` from
`shell.edge_iter()` (first appearance = the ordinal). Every output vector is
sorted by these ordinals (edges by edge ordinal; vertices by vertex ordinal;
orientation conflicts by (edge ordinal, face ordinal) where face ordinals
come from `shell.face_iter()` the same way). An entity absent from the maps
cannot exist in a well-formed shell; if you ever meet one, that is a
SPEC_GAP — report it, do not improvise an ordering.

**Edge census.** Iterate `shell.face_iter()`; for each face iterate
`face.absolute_boundaries()` wires and their edge uses. Count uses per
`EdgeID<C>` and remember, per use, (face ordinal, effective direction), where
the **effective direction** of a use is the stored (absolute) direction XOR
`face.orientation()` — this is the accessor discipline: structure from
`absolute_boundaries`, direction corrected by `orientation`. Then:

- use count == 1 → `boundary_edges`.
- use count == 2 with both uses from the SAME face → `DoublyUsedByOneFace`.
- use count == 2 from different faces with EQUAL effective direction →
  `SameDirectionUses` AND an `OrientationConflict { edge, face_a, face_b }`
  (face_a = lower face ordinal).
- use count >= 3 → `OverShared { use_count }`.

**Vertex links.** For every vertex occurrence in every face's
orientation-ADJUSTED outer wires (`face.boundaries()` — the effective
traversal; a face occurrence of v contributes a link edge between v's
predecessor and successor in that wire's cyclic order), collect link edges.
Classify the link multigraph at v: any link-vertex with degree ≠ 2 →
`NonManifold`; all degrees == 2 and exactly one connected component →
`ClosedCycle`; exactly two degree-1 vertices and exactly one component (a
path) → `BoundaryPath`; otherwise (disconnected) → `NonManifold`. A vertex
with no incident edge uses → `Isolated`. Every shell vertex gets exactly one
entry (deterministic order per the rule above). Note this deliberately
classifies ALL vertices, a superset of the substrate's
`singular_vertices()` — the substrate answers "which are broken", the
aggregate answers "what is each vertex".

**Parity walk** (the third deliverable, separate function):

```rust
/// A consistent orientation parity assignment (face ordinal -> flip flag:
/// `true` = the face's stored orientation is already consistent), or `None`
/// when the shell's orientations conflict. BFS over `face_adjacency()`
/// starting from the lowest-ordinal face assigned `true`; crossing a shared
/// edge to the next face requires opposite effective edge directions.
/// Deterministic; analysis only — applying an assignment is somebody else's
/// explicit op.
pub fn orientation_parity<P, C, S>(
    shell: &Shell<P, C, S>,
) -> Option<HashMap<FaceID<S>, bool>>
```

The BFS is seeded deterministically (lowest face ordinal from
`face_iter()`), neighbors are visited in the deterministic face/edge ordinal
order, and the effective-direction comparison uses the same accessor
discipline as the census. `diagnose`'s `orientation_conflicts` and
`orientation_parity`'s `None` must AGREE on every input (the tests pin one
direction of this).

`ShellCondition`, `connected_components` count: call the substrate, copy the
answers. No duplication of their logic.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, and no
  out-of-range indexing in added lines. (Note: the substrate's own
  `singular_vertices` indexes `wire[0]` — pre-existing, grandfathered; your
  added code may NOT copy that pattern.)
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item, derive `Debug` on
  every public type.
- **GATE-1**: the new `manifold.rs` starts with `#![deny(clippy::unwrap_used)]`
  (the new test file gets the same header).
- **H-3**: no `1e-…` literals anywhere (nothing here needs tolerances at all —
  the diagnostics are purely combinatorial; do not invent float comparisons).
- **GATE-3**: no `debug_new`, no `cfg!(debug_assertions)`.
- No `unscaled_legacy(` calls (GATE-4).
- Do NOT edit anything under `src/invariants/` — their checkers stay
  byte-identical; the aggregate classifies richer than the binary checker by
  design, and its docs point at `invariants::vertex_link::check` as the
  certification-grade binary form.

## Tests required — `tests/manifold_diag.rs` (new file)

All fixtures are pure combinatorial topology with `()` payloads (line edges,
no curves, no tessellation, no geometry) — debug-safe by construction. Build
fixtures through `Face::new` / `Shell::from` exactly like the crate's own
doctests. **Every fixture's premise is machine-checked first** (e.g. assert
the cube fixture's `shell_condition() == ShellCondition::Closed` before
asserting anything about its diagnostics) — a diagnostic suite built on an
unverified fixture is noise.

1. `closed_cube_is_closed_manifold` — a 6-face outward-oriented cube:
   `shell_condition == Closed`, `connected_components == 1`,
   `boundary_edges` empty, `orientation_conflicts` empty, all 8 vertices
   `ClosedCycle`, `irregular_edges` empty.
2. `open_box_has_boundary_path_links` — the cube minus the top face: 4
   boundary edges (the top rim), the 4 top vertices `BoundaryPath`, the 4
   bottom vertices `ClosedCycle`.
3. `two_sheets_pinch_is_nonmanifold_at_vertex` — two triangles sharing
   exactly one vertex: the shared vertex `NonManifold`, the other four
   `BoundaryPath`, and 6 boundary edges.
4. `irregular_shell_lists_over_shared_edge` — the 3-face fan from the
   `ShellCondition::Irregular` doctest: the shared edge appears in
   `irregular_edges` as `OverShared { use_count: 3 }`.
5. `inverted_face_produces_orientation_conflicts` — the cube with one face
   inverted (`.invert()` before assembling the shell): `orientation_conflicts`
   has exactly 4 entries (that face against each neighbor),
   `orientation_parity` is `None`, and every conflict's `face_a`/`face_b`
   names the inverted face on one side.
6. `parity_assignment_is_some_for_oriented_shell` — the clean cube:
   `orientation_parity` is `Some` with all 6 faces present; the seed face's
   entry is `true` by construction. Also: `orientation_conflicts` empty AND
   parity `Some` agree.
7. `diagnostics_output_order_is_deterministic` — build a shell whose
   natural iteration order does NOT match vertex creation order (e.g. faces
   assembled in a scrambled order), run `diagnose` twice on clones, assert
   the two answers are equal AND each vector is sorted by the shell's
   iteration ordinals (recompute the ordinals in the test the same way the
   implementation does).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets -- -D warnings
cargo test -p truck-topology --lib --tests
```

Never run a bare `cargo test`. The module is purely additive over the
workspace; the verifier runs the workspace gates authoritatively. Send cargo
output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `src/invariants/**`,
`src/shell.rs`, `src/face.rs`, `Cargo.toml`, `Cargo.lock`,
`scripts/kernel-gates.sh`. Adding any repair/mutation API (this packet is
analysis only). Adding the signed-volume check (wrong crate — dependency
cycle; booked in the FAC audit). Adding `#[ignore]`. Adding `#[allow]`
without a same-line justification. Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A6 must read 0 — it proves
  the module does not exist yet)
- the design as written cannot compile as specified → `SPEC_GAP`, naming the
  exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-006-DIAG","status":"DONE","contracts":["BG-CG-006-DIAG"],
 "tests_added":7,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1,"A5":1,"A6":0,"A7":1},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(topology): actionable manifold diagnostics aggregate (BG-CG-006-DIAG)`
BEFORE writing `RESULT.json`.
