//! Probe: for a given model + source_face_id, run the R01 source-edge
//! traversal on every boundary edge and report the verdict and the exact
//! `Unresolved` reason. Diagnostic only.

use std::env;

use truck_meshalgo::prelude::*;
use truck_meshalgo::tessellation::source_edge::{
    SourceEdgeTraversal, establish_source_edge_traversal,
};
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
    if args.len() < 2 {
        eprintln!("usage: r01_edge_probe MODEL.step FACE_ID [FACE_ID ...]");
        return Ok(());
    }
    let model_path = &args[0];
    let targets: Vec<u64> = args[1..].iter().filter_map(|a| a.parse().ok()).collect();

    let table = load(model_path)?;
    for (&shell_id, shell) in table.shell.iter() {
        let (cshell, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let source_tolerance = cshell
            .source_geometric_uncertainty
            .filter(|u| u.is_finite() && *u > 0.0)
            .unwrap_or(truck_meshalgo::tessellation::source_edge::SOURCE_INCIDENCE_TOLERANCE);
        println!(
            "shell #{shell_id}: source_geometric_uncertainty={:?}  source_tolerance={source_tolerance:.3e}",
            cshell.source_geometric_uncertainty
        );
        for (fi, face) in cshell.faces.iter().enumerate() {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let id_u64 = id.get();
            if !targets.contains(&id_u64) {
                continue;
            }
            println!(
                "FACE #{id_u64} (idx {fi}) boundaries={}",
                face.boundaries.len()
            );
            for (bi, boundary) in face.boundaries.iter().enumerate() {
                for edge_index in boundary {
                    let edge = &cshell.edges[edge_index.index];
                    let start_pos = cshell.vertices[edge.vertices.0];
                    let end_pos = cshell.vertices[edge.vertices.1];
                    let closed = edge.vertices.0 == edge.vertices.1;
                    let verdict = establish_source_edge_traversal(
                        &edge.curve,
                        start_pos,
                        end_pos,
                        closed,
                        source_tolerance,
                        // The caller's chord tolerance. Override with
                        // `PROBE_CALLER_TOL` to match production
                        // `model.diameter() * 0.001`.
                        env::var("PROBE_CALLER_TOL")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1.0e-3),
                    );
                    let (range_lo, range_hi) = edge.curve.evaluation_range();
                    let msg = match &verdict {
                        SourceEdgeTraversal::CanonicalByEvalRange { range } => {
                            format!("CanonicalByEvalRange {range:?}")
                        }
                        SourceEdgeTraversal::CanonicalBySourceInterval { traversal, witness } => {
                            format!(
                                "CanonicalBySourceInterval {traversal:?} res=({:.3e},{:.3e}) cand=({},{})",
                                witness.start_residual,
                                witness.end_residual,
                                witness.start_candidates,
                                witness.end_candidates
                            )
                        }
                        SourceEdgeTraversal::Unresolved { reason } => {
                            format!("UNRESOLVED: {reason}")
                        }
                    };
                    // For an unresolved verdict, also report where the shared
                    // vertex's nearest on-curve residual sits (the raw
                    // `search_parameter` distance), to distinguish a genuine
                    // multi-root geometry from a strict-uniqueness false alarm.
                    let shared_probe = if matches!(verdict, SourceEdgeTraversal::Unresolved { .. })
                    {
                        let (rt0, rt1) =
                            edge.curve.try_range_tuple().unwrap_or((range_lo, range_hi));
                        let mut hints: Vec<String> = vec![format!("declared=({rt0:.4},{rt1:.4})")];
                        for (name, p) in [("start", start_pos), ("end", end_pos)] {
                            match edge.curve.search_parameter(p, None, 500) {
                                Some(t) => {
                                    let res = edge.curve.subs(t).distance(p);
                                    let pu = edge.curve.basis_is_partition_of_unity(t);
                                    hints.push(format!("{name}:t={t:.4} res={res:.3e} pu={pu}"));
                                }
                                None => hints.push(format!("{name}:no_search_root")),
                            }
                            // Coarse in-domain minimum residual: the closest the
                            // curve gets to the vertex inside [lo,hi].
                            let mut best = f64::INFINITY;
                            let mut best_t = 0.0f64;
                            let n = 65536;
                            let step = (range_hi - range_lo) / n as f64;
                            for i in 0..=n {
                                let t = range_lo + step * i as f64;
                                let d = edge.curve.subs(t).distance(p);
                                if d < best {
                                    best = d;
                                    best_t = t;
                                }
                            }
                            hints
                                .push(format!("{name}:in_domain_min_res={best:.3e}@t={best_t:.4}"));
                            // Scan a 2% margin past each evaluator end, to see
                            // whether the vertex is realized just outside the
                            // domain (root just past the end) rather than off
                            // the curve everywhere.
                            let margin = (range_hi - range_lo) * 0.02;
                            let mut bestm = f64::INFINITY;
                            let mut bestm_t = 0.0f64;
                            for i in 0..=2000 {
                                let t = range_lo - margin
                                    + (range_hi - range_lo + 2.0 * margin) * i as f64 / 2000.0;
                                let d = edge.curve.subs(t).distance(p);
                                if d < bestm {
                                    bestm = d;
                                    bestm_t = t;
                                }
                            }
                            hints.push(format!(
                                "{name}:margined_min_res={bestm:.3e}@t={bestm_t:.4}"
                            ));
                        }
                        let joined = hints.join(" | ");
                        format!("  [{joined}]")
                    } else {
                        String::new()
                    };
                    println!(
                        "  bound {bi} edge idx {} verts {:?} closed={closed} eval=({range_lo:.4},{range_hi:.4}) {msg}{shared_probe}",
                        edge_index.index, edge.vertices
                    );
                }
            }
        }
    }
    Ok(())
}
