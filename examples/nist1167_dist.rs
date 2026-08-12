//! Direct distance check: do the boundary curves #606/#607 lie on surface #506?
//! Measures each boundary curve's control points and samples against the
//! surface by nearest parameter, and compares `search_parameter` vs
//! `search_nearest_parameter` residuals.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Curve3D};

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
    if args.is_empty() {
        eprintln!("usage: nist1167_dist MODEL.step");
        return Ok(());
    }
    let table = load(&args[0])?;
    for (&shell_id, shell) in table.shell.iter() {
        let (cshell, _) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        for (fi, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let g = id.get();
            if g != 1167 && g != 1169 {
                continue;
            }
            println!("=== face #{g} index {fi} ===");
            for (wi, wire) in face.boundaries.iter().enumerate() {
                for (ei, e) in wire.iter().enumerate() {
                    let curve = &cshell.edges[e.index].curve;
                    let (t0, t1) = curve.range_tuple();
                    // 1. control-point distances
                    let cps: Vec<Point3> = if let Curve3D::BSplineCurve(c) = curve {
                        c.control_points().to_vec()
                    } else {
                        Vec::new()
                    };
                    if !cps.is_empty() {
                        let mut best = f64::INFINITY;
                        let mut worst = 0.0f64;
                        let mut best_uv = None;
                        for p in &cps {
                            let np = face
                                .surface
                                .search_nearest_parameter(*p, None, 200)
                                .unwrap();
                            let res = face.surface.subs(np.0, np.1).distance(*p);
                            best = best.min(res);
                            worst = worst.max(res);
                            if best_uv.is_none() {
                                best_uv = Some(np);
                            }
                        }
                        println!(
                            "  bound[{wi}] edge[{ei}] n_ctrl={} ctrl_dist min={best:.3e} max={worst:.3e}",
                            cps.len()
                        );
                    }
                    // 2. per-sample residual histogram over the reported range
                    let n = 128;
                    let mut bins: Vec<(f64, f64, usize)> = vec![
                        (0.0, 1e-6, 0),
                        (1e-6, 1e-3, 0),
                        (1e-3, 1.0, 0),
                        (1.0, f64::INFINITY, 0),
                    ];
                    let mut search_ok = 0usize;
                    let mut prev: Option<(f64, f64)> = None;
                    for s in 0..=n {
                        let t = t0 + (t1 - t0) * (s as f64 / n as f64);
                        let pt = curve.subs(t);
                        if let Some(uv) = face.surface.search_parameter(pt, prev, 200) {
                            let res = face.surface.subs(uv.0, uv.1).distance(pt);
                            search_ok += 1;
                            for (i, (lo, hi, _)) in bins.iter_mut().enumerate() {
                                if res >= *lo && res <= *hi {
                                    bins[i].2 += 1;
                                    break;
                                }
                            }
                            prev = Some(uv);
                        } else {
                            // search_parameter failed: classify by nearest residual
                            let np = face
                                .surface
                                .search_nearest_parameter(pt, prev, 200)
                                .or_else(|| face.surface.search_nearest_parameter(pt, None, 200))
                                .unwrap();
                            let res = face.surface.subs(np.0, np.1).distance(pt);
                            for (i, (lo, hi, _)) in bins.iter_mut().enumerate() {
                                if res >= *lo && res <= *hi {
                                    bins[i].2 += 1;
                                    break;
                                }
                            }
                        }
                    }
                    let hist = bins
                        .iter()
                        .map(|(lo, hi, c)| format!("[{lo:.0e},{hi:.0e})={c}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "  bound[{wi}] edge[{ei}] range=({t0:.4},{t1:.4}) samples={} search_parameter_ok={search_ok} resid {hist}",
                        n + 1
                    );
                }
            }
        }
    }
    Ok(())
}
