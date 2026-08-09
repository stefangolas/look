//! Scratch probe (Phase 1 comparison): for a canonical face (00007705), check
//! whether the spline boundary edges are closed loops whose source vertices sit
//! at the evaluation_range endpoints, vs interior. Prints the same root/closure
//! evidence as the 00007667 probe so the two populations can be compared.
//! Usage: spline_edge_canonical_probe MODEL.step --face 120193

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
        let face = &cshell.faces[face_idx];
        println!("SHELL containing face {face_filter} (shell face idx {face_idx})");
        println!(
            "  family={:?} bounds={}",
            look::step_support_schema_of(&face.surface),
            face.boundaries.len()
        );
        for (bi, wire) in face.boundaries.iter().enumerate() {
            println!("  bound {bi}:");
            for (ui, idx) in wire.iter().enumerate() {
                let e = &cshell.edges[idx.index];
                let (va, vb) = e.vertices;
                let pv_a = cshell.vertices[va];
                let pv_b = cshell.vertices[vb];
                let rt = e.curve.range_tuple();
                let er = e.curve.evaluation_range();
                let fam = match &e.curve {
                    Curve3D::BSplineCurve(_) => "bspline",
                    Curve3D::NurbsCurve(_) => "nurbs",
                    _ => "other",
                };
                println!(
                    "    use {ui}: edge_idx={} ori={} verts=({va},{vb}) fam={fam} rt=({rt:?}) er=({er:?})",
                    idx.index, idx.orientation
                );
                if fam != "bspline" && fam != "nurbs" {
                    continue;
                }
                let closed_loop = e.curve.subs(er.0).distance(e.curve.subs(er.1)) < 1e-6;
                println!("      closed_loop over er: {closed_loop}");
                println!("      subs(er.0)={:?}", e.curve.subs(er.0));
                println!("      pv_a={pv_a:?}");
                println!(
                    "      res er.0 vs pv_a: {:.3e}",
                    e.curve.subs(er.0).distance(pv_a)
                );
                println!(
                    "      res er.1 vs pv_b: {:.3e}",
                    e.curve.subs(er.1).distance(pv_b)
                );
                // roots for each vertex over the eval range
                for (tag, v) in [("pv_a", pv_a), ("pv_b", pv_b)] {
                    let (t, d) = root_for(&e.curve, er, v);
                    println!(
                        "      root {tag}: t={t:.9} d={d:.3e} POU={}",
                        e.curve.basis_is_partition_of_unity(t)
                    );
                }
            }
        }
        break;
    }
    Ok(())
}

fn root_for(curve: &Curve3D, er: (f64, f64), v: Point3) -> (f64, f64) {
    let n = 8192;
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..=n {
        let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
        let d = curve.subs(t).distance(v);
        if d < best.0 {
            best = (d, t);
        }
    }
    best
}
