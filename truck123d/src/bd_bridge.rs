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
}

/// A node of the submitted construction tree: either one placed solid (a
/// part) or a group (a compound) of child nodes, in script order.
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

/// The world bounding box of one placed solid: rotate the local bbox corners
/// about z by `rz`, then translate.
fn part_world_bbox(part: &PartSpec) -> Result<[[f64; 3]; 2], Refusal> {
    let local = solid_local_bbox(&part.solid)?;
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
        let (x, y) = if rz == 0.0 {
            (corner[0], corner[1])
        } else {
            (
                corner[0] * cos - corner[1] * sin,
                corner[0] * sin + corner[1] * cos,
            )
        };
        let point = [x + part.x, y + part.y, corner[2] + part.z];
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok([min, max])
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
    Ok(Facts {
        solid_count: count,
        volume,
        bbox: [min, max],
    })
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
        TreeNode::Group { group } => {
            let mut volume = 0.0;
            for child in group {
                if let TreeNode::Part { part } = child {
                    volume += solid_volume(&part.solid)?;
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
type Triangle = [f64; 9];

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
            let rz = part.rz.to_radians();
            let cos = rz.cos();
            let sin = rz.sin();
            for tri in local {
                out.push(place_triangle(tri, cos, sin, part.x, part.y, part.z));
            }
            Ok(())
        }
        TreeNode::Group { group } => {
            for child in group {
                append_node_mesh(child, out)?;
            }
            Ok(())
        }
    }
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
    }
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

/// The pyo3 measurement entry: takes the construction tree JSON and returns
/// the facts JSON (`{solid_count, volume, bbox}`).
#[pyfunction]
pub fn bd_facts(py: Python<'_>, tree_json: &str) -> PyResult<String> {
    let tree = parse_tree(tree_json).map_err(|refusal| refusal_to_pyerr(py, &refusal))?;
    let outcome = crate::gil::with_kernel_gil_released(py, move || tree_facts(&tree));
    match outcome {
        Ok(facts) => serde_json::to_string(&serde_json::json!({
            "solid_count": facts.solid_count,
            "volume": facts.volume,
            "bbox": facts.bbox,
        }))
        .map_err(|e| PyRuntimeError::new_err(format!("facts serialization failed: {e}"))),
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
}
