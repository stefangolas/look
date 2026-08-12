//! Scratch epistemic probe for the 00007667 shared spline edge (Phase 1).
//!
//! Deep-dives ONE spline edge of a swept/extruded face: full knot vector,
//! multiplicities, degree, control points / weights, declared vs evaluable
//! range, basis partition-of-unity behaviour around the interesting
//! parameters, and a parameter search for the source vertices. It does NOT
//! classify — it reports evidence.
//! Usage: spline_edge_00007667_probe MODEL.step --edge 30

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("model path");
    let mut target: Option<usize> = None;
    let mut face_filter: Option<u64> = None;
    for i in 2..args.len() {
        if args[i] == "--edge" {
            target = args[i + 1].parse().ok();
        }
        if args[i] == "--face" {
            face_filter = args[i + 1].parse().ok();
        }
    }
    let target = target.expect("--edge IDX");
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

    for (&shell_id, shell) in table.shell.iter() {
        let Ok((cshell, losses)) = table.to_compressed_shell_with_losses(shell_id, shell) else {
            continue;
        };
        let _ = losses;
        // Match by index if given, but only when that index is a matching
        // spline edge; otherwise skip this shell (edge indices are per-shell).
        if target >= cshell.edges.len() {
            continue;
        }
        let cand = &cshell.edges[target];
        let rt0 = cand.curve.range_tuple().0;
        if rt0 > -0.4 {
            // Not the shared spline (its declared range.lo ~= -0.5147).
            continue;
        }
        let edge = cand;
        let (va, vb) = edge.vertices;
        let pv_a = cshell.vertices[va];
        let pv_b = cshell.vertices[vb];
        let curve = &edge.curve;
        println!("TARGET EDGE idx={target}");
        println!("  vertices=({va},{vb})");
        println!("  pv_a={pv_a:?}");
        println!("  pv_b={pv_b:?}");
        println!(
            "  family={:?}",
            match curve {
                Curve3D::BSplineCurve(_) => "bspline",
                Curve3D::NurbsCurve(_) => "nurbs",
                _ => "other",
            }
        );
        let rt = curve.range_tuple();
        let er = curve.evaluation_range();
        println!("  range_tuple=({rt:?})");
        println!("  evaluation_range=({er:?})");
        println!("  period={:?}", curve.period());
        for t in [rt.0, er.0, rt.1, er.1] {
            let p = curve.subs(t);
            let po = p.distance(Point3::origin());
            println!(
                "  subs({t:.6}) = ({:.6},{:.6},{:.6})  |p|= {po:.3e}  POU={}",
                p.x,
                p.y,
                p.z,
                curve.basis_is_partition_of_unity(t)
            );
        }
        match curve {
            Curve3D::BSplineCurve(bsp) => {
                dump_bsp(bsp, rt, er);
            }
            Curve3D::NurbsCurve(nurbs) => {
                dump_nurbs(nurbs, rt, er);
            }
            _ => {}
        }
        // subs(t + 1.0) vs subs(t): is the curve periodic with period 1.0?
        let p0 = curve.subs(er.0);
        let p1 = curve.subs(er.0 + 1.0);
        let p2 = curve.subs(er.0 + 0.24);
        let p3 = curve.subs(er.0 + 1.24);
        println!(
            "  periodicity: subs(0) vs subs(1) d={:.3e}; subs(0.24) vs subs(1.24) d={:.3e}",
            p0.distance(p1),
            p2.distance(p3)
        );
        // Parametric residual field: distance from curve(t) to each source
        // vertex over the declared range, coarse scan.
        println!("  === fine residual scan over declared range {rt:?} ===");
        let mut best_a: (f64, f64) = (f64::INFINITY, 0.0);
        let mut best_b: (f64, f64) = (f64::INFINITY, 0.0);
        let n = 8192;
        for i in 0..=n {
            let t = rt.0 + (rt.1 - rt.0) * i as f64 / n as f64;
            let p = curve.subs(t);
            let da = p.distance(pv_a);
            let db = p.distance(pv_b);
            if da < best_a.0 {
                best_a = (da, t);
            }
            if db < best_b.0 {
                best_b = (db, t);
            }
        }
        println!(
            "  best_a: dist={:.3e} at t={:.6} (POU={})",
            best_a.0,
            best_a.1,
            curve.basis_is_partition_of_unity(best_a.1)
        );
        println!(
            "  best_b: dist={:.3e} at t={:.6} (POU={})",
            best_b.0,
            best_b.1,
            curve.basis_is_partition_of_unity(best_b.1)
        );
        println!("  coarse residual scan over eval range, da<5e-2 or db<5e-2:");
        for i in 0..=256 {
            let t = er.0 + (er.1 - er.0) * i as f64 / 256.0;
            let p = curve.subs(t);
            let da = p.distance(pv_a);
            let db = p.distance(pv_b);
            if da < 5e-2 || db < 5e-2 {
                println!(
                    "  t={t:.6} da={da:.3e} db={db:.3e} p=({:.4},{:.4},{:.4})",
                    p.x, p.y, p.z
                );
            }
        }
        // Full loop sample so we can see the shape over the genuine interval.
        println!("  loop over eval range:");
        for i in 0..=16 {
            let t = er.0 + (er.1 - er.0) * i as f64 / 16.0;
            let p = curve.subs(t);
            println!("    t={t:.4} p=({:.5},{:.5},{:.5})", p.x, p.y, p.z);
        }
        // The faces that use this edge, and the other edges of each boundary.
        for (fi, face) in cshell.faces.iter().enumerate() {
            let id = face.provenance.best_id().map(|id| id.get()).unwrap_or(0);
            if let Some(fid) = face_filter
                && id != fid
            {
                continue;
            }
            let uses: Vec<(usize, usize, bool)> = face
                .boundaries
                .iter()
                .enumerate()
                .flat_map(|(bi, wire)| {
                    wire.iter()
                        .enumerate()
                        .filter(|(_, idx)| idx.index == target)
                        .map(|(ui, idx)| (bi, ui, idx.orientation))
                        .collect::<Vec<_>>()
                })
                .collect();
            if !uses.is_empty() {
                println!("  FACE {id} (shell face idx {fi}) uses edge: uses={uses:?}");
                for (bi, wire) in face.boundaries.iter().enumerate() {
                    for (ui, idx) in wire.iter().enumerate() {
                        let e = &cshell.edges[idx.index];
                        let (x0, x1) = e.vertices;
                        let (va_, vb_) = (cshell.vertices[x0], cshell.vertices[x1]);
                        println!(
                            "    bound={bi} use={ui} edge_idx={} ori={} verts=({x0},{x1}) \
                             a=({:.6},{:.6},{:.6}) b=({:.6},{:.6},{:.6})",
                            idx.index, idx.orientation, va_.x, va_.y, va_.z, vb_.x, vb_.y, vb_.z
                        );
                    }
                }
                println!(
                    "    surface family: {:?}",
                    look::step_support_schema_of(&face.surface)
                );
            }
        }
    }
    Ok(())
}

fn dump_bsp(
    bsp: &truck_stepio::r#in::step_geometry::BSplineCurve<Point3>,
    rt: (f64, f64),
    er: (f64, f64),
) {
    use truck_stepio::r#in::step_geometry::ParametricCurve;
    println!("  degree={}", bsp.degree());
    let knots = bsp.knot_vec();
    let (distinct, mults) = knots.to_single_multi();
    println!("  distinct_knots={distinct:?}");
    println!("  multiplicities={mults:?}");
    println!("  control_point_count={}", bsp.control_points().len());
    println!("  control_points:");
    for (i, p) in bsp.control_points().iter().enumerate() {
        println!("    [{i}] ({:.6},{:.6},{:.6})", p.x, p.y, p.z);
    }
    // basis sum around the interesting points
    println!("  basis_sum scan:");
    let interesting = [
        rt.0,
        (rt.0 + er.0) / 2.0,
        er.0,
        er.0 + 1e-3,
        er.1 - 1e-3,
        er.1,
        (er.1 + rt.1) / 2.0,
        rt.1,
    ];
    for t in interesting {
        if t >= rt.0 && t <= rt.1 {
            let sum = basis_sum_bsp(bsp, t);
            let p = bsp.subs(t);
            println!(
                "    t={t:.6} basis_sum={sum:.6} POU={} p=({:.4},{:.4},{:.4}) |p|={:.3e}",
                bsp.basis_is_partition_of_unity(t),
                p.x,
                p.y,
                p.z,
                p.distance(Point3::origin())
            );
        }
    }
}

fn dump_nurbs(
    nurbs: &truck_stepio::r#in::step_geometry::NurbsCurve<
        truck_stepio::r#in::step_geometry::Vector4,
    >,
    rt: (f64, f64),
    er: (f64, f64),
) {
    use truck_stepio::r#in::step_geometry::ParametricCurve;
    println!("  degree={}", nurbs.degree());
    let knots = nurbs.knot_vec();
    let (distinct, mults) = knots.to_single_multi();
    println!("  distinct_knots={distinct:?}");
    println!("  multiplicities={mults:?}");
    println!("  control_point_count={}", nurbs.control_points().len());
    println!("  control_points (xyz, w):");
    for (i, h) in nurbs.control_points().iter().enumerate() {
        println!(
            "    [{i}] ({:.6},{:.6},{:.6}) w={:.6}",
            h.truncate().x,
            h.truncate().y,
            h.truncate().z,
            h.w
        );
    }
    println!("  basis_sum scan:");
    let interesting = [
        rt.0,
        (rt.0 + er.0) / 2.0,
        er.0,
        er.0 + 1e-3,
        er.1 - 1e-3,
        er.1,
        (er.1 + rt.1) / 2.0,
        rt.1,
    ];
    for t in interesting {
        if t >= rt.0 && t <= rt.1 {
            let sum = basis_sum_nurbs(nurbs, t);
            let p = nurbs.subs(t);
            println!(
                "    t={t:.6} basis_sum={sum:.6} POU={} p=({:.4},{:.4},{:.4}) |p|={:.3e}",
                nurbs.basis_is_partition_of_unity(t),
                p.x,
                p.y,
                p.z,
                p.distance(Point3::origin())
            );
        }
    }
}

fn basis_sum_bsp(bsp: &truck_stepio::r#in::step_geometry::BSplineCurve<Point3>, t: f64) -> f64 {
    let window = bsp.knot_vec().bspline_basis_functions(bsp.degree(), 0, t);
    window.as_slice().iter().sum()
}

fn basis_sum_nurbs(
    nurbs: &truck_stepio::r#in::step_geometry::NurbsCurve<
        truck_stepio::r#in::step_geometry::Vector4,
    >,
    t: f64,
) -> f64 {
    let window = nurbs
        .knot_vec()
        .bspline_basis_functions(nurbs.degree(), 0, t);
    window.as_slice().iter().sum()
}
