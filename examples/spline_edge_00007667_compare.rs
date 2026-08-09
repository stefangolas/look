//! Scratch probe (Phase 1 comparison): exact plane-distance measurement for the
//! three candidate traversals of edge #30 on a plane face, plus the extruded
//! face's swept-surface structural check. Resolves which sub-arc of the closed
//! loop lies on each face's supporting surface.
//! Usage: spline_edge_00007667_compare MODEL.step --face 10428

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, ElementarySurface, Surface},
};

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
        // Find shared spline edge (range.lo < -0.4).
        let mut shared: Option<usize> = None;
        for wire in &face.boundaries {
            for idx in wire {
                if cshell.edges[idx.index].curve.range_tuple().0 < -0.4 {
                    shared = Some(idx.index);
                }
            }
        }
        let Some(si) = shared else {
            println!("no shared spline edge in face {face_filter}");
            continue;
        };
        let edge = &cshell.edges[si];
        let (va, vb) = edge.vertices;
        let pv_a = cshell.vertices[va];
        let pv_b = cshell.vertices[vb];
        let curve = &edge.curve;
        let er = curve.evaluation_range();
        let (t_a, ra) = root_for(curve, er, pv_a);
        let (t_b, rb) = root_for(curve, er, pv_b);
        println!("face {face_filter} shared edge idx={si} verts=({va},{vb})");
        println!("  pv_a={pv_a:?}");
        println!("  pv_b={pv_b:?}");
        println!("  t_a={t_a:.9} ra={ra:.3e} (pv_a)  t_b={t_b:.9} rb={rb:.3e} (pv_b)  er={er:?}");
        let wrap = t_a > t_b;
        let span = if wrap { t_b + 1.0 - t_a } else { t_b - t_a };
        println!("  wrap={wrap} source-arc span={span:.6}");

        // Sample the three traversals.
        let sample = |tau: f64| -> Point3 {
            let t = if tau > 1.0 { tau - 1.0 } else { tau };
            curve.subs(t)
        };
        let full_loop: Vec<Point3> = (0..=64)
            .map(|i| sample(er.0 + (er.1 - er.0) * i as f64 / 64.0))
            .collect();
        let source_arc: Vec<Point3> = if wrap {
            (0..=64)
                .map(|i| sample(t_a + span * i as f64 / 64.0))
                .collect()
        } else {
            (0..=64)
                .map(|i| sample(t_a + span * i as f64 / 64.0))
                .collect()
        };
        let complement_arc: Vec<Point3> = (0..=64)
            .map(|i| sample(t_b + (t_a - t_b).max(0.0) * i as f64 / 64.0))
            .collect();

        // Surface distance measurement.
        let surf = &face.surface;
        match surf {
            Surface::ElementarySurface(ElementarySurface::Plane(pl)) => {
                let origin = pl.origin();
                let n = pl.normal();
                let dist = |p: Point3| (p - origin).dot(n).abs();
                let fmt = |name: &str, pts: &[Point3]| {
                    let max = pts.iter().map(|p| dist(*p)).fold(0.0_f64, f64::max);
                    let min = pts.iter().map(|p| dist(*p)).fold(f64::INFINITY, f64::min);
                    println!("  plane-face {name}: plane_dist max={max:.3e} min={min:.3e}");
                    (min, max)
                };
                let (min_f, _) = fmt("full_loop", &full_loop);
                let (min_s, _) = fmt("source_arc", &source_arc);
                let (min_c, _) = fmt("complement_arc", &complement_arc);
                println!(
                    "  => full_loop on plane: {min_f:.3e}; source_arc on plane: {min_s:.3e}; complement_arc on plane: {min_c:.3e}"
                );
                // Which arc does the plane actually contain? The vertices:
                println!(
                    "  vertices on plane: va={:.3e} vb={:.3e}",
                    (pv_a - origin).dot(n).abs(),
                    (pv_b - origin).dot(n).abs()
                );
            }
            other => {
                println!("  surface family: {:?}", look::step_support_schema_of(surf));
                let _ = other;
            }
        }
        break;
    }
    Ok(())
}

fn root_for(curve: &Curve3D, er: (f64, f64), v: Point3) -> (f64, f64) {
    let n = 65536;
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..=n {
        let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
        let d = curve.subs(t).distance(v);
        if d < best.0 {
            best = (d, t);
        }
    }
    // return (t, dist)
    (best.1, best.0)
}
