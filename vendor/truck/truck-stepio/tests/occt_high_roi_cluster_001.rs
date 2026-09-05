// OCCT-HIGH-ROI-CLUSTER-001: the cheap high-yield OCCT ingestion cluster.
//
// These tests pin, in truck-stepio:
//   C1  a `SURFACE_CURVE` honors its mandatory declared 3D curve over any
//       `PCURVE_S1`/`PCURVE_S2` master preference;
//   C2  the gated pcurve fallback rescues a primary that parses but does not
//       reconcile, and refuses (with residual evidence) when nothing does;
//   C3  `TRIMMED_CURVE` edge geometry is realized (parameter trims, point
//       trims, the trim-duality certificate, and the line parameter
//       re-anchor);
//   C4  `BREP_WITH_VOIDS` bookkeeping: exterior shell first, then each void
//       shell, face count the sum of both shells' faces.
//
// STEP inputs are built as in-test strings following the landed
// `tests/input/` recipe module: a `DATA;...;ENDSEC;` data section parsed into
// a `Table`, then the `EDGE_CURVE` resolved to its owned entity.

use ruststep::ast::DataSection;
use ruststep::tables::EntityTable;
use std::str::FromStr;
use truck_stepio::r#in::{step_geometry::*, *};

/// Absolute comparison budget. Model units are on the order of a few units and
/// every fixture is exact in f64, so 1.0e-6 (the legacy truck epsilon this
/// crate's own conversions use) is comfortably loose for the geometry here.
const EPS: f64 = 1.0e-6; // H-3

fn data_section(data: &str) -> Table {
    Table::from_data_section(&DataSection::from_str(data).unwrap()) // H-1: trusted fixture string
}

fn roundtrip(x: f64) -> String {
    format!("{x:?}")
}

fn owned_edge(table: &Table, id: u64) -> EdgeCurve {
    EntityTable::<EdgeCurveHolder>::get_owned(table, id).unwrap() // H-1: fixture must resolve
}

fn near(a: Point3, b: Point3) {
    let d = (a - b).magnitude();
    assert!(d <= EPS, "expected {a:?}, got {b:?}, distance {d}");
}

fn near_scalar(a: f64, b: f64) {
    let d = (a - b).abs();
    assert!(d <= EPS, "expected {a}, got {b}, difference {d}");
}

/// The shared unit geometry: a circle of `radius` in the z=0 plane (axis along
/// z at the origin) and a cylinder of the same axis/radius whose `(u, v)`
/// chart is (angle around the axis, height). Returns the record lines.
///
/// id map:
///  #10 origin, #11 axis, #12 ref direction, #13 placement,
///  #14 circle, #15 cylinder.
fn unit_geometry(radius: f64) -> Vec<String> {
    vec![
        "#10 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));".to_string(),
        "#11 = DIRECTION('', (0.0, 0.0, 1.0));".to_string(),
        "#12 = DIRECTION('', (1.0, 0.0, 0.0));".to_string(),
        "#13 = AXIS2_PLACEMENT_3D('', #10, #11, #12);".to_string(),
        format!("#14 = CIRCLE('', #13, {});", roundtrip(radius)),
        format!("#15 = CYLINDRICAL_SURFACE('', #13, {});", roundtrip(radius)),
    ]
}

/// Records for the two associated pcurves (both over the same cylinder, both
/// drawn as a horizontal 2D line at height `v` in the cylinder's `(angle,
/// height)` chart).
fn pcurve_pair_records(v: f64) -> Vec<String> {
    // The `#24 NONSENSE` record lands in `dummy`, which is where the
    // `DEFINITIONAL_REPRESENTATION` context reference resolves.
    vec![
        format!("#20 = CARTESIAN_POINT('', (0.0, {}));", roundtrip(v)),
        "#21 = DIRECTION('', (1.0, 0.0));".to_string(),
        "#22 = VECTOR('', #21, 1.0);".to_string(),
        "#23 = LINE('', #20, #22);".to_string(),
        "#24 = NONSENSE('');".to_string(),
        "#25 = DEFINITIONAL_REPRESENTATION('', (#23), #24);".to_string(),
        "#26 = PCURVE('', #15, #25);".to_string(),
        "#27 = PCURVE('', #15, #25);".to_string(),
    ]
}

/// A complete data section for a `SURFACE_CURVE` edge test.
///
/// `curve_3d` is `#14` (the circle at z=0, radius `radius`); the two
/// associated pcurves live over `#15` at height `v`; the edge runs between the
/// points at angles `theta0` and `theta1` on the circle.
#[allow(clippy::too_many_arguments)]
fn surface_curve_edge(
    radius: f64,
    theta0: f64,
    theta1: f64,
    v: f64,
    master: &str,
    broken_curve_3d: bool,
) -> String {
    let mut records = unit_geometry(radius);
    records.append(&mut pcurve_pair_records(v));
    records.push(format!(
        "#40 = CARTESIAN_POINT('', ({}, {}, 0.0));",
        roundtrip(radius * theta0.cos()),
        roundtrip(radius * theta0.sin())
    ));
    records.push(format!(
        "#41 = CARTESIAN_POINT('', ({}, {}, 0.0));",
        roundtrip(radius * theta1.cos()),
        roundtrip(radius * theta1.sin())
    ));
    records.push("#42 = VERTEX_POINT('', #40);".to_string());
    records.push("#43 = VERTEX_POINT('', #41);".to_string());
    if broken_curve_3d {
        // A zero radius circle has a singular placement matrix: converting the
        // declared 3D curve refuses, and the parse error must surface rather
        // than a pcurve substitution.
        records.push("#50 = CIRCLE('', #13, 0.0);".to_string());
        records.push(format!(
            "#51 = SURFACE_CURVE('', #50, (#26, #27), {master});"
        ));
    } else {
        records.push(format!(
            "#51 = SURFACE_CURVE('', #14, (#26, #27), {master});"
        ));
    }
    records.push("#52 = EDGE_CURVE('', #42, #43, #51, .T.);".to_string());
    format!("DATA;\n{}\nENDSEC;", records.join("\n"))
}

/// `sem_pcurve_master_001` correction: a `SURFACE_CURVE` whose declared 3D
/// curve reconciles must be realized from that 3D curve regardless of its
/// `PCURVE_S1` master. The pcurves here are drawn at a different height and
/// would not reconcile, so honoring the master would be visible.
#[test]
fn sem_pcurve_master_001_pcurve_s1_uses_declared_3d_curve() {
    let theta0 = 0.9;
    let theta1 = 2.1;
    let edge = owned_edge(
        &data_section(&surface_curve_edge(
            1.0,
            theta0,
            theta1,
            1.7,
            ".PCURVE_S1.",
            false,
        )),
        52,
    );
    let curve = edge.parse_curve3d().unwrap(); // H-1: the declared 3D curve reconciles
    for t in [0.9f64, 1.2, 1.5, 1.8, 2.1] {
        let p = curve.subs(t);
        near(p, Point3::new(t.cos(), t.sin(), 0.0));
    }
    near(curve.front(), Point3::new(theta0.cos(), theta0.sin(), 0.0));
    near(curve.back(), Point3::new(theta1.cos(), theta1.sin(), 0.0));
}

/// The `PCURVE_S2` twin of the correction test above.
#[test]
fn sem_pcurve_master_001_pcurve_s2_uses_declared_3d_curve() {
    let theta0 = 0.9;
    let theta1 = 2.1;
    let edge = owned_edge(
        &data_section(&surface_curve_edge(
            1.0,
            theta0,
            theta1,
            1.7,
            ".PCURVE_S2.",
            false,
        )),
        52,
    );
    let curve = edge.parse_curve3d().unwrap(); // H-1: the declared 3D curve reconciles
    for t in [0.9f64, 1.2, 1.5, 1.8, 2.1] {
        let p = curve.subs(t);
        near(p, Point3::new(t.cos(), t.sin(), 0.0));
    }
    near(curve.front(), Point3::new(theta0.cos(), theta0.sin(), 0.0));
    near(curve.back(), Point3::new(theta1.cos(), theta1.sin(), 0.0));
}

/// A `SURFACE_CURVE` whose declared 3D curve cannot be converted returns that
/// parse error. The gated pcurve fallback must not fire here: it only rescues
/// a primary that *parses* but fails endpoint reconciliation.
#[test]
fn sem_pcurve_master_001_broken_curve_3d_refuses() {
    let edge = owned_edge(
        &data_section(&surface_curve_edge(1.0, 0.9, 2.1, 1.7, ".PCURVE_S1.", true)),
        52,
    );
    let err = edge.parse_curve3d().unwrap_err(); // H-1: the broken curve must refuse
    let message = format!("{err:?}");
    assert!(
        message.contains("Circle"),
        "expected the circle conversion error, got: {message}"
    );
}

/// A seam-crossing trim extent (u: 5.9 -> 6.4 on the 2π-periodic chart)
/// survives: the realized curve's parameter range spans the declared extent,
/// not the principal-branch fold that would run the short way around.
#[test]
fn sem_pcurve_master_001_seam_crossing_extent_reconciles() {
    let edge = owned_edge(
        &data_section(&surface_curve_edge(
            1.0,
            5.9,
            6.4,
            1.7,
            ".PCURVE_S1.",
            false,
        )),
        52,
    );
    let curve = edge.parse_curve3d().unwrap(); // H-1: the declared 3D curve reconciles
    let (lo, hi) = curve.range_tuple();
    near_scalar(lo, 5.9);
    near_scalar(hi, 6.4);
    near(curve.front(), Point3::new(5.9f64.cos(), 5.9f64.sin(), 0.0));
    near(curve.back(), Point3::new(6.4f64.cos(), 6.4f64.sin(), 0.0));
}

/// Records for a `TRIMMED_CURVE` over a circle of `radius` in the plane z=0,
/// trimmed between the angles `theta0` and `theta1`.
fn circle_trimmed_edge(theta0: f64, theta1: f64, radius: f64, trim_line: &str) -> String {
    let mut records = vec![
        "#10 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));".to_string(),
        "#11 = DIRECTION('', (0.0, 0.0, 1.0));".to_string(),
        "#12 = DIRECTION('', (1.0, 0.0, 0.0));".to_string(),
        "#13 = AXIS2_PLACEMENT_3D('', #10, #11, #12);".to_string(),
        format!("#14 = CIRCLE('', #13, {});", roundtrip(radius)),
    ];
    records.push(format!(
        "#40 = CARTESIAN_POINT('', ({}, {}, 0.0));",
        roundtrip(radius * theta0.cos()),
        roundtrip(radius * theta0.sin())
    ));
    records.push(format!(
        "#41 = CARTESIAN_POINT('', ({}, {}, 0.0));",
        roundtrip(radius * theta1.cos()),
        roundtrip(radius * theta1.sin())
    ));
    records.push("#42 = VERTEX_POINT('', #40);".to_string());
    records.push("#43 = VERTEX_POINT('', #41);".to_string());
    records.push(format!("#50 = TRIMMED_CURVE('', #14, {trim_line});"));
    records.push("#51 = EDGE_CURVE('', #42, #43, #50, .T.);".to_string());
    format!("DATA;\n{}\nENDSEC;", records.join("\n"))
}

/// A `TRIMMED_CURVE` over a `CIRCLE` basis with two `PARAMETER_VALUE` trims
/// realizes the arc between them: sampled points land on the circle at the
/// declared angles.
#[test]
fn occt_high_roi_cluster_001_trimmed_curve_parameter_trim() {
    let theta0 = 0.7;
    let theta1 = 1.9;
    let data = circle_trimmed_edge(
        theta0,
        theta1,
        2.0,
        "(PARAMETER_VALUE(0.7)), (PARAMETER_VALUE(1.9)), .T., .PARAMETER.",
    );
    let edge = owned_edge(&data_section(&data), 51);
    let curve = edge.parse_curve3d().unwrap(); // H-1: the declared trim reconciles
    for t in [0.7f64, 1.0, 1.3, 1.6, 1.9] {
        let p = curve.subs(t);
        near(p, Point3::new(2.0 * t.cos(), 2.0 * t.sin(), 0.0));
    }
    let (lo, hi) = curve.range_tuple();
    near_scalar(lo, theta0);
    near_scalar(hi, theta1);
}

/// The same arc trimmed by `CARTESIAN_POINT` selects under a `.CARTESIAN.`
/// master: the point trims are solved against the basis.
#[test]
fn occt_high_roi_cluster_001_trimmed_curve_point_trim() {
    let theta0 = 0.7;
    let theta1 = 1.9;
    let data = circle_trimmed_edge(theta0, theta1, 2.0, "(#40), (#41), .T., .CARTESIAN.");
    let edge = owned_edge(&data_section(&data), 51);
    let curve = edge.parse_curve3d().unwrap(); // H-1: the point trims reconcile
    for t in [0.7f64, 1.0, 1.3, 1.6, 1.9] {
        let p = curve.subs(t);
        near(p, Point3::new(2.0 * t.cos(), 2.0 * t.sin(), 0.0));
    }
    let (lo, hi) = curve.range_tuple();
    near_scalar(lo, theta0);
    near_scalar(hi, theta1);
}

/// A trim carrying both a point and a parameter whose readings disagree is a
/// typed refusal naming both readings, never a silent pick.
#[test]
fn occt_high_roi_cluster_001_trimmed_curve_dual_trim_disagreement_refuses() {
    // trim_1 carries the point at angle 0.7 *and* the declared parameter 1.9;
    // the solved parameter of the point (0.7) disagrees with the declaration.
    let data = circle_trimmed_edge(
        0.7,
        1.9,
        2.0,
        "(#40, PARAMETER_VALUE(1.9)), (PARAMETER_VALUE(2.6)), .T., .PARAMETER.",
    );
    let edge = owned_edge(&data_section(&data), 51);
    let err = edge.parse_curve3d().unwrap_err(); // H-1: the duality certificate refuses
    let message = format!("{err:?}");
    assert!(
        message.contains("disagree"),
        "expected the duality disagreement refusal, got: {message}"
    );
    assert!(
        message.contains("PARAMETER_VALUE"),
        "expected both readings named, got: {message}"
    );
}

/// A `TRIMMED_CURVE` over a `LINE` basis with parameter trims t0=2.5, t1=4.0
/// (in units of the direction vector) realizes the segment from `pnt + 2.5·dir`
/// to `pnt + 4.0·dir`, NOT the 0..=1 range misread.
#[test]
fn occt_high_roi_cluster_001_trimmed_curve_line_parameter_scaling() {
    let data = "DATA;
#1 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));
#2 = DIRECTION('', (1.0, 0.0, 0.0));
#3 = VECTOR('', #2, 1.0);
#4 = LINE('', #1, #3);
#5 = TRIMMED_CURVE('', #4, (2.5), (4.0), .T., .PARAMETER.);
#6 = CARTESIAN_POINT('', (2.5, 0.0, 0.0));
#7 = CARTESIAN_POINT('', (4.0, 0.0, 0.0));
#8 = VERTEX_POINT('', #6);
#9 = VERTEX_POINT('', #7);
#10 = EDGE_CURVE('', #8, #9, #5, .T.);
ENDSEC;";
    let edge = owned_edge(&data_section(data), 10);
    let curve = edge.parse_curve3d().unwrap(); // H-1: the re-anchored line reconciles
    assert!(
        matches!(curve, Curve3D::Line(_)),
        "expected a line realization, got {curve:?}"
    );
    near(curve.front(), Point3::new(2.5, 0.0, 0.0));
    near(curve.back(), Point3::new(4.0, 0.0, 0.0));
    near(curve.subs(0.5), Point3::new(3.25, 0.0, 0.0));
}

/// Records for a `SURFACE_CURVE` edge whose declared `curve_3d` is the z=0
/// circle of `radius`, which cannot host the vertex points (they sit at height
/// `vertex_z` on the cylinder), with one pcurve drawn at height `pcurve_v` on
/// that same cylinder.
fn fallback_edge(radius: f64, theta0: f64, theta1: f64, vertex_z: f64, pcurve_v: f64) -> String {
    let mut records = unit_geometry(radius);
    records.append(&mut pcurve_pair_records(pcurve_v));
    records.push(format!(
        "#40 = CARTESIAN_POINT('', ({}, {}, {}));",
        roundtrip(radius * theta0.cos()),
        roundtrip(radius * theta0.sin()),
        roundtrip(vertex_z)
    ));
    records.push(format!(
        "#41 = CARTESIAN_POINT('', ({}, {}, {}));",
        roundtrip(radius * theta1.cos()),
        roundtrip(radius * theta1.sin()),
        roundtrip(vertex_z)
    ));
    records.push("#42 = VERTEX_POINT('', #40);".to_string());
    records.push("#43 = VERTEX_POINT('', #41);".to_string());
    // `#14` (the circle at z=0) is not on the vertex plane, so the primary
    // conversion parses but cannot reconcile; only the first pcurve (`#26`) is
    // what a correct fallback would accept.
    records.push("#50 = SURFACE_CURVE('', #14, (#26), .PCURVE_S1.);".to_string());
    records.push("#51 = EDGE_CURVE('', #42, #43, #50, .T.);".to_string());
    format!("DATA;\n{}\nENDSEC;", records.join("\n"))
}

/// A `SURFACE_CURVE` whose declared 3D-curve conversion cannot host the vertex
/// points is rescued by its pcurve realization: the realized curve reconciles
/// at both ends.
#[test]
fn occt_high_roi_cluster_001_pcurve_fallback_reconciles() {
    let theta0 = 1.0;
    let theta1 = 2.2;
    let vertex_z = 0.2;
    let data = fallback_edge(1.0, theta0, theta1, vertex_z, vertex_z);
    let edge = owned_edge(&data_section(&data), 51);
    let curve = edge.parse_curve3d().unwrap(); // H-1: the pcurve fallback reconciles
                                               // The rescued curve must be the pcurve realization, and it must reconcile
                                               // with the vertices at both ends.
    assert!(
        matches!(curve, Curve3D::PCurve(_)),
        "expected a pcurve realization, got {curve:?}"
    );
    near(
        curve.front(),
        Point3::new(theta0.cos(), theta0.sin(), vertex_z),
    );
    near(
        curve.back(),
        Point3::new(theta1.cos(), theta1.sin(), vertex_z),
    );
    let mid = curve.subs(0.5);
    let mid_angle = (theta0 + theta1) / 2.0;
    near(mid, Point3::new(mid_angle.cos(), mid_angle.sin(), vertex_z));
}

/// The issue-#1 shape: a `SURFACE_CURVE` whose pcurve realization is
/// branch-folded across the seam. The fallback must not accept it: the
/// conversion refuses with residual evidence, and the realized curve is never
/// the folded one.
#[test]
fn occt_high_roi_cluster_001_pcurve_fallback_rejects_wrong_branch() {
    // The edge straddles the seam (angles 5.9 and 6.4 on the 2π-periodic
    // chart). The primary curve (the z=0 circle) cannot reconcile with the
    // vertices at height 0.2, and the pcurve candidate is drawn at height 1.9
    // — its realized endpoints land on the wrong side of the seam, at a
    // residual far beyond tolerance.
    let vertex_z = 0.2;
    let data = fallback_edge(1.0, 5.9, 6.4, vertex_z, 1.9);
    let edge = owned_edge(&data_section(&data), 51);
    let err = edge.parse_curve3d().unwrap_err(); // H-1: nothing reconciles, honest refusal
    let message = format!("{err:?}");
    assert!(
        message.contains("residual"),
        "the refusal must carry residual evidence, got: {message}"
    );
}

/// `BREP_WITH_VOIDS` bookkeeping: one exterior `CLOSED_SHELL` and one void
/// `CLOSED_SHELL` convert with both shells present, exterior first, and the
/// face count is the sum of both shells' faces.
#[test]
fn occt_high_roi_cluster_001_void_solid_bookkeeping() {
    let data = "DATA;
#1 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));
#2 = CARTESIAN_POINT('', (1.0, 0.0, 0.0));
#3 = CARTESIAN_POINT('', (0.0, 1.0, 0.0));
#4 = CARTESIAN_POINT('', (0.0, 0.0, 0.5));
#5 = CARTESIAN_POINT('', (1.0, 0.0, 0.5));
#6 = CARTESIAN_POINT('', (0.0, 1.0, 0.5));
#10 = DIRECTION('', (0.0, 0.0, 1.0));
#11 = DIRECTION('', (1.0, 0.0, 0.0));
#12 = AXIS2_PLACEMENT_3D('', #1, #10, #11);
#13 = PLANE('', #12);
#20 = CARTESIAN_POINT('', (0.0, 0.0, 0.5));
#21 = AXIS2_PLACEMENT_3D('', #20, #10, #11);
#22 = PLANE('', #21);
#30 = DIRECTION('', (1.0, 0.0, 0.0));
#31 = VECTOR('', #30, 1.0);
#32 = LINE('', #1, #31);
#40 = VERTEX_POINT('', #1);
#41 = VERTEX_POINT('', #2);
#42 = VERTEX_POINT('', #3);
#50 = VERTEX_POINT('', #4);
#51 = VERTEX_POINT('', #5);
#52 = VERTEX_POINT('', #6);
#60 = EDGE_CURVE('', #40, #41, #32, .T.);
#61 = EDGE_CURVE('', #41, #42, #32, .T.);
#62 = EDGE_CURVE('', #42, #40, #32, .T.);
#63 = EDGE_LOOP('', (#60, #61, #62));
#64 = FACE_BOUND('', #63, .T.);
#65 = FACE_SURFACE('', (#64), #13, .T.);
#66 = CLOSED_SHELL('', (#65));
#70 = EDGE_CURVE('', #50, #51, #32, .T.);
#71 = EDGE_CURVE('', #51, #52, #32, .T.);
#72 = EDGE_CURVE('', #52, #50, #32, .T.);
#73 = EDGE_LOOP('', (#70, #71, #72));
#74 = FACE_BOUND('', #73, .T.);
#75 = FACE_SURFACE('', (#74), #22, .T.);
#76 = CLOSED_SHELL('', (#75));
#77 = ORIENTED_CLOSED_SHELL('', *, #76, .T.);
#78 = BREP_WITH_VOIDS('', #66, (#77));
ENDSEC;";
    let table = data_section(data);
    let solid = table.manifold_solid_brep.values().next().unwrap(); // H-1: the fixture declares one solid
    let csolid = table.to_compressed_solid(solid).unwrap(); // H-1: the fixture must convert
    assert_eq!(
        csolid.boundaries.len(),
        2,
        "outer shell and void shell both present"
    );
    let exterior = &csolid.boundaries[0];
    let void = &csolid.boundaries[1];
    assert_eq!(
        exterior.faces.len(),
        1,
        "the exterior shell contributes its face"
    );
    assert_eq!(void.faces.len(), 1, "the void shell contributes its face");
    assert_eq!(
        csolid
            .boundaries
            .iter()
            .map(|b| b.faces.len())
            .sum::<usize>(),
        exterior.faces.len() + void.faces.len(),
        "face count is the sum of both shells' faces"
    );
    // Ordering oracle: the exterior shell comes first (its vertices sit at
    // z = 0.0), then the void shell (its vertices at z = 0.5).
    let exterior_z = exterior
        .vertices
        .iter()
        .map(|p| p.z)
        .fold(f64::NEG_INFINITY, f64::max);
    let void_z = void
        .vertices
        .iter()
        .map(|p| p.z)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(exterior_z < 0.25, "the exterior shell is first");
    assert!(void_z > 0.25, "the void shell is second");
}
