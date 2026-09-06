//! The build123d-shaped Python facade (PB-005).
//!
//! Design doctrine (spec §5, PB-000 §1): the Python side EDITS TABLES, then
//! submits. Builder-mode statefulness is Python-side only — a context manager
//! collects operations into a [`FacadeTable`] and submits once; no kernel
//! state crosses the boundary. This module owns the submitted table schema,
//! the single native facade entry ([`run_facade`]), and the deterministic
//! facade report. Nothing computes in Python and nothing geometric is computed
//! here either: [`run_facade`] validates the table against the envelope rules
//! the landed kernel doctrine fixes (empty domains refuse `Empty`; STEP out of
//! a constructive/swept part stays behind the TR-NRB-001 boundary and refuses
//! typed) and returns a byte-deterministic ledger. Identical ordered input →
//! identical tables → identical reports.
//!
//! The op vocabulary is the corpus surface (spec §8) made expressible as table
//! rows, no more:
//!
//! * builder frames — `push_part` / `push_sketch` / `pop`;
//! * `Mode` algebra — `mode` (`union` / `subtract` / `intersect`, the §3.2
//!   encoding of build123d's `Add` / `Subtract` / `Intersect`);
//! * primitives — `box`, `cylinder`, `sphere`, `torus` (solids) and `polygon`,
//!   `polyline`, `spline` (sketch carriers);
//! * topology verb — `make_face`;
//! * verbs — `extrude`, `revolve`, `sweep`, `loft`, `fillet`, `chamfer`,
//!   `mirror`;
//! * fluent selectors (PB-001's machinery exposed thinly as rows) — the
//!   four-expression vocabulary is `select_faces`, `select_edges`,
//!   `select_filter`, `select_take`. A `fillet`/`chamfer` row binds the edges
//!   named by the selection rows recorded immediately before it in the same
//!   frame; a blend with no recorded selection is an empty domain and refuses.
//! * exports — `export_stl` (any part) and `export_step` (prismatic parts
//!   only; STEP for a swept/constructive part refuses `NonCanonicalCarrier`).
//!
//! Every entry point lands here as one Rust fn taking the submitted table —
//! [`run_facade`] — so the native Rust facade entry (PB-008's third timing
//! regime) calls the same fn without Python. [`facade_submit`] is the thin
//! pyo3 wrapper that hands a marshaled refusal to Python as the typed
//! `Refused`/`Unresolved` exception.
//!
//! H-1 applies: nothing here may panic, unwrap, expect, or index, because the
//! submitted table is geometry-adjacent input even though no kernel geometry
//! runs.

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

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use truck_base::evidence::{EnvelopeCase, Refusal};

use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload};
use crate::python;

/// The `Mode` algebra vocabulary (§3.2): build123d's `Add`/`Subtract`/
/// `Intersect` are marshaled as `union`/`subtract`/`intersect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeValue {
    /// build123d `Mode.ADD` — the default combine mode.
    #[serde(rename = "union")]
    Add,
    /// build123d `Mode.SUBTRACT`.
    Subtract,
    /// build123d `Mode.INTERSECT`.
    Intersect,
}

/// The axis vocabulary of `select_filter` and `mirror`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AxisValue {
    /// The x axis.
    X,
    /// The y axis.
    Y,
    /// The z axis.
    Z,
}

/// One row of a submitted facade session. Every row is data only; the kernel
/// executes the table behind the client layer, never here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FacadeOp {
    /// Enter a `BuildPart` context (builder-mode frame).
    PushPart,
    /// Enter a `BuildSketch` context within the current part.
    PushSketch,
    /// Leave the innermost builder context.
    Pop,
    /// Switch the part's combine mode for the following solid ops (`Mode`
    /// algebra sugar).
    Mode {
        /// The mode to switch to.
        value: ModeValue,
    },
    /// `Box(length, width, height)` — a solid primitive.
    Box {
        /// Length along x.
        length: f64,
        /// Width along y.
        width: f64,
        /// Height along z.
        height: f64,
    },
    /// `Cylinder(radius, height)` — a solid primitive, axis-aligned on z.
    Cylinder {
        /// Base radius.
        radius: f64,
        /// Height along z.
        height: f64,
    },
    /// `Sphere(radius)` — a solid primitive.
    Sphere {
        /// The sphere radius.
        radius: f64,
    },
    /// `Torus(major_radius, minor_radius)` — a solid primitive on z.
    Torus {
        /// The major (ring) radius.
        major_radius: f64,
        /// The minor (tube) radius.
        minor_radius: f64,
    },
    /// `Polygon(*points)` — a closed sketch profile on the working plane.
    Polygon {
        /// The ordered profile vertices, `[x, y]` on the working plane.
        points: Vec<[f64; 2]>,
    },
    /// `Polyline(*points)` — an open sketch carrier on the working plane.
    Polyline {
        /// The ordered carrier vertices, `[x, y]` on the working plane.
        points: Vec<[f64; 2]>,
    },
    /// `Spline(*points, periodic=...)` — a spline sketch carrier (PB-002
    /// authoring). A spline carrier is a non-canonical carrier, so a part
    /// built from one is constructive for the STEP boundary.
    Spline {
        /// The ordered control/interpolating points.
        points: Vec<[f64; 3]>,
        /// Whether the spline closes on itself.
        periodic: bool,
    },
    /// `make_face()` — face a closed planar profile (the topology verb).
    MakeFace,
    /// `extrude(amount=...)` — extrude the current sketch along z.
    Extrude {
        /// The extrusion distance.
        amount: f64,
    },
    /// `revolve(angle=...)` — revolve the current sketch about z.
    Revolve {
        /// The revolve angle in degrees.
        angle_deg: f64,
    },
    /// `sweep(...)` — a spine sweep. Constructive for the STEP boundary.
    Sweep,
    /// `loft(...)` — a loft over the current section set. Constructive for
    /// the STEP boundary.
    Loft,
    /// `fillet(edges, radius)` — blend the edges named by the selection rows
    /// recorded immediately before this row.
    Fillet {
        /// The blend radius.
        radius: f64,
    },
    /// `chamfer(edges, length)` — chamfer the edges named by the selection
    /// rows recorded immediately before this row.
    Chamfer {
        /// The chamfer length.
        length: f64,
    },
    /// `mirror(axis=...)` — mirror the current solid about the given
    /// axis-aligned plane.
    Mirror {
        /// The axis whose plane mirrors the solid.
        axis: AxisValue,
    },
    /// `faces()` — record a faces census of the current solid (the first of
    /// the four-expression selector vocabulary).
    SelectFaces,
    /// `edges()` — record an edges census of the current solid (the second
    /// of the four-expression selector vocabulary).
    SelectEdges,
    /// `filter_by(axis)` — narrow the current selection to the entities whose
    /// census sits on `axis` (the third of the four-expression selector
    /// vocabulary; PB-001 resolves the actual entities at kernel time).
    SelectFilter {
        /// The census axis.
        axis: AxisValue,
    },
    /// `take(count)` — keep the first `count` entities of the current
    /// selection (the fourth of the four-expression selector vocabulary).
    SelectTake {
        /// How many entities to keep.
        count: usize,
    },
    /// `export_stl(path)` — record an STL export entry for the finished part.
    ExportStl {
        /// The requested output path.
        path: String,
    },
    /// `export_step(path)` — record a STEP export entry. Refuses typed
    /// (`NonCanonicalCarrier`) when the part carries a constructive surface:
    /// STEP out stays behind the TR-NRB-001 boundary (STL only).
    ExportStep {
        /// The requested output path.
        path: String,
    },
}

/// The submitted session table: the ordered operation log a builder session
/// records. Order is load-bearing and preserved end to end (determinism).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeTable {
    /// The operation rows, in session order.
    pub ops: Vec<FacadeOp>,
}

/// The serialized `schema` tag of a facade report.
pub const REPORT_SCHEMA: &str = "facade_report.v1";

/// One export entry of a facade report. Only in-envelope exports appear here:
/// a refused export aborts the run as a typed refusal instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExportEntry {
    /// The export verb that produced the entry (`export_stl`/`export_step`).
    pub op: &'static str,
    /// The requested output path.
    pub path: String,
}

/// The deterministic ledger [`run_facade`] returns for an in-envelope table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeReport {
    /// The report schema tag (`facade_report.v1`).
    pub schema: &'static str,
    /// Always `true` on the `Ok` path; present so a report is self-describing.
    pub ok: bool,
    /// The number of submitted op rows.
    pub op_count: usize,
    /// Whether the part carries a constructive/swept surface (the fixture
    /// classification the TR-NRB-001 STEP boundary keys on).
    pub constructive: bool,
    /// The export entries, in session order.
    pub exports: Vec<ExportEntry>,
}

/// The single native facade entry: takes the submitted table and returns the
/// deterministic report, or the typed kernel refusal the envelope produced.
/// This is the fn PB-008's native timing regime calls without Python.
pub fn run_facade(table: &FacadeTable) -> Result<FacadeReport, Refusal> {
    let mut constructive = false;
    let mut exports: Vec<ExportEntry> = Vec::new();
    let mut solid_ops = 0usize;
    // A blend row must bind the selection rows recorded immediately before it
    // in the same frame; a blend with no recorded selection is an empty edge
    // domain and refuses (`Refusal::Empty`).
    let mut selection_pending = false;

    for op in &table.ops {
        match op {
            FacadeOp::Sweep | FacadeOp::Loft => {
                constructive = true;
                solid_ops += 1;
                selection_pending = false;
            }
            FacadeOp::Spline { .. } => {
                // A spline carrier is a non-canonical carrier: any part built
                // over one is constructive for the STEP boundary.
                constructive = true;
                selection_pending = false;
            }
            FacadeOp::Box { .. }
            | FacadeOp::Cylinder { .. }
            | FacadeOp::Sphere { .. }
            | FacadeOp::Torus { .. }
            | FacadeOp::MakeFace
            | FacadeOp::Extrude { .. }
            | FacadeOp::Revolve { .. }
            | FacadeOp::Mirror { .. } => {
                solid_ops += 1;
                selection_pending = false;
            }
            FacadeOp::Polygon { .. } | FacadeOp::Polyline { .. } => {
                // Sketch carriers: not a solid and not constructive by
                // themselves.
                selection_pending = false;
            }
            FacadeOp::SelectFaces | FacadeOp::SelectEdges => {
                selection_pending = true;
            }
            FacadeOp::SelectFilter { .. } | FacadeOp::SelectTake { .. } => {}
            FacadeOp::Fillet { .. } | FacadeOp::Chamfer { .. } => {
                if !selection_pending {
                    return Err(Refusal::Empty);
                }
                selection_pending = false;
            }
            FacadeOp::ExportStl { path } => {
                if solid_ops == 0 {
                    return Err(Refusal::Empty);
                }
                selection_pending = false;
                exports.push(ExportEntry {
                    op: "export_stl",
                    path: path.clone(),
                });
            }
            FacadeOp::ExportStep { path } => {
                if solid_ops == 0 {
                    return Err(Refusal::Empty);
                }
                if constructive {
                    // TR-NRB-001: STEP out of constructive/swept surfaces is a
                    // typed kernel refusal, never a silent STL fallback.
                    return Err(Refusal::UnsupportedEnvelope(
                        EnvelopeCase::NonCanonicalCarrier,
                    ));
                }
                selection_pending = false;
                exports.push(ExportEntry {
                    op: "export_step",
                    path: path.clone(),
                });
            }
            FacadeOp::PushPart | FacadeOp::PushSketch | FacadeOp::Pop | FacadeOp::Mode { .. } => {}
        }
    }

    Ok(FacadeReport {
        schema: REPORT_SCHEMA,
        ok: true,
        op_count: table.ops.len(),
        constructive,
        exports,
    })
}

/// The pyo3 wrapper over [`run_facade`]: parses the submitted table JSON,
/// runs the facade, and returns the report JSON. A typed kernel refusal
/// (empty domain, constructive-part STEP export, ...) raises the `Refused`/
/// `Unresolved` exception with the marshaled payload — never a bare
/// `Exception`. Nothing computes here and no kernel call is made, so the GIL
/// is not released (the detached-closure choke point is for kernel calls).
#[pyfunction]
pub fn facade_submit(py: Python<'_>, table_json: &str) -> PyResult<String> {
    let table: FacadeTable = serde_json::from_str(table_json)
        .map_err(|e| PyValueError::new_err(format!("invalid facade table JSON: {e}")))?;
    match run_facade(&table) {
        Ok(report) => serde_json::to_string(&report).map_err(|e| {
            PyRuntimeError::new_err(format!("facade report serialization failed: {e}"))
        }),
        Err(refusal) => {
            let marshaled = Marshaled::from_refusal(&refusal);
            let payload_json = match &marshaled.payload {
                MarshaledPayload::Refused(payload) => serde_json::to_string(payload).map_err(|e| {
                    PyRuntimeError::new_err(format!("Refused payload serialization failed: {e}"))
                }),
                MarshaledPayload::Unresolved(payload) => {
                    serde_json::to_string(payload).map_err(|e| {
                        PyRuntimeError::new_err(format!(
                            "Unresolved payload serialization failed: {e}"
                        ))
                    })
                }
            }?;
            let instance = match marshaled.class {
                ExceptionClass::Refused => python::marshal_refused(py, &payload_json)?,
                ExceptionClass::Unresolved => python::marshal_unresolved(py, &payload_json)?,
            };
            Err(PyErr::from_value(instance.bind(py).clone()))
        }
    }
}
