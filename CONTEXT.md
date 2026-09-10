# CONTEXT.md - mechanically generated from the tree at dispatch time.
# Signatures, callers, tests only. No claims. Regenerated per dispatch.

## WRITE: truck123d/src/binding.rs
L108   struct   Bracket - A closed two-sided bracket `[lo, hi]`. The only numeric shape an export
L117   struct   MalformedRow - A malformed input row (a caller defect, distinct from a kernel refusal).
L126   enum     BindingError - The two typed failure modes of an export: a marshaled kernel refusal, or a
L138   struct   ConeRow - A certified normal-cone row: the anchor direction and the certified upper
L145   impl     ConeRow
L147   fn       to_cone - The landed certified-funnel cone value this row records.
L160   struct   BooleanRow - The boolean-dispatch row: the carrier-class pair and mode of the facade
L176   struct   BooleanOutcome - The observed verdict of one boolean pair through the runtime twin.
L201   struct   VolumeRow - The volume-facts row: an admitted construction's recorded patch shape (the
L213   struct   VolumeOutcome - The certified volume fact of one admitted patch.
L233   struct   TrimRow - The trim-facts row: the polynomial density net and the algebraic trim net
L245   struct   TrimOutcome - The certified bracket of one algebraically trimmed domain.
L278   fn       register_certified_entries - Registers the certified funnel engines into the `truck-evidence`
L288   fn       boolean_dispatch - The certified boolean dispatch: the facade mirror's runtime twin. The
L336   fn       volume_facts - The certified volume-facts entry: assemble the recorded patch row and run
L366   fn       trim_facts - The certified trim-facts entry for the algebraic-trim path: run the landed
L412   fn       binding_boolean_dispatch - The pyo3 export of [`boolean_dispatch`].
L420   fn       binding_volume_facts - The pyo3 export of [`volume_facts`].
L428   fn       binding_trim_facts - The pyo3 export of [`trim_facts`].
L476   struct   TrimExtrudeRow - The spline-trimmed extrude row: the base profile plus the recorded closed
L508   struct   TrimCrossingRecord - One certified trim crossing record: the crossing is a `TrimCrossing` node
L520   struct   TrimExtrudeOutcome - The certified outcome of one spline-trimmed extrude.
L537   struct   TrimExtrudeFacts - The realized facts of one trim-prism row (the executor's shape): the
L555   impl     PlaneBasis
L937   fn       trim_extrude - The composition: extrude the profile, cut it by the recorded spline trim,
L1057  fn       trim_extrude_solid - The realized facts of one trim-prism row: the certified volume value plus
L1096  fn       binding_trim_extrude - The pyo3 export of [`trim_extrude`].
tests: cone, canonical_row, canonical_volume_row, canonical_trim_row, verdict, binding_exports_the_certified_boolean_entry, kernel_refusals_marshall_to_typed_door_vocabulary, canonical_pair_end_to_end_through_the_export, binding_is_deterministic_bit_identical_rerun, square_profile, inner_square_trim, quarter_disk_trim_net, spline_trim_extrude_volume_matches_recorded_reference, trim_crossings_are_certified_nodes_not_coordinates, self_crossing_trim_loop_refuses_typed, full_extrude_without_trim_answers_bit_identically

## WRITE: truck123d/src/bd_bridge.rs
L67    enum     LatheEdge - The census-recorded curve of one lathe profile edge, in the `y = 0`
L87    enum     ProfileEdge - The census-recorded curve of one profile edge of a general authoring arm
L101   enum     SolidSpec - The solid carrier of one construction row.
L224   struct   RotationFrame - The recorded full orthonormal placement rotation of one part (the frame
L233   impl     RotationFrame
L255   struct   PartSpec - One placed construction row: a solid plus its world frame.
L292   struct   BooleanNode - One recorded boolean row: the mode and the two placed operand nodes.
L306   enum     TreeNode - A node of the submitted construction tree: either one placed solid (a
L326   struct   Facts - The measured facts of a submitted construction tree.
L1218  struct   PrismGeom - The local bounding box and volume of an exact prism extruded from a closed
L1228  fn       prism_geom
L1254  fn       prism_bbox - The exact local AABB of an extruded prism: the profile loop's support along
L1635  fn       tree_facts - Measures the submitted tree with the recorded OCC top-node semantics.
L1750  type     Triangle - One triangle: nine `f64` coordinates (three `x y z` vertices), world
L1905  fn       prism_mesh - The local mesh of an extruded prism: the profile boundary swept between the
L2330  fn       write_tree_stl - Writes the tree's triangle soup as a binary STL file at `path`, returning
L2376  fn       bd_facts - The pyo3 measurement entry: takes the construction tree JSON and returns
L2407  fn       bd_stl - The pyo3 export entry: writes the construction tree's STL to `path` and
tests: part, mirrored_part, line_lathe, primitive_facts_are_analytic, lathe_facts_match_occt_frustum_volume, partial_arc_lathe_refuses_typed, stl_writer_emits_binary_stl, drop_in_module_answers_census_vocabulary_name_for_name, unsupported_carrier_refusal_maps_to_the_refused_exception_class, dome_shell_profile, span_volume_by_quadrature, spline_segment_volume_matches_independent_quadrature, spline_shell_facts_are_not_a_polygon_flattening, spline_profile_bbox_covers_reconstructed_extrema, line_profile_lathe_volume_is_bit_identical_via_profile_form, line_loop3, square, prism_facts_are_exact_prism_arithmetic, loft_volume_matches_the_segment_moment_derivation, closed_halo_loft_certifies_its_seam_and_refuses_open_mismatch, mirror_placed_carrier_transforms_facts_without_recomputing_geometry, corpus_ttc_dir, run_door_python, placed_frame_facts_match_unplaced_facts_under_rigid_motion, plane_frame_extrude_answers_world_facts, pos_placed_assembly_counts_solids, vector_surface_answers_direction_math, revolve_about_nonz_axis_answers_world_facts, z_revolve_rows_answer_bit_identically, revolve_refuses_unsupported_axes_typed, span_derivative, bridge_path_query, spline_path_position_and_tangent_answer_exactly, sweep_line_path_records_loft_chain, sweep_spline_path_records_stations, z_revolve_and_loft_rows_answer_bit_identically, boolean, loft_carrier, revolved_carrier, canonical_carrier

## WRITE: corpus/ttc/door.py

## READ: loop/results/CG-BINDING.json (MISSING at dispatch time)

## READ: docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
L119   enum     ClaimVerdict - A proposition about an object that already exists.
L126   type     Construction - The outcome of an attempt to construct an object.
L128   struct   Refusal
L155   trait    CertifiedPatch
L167   trait    CertifiedPatchC2 - Required by R2, R7, and the contact classifier. NOT required by R1 tracing,
L173   trait    CertifiedPatchC3 - Required only by the A2 cusp classifier. Takes a BOX, not a point:
L277   struct   Frame3
L280   struct   SpineFrameRecipe
L290   enum     Spine
L312   enum     FrameLaw
L336   enum     ProfileLaw
L348   enum     SamplingPolicy
L384   struct   EdgeSampleLedger
L399   struct   ManifoldDiagnostics
L422   struct   SpineFrameSurface
L752   struct   ContactCert
L836   struct   Canal
L939   struct   TopologyClaim
L945   fn       certify_claimed

## READ: docs/TTC_CENSUS_FINAL.md

