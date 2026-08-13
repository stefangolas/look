//! STEP colour join diagnostic.
//!
//! For a STEP file, report how the resolved face-appearance map (keyed by the
//! source `FACE_SURFACE` / `ADVANCED_FACE` entity id) joins against the
//! `FaceProvenance.definition_id` of every converted face. The join rate is
//! the number of styled source face ids that actually reach a tessellated
//! face. Curve and annotation styling is counted separately to show it never
//! becomes a face colour.
//!
//! Usage: `cargo run --release --example step_color_diag -- MODEL.step`

use std::collections::BTreeMap;
use std::time::Instant;

use look::step::appearance;
use truck_stepio::r#in::Table;
use truck_topology::compress::SourceEntityId;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: step_color_diag MODEL.step");
    let bytes = std::fs::read(&path).expect("read model");
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let started = Instant::now();
    let exchange = look::step::part21::parse(&text)
        .unwrap_or_else(|_| ruststep::parser::parse(&text).expect("parse step"));
    let table = Table::from_owned_data_section(exchange.data.into_iter().next().unwrap());
    eprintln!("table built in {:?}", started.elapsed());

    let (map, unresolved) = appearance::resolve_face_appearances(&table);
    eprintln!(
        "resolved {} styled face ids; {} unresolved chain(s)",
        map.len(),
        unresolved.len()
    );
    for u in unresolved.iter().take(12) {
        eprintln!("  unresolved #{} ({}) {}", u.entity_id, u.kind, u.reason);
    }

    // What the styled items name, by target kind.
    let mut styled_targets = BTreeMap::<&'static str, u64>::new();
    for styled in table.styled_item.values() {
        if let Some(target) = styled.item {
            *styled_targets
                .entry(if table.face_surface.contains_key(&target) {
                    "face"
                } else if table.manifold_solid_brep.contains_key(&target)
                    || table.shell_based_surface_model.contains_key(&target)
                    || table.shell.contains_key(&target)
                    || table.oriented_shell.contains_key(&target)
                {
                    "shape"
                } else {
                    "other"
                })
                .or_insert(0) += 1;
        }
    }
    eprintln!("styled targets: {styled_targets:?}");

    // Every converted face's definition_id, and whether the appearance map
    // matched it.
    let mut definition_ids: Vec<u64> = Vec::new();
    let mut matched = 0u64;
    let mut unmatched = 0u64;
    let mut faces_by_color: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (&shell_id, shell) in &table.shell {
        if let Ok(compressed) = table.to_compressed_shell(shell_id, shell) {
            for face in compressed.faces {
                if let Some(id) = face.provenance.definition_id.map(SourceEntityId::get) {
                    definition_ids.push(id);
                    if let Some(color) = map.get(&id) {
                        matched += 1;
                        faces_by_color
                            .entry(
                                ((color.color[0] * 1000.0) as u64) * 1_000_000
                                    + ((color.color[1] * 1000.0) as u64) * 1_000
                                    + ((color.color[2] * 1000.0) as u64),
                            )
                            .or_default()
                            .push(id);
                    } else {
                        unmatched += 1;
                    }
                }
            }
        }
    }

    eprintln!("converted faces: {}", definition_ids.len());
    eprintln!("  matched a style: {matched}");
    eprintln!("  unstyled:        {unmatched}");

    let styled_ids: std::collections::BTreeSet<u64> = map.keys().copied().collect();
    let definition_set: std::collections::BTreeSet<u64> = definition_ids.iter().copied().collect();
    let join = styled_ids.intersection(&definition_set).count();
    eprintln!(
        "join: {} of {} styled face ids appear as a FaceProvenance.definition_id ({:.1}%)",
        join,
        styled_ids.len(),
        100.0 * join as f64 / styled_ids.len().max(1) as f64
    );

    // A diagnostic table for the styled faces that actually joined.
    println!(
        "{:<10} {:<12} {:<24} {:<14}",
        "face_id", "definition_id", "effective_rgb", "matched"
    );
    for &id in styled_ids.iter().take(20) {
        let color = map.get(&id).expect("in styled set").color;
        let matched = definition_set.contains(&id);
        let rgb = format!("({:.3}, {:.3}, {:.3})", color[0], color[1], color[2]);
        println!(
            "{:<10} {:<12} {:<24} {:<14}",
            id,
            if matched { id.to_string() } else { "-".into() },
            rgb,
            if matched { "yes" } else { "no" }
        );
    }
    if styled_ids.len() > 20 {
        println!("  ... and {} more styled faces", styled_ids.len() - 20);
    }
    println!("unresolved chains: {}", unresolved.len());
}
