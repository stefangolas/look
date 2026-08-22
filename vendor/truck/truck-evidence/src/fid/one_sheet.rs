//! BG-FID-008: the one-sheet condition (iv-a) for CURVE components.
//!
//! Conditions (i)-(iii) of the isotopy lemma make the normal projection
//! restricted to an approximant a proper local homeomorphism — a covering of
//! SOME constant finite degree. They do NOT force degree one, so a checker
//! implementing only (i)-(iii) passes topologically wrong output. This module
//! discharges **(iv-a)** for curves: [`fibre_degree_one`] certifies that one
//! witnessed normal disc meets the approximant exactly once, by root isolation
//! over the whole approximant parameter span (the Krawczyk operator,
//! BG-NUM-003, N=1) with certified exclusion everywhere else.
//!
//! What a positive answer establishes is degree-one ON ONE DISC. Nothing in
//! this module is an isotopy, homeomorphism or one-sheet certificate, and
//! nothing claims any bridge lemma as proved: the bridge lemmas L-TUBE /
//! L-COVERING / L-SEPARATES remain OPEN obligations that this module cites as
//! fed, never as proved.
//!
//! Deferrals (both documented, neither stubbed):
//! - the SURFACE case needs 2D root certification in the normal bundle and
//!   lands with **BG-FID-005**, where the emitter's own cell partition makes
//!   discharge (iv-b) free;
//! - discharge **(iv-b)** itself also lands with BG-FID-005 — no emitter
//!   partition exists here to feed it.
//!
//! The reduction to a single fibre is licensed ONLY by conditions (i)-(iii)
//! already holding on this component; the function takes no (i)-(iii) data and
//! its contract states that precondition verbatim.

#![deny(clippy::unwrap_used)]

use crate::enclosure::{interval_at, Box3, EnclosureCurve, Interval};
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::evidence::Budget;

/// What the witnessed disc certified.
///
/// @feeds-open-lemma FID-L-COVERING      # degree-one fibre evidence, per component
/// @establishes certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibreMultiplicity {
    /// Exactly one approximant point on the closed normal disc at x.
    ExactlyOne,
    /// Certified cardinality != 1 on that disc. `count` is the CERTIFIED
    /// lower bound on distinct geometric intersections; `count == 0` means
    /// the fibre missed entirely (a coverage violation, equally fatal).
    NotOne { count: usize },
}

/// Typed failures. SheetCountUnresolved is EPISTEMIC: the root count could
/// not be certified within budget — it is a claim about the run, never
/// about geometry in either direction.
///
/// @feeds-open-lemma FID-L-COVERING      # degree-one fibre evidence, per component
/// @establishes certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneSheetError {
    /// The witness parameter's tangent is undefined or zero-magnitude.
    InvalidWitness,
    /// Root isolation did not resolve within budget / width floor.
    SheetCountUnresolved,
}

/// Certifies the fibre cardinality of one witnessed normal disc: how many
/// times the approximant meets the closed normal disc at `x = exact.subs(t_x)`.
///
/// @feeds-open-lemma FID-L-COVERING      # degree-one fibre evidence, per component
/// @establishes certified fibre cardinality on ONE witnessed normal disc
/// @does-not-establish
///   isotopy | homeomorphism | side separation | whole-span one-sheet
///
/// @precondition BG-FID-003 (i)-(iii) hold on this component; calling this without them proves nothing.
///
/// The witness point is `x = exact.subs(t_x)` and the unit tangent `u` is the
/// midpoint of `exact.enclose_der(1, degenerate(t_x))`, magnitude-checked.
/// The normal disc `{ p : <p - x, u> == 0, |p - x| <= eps }` is intersected
/// with the approximant by isolating the roots of the univariate equation
/// `h(t) = <approx.subs(t) - x, u> == 0` over the whole approximant parameter
/// span, with `|approx.subs(t) - x| <= eps` as the disc-membership gate.
///
/// Root isolation is the Krawczyk operator (BG-NUM-003) on the N=1 system
/// `f(t) = h(t)` over a bisection worklist: interval `h` prunes boxes whose
/// plane interval excludes 0; the disc test prunes boxes whose whole image
/// lies beyond `eps`; a box that survives runs Krawczyk, whose `Unique` proof
/// certifies one root in the box. Certified roots whose point-boxes OVERLAP
/// are the same geometric point and count ONCE (a closed curve hits the same
/// point at `t*` and `t* + period`). Count > 1 exits early as `NotOne`;
/// worklist drained with count != 1 is `NotOne` (0 included); exactly one
/// in-disc intersection is `ExactlyOne`.
///
/// A tangential contact (an even-multiplicity touch of the plane inside the
/// ball) never yields `Unique` and drains to `SheetCountUnresolved` — that is
/// correct, and reporting degree one for it would be the classic false pass.
///
/// `InvalidWitness`: `eps <= 0`, non-finite `eps`, `t_x` outside the exact
/// curve's parameter range, or a tangent enclosure containing the zero vector
/// (or an undefined tangent midpoint).
pub fn fibre_degree_one(
    exact: &impl EnclosureCurve,
    approx: &impl EnclosureCurve,
    t_x: f64,
    eps: f64,
    budget: &mut Budget,
) -> Result<FibreMultiplicity, OneSheetError> {
    if eps <= 0.0 || !eps.is_finite() || !t_x.is_finite() {
        return Err(OneSheetError::InvalidWitness);
    }
    if let Some((lo, hi)) = exact.try_range_tuple() {
        if t_x < lo || t_x > hi {
            return Err(OneSheetError::InvalidWitness);
        }
    }

    let x = exact.subs(t_x);
    let tangent_box = exact.enclose_der(1, interval_at(t_x));
    if box3_contains_zero(&tangent_box) {
        return Err(OneSheetError::InvalidWitness);
    }
    let mid = Vector3::new(
        tangent_box.x.mid(),
        tangent_box.y.mid(),
        tangent_box.z.mid(),
    );
    if !(mid.x.is_finite() && mid.y.is_finite() && mid.z.is_finite()) {
        return Err(OneSheetError::InvalidWitness);
    }
    let u = mid.normalize();

    // The bisection worklist lives on the approximant's (bounded) parameter
    // range. An unbounded or degenerate range cannot be searched exhaustively.
    let Some((a_lo, a_hi)) = approx.try_range_tuple() else {
        return Err(OneSheetError::SheetCountUnresolved);
    };
    if !(a_lo.is_finite() && a_hi.is_finite()) || a_lo >= a_hi {
        return Err(OneSheetError::SheetCountUnresolved);
    }

    let system = FibreSystem { approx, x, u };
    let u_b = u_box(u);
    let x_b = Box3::point(x);

    let mut count: usize = 0;
    let mut in_disc: Vec<Box3> = Vec::new();
    let mut worklist: Vec<Interval> =
        vec![Interval::try_from((a_lo, a_hi)).unwrap_or(Interval::EMPTY)];

    while let Some(tt) = worklist.pop() {
        let image = approx.enclose(tt);
        // Step 1: interval h; prune when it excludes 0 (no plane crossing).
        let h = dot_box(&box_minus_point(&image, x), &u_b);
        if !h.contains(0.0) {
            continue;
        }
        // Step 2: prune when the whole box image lies beyond the disc.
        if box_distance(&image, &x_b) > eps {
            continue;
        }
        let width = tt.sup() - tt.inf();
        match krawczyk(&system, &[tt], budget) {
            Ok(cert) => match cert.value {
                KrawczykProof::Unique => {
                    if width <= DISC_DECIDE_WIDTH {
                        // A narrow box around the root: the disc-membership
                        // decision is trustworthy, and the box is resolved.
                        let d = box_distance(&image, &x_b);
                        if d <= eps && !in_disc.iter().any(|pb| boxes_overlap(pb, &image)) {
                            count += 1;
                            if count > 1 {
                                return Ok(FibreMultiplicity::NotOne { count });
                            }
                            in_disc.push(image);
                        }
                    } else {
                        // The certified root lives in some sub-box of `tt`; the
                        // box may hold more roots, so keep subdividing to find
                        // them (the dedupe rule re-merges the same point).
                        push_children(tt, &mut worklist, budget)?;
                    }
                }
                KrawczykProof::NoRoot => {}
            },
            Err(_) => {
                if width <= WIDTH_FLOOR {
                    return Err(OneSheetError::SheetCountUnresolved);
                }
                push_children(tt, &mut worklist, budget)?;
            }
        }
    }

    if count == 1 {
        Ok(FibreMultiplicity::ExactlyOne)
    } else {
        Ok(FibreMultiplicity::NotOne { count })
    }
}

/// The Krawczyk system whose single unknown is the fibre parameter `t` and
/// whose residual is `f(t) = h(t) = <approx.subs(t) - x, u>`, with the
/// constant unit normal `u` (the witness tangent) held fixed.
struct FibreSystem<'a, C: EnclosureCurve> {
    /// The approximant curve.
    approx: &'a C,
    /// The witness point on the exact curve.
    x: Point3,
    /// The unit tangent (normal to the disc) at the witness point.
    u: Vector3,
}

impl<'a, C: EnclosureCurve> KrawczykSystem<1> for FibreSystem<'a, C> {
    fn f_point(&self, t: &[f64; 1]) -> [Interval; 1] {
        let [t0] = *t;
        // The point evaluation is a degenerate interval (the Krawczyk contract
        // forbids interval-centre decorrelation).
        [interval_at((self.approx.subs(t0) - self.x).dot(self.u))]
    }

    fn jacobian(&self, b: &[Interval; 1]) -> [[Interval; 1]; 1] {
        let [b0] = *b;
        // h'(t) = <approx'(t), u> — the chain rule against the CONSTANT u, so
        // the Jacobian is the first-derivative enclosure dotted with u.
        [[dot_box(&self.approx.enclose_der(1, b0), &u_box(self.u))]]
    }

    fn preconditioner(&self, t: &[f64; 1]) -> Option<[[f64; 1]; 1]> {
        let [t0] = *t;
        // The float approximate inverse of J at the point: 1/h'(m) with
        // h'(m) read from the derivative enclosure's midpoint (the
        // preconditioner is an approximation by design, and this avoids any
        // dependence on the associated `Vector` type).
        let d = self.approx.enclose_der(1, interval_at(t0));
        let hprime = Vector3::new(d.x.mid(), d.y.mid(), d.z.mid()).dot(self.u);
        if hprime.is_finite() && hprime != 0.0 {
            Some([[1.0 / hprime]])
        } else {
            None
        }
    }
}

/// The interval dot product of two boxes, an enclosure of `{ a · b : a in A,
/// b in B }`. Duplicated locally exactly as `lfs.rs` did; `enclosure.rs`
/// visibility stays untouched.
fn dot_box(a: &Box3, b: &Box3) -> Interval {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// A lower bound on the point-set distance between two boxes: per-axis
/// `max(lo_b - hi_a, lo_a - hi_b)` clamped at 0, Euclidean-combined.
/// Duplicated locally exactly as `lfs.rs` did.
fn box_distance(a: &Box3, b: &Box3) -> f64 {
    let gap = |lo_a: f64, hi_a: f64, lo_b: f64, hi_b: f64| (lo_b - hi_a).max(lo_a - hi_b).max(0.0);
    let dx = gap(a.x.inf(), a.x.sup(), b.x.inf(), b.x.sup());
    let dy = gap(a.y.inf(), a.y.sup(), b.y.inf(), b.y.sup());
    let dz = gap(a.z.inf(), a.z.sup(), b.z.inf(), b.z.sup());
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Shift a box by minus a point: `{ p - q : p in box }` for fixed `q`.
fn box_minus_point(a: &Box3, p: Point3) -> Box3 {
    Box3 {
        x: a.x - interval_at(p.x),
        y: a.y - interval_at(p.y),
        z: a.z - interval_at(p.z),
    }
}

/// The degenerate box at a unit vector.
fn u_box(u: Vector3) -> Box3 {
    Box3 {
        x: interval_at(u.x),
        y: interval_at(u.y),
        z: interval_at(u.z),
    }
}

/// Whether the box contains the zero vector (every coordinate interval
/// contains 0).
fn box3_contains_zero(b: &Box3) -> bool {
    b.x.contains(0.0) && b.y.contains(0.0) && b.z.contains(0.0)
}

/// Whether two axis-aligned boxes overlap (their coordinate intervals
/// intersect on every axis).
fn boxes_overlap(a: &Box3, b: &Box3) -> bool {
    !a.x.intersection(b.x).is_empty()
        && !a.y.intersection(b.y).is_empty()
        && !a.z.intersection(b.z).is_empty()
}

/// Bisect a parameter box at its midpoint and push both halves, spending one
/// subdivision from the budget. `SheetCountUnresolved` when the budget cannot
/// pay for the split.
fn push_children(
    tt: Interval,
    worklist: &mut Vec<Interval>,
    budget: &mut Budget,
) -> Result<(), OneSheetError> {
    budget
        .spend_subdiv(1)
        .map_err(|_| OneSheetError::SheetCountUnresolved)?;
    let mid = 0.5 * tt.inf() + 0.5 * tt.sup();
    let lo = Interval::try_from((tt.inf(), mid)).unwrap_or(Interval::EMPTY);
    let hi = Interval::try_from((mid, tt.sup())).unwrap_or(Interval::EMPTY);
    worklist.push(hi);
    worklist.push(lo);
    Ok(())
}

/// At or below this width a parameter box cannot subdivide further.
/// H-3: 8 ulps of a unit-width parameter interval — a dimensionless width in
/// parameter units, not a model-space length.
const WIDTH_FLOOR: f64 = 8.0 * f64::EPSILON; // H-3: width floor, 8 ulps of a unit-width parameter interval

/// Parameter width below which a certified root's box is narrow enough that
/// the disc-membership decision (`box_distance <= eps`) is a decision about
/// the root itself: the box's image spans at most `R * DISC_DECIDE_WIDTH` in
/// model space, far under the disc radius the witnesses use.
/// H-3: a dimensionless width in parameter units, not a model-space length.
const DISC_DECIDE_WIDTH: f64 = 1.0e-4; // H-3: root-box width for the disc decision, parameter units

#[cfg(test)]
mod tests {
    // GATE-1: the fid module (including its test module) stays under the
    // crate's unwrap denial; unit tests assert on hand-built witnesses.
    #![deny(clippy::unwrap_used)]

    use super::*;
    use crate::elementary::{cos, sin};
    use crate::enclosure::DirCone;
    use std::ops::Bound;
    use truck_base::cgmath64::{EuclideanSpace, Point3, Vector3, Zero};
    use truck_geotrait::{ParameterRange, ParametricCurve};

    /// Exact circle radius, model units.
    const RADIUS: f64 = 2.0; // H-3: exact circle radius in model units, the witness length scale
    /// Closed normal-disc radius at the witness point.
    const DISC_RADIUS: f64 = 0.05; // H-3: disc radius, a model-space length relative to RADIUS
    /// Witness parameter, off every dyadic bisection midpoint and off the
    /// domain endpoints of `[0, 2π]`.
    const WITNESS_T: f64 = 0.7; // H-3: witness parameter in radians, dimensionless (an angle, not a length)
    /// The single-sheet approximant's radius `R + eps`.
    const SINGLE_SHEET_RADIUS: f64 = RADIUS + DISC_RADIUS; // H-3: single-sheet radius, a model-space length
    /// The offset approximant's radius `R + 3*eps`, exceeding the disc radius.
    const OFFSET_RADIUS: f64 = RADIUS + 3.0 * DISC_RADIUS; // H-3: offset-sheet radius, a model-space length
    /// The tangential approximant's touch curvature constant.
    const TOUCH_CURVATURE: f64 = 0.01; // H-3: touch curvature, a model-space length per radian squared
    /// Half of the tangential approximant's parameter range.
    const TANGENT_HALF_SPAN: f64 = 1.0; // H-3: tangential half-parameter-span, dimensionless
    /// The full-circle parameter span `[0, 2π]`.
    const FULL_SPAN: f64 = core::f64::consts::TAU; // H-3: the full circle span in radians, dimensionless
    /// The double-cover parameter span `[0, 4π]`.
    const DOUBLE_SPAN: f64 = 2.0 * core::f64::consts::TAU; // H-3: the double-cover span in radians, dimensionless
    /// Default subdivision budget for the certifying tests.
    const TEST_BUDGET_SUBDIV: u32 = 65536; // H-3: subdivision budget count, dimensionless
    /// Subdivision budget for the tangential (unresolved) test.
    const TANGENT_BUDGET_SUBDIV: u32 = 4096; // H-3: subdivision budget count, dimensionless

    /// A circle `r * e(t)` over `[lo, hi]`, the exact curve of all witnesses
    /// and the base of the single-sheet and offset approximants.
    #[derive(Clone)]
    struct Circle {
        r: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for Circle {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            Point3::new(self.r * t.cos(), self.r * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            Vector3::new(-self.r * t.sin(), self.r * t.cos(), 0.0)
        }

        fn der2(&self, t: f64) -> Vector3 {
            Vector3::new(-self.r * t.cos(), -self.r * t.sin(), 0.0)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            match n % 4 {
                0 => self.subs(t).to_vec(),
                1 => self.der(t),
                2 => self.der2(t),
                _ => Vector3::new(self.r * t.sin(), -self.r * t.cos(), 0.0),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for Circle {
        fn enclose(&self, tt: Interval) -> Box3 {
            Box3 {
                x: cos(tt) * interval_at(self.r),
                y: sin(tt) * interval_at(self.r),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            match n % 4 {
                0 => self.enclose(tt),
                1 => Box3 {
                    x: -sin(tt) * interval_at(self.r),
                    y: cos(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
                2 => Box3 {
                    x: -cos(tt) * interval_at(self.r),
                    y: -sin(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
                _ => Box3 {
                    x: sin(tt) * interval_at(self.r),
                    y: -cos(tt) * interval_at(self.r),
                    z: interval_at(0.0),
                },
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The double-cover approximant `(R + eps*cos(t/2)) * e(t)` over `[0, 4π]`,
    /// the spec's canonical 2-to-1 witness.
    #[derive(Clone)]
    struct DoubleCover {
        r: f64,
        eps: f64,
        lo: f64,
        hi: f64,
    }

    impl DoubleCover {
        fn radius(&self, t: f64) -> f64 {
            self.r + self.eps * (t / 2.0).cos()
        }
    }

    impl ParametricCurve for DoubleCover {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            let rad = self.radius(t);
            Point3::new(rad * t.cos(), rad * t.sin(), 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            let rad = self.radius(t);
            let drad = -0.5 * self.eps * (t / 2.0).sin();
            Vector3::new(
                drad * t.cos() - rad * t.sin(),
                drad * t.sin() + rad * t.cos(),
                0.0,
            )
        }

        fn der2(&self, t: f64) -> Vector3 {
            let rad = self.radius(t);
            let drad = -0.5 * self.eps * (t / 2.0).sin();
            let d2rad = -0.25 * self.eps * (t / 2.0).cos();
            Vector3::new(
                (d2rad - rad) * t.cos() - 2.0 * drad * t.sin(),
                (d2rad - rad) * t.sin() + 2.0 * drad * t.cos(),
                0.0,
            )
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            // Leibniz: subs^(n) = Σ_k C(n,k) * rad^(k) * e^(n-k), with
            // rad^(k) = eps * 2^-k * cos(t/2 + k*pi/2) (k >= 1) and
            // e^(m)(t) = (cos(t + m*pi/2), sin(t + m*pi/2)).
            if n == 0 {
                return self.subs(t).to_vec();
            }
            let mut acc = Vector3::new(0.0, 0.0, 0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = if k == 0 {
                    self.radius(t)
                } else {
                    self.eps
                        * 0.5_f64.powi(k as i32)
                        * (t / 2.0 + (k as f64) * core::f64::consts::FRAC_PI_2).cos()
                };
                let angle = t + (n - k) as f64 * core::f64::consts::FRAC_PI_2;
                acc += Vector3::new(angle.cos(), angle.sin(), 0.0) * (binom * rad_k);
                binom *= (n - k) as f64 / (k + 1) as f64;
            }
            acc
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for DoubleCover {
        fn enclose(&self, tt: Interval) -> Box3 {
            let rad = interval_at(self.r) + interval_at(self.eps) * cos(tt / interval_at(2.0));
            Box3 {
                x: rad * cos(tt),
                y: rad * sin(tt),
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let half = interval_at(2.0);
            let mut x = interval_at(0.0);
            let mut y = interval_at(0.0);
            let mut binom = 1.0_f64;
            for k in 0..=n {
                let rad_k = if k == 0 {
                    interval_at(self.r) + interval_at(self.eps) * cos(tt / half)
                } else {
                    interval_at(self.eps)
                        * interval_at(0.5_f64.powi(k as i32))
                        * cos(tt / half + interval_at((k as f64) * core::f64::consts::FRAC_PI_2))
                };
                let shift = (n - k) as f64 * core::f64::consts::FRAC_PI_2;
                let ex = cos(tt + interval_at(shift));
                let ey = sin(tt + interval_at(shift));
                let c = interval_at(binom);
                x += ex * rad_k * c;
                y += ey * rad_k * c;
                binom *= (n - k) as f64 / (k + 1) as f64;
            }
            Box3 {
                x,
                y,
                z: interval_at(0.0),
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The tangential-contact approximant `x + u * (-c*(t - t*)^2)`: a
    /// parabola along the disc normal through the witness point. Its signed
    /// plane coordinate is `-c*(t - t*)^2`, a double-touch extremum at `t*`
    /// inside the closed ball. The constant `c` is chosen so the curve stays
    /// within `eps` of the plane (and of the witness point) over its whole
    /// parameter span: `c * TANGENT_HALF_SPAN^2 = TOUCH_CURVATURE` lies
    /// strictly below `DISC_RADIUS`.
    #[derive(Clone)]
    struct Tangential {
        x: Point3,
        u: Vector3,
        c: f64,
        t_star: f64,
        lo: f64,
        hi: f64,
    }

    impl ParametricCurve for Tangential {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            let h0 = -self.c * (t - self.t_star) * (t - self.t_star);
            Point3::new(
                self.x.x + self.u.x * h0,
                self.x.y + self.u.y * h0,
                self.x.z + self.u.z * h0,
            )
        }

        fn der(&self, t: f64) -> Vector3 {
            self.u * (-2.0 * self.c * (t - self.t_star))
        }

        fn der2(&self, _t: f64) -> Vector3 {
            self.u * (-2.0 * self.c)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            match n {
                0 => self.subs(t).to_vec(),
                1 => self.der(t),
                2 => self.der2(t),
                _ => Vector3::zero(),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.lo), Bound::Included(self.hi))
        }
    }

    impl EnclosureCurve for Tangential {
        fn enclose(&self, tt: Interval) -> Box3 {
            let s = tt - interval_at(self.t_star);
            let h0 = -interval_at(self.c) * s.sqr();
            Box3 {
                x: interval_at(self.x.x) + interval_at(self.u.x) * h0,
                y: interval_at(self.x.y) + interval_at(self.u.y) * h0,
                z: interval_at(self.x.z) + interval_at(self.u.z) * h0,
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            if n == 0 {
                return self.enclose(tt);
            }
            let d = match n {
                1 => interval_at(-2.0 * self.c) * (tt - interval_at(self.t_star)),
                _ => interval_at(-2.0 * self.c),
            };
            Box3 {
                x: interval_at(self.u.x) * d,
                y: interval_at(self.u.y) * d,
                z: interval_at(self.u.z) * d,
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// A cusp curve `(t^2, t^3, 0)` over `[-1, 1]`: its tangent vanishes at
    /// `t = 0`, the pole-straddling witness.
    #[derive(Clone)]
    struct Cusp;

    impl ParametricCurve for Cusp {
        type Point = Point3;
        type Vector = Vector3;

        fn subs(&self, t: f64) -> Point3 {
            Point3::new(t * t, t * t * t, 0.0)
        }

        fn der(&self, t: f64) -> Vector3 {
            Vector3::new(2.0 * t, 3.0 * t * t, 0.0)
        }

        fn der2(&self, t: f64) -> Vector3 {
            Vector3::new(2.0, 6.0 * t, 0.0)
        }

        fn der_n(&self, n: usize, t: f64) -> Vector3 {
            match n {
                0 => self.subs(t).to_vec(),
                1 => self.der(t),
                2 => self.der2(t),
                3 => Vector3::new(0.0, 6.0, 0.0),
                _ => Vector3::zero(),
            }
        }

        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(-1.0), Bound::Included(1.0))
        }
    }

    impl EnclosureCurve for Cusp {
        fn enclose(&self, tt: Interval) -> Box3 {
            let t2 = tt.sqr();
            Box3 {
                x: t2,
                y: t2 * tt,
                z: interval_at(0.0),
            }
        }

        fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
            match n {
                0 => self.enclose(tt),
                1 => Box3 {
                    x: interval_at(2.0) * tt,
                    y: interval_at(3.0) * tt.sqr(),
                    z: interval_at(0.0),
                },
                2 => Box3 {
                    x: interval_at(2.0),
                    y: interval_at(6.0) * tt,
                    z: interval_at(0.0),
                },
                3 => Box3 {
                    x: interval_at(0.0),
                    y: interval_at(6.0),
                    z: interval_at(0.0),
                },
                _ => Box3 {
                    x: interval_at(0.0),
                    y: interval_at(0.0),
                    z: interval_at(0.0),
                },
            }
        }

        fn tangent_cone(&self, _tt: Interval) -> Option<DirCone> {
            None
        }
    }

    /// The exact circle for every witness: radius RADIUS over `[0, 2π]`.
    fn exact_circle() -> Circle {
        Circle {
            r: RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        }
    }

    /// The witness normal pair `(u, w)`: the unit tangent at the witness
    /// point and a unit vector perpendicular to it (an in-plane direction).
    fn normal_pair() -> (Vector3, Vector3) {
        let u = Vector3::new(-WITNESS_T.sin(), WITNESS_T.cos(), 0.0);
        let w = Vector3::new(WITNESS_T.cos(), WITNESS_T.sin(), 0.0);
        (u, w)
    }

    /// The witness point `x = exact.subs(WITNESS_T)`.
    fn witness_point() -> Point3 {
        Point3::new(RADIUS * WITNESS_T.cos(), RADIUS * WITNESS_T.sin(), 0.0)
    }

    /// Test-only unwrap that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a refusal here is a test bug.
    fn must(r: Result<FibreMultiplicity, OneSheetError>) -> FibreMultiplicity {
        match r {
            Ok(value) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    #[test]
    fn single_sheet_circle_certifies_degree_one() {
        // X' = (R + eps) * e(t) over [0, 2π]: the plane crossings at
        // WITNESS_T (in-disc, distance exactly eps) and WITNESS_T + π
        // (~2R + eps out, excluded by the disc test) leave exactly one
        // in-disc point, so the fibre is degree one.
        let exact = exact_circle();
        let approx = Circle {
            r: SINGLE_SHEET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = must(fibre_degree_one(
            &exact,
            &approx,
            WITNESS_T,
            DISC_RADIUS,
            &mut budget,
        ));
        assert_eq!(out, FibreMultiplicity::ExactlyOne);
    }

    #[test]
    fn double_cover_witness_refuses() {
        // The canonical 2-to-1 witness: (R + eps*cos(t/2)) * e(t) over
        // [0, 4π]. The crossings near WITNESS_T and WITNESS_T + 2π are
        // genuinely distinct in-disc points ((R ± eps*cos(t/2)) * e(t)), while
        // the crossings at WITNESS_T + π and WITNESS_T + 3π sit ~2R outside
        // the ball and must be excluded by the disc test. The count must be
        // exactly 2: less fails an under-counting bug, more an over-counting
        // one.
        let exact = exact_circle();
        let approx = DoubleCover {
            r: RADIUS,
            eps: DISC_RADIUS,
            lo: 0.0,
            hi: DOUBLE_SPAN,
        };
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = must(fibre_degree_one(
            &exact,
            &approx,
            WITNESS_T,
            DISC_RADIUS,
            &mut budget,
        ));
        assert_eq!(out, FibreMultiplicity::NotOne { count: 2 });
    }

    #[test]
    fn offset_sheet_outside_disc_ignored() {
        // The approximant offset radially by 3*eps (> the disc radius): no
        // in-disc intersection exists, so the fibre misses entirely.
        let exact = exact_circle();
        let approx = Circle {
            r: OFFSET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = must(fibre_degree_one(
            &exact,
            &approx,
            WITNESS_T,
            DISC_RADIUS,
            &mut budget,
        ));
        assert_eq!(out, FibreMultiplicity::NotOne { count: 0 });
    }

    #[test]
    fn tangential_contact_is_unresolved_not_degree_one() {
        // The signed plane coordinate h(t) = -c*(t - t*)^2 is a double-touch
        // extremum inside the ball: an even-multiplicity zero that Krawczyk's
        // strict-interior rule never certifies. Reporting degree one here would
        // be the classic false pass.
        let (u, _w) = normal_pair();
        let x = witness_point();
        let approx = Tangential {
            x,
            u,
            c: TOUCH_CURVATURE,
            t_star: WITNESS_T,
            lo: WITNESS_T - TANGENT_HALF_SPAN,
            hi: WITNESS_T + TANGENT_HALF_SPAN,
        };
        let mut budget = Budget::new(TANGENT_BUDGET_SUBDIV, 0, 0);
        let out = fibre_degree_one(
            &exact_circle(),
            &approx,
            WITNESS_T,
            DISC_RADIUS,
            &mut budget,
        );
        assert!(
            matches!(out, Err(OneSheetError::SheetCountUnresolved)),
            "a tangential contact must refuse as SheetCountUnresolved, got {out:?}"
        );
    }

    #[test]
    fn zero_budget_refuses_unresolved() {
        // An empty budget cannot pay for the subdivision that isolating even a
        // single root requires.
        let exact = exact_circle();
        let approx = Circle {
            r: SINGLE_SHEET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        let mut budget = Budget::new(0, 0, 0);
        let out = fibre_degree_one(&exact, &approx, WITNESS_T, DISC_RADIUS, &mut budget);
        assert!(
            matches!(out, Err(OneSheetError::SheetCountUnresolved)),
            "a zero budget must refuse as SheetCountUnresolved, got {out:?}"
        );
    }

    #[test]
    fn invalid_witness_refuses() {
        let exact = exact_circle();
        let approx = Circle {
            r: SINGLE_SHEET_RADIUS,
            lo: 0.0,
            hi: FULL_SPAN,
        };
        // eps <= 0.
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = fibre_degree_one(&exact, &approx, WITNESS_T, 0.0, &mut budget);
        assert_eq!(out, Err(OneSheetError::InvalidWitness));
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = fibre_degree_one(&exact, &approx, WITNESS_T, -DISC_RADIUS, &mut budget);
        assert_eq!(out, Err(OneSheetError::InvalidWitness));
        // A pole-straddling witness parameter: the cusp's tangent vanishes at
        // t = 0, so the tangent enclosure contains zero.
        let mut budget = Budget::new(TEST_BUDGET_SUBDIV, 0, 0);
        let out = fibre_degree_one(&Cusp, &Cusp, 0.0, DISC_RADIUS, &mut budget);
        assert_eq!(out, Err(OneSheetError::InvalidWitness));
    }
}
