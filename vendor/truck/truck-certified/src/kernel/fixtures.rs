#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The kernel-v2 fixture kit: six machine-checked ground truths
//! (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **This module is `#[doc(hidden)] pub`: TEST SUPPORT ONLY, explicitly
//! excluded from the certified API surface (the BG-CK-P2-CONTRACT rule), but
//! reachable by wave workers' integration tests through the crate's public
//! path.**
//!
//! Each fixture is a `pub fn` returning constructed shim types plus a
//! doc-stated NUMERIC ground truth; a `#[cfg(test)]` test machine-checks it by
//! direct evaluation — never by solving. Fixture ground truths that would
//! require solving are out of scope by construction (the list is frozen; see
//! the packet stop conditions).
//!
//! 1. `transversal_sphere_plane`: sphere center `(0,0,0)` r=1 + plane `z=0`;
//!    ground truth: intersection circle radius 1 at `z=0`.
//! 2. `coaxial_cylinders`: two r=1 cylinders on the z-axis; ground truth:
//!    ExactSheet candidate, `psi = Identity`, normal dot identically `+1`.
//! 3. `determinant_spans_zero`: residual `F = (x^2-1, y)` on `[-2,2]^2`;
//!    `det DF = 2x` spans zero; ground truth: the box is NOT certifiable by C1
//!    — refused with backing Inconclusive.
//! 4. `weight_straddles_zero`: homogeneous quadratic weights `(1,-1,1)`;
//!    `w(t) = 1 - 4t(1-t)`; `w(0.5) = 0` exactly; ground truth: the
//!    interval-style enclosure over `[0,1]` contains 0, so the
//!    `CertifiedPositive` bound construction refuses `WeightDegenerate` with
//!    backing Disproven, while a shifted box where `w > 0` certifies fine.
//! 5. `deck_wrap`: pcurve parameter run `5.9 -> 6.4` over period `2*PI`;
//!    ground truth: deck displacement `+1` (exact integer from floor-div on
//!    the crossing count).
//! 6. `c1_discontinuity`: polyline `[(0,0,0),(1,0,0),(1,1,0)]`; ground truth:
//!    tangent direction jumps 90 degrees between the segments (dot of unit
//!    tangents `0`, `> TOL_PARAMETER` discontinuity). Carried as DATA — the C1
//!    wave packet owns the `SpineNotC1` wiring.

use crate::formal::exact::CertifiedInterval;
use crate::kernel::certs::{PsiMapKind, SheetCert};
use crate::kernel::evidence::{Refusal, RefusalEvidence, RefusalKind, VerdictClass};
use crate::kernel::graph::{ChartId, Param};
use crate::kernel::leaf::{CarrierData, RationalCarrier, RationalCarrierKind};
use crate::kernel::patch::{CertifiedPositive, IBox2};
use crate::kernel::residual::ResidualId;

/// The stored period of the deck fixture: `2*PI`, kept as data (never
/// recomputed as a transcendental by the fixtures or their tests).
const PERIOD: f64 = 6.283_185_307_179_586;

/// The canonical unwrap of the deck fixture's raw end parameter
/// `6.4 - 2*PI`, kept as data.
const CANONICAL_END_U: f64 = 0.116_814_692_820_414_12;

/// The ground-truth circle sampled by the sphere/plane fixture: unit circle at
/// `z = 0`.
#[derive(Debug, Clone)]
pub struct SpherePlaneFixture {
    /// The unit sphere carrier, center `(0,0,0)`.
    pub sphere: RationalCarrier,
    /// The plane carrier `z = 0`.
    pub plane: RationalCarrier,
}

/// The coincident-cylinder sheet fixture.
#[derive(Debug, Clone)]
pub struct CoaxialCylinderFixture {
    /// The first unit cylinder on the z-axis (outward orientation).
    pub first: RationalCarrier,
    /// The second, co-oriented unit cylinder on the z-axis.
    pub second: RationalCarrier,
    /// A twin with an anti-parallel axis (reversed orientation).
    pub anti_parallel: RationalCarrier,
    /// The constant-sign fact: oriented normals of `first` and `second` agree
    /// identically, `dot = +1` (input data, not a solved certificate).
    pub normal_dot: f64,
}

/// The determinant-spans-zero fixture.
#[derive(Debug, Clone)]
pub struct DeterminantFixture {
    /// The residual's domain box `[-2,2]^2`.
    pub domain: IBox2,
    /// The interval-style enclosure of `det DF = 2x` over the domain;
    /// contains 0.
    pub det_lo: f64,
    /// The interval-style enclosure of `det DF = 2x` over the domain;
    /// contains 0.
    pub det_hi: f64,
    /// The C1 certification refusal over the box: backing Inconclusive.
    pub refusal: Refusal,
}

/// The weight-straddles-zero fixture.
#[derive(Debug, Clone)]
pub struct WeightFixture {
    /// The homogeneous quadratic weights `(1, -1, 1)`.
    pub weights: [f64; 3],
    /// `w(t) = 1 - 4t(1-t)`, the weight polynomial of the homogeneous
    /// quadratic.
    pub w_at_half: f64,
    /// The interval-style enclosure lower bound of `w` over `[0,1]`.
    pub hull_lo: f64,
    /// The interval-style enclosure upper bound of `w` over `[0,1]`.
    pub hull_hi: f64,
    /// The §7.1 Disproven member: `WeightDegenerate`, backing Disproven.
    pub refusal_disproven: Refusal,
    /// The §7.1 Inconclusive member: `WeightDegenerate`, backing Inconclusive.
    pub refusal_inconclusive: Refusal,
    /// The shifted box `[0.6, 1.0]` where the weight is strictly positive.
    pub shifted_box: (f64, f64),
    /// The certified positive lower bound over the shifted box.
    pub shifted: CertifiedPositive,
}

/// The deck-wrap fixture.
#[derive(Debug, Clone)]
pub struct DeckWrapFixture {
    /// The chart period `2*PI`, stored as data.
    pub period: f64,
    /// The start parameter: deck 0, canonical `u = 5.9`.
    pub start: Param,
    /// The end parameter: deck 1, canonical `u = 6.4 - 2*PI`.
    pub end: Param,
    /// The canonical unwrap of raw `6.4` (`6.4 - 2*PI`), stored as data.
    pub canonical_end_u: f64,
    /// The ground-truth deck displacement: `+1`.
    pub displacement: i32,
}

/// The C1-discontinuity polyline fixture.
#[derive(Debug, Clone)]
pub struct C1DiscontinuityFixture {
    /// The polyline positions `[(0,0,0),(1,0,0),(1,1,0)]`.
    pub positions: Vec<[f64; 3]>,
    /// The unit tangent of the first segment `(1,0,0)`.
    pub tangent0: [f64; 3],
    /// The unit tangent of the second segment `(0,1,0)`.
    pub tangent1: [f64; 3],
}

/// Fixture 1: the transversal sphere/plane pair.
pub fn transversal_sphere_plane() -> Result<SpherePlaneFixture, Refusal> {
    let theta_domain = IBox2::try_new(
        [0.0, -std::f64::consts::FRAC_PI_2],
        [PERIOD, std::f64::consts::FRAC_PI_2],
    )?;
    let sphere = RationalCarrier::try_new(
        RationalCarrierKind::Sphere,
        CarrierData::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        },
        theta_domain,
    )?;
    let plane_domain = IBox2::try_new([-2.0, -2.0], [2.0, 2.0])?;
    let plane = RationalCarrier::try_new(
        RationalCarrierKind::Plane,
        CarrierData::Plane {
            origin: [0.0, 0.0, 0.0],
            u_dir: [1.0, 0.0, 0.0],
            v_dir: [0.0, 1.0, 0.0],
        },
        plane_domain,
    )?;
    Ok(SpherePlaneFixture { sphere, plane })
}

/// Fixture 2: two coincident unit cylinders on the z-axis.
pub fn coaxial_cylinders() -> Result<CoaxialCylinderFixture, Refusal> {
    let first = cylinder_carrier([0.0, 0.0, 1.0])?;
    let second = cylinder_carrier([0.0, 0.0, 1.0])?;
    let anti_parallel = cylinder_carrier([0.0, 0.0, -1.0])?;
    Ok(CoaxialCylinderFixture {
        first,
        second,
        anti_parallel,
        normal_dot: 1.0,
    })
}

/// Construct the identity [`SheetCert`] for two coincident unit cylinders on
/// the z-axis from their carrier data fields (fixture-scoped helper).
///
/// No solver: the orientation SIGN is input data. The certificate accepts the
/// consistent, co-oriented pair and refuses an anti-parallel (flipped) twin,
/// because the identity correspondence between coincident cylinders is only
/// certifiable when the oriented normals agree.
pub fn coaxial_cylinder_sheet(
    first: &RationalCarrier,
    second: &RationalCarrier,
) -> Result<SheetCert, Refusal> {
    let first_axis = cylinder_axis(first)?;
    let second_axis = cylinder_axis(second)?;
    let axis_dot = first_axis[0] * second_axis[0]
        + first_axis[1] * second_axis[1]
        + first_axis[2] * second_axis[2];
    if !(axis_dot > 1.0 - crate::kernel::config::EPS_REP) {
        return Err(Refusal::new(
            RefusalKind::ClaimRefuted,
            RefusalEvidence::Predicate {
                name: "exact_sheet_axis_orientation_inconsistent",
                detail: format!(
                    "identity exact-sheet certificate requires co-oriented coincident \
                     cylinders; axis dot is {axis_dot}"
                ),
            },
        ));
    }
    SheetCert::try_new(first.domain, PsiMapKind::Identity, 1.0)
}

/// Fixture 3: residual `F = (x^2-1, y)` on `[-2,2]^2` whose `det DF = 2x`
/// spans zero.
pub fn determinant_spans_zero() -> Result<DeterminantFixture, Refusal> {
    let domain = IBox2::try_new([-2.0, -2.0], [2.0, 2.0])?;
    let x = CertifiedInterval { lo: -2.0, hi: 2.0 };
    let two = CertifiedInterval::point(2.0);
    let det = two.mul(&x);
    let refusal = Refusal::new(
        RefusalKind::R5EnclosureFailed,
        RefusalEvidence::Residual {
            residual: ResidualId::R5,
            box_: domain,
            note: "det DF = 2x spans zero over the box: C1 not certifiable",
        },
    );
    Ok(DeterminantFixture {
        domain,
        det_lo: det.lo,
        det_hi: det.hi,
        refusal,
    })
}

/// Fixture 4: the homogeneous quadratic weights `(1,-1,1)` with
/// `w(t) = 1 - 4t(1-t)`.
pub fn weight_straddles_zero() -> Result<WeightFixture, Refusal> {
    let weights = [1.0, -1.0, 1.0];
    let w_at_half = weight_on(weights, 0.5);
    let one = CertifiedInterval::point(1.0);
    let four = CertifiedInterval::point(4.0);
    let t = CertifiedInterval { lo: 0.0, hi: 1.0 };
    let one_minus_t = CertifiedInterval { lo: 0.0, hi: 1.0 };
    let enclosure = one.sub(&four.mul(&t.mul(&one_minus_t)));
    let refusal_disproven = match CertifiedPositive::try_new(w_at_half) {
        Err(refusal) => refusal,
        Ok(_) => {
            return Err(Refusal::new(
                RefusalKind::WeightDegenerate,
                RefusalEvidence::Predicate {
                    name: "weight_fixture_not_degenerate",
                    detail: "w(0.5) must be exactly 0 for the straddle fixture".to_string(),
                },
            ))
        }
    };
    let refusal_inconclusive = Refusal::with_backing(
        RefusalKind::WeightDegenerate,
        VerdictClass::Inconclusive,
        RefusalEvidence::Predicate {
            name: "weight_straddle_inconclusive",
            detail: "the interval-style enclosure straddles zero: sign not decidable".to_string(),
        },
    );
    let shifted_box = (0.6, 1.0);
    let shifted = match CertifiedPositive::try_new(weight_on(weights, shifted_box.0)) {
        Ok(cert) => cert,
        Err(_) => {
            return Err(Refusal::new(
                RefusalKind::WeightDegenerate,
                RefusalEvidence::Predicate {
                    name: "weight_shifted_box_not_positive",
                    detail: "w on [0.6, 1.0] must certify strictly positive".to_string(),
                },
            ))
        }
    };
    Ok(WeightFixture {
        weights,
        w_at_half,
        hull_lo: enclosure.lo,
        hull_hi: enclosure.hi,
        refusal_disproven,
        refusal_inconclusive,
        shifted_box,
        shifted,
    })
}

/// Fixture 5: the pcurve deck wrap over a period-`2*PI` chart.
pub fn deck_wrap() -> Result<DeckWrapFixture, Refusal> {
    let chart = ChartId(0);
    let start = Param::try_new(chart, 0, 5.9, 0.0)?;
    let end = Param::try_new(chart, 1, CANONICAL_END_U, 0.0)?;
    Ok(DeckWrapFixture {
        period: PERIOD,
        start,
        end,
        canonical_end_u: CANONICAL_END_U,
        displacement: 1,
    })
}

/// Fixture 6: the 90-degree tangent-jump polyline.
pub fn c1_discontinuity() -> C1DiscontinuityFixture {
    let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]];
    let tangent0 = unit_tangent(positions[0], positions[1]);
    let tangent1 = unit_tangent(positions[1], positions[2]);
    C1DiscontinuityFixture {
        positions,
        tangent0,
        tangent1,
    }
}

fn cylinder_carrier(axis: [f64; 3]) -> Result<RationalCarrier, Refusal> {
    let domain = IBox2::try_new([0.0, 0.0], [PERIOD, 1.0])?;
    RationalCarrier::try_new(
        RationalCarrierKind::Cylinder,
        CarrierData::Cylinder {
            origin: [0.0, 0.0, 0.0],
            axis,
            radius: 1.0,
            height: (0.0, 1.0),
        },
        domain,
    )
}

fn cylinder_axis(carrier: &RationalCarrier) -> Result<[f64; 3], Refusal> {
    match carrier.data {
        CarrierData::Cylinder { axis, .. } => Ok(axis),
        _ => Err(Refusal::new(
            RefusalKind::ClaimRefuted,
            RefusalEvidence::Predicate {
                name: "exact_sheet_requires_cylinders",
                detail: "coaxial sheet fixture consumes two cylinder carriers".to_string(),
            },
        )),
    }
}

/// The weight polynomial of the homogeneous quadratic weights `(w0,w1,w2)` in
/// Bernstein form: `(1-t)^2 w0 + 2t(1-t) w1 + t^2 w2`, which for `(1,-1,1)`
/// is `1 - 4t(1-t)`.
fn weight_on(weights: [f64; 3], t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * weights[0] + 2.0 * t * mt * weights[1] + t * t * weights[2]
}

fn unit_tangent(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    [d[0] / len, d[1] / len, d[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::config::TOL_PARAMETER;
    use crate::kernel::evidence::VerdictClass;

    /// The fixture ground-truth agreement tolerance.
    const GT_TOL: f64 = 1e-12; // H-3: dyadic fixture ground-truth comparison tolerance
    /// The threshold for "clearly off the circle".
    const GT_OFF: f64 = 1e-3; // H-3: fixture off-circle separation threshold

    /// Extract an `Ok` construction or fail the test (early return).
    macro_rules! ok_or_fail {
        ($result:expr) => {{
            let result = $result;
            let succeeded = result.is_ok();
            match result {
                Ok(value) => value,
                Err(refusal) => {
                    assert!(succeeded, "fixture construction refused: {refusal:?}");
                    return;
                }
            }
        }};
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() <= GT_TOL
    }

    #[test]
    fn sphere_plane_circle_ground_truth() {
        let fx = ok_or_fail!(transversal_sphere_plane());
        let expected_sphere_domain = ok_or_fail!(IBox2::try_new(
            [0.0, -std::f64::consts::FRAC_PI_2],
            [PERIOD, std::f64::consts::FRAC_PI_2],
        ));
        let expected_sphere = ok_or_fail!(RationalCarrier::try_new(
            RationalCarrierKind::Sphere,
            CarrierData::Sphere {
                center: [0.0, 0.0, 0.0],
                radius: 1.0,
            },
            expected_sphere_domain,
        ));
        let expected_plane = ok_or_fail!(RationalCarrier::try_new(
            RationalCarrierKind::Plane,
            CarrierData::Plane {
                origin: [0.0, 0.0, 0.0],
                u_dir: [1.0, 0.0, 0.0],
                v_dir: [0.0, 1.0, 0.0],
            },
            ok_or_fail!(IBox2::try_new([-2.0, -2.0], [2.0, 2.0])),
        ));
        assert_eq!(fx.sphere, expected_sphere);
        assert_eq!(fx.plane, expected_plane);

        let sphere_gap = |p: [f64; 3]| p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.0;
        let plane_gap = |p: [f64; 3]| p[2];
        let on_circle = [1.0, 0.0, 0.0];
        assert!(
            approx(sphere_gap(on_circle), 0.0),
            "on-circle point is on the sphere, gap {}",
            sphere_gap(on_circle)
        );
        assert!(
            approx(plane_gap(on_circle), 0.0),
            "on-circle point is on the plane z=0"
        );
        let plane_only = [0.5, 0.5, 0.0];
        assert!(plane_gap(plane_only).abs() <= GT_TOL, "on the plane");
        assert!(
            sphere_gap(plane_only).abs() > GT_OFF,
            "off the sphere: gap {}",
            sphere_gap(plane_only)
        );
        let sphere_only = [0.0, 0.0, 1.0];
        assert!(sphere_gap(sphere_only).abs() <= GT_TOL, "on the sphere");
        assert!(
            plane_gap(sphere_only).abs() > GT_OFF,
            "off the plane z=0: z {}",
            plane_gap(sphere_only)
        );
    }

    #[test]
    fn coaxial_cylinders_identity_sheet_sign() {
        let fx = ok_or_fail!(coaxial_cylinders());
        assert!(approx(fx.normal_dot, 1.0));
        let sheet = ok_or_fail!(coaxial_cylinder_sheet(&fx.first, &fx.second));
        assert_eq!(sheet.psi_kind, PsiMapKind::Identity);
        assert!(approx(sheet.det_dpsi.value(), 1.0));
        let result = coaxial_cylinder_sheet(&fx.first, &fx.anti_parallel);
        assert!(
            result.is_err(),
            "anti-parallel twin must refuse the identity exact-sheet certificate"
        );
        let refused = match result {
            Err(refusal) => refusal,
            Ok(_) => {
                assert!(result.is_err());
                return;
            }
        };
        assert_eq!(refused.kind, RefusalKind::ClaimRefuted);
        assert_eq!(refused.backing, VerdictClass::Disproven);
    }

    #[test]
    fn determinant_spans_zero_is_inconclusive_backed() {
        let fx = ok_or_fail!(determinant_spans_zero());
        assert!(fx.det_lo <= 0.0 && 0.0 <= fx.det_hi, "enclosure contains 0");
        assert_eq!(fx.refusal.backing, VerdictClass::Inconclusive);
        let evidence_is_residual = matches!(&fx.refusal.evidence, RefusalEvidence::Residual { .. });
        assert!(
            evidence_is_residual,
            "determinant refusal must carry residual evidence over the box"
        );
        let (residual, box_, note) = match &fx.refusal.evidence {
            RefusalEvidence::Residual {
                residual,
                box_,
                note,
            } => (*residual, *box_, *note),
            _ => {
                assert!(evidence_is_residual);
                return;
            }
        };
        assert_eq!(residual, ResidualId::R5);
        assert_eq!(box_, fx.domain);
        assert_eq!(
            note,
            "det DF = 2x spans zero over the box: C1 not certifiable"
        );
        let det_at = |x: f64| 2.0 * x;
        assert!(det_at(-2.0) < 0.0 && det_at(2.0) > 0.0, "det spans zero");
    }

    #[test]
    fn weight_straddle_disproven_and_shifted_certifies() {
        let fx = ok_or_fail!(weight_straddles_zero());
        assert!(approx(fx.w_at_half, 0.0), "w(0.5) == 0 exactly");
        assert!(
            fx.hull_lo <= 0.0 && 0.0 <= fx.hull_hi,
            "enclosure contains 0"
        );
        assert_eq!(fx.refusal_disproven.kind, RefusalKind::WeightDegenerate);
        assert_eq!(fx.refusal_disproven.backing, VerdictClass::Disproven);
        assert_eq!(fx.refusal_inconclusive.kind, RefusalKind::WeightDegenerate);
        assert_eq!(fx.refusal_inconclusive.backing, VerdictClass::Inconclusive);
        assert!(
            fx.shifted.value() > 0.0,
            "shifted bound is strictly positive"
        );
        assert!(
            fx.shifted.value() <= weight_on(fx.weights, 0.7),
            "stored lower bound is a genuine lower bound on the shifted box"
        );
    }

    #[test]
    fn deck_wrap_displacement_is_plus_one() {
        let fx = ok_or_fail!(deck_wrap());
        assert!(approx(fx.period, std::f64::consts::TAU));
        assert!(approx(fx.canonical_end_u, 6.4 - std::f64::consts::TAU));
        assert_eq!(fx.displacement, 1);
        assert_eq!(fx.end.deck - fx.start.deck, fx.displacement);
        assert!(
            fx.start.u < fx.period && fx.end.u < fx.period,
            "both stored u values are canonical"
        );
        let raw_start = fx.start.u + fx.period * fx.start.deck as f64;
        let raw_end = fx.end.u + fx.period * fx.end.deck as f64;
        assert!(approx(raw_end, 6.4), "raw end recovers 6.4");
        let crossings = (raw_end / fx.period).floor() - (raw_start / fx.period).floor();
        assert_eq!(crossings, 1.0, "exactly one seam crossing");
        assert_eq!(crossings as i32, fx.displacement);
    }

    #[test]
    fn c1_discontinuity_tangent_jump_is_exact() {
        let fx = c1_discontinuity();
        let dot = fx.tangent0[0] * fx.tangent1[0]
            + fx.tangent0[1] * fx.tangent1[1]
            + fx.tangent0[2] * fx.tangent1[2];
        assert!(approx(dot, 0.0), "consecutive unit tangents are orthogonal");
        assert!(
            (1.0 - dot) > TOL_PARAMETER,
            "the tangent jump exceeds the C1 detection tolerance"
        );
        assert_eq!(fx.positions.len(), 3);
        assert!(approx(fx.tangent0[0], 1.0) && approx(fx.tangent0[1], 0.0));
        assert!(approx(fx.tangent1[1], 1.0) && approx(fx.tangent1[0], 0.0));
    }
}
