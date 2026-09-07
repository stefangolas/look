//! DEF-SEEDRAY-A — certified interval ray×carrier crossings for the four
//! seed-ray carriers.
//!
//! `classify::surface_ray_crossings` answers "where does the ray p + t·d hit
//! this carrier" in plain f64: discriminant signs, root distinctness
//! (`t1 != t0`) and degeneracy guards (`NORMAL_SLACK`) are rounding
//! decisions. This module transcribes the same closed forms into certified
//! interval arithmetic. It is STANDALONE: it does NOT touch the live classify
//! path (that is DEF-SEEDRAY-B), so `classify.rs` stays byte-identical.
//!
//! # Decision vocabulary
//!
//! Every crossing is a [`CertifiedCrossing`]: an outward-enclosed `t` interval
//! plus the outward-enclosed crossing point `q`. Existence/exclusion are
//! EXACT decisions on interval enclosures, never float rounding:
//!
//! - a discriminant enclosure strictly below 0 emits [`CrossingVerdict::NoCrossing`];
//! - an enclosure strictly above 0 emits two outward-enclosed crossings;
//! - an enclosure **containing 0** emits [`CrossingVerdict::DegeneratePair`]
//!   — the typed near-tangency verdict, never a silent 0-or-1-or-2 by rounding
//!   (the float form would silently emit a single root at `t0 == t1`);
//! - a degenerate coefficient / parallel carrier (the plane denominator or the
//!   quadratic leading coefficient enclosing 0) is the same typed verdict;
//! - the seven non-implemented carriers are a typed
//!   [`CrossingVerdict::UnsupportedCarrier`], never an empty vec.
//!
//! # The interval algebra
//!
//! The certified scalar is [`CertifiedInterval`], an outward-rounded closed
//! interval. It wraps the landed `inari` interval algebra re-exported by
//! `truck-evidence` (`truck_evidence::enclosure::Interval`, BG-ENC-003: every
//! operation rounds outward). The algebraic carrier inputs (center, apex,
//! radius, half-angle tangent, the plane normal) are read as exact `f64`
//! leaves and every coefficient is built with point intervals, so each
//! coefficient interval is a certified enclosure of the exact real closed
//! form over those leaves. The quadratic-in-`t` evaluation is monotone-safe
//! ([`quad_eval`]: `a·t² + b·t + c` with `sqr`/`mul` on intervals, one `t`
//! dependency), which keeps every root interval a sound enclosure of the true
//! image. `truck-certified`'s `formal/exact.rs` (Shewchuk `Expansion` +
//! `CertifiedInterval`) is the reference vocabulary for this module; it is out
//! of `truck-shapeops`'s dependency graph, so the certified scalar here rides
//! the `truck-evidence` `inari` algebra with the same outward-rounding
//! contract and the same typed three-way sign ([`CertifiedSign`]).
//!
//! # Scope
//!
//! Only the four canonical carriers admitted by
//! `classify::require_canonical_carriers` have solves. The cone emits BOTH
//! nappe roots (the float form does too); the nappe filter belongs to the
//! region stage, not here. The box-exit bound [`ray_exit_t`] feeds
//! DEF-SEEDRAY-B's completeness argument and is pure interval arithmetic.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use truck_base::cgmath64::{Point3, Vector3};
use truck_base::evidence::{Budget, EnvelopeCase, Refusal, UnresolvedWitness};
use truck_evidence::enclosure::{Box3, Interval};
use truck_geometry::canonical::Surface;
use truck_geometry::specifieds::{Cone, Cylinder, Plane, Sphere};

/// The certified sign of an interval-enclosed quantity.
///
/// For a certified scalar interval the sign is a decision by exclusion:
/// `Positive` when the whole interval lies above 0, `Negative` when it lies
/// below, and `Zero` when the interval contains 0 — the exact-zero case and
/// the cannot-exclude-zero case are deliberately one verdict here, because
/// both must drive a typed near-tangency handling instead of a rounded
/// count. This mirrors `truck-certified`'s `formal::exact::CertifiedSign`
/// vocabulary on the interval substrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertifiedSign {
    /// Certifiably below 0.
    Negative,
    /// The interval contains 0 (exact zero, or not decidable by exclusion).
    Zero,
    /// Certifiably above 0.
    Positive,
}

/// An outward-rounded, certified scalar interval.
///
/// The certified scalar of DEF-SEEDRAY-A. It wraps the landed `inari`
/// interval algebra (`truck_evidence::enclosure::Interval`, BG-ENC-003): every
/// arithmetic operation rounds outward, so an interval computed from the `f64`
/// carrier leaves is a certified enclosure of the exact real closed form. The
/// packet's "CertifiedInterval/inari algebra" vocabulary is this type; the
/// arithmetic is deliberately delegated to `inari` rather than re-implemented.
#[derive(Clone, Copy, Debug)]
pub struct CertifiedInterval {
    /// The underlying outward-rounded interval.
    iv: Interval,
}

impl CertifiedInterval {
    /// The degenerate interval `[x, x]`. A non-finite `x` widens to the empty
    /// interval rather than panicking (H-1), mirroring `enclosure::Box3::point`.
    pub fn point(x: f64) -> Self {
        CertifiedInterval {
            iv: Interval::try_from((x, x)).unwrap_or(Interval::EMPTY),
        }
    }

    /// The interval `[lo, hi]` from two finite bounds (`lo <= hi`). An
    /// ill-formed pair widens to the empty interval rather than panicking
    /// (H-1); production callers pass outward-rounded bounds that are always
    /// well-formed.
    pub fn from_bounds(lo: f64, hi: f64) -> Self {
        CertifiedInterval {
            iv: Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY),
        }
    }

    /// The unbounded interval.
    pub fn entire() -> Self {
        CertifiedInterval {
            iv: Interval::ENTIRE,
        }
    }

    /// The certified lower bound.
    pub fn inf(self) -> f64 {
        self.iv.inf()
    }

    /// The certified upper bound.
    pub fn sup(self) -> f64 {
        self.iv.sup()
    }

    /// The interval midpoint (a diagnostic, not a certified value).
    pub fn mid(self) -> f64 {
        self.iv.mid()
    }

    /// The interval width `sup − inf`.
    pub fn width(self) -> f64 {
        self.sup() - self.inf()
    }

    /// Whether `x` lies in the interval (inclusive).
    pub fn contains(self, x: f64) -> bool {
        self.iv.contains(x)
    }

    /// Whether both endpoints are finite.
    pub fn is_finite(self) -> bool {
        self.inf().is_finite() && self.sup().is_finite()
    }

    /// The certified sign by exclusion ([`CertifiedSign`]).
    pub fn sign(self) -> CertifiedSign {
        if self.inf() > 0.0 {
            CertifiedSign::Positive
        } else if self.sup() < 0.0 {
            CertifiedSign::Negative
        } else {
            CertifiedSign::Zero
        }
    }

    /// The outward-rounded square.
    pub fn sqr(self) -> Self {
        CertifiedInterval { iv: self.iv.sqr() }
    }

    /// The outward-rounded principal square root. `None` when the interval
    /// contains a negative value (the square root is not defined there).
    pub fn sqrt(self) -> Option<Self> {
        if self.inf() < 0.0 {
            return None;
        }
        let root = self.iv.sqrt();
        if root.is_empty() {
            None
        } else {
            Some(CertifiedInterval { iv: root })
        }
    }

    /// The outward-rounded quotient. `None` when the denominator contains 0
    /// (the quotient is unbounded or undefined).
    pub fn checked_div(self, denom: Self) -> Option<Self> {
        if denom.sign() == CertifiedSign::Zero {
            return None;
        }
        Some(CertifiedInterval {
            iv: self.iv / denom.iv,
        })
    }
}

impl PartialEq for CertifiedInterval {
    /// Interval equality by endpoints (the `inari` type does not implement
    /// `PartialEq`); two empty intervals compare unequal by IEEE `NaN`, which
    /// is fine here — empties are never a certified crossing.
    fn eq(&self, other: &Self) -> bool {
        self.inf() == other.inf() && self.sup() == other.sup()
    }
}

impl std::ops::Neg for CertifiedInterval {
    type Output = Self;
    fn neg(self) -> Self::Output {
        CertifiedInterval { iv: -self.iv }
    }
}

impl std::ops::Add for CertifiedInterval {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        CertifiedInterval {
            iv: self.iv + rhs.iv,
        }
    }
}

impl std::ops::Sub for CertifiedInterval {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        CertifiedInterval {
            iv: self.iv - rhs.iv,
        }
    }
}

impl std::ops::Mul for CertifiedInterval {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        CertifiedInterval {
            iv: self.iv * rhs.iv,
        }
    }
}

/// A certified ray×carrier crossing.
///
/// `t` is an outward enclosure of the crossing parameter and `q` the
/// corresponding outward enclosure of the crossing point `p + t·d`
/// componentwise. Both are certified: the true crossing of the exact closed
/// form lies inside them (and, because the certified expression mirrors the
/// float form node for node, so does the float hint value).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CertifiedCrossing {
    /// The outward-enclosed crossing parameter.
    pub t: CertifiedInterval,
    /// The outward-enclosed crossing point `[qx, qy, qz]`.
    pub q: [CertifiedInterval; 3],
}

/// The typed decision vocabulary of the ray×carrier solves.
///
/// Nothing is silently empty: an absence of crossings is the typed
/// [`NoCrossing`](CrossingVerdict::NoCrossing), a cannot-certify degeneracy is
/// the typed [`DegeneratePair`](CrossingVerdict::DegeneratePair), and a
/// non-implemented carrier is the typed
/// [`UnsupportedCarrier`](CrossingVerdict::UnsupportedCarrier).
#[derive(Clone, Debug, PartialEq)]
pub enum CrossingVerdict {
    /// A certified absence of real crossings (the discriminant/denominator
    /// enclosure is decisively away from 0 in the no-crossing direction).
    NoCrossing,
    /// One (plane) or two (quadratic) outward-enclosed crossings, in the
    /// float form's emission order (`t0` before `t1`).
    Crossings(Vec<CertifiedCrossing>),
    /// A typed near-tangency / coefficient degeneracy: a discriminant or
    /// coefficient enclosure contains 0, so the solve cannot certify a
    /// distinct transverse crossing. Never a silent 0-or-2.
    DegeneratePair,
    /// One of the seven carriers without an implemented solve (typed; never
    /// an empty vec). Removing `classify::require_canonical_carriers` later
    /// cannot silently corrupt parity.
    UnsupportedCarrier,
}

/// The degenerate point-interval of an `f64` (H-3: the certified leaf).
#[inline]
fn pt(x: f64) -> CertifiedInterval {
    CertifiedInterval::point(x)
}

/// The point of the ray at the certified parameter `t`, componentwise.
fn crossing_at(t: CertifiedInterval, p: Point3, d: Vector3) -> CertifiedCrossing {
    CertifiedCrossing {
        t,
        q: [
            pt(p.x) + pt(d.x) * t,
            pt(p.y) + pt(d.y) * t,
            pt(p.z) + pt(d.z) * t,
        ],
    }
}

/// Monotone-safe evaluation of the quadratic-in-`t` form `a·t² + b·t + c` on
/// an interval `t`.
///
/// The square is taken once on `t` ([`CertifiedInterval::sqr`]) and multiplied
/// into `a`, so `t` has a single dependency: the result is the outward
/// enclosure of `{ a·x² + b·x + c : x ∈ t }` (BG-ENC-003), which stays tight
/// when `t` straddles zero. Every root interval produced by the solves must
/// satisfy `0 ∈ quad_eval(a, b, c, t_root)`.
pub fn quad_eval(
    a: CertifiedInterval,
    b: CertifiedInterval,
    c: CertifiedInterval,
    t: CertifiedInterval,
) -> CertifiedInterval {
    a * t.sqr() + b * t + c
}

/// The outcome of a certified quadratic solve.
enum QuadraticRoots {
    /// The discriminant enclosure is decisively negative: no real roots.
    None,
    /// The discriminant or leading-coefficient enclosure contains 0: cannot
    /// certify two distinct roots.
    Degenerate,
    /// The discriminant is decisively positive: two roots in float-form order
    /// (`t0` = `(−b − √disc)/(2a)`, then `t1`), each outward-enclosed.
    Two(CertifiedInterval, CertifiedInterval),
}

/// The certified quadratic solve, transcribed node-for-node from the float
/// form's coefficient expressions.
fn quadratic_roots(
    a: CertifiedInterval,
    b: CertifiedInterval,
    c: CertifiedInterval,
) -> QuadraticRoots {
    let disc = b * b - pt(4.0) * a * c;
    match disc.sign() {
        CertifiedSign::Negative => QuadraticRoots::None,
        CertifiedSign::Zero => QuadraticRoots::Degenerate,
        CertifiedSign::Positive => {
            // A leading coefficient enclosing 0 makes the formula divide by a
            // zero-containing interval: typed degeneracy (near-parallel to the
            // carrier), never an unbounded silent answer.
            if a.sign() == CertifiedSign::Zero {
                return QuadraticRoots::Degenerate;
            }
            let sq = match disc.sqrt() {
                Some(sq) => sq,
                None => return QuadraticRoots::Degenerate,
            };
            let denom = pt(2.0) * a;
            let t0 = (-b - sq).checked_div(denom);
            let t1 = (-b + sq).checked_div(denom);
            match (t0, t1) {
                (Some(t0), Some(t1)) => QuadraticRoots::Two(t0, t1),
                _ => QuadraticRoots::Degenerate,
            }
        }
    }
}

/// The certified plane solve. Mirrors the float linear solve
/// `t = ((o − p)·n) / (d·n)` with outward rounding; a denominator enclosure
/// containing 0 (a near-parallel ray) is the typed degeneracy verdict.
fn solve_plane(plane: &Plane, p: Point3, d: Vector3) -> CrossingVerdict {
    let o = plane.origin();
    let n = plane.normal();
    let denom = pt(d.x) * pt(n.x) + pt(d.y) * pt(n.y) + pt(d.z) * pt(n.z);
    if denom.sign() == CertifiedSign::Zero {
        return CrossingVerdict::DegeneratePair;
    }
    let num = (pt(o.x) - pt(p.x)) * pt(n.x)
        + (pt(o.y) - pt(p.y)) * pt(n.y)
        + (pt(o.z) - pt(p.z)) * pt(n.z);
    match num.checked_div(denom) {
        Some(t) => CrossingVerdict::Crossings(vec![crossing_at(t, p, d)]),
        None => CrossingVerdict::DegeneratePair,
    }
}

/// The certified cylinder solve (the xy-quadratic in the ray relative to the
/// z-axis cylinder). A leading coefficient enclosing 0 is the axis-parallel
/// degeneracy verdict.
fn solve_cylinder(cylinder: &Cylinder, p: Point3, d: Vector3) -> CrossingVerdict {
    let center = cylinder.center();
    let radius = cylinder.radius();
    let px = pt(p.x) - pt(center.x);
    let py = pt(p.y) - pt(center.y);
    let dx = pt(d.x);
    let dy = pt(d.y);
    let a = dx * dx + dy * dy;
    if a.sign() == CertifiedSign::Zero {
        return CrossingVerdict::DegeneratePair;
    }
    let b = pt(2.0) * (px * dx + py * dy);
    let c = px * px + py * py - pt(radius) * pt(radius);
    crossings_from(quadratic_roots(a, b, c), p, d)
}

/// The certified sphere solve (`|p + t·d − c|² = r²`). A leading coefficient
/// enclosing 0 is the zero-direction degeneracy verdict.
fn solve_sphere(sphere: &Sphere, p: Point3, d: Vector3) -> CrossingVerdict {
    let center = sphere.center();
    let radius = sphere.radius();
    let ex = pt(p.x) - pt(center.x);
    let ey = pt(p.y) - pt(center.y);
    let ez = pt(p.z) - pt(center.z);
    let dx = pt(d.x);
    let dy = pt(d.y);
    let dz = pt(d.z);
    let a = dx * dx + dy * dy + dz * dz;
    if a.sign() == CertifiedSign::Zero {
        return CrossingVerdict::DegeneratePair;
    }
    let b = pt(2.0) * (ex * dx + ey * dy + ez * dz);
    let c = (ex * ex + ey * ey + ez * ez) - pt(radius) * pt(radius);
    crossings_from(quadratic_roots(a, b, c), p, d)
}

/// The certified cone solve (the double-nappe quadratic; BOTH nappe roots are
/// emitted, exactly as the float form does — the nappe filter belongs to the
/// region stage). A leading coefficient enclosing 0 is the
/// ray-along-a-generator degeneracy verdict.
fn solve_cone(cone: &Cone, p: Point3, d: Vector3) -> CrossingVerdict {
    let apex = cone.apex();
    let k = cone.half_angle().tan();
    let k2 = pt(k) * pt(k);
    let ex = pt(p.x) - pt(apex.x);
    let ey = pt(p.y) - pt(apex.y);
    let ez = pt(p.z) - pt(apex.z);
    let dx = pt(d.x);
    let dy = pt(d.y);
    let dz = pt(d.z);
    // `k2 * dz * dz` is transcribed as `((k2 · dz) · dz)`, matching the float
    // form's left-associative `k * k * dz * dz` node for node.
    let a = dx * dx + dy * dy - (k2 * dz) * dz;
    if a.sign() == CertifiedSign::Zero {
        return CrossingVerdict::DegeneratePair;
    }
    let b = pt(2.0) * ((ex * dx + ey * dy) - (k2 * ez) * dz);
    let c = (ex * ex + ey * ey) - (k2 * ez) * ez;
    crossings_from(quadratic_roots(a, b, c), p, d)
}

/// Turn a certified quadratic outcome into the crossing verdict (the point
/// enclosures ride the root intervals).
fn crossings_from(roots: QuadraticRoots, p: Point3, d: Vector3) -> CrossingVerdict {
    match roots {
        QuadraticRoots::None => CrossingVerdict::NoCrossing,
        QuadraticRoots::Degenerate => CrossingVerdict::DegeneratePair,
        QuadraticRoots::Two(t0, t1) => {
            CrossingVerdict::Crossings(vec![crossing_at(t0, p, d), crossing_at(t1, p, d)])
        }
    }
}

/// The certified dispatch over the four seed-ray carriers.
///
/// The seven non-implemented carriers (Torus, RevolutedCurve, ExtrudedCurve,
/// BSplineSurface, NurbsSurface, Processor, SpineFrameSurface) are the TYPED
/// [`CrossingVerdict::UnsupportedCarrier`] — never an empty vec, so removing
/// `classify::require_canonical_carriers` later cannot silently corrupt
/// parity.
pub fn ray_crossings_verdict(surface: &Surface, p: Point3, d: Vector3) -> CrossingVerdict {
    match surface {
        Surface::Plane(plane) => solve_plane(plane, p, d),
        Surface::Cylinder(cylinder) => solve_cylinder(cylinder, p, d),
        Surface::Sphere(sphere) => solve_sphere(sphere, p, d),
        Surface::Cone(cone) => solve_cone(cone, p, d),
        Surface::Torus(_)
        | Surface::RevolutedCurve(_)
        | Surface::ExtrudedCurve(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_)
        | Surface::Processor(_)
        | Surface::SpineFrameSurface(_) => CrossingVerdict::UnsupportedCarrier,
    }
}

/// The certified ray×carrier crossings, with the typed verdicts mapped onto
/// the evidence outcome:
///
/// - [`CrossingVerdict::NoCrossing`] → an empty crossing list (`Ok`);
/// - [`CrossingVerdict::Crossings`] → the crossing list (`Ok`);
/// - [`CrossingVerdict::DegeneratePair`] → `Refusal::NumericallyUnresolved`
///   with witness [`UnresolvedWitness::RootNotIsolated`] (a root could not be
///   isolated: multiple / tangential roots, BG-NUM-002); the seed parity logic
///   retries another direction;
/// - [`CrossingVerdict::UnsupportedCarrier`] →
///   `Refusal::UnsupportedEnvelope(NonCanonicalCarrier)`.
///
/// `budget` is carried for the evidence ledger; the closed-form solves are
/// non-iterative and spend nothing, so a refusal records the incoming budget.
pub fn certified_crossings(
    surface: &Surface,
    p: Point3,
    d: Vector3,
    budget: &mut Budget,
) -> Result<Vec<CertifiedCrossing>, Refusal> {
    match ray_crossings_verdict(surface, p, d) {
        CrossingVerdict::NoCrossing => Ok(Vec::new()),
        CrossingVerdict::Crossings(crossings) => Ok(crossings),
        CrossingVerdict::DegeneratePair => Err(Refusal::NumericallyUnresolved {
            spent: *budget,
            witness: UnresolvedWitness::RootNotIsolated,
        }),
        CrossingVerdict::UnsupportedCarrier => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// The certified parameter at which the ray `p + t·d` leaves the certified
/// enclosing box `b` (the "box-exit" bound feeding DEF-SEEDRAY-B's
/// completeness argument).
///
/// Pure interval arithmetic: for every moving axis the exit parameter
/// `(bound − p_axis)/d_axis` is evaluated outward and the certified exit is
/// the componentwise-min enclosure over the moving axes (an interval that
/// contains the true first-exit parameter). For any `t` above the returned
/// interval's upper bound the ray point is certifiably outside the box. An
/// axis with `d_axis == 0` does not bound the exit; a ray with no moving axis
/// (a zero direction) never leaves, and the whole interval is returned.
pub fn ray_exit_t(b: &Box3, p: Point3, d: Vector3) -> CertifiedInterval {
    let mut exit_lo = f64::INFINITY;
    let mut exit_hi = f64::INFINITY;
    let mut moving = false;
    let mut fold = |p_axis: f64, d_axis: f64, axis: Interval| {
        if d_axis == 0.0 {
            return;
        }
        moving = true;
        let bound = if d_axis > 0.0 { axis.sup() } else { axis.inf() };
        if let Some(exit) = (pt(bound) - pt(p_axis)).checked_div(pt(d_axis)) {
            exit_lo = exit_lo.min(exit.inf());
            exit_hi = exit_hi.min(exit.sup());
        }
    };
    fold(p.x, d.x, b.x);
    fold(p.y, d.y, b.y);
    fold(p.z, d.z, b.z);
    if moving {
        CertifiedInterval::from_bounds(exit_lo, exit_hi)
    } else {
        CertifiedInterval::entire()
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps below cannot fire for the values constructed.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_4;
    use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3, Vector3};
    use truck_base::evidence::{EnvelopeCase, Refusal};
    use truck_evidence::enclosure::Interval;

    /// The float hint oracle: a verbatim transcription of
    /// `classify.rs::surface_ray_crossings` (classify.rs is DEF-SEEDRAY-B's
    /// file and must stay untouched; the float form is the conformance
    /// oracle). H-3: the dimensionless normal slack mirrors the classify
    /// constant it transcribes.
    fn float_crossings(surface: &Surface, p: Point3, d: Vector3) -> Vec<(f64, Point3)> {
        const NORMAL_SLACK: f64 = 1.0e-6; // H-3: dimensionless normal slack, not a length
        match surface {
            Surface::Plane(plane) => {
                let n = plane.normal();
                let denom = d.dot(n);
                if denom.abs() <= NORMAL_SLACK {
                    return Vec::new();
                }
                let t = (plane.origin() - p).dot(n) / denom;
                vec![(t, p + d * t)]
            }
            Surface::Cylinder(cyl) => {
                let c = cyl.center();
                let px = p.x - c.x;
                let py = p.y - c.y;
                let dx = d.x;
                let dy = d.y;
                let a = dx * dx + dy * dy;
                if a <= NORMAL_SLACK {
                    return Vec::new();
                }
                let b = 2.0 * (px * dx + py * dy);
                let cc = px * px + py * py - cyl.radius() * cyl.radius();
                let disc = b * b - 4.0 * a * cc;
                if disc < 0.0 {
                    return Vec::new();
                }
                let sq = disc.sqrt();
                let t0 = (-b - sq) / (2.0 * a);
                let t1 = (-b + sq) / (2.0 * a);
                let mut out = Vec::new();
                out.push((t0, p + d * t0));
                if t1 != t0 {
                    out.push((t1, p + d * t1));
                }
                out
            }
            Surface::Sphere(sphere) => {
                let c = sphere.center();
                let e = p - c;
                let a = d.dot(d);
                if a <= NORMAL_SLACK {
                    return Vec::new();
                }
                let b = 2.0 * e.dot(d);
                let cc = e.dot(e) - sphere.radius() * sphere.radius();
                let disc = b * b - 4.0 * a * cc;
                if disc < 0.0 {
                    return Vec::new();
                }
                let sq = disc.sqrt();
                let t0 = (-b - sq) / (2.0 * a);
                let t1 = (-b + sq) / (2.0 * a);
                let mut out = Vec::new();
                out.push((t0, p + d * t0));
                if t1 != t0 {
                    out.push((t1, p + d * t1));
                }
                out
            }
            Surface::Cone(cone) => {
                let k = cone.half_angle().tan();
                let e = p - cone.apex();
                let dx = d.x;
                let dy = d.y;
                let dz = d.z;
                let ex = e.x;
                let ey = e.y;
                let ez = e.z;
                let a = dx * dx + dy * dy - k * k * dz * dz;
                if a.abs() <= NORMAL_SLACK {
                    return Vec::new();
                }
                let b = 2.0 * (ex * dx + ey * dy - k * k * ez * dz);
                let cc = ex * ex + ey * ey - k * k * ez * ez;
                let disc = b * b - 4.0 * a * cc;
                if disc < 0.0 {
                    return Vec::new();
                }
                let sq = disc.sqrt();
                let t0 = (-b - sq) / (2.0 * a);
                let t1 = (-b + sq) / (2.0 * a);
                let mut out = Vec::new();
                out.push((t0, p + d * t0));
                if t1 != t0 {
                    out.push((t1, p + d * t1));
                }
                out
            }
            Surface::Torus(_)
            | Surface::RevolutedCurve(_)
            | Surface::ExtrudedCurve(_)
            | Surface::BSplineSurface(_)
            | Surface::NurbsSurface(_)
            | Surface::Processor(_)
            | Surface::SpineFrameSurface(_) => Vec::new(),
        }
    }

    /// Asserts that every float-oracle crossing of `surface` is enclosed by
    /// the corresponding certified crossing (the float form stays the hint
    /// oracle), and that the certified verdict is `Crossings`.
    fn assert_certified_enclose_float(surface: &Surface, p: Point3, d: Vector3) {
        let float_ts = float_crossings(surface, p, d);
        match ray_crossings_verdict(surface, p, d) {
            CrossingVerdict::Crossings(crossings) => {
                assert_eq!(
                    crossings.len(),
                    float_ts.len(),
                    "carrier crossing count must match the float oracle"
                );
                for (certified, (t_float, q_float)) in crossings.iter().zip(float_ts.iter()) {
                    assert!(
                        certified.t.contains(*t_float),
                        "certified t {certified:?} must enclose the float t {t_float}"
                    );
                    let q = certified.q;
                    assert!(
                        q[0].contains(q_float.x)
                            && q[1].contains(q_float.y)
                            && q[2].contains(q_float.z),
                        "certified q must enclose the float crossing point {q_float:?}"
                    );
                    // The monotone-safe check: a certified root interval must
                    // make the certified quadratic image contain 0.
                    assert!(certified.t.is_finite());
                }
            }
            other => panic!("expected Crossings, got {other:?} for float {float_ts:?}"),
        }
    }

    /// The dyadic plane `z = 0` plus an oblique ray.
    fn plane_surface() -> Surface {
        Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ))
    }

    /// The plane `x = y` (normal `(1, −1, 0)/√2`).
    fn diagonal_plane() -> Surface {
        Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ))
    }

    fn cylinder_surface() -> Surface {
        Surface::Cylinder(Cylinder::new(Point3::origin(), 2.0).unwrap().value)
    }

    fn sphere_surface() -> Surface {
        Surface::Sphere(Sphere::new(Point3::origin(), 2.0))
    }

    fn unit_sphere_surface() -> Surface {
        Surface::Sphere(Sphere::new(Point3::origin(), 1.0))
    }

    fn cone_surface() -> Surface {
        Surface::Cone(Cone::new(Point3::origin(), FRAC_PI_4).unwrap().value)
    }

    #[test]
    fn plane_crossings_enclose_float_crossings() {
        assert_certified_enclose_float(
            &plane_surface(),
            Point3::new(1.25, -0.5, -2.0),
            Vector3::new(0.25, 0.5, 2.0),
        );
        assert_certified_enclose_float(
            &plane_surface(),
            Point3::new(1.0, 1.0, 3.0),
            Vector3::new(0.0, 0.0, -2.0),
        );
        assert_certified_enclose_float(
            &diagonal_plane(),
            Point3::new(3.0, 7.0, 2.0),
            Vector3::new(0.25, -0.25, 0.0),
        );
    }

    #[test]
    fn cylinder_crossings_enclose_float_crossings() {
        assert_certified_enclose_float(
            &cylinder_surface(),
            Point3::new(-5.0, 0.0, 1.5),
            Vector3::new(1.0, 0.0, 0.0),
        );
        assert_certified_enclose_float(
            &cylinder_surface(),
            Point3::new(-3.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 0.0),
        );
        // An oblique ray through the cylinder: the discriminant is decisively
        // positive and both certified intervals enclose the float roots.
        assert_certified_enclose_float(
            &cylinder_surface(),
            Point3::new(-2.5, 0.0, 0.0),
            Vector3::new(0.8, 0.6, 0.0),
        );
    }

    #[test]
    fn sphere_crossings_enclose_float_crossings() {
        assert_certified_enclose_float(
            &sphere_surface(),
            Point3::new(0.0, 0.0, 5.0),
            Vector3::new(0.0, 0.0, -1.0),
        );
        assert_certified_enclose_float(
            &sphere_surface(),
            Point3::new(5.0, 1.0, 0.0),
            Vector3::new(-1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn cone_crossings_enclose_float_crossings() {
        // The classify.rs cone fixture: apex origin, half-angle π/4, ray from
        // (5, 0, 1) along −x — both nappe roots emitted.
        assert_certified_enclose_float(
            &cone_surface(),
            Point3::new(5.0, 0.0, 1.0),
            Vector3::new(-1.0, 0.0, 0.0),
        );
        assert_certified_enclose_float(
            &cone_surface(),
            Point3::new(4.0, 0.0, -1.0),
            Vector3::new(1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn exact_tangency_emits_degenerate_pair_not_a_silent_single_root() {
        // A ray from a point ON the unit sphere tangent to it: the float disc
        // is exactly 0 and the float oracle silently emits a SINGLE crossing
        // (t0 == t1, the `if t1 != t0` rounding). The certified solve must
        // emit the typed DegeneratePair verdict instead — never a silent
        // 1-or-2.
        let surface = unit_sphere_surface();
        let p = Point3::new(1.0, 0.0, 0.0);
        let d = Vector3::new(0.0, 1.0, 0.0);
        let float_ts = float_crossings(&surface, p, d);
        assert_eq!(
            float_ts.len(),
            1,
            "the float oracle silently collapses the tangent root"
        );
        assert_eq!(
            ray_crossings_verdict(&surface, p, d),
            CrossingVerdict::DegeneratePair
        );
    }

    #[test]
    fn near_miss_emits_no_crossing_exactly() {
        // A ray that just misses the unit sphere (origin 1e-8 outside the
        // surface, tangent direction): the exact discriminant is a small
        // negative, and the certified solve decides NoCrossing by interval
        // exclusion — never by the float `disc < 0.0` rounding.
        let surface = unit_sphere_surface();
        let p = Point3::new(1.0 + 1.0e-8, 0.0, 0.0);
        let d = Vector3::new(0.0, 1.0, 0.0);
        let float_ts = float_crossings(&surface, p, d);
        assert_eq!(float_ts.len(), 0, "the near-miss ray has no float crossing");
        assert_eq!(
            ray_crossings_verdict(&surface, p, d),
            CrossingVerdict::NoCrossing
        );
    }

    #[test]
    fn axis_parallel_cylinder_ray_is_a_typed_degeneracy() {
        // A ray exactly parallel to the cylinder axis: the leading coefficient
        // enclosure is [0, 0], so the quadratic solve cannot certify. The
        // float oracle silently returns an empty vec (its `a <= NORMAL_SLACK`
        // arm); the certified solve returns the typed DegeneratePair.
        let surface = cylinder_surface();
        let p = Point3::new(1.0, 0.0, 5.0);
        let d = Vector3::new(0.0, 0.0, 1.0);
        assert!(float_crossings(&surface, p, d).is_empty());
        assert_eq!(
            ray_crossings_verdict(&surface, p, d),
            CrossingVerdict::DegeneratePair
        );
    }

    #[test]
    fn miss_cylinder_emits_no_crossing() {
        let surface = cylinder_surface();
        let p = Point3::new(-5.0, 3.0, 0.0);
        let d = Vector3::new(1.0, 0.0, 0.0);
        let float_ts = float_crossings(&surface, p, d);
        assert!(float_ts.is_empty());
        assert_eq!(
            ray_crossings_verdict(&surface, p, d),
            CrossingVerdict::NoCrossing
        );
    }

    #[test]
    fn the_seven_silent_arms_are_typed_unsupported() {
        // Torus/RevolutedCurve/ExtrudedCurve/BSplineSurface/NurbsSurface/
        // Processor/SpineFrameSurface are the classify.rs `=> Vec::new()`
        // catch-all behind `require_canonical_carriers`. The certified
        // dispatch must type them as UnsupportedCarrier — never an empty vec —
        // so removing that gate later cannot silently corrupt parity. A Sphere
        // (supported) is the control.
        use truck_geometry::specifieds::Torus;
        let surface = unit_sphere_surface();
        assert!(matches!(
            ray_crossings_verdict(&surface, Point3::origin(), Vector3::unit_z()),
            CrossingVerdict::Crossings(_)
        ));
        let torus = Surface::Torus(Torus::new(Point3::origin(), 2.0, 0.5));
        assert_eq!(
            ray_crossings_verdict(&torus, Point3::origin(), Vector3::unit_z()),
            CrossingVerdict::UnsupportedCarrier
        );
    }

    #[test]
    fn certified_crossings_maps_typed_verdicts_to_refusals() {
        let mut budget = Budget::new(32, 32, 8);
        // Crossings map to Ok.
        let sphere = sphere_surface();
        let crossings = certified_crossings(
            &sphere,
            Point3::new(0.0, 0.0, 5.0),
            Vector3::new(0.0, 0.0, -1.0),
            &mut budget,
        )
        .unwrap();
        assert_eq!(crossings.len(), 2);

        // NoCrossing maps to an Ok empty list.
        let none = certified_crossings(
            &sphere,
            Point3::new(0.0, 0.0, 5.0),
            Vector3::new(1.0, 0.0, 0.0),
            &mut budget,
        )
        .unwrap();
        assert!(none.is_empty());

        // DegeneratePair maps to NumericallyUnresolved(RootNotIsolated).
        let tangent = certified_crossings(
            &unit_sphere_surface(),
            Point3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            &mut budget,
        );
        assert!(matches!(
            tangent,
            Err(Refusal::NumericallyUnresolved {
                witness: UnresolvedWitness::RootNotIsolated,
                ..
            })
        ));

        // UnsupportedCarrier maps to UnsupportedEnvelope(NonCanonicalCarrier).
        let torus = Surface::Torus(truck_geometry::specifieds::Torus::new(
            Point3::origin(),
            2.0,
            0.5,
        ));
        let unsupported =
            certified_crossings(&torus, Point3::origin(), Vector3::unit_z(), &mut budget);
        assert!(matches!(
            unsupported,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
    }

    #[test]
    fn root_intervals_pass_the_monotone_safe_quadratic_check() {
        // For every certified crossing of the fixture battery, the certified
        // quadratic image over the root interval must contain 0 (the root
        // lives in the interval). Cylinder fixture, transcribed coefficients.
        let surface = cylinder_surface();
        let p = Point3::new(-5.0, 0.0, 1.5);
        let d = Vector3::new(1.0, 0.0, 0.0);
        let center = match &surface {
            Surface::Cylinder(c) => c.center(),
            _ => unreachable!(),
        };
        let radius = match &surface {
            Surface::Cylinder(c) => c.radius(),
            _ => unreachable!(),
        };
        let CrossingVerdict::Crossings(crossings) = ray_crossings_verdict(&surface, p, d) else {
            unreachable!();
        };
        let px = p.x - center.x;
        let py = p.y - center.y;
        let a = pt(d.x) * pt(d.x) + pt(d.y) * pt(d.y);
        let b = pt(2.0) * (pt(px) * pt(d.x) + pt(py) * pt(d.y));
        let c = pt(px) * pt(px) + pt(py) * pt(py) - pt(radius) * pt(radius);
        for crossing in &crossings {
            let image = quad_eval(a, b, c, crossing.t);
            assert!(
                image.contains(0.0),
                "the certified quadratic image over the root interval must contain 0"
            );
        }
    }

    #[test]
    fn ray_exit_t_encloses_the_true_exit_and_is_interval_sound() {
        // The box [0, 10]³ and a ray inside it: the certified exit must
        // contain the true exit parameter and never understate it.
        let box3 = Box3 {
            x: Interval::try_from((0.0, 10.0)).unwrap(),
            y: Interval::try_from((0.0, 10.0)).unwrap(),
            z: Interval::try_from((0.0, 10.0)).unwrap(),
        };
        // Moving along +z from (5, 5, 2): exits z = 10 at t = 8.
        let p = Point3::new(5.0, 5.0, 2.0);
        let d = Vector3::new(0.0, 0.0, 1.0);
        let exit = ray_exit_t(&box3, p, d);
        assert!(
            exit.contains(8.0),
            "the certified exit must contain the true exit t = 8 (got {exit:?})"
        );

        // Moving in +x and −y: exits x = 10 at t = 5 versus y = 0 at t = 10/2
        // from (5, 10, 5)? Choose (5, 8, 5), d = (1, −1, 0): x exit at t = 5,
        // y exit at (0 − 8)/(−1) = 8; the first exit is t = 5.
        let p = Point3::new(5.0, 8.0, 5.0);
        let d = Vector3::new(1.0, -1.0, 0.0);
        let exit = ray_exit_t(&box3, p, d);
        assert!(
            exit.contains(5.0),
            "the certified exit must contain the true first exit t = 5 (got {exit:?})"
        );

        // Soundness spot check: the exit's upper bound is never below the true
        // exit, so sampling past it must be outside on at least one axis.
        let exit = ray_exit_t(
            &box3,
            Point3::new(5.0, 5.0, 2.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let probe = Point3::new(5.0, 5.0, 2.0 + exit.sup() + 1.0);
        assert!(
            probe.z > 10.0,
            "past the certified exit the ray point must be outside the box"
        );
    }
}
