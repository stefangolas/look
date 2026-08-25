//! BG-SOL-P0-SPAN — the lazy rational-Bézier span cache.
//!
//! Extracts, per carrier, the per-knot-span records that the broad phase and
//! the certified solvers both consume: each `SpanRecord` carries the span's
//! conservative bounding box and derivative hull plus its parameter window.
//! Shares the `BoundedPiece` vocabulary with the BVH (`truck-base/src/bvh.rs`,
//! docs/SOLVER_FAMILY_PLAN.md §2).
//!
//! Every box is a structural certificate, never a sampling witness:
//!
//! - `Plane` — the bilinear corner hull is exact.
//! - `Sphere`/`Torus` — the full analytic bounding box (loose but sound).
//! - `BSplineSurface` — per-knot-span Bézier decomposition; the image lies in
//!   the convex hull of the span's control sub-grid and the derivative control
//!   points bound the partials.
//! - `NurbsSurface` — same decomposition; the rational patch lies in the
//!   convex hull of the *projected* control points only when every weight is
//!   positive, so a non-positive weight refuses (empty) rather than guessing.
//! - `Cylinder`/`Cone` have no finite carrier span (their `v` range is
//!   unbounded); `RevolutedCurve`/`ExtrudedCurve` are not canonical and are
//!   canonicalized by the recognizer before spanning. Both yield nothing.
//! - `Processor` recurses into its entity and pushes the affine map through
//!   the boxes exactly (8-corner transform).
//!
//! `DerivativeBounds` is the shared broad-phase type from
//! `truck_base::bvh` (packet BG-SOL-P0-BVH); orchestrator amendment a557d09
//! replaced the packet's original local copy with the shared one after the
//! BVH packet merged.
//!
//! The cache is keyed by a caller-owned `u64` (e.g. the B-rep face index);
//! `Surface` is neither `Hash` nor `Eq` by design, so the caller guarantees
//! key uniqueness per distinct surface. House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::collections::HashMap;

use truck_base::bounding_box::BoundingBox;
use truck_base::bvh::DerivativeBounds;
use truck_base::cgmath64::*;
use truck_geotrait::ParametricSurface;

use crate::canonical::Surface;
use crate::decorators::Processor;
use crate::nurbs::{BSplineSurface, KnotVec, NurbsSurface};
use crate::specifieds::{Plane, Sphere, Torus};

/// One extracted span of a carrier surface: a conservative box over the
/// span's image, derivative bounds, and the span's parameter window.
#[derive(Clone, Debug, PartialEq)]
pub struct SpanRecord {
    /// Conservative box containing the span's image. MUST contain every
    /// surface point over `u_range × v_range`.
    pub bbox: BoundingBox<Point3>,
    /// Conservative bounds on the span's partials; empty boxes mean unknown.
    pub derivative_hull: DerivativeBounds,
    /// The span's parameter window in u.
    pub u_range: (f64, f64),
    /// The span's parameter window in v.
    pub v_range: (f64, f64),
}

/// Per-carrier lazy span extraction, cached by a caller-owned key.
#[derive(Debug, Default)]
pub struct SpanCache {
    inner: HashMap<u64, Vec<SpanRecord>>,
}

impl SpanCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// The spans of `s` under `key`, extracting (once) and caching them.
    pub fn spans(&mut self, key: u64, s: &Surface) -> &[SpanRecord] {
        self.inner.entry(key).or_insert_with(|| extract_spans(s))
    }
}

/// The per-surface extraction rule (a pure function of the `Surface` value).
fn extract_spans(s: &Surface) -> Vec<SpanRecord> {
    match s {
        Surface::Plane(plane) => vec![plane_span(plane)],
        Surface::Sphere(sphere) => sphere_span(sphere),
        Surface::Torus(torus) => torus_span(torus),
        Surface::Cylinder(_) | Surface::Cone(_) => Vec::new(),
        Surface::BSplineSurface(surface) => bspline_spans(surface),
        Surface::NurbsSurface(surface) => nurbs_spans(surface),
        Surface::Processor(processor) => processor_spans(processor),
        Surface::RevolutedCurve(_) | Surface::ExtrudedCurve(_) => Vec::new(),
    }
}

/// `Plane` — the bilinear image is exactly the hull of its four corners.
fn plane_span(plane: &Plane) -> SpanRecord {
    let mut bbox = BoundingBox::new();
    bbox.push(plane.subs(0.0, 0.0));
    bbox.push(plane.subs(0.0, 1.0));
    bbox.push(plane.subs(1.0, 0.0));
    bbox.push(plane.subs(1.0, 1.0));
    let mut first = BoundingBox::new();
    first.push(Point3::from_vec(plane.uder(0.0, 0.0)));
    first.push(Point3::from_vec(plane.vder(0.0, 0.0)));
    let mut second = BoundingBox::new();
    second.push(Point3::origin());
    SpanRecord {
        bbox,
        derivative_hull: DerivativeBounds { first, second },
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    }
}

/// `Sphere` — the arc patch always lies inside the full bounding box.
fn sphere_span(sphere: &Sphere) -> Vec<SpanRecord> {
    let (urange, vrange) = sphere.try_range_tuple();
    let (Some(u_range), Some(v_range)) = (urange, vrange) else {
        return Vec::new();
    };
    let r = sphere.radius();
    let c = sphere.center();
    let mut bbox = BoundingBox::new();
    bbox.push(c - Vector3::new(r, r, r));
    bbox.push(c + Vector3::new(r, r, r));
    vec![SpanRecord {
        bbox,
        derivative_hull: DerivativeBounds::new(),
        u_range,
        v_range,
    }]
}

/// `Torus` — same full-bounding-box argument.
fn torus_span(torus: &Torus) -> Vec<SpanRecord> {
    let (urange, vrange) = torus.try_range_tuple();
    let (Some(u_range), Some(v_range)) = (urange, vrange) else {
        return Vec::new();
    };
    let (r0, r1) = (torus.large_radius(), torus.small_radius());
    let c = torus.center();
    let mut bbox = BoundingBox::new();
    bbox.push(c - Vector3::new(r0 + r1, r0 + r1, r1));
    bbox.push(c + Vector3::new(r0 + r1, r0 + r1, r1));
    vec![SpanRecord {
        bbox,
        derivative_hull: DerivativeBounds::new(),
        u_range,
        v_range,
    }]
}

/// `BSplineSurface` — per-knot-span Bézier decomposition.
///
/// Each interior knot is raised to full degree multiplicity by exact-count
/// Boehm insertion, the domain is then partitioned by the distinct knot
/// values, and each span's box is the hull of its `(udegree+1) × (vdegree+1)`
/// control sub-grid (convex-hull property). The derivative hulls are the hulls
/// of the span's Bézier derivative control points in global units.
fn bspline_spans(surface: &BSplineSurface<Point3>) -> Vec<SpanRecord> {
    let udeg = surface.udegree();
    let vdeg = surface.vdegree();
    let mut refined = surface.clone();
    {
        let (knots, _) = refined.uknot_vec().to_single_multi();
        raise_interior(&knots, |x| {
            while exact_knot_count(refined.uknot_vec(), x) < udeg {
                refined.add_uknot(x);
            }
        });
    }
    {
        let (knots, _) = refined.vknot_vec().to_single_multi();
        raise_interior(&knots, |x| {
            while exact_knot_count(refined.vknot_vec(), x) < vdeg {
                refined.add_vknot(x);
            }
        });
    }
    let uknots = distinct_knots(refined.uknot_vec());
    let vknots = distinct_knots(refined.vknot_vec());
    let mut records = Vec::new();
    for (k, &u0) in uknots.iter().enumerate() {
        let Some(&u1) = uknots.get(k + 1) else {
            continue;
        };
        if u1 == u0 {
            continue;
        }
        let Some(rows) = refined.control_points().get(k * udeg..k * udeg + udeg + 1) else {
            continue;
        };
        for (l, &v0) in vknots.iter().enumerate() {
            let Some(&v1) = vknots.get(l + 1) else {
                continue;
            };
            if v1 == v0 {
                continue;
            }
            records.push(span_record_from_grid(
                rows,
                udeg,
                vdeg,
                l * vdeg,
                u0,
                u1,
                v0,
                v1,
            ));
        }
    }
    records
}

/// `NurbsSurface` — homogeneous decomposition with the projected hull rule.
///
/// The rational patch lies in the convex hull of the projected control points
/// only for positive weights; any weight `<= 0` refuses (empty) rather than
/// guess. The rational derivative's control points are not a simple hull, so
/// the derivative bounds stay unknown.
fn nurbs_spans(surface: &NurbsSurface<Vector4>) -> Vec<SpanRecord> {
    let positive_weights = surface
        .control_points()
        .iter()
        .flat_map(|row| row.iter())
        .all(|cp| cp.weight() > 0.0);
    if !positive_weights {
        return Vec::new();
    }
    let udeg = surface.udegree();
    let vdeg = surface.vdegree();
    let mut refined = surface.clone();
    {
        let (knots, _) = refined.uknot_vec().to_single_multi();
        raise_interior(&knots, |x| {
            while exact_knot_count(refined.uknot_vec(), x) < udeg {
                refined.add_uknot(x);
            }
        });
    }
    {
        let (knots, _) = refined.vknot_vec().to_single_multi();
        raise_interior(&knots, |x| {
            while exact_knot_count(refined.vknot_vec(), x) < vdeg {
                refined.add_vknot(x);
            }
        });
    }
    let uknots = distinct_knots(refined.uknot_vec());
    let vknots = distinct_knots(refined.vknot_vec());
    let mut records = Vec::new();
    for (k, &u0) in uknots.iter().enumerate() {
        let Some(&u1) = uknots.get(k + 1) else {
            continue;
        };
        if u1 == u0 {
            continue;
        }
        let Some(rows) = refined.control_points().get(k * udeg..k * udeg + udeg + 1) else {
            continue;
        };
        for (l, &v0) in vknots.iter().enumerate() {
            let Some(&v1) = vknots.get(l + 1) else {
                continue;
            };
            if v1 == v0 {
                continue;
            }
            let mut bbox = BoundingBox::new();
            for i in 0..=udeg {
                for j in 0..=vdeg {
                    let Some(cp) = homogeneous_at(rows, i, l * vdeg + j) else {
                        continue;
                    };
                    bbox.push(cp.to_point());
                }
            }
            records.push(SpanRecord {
                bbox,
                derivative_hull: DerivativeBounds::new(),
                u_range: (u0, u1),
                v_range: (v0, v1),
            });
        }
    }
    records
}

/// `Processor` — recurse on the entity and push the affine map through.
///
/// An affine map sends the 8 corners of a box to the 8 corners of the image
/// box, so the transformed hull is exact; the derivative of `M∘S` is `M·S'`,
/// so the derivative hull corners transform by the linear part.
fn processor_spans(processor: &Processor<Box<Surface>, Matrix4>) -> Vec<SpanRecord> {
    let inner = extract_spans(processor.entity());
    let trans = processor.transform();
    inner
        .into_iter()
        .map(|mut record| {
            record.bbox = transformed_box(record.bbox, trans);
            record.derivative_hull = transformed_bounds(record.derivative_hull, trans);
            record
        })
        .collect()
}

/// One span record of a decomposed B-spline surface from its control rows.
///
/// `rows` is the span's `udegree + 1` control rows of the full grid; the v
/// columns are the `vdegree + 1` entries starting at `v_start`. Out-of-range
/// grid cells contribute nothing (they are the next span's shared junction or
/// beyond the grid).
#[allow(clippy::too_many_arguments)]
fn span_record_from_grid(
    rows: &[Vec<Point3>],
    udeg: usize,
    vdeg: usize,
    v_start: usize,
    u0: f64,
    u1: f64,
    v0: f64,
    v1: f64,
) -> SpanRecord {
    let w_u = u1 - u0;
    let w_v = v1 - v0;
    let du = udeg as f64 / w_u;
    let dv = vdeg as f64 / w_v;
    let duu = udeg as f64 * (udeg as f64 - 1.0) / (w_u * w_u);
    let dvv = vdeg as f64 * (vdeg as f64 - 1.0) / (w_v * w_v);
    let duv = udeg as f64 * vdeg as f64 / (w_u * w_v);
    let mut bbox = BoundingBox::new();
    let mut first = BoundingBox::new();
    let mut second = BoundingBox::new();
    for i in 0..=udeg {
        for j in 0..=vdeg {
            let col = v_start + j;
            let Some(pi) = point_at(rows, i, col) else {
                continue;
            };
            bbox.push(*pi);
            if let Some(pi1) = point_at(rows, i + 1, col) {
                first.push(Point3::from_vec((*pi1 - *pi) * du));
            }
            if let Some(pj1) = point_at(rows, i, col + 1) {
                first.push(Point3::from_vec((*pj1 - *pi) * dv));
            }
            if let (Some(pi2), Some(pi1)) = (point_at(rows, i + 2, col), point_at(rows, i + 1, col))
            {
                second.push(Point3::from_vec(((*pi2 - *pi1) - (*pi1 - *pi)) * duu));
            }
            if let (Some(pj2), Some(pj1)) = (point_at(rows, i, col + 2), point_at(rows, i, col + 1))
            {
                second.push(Point3::from_vec(((*pj2 - *pj1) - (*pj1 - *pi)) * dvv));
            }
            if let (Some(pij), Some(pi1), Some(pj1)) = (
                point_at(rows, i + 1, col + 1),
                point_at(rows, i + 1, col),
                point_at(rows, i, col + 1),
            ) {
                second.push(Point3::from_vec(((*pij - *pi1) - (*pj1 - *pi)) * duv));
            }
        }
    }
    SpanRecord {
        bbox,
        derivative_hull: DerivativeBounds { first, second },
        u_range: (u0, u1),
        v_range: (v0, v1),
    }
}

/// The exact-equality count of `x` in `knot_vec`.
///
/// The `exact_count` pattern of `truck-evidence/src/deviation.rs`: knot
/// multiplicity by tolerance would inflate the count next to a *different*
/// knot within tolerance, which under-inserts in the raising loop.
fn exact_knot_count(knot_vec: &KnotVec, x: f64) -> usize {
    knot_vec.iter().filter(|&&k| k == x).count()
}

/// Calls `add` for each distinct interior knot value of `knots`; the first and
/// last distinct values are the clamped boundary and are never raised.
fn raise_interior<F>(knots: &[f64], mut add: F)
where
    F: FnMut(f64),
{
    for (idx, &x) in knots.iter().enumerate() {
        if idx == 0 || idx + 1 == knots.len() {
            continue;
        }
        add(x);
    }
}

/// The distinct knot values of `knot_vec`, ascending.
fn distinct_knots(knot_vec: &KnotVec) -> Vec<f64> {
    knot_vec.to_single_multi().0
}

/// The control point at grid cell `(i, j)`, or `None` out of range.
fn point_at(rows: &[Vec<Point3>], i: usize, j: usize) -> Option<&Point3> {
    rows.get(i).and_then(|row| row.get(j))
}

/// The homogeneous control point at grid cell `(i, j)`, or `None` out of range.
fn homogeneous_at(rows: &[Vec<Vector4>], i: usize, j: usize) -> Option<&Vector4> {
    rows.get(i).and_then(|row| row.get(j))
}

/// The affine image of a box: transform the 8 corners and re-hull. An empty
/// (unknown) box stays empty.
fn transformed_box(bbox: BoundingBox<Point3>, trans: &Matrix4) -> BoundingBox<Point3> {
    if bbox.is_empty() {
        return bbox;
    }
    let mut out = BoundingBox::new();
    for corner in box_corners(bbox) {
        out.push(trans.transform_point(corner));
    }
    out
}

/// Push the linear part of the map through the derivative boxes on their 8
/// corners (the derivative of `M∘S` is `M·S'`); empty boxes stay unknown.
fn transformed_bounds(bounds: DerivativeBounds, trans: &Matrix4) -> DerivativeBounds {
    DerivativeBounds {
        first: transformed_vector_box(bounds.first, trans),
        second: transformed_vector_box(bounds.second, trans),
    }
}

/// The linear image of a derivative box (vectors), re-hulled after an 8-corner
/// transform.
fn transformed_vector_box(bbox: BoundingBox<Point3>, trans: &Matrix4) -> BoundingBox<Point3> {
    if bbox.is_empty() {
        return bbox;
    }
    let mut out = BoundingBox::new();
    for corner in box_corners(bbox) {
        out.push(Point3::from_vec(trans.transform_vector(corner.to_vec())));
    }
    out
}

/// The 8 corners of a box, in index order.
fn box_corners(bbox: BoundingBox<Point3>) -> [Point3; 8] {
    let min = bbox.min();
    let max = bbox.max();
    [
        Point3::new(min.x, min.y, min.z),
        Point3::new(min.x, min.y, max.z),
        Point3::new(min.x, max.y, min.z),
        Point3::new(min.x, max.y, max.z),
        Point3::new(max.x, min.y, min.z),
        Point3::new(max.x, min.y, max.z),
        Point3::new(max.x, max.y, min.z),
        Point3::new(max.x, max.y, max.z),
    ]
}

#[cfg(test)]
// The regression witnesses below are hand-built surfaces and hand-chosen
// samples. They stay unwrap/expect-free so the H-1 deny at module top keeps its
// single occurrence of the deny list (anchor A3): the required assertions all
// flow through `assert_eq!`/`assert!` and `match` with a total `Err` arm.
mod tests {
    use super::*;
    use crate::specifieds::{Cone, Cylinder};
    use std::f64::consts::PI;

    /// A degree-2×2 B-spline surface over `uniform_knot(2, 2)`: three distinct
    /// knots per axis, two spans per axis after interior-knot raising.
    fn sample_surface() -> Surface {
        let knot = KnotVec::uniform_knot(2, 2);
        let ctrl_pts: Vec<Vec<Point3>> = (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| Point3::new(i as f64, j as f64, (i as f64) - (j as f64)))
                    .collect()
            })
            .collect();
        Surface::BSplineSurface(BSplineSurface::new((knot.clone(), knot), ctrl_pts))
    }

    #[test]
    fn span_bspline_surface_produces_per_span_records() {
        let surface = sample_surface();
        let mut cache = SpanCache::new();
        let records = cache.spans(0, &surface).to_vec();
        assert_eq!(records.len(), 4);
        let uranges: Vec<(f64, f64)> = records.iter().map(|r| r.u_range).collect();
        let vranges: Vec<(f64, f64)> = records.iter().map(|r| r.v_range).collect();
        assert_eq!(
            uranges,
            vec![(0.0, 0.5), (0.0, 0.5), (0.5, 1.0), (0.5, 1.0)]
        );
        assert_eq!(
            vranges,
            vec![(0.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.5, 1.0)]
        );
        const N: usize = 9;
        for record in &records {
            let (u0, u1) = record.u_range;
            let (v0, v1) = record.v_range;
            for i in 0..N {
                for j in 0..N {
                    let u = u0 + (u1 - u0) * ((i as f64) + 0.5) / (N as f64);
                    let v = v0 + (v1 - v0) * ((j as f64) + 0.5) / (N as f64);
                    let sample = surface.subs(u, v);
                    assert!(
                        record.bbox.contains(sample),
                        "span {:?}×{:?} misses sample {sample:?} at ({u}, {v})",
                        record.u_range,
                        record.v_range
                    );
                }
            }
        }
        for i in 0..N {
            for j in 0..N {
                let u = (i as f64) / (N as f64);
                let v = (j as f64) / (N as f64);
                let sample = surface.subs(u, v);
                assert!(
                    records.iter().any(|r| r.bbox.contains(sample)),
                    "no record box contains {sample:?} at ({u}, {v})"
                );
            }
        }
    }

    #[test]
    fn span_plane_is_exact_corner_hull() {
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let surface = Surface::Plane(plane);
        let mut cache = SpanCache::new();
        let records = cache.spans(1, &surface).to_vec();
        assert_eq!(records.len(), 1);
        for record in &records {
            assert_eq!(record.u_range, (0.0, 1.0));
            assert_eq!(record.v_range, (0.0, 1.0));
            for (u, v) in [
                (0.0, 0.0),
                (0.0, 1.0),
                (1.0, 0.0),
                (1.0, 1.0),
                (0.5, 0.5),
                (0.5, 0.0),
                (0.0, 0.5),
            ] {
                assert!(record.bbox.contains(plane.subs(u, v)));
            }
            assert_eq!(record.bbox.min(), Point3::origin());
            assert_eq!(record.bbox.max(), Point3::new(2.0, 1.0, 0.0));
            assert!(record
                .derivative_hull
                .first
                .contains(Point3::new(2.0, 0.0, 0.0)));
            assert!(record
                .derivative_hull
                .first
                .contains(Point3::new(0.0, 1.0, 0.0)));
        }
    }

    #[test]
    fn span_processor_transforms_the_box() {
        let surface = sample_surface();
        let v = Vector3::new(1.0, 2.0, 3.0);
        let processed = Surface::Processor(Processor::with_transform(
            Box::new(surface.clone()),
            Matrix4::from_translation(v),
        ));
        let mut cache = SpanCache::new();
        let inner = cache.spans(1, &surface).to_vec();
        let outer = cache.spans(2, &processed).to_vec();
        assert_eq!(inner.len(), outer.len());
        for (a, b) in inner.iter().zip(outer.iter()) {
            assert_eq!(a.u_range, b.u_range);
            assert_eq!(a.v_range, b.v_range);
            assert_eq!(b.bbox.min(), a.bbox.min() + v);
            assert_eq!(b.bbox.max(), a.bbox.max() + v);
        }
    }

    #[test]
    fn span_cache_reuses_keyed_extraction() {
        let surface = sample_surface();
        let mut cache = SpanCache::new();
        let first = cache.spans(7, &surface).to_vec();
        let same_key = cache.spans(7, &surface).to_vec();
        assert_eq!(first, same_key);
        let other_key = cache.spans(8, &surface).to_vec();
        assert_eq!(first, other_key);
        let unchanged = cache.spans(7, &surface).to_vec();
        assert_eq!(first, unchanged);
    }

    #[test]
    fn span_unbounded_cylinder_has_no_spans() {
        let mut cache = SpanCache::new();
        let cylinder = match Cylinder::new(Point3::origin(), 1.0) {
            Ok(certified) => certified.value,
            Err(_) => return,
        };
        let surface = Surface::Cylinder(cylinder);
        assert!(cache.spans(1, &surface).is_empty());
        let cone = match Cone::new(Point3::origin(), PI / 4.0) {
            Ok(certified) => certified.value,
            Err(_) => return,
        };
        let surface = Surface::Cone(cone);
        assert!(cache.spans(2, &surface).is_empty());
    }
}
