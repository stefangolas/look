//! BG-KV2-202-S1A integration tests: the §7 R8/R9 square residuals — the
//! curve–surface system `H(t,u,v) = C(t) − S(u,v)` (arity 3, square C1 via
//! `krawczyk_c1_n3`) and the one-chart curve–curve system `J(t,r) = C₁(t) −
//! C₂(r)` (arity 2, square C1 via `krawczyk_c1`) — their constructors, their
//! C1 certification, and their N5/N4 discipline.

#![deny(clippy::unwrap_used)]

use std::fmt::Debug;

use truck_certified::kernel::certs::PointCert3;
use truck_certified::kernel::config;
use truck_certified::kernel::engine::{krawczyk_c1, krawczyk_c1_n3, SquareResidualEval};
use truck_certified::kernel::evidence::{ClaimVerdict, Refusal, RefusalKind, VerdictClass};
use truck_certified::kernel::graph::ChartId;
use truck_certified::kernel::leaf::BezierLeaf;
use truck_certified::kernel::patch::{CertifiedPatch, CertifiedPositive, IBox2, IBox3};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::{BezierLeaf1, CurveCarrierKind, R8System, R9System};
use truck_certified::kernel::Interval;

/// Extract the `Ok` of any fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("a construction that must succeed was refused: {err:?}"),
    }
}

/// A 2-axis parameter box.
fn box2(u_lo: f64, u_hi: f64, v_lo: f64, v_hi: f64) -> IBox2 {
    construct_ok(IBox2::try_new([u_lo, v_lo], [u_hi, v_hi]))
}

/// A 3-axis parameter box.
fn box3(lo: [f64; 3], hi: [f64; 3]) -> IBox3 {
    construct_ok(IBox3::try_new(lo, hi))
}

/// One certified positive unit weight (the §7.1 value argument).
fn positive_one() -> CertifiedPositive {
    construct_ok(CertifiedPositive::try_new(1.0))
}

/// The certified positive weight bound of a surface leaf over a `(u,v)` box,
/// obtained from [`CertifiedPatch::weight_bound`] (the §7.1 value-argument
/// discipline).
fn leaf_weight(surface: &BezierLeaf, uv: IBox2) -> CertifiedPositive {
    match CertifiedPatch::weight_bound(surface, uv) {
        Some(ClaimVerdict::Proven(positive)) => positive,
        Some(other) => panic!("the fixture leaf must yield a Proven weight bound: {other:?}"),
        None => panic!("BezierLeaf::weight_bound never returns None"),
    }
}

/// A point interval.
fn iv(x: f64) -> Interval {
    Interval::point(x)
}

// ---------------------------------------------------------------------------
// R8 fixtures
// ---------------------------------------------------------------------------

/// The unit-weight plane `S(u, v) = (u, v, 0)` at bidegree `(1, 1)`.
fn plane_surface() -> BezierLeaf {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

/// The unit-weight plane `S(u, v) = (u, v, u)` (the plane `z = x`) at
/// bidegree `(1, 1)`.
fn slant_plane_surface() -> BezierLeaf {
    let control = vec![
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
    ];
    construct_ok(BezierLeaf::try_new(1, 1, control))
}

/// A degree-1 curve leaf from its two homogeneous endpoints.
fn line_leaf(chart: ChartId, p0: [f64; 4], p1: [f64; 4]) -> BezierLeaf1 {
    construct_ok(BezierLeaf1::try_new(1, vec![p0, p1], chart))
}

/// The regular-root fixture: the line `C(t) = (−¼ + t, ¼ + t/2, −½ + t)`
/// pierces the plane `S(u,v) = (u, v, 0)` exactly at the dyadic root
/// `(t*, u*, v*) = (½, ¼, ½)`, with `C'(t) = (1, ½, 1)` transverse to the
/// tangent plane (`det DH = 1`).
fn line_pierce_system() -> R8System {
    let curve = line_leaf(ChartId(0), [-0.25, 0.25, -0.5, 1.0], [0.75, 0.75, 0.5, 1.0]);
    construct_ok(R8System::try_new(&curve, &plane_surface()))
}

/// The tangency fixture: the line `C(t) = (t, ½, t)` lies IN the plane
/// `S(u,v) = (u, v, u)` (its tangent plane at the contact), so `DH` is
/// singular everywhere along the contact and the C1 must refuse.
fn tangent_line_system() -> R8System {
    let curve = line_leaf(ChartId(0), [0.0, 0.5, 0.0, 1.0], [1.0, 0.5, 1.0, 1.0]);
    construct_ok(R8System::try_new(&curve, &slant_plane_surface()))
}

// ---------------------------------------------------------------------------
// R9 fixtures
// ---------------------------------------------------------------------------

/// A degree-2 curve leaf from its three homogeneous `(x, y, z, w)` control
/// points.
fn quadratic_leaf(chart: ChartId, control: Vec<[f64; 4]>) -> BezierLeaf1 {
    construct_ok(BezierLeaf1::try_new(2, control, chart))
}

/// The first crossing curve `C1(t) = (2t, 4t(1 − t))`: control polygon
/// `(0,0), (1,2), (2,0)` at unit weight, planar `z = 0`.
fn curve_a() -> BezierLeaf1 {
    quadratic_leaf(
        ChartId(3),
        vec![
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 2.0, 0.0, 1.0],
            [2.0, 0.0, 0.0, 1.0],
        ],
    )
}

/// The second crossing curve `C2(r) = (2r, −1 + 6r − 4r²)`: control polygon
/// `(0,−1), (1,2), (2,1)` at unit weight. It crosses `C1` at the single dyadic
/// parameter pair `(t*, r*) = (½, ½)` at the point `(1, 1)`.
fn curve_b() -> BezierLeaf1 {
    quadratic_leaf(
        ChartId(3),
        vec![
            [0.0, -1.0, 0.0, 1.0],
            [1.0, 2.0, 0.0, 1.0],
            [2.0, 1.0, 0.0, 1.0],
        ],
    )
}

/// The non-crossing curve `C2n(r) = (2r, −2 + 4r − 3r²)`: control polygon
/// `(0,−2), (1,0), (2,−1)`. It never meets `C1` (the `y` gap is `2 − t² ≥ 1`
/// on the shared `x = 2s` parameter line).
fn curve_c_non_crossing() -> BezierLeaf1 {
    quadratic_leaf(
        ChartId(3),
        vec![
            [0.0, -2.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [2.0, -1.0, 0.0, 1.0],
        ],
    )
}

// ---------------------------------------------------------------------------
// Constructor tests
// ---------------------------------------------------------------------------

#[test]
fn r8_system_builds_from_curve_and_surface_leaves() {
    let curve = line_leaf(ChartId(0), [-0.25, 0.25, -0.5, 1.0], [0.75, 0.75, 0.5, 1.0]);
    let surface = plane_surface();
    let sys = construct_ok(R8System::try_new(&curve, &surface));

    assert_eq!(sys.curve().degree, 1);
    assert_eq!(sys.curve().control, curve.control);
    assert_eq!(sys.surface().degree_u, 1);
    assert_eq!(sys.surface().degree_v, 1);
    assert_eq!(sys.arity(), 3);

    // The homogeneous residual vanishes (up to the certified enclosure) at the
    // known pierce point `(t, u, v) = (½, ¼, ½)`.
    let at_root = sys.eval(&[iv(0.5), iv(0.25), iv(0.5)]);
    for (k, component) in at_root.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the R8 residual must vanish at the pierce point: component {k} = {component:?}"
        );
    }
}

#[test]
fn r8_refuses_nonfinite_or_degree_zero_inputs() {
    // A zero-degree curve leaf refuses ClaimRefuted (Disproven).
    let zero_degree = BezierLeaf1::try_new(0, vec![[0.0, 0.0, 0.0, 1.0]], ChartId(0));
    match zero_degree {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
            match &refusal.evidence {
                truck_certified::kernel::evidence::RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(*name, "bezier1_zero_degree");
                }
                _ => panic!("the zero-degree refusal must carry predicate evidence"),
            }
        }
        Ok(_) => panic!("a zero-degree curve leaf must refuse"),
    }

    // Non-finite coefficients refuse NonFinite (Disproven).
    let non_finite = BezierLeaf1::try_new(
        1,
        vec![[0.0, 0.0, 0.0, 1.0], [f64::NAN, 0.0, 0.0, 1.0]],
        ChartId(0),
    );
    match non_finite {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NonFinite);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a non-finite curve leaf must refuse"),
    }

    // A non-positive weight refuses WeightDegenerate (Disproven), the §7.1
    // degenerate-positive-certificate class.
    let degenerate = BezierLeaf1::try_new(
        1,
        vec![[0.0, 0.0, 0.0, 1.0], [0.5, 0.5, 0.0, 0.0]],
        ChartId(0),
    );
    match degenerate {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::WeightDegenerate);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a zero-weight curve leaf must refuse"),
    }

    // A zero-degree surface leaf refuses through the landed BezierLeaf gate.
    match BezierLeaf::try_new(0, 0, vec![[0.0, 0.0, 0.0, 1.0]]) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a zero-degree surface leaf must refuse"),
    }

    // The R8 system re-runs the admission gate on raw leaves (the fields are
    // public, so an unvalidated net can reach the residual): a raw degree-0
    // curve leaf refuses at R8System::try_new.
    let raw_zero_degree = BezierLeaf1 {
        degree: 0,
        control: vec![[0.0, 0.0, 0.0, 1.0]],
        chart: ChartId(0),
        carrier: CurveCarrierKind::Rational,
    };
    let sys = R8System::try_new(&raw_zero_degree, &plane_surface());
    match sys {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a raw zero-degree curve leaf must refuse the R8 system"),
    }

    // A transcendental-carrier curve marker (the rational-leaves-only gate:
    // a transcendental-only carrier cannot be certified by a rational
    // residual) refuses TranscendentalCarrier (Disproven) at try_new. The
    // marker is carried as curve-leaf provenance — the §3.2 refusal kind is
    // caller-constructible, and this module is the R8 caller.
    let transcendental = BezierLeaf1 {
        degree: 1,
        control: vec![[-0.25, 0.25, -0.5, 1.0], [0.75, 0.75, 0.5, 1.0]],
        chart: ChartId(0),
        carrier: CurveCarrierKind::Transcendental,
    };
    let sys = R8System::try_new(&transcendental, &plane_surface());
    match sys {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::TranscendentalCarrier);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a transcendental-carrier curve leaf must refuse the R8 system"),
    }

    // A non-finite raw curve leaf refuses NonFinite at the system gate too.
    let raw_non_finite = BezierLeaf1 {
        degree: 1,
        control: vec![[0.0, 0.0, 0.0, 1.0], [0.5, f64::INFINITY, 0.0, 1.0]],
        chart: ChartId(0),
        carrier: CurveCarrierKind::Rational,
    };
    match R8System::try_new(&raw_non_finite, &plane_surface()) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::NonFinite);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a non-finite raw curve leaf must refuse the R8 system"),
    }

    // A raw zero-degree surface leaf refuses at the R8 system gate.
    let raw_surface = BezierLeaf {
        degree_u: 0,
        degree_v: 1,
        control: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]],
    };
    let curve = line_leaf(ChartId(0), [-0.25, 0.25, -0.5, 1.0], [0.75, 0.75, 0.5, 1.0]);
    match R8System::try_new(&curve, &raw_surface) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a raw zero-degree surface leaf must refuse the R8 system"),
    }
}

#[test]
fn r8_regular_root_certifies_with_point_cert() {
    let sys = line_pierce_system();
    // The unique root `(t, u, v) = (½, ¼, ½)` is interior to the box.
    let root = [0.5, 0.25, 0.5];
    let b = box3([0.45, 0.2, 0.45], [0.55, 0.3, 0.55]);

    // The §7.1 weight value argument: the certified positive bound of the
    // surface's unit weight over the `(u, v)` box.
    let uv = box2(0.2, 0.3, 0.45, 0.55);
    let w = vec![leaf_weight(sys.surface(), uv)];

    // Ground the fixture: the homogeneous residual vanishes at the root.
    let at_root = sys.eval(&[iv(root[0]), iv(root[1]), iv(root[2])]);
    for (k, component) in at_root.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the R8 residual must vanish at the known root: component {k} = {component:?}"
        );
    }

    match krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.box_, b);
            // The engine stamps R1; rebuild through the documented one-line
            // seam with the residual's own id.
            let cert = construct_ok(PointCert3::try_new(ResidualId::R8, cert.box_, cert.rho));
            assert_eq!(cert.residual, ResidualId::R8);
            assert_eq!(cert.box_, b);
            for axis in 0..3 {
                assert!(
                    cert.box_.lo[axis] <= root[axis] && root[axis] <= cert.box_.hi[axis],
                    "certified box must contain the pierce root on axis {axis}"
                );
            }
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the line-pierce-plane root must certify Proven, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the line-pierce-plane root must certify Proven, inconclusive: {reason}")
        }
    }
}

#[test]
fn r8_transversality_refusal_when_tangent_to_surface() {
    let sys = tangent_line_system();
    // The line lies IN the plane `z = x` (its tangent plane at the contact),
    // so `det DH = 0` everywhere along the contact and no isolated root of the
    // cross-multiplied residual exists in any box about it: the C1 must refuse
    // with the conditioning-class outcome (never a wrong Proven).
    let b = box3([0.4, 0.4, 0.45], [0.6, 0.6, 0.55]);
    let w = vec![positive_one()];

    // Ground the fixture: a contact point `(t, u, v) = (½, ½, ½)` is ON the
    // shared locus, so the residual vanishes there while the Jacobian stays
    // singular.
    let at_contact = sys.eval(&[iv(0.5), iv(0.5), iv(0.5)]);
    for (k, component) in at_contact.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the residual must vanish along the tangential contact: component {k} = {component:?}"
        );
    }

    match krawczyk_c1_n3(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            panic!("a tangential (non-transverse) curve must never certify a point: {cert:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            // The singular midpoint Jacobian refuses as Inconclusive (the
            // conditioning class of the S2A backing table). This is the
            // regularity note of §7 R8 in action: a Proven PointCert IS the
            // transversality certificate, and this box has none.
            assert!(
                !reason.is_empty(),
                "an Inconclusive refusal must carry a reason"
            );
        }
        ClaimVerdict::Disproven(refusal) => {
            // A refused conditioning-class construction is the other honest
            // refusal arm; the assertion is that the certificate was NOT
            // issued (the S2A backing table never lets a tangency box certify).
            assert!(
                refusal.kind == RefusalKind::ClaimRefuted
                    || refusal.kind == RefusalKind::Conditioning,
                "a tangential refusal must be conditioning-class, got {refusal:?}"
            );
        }
    }
}

#[test]
fn r9_system_builds_from_two_curve_leaves_in_one_chart() {
    let a = curve_a();
    let b = curve_b();
    let sys = construct_ok(R9System::try_new(&a, &b));

    assert_eq!(sys.chart, ChartId(3));
    assert_eq!(sys.a().degree, 2);
    assert_eq!(sys.b().degree, 2);
    assert_eq!(sys.arity(), 2);

    // The residual vanishes at the known crossing parameter pair.
    let at_root = sys.eval(&[iv(0.5), iv(0.5)]);
    for (k, component) in at_root.iter().enumerate() {
        assert!(
            component.contains(0.0),
            "the R9 residual must vanish at the crossing: component {k} = {component:?}"
        );
    }

    // A mismatched chart refuses with the `r9_requires_one_chart` predicate.
    let other_chart = BezierLeaf1 {
        degree: b.degree,
        control: b.control.clone(),
        chart: ChartId(4),
        carrier: CurveCarrierKind::Rational,
    };
    let mismatched = R9System::try_new(&a, &other_chart);
    match mismatched {
        Err(Refusal {
            kind,
            evidence: truck_certified::kernel::evidence::RefusalEvidence::Predicate { name, .. },
            ..
        }) => {
            assert_eq!(kind, RefusalKind::ChartExhausted);
            assert_eq!(name, "r9_requires_one_chart");
        }
        Err(other) => panic!(
            "a mismatched chart must refuse with the r9_requires_one_chart predicate: {other:?}"
        ),
        Ok(_) => panic!("two curves in different charts must not build an R9 system"),
    }
}

#[test]
fn r9_crossing_certifies_and_non_crossing_disproves() {
    // --- Crossing pair: C1 meets C2 at the single dyadic pair (½, ½). ---
    let sys = construct_ok(R9System::try_new(&curve_a(), &curve_b()));
    let root = [0.5, 0.5];
    let b = box2(0.48, 0.52, 0.48, 0.52);
    let w = vec![positive_one()];

    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Proven(cert) => {
            assert!(cert.rho <= config::RHO_MAX, "rho must satisfy the ceiling");
            assert_eq!(cert.box_, b);
            // Engine stamps R1; rebuild with the residual's own id.
            let cert = construct_ok(truck_certified::kernel::certs::PointCert::try_new(
                ResidualId::R9,
                cert.box_,
                cert.rho,
            ));
            assert_eq!(cert.residual, ResidualId::R9);
            for axis in 0..2 {
                assert!(
                    cert.box_.lo[axis] <= root[axis] && root[axis] <= cert.box_.hi[axis],
                    "certified box must contain the crossing on axis {axis}"
                );
            }
        }
        ClaimVerdict::Disproven(refusal) => {
            panic!("the quadratic crossing must certify Proven, refused: {refusal:?}")
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("the quadratic crossing must certify Proven, inconclusive: {reason}")
        }
    }

    // --- Non-crossing pair over a shared box: no crossing exists, so the C1
    // outcome is Disproven-backed (K disjoint from B) per the S2A backing
    // table. ---
    let sys = construct_ok(R9System::try_new(&curve_a(), &curve_c_non_crossing()));
    let b = box2(0.3, 0.7, 0.3, 0.7);
    match krawczyk_c1(&sys, b, &w) {
        ClaimVerdict::Disproven(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        ClaimVerdict::Inconclusive(reason) => {
            panic!("a fully separated curve pair must give a disjoint Krawczyk image, got Inconclusive: {reason}");
        }
        ClaimVerdict::Proven(cert) => {
            panic!("non-crossing curves must never certify a crossing: {cert:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// N5/N4 discipline scans
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
fn homogeneous_evaluation_no_premature_division() {
    // N5: the residual module evaluates ONLY the homogeneous cross-multiplied
    // polynomials — no `/` may appear on any code line of residuals_r89.rs
    // (there is no weight-bearing interval division anywhere; the §7.1 weights
    // are a value argument to the C1 entries, never a denominator here). Any
    // slash outside comments is a premature dehomogenization.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/residuals_r89.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("residuals_r89.rs must be readable: {err}"),
    };
    let code = strip_comments(&source);
    let slash_lines: Vec<&str> = code.lines().filter(|line| line.contains('/')).collect();
    assert!(
        slash_lines.is_empty(),
        "no division may appear outside comments in residuals_r89.rs: {slash_lines:?}"
    );
}

#[test]
fn no_transcendental_call_in_r89_module() {
    // N4: the residual module performs no transcendental call — no sin, cos,
    // atan2, exp, ln, log, powf, and no sqrt anywhere (whole words, comments
    // stripped).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/kernel/residuals_r89.rs");
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => panic!("residuals_r89.rs must be readable: {err}"),
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
            "no transcendental call may appear outside comments in residuals_r89.rs (found {needle})"
        );
    }
}
