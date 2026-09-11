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

use truck_base::evidence::{EnvelopeCase, Refusal as BaseRefusal};
use truck_certified::construct::admission::{NormalCone, certify_transverse_pair};
use truck_certified::construct::patches::{PatchParent, TensorBernsteinPatch};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::setback::certified_profile_solve;
use truck_certified::construct::volume_facts::{
    VolumeOptions, certify_algebraic_trim_bracket, certify_patch_form,
};
use truck_certified::kernel::Interval;
use truck_certified::kernel::certs::{ArcCert, Frame, PointCert};
use truck_certified::kernel::evidence::{Refusal as KernelRefusal, RefusalEvidence, RefusalKind};
use truck_certified::kernel::graph::{
    AnyArc, Approx, Arc, ArcEnd, ArcId, Break, CertifiedGraph, ChartId, HermiteSegment,
    HermiteSpline, Node, NodeCert, NodeId, Param, Point4, TopoNode,
};
use truck_certified::kernel::patch::{IBox, IBox2};
use truck_certified::kernel::residual::ResidualId;
use truck_certified::kernel::residuals_r89::BezierLeaf1;
use truck_certified::kernel::trimclip::{TrimLoop, trim_clip};

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
// AUTHOR-EXT-FILLET-HALO — the fillet recording arm.
//
// The base solid's recorded carrier data plus the recorded edge selectors are
// dispatched into the landed CC-033 setback profile machinery
// ([`certified_profile_solve`]) — the narrowest certified surface of the
// landed blend program (`construct/blend.rs`, `blend_varradius.rs`,
// `setback.rs`) that consumes pure recorded data. The wider blend entries
// (`trace_blend_chain`, `edge_stratum`, `canal_regularity`) take an admitted
// `CertifiedSurfaceMap`/`CertifiedCurveMap`, which is constructed only through
// `admit_surface`/`admit_curve` over a `truck_geometry::BSplineSurface`; the
// loop-side crate cannot name that type without a new manifest edge, so the
// profile solve is the surface this packet exposes (the ONE delegated
// judgement, recorded in RESULT notes).
//
// The recorded edge selector is the base part's OWN edge reference (its two
// local-frame endpoints), exactly the vocabulary the landed shapeops fillet
// uses (`FilletSpec { a, b, radius }`). A selector the recorded base cannot
// resolve refuses typed naming the open carrier; a cross field the certified
// solve cannot reproduce refuses the kernel's own refusal vocabulary. Nothing
// here approximates and nothing invents an edge-selection solver: the box's
// twelve edges ARE the recorded vocabulary of a box.
// ===========================================================================

/// One recorded fillet edge selector: the base part's own edge reference,
/// recorded as the two local-frame endpoints of the edge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeSelectorRow {
    /// The local-frame start point of the selected edge.
    pub a: [f64; 3],
    /// The local-frame end point of the selected edge.
    pub b: [f64; 3],
}

/// The recorded base carrier a fillet row's edge selectors resolve against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FilletBaseRow {
    /// A box base: its twelve edges are the recorded edge vocabulary.
    Box {
        /// Extent along x.
        length: f64,
        /// Extent along y.
        width: f64,
        /// Extent along z.
        height: f64,
    },
}

/// The fillet row: the base solid's recorded carrier data plus the blend
/// radius and the recorded edge selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilletRow {
    /// The base solid's recorded carrier data.
    pub base: FilletBaseRow,
    /// The requested blend radius.
    pub radius: f64,
    /// The recorded edge selectors, in script order.
    pub edges: Vec<EdgeSelectorRow>,
}

/// The typed refusal of a fillet row: an invalid request, an edge the recorded
/// base cannot resolve, or a certified blend refusal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilletRefusal {
    /// The radius or edge list is outside the frozen rule.
    InvalidInput,
    /// The selector does not name an edge of the base's recorded vocabulary.
    UnresolvableEdge,
    /// The landed blend profile machinery refused the recorded cross field.
    Blend(ConstructRefusal),
}

/// The certified blend fact of one fillet row.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilletOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified blend radius.
    pub radius: f64,
    /// The number of resolved (certified) edges.
    pub edges: usize,
    /// The maximum certified L2 control width of the resolved edge cross
    /// fields (exactly zero for an exactly recovered constant field).
    pub max_profile_width: f64,
}

/// The twelve edges of a box, in a fixed deterministic order.
fn box_edge_vocabulary(length: f64, width: f64, height: f64) -> Vec<([f64; 3], [f64; 3])> {
    let hx = 0.5 * length;
    let hy = 0.5 * width;
    let hz = 0.5 * height;
    let mut edges = Vec::with_capacity(12);
    for sy in [-hy, hy] {
        for sz in [-hz, hz] {
            edges.push(([-hx, sy, sz], [hx, sy, sz]));
        }
    }
    for sx in [-hx, hx] {
        for sz in [-hz, hz] {
            edges.push(([sx, -hy, sz], [sx, hy, sz]));
        }
    }
    for sx in [-hx, hx] {
        for sy in [-hy, hy] {
            edges.push(([sx, sy, -hz], [sx, sy, hz]));
        }
    }
    edges
}

/// Whether two endpoint pairs name the same unordered segment, exactly.
fn same_edge(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> bool {
    (a == c && b == d) || (a == d && b == c)
}

/// The recorded cross-tangent field of one resolved box edge: the constant
/// outward bisector of the two adjacent face normals, scaled by the radius.
/// The two adjacent faces are read from the edge's own constant coordinates
/// (the base's recorded vocabulary, never an inferred selection).
fn edge_cross_field(
    base: [f64; 3],
    other: [f64; 3],
    radius: f64,
) -> Result<[[f64; 3]; 4], FilletRefusal> {
    let mut varying: Option<usize> = None;
    for (axis, (from, to)) in base.iter().zip(other.iter()).enumerate() {
        if from != to {
            if varying.is_some() {
                // A selector that is not axis-aligned is not a box edge.
                return Err(FilletRefusal::UnresolvableEdge);
            }
            varying = Some(axis);
        }
    }
    let varying = varying.ok_or(FilletRefusal::UnresolvableEdge)?;
    let mut bisector = [0.0_f64; 3];
    for (axis, (coordinate, component)) in base.iter().zip(bisector.iter_mut()).enumerate() {
        if axis == varying {
            continue;
        }
        *component = if *coordinate > 0.0 {
            1.0
        } else if *coordinate < 0.0 {
            -1.0
        } else {
            // A constant coordinate on the symmetry plane names no face.
            return Err(FilletRefusal::UnresolvableEdge);
        };
    }
    let mag =
        (bisector[0] * bisector[0] + bisector[1] * bisector[1] + bisector[2] * bisector[2]).sqrt();
    if !mag.is_finite() || mag <= 0.0 {
        return Err(FilletRefusal::UnresolvableEdge);
    }
    for component in bisector.iter_mut() {
        *component *= radius / mag;
    }
    Ok([bisector; 4])
}

/// The certified blend fact of one fillet row: resolve every recorded edge
/// selector against the base's recorded edge vocabulary, certify the edge's
/// cross field through the landed profile machinery, and return the certified
/// outcome. An unresolvable selector refuses [`FilletRefusal::UnresolvableEdge`];
/// a certified-solve refusal propagates the kernel's own vocabulary.
pub fn fillet_facts(row: &FilletRow) -> Result<FilletOutcome, FilletRefusal> {
    if !row.radius.is_finite() || row.radius <= 0.0 {
        return Err(FilletRefusal::InvalidInput);
    }
    if row.edges.is_empty() {
        return Err(FilletRefusal::InvalidInput);
    }
    let vocabulary = match &row.base {
        FilletBaseRow::Box {
            length,
            width,
            height,
        } => {
            if !length.is_finite()
                || !width.is_finite()
                || !height.is_finite()
                || *length <= 0.0
                || *width <= 0.0
                || *height <= 0.0
            {
                return Err(FilletRefusal::InvalidInput);
            }
            box_edge_vocabulary(*length, *width, *height)
        }
    };
    let mut max_profile_width = 0.0_f64;
    let mut resolved = 0usize;
    for selector in &row.edges {
        let matched = vocabulary
            .iter()
            .find(|(c, d)| same_edge(selector.a, selector.b, *c, *d));
        let Some((c, d)) = matched else {
            return Err(FilletRefusal::UnresolvableEdge);
        };
        let cross = edge_cross_field(*c, *d, row.radius)?;
        let width = certified_profile_solve(&cross).map_err(FilletRefusal::Blend)?;
        if width > max_profile_width {
            max_profile_width = width;
        }
        resolved = resolved.saturating_add(1);
    }
    Ok(FilletOutcome {
        ok: true,
        radius: row.radius,
        edges: resolved,
        max_profile_width,
    })
}

/// The pyo3 export of [`fillet_facts`]. A refusal marshals to the typed door
/// vocabulary (the `construct_refused` case for the blend, the
/// `non_canonical_carrier` envelope for an unresolvable selector), never a
/// bare string.
#[pyfunction]
pub fn binding_fillet(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: FilletRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid fillet row JSON: {e}")))?;
    match fillet_facts(&row) {
        Ok(outcome) => to_json(&outcome).map_err(|e| to_pyerr(py, e)),
        Err(FilletRefusal::InvalidInput) => Err(to_pyerr(
            py,
            BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(
                ConstructRefusal::InvalidInput,
            ))),
        )),
        Err(FilletRefusal::UnresolvableEdge) => Err(to_pyerr(
            py,
            BindingError::Refusal(Box::new(Marshaled::from_refusal(
                &BaseRefusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier),
            ))),
        )),
        Err(FilletRefusal::Blend(refusal)) => Err(to_pyerr(
            py,
            BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal))),
        )),
    }
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

/// The constructor's control-point envelope for the R9 clip. The clip's
/// subdivision is exponential in the leaf degree, so a corpus spline recorded
/// as one high-degree control loop refuses typed rather than running an
/// unbounded isolation.
const MAX_TRIM_CONTROL_POINTS: usize = 16;

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
            // A conic/arc section is not a point-list carrier; the trim-extrude
            // constructor's point fit does not consume it.
            ProfileEdge::Circle { .. }
            | ProfileEdge::Ellipse { .. }
            | ProfileEdge::Arc { .. } => {}
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
            ProfileEdge::Spline { .. }
            | ProfileEdge::Circle { .. }
            | ProfileEdge::Ellipse { .. }
            | ProfileEdge::Arc { .. } => return None,
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
        lo[0] = lo[0].min(p[0]);
        lo[1] = lo[1].min(p[1]);
        hi[0] = hi[0].max(p[0]);
        hi[1] = hi[1].max(p[1]);
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
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
fn cert_at(point: [f64; 2]) -> Result<PointCert, KernelRefusal> {
    let box_ = IBox2::try_new([point[0], point[1]], [point[0], point[1]])?;
    PointCert::try_new(ResidualId::R1, box_, TRIM_EXTRUDE_RHO)
}

/// A certified boundary node on the constructor's chart at a chart point.
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
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
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
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
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
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
// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
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
    let control: Vec<[f64; 4]> = trim2.iter().map(|p| [p[0], p[1], 0.0, 1.0]).collect();
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
        let (Some(a0), Some(a1)) = (points.get(i).copied(), points.get((i + 1) % n).copied())
        else {
            continue;
        };
        for j in (i + 1)..n {
            // Adjacent segments share an endpoint; skip them.
            if j == i + 1 || (i == 0 && j + 1 == n) {
                continue;
            }
            let (Some(b0), Some(b1)) = (points.get(j).copied(), points.get((j + 1) % n).copied())
            else {
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
                if let [a, b] = pair {
                    next.push([mt * a[0] + t * b[0], mt * a[1] + t * b[1]]);
                }
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
        return Err(malformed("trim-extrude amount must be finite and positive"));
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
    // The certified volume bracket is assembled over the recorded pullback net.
    // Without it the ADM-003 bracket cannot close, so the constructor refuses
    // TYPED before paying for the crossing isolation — never an unbounded
    // subdivision on a curve whose kept region is not recorded.
    if row.trim_net.is_empty() {
        return Err(trim_clip_failed(
            "trim_pullback_missing",
            "the spline curve has no recorded pullback net; the ADM-003 bracket \
             cannot be assembled, refusing TrimClipFailed (Inconclusive)"
                .to_string(),
        ));
    }
    // The R9 clip certifies low-degree chart leaves; a corpus spline recorded
    // as one high-degree control loop is outside that discipline and refuses
    // typed rather than exploding the subdivision.
    if trim2.len() > MAX_TRIM_CONTROL_POINTS {
        return Err(trim_clip_failed(
            "trim_degree_out_of_envelope",
            format!(
                "the trim control loop carries {} points, above the constructor's \
                 {MAX_TRIM_CONTROL_POINTS}-point envelope; refusing TrimClipFailed (Inconclusive)",
                trim2.len()
            ),
        ));
    }
    let graph = build_graph(&base2)
        .map_err(|r| BindingError::Refusal(Box::new(marshal_kernel_refusal(&r))))?;
    let leaf = bezier_trim_leaf(&trim2)
        .map_err(|r| BindingError::Refusal(Box::new(marshal_kernel_refusal(&r))))?;
    let trim_loop = TrimLoop {
        chart: TRIM_EXTRUDE_CHART,
        curve: leaf,
        closed: true,
    };
    let clipped = trim_clip(&graph, &[trim_loop])
        .map_err(|r| BindingError::Refusal(Box::new(marshal_kernel_refusal(&r))))?;
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

// ===========================================================================
// MONO-1-DATA-ROWS — kernel-native probe data rows.
//
// The corpus helpers (`corpus/ttc/trees/f1/src/lib/surfaces.py`) probe
// `shape.wrapped` for `bbox`, `obox`, `is_valid_shape` and face counts. On a
// kernel-engine row the drop-in's `wrapped` cannot serve an OCC probe, so the
// row instead carries the kernel's own certificates and these exports answer
// from them:
//
//   * `bbox` / `obox` — the rigorous carrier-derived box. Every boundary patch
//     is a rational tensor-Bernstein patch `X = Â/Ŵ` with certified positive
//     weights, so its affine control net `Â_ij / Ŵ_ij` bounds the patch by the
//     convex-hull property. A patch whose hull is looser than the fixture slack
//     is subdivided (exact geometry-preserving Bernstein de Casteljau) until
//     every leaf hull is within the slack; the union hull is the certified
//     bound. `obox` is the same bound carried into the row's recorded frame
//     (the `lo, hi` corner-pair form the corpus helper consumes).
//   * `is_valid_shape` — the landed closed/oriented 2-cycle invariant the row
//     recorded at construction. Never an OCC probe.
//   * `face_count` — the number of patches of the row's boundary grid.
//
// Every returned number is a certified bound, never a sampling; the dense
// sampling below appears only in the tests that assert the bound's tightness.
// ===========================================================================

/// The certified bbox slack target: a leaf is accepted once its affine control
/// hull is no wider than this (the packet's 1 mm fixture target).
const DATA_ROW_BBOX_SLACK: f64 = 1.0;

/// The subdivision depth cap of the certified bbox refinement. A patch whose
/// hull cannot reach the slack within the cap is still bounded by its (looser)
/// leaf hulls — the returned number is always a certified bound.
const DATA_ROW_BBOX_MAX_DEPTH: u32 = 8;

/// One rational tensor-Bernstein patch data row: the numerator grid `Â` and the
/// same-shape weight grid `Ŵ` (rows over `u`, columns over `v`), exactly the
/// shape the landed [`TensorBernsteinPatch`] consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchRow {
    /// The row-major `R³` numerator grid `Â`.
    pub numerator: Vec<Vec<[f64; 3]>>,
    /// The same-shape scalar weight grid `Ŵ`.
    pub weights: Vec<Vec<f64>>,
}

/// The recorded orthonormal frame of a placed shape row: origin plus the
/// rotation columns `(x_dir, y_dir, z_dir)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRow {
    /// The frame origin (the translation).
    pub origin: [f64; 3],
    /// The world image of the local `+x` axis.
    pub x_dir: [f64; 3],
    /// The world image of the local `+y` axis.
    pub y_dir: [f64; 3],
    /// The world image of the local `+z` axis.
    pub z_dir: [f64; 3],
}

/// A kernel-native shape data row: the boundary grid of rational patches plus
/// the constructive closed/oriented invariant recorded at construction. The
/// same row surface carries a `Face`, a `Shell` or a `Solid`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeRow {
    /// The boundary grid: one [`PatchRow`] per boundary face.
    pub patches: Vec<PatchRow>,
    /// The landed closed 2-cycle invariant (every boundary side is glued
    /// exactly once).
    pub closed: bool,
    /// The landed consistent-orientation invariant.
    pub oriented: bool,
    /// The recorded placement frame, when the row is placed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<FrameRow>,
}

/// The certified bound of one shape row: `[lo, hi]` corner points.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BboxOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The certified bound corner pair `[lo, hi]`.
    pub bbox: [[f64; 3]; 2],
    /// The number of refined control-net leaves the bound was assembled from.
    pub leaves: usize,
}

/// The constructive validity answer of one shape row.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidityOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The closed/oriented 2-cycle invariant the row recorded.
    pub valid: bool,
    /// The recorded closed invariant.
    pub closed: bool,
    /// The recorded oriented invariant.
    pub oriented: bool,
}

/// The face-count answer of one shape row: the number of patches of the
/// boundary grid.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaceCountOutcome {
    /// Always `true` on the `Ok` path.
    pub ok: bool,
    /// The number of boundary patches.
    pub n_faces: usize,
}

/// A homogeneous `[x, y, z, w]` control net over a patch's unit square.
#[derive(Debug, Clone)]
struct Net4 {
    rows: usize,
    cols: usize,
    data: Vec<[f64; 4]>,
}

impl Net4 {
    /// Validates a patch row through the landed refusing constructor and reads
    /// its homogeneous control net. A malformed grid (or a non-positive weight)
    /// marshals the construct refusal typed.
    fn from_row(row: &PatchRow) -> Result<Net4, BindingError> {
        let _validated = TensorBernsteinPatch::try_new(
            row.numerator.clone(),
            row.weights.clone(),
            unit_domain(),
            PatchParent::new(0, None),
        )
        .map_err(|refusal| {
            BindingError::Refusal(Box::new(Marshaled::from_construct_refusal(refusal)))
        })?;
        let rows = row.weights.len();
        let cols = row.weights.first().map_or(0, |weight_row| weight_row.len());
        let mut data = Vec::with_capacity(rows.saturating_mul(cols));
        for (num_row, weight_row) in row.numerator.iter().zip(row.weights.iter()) {
            for (a, w) in num_row.iter().zip(weight_row.iter()) {
                data.push([a[0], a[1], a[2], *w]);
            }
        }
        Ok(Net4 { rows, cols, data })
    }

    /// The homogeneous coefficient at `(i, j)`, when in range.
    fn get(&self, i: usize, j: usize) -> Option<[f64; 4]> {
        self.data
            .get(i.checked_mul(self.cols)?.checked_add(j)?)
            .copied()
    }

    /// Overwrites the homogeneous coefficient at `(i, j)`, when in range.
    fn set(&mut self, i: usize, j: usize, value: [f64; 4]) {
        if let Some(slot) = i
            .checked_mul(self.cols)
            .and_then(|base| base.checked_add(j))
            .and_then(|idx| self.data.get_mut(idx))
        {
            *slot = value;
        }
    }

    /// The affine control point `Â_ij / Ŵ_ij`, when in range and non-degenerate.
    fn affine(&self, i: usize, j: usize) -> Option<[f64; 3]> {
        let h = self.get(i, j)?;
        if h[3] == 0.0 {
            return None;
        }
        Some([h[0] / h[3], h[1] / h[3], h[2] / h[3]])
    }

    /// The net carried into a recorded frame (exact rigid motion applied to the
    /// homogeneous numerator: `Â ↦ R·Â + origin·Ŵ`, weights unchanged).
    fn transformed(&self, frame: &FrameRow) -> Net4 {
        let data = self
            .data
            .iter()
            .map(|h| {
                let w = h[3];
                let x = h[0] / w;
                let y = h[1] / w;
                let z = h[2] / w;
                let wx =
                    frame.origin[0] + frame.x_dir[0] * x + frame.y_dir[0] * y + frame.z_dir[0] * z;
                let wy =
                    frame.origin[1] + frame.x_dir[1] * x + frame.y_dir[1] * y + frame.z_dir[1] * z;
                let wz =
                    frame.origin[2] + frame.x_dir[2] * x + frame.y_dir[2] * y + frame.z_dir[2] * z;
                [wx * w, wy * w, wz * w, w]
            })
            .collect();
        Net4 {
            rows: self.rows,
            cols: self.cols,
            data,
        }
    }
}

/// The linear interpolation of two homogeneous coefficients.
fn lerp4(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// The exact Bernstein de Casteljau split of a homogeneous net at `t` along the
/// requested axis (geometry-preserving: the two leaves re-cover the patch).
fn split_net(net: &Net4, along_u: bool, t: f64) -> (Net4, Net4) {
    let lines = if along_u { net.cols } else { net.rows };
    let seq_len = if along_u { net.rows } else { net.cols };
    let mut lo = Net4 {
        rows: net.rows,
        cols: net.cols,
        data: vec![[0.0; 4]; net.data.len()],
    };
    let mut hi = lo.clone();
    if lines == 0 || seq_len == 0 {
        return (lo, hi);
    }
    for line in 0..lines {
        let mut level: Vec<[f64; 4]> = Vec::with_capacity(seq_len);
        for k in 0..seq_len {
            let (i, j) = if along_u { (k, line) } else { (line, k) };
            if let Some(h) = net.get(i, j) {
                level.push(h);
            }
        }
        if let (Some(first), Some(last)) = (level.first().copied(), level.last().copied()) {
            let (i0, j0) = if along_u { (0, line) } else { (line, 0) };
            lo.set(i0, j0, first);
            let last_index = seq_len.saturating_sub(1);
            let (i1, j1) = if along_u {
                (last_index, line)
            } else {
                (line, last_index)
            };
            hi.set(i1, j1, last);
        }
        for r in 1..seq_len {
            let mut next: Vec<[f64; 4]> = Vec::with_capacity(level.len().saturating_sub(1));
            for pair in level.windows(2) {
                if let [a, b] = pair {
                    next.push(lerp4(*a, *b, t));
                }
            }
            if let Some(v) = next.first().copied() {
                let (i, j) = if along_u { (r, line) } else { (line, r) };
                lo.set(i, j, v);
            }
            if let Some(v) = next.last().copied() {
                let target = seq_len.saturating_sub(1).saturating_sub(r);
                let (i, j) = if along_u {
                    (target, line)
                } else {
                    (line, target)
                };
                hi.set(i, j, v);
            }
            level = next;
        }
    }
    (lo, hi)
}

/// The coordinate-wise hull of a homogeneous net's affine control points.
fn net_affine_hull(net: &Net4) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for h in &net.data {
        if h[3] == 0.0 {
            continue;
        }
        let x = h[0] / h[3];
        let y = h[1] / h[3];
        let z = h[2] / h[3];
        lo[0] = lo[0].min(x);
        lo[1] = lo[1].min(y);
        lo[2] = lo[2].min(z);
        hi[0] = hi[0].max(x);
        hi[1] = hi[1].max(y);
        hi[2] = hi[2].max(z);
    }
    (lo, hi)
}

/// The coordinate-wise extent of a hull.
fn hull_span(lo: [f64; 3], hi: [f64; 3]) -> [f64; 3] {
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

/// The Euclidean diameter of a hull's extent (the bound's over-report).
fn span_diameter(span: [f64; 3]) -> f64 {
    (span[0] * span[0] + span[1] * span[1] + span[2] * span[2]).sqrt()
}

/// Merges a hull into an accumulator.
fn merge_hull(lo: &mut [f64; 3], hi: &mut [f64; 3], a: [f64; 3], b: [f64; 3]) {
    lo[0] = lo[0].min(a[0]);
    lo[1] = lo[1].min(a[1]);
    lo[2] = lo[2].min(a[2]);
    hi[0] = hi[0].max(b[0]);
    hi[1] = hi[1].max(b[1]);
    hi[2] = hi[2].max(b[2]);
}

/// The `u`-direction control-net extent (the split-direction heuristic).
fn net_extent_u(net: &Net4) -> f64 {
    if net.rows < 2 {
        return 0.0;
    }
    let mut extent = 0.0_f64;
    for j in 0..net.cols {
        if let (Some(a), Some(b)) = (net.affine(0, j), net.affine(net.rows - 1, j)) {
            extent = extent.max(norm3(sub3(b, a)));
        }
    }
    extent
}

/// The `v`-direction control-net extent (the split-direction heuristic).
fn net_extent_v(net: &Net4) -> f64 {
    if net.cols < 2 {
        return 0.0;
    }
    let mut extent = 0.0_f64;
    for i in 0..net.rows {
        if let (Some(a), Some(b)) = (net.affine(i, 0), net.affine(i, net.cols - 1)) {
            extent = extent.max(norm3(sub3(b, a)));
        }
    }
    extent
}

/// The rigorous bound of one patch: subdivide until every leaf's affine control
/// hull is within the slack (or the depth cap), and return the union hull.
fn patch_rigorous_bbox(net: Net4) -> ([f64; 3], [f64; 3], usize) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut leaves = 0usize;
    let mut stack: Vec<(Net4, u32)> = vec![(net, 0)];
    while let Some((current, depth)) = stack.pop() {
        let (leaf_lo, leaf_hi) = net_affine_hull(&current);
        let span = hull_span(leaf_lo, leaf_hi);
        if span_diameter(span) <= DATA_ROW_BBOX_SLACK || depth >= DATA_ROW_BBOX_MAX_DEPTH {
            merge_hull(&mut lo, &mut hi, leaf_lo, leaf_hi);
            leaves += 1;
        } else {
            let along_u = net_extent_u(&current) > net_extent_v(&current);
            let (a, b) = split_net(&current, along_u, 0.5);
            stack.push((a, depth + 1));
            stack.push((b, depth + 1));
        }
    }
    (lo, hi, leaves)
}

/// The rigorous bound of a whole shape row (optionally carried into its
/// recorded frame).
fn shape_bbox_bound(
    row: &ShapeRow,
    apply_frame: bool,
) -> Result<([f64; 3], [f64; 3], usize), BindingError> {
    if row.patches.is_empty() {
        return Err(malformed("a kernel shape row carries no boundary patches"));
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut leaves = 0usize;
    for patch in &row.patches {
        let net = Net4::from_row(patch)?;
        let net = match (apply_frame, row.frame) {
            (true, Some(frame)) => net.transformed(&frame),
            _ => net,
        };
        let (patch_lo, patch_hi, patch_leaves) = patch_rigorous_bbox(net);
        merge_hull(&mut lo, &mut hi, patch_lo, patch_hi);
        leaves += patch_leaves;
    }
    Ok((lo, hi, leaves))
}

/// The carrier-derived rigorous bound of a kernel shape row (the corpus `bbox`
/// probe surface): `[lo, hi]` corner points, every number a certified bound.
pub fn bbox_facts(row: &ShapeRow) -> Result<String, BindingError> {
    let (lo, hi, leaves) = shape_bbox_bound(row, false)?;
    to_json(&BboxOutcome {
        ok: true,
        bbox: [lo, hi],
        leaves,
    })
}

/// The corpus `obox` probe surface: the same rigorous bound carried into the
/// row's recorded frame (the `lo, hi` corner-point form the helper consumes).
pub fn obox_facts(row: &ShapeRow) -> Result<String, BindingError> {
    let (lo, hi, leaves) = shape_bbox_bound(row, true)?;
    to_json(&BboxOutcome {
        ok: true,
        bbox: [lo, hi],
        leaves,
    })
}

/// The corpus `is_valid_shape` probe surface: the landed closed/oriented
/// 2-cycle invariant the row recorded at construction. Never an OCC probe; a
/// malformed boundary grid marshals its construct refusal typed.
pub fn is_valid_shape(row: &ShapeRow) -> Result<String, BindingError> {
    for patch in &row.patches {
        let _validated = Net4::from_row(patch)?;
    }
    let valid = !row.patches.is_empty() && row.closed && row.oriented;
    to_json(&ValidityOutcome {
        ok: true,
        valid,
        closed: row.closed,
        oriented: row.oriented,
    })
}

/// The corpus face-count probe surface: the number of patches of the row's
/// boundary grid.
pub fn face_count(row: &ShapeRow) -> Result<String, BindingError> {
    to_json(&FaceCountOutcome {
        ok: true,
        n_faces: row.patches.len(),
    })
}

/// The pyo3 export of [`bbox_facts`].
#[pyfunction]
pub fn binding_bbox(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: ShapeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid shape row JSON: {e}")))?;
    bbox_facts(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`obox_facts`].
#[pyfunction]
pub fn binding_obox(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: ShapeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid shape row JSON: {e}")))?;
    obox_facts(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`is_valid_shape`].
#[pyfunction]
pub fn binding_is_valid_shape(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: ShapeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid shape row JSON: {e}")))?;
    is_valid_shape(&row).map_err(|e| to_pyerr(py, e))
}

/// The pyo3 export of [`face_count`].
#[pyfunction]
pub fn binding_face_count(py: Python<'_>, row_json: &str) -> PyResult<String> {
    let row: ShapeRow = serde_json::from_str(row_json)
        .map_err(|e| PyValueError::new_err(format!("invalid shape row JSON: {e}")))?;
    face_count(&row).map_err(|e| to_pyerr(py, e))
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
            assert!(
                crossing.certified,
                "every crossing node is certified exactly"
            );
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
                label: None,
                color: None,
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

    // -----------------------------------------------------------------------
    // MONO-1-DATA-ROWS: kernel-native probe data rows.
    // -----------------------------------------------------------------------

    /// The bilinear patch `X(u, v) = (2u, 3v, 5)` — a known control hull.
    fn flat_patch_row() -> PatchRow {
        PatchRow {
            numerator: vec![
                vec![[0.0, 0.0, 5.0], [0.0, 3.0, 5.0]],
                vec![[2.0, 0.0, 5.0], [2.0, 3.0, 5.0]],
            ],
            weights: vec![vec![1.0, 1.0], vec![1.0, 1.0]],
        }
    }

    /// The curved patch `X(u, v) = (u, v, 100·2u(1−u))`: its `z` control hull
    /// is `[0, 100]` while the surface's `z` range is `[0, 50]`.
    fn curved_patch_row() -> PatchRow {
        PatchRow {
            numerator: vec![
                vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                vec![[0.5, 0.0, 100.0], [0.5, 1.0, 100.0]],
                vec![[1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            ],
            weights: vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]],
        }
    }

    fn shape_row(patches: Vec<PatchRow>, closed: bool, oriented: bool) -> ShapeRow {
        ShapeRow {
            patches,
            closed,
            oriented,
            frame: None,
        }
    }

    /// Reads the `[lo, hi]` corner pair out of a `bbox`/`obox` outcome.
    fn bbox_corners(json: &str) -> ([f64; 3], [f64; 3]) {
        let value: serde_json::Value = serde_json::from_str(json).expect("bbox outcome json");
        let corners = value["bbox"].as_array().expect("bbox corner pair");
        let read = |corner: &serde_json::Value| -> [f64; 3] {
            let coords = corner.as_array().expect("corner");
            [
                coords[0].as_f64().expect("x"),
                coords[1].as_f64().expect("y"),
                coords[2].as_f64().expect("z"),
            ]
        };
        (read(&corners[0]), read(&corners[1]))
    }

    /// The binomial coefficient `C(n, k)` as an `f64`.
    fn binomial(n: usize, k: usize) -> f64 {
        let mut value = 1.0_f64;
        for i in 0..k {
            value = value * (n - i) as f64 / (i + 1) as f64;
        }
        value
    }

    /// The scalar Bernstein basis function `B_{i,n}(t)`.
    fn bernstein(i: usize, n: usize, t: f64) -> f64 {
        binomial(n, i) * t.powi(i as i32) * (1.0 - t).powi((n - i) as i32)
    }

    /// The exact rational patch point at `(u, v)` (a test-only dense sampler).
    fn patch_point(row: &PatchRow, u: f64, v: f64) -> [f64; 3] {
        let m = row.numerator.len() - 1;
        let n = row.numerator[0].len() - 1;
        let mut numerator = [0.0_f64; 3];
        let mut weight = 0.0_f64;
        for i in 0..=m {
            for j in 0..=n {
                let b = bernstein(i, m, u) * bernstein(j, n, v);
                let a = row.numerator[i][j];
                numerator[0] += b * a[0];
                numerator[1] += b * a[1];
                numerator[2] += b * a[2];
                weight += b * row.weights[i][j];
            }
        }
        [
            numerator[0] / weight,
            numerator[1] / weight,
            numerator[2] / weight,
        ]
    }

    #[test]
    fn bbox_is_control_hull_bound() {
        // The bilinear patch's affine control hull is exact, so the certified
        // bound must equal it: the bracket is the control-hull bound.
        let row = shape_row(vec![flat_patch_row()], true, true);
        let (lo, hi) = bbox_corners(&bbox_facts(&row).expect("bbox"));
        assert_eq!(lo, [0.0, 0.0, 5.0]);
        assert_eq!(hi, [2.0, 3.0, 5.0]);

        // The obox surface is the same bound (the row carries no frame).
        let (obox_lo, obox_hi) = bbox_corners(&obox_facts(&row).expect("obox"));
        assert_eq!(obox_lo, lo);
        assert_eq!(obox_hi, hi);
    }

    #[test]
    fn bbox_subdivides_to_slack() {
        // The raw control hull's z range is [0, 100] against a surface range of
        // [0, 50]; the subdivision must tighten the bound to within the 1 mm
        // slack of a dense sampling.
        let row = shape_row(vec![curved_patch_row()], true, true);
        let (lo, hi) = bbox_corners(&bbox_facts(&row).expect("bbox"));
        let samples = 128;
        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;
        for i in 0..=samples {
            for j in 0..=samples {
                let u = i as f64 / samples as f64;
                let v = j as f64 / samples as f64;
                let p = patch_point(&curved_patch_row(), u, v);
                z_min = z_min.min(p[2]);
                z_max = z_max.max(p[2]);
            }
        }
        assert!(
            lo[2] <= z_min + 1.0e-9,
            "the bound must enclose the surface"
        );
        assert!(
            hi[2] >= z_max - 1.0e-9,
            "the bound must enclose the surface"
        );
        assert!(
            z_min - lo[2] <= 1.0 + 1.0e-6,
            "the lower bound must be within the 1 mm slack"
        );
        assert!(
            hi[2] - z_max <= 1.0 + 1.0e-6,
            "the upper bound must be within the 1 mm slack"
        );
        assert!(
            hi[2] < 99.0,
            "subdivision must tighten the loose control hull (was 100)"
        );
    }

    #[test]
    fn validity_row_answers_constructive_invariant() {
        // The row answers the recorded closed/oriented 2-cycle invariant, not
        // an OCC probe.
        let valid = shape_row(vec![flat_patch_row()], true, true);
        let value: serde_json::Value =
            serde_json::from_str(&is_valid_shape(&valid).expect("valid")).expect("json");
        assert_eq!(value["valid"], serde_json::Value::Bool(true));
        assert_eq!(value["closed"], serde_json::Value::Bool(true));
        assert_eq!(value["oriented"], serde_json::Value::Bool(true));

        let open = shape_row(vec![flat_patch_row()], false, true);
        let value: serde_json::Value =
            serde_json::from_str(&is_valid_shape(&open).expect("open")).expect("json");
        assert_eq!(value["valid"], serde_json::Value::Bool(false));

        let unoriented = shape_row(vec![flat_patch_row()], true, false);
        let value: serde_json::Value =
            serde_json::from_str(&is_valid_shape(&unoriented).expect("unoriented")).expect("json");
        assert_eq!(value["valid"], serde_json::Value::Bool(false));
    }

    #[test]
    fn face_count_row_matches_grid() {
        let row = shape_row(
            vec![flat_patch_row(), curved_patch_row(), flat_patch_row()],
            true,
            true,
        );
        let value: serde_json::Value =
            serde_json::from_str(&face_count(&row).expect("face count")).expect("json");
        assert_eq!(value["n_faces"].as_u64(), Some(3));
    }

    #[test]
    fn data_row_refusals_stay_typed() {
        // A non-positive weight cannot certify the patch: the refusing
        // constructor marshals the typed construct-refusal door case.
        let bad = PatchRow {
            numerator: vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]],
            weights: vec![vec![1.0, -1.0]],
        };
        let row = shape_row(vec![bad], true, true);
        match bbox_facts(&row) {
            Err(BindingError::Refusal(marshaled)) => {
                assert_eq!(marshaled.class, ExceptionClass::Refused);
                match marshaled.payload {
                    MarshaledPayload::Refused(payload) => {
                        assert_eq!(payload.case, "construct_refused");
                    }
                    MarshaledPayload::Unresolved(_) => {
                        panic!("a construct refusal must marshal as the Refused class")
                    }
                }
            }
            other => panic!("a non-positive weight must refuse typed, got {other:?}"),
        }

        // An empty boundary grid is a caller defect: typed Malformed, never a
        // panic and never an approximation.
        let empty = shape_row(Vec::new(), true, true);
        assert!(matches!(
            bbox_facts(&empty),
            Err(BindingError::Malformed(_))
        ));
    }
}
