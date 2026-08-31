# BG-CG-009-BREP — the parametric realization: SpineFrameSurface, SpineFrameCurve, and the authored-topology sweep constructor

```yaml
id:          BG-CG-009-BREP
contract:    [BG-CG-009-BREP]
class:       design
crates:      [truck-geometry, truck-modeling, truck-stepio, truck-shapeops, truck-meshalgo, truck-topology]
depends_on:  [BG-CG-000-CONTRACT, BG-CG-001-RECIPE, BG-CG-002-FRAMES-ANALYTIC, BG-CG-003-TRANSPORT, BG-CG-004-FACET]
write_allow:
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/decorators/spine_frame.rs
  - vendor/truck/truck-geometry/src/decorators/mod.rs
  - vendor/truck/truck-geometry/tests/spine_frame_brep.rs
  - vendor/truck/truck-modeling/src/spine_sweep.rs
  - vendor/truck/truck-modeling/src/lib.rs
  - vendor/truck/truck-modeling/tests/spine_sweep_conformance.rs
  - vendor/truck/truck-stepio/src/in/mod.rs
  - vendor/truck/truck-stepio/src/in/step_geometry/geom_impls.rs
  - vendor/truck/truck-shapeops/src/section.rs
  - vendor/truck/truck-shapeops/src/boolean/classify.rs
  - vendor/truck/truck-shapeops/src/boolean/split.rs
  - vendor/truck/truck-meshalgo/src/tessellation/triangulation.rs
  - vendor/truck/truck-meshalgo/src/tessellation/formal/xmonotone.rs
  - vendor/truck/truck-modeling/src/builder.rs
  - vendor/truck/truck-modeling/src/cad.rs
  - vendor/truck/truck-modeling/src/extrude.rs
  - vendor/truck/truck-modeling/src/revolve.rs
  - vendor/truck/truck-modeling/src/until.rs
  - vendor/truck/truck-topology/src/face.rs
  - vendor/truck/truck-topology/src/invariants/wedge.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_PLAN.md
  - vendor/truck/truck-geometry/src/canonical.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/recipe.rs
  - vendor/truck/truck-geometry/src/constructive/frame_fixed.rs
  - vendor/truck/truck-geometry/src/constructive/frame_up.rs
  - vendor/truck/truck-geometry/src/constructive/frame_radial.rs
  - vendor/truck/truck-geometry/src/constructive/frame_transport.rs
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/decorators/coons.rs
  - vendor/truck/truck-geometry/src/decorators/intersection_curve.rs
  - vendor/truck/truck-modeling/src/facet_sweep.rs
  - vendor/truck/truck-modeling/src/builder.rs
  - vendor/truck/truck-topology/src/shell.rs
tests_required:
  - surface_variant_forwarding_all_landed_methods
  - curve_variant_forwarding_all_landed_methods
  - spine_frame_surface_evaluates_the_recipe
  - trajectory_curve_matches_surface_offset
  - search_parameter_newton_recovers_station_and_vertex
  - transform_of_surface_refuses_or_composes_typed
  - prism_sweep_assembles_closed_solid
  - side_faces_share_trajectory_edges_by_identity
  - tessellation_density_does_not_change_topology
  - nonplanar_cap_refuses_typed
  - convex_prism_volume_matches_analytic
  - step_out_refuses_spine_frame_variants_typed
budget:      {turns: 55, ctx_tokens: 140000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub enum Surface' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub enum Curve' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'SpineFrameSurface' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A4, expect: 0, cmd: "grep -c 'SpineFrameCurve' vendor/truck/truck-geometry/src/canonical.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'spine_frame' vendor/truck/truck-geometry/src/decorators/mod.rs"}
  - {id: A6, expect: 0, cmd: "grep -c 'spine_sweep' vendor/truck/truck-modeling/src/lib.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn try_attach_plane' vendor/truck/truck-modeling/src/builder.rs"}
  - {id: A8, expect: 1, cmd: "grep -c 'pub struct SpineFrameRecipe' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
  - {id: A9, expect: 1, cmd: "grep -c 'pub struct CoonsSurface' vendor/truck/truck-geometry/src/decorators/coons.rs"}
```

## What this packet is

The last packet of the CG program: the parametric realization (build-spec
§8B, quoted where load-bearing) and the authored-topology sweep constructor.
Two closed-enum variants land TOGETHER so this release is the last breaking
one (the `ExtrudedCurve` precedent — its RESERVED comment in `canonical.rs`
is the pattern to follow for macro lists and forwarding).

**Spec (§8B.1–8B.3, normative):** for a profile with k edges, construct k
side BREP faces plus optional caps — never one face per spine sample. Each
side face realizes the same `X(s,v) = C(s) + frame(s)·P(s,v)` continuously.
The trajectory of profile vertex p_j, `E_j(s) = X(s, p_j)`, is represented
ONCE and shared by the two adjacent side faces. No sewing. Reuse the keyed
entity-cache pattern of the facet backend's grid registry (§3.3) for any
construction-time sharing. **Tessellation density MUST NOT change BREP
topology.**

## Section 1 — the enum ripple (canonical.rs, integrator-owned)

Add to `pub enum Surface` and `pub enum Curve`:

```rust
/// The parametric spine/profile realization surface (BG-CG-009-BREP).
/// Realizes `X(s,v) = C(s) + frame(s)·P(s,v)` over the landed recipe
/// evaluators. One side face of a spine sweep; v-runs along a profile edge.
SpineFrameSurface(SpineFrameSurface<...>),
/// The trajectory of one fixed profile point under the recipe,
/// `E(s) = X(s, p)`. Shared by adjacent side faces; never re-derived.
SpineFrameCurve(SpineFrameCurve<...>),
```

The `...` payload types are the decorator structs of Section 2 (canonical.rs
already sees `decorators` types — follow how `CoonsSurface`/`IntersectionCurve`
are referenced there). Extend the `derive_curve_method!` /
`derive_surface_method!` macro invocations exactly as the `ExtrudedCurve`
comment records was done for that variant. `truck-modeling/src/geometry.rs`
re-exports `canonical::*` and `decorators::*` by glob — no edit needed there;
`truck-stepio`'s out-direction and every hand-written exhaustive match site
gets its arm per Section 4.

## Section 2 — the decorators (NEW spine_frame.rs)

`SpineFrameSurface` over the landed recipe types
(`truck_geometry::constructive::*`): holds the spine (a landed `Spine`
impl), the frame law, and the profile-edge window `[v0, v1]` of one profile
edge. Constructors validate to `DirectTolerance::position` and refuse via
`ConstructError` (this module may return `Result<_, ConstructError>` — it is
geometry-local; the `Outcome` mapping happens at modeling's entry, CG-007's
pattern).

Trait checklist (§8B.4 audit list; follow the landed `CoonsSurface`
checklist shape — read `decorators/coons.rs` first, it landed the same
discipline): `ParametricSurface, ParametricSurface3D, BoundedSurface,
ParameterDivision2D, SearchParameter<D2>, Invertible, Transformed<Matrix4>,
IncludeCurve`. Derivatives: S_v is analytic (the profile law is linear in v
along an edge — landed `profile.rs`); S_s uses the spine derivative plus the
frame evaluator — central differences at `DirectTolerance::parameter` scale
are sanctioned for the SEARCH path only (SearchParameter is a numerical
search; certificates never quote them). `Transformed` composes the transform
into the surface (a stored `Matrix4`, Coons/IntersectionCurve pattern) — if
any trait method cannot be implemented honestly for the stored-transform
form, refuse typed (`ConstructError::InvalidInput`) and record the boundary;
do not approximate.

`SpineFrameCurve` holds the same recipe fragment plus the fixed profile
point `p`: evaluates `E(s) = X(s, p)`. Checklist:
`ParametricCurve3D, BoundedCurve, SearchParameter<D1>, Invertible,
Transformed<Matrix4>, IncludeCurve`. Its `SearchParameter` delegates to the
host surface's `SearchParameter` restricted to the vertex line (the landed
`IntersectionCurve` Newton pattern — read `decorators/intersection_curve.rs`).

## Section 3 — the constructor (NEW truck-modeling/src/spine_sweep.rs)

```rust
/// The authored-topology sweep constructor (build-spec §8B; plan §4 CG-009).
/// Side faces per profile edge, trajectory edges shared by identity,
/// caps via `try_attach_plane`. No sewing anywhere.
pub fn spine_sweep<S: Spine>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
) -> Outcome<Solid<Point3, Curve, Surface>>
```

V1 domain (pre-decided; outside it, typed refusals — never silent clamping):

- `ProfileLaw::Constant` and `ProfileLaw::Scale` (non-through-zero uniform
  scale) with STRAIGHT profile edges only. `LinearCorrespondence` accepted
  when both profiles are straight-edged with identical edge counts (each ring
  edge is the lerp of two straight edges — straight). Curved profile edges,
  through-zero scale, and mismatched correspondence refuse
  (`ConstructError::InvalidInput` at the entry) — booked boundary, not a bug.
- Spine: any landed `Spine` (the C1 gate is the landed `PolylineSpine`
  refusal). Frame laws: all four landed laws (their singularity refusals are
  the landed ones).
- Stations: sorted, deduped, at least two (the landed facet entry's
  validation pattern; reuse its checks verbatim in shape).

Assembly (the thesis — authored topology, no sewing): for profile edge j,
one side `Face` on `SpineFrameSurface` over `[s_first, s_last] × [v0_j,
v1_j]`, its wire built from the shared `Edge` objects: `SpineFrameCurve`s
E_j and E_{j+1} constructed ONCE each and cloned into both adjacent faces'
wires (same `Edge` handle — identity, not coordinates); ring edges at the
first/last stations are `Line`s (straight profile edges under rigid frames /
uniform scale stay straight — assert this in a test). Caps: planar start/end
rings via the landed `try_attach_plane` (builder.rs); a nonplanar cap ring
refuses typed (`nonplanar_cap_refuses_typed`). Final assembly through the
landed `Solid::try_new` path (validation lives there; `Face::debug_new` is
BANNED in added lines — GATE-3/H-4; use the same fragment-construction the
landed boolean assembler uses).

The landed `facet_sweep` is untouched (V8 identity; the facet backend is the
rendering fast path, this is the topology stage).

## Section 4 — the downstream ripple (arms only)

The 14 files in `write_allow` under stepio/shapeops/meshalgo/modeling/
topology hold hand-written `match`es over `Curve`/`Surface` (census taken at
packet-writing time). For each the compiler proves non-exhaustive
(`cargo check --workspace --all-targets` names them): ADD the minimal arms —
delegating to the forwarded macro method where the enum forwarding covers
the behavior, and for STEP-in: the new variants cannot arrive from STEP
(refuse typed, the CG-007 entry's vocabulary is NOT in scope here — a plain
typed error per the landed stepio refusal style). NO logic changes in these
files; a repair needing more than arms is a stop condition. STEP-OUT of the
new variants refuses typed (`step_out_refuses_spine_frame_variants_typed`) —
STEP writing of sweep surfaces is TR-NRB-001's booked business, not ours.

House rules: H-3 float-literal opt-out is same-line only (`// H-3`).
`#![deny(clippy::unwrap_used)]` in both new files; match-based unwraps only.
Transforms of circle/torus-carrying solids panic in debug builds (the landed
trap) — your fixtures use the NEW variants and line/SpineFrameCurve edges,
so this does not bite; do not add fixtures that transform analytic-carrier
solids through `Solid::mapped`.

## Done-when

- `cargo fmt --all -- --check` clean.
- `cargo clippy -p truck-geometry -p truck-modeling -p truck-stepio
  -p truck-shapeops -p truck-meshalgo -p truck-topology --all-targets
  --message-format=short --no-deps` — zero findings.
- `cargo test -p truck-geometry --lib --tests`, `cargo test
  -p truck-modeling --lib --tests`, and the same for topology/stepio green.
- `cargo check --workspace --all-targets` green.

## Stop conditions

Stop, commit nothing beyond WIP evidence, write RESULT.json (AT THE WORKTREE
ROOT) with the finding verbatim if:

1. A macro-forwarding site in `canonical.rs` cannot take the new variants
   mechanically (the ExtrudedCurve precedent fails to extend) — name the
   macro and the method.
2. A downstream repair needs logic, not an arm.
3. `Solid::try_new` refuses the authored shell for reasons traceable to the
   shared-edge construction (orientation pairing: every shared edge appears
   with OPPOSITE orientations in its two faces — the P12 fixture rule; check
   your wire directions before suspecting the constructor).

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

Commit your work on the current branch (subject: `feat(geometry):
parametric realization and authored sweep (BG-CG-009-BREP)`) BEFORE writing
`RESULT.json`. All tests above are contract; `tests_required` names must
exist verbatim.
