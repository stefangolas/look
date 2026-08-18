//! Unit and property tests for the cylinder and cone analytic carriers
//! (BG-CE-006-CYL-CONE).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use proptest::{prelude::*, property_test};
use std::f64::consts::PI;
use truck_base::evidence::{EnvelopeCase, Refusal};
use truck_geometry::prelude::*;

fn cylinder(center: Point3, radius: f64) -> Cylinder {
    match Cylinder::new(center, radius) {
        Ok(certified) => certified.value,
        Err(_) => unreachable!("a finite positive radius is always accepted"),
    }
}

fn cone(apex: Point3, half_angle: f64) -> Cone {
    match Cone::new(apex, half_angle) {
        Ok(certified) => certified.value,
        Err(_) => unreachable!("a finite half angle in the open interval is always accepted"),
    }
}

/// `subs(u, v) - (center + (0, 0, v))` is `normal(u, v) * radius`, the same
/// relation the Sphere doc example asserts. This catches a swapped `u`/`v`
/// and a wrong normal sign in one assertion.
#[test]
fn cylinder_point_normal_relation() {
    let center = Point3::new(1.0, 2.0, 3.0);
    let radius = 4.56;
    let cylinder = cylinder(center, radius);
    const N: usize = 100;
    for i in 0..=N {
        for j in 0..=N {
            let u = 2.0 * PI * i as f64 / N as f64;
            let v = 2.0 * (j as f64 / N as f64) - 1.0;
            assert_near!(
                cylinder.subs(u, v) - (center + Vector3::new(0.0, 0.0, v)),
                cylinder.normal(u, v) * radius
            );
        }
    }
}

/// A point constructed on the cylinder round-trips through `search_parameter`;
/// a point displaced ten times the tolerance off the surface does not.
#[test]
fn cylinder_round_trips_through_search_parameter() {
    let center = Point3::new(1.0, 2.0, 3.0);
    let radius = 4.56;
    let cylinder = cylinder(center, radius);
    const N: usize = 50;
    for i in 0..N {
        for j in 0..=N {
            let u = 2.0 * PI * i as f64 / N as f64;
            let v = 4.0 * (j as f64 / N as f64) - 2.0;
            match cylinder.search_parameter(cylinder.subs(u, v), None, 100) {
                Some((u0, v0)) => {
                    assert_near!(u, u0);
                    assert_near!(v, v0);
                }
                None => unreachable!("a point constructed on the cylinder must be found"),
            }
            let off_surface = cylinder.subs(u, v) + cylinder.normal(u, v) * (10.0 * TOLERANCE);
            assert!(
                cylinder.search_parameter(off_surface, None, 100).is_none(),
                "a point ten tolerances off the surface must not be found"
            );
        }
    }
}

/// The apex is a first-class point: it lies on the surface, `uder` vanishes
/// there, and the normal is zero there rather than an arbitrary unit vector.
#[test]
fn cone_apex_is_a_first_class_point() {
    let apex = Point3::new(1.0, 2.0, 3.0);
    let cone = cone(apex, PI / 6.0);
    for i in 0..=16 {
        let u = 2.0 * PI * i as f64 / 16.0;
        assert_near!(cone.subs(u, 0.0), apex);
        assert!(cone.uder(u, 0.0).so_small());
        assert!(cone.normal(u, 0.0).so_small());
    }
}

/// A point constructed on the cone (for `v > 0`) round-trips through
/// `search_parameter`; a point displaced ten times the tolerance off the
/// surface does not.
#[test]
fn cone_round_trips_through_search_parameter() {
    let apex = Point3::new(1.0, 2.0, 3.0);
    let cone = cone(apex, PI / 6.0);
    const N: usize = 50;
    for i in 0..N {
        for j in 0..=N {
            let u = 2.0 * PI * i as f64 / N as f64;
            let v = 4.0 * (j as f64 / N as f64) + 0.1;
            match cone.search_parameter(cone.subs(u, v), None, 100) {
                Some((u0, v0)) => {
                    assert_near!(u, u0);
                    assert_near!(v, v0);
                }
                None => unreachable!("a point constructed on the cone must be found"),
            }
            let off_surface = cone.subs(u, v) + cone.normal(u, v) * (10.0 * TOLERANCE);
            assert!(
                cone.search_parameter(off_surface, None, 100).is_none(),
                "a point ten tolerances off the surface must not be found"
            );
        }
    }
}

/// Constructors refuse degenerate input (H-1): they return
/// `UnsupportedEnvelope(ChartDegenerate)` instead of panicking.
#[test]
fn degenerate_radius_refuses() {
    let degenerate_radius = [0.0, -1.0, f64::NAN, f64::INFINITY];
    for radius in degenerate_radius {
        assert!(
            matches!(
                Cylinder::new(Point3::origin(), radius),
                Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
            ),
            "cylinder radius {radius} must be refused"
        );
    }
    let degenerate_half_angle = [0.0, PI / 2.0, -0.1, f64::NAN];
    for half_angle in degenerate_half_angle {
        assert!(
            matches!(
                Cone::new(Point3::origin(), half_angle),
                Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
            ),
            "cone half angle {half_angle} must be refused"
        );
    }
}

/// The named derivatives agree with `der_mn` at random parameters for both
/// carriers. `der_mn` is written in the cyclic `m % 4` style; a mismatch
/// between it and the named methods is the most likely defect in this packet.
#[property_test]
fn ders_agree_with_der_mn(
    #[strategy = 0.0..=2.0 * PI] u: f64,
    #[strategy = -5.0..=5.0] v: f64,
    #[strategy = prop::array::uniform3(-100.0..=100.0)] base: [f64; 3],
    #[strategy = 0.1..=10.0] radius: f64,
    #[strategy = 0.1..=(PI / 2.0 - 0.05)] half_angle: f64,
) {
    let base = Point3::from(base);
    let cylinder = cylinder(base, radius);
    let cone = cone(base, half_angle);
    let named = [
        cylinder.uder(u, v),
        cylinder.vder(u, v),
        cylinder.uuder(u, v),
        cylinder.uvder(u, v),
        cylinder.vvder(u, v),
        cone.uder(u, v),
        cone.vder(u, v),
        cone.uuder(u, v),
        cone.uvder(u, v),
        cone.vvder(u, v),
    ];
    let der = [
        cylinder.der_mn(1, 0, u, v),
        cylinder.der_mn(0, 1, u, v),
        cylinder.der_mn(2, 0, u, v),
        cylinder.der_mn(1, 1, u, v),
        cylinder.der_mn(0, 2, u, v),
        cone.der_mn(1, 0, u, v),
        cone.der_mn(0, 1, u, v),
        cone.der_mn(2, 0, u, v),
        cone.der_mn(1, 1, u, v),
        cone.der_mn(0, 2, u, v),
    ];
    for (named, der) in named.iter().zip(der.iter()) {
        prop_assert_near!(named, der);
    }
}
