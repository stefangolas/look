//! NOP audit: dump the source edge structure of a target face so the
//! bucket-A (rank-2 world, collapsed UV chart) collapse site can be identified.
//!
//! For each target face prints: shell edges referenced by the face's wires,
//! each edge's vertex handles (topologically closed?), curve kind, curve range,
//! and whether the compressed shell kept the edge as Mesh or Unresolved.
//!
//! ```console
//! nop_edge_probe MODEL.step FACE_ID
//! ```

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
        anyhow::bail!("usage: nop_edge_probe MODEL.step FACE_ID");
    }
    let model = &args[0];
    let target: u64 = args[1].parse()?;

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
            if id.get() != target {
                continue;
            }
            println!(
                "FACE source_face_id={target} shell_entity={shell_entity} face_index={fi} \
                 kind={} orientation={}",
                surface_kind(&face.surface),
                face.orientation
            );
            // Plane axes: the parameterization's metric. A plane whose u/v
            // axes are near-parallel or tiny is a degenerate surface map.
            if let Surface::ElementarySurface(ElementarySurface::Plane(p)) = &face.surface {
                let o = p.origin();
                let ua = p.u_axis();
                let va = p.v_axis();
                let n = p.normal();
                let cross = ua.cross(va);
                println!(
                    "  PLANE origin=({:.6},{:.6},{:.6}) u_axis=({:.6},{:.6},{:.6}) \
                     v_axis=({:.6},{:.6},{:.6}) |u|={:.6} |v|={:.6} |u x v|={:.3e} \
                     u.v={:.6} n=({:.6},{:.6},{:.6})",
                    o.x,
                    o.y,
                    o.z,
                    ua.x,
                    ua.y,
                    ua.z,
                    va.x,
                    va.y,
                    va.z,
                    ua.magnitude(),
                    va.magnitude(),
                    cross.magnitude(),
                    ua.dot(va),
                    n.x,
                    n.y,
                    n.z,
                );
            }
            let (urange, vrange) = face.surface.try_range_tuple();
            println!("  surface range u={urange:?} v={vrange:?}");
            println!("  bound_count={}", face.boundaries.len());
            for (bi, wire) in face.boundaries.iter().enumerate() {
                println!("  WIRE[{bi}] edge_uses={}", wire.len());
                for (ui, edge_idx) in wire.iter().enumerate() {
                    let Some(entry) = cshell.edges.get(edge_idx.index) else {
                        println!("    use[{ui}] index={} -> MISSING", edge_idx.index);
                        continue;
                    };
                    let (t0, t1) = entry.curve.range_tuple();
                    let a = entry.curve.subs(t0);
                    let b = entry.curve.subs(t1);
                    let (va, vb) = entry.vertices;
                    let closed = va == vb;
                    println!(
                        "    use[{ui}] index={} orientation={} verts=({va},{vb}) topo_closed={closed} \
                         range=[{t0:.9},{t1:.9}] p0=({:.6},{:.6},{:.6}) p1=({:.6},{:.6},{:.6})",
                        edge_idx.index, edge_idx.orientation, a.x, a.y, a.z, b.x, b.y, b.z,
                    );
                }
            }
            println!("  shell edge count={}", cshell.edges.len());
            // Sample the boundary curves in world and project to the surface UV.
            for (bi, wire) in face.boundaries.iter().enumerate() {
                for (ui, edge_idx) in wire.iter().enumerate() {
                    let Some(entry) = cshell.edges.get(edge_idx.index) else {
                        continue;
                    };
                    let curve = &entry.curve;
                    let (t0, t1) = curve.range_tuple();
                    // Distance of each sampled world point to the surface.
                    let mut on_surface = 0usize;
                    let mut total = 0usize;
                    let mut max_dist = 0.0f64;
                    let mut off_t: Vec<f64> = Vec::new();
                    for i in 0..=64 {
                        let t = t0 + (t1 - t0) * f64::from(i) / 64.0;
                        let p = curve.subs(t);
                        if let Some((u, v)) = face.surface.search_parameter(p, None, 100) {
                            let q = face.surface.subs(u, v);
                            let d = q.distance(p);
                            on_surface += usize::from(d < 1e-6);
                            max_dist = max_dist.max(d);
                        } else {
                            off_t.push(t);
                        }
                        total += 1;
                    }
                    println!(
                        "    ONSURF[{bi}][{ui}] on_surface={on_surface}/{total} max_dist={max_dist:.3e} off_t={:?}",
                        off_t.iter().map(|t| format!("{t:.3}")).collect::<Vec<_>>()
                    );
                    // Is the unit-domain subset [0,1] fully on-surface?
                    if t1 - t0 > 1.0 {
                        let (mut on, mut tot) = (0usize, 0usize);
                        for i in 0..=64 {
                            let t = f64::from(i) / 64.0;
                            let p = curve.subs(t);
                            if let Some((u, v)) = face.surface.search_parameter(p, None, 100) {
                                on += usize::from(face.surface.subs(u, v).distance(p) < 1e-6);
                            }
                            tot += 1;
                        }
                        println!("    ONSURF[{bi}][{ui}] unit-domain [0,1]: on_surface={on}/{tot}");
                    }
                }
            }
        }
    }
    Ok(())
}
