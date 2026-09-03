# WORK PACKET BG-SOL-RW2-SPLIT - the fragment splitter

The Boundary Rewrite's first topology packet: split each face of both
solids along the certified contact loci, producing the `FragmentMesh` the
§12 classifier (next packet) and the assembler consume. Every
`ContactLocus` arm has a defined behavior. The wire-mutation pattern is
the old transversal code's (read it; rebuild the services, do not edit
transversal). The design is booked in
`docs/SOLVER_FAMILY_PLAN.md` §4 Phase 4 (session 36 amendment) - read
that amendment first; this packet restates only what you build. If live
code contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-RW2-SPLIT","status":"DONE","contracts":["BG-SOL-RW2-SPLIT"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-RW2-SPLIT
contract:    [BG-SOL-RW2-SPLIT]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/Cargo.toml
  - Cargo.lock
read_allow:
  - vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/divide_face/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/faces_classification/mod.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-evidence/src/contact/fe_ee.rs
  - vendor/truck/truck-evidence/src/analytic/mod.rs
  - vendor/truck/truck-geometry/src/recognize.rs
  - vendor/truck/truck-modeling/src/extrude.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - split_flagship_top_face_by_ff_circle
  - split_cuts_edges_at_point_contacts
  - split_open_arc_uses_point_events_for_trimming
  - split_region2_disjoint_regions_is_no_coincidence
  - split_region2_partial_overlap_refuses
  - split_refuses_deferred_loci
budget:      {turns: 40, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "ls vendor/truck/truck-shapeops/src/boolean | wc -l"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod boolean' vendor/truck/truck-shapeops/src/lib.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'truck-evidence' vendor/truck/truck-shapeops/Cargo.toml"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn fragment_decision' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn integrate_by_component' vendor/truck/truck-shapeops/src/transversal/faces_classification/mod.rs"}
```

A1 becomes 2 (`split.rs` joins `mod.rs`); A3 becomes ≥ 1 (the dependency
line). A2, A4, A5 stay.

## Problem

`boolean()` (a later packet) will classify each FACE FRAGMENT by material
state and sew the kept ones into the result solid. Before that, every
face that a contact locus crosses must be SPLIT along the locus, and the
split edges must be SHARED INSTANCES between the two solids' fragment
wires (Arc identity is what makes the assembled shell close). The old
procedural Boolean did this with polyline marchers and triangulations;
this packet does it from the Contact Layer's certified records, reusing
the old code's topology-mutation patterns.

## Decisions already made

### 0. The manifest edge

Add `truck-evidence = { version = "0.1.0", path = "../truck-evidence" }`
to truck-shapeops's `[dependencies]` (edit `Cargo.toml`, run any cargo
command once to refresh `Cargo.lock`, commit both). Acyclic: truck-evidence
depends only on truck-base/truck-geotrait/truck-geometry/inari. This is
the BG-INV-104 layering precedent.

### 1. Module shape

`vendor/truck/truck-shapeops/src/boolean/split.rs` (new) + one line
`pub mod split;` in `boolean/mod.rs` (alphabetical position: after the
doc header, before `mod`-less content - match the file's existing style;
the `pub use` surface is your choice). Carry the H-1 deny header inside
the module exactly like `boolean/mod.rs` does. Every public item carries
a doc comment (the crate warns on `missing_docs`).

### 2. The booked types (plan §4 Phase 4, session 36 - copy verbatim)

```rust
use truck_evidence::contact::{ContactRecord, ContactDimension, ContactEventKind, ContactLocus};
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
use truck_evidence::{Outcome, Refusal};
use truck_topology::{Edge, Face, Shell, Wire};
use crate::{Curve, Point3, Surface};

/// Which solid a stratum reference belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidRef { A, B }

/// Where a contact event's record came from. Faces index
/// `shell.face_iter()` order; an edge names its position in
/// `face.absolute_boundaries()` flattened wire-by-wire, edge-by-edge, in
/// order. Edge identity is resolved by `EdgeID` (the same instance
/// appears in adjacent faces).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StratumRef {
    Face { solid: SolidRef, index: usize },
    Edge { solid: SolidRef, face: usize, edge: usize },
}

/// One contact record with the provenance the splitter needs.
#[derive(Clone, Debug)]
pub struct ContactEvent {
    pub record: ContactRecord,
    pub lhs: StratumRef,
    pub rhs: StratumRef,
}

pub enum FragmentOrigin { A { parent: usize }, B { parent: usize } }
pub struct Fragment { pub face: Face<Point3, Curve, Surface>, pub origin: FragmentOrigin }
pub enum AdjacencyParity { Same, Flip }
pub struct FragmentAdjacency { pub lhs: usize, pub rhs: usize, pub parity: AdjacencyParity }
pub enum CoincidentOrientation { Identical, Anti }
pub struct CoincidentPair { pub a: usize, pub b: usize, pub orientation: CoincidentOrientation }
pub struct FragmentMesh {
    pub fragments: Vec<Fragment>,
    pub adjacency: Vec<FragmentAdjacency>,
    pub coincident: Vec<CoincidentPair>,
}

/// Split both shells along the contact events.
pub fn split_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    events: &[ContactEvent],
    tol: f64,
) -> Outcome<FragmentMesh>;
```

Field order/names are the contract (RW3/RW4 consume them); derive
`Clone, Copy` etc. to taste but keep the field names. `FragmentMesh`'s
`adjacency` entries are PER SHARED EDGE INSTANCE (one pair of fragments
sharing two sub-edges appears twice); `coincident` entries are
cross-solid pairs (never adjacency entries).

### 3. The services to rebuild (patterns, from transversal - do not edit it)

- **Wire→parameter polygon**: `divide_face::create_parameter_boundary`
  (transversal/divide_face/mod.rs:22) - project each boundary edge's
  division points into `(u,v)` via `search_parameter` with the periodic
  unwrap (`unwrap_periodic_parameter`, same file). You need it per-face
  on the MUTATED structure; cache per edge id.
- **Edge cutting**: `LoopsStore::commit_polygon_vertex` +
  `change_vertex` + `swap_edge_into_wire` (transversal/loops_store/
  mod.rs:345, 200, 232) - cut an edge at a parameter, propagate the
  halves to EVERY wire referencing the edge id (in BOTH shells), unify
  vertices through the shared `emap` (and EVICT ids of dropped halves -
  the BG-CE-003-MIGRATE-r2 lesson is documented in that code).
- **Arc insertion**: `Loops::add_edge` (loops_store/mod.rs:251) for open
  arcs (rotate-to-endpoint, splice, split wires) and
  `add_independent_loop` (line 245) for closed arcs (cut at the midpoint
  parameter into two half-edges; the pair of opposite wires is what
  makes `divide_one_face` produce two fragments).
- **Region division**: `divide_one_face` (divide_face/mod.rs:69) -
  parameter-polygon area sign, negative wires attach into containing
  positive regions (`include`), `Face::debug_new`, invert when the
  parent face's `orientation()` is false.

### 4. The per-arm split semantics (every arm matched; no `_` wildcard)

Process per FACE: group events by the face they touch (an event touches
a face through either side). Then:

- **FF Transverse `Analytic(Curve(c))` / `TwoCurves([c0, c1])`**
  (dimension Arc1, kind Transverse): insert each curve into BOTH named
  faces' structures as SHARED edge instances. Convert `ExactCurve` to
  the `Curve` enum: `Line→Curve::Line`, `Circle|Ellipse→Curve::Circle`
  (the `PlacedCircle` payload type is identical - an ellipse IS the unit
  circle under a non-conformal placement). `Parabola | Hyperbola` have
  no `Curve` arm: refuse the whole call with
  `Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)`.
  - A CLOSED curve (a `Circle`/`Ellipse` whose parameter range is the
    full period - decide by the curve's own range, not by sampling):
    if its parameter polygon lies strictly inside the face's region
    (parameter-polygon containment, tolerance `tol`), insert it as the
    doubled independent loop. If it CROSSES the face's boundary, the
    crossing points must be certified by Point events on that face's
    boundary edges (see below); clip the curve's parameter range at the
    two extreme crossings and insert the open arc. If the crossings are
    not certified by Point events, refuse
    `UnsupportedEnvelope(ContactReductionDeferred)` - never solve
    curve×boundary yourself in this packet.
  - An OPEN curve (a `Line`): its endpoints-in-region must be certified
    by Point events on the face's boundary edges; clip to the extreme
    ones and insert. Same refusal when missing.
- **`Point(p)` loci, kind Transverse** (FE punctures / EE crossings):
  cut the named edge(s) at the parameter projection of `p`
  (`search_parameter` on the edge's curve with `p`; a failed/unstable
  projection refuses `NumericallyUnresolved` - build the
  `UnresolvedWitness::UncertifiedContainment` witness and spend an empty
  `Budget`). These cut points are the trimming oracle for arcs (above).
  `EndpointTouch` points refuse (deferred).
- **FE `BoundedCurve { curve, t_range }`** (kind CoincidentInterval - an
  edge of one solid lying ON a face of the other): the SEWING ORACLE.
  When the face's split (by an FF arc or a Region2 split) produces a
  boundary along this edge's carrier, REUSE the edge (cut to the arc's
  extent) instead of creating a duplicate - the shared instance is the
  seam of the final solid. Carrier identity between the record's
  `ExactCurve` and the edge's `CanonicalCurve` is GEOMETRIC (a line:
  same two points up to parameterization; a circle: same center,
  radius, and plane), NOT struct equality of placements - the FF
  circle and the rim edge are the same geometric circle built by
  different code paths. Write the identity predicate for Line and
  Circle; other carriers refuse.
- **Region2 `Coincident`** (any locus path: `ContactLocus::Coincident`
  or `Analytic(Coincident)`, dimension Region2): the containment screen
  between the two named faces. Project both faces' boundary wires to
  parameter polygons (same carrier ⇒ compatible parameterizations; for
  struct-equal carriers the identity map). Decide:
  - **no boundary crossings between the two faces' wires** AND **one
    region contains the other** (region-level point tests: a boundary
    sample point of one face inside the other's region - outer polygon
    minus hole polygons): containment coincidence. Split the CONTAINING
    face along the CONTAINED face's boundary wires (inserting arcs as
    above - REUSING already-inserted shared instances where the carriers
    geometrically identify, so the flagship's rim circle is inserted
    once), and emit `CoincidentPair { a: <the containing solid's
    fragment covering the overlap>, b: <the contained solid's
    fragment>, orientation }` where `orientation` is `Identical` when
    the two faces' ABSOLUTE normals agree (both `orientation()` flags
    equal after mapping to absolute normals), `Anti` otherwise.
  - **no crossings and neither contains the other** (disjoint regions -
    the parameter-box over-approximation false positive): no pair, no
    split; both faces pass through.
  - **crossings exist** (the wires of the two faces intersect): refuse
    `UnsupportedEnvelope(ContactReductionDeferred)` (the partial
    overlap family - named follow-up RW-COPLANAR).
- **`ValidatedBranchCover` locus / any record with kind `Tangency`**:
  refuse `UnsupportedEnvelope(ContactReductionDeferred)` (named
  follow-ups RW-ARC-CONT / RW-TANGENT). The match must be exhaustive
  WITHOUT a `_` arm - rustc then enforces that a future locus arm
  cannot be silently dropped.

### 5. The parity rule

An `AdjacencyParity` entry is `Flip` when the shared edge is a
contact-introduced arc (the boundary of the OTHER solid's material
within this face's carrier - both FF transverse arcs and Region2
containment-split arcs), `Same` when it is (a sub-edge of) one solid's
original edge. Cross-solid shared edges are sewing, not adjacency - they
appear in no `adjacency` entry. Same-parent fragment pairs (sharing
contact arcs) are always `Flip` in v1.

### 6. Faces with no events pass through

A face no event touches becomes exactly one fragment (its
`absolute_boundaries` cloned, its `orientation()` preserved). The
original sub-edges it shares with its same-solid neighbors produce
`Same` adjacencies. `FragmentOrigin` records the parent face index.

### 7. Tolerance class (do not relitigate)

Insertion geometry (parameter projections, polygon containment) is
tolerance-class at `tol` - the same class the old code used. The
DECISIONS (which loci, which faces, coincident vs disjoint) come from
the certified records. Never widen `tol` to make a test pass; a witness
that needs it is a finding (`disagreements`).

## Tests required

Dyadic witnesses throughout (H-3). Build solids with
`truck_modeling::extrude::extrude_profile` + `truck_geometry::arrange`
(the extrude.rs test module's `plate_with_hole` helper is the pattern;
truck-modeling is already a dev-dependency). `extrude_profile`'s disk
wall-orientation defect (being fixed in parallel by
BG-SOL-S2-DISK-ORIENT) is IRRELEVANT here: the splitter preserves
whatever orientation the parent face carries. Construct `ContactEvent`s
BY HAND - no `contact()` calls (that wiring is a later packet).

1. `split_flagship_top_face_by_ff_circle`: a = extrude of the 4×4
   rectangle (profile: four `Curve::Line`s (0,0)→(4,0)→(4,4)→(0,4)→(0,0)),
   height 2; b = extrude of the disk (one `Curve::Circle` at (2,2) r=1,
   the plate_with_hole helper's construction), height 2. Events:
   - FF: `{Arc1, Transverse, Analytic(Curve(ExactCurve::Circle(<placed
     circle at (2,2,2), r=1>)))}` between a's TOP face (the `Plane`
     with `orientation() == true` whose wires' vertices are at z=2) and
     b's CYLINDER face.
   - FE: `{Arc1, CoincidentInterval, BoundedCurve { curve: <the same
     circle>, t_range: (0.0, TAU) }}` between a's top face and the
     cylinder face's TOP wire's edge (the closed rim edge - two wires,
     take the z=2 one).
   - Region2: `{Region2, CoincidentInterval, ContactLocus::Coincident}`
     between a's top face and b's TOP cap face.
   Assert: a's top face becomes TWO fragments (the disk: one wire of
   two half-edges; the annulus: the square wire plus the hole wire of
   the same two half-edges inverted); every OTHER face of both shells is
   exactly one fragment (a: 5 side/cap faces + b: wall + bottom cap);
   the two rim half-edge INSTANCES are EdgeID-identical across the disk
   fragment, the annulus fragment, b's wall wire, and b's top-cap wire
   (the cut propagated: the wall's and cap's wires now carry the two
   halves); `coincident` has exactly one entry pairing the disk fragment
   with b's top-cap fragment, `Identical` (both faces
   `orientation() == true`); `adjacency` contains the disk↔annulus
   `Flip` entry (twice - once per half-edge) and `Same` entries among
   a's untouched faces; no `a`-side fragment pairs with a `b`-side one
   in `adjacency`.
2. `split_cuts_edges_at_point_contacts`: a = the plate; one synthetic
   event: `{Point0, Transverse, Point(Point3::new(2.0, 0.0, 2.0))}`
   with the edge side naming a's top face's FIRST boundary edge (the
   (0,0,2)→(4,0,2) line) and the face side naming the top face. Assert:
   the top face is still ONE fragment, its wire now has 5 edges, the new
   vertex sits at (2,0,2), the two halves are `Curve::Line` with the
   right endpoints, and the top-face↔front-side-face `Same` adjacency
   appears once per half.
3. `split_open_arc_uses_point_events_for_trimming`: a = the plate; b =
   extrude of the disk at (4, 2) r=1, height 2. Events: FF
   `{Arc1, Transverse, Analytic(TwoCurves([<line x=4,y=1>, <line
   x=4,y=3>]))}` between a's x=4 SIDE face (the `Plane` through
   (4,0,0),(4,4,0) - identify it by its surface's origin) and b's
   cylinder face; PLUS four `Point` events naming a's side face's
   BOTTOM and TOP boundary edges (the (4,0,0)→(4,4,0) and (4,0,2)→(4,4,2)
   lines) at (4,1,0), (4,3,0), (4,1,2), (4,3,2). Assert: the side face
   becomes THREE fragments; the bottom and top edges are each cut at
   their two points (3 edges each); the two inserted line edges are
   shared instances between the middle strip and each outer strip; the
   adjacency entries across the two inserted lines are `Flip` (twice
   each), the outer strips' adjacencies to the neighboring untouched
   faces are `Same`.
4. `split_region2_disjoint_regions_is_no_coincidence`: a = extrude of
   the M1 plate-with-hole profile (rectangle + circle at (2,2) r=1);
   b's face = a hand-built `Face` on `Surface::Plane` z=2 with a single
   closed-circle wire of radius 0.8 at (2,2) (build the edge
   `Edge::new_unchecked` on a `Curve::Circle` - the self-loop pattern,
   see extrude.rs); assemble a one-face shell for b. Event: Region2
   `Coincident` between a's top face and b's face. Assert: NO
   coincident pair, no split - a's top face is one fragment (the disk
   r=0.8 lies strictly inside the hole; the parameter boxes overlap but
   the regions are disjoint - the containment screen's rescue path).
5. `split_region2_partial_overlap_refuses`: same a; b's face = the
   hand-built disk face with radius 1.5 at (2,2) (its circle crosses
   the hole circle r=1 twice). Assert: the whole call refuses
   `UnsupportedEnvelope(ContactReductionDeferred)`.
6. `split_refuses_deferred_loci`: three one-event calls, each refusing
   `UnsupportedEnvelope(ContactReductionDeferred)`: (a) kind `Tangency`
   with `Analytic(TangentLine(<a line on the face>))`; (b) kind
   `Transverse` with `Analytic(Curve(ExactCurve::Parabola(<placed
   parabola>)))`; (c) kind `EndpointTouch` with `Point(<a point>)`.

Machine-check every geometric number above before asserting it (the
BG-NUM-002 rule): the witness geometry is dyadic and small; derive each
expected count (fragments, edges, adjacencies) from the construction
and write the derivation as inline comments. Where this packet's prose
and your derivation disagree, follow your derivation and record the
difference in `deviations` with the numbers.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a
bare `1e-N` float literal must end `// H-3`; prefer dyadic values,
`TAU`/`std::f64::consts`, or named constants. GATE-2 scans the diff.
Run `bash scripts/kernel-gates.sh <your base commit>` before writing
RESULT.json - a failing gate is a finding to report, never one to work
around.

## Done when

```console
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-shapeops --all-targets
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command. The
`cargo check --locked` run must come AFTER the Cargo.lock refresh - if
`--locked` fails because the lock is stale, refresh it once and commit
both files.

**Commit your work on the current branch** (subject
`shapeops: fragment splitting from contact records (BG-SOL-RW2-SPLIT)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow` (in particular: do NOT edit
`vendor/truck/truck-shapeops/src/transversal/**`, `lib.rs`, or
`truck-evidence`); making transversal's services public instead of
rebuilding them; a `_` wildcard arm in the `ContactLocus` match; calling
`truck_evidence::contact::contact()`; adding classification, seed, or
assembly logic (RW3/RW4); `#[ignore]`; loosening a gate; changing the
GATE-4 ceiling; renaming or deleting a pre-existing test; widening
`tol` mid-implementation.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a booked type cannot be realized as specified (e.g. `Face` is not
  `Clone`, a field name collides) -> `SPEC_GAP` with the compile error
  and your proposed shape - do NOT silently change the booked field
  names, they are the inter-packet contract;
- the flagship witness's expected fragment/wire/adjacency counts cannot
  be derived consistently -> `SPEC_GAP` with both derivations;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes`: the flagship mesh's exact counts (fragments per face,
adjacency entries by parity, the coincident pair), any tolerance-class
decision that surprised you, and whether you found the old
loops_store/divide_face services adequate as patterns (RW3 will reuse
your verdict).
