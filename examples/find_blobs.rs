//! Find the shells that mesh far larger than they are.
//!
//! This is the cheap sibling of `find_untrimmed`. That tool tessellates every
//! face in isolation, which costs 24,000 separate triangulations on a real
//! model and runs for minutes. The faces that actually ruin a render are not
//! subtle: they mesh to a large fraction of the *whole model* while belonging
//! to a small shell. Tessellating each shell once — which is exactly what the
//! renderer does, and takes seconds — is enough to name them.
//!
//! Both boxes are measured by sampling edge curves, never by vertices alone. A
//! shell bounded by full circles has one topological vertex per circle, so a
//! vertex-only measure understates a washer by 70x and reports every face on
//! it as enormous. That mistake is what made the previous count untrustworthy.
//!
//! ```console
//! cargo run --release --example find_blobs -- MODEL.step
//! ```

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};
use truck_topology::compress::CompressedShell;

type Cshell = CompressedShell<Point3, truck_stepio::r#in::step_geometry::Curve3D, Surface>;

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

/// How much larger than itself a shell may mesh before it is a blob.
///
/// A correct mesh is bounded by the solid it came from, so anything above 1 is
/// already wrong; the margin only absorbs curve sampling on a coarse tolerance.
const BLOB_RATIO: f64 = 1.5;

fn push_shell_extent(bounds: &mut BoundingBox<Point3>, shell: &Cshell) {
    for vertex in &shell.vertices {
        bounds.push(*vertex);
    }
    for edge in &shell.edges {
        let (start, end) = edge.curve.range_tuple();
        for step in 0..=EDGE_SAMPLES {
            let t = start + (end - start) * f64::from(step) / f64::from(EDGE_SAMPLES);
            bounds.push(edge.curve.subs(t));
        }
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
    let model = env::args().nth(1).expect("usage: find_blobs MODEL.step");
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
        .filter_map(|(id, shell)| table.to_compressed_shell(*id, shell).ok().map(|cs| (*id, cs)))
        .collect::<Vec<_>>();

    let mut model_box = BoundingBox::<Point3>::new();
    for (_, shell) in &shells {
        push_shell_extent(&mut model_box, shell);
    }
    let model_size = model_box.diameter();
    let scaled = model_size * RELATIVE_TOLERANCE;
    // An explicit tolerance lets one extracted shell be meshed at the tolerance
    // its parent model would have imposed. That is the whole experiment: the
    // same geometry is correct at its own scale and a blob at the model's.
    let tolerance = match env::args().nth(2).and_then(|arg| arg.parse::<f64>().ok()) {
        Some(explicit) => explicit,
        None if scaled.is_finite() && scaled > 0.0 => scaled.max(MINIMUM_TOLERANCE),
        None => DEGENERATE_TOLERANCE,
    };
    println!(
        "{} shells, {} faces, model diameter {model_size:.4}, tolerance {tolerance:.6}",
        shells.len(),
        shells.iter().map(|(_, s)| s.faces.len()).sum::<usize>(),
    );

    // entity id, ratio, own size, meshed size, faces
    let mut blobs: Vec<(u64, f64, f64, f64, usize)> = Vec::new();
    for (entity, shell) in &shells {
        let mut own = BoundingBox::<Point3>::new();
        push_shell_extent(&mut own, shell);
        let own_size = diagonal(&own);
        if own_size <= 0.0 {
            continue;
        }
        let meshed = shell.robust_triangulation(tolerance).to_polygon();
        let mut mesh_box = BoundingBox::<Point3>::new();
        for point in meshed.positions() {
            mesh_box.push(*point);
        }
        let mesh_size = diagonal(&mesh_box);
        let ratio = mesh_size / own_size;
        if ratio > BLOB_RATIO {
            blobs.push((*entity, ratio, own_size, mesh_size, shell.faces.len()));
        }
    }

    blobs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!(
        "\n{} of {} shells mesh beyond {BLOB_RATIO}x their own extent",
        blobs.len(),
        shells.len()
    );
    println!(
        "  {:>12} {:>10} {:>12} {:>12} {:>7}",
        "shell #id", "ratio", "own size", "meshed size", "faces"
    );
    for (entity, ratio, own, mesh, faces) in blobs.iter().take(20) {
        println!("  {entity:>12} {ratio:>10.1} {own:>12.5} {mesh:>12.5} {faces:>7}");
    }

    // The smallest blob shell is the best reproducer: fewest faces that still
    // shows the defect, so the fixture stays readable.
    if let Some((entity, ..)) = blobs.iter().min_by_key(|(_, _, _, _, faces)| *faces) {
        let faces = blobs.iter().find(|b| b.0 == *entity).unwrap().4;
        println!("\nsmallest reproducer: shell #{entity} with {faces} faces");
    }
    Ok(())
}
