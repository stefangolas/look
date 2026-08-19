#[doc(hidden)]
pub use truck_geometry::prelude::{algo, inv_or_zero};
pub use truck_geometry::{canonical::*, decorators::*, nurbs::*, specifieds::*};

#[cfg(test)]
// BG-S0-001 tests. Only the test that needs truck-topology types stays here:
// `truck-geometry` must not depend on `truck-topology`. The rest of the module
// lives in `truck-geometry::canonical`.
mod include_intersection_curve_tests {
    use crate::*;

    /// The plane z = 0 through the origin.
    fn zx_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// The plane x = 0 through the origin.
    fn yz_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
    }

    /// The plane y = 0 through the origin.
    fn xz_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
    }

    fn intersection_curve(surface0: Surface, surface1: Surface, leader: Curve) -> Curve {
        Curve::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(leader),
        ))
    }

    #[test]
    fn boolean_derived_face_consistency_returns() {
        // Spec regression: a face whose boundary carries an
        // `IntersectionCurve` (the variant Booleans produce) previously aborted
        // in `Surface::include` via `unimplemented!()`. It must now return —
        // here through `Face::is_geometric_consistent`, which fails closed on
        // `NumericallyUnresolved`.
        let v0 = Vertex::new(Point3::new(0.0, 0.0, -1.0));
        let v1 = Vertex::new(Point3::new(0.0, 0.0, 1.0));
        let isc = intersection_curve(
            Surface::Plane(xz_plane()),
            Surface::Plane(yz_plane()),
            Curve::Line(Line(
                Point3::new(0.0, 0.0, -1.0),
                Point3::new(0.0, 0.0, 1.0),
            )),
        );
        let wire: Wire = vec![Edge::new(&v0, &v1, isc.clone()), Edge::new(&v1, &v0, isc)].into();
        let face = Face::new(vec![wire], Surface::Plane(zx_plane()));
        // The ISC edge is off the capping plane, so the face is certified
        // inconsistent — the point of the regression is that this returns
        // instead of aborting.
        assert!(!face.is_geometric_consistent());
    }
}
