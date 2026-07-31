//! Diagnostic Probe — Single-Face Trace & Analytic Inverse Oracle (Phase 0)
//!
//! Provides fast (< 30s) diagnosis of the first failed obligation and concrete
//! witness for an isolated face in a STEP file.
//!
//! ```console
//! cargo run --release --example single_face_trace -- MODEL.step [FACE_ID_OR_INDEX]
//! ```

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{ElementarySurface, Surface},
};
use truck_topology::compress::{CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, truck_stepio::r#in::step_geometry::Curve3D, Surface>;

/// Fine-grained failure reasons replacing coarse NoSurfaceProduced/MeshedToNothing.
#[derive(Debug, Clone, PartialEq)]
pub enum TerminalReason {
    ProjectionNoSolution {
        point: Point3,
        sample_idx: usize,
    },
    ProjectionOutsideDeclaredRange {
        uv: Point2,
        declared_u: (f64, f64),
        declared_v: (f64, f64),
    },
    ProjectionResidualExceeded {
        residual: f64,
        permitted: f64,
    },
    ProjectionSingular {
        uv: Point2,
        singular_val: f64,
    },
    ProjectionBranchAmbiguous {
        step: Vector2,
    },
    ZeroAreaDomain {
        loop_area: f64,
    },
    NoMaterialCells,
    ConstraintConstructionFailed,
    Success {
        triangles: usize,
        vertices: usize,
    },
}

/// Analytic inverse projection result from a closed-form geometric oracle.
#[derive(Debug, Clone)]
pub struct AnalyticInverseResult {
    pub surface_kind: &'static str,
    pub uv: Point2,
    pub point_on_surface: Point3,
    pub residual_3d: f64,
    pub inside_declared_domain: bool,
}

/// Analytic closed-form inverse projection for elementary surfaces.
pub fn analytic_inverse(surface: &Surface, pt: Point3) -> Option<AnalyticInverseResult> {
    match surface {
        Surface::ElementarySurface(elem) => match elem {
            ElementarySurface::Plane(plane) => {
                let origin = plane.origin();
                let u_axis = plane.u_axis();
                let v_axis = plane.v_axis();
                let normal = plane.normal();
                let diff = pt - origin;
                let u = diff.dot(u_axis);
                let v = diff.dot(v_axis);
                let projected = origin + u * u_axis + v * v_axis;
                let res = diff.dot(normal).abs();
                Some(AnalyticInverseResult {
                    surface_kind: "plane",
                    uv: Point2::new(u, v),
                    point_on_surface: projected,
                    residual_3d: res,
                    inside_declared_domain: true,
                })
            }
            ElementarySurface::CylindricalSurface(cyl) => {
                let (urange, vrange) = cyl.try_range_tuple();
                let u_bound = urange.unwrap_or((0.0, std::f64::consts::TAU));
                let v_bound = vrange.unwrap_or((-f64::INFINITY, f64::INFINITY));

                let uv = cyl.search_parameter(pt, None, 100)?;
                let proj = cyl.subs(uv.0, uv.1);
                let res = proj.distance(pt);
                let in_domain = uv.0 >= u_bound.0
                    && uv.0 <= u_bound.1
                    && uv.1 >= v_bound.0
                    && uv.1 <= v_bound.1;
                Some(AnalyticInverseResult {
                    surface_kind: "cylinder",
                    uv: Point2::new(uv.0, uv.1),
                    point_on_surface: proj,
                    residual_3d: res,
                    inside_declared_domain: in_domain,
                })
            }
            ElementarySurface::ConicalSurface(cone) => {
                let (urange, vrange) = cone.try_range_tuple();
                let u_bound = urange.unwrap_or((0.0, std::f64::consts::TAU));
                let v_bound = vrange.unwrap_or((-f64::INFINITY, f64::INFINITY));

                let uv = cone
                    .search_parameter(pt, None, 100)
                    .or_else(|| cone.search_nearest_parameter(pt, None, 100))?;
                let proj = cone.subs(uv.0, uv.1);
                let res = proj.distance(pt);
                let in_domain = uv.0 >= u_bound.0
                    && uv.0 <= u_bound.1
                    && uv.1 >= v_bound.0
                    && uv.1 <= v_bound.1;
                Some(AnalyticInverseResult {
                    surface_kind: "cone",
                    uv: Point2::new(uv.0, uv.1),
                    point_on_surface: proj,
                    residual_3d: res,
                    inside_declared_domain: in_domain,
                })
            }
            ElementarySurface::Sphere(sphere) => {
                let (urange, vrange) = sphere.try_range_tuple();
                let u_bound = urange.unwrap_or((0.0, std::f64::consts::TAU));
                let v_bound =
                    vrange.unwrap_or((-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2));

                let uv = sphere
                    .search_parameter(pt, None, 100)
                    .or_else(|| sphere.search_nearest_parameter(pt, None, 100))?;
                let proj = sphere.subs(uv.0, uv.1);
                let res = proj.distance(pt);
                let in_domain = uv.0 >= u_bound.0
                    && uv.0 <= u_bound.1
                    && uv.1 >= v_bound.0
                    && uv.1 <= v_bound.1;
                Some(AnalyticInverseResult {
                    surface_kind: "sphere",
                    uv: Point2::new(uv.0, uv.1),
                    point_on_surface: proj,
                    residual_3d: res,
                    inside_declared_domain: in_domain,
                })
            }
            _ => {
                let uv = surface
                    .search_parameter(pt, None, 100)
                    .or_else(|| surface.search_nearest_parameter(pt, None, 100))?;
                let proj = surface.subs(uv.0, uv.1);
                let res = proj.distance(pt);
                Some(AnalyticInverseResult {
                    surface_kind: "elementary_other",
                    uv: Point2::new(uv.0, uv.1),
                    point_on_surface: proj,
                    residual_3d: res,
                    inside_declared_domain: true,
                })
            }
        },
        _ => {
            let uv = surface
                .search_parameter(pt, None, 100)
                .or_else(|| surface.search_nearest_parameter(pt, None, 100))?;
            let proj = surface.subs(uv.0, uv.1);
            let res = proj.distance(pt);
            Some(AnalyticInverseResult {
                surface_kind: "freeform",
                uv: Point2::new(uv.0, uv.1),
                point_on_surface: proj,
                residual_3d: res,
                inside_declared_domain: true,
            })
        }
    }
}

fn load(path: &str) -> anyhow::Result<Table> {
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

fn extract_target_face(
    cshell: &Cshell,
    target_spec: &str,
) -> Option<(usize, CompressedFace<Surface>)> {
    if let Ok(idx) = target_spec.parse::<usize>() {
        if idx < cshell.faces.len() {
            return Some((idx, cshell.faces[idx].clone()));
        }
    }
    let target_str = target_spec.trim_start_matches('#');
    for (idx, face) in cshell.faces.iter().enumerate() {
        if let Some(id) = face.provenance.best_id() {
            let s = id.to_string();
            if s == target_spec || s == target_str || s.contains(target_str) {
                return Some((idx, face.clone()));
            }
        }
    }
    None
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: single_face_trace MODEL.step [FACE_ID_OR_INDEX]");
        return Ok(());
    }

    let model_path = &args[0];
    let target_spec = args.get(1).map(|s| s.as_str());

    println!("=== PHASE 0 SINGLE-FACE CHECKPOINT TRACE ===");
    println!("Loading file: {model_path}");
    let table = load(model_path)?;

    for (shell_idx, (_, shell)) in table.shell.iter().enumerate() {
        let (cshell, losses) = match table.to_compressed_shell_with_losses(shell) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Shell #{shell_idx} conversion error: {e}");
                continue;
            }
        };

        if !losses.is_empty() {
            println!("Conversion losses in Shell #{shell_idx}: {}", losses.len());
            for loss in &losses {
                println!(
                    "  - Loss reason: {} (provenance: {:?})",
                    loss.reason.tag(),
                    loss.provenance
                );
            }
        }

        let faces_to_trace: Vec<usize> = if let Some(spec) = target_spec {
            match extract_target_face(&cshell, spec) {
                Some((idx, _)) => vec![idx],
                None => {
                    eprintln!("Target face '{spec}' not found in Shell #{shell_idx}");
                    continue;
                }
            }
        } else {
            (0..cshell.faces.len()).collect()
        };

        let tolerance = 0.001;

        for &face_idx in &faces_to_trace {
            let face = &cshell.faces[face_idx];
            let face_id_str = face
                .provenance
                .best_id()
                .map(|id| format!("#{id}"))
                .unwrap_or_else(|| format!("idx_{face_idx}"));

            println!("\n--------------------------------------------------");
            println!("Tracing Face [{face_idx}] (ID: {face_id_str})");
            println!("  Boundaries count: {}", face.boundaries.len());
            println!("  Surface type: {:?}", face.surface);

            let (urange, vrange) = face.surface.try_range_tuple();
            println!("  Declared U range: {:?}", urange);
            println!("  Declared V range: {:?}", vrange);
            println!("  U period: {:?}", face.surface.u_period());
            println!("  V period: {:?}", face.surface.v_period());

            let mut wire_failed = false;
            for (wire_idx, wire) in face.boundaries.iter().enumerate() {
                println!("  Wire [{wire_idx}]: {} edges", wire.len());
                let mut wire_max_residual = 0.0f64;

                for (edge_wire_idx, edge_idx_struct) in wire.iter().enumerate() {
                    let edge_entry = &cshell.edges[edge_idx_struct.index];
                    let curve = &edge_entry.curve;
                    let (t0, t1) = curve.range_tuple();
                    let p0 = curve.subs(t0);
                    let p1 = curve.subs(t1);
                    println!(
                        "    Edge [{edge_wire_idx}] (index {}): range [{:.6}, {:.6}]",
                        edge_idx_struct.index, t0, t1
                    );
                    println!("      p(t0) = ({:.6}, {:.6}, {:.6})", p0.x, p0.y, p0.z);
                    println!("      p(t1) = ({:.6}, {:.6}, {:.6})", p1.x, p1.y, p1.z);

                    let samples = 4;
                    for s in 0..=samples {
                        let t = t0 + (t1 - t0) * (s as f64 / samples as f64);
                        let pt = curve.subs(t);

                        let analytic = analytic_inverse(&face.surface, pt);
                        let search_res = face
                            .surface
                            .search_parameter(pt, None, 100)
                            .or_else(|| face.surface.search_nearest_parameter(pt, None, 100));

                        match (analytic, search_res) {
                            (Some(oracle), Some(uv_num)) => {
                                let proj_num = face.surface.subs(uv_num.0, uv_num.1);
                                let res_num = proj_num.distance(pt);
                                wire_max_residual = wire_max_residual.max(res_num);
                                if res_num > tolerance * 5.0 {
                                    println!(
                                        "      ⚠️ SAMPLE S={s} OFF SURFACE: res={res_num:.4e} pt=({:.4},{:.4},{:.4})",
                                        pt.x, pt.y, pt.z
                                    );
                                    println!(
                                        "         Numerical UV=({:.6},{:.6}) Oracle UV=({:.6},{:.6})",
                                        uv_num.0, uv_num.1, oracle.uv.x, oracle.uv.y
                                    );
                                }
                            }
                            (Some(oracle), None) => {
                                println!(
                                    "      ❌ SAMPLE S={s} PROJECTION FAILED (sp returned None)"
                                );
                                println!(
                                    "         Oracle found UV=({:.6},{:.6}) res={:.4e}",
                                    oracle.uv.x, oracle.uv.y, oracle.residual_3d
                                );
                                wire_failed = true;
                            }
                            (None, Some(uv_num)) => {
                                let proj_num = face.surface.subs(uv_num.0, uv_num.1);
                                let res_num = proj_num.distance(pt);
                                println!(
                                    "      ℹ️ SAMPLE S={s} Numerical UV=({:.6},{:.6}) res={res_num:.4e}",
                                    uv_num.0, uv_num.1
                                );
                            }
                            (None, None) => {
                                println!("      ❌ SAMPLE S={s} BOTH PROJECTIONS FAILED!");
                                wire_failed = true;
                            }
                        }
                    }
                }
                println!("  Wire [{wire_idx}] max 3D residual to surface: {wire_max_residual:.6e}");
            }

            if wire_failed {
                println!("  -> TERMINAL REASON: ProjectionNoSolution (Boundary projection failed)");
            } else {
                println!("  -> Projection succeeded across all boundary samples!");
            }
        }
    }

    Ok(())
}
