//! Scratch probe (Phase 1): exact parameter roots for the shared spline edge's
//! source vertices, the oriented arc traversal, bound-closure under that
//! traversal, and swept-surface compatibility.
//! Usage: spline_edge_00007667_solve MODEL.step --face 19018

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
        let mut shared: Option<usize> = None;
        for wire in &face.boundaries {
            for idx in wire {
                let rt = cshell.edges[idx.index].curve.range_tuple();
                if rt.0 < -0.4 {
                    shared = Some(idx.index);
                }
            }
        }
        let Some(si) = shared else { continue };
        let edge = &cshell.edges[si];
        let (va, vb) = edge.vertices;
        let pv_a = cshell.vertices[va];
        let pv_b = cshell.vertices[vb];
        let curve = &edge.curve;
        let er = curve.evaluation_range();
        println!("face {face_filter} shared edge idx={si} vertices=({va},{vb}) er={er:?}");
        println!("  pv_a={pv_a:?}");
        println!("  pv_b={pv_b:?}");
        println!(
            "  curve family: {}",
            match curve {
                Curve3D::BSplineCurve(_) => "bspline",
                Curve3D::NurbsCurve(_) => "nurbs",
                _ => "other",
            }
        );

        // ---- exact root solving: find t in [0,1] minimizing |C(t) - v| ----
        let t_for = |v: Point3| -> (f64, f64) {
            // bracket by coarse scan then golden-section refine
            let n = 4096;
            let mut best = (f64::INFINITY, 0.0);
            for i in 0..=n {
                let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
                let d = curve.subs(t).distance(v);
                if d < best.0 {
                    best = (d, t);
                }
            }
            let (mut lo, mut hi) = (
                best.1 - (er.1 - er.0) / n as f64,
                best.1 + (er.1 - er.0) / n as f64,
            );
            lo = lo.max(er.0);
            hi = hi.min(er.1);
            // golden-section
            const PHI: f64 = 1.618033988749895;
            let mut a = lo;
            let mut b = hi;
            let mut c = b - (b - a) / PHI;
            let mut d = a + (b - a) / PHI;
            for _ in 0..80 {
                if (c - d).abs() < 1e-15 {
                    break;
                }
                if curve.subs(c).distance(v) < curve.subs(d).distance(v) {
                    b = d;
                } else {
                    a = c;
                }
                c = b - (b - a) / PHI;
                d = a + (b - a) / PHI;
            }
            let t = (a + b) / 2.0;
            let dist = curve.subs(t).distance(v);
            // Also scan the whole range for a second, distinct local min.
            let mut second = (f64::INFINITY, 0.0);
            for i in 0..=n {
                let t = er.0 + (er.1 - er.0) * i as f64 / n as f64;
                let d = curve.subs(t).distance(v);
                if (t - best.1).abs() > 0.2 && d < second.0 {
                    second = (d, t);
                }
            }
            if second.0 < 1e-6 {
                // there is a second arc near this vertex
                eprintln!(
                    "  WARNING second candidate for vertex: t={} d={:.3e}",
                    second.1, second.0
                );
            }
            (t, dist)
        };

        let (t_a, r_a) = t_for(pv_a);
        let (t_b, r_b) = t_for(pv_b);
        println!(
            "  root for pv_a: t={t_a:.9} residual={r_a:.3e} POU={}",
            curve.basis_is_partition_of_unity(t_a)
        );
        println!(
            "  root for pv_b: t={t_b:.9} residual={r_b:.3e} POU={}",
            curve.basis_is_partition_of_unity(t_b)
        );

        // ---- Oriented traversal. The edge runs from pv_a to pv_b (same_sense
        // .T., vertices (front=a, back=b)). The curve is a closed loop over
        // [0,1] with C(0)=C(1). Increasing-parameter arc from t_a to t_b wraps
        // through the seam if t_a > t_b.
        let wrap = t_a > t_b;
        println!("  wrap through seam (t_a > t_b): {wrap}");
        let span = if wrap { t_b + 1.0 - t_a } else { t_b - t_a };
        println!("  arc parameter span (increasing, seam-wrapped): {span:.6}");
        // Sample the traversal as (normalized) polyline of N samples.
        const N: usize = 64;
        let sample = |tau: f64| -> Point3 {
            let t = if tau > 1.0 { tau - 1.0 } else { tau };
            curve.subs(t)
        };
        let pts: Vec<Point3> = (0..=N)
            .map(|i| {
                let tau = t_a + (span) * i as f64 / N as f64;
                sample(tau)
            })
            .collect();
        let first = pts[0];
        let last = pts[N];
        println!("  sampled arc first={first:?} last={last:?}");
        println!("  arc first vs pv_a: {:.3e}", first.distance(pv_a));
        println!("  arc last  vs pv_b: {:.3e}", last.distance(pv_b));
        println!(
            "  arc interior POU all true: {}",
            (0..=N).all(|i| {
                let tau = t_a + span * i as f64 / N as f64;
                curve.basis_is_partition_of_unity(if tau > 1.0 { tau - 1.0 } else { tau })
            })
        );
        // mid-sample sanity: not origin
        let mut minmag = f64::INFINITY;
        for p in &pts {
            minmag = minmag.min(p.distance(Point3::origin()));
        }
        println!("  arc min |p|: {minmag:.3e} (must be >> 0)");

        // ---- bound closure: the paired edge in bound 0 ----
        for (bi, wire) in face.boundaries.iter().enumerate() {
            if !wire.iter().any(|i| i.index == si) {
                continue;
            }
            let shared_ori = wire
                .iter()
                .find(|i| i.index == si)
                .map(|i| i.orientation)
                .unwrap();
            let shared_forward = if shared_ori {
                (pv_a, pv_b)
            } else {
                (pv_b, pv_a)
            };
            println!(
                "  bound {bi} contains shared edge (ori={shared_ori}); shared_forward from {:?} to {:?}",
                shared_forward.0, shared_forward.1
            );
            for (ui, idx) in wire.iter().enumerate() {
                if idx.index == si {
                    continue;
                }
                let pe = &cshell.edges[idx.index];
                let (x0, x1) = pe.vertices;
                let (q0, q1) = (cshell.vertices[x0], cshell.vertices[x1]);
                let paired_forward = match idx.orientation {
                    true => (q0, q1),
                    false => (q1, q0),
                };
                println!(
                    "    paired edge bound {bi} use {ui} idx={} ori={} verts=({x0},{x1})",
                    idx.index, idx.orientation
                );
                println!(
                    "      paired_forward from {:?} to {:?}",
                    paired_forward.0, paired_forward.1
                );
                let closes_pair_first = paired_forward.1.distance(shared_forward.0) < 1e-4;
                let closes_pair_second = shared_forward.1.distance(paired_forward.0) < 1e-4;
                println!(
                    "      closes (paired then shared): {closes_pair_first} | (shared then paired): {closes_pair_second}"
                );
            }
        }

        // ---- swept-surface compatibility ----
        let surf = &face.surface;
        println!("  surface family: {:?}", look::step_support_schema_of(surf));
        // Check the arc lies on the surface: sample each arc point and compute
        // nearest-surface residual via search_parameter if available.
        let mut max_res = 0.0_f64;
        let mut n_on = 0usize;
        for p in &pts {
            let res = nearest_surface_residual(surf, *p);
            if res < 1e-4 {
                n_on += 1;
            }
            max_res = max_res.max(res);
        }
        println!(
            "  surface residual over arc: max={max_res:.3e}, points within 1e-4: {n_on}/{}",
            pts.len()
        );

        // Exact structural check: for a swept/extruded surface the edge curve
        // should be the profile translated along the extrusion direction.
        if let truck_stepio::r#in::step_geometry::Surface::SweptCurve(
            truck_stepio::r#in::step_geometry::SweptCurve::ExtrudedCurve(ext),
        ) = surf
        {
            let profile = ext.entity_curve();
            let dir = ext.extruding_vector();
            println!(
                "  extruded surface: profile={:?} dir=({:.4},{:.4},{:.4})",
                match profile {
                    Curve3D::BSplineCurve(_) => "bspline",
                    Curve3D::NurbsCurve(_) => "nurbs",
                    _ => "other",
                },
                dir.x,
                dir.y,
                dir.z
            );
            // Direct geometric check: for each sample p on the genuine arc, the
            // point p - v*dir should lie ON the profile curve image. v is fixed
            // (the extrusion level); profile distance uses a fine scan over the
            // profile's own parameter domain.
            let mut max_off = 0.0_f64;
            let mut used_v = 0.0_f64;
            let mut first = true;
            for p in &pts {
                // v = projection of (edge sample - some profile point) onto dir.
                // Use the profile start for a stable first estimate, then refine
                // by scanning the profile at that candidate level.
                let base = profile.subs(er.0);
                let d0 = p - base;
                let v = d0.dot(dir) / dir.magnitude2();
                // scan profile over [0,1] to find min distance to p - v*dir
                let q = p - dir * v;
                let mut best = f64::INFINITY;
                for i in 0..=4096 {
                    let t = er.0 + (er.1 - er.0) * i as f64 / 4096.0;
                    let d = profile.subs(t).distance(q);
                    if d < best {
                        best = d;
                    }
                }
                if first {
                    used_v = v;
                    first = false;
                }
                // measure deviation from a FIXED level: at the fixed v, how far
                // is p from the profile?
                let qf = p - dir * used_v;
                let mut best_fixed = f64::INFINITY;
                for i in 0..=4096 {
                    let t = er.0 + (er.1 - er.0) * i as f64 / 4096.0;
                    let d = profile.subs(t).distance(qf);
                    if d < best_fixed {
                        best_fixed = d;
                    }
                }
                max_off = max_off.max(best_fixed);
            }
            println!(
                "  on-surface residual at fixed extrusion level v={used_v:.6}: max={max_off:.3e}"
            );
        }
        break;
    }
    Ok(())
}

/// Brute-force nearest surface residual via search_parameter where possible.
fn nearest_surface_residual(surf: &truck_stepio::r#in::step_geometry::Surface, p: Point3) -> f64 {
    use truck_stepio::r#in::step_geometry::ParametricSurface;
    let (ur, vr) = surf.parameter_range();
    let (ulo, uhi) = range_bounds(ur);
    let (vlo, vhi) = range_bounds(vr);
    let mut best = f64::INFINITY;
    let n = 64;
    for i in 0..=n {
        let u = ulo + (uhi - ulo) * i as f64 / n as f64;
        for j in 0..=n {
            let q = surf.subs(u, vlo + (vhi - vlo) * j as f64 / n as f64);
            let d = q.distance(p);
            if d < best {
                best = d;
            }
        }
    }
    best
}

fn range_bounds(r: (std::ops::Bound<f64>, std::ops::Bound<f64>)) -> (f64, f64) {
    use std::ops::Bound;
    let lo = match r.0 {
        Bound::Included(x) | Bound::Excluded(x) => x,
        Bound::Unbounded => -1.0,
    };
    let hi = match r.1 {
        Bound::Included(x) | Bound::Excluded(x) => x,
        Bound::Unbounded => 1.0,
    };
    (lo, hi)
}
