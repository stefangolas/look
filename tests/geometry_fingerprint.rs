//! A fingerprint of what the kernel actually produces for a real part.
//!
//! Every other test in this repo that touches a STEP file asserts something
//! *structural*: that a scene has one geometry, that an index buffer is a whole
//! number of triangles, that it is not empty. Those hold for a wildly wrong
//! mesh. The V9 gate in `loop/verify.py` was built on them and was measured
//! passing with `truck_base::TOLERANCE` loosened from 1e-6 to 1e-1 -- a change
//! that should be visible from orbit.
//!
//! These tests assert on the mesh itself: how many triangles came out, how many
//! vertices, and where the thing sits in space. That is what moves when a
//! tolerance, a healing pass, or a tessellation predicate changes, and it is
//! what the BG-TOL-001 migration shards need watched, since they are supposed
//! to change no threshold at all.
//!
//! **When one of these fails, do not update the number until you know why it
//! moved.** A changed triangle count is the finding. It is only noise if you
//! can say which deliberate change produced it.

use std::path::PathBuf;

use look::{config::UpAxis, scene::compile_scene, timing::Timings};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

struct Fingerprint {
    triangles: usize,
    vertices: usize,
    min: [f32; 3],
    max: [f32; 3],
}

fn fingerprint(name: &str) -> Fingerprint {
    let mut timings = Timings::default();
    let scene = compile_scene(&fixture(name), UpAxis::Y, &mut timings)
        .unwrap_or_else(|e| panic!("{name} should tessellate: {e:?}"));

    let mut triangles = 0usize;
    let mut vertices = 0usize;
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for g in &scene.geometries {
        triangles += g.indices.len() / 3;
        vertices += g.vertices.len();
        for v in &g.vertices {
            for a in 0..3 {
                min[a] = min[a].min(v.position[a]);
                max[a] = max[a].max(v.position[a]);
            }
        }
    }
    Fingerprint {
        triangles,
        vertices,
        min,
        max,
    }
}

/// Relative to the part's own extent, deliberately. These two fixtures are
/// modelled in different units -- bracket.step spans tens of millimetres,
/// washer_circular_edges.step hundredths of a metre -- so a single absolute
/// epsilon is either meaningless on one or vacuous on the other. That is the
/// same scale-relative argument BG-TOL-001 is about, and it applies to the
/// tests too.
fn assert_bounds_near(f: &Fingerprint, min: [f32; 3], max: [f32; 3], what: &str) {
    let mut diag = 0.0f32;
    for a in 0..3 {
        diag = diag.max(max[a] - min[a]);
    }
    let tol = diag * 1.0e-3; // H-3: a thousandth of the part's own extent, a display tolerance
    for a in 0..3 {
        for (got, want, side) in [(f.min[a], min[a], "min"), (f.max[a], max[a], "max")] {
            let d = (got - want).abs();
            assert!(
                d < tol,
                "{what} {side}[{a}] moved: got {got}, want {want} (delta {d}, tol {tol})"
            );
        }
    }
}

#[test]
fn bracket_tessellates_to_a_known_mesh() {
    let f = fingerprint("bracket.step");
    assert_eq!(f.triangles, 1814, "bracket triangle count moved");
    assert_eq!(f.vertices, 5442, "bracket vertex count moved");
    assert_bounds_near(&f, [-9.0, -9.0, 0.0], [60.0, 40.0, 22.0], "bracket");
}

#[test]
fn washer_with_circular_edges_tessellates_to_a_known_mesh() {
    // The fixture that BG-CE-006's circle work is about, and modelled in
    // metres -- 9,518 triangles over a part 10mm across, which is why its
    // triangle count is a sharp instrument for a tolerance change.
    let f = fingerprint("washer_circular_edges.step");
    assert_eq!(f.triangles, 9518, "washer triangle count moved");
    assert_eq!(f.vertices, 28554, "washer vertex count moved");
    assert_bounds_near(
        &f,
        [-0.016, -0.004_999_012, -0.02],
        [-0.006, 0.004_999_012, -0.0175],
        "washer",
    );
}
