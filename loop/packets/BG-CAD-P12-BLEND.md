---
id: BG-CAD-P12-BLEND
class: design
crates: [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/rewrite.rs
  - vendor/truck/truck-shapeops/tests/fillet_circle.rs
tests_required:
  - fillet_circle_top_rim
  - fillet_circle_bottom_rim
  - fillet_circle_both_rims
  - fillet_circle_junction_certificate
  - fillet_circle_torus_face_rides_p11_pairs
  - fillet_circle_multiwire_cap_refuses
  - fillet_circle_overflow_refuses
budget: {turns: 45, ctx_tokens: 140000}
---

# BG-CAD-P12-BLEND — realization table 6.4 extended: center locus Circle → Torus (F2)

Program: `docs/BUILD123D_COVERAGE_PLAN.md` P12 (Tier 2), scoped by the
session-42 frontier to the table 6.4 row the landed P6/P7 packets did not
take: **center locus Circle → Torus** (the F2 constant-radius rolling-ball
on a curved spine, canonical-output). Rides the LANDED P11 torus pairs
(the realized torus face consumes through `contact()`) and the LANDED
P6/P7 rewrite engine module. The pre-dispatch num3-scratch probe is DONE
and PASSED (its evidence is QUOTED here; the probe source is untracked
scratch — everything load-bearing is in this packet). Everything below is
pre-decided; churn, don't design. Contradiction with the tree = `SPEC_GAP`.

## Problem

`fillet` (P7) realizes plane-plane edges: center locus Line → Cylinder,
three-plane corner → Sphere. The remaining canonical row is the circular
rim: a canonical z-axis `Cylinder` wall meeting a perpendicular canonical
`Plane` cap at a full circle. The rolling-ball center locus is a circle
(radius R−r at height cap_z ± r), and the realized face is a canonical
z-axis `Torus` (table 6.4: "center locus Circle → Torus (constant-frame
case)"). The junction geometry — how the torus patch meets the adjacent
faces at the junction circles — was the one unprobed derivation; the probe
machine-validated the complete target state (below).

## The probe evidence (quoted — the worker's worktree has no scratch/)

Every witness built DIRECTLY with the landed construction primitives and
accepted by `Solid::try_new`; every number machine-measured.

**Finding 1 — the landed extruded-cylinder census (the construction
vocabulary).** `extrude_profile` over a disk profile (radius 2, height 2)
yields 3 faces, 2 unique edges, 2 unique vertices: each cap is a `Plane`
face with ONE wire holding ONE self-loop `Curve::Circle` edge
(`Edge::new_unchecked` — the self-loop IS the seam; no seam generator
edges exist); the wall is a `Cylinder` face with TWO boundary wires (each
one self-loop circle, bottom + top). Full-revolution faces carry their
boundary circles as self-loop-edge WIRES, one wire per circle.

**Finding 2 — the top-rim fillet target state ASSEMBLES FIRST TRY.**
Cylinder R=2, h=2, fillet r=0.5 at the top rim: 4 faces, 3 unique edges,
3 unique vertices —

- wall: the SAME `Cylinder` carrier (center (0,0,0), R=2), wires
  [bottom circle (existing instance), NEW junction circle r=2 at z=1.5];
- torus face: `Surface::Torus(Torus::new((0,0,1.5), 1.5, 0.5))` with TWO
  wires: junction circle r=2@z=1.5 (the tube's outer equator, v=0) and
  junction circle r=1.5@z=2 (the cap-tangent circle, v=π/2);
- top cap: the SAME `Plane` carrier concept (plane z=2), wire = the new
  circle r=1.5@z=2;
- bottom cap untouched.

The junction circles are SELF-LOOP circle edges minted ONCE and shared as
instances by their two adjacent faces. **Coedge orientation recipe
(measured): the wall takes its junction edge FORWARD, the torus takes both
junction edges INVERSE, each cap takes whichever pairs it (inverse against
the wall's forward, forward against the torus's inverse).** A wrong pairing
refuses with exactly "This shell is not oriented and closed."
Concretely accepted: bottom cap wire = [e_bot.inverse()], wall wires =
[e_bot, e_jw] forward, torus wires = [e_jw.inverse(), e_jc.inverse()],
top cap wire = [e_jc].

**Finding 3 — the bottom rim (the s-rule's other branch).** Fillet r=0.5
at the bottom rim (cap z=0 below, wall up to z=2): torus
`Torus::new((0,0,0.5), 1.5, 0.5)`, wall junction r=2@z=0.5, cap junction
r=1.5@z=0; 4 faces / 3 edges / 3 vertices; same orientation recipe.

**Finding 4 — both rims.** Two fillets: 5 faces, 4 unique edges, 4 unique
vertices (wall shorter, two tori, two caps).

**Finding 5 — the washer is the multi-wire refusal boundary.** The annulus
extrude (4×4 square + hole r=1, height 1) has 7 faces; its top face is a
TWO-wire `Plane` (outer square + hole self-loop circle). Filleting the
hole's rim (center (0,0,1), edge radius 1) has that 2-wire face as the cap
→ the neighborhood check refuses. This is the booked v1 boundary (concave
and annular fixtures are out of envelope).

**Finding 6 — the P11 ride.** The realized torus face through the LANDED
dispatcher: `contact(torus-patch-stratum, plane z=1.75)` (band-clear:
torus-local z = +0.25 = r/2) returns ONE record (dim Arc1, kind
Transverse, method Interval) whose locus is `ValidatedBranchCover` with 36
certified points, ALL on the closed-form branches r̂ = 1.5 ± sqrt(0.1875),
z = 1.75 exactly, torus residual certified. BOTH branches certify because
the full-ring patch's world box contains the inner circle's positions (the
landed `torus_pairs.rs` two-circle shape). The junction planes themselves
are NOT rideable: the cap junction circle sits ON the landed equator-band
locus r̂ = R_torus (P11's Finding 3) — a plane through it defers.

**Finding 7 — vertex-derived bboxes are MEANINGLESS here.** The only
vertices are self-loop seam vertices; the extreme points live on the
circle carriers. All certificates are carrier-derived (circle curves +
torus subs), never vertex-bbox. (The P6/P7 bbox-exact certificate pattern
does NOT transfer.)

## Anchors (measured 2026-08-29 at HEAD `abb42ef`; re-derive before writing RESULT.json)

| id | file | pattern | count |
|----|------|---------|-------|
| A1 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn fillet\(` | 1 |
| A2 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub struct FilletSpec` | 1 |
| A3 | vendor/truck/truck-shapeops/src/rewrite.rs | `pub fn chamfer\(` | 1 |
| A4 | vendor/truck/truck-shapeops/src/lib.rs | `pub mod rewrite` | 1 |
| A5 | vendor/truck/truck-shapeops/tests | `fillet_circle.rs` | 0 (new file) |
| A6 | vendor/truck/truck-geometry/src/specifieds/torus.rs | `pub fn new\(center: Point3, large_radius: f64, small_radius: f64\)` | 1 |

All anchors are the PRE-packet tree. This packet adds NO new module: the
circular-rim fillet lives IN `rewrite.rs` beside the chamfer and fillet
(same declared module — A4 never diverges).

## Decisions already made for you

**D1 — the entry and spec.**

```rust
/// One filleted CIRCULAR rim: the rim edge is named by its circle
/// geometry (the canonical z-axis rim circle's center and radius).
/// `radius` is the single rolling-ball radius.
#[derive(Clone, Copy, Debug)]
pub struct CircleFilletSpec {
    pub center: Point3,
    pub edge_radius: f64,
    pub radius: f64,
}

pub fn fillet_circle(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[CircleFilletSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>>;
```

Resolution: the unique `Curve::Circle`-carried edge in the solid's wires
whose circle center is within the module's insertion-tolerance class of
`spec.center` and whose radius matches `spec.edge_radius` (the P6
edge-resolution convention). Zero matches → `Refusal::Empty`; multiple →
`UnsupportedEnvelope(NonCanonicalCarrier)`. A non-finite or non-positive
`radius` or `edge_radius` → `Empty`.

**D2 — the neighborhood lift (NOT the P6 polygon lift).** The P6 `lift`
refuses Cylinder carriers and Circle edges by design; `fillet_circle`
validates ONLY the resolved rim's neighborhood, and other faces ride
UNTOUCHED (their faces, wires, and edge instances are reused verbatim):

1. The rim edge has exactly TWO adjacent faces (machine-check from the
   shell; anything else → `NonCanonicalCarrier`).
2. One is a canonical `Surface::Cylinder` carrier (the wall) and one is a
   canonical `Surface::Plane` carrier whose plane normal is PARALLEL to
   the z-axis (the perpendicular cap — the constant-frame case). Any other
   carrier kind, an oblique cap, or a wall that is not a bare canonical
   z-axis cylinder → `NonCanonicalCarrier`.
3. The cap face's boundary must be a SINGLE wire holding a single
   self-loop circle edge concentric with the rim (the Finding 1 cap
   shape); multi-wire caps (the washer's Finding 5 shape) refuse
   `NonCanonicalCarrier`. (This single check excludes the concave and
   annular families structurally — see D6.)
4. The wall face's OTHER boundary wire is a single self-loop circle edge
   concentric with the rim (the wall's other rim, radius R at z_other).
   Anything else → `NonCanonicalCarrier`.

**D3 — the realization (the probe recipe, s-rule form).** With cap plane
height `cap_z` (the cap junction circle's z), rim radius R, wall's other
rim at `z_other`, and `s = +1` if `z_other > cap_z` else `−1` (the
material side of the cap — the side the wall is on):

- the torus center locus is the circle radius `R − r` at height
  `cap_z + s·r`; carrier `Torus::new((cx, cy, cap_z + s·r), R − r, r)`
  (A6's constructor);
- the wall junction circle: radius R at z = `cap_z + s·r` (the tube's
  outer equator);
- the cap junction circle: radius `R − r` at z = `cap_z`;
- overflow: `r ≥ |z_other − cap_z|` (the wall would vanish) or
  `r ≥ R` (the cap would collapse) → `Refusal::Empty` — check BEFORE
  minting anything;
- the rebuilt wall face reuses the SAME `Cylinder` carrier instance with
  wires [other-rim circle (existing edge instance), wall junction circle
  (new)]; the rebuilt cap face reuses the SAME `Plane` carrier instance
  with wire [cap junction circle (new)]; the torus face is new with wires
  [wall junction, cap junction];
- the coedge orientation recipe is Finding 2's, machine-checked by the
  `Solid::try_new` gate: wall junction edges FORWARD in the wall's wires,
  torus takes both junction edges INVERSE, the caps take whichever pairs
  (the other-rim circle against the wall's forward convention exactly as
  the landed extrude's census — Finding 1 — pairs it).

**D4 — multiple specs process SEQUENTIALLY.** Each spec applies to the
current solid through the D2/D3 rewrite (the intermediate solid is a real
assembled `Solid::try_new` result). The both-rims fixture (Finding 4) is
two sequential applications; its final census is 5 faces / 4 unique edges
/ 4 unique vertices. A spec whose rim no longer exists on the current
solid (double-filleting the same rim) refuses `Empty` at resolution.

**D5 — refusals (zero new arms).** `NonCanonicalCarrier` (D2's lift,
ambiguous resolution) and `Empty` (D1 degenerate specs, D3 overflow, D4
disappeared rim). No new `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/
`Collapse` arms — a perceived need is a SPEC_GAP.

**D6 — certificates (carrier-derived; Finding 7 forbids vertex-bbox).**
`Solid::try_new` is the acceptance gate. Tests additionally certify, per
realized rim: the torus carrier's center/major/minor exact; sampled
points of each junction circle lie ON the torus (`(r̂ − major)² +
(z − cz)² = minor²` exactly at the samples) AND on their adjacent carrier
(wall: `r̂ = R`; cap: `z = cap_z`, `r̂ = R − r`); the cap junction circle's
curve center/radius exact.

**D7 — boundaries (booked follow-ups; refuse, do not attempt).** Oblique
caps, non-perpendicular rims, non-circular rims, concave/annular fixtures
(Finding 5), variable-radius blends, F3 chains, general canal surfaces,
and torus×torus offsets are OUT of envelope. `ExtrudedCurve`/
`RevolutedCurve` carriers refuse `NonCanonicalCarrier` at D2. The
three-quarter-tube ambiguity of a two-circle torus face (the wires do not
disambiguate the patch's angular extent; the construction's meaning is the
outer quarter) is a RECORDED note, not a check — the meaning is fixed by
D3's construction.

## Template

- `vendor/truck/truck-shapeops/src/rewrite.rs` — the landed engine module
  (A1/A2/A3): reuse the module's helpers (pools, `non_canonical()`,
  `invalid_shell()`, the insertion-tolerance class) where they fit; do not
  duplicate them.
- `vendor/truck/truck-geometry/src/specifieds/torus.rs` — the carrier
  (A6): `subs(u,v) = center + ((R+r·cos v)·cos u, (R+r·cos v)·sin u,
  r·sin v)`; u = revolution, v = tube; v=0 is the outer equator.
- `vendor/truck/truck-shapeops/tests/boolean_m2.rs` — the fixture recipe
  (`placed_circle` + `disk_profile` + `arrange` + `extrude_profile`);
  read, do not edit. `truck-shapeops` already depends on `truck-evidence`
  (normal dep) — the D-ride test imports `truck_evidence::contact`
  directly.
- `vendor/truck/truck-evidence/tests/torus_pairs.rs` — the P11 ride's
  stratum/record recipe (`BoundedStratum::Face`,
  `ContactLocus::ValidatedBranchCover`); read, do not edit.

## Tests required (new file `tests/fillet_circle.rs`, dyadic witnesses only)

Fixtures: the landed cylinder via the boolean_m2 recipe (disk profile
R=2 → `extrude_profile` height 2). Primary witness: top rim, spec
center (0,0,2), edge_radius 2, radius 0.5.

1. `fillet_circle_top_rim` — the Finding 2 witness THROUGH THE ENTRY: 4
   faces, 3 unique edges, 3 unique vertices; exactly one `Torus` face
   (center (0,0,1.5), major 1.5, minor 0.5); the junction circles exact
   (r=2@z=1.5 shared by wall+torus; r=1.5@z=2 shared by torus+cap); the
   bottom cap untouched.
2. `fillet_circle_bottom_rim` — the Finding 3 witness: spec center
   (0,0,0), edge_radius 2, radius 0.5; torus center (0,0,0.5); 4/3/3.
3. `fillet_circle_both_rims` — the Finding 4 witness: both specs in ONE
   entry call; 5 faces, 4 unique edges, 4 unique vertices; two tori
   (centers (0,0,1.5) and (0,0,0.5)).
4. `fillet_circle_junction_certificate` — D6's machine-checks on the
   test-1 result (torus carrier exact; sampled junction points on both
   carriers; cap circle exact).
5. `fillet_circle_torus_face_rides_p11_pairs` — the Finding 6 witness:
   lift the RESULT's torus face into `contact()` as
   `BoundedStratum::Face { surface: CanonicalSurface::Torus(..),
   u_range: (0, TAU), v_range: (0, π/2) }` against a plane stratum at
   z = 1.75: the answer carries `ValidatedBranchCover` certified points,
   every point at z = 1.75 exactly, r̂ = 1.5 ± sqrt(0.1875) (both
   branches), on the torus at certification precision (use the precision
   YOU achieve, recorded; the probe achieved 1e-9).
6. `fillet_circle_multiwire_cap_refuses` — the Finding 5 witness: build
   the washer (4×4 square profile + hole circle r=1, extrude height 1 —
   the boolean_m2 `plate_with_hole_profile` recipe), spec center (0,0,1),
   edge_radius 1, radius 0.25 → `UnsupportedEnvelope(NonCanonicalCarrier)`
   at the D2 lift, budget untouched.
7. `fillet_circle_overflow_refuses` — radius 2 on the primary fixture
   (consumes the whole wall AND collapses the cap) → `Refusal::Empty`
   (machine-check the arm you got; record it).

## H-3 (house rule; V4 is a text gate on your diff)

No ADDED line carries a bare absolute small literal (`1e-N`) without the
same-line `// H-3` opt-out. Dyadic constants and geometry-derived values
only. Run `& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh
HEAD` before writing RESULT.json (bare `bash` is the WSL stub). CLIPPY
EVERY CHANGED FILE — run `cargo clippy --locked -p truck-shapeops
--all-targets` UNFILTERED and fix all findings BEFORE committing (five
prior packets each lost verify rounds to partial clippy runs).

## Done when

Commit on the current branch (subject
`BG-CAD-P12-BLEND: table 6.4 Circle->Torus row on the rewrite engine`)
BEFORE writing RESULT.json AT THE WORKTREE ROOT (that exact path — not
`loop/results/`), then, all green:

```
cargo check --locked -p truck-shapeops
cargo fmt --check -p truck-shapeops
cargo test --locked -p truck-shapeops --lib
cargo test --locked -p truck-shapeops --test fillet_circle
cargo test --locked -p truck-shapeops --test chamfer
cargo test --locked -p truck-shapeops --test fillet_pp
cargo test --locked -p truck-shapeops --test boolean_m2
cargo test --locked -p truck-shapeops --test interior_loop
cargo test --locked -p truck-shapeops --test resew
cargo test --locked -p truck-shapeops --test cut_boundaries
cargo test --locked -p truck-shapeops --test split_plane
cargo clippy --locked -p truck-shapeops --all-targets
& "C:\Program Files\Git\bin\bash.exe" scripts/kernel-gates.sh HEAD
```

All landed suites (`chamfer`, `fillet_pp`, `boolean_m2`, `interior_loop`,
`resew`, `cut_boundaries`, `split_plane`) must pass UNCHANGED. The lib
suite's `healing::tests::step_import` failure is the recorded
environmental one (fails at base, V5 knows).

## Forbidden

- Do not edit `boolean/**`, `section.rs`, `Cargo.toml`, or anything
  outside `write_allow`.
- Do not add `Refusal`/`EnvelopeCase`/`UnresolvedWitness`/`Collapse` arms
  (D5).
- Do not attempt D7's booked follow-ups (oblique caps, concave/annular
  fixtures, chains, canals, F3).
- GATE-3/H-4: `Face::debug_new` is banned in added lines.
- Do not write the tolerance-migration constructor name in prose or
  comments (GATE-4 counts raw text, comments included).
- No instrumentation traces may survive in the committed diff.

## Stop conditions

- `ANCHOR_MISMATCH` — report the measured count, change nothing.
- `SPEC_GAP` — a decision contradicts the tree; QUESTION.md with the
  empirical proof.
- A booked happy-path case (tests 1-4) fails `Solid::try_new` after a
  D3-faithful construction — stop and report the closure witness
  verbatim (the probe's Finding 2 structure is the reproducibility
  witness).
- The P11 ride (test 5) does not produce `ValidatedBranchCover` points on
  the closed-form branches through the landed dispatcher — stop and
  report the actual outcome verbatim.

RESULT.json: `{"id":"BG-CAD-P12-BLEND","status":"DONE","contracts":[...],
"tests_added":7,"deviations":[...],"notes":"..."}` — every deviation with
your derivation; deviations are expected to be RIGHT.
