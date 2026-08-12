//! Probe: tally `source_geometric_uncertainty` across every shell of every
//! model in the corpus, to quantify how many shells carry a declared
//! uncertainty that is far below the fixed numerical tolerance (1e-6) and
//! would therefore fail R01's vertex-incidence checks.

use std::collections::HashMap;
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

fn bucket(v: Option<f64>) -> String {
    match v {
        None => "none".to_string(),
        Some(x) if x < 1.0e-12 => format!("tiny(<1e-12): {x:.1e}"),
        Some(x) if x < 1.0e-6 => format!("small(<1e-6): {x:.1e}"),
        Some(x) if x <= 1.0e-3 => format!("normal(1e-6..1e-3): {x:.1e}"),
        Some(x) => format!("loose(>1e-3): {x:.1e}"),
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: source_tol_tally MODEL.step [MORE...]");
        return Ok(());
    }
    let mut per_model: Vec<(String, usize, HashMap<String, usize>)> = Vec::new();
    for path in &args {
        let table = load(path)?;
        let mut tally: HashMap<String, usize> = HashMap::new();
        let mut shells = 0usize;
        for (&shell_id, shell) in table.shell.iter() {
            let (cshell, _) = table
                .to_compressed_shell_with_losses(shell_id, shell)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            shells += 1;
            *tally
                .entry(bucket(cshell.source_geometric_uncertainty))
                .or_default() += 1;
        }
        per_model.push((path.clone(), shells, tally));
    }
    for (path, shells, tally) in &per_model {
        let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
        println!("== {name}  ({shells} shells) ==");
        let mut rows: Vec<_> = tally.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        for (k, n) in rows {
            println!("  {n:>6}  {k}");
        }
    }
    Ok(())
}
