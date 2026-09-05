//! Profile builders for the showcases: closed `Profile2D` polygons in the
//! frame plane. Convention (`truck-geometry/src/constructive/recipe.rs`):
//! profile-x rides the frame NORMAL, profile-y rides the frame BINORMAL; the
//! vertex ring is CCW about the profile normal; vertex `j` sits at ring
//! parameter `v = j / k`.
//!
//! For the waterslide the frame normal is the chute's UP, so the U opens
//! toward +profile-x; "across" is profile-y.

use truck_base::cgmath64::Point2;
use truck_geometry::constructive::{ConstructError, Profile2D};

/// The closed U-chute cross-section: outer width `width` (across), wall top
/// at up = `wall_height`, wall thickness `wall_thickness`, floor thickness
/// `floor_thickness` (floor outer face at up = 0).
///
/// CONCAVE — the direct facet backend refuses concave caps (the cap rings
/// must be convex, `facet_sweep.rs`'s `ring_is_convex` gate), so this profile
/// is the battery's typed-refusal fixture for the facet path. The BREP path
/// (`spine_sweep`, caps via `try_attach_plane`) accepts it.
pub fn u_chute(
    width: f64,
    wall_height: f64,
    wall_thickness: f64,
    floor_thickness: f64,
) -> Result<Profile2D, ConstructError> {
    let hw = width / 2.0;
    let iw = hw - wall_thickness;
    if iw <= 0.0 || wall_height <= floor_thickness {
        return Err(ConstructError::InvalidInput);
    }
    let vertices = vec![
        Point2::new(0.0, -hw),
        Point2::new(0.0, hw),
        Point2::new(wall_height, hw),
        Point2::new(wall_height, iw),
        Point2::new(floor_thickness, iw),
        Point2::new(floor_thickness, -iw),
        Point2::new(wall_height, -iw),
        Point2::new(wall_height, -hw),
    ];
    Profile2D::try_closed(ensure_ccw(vertices))
}

/// The convex trapezoid chute: full width `width` at the floor (up = 0),
/// narrowing to `width * top_fraction` at the wall top (up = `wall_height`).
/// Convex, so both realization backends accept it — this is the showcase's
/// working cross-section.
pub fn trapezoid_chute(width: f64, wall_height: f64, top_fraction: f64) -> Result<Profile2D, ConstructError> {
    if !(0.0..=1.0).contains(&top_fraction) || width <= 0.0 || wall_height <= 0.0 {
        return Err(ConstructError::InvalidInput);
    }
    let hw = width / 2.0;
    let ht = hw * top_fraction;
    let vertices = vec![
        Point2::new(0.0, -hw),
        Point2::new(wall_height, -ht),
        Point2::new(wall_height, ht),
        Point2::new(0.0, hw),
    ];
    Profile2D::try_closed(ensure_ccw(vertices))
}

/// A regular `n`-gon of circumradius `radius`, first vertex at angle `phase`.
pub fn regular_polygon(radius: f64, n: usize, phase: f64) -> Result<Profile2D, ConstructError> {
    if n < 3 || radius <= 0.0 {
        return Err(ConstructError::InvalidInput);
    }
    let vertices = (0..n)
        .map(|i| {
            let t = phase + 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            Point2::new(radius * t.cos(), radius * t.sin())
        })
        .collect();
    Profile2D::try_closed(ensure_ccw(vertices))
}

/// Reverses the ring if the shoelace signed area is negative, so every
/// builder delivers CCW regardless of how the vertices were written down.
fn ensure_ccw(vertices: Vec<Point2>) -> Vec<Point2> {
    let n = vertices.len();
    let area: f64 = (0..n)
        .map(|i| {
            let a = vertices[i];
            let b = vertices[(i + 1) % n];
            a.x * b.y - b.x * a.y
        })
        .sum();
    if area < 0.0 {
        vertices.into_iter().rev().collect()
    } else {
        vertices
    }
}
