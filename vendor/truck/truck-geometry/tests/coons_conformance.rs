#![deny(clippy::unwrap_used)]

use truck_geometry::constructive::{ConstructError, DirectTolerance};
use truck_geometry::prelude::*;

fn unit_square() -> (Line<Point3>, Line<Point3>, Line<Point3>, Line<Point3>) {
    let bottom = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
    let top = Line(Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0));
    let left = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0));
    let right = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0));
    (bottom, top, left, right)
}

fn warped_quad() -> (Line<Point3>, Line<Point3>, Line<Point3>, Line<Point3>) {
    let bottom = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
    let top = Line(Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0));
    let left = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 1.0));
    let right = Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
    (bottom, top, left, right)
}

fn near(a: Point3, b: Point3, tol: f64) -> bool {
    (a - b).magnitude() <= tol
}

#[test]
fn coons_corners_validate_and_refuse_mismatched() {
    let (bottom, top, left, right) = unit_square();
    assert!(CoonsSurface::try_new(bottom, top, left, right).is_ok());
    let top_moved = Line(
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(1.0, 1.0, 10.0 * TOLERANCE),
    );
    assert!(matches!(
        CoonsSurface::try_new(bottom, top_moved, left, right),
        Err(ConstructError::InvalidInput)
    ));
}

#[test]
fn coons_boundary_interpolates_exactly() {
    let (bottom, top, left, right) = warped_quad();
    let result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(result.is_ok(), "warped quad is corner-consistent");
    let Ok(surface) = result else {
        return;
    };
    let tol = DirectTolerance::default().position;
    for i in 0..=10 {
        let u = i as f64 / 10.0;
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            assert!(near(surface.subs(u, 0.0), bottom.subs(u), tol));
            assert!(near(surface.subs(u, 1.0), top.subs(u), tol));
            assert!(near(surface.subs(0.0, v), left.subs(v), tol));
            assert!(near(surface.subs(1.0, v), right.subs(v), tol));
        }
    }
}

#[test]
fn coons_first_derivatives_match_finite_differences() {
    let (bottom, top, left, right) = warped_quad();
    let result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(result.is_ok(), "warped quad is corner-consistent");
    let Ok(surface) = result else {
        return;
    };
    let h = 1.0 / 1024.0;
    let bound = 64.0 * TOLERANCE;
    for i in 0..7 {
        let u = (i as f64 + 1.0) / 8.0;
        for j in 0..7 {
            let v = (j as f64 + 1.0) / 8.0;
            let su = (surface.subs(u + h, v) - surface.subs(u - h, v)) / (2.0 * h);
            let sv = (surface.subs(u, v + h) - surface.subs(u, v - h)) / (2.0 * h);
            assert!((surface.uder(u, v) - su).magnitude() <= bound);
            assert!((surface.vder(u, v) - sv).magnitude() <= bound);
        }
    }
}

#[test]
fn coons_degenerate_u_collapse_has_vanishing_jacobian() {
    let bottom = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0));
    let top = Line(Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0));
    let left = Line(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0));
    let right = left;
    let result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(result.is_ok(), "degenerate quad is construction-valid");
    let Ok(surface) = result else {
        return;
    };
    let tol = DirectTolerance::default().position;
    for i in 0..=10 {
        let u = i as f64 / 10.0;
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            assert!(surface.jacobian(u, v).magnitude() <= tol);
        }
    }
}

#[test]
fn coons_convenience_constructor_picks_a_consistent_orientation() {
    let (bottom, top, left, right) = warped_quad();
    assert!(matches!(
        CoonsSurface::try_new(bottom, top.inverse(), left, right.inverse()),
        Err(ConstructError::InvalidInput)
    ));
    let result =
        CoonsSurface::try_new_any_orientation(bottom, top.inverse(), left, right.inverse());
    assert!(
        result.is_ok(),
        "try_new_any_orientation must find a consistent orientation"
    );
    let Ok((surface, flips)) = result else {
        return;
    };
    assert_eq!(flips, [false, true, false, true]);
    let forwarded_result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(forwarded_result.is_ok(), "warped quad is corner-consistent");
    let forwarded = match forwarded_result {
        Ok(s) => s,
        Err(_) => return,
    };
    let tol = DirectTolerance::default().position;
    for i in 0..=10 {
        let u = i as f64 / 10.0;
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            assert!(near(surface.subs(u, v), forwarded.subs(u, v), tol));
        }
    }
}

#[test]
fn coons_inverse_matches_reparametrization() {
    let (bottom, top, left, right) = warped_quad();
    let result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(result.is_ok(), "warped quad is corner-consistent");
    let Ok(surface) = result else {
        return;
    };
    let inverse = surface.inverse();
    let tol = DirectTolerance::default().position;
    for i in 0..=10 {
        let u = i as f64 / 10.0;
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            assert!(near(inverse.subs(u, v), surface.subs(1.0 - u, v), tol));
            let n_inv = inverse.normal(u, v);
            let n_orig = surface.normal(1.0 - u, v);
            assert!((n_inv + n_orig).magnitude() <= tol);
        }
    }
}

#[test]
fn coons_higher_derivatives_vanish() {
    let (bottom, top, left, right) = warped_quad();
    let result = CoonsSurface::try_new(bottom, top, left, right);
    assert!(result.is_ok(), "warped quad is corner-consistent");
    let Ok(surface) = result else {
        return;
    };
    for i in 0..=10 {
        let u = i as f64 / 10.0;
        for j in 0..=10 {
            let v = j as f64 / 10.0;
            let d12 = surface.der_mn(1, 2, u, v);
            assert!(d12.x == 0.0 && d12.y == 0.0 && d12.z == 0.0);
            let d30 = surface.der_mn(3, 0, u, v);
            assert!(d30.x == 0.0 && d30.y == 0.0 && d30.z == 0.0);
            let d21 = surface.der_mn(2, 1, u, v);
            assert!(d21.x == 0.0 && d21.y == 0.0 && d21.z == 0.0);
        }
    }
}
