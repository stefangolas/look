//! Attribute STEP tessellation cost to individual faces.
//!
//! Two models exceed five minutes and one takes 230 seconds, and the plan
//! carries two competing explanations for it:
//!
//! 1. `PolyBoundary::include` walks every segment of every boundary loop, and
//!    is called once per parameter-grid point and again per candidate
//!    triangle. That makes a face cost O(U*V*B + T*B) in the boundary length
//!    `B`, which nothing bounds.
//! 2. `robust_triangulation` projects every boundary point with up to four
//!    Newton solves of a hundred iterations. Offset surfaces have no
//!    closed-form inverse, so they are exactly the case that converges badly —
//!    and offset surfaces are newly tessellated rather than newly dropped.
//!
//! The two predict different things. Under (1) time concentrates in a few
//! faces with a large `B` regardless of surface type; under (2) it concentrates
//! on offset surfaces roughly independently of `B`. This measures both per
//! face, so the shape of the answer picks one.
//!
//! Structural columns — surface kind, boundary length, edge count, output
//! triangles — are deterministic and mean the same thing whatever the machine
//! is doing. Timings do not: read `--trust-timings` before quoting one.
//!
//! ```console
//! cargo run --release --example face_profile -- MODEL.step --top 25
//! ```

use std::{collections::HashMap, env, path::PathBuf, time::Instant};

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{ElementarySurface, Surface},
};
use truck_topology::compress::{CompressedEdgeIndex, CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, truck_stepio::r#in::step_geometry::Curve3D, Surface>;

/// Matches `src/step.rs`. A shell-local tolerance would mesh a part split into
/// many small shells far finer than the same part as one shell.
const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;

struct FaceRecord {
    shell: usize,
    face: usize,
    kind: &'static str,
    boundary_points: usize,
    boundary_edges: usize,
    triangles: usize,
    dropped: bool,
    empty: bool,
    micros: u128,
}

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(elementary) => match elementary {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(_) => "swept",
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

/// A shell holding one face and only the edges that face uses.
///
/// This is what makes per-face timing possible without touching the fork:
/// `cshell_tessellation` tessellates every edge in the shell before any face,
/// so handing it the whole shell once per face would be quadratic. It never
/// reads `vertices` — only the edge curves and the face's own boundaries — so
/// those are left empty rather than cloned per face.
fn single_face_shell(shell: &Cshell, index: usize) -> Cshell {
    let face = &shell.faces[index];
    let mut edges = Vec::new();
    let mut seen: HashMap<usize, usize> = HashMap::new();
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

/// Total polyline points across a face's boundary — the `B` that `include`
/// walks on every call — plus how many of the face's edge references point
/// past the end of the shell's edge list.
///
/// That last count should be structurally impossible and is not: see
/// `shell_edges` in the fork, which reserves an index in `eidx_map` before
/// attempting the conversion that decides whether the edge exists at all.
/// `cshell_tessellation` hides it by resolving edges through `edges.get(i)?`
/// inside a `filter_map`, so a dangling reference silently shortens the
/// boundary instead of failing.
fn boundary_points(shell: &Cshell, index: usize, tolerance: f64) -> (usize, usize, usize) {
    let face = &shell.faces[index];
    let mut points = 0;
    let mut edges = 0;
    let mut dangling = 0;
    for wire in &face.boundaries {
        for edge in wire {
            let Some(entry) = shell.edges.get(edge.index) else {
                dangling += 1;
                continue;
            };
            let curve = &entry.curve;
            let poly = PolylineCurve::from_curve(curve, curve.range_tuple(), tolerance);
            points += poly.len();
            edges += 1;
        }
    }
    (points, edges, dangling)
}

fn main() -> anyhow::Result<()> {
    let mut models = Vec::<PathBuf>::new();
    let mut top = 20usize;
    let mut budget_s = 900u64;
    let mut trust_timings = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--top" => top = args.next().unwrap_or_default().parse()?,
            "--budget" => budget_s = args.next().unwrap_or_default().parse()?,
            "--trust-timings" => trust_timings = true,
            other => models.push(PathBuf::from(other)),
        }
    }
    if models.is_empty() {
        anyhow::bail!("usage: face_profile MODEL.step [--top N] [--budget SECONDS]");
    }

    for model in &models {
        println!("\n=== {} ===", model.display());
        let bytes = std::fs::read(model)?;
        println!("file: {:.1} MB", bytes.len() as f64 / 1e6);

        let text = std::str::from_utf8(&bytes)
            .map(std::borrow::Cow::Borrowed)
            .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());

        let started = Instant::now();
        let mut exchange = match look::step::part21::parse(&text) {
            Ok(exchange) => exchange,
            Err(_) => ruststep::parser::parse(&text)
                .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
        };
        println!("parse: {:.1} s", started.elapsed().as_secs_f64());

        let section = exchange.data.swap_remove(0);
        // Release the text and the file before building the table, so peak
        // residency here is the tree plus the table rather than all four.
        drop(text);
        drop(bytes);
        let started = Instant::now();
        let table = Table::from_owned_data_section(section);
        println!("table: {:.1} s", started.elapsed().as_secs_f64());

        let started = Instant::now();
        let shells = table
            .shell
            .values()
            .filter_map(|shell| table.to_compressed_shell(shell).ok())
            .collect::<Vec<_>>();
        println!(
            "resolved {} shells in {:.1} s",
            shells.len(),
            started.elapsed().as_secs_f64()
        );

        let mut model_box = BoundingBox::<Point3>::new();
        for shell in &shells {
            for vertex in &shell.vertices {
                model_box.push(*vertex);
            }
        }
        let diameter = model_box.diameter();
        let scaled = diameter * RELATIVE_TOLERANCE;
        let tolerance = if scaled.is_finite() && scaled > 0.0 {
            scaled
        } else {
            DEGENERATE_TOLERANCE
        };
        let faces: usize = shells.iter().map(|shell| shell.faces.len()).sum();
        println!("diameter {diameter:.6}, tolerance {tolerance:.9}, {faces} faces");

        // Structural pass. Nothing here depends on machine state.
        let started = Instant::now();
        let mut by_kind: HashMap<&'static str, (usize, usize)> = HashMap::new();
        let mut dangling_refs = 0usize;
        let mut faces_with_dangling = 0usize;
        let mut faces_wholly_dangling = 0usize;
        let mut shells_with_dangling = 0usize;
        for shell in &shells {
            let before = dangling_refs;
            for index in 0..shell.faces.len() {
                let (points, edges, dangling) = boundary_points(shell, index, tolerance);
                if dangling > 0 {
                    faces_with_dangling += 1;
                    dangling_refs += dangling;
                    if edges == 0 {
                        faces_wholly_dangling += 1;
                    }
                }
                let entry = by_kind
                    .entry(surface_kind(&shell.faces[index].surface))
                    .or_default();
                entry.0 += 1;
                entry.1 += points;
            }
            if dangling_refs > before {
                shells_with_dangling += 1;
            }
        }
        println!(
            "\nstructural pass ({:.1} s) — faces and total boundary points by surface kind:",
            started.elapsed().as_secs_f64()
        );
        let mut kinds = by_kind.into_iter().collect::<Vec<_>>();
        kinds.sort_by_key(|(_, (_, points))| std::cmp::Reverse(*points));
        println!(
            "  {:<10} {:>8} {:>14} {:>10}",
            "kind", "faces", "bdry points", "mean B"
        );
        for (kind, (count, points)) in &kinds {
            println!(
                "  {:<10} {:>8} {:>14} {:>10.0}",
                kind,
                count,
                points,
                *points as f64 / *count as f64
            );
        }

        println!(
            "\ndangling edge references: {dangling_refs} across {faces_with_dangling} faces \
             in {shells_with_dangling} of {} shells; {faces_wholly_dangling} faces lost \
             their entire boundary",
            shells.len()
        );
        println!(
            "  (a face referencing an edge index past the end of its own shell's edge list \
             is not possible in well-formed output. Every such reference means the index \
             map and the edge vector disagree, which also means the in-range indices after \
             the first failure address the WRONG edge.)"
        );

        // Timing pass, sequential so that cost lands on the face that caused it.
        println!("\ntiming pass (sequential, budget {budget_s} s)...");
        let mut records = Vec::new();
        let pass_started = Instant::now();
        let mut exhausted = false;
        'outer: for (shell_index, shell) in shells.iter().enumerate() {
            for index in 0..shell.faces.len() {
                if pass_started.elapsed().as_secs() >= budget_s {
                    exhausted = true;
                    break 'outer;
                }
                let (points, edges, _) = boundary_points(shell, index, tolerance);
                let one = single_face_shell(shell, index);
                let started = Instant::now();
                let meshed = one.robust_triangulation(tolerance);
                let micros = started.elapsed().as_micros();
                let surface = &meshed.faces[0].surface;
                records.push(FaceRecord {
                    shell: shell_index,
                    face: index,
                    kind: surface_kind(&shell.faces[index].surface),
                    boundary_points: points,
                    boundary_edges: edges,
                    triangles: surface.as_ref().map(|m| m.tri_faces().len()).unwrap_or(0),
                    dropped: surface.is_none(),
                    empty: surface.as_ref().is_some_and(|m| m.tri_faces().is_empty()),
                    micros,
                });
            }
        }
        report(
            &records,
            top,
            exhausted,
            pass_started.elapsed().as_secs_f64(),
        );
        if !trust_timings {
            println!(
                "\nNOTE: timings above are attribution only. Re-run with the machine \
                 quiet and pass --trust-timings before quoting any absolute number."
            );
        }
    }
    Ok(())
}

fn report(records: &[FaceRecord], top: usize, exhausted: bool, elapsed: f64) {
    if records.is_empty() {
        println!("no faces timed");
        return;
    }
    let total: u128 = records.iter().map(|r| r.micros).sum();
    println!(
        "timed {} faces in {:.1} s{}",
        records.len(),
        elapsed,
        if exhausted { " (budget exhausted)" } else { "" }
    );

    let mut ranked = records.iter().collect::<Vec<_>>();
    ranked.sort_by_key(|r| std::cmp::Reverse(r.micros));
    println!("\ntop {top} faces by time:");
    println!(
        "  {:>7} {:>7} {:<10} {:>10} {:>7} {:>9} {:>7} {:>6}",
        "shell", "face", "kind", "ms", "share", "bdry pts", "edges", "tris"
    );
    for record in ranked.iter().take(top) {
        println!(
            "  {:>7} {:>7} {:<10} {:>10.1} {:>6.1}% {:>9} {:>7} {:>6}{}",
            record.shell,
            record.face,
            record.kind,
            record.micros as f64 / 1000.0,
            100.0 * record.micros as f64 / total as f64,
            record.boundary_points,
            record.boundary_edges,
            record.triangles,
            if record.dropped {
                "  DROPPED"
            } else if record.empty {
                "  EMPTY"
            } else {
                ""
            }
        );
    }

    // How concentrated is the cost? This is the question that separates a few
    // pathological faces from a cost spread across the whole model.
    let mut cumulative = 0u128;
    for (rank, record) in ranked.iter().enumerate() {
        cumulative += record.micros;
        if cumulative * 2 >= total {
            println!(
                "\nhalf of all tessellation time is in the slowest {} of {} faces ({:.2}%)",
                rank + 1,
                records.len(),
                100.0 * (rank + 1) as f64 / records.len() as f64
            );
            break;
        }
    }

    println!("\nby surface kind:");
    let mut by_kind: HashMap<&'static str, (usize, u128, usize, usize, usize)> = HashMap::new();
    for record in records {
        let entry = by_kind.entry(record.kind).or_default();
        entry.0 += 1;
        entry.1 += record.micros;
        entry.2 += record.boundary_points;
        entry.3 += usize::from(record.dropped);
        entry.4 += usize::from(record.empty);
    }
    let mut kinds = by_kind.into_iter().collect::<Vec<_>>();
    kinds.sort_by_key(|(_, (_, micros, ..))| std::cmp::Reverse(*micros));
    println!(
        "  {:<10} {:>7} {:>10} {:>7} {:>10} {:>9} {:>8} {:>7}",
        "kind", "faces", "ms", "share", "us/face", "mean B", "dropped", "empty"
    );
    for (kind, (count, micros, points, dropped, empty)) in &kinds {
        println!(
            "  {:<10} {:>7} {:>10.1} {:>6.1}% {:>10.0} {:>9.0} {:>8} {:>7}",
            kind,
            count,
            *micros as f64 / 1000.0,
            100.0 * *micros as f64 / total as f64,
            *micros as f64 / *count as f64,
            *points as f64 / *count as f64,
            dropped,
            empty
        );
    }

    let dropped: usize = records.iter().filter(|r| r.dropped).count();
    let empty: usize = records.iter().filter(|r| r.empty).count();
    println!(
        "\nface loss: {dropped} produced no surface at all, {empty} produced an empty mesh \
         (the second kind is invisible to the warning in step.rs, which counts only the first)"
    );
}
