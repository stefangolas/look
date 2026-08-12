//! Diagnostic: reproduce the production boundary sampling range for face
//! #1167's two edges (the self-loops on curves #606, #607), then project the
//! samples and print the UV path. This mirrors `tessellate_edge`'s range
//! computation exactly.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::Table;

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
        eprintln!("usage: nist1167_range MODEL.step");
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
                    let is_self_loop =
                        cshell.edges[e.index].vertices.0 == cshell.edges[e.index].vertices.1;
                    let (ev0, ev1) = curve.evaluation_range();
                    let mut range = (ev0, ev1);
                    let try_range = curve.try_range_tuple();
                    let pu0 = try_range.map(|(lo, _)| curve.basis_is_partition_of_unity(lo));
                    let pu1 = try_range.map(|(_, hi)| curve.basis_is_partition_of_unity(hi));
                    if is_self_loop {
                        if let Some((rt_lo, _)) = try_range
                            && rt_lo < range.0 - 1e-12
                            && curve.basis_is_partition_of_unity(rt_lo)
                        {
                            range.0 = rt_lo;
                        }
                        if let Some((_, rt_hi)) = try_range
                            && rt_hi > range.1 + 1e-12
                            && curve.basis_is_partition_of_unity(rt_hi)
                        {
                            range.1 = rt_hi;
                        }
                    }
                    let (r0, r1) = range;
                    println!(
                        "  bound[{wi}] edge[{ei}] self_loop={is_self_loop} \
                         eval_range=({ev0:.6},{ev1:.6}) try_range={try_range:?} \
                         pu_at_lo={pu0:?} pu_at_hi={pu1:?} prod_range=({r0:.6},{r1:.6})"
                    );
                    // Project over the production range, following hints.
                    let n = 64;
                    let mut chain_clean: Vec<String> = Vec::new();
                    let mut chain_wrapped: Vec<String> = Vec::new();
                    let mut prev_c: Option<(f64, f64)> = None;
                    let mut prev_w: Option<(f64, f64)> = None;
                    let mut min_u = f64::INFINITY;
                    let mut max_u = f64::NEG_INFINITY;
                    let mut min_v = f64::INFINITY;
                    let mut max_v = f64::NEG_INFINITY;
                    let mut max_res = 0.0f64;
                    let mut ok = 0usize;
                    let mut failed = 0usize;
                    let (rg0, rg1) = range;
                    let period = 1.0;
                    for s in 0..=n {
                        let t = rg0 + (rg1 - rg0) * (s as f64 / n as f64);
                        let pt = curve.subs(t);
                        let uv = face
                            .surface
                            .search_parameter(pt, prev_c, 200)
                            .or_else(|| face.surface.search_parameter(pt, None, 200))
                            .or_else(|| {
                                face.surface
                                    .search_nearest_parameter(pt, prev_c, 200)
                                    .or_else(|| {
                                        face.surface.search_nearest_parameter(pt, None, 200)
                                    })
                            });
                        match uv {
                            Some(uv) => {
                                let res = face.surface.subs(uv.0, uv.1).distance(pt);
                                max_res = max_res.max(res);
                                min_u = min_u.min(uv.0);
                                max_u = max_u.max(uv.0);
                                min_v = min_v.min(uv.1);
                                max_v = max_v.max(uv.1);
                                ok += 1;
                                // clean: raw projection
                                let (cu, cv) = uv;
                                if let Some((u0, _)) = prev_c {
                                    // no period -> no unwrap
                                    let _ = (u0, cu);
                                }
                                prev_c = Some((cu, cv));
                                if s % 4 == 0 {
                                    chain_clean.push(format!("({:.3},{:.3})", cu, cv));
                                }
                                // wrapped: apply get_mindiff on v with period 1.0
                                let (wu, mut wv) = uv;
                                if let Some((_, v0)) = prev_w {
                                    wv = wv + f64::round((v0 - wv) / period) * period;
                                }
                                prev_w = Some((wu, wv));
                                if s % 4 == 0 {
                                    chain_wrapped.push(format!("({:.3},{:.3})", wu, wv));
                                }
                            }
                            None => {
                                failed += 1;
                                prev_c = None;
                                prev_w = None;
                            }
                        }
                    }
                    println!(
                        "    proj ok={ok} failed={failed} res_max={max_res:.3e} \
                         UV u∈[{min_u:.3},{max_u:.3}] v∈[{min_v:.3},{max_v:.3}]"
                    );
                    println!("    clean chain:    {}", chain_clean.join(" -> "));
                    println!("    wrapped chain:  {}", chain_wrapped.join(" -> "));
                }
            }
        }
    }
    Ok(())
}
