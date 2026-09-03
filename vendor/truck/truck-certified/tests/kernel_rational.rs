//! The rational-carrier `CertifiedPatch` tests (BG-KV2-104-RATCARRIER): the
//! seven required machine-checked ground truths for the Plane/Sphere/Cylinder
//! rational half-angle carriers.
//!
//! Everything is a direct evaluation of the closed forms over the rational
//! charts — no solver is invoked. The source-scan test reads `rational.rs`
//! itself to pin the N4 guarantee.

#![deny(clippy::unwrap_used)]

use truck_certified::kernel::evidence::{
    ClaimVerdict, Refusal as KernelRefusal, RefusalEvidence, RefusalKind, VerdictClass,
};
use truck_certified::kernel::leaf::{CarrierData, RationalCarrier, RationalCarrierKind};
use truck_certified::kernel::patch::{CertifiedPatch, IBox2, IBox3};
use truck_certified::kernel::rational;

/// The pointwise implicit-form comparison tolerance (H-3).
const GT_TOL: f64 = 1e-12; // H-3: dyadic pointwise implicit-form comparison tolerance
/// The certified-enclosure containment slack (H-3): certified enclosures are
/// outward-rounded, so a sample point never lands more than this far outside.
const ENCLOSURE_SLACK: f64 = 1e-9; // H-3: certified-enclosure containment slack
/// The weight lower-bound rounding slack (H-3): the certified lower bound of
/// `1 + u² + v²` on a zero-containing box is 1 within outward rounding.
const WEIGHT_SLACK: f64 = 1e-9; // H-3: weight lower-bound outward-rounding slack

/// Assert two floats agree to the pointwise ground-truth tolerance.
fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= GT_TOL
}

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, KernelRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// A unit sphere carrier, chart box `[-1, 1]²` (avoids the degeneration).
fn sphere() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Sphere,
        CarrierData::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// A radius-`1.5` cylinder on the exact `z` axis.
fn cylinder() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Cylinder,
        CarrierData::Cylinder {
            origin: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            radius: 1.5,
            height: (0.0, 1.0),
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// A plane carrier through the origin spanned by the `x` and `y` axes.
fn plane() -> RationalCarrier {
    construct(RationalCarrier::try_new(
        RationalCarrierKind::Plane,
        CarrierData::Plane {
            origin: [0.0, 0.0, 0.0],
            u_dir: [1.0, 0.0, 0.0],
            v_dir: [0.0, 1.0, 0.0],
        },
        construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0])),
    ))
}

/// The pointwise rational sphere image `(2u, 2v, 1 − u² − v²)/(1 + u² + v²)`.
fn sphere_point(u: f64, v: f64) -> [f64; 3] {
    let d = 1.0 + u * u + v * v;
    [2.0 * u / d, 2.0 * v / d, (1.0 - u * u - v * v) / d]
}

/// The pointwise rational cylinder image on the `z` axis at radius `1.5`:
/// `(1.5(1 − u²), 3u, v(1 + u²))/(1 + u²)`.
fn cylinder_point(u: f64, v: f64) -> [f64; 3] {
    let w = 1.0 + u * u;
    let cx = (1.0 - u * u) / w;
    let sx = 2.0 * u / w;
    [1.5 * cx, 1.5 * sx, v]
}

/// The pointwise rational plane image.
fn plane_point(u: f64, v: f64) -> [f64; 3] {
    [u, v, 0.0]
}

/// Whether every coordinate of `p` lies in the certified enclosure `box_`,
/// to the outward-rounding containment slack.
fn enclosed(box_: IBox3, p: [f64; 3]) -> bool {
    for k in 0..3 {
        if p[k] < box_.lo[k] - ENCLOSURE_SLACK || p[k] > box_.hi[k] + ENCLOSURE_SLACK {
            return false;
        }
    }
    true
}

#[test]
fn sphere_rational_param_matches_implicit_form_on_grid() {
    let _ = sphere();
    let mut count = 0u32;
    for i in 0..=8u32 {
        for j in 0..=8u32 {
            let u = -1.0 + i as f64 * 0.25;
            let v = -1.0 + j as f64 * 0.25;
            let p = sphere_point(u, v);
            let gap = p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.0;
            assert!(approx(gap, 0.0), "sphere implicit gap {gap} at ({u}, {v})");
            count += 1;
        }
    }
    assert_eq!(count, 81);
    // Pointwise identity checks on the closed form.
    assert!(approx(sphere_point(0.0, 0.0)[0], 0.0));
    assert!(approx(sphere_point(0.0, 0.0)[1], 0.0));
    assert!(approx(sphere_point(0.0, 0.0)[2], 1.0));
    assert!(approx(sphere_point(1.0, 0.0)[0], 1.0));
    assert!(approx(sphere_point(1.0, 0.0)[2], 0.0));
    assert!(approx(sphere_point(0.0, 1.0)[1], 1.0));
    assert!(approx(sphere_point(-1.0, 0.0)[0], -1.0));
}

#[test]
fn cylinder_rational_param_matches_implicit_form_on_grid() {
    let _ = cylinder();
    let mut count = 0u32;
    for i in 0..=8u32 {
        for j in 0..=8u32 {
            let u = -1.0 + i as f64 * 0.25;
            let v = -1.0 + j as f64 * 0.25;
            let p = cylinder_point(u, v);
            let radial = p[0] * p[0] + p[1] * p[1] - 2.25;
            assert!(
                approx(radial, 0.0),
                "cylinder implicit gap {radial} at ({u}, {v})"
            );
            assert!(approx(p[2], v), "axial coordinate is linear in v");
            count += 1;
        }
    }
    assert_eq!(count, 81);
}

#[test]
fn enclosures_contain_sampled_points_all_three_carriers() {
    let plane_box = construct(IBox2::try_new([-0.75, -0.75], [0.75, 0.75]));
    let plane_enclosure = plane().enclose(plane_box);
    for i in -2..=2i32 {
        for j in -2..=2i32 {
            let u = i as f64 * 0.25;
            let v = j as f64 * 0.25;
            assert!(
                enclosed(plane_enclosure, plane_point(u, v)),
                "plane enclosure misses ({u}, {v})"
            );
        }
    }

    let sphere_box = construct(IBox2::try_new([-0.5, -0.5], [0.5, 0.5]));
    let sphere_enclosure = sphere().enclose(sphere_box);
    for i in -2..=2i32 {
        for j in -2..=2i32 {
            let u = i as f64 * 0.25;
            let v = j as f64 * 0.25;
            assert!(
                enclosed(sphere_enclosure, sphere_point(u, v)),
                "sphere enclosure misses ({u}, {v})"
            );
        }
    }

    let cylinder_box = construct(IBox2::try_new([-0.5, -0.5], [0.5, 0.5]));
    let cylinder_enclosure = cylinder().enclose(cylinder_box);
    for i in -2..=2i32 {
        for j in -2..=2i32 {
            let u = i as f64 * 0.25;
            let v = j as f64 * 0.25;
            assert!(
                enclosed(cylinder_enclosure, cylinder_point(u, v)),
                "cylinder enclosure misses ({u}, {v})"
            );
        }
    }
}

#[test]
fn regularity_proven_away_from_the_rational_degeneration() {
    let sphere_carrier = sphere();
    // The full chart box of a sphere chart that avoids the degeneration.
    let chart_box = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    match sphere_carrier.regularity(chart_box) {
        ClaimVerdict::Proven(bound) => {
            assert!(
                bound.value() > 0.0,
                "regularity lower bound {} must be positive",
                bound.value()
            );
        }
        _ => panic!("sphere regularity must be Proven on the full chart box"),
    }
    // A box that reaches the chart degeneration (coordinates so large the
    // certified arithmetic is no longer finite) refuses the Proven claim.
    let degeneration_box = construct(IBox2::try_new([-1e200, -1e200], [1e200, 1e200]));
    match sphere_carrier.regularity(degeneration_box) {
        ClaimVerdict::Proven(_) => panic!("the degeneration-point box must refuse Proven"),
        ClaimVerdict::Disproven(witness) => assert_eq!(witness.box_, degeneration_box),
        ClaimVerdict::Inconclusive(_) => {}
    }
}

#[test]
fn weight_bound_proven_denominator_separated_from_zero() {
    let plane_carrier = plane();
    let sphere_carrier = sphere();
    let cylinder_carrier = cylinder();
    let plane_box = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let sphere_box = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let cylinder_box = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let cases = [
        (&plane_carrier, plane_box),
        (&sphere_carrier, sphere_box),
        (&cylinder_carrier, cylinder_box),
    ];
    for (carrier, box_) in cases {
        match carrier.weight_bound(box_) {
            Some(ClaimVerdict::Proven(bound)) => {
                assert!(
                    bound.value() > 0.0,
                    "the certified weight bound must be strictly positive"
                );
                assert!(
                    bound.value() >= 1.0 - WEIGHT_SLACK,
                    "the chart denominators are bounded below by 1 by construction"
                );
            }
            other => panic!("weight_bound must be Proven on a chart box: {other:?}"),
        }
    }
    // The sphere form at u = v = 0: the denominator lower bound is exactly 1
    // (outward rounding keeps the certified bound within an ulp of it).
    let origin_box = construct(IBox2::try_new([0.0, 0.0], [0.0, 0.0]));
    match sphere_carrier.weight_bound(origin_box) {
        Some(ClaimVerdict::Proven(bound)) => {
            assert!(
                bound.value() >= 1.0 - WEIGHT_SLACK,
                "sphere denominator at u=v=0 is bounded below by 1"
            );
        }
        other => panic!("sphere weight_bound must be Proven at u=v=0: {other:?}"),
    }
}

#[test]
fn no_transcendental_call_in_rational_module() {
    let source = include_str!("../src/kernel/rational.rs");
    let banned = ["sin", "cos", "atan2", "exp", "ln", "log", "powf", "sqrt"];
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    for (line_no, raw) in source.lines().enumerate() {
        let code = match raw.find("//") {
            Some(index) => &raw[..index],
            None => raw,
        };
        for token in banned {
            for (at, _) in code.match_indices(token) {
                let after = at + token.len();
                let left_clear = code[..at].chars().next_back().is_none_or(|c| !is_word(c));
                let right_clear = code[after..].chars().next().is_none_or(|c| !is_word(c));
                assert!(
                    !(left_clear && right_clear),
                    "line {} carries the transcendental call token {token}: {code}",
                    line_no + 1
                );
            }
        }
    }
}

#[test]
fn cone_variant_refuses_pending_its_packet() {
    let domain = construct(IBox2::try_new([-1.0, -1.0], [1.0, 1.0]));
    let pending = [
        construct(RationalCarrier::try_new(
            RationalCarrierKind::Cone,
            CarrierData::Cone {
                apex: [0.0, 0.0, 0.0],
                axis: [0.0, 0.0, 1.0],
                half_angle: 0.5,
                height: (0.0, 1.0),
            },
            domain,
        )),
        construct(RationalCarrier::try_new(
            RationalCarrierKind::Torus,
            CarrierData::Torus {
                center: [0.0, 0.0, 0.0],
                axis: [0.0, 0.0, 1.0],
                major_r: 2.0,
                minor_r: 0.5,
            },
            domain,
        )),
    ];
    for carrier in pending {
        match rational::admit(&carrier) {
            Err(refusal) => {
                assert_eq!(refusal.kind, RefusalKind::CarrierSingularity);
                assert_eq!(refusal.backing, VerdictClass::Disproven);
                match refusal.evidence {
                    RefusalEvidence::Predicate { name, .. } => {
                        assert_eq!(name, "cone_torus_carrier_packet_pending");
                    }
                    _ => panic!("the pending refusal must carry the named predicate"),
                }
            }
            Ok(()) => panic!("a Cone/Torus carrier must refuse the Wave-1 admission"),
        }
        let box_ = carrier.domain;
        match carrier.regularity(box_) {
            ClaimVerdict::Inconclusive(reason) => {
                assert_eq!(reason, "cone_torus_carrier_packet_pending");
            }
            _ => panic!("a Cone/Torus carrier must not certify regularity"),
        }
        match carrier.weight_bound(box_) {
            Some(ClaimVerdict::Inconclusive(reason)) => {
                assert_eq!(reason, "cone_torus_carrier_packet_pending");
            }
            _ => panic!("a Cone/Torus carrier must not certify a weight bound"),
        }
    }
}
