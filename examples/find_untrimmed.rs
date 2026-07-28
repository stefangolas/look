//! Find faces that meshed their whole surface instead of their trimmed region.
//!
//! `PolyBoundary::new` falls back to the entire surface parameter rectangle
//! when `loop_orientation` finds no positively-oriented closed loop. The face
//! then meshes everything, which is why `00009190` renders as a handful of
//! giant lens shapes where OCCT renders a submarine hull.
//!
//! Such a face is identifiable without timing anything: its mesh extends far
//! beyond the boundary curves that were supposed to bound it. Comparing those
//! two bounding boxes costs nothing and gives an exact list.
//!
//! Reports STEP entity IDs, because the point is to extract one of these
//! shells into a small fixture that can be iterated on in milliseconds.
//!
//! ```console
//! cargo run --release --example find_untrimmed -- MODEL.step
//! ```

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};
use truck_topology::compress::{CompressedEdgeIndex, CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, truck_stepio::r#in::step_geometry::Curve3D, Surface>;

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;

/// How much larger than its own *shell* a face's mesh may be before it is
/// treated as untrimmed.
///
/// Comparing a face against its own boundary curves looks like the obvious
/// test and is not: a degenerate face whose boundary collapses to nearly a
/// point divides by almost zero and reports a huge ratio while meshing to
/// something perfectly small. Shell #160619 scored 389x that way and renders
/// identically to OCCT.
///
/// A face cannot legitimately be larger than the solid it belongs to, so
/// measuring against the shell has no such failure mode.
const UNTRIMMED_RATIO: f64 = 1.5;

fn single_face_shell(shell: &Cshell, index: usize) -> Cshell {
    let face = &shell.faces[index];
    let mut edges = Vec::new();
    let mut seen = std::collections::HashMap::<usize, usize>::new();
    let mut boundaries = Vec::with_capacity(face.boundaries.len());
    for wire in &face.boundaries {
        let mut rewired = Vec::with_capacity(wire.len());
        for edge in wire {
            if edge.index >= shell.edges.len() {
                continue;
            }
            let remapped = *seen.entry(edge.index).or_insert_with(|| {
                edges.push(shell.edges[edge.index].clone());
                edges.len() - 1
            });
            rewired.push(CompressedEdgeIndex {
                index: remapped,
                orientation: edge.orientation,
            });
        }
        boundaries.push(rewired);
    }
    CompressedShell {
        vertices: Vec::new(),
        edges,
        faces: vec![CompressedFace {
            boundaries,
            orientation: face.orientation,
            surface: face.surface.clone(),
        }],
    }
}

fn diagonal(bounds: &BoundingBox<Point3>) -> f64 {
    if bounds.is_empty() {
        0.0
    } else {
        bounds.diameter()
    }
}

fn main() -> anyhow::Result<()> {
    let model = env::args()
        .nth(1)
        .expect("usage: find_untrimmed MODEL.step");
    let bytes = std::fs::read(&model)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    drop(text);
    drop(bytes);
    let table = Table::from_owned_data_section(section);

    let shells = table
        .shell
        .iter()
        .filter_map(|(id, shell)| table.to_compressed_shell(shell).ok().map(|cs| (*id, cs)))
        .collect::<Vec<_>>();

    let mut model_box = BoundingBox::<Point3>::new();
    for (_, shell) in &shells {
        for vertex in &shell.vertices {
            model_box.push(*vertex);
        }
    }
    let scaled = model_box.diameter() * RELATIVE_TOLERANCE;
    let tolerance = if scaled.is_finite() && scaled > 0.0 {
        scaled
    } else {
        DEGENERATE_TOLERANCE
    };
    println!(
        "{} shells, {} faces, model diameter {:.4}, tolerance {:.6}",
        shells.len(),
        shells.iter().map(|(_, s)| s.faces.len()).sum::<usize>(),
        model_box.diameter(),
        tolerance
    );

    // shell entity id -> (untrimmed faces, total faces, worst ratio)
    let mut per_shell: Vec<(u64, usize, usize, f64)> = Vec::new();
    let mut untrimmed_total = 0usize;
    let mut face_total = 0usize;

    for (entity, shell) in &shells {
        let mut untrimmed = 0usize;
        let mut worst = 0.0f64;
        // The solid this face belongs to. Its own vertices bound it honestly,
        // whatever the trimming code later decides.
        let mut shell_box = BoundingBox::<Point3>::new();
        for vertex in &shell.vertices {
            shell_box.push(*vertex);
        }
        let shell_size = diagonal(&shell_box);
        if shell_size <= 0.0 {
            continue;
        }
        for index in 0..shell.faces.len() {
            face_total += 1;
            let meshed = single_face_shell(shell, index).robust_triangulation(tolerance);
            let Some(polygon) = meshed.faces[0].surface.as_ref() else {
                continue;
            };
            let mut mesh = BoundingBox::<Point3>::new();
            for point in polygon.positions() {
                mesh.push(*point);
            }
            let ratio = diagonal(&mesh) / shell_size;
            if ratio > UNTRIMMED_RATIO {
                untrimmed += 1;
                untrimmed_total += 1;
                worst = f64::max(worst, ratio);
            }
        }
        if untrimmed > 0 {
            per_shell.push((*entity, untrimmed, shell.faces.len(), worst));
        }
    }

    println!(
        "\n{untrimmed_total} of {face_total} faces meshed beyond {UNTRIMMED_RATIO}x their own \
         boundary, across {} shells",
        per_shell.len()
    );

    // Smallest affected shells first: the point is a fixture, so the best
    // candidate is the one with the fewest faces that still shows the bug.
    per_shell.sort_by_key(|(_, untrimmed, total, _)| (*total, std::cmp::Reverse(*untrimmed)));
    println!("\nbest fixture candidates (smallest shells showing the bug):");
    println!(
        "  {:>12} {:>10} {:>8} {:>12}",
        "shell #id", "untrimmed", "faces", "worst ratio"
    );
    for (entity, untrimmed, total, worst) in per_shell.iter().take(15) {
        println!("  {entity:>12} {untrimmed:>10} {total:>8} {worst:>12.1}");
    }
    Ok(())
}
