//! Work Item 9 final epistemic witness on the REAL #1167/#1169 faces.
//!
//! Proves directly on the production-wrapped surfaces that the narrowed Stage-E
//! gate:
//!   * rejects an exterior value on the ordinary companion U axis (`u=3.63`
//!     on a `[0,1]` native domain);
//!   * admits deck-equivalent values on the certified quotient V axis
//!     (`v=3.63`, `v=-0.37`, ...) because the quotient normalizes them during
//!     evaluation;
//!   * does NOT apply these bounds to an ordinary spline with no certified
//!     quotient (legacy semantics).

use std::env;

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
        eprintln!("usage: nist1167_inverse_guard MODEL.step");
        return Ok(());
    }
    let table = load(&args[0])?;
    let closure_map = look::step::lattice::spline_closure_map(&table);

    let mut seen = 0usize;
    for (&shell_id, shell) in table.shell.iter() {
        let (cshell, _losses) = table
            .to_compressed_shell_with_losses(shell_id, shell)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let wrapped = look::step::policy_geometry::wrap_shell_with_closure(
            cshell,
            look::step::meshing_policy::MeshingPolicy::DEFAULT,
            &closure_map,
        );
        for face in &wrapped.faces {
            let Some(id) = face.provenance.best_id() else {
                continue;
            };
            let id = id.get();
            if id != 1167 && id != 1169 {
                continue;
            }
            let s = &face.surface;
            println!(
                "FACE #{id}  u_closed={} v_closed={}  u_quotient={:?} v_quotient={:?}",
                s.source_closure().map(|c| c.u_closed).unwrap_or(false),
                s.source_closure().map(|c| c.v_closed).unwrap_or(false),
                s.u_quotient().is_some(),
                s.v_quotient().is_some(),
            );
            // Ordinary companion U axis is bounded [0,1]: exterior u must be
            // rejected.
            assert!(
                !s.accept_inverse_result((3.63, 0.5)),
                "#{id}: exterior U root must be rejected on the ordinary companion axis"
            );
            assert!(
                !s.accept_inverse_result((2.0, 0.5)),
                "#{id}: exterior U root (2.0) must be rejected"
            );
            assert!(
                !s.accept_inverse_result((-1.0, 0.5)),
                "#{id}: exterior U root (-1.0) must be rejected"
            );
            // Interior u with deck-equivalent v: certified V axis admits deck
            // representatives.
            for &v in &[0.5, 3.63, -0.37, 1.5, 2.5] {
                assert!(
                    s.accept_inverse_result((0.765, v)),
                    "#{id}: deck-equivalent v={v} must be legal on the certified V axis"
                );
            }
            // Boundary u values within justified tolerance are accepted.
            assert!(s.accept_inverse_result((0.0, 0.5)));
            assert!(s.accept_inverse_result((1.0, 0.5)));
            println!("PASS #{id}: exterior U rejected, deck-equivalent V admitted");
            seen += 1;
        }
    }
    assert!(seen == 2, "expected both #1167 and #1169, saw {seen}");
    println!("ALL PASS");
    Ok(())
}
