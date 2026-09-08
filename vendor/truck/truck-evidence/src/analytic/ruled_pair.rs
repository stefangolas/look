//! FSSI-004-RULED (BG-ANA-002-RULED): ruled × ruled exact analytic locus — the
//! scalar predicate, generator excision, and domain-crossing events.
//!
//! Theory §9.3, packet 4: for `X(u, t) = A(t) + u·d(t)` and
//! `Y(v, s) = B(s) + v·e(s)` with `d × e ≠ 0`, the intersection of the two
//! generator lines is the exact scalar predicate
//!
//! ```text
//! λ(t, s) = (B(s) − A(t)) · (d(t) × e(s)) = 0,
//! ```
//!
//! and on that locus the generator parameters recover rationally as
//!
//! ```text
//! u = det[B − A, e, d × e] / ‖d × e‖²,   v = det[B − A, d, d × e] / ‖d × e‖².
//! ```
//!
//! This is the smallest solver in the FSSI family: the only reduction with **no
//! preconditioner and no interval wrapping term**. Fixed-`t` continuation is a
//! scalar root solve under the Bernstein/Descartes discipline
//! (`crate::num::roots`), never a 2-D Krawczyk.
//!
//! # Admission (BG-ANA-002 house style)
//!
//! Recognition is **exact and conjunctive**: a carrier is admitted exactly when
//! it IS a recognized ruled span — a linear extrusion whose directrix is one
//! straight segment and whose generator direction is constant,
//!
//! ```text
//! S(t, u) = origin + t·directrix + u·generator,   t ∈ [0, 1],  u ∈ [u_lo, u_hi].
//! ```
//!
//! That is exactly the side span of a linear extrude over a straight profile
//! edge, and it is the only ruled span this cell can solve in closed form.
//! Anything not exactly of this ruled form is **not** this family's input — the
//! general path keeps it. Do not approximate a ruling (FSSI-004 stop
//! condition: a ruled carrier that is not exactly recognizable with the landed
//! substrate is a `SPEC_GAP`, never a tessellated stand-in).
//!
//! # Revision-2 scope (corrections, not commentary)
//!
//! 1. `λ = 0` characterizes the INFINITE generator lines. The bounded-surface
//!    intersection additionally requires the **domain-crossing events** in
//!    `(t, s)` — `u − u_lo = 0`, `u − u_hi = 0`, `t = 0`, `t = 1` on `X` and
//!    the mirror on `Y`. A crossing whose `u, v` recovery exits a generator
//!    domain terminates the locus at a **certified boundary event**, never at
//!    a clipped float.
//! 2. The parallel-generator locus `d × e = 0` is EXCISED: `u, v` blow up in a
//!    neighborhood, so it gets its own enclosure test and a typed refusal. For
//!    constant generators the excision set is the whole domain (not a set of
//!    isolated points), so the two-equation two-unknown isolation refuses
//!    `Refusal::NumericallyUnresolved` with the landed witness vocabulary —
//!    `UnresolvedWitness::RootNotIsolated`, no new witness variant
//!    (FSSI-REFUSAL).
//!
//! # Analytic preservation
//!
//! `λ`, its partials, and the `u, v` recovery are exact interval predicates
//! over the `f64` carrier parameters — never float decisions (SFC applies to
//! the search over `(t, s)` only). Dyadic-clean witnesses give degenerate
//! intervals, so exact classifications stay exact. An enclosure that merely
//! contains zero proves nothing: it is `Refusal::NumericallyUnresolved`, never
//! a confident guess (BG-ANA-002). The emitted segment coordinates are computed
//! in `f64`; the certificate's obligation is the on-both-carriers property to
//! machine precision (H-3), exactly as in the landed analytic families. There
//! is no `τ_rep` anywhere.
//!
//! # Consumption
//!
//! The emitted locus is the landed
//! [`AnalyticIntersection::Curve`](crate::analytic::AnalyticIntersection::Curve)
//! ([`ExactCurve::Line`](crate::analytic::ExactCurve::Line)) arm — a bounded
//! segment whose endpoints ARE the certified domain-crossing events — which
//! rides the existing `ContactLocus::Analytic` transverse path in
//! `truck-shapeops/src/boolean/split.rs` (no splitter change; a splitter write
//! is a stop-and-file, not scope growth). A distinct `RuledCrossing` variant is
//! booked ONLY if a consumer needs to distinguish; none does today.

use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Prop, PropMap, Refusal,
    Truth, UnresolvedWitness,
};
use truck_geometry::specifieds::Line;

use super::{AnalyticIntersection, AnalyticOutcome, ExactCurve};

/// One exactly-recognized ruled span (FSSI-004-RULED admission).
///
/// The surface is the linear extrusion
///
/// ```text
/// S(t, u) = origin + t·directrix + u·generator,   t ∈ [0, 1],  u ∈ [u_lo, u_hi],
/// ```
///
/// i.e. the straight-generator span of the extrude family with a straight
/// directrix. In the theory §9.3 vocabulary, `A(t) = origin + t·directrix` is
/// the directrix curve and `generator` is the constant ruling direction `d`.
/// The two parameters are not symmetric: `t` is the directrix (profile)
/// parameter on the fixed unit domain and `u` is the generator parameter on
/// `[u_lo, u_hi]`. Every coordinate must be finite and the directrix must not
/// be parallel to the generator (the span would collapse to a line — a
/// degenerate carrier, not this family's input).
#[derive(Clone, Copy, Debug)]
pub struct RuledSpan {
    /// The directrix start: `origin + u·generator` is the `u`-domain edge that
    /// the directrix sweeps.
    pub origin: Point3,
    /// The directrix chord `S(1, ·) − S(0, ·)`; `t` runs from 0 to 1 along it.
    pub directrix: Vector3,
    /// The constant generator (ruling) direction; `u` runs on `[u_lo, u_hi]`.
    pub generator: Vector3,
    /// The generator parameter lower bound.
    pub u_lo: f64,
    /// The generator parameter upper bound.
    pub u_hi: f64,
}

impl RuledSpan {
    /// The span's plane normal `directrix × generator`, raw (never normalized —
    /// the raw-frame doctrine).
    fn normal(self) -> Vector3 {
        self.directrix.cross(self.generator)
    }

    /// The point on the directrix at parameter `t` (generator parameter 0).
    fn directrix_point(self, t: f64) -> Point3 {
        self.origin + t * self.directrix
    }

    /// Whether the span data is finite and the generator domain non-empty.
    fn structurally_finite(self) -> bool {
        [
            self.origin.x,
            self.origin.y,
            self.origin.z,
            self.directrix.x,
            self.directrix.y,
            self.directrix.z,
            self.generator.x,
            self.generator.y,
            self.generator.z,
            self.u_lo,
            self.u_hi,
        ]
        .iter()
        .all(|c| c.is_finite())
            && self.u_lo < self.u_hi
    }
}

/// The exact scalar predicate `λ(t, s) = (B(s) − A(t)) · (d × e)`.
///
/// `d` and `e` are the two spans' constant generator directions, `A(t)` and
/// `B(s)` their directrices. The predicate is evaluated as an outward-rounded
/// interval enclosure of the `f64` carrier parameters; on a certified locus
/// parameter pair the enclosure is the degenerate `[0, 0]` (decisive zero).
pub fn lambda(x: &RuledSpan, y: &RuledSpan, t: f64, s: f64) -> Interval {
    let a = x.directrix_point(t);
    let b = y.directrix_point(s);
    dot_vec_iv(
        b.x - a.x,
        b.y - a.y,
        b.z - a.z,
        interval_cross(x.generator, y.generator),
    )
}

/// The rational generator recovery on the locus: `(u, v)` at the directrix
/// parameter pair `(t, s)`, as exact interval enclosures.
///
/// Theory §9.3: `u = det[B − A, e, d × e] / ‖d × e‖²` and
/// `v = det[B − A, d, d × e] / ‖d × e‖²`. `None` when the denominator's
/// enclosure cannot exclude zero — the parallel-generator excision, which the
/// pair solver refuses before any recovery is attempted.
pub fn recover_u_v(x: &RuledSpan, y: &RuledSpan, t: f64, s: f64) -> Option<(Interval, Interval)> {
    let a = x.directrix_point(t);
    let b = y.directrix_point(s);
    let delta = [
        interval_at(b.x - a.x),
        interval_at(b.y - a.y),
        interval_at(b.z - a.z),
    ];
    let n = interval_cross(x.generator, y.generator);
    let denom = norm2(n);
    if !excludes_zero(denom) {
        return None;
    }
    // det[delta, w, n] = delta · (w × n).
    let u_num = dot_triple(delta, y.generator, n);
    let v_num = dot_triple(delta, x.generator, n);
    let u = u_num / denom;
    let v = v_num / denom;
    Some((u, v))
}

/// Classifies the ruled pair `x × y` exactly (FSSI-004-RULED).
///
/// Returns:
///
/// - [`AnalyticIntersection::Curve`] with an [`ExactCurve::Line`] segment when
///   the two bounded spans cross transversely — the segment's endpoints are the
///   certified domain-crossing events (`u − u_lo = 0`, …) where the locus
///   enters and leaves either carrier's domain;
/// - [`AnalyticIntersection::Parallel`] for parallel carrier planes that never
///   meet;
/// - [`AnalyticIntersection::Coincident`] for coincident carrier planes;
/// - [`AnalyticIntersection::TangentPoint`] for a degenerate point crossing of
///   two transverse bounded spans;
/// - [`AnalyticIntersection::Empty`] when the bounded spans miss.
///
/// `Err(Refusal::NumericallyUnresolved { .. RootNotIsolated })` is the typed
/// refusal of the parallel-generator excision: with `generator_x × generator_y`
/// decisively zero the `u, v` recovery blows up in a neighborhood of every
/// point, so no locus is certified. A degenerate (non-finite or collapsed)
/// carrier refuses `UnsupportedEnvelope(EnvelopeCase::ChartDegenerate)`, and an
/// undecidable interval predicate refuses `NumericallyUnresolved`, never a
/// guess.
pub fn ruled_pair(x: &RuledSpan, y: &RuledSpan) -> AnalyticOutcome {
    if !x.structurally_finite() || !y.structurally_finite() {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::ChartDegenerate,
        ));
    }

    // Carrier validity: the directrix must not be parallel to its own
    // generator, or the span collapses to a line (a degenerate chart).
    if !is_decisively_nonzero_cross(x.directrix, x.generator)
        || !is_decisively_nonzero_cross(y.directrix, y.generator)
    {
        return Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::ChartDegenerate,
        ));
    }

    // The parallel-generator excision (revision-2 scope item 2). With constant
    // generators the excision set is the whole domain, not isolated points, so
    // the enclosure test refuses typed.
    let gn = interval_cross(x.generator, y.generator);
    if gn.iter().all(|c| decisive_zero(*c)) {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }
    if !gn.iter().any(|c| excludes_zero(*c)) {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }

    let na = x.normal();
    let nb = y.normal();
    let ln = interval_cross(na, nb);
    if ln.iter().any(|c| excludes_zero(*c)) {
        // Transverse carrier planes: the two bounded spans meet in one straight
        // segment (or, degenerately, a point) on the plane-pair intersection
        // line, clipped by the four domain-crossing event families.
        return Ok(match transverse_locus(x, y) {
            TransverseLocus::Segment(p0, p1) => Certified::new(
                AnalyticIntersection::Curve(ExactCurve::Line(Line(p0, p1))),
                exact_certificate(),
            ),
            TransverseLocus::Point(p) => Certified::new(
                AnalyticIntersection::TangentPoint(p),
                exact_certificate(),
            ),
            TransverseLocus::Empty => {
                Certified::new(AnalyticIntersection::Empty, exact_certificate())
            }
        });
    }
    if !ln.iter().all(|c| decisive_zero(*c)) {
        // The plane-normal cross product straddles zero: the carriers are too
        // close to parallel to classify. A stop, not a guess.
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }

    // Parallel carrier planes: the offset between them decides coincidence.
    let offset = interval_at(y.origin.x - x.origin.x) * interval_at(na.x)
        + interval_at(y.origin.y - x.origin.y) * interval_at(na.y)
        + interval_at(y.origin.z - x.origin.z) * interval_at(na.z);
    let value = if decisive_zero(offset) {
        AnalyticIntersection::Coincident
    } else if excludes_zero(offset) {
        AnalyticIntersection::Parallel
    } else {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    };
    Ok(Certified::new(value, exact_certificate()))
}

/// The outcome of the transverse bounded-span solve.
enum TransverseLocus {
    /// A positive-length crossing segment; `p0` precedes `p1` along the
    /// plane-pair intersection line.
    Segment(Point3, Point3),
    /// A degenerate point crossing (the feasible locus interval collapses).
    Point(Point3),
    /// The bounded spans miss each other along the plane-pair intersection
    /// line (the carriers' planes cross, their finite spans do not).
    Empty,
}

/// One affine parameter functional `w(p) = (p − origin)·wvec` extracting a
/// carrier parameter from a point on its plane, together with the domain
/// bounds the parameter clips against.
struct ParamClip {
    /// The constant coefficient vector `wvec` of the affine functional.
    wvec: Vector3,
    /// The span origin the functional is anchored at.
    origin: Point3,
    /// The parameter's lower domain bound.
    lo: f64,
    /// The parameter's upper domain bound.
    hi: f64,
}

impl ParamClip {
    /// The functional value at `p`.
    fn value_at(&self, p: Point3) -> f64 {
        (p - self.origin).dot(self.wvec)
    }

    /// The functional's slope along the crossing line direction.
    fn slope_along(&self, line_dir: Vector3) -> f64 {
        line_dir.dot(self.wvec)
    }
}

/// The Cramer extraction of a span's parameter.
///
/// On the span `S(t, u) = origin + t·d1 + u·d2` with plane normal
/// `n = d1 × d2`, the coordinate along `d1` of a point `p` of the plane is
///
/// ```text
/// t(p) = (p − origin)·(d2 × n) / ‖n‖²,
/// ```
///
/// and the coordinate along `d2` is
///
/// ```text
/// u(p) = −(p − origin)·(d1 × n) / ‖n‖²,
/// ```
///
/// both because `d_i·(d_j × n) = ±‖n‖²` and `d_i·(d_i × n) = 0`. `n·n > 0`
/// whenever the span is a non-degenerate parallelogram, so no division by zero
/// is possible.
fn param_clip(span: RuledSpan, other: Vector3, n: Vector3, invert: bool, lo: f64, hi: f64) -> ParamClip {
    let denom = n.dot(n);
    let sign = if invert { -1.0 } else { 1.0 };
    ParamClip {
        wvec: (other.cross(n)) * (sign / denom),
        origin: span.origin,
        lo,
        hi,
    }
}

/// Solves the transverse crossing of two bounded ruled spans.
///
/// The two infinite carrier planes meet in the line `L = {q + τ·ℓ}`. Each
/// carrier's parameters are affine functionals on `L` (`t`, `u` on `X`; `s`,
/// `v` on `Y`), so every domain-crossing event — `t = 0`, `t = 1`,
/// `u = u_lo`, `u = u_hi`, and the mirror on `Y` — restricts `τ` to a
/// half-line. The feasible set is the intersection of those four intervals, and
/// its endpoints are exactly the certified boundary events: each endpoint
/// saturates at least one of the four domain-crossing event families, and the
/// corresponding point lies on both carriers.
fn transverse_locus(x: &RuledSpan, y: &RuledSpan) -> TransverseLocus {
    let na = x.normal();
    let nb = y.normal();
    let line_dir = na.cross(nb);
    let denom = line_dir.dot(line_dir);
    // The point on both planes closest to the origin, via the two plane
    // constants h_a = na·origin_x and h_b = nb·origin_y (the classic
    // point+direction construction, cf. plane_plane.rs): the result satisfies
    // p·na = h_a and p·nb = h_b, and the division is safe because transverse
    // planes give denom = ‖na × nb‖² > 0.
    let ha = na.dot(x.origin - Point3::new(0.0, 0.0, 0.0));
    let hb = nb.dot(y.origin - Point3::new(0.0, 0.0, 0.0));
    let num = ha * (nb.cross(line_dir)) + hb * (line_dir.cross(na));
    let q = Point3::new(num.x / denom, num.y / denom, num.z / denom);

    // The four domain-crossing event families as affine clips on τ.
    let t_clip = param_clip(*x, x.generator, na, false, 0.0, 1.0);
    let u_clip = param_clip(*x, x.directrix, na, true, x.u_lo, x.u_hi);
    let s_clip = param_clip(*y, y.generator, nb, false, 0.0, 1.0);
    let v_clip = param_clip(*y, y.directrix, nb, true, y.u_lo, y.u_hi);

    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for clip in [t_clip, u_clip, s_clip, v_clip] {
        let w0 = clip.value_at(q);
        let w1 = clip.slope_along(line_dir);
        if w1 == 0.0 {
            // The constraint runs parallel to the crossing line: feasible only
            // if the constant parameter value already lies in its domain.
            if w0 < clip.lo || w0 > clip.hi {
                return TransverseLocus::Empty;
            }
            continue;
        }
        let (a0, a1) = if w1 > 0.0 {
            ((clip.lo - w0) / w1, (clip.hi - w0) / w1)
        } else {
            ((clip.hi - w0) / w1, (clip.lo - w0) / w1)
        };
        lo = lo.max(a0);
        hi = hi.min(a1);
        if lo > hi {
            return TransverseLocus::Empty;
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return TransverseLocus::Empty;
    }
    if lo == hi {
        return TransverseLocus::Point(q + lo * line_dir);
    }
    TransverseLocus::Segment(q + lo * line_dir, q + hi * line_dir)
}

// ---------------------------------------------------------------------------
// Interval predicates (BG-ANA-002 house style)
// ---------------------------------------------------------------------------

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The cross product `a × b`, component-wise in interval arithmetic.
fn interval_cross(a: Vector3, b: Vector3) -> [Interval; 3] {
    [
        interval_at(a.y * b.z - a.z * b.y),
        interval_at(a.z * b.x - a.x * b.z),
        interval_at(a.x * b.y - a.y * b.x),
    ]
}

/// Whether the interval proves a zero: it must be the degenerate `[0, 0]`.
fn decisive_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval proves a nonzero value: it lies strictly away from 0.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// Whether the cross product of two raw direction vectors is decisively
/// nonzero (its interval enclosure excludes zero).
fn is_decisively_nonzero_cross(a: Vector3, b: Vector3) -> bool {
    let c = interval_cross(a, b);
    c.iter().any(|iv| excludes_zero(*iv))
}

/// The interval dot product `delta · (w × n)`, i.e. the scalar triple product
/// `det(delta, w, n)` in outward-rounded arithmetic.
fn dot_triple(delta: [Interval; 3], w: Vector3, n: [Interval; 3]) -> Interval {
    let [wx, wy, wz] = n;
    let c0 = interval_at(w.y) * wz - interval_at(w.z) * wy;
    let c1 = interval_at(w.z) * wx - interval_at(w.x) * wz;
    let c2 = interval_at(w.x) * wy - interval_at(w.y) * wx;
    dot_vec_iv_at(&delta, c0, c1, c2)
}

/// `(dx, dy, dz) · n` for a raw `f64` difference vector and an interval normal,
/// component-wise in interval arithmetic.
fn dot_vec_iv(dx: f64, dy: f64, dz: f64, n: [Interval; 3]) -> Interval {
    let [nx, ny, nz] = n;
    dot_vec_iv_at(&[interval_at(dx), interval_at(dy), interval_at(dz)], nx, ny, nz)
}

/// `delta · (c0, c1, c2)` without any slice indexing (H-1/H-4).
fn dot_vec_iv_at(delta: &[Interval; 3], c0: Interval, c1: Interval, c2: Interval) -> Interval {
    let [d0, d1, d2] = *delta;
    d0 * c0 + d1 * c1 + d2 * c2
}

/// `‖n‖²` for an interval vector, without any slice indexing.
fn norm2(n: [Interval; 3]) -> Interval {
    let [nx, ny, nz] = n;
    nx * nx + ny * ny + nz * nz
}

/// The `Method::Exact` certificate of this family: the classification is
/// decided by decisive interval predicates on the carrier parameters and the
/// locus is the closed-form segment; no budget is consumed (the reduction has
/// no preconditioner and no interval wrapping term).
fn exact_certificate() -> Certificate {
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Certificate {
        props,
        method: Method::Exact,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}
