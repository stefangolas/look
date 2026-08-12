use truck_meshalgo::prelude::*;
use truck_stepio::r#in::Table;
fn load(path: &str) -> Result<Table, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes)
        .map(std::borrow::Cow::Borrowed)
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
    let mut exchange = match look::step::part21::parse(&text) {
        Ok(e) => e,
        Err(_) => ruststep::parser::parse(&text).map_err(|e| e.to_string())?,
    };
    let section = exchange.data.swap_remove(0);
    Ok(Table::from_owned_data_section(section))
}
fn main() {
    for p in std::env::args().skip(1) {
        let t = match load(&p) {
            Ok(t) => t,
            Err(e) => {
                println!("{p}: load error {e}");
                continue;
            }
        };
        let mut bb = BoundingBox::<Point3>::new();
        for (&sid, sh) in t.shell.iter() {
            match t.to_compressed_shell_with_losses(sid, sh) {
                Ok((cs, _)) => {
                    for v in &cs.vertices {
                        bb.push(*v);
                    }
                    for e in &cs.edges {
                        let (a, b) = e.curve.range_tuple();
                        for i in 0..=4 {
                            bb.push(e.curve.subs(a + (b - a) * f64::from(i) / 4.0));
                        }
                    }
                }
                Err(e) => {
                    println!("{p}: shell conv error {e}");
                }
            }
        }
        let name = p.rsplit(['\\', '/']).next().unwrap_or(&p).to_string();
        println!(
            "{name}: diameter={:.4} tol={:.6}",
            bb.diameter(),
            bb.diameter() * 0.001
        );
    }
}
