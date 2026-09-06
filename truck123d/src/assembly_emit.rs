//! N-solid assembly emission (PB-006-ASSEMBLY, R2 amendment).
//!
//! The client-side emission entry the bridge lacked: N solids + an
//! intended-contact/evidence list → a STEP assembly whose nodes carry the
//! recorded contact intents, typed INTENT, until the BIE/CTE programs land
//! certified contact resolution.
//!
//! Design (scope decisions 1–6 of the R2 amendment, not relitigated here):
//!
//! * **Client-side and additive.** The emission lives in this crate, built on
//!   the landed `truck-assembly` DAG/assy surface (the single dependency edge
//!   the amendment authorizes). `vendor/truck/truck-assembly/**` is read-only;
//!   only the public graph API (`create_node`, `all_nodes`) is used.
//! * **Contact intents are RECORDED, never resolved.** An
//!   [`AssemblyContactIntent`] is data typed [`EvidenceRowKind::Intent`]; the
//!   bridge does no contact work. Each intended contact is recorded as an
//!   evidence row on *both* participating nodes (deterministic, symmetric).
//! * **Solid representation is the bridge's marshaled handle.** A solid never
//!   crosses the kernel boundary; [`SolidHandle`] is the opaque client handle
//!   (id + kind tag, mirroring the `PyTruckSolid` shape). `python.rs` is not
//!   widened.
//! * **The STEP assembly is structural.** Each solid becomes one assembly
//!   node = one `PRODUCT` in the emitted Part 21 text (AP214), in insertion
//!   order. Every node carries its recorded intents as a typed, serde node
//!   record in the product `description`, so the STEP *file itself* carries
//!   the recorded intents. No geometry is emitted (zero geometric content in
//!   this crate); a recorded intent is data, never a claim.
//!
//! Determinism: node order is insertion order (the `truck-assembly` `Vec`
//! order is never reordered) and no hash ordering appears in the output.
//! H-1 applies: nothing here panics, unwraps, expects, or indexes.

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
use truck_assembly::assy::{Assembly, NodeEntity};
use truck_base::evidence::Refusal;

/// The STEP schema name of an emitted assembly file (AP214, like the landed
/// `look` assembly fixture).
const STEP_SCHEMA: &str = "AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }";

/// The kind discriminant of a recorded contact-intent evidence row.
///
/// There is exactly one arm: an intended contact is recorded as `Intent`. A
/// *certified* contact cannot be expressed here — the BIE/CTE programs supply
/// certified resolution later, and "a recorded intent is data, never a claim".
/// (The serde name is `intent`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRowKind {
    /// The intended contact is recorded, not resolved.
    Intent,
}

/// One recorded, unresolved contact intent between two assembly nodes.
///
/// Recorded as an evidence row on each participating node, typed
/// [`EvidenceRowKind::Intent`] — never a certified contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyContactIntent {
    /// The first participating node (part name).
    pub node_a: String,
    /// The second participating node (part name).
    pub node_b: String,
    /// The record kind — always `Intent` (this packet records, never certifies).
    pub kind: EvidenceRowKind,
    /// Free-form note (e.g. the intended junction).
    pub note: String,
}

/// The bridge's marshaled solid handle: an opaque reference to a kernel-side
/// solid. Only identity rides here; no kernel type crosses the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidHandle {
    /// The opaque solid id (product `id` in the emitted STEP).
    pub id: String,
    /// The producing-op kind tag (mirrors the `PyTruckSolid.kind` tag).
    pub kind: String,
}

/// One input solid of an assembly emission: a name (the node/product name) and
/// its marshaled handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyPart {
    /// The part name — also the assembly node name and STEP product name.
    pub name: String,
    /// The marshaled handle of the solid.
    pub solid: SolidHandle,
}

/// One node of an emitted/read-back assembly: the part and the intended
/// contacts recorded as evidence rows on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyNode {
    /// The node (part) name.
    pub name: String,
    /// The marshaled handle of the node's solid.
    pub solid: SolidHandle,
    /// The intended contacts recorded as evidence rows on this node.
    pub intents: Vec<AssemblyContactIntent>,
}

/// The result of an emission or a read-back: the nodes in insertion order and
/// the STEP assembly text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblyReport {
    /// The nodes of the assembly, in insertion order (one per input solid).
    pub nodes: Vec<AssemblyNode>,
    /// The emitted STEP assembly text (echoed unchanged by a read-back).
    pub step: String,
}

/// Node attributes held on the `truck-assembly` graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AssemblyNodeAttrs {
    /// The node (part) name.
    name: String,
    /// The intended contacts recorded on this node, typed `Intent`.
    rows: Vec<AssemblyContactIntent>,
}

/// The per-node payload embedded in a STEP product `description`: the solid
/// kind tag plus the recorded intent rows. This is what makes the emitted STEP
/// file carry the recorded intents as typed data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeRecord {
    /// The solid kind tag of the node's handle.
    solid_kind: String,
    /// The recorded intent rows on this node.
    intents: Vec<AssemblyContactIntent>,
}

/// The graph type of an emission: node shape = the solid handle, node attrs =
/// name + recorded rows; no edges (contacts are recorded, not placements).
type AssemblyGraph = Assembly<SolidHandle, AssemblyNodeAttrs, (), ()>;

/// Emits `parts` plus the intended-contact list as a STEP assembly.
///
/// Each solid becomes one assembly node (one `PRODUCT`) in insertion order,
/// and every intended contact is recorded as an evidence row typed `Intent`
/// on each of its two participating nodes. Returns the [`AssemblyReport`]
/// (nodes + STEP text). Validation failures — an empty part list, a duplicate
/// part name, a non-empty id/name containing a control character, or an intent
/// naming an unknown node — refuse typed with [`Refusal::Empty`].
pub fn emit_step_assembly(
    parts: &[AssemblyPart],
    intents: &[AssemblyContactIntent],
) -> Result<AssemblyReport, Refusal> {
    if parts.is_empty() {
        return Err(Refusal::Empty);
    }

    let mut seen = std::collections::HashSet::new();
    for part in parts {
        if part.name.is_empty()
            || has_control_char(&part.name)
            || part.solid.id.is_empty()
            || has_control_char(&part.solid.id)
            || !seen.insert(part.name.as_str())
        {
            return Err(Refusal::Empty);
        }
    }

    for intent in intents {
        let names_a = parts.iter().any(|p| p.name == intent.node_a);
        let names_b = parts.iter().any(|p| p.name == intent.node_b);
        if !names_a || !names_b {
            return Err(Refusal::Empty);
        }
    }

    let mut graph = AssemblyGraph::new();
    for part in parts {
        graph.create_node(NodeEntity {
            shape: part.solid.clone(),
            attrs: AssemblyNodeAttrs {
                name: part.name.clone(),
                rows: rows_touching(&part.name, intents),
            },
        });
    }

    let mut nodes = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut index = 9usize;
    for node in graph.all_nodes() {
        let entity = node.entity();
        let handle = &entity.shape;
        let attrs = &entity.attrs;
        let record = NodeRecord {
            solid_kind: handle.kind.clone(),
            intents: attrs.rows.clone(),
        };
        let description =
            escape_step_string(&serde_json::to_string(&record).map_err(|_| Refusal::Empty)?);
        let id = escape_step_string(&handle.id);
        let name = escape_step_string(&attrs.name);

        let base = index;
        lines.push(format!(
            "#{base} = PRODUCT('{id}','{name}','{description}',(#2));"
        ));
        lines.push(format!(
            "#{} = PRODUCT_DEFINITION_FORMATION('','',#{base});",
            base + 1
        ));
        lines.push(format!(
            "#{} = PRODUCT_DEFINITION('design','',#{},#3);",
            base + 2,
            base + 1
        ));
        lines.push(format!(
            "#{} = PRODUCT_DEFINITION_SHAPE('','',#{});",
            base + 3,
            base + 2
        ));
        lines.push(format!(
            "#{} = SHAPE_REPRESENTATION('{name}',(#8),#4);",
            base + 4
        ));
        lines.push(format!(
            "#{} = SHAPE_DEFINITION_REPRESENTATION(#{},#{});",
            base + 5,
            base + 3,
            base + 4
        ));
        index += 6;

        nodes.push(AssemblyNode {
            name: attrs.name.clone(),
            solid: handle.clone(),
            intents: attrs.rows.clone(),
        });
    }

    let mut step = String::new();
    step.push_str("ISO-10303-21;\n");
    step.push_str("HEADER;\n");
    step.push_str("FILE_DESCRIPTION(('truck123d assembly emission'),'2;1');\n");
    step.push_str("FILE_NAME('assembly.step','',(''),(''),'truck123d','','');\n");
    step.push_str("FILE_SCHEMA(('");
    step.push_str(STEP_SCHEMA);
    step.push_str("'));\n");
    step.push_str("ENDSEC;\n");
    step.push_str("DATA;\n");
    step.push_str("#1 = APPLICATION_CONTEXT('assembly context');\n");
    step.push_str("#2 = PRODUCT_CONTEXT('',#1,'mechanical');\n");
    step.push_str("#3 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');\n");
    step.push_str("#4 = REPRESENTATION_CONTEXT('Context #1','3D Context');\n");
    step.push_str("#5 = DIRECTION('',(0.,0.,1.));\n");
    step.push_str("#6 = DIRECTION('',(1.,0.,0.));\n");
    step.push_str("#7 = CARTESIAN_POINT('',(0.,0.,0.));\n");
    step.push_str("#8 = AXIS2_PLACEMENT_3D('',#7,#5,#6);\n");
    for line in lines {
        step.push_str(&line);
        step.push('\n');
    }
    step.push_str("ENDSEC;\n");
    step.push_str("END-ISO-10303-21;\n");

    Ok(AssemblyReport { nodes, step })
}

/// Reads an emitted STEP assembly back: parses the product records of `step`
/// (one per node), decodes each node's typed record, and returns the report.
///
/// This is the round-trip partner of [`emit_step_assembly`]. A read-back that
/// finds no product records, cannot parse a product line, or cannot decode a
/// node record as a typed [`NodeRecord`] (an unreadable/foreign assembly)
/// refuses with [`Refusal::Empty`].
pub fn read_step_assembly(step: &str) -> Result<AssemblyReport, Refusal> {
    let products = parse_products(step)?;
    if products.is_empty() {
        return Err(Refusal::Empty);
    }
    let mut nodes = Vec::with_capacity(products.len());
    for (id, name, description) in products {
        let record: NodeRecord = serde_json::from_str(&description).map_err(|_| Refusal::Empty)?;
        nodes.push(AssemblyNode {
            name,
            solid: SolidHandle {
                id,
                kind: record.solid_kind,
            },
            intents: record.intents,
        });
    }
    Ok(AssemblyReport {
        nodes,
        step: step.to_string(),
    })
}

/// Walks the landed assembly graph (the parts, in insertion order, built into
/// the same graph [`emit_step_assembly`] walks) into the per-node occurrence
/// labels that name the GLB emission's node payloads (PB-009).
///
/// The corpus convention is frozen by the vendored tree, not researched: the
/// `f1.step.js` OCCURRENCES block pins each top-level child of the root
/// assembly `o1` by insertion order (`front_wing: "#o1.1"` … `details:
/// "#o1.28"`). This function reproduces that table from assembly insertion
/// order alone: the `p`-th part under the root occurrence `root` is named
/// `#root.{p + 1}`. Returns one label per part, in insertion order (an empty
/// part list yields an empty label list).
pub fn assembly_occurrence_labels(root: &str, parts: &[AssemblyPart]) -> Vec<String> {
    let mut graph = AssemblyGraph::new();
    for part in parts {
        graph.create_node(NodeEntity {
            shape: part.solid.clone(),
            attrs: AssemblyNodeAttrs {
                name: part.name.clone(),
                rows: Vec::new(),
            },
        });
    }
    let mut labels = Vec::with_capacity(parts.len());
    for (position, _node) in graph.all_nodes().enumerate() {
        labels.push(format!("#{root}.{}", position + 1));
    }
    labels
}

/// The evidence rows of the node `name`: every intended contact in which
/// `name` participates (as `node_a` or `node_b`), in list order.
fn rows_touching(name: &str, intents: &[AssemblyContactIntent]) -> Vec<AssemblyContactIntent> {
    intents
        .iter()
        .filter(|intent| intent.node_a == name || intent.node_b == name)
        .cloned()
        .collect()
}

/// Escapes a string for a STEP simple string literal: `'` becomes `''`.
fn escape_step_string(content: &str) -> String {
    content.replace('\'', "''")
}

/// Parses the `PRODUCT` records of an emitted STEP text, returning each
/// record's `(id, name, description)` triple in document order.
fn parse_products(step: &str) -> Result<Vec<(String, String, String)>, Refusal> {
    let mut out = Vec::new();
    for line in step.lines() {
        let Some(marker) = line.find("= PRODUCT(") else {
            continue;
        };
        let Some(rest) = line.get(marker + "= PRODUCT(".len()..) else {
            return Err(Refusal::Empty);
        };
        let mut chars = rest.chars().peekable();
        let id = read_step_string(&mut chars).ok_or(Refusal::Empty)?;
        let name = read_step_string(&mut chars).ok_or(Refusal::Empty)?;
        let description = read_step_string(&mut chars).ok_or(Refusal::Empty)?;
        out.push((id, name, description));
    }
    Ok(out)
}

/// Reads one STEP simple string literal, unescaping doubled single quotes.
/// Skips any leading separator characters (commas, whitespace) up to the
/// opening quote; `None` when the stream ends before a string opens.
fn read_step_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<String> {
    loop {
        match chars.next() {
            Some('\'') => break,
            Some(_) => {}
            None => return None,
        }
    }
    let mut value = String::new();
    loop {
        match chars.next() {
            Some('\'') => {
                if matches!(chars.peek(), Some('\'')) {
                    let _ = chars.next();
                    value.push('\'');
                } else {
                    return Some(value);
                }
            }
            Some(c) => value.push(c),
            None => return None,
        }
    }
}

/// Whether `s` contains any control character (would break a line-based STEP
/// simple string or a deterministic record).
fn has_control_char(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}
