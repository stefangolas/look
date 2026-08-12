//! Probe for the `#1167` source-semantic audit.
//!
//! Verifies numerically, for the B-spline surface underlying face `#1167` in
//! `nist_ctc_02_asme1_ap203.stp`:
//!   1. the evaluator's V-axis periodicity: `S(u, v) == S(u, v + P)`;
//!   2. the evaluator's U-axis status;
//!   3. the boundary curve images on the surface and their UV extents;
//!   4. the raw data `lattice_of` currently supplies to the tessellator.
//!
//! This is a probe only; it writes no production code.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};
use truck_topology::compress::{CompressedFace, CompressedShell};

type Cshell = CompressedShell<Point3, Curve3D, Surface>;

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

fn extract_target_face<'a>(
    cshell: &'a Cshell,
    target_spec: &str,
) -> Option<(usize, &'a CompressedFace<Surface>)> {
    if let Ok(idx) = target_spec.parse::<usize>()
        && idx < cshell.faces.len()
    {
        return Some((idx, &cshell.faces[idx]));
    }
    let target_str = target_spec.trim_start_matches('#');
    for (idx, face) in cshell.faces.iter().enumerate() {
        if let Some(id) = face.provenance.best_id() {
            let s = id.to_string();
            if s == target_spec || s == target_str || s.contains(target_str) {
                return Some((idx, face));
            }
        }
    }
    None
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: nist1167_periodicity_probe MODEL.step [FACE_ID_OR_INDEX]");
        return Ok(());
    }
    let model_path = &args[0];
    let target_spec = args.get(1).map(|s| s.as_str()).unwrap_or("#1167");

    let table = load(model_path)?;
    let mut found = false;
    for (shell_idx, (&shell_id, shell)) in table.shell.iter().enumerate() {
        let (cshell, losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        if !losses.is_empty() {
            println!("Shell #{shell_idx}: {0} conversion losses", losses.len());
        }
        let Some((face_idx, face)) = extract_target_face(&cshell, target_spec) else {
            continue;
        };
        found = true;
        let id = face
            .provenance
            .best_id()
            .map(|id| format!("#{id}"))
            .unwrap_or_else(|| format!("idx_{face_idx}"));
        println!("=== face {id} (shell #{shell_idx}) ===");
        println!(
            "surface variant: {}",
            match &face.surface {
                Surface::ElementarySurface(_) => "elementary",
                Surface::SweptCurve(_) => "swept",
                Surface::BSplineSurface(_) => "bspline",
                Surface::NurbsSurface(_) => "nurbs",
                Surface::OffsetSurface(_) => "offset",
            }
        );
        let (ur, vr) = face.surface.try_range_tuple();
        println!("try_range_tuple: u={ur:?} v={vr:?}");
        println!("u_period(): {:?}", face.surface.u_period());
        println!("v_period(): {:?}", face.surface.v_period());
        let lattice = look::step_lattice_of(&face.surface);
        println!(
            "lattice_of: u_gen={:?} v_gen={:?} declared_u={:?} declared_v={:?}",
            lattice.u_generator(),
            lattice.v_generator(),
            lattice.declared_u_period(),
            lattice.declared_v_period(),
        );

        // Concrete representation facts (only when the surface is a B-spline).
        if let Surface::BSplineSurface(surf) = &face.surface {
            let (uk, vk) = surf.knot_vecs();
            let (ud, vd) = surf.degrees();
            let net = surf.control_points();
            println!("BSplineSurface: udegree={ud} vdegree={vd}");
            println!("  u knots ({0}): {1:?}", uk.len(), uk.as_slice());
            println!("  v knots ({0}): {1:?}", vk.len(), vk.as_slice());
            println!("  net dims: {} rows x {} cols", net.len(), net[0].len());
            println!("  u clamped: {}", uk.is_clamped(ud));
            println!("  v clamped: {}", vk.is_clamped(vd));
            // Control-net wrap: for a source-closed V axis, the first `vd`
            // columns should coincide with the last `vd` columns (or a
            // documented exporter subset thereof).
            for (i, row) in net.iter().enumerate() {
                let ncols = row.len();
                let wraps: Vec<String> = (1..=vd)
                    .map(|k| {
                        let a = &row[0];
                        let b = &row[ncols - k];
                        format!("{k}:{}", a.distance(*b))
                    })
                    .collect();
                println!("  row {i} V wrap: {}", wraps.join("  "));
            }
            let vklen = vk.len();
            let period = vk[vklen - 1 - vd] - vk[vd];
            println!("  v span [knot[{vd}], knot[{vklen}-1-{vd}]] => period candidate {period}");
            // Seam identification: a closed spline's active domain is
            // [knot[d], knot[n]]; the seam closes iff S(u, knot[d]) ==
            // S(u, knot[n]) for all u.
            let u0 = vk[vd];
            let u1 = vk[34.min(vklen - 1)];
            let mut seam_res = 0.0f64;
            for i in 0..9 {
                let u = 0.125 * i as f64;
                let a = surf.subs(u, u0);
                let b = surf.subs(u, u1);
                seam_res = seam_res.max(a.distance(b));
            }
            println!(
                "  V seam identification S(u,{u0}) vs S(u,{u1}): max residual = {seam_res:.3e}"
            );
            // U check: is S(u+1, v) == S(u, v)?
            let mut u_res = 0.0f64;
            for i in 0..5 {
                let u = 0.125 * i as f64;
                for j in 0..9 {
                    let v = 0.125 * j as f64;
                    let a = surf.subs(u, v);
                    let b = surf.subs(u + 1.0, v);
                    let d = a.distance(b);
                    u_res = u_res.max(d);
                }
            }
            println!("  U-period identity S(u+1,v) vs S(u,v): max residual = {u_res:.3e}");
        }

        for (wi, wire) in face.boundaries.iter().enumerate() {
            println!("  boundary[{wi}]: {} edges", wire.len());
            for (ei, e) in wire.iter().enumerate() {
                let curve = &cshell.edges[e.index].curve;
                let (t0, t1) = curve.range_tuple();
                let samples = 32;
                let mut min_v = f64::INFINITY;
                let mut max_v = f64::NEG_INFINITY;
                let mut min_u = f64::INFINITY;
                let mut max_u = f64::NEG_INFINITY;
                let mut max_res = 0.0f64;
                let mut first_uv = None;
                let mut last_uv = None;
                let mut previous: Option<(f64, f64)> = None;
                let mut ok = 0usize;
                for s in 0..=samples {
                    let t = t0 + (t1 - t0) * (s as f64 / samples as f64);
                    let pt = curve.subs(t);
                    let proj = face
                        .surface
                        .search_parameter(pt, previous, 200)
                        .or_else(|| face.surface.search_parameter(pt, None, 200));
                    match proj {
                        Some(uv) => {
                            let projected = face.surface.subs(uv.0, uv.1);
                            max_res = max_res.max(projected.distance(pt));
                            min_v = min_v.min(uv.1);
                            max_v = max_v.max(uv.1);
                            min_u = min_u.min(uv.0);
                            max_u = max_u.max(uv.0);
                            if first_uv.is_none() {
                                first_uv = Some(uv);
                            }
                            last_uv = Some(uv);
                            previous = Some(uv);
                            ok += 1;
                        }
                        None => {
                            let (px, py, pz) = (pt.x, pt.y, pt.z);
                            println!(
                                "    edge {ei} sample {s}: projection FAILED at ({px:.4},{py:.4},{pz:.4})"
                            )
                        }
                    }
                }
                println!("    edge {ei}: t∈[{t0:.6},{t1:.6}] curve3d={curve:?}");
                println!(
                    "      UV extent: u∈[{min_u:.6},{max_u:.6}] v∈[{min_v:.6},{max_v:.6}] res_max={max_res:.3e} ok={ok}"
                );
                println!("      first_uv={first_uv:?} last_uv={last_uv:?}");
            }
        }
        break;
    }
    if !found {
        anyhow::bail!("target face {target_spec} not found");
    }
    Ok(())
}
