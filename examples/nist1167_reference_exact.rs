//! Trim-exact reference for the #1167/#1169 band: integrate the surface area
//! of the material region `u_inner(v) <= u <= 1` over one V period, where
//! `u_inner(v)` is derived by sampling the source inner trim curve (#606) and
//! brute-force projecting each sample onto the surface's (u,v) grid.
//!
//! The earlier rectangular references (u >= 0.55/0.60/0.65) are only
//! order-of-magnitude: the real inner boundary is not constant-U, so a
//! rectangle understates the band where the inner boundary dips below the
//! rectangle and overstates it where the inner boundary rises above it.
//!
//! This is a probe only; it is not official NIST ground truth.

use std::env;

use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{Table, step_geometry::Surface};

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
        eprintln!("usage: nist1167_reference_exact MODEL.step");
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
            let src = &cshell.faces[fi].surface;
            // The inner trim is boundary 0 (the non-rim loop). The outer rim is
            // the u=1 boundary.
            let (t0, t1) = cshell.edges[face.boundaries[0][0].index]
                .curve
                .range_tuple();
            let inner_curve = &cshell.edges[face.boundaries[0][0].index].curve;
            // Brute-force projection of the inner trim onto (u,v). The surface
            // is smooth and the true parameter lies in [0.5,1]x[0,1], so a
            // grid search over that box is reliable and search-independent.
            const NT: usize = 512;
            const NU: usize = 120;
            const NV: usize = 256;
            let mut inner_uv: Vec<(f64, f64)> = Vec::with_capacity(NT + 1);
            for s in 0..=NT {
                let t = t0 + (t1 - t0) * (s as f64 / NT as f64);
                let pt = inner_curve.subs(t);
                let mut best = (f64::INFINITY, (0.5, 0.0));
                for i in 0..=NU {
                    let u = 0.5 + 0.5 * (i as f64 / NU as f64);
                    for j in 0..=NV {
                        let v = j as f64 / NV as f64;
                        let d = src.subs(u, v).distance(pt);
                        if d < best.0 {
                            best = (d, (u, v));
                        }
                    }
                }
                if best.0 > 1.0e-1 {
                    println!(
                        "WARN #{g} inner sample {s} projected with residual {:.3e}",
                        best.0
                    );
                }
                inner_uv.push(best.1);
            }
            let (mut u_min, mut u_max) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut v_min, mut v_max) = (f64::INFINITY, f64::NEG_INFINITY);
            for &(u, v) in &inner_uv {
                u_min = u_min.min(u);
                u_max = u_max.max(u);
                v_min = v_min.min(v);
                v_max = v_max.max(v);
            }
            // Integrate the band area: for each (v,u) cell, include it when u
            // is above the inner boundary at that v. Interpolate u_inner(v)
            // from the sampled boundary (v is monotone along the trim; handle
            // the wrap by treating v as a circle).
            let v_to_inner = |v: f64| -> f64 {
                let v = v - v_min; // shift so the trim starts near v=0
                let span = v_max - v_min;
                let v = v - span * (v / span).floor();
                let idx = (v / span * (NT as f64)).round() as usize;
                let idx = idx.min(NT);
                inner_uv[idx].0
            };
            let mut area = 0.0f64;
            let mut count_in = 0usize;
            let nv = 720;
            let nu = 240;
            for i in 0..nu {
                let u = 0.5 + 0.5 * ((i as f64 + 0.5) / nu as f64);
                for j in 0..nv {
                    let v = (j as f64 + 0.5) / nv as f64;
                    if u < v_to_inner(v) {
                        continue;
                    }
                    count_in += 1;
                    let du = src.uder(u, v);
                    let dv = src.vder(u, v);
                    area += du.cross(dv).magnitude() * (0.5 / nu as f64) * (1.0 / nv as f64);
                }
            }
            println!(
                "#{g} trim-exact band: area={area:.1} inner_u∈[{u_min:.4},{u_max:.4}] \
                 inner_v∈[{v_min:.4},{v_max:.4}] cells_in={count_in}/{}",
                nu * nv
            );
            // The rectangular references for comparison.
            for (umin, tag) in [
                (0.50f64, "u[0.50,1]"),
                (0.55, "u[0.55,1]"),
                (0.60, "u[0.60,1]"),
                (0.65, "u[0.65,1]"),
            ] {
                let mut rect = 0.0f64;
                for i in 0..nu {
                    let u = umin + (1.0 - umin) * ((i as f64 + 0.5) / nu as f64);
                    for j in 0..nv {
                        let v = (j as f64 + 0.5) / nv as f64;
                        let du = src.uder(u, v);
                        let dv = src.vder(u, v);
                        rect += du.cross(dv).magnitude()
                            * ((1.0 - umin) / nu as f64)
                            * (1.0 / nv as f64);
                    }
                }
                println!("  #{g} rectangle {tag}: area={rect:.1}");
            }
        }
    }
    Ok(())
}
