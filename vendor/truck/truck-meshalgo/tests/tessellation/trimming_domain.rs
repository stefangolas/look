//! The trimmed region of a face must not depend on the handedness of its chart.
//!
//! `PolyBoundary::new` decides whether a face's boundary is complete by testing
//! whether any closed loop has positive signed area, and appends the whole
//! surface parameter rectangle when none does. Signed area is not invariant
//! under an orientation-reversing reparameterization: for `phi(u, v) = (u, -v)`,
//! `A(phi . gamma) = -A(gamma)`, while the region the face occupies in space is
//! unchanged. A predicate that is not invariant cannot decide a property that
//! is, so the same solid meshes differently depending on how its surface
//! happens to be parameterized.
//!
//! These tests are the specification for that invariant. They are expected to
//! fail until the domain is resolved from containment rather than from the sign
//! of an area.

use super::*;

const TOL: f64 = 0.01;

/// The loop, in 3D. A square of side 0.6 centred in the patch, traversed so
/// that it is *clockwise* in the chart built by [`patch`] and therefore
/// counter-clockwise in the v-reversed chart. Same points, same order, both
/// times: only the surface under it changes.
const LOOP_CORNERS: [[f64; 3]; 4] = [
    [0.2, 0.2, 0.0],
    [0.2, 0.8, 0.0],
    [0.8, 0.8, 0.0],
    [0.8, 0.2, 0.0],
];

/// A bilinear patch over `uv` in `[0, 1]^2`.
///
/// A B-spline is used rather than a `Plane` deliberately: the defect only fires
/// when `try_range_tuple()` is `Some`, and a plane is unbounded. `flip_v` maps
/// `S'(u, v) = S(u, 1 - v)`, which is the same surface in space with the
/// opposite chart handedness.
fn patch(flip_v: bool) -> Surface {
    let corner = |u: f64, v: f64| Point3::new(u, v, 0.0);
    let (v0, v1) = match flip_v {
        false => (0.0, 1.0),
        true => (1.0, 0.0),
    };
    let control_points = vec![
        vec![corner(0.0, v0), corner(0.0, v1)],
        vec![corner(1.0, v0), corner(1.0, v1)],
    ];
    let knots = KnotVec::bezier_knot(1);
    Surface::BSplineSurface(BSplineSurface::new((knots.clone(), knots), control_points))
}

/// The square loop as a closed wire of straight edges.
fn square_wire() -> Wire {
    let vertices: Vec<Vertex> = LOOP_CORNERS
        .iter()
        .map(|p| builder::vertex(Point3::from(*p)))
        .collect();
    (0..4)
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % 4]))
        .collect()
}

fn mesh_bounds(surface: Surface) -> BoundingBox<Point3> {
    let face = Face::new(vec![square_wire()], surface);
    let shell: Shell = vec![face].into();
    let polygon = shell.robust_triangulation(TOL).to_polygon();
    let mut bounds = BoundingBox::<Point3>::new();
    for point in polygon.positions() {
        bounds.push(*point);
    }
    bounds
}

/// The minimal failing case.
///
/// One bounded loop, a support surface with a finite parameter range, and no
/// periodicity anywhere: the face is the interior of the loop and nothing else.
/// The defect returns `R \ interior(gamma)` instead, so the mesh spans the
/// whole patch rather than the middle 60% of it.
#[test]
fn a_clockwise_loop_meshes_its_interior_not_its_complement() {
    let bounds = mesh_bounds(patch(false));
    assert!(!bounds.is_empty(), "the face meshed to nothing");

    let (min, max) = (bounds.min(), bounds.max());
    assert!(
        min[0] > 0.2 - TOL && min[1] > 0.2 - TOL,
        "mesh reaches outside the loop: min {min:?}, expected to start at (0.2, 0.2). \
         Reaching 0 means the surface parameter rectangle was meshed instead."
    );
    assert!(
        max[0] < 0.8 + TOL && max[1] < 0.8 + TOL,
        "mesh reaches outside the loop: max {max:?}, expected to stop at (0.8, 0.8). \
         Reaching 1 means the surface parameter rectangle was meshed instead."
    );
}

/// The invariance the whole defect violates.
///
/// Both faces occupy the identical region of space, bounded by the identical
/// 3D curves. They differ only in the handedness of the chart used to describe
/// the surface, which no observer of the solid can see. Their meshes must
/// agree.
#[test]
fn the_mesh_does_not_depend_on_chart_handedness() {
    let forward = mesh_bounds(patch(false));
    let reversed = mesh_bounds(patch(true));

    assert!(
        !forward.is_empty() && !reversed.is_empty(),
        "a face meshed to nothing: forward {forward:?}, reversed {reversed:?}"
    );
    for axis in 0..3 {
        assert!(
            (forward.min()[axis] - reversed.min()[axis]).abs() < TOL
                && (forward.max()[axis] - reversed.max()[axis]).abs() < TOL,
            "reversing the chart changed the meshed region on axis {axis}: \
             forward {forward:?} vs reversed {reversed:?}"
        );
    }
}
