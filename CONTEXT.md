# CONTEXT.md - mechanically generated from the tree at dispatch time.
# Signatures, callers, tests only. No claims. Regenerated per dispatch.

## WRITE: vendor/truck/truck-evidence/src/analytic/ruled_pair.rs (MISSING at dispatch time)

## WRITE: vendor/truck/truck-evidence/src/analytic/mod.rs
L25    type     PlacedCircle - A full circle placed in space: the trimmed unit circle under an affine
L29    type     PlacedParabola - A parabola placed in space: the trimmed unit parabola under an affine
L33    type     PlacedHyperbola - One branch of a hyperbola placed in space: the trimmed unit hyperbola under
L48    enum     ExactCurve - An exactly parameterized intersection curve (BG-ANA-001).
L71    enum     AnalyticIntersection - The result of an exactly-solved surface pair (BG-ANA-001): an exact curve,
L116   type     AnalyticOutcome - What every analytic pair family returns (BG-ANA-001).
L119   mod      coaxial - BG-ANA-001-COAX: coaxial pairs. Scaffolded empty; the packet fills it.
L122   mod      equal_radius_cylinders - BG-ANA-001-EQRCYL: equal-radius cylinders with intersecting axes.
L125   mod      parallel_cylinders - BG-ANA-001-PARCYL: parallel-axis cylinders. Scaffolded empty; the packet
L127   mod      plane_cone - BG-ANA-001-PCONE: plane × cone. Scaffolded empty; the packet fills it.
L129   mod      plane_cylinder - BG-ANA-001-PCYL: plane × cylinder. Scaffolded empty; the packet fills it.
L131   mod      plane_plane - BG-ANA-001-PP: plane × plane. Scaffolded empty; the packet fills it.
L133   mod      plane_sphere - BG-ANA-001-PS: plane × sphere. Scaffolded empty; the packet fills it.
L135   mod      sphere_sphere - BG-ANA-001-SS: sphere × sphere. Scaffolded empty; the packet fills it.
tests: placed, circle_arm_evaluates_on_the_expected_circle, ellipse_arm_evaluates_with_distinct_semi_axes, degenerate_arms_are_the_classification

## WRITE: vendor/truck/truck-certified/src/pair_dispatch.rs
L124   enum     CertifiedPairParticipant - One side of a dispatched pair: the certified witness of an identified
L133   impl     CertifiedPairParticipant
L136   fn       from_support_schema - Route a landed support-surface schema: the certified plane arm becomes
L145   fn       from_cylinder_identification - Route a landed cylinder identification: the certified arm becomes a
L154   fn       from_sphere_identification - Route a landed sphere identification: the certified arm becomes a
L168   fn       from_cone_identification - The cone route, known to the routing but not this packet.
L180   fn       from_torus_identification - The torus route, known to the routing but not this packet.
L191   enum     ContactLocus - The certified contact locus of an admitted pair. Raw-frame doctrine:
L218   struct   CertifiedPairContact - The certified contact: the sorted participants and the shared locus.
L236   enum     CertifiedPairResult - The result of dispatching one admitted-or-refused pair. Shape mirrors the
L255   fn       dispatch_pair - Dispatch one analytic surface pair. Operand order is canonical (D-sorted).

## WRITE: vendor/truck/truck-evidence/tests/ruled_pair_conformance.rs (MISSING at dispatch time)

## READ: docs/FSSI_BUILD_SPEC.md

## READ: vendor/truck/truck-evidence/src/ (MISSING at dispatch time)

## READ: vendor/truck/truck-certified/src/pair_dispatch.rs
L124   enum     CertifiedPairParticipant - One side of a dispatched pair: the certified witness of an identified
L133   impl     CertifiedPairParticipant
L136   fn       from_support_schema - Route a landed support-surface schema: the certified plane arm becomes
L145   fn       from_cylinder_identification - Route a landed cylinder identification: the certified arm becomes a
L154   fn       from_sphere_identification - Route a landed sphere identification: the certified arm becomes a
L168   fn       from_cone_identification - The cone route, known to the routing but not this packet.
L180   fn       from_torus_identification - The torus route, known to the routing but not this packet.
L191   enum     ContactLocus - The certified contact locus of an admitted pair. Raw-frame doctrine:
L218   struct   CertifiedPairContact - The certified contact: the sorted participants and the shared locus.
L236   enum     CertifiedPairResult - The result of dispatching one admitted-or-refused pair. Shape mirrors the
L255   fn       dispatch_pair - Dispatch one analytic surface pair. Operand order is canonical (D-sorted).

## READ: vendor/truck/truck-shapeops/src/boolean/split.rs
L88    enum     SolidRef - Which solid a stratum reference belongs to.
L101   enum     StratumRef - Where a contact event's record came from. Faces index
L122   struct   ContactEvent - One contact record with the provenance the splitter needs.
L133   enum     FragmentOrigin - Which parent face a fragment came from.
L148   struct   Fragment - One fragment of a split face.
L157   enum     AdjacencyParity - The parity of a shared edge between two fragments.
L168   struct   FragmentAdjacency - One adjacency entry between two fragments of the SAME solid, per shared
L179   enum     CoincidentOrientation - The relative orientation of a coincident fragment pair.
L188   struct   CoincidentPair - A cross-solid coincident fragment pair (the seam of the assembled shell).
L199   struct   FragmentMesh - The output of the splitter.
L217   fn       split_fragments - Split both shells along the contact events.
L279   impl     Loops
L2084  fn       near_pt - Whether two points are within `tol` of each other.
L2098  fn       create_parameter_boundary - Projects the boundary edge's division points into the face's `(u, v)`
L2399  fn       region_contains - Whether the parameter point is strictly inside the region bounded by the
L2482  fn       point_segment_distance - The perpendicular distance from `p` to the segment `a`-`b`.
L2592  fn       region_representative - An interior representative point of the region, if one can be found: the
tests: placed_circle, block_profile, plate_with_hole_profile, disk_profile, extrude_shell, plane_face_at_z, cylinder_face, flat_edge_at_z, fragments_of_origin, fragment_edge_ids, wire_edge_counts, disk_face, ev, ff_curve_record, split_flagship_top_face_by_ff_circle, split_six_event_flagship_bottom_face_divides_like_the_top, split_sewn_rim_directions_preserve_effective_traversals, split_cuts_edges_at_point_contacts, split_open_arc_uses_point_events_for_trimming, split_region2_disjoint_regions_is_no_coincidence, split_region2_partial_overlap_refuses, split_refuses_deferred_loci, split_ff_only_circle_skips_the_on_boundary_wall

## CALLER SITES (grep of defining names outside their file)
PlacedCircle  vendor\truck\truck-evidence\tests\conjugation.rs:32  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};
PlacedCircle  vendor\truck\truck-evidence\tests\conjugation.rs:128  fn two_ellipses(out: &Certified<ContactComplex>) -> (PlacedCircle, PlacedCircle) {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\coaxial.rs:26  //! own. The emitted circles are [`crate::analytic::PlacedCircle`]: the trimmed
PlacedCircle  vendor\truck\truck-evidence\src\analytic\coaxial.rs:42  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
PlacedCircle  vendor\truck\truck-evidence\src\analytic\coaxial.rs:813  fn circle_at(axis: (f64, f64), z: f64, r: f64) -> PlacedCircle {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\coaxial.rs:879  fn as_two_circles(value: &AnalyticIntersection) -> [PlacedCircle; 2] {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:49  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
PlacedCircle  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:217  fn placed_ellipse(u: Vector3, v: Vector3, o: Point3, ru: f64, rv: f64) -> PlacedCircle {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:256  fn two_ellipses(out: &AnalyticIntersection) -> (&PlacedCircle, &PlacedCircle) {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:266  fn ellipse_ratio(e: &PlacedCircle) -> f64 {
PlacedCircle  vendor\truck\truck-evidence\src\analytic\mod.rs:52  Circle(PlacedCircle),
PlacedCircle  vendor\truck\truck-evidence\src\analytic\mod.rs:54  Ellipse(PlacedCircle),
PlacedParabola  vendor\truck\truck-evidence\src\analytic\mod.rs:56  Parabola(PlacedParabola),
PlacedHyperbola  vendor\truck\truck-evidence\src\analytic\mod.rs:58  Hyperbola(PlacedHyperbola),
ExactCurve  vendor\truck\truck-evidence\tests\conjugation.rs:32  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};
ExactCurve  vendor\truck\truck-evidence\tests\conjugation.rs:134  [ExactCurve::Ellipse(e0), ExactCurve::Ellipse(e1)],
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:25  //! [`crate::analytic::ExactCurve`]); this module defines no result type of its
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:42  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:190  ExactCurve::Circle(circle_at((x0, y0), za - half, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:191  ExactCurve::Circle(circle_at((x0, y0), za + half, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:225  ExactCurve::Circle(circle_at((x0, y0), zs - root, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:226  ExactCurve::Circle(circle_at((x0, y0), zs + root, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:290  ExactCurve::Circle(circle_at((x0, y0), zt - root, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:291  ExactCurve::Circle(circle_at((x0, y0), zt + root, rc)),
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:379  AnalyticIntersection::Curve(ExactCurve::Circle(circle_at((x0, y0), *z, radius(*z))))
ExactCurve  vendor\truck\truck-evidence\src\analytic\coaxial.rs:382  ExactCurve::Circle(circle_at((x0, y0), *z0, radius(*z0))),
ExactCurve  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1081  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
ExactCurve  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1216  ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(_)))
ExactCurve  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1286  let ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(c))) =
ExactCurve  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1329  let ExactCurve::Circle(c) = curve else {
ExactCurve  vendor\truck\truck-shapeops\src\boolean\classify.rs:882  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
ExactCurve  vendor\truck\truck-shapeops\src\boolean\classify.rs:1011  fn ff_curve_record(exact: ExactCurve) -> ContactRecord {
ExactCurve  vendor\truck\truck-shapeops\src\boolean\classify.rs:1109  let exact = ExactCurve::Circle(placed_circle(Point3::new(2.0, 2.0, 2.0), 1.0));
ExactCurve  vendor\truck\truck-shapeops\src\boolean\classify.rs:1358  let line1 = ExactCurve::Line(Line(Point3::new(4.0, 1.0, 0.0), Point3::new(4.0, 1.0, 2.0)));
ExactCurve  vendor\truck\truck-shapeops\src\boolean\classify.rs:1359  let line2 = ExactCurve::Line(Line(Point3::new(4.0, 3.0, 0.0), Point3::new(4.0, 3.0, 2.0)));
ExactCurve  vendor\truck\truck-shapeops\src\boolean\split.rs:52  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
ExactCurve  vendor\truck\truck-shapeops\src\boolean\split.rs:651  ExactCurve::Line(_) | ExactCurve::Circle(_) | ExactCurve::Ellipse(_) => {}
ExactCurve  vendor\truck\truck-shapeops\src\boolean\split.rs:652  ExactCurve::Parabola(_) | ExactCurve::Hyperbola(_) => return Err(refused()),
AnalyticIntersection  vendor\truck\truck-evidence\tests\conjugation.rs:32  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};
AnalyticIntersection  vendor\truck\truck-evidence\tests\conjugation.rs:133  let ContactLocus::Analytic(AnalyticIntersection::TwoCurves(
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:24  //! The shared result type is [`crate::analytic::AnalyticIntersection`] (with
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:42  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:141  AnalyticIntersection::Coincident
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:143  AnalyticIntersection::Empty
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:173  AnalyticIntersection::Empty,
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:189  AnalyticIntersection::TwoCurves([
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:224  AnalyticIntersection::TwoCurves([
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:241  AnalyticIntersection::TangentCircle(circle_at((x0, y0), zs, rc)),
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:255  AnalyticIntersection::Empty,
AnalyticIntersection  vendor\truck\truck-evidence\src\analytic\coaxial.rs:289  AnalyticIntersection::TwoCurves([
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\assemble.rs:38  use truck_evidence::analytic::AnalyticIntersection;
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\assemble.rs:377  ContactLocus::Analytic(AnalyticIntersection::Coincident),
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1081  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1216  ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(_)))
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1286  let ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(c))) =
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\classify.rs:882  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\classify.rs:1015  locus: ContactLocus::Analytic(AnalyticIntersection::Curve(exact)),
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\classify.rs:1364  locus: ContactLocus::Analytic(AnalyticIntersection::TwoCurves([line1, line2])),
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\split.rs:52  use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\split.rs:908  ContactLocus::Analytic(AnalyticIntersection::Curve(exact)),
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\split.rs:919  ContactLocus::Analytic(AnalyticIntersection::TwoCurves([c0, c1])),
AnalyticIntersection  vendor\truck\truck-shapeops\src\boolean\split.rs:930  (ContactLocus::Analytic(AnalyticIntersection::Curve(_)), _, _) => Err(refused()),
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:42  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:119  pub fn coaxial(pair: &CoaxialPair) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:139  fn cyl_cyl(a: &Cylinder, b: &Cylinder) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:165  fn cyl_cone(cyl: &Cylinder, cone: &Cone) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:212  fn cyl_sphere(cyl: &Cylinder, sphere: &Sphere) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:277  fn cyl_torus(cyl: &Cylinder, torus: &Torus) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:346  fn cone_cone(a: &Cone, b: &Cone) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:412  fn cone_sphere(cone: &Cone, sphere: &Sphere) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:466  fn cone_torus(cone: &Cone, torus: &Torus) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:517  fn sphere_torus(sphere: &Sphere, torus: &Torus) -> AnalyticOutcome {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\coaxial.rs:871  fn value_of(out: AnalyticOutcome) -> AnalyticIntersection {
AnalyticOutcome  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:49  use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:237  .expect("an identity-placed coaxial pair is decidable");
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:240  contact(&bare_c, &bare_d, &mut budget).expect("a bare coaxial pair is decidable");
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:246  .expect("the overlapping coaxial pair emits one record");
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:283  .expect("a folded coaxial placed pair is decidable");
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:286  contact(&bare_c, &bare_d, &mut budget).expect("the bare coaxial pair is decidable");
coaxial  vendor\truck\truck-evidence\tests\conjugation.rs:292  .expect("the folded coaxial pair emits one record");
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:1  //! BG-ANA-001-COAX: coaxial pairs (cylinder/cone/sphere/torus) — circles or
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:44  /// A coaxial pair of carriers sharing the z axis (BG-ANA-001-COAX).
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:104  /// Classifies a coaxial pair exactly (BG-ANA-001-COAX).
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:119  pub fn coaxial(pair: &CoaxialPair) -> AnalyticOutcome {
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:385  _ => unreachable!("two coaxial cones meet in at most two circles"),
coaxial  vendor\truck\truck-evidence\src\analytic\coaxial.rs:895  let value = value_of(coaxial(&CoaxialPair::CylSphere(&cyl, &sph)));
coaxial  vendor\truck\truck-certified\src\pair_dispatch.rs:20  //! | cylinder~cylinder (coaxial/parallel subset) | 5,354 |
coaxial  vendor\truck\truck-certified\src\pair_dispatch.rs:780  // Arm 5: cylinder~cylinder (5,354; the coaxial/parallel subset only)
coaxial  vendor\truck\truck-certified\src\pair_dispatch.rs:799  // Collinear (coaxial): `(o2 − o1) × axis` is exactly the zero vector.
coaxial  vendor\truck\truck-certified\tests\kernel_contract.rs:596  fn fixture_coaxial_cylinders_sheet_ground_truth() {
coaxial  vendor\truck\truck-certified\tests\kernel_contract.rs:597  let fixture = construct(fx::coaxial_cylinders());
coaxial  vendor\truck\truck-certified\tests\kernel_contract.rs:601  let sheet = construct(fx::coaxial_cylinder_sheet(&fixture.first, &fixture.second));
coaxial  vendor\truck\truck-certified\tests\kernel_contract.rs:606  let flipped = fx::coaxial_cylinder_sheet(&fixture.first, &fixture.anti_parallel);
coaxial  vendor\truck\truck-certified\tests\kernel_s03a.rs:384  let fixture = construct_ok(fixtures::coaxial_cylinders());
coaxial  vendor\truck\truck-certified\tests\kernel_sheet.rs:98  /// the identity closed-form psi (the shim kit's coaxial fixture): `SheetCert`
coaxial  vendor\truck\truck-certified\tests\kernel_sheet.rs:102  let fx = construct(fixtures::coaxial_cylinders());
coaxial  vendor\truck\truck-certified\tests\kernel_sheet.rs:110  other => panic!("the coaxial identity sheet must certify: {other:?}"),
coaxial  vendor\truck\truck-certified\tests\kernel_sheet.rs:158  let fx = construct(fixtures::coaxial_cylinders());
equal_radius_cylinders  vendor\truck\truck-evidence\tests\conjugation.rs:6  //! via `equal_radius_cylinders`), the W3 fold (a translation + z-rotation +
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:60  pub fn equal_radius_cylinders(
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:275  let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:332  let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:377  equal_radius_cylinders(UNIT_RADIUS, &axis, &axis),
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:384  equal_radius_cylinders(UNIT_RADIUS, &axis, &opposite),
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:398  equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1),
equal_radius_cylinders  vendor\truck\truck-evidence\src\analytic\equal_radius_cylinders.rs:409  let out = equal_radius_cylinders(UNIT_RADIUS, &axis0, &axis1).unwrap();
equal_radius_cylinders  vendor\truck\truck-evidence\src\contact\mod.rs:41  use crate::analytic::equal_radius_cylinders::equal_radius_cylinders;
equal_radius_cylinders  vendor\truck\truck-evidence\src\contact\mod.rs:1201  /// downstream rather than panicking (the `equal_radius_cylinders.rs` pattern).
equal_radius_cylinders  vendor\truck\truck-evidence\src\contact\mod.rs:1237  /// (the `equal_radius_cylinders.rs` pattern).
equal_radius_cylinders  vendor\truck\truck-evidence\src\contact\mod.rs:1468  /// `equal_radius_cylinders` cell runs on the WORLD poses (it is frame-free —
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:105  pub fn parallel_cylinders(cylinder0: &Cylinder, cylinder1: &Cylinder) -> AnalyticOutcome {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:344  let Ok(cert) = parallel_cylinders(&c0, &c1) else {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:423  match parallel_cylinders(&c0, &c1) {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:447  let Ok(cert) = parallel_cylinders(&c0, &c1) else {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:469  let Ok(cert) = parallel_cylinders(&c0, &c2) else {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:484  let Ok(cert) = parallel_cylinders(&a, &same) else {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:491  let Ok(cert) = parallel_cylinders(&a, &nested) else {
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:533  let out = parallel_cylinders(&cyl(0.0, 0.0, 1.0), &c1);
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:545  parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(1.0, 0.0, 1.0)),
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:549  parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(2.0, 0.0, 1.0)),
parallel_cylinders  vendor\truck\truck-evidence\src\analytic\parallel_cylinders.rs:553  parallel_cylinders(&cyl(0.0, 0.0, 1.0), &cyl(3.0, 0.0, 1.0)),
parallel_cylinders  vendor\truck\truck-evidence\src\contact\mod.rs:42  use crate::analytic::parallel_cylinders::parallel_cylinders;
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:117  pub fn plane_cone(plane: &Plane, cone: &Cone) -> AnalyticOutcome {
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:614  Err(refusal) => unreachable!("plane_cone refused this witness: {refusal:?}"),
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:621  Err(refusal) => unreachable!("plane_cone refused this witness: {refusal:?}"),
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:634  let out = plane_cone(&plane, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:652  let out = plane_cone(&through, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:668  let out = plane_cone(&plane, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:712  let out = plane_cone(&plane, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:744  let out = plane_cone(&plane, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:768  let out = plane_cone(&plane, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:790  let out = plane_cone(&steep, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:803  let out = plane_cone(&shallow, &cone);
plane_cone  vendor\truck\truck-evidence\src\analytic\plane_cone.rs:817  let out = plane_cone(&boundary, &cone);
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:110  pub fn plane_cylinder(plane: &Plane, cylinder: &Cylinder) -> AnalyticOutcome {
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:326  let value = value_of(plane_cylinder(&plane, &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:357  let value = value_of(plane_cylinder(&plane_x(1.0), &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:376  let value = value_of(plane_cylinder(&plane_x(2.0), &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:388  let value = value_of(plane_cylinder(&plane, &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:418  let value = value_of(plane_cylinder(&plane, &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:493  let out = plane_cylinder(&plane_x(offset), &cylinder);
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:509  let out = plane_cylinder(&straddle_plane, &cylinder);
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:522  assert_exact(plane_cylinder(&plane_x(THREE_FIFTHS), &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:523  assert_exact(plane_cylinder(
plane_cylinder  vendor\truck\truck-evidence\src\analytic\plane_cylinder.rs:527  assert_exact(plane_cylinder(&tilted_plane(), &cylinder));
plane_cylinder  vendor\truck\truck-evidence\src\contact\mod.rs:44  use crate::analytic::plane_cylinder::plane_cylinder;
plane_cylinder  vendor\truck\truck-certified\src\pair_dispatch.rs:269  plane_cylinder(p, c)
plane_cylinder  vendor\truck\truck-certified\src\pair_dispatch.rs:548  fn plane_cylinder(plane: PlaneSchema, cyl: CertifiedEmbeddedCylinder) -> CertifiedPairResult {
plane_cylinder  vendor\truck\truck-certified\src\pair_dispatch.rs:577  return plane_cylinder_parallel(plane, cyl, nf, n);
plane_cylinder  vendor\truck\truck-certified\src\pair_dispatch.rs:602  fn plane_cylinder_parallel(
plane_cylinder  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:244  fn plane_cylinder_transverse_emits_circle() {
plane_cylinder  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:282  fn plane_cylinder_tangent_emits_generatrix_line_and_offset_is_disjoint() {
plane_cylinder  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:34  //! 2. **plane × cylinder** ([`plane_cylinder_fixture`]) — a transverse plane
plane_cylinder  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:161  pub fn plane_cylinder_fixture() -> Result<PlaneCylinderFixture, ConstructRefusal> {
plane_cylinder  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:335  plane_cylinder: plane_cylinder_fixture()?,
plane_cylinder  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:497  fn fixture_plane_cylinder_ground_truth() {
plane_cylinder  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:498  let built = plane_cylinder_fixture();
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:39  pub fn plane_plane(plane0: &Plane, plane1: &Plane) -> AnalyticOutcome {
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:236  let out = plane_plane(&a, &b).expect("dyadic transverse witness is decidable");
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:289  let out = plane_plane(&z0, &z2).expect("dyadic parallel witness is decidable");
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:292  let out = plane_plane(&z0, &z0).expect("dyadic coincident witness is decidable");
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:312  let out = plane_plane(p0, p1).expect("dyadic coincident witness is decidable");
plane_plane  vendor\truck\truck-evidence\src\analytic\plane_plane.rs:365  let out = plane_plane(a, b).expect("dyadic witness is decidable");
plane_plane  vendor\truck\truck-evidence\src\contact\fe_ee.rs:65  use crate::analytic::plane_plane::plane_plane;
plane_plane  vendor\truck\truck-evidence\src\contact\fe_ee.rs:128  // Decisive interval predicates (copied from analytic/plane_plane.rs, verbatim
plane_plane  vendor\truck\truck-evidence\src\contact\fe_ee.rs:663  let out = plane_plane(&circle_plane, plane)?;
plane_plane  vendor\truck\truck-evidence\src\contact\fe_ee.rs:683  // `plane_plane` emits only the arms above (or a NumericallyUnresolved
plane_plane  vendor\truck\truck-evidence\src\contact\mod.rs:45  use crate::analytic::plane_plane::plane_plane;
plane_plane  vendor\truck\truck-evidence\src\contact\mod.rs:1029  (CanonicalSurface::Plane(a), CanonicalSurface::Plane(b)) => plane_plane(a, b),
plane_plane  vendor\truck\truck-certified\src\pair_dispatch.rs:266  plane_plane(pa, pb)
plane_plane  vendor\truck\truck-certified\src\pair_dispatch.rs:501  fn plane_plane(a: PlaneSchema, b: PlaneSchema) -> CertifiedPairResult {
plane_plane  vendor\truck\truck-certified\src\pair_dispatch.rs:509  locus: plane_plane_line(&a, &b),
plane_plane  vendor\truck\truck-certified\src\pair_dispatch.rs:528  fn plane_plane_line(a: &PlaneSchema, b: &PlaneSchema) -> ContactLocus {
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:101  pub fn plane_sphere(plane: &Plane, sphere: &Sphere) -> AnalyticOutcome {
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:249  let circle = circle(plane_sphere(&plane, &sphere).unwrap());
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:270  let circle = circle(plane_sphere(&plane, &sphere).unwrap());
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:288  let out = plane_sphere(&plane, &sphere).unwrap();
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:295  let out = plane_sphere(&raised, &sphere).unwrap();
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:302  let out = plane_sphere(&plane, &high).unwrap();
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:343  assert!(plane_sphere(&plane, &Sphere::new(center, r_up)).is_ok());
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:345  assert!(plane_sphere(&plane, &Sphere::new(center, r_down)).is_ok());
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:365  plane_sphere(&tilted, &sphere),
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:374  plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 1.0), 1.25)).unwrap();
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:382  plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 1.0), 1.0)).unwrap();
plane_sphere  vendor\truck\truck-evidence\src\analytic\plane_sphere.rs:390  plane_sphere(&plane, &Sphere::new(Point3::new(0.0, 0.0, 2.0), 1.0)).unwrap();
plane_sphere  vendor\truck\truck-certified\src\pair_dispatch.rs:272  plane_sphere(p, s)
plane_sphere  vendor\truck\truck-certified\src\pair_dispatch.rs:642  fn plane_sphere(plane: PlaneSchema, sphere: CertifiedEmbeddedSphere) -> CertifiedPairResult {
plane_sphere  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:315  fn plane_sphere_transverse_emits_circle_with_enclosing_radius() {
plane_sphere  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:29  //! 1. **plane × sphere** ([`plane_sphere_fixture`]) — the plane z = 1 cuts a
plane_sphere  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:96  pub fn plane_sphere_fixture() -> PlaneSphereFixture {
plane_sphere  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:334  plane_sphere: plane_sphere_fixture(),
plane_sphere  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:429  fn fixture_plane_sphere_ground_truth() {
plane_sphere  vendor\truck\truck-certified\src\construct\bie\fixtures.rs:430  let fixture = plane_sphere_fixture();
plane_sphere  vendor\truck\truck-certified\src\construct\bie\ssi4.rs:2631  use crate::construct::bie::fixtures::{plane_sphere_fixture, sweep_plane_fixture};
plane_sphere  vendor\truck\truck-certified\src\construct\bie\ssi4.rs:2636  fn plane_sphere_form() -> FForm {
plane_sphere  vendor\truck\truck-certified\src\construct\bie\ssi4.rs:2637  let fixture = plane_sphere_fixture();
plane_sphere  vendor\truck\truck-certified\src\construct\bie\ssi4.rs:2676  let form = plane_sphere_form();
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:44  pub fn sphere_sphere(sphere0: &Sphere, sphere1: &Sphere) -> AnalyticOutcome {
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:299  let out = sphere_sphere(&s0, &s1).unwrap();
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:328  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:343  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:359  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:376  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:384  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:392  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:400  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:435  let tangent = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:449  let near = sphere_sphere(
sphere_sphere  vendor\truck\truck-evidence\src\analytic\sphere_sphere.rs:462  let out = sphere_sphere(
sphere_sphere  vendor\truck\truck-certified\src\pair_dispatch.rs:278  sphere_sphere(sa, sb)
sphere_sphere  vendor\truck\truck-certified\src\pair_dispatch.rs:700  fn sphere_sphere(a: CertifiedEmbeddedSphere, b: CertifiedEmbeddedSphere) -> CertifiedPairResult {
sphere_sphere  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:367  fn sphere_sphere_transverse_emits_radical_circle_and_tangent_emits_point() {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:87  //! [`CertifiedPairParticipant::from_cone_identification`] /
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:88  //! [`CertifiedPairParticipant::from_torus_identification`] map every
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:265  (CertifiedPairParticipant::Plane(pa), CertifiedPairParticipant::Plane(pb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:268  (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Cylinder(c)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:271  (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Sphere(s)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:274  (CertifiedPairParticipant::Cylinder(ca), CertifiedPairParticipant::Cylinder(cb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:277  (CertifiedPairParticipant::Sphere(sa), CertifiedPairParticipant::Sphere(sb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:292  fn participant_cmp(a: &CertifiedPairParticipant, b: &CertifiedPairParticipant) -> Ordering {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:294  (CertifiedPairParticipant::Plane(x), CertifiedPairParticipant::Plane(y)) => plane_cmp(x, y),
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:295  (CertifiedPairParticipant::Cylinder(x), CertifiedPairParticipant::Cylinder(y)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:298  (CertifiedPairParticipant::Sphere(x), CertifiedPairParticipant::Sphere(y)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:301  (CertifiedPairParticipant::Plane(_), CertifiedPairParticipant::Cylinder(_))
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:87  //! [`CertifiedPairParticipant::from_cone_identification`] /
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:88  //! [`CertifiedPairParticipant::from_torus_identification`] map every
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:265  (CertifiedPairParticipant::Plane(pa), CertifiedPairParticipant::Plane(pb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:268  (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Cylinder(c)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:271  (CertifiedPairParticipant::Plane(p), CertifiedPairParticipant::Sphere(s)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:274  (CertifiedPairParticipant::Cylinder(ca), CertifiedPairParticipant::Cylinder(cb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:277  (CertifiedPairParticipant::Sphere(sa), CertifiedPairParticipant::Sphere(sb)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:292  fn participant_cmp(a: &CertifiedPairParticipant, b: &CertifiedPairParticipant) -> Ordering {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:294  (CertifiedPairParticipant::Plane(x), CertifiedPairParticipant::Plane(y)) => plane_cmp(x, y),
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:295  (CertifiedPairParticipant::Cylinder(x), CertifiedPairParticipant::Cylinder(y)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:298  (CertifiedPairParticipant::Sphere(x), CertifiedPairParticipant::Sphere(y)) => {
CertifiedPairParticipant  vendor\truck\truck-certified\src\pair_dispatch.rs:301  (CertifiedPairParticipant::Plane(_), CertifiedPairParticipant::Cylinder(_))
from_support_schema  vendor\truck\truck-certified\src\pair_dispatch.rs:136  pub fn from_support_schema(schema: &SupportSurfaceSchema) -> Option<Self> {
from_support_schema  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:47  CertifiedPairParticipant::from_support_schema(&schema)
from_support_schema  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:504  CertifiedPairParticipant::from_support_schema(&non_plane).is_none(),
from_cylinder_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:145  pub fn from_cylinder_identification(id: CylinderIdentification) -> Option<Self> {
from_cylinder_identification  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:72  CertifiedPairParticipant::from_cylinder_identification(identify_cylinder(&revo))
from_sphere_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:154  pub fn from_sphere_identification(id: SphereIdentification) -> Option<Self> {
from_sphere_identification  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:77  CertifiedPairParticipant::from_sphere_identification(identify_sphere_world(center, radius))
from_cone_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:87  //! [`CertifiedPairParticipant::from_cone_identification`] /
from_cone_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:168  pub fn from_cone_identification(id: ConeIdentification) -> Option<Self> {
from_cone_identification  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:486  CertifiedPairParticipant::from_cone_identification(identify_cone(&cone)).is_none(),
from_torus_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:88  //! [`CertifiedPairParticipant::from_torus_identification`] map every
from_torus_identification  vendor\truck\truck-certified\src\pair_dispatch.rs:180  pub fn from_torus_identification(id: TorusIdentification) -> Option<Self> {
from_torus_identification  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:493  CertifiedPairParticipant::from_torus_identification(identify_torus(&torus)).is_none(),
ContactLocus  vendor\truck\truck-evidence\tests\conjugation.rs:33  use truck_evidence::contact::{contact, BoundedStratum, ContactComplex, ContactLocus};
ContactLocus  vendor\truck\truck-evidence\tests\conjugation.rs:133  let ContactLocus::Analytic(AnalyticIntersection::TwoCurves(
ContactLocus  vendor\truck\truck-evidence\tests\torus_pairs.rs:20  use truck_evidence::contact::{contact, BoundedStratum, ContactLocus};
ContactLocus  vendor\truck\truck-evidence\tests\torus_pairs.rs:81  if let ContactLocus::ValidatedBranchCover(cover) = &record.locus {
ContactLocus  vendor\truck\truck-evidence\tests\torus_pairs.rs:282  assert!(matches!(record.locus, ContactLocus::Coincident));
ContactLocus  vendor\truck\truck-evidence\tests\torus_pairs.rs:318  let ContactLocus::ValidatedBranchCover(cover) = &record.locus else {
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:6  //! bounded to both strata. The bounded locus forms are `ContactLocus::Point`
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:7  //! (an isolated contact point) and `ContactLocus::BoundedCurve` (an exact
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:64  use super::{ContactComplex, ContactLocus, ContactRecord};
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:114  locus: ContactLocus::Point(q),
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:123  locus: ContactLocus::BoundedCurve { curve, t_range },
ContactLocus  vendor\truck\truck-evidence\src\contact\fe_ee.rs:1332  fn loci_equal(a: &ContactLocus, b: &ContactLocus) -> bool {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:528  fn plane_plane_line(a: &PlaneSchema, b: &PlaneSchema) -> ContactLocus {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:537  ContactLocus::Line {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:565  locus: ContactLocus::Circle {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:601  /// [`ContactLocus::Line`]) and refuses `UnsupportedPairClass`.
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:625  locus: ContactLocus::Line {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:680  locus: ContactLocus::Circle {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:690  locus: ContactLocus::Point { point: foot() },
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:741  locus: ContactLocus::Point { point },
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:771  locus: ContactLocus::Circle {
ContactLocus  vendor\truck\truck-certified\src\pair_dispatch.rs:830  locus: ContactLocus::Line {
ContactLocus  vendor\truck\truck-certified\src\ssi_admit.rs:637  use truck_evidence::contact::{spline_analytic_contact, take_spline_ssi_entry, ContactLocus};
ContactLocus  vendor\truck\truck-certified\src\ssi_admit.rs:812  let ContactLocus::Point(point) = record.locus else {
ContactLocus  vendor\truck\truck-shapeops\tests\fillet_circle.rs:37  use truck_evidence::contact::{contact, BoundedStratum, ContactLocus};
ContactLocus  vendor\truck\truck-shapeops\tests\fillet_circle.rs:433  let ContactLocus::ValidatedBranchCover(cover) = &record.locus else {
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:39  use truck_evidence::contact::{contact, face_stratum, BoundedStratum, ContactLocus};
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:375  (ContactLocus::Coincident, ContactDimension::Arc1)
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:377  ContactLocus::Analytic(AnalyticIntersection::Coincident),
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1082  use truck_evidence::contact::{sweep_stratum, ContactLocus};
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1209  .filter(|e| matches!(&e.record.locus, ContactLocus::Coincident))
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1216  ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(_)))
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1222  .filter(|e| matches!(&e.record.locus, ContactLocus::BoundedCurve { .. }))
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1286  let ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(c))) =
ContactLocus  vendor\truck\truck-shapeops\src\boolean\assemble.rs:1325  let ContactLocus::BoundedCurve { curve, t_range } = &e.record.locus else {
ContactLocus  vendor\truck\truck-shapeops\src\boolean\classify.rs:883  use truck_evidence::contact::{ContactLocus, ContactRecord};
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:240  Contact(CertifiedPairContact),
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:506  return CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:562  return CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:622  CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:677  CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:687  CertifiedSign::Zero => CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:738  return CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:768  CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\src\pair_dispatch.rs:827  CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairContact  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:174  fn expect_contact(result: CertifiedPairResult) -> CertifiedPairContact {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:253  /// [`CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)`]
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:282  _ => CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass),
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:501  fn plane_plane(a: PlaneSchema, b: PlaneSchema) -> CertifiedPairResult {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:506  return CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:516  CertifiedPairResult::Unsupported(PairUnsupported::Overlap)
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:518  CertifiedPairResult::Disjoint
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:548  fn plane_cylinder(plane: PlaneSchema, cyl: CertifiedEmbeddedCylinder) -> CertifiedPairResult {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:562  return CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:582  CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:622  CertifiedPairResult::Contact(CertifiedPairContact {
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:631  CertifiedSign::Positive => CertifiedPairResult::Disjoint,
CertifiedPairResult  vendor\truck\truck-certified\src\pair_dispatch.rs:633  CertifiedPairResult::Unsupported(PairUnsupported::UnsupportedPairClass)
dispatch_pair  vendor\truck\truck-certified\src\pair_dispatch.rs:64  //! participant identity and `dispatch_pair(a, b) == dispatch_pair(b, a)` (a
dispatch_pair  vendor\truck\truck-certified\src\pair_dispatch.rs:250  /// The pair is sorted by participant identity, so `dispatch_pair(a, b) ==
dispatch_pair  vendor\truck\truck-certified\src\pair_dispatch.rs:251  /// dispatch_pair(b, a)`. Unroutable classes (any side the enum cannot carry,
dispatch_pair  vendor\truck\truck-certified\src\pair_dispatch.rs:255  pub fn dispatch_pair(
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:15  //! - `dispatch_pair(a, b) == dispatch_pair(b, a)` across all arms;
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:199  let result = dispatch_pair(&a, &b);
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:227  assert_eq!(dispatch_pair(&a, &b), CertifiedPairResult::Disjoint);
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:238  dispatch_pair(&a, &c),
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:258  let contact = expect_contact(dispatch_pair(&plane, &cyl));
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:295  let contact = expect_contact(dispatch_pair(&tangent, &cyl));
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:311  assert_eq!(dispatch_pair(&offset, &cyl), CertifiedPairResult::Disjoint);
dispatch_pair  vendor\truck\truck-certified\tests\pair_dispatch_conformance.rs:323  let contact = expect_contact(dispatch_pair(&plane, &sphere));

