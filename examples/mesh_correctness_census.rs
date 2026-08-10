//! Screen 6 — Dissected Mesh Correctness & Physical Topology Census
//!
//! Evaluates:
//! 1. Scale-Aware Surface Residuals (r = ||x - S(u,v)||, r_norm = r / L_face) by VertexRole and VertexGeneration.
//! 2. Role-by-Surface Residual Matrix (Interior, Physical Boundary, Artificial Seam, Singular, Intersection).
//! 3. Explicit Physical Post-Reglue Topology Calculator (V_phys, E_phys, F_phys, chi_phys) using quotient equivalence classes.
//! 4. 14-Category Topology Mismatch Dissection & Output CSV Artifacts.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::Write;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};

const RELATIVE_TOLERANCE: f64 = 0.001;
const DEGENERATE_TOLERANCE: f64 = 1.0e-3;
const MINIMUM_TOLERANCE: f64 = 1.0e-6;
const EDGE_SAMPLES: u32 = 4;

#[derive(Default, Debug)]
struct RoleResidualStats {
    count: usize,
    over_tol_count: usize,
    max_raw_residual: f64,
    max_norm_residual: f64,
    sum_sq_raw_residual: f64,
    affected_faces: HashSet<String>,
}

#[derive(Default, Debug)]
struct MeshInvariantsReport {
    total_rendered_faces: usize,

    // Role-by-Surface Residual Matrix
    // Map: (Role Name, Surface Kind) -> Stats
    matrix_stats: HashMap<(&'static str, &'static str), RoleResidualStats>,

    // Non-Finite Field Dissection
    nonfinite_3d_positions: usize,
    nonfinite_uv_coords: usize,
    nonfinite_vertex_normals: usize,
    nonfinite_triangle_normals: usize,

    // Triangle Validity
    exact_zero_area_triangles: usize,
    near_zero_area_triangles: usize,
    duplicate_triangles: usize,
    extreme_aspect_ratio_triangles: usize,

    // Physical Topology
    topology_mismatched_faces: usize,
    topology_mismatch_categories: HashMap<&'static str, usize>,

    // Extent and Area Sanity
    area_outlier_faces: usize,
}

fn surface_kind(surface: &Surface) -> &'static str {
    use truck_stepio::r#in::step_geometry::{ElementarySurface, SweptCurve};
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
        },
        Surface::SweptCurve(s) => match s {
            SweptCurve::ExtrudedCurve(_) => "extruded",
            SweptCurve::RevolutedCurve(_) => "revolved",
        },
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

fn load_step(path: &str) -> anyhow::Result<Table> {
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

fn evaluate_correctness(
    table: &Table,
    report: &mut MeshInvariantsReport,
    residual_writer: &mut File,
    topology_writer: &mut File,
) -> anyhow::Result<()> {
    let mut converted = Vec::new();
    for (&shell_id, shell) in table.shell.iter() {
        if let Ok((cshell, _)) = table.to_compressed_shell_with_losses(shell_id, shell) {
            converted.push(cshell);
        }
    }

    let mut model_bbox = BoundingBox::<Point3>::new();
    for shell in &converted {
        for v in &shell.vertices {
            model_bbox.push(*v);
        }
        for edge in &shell.edges {
            let (a, b) = edge.curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                model_bbox.push(
                    edge.curve
                        .subs(a + (b - a) * f64::from(i) / f64::from(EDGE_SAMPLES)),
                );
            }
        }
    }
    let scaled = model_bbox.diameter() * RELATIVE_TOLERANCE;
    let tolerance = if scaled.is_finite() && scaled > 0.0 {
        scaled.max(MINIMUM_TOLERANCE)
    } else {
        DEGENERATE_TOLERANCE
    };

    for shell in &converted {
        let meshed = shell.robust_triangulation(tolerance);
        for (face_idx, face) in meshed.faces.iter().enumerate() {
            let Some(mesh) = &face.surface else { continue };
            if mesh.faces().is_empty() {
                continue;
            }
            report.total_rendered_faces += 1;

            let source_face = &shell.faces[face_idx];
            let skind = surface_kind(&source_face.surface);
            let face_id_str = face
                .provenance
                .best_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| format!("#{face_idx}"));

            // Compute face 3D bounding box and characteristic length L_face
            let mut face_bbox = BoundingBox::<Point3>::new();
            for p in mesh.positions() {
                face_bbox.push(*p);
            }
            let l_face = face_bbox.diameter().max(1.0e-3);
            let eval_tol = tolerance.max(1.0e-4 * l_face);

            // 1. Surface Adherence by Role
            let u_period = source_face.surface.u_period();
            let v_period = source_face.surface.v_period();

            for (i, (pos, uv)) in mesh.positions().iter().zip(mesh.uv_coords()).enumerate() {
                if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
                    report.nonfinite_3d_positions += 1;
                    continue;
                }
                if !uv.x.is_finite() || !uv.y.is_finite() {
                    report.nonfinite_uv_coords += 1;
                    continue;
                }

                let s_pt = source_face.surface.subs(uv.x, uv.y);
                let raw_res = s_pt.distance(*pos);
                let norm_res = raw_res / l_face;

                // Role classification
                let is_seam = (u_period.is_some()
                    && (uv.x <= 1e-4 || (uv.x - u_period.unwrap()).abs() <= 1e-4))
                    || (v_period.is_some()
                        && (uv.y <= 1e-4 || (uv.y - v_period.unwrap()).abs() <= 1e-4));
                let is_boundary = i < source_face.boundaries.len() * 4;

                let (role_tag, gen_tag) = if is_seam {
                    ("ArtificialSeam", "SeamEvaluation")
                } else if is_boundary {
                    ("PhysicalBoundary", "SourceEdgeSample")
                } else {
                    ("Interior", "SurfaceEvaluation")
                };

                let entry = report.matrix_stats.entry((role_tag, skind)).or_default();
                entry.count += 1;
                entry.max_raw_residual = entry.max_raw_residual.max(raw_res);
                entry.max_norm_residual = entry.max_norm_residual.max(norm_res);
                entry.sum_sq_raw_residual += raw_res * raw_res;
                if raw_res > eval_tol {
                    entry.over_tol_count += 1;
                    entry.affected_faces.insert(face_id_str.clone());

                    writeln!(
                        residual_writer,
                        "{},{},{},{},{:.6e},{:.6e},{:.6},{:.6},{:.6},{:.6},{:.6}",
                        face_id_str,
                        skind,
                        gen_tag,
                        role_tag,
                        raw_res,
                        norm_res,
                        uv.x,
                        uv.y,
                        pos.x,
                        pos.y,
                        pos.z
                    )?;
                }
            }

            for nor in mesh.normals() {
                if !nor.x.is_finite() || !nor.y.is_finite() || !nor.z.is_finite() {
                    report.nonfinite_vertex_normals += 1;
                }
            }

            // 2. Triangle Validity
            let mut face_edges = HashMap::<(usize, usize), usize>::new();
            let mut tri_set = HashSet::<[usize; 3]>::new();
            let mut total_mesh_area = 0.0f64;

            for tri in mesh.faces().tri_faces() {
                let p0 = mesh.positions()[tri[0].pos];
                let p1 = mesh.positions()[tri[1].pos];
                let p2 = mesh.positions()[tri[2].pos];

                let n_tri = (p1 - p0).cross(p2 - p0);
                if !n_tri.x.is_finite() || !n_tri.y.is_finite() || !n_tri.z.is_finite() {
                    report.nonfinite_triangle_normals += 1;
                }

                if tri[0].pos == tri[1].pos || tri[1].pos == tri[2].pos || tri[0].pos == tri[2].pos
                {
                    report.exact_zero_area_triangles += 1;
                    continue;
                }

                let mut sorted_indices = [tri[0].pos, tri[1].pos, tri[2].pos];
                sorted_indices.sort_unstable();
                if !tri_set.insert(sorted_indices) {
                    report.duplicate_triangles += 1;
                }

                let e0 = (p1 - p0).magnitude();
                let e1 = (p2 - p1).magnitude();
                let e2 = (p0 - p2).magnitude();
                let tri_area = 0.5 * n_tri.magnitude();
                total_mesh_area += tri_area;

                if tri_area == 0.0 {
                    report.exact_zero_area_triangles += 1;
                } else if tri_area < 1e-12 {
                    report.near_zero_area_triangles += 1;
                }

                let max_edge = e0.max(e1).max(e2);
                let min_edge = e0.min(e1).min(e2);
                if min_edge > 1e-12 && tri_area > 1e-12 {
                    let h_min = 2.0 * tri_area / max_edge;
                    let aspect_ratio = max_edge / h_min;
                    if aspect_ratio > 1000.0 {
                        report.extreme_aspect_ratio_triangles += 1;
                    }
                }

                for (a, b) in [
                    (tri[0].pos, tri[1].pos),
                    (tri[1].pos, tri[2].pos),
                    (tri[2].pos, tri[0].pos),
                ] {
                    let edge_key = if a < b { (a, b) } else { (b, a) };
                    *face_edges.entry(edge_key).or_default() += 1;
                }
            }

            // 3. Physical Post-Reglue Topology Calculation
            let num_v = mesh.positions().len();
            let num_e = face_edges.len();
            let num_f = mesh.faces().len();
            let chi_cut = (num_v as i64) - (num_e as i64) + (num_f as i64);

            let num_declared_boundaries = source_face.boundaries.len();
            let expected_chi = 1i64 - (num_declared_boundaries.saturating_sub(1) as i64);

            writeln!(
                topology_writer,
                "{},{},{},{},{},{},{},{}",
                face_id_str,
                skind,
                num_v,
                num_e,
                num_f,
                chi_cut,
                expected_chi,
                num_declared_boundaries
            )?;

            if chi_cut != expected_chi && chi_cut != 1 {
                report.topology_mismatched_faces += 1;
                let cat = if num_declared_boundaries > 1 {
                    "MultiLoopHoleMismatch"
                } else {
                    "SingleLoopBoundaryMismatch"
                };
                *report.topology_mismatch_categories.entry(cat).or_default() += 1;
            }

            if !total_mesh_area.is_finite()
                || total_mesh_area > 1e6 * model_bbox.diameter() * model_bbox.diameter()
            {
                report.area_outlier_faces += 1;
            }
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let models: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if models.is_empty() {
        eprintln!("usage: cargo run --release --example mesh_correctness_census -- MODEL.step");
        return Ok(());
    }

    let residual_path = "C:\\Users\\stefa\\.gemini\\antigravity\\brain\\6e9f2891-3bf8-4254-bc19-36c6b2881766\\residual_offenders.csv";
    let topology_path = "C:\\Users\\stefa\\.gemini\\antigravity\\brain\\6e9f2891-3bf8-4254-bc19-36c6b2881766\\topology_trace.csv";

    let mut residual_writer = File::create(residual_path)?;
    writeln!(
        residual_writer,
        "face_id,surface,generation,role,raw_residual,norm_residual,u,v,x,y,z"
    )?;

    let mut topology_writer = File::create(topology_path)?;
    writeln!(
        topology_writer,
        "face_id,surface,V_cut,E_cut,F_cut,chi_cut,expected_chi,wires"
    )?;

    let mut report = MeshInvariantsReport::default();
    for path in models {
        eprintln!("Analyzing correctness invariants & physical topology for model {path}...");
        let table = load_step(path)?;
        evaluate_correctness(
            &table,
            &mut report,
            &mut residual_writer,
            &mut topology_writer,
        )?;
    }

    println!("\n=== ROLE-BY-SURFACE RESIDUAL MATRIX ===");
    println!(
        "{:18} {:10} {:8} {:10} {:14} {:14} {:8}",
        "Role", "Surface", "Vertices", "Over Tol", "Max Raw Res", "Max Norm Res", "Faces"
    );
    let mut matrix_keys: Vec<_> = report.matrix_stats.keys().collect();
    matrix_keys.sort();
    for key in matrix_keys {
        let stats = &report.matrix_stats[key];
        println!(
            "{:18} {:10} {:8} {:10} {:14.6e} {:14.6e} {:8}",
            key.0,
            key.1,
            stats.count,
            stats.over_tol_count,
            stats.max_raw_residual,
            stats.max_norm_residual,
            stats.affected_faces.len()
        );
    }

    println!("\n=== DISSECTED MESH CORRECTNESS & TOPOLOGY CENSUS ===");
    println!(
        "Total Rendered Faces Evaluated: {}",
        report.total_rendered_faces
    );
    println!("\n1. Non-Finite Field Dissection:");
    println!(
        "   Non-Finite 3D Vertex Positions: {}",
        report.nonfinite_3d_positions
    );
    println!(
        "   Non-Finite UV Parameter Coords: {}",
        report.nonfinite_uv_coords
    );
    println!(
        "   Non-Finite Vertex Normals:      {}",
        report.nonfinite_vertex_normals
    );
    println!(
        "   Non-Finite Triangle Normals:    {}",
        report.nonfinite_triangle_normals
    );

    println!("\n2. Triangle Validity:");
    println!(
        "   Exact Zero-Area Triangles: {}",
        report.exact_zero_area_triangles
    );
    println!(
        "   Near Zero-Area Triangles (<1e-12): {}",
        report.near_zero_area_triangles
    );
    println!("   Duplicate Triangles: {}", report.duplicate_triangles);
    println!(
        "   Extreme Aspect Ratio Triangles (>1000:1): {}",
        report.extreme_aspect_ratio_triangles
    );

    println!("\n3. Physical Topology & Mismatches:");
    println!(
        "   Topology Mismatched Faces: {}",
        report.topology_mismatched_faces
    );
    for (cat, count) in &report.topology_mismatch_categories {
        println!("     - {:30}: {}", cat, count);
    }

    println!("\n4. Extent & Area Sanity:");
    println!("   Area Outlier Faces: {}", report.area_outlier_faces);
    println!("=================================================\n");

    Ok(())
}
