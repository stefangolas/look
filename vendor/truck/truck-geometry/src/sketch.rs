//! PB-002-SKETCH-ARCS — the sketch authoring layer over the (s, v) chart.
//!
//! This module is the AUTHORING half of the arrangement packet family
//! (`arrange.rs` carries the engine): 3-point and radius arc constructors that
//! produce trimmed [`SketchArc`]s, periodic/open spline authoring over the
//! certified-PL [`ChartCurve`] machinery (the text-to-cad corpus idiom
//! `Spline(*pts, periodic=True)` — a direct analytic spline carrier is NOT
//! landed here, the certified-PL path is), and mixed-loop assembly that pairs
//! shared endpoints exactly within the ctx tolerance before the landed
//! `arrange`/`arrange_chart` engines subdivide the profile.
//!
//! The module is a child of `arrange` (registered there by `#[path]`) so the
//! constructors can reach the arrangement's private carrier machinery
//! (`Carrier2D`, the exact dyadic intersection dispatcher). All authoring is
//! additive and refuses typed on degenerate input (H-1, H-3): a collinear
//! three-point arc, a zero-radius or inverted-range radius arc, non-finite
//! input, an open spline handed a loop, and an unclosed mixed loop are typed
//! `Refusal`s, never panics and never silent geometry.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use super::*;
use std::f64::consts::TAU;
use truck_base::evidence::{EnvelopeCase, Refusal};

/// The ring-closure tolerance of the assembly step: the same `64 * TOLERANCE`
/// the landed `arrange` chain check uses (shared endpoints must agree within
/// the ctx tolerance; mismatches refuse typed).
const RING_TOLERANCE: f64 = 64.0 * TOLERANCE;

/// The typed refusal of a degenerate authoring input (a collinear or
/// coincident three-point set, a zero radius, an inverted or over-tight arc
/// window).
fn degenerate_refusal() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate)
}

/// The typed refusal of a construction outside the authoring envelope.
fn construct_refused() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)
}

/// Whether both coordinates of `p` are finite.
fn finite2(p: Point2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The CCW angle of `p` about `center`, normalized into `[0, TAU)`.
fn angle_of(center: Point2, p: Point2) -> f64 {
    let v = p - center;
    let mut ang = f64::atan2(v.y, v.x);
    if ang < 0.0 {
        ang += TAU;
    }
    ang
}

/// The in-plane carrier of an authored arc: an axis-aligned trimmed circle
/// (the `e_u`/`e_v` basis is the +x/+y frame, radius-scaled) over a CCW angle
/// window `[t0, t1]`, with an optional reversed (CW-traversed) parameterization
/// for three-point arcs whose through point forces the long way round.
fn axis_carrier(center: Point2, radius: f64, t0: f64, t1: f64, reversed: bool) -> CircleCarrier {
    CircleCarrier {
        center,
        radius,
        t0,
        t1,
        e_u: Vector2::new(radius, 0.0),
        e_v: Vector2::new(0.0, radius),
        reversed,
    }
}

/// An authored planar arc: a trimmed-circle carrier over an angle window.
///
/// The trimmed span is exactly the `(t0, t1)` window the landed
/// `Carrier2D::Circle` envelope carries, so an authored arc participates in
/// the certified arrangement crossings directly.
#[derive(Clone, Copy, Debug)]
pub struct SketchArc {
    carrier: CircleCarrier,
}

impl SketchArc {
    /// The arc's center.
    pub fn center(&self) -> Point2 {
        self.carrier.center
    }

    /// The arc's radius.
    pub fn radius(&self) -> f64 {
        self.carrier.radius
    }

    /// The trimmed parameter window `(t0, t1)` on the supporting circle.
    pub fn range(&self) -> (f64, f64) {
        (self.carrier.t0, self.carrier.t1)
    }

    /// Whether the arc is a full turn (its window closes on itself).
    pub fn is_full(&self) -> bool {
        self.carrier.t1 - self.carrier.t0 == TAU
    }

    /// The arc's start point (its `t0` end).
    pub fn start(&self) -> Point2 {
        self.carrier.subs(self.carrier.t0)
    }

    /// The arc's end point (its `t1` end).
    pub fn end(&self) -> Point2 {
        self.carrier.subs(self.carrier.t1)
    }

    /// Evaluates the arc at parameter `t` (in its own trimmed window).
    pub fn subs(&self, t: f64) -> Point2 {
        self.carrier.subs(t)
    }

    /// Re-emits the arc as the canonical `Curve::Circle` profile curve the
    /// landed `arrange` consumes (a placed, trimmed unit circle in the z = 0
    /// plane).
    pub fn to_curve(&self) -> Curve {
        let c = self.carrier.center;
        let m = Matrix4 {
            x: Vector4::new(self.carrier.radius, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, self.carrier.radius, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(c.x, c.y, 0.0, 1.0),
        };
        let trimmed = TrimmedCurve::new(
            UnitCircle::<Point3>::new(),
            (self.carrier.t0, self.carrier.t1),
        );
        let processor = Processor::with_transform(trimmed, m);
        let circle = if self.carrier.reversed {
            processor.inverse()
        } else {
            processor
        };
        Curve::Circle(circle)
    }
}

/// Authors the arc from `center` over radius `radius`, sweeping CCW from angle
/// `a0` to angle `a1` (radians, any finite values; the window is normalized
/// into one turn). A non-positive radius, a non-finite input, or an inverted
/// (`a1 <= a0`) or over-tight (`a1 - a0 > TAU`) window refuses typed.
pub fn arc_radius(
    center: Point2,
    radius: f64,
    a0: f64,
    a1: f64,
) -> std::result::Result<SketchArc, Refusal> {
    if !finite2(center) || !radius.is_finite() || !a0.is_finite() || !a1.is_finite() {
        return Err(degenerate_refusal());
    }
    if radius <= 0.0 {
        return Err(degenerate_refusal());
    }
    let span = a1 - a0;
    if span <= 0.0 || span > TAU {
        return Err(degenerate_refusal());
    }
    let t0 = a0.rem_euclid(TAU);
    let t1 = t0 + span;
    Ok(SketchArc {
        carrier: axis_carrier(center, radius, t0, t1, false),
    })
}

/// Authors the arc through `p0`, `p1` and `p2`: the arc starts at `p0`, passes
/// through `p1`, and ends at `p2`, on the unique circle through the three
/// points (the trimmed window is the one that actually contains `p1`). A
/// collinear or coincident triple (no circle exists) or a non-finite input
/// refuses typed.
pub fn arc_three_point(
    p0: Point2,
    p1: Point2,
    p2: Point2,
) -> std::result::Result<SketchArc, Refusal> {
    if !finite2(p0) || !finite2(p1) || !finite2(p2) {
        return Err(degenerate_refusal());
    }
    let a = p1 - p0;
    let b = p2 - p0;
    let cross = a.x * b.y - a.y * b.x;
    if cross == 0.0 {
        // `p0`, `p1`, `p2` collinear (or coincident): no circle through them.
        return Err(degenerate_refusal());
    }
    let aa = a.x * a.x + a.y * a.y;
    let bb = b.x * b.x + b.y * b.y;
    // The center `p0 + c` solves `2 c·a = |a|²` and `2 c·b = |b|²` (the
    // perpendicular-bisector system), exactly in floats for this envelope.
    let denom = 2.0 * cross;
    let cx = (aa * b.y - a.y * bb) / denom;
    let cy = (a.x * bb - aa * b.x) / denom;
    let center = p0 + Vector2::new(cx, cy);
    let radius = (p0 - center).magnitude();
    if !radius.is_finite() || radius <= 0.0 {
        return Err(degenerate_refusal());
    }
    let t0 = angle_of(center, p0);
    let t2 = angle_of(center, p2);
    let t1 = angle_of(center, p1);
    // The CCW sweep from `p0` to `p2` and where `p1` sits on it.
    let ccw = (t2 - t0).rem_euclid(TAU);
    let rel = (t1 - t0).rem_euclid(TAU);
    if ccw == 0.0 {
        return Err(degenerate_refusal());
    }
    if rel > 0.0 && rel < ccw {
        // `p1` lies on the CCW arc `p0 -> p2`.
        Ok(SketchArc {
            carrier: axis_carrier(center, radius, t0, t0 + ccw, false),
        })
    } else {
        // `p1` lies on the long way round: the CW arc `p0 -> p2`, which is the
        // CCW window `angle(p2) -> angle(p0)` traversed in reverse. A reversed
        // carrier starts (at parameter `a`) on the CCW angle `b`, so the range
        // `(a, b) = (angle(p2), angle(p0))` (lifted to keep `b > a`) starts at
        // `p0` and ends at `p2`.
        let (a, b) = if t2 < t0 { (t2, t0) } else { (t2, t0 + TAU) };
        Ok(SketchArc {
            carrier: axis_carrier(center, radius, a, b, true),
        })
    }
}

/// Authors a chart curve through `points` over the certified-PL
/// [`ChartCurve`] machinery of BIE-005-ARRANGE.
///
/// - `periodic == true`: the curve is a closed ring. It closes EXACTLY: when
///   the final point already repeats the first, the samples are used as given;
///   otherwise the first point is appended as the seam sample.
/// - `periodic == false`: the curve is an open chain. A point list whose first
///   and last samples coincide is a loop handed to the OPEN authoring and
///   refuses typed (author it periodic instead).
/// - fewer than two points or any non-finite point refuses typed; the
///   underlying [`ChartCurve::try_new`] refusal (zero-length segments, an
///   under-sized ring) is inherited unchanged.
pub fn spline(points: &[Point2], periodic: bool) -> std::result::Result<ChartCurve, Refusal> {
    let first = match points.first() {
        Some(&p) => p,
        None => return Err(construct_refused()),
    };
    let last = match points.last() {
        Some(&p) => p,
        None => return Err(construct_refused()),
    };
    let mut samples: Vec<(f64, f64)> = Vec::with_capacity(points.len() + 1);
    for p in points {
        if !finite2(*p) {
            return Err(construct_refused());
        }
        samples.push((p.x, p.y));
    }
    if periodic {
        if last != first {
            samples.push((first.x, first.y));
        }
    } else if first == last {
        // An open spline cannot author a loop: a ring must be declared
        // periodic (ChartCurve::try_new enforces the same closed/open split).
        return Err(construct_refused());
    }
    let ok = ChartCurve::try_new(samples, periodic, true)?;
    Ok(ok.value)
}

/// One segment of a sketch profile: an analytic profile curve (a line or an
/// authored arc) or a certified-PL chart curve (an authored spline). A chart
/// segment expands to one z = 0 `Curve::Line` per PL sample when the loop is
/// flattened for the landed `arrange`. The analytic carrier is boxed: `Curve`
/// is large next to a [`ChartCurve`], and segments are assembled transiently.
#[derive(Clone, Debug)]
pub enum SketchSegment {
    /// An analytic profile curve (`Curve::Line` or a `SketchArc`'s
    /// `Curve::Circle`).
    Analytic(Box<Curve>),
    /// A certified-PL chart curve (an authored spline).
    Chart(ChartCurve),
}

/// A closed mixed loop: the flattened analytic profile (lines, arcs, and the
/// PL segments of authored splines) whose chains all close exactly within the
/// ctx tolerance. Feed [`SketchLoop::profile`] to the landed `arrange`.
#[derive(Clone, Debug)]
pub struct SketchLoop {
    profile: Vec<Curve>,
}

impl SketchLoop {
    /// The flattened closed profile, in segment order — the form the landed
    /// `arrange` engine consumes.
    pub fn profile(&self) -> &[Curve] {
        &self.profile
    }
}

/// The start point of a profile curve.
fn curve_start(c: &Curve) -> Point3 {
    c.subs(c.range_tuple().0)
}

/// The end point of a profile curve.
fn curve_end(c: &Curve) -> Point3 {
    c.subs(c.range_tuple().1)
}

/// Groups a flattened profile into maximal endpoint-adjacent chains, the same
/// rule the landed `arrange` chain check applies (consecutive curves whose end
/// meets the next start within the ctx tolerance share a chain).
fn chain_groups(profile: &[Curve]) -> Vec<Vec<usize>> {
    let mut chains: Vec<Vec<usize>> = Vec::new();
    for i in 0..profile.len() {
        let cur = match profile.get(i) {
            Some(c) => c,
            None => continue,
        };
        let cur_start = curve_start(cur);
        let joins = match chains.last().and_then(|c| c.last()).copied() {
            Some(prev) => match profile.get(prev) {
                Some(pc) => (curve_end(pc) - cur_start).magnitude() <= RING_TOLERANCE,
                None => false,
            },
            None => false,
        };
        if joins {
            if let Some(chain) = chains.last_mut() {
                chain.push(i);
            }
        } else {
            chains.push(vec![i]);
        }
    }
    chains
}

/// Whether a chain closes: its first curve's start meets its last curve's end
/// within the ctx tolerance.
fn chain_closed(chain: &[usize], profile: &[Curve]) -> bool {
    let first = match chain.first().and_then(|&i| profile.get(i)) {
        Some(c) => c,
        None => return false,
    };
    let last = match chain.last().and_then(|&i| profile.get(i)) {
        Some(c) => c,
        None => return false,
    };
    (curve_start(first) - curve_end(last)).magnitude() <= RING_TOLERANCE
}

/// Assembles a closed mixed profile loop from analytic and chart segments.
///
/// The segments are flattened in order (each chart curve becomes one
/// `Curve::Line` per PL sample), grouped into endpoint-adjacent chains, and
/// every chain must close within the ctx tolerance — shared endpoints must
/// agree within `64 * TOLERANCE`, and a mismatch (a gap between consecutive
/// segments or an unclosed chain) refuses typed, mirroring the landed
/// `arrange` boundary-contradiction discipline. An empty profile refuses
/// `Refusal::Empty`.
pub fn assemble(segments: &[SketchSegment]) -> std::result::Result<SketchLoop, Refusal> {
    let mut profile: Vec<Curve> = Vec::new();
    for segment in segments {
        match segment {
            SketchSegment::Analytic(c) => profile.push((**c).clone()),
            SketchSegment::Chart(ch) => profile.extend(ch.to_line_curves()),
        }
    }
    if profile.is_empty() {
        return Err(Refusal::Empty);
    }
    for chain in chain_groups(&profile) {
        if !chain_closed(&chain, &profile) {
            return Err(contradiction());
        }
    }
    Ok(SketchLoop { profile })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::arrange::{arrange, Arrangement, ChartCurve};
    use std::f64::consts::{PI, TAU};

    fn p3(x: f64, y: f64) -> Point3 {
        Point3::new(x, y, 0.0)
    }

    fn pt2(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn line(a: Point2, b: Point2) -> Curve {
        Curve::Line(Line(p3(a.x, a.y), p3(b.x, b.y)))
    }

    fn seg_analytic(c: Curve) -> SketchSegment {
        SketchSegment::Analytic(Box::new(c))
    }

    fn seg_chart(c: ChartCurve) -> SketchSegment {
        SketchSegment::Chart(c)
    }

    fn ascending_on(arr: &Arrangement, curve: usize) -> usize {
        arr.half_edges
            .iter()
            .filter(|he| he.curve == curve && he.u_range.0 < he.u_range.1)
            .count()
    }

    #[test]
    fn arc_constructors_refuse_degenerate() {
        // A collinear triple has no circle through it.
        assert!(arc_three_point(pt2(0.0, 0.0), pt2(1.0, 1.0), pt2(2.0, 2.0)).is_err());
        // A coincident triple is degenerate too.
        assert!(arc_three_point(pt2(1.0, 0.0), pt2(1.0, 0.0), pt2(-1.0, 0.0)).is_err());
        // Non-finite input refuses typed.
        assert!(arc_three_point(pt2(0.0, 0.0), pt2(1.0, f64::INFINITY), pt2(2.0, 0.0)).is_err());
        assert!(arc_radius(pt2(f64::NAN, 0.0), 1.0, 0.0, 1.0).is_err());

        // A zero radius refuses typed.
        assert!(arc_radius(pt2(0.0, 0.0), 0.0, 0.0, 1.0).is_err());
        // An inverted range refuses typed.
        assert!(arc_radius(pt2(0.0, 0.0), 1.0, 1.0, 0.5).is_err());
        // A degenerate zero-span or over-tight range refuses typed.
        assert!(arc_radius(pt2(0.0, 0.0), 1.0, 1.0, 1.0).is_err());
        assert!(arc_radius(pt2(0.0, 0.0), 1.0, 0.0, TAU + 0.5).is_err());

        // The positive constructors still author: a CCW unit quarter arc from
        // the +x axis lands on the expected axis points.
        let arc = arc_radius(pt2(1.0, 1.0), 1.0, 0.0, PI / 2.0).unwrap();
        assert_eq!(arc.center(), pt2(1.0, 1.0));
        assert_eq!(arc.radius(), 1.0);
        assert_eq!(arc.range().0, 0.0);
        assert_eq!(arc.start(), pt2(2.0, 1.0));
        let end = arc.end();
        assert!((end - pt2(1.0, 2.0)).magnitude() < 64.0 * TOLERANCE);
        assert!(!arc.is_full());

        // A three-point arc through a right angle starts exactly at `p0`.
        let tri = arc_three_point(pt2(1.0, 0.0), pt2(0.0, 1.0), pt2(-1.0, 0.0)).unwrap();
        assert_eq!(tri.center(), pt2(0.0, 0.0));
        assert_eq!(tri.radius(), 1.0);
        assert_eq!(tri.start(), pt2(1.0, 0.0));
        let tri_end = tri.end();
        assert!((tri_end - pt2(-1.0, 0.0)).magnitude() < 64.0 * TOLERANCE);
    }

    #[test]
    fn arc_line_cells_certify_known_crossings() {
        // Dyadic arc x line: the upper half of the radius-2 circle about (2, 0)
        // crossed by the vertical x = 2. The crossing (2, 2) is exactly
        // dyadic; the full-circle mate (2, -2) is OFF the arc window, so the
        // certified predicate reports exactly the on-arc contact.
        let arc = arc_radius(pt2(2.0, 0.0), 2.0, 0.0, PI).unwrap().to_curve();
        let vertical = line(pt2(2.0, -1.0), pt2(2.0, 3.0));
        let ok = arrange(&[arc, vertical], None).unwrap();
        let arr = &ok.value;
        assert!(
            arr.vertices.iter().any(|v| v.point == p3(2.0, 2.0)),
            "the dyadic arc x line crossing must certify the exact vertex"
        );
        // The arc is split into two ascending sub-arcs at the crossing.
        assert_eq!(ascending_on(arr, 0), 2);

        // Dyadic arc x arc: radius-5 about (0, 0) and radius-4 about (3, 0)
        // cross at the exact dyadic points (3, 4) and (3, -4); the arc windows
        // contain only (3, 4), so exactly that cell certifies.
        let a1 = arc_radius(pt2(0.0, 0.0), 5.0, 0.5, 1.5).unwrap().to_curve();
        let a2 = arc_radius(pt2(3.0, 0.0), 4.0, 1.0, 2.4).unwrap().to_curve();
        let ok = arrange(&[a1, a2], None).unwrap();
        let arr = &ok.value;
        assert!(
            arr.vertices.iter().any(|v| v.point == p3(3.0, 4.0)),
            "the dyadic arc x arc crossing must certify the exact vertex"
        );
        assert_eq!(ascending_on(arr, 0), 2);
        assert_eq!(ascending_on(arr, 1), 2);

        // A NON-dyadic arc x line crossing (x = 0.5 on the same circle: the
        // roots sit at the irrational y = ±sqrt(15)/2) refuses typed.
        let bad_arc = arc_radius(pt2(2.0, 0.0), 2.0, 0.0, PI).unwrap().to_curve();
        let bad_line = line(pt2(0.5, -1.0), pt2(0.5, 3.0));
        assert!(arrange(&[bad_arc, bad_line], None).is_err());

        // A NON-dyadic arc x arc crossing (two radius-2 circles a unit apart:
        // the roots sit at the irrational y = ±sqrt(15)/2) refuses typed.
        let bad1 = arc_radius(pt2(0.0, 0.0), 2.0, 0.5, 2.0).unwrap().to_curve();
        let bad2 = arc_radius(pt2(1.0, 0.0), 2.0, 0.8, 2.5).unwrap().to_curve();
        assert!(arrange(&[bad1, bad2], None).is_err());
    }

    #[test]
    fn spline_authoring_periodic_and_open() {
        // Six distinct points, periodic: the ring closes EXACTLY (the first
        // point is appended as the seam sample) and passes through every
        // input point exactly.
        let pts: Vec<Point2> = vec![
            pt2(0.0, 0.0),
            pt2(2.0, 0.0),
            pt2(3.0, 1.0),
            pt2(2.0, 2.0),
            pt2(0.0, 2.0),
            pt2(-1.0, 1.0),
        ];
        let ring = spline(&pts, true).unwrap();
        assert!(ring.is_closed());
        let vertices = ring.vertices();
        assert_eq!(vertices.len(), 7);
        assert_eq!(
            vertices.first().copied().unwrap(),
            pts.first().copied().unwrap()
        );
        assert_eq!(
            vertices.last().copied().unwrap(),
            pts.first().copied().unwrap()
        );
        for p in &pts {
            assert!(
                vertices.contains(p),
                "a periodic spline must pass through every authored point exactly"
            );
        }

        // Open authoring produces an OPEN carrier over the given points.
        let open = spline(&[pt2(0.0, 0.0), pt2(2.0, 0.0), pt2(2.0, 2.0)], false).unwrap();
        assert!(!open.is_closed());
        assert_eq!(open.vertices().len(), 3);

        // An open spline cannot author a loop: a point list whose first and
        // last samples coincide refuses typed.
        assert!(spline(&[pt2(0.0, 0.0), pt2(2.0, 0.0), pt2(0.0, 0.0)], false).is_err());
        // Too few points refuse typed in both modes.
        assert!(spline(&[pt2(0.0, 0.0)], false).is_err());
        assert!(spline(&[], true).is_err());

        // Non-finite points refuse typed in both modes.
        assert!(spline(&[pt2(0.0, 0.0), pt2(f64::NAN, 1.0), pt2(2.0, 0.0)], true).is_err());
        assert!(spline(
            &[pt2(0.0, 0.0), pt2(f64::INFINITY, 1.0), pt2(2.0, 0.0)],
            false,
        )
        .is_err());
    }

    #[test]
    fn mixed_loop_profile_assembles() {
        // A closed mixed loop: bottom line, a quarter arc (rounded corner),
        // a top line, and an open spline returning down the left edge. All
        // shared endpoints are exact (the arc's t0 = 0 start and its t1 = pi/2
        // end are exactly representable), so the flattened profile assembles
        // and the landed `arrange` subdivides it.
        let bottom = line(pt2(0.0, 0.0), pt2(4.0, 0.0));
        let corner = arc_radius(pt2(2.0, 0.0), 2.0, 0.0, PI / 2.0).unwrap();
        let corner_end = corner.end();
        let top = line(corner_end, pt2(0.0, 2.0));
        let left = spline(&[pt2(0.0, 2.0), pt2(0.0, 0.0)], false).unwrap();
        let hole = arc_radius(pt2(2.0, 1.0), 0.5, 0.0, TAU).unwrap().to_curve();

        let loop_ = assemble(&[
            seg_analytic(bottom),
            seg_analytic(corner.to_curve()),
            seg_analytic(top),
            seg_chart(left),
            seg_analytic(hole),
        ])
        .unwrap();

        // The landed `arrange` accepts the flat profile and yields the known
        // ground truth: an outer mixed ring with an interior hole -> the
        // plate, the hole, and the exterior = three regions.
        let ok = arrange(loop_.profile(), None).unwrap();
        let arr = &ok.value;
        assert_eq!(arr.regions.len(), 3);

        let exterior = arr.regions.iter().find(|r| !r.bounded).unwrap();
        assert_eq!(exterior.winding, 0);
        let plate = arr
            .regions
            .iter()
            .find(|r| r.bounded && r.boundaries.len() == 2)
            .unwrap();
        assert!(plate.winding == 1 || plate.winding == -1);
        let hole_r = arr
            .regions
            .iter()
            .find(|r| r.bounded && r.boundaries.len() == 1)
            .unwrap();
        assert!(hole_r.winding == 1 || hole_r.winding == -1);
        // The ring's joints are all present as exact vertices.
        for v in [pt2(0.0, 0.0), pt2(4.0, 0.0), corner_end, pt2(0.0, 2.0)] {
            assert!(
                arr.vertices
                    .iter()
                    .any(|vertex| vertex.point == p3(v.x, v.y)),
                "the mixed loop's shared endpoints must appear as exact vertices"
            );
        }

        // A mismatched ring (a gap between consecutive segments) refuses typed
        // at assembly: the shared endpoint does not agree within the ctx
        // tolerance.
        let broken = assemble(&[
            seg_analytic(line(pt2(0.0, 0.0), pt2(4.0, 0.0))),
            seg_analytic(line(pt2(5.0, 0.0), pt2(5.0, 4.0))),
        ]);
        assert!(broken.is_err());
        // An unclosed open chain refuses typed too.
        let unclosed = assemble(&[
            seg_analytic(line(pt2(0.0, 0.0), pt2(4.0, 0.0))),
            seg_analytic(line(pt2(4.0, 0.0), pt2(4.0, 4.0))),
        ]);
        assert!(unclosed.is_err());
    }
}
