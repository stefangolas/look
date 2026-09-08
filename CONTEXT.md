# CONTEXT.md - mechanically generated from the tree at dispatch time.
# Signatures, callers, tests only. No claims. Regenerated per dispatch.

## WRITE: vendor/truck/truck-certified/src/ssi_gate.rs (MISSING at dispatch time)

## WRITE: vendor/truck/truck-certified/src/ssi.rs
L107   enum     SsiRefusal - Why an SSI square-system or Krawczyk3 operation could not be certified.
L149   impl     SsiRefusal
L151   fn       tag - A short stable tag, for diagnostics.
L168   impl     From
L180   impl     From
L201   struct   FoldCert - The refusing-constructor carrier of a certified ordinary fold (FSSI-000
L211   impl     FoldCert
L214   fn       new - Construct a fold-verdict record, refusing a malformed one: a `sigma`
L245   impl     Tensor4
L749   fn       partial_enclosure - The certified partial-derivative enclosure of one stored component grid
L891   struct   SideBoxKey - A deterministic memoization key: one side's unit box (spec Corollary 1.4).
L909   struct   PerSideHullMemo - The deterministic per-side hull memo (spec Corollary 1.4): hulls keyed on
L913   impl     PerSideHullMemo
L959   fn       component_value_unit - The memoized per-side value enclosure of one component over a unit
L982   fn       distinct_entries - The number of distinct (side, box, function) hull entries held.
L996   fn       f3_diagonal_derivatives - Build the FROZEN [`SquareSystemInput`] of the reduced square system for a
L1032  fn       select_continuation_coordinate - Select the continuation coordinate by the FROZEN rule, verbatim.
L1213  fn       krawczyk3_certificate - Certify a unique root of the reduced square system on the slice
L1356  struct   RationalBipatch - A certified-admitted rational tensor-Bernstein patch (spline-admissible).
L1365  impl     RationalBipatch
L1368  fn       new - Construct a patch, refusing a degree-0 bidegree, empty or ragged
L1392  fn       m - Bidegree in the first parameter.
L1397  fn       n - Bidegree in the second parameter.
L1402  fn       numerator - The homogeneous numerator grids, `(x, y, z)` order.
L1407  fn       weights - The strictly positive weight grid.
L1414  enum     SsiParticipant - One side of a square-system construction.
L1436  fn       construct_square_system - Construct the square surface–surface difference system from two
L1504  fn       cone_overlap_did_not_shrink - Whether the certified cone-overlap outcome recorded at `later` failed to
L1512  struct   CascadeClassifyInput - The certified cascade inputs of one routed box: the `system`/`graph`/
L1534  enum     StagnationOutcome - The outcome of the CFP-008 stagnation route on one recorded subdivision
L1564  fn       route_stagnation - The CFP-008 route: when cone overlap does not shrink between subdivision
L1696  impl     Dense4
tests: binom, coord_grid, zero_grid, monomial_grid, parabola_system, from_rows, into_rows, at, second_difference_reference, second_partials_match_finite_difference_on_fixture, dot_exp, expansion_strictly_gt, separated_by_exact_dot, fc3_separation_exact_sign_certificate, intervals_agree, coefficient_hull, per_side_hull_set_identical_to_grid_hull_on_fixtures, eval_2d_float_test, minor_factorization_matches_direct_expansion, memoization_distinct_boxes_bounded, normal_net_degree_and_d3_count, ssi_fixture_tag, landed_ssi_fixture_verdicts_unchanged, refusal_tags_stable, fold_cert_refusing_constructor

## WRITE: vendor/truck/truck-certified/src/construct/bie/ssi4.rs
L108   struct   Ssi4Parameters - The restricted-pair solver parameters: the certified geometry scale, the
L135   impl     Default
L153   struct   CertifiedChartCurve - A certified interaction-curve branch in the 4-D product chart (frozen
L162   impl     CertifiedChartCurve
L164   fn       is_unresolved - Whether the branch is empty but typed unresolved (never a guess).
L173   struct   ChartSample - One certified sample of an interaction branch: the parameter cell (the
L193   enum     RestrictedChart - A restricted-pair carrier chart: one of the pole-free sweep / canonical
L223   impl     RestrictedChart
L225   fn       from_plane - A plane carrier from a landed [`Plane`] (its own origin/axis basis).
L234   fn       from_sphere - A sphere carrier from a landed [`Sphere`].
L243   fn       from_cylinder - A cylinder carrier from a landed [`Cylinder`] (canonical axis-aligned
L253   fn       circular_sweep - A pole-free circular-section sweep carrier from the windowed straight-
L283   struct   CircularSweepUnit - The closed-form pole-free circular-section sweep of the restricted normal
L306   impl     CircularSweepUnit
L422   impl     RestrictedChart
L800   struct   FForm - The restricted-pair F-form: `F(x) = X_A(u, v) − X_B(s, t)` over the 4-D
L807   impl     FForm
L809   fn       residual_f - The float residual at the 4-D chart point.
L816   fn       partial_columns_f - The float 3×4 Jacobian columns `(X_u, X_v, −X_s, −X_t)` at the point.
L823   fn       residual_iv - The interval residual over the 4-D box (outward-rounded).
L830   fn       partial_columns_iv - The interval 3×4 Jacobian columns over the 4-D box.
L841   fn       second_columns_f - The float second partial columns: `out[a][j]` is the 3-vector
L859   fn       second_columns_iv - The interval second partial columns over the 4-D box (outward-rounded):
L904   struct   Ssi3System - The N=3 Krawczyk system over the F-form with one product coordinate fixed:
L913   impl     Ssi3System
L915   fn       new - Builds the square system over the F-form with `axis` fixed at `value`.
L965   impl     KrawczykSystem
L1002  struct   Ssi4System - The N=4 Krawczyk system over the F-form augmented by the hyperplane
L1011  impl     Ssi4System
L1013  fn       new - Builds the augmented N=4 system.
L1018  impl     KrawczykSystem
L1186  fn       minor_sign_expansion - The (R′) minor-sign predicate at a point: the exact sign of the 3×3
L1205  fn       minor_det_sign_iv - The (R′) box-level minor sign: the certified sign of the 3×3 determinant
L1221  fn       minor3_of_jacobian - The 3×3 float minor of a 3×4 float Jacobian (given as columns) after
L1238  fn       minor3_iv_of_jacobian - The interval 3×3 minor of a 3×4 interval Jacobian (given as columns) after
L1260  fn       choose_free_axis - The closed-form transversal column choice (scope decision 5): try all four
L1280  fn       choose_free_axis_over_box - The box-level transversal column choice: the free axis whose interval 3×3
L2174  fn       certify_restricted_pair - The certified restricted-pair solve: seed every boundary stratum, trace each
L2291  struct   RestrictedPairSolver - The certified restricted-pair engine (BIE-002) behind the
L2483  impl     RestrictedPairSolver
L2541  impl     truck_evidence
L2623  fn       register_restricted_pair_solver - Registers the certified restricted-pair engine into the `truck-evidence`
L3186  impl     CountingSystem
L3207  impl     KrawczykSystem
tests: plane_sphere_form, sweep_plane_form, must_certified, column_choice_finds_transversal_subset, minor_sign_predicate_matches_expansion, boundary_seed_resolves_exf_and_fxe, continuation_tracks_known_curve, unresolved_elsewhere_is_typed, ensure_engine_registered, circle_sweep_fixture, circle_ring_sweep, sweep_sweep_pair_fixture, fixture_plane, sweep_stratum_fixture, plane_stratum_fixture, entry_trait_implemented_by_certified_engine, funnel_closes_sweep_pair_end_to_end, sweep_sweep_pair_dispatches, new, fresh, last_y, f_point, jacobian, preconditioner, straight_branch_form, straight_branch_tangent, straight_branch_point, straight_branch_system, identity4, certified_with_cell, cube_cell, preconditioner_reused_across_consecutive_samples, preconditioner_invalidated_on_inclusion_failure, fc5_branch_fixture, battery_run, verdicts_bit_identical_with_reuse_enabled, fc5_branch_fixture_prune_count_unchanged, curve, eps_inflation_sequence_is_fixed_and_bounded, eps_inflation_recovers_marginal_sample

## WRITE: vendor/truck/truck-certified/tests/ssi_gate_conformance.rs (MISSING at dispatch time)

## READ: docs/FSSI_BUILD_SPEC.md

## READ: vendor/truck/truck-certified/src/ (MISSING at dispatch time)

## READ: vendor/truck/truck-evidence/src/ (MISSING at dispatch time)

## CALLER SITES (grep of defining names outside their file)
SsiRefusal  vendor\truck\truck-certified\src\patch_admit.rs:45  use crate::ssi::{RationalBipatch, SsiRefusal};
SsiRefusal  vendor\truck\truck-certified\src\patch_admit.rs:91  fn from(_: SsiRefusal) -> Self {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:72  //! [`SsiRefusal`] wraps the landed named causes verbatim (D-reuse): class
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:99  /// [`SsiRefusal::TangentCurveSuspected`] and
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:100  /// [`SsiRefusal::CoincidentPatchSuspected`]. They are refusals (the gate
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:214  pub fn new(chart: usize, sigma: i8, det_enclosure: (f64, f64)) -> Result<Self, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:217  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:220  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:283  fn partial_axis(&self, axis: usize) -> Result<Tensor4, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:287  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:363  fn partial2_axis(&self, j: usize, l: usize) -> Result<Tensor4, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:365  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\patch_admit.rs:45  use crate::ssi::{RationalBipatch, SsiRefusal};
SsiRefusal  vendor\truck\truck-certified\src\patch_admit.rs:91  fn from(_: SsiRefusal) -> Self {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:72  //! [`SsiRefusal`] wraps the landed named causes verbatim (D-reuse): class
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:99  /// [`SsiRefusal::TangentCurveSuspected`] and
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:100  /// [`SsiRefusal::CoincidentPatchSuspected`]. They are refusals (the gate
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:214  pub fn new(chart: usize, sigma: i8, det_enclosure: (f64, f64)) -> Result<Self, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:217  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:220  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:283  fn partial_axis(&self, axis: usize) -> Result<Tensor4, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:287  return Err(SsiRefusal::InvalidInput);
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:363  fn partial2_axis(&self, j: usize, l: usize) -> Result<Tensor4, SsiRefusal> {
SsiRefusal  vendor\truck\truck-certified\src\ssi.rs:365  return Err(SsiRefusal::InvalidInput);
tag  vendor\truck\truck-certified\src\bvh.rs:74  pub fn tag(self) -> &'static str {
tag  vendor\truck\truck-certified\src\certified_map.rs:11  //! Pre-made decisions (packet tags; do not relitigate):
tag  vendor\truck\truck-certified\src\certified_map.rs:114  pub fn tag(self) -> &'static str {
tag  vendor\truck\truck-certified\src\contract.rs:92  /// `Method`-tagged (H-6).
tag  vendor\truck\truck-certified\src\contract.rs:126  /// The method tag is fixed at `Method::Interval`: interval work only, a
tag  vendor\truck\truck-certified\src\contract.rs:151  /// The method tag. Fixed at `Method::Interval` in the freeze.
tag  vendor\truck\truck-certified\src\contract.rs:375  /// division, keeping the `Method::Interval` tag this freeze pins.
tag  vendor\truck\truck-certified\src\hull.rs:9  //! Pre-made decisions (packet tags; do not relitigate):
tag  vendor\truck\truck-certified\src\hull.rs:57  pub fn tag(self) -> &'static str {
tag  vendor\truck\truck-certified\src\pair_dispatch.rs:36  //! # Pre-made decisions (packet tags; do not relitigate)
tag  vendor\truck\truck-certified\src\pair_dispatch.rs:46  //! variant, [`PairUnsupported::UnsupportedPairClass`], tag
tag  vendor\truck\truck-certified\src\patch_admit.rs:5  //! # Pre-made decisions (packet tags; do not relitigate)
tag  vendor\truck\truck-evidence\tests\torus_pairs.rs:276  .expect("equal dyadic torus carriers decide at the identity stage");
tag  vendor\truck\truck-evidence\src\contact\fe_ee.rs:2  //! (Edge Ã— Face) and EE (Edge Ã— Edge) funnel stages (plan Â§4 Phase 3).
tag  vendor\truck\truck-evidence\src\contact\gff.rs:416  // validated stage (spec §3).
tag  vendor\truck\truck-evidence\src\contact\gff.rs:486  ///   granted (the F-C6 stagnation class).
tag  vendor\truck\truck-evidence\src\contact\gff.rs:536  /// singular/cascade stage is unreachable from it (decision 3), and the gff
tag  vendor\truck\truck-evidence\src\contact\implicit.rs:4  //! The Contact Layer's general validated FF stage (offset mixed quadrics,
tag  vendor\truck\truck-evidence\src\contact\implicit2d.rs:2  //! stage (Theorem 4, `docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3).
tag  vendor\truck\truck-evidence\src\contact\implicit2d.rs:685  /// Returns `None` when the stage does not apply (route through the old
tag  vendor\truck\truck-evidence\src\contact\implicit2d.rs:805  /// The verdict of one reduction over an `h` net on the local chart (stage
tag  vendor\truck\truck-evidence\src\contact\implicit2d.rs:1243  fn implicit_stage_reduces_plane_spline_theorem4() {
tag  vendor\truck\truck-evidence\src\contact\instrument.rs:24  //! [`InstrumentCounters::stage5_pair_class`] is the histogram population (the
tag  vendor\truck\truck-evidence\src\contact\instrument.rs:63  /// Pair-type histogram count at stage-5 entry (this crate's owned counter).
From  vendor\truck\truck-certified\src\ssi.rs:25  //! From two certified-admitted rational tensor-Bernstein patches (control
From  vendor\truck\truck-certified\tests\kernel_claims.rs:303  // (D6: no From<ClaimedGraph> for CertifiedGraph), so a Boolean requiring
From  vendor\truck\truck-certified\src\domain\ambient.rs:264  witness: PeriodWitness::InheritedFromGeneratrix {
From  vendor\truck\truck-certified\src\formal\ambient.rs:663  Self::DeclaredValueDiffersFromCertifiedGenerator { .. } => {
From  vendor\truck\truck-certified\src\formal\ambient.rs:1547  CountingProcedure::StructuralFromResolvedType,
From  vendor\truck\truck-certified\src\formal\ambient.rs:1799  PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
From  vendor\truck\truck-certified\src\formal\ambient.rs:2110  witness: PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:64  //! [`SourceConformance::RecoveredFromMalformedSource`], so a recovery from a
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:137  RecoveredFromMalformedSource(NonconformantRepair),
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:198  SourceConformance::RecoveredFromMalformedSource(repair)
From  vendor\truck\truck-certified\src\formal\envelope.rs:300  CountingProcedure::StructuralFromResolvedType
From  vendor\truck\truck-certified\src\formal\quotient.rs:384  basis: DeckLabelBasis::InheritedFromParent,
From  vendor\truck\truck-evidence\src\fid\lfs.rs:74  /// From [`curvature_radius_lower`]; `+inf` permitted (flat cell).
From  vendor\truck\truck-evidence\src\num\parallelotope.rs:21  //! 1. **θ (predict).** From a certified point `p_k` on the branch and its unit
From  vendor\truck\truck-certified\src\ssi.rs:25  //! From two certified-admitted rational tensor-Bernstein patches (control
From  vendor\truck\truck-certified\tests\kernel_claims.rs:303  // (D6: no From<ClaimedGraph> for CertifiedGraph), so a Boolean requiring
From  vendor\truck\truck-certified\src\domain\ambient.rs:264  witness: PeriodWitness::InheritedFromGeneratrix {
From  vendor\truck\truck-certified\src\formal\ambient.rs:663  Self::DeclaredValueDiffersFromCertifiedGenerator { .. } => {
From  vendor\truck\truck-certified\src\formal\ambient.rs:1547  CountingProcedure::StructuralFromResolvedType,
From  vendor\truck\truck-certified\src\formal\ambient.rs:1799  PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
From  vendor\truck\truck-certified\src\formal\ambient.rs:2110  witness: PeriodContradictionWitness::DeclaredValueDiffersFromCertifiedGenerator {
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:64  //! [`SourceConformance::RecoveredFromMalformedSource`], so a recovery from a
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:137  RecoveredFromMalformedSource(NonconformantRepair),
From  vendor\truck\truck-certified\src\formal\cylinder_band.rs:198  SourceConformance::RecoveredFromMalformedSource(repair)
From  vendor\truck\truck-certified\src\formal\envelope.rs:300  CountingProcedure::StructuralFromResolvedType
From  vendor\truck\truck-certified\src\formal\quotient.rs:384  basis: DeckLabelBasis::InheritedFromParent,
From  vendor\truck\truck-evidence\src\fid\lfs.rs:74  /// From [`curvature_radius_lower`]; `+inf` permitted (flat cell).
From  vendor\truck\truck-evidence\src\num\parallelotope.rs:21  //! 1. **θ (predict).** From a certified point `p_k` on the branch and its unit
FoldCert  vendor\truck\truck-certified\src\ssi.rs:222  Ok(FoldCert {
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2427  let ok = FoldCert::new(3, 1, (0.5, 2.0)).expect("a well-formed record admits");
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2432  FoldCert::new(3, -1, (0.5, 2.0))
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2446  FoldCert::new(chart, sigma, det_enclosure),
FoldCert  vendor\truck\truck-certified\src\ssi.rs:222  Ok(FoldCert {
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2427  let ok = FoldCert::new(3, 1, (0.5, 2.0)).expect("a well-formed record admits");
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2432  FoldCert::new(3, -1, (0.5, 2.0))
FoldCert  vendor\truck\truck-certified\src\ssi.rs:2446  FoldCert::new(chart, sigma, det_enclosure),
new  vendor\truck\truck-certified\src\bvh.rs:31  //! packet adds no second cache. (5) Zero new top-level evidence kinds: a
new  vendor\truck\truck-certified\src\bvh.rs:111  pub fn new(axes: [CertifiedInterval; 3]) -> Result<Self, BvhRefusal> {
new  vendor\truck\truck-certified\src\bvh.rs:162  pub fn new(
new  vendor\truck\truck-certified\src\bvh.rs:260  let mut nodes = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:473  let mut candidates = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:474  let mut prune_records = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:476  let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
new  vendor\truck\truck-certified\src\bvh.rs:555  let hint = match FloatHint::new(direction) {
new  vendor\truck\truck-certified\src\bvh.rs:741  let box_ = EnclosureBox::new(axes)?;
new  vendor\truck\truck-certified\src\bvh.rs:760  SpanLeaf::new(patch.cell, box_, net)
new  vendor\truck\truck-certified\src\bvh.rs:822  EnclosureBox::new([
new  vendor\truck\truck-certified\src\bvh.rs:848  let box_ = EnclosureBox::new([
new  vendor\truck\truck-evidence\src\bspline.rs:337  let bsp = BSplineCurve::new(KnotVec::bezier_knot(2), vec![Point3::new(0.0, 0.0, 0.0); 3]);
new  vendor\truck\truck-evidence\src\bspline.rs:379  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:382  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:383  Point3::new(-0.5, 0.5, -0.5),
new  vendor\truck\truck-evidence\src\bspline.rs:384  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:393  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:396  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:397  Point3::new(-1.0 / 3.0, -1.0 / 3.0, -1.0 / 3.0),
new  vendor\truck\truck-evidence\src\bspline.rs:398  Point3::new(-2.0 / 3.0, -2.0 / 3.0, -2.0 / 3.0),
new  vendor\truck\truck-evidence\src\bspline.rs:399  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:408  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:411  Point3::new(0.0, 0.0, 0.0),
Tensor4  vendor\truck\truck-certified\src\ssi.rs:283  fn partial_axis(&self, axis: usize) -> Result<Tensor4, SsiRefusal> {
Tensor4  vendor\truck\truck-certified\src\ssi.rs:338  Ok(Tensor4 { degrees, rows: out })
Tensor4  vendor\truck\truck-certified\src\ssi.rs:363  fn partial2_axis(&self, j: usize, l: usize) -> Result<Tensor4, SsiRefusal> {
Tensor4  vendor\truck\truck-certified\src\ssi.rs:412  fn hull_tensor4(t: &Tensor4, box_axis: [(f64, f64); 4]) -> Result<CertifiedInterval, SsiRefusal> {
Tensor4  vendor\truck\truck-certified\src\ssi.rs:1833  let tensor = Tensor4::from_grid(&grid, degrees);
Tensor4  vendor\truck\truck-certified\src\ssi.rs:1866  let z_tensor = Tensor4::from_grid(z_grid, degrees);
Tensor4  vendor\truck\truck-certified\src\tangency\chart.rs:466  /// counterpart of the landed `Tensor4::partial_axis`).
Tensor4  vendor\truck\truck-certified\src\tangency\minors.rs:25  //! coefficient grid (the pattern of the SSI `Tensor4` / kernel `Grid4`,
partial_enclosure  vendor\truck\truck-certified\src\ssi.rs:749  pub fn partial_enclosure(
partial_enclosure  vendor\truck\truck-certified\src\ssi.rs:1007  let enc = partial_enclosure(system, i, *axis, box_)?;
partial_enclosure  vendor\truck\truck-certified\src\ssi.rs:1177  let enc = partial_enclosure(system, row, retained[col], box4)?;
partial_enclosure  vendor\truck\truck-certified\src\ssi.rs:1199  ///    over `X` ([`partial_enclosure`]); the determinant enclosure is their
partial_enclosure  vendor\truck\truck-certified\src\ssi.rs:1263  *cell = partial_enclosure(system, row, retained[col], center_box)?;
partial_enclosure  vendor\truck\truck-certified\src\ssi_trace.rs:542  let enc = partial_enclosure(system, component, axis, box_).map_err(map_ssi_refusal)?;
partial_enclosure  vendor\truck\truck-certified\src\kernel\tracer.rs:670  fn partial_enclosure(
partial_enclosure  vendor\truck\truck-certified\src\kernel\tracer.rs:706  *cell = partial_enclosure(sys, r, c, box_)?;
partial_enclosure  vendor\truck\truck-certified\src\tangency\a2.rs:98  use crate::ssi::{partial_enclosure, SsiRefusal};
partial_enclosure  vendor\truck\truck-certified\src\tangency\a2.rs:584  Row::Grid(c) => partial_enclosure(system, *c, axis, *b).map_err(map_ssi),
partial_enclosure  vendor\truck\truck-certified\src\tangency\chart.rs:30  //! partials ([`partial_enclosure`]) over the pivot's rows and columns; the
partial_enclosure  vendor\truck\truck-certified\src\tangency\chart.rs:535  let a00 = crate::ssi::partial_enclosure(system, rows[0], ya, b)?;
SideBoxKey  vendor\truck\truck-certified\src\ssi.rs:891  pub(crate) struct SideBoxKey {
SideBoxKey  vendor\truck\truck-certified\src\ssi.rs:910  cache: std::collections::BTreeMap<SideBoxKey, CertifiedInterval>,
PerSideHullMemo  vendor\truck\truck-certified\src\ssi.rs:909  pub(crate) struct PerSideHullMemo {
PerSideHullMemo  vendor\truck\truck-certified\src\ssi.rs:2194  let mut memo = PerSideHullMemo::default();
PerSideHullMemo  vendor\truck\truck-certified\src\ssi.rs:909  pub(crate) struct PerSideHullMemo {
PerSideHullMemo  vendor\truck\truck-certified\src\ssi.rs:2194  let mut memo = PerSideHullMemo::default();
component_value_unit  vendor\truck\truck-certified\src\ssi.rs:649  fn component_value_unit(
component_value_unit  vendor\truck\truck-certified\src\ssi.rs:959  pub(crate) fn component_value_unit(
component_value_unit  vendor\truck\truck-certified\src\ssi.rs:1152  let hull = component_value_unit(system, component, unit)?;
component_value_unit  vendor\truck\truck-certified\src\ssi.rs:2057  let per_side = component_value_unit(system, component, cell)
component_value_unit  vendor\truck\truck-certified\src\ssi.rs:2225  .component_value_unit(&system, 0, cell)
distinct_entries  vendor\truck\truck-certified\src\ssi.rs:982  pub(crate) fn distinct_entries(&self) -> usize {
distinct_entries  vendor\truck\truck-certified\src\ssi.rs:2238  let entries = memo.distinct_entries() as u64;
f3_diagonal_derivatives  vendor\truck\truck-certified\src\ssi.rs:52  //! (the fixture kit's documented identity pairing). [`f3_diagonal_derivatives`]
f3_diagonal_derivatives  vendor\truck\truck-certified\src\ssi.rs:996  pub fn f3_diagonal_derivatives(
f3_diagonal_derivatives  vendor\truck\truck-certified\src\ssi.rs:1037  let input = f3_diagonal_derivatives(system, continuation_axis, box_)?;
f3_diagonal_derivatives  vendor\truck\truck-certified\tests\ssi_system.rs:191  let certified = match f3_diagonal_derivatives(system, continuation_axis, SELECTION_BOX) {
select_continuation_coordinate  vendor\truck\truck-certified\src\contract.rs:376  pub fn select_continuation_coordinate(
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi.rs:1029  /// `contract::select_continuation_coordinate` exactly: largest relative
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi.rs:1032  pub fn select_continuation_coordinate(
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi.rs:1038  crate::contract::select_continuation_coordinate(&input).map_err(SsiRefusal::from)
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi.rs:1222  select_continuation_coordinate(system, continuation_axis, box_)?;
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi_trace.rs:671  /// ([`select_continuation_coordinate`], never a local re-implementation); the
select_continuation_coordinate  vendor\truck\truck-certified\src\ssi_trace.rs:681  let coordinate = select_continuation_coordinate(system, axis, box_).map_err(map_ssi_refusal)?;
select_continuation_coordinate  vendor\truck\truck-certified\tests\contract_freeze.rs:170  select_continuation_coordinate(&system).expect("coordinate 1 certifies away-from-zero");
select_continuation_coordinate  vendor\truck\truck-certified\tests\contract_freeze.rs:215  let refusal = select_continuation_coordinate(&system)
select_continuation_coordinate  vendor\truck\truck-certified\tests\contract_freeze.rs:258  select_continuation_coordinate(&none_certified),
select_continuation_coordinate  vendor\truck\truck-certified\tests\contract_freeze.rs:259  select_continuation_coordinate(&none_certified),
select_continuation_coordinate  vendor\truck\truck-certified\tests\ssi_system.rs:211  match select_continuation_coordinate(system, continuation_axis, SELECTION_BOX) {
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi.rs:1213  pub fn krawczyk3_certificate(
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi.rs:2293  match krawczyk3_certificate(system, axis, box_) {
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi_admit.rs:25  //! path is [`krawczyk3_certificate`] (instantiated, never extended —
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi_admit.rs:173  match krawczyk3_certificate(&system, axis, box_) {
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi_trace.rs:578  /// (`ssi.rs::krawczyk3_certificate`) owns its interval-adjugate preconditioner
krawczyk3_certificate  vendor\truck\truck-certified\src\ssi_trace.rs:594  match krawczyk3_certificate(system, axis, box_) {
krawczyk3_certificate  vendor\truck\truck-certified\tests\ssi_system.rs:255  let cert = match krawczyk3_certificate(system, axis, ROOT_BOX) {
krawczyk3_certificate  vendor\truck\truck-certified\tests\ssi_system.rs:289  match krawczyk3_certificate(system, axis, fixture.box_) {
krawczyk3_certificate  vendor\truck\truck-certified\tests\ssi_system.rs:307  match krawczyk3_certificate(system, axis, fixture.box_) {
krawczyk3_certificate  vendor\truck\truck-certified\tests\ssi_system.rs:334  match krawczyk3_certificate(system, axis, ROOT_BOX) {
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:45  use crate::ssi::{RationalBipatch, SsiRefusal};
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:251  let patch = RationalBipatch::new(m, n, [xg, yg, zg], weights)?;
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:374  let participant = |p: &RationalBipatch| SsiParticipant::RationalBipatch(p.clone());
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1388  Ok(RationalBipatch { m, n, num, w })
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1416  RationalBipatch(RationalBipatch),
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1441  SsiParticipant::RationalBipatch(p) => p,
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1447  SsiParticipant::RationalBipatch(p) => p,
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1661  let plane = RationalBipatch::new(
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1672  let parabola = RationalBipatch::new(
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1684  &SsiParticipant::RationalBipatch(plane),
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1685  &SsiParticipant::RationalBipatch(parabola),
RationalBipatch  vendor\truck\truck-certified\src\ssi_admit.rs:22  //!   becomes an `SsiParticipant::RationalBipatch` with no new math.
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:45  use crate::ssi::{RationalBipatch, SsiRefusal};
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:251  let patch = RationalBipatch::new(m, n, [xg, yg, zg], weights)?;
RationalBipatch  vendor\truck\truck-certified\src\patch_admit.rs:374  let participant = |p: &RationalBipatch| SsiParticipant::RationalBipatch(p.clone());
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1388  Ok(RationalBipatch { m, n, num, w })
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1416  RationalBipatch(RationalBipatch),
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1441  SsiParticipant::RationalBipatch(p) => p,
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1447  SsiParticipant::RationalBipatch(p) => p,
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1661  let plane = RationalBipatch::new(
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1672  let parabola = RationalBipatch::new(
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1684  &SsiParticipant::RationalBipatch(plane),
RationalBipatch  vendor\truck\truck-certified\src\ssi.rs:1685  &SsiParticipant::RationalBipatch(parabola),
RationalBipatch  vendor\truck\truck-certified\src\ssi_admit.rs:22  //!   becomes an `SsiParticipant::RationalBipatch` with no new math.
new  vendor\truck\truck-certified\src\bvh.rs:31  //! packet adds no second cache. (5) Zero new top-level evidence kinds: a
new  vendor\truck\truck-certified\src\bvh.rs:111  pub fn new(axes: [CertifiedInterval; 3]) -> Result<Self, BvhRefusal> {
new  vendor\truck\truck-certified\src\bvh.rs:162  pub fn new(
new  vendor\truck\truck-certified\src\bvh.rs:260  let mut nodes = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:473  let mut candidates = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:474  let mut prune_records = Vec::new();
new  vendor\truck\truck-certified\src\bvh.rs:476  let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
new  vendor\truck\truck-certified\src\bvh.rs:555  let hint = match FloatHint::new(direction) {
new  vendor\truck\truck-certified\src\bvh.rs:741  let box_ = EnclosureBox::new(axes)?;
new  vendor\truck\truck-certified\src\bvh.rs:760  SpanLeaf::new(patch.cell, box_, net)
new  vendor\truck\truck-certified\src\bvh.rs:822  EnclosureBox::new([
new  vendor\truck\truck-certified\src\bvh.rs:848  let box_ = EnclosureBox::new([
new  vendor\truck\truck-evidence\src\bspline.rs:337  let bsp = BSplineCurve::new(KnotVec::bezier_knot(2), vec![Point3::new(0.0, 0.0, 0.0); 3]);
new  vendor\truck\truck-evidence\src\bspline.rs:379  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:382  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:383  Point3::new(-0.5, 0.5, -0.5),
new  vendor\truck\truck-evidence\src\bspline.rs:384  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:393  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:396  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:397  Point3::new(-1.0 / 3.0, -1.0 / 3.0, -1.0 / 3.0),
new  vendor\truck\truck-evidence\src\bspline.rs:398  Point3::new(-2.0 / 3.0, -2.0 / 3.0, -2.0 / 3.0),
new  vendor\truck\truck-evidence\src\bspline.rs:399  Point3::new(0.0, 0.0, 0.0),
new  vendor\truck\truck-evidence\src\bspline.rs:408  BSplineCurve::new(
new  vendor\truck\truck-evidence\src\bspline.rs:411  Point3::new(0.0, 0.0, 0.0),
m  vendor\truck\truck-certified\src\bvh.rs:4  //! ([`crate::patch_admit::SplinePatchStack`]); a loft×loft `contact()` call
m  vendor\truck\truck-certified\src\bvh.rs:5  //! would otherwise enumerate the full span-pair Cartesian product (a 20×30-span
m  vendor\truck\truck-certified\src\bvh.rs:11  //! (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3) is pruned *hereditarily* — one
m  vendor\truck\truck-certified\src\bvh.rs:14  //! **SFC discipline (spine decision 2, verbatim).** The search for a
m  vendor\truck\truck-certified\src\bvh.rs:18  //! `min_α λ·P¹_α > max_β λ·P²_β` over the node's net constants — O(n₁+n₂),
m  vendor\truck\truck-certified\src\bvh.rs:25  //! `enclose` boxes (the computation `hull_bernstein_2d` already performs) and
m  vendor\truck\truck-certified\src\bvh.rs:28  //! re-verification. (3) Surviving contact-candidate span pairs are emitted in
m  vendor\truck\truck-certified\src\bvh.rs:30  //! shorter. (4) Dyadic memoization rides CFP-003's per-side hull cache; this
m  vendor\truck\truck-certified\src\bvh.rs:36  //! **Determinism.** Fixed build order (span stack order), fixed candidate
m  vendor\truck\truck-certified\src\bvh.rs:46  use crate::formal::exact::{CertifiedInterval, CertifiedSign, Expansion};
m  vendor\truck\truck-certified\src\bvh.rs:48  use crate::patch_admit::{AdmittedPatch, SplinePatchStack};
m  vendor\truck\truck-certified\src\bvh.rs:59  /// A certified box axis is not finite or is misordered (`lo > hi`).
m  vendor\truck\truck-evidence\src\bspline.rs:9  //! tighter than naive interval arithmetic on the basis sum (which suffers
m  vendor\truck\truck-evidence\src\bspline.rs:12  //! The tangent cone comes off the **hodograph**: `BSplineCurve::derivation()`
m  vendor\truck\truck-evidence\src\bspline.rs:17  //! Four decisions deviate from the packet (recorded in `RESULT.json`):
m  vendor\truck\truck-evidence\src\bspline.rs:19  //! - **Out-of-range `tt`.** Decision 5 unions `Box3::point(origin)` on the claim
m  vendor\truck\truck-evidence\src\bspline.rs:35  //!   extracted sub-curve's control-point hull (measured up to ~10 ulps). The
m  vendor\truck\truck-evidence\src\bspline.rs:40  //!   hull therefore also includes `subs(lo)` and `subs(hi)` themselves.
m  vendor\truck\truck-evidence\src\bspline.rs:42  use crate::enclosure::{midpoint_ball_cone, Box3, DirCone, EnclosureCurve};
m  vendor\truck\truck-evidence\src\bspline.rs:44  use truck_base::cgmath64::control_point::ControlPoint;
m  vendor\truck\truck-evidence\src\bspline.rs:45  use truck_base::cgmath64::{Point3, Vector3};
m  vendor\truck\truck-evidence\src\bspline.rs:47  use truck_geometry::nurbs::BSplineCurve;
m  vendor\truck\truck-evidence\src\bspline.rs:48  use truck_geotrait::{Cut, ParametricCurve};
m  vendor\truck\truck-evidence\src\bspline.rs:51  /// (deviation from decision 4's single `next_down`/`next_up` step).
n  vendor\truck\truck-certified\src\bvh.rs:1  //! Per-carrier span BVH broadphase (CFP-005-BRANCH-BVH).
n  vendor\truck\truck-certified\src\bvh.rs:4  //! ([`crate::patch_admit::SplinePatchStack`]); a loft×loft `contact()` call
n  vendor\truck\truck-certified\src\bvh.rs:5  //! would otherwise enumerate the full span-pair Cartesian product (a 20×30-span
n  vendor\truck\truck-certified\src\bvh.rs:11  //! (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3) is pruned *hereditarily* — one
n  vendor\truck\truck-certified\src\bvh.rs:14  //! **SFC discipline (spine decision 2, verbatim).** The search for a
n  vendor\truck\truck-certified\src\bvh.rs:16  //! [`FloatHint`] (the spine's type — carries no evidence status). The
n  vendor\truck\truck-certified\src\bvh.rs:18  //! `min_α λ·P¹_α > max_β λ·P²_β` over the node's net constants — O(n₁+n₂),
n  vendor\truck\truck-certified\src\bvh.rs:23  //! **Scope decisions (CFP-005; do not relitigate).** (1) The leaves are the
n  vendor\truck\truck-certified\src\bvh.rs:25  //! `enclose` boxes (the computation `hull_bernstein_2d` already performs) and
n  vendor\truck\truck-certified\src\bvh.rs:26  //! internal nodes bound the union. (2) Separation certificates inherit: a
n  vendor\truck\truck-certified\src\bvh.rs:28  //! re-verification. (3) Surviving contact-candidate span pairs are emitted in
n  vendor\truck\truck-certified\src\bvh.rs:30  //! shorter. (4) Dyadic memoization rides CFP-003's per-side hull cache; this
n  vendor\truck\truck-evidence\src\bspline.rs:9  //! tighter than naive interval arithmetic on the basis sum (which suffers
n  vendor\truck\truck-evidence\src\bspline.rs:10  //! dependency loss) and cheaper (no interval basis evaluation).
n  vendor\truck\truck-evidence\src\bspline.rs:12  //! The tangent cone comes off the **hodograph**: `BSplineCurve::derivation()`
n  vendor\truck\truck-evidence\src\bspline.rs:17  //! Four decisions deviate from the packet (recorded in `RESULT.json`):
n  vendor\truck\truck-evidence\src\bspline.rs:19  //! - **Out-of-range `tt`.** Decision 5 unions `Box3::point(origin)` on the claim
n  vendor\truck\truck-evidence\src\bspline.rs:24  //!   (verified: the quadratic `t²−t` witness returns `subs(−10) = (110, 0, 0)`
n  vendor\truck\truck-evidence\src\bspline.rs:25  //!   and `subs(10) = (90, 0, 0)`), which is unbounded as `|t| → ∞`. The origin
n  vendor\truck\truck-evidence\src\bspline.rs:30  //!   `subs(0.25)`; the packet's own witness requires that point box, so `lo ==
n  vendor\truck\truck-evidence\src\bspline.rs:31  //!   hi` is hulled as the point (decision 4's widening included).
n  vendor\truck\truck-evidence\src\bspline.rs:35  //!   extracted sub-curve's control-point hull (measured up to ~10 ulps). The
n  vendor\truck\truck-evidence\src\bspline.rs:36  //!   endpoints are therefore padded by `HULL_PAD (1 + |·|)` instead.
n  vendor\truck\truck-evidence\src\bspline.rs:37  //! - **Degree-0 boundary values.** `subs(hi)` of the source curve uses the
numerator  vendor\truck\truck-certified\src\bvh.rs:729  /// each numerator component over the unit square (the span's Bézier image under
numerator  vendor\truck\truck-certified\src\bvh.rs:733  let numerator = patch_box.numerator();
numerator  vendor\truck\truck-certified\src\bvh.rs:737  let hull = hull_bernstein_2d(&numerator[component], (0.0, 1.0), (0.0, 1.0))
numerator  vendor\truck\truck-certified\src\contract.rs:45  //! | rational NURBS numerator/denominator | interval composition | homogeneous control points bound
numerator  vendor\truck\truck-certified\src\contract.rs:196  /// The rational NURBS numerator (homogeneous control points, hulls).
numerator  vendor\truck\truck-certified\src\patch_admit.rs:19  //! Such a surface is admitted carrying weight exactly 1 (numerator = the
numerator  vendor\truck\truck-certified\src\patch_admit.rs:36  //! per-side carriers hold `4·(8+1)² = 324` coefficients each (numerators and
numerator  vendor\truck\truck-certified\src\patch_admit.rs:319  let num = admitted.patch.numerator();
numerator  vendor\truck\truck-certified\src\ssi.rs:661  let ha_num = hull_bivariate(&side_a.numerator()[component], ma, na, unit[0], unit[1])?;
numerator  vendor\truck\truck-certified\src\ssi.rs:663  let hb_num = hull_bivariate(&side_b.numerator()[component], mb, nb, unit[2], unit[3])?;
numerator  vendor\truck\truck-certified\src\ssi.rs:696  let ha_num = hull_bivariate(&side_a.numerator()[component], ma, na, unit[0], unit[1])?;
numerator  vendor\truck\truck-certified\src\ssi.rs:698  let hb_num = hull_bivariate(&side_b.numerator()[component], mb, nb, unit[2], unit[3])?;
numerator  vendor\truck\truck-evidence\src\fid\isotopy.rs:269  /// numerator bracket is 0 (a straight line). Uses
numerator  vendor\truck\truck-evidence\src\fid\isotopy.rs:311  let numerator = norm_sup(&cross_box(&d1, &d2));
numerator  vendor\truck\truck-evidence\src\fid\lfs.rs:177  // Flat within enclosure (a plane, say): every numerator is zero.
weights  vendor\truck\truck-certified\src\bvh.rs:734  let weights = patch_box.weights();
weights  vendor\truck\truck-certified\src\patch_admit.rs:37  //! weights), 648 total. A CFP-005 span-BVH leaf (one admitted patch pair)
weights  vendor\truck\truck-certified\src\patch_admit.rs:251  let patch = RationalBipatch::new(m, n, [xg, yg, zg], weights)?;
weights  vendor\truck\truck-certified\src\patch_admit.rs:358  // Rational weights (≠ 1) refuse typed (NonPositiveNurbsWeight).
weights  vendor\truck\truck-certified\src\ssi.rs:662  let ha_w = hull_bivariate(side_a.weights(), ma, na, unit[0], unit[1])?;
weights  vendor\truck\truck-certified\src\ssi.rs:664  let hb_w = hull_bivariate(side_b.weights(), mb, nb, unit[2], unit[3])?;
weights  vendor\truck\truck-certified\src\ssi.rs:697  let ha_w = hull_bivariate(side_a.weights(), ma, na, unit[0], unit[1])?;
weights  vendor\truck\truck-certified\src\ssi.rs:699  let hb_w = hull_bivariate(side_b.weights(), mb, nb, unit[2], unit[3])?;
weights  vendor\truck\truck-certified\src\ssi.rs:704  let dw = bivariate_derivative(side_a.weights(), ma, na, axis)?;
weights  vendor\truck\truck-certified\src\ssi.rs:718  let dw = bivariate_derivative(side_b.weights(), mb, nb, side_axis)?;
weights  vendor\truck\truck-certified\src\ssi.rs:773  #[allow(dead_code)] // CFP-003 per-side substrate: Bernstein product weights (normal-net composition
weights  vendor\truck\truck-certified\src\ssi.rs:840  /// The per-side unit normal net of one carrier under D2 (unit weights): the
weights  vendor\truck\truck-evidence\src\nurbs.rs:238  /// All sub-curve weights are positive (decision 3's gate, preserved by Boehm
weights  vendor\truck\truck-evidence\src\nurbs.rs:274  fn positive_weights(curve: &NurbsCurve<Vector4>) -> bool {
weights  vendor\truck\truck-evidence\src\nurbs.rs:346  if !positive_weights(curve) {
weights  vendor\truck\truck-evidence\src\nurbs.rs:384  if !positive_weights(self) {
weights  vendor\truck\truck-evidence\src\nurbs.rs:417  if !positive_weights(self) {
weights  vendor\truck\truck-evidence\src\nurbs.rs:458  if !positive_weights(self) {
weights  vendor\truck\truck-evidence\src\nurbs.rs:576  NurbsCurve::try_from_bspline_and_weights(bsp, vec![1.0, 1.0, 1.0])
weights  vendor\truck\truck-evidence\src\nurbs.rs:592  NurbsCurve::try_from_bspline_and_weights(bsp, vec![1.0, 1.5, 2.0])
weights  vendor\truck\truck-evidence\src\nurbs.rs:593  .expect("the constant curve with positive weights is valid")
weights  vendor\truck\truck-evidence\src\nurbs.rs:607  NurbsCurve::try_from_bspline_and_weights(bsp, vec![weight, 1.0, 1.0])
weights  vendor\truck\truck-evidence\src\contact\implicit2d.rs:513  /// weights `C(p_a,i₁)·C(p_b,k−i₁)/C(p_a+p_b,k)` per axis.
weights  vendor\truck\truck-evidence\src\contact\implicit2d.rs:663  ///   with **exact unit weights** (the D2 admission; a multi-span or rational
SsiParticipant  vendor\truck\truck-certified\src\patch_admit.rs:291  use crate::ssi::{construct_square_system, SsiParticipant};
SsiParticipant  vendor\truck\truck-certified\src\patch_admit.rs:374  let participant = |p: &RationalBipatch| SsiParticipant::RationalBipatch(p.clone());
... (truncated at 400 lines)
