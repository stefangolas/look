# The B-rep generation API — shape and contract

This is a reference for the **landed** public API of the kernel's B-rep
generation pipeline: what a caller invokes, what the types mean, what
every refusal says, and where the v1 envelope ends. Every signature below
was re-derived from the tree (the loop's rule: a claim a doc makes about
the tree must be reproducible by command).

Status: post-M2 (session 39, `integration/kernel-bg`). Companion docs:
`SOLVER_FAMILY_PLAN.md` (the approved design — books the *future* API and
the parallel graph), `GENERATION_KERNEL_BUILD_SPEC.md` (the contract
backlog the loop built this from), `FORMAL_SYSTEM_BREP_GENERATION.md`
(the mathematics). When this doc and the code disagree, the code wins;
file the discrepancy.

## 1. The evidence contract — every fallible call

```rust
// truck_base::evidence
pub type Outcome<T> = Result<Certified<T>, Refusal>;

pub struct Certified<T> { pub value: T, pub cert: Certificate }

pub struct Budget { pub subdiv: u32, pub newton: u32, pub depth: u32 }
```

Nothing in the pipeline panics on bad input and nothing returns an
uncertified maybe-answer. A call either returns `Ok(Certified<T>)` (the
value plus the certificate that produced it) or a typed `Refusal`. The
`Budget` is threaded through every stage that can recurse or iterate;
exhaustion is `NumericallyUnresolved`, never a hang.

## 2. Module map

| crate | module | role |
|---|---|---|
| `truck-geometry` | `recognize` | canonical-carrier recognition (witnesses) |
| `truck-geometry` | `arrange` | 2-D profile arrangement (M1 stage 1) |
| `truck-modeling` | `extrude` | arrangement → extruded `Solid` (M1 stage 2) |
| `truck-evidence` | `contact` | certified contact between bounded strata |
| `truck-evidence` | `analytic` | exact intersection carriers (`ExactCurve`, …) |
| `truck-shapeops` | `boolean` | the Boundary Rewrite: algebra, splitter, classifier, assembler |
| `truck-base` | `evidence` | `Outcome`/`Certified`/`Refusal`/`Budget`/`Prop` |

The topology types (`Solid`, `Shell`, `Face`, `Wire`, `Edge`) are
`truck_topology`'s, parameterized over the canonical geometry types
`Curve`/`Surface` from `truck_geometry::canonical`.

## 3. The boolean entry

```rust
// truck_shapeops::boolean::assemble
pub fn boolean(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>>
```

The regularized Boolean of two **single-shell** solids over canonical
carriers. Internally: lift every face/edge to a canonical bounded
stratum → sweep certified contact events over AABB-screened cross-solid
pairs → split → classify → decide → sew. The acceptance gate is
`Solid::try_new` — a returned `Ok` solid is closed, manifold, and
connected (or the empty solid: zero shells, e.g. an all-discarded
Difference). The sweep itself (`sweep_contact_events`) is `pub(crate)`:
callers who already hold a certified event list compose the stages
directly (§5).

Measured ground truth (the M2 flagship, `tests/boolean_m2.rs`): the 4×4
block minus the concentric r=1 disk extrude is congruent to the direct
`Extrude(P−Q)` construction — 7 faces both sides, 256/256 per-point grid
agreement; Intersection is the 3-face cylinder; Union is 8 faces in both
argument orders.

## 4. The decision algebra

```rust
// truck_shapeops::boolean
pub enum BoolOp { Union, Intersection, Difference, Xor }
impl BoolOp { pub fn eval(&self, s: State) -> bool }

pub struct State { pub in_a: bool, pub in_b: bool }

pub struct MaterialState4 { pub a_minus: bool, pub a_plus: bool,
                            pub b_minus: bool, pub b_plus: bool }

pub enum FragmentDecision { Keep { flip: bool }, Discard }

pub fn fragment_decision(op: BoolOp, m: MaterialState4) -> FragmentDecision
```

The §13.1 primitive: evaluate the truth function on both sides of a
boundary fragment; keep iff the sides differ; orient (`flip`) toward the
empty side. No case enumeration — the regularized-Boolean orientation
table (including the coincident-fragment cells: `A∪A=A`, `A−A=∅`, …)
falls out of the rule. This is pure logic; it touches no shapes.

## 5. The stages — composable with your own events

```rust
// truck_shapeops::boolean::split
pub fn split_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    events: &[ContactEvent],
    tol: f64,
) -> Outcome<FragmentMesh>

// truck_shapeops::boolean::classify
pub fn classify_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    tol: f64,
) -> Outcome<FragmentClassification>
```

The mesh vocabulary (all `pub` in `boolean::split`):

- `ContactEvent { record: ContactRecord, lhs: StratumRef, rhs: StratumRef }`
  — one certified contact, with the strata that produced it.
- `StratumRef` — `Face { solid, index }` or `Edge { solid, face, edge }`;
  `SolidRef` — `A` or `B`.
- `FragmentMesh { fragments, adjacency, coincident, … }` — the divided
  faces (`Fragment`: a `Face` plus its `FragmentOrigin`), the shared-edge
  adjacency graph (`FragmentAdjacency` with `AdjacencyParity`
  `Same`/`Flip`), and the coincident-face pairing (`CoincidentPair` with
  `CoincidentOrientation` `Identical`/`Anti`).
- `FragmentClassification` — per-fragment inside-other bits
  (`inside_other`), consumed by the decision table.

`classify_fragments` verifies its own parity graph: an under-split or
self-contradictory mesh refuses `Contradictory`, not a wrong answer.

## 6. The constructive path (M1 — no 3-D Boolean)

```rust
// truck_geometry::arrange
pub fn arrange(profile: &[Curve],
               domain: Option<BoundingBox<Point2>>) -> Outcome<Arrangement>

// truck_modeling::extrude
pub fn extrude_profile(profile: &[Curve],
                       arrangement: &Arrangement,
                       height: f64) -> Outcome<Solid>
```

Profile curves (v1: analytic lines and circles) → arrangement (winding
and nesting decide material; the hole's wall is built as an annulus with
two boundary wires) → direct extrusion to a valid B-rep. v1 scope
(documented refusals, not silent clamps): dyadic vertices; the algebraic
intersection-vertex case refuses.

## 7. Recognition — witnesses, not types

```rust
// truck_geometry::recognize
pub fn recognize_curve(c: &Curve) -> CanonicalCarrierWitness
pub fn recognize_surface(s: &Surface) -> CanonicalCarrierWitness

pub enum CanonicalCarrierWitness {
    ExactCanonical { carrier: CanonicalCarrier, map: CanonicalParamMap },
    Derived { carrier: CanonicalCarrier,
              provenance: ConstructionWitness, map: CanonicalParamMap },
    Unrecognized,
}
```

Lines/circles and planes/cylinders/cones/spheres/torus are recognized
exactly (or derived with a construction witness); anything else is
`Unrecognized`, which the boolean pipeline surfaces as
`UnsupportedEnvelope(NonCanonicalCarrier)` at the lift boundary — before
any contact work is attempted.

## 8. The contact substrate

```rust
// truck_evidence::contact
pub enum BoundedStratum { Face { surface: CanonicalSurface,
                                 u_range: (f64, f64), v_range: (f64, f64) }, /* Edge */ … }

pub fn contact(lhs: &BoundedStratum, rhs: &BoundedStratum,
               budget: &mut Budget) -> Outcome<ContactComplex>

pub fn face_stratum(witness: CanonicalCarrierWitness,
                    u_range: (f64, f64), v_range: (f64, f64))
    -> Result<BoundedStratum, Refusal>
```

`contact` decides identity/overlap (equal canonical carriers),
analytic FF intersections (circles, lines, points), FE/EE coincident
intervals, and defers the not-yet-implemented reductions. Its records
(`ContactRecord { dimension, kind, locus }`, `ContactLocus`) are what
`ContactEvent` carries into the splitter. Callers composing their own
event lists (§5) build them from `contact` output.

## 9. The refusal taxonomy

| `Refusal` | means |
|---|---|
| `Empty` | the operation's domain was empty |
| `UnsupportedEnvelope(case)` | the input is outside the certified envelope — see cases below |
| `NumericallyUnresolved { spent, witness }` | budget exhausted; `witness` names what could not be certified (containment, root isolation, Krawczyk indeterminacy, fillet contact curve, deviation) |
| `CompositionMarginExhausted(..)` | composition consumed the topological stability margin |
| `InputOutsideBackwardBudget(..)` | the input violates the repair budget |
| `Contradictory(witness)` | the evidence contradicts itself (e.g. an under-split mesh's parity graph) |
| `Collapsed(..)` | the exact object collapsed — certified, but not a realisation |
| `ForwardToleranceExceeded { bound, allowed }` | a forward error bound exceeded what was certifiable |

`EnvelopeCase`: `ChartDegenerate`, `ReachTooSmall`, `NonCanonicalCarrier`,
`NonPositiveNurbsWeight`, `ContactReductionDeferred` (the family of
not-yet-implemented contact reductions — also the boolean entry's
catch-all for its documented folds, below).

## 10. The v1 envelope — deliberate, typed, and tested

The following refuse rather than guess (each is asserted by a landed
test or recorded in the plan's named follow-up families):

- **Multi-shell inputs** to `boolean()` (the RW-MULTISHELL cavity fold).
- **Self-pair inputs** (`boolean(A, op, A)`): the self-pair sweep's
  intra-solid adjacency events are an event class no well-posed
  cross-solid input produces; both `A∪A` and `A−A` currently refuse
  `UnsupportedEnvelope(ContactReductionDeferred)` (measured; the
  idempotence *algebra* is pinned at the `fragment_decision` level).
- **Partial-overlap coincident faces and EE-butt joins** (RW-COPLANAR),
  tangency parity + fragment merge (RW-TANGENT), branch-cover arc
  continuation (RW-ARC-CONT), conic split curves (RW-CONIC).
- **Fragments that straddle the other solid's boundary** (the
  through-hole family): the classifier's recorded limitation.
- The insertion tolerance is a fixed class (shared with the splitter and
  classifier); it is never a caller- or test-tunable lever.

## 11. A minimal end-to-end call

```rust
use truck_base::evidence::Budget;
use truck_geometry::arrange::arrange;
use truck_geometry::canonical::Curve;
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::{assemble::boolean, BoolOp};

fn plate_with_hole_by_boolean(
    block: (Vec<Curve>, /* arrangement */),
    disk: (Vec<Curve>, /* arrangement */),
) -> truck_base::evidence::Outcome<truck_topology::Solid<
        truck_base::cgmath64::Point3, Curve,
        truck_geometry::canonical::Surface>> {
    let a = extrude_profile(&block.0, &block.1, 2.0)?.value;
    let b = extrude_profile(&disk.0, &disk.1, 2.0)?.value;
    boolean(&a, BoolOp::Difference, &b, &mut Budget::new(1000, 1000, 1000))
}
```

Match on the `Outcome`: `Ok(cert) => cert.value` is the validated solid;
`Err(refusal)` is one of §9. (The real, runnable form of this example is
`truck-shapeops/tests/boolean_m2.rs`; the construction helpers live in
`split.rs`'s test module.)

## Maintenance

This doc describes landed code; it goes stale silently. Anyone editing
the pipeline's public surface should re-derive every signature here by
command (the anchors habit: `grep` the `pub fn` lines, don't trust
prose) and update this file in the same change. The M2 battery
(`tests/boolean_m2.rs`) is the executable form of §3's claims — if it
fails, this doc's §3/§10 claims moved with it.
