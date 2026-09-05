use std::path::PathBuf;

use showcases::cc_ports::LandedPorts;
use showcases::waterslide::{WaterslideTable, build};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let table_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("showcases/tables/waterslide.json"));
    let out_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("showcases/out/waterslide"));

    let table: WaterslideTable = if table_path.exists() {
        let raw = std::fs::read_to_string(&table_path).expect("read table");
        serde_json::from_str(&raw).expect("parse table")
    } else {
        WaterslideTable::default()
    };

    match build(&table, &out_dir, &LandedPorts) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).expect("report json"));
        }
        Err(e) => {
            eprintln!("waterslide build failed: {e}");
            std::process::exit(1);
        }
    }
}
