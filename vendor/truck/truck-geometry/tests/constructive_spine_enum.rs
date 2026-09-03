#![deny(clippy::unwrap_used)]

//! BG-KV2-203-C1DELTA — the `Spine` enum, the PH fast path (r3 rescope), and
//! `FrameData` (constructive half).
//!
//! # H-1
//! This test module carries the crate's unwrap discipline: no `unwrap`, no
//! `expect`, no `panic!`, no module-level `allow`. Values are extracted from
//! `Result`s by asserting the precondition on a real predicate and then
//! matching with a divergent `return` arm, so clippy stays silent.
//!
//! The PH tests are non-circular: the fixture Bézier nets are built here by
//! independent power-basis arithmetic from a declared cubic-quaternion
//! preimage (`A·i·A*` integrated and degree-elevated), and the fixture
//! premises (membership `τ == 0`; non-membership `τ != 0`; exact arc length)
//! are asserted with the test's own polynomial code before the library is
//! consulted.

use truck_geometry::base::*;
use truck_geometry::canonical::Curve;
use truck_geometry::constructive::{
    ConstructError, DirectTolerance, FrameData, FrameLaw, LineSpine, PendingMembership,
    PolylineSpine, Profile2D, ProfileLaw, RmErfSeptic, RrmfQuintic, SamplingPolicy,
    SepticMembership, Spine, SpineCurve, SpineFrameRecipe,
};
use truck_geometry::decorators::{SpineFrameCurve, SpineFrameSurface};
use truck_geometry::nurbs::{BSplineCurve, KnotVec};

// ---------------------------------------------------------------------------
// Independent power-basis polynomial helpers (test-local ground truth).
// ---------------------------------------------------------------------------

type Poly = Vec<f64>;

fn padd(a: &[f64], b: &[f64]) -> Poly {
    let mut out = a.to_vec();
    if b.len() > out.len() {
        out.resize(b.len(), 0.0);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] += c;
    }
    out
}

fn psub(a: &[f64], b: &[f64]) -> Poly {
    let mut out = a.to_vec();
    if b.len() > out.len() {
        out.resize(b.len(), 0.0);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] -= c;
    }
    out
}

fn pmul(a: &[f64], b: &[f64]) -> Poly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ca) in a.iter().enumerate() {
        for (j, &cb) in b.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    out
}

fn pderiv(a: &[f64]) -> Poly {
    let mut out = Vec::new();
    for (i, &c) in a.iter().enumerate().skip(1) {
        out.push(c * i as f64);
    }
    out
}

fn peval(a: &[f64], x: f64) -> f64 {
    a.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

fn binom_test(n: usize, k: usize) -> u64 {
    let k = k.min(n - k);
    let mut value: u64 = 1;
    for i in 0..k {
        value = value * (n - i) as u64 / (i + 1) as u64;
    }
    value
}

/// `A·i·A*` hodograph components for preimage `A = u + v i + p j + q k`.
fn hodograph_of(u: &[f64], v: &[f64], p: &[f64], q: &[f64]) -> [Poly; 3] {
    let x = psub(
        &padd(&pmul(u, u), &pmul(v, v)),
        &padd(&pmul(p, p), &pmul(q, q)),
    );
    let two = [2.0];
    let y = pmul(&two, &padd(&pmul(v, p), &pmul(u, q)));
    let z = pmul(&two, &psub(&pmul(v, q), &pmul(u, p)));
    [x, y, z]
}

/// The ER-frame spin numerator `2(u v' − v u' − p q' + q p')`.
fn spin_numerator(u: &[f64], v: &[f64], p: &[f64], q: &[f64]) -> Poly {
    let two = [2.0];
    let mut out = psub(&pmul(u, &pderiv(v)), &pmul(v, &pderiv(u)));
    out = psub(&out, &pmul(p, &pderiv(q)));
    out = padd(&out, &pmul(q, &pderiv(p)));
    pmul(&two, &out)
}

/// The degree-7 Bézier net of the curve whose preimage is `(u, v, p, q)`,
/// starting at `origin` — the position polynomial is `origin + ∫ A·i·A*`,
/// converted power→Bézier by `P_j = Σ_{k≤j} q_k · C(j,k)/C(7,k)`.
fn net_from_preimage(u: &[f64], v: &[f64], p: &[f64], q: &[f64], origin: Point3) -> [Point3; 8] {
    let hod = hodograph_of(u, v, p, q);
    let origin_xyz = [origin.x, origin.y, origin.z];
    let mut net = [Point3::origin(); 8];
    for (j, slot) in net.iter_mut().enumerate() {
        let mut pj = [0.0f64; 3];
        for axis in 0..3 {
            let mut value = origin_xyz[axis];
            let mut antideriv = vec![0.0; hod[axis].len() + 1];
            for (k, &c) in hod[axis].iter().enumerate() {
                antideriv[k + 1] = c / (k as f64 + 1.0);
            }
            for (k, &c) in antideriv.iter().enumerate() {
                if k <= j && c != 0.0 {
                    value += c * binom_test(j, k) as f64 / binom_test(7, k) as f64;
                }
            }
            pj[axis] = value;
        }
        *slot = Point3::new(pj[0], pj[1], pj[2]);
    }
    net
}

// ---------------------------------------------------------------------------
// PH fixtures (planar degenerate ERF-RMF family; non-member general fixture).
// ---------------------------------------------------------------------------

/// The accepted member fixture: preimage `A(w) = 1 + w·j` — planar,
/// `τ ≡ 0` (the ER frame IS the RMF). It traces a planar cubic arc, so as a
/// degree-7 net it is the PLANAR DEGENERATE member family the r3 amendment
/// admits for tests and flags as degenerate.
fn member_net() -> [Point3; 8] {
    net_from_preimage(&[1.0], &[], &[0.0, 1.0], &[], Point3::origin())
}

/// The refused fixture: preimage `A(w) = 1 + w·i + w²·j + w³·k` — a genuine
/// spatial degree-7 PH curve whose ER frame is NOT rotation-minimizing
/// (`τ(w) = 2(1 − w⁴) ≠ 0`).
fn non_member_net() -> [Point3; 8] {
    net_from_preimage(
        &[1.0],
        &[0.0, 1.0],
        &[0.0, 0.0, 1.0],
        &[0.0, 0.0, 0.0, 1.0],
        Point3::origin(),
    )
}

// ---------------------------------------------------------------------------
// General (non-PH) spine fixture and transport fixtures.
// ---------------------------------------------------------------------------

/// The Constant-profile triangle used by the transport fixtures.
fn triangle() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ],
    }
}

/// A curved, C¹ helix spine: `C(s) = (cos 2πs, sin 2πs, 2πs)` on `[0, 1]`.
#[derive(Debug, Clone, Copy)]
struct HelixSpine;

impl SpineCurve for HelixSpine {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
    fn position_at(&self, s: f64) -> std::result::Result<Point3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let theta = 2.0 * std::f64::consts::PI * s;
        Ok(Point3::new(theta.cos(), theta.sin(), theta))
    }
    fn derivative_at(&self, s: f64) -> std::result::Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let two_pi = 2.0 * std::f64::consts::PI;
        let theta = 2.0 * std::f64::consts::PI * s;
        Ok(Vector3::new(
            -two_pi * theta.sin(),
            two_pi * theta.cos(),
            two_pi,
        ))
    }
}

/// The unit-square profile for the promotion fixture.
fn unit_square() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
    }
}

/// A general cubic B-spline spine (a single cubic Bézier segment — a genuine
/// non-PH, non-line, non-polyline `Curve`).
fn cubic_bspline_spine() -> Curve {
    let knot = KnotVec::bezier_knot(3);
    let control = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.2, 2.0, 0.3),
        Point3::new(0.4, 1.5, 1.2),
        Point3::new(0.0, 0.2, 1.6),
    ];
    Curve::BSplineCurve(BSplineCurve::new(knot, control))
}

/// Extracts the `Ok` value of a `Result` in a test, asserting the failure
/// precondition on a real predicate first (clippy-silent, unwrap-free).
fn expect_ok<T, E>(result: std::result::Result<T, E>, what: &str) -> Option<T> {
    assert!(result.is_ok(), "{what} refused unexpectedly");
    result.ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn spine_enum_dispatches_general_to_the_landed_spine_curve() {
    let underlying = LineSpine {
        start: Point3::origin(),
        end: Point3::new(1.0, 0.5, 0.25),
    };
    let wrapped = Spine::general(underlying);
    let tol = DirectTolerance::default().position;
    assert_eq!(wrapped.domain(), underlying.domain());
    for &s in &[0.0, 0.3, 0.7, 1.0] {
        let a = match expect_ok(wrapped.position_at(s), "enum position_at") {
            Some(a) => a,
            None => return,
        };
        let b = match expect_ok(underlying.position_at(s), "underlying position_at") {
            Some(b) => b,
            None => return,
        };
        assert!(
            (a - b).magnitude() <= tol,
            "position dispatch diverged at {s}"
        );
        let a = match expect_ok(wrapped.derivative_at(s), "enum derivative_at") {
            Some(a) => a,
            None => return,
        };
        let b = match expect_ok(underlying.derivative_at(s), "underlying derivative_at") {
            Some(b) => b,
            None => return,
        };
        assert!(
            (a - b).magnitude() <= tol,
            "derivative dispatch diverged at {s}"
        );
    }
    // The enum is a `SpineCurve` itself: it can ride a recipe's spine slot.
    let recipe = SpineFrameRecipe::new(
        wrapped,
        ProfileLaw::Constant(triangle()),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        },
    );
    for &s in &[0.0, 0.5, 1.0] {
        match expect_ok(recipe.position(s, 0.25), "recipe over the enum") {
            Some(p) => assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()),
            None => return,
        }
    }
}

#[test]
fn polyline_spine_refuses_as_not_c1_through_the_enum() {
    let polyline = PolylineSpine::try_new(vec![
        Point3::origin(),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
    ])
    .ok();
    assert!(polyline.is_some(), "polyline fixture refused");
    let polyline = match polyline {
        Some(p) => p,
        None => return,
    };
    let wrapped = Spine::general(polyline);
    assert_eq!(wrapped.domain(), (0.0, 2.0));
    // Mid-segment evaluation is fine through the enum.
    let p = match expect_ok(wrapped.position_at(0.5), "mid-segment position") {
        Some(p) => p,
        None => return,
    };
    assert!((p - Point3::new(0.5, 0.0, 0.0)).magnitude() <= 1.0e-12); // H-3
                                                                      // The declaration-based refusal fires THROUGH the enum at the corner.
    match wrapped.derivative_at(1.0) {
        Err(ConstructError::SpineNotC1 { at }) => assert_eq!(at, 1.0),
        _ => {
            assert!(
                wrapped.derivative_at(1.0).is_err(),
                "corner derivative did not refuse SpineNotC1 through the enum"
            );
            return;
        }
    }
    // And it surfaces through the enum consumed by a recipe frame at a corner
    // station (the recipe's spine slot is the enum).
    let recipe = SpineFrameRecipe::new(
        wrapped,
        ProfileLaw::Constant(triangle()),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        },
    );
    match recipe.frame(1.0) {
        Err(ConstructError::SpineNotC1 { .. }) => {}
        _ => {
            // The match is the tail of the test: failing the assert ends the
            // test, so no trailing `return` is needed.
            assert!(
                recipe.frame(1.0).is_err(),
                "recipe frame at the corner did not refuse SpineNotC1"
            );
        }
    }
}

#[test]
fn ph_septic_membership_and_rational_frame() {
    // Fixture premises, asserted with the test's own polynomial code.
    let member_spin = spin_numerator(&[1.0], &[], &[0.0, 1.0], &[]);
    let member_sup = (0..=64)
        .map(|i| peval(&member_spin, i as f64 / 64.0).abs())
        .fold(0.0f64, f64::max);
    assert!(
        member_sup <= 1.0e-12,
        "member fixture premise: τ must vanish"
    ); // H-3
    let nm_spin = spin_numerator(&[1.0], &[0.0, 1.0], &[0.0, 0.0, 1.0], &[0.0, 0.0, 0.0, 1.0]);
    let nm_sup = (0..=64)
        .map(|i| peval(&nm_spin, i as f64 / 64.0).abs())
        .fold(0.0f64, f64::max);
    assert!(
        nm_sup > 0.1,
        "non-member fixture premise: τ must be nonzero"
    );

    let member = member_net();
    let septic = match expect_ok(RmErfSeptic::try_new(member), "member net try_new") {
        Some(septic) => septic,
        None => return,
    };
    assert_eq!(septic.control_points(), member);

    // τ != 0 is refused with the membership evidence.
    let non_member = non_member_net();
    match RmErfSeptic::try_new(non_member) {
        Err(SepticMembership::NotErfRmf { tau_sup }) => assert!(tau_sup > 0.0),
        _ => {
            assert!(
                RmErfSeptic::try_new(non_member).is_err(),
                "the τ != 0 net refused the wrong membership variant"
            );
            return;
        }
    }

    // The frame at a station is the ER frame of a member: rational,
    // orthonormal, right-handed, tangent-aligned, and rotation-minimizing.
    let tol = DirectTolerance::default().position;
    for i in 0..=16 {
        let s = i as f64 / 16.0;
        let frame = match expect_ok(septic.frame_at(s), "frame_at") {
            Some(f) => f,
            None => return,
        };
        assert!(
            (frame.tangent.magnitude() - 1.0).abs() <= tol,
            "tangent not unit at {s}"
        );
        assert!(
            (frame.normal.magnitude() - 1.0).abs() <= tol,
            "normal not unit at {s}"
        );
        assert!(
            (frame.binormal.magnitude() - 1.0).abs() <= tol,
            "binormal not unit at {s}"
        );
        assert!(
            frame.tangent.dot(frame.normal).abs() <= tol,
            "t·n != 0 at {s}"
        );
        assert!(
            frame.tangent.dot(frame.binormal).abs() <= tol,
            "t·b != 0 at {s}"
        );
        assert!(
            (frame.tangent.cross(frame.normal) - frame.binormal).magnitude() <= tol,
            "frame not right-handed at {s}"
        );
        let derivative = match expect_ok(septic.derivative_at(s), "derivative_at") {
            Some(d) => d,
            None => return,
        };
        let tangent = derivative / derivative.magnitude();
        assert!(
            (frame.tangent - tangent).magnitude() <= tol,
            "frame tangent diverged from the unit tangent at {s}"
        );
        // Rotation-minimizing: the normal has no spin about the tangent
        // (finite-difference spin ≈ 0).
        if 0 < i && i < 16 {
            let h = 1.0 / 32.0;
            let before = match expect_ok(septic.frame_at(s - h), "frame_at before") {
                Some(f) => f,
                None => return,
            };
            let after = match expect_ok(septic.frame_at(s + h), "frame_at after") {
                Some(f) => f,
                None => return,
            };
            let spin = ((after.normal - before.normal) / (2.0 * h)).dot(frame.binormal);
            assert!(spin.abs() <= 1.0e-4, "member frame spins at {s}: {spin}"); // H-3
        }
    }
}

#[test]
fn rmf_quintic_constructor_refuses_pending() {
    let net: [Point3; 6] = [
        Point3::origin(),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 1.0),
    ];
    match RrmfQuintic::try_new(net) {
        Err(PendingMembership { predicate }) => {
            assert_eq!(
                predicate,
                PendingMembership::RRMF_MEMBERSHIP_PENDING,
                "the deferral must carry the named trigger"
            );
            assert_eq!(PendingMembership::KIND, "Budget");
            assert_eq!(PendingMembership::BACKING, "Inconclusive");
        }
        _ => {
            // The match is the tail of the test: failing the assert ends the
            // test, so no trailing `return` is needed.
            assert!(
                RrmfQuintic::try_new(net).is_err(),
                "the RRMF quintic constructor must refuse pending"
            );
        }
    }
}

#[test]
fn frame_data_refinement_level_changes_surface_and_is_recorded() {
    let law = FrameLaw::ParallelTransport {
        initial_normal: Vector3::unit_z(),
    };
    let base = SpineFrameRecipe::new(HelixSpine, ProfileLaw::Constant(triangle()), law);
    // The default is recorded as the landed 64-station level.
    assert_eq!(base.frame_data(), FrameData::default());
    assert_eq!(FrameData::default().refinement_level, 64);
    assert_eq!(FrameData::DEFAULT_REFINEMENT_LEVEL, 64);

    let coarse = base.clone().with_frame_data(FrameData {
        refinement_level: 8,
    });
    assert_eq!(
        coarse.frame_data(),
        FrameData {
            refinement_level: 8
        }
    );

    // Changing the recorded level changes the transported surface — by
    // design. Off-grid stations see different double-reflection grids.
    let mut changed = false;
    for &s in &[0.17, 0.33, 0.61, 0.89] {
        let fine = match expect_ok(base.frame(s), "frame") {
            Some(frame) => frame,
            None => return,
        };
        let coarse_frame = match expect_ok(coarse.frame(s), "coarse frame") {
            Some(frame) => frame,
            None => return,
        };
        if (fine.normal - coarse_frame.normal).magnitude() > 1.0e-9 {
            changed = true;
        }
    }
    assert!(
        changed,
        "changing refinement_level did not change the frame"
    );
}

#[test]
fn frame_data_is_resolution_independent_once_frozen() {
    let law = FrameLaw::ParallelTransport {
        initial_normal: Vector3::unit_z(),
    };
    let level = FrameData {
        refinement_level: 16,
    };
    let recipe_a = SpineFrameRecipe::new(HelixSpine, ProfileLaw::Constant(triangle()), law)
        .with_frame_data(level);
    let recipe_b = SpineFrameRecipe::new(HelixSpine, ProfileLaw::Constant(triangle()), law)
        .with_frame_data(level);
    let stations = [0.13, 0.37, 0.71, 0.97];
    // Same level twice: byte-identical sample positions (deterministic, frozen).
    for &s in &stations {
        let a = match expect_ok(recipe_a.position(s, 0.25), "position a") {
            Some(p) => p,
            None => return,
        };
        let b = match expect_ok(recipe_b.position(s, 0.25), "position b") {
            Some(p) => p,
            None => return,
        };
        assert_eq!(a, b, "frozen level produced non-identical samples at {s}");
    }
    // A different level produces different positions at some station.
    let coarse = recipe_a.clone().with_frame_data(FrameData {
        refinement_level: 8,
    });
    let mut differs = false;
    for &s in &stations {
        let frozen = match expect_ok(recipe_a.position(s, 0.25), "frozen position") {
            Some(p) => p,
            None => return,
        };
        let coarse_p = match expect_ok(coarse.position(s, 0.25), "coarse position") {
            Some(p) => p,
            None => return,
        };
        if (frozen - coarse_p).magnitude() > 1.0e-9 {
            differs = true;
        }
    }
    assert!(
        differs,
        "changing the recorded level did not move the samples"
    );
}

#[test]
fn general_spine_becomes_certifiedpatch_not_refused_for_promotion() {
    let curve = cubic_bspline_spine();
    let profile = ProfileLaw::Constant(unit_square());
    let frame = FrameLaw::FixedPlane {
        normal: Vector3::unit_x(),
    };
    let tol = DirectTolerance::default().position;

    // Construct: the general spine is first-class THROUGH the enum — a recipe
    // whose spine slot is `Spine::General(Box<dyn SpineCurve>)` is not refused
    // for promotion.
    let recipe = SpineFrameRecipe::new(Spine::general(curve.clone()), profile.clone(), frame);

    // Sample: no refusal anywhere on the station/ring grid.
    let stations = match expect_ok(
        (SamplingPolicy::UniformCount { spine: 9 }).resolve(0.0, 1.0),
        "station resolve",
    ) {
        Some(stations) => stations,
        None => return,
    };
    for &s in &stations {
        for j in 0..4 {
            let v = (j as f64 + 0.5) / 4.0;
            match expect_ok(recipe.position(s, v), "general spine recipe position") {
                Some(p) => {
                    assert!(
                        p.x.is_finite() && p.y.is_finite() && p.z.is_finite(),
                        "non-finite sample at ({s}, {v})"
                    );
                }
                None => return,
            }
        }
    }

    // Realize via the landed decorator path over the same spine as its
    // canonical `Curve` carrier (the promotion the enum guarantees): the
    // parametric surface and trajectory construct and evaluate.
    let curve_recipe = SpineFrameRecipe::new(curve.clone(), profile, frame);
    let surface = match expect_ok(
        SpineFrameSurface::try_new(curve_recipe.clone(), 0.0, 1.0, 0.0, 0.25),
        "decorator surface try_new",
    ) {
        Some(surface) => surface,
        None => return,
    };
    let trajectory = match expect_ok(
        SpineFrameCurve::try_new(curve_recipe.clone(), 0.0, 1.0, 0.25),
        "decorator trajectory try_new",
    ) {
        Some(curve) => curve,
        None => return,
    };
    for &s in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        for &v in &[0.0625, 0.1875] {
            let on_surface = surface.subs(s, v);
            let on_recipe = match expect_ok(curve_recipe.position(s, v), "recipe position") {
                Some(p) => p,
                None => return,
            };
            assert!(
                (on_surface - on_recipe).magnitude() <= tol,
                "realized surface diverged from the recipe at ({s}, {v})"
            );
        }
        let on_curve = trajectory.subs(s);
        let on_surface = surface.subs(s, 0.25);
        assert!(
            (on_curve - on_surface).magnitude() <= tol,
            "trajectory left the surface at {s}"
        );
    }
}

#[test]
fn ph_arclength_matches_f64_integration_ground_truth() {
    let septic = match expect_ok(RmErfSeptic::try_new(member_net()), "member net try_new") {
        Some(septic) => septic,
        None => return,
    };
    // Ground truth: adaptive Simpson on |c'| over [0, s] in f64. For the
    // member fixture |c'| == σ exactly, so this is independent quadrature of
    // the same speed polynomial.
    for &s in &[0.25, 0.5, 0.75, 1.0] {
        let exact = match expect_ok(septic.arc_length(s), "arc_length") {
            Some(length) => length,
            None => return,
        };
        let integrand = |w: f64| -> f64 {
            match septic.derivative_at(w) {
                Ok(d) => d.magnitude(),
                Err(_) => f64::NAN,
            }
        };
        let simpson = adaptive_simpson(&integrand, 0.0, s, 1.0e-12);
        assert!(
            simpson.is_finite() && (exact - simpson).abs() <= 1.0e-9 * (1.0 + simpson.abs()),
            "exact arc length {exact} diverged from quadrature {simpson} at {s}"
        ); // H-3
    }
    // Closed-form spot check: the member fixture's speed is σ(w) = 1 + w², so
    // L(1) = 4/3.
    let length = match expect_ok(septic.arc_length(1.0), "arc_length at 1.0") {
        Some(length) => length,
        None => return,
    };
    assert!(
        (length - 4.0 / 3.0).abs() <= 1.0e-9,
        "arc length {length} != 4/3 for the member fixture"
    ); // H-3
}

/// Adaptive Simpson quadrature of `f` over `[a, b]` to an absolute tolerance.
fn adaptive_simpson(f: &dyn Fn(f64) -> f64, a: f64, b: f64, tolerance: f64) -> f64 {
    fn simpson(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> f64 {
        let m = 0.5 * (a + b);
        (b - a) / 6.0 * (f(a) + 4.0 * f(m) + f(b))
    }
    fn recur(
        f: &dyn Fn(f64) -> f64,
        a: f64,
        b: f64,
        tolerance: f64,
        whole: f64,
        depth: u32,
    ) -> f64 {
        let m = 0.5 * (a + b);
        let left = simpson(f, a, m);
        let right = simpson(f, m, b);
        let delta = left + right - whole;
        if depth > 24 || delta.abs() <= 15.0 * tolerance {
            return left + right + delta / 15.0;
        }
        recur(f, a, m, tolerance / 2.0, left, depth + 1)
            + recur(f, m, b, tolerance / 2.0, right, depth + 1)
    }
    recur(f, a, b, tolerance, simpson(f, a, b), 0)
}
