//! The drop-in executor binding (TTC-EXECUTOR-BINDING): deterministic
//! geometry behind the facade vocabulary.
//!
//! The corpus→kernel executor (docs/TT_MODEL_CODEPATH_AUDIT.md gap 1) lands
//! here: the facade's classifier (`run_facade`) is the pre-flight; this module
//! is the geometry executor that consumes the same *construction vocabulary*
//! and attaches the geometry facts the door measures. The corpus scripts run
//! unmodified on the kernel engine regime (`corpus/ttc/door.py --engine
//! truck`); the drop-in build123d-named Python surface that the door installs
//! records each census call as a data row and submits the row set here.
//!
//! Zero geometric content lives in Python (spec §5): every volume, bounding
//! box and triangle computed by this module is pure, deterministic, analytic
//! arithmetic over the submitted row set. The supported carrier forms are the
//! canonical S3/S6 constructions (box/cylinder/sphere/torus primitives and
//! the full 360° lathe over a closed `y = 0` profile — line edges exactly and
//! spline-profile edges integrated as the TRUE reconstructed interpolating
//! spline, never a flattening polygon; FH-SPLINE-LATHE). A name whose form is
//! outside this envelope (a partial arc, a swept/lofted carrier, a spline
//! profile with a non-recoverable interpolation) refuses with the typed
//! kernel refusal (`NonCanonicalCarrier`), which the pyo3 surface maps to the
//! landed `Refused`/`Unresolved` exception classes — loud, never a silent OCC
//! fallback.
//!
//! Determinism: the submitted row set is an ordered tree (the script's
//! construction log); identical rows produce identical facts and identical
//! STL bytes. The row order is preserved end to end; no hash ordering appears
//! in any output.
//!
//! H-1 applies (nothing here panics, unwraps, expects or indexes outside
//! tests).

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

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use truck_base::evidence::{EnvelopeCase, Refusal};

use crate::facade::{BooleanPairVerdict, CarrierClass, SweptBooleanEvent};
use crate::marshal::{ExceptionClass, Marshaled, MarshaledPayload};
use crate::python;

const TAU: f64 = std::f64::consts::TAU;

/// The census-recorded curve of one lathe profile edge, in the `y = 0`
/// profile plane. Profile coordinates are `(x, z)` with `x >= 0` the revolve
/// radius (so `y == 0`).
///
/// A spline edge records its DEFINING samples (the points the corpus passed
/// to `Edge.make_spline`, which is how a data-only surface can record the
/// carrier without any geometry computing in Python). The kernel reconstructs
/// the interpolating curve from the samples; it never sees a flattening
/// polygon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LatheEdge {
    /// A straight profile edge from `a` to `b`.
    Line { a: [f64; 2], b: [f64; 2] },
    /// A spline profile edge: the interpolation samples passed to
    /// `Edge.make_spline(points)` with no tangents/parameters. The OCC
    /// interpolation convention is recoverable from the sample list (chord
    /// length parameters, clamped cubic, endpoint tangents from the Lagrange
    /// derivative of the first/last four samples), so the recorded data is the
    /// carrier's defining data.
    Spline { points: Vec<[f64; 2]> },
}

/// The census-recorded curve of one profile edge of a general authoring arm
/// (extrude prism / loft section), in the part's local 3-D frame. A line edge
/// records its endpoints exactly; a spline edge records its DEFINING samples
/// (the points the corpus passed to `Edge.make_spline`). The recording scheme
/// never flattens a spline to a polygon: the carrier keeps the samples and the
/// arm either integrates the true curve or refuses typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileEdge {
    /// A straight profile edge from `a` to `b`.
    Line { a: [f64; 3], b: [f64; 3] },
    /// A spline profile edge: the interpolation samples passed to
    /// `Edge.make_spline(points)` with no tangents/parameters.
    Spline { points: Vec<[f64; 3]> },
}

/// The solid carrier of one construction row.
///
/// Data only: every field is a physical length/angle the corpus script
/// computed. The analytic arms below turn this into facts and triangles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SolidSpec {
    /// `Box(length, width, height)`, centered on the local origin.
    Box {
        /// Extent along x.
        length: f64,
        /// Extent along y.
        width: f64,
        /// Extent along z.
        height: f64,
    },
    /// `Cylinder(radius, height)`, centered on the local origin.
    Cylinder {
        /// The base radius.
        radius: f64,
        /// The height along the axis.
        height: f64,
        /// The cylinder axis (`"z"`, `"x"` or `"y"` — the corpus's Euler
        /// rotation applied to the default z axis).
        axis: String,
    },
    /// `Sphere(radius)`.
    Sphere {
        /// The sphere radius.
        radius: f64,
    },
    /// `Torus(major_radius, minor_radius)` on the local z axis.
    Torus {
        /// The major (ring) radius.
        major: f64,
        /// The minor (tube) radius.
        minor: f64,
    },
    /// `revolve(face, axis=z, revolution_arc=360)` over a closed profile
    /// lying in the `y = 0` plane. `profile` is the boundary in order: line
    /// edges and interpolating-spline edges (whose defining samples the edge
    /// records). Profile coordinates are `(x, z)` with `x >= 0` the revolve
    /// radius.
    Lathe {
        /// The closed `(x, z)` profile boundary edges, in order.
        profile: Vec<LatheEdge>,
        /// The swept arc in degrees. Only the full revolution is in envelope
        /// for this executor's analytic lathe arm.
        arc_deg: f64,
    },
    /// `extrude(face, amount, both)`: a closed planar line-loop profile swept
    /// along its own plane normal. `profile` is the recorded boundary in order
    /// (part-local 3-D coordinates). The extruded solid is an exact prism: the
    /// volume is the profile area times the swept length (`amount`, or `2 *
    /// amount` when `both`), the bbox is the profile support widened along the
    /// normal, and the mesh sweeps the boundary deterministically.
    Prism {
        /// The closed planar line-loop boundary edges, in order.
        profile: Vec<ProfileEdge>,
        /// The swept length along the profile-plane normal.
        amount: f64,
        /// Extrude on both sides of the profile plane (`both=True`: the total
        /// extent is `2 * amount`, symmetric about the plane).
        #[serde(default)]
        both: bool,
    },
    /// The spline-trimmed extrude constructor row (TRIM-EXTRUDE-CTOR): the
    /// base profile plus a recorded closed spline trim curve and (optionally)
    /// the trim's algebraic pullback net. The composition itself lives in the
    /// binding layer (`crate::python::binding::trim_extrude`); this arm is a
    /// pure pass-through so the door vocabulary reaches the constructor and
    /// the facts/mesh re-enter the normal executor path.
    TrimPrism {
        /// The base profile boundary edges (line loop; a spline edge is a
        /// recorded trim carrier the constructor classifies).
        profile: Vec<ProfileEdge>,
        /// The swept length along the profile-plane normal.
        amount: f64,
        /// Extrude symmetrically about the profile plane.
        #[serde(default)]
        both: bool,
        /// The closed spline curve control points (first == last for a closed
        /// loop); empty for a full extrude.
        #[serde(default)]
        trim_curve: Vec<[f64; 3]>,
        /// The curve's pullback net (row-major Bernstein grid over
        /// the unit square, `>= 0` kept); empty when only the parametric curve
        /// is recorded.
        #[serde(default)]
        trim_net: Vec<Vec<f64>>,
        /// The requested certified bracket tolerance.
        #[serde(default = "default_trim_tolerance")]
        tolerance: f64,
    },
    /// `loft(sections)` (and, for a closed station list, the sweep-as-loft-chain
    /// form): an ordered stack of section profiles, each a closed planar
    /// line-loop in the part's local 3-D frame, interpolated by matching
    /// vertices in order (the ruled carrier the recording scheme fixes).
    ///
    /// When `closed` is true the row is a halo-style loop: the last station is
    /// the exact return to the first station and the loft surface closes on
    /// itself with no end caps. The arm certifies the closure seam with the
    /// exact aligned-meeting-edge identity (`A0 W1 - A1 W0 == 0`, weights 1 for
    /// the recorded line carrier); a station list that does not close refuses
    /// typed with the mismatch evidence.
    Loft {
        /// The section profiles, in station order.
        sections: Vec<Vec<ProfileEdge>>,
        /// Whether the station list is a closed halo loop (last station == the
        /// return to the first station).
        #[serde(default)]
        closed: bool,
    },
}

/// The default certified bracket tolerance of a trim-prism row (the door's
/// recorded `1e-3`; dimensionless, per ADM-003's trim tolerance scale).
fn default_trim_tolerance() -> f64 {
    1.0e-3
}

/// The recorded full orthonormal placement rotation of one part (the frame
/// generalization of the pure-z rotation). The columns are the world
/// images of the part-local x, y and z axes after the placement rotation: a
/// local point `p` maps to `x_dir * p.x + y_dir * p.y + z_dir * p.z`. The
/// client records an orthonormal frame (fixed-order Gram-Schmidt); volume is
/// invariant under `R` and only the world bbox and mesh re-derive from it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationFrame {
    /// The world image of the part-local x axis.
    pub x_dir: [f64; 3],
    /// The world image of the part-local y axis.
    pub y_dir: [f64; 3],
    /// The world image of the part-local z axis.
    pub z_dir: [f64; 3],
}

impl RotationFrame {
    /// Applies the recorded orthonormal rotation to a local point.
    fn apply(self, p: [f64; 3]) -> [f64; 3] {
        let [x0, x1, x2] = self.x_dir;
        let [y0, y1, y2] = self.y_dir;
        let [z0, z1, z2] = self.z_dir;
        [
            x0 * p[0] + y0 * p[1] + z0 * p[2],
            x1 * p[0] + y1 * p[1] + z1 * p[2],
            x2 * p[0] + y2 * p[1] + z2 * p[2],
        ]
    }
}

/// One placed construction row: a solid plus its world frame.
///
/// The corpus places parts by translation (`.moved`/`.locate`) and, for the
/// revolve helpers, by rotation about the local z axis before translation
/// (`.rotate(Axis.Z, ...)`). The client layer records the frame as data; the
/// geometry arms apply it here, never in Python.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartSpec {
    /// The solid carrier.
    pub solid: SolidSpec,
    /// World x translation.
    pub x: f64,
    /// World y translation.
    pub y: f64,
    /// World z translation.
    pub z: f64,
    /// Rotation about the local z axis, in degrees (applied before the
    /// translation).
    #[serde(default)]
    pub rz: f64,
    /// The full orthonormal placement rotation recorded from an authoring
    /// frame (a `Plane`/`Pos`/`Rotation` frame row). When present it replaces
    /// the pure-z rotation exactly (`world = translate(o) ∘ R ∘ M`); a row
    /// without it keeps the pure-z rotation path byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<RotationFrame>,
    /// A placed-carrier reflection about a coordinate plane through the local
    /// origin, recorded by the mirror arm: `"x"` (YZ plane, x -> -x), `"y"`
    /// (XZ plane, y -> -y) or `"z"` (XY plane, z -> -z). The reflection is a
    /// congruence: it applies to the local geometry before the rz rotation and
    /// translation, facts transform with the placement, and no geometry is
    /// recomputed.
    #[serde(default)]
    pub mirror: Option<String>,
}

/// One recorded boolean row: the mode and the two placed operand nodes.
///
/// The operands' LOCAL carrier classes dispatch (a placement never reaches the
/// dispatch), and the routed event is recorded on the measured facts. A
/// boolean of a boolean result is the recorded open composition cell and
/// refuses typed (depth-1 only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanNode {
    /// The boolean mode (`union` / `subtract` / `intersect`).
    pub mode: crate::facade::ModeValue,
    /// The base operand node.
    pub a: Box<TreeNode>,
    /// The tool operand node.
    pub b: Box<TreeNode>,
}

/// A node of the submitted construction tree: either one placed solid (a
/// part), a group (a compound) of child nodes, or one recorded boolean row, in
/// script order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TreeNode {
    /// One placed solid.
    Part {
        /// The placed solid.
        part: PartSpec,
    },
    /// A compound/group of child rows.
    Group {
        /// The child rows, in script order.
        group: Vec<TreeNode>,
    },
    /// One recorded boolean row over two operand nodes.
    Boolean {
        /// The recorded boolean row.
        boolean: BooleanNode,
    },
}

/// The measured facts of a submitted construction tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// The solid count (every part anywhere in the tree).
    pub solid_count: u64,
    /// The volume of the measured top node, following the recorded OCC
    /// compound semantics the reference was measured with: a group's volume
    /// is the sum over its *immediate child parts only* (nested groups
    /// contribute nothing); a part's volume is its own.
    pub volume: f64,
    /// The axis-aligned bounding box of every part in the tree, `[min, max]`.
    pub bbox: [[f64; 3]; 2],
    /// The measured seam mismatch of a single-loft tree, when the top node is
    /// one closed halo loft row: the exact aligned-meeting-edge identity
    /// (`A0 W1 - A1 W0 == 0`) holds when this is `0.0`. `None` when the tree is
    /// not a single closed loft part (no seam to certify).
    pub seam_mismatch: Option<f64>,
    /// The routed boolean events of the tree, in script order: one routed
    /// row per admitted swept-carrier boolean pair. A canonical x canonical
    /// pair lands on the canonical path and records no event.
    pub boolean_events: Vec<SweptBooleanEvent>,
}

// ---------------------------------------------------------------------------
// Facts: analytic volume and bounding box
// ---------------------------------------------------------------------------

/// The analytic volume of one unplaced solid.
fn solid_volume(solid: &SolidSpec) -> Result<f64, Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => {
            let (length, width, height) = (*length, *width, *height);
            Ok(length * width * height)
        }
        SolidSpec::Cylinder { radius, height, .. } => {
            let (radius, height) = (*radius, *height);
            Ok(std::f64::consts::PI * radius * radius * height)
        }
        SolidSpec::Sphere { radius } => {
            let radius = *radius;
            Ok(4.0 / 3.0 * std::f64::consts::PI * radius.powi(3))
        }
        SolidSpec::Torus { major, minor } => {
            let (major, minor) = (*major, *minor);
            Ok(2.0 * std::f64::consts::PI * std::f64::consts::PI * major * minor * minor)
        }
        SolidSpec::Lathe { profile, arc_deg } => {
            if *arc_deg != 360.0 {
                // The partial-arc form is a certified facade op (PB-014), but
                // this executor's analytic lathe arm covers the full
                // revolution; a partial arc refuses typed, never approximates.
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
            if let Some(points) = line_profile_vertices(profile) {
                // The line-profile arm: the frustum telescoping of the closed
                // vertex loop, bit-identical to the landed line-profile facts.
                lathe_volume(&points)
            } else {
                lathe_profile_volume(profile)
            }
        }
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => {
            let geom = prism_geom(profile, *amount, *both)?;
            Ok(geom.volume)
        }
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => crate::python::binding::trim_extrude_solid(
            profile, *amount, *both, trim_curve, trim_net, *tolerance,
        )
        .map(|facts| facts.volume)
        .map_err(trim_binding_error_to_refusal),
        SolidSpec::Loft { sections, closed } => {
            // The landed analytic ruled arm stays the fast path for the class
            // it already certifies (V5: bit-identical). A spline-section
            // carrier falls to the certified smooth arm; a closed halo chain
            // has no smooth certificate.
            if let Ok(validated) = loft_sections(sections) {
                return loft_volume(&validated, *closed);
            }
            if *closed {
                return Err(open_smooth_loft());
            }
            certified_spline_loft_volume(sections).map(|(value, _, _)| value)
        }
    }
}

/// The closed `(x, z)` vertex loop of an all-line profile, in boundary order.
/// `None` when any profile edge is a spline (the spline arm then applies).
fn line_profile_vertices(profile: &[LatheEdge]) -> Option<Vec<[f64; 2]>> {
    let mut vertices = Vec::with_capacity(profile.len());
    for edge in profile {
        match edge {
            LatheEdge::Line { a, .. } => vertices.push(*a),
            LatheEdge::Spline { .. } => return None,
        }
    }
    Some(vertices)
}

/// The volume enclosed by revolving a closed `(x, z)` polygon profile about
/// the z axis. Exact for a simple polygon lying at `x >= 0`: each boundary
/// edge sweeps a conical frustum and the signed sum telescopes to the
/// enclosed volume (the absolute value absorbs the profile winding direction,
/// which is not part of the row data).
fn lathe_volume(points: &[[f64; 2]]) -> Result<f64, Refusal> {
    if points.len() < 3 {
        return Err(Refusal::Empty);
    }
    let mut sum = 0.0;
    let mut previous: Option<&[f64; 2]> = None;
    for point in points.iter().chain(points.first().into_iter()) {
        if let Some(prev) = previous {
            let (x0, z0) = (prev[0], prev[1]);
            let (x1, z1) = (point[0], point[1]);
            if x0 < 0.0
                || x1 < 0.0
                || !x0.is_finite()
                || !z0.is_finite()
                || !x1.is_finite()
                || !z1.is_finite()
            {
                return Err(Refusal::Empty);
            }
            sum += std::f64::consts::PI / 3.0 * (z1 - z0) * (x0 * x0 + x0 * x1 + x1 * x1);
        }
        previous = Some(point);
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum.abs())
}

// ---------------------------------------------------------------------------
// Spline-profile lathe facts
// ---------------------------------------------------------------------------
//
// A spline profile edge records the samples the corpus passed to
// `Edge.make_spline(points)`. The kernel reconstructs the interpolating curve
// the OCC reference revolved — chord-length parameters, a clamped cubic with
// a knot at every sample, C2 at the interior samples, and endpoint tangents
// equal to the derivative of the degree-3 Lagrange interpolant of the first
// (resp. last) four samples. This is exactly the curve `GeomAPI_Interpolate`
// builds for a non-periodic point list with no tangents (the convention
// `make_spline(points)` fixes); it is recoverable from the samples, so the
// arm integrates the TRUE spline and never a flattening polygon.
//
// Facts are exact polynomial arithmetic: over one span, `r` and `z` are
// cubic polynomials in the span parameter, and the segment's solid-of-
// revolution volume `pi * int r(u)^2 * z'(u) du` is a degree-8 polynomial
// integral evaluated in closed form (never sampled). The line-profile
// frustum telescoping in `lathe_volume` is the degree-1 special case and is
// preserved bit-identical for all-line profiles (see `solid_volume`).

/// One reconstructed spline span, in power basis on `u in [0, 1]`.
#[derive(Debug, Clone, Copy)]
struct SpanPoly {
    /// Radius `r(u)`.
    r: [f64; 4],
    /// Axial coordinate `z(u)`.
    z: [f64; 4],
}

/// Whether every profile edge is a valid finite carrier at `x >= 0`.
fn check_profile_edges(profile: &[LatheEdge]) -> Result<(), Refusal> {
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                let (ax, az) = (a[0], a[1]);
                let (bx, bz) = (b[0], b[1]);
                if ax < 0.0
                    || bx < 0.0
                    || !ax.is_finite()
                    || !az.is_finite()
                    || !bx.is_finite()
                    || !bz.is_finite()
                {
                    return Err(Refusal::Empty);
                }
            }
            LatheEdge::Spline { points } => {
                if points.len() < 2 {
                    return Err(Refusal::Empty);
                }
                for p in points {
                    let (x, z) = (p[0], p[1]);
                    if x < 0.0 || !x.is_finite() || !z.is_finite() {
                        return Err(Refusal::Empty);
                    }
                }
            }
        }
    }
    if profile.len() < 3 {
        return Err(Refusal::Empty);
    }
    Ok(())
}

/// The volume of a spline-bearing lathe profile: the exact segment-moment sum
/// over the profile boundary (spline spans integrated by closed-form
/// polynomial arithmetic, line edges by the frustum formula), absolute value
/// absorbing the winding.
fn lathe_profile_volume(profile: &[LatheEdge]) -> Result<f64, Refusal> {
    check_profile_edges(profile)?;
    let mut sum = 0.0;
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                let (x0, z0) = (a[0], a[1]);
                let (x1, z1) = (b[0], b[1]);
                sum += std::f64::consts::PI / 3.0 * (z1 - z0) * (x0 * x0 + x0 * x1 + x1 * x1);
            }
            LatheEdge::Spline { points } => {
                sum += spline_edge_volume(points)?;
            }
        }
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum.abs())
}

/// The signed volume contribution of one spline profile edge: the exact
/// segment-moment integral `pi * int r(u)^2 z'(u) du` over the whole edge.
fn spline_edge_volume(points: &[[f64; 2]]) -> Result<f64, Refusal> {
    let spans = spline_spans(points)?;
    let mut sum = 0.0;
    for span in &spans {
        sum += span_volume(span);
    }
    if !sum.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(sum)
}

/// The exact `pi * int_0^1 r(u)^2 z'(u) du` of one span. `r(u)` is a cubic
/// (degree 6 when squared), `z'(u)` a quadratic; the product is degree 8 and
/// integrates term-by-term (`int u^k = 1/(k+1)`) — no quadrature, no
/// sampling.
fn span_volume(span: &SpanPoly) -> f64 {
    // r(u)^2 in the power basis (degree 6).
    let mut r2 = [0.0f64; 7];
    for (i, ri) in span.r.iter().enumerate() {
        for (j, rj) in span.r.iter().enumerate() {
            r2[i + j] += ri * rj;
        }
    }
    // z'(u) in the power basis (degree 2).
    let dz = [span.z[1], 2.0 * span.z[2], 3.0 * span.z[3]];
    let mut integral = 0.0;
    for (k, ck) in r2.iter().enumerate() {
        for (l, dl) in dz.iter().enumerate() {
            integral += ck * dl / (k + l + 1) as f64;
        }
    }
    std::f64::consts::PI * integral
}

/// The reconstructed spans of the OCC interpolating curve through the samples.
fn spline_spans(points: &[[f64; 2]]) -> Result<Vec<SpanPoly>, Refusal> {
    let n = points.len();
    if n < 2 {
        return Err(Refusal::Empty);
    }
    for p in points {
        let (x, z) = (p[0], p[1]);
        if x < 0.0 || !x.is_finite() || !z.is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let params = chord_params(points);
    if n == 2 {
        // A two-sample spline interpolates the chord itself.
        let (x0, z0) = (points[0][0], points[0][1]);
        let (x1, z1) = (points[1][0], points[1][1]);
        return Ok(vec![SpanPoly {
            r: [x0, x1 - x0, 0.0, 0.0],
            z: [z0, z1 - z0, 0.0, 0.0],
        }]);
    }
    if n == 3 {
        // A three-sample spline is the quadratic through the three points.
        let t0 = params[0];
        let t2 = params[2];
        let u1 = (params[1] - t0) / (t2 - t0);
        let (x0, z0) = (points[0][0], points[0][1]);
        let (x2, z2) = (points[2][0], points[2][1]);
        let (x1, z1) = (points[1][0], points[1][1]);
        let denom = 2.0 * u1 * (1.0 - u1);
        let qx1 = (x1 - (1.0 - u1) * (1.0 - u1) * x0 - u1 * u1 * x2) / denom;
        let qz1 = (z1 - (1.0 - u1) * (1.0 - u1) * z0 - u1 * u1 * z2) / denom;
        // Quadratic Bezier (x0, qx1, x2) in the power basis over u in [0, 1].
        let bx = [x0, 2.0 * (qx1 - x0), x0 - 2.0 * qx1 + x2, 0.0];
        let bz = [z0, 2.0 * (qz1 - z0), z0 - 2.0 * qz1 + z2, 0.0];
        return Ok(vec![SpanPoly { r: bx, z: bz }]);
    }
    let slopes = clamped_slopes(points, &params)?;
    let mut spans = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let h = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        let (x0, z0) = {
            let p = points.get(i).ok_or(Refusal::Empty)?;
            (p[0], p[1])
        };
        let (x1, z1) = {
            let p = points.get(i + 1).ok_or(Refusal::Empty)?;
            (p[0], p[1])
        };
        let (s0x, s0z) = {
            let s = slopes.get(i).ok_or(Refusal::Empty)?;
            (s[0], s[1])
        };
        let (s1x, s1z) = {
            let s = slopes.get(i + 1).ok_or(Refusal::Empty)?;
            (s[0], s[1])
        };
        let span = SpanPoly {
            r: hermite_power(x0, x1, h * s0x, h * s1x),
            z: hermite_power(z0, z1, h * s0z, h * s1z),
        };
        spans.push(span);
    }
    Ok(spans)
}

/// The chord-length parameters of the samples.
fn chord_params(points: &[[f64; 2]]) -> Vec<f64> {
    let mut params = Vec::with_capacity(points.len());
    params.push(0.0);
    for pair in points.windows(2) {
        let (x0, z0) = (pair[0][0], pair[0][1]);
        let (x1, z1) = (pair[1][0], pair[1][1]);
        let dx = x1 - x0;
        let dz = z1 - z0;
        let next = params.last().copied().unwrap_or(0.0) + (dx * dx + dz * dz).sqrt();
        params.push(next);
    }
    params
}

/// The power-basis coefficients of the cubic Hermite interpolant over
/// `u in [0, 1]` with `C(0)=y0`, `C'(0)=d0`, `C(1)=y1`, `C'(1)=d1`.
fn hermite_power(y0: f64, y1: f64, d0: f64, d1: f64) -> [f64; 4] {
    [
        y0,
        d0,
        -3.0 * y0 - 2.0 * d0 + 3.0 * y1 - d1,
        2.0 * y0 + d0 - 2.0 * y1 + d1,
    ]
}

/// The derivative, at its first sample, of the degree-3 Lagrange polynomial
/// through the four given `(x, z)` samples at their chord parameters.
fn lagrange_first_tangent(points: &[[f64; 2]], params: &[f64]) -> [f64; 2] {
    // Closed form for evaluation at node 0 over nodes 0..3:
    //   L'(x0) = sum_i P_i * l_i'(x0)
    let l0prime = 1.0 / (params[0] - params[1])
        + 1.0 / (params[0] - params[2])
        + 1.0 / (params[0] - params[3]);
    let l1prime = 1.0 / (params[1] - params[0])
        * ((params[0] - params[2]) / (params[1] - params[2]))
        * ((params[0] - params[3]) / (params[1] - params[3]));
    let l2prime = 1.0 / (params[2] - params[0])
        * ((params[0] - params[1]) / (params[2] - params[1]))
        * ((params[0] - params[3]) / (params[2] - params[3]));
    let l3prime = 1.0 / (params[3] - params[0])
        * ((params[0] - params[1]) / (params[3] - params[1]))
        * ((params[0] - params[2]) / (params[3] - params[2]));
    let mut tx = 0.0;
    let mut tz = 0.0;
    let factors = [l0prime, l1prime, l2prime, l3prime];
    for i in 0..4 {
        tx += points[i][0] * factors[i];
        tz += points[i][1] * factors[i];
    }
    [tx, tz]
}

/// The derivative, at its last sample, of the degree-3 Lagrange polynomial
/// through the four given `(x, z)` samples at their chord parameters.
fn lagrange_last_tangent(points: &[[f64; 2]], params: &[f64]) -> [f64; 2] {
    // Closed form for evaluation at node 3 over nodes 0..3.
    let l0prime = 1.0 / (params[0] - params[3])
        * ((params[3] - params[1]) / (params[0] - params[1]))
        * ((params[3] - params[2]) / (params[0] - params[2]));
    let l1prime = 1.0 / (params[1] - params[3])
        * ((params[3] - params[0]) / (params[1] - params[0]))
        * ((params[3] - params[2]) / (params[1] - params[2]));
    let l2prime = 1.0 / (params[2] - params[3])
        * ((params[3] - params[0]) / (params[2] - params[0]))
        * ((params[3] - params[1]) / (params[2] - params[1]));
    let l3prime = 1.0 / (params[3] - params[0])
        + 1.0 / (params[3] - params[1])
        + 1.0 / (params[3] - params[2]);
    let mut tx = 0.0;
    let mut tz = 0.0;
    let factors = [l0prime, l1prime, l2prime, l3prime];
    for i in 0..4 {
        tx += points[i][0] * factors[i];
        tz += points[i][1] * factors[i];
    }
    [tx, tz]
}

/// The clamped cubic spline slopes (first derivatives `d/d(parameter)`) at
/// every sample: the interior slopes solve the C2 tridiagonal system, the
/// end slopes are the Lagrange end tangents of the first/last four samples.
fn clamped_slopes(points: &[[f64; 2]], params: &[f64]) -> Result<Vec<[f64; 2]>, Refusal> {
    let n = points.len();
    if n < 4 {
        return Err(Refusal::Empty);
    }
    let first4 = points.get(0..4).ok_or(Refusal::Empty)?;
    let last4 = points.get(n - 4..).ok_or(Refusal::Empty)?;
    let first_t = params.get(0..4).ok_or(Refusal::Empty)?;
    let last_t = params.get(n - 4..).ok_or(Refusal::Empty)?;
    let mut slopes = vec![[0.0, 0.0]; n];
    if let Some(first) = slopes.first_mut() {
        *first = lagrange_first_tangent(first4, first_t);
    }
    if let Some(last) = slopes.last_mut() {
        *last = lagrange_last_tangent(last4, last_t);
    }
    let nk = n - 2;
    let mut h = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let hi = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        if hi <= 0.0 || !hi.is_finite() {
            return Err(Refusal::Empty);
        }
        h.push(hi);
    }
    let mut delta = Vec::with_capacity(n - 1);
    for pair in points.windows(2) {
        let (x0, z0) = (pair[0][0], pair[0][1]);
        let (x1, z1) = (pair[1][0], pair[1][1]);
        let len = delta.len();
        let hi = h.get(len).copied().unwrap_or(1.0);
        delta.push([(x1 - x0) / hi, (z1 - z0) / hi]);
    }
    let mut rhs = vec![[0.0, 0.0]; nk];
    let mut a = vec![0.0; nk];
    let mut b = vec![0.0; nk];
    let mut c = vec![0.0; nk];
    for row in 0..nk {
        let i = row + 1;
        let hi = h.get(i).copied().ok_or(Refusal::Empty)?;
        let hm = h.get(i - 1).copied().ok_or(Refusal::Empty)?;
        if let Some(av) = a.get_mut(row) {
            *av = hi;
        }
        if let Some(bv) = b.get_mut(row) {
            *bv = 2.0 * (hm + hi);
        }
        if let Some(cv) = c.get_mut(row) {
            *cv = hm;
        }
        let dprev = delta.get(i - 1).copied().ok_or(Refusal::Empty)?;
        let dcur = delta.get(i).copied().ok_or(Refusal::Empty)?;
        let mut r0 = 3.0 * (hi * dprev[0] + hm * dcur[0]);
        let mut r1 = 3.0 * (hi * dprev[1] + hm * dcur[1]);
        if i == 1 {
            let s0 = slopes.get(0).copied().unwrap_or([0.0, 0.0]);
            r0 -= hi * s0[0];
            r1 -= hi * s0[1];
        }
        if i == n - 2 {
            let sn = slopes.get(n - 1).copied().unwrap_or([0.0, 0.0]);
            r0 -= hm * sn[0];
            r1 -= hm * sn[1];
        }
        if let Some(r) = rhs.get_mut(row) {
            *r = [r0, r1];
        }
    }
    // Thomas algorithm.
    let mut cp = vec![0.0; nk];
    let mut dp = vec![[0.0, 0.0]; nk];
    {
        let b0 = b.get(0).copied().ok_or(Refusal::Empty)?;
        if b0 == 0.0 {
            return Err(Refusal::Empty);
        }
        let c0 = c.get(0).copied().unwrap_or(0.0);
        if let Some(v) = cp.first_mut() {
            *v = c0 / b0;
        }
        let r = rhs.get(0).copied().ok_or(Refusal::Empty)?;
        if let Some(v) = dp.first_mut() {
            *v = [r[0] / b0, r[1] / b0];
        }
    }
    for row in 1..nk {
        let ai = a.get(row).copied().unwrap_or(0.0);
        let bi = b.get(row).copied().unwrap_or(0.0);
        let ci = c.get(row).copied().unwrap_or(0.0);
        let cp_prev = cp.get(row - 1).copied().unwrap_or(0.0);
        let dp_prev = dp.get(row - 1).copied().unwrap_or([0.0, 0.0]);
        let r = rhs.get(row).copied().unwrap_or([0.0, 0.0]);
        let denom = bi - ai * cp_prev;
        if denom == 0.0 || !denom.is_finite() {
            return Err(Refusal::Empty);
        }
        if row < nk - 1 {
            if let Some(v) = cp.get_mut(row) {
                *v = ci / denom;
            }
        }
        let v0 = (r[0] - ai * dp_prev[0]) / denom;
        let v1 = (r[1] - ai * dp_prev[1]) / denom;
        if !v0.is_finite() || !v1.is_finite() {
            return Err(Refusal::Empty);
        }
        if let Some(v) = dp.get_mut(row) {
            *v = [v0, v1];
        }
    }
    let mut x = vec![[0.0, 0.0]; nk];
    if let Some(last) = x.last_mut() {
        *last = dp.get(nk - 1).copied().unwrap_or([0.0, 0.0]);
    }
    for row in (0..nk - 1).rev() {
        let dpv = dp.get(row).copied().unwrap_or([0.0, 0.0]);
        let cpv = cp.get(row).copied().unwrap_or(0.0);
        let xv = x.get(row + 1).copied().unwrap_or([0.0, 0.0]);
        if let Some(v) = x.get_mut(row) {
            *v = [dpv[0] - cpv * xv[0], dpv[1] - cpv * xv[1]];
        }
    }
    for row in 0..nk {
        let v = x.get(row).copied().unwrap_or([0.0, 0.0]);
        if let Some(s) = slopes.get_mut(row + 1) {
            *s = v;
        }
    }
    Ok(slopes)
}

/// The local bounding box (`[min, max]`) of one unplaced solid.
fn solid_local_bbox(solid: &SolidSpec) -> Result<[[f64; 3]; 2], Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => {
            let (length, width, height) = (*length, *width, *height);
            Ok([
                [-length / 2.0, -width / 2.0, -height / 2.0],
                [length / 2.0, width / 2.0, height / 2.0],
            ])
        }
        SolidSpec::Cylinder {
            radius,
            height,
            axis,
        } => {
            let (radius, height) = (*radius, *height);
            match axis.as_str() {
                "x" => Ok([
                    [-height / 2.0, -radius, -radius],
                    [height / 2.0, radius, radius],
                ]),
                "y" => Ok([
                    [-radius, -height / 2.0, -radius],
                    [radius, height / 2.0, radius],
                ]),
                _ => Ok([
                    [-radius, -radius, -height / 2.0],
                    [radius, radius, height / 2.0],
                ]),
            }
        }
        SolidSpec::Sphere { radius } => {
            let radius = *radius;
            Ok([[-radius, -radius, -radius], [radius, radius, radius]])
        }
        SolidSpec::Torus { major, minor } => {
            let (major, minor) = (*major, *minor);
            Ok([
                [-(major + minor), -(major + minor), -minor],
                [major + minor, major + minor, minor],
            ])
        }
        SolidSpec::Lathe { profile, .. } => {
            if let Some(points) = line_profile_vertices(profile) {
                let mut max_r = 0.0f64;
                let mut z0 = f64::INFINITY;
                let mut z1 = f64::NEG_INFINITY;
                for point in &points {
                    if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
                        return Err(Refusal::Empty);
                    }
                    max_r = max_r.max(point[0]);
                    z0 = z0.min(point[1]);
                    z1 = z1.max(point[1]);
                }
                if !z0.is_finite() || !z1.is_finite() {
                    return Err(Refusal::Empty);
                }
                Ok([[-max_r, -max_r, z0], [max_r, max_r, z1]])
            } else {
                lathe_profile_bbox(profile)
            }
        }
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => prism_bbox(profile, *amount, *both),
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => crate::python::binding::trim_extrude_solid(
            profile, *amount, *both, trim_curve, trim_net, *tolerance,
        )
        .map(|facts| facts.bbox)
        .map_err(trim_binding_error_to_refusal),
        SolidSpec::Loft { sections, .. } => {
            if let Ok(validated) = loft_sections(sections) {
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                for loop3 in &validated.loops {
                    let box3 = loop_bbox(loop3);
                    for axis in 0..3 {
                        min[axis] = min[axis].min(box3[0][axis]);
                        max[axis] = max[axis].max(box3[1][axis]);
                    }
                }
                if !min[0].is_finite() || !max[0].is_finite() {
                    return Err(Refusal::Empty);
                }
                return Ok([min, max]);
            }
            // The ruled side surface is linear in the station axis, so the
            // exact AABB is the union of the two section loops' exact AABBs.
            if sections.len() != 2 {
                return Err(open_smooth_loft());
            }
            let first = spline_loop_spans(sections.first().ok_or(Refusal::Empty)?)?;
            let last = spline_loop_spans(sections.get(1).ok_or(Refusal::Empty)?)?;
            let box_a = spline_loop_bbox3(&first)?;
            let box_b = spline_loop_bbox3(&last)?;
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for axis in 0..3 {
                min[axis] = box_a[0][axis].min(box_b[0][axis]);
                max[axis] = box_a[1][axis].max(box_b[1][axis]);
            }
            if !min[0].is_finite() || !max[0].is_finite() {
                return Err(Refusal::Empty);
            }
            Ok([min, max])
        }
    }
}

/// The exact axis-aligned bbox of a spline-bearing lathe: `max_r` is the
/// profile's largest radius and `z` spans the profile's exact extrema (for a
/// spline edge the cubic span extrema, solved from the derivative roots).
fn lathe_profile_bbox(profile: &[LatheEdge]) -> Result<[[f64; 3]; 2], Refusal> {
    check_profile_edges(profile)?;
    let mut max_r = 0.0f64;
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                for p in [a, b] {
                    max_r = max_r.max(p[0]);
                    z_min = z_min.min(p[1]);
                    z_max = z_max.max(p[1]);
                }
            }
            LatheEdge::Spline { points } => {
                let spans = spline_spans(points)?;
                for span in &spans {
                    let x0 = span.r[0];
                    let x1 = span.r[0] + span.r[1] + span.r[2] + span.r[3];
                    max_r = max_r.max(x0).max(x1);
                    for root in cubic_roots(&span.r) {
                        max_r = max_r.max(
                            span.r[0] + root * (span.r[1] + root * (span.r[2] + root * span.r[3])),
                        );
                    }
                    let z0 = span.z[0];
                    let z1 = span.z[0] + span.z[1] + span.z[2] + span.z[3];
                    z_min = z_min.min(z0).min(z1);
                    z_max = z_max.max(z0).max(z1);
                    for root in cubic_roots(&span.z) {
                        let vz =
                            span.z[0] + root * (span.z[1] + root * (span.z[2] + root * span.z[3]));
                        z_min = z_min.min(vz);
                        z_max = z_max.max(vz);
                    }
                }
            }
        }
    }
    if !z_min.is_finite() || !z_max.is_finite() || !max_r.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([[-max_r, -max_r, z_min], [max_r, max_r, z_max]])
}

/// The roots in `(0, 1)` of the derivative of the cubic with the given power
/// coefficients `c[0] + c[1] u + c[2] u^2 + c[3] u^3`.
fn cubic_roots(c: &[f64; 4]) -> Vec<f64> {
    let mut roots = Vec::with_capacity(2);
    let a = 3.0 * c[3];
    let b = 2.0 * c[2];
    let cc = c[1];
    if a == 0.0 {
        // Quadratic (or linear) derivative.
        if b != 0.0 {
            let root = -cc / b;
            if root > 0.0 && root < 1.0 {
                roots.push(root);
            }
        }
        return roots;
    }
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return roots;
    }
    let sq = disc.sqrt();
    for root in [(-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a)] {
        if root > 0.0 && root < 1.0 {
            roots.push(root);
        }
    }
    roots
}

// ---------------------------------------------------------------------------
// Spline-section loft facts: the certified smooth-volume arm
// (BRIDGE-LOFT-FACTS)
// ---------------------------------------------------------------------------
//
// A spline-section loft's exact volume is a kernel volume fact over the smooth
// surface's L1-extracted tensor-Bernstein patches (ADM-003). The bridge cannot
// name the kernel's `BSplineSurface<Vector4>` carrier directly (the one
// sanctioned crate edge exposes only the stabilized facade's plain-data
// entries), so it assembles the smooth surface's patches itself from the
// recorded section curves and submits every patch to the sanctioned
// `binding_volume_facts` entry, summing the certified brackets. The section
// curves are reconstructed exactly (chord-length parameters, clamped cubic,
// endpoint tangents from the degree-3 Lagrange interpolant — the same
// convention the landed lathe arm reconstructs `Edge.make_spline` with), never
// a flattening polygon.
//
// **The honesty line (scope decision 1).** The smooth station interpolation is
// exactly determined by the recorded data only for a two-station loft, where
// the smooth surface IS the ruled surface (every interpolation degree reduces
// to the linear one across two stations). A three-or-more-station smooth
// loft's station parameterization is an OCC `ThruSections` convention the row
// does not record, so the arm refuses TYPED naming the open smooth carrier
// rather than substituting a ruled approximation.

/// One reconstructed 3-D spline span in the power basis over `u in [0, 1]`.
#[derive(Debug, Clone, Copy)]
struct SpanPoly3 {
    /// The x component `x(u)`.
    x: [f64; 4],
    /// The y component `y(u)`.
    y: [f64; 4],
    /// The z component `z(u)`.
    z: [f64; 4],
}

/// The typed refusal of a loft carrier whose smooth surface the recorded data
/// does not determine (the open smooth carrier), never an approximation.
fn open_smooth_loft() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

/// The chord-length parameters of the 3-D samples.
fn chord_params3(points: &[[f64; 3]]) -> Vec<f64> {
    let mut params = Vec::with_capacity(points.len());
    params.push(0.0);
    for pair in points.windows(2) {
        let dx = pair[1][0] - pair[0][0];
        let dy = pair[1][1] - pair[0][1];
        let dz = pair[1][2] - pair[0][2];
        let next = params.last().copied().unwrap_or(0.0) + (dx * dx + dy * dy + dz * dz).sqrt();
        params.push(next);
    }
    params
}

/// The power-basis quadratic span through three samples at normalized
/// parameter `u1` (the 3-D specialization of the landed 2-D arm).
fn quadratic_span3(p0: f64, p1: f64, p2: f64, u1: f64, denom: f64) -> [f64; 4] {
    let q1 = (p1 - (1.0 - u1) * (1.0 - u1) * p0 - u1 * u1 * p2) / denom;
    [p0, 2.0 * (q1 - p0), p0 - 2.0 * q1 + p2, 0.0]
}

/// The reconstructed spans of the 3-D OCC interpolating curve through the
/// samples: the same fixed-order clamped cubic the landed lathe arm uses,
/// component-wise.
fn spline_spans3(points: &[[f64; 3]]) -> Result<Vec<SpanPoly3>, Refusal> {
    let n = points.len();
    if n < 2 {
        return Err(Refusal::Empty);
    }
    for p in points {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(Refusal::Empty);
        }
    }
    let params = chord_params3(points);
    if n == 2 {
        let a = points[0];
        let b = points[1];
        return Ok(vec![SpanPoly3 {
            x: [a[0], b[0] - a[0], 0.0, 0.0],
            y: [a[1], b[1] - a[1], 0.0, 0.0],
            z: [a[2], b[2] - a[2], 0.0, 0.0],
        }]);
    }
    if n == 3 {
        let u1 = (params[1] - params[0]) / (params[2] - params[0]);
        let denom = 2.0 * u1 * (1.0 - u1);
        if denom == 0.0 || !denom.is_finite() {
            return Err(Refusal::Empty);
        }
        return Ok(vec![SpanPoly3 {
            x: quadratic_span3(points[0][0], points[1][0], points[2][0], u1, denom),
            y: quadratic_span3(points[0][1], points[1][1], points[2][1], u1, denom),
            z: quadratic_span3(points[0][2], points[1][2], points[2][2], u1, denom),
        }]);
    }
    let xy: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[1]]).collect();
    let xz: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[2]]).collect();
    let sxy = clamped_slopes(&xy, &params)?;
    let sxz = clamped_slopes(&xz, &params)?;
    let mut spans = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let h = params.get(i + 1).copied().ok_or(Refusal::Empty)?
            - params.get(i).copied().ok_or(Refusal::Empty)?;
        let p0 = points.get(i).ok_or(Refusal::Empty)?;
        let p1 = points.get(i + 1).ok_or(Refusal::Empty)?;
        let s0xy = sxy.get(i).ok_or(Refusal::Empty)?;
        let s1xy = sxy.get(i + 1).ok_or(Refusal::Empty)?;
        let s0xz = sxz.get(i).ok_or(Refusal::Empty)?;
        let s1xz = sxz.get(i + 1).ok_or(Refusal::Empty)?;
        spans.push(SpanPoly3 {
            x: hermite_power(p0[0], p1[0], h * s0xy[0], h * s1xy[0]),
            y: hermite_power(p0[1], p1[1], h * s0xy[1], h * s1xy[1]),
            z: hermite_power(p0[2], p1[2], h * s0xz[1], h * s1xz[1]),
        });
    }
    Ok(spans)
}

/// The degree-3 Bernstein controls of a power-basis cubic (exact degree
/// elevation is a no-op here: a lower-degree span carries zero high
/// coefficients).
fn cubic_bernstein(p: &[f64; 4]) -> [f64; 4] {
    [
        p[0],
        p[0] + p[1] / 3.0,
        p[0] + 2.0 * p[1] / 3.0 + p[2] / 3.0,
        p[0] + p[1] + p[2] + p[3],
    ]
}

/// The `4 x 3` Bernstein control row of one reconstructed span.
fn span3_bernstein(span: &SpanPoly3) -> [[f64; 3]; 4] {
    let bx = cubic_bernstein(&span.x);
    let by = cubic_bernstein(&span.y);
    let bz = cubic_bernstein(&span.z);
    [
        [bx[0], by[0], bz[0]],
        [bx[1], by[1], bz[1]],
        [bx[2], by[2], bz[2]],
        [bx[3], by[3], bz[3]],
    ]
}

/// The reconstructed spans of a closed section loop (line and spline edges),
/// with the exact seam closure checked. Every edge's recorded end must meet the
/// next edge's recorded start and the last must return to the first.
fn spline_loop_spans(profile: &[ProfileEdge]) -> Result<Vec<SpanPoly3>, Refusal> {
    if profile.is_empty() {
        return Err(Refusal::Empty);
    }
    let mut spans: Vec<SpanPoly3> = Vec::new();
    let mut starts: Vec<[f64; 3]> = Vec::new();
    let mut ends: Vec<[f64; 3]> = Vec::new();
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, b } => {
                if !a.iter().all(|c| c.is_finite()) || !b.iter().all(|c| c.is_finite()) {
                    return Err(Refusal::Empty);
                }
                starts.push(*a);
                ends.push(*b);
                spans.push(SpanPoly3 {
                    x: [a[0], b[0] - a[0], 0.0, 0.0],
                    y: [a[1], b[1] - a[1], 0.0, 0.0],
                    z: [a[2], b[2] - a[2], 0.0, 0.0],
                });
            }
            ProfileEdge::Spline { points } => {
                if points.len() < 2 {
                    return Err(Refusal::Empty);
                }
                starts.push(*points.first().ok_or(Refusal::Empty)?);
                ends.push(*points.last().ok_or(Refusal::Empty)?);
                spans.extend(spline_spans3(points)?);
            }
        }
    }
    let count = starts.len();
    if count < 2 {
        return Err(open_smooth_loft());
    }
    let mut scale = 0.0f64;
    for p in starts.iter().chain(ends.iter()) {
        for c in p {
            scale = scale.max(c.abs());
        }
    }
    let tol = 1e-9 * (1.0 + scale);
    for i in 0..count {
        let next = starts.get((i + 1) % count).ok_or(Refusal::Empty)?;
        let gap = v3_norm(v3_sub(*ends.get(i).ok_or(Refusal::Empty)?, *next));
        if !(gap <= tol) {
            return Err(open_smooth_loft());
        }
    }
    Ok(spans)
}

/// The exact `int_0^1 a(u) b(u) du` of two power-basis polynomials.
fn poly_integral(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let mut acc = 0.0;
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            acc += ai * bj / ((i + j + 1) as f64);
        }
    }
    acc
}

/// The signed half area moment `(1/2) int_0^1 r(u) x r'(u) du` of one span.
fn span_area_moment(span: &SpanPoly3) -> [f64; 3] {
    let dx = [span.x[1], 2.0 * span.x[2], 3.0 * span.x[3], 0.0];
    let dy = [span.y[1], 2.0 * span.y[2], 3.0 * span.y[3], 0.0];
    let dz = [span.z[1], 2.0 * span.z[2], 3.0 * span.z[3], 0.0];
    [
        0.5 * (poly_integral(&span.y, &dz) - poly_integral(&span.z, &dy)),
        0.5 * (poly_integral(&span.z, &dx) - poly_integral(&span.x, &dz)),
        0.5 * (poly_integral(&span.x, &dy) - poly_integral(&span.y, &dx)),
    ]
}

/// The signed area vector of a closed reconstructed loop (Green's theorem).
fn spline_loop_area_vector(spans: &[SpanPoly3]) -> [f64; 3] {
    let mut area = [0.0f64; 3];
    for span in spans {
        let moment = span_area_moment(span);
        area[0] += moment[0];
        area[1] += moment[1];
        area[2] += moment[2];
    }
    area
}

/// The exact local AABB of a closed reconstructed loop: the span endpoints plus
/// every interior extremum of each cubic component.
fn spline_loop_bbox3(spans: &[SpanPoly3]) -> Result<[[f64; 3]; 2], Refusal> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for span in spans {
        for (axis, coeffs) in [&span.x, &span.y, &span.z].into_iter().enumerate() {
            let start = coeffs[0];
            let end = coeffs[0] + coeffs[1] + coeffs[2] + coeffs[3];
            min[axis] = min[axis].min(start).min(end);
            max[axis] = max[axis].max(start).max(end);
            for root in cubic_roots(coeffs) {
                let value = coeffs[0] + root * (coeffs[1] + root * (coeffs[2] + root * coeffs[3]));
                min[axis] = min[axis].min(value);
                max[axis] = max[axis].max(value);
            }
        }
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([min, max])
}

/// The certified volume of a two-station spline-section loft: every ruled side
/// patch's face form through the sanctioned `binding_volume_facts` entry plus
/// the exact planar end-cap moments, returned as `(value, lo, hi)`. Any other
/// section stack refuses typed naming the open smooth carrier.
fn certified_spline_loft_volume(sections: &[Vec<ProfileEdge>]) -> Result<(f64, f64, f64), Refusal> {
    if sections.len() != 2 {
        return Err(open_smooth_loft());
    }
    let first = spline_loop_spans(sections.first().ok_or(Refusal::Empty)?)?;
    let last = spline_loop_spans(sections.get(1).ok_or(Refusal::Empty)?)?;
    if first.is_empty() || first.len() != last.len() {
        return Err(open_smooth_loft());
    }
    let mut value = 0.0f64;
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    for (a, b) in first.iter().zip(last.iter()) {
        let ba = span3_bernstein(a);
        let bb = span3_bernstein(b);
        let mut numerator: Vec<Vec<[f64; 3]>> = Vec::with_capacity(4);
        for i in 0..4 {
            numerator.push(vec![
                *ba.get(i).ok_or(Refusal::Empty)?,
                *bb.get(i).ok_or(Refusal::Empty)?,
            ]);
        }
        let weights = vec![vec![1.0, 1.0]; 4];
        let row = crate::python::binding::VolumeRow {
            numerator,
            weights,
            orientation: 1.0,
        };
        let json = crate::python::binding::volume_facts(&row).map_err(|_| open_smooth_loft())?;
        let outcome: serde_json::Value = serde_json::from_str(&json).map_err(|_| Refusal::Empty)?;
        value += outcome
            .get("value")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
        lo += outcome
            .pointer("/bracket/lo")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
        hi += outcome
            .pointer("/bracket/hi")
            .and_then(serde_json::Value::as_f64)
            .ok_or(Refusal::Empty)?;
    }

    // The planar end caps: `(1/3) d A` with the recorded-loop winding, exactly
    // as the landed line-loft arm's cap terms.
    let area_a = spline_loop_area_vector(&first);
    let area_b = spline_loop_area_vector(&last);
    let mag_a = v3_norm(area_a);
    let mag_b = v3_norm(area_b);
    if !(mag_a > 0.0) || !(mag_b > 0.0) {
        return Err(open_smooth_loft());
    }
    let normal_a = [area_a[0] / mag_a, area_a[1] / mag_a, area_a[2] / mag_a];
    let normal_b = [area_b[0] / mag_b, area_b[1] / mag_b, area_b[2] / mag_b];
    let start_a = first.first().ok_or(Refusal::Empty)?;
    let start_b = last.first().ok_or(Refusal::Empty)?;
    let point_a = [start_a.x[0], start_a.y[0], start_a.z[0]];
    let point_b = [start_b.x[0], start_b.y[0], start_b.z[0]];
    let cap_a = -(1.0 / 3.0) * mag_a * v3_dot(normal_a, point_a);
    let cap_b = (1.0 / 3.0) * mag_b * v3_dot(normal_b, point_b);
    value += cap_a + cap_b;
    lo += cap_a + cap_b;
    hi += cap_a + cap_b;
    if !value.is_finite() || !lo.is_finite() || !hi.is_finite() {
        return Err(Refusal::Empty);
    }
    if value < 0.0 {
        Ok((value.abs(), -hi, -lo))
    } else {
        Ok((value, lo, hi))
    }
}

/// Evaluates a reconstructed 3-D span at `u in [0, 1]`.
fn eval_span3(span: &SpanPoly3, u: f64) -> [f64; 3] {
    [
        span.x[0] + u * (span.x[1] + u * (span.x[2] + u * span.x[3])),
        span.y[0] + u * (span.y[1] + u * (span.y[2] + u * span.y[3])),
        span.z[0] + u * (span.z[1] + u * (span.z[2] + u * span.z[3])),
    ]
}

/// Samples every span of a closed reconstructed loop at `MESH_SEGMENTS` steps.
fn sample_loop3(spans: &[SpanPoly3]) -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(spans.len() * MESH_SEGMENTS);
    for span in spans {
        for step in 0..MESH_SEGMENTS {
            out.push(eval_span3(span, step as f64 / MESH_SEGMENTS as f64));
        }
    }
    out
}

/// The centroid of a point cloud.
fn centroid3(points: &[[f64; 3]]) -> [f64; 3] {
    let mut c = [0.0f64; 3];
    for p in points {
        c[0] += p[0];
        c[1] += p[1];
        c[2] += p[2];
    }
    let k = 1.0 / (points.len() as f64);
    [c[0] * k, c[1] * k, c[2] * k]
}

/// The deterministic local mesh of a two-station spline-section loft: ruled
/// quads between the sampled reconstructed loops plus the two end-cap fans.
fn spline_loft_mesh(sections: &[Vec<ProfileEdge>]) -> Result<Vec<Triangle>, Refusal> {
    if sections.len() != 2 {
        return Err(open_smooth_loft());
    }
    let first = spline_loop_spans(sections.first().ok_or(Refusal::Empty)?)?;
    let last = spline_loop_spans(sections.get(1).ok_or(Refusal::Empty)?)?;
    if first.is_empty() || first.len() != last.len() {
        return Err(open_smooth_loft());
    }
    let pa = sample_loop3(&first);
    let pb = sample_loop3(&last);
    let count = pa.len();
    let mut out = Vec::new();
    for i in 0..count {
        let j = (i + 1) % count;
        push_quad(
            &mut out,
            *pa.get(i).ok_or(Refusal::Empty)?,
            *pa.get(j).ok_or(Refusal::Empty)?,
            *pb.get(j).ok_or(Refusal::Empty)?,
            *pb.get(i).ok_or(Refusal::Empty)?,
        );
    }
    let ca = centroid3(&pa);
    let cb = centroid3(&pb);
    for i in 0..count {
        let j = (i + 1) % count;
        push_tri(
            &mut out,
            ca,
            *pa.get(i).ok_or(Refusal::Empty)?,
            *pa.get(j).ok_or(Refusal::Empty)?,
        );
        push_tri(
            &mut out,
            cb,
            *pb.get(i).ok_or(Refusal::Empty)?,
            *pb.get(j).ok_or(Refusal::Empty)?,
        );
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Authoring arms: 3-D profile helpers (prism extrude / loft / sweep chain)
// ---------------------------------------------------------------------------
//
// The extrude and loft recording arms carry their profiles as closed planar
// line loops in the part's local 3-D frame. All facts are exact analytic
// arithmetic over those loops:
//
// * `Prism`: the volume is the profile area times the swept length along the
//   profile-plane normal (OCC's Face-normal extrusion convention: a single-
//   sided sweep runs +right-hand-normal, `both` sweeps symmetrically), the
//   bbox is the profile support widened along the normal and the mesh sweeps
//   the boundary deterministically.
// * `Loft`: the ruled matched-vertex interpolation. Over each pair of stations
//   the cross-section polygon area is a quadratic in the stack parameter, so a
//   chain of two-section lofts is the "segment-moment generalisation" of the
//   FH-SPLINE-LATHE line-profile revolution (whose frustum telescoping is the
//   degenerate two-section case). The volume is the exact divergence-form
//   boundary integral over the loft's faces: each planar cap contributes
//   `(1/3) n . int x dA` (exact shoelace moments over the cap polygon) and
//   every ruled side patch between matching edges contributes the exact
//   bilinear moment `(1/3) int int X.(Xu x Xv) du dv`, evaluated by two-point
//   Gauss-Legendre quadrature (exact: the integrand is a polynomial of degree
//   (2,2) in (u, v)). A closed halo row has no end caps; the seam is certified
//   by the exact aligned-meeting-edge identity on the stations that return to
//   the start.
// * `mirror`: a placed-carrier reflection; the solid spec is untouched and the
//   facts transform with the placement (no geometry recomputation).

/// A closed planar line-loop profile extracted from a recorded profile edge
/// list.
struct ProfileLoop {
    /// The boundary vertices in order (no duplicated closing point).
    verts: Vec<[f64; 3]>,
    /// `area_vec / |area_vec|`.
    normal: [f64; 3],
    /// The loop's signed area (`|area_vec|`).
    area: f64,
    /// The loop scale (max absolute coordinate), used for tolerances.
    scale: f64,
}

fn v3_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn v3_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn v3_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The all-line vertex list of a recorded 3-D profile (one vertex per edge, in
/// order). `None` when any edge is a spline: a spline carrier cannot be
/// flattened to its sample polygon, so the caller refuses typed rather than
/// approximate.
fn profile3_vertices(profile: &[ProfileEdge]) -> Option<Vec<[f64; 3]>> {
    let mut out = Vec::with_capacity(profile.len());
    for edge in profile {
        match edge {
            ProfileEdge::Line { a, .. } => out.push(*a),
            ProfileEdge::Spline { .. } => return None,
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

/// Builds and validates one closed planar line-loop profile.
fn profile_loop(profile: &[ProfileEdge]) -> Result<ProfileLoop, Refusal> {
    let verts = profile3_vertices(profile).ok_or_else(|| {
        // A spline profile edge is not flattenable to its sample polygon; the
        // recording arm keeps the samples and the fact arm refuses typed.
        Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
    })?;
    if verts.len() < 3 {
        return Err(Refusal::Empty);
    }
    let mut scale = 0.0f64;
    for v in &verts {
        for c in v {
            if !c.is_finite() {
                return Err(Refusal::Empty);
            }
            scale = scale.max(c.abs());
        }
    }
    // The recorded edges must chain into a closed loop: consecutive edges share
    // their meeting point and the last edge returns to the first vertex. The
    // maximum gap is the seam mismatch evidence of the carrier.
    let seam_tol = 1e-9 * (1.0 + scale);
    let mut max_gap = 0.0f64;
    let n = verts.len();
    for i in 0..n {
        let b = match profile.get(i) {
            Some(ProfileEdge::Line { b, .. }) => *b,
            _ => verts[(i + 1) % n],
        };
        let next = verts[(i + 1) % n];
        let gap = if i + 1 < n {
            v3_norm(v3_sub(b, next))
        } else {
            v3_norm(v3_sub(b, verts[0]))
        };
        max_gap = max_gap.max(gap);
    }
    if max_gap > seam_tol {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    // Right-hand area vector.
    let mut ax = 0.0f64;
    let mut ay = 0.0f64;
    let mut az = 0.0f64;
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        ax += a[1] * b[2] - a[2] * b[1];
        ay += a[2] * b[0] - a[0] * b[2];
        az += a[0] * b[1] - a[1] * b[0];
    }
    let area_vec = [ax * 0.5, ay * 0.5, az * 0.5];
    let mag = v3_norm(area_vec);
    if !(mag > 0.0) || !mag.is_finite() {
        return Err(Refusal::Empty);
    }
    let normal = [area_vec[0] / mag, area_vec[1] / mag, area_vec[2] / mag];
    // The loop must be planar (the recorded surface is elementary).
    let origin = verts[0];
    let planarity_tol = 1e-7 * (1.0 + scale);
    for a in &verts {
        let rel = v3_sub(*a, origin);
        if v3_dot(rel, normal).abs() > planarity_tol {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(ProfileLoop {
        verts,
        normal,
        area: mag,
        scale,
    })
}

/// The unit normal of a difference vector, or `None` when it is degenerate.
fn v3_norm(a: [f64; 3]) -> f64 {
    v3_dot(a, a).sqrt()
}

/// The local bounding box and volume of an exact prism extruded from a closed
/// planar line-loop profile along its plane normal.
pub(crate) struct PrismGeom {
    loop3: ProfileLoop,
    /// Signed start offset along the normal (0 or -amount).
    t_lo: f64,
    /// Signed end offset along the normal (amount, or amount when both).
    t_hi: f64,
    /// The exact prism volume.
    pub(crate) volume: f64,
}

pub(crate) fn prism_geom(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<PrismGeom, Refusal> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(Refusal::Empty);
    }
    let loop3 = profile_loop(profile)?;
    let (t_lo, t_hi) = if both {
        (-amount, amount)
    } else {
        (0.0, amount)
    };
    let height = t_hi - t_lo;
    let volume = loop3.area * height;
    Ok(PrismGeom {
        loop3,
        t_lo,
        t_hi,
        volume,
    })
}

/// The exact local AABB of an extruded prism: the profile loop's support along
/// each axis, widened by the extrusion range along the plane normal.
pub(crate) fn prism_bbox(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<[[f64; 3]; 2], Refusal> {
    let geom = prism_geom(profile, amount, both)?;
    let n = geom.loop3.normal;
    let box0 = loop_bbox(&geom.loop3);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for axis in 0..3 {
        // The t contribution is constant over the cross-section, so the prism
        // extent is the loop support plus the t range projected on the axis.
        let (d_min, d_max) = if n[axis] >= 0.0 {
            (geom.t_lo * n[axis], geom.t_hi * n[axis])
        } else {
            (geom.t_hi * n[axis], geom.t_lo * n[axis])
        };
        min[axis] = box0[0][axis] + d_min;
        max[axis] = box0[1][axis] + d_max;
    }
    if !min[0].is_finite() || !max[0].is_finite() {
        return Err(Refusal::Empty);
    }
    Ok([min, max])
}

/// The exact AABB of a profile loop: the support of any axis-aligned linear
/// function over a polygon is attained at a polygon vertex, so the loop's
/// bbox over its vertices is exact for the loop region.
fn loop_bbox(loop3: &ProfileLoop) -> [[f64; 3]; 2] {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &loop3.verts {
        for axis in 0..3 {
            min[axis] = min[axis].min(v[axis]);
            max[axis] = max[axis].max(v[axis]);
        }
    }
    [min, max]
}

/// The exact volume of a ruled two-section loft between two matched parallel
/// planar line-loop sections (the "segment moment" of one loft pair). The
/// cross-section polygon's area is quadratic in the stack parameter, so the
/// segment volume is the exact Simpson value over the end and mid sections.
/// (Test-side independent machine check of the divergence-form volume arm.)
#[cfg(test)]
fn loft_segment_volume(a: &ProfileLoop, b: &ProfileLoop) -> f64 {
    let n = a.verts.len();
    // Mid-section vertices (matched linear interpolation at u = 1/2).
    let mut mid = Vec::with_capacity(n);
    for i in 0..n {
        let p = a.verts[i];
        let q = b.verts[i];
        mid.push([
            0.5 * (p[0] + q[0]),
            0.5 * (p[1] + q[1]),
            0.5 * (p[2] + q[2]),
        ]);
    }
    // The mid polygon lies in a plane halfway between the (parallel) section
    // planes; its right-hand area along the shared normal.
    let mut cross = 0.0f64;
    let normal = a.normal;
    for i in 0..n {
        let p = mid[i];
        let q = mid[(i + 1) % n];
        cross += v3_dot(v3_cross(p, q), normal);
    }
    let area_mid = cross * 0.5;
    // The perpendicular separation of the two section planes.
    let d = v3_sub(b.verts[0], a.verts[0]);
    let h = v3_dot(d, normal).abs();
    // Simpson over the quadratic cross-section area: exact.
    (h / 6.0) * (a.area + 4.0 * area_mid + b.area)
}

/// The signed side-patch moment `(1/3) int int X.(Xu x Xv) du dv` of one ruled
/// patch between the matching edges `(a0 -> a1)` of one section and
/// `(b0 -> b1)` of the next, evaluated by two-point Gauss-Legendre quadrature
/// (exact for the polynomial integrand).
fn side_patch_moment(a0: [f64; 3], a1: [f64; 3], b0: [f64; 3], b1: [f64; 3]) -> f64 {
    let inv_sqrt3 = 1.0 / 3.0f64.sqrt();
    let nodes = [0.5 - 0.5 * inv_sqrt3, 0.5 + 0.5 * inv_sqrt3];
    let da = v3_sub(a1, a0);
    let db = v3_sub(b1, b0);
    let ga = v3_sub(b0, a0);
    let gb = v3_sub(b1, a1);
    let mut acc = 0.0f64;
    for ui in nodes {
        for vi in nodes {
            // A(u) and B(u).
            let a = [a0[0] + ui * da[0], a0[1] + ui * da[1], a0[2] + ui * da[2]];
            let b = [b0[0] + ui * db[0], b0[1] + ui * db[1], b0[2] + ui * db[2]];
            // X = A + v (B - A); Xu = (1-v) da + v db; Xv = (1-u) ga + u gb.
            let ab = v3_sub(b, a);
            let x = [a[0] + vi * ab[0], a[1] + vi * ab[1], a[2] + vi * ab[2]];
            let xu = [
                (1.0 - vi) * da[0] + vi * db[0],
                (1.0 - vi) * da[1] + vi * db[1],
                (1.0 - vi) * da[2] + vi * db[2],
            ];
            let xv = [
                (1.0 - ui) * ga[0] + ui * gb[0],
                (1.0 - ui) * ga[1] + ui * gb[1],
                (1.0 - ui) * ga[2] + ui * gb[2],
            ];
            acc += v3_dot(x, v3_cross(xu, xv));
        }
    }
    // 2x2 Gauss: each node weight is 1/4 on [0,1]^2.
    acc / 12.0
}

/// A loft arm's per-section data validated for the exact ruled carrier.
struct LoftSections {
    loops: Vec<ProfileLoop>,
}

/// Validates a loft section stack for the exact ruled carrier: every section
/// must be a closed planar line loop, all sections must carry the same number
/// of matched vertices, and consecutive sections must stack with their
/// recorded winding normals (the gate below refuses a non-uniform or folded
/// frame instead of approximating it).
fn loft_sections(sections: &[Vec<ProfileEdge>]) -> Result<LoftSections, Refusal> {
    if sections.len() < 2 {
        return Err(Refusal::Empty);
    }
    let loops: Vec<ProfileLoop> = sections
        .iter()
        .map(|s| profile_loop(s))
        .collect::<Result<_, _>>()?;
    let count = loops[0].verts.len();
    for loop3 in &loops {
        if loop3.verts.len() != count {
            // A matched ruled carrier needs equal vertex counts.
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    Ok(LoftSections { loops })
}

/// The seam certificate of a closed halo loft row: the last station is the
/// exact return to the first station, so the aligned meeting edges across the
/// chain closure satisfy `A0 W1 - A1 W0 == 0` (weights 1 for the recorded line
/// carrier). Returns the maximum mismatch over the aligned station vertices; a
/// station list that does not close returns `Err` with the mismatch evidence.
fn loft_seam_mismatch(sections: &LoftSections, closed: bool) -> Result<f64, Refusal> {
    if !closed {
        return Ok(f64::INFINITY); // no seam to certify on an open chain
    }
    let first = &sections.loops[0];
    let last = sections.loops.last().ok_or(Refusal::Empty)?;
    let scale = (first.scale).max(last.scale);
    let tol = 1e-7 * (1.0 + scale);
    let n = first.verts.len();
    let mut max_gap = 0.0f64;
    for i in 0..n {
        let gap = v3_norm(v3_sub(first.verts[i], last.verts[i]));
        max_gap = max_gap.max(gap);
    }
    if max_gap > tol {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        ));
    }
    Ok(max_gap)
}

/// The exact volume of a loft chain. For an open chain the boundary is the
/// ruled side surface between consecutive stations plus the two end caps; for
/// a closed halo row the station list returns to its start and the side
/// surface closes on itself (no caps, the last station repeats the first and
/// its zero-length wrap segment contributes nothing). The winding direction is
/// not part of the row data, so the orientation of the recorded loops is
/// absorbed exactly as in the lathe arm.
fn loft_volume(sections: &LoftSections, closed: bool) -> Result<f64, Refusal> {
    let mut total = 0.0f64;
    let n_sec = sections.loops.len();
    let segment_count = if closed { n_sec - 1 } else { n_sec - 1 };
    for j in 0..segment_count {
        let a = &sections.loops[j];
        let b = &sections.loops[j + 1];
        let n = a.verts.len();
        // Matched ruled side patches between the corresponding edges.
        for i in 0..n {
            let a0 = a.verts[i];
            let a1 = a.verts[(i + 1) % n];
            let b0 = b.verts[i];
            let b1 = b.verts[(i + 1) % n];
            total += side_patch_moment(a0, a1, b0, b1);
        }
    }
    if !closed {
        // End caps: the recorded section is a flat polygon in the plane
        // `normal . x = d`, so its outward-facing divergence contribution is
        // `(1/3) d A`. The outward cap normal is opposite the recorded winding
        // normal at the first station (interior lies along the chain) and along
        // it at the last station.
        let first = &sections.loops[0];
        let last = sections.loops.last().ok_or(Refusal::Empty)?;
        let d_first = v3_dot(first.normal, first.verts[0]);
        let d_last = v3_dot(last.normal, last.verts[0]);
        let cap_first = -(1.0 / 3.0) * first.area * d_first;
        let cap_last = (1.0 / 3.0) * last.area * d_last;
        total += cap_first + cap_last;
    }
    if !total.is_finite() {
        return Err(Refusal::Empty);
    }
    Ok(total.abs())
}

// ---------------------------------------------------------------------------
// Authoring arms: mirror placed-carrier
// ---------------------------------------------------------------------------

/// Applies the part's placed-carrier reflection to a local point.
fn mirror_point(p: [f64; 3], mirror: &str) -> [f64; 3] {
    match mirror {
        "x" => [-p[0], p[1], p[2]],
        "z" => [p[0], p[1], -p[2]],
        _ => [p[0], -p[1], p[2]],
    }
}

/// The world bounding box of one placed solid: mirror the local geometry about
/// the coordinate plane (when the mirror arm recorded one), rotate the local
/// bbox corners by the recorded frame (or by the pure-z `rz` when no frame was
/// recorded), then translate. The AABB of the 8 rotated local corners is the
/// exact world bbox under the orthonormal placement rotation.
fn part_world_bbox(part: &PartSpec) -> Result<[[f64; 3]; 2], Refusal> {
    let local = solid_local_bbox(&part.solid)?;
    let frame = part.rotation;
    let rz = part.rz.to_radians();
    let cos = rz.cos();
    let sin = rz.sin();
    let corners = [
        [local[0][0], local[0][1], local[0][2]],
        [local[0][0], local[0][1], local[1][2]],
        [local[0][0], local[1][1], local[0][2]],
        [local[0][0], local[1][1], local[1][2]],
        [local[1][0], local[0][1], local[0][2]],
        [local[1][0], local[0][1], local[1][2]],
        [local[1][0], local[1][1], local[0][2]],
        [local[1][0], local[1][1], local[1][2]],
    ];
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for corner in corners {
        // The mirror placed-carrier reflects the local bbox before the rz
        // rotation (a reflection is a congruence: reflecting the AABB of the
        // local solid is the AABB of the reflected solid).
        let corner = match part.mirror.as_deref() {
            Some(axis) => mirror_point(corner, axis),
            None => corner,
        };
        let point = match frame {
            Some(rotation) => {
                // The recorded full orthonormal frame replaces the pure-z
                // rotation: the AABB of the 8 rotated local corners is exact.
                let p = rotation.apply(corner);
                [p[0] + part.x, p[1] + part.y, p[2] + part.z]
            }
            None => {
                let (x, y) = if rz == 0.0 {
                    (corner[0], corner[1])
                } else {
                    (
                        corner[0] * cos - corner[1] * sin,
                        corner[0] * sin + corner[1] * cos,
                    )
                };
                [x + part.x, y + part.y, corner[2] + part.z]
            }
        };
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok([min, max])
}

/// The boolean-carrier class of one unplaced solid (the facade's taxonomy).
/// The LOCAL geometry classifies, never a placement: a placed operand's frame
/// composes after the dispatch, so admission is placement-blind.
fn solid_carrier_class(solid: &SolidSpec) -> CarrierClass {
    match solid {
        SolidSpec::Box { .. }
        | SolidSpec::Cylinder { .. }
        | SolidSpec::Sphere { .. }
        | SolidSpec::Prism { .. }
        | SolidSpec::TrimPrism { .. } => CarrierClass::Canonical,
        SolidSpec::Torus { .. } => CarrierClass::Torus,
        SolidSpec::Lathe { profile, .. } => {
            if profile
                .iter()
                .any(|edge| matches!(edge, LatheEdge::Spline { .. }))
            {
                CarrierClass::Revolved
            } else {
                CarrierClass::Canonical
            }
        }
        SolidSpec::Loft { .. } => CarrierClass::Swept,
    }
}

/// The carrier class of one boolean operand node. A placed part classifies by
/// its LOCAL solid (the dispatch never sees a placement); a boolean operand is
/// the recorded depth-2 open composition cell and refuses typed; a group
/// classifies by its first part in script order.
fn node_carrier_class(node: &TreeNode) -> Result<CarrierClass, Refusal> {
    match node {
        TreeNode::Part { part } => Ok(solid_carrier_class(&part.solid)),
        TreeNode::Group { group } => {
            for child in group {
                match node_carrier_class(child) {
                    Ok(class) => return Ok(class),
                    Err(Refusal::Empty) => continue,
                    Err(other) => return Err(other),
                }
            }
            Err(Refusal::Empty)
        }
        // Depth-1 only: a boolean of a boolean result is the recorded open
        // composition cell and refuses typed rather than silently recursing.
        TreeNode::Boolean { .. } => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// Dispatches one recorded boolean row through the facade's boolean entry
/// (the admission consult the binding's `boolean_dispatch` composes) and
/// returns the routed event. The dispatch is placement-blind: it sees each
/// operand's LOCAL carrier class. A refused pair keeps its typed, localized
/// envelope case (`NonCanonicalCarrier` for the not-yet-admitted carrier
/// class pair, `ContactReductionDeferred` for a torus carrier in a swept pair).
fn dispatch_boolean(node: &BooleanNode) -> Result<Option<SweptBooleanEvent>, Refusal> {
    let base = node_carrier_class(&node.a)?;
    let tool = node_carrier_class(&node.b)?;
    match crate::facade::dispatch_swept_carrier_boolean(base, tool, node.mode) {
        BooleanPairVerdict::CanonicalLanded => Ok(None),
        BooleanPairVerdict::Routed(route) => Ok(Some(SweptBooleanEvent {
            mode: route.mode,
            base: route.base,
            tool: route.tool,
        })),
        BooleanPairVerdict::Refused(refusal) => Err(Refusal::UnsupportedEnvelope(refusal.case)),
    }
}

/// Collects the routed boolean events of a tree, in script order, and
/// validates every row (a refused pair propagates its typed envelope case).
fn collect_boolean_events(
    node: &TreeNode,
    events: &mut Vec<SweptBooleanEvent>,
) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { .. } => Ok(()),
        TreeNode::Group { group } => {
            for child in group {
                collect_boolean_events(child, events)?;
            }
            Ok(())
        }
        TreeNode::Boolean { boolean } => {
            if let Some(event) = dispatch_boolean(boolean)? {
                events.push(event);
            }
            Ok(())
        }
    }
}

/// Measures the submitted tree with the recorded OCC top-node semantics.
pub fn tree_facts(root: &TreeNode) -> Result<Facts, Refusal> {
    let mut count = 0u64;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    count_and_union(root, &mut count, &mut min, &mut max)?;
    if count == 0 || !min[0].is_finite() {
        return Err(Refusal::Empty);
    }
    let volume = top_volume(root)?;
    let seam_mismatch = top_seam_mismatch(root)?;
    let mut boolean_events = Vec::new();
    collect_boolean_events(root, &mut boolean_events)?;
    Ok(Facts {
        solid_count: count,
        volume,
        bbox: [min, max],
        seam_mismatch,
        boolean_events,
    })
}

/// The seam certificate of a single closed halo loft row, when the top node is
/// exactly one part carrying a `closed` loft. A non-closing chain refuses
/// typed with the mismatch evidence; open chains and non-loft rows carry no
/// seam (`None`).
fn top_seam_mismatch(root: &TreeNode) -> Result<Option<f64>, Refusal> {
    match root {
        TreeNode::Part { part } => match &part.solid {
            SolidSpec::Loft { sections, closed } => {
                if let Ok(validated) = loft_sections(sections) {
                    let mismatch = loft_seam_mismatch(&validated, *closed)?;
                    if *closed {
                        Ok(Some(mismatch))
                    } else {
                        Ok(None)
                    }
                } else if *closed {
                    // A closed halo chain has no smooth certificate: the seam
                    // identity is only defined over the recorded line carrier.
                    Err(open_smooth_loft())
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        },
        // A routed boolean row measures its base operand (the tool is a
        // certificate witness); the dispatch validates the pair.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            top_seam_mismatch(&boolean.a)
        }
        TreeNode::Group { .. } => Ok(None),
    }
}

/// Recursively counts every part and unions every part's world bbox.
fn count_and_union(
    node: &TreeNode,
    count: &mut u64,
    min: &mut [f64; 3],
    max: &mut [f64; 3],
) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            *count = count.saturating_add(1);
            let bbox = part_world_bbox(part)?;
            for axis in 0..3 {
                min[axis] = min[axis].min(bbox[0][axis]);
                max[axis] = max[axis].max(bbox[1][axis]);
            }
            Ok(())
        }
        // A routed boolean row's measured geometry is its base operand; the
        // dispatch validates the pair (a refused pair propagates its typed
        // envelope case) and the routed event is collected separately.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            count_and_union(&boolean.a, count, min, max)
        }
        TreeNode::Group { group } => {
            for child in group {
                count_and_union(child, count, min, max)?;
            }
            Ok(())
        }
    }
}

/// The measured volume of a top node: a group sums only its immediate child
/// parts (nested groups contribute nothing, mirroring the OCC `Compound`
/// measurement the reference was recorded with); a part is its own volume.
fn top_volume(node: &TreeNode) -> Result<f64, Refusal> {
    match node {
        TreeNode::Part { part } => solid_volume(&part.solid),
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            top_volume(&boolean.a)
        }
        TreeNode::Group { group } => {
            let mut volume = 0.0;
            for child in group {
                match child {
                    TreeNode::Part { part } => volume += solid_volume(&part.solid)?,
                    TreeNode::Boolean { boolean } => {
                        dispatch_boolean(boolean)?;
                        volume += top_volume(&boolean.a)?;
                    }
                    TreeNode::Group { .. } => {}
                }
            }
            Ok(volume)
        }
    }
}

// ---------------------------------------------------------------------------
// STL meshing
// ---------------------------------------------------------------------------

/// One triangle: nine `f64` coordinates (three `x y z` vertices), world
/// space.
pub(crate) type Triangle = [f64; 9];

/// The angular resolution of the mesh (fixed, deterministic).
const MESH_SEGMENTS: usize = 64;

/// Generates the world-space triangle soup of every part in the tree.
fn tree_mesh(root: &TreeNode) -> Result<Vec<Triangle>, Refusal> {
    let mut triangles = Vec::new();
    append_node_mesh(root, &mut triangles)?;
    Ok(triangles)
}

fn append_node_mesh(node: &TreeNode, out: &mut Vec<Triangle>) -> Result<(), Refusal> {
    match node {
        TreeNode::Part { part } => {
            let local = solid_mesh(&part.solid)?;
            // The mirror placed-carrier reflects the local geometry before
            // the rz rotation and translation (no geometry recomputation).
            let reflected: Vec<Triangle> = local
                .iter()
                .map(|tri| match part.mirror.as_deref() {
                    Some(axis) => reflect_triangle(*tri, axis),
                    None => *tri,
                })
                .collect();
            if let Some(frame) = part.rotation {
                // The recorded full orthonormal frame transforms the mesh
                // per-vertex (never recomputed geometry).
                for tri in reflected {
                    out.push(place_triangle_frame(tri, frame, part.x, part.y, part.z));
                }
            } else {
                let rz = part.rz.to_radians();
                let cos = rz.cos();
                let sin = rz.sin();
                for tri in reflected {
                    out.push(place_triangle(tri, cos, sin, part.x, part.y, part.z));
                }
            }
            Ok(())
        }
        // A routed boolean row's mesh is its base operand's mesh; the
        // dispatch validates the pair.
        TreeNode::Boolean { boolean } => {
            dispatch_boolean(boolean)?;
            append_node_mesh(&boolean.a, out)
        }
        TreeNode::Group { group } => {
            for child in group {
                append_node_mesh(child, out)?;
            }
            Ok(())
        }
    }
}

/// Reflects one local triangle across the recorded mirror coordinate plane.
fn reflect_triangle(tri: Triangle, axis: &str) -> Triangle {
    let mut reflected = [0.0f64; 9];
    for vertex in 0..3 {
        let p = [tri[vertex * 3], tri[vertex * 3 + 1], tri[vertex * 3 + 2]];
        let q = mirror_point(p, axis);
        reflected[vertex * 3] = q[0];
        reflected[vertex * 3 + 1] = q[1];
        reflected[vertex * 3 + 2] = q[2];
    }
    reflected
}

/// Translates a triangle by the part's world frame.
fn place_triangle(tri: Triangle, cos: f64, sin: f64, x: f64, y: f64, z: f64) -> Triangle {
    let mut placed = [0.0f64; 9];
    for vertex in 0..3 {
        let vx = tri[vertex * 3];
        let vy = tri[vertex * 3 + 1];
        let vz = tri[vertex * 3 + 2];
        let (px, py) = if sin == 0.0 && cos == 1.0 {
            (vx, vy)
        } else {
            (vx * cos - vy * sin, vx * sin + vy * cos)
        };
        placed[vertex * 3] = px + x;
        placed[vertex * 3 + 1] = py + y;
        placed[vertex * 3 + 2] = vz + z;
    }
    placed
}

/// Translates a triangle by the part's world frame carrying a recorded full
/// orthonormal rotation (the frame replaces the pure-z rotation path).
fn place_triangle_frame(tri: Triangle, frame: RotationFrame, x: f64, y: f64, z: f64) -> Triangle {
    let mut placed = [0.0f64; 9];
    for vertex in 0..3 {
        let vx = tri[vertex * 3];
        let vy = tri[vertex * 3 + 1];
        let vz = tri[vertex * 3 + 2];
        let p = frame.apply([vx, vy, vz]);
        placed[vertex * 3] = p[0] + x;
        placed[vertex * 3 + 1] = p[1] + y;
        placed[vertex * 3 + 2] = p[2] + z;
    }
    placed
}

/// The local (origin-frame) triangle soup of one solid.
fn solid_mesh(solid: &SolidSpec) -> Result<Vec<Triangle>, Refusal> {
    match solid {
        SolidSpec::Box {
            length,
            width,
            height,
        } => Ok(box_mesh(*length, *width, *height)),
        SolidSpec::Cylinder {
            radius,
            height,
            axis,
        } => {
            let about_z = cylinder_z_mesh(*radius, *height)?;
            Ok(match axis.as_str() {
                // the corpus's (0, 90, 0) rotation: z axis -> x axis
                "x" => about_z.iter().map(|tri| rotate_mesh_y90(*tri)).collect(),
                // the corpus's (90, 0, 0) rotation: z axis -> y axis
                "y" => about_z
                    .iter()
                    .map(|tri| rotate_mesh_x_neg90(*tri))
                    .collect(),
                _ => about_z,
            })
        }
        SolidSpec::Sphere { radius } => sphere_mesh(*radius),
        SolidSpec::Torus { major, minor } => torus_mesh(*major, *minor),
        SolidSpec::Lathe { profile, .. } => lathe_mesh(profile),
        SolidSpec::Prism {
            profile,
            amount,
            both,
        } => prism_mesh(profile, *amount, *both),
        SolidSpec::TrimPrism {
            profile,
            amount,
            both,
            trim_curve,
            trim_net,
            tolerance,
        } => crate::python::binding::trim_extrude_solid(
            profile, *amount, *both, trim_curve, trim_net, *tolerance,
        )
        .map(|facts| facts.mesh)
        .map_err(trim_binding_error_to_refusal),
        SolidSpec::Loft { sections, closed } => loft_mesh(sections, *closed),
    }
}

/// The local mesh of an extruded prism: the profile boundary swept between the
/// two end planes plus cap fans. Deterministic, closed, no duplicate vertices.
pub(crate) fn prism_mesh(
    profile: &[ProfileEdge],
    amount: f64,
    both: bool,
) -> Result<Vec<Triangle>, Refusal> {
    let geom = prism_geom(profile, amount, both)?;
    let norm = geom.loop3.normal;
    let verts = &geom.loop3.verts;
    let count = verts.len();
    let centroid = {
        let mut c = [0.0f64; 3];
        for v in verts {
            c[0] += v[0];
            c[1] += v[1];
            c[2] += v[2];
        }
        let k = 1.0 / count as f64;
        [c[0] * k, c[1] * k, c[2] * k]
    };
    let bottom: Vec<[f64; 3]> = verts
        .iter()
        .map(|v| {
            [
                v[0] + geom.t_lo * norm[0],
                v[1] + geom.t_lo * norm[1],
                v[2] + geom.t_lo * norm[2],
            ]
        })
        .collect();
    let top: Vec<[f64; 3]> = verts
        .iter()
        .map(|v| {
            [
                v[0] + geom.t_hi * norm[0],
                v[1] + geom.t_hi * norm[1],
                v[2] + geom.t_hi * norm[2],
            ]
        })
        .collect();
    let cb = [
        centroid[0] + geom.t_lo * norm[0],
        centroid[1] + geom.t_lo * norm[1],
        centroid[2] + geom.t_lo * norm[2],
    ];
    let ct = [
        centroid[0] + geom.t_hi * norm[0],
        centroid[1] + geom.t_hi * norm[1],
        centroid[2] + geom.t_hi * norm[2],
    ];
    let mut out = Vec::new();
    for i in 0..count {
        let j = (i + 1) % count;
        push_quad(&mut out, bottom[i], bottom[j], top[j], top[i]);
    }
    for i in 0..count {
        let j = (i + 1) % count;
        push_tri(&mut out, cb, bottom[j], bottom[i]);
        push_tri(&mut out, ct, top[i], top[j]);
    }
    Ok(out)
}

/// The local mesh of a loft chain: ruled quads between matching vertices of
/// consecutive sections, plus cap fans on the two end sections of an open
/// chain. A closed halo row's last station repeats the first station, so the
/// side surface closes on itself and needs no caps.
fn loft_mesh(sections: &[Vec<ProfileEdge>], closed: bool) -> Result<Vec<Triangle>, Refusal> {
    // The certified spline-section class meshes through the reconstructed
    // ruled surface; a closed smooth chain has no certificate.
    if loft_sections(sections).is_err() {
        if closed {
            return Err(open_smooth_loft());
        }
        return spline_loft_mesh(sections);
    }
    let validated = loft_sections(sections)?;
    let loops = &validated.loops;
    let mut out = Vec::new();
    let seg_count = loops.len() - 1;
    for j in 0..seg_count {
        let a = &loops[j];
        let b = &loops[j + 1];
        let n = a.verts.len();
        for i in 0..n {
            let k = (i + 1) % n;
            push_quad(&mut out, a.verts[i], a.verts[k], b.verts[k], b.verts[i]);
        }
    }
    if !closed {
        let first = &loops[0];
        let last = loops.last().ok_or(Refusal::Empty)?;
        let c_first = loop_centroid(first);
        let c_last = loop_centroid(last);
        let n = first.verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            push_tri(&mut out, c_first, first.verts[i], first.verts[j]);
        }
        let n = last.verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            push_tri(&mut out, c_last, last.verts[i], last.verts[j]);
        }
    }
    Ok(out)
}

fn loop_centroid(loop3: &ProfileLoop) -> [f64; 3] {
    let n = loop3.verts.len();
    let mut c = [0.0f64; 3];
    for v in &loop3.verts {
        c[0] += v[0];
        c[1] += v[1];
        c[2] += v[2];
    }
    let k = 1.0 / n as f64;
    [c[0] * k, c[1] * k, c[2] * k]
}

/// Applies the rotation used for x-axis cylinders: `(x, y, z) -> (z, y, -x)`
/// (the sign is immaterial for a symmetric cylinder).
fn rotate_mesh_y90(tri: Triangle) -> Triangle {
    let mut out = [0.0f64; 9];
    for vertex in 0..3 {
        let x = tri[vertex * 3];
        let y = tri[vertex * 3 + 1];
        let z = tri[vertex * 3 + 2];
        out[vertex * 3] = z;
        out[vertex * 3 + 1] = y;
        out[vertex * 3 + 2] = -x;
    }
    out
}

/// Applies the rotation used for y-axis cylinders: `(x, y, z) -> (x, z, -y)`.
fn rotate_mesh_x_neg90(tri: Triangle) -> Triangle {
    let mut out = [0.0f64; 9];
    for vertex in 0..3 {
        let x = tri[vertex * 3];
        let y = tri[vertex * 3 + 1];
        let z = tri[vertex * 3 + 2];
        out[vertex * 3] = x;
        out[vertex * 3 + 1] = z;
        out[vertex * 3 + 2] = -y;
    }
    out
}

/// A unit box centered on the origin: six faces, two triangles each.
fn box_mesh(length: f64, width: f64, height: f64) -> Vec<Triangle> {
    let (hx, hy, hz) = (length / 2.0, width / 2.0, height / 2.0);
    let c = [
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];
    let mut out = Vec::new();
    // +z, -z, +x, -x, +y, -y faces (each two triangles)
    push_quad(&mut out, c[4], c[5], c[6], c[7]);
    push_quad(&mut out, c[0], c[3], c[2], c[1]);
    push_quad(&mut out, c[1], c[5], c[6], c[2]);
    push_quad(&mut out, c[0], c[4], c[7], c[3]);
    push_quad(&mut out, c[3], c[7], c[6], c[2]);
    push_quad(&mut out, c[0], c[1], c[5], c[4]);
    out
}

/// A z-axis cylinder mesh (radius `r`, height `h`, centered on the origin).
fn cylinder_z_mesh(radius: f64, height: f64) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || height <= 0.0 || !radius.is_finite() || !height.is_finite() {
        return Err(Refusal::Empty);
    }
    let ring = ring_points(radius, MESH_SEGMENTS);
    let z_top = height / 2.0;
    let z_bottom = -height / 2.0;
    let mut out = Vec::new();
    for segment in 0..MESH_SEGMENTS {
        let next = segment + 1;
        let (x0, y0) = ring_point(&ring, segment);
        let (x1, y1) = ring_point(&ring, next);
        push_quad(
            &mut out,
            [x0, y0, z_bottom],
            [x1, y1, z_bottom],
            [x1, y1, z_top],
            [x0, y0, z_top],
        );
    }
    for z in [z_top, z_bottom] {
        for segment in 0..MESH_SEGMENTS {
            let next = segment + 1;
            let (x0, y0) = ring_point(&ring, segment);
            let (x1, y1) = ring_point(&ring, next);
            push_tri(&mut out, [0.0, 0.0, z], [x0, y0, z], [x1, y1, z]);
        }
    }
    Ok(out)
}

fn ring_point(ring: &[(f64, f64)], index: usize) -> (f64, f64) {
    if index == ring.len() {
        ring.first().copied().unwrap_or((0.0, 0.0))
    } else {
        ring.get(index).copied().unwrap_or((0.0, 0.0))
    }
}

fn ring_points(radius: f64, count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|segment| {
            let angle = TAU * segment as f64 / count as f64;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// A UV sphere (meridian `MESH_SEGMENTS`, `MESH_SEGMENTS/2` latitude bands,
/// poles included as degenerate rings so the fanning is uniform).
fn sphere_mesh(radius: f64) -> Result<Vec<Triangle>, Refusal> {
    if radius <= 0.0 || !radius.is_finite() {
        return Err(Refusal::Empty);
    }
    let bands = MESH_SEGMENTS / 2;
    // ring `i` sits at latitude pi*i/bands; rings 0 and `bands` are the poles.
    let rings: Vec<Vec<(f64, f64, f64)>> = (0..=bands)
        .map(|i| {
            let theta = std::f64::consts::PI * i as f64 / bands as f64;
            let z = radius * theta.cos();
            let r = radius * theta.sin();
            (0..MESH_SEGMENTS)
                .map(|segment| {
                    let angle = TAU * segment as f64 / MESH_SEGMENTS as f64;
                    (r * angle.cos(), r * angle.sin(), z)
                })
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for band in 0..bands {
        let lower = ring3(&rings, band);
        let upper = ring3(&rings, band + 1);
        for segment in 0..MESH_SEGMENTS {
            let next = segment + 1;
            let a = ring3_at(lower, segment);
            let b = ring3_at(lower, next);
            let c = ring3_at(upper, next);
            let d = ring3_at(upper, segment);
            push_quad(&mut out, a, b, c, d);
        }
    }
    Ok(out)
}

fn ring3(rings: &[Vec<(f64, f64, f64)>], index: usize) -> &[(f64, f64, f64)] {
    rings.get(index).map(|ring| ring.as_slice()).unwrap_or(&[])
}

fn ring3_at(ring: &[(f64, f64, f64)], index: usize) -> [f64; 3] {
    if index == ring.len() {
        ring.first()
            .map(|(x, y, z)| [*x, *y, *z])
            .unwrap_or([0.0, 0.0, 0.0])
    } else {
        ring.get(index)
            .map(|(x, y, z)| [*x, *y, *z])
            .unwrap_or([0.0, 0.0, 0.0])
    }
}

/// A torus about the z axis centered on the origin.
fn torus_mesh(major: f64, minor: f64) -> Result<Vec<Triangle>, Refusal> {
    if major <= 0.0 || minor <= 0.0 || !major.is_finite() || !minor.is_finite() {
        return Err(Refusal::Empty);
    }
    let mut out = Vec::new();
    for ring_segment in 0..MESH_SEGMENTS {
        let ring_a = TAU * ring_segment as f64 / MESH_SEGMENTS as f64;
        let ring_b = TAU * (ring_segment + 1) as f64 / MESH_SEGMENTS as f64;
        for tube_segment in 0..MESH_SEGMENTS {
            let tube_a = TAU * tube_segment as f64 / MESH_SEGMENTS as f64;
            let tube_b = TAU * (tube_segment + 1) as f64 / MESH_SEGMENTS as f64;
            let p00 = torus_point(major, minor, ring_a, tube_a);
            let p10 = torus_point(major, minor, ring_b, tube_a);
            let p11 = torus_point(major, minor, ring_b, tube_b);
            let p01 = torus_point(major, minor, ring_a, tube_b);
            push_quad(&mut out, p00, p10, p11, p01);
        }
    }
    Ok(out)
}

fn torus_point(major: f64, minor: f64, ring_angle: f64, tube_angle: f64) -> [f64; 3] {
    let ring_radius = major + minor * tube_angle.cos();
    [
        ring_radius * ring_angle.cos(),
        ring_radius * ring_angle.sin(),
        minor * tube_angle.sin(),
    ]
}

/// The number of linear subdivisions per reconstructed spline span in the
/// lathe mesh (fixed, deterministic).
const SPLINE_MESH_STEPS: usize = 4;

/// A full-revolution lathe mesh: sample the profile boundary (line edges
/// exactly, spline edges at the fixed subdivision of each reconstructed span)
/// and sweep each consecutive sample segment over the angular segments. Each
/// band is a conical quad strip; no separate caps are needed because a closed
/// profile's swept boundary covers the whole surface. For an all-line profile
/// the ring is exactly the vertex loop, so the mesh is identical to the landed
/// line-profile mesh.
fn lathe_mesh(profile: &[LatheEdge]) -> Result<Vec<Triangle>, Refusal> {
    let ring = profile_ring(profile)?;
    sweep_ring_mesh(&ring)
}

/// Appends `p` to the ring unless it equals the current last ring point.
fn push_ring_point(ring: &mut Vec<[f64; 2]>, p: [f64; 2]) {
    let duplicate = ring.last().is_some_and(|last| *last == p);
    if !duplicate {
        ring.push(p);
    }
}

/// The ordered ring of boundary samples of the profile, one point per sample
/// (no duplicate closing point).
fn profile_ring(profile: &[LatheEdge]) -> Result<Vec<[f64; 2]>, Refusal> {
    check_profile_edges(profile)?;
    let mut ring: Vec<[f64; 2]> = Vec::new();
    for edge in profile {
        match edge {
            LatheEdge::Line { a, b } => {
                push_ring_point(&mut ring, *a);
                push_ring_point(&mut ring, *b);
            }
            LatheEdge::Spline { points } => {
                for sample in spline_ring_samples(points)? {
                    push_ring_point(&mut ring, sample);
                }
            }
        }
    }
    // Drop the trailing duplicate of the ring start, if any.
    if ring.len() > 1 {
        let first = ring[0];
        if let Some(last) = ring.last() {
            if *last == first {
                ring.pop();
            }
        }
    }
    Ok(ring)
}

/// The sampled polyline of one spline profile edge: its exact samples at the
/// reconstructed span joints plus the fixed interior subdivision of each span.
fn spline_ring_samples(points: &[[f64; 2]]) -> Result<Vec<[f64; 2]>, Refusal> {
    let spans = spline_spans(points)?;
    let mut out = Vec::new();
    out.push(points[0]);
    for (i, span) in spans.iter().enumerate() {
        for s in 1..SPLINE_MESH_STEPS {
            let u = s as f64 / SPLINE_MESH_STEPS as f64;
            out.push(eval_span(span, u));
        }
        if i + 1 < points.len() {
            out.push(points[i + 1]);
        }
    }
    Ok(out)
}

/// Evaluates a reconstructed span at `u in [0, 1]`.
fn eval_span(span: &SpanPoly, u: f64) -> [f64; 2] {
    [
        span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3])),
        span.z[0] + u * (span.z[1] + u * (span.z[2] + u * span.z[3])),
    ]
}

/// Sweeps the boundary ring over the angular segments.
fn sweep_ring_mesh(ring: &[[f64; 2]]) -> Result<Vec<Triangle>, Refusal> {
    if ring.len() < 3 {
        return Err(Refusal::Empty);
    }
    for point in ring {
        if point[0] < 0.0 || !point[0].is_finite() || !point[1].is_finite() {
            return Err(Refusal::Empty);
        }
    }
    let mut out = Vec::new();
    for ring_index in 0..ring.len() {
        let next = (ring_index + 1) % ring.len();
        let x0 = ring[ring_index][0];
        let z0 = ring[ring_index][1];
        let x1 = ring[next][0];
        let z1 = ring[next][1];
        for segment in 0..MESH_SEGMENTS {
            let angle_a = TAU * segment as f64 / MESH_SEGMENTS as f64;
            let angle_b = TAU * (segment + 1) as f64 / MESH_SEGMENTS as f64;
            let a = [x0 * angle_a.cos(), x0 * angle_a.sin(), z0];
            let b = [x0 * angle_b.cos(), x0 * angle_b.sin(), z0];
            let c = [x1 * angle_b.cos(), x1 * angle_b.sin(), z1];
            let d = [x1 * angle_a.cos(), x1 * angle_a.sin(), z1];
            push_quad(&mut out, a, b, c, d);
        }
    }
    Ok(out)
}

/// Appends one triangle.
fn push_tri(out: &mut Vec<Triangle>, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
    out.push([a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
}

/// Appends a quad as two triangles.
fn push_quad(out: &mut Vec<Triangle>, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) {
    push_tri(out, a, b, c);
    push_tri(out, a, c, d);
}

// ---------------------------------------------------------------------------
// STL writer
// ---------------------------------------------------------------------------

/// Writes the tree's triangle soup as a binary STL file at `path`, returning
/// the triangle count. Binary STL: 80-byte header, u32 triangle count, then
/// per triangle a normal (zeroed), three vertices as f32 and a u16 attribute.
pub fn write_tree_stl(root: &TreeNode, path: &str) -> Result<u64, Refusal> {
    let triangles = tree_mesh(root)?;
    if triangles.is_empty() {
        return Err(Refusal::Empty);
    }
    let count = u64::try_from(triangles.len()).map_err(|_| Refusal::Empty)?;
    let triangle_count = u32::try_from(triangles.len()).map_err(|_| Refusal::Empty)?;
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&[0u8; 80]);
    bytes.extend_from_slice(&triangle_count.to_le_bytes());
    for tri in &triangles {
        bytes.extend_from_slice(&[0u8; 12]); // normal (0,0,0) as three f32
        for coordinate in tri {
            bytes.extend_from_slice(&(*coordinate as f32).to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|_| Refusal::Empty)?;
    Ok(count)
}

// ---------------------------------------------------------------------------
// pyo3 surface
// ---------------------------------------------------------------------------

/// Parses a submitted construction tree JSON.
fn parse_tree(tree_json: &str) -> Result<TreeNode, Refusal> {
    serde_json::from_str(tree_json).map_err(|_| Refusal::Empty)
}

/// Maps the trim-extrude constructor's typed error back to the executor's
/// typed kernel refusal. The constructor owns the precise kernel kind; the
/// executor's `Refusal` vocabulary carries the typed envelope case the door
/// already re-marshals, so no refusal is ever lost or turned into a panic.
fn trim_binding_error_to_refusal(error: crate::python::binding::BindingError) -> Refusal {
    match error {
        crate::python::binding::BindingError::Malformed(_) => Refusal::Empty,
        crate::python::binding::BindingError::Refusal(_) => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
    }
}

/// The pyo3 measurement entry: takes the construction tree JSON and returns
/// the facts JSON (`{solid_count, volume, bbox}`).
#[pyfunction]
pub fn bd_facts(py: Python<'_>, tree_json: &str) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|refusal| refusal_to_pyerr(py, &refusal))?;
    let outcome = crate::gil::with_kernel_gil_released(py, move || tree_facts(&tree));
    match outcome {
        Ok(facts) => {
            let mut value = serde_json::json!({
                "solid_count": facts.solid_count,
                "volume": facts.volume,
                "bbox": facts.bbox,
            });
            if let Some(mismatch) = facts.seam_mismatch {
                value["seam_mismatch"] = serde_json::json!(mismatch);
            }
            if !facts.boolean_events.is_empty()
                && let Some(map) = value.as_object_mut()
            {
                map.insert(
                    "boolean_events".to_string(),
                    serde_json::json!(facts.boolean_events),
                );
            }
            serde_json::to_string(&value)
                .map_err(|e| PyRuntimeError::new_err(format!("facts serialization failed: {e}")))
        }
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// The pyo3 export entry: writes the construction tree's STL to `path` and
/// returns `{"triangles": n}`.
#[pyfunction]
pub fn bd_stl(py: Python<'_>, tree_json: &str, path: &str) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|refusal| refusal_to_pyerr(py, &refusal))?;
    let owned_path = path.to_string();
    let outcome =
        crate::gil::with_kernel_gil_released(py, move || write_tree_stl(&tree, &owned_path));
    match outcome {
        Ok(triangles) => serde_json::to_string(&serde_json::json!({ "triangles": triangles }))
            .map_err(|e| PyRuntimeError::new_err(format!("stl serialization failed: {e}"))),
        Err(refusal) => Err(refusal_to_pyerr(py, &refusal)),
    }
}

/// Maps a typed kernel refusal to the mapped Python exception (the landed
/// `Refused`/`Unresolved` classes), never a bare `Exception`.
fn refusal_to_pyerr(py: Python<'_>, refusal: &Refusal) -> PyErr {
    let marshaled = Marshaled::from_refusal(refusal);
    let payload_json = match &marshaled.payload {
        MarshaledPayload::Refused(payload) => {
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        }
        MarshaledPayload::Unresolved(payload) => {
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        }
    };
    let instance = match marshaled.class {
        ExceptionClass::Refused => python::marshal_refused(py, &payload_json),
        ExceptionClass::Unresolved => python::marshal_unresolved(py, &payload_json),
    };
    match instance {
        Ok(value) => PyErr::from_value(value.bind(py).clone()),
        Err(e) => e,
    }
}

/// A convenience parse used by tests and by the pyo3 surface (kept `pub` so
/// the in-crate suite can construct trees from literals).
#[cfg(test)]
mod tests {
    use super::*;

    fn part(solid: SolidSpec, x: f64, y: f64, z: f64) -> TreeNode {
        TreeNode::Part {
            part: PartSpec {
                solid,
                x,
                y,
                z,
                rz: 0.0,
                rotation: None,
                mirror: None,
            },
        }
    }

    /// Builds a placed part carrying the mirror placed-carrier transform.
    fn mirrored_part(solid: SolidSpec, axis: &str) -> TreeNode {
        TreeNode::Part {
            part: PartSpec {
                solid,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                rz: 0.0,
                rotation: None,
                mirror: Some(axis.to_string()),
            },
        }
    }

    /// Builds an all-line lathe from a closed vertex loop (the census form the
    /// door's line-profile revolve produces).
    fn line_lathe(points: &[[f64; 2]], arc_deg: f64) -> SolidSpec {
        let mut profile = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            let b = if i + 1 < points.len() {
                points[i + 1]
            } else {
                points[0]
            };
            profile.push(LatheEdge::Line { a: points[i], b });
        }
        SolidSpec::Lathe { profile, arc_deg }
    }

    #[test]
    fn primitive_facts_are_analytic() {
        let tree = TreeNode::Group {
            group: vec![
                part(
                    SolidSpec::Cylinder {
                        radius: 150.0,
                        height: 770.0,
                        axis: "z".to_string(),
                    },
                    470.0,
                    0.0,
                    1515.0,
                ),
                part(
                    SolidSpec::Torus {
                        major: 118.0,
                        minor: 54.0,
                    },
                    470.0,
                    0.0,
                    1640.0,
                ),
                part(SolidSpec::Sphere { radius: 118.0 }, 470.0, 0.0, 1900.0),
                part(
                    SolidSpec::Box {
                        length: 190.0,
                        width: 46.0,
                        height: 16.0,
                    },
                    250.0,
                    0.0,
                    1500.0,
                ),
            ],
        };
        let facts = tree_facts(&tree).expect("analytic primitives are in envelope");
        let expected = std::f64::consts::PI * 150.0 * 150.0 * 770.0
            + 2.0 * std::f64::consts::PI * std::f64::consts::PI * 118.0 * 54.0 * 54.0
            + 4.0 / 3.0 * std::f64::consts::PI * 118.0f64.powi(3)
            + 190.0 * 46.0 * 16.0;
        assert!((facts.volume - expected).abs() / expected < 1e-12);
        assert_eq!(facts.solid_count, 4);
        // Union bbox across the four parts (box 155..345 in x, torus ±172 in
        // y, cylinder 1130..1900 and sphere 1782..2018 in z).
        assert_eq!(facts.bbox[0], [155.0, -172.0, 1130.0]);
        assert_eq!(facts.bbox[1], [642.0, 172.0, 2018.0]);
    }

    #[test]
    fn lathe_facts_match_occt_frustum_volume() {
        let housing = line_lathe(
            &[
                [0.001, 1040.0],
                [95.0, 1110.0],
                [120.0, 1130.0],
                [0.001, 1130.0],
            ],
            360.0,
        );
        let facts = tree_facts(&part(housing, 470.0, 0.0, 0.0))
            .expect("a full-arc line-profile lathe is in envelope");
        // OCC records the same turbine-housing solid at 1,390,947.111 mm^3.
        assert!((facts.volume - 1_390_947.111).abs() / 1_390_947.111 < 1e-9);
        assert_eq!(facts.bbox[0], [350.0, -120.0, 1040.0]);
        assert_eq!(facts.bbox[1], [590.0, 120.0, 1130.0]);
    }

    #[test]
    fn partial_arc_lathe_refuses_typed() {
        let partial = line_lathe(
            &[[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]],
            270.0,
        );
        let refusal = tree_facts(&part(partial, 0.0, 0.0, 0.0))
            .expect_err("a partial-arc lathe is outside this executor's arm");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn stl_writer_emits_binary_stl() {
        let tree = TreeNode::Group {
            group: vec![part(SolidSpec::Sphere { radius: 10.0 }, 1.0, 2.0, 3.0)],
        };
        let dir = std::env::temp_dir().join(format!("truck123d_bd_stl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir must be creatable"); // H-3: test scratch
        let path = dir.join("sphere.stl");
        let triangles = write_tree_stl(&tree, &path.to_string_lossy()).expect("stl writes");
        assert!(triangles > 0);
        let bytes = std::fs::read(&path).expect("stl reads back"); // H-3: output of our own writer
        assert!(bytes.len() > 84);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The census vocabulary the drop-in answers name-for-name (spec 8 rows
    /// S1–S8's corpus-facing names). Every name must resolve on the truck
    /// drop-in module surface (`corpus/ttc/door.py` installs it under
    /// `cadgen.build123d` in the `--engine truck` regime) — never a silent
    /// OCC fallback.
    const CENSUS_NAMES: &[&str] = &[
        "Box",
        "Cylinder",
        "Sphere",
        "Torus",
        "Compound",
        "Vector",
        "Location",
        "Axis",
        "Edge",
        "Wire",
        "Face",
        "revolve",
        "extrude",
        "sweep",
        "loft",
        "fillet",
        "chamfer",
        "mirror",
        "Spline",
        "Circle",
        "Polygon",
        "Polyline",
        "make_face",
        "export_stl",
        "Plane",
        "Pos",
        "Rotation",
        "Mode",
    ];

    #[test]
    fn drop_in_module_answers_census_vocabulary_name_for_name() {
        // The names the drop-in must answer, name for name (spec 8): every
        // census name must be wired on the truck surface source AND map to an
        // executor/facade arm in this module (a geometry arm, or the typed
        // refusal path for an unmapped carrier form).
        let door_source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../corpus/ttc/door.py"
        ))
        .expect("the corpus door source must be readable"); // H-3: fixture under test, read-only
        for name in CENSUS_NAMES {
            assert!(
                door_source.contains(name),
                "census name {name} must be wired on the truck drop-in module surface"
            );
        }
        // The truck regime installs the truck bridge module in place of
        // build123d and the OCC baseline stays the untouched reference path.
        assert!(door_source.contains("def install_truck_alias"));
        assert!(door_source.contains("install_cadgen_alias"));
        assert!(door_source.contains("--engine"));
        assert_eq!(
            door_source.matches("def install_cadgen_alias").count(),
            1,
            "anchor A1: the OCC alias installer stays a single definition"
        );
        // The geometry bridge entries this module registers on truck123d are
        // the ones the truck regime drives (facts + STL).
        assert!(door_source.contains("bd_facts"));
        assert!(door_source.contains("bd_stl"));
        // Signature discipline: the drop-in never wraps/improves a census
        // name beyond the corpus call patterns (no extra convenience sugar in
        // the truck regime beyond the answered names).
        assert!(door_source.contains("_TRUCK_NAMES"));
    }

    #[test]
    fn unsupported_carrier_refusal_maps_to_the_refused_exception_class() {
        // A refusal on an unmapped/unsupported carrier form is typed end to
        // end: the executor produces `UnsupportedEnvelope(NonCanonicalCarrier)`
        // (see partial_arc_lathe_refuses_typed) and the marshal layer maps that
        // refusal to the `Refused` Python exception class — never a panic and
        // never a bare Exception.
        let partial = line_lathe(
            &[[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]],
            270.0,
        );
        let refusal = tree_facts(&part(partial, 0.0, 0.0, 0.0))
            .expect_err("the partial-arc lathe is outside this executor's arm");
        let marshaled = crate::marshal::Marshaled::from_refusal(&refusal);
        assert_eq!(
            marshaled.class,
            crate::marshal::ExceptionClass::Refused,
            "an unsupported-carrier refusal must surface as the mapped Refused class"
        );
    }

    // -----------------------------------------------------------------------
    // Spline-profile lathe facts
    // -----------------------------------------------------------------------

    /// A synthetic curved shell profile: two dome spline arcs (inner and outer,
    /// the outer +10 in radius over the same samples) joined by radial caps at
    /// `z = 0`, mirroring the corpus's `revolved_shell` profile shape.
    fn dome_shell_profile() -> Vec<LatheEdge> {
        let inner = [[20.0, 0.0], [38.0, 18.0], [62.0, 18.0], [80.0, 0.0]];
        let outer = [[90.0, 0.0], [72.0, 18.0], [48.0, 18.0], [30.0, 0.0]];
        vec![
            LatheEdge::Spline {
                points: inner.to_vec(),
            },
            LatheEdge::Line {
                a: inner[3],
                b: outer[0],
            },
            LatheEdge::Spline {
                points: outer.to_vec(),
            },
            LatheEdge::Line {
                a: outer[3],
                b: inner[0],
            },
        ]
    }

    /// The 5-point Gauss-Legendre quadrature of `pi * r(u)^2 * z'(u)` over one
    /// span: exact for the degree-8 integrand, an independent machine check of
    /// the closed-form segment-moment integral.
    fn span_volume_by_quadrature(span: &SpanPoly) -> f64 {
        let nodes = [
            0.906_179_845_938_664,
            0.538_469_310_105_683,
            0.0,
            -0.538_469_310_105_683,
            -0.906_179_845_938_664,
        ];
        let weights = [
            0.236_926_885_056_189,
            0.478_628_670_499_366,
            0.568_888_888_888_889,
            0.478_628_670_499_366,
            0.236_926_885_056_189,
        ];
        let mut acc = 0.0;
        for i in 0..5 {
            let u = (nodes[i] + 1.0) / 2.0;
            let x = span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3]));
            let dzdu = span.z[1] + u * (2.0 * span.z[2] + u * 3.0 * span.z[3]);
            acc += weights[i] * x * x * dzdu / 2.0;
        }
        std::f64::consts::PI * acc
    }

    #[test]
    fn spline_segment_volume_matches_independent_quadrature() {
        // The closed-form segment-moment integration is machine-checked
        // against an independent high-order quadrature of the same
        // reconstructed spans (exact for the degree-8 integrand).
        for edge in dome_shell_profile() {
            if let LatheEdge::Spline { points } = edge {
                let spans = spline_spans(&points).expect("synthetic spline reconstructs");
                for span in &spans {
                    let exact = span_volume(span);
                    let quadrature = span_volume_by_quadrature(span);
                    assert!(
                        (exact - quadrature).abs() / exact.abs().max(1e-12) < 1e-9,
                        "span moment integral drifted: exact {exact} vs quadrature {quadrature}"
                    );
                }
            }
        }
    }

    #[test]
    fn spline_shell_facts_are_not_a_polygon_flattening() {
        // The spline arm integrates the TRUE reconstructed spline. Flattening
        // the spline edges to the sample polygon must differ from the exact
        // facts by more than the facts volume tolerance on this curved
        // fixture — the deviation is the no-silent-flattening test.
        let profile = dome_shell_profile();
        let solid = SolidSpec::Lathe {
            profile,
            arc_deg: 360.0,
        };
        let facts = tree_facts(&part(solid, 0.0, 0.0, 0.0))
            .expect("a full-arc spline-profile lathe is in envelope");
        let exact = facts.volume;

        // The sample-polygon approximation: the same boundary with every
        // spline edge replaced by straight chords through its samples.
        let polygon = {
            let mut vertices: Vec<[f64; 2]> = Vec::new();
            for edge in dome_shell_profile() {
                match edge {
                    LatheEdge::Line { a, b } => {
                        if vertices.last() != Some(&a) {
                            vertices.push(a);
                        }
                        if vertices.last() != Some(&b) {
                            vertices.push(b);
                        }
                    }
                    LatheEdge::Spline { points } => {
                        for p in &points {
                            if vertices.last() != Some(p) {
                                vertices.push(*p);
                            }
                        }
                    }
                }
            }
            if vertices.len() > 1 && vertices[0] == *vertices.last().expect("ring nonempty") {
                vertices.pop();
            }
            vertices
        };
        let polygon_solid = SolidSpec::Lathe {
            profile: polygon
                .iter()
                .enumerate()
                .map(|(i, a)| LatheEdge::Line {
                    a: *a,
                    b: polygon[(i + 1) % polygon.len()],
                })
                .collect(),
            arc_deg: 360.0,
        };
        let polygon_facts = tree_facts(&part(polygon_solid, 0.0, 0.0, 0.0))
            .expect("the polygon fixture is a line-profile lathe");
        let deviation = (exact - polygon_facts.volume).abs() / exact;
        assert!(
            deviation > 1.0e-4,
            "the exact spline facts must differ from a sample-polygon flattening by more than \
             the facts tolerance (deviation {deviation})"
        );
    }

    #[test]
    fn spline_profile_bbox_covers_reconstructed_extrema() {
        // The profile's largest radius must not be below the exact radius of
        // the reconstructed spline at its span interiors (sampled finely).
        let profile = dome_shell_profile();
        let solid = SolidSpec::Lathe {
            profile,
            arc_deg: 360.0,
        };
        let bbox = solid_local_bbox(&solid).expect("spline profile bbox");
        let mut sampled_max_r = 0.0f64;
        for edge in &dome_shell_profile() {
            if let LatheEdge::Spline { points } = edge {
                for span in spline_spans(points).expect("reconstructs") {
                    for i in 0..=1000 {
                        let u = i as f64 / 1000.0;
                        let x = span.r[0] + u * (span.r[1] + u * (span.r[2] + u * span.r[3]));
                        sampled_max_r = sampled_max_r.max(x);
                    }
                }
            }
        }
        assert!(bbox[1][0] + 1e-9 >= sampled_max_r);
        assert!(bbox[0][0] - 1e-9 <= -sampled_max_r);
    }

    #[test]
    fn line_profile_lathe_volume_is_bit_identical_via_profile_form() {
        // The V5 pair: the profile-edge census form must produce byte-identical
        // line-profile facts to the landed vertex-loop arm.
        let vertices = [
            [10.0, 0.0],
            [20.0, 0.0],
            [20.0, 10.0],
            [15.0, 15.0],
            [10.0, 10.0],
        ];
        let vertex_arm = SolidSpec::Lathe {
            profile: vertices
                .iter()
                .enumerate()
                .map(|(i, a)| LatheEdge::Line {
                    a: *a,
                    b: vertices[(i + 1) % vertices.len()],
                })
                .collect(),
            arc_deg: 360.0,
        };
        let facts = tree_facts(&part(vertex_arm, 0.0, 0.0, 0.0)).expect("line profile facts");
        // The frustum telescoping value, computed with the landed formula.
        let points: Vec<[f64; 2]> = vertices.to_vec();
        let expected = lathe_volume(&points).expect("landed lathe volume");
        assert_eq!(facts.volume.to_bits(), expected.to_bits());
    }

    // -----------------------------------------------------------------------
    // Authoring arms: extrude prism, loft (with the seam certificate) and the
    // mirror placed carrier.
    // -----------------------------------------------------------------------

    /// A closed 3-D line-loop profile from `(x, y, z)` vertices.
    fn line_loop3(pts: &[[f64; 3]]) -> Vec<ProfileEdge> {
        pts.iter()
            .enumerate()
            .map(|(i, a)| ProfileEdge::Line {
                a: *a,
                b: pts[(i + 1) % pts.len()],
            })
            .collect()
    }

    /// A closed unit square in the `z = z0` plane, wound CCW about +z.
    fn square(z0: f64) -> Vec<ProfileEdge> {
        line_loop3(&[
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ])
    }

    #[test]
    fn prism_facts_are_exact_prism_arithmetic() {
        // The extrusion recording arm: a closed line-loop planar profile swept
        // along its own plane normal. Volume = area * swept length; `both`
        // sweeps both directions (total 2*amount), single-sided sweeps +normal.
        let profile = square(0.0);
        let both = tree_facts(&part(
            SolidSpec::Prism {
                profile: profile.clone(),
                amount: 3.0,
                both: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("a line-loop prism is in envelope");
        assert!((both.volume - 6.0).abs() < 1e-12);
        // bbox: the square at z in [-3, 3].
        assert_eq!(both.bbox[0], [0.0, 0.0, -3.0]);
        assert_eq!(both.bbox[1], [1.0, 1.0, 3.0]);

        let single = tree_facts(&part(
            SolidSpec::Prism {
                profile,
                amount: 3.0,
                both: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("single-sided prism is in envelope");
        assert!((single.volume - 3.0).abs() < 1e-12);
        assert_eq!(single.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(single.bbox[1], [1.0, 1.0, 3.0]);
    }

    #[test]
    fn loft_volume_matches_the_segment_moment_derivation() {
        // The loft arm over a parallel two-section stack: the volume is the
        // exact divergence-form integral over the ruled faces. The degenerate
        // two-section line-profile case (two identical aligned sections) is a
        // prism whose volume must be bit-identical to the extrude arm's, and
        // the general two-section case must equal the Simpson value over the
        // (quadratic) cross-section area.
        let a = square(0.0);
        let b = square(5.0);
        let solid = SolidSpec::Loft {
            sections: vec![a, b],
            closed: false,
        };
        let facts = tree_facts(&part(solid.clone(), 0.0, 0.0, 0.0))
            .expect("a line-section loft is in envelope");
        // Volume = 1 * 1 * 5.
        assert!((facts.volume - 5.0).abs() / 5.0 < 1e-12);

        // Two identical sections at z=0 and z=5 through the extrude arm: the
        // prism arm and the degenerate two-section loft must agree bit for bit.
        let prism = tree_facts(&part(
            SolidSpec::Prism {
                profile: square(0.0),
                amount: 5.0,
                both: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("prism");
        assert_eq!(facts.volume.to_bits(), prism.volume.to_bits());

        // Coaxial similar sections (a pyramid frustum): the general loft
        // segment volume equals the independent Simpson machine check and the
        // closed-form frustum identity h/3 (A0 + A1 + sqrt(A0 A1)) — the same
        // algebraic special case as the lathe line-profile frustum telescoping.
        let small = line_loop3(&[
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ]);
        let big = line_loop3(&[
            [0.0, 0.0, 3.0],
            [4.0, 0.0, 3.0],
            [4.0, 4.0, 3.0],
            [0.0, 4.0, 3.0],
        ]);
        let a = profile_loop(&small).expect("small loop");
        let b = profile_loop(&big).expect("big loop");
        let simpson = loft_segment_volume(&a, &b);
        let frustum = 3.0 / 3.0 * (a.area + b.area + (a.area * b.area).sqrt());
        assert!((simpson - frustum).abs() / frustum < 1e-12);
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![small, big],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("frustum loft");
        assert!((facts.volume - frustum).abs() / frustum < 1e-12);
    }

    #[test]
    fn closed_halo_loft_certifies_its_seam_and_refuses_open_mismatch() {
        // The halo form: the station list returns to the start, so the aligned
        // meeting edges across the chain closure satisfy the seam identity and
        // the facts carry the (zero) mismatch. A station list recorded closed
        // whose last station does not return refuses typed with the evidence.
        //
        // The fixture is a closed ring: a vertical square profile (radial
        // extent 4..5, height -1..1) sampled every 45 degrees around the z
        // axis, with the last station the exact return to the first.
        let mut sections = Vec::new();
        let make_station = |theta_deg: f64| -> Vec<ProfileEdge> {
            let th = theta_deg.to_radians();
            let (c, s) = (th.cos(), th.sin());
            let mut pts = Vec::new();
            for (r, h) in [(4.0, -1.0), (5.0, -1.0), (5.0, 1.0), (4.0, 1.0)] {
                pts.push([r * c, r * s, h]);
            }
            line_loop3(&pts)
        };
        for i in 0..8 {
            sections.push(make_station(i as f64 * 45.0));
        }
        // The return to the first station is the exact recorded first section
        // (the halo chain closes onto its own start).
        sections.push(sections[0].clone());
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections,
                closed: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("a closing halo chain is in envelope");
        assert_eq!(facts.seam_mismatch, Some(0.0));
        assert!(facts.volume > 0.0);
        assert_eq!(facts.solid_count, 1);

        // A non-closing chain: the recorded last station is shifted off the
        // return so the aligned meeting edges no longer satisfy the identity.
        let mut broken = Vec::new();
        for i in 0..8 {
            broken.push(make_station(i as f64 * 45.0));
        }
        // A perturbed "return": rotated 0.1 degree off the seam.
        broken.push(make_station(360.1));
        let refusal = tree_facts(&part(
            SolidSpec::Loft {
                sections: broken,
                closed: true,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect_err("a non-closing halo chain must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn mirror_placed_carrier_transforms_facts_without_recomputing_geometry() {
        // The mirror arm records a placed-carrier reflection over the census
        // row: the volume is unchanged, the bbox is reflected about the plane,
        // and no geometry is recomputed (the solid spec is untouched).
        let solid = SolidSpec::Prism {
            profile: square(0.0),
            amount: 2.0,
            both: true,
        };
        let base = tree_facts(&part(solid.clone(), 10.0, 0.0, 0.0)).expect("unmirrored prism");
        let mirrored = tree_facts(&mirrored_part(solid, "y")).expect("mirror placed-carrier prism");
        assert_eq!(base.volume.to_bits(), mirrored.volume.to_bits());
        assert_eq!(mirrored.bbox[0], [0.0, -1.0, -2.0]);
        assert_eq!(mirrored.bbox[1], [1.0, 0.0, 2.0]);
    }

    // -----------------------------------------------------------------------
    // Authoring frames: the full orthonormal placement rotation and the door's
    // Plane-frame / Pos / Vector client rows (AUTHOR-FRAME-CARRIERS).
    // -----------------------------------------------------------------------

    /// The corpus `ttc` directory that owns `door.py` (the door-driven
    /// authoring tests below import it from a real interpreter).
    fn corpus_ttc_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("corpus")
            .join("ttc")
    }

    /// Runs `python -c <script>` with the corpus `ttc` directory as its sole
    /// positional argument and returns stdout. The corpus door's authoring
    /// rows are pure client-side data, so no native-module staging is needed.
    fn run_door_python(script: &str) -> String {
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(script)
            .arg(corpus_ttc_dir())
            .output()
            .expect("spawn python for the door row");
        assert!(
            output.status.success(),
            "python failed:\nstdout:{}\nstderr:{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn placed_frame_facts_match_unplaced_facts_under_rigid_motion() {
        // Placed-frame rigidity: a solid under an ARBITRARY orthonormal frame
        // reports the unplaced facts volume-wise EXACTLY (placement never
        // recomputes volume) and bbox-wise as the AABB of the 8 rotated local
        // corners. The fixture is an exact 90-degree frame about z: the local
        // 20 x 10 x 30 box maps its x/y extents onto y/x, unchanged z.
        let solid = SolidSpec::Box {
            length: 20.0,
            width: 10.0,
            height: 30.0,
        };
        let unplaced = tree_facts(&part(solid.clone(), 0.0, 0.0, 0.0)).expect("unplaced box");
        // The placed row is submitted as the recorded JSON (the pure-z field is
        // defaulted): an exact 90-degree orthonormal frame about z.
        let placed_json = serde_json::json!({
            "part": {
                "solid": { "kind": "box", "length": 20.0, "width": 10.0, "height": 30.0 },
                "x": 7.0,
                "y": -3.0,
                "z": 2.0,
                "rotation": {
                    "x_dir": [0.0, 1.0, 0.0],
                    "y_dir": [-1.0, 0.0, 0.0],
                    "z_dir": [0.0, 0.0, 1.0]
                }
            }
        });
        let placed = parse_tree(&placed_json.to_string()).expect("frame-placed box row");
        let facts = tree_facts(&placed).expect("frame-placed box");
        assert_eq!(
            facts.volume.to_bits(),
            unplaced.volume.to_bits(),
            "volume must be invariant under the placement rotation"
        );
        assert_eq!(facts.solid_count, 1);
        // Rotated-AABB identity: local x range [-10, 10] maps to world y, local
        // y range [-5, 5] maps to world -x, z unchanged; then translated.
        assert_eq!(facts.bbox[0], [2.0, -13.0, -13.0]);
        assert_eq!(facts.bbox[1], [12.0, 7.0, 17.0]);
    }

    #[test]
    fn plane_frame_extrude_answers_world_facts() {
        // The Plane frame row is client-side data: a unit square drawn in the
        // `Plane(origin=(50,0,0), x_dir=(0,1,0), z_dir=(1,0,0))` section frame
        // is recorded by the door at its world location, and a single-sided
        // extrude of that Plane-frame face answers WORLD facts (an exact prism
        // spanning x in [50, 60], volume 10). The refusal surface of the old
        // `Plane` stub is gone on this carrier.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
bd = door._build_truck_module()

def v(x, y, z):
    return door.Vector(x, y, z)

# A unit square, CCW in the section plane's local (u, v) frame.
wire = door.Wire([
    door.Edge.make_line(v(0, 0, 0), v(1, 0, 0)),
    door.Edge.make_line(v(1, 0, 0), v(1, 1, 0)),
    door.Edge.make_line(v(1, 1, 0), v(0, 1, 0)),
    door.Edge.make_line(v(0, 1, 0), v(0, 0, 0)),
])
plane = door.Plane(origin=(50.0, 0.0, 0.0), x_dir=(0.0, 1.0, 0.0), z_dir=(1.0, 0.0, 0.0))
face = plane * bd.make_face(wire)
part = bd.extrude(face, amount=10.0, both=False)
node = part._node()
assert node["part"]["solid"]["kind"] == "prism", node
print(json.dumps(node))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("the door prism row must parse");
        let facts = tree_facts(&tree).expect("a Plane-frame extrude is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!((facts.volume - 10.0).abs() / 10.0 < 1e-12);
        // World facts: the section plane at x = 50 and its unit square.
        assert_eq!(facts.bbox[0], [50.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [60.0, 1.0, 1.0]);
    }

    #[test]
    fn pos_placed_assembly_counts_solids() {
        // Pos placement records a translation frame row; a multi-solid
        // assembly placed by `Pos` still reports its placed solid count (and
        // the placed union bbox).
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door
bd = door._build_truck_module()

box = bd.Pos(10.0, 20.0, 0.0) * bd.Box(1, 1, 1)
sphere = bd.Pos(0.0, 0.0, 5.0) * bd.Sphere(2.0)
group = bd.Compound(children=[box, sphere])
node = group._node()
assert node["group"][0]["part"]["x"] == 10.0, node
assert node["group"][1]["part"]["x"] == 0.0, node
print(json.dumps(node))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("the Pos-placed assembly must parse");
        let facts = tree_facts(&tree).expect("Pos-placed primitives are in envelope");
        assert_eq!(facts.solid_count, 2);
        // Placed union AABB: the box at (10, 20, 0) spans x in [9.5, 10.5] and
        // the sphere at (0, 0, 5) spans x/y in [-2, 2], z in [3, 7].
        assert_eq!(facts.bbox[0], [-2.0, -2.0, -0.5]);
        assert_eq!(facts.bbox[1], [10.5, 20.5, 7.0]);
    }

    #[test]
    fn vector_surface_answers_direction_math() {
        // The Vector attribute surface is pure client-side data (never a
        // kernel row): normalized / dot / cross / length and the scalar and
        // difference arithmetic the census DNF rows call must all answer.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

a = door.Vector(3.0, 0.0, 4.0)
assert abs(a.length - 5.0) < 1e-12
n = a.normalized()
assert abs(n.length - 1.0) < 1e-12
assert abs(n.x - 0.6) < 1e-12 and abs(n.z - 0.8) < 1e-12
assert abs(n.dot(a) - 5.0) < 1e-9
c = door.Vector(0.0, 1.0, 0.0).cross(door.Vector(1.0, 0.0, 0.0))
assert abs(c.z + 1.0) < 1e-12, c
d = door.Vector(5.0, 1.0, 1.0) - door.Vector(1.0, 1.0, 1.0)
assert d.to_tuple() == (4.0, 0.0, 0.0)
e = door.Vector(1.0, 2.0, 3.0) * 2.0
assert e.to_tuple() == (2.0, 4.0, 6.0)
f = 3.0 * door.Vector(0.0, 1.0, 0.0)
assert f.to_tuple() == (0.0, 3.0, 0.0)
g = -door.Vector(0.0, 0.0, 2.0)
assert g.to_tuple() == (0.0, 0.0, -2.0)
assert door.Vector(2, 0, 0).X == 2 and door.Vector(0, 7, 0).Y == 7 and door.Vector(0, 0, 9).Z == 9
print(json.dumps({"ok": True, "length": a.length}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("vector json");
        assert_eq!(record["ok"], true);
        assert_eq!(record["length"], 5.0);
    }

    // -----------------------------------------------------------------------
    // The recorded revolve axis frame (FRAME-REVOLVE): a non-z revolve is the
    // z-lathe of the meridian profile placed by the recorded frame whose local
    // z is the axis direction. Facts are invariant under the placement; the
    // world row answers the rotated world bbox and the exact volume.
    // -----------------------------------------------------------------------

    #[test]
    fn revolve_about_nonz_axis_answers_world_facts() {
        // A ring revolved about the wheel-frame axle (world Y) is recorded as a
        // meridian lathe plus the axis frame. Its world facts equal the exact
        // lathe volume (volume is placement-invariant) and the rotated world
        // bbox (x/z span the outer radius, y spans the axial extent).
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(door.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return door.Face(door.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

# The axle ring, drawn in the plane x=0 containing the axle (world Y):
# axial runs along Y from -5..5, radius along Z from 10..20.
axle = ring([
    v(0.0, -5.0, 10.0),
    v(0.0, -5.0, 20.0),
    v(0.0, 5.0, 20.0),
    v(0.0, 5.0, 10.0),
])
part = door.revolve(axle, axis=door.Axis.Y)
print(json.dumps(part._node()))
"#;
        let stdout = run_door_python(script);
        let node: serde_json::Value = serde_json::from_str(stdout.trim()).expect("axle row json");
        let tree = parse_tree(&node.to_string()).expect("axle revolve row parses");
        let facts = tree_facts(&tree).expect("a non-z revolve is in envelope");
        assert_eq!(facts.solid_count, 1);
        // Volume of the 10..20 radius ring over axial extent 10.
        let expected = std::f64::consts::PI * (20.0 * 20.0 - 10.0 * 10.0) * 10.0;
        assert!(
            (facts.volume - expected).abs() / expected < 1e-12,
            "world volume {} != expected {}",
            facts.volume,
            expected
        );
        // The placed ring about world Y: radius in x/z, axial along y.
        assert_eq!(facts.bbox[0], [-20.0, -5.0, -20.0]);
        assert_eq!(facts.bbox[1], [20.0, 5.0, 20.0]);
    }

    #[test]
    fn z_revolve_rows_answer_bit_identically() {
        // The z-axis path keeps the recorded legacy row shape bit-for-bit: the
        // profile coordinates are the world (radius x, axial z) pairs with no
        // recorded placement frame, and the measured facts equal the exact
        // analytic lathe of the same profile ring.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(door.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return door.Face(door.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

# A z-axis revolve ring: profile in the y=0 plane with radius along x and
# axial along z, 10..20 radius over axial extent -5..5.
ring = ring([
    v(10.0, 0.0, -5.0),
    v(20.0, 0.0, -5.0),
    v(20.0, 0.0, 5.0),
    v(10.0, 0.0, 5.0),
])
part = door.revolve(ring, axis=door.Axis.Z)
print(json.dumps(part._node()))
"#;
        let stdout = run_door_python(script);
        let node: serde_json::Value = serde_json::from_str(stdout.trim()).expect("z row json");
        // Legacy row shape: a lathe profile with the world (radius, axial)
        // coordinates, and no placement frame fields beyond the pure-z row.
        let part_node = &node["part"];
        assert_eq!(part_node["solid"]["kind"], "lathe");
        assert_eq!(part_node["rz"], 0.0);
        assert!(part_node.get("x").is_some(), "legacy row keeps its x field");
        let tree = parse_tree(&node.to_string()).expect("z revolve row parses");
        let facts = tree_facts(&tree).expect("a z revolve is in envelope");
        assert_eq!(facts.solid_count, 1);
        // Bit-identical facts: the analytic lathe of the same profile ring.
        let expected = std::f64::consts::PI * (20.0 * 20.0 - 10.0 * 10.0) * 10.0;
        assert!(
            (facts.volume - expected).abs() / expected < 1e-12,
            "z volume {} != expected {}",
            facts.volume,
            expected
        );
        assert_eq!(facts.bbox[0], [-20.0, -20.0, -5.0]);
        assert_eq!(facts.bbox[1], [20.0, 20.0, 5.0]);
    }

    #[test]
    fn revolve_refuses_unsupported_axes_typed() {
        // A zero-length axis cannot be recorded: the door refuses typed with
        // the degenerate-axis name (never a silent normalization past zero).
        let script = r#"
import json, sys, types
sys.path.insert(0, sys.argv[1])
import door

class _Refused(Exception):
    def __init__(self, msg):
        super().__init__(msg)
        self.msg = msg

door._T123D = types.SimpleNamespace(Refused=_Refused)
bd = door._build_truck_module()

def ring(corners):
    edges = []
    for i in range(len(corners)):
        edges.append(bd.Edge.make_line(corners[i], corners[(i + 1) % len(corners)]))
    return bd.Face(bd.Wire(edges))

def v(x, y, z):
    return door.Vector(x, y, z)

face = ring([
    v(10.0, 0.0, -5.0),
    v(20.0, 0.0, -5.0),
    v(20.0, 0.0, 5.0),
    v(10.0, 0.0, 5.0),
])
try:
    bd.revolve(face, axis=door.Axis((0.0, 0.0, 0.0)))
    print(json.dumps({"refused": False, "message": "no refusal"}))
except _Refused as exc:
    print(json.dumps({"refused": True, "message": exc.msg}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value = serde_json::from_str(stdout.trim()).expect("refusal json");
        assert_eq!(record["refused"], true, "degenerate axis must refuse typed");
        let message = record["message"].as_str().unwrap_or("");
        assert!(
            message.contains("DegenerateRevolveAxis"),
            "refusal must name the degenerate axis: {message}"
        );
    }

    // -----------------------------------------------------------------------
    // The spline-path sweep carrier (SWEEP-PATH): exact path queries, the
    // recorded station frames, and the V5 net over the landed loft/revolve
    // rows.
    // -----------------------------------------------------------------------

    /// The bridge's exact reconstruction of a planar path at the normalized
    /// parameter `t`: position and derivative with respect to `t`. The path
    /// lies in the `(x, z)` plane, so the bridge's 2-D `spline_spans` is the
    /// same fixed-order interpolant the volume arms use.
    fn span_derivative(span: &SpanPoly, u: f64, total: f64, h: f64) -> [f64; 3] {
        [
            (span.r[1] + u * (2.0 * span.r[2] + u * 3.0 * span.r[3])) * total / h,
            0.0,
            (span.z[1] + u * (2.0 * span.z[2] + u * 3.0 * span.z[3])) * total / h,
        ]
    }

    fn bridge_path_query(samples: &[[f64; 2]], t: f64) -> ([f64; 3], [f64; 3]) {
        let n = samples.len();
        let params = chord_params(samples);
        let total = params[n - 1];
        let spans = spline_spans(samples).expect("the planar path reconstructs");
        if t <= 0.0 {
            let h = params[1] - params[0];
            return (
                [samples[0][0], 0.0, samples[0][1]],
                span_derivative(&spans[0], 0.0, total, h),
            );
        }
        if t >= 1.0 {
            let h = params[n - 1] - params[n - 2];
            return (
                [samples[n - 1][0], 0.0, samples[n - 1][1]],
                span_derivative(&spans[n - 2], 1.0, total, h),
            );
        }
        let s = t * total;
        let mut j = 0;
        while j + 1 < n - 1 && s > params[j + 1] {
            j += 1;
        }
        let h = params[j + 1] - params[j];
        let u = (s - params[j]) / h;
        let value = eval_span(&spans[j], u);
        (
            [value[0], 0.0, value[1]],
            span_derivative(&spans[j], u, total, h),
        )
    }

    #[test]
    fn spline_path_position_and_tangent_answer_exactly() {
        // The door's exact path query must answer through the SAME fixed-order
        // interpolant the volume arms reconstruct: a planar spline path is
        // queried at interior parameters and every answer is compared to the
        // bridge's own `spline_spans` reconstruction of the same samples.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

pts = [door.Vector(0.0, 0.0, 0.0), door.Vector(1.0, 0.0, 2.0),
       door.Vector(3.0, 0.0, 1.0), door.Vector(4.0, 0.0, 3.0)]
edge = door.Edge.make_spline(pts)
rows = []
for t in (0.0, 0.25, 0.5, 0.75, 1.0):
    p = edge.position_at(t)
    d = edge.tangent_at(t)
    rows.append({"t": t, "p": list(p.to_tuple()), "d": list(d.to_tuple())})
print(json.dumps(rows))
"#;
        let stdout = run_door_python(script);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(stdout.trim()).expect("path query rows parse");
        assert_eq!(rows.len(), 5);
        let samples = [[0.0, 0.0], [1.0, 2.0], [3.0, 1.0], [4.0, 3.0]];
        for row in rows {
            let t = row["t"].as_f64().expect("t");
            let p = row["p"].as_array().expect("p");
            let d = row["d"].as_array().expect("d");
            let (want_p, want_d) = bridge_path_query(&samples, t);
            for axis in 0..3 {
                let got_p = p[axis].as_f64().expect("position coordinate");
                let got_d = d[axis].as_f64().expect("derivative coordinate");
                assert!(
                    (got_p - want_p[axis]).abs() <= 1e-12 * (1.0 + want_p[axis].abs()),
                    "position at t={t} axis {axis}: {got_p} vs {}",
                    want_p[axis]
                );
                assert!(
                    (got_d - want_d[axis]).abs() <= 1e-12 * (1.0 + want_d[axis].abs()),
                    "derivative at t={t} axis {axis}: {got_d} vs {}",
                    want_d[axis]
                );
            }
            assert_eq!(p[1].as_f64().expect("y"), 0.0, "the path lies in y = 0");
        }
        // Endpoints stay exact.
        let first = bridge_path_query(&samples, 0.0);
        let last = bridge_path_query(&samples, 1.0);
        assert_eq!(first.0, [0.0, 0.0, 0.0]);
        assert_eq!(last.0, [4.0, 0.0, 3.0]);
    }

    #[test]
    fn sweep_line_path_records_loft_chain() {
        // A line path records the landed two-station straight-sweep chain: a
        // unit square swept along +z by 5 is an exact prism (volume 5) whose
        // recorded row is the landed loft carrier.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

wire = door.Wire([
    door.Edge.make_line(v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0)),
    door.Edge.make_line(v(1.0, 0.0, 0.0), v(1.0, 1.0, 0.0)),
    door.Edge.make_line(v(1.0, 1.0, 0.0), v(0.0, 1.0, 0.0)),
    door.Edge.make_line(v(0.0, 1.0, 0.0), v(0.0, 0.0, 0.0)),
])
section = door.Face(wire)
path = door.Edge.make_line(v(0.0, 0.0, 0.0), v(0.0, 0.0, 5.0))
print(json.dumps(door.sweep(section, path)._node()))
"#;
        let stdout = run_door_python(script);
        let tree = parse_tree(&stdout).expect("sweep row parses");
        let sections = match &tree {
            TreeNode::Part { part } => match &part.solid {
                SolidSpec::Loft { sections, closed } => {
                    assert!(!*closed, "a straight sweep is an open chain");
                    sections.clone()
                }
                other => panic!("a line-path sweep must record a loft, got {other:?}"),
            },
            _ => panic!("a sweep row must be a single placed part"),
        };
        assert_eq!(sections.len(), 2, "a line path records two stations");
        assert_eq!(sections[0].len(), 4);
        assert_eq!(sections[1].len(), 4);
        // The second station is the first translated along +z by 5.
        let first = match &sections[0][0] {
            ProfileEdge::Line { a, .. } => *a,
            _ => panic!("a sweep section is all line edges"),
        };
        let second = match &sections[1][0] {
            ProfileEdge::Line { a, .. } => *a,
            _ => panic!("a sweep section is all line edges"),
        };
        assert!((second[0] - first[0]).abs() < 1e-12);
        assert!((second[1] - first[1]).abs() < 1e-12);
        assert!((second[2] - first[2] - 5.0).abs() < 1e-12);
        let facts = tree_facts(&tree).expect("a straight sweep is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!((facts.volume - 5.0).abs() / 5.0 < 1e-12);
        assert_eq!(facts.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [1.0, 1.0, 5.0]);
    }

    #[test]
    fn sweep_spline_path_records_stations() {
        // A spline path records one loft station per recorded sample; the
        // station frames carry the section centroid onto each sample.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

path = door.Edge.make_spline([v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0),
                              v(2.0, 0.0, 1.0), v(3.0, 0.0, 2.0)])
plane = door.Plane(origin=path.position_at(0), z_dir=path.tangent_at(0))
wire = door.Wire([
    door.Edge.make_line(v(-0.5, -0.5, 0.0), v(0.5, -0.5, 0.0)),
    door.Edge.make_line(v(0.5, -0.5, 0.0), v(0.5, 0.5, 0.0)),
    door.Edge.make_line(v(0.5, 0.5, 0.0), v(-0.5, 0.5, 0.0)),
    door.Edge.make_line(v(-0.5, 0.5, 0.0), v(-0.5, -0.5, 0.0)),
])
section = plane * door.Face(wire)
node = door.sweep(section, path)._node()
stations = [list(point.to_tuple()) for point in path.points]
print(json.dumps({"node": node, "stations": stations}))
"#;
        let stdout = run_door_python(script);
        let record: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("spline sweep json");
        let tree = parse_tree(&record["node"].to_string()).expect("spline sweep row parses");
        let sections = match &tree {
            TreeNode::Part { part } => match &part.solid {
                SolidSpec::Loft { sections, .. } => sections.clone(),
                other => panic!("a spline-path sweep must record a loft, got {other:?}"),
            },
            _ => panic!("a sweep row must be a single placed part"),
        };
        assert_eq!(sections.len(), 4, "one station per recorded path sample");
        let stations = record["stations"].as_array().expect("stations");
        for (i, section) in sections.iter().enumerate() {
            let mut centroid = [0.0f64; 3];
            for edge in section {
                let a = match edge {
                    ProfileEdge::Line { a, .. } => *a,
                    _ => panic!("a sweep section is all line edges"),
                };
                for axis in 0..3 {
                    centroid[axis] += a[axis];
                }
            }
            let count = section.len() as f64;
            let station = stations[i].as_array().expect("station");
            for axis in 0..3 {
                let want = station[axis].as_f64().expect("station coordinate");
                assert!(
                    (centroid[axis] / count - want).abs() < 1e-9 * (1.0 + want.abs()),
                    "station {i} axis {axis}: centroid {} vs {want}",
                    centroid[axis] / count
                );
            }
        }
        let facts = tree_facts(&tree).expect("a spline-path sweep is in envelope");
        assert_eq!(facts.solid_count, 1);
        assert!(facts.volume > 0.0);
    }

    #[test]
    fn z_revolve_and_loft_rows_answer_bit_identically() {
        // V5 net: the door-recorded z-revolve row and two-station loft row
        // answer bit-identically to the landed kernel arms.
        let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
import door

def v(x, y, z):
    return door.Vector(x, y, z)

def square(z):
    pts = [(0.0, 0.0, z), (1.0, 0.0, z), (1.0, 1.0, z), (0.0, 1.0, z)]
    return door.Face(door.Wire([
        door.Edge.make_line(v(*pts[i]), v(*pts[(i + 1) % 4])) for i in range(4)
    ]))

ring = door.Face(door.Wire([
    door.Edge.make_line(v(10.0, 0.0, -5.0), v(20.0, 0.0, -5.0)),
    door.Edge.make_line(v(20.0, 0.0, -5.0), v(20.0, 0.0, 5.0)),
    door.Edge.make_line(v(20.0, 0.0, 5.0), v(10.0, 0.0, 5.0)),
    door.Edge.make_line(v(10.0, 0.0, 5.0), v(10.0, 0.0, -5.0)),
]))
z_row = door.revolve(ring, axis=door.Axis.Z)._node()
loft_row = door.loft([square(0.0), square(5.0)], ruled=False)._node()
print(json.dumps([z_row, loft_row]))
"#;
        let stdout = run_door_python(script);
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(stdout.trim()).expect("landed row json");
        let z_tree = parse_tree(&rows[0].to_string()).expect("z row parses");
        let loft_tree = parse_tree(&rows[1].to_string()).expect("loft row parses");
        let z_facts = tree_facts(&z_tree).expect("z revolve facts");
        let loft_facts = tree_facts(&loft_tree).expect("loft facts");
        let z_landed = tree_facts(&part(
            line_lathe(
                &[[10.0, -5.0], [20.0, -5.0], [20.0, 5.0], [10.0, 5.0]],
                360.0,
            ),
            0.0,
            0.0,
            0.0,
        ))
        .expect("landed z lathe");
        assert_eq!(
            z_facts.volume.to_bits(),
            z_landed.volume.to_bits(),
            "the door z-revolve row must answer bit-identically to the landed lathe arm"
        );
        let loft_landed = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![square(0.0), square(5.0)],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("landed loft");
        assert_eq!(
            loft_facts.volume.to_bits(),
            loft_landed.volume.to_bits(),
            "the door loft row must answer bit-identically to the landed loft arm"
        );
        assert_eq!(z_facts.solid_count, 1);
        assert_eq!(loft_facts.solid_count, 1);
    }

    // -----------------------------------------------------------------------

    // Spline-section loft facts through the certified volume arm
    // (BRIDGE-LOFT-FACTS): the smooth two-station class certifies through the
    // binding, the landed line class stays bit-identical, and a smooth carrier
    // the recorded data does not determine refuses typed.
    // -----------------------------------------------------------------------

    /// A closed section loop of four spline edges, each through its two corners
    /// and their midpoint (collinear): geometrically a unit square, carried by
    /// `ProfileEdge::Spline` so the analytic line arm refuses it.
    fn spline_square(z0: f64) -> Vec<ProfileEdge> {
        let corners = [
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ];
        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let mid = [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ];
            edges.push(ProfileEdge::Spline {
                points: vec![a, mid, b],
            });
        }
        edges
    }

    /// A closed section loop of four quadratic spline edges bulging outward
    /// from the unit square: a genuinely curved spline section carrier.
    fn spline_lens(z0: f64) -> Vec<ProfileEdge> {
        let corners = [
            [0.0, 0.0, z0],
            [1.0, 0.0, z0],
            [1.0, 1.0, z0],
            [0.0, 1.0, z0],
        ];
        let center = [0.5, 0.5, z0];
        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), z0];
            let dx = mid[0] - center[0];
            let dy = mid[1] - center[1];
            let len = (dx * dx + dy * dy).sqrt();
            let outward = [mid[0] + 0.15 * dx / len, mid[1] + 0.15 * dy / len, z0];
            edges.push(ProfileEdge::Spline {
                points: vec![a, outward, b],
            });
        }
        edges
    }

    /// The 5-point Gauss-Legendre nodes on `[0, 1]`.
    const GAUSS5_NODES: [f64; 5] = [
        0.046_910_077_030_668_0,
        0.230_765_344_947_158_5,
        0.5,
        0.769_234_655_052_841_5,
        0.953_089_922_969_332_0,
    ];
    /// The 5-point Gauss-Legendre weights on `[0, 1]`.
    const GAUSS5_WEIGHTS: [f64; 5] = [
        0.118_463_442_528_094_6,
        0.239_314_335_249_683_2,
        0.284_444_444_444_444_4,
        0.239_314_335_249_683_2,
        0.118_463_442_528_094_6,
    ];

    /// The derivative of a reconstructed 3-D span at `u`.
    fn span3_derivative(span: &SpanPoly3, u: f64) -> [f64; 3] {
        [
            span.x[1] + u * (2.0 * span.x[2] + u * 3.0 * span.x[3]),
            span.y[1] + u * (2.0 * span.y[2] + u * 3.0 * span.y[3]),
            span.z[1] + u * (2.0 * span.z[2] + u * 3.0 * span.z[3]),
        ]
    }

    /// The exact side-surface divergence-form integral of a two-station ruled
    /// spline loft, computed independently of the kernel's Bernstein route by
    /// 5x2 Gauss-Legendre quadrature (exact: the integrand is bidegree (8, 2)).
    fn gauss_side_volume(sections: &[Vec<ProfileEdge>]) -> f64 {
        let a = spline_loop_spans(&sections[0]).expect("reconstruct first");
        let b = spline_loop_spans(&sections[1]).expect("reconstruct last");
        let inv3 = 1.0 / 3.0f64.sqrt();
        let nodes2 = [0.5 - 0.5 * inv3, 0.5 + 0.5 * inv3];
        let weights2 = [0.5, 0.5];
        let mut total = 0.0;
        for (sa, sb) in a.iter().zip(b.iter()) {
            for (iu, &u) in GAUSS5_NODES.iter().enumerate() {
                let aval = eval_span3(sa, u);
                let bval = eval_span3(sb, u);
                let da = span3_derivative(sa, u);
                let db = span3_derivative(sb, u);
                for (iv, &v) in nodes2.iter().enumerate() {
                    let x = [
                        (1.0 - v) * aval[0] + v * bval[0],
                        (1.0 - v) * aval[1] + v * bval[1],
                        (1.0 - v) * aval[2] + v * bval[2],
                    ];
                    let xu = [
                        (1.0 - v) * da[0] + v * db[0],
                        (1.0 - v) * da[1] + v * db[1],
                        (1.0 - v) * da[2] + v * db[2],
                    ];
                    let xv = [bval[0] - aval[0], bval[1] - aval[1], bval[2] - aval[2]];
                    total += GAUSS5_WEIGHTS[iu] * weights2[iv] * v3_dot(x, v3_cross(xu, xv)) / 3.0;
                }
            }
        }
        total
    }

    #[test]
    fn spline_section_loft_volume_certified_bracket() {
        // A genuinely curved spline-section loft certifies through the binding:
        // the returned bracket encloses the value, the bracket is tight, and
        // the value matches an independent Gauss-Legendre integration of the
        // same ruled surface plus the exact planar caps.
        let sections = vec![spline_lens(0.0), spline_lens(5.0)];
        let (value, lo, hi) = certified_spline_loft_volume(&sections)
            .expect("a two-station spline-section loft certifies");
        assert!(
            lo <= value && value <= hi,
            "the bracket [{lo}, {hi}] must enclose {value}"
        );
        assert!(
            (hi - lo) < 1.0e-6,
            "the certified bracket is tight: [{lo}, {hi}]"
        );
        let first = spline_loop_spans(&sections[0]).expect("first loop");
        let last = spline_loop_spans(&sections[1]).expect("last loop");
        let av = spline_loop_area_vector(&first);
        let bv = spline_loop_area_vector(&last);
        let ma = v3_norm(av);
        let mb = v3_norm(bv);
        let pa = [first[0].x[0], first[0].y[0], first[0].z[0]];
        let pb = [last[0].x[0], last[0].y[0], last[0].z[0]];
        let caps = -(1.0 / 3.0) * ma * v3_dot([av[0] / ma, av[1] / ma, av[2] / ma], pa)
            + (1.0 / 3.0) * mb * v3_dot([bv[0] / mb, bv[1] / mb, bv[2] / mb], pb);
        let expected = gauss_side_volume(&sections) + caps;
        assert!(
            (value - expected.abs()).abs() < 1.0e-9,
            "the certified value {value} must match the independent {expected}"
        );
    }

    #[test]
    fn loft_facts_match_recorded_reference_on_flip() {
        // The recorded reference for the equivalent line-loop prism is 5.0
        // (the landed line-loft arm's exact value). The spline-section carrier
        // flips from the analytic arm's typed refusal to the certified arm's
        // answer, matching the reference exactly.
        let sections = vec![spline_square(0.0), spline_square(5.0)];
        assert!(
            loft_sections(&sections).is_err(),
            "the analytic line arm must refuse the spline carrier"
        );
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: sections.clone(),
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("the spline-section loft certifies");
        let reference = 5.0f64;
        assert!(
            (facts.volume - reference).abs() < 1.0e-9,
            "spline-section facts {} must match the recorded reference {reference}",
            facts.volume
        );
        assert_eq!(facts.solid_count, 1);
        assert_eq!(facts.bbox[0], [0.0, 0.0, 0.0]);
        assert_eq!(facts.bbox[1], [1.0, 1.0, 5.0]);
    }

    #[test]
    fn line_loft_rows_answer_bit_identically() {
        // V5 net: the landed line-section loft rows answer bit-identically
        // through the facts arm (the analytic ruled path stays the fast path).
        let a = square(0.0);
        let b = square(5.0);
        let facts = tree_facts(&part(
            SolidSpec::Loft {
                sections: vec![a.clone(), b.clone()],
                closed: false,
            },
            0.0,
            0.0,
            0.0,
        ))
        .expect("line-section loft");
        let validated = loft_sections(&[a, b]).expect("line sections");
        let landed = loft_volume(&validated, false).expect("landed volume");
        assert_eq!(facts.volume.to_bits(), landed.to_bits());
        assert_eq!(facts.volume.to_bits(), 5.0f64.to_bits());
    }

    #[test]
    fn unmatched_loft_carrier_refuses_typed() {
        // A three-station smooth spline loft: the station interpolation is not
        // recorded, so the arm refuses typed naming the open smooth carrier.
        let three = SolidSpec::Loft {
            sections: vec![spline_lens(0.0), spline_lens(2.0), spline_lens(5.0)],
            closed: false,
        };
        let refusal = tree_facts(&part(three, 0.0, 0.0, 0.0))
            .expect_err("three smooth stations must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));

        // Mismatched section span counts refuse typed too.
        let mismatched = SolidSpec::Loft {
            sections: vec![
                spline_square(0.0),
                line_loop3(&[[0.0, 0.0, 5.0], [1.0, 0.0, 5.0], [0.5, 1.0, 5.0]]),
            ],
            closed: false,
        };
        let refusal = tree_facts(&part(mismatched, 0.0, 0.0, 0.0))
            .expect_err("mismatched sections must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    // Boolean rows through the boolean funnel (BRIDGE-BOOLEANS): the door
    // shim's cut/fuse/intersect record BooleanOp rows {mode, a, b} and dispatch
    // through the boolean entry; placed operands are canonical (the LOCAL
    // solid classifies) and the placement composes after; a refused pair keeps
    // its typed envelope case; a chain deeper than depth-1 refuses typed.
    // -----------------------------------------------------------------------

    /// One recorded boolean row over two operand nodes.
    fn boolean(mode: crate::facade::ModeValue, a: TreeNode, b: TreeNode) -> TreeNode {
        TreeNode::Boolean {
            boolean: BooleanNode {
                mode,
                a: Box::new(a),
                b: Box::new(b),
            },
        }
    }

    /// A ruled two-station loft (the `Swept` carrier class).
    fn loft_carrier() -> SolidSpec {
        SolidSpec::Loft {
            sections: vec![square(0.0), square(5.0)],
            closed: false,
        }
    }

    /// A spline-profile full-arc lathe (the `Revolved` carrier class).
    fn revolved_carrier() -> SolidSpec {
        SolidSpec::Lathe {
            profile: dome_shell_profile(),
            arc_deg: 360.0,
        }
    }

    /// A canonical box primitive.
    fn canonical_carrier() -> SolidSpec {
        SolidSpec::Box {
            length: 2.0,
            width: 2.0,
            height: 2.0,
        }
    }

    #[test]
    fn cut_swept_canonical_certifies_end_to_end() {
        // A swept base cut by a canonical tool routes through the boolean
        // entry: the routed event is recorded (placement-blind carrier classes)
        // and the routed row measures its base operand.
        let base = part(loft_carrier(), 0.0, 0.0, 0.0);
        let tree = boolean(
            crate::facade::ModeValue::Subtract,
            base.clone(),
            part(canonical_carrier(), 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&tree).expect("a swept x canonical cut routes");
        assert_eq!(facts.solid_count, 1);
        assert_eq!(facts.boolean_events.len(), 1);
        let event = facts.boolean_events[0];
        assert_eq!(event.mode, crate::facade::ModeValue::Subtract);
        assert_eq!(event.base, crate::facade::CarrierClass::Swept);
        assert_eq!(event.tool, crate::facade::CarrierClass::Canonical);
        let base_facts = tree_facts(&base).expect("base loft facts");
        assert_eq!(facts.volume.to_bits(), base_facts.volume.to_bits());
        assert_eq!(facts.bbox, base_facts.bbox);
    }

    #[test]
    fn fuse_swept_swept_certifies_end_to_end() {
        // Two swept-family carriers the admission consult admits (a ruled loft
        // and a spline-profile revolve, both funnel carriers): the fuse routes
        // and records its event.
        let tree = boolean(
            crate::facade::ModeValue::Add,
            part(loft_carrier(), 0.0, 0.0, 0.0),
            part(revolved_carrier(), 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&tree).expect("a swept x revolved fuse routes");
        assert_eq!(facts.boolean_events.len(), 1);
        let event = facts.boolean_events[0];
        assert_eq!(event.mode, crate::facade::ModeValue::Add);
        assert_eq!(event.base, crate::facade::CarrierClass::Swept);
        assert_eq!(event.tool, crate::facade::CarrierClass::Revolved);
    }

    #[test]
    fn intersect_swept_canonical_certifies_end_to_end() {
        // A revolved base intersected with a canonical tool routes through the
        // boolean entry with the intersect mode recorded.
        let tree = boolean(
            crate::facade::ModeValue::Intersect,
            part(revolved_carrier(), 0.0, 0.0, 0.0),
            part(canonical_carrier(), 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&tree).expect("a revolved x canonical intersect routes");
        assert_eq!(facts.boolean_events.len(), 1);
        let event = facts.boolean_events[0];
        assert_eq!(event.mode, crate::facade::ModeValue::Intersect);
        assert_eq!(event.base, crate::facade::CarrierClass::Revolved);
        assert_eq!(event.tool, crate::facade::CarrierClass::Canonical);
    }

    #[test]
    fn placed_operands_transform_to_canonical_before_dispatch() {
        // The dispatch sees the operand's LOCAL carrier class, never its
        // placement; the placement composes after and is reflected in the
        // measured world facts.
        let placed_base = part(loft_carrier(), 10.0, 0.0, 0.0);
        let tree = boolean(
            crate::facade::ModeValue::Subtract,
            placed_base,
            part(canonical_carrier(), 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&tree).expect("a placed swept base routes");
        assert_eq!(facts.boolean_events.len(), 1);
        assert_eq!(
            facts.boolean_events[0].base,
            crate::facade::CarrierClass::Swept
        );
        let unplaced = tree_facts(&part(loft_carrier(), 0.0, 0.0, 0.0)).expect("unplaced base");
        assert!((facts.bbox[0][0] - (unplaced.bbox[0][0] + 10.0)).abs() < 1e-12);
        assert!((facts.bbox[1][0] - (unplaced.bbox[1][0] + 10.0)).abs() < 1e-12);
    }

    #[test]
    fn refused_pairs_still_refuse_typed_unchanged() {
        // A both-Swept pair keeps the constructive-carrier refusal at the
        // boolean boundary.
        let swept_swept = boolean(
            crate::facade::ModeValue::Add,
            part(loft_carrier(), 0.0, 0.0, 0.0),
            part(loft_carrier(), 0.0, 0.0, 0.0),
        );
        let refusal = tree_facts(&swept_swept).expect_err("two lofts must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));

        // A torus carrier in a swept pair keeps the localized refusal.
        let torus_pair = boolean(
            crate::facade::ModeValue::Subtract,
            part(loft_carrier(), 0.0, 0.0, 0.0),
            part(
                SolidSpec::Torus {
                    major: 5.0,
                    minor: 1.0,
                },
                0.0,
                0.0,
                0.0,
            ),
        );
        let refusal = tree_facts(&torus_pair).expect_err("a torus pair must refuse typed");
        assert!(matches!(
            refusal,
            Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
        ));

        // A canonical x canonical pair lands on the canonical path and records
        // no routed event.
        let canonical = boolean(
            crate::facade::ModeValue::Subtract,
            part(canonical_carrier(), 0.0, 0.0, 0.0),
            part(canonical_carrier(), 0.0, 0.0, 0.0),
        );
        let facts = tree_facts(&canonical).expect("canonical x canonical lands");
        assert!(facts.boolean_events.is_empty());
    }
}
