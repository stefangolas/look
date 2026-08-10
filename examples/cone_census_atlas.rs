//! Canonical Atlas Classification Tool for Cone MeshedToNothing Failures.
//!
//! Consumes the shared `TraversalSemantics` and `project_boundary_curve` API from
//! `truck_meshalgo::tessellation::domain::projection` to audit failing cone faces.
//! Emits raw intermediate edge traversal counts before assigning canonical Atlas Cell labels.

use std::collections::HashMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::domain::projection::{
    TraversalSemantics, project_boundary_curve,
};
use truck_stepio::r#in::{
    Table,
    step_geometry::{ElementarySurface, Surface},
};
use truck_topology::compress::{CompressedEdge, CompressedFace};

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AtlasCell {
    ApexDisk,
    ApexSector,
    TruncatedSector,
    TruncatedAnnulus,
    ApexDiskWithHoles,
    RegularDiskWithHoles,
    ArrangementRequired,
    InconsistentBoundary,
    UnresolvedProjection,
}

impl AtlasCell {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ApexDisk => "C-APEX-DISK",
            Self::ApexSector => "C-APEX-SECTOR",
            Self::TruncatedSector => "C-TRUNC-SECTOR",
            Self::TruncatedAnnulus => "C-TRUNC-ANNULUS",
            Self::ApexDiskWithHoles => "C-APEX-DISK-H",
            Self::RegularDiskWithHoles => "C-REGULAR-DISK-H",
            Self::ArrangementRequired => "C-ARRANGEMENT-REQUIRED",
            Self::InconsistentBoundary => "C-INCONSISTENT",
            Self::UnresolvedProjection => "C-UNRESOLVED-PROJECTION",
        }
    }

    pub fn missing_capability(&self) -> &'static str {
        match self {
            Self::ApexDisk => "None (Implemented)",
            Self::ApexSector => "Sector Domain Realizer",
            Self::TruncatedSector => "Sector Domain Realizer",
            Self::TruncatedAnnulus => "Annulus Selection / Seam Pair Realizer",
            Self::ApexDiskWithHoles => "Hole Containment & Partition Realizer",
            Self::RegularDiskWithHoles => "Hole Containment & Partition Realizer",
            Self::ArrangementRequired => "Planar Arrangement Construction",
            Self::InconsistentBoundary => "Source Topology Repair / Rejection",
            Self::UnresolvedProjection => "Projection Robustness / Inversion Gate",
        }
    }
}

#[derive(Debug, Clone)]
struct FaceEvidence {
    face_id: String,
    closed_loops: usize,
    open_chains: usize,
    traversal_full_period: usize,
    traversal_ordinary: usize,
    traversal_degenerate: usize,
    projection_failed: usize,
    deck_winding: i64,
    v_span: f64,
    has_apex: bool,
    generator_sides: usize,
    circular_arcs: usize,
    cell: AtlasCell,
}

fn solve_cone_apex_u(surface: &Surface) -> Option<(f64, f64)> {
    let vp = surface.v_period()?;
    let w = |u: f64| -> Vector3 {
        let p0 = surface.subs(u, 0.0);
        let p_half = surface.subs(u, 0.5 * vp);
        p0 - p_half
    };

    let w0 = w(0.0);
    let w1 = w(1.0);
    let dw = w1 - w0;
    let dw2 = dw.magnitude2();

    if dw2 < 1e-12 {
        return None;
    }

    let u_apex = -w0.dot(dw) / dw2;
    let res = w(u_apex).magnitude();
    if res <= 1e-3 {
        Some((u_apex, res))
    } else {
        None
    }
}

fn analyze_cone_face(
    face: &CompressedFace<Surface>,
    edges: &[CompressedEdge<truck_stepio::r#in::step_geometry::Curve3D>],
) -> FaceEvidence {
    let face_id = face
        .provenance
        .best_id()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "#?".into());

    let mut closed_loops = 0;
    let mut open_chains = 0;
    let mut traversal_full_period = 0;
    let mut traversal_ordinary = 0;
    let mut traversal_degenerate = 0;
    let mut projection_failed = 0;
    let mut deck_winding = 0;
    let mut v_min = f64::INFINITY;
    let mut v_max = f64::NEG_INFINITY;
    let mut generator_sides = 0;
    let mut circular_arcs = 0;

    let vp = face
        .surface
        .v_period()
        .unwrap_or(2.0 * std::f64::consts::PI);

    for wire in &face.boundaries {
        if wire.is_empty() {
            continue;
        }

        let mut points_uv = Vec::new();
        for edge_ref in wire {
            if let Some(edge) = edges.get(edge_ref.index) {
                let semantics = TraversalSemantics::resolve(&edge.curve, &face.surface, 1e-4);
                match semantics {
                    TraversalSemantics::FullPeriod { winding, .. } => {
                        traversal_full_period += 1;
                        deck_winding += winding;
                    }
                    TraversalSemantics::DegeneratePoint => traversal_degenerate += 1,
                    _ => traversal_ordinary += 1,
                }

                match project_boundary_curve(&edge.curve, &face.surface, semantics, 1e-3) {
                    Ok(path) => {
                        for sp in path.samples {
                            points_uv.push(sp.uv);
                        }
                    }
                    Err(_) => {
                        projection_failed += 1;
                    }
                }
            }
        }

        if points_uv.is_empty() {
            continue;
        }

        // Continuous parameter lifting of v
        let mut lifted_v = Vec::with_capacity(points_uv.len());
        let mut curr_v = points_uv[0].y;
        lifted_v.push(curr_v);

        for pt in points_uv.iter().skip(1) {
            let raw_v = pt.y;
            let dv = raw_v - (curr_v % vp);
            let k = (-dv / vp).round();
            curr_v = raw_v + k * vp;
            lifted_v.push(curr_v);
        }

        let first = points_uv[0];
        let last = points_uv[points_uv.len() - 1];
        let is_closed = first.distance(last) < 1e-4
            || (first.x - last.x).abs() < 1e-4
            || traversal_full_period > 0;

        if is_closed {
            closed_loops += 1;
        } else {
            open_chains += 1;
        }

        for &v in &lifted_v {
            v_min = v_min.min(v);
            v_max = v_max.max(v);
        }

        for i in 0..points_uv.len().saturating_sub(1) {
            let du = (points_uv[i + 1].x - points_uv[i].x).abs();
            let dv = (points_uv[i + 1].y - points_uv[i].y).abs();
            if dv < 0.1 && du > 1e-4 {
                generator_sides += 1;
            } else if du < 0.1 && dv > 1e-4 {
                circular_arcs += 1;
            }
        }
    }

    let v_span = if v_max.is_finite() && v_min.is_finite() {
        v_max - v_min
    } else if traversal_full_period > 0 {
        vp
    } else {
        0.0
    };

    let apex_info = solve_cone_apex_u(&face.surface);
    let has_apex = apex_info.is_some();

    // Atlas Cell Classification
    let cell = if (closed_loops == 1 || traversal_full_period > 0)
        && open_chains == 0
        && v_span >= 0.75 * vp
        && has_apex
    {
        AtlasCell::ApexDisk
    } else if v_span < 0.75 * vp && has_apex && generator_sides > 0 {
        AtlasCell::ApexSector
    } else if v_span < 0.75 * vp && !has_apex && generator_sides > 0 {
        AtlasCell::TruncatedSector
    } else if closed_loops >= 2 && v_span >= 0.75 * vp && !has_apex {
        AtlasCell::TruncatedAnnulus
    } else if closed_loops >= 2 && has_apex {
        AtlasCell::ApexDiskWithHoles
    } else if closed_loops >= 2 && !has_apex {
        AtlasCell::RegularDiskWithHoles
    } else if open_chains > 2 || (closed_loops > 1 && generator_sides > 0) {
        AtlasCell::ArrangementRequired
    } else if v_span == 0.0 && traversal_full_period == 0 && projection_failed > 0 {
        AtlasCell::UnresolvedProjection
    } else {
        AtlasCell::InconsistentBoundary
    };

    FaceEvidence {
        face_id,
        closed_loops,
        open_chains,
        traversal_full_period,
        traversal_ordinary,
        traversal_degenerate,
        projection_failed,
        deck_winding,
        v_span,
        has_apex,
        generator_sides,
        circular_arcs,
        cell,
    }
}

fn load_table(path: &str) -> anyhow::Result<Table> {
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => ruststep::parser::parse(&text)
            .map_err(|error| anyhow::anyhow!("failed to parse STEP: {error}"))?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "usage: cargo run --release --example cone_census_atlas -- MODEL.step [MORE.step ...]"
        );
        return Ok(());
    }

    let mut failing_faces = Vec::new();

    for model_path in &args {
        let table = match load_table(model_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Skipping {model_path}: {e}");
                continue;
            }
        };

        let mut converted = Vec::new();
        for (&shell_id, shell) in table.shell.iter() {
            if let Ok((cshell, _)) = table.to_compressed_shell_with_losses(shell_id, shell) {
                converted.push(cshell);
            }
        }

        let mut model_box = BoundingBox::<Point3>::new();
        for shell in &converted {
            for v in &shell.vertices {
                model_box.push(*v);
            }
        }
        let scaled = model_box.diameter() * RELATIVE_TOLERANCE;
        let tolerance = if scaled.is_finite() && scaled > 0.0 {
            scaled.max(MINIMUM_TOLERANCE)
        } else {
            DEGENERATE_TOLERANCE
        };

        for shell in &converted {
            let meshed = shell.robust_triangulation(tolerance);
            for (i, face) in meshed.faces.iter().enumerate() {
                let orig_face = &shell.faces[i];
                let is_cone = matches!(
                    orig_face.surface,
                    Surface::ElementarySurface(ElementarySurface::ConicalSurface(_))
                );
                if is_cone {
                    let is_meshed_to_nothing = match &face.surface {
                        Some(mesh) => mesh.faces().is_empty(),
                        None => false,
                    };
                    if is_meshed_to_nothing {
                        let evidence = analyze_cone_face(orig_face, &shell.edges);
                        failing_faces.push(evidence);
                    }
                }
            }
        }
    }

    println!("\n=== CONE MESHED-TO-NOTHING CANONICAL ATLAS CENSUS ===");
    println!(
        "Total failing cone faces analyzed: {}\n",
        failing_faces.len()
    );

    let mut total_full_period = 0;
    let mut total_ordinary = 0;
    let mut total_degenerate = 0;
    let mut total_projection_failed = 0;
    let mut total_deck_winding = 0;
    let mut total_has_apex = 0;

    for f in &failing_faces {
        total_full_period += f.traversal_full_period;
        total_ordinary += f.traversal_ordinary;
        total_degenerate += f.traversal_degenerate;
        total_projection_failed += f.projection_failed;
        total_deck_winding += f.deck_winding;
        if f.has_apex {
            total_has_apex += 1;
        }
    }

    println!("--- RAW INTERMEDIATE TRAVERSAL METRICS ---");
    println!("  Edges with FullPeriod Traversal: {}", total_full_period);
    println!("  Edges with Ordinary Traversal:   {}", total_ordinary);
    println!("  Edges with DegeneratePoint:      {}", total_degenerate);
    println!(
        "  Edges with Projection Failed:    {}",
        total_projection_failed
    );
    println!("  Total Deck Winding (+1/-1):      {}", total_deck_winding);
    println!("  Faces with Certified Cone Apex:  {}\n", total_has_apex);

    let mut cell_counts: HashMap<AtlasCell, usize> = HashMap::new();
    let mut cell_examples: HashMap<AtlasCell, Vec<String>> = HashMap::new();

    for f in &failing_faces {
        *cell_counts.entry(f.cell).or_default() += 1;
        let ex = cell_examples.entry(f.cell).or_default();
        if ex.len() < 3 {
            ex.push(f.face_id.clone());
        }
    }

    println!(
        "{:<25} {:<8} {:<8} {:<38} {:<20}",
        "Canonical Atlas Cell", "Count", "Share", "Missing Capability", "Representative Faces"
    );
    println!("{}", "-".repeat(105));

    let total = failing_faces.len().max(1) as f64;
    let mut sorted_cells: Vec<_> = cell_counts.keys().copied().collect();
    sorted_cells.sort_by_key(|c| std::cmp::Reverse(cell_counts[c]));

    for cell in sorted_cells {
        let count = cell_counts[&cell];
        let share = (count as f64 / total) * 100.0;
        let examples = cell_examples
            .get(&cell)
            .map(|ex| ex.join(", "))
            .unwrap_or_default();
        println!(
            "{:<25} {:<8} {:<7.1}% {:<38} {}",
            cell.name(),
            count,
            share,
            cell.missing_capability(),
            examples
        );
    }

    Ok(())
}
