//! PB-009 required Rust suite (three tests) over the GLB (glTF 2.0 binary)
//! assembly emission (`truck123d::glb_emit`) and the landed-graph occurrence
//! naming it carries (PB-006's assembly graph, walked by
//! `truck123d::assembly_occurrence_labels`).
//!
//! The emission is client-side and mesh-based: per-node mesh payloads
//! (positions `f32`, indices `u32`, one sRGB color, one name, one parent
//! index, and a local transform) become a GLB whose nodes carry the assembly
//! names, the hierarchy transforms, and the per-solid linear base colors. All
//! three tests are pure Rust — no embedded interpreter, no geometry, no
//! `showcases/` linkage. The parity test reads the vendored F1 occurrence
//! table (`f1.step.js`, a read-only corpus fixture) at compile time and pins
//! the naming function to it exactly.
//!
//! H-1 applies to this new file: nothing here unwraps, expects, or panics —
//! helpers return `Result<_, String>` and tests propagate with `?`.

use truck123d::{
    AssemblyPart, GlbMesh, GlbNodePayload, SolidHandle, SrgbColor, assembly_occurrence_labels,
    emit_glb,
};

/// The vendored F1 OCCURRENCES table (`corpus/ttc/trees/f1/STEP/f1.step.js`),
/// read at compile time — read-only corpus fixture, never modified.
const F1_STEP_JS: &str = include_str!("../../corpus/ttc/trees/f1/STEP/f1.step.js");

/// The 28 top-level part names the vendored table pins, in insertion order.
/// The sidecar comment says the order is frozen by `src/f1.py`'s `assemble()`.
const F1_OCCURRENCE_NAMES: [&str; 28] = [
    "front_wing",
    "nose",
    "monocoque",
    "halo",
    "cockpit",
    "sidepod_left",
    "sidepod_right",
    "engine_cover",
    "airbox",
    "floor",
    "diffuser",
    "cooling",
    "power_unit",
    "drivetrain",
    "rear_wing",
    "drs_flap",
    "drs_actuator",
    "beam_wing",
    "suspension_front",
    "suspension_rear",
    "corner_fl",
    "corner_fr",
    "track_rod_left",
    "track_rod_right",
    "corner_rl",
    "corner_rr",
    "steering_rack",
    "details",
];

#[test]
fn glb_binary_structure_is_valid() -> Result<(), String> {
    let payload = triangle_payload(
        "triangle",
        SrgbColor::new(0.9, 0.2, 0.1, 1.0),
        None,
        identity_matrix(),
    );
    let glb = emit(std::slice::from_ref(&payload))?;
    let again = emit(std::slice::from_ref(&payload))?;
    assert_eq!(
        glb, again,
        "same input table must produce a byte-identical GLB"
    );

    // The 12-byte GLB header: magic `glTF`, format version 2, total length.
    assert_eq!(&glb[0..4], b"glTF", "GLB magic must be `glTF`");
    assert_eq!(read_u32_at(&glb, 4)?, 2, "GLB version must be 2");
    let total_length = read_u32_at(&glb, 8)? as usize;
    assert_eq!(
        total_length,
        glb.len(),
        "header total length must equal the file length"
    );

    // The JSON chunk header follows the container header at offset 12.
    let json_length = read_u32_at(&glb, 12)? as usize;
    let json_type = read_u32_at(&glb, 16)?;
    assert_eq!(json_type, 0x4e4f_534a, "JSON chunk type must be 0x4E4F534A");
    assert_eq!(
        json_length % 4,
        0,
        "JSON chunk length must be 4-byte aligned"
    );
    let json_data = 20usize;

    // The BIN chunk follows the (padded) JSON chunk.
    let bin_header = json_data
        .checked_add(json_length)
        .ok_or("json chunk overflows")?;
    let bin_length = read_u32_at(&glb, bin_header)? as usize;
    let bin_type = read_u32_at(&glb, bin_header + 4)?;
    assert_eq!(bin_type, 0x004e_4942, "BIN chunk type must be 0x004E4942");
    assert_eq!(bin_length % 4, 0, "BIN chunk length must be 4-byte aligned");

    // The chunk lengths account for the whole file: 12 header + two 8-byte
    // chunk headers + both chunk payloads.
    let bin_data = bin_header.checked_add(8).ok_or("bin chunk overflows")?;
    assert_eq!(
        glb.len(),
        bin_data
            .checked_add(bin_length)
            .ok_or("bin data overflows")?,
        "file length must be header + both chunks"
    );

    // The JSON chunk parses as the glTF document, and the single buffer's
    // byteLength equals the padded BIN chunk data length.
    let document: serde_json::Value =
        serde_json::from_slice(&glb[json_data..bin_header]).map_err(|e| e.to_string())?;
    let buffer_length = document
        .get("buffers")
        .and_then(|value| value.as_array())
        .and_then(|buffers| buffers.first())
        .and_then(|buffer| buffer.get("byteLength"))
        .and_then(|value| value.as_u64())
        .ok_or("GLB document must carry a single buffer with byteLength")?;
    assert_eq!(
        buffer_length as usize, bin_length,
        "buffer byteLength must equal the BIN chunk length"
    );
    Ok(())
}

#[test]
fn glb_round_trips_names_hierarchy_colors() -> Result<(), String> {
    // A 3-node fixture: one root with two children, each with a distinct local
    // transform and a distinct sRGB color.
    let root = triangle_payload(
        "root",
        SrgbColor::new(0.9, 0.2, 0.1, 1.0),
        None,
        identity_matrix(),
    );
    let child_a = triangle_payload(
        "child_a",
        SrgbColor::new(0.5, 0.5, 0.5, 1.0),
        Some(0),
        translation_matrix(200.0, 0.0, 0.0),
    );
    let child_b = triangle_payload(
        "child_b",
        SrgbColor::new(0.04, 0.9, 0.8, 0.5),
        Some(0),
        translation_matrix(0.0, -150.0, 0.0),
    );
    let glb = emit(&[root, child_a, child_b])?;
    let (document, _bin) = parse_glb(&glb)?;

    // Scene: the single scene lists the single root (node 0).
    let scenes = document
        .get("scenes")
        .and_then(|v| v.as_array())
        .ok_or("scenes missing")?;
    assert_eq!(scenes.len(), 1, "one default scene");
    let scene_nodes = u64_array(scenes[0].get("nodes")).ok_or("scene nodes missing")?;
    assert_eq!(scene_nodes, vec![0], "the root node is the scene root");

    // Node names and hierarchy: parent/child indices ride as the parent's
    // `children` array, in insertion order.
    let nodes = document
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or("nodes missing")?;
    assert_eq!(nodes.len(), 3, "one GLB node per payload");
    let node_names = nodes
        .iter()
        .map(|node| node.get("name").and_then(|v| v.as_str()))
        .collect::<Option<Vec<&str>>>()
        .ok_or("every node carries a name")?;
    assert_eq!(node_names, ["root", "child_a", "child_b"]);
    assert_eq!(
        u64_array(nodes[0].get("children")).ok_or("root children missing")?,
        vec![1, 2],
        "root lists both children"
    );
    assert!(nodes[1].get("children").is_none(), "child_a is a leaf");
    assert!(nodes[2].get("children").is_none(), "child_b is a leaf");

    // Matrices: each node carries its local column-major 4x4 transform.
    assert_matrix(&nodes[0], &identity_matrix())?;
    assert_matrix(&nodes[1], &translation_matrix(200.0, 0.0, 0.0))?;
    assert_matrix(&nodes[2], &translation_matrix(0.0, -150.0, 0.0))?;

    // Meshes and materials are derived in node (insertion) order: mesh i is
    // node i's mesh and references material i, whose baseColorFactor is the
    // node color converted sRGB → linear (alpha unchanged).
    let meshes = document
        .get("meshes")
        .and_then(|v| v.as_array())
        .ok_or("meshes missing")?;
    let materials = document
        .get("materials")
        .and_then(|v| v.as_array())
        .ok_or("materials missing")?;
    assert_eq!(meshes.len(), 3, "one mesh per solid payload");
    assert_eq!(materials.len(), 3, "one material per solid payload");
    for (index, node) in nodes.iter().enumerate() {
        assert_eq!(
            node.get("mesh").and_then(|v| v.as_u64()),
            Some(index as u64),
            "node {index} references mesh {index}"
        );
        let material = meshes[index]
            .get("primitives")
            .and_then(|v| v.as_array())
            .and_then(|primitives| primitives.first())
            .and_then(|primitive| primitive.get("material"))
            .and_then(|v| v.as_u64());
        assert_eq!(
            material,
            Some(index as u64),
            "mesh {index} references material {index}"
        );
    }

    assert_base_color(materials, 0, &[0.9, 0.2, 0.1, 1.0])?;
    assert_base_color(materials, 1, &[0.5, 0.5, 0.5, 1.0])?;
    assert_base_color(materials, 2, &[0.04, 0.9, 0.8, 0.5])?;
    Ok(())
}

#[test]
fn f1_occurrence_naming_parity() -> Result<(), String> {
    // The vendored OCCURRENCES block: parse the pinned (name, "#o1.N") pairs.
    let pinned = vendored_occurrences(F1_STEP_JS);
    assert_eq!(
        pinned.len(),
        F1_OCCURRENCE_NAMES.len(),
        "the vendored table pins 28 top-level occurrences"
    );
    let pinned_names: Vec<&str> = pinned.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        pinned_names, F1_OCCURRENCE_NAMES,
        "the vendored insertion order is the fixture order"
    );
    let pinned_labels: Vec<String> = pinned.iter().map(|(_, label)| label.clone()).collect();
    assert_eq!(
        pinned_labels,
        (1..=28)
            .map(|n| format!("#o1.{n}"))
            .collect::<Vec<String>>(),
        "labels are #o1.1..#o1.28 in insertion order"
    );

    // The landed assembly graph (28 parts in the vendored insertion order),
    // walked by assembly_occurrence_labels under the root occurrence `o1`,
    // must reproduce the pinned labels exactly.
    let parts: Vec<AssemblyPart> = pinned_names
        .iter()
        .map(|name| AssemblyPart {
            name: name.to_string(),
            solid: SolidHandle {
                id: name.to_string(),
                kind: "extrude".to_string(),
            },
        })
        .collect();
    let labels = assembly_occurrence_labels("o1", &parts);
    assert_eq!(
        labels, pinned_labels,
        "the naming function reproduces the vendored occurrence table"
    );

    // End to end: the GLB emission names its nodes by those occurrence labels,
    // in insertion order, so the render payload carries the frozen labels.
    let payloads: Vec<GlbNodePayload> = labels
        .iter()
        .map(|label| {
            triangle_payload(
                label,
                SrgbColor::new(0.5, 0.5, 0.5, 1.0),
                None,
                identity_matrix(),
            )
        })
        .collect();
    let glb = emit(&payloads)?;
    let (document, _bin) = parse_glb(&glb)?;
    let nodes = document
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or("nodes missing")?;
    let node_names = nodes
        .iter()
        .map(|node| {
            node.get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect::<Option<Vec<String>>>()
        .ok_or("every node carries a name")?;
    assert_eq!(
        node_names, labels,
        "the GLB node names are the #o1.N occurrence labels in order"
    );
    Ok(())
}

/// One minimal triangle mesh payload (the fixture geometry of the suite).
fn triangle_payload(
    name: &str,
    color: SrgbColor,
    parent: Option<usize>,
    matrix: [f32; 16],
) -> GlbNodePayload {
    GlbNodePayload {
        name: name.to_string(),
        color,
        mesh: Some(GlbMesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            indices: vec![0, 1, 2],
        }),
        parent,
        matrix,
    }
}

/// Emits a payload table as a GLB, mapping the typed refusal to a test error
/// (H-1: helpers propagate, never unwrap).
fn emit(payloads: &[GlbNodePayload]) -> Result<Vec<u8>, String> {
    emit_glb(payloads).map_err(|refusal| format!("emission refused: {refusal:?}"))
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// A column-major translation matrix.
fn translation_matrix(x: f32, y: f32, z: f32) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
    ]
}

/// Asserts that the GLB node's `matrix` equals `expected` (each component
/// within a tiny float tolerance; the payload f32 → JSON f64 widening is
/// lossless).
fn assert_matrix(node: &serde_json::Value, expected: &[f32; 16]) -> Result<(), String> {
    let actual = node
        .get("matrix")
        .and_then(|v| v.as_array())
        .ok_or("node matrix missing")?;
    assert_eq!(actual.len(), 16, "a node matrix has 16 components");
    for (index, component) in expected.iter().enumerate() {
        let value = actual[index]
            .as_f64()
            .ok_or("matrix component not a number")?;
        assert!(
            (value - f64::from(*component)).abs() < 1e-9,
            "matrix component {index} differs: {value} vs {component}"
        );
    }
    Ok(())
}

/// Asserts that material `index`'s `baseColorFactor` is the sRGB→linear
/// conversion of `color` (a `[r, g, b, a]` sRGB record in 0..=1). Alpha is
/// not a color: it passes through unconverted.
fn assert_base_color(
    materials: &[serde_json::Value],
    index: usize,
    color: &[f64; 4],
) -> Result<(), String> {
    let material = materials.get(index).ok_or("material index out of range")?;
    let factor = material
        .get("pbrMetallicRoughness")
        .and_then(|v| v.get("baseColorFactor"))
        .and_then(|v| v.as_array())
        .ok_or("material baseColorFactor missing")?;
    assert_eq!(factor.len(), 4, "baseColorFactor is RGBA");
    let expected = [
        srgb_to_linear(color[0]),
        srgb_to_linear(color[1]),
        srgb_to_linear(color[2]),
        color[3],
    ];
    for (component, want) in factor.iter().zip(expected.iter()) {
        let got = component
            .as_f64()
            .ok_or("baseColorFactor component not a number")?;
        // 1e-6 absolute on 0..=1 values: wide enough to absorb the f32 -> f64
        // widening of the sRGB input (0.9f32 = 0.899999976...), tight enough
        // that a missing sRGB -> linear conversion fails by ~0.3.
        assert!(
            (got - want).abs() < 1e-6,
            "baseColorFactor component {got} differs from {want}"
        );
    }
    assert_eq!(
        material
            .get("pbrMetallicRoughness")
            .and_then(|v| v.get("metallicFactor"))
            .and_then(|v| v.as_f64()),
        Some(1.0),
        "fixed metallic default"
    );
    assert_eq!(
        material
            .get("pbrMetallicRoughness")
            .and_then(|v| v.get("roughnessFactor"))
            .and_then(|v| v.as_f64()),
        Some(0.5),
        "fixed roughness default"
    );
    Ok(())
}

/// The standard sRGB → linear piecewise transfer (mirror of the emission).
fn srgb_to_linear(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Parses a GLB into its JSON document and the raw BIN chunk data.
fn parse_glb(glb: &[u8]) -> Result<(serde_json::Value, &[u8]), String> {
    let json_length = read_u32_at(glb, 12)? as usize;
    let json_data = 20usize;
    let bin_header = json_data
        .checked_add(json_length)
        .ok_or("json chunk overflows")?;
    let bin_length = read_u32_at(glb, bin_header)? as usize;
    let bin_data = bin_header.checked_add(8).ok_or("bin chunk overflows")?;
    let document =
        serde_json::from_slice(&glb[json_data..bin_header]).map_err(|e| e.to_string())?;
    let bin = glb
        .get(
            bin_data
                ..bin_data
                    .checked_add(bin_length)
                    .ok_or("bin data overflows")?,
        )
        .ok_or("bin chunk out of range")?;
    Ok((document, bin))
}

/// Reads a little-endian `u32` at `offset`, refusing when out of range.
fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset
        .checked_add(4)
        .filter(|end| *end <= bytes.len())
        .ok_or("read out of range")?;
    let slice = bytes.get(offset..end).ok_or("read out of range")?;
    let fixed: [u8; 4] = slice.try_into().map_err(|_| "read out of range")?;
    Ok(u32::from_le_bytes(fixed))
}

/// The contents of a JSON array of non-negative integers as `u64`s, or `None`
/// when the value is missing or holds a non-number element.
fn u64_array(value: Option<&serde_json::Value>) -> Option<Vec<u64>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_u64())
        .collect()
}

/// Parses the vendored F1 OCCURRENCES block: every `name: "#o1.N",` row, in
/// file (insertion) order, as `(part name, occurrence label)`.
fn vendored_occurrences(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(colon) = line.find(':') else {
            continue;
        };
        let Some(after) = line.get(colon + 1..) else {
            continue;
        };
        let Some(open) = after.find('"') else {
            continue;
        };
        let Some(rest) = after.get(open + 1..) else {
            continue;
        };
        let Some(close) = rest.find('"') else {
            continue;
        };
        let Some(label) = rest.get(..close) else {
            continue;
        };
        if !label.starts_with("#o1.") {
            continue;
        }
        let name = line.get(..colon).map(str::trim).unwrap_or("");
        if !name.is_empty() {
            out.push((name.to_string(), label.to_string()));
        }
    }
    out
}
