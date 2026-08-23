use super::*;

#[test]
fn construct_polylines_positive0() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 9);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0 || f64::abs(x) == 7.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

#[test]
fn construct_polylines_positive1() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(0.0, 0.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 2);
    assert_eq!(polyline[0].len(), 5);
    assert_eq!(polyline[1].len(), 5);
}

#[test]
fn construct_polylines_positive2() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 9);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0 || f64::abs(x) == 7.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

#[test]
fn construct_polylines_positive3() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 8);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

/// Builds the two-segment weld scenario translated by `t` and asserts the
/// shared sub-tolerance endpoint welds into ONE three-node polyline.
fn assert_welded_wire_at(t: Vector3) {
    const WELD_SEP: f64 = 5.0e-7; // H-3: shared-endpoint separation, half the legacy tolerance (1e-6)
    const SEGMENT: f64 = 1.0; // H-3: segment length in model units, far above the weld scale
    let shared_a = Point3::new(0.0, 0.0, 0.0) + t;
    let shared_b = Point3::new(WELD_SEP, 0.0, 0.0) + t;
    let lines = vec![
        (shared_a, Point3::new(SEGMENT, 0.0, 0.0) + t),
        (shared_b, Point3::new(0.0, SEGMENT, 0.0) + t),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(
        polyline.len(),
        1,
        "the shared sub-tolerance endpoint must weld into one polyline at {t:?}"
    );
    assert_eq!(
        polyline[0].len(),
        3,
        "the welded wire visits each outer end once and the shared node once, at {t:?}"
    );
}

#[test]
fn welds_subtolerance_shared_endpoints_at_offsets() {
    // Two segments whose logical shared endpoint is represented by two points
    // WELD_SEP apart. The legacy hash grid (pitch 2e-6) split such a pair into
    // DIFFERENT cells at some absolute positions — position-dependent node
    // identity, F-2's failure direction. Canonical near_pt representatives
    // must weld them at every offset, including large ones.
    const MID_TX: f64 = 1.0e3; // H-3: mid-scale offset in x, a position, not a tolerance
    const MID_TY: f64 = -2.0e3; // H-3: mid-scale offset in y, a position, not a tolerance
    const MID_TZ: f64 = 3.0e3; // H-3: mid-scale offset in z, a position, not a tolerance
    const FAR_TX: f64 = 1.0e6; // H-3: far-scale offset in x, a position, not a tolerance
    const FAR_TY: f64 = 5.0e5; // H-3: far-scale offset in y, a position, not a tolerance
    const FAR_TZ: f64 = -7.0e5; // H-3: far-scale offset in z, a position, not a tolerance

    // The old grid demonstrably splits the weld pair at SPLIT_TX: the two
    // endpoints land in cells (500000000000, ...) and (500000000001, ...).
    // The new node identity must still weld.
    const SPLIT_TX: f64 = 1.0e6 + 7.5e-7; // H-3: split-verified far-scale offset in x (a grid-cell boundary), a position
    const SPLIT_TY: f64 = 5.0e5; // H-3: split-verified far-scale offset in y, a position, not a tolerance
    const SPLIT_TZ: f64 = -7.0e5; // H-3: split-verified far-scale offset in z, a position, not a tolerance
    let offsets = [
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(MID_TX, MID_TY, MID_TZ),
        Vector3::new(FAR_TX, FAR_TY, FAR_TZ),
        Vector3::new(SPLIT_TX, SPLIT_TY, SPLIT_TZ),
    ];
    for t in offsets {
        assert_welded_wire_at(t);
    }
}

#[test]
fn keeps_distinct_nearby_endpoints_separate() {
    // Endpoints DISTINCT_SEP apart (3x the legacy tolerance) must NOT weld:
    // each segment stays its own two-node polyline, so the resulting count
    // and lengths are asserted exactly.
    const DISTINCT_SEP: f64 = 3.0e-6; // H-3: endpoint separation, 3x the legacy tolerance, above the weld threshold
    const SEGMENT: f64 = 1.0; // H-3: segment length in model units, far above the separation scale
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(SEGMENT, 0.0, 0.0)),
        (
            Point3::new(DISTINCT_SEP, 0.0, 0.0),
            Point3::new(0.0, SEGMENT, 0.0),
        ),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(
        polyline.len(),
        2,
        "endpoints 3x the legacy tolerance apart stay distinct nodes"
    );
    assert_eq!(polyline[0].len(), 2);
    assert_eq!(polyline[1].len(), 2);
}
