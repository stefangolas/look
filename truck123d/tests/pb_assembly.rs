//! PB-006 required Rust suite (three tests) over the multi-solid assembly
//! emitter (`truck123d::assembly_emit`, R2 amendment).
//!
//! The emission is client-side and structural: N solids (as marshaled opaque
//! handles) + an intended-contact list → a STEP assembly whose nodes carry the
//! recorded contact intents, typed INTENT, until BIE/CTE land certified
//! contact resolution. All three tests are pure Rust — no embedded interpreter,
//! no geometry, no `showcases/` linkage. The teapot fixture builds its three
//! parts through the bridge tables (`showcases/tables/teapot.json` → facade
//! primitive/extrude sessions); shape fidelity is not under test here.

use std::path::Path;

use truck123d::{
    AssemblyContactIntent, AssemblyPart, AssemblyReport, EvidenceRowKind, FacadeOp, FacadeTable,
    SolidHandle, TeapotTable, emit_step_assembly, read_step_assembly, run_facade,
};

/// A part handle with its id equal to its name (the fixture convention).
fn handle(name: &str, kind: &str) -> AssemblyPart {
    AssemblyPart {
        name: name.to_string(),
        solid: SolidHandle {
            id: name.to_string(),
            kind: kind.to_string(),
        },
    }
}

/// One recorded (never resolved) intended contact between two parts.
fn intent(a: &str, b: &str, note: &str) -> AssemblyContactIntent {
    AssemblyContactIntent {
        node_a: a.to_string(),
        node_b: b.to_string(),
        kind: EvidenceRowKind::Intent,
        note: note.to_string(),
    }
}

/// The absolute path of the showcase tables directory.
fn tables_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../showcases/tables")
}

/// Round-trips `parts` + `intents` through emission → read-back and returns
/// the read-back report (the round-trip node records).
fn round_trip(parts: &[AssemblyPart], intents: &[AssemblyContactIntent]) -> AssemblyReport {
    let emitted = emit_step_assembly(parts, intents).expect("fixture is in envelope");
    read_step_assembly(&emitted.step).expect("own emission must read back")
}

#[test]
fn assembly_emits_n_solids_step() {
    // Three solids → a STEP assembly whose read-back reports three nodes.
    let parts = [
        handle("solid_a", "box"),
        handle("solid_b", "cylinder"),
        handle("solid_c", "extrude"),
    ];
    let emitted = emit_step_assembly(&parts, &[]).expect("three solids emit");

    // The STEP text is a real Part 21 assembly with one PRODUCT per node.
    assert!(emitted.step.starts_with("ISO-10303-21;\n"));
    assert!(emitted.step.contains("ENDSEC;\nEND-ISO-10303-21;"));
    assert_eq!(product_count(&emitted.step), 3);

    // Round-trip read-back asserts the node count: exactly one node per solid,
    // in insertion order, with no recorded intents.
    let read = read_step_assembly(&emitted.step).expect("own emission reads back");
    assert_eq!(read.nodes.len(), 3);
    let names: Vec<&str> = read.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["solid_a", "solid_b", "solid_c"]);
    for node in &read.nodes {
        assert!(node.intents.is_empty(), "no intents were recorded");
    }
    // The full round trip is lossless: emission nodes equal read-back nodes.
    assert_eq!(read.nodes, emitted.nodes);
}

#[test]
fn contact_intents_recorded_until_bie() {
    // Each intended contact appears as an evidence row on the node(s) it
    // names, typed INTENT — never as a certified contact. Recording is data:
    // nothing resolves, no contact work happens, and the row kind vocabulary
    // has no certified arm.
    let parts = [
        handle("body", "cylinder"),
        handle("spout", "extrude"),
        handle("handle", "box"),
    ];
    let intents = [
        intent("body", "spout", "body-spout junction (recorded seam)"),
        intent("handle", "body", "body-handle junction (recorded seam)"),
    ];
    let read = round_trip(&parts, &intents);

    // The three parts come back in insertion order.
    let names: Vec<&str> = read.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["body", "spout", "handle"]);

    // Every recorded row on every node is typed INTENT — a certified contact
    // cannot even be expressed (EvidenceRowKind has only the Intent arm).
    let total_rows: usize = read.nodes.iter().map(|node| node.intents.len()).sum();
    assert_eq!(total_rows, intents.len() * 2);
    for node in &read.nodes {
        for row in &node.intents {
            assert_eq!(row.kind, EvidenceRowKind::Intent);
        }
    }

    // Each intended contact is recorded on BOTH participating nodes, so the
    // node census is: body carries the two junction intents, spout and handle
    // each carry their junction with the body.
    let by_name = |name: &str| -> &Vec<AssemblyContactIntent> {
        let index = read
            .nodes
            .iter()
            .position(|node| node.name == name)
            .expect("part must be present");
        &read.nodes[index].intents
    };
    assert_eq!(by_name("body"), &intents);
    assert_eq!(by_name("spout"), &[intents[0].clone()]);
    assert_eq!(by_name("handle"), &[intents[1].clone()]);

    // Round-trip fidelity: the recorded row (endpoints, kind, note) survives
    // the emission → STEP → read-back losslessly, including a quoted note.
    let spout_row = &by_name("spout")[0];
    assert_eq!(spout_row.node_a, "body");
    assert_eq!(spout_row.node_b, "spout");
    assert_eq!(spout_row.note, "body-spout junction (recorded seam)");
}

#[test]
fn teapot_body_spout_handle_assembles() {
    // The teapot decomposes into body + spout + handle. This fixture builds
    // the three parts through the bridge tables: parse the real teapot table,
    // express each part as a simple prismatic facade session (primitives +
    // extrude), and certify each session in-envelope. Shape fidelity is not
    // under test; the assembly census and the recorded intents are.
    let table_text = std::fs::read_to_string(tables_dir().join("teapot.json"))
        .expect("the real teapot table must be present");
    let table: TeapotTable =
        serde_json::from_str(&table_text).expect("teapot table must validate against schema v1");

    // Body: the revolved silhouette's belly radius and rim height as a solid
    // primitive (cylinder). Spout: a polygon profile of the base ring radius,
    // extruded by the spout's rise. Handle: a prism over the tube radius.
    let belly = table.body_stations[table.body_stations.len() / 2][1];
    let rim = table.body_stations[table.body_stations.len() - 1][0];
    let body_build = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Cylinder {
                radius: belly,
                height: rim,
            },
            FacadeOp::Pop,
        ],
    };
    let r0 = table.spout_r0;
    let rise = table.spout_points[table.spout_points.len() - 1][2] - table.spout_points[0][2];
    let spout_build = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::PushSketch,
            FacadeOp::Polygon {
                points: vec![[-r0, -r0], [r0, -r0], [r0, r0], [-r0, r0]],
            },
            FacadeOp::Pop,
            FacadeOp::Extrude { amount: rise },
            FacadeOp::Pop,
        ],
    };
    let r = table.handle_radius;
    let handle_build = FacadeTable {
        ops: vec![
            FacadeOp::PushPart,
            FacadeOp::Box {
                length: 2.0 * r,
                width: 2.0 * r,
                height: 4.0 * r,
            },
            FacadeOp::Pop,
        ],
    };

    // All three parts build as in-envelope (prismatic) bridge sessions: STEP
    // export of the parts is inside the TR-NRB-001 envelope.
    for build in [&body_build, &spout_build, &handle_build] {
        let report = run_facade(build).expect("each teapot part is in envelope");
        assert!(
            !report.constructive,
            "prismatic teapot parts are non-constructive"
        );
    }

    // Assemble: body + spout + handle as three nodes carrying the recorded
    // junction intents (recorded, never resolved — BIE lands later).
    let parts = [
        handle("body", "cylinder"),
        handle("spout", "extrude"),
        handle("handle", "box"),
    ];
    let intents = [
        intent("body", "spout", "body-spout junction (recorded until BIE)"),
        intent(
            "body",
            "handle",
            "body-handle junction (recorded until BIE)",
        ),
    ];
    let read = round_trip(&parts, &intents);

    // The report lists the three parts and their intents.
    assert_eq!(read.nodes.len(), 3);
    let names: Vec<&str> = read.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["body", "spout", "handle"]);
    let by_name = |name: &str| -> &Vec<AssemblyContactIntent> {
        let index = read
            .nodes
            .iter()
            .position(|node| node.name == name)
            .expect("part must be present");
        &read.nodes[index].intents
    };
    assert_eq!(by_name("body"), &intents);
    assert_eq!(by_name("spout"), &[intents[0].clone()]);
    assert_eq!(by_name("handle"), &[intents[1].clone()]);
    for node in &read.nodes {
        for row in &node.intents {
            assert_eq!(row.kind, EvidenceRowKind::Intent);
        }
    }
}

/// Counts the `PRODUCT` records in an emitted STEP text (one per assembly
/// node). The test-side counterpart of the read-back node census.
fn product_count(step: &str) -> usize {
    step.lines()
        .filter(|line| line.contains("= PRODUCT("))
        .count()
}
