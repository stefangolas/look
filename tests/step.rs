use std::path::PathBuf;

use look::{config::UpAxis, scene::compile_scene, step::part21, timing::Timings};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// look reads the exchange structure with its own Part 21 reader because
/// ruststep's nom grammar dominates STEP wall clock. That is only sound while
/// the two agree, so the syntax trees are compared outright rather than the
/// renders being spot-checked.
fn assert_matches_ruststep(path: &std::path::Path) {
    let bytes = std::fs::read(path).expect("fixture should be readable");
    // Same Latin-1 rescue the renderer applies, so the comparison runs on the
    // text the reader will actually be given.
    let text = String::from_utf8(bytes.clone())
        .unwrap_or_else(|_| bytes.iter().map(|&byte| byte as char).collect());
    let ours = part21::parse(&text)
        .unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
    let theirs = ruststep::parser::parse(&text)
        .unwrap_or_else(|error| panic!("{} should parse under ruststep: {error}", path.display()));
    assert_eq!(
        ours.data,
        theirs.data,
        "data section differs from ruststep for {}",
        path.display()
    );
    assert_eq!(
        ours.header,
        theirs.header,
        "header differs from ruststep for {}",
        path.display()
    );
}

#[test]
fn part21_agrees_with_ruststep() {
    assert_matches_ruststep(&fixture("bracket.step"));
}

/// The repository fixture is one exporter's dialect. Point `LOOK_STEP_CORPUS`
/// at a directory of real CAD files to hold the reader to the same equality
/// across all of them; the NIST MBE PMI set is what this was developed against.
#[test]
fn part21_agrees_with_ruststep_across_a_corpus() {
    let Some(root) = std::env::var_os("LOOK_STEP_CORPUS") else {
        return;
    };

    let mut checked = 0;
    let mut pending = vec![PathBuf::from(root)];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("corpus directory should be readable") {
            let path = entry.expect("corpus entry should be readable").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase);
            if !matches!(extension.as_deref(), Some("step" | "stp")) {
                continue;
            }
            assert_matches_ruststep(&path);
            checked += 1;
        }
    }

    assert!(checked > 0, "LOOK_STEP_CORPUS held no STEP files");
    eprintln!("part21 matched ruststep on {checked} corpus files");
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
        geometry
            .indices
            .iter()
            .all(|index| (*index as usize) < geometry.vertices.len()),
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
