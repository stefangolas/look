use std::path::PathBuf;

use look::{config::UpAxis, scene::compile_scene, step::part21, timing::Timings};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Read a STEP file the way the renderer does, applying the same Latin-1
/// rescue so a comparison runs on the text the reader will actually be given.
fn read_step(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("STEP file should be readable");
    String::from_utf8(bytes.clone())
        .unwrap_or_else(|_| bytes.iter().map(|&byte| byte as char).collect())
}

/// look reads the exchange structure with its own Part 21 reader because
/// ruststep's nom grammar dominates STEP wall clock. That is only sound while
/// the two agree, so the syntax trees are compared outright rather than the
/// renders being spot-checked.
///
/// Returns how the two readers differed, or `None` when they agree. A file
/// only this reader rejects is not a divergence: those fall back to ruststep
/// and still render, so it is reported by the caller but does not fail.
fn compare_with_ruststep(path: &std::path::Path) -> Option<String> {
    let text = read_step(path);
    let ours = match part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(error) => return Some(format!("part21 declined, falling back: {error}")),
    };
    let theirs = match ruststep::parser::parse(&text) {
        Ok(exchange) => exchange,
        // Files the fork's ruststep cannot read at all are outside what this
        // comparison can say anything about.
        Err(_) => return None,
    };
    if ours.data != theirs.data {
        return Some("data section differs from ruststep".to_string());
    }
    if ours.header != theirs.header {
        return Some("header differs from ruststep".to_string());
    }
    None
}

#[test]
fn part21_agrees_with_ruststep() {
    assert_eq!(compare_with_ruststep(&fixture("bracket.step")), None);
}

/// The repository fixture is one exporter's dialect. Point `LOOK_STEP_CORPUS`
/// at a directory of real CAD files to hold the reader to the same equality
/// across all of them; the NIST MBE PMI set is what this was developed against.
///
/// Every file is checked before anything fails, because the useful output over
/// a real corpus is the set of distinct disagreements, not the first one.
#[test]
fn part21_agrees_with_ruststep_across_a_corpus() {
    let Some(root) = std::env::var_os("LOOK_STEP_CORPUS") else {
        return;
    };

    let mut files = Vec::new();
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
            if matches!(extension.as_deref(), Some("step" | "stp")) {
                files.push(path);
            }
        }
    }
    assert!(!files.is_empty(), "LOOK_STEP_CORPUS held no STEP files");

    let mut divergences = Vec::new();
    let mut fallbacks = Vec::new();
    for path in &files {
        if let Some(reason) = compare_with_ruststep(path) {
            let name = path.file_name().unwrap_or(path.as_os_str());
            if reason.starts_with("part21 declined") {
                fallbacks.push(format!("{}: {reason}", name.to_string_lossy()));
            } else {
                divergences.push(format!("{}: {reason}", name.to_string_lossy()));
            }
        }
    }

    eprintln!(
        "part21 matched ruststep on {} of {} corpus files ({} fell back)",
        files.len() - divergences.len() - fallbacks.len(),
        files.len(),
        fallbacks.len()
    );
    for fallback in &fallbacks {
        eprintln!("  fallback: {fallback}");
    }

    assert!(
        divergences.is_empty(),
        "part21 read {} of {} files differently from ruststep:\n  {}",
        divergences.len(),
        files.len(),
        divergences.join("\n  ")
    );
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
