//! The kernel-v2 contract tests (BG-KV2-000-CONTRACT): the shared shapes,
//! their refusing constructors, and the fixture kit's machine-checked ground
//! truths. No solver is implemented or invoked here — every numerical fact is
//! a direct evaluation of stored data, an exact coefficient evaluation, or a
//! CertifiedInterval-style enclosure of a polynomial.

#![deny(clippy::unwrap_used)]

use truck_certified::formal::exact::CertifiedInterval;
use truck_certified::kernel::certs::{ArcCert, ContactCert, Frame, PointCert, PsiMapKind};
use truck_certified::kernel::config;
use truck_certified::kernel::evidence::{
    default_backing, Refusal as KernelRefusal, RefusalEvidence, RefusalKind, VerdictClass,
};
use truck_certified::kernel::fixtures as fx;
use truck_certified::kernel::graph::{Param, SegmentBreak, TopoNode};
use truck_certified::kernel::patch::{CertifiedNonzero, CertifiedPositive, IBox2};
use truck_certified::kernel::residual::{implication, Implication, ResidualId};

/// The fixture kit's dyadic ground-truth comparison tolerance (H-3).
const GT_TOL: f64 = 1e-12; // H-3: fixture ground-truth comparison tolerance
/// The threshold for "clearly off the circle".
const GT_OFF: f64 = 1e-3; // H-3: fixture off-circle separation threshold

/// Assert two floats agree to the fixture kit's ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < GT_TOL
}

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Parse the unit-variant names of an enum declaration from source text.
fn enum_variant_names(source: &str, enum_name: &str) -> Vec<String> {
    let marker = format!("pub enum {enum_name} {{");
    let start = match source.find(&marker) {
        Some(index) => index + marker.len(),
        None => panic!("enum {enum_name} not found in source"),
    };
    let body = &source[start..];
    let end = match body.find('}') {
        Some(index) => index,
        None => panic!("enum {enum_name} body not closed in source"),
    };
    let mut names = Vec::new();
    for line in body[..end].lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }
        if let Some(name) = trimmed.strip_suffix(',') {
            let name = name.trim();
            if name.starts_with(|c: char| c.is_uppercase()) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// The 25 §17 variants, in declaration order.
fn all_refusal_kinds() -> Vec<RefusalKind> {
    use RefusalKind::*;
    vec![
        SpineNotC1,
        FrameSingular,
        ProfileCollapse,
        ProfileCorrespondenceMismatch,
        NonFinite,
        WindingAuditFailed,
        NonDyadicSharedRequest,
        CarrierSingularity,
        ChartExhausted,
        TranscendentalCarrier,
        WeightDegenerate,
        DeckExhausted,
        Conditioning,
        TangentialCurve,
        HighOrderJet,
        IncompleteStartSet,
        R5EnclosureFailed,
        TrimClipFailed,
        NearOverlap,
        OffsetDegenerate,
        OffsetSwallowtail,
        CornerUnsolved,
        SliverOrNearOverlap,
        ClaimRefuted,
        Budget,
    ]
}

/// The §17 backing class each kind must default to (spec table).
fn expected_backing(kind: RefusalKind) -> VerdictClass {
    use RefusalKind::*;
    match kind {
        DeckExhausted | Conditioning | TangentialCurve | HighOrderJet | IncompleteStartSet
        | R5EnclosureFailed | TrimClipFailed | CornerUnsolved | SliverOrNearOverlap | Budget => {
            VerdictClass::Inconclusive
        }
        _ => VerdictClass::Disproven,
    }
}

/// All 11 §7 residual ids, in declaration order.
fn all_residual_ids() -> Vec<ResidualId> {
    use ResidualId::*;
    vec![R1, R2, R3, R4, R4Prime, R5, R6, R7, R8, R9, Carrier]
}

/// A valid identity frame at dimension `N`, used to exercise the frame checks.
fn valid_frame() -> Frame<3> {
    let z_hat = [0.0, 0.0, 1.0];
    let q = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let q_tau = [1.0, 0.0, 0.0];
    let q_perp = [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
    let a = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    construct(Frame::try_new(z_hat, q, q_tau, q_perp, a))
}

/// A valid dimension-2 frame for the tube-certificate tests.
fn valid_frame2() -> Frame<2> {
    let z_hat = [0.0, 1.0];
    let q = [[1.0, 0.0], [0.0, 1.0]];
    let q_tau = [1.0, 0.0];
    let q_perp = [[0.0, 1.0], [1.0, 0.0]];
    let a = [[1.0, 0.0], [0.0, 1.0]];
    construct(Frame::try_new(z_hat, q, q_tau, q_perp, a))
}

/// A valid dimension-2 tube certificate at the given contraction rate.
fn valid_arc2(rho: f64) -> Result<ArcCert<2>, KernelRefusal> {
    let box_ = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    ArcCert::try_new(
        ResidualId::R1,
        valid_frame2(),
        CertifiedInterval::point(0.0),
        box_,
        rho,
        vec![[-1.0, 1.0]],
        Some(vec![construct(CertifiedPositive::try_new(1.0))]),
    )
}

fn weight_on(weights: [f64; 3], t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * weights[0] + 2.0 * t * mt * weights[1] + t * t * weights[2]
}

#[test]
fn kernel_config_constants_match_spec_defaults() {
    assert_eq!(config::EPS_REP, 1e-9); // H-3: normative §0.4 default
    assert_eq!(config::RHO_MAX, 0.5);
    assert_eq!(config::KAPPA_MAX, 1e6);
    assert_eq!(config::DEPTH_MAX, 40);
    assert_eq!(config::KA, 4);
    assert_eq!(config::DECK_MAX, 8);
    assert_eq!(config::TOL_POSITION, 1e-9); // H-3: normative §0.4 default
    assert_eq!(config::TOL_PARAMETER, 1e-11); // H-3: normative §0.4 default
    assert_eq!(config::TOL_JACOBIAN, 1e-12); // H-3: normative §0.4 default
    assert_eq!(config::TOL_INTERSECTION, config::EPS_REP);
}

#[test]
fn refusal_kind_has_all_spec_variants() {
    let kinds = all_refusal_kinds();
    assert_eq!(kinds.len(), 25, "exactly the 25 §17 variants");
    let mut seen = std::collections::HashSet::new();
    for kind in &kinds {
        assert!(seen.insert(*kind), "variant {kind:?} duplicated");
    }
    let source = include_str!("../src/kernel/evidence.rs");
    let parsed = enum_variant_names(source, "RefusalKind");
    let expected: Vec<String> = kinds.iter().map(|k| format!("{k:?}")).collect();
    assert_eq!(
        parsed, expected,
        "the source enum body must carry exactly the 25 §17 variants"
    );
}

#[test]
fn refusal_backing_class_matches_spec() {
    for kind in all_refusal_kinds() {
        assert_eq!(
            default_backing(kind),
            expected_backing(kind),
            "default backing of {kind:?}"
        );
    }
    assert_eq!(
        default_backing(RefusalKind::WeightDegenerate),
        VerdictClass::Disproven
    );
    assert_eq!(
        default_backing(RefusalKind::NearOverlap),
        VerdictClass::Disproven
    );
    assert_eq!(
        default_backing(RefusalKind::DeckExhausted),
        VerdictClass::Inconclusive
    );
}

#[test]
fn residual_implication_order_is_exactly_rule_c() {
    let ids = all_residual_ids();
    assert_eq!(ids.len(), 11, "the 11 residual ids");
    for &stronger in &ids {
        for &weaker in &ids {
            let expected = if stronger == weaker {
                Implication::Equivalent
            } else if stronger == ResidualId::R2 && weaker == ResidualId::R1 {
                Implication::Stronger
            } else {
                Implication::None
            };
            assert_eq!(
                implication(stronger, weaker),
                expected,
                "implication({stronger:?} over {weaker:?})"
            );
        }
    }
    assert_eq!(
        implication(ResidualId::R2, ResidualId::R1),
        Implication::Stronger
    );
    assert_eq!(
        implication(ResidualId::R6, ResidualId::R6),
        Implication::Equivalent
    );
    assert_eq!(
        implication(ResidualId::R8, ResidualId::R1),
        Implication::None
    );
    assert_eq!(
        implication(ResidualId::R9, ResidualId::R5),
        Implication::None
    );
    assert_eq!(
        implication(ResidualId::R7, ResidualId::R4),
        Implication::None
    );
}

#[test]
fn certified_positive_nonzero_refuse_bad_bounds() {
    assert!(CertifiedPositive::try_new(0.0).is_err(), "0 is not > 0");
    assert!(
        CertifiedPositive::try_new(-1.0).is_err(),
        "negative is not > 0"
    );
    assert!(
        CertifiedPositive::try_new(f64::NAN).is_err(),
        "NaN is not a positive bound"
    );
    assert!(
        CertifiedPositive::try_new(f64::INFINITY).is_err(),
        "infinity is not a finite positive bound"
    );
    let positive = construct(CertifiedPositive::try_new(2.5));
    assert_eq!(positive.value(), 2.5);

    assert!(CertifiedNonzero::try_new(0.0).is_err(), "0 is not nonzero");
    assert!(
        CertifiedNonzero::try_new(f64::NAN).is_err(),
        "NaN is not a nonzero bound"
    );
    let minus = construct(CertifiedNonzero::try_new(-3.0));
    assert_eq!(minus.value(), -3.0);
    let one = construct(CertifiedNonzero::try_new(1.0));
    assert_eq!(one.value(), 1.0);
}

#[test]
fn frame_refuses_non_orthonormal_basis() {
    let valid = valid_frame();
    assert_eq!(valid.q_tau, [1.0, 0.0, 0.0]);

    let z_hat = [0.0, 0.0, 1.0];
    let q_tau = [1.0, 0.0, 0.0];
    let a = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let q_perp = [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];

    // A column that is not unit length refuses.
    let bad_unit = [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(Frame::<3>::try_new(z_hat, bad_unit, q_tau, q_perp, a).is_err());

    // Unit columns that are not mutually orthogonal refuse.
    let not_orthogonal = [
        [1.0, 0.0, 0.0],
        [0.707_106_781_186_547_6, 0.707_106_781_186_547_6, 0.0],
        [0.0, 0.0, 1.0],
    ];
    assert!(Frame::<3>::try_new(z_hat, not_orthogonal, q_tau, q_perp, a).is_err());

    // A non-unit tangent refuses.
    assert!(Frame::<3>::try_new(
        z_hat,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [2.0, 0.0, 0.0],
        q_perp,
        a
    )
    .is_err());

    // A q_perp that is not the column-wise complement refuses.
    let wrong_complement = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(Frame::<3>::try_new(
        z_hat,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        q_tau,
        wrong_complement,
        a
    )
    .is_err());

    // z_hat is a point (§8.1): a non-unit z_hat is accepted with a valid
    // basis, and only a non-finite z_hat refuses.
    construct(Frame::<3>::try_new(
        [0.0, 0.0, 2.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        q_tau,
        q_perp,
        a,
    ));
    assert!(Frame::<3>::try_new(
        [0.0, f64::NAN, 2.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        q_tau,
        q_perp,
        a
    )
    .is_err());
}

#[test]
fn frame_try_new_accepts_nonunit_z_hat_and_still_gates_the_basis() {
    // A non-unit point (§8.1 expansion point) with a valid basis is accepted.
    let z_hat = [0.3, 0.7, 1.2, 0.5];
    let norm_sq = z_hat.iter().map(|c| c * c).sum::<f64>();
    assert!(
        (norm_sq - 1.0).abs() > config::TOL_JACOBIAN,
        "the probe z_hat must be non-unit"
    );
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
    let a = q;
    let frame = construct(Frame::<4>::try_new(z_hat, q, q_tau, q_perp, a));
    assert_eq!(frame.z_hat, z_hat);
    assert_eq!(frame.q_tau, [1.0, 0.0, 0.0, 0.0]);

    // The basis gates stand: a non-unit q_tau still refuses.
    assert!(Frame::<4>::try_new(z_hat, q, [2.0, 0.0, 0.0, 0.0], q_perp, a).is_err());

    // The point gate narrows to finiteness: a non-finite z_hat refuses.
    assert!(Frame::<4>::try_new([0.3, f64::INFINITY, 1.2, 0.5], q, q_tau, q_perp, a).is_err());
    assert!(Frame::<4>::try_new([0.3, f64::NAN, 1.2, 0.5], q, q_tau, q_perp, a).is_err());
}

#[test]
fn point_and_arc_cert_refuse_rho_above_max() {
    let box_ = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let point_ok = construct(PointCert::try_new(ResidualId::R1, box_, 0.3));
    assert_eq!(point_ok.rho, 0.3);
    assert!(PointCert::try_new(ResidualId::R1, box_, config::RHO_MAX + 0.01).is_err());
    assert!(PointCert::try_new(ResidualId::R1, box_, 0.9).is_err());

    let arc_ok = construct(valid_arc2(0.3));
    assert_eq!(arc_ok.rho, 0.3);
    let high = valid_arc2(0.9);
    assert!(
        high.is_err(),
        "rho above RHO_MAX refuses the tube certificate"
    );
    match high {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::Conditioning);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(_) => panic!("rho above RHO_MAX must refuse"),
    }

    // The load-bearing §8.3 ban: R2 is never an instance of the tube cert.
    let box2 = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let r2 = ArcCert::try_new(
        ResidualId::R2,
        valid_frame2(),
        CertifiedInterval::point(0.0),
        box2,
        0.2,
        vec![[-1.0, 1.0]],
        None,
    );
    match r2 {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::Conditioning);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
            match refusal.evidence {
                RefusalEvidence::Predicate { name, .. } => {
                    assert_eq!(name, "R2_never_reaches_C2");
                }
                _ => panic!("R2 refusal must carry the R2_never_reaches_C2 predicate"),
            }
        }
        Ok(_) => panic!("an R2 residual must never reach a tube certificate"),
    }
}

#[test]
fn contact_cert_requires_gap_at_tolerance() {
    let box_ = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let point = construct(PointCert::try_new(ResidualId::R1, box_, 0.2));
    let positive = truck_certified::kernel::SignCert::Positive;

    // Proven case: 0 in gap and width within TOL_INTERSECTION.
    let contained = CertifiedInterval {
        lo: 0.0,
        hi: config::TOL_INTERSECTION,
    };
    let cert = construct(ContactCert::try_new(point, contained, positive));
    assert_eq!(cert.tolerance, config::TOL_INTERSECTION);

    // Gap excluding zero is the refuted case.
    let excludes = CertifiedInterval { lo: 1.0, hi: 2.0 };
    match ContactCert::try_new(point, excludes, positive) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("a gap excluding 0 must not produce a proven contact cert"),
    }

    // A gap containing 0 but wider than the tolerance is inconclusive.
    let wide = CertifiedInterval {
        lo: -3.0 * config::TOL_INTERSECTION,
        hi: 3.0 * config::TOL_INTERSECTION,
    };
    assert!(wide.contains(0.0));
    match ContactCert::try_new(point, wide, positive) {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::R5EnclosureFailed);
            assert_eq!(refusal.backing, VerdictClass::Inconclusive);
        }
        Ok(_) => panic!("a gap wider than the tolerance must not certify proven contact"),
    }
}

#[test]
fn ibox_and_param_refuse_inverted_or_nonfinite() {
    let ok = construct(IBox2::try_new([-2.0, -2.0], [2.0, 2.0]));
    assert_eq!(ok.lo, [-2.0, -2.0]);
    assert!(
        IBox2::try_new([1.0, -2.0], [-1.0, 2.0]).is_err(),
        "inverted"
    );
    assert!(
        IBox2::try_new([-2.0, -2.0], [2.0, f64::NAN]).is_err(),
        "non-finite"
    );
    assert!(
        IBox2::try_new([f64::NEG_INFINITY, -2.0], [2.0, 2.0]).is_err(),
        "non-finite lower bound"
    );

    let param_ok = construct(Param::try_new(
        truck_certified::kernel::graph::ChartId(0),
        0,
        5.9,
        0.0,
    ));
    assert_eq!(param_ok.deck, 0);
    assert!(Param::try_new(truck_certified::kernel::graph::ChartId(0), 0, f64::NAN, 0.0).is_err());
    assert!(Param::try_new(
        truck_certified::kernel::graph::ChartId(0),
        0,
        5.9,
        f64::INFINITY
    )
    .is_err());
}

#[test]
fn topological_node_enums_have_no_refuse_variant() {
    let source = include_str!("../src/kernel/graph.rs");

    let topo = enum_variant_names(source, "TopoNode");
    let expected_topo = vec![
        "Boundary",
        "TrimCrossing",
        "MorseSaddle",
        "MorseExtremum",
        "A2Cusp",
        "OverlapBoundary",
        "FilletEnd",
    ];
    assert_eq!(
        topo, expected_topo,
        "TopoNode variants are exactly the §16 list"
    );
    assert!(
        !topo.iter().any(|name| name.contains("Refuse")),
        "Refuse must not appear in TopoNode"
    );

    let breaks = enum_variant_names(source, "SegmentBreak");
    let expected_breaks = vec![
        "ChartSwitch",
        "FrameSwitch",
        "LeafBoundary",
        "DeckStep",
        "R6ChartSwitch",
        "R6BaseSwap",
    ];
    assert_eq!(
        breaks, expected_breaks,
        "SegmentBreak variants are exactly the §16 list"
    );
    assert!(
        !breaks.iter().any(|name| name.contains("Refuse")),
        "Refuse must not appear in SegmentBreak"
    );

    // The two topology types exist as types with the expected payload shapes.
    fn _takes_node(_node: &truck_certified::kernel::graph::Node) {}
    fn _takes_break(_brk: &truck_certified::kernel::graph::Break) {}
    let _ = (TopoNode::Boundary, SegmentBreak::DeckStep);
}

#[test]
fn fixture_transversal_sphere_plane_ground_truth() {
    let fixture = construct(fx::transversal_sphere_plane());

    let expected_domain = construct(IBox2::try_new(
        [0.0, -std::f64::consts::FRAC_PI_2],
        [std::f64::consts::TAU, std::f64::consts::FRAC_PI_2],
    ));
    let expected_sphere = construct(truck_certified::kernel::leaf::RationalCarrier::try_new(
        truck_certified::kernel::leaf::RationalCarrierKind::Sphere,
        truck_certified::kernel::leaf::CarrierData::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        },
        expected_domain,
    ));
    let expected_plane = construct(truck_certified::kernel::leaf::RationalCarrier::try_new(
        truck_certified::kernel::leaf::RationalCarrierKind::Plane,
        truck_certified::kernel::leaf::CarrierData::Plane {
            origin: [0.0, 0.0, 0.0],
            u_dir: [1.0, 0.0, 0.0],
            v_dir: [0.0, 1.0, 0.0],
        },
        construct(IBox2::try_new([-2.0, -2.0], [2.0, 2.0])),
    ));
    assert_eq!(fixture.sphere, expected_sphere);
    assert_eq!(fixture.plane, expected_plane);

    // Sample the implicit forms at the circle and off it.
    let sphere_gap = |p: [f64; 3]| p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.0;
    let plane_gap = |p: [f64; 3]| p[2];
    let on_circle = [1.0, 0.0, 0.0];
    assert!(
        approx(sphere_gap(on_circle), 0.0),
        "circle point lies on the sphere"
    );
    assert!(
        approx(plane_gap(on_circle), 0.0),
        "circle point lies on z=0"
    );
    let plane_only = [0.5, 0.5, 0.0];
    assert!(plane_gap(plane_only).abs() < GT_TOL, "on the plane");
    assert!(sphere_gap(plane_only).abs() > GT_OFF, "off the sphere");
    let sphere_only = [0.0, 0.0, 1.0];
    assert!(sphere_gap(sphere_only).abs() < GT_TOL, "on the sphere");
    assert!(plane_gap(sphere_only).abs() > GT_OFF, "off the plane");
}

#[test]
fn fixture_coaxial_cylinders_sheet_ground_truth() {
    let fixture = construct(fx::coaxial_cylinders());
    assert!(approx(fixture.normal_dot, 1.0));

    // The consistent co-oriented pair certifies the identity sheet.
    let sheet = construct(fx::coaxial_cylinder_sheet(&fixture.first, &fixture.second));
    assert_eq!(sheet.psi_kind, PsiMapKind::Identity);
    assert!(approx(sheet.det_dpsi.value(), 1.0));

    // The anti-parallel twin refuses the identity certificate.
    let flipped = fx::coaxial_cylinder_sheet(&fixture.first, &fixture.anti_parallel);
    assert!(
        flipped.is_err(),
        "anti-parallel normals refuse the identity exact-sheet certificate"
    );
    match flipped {
        Err(refusal) => {
            assert_eq!(refusal.kind, RefusalKind::ClaimRefuted);
            assert_eq!(refusal.backing, VerdictClass::Disproven);
        }
        Ok(_) => panic!("anti-parallel normals must refuse"),
    }
}

#[test]
fn fixture_determinant_spans_zero_is_inconclusive_backed() {
    let fixture = construct(fx::determinant_spans_zero());
    assert!(
        fixture.det_lo <= 0.0 && 0.0 <= fixture.det_hi,
        "the det DF enclosure contains 0"
    );
    // det DF = 2x changes sign across the domain.
    assert!(2.0 * -2.0 < 0.0 && 2.0 * 2.0 > 0.0, "det spans zero");
    assert_eq!(fixture.refusal.backing, VerdictClass::Inconclusive);
    match &fixture.refusal.evidence {
        RefusalEvidence::Residual {
            residual,
            box_,
            note,
        } => {
            assert_eq!(*residual, ResidualId::R5);
            assert_eq!(*box_, fixture.domain);
            assert_eq!(
                *note,
                "det DF = 2x spans zero over the box: C1 not certifiable"
            );
        }
        _ => panic!("determinant refusal must carry residual evidence over the box"),
    }
}

#[test]
fn fixture_weight_straddles_zero_is_weight_degenerate() {
    let fixture = construct(fx::weight_straddles_zero());
    // w(0.5) == 0 exactly: the interval-style enclosure contains 0 and the
    // positive bound construction refuses WeightDegenerate, Disproven.
    assert!(approx(fixture.w_at_half, 0.0));
    assert!(fixture.hull_lo <= 0.0 && 0.0 <= fixture.hull_hi);
    assert_eq!(
        fixture.refusal_disproven.kind,
        RefusalKind::WeightDegenerate
    );
    assert_eq!(fixture.refusal_disproven.backing, VerdictClass::Disproven);
    // The §7.1 pair: the straddle is re-classed Inconclusive.
    assert_eq!(
        fixture.refusal_inconclusive.kind,
        RefusalKind::WeightDegenerate
    );
    assert_eq!(
        fixture.refusal_inconclusive.backing,
        VerdictClass::Inconclusive
    );
    // A shifted box where w > 0 certifies fine.
    assert!(fixture.shifted.value() > 0.0);
    assert!(fixture.shifted.value() <= weight_on(fixture.weights, 0.7));
    assert!(fixture.shifted.value() <= weight_on(fixture.weights, 1.0));
    for t in [0.6, 0.7, 0.8, 0.9, 1.0] {
        assert!(
            weight_on(fixture.weights, t) > 0.0,
            "the shifted box keeps the weight strictly positive at t={t}"
        );
    }
}

#[test]
fn fixture_deck_wrap_displacement_is_one() {
    let fixture = construct(fx::deck_wrap());
    assert!(approx(fixture.period, std::f64::consts::TAU));
    assert!(approx(fixture.canonical_end_u, 6.4 - std::f64::consts::TAU));
    assert_eq!(fixture.displacement, 1);
    assert_eq!(fixture.end.deck - fixture.start.deck, fixture.displacement);
    assert!(
        fixture.start.u < fixture.period && fixture.end.u < fixture.period,
        "both stored u values are canonical"
    );
    let raw_start = fixture.start.u + fixture.period * fixture.start.deck as f64;
    let raw_end = fixture.end.u + fixture.period * fixture.end.deck as f64;
    assert!(approx(raw_end, 6.4), "raw end recovers 6.4");
    // Deck displacement is the exact integer from floor-div on the crossing
    // count.
    let crossings = (raw_end / fixture.period).floor() - (raw_start / fixture.period).floor();
    assert_eq!(crossings, 1.0, "exactly one seam crossing");
    assert_eq!(crossings as i32, fixture.displacement);
}

#[test]
fn fixture_c1_discontinuity_tangent_jump_exceeds_tolerance() {
    let fixture = fx::c1_discontinuity();
    assert_eq!(fixture.positions.len(), 3);
    let dot = fixture.tangent0[0] * fixture.tangent1[0]
        + fixture.tangent0[1] * fixture.tangent1[1]
        + fixture.tangent0[2] * fixture.tangent1[2];
    assert!(
        approx(dot, 0.0),
        "the tangent directions jump by 90 degrees"
    );
    // The jump measure (1 - dot) far exceeds the C1 detection tolerance.
    assert!(1.0 - dot > config::TOL_PARAMETER);
    // The unit tangents are exactly the two segment directions.
    assert!(approx(fixture.tangent0[0], 1.0));
    assert!(approx(fixture.tangent1[1], 1.0));
}
