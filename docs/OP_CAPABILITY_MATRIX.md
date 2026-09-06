# Op Capability Matrix - machine-checked per-(op x carrier-class) verdicts (PB-013-OP-CAPABILITY-MATRIX)

**Status:** LANDED by work packet PB-013-OP-CAPABILITY-MATRIX.

This matrix turns the PB-010-TTC-PARITY-AUDIT op census
(`loop/results/PB-010-TTC-PARITY-AUDIT.json`, 93 site rows) into one annotated cell per
(op x carrier-class) pair, then makes every cell executable: `truck123d/tests/pb_op_matrix.rs`
drives the facade/compat entry for each cell and asserts the annotated `current` verdict
against live behavior (the audit being corrected by execution, per the packet). The cell set is
derived from the census `op` fields; every census row maps to exactly one cell and the mapping is
asserted in the test. Verdict vocabulary is closed and matches the compat surface: `certified` /
`refuses(<EnvelopeCase or reason>)` / `client-layer` / `unavailable`.

Determinism: cell order below is fixed (the doc order); no hash iteration appears.

## Verdicts

| # | cell | current | flipped-by | target | surface | census rows |
|---:|---|---|---|---|---|---|
| 1 | fuse(swept,swept) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/surfaces.py:fuse_all; f1/src/lib/engine_cover.py:fuse_cover; f1/src/lib/engine_cover.py:build_airbox_fuse; f1/src/lib/details.py:fuse_pod; f1/src/lib/drivetrain.py:_fuse; f1/src/lib/power_unit.py:fuse_pu |
| 2 | fuse(swept,canonical) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/suspension.py:_fuse; f1/src/lib/mono_tub.py:fuse_proud |
| 3 | cut(swept,swept) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/engine_cover.py:cut_cover; f1/src/lib/rear_wing.py:cut_endplate; f1/src/lib/details.py:cut_mirror; f1/src/lib/mono_tub.py:cut_tub |
| 4 | cut(revolved,canonical) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/wheels.py:_cut |
| 5 | cut(swept,canonical) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/surfaces.py:cut; f1/src/lib/rear_wing.py:boolean_crank; f1/src/lib/suspension.py:_cut; f1/src/lib/floor.py:cut_floor; f1/src/lib/drivetrain.py:_cut; f1/src/lib/engine_cover.py:louvres_cut; f1/src/lib/details.py:dzus_cut; f1/src/lib/drivetrain.py:gearbox_cut |
| 6 | intersect(swept,canonical) | refuses(NonCanonicalCarrier) | PB-011 | certified | S1 | f1/src/lib/surfaces.py:accent_stripe_solid; f1/src/lib/drivetrain.py:intersect; f1/src/lib/front_wing.py:accent_stripe |
| 7 | cut(canonical,canonical) | certified | - | certified | S1 | f1/src/lib/drivetrain.py:clevis_bore; f1/src/lib/rear_wing.py:pylon_cut |
| 8 | heal(boolean-result) | unavailable | - | unavailable | S1 | f1/src/lib/surfaces.py:repair |
| 9 | revolve(full,spline-profile) | certified | - | certified | S6 | falcon_heavy/src/lib/merlin_common.py:revolved_shell; falcon_heavy/src/lib/merlin_common.py:revolved_solid; falcon_heavy/src/lib/falcon_common.py:_tube_z; falcon_heavy/src/lib/falcon_common.py:_dome; falcon_heavy/src/lib/falcon_common.py:make_mvac_revolve; falcon_heavy/src/lib/falcon_common.py:make_fairing_shell; f1/src/lib/wheels.py:revolve_tyre |
| 10 | revolve(partial-arc,spline-profile) | unavailable | PB-011 | certified | S6 | falcon_heavy/src/lib/falcon_common.py:revolved_solid_barrel |
| 11 | sweep(spine,closed-section) | certified | - | certified | S6 | falcon_heavy/src/lib/merlin_common.py:tube |
| 12 | loft(spline-section) | certified | - | certified | S5 | f1/src/lib/surfaces.py:loft_solid; f1/src/lib/surfaces.py:body_loft; f1/src/lib/wheels.py:_loft; f1/src/lib/floor.py:loft_stack; f1/src/lib/mono_tub.py:loft_half_section; f1/src/lib/power_unit.py:loft_tube; f1/src/lib/mono_halo.py:loft_loop; f1/src/lib/cockpit.py:ruled_loft; f1/src/lib/front_wing.py:loft_cascade; f1/src/lib/nose.py:loft_nose; f1/src/lib/sidepods.py:loft_skin; f1/src/lib/sidepods.py:cavity_solid; f1/src/lib/floor.py:diffuser_loft |
| 13 | extrude(profile) | certified | - | certified | S6 | falcon_heavy/src/lib/merlin_common.py:extrude; f1/src/lib/engine_cover.py:extrude |
| 14 | fillet(edge-selector) | certified | - | certified | S6 | f1/src/lib/surfaces.py:safe_fillet; f1/src/lib/rear_wing.py:fillet_select |
| 15 | chamfer(edge-selector) | certified | - | certified | S6 | f1/src/lib/surfaces.py:safe_chamfer |
| 16 | mirror(axis-plane) | certified | - | certified | S6 | f1/src/lib/surfaces.py:mirror_y |
| 17 | author(spline-profile) | certified | - | certified | S5 | falcon_heavy/src/lib/merlin_common.py:profile_face; f1/src/lib/surfaces.py:airfoil_profile; f1/src/lib/suspension.py:loft_face |
| 18 | author(polyline-profile) | certified | - | certified | S4 | - |
| 19 | author(circle-profile) | unavailable | PB-011 | certified | none | f1/src/lib/cockpit.py:circle_section |
| 20 | validity-check | client-layer | - | client-layer | S7 | f1/src/lib/surfaces.py:selector_census |
| 21 | primitive(canonical-solid) | certified | - | certified | S3 | falcon_heavy/src/lib/merlin_common.py:cylinder; falcon_heavy/src/lib/merlin_common.py:torus; falcon_heavy/src/lib/merlin_common.py:sphere; falcon_heavy/src/lib/merlin_common.py:box; falcon_heavy/src/falcon_heavy_exploded.py:guide_cyl |
| 22 | compound(group) | certified | - | certified | S3 | falcon_heavy/src/lib/merlin_common.py:group_compound; falcon_heavy/src/lib/falcon_common.py:compound_from_instances; falcon_heavy/src/falcon_heavy.py:compound_vehicle; f1/src/lib/surfaces.py:as_body_compound; f1/src/lib/drivetrain.py:compound_solids; f1/src/lib/monocoque.py:group; f1/src/f1.py:assembly_add; f1/src/lib/cockpit.py:build_cockpit |
| 23 | label | client-layer | - | client-layer | S2 | falcon_heavy/src/lib/merlin_common.py:label_color; f1/src/lib/surfaces.py:styled; falcon_heavy/src/lib/falcon_common.py:_lab; falcon_heavy/src/lib/falcon_common.py:label_tank; falcon_heavy/src/falcon_heavy.py:label_vehicle |
| 24 | color | client-layer | - | client-layer | S2 | falcon_heavy/src/falcon_heavy_exploded.py:color; f1/src/lib/spec.py:color_palette; f1/src/lib/wheels.py:colour_tyre |
| 25 | placement | client-layer | - | client-layer | S2 | falcon_heavy/src/lib/merlin_common.py:rotate; falcon_heavy/src/falcon_heavy_exploded.py:moved_offset; f1/src/lib/rear_wing.py:rotate_placement; f1/src/lib/mono_halo.py:stud_placement; f1/src/lib/cockpit.py:plane_offset; f1/src/lib/floor.py:rotated_plane; f1/src/lib/surfaces.py:section_plane |
| 26 | step-export(constructive) | refuses(NonCanonicalCarrier) | - | refuses(NonCanonicalCarrier) | none | falcon_heavy/src/falcon_heavy_cutaway.py:step_out; f1/src/f1.py:step_out; f1/src/airbox.py:dispatch; f1/src/corner_fl.py:dispatch; f1/src/monocoque.py:dispatch |

## Notes

- The audit classifications mix row-stage and op-capability; a cell annotates the **capability** of
  the (op x carrier-class) pair, so a row staged-skip for a downstream swept boolean can map to a
  cell whose op is itself landed (e.g. loft(spline-section)).
- **G1 (swept-carrier boolean cells).** The census records no landed swept-pair boolean path; the
  typed refusal the facade answers today is the constructive-carrier envelope case
  (`NonCanonicalCarrier`) at the STEP boundary (TR-NRB-001; a swept part exports STL/GLB, never
  STEP). Each G1 cell is pinned by `swept_carrier_boolean_cells_refuse_typed_today` and flips to
  certified when PB-011 routes the swept-pair boolean.
- **G5 (partial-arc revolve).** SKIPS.json records falcon_heavy/cutaway under
  booleans-on-swept-carriers, but the vendored code path is a partial-arc revolve (revolution_arc +
  start rotate); the reason text is a census-level mis-description. The cell is annotated per the
  G5 finding (current unavailable, flipped-by PB-011, target certified).
- **G3 (circle-profile authoring).** S5 names Spline authoring only; a closed-circle profile carrier
  has no named facade row (missing-facade). author(circle-profile) is `unavailable` and flips when
  PB-011 lands the row.
- Refusal is never worked around: a cell annotated `refuses(...)` asserts the refusal, and PB-011 flips
  by editing this row + the asserted expectation together, one commit per flip (V5).

- **fuse(swept,swept)** - The census G1 class pinned by swept_carrier_boolean_cells_refuse_typed_today: a boolean union of two swept/lofted carriers has no landed geometry path; the typed refusal the facade answers today is the constructive-carrier NonCanonicalCarrier envelope (a swept product exports STL/GLB, never STEP - TR-NRB-001). PB-011 routes the swept-pair boolean.
- **fuse(swept,canonical)** - G1 class: union of a swept carrier with canonical tools (proud bosses onto the lofted tub shell; suspension wishbone-plate fusion) refuses the constructive-carrier envelope today.
- **cut(swept,swept)** - G1 class: subtract of two swept carriers (lofted cover skin minus lofted cavity/sill; lofted tub skin minus lofted cavity; lofted mirror shells) refuses the constructive-carrier envelope today.
- **cut(revolved,canonical)** - G1 class: rim/tyre/brake clearances cut out of revolved carriers (wheels._cut) refuse the constructive-carrier envelope today.
- **cut(swept,canonical)** - G1 class: canonical cutters against lofted/swept bases (surfaces.cut, suspension/floor/drivetrain bore clearances, louvre/pocket/casing cuts) refuse the constructive-carrier envelope today.
- **intersect(swept,canonical)** - G1 class: intersect of a swept body with a canonical slab/rbox (accent stripes, drivetrain casing rbox) refuses the constructive-carrier envelope today.
- **cut(canonical,canonical)** - Landed S1 canonical x canonical subtraction (clevis/disc bores, pylon rod bores). The census rows stay staged because their enclosing F1 rows fuse swept carriers elsewhere, but this cell's op form is landed and certifies.
- **heal(boolean-result)** - OCC ShapeFix healing (repair) has no kernel analogue and none is planned (ShapeFix analogues are an explicit non-goal); the helper only guards swept-carrier boolean output, so it inherits the G1 deferral without adding a landed path. No facade/compat row names it.
- **revolve(full,spline-profile)** - Full (360 deg) revolve of a spline/line profile over the landed S6 revolve row; canonical Falcon-Heavy rows run it end to end today (STL out).
- **revolve(partial-arc,spline-profile)** - Audit G5: the falcon_heavy/cutaway sectioning is a partial-arc revolve (revolution_arc + start rotate), mis-recorded in SKIPS as 'boolean sectioning'. No enrolled row exercises partial-arc revolve coverage today (its only census row is skip-listed), so the cell is annotated per the G5 finding: current unavailable, flipped-by PB-011, target certified.
- **sweep(spine,closed-section)** - The Merlin tube idiom (spline spine + closed circle section) runs on canonical rows through the landed sweep row (S6); the circle-section authoring form itself is the G3 missing-facade candidate tracked by author(circle-profile).
- **loft(spline-section)** - Loft over spline sections is landed (S5 spline authoring + CC-port loft); every census row in this cell is staged-skip only because its enclosing script boolean-composes the lofted result.
- **extrude(profile)** - Extrusion over a planar profile is landed S6 (Merlin thrust gussets run canonical; the F1 sill-tool extrude is a skip-row member whose block is downstream).
- **fillet(edge-selector)** - Fillet over the four-expression selector vocabulary is landed S6/S7; the census rows are staged because their enclosing F1 rows boolean-compose swept carriers.
- **chamfer(edge-selector)** - Chamfer is landed S6; the census row is staged because its enclosing F1 part row boolean-composes swept carriers.
- **mirror(axis-plane)** - Mirror about an axis-aligned plane is landed S6; the census row is staged because it runs only inside F1 part builders that are staged-skip rows.
- **author(spline-profile)** - Spline section/profile authoring is landed S5 (facade Spline row; PB-002 amended scope); profile_face feeds the canonical Falcon revolves, the F1 sections are skip-row members.
- **author(polyline-profile)** - Closed line-loop (polyline/polygon) profile authoring over the landed facade Polygon/Polyline rows; the census records it inside the mixed profile rows (Edge.make_line in profile_face, tube outlines) rather than as a dedicated row.
- **author(circle-profile)** - Audit G3 missing-facade: no named compat/facade row lists a closed-circle profile primitive (S5 names Spline authoring only); bd.Circle section authoring (cockpit ring stacks, Merlin tube) is unlistable today. Annotated per authoring_cells_match_surface_rows.
- **validity-check** - Corpus-side is_valid/BRepCheck + volume inspection is client-layer data (no kernel geometry op), matching the S7 selector/validity note.
- **primitive(canonical-solid)** - The canonical solid primitives Box/Cylinder/Sphere/Torus are landed S3; guide_cyl's row context is render-client annotation, but the primitive capability cell is certified.
- **compound(group)** - Compound/group composition over landed assembly emission (PB-006 emit_step_assembly covers node names in insertion order); the door answers compound_from_instances as a plain placement Compound on canonical rows.
- **label** - Shape.label assignment is an S2-class data row (zero kernel content); it rides as the GLB node name / STEP product name. Rows _label/_lab/styled also assign color (see the color cell).
- **color** - Shape.color is an S2/S8-class client record (zero kernel geometry); the GLB emission captures it as a linear baseColorFactor (PB-009). The verdict stays client-layer.
- **placement** - Plane/Location frame algebra (rotate, moved, Pos*Rotation*, plane.offset/rotated, section planes) is S2 recorded-client-layer data; the GLB node matrix carries the placement without kernel geometry.
- **step-export(constructive)** - STEP out of constructive/swept geometry is the TR-NRB-001 recorded boundary (facade ExportStep refuses NonCanonicalCarrier); cadgen's @step store is never reimplemented. The refusal persists (STL/GLB cover render).

## Machine-check contract

`truck123d/tests/pb_op_matrix.rs` parses the census JSON and this table, asserts that every census
row maps to exactly one cell (and the doc lists exactly that mapping in this order), and drives live
behavior per cell against the annotated `current` verdict.
