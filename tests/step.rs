use std::path::PathBuf;

use look::{config::UpAxis, scene::compile_scene, timing::Timings};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// STEP carries no triangles, so this covers the whole boundary-representation
/// path: parse, resolve the entity graph, and tessellate trimmed surfaces.
#[test]
fn tessellates_step_boundary_representation() {
    let mut timings = Timings::default();
    let scene = compile_scene(&fixture("bracket.step"), UpAxis::Y, &mut timings)
        .expect("bracket.step should tessellate");

    assert_eq!(scene.geometries.len(), 1);
    assert_eq!(scene.instances.len(), 1);

    let geometry = &scene.geometries[0];
    assert!(
        geometry.indices.len() % 3 == 0,
        "index buffer must be whole triangles, got {}",
        geometry.indices.len()
    );
    // The fixture is a block with a boss and a bore. A correct tessellation of
    // its cylindrical faces needs far more than the twelve triangles a bare
    // box would produce; this catches silently dropped curved faces.
    assert!(
        geometry.indices.len() / 3 > 100,
        "expected a detailed tessellation, got {} triangles",
        geometry.indices.len() / 3
    );
    assert!(
        geometry.indices.iter().all(|index| (*index as usize) < geometry.vertices.len()),
        "every index must address a real vertex"
    );

    // A bounded solid must have finite, non-degenerate extents.
    let min = scene.bounds.min;
    let max = scene.bounds.max;
    for axis in 0..3 {
        assert!(
            min[axis].is_finite() && max[axis].is_finite(),
            "bounds must be finite, got {min:?}..{max:?}"
        );
        assert!(
            max[axis] > min[axis],
            "axis {axis} is degenerate: {}..{}",
            min[axis],
            max[axis]
        );
    }

    // Normals are generated per triangle, so they must be unit length.
    for vertex in geometry.vertices.iter().take(500) {
        let length = (vertex.normal[0] * vertex.normal[0]
            + vertex.normal[1] * vertex.normal[1]
            + vertex.normal[2] * vertex.normal[2])
            .sqrt();
        assert!(
            (length - 1.0).abs() < 1.0e-3,
            "normal should be unit length, got {length}"
        );
    }
}

#[test]
fn step_and_stp_extensions_are_both_accepted() {
    let mut timings = Timings::default();
    // Same bytes under the alternate extension the CAD world also uses.
    let source = fixture("bracket.step");
    let alias = std::env::temp_dir().join("look-extension-alias.stp");
    std::fs::copy(&source, &alias).expect("copy fixture");

    let scene = compile_scene(&alias, UpAxis::Y, &mut timings).expect(".stp should be accepted");
    assert!(!scene.geometries[0].indices.is_empty());

    let _ = std::fs::remove_file(&alias);
}
