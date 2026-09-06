//! GLB (glTF 2.0 binary) assembly emission (PB-009-GLB-EMIT).
//!
//! The corpus's colored assemblies are unreachable from the bridge as STEP
//! (TR-NRB-001: swept/lofted carriers export mesh only), so this module emits
//! the same payload the STEP assembly emission carried into the corpus's render
//! stack — node names, hierarchy transforms, per-node base colors — as a GLB:
//! the corpus-side renderer reads GLB, and glTF carries the payload natively.
//!
//! Design (scope decisions 1–7 of the packet, not relitigated here):
//!
//! * **Mesh input, never solids.** Emission takes per-node mesh payloads
//!   (positions `f32`, triangle indices `u32`, one sRGB color, one name, one
//!   parent index + local transform) exactly as the harness door's STL path
//!   marshals them. No new kernel surface; zero new `Refusal` arms.
//! * **Hand-rolled writer over `serde_json`.** GLB is a 12-byte header, one
//!   JSON chunk (`0x4E4F534A`), one BIN chunk (`0x004E4942`), 4-byte
//!   alignment. The in-repo precedent is `examples/generate_ball_bearing.rs`
//!   (`serde_json` document → bytes); no glTF-writer crate is added.
//! * **Color payload is the client-layer record.** `shape.color` (an S2-class
//!   client data row) rides as `pbrMetallicRoughness.baseColorFactor`; the
//!   corpus sRGB color is converted to linear on emission (glTF base colors are
//!   linear). Metallic/roughness are fixed defaults (1.0 / 0.5) — renderer
//!   config, not file content.
//! * **Naming is corpus-fixed.** The GLB node names are the payload names; the
//!   corpus's frozen occurrence convention (`#o1.N`, from assembly insertion
//!   order) is reproduced by `assembly_emit::assembly_occurrence_labels`,
//!   which feeds the payload `name` fields.
//! * **Additive.** Everything here is a new file; no landed bridge surface
//!   changes.
//!
//! Determinism: node ordering is payload (insertion) order; meshes, materials,
//! bufferViews and accessors are derived in the same fixed order; no hash
//! ordering appears. Same input table → byte-identical GLB.
//!
//! H-1 applies: nothing here panics, unwraps, expects, or indexes (a malformed
//! payload — non-finite floats, ragged position/index lists, out-of-range
//! parents or indices — refuses with [`Refusal::Empty`] before any output is
//! produced).

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use truck_base::evidence::Refusal;

/// The GLB magic word, `glTF`, as a little-endian `u32`.
const GLB_MAGIC: u32 = 0x4654_6c67;
/// The glTF container format version this writer emits (glTF 2.0).
const GLB_VERSION: u32 = 2;
/// JSON chunk type (`"JSON"`).
const CHUNK_JSON: u32 = 0x4e4f_534a;
/// BIN chunk type (`"BIN\0"`).
const CHUNK_BIN: u32 = 0x004e_4942;
/// `componentType` for `f32` accessor components.
const COMPONENT_TYPE_FLOAT: u16 = 5126;
/// `componentType` for `u32` accessor components.
const COMPONENT_TYPE_UNSIGNED_INT: u16 = 5125;
/// `target` of a vertex-attribute bufferView.
const TARGET_ARRAY_BUFFER: u16 = 34962;
/// `target` of an element-index bufferView.
const TARGET_ELEMENT_ARRAY_BUFFER: u16 = 34963;
/// Primitive `mode`: triangles.
const MODE_TRIANGLES: u16 = 4;
/// Fixed metallic default (scope decision 4: renderer config is not file
/// content; only the client color record rides in the file).
const DEFAULT_METALLIC: f64 = 1.0;
/// Fixed roughness default.
const DEFAULT_ROUGHNESS: f64 = 0.5;
/// The sRGB → linear piecewise cutoff (the lower branch is `c / 12.92`).
const SRGB_CUTOFF: f64 = 0.04045;

/// The client-layer color record of one solid (corpus `shape.color`, an
/// S2-class data row): sRGB RGBA components in `0..=1`. The emission converts
/// the RGB channels to linear for the GLB `baseColorFactor`; alpha passes
/// through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SrgbColor {
    /// sRGB red, `0..=1`.
    pub r: f32,
    /// sRGB green, `0..=1`.
    pub g: f32,
    /// sRGB blue, `0..=1`.
    pub b: f32,
    /// Alpha (opacity), `0..=1`.
    pub a: f32,
}

impl SrgbColor {
    /// The sRGB RGBA color record of a solid.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// One mesh payload of a GLB node: positions and triangle indices, exactly as
/// the harness door's STL path marshals mesh data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlbMesh {
    /// Flat vertex positions: `xyz` triplets, `f32` (length is a multiple of 3).
    pub positions: Vec<f32>,
    /// Triangle indices into `positions` (length is a multiple of 3).
    pub indices: Vec<u32>,
}

/// One node payload of an assembly GLB emission: name, one sRGB color, the
/// mesh, and the assembly placement (parent index + local transform). Payload
/// order is insertion order — node `i` of the emitted GLB is payload `i`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlbNodePayload {
    /// The node (occurrence) name carried verbatim as the GLB node name.
    pub name: String,
    /// The solid's sRGB color record (linearized into the GLB material).
    pub color: SrgbColor,
    /// The node's mesh. `None` emits a geometry-less (group) node.
    pub mesh: Option<GlbMesh>,
    /// The index of this node's parent in the payload list. `None` makes the
    /// node a scene root. A parent must precede its child in insertion order.
    pub parent: Option<usize>,
    /// The node's local transform, as a column-major 4×4 matrix (`f32`).
    pub matrix: [f32; 16],
}

/// The validated placement layout of an emission.
struct GlbLayout {
    /// Per-node child indices (each child list in ascending node order).
    children: Vec<Vec<u32>>,
    /// The root node indices (payload insertion order).
    roots: Vec<u32>,
}

/// Emits `payloads` as a GLB (glTF 2.0 binary) container.
///
/// Each payload becomes one GLB node, in insertion order, named by its `name`,
/// carrying its local `matrix`; mesh nodes also carry a single-triangle-set
/// mesh whose material holds the linearized `baseColorFactor`. Root payloads
/// (no `parent`) are listed on the single default scene. Returns the GLB
/// bytes, byte-identical for the same input table. A payload list that is
/// empty, has no root, carries a non-finite float, a ragged position/index
/// list, an out-of-range parent or an out-of-range triangle index refuses
/// with [`Refusal::Empty`] (zero new `Refusal` arms).
pub fn emit_glb(payloads: &[GlbNodePayload]) -> Result<Vec<u8>, Refusal> {
    let layout = validate_and_layout(payloads)?;

    let mut binary: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<serde_json::Value> = Vec::new();
    let mut accessors: Vec<serde_json::Value> = Vec::new();
    let mut meshes: Vec<serde_json::Value> = Vec::new();
    let mut materials: Vec<serde_json::Value> = Vec::new();
    let mut node_values: Vec<serde_json::Value> = Vec::new();

    for (index, payload) in payloads.iter().enumerate() {
        let mut mesh_index = None;
        if let Some(mesh) = &payload.mesh {
            let position_accessor =
                append_geometry(&mut binary, &mut buffer_views, &mut accessors, mesh)?;
            let material_index = index_of(&materials)?;
            let this_mesh_index = index_of(&meshes)?;
            materials.push(json!({
                "pbrMetallicRoughness": {
                    "baseColorFactor": [
                        srgb_to_linear(f64::from(payload.color.r)),
                        srgb_to_linear(f64::from(payload.color.g)),
                        srgb_to_linear(f64::from(payload.color.b)),
                        f64::from(payload.color.a),
                    ],
                    "metallicFactor": DEFAULT_METALLIC,
                    "roughnessFactor": DEFAULT_ROUGHNESS,
                }
            }));
            meshes.push(json!({
                "primitives": [{
                    "attributes": { "POSITION": position_accessor.0 },
                    "indices": position_accessor.1,
                    "material": material_index,
                    "mode": MODE_TRIANGLES,
                }]
            }));
            mesh_index = Some(this_mesh_index);
        }

        let mut node_object = serde_json::Map::new();
        node_object.insert("name".to_string(), json!(payload.name.clone()));
        node_object.insert(
            "matrix".to_string(),
            json!(
                payload
                    .matrix
                    .iter()
                    .map(|c| f64::from(*c))
                    .collect::<Vec<f64>>()
            ),
        );
        if let Some(mesh) = mesh_index {
            node_object.insert("mesh".to_string(), json!(mesh));
        }
        let children = layout.children.get(index).cloned().unwrap_or_default();
        if !children.is_empty() {
            node_object.insert("children".to_string(), json!(children));
        }
        node_values.push(serde_json::Value::Object(node_object));
    }

    pad_to_multiple(&mut binary, 0);
    let bin_length = len_u32(&binary)?;
    let document = json!({
        "asset": {
            "version": "2.0",
            "generator": "truck123d bridge GLB emission (PB-009-GLB-EMIT)",
        },
        "scene": 0,
        "scenes": [{ "nodes": layout.roots }],
        "nodes": node_values,
        "meshes": meshes,
        "materials": materials,
        "bufferViews": buffer_views,
        "accessors": accessors,
        "buffers": [{ "byteLength": bin_length }],
    });

    assemble_glb(&document, &binary)
}

/// Validates the payload table and derives its placement layout (per-node
/// child lists and the root set). Every failure mode refuses with
/// [`Refusal::Empty`].
fn validate_and_layout(payloads: &[GlbNodePayload]) -> Result<GlbLayout, Refusal> {
    if payloads.is_empty() {
        return Err(Refusal::Empty);
    }

    let mut children: Vec<Vec<u32>> = vec![Vec::new(); payloads.len()];
    let mut roots = Vec::new();
    for (index, payload) in payloads.iter().enumerate() {
        if !is_finite_payload(payload) {
            return Err(Refusal::Empty);
        }
        match payload.parent {
            Some(parent) => {
                if parent >= index {
                    return Err(Refusal::Empty);
                }
                children
                    .get_mut(parent)
                    .ok_or(Refusal::Empty)?
                    .push(len_u32_checked(index)?);
            }
            None => roots.push(len_u32_checked(index)?),
        }
        if let Some(mesh) = &payload.mesh {
            if mesh.positions.is_empty()
                || mesh.indices.is_empty()
                || mesh.positions.len() % 3 != 0
                || mesh.indices.len() % 3 != 0
            {
                return Err(Refusal::Empty);
            }
            let vertex_count = mesh.positions.len() / 3;
            for &vertex in &mesh.indices {
                if (vertex as usize) >= vertex_count {
                    return Err(Refusal::Empty);
                }
            }
        }
    }
    if roots.is_empty() {
        return Err(Refusal::Empty);
    }
    Ok(GlbLayout { children, roots })
}

/// Whether every float in a payload is finite (a non-finite float could not be
/// written deterministically and would panic the `json!` serializer).
fn is_finite_payload(payload: &GlbNodePayload) -> bool {
    let color_finite = payload.color.r.is_finite()
        && payload.color.g.is_finite()
        && payload.color.b.is_finite()
        && payload.color.a.is_finite();
    let matrix_finite = payload.matrix.iter().all(|c| c.is_finite());
    if !color_finite || !matrix_finite {
        return false;
    }
    payload
        .mesh
        .as_ref()
        .is_none_or(|mesh| mesh.positions.iter().all(|c| c.is_finite()))
}

/// Appends one mesh's positions and indices to the binary buffer and pushes
/// its bufferViews/accessors, returning the `(POSITION accessor, indices
/// accessor)` indices.
fn append_geometry(
    binary: &mut Vec<u8>,
    buffer_views: &mut Vec<serde_json::Value>,
    accessors: &mut Vec<serde_json::Value>,
    mesh: &GlbMesh,
) -> Result<(u32, u32), Refusal> {
    let ((min_x, min_y, min_z), (max_x, max_y, max_z)) =
        component_bounds(&mesh.positions).ok_or(Refusal::Empty)?;
    let vertex_count = len_u32_checked(mesh.positions.len() / 3)?;
    let index_count = len_u32_checked(mesh.indices.len())?;

    let position_offset = len_u32(binary)?;
    for component in &mesh.positions {
        binary.extend_from_slice(&component.to_le_bytes());
    }
    let position_bytes = len_u32_checked(mesh.positions.len() * 4)?;

    let index_offset = len_u32(binary)?;
    for &vertex in &mesh.indices {
        binary.extend_from_slice(&vertex.to_le_bytes());
    }
    let index_bytes = len_u32_checked(mesh.indices.len() * 4)?;

    let position_view = index_of(buffer_views)?;
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": position_offset,
        "byteLength": position_bytes,
        "target": TARGET_ARRAY_BUFFER,
    }));
    let position_accessor = index_of(accessors)?;
    accessors.push(json!({
        "bufferView": position_view,
        "componentType": COMPONENT_TYPE_FLOAT,
        "count": vertex_count,
        "type": "VEC3",
        "min": [f64::from(min_x), f64::from(min_y), f64::from(min_z)],
        "max": [f64::from(max_x), f64::from(max_y), f64::from(max_z)],
    }));

    let index_view = index_of(buffer_views)?;
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": index_offset,
        "byteLength": index_bytes,
        "target": TARGET_ELEMENT_ARRAY_BUFFER,
    }));
    let index_accessor = index_of(accessors)?;
    accessors.push(json!({
        "bufferView": index_view,
        "componentType": COMPONENT_TYPE_UNSIGNED_INT,
        "count": index_count,
        "type": "SCALAR",
    }));

    Ok((position_accessor, index_accessor))
}

/// The three components of one axis direction (x, y, z).
type Component3 = (f32, f32, f32);

/// The per-axis bounds of a position list: `(min, max)`, each an `xyz` triple.
type AxisBounds = (Component3, Component3);

/// The per-axis bounds of `positions` as `(min, max)` triples. `None` only
/// when the list is not a whole number of `xyz` triplets (already refused by
/// validation; the iterator discipline keeps H-1 intact).
fn component_bounds(positions: &[f32]) -> Option<AxisBounds> {
    let mut values = positions.iter();
    let (Some(x), Some(y), Some(z)) = (values.next(), values.next(), values.next()) else {
        return None;
    };
    let mut min = (*x, *y, *z);
    let mut max = (*x, *y, *z);
    loop {
        match (values.next(), values.next(), values.next()) {
            (Some(x), Some(y), Some(z)) => {
                min.0 = min.0.min(*x);
                min.1 = min.1.min(*y);
                min.2 = min.2.min(*z);
                max.0 = max.0.max(*x);
                max.1 = max.1.max(*y);
                max.2 = max.2.max(*z);
            }
            (None, None, None) => break,
            _ => return None,
        }
    }
    Some((min, max))
}

/// Converts one sRGB channel in `0..=1` to linear (glTF base colors are
/// linear; corpus `shape.color` is sRGB). The standard piecewise sRGB
/// transfer: `c / 12.92` below the cutoff, the `1.055` power curve above.
fn srgb_to_linear(channel: f64) -> f64 {
    if channel <= SRGB_CUTOFF {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Assembles the GLB container: 12-byte header, padded JSON chunk, padded BIN
/// chunk. `binary` is already 4-byte padded; the JSON chunk is padded with
/// spaces (`0x20`) to the 4-byte alignment the format requires.
fn assemble_glb(document: &serde_json::Value, binary: &[u8]) -> Result<Vec<u8>, Refusal> {
    let mut json_bytes = serde_json::to_vec(document).map_err(|_| Refusal::Empty)?;
    pad_to_multiple(&mut json_bytes, b' ');

    let json_length = len_u32(&json_bytes)?;
    let bin_length = len_u32(binary)?;
    let payload_length = json_bytes
        .len()
        .checked_add(binary.len())
        .ok_or(Refusal::Empty)?;
    let total_length = len_u32_checked(
        12usize
            .checked_add(8)
            .and_then(|header| header.checked_add(payload_length))
            .and_then(|header| header.checked_add(8))
            .ok_or(Refusal::Empty)?,
    )?;

    let mut glb = Vec::with_capacity(total_length as usize);
    glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    glb.extend_from_slice(&GLB_VERSION.to_le_bytes());
    glb.extend_from_slice(&total_length.to_le_bytes());
    glb.extend_from_slice(&json_length.to_le_bytes());
    glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&bin_length.to_le_bytes());
    glb.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    glb.extend_from_slice(binary);
    Ok(glb)
}

/// Pads `bytes` up to a 4-byte multiple with `value` (the JSON chunk pads with
/// spaces, the BIN chunk with zeros).
fn pad_to_multiple(bytes: &mut Vec<u8>, value: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(value);
    }
}

/// The current length of a collection as `u32` (GLB lengths are `u32`).
fn len_u32(bytes: &[u8]) -> Result<u32, Refusal> {
    len_u32_checked(bytes.len())
}

/// A `usize` length/index as `u32`, refusing when it exceeds the format width.
fn len_u32_checked(value: usize) -> Result<u32, Refusal> {
    u32::try_from(value).map_err(|_| Refusal::Empty)
}

/// The current length of a JSON array as `u32` (a JSON index).
fn index_of(values: &[serde_json::Value]) -> Result<u32, Refusal> {
    len_u32_checked(values.len())
}
