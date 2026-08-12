//! Reference geometry: build the intended annular-band mesh for face #1167
//! directly by sampling surface #506 over the genuine domain u in [u_inner, 1],
//! v in [0, 1], bounded by the two closed loops. This is the geometry a correct
//! periodic render should produce; used as a certificate comparison target.

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
        eprintln!("usage: nist1167_reference MODEL.step");
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
            // The inner boundary loop's u range is roughly [0.57, 0.77]; the
            // band material region is u in [0.6, 1.0] (with margin). Sample a
            // regular grid and report area + extents.
            for (umin, tag) in [
                (0.55f64, "u[0.55,1]"),
                (0.60, "u[0.60,1]"),
                (0.65, "u[0.65,1]"),
            ] {
                let nu = 48;
                let nv = 96;
                let mut area = 0.0;
                let mut min_p = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
                let mut max_p =
                    Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
                for i in 0..nu {
                    for j in 0..nv {
                        let u = umin + (1.0 - umin) * (i as f64 + 0.5) / nu as f64;
                        let v = (j as f64 + 0.5) / nv as f64;
                        let du = src.uder(u, v);
                        let dv = src.vder(u, v);
                        area +=
                            du.cross(dv).magnitude() * (1.0 - umin) / nu as f64 * (1.0 / nv as f64);
                        let p = src.subs(u, v);
                        min_p.x = min_p.x.min(p.x);
                        min_p.y = min_p.y.min(p.y);
                        min_p.z = min_p.z.min(p.z);
                        max_p.x = max_p.x.max(p.x);
                        max_p.y = max_p.y.max(p.y);
                        max_p.z = max_p.z.max(p.z);
                    }
                }
                let diag = (max_p - min_p).magnitude();
                let (mpx0, mpy0, mpz0) = (min_p.x, min_p.y, min_p.z);
                let (mpx1, mpy1, mpz1) = (max_p.x, max_p.y, max_p.z);
                println!(
                    "#{g} reference band {tag}: area={area:.1} bbox_diag={diag:.2} \
                     x[{mpx0:.1},{mpx1:.1}] y[{mpy0:.1},{mpy1:.1}] z[{mpz0:.1},{mpz1:.1}]"
                );
            }
        }
    }
    Ok(())
}
