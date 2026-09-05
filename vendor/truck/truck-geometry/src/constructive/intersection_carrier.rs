//! BIE-003-CARRIER — the certified implicit intersection curve carrier.
//!
//! [`CertifiedImplicitIntersectionCurve`] is the frozen edge carrier of the
//! Certified Interaction Engine (BIE) program (BIE-000-CONTRACT §8.1,
//! BIE_BUILD_SPINE §3): a canonical `Curve` variant carrying a **certified**
//! 3-D polyline with per-sample tangent frames plus the unresolved witness
//! slot. `truck-geometry` does not depend on `truck-certified`, so the
//! witness slot is a minimal carrier-local mirror of the BIE-000
//! `InteractionOutcome::Unresolved { κ, cell, slope }` record (see
//! [`CarrierUnresolved`]); the producing-method tag rides on each
//! [`CertifiedSample`] exactly as BIE-000's `CertificateValue` carries its
//! `Method` (H-6).
//!
//! **PL at tessellation only.** The carrier is procedural: it is evaluable
//! continuously through its stored frames (a cubic Hermite through the
//! certified stations whose knot tangents are the stored frame tangents), and
//! its polyline form ([`CertifiedImplicitIntersectionCurve::polyline`]) is
//! consumed ONLY at tessellation — the certified sample data the landed
//! `truck-meshalgo` `EdgeSampleLedger` records (`parameters` + the interned
//! positions). `truck-meshalgo` is read-only for the BIE program; this
//! module merely carries the data in the ledger's shape.
//!
//! **Refusing-constructors discipline.** A certified carrier is never built
//! from bare floats: every station is a [`CertifiedSample`] tagged with the
//! `Method` that produced it, and a `Method::None` tag (bare data, no
//! certificate) is refused typed — H-2, never a panic. The certificate rides
//! on the polyline, not as a claim of exactness (H-6): the returned
//! `Outcome`'s certificate stamps the uniform producing `Method` of the
//! accepted sample stream.
//!
//! House rules H-1, H-2, H-3, H-6 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Bound;
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal,
};

use super::{ConstructError, Frame3};

/// The right-handed orthonormal tangent frame at one certified station.
///
/// `tangent` is the curve direction at the station; the triple satisfies the
/// same validation as [`Frame3`] (finite, unit, pairwise orthogonal,
/// `tangent × normal == binormal`). The continuous evaluation interpolates
/// through the `tangent`; the transverse vectors ride for the BIE consumers
/// that escalate/consume the carrier (BIE-004/BIE-005).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntersectionFrame {
    /// The unit curve tangent at the station.
    pub tangent: Vector3,
    /// A unit vector orthogonal to `tangent`.
    pub normal: Vector3,
    /// `tangent × normal`.
    pub binormal: Vector3,
}

impl IntersectionFrame {
    /// Validates and builds a frame: every component finite, every vector unit
    /// length, all three pairwise orthogonal, and the triple right-handed
    /// (`tangent × normal == binormal`) — the [`Frame3::try_new`] gate.
    pub fn try_new(
        tangent: Vector3,
        normal: Vector3,
        binormal: Vector3,
    ) -> std::result::Result<Self, ConstructError> {
        let frame = Frame3::try_new(tangent, normal, binormal)?;
        Ok(IntersectionFrame {
            tangent: frame.tangent,
            normal: frame.normal,
            binormal: frame.binormal,
        })
    }

    /// The frame of the reversed traversal: `tangent` and `normal` flip sign,
    /// `binormal` stays, keeping the triple right-handed (`(-t) × (-n) = b`).
    #[inline(always)]
    fn reversed(&self) -> Self {
        IntersectionFrame {
            tangent: -self.tangent,
            normal: -self.normal,
            binormal: self.binormal,
        }
    }
}

/// One certified station: a certified polyline vertex with its position, its
/// per-sample tangent frame, and the `Method` that produced it (H-6).
///
/// A station tagged `Method::None` is bare float data with no certificate;
/// every certified construction refuses it typed (H-2). The producing method
/// is provenance supplied by the certified engine that sampled the curve
/// (BIE-002 output), never inferred from the floats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CertifiedSample {
    /// The certified position of the station.
    pub position: Point3,
    /// The per-sample tangent frame at the station.
    pub frame: IntersectionFrame,
    /// The method that produced the sample.
    pub method: Method,
}

/// A `(u, v) × (s, t)` parameter-cell record — the minimal carrier-local
/// mirror of the BIE-000 `WitnessCell`: the four scalar parameter intervals
/// of the product-domain cell the restricted-pair solver leaves unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CarrierCell {
    /// The `u`-parameter interval.
    pub u: (f64, f64),
    /// The `v`-parameter interval.
    pub v: (f64, f64),
    /// The `s`-parameter interval.
    pub s: (f64, f64),
    /// The `t`-parameter interval.
    pub t: (f64, f64),
}

/// The unresolved witness slot record — the minimal carrier-local mirror of
/// the BIE-000 `InteractionOutcome::Unresolved { κ, cell, slope }` witness
/// (BIE-000-CONTRACT; truck-geometry does not depend on truck-certified). It
/// maps onto the landed `Refusal::NumericallyUnresolved` / `KrawczykIndeterminate`
/// projection downstream; a `None` slot means the carried polyline is fully
/// certified.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CarrierUnresolved {
    /// The conditioning / curvature witness that kept a cell from a certified
    /// answer.
    pub kappa: f64,
    /// The `(u, v) × (s, t)` parameter cell that stayed unresolved.
    pub cell: CarrierCell,
    /// The §5.4 slope diagnostic of the unresolved cell.
    pub slope: f64,
}

/// The certified implicit intersection curve carrier (BIE-003-CARRIER).
///
/// Carries a certified 3-D polyline (one position and one tangent frame per
/// station) and the unresolved witness slot. Procedural: `subs`/`der` evaluate
/// continuously through the stored frames (cubic Hermite whose knot tangents
/// are the frame tangents). The carrier's parameterization is the cumulative
/// chord length of the certified polyline — unit-speed by construction — so
/// the stored unit tangents ARE the parameter derivatives: `der` at a
/// certified station is exactly the stored tangent and the positions at the
/// certified parameters are exactly the stored polyline vertices. The
/// polyline form is consumed ONLY at tessellation (the
/// [`EdgeSampleLedger`](https://docs.rs) sample shape), never by the
/// continuous evaluation.
///
/// `PartialEq` is deliberately NOT derived: the carrier is geometry data on
/// the canonical `Curve` enum, which carries no equality (the landed
/// decorators' precedent).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertifiedImplicitIntersectionCurve {
    /// The carrier parameters (cumulative chord length), strictly ascending.
    parameters: Vec<f64>,
    /// The certified polyline vertices, aligned with `parameters`.
    positions: Vec<Point3>,
    /// The per-sample tangent frames, aligned with `parameters`.
    frames: Vec<IntersectionFrame>,
    /// The unresolved witness slot; `None` when fully certified.
    unresolved: Option<CarrierUnresolved>,
}

impl CertifiedImplicitIntersectionCurve {
    /// The carrier parameters (cumulative chord length of the certified
    /// polyline), strictly ascending.
    #[inline(always)]
    pub fn parameters(&self) -> &[f64] {
        &self.parameters
    }

    /// The certified polyline vertices, aligned with [`Self::parameters`].
    #[inline(always)]
    pub fn positions(&self) -> &[Point3] {
        &self.positions
    }

    /// The per-sample tangent frames, aligned with [`Self::parameters`].
    #[inline(always)]
    pub fn frames(&self) -> &[IntersectionFrame] {
        &self.frames
    }

    /// The certified polyline — the tessellation-facing accessor.
    ///
    /// This is the form the mesh consumes (and the `EdgeSampleLedger`
    /// records): the certified vertices, at the certified parameters. The
    /// continuous evaluation does NOT round-trip through this accessor; it
    /// interpolates through the stored frames directly.
    #[inline(always)]
    pub fn polyline(&self) -> &[Point3] {
        &self.positions
    }

    /// The unresolved witness slot (`None` when fully certified).
    #[inline(always)]
    pub fn unresolved(&self) -> Option<CarrierUnresolved> {
        self.unresolved
    }

    /// Certified construction from the certified sample stream.
    ///
    /// Refuses typed (H-2, never panics) when:
    ///
    /// - fewer than two stations,
    /// - any position is non-finite or two consecutive stations coincide (a
    ///   zero chord: no certified segment to interpolate),
    /// - a frame fails the [`Frame3::try_new`] orthonormality gate, or
    /// - a station carries `Method::None` (bare float data with no
    ///   certificate) or the stream's producing methods are not uniform.
    ///
    /// The carrier's parameterization is the cumulative chord length of the
    /// certified polyline (unit-speed by construction). The returned
    /// certificate stamps the uniform producing `Method` of the accepted
    /// stream (H-6: float-computed samples certify `Float`, never `Exact`).
    pub fn try_new(
        samples: &[CertifiedSample],
        unresolved: Option<CarrierUnresolved>,
    ) -> Outcome<Self> {
        if samples.len() < 2 {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
        }
        let method = match samples.first() {
            Some(first) => first.method,
            None => {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
            }
        };
        if method == Method::None {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
        }
        let mut parameters = Vec::with_capacity(samples.len());
        let mut positions = Vec::with_capacity(samples.len());
        let mut frames = Vec::with_capacity(samples.len());
        let mut arc = 0.0;
        let mut previous: Option<Point3> = None;
        for sample in samples {
            if sample.method == Method::None || sample.method != method {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
            }
            let finite = sample.position.x.is_finite()
                && sample.position.y.is_finite()
                && sample.position.z.is_finite();
            if !finite {
                return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
            }
            let _ = Frame3::try_new(
                sample.frame.tangent,
                sample.frame.normal,
                sample.frame.binormal,
            )
            .map_err(|_| Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused))?;
            match previous {
                None => parameters.push(0.0),
                Some(previous_position) => {
                    let step = (sample.position - previous_position).magnitude();
                    if !step.is_finite() || step <= 0.0 {
                        return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused));
                    }
                    arc += step;
                    parameters.push(arc);
                }
            }
            positions.push(sample.position);
            frames.push(sample.frame);
            previous = Some(sample.position);
        }
        Ok(Certified::new(
            CertifiedImplicitIntersectionCurve {
                parameters,
                positions,
                frames,
                unresolved,
            },
            Certificate {
                props: PropMap::new(),
                method,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }

    /// The parameter at the first station.
    #[inline(always)]
    fn first_parameter(&self) -> f64 {
        self.parameters.first().copied().unwrap_or(0.0)
    }

    /// The parameter at the last station.
    #[inline(always)]
    fn last_parameter(&self) -> f64 {
        self.parameters.last().copied().unwrap_or(0.0)
    }

    /// The continuous evaluation at `t`: the cubic Hermite through the stored
    /// frames. Returns `(position, derivative, second derivative)`.
    fn eval(&self, t: f64) -> Option<(Point3, Vector3, Vector3)> {
        let seg_count = self.parameters.len().checked_sub(1)?;
        if seg_count == 0 {
            return None;
        }
        let mut k = seg_count - 1;
        for i in 0..seg_count {
            let Some(&b) = self.parameters.get(i + 1) else {
                continue;
            };
            if t <= b {
                k = i;
                break;
            }
        }
        let (Some(&a), Some(&b)) = (self.parameters.get(k), self.parameters.get(k + 1)) else {
            return None;
        };
        let delta = b - a;
        if delta <= 0.0 {
            return None;
        }
        let (Some(&pa), Some(&pb)) = (self.positions.get(k), self.positions.get(k + 1)) else {
            return None;
        };
        let (Some(&fa), Some(&fb)) = (self.frames.get(k), self.frames.get(k + 1)) else {
            return None;
        };
        let u = (t - a) / delta;
        Some(eval_hermite(
            pa.to_vec(),
            fa.tangent,
            pb.to_vec(),
            fb.tangent,
            delta,
            u,
        ))
    }
}

/// The cubic Hermite evaluation of one segment.
///
/// `p0`/`p1` are the segment's endpoint positions, `m0`/`m1` the endpoint
/// frame tangents, `delta` the parameter width, `u ∈ [0, 1]` the normalized
/// position. Returns `(position, derivative, second derivative)` with
/// `der(a) = m0` and `der(b) = m1` exactly.
fn eval_hermite(
    p0: Vector3,
    m0: Vector3,
    p1: Vector3,
    m1: Vector3,
    delta: f64,
    u: f64,
) -> (Point3, Vector3, Vector3) {
    let u2 = u * u;
    let u3 = u2 * u;
    let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
    let h10 = (u3 - 2.0 * u2 + u) * delta;
    let h01 = -2.0 * u3 + 3.0 * u2;
    let h11 = (u3 - u2) * delta;
    let dh00 = 6.0 * u2 - 6.0 * u;
    let dh10 = (3.0 * u2 - 4.0 * u + 1.0) * delta;
    let dh01 = -6.0 * u2 + 6.0 * u;
    let dh11 = (3.0 * u2 - 2.0 * u) * delta;
    let ddh00 = 12.0 * u - 6.0;
    let ddh10 = (6.0 * u - 4.0) * delta;
    let ddh01 = -12.0 * u + 6.0;
    let ddh11 = (6.0 * u - 2.0) * delta;
    let position = Point3::from_vec(h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1);
    let der = (dh00 * p0 + dh10 * m0 + dh01 * p1 + dh11 * m1) / delta;
    let der2 = (ddh00 * p0 + ddh10 * m0 + ddh01 * p1 + ddh11 * m1) / (delta * delta);
    (position, der, der2)
}

/// Re-orthonormalizes a mapped frame triple; `None` when the linear image is
/// degenerate (a zero or non-finite tangent or normal), in which case the
/// caller keeps the original frame by identity — never a fabricated one.
fn reorthonormalize(tangent: Vector3, normal: Vector3) -> Option<IntersectionFrame> {
    let t_len = tangent.magnitude();
    if !t_len.is_finite() || t_len == 0.0 {
        return None;
    }
    let tangent = tangent / t_len;
    let lateral = normal - tangent * normal.dot(tangent);
    let n_len = lateral.magnitude();
    if !n_len.is_finite() || n_len == 0.0 {
        return None;
    }
    let normal = lateral / n_len;
    let binormal = tangent.cross(normal);
    let b_len = binormal.magnitude();
    if !b_len.is_finite() || b_len == 0.0 {
        return None;
    }
    Some(IntersectionFrame {
        tangent,
        normal,
        binormal: binormal / b_len,
    })
}

impl ParametricCurve for CertifiedImplicitIntersectionCurve {
    type Point = Point3;
    type Vector = Vector3;

    #[inline(always)]
    fn subs(&self, t: f64) -> Point3 {
        let lo = self.first_parameter();
        let hi = self.last_parameter();
        let t = clamp_parameter(t, lo, hi);
        match self.eval(t) {
            Some((position, _, _)) => position,
            None => self
                .positions
                .first()
                .copied()
                .unwrap_or_else(Point3::origin),
        }
    }
    #[inline(always)]
    fn der(&self, t: f64) -> Vector3 {
        let lo = self.first_parameter();
        let hi = self.last_parameter();
        let t = clamp_parameter(t, lo, hi);
        match self.eval(t) {
            Some((_, der, _)) => der,
            None => Vector3::zero(),
        }
    }
    #[inline(always)]
    fn der2(&self, t: f64) -> Vector3 {
        let lo = self.first_parameter();
        let hi = self.last_parameter();
        let t = clamp_parameter(t, lo, hi);
        match self.eval(t) {
            Some((_, _, der2)) => der2,
            None => Vector3::zero(),
        }
    }
    #[inline(always)]
    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        match n {
            0 => self.subs(t).to_vec(),
            1 => self.der(t),
            2 => self.der2(t),
            _ => Vector3::zero(),
        }
    }
    #[inline(always)]
    fn parameter_range(&self) -> ParameterRange {
        (
            Bound::Included(self.first_parameter()),
            Bound::Included(self.last_parameter()),
        )
    }
}

impl BoundedCurve for CertifiedImplicitIntersectionCurve {}

impl ParameterDivision1D for CertifiedImplicitIntersectionCurve {
    type Point = Point3;

    /// The certified division of `range`: the certified polyline's parameters
    /// and vertices inside the range, with the range endpoints evaluated
    /// continuously. Over the carrier's own domain this is exactly the
    /// certified polyline — the tessellation/ledger consumption point.
    fn parameter_division(&self, range: (f64, f64), _tol: f64) -> (Vec<f64>, Vec<Point3>) {
        let (r0, r1) = if range.0 <= range.1 {
            range
        } else {
            (range.1, range.0)
        };
        let mut parameters = Vec::with_capacity(self.parameters.len() + 2);
        let mut points = Vec::with_capacity(self.parameters.len() + 2);
        parameters.push(r0);
        points.push(self.subs(r0));
        for (i, &parameter) in self.parameters.iter().enumerate() {
            if parameter > r0 && parameter < r1 {
                let Some(&position) = self.positions.get(i) else {
                    continue;
                };
                parameters.push(parameter);
                points.push(position);
            }
        }
        parameters.push(r1);
        points.push(self.subs(r1));
        (parameters, points)
    }
}

impl Cut for CertifiedImplicitIntersectionCurve {
    /// Splits the carrier at the certified station nearest `t`.
    ///
    /// Both halves keep ONLY certified stations (no interpolated station is
    /// ever fabricated into a certified polyline): the head keeps the
    /// stations up to the split station and the tail keeps the split station
    /// onward, sharing it like [`PolylineCurve`](truck_polymesh) shares a cut
    /// vertex. The split index is clamped so both halves keep at least two
    /// stations; a two-station carrier (a single certified segment) cannot be
    /// split into two certified pieces, so cut is a total no-op tail there.
    fn cut(&mut self, t: f64) -> Self {
        let n = self.parameters.len();
        if n < 3 {
            return self.clone();
        }
        let lo = self.first_parameter();
        let hi = self.last_parameter();
        let t = clamp_parameter(t, lo, hi);
        let mut best = 1usize;
        let mut best_distance = f64::INFINITY;
        for (idx, &parameter) in self.parameters.iter().enumerate() {
            if idx == 0 || idx >= n - 1 {
                continue;
            }
            let distance = (parameter - t).abs();
            if distance < best_distance {
                best_distance = distance;
                best = idx;
            }
        }
        let tail = CertifiedImplicitIntersectionCurve {
            parameters: self.parameters.iter().skip(best).copied().collect(),
            positions: self.positions.iter().skip(best).copied().collect(),
            frames: self.frames.iter().skip(best).copied().collect(),
            unresolved: self.unresolved,
        };
        let head = CertifiedImplicitIntersectionCurve {
            parameters: self.parameters.iter().take(best + 1).copied().collect(),
            positions: self.positions.iter().take(best + 1).copied().collect(),
            frames: self.frames.iter().take(best + 1).copied().collect(),
            unresolved: self.unresolved,
        };
        *self = head;
        tail
    }
}

impl Invertible for CertifiedImplicitIntersectionCurve {
    fn invert(&mut self) {
        let span = self.first_parameter() + self.last_parameter();
        for parameter in self.parameters.iter_mut() {
            *parameter = span - *parameter;
        }
        self.parameters.reverse();
        self.positions.reverse();
        self.frames.reverse();
        for frame in self.frames.iter_mut() {
            *frame = frame.reversed();
        }
    }
}

impl Transformed<Matrix4> for CertifiedImplicitIntersectionCurve {
    fn transform_by(&mut self, trans: Matrix4) {
        for (position, frame) in self.positions.iter_mut().zip(self.frames.iter_mut()) {
            *position = trans.transform_point(*position);
            let mapped = reorthonormalize(
                trans.transform_vector(frame.tangent),
                trans.transform_vector(frame.normal),
            );
            if let Some(mapped) = mapped {
                *frame = mapped;
            }
        }
    }
}

impl SearchParameter<D1> for CertifiedImplicitIntersectionCurve {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint1D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(t) => t,
            SPHint1D::Range(a, b) => {
                algo::curve::presearch(self, point, (a, b), crate::PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), crate::PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_parameter(self, point, hint, trials)
    }
}

impl SearchNearestParameter<D1> for CertifiedImplicitIntersectionCurve {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint1D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let hint = match hint.into() {
            SPHint1D::Parameter(t) => t,
            SPHint1D::Range(a, b) => {
                algo::curve::presearch(self, point, (a, b), crate::PRESEARCH_DIVISION)
            }
            SPHint1D::None => {
                algo::curve::presearch(self, point, self.range_tuple(), crate::PRESEARCH_DIVISION)
            }
        };
        algo::curve::search_nearest_parameter(self, point, hint, trials)
    }
}

/// Clamps `t` into `[lo, hi]` (total; out-of-range evaluation reads the end
/// stations, the [`PolylineCurve`](truck_polymesh) convention).
#[inline(always)]
fn clamp_parameter(t: f64, lo: f64, hi: f64) -> f64 {
    if t < lo {
        lo
    } else if t > hi {
        hi
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Consecutive pairs of a `Copy` slice (window 2 without indexing).
    fn consecutive<T: Copy>(slice: &[T]) -> impl Iterator<Item = (T, T)> + '_ {
        slice
            .iter()
            .zip(slice.iter().skip(1))
            .map(|(&a, &b)| (a, b))
    }

    /// The unit frame whose tangent is the angle-`theta` arc tangent of the
    /// unit circle in the xy plane: `tangent = (-sin, cos, 0)`, `normal =
    /// +z`, `binormal = (cos, sin, 0)` (right-handed, unit, orthogonal by
    /// construction).
    fn arc_frame(theta: f64) -> IntersectionFrame {
        let tangent = Vector3::new(-theta.sin(), theta.cos(), 0.0);
        let normal = Vector3::unit_z();
        let binormal = tangent.cross(normal);
        IntersectionFrame {
            tangent,
            normal,
            binormal,
        }
    }

    /// Builds the certified sample stream of a quarter unit circle from
    /// `stations` samples, tagged `Method::Float`.
    fn quarter_circle_samples(stations: usize) -> Vec<CertifiedSample> {
        let step = PI / 2.0 / stations as f64;
        (0..=stations)
            .map(|i| {
                let theta = i as f64 * step;
                CertifiedSample {
                    position: Point3::new(theta.cos(), theta.sin(), 0.0),
                    frame: arc_frame(theta),
                    method: Method::Float,
                }
            })
            .collect()
    }

    /// Asserts the precondition then yields the certified curve value
    /// (clippy-silent, unwrap-free: the divergent tail is a `None`).
    fn expect_curve(samples: &[CertifiedSample]) -> Option<CertifiedImplicitIntersectionCurve> {
        let outcome = CertifiedImplicitIntersectionCurve::try_new(samples, None);
        assert!(
            outcome.is_ok(),
            "certified construction refused unexpectedly"
        );
        match outcome {
            Ok(certified) => Some(certified.value),
            Err(_) => None,
        }
    }

    #[test]
    fn carrier_constructs_from_certified_polyline() {
        // A certified quarter-circle polyline + frames constructs, and the
        // frame-based continuous evaluation interpolates the arc.
        let samples = quarter_circle_samples(64);
        let carrier = match expect_curve(&samples) {
            Some(carrier) => carrier,
            None => return,
        };
        assert_eq!(carrier.parameters().len(), 65);
        assert_eq!(carrier.polyline().len(), 65);
        assert_eq!(carrier.frames().len(), 65);
        assert!(carrier.unresolved().is_none());
        // `subs` at the certified (chord-length) parameters returns the
        // certified vertices.
        let tolerance = 1.0e-6; // H-3
        for (i, (&parameter, &position)) in carrier
            .parameters()
            .iter()
            .zip(carrier.polyline().iter())
            .enumerate()
        {
            let on_curve = carrier.subs(parameter);
            assert!(
                (on_curve - position).magnitude() <= tolerance,
                "subs drifted from the certified polyline at station {i}"
            );
        }
        // `subs` between stations interpolates through the stored frames: a
        // mid-segment point lies near the circle's own point at that angle.
        let arc_tolerance = 2.0e-3; // H-3
        let step = PI / 2.0 / 64.0;
        for (i, (a, b)) in consecutive(carrier.parameters()).enumerate() {
            let t = 0.5 * (a + b);
            let theta = (i as f64 + 0.5) * step;
            let expected = Point3::new(theta.cos(), theta.sin(), 0.0);
            let point = carrier.subs(t);
            assert!(
                (point - expected).magnitude() <= arc_tolerance,
                "frame interpolation drifted from the arc at segment {i}"
            );
        }
    }

    #[test]
    fn carrier_refuses_uncertified_input() {
        // Bare floats without a certificate refuse (H-2) and never panic: the
        // sample stream tagged `Method::None` is refused typed.
        let mut samples = quarter_circle_samples(8);
        for sample in samples.iter_mut() {
            sample.method = Method::None;
        }
        let outcome = CertifiedImplicitIntersectionCurve::try_new(&samples, None);
        assert!(
            matches!(outcome, Err(Refusal::UnsupportedEnvelope(_))),
            "bare floats without a certificate must refuse, got {outcome:?}"
        );
        // Two coincident stations (no certified segment) refuse too.
        let frame = arc_frame(0.0);
        let degenerate = [
            CertifiedSample {
                position: Point3::new(0.0, 0.0, 0.0),
                frame,
                method: Method::Float,
            },
            CertifiedSample {
                position: Point3::new(0.0, 0.0, 0.0),
                frame,
                method: Method::Float,
            },
        ];
        let outcome = CertifiedImplicitIntersectionCurve::try_new(&degenerate, None);
        assert!(
            matches!(outcome, Err(Refusal::UnsupportedEnvelope(_))),
            "a coincident-station polyline must refuse, got {outcome:?}"
        );
    }

    #[test]
    fn carrier_pl_at_tessellation_only() {
        // A coarsely sampled quarter circle: the certified polyline is the
        // tessellation-facing form, while the continuous evaluation goes
        // through the stored frames and does NOT round-trip through the
        // polyline (an interior evaluation is not the chord of the polyline).
        let samples = quarter_circle_samples(4);
        let carrier = match expect_curve(&samples) {
            Some(carrier) => carrier,
            None => return,
        };
        // The tessellation division over the whole domain is exactly the
        // certified polyline (parameters + vertices), in order.
        let (parameters, points) = carrier.parameter_division(carrier.range_tuple(), 0.0);
        assert_eq!(parameters, carrier.parameters());
        assert_eq!(points, carrier.polyline());
        // The derivative at a certified station is exactly the stored frame
        // tangent (the continuous path's knot condition).
        let tolerance = 1.0e-9; // H-3
        for (&parameter, &frame) in carrier.parameters().iter().zip(carrier.frames().iter()) {
            let tangent = carrier.der(parameter);
            assert!(
                (tangent - frame.tangent).magnitude() <= tolerance,
                "der at a station must equal the stored frame tangent"
            );
        }
        // Mid-segment, the frame-based path is NOT the chord: the coarse arc
        // bulges away from the segment chord by more than a rounding sliver.
        let (a, b) = match consecutive(carrier.parameters()).nth(1) {
            Some(pair) => pair,
            None => return,
        };
        let (pa, pb) = match consecutive(carrier.polyline()).nth(1) {
            Some(pair) => pair,
            None => return,
        };
        let t = 0.5 * (a + b);
        let on_curve = carrier.subs(t);
        let chord = pa + (pb - pa) * 0.5;
        let bow = (on_curve - chord).magnitude();
        assert!(
            bow > 1.0e-3, // H-3
            "the continuous evaluation must not be the polyline chord (bow {bow})"
        );
    }
}
