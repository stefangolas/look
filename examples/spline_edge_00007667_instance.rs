//! Scratch probe (Phase 1): dump ONE shell instance containing a target face,
//! its shared spline edge, vertex positions, fine parameter search, and the
//! 2-edge bound closure under candidate traversals.
//! Usage: spline_edge_00007667_instance MODEL.step --face 19018

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("model path");
    let mut face_filter: Option<u64> = None;
    for i in 2..args.len() {
        if args[i] == "--face" {
            face_filter = args[i + 1].parse().ok();
        }
    }
    let face_filter = face_filter.expect("--face ID");
    let bytes = std::fs::read(path)?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(exchange) => exchange,
        Err(_) => {
            ruststep::parser::parse(&text).map_err(|e| anyhow::anyhow!("parse failed: {e}"))?
        }
    };
    let section = exchange.data.swap_remove(0);
    let table = Table::from_owned_data_section(section);

    let mut found = false;
    for (_, shell) in table.shell.iter() {
        let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell) else {
            continue;
        };
        let _ = losses;
        let Some(face_idx) = cshell
            .faces
            .iter()
            .position(|f| f.provenance.best_id().map(|id| id.get()) == Some(face_filter))
        else {
            continue;
        };
        found = true;
        let face = &cshell.faces[face_idx];
        println!("SHELL containing face {face_filter} (shell face idx {face_idx})");
        println!(
            "  family={:?} bounds={}",
            look::step_support_schema_of(&face.surface),
            face.boundaries.len()
        );
        // The shared spline edge: any bspline/nurbs edge with declared range.lo < -0.4
        let mut shared: Option<usize> = None;
        for (bi, wire) in face.boundaries.iter().enumerate() {
            println!("  bound {bi}:");
            for (ui, idx) in wire.iter().enumerate() {
                let e = &cshell.edges[idx.index];
                let rt = e.curve.range_tuple();
                let fam = match &e.curve {
                    Curve3D::BSplineCurve(_) => "bspline",
                    Curve3D::NurbsCurve(_) => "nurbs",
                    _ => "other",
                };
                println!(
                    "    use {ui}: edge_idx={} ori={} verts={:?} fam={fam} rt=({rt:?})",
                    idx.index, idx.orientation, e.vertices
                );
                if rt.0 < -0.4 {
                    shared = Some(idx.index);
                }
            }
        }
        let Some(si) = shared else {
            println!("  NO SHARED SPLINE EDGE in this face");
            continue;
        };
        let edge = &cshell.edges[si];
        let (va, vb) = edge.vertices;
        let pv_a = cshell.vertices[va];
        let pv_b = cshell.vertices[vb];
        println!("  SHARED EDGE idx={si} vertices=({va},{vb})");
        println!("    pv_a={pv_a:?}");
        println!("    pv_b={pv_b:?}");
        let curve = &edge.curve;
        let rt = curve.range_tuple();
        let er = curve.evaluation_range();
        println!("    range_tuple=({rt:?}) evaluation_range=({er:?})");
        println!(
            "    subs(er.0)={:?} subs(er.1)={:?}",
            curve.subs(er.0),
            curve.subs(er.1)
        );
        println!(
            "    res at er endpoints: d_a={:.3e} d_b={:.3e}",
            curve.subs(er.0).distance(pv_a),
            curve.subs(er.1).distance(pv_b)
        );
        match curve {
            Curve3D::BSplineCurve(bsp) => {
                println!("    degree={}", bsp.degree());
                let (knots, mults) = bsp.knot_vec().to_single_multi();
                println!("    knots={knots:?} mults={mults:?}");
                println!("    control_points:");
                for (i, p) in bsp.control_points().iter().enumerate() {
                    println!("      [{i}] ({:.8},{:.8},{:.8})", p.x, p.y, p.z);
                }
            }
            Curve3D::NurbsCurve(nurbs) => {
                println!("    degree={}", nurbs.degree());
                let (knots, mults) = nurbs.knot_vec().to_single_multi();
                println!("    knots={knots:?} mults={mults:?}");
                println!("    control_points (xyz,w):");
                for (i, h) in nurbs.control_points().iter().enumerate() {
                    println!(
                        "      [{i}] ({:.8},{:.8},{:.8}) w={:.6}",
                        h.truncate().x,
                        h.truncate().y,
                        h.truncate().z,
                        h.w
                    );
                }
            }
            _ => {}
        }
        // Fine parameter search against both vertices over the whole declared
        // range, reporting basis validity.
        println!("    fine residual search:");
        let n = 65536;
        let mut hits_a: Vec<(f64, f64)> = Vec::new();
        let mut hits_b: Vec<(f64, f64)> = Vec::new();
        for i in 0..=n {
            let t = rt.0 + (rt.1 - rt.0) * i as f64 / n as f64;
            let p = curve.subs(t);
            let da = p.distance(pv_a);
            let db = p.distance(pv_b);
            if da < 5e-4 {
                hits_a.push((t, da));
            }
            if db < 5e-4 {
                hits_b.push((t, db));
            }
        }
        println!("    a-vertex candidates (t, dist) under 5e-4:");
        for (t, d) in &hits_a {
            println!(
                "      t={t:.6} d={d:.3e} POU={}",
                curve.basis_is_partition_of_unity(*t)
            );
        }
        println!("    b-vertex candidates (t, dist) under 5e-4:");
        for (t, d) in &hits_b {
            println!(
                "      t={t:.6} d={d:.3e} POU={}",
                curve.basis_is_partition_of_unity(*t)
            );
        }
        // Basis POU extent: find the maximal interval where basis sums to 1.
        println!("    POU extent (coarse):");
        let mut pou = Vec::new();
        for i in 0..=256 {
            let t = rt.0 + (rt.1 - rt.0) * i as f64 / 256.0;
            if curve.basis_is_partition_of_unity(t) {
                pou.push(t);
            }
        }
        if let (Some(&lo), Some(&hi)) = (pou.first(), pou.last()) {
            println!("      POU on ~[{lo:.6}, {hi:.6}]");
        }
        println!("    loop samples over eval range:");
        for i in 0..=16 {
            let t = er.0 + (er.1 - er.0) * i as f64 / 16.0;
            let p = curve.subs(t);
            println!("      t={t:.4} p=({:.6},{:.6},{:.6})", p.x, p.y, p.z);
        }
        // the paired edge in the bound and its endpoints
        for (bi, wire) in face.boundaries.iter().enumerate() {
            for (ui, idx) in wire.iter().enumerate() {
                if idx.index == si {
                    continue;
                }
                let e = &cshell.edges[idx.index];
                let (x0, x1) = e.vertices;
                println!(
                    "    paired edge in bound {bi} use {ui}: edge_idx={} ori={} verts=({x0},{x1})",
                    idx.index, idx.orientation
                );
            }
        }
        break;
    }
    if !found {
        println!("face {face_filter} not found in any shell");
    }
    Ok(())
}
