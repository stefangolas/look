#![allow(clippy::too_many_arguments)]

use proptest::{property_test, test_runner::TestCaseError, *};
use ruststep::{ast::DataSection, tables::*};
use std::{f64::consts::PI, str::FromStr};
use truck_geometry::prelude as truck;
use truck_stepio::{
    out::*,
    r#in::{step_geometry::*, *},
};

fn float_to_str(x: f64) -> String {
    if f64::abs(x) < 1.0e-6 {
        "0.0".to_string()
    } else if f64::abs(x) < 1.0e-2 && x != 0.0 {
        format!("{x:.7E}")
    } else {
        format!("{x:?}")
    }
}

/// create uniform unit vector from [0.0f64..1.0f64; 2]
fn dir_from_array(arr: [f64; 2]) -> Vector3 {
    let z = 2.0 * arr[1] - 1.0;
    let theta = 2.0 * PI * arr[0];
    let r = f64::sqrt(f64::max(1.0 - z * z, 0.0));
    Vector3::new(r * f64::cos(theta), r * f64::sin(theta), z)
}

fn step_to_entity<THolder>(step_str: &str) -> THolder::Owned
where
    THolder: Holder<Table = Table>,
    Table: EntityTable<THolder>,
{
    let data_section = DataSection::from_str(step_str).unwrap();
    let table = Table::from_data_section(&data_section);
    EntityTable::<THolder>::get_owned(&table, 1).unwrap()
}

fn exec_test_near<THolder, T>(ans: T, step_str: &str)
where
    THolder: Holder<Table = Table>,
    Table: EntityTable<THolder>,
    T: for<'a> From<&'a THolder::Owned> + std::fmt::Debug + Tolerance,
{
    let entity = step_to_entity(step_str);
    let res = T::from(&entity);
    assert_near!(res, ans);
}

fn exec_cartesian_point(coord: [f64; 3]) {
    let pt = Point2::new(coord[0], coord[1]);
    exec_test_near::<CartesianPointHolder, Point2>(
        pt,
        &format!(
            "DATA;{}ENDSEC;",
            truck_stepio::out::StepDataDisplay::new(pt, 1)
        ),
    );
    let pt = Point3::from(coord);
    exec_test_near::<CartesianPointHolder, Point3>(
        pt,
        &format!(
            "DATA;{}ENDSEC;",
            truck_stepio::out::StepDataDisplay::new(pt, 1)
        ),
    );
}

#[property_test]
fn cartesian_point(#[strategy = array::uniform3(-100.0f64..100.0f64)] coord: [f64; 3]) {
    exec_cartesian_point(coord)
}

fn exec_direction(dir_array: [f64; 2]) {
    let theta = 2.0 * PI * dir_array[0];
    let vec = Vector2::new(f64::cos(theta), f64::sin(theta));
    exec_test_near::<DirectionHolder, Vector2>(
        vec,
        &format!(
            "DATA;#1 = DIRECTION('', ({}, {}));ENDSEC;",
            float_to_str(vec[0]),
            float_to_str(vec[1])
        ),
    );
    let vec = dir_from_array(dir_array);
    exec_test_near::<DirectionHolder, Vector3>(
        vec,
        &format!(
            "DATA;#1 = DIRECTION('', ({}, {}, {}));ENDSEC;",
            float_to_str(vec[0]),
            float_to_str(vec[1]),
            float_to_str(vec[2])
        ),
    );
}

#[property_test]
fn direction(#[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2]) {
    exec_direction(dir_array)
}

fn exec_vector(elem: [f64; 3]) {
    let vec = Vector2::new(elem[0], elem[1]);
    exec_test_near::<VectorHolder, Vector2>(
        vec,
        &format!("DATA;{}ENDSEC;", StepDataDisplay::new(vec, 1)),
    );
    let vec = Vector3::from(elem);
    exec_test_near::<VectorHolder, Vector3>(
        vec,
        &format!("DATA;{}ENDSEC;", StepDataDisplay::new(vec, 1)),
    );
}

#[property_test]
fn vector(#[strategy = array::uniform3(-100.0f64..100.0f64)] elem: [f64; 3]) {
    exec_vector(elem)
}

fn exec_placement(org_coord: [f64; 3]) {
    let org = Point2::new(org_coord[0], org_coord[1]);
    exec_test_near::<PlacementHolder, Point2>(
        org,
        &format!(
            "DATA;#1 = PLACEMENT('', #2);{}ENDSEC;",
            StepDataDisplay::new(org, 2)
        ),
    );
    let org = Point3::from(org_coord);
    exec_test_near::<PlacementHolder, Point3>(
        org,
        &format!(
            "DATA;#1 = PLACEMENT('', #2);{}ENDSEC;",
            StepDataDisplay::new(org, 2)
        ),
    );
}

#[property_test]
fn placement(#[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3]) {
    exec_placement(org_coord)
}

fn exec_axis1_placement(org_coord: [f64; 3], dir_array: [f64; 2]) {
    let p = Point2::new(org_coord[0], org_coord[1]);
    let theta = 2.0 * PI * dir_array[0];
    let dir = Vector2::new(f64::cos(theta), f64::sin(theta));
    let step_str = format!(
        "DATA;#1 = AXIS1_PLACEMENT('', #2, #3);{}{}ENDSEC;",
        StepDataDisplay::new(p, 2),
        StepDataDisplay::new(VectorAsDirection(dir), 3)
    );
    let placement = step_to_entity::<Axis1PlacementHolder>(&step_str);
    assert_near!(p, Point2::from(&placement.location));
    assert_near!(dir, placement.direction().truncate());

    let p = Point3::from(org_coord);
    let dir = dir_from_array(dir_array);
    let step_str = format!(
        "DATA;#1 = AXIS1_PLACEMENT('', #2, #3);{}{}ENDSEC;",
        StepDataDisplay::new(p, 2),
        StepDataDisplay::new(VectorAsDirection(dir), 3)
    );
    let placement = step_to_entity::<Axis1PlacementHolder>(&step_str);
    assert_near!(p, Point3::from(&placement.location));
    assert_near!(dir, placement.direction());
}

#[property_test]
fn axis1_placement(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2],
) {
    exec_axis1_placement(org_coord, dir_array)
}

fn exec_axis2_placement2d(org_coord: [f64; 2], theta: f64) {
    let origin = Point2::from(org_coord);
    let dir = Vector2::new(f64::cos(theta), f64::sin(theta));
    let step_str = format!(
        "DATA;#1 = AXIS2_PLACEMENT_2D('', #2, #3);{}{}ENDSEC;",
        StepDataDisplay::new(origin, 2),
        StepDataDisplay::new(VectorAsDirection(dir), 3),
    );
    let placement = step_to_entity::<Axis2Placement2dHolder>(&step_str);
    let res: Matrix3 = (&placement).into();
    let n = Vector2::new(-dir.y, dir.x);
    let ans = Matrix3::from_cols(dir.extend(0.0), n.extend(0.0), origin.to_vec().extend(1.0));
    assert_near!(res, ans);
}

#[property_test]
fn axis2_placement_2d(
    #[strategy = array::uniform2(-100.0f64..100.0f64)] org_coord: [f64; 2],
    #[strategy = 0.0f64..2.0 * PI] theta: f64,
) {
    exec_axis2_placement2d(org_coord, theta)
}

fn exec_axis2_placement3d(org_coord: [f64; 3], dir_array: [f64; 2], ref_dir_array: [f64; 2]) {
    let p = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA;#1 = AXIS2_PLACEMENT_3D('', #2, #3, #4);{}{}{}ENDSEC;",
        StepDataDisplay::new(p, 2),
        StepDataDisplay::new(VectorAsDirection(z), 3),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 4),
    );
    let placement = step_to_entity::<Axis2Placement3dHolder>(&step_str);
    let res: Matrix4 = (&placement).into();
    let ans = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        p.to_vec().extend(1.0),
    );
    assert_near!(res, ans);
}

#[property_test]
fn axis2_placement_3d(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
) {
    exec_axis2_placement3d(org_coord, dir_array, ref_dir_array)
}

fn exec_line(org_coord: [f64; 3], vec_elem: [f64; 3]) {
    let p = Point3::from(org_coord);
    let v = Vector3::from(vec_elem);
    let q = p + v;
    let step_str = format!(
        "DATA;#1 = LINE('', #2, #3);{}{}ENDSEC;",
        StepDataDisplay::new(p, 2),
        StepDataDisplay::new(v, 3),
    );
    let line = step_to_entity::<LineHolder>(&step_str);
    let res: truck::Line<Point3> = (&line).into();
    let ans = truck::Line(p, q);
    assert_near!(res.0, ans.0);
    assert_near!(res.1, ans.1);
}

#[property_test]
fn line(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform3(-100.0f64..100.0f64)] vec_elem: [f64; 3],
) {
    exec_line(org_coord, vec_elem)
}

fn exec_polyline(length: usize, coords: Vec<[f64; 3]>) {
    let p = coords
        .into_iter()
        .take(length)
        .map(Point3::from)
        .collect::<Vec<_>>();
    let point_displays = p
        .iter()
        .enumerate()
        .map(|(idx, p)| StepDataDisplay::new(p, 2 + idx).to_string())
        .collect::<Vec<_>>()
        .concat();
    let index_slice = (0..length).map(|idx| 2 + idx);
    let step_str = format!(
        "DATA;#1 = POLYLINE('', {});{}ENDSEC;",
        IndexSliceDisplay(index_slice),
        point_displays
    );
    let polyline = step_to_entity::<PolylineHolder>(&step_str);
    let tpoly: PolylineCurve<Point3> = (&polyline).into();
    let res = tpoly.0;
    let ans = p;
    assert_eq!(res.len(), ans.len());
    res.into_iter()
        .zip(ans)
        .for_each(|(p, q)| assert_near!(p, q));
}

#[property_test]
fn polyline(
    #[strategy = 2usize..100] length: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 100)] coords: Vec<[f64; 3]>,
) {
    exec_polyline(length, coords)
}

fn exec_b_spline_curve_with_knots(
    knot_len: usize,
    knot_incrs: Vec<f64>,
    knot_mults: Vec<usize>,
    degree: usize,
    ctrlpt_coords: Vec<[f64; 3]>,
) -> std::result::Result<(), TestCaseError> {
    let mut s = 0.0;
    let vec = knot_mults
        .iter()
        .take(knot_len)
        .zip(knot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let knots = KnotVec::from(vec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_curve_with_knots_degenerate_active_domain_refuses
    prop_assume!(knots[degree] < knots[knots.len() - degree - 1]);
    let cps = ctrlpt_coords
        .into_iter()
        .take(knots.len() - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    let bsp = BSplineCurve::new(knots, cps);
    let step_str = format!("DATA;{}ENDSEC;", StepDataDisplay::new(&bsp, 1));
    let bsp_step = step_to_entity::<BSplineCurveWithKnotsHolder>(&step_str);
    let res: BSplineCurve<Point3> = (&bsp_step).try_into().unwrap();
    assert_eq!(res.knot_vec().len(), bsp.knot_vec().len());
    assert_eq!(res.control_points().len(), bsp.control_points().len());
    res.knot_vec()
        .iter()
        .zip(bsp.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(bsp.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
    Ok(())
}

#[property_test]
fn b_spline_curve_with_knots(
    #[strategy = 7usize..20] knot_len: usize,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 20)] knot_incrs: Vec<f64>,
    #[strategy = collection::vec(1usize..4usize, 20)] knot_mults: Vec<usize>,
    #[strategy = 2usize..6] degree: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 80)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
) -> std::result::Result<(), TestCaseError> {
    exec_b_spline_curve_with_knots(knot_len, knot_incrs, knot_mults, degree, ctrlpt_coords)
}

/// A `b_spline_curve_with_knots` whose knot interval is nonzero but smaller
/// than truck's `TOLERANCE` (absolute) must still convert. The exporter chose
/// a tiny parameter span; normalizing the knot vector to `[0, 1]` is an exact,
/// shape-preserving reparameterization of the same curve, which is the
/// canonical geometric interpretation the source justifies.
#[test]
fn b_spline_curve_with_knots_tiny_knot_interval_converts() {
    let degree = 3;
    let points: Vec<Point3> = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, -0.5),
        Point3::new(2.0, 1.0, 0.25),
        Point3::new(3.0, 3.0, 1.0),
    ];
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    // A knot span of 5e-7: below TOLERANCE, above a true zero range.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_CURVE_WITH_KNOTS('', {degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., (4,4), (0.0, 5.0E-7), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineCurveWithKnotsHolder>(&step_str);
    let res: BSplineCurve<Point3> = (&bsp_step).try_into().expect("tiny-range curve converts");
    assert_eq!(res.knot_vec().len(), 8, "knot count preserved");
    res.knot_vec()
        .iter()
        .zip([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0])
        .for_each(|(x, y)| {
            assert_near!(*x, y, "normalized to [0, 1]");
        });
    assert_eq!(res.control_points().len(), points.len());
    // The normalized curve is exactly the Bezier curve the source control
    // points define: a clamped cubic with knot vector `[0,0,0,0,1,1,1,1]`
    // evaluates to the same 3D points as the source parameterization over
    // `[0, 5e-7]` (a linear reparameterization changes nothing geometric).
    for i in 0..=100 {
        let t = i as f64 / 100.0;
        let a = (1.0 - t).powi(3);
        let b = 3.0 * (1.0 - t) * (1.0 - t) * t;
        let c = 3.0 * (1.0 - t) * t * t;
        let d = t.powi(3);
        let ans = points[0].to_vec() * a
            + points[1].to_vec() * b
            + points[2].to_vec() * c
            + points[3].to_vec() * d;
        let got = res.subs(t).to_vec();
        assert!(
            (got - ans).magnitude() < 1.0e-9,
            "t:{t} got:{got:?} ans:{ans:?}"
        );
    }
}

/// A malformed knot vector the source does not justify repairing must still
/// refuse. An unsorted (decreasing) knot sequence is not a reparameterization
/// of anything, so conversion must refuse rather than normalize it away.
#[test]
fn b_spline_curve_with_knots_unsorted_knots_still_refuse() {
    let degree = 3;
    let points: Vec<Point3> = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, -0.5),
        Point3::new(2.0, 1.0, 0.25),
        Point3::new(3.0, 3.0, 1.0),
    ];
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_CURVE_WITH_KNOTS('', {degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., (4,4), (0.5, 0.4), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineCurveWithKnotsHolder>(&step_str);
    assert!(
        BSplineCurve::<Point3>::try_from(&bsp_step).is_err(),
        "an unsorted knot vector must still refuse",
    );
}

/// A supplied knot vector whose active domain collapses to a single value is
/// not a reparameterization of anything, so conversion must refuse rather than
/// substitute synthesized knots for the source's.
#[test]
fn b_spline_curve_with_knots_degenerate_active_domain_refuses() {
    let degree = 3;
    let points: Vec<Point3> = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, -0.5),
        Point3::new(2.0, 1.0, 0.25),
        Point3::new(3.0, 3.0, 1.0),
    ];
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    // `knot_multiplicities (5,3)` over `knots (0.0, 1.0)` expands to eight
    // knots, but the active domain is `[T_3, T_4] = [0.0, 0.0]`.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_CURVE_WITH_KNOTS('', {degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., (5,3), (0.0, 1.0), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineCurveWithKnotsHolder>(&step_str);
    assert!(
        BSplineCurve::<Point3>::try_from(&bsp_step).is_err(),
        "a degenerate active domain must refuse",
    );
}

/// A supplied knot vector whose expanded length is short of `N + degree + 1`
/// must refuse instead of silently replacing the source knots.
#[test]
fn b_spline_curve_with_knots_knot_count_mismatch_refuses() {
    let degree = 3;
    let points: Vec<Point3> = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, -0.5),
        Point3::new(2.0, 1.0, 0.25),
        Point3::new(3.0, 3.0, 1.0),
    ];
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    // `knot_multiplicities (2,2)` over `knots (0.0, 1.0)` expands to four
    // knots, but `N = 4`, `degree = 3` need eight.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_CURVE_WITH_KNOTS('', {degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., (2,2), (0.0, 1.0), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineCurveWithKnotsHolder>(&step_str);
    assert!(
        BSplineCurve::<Point3>::try_from(&bsp_step).is_err(),
        "a knot-count mismatch must refuse",
    );
}

/// `QUASI_UNIFORM_CURVE` has no source knot list, but a degree that exceeds the
/// control-point count cannot be synthesized either; it must refuse.
#[test]
fn quasi_uniform_curve_degree_exceeds_control_points_refuses() {
    let degree = 5;
    let points: Vec<Point3> = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, -0.5),
        Point3::new(2.0, 1.0, 0.25),
    ];
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = QUASI_UNIFORM_CURVE('', {degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<QuasiUniformCurveHolder>(&step_str);
    assert!(
        BSplineCurve::<Point3>::try_from(&bsp_step).is_err(),
        "a degree exceeding the control-point count must refuse",
    );
}

fn step_bsp_curve_ctrls(points: &[Point3]) -> (String, String) {
    (
        IndexSliceDisplay((0..points.len()).map(|i| 2 + i)).to_string(),
        points
            .iter()
            .enumerate()
            .map(|(i, p)| StepDataDisplay::new(*p, i + 2).to_string())
            .collect::<Vec<_>>()
            .concat(),
    )
}

fn exec_bezier_curve(degree: usize, ctrlpt_coords: Vec<[f64; 3]>) {
    let points = ctrlpt_coords
        .into_iter()
        .take(degree + 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = BEZIER_CURVE('', {degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BezierCurveHolder>(&step_str);
    let res: BSplineCurve<Point3> = (&bsp_step).try_into().unwrap();
    let ans = BSplineCurve::new(KnotVec::bezier_knot(degree), points);
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn bezier_curve(
    #[strategy = 1usize..6] degree: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 6)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
) {
    exec_bezier_curve(degree, ctrlpt_coords)
}

fn exec_quasi_uniform_curve(degree: usize, division: usize, ctrlpt_coords: Vec<[f64; 3]>) {
    let mut knots = KnotVec::uniform_knot(degree, division);
    knots.transform(division as f64, 0.0);
    let points = ctrlpt_coords
        .into_iter()
        .take(knots.len() - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = QUASI_UNIFORM_CURVE('', {degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<QuasiUniformCurveHolder>(&step_str);
    let res: BSplineCurve<Point3> = (&bsp_step).try_into().unwrap();
    let ans = BSplineCurve::new(knots, points);
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn quasi_uniform_curve(
    #[strategy = 1usize..4] degree: usize,
    #[strategy = 3usize..5] division: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 20)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
) {
    exec_quasi_uniform_curve(degree, division, ctrlpt_coords)
}

fn exec_uniform_curve(degree: usize, knot_len: usize, ctrlpt_coords: Vec<[f64; 3]>) {
    let knots = KnotVec::from_iter((0..knot_len).map(|i| i as f64 - degree as f64));
    let points = ctrlpt_coords
        .into_iter()
        .take(knot_len - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = UNIFORM_CURVE('', {degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<UniformCurveHolder>(&step_str);
    let res: BSplineCurve<Point3> = (&bsp_step).try_into().unwrap();
    let ans = BSplineCurve::new(knots, points);
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn uniform_curve(
    #[strategy = 1usize..4] degree: usize,
    #[strategy = 6usize..10] knot_len: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 40)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
) {
    exec_uniform_curve(degree, knot_len, ctrlpt_coords)
}

fn exec_nurbs_curve_b_spline_with_knots(
    knot_len: usize,
    knot_incrs: Vec<f64>,
    knot_mults: Vec<usize>,
    mut weights: Vec<f64>,
    degree: usize,
    ctrlpt_coords: Vec<[f64; 3]>,
) -> std::result::Result<(), TestCaseError> {
    let mut s = 0.0;
    let vec = knot_mults
        .iter()
        .take(knot_len)
        .zip(knot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let knots = KnotVec::from(vec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_curve_with_knots_degenerate_active_domain_refuses
    prop_assume!(knots[degree] < knots[knots.len() - degree - 1]);
    let cps = ctrlpt_coords
        .into_iter()
        .take(knots.len() - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    weights.truncate(cps.len());
    let bsp = BSplineCurve::new(knots, cps);
    let nurbs = NurbsCurve::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    let step_str = format!("DATA;{}ENDSEC;", StepDataDisplay::new(&nurbs, 1));
    let nurbs_step = step_to_entity::<RationalBSplineCurveHolder>(&step_str);
    let res: NurbsCurve<Vector4> = (&nurbs_step).try_into().unwrap();
    assert_eq!(res.knot_vec().len(), nurbs.knot_vec().len());
    assert_eq!(res.control_points().len(), nurbs.control_points().len());
    res.knot_vec()
        .iter()
        .zip(nurbs.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(nurbs.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
    Ok(())
}

#[property_test]
fn nurbs_curve_b_spline_curve_with_knots(
    #[strategy = 7usize..20] knot_len: usize,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 20)] knot_incrs: Vec<f64>,
    #[strategy = collection::vec(1usize..4usize, 20)] knot_mults: Vec<usize>,
    #[strategy = collection::vec(0.01f64..100.0, 80)] weights: Vec<f64>,
    #[strategy = 2usize..6] degree: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 80)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
) -> std::result::Result<(), TestCaseError> {
    exec_nurbs_curve_b_spline_with_knots(
        knot_len,
        knot_incrs,
        knot_mults,
        weights,
        degree,
        ctrlpt_coords,
    )
}

fn exec_nurbs_curve_bezier_curve(
    degree: usize,
    ctrlpt_coords: Vec<[f64; 3]>,
    mut weights: Vec<f64>,
) {
    let points = ctrlpt_coords
        .into_iter()
        .take(degree + 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    weights.truncate(points.len());
    let weights_display = SliceDisplay(&weights);
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BEZIER_CURVE()
    BOUNDED_CURVE()
    B_SPLINE_CURVE({degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE({weights_display})
    REPRESENTATION_ITEM('')
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineCurveHolder>(&step_str);
    let res: NurbsCurve<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineCurve::new(KnotVec::bezier_knot(degree), points);
    let ans = NurbsCurve::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn nurbs_curve_bezier_curve(
    #[strategy = 1usize..6] degree: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 6)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
    #[strategy = collection::vec(0.01f64..100.0, 6)] weights: Vec<f64>,
) {
    exec_nurbs_curve_bezier_curve(degree, ctrlpt_coords, weights)
}

fn exec_nurbs_curve_quasi_uniform_curve(
    degree: usize,
    division: usize,
    ctrlpt_coords: Vec<[f64; 3]>,
    mut weights: Vec<f64>,
) {
    let mut knots = KnotVec::uniform_knot(degree, division);
    knots.transform(division as f64, 0.0);
    let points = ctrlpt_coords
        .into_iter()
        .take(knots.len() - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    weights.truncate(points.len());
    let weights_display = SliceDisplay(&weights);
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE({degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    QUASI_UNIFORM_CURVE()
    RATIONAL_B_SPLINE_CURVE({weights_display})
    REPRESENTATION_ITEM('')
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineCurveHolder>(&step_str);
    let res: NurbsCurve<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineCurve::new(knots, points);
    let ans = NurbsCurve::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn nurbs_curve_quasi_uniform_curve(
    #[strategy = 1usize..4] degree: usize,
    #[strategy = 3usize..5] division: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 20)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
    #[strategy = collection::vec(0.01f64..100.0, 20)] weights: Vec<f64>,
) {
    exec_nurbs_curve_quasi_uniform_curve(degree, division, ctrlpt_coords, weights)
}

fn exec_nurbs_curve_uniform_curve(
    degree: usize,
    knot_len: usize,
    ctrlpt_coords: Vec<[f64; 3]>,
    mut weights: Vec<f64>,
) {
    let knots = KnotVec::from_iter((0..knot_len).map(|i| i as f64 - degree as f64));
    let points = ctrlpt_coords
        .into_iter()
        .take(knot_len - degree - 1)
        .map(Point3::from)
        .collect::<Vec<_>>();
    weights.truncate(points.len());
    let weights_display = SliceDisplay(&weights);
    let (step_cps_indices, step_cps) = step_bsp_curve_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE({degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE({weights_display})
    REPRESENTATION_ITEM('')
    UNIFORM_CURVE()
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineCurveHolder>(&step_str);
    let res: NurbsCurve<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineCurve::new(knots, points);
    let ans = NurbsCurve::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    assert_eq!(res.knot_vec().len(), ans.knot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.knot_vec()
        .iter()
        .zip(ans.knot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .zip(ans.control_points())
        .for_each(|(x, y)| assert_near!(x, y));
}

#[property_test]
fn nurbs_curve_uniform_curve(
    #[strategy = 1usize..4] degree: usize,
    #[strategy = 6usize..10] knot_len: usize,
    #[strategy = collection::vec(array::uniform3(-100.0f64..100.0f64), 40)] ctrlpt_coords: Vec<
        [f64; 3],
    >,
    #[strategy = collection::vec(0.01f64..100.0, 40)] weights: Vec<f64>,
) {
    exec_nurbs_curve_uniform_curve(degree, knot_len, ctrlpt_coords, weights)
}

fn exec_circle(org_coord: [f64; 3], dir_array: [f64; 2], ref_dir_array: [f64; 2], radius: f64) {
    let origin = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA; #1 = CIRCLE('', #2, {radius}); #2 = AXIS2_PLACEMENT_3D('', #3, #4, #5); {}{}{}ENDSEC;",
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_circle = step_to_entity::<CircleHolder>(&step_str);
    let ellipse: step_geometry::Ellipse<Point3, Matrix4> = (&step_circle).try_into().unwrap();
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        origin.to_vec().extend(1.0),
    );
    (0..10).for_each(|i| {
        let t = 2.0 * PI * i as f64 / 10.0;
        let p = Point3::new(radius * f64::cos(t), radius * f64::sin(t), 0.0);
        assert_near!(ellipse.subs(t), mat.transform_point(p));
    });
}

#[property_test]
fn circle(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0)] ref_dir_array: [f64; 2],
    #[strategy = 1.0e-2f64..100.0] radius: f64,
) {
    exec_circle(org_coord, dir_array, ref_dir_array, radius)
}

/// The source conic family survives import.
///
/// `circle` and `ellipse` realize to the same geometry — a unit circle under
/// an affine transform — so before this distinction existed both landed in
/// `Conic3D::Ellipse` and every consumer had to re-prove circularity from a
/// transform built out of the file's finite-precision direction cosines. That
/// transform is a similarity in exact arithmetic and not after rounding, so an
/// exact predicate refuses a perfectly ordinary `circle`. Keeping the family
/// is what lets a consumer ask the right question.
///
/// The direction cosines below are deliberately neither axis-aligned nor
/// exactly representable, so the derived basis is orthonormal only to rounding.
#[test]
fn the_source_conic_family_survives_import() {
    let placement = "#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);         #3 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));         #4 = DIRECTION('', (0.3, 0.5, 0.81));         #5 = DIRECTION('', (0.77, -0.13, 0.62));";

    let circle = step_to_entity::<ConicHolder>(&format!(
        "DATA; #1 = CIRCLE('', #2, 3.7); {placement} ENDSEC;"
    ));
    let conic: step_geometry::Conic3D = (&circle).try_into().unwrap();
    assert!(
        matches!(conic, step_geometry::Conic3D::Circle(_)),
        "a source CIRCLE must import as Conic3D::Circle, got {conic:?}"
    );

    let ellipse = step_to_entity::<ConicHolder>(&format!(
        "DATA; #1 = ELLIPSE('', #2, 3.7, 3.7); {placement} ENDSEC;"
    ));
    let conic: step_geometry::Conic3D = (&ellipse).try_into().unwrap();
    // Equal semi-axes: geometrically a circle, and still an `ellipse`. The
    // family is what the source said, never what the numbers look like.
    assert!(
        matches!(conic, step_geometry::Conic3D::Ellipse(_)),
        "a source ELLIPSE must stay Conic3D::Ellipse, got {conic:?}"
    );
}

fn exec_ellipse(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radius: [f64; 2],
) {
    let origin = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA; #1 = ELLIPSE('', #2, {}, {}); #2 = AXIS2_PLACEMENT_3D('', #3, #4, #5); {}{}{}ENDSEC;",
        FloatDisplay(radius[0]),
        FloatDisplay(radius[1]),
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_ellipse = step_to_entity::<EllipseHolder>(&step_str);
    let ellipse: step_geometry::Ellipse<Point3, Matrix4> = (&step_ellipse).try_into().unwrap();
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        origin.to_vec().extend(1.0),
    );
    (0..10).for_each(|i| {
        let t = 2.0 * PI * i as f64 / 10.0;
        let p = Point3::new(radius[0] * f64::cos(t), radius[1] * f64::sin(t), 0.0);
        assert_near!(ellipse.subs(t), mat.transform_point(p));
    });
}

#[property_test]
fn ellipse(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0)] ref_dir_array: [f64; 2],
    #[strategy = array::uniform2(1.0e-2f64..100.0)] radius: [f64; 2],
) {
    exec_ellipse(org_coord, dir_array, ref_dir_array, radius)
}

fn exec_hyperbola(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radius: [f64; 2],
) {
    let origin = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA; #1 = HYPERBOLA('', #2, {}, {}); #2 = AXIS2_PLACEMENT_3D('', #3, #4, #5); {}{}{}ENDSEC;",
        FloatDisplay(radius[0]),
        FloatDisplay(radius[1]),
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_hyperbola = step_to_entity::<HyperbolaHolder>(&step_str);
    let hyperbola: step_geometry::Hyperbola<Point3, Matrix4> =
        (&step_hyperbola).try_into().unwrap();
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        origin.to_vec().extend(1.0),
    );
    (0..10).for_each(|i| {
        let t = 2.0 * i as f64 / 10.0 - 1.0;
        let p = Point3::new(radius[0] * f64::cosh(t), radius[1] * f64::sinh(t), 0.0);
        assert_near!(hyperbola.subs(t), mat.transform_point(p));
    });
}

#[property_test]
fn hyperbola(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0)] ref_dir_array: [f64; 2],
    #[strategy = array::uniform2(1.0e-2f64..100.0)] radius: [f64; 2],
) {
    exec_hyperbola(org_coord, dir_array, ref_dir_array, radius)
}

fn exec_parabola(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    focal_dist: f64,
) {
    let origin = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA; #1 = PARABOLA('', #2, {}); #2 = AXIS2_PLACEMENT_3D('', #3, #4, #5); {}{}{}ENDSEC;",
        FloatDisplay(focal_dist),
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_parabola = step_to_entity::<ParabolaHolder>(&step_str);
    let parabola: step_geometry::Parabola<Point3, Matrix4> = (&step_parabola).try_into().unwrap();
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        origin.to_vec().extend(1.0),
    );
    (0..10).for_each(|i| {
        let t = 2.0 * i as f64 / 10.0 - 1.0;
        let p = Point3::new(focal_dist * t * t, focal_dist * 2.0 * t, 0.0);
        assert_near!(parabola.subs(t), mat.transform_point(p));
    });
}

#[property_test]
fn parabola(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0)] ref_dir_array: [f64; 2],
    #[strategy = 0.01f64..100.0] focal_dist: f64,
) {
    exec_parabola(org_coord, dir_array, ref_dir_array, focal_dist)
}

fn exec_plane(org_coord: [f64; 3], dir_array: [f64; 2], ref_dir_array: [f64; 2]) {
    let origin = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA;
#1 = PLANE('', #2);
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_plane = step_to_entity::<PlaneHolder>(&step_str);
    let plane = truck::Plane::from(&step_plane);
    assert_near!(plane.origin(), origin);
    assert_near!(plane.u_axis(), x);
    assert_near!(plane.v_axis(), y);
}

#[property_test]
fn plane(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
) {
    exec_plane(org_coord, dir_array, ref_dir_array)
}

fn exec_spherical_surface(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radius: f64,
) {
    let p = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA;
#1 = SPHERICAL_SURFACE('', #2, {radius});
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(p, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_sphere = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str);
    let sphere: step_geometry::ElementarySurface = (&step_sphere)
        .try_into()
        .expect("elementary surface conversion");
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        p.to_vec().extend(1.0),
    );
    (0..=10)
        .flat_map(move |i| (0..=10).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let u = 2.0 * PI * i as f64 / 10.0;
            let v = PI * j as f64 / 10.0 - PI / 2.0;
            let res = sphere.subs(u, v);
            let ans = mat.transform_point(Point3::new(
                radius * f64::cos(u) * f64::cos(v),
                radius * f64::sin(u) * f64::cos(v),
                radius * f64::sin(v),
            ));
            assert_near!(res, ans);
        })
}

#[property_test]
fn spherical_surface(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
    #[strategy = 1.0e-2f64..100.0f64] radius: f64,
) {
    exec_spherical_surface(org_coord, dir_array, ref_dir_array, radius)
}

fn exec_cylindrical_surface(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radius: f64,
) {
    let p = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str0 = format!(
        "DATA;
#1 = CYLINDRICAL_SURFACE('', #2, {radius});
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(p, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_cylinder0 = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str0);
    let cylinder0: step_geometry::ElementarySurface = (&step_cylinder0)
        .try_into()
        .expect("elementary surface conversion");

    // It has its own output, so test it accordingly.
    let step_str1 = format!("DATA;\n{}ENDSEC;", StepDataDisplay::new(&cylinder0, 1));
    let step_cylinder1 = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str1);
    let cylinder1: step_geometry::ElementarySurface = (&step_cylinder1)
        .try_into()
        .expect("elementary surface conversion");

    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        p.to_vec().extend(1.0),
    );
    (0..=10)
        .flat_map(move |i| (0..=10).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let u = 2.0 * PI * i as f64 / 10.0;
            let v = j as f64;
            let res0 = cylinder0.subs(u, v);
            let res1 = cylinder1.subs(u, v);
            let ans =
                mat.transform_point(Point3::new(radius * f64::cos(u), radius * f64::sin(u), v));
            assert_near!(res0, ans, "u:{u} v:{v} res:{res0:?} ans:{ans:?}");
            assert_near!(res1, ans, "u:{u} v:{v} res:{res1:?} ans:{ans:?}");
        })
}

#[property_test]
fn cylindrical_surface(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
    #[strategy = 1.0e-2f64..100.0f64] radius: f64,
) {
    exec_cylindrical_surface(org_coord, dir_array, ref_dir_array, radius)
}

fn exec_toroidal_surface(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radii: [f64; 2],
) {
    let p = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let major_radius = f64::max(radii[0], radii[1]);
    let minor_radius = (f64::min(radii[0], radii[1])) / 2.0;
    let step_str = format!(
        "DATA;
#1 = TOROIDAL_SURFACE('', #2, {major_radius}, {minor_radius});
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(p, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_toroidal = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str);
    let toroidal: step_geometry::ElementarySurface = (&step_toroidal)
        .try_into()
        .expect("elementary surface conversion");
    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        p.to_vec().extend(1.0),
    );
    (0..=10)
        .flat_map(move |i| (0..=10).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let u = 2.0 * PI * i as f64 / 10.0;
            let v = 2.0 * PI * j as f64 / 10.0;
            let res = toroidal.subs(u, v);
            let ans = mat.transform_point(Point3::new(
                (major_radius + minor_radius * f64::cos(v)) * f64::cos(u),
                (major_radius + minor_radius * f64::cos(v)) * f64::sin(u),
                minor_radius * f64::sin(v),
            ));
            assert_near!(res, ans, "u:{u} v:{v} res:{res:?} ans:{ans:?}");
        })
}

#[property_test]
fn toroidal_surface(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
    #[strategy = array::uniform2(1.0e-2f64..100.0f64)] radii: [f64; 2],
) {
    exec_toroidal_surface(org_coord, dir_array, ref_dir_array, radii)
}

fn exec_degenerate_toroidal_surface(select_outer: bool) {
    // A spindle torus (major < minor). The analytic surface is the same torus;
    // the source-defined sheet restricts the usable v domain.
    let major_radius = 0.5;
    let minor_radius = 1.0;
    let select = if select_outer { ".T." } else { ".F." };
    let step_str = format!(
        "DATA;
#1 = DEGENERATE_TOROIDAL_SURFACE('', #2, {major_radius}, {minor_radius}, {select});
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(Point3::origin(), 3),
        StepDataDisplay::new(VectorAsDirection(Vector3::unit_z()), 4),
        StepDataDisplay::new(VectorAsDirection(Vector3::unit_x()), 5),
    );
    let step_deg = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str);
    assert!(
        matches!(step_deg, ElementarySurfaceAny::DegenerateToroidalSurface(_)),
        "the record must dispatch to the degenerate arm, not `dummy`",
    );
    let deg: step_geometry::ElementarySurface = (&step_deg)
        .try_into()
        .expect("degenerate toroidal surface conversion");
    let step_geometry::ElementarySurface::DegenerateToroidalSurface(surface) = &deg else {
        panic!("expected DegenerateToroidalSurface");
    };
    let phi = f64::acos(-major_radius / minor_radius);
    let (v0, v1) = surface.entity().v_range();
    match select_outer {
        true => {
            assert!((v0 + phi).abs() < 1.0e-12);
            assert!((v1 - phi).abs() < 1.0e-12);
        }
        false => {
            assert!((v0 - phi).abs() < 1.0e-12);
            assert!((v1 - (2.0 * PI - phi)).abs() < 1.0e-12);
        }
    }
    (0..=10)
        .flat_map(move |i| (0..=10).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let u = 2.0 * PI * i as f64 / 10.0;
            let v = v0 + (v1 - v0) * j as f64 / 10.0;
            let res = surface.subs(u, v);
            let ans = Point3::new(
                (major_radius + minor_radius * f64::cos(v)) * f64::cos(u),
                (major_radius + minor_radius * f64::cos(v)) * f64::sin(u),
                minor_radius * f64::sin(v),
            );
            assert_near!(res, ans, "u:{u} v:{v} res:{res:?} ans:{ans:?}");
            let (u2, v2) = surface
                .search_parameter(res, SPHint2D::None, 100)
                .expect("on-sheet inverse");
            assert_near!(
                surface.subs(u2, v2),
                res,
                "search round trip failed on sheet {select_outer}",
            );
        });
}

#[test]
fn degenerate_toroidal_surface_outer_sheet() {
    exec_degenerate_toroidal_surface(true)
}

#[test]
fn degenerate_toroidal_surface_inner_sheet() {
    exec_degenerate_toroidal_surface(false)
}

/// A record violating the EXPRESS WHERE clause `major_radius < minor_radius`
/// must refuse conversion rather than degrade to an ordinary torus.
#[test]
fn degenerate_toroidal_surface_refuses_invalid_radii() {
    let step_str = "DATA;
#1 = DEGENERATE_TOROIDAL_SURFACE('', #2, 2.0, 1.0, .T.);
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
#3 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));
#4 = DIRECTION('', (0.0, 0.0, 1.0));
#5 = DIRECTION('', (1.0, 0.0, 0.0));
ENDSEC;";
    let step_deg = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str);
    assert!(
        matches!(step_deg, ElementarySurfaceAny::DegenerateToroidalSurface(_)),
        "the record must parse into the degenerate arm",
    );
    assert!(
        step_geometry::ElementarySurface::try_from(&step_deg).is_err(),
        "R >= r must refuse conversion",
    );
}

fn exec_conical_surface(
    org_coord: [f64; 3],
    dir_array: [f64; 2],
    ref_dir_array: [f64; 2],
    radius: f64,
    semi_angle: f64,
) {
    let p = Point3::from(org_coord);
    let z = dir_from_array(dir_array);
    let ref_dir = dir_from_array(ref_dir_array);
    let v = z.cross(ref_dir);
    let y = match v.so_small() {
        true => return,
        false => v.normalize(),
    };
    let x = y.cross(z).normalize();
    let step_str = format!(
        "DATA;
#1 = CONICAL_SURFACE('', #2, {radius}, {semi_angle});
#2 = AXIS2_PLACEMENT_3D('', #3, #4, #5);
{}{}{}ENDSEC;",
        StepDataDisplay::new(p, 3),
        StepDataDisplay::new(VectorAsDirection(z), 4),
        StepDataDisplay::new(VectorAsDirection(ref_dir.normalize()), 5),
    );
    let step_conical = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str);
    let conical: step_geometry::ElementarySurface = (&step_conical)
        .try_into()
        .expect("elementary surface conversion");

    // It has its own output, so test it accordingly.
    let step_str1 = format!("DATA;\n{}ENDSEC;", StepDataDisplay::new(conical, 1));
    let step_cylinder1 = step_to_entity::<ElementarySurfaceAnyHolder>(&step_str1);
    let conical1: step_geometry::ElementarySurface = (&step_cylinder1)
        .try_into()
        .expect("elementary surface conversion");

    let mat = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        p.to_vec().extend(1.0),
    );
    (0..=10)
        .flat_map(move |i| (0..=10).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let u = 2.0 * PI * i as f64 / 10.0;
            let v = j as f64 / 10.0;
            let tan = f64::tan(semi_angle);
            let res = conical.subs(u, v);
            let res1 = conical1.subs(u, v);
            let ans = mat.transform_point(Point3::new(
                (radius + v * tan) * f64::cos(u),
                (radius + v * tan) * f64::sin(u),
                v,
            ));
            assert_near!(res, ans, "u:{u} v:{v} res:{res:?} ans:{ans:?}");
            assert_near!(res1, ans, "u:{u} v:{v} res:{res1:?} ans:{ans:?}");
        })
}

#[property_test]
fn conical_surface(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0f64)] dir_array: [f64; 2],
    #[strategy = array::uniform2(0.0f64..1.0f64)] ref_dir_array: [f64; 2],
    #[strategy = 0.01f64..100.0f64] radius: f64,
    #[strategy = 0.0f64..PI / 2.0] semi_angle: f64,
) {
    exec_conical_surface(org_coord, dir_array, ref_dir_array, radius, semi_angle)
}

fn coords_to_points(
    upoints_len: usize,
    vpoints_len: usize,
    coords: Vec<Vec<[f64; 3]>>,
) -> Vec<Vec<Point3>> {
    coords
        .into_iter()
        .take(upoints_len)
        .map(move |vec: Vec<[f64; 3]>| {
            vec.into_iter()
                .take(vpoints_len)
                .map(Point3::from)
                .collect()
        })
        .collect()
}

fn compare_bsp_surfaces(res: &BSplineSurface<Point3>, ans: &BSplineSurface<Point3>) {
    assert_eq!(res.uknot_vec().len(), ans.uknot_vec().len());
    assert_eq!(res.vknot_vec().len(), ans.vknot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.uknot_vec()
        .iter()
        .zip(ans.uknot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.vknot_vec()
        .iter()
        .zip(ans.vknot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .flatten()
        .zip(ans.control_points().iter().flatten())
        .for_each(|(x, y)| assert_near!(x, y));
}

fn compare_nurbs_surfaces(res: &NurbsSurface<Vector4>, ans: &NurbsSurface<Vector4>) {
    assert_eq!(res.uknot_vec().len(), ans.uknot_vec().len());
    assert_eq!(res.vknot_vec().len(), ans.vknot_vec().len());
    assert_eq!(res.control_points().len(), ans.control_points().len());
    res.uknot_vec()
        .iter()
        .zip(ans.uknot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.vknot_vec()
        .iter()
        .zip(ans.vknot_vec())
        .for_each(|(x, y)| assert_near!(x, y));
    res.control_points()
        .iter()
        .flatten()
        .zip(ans.control_points().iter().flatten())
        .for_each(|(x, y)| assert_near!(x, y));
}

fn exec_b_spline_surface_with_knots(
    uknot_len: usize,
    uknot_mults: Vec<usize>,
    uknot_incrs: Vec<f64>,
    udegree: usize,
    vknot_len: usize,
    vknot_mults: Vec<usize>,
    vknot_incrs: Vec<f64>,
    vdegree: usize,
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) -> std::result::Result<(), TestCaseError> {
    let mut s = 0.0;
    let uvec = uknot_mults
        .iter()
        .take(uknot_len)
        .zip(uknot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let uknots = KnotVec::from(uvec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_surface_with_knots_degenerate_active_domain_refuses
    prop_assume!(uknots[udegree] < uknots[uknots.len() - udegree - 1]);
    let mut s = 0.0;
    let vvec = vknot_mults
        .iter()
        .take(vknot_len)
        .zip(vknot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let vknots = KnotVec::from(vvec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_surface_with_knots_degenerate_active_domain_refuses
    prop_assume!(vknots[vdegree] < vknots[vknots.len() - vdegree - 1]);
    let cps = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    let bsp = BSplineSurface::new((uknots, vknots), cps);
    let step_str = format!("DATA;{}ENDSEC;", StepDataDisplay::new(&bsp, 1));
    let bsp_step = step_to_entity::<BSplineSurfaceWithKnotsHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step).try_into().unwrap();
    compare_bsp_surfaces(&res, &bsp);
    Ok(())
}

#[property_test]
fn b_spline_surface_with_knots(
    #[strategy = 7usize..10] uknot_len: usize,
    #[strategy = collection::vec(1usize..4usize, 10)] uknot_mults: Vec<usize>,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 10)] uknot_incrs: Vec<f64>,
    #[strategy = 2usize..6] udegree: usize,
    #[strategy = 7usize..10] vknot_len: usize,
    #[strategy = collection::vec(1usize..4usize, 10)] vknot_mults: Vec<usize>,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 10)] vknot_incrs: Vec<f64>,
    #[strategy = 2usize..6] vdegree: usize,
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 40), 40)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) -> std::result::Result<(), TestCaseError> {
    exec_b_spline_surface_with_knots(
        uknot_len,
        uknot_mults,
        uknot_incrs,
        udegree,
        vknot_len,
        vknot_mults,
        vknot_incrs,
        vdegree,
        ctrlpt_coords,
    )
}

fn step_bsp_surface_ctrls(points: &[Vec<Point3>]) -> (String, String) {
    let indices = (0..points.len())
        .map(|i| {
            IndexSliceDisplay(
                (0..points[0].len())
                    .map(|j| 2 + i * points[0].len() + j)
                    .collect::<Vec<usize>>(),
            )
        })
        .collect::<Vec<_>>();
    let step_cps_indices = SliceDisplay(&indices).to_string();
    let step_cps = points
        .iter()
        .flatten()
        .enumerate()
        .fold(String::new(), |string, (i, p)| {
            let display: StepDataDisplay<Point3> = StepDataDisplay::new(*p, 2 + i);
            string + &display.to_string()
        });
    (step_cps_indices, step_cps)
}

/// A 4x4 grid of distinct control points with small coordinates.
fn unit_grid_surface() -> Vec<Vec<Point3>> {
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| Point3::new(i as f64 * 0.25, j as f64 * 0.5, (i * j) as f64 * 0.125))
                .collect()
        })
        .collect()
}

/// Asserts that `res` evaluates, on a clamped `[0,1]`-grid, exactly as the
/// tensor-product cubic Bernstein surface the control grid `points` defines.
/// Both knot axes are expected to be clamped-cubic `[0,0,0,0,1,1,1,1]` after
/// import, so `subs` is the tensor product of cubic Bernstein basis functions.
fn assert_tensor_cubic_bernstein(res: &BSplineSurface<Point3>, points: &[Vec<Point3>]) {
    let bernstein = |u: f64| -> [f64; 4] {
        let t = 1.0 - u;
        [t * t * t, 3.0 * t * t * u, 3.0 * t * u * u, u * u * u]
    };
    for i in 0..=20 {
        let s = i as f64 / 20.0;
        let bs = bernstein(s);
        for j in 0..=20 {
            let t = j as f64 / 20.0;
            let bt = bernstein(t);
            let mut ans = Vector3::new(0.0, 0.0, 0.0);
            for (ip, bi) in bs.iter().enumerate() {
                for (jp, bj) in bt.iter().enumerate() {
                    ans += points[ip][jp].to_vec() * *bi * *bj;
                }
            }
            let got = res.subs(s, t).to_vec();
            let slack = 1.0e-9; // H-3: float slack between two evaluations of one tensor-Bernstein point, not a length
            assert!(
                (got - ans).magnitude() < slack,
                "s:{s} t:{t} got:{got:?} ans:{ans:?}"
            );
        }
    }
}

/// A u-axis knot vector whose active domain collapses to a single value must
/// refuse instead of silently substituting synthesized knots.
#[test]
fn b_spline_surface_with_knots_degenerate_active_domain_refuses() {
    let u_degree = 3;
    let v_degree = 3;
    let points = unit_grid_surface();
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    // u: `u_multiplicities (5,3)` over `u_knots (0.0, 1.0)` expands to eight
    // knots, but the active domain is `[T_3, T_4] = [0.0, 0.0]`; v is valid.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_SURFACE_WITH_KNOTS('', {u_degree}, {v_degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., .F., (5,3), (4,4), (0.0, 1.0), (0.0, 1.0), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineSurfaceWithKnotsHolder>(&step_str);
    assert!(
        BSplineSurface::<Point3>::try_from(&bsp_step).is_err(),
        "a degenerate u-axis active domain must refuse",
    );
}

/// A u-axis knot vector whose expanded length is short of `N + degree + 1`
/// must refuse instead of silently replacing the source knots.
#[test]
fn b_spline_surface_with_knots_knot_count_mismatch_refuses() {
    let u_degree = 3;
    let v_degree = 3;
    let points = unit_grid_surface();
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    // u: `u_multiplicities (2,2)` over `u_knots (0.0, 1.0)` expands to four
    // knots, but `N = 4`, `u_degree = 3` need eight; v is valid.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_SURFACE_WITH_KNOTS('', {u_degree}, {v_degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., .F., (2,2), (4,4), (0.0, 1.0), (0.0, 1.0), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineSurfaceWithKnotsHolder>(&step_str);
    assert!(
        BSplineSurface::<Point3>::try_from(&bsp_step).is_err(),
        "a u-axis knot-count mismatch must refuse",
    );
}

/// `QUASI_UNIFORM_SURFACE` has no source knot list, but a degree that exceeds
/// the control-point count cannot be synthesized either; it must refuse.
#[test]
fn quasi_uniform_surface_degree_exceeds_control_points_refuses() {
    let u_degree = 2;
    let v_degree = 5;
    let points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| Point3::new(i as f64, j as f64, (i * j) as f64))
                .collect()
        })
        .collect::<Vec<Vec<Point3>>>();
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = QUASI_UNIFORM_SURFACE('', {u_degree}, {v_degree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<QuasiUniformSurfaceHolder>(&step_str);
    assert!(
        BSplineSurface::<Point3>::try_from(&bsp_step).is_err(),
        "a v-degree exceeding the control-point count must refuse",
    );
}

/// A `b_spline_surface_with_knots` whose u-axis knot interval is nonzero but
/// smaller than truck's `TOLERANCE` (absolute) must still convert. Normalizing
/// the u knot vector to `[0, 1]` is an exact, shape-preserving
/// reparameterization of the same surface; the v axis is untouched.
#[test]
fn b_spline_surface_with_knots_tiny_u_interval_converts() {
    let u_degree = 3;
    let v_degree = 3;
    let points = unit_grid_surface();
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    // A u-axis knot span of 5e-7: below TOLERANCE, above a true zero range.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_SURFACE_WITH_KNOTS('', {u_degree}, {v_degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., .F., (4,4), (4,4), (0.0, 5.0E-7), (0.0, 1.0), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineSurfaceWithKnotsHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step)
        .try_into()
        .expect("a tiny u-axis range surface converts");
    assert_eq!(res.uknot_vec().len(), 8, "u knot count preserved");
    res.uknot_vec()
        .iter()
        .zip([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0])
        .for_each(|(x, y)| assert_near!(*x, y, "u normalized to [0, 1]"));
    assert_eq!(res.vknot_vec().len(), 8, "v knot count preserved");
    res.vknot_vec()
        .iter()
        .zip([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0])
        .for_each(|(x, y)| assert_near!(*x, y, "v knot vector untouched"));
    assert_eq!(res.control_points().len(), points.len());
    res.control_points()
        .iter()
        .flatten()
        .zip(points.iter().flatten())
        .for_each(|(x, y)| assert_near!(x, y, "control points preserved"));
    assert_tensor_cubic_bernstein(&res, &points);
}

/// The mirror image of the tiny-u test: a tiny knot span on the v axis, an
/// ordinary `[0, 1]` span on u; v normalizes and u is untouched.
#[test]
fn b_spline_surface_with_knots_tiny_v_interval_converts() {
    let u_degree = 3;
    let v_degree = 3;
    let points = unit_grid_surface();
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    // A v-axis knot span of 5e-7: below TOLERANCE, above a true zero range.
    let step_str = format!(
        "DATA;
#1 = B_SPLINE_SURFACE_WITH_KNOTS('', {u_degree}, {v_degree}, {step_cps_indices}, .UNSPECIFIED., .F., .F., .F., (4,4), (4,4), (0.0, 1.0), (0.0, 5.0E-7), .UNSPECIFIED.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BSplineSurfaceWithKnotsHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step)
        .try_into()
        .expect("a tiny v-axis range surface converts");
    assert_eq!(res.uknot_vec().len(), 8, "u knot count preserved");
    res.uknot_vec()
        .iter()
        .zip([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0])
        .for_each(|(x, y)| assert_near!(*x, y, "u knot vector untouched"));
    assert_eq!(res.vknot_vec().len(), 8, "v knot count preserved");
    res.vknot_vec()
        .iter()
        .zip([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0])
        .for_each(|(x, y)| assert_near!(*x, y, "v normalized to [0, 1]"));
    assert_eq!(res.control_points().len(), points.len());
    res.control_points()
        .iter()
        .flatten()
        .zip(points.iter().flatten())
        .for_each(|(x, y)| assert_near!(x, y, "control points preserved"));
    assert_tensor_cubic_bernstein(&res, &points);
}

fn exec_bezier_surface([udegree, vdegree]: [usize; 2], ctrlpt_coords: Vec<Vec<[f64; 3]>>) {
    let points = coords_to_points(udegree + 1, vdegree + 1, ctrlpt_coords);
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = BEZIER_SURFACE('', {udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<BezierSurfaceHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step).into();
    let ans = BSplineSurface::new(
        (KnotVec::bezier_knot(udegree), KnotVec::bezier_knot(vdegree)),
        points,
    );
    compare_bsp_surfaces(&res, &ans);
}

#[property_test]
fn bezier_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 6), 6)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) {
    exec_bezier_surface(degrees, ctrlpt_coords)
}

fn exec_quasi_uniform_surface(
    [udegree, vdegree]: [usize; 2],
    [udivision, vdivision]: [usize; 2],
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) {
    let mut uknots = KnotVec::uniform_knot(udegree, udivision);
    uknots.transform(udivision as f64, 0.0);
    let mut vknots = KnotVec::uniform_knot(vdegree, vdivision);
    vknots.transform(vdivision as f64, 0.0);
    let points = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = QUASI_UNIFORM_SURFACE('', {udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<QuasiUniformSurfaceHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step).try_into().unwrap();
    let ans = BSplineSurface::new((uknots, vknots), points);
    compare_bsp_surfaces(&res, &ans);
}

#[property_test]
fn quasi_uniform_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = array::uniform2(2usize..5)] divisions: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 30), 30)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) {
    exec_quasi_uniform_surface(degrees, divisions, ctrlpt_coords)
}

fn exec_uniform_surface(
    [udegree, vdegree]: [usize; 2],
    [uknot_len, vknot_len]: [usize; 2],
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) {
    let uknots = KnotVec::from_iter((0..uknot_len).map(|i| i as f64 - udegree as f64));
    let vknots = KnotVec::from_iter((0..vknot_len).map(|i| i as f64 - vdegree as f64));
    let points = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = UNIFORM_SURFACE('', {udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<UniformSurfaceHolder>(&step_str);
    let res: BSplineSurface<Point3> = (&bsp_step).try_into().unwrap();
    let ans = BSplineSurface::new((uknots, vknots), points);
    compare_bsp_surfaces(&res, &ans);
}

#[property_test]
fn uniform_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = array::uniform2(7usize..30)] knot_lens: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 30), 30)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
) {
    exec_uniform_surface(degrees, knot_lens, ctrlpt_coords)
}

fn exec_nurbs_surface_b_spline_surface_with_knots(
    uknot_len: usize,
    uknot_mults: Vec<usize>,
    uknot_incrs: Vec<f64>,
    udegree: usize,
    vknot_len: usize,
    vknot_mults: Vec<usize>,
    vknot_incrs: Vec<f64>,
    vdegree: usize,
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    mut weights: Vec<Vec<f64>>,
) -> std::result::Result<(), TestCaseError> {
    let mut s = 0.0;
    let uvec = uknot_mults
        .iter()
        .take(uknot_len)
        .zip(uknot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let uknots = KnotVec::from(uvec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_surface_with_knots_degenerate_active_domain_refuses
    prop_assume!(uknots[udegree] < uknots[uknots.len() - udegree - 1]);
    let mut s = 0.0;
    let vvec = vknot_mults
        .iter()
        .take(vknot_len)
        .zip(vknot_incrs)
        .flat_map(|(&m, x)| {
            s += x;
            std::iter::repeat_n(s, m)
        })
        .collect::<Vec<f64>>();
    let vknots = KnotVec::from(vvec);
    // non-degenerate active domain; the refusal counterpart is
    // b_spline_surface_with_knots_degenerate_active_domain_refuses
    prop_assume!(vknots[vdegree] < vknots[vknots.len() - vdegree - 1]);
    let cps = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    weights.truncate(cps.len());
    weights
        .iter_mut()
        .zip(&cps)
        .for_each(|(vec, vec0)| vec.truncate(vec0.len()));
    let bsp = BSplineSurface::new((uknots, vknots), cps);
    let ans = NurbsSurface::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    let step_str = format!("DATA;{}ENDSEC;", StepDataDisplay::new(&ans, 1));
    let bsp_step = step_to_entity::<RationalBSplineSurfaceHolder>(&step_str);
    let res: NurbsSurface<Vector4> = (&bsp_step).try_into().unwrap();
    compare_nurbs_surfaces(&res, &ans);
    Ok(())
}

#[property_test]
fn nurbs_surface_b_spline_surface_with_knots(
    #[strategy = 7usize..10] uknot_len: usize,
    #[strategy = collection::vec(1usize..4usize, 10)] uknot_mults: Vec<usize>,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 10)] uknot_incrs: Vec<f64>,
    #[strategy = 2usize..6] udegree: usize,
    #[strategy = 7usize..10] vknot_len: usize,
    #[strategy = collection::vec(1usize..4usize, 10)] vknot_mults: Vec<usize>,
    #[strategy = collection::vec(1.0e-3f64..100.0f64, 10)] vknot_incrs: Vec<f64>,
    #[strategy = 2usize..6] vdegree: usize,
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 40), 40)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    #[strategy = collection::vec(collection::vec(0.01f64..100.0f64, 40), 40)] weights: Vec<
        Vec<f64>,
    >,
) -> std::result::Result<(), TestCaseError> {
    exec_nurbs_surface_b_spline_surface_with_knots(
        uknot_len,
        uknot_mults,
        uknot_incrs,
        udegree,
        vknot_len,
        vknot_mults,
        vknot_incrs,
        vdegree,
        ctrlpt_coords,
        weights,
    )
}

fn exec_nurbs_surface_bezier_surface(
    [udegree, vdegree]: [usize; 2],
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    mut weights: Vec<Vec<f64>>,
) {
    let points = coords_to_points(udegree + 1, vdegree + 1, ctrlpt_coords);
    weights.truncate(points.len());
    weights
        .iter_mut()
        .zip(&points)
        .for_each(|(vec, vec0)| vec.truncate(vec0.len()));
    let weights_displays = weights
        .iter()
        .map(|vec| SliceDisplay(vec))
        .collect::<Vec<_>>();
    let weights_display = SliceDisplay(&weights_displays);
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BEZIER_SURFACE()
    BOUNDED_SURFACE()
    B_SPLINE_SURFACE({udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.)
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_SURFACE({weights_display})
    REPRESENTATION_ITEM('')
    SURFACE()
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineSurfaceHolder>(&step_str);
    let res: NurbsSurface<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineSurface::new(
        (KnotVec::bezier_knot(udegree), KnotVec::bezier_knot(vdegree)),
        points,
    );
    let ans = NurbsSurface::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    compare_nurbs_surfaces(&res, &ans);
}

#[property_test]
fn nurbs_surface_bezier_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 6), 6)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    #[strategy = collection::vec(collection::vec(0.01f64..100.0f64, 6), 6)] weights: Vec<Vec<f64>>,
) {
    exec_nurbs_surface_bezier_surface(degrees, ctrlpt_coords, weights)
}

fn exec_nurbs_surface_quasi_uniform_surface(
    [udegree, vdegree]: [usize; 2],
    [udivision, vdivision]: [usize; 2],
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    mut weights: Vec<Vec<f64>>,
) {
    let mut uknots = KnotVec::uniform_knot(udegree, udivision);
    uknots.transform(udivision as f64, 0.0);
    let mut vknots = KnotVec::uniform_knot(vdegree, vdivision);
    vknots.transform(vdivision as f64, 0.0);
    let points = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    weights.truncate(points.len());
    weights
        .iter_mut()
        .zip(&points)
        .for_each(|(vec, vec0)| vec.truncate(vec0.len()));
    let weights_displays = weights
        .iter()
        .map(|vec| SliceDisplay(vec))
        .collect::<Vec<_>>();
    let weights_display = SliceDisplay(&weights_displays);
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BOUNDED_SURFACE()
    B_SPLINE_SURFACE({udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.)
    GEOMETRIC_REPRESENTATION_ITEM()
    QUASI_UNIFORM_SURFACE()
    RATIONAL_B_SPLINE_SURFACE({weights_display})
    REPRESENTATION_ITEM('')
    SURFACE()
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineSurfaceHolder>(&step_str);
    let res: NurbsSurface<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineSurface::new((uknots, vknots), points);
    let ans = NurbsSurface::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    compare_nurbs_surfaces(&res, &ans);
}

#[property_test]
fn nurbs_surface_quasi_uniform_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = array::uniform2(2usize..5)] divisions: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 30), 30)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    #[strategy = collection::vec(collection::vec(0.01f64..100.0f64, 30), 30)] weights: Vec<
        Vec<f64>,
    >,
) {
    exec_nurbs_surface_quasi_uniform_surface(degrees, divisions, ctrlpt_coords, weights)
}

fn exec_nurbs_surface_uniform_surface(
    [udegree, vdegree]: [usize; 2],
    [uknot_len, vknot_len]: [usize; 2],
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    mut weights: Vec<Vec<f64>>,
) {
    let uknots = KnotVec::from_iter((0..uknot_len).map(|i| i as f64 - udegree as f64));
    let vknots = KnotVec::from_iter((0..vknot_len).map(|i| i as f64 - vdegree as f64));
    let points = coords_to_points(
        uknots.len() - udegree - 1,
        vknots.len() - vdegree - 1,
        ctrlpt_coords,
    );
    println!("{} {}", uknots.len(), points.len());
    weights.truncate(points.len());
    weights
        .iter_mut()
        .zip(&points)
        .for_each(|(vec, vec0)| vec.truncate(vec0.len()));
    let weights_displays = weights
        .iter()
        .map(|vec| SliceDisplay(vec))
        .collect::<Vec<_>>();
    let weights_display = SliceDisplay(&weights_displays);
    let (step_cps_indices, step_cps) = step_bsp_surface_ctrls(&points);
    let step_str = format!(
        "DATA;
#1 = (
    BOUNDED_SURFACE()
    B_SPLINE_SURFACE({udegree}, {vdegree}, {step_cps_indices}, .UNSPECIFIED., .U., .U., .U.)
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_SURFACE({weights_display})
    REPRESENTATION_ITEM('')
    SURFACE()
    UNIFORM_SURFACE()
);
{step_cps}ENDSEC;"
    );
    let bsp_step = step_to_entity::<RationalBSplineSurfaceHolder>(&step_str);
    let res: NurbsSurface<Vector4> = (&bsp_step).try_into().unwrap();
    let bsp = BSplineSurface::new((uknots, vknots), points);
    let ans = NurbsSurface::<Vector4>::try_from_bspline_and_weights(bsp, weights).unwrap();
    compare_nurbs_surfaces(&res, &ans);
}

#[property_test]
fn nurbs_surface_uniform_surface(
    #[strategy = array::uniform2(1usize..6)] degrees: [usize; 2],
    #[strategy = array::uniform2(7usize..30)] knot_lens: [usize; 2],
    #[strategy = collection::vec(collection::vec(array::uniform3(-100.0f64..100.0f64), 30), 30)]
    ctrlpt_coords: Vec<Vec<[f64; 3]>>,
    #[strategy = collection::vec(collection::vec(0.01f64..100.0f64, 30), 30)] weights: Vec<
        Vec<f64>,
    >,
) {
    exec_nurbs_surface_uniform_surface(degrees, knot_lens, ctrlpt_coords, weights)
}

fn exec_surface_of_linear_extrusion(
    point0_coord: [f64; 3],
    point1_coord: [f64; 3],
    axis_elem: [f64; 3],
) {
    let line = Line(Point3::from(point0_coord), Point3::from(point1_coord));
    if line.0.near(&line.1) {
        return;
    }
    let axis = Vector3::from(axis_elem);
    let step_str = format!(
        "DATA;#1 = SURFACE_OF_LINEAR_EXTRUSION('', #4, #2);{}{}ENDSEC;",
        StepDataDisplay::new(axis, 2),
        StepDataDisplay::new(&line, 4),
    );
    let step_surface = step_to_entity::<SurfaceOfLinearExtrusionHolder>(&step_str);
    let surface: StepExtrudedCurve = (&step_surface).try_into().unwrap();
    (0..=100)
        .flat_map(move |i| (0..=100).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let (u, v) = (i as f64 / 10.0, j as f64 / 10.0);
            assert_near!(surface.subs(u, v), line.subs(u) + axis * v);
        });
}

#[property_test]
fn surface_of_linear_extrusion(
    #[strategy = array::uniform3(-100.0f64..100.0f64)] point0_coord: [f64; 3],
    #[strategy = array::uniform3(-100.0f64..100.0f64)] point1_coord: [f64; 3],
    #[strategy = array::uniform3(-100.0f64..100.0f64)] axis_elem: [f64; 3],
) {
    exec_surface_of_linear_extrusion(point0_coord, point1_coord, axis_elem)
}

fn exec_surface_of_revolution(
    point0_coord: [f64; 3],
    point1_coord: [f64; 3],
    org_coord: [f64; 3],
    axis_array: [f64; 2],
) {
    let line = Line(Point3::from(point0_coord), Point3::from(point1_coord));
    if line.0.near(&line.1) {
        return;
    }
    let origin = Point3::from(org_coord);
    let dir = dir_from_array(axis_array);
    let step_str = format!(
        "DATA;
#1 = SURFACE_OF_REVOLUTION('', #5, #2);
#2 = AXIS1_PLACEMENT('', #3, #4);
{}{}{}ENDSEC;",
        StepDataDisplay::new(origin, 3),
        StepDataDisplay::new(VectorAsDirection(dir), 4),
        StepDataDisplay::new(&line, 5),
    );
    let step_surface = step_to_entity::<SurfaceOfRevolutionHolder>(&step_str);
    let surface: StepRevolutedCurve = (&step_surface).try_into().unwrap();
    (0..=100)
        .flat_map(move |i| (0..=100).map(move |j| (i, j)))
        .for_each(|(i, j)| {
            let (u, v) = (i as f64 / 10.0, j as f64 / 10.0);
            let lc = line.subs(v) - origin;
            let ans = origin
                + lc * f64::cos(u)
                + dir * lc.dot(dir) * (1.0 - f64::cos(u))
                + dir.cross(lc) * f64::sin(u);
            assert_near!(surface.subs(u, v), ans);
        });
}

#[property_test]
fn surface_of_revolution(
    #[strategy = array::uniform3(-100f64..100.0)] point0_coord: [f64; 3],
    #[strategy = array::uniform3(-100f64..100.0)] point1_coord: [f64; 3],
    #[strategy = array::uniform3(-100f64..100.0)] org_coord: [f64; 3],
    #[strategy = array::uniform2(0.0f64..1.0)] axis_array: [f64; 2],
) {
    exec_surface_of_revolution(point0_coord, point1_coord, org_coord, axis_array)
}
