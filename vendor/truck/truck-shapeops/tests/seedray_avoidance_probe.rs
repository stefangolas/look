//! DEF-SEEDRAY-C probe — measured evidence for the certified seed-ray
//! edge/vertex avoidance criterion (docs/AUDIT_SEEDRAY_AVOIDANCE.md).
//!
//! This is a MEASUREMENT probe, not the criterion: DEF-SEEDRAY-B owns the
//! production implementation in `boolean/classify.rs`. Everything here runs
//! under `#[ignore]` (on demand, `-- --ignored --nocapture`) and prints the
//! census tables the audit document cites.
//!
//! Methodology (stated precisely so the numbers are reproducible):
//!
//! - Corpus = the classify.rs ray-seeded fixtures (`boolean/classify.rs` tests
//!   2, 3, 4 — disjoint, contained, ambiguous-direction) plus probe-constructed
//!   near configurations over the SAME shells. Contact-free fixtures are the
//!   corpus because only their components take rule-(b) ray seeds; components
//!   touching a contact arc seed by the arc-side rule and never cast a ray.
//! - A "seed" is a rule-(b) representative point on one solid, measured
//!   against the OTHER solid's boundary (`dist(ray, edges ∪ vertices of ∂other)`
//!   over the whole half-line t >= 0 — the parity ray is infinite).
//! - Edge curves are Lines and Circles only in this corpus. Distance to a Line
//!   edge is exact point-set distance between the ray half-line and the
//!   segment; distance to a Circle edge is the ray-to-segment distance over a
//!   uniform tessellation (sagitta error <= r·(2π/N)²/8, disclosed, N=2048).
//! - Criterion (b) ("sign-stability of (C(t) − p) × d over the edge parameter
//!   box") is evaluated EXACTLY for Line and Circle edges (no interval lib):
//!   for a Line the three components of g(t) = (C(t) − p) × d are affine in t
//!   over any sub-box, so their ranges are the endpoint values; for a Circle,
//!   C(t) = c + cos(t)·e1 + sin(t)·e2 (columns of the Processor transform), so
//!   each component is a + b·cos(t) + d·sin(t) whose range over a sub-interval
//!   is closed-form (amplitude + interior critical points). A sub-box is
//!   EXCLUDED when some component's range strictly avoids 0; subdivision
//!   continues otherwise, depth-capped (leaf width = (t1−t0)·2⁻²⁴). A
//!   direction is criterion-(b)-refused when some Line/Circle edge of the
//!   target has a non-excludable leaf. Refusal leaves report whether the
//!   near/crossing point lies ahead of (t_c > 0) or behind (t_c < 0) the seed
//!   origin — the half-line caveat.
//!
//! All δ thresholds: δ_abs = 1.0e-2 (the classifier tolerance class, H-3,
//! dimensionless on the unit-scale witnesses) and, where the scale-relative
//! question (H-3) matters, δ_rel = 1.0e-3 · scale (scale = target bounding-box
//! diagonal). Numbers below are deterministic on a fixed build.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};
use std::time::Instant;

use truck_base::cgmath64::{InnerSpace, Matrix4, Point2, Point3, Vector3, Vector4};
use truck_geometry::arrange::{arrange, Arrangement};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::prelude::*;
use truck_geotrait::{BoundedCurve, ParametricCurve};
use truck_modeling::extrude::extrude_profile;
use truck_shapeops::boolean::classify::classify_fragments;
use truck_shapeops::boolean::split::split_fragments;
use truck_topology::{Edge, EdgeID, Face, Shell, Vertex, Wire};

/// The insertion/classification tolerance class (H-3; matches the fixtures).
const TOL: f64 = 1.0e-2;

/// The absolute δ for "near" (edge/vertex distance below this is a hit).
const DELTA_ABS: f64 = 1.0e-2;

/// The scale-relative δ multiplier (H-3 question: absolute vs scale-relative).
const DELTA_REL_MULT: f64 = 1.0e-3;

/// Circle tessellation for the distance census (sagitta error disclosed).
const CIRCLE_SEGMENTS: usize = 2048;

/// Criterion-(b) recursion depth cap.
const B_MAX_DEPTH: u32 = 24;

/// Exclusion padding: a sub-box is excluded only when some g component's
/// every sample stays this far from 0. Absorbs float error at exact zeros
/// (e.g. `cos(pi/2)` is not 0 in f64) so a box containing a true root is
/// never falsely excluded; it also makes the criterion refuse ultra-tight
/// near-tangent misses (dist < ~1e-12) instead of certifying them.
const B_EPS: f64 = 1.0e-12;

/// The near-window shown by the verbose per-direction table.
const NEAR_WINDOW: f64 = 1.0e-1;

type Sh = Shell<Point3, Curve, Surface>;
type FACE = Face<Point3, Curve, Surface>;

// ---------------------------------------------------------------------------
// fixture builders (verbatim from classify.rs's test module; in-crate, proven)
// ---------------------------------------------------------------------------

/// A placed full-period circle at `center` with radius `r`.
fn placed_circle(center: Point3, r: f64) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(center.x, center.y, center.z, 1.0),
        },
    )
}

/// The 4x4 block profile: four `Curve::Line`s, CCW.
fn block_profile() -> (Vec<Curve>, Arrangement) {
    let profile = vec![
        Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
        Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
    ];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// A pure-disk profile: one full circle of radius `r` at `center`.
fn disk_profile(center: Point2, r: f64) -> (Vec<Curve>, Arrangement) {
    let circle = Curve::Circle(placed_circle(Point3::new(center.x, center.y, 0.0), r));
    let profile = vec![circle];
    let ok = arrange(&profile, None).unwrap();
    (profile, ok.value)
}

/// The shell of the `height`-extrude of a profile.
fn extrude_shell(profile: &[Curve], arr: &Arrangement, height: f64) -> Sh {
    let solid = extrude_profile(profile, arr, height).unwrap().value;
    solid.boundaries().first().unwrap().clone()
}

/// The 4x4 block shell, z in [0, 2].
fn block_shell() -> Sh {
    let (profile, arr) = block_profile();
    extrude_shell(&profile, &arr, 2.0)
}

/// The extrude of the disk profile (caps at z = 0 and z = h).
fn disk_column(center: Point2, r: f64, h: f64) -> Sh {
    let (profile, arr) = disk_profile(center, r);
    extrude_shell(&profile, &arr, h)
}

/// The hand-built raised-disk solid (circle self-loop edges SHARED between the
/// caps and the wall).
fn raised_disk(center: Point2, r: f64, z_lo: f64, z_hi: f64) -> Sh {
    let bottom_center = Point3::new(center.x, center.y, z_lo);
    let top_center = Point3::new(center.x, center.y, z_hi);
    let bottom_circle = placed_circle(bottom_center, r);
    let top_circle = placed_circle(top_center, r);
    let v0 = Vertex::new(bottom_circle.subs(0.0));
    let v1 = Vertex::new(top_circle.subs(0.0));
    let bottom_edge = Edge::new_unchecked(&v0, &v0, Curve::Circle(bottom_circle));
    let top_edge = Edge::new_unchecked(&v1, &v1, Curve::Circle(top_circle));

    let bottom_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, z_lo),
        Point3::new(1.0, 0.0, z_lo),
        Point3::new(0.0, 1.0, z_lo),
    ));
    let mut bottom_cap =
        FACE::try_new(vec![Wire::from(vec![bottom_edge.clone()])], bottom_surface).unwrap();
    bottom_cap.invert();

    let top_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, z_hi),
        Point3::new(1.0, 0.0, z_hi),
        Point3::new(0.0, 1.0, z_hi),
    ));
    let top_cap = FACE::try_new(vec![Wire::from(vec![top_edge.clone()])], top_surface).unwrap();

    let cyl = Cylinder::new(Point3::new(center.x, center.y, 0.0), r)
        .unwrap()
        .value;
    let wall = FACE::try_new(
        vec![
            Wire::from(vec![bottom_edge]),
            Wire::from(vec![top_edge.inverse()]),
        ],
        Surface::Cylinder(cyl),
    )
    .unwrap();

    vec![bottom_cap, top_cap, wall].into()
}

/// The fourteen deterministic ray-seed directions (classify.rs verbatim).
fn ray_directions() -> [Vector3; 14] {
    let s = 1.0 / 3.0_f64.sqrt();
    [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(s, s, s),
        Vector3::new(s, s, -s),
        Vector3::new(s, -s, s),
        Vector3::new(s, -s, -s),
        Vector3::new(-s, s, s),
        Vector3::new(-s, s, -s),
        Vector3::new(-s, -s, s),
        Vector3::new(-s, -s, -s),
    ]
}

/// Short direction labels, index-aligned with [`ray_directions`].
fn dir_label(i: usize, d: &Vector3) -> String {
    let v = match i {
        0 => "+z",
        1 => "+x",
        2 => "+y",
        3 => "-z",
        4 => "-x",
        5 => "-y",
        _ => "diag",
    };
    format!(
        "{}({},{},{})",
        v,
        fmt_scalar(d.x),
        fmt_scalar(d.y),
        fmt_scalar(d.z)
    )
}

fn fmt_scalar(x: f64) -> String {
    if (x - x.round()).abs() < 1.0e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{:.4}", x)
    }
}

fn fmt_vec3(p: Point3) -> String {
    format!("({:.6},{:.6},{:.6})", p.x, p.y, p.z)
}

fn fmt_dist(x: f64) -> String {
    format!("{:.6e}", x)
}

// ---------------------------------------------------------------------------
// ray geometry primitives
// ---------------------------------------------------------------------------

/// The distance from the ray half-line {p + t·d : t >= 0}, d unit, to a point.
fn ray_point_dist(p: Point3, d: Vector3, q: Point3) -> f64 {
    let e = q - p;
    let c = d.dot(e);
    if c <= 0.0 {
        e.magnitude()
    } else {
        (e - d * c).magnitude()
    }
}

/// The point-set distance between the ray half-line {p + t·d, t >= 0}, d unit,
/// and the segment a..b. Solved as the convex QP over the rectangle
/// t >= 0, s in [0,1]; the unconstrained optimum is checked for feasibility
/// and otherwise the three boundary minimisations are evaluated.
fn ray_segment_dist(p: Point3, d: Vector3, a: Point3, b: Point3) -> f64 {
    let r0 = p - a;
    let w = b - a;
    let aa = d.dot(d);
    let bb = d.dot(w);
    let cc = w.dot(w);
    let g0 = d.dot(r0);
    let h0 = w.dot(r0);
    // Interior candidate: F(t,s) = r0 + t·d − s·w.
    let mut best: f64 = f64::INFINITY;
    let det = bb * bb - aa * cc;
    if det.abs() > 1.0e-30 {
        let t = (g0 * cc - bb * h0) / det;
        let s = (bb * g0 - aa * h0) / det;
        if t >= 0.0 && (0.0..=1.0).contains(&s) {
            let f = r0 + d * t - w * s;
            best = best.min(f.dot(f));
        }
    }
    // Boundary: s = 0 (point a vs the ray).
    best = best.min(ray_point_dist(p, d, a).powi(2));
    // Boundary: s = 1 (point b vs the ray).
    best = best.min(ray_point_dist(p, d, b).powi(2));
    // Boundary: t = 0 (point p vs the segment). Standard point-segment.
    let e = a - p;
    let denom = cc;
    let s = if denom.abs() > 1.0e-30 {
        (-e.dot(w)) / denom
    } else {
        0.0
    };
    let s = s.clamp(0.0, 1.0);
    let close = a + w * s - p;
    best = best.min(close.dot(close));
    best.sqrt()
}

// ---------------------------------------------------------------------------
// target geometry (distinct topological edges of one shell, deduped by id)
// ---------------------------------------------------------------------------

/// One distinct topological edge of the target, ready for both measurement
/// paths: a segment list (distance census) and the curve + analytic metadata
/// (criterion (b)).
struct EdgeGeom {
    curve: Curve,
    segs: Vec<(Point3, Point3)>,
}

struct TargetGeom {
    edges: Vec<EdgeGeom>,
    /// Curve-kind census: (lines, circles, other).
    kinds: (usize, usize, usize),
    vertices: Vec<Point3>,
    /// The axis-aligned bounding-box diagonal (scale for δ_rel).
    scale: f64,
}

/// Sample an arbitrary curve over its domain as `n` consecutive segments.
fn sample_segments(curve: &Curve, n: usize) -> Vec<(Point3, Point3)> {
    let (t0, t1) = curve.range_tuple();
    (0..n)
        .map(|i| {
            let a = curve.subs(t0 + (t1 - t0) * (i as f64) / (n as f64));
            let b = curve.subs(t0 + (t1 - t0) * ((i + 1) as f64) / (n as f64));
            (a, b)
        })
        .collect()
}

fn build_target(shell: &Sh) -> TargetGeom {
    let mut seen: HashSet<EdgeID<Curve>> = HashSet::new();
    let mut edges: Vec<EdgeGeom> = Vec::new();
    let mut vertices: Vec<Point3> = Vec::new();
    let mut kinds = (0usize, 0usize, 0usize);
    for face in shell.face_iter() {
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let id = edge.id();
                if !seen.insert(id) {
                    continue;
                }
                let curve = edge.curve();
                let (t0, t1) = curve.range_tuple();
                vertices.push(curve.subs(t0));
                vertices.push(curve.subs(t1));
                match &curve {
                    Curve::Line(_) => {
                        kinds.0 += 1;
                        edges.push(EdgeGeom {
                            segs: vec![(curve.subs(t0), curve.subs(t1))],
                            curve,
                        });
                    }
                    Curve::Circle(_) => {
                        kinds.1 += 1;
                        edges.push(EdgeGeom {
                            segs: sample_segments(&curve, CIRCLE_SEGMENTS),
                            curve,
                        });
                    }
                    _ => {
                        kinds.2 += 1;
                        edges.push(EdgeGeom {
                            segs: sample_segments(&curve, CIRCLE_SEGMENTS),
                            curve,
                        });
                    }
                }
            }
        }
    }
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for e in &edges {
        for (a, b) in &e.segs {
            for pt in [a, b] {
                lo.x = lo.x.min(pt.x);
                lo.y = lo.y.min(pt.y);
                lo.z = lo.z.min(pt.z);
                hi.x = hi.x.max(pt.x);
                hi.y = hi.y.max(pt.y);
                hi.z = hi.z.max(pt.z);
            }
        }
    }
    let scale = (hi - lo).magnitude();
    TargetGeom {
        edges,
        kinds,
        vertices,
        scale,
    }
}

// ---------------------------------------------------------------------------
// criterion (b): exact sign-stability exclusion on Line / Circle edges
// ---------------------------------------------------------------------------

/// The k-th coordinate of a vector (criterion-(b) component access).
fn comp_at(v: Vector3, k: usize) -> f64 {
    match k {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

/// Componentwise min/max of g(u) = (C(u) − p) × d over the candidate
/// parameters. For a Line the endpoints suffice (affine); for a Circle the
/// candidate set carries the interior stationary points, so the samples bound
/// the true ranges up to the float phase error (guarded by `B_EPS`).
fn sample_ranges(curve: &Curve, p: Point3, d: Vector3, cands: &[f64]) -> [(f64, f64); 3] {
    let mut mn = [f64::INFINITY; 3];
    let mut mx = [f64::NEG_INFINITY; 3];
    for &u in cands {
        let g = (curve.subs(u) - p).cross(d);
        for k in 0..3 {
            let v = comp_at(g, k);
            mn[k] = mn[k].min(v);
            mx[k] = mx[k].max(v);
        }
    }
    [(mn[0], mx[0]), (mn[1], mx[1]), (mn[2], mx[2])]
}

/// The analytic circle model: C(t) = c + cos(w)·e1 + sin(w)·e2 over the
/// Processor's parameter domain, w the orientation-corrected angle.
struct CircleModel {
    c: Point3,
    e1: Vector3,
    e2: Vector3,
    flip: bool,
}

fn circle_model(curve: &Curve) -> Option<CircleModel> {
    match curve {
        Curve::Circle(proc) => {
            let m = proc.transform();
            let c = Point3::new(m.w.x, m.w.y, m.w.z);
            let e1 = Vector3::new(m.x.x, m.x.y, m.x.z);
            let e2 = Vector3::new(m.y.x, m.y.y, m.y.z);
            let flip = !proc.orientation();
            Some(CircleModel { c, e1, e2, flip })
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeVerdict {
    /// The whole edge's carrier is excluded (the ray LINE avoids it).
    Clear,
    /// A non-excludable leaf was reached: refuse under criterion (b).
    Refuse,
}

struct BLeafStats {
    leaves: usize,
    behind: usize,
}

/// Criterion (b) verdict over ONE analytic (Line/Circle) edge.
///
/// `unresolved` gathers, for refused edges, the count of non-excludable leaves
/// and how many lie behind the seed origin (t_c < 0) — the half-line caveat.
fn edge_b_verdict(
    curve: &Curve,
    p: Point3,
    d: Vector3,
    unresolved: &mut BLeafStats,
) -> EdgeVerdict {
    let (t0, t1) = curve.range_tuple();
    let domain_w = t1 - t0;
    // Per-component g = (C − p) × d.
    let coeff: Option<(Vector3, Vector3, Vector3)> = if let Curve::Line(_) = curve {
        None
    } else if let Some(model) = circle_model(curve) {
        Some(((model.c - p).cross(d), model.e1.cross(d), model.e2.cross(d)))
    } else {
        // Not an analytic (Line/Circle) edge: not measured by criterion (b).
        return EdgeVerdict::Clear;
    };

    // Orientation-corrected angle bookkeeping (criterion (b) models circles as
    // C(t) = c + cos(w)·e1 + sin(w)·e2 with w = t or w = t0 + t1 − t).
    let flip = coeff_is_flipped(curve);
    let from_w = |w: f64| if flip { t0 + t1 - w } else { w };

    // Returns whether the box [lo, hi] of the curve parameter is excluded.
    let box_excluded = |lo: f64, hi: f64| -> bool {
        let ranges: [(f64, f64); 3] = if curve_is_line(curve) {
            // Affine: g's components take their extrema at the endpoints.
            sample_ranges(curve, p, d, &[lo, hi])
        } else {
            // Circle: each g component is a sinusoid in the angle, so its range
            // over the box is attained at the endpoints or at interior
            // stationary points. Sample the actual curve there (no closed-form
            // reconstruction, which loses precision at exact zeros) plus the
            // box midpoint as a phase-float guard.
            let (_a, bv, dv) = coeff.unwrap();
            let mut cands: Vec<f64> = vec![lo, hi, 0.5 * (lo + hi)];
            let wlo = if flip { t0 + t1 - hi } else { lo };
            let whi = if flip { t0 + t1 - lo } else { hi };
            for k in 0..3 {
                let (bb, dd) = (comp_at(bv, k), comp_at(dv, k));
                if bb * bb + dd * dd <= 1.0e-30 {
                    continue;
                }
                let phi = dd.atan2(bb);
                let m0 = (((wlo - phi) / PI).floor() - 1.0) as i64;
                for m in m0..m0 + 4 {
                    let w = phi + (m as f64) * PI;
                    if w > wlo && w < whi {
                        cands.push(from_w(w));
                    }
                }
            }
            sample_ranges(curve, p, d, &cands)
        };
        ranges.iter().any(|(l, h)| *l > B_EPS || *h < -B_EPS)
    };

    let mut stack: Vec<(f64, f64, u32)> = vec![(t0, t1, 0)];
    let mut refused = false;
    // Coincident-carrier safety: a ray line that overlaps an edge carrier
    // never excludes; cap the boxes visited so the probe cannot runaway.
    let mut visited = 0usize;
    while let Some((lo, hi, depth)) = stack.pop() {
        visited += 1;
        if visited > 8192 {
            refused = true;
            break;
        }
        if box_excluded(lo, hi) {
            continue;
        }
        let width = hi - lo;
        if depth >= B_MAX_DEPTH || width <= domain_w * 2.0f64.powi(-(B_MAX_DEPTH as i32)) {
            refused = true;
            unresolved.leaves += 1;
            let u_mid = 0.5 * (lo + hi);
            let q = curve.subs(u_mid);
            let tc = d.dot(q - p);
            if tc < 0.0 {
                unresolved.behind += 1;
            }
            continue;
        }
        let mid = 0.5 * (lo + hi);
        stack.push((mid, hi, depth + 1));
        stack.push((lo, mid, depth + 1));
    }
    if refused {
        EdgeVerdict::Refuse
    } else {
        EdgeVerdict::Clear
    }
}

fn curve_is_line(curve: &Curve) -> bool {
    matches!(curve, Curve::Line(_))
}

fn coeff_is_flipped(curve: &Curve) -> bool {
    match circle_model(curve) {
        Some(m) => m.flip,
        None => false,
    }
}

// ---------------------------------------------------------------------------
// per-seed-direction measurements
// ---------------------------------------------------------------------------

struct DirMeasure {
    dir_index: usize,
    edge_dist: f64,
    vertex_dist: f64,
    /// min distance to any edge or vertex of the target.
    min_dist: f64,
    b_clear: bool,
    b_leaves: usize,
    b_behind: usize,
}

/// One full 14-direction measurement of a seed against a target.
struct SeedMeasure {
    trial: String,
    seed_label: String,
    origin: Point3,
    target_desc: String,
    dirs: Vec<DirMeasure>,
}

/// The seed's own origins must not be inside the target (rule-(b) pre-screen).
fn measure_seed(trial: &str, seed_label: &str, origin: Point3, target: &TargetGeom) -> SeedMeasure {
    let dirs = ray_directions();
    let mut out = Vec::with_capacity(14);
    for (i, d) in dirs.iter().enumerate() {
        let mut edge_dist = f64::INFINITY;
        for e in &target.edges {
            for (a, b) in &e.segs {
                edge_dist = edge_dist.min(ray_segment_dist(origin, *d, *a, *b));
            }
        }
        let mut vertex_dist = f64::INFINITY;
        for v in &target.vertices {
            vertex_dist = vertex_dist.min(ray_point_dist(origin, *d, *v));
        }
        let mut stats = BLeafStats {
            leaves: 0,
            behind: 0,
        };
        let mut b_clear = true;
        for e in &target.edges {
            if edge_b_verdict(&e.curve, origin, *d, &mut stats) == EdgeVerdict::Refuse {
                b_clear = false;
            }
        }
        let min_dist = edge_dist.min(vertex_dist);
        out.push(DirMeasure {
            dir_index: i,
            edge_dist,
            vertex_dist,
            min_dist,
            b_clear,
            b_leaves: stats.leaves,
            b_behind: stats.behind,
        });
    }
    SeedMeasure {
        trial: trial.to_string(),
        seed_label: seed_label.to_string(),
        origin,
        target_desc: target_desc_label(target),
        dirs: out,
    }
}

fn target_desc_label(t: &TargetGeom) -> String {
    format!(
        "edges(L={},C={},O={}) verts={} scale={:.4}",
        t.kinds.0,
        t.kinds.1,
        t.kinds.2,
        t.vertices.len(),
        t.scale
    )
}

// ---------------------------------------------------------------------------
// the #[ignore] probe tests
// ---------------------------------------------------------------------------

/// Fixture green-register: run the real splitter + classifier on each corpus
/// pair to record that the fixtures are green today (rule-(b) seeds resolve).
#[test]
#[ignore]
fn probe_fixture_green_register() {
    println!();
    println!("== DEF-SEEDRAY-C PROBE: fixture green register ==");
    println!(
        "== run: cargo test -p truck-shapeops --test seedray_avoidance_probe -- --ignored --nocapture"
    );
    println!();
    println!(
        "{:<28} {:>12} {:>12} {:>12} {:>16}",
        "fixture-pair", "faces", "fragments", "split-ok", "classify(a,b)"
    );
    let a_block = block_shell();
    let pairs: Vec<(&str, Sh, Sh)> = vec![
        (
            "disjoint",
            a_block.clone(),
            disk_column(Point2::new(6.0, 6.0), 1.0, 2.0),
        ),
        (
            "contained",
            a_block.clone(),
            raised_disk(Point2::new(2.0, 2.0), 1.0, 0.5, 1.5),
        ),
        (
            "ambiguous",
            a_block.clone(),
            raised_disk(Point2::new(2.5, 2.0), 0.5, 0.5, 1.5),
        ),
    ];
    let a_faces = a_block.face_iter().count();
    for (name, a, b) in &pairs {
        let b_faces = b.face_iter().count();
        let split = split_fragments(a, b, &[], TOL);
        let (frags, green) = match &split {
            Ok(c) => {
                let mesh = &c.value;
                let green = classify_fragments(a, b, mesh, TOL).is_ok();
                (mesh.fragments.len(), green)
            }
            Err(_) => (0, false),
        };
        println!(
            "{:<28} {:>12} {:>12} {:>12} {:>16}",
            name,
            a_faces + b_faces,
            frags,
            split.is_ok(),
            green
        );
    }
    println!();
}

/// T-1: the direction-table hit profile. One row per table direction: over all
/// seed-direction decisions in the corpus, how many hit a near edge / vertex
/// (δ_abs) and how many criterion-(b) would refuse.
#[test]
#[ignore]
fn probe_direction_hit_profile() {
    let seeds = corpus_seeds();
    let dirs = ray_directions();
    println!();
    println!("== T-1 direction-table hit profile ==");
    println!(
        "corpus: {} seed-direction decisions over {} seeds (all 14 directions each)",
        seeds.iter().map(|s| s.dirs.len()).sum::<usize>(),
        seeds.len()
    );
    println!(
        "near thresholds: delta_abs = {:.3e}; rows per table direction",
        DELTA_ABS
    );
    println!();
    println!(
        "{:<6} {:<22} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "dir", "vector", "tested", "edge_hit", "vert_hit", "near", "b_refuse", "b_behind"
    );
    let mut totals = [0usize; 6];
    for (i, d) in dirs.iter().enumerate() {
        let mut tested = 0usize;
        let mut edge_hit = 0usize;
        let mut vert_hit = 0usize;
        let mut near = 0usize;
        let mut b_refuse = 0usize;
        let mut b_behind = 0usize;
        for s in &seeds {
            let m = &s.dirs[i];
            tested += 1;
            if m.edge_dist < DELTA_ABS {
                edge_hit += 1;
            }
            if m.vertex_dist < DELTA_ABS {
                vert_hit += 1;
            }
            if m.min_dist < DELTA_ABS {
                near += 1;
            }
            if !m.b_clear {
                b_refuse += 1;
                b_behind += (m.b_behind > 0) as usize;
            }
        }
        println!(
            "{:<6} {:<22} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
            i,
            dir_label(i, d),
            tested,
            edge_hit,
            vert_hit,
            near,
            b_refuse,
            b_behind
        );
        totals[0] += tested;
        totals[1] += edge_hit;
        totals[2] += vert_hit;
        totals[3] += near;
        totals[4] += b_refuse;
        totals[5] += b_behind;
    }
    println!(
        "{:<6} {:<22} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "TOTAL", "", totals[0], totals[1], totals[2], totals[3], totals[4], totals[5]
    );
    println!();
}

/// T-2: criterion-(b) refuse-rate and doctrine-floor inputs, per trial.
#[test]
#[ignore]
fn probe_criterion_b_refuse_rate() {
    let seeds = corpus_seeds();
    println!();
    println!("== T-2 criterion (b) refuse-rate and certified-direction availability ==");
    println!("criterion (b): sign-stability exclusion on Line/Circle edges, depth cap 24");
    println!();
    println!(
        "{:<28} {:<10} {:>12} {:>10} {:>10} {:>12} {:>12}",
        "trial.seed", "target", "decisions", "b_refuse", "b_clear", "census_near", "clear_dirs"
    );
    let mut agg_refuse = 0usize;
    let mut agg_total = 0usize;
    let mut agg_near = 0usize;
    for s in &seeds {
        let decisions = s.dirs.len();
        let b_refuse = s.dirs.iter().filter(|m| !m.b_clear).count();
        let b_clear = decisions - b_refuse;
        let near = s.dirs.iter().filter(|m| m.min_dist < DELTA_ABS).count();
        // "clear_dirs" = directions that are BOTH criterion-(b) certified and
        // census-clear at delta_abs: the certified path's direction budget.
        let clear_dirs = s
            .dirs
            .iter()
            .filter(|m| m.b_clear && m.min_dist >= DELTA_ABS)
            .count();
        agg_refuse += b_refuse;
        agg_total += decisions;
        agg_near += near;
        let short = short_trial_seed(&s.trial, &s.seed_label);
        println!(
            "{:<28} {:<10} {:>12} {:>10} {:>10} {:>12} {:>12}",
            short,
            s.target_desc.split(' ').next().unwrap_or(""),
            decisions,
            b_refuse,
            b_clear,
            near,
            clear_dirs
        );
    }
    println!();
    println!(
        "aggregate: {}/{} decisions criterion-(b)-refused ({:.3}); census-near {}/{} ({:.3})",
        agg_refuse,
        agg_total,
        agg_refuse as f64 / agg_total as f64,
        agg_near,
        agg_total,
        agg_near as f64 / agg_total as f64
    );
    println!();
}

fn short_trial_seed(trial: &str, seed: &str) -> String {
    // trial already short; seed labels are short.
    let seed_short = seed.replace("origin=", "");
    format!("{}.{}", trial, seed_short)
}

/// T-3: verbose per-direction rows inside the near-window for the ambiguous /
/// vertex / eps trials (rows whose min distance < 1e-1 or criterion-(b)
/// refuses).
#[test]
#[ignore]
fn probe_near_direction_rows() {
    let seeds = corpus_seeds();
    println!();
    println!("== T-3 per-direction rows in the near window ==");
    println!(
        "rows shown when min(edge,vertex) < {:.0e} or criterion-(b) refuses; all 14 directions measured",
        NEAR_WINDOW
    );
    println!();
    for s in &seeds {
        let mut printed = false;
        for m in &s.dirs {
            if m.min_dist >= NEAR_WINDOW && m.b_clear {
                continue;
            }
            if !printed {
                printed = true;
                println!(
                    "trial={} seed={} origin={} target[{}]",
                    s.trial,
                    s.seed_label,
                    fmt_vec3(s.origin),
                    s.target_desc
                );
                println!(
                    "{:<6} {:<24} {:>12} {:>12} {:>12} {:>10} {:>10} {:>10}",
                    "dir",
                    "vector",
                    "edge_dist",
                    "vert_dist",
                    "min_dist",
                    "b_clear",
                    "b_leaves",
                    "b_behind"
                );
            }
            println!(
                "{:<6} {:<24} {:>12} {:>12} {:>12} {:>10} {:>10} {:>10}",
                m.dir_index,
                dir_label(m.dir_index, &ray_directions()[m.dir_index]),
                fmt_dist(m.edge_dist),
                fmt_dist(m.vertex_dist),
                fmt_dist(m.min_dist),
                m.b_clear,
                m.b_leaves,
                m.b_behind
            );
        }
        if printed {
            println!();
        }
    }
}

/// T-4: measured runtime overhead per seed decision.
#[test]
#[ignore]
fn probe_runtime_overhead() {
    let a_block = block_shell();
    let disk = disk_column(Point2::new(6.0, 6.0), 1.0, 2.0);
    let t_block = build_target(&a_block);
    let t_disk = build_target(&disk);
    let p = Point3::new(2.0, 2.0, 0.0);
    let d = ray_directions()[0];
    println!();
    println!("== T-4 runtime overhead per seed decision (this run) ==");
    println!("note: debug build under the Done command; timings indicative, not release");

    // Baseline float path (what classify does per decision today): analytic
    // ray x carrier solves are internal to classify.rs; the probe's nearest
    // reproducible float analogue is the per-direction distance census QP.
    let reps_census = 60usize;
    let start = Instant::now();
    let mut sink = 0.0f64;
    for _ in 0..reps_census {
        for e in &t_disk.edges {
            for (a, b) in &e.segs {
                sink += ray_segment_dist(p, d, *a, *b);
            }
        }
    }
    let census_secs = start.elapsed().as_secs_f64() / (reps_census as f64);
    let segs = t_disk.edges.iter().map(|e| e.segs.len()).sum::<usize>();
    let per_seg_us = census_secs * 1.0e6 / (segs as f64);
    println!(
        "distance census (disk target, {} circle segments): {:.3} us per ray-segment call; {:.3} us per seed-direction decision",
        segs,
        per_seg_us,
        census_secs * 1.0e6
    );
    println!("(sink guard: {:.6e})", sink);

    // Criterion (b) per seed-direction decision on the block target (12 Line
    // edges) and the disk target (2 Circle edges).
    let mut stats = BLeafStats {
        leaves: 0,
        behind: 0,
    };
    let reps_b_block = 4000usize;
    let start = Instant::now();
    for _ in 0..reps_b_block {
        for e in &t_block.edges {
            let _ = edge_b_verdict(&e.curve, p, d, &mut stats);
        }
    }
    let b_block_us = start.elapsed().as_secs_f64() * 1.0e6
        / (reps_b_block as f64)
        / (t_block.edges.len() as f64);
    println!(
        "criterion (b): {:.3} us per Line-edge verdict; {:.3} us per seed-direction over the 12-edge block target",
        b_block_us,
        b_block_us * (t_block.edges.len() as f64)
    );

    let reps_b_disk = 200usize;
    let start = Instant::now();
    for _ in 0..reps_b_disk {
        for e in &t_disk.edges {
            let _ = edge_b_verdict(&e.curve, p, d, &mut stats);
        }
    }
    let b_disk_us =
        start.elapsed().as_secs_f64() * 1.0e6 / (reps_b_disk as f64) / (t_disk.edges.len() as f64);
    println!(
        "criterion (b): {:.3} us per Circle-edge verdict; {:.3} us per seed-direction over the 2-circle disk target",
        b_disk_us,
        b_disk_us * (t_disk.edges.len() as f64)
    );
    println!(
        "(leaf stats guard: leaves={} behind={})",
        stats.leaves, stats.behind
    );
    println!();
}

/// T-5: the ε-sweep (near-rim offset): criterion (b) certification vs the
/// measured distance, and the direction-budget consequence.
#[test]
#[ignore]
fn probe_eps_sweep() {
    let target = build_target(&raised_disk(Point2::new(2.5, 2.0), 0.5, 0.5, 1.5));
    let eps: Vec<f64> = vec![
        5.0e-2, 2.0e-2, 1.0e-2, 5.0e-3, 2.0e-3, 1.0e-3, 5.0e-4, 1.0e-4,
    ];
    let dirs = ray_directions();
    let plus_z = dirs[0];
    let plus_x = dirs[1];
    println!();
    println!("== T-5 eps-sweep: near-rim offset vs certification ==");
    println!("target = raised_disk((2.5,2), r=0.5, z in [0.5,1.5]); seed = (2+eps, 2, 0)");
    println!("eps is the radial gap to the rim: +z passes the bottom rim at distance ~ eps");
    println!(
        "delta_abs = {:.3e}; delta_rel = {:.3e} x scale={:.4}",
        DELTA_ABS, DELTA_REL_MULT, target.scale
    );
    println!();
    println!(
        "{:>10} {:>14} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8} {:>8}",
        "eps",
        "+z edge_d",
        "b_clear",
        "near_abs",
        "near_rel",
        "+x edge_d",
        "b_clear_x",
        "clear",
        "refuse"
    );
    for e in &eps {
        let origin = Point3::new(2.0 + e, 2.0, 0.0);
        let m_z = measure_dir(&target, origin, plus_z);
        let m_x = measure_dir(&target, origin, plus_x);
        let mut clear = 0usize;
        let mut refuse = 0usize;
        for d in &dirs {
            let m = measure_dir(&target, origin, *d);
            if m.b_clear && m.min_dist >= DELTA_ABS {
                clear += 1;
            } else {
                refuse += 1;
            }
        }
        let rel_thr = DELTA_REL_MULT * target.scale;
        println!(
            "{:>10} {:>14} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8} {:>8}",
            fmt_dist(*e),
            fmt_dist(m_z.edge_dist),
            m_z.b_clear,
            m_z.min_dist < DELTA_ABS,
            m_z.min_dist < rel_thr,
            fmt_dist(m_x.edge_dist),
            m_x.b_clear,
            clear,
            refuse
        );
    }
    println!();
}

/// The 14-direction measurement is amortised in T-5 by measuring per needed
/// direction on demand.
fn measure_dir(target: &TargetGeom, origin: Point3, d: Vector3) -> DirMeasure {
    let mut edge_dist = f64::INFINITY;
    for e in &target.edges {
        for (a, b) in &e.segs {
            edge_dist = edge_dist.min(ray_segment_dist(origin, d, *a, *b));
        }
    }
    let mut vertex_dist = f64::INFINITY;
    for v in &target.vertices {
        vertex_dist = vertex_dist.min(ray_point_dist(origin, d, *v));
    }
    let mut stats = BLeafStats {
        leaves: 0,
        behind: 0,
    };
    let mut b_clear = true;
    for e in &target.edges {
        if edge_b_verdict(&e.curve, origin, d, &mut stats) == EdgeVerdict::Refuse {
            b_clear = false;
        }
    }
    DirMeasure {
        dir_index: 0,
        edge_dist,
        vertex_dist,
        min_dist: edge_dist.min(vertex_dist),
        b_clear,
        b_leaves: stats.leaves,
        b_behind: stats.behind,
    }
}

// ---------------------------------------------------------------------------
// corpus assembly
// ---------------------------------------------------------------------------

/// The corpus of (trial, seed) measurements. Trials come from classify.rs's
/// ray-seeded fixtures; near configurations reuse the same shells.
fn corpus_seeds() -> Vec<SeedMeasure> {
    let a_block = block_shell();
    let mut all = Vec::new();
    let trials: Vec<(String, Sh)> = vec![
        (
            "disjoint".to_string(),
            disk_column(Point2::new(6.0, 6.0), 1.0, 2.0),
        ),
        (
            "contained".to_string(),
            raised_disk(Point2::new(2.0, 2.0), 1.0, 0.5, 1.5),
        ),
        (
            "ambiguous".to_string(),
            raised_disk(Point2::new(2.5, 2.0), 0.5, 0.5, 1.5),
        ),
    ];
    for (name, target) in &trials {
        let t = build_target(target);
        let t_block = build_target(&a_block);
        // A-side seed: the block bottom-face representative, against the disk.
        all.push(measure_seed(
            name,
            "origin=a_bottom(2,2,0)",
            Point3::new(2.0, 2.0, 0.0),
            &t,
        ));
        // B-side seed: the disk's own bottom-cap representative, against the
        // block. For the contained/ambiguous disks that representative sits at
        // the cap centre at z = z_lo; for the disjoint column at (6,6,0).
        let b_rep = if name == "disjoint" {
            Point3::new(6.0, 6.0, 0.0)
        } else if name == "contained" {
            Point3::new(2.0, 2.0, 0.5)
        } else {
            Point3::new(2.5, 2.0, 0.5)
        };
        let b_label = if name == "disjoint" {
            "origin=b_bottom(6,6,0)"
        } else if name == "contained" {
            "origin=b_bottom(2,2,0.5)"
        } else {
            "origin=b_bottom(2.5,2,0.5)"
        };
        all.push(measure_seed(name, b_label, b_rep, &t_block));
    }
    // Vertex-graze: +z from (3,2,0) passes exactly through the rim vertex
    // (3,2,0.5) of the ambiguous raised disk.
    let amb = build_target(&raised_disk(Point2::new(2.5, 2.0), 0.5, 0.5, 1.5));
    all.push(measure_seed(
        "vertex-graze",
        "origin=a(3,2,0)",
        Point3::new(3.0, 2.0, 0.0),
        &amb,
    ));
    all
}
