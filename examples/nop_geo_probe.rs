//! NOP audit geometric probe: for target source face ids, report the face's
//! physical 3D boundary extent, 3D polyline length, UV boundary extent, and
//! repeated-edge structure directly from the compressed shell — independent of
//! the CDT/parity/validation pipeline.
//!
//! ```console
//! nop_geo_probe MODEL.step FACE_ID[,FACE_ID...]
//! ```
//!
//! Output is one tab-separated `GEO` line per target face:
//!   source_face_id, surface_kind, bound_count, 3d_boundary_diameter,
//!   3d_polyline_length, uv_extent_u, uv_extent_v, edge_use_count,
//!   distinct_3d_vertex_count, repeated_edge_use_count, shell_entity.
//!
//! The 3D boundary is read off the source edge curves (the same compressed
//! shell the renderer receives), so `3d_boundary_diameter` is a witness for
//! whether the face defines finite physical material regardless of chart/parity
//! behaviour.

use std::collections::HashMap;
use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{ElementarySurface, Surface},
};

fn surface_kind(surface: &Surface) -> &'static str {
    match surface {
        Surface::ElementarySurface(e) => match e {
            ElementarySurface::Plane(_) => "plane",
            ElementarySurface::Sphere(_) => "sphere",
            ElementarySurface::CylindricalSurface(_) => "cylinder",
            ElementarySurface::ToroidalSurface(_) => "torus",
            ElementarySurface::ConicalSurface(_) => "cone",
            ElementarySurface::DegenerateToroidalSurface(_) => "torus_degen",
        },
        Surface::SweptCurve(_) => "swept",
        Surface::BSplineSurface(_) => "bspline",
        Surface::NurbsSurface(_) => "nurbs",
        Surface::OffsetSurface(_) => "offset",
    }
}

fn fp_rank_tolerance(scale: f64) -> f64 {
    8.0 * f64::EPSILON * scale
}

/// World-rank certificate mirroring FACE-VALIDITY's farthest-pair test.
fn world_rank_of(points: &[Point3]) -> (u8, f64, f64, f64) {
    let scale = points
        .iter()
        .fold(0.0_f64, |acc, p| acc.max(p.to_vec().magnitude()));
    let tol = fp_rank_tolerance(scale);
    if points.len() < 2 {
        return (0, 0.0, 0.0, tol);
    }
    let mut span = 0.0_f64;
    let mut a = points[0];
    let mut b = points[1];
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = (points[i] - points[j]).magnitude();
            if d > span {
                span = d;
                a = points[i];
                b = points[j];
            }
        }
    }
    if span <= tol {
        return (0, span, 0.0, tol);
    }
    let direction = b - a;
    let direction_len = direction.magnitude();
    let mut max_perp = 0.0_f64;
    for p in points {
        let perp = (p - a).cross(direction).magnitude() / direction_len;
        if perp > max_perp {
            max_perp = perp;
        }
    }
    if max_perp <= tol {
        (1, span, max_perp, tol)
    } else {
        (2, span, max_perp, tol)
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 2 {
        anyhow::bail!("usage: nop_geo_probe MODEL.step FACE_ID[,FACE_ID...]");
    }
    let model = &args[0];
    let targets: Vec<u64> = args[1]
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();

    let table = load(model)?;
    for (&shell_entity, shell) in table.shell.iter() {
        let (cshell, _losses) = match table.to_compressed_shell_with_losses(shell_entity, shell) {
            Ok(x) => x,
            Err(_) => continue,
        };
        for (fi, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let idv = id.get();
            if !targets.contains(&idv) {
                continue;
            }
            // 3D boundary: sample the source edge curves along each bound.
            let mut bounds = BoundingBox::<Point3>::new();
            let mut length = 0.0f64;
            let mut edge_uses = 0usize;
            let mut repeated = 0usize;
            let _distinct_3d: Vec<Point3> = Vec::new();
            let mut uv_min = Point2::new(f64::INFINITY, f64::INFINITY);
            let mut uv_max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            let _uv_edges: Vec<(Point2, Point2)> = Vec::new();
            for wire in &face.boundaries {
                let mut seen: HashMap<(u32, bool), usize> = HashMap::new();
                for edge in wire {
                    edge_uses += 1;
                    let key = (edge.index as u32, edge.orientation);
                    *seen.entry(key).or_insert(0) += 1;
                    let Some(entry) = cshell.edges.get(edge.index) else {
                        continue;
                    };
                    let curve = &entry.curve;
                    let (t0, t1) = curve.range_tuple();
                    let n = 24;
                    let mut prev: Option<Point3> = None;
                    for i in 0..=n {
                        let t = t0 + (t1 - t0) * f64::from(i) / f64::from(n);
                        let p = curve.subs(t);
                        bounds.push(p);
                        if let Some(q) = prev {
                            length += q.distance(p);
                        }
                        prev = Some(p);
                    }
                    // UV: project the boundary endpoints back onto the surface.
                    if let Some((u, v)) = face.surface.search_parameter(curve.subs(t0), None, 100) {
                        uv_min.x = uv_min.x.min(u);
                        uv_min.y = uv_min.y.min(v);
                        uv_max.x = uv_max.x.max(u);
                        uv_max.y = uv_max.y.max(v);
                    }
                    if let Some((u2, v2)) = face.surface.search_parameter(curve.subs(t1), None, 100)
                    {
                        uv_min.x = uv_min.x.min(u2);
                        uv_min.y = uv_min.y.min(v2);
                        uv_max.x = uv_max.x.max(u2);
                        uv_max.y = uv_max.y.max(v2);
                    }
                }
                for (_, count) in seen {
                    if count > 1 {
                        repeated += 1;
                    }
                }
            }
            // distinct 3D vertices: boundary edge endpoints
            let mut verts: Vec<Point3> = Vec::new();
            for wire in &face.boundaries {
                for edge in wire {
                    let Some(entry) = cshell.edges.get(edge.index) else {
                        continue;
                    };
                    let (t0, t1) = entry.curve.range_tuple();
                    for t in [t0, t1] {
                        let p = entry.curve.subs(t);
                        if !verts.iter().any(|q| q.distance(p) < 1e-9) {
                            verts.push(p);
                        }
                    }
                }
            }
            let uv_extent = (
                if uv_min.x.is_finite() && uv_max.x.is_finite() {
                    uv_max.x - uv_min.x
                } else {
                    0.0
                },
                if uv_min.y.is_finite() && uv_max.y.is_finite() {
                    uv_max.y - uv_min.y
                } else {
                    0.0
                },
            );
            // World-rank certificate, mirroring FACE-VALIDITY's farthest-pair
            // test (validity.rs world_rank_of): rank 0 = all points coincide,
            // rank 1 = all on one line, rank 2 = real 2D region. The tolerance
            // is floating-point conditioning of the coordinates, never the
            // meshing tolerance.
            let mut world_pts: Vec<Point3> = Vec::new();
            for wire in &face.boundaries {
                for edge in wire {
                    let Some(entry) = cshell.edges.get(edge.index) else {
                        continue;
                    };
                    let curve = &entry.curve;
                    let (t0, t1) = curve.range_tuple();
                    for i in 0..=8 {
                        let t = t0 + (t1 - t0) * f64::from(i) / 8.0;
                        let p = curve.subs(t);
                        if !world_pts.iter().any(|q| q.distance(p) < 1e-12) {
                            world_pts.push(p);
                        }
                    }
                }
            }
            let (rank, span, max_perp, rank_tol) = world_rank_of(&world_pts);
            println!(
                "GEO\tsource_face_id={idv}\tkind={}\tshell_entity={shell_entity}\tface_index={fi}\t\
                 bound_count={}\tedge_use_count={edge_uses}\t\
                 3d_boundary_diameter={}\t3d_polyline_length={length:.6}\t\
                 distinct_3d_vertex_count={}\trepeated_edge_use_count={repeated}\t\
                 uv_extent_u={:.9}\tuv_extent_v={:.9}\t\
                 world_rank={rank}\trank_span={span:.6}\trank_max_perp={max_perp:.3e}\trank_tol={rank_tol:.3e}",
                surface_kind(&face.surface),
                face.boundaries.len(),
                bounds.diameter(),
                verts.len(),
                uv_extent.0,
                uv_extent.1,
            );
        }
    }
    Ok(())
}
