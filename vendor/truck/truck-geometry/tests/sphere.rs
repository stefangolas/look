use proptest::{prelude::*, property_test};
use std::f64::consts::PI;
use truck_geometry::prelude::*;

#[property_test]
fn test_der_mn(
    #[strategy = (0f64..=PI, 0f64..=2.0 * PI)] (u, v): (f64, f64),
    #[strategy = (0usize..=4, 0usize..=4)] (m, n): (usize, usize),
    #[strategy = prop::array::uniform3(-100f64..=100.0)] center: [f64; 3],
    #[strategy = 0.1f64..=10.0] radius: f64,
    #[strategy = prop::bool::ANY] u_derivate: bool,
) {
    let sphere = Sphere::new(Point3::from(center), radius);

    const EPS: f64 = 1.0e-4;
    let (der0, der1) = if u_derivate {
        let der0 = sphere.der_mn(m + 1, n, u, v);
        let der1 =
            (sphere.der_mn(m, n, u + EPS, v) - sphere.der_mn(m, n, u - EPS, v)) / (2.0 * EPS);
        (der0, der1)
    } else {
        let der0 = sphere.der_mn(m, n + 1, u, v);
        let der1 =
            (sphere.der_mn(m, n, u, v + EPS) - sphere.der_mn(m, n, u, v - EPS)) / (2.0 * EPS);
        (der0, der1)
    };
    prop_assert!((der0 - der1).magnitude() < 0.01 * der0.magnitude());
}

fn exec_search_parameter_test(
    center: [f64; 3],
    radius: f64,
    (u, v): (f64, f64),
    disp: [f64; 3],
    sign: [bool; 3],
) -> std::result::Result<(), TestCaseError> {
    let center = Point3::from(center);
    let sphere = Sphere::new(center, radius);
    let pt = sphere.subs(u, v);
    let (u0, v0) = sphere.search_parameter(pt, None, 100).unwrap();
    prop_assert_near!(Vector2::new(u, v), Vector2::new(u0, v0));
    let boolnum = |t: bool| if t { 1.0 } else { -1.0 };
    let pt = pt
        + Vector3::new(
            disp[0] * boolnum(sign[0]),
            disp[1] * boolnum(sign[1]),
            disp[2] * boolnum(sign[2]),
        );
    prop_assert!(sphere.search_parameter(pt, None, 100).is_none());
    let (u, v) = sphere.search_nearest_parameter(pt, None, 100).unwrap();
    prop_assert_near!(
        sphere.subs(u, v),
        center + (pt - center).normalize() * radius
    );
    Ok(())
}

#[property_test]
fn search_parameter_test(
    #[strategy = prop::array::uniform3(-50f64..=50f64)] center: [f64; 3],
    #[strategy = 0.1f64..100f64] radius: f64,
    #[strategy = (0f64..=PI, 0f64..=(2.0 * PI))] (u, v): (f64, f64),
    #[strategy = prop::array::uniform3(0.01f64..0.1f64)] disp: [f64; 3],
    #[strategy = prop::array::uniform3(prop::bool::ANY)] sign: [bool; 3],
) {
    exec_search_parameter_test(center, radius, (u, v), disp, sign)?;
}

#[test]
fn sphere_derivation_test() {
    let center = Point3::new(1.0, 2.0, 3.0);
    let radius = 4.56;
    let sphere = Sphere::new(center, radius);
    const N: usize = 100;
    for i in 0..N {
        for j in 0..N {
            let u = PI * i as f64 / N as f64;
            let v = 2.0 * PI * j as f64 / N as f64;
            let normal = sphere.normal(u, v);
            assert!(normal.dot(sphere.uder(u, v)).so_small());
            assert!(normal.dot(sphere.vder(u, v)).so_small());
        }
    }
}

/// A tolerance coarser than the sphere used to panic, which took down the
/// whole tessellation of CAD assemblies whose overall extent sets a tolerance
/// larger than their smallest features. Such a sphere has to mesh coarsely
/// instead.
#[test]
fn parameter_division_tolerates_a_tolerance_above_the_radius() {
    let ranges = ((0.0, PI), (0.0, 2.0 * PI));
    for tol in [0.001, 0.0005025, 0.01, 1.0, 1000.0] {
        let sphere = Sphere::new(Point3::origin(), 0.0005);
        let (udiv, vdiv) = sphere.parameter_division(ranges, tol);
        assert!(udiv.len() >= 2, "tol {tol} produced no u division");
        assert!(vdiv.len() >= 2, "tol {tol} produced no v division");
        assert!(
            udiv.iter().chain(&vdiv).all(|t| t.is_finite()),
            "tol {tol} produced a non-finite division"
        );
    }
}

/// Clamping must not disturb the ordinary case, where the tolerance is finer
/// than the sphere and the subdivision follows the chord deviation.
#[test]
fn parameter_division_is_unchanged_below_the_radius() {
    let sphere = Sphere::new(Point3::origin(), 1.0);
    let ranges = ((0.0, PI), (0.0, 2.0 * PI));
    let (udiv, vdiv) = sphere.parameter_division(ranges, 0.01);
    let delta = 2.0 * f64::acos(1.0 - 0.01);
    assert_eq!(udiv.len(), 1 + (1 + (PI / delta).floor() as usize));
    assert_eq!(vdiv.len(), 1 + (1 + (2.0 * PI / delta).floor() as usize));
}

/// A finer tolerance must still mesh at least as finely as a coarser one.
#[test]
fn parameter_division_is_monotone_in_tolerance() {
    let sphere = Sphere::new(Point3::origin(), 1.0);
    let ranges = ((0.0, PI), (0.0, 2.0 * PI));
    let fine = sphere.parameter_division(ranges, 0.001).0.len();
    let coarse = sphere.parameter_division(ranges, 0.5).0.len();
    assert!(
        fine >= coarse,
        "fine {fine} should not be coarser than {coarse}"
    );
}
