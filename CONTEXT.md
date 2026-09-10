# CONTEXT.md - mechanically generated from the tree at dispatch time.
# Signatures, callers, tests only. No claims. Regenerated per dispatch.

## WRITE: truck123d/src/bd_bridge.rs
L66    enum     LatheEdge - The census-recorded curve of one lathe profile edge, in the `y = 0`
L86    enum     ProfileEdge - The census-recorded curve of one profile edge of a general authoring arm
L100   enum     SolidSpec - The solid carrier of one construction row.
L189   struct   RotationFrame - The recorded full orthonormal placement rotation of one part (the frame
L198   impl     RotationFrame
L220   struct   PartSpec - One placed construction row: a solid plus its world frame.
L253   enum     TreeNode - A node of the submitted construction tree: either one placed solid (a
L268   struct   Facts - The measured facts of a submitted construction tree.
L1447  fn       tree_facts - Measures the submitted tree with the recorded OCC top-node semantics.
L2095  fn       write_tree_stl - Writes the tree's triangle soup as a binary STL file at `path`, returning
L2128  fn       bd_facts - The pyo3 measurement entry: takes the construction tree JSON and returns
L2151  fn       bd_stl - The pyo3 export entry: writes the construction tree's STL to `path` and
tests: part, mirrored_part, line_lathe, primitive_facts_are_analytic, lathe_facts_match_occt_frustum_volume, partial_arc_lathe_refuses_typed, stl_writer_emits_binary_stl, drop_in_module_answers_census_vocabulary_name_for_name, unsupported_carrier_refusal_maps_to_the_refused_exception_class, dome_shell_profile, span_volume_by_quadrature, spline_segment_volume_matches_independent_quadrature, spline_shell_facts_are_not_a_polygon_flattening, spline_profile_bbox_covers_reconstructed_extrema, line_profile_lathe_volume_is_bit_identical_via_profile_form, line_loop3, square, prism_facts_are_exact_prism_arithmetic, loft_volume_matches_the_segment_moment_derivation, closed_halo_loft_certifies_its_seam_and_refuses_open_mismatch, mirror_placed_carrier_transforms_facts_without_recomputing_geometry, corpus_ttc_dir, run_door_python, placed_frame_facts_match_unplaced_facts_under_rigid_motion, plane_frame_extrude_answers_world_facts, pos_placed_assembly_counts_solids, vector_surface_answers_direction_math, revolve_about_nonz_axis_answers_world_facts, z_revolve_rows_answer_bit_identically, revolve_refuses_unsupported_axes_typed

## WRITE: truck123d/src/facade.rs
L82    enum     ModeValue - The `Mode` algebra vocabulary (§3.2): build123d's `Add`/`Subtract`/
L95    enum     AxisValue - The axis vocabulary of `select_filter` and `mirror`.
L111   enum     CarrierClass - The boolean-carrier class of a solid — the carrier taxonomy the
L128   impl     CarrierClass
L131   fn       is_funnel_carrier - Whether the class is a swept carrier the certified funnel supports
L145   struct   SweptBooleanEvent - One routed swept-carrier boolean row of a facade session (PB-011 G1): a
L158   struct   CertifiedBooleanRoute - The certified route of one swept-carrier boolean pair into the landed
L175   struct   SweptBooleanRefusal - The typed, localized refusal of a swept-carrier boolean pair the certified
L189   enum     BooleanPairVerdict - The observed verdict of one boolean-carrier pair through the facade
L240   fn       dispatch_swept_carrier_boolean - Routes one boolean carrier pair through the landed certified entry
L326   enum     FacadeOp - One row of a submitted facade session. Every row is data only; the kernel
L483   struct   FacadeTable - The submitted session table: the ordered operation log a builder session
L489   const    REPORT_SCHEMA - The serialized `schema` tag of a facade report.
L495   struct   ExportEntry - One export entry of a facade report. Only in-envelope exports appear here:
L505   struct   FacadeReport - The deterministic ledger [`run_facade`] returns for an in-envelope table.
L527   fn       run_facade - The single native facade entry: takes the submitted table and returns the
L725   fn       facade_submit - The pyo3 wrapper over [`run_facade`]: parses the submitted table JSON,

## WRITE: corpus/ttc/door.py

## READ: loop/results/AUTHOR-FRAME-CARRIERS.json

## READ: loop/results/FRAME-REVOLVE.json (MISSING at dispatch time)

## READ: docs/TTC_CENSUS_FINAL.md

