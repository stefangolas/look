//! BG-KV2-401-S3C integration tests: the §9.4 trim clip - certified R9
//! crossings between an arc pcurve and the trim loops of the same chart, arc
//! splitting at the certified crossings, and inside/outside classification of
//! the sub-arcs by the winding number of the closed trim loop about a
//! certified-off interior sample. Outside sub-arcs are discarded; the trim
//! boundary endpoints become `TopoNode::TrimCrossing` nodes.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::{ArcCert, Frame, PointCert};
use truck_certified::kernel::config;
use truck_certified::kernel::evidence::{RefusalKind, VerdictClass};
use truck_certified::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, Break, CertifiedGraph, ChartId, HermiteSegment,
    HermiteSpline, Node, NodeCert, NodeId, Param, Point4, TopoNode,
};
use truck_certified::kernel::patch::IBox;
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::BezierLeaf1;
use truck_certified::kernel::trimclip::{
    certify_crossings, certify_off_loop, trim_clip, winding_number, TrimLoop,
};
use truck_certified::kernel::Interval;

/// The chart of the trim-clip fixtures.
const CH: ChartId = ChartId(9);

/// The certified contraction rate of the fixture certificates (<= RHO_MAX).
const RHO: f64 = 0.125;

/// Extract the `Ok` of a fallible construction; fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A unit-weight degree-1 leaf from two affine chart points.
fn line_leaf(chart: ChartId, from: [f64; 2], to: [f64; 2]) -> BezierLeaf1 {
    construct(BezierLeaf1::try_new(
        1,
        vec![[from[0], from[1], 0.0, 1.0], [to[0], to[1], 0.0, 1.0]],
        chart,
    ))
}

/// A unit-weight degree-2 leaf from three affine chart points.
fn quad_leaf(chart: ChartId, pts: [[f64; 2]; 3]) -> BezierLeaf1 {
    construct(BezierLeaf1::try_new(
        2,
        vec![
            [pts[0][0], pts[0][1], 0.0, 1.0],
            [pts[1][0], pts[1][1], 0.0, 1.0],
            [pts[2][0], pts[2][1], 0.0, 1.0],
        ],
        chart,
    ))
}

// ---------------------------------------------------------------------------
// The closed trim-loop fixture
// ---------------------------------------------------------------------------

/// The closed trim loop of the fixtures: a closed cubic loop
/// `C(r), r in [0, 1]`, `C(0) = C(1)`, enclosing a lens-shaped region around
/// the lower interior of the chart (its trace runs up the right side to the
/// top and back down the left side).
fn loop_leaf() -> BezierLeaf1 {
    construct(BezierLeaf1::try_new(
        3,
        vec![
            [0.5, 0.08, 0.0, 1.0],
            [0.98, 0.6, 0.0, 1.0],
            [0.02, 0.6, 0.0, 1.0],
            [0.5, 0.08, 0.0, 1.0],
        ],
        CH,
    ))
}

fn closed_loop() -> TrimLoop {
    TrimLoop {
        chart: CH,
        curve: loop_leaf(),
        closed: true,
    }
}

// ---------------------------------------------------------------------------
// Independent dense reference winding (angle sum over a sampled polygon).
// ---------------------------------------------------------------------------

/// Float de Casteljau over one coordinate list (independent of the module).
fn eval1(coeffs: &[f64], t: f64) -> f64 {
    let mut level: Vec<f64> = coeffs.to_vec();
    let mt = 1.0 - t;
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for pair in level.windows(2) {
            next.push(mt * pair[0] + t * pair[1]);
        }
        level = next;
    }
    level[0]
}

fn chart_at(leaf: &BezierLeaf1, t: f64) -> [f64; 2] {
    let x = eval1(&leaf.control.iter().map(|p| p[0]).collect::<Vec<f64>>(), t);
    let y = eval1(&leaf.control.iter().map(|p| p[1]).collect::<Vec<f64>>(), t);
    let w = eval1(&leaf.control.iter().map(|p| p[3]).collect::<Vec<f64>>(), t);
    [x / w, y / w]
}

/// The independent reference winding of a closed leaf about a sample: the
/// signed angle sum over a dense sampled polygon, divided by `2*pi`. This is a
/// different algorithm from the module's polynomial ray count (angle
/// arithmetic vs sign discipline); for a sample off the loop it is an integer.
fn ref_winding(leaf: &BezierLeaf1, sample: [f64; 2]) -> f64 {
    const N: usize = 4096;
    let mut angle = 0.0f64;
    let mut prev = chart_at(leaf, 0.0);
    for i in 1..=N {
        let t = i as f64 / N as f64;
        let cur = chart_at(leaf, t);
        let ax = prev[0] - sample[0];
        let ay = prev[1] - sample[1];
        let bx = cur[0] - sample[0];
        let by = cur[1] - sample[1];
        let cross = ax * by - ay * bx;
        let dot = ax * bx + ay * by;
        angle += cross.atan2(dot);
        prev = cur;
    }
    angle / std::f64::consts::TAU
}

/// The minimum distance of the sample to the sampled loop polygon (for
/// choosing well-inside / well-outside fixture points).
fn polygon_separation(leaf: &BezierLeaf1, sample: [f64; 2]) -> f64 {
    const N: usize = 1024;
    let mut min_sq = f64::INFINITY;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let p = chart_at(leaf, t);
        let dx = p[0] - sample[0];
        let dy = p[1] - sample[1];
        min_sq = min_sq.min(dx * dx + dy * dy);
    }
    min_sq.sqrt()
}

// ---------------------------------------------------------------------------
// Certified-graph fixture helpers
// ---------------------------------------------------------------------------

/// A certified point certificate at a residual over a degenerate box at a
/// chart point.
fn cert_at(residual: ResidualId, point: [f64; 2]) -> PointCert {
    let box_ = construct(truck_certified::kernel::patch::IBox2::try_new(
        [point[0], point[1]],
        [point[0], point[1]],
    ));
    construct(PointCert::try_new(residual, box_, RHO))
}

/// A certified topology node on the trim chart at a chart point.
fn node(id: usize, point: [f64; 2]) -> Node {
    let at = Point4 {
        p1: construct(Param::try_new(CH, 0, point[0], point[1])),
        p2: construct(Param::try_new(CH, 0, point[0], point[1])),
    };
    Node {
        id: NodeId(id),
        at,
        kind: TopoNode::Boundary,
        cert: NodeCert::Exact(cert_at(ResidualId::R1, point)),
    }
}

/// A certified straight ordinary arc between two chart points, over unit
/// parameter, referencing the two node ends.
fn straight_arc4(id: usize, from: [f64; 2], to: [f64; 2], first: ArcEnd, second: ArcEnd) -> Arc<4> {
    let z_hat = [from[0], from[1], 0.0, 1.0];
    let q = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let q_tau = [1.0, 0.0, 0.0, 0.0];
    let q_perp = [
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let a = [[0.0; 4]; 4];
    let frame = construct(Frame::try_new(z_hat, q, q_tau, q_perp, a));
    let i_tau = Interval { lo: 0.0, hi: 1.0 };
    let b_perp = construct(IBox::<4>::try_new([-1.0; 4], [1.0; 4]));
    let arc_cert = construct(ArcCert::try_new(
        ResidualId::R1,
        frame,
        i_tau,
        b_perp,
        RHO,
        vec![[0.0, 0.0]; 4],
        None,
    ));
    let d = [to[0] - from[0], to[1] - from[1], 0.0];
    let spline = construct(HermiteSpline::try_new(vec![HermiteSegment {
        p0: [from[0], from[1], 0.0],
        p1: [to[0], to[1], 0.0],
        t0: d,
        t1: d,
    }]));
    Arc {
        id: ArcId(id),
        approx: Approx { gamma: spline },
        cert: arc_cert,
        ends: (first, second),
    }
}

/// An empty break list (the fixtures carry no segment breaks).
fn no_breaks() -> Vec<Break> {
    Vec::new()
}

/// A certified graph over the given nodes and ordinary straight arcs.
fn graph_of(nodes: Vec<Node>, arcs: Vec<AnyArc>) -> CertifiedGraph {
    CertifiedGraph {
        nodes,
        breaks: no_breaks(),
        arcs,
        sheets: Vec::new(),
        exhaustive: false,
    }
}

// ---------------------------------------------------------------------------
// Test 1: certified R9 crossings
// ---------------------------------------------------------------------------

#[test]
fn r9_crossings_certify_between_arc_pcurve_and_trim_loop() {
    // The pcurve is the diagonal line C1(t) = (t, t); the trim curve is the
    // quadratic C2(r) = (r, r^2 + (5/3)r - 1/3) whose affine difference from
    // the diagonal vanishes exactly once, at the transverse crossing
    // (t, r) = (1/3, 1/3), whose chart point is (1/3, 1/3). 1/3 is not a
    // dyadic rational, so no subdivision boundary below DEPTH_MAX can carry
    // the root and the global solver must isolate it.
    let pcurve = line_leaf(CH, [0.0, 0.0], [1.0, 1.0]);
    let trim = quad_leaf(CH, [[0.0, -1.0 / 3.0], [0.5, 0.5], [1.0, 7.0 / 3.0]]);
    let crossings = construct(certify_crossings(&pcurve, &trim));
    assert_eq!(
        crossings.len(),
        1,
        "the diagonal must cross the quadratic trim exactly once, got {crossings:?}"
    );
    let crossing = &crossings[0];
    assert_eq!(crossing.cert.residual, ResidualId::R9);
    assert!(crossing.cert.rho <= config::RHO_MAX);
    // The certified (t, r) box must contain the transverse crossing pair.
    assert!(
        crossing.cert.box_.lo[0] <= 1.0 / 3.0 + 1e-9
            && 1.0 / 3.0 - 1e-9 <= crossing.cert.box_.hi[0],
        "certified t-axis must contain 1/3: {:?}",
        crossing.cert.box_
    );
    assert!(
        crossing.cert.box_.lo[1] <= 1.0 / 3.0 + 1e-9
            && 1.0 / 3.0 - 1e-9 <= crossing.cert.box_.hi[1],
        "certified r-axis must contain 1/3: {:?}",
        crossing.cert.box_
    );
    // The certified chart point is the pcurve image of the certified
    // t-axis midpoint (a representative of the certified box; the box itself
    // is the certificate).
    let t_mid = 0.5 * (crossing.cert.box_.lo[0] + crossing.cert.box_.hi[0]);
    let expected_point = chart_at(&pcurve, t_mid);
    assert!(
        (crossing.point[0] - expected_point[0]).abs() < 1e-12
            && (crossing.point[1] - expected_point[1]).abs() < 1e-12,
        "the certified chart point must be the pcurve image of the certified \
         t-midpoint {t_mid}: got {:?}, expected {expected_point:?}",
        crossing.point
    );
}

// ---------------------------------------------------------------------------
// Test 5: certified-off sample
// ---------------------------------------------------------------------------

#[test]
fn sample_certified_off_the_loop() {
    // A well-inside and a well-outside point of the closed loop are certified
    // off by R9 distance-positivity data; the certificate carries a positive
    // separation in some component over every box of the trim-parameter
    // partition.
    let inside = [0.5, 0.3];
    let outside = [0.5, 1.2];
    for sample in [inside, outside] {
        let cert = construct(certify_off_loop(sample, &closed_loop()));
        assert_eq!(cert.sample, sample);
        assert!(
            !cert.exclusions.is_empty(),
            "the off-loop certificate must partition the trim parameter"
        );
        // The partition covers [0, 1]: contiguous, ordered, first at 0 and
        // last at 1.
        assert!((cert.exclusions[0].r.0 - 0.0).abs() < 1e-12);
        for pair in cert.exclusions.windows(2) {
            assert!(
                (pair[1].r.0 - pair[0].r.1).abs() < 1e-9,
                "the exclusion boxes must form a contiguous partition: {:?} then {:?}",
                pair[0].r,
                pair[1].r
            );
        }
        let last = &cert.exclusions[cert.exclusions.len() - 1];
        assert!((last.r.1 - 1.0).abs() < 1e-12);
        for exclusion in &cert.exclusions {
            assert!(
                !exclusion.separation.contains(0.0),
                "every exclusion box must carry a certified component excluding zero"
            );
        }
    }

    // A sample that lies ON the loop cannot be certified off: both R9 residual
    // components contain zero about the on-loop parameter at every depth, so
    // the certificate refuses TrimClipFailed (Inconclusive).
    let on_loop = chart_at(&loop_leaf(), 0.25);
    match certify_off_loop(on_loop, &closed_loop()) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::TrimClipFailed);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(cert) => {
            panic!("an on-loop sample must not certify off the loop: {cert:?} at {on_loop:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Test 3: winding classification inside and outside
// ---------------------------------------------------------------------------

#[test]
fn winding_classification_inside_and_outside_on_fixture() {
    // The module's certified ray-crossing winding agrees with the independent
    // dense angle-sum reference at a well-inside point (non-zero, the signed
    // index) and at a well-outside point (zero), both certified off the loop.
    let inside = [0.5, 0.3];
    let outside = [0.5, 1.2];
    assert!(
        polygon_separation(&loop_leaf(), inside) > 0.05,
        "the inside sample must be clear of the loop"
    );
    assert!(
        polygon_separation(&loop_leaf(), outside) > 0.05,
        "the outside sample must be clear of the loop"
    );

    let expected_inside = ref_winding(&loop_leaf(), inside).round() as i64;
    let expected_outside = ref_winding(&loop_leaf(), outside).round() as i64;
    assert_ne!(
        expected_inside, 0,
        "the inside sample must have non-zero winding by the reference"
    );
    assert_eq!(
        expected_outside, 0,
        "the outside sample must have zero winding by the reference"
    );

    let winding = construct(winding_number(&closed_loop(), inside));
    assert_eq!(winding, expected_inside);
    let winding = construct(winding_number(&closed_loop(), outside));
    assert_eq!(winding, expected_outside);

    // The classification itself: inside is `winding != 0`, outside is 0, and
    // the sound precondition (certified off the loop) holds for both.
    construct(certify_off_loop(inside, &closed_loop()));
    construct(certify_off_loop(outside, &closed_loop()));
    assert_ne!(expected_inside, 0);
    assert_eq!(expected_outside, 0);
}

// ---------------------------------------------------------------------------
// Test 2: an arc splits at its certified trim crossings
// ---------------------------------------------------------------------------

#[test]
fn arc_splits_at_certified_trim_crossings() {
    // A single certified straight arc along the horizontal line y = 0.3 from
    // the left chart edge to the right chart edge crosses the closed loop
    // exactly twice (certified boxes at arc parameter about 0.34-0.38 and
    // 0.62-0.66). The clip splits the arc at those crossings: the outside
    // leading and trailing runs are discarded and the inside run between the
    // two TrimCrossing nodes is retained.
    let n0 = node(0, [0.0, 0.3]);
    let n1 = node(1, [1.0, 0.3]);
    let arc0 = AnyArc::Ordinary(straight_arc4(
        0,
        [0.0, 0.3],
        [1.0, 0.3],
        ArcEnd::Topo(NodeId(0)),
        ArcEnd::Topo(NodeId(1)),
    ));
    let graph = graph_of(vec![n0, n1], vec![arc0]);
    let out = construct(trim_clip(&graph, &[closed_loop()]));

    // The two crossings become TrimCrossing nodes; the two original boundary
    // nodes are untouched.
    assert_eq!(out.nodes.len(), 4, "nodes: 2 boundaries + 2 trim crossings");
    let crossings: Vec<&Node> = out
        .nodes
        .iter()
        .filter(|node| node.kind == TopoNode::TrimCrossing)
        .collect();
    assert_eq!(crossings.len(), 2, "exactly two TrimCrossing nodes");
    for crossing in &crossings {
        assert!(
            matches!(crossing.cert, NodeCert::Exact(_)),
            "a TrimCrossing node is certified exactly"
        );
        assert!(
            (crossing.at.p1.v - 0.3).abs() < 1e-9,
            "the crossings lie on the arc's line y = 0.3"
        );
        assert!(
            crossing.at.p1.u > 0.3 && crossing.at.p1.u < 0.7,
            "both crossings are interior to the chord: u = {}",
            crossing.at.p1.u
        );
    }

    // The single arc is split into three pieces; the two outside pieces are
    // discarded and the inside piece is retained between the two TrimCrossing
    // nodes.
    assert_eq!(out.arcs.len(), 1, "only the inside run is retained");
    let kept = match &out.arcs[0] {
        AnyArc::Ordinary(arc) => arc,
        _ => panic!("the retained sub-arc must be an ordinary arc"),
    };
    let (ArcEnd::Topo(a), ArcEnd::Topo(b)) = kept.ends else {
        panic!("the retained sub-arc must end at the two TrimCrossing nodes")
    };
    let a_kind = out
        .nodes
        .iter()
        .find(|node| node.id == a)
        .map(|node| node.kind);
    let b_kind = out
        .nodes
        .iter()
        .find(|node| node.id == b)
        .map(|node| node.kind);
    assert_eq!(a_kind, Some(TopoNode::TrimCrossing));
    assert_eq!(b_kind, Some(TopoNode::TrimCrossing));
    let p0 = kept.approx.gamma.segments[0].p0;
    let p1 = kept.approx.gamma.segments[0].p1;
    assert!((p0[1] - 0.3).abs() < 1e-9 && (p1[1] - 0.3).abs() < 1e-9);
    assert!(
        p0[0] < p1[0],
        "the retained run is ordered along the chord: {p0:?} to {p1:?}"
    );
    assert!(
        p0[0] > 0.3 && p1[0] < 0.7,
        "the retained run lies between the two crossings"
    );
    // The leading and trailing outside runs are gone: no arc touches the
    // original boundary nodes any more.
    for arc in &out.arcs {
        let ends = match arc {
            AnyArc::Ordinary(arc) => [arc.ends.0, arc.ends.1],
            _ => panic!("every retained arc in this fixture is ordinary"),
        };
        for end in ends {
            match end {
                ArcEnd::Topo(id) => assert!(
                    id != NodeId(0) && id != NodeId(1),
                    "outside pieces reaching the original ends must be discarded"
                ),
                ArcEnd::Seg(_) => panic!("no segment breaks are created by the clip"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test 4: an interior loop crossing a trim is clipped with no special case
// ---------------------------------------------------------------------------

#[test]
fn interior_loop_crossing_no_leaf_boundary_is_clipped() {
    // The §9 no-special-case fixture: a CLOSED interior loop of the 1-complex
    // (four certified straight arcs forming a square) that lies strictly inside
    // the chart and misses every leaf boundary, but crosses the face's closed
    // trim loop. Steps 3-6 clip it with no leaf-boundary handling: the two
    // vertical sides each cross the loop once (TrimCrossing nodes), the bottom
    // side stays inside (retained whole), and the top side plus the upper
    // vertical runs are outside (discarded).
    let corners: [[f64; 2]; 4] = [[0.42, 0.2], [0.58, 0.2], [0.58, 0.62], [0.42, 0.62]];
    // The ring misses every leaf boundary: it is strictly interior to the
    // unit chart.
    for corner in corners {
        assert!(corner[0] > 0.0 && corner[0] < 1.0 && corner[1] > 0.0 && corner[1] < 1.0);
    }
    let mut nodes = Vec::new();
    for (i, corner) in corners.iter().enumerate() {
        nodes.push(node(i, *corner));
    }
    let mut arcs = Vec::new();
    for i in 0..4 {
        arcs.push(AnyArc::Ordinary(straight_arc4(
            i,
            corners[i],
            corners[(i + 1) % 4],
            ArcEnd::Topo(NodeId(i)),
            ArcEnd::Topo(NodeId((i + 1) % 4)),
        )));
    }
    let graph = graph_of(nodes, arcs);
    let out = construct(trim_clip(&graph, &[closed_loop()]));

    // The clip produced two TrimCrossing nodes (one per vertical crossing) and
    // no leaf-boundary or other segment machinery.
    assert_eq!(out.nodes.len(), 6, "4 ring corners + 2 trim crossings");
    assert_eq!(
        out.nodes
            .iter()
            .filter(|node| node.kind == TopoNode::TrimCrossing)
            .count(),
        2
    );
    assert!(
        out.breaks.is_empty(),
        "no segment breaks are created by the clip"
    );
    for crossing in out
        .nodes
        .iter()
        .filter(|node| node.kind == TopoNode::TrimCrossing)
    {
        assert!(
            (crossing.at.p1.v - 0.446).abs() < 1e-2,
            "the crossings lie on the loop's upper arc"
        );
    }

    // Three retained arcs: the bottom side whole, and the inside lower run of
    // each vertical side. The top side and the upper vertical runs are outside
    // and were discarded.
    assert_eq!(out.arcs.len(), 3, "bottom + two vertical inside runs");
    let mut retained_nodes: Vec<Node> = Vec::new();
    for arc in &out.arcs {
        let AnyArc::Ordinary(arc) = arc else {
            panic!("every retained ring arc is ordinary")
        };
        let segment = &arc.approx.gamma.segments[0];
        let y0 = segment.p0[1];
        let y1 = segment.p1[1];
        assert!(
            y0.max(y1) <= 0.46,
            "no retained piece lies above the loop's upper crossing (v {})",
            y0.max(y1)
        );
        for end in [arc.ends.0, arc.ends.1] {
            match end {
                ArcEnd::Topo(id) => {
                    let node = match out.nodes.iter().find(|node| node.id == id) {
                        Some(node) => *node,
                        None => panic!("the retained arc end {id:?} must resolve to a node"),
                    };
                    retained_nodes.push(node);
                }
                ArcEnd::Seg(_) => panic!("no segment break ends a retained piece"),
            }
        }
    }
    // Every retained piece ends at a certified node: the original ring corners
    // (Boundary) or the two TrimCrossing nodes.
    for node in retained_nodes {
        assert!(matches!(node.cert, NodeCert::Exact(_)));
    }
}

// ---------------------------------------------------------------------------
// Test 6: depth-max failure refuses TrimClipFailed
// ---------------------------------------------------------------------------

#[test]
fn depth_max_failure_refuses_trim_clip_failed() {
    // The parabola C1(t) = (t, (t - 0.25)^2) is exactly tangent to the
    // horizontal trim C2(r) = (r, 0) at the dyadic contact (0.25, 0.25): the
    // R9 residual vanishes there but its Jacobian is singular, so no box
    // containing the contact can ever isolate a transverse root. The
    // subdivision stalls at DEPTH_MAX and the clip refuses TrimClipFailed
    // (Inconclusive) - the named §9.4 refusal. The fixture is NOT supposed to
    // isolate (a tangent contact has no transverse crossing), so the refusal
    // is the deliverable.
    let parabola = quad_leaf(CH, [[0.0, 0.0625], [0.5, -0.1875], [1.0, 0.5625]]);
    let axis = line_leaf(CH, [0.0, 0.0], [1.0, 0.0]);

    match certify_crossings(&parabola, &axis) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::TrimClipFailed);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(crossings) => panic!(
            "a tangential contact must refuse TrimClipFailed at depth max, got {crossings:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// Test 7: N4 discipline scan
// ---------------------------------------------------------------------------

/// Strip `//` line comments, `///`/`//!` doc comments, and `/* ... */` blocks.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < chars.len() && depth > 0 {
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if chars[i] == '/' && (i + 1 >= chars.len() || chars[i + 1] == '/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[test]
fn no_transcendental_call_in_trimclip_module() {
    // N4: the module performs no transcendental call - no sin, cos, atan2,
    // exp, ln, log, powf, and no sqrt anywhere (whole words, comments
    // stripped).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/trimclip.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("trimclip.rs must be readable: {err}"),
    };
    let code = strip_comments(&source);
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let contains_word = |hay: &str, word: &str| {
        hay.match_indices(word).any(|(i, _)| {
            let before = i
                .checked_sub(1)
                .map(|j| hay.as_bytes()[j] as char)
                .map(is_word)
                .unwrap_or(false);
            let after = hay
                .as_bytes()
                .get(i + word.len())
                .map(|b| *b as char)
                .map(is_word)
                .unwrap_or(false);
            !before && !after
        })
    };
    for needle in ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"] {
        let present = code
            .lines()
            .any(|line| contains_word(line, needle) || line.contains("std::f64::consts"));
        assert!(
            !present,
            "no transcendental call may appear outside comments in trimclip.rs (found {needle})"
        );
    }
}
