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
//!   `polyline`, `spline`, `circle` (sketch carriers);
//! * topology verb — `make_face`;
//! * verbs — `extrude`, `revolve`, `revolve_arc`, `sweep`, `loft`, `fillet`,
//!   `chamfer`, `mirror`;
//! * fluent selectors (PB-001's machinery exposed thinly as rows) — the
//!   four-expression vocabulary is `select_faces`, `select_edges`,
//!   `select_filter`, `select_take`. A `fillet`/`chamfer` row binds the edges
//!   named by the selection rows recorded immediately before it in the same
//!   frame; a blend with no recorded selection is an empty domain and refuses.
//! * exports — `export_stl` (any part) and `export_step` (prismatic parts
//!   only; STEP for a swept/constructive part refuses `NonCanonicalCarrier`).
//! * swept-carrier booleans (PB-011 G1, ADM-004-FUNNEL-WIRING) — a `Mode` row
//!   over a swept carrier (spline/swept/revolved carrier class) is a
//!   swept-carrier boolean row: [`run_facade`] consults admission (the ADM-000
//!   dispatch rule — canonical dispatch stays ahead) and dispatches the
//!   admitted pair through the landed certified entry
//!   ([`dispatch_swept_carrier_boolean`], the facade mirror of the CL-006
//!   solver-entry dispatch), recording each accepted row as a
//!   [`SweptBooleanEvent`] on the report. A pair the certified funnel refuses
//!   answers the typed, localized refusal — a torus carrier in a swept pair is
//!   `ContactReductionDeferred`, and a both-Swept pair (two spline-loft
//!   solids, not admitted at the carrier-class granularity) keeps the
//!   constructive-carrier `NonCanonicalCarrier` refusal at the boolean
//!   boundary — fail-closed, never a bare `Err`. Canonical x canonical pairs
//!   are untouched (the landed S1 path: no event, no dispatch).
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

/// The boolean-carrier class of a solid — the carrier taxonomy the
/// swept-carrier boolean routing keys on, mirroring the landed funnel's
/// stage contract on the kernel side (CFP-004): spline/swept/revolved
/// carriers route into the certified entry; a torus carrier inside a swept
/// pair stays a typed refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarrierClass {
    /// A canonical carrier (box/cylinder/sphere solid, or the exact
    /// conic/polygon profile family). Canonical x canonical booleans land on
    /// the S1 canonical path, never the certified dispatch.
    Canonical,
    /// A spline sketch carrier (S5 section authoring).
    Spline,
    /// A swept/lofted carrier (a `loft`/`sweep` solid over a profile).
    Swept,
    /// A revolved carrier (a `revolve`/`revolve_arc` solid over a spline
    /// profile).
    Revolved,
    /// A torus carrier: the certified funnel refuses torus carriers in swept
    /// pairs (the torus is excluded from the implicit-reduction stage).
    Torus,
}

impl CarrierClass {
    /// Whether the class is a swept carrier the certified funnel supports
    /// (spline/swept/revolved per the CFP-004 stage contract).
    pub const fn is_funnel_carrier(self) -> bool {
        matches!(
            self,
            CarrierClass::Spline | CarrierClass::Swept | CarrierClass::Revolved
        )
    }
}

/// One routed swept-carrier boolean row of a facade session (PB-011 G1): a
/// `Mode` row combined the current swept carrier with a tool and the pair was
/// dispatched through the certified entry, which accepted it. Session order is
/// preserved (determinism).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SweptBooleanEvent {
    /// The boolean mode of the routed row.
    pub mode: ModeValue,
    /// The carrier class of the base solid the row combined.
    pub base: CarrierClass,
    /// The carrier class of the tool solid the row combined.
    pub tool: CarrierClass,
}

/// The certified route of one swept-carrier boolean pair into the landed
/// certified entry (CL-006 solver-entry dispatch): the pair was accepted and
/// the boolean runs on the certified funnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedBooleanRoute {
    /// The boolean mode of the routed pair.
    pub mode: ModeValue,
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
}

/// The typed, localized refusal of a swept-carrier boolean pair the certified
/// funnel refuses. Fail-closed: the refusal carries the envelope case and the
/// carrier-class pair identity (the stratum-pair localization). The envelope
/// case is [`EnvelopeCase::ContactReductionDeferred`] for a torus carrier in a
/// swept pair, and [`EnvelopeCase::NonCanonicalCarrier`] for a swept pair the
/// admission consult keeps outside the certified funnel (the constructive-
/// carrier refusal at the boolean boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweptBooleanRefusal {
    /// The boolean mode of the refused pair.
    pub mode: ModeValue,
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
    /// The typed envelope case of the refusal.
    pub case: EnvelopeCase,
}

/// The observed verdict of one boolean-carrier pair through the facade
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanPairVerdict {
    /// Canonical x canonical: the landed S1 canonical path — in envelope, no
    /// certified dispatch row.
    CanonicalLanded,
    /// The swept pair dispatched into the certified entry and certified.
    Routed(CertifiedBooleanRoute),
    /// The swept pair was refused typed, with the envelope case and pair
    /// identity.
    Refused(SweptBooleanRefusal),
}

/// The ADM-004 admission consult for one swept-carrier class pair (the facade
/// mirror of the ADM-000 dispatch rule, wired by ADM-004-FUNNEL-WIRING):
/// whether the certified funnel admits the pair at the carrier-class
/// granularity the facade owns.
///
/// The certified funnel admits swept-carrier booleans end to end for the
/// classes whose faces the Theorem A adapter serves (ADM-001+) — a swept pair
/// against a canonical or revolved carrier, and the routed corner/shell pairs
/// of the PB-011 waves. MONO-8-SWEPT-ADMISSION-WIRING widens the consult to the
/// `Swept` × `Swept` cell: the executor now extracts both operands' patch
/// 2-cycles and routes the pair through the fixed gate order (extraction →
/// transversality → solver), so the fine-grained verdict — a certified bracket
/// or the typed stage refusal — is decided at the geometry granularity rather
/// than by the carrier class. The class-level consult therefore admits every
/// pair carrying a funnel carrier; canonical dispatch stays ahead, and the
/// consult fires only where the old path refused `NonCanonicalCarrier`.
fn swept_pair_is_admitted(base: CarrierClass, tool: CarrierClass) -> bool {
    base.is_funnel_carrier() || tool.is_funnel_carrier()
}

/// Routes one boolean carrier pair through the landed certified entry
/// (PB-011 scope decision 1, the "exposed row"). A pair carrying a
/// spline/swept/revolved carrier is the corpus's swept-carrier boolean: it
/// consults admission (ADM-004-FUNNEL-WIRING, the ADM-000 dispatch rule) and
/// dispatches into the certified funnel when the class pair is admitted. The
/// `Swept` × `Swept` cell is admitted by MONO-8-SWEPT-ADMISSION-WIRING: the
/// executor extracts both operands and runs the fixed gate order, so the pair
/// routes here and the fine-grained verdict is the executor's typed stage
/// refusal. A pair coupling a swept carrier with a torus carrier is a class
/// the funnel refuses (the torus is excluded from the implicit-reduction
/// stage, Theorem 4 economics) and answers the typed, localized
/// `ContactReductionDeferred` refusal — fail-closed, never a bare `Err`. A
/// canonical x canonical pair is the landed S1 canonical path and is reported
/// as [`BooleanPairVerdict::CanonicalLanded`], never dispatched.
///
/// This is the deterministic per-pair mirror of the kernel's CL-006
/// `dispatch_restricted_sweep`; the loop-side facade cannot name the
/// certified crate (dependency law, same as PB-013/PB-014), so the certified
/// outcome is asserted here at the pair-verdict granularity the report layer
/// consumes.
pub fn dispatch_swept_carrier_boolean(
    base: CarrierClass,
    tool: CarrierClass,
    mode: ModeValue,
) -> BooleanPairVerdict {
    let base_swept = base.is_funnel_carrier();
    let tool_swept = tool.is_funnel_carrier();
    if !base_swept && !tool_swept {
        return BooleanPairVerdict::CanonicalLanded;
    }
    if base == CarrierClass::Torus || tool == CarrierClass::Torus {
        return BooleanPairVerdict::Refused(SweptBooleanRefusal {
            mode,
            base,
            tool,
            case: EnvelopeCase::ContactReductionDeferred,
        });
    }
    // The admission consult (scope decision 2): run_facade consults admission
    // BEFORE the `NonCanonicalCarrier` refusal — canonical dispatch stays
    // ahead, and a pair outside the funnel carriers keeps the exact refusal.
    if !swept_pair_is_admitted(base, tool) {
        return BooleanPairVerdict::Refused(SweptBooleanRefusal {
            mode,
            base,
            tool,
            case: EnvelopeCase::NonCanonicalCarrier,
        });
    }
    BooleanPairVerdict::Routed(CertifiedBooleanRoute { mode, base, tool })
}

/// The carrier class a solid-producing op yields, or `None` for an op that
/// does not change the current solid's carrier. `profile` is the class of the
/// current sketch/profile carrier (a spline profile makes a revolve/loft
/// constructive).
fn produced_carrier_class(op: &FacadeOp, profile: CarrierClass) -> Option<CarrierClass> {
    match op {
        FacadeOp::Box { .. } | FacadeOp::Cylinder { .. } | FacadeOp::Sphere { .. } => {
            Some(CarrierClass::Canonical)
        }
        FacadeOp::Torus { .. } => Some(CarrierClass::Torus),
        FacadeOp::Loft | FacadeOp::Sweep => Some(CarrierClass::Swept),
        FacadeOp::Extrude { .. } => Some(if profile == CarrierClass::Spline {
            CarrierClass::Swept
        } else {
            CarrierClass::Canonical
        }),
        FacadeOp::Revolve { .. } | FacadeOp::RevolveArc { .. } => {
            Some(if profile == CarrierClass::Spline {
                CarrierClass::Revolved
            } else {
                CarrierClass::Canonical
            })
        }
        FacadeOp::Polygon { .. }
        | FacadeOp::Polyline { .. }
        | FacadeOp::Circle { .. }
        | FacadeOp::Spline { .. }
        | FacadeOp::MakeFace
        | FacadeOp::Mirror { .. }
        | FacadeOp::Fillet { .. }
        | FacadeOp::Chamfer { .. }
        | FacadeOp::SelectFaces
        | FacadeOp::SelectEdges
        | FacadeOp::SelectFilter { .. }
        | FacadeOp::SelectTake { .. }
        | FacadeOp::ExportStl { .. }
        | FacadeOp::ExportStep { .. }
        | FacadeOp::PushPart
        | FacadeOp::PushSketch
        | FacadeOp::Pop
        | FacadeOp::Mode { .. } => None,
    }
}

/// The product carrier class of a boolean: a swept product stays in the swept
/// family (the base class when the base is a swept carrier, else the tool's).
fn boolean_product_carrier(base: CarrierClass, tool: CarrierClass) -> CarrierClass {
    if base.is_funnel_carrier() { base } else { tool }
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
    /// `Circle(radius)` — a closed-circle profile carrier on the working
    /// plane (audit G3, PB-014). Mirrors the Spline section-authoring row
    /// (S5): a closed circle section plus `make_face` yields a disc profile
    /// that feeds the landed revolve/loft rows (cockpit ring stacks, Merlin
    /// tube rings). A circle is an exact conic profile, so the carrier is
    /// canonical; a part built over circle sections becomes constructive when
    /// the verb is (`loft`/`sweep`, or a spline-carrier revolve), never by
    /// the carrier alone.
    Circle {
        /// The radius of the closed circle profile.
        radius: f64,
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
    /// `revolve_arc(arc_deg=..., start_deg=...)` — a partial-arc revolve
    /// (audit G5, PB-014). Over the landed full-revolve surface the bridge
    /// builds the full revolution, then emits a shell of trimmed faces whose
    /// v-range is `[start_deg, start_deg + arc_deg]` plus two planar cap
    /// faces bounded by the profile and its rotated image (the caps close the
    /// solid). Constructive classification follows the profile carrier,
    /// exactly as a full `revolve`.
    RevolveArc {
        /// The swept arc in degrees (`(0, 360]`).
        arc_deg: f64,
        /// The v-range start in degrees.
        start_deg: f64,
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
    /// The routed swept-carrier boolean rows of the session (PB-011 G1), in
    /// session order. Empty (and omitted from the serialized report) when no
    /// swept-carrier boolean row occurred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boolean_events: Vec<SweptBooleanEvent>,
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

    // PB-011 G1 routing state: the boolean carrier class of the current
    // solid, the pending `Mode` row (the sugar emits one `mode` row directly
    // before the solid op it modifies), the current sketch/profile carrier,
    // and the routed swept-carrier boolean rows in session order.
    let mut carrier = CarrierClass::Canonical;
    let mut profile = CarrierClass::Canonical;
    let mut pending_mode: Option<ModeValue> = None;
    let mut boolean_events: Vec<SweptBooleanEvent> = Vec::new();

    for op in &table.ops {
        match op {
            FacadeOp::Sweep | FacadeOp::Loft => {
                constructive = true;
                solid_ops += 1;
                selection_pending = false;
                let profile_before = profile;
                profile = CarrierClass::Canonical;
                carrier = apply_solid_class(
                    op,
                    carrier,
                    &mut pending_mode,
                    profile_before,
                    &mut boolean_events,
                )?;
            }
            FacadeOp::Spline { .. } => {
                // A spline carrier is a non-canonical carrier: any part built
                // over one is constructive for the STEP boundary.
                constructive = true;
                profile = CarrierClass::Spline;
                selection_pending = false;
            }
            FacadeOp::Polygon { .. } | FacadeOp::Polyline { .. } | FacadeOp::Circle { .. } => {
                // Sketch carriers: not a solid and not constructive by
                // themselves (a circle is an exact conic profile, canonical
                // like a polygon; only the constructive verbs over one mark
                // the part constructive).
                profile = CarrierClass::Canonical;
                selection_pending = false;
            }
            FacadeOp::Box { .. }
            | FacadeOp::Cylinder { .. }
            | FacadeOp::Sphere { .. }
            | FacadeOp::Torus { .. }
            | FacadeOp::Extrude { .. }
            | FacadeOp::Revolve { .. }
            | FacadeOp::RevolveArc { .. } => {
                solid_ops += 1;
                selection_pending = false;
                let profile_before = profile;
                profile = CarrierClass::Canonical;
                carrier = apply_solid_class(
                    op,
                    carrier,
                    &mut pending_mode,
                    profile_before,
                    &mut boolean_events,
                )?;
            }
            FacadeOp::MakeFace => {
                // The topology verb faces a closed planar profile; it is not a
                // solid and does not advance the part's carrier or consume the
                // profile (the verb that runs after `Pop` consumes it).
                solid_ops += 1;
                selection_pending = false;
            }
            FacadeOp::Mirror { .. } => {
                // A mirror keeps the part's carrier class (and its STEP
                // constructiveness): a mirrored swept product is still swept.
                solid_ops += 1;
                selection_pending = false;
            }
            FacadeOp::Mode { value } => {
                // The `Mode` row is the facade's `boolean_op` encoding: it
                // arms the boolean that the next solid op performs. Rows on
                // non-canonical carriers reach the certified dispatch (PB-011
                // G1) instead of being refused pre-execution; the dispatch
                // itself runs when the tool's class is known, on the next
                // solid op.
                pending_mode = Some(*value);
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
            FacadeOp::PushPart => {
                // A fresh part frame starts canonical; a part opened inside
                // another frame inherits nothing from the enclosing part.
                carrier = CarrierClass::Canonical;
                profile = CarrierClass::Canonical;
                pending_mode = None;
            }
            FacadeOp::PushSketch => {
                // A fresh sketch frame: the profile carrier starts canonical.
                profile = CarrierClass::Canonical;
            }
            FacadeOp::Pop => {
                // Popping a sketch/part frame: the profile was consumed by the
                // verb that ran inside the frame; the carrier follows the last
                // solid of the frame (the existing model keeps no frame stack,
                // so a pop never rewinds the carrier).
            }
        }
    }

    Ok(FacadeReport {
        schema: REPORT_SCHEMA,
        ok: true,
        op_count: table.ops.len(),
        constructive,
        boolean_events,
        exports,
    })
}

/// Applies a solid-producing op to the carrier state: resolves the pending
/// `Mode` row (the swept-carrier boolean dispatch) and records routed rows or
/// refuses typed, then advances the current carrier.
fn apply_solid_class(
    op: &FacadeOp,
    base: CarrierClass,
    pending_mode: &mut Option<ModeValue>,
    profile: CarrierClass,
    boolean_events: &mut Vec<SweptBooleanEvent>,
) -> Result<CarrierClass, Refusal> {
    let produced = produced_carrier_class(op, profile).unwrap_or(CarrierClass::Canonical);
    match *pending_mode {
        None => Ok(produced),
        Some(mode) => {
            *pending_mode = None;
            match dispatch_swept_carrier_boolean(base, produced, mode) {
                BooleanPairVerdict::CanonicalLanded => Ok(produced),
                BooleanPairVerdict::Routed(route) => {
                    boolean_events.push(SweptBooleanEvent {
                        mode: route.mode,
                        base: route.base,
                        tool: route.tool,
                    });
                    Ok(boolean_product_carrier(route.base, route.tool))
                }
                BooleanPairVerdict::Refused(refusal) => {
                    Err(Refusal::UnsupportedEnvelope(refusal.case))
                }
            }
        }
    }
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
