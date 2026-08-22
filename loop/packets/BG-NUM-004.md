# WORK PACKET BG-NUM-004 — certified clustering; F-2 hash-grid fix

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-NUM-004","status":"DONE","contracts":["BG-NUM-004"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-NUM-004
contract:    [BG-NUM-004]
class:       design
crates:      [truck-evidence, truck-shapeops]
write_allow:
  - vendor/truck/truck-evidence/src/num/cluster.rs
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/polyline_construction/tests.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
  - vendor/truck/truck-base/src/tolerance.rs
budget:      {turns: 40, ctx_tokens: 100000}
anchors:
  # Measured under Git Bash on integration HEAD at packet-writing time.
  # A count mismatch is a stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: B1, expect: 12, cmd: "grep -c 'PointIndex' vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs"}
  - {id: B2, expect: 1, cmd: "grep -c 'FIXME(BG-TOL-001)' vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs"}
  - {id: B3, expect: 2, cmd: "grep -c '^pub mod' vendor/truck/truck-evidence/src/num/mod.rs"}
  - {id: B4, expect: 1, cmd: "grep -c 'pub fn isolate_roots' vendor/truck/truck-evidence/src/num/roots.rs"}
  - {id: B5, expect: 4, cmd: "grep -c 'fn construct_polylines' vendor/truck/truck-shapeops/src/transversal/polyline_construction/tests.rs"}
  - {id: B6, expect: 1, cmd: "grep -c 'near_pt' vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs"}
  - {id: B7, expect: 3, cmd: "grep -c 'PolylineCurve' vendor/truck/truck-shapeops/src/transversal/polyline_construction/mod.rs"}
  - {id: B8, expect: 0, cmd: "grep -c 'pub mod cluster' vendor/truck/truck-evidence/src/num/mod.rs"}
```

## Problem

Audit F-2: node identity in
`truck-shapeops/src/transversal/polyline_construction/mod.rs` is a hash grid —
`impl From<Point3> for PointIndex` quantizes each point to a cell of pitch
`2*TOLERANCE`. Two endpoints of one logical node land in DIFFERENT cells or
the SAME cell depending on absolute position: the graph splits where it should
weld (points ~1e-9 apart) and welds where it should split. The defect is
position-dependent, so it evades fixed-coordinate tests by construction.

The spec's replacement rule: clusters are determined by **certified ball
overlap**, never grid quantisation and never transitive closure of pairwise
nearness-as-tolerance (p ~τ q is NOT transitive and is not used). This packet
ships two halves:

- **`truck-evidence/src/num/cluster.rs`** — the general certified clustering
  core (pure, topology-free).
- **The F-2 fix itself** — polyline node identity switches from grid cells to
  canonical near_pt representatives. The polyline use case has no solve
  residuals to radii from, so it does NOT route through cluster() yet; that
  wiring lands with the emitter packets that own residuals. Documented
  deferral, not an oversight.

## Decisions already made for you

### Decision 0 — cluster.rs API (topology-free core)

```rust
pub struct Cluster {
    /// Indices into the input slice, sorted ascending.
    pub members: Vec<usize>,
    /// Center of a CERTIFIED enclosing ball for all member balls.
    pub center: Point3,
    /// Radius of that certified enclosing ball: an UPPER bound on the
    /// cluster's extent. It is an enclosure quantity, not a feature size.
    pub enclosing_radius: f64,
}

pub struct ClusterPolicy {
    /// Ball-inflation margin applied per endpoint: i~j iff
    /// ball(X_i, r_i + eps) overlaps ball(X_j, r_j + eps).
    pub eps: f64,
    /// Collision tolerance ceiling from the caller's context.
    pub tau_col: f64,
    /// Caller-supplied certified scale bound with lfs-shaped semantics.
    /// None means unconstrained (+inf semantics): admissibility then
    /// degrades to tau_col alone. Wiring real stratified evidence into this
    /// slot is later work; do not import fid::lfs here.
    pub scale_lower: Option<f64>,
}
```

Fixed constant: `THETA: f64 = 0.25` (H-3-commented) — the spec requires
theta < 1/2; 0.25 is chosen once, here, so callers cannot tune it toward the
degenerate limit.

```rust
pub fn cluster(
    points: &[Point3],
    radii: &[f64],
    policy: &ClusterPolicy,
    budget: &mut Budget,
) -> Outcome<Vec<Cluster>>
```

### Decision 1 — adjacency and components

Connect i~j iff `d(X_i, X_j) <= (r_i + eps) + (r_j + eps)`, computed on
SQUARED distances (`d² <= s²`) with a small relative slack const that biases
BORDERLINE cases toward MERGING (document why: over-merge degrades precision;
under-merge silently splits a feature, which is F-2's failure direction).
Components = union-find / BFS over that graph — connected COMPONENTS, which
is where transitivity legitimately lives (a chain of overlapping balls is one
cluster even when its ends are far apart; pairwise tolerance chaining as a
*predicate* is what is forbidden, and the test list exercises exactly this
distinction).

### Decision 2 — certified enclosing ball

Center = coordinate-wise midpoint of the bounding box of `{X_i ± r_i}` over
members; `enclosing_radius` = half the box diagonal. Coarse but CERTIFIED
(contains every member ball), deterministic, order-independent up to member
set. No smallest-enclosing-ball optimization.

### Decision 3 — admissibility refuses typed; refinement is the caller's loop

Admissible iff `enclosing_radius <= min(tau_col, THETA * scale_lower)` when
`scale_lower = Some(s)` (and `s > 0`), else iff
`enclosing_radius <= tau_col`. A violated cluster returns
`Err(Refusal::NumericallyUnresolved)` whose witness NAMES the violating
cluster index, its enclosing radius and the bound it exceeded. The spec's
"refine before any refusal" loop lives with the EMITTER (it owns the solver
that would produce smaller residuals): it re-solves and calls again. This
core cannot re-solve, so it must not pretend to. If you find a clean way to
express the refine step inside this signature without inventing solver
callbacks, propose it in `disagreements`; otherwise ship the refusal.

Budget: no bisection happens here; report it unchanged (spent zero).

### Decision 4 — the F-2 fix (polyline side)

Replace `PointIndex([i64; 3])` grid keys with CANONICAL REPRESENTATIVE
indices:

- The graph keeps a `Vec<Node>` (insertion-ordered) plus, per new point, an
  O(n) scan of stored representatives: the FIRST representative `r` with
  `ctx.near_pt(new, r.coord)` IS the same node; otherwise push a new node.
  Node identity is now position-independent Euclidean welding at the legacy
  tolerance — the exact defect class F-2 names, removed at its root.
- Adjacency sets hold representative indices (`HashSet<usize>`); the
  `PointIndex` struct and its `From<Point3>` impl are DELETED, including the
  FIXME comment (its site disappears).
- Traversal order becomes Vec-index order — strictly MORE deterministic than
  today's hashmap iteration; keep `get_one`/`get_a_next_node` semantics
  identical otherwise.
- The existing `add_edge` near_pt degenerate-edge suppression stays.
- All four existing tests MUST pass UNMODIFIED (their coordinates are exact,
  so welding behavior is identical).

O(n²) insertion is accepted: these are contact-network polylines, tens of
nodes. Say so in a comment; do not add a spatial index.

### Decision 5 — module hygiene

`num/mod.rs` gains exactly one line: `pub mod cluster;` (alphabetical, after
krawczyk — rustfmt-checked; update the module-list doc comment to mention
three modules). cluster.rs carries `#![deny(clippy::unwrap_used)]` including
its test module (GATE-1 gates NEW modules on unwrap_used), derives Debug on
public types, and every public item gets a doc comment stating what it
certifies. Every float literal in BOTH crates' diffs is a named const with a
same-line `// H-3:` comment. No new manifest edges: the shapeops edit uses
only `truck_base::tolerance` imports already present.

### Decision 6 — tests

cluster.rs test module:

1. `f2_close_points_cluster_at_any_translation` — two points 1e-9 apart,
   zero radii, eps = 1e-9: ONE cluster, run at translations
   {origin, ~(1e3,-2e3,3e3), ~(1e6,5e5,-7e5)} (named consts).
2. `f2_separated_points_stay_distinct_at_any_translation` — two points 3e-6
   apart, radii 1e-9, eps = 1e-7: TWO clusters, same three translations.
3. `chain_of_pairs_is_one_component_not_transitive_tolerance` — three points
   A,B,C with A~B, B~C overlapping but d(A,C) > threshold: still ONE cluster
   of 3 members (components are where transitivity lives), AND a control
   case where only A~B holds giving TWO clusters.
4. `partition_equivariant_under_translation` — a mixed 4-point input,
   translated: member INDEX SETS equal exactly.
5. `partition_equivariant_under_uniform_scale` — same input scaled by k
   (points, radii, eps, tau_col, scale_lower ALL scaled): same partition.
6. `admissibility_violation_refuses_with_witness` — tight tau_col forces
   Err; assert the witness names cluster 0 and both radius and bound.
7. `scale_bound_tightens_admissibility` — Some(scale) smaller than
   enclosing_radius/THETA refuses where None admitted.

tests.rs additions:

8. `welds_subtolerance_shared_endpoints_at_offsets` — two segments sharing a
   logical endpoint 5e-7 apart: ONE polyline through the welded node, run at
   several absolute offsets including large ones (~1e6) where the old grid
   split nodes.
9. `keeps_distinct_nearby_endpoints_separate` — endpoints 3e-6 apart stay
   distinct nodes (correct polylines lengths asserted).

All float comparisons via named consts with `// H-3:` same-line comments.
No bare 1e-N anywhere.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. EVERY
epsilon, offset and slack above is a named const whose defining line carries
a same-line `// H-3:` comment naming the dimensionless quantity. Run
`bash scripts/kernel-gates.sh <your base>` before writing RESULT.json.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence -p truck-shapeops
cargo clippy -p truck-evidence -p truck-shapeops --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast
cargo test -p truck-shapeops --lib --tests --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

Both crates are green at baseline (measured this session). Any baseline
failure you did not cause is a stop condition. Send cargo output to a file
and read the tail. Never run a bare `cargo test`.

## Forbidden

Editing files outside `write_allow`. Importing `fid::lfs` into cluster.rs
(scale_lower is injected; promoting primitive evidence into an lfs name is a
different packet's obligation). Adding a spatial index or
smallest-enclosing-ball solver. Touching `Cargo.toml` anywhere. Changing the
four existing tests. Bare float literals without `// H-3`.
`unwrap()`/`expect()` on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the existing polyline tests fail after the representative rewrite for a
  reason you cannot trace to your own change → `SPEC_GAP` naming the
  behavior delta
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(evidence,num): certified ball-overlap clustering; polyline node identity off the hash grid (BG-NUM-004)
```
