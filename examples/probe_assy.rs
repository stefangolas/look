use truck_assembly::assy::*;
use truck_meshalgo::prelude::*;
use truck_stepio::common::PartAttrs;
use truck_stepio::r#in::Table;
use truck_stepio::r#in::convert::*;

fn main() {
    let path = std::env::args().nth(1).expect("path");
    let text = std::fs::read_to_string(&path).unwrap();
    let table = Table::from_step(&text).unwrap();
    let assy = table.step_assy().unwrap();
    let mapped = assy.map(
        |node: &NodeEntity<Vec<ProductShape>, PartAttrs>| NodeEntity {
            shape: &node.shape,
            attrs: node.attrs.clone(),
        },
        |edge: &EdgeEntity<NodeMatrix, PartAttrs>| EdgeEntity {
            matrix: Matrix4::try_from(&edge.matrix).unwrap(),
            attrs: edge.attrs.clone(),
        },
    );

    let mut occurrence_targets =
        std::collections::HashMap::<truck_assembly::dag::NodeIndex, usize>::new();
    let mut total_occurrences = 0usize;
    for top in mapped.top_nodes() {
        for path in mapped.paths_iter(top.index()) {
            if path.edges().is_empty() {
                continue;
            }
            total_occurrences += 1;
            *occurrence_targets
                .entry(path.terminal_node().index())
                .or_insert(0) += 1;
        }
    }

    let mut geometry_nodes = 0usize;
    let mut matrix_only = 0usize;
    for (i, node) in mapped.all_nodes().enumerate() {
        let has_geometry = node
            .shape()
            .iter()
            .any(|s| matches!(s, ProductShape::Solid(..) | ProductShape::Shells(..)));
        let occurrences = occurrence_targets.get(&node.index()).copied().unwrap_or(0);
        if has_geometry {
            geometry_nodes += 1;
        } else {
            matrix_only += 1;
        }
        println!(
            "node#{i:02} occurrences={occurrences:3} geometry={has_geometry:5} name='{}'",
            node.entity().attrs.name
        );
    }
    println!(
        "TOTAL occurrences={total_occurrences} geometry_nodes={geometry_nodes} matrix_only={matrix_only}"
    );
}
