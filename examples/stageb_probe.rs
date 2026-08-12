//! Temporary probe for the Stage B focused tests: verify a constructed
//! source-closed B-spline satisfies the evaluator seam identification used by
//! `spline_seam_compatible`, so the A1-A5 tests can rely on the construction.

use look::step::lattice::{SplineAxisClosure, lattice_of_with_closure};
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};
use truck_meshalgo::prelude::BoundedSurface;
use truck_stepio::r#in::step_geometry::Surface;

fn closed_v_surface() -> Surface {
    let knots = (KnotVec::uniform_knot(2, 2), KnotVec::uniform_knot(2, 2));
    let column = |r: f64, z: f64| {
        let p0 = Point3::new(r, 0.0, z);
        let p1 = Point3::new(r * 0.3, r, z);
        let p2 = Point3::new(p0.x * 2.0 - p1.x, p0.y * 2.0 - p1.y, z);
        vec![p0, p1, p2, p0]
    };
    let control_points = vec![
        column(1.0, 0.0),
        column(1.3, 0.5),
        column(1.0, 1.0),
        column(0.7, 1.5),
    ];
    Surface::BSplineSurface(BSplineSurface::new(knots, control_points))
}

fn main() {
    let surface = closed_v_surface();
    match &surface {
        Surface::BSplineSurface(spline) => {
            let ((u0, u1), (v0, v1)) = BoundedSurface::evaluation_range(spline);
            println!("u range = [{u0}, {u1}], v range = [{v0}, {v1}]");

            use truck_meshalgo::prelude::{InnerSpace, ParametricSurface};
            for i in 0..=8 {
                let t = u0 + (u1 - u0) * (i as f64 / 8.0);
                let p0 = surface.subs(t, v0);
                let p1 = surface.subs(t, v1);
                let d0 = surface.vder(t, v0);
                let d1 = surface.vder(t, v1);
                println!(
                    "t={t:.3} pos_res={:.3e} der_res={:.3e}",
                    (p0 - p1).magnitude(),
                    (d0 - d1).magnitude()
                );
            }
        }
        _ => unreachable!(),
    }

    let closure = SplineAxisClosure {
        u_closed: false,
        v_closed: true,
    };
    let lattice = lattice_of_with_closure(&surface, Some(closure));
    println!("closed lattice: {lattice:#?}");
    let open = lattice_of_with_closure(&surface, Some(SplineAxisClosure::OPEN));
    println!("open lattice: {open:#?}");
}
