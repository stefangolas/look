# QUESTION.md — BG-SOL-S2-PCURVE

## Proposed amendment to `docs/SOLVER_FAMILY_PLAN.md`

### 1. §4 Phase 2, "pcurves" line

The landed S2 extrude (`truck-modeling/src/extrude.rs`) returns a
`Solid<Point3, Curve, Surface>` whose edges all carry `PC = ()`. This packet
(PBG-SOL-S2-PCURVE) proved empirically that this is a **structural
impossibility, not a missing implementation**:

- `Wire<P, C>` holds `VecDeque<Edge<P, C>>` (`truck-topology/src/lib.rs:137-139`),
  and `Edge<P, C>` defaults `PC` to `()` (`lib.rs:125-130`).
- `with_pcurve<Q>` returns `Edge<P, C, Q>` (`edge.rs:529`), so a real pcurve
  produces `Edge<P, C, PCurve<...>>`, which cannot occupy a `Wire`'s
  `VecDeque<Edge<P, C, ()>>`. A scratch compile probe confirms it: a
  `Wire<Point3, Curve>` cannot be built from
  `Edge<Point3, Curve, PCurve<Line<Point2>, Plane>>` (E0277,
  `From` not implemented).
- `PC` appears nowhere above `Wire`; it is erased at the Wire boundary.

**Amend the plan** so §4 Phase 2 no longer books "pcurves" as deliverable by S2
on the returned `Solid`'s edges. State instead that delivering pcurves on the
`Solid`'s edges requires threading `PC` through
`Wire<P, C>` -> `Face<P, C, S>` -> `Shell<P, C, S>` -> `Solid<P, C, S>` — a
cross-crate topology-wide type change (every `Wire` use across meshalgo,
shapeops, modeling, stepio) that is its own packet or family. This matches the
spec's own BG-CE-001 record ("the packet that wires real pcurves owns trace
splitting").

### 2. §7 M1, "exercises ... pcurves" milestone

The M1 construction as landed (S1 arrange + S2 direct extrude) builds the
correct closed, connected, outward-oriented solid but attaches no pcurves. The
topology DOES already exercise the pcurve **carrier** and its invariant layer:
`PCurve<C, S>` exists (`truck-geometry/src/decorators/mod.rs:222`), and
truck-topology's BG-INV-001 same-parameter checker certifies pcurve-carrying
edges (`invariants/same_parameter.rs`, `same_parameter_exact_pcurve_edge_holds`,
`same_parameter_offset_pcurve_edge_violates`) on standalone edges.

**Amend the plan** so M1's "exercises pcurves" is recorded as satisfied by the
S2 construction plus the topology's pcurve carrier / same-parameter invariant
existing, OR re-scope that milestone to the future PC-threading program above.
Either way, the plan should not imply that M1's delivered `Solid` carries
pcurves on its edges.
