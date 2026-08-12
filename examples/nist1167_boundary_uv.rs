//! Diagnostic: where do the two boundary curves of face #1167/#1169 actually
//! lie on their surfaces? Uses `search_nearest_parameter` (always converges) to
//! measure the true UV footprint and residual of every boundary sample.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{
    Table,
    step_geometry::{Curve3D, Surface},
};

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
        eprintln!("usage: nist1167_boundary_uv MODEL.step");
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
            println!(
                "surface range u={:?} v={:?}",
                face.surface.try_range_tuple().0,
                face.surface.try_range_tuple().1
            );
            for (wi, wire) in face.boundaries.iter().enumerate() {
                for (ei, e) in wire.iter().enumerate() {
                    let curve = &cshell.edges[e.index].curve;
                    let (t0, t1) = curve.range_tuple();
                    let n = 65;
                    let mut min_u = f64::INFINITY;
                    let mut max_u = f64::NEG_INFINITY;
                    let mut min_v = f64::INFINITY;
                    let mut max_v = f64::NEG_INFINITY;
                    let mut max_res = 0.0f64;
                    let mut ok = 0usize;
                    let mut chain: Vec<String> = Vec::new();
                    let mut prev: Option<(f64, f64)> = None;
                    for s in 0..=n {
                        let t = t0 + (t1 - t0) * (s as f64 / n as f64);
                        let pt = curve.subs(t);
                        // nearest always converges; report its residual
                        let np = face
                            .surface
                            .search_nearest_parameter(pt, prev, 100)
                            .unwrap_or_else(|| {
                                face.surface
                                    .search_nearest_parameter(pt, None, 100)
                                    .unwrap()
                            });
                        let res = face.surface.subs(np.0, np.1).distance(pt);
                        max_res = max_res.max(res);
                        min_u = min_u.min(np.0);
                        max_u = max_u.max(np.0);
                        min_v = min_v.min(np.1);
                        max_v = max_v.max(np.1);
                        ok += 1;
                        prev = Some(np);
                        if s % 8 == 0 {
                            chain.push(format!("({:.3},{:.3})", np.0, np.1));
                        }
                    }
                    println!(
                        "  bound[{wi}] edge[{ei}] curve range=({t0:.4},{t1:.4}) \
                         nearest UV u∈[{min_u:.4},{max_u:.4}] v∈[{min_v:.4},{max_v:.4}] \
                         max_res={max_res:.3e} ok={ok}"
                    );
                    println!("    chain: {}", chain.join(" -> "));
                }
            }
        }
    }
    Ok(())
}
