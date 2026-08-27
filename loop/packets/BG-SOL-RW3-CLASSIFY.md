# WORK PACKET BG-SOL-RW3-CLASSIFY - the fragment classifier

The Boundary Rewrite's §12 stage: classify every fragment of a
`FragmentMesh` as inside or outside the OTHER solid's closure, by
seed-and-propagate over the parity graph — not per-face ray casting.
The design is booked in `docs/SOLVER_FAMILY_PLAN.md` §4 Phase 4
(session 36 amendment, "The classifier (RW3)"); this packet realizes it
with every decision pre-made and machine-validated: the orchestrator
ran a full prototype (`scratch/rw3probe`, preserved) against the LANDED
splitter on the flagship and four ray-seed witnesses, and every number
quoted below is measured, not estimated. If live code contradicts this
packet, report it in `disagreements`.

This packet dispatches AFTER BG-SOL-SPLIT-PERIODIC lands (the flagship
witness's full event set refuses without it; that packet's test list
and the flagship test itself must be green at your fork point).

```json
{"id":"BG-SOL-RW3-CLASSIFY","status":"DONE","contracts":["BG-SOL-RW3-CLASSIFY"],
 "tests_added":6,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-RW3-CLASSIFY
contract:    [BG-SOL-RW3-CLASSIFY]
class:       design
crates:      [truck-base, truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-base/src/evidence.rs
read_allow:
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-modeling/src/extrude.rs
  - vendor/truck/truck-base/src/evidence.rs
  - docs/SOLVER_FAMILY_PLAN.md
tests_required:
  - classify_flagship_bits_are_exact
  - classify_disjoint_solids_all_outside
  - classify_contained_solid_ray_seed
  - classify_ray_seed_retries_ambiguous_direction
  - classify_contradictory_mesh_refuses
  - classify_cone_and_sphere_ray_solves
budget:      {turns: 40, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod split' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
  - {id: A2, expect: 2, cmd: "ls vendor/truck/truck-shapeops/src/boolean | wc -l"}
  - {id: A3, expect: 7, cmd: "grep -cF '#[test]' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn split_fragments' vendor/truck/truck-shapeops/src/boolean/split.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'FragmentInsideOther' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub enum Prop' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A7, expect: 1, cmd: "grep -cF 'fn point_in_solid' vendor/truck/truck-modeling/src/extrude.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub fn fragment_decision' vendor/truck/truck-shapeops/src/boolean/mod.rs"}
```

A2 becomes 3 (`classify.rs` joins `mod.rs` and `split.rs`); A5 becomes
1 (the new `Prop` arm). All others stay.

## Problem

`boolean()` (RW4) decides each fragment by `fragment_decision(op, m)`
over a `MaterialState4`. For a non-coincident fragment of solid A the
B-pair of the witnesses is `(s, s)` where `s` is one bit: whether the
fragment lies in B's closure. This packet computes that bit for every
fragment, index-aligned, as `FragmentClassification::inside_other`. A
per-fragment point-membership test cannot do this correctly (a fragment
straddling nothing still needs the bit, and surface points lie ON the
other solid's boundary half the time); the §12 design is a parity graph
with one certified seed per connected component and every non-tree edge
verified.

## Decisions already made

### 1. Module shape

`vendor/truck/truck-shapeops/src/boolean/classify.rs` (new) + one line
`pub mod classify;` in `boolean/mod.rs` beside `pub mod split;`. Carry
the H-1 deny header inside the module exactly like `boolean/mod.rs`
does (unwrap/expect/panic/todo/unimplemented/indexing_slicing). Every
public item carries a doc comment (the crate warns on
`missing_docs`).

### 2. The booked type and signature

```rust
use truck_evidence::Outcome;
use truck_topology::Shell;
use crate::{Curve, Point3, Surface};
use super::split::FragmentMesh;

/// One bit per fragment (index-aligned): inside the OTHER solid's
/// closure. For coincident fragments the bit is computed but NOT used
/// by the decision - the CoincidentPair's witnesses take precedence
/// there (RW4).
pub struct FragmentClassification {
    pub inside_other: Vec<bool>,
}

/// Classify every fragment of `mesh` against the other solid.
pub fn classify_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    tol: f64,
) -> Outcome<FragmentClassification>;
```

No events parameter: the classifier is self-contained (the arc-side
test finds the other solid's normal by searching ITS faces, and the ray
seed solves against its carriers). `mesh.coincident` is not consumed by
classification — coincident fragments get their bits by propagation
like every other fragment; the pairs matter only at RW4's decision.

### 3. The parity graph and components

Nodes: fragment indices. Edges: every `mesh.adjacency` entry, carrying
`parity == Flip` as a boolean. Connected components by BFS over the
adjacency (adjacency is same-solid by construction, so every component
is single-solid). Process components in order of their lowest fragment
index; inside a component everything is deterministic (lowest index
wins every choice below).

### 4. Seeds - one per component, two rules, in priority order

**(a) ARC-SIDE seed - when the component has any Flip adjacency.** The
seed fragment is the component's LOWEST-INDEX fragment that touches a
Flip adjacency. Take that fragment's Flip adjacencies in
`mesh.adjacency` order; for the first one, intersect the two fragments'
boundary edge-id sets (iterate `face.boundaries()` wires and
`wire.edge_iter()`); for the first shared edge id, evaluate at sample
parameters `[0.5, 0.25, 0.75]` of the edge's curve range (the first
decisive sample wins; on failure continue with the next shared edge /
next adjacency):

- `p = curve.subs(t)`; `der = curve.der(t)` negated if the edge use in
  the seed fragment's effective boundary wire has
  `edge.orientation() == false` (find the use by id in
  `face.boundaries()` — the effective wires; the same id appears in
  exactly one wire of the fragment face).
- `n_F`: the fragment's ABSOLUTE normal - `surface.normal(u, v)` at
  `surface.search_parameter(p)`'s parameters, negated when
  `face.orientation() == false`.
- `s_F`: the fragment's wire-orientation sign, `+1` iff
  `(A > 0.0) == face.orientation()` where `A` is the signed parameter
  polygon area of the fragment's FIRST effective boundary wire
  (projected with `create_parameter_boundary`). This calibrates
  "interior on the left of travel" for BOTH stored-wire conventions.
  Measured on the flagship: every flip-touching fragment reads
  `s_F = +1`. If the first wire's polygon is degenerate
  (`A == 0.0`, a band wall that somehow touches a Flip edge), refuse
  `NumericallyUnresolved` (defensive; not reachable from a well-posed
  split, since a Flip adjacency implies the face was divided into
  proper regions).
- `n_B` candidates: every face of the OTHER solid whose carrier
  contains `p` (`search_parameter(p)` succeeds AND
  `|surface.subs(uv) - p| <= tol`); its absolute normal the same way.
- For each candidate compute `val = s_F * (n_F × der) · n_B`.
  Candidates with `|val| <= 1.0e-6` (a named dimensionless const with
  an `// H-3` comment; the carrier is parallel to the fragment's —
  uninformative) are skipped. The sample is DECISIVE iff at least one
  candidate remains and all remaining candidates agree in sign.
- **The bit is `val < 0.0`** — the booked sign convention
  `(n_F × der) · n_B < 0 ⇒ INSIDE`. Machine-verified on the flagship:
  the annulus fragment reads exactly `val = +1.0` at the rim half-edge
  midpoint `(2, 3, 2)` (n_F = +ẑ, der = its traversal direction,
  n_B = the wall's +ŷ radial normal) ⇒ bit false; the disk fragment,
  same edge traversed oppositely, reads −1.0 ⇒ bit true.

**(b) RAY-PARITY seed - otherwise (contact-free component).** The seed
fragment is the component's LOWEST-INDEX fragment whose region
representative RESOLVES (`region_representative` over its wire
polygons; band-form cylinder walls have degenerate polygons and yield
none — fall through to the next fragment; if none resolves, refuse
`NumericallyUnresolved`). The representative's 3-D point `rep` is
`surface.subs(uv)`.

- **On-boundary pre-screen first:** for each face of the other solid
  whose carrier contains `rep` (same carrier test as above), classify
  `rep`'s `(u, v)` by the region classifier of decision 6: `Inside` ⇒
  return the bit `true` (in the closure); `Boundary` ⇒ refuse
  `NumericallyUnresolved`; `Outside` ⇒ continue to the next face. (On
  the flagship this fires: b's bottom-cap representative `(2, 2, 0)`
  lies inside a's bottom face's region ⇒ bit true, no ray cast.)
- **Ray casting:** a deterministic direction table, in order:
  `+ẑ, +x̂, +ŷ, −ẑ, −x̂, −ŷ,` then the eight diagonals
  `(±1, ±1, ±1)/√3` (a named const table; `1/√3` as a named constant
  or with `// H-3`). For each direction: winding = 0; for each face of
  the other solid, solve the ray×carrier crossings (decision 5); for
  each crossing `(t, q)` with `t > tol` (the extrude.rs rule —
  crossings at or behind the origin are skipped), classify `q`'s
  region: `Inside` ⇒ add the SIGNED crossing
  `+1` if `d · n_eff < 0` else `−1` (`n_eff` the face's absolute
  normal at `q`; the sign convention is extrude.rs's
  `point_in_solid`); `Boundary` ⇒ the whole DIRECTION is ambiguous,
  try the next one; `Outside` ⇒ ignore. A direction with no ambiguity
  answers `winding != 0`. Table exhausted ⇒ `NumericallyUnresolved`
  (`UnresolvedWitness::UncertifiedContainment`, an empty `Budget`).
  (The signed winding, not parity, because a single closed shell can
  be genus ≥ 1 — the plate-with-hole torus — and the extrude.rs
  pattern uses exactly this.)

### 5. The analytic ray×carrier solves (the other solid's faces)

Only four carriers have solves; a face with any other `Surface` arm in
the other solid refuses the whole call with
`UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)` — the ray seed
runs only when every face of the other solid is one of the four (the
arc-side seed has no such restriction; it works on any carrier).

- **Plane**: `denom = d · n`; `|denom| <= 1e-6` ⇒ no crossing; else the
  single `t = (origin − p) · n / denom`.
- **Cylinder** (`center`, `radius`): the quadratic over the
  xy-components relative to the center — extrude.rs's
  `face_ray_crossings` verbatim (`a = dx² + dy²`, `a <= 1e-6` ⇒ none;
  disc < 0 ⇒ none; both roots).
- **Sphere** (`center`, `radius`): `|p + t·d − c|² = r²` — both roots.
- **Cone** (`apex`, `half_angle`, axis +z): with `k = tan(half_angle)`,
  `e = p − apex`: `a = dx² + dy² − k²·dz²`,
  `b = 2(ex·dx + ey·dy − k²·ez·dz)`, `c = ex² + ey² − k²·ez²`;
  `|a| <= 1e-6` ⇒ none; disc < 0 ⇒ none; both roots. (The equation
  covers both nappes; the region check filters.)

### 6. The crossing/point region classifier (trichotomy)

`enum Region { Inside, Boundary, Outside }` for a query `(u, v)`
against a face:

- **Plane**: the polygon rule — `boundary_distance(polys, uv) <= tol`
  ⇒ `Boundary`; else `region_contains(polys, uv, u_period)` ⇒ `Inside`;
  else `Outside`. (`boundary_distance` = the min point-segment distance
  over all wire polygons' segments; use the exported
  `point_segment_distance`.)
- **Cylinder / Cone** (u periodic): compute the face's wire polygons;
  if EVERY polygon is degenerate (`|area| <= 1e-9`, named dimensionless
  const, `// H-3`) AND spans at least `u_period − 1e-9` in u, it is the
  BAND FORM (the extrude-wall signature, cut or uncut): `lo`/`hi` = the
  min/max v over all polygon points; `lo + tol < v < hi − tol` ⇒
  `Inside`; `|v − lo| <= tol || |v − hi| <= tol` ⇒ `Boundary`; else
  `Outside`. Otherwise the polygon rule (with the u_period passed to
  `region_contains`).
- **Sphere** (v is the azimuth — periodic in v, u is the polar angle):
  the same band test with the roles swapped (degenerate polygons
  spanning a full v period ⇒ the u-band rule on the polar coordinate);
  otherwise the polygon rule.
- Any other surface arm: unreachable here (decision 5 refused first).

### 7. Propagation and verification

From the seed node, BFS over the adjacency: `bit(v) = bit(u) XOR
(edge is Flip)`. Then EVERY adjacency edge of the component is checked
(tree edges hold by construction — check them anyway, it is cheaper
than tracking the tree): `Same` requires the bits equal, `Flip`
requires them different. The FIRST violation (in `mesh.adjacency`
order) refuses with

```rust
Refusal::Contradictory(ContradictionWitness {
    prop: Prop::FragmentInsideOther,
    left: /* Truth of the propagated bit of the edge's rhs */,
    right: /* Truth of the bit implied for rhs by lhs and the parity */,
})
```

`Truth::True`/`Truth::False` from the bools. Machine-verified on the
open-arc mesh (block + disk at (4,2), the RW2 test-3 events): BOTH
components are parity-inconsistent (the caps straddle the other solid's
boundary because the event list has no Region2 record) and the
prototype refuses at the first offending Same edge.

### 8. The `Prop` arm (truck-base, one line + doc)

Add to `Prop` in `vendor/truck/truck-base/src/evidence.rs`:

```rust
/// §12: a boundary fragment lies inside the other solid's closure
/// (BG-SOL-RW3).
FragmentInsideOther,
```

No exhaustive `match` on `Prop` exists anywhere (checked by grep —
only `set`/`get` call sites), so the arm has zero ripple. Do not touch
any other arm.

### 9. The pub(crate) exports from split.rs (visibility only)

`create_parameter_boundary`, `region_contains`, `region_representative`,
`point_segment_distance`, and `near_pt` become `pub(crate)` (doc
comments already exist; add none). Their signatures are unchanged
(`region_contains` carries the `u_period` parameter the periodic fix
gave it; pass the fragment's/face's `surface().u_period()` where the
query frame differs from the polygon frame, `None` where
self-consistent). No behavior change to split.rs otherwise.

### 10. Tolerance class

`tol` is the same insertion tolerance class the splitter uses (length).
The dimensionless constants (`1.0e-6` normal slack, `1.0e-9` area /
span slacks) are named consts each carrying `// H-3`. Never widen `tol`
to make a test pass; a witness that needs it is a finding
(`disagreements`).

## Tests required

Dyadic witnesses throughout (H-3). Copy the construction helpers from
split.rs's test module (`placed_circle`, `block_profile`,
`disk_profile`, `extrude_shell`, `plane_face_at_z`, `cylinder_face`,
`flat_edge_at_z`, `ev`, `ff_curve_record`) — they are the house
pattern. `TOL = 1.0e-2` like split.rs's tests. Identify fragments by
ORIGIN + wire structure (never by raw index without derivation) and
write the expected-bit derivation as inline comments.

1. `classify_flagship_bits_are_exact`: the flagship inputs and the FULL
   event set of `split_flagship_top_face_by_ff_circle` (FF circle + FE
   BoundedCurve + Region2 — copy the landed test's three events). The
   mesh is the landed test's (10 fragments; the annulus is the
   `[4, 2]`-wire fragment of a's top face, the disk the `[2]`-wire
   one). Expected bits, measured by the prototype: a's bottom `false`,
   the annulus `false`, the disk `true`, the four sides `false`; b's
   bottom cap `true`, top cap `true`, wall `true` — i.e. exactly
   `[F, F, T, F, F, F, F, T, T, T]` in fragment order. Assert the
   classification length and each fragment's bit by structure.
2. `classify_disjoint_solids_all_outside`: a = the block; b = the disk
   extrude at (6, 6) r=1 (height 2); NO events (the split call with an
   empty event list). Expected: 9 fragments, every bit `false` (both
   components ray-seed with winding 0 on direction 1).
3. `classify_contained_solid_ray_seed`: a = the block; b = a HAND-BUILT
   raised disk (the extrude recipe with a z offset, so no caps are
   coplanar with a's): bottom cap = `Face::try_new([Wire[circle at
   z=0.5]], Plane z=0.5).invert()`, top cap = the same form un-inverted
   at z=1.5, wall = `Face::try_new([Wire[circle z=0.5],
   Wire[circle⁻¹ z=1.5]], Cylinder(center (2,2,0), r=1))`; the circle
   edges are `Edge::new_unchecked(&v, &v, Curve::Circle(...))`
   self-loops (the BG-TOL-001-MESHALGO precedent) SHARED between the
   caps and the wall (each appears in exactly two faces with opposite
   orientations — that closes the shell). NO events. Expected: a's six
   fragments all `false` (the ray from a's bottom-face representative
   `(2, 2, 0)` crosses b's two caps, winding `+1 − 1 = 0`); b's three
   fragments all `true` (b's bottom-cap representative `(2, 2, 0.5)`
   is strictly inside a: the +ẑ ray crosses a's top face once,
   exiting, winding −1).
4. `classify_ray_seed_retries_ambiguous_direction`: a = the block; b =
   the same hand-built raised disk at center (2.5, 2) r=0.5 (z 0.5 to
   1.5) — DYADIC; a's bottom-face representative is `(2, 2, 0)`, which
   sits at radial distance exactly 0.5 from b's axis, ON b's caps'
   boundary circle. Expected: the first direction's crossings at
   `(2, 2, 0.5)` and `(2, 2, 1.5)` classify `Boundary` (ambiguous) so
   the seed retries; the second direction (+x̂) answers winding 0 —
   its wall crossings are at t=0 (skipped, `t <= tol`) and t=1 (the
   point `(3, 2, 0)`, rejected by the BAND rule: v=0 outside the
   [0.5, 1.5] band; if the band rule were broken and counted it, d·n
   > 0 makes it an exit, the winding would be −1, and the bit would
   flip — this is the test's real teeth). All a-fragments `false`;
   b's three fragments `true`. The pre-screen must NOT fire for a's
   representative: it lies ON b's wall CARRIER but OUTSIDE the wall's
   trimmed band, which is not the boundary — assert the outcome
   distinguishes carrier from region.
5. `classify_contradictory_mesh_refuses`: the open-arc mesh — a = the
   block, b = the disk extrude at (4, 2) r=1, the events of
   `split_open_arc_uses_point_events_for_trimming` (FF TwoCurves + the
   four Point events; NO Region2 record). The split succeeds (the RW2
   test is green); the classification MUST refuse
   `Refusal::Contradictory` with `prop == FragmentInsideOther` (both
   solids' cap fragments straddle the other solid's boundary — the
   missing Region2 record makes the mesh parity-inconsistent, and the
   non-tree-edge verification is what catches it).
6. `classify_cone_and_sphere_ray_solves` (unit test of the solve
   helpers, private fns callable from the test module): the sphere
   `center (0,0,0) r=2` against the ray from `(0,0,5)` along `−ẑ`
   crosses at `t = 3` and `t = 7`; the cone `apex (0,0,0)` half-angle
   `π/4` against the ray from `(5, 0, 1)` along `−x̂` crosses at
   `t = 4` (point `(1, 0, 1)`) and `t = 6` (point `(−1, 0, 1)`) —
   derive both quadratics by hand in the test comments before
   asserting (the BG-NUM-002 rule).

Machine-check every geometric number above before asserting it. Where
your derivation and this packet disagree, follow your derivation and
record the difference in `deviations` with the numbers.

## House form (H-3)

This crate is under the kernel's house rules. Any ADDED line with a
bare `1e-N` float literal must end `// H-3`; prefer dyadic values,
`TAU`/`std::f64::consts`, or named constants. GATE-2 scans the diff.
Run `bash scripts/kernel-gates.sh <your base commit>` before writing
RESULT.json - a failing gate is a finding to report, never one to work
around.

## Done when

```console
cargo fmt --check -p truck-base -p truck-shapeops
cargo clippy -p truck-base -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-base -p truck-shapeops --all-targets
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`shapeops: the section-12 fragment classifier (BG-SOL-RW3-CLASSIFY)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing anything outside `write_allow`; changing any split.rs
behavior (the exports are visibility-only); renaming or deleting a
pre-existing test; a `_` wildcard arm in any `Surface` match the
decision text enumerates; `#[ignore]`; loosening a gate; changing the
GATE-4 ceiling; widening `tol`; adding classification, seed, or
assembly logic beyond this packet's scope (RW4's decision/assembly is
NOT yours); calling `truck_evidence::contact::contact()`.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- a booked type cannot be realized as specified -> `SPEC_GAP` with the
  compile error and your proposed shape - do NOT silently change the
  booked field names, they are the inter-packet contract;
- the flagship's expected bits cannot be derived consistently from the
  mesh -> `SPEC_GAP` with both derivations;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not
`loop/results/`. Record in `notes`: the measured flagship bit vector,
which seed rule fired for each component in each test (arc-side / ray
with which direction index), any tolerance-class decision that
surprised you, and whether the prototype's predictions
(`scratch/rw3probe`, preserved) matched your implementation.
