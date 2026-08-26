//! BG-SOL-S4-FE-EE â€” the Contact Layer strata reductions for the FE
//! (Edge Ã— Face) and EE (Edge Ã— Edge) funnel stages (plan Â§4 Phase 3).
//!
//! `fe_contact` answers "where does an edge meet a face, and how", and
//! `ee_contact` answers "where do two edges meet, and how", both certified and
//! bounded to both strata. The bounded locus forms are `ContactLocus::Point`
//! (an isolated contact point) and `ContactLocus::BoundedCurve` (an exact
//! curve clipped to a parameter range in the curve's own parameterization).
//! The Boundary Rewrite (Phase 4) splits edges and faces against exactly these
//! forms, so every reported point or arc lies within BOTH strata's bounds â€”
//! the edge's `t_range` and the face's `(u, v)` box.
//!
//! The FE analytic table landed here:
//!
//! | edge | face | implementation |
//! |---|---|---|
//! | Line | Plane | Â§5.1 linear solve |
//! | Line | Cylinder | Â§5.2 quadratic solve + generator coincident |
//! | Circle | Plane | Â§5.3 chord solve + coincident arc clip |
//! | Circle | Cylinder | Â§5.4 latitudinal coincident only |
//!
//! The EE analytic table:
//!
//! | lhs | rhs | implementation |
//! |---|---|---|
//! | Line | Line | Â§6.1 |
//! | Line | Circle | Â§6.2 (order-insensitive) |
//!
//! Everything else in scope â€” `Line`Ã—`Cone`/`Sphere`, `Circle`Ã—`Cone`/
//! `Sphere`, `Circle`Ã—`Cylinder` transverse, `Circle`Ã—`Circle` â€” returns the
//! deferred funnel refusal
//! `Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)`.
//!
//! House rules H-1..H-8 apply. Every classification predicate is decided by
//! decisive interval enclosures of quantities derived from the f64 carrier
//! parameters (BG-ANA-002); a straddling enclosure is
//! `Refusal::NumericallyUnresolved`, never a guess. Nothing is spent from the
//! caller's `budget` â€” no subdivision happens anywhere in this packet â€” and
//! every certificate is the explicit field-by-field exact certificate.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::cmp::Ordering;
use std::f64::consts::TAU;

use inari::Interval;
use truck_base::cgmath64::{Homogeneous, InnerSpace, Point3, Vector3};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, Prop, PropMap,
    Refusal, Truth, UnresolvedWitness,
};
use truck_geometry::recognize::{CanonicalCurve, CanonicalSurface};
use truck_geometry::specifieds::{Cylinder, Line, Plane};
use truck_geotrait::ParametricCurve;

use super::{ContactComplex, ContactLocus, ContactRecord};
use crate::analytic::plane_plane::plane_plane;
use crate::analytic::{AnalyticIntersection, ExactCurve, PlacedCircle};

/// The typed refusal of the deferred funnel.
fn deferred() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

/// A numerically undecidable predicate: a stop, never a guess (BG-ANA-002).
fn unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::RootNotIsolated,
    }
}

/// The certificate every decided outcome carries: analytic carrier, exact
/// method, the untouched budget, unbounded margin and modulus (explicit
/// field-by-field, BG-EVD-002).
fn exact_certificate(budget: &Budget) -> Certificate {
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Certificate {
        props,
        method: Method::Exact,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// A certified `ContactComplex` with the exact certificate.
fn certified(contacts: Vec<ContactRecord>, budget: &Budget) -> Outcome<ContactComplex> {
    Ok(Certified::new(
        ContactComplex { contacts },
        exact_certificate(budget),
    ))
}

/// A certified "no contact" â€” a decided empty pair is a certified answer.
fn empty(budget: &Budget) -> Outcome<ContactComplex> {
    certified(Vec::new(), budget)
}

/// An isolated contact point record.
fn point_record(q: Point3, kind: ContactEventKind) -> ContactRecord {
    ContactRecord {
        dimension: ContactDimension::Point0,
        kind,
        locus: ContactLocus::Point(q),
    }
}

/// A coincident sub-arc record.
fn arc_record(curve: ExactCurve, t_range: (f64, f64)) -> ContactRecord {
    ContactRecord {
        dimension: ContactDimension::Arc1,
        kind: ContactEventKind::CoincidentInterval,
        locus: ContactLocus::BoundedCurve { curve, t_range },
    }
}

// ---------------------------------------------------------------------------
// Decisive interval predicates (copied from analytic/plane_plane.rs, verbatim
// per BG-ANA-002).
// ---------------------------------------------------------------------------

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// Whether the interval proves zero: it must be the degenerate `[0, 0]`.
///
/// An inari enclosure of a quantity that is exactly zero only through
/// cancellation is a wide-ish `[-ulp, +ulp]`; claiming that proves zero is
/// exactly the wrong-but-confident answer BG-ANA-002 forbids. Dyadic-clean
/// inputs produce degenerate intervals, so exact classifications stay exact.
fn decisively_zero(i: Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// Whether the interval lies strictly away from zero.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// The dot product `a Â· b` in interval arithmetic.
fn interval_dot(a: Vector3, b: Vector3) -> Interval {
    interval_at(a.x) * interval_at(b.x)
        + interval_at(a.y) * interval_at(b.y)
        + interval_at(a.z) * interval_at(b.z)
}

/// The cross product `a Ã— b`, computed per component in interval arithmetic.
fn interval_cross(a: Vector3, b: Vector3) -> [Interval; 3] {
    [
        interval_at(a.y) * interval_at(b.z) - interval_at(a.z) * interval_at(b.y),
        interval_at(a.z) * interval_at(b.x) - interval_at(a.x) * interval_at(b.z),
        interval_at(a.x) * interval_at(b.y) - interval_at(a.y) * interval_at(b.x),
    ]
}

/// A three-way comparison of two interval enclosures, decided only when the
/// ordering is unambiguous; `None` â€” undecidable â€” is a stop, not a guess.
fn three_way(a: Interval, b: Interval) -> Option<Ordering> {
    if excludes_zero(a - b) {
        if (a - b).inf() > 0.0 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    } else if decisively_zero(a - b) {
        Some(Ordering::Equal)
    } else {
        None
    }
}

/// The sign of a quadratic discriminant interval.
enum Discriminant {
    Negative,
    Zero,
    Positive,
}

/// Decides the discriminant's sign; `None` when the enclosure straddles zero.
fn classify_discriminant(d: Interval) -> Option<Discriminant> {
    if d.sup() < 0.0 {
        Some(Discriminant::Negative)
    } else if decisively_zero(d) {
        Some(Discriminant::Zero)
    } else if d.inf() > 0.0 {
        Some(Discriminant::Positive)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Bounded-range helpers.
// ---------------------------------------------------------------------------

/// The position of a scalar relative to a closed interval. Computed f64 values
/// are degenerate enclosures, so the classification is total and exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RangePosition {
    Below,
    Inside,
    AtBoundary,
    Above,
}

fn locate(value: f64, (lo, hi): (f64, f64)) -> RangePosition {
    let v = interval_at(value);
    if v.sup() < lo {
        RangePosition::Below
    } else if v.inf() > hi {
        RangePosition::Above
    } else if v.sup() == lo || v.inf() == hi {
        RangePosition::AtBoundary
    } else {
        RangePosition::Inside
    }
}

/// Intersect two closed intervals, inclusive endpoints. `None` when empty.
fn clip_interval_to_range((lo, hi): (f64, f64), (r0, r1): (f64, f64)) -> Option<(f64, f64)> {
    let lo = lo.max(r0);
    let hi = hi.min(r1);
    if hi < lo {
        None
    } else {
        Some((lo, hi))
    }
}

/// Clip `[t_lo, t_hi]` against the affine constraint `alpha + betaÂ·t âˆˆ [r0, r1]`.
/// `beta` is a computed f64, so its sign is exact. `None` when the constraint
/// proves empty; inclusive endpoints.
fn clip_affine(
    t_lo: f64,
    t_hi: f64,
    alpha: f64,
    beta: f64,
    (r0, r1): (f64, f64),
) -> Option<(f64, f64)> {
    let mut lo = t_lo;
    let mut hi = t_hi;
    if beta == 0.0 {
        if alpha < r0 || alpha > r1 {
            return None;
        }
    } else {
        let t_lower = (r0 - alpha) / beta;
        let t_upper = (r1 - alpha) / beta;
        lo = lo.max(t_lower.min(t_upper));
        hi = hi.min(t_lower.max(t_upper));
    }
    if hi < lo {
        None
    } else {
        Some((lo, hi))
    }
}

/// Where a point sits in a plane face's parameter box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceLocation {
    Inside,
    Boundary,
    Outside,
}

fn plane_box_location(
    plane: &Plane,
    q: Point3,
    u_range: (f64, f64),
    v_range: (f64, f64),
) -> FaceLocation {
    let prm = plane.get_parameter(q);
    match (locate(prm.x, u_range), locate(prm.y, v_range)) {
        (RangePosition::Below | RangePosition::Above, _)
        | (_, RangePosition::Below | RangePosition::Above) => FaceLocation::Outside,
        (RangePosition::AtBoundary, _) | (_, RangePosition::AtBoundary) => FaceLocation::Boundary,
        _ => FaceLocation::Inside,
    }
}

/// The event kind of a contact point: `EndpointTouch` at a stratum boundary,
/// `Transverse` strictly inside both strata (Â§4).
fn point_kind(t: f64, edge_t_range: (f64, f64), face: FaceLocation) -> ContactEventKind {
    let edge_boundary = matches!(locate(t, edge_t_range), RangePosition::AtBoundary);
    if edge_boundary || face == FaceLocation::Boundary {
        ContactEventKind::EndpointTouch
    } else {
        ContactEventKind::Transverse
    }
}

// ---------------------------------------------------------------------------
// Circle frame.
// ---------------------------------------------------------------------------

/// The circle's frame from its placement matrix: `(center, in-plane x axis,
/// in-plane y axis, radius)`. The transform columns are the in-plane axes
/// `x`, `y` (both of length `r` for a circle), the plane normal `z` and the
/// center `w` (Â§5.3).
fn circle_frame(circle: &PlacedCircle) -> (Point3, Vector3, Vector3, f64) {
    let m = circle.transform();
    let x = Vector3::new(m.x.x, m.x.y, m.x.z);
    let y = Vector3::new(m.y.x, m.y.y, m.y.z);
    (m.w.to_point(), x, y, x.magnitude())
}

/// The circle parameter angle of `q`, wrapped into `[0, TAU)`.
fn angle_of(q: Point3, center: Point3, u_hat: Vector3, v_hat: Vector3) -> f64 {
    let w = q - center;
    let mut theta = w.dot(v_hat).atan2(w.dot(u_hat));
    if theta < 0.0 {
        theta += TAU;
    }
    theta
}

// ---------------------------------------------------------------------------
// FE dispatcher.
// ---------------------------------------------------------------------------

/// Answers "where do an edge and a face meet, and how", certified and bounded
/// to both strata. Arguments are normalized so the solver always sees
/// `(edge, face)`; [`super::contact`] swaps the `(Face, Edge)` order into this
/// same call, so the two orders produce structurally equal results.
pub fn fe_contact(
    edge_curve: &CanonicalCurve,
    edge_t_range: &(f64, f64),
    face_surface: &CanonicalSurface,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    match (edge_curve, face_surface) {
        (CanonicalCurve::Line(line), CanonicalSurface::Plane(plane)) => line_plane(
            line,
            edge_t_range,
            plane,
            face_u_range,
            face_v_range,
            budget,
        ),
        (CanonicalCurve::Line(line), CanonicalSurface::Cylinder(cylinder)) => line_cylinder(
            line,
            edge_t_range,
            cylinder,
            face_u_range,
            face_v_range,
            budget,
        ),
        (CanonicalCurve::Circle(circle), CanonicalSurface::Plane(plane)) => circle_plane(
            circle,
            edge_t_range,
            plane,
            face_u_range,
            face_v_range,
            budget,
        ),
        (CanonicalCurve::Circle(circle), CanonicalSurface::Cylinder(cylinder)) => circle_cylinder(
            circle,
            edge_t_range,
            cylinder,
            face_u_range,
            face_v_range,
            budget,
        ),
        _ => Err(deferred()),
    }
}

// ---------------------------------------------------------------------------
// Â§5.1 Line Ã— Plane.
// ---------------------------------------------------------------------------

fn line_plane(
    line: &Line<Point3>,
    edge_t_range: &(f64, f64),
    plane: &Plane,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let Line(a, b) = *line;
    let d = b - a;
    let n = plane.normal();
    let o = plane.origin();
    let denom_iv = interval_dot(d, n);
    let denom = d.dot(n);
    let num = (o - a).dot(n);
    if excludes_zero(denom_iv) {
        // The line meets the plane exactly once at t0 = num / denom.
        let t0 = num / denom;
        match locate(t0, *edge_t_range) {
            RangePosition::Below | RangePosition::Above => empty(budget),
            _ => {
                let q = a + t0 * d;
                let location = plane_box_location(plane, q, *face_u_range, *face_v_range);
                match location {
                    FaceLocation::Outside => empty(budget),
                    _ => certified(
                        vec![point_record(q, point_kind(t0, *edge_t_range, location))],
                        budget,
                    ),
                }
            }
        }
    } else if decisively_zero(denom_iv) {
        // The line is parallel to the plane; the offset decides.
        let num_iv = interval_dot(o - a, n);
        if decisively_zero(num_iv) {
            line_in_plane_clip(
                *line,
                *edge_t_range,
                plane,
                *face_u_range,
                *face_v_range,
                budget,
            )
        } else if excludes_zero(num_iv) {
            empty(budget)
        } else {
            Err(unresolved())
        }
    } else {
        Err(unresolved())
    }
}

/// The coincident Line-in-Plane clip: the maximal sub-interval of the edge
/// whose image lies in the face's `(u, v)` box, intersected with the edge's
/// own `t_range`.
fn line_in_plane_clip(
    line: Line<Point3>,
    edge_t_range: (f64, f64),
    plane: &Plane,
    face_u_range: (f64, f64),
    face_v_range: (f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let Line(a, b) = line;
    let pa = plane.get_parameter(a);
    let pb = plane.get_parameter(b);
    let clip = clip_affine(
        edge_t_range.0,
        edge_t_range.1,
        pa.x,
        pb.x - pa.x,
        face_u_range,
    )
    .and_then(|(lo, hi)| clip_affine(lo, hi, pa.y, pb.y - pa.y, face_v_range));
    match clip {
        Some((t_lo, t_hi)) => certified(
            vec![arc_record(ExactCurve::Line(line), (t_lo, t_hi))],
            budget,
        ),
        None => empty(budget),
    }
}

// ---------------------------------------------------------------------------
// Â§5.2 Line Ã— Cylinder.
// ---------------------------------------------------------------------------

fn line_cylinder(
    line: &Line<Point3>,
    edge_t_range: &(f64, f64),
    cylinder: &Cylinder,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let Line(a, b) = *line;
    let d = b - a;
    let c = cylinder.center();
    let dx_iv = interval_at(d.x);
    let dy_iv = interval_at(d.y);
    if decisively_zero(dx_iv) && decisively_zero(dy_iv) {
        // The line is parallel to the axis: the radial distance decides.
        let ax = a.x - c.x;
        let ay = a.y - c.y;
        let rho_iv = interval_at(ax) * interval_at(ax) + interval_at(ay) * interval_at(ay);
        let r_sq_iv = interval_at(cylinder.radius()) * interval_at(cylinder.radius());
        match three_way(rho_iv, r_sq_iv) {
            Some(Ordering::Equal) => {
                // A generator: coincident with the wall over the segment. Its
                // angle must lie in the face's u_range (Â§4) before the v clip.
                let mut u = ay.atan2(ax);
                if u < 0.0 {
                    u += TAU;
                }
                match locate(u, *face_u_range) {
                    RangePosition::Below | RangePosition::Above => return empty(budget),
                    RangePosition::Inside | RangePosition::AtBoundary => {}
                }
                let clip = clip_affine(
                    edge_t_range.0,
                    edge_t_range.1,
                    a.z - c.z,
                    d.z,
                    *face_v_range,
                );
                match clip {
                    Some((t_lo, t_hi)) => certified(
                        vec![arc_record(ExactCurve::Line(*line), (t_lo, t_hi))],
                        budget,
                    ),
                    None => empty(budget),
                }
            }
            Some(Ordering::Less) | Some(Ordering::Greater) => empty(budget),
            None => Err(unresolved()),
        }
    } else if excludes_zero(dx_iv) || excludes_zero(dy_iv) {
        line_cylinder_quadratic(
            a,
            d,
            cylinder,
            edge_t_range,
            face_u_range,
            face_v_range,
            budget,
        )
    } else {
        Err(unresolved())
    }
}

/// The lineÃ—cylinder quadratic: `(ax + tÂ·dx âˆ’ cx)Â² + (ay + tÂ·dy âˆ’ cy)Â² = rÂ²`,
/// with a decisive discriminant and every root bounded to both strata.
fn line_cylinder_quadratic(
    a: Point3,
    d: Vector3,
    cylinder: &Cylinder,
    edge_t_range: &(f64, f64),
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let c = cylinder.center();
    let r = cylinder.radius();
    let (ax, ay) = (a.x - c.x, a.y - c.y);
    let a_q_iv = interval_at(d.x) * interval_at(d.x) + interval_at(d.y) * interval_at(d.y);
    let b_q_iv = interval_at(2.0)
        * (interval_at(ax) * interval_at(d.x) + interval_at(ay) * interval_at(d.y));
    let c_q_iv = interval_at(ax) * interval_at(ax) + interval_at(ay) * interval_at(ay)
        - interval_at(r) * interval_at(r);
    let disc_iv = b_q_iv * b_q_iv - interval_at(4.0) * a_q_iv * c_q_iv;

    let a_q = d.x * d.x + d.y * d.y;
    let b_q = 2.0 * (ax * d.x + ay * d.y);
    let c_q = ax * ax + ay * ay - r * r;
    let disc = b_q * b_q - 4.0 * a_q * c_q;

    let mut records = Vec::new();
    match classify_discriminant(disc_iv) {
        Some(Discriminant::Negative) => return empty(budget),
        Some(Discriminant::Zero) => {
            let t0 = -b_q / (2.0 * a_q);
            if let Some(record) = cylinder_point_candidate(
                t0,
                a,
                d,
                cylinder,
                *edge_t_range,
                *face_u_range,
                *face_v_range,
                true,
            ) {
                records.push(record);
            }
        }
        Some(Discriminant::Positive) => {
            let s = disc.sqrt();
            for t in [(-b_q + s) / (2.0 * a_q), (-b_q - s) / (2.0 * a_q)] {
                if let Some(record) = cylinder_point_candidate(
                    t,
                    a,
                    d,
                    cylinder,
                    *edge_t_range,
                    *face_u_range,
                    *face_v_range,
                    false,
                ) {
                    records.push(record);
                }
            }
        }
        None => return Err(unresolved()),
    }
    certified(records, budget)
}

/// The candidate contact record for one lineÃ—cylinder root: the point must lie
/// in the edge's `t_range` and the face's `(u, v)` box, where the cylinder's
/// `u` is `atan2` wrapped into `[0, 2Ï€)`. A tangent root is `Tangency`; a
/// transverse root at a stratum boundary is `EndpointTouch`.
#[allow(clippy::too_many_arguments)]
fn cylinder_point_candidate(
    t: f64,
    a: Point3,
    d: Vector3,
    cylinder: &Cylinder,
    edge_t_range: (f64, f64),
    face_u_range: (f64, f64),
    face_v_range: (f64, f64),
    tangent: bool,
) -> Option<ContactRecord> {
    let edge_position = locate(t, edge_t_range);
    if matches!(edge_position, RangePosition::Below | RangePosition::Above) {
        return None;
    }
    let c = cylinder.center();
    let q = a + t * d;
    let mut u = (q.y - c.y).atan2(q.x - c.x);
    if u < 0.0 {
        u += TAU;
    }
    let v = q.z - c.z;
    let on_face_boundary = match (locate(u, face_u_range), locate(v, face_v_range)) {
        (RangePosition::Below | RangePosition::Above, _)
        | (_, RangePosition::Below | RangePosition::Above) => return None,
        (RangePosition::AtBoundary, _) | (_, RangePosition::AtBoundary) => true,
        _ => false,
    };
    let kind = if tangent {
        ContactEventKind::Tangency
    } else if on_face_boundary || edge_position == RangePosition::AtBoundary {
        ContactEventKind::EndpointTouch
    } else {
        ContactEventKind::Transverse
    };
    Some(point_record(q, kind))
}

// ---------------------------------------------------------------------------
// Â§5.3 Circle Ã— Plane.
// ---------------------------------------------------------------------------

fn circle_plane(
    circle: &PlacedCircle,
    edge_t_range: &(f64, f64),
    plane: &Plane,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let (center, x, y, _) = circle_frame(circle);
    let circle_plane = Plane::new(center, center + x, center + y);
    let out = plane_plane(&circle_plane, plane)?;
    match out.value {
        AnalyticIntersection::Coincident => circle_plane_coincident(
            circle,
            edge_t_range,
            plane,
            face_u_range,
            face_v_range,
            budget,
        ),
        AnalyticIntersection::Curve(ExactCurve::Line(line)) => circle_plane_chord(
            circle,
            edge_t_range,
            &line,
            plane,
            face_u_range,
            face_v_range,
            budget,
        ),
        AnalyticIntersection::Parallel | AnalyticIntersection::Empty => empty(budget),
        // `plane_plane` emits only the arms above (or a NumericallyUnresolved
        // refusal, already propagated); any other arm is outside this family.
        _ => Err(deferred()),
    }
}

/// The coincident Circle-in-Plane clip: the face box's four boundary lines cut
/// the circle in up to eight crossing angles; each maximal contained angular
/// interval, intersected with the edge's `t_range`, becomes a `BoundedCurve`.
fn circle_plane_coincident(
    circle: &PlacedCircle,
    edge_t_range: &(f64, f64),
    plane: &Plane,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let contained = circle_box_intervals(circle, plane, *face_u_range, *face_v_range)?;
    let mut records = Vec::new();
    for (lo, hi) in contained {
        if let Some((t_lo, t_hi)) = clip_interval_to_range((lo, hi), *edge_t_range) {
            records.push(arc_record(ExactCurve::Circle(*circle), (t_lo, t_hi)));
        }
    }
    certified(records, budget)
}

/// The maximal angular intervals on `[0, TAU)` of the circle contained in the
/// plane face's parameter box. A whole circle inside the box is the single
/// interval `[0, TAU)`. `Err` when a box-line/circle crossing is numerically
/// undecidable (a stop).
fn circle_box_intervals(
    circle: &PlacedCircle,
    plane: &Plane,
    face_u_range: (f64, f64),
    face_v_range: (f64, f64),
) -> Result<Vec<(f64, f64)>, Refusal> {
    let (center, x, y, radius) = circle_frame(circle);
    let u_hat = x / radius;
    let v_hat = y / radius;
    let o = plane.origin();
    let e0 = plane.u_axis();
    let e1 = plane.v_axis();
    let lines = [
        (o + face_u_range.0 * e0, e1),
        (o + face_u_range.1 * e0, e1),
        (o + face_v_range.0 * e1, e0),
        (o + face_v_range.1 * e1, e0),
    ];
    let mut angles = Vec::new();
    for (p0, dl) in lines {
        let crossings = line_circle_angles(p0, dl, center, radius, u_hat, v_hat)?;
        angles.extend(crossings);
    }
    if angles.is_empty() {
        // No boundary line cuts the circle: either the whole circle is in the
        // box or none of it is. Probe one point on it.
        let probe = circle.subs(0.0);
        return Ok(
            match plane_box_location(plane, probe, face_u_range, face_v_range) {
                FaceLocation::Outside => Vec::new(),
                _ => vec![(0.0, TAU)],
            },
        );
    }
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    angles.dedup();
    let mut intervals = Vec::new();
    for window in angles.windows(2) {
        let [lo, hi] = window else {
            unreachable!("windows(2) yields exactly two elements");
        };
        let probe = circle.subs((*lo + *hi) / 2.0);
        match plane_box_location(plane, probe, face_u_range, face_v_range) {
            FaceLocation::Outside => {}
            _ => intervals.push((*lo, *hi)),
        }
    }
    // The wrap-around interval from the last angle back to the first crosses
    // the parameter seam at TAU.
    if let (Some(last), Some(first)) = (angles.last(), angles.first()) {
        let probe = circle.subs((*last + *first + TAU) / 2.0);
        match plane_box_location(plane, probe, face_u_range, face_v_range) {
            FaceLocation::Outside => {}
            _ => intervals.push((*last, *first + TAU)),
        }
    }
    Ok(intervals)
}

/// The angles in `[0, TAU)` where the line `p0 + tÂ·dl` meets the circle.
fn line_circle_angles(
    p0: Point3,
    dl: Vector3,
    center: Point3,
    radius: f64,
    u_hat: Vector3,
    v_hat: Vector3,
) -> Result<Vec<f64>, Refusal> {
    let w = p0 - center;
    let a_iv = interval_dot(dl, dl);
    let b_iv = interval_at(2.0) * interval_dot(w, dl);
    let c_iv = interval_dot(w, w) - interval_at(radius) * interval_at(radius);
    let disc_iv = b_iv * b_iv - interval_at(4.0) * a_iv * c_iv;
    let a = dl.dot(dl);
    let b = 2.0 * w.dot(dl);
    let c = w.dot(w) - radius * radius;
    let disc = b * b - 4.0 * a * c;
    match classify_discriminant(disc_iv) {
        Some(Discriminant::Negative) => Ok(Vec::new()),
        Some(Discriminant::Zero) => {
            let t0 = -b / (2.0 * a);
            Ok(vec![angle_of(p0 + t0 * dl, center, u_hat, v_hat)])
        }
        Some(Discriminant::Positive) => {
            let s = disc.sqrt();
            Ok(vec![
                angle_of(p0 + (-b + s) / (2.0 * a) * dl, center, u_hat, v_hat),
                angle_of(p0 + (-b - s) / (2.0 * a) * dl, center, u_hat, v_hat),
            ])
        }
        None => Err(unresolved()),
    }
}

/// The Circle Ã— Plane transverse chord: the circle meets the two planes'
/// intersection line in 0/1/2 points, each bounded to the face box and the
/// circle's edge (a circle edge is the whole `[0, TAU)`).
fn circle_plane_chord(
    circle: &PlacedCircle,
    edge_t_range: &(f64, f64),
    line: &Line<Point3>,
    plane: &Plane,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let (center, x, y, radius) = circle_frame(circle);
    let u_hat = x / radius;
    let v_hat = y / radius;
    let Line(p0, p1) = *line;
    let dl = p1 - p0;
    let w = p0 - center;
    let a_iv = interval_dot(dl, dl);
    let b_iv = interval_at(2.0) * interval_dot(w, dl);
    let c_iv = interval_dot(w, w) - interval_at(radius) * interval_at(radius);
    let disc_iv = b_iv * b_iv - interval_at(4.0) * a_iv * c_iv;
    let a = dl.dot(dl);
    let b = 2.0 * w.dot(dl);
    let c = w.dot(w) - radius * radius;
    let disc = b * b - 4.0 * a * c;

    let mut records = Vec::new();
    match classify_discriminant(disc_iv) {
        Some(Discriminant::Negative) => return empty(budget),
        Some(Discriminant::Zero) => {
            let s = -b / (2.0 * a);
            chord_candidate(
                p0 + s * dl,
                plane,
                *face_u_range,
                *face_v_range,
                center,
                u_hat,
                v_hat,
                *edge_t_range,
                true,
                &mut records,
            );
        }
        Some(Discriminant::Positive) => {
            let s = disc.sqrt();
            for root in [(-b + s) / (2.0 * a), (-b - s) / (2.0 * a)] {
                chord_candidate(
                    p0 + root * dl,
                    plane,
                    *face_u_range,
                    *face_v_range,
                    center,
                    u_hat,
                    v_hat,
                    *edge_t_range,
                    false,
                    &mut records,
                );
            }
        }
        None => return Err(unresolved()),
    }
    certified(records, budget)
}

/// One chord point: bounded to the face box and the circle's edge.
#[allow(clippy::too_many_arguments)]
fn chord_candidate(
    q: Point3,
    plane: &Plane,
    face_u_range: (f64, f64),
    face_v_range: (f64, f64),
    center: Point3,
    u_hat: Vector3,
    v_hat: Vector3,
    edge_t_range: (f64, f64),
    tangent: bool,
    records: &mut Vec<ContactRecord>,
) {
    let face = plane_box_location(plane, q, face_u_range, face_v_range);
    if face == FaceLocation::Outside {
        return;
    }
    let theta_position = locate(angle_of(q, center, u_hat, v_hat), edge_t_range);
    if matches!(theta_position, RangePosition::Below | RangePosition::Above) {
        return;
    }
    let kind = if tangent {
        ContactEventKind::Tangency
    } else if face == FaceLocation::Boundary || theta_position == RangePosition::AtBoundary {
        ContactEventKind::EndpointTouch
    } else {
        ContactEventKind::Transverse
    };
    records.push(point_record(q, kind));
}

// ---------------------------------------------------------------------------
// Â§5.4 Circle Ã— Cylinder â€” latitudinal coincident only.
// ---------------------------------------------------------------------------

fn circle_cylinder(
    circle: &PlacedCircle,
    edge_t_range: &(f64, f64),
    cylinder: &Cylinder,
    face_u_range: &(f64, f64),
    face_v_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let (center, x, y, radius) = circle_frame(circle);
    let normal = x.cross(y).normalize();
    let c = cylinder.center();
    let r = cylinder.radius();
    let z_hat = Vector3::unit_z();
    // Latitudinal iff (all decisive predicates): the circle's plane normal is
    // parallel to the axis, the center is on the axis, the radii agree, and
    // the circle's height is within the face's v_range. Anything else is
    // conicÃ—conic and defers.
    let axis_parallel = interval_cross(normal, z_hat)
        .iter()
        .all(|c| decisively_zero(*c));
    let on_axis = decisively_zero(interval_at(center.x - c.x))
        && decisively_zero(interval_at(center.y - c.y));
    let equal_radius = decisively_zero(interval_at(radius - r));
    if !(axis_parallel && on_axis && equal_radius) {
        return Err(deferred());
    }
    match locate(center.z, *face_v_range) {
        RangePosition::Below | RangePosition::Above => return empty(budget),
        RangePosition::Inside | RangePosition::AtBoundary => {}
    }
    // The wall's u IS the circle's angle, so the coincident sub-arc is
    // [0, TAU) âˆ© u_range, intersected with the edge's own t_range.
    let t_range = clip_interval_to_range((0.0, TAU), *face_u_range)
        .and_then(|r| clip_interval_to_range(r, *edge_t_range));
    match t_range {
        Some((t_lo, t_hi)) => certified(
            vec![arc_record(ExactCurve::Circle(*circle), (t_lo, t_hi))],
            budget,
        ),
        None => empty(budget),
    }
}

// ---------------------------------------------------------------------------
// EE dispatcher.
// ---------------------------------------------------------------------------

/// Answers "where do two edges meet, and how", certified and bounded to both
/// edges' `t_range`s. The Line/Circle order is normalized inside: the solver
/// always sees `(line, circle)`, so the two orders commute.
pub fn ee_contact(
    lhs_curve: &CanonicalCurve,
    lhs_t_range: &(f64, f64),
    rhs_curve: &CanonicalCurve,
    rhs_t_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    match (lhs_curve, rhs_curve) {
        (CanonicalCurve::Line(l), CanonicalCurve::Line(r)) => {
            line_line(l, lhs_t_range, r, rhs_t_range, budget)
        }
        (CanonicalCurve::Line(l), CanonicalCurve::Circle(c)) => {
            line_circle(l, lhs_t_range, c, rhs_t_range, budget)
        }
        (CanonicalCurve::Circle(c), CanonicalCurve::Line(l)) => {
            line_circle(l, rhs_t_range, c, lhs_t_range, budget)
        }
        (CanonicalCurve::Circle(_), CanonicalCurve::Circle(_)) => Err(deferred()),
    }
}

// ---------------------------------------------------------------------------
// Â§6.1 Line Ã— Line.
// ---------------------------------------------------------------------------

fn line_line(
    l: &Line<Point3>,
    lhs_t_range: &(f64, f64),
    r: &Line<Point3>,
    rhs_t_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let Line(a0, b0) = *l;
    let Line(a1, b1) = *r;
    let d0 = b0 - a0;
    let d1 = b1 - a1;
    let cross = interval_cross(d0, d1);
    if cross.iter().any(|c| excludes_zero(*c)) {
        // Not parallel: coplanarity decides a crossing point vs skew.
        let t_iv = interval_dot(d0.cross(d1), a1 - a0);
        if decisively_zero(t_iv) {
            // Coplanar: solve sÂ·d0 âˆ’ tÂ·d1 = a1 âˆ’ a0 by the 2Ã—2 system in the
            // two dot products (determinant |d0|Â²|d1|Â² âˆ’ (d0Â·d1)Â² = |c|Â²,
            // decisive nonzero).
            let v = a1 - a0;
            let m00 = d0.dot(d0);
            let m11 = d1.dot(d1);
            let m01 = d0.dot(d1);
            let det = m00 * m11 - m01 * m01;
            let r0 = v.dot(d0);
            let r1 = v.dot(d1);
            let s = (-r0 * m11 + m01 * r1) / -det;
            let t = (m00 * r1 - m01 * r0) / -det;
            let q = a0 + s * d0;
            let s_position = locate(s, *lhs_t_range);
            let t_position = locate(t, *rhs_t_range);
            if matches!(
                (s_position, t_position),
                (RangePosition::Below | RangePosition::Above, _)
                    | (_, RangePosition::Below | RangePosition::Above)
            ) {
                return empty(budget);
            }
            let kind = if s_position == RangePosition::AtBoundary
                || t_position == RangePosition::AtBoundary
            {
                ContactEventKind::EndpointTouch
            } else {
                ContactEventKind::Transverse
            };
            certified(vec![point_record(q, kind)], budget)
        } else if excludes_zero(t_iv) {
            // Skew: no contact.
            empty(budget)
        } else {
            Err(unresolved())
        }
    } else if cross.iter().all(|c| decisively_zero(*c)) {
        // Parallel: collinearity decides a coincident arc vs nothing.
        let w = interval_cross(a1 - a0, d0);
        if w.iter().all(|c| decisively_zero(*c)) {
            // Collinear: the rhs segment spans
            // [t_base, t_base + |d1|/|d0|] (direction sign from d1Â·d0) in the
            // lhs line's parameter; the overlap with the lhs range is the
            // coincident sub-arc.
            let t_base = (a1 - a0).dot(d0) / d0.dot(d0);
            let len_ratio = d1.magnitude() / d0.magnitude();
            let dir = if d1.dot(d0) >= 0.0 { 1.0 } else { -1.0 };
            let span_hi = t_base + dir * len_ratio;
            let (lo, hi) = if span_hi >= t_base {
                (t_base, span_hi)
            } else {
                (span_hi, t_base)
            };
            match clip_interval_to_range((lo, hi), *lhs_t_range) {
                Some((t_lo, t_hi)) => {
                    certified(vec![arc_record(ExactCurve::Line(*l), (t_lo, t_hi))], budget)
                }
                None => empty(budget),
            }
        } else if w.iter().any(|c| excludes_zero(*c)) {
            empty(budget)
        } else {
            Err(unresolved())
        }
    } else {
        Err(unresolved())
    }
}

// ---------------------------------------------------------------------------
// Â§6.2 Line Ã— Circle.
// ---------------------------------------------------------------------------

fn line_circle(
    line: &Line<Point3>,
    line_t_range: &(f64, f64),
    circle: &PlacedCircle,
    circle_t_range: &(f64, f64),
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let Line(a, b) = *line;
    let d = b - a;
    let (center, x, y, radius) = circle_frame(circle);
    let u_hat = x / radius;
    let v_hat = y / radius;
    let normal = x.cross(y).normalize();
    let dn_iv = interval_dot(d, normal);
    if excludes_zero(dn_iv) {
        // The line meets the circle's plane exactly once.
        let dn = d.dot(normal);
        let t0 = (center - a).dot(normal) / dn;
        let edge_position = locate(t0, *line_t_range);
        if matches!(edge_position, RangePosition::Below | RangePosition::Above) {
            return empty(budget);
        }
        let q = a + t0 * d;
        let w = q - center;
        let rho_iv = interval_at(w.dot(u_hat)) * interval_at(w.dot(u_hat))
            + interval_at(w.dot(v_hat)) * interval_at(w.dot(v_hat));
        let r_sq_iv = interval_at(radius) * interval_at(radius);
        match three_way(rho_iv, r_sq_iv) {
            Some(Ordering::Equal) => {
                let theta_position = locate(angle_of(q, center, u_hat, v_hat), *circle_t_range);
                if matches!(theta_position, RangePosition::Below | RangePosition::Above) {
                    return empty(budget);
                }
                let kind = if edge_position == RangePosition::AtBoundary
                    || theta_position == RangePosition::AtBoundary
                {
                    ContactEventKind::EndpointTouch
                } else {
                    ContactEventKind::Transverse
                };
                certified(vec![point_record(q, kind)], budget)
            }
            Some(Ordering::Less) | Some(Ordering::Greater) => empty(budget),
            None => Err(unresolved()),
        }
    } else if decisively_zero(dn_iv) {
        // The line is parallel to the circle's plane; the in-plane offset
        // decides.
        let h_iv = interval_dot(a - center, normal);
        if decisively_zero(h_iv) {
            line_circle_chord(
                a,
                d,
                line_t_range,
                circle_t_range,
                center,
                u_hat,
                v_hat,
                radius,
                budget,
            )
        } else if excludes_zero(h_iv) {
            empty(budget)
        } else {
            Err(unresolved())
        }
    } else {
        Err(unresolved())
    }
}

/// The 2-D chord: the line lies in the circle's plane; the quadratic
/// `|a + tÂ·d âˆ’ m|Â² = rÂ²` gives 0/1/2 roots, each bounded to both edges'
/// `t_range`s.
#[allow(clippy::too_many_arguments)]
fn line_circle_chord(
    a: Point3,
    d: Vector3,
    line_t_range: &(f64, f64),
    circle_t_range: &(f64, f64),
    center: Point3,
    u_hat: Vector3,
    v_hat: Vector3,
    radius: f64,
    budget: &Budget,
) -> Outcome<ContactComplex> {
    let w = a - center;
    let a_iv = interval_dot(d, d);
    let b_iv = interval_at(2.0) * interval_dot(w, d);
    let c_iv = interval_dot(w, w) - interval_at(radius) * interval_at(radius);
    let disc_iv = b_iv * b_iv - interval_at(4.0) * a_iv * c_iv;
    let aq = d.dot(d);
    let bq = 2.0 * w.dot(d);
    let cq = w.dot(w) - radius * radius;
    let disc = bq * bq - 4.0 * aq * cq;

    let mut records = Vec::new();
    match classify_discriminant(disc_iv) {
        Some(Discriminant::Negative) => return empty(budget),
        Some(Discriminant::Zero) => {
            let t0 = -bq / (2.0 * aq);
            if let Some(record) = circle_chord_candidate(
                a + t0 * d,
                t0,
                *line_t_range,
                center,
                u_hat,
                v_hat,
                *circle_t_range,
                true,
            ) {
                records.push(record);
            }
        }
        Some(Discriminant::Positive) => {
            let s = disc.sqrt();
            for t in [(-bq + s) / (2.0 * aq), (-bq - s) / (2.0 * aq)] {
                if let Some(record) = circle_chord_candidate(
                    a + t * d,
                    t,
                    *line_t_range,
                    center,
                    u_hat,
                    v_hat,
                    *circle_t_range,
                    false,
                ) {
                    records.push(record);
                }
            }
        }
        None => return Err(unresolved()),
    }
    certified(records, budget)
}

/// One in-plane chord point, bounded to both edges' `t_range`s.
#[allow(clippy::too_many_arguments)]
fn circle_chord_candidate(
    q: Point3,
    t: f64,
    line_t_range: (f64, f64),
    center: Point3,
    u_hat: Vector3,
    v_hat: Vector3,
    circle_t_range: (f64, f64),
    tangent: bool,
) -> Option<ContactRecord> {
    let t_position = locate(t, line_t_range);
    if matches!(t_position, RangePosition::Below | RangePosition::Above) {
        return None;
    }
    let theta_position = locate(angle_of(q, center, u_hat, v_hat), circle_t_range);
    if matches!(theta_position, RangePosition::Below | RangePosition::Above) {
        return None;
    }
    let kind = if tangent {
        ContactEventKind::Tangency
    } else if t_position == RangePosition::AtBoundary || theta_position == RangePosition::AtBoundary
    {
        ContactEventKind::EndpointTouch
    } else {
        ContactEventKind::Transverse
    };
    Some(point_record(q, kind))
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built dyadic witnesses are not such a
// path; the unwraps below cannot fire for the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::*;
    use super::*;
    use truck_base::cgmath64::{EuclideanSpace, Matrix4, Vector4};
    use truck_geometry::decorators::{Processor, TrimmedCurve};
    use truck_geometry::specifieds::UnitCircle;

    /// Float slack on unit-scale witness coordinates and residuals â€”
    /// dimensionless in every use, never a model-space length.
    const SLACK: f64 = 1.0e-9; // H-3: float slack on unit-scale witness coordinates and residuals, not a length

    /// A full-range unit circle with the given center, in the z = 0 plane.
    fn unit_circle(center: Point3) -> PlacedCircle {
        let m = Matrix4 {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        };
        Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            m,
        )
    }

    fn edge_line(a: Point3, b: Point3) -> BoundedStratum {
        BoundedStratum::Edge {
            curve: CanonicalCurve::Line(Line(a, b)),
            t_range: (0.0, 1.0),
        }
    }

    fn edge_line_t(a: Point3, b: Point3, t_range: (f64, f64)) -> BoundedStratum {
        BoundedStratum::Edge {
            curve: CanonicalCurve::Line(Line(a, b)),
            t_range,
        }
    }

    fn edge_circle(circle: PlacedCircle) -> BoundedStratum {
        BoundedStratum::Edge {
            curve: CanonicalCurve::Circle(circle),
            t_range: (0.0, TAU),
        }
    }

    fn face_plane(plane: Plane, u_range: (f64, f64), v_range: (f64, f64)) -> BoundedStratum {
        BoundedStratum::Face {
            surface: CanonicalSurface::Plane(plane),
            u_range,
            v_range,
        }
    }

    fn face_cylinder(
        cylinder: Cylinder,
        u_range: (f64, f64),
        v_range: (f64, f64),
    ) -> BoundedStratum {
        BoundedStratum::Face {
            surface: CanonicalSurface::Cylinder(cylinder),
            u_range,
            v_range,
        }
    }

    /// A unit cylinder at the given center, avoiding `unwrap` by construction.
    fn unit_cylinder(center: Point3) -> Cylinder {
        match Cylinder::new(center, 1.0) {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!("a unit cylinder cannot refuse: {refusal:?}"),
        }
    }

    /// Structural equality of two contact complexes: `(dimension, kind,
    /// locus)`, with float slack on point coordinates.
    fn complex_structurally_equal(a: &ContactComplex, b: &ContactComplex) -> bool {
        if a.contacts.len() != b.contacts.len() {
            return false;
        }
        a.contacts.iter().zip(&b.contacts).all(|(x, y)| {
            x.dimension == y.dimension && x.kind == y.kind && loci_equal(&x.locus, &y.locus)
        })
    }

    fn loci_equal(a: &ContactLocus, b: &ContactLocus) -> bool {
        match (a, b) {
            (ContactLocus::Coincident, ContactLocus::Coincident) => true,
            (ContactLocus::Point(p), ContactLocus::Point(q)) => (*p - *q).magnitude() < SLACK,
            (
                ContactLocus::BoundedCurve {
                    curve: c0,
                    t_range: r0,
                },
                ContactLocus::BoundedCurve {
                    curve: c1,
                    t_range: r1,
                },
            ) => {
                (r0.0 - r1.0).abs() < SLACK
                    && (r0.1 - r1.1).abs() < SLACK
                    && format!("{c0:?}") == format!("{c1:?}")
            }
            _ => false,
        }
    }

    #[test]
    fn contact_fe_line_punctures_cylinder_wall_returns_point() {
        // A line through the axis region crossing the unit cylinder wall once:
        // edge (0,0,0)â†’(2,2,0) punctures the wall at t = âˆš2/4, the point
        // (1/âˆš2, 1/âˆš2, 0).
        let cylinder = unit_cylinder(Point3::origin());
        let edge = edge_line(Point3::origin(), Point3::new(2.0, 2.0, 0.0));
        let face = face_cylinder(cylinder, (0.0, TAU), (-1.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge, &face, &mut budget)
            .expect("a dyadic line/cylinder puncture is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.cert.props.get(Prop::AnalyticCarrier), Truth::True);
        assert_eq!(out.value.contacts.len(), 1, "exactly one puncture");
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Point0);
        assert_eq!(record.kind, ContactEventKind::Transverse);
        let q = match &record.locus {
            ContactLocus::Point(q) => q,
            other => unreachable!("expected a Point locus, got {other:?}"),
        };
        // The puncture lies on the wall at 45Â°.
        assert!(
            (q.x - std::f64::consts::FRAC_1_SQRT_2).abs() < SLACK,
            "{q:?}"
        ); // H-3: float slack on a unit-scale coordinate, not a length
        assert!(
            (q.y - std::f64::consts::FRAC_1_SQRT_2).abs() < SLACK,
            "{q:?}"
        ); // H-3: float slack on a unit-scale coordinate, not a length
        assert!(q.z.abs() < SLACK, "{q:?}"); // H-3: float slack on a unit-scale coordinate, not a length
        assert!(q.z > -1.0 && q.z < 1.0, "{q:?}"); // inside the face's v_range
    }

    #[test]
    fn contact_fe_line_in_plane_returns_coincident_arc() {
        // A line lying in the z = 0 plane, clipped to the face box: the whole
        // edge is inside the box, so the reported sub-arc is the whole edge
        // and its endpoint images lie strictly inside the face box.
        let edge = edge_line(Point3::new(0.25, 0.25, 0.0), Point3::new(0.75, 0.75, 0.0));
        let face = face_plane(Plane::xy(), (0.0, 1.0), (0.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge, &face, &mut budget).expect("a dyadic in-plane edge is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        let (curve, t_range) = match &record.locus {
            ContactLocus::BoundedCurve { curve, t_range } => (curve, t_range),
            other => unreachable!("expected a BoundedCurve locus, got {other:?}"),
        };
        assert!(matches!(curve, ExactCurve::Line(_)));
        assert_eq!(*t_range, (0.0, 1.0), "the clip contains the whole edge");
        assert!(
            t_range.0 >= 0.0 && t_range.1 <= 1.0,
            "the sub-arc is inside the edge's own t_range"
        );
        // The sub-arc's endpoint images lie strictly inside the face box.
        let ExactCurve::Line(line) = curve else {
            unreachable!("just matched the line arm");
        };
        for t in [0.0, 1.0] {
            let prm = Plane::xy().get_parameter(line.subs(t));
            assert!(
                (0.0..=1.0).contains(&prm.x),
                "u = {} outside the face box",
                prm.x
            );
            assert!(
                (0.0..=1.0).contains(&prm.y),
                "v = {} outside the face box",
                prm.y
            );
        }
    }

    #[test]
    fn contact_fe_circle_on_plane_returns_coincident_arc() {
        // A cap circle (unit circle at z = 0) lying in the xy-plane face: the
        // whole circle is inside the box, so the coincident locus is the whole
        // [0, TAU).
        let edge = edge_circle(unit_circle(Point3::origin()));
        let face = face_plane(Plane::xy(), (-2.0, 2.0), (-2.0, 2.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge, &face, &mut budget)
            .expect("a dyadic coincident cap circle is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        let (curve, t_range) = match &record.locus {
            ContactLocus::BoundedCurve { curve, t_range } => (curve, t_range),
            other => unreachable!("expected a BoundedCurve locus, got {other:?}"),
        };
        assert!(matches!(curve, ExactCurve::Circle(_)));
        assert_eq!(*t_range, (0.0, TAU), "the whole circle is inside the box");
    }

    #[test]
    fn contact_fe_puncture_outside_bounds_returns_empty() {
        // The same lineÃ—cylinder geometry, with the edge's t_range cut so the
        // puncture (t = âˆš2/4 â‰ˆ 0.3536) is outside it.
        let cylinder = unit_cylinder(Point3::origin());
        let edge = edge_line_t(Point3::origin(), Point3::new(2.0, 2.0, 0.0), (0.5, 1.0));
        let face = face_cylinder(cylinder, (0.0, TAU), (-1.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge, &face, &mut budget).expect("a bounded puncture is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert!(
            out.value.contacts.is_empty(),
            "the puncture is outside the edge's t_range"
        );
    }

    #[test]
    fn contact_ee_line_circle_returns_point() {
        // A vertical line edge through (0, 1) crossing the unit cap circle at
        // z = 0 at (0, 1, 0): one transverse point.
        let line_edge = edge_line(Point3::new(0.0, 1.0, -1.0), Point3::new(0.0, 1.0, 1.0));
        let circle_edge = edge_circle(unit_circle(Point3::origin()));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&line_edge, &circle_edge, &mut budget)
            .expect("a dyadic line/circle crossing is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Point0);
        assert_eq!(record.kind, ContactEventKind::Transverse);
        let q = match &record.locus {
            ContactLocus::Point(q) => q,
            other => unreachable!("expected a Point locus, got {other:?}"),
        };
        assert!(q.x.abs() < SLACK, "{q:?}"); // H-3: float slack on a unit-scale coordinate, not a length
        assert!((q.y - 1.0).abs() < SLACK, "{q:?}"); // H-3: float slack on a unit-scale coordinate, not a length
        assert!(q.z.abs() < SLACK, "{q:?}"); // H-3: float slack on a unit-scale coordinate, not a length
    }

    #[test]
    fn contact_ee_coincident_lines_return_arc() {
        // Two collinear overlapping segments on the x-axis: [0, 1] and
        // [0.5, 1.5]. The overlap is the lhs sub-arc [0.5, 1].
        let lhs = edge_line(Point3::origin(), Point3::new(1.0, 0.0, 0.0));
        let rhs = edge_line(Point3::new(0.5, 0.0, 0.0), Point3::new(1.5, 0.0, 0.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget)
            .expect("collinear overlapping dyadic segments are decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        let (curve, t_range) = match &record.locus {
            ContactLocus::BoundedCurve { curve, t_range } => (curve, t_range),
            other => unreachable!("expected a BoundedCurve locus, got {other:?}"),
        };
        assert!(matches!(curve, ExactCurve::Line(_)));
        // [0.5, 1.5] âˆ© [0, 1] = [0.5, 1] in the lhs line's parameter.
        assert_eq!(*t_range, (0.5, 1.0));
    }

    #[test]
    fn contact_fe_ee_commutes() {
        // FE: the lineÃ—cylinder puncture; EE: two coplanar crossing lines.
        // Both orders must produce structurally equal ContactComplex values.
        let fe_edge = edge_line(Point3::origin(), Point3::new(2.0, 2.0, 0.0));
        let fe_face = face_cylinder(unit_cylinder(Point3::origin()), (0.0, TAU), (-1.0, 1.0));
        let mut budget = Budget::new(100, 100, 100);
        let ab = contact(&fe_edge, &fe_face, &mut budget).expect("decidable FE pair");
        let mut budget = Budget::new(100, 100, 100);
        let ba = contact(&fe_face, &fe_edge, &mut budget).expect("decidable FE pair (swapped)");
        assert!(
            complex_structurally_equal(&ab.value, &ba.value),
            "the FE pair commutes"
        );

        let ee_lhs = edge_line(Point3::new(-1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
        let ee_rhs = edge_line(Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0));
        let mut budget = Budget::new(100, 100, 100);
        let ab = contact(&ee_lhs, &ee_rhs, &mut budget).expect("decidable EE pair");
        let mut budget = Budget::new(100, 100, 100);
        let ba = contact(&ee_rhs, &ee_lhs, &mut budget).expect("decidable EE pair (swapped)");
        assert!(
            complex_structurally_equal(&ab.value, &ba.value),
            "the EE pair commutes"
        );
    }

    #[test]
    fn contact_fe_circle_latitudinal_on_cylinder_returns_coincident() {
        // A unit circle at z = 1 is a latitudinal circle on the unit cylinder:
        // the coincident locus is the whole [0, TAU) (the face's u_range and
        // the edge are both full).
        let edge = edge_circle(unit_circle(Point3::new(0.0, 0.0, 1.0)));
        let face = face_cylinder(unit_cylinder(Point3::origin()), (0.0, TAU), (-2.0, 2.0));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&edge, &face, &mut budget)
            .expect("a dyadic latitudinal circle/cylinder pair is decidable");
        assert_eq!(out.cert.method, Method::Exact);
        assert_eq!(out.value.contacts.len(), 1);
        let record = out.value.contacts.first().expect("one record");
        assert_eq!(record.dimension, ContactDimension::Arc1);
        assert_eq!(record.kind, ContactEventKind::CoincidentInterval);
        let (curve, t_range) = match &record.locus {
            ContactLocus::BoundedCurve { curve, t_range } => (curve, t_range),
            other => unreachable!("expected a BoundedCurve locus, got {other:?}"),
        };
        assert!(matches!(curve, ExactCurve::Circle(_)));
        assert_eq!(*t_range, (0.0, TAU));
    }

    #[test]
    fn contact_deferred_families_still_refuse() {
        // A CircleÃ—Circle EE pair is not in the Â§6 table: it refuses with
        // ContactReductionDeferred.
        let lhs = edge_circle(unit_circle(Point3::origin()));
        let rhs = edge_circle(unit_circle(Point3::new(0.0, 0.0, 1.0)));
        let mut budget = Budget::new(100, 100, 100);
        let out = contact(&lhs, &rhs, &mut budget);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "a CircleÃ—Circle EE pair is the deferred funnel"
        );
    }
}
