# WORK PACKET BG-CG-005-LEDGER — the edge-sample ledger: integer identity for shared boundary positions

You are landing the meshalgo ledger deliverable of the constructive geometry
program (plan §3.4, CG-005): the `EdgeSampleLedger` and a NEW parallel entry
point that returns it beside the unchanged production outcome. The frozen
index-identity convention it implements lives in
`truck-geometry/src/constructive/mod.rs` (CG-000's module docs — read them,
do not restate them differently). The design is already made — transcribe it.
Do not read other spec files and do not redesign anything named here. If
something you need is genuinely missing, that is a SPEC_GAP (see "Stop
conditions"): you stop and report, you do not research it.

```yaml
id:          BG-CG-005-LEDGER
contract:    [BG-CG-005-LEDGER]
class:       mechanical
crates:      [truck-meshalgo]
depends_on:  [BG-CG-000-CONTRACT]
write_allow:
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation_with_ledger.rs
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-meshalgo/tests/ledger_identity.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-meshalgo/src/tessellation/mod.rs
  - vendor/truck/truck-topology/src/compress.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
tests_required:
  - ledger_covers_every_unique_edge_once
  - shared_edge_identity_as_integers
  - ledger_matches_production_sampling_bit_for_bit
  - closed_shell_has_no_boundary_edge_uses
  - ledger_outcome_equals_unchanged_entry_outcome
budget:      {turns: 45, ctx_tokens: 110000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn cshell_tessellation_inner' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'fn robust_triangulation_with_schema_outcome' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct CompressedFace' vendor/truck/truck-topology/src/compress.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'triangulation_with_ledger' vendor/truck/truck-meshalgo/src/tessellation/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub struct MeshedShellOutcome' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
  - {id: A6, expect: 8, cmd: "grep -c 'PolylineCurve::from_curve' vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs"}
```

## The two lines you add to `tessellation/mod.rs`

With the other private module declarations (near where the tessellation
submodules are declared):

```rust
mod triangulation_with_ledger;
pub use triangulation_with_ledger::{EdgeSampleLedger, EdgeSampleLedgerSet, triangulation_with_ledger};
```

Nothing else in mod.rs moves. **The existing entry points, their signatures,
and their behavior stay BIT-IDENTICAL** — the V5 identity guard is law here;
the ledger is a strictly parallel path that calls them.

## Context — what already exists (verified; do not re-derive)

The production robust path (`cshell_tessellation_inner`,
`triangulation.rs:1617`) already samples every UNIQUE compressed edge
exactly once (`tessellate_edge`, via `PolylineCurve::from_curve(&curve,
range, tol)` — 8 call sites across the file's paths), meshed faces reference
those shared samples, and `MeshedShellOutcome` returns the shell plus
per-face failure evidence. What it does NOT do is hand the caller the
(edge identity → sample ordinal → position index) map that lets a consumer
prove watertightness BY CONSTRUCTION instead of by positional welding
(`put_together_same_attrs`). That map is your deliverable.

`CompressedShell` (truck-topology/src/compress.rs) is the identity
vocabulary: `CompressedEdgeIndex { index, orientation }` per boundary use;
`CompressedFace { boundaries: Vec<Vec<CompressedEdgeIndex>>, orientation,
surface, provenance }`. **The compressed edge INDEX is the entity identity
of this representation** — the plan §3.4 convention is stated over
`EdgeID<Curve>`; here that identity is the compressed index (booked
spelling adjustment; the CG-000 module doc's convention applies verbatim
once you substitute the identity type).

## The types (all in `triangulation_with_ledger.rs`, new file)

File header `#![deny(clippy::unwrap_used)]` (GATE-1).

```rust
//! BG-CG-005-LEDGER — the edge-sample ledger: a mesh position index is a
//! pure function of (entity identity, sample ordinal), never of coordinates
//! (the frozen convention, CG-000 module docs; plan §3.4).

use crate::tessellation::{MeshedShellOutcome, ...};  // extend as needed
use truck_topology::compress::{CompressedShell, CompressedEdgeIndex};
use truck_base::cgmath64::*;

/// One unique edge's sample record: the sampled parameters, and the global
/// position indices of the sampled positions. A reversed edge USE consumes
/// the same integer sequence reversed — no second sampling, ever.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSampleLedger {
    /// The compressed edge index — the entity identity in this
    /// representation (the plan's `EdgeID<Curve>`, booked spelling).
    pub edge: usize,
    /// The sampled parameters, ascending.
    pub parameters: Vec<f64>,
    /// The global position index of each sampled position, aligned with
    /// `parameters`.
    pub position_indices: Vec<usize>,
}

/// The whole ledger for one shell: one entry per unique compressed edge,
/// plus the global position table the indices reference.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSampleLedgerSet {
    /// One entry per unique edge, ordered by edge index ascending.
    pub entries: Vec<EdgeSampleLedger>,
    /// The global position table. Positions are interned ONCE across the
    /// whole shell: two sampled positions that are exactly equal (f64 `==`
    /// on all three components) share one index. Nothing here merges
    /// near-equal positions — exact equality only; there is no welding.
    pub positions: Vec<Point3>,
}
```

## The entry point (exact semantics)

```rust
/// Runs the UNCHANGED robust outcome path and, beside it, returns the
/// edge-sample ledger the watertightness invariant is stated over.
pub fn triangulation_with_ledger<C, S>(
    shell: &CompressedShell<Point3, C, S>,
    tol: f64,
    lattice_of: impl Fn(&S) -> CertifiedLattice + Parallelizable,
    schema_of: impl Fn(&S) -> formal::SupportSurfaceSchema + Parallelizable,
    curve_schema_of: impl Fn(&C) -> formal::CurveSchema + Parallelizable,
) -> (EdgeSampleLedgerSet, MeshedShellOutcome)
where
    C: PolylineableCurve,
    S: PreMeshableSurface,
```

Body, exactly:

1. `let outcome = shell.robust_triangulation_with_schema_outcome(tol,
   lattice_of, schema_of, curve_schema_of);` — the production path, called
   once, UNCHANGED. (Delegate through the existing trait; do not re-enter
   `cshell_tessellation_inner` by hand.)
2. The ledger, built independently of (1): for each unique compressed edge
   `e` at index `i` of `shell.edges`, sample the curve the SAME way the
   production path samples it — the SAME parameter division
   `PolylineCurve::from_curve` performs at the same `tol` over the same
   `curve.range_tuple()`. Read `from_curve`'s mechanism first (it is in
   truck-polymesh or the meshalgo polyline plumbing) and reproduce it
   exactly; if it goes through `ParameterDivision1D`, call that. The ledger
   records: the parameters (ascending), the sampled positions
   (`curve.subs(t)` per parameter — the SAME evaluation `from_curve` uses),
   and the position indices after interning into the shared position table
   (exact-equality interning; two edges that meet at a shared VERTEX
   position share that position's index — this is what makes the cap ring
   and the wall agree by construction).
3. Return `(ledger, outcome)`.

Edge cases, pre-decided: a compressed shell with zero edges yields an empty
ledger (and the outcome path still runs); a self-loop edge (both endpoint
vertices equal — the seam-circle class) is sampled like any other edge; a
curve whose parameter division yields a single point (degenerate) produces
a one-entry parameters/positions list — no special-casing, no refusal (the
ledger is a REPORT, not an opinion).

## The tests — `tests/ledger_identity.rs` (new file)

Header `#![deny(clippy::unwrap_used)]` (GATE-1). Fixture: a unit cube as a
`CompressedShell` with line curves — build it the way existing meshalgo
tests build compressed shells (read the test module of
`triangulation.rs`/existing meshalgo tests for the house fixture pattern;
if none exists, build the `CompressedShell` struct directly from
`truck-topology::compress` — its fields are public). Every fixture premise
is machine-checked first. No `1e-…` literals; tolerances via the `tol`
parameter passed to the entry (use `TOLERANCE` from truck-base).

1. `ledger_covers_every_unique_edge_once` — the cube's ledger has exactly
   12 entries (one per unique edge), indices strictly ascending, every
   entry's `parameters` ascending with ≥ 2 samples, `position_indices`
   aligned in length, every index in bounds of `positions`.
2. `shared_edge_identity_as_integers` — for EVERY pair of cube faces
   sharing a boundary edge index: the two faces' effective traversals of
   that edge are OPPOSITE (orientation of the `CompressedEdgeIndex` use XOR
   the face's `orientation`), and the ledger integer sequence consumed by
   the two uses satisfies `I(A, E) == reverse(I(B, E))` — asserted as
   INTEGERS (`position_indices`), never as coordinates. This is the plan §7
   gate.
3. `ledger_matches_production_sampling_bit_for_bit` — for every edge, the
   ledger's positions (via `position_indices`) equal the corresponding
   polyline positions of the meshed shell's edges (the outcome's shell
   carries `PolylineCurve` per unique edge) EXACTLY (f64 `==`, component
   by component) — proving the ledger sampled the same division, not a
   similar one.
4. `closed_shell_has_no_boundary_edge_uses` — every compressed edge index
   of the cube appears in exactly two face-uses; the effective traversals
   per index are one forward, one backward. (The watertightness premise.)
5. `ledger_outcome_equals_unchanged_entry_outcome` —
   `triangulation_with_ledger(...)`'s outcome compares EQUAL (whole
   `MeshedShellOutcome`: shell, every failure/diagnosis/band vector) to
   `robust_triangulation_with_schema_outcome(...)` on the same input with
   the same resolvers — the "existing entry points bit-identical" gate,
   asserted at the API level.

No existing test may be deleted, `#[ignore]`d, or weakened.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` — in
  code and tests. **No `debug_new` in added lines** (GATE-3) — the
  pre-existing path uses it; you must not.
- **H-3** No `1e-…` literals anywhere.
- The crate warns `missing_docs, missing_debug_implementations` and denies
  warnings in release: doc-comment every public item, derive `Debug`.
- No `unscaled_legacy(` calls (GATE-4); no `cfg!(debug_assertions)`
  semantics (GATE-3).
- Determinism (plan §7): the ledger is built in a fixed order (edge index
  ascending), the position table interns in first-appearance order, and no
  output ordering derives from hash-map iteration. If you use a HashMap for
  interning, the OUTPUT order must still be deterministic — sort or key by
  index, and say which in a comment.
- This packet does NOT change how faces are meshed, does not touch
  `put_together_same_attrs`, and does not modify any existing file beyond
  the two mod.rs lines.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-meshalgo
cargo clippy -p truck-meshalgo --all-targets -- -D warnings
cargo test -p truck-meshalgo --lib --tests
```

Never run a bare `cargo test`. truck-meshalgo has clippy deny-lints in
UNTOUCHED files that fail at base (recorded environmental) — scope your
clippy reading to YOUR files' findings and say so in RESULT notes if the
untouched-file noise is present. Send cargo output to a file and read the
tail.

## Forbidden

Editing any file outside `write_allow` — especially
`tessellation/triangulation.rs` (read-only reference; the production path
must stay byte-identical), `truck-topology/**`, `Cargo.toml`, `Cargo.lock`,
`scripts/kernel-gates.sh`. Adding welding, sewing, or any position-merging
beyond exact-equality interning (the convention's whole point). Adding
`#[ignore]`. Adding `#[allow]` without a same-line justification.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH` (A4 must read 0 — the entry
  does not exist yet; A6 must read 8 — the production sampling sites are
  where the packet's bit-identity duty points)
- the design as written cannot compile as specified (e.g. `PolylineCurve` /
  the sampling mechanism is not reachable from your write set) → `SPEC_GAP`,
  naming the exact conflict
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"BG-CG-005-LEDGER","status":"DONE","contracts":["BG-CG-005-LEDGER"],
 "tests_added":5,"anchors_verified":{"A1":1,"A2":2,"A3":1,"A4":0,"A5":1,"A6":8},
 "notes":"any deviation from the quoted design, with the reason"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it: what you attempted, the
exact ambiguity, and the readings you could not choose between.

Commit on the current branch with subject
`feat(meshalgo): edge-sample ledger with integer shared-edge identity (BG-CG-005-LEDGER)`
BEFORE writing `RESULT.json`.
