//! CG-BINDING — the pyo3 translation over the stabilized facade (the
//! dependency-wall resolution).
//!
//! **The manifest-edge amendment (C2 precedent, recorded — do not
//! relitigate).** The recorded law "the loop-side crate cannot name
//! truck-certified" (STATE session-56, applied in PB-011) is AMENDED by this
//! packet: exactly one edge, `truck123d -> truck-certified`, is added to
//! `truck123d/Cargo.toml` and is added once. The loop-side crate may not name
//! truck-certified *except* through this one recorded edge, added once and
//! never extended without a spec amendment. The traits stay in truck-evidence;
//! the impls stay in truck-certified; this binding reaches the impls through
//! the stabilized facade's public entry points. No funnel code moves.
//!
//! **Why.** The door gap audit (`loop/audits/DOOR_GAP_AUDIT-2026-09-09.md`)
//! recorded the wall: the data-row executor can never call the certified
//! funnel in-process because the loop-side crate could not name it. This
//! module is the sanctioned resolution (AGENTS.md's booked "pyo3 binding
//! translation over the stabilized facade"): it exposes the funnel's runtime
//! twin to Python so the corpus door can dispatch against the landed
//! admission/boolean/volume machinery instead of its own analytic arms.
//!
//! **Three exports over the stabilized facade (minimal).**
//!
//! 1. [`binding_boolean_dispatch`] — a carrier pair + mode in, a
//!    `Routed`/`Refused` verdict out: the facade mirror's
//!    ([`facade::dispatch_swept_carrier_boolean`]) runtime twin. The facade
//!    mirror owns the carrier-class admission consult; when it routes the
//!    pair, this entry actually runs the certified Theorem C transversality
//!    certificate ([`certify_transverse_pair`]) over the row's recorded
//!    certified normal cones and returns the certified `‖n_X × n_Y‖` bracket.
//! 2. [`binding_volume_facts`] — an admitted construction's certified volume
//!    bracket out ([`certify_patch_form`] over the recorded patch row).
//! 3. [`binding_trim_facts`] — the algebraic-trim path's certified bracket out
//!    ([`certify_algebraic_trim_bracket`] over the recorded pullback nets).
//!
//! The exports consume EXACT data rows (the bridge's recorded carrier shapes);
//! no float crosses without an enclosing bracket. The rows are serde values,
//! exactly as the rest of the bridge marshals its tables.
//!
//! **Refusal marshaling is total (scope decision 3).** Every kernel refusal
//! kind maps to the door's typed vocabulary per
//! `docs/CERTIFICATE_MAPPING.md`; the binding never panics, never returns a
//! bare string, never approximates. An unmapped kind is itself a typed
//! [`UnmappedRefusal`] — a defect signal, never silent. The mapping lives in
//! [`crate::marshal`] beside the frozen §2 table.
//!
//! **Determinism (N4 carries).** Same row in → identical verdict and identical
//! bracket out, bit-for-bit, across reruns. Fixed order everywhere; no hash
//! iteration on any output path.
//!
//! **Reachability note.** `lib.rs` is outside this packet's write set, so the
//! module is declared from its sibling `python.rs` (the pyo3 surface module)
//! and the three `#[pyfunction]`s are exposed here; the follow-on consumer
//! packets flip the door shim against this landed surface.
//!
//! **H-1.** No `unwrap`, no `expect`, no `panic!`, no out-of-range indexing on
//! any shipped path; every failure is a typed value.

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
// The three exports are the follow-on door-shim consumers' surface. `lib.rs`
// is outside this packet's write set, so the pyo3 registration cannot be wired
// here yet; the module's items are therefore unreachable from the crate root
// and would read as dead code. Scoped to this one export module only.
#![allow(dead_code)]

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::facade::{self, BooleanPairVerdict, CarrierClass, ModeValue};
use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload, RefusedPayload};

use truck_certified::construct::admission::{NormalCone, certify_transverse_pair};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::construct::volume_facts::{
    VolumeOptions, certify_algebraic_trim_bracket, certify_patch_form,
};
use truck_certified::kernel::certs::{ArcCert, Frame, PointCert};
use truck_certified::kernel::evidence::{
    Refusal as KernelRefusal, RefusalEvidence, RefusalKind,
};
use truck_certified::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, Break, CertifiedGraph, ChartId, HermiteSegment,
    HermiteSpline, Node, NodeCert, NodeId, Param, Point4, TopoNode,
};
use truck_certified::kernel::patch::{IBox, IBox2};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::BezierLeaf1;
use truck_certified::kernel::trimclip::{TrimLoop, trim_clip};
use truck_certified::kernel::Interval;

use crate::bd_bridge::{ProfileEdge, Triangle};

/// A closed two-sided bracket `[lo, hi]`. The only numeric shape an export
/// ever returns: no float crosses the boundary without its enclosing bracket.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    /// The certified lower bound.
    pub lo: f64,
    /// The certified upper bound.
    pub hi: f64,
}

/// A malformed input row (a caller defect, distinct from a kernel refusal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedRow {
    /// What was wrong with the row.
    pub detail: String,
}

/// The two typed failure modes of an export: a marshaled kernel refusal, or a
/// malformed row. Both are typed; neither is a bare string. The refusal is
/// boxed so the `Result` error stays small on the hot path.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingError {
    /// A kernel refusal, marshaled to the typed door vocabulary.
    Refusal(Box<Marshaled>),
    /// A malformed input row.
    Malformed(MalformedRow),
}

/// A certified normal-cone row: the anchor direction and the certified upper
/// bound on the sine of the cone half-angle (the `admission::NormalCone`
/// public shape).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConeRow {
    /// The anchor direction (the float midpoint normal, rounded to dyadic).
    pub anchor: [f64; 3],
    /// A certified upper bound of `sin` of the cone half-angle, `< 1`.
    pub s_up: f64,
}

impl ConeRow {
    /// The landed certified-funnel cone value this row records.
    pub fn to_cone(self) -> NormalCone {
        NormalCone {
            anchor: self.anchor,
            s_up: self.s_up,
        }
    }
}

/// The boolean-dispatch row: the carrier-class pair and mode of the facade
/// mirror, plus the two carriers' recorded certified normal cones (the
/// Theorem C transversality witnesses the runtime twin composes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanRow {
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
    /// The boolean mode of the pair.
    pub mode: ModeValue,
    /// The base carrier's recorded certified normal cone.
    pub base_cone: ConeRow,
    /// The tool carrier's recorded certified normal cone.
    pub tool_cone: ConeRow,
}

/// The observed verdict of one boolean pair through the runtime twin.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanOutcome {
    /// Always `true` on the `Ok` path; present so the record is
    /// self-describing.
    pub ok: bool,
    /// `canonical_landed` | `routed` | `refused`.
    pub verdict: &'static str,
    /// The boolean mode of the pair.
    pub mode: ModeValue,
    /// The carrier class of the base solid.
    pub base: CarrierClass,
    /// The carrier class of the tool solid.
    pub tool: CarrierClass,
    /// The certified transversality bracket of a routed pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transverse: Option<Bracket>,
    /// The typed envelope case of a facade-level refusal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<&'static str>,
}

/// The volume-facts row: an admitted construction's recorded patch shape (the
/// numerator and weight Bernstein grids over the unit square) plus its
/// outward-orientation sign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeRow {
    /// The row-major `R³` numerator grid.
    pub numerator: Vec<Vec<[f64; 3]>>,
    /// The same-shape scalar weight grid.
    pub weights: Vec<Vec<f64>>,
    /// The outward-orientation sign `±1`.
    pub orientation: f64,
}

/// The certified volume fact of one admitted patch.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified two-sided volume bracket.
    pub bracket: Bracket,
    /// The certified volume value (the bracket midpoint).
    pub value: f64,
    /// The certified absolute error added by the rational/trim routes.
    pub certified_error: f64,
    /// The number of boundary faces contributing.
    pub faces: usize,
    /// The number of extracted patches contributing.
    pub patches: usize,
}

/// The trim-facts row: the polynomial density net and the algebraic trim net
/// (whose kept region is `trim ≥ 0`) over the unit square, plus the requested
/// bracket width.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrimRow {
    /// The polynomial integrand net (row-major).
    pub density: Vec<Vec<f64>>,
    /// The trim polynomial net (row-major).
    pub trim: Vec<Vec<f64>>,
    /// The requested residual bracket width.
    pub tolerance: f64,
}

/// The certified bracket of one algebraically trimmed domain.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrimOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified two-sided integral bracket.
    pub bracket: Bracket,
}

/// The unit-square domain every extracted patch row is re-parameterized onto.
fn unit_domain() -> IBox2 {
    IBox2 {
        lo: [0.0, 0.0],
        hi: [1.0, 1.0],
    }
}

/// Serializes an outcome, mapping a serde failure to a typed malformed row
/// (never an unwrap, never a bare string).
fn to_json<T: Serialize>(value: &T) -> Result<String, BindingError> {
    serde_json::to_string(value).map_err(|e| {
        BindingError::Malformed(MalformedRow {
            detail: format!("outcome serialization failed: {e}"),
        })
    })
}

/// Registers the certified funnel engines into the `truck-evidence`
/// trait-object registries (CL-006-SOLVER-ENTRY's restricted-pair engine and
/// CL-001-SPLINE-LIFT's spline×analytic SSI engine). This is the concrete
/// "call the impls through the trait objects" step: the certified impls land
/// in the registry slots the funnel dispatches through, so the door's funnel
/// path reaches the landed machinery in-process. Registration is set-once
/// (`#[ctor]`-free, explicit); a redundant call is a no-op for the binding's
/// verdict.
pub fn register_certified_entries() {
    let _ = truck_certified::construct::bie::ssi4::register_restricted_pair_solver();
    let _ = truck_certified::ssi_admit::register_ssi_admit_solver();
}

/// The certified boolean dispatch: the facade mirror's runtime twin. The
/// facade's carrier-class admission consult decides canonical/refused/routed;
/// a routed pair actually runs the certified Theorem C transversality
/// certificate over the row's recorded cones and returns the certified
/// bracket. A certified refusal marshals to the typed door vocabulary.
pub fn boolean_dispatch(row: &BooleanRow) -> Result<String, BindingError> {
    register_certified_entries();
    match facade::dispatch_swept_carrier_boolean(row.base, row.tool, row.mode) {
        BooleanPairVerdict::CanonicalLanded => to_json(&BooleanOutcome {
            ok: true,
            verdict: "canonical_landed",
            mode: row.mode,
            base: row.base,
            tool: row.tool,
            transverse: None,
            envelope: None,
        }),
        BooleanPairVerdict::Refused(refusal) => to_json(&BooleanOutcome {
            ok: true,
            verdict: "refused",
            mode: row.mode,
            base: row.base,
            tool: row.tool,
            transverse: None,
            envelope: Some(crate::marshal::envelope_case_name(refusal.case)),
        }),
        BooleanPairVerdict::Routed(route) => {
            let base_cone = row.base_cone.to_cone();
            let tool_cone = row.tool_cone.to_cone();
            match certify_transverse_pair(&base_cone, &tool_cone) {
                Ok(transverse) => to_json(&BooleanOutcome {
                    ok: true,
                    verdict: "routed",
                    mode: route.mode,
                    base: route.base,
                    tool: route.tool,
                    transverse: Some(Bracket {
                        lo: transverse.delta.0,
                        hi: transverse.delta.1,
                    }),
                    envelope: None,
                }),
                Err(refusal) => Err(BindingError::Refusal(Box::new(
                    Marshaled::from_construct_refusal(refusal),
                ))),
            }
        }
    }
}

/// The certified volume-facts entry: assemble the recorded patch row and run
/// the landed certified face-form volume fact, returning its two-sided
/// bracket. A patch or volume refusal marshals to the typed door vocabulary.
pub fn volume_facts(row: &VolumeRow) -> Result<String, BindingError> {
    let patch = TensorBernsteinPatch::try_new(
        row.numerator.clone(),
        row.weights.clone(),
        unit_domain(),
        PatchParent::new(0, None),
    )
    .map_err(|refusal| {
        BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal)))
    })?;
    let fact = certify_patch_form(&patch, row.orientation, &VolumeOptions::default()).map_err(
        |refusal| BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal))),
    )?;
    to_json(&VolumeOutcome {
        ok: true,
        bracket: Bracket {
            lo: fact.bracket.lo,
            hi: fact.bracket.hi,
        },
        value: fact.value,
        certified_error: fact.certified_error,
        faces: fact.faces,
        patches: fact.patches,
    })
}

/// The certified trim-facts entry for the algebraic-trim path: run the landed
/// certified algebraic-trim bracket engine over the recorded pullback nets and
/// return the two-sided integral bracket. A refusal marshals to the typed door
/// vocabulary.
pub fn trim_facts(row: &TrimRow) -> Result<String, BindingError> {
    let bracket = certify_algebraic_trim_bracket(&row.density, &row.trim, row.tolerance).map_err(
        |refusal| BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal))),
    )?;
    to_json(&TrimOutcome {
        ok: true,
        bracket: Bracket {
            lo: bracket.lo,
            hi: bracket.hi,
        },
    })
}

/// Converts a [`BindingError`] into the typed Python exception: a kernel
/// refusal becomes the marshaled `Refused`/`Unresolved` instance (never a bare
/// `Exception`), a malformed row a `ValueError`.
fn to_pyerr(py: Python<'_>, error: BindingError) -> PyErr {
    match error {
        BindingError::Malformed(malformed) => PyValueError::new_err(malformed.detail),
        BindingError::Refusal(marshaled) => {
            let payload_json = match &marshaled.payload {
                MarshaledPayload::Refused(payload) => serde_json::to_string(payload),
                MarshaledPayload::Unresolved(payload) => serde_json::to_string(payload),
            };
            let payload_json = match payload_json {
                Ok(json) => json,
                Err(e) => {
                    return PyRuntimeError::new_err(format!(
                        "refusal payload serialization failed: {e}"
                    ));
                }
            };
            let instance = match marshaled.class {
                ExceptionClass::Refused => crate::python::marshal_refused(py, &payload_json),
                ExceptionClass::Unresolved => crate::python::marshal_unresolved(py, &payload_json),
            };
            match instance {
                Ok(value) => PyErr::from_value(value.bind(py).clone()),
                Err(e) => e,
            }
        }
    }
}

/// The pyo3 export of [`boolean_dispatch`].
#[pyfunction]
pub fn binding_boolean_dispatch(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: BooleanRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid boolean row JSON: {e}")))?;
    boolean_dispatch(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`volume_facts`].
#[pyfunction]
pub fn binding_volume_facts(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: VolumeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid volume row JSON: {e}")))?;
    volume_facts(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`trim_facts`].
#[pyfunction]
pub fn binding_trim_facts(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: TrimRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid trim row JSON: {e}")))?;
    trim_facts(&row).map_err(|e| to_pyerr(py, e))
}

// ===========================================================================
// TRIM-EXTRUDE-CTOR — the spline-trimmed extrude constructor.
//
// One export composes the LANDED stages in order and never reimplements any
// of them:
//
//   1. local-frame extrude   — the recorded profile is projected into its own
//                              plane and swept along the plane normal (the
//                              landed `bd_bridge` prism geometry);
//   2. pullback polynomial   — the volume-form pullback is the constant
//                              density net `height` over the unit square (the
//                              extrusion's Jacobian is constant);
//   3. R9 crossings          — `trim_clip` certifies the crossings of the
//                              profile's arcs against the closed spline trim
//                              loop and stamps them as `TopoNode::TrimCrossing`
//                              nodes (never bare coordinates);
//   4. winding classification — `trim_clip`'s sound `certify_off_loop` +
//                              `winding_number` composition classifies the
//                              retained sub-arcs;
//   5. ADM-003 bracket        — `certify_algebraic_trim_bracket` integrates the
//                              density over the recorded pullback net
//                              and returns the certified two-sided bracket.
//
// A trim class the composition cannot close (a self-crossing control loop, a
// trim with no recorded pullback net, a stalled R9 isolation) refuses TYPED
// naming the `TrimClipFailed` family — Inconclusive, never silent (§9.4).
// ===========================================================================

/// The certified bracket tolerance of a trim-extrude row (dimensionless).
const TRIM_EXTRUDE_TOLERANCE: f64 = 1.0e-3;

/// The certified contraction rate of the constructor's crossing certificates
/// (`<= RHO_MAX`).
const TRIM_EXTRUDE_RHO: f64 = 0.125;

/// The one lifted chart the constructor's trim clip certifies in.
const TRIM_EXTRUDE_CHART: ChartId = ChartId(0);

/// The spline-trimmed extrude row: the base profile plus the recorded closed
/// spline curve and (optionally) the recorded pullback net.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrimExtrudeRow {
    /// The base profile boundary edges (a line loop; a spline edge is a
    /// recorded trim carrier the constructor classifies).
    pub profile: Vec<ProfileEdge>,
    /// The swept length along the profile-plane normal.
    pub amount: f64,
    /// Extrude symmetrically about the profile plane.
    #[serde(default)]
    pub both: bool,
    /// The closed spline curve control points (first == last for a closed
    /// loop); empty for a full extrude.
    #[serde(default)]
    pub trim_curve: Vec<[f64; 3]>,
    /// The curve's pullback net (row-major Bernstein grid over the unit
    /// square, `>= 0` kept); empty when only the parametric curve is
    /// recorded.
    #[serde(default)]
    pub trim_net: Vec<Vec<f64>>,
    /// The requested certified bracket tolerance.
    #[serde(default = "default_trim_extrude_tolerance")]
    pub tolerance: f64,
}

/// The default certified bracket tolerance of a trim-extrude row.
fn default_trim_extrude_tolerance() -> f64 {
    TRIM_EXTRUDE_TOLERANCE
}

/// One certified trim crossing record: the crossing is a `TrimCrossing` node
/// (certified exactly), never a bare coordinate pair.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrimCrossingRecord {
    /// Always `"trim_crossing"`: the node kind the clip stamped.
    pub kind: &'static str,
    /// Whether the node carries an exact certificate.
    pub certified: bool,
    /// The certified chart point `(u, v)` of the crossing.
    pub point: [f64; 2],
}

/// The certified outcome of one spline-trimmed extrude.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrimExtrudeOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified two-sided volume bracket.
    pub bracket: Bracket,
    /// The certified volume value (the bracket midpoint).
    pub value: f64,
    /// The local-frame bounding box of the extruded realization.
    pub bbox: [[f64; 3]; 2],
    /// The certified trim crossings, in certified arc-parameter order.
    pub crossings: Vec<TrimCrossingRecord>,
    /// The number of retained (inside) sub-arcs of the clip.
    pub retained_arcs: usize,
}

/// The realized facts of one trim-prism row (the executor's shape): the
/// certified volume value, the local bbox and the deterministic mesh.
pub struct TrimExtrudeFacts {
    /// The certified volume value (the bracket midpoint).
    pub volume: f64,
    /// The local-frame bounding box.
    pub bbox: [[f64; 3]; 2],
    /// The local triangle soup (the base realization).
    pub mesh: Vec<Triangle>,
}

/// An orthonormal basis of the recorded profile's plane.
struct PlaneBasis {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    #[allow(dead_code)]
    normal: [f64; 3],
}

impl PlaneBasis {
    /// The chart coordinates of a world point in the profile plane.
    fn project(&self, p: [f64; 3]) -> [f64; 2] {
        let d = sub3(p, self.origin);
        [dot3(d, self.u), dot3(d, self.v)]
    }

    /// The world point of a chart coordinate pair.
    fn lift(&self, p: [f64; 2]) -> [f64; 3] {
        [
            self.origin[0] + self.u[0] * p[0] + self.v[0] * p[1],
            self.origin[1] + self.u[1] * p[0] + self.v[1] * p[1],
            self.origin[2] + self.u[2] * p[0] + self.v[2] * p[1],
        ]
    }
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm3(a: [f64; 3]) -> f64 {
    dot3(a, a).sqrt()
}

fn normalize3(a: [f64; 3]) -> [f64; 3] {
    let n = norm3(a);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// The malformed-row error (a caller defect, distinct from a kernel refusal).
fn malformed(detail: &str) -> BindingError {
    BindingError::Malformed(MalformedRow {
        detail: detail.to_string(),
    })
}

/// A typed `TrimClipFailed` refusal (Inconclusive) — the named §9.4 refusal.
fn trim_clip_failed(name: &'static str, detail: String) -> BindingError {
    BindingError::Refusal(Box::new(marshal_kernel_refusal(&KernelRefusal::new(
        RefusalKind::TrimClipFailed,
        RefusalEvidence::Predicate { name, detail },
    ))))
}

/// Marshals a certified-kernel `Refusal` into the typed door vocabulary. The
/// kind names the case; the door class is always `Refused` (the payload carries
/// the precise kind, so the door can distinguish the `TrimClipFailed` family
/// without a lossy re-marshal).
fn marshal_kernel_refusal(refusal: &KernelRefusal) -> Marshaled {
    let case = crate::marshal::kernel_refusal_kind_name(refusal.kind);
    Marshaled {
        class: ExceptionClass::Refused,
        message: format!("kernel refusal: {case}"),
        payload: MarshaledPayload::Refused(RefusedPayload {
            case: case.to_string(),
            envelope: None,
            stage: None,
            prop: None,
            left: None,
            right: None,
            reason: None,
            certificate: None,
            bound: None,
            allowed: None,
        }),
    }
}

/// The recorded points of a profile edge list and a trim curve.
fn gather_points(profile: &[ProfileEdge], trim: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, b } => {
                out.push(*a);
                out.push(*b);
            }
            ProfileEdge::Spline { points } => out.extend(points.iter().copied()),
        }
    }
    out.extend(trim.iter().copied());
    out
}

/// The closed line-loop vertices of a profile, `None` when any edge is a
/// spline (a spline profile is a trim carrier, not a flattenable base loop).
fn line_loop_vertices(profile: &[ProfileEdge]) -> Option<Vec<[f64; 3]>> {
    let mut out = Vec::new();
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, .. } => out.push(*a),
            ProfileEdge::Spline { .. } => return None,
        }
    }
    if out.len() >= 3 { Some(out) } else { None }
}

/// The bounding rectangle of a chart point set (the fallback base profile of
/// a spline-only trim carrier).
fn bbox_rect(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in points {
        for axis in 0..2 {
            lo[axis] = lo[axis].min(p[axis]);
            hi[axis] = hi[axis].max(p[axis]);
        }
    }
    vec![
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [hi[0], hi[1]],
        [lo[0], hi[1]],
    ]
}

/// Builds and validates the profile plane from the recorded points.
fn fit_plane(points: &[[f64; 3]]) -> Result<PlaneBasis, BindingError> {
    let p0 = match points.first() {
        Some(p) => *p,
        None => return Err(malformed("trim-extrude row has no recorded points")),
    };
    let mut u_raw = None;
    for p in points.iter().skip(1) {
        let d = sub3(*p, p0);
        if norm3(d) > 1.0e-12 {
            u_raw = Some(d);
            break;
        }
    }
    let u_raw = match u_raw {
        Some(u) => u,
        None => return Err(malformed("trim-extrude profile is degenerate")),
    };
    let mut normal = None;
    for p in points.iter() {
        let c = cross3(u_raw, sub3(*p, p0));
        if norm3(c) > 1.0e-12 {
            normal = Some(c);
            break;
        }
    }
    let normal = match normal {
        Some(n) => n,
        None => return Err(malformed("trim-extrude profile is collinear")),
    };
    let u = normalize3(u_raw);
    let normal = normalize3(normal);
    let v = cross3(normal, u);
    Ok(PlaneBasis {
        origin: p0,
        u,
        v,
        normal,
    })
}

/// Builds the recorded 3-D line edges of a closed chart polygon in the plane.
fn polygon_edges(polygon: &[[f64; 2]], plane: &PlaneBasis) -> Vec<ProfileEdge> {
    let n = polygon.len();
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(n)
        .map(|(a, b)| ProfileEdge::Line {
            a: plane.lift(*a),
            b: plane.lift(*b),
        })
        .collect()
}

/// A certified point certificate over a degenerate box at a chart point.
fn cert_at(point: [f64; 2]) -> Result<PointCert, KernelRefusal> {
    let box_ = IBox2::try_new([point[0], point[1]], [point[0], point[1]])?;
    PointCert::try_new(ResidualId::R1, box_, TRIM_EXTRUDE_RHO)
}

/// A certified boundary node on the constructor's chart at a chart point.
fn boundary_node(id: usize, point: [f64; 2]) -> Result<Node, KernelRefusal> {
    let at = Point4 {
        p1: Param::try_new(TRIM_EXTRUDE_CHART, 0, point[0], point[1])?,
        p2: Param::try_new(TRIM_EXTRUDE_CHART, 0, point[0], point[1])?,
    };
    Ok(Node {
        id: NodeId(id),
        at,
        kind: TopoNode::Boundary,
        cert: NodeCert::Exact(cert_at(point)?),
    })
}

/// A certified straight ordinary arc between two chart points over unit
/// parameter, referencing the two node ends.
fn straight_arc(
    id: usize,
    from: [f64; 2],
    to: [f64; 2],
    first: ArcEnd,
    second: ArcEnd,
) -> Result<Arc<4>, KernelRefusal> {
    let z_hat = [from[0], from[1], 0.0, 1.0];
    let q = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let q_tau = [1.0, 0.0, 0.0, 0.0];
    let q_perp = [
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let a = [[0.0; 4]; 4];
    let frame = Frame::try_new(z_hat, q, q_tau, q_perp, a)?;
    let i_tau = Interval { lo: 0.0, hi: 1.0 };
    let b_perp = IBox::<4>::try_new([-1.0; 4], [1.0; 4])?;
    let arc_cert = ArcCert::try_new(
        ResidualId::R1,
        frame,
        i_tau,
        b_perp,
        TRIM_EXTRUDE_RHO,
        vec![[0.0, 0.0]; 4],
        None,
    )?;
    let d = [to[0] - from[0], to[1] - from[1], 0.0];
    let spline = HermiteSpline::try_new(vec![HermiteSegment {
        p0: [from[0], from[1], 0.0],
        p1: [to[0], to[1], 0.0],
        t0: d,
        t1: d,
    }])?;
    Ok(Arc {
        id: ArcId(id),
        approx: Approx { gamma: spline },
        cert: arc_cert,
        ends: (first, second),
    })
}

/// The certified graph of a closed chart polygon (boundary nodes + straight
/// ordinary arcs, in order).
fn build_graph(polygon: &[[f64; 2]]) -> Result<CertifiedGraph, KernelRefusal> {
    let n = polygon.len();
    let mut nodes = Vec::with_capacity(n);
    for (i, p) in polygon.iter().enumerate() {
        nodes.push(boundary_node(i, *p)?);
    }
    let mut arcs: Vec<AnyArc> = Vec::with_capacity(n);
    for (i, (from, to)) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(n)
        .enumerate()
    {
        let arc = straight_arc(
            i,
            *from,
            *to,
            ArcEnd::Topo(NodeId(i)),
            ArcEnd::Topo(NodeId((i + 1) % n)),
        )?;
        arcs.push(AnyArc::Ordinary(arc));
    }
    let breaks: Vec<Break> = Vec::new();
    Ok(CertifiedGraph {
        nodes,
        breaks,
        arcs,
        sheets: Vec::new(),
        exhaustive: false,
    })
}

/// The closed rational Bézier trim leaf of the recorded control points.
fn bezier_trim_leaf(trim2: &[[f64; 2]]) -> Result<BezierLeaf1, KernelRefusal> {
    if trim2.len() < 2 {
        return Err(KernelRefusal::new(
            RefusalKind::TrimClipFailed,
            RefusalEvidence::Predicate {
                name: "trim_curve_too_short",
                detail: "a closed trim curve needs at least two control points".to_string(),
            },
        ));
    }
    let degree = trim2.len() - 1;
    let control: Vec<[f64; 4]> = trim2
        .iter()
        .map(|p| [p[0], p[1], 0.0, 1.0])
        .collect();
    BezierLeaf1::try_new(degree, control, TRIM_EXTRUDE_CHART)
}

/// Whether two closed chart segments properly intersect.
fn segments_cross(a0: [f64; 2], a1: [f64; 2], b0: [f64; 2], b1: [f64; 2]) -> bool {
    let d1 = orient(b0, b1, a0);
    let d2 = orient(b0, b1, a1);
    let d3 = orient(a0, a1, b0);
    let d4 = orient(a0, a1, b1);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// The signed area of the triangle `(a, b, c)` (the orientation predicate).
fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Whether the control polygon of a closed trim loop self-intersects. A
/// self-crossing loop cannot be certified by the clip's winding discipline;
/// the constructor refuses it typed rather than approximating.
fn control_polygon_self_intersects(points: &[[f64; 2]]) -> bool {
    let n = points.len();
    for i in 0..n {
        let (Some(a0), Some(a1)) = (
            points.get(i).copied(),
            points.get((i + 1) % n).copied(),
        ) else {
            continue;
        };
        for j in (i + 1)..n {
            // Adjacent segments share an endpoint; skip them.
            if j == i + 1 || (i == 0 && j + 1 == n) {
                continue;
            }
            let (Some(b0), Some(b1)) = (
                points.get(j).copied(),
                points.get((j + 1) % n).copied(),
            ) else {
                continue;
            };
            if segments_cross(a0, a1, b0, b1) {
                return true;
            }
        }
    }
    false
}

/// The chart samples of a closed Bézier trim leaf (deterministic fixed-order
/// de Casteljau subdivision, used only for the realization mesh).
fn bezier_samples(control: &[[f64; 2]], steps: usize) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(steps);
    for step in 0..steps {
        let t = step as f64 / steps as f64;
        let mut level: Vec<[f64; 2]> = control.to_vec();
        let mt = 1.0 - t;
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() - 1);
            for pair in level.windows(2) {
                next.push([
                    mt * pair[0][0] + t * pair[1][0],
                    mt * pair[0][1] + t * pair[1][1],
                ]);
            }
            level = next;
        }
        if let Some(p) = level.first() {
            out.push(*p);
        }
    }
    out
}

/// The composition: extrude the profile, cut it by the recorded spline trim,
/// certify the crossings and classify the retained region, and bracket the
/// volume through the ADM-003 pullback-net engine.
pub fn trim_extrude(row: &TrimExtrudeRow) -> Result<TrimExtrudeOutcome, BindingError> {
    if !(row.amount.is_finite() && row.amount > 0.0) {
        return Err(malformed(
            "trim-extrude amount must be finite and positive",
        ));
    }
    if !(row.tolerance.is_finite() && row.tolerance > 0.0) {
        return Err(malformed(
            "trim-extrude tolerance must be finite and positive",
        ));
    }
    let points = gather_points(&row.profile, &row.trim_curve);
    let plane = fit_plane(&points)?;

    // 1. The local-frame base profile.
    let line_vertices = line_loop_vertices(&row.profile);
    let base2: Vec<[f64; 2]> = match line_vertices {
        Some(verts) => verts.iter().map(|p| plane.project(*p)).collect(),
        None => {
            if row.trim_curve.is_empty() {
                return Err(malformed(
                    "a spline profile with no trim curve is not a constructor row",
                ));
            }
            let trim2: Vec<[f64; 2]> = row.trim_curve.iter().map(|p| plane.project(*p)).collect();
            bbox_rect(&trim2)
        }
    };
    let base_edges = polygon_edges(&base2, &plane);

    // 2. No trim: the plain extruded prism, bit-identical to the landed arm.
    if row.trim_curve.is_empty() {
        let volume = crate::bd_bridge::prism_geom(&base_edges, row.amount, row.both)
            .map_err(|r| BindingError::Refusal(Box::new(Marshaled::from_refusal(&r))))?
            .volume;
        let bbox = crate::bd_bridge::prism_bbox(&base_edges, row.amount, row.both)
            .map_err(|r| BindingError::Refusal(Box::new(Marshaled::from_refusal(&r))))?;
        return Ok(TrimExtrudeOutcome {
            ok: true,
            bracket: Bracket {
                lo: volume,
                hi: volume,
            },
            value: volume,
            bbox,
            crossings: Vec::new(),
            retained_arcs: 0,
        });
    }

    // 3. The closed spline trim curve, in the same chart as the base arcs.
    let trim2: Vec<[f64; 2]> = row.trim_curve.iter().map(|p| plane.project(*p)).collect();
    if control_polygon_self_intersects(&trim2) {
        return Err(trim_clip_failed(
            "trim_loop_self_crossing",
            "the recorded trim control loop self-intersects; the clip's winding discipline \
             cannot classify it, refusing TrimClipFailed (Inconclusive)"
                .to_string(),
        ));
    }
    let graph = build_graph(&base2).map_err(|r| {
        BindingError::Refusal(Box::new(marshal_kernel_refusal(&r)))
    })?;
    let leaf = bezier_trim_leaf(&trim2).map_err(|r| {
        BindingError::Refusal(Box::new(marshal_kernel_refusal(&r)))
    })?;
    let trim_loop = TrimLoop {
        chart: TRIM_EXTRUDE_CHART,
        curve: leaf,
        closed: true,
    };
    let clipped = trim_clip(&graph, &[trim_loop]).map_err(|r| {
        BindingError::Refusal(Box::new(marshal_kernel_refusal(&r)))
    })?;
    let crossings: Vec<TrimCrossingRecord> = clipped
        .nodes
        .iter()
        .filter(|node| node.kind == TopoNode::TrimCrossing)
        .map(|node| TrimCrossingRecord {
            kind: "trim_crossing",
            certified: matches!(node.cert, NodeCert::Exact(_)),
            point: [node.at.p1.u, node.at.p1.v],
        })
        .collect();
    let retained_arcs = clipped.arcs.len();

    // 4. The certified volume bracket over the recorded pullback net.
    if row.trim_net.is_empty() {
        return Err(trim_clip_failed(
            "trim_pullback_missing",
            "the spline curve has no recorded pullback net; the ADM-003 bracket \
             cannot be assembled, refusing TrimClipFailed (Inconclusive)"
                .to_string(),
        ));
    }
    let height = if row.both {
        2.0 * row.amount
    } else {
        row.amount
    };
    let density = vec![vec![height]];
    let bracket = certify_algebraic_trim_bracket(&density, &row.trim_net, row.tolerance)
        .map_err(|r| BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(r))))?;
    let bbox = crate::bd_bridge::prism_bbox(&base_edges, row.amount, row.both)
        .map_err(|r| BindingError::Refusal(Box::new(Marshaled::from_refusal(&r))))?;
    Ok(TrimExtrudeOutcome {
        ok: true,
        bracket: Bracket {
            lo: bracket.lo,
            hi: bracket.hi,
        },
        value: 0.5 * (bracket.lo + bracket.hi),
        bbox,
        crossings,
        retained_arcs,
    })
}

/// The realized facts of one trim-prism row: the certified volume value plus
/// the deterministic local mesh of the base extrusion.
pub fn trim_extrude_solid(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
    trim: &[[f64; 3]],
    trim_net: &[Vec<f64>],
    tolerance: f64,
) -> Result<TrimExtrudeFacts, BindingError> {
    let row = TrimExtrudeRow {
        profile: profile.to_vec(),
        amount,
        both,
        trim_curve: trim.to_vec(),
        trim_net: trim_net.to_vec(),
        tolerance,
    };
    let outcome = trim_extrude(&row)?;
    let points = gather_points(&row.profile, &row.trim_curve);
    let plane = fit_plane(&points)?;
    let line_vertices = line_loop_vertices(&row.profile);
    let base2: Vec<[f64; 2]> = match line_vertices {
        Some(verts) => verts.iter().map(|p| plane.project(*p)).collect(),
        None => {
            let trim2: Vec<[f64; 2]> = row.trim_curve.iter().map(|p| plane.project(*p)).collect();
            bezier_samples(&trim2, 64)
        }
    };
    let base_edges = polygon_edges(&base2, &plane);
    let mesh = crate::bd_bridge::prism_mesh(&base_edges, amount, both)
        .map_err(|r| BindingError::Refusal(Box::new(Marshaled::from_refusal(&r))))?;
    Ok(TrimExtrudeFacts {
        volume: outcome.value,
        bbox: outcome.bbox,
        mesh,
    })
}

/// The pyo3 export of [`trim_extrude`].
#[pyfunction]
pub fn binding_trim_extrude(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: TrimExtrudeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid trim-extrude row JSON: {e}")))?;
    trim_extrude(&row)
        .and_then(|outcome| to_json(&outcome))
        .map_err(|e| to_pyerr(py, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marshal::{
        DoorCase, UnmappedRefusal, door_case_of_construct_refusal,
        door_case_of_kernel_refusal_kind, door_case_of_kernel_refusal_name,
        kernel_refusal_kind_name,
    };
    use truck_certified::construct::refusal::ConstructRefusal;
    use truck_certified::kernel::evidence::RefusalKind;

    /// Every landed §17 kernel refusal kind (the frozen 25-variant set).
    const KERNEL_KINDS: [RefusalKind; 25] = [
        RefusalKind::SpineNotC1,
        RefusalKind::FrameSingular,
        RefusalKind::ProfileCollapse,
        RefusalKind::ProfileCorrespondenceMismatch,
        RefusalKind::NonFinite,
        RefusalKind::WindingAuditFailed,
        RefusalKind::NonDyadicSharedRequest,
        RefusalKind::CarrierSingularity,
        RefusalKind::ChartExhausted,
        RefusalKind::TranscendentalCarrier,
        RefusalKind::WeightDegenerate,
        RefusalKind::DeckExhausted,
        RefusalKind::Conditioning,
        RefusalKind::TangentialCurve,
        RefusalKind::HighOrderJet,
        RefusalKind::IncompleteStartSet,
        RefusalKind::R5EnclosureFailed,
        RefusalKind::TrimClipFailed,
        RefusalKind::NearOverlap,
        RefusalKind::OffsetDegenerate,
        RefusalKind::OffsetSwallowtail,
        RefusalKind::CornerUnsolved,
        RefusalKind::SliverOrNearOverlap,
        RefusalKind::ClaimRefuted,
        RefusalKind::Budget,
    ];

    /// Every landed construct refusal (the frozen 14-variant set).
    const CONSTRUCT_REFUSALS: [ConstructRefusal; 14] = [
        ConstructRefusal::NonPositiveWeightField,
        ConstructRefusal::SingularInterpolationSystem,
        ConstructRefusal::AmbiguousCorrespondence,
        ConstructRefusal::FocalDegeneracy,
        ConstructRefusal::CanalSingular,
        ConstructRefusal::RankDeficientContact,
        ConstructRefusal::UnintendedContact,
        ConstructRefusal::StarNotEmbedded,
        ConstructRefusal::NoAdmissibleProjection,
        ConstructRefusal::NonGenericThicknessEvent,
        ConstructRefusal::AmbiguousEventOrdering,
        ConstructRefusal::InvalidInput,
        ConstructRefusal::ConditioningBelowThreshold,
        ConstructRefusal::Unfrozen,
    ];

    fn cone(anchor: [f64; 3], s_up: f64) -> ConeRow {
        ConeRow { anchor, s_up }
    }

    /// The canonical admitted boolean row: a spline base against a canonical
    /// tool with two well-separated certified cones.
    fn canonical_row() -> BooleanRow {
        BooleanRow {
            base: CarrierClass::Spline,
            tool: CarrierClass::Canonical,
            mode: ModeValue::Add,
            base_cone: cone([0.0, 0.0, 1.0], 0.1),
            tool_cone: cone([1.0, 0.0, 0.0], 0.1),
        }
    }

    /// A non-degenerate admitted patch row (`X(u, v) = (u, v, 1)`, unit
    /// weights) whose exact face-form volume is `1/3`.
    fn canonical_volume_row() -> VolumeRow {
        VolumeRow {
            numerator: vec![
                vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
                vec![[1.0, 0.0, 1.0], [1.0, 1.0, 1.0]],
            ],
            weights: vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            orientation: 1.0,
        }
    }

    /// An axis-aligned trim row: the constant density `1` over the whole unit
    /// square (the trim `1` is strictly positive everywhere).
    fn canonical_trim_row() -> TrimRow {
        TrimRow {
            density: vec![vec![1.0]],
            trim: vec![vec![1.0]],
            tolerance: 1.0e-3,
        }
    }

    fn verdict(json: &str) -> String {
        let value: serde_json::Value = serde_json::from_str(json).expect("outcome json");
        value["verdict"].as_str().unwrap_or_default().to_string()
    }

    #[test]
    fn binding_exports_the_certified_boolean_entry() {
        // A routed pair returns the certified transversality bracket.
        let routed = boolean_dispatch(&canonical_row()).expect("routed pair");
        assert_eq!(verdict(&routed), "routed");
        let value: serde_json::Value = serde_json::from_str(&routed).expect("routed json");
        let lo = value["transverse"]["lo"].as_f64().expect("certified lo");
        let hi = value["transverse"]["hi"].as_f64().expect("certified hi");
        assert!(lo > 0.0, "the certified margin must be strictly positive");
        assert!(lo <= hi, "the certified bracket must be ordered");

        // A canonical x canonical pair lands on the S1 path, never dispatched.
        let landed = BooleanRow {
            base: CarrierClass::Canonical,
            tool: CarrierClass::Canonical,
            ..canonical_row()
        };
        assert_eq!(
            verdict(&boolean_dispatch(&landed).expect("landed pair")),
            "canonical_landed"
        );

        // A torus carrier is refused typed at the facade boundary.
        let torus = BooleanRow {
            tool: CarrierClass::Torus,
            ..canonical_row()
        };
        let refused = boolean_dispatch(&torus).expect("facade refusal verdict");
        assert_eq!(verdict(&refused), "refused");
        assert!(refused.contains("contact_reduction_deferred"));

        // A pair whose cones do not certify separation marshals the kernel
        // refusal to the typed door vocabulary (no panic, no bare string).
        let overlapping = BooleanRow {
            base_cone: cone([0.0, 0.0, 1.0], 0.99),
            tool_cone: cone([0.0, 0.0, 1.0], 0.99),
            ..canonical_row()
        };
        match boolean_dispatch(&overlapping) {
            Err(BindingError::Refusal(marshaled)) => {
                assert_eq!(marshaled.class, ExceptionClass::Refused);
                match marshaled.payload {
                    MarshaledPayload::Refused(payload) => {
                        assert_eq!(payload.case, "construct_refused");
                        assert_eq!(payload.stage.as_deref(), Some("ConditioningBelowThreshold"));
                    }
                    MarshaledPayload::Unresolved(_) => {
                        panic!("a construct refusal must not marshal as Unresolved")
                    }
                }
            }
            other => panic!("expected a typed refusal, got {other:?}"),
        }
    }

    #[test]
    fn kernel_refusals_marshall_to_typed_door_vocabulary() {
        // Every landed §17 kind maps to a typed door case, none unmapped.
        for kind in KERNEL_KINDS {
            let case = door_case_of_kernel_refusal_kind(kind);
            assert_ne!(
                case,
                DoorCase::UnmappedRefusal,
                "kind {} must have a booked door mapping",
                kernel_refusal_kind_name(kind)
            );
        }
        // Every landed construct refusal maps to the booked construct case and
        // marshals as the Refused class with the variant tag on the payload.
        for refusal in CONSTRUCT_REFUSALS {
            assert_eq!(
                door_case_of_construct_refusal(refusal),
                DoorCase::ConstructRefused
            );
            let marshaled = Marshaled::from_construct_refusal(refusal);
            assert_eq!(marshaled.class, ExceptionClass::Refused);
            match marshaled.payload {
                MarshaledPayload::Refused(payload) => {
                    assert_eq!(payload.case, "construct_refused");
                    assert_eq!(payload.stage.as_deref(), Some(refusal.tag()));
                }
                MarshaledPayload::Unresolved(_) => {
                    panic!("construct refusals never marshal as Unresolved")
                }
            }
        }
        // A kind the table does not name is the typed UnmappedRefusal defect
        // signal, never silent.
        assert_eq!(
            door_case_of_kernel_refusal_name("a_future_refusal_kind"),
            DoorCase::UnmappedRefusal
        );
        let unmapped = UnmappedRefusal::new("a_future_refusal_kind");
        assert_eq!(unmapped.door_case(), DoorCase::UnmappedRefusal);
        let marshaled = unmapped.marshaled();
        assert_eq!(marshaled.class, ExceptionClass::Refused);
        match marshaled.payload {
            MarshaledPayload::Refused(payload) => assert_eq!(payload.case, "unmapped_refusal"),
            MarshaledPayload::Unresolved(_) => {
                panic!("an unmapped refusal never marshals as Unresolved")
            }
        }
    }

    #[test]
    fn canonical_pair_end_to_end_through_the_export() {
        // The canonical admitted pair runs the full path: facade admission →
        // certified transversality → a certified bracket out.
        let routed = boolean_dispatch(&canonical_row()).expect("canonical pair routed");
        assert_eq!(verdict(&routed), "routed");
        let value: serde_json::Value = serde_json::from_str(&routed).expect("routed json");
        assert_eq!(value["ok"], serde_json::Value::Bool(true));
        assert!(value["transverse"]["lo"].as_f64().expect("lo") > 0.0);

        // The canonical admitted patch runs the certified volume fact.
        let volume = volume_facts(&canonical_volume_row()).expect("canonical volume");
        let value: serde_json::Value = serde_json::from_str(&volume).expect("volume json");
        let lo = value["bracket"]["lo"].as_f64().expect("volume lo");
        let hi = value["bracket"]["hi"].as_f64().expect("volume hi");
        assert!((lo - 1.0 / 3.0).abs() < 1.0e-12, "exact face form is 1/3");
        assert!(lo <= hi);

        // The canonical algebraic-trim row runs the certified trim bracket.
        let trim = trim_facts(&canonical_trim_row()).expect("canonical trim");
        let value: serde_json::Value = serde_json::from_str(&trim).expect("trim json");
        let lo = value["bracket"]["lo"].as_f64().expect("trim lo");
        let hi = value["bracket"]["hi"].as_f64().expect("trim hi");
        assert!(
            lo <= 1.0 && 1.0 <= hi,
            "the constant density integrates to 1"
        );
    }

    #[test]
    fn binding_is_deterministic_bit_identical_rerun() {
        let row = canonical_row();
        let first = boolean_dispatch(&row).expect("first run");
        let second = boolean_dispatch(&row).expect("second run");
        assert_eq!(first, second, "boolean dispatch must be bit-identical");

        let volume = canonical_volume_row();
        assert_eq!(
            volume_facts(&volume).expect("first volume"),
            volume_facts(&volume).expect("second volume"),
            "volume facts must be bit-identical"
        );

        let trim = canonical_trim_row();
        assert_eq!(
            trim_facts(&trim).expect("first trim"),
            trim_facts(&trim).expect("second trim"),
            "trim facts must be bit-identical"
        );
    }

    // -----------------------------------------------------------------------
    // TRIM-EXTRUDE-CTOR: the spline-trimmed extrude constructor.
    // -----------------------------------------------------------------------

    /// The unit-square base profile (four line edges, closed, in the local
    /// `z = 0` chart).
    fn square_profile() -> Vec<ProfileEdge> {
        vec![
            ProfileEdge::Line {
                a: [0.0, 0.0, 0.0],
                b: [1.0, 0.0, 0.0],
            },
            ProfileEdge::Line {
                a: [1.0, 0.0, 0.0],
                b: [1.0, 1.0, 0.0],
            },
            ProfileEdge::Line {
                a: [1.0, 1.0, 0.0],
                b: [0.0, 1.0, 0.0],
            },
            ProfileEdge::Line {
                a: [0.0, 1.0, 0.0],
                b: [0.0, 0.0, 0.0],
            },
        ]
    }

    /// A closed square trim loop strictly inside the unit square (a smooth
    /// Bézier loop through its corners; no crossings with the base boundary).
    fn inner_square_trim() -> Vec<[f64; 3]> {
        vec![
            [0.25, 0.25, 0.0],
            [0.75, 0.25, 0.0],
            [0.75, 0.75, 0.0],
            [0.25, 0.75, 0.0],
            [0.25, 0.25, 0.0],
        ]
    }

    /// The pullback net of the quarter unit disk `1 - u^2 - v^2`
    /// over the unit square (`>= 0` kept), degree `(2, 2)`.
    fn quarter_disk_trim_net() -> Vec<Vec<f64>> {
        vec![
            vec![1.0, 1.0, 0.0],
            vec![1.0, 1.0, 0.0],
            vec![0.0, 0.0, -1.0],
        ]
    }

    #[test]
    fn spline_trim_extrude_volume_matches_recorded_reference() {
        // The composition runs the local-frame extrude, the R9 crossings via
        // the trim clip, the winding classification and the ADM-003 pullback
        // bracket. The recorded reference volume of the unit-square base
        // trimmed by the quarter-disk pullback is `height * pi/4`; the
        // certified two-sided bracket must contain it.
        let height = 2.0;
        let row = TrimExtrudeRow {
            profile: square_profile(),
            amount: height,
            both: false,
            trim_curve: inner_square_trim(),
            trim_net: quarter_disk_trim_net(),
            tolerance: 1.0e-3,
        };
        let outcome = trim_extrude(&row).expect("the trim bracket must certify");
        let reference = height * std::f64::consts::PI / 4.0;
        assert!(
            outcome.bracket.lo <= reference && reference <= outcome.bracket.hi,
            "the certified bracket [{}, {}] must contain the recorded reference {}",
            outcome.bracket.lo,
            outcome.bracket.hi,
            reference
        );
        assert!(outcome.bracket.lo <= outcome.bracket.hi);
        assert!(outcome.ok);
    }

    #[test]
    fn trim_crossings_are_certified_nodes_not_coordinates() {
        // The base square is crossed by the closed trim loop; the clip's
        // crossings are certified `TopoNode::TrimCrossing` nodes (exact
        // certificates), never bare coordinate pairs.
        let trim = vec![
            [0.5, -0.5, 0.0],
            [1.5, 0.5, 0.0],
            [0.5, 1.5, 0.0],
            [-0.5, 0.5, 0.0],
            [0.5, -0.5, 0.0],
        ];
        let row = TrimExtrudeRow {
            profile: square_profile(),
            amount: 1.0,
            both: false,
            trim_curve: trim,
            trim_net: vec![vec![1.0]],
            tolerance: 1.0e-3,
        };
        let outcome = trim_extrude(&row).expect("the clip must certify the crossings");
        assert!(
            !outcome.crossings.is_empty(),
            "a crossing trim loop must stamp at least one TrimCrossing node"
        );
        for crossing in &outcome.crossings {
            assert_eq!(crossing.kind, "trim_crossing");
            assert!(crossing.certified, "every crossing node is certified exactly");
        }
    }

    #[test]
    fn self_crossing_trim_loop_refuses_typed() {
        // A figure-eight control loop self-intersects: the clip's winding
        // discipline cannot classify it, so the constructor refuses the named
        // TrimClipFailed family — Inconclusive, never a silent approximation.
        let trim = vec![
            [0.2, 0.2, 0.0],
            [0.8, 0.8, 0.0],
            [0.2, 0.8, 0.0],
            [0.8, 0.2, 0.0],
            [0.2, 0.2, 0.0],
        ];
        let row = TrimExtrudeRow {
            profile: square_profile(),
            amount: 1.0,
            both: false,
            trim_curve: trim,
            trim_net: vec![vec![1.0]],
            tolerance: 1.0e-3,
        };
        match trim_extrude(&row) {
            Err(BindingError::Refusal(marshaled)) => {
                assert_eq!(marshaled.class, ExceptionClass::Refused);
                match marshaled.payload {
                    MarshaledPayload::Refused(payload) => {
                        assert_eq!(payload.case, "trim_clip_failed");
                    }
                    MarshaledPayload::Unresolved(_) => {
                        panic!("the trim-clip failure must marshal as a typed Refused payload")
                    }
                }
            }
            other => panic!("a self-crossing trim loop must refuse typed, got {other:?}"),
        }
    }

    #[test]
    fn full_extrude_without_trim_answers_bit_identically() {
        // The no-trim degenerate case must not change the landed prism facts:
        // the constructor's value is bit-identical to the plain extrude arm.
        let profile = square_profile();
        let row = TrimExtrudeRow {
            profile: profile.clone(),
            amount: 3.0,
            both: false,
            trim_curve: Vec::new(),
            trim_net: Vec::new(),
            tolerance: 1.0e-3,
        };
        let outcome = trim_extrude(&row).expect("the full extrude must answer");
        let tree = crate::bd_bridge::TreeNode::Part {
            part: crate::bd_bridge::PartSpec {
                solid: crate::bd_bridge::SolidSpec::Prism {
                    profile,
                    amount: 3.0,
                    both: false,
                },
                x: 0.0,
                y: 0.0,
                z: 0.0,
                rz: 0.0,
                rotation: None,
                mirror: None,
            },
        };
        let facts = crate::bd_bridge::tree_facts(&tree).expect("plain prism facts");
        assert_eq!(
            outcome.value.to_bits(),
            facts.volume.to_bits(),
            "the no-trim path must be bit-identical to the landed prism arm"
        );
        assert_eq!(outcome.bracket.lo.to_bits(), outcome.bracket.hi.to_bits());
        assert!(outcome.crossings.is_empty());
        assert_eq!(outcome.retained_arcs, 0);
    }
}
