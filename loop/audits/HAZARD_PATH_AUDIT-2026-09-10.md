# HAZARD-PATH AUDIT — the exact code paths to fill for battery green

2026-09-10, orchestrator. Maps all 38 hazard fixtures onto the four surfaces
(door.py shim -> bd_bridge.rs arms -> binding.rs exports -> certified funnel)
and names, per fixture, the path that must exist, its state, and the packet
that fills it. State keys: LANDED (green today), CHAIN (fills via
BRIDGE-BOOLEANS / BRIDGE-LOFT-FACTS / TRIM-EXTRUDE-CTOR, all queued),
PACKET (needs a new arm packet), DOCTRINE (typed is the permanent correct
verdict).

## State after the current chain completes (projected)

| Fixture | Path to green | State |
|---|---|---|
| apex_revolve | revolve arm (landed) | **LANDED** |
| c0_knot_profile | polyline prism arm (landed) | **LANDED** |
| scale_span_shock | compound arm (landed) | **LANDED** |
| prototype_reuse / mirror_twins / nested_compounds / frame_composition_chain | compound + frame placement (landed) | **LANDED** |
| cylinder_wrap, seam_split, deep_cavity_cut, small_feature_large_origin | boolean dispatch, canonical x canonical (cell-7 class) through binding_boolean_dispatch | CHAIN (BRIDGE-BOOLEANS) |
| rational_heavy_spline | loft facts arm via binding_volume_facts (spline sections) | CHAIN (BRIDGE-LOFT-FACTS) |
| near_tangency | transversality certificate returns Disproven-with-certified-gap -> typed | CHAIN (BRIDGE-BOOLEANS); typed is the designed verdict |
| zero_thickness_residual | non-manifold promotion refusal marshaled typed | CHAIN; typed permanent |
| boolean_of_boolean, long_chain | depth-1 dispatch rule -> typed | CHAIN; typed until the composition cell closes (FSSI-LAYER decision) |
| disjoint_fuse | empty-result class -> typed | CHAIN; typed permanent |
| twisted_loft_stations | station correspondence (CC-013) not exported -> typed | PACKET (correspondence export) or honest typed |
| rear_wing/suspension/steering_rack class | TRIM-EXTRUDE-CTOR | CHAIN (queued) |

Count projection after the chain: **~15 green, ~5 typed-permanent, ~18
staged**.

## The staged class -> the packets that fill it

### P1. CONTACT-EXPORT (fixtures: sphere_plane_kiss, coaxial_equal_cylinders,
equal_cylinders_perpendicular, coplanar_face_fuse, sliver_faces)
The tangential/coincident class. The classified-contact funnel is landed
(CL-004/005, CC-020) but not exported; CG-BINDING exported the transversal
path only. Fill: a `binding_contact_dispatch` export routing
tangential/coincident pairs to the contact funnel, plus the facade routing
arm that sends a tangential pair there instead of
`ContactReductionDeferred`. ~500-1,000 LOC. Note: the kiss fixture's honest
verdict may remain typed (measure-zero result emission), but the CONTACT
decision must be exported either way — today it refuses with the WRONG
typed reason.

### P2. SHELL-ARM (box_shell, cylinder_shell, spline_loft_shell,
variable_thickness_offset, thicken_sheet)
Export the landed CC strata (CC-021 k=1/2/3, CC-023 shell certificate,
CC-026 thickness, CC-031 variable radius) as a shell/thicken entry:
solid + thickness + open-face spec -> certified strata -> facts + realized
mesh. `spline_loft_shell` is the hard sub-case (strata over spline faces —
may stay typed first pass). `offset_self_intersect` stays typed (doctrine).
~600-1,000 LOC.

### P3. DRAFT-ARM (drafted_extrude)
Tapered prism: profile affine-scaled along s. For polygon profiles this is
exact linear algebra in the existing prism arm — a parameter, not a new
theory. ~150-300 LOC.

### P4. SECTION-ARM (section_slice)
Plane-solid cross-section: the landed FSSI-004 loci + plane-intersection
classes exported as a section entry returning the section face/wire.
~300-500 LOC.

### P5. SWEEP-EXT (helical_sweep, twisted_sweep)
Helix = sweep along a helical spine (deck discipline under rotation — the
spec's deck_max gate); twist = the frame-transport law with a recorded
rotation profile (SWEEP-PATH's FrameData pattern extended). ~300-600 LOC.

### P6. PROJECTION-ARM (project_curve_to_face)
Curve-on-surface projection: a new residual consumer (R-family member:
point-on-surface + direction constraint — square, C1, in-family). ~300-500
LOC.

### P7. SCALE-ARM (nonuniform_scale)
Non-rigid affine placement: exact for polygon/prism carriers (linear map);
for rational spline carriers exact as a reparametrization+reweight. New
placement-carrier class in the frame law. ~200-400 LOC.

### P8. CHAMFER-ASYM (asymmetric_chamfer)
Two-setback chamfer over the landed chamfer arm. ~100-200 LOC.

### P9. BLEND-ON-SPLINE (fillet_spline_edge)
Fillet on non-canonical faces: the CC blend machinery is canonical-carrier
scoped; arming it to spline faces is the hardest arm on the board. ~500-1,000
LOC. Expect typed first pass.

### P10. CHAIN-CLOSURE (boolean_of_boolean lifted, long_chain lifted)
Theory-gated: the FSSI-LAYER splitter decision + trimmed-carrier admission.
Explicitly NOT booked with a LOC number — this is the transition-matrix
cell flagged in DOOR-GAP-AUDIT.

### P0 (free). pattern_composition
Client-side unrolling: a pattern is N sequential depth-1 cuts — the shim can
unroll and dispatch each at depth 1. Zero kernel work; green after
BRIDGE-BOOLEANS by rewriting the fixture's dispatch shape.

## Totals

- After the current chain: ~15 green / ~5 typed-permanent / ~18 staged.
- After P1-P9: ~33-35 green, ~5 typed-permanent (doctrine), remainder
  staged pending P10.
- Remaining LOC beyond the current chain: **~2,700-4,700 across 9 packets**,
  of which P1 (contact) and P2 (shell) carry the most corpus value, and P9
  (blend-on-spline) is the highest-risk arm.
