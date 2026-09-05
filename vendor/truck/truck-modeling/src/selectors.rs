//! PB-001-SELECTORS — the scoped selector layer over live topology.
//!
//! The facade books Python selectors out of scope (`truck-shapeops` facade
//! header, line 10); the build123d-shaped fluent vocabulary
//! (`> Faces().SortBy(Axis.Z)` and friends) needs a Rust substrate that this
//! module provides. It is deliberately shallow and deterministic:
//!
//! * [`faces`] / [`edges`] enumerate a solid's faces / unique edges in a
//!   fixed topological order. Every returned [`FaceRef`] / [`EdgeRef`] carries
//!   an [`EntityId`]-derived identity — `EntityId::sel(base, selector)`, never
//!   a parallel identity scheme — where `base` is the caller-supplied id of the
//!   solid itself (an import `EntityId::src(i)`, an `Op::output`, or any other
//!   construction id). Ordering is the solid's structural enumeration order
//!   (shell-major, then stored face / absolute-wire / wire-position order),
//!   which is exactly the EntityId order: each ref's selector index *is* its
//!   structural index. No hashing order anywhere.
//! * [`FaceFact`] carries the geometric facts per face: an area-weighted
//!   centroid and an axis-aligned bounding box, plus a [`Method`] tag. A planar
//!   face whose boundary is a single straight-edge loop (an analytic polygon)
//!   gets the facts in closed form from its boundary — tag [`Method::Exact`];
//!   everything else is sampled (a parameter-domain fan grid mirroring the
//!   `showcases` harness `brep_volume` pattern, or a boundary polyline sample)
//!   and is tagged [`Method::Float`]. H-6: sampled facts are never `Exact`.
//! * The query vocabulary: [`sort_by_axis`](FaceSelection::sort_by_axis),
//!   [`group_by_axis`](FaceSelection::group_by_axis),
//!   [`filter_by_plane`](FaceSelection::filter_by_plane),
//!   [`take`](FaceSelection::take) / [`last`](FaceSelection::last).
//! * [`EdgeRef::edge_name`] resolves a selected edge to the name the landed
//!   `BlendSpec`/fillet/chamfer machinery consumes: straight edges name their
//!   two absolute endpoints (`a`, `b` — exactly the `FilletSpec`/`ChamferSpec`
//!   `a`/`b` endpoint fields, matched either order within the landed
//!   1e-2 insertion tolerance), and full-circle rims name their canonical
//!   circle geometry (`center`, `radius` — exactly the `CircleFilletSpec`
//!   `center`/`edge_radius` fields). A trimmed arc or any non-line/circle
//!   carrier has no blend target and resolves to `None`.
//!
//! This module never panics, never `unwrap`s, and never indexes without
//! bounds on any path reachable from a caller-supplied solid (H-1).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::f64::consts::TAU;

use crate::{Curve, Edge, Face, Plane, Point3, Solid, Surface, Vector3, Wire};
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace};
use truck_base::evidence::Method;
use truck_geotrait::{BoundedCurve, ParametricCurve, ParametricSurface};
use truck_topology::{EntityId, Selector};

/// The per-dimension fan-grid resolution used for sampled face facts.
const GRID: usize = 32;
/// The per-edge sample count used when a curved boundary is polyline-sampled.
const EDGE_SAMPLES: usize = 16;
/// The machine-scale tolerance used to decide "the circle spans a full turn".
const FULL_TURN: f64 = 1.0e-9;

/// An axis of the world frame, for `sort_by_axis` / `group_by_axis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The x-axis.
    X,
    /// The y-axis.
    Y,
    /// The z-axis.
    Z,
}

impl Axis {
    /// The `axis` coordinate of a point.
    pub fn coord(self, point: Point3) -> f64 {
        match self {
            Axis::X => point.x,
            Axis::Y => point.y,
            Axis::Z => point.z,
        }
    }
}

/// The geometric facts of one face: centroid, axis-aligned bounding box, and
/// the method tag (H-6: a sampled fact is never tagged `Exact`).
#[derive(Clone, Debug)]
pub struct FaceFact {
    /// The area-weighted centroid of the face (closed-form for an analytic
    /// planar polygon, fan-sampled otherwise).
    pub centroid: Point3,
    /// The axis-aligned bounding box of the face.
    pub aabb: BoundingBox<Point3>,
    /// `Exact` when the facts come from the analytic boundary in closed form;
    /// `Float` when they were computed by sampling.
    pub method: Method,
}

/// One face of a selection: its EntityId-derived identity and its live facts.
#[derive(Clone, Debug)]
pub struct FaceRef {
    id: EntityId,
    face: Face,
    fact: FaceFact,
}

impl FaceRef {
    /// The EntityId-derived identity of this face.
    pub fn id(&self) -> &EntityId {
        &self.id
    }

    /// The face's geometric facts.
    pub fn fact(&self) -> &FaceFact {
        &self.fact
    }

    /// The underlying face.
    pub fn face(&self) -> &Face {
        &self.face
    }
}

/// The deterministic list of faces of a solid together with the selection
/// vocabulary.
#[derive(Clone, Debug)]
pub struct FaceSelection {
    refs: Vec<FaceRef>,
}

impl FaceSelection {
    /// The number of selected faces.
    pub fn len(&self) -> usize {
        self.refs.len()
    }

    /// Whether no face is selected.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// The selected refs in current order.
    pub fn refs(&self) -> &[FaceRef] {
        &self.refs
    }

    /// Sorts the selection stably by the centroid coordinate along `axis`,
    /// ascending. Equal coordinates keep their deterministic order.
    pub fn sort_by_axis(mut self, axis: Axis) -> Self {
        self.refs.sort_by(|a, b| {
            axis.coord(a.fact.centroid)
                .total_cmp(&axis.coord(b.fact.centroid))
        });
        self
    }

    /// Groups the selection by the centroid coordinate along `axis`, in
    /// ascending coordinate order. Each group is returned with its coordinate
    /// and the faces whose centroid sits on it (faces sorted within a group).
    pub fn group_by_axis(mut self, axis: Axis) -> Vec<(f64, FaceSelection)> {
        self.refs.sort_by(|a, b| {
            axis.coord(a.fact.centroid)
                .total_cmp(&axis.coord(b.fact.centroid))
        });
        let mut groups: Vec<(f64, FaceSelection)> = Vec::new();
        for face in self.refs {
            let c = axis.coord(face.fact.centroid);
            match groups.last_mut() {
                Some((key, group)) if (*key - c).abs() <= FULL_TURN => {
                    group.refs.push(face);
                }
                _ => {
                    groups.push((c, FaceSelection { refs: vec![face] }));
                }
            }
        }
        groups
    }

    /// Keeps only the faces lying in `plane`: every boundary vertex of the
    /// face must sit within `tol` of the plane (so a face is kept only when it
    /// is fully on the plane).
    pub fn filter_by_plane(self, plane: &Plane, tol: f64) -> Self {
        let normal = plane.normal();
        let origin = plane.origin();
        let on_plane = |face: &FaceRef| {
            face.face.absolute_boundaries().iter().all(|wire| {
                wire.edge_iter().all(|edge| {
                    let (a, b) = edge.absolute_ends();
                    dist_to_plane(a.point(), origin, normal) <= tol
                        && dist_to_plane(b.point(), origin, normal) <= tol
                })
            })
        };
        FaceSelection {
            refs: self.refs.into_iter().filter(on_plane).collect(),
        }
    }

    /// The first `n` faces in the current order.
    pub fn take(mut self, n: usize) -> Self {
        self.refs.truncate(n);
        self
    }

    /// The last face in the current order, if any.
    pub fn last(mut self) -> Option<FaceRef> {
        self.refs.pop()
    }

    /// Consumes the selection into its refs.
    pub fn into_vec(self) -> Vec<FaceRef> {
        self.refs
    }
}

/// One unique edge of a solid together with its EntityId-derived identity.
#[derive(Clone, Debug)]
pub struct EdgeRef {
    id: EntityId,
    edge: Edge,
}

impl EdgeRef {
    /// The EntityId-derived identity of this edge.
    pub fn id(&self) -> &EntityId {
        &self.id
    }

    /// The underlying edge.
    pub fn edge(&self) -> &Edge {
        &self.edge
    }

    /// Resolves this edge to the name the landed `BlendSpec`/fillet/chamfer
    /// machinery consumes: the two absolute endpoints for a straight edge, the
    /// canonical circle geometry for a full-circle rim. Trimmed arcs and
    /// non-line/circle carriers have no blend target and resolve to `None`.
    pub fn edge_name(&self) -> Option<EdgeName> {
        let curve = self.edge.curve();
        match curve {
            Curve::Line(_) => {
                let (a, b) = self.edge.absolute_ends();
                Some(EdgeName::Straight {
                    a: a.point(),
                    b: b.point(),
                })
            }
            c @ Curve::Circle(_) => {
                // The canonical rim is a full-turn circle. A trimmed arc is not
                // a rim and has no `CircleFilletSpec` target.
                let (t0, t1) = c.range_tuple();
                if (t1 - t0 - TAU).abs() > FULL_TURN {
                    return None;
                }
                let Curve::Circle(circle) = c else {
                    return None;
                };
                let t = circle.transform();
                let center = Point3::new(t.w.x, t.w.y, t.w.z);
                let radius = t.x.magnitude();
                Some(EdgeName::Circular { center, radius })
            }
            _ => None,
        }
    }
}

/// The blend-target name of a selected edge: the data the landed
/// `FilletSpec`/`CircleFilletSpec`/`ChamferSpec` rows carry.
#[derive(Clone, Copy, Debug)]
pub enum EdgeName {
    /// A straight (plane-plane) edge, named by its two absolute endpoints.
    /// Fills `FilletSpec { a, b, .. }` and `ChamferSpec { a, b, .. }` directly
    /// (either endpoint order is accepted by the landed resolver).
    Straight {
        /// One endpoint of the edge.
        a: Point3,
        /// The other endpoint of the edge.
        b: Point3,
    },
    /// A full-circle rim, named by its canonical circle geometry. Fills
    /// `CircleFilletSpec { center, edge_radius: radius, .. }` directly.
    Circular {
        /// The circle's center.
        center: Point3,
        /// The circle's radius.
        radius: f64,
    },
}

/// The deterministic list of unique edges of a solid together with the
/// selection vocabulary.
#[derive(Clone, Debug)]
pub struct EdgeSelection {
    refs: Vec<EdgeRef>,
}

impl EdgeSelection {
    /// The number of selected unique edges.
    pub fn len(&self) -> usize {
        self.refs.len()
    }

    /// Whether no edge is selected.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// The selected refs in current order.
    pub fn refs(&self) -> &[EdgeRef] {
        &self.refs
    }

    /// The first `n` edges in the current order.
    pub fn take(mut self, n: usize) -> Self {
        self.refs.truncate(n);
        self
    }

    /// The last edge in the current order, if any.
    pub fn last(mut self) -> Option<EdgeRef> {
        self.refs.pop()
    }

    /// Consumes the selection into its refs.
    pub fn into_vec(self) -> Vec<EdgeRef> {
        self.refs
    }
}

/// Enumerates the faces of `solid` in deterministic topological order. Each
/// ref's identity is `EntityId::sel(base, Selector::BoundaryWire(i))` where `i`
/// is the face's index in the structural enumeration (shell-major, then stored
/// face order) — the EntityId order of the returned refs IS the enumeration
/// order, and iterating the same solid twice yields the same refs.
pub fn faces(solid: &Solid, base: EntityId) -> FaceSelection {
    let mut refs = Vec::new();
    for (i, face) in solid.face_iter().enumerate() {
        let id = EntityId::sel(base.clone(), Selector::BoundaryWire(i as u32));
        refs.push(FaceRef {
            id,
            face: face.clone(),
            fact: face_fact(face),
        });
    }
    FaceSelection { refs }
}

/// Enumerates the unique edges of `solid` in deterministic topological order
/// (first structural occurrence of each unique edge, deduplicated by edge
/// identity — never by hash order). Each ref's identity is
/// `EntityId::sel(base, Selector::BoundaryWire(f))` →
/// `Selector::BoundaryWire(w)` → `Selector::WireEdge(p)` for its canonical
/// occurrence `(f, w, p)`.
pub fn edges(solid: &Solid, base: EntityId) -> EdgeSelection {
    let mut seen: HashSet<truck_topology::EdgeID<Curve>> = HashSet::new();
    let mut refs = Vec::new();
    for (f, face) in solid.face_iter().enumerate() {
        let face_id = EntityId::sel(base.clone(), Selector::BoundaryWire(f as u32));
        for (w, wire) in face.absolute_boundaries().iter().enumerate() {
            let wire_id = EntityId::sel(face_id.clone(), Selector::BoundaryWire(w as u32));
            for (p, edge) in wire.edge_iter().enumerate() {
                if !seen.insert(edge.id()) {
                    continue;
                }
                let id = EntityId::sel(wire_id.clone(), Selector::WireEdge(p as u32));
                refs.push(EdgeRef {
                    id,
                    edge: edge.clone(),
                });
            }
        }
    }
    EdgeSelection { refs }
}

/// The facts of one face: closed form for an analytic planar polygon, a
/// fan-sampled grid over the surface parameter domain otherwise (H-6: sampled
/// facts are tagged [`Method::Float`], never `Exact`).
fn face_fact(face: &Face) -> FaceFact {
    match face.shared_surface() {
        Surface::Plane(plane) => planar_facts(face, plane),
        _ => sampled_surface_facts(face),
    }
}

/// The facts of a planar face: exact polygon facts when the face is one
/// straight-edge loop; otherwise a sampled boundary polyline (Float).
fn planar_facts(face: &Face, plane: &Plane) -> FaceFact {
    let wires = face.absolute_boundaries();
    if let Some(fact) = wires
        .first()
        .and_then(|wire| exact_planar_polygon_facts(wire, plane))
    {
        return fact;
    }
    sampled_planar_facts(face, plane)
}

/// Closed-form centroid + AABB of a planar face whose boundary is one
/// straight-edge wire: the region IS the polygon of its vertices, so the
/// signed-area (shoelace) centroid and the vertex AABB are exact (tag
/// `Exact`). `None` when the polygon is degenerate or not straight-edged.
fn exact_planar_polygon_facts(wire: &Wire, plane: &Plane) -> Option<FaceFact> {
    let mut world: Vec<Point3> = Vec::new();
    for edge in wire.edge_iter() {
        if !matches!(edge.curve(), Curve::Line(_)) {
            return None;
        }
        world.push(edge.front().point());
    }
    if world.len() < 3 {
        return None;
    }
    let mut uvs: Vec<(f64, f64)> = Vec::with_capacity(world.len());
    for point in &world {
        let uv = plane.get_parameter(*point);
        uvs.push((uv.x, uv.y));
    }
    let (cu, cv, _twice) = shoelace_centroid(&uvs)?;
    let centroid = plane.subs(cu, cv);
    let mut aabb = BoundingBox::new();
    for point in &world {
        aabb.push(*point);
    }
    Some(FaceFact {
        centroid,
        aabb,
        method: Method::Exact,
    })
}

/// The sampled centroid + AABB of a planar face with curved boundary or holes:
/// each absolute boundary loop is polyline-sampled (curved edges at
/// `EDGE_SAMPLES` per edge), the outer loop is the largest-area loop, and the
/// region centroid is the area-weighted outer-minus-holes combination.
fn sampled_planar_facts(face: &Face, plane: &Plane) -> FaceFact {
    let mut loops: Vec<Vec<Point3>> = Vec::new();
    for wire in face.absolute_boundaries() {
        loops.push(sample_wire_loop(wire));
    }
    let fallback = |loops: &Vec<Vec<Point3>>| {
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        let mut count = 0.0;
        let mut aabb = BoundingBox::new();
        for loop_points in loops {
            for point in loop_points {
                sum += point.to_vec();
                count += 1.0;
                aabb.push(*point);
            }
        }
        FaceFact {
            centroid: if count > 0.0 {
                Point3::from_vec(sum / count)
            } else {
                Point3::new(0.0, 0.0, 0.0)
            },
            aabb,
            method: Method::Float,
        }
    };
    if loops.is_empty() {
        return fallback(&loops);
    }
    // (loop index, centroid-u, centroid-v, signed twice-area) per sampled loop.
    let mut facts: Vec<(usize, f64, f64, f64)> = Vec::new();
    for (i, loop_points) in loops.iter().enumerate() {
        let mut uvs: Vec<(f64, f64)> = Vec::with_capacity(loop_points.len());
        for point in loop_points {
            let uv = plane.get_parameter(*point);
            uvs.push((uv.x, uv.y));
        }
        if let Some((cu, cv, twice)) = shoelace_centroid(&uvs) {
            facts.push((i, cu, cv, twice));
        }
    }
    if facts.is_empty() {
        return fallback(&loops);
    }
    let outer = facts.iter().fold(0usize, |best, (i, _, _, twice)| {
        let (_, _, _, best_twice) = *facts.get(best).unwrap_or(&(0, 0.0, 0.0, 0.0));
        if twice.abs() > best_twice.abs() {
            *i
        } else {
            best
        }
    });
    let (_, outer_cu, outer_cv, outer_twice) = match facts.get(outer) {
        Some(fact) => *fact,
        None => return fallback(&loops),
    };
    let outer_area = outer_twice.abs() * 0.5;
    let mut area_sum = outer_area;
    let mut weighted = plane.subs(outer_cu, outer_cv).to_vec() * outer_area;
    for &(i, cu, cv, twice) in &facts {
        if i == outer {
            continue;
        }
        let hole_area = twice.abs() * 0.5;
        let hole_centroid = plane.subs(cu, cv);
        area_sum -= hole_area;
        weighted -= hole_centroid.to_vec() * hole_area;
    }
    let centroid = if area_sum.abs() > FULL_TURN {
        Point3::from_vec(weighted / area_sum)
    } else {
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        let mut count = 0.0;
        for loop_points in &loops {
            for point in loop_points {
                sum += point.to_vec();
                count += 1.0;
            }
        }
        Point3::from_vec(if count > 0.0 { sum / count } else { sum })
    };
    let mut aabb = BoundingBox::new();
    for loop_points in &loops {
        for point in loop_points {
            aabb.push(*point);
        }
    }
    FaceFact {
        centroid,
        aabb,
        method: Method::Float,
    }
}

/// The fan-grid sampled centroid + AABB of a non-planar face: samples the
/// bounded surface parameter domain on a `GRID` x `GRID` fan (two triangles
/// per cell, the `showcases` harness pattern). Unbounded surfaces fall back to
/// a boundary-polyline sample. Always tagged `Float` (H-6).
fn sampled_surface_facts(face: &Face) -> FaceFact {
    let surface = face.surface();
    let (ur, vr) = surface.try_range_tuple();
    if let (Some((u0, u1)), Some((v0, v1))) = (ur, vr) {
        if u1 - u0 <= 0.0 || v1 - v0 <= 0.0 {
            return boundary_facts(face);
        }
        let mut area_sum = 0.0;
        let mut weighted = Vector3::new(0.0, 0.0, 0.0);
        let mut aabb = BoundingBox::new();
        for i in 0..GRID {
            for j in 0..GRID {
                let a = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / GRID as f64),
                    v0 + (v1 - v0) * (j as f64 / GRID as f64),
                );
                let b = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / GRID as f64),
                    v0 + (v1 - v0) * (j as f64 / GRID as f64),
                );
                let c = surface.subs(
                    u0 + (u1 - u0) * ((i + 1) as f64 / GRID as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / GRID as f64),
                );
                let d = surface.subs(
                    u0 + (u1 - u0) * (i as f64 / GRID as f64),
                    v0 + (v1 - v0) * ((j + 1) as f64 / GRID as f64),
                );
                aabb.push(a);
                aabb.push(b);
                aabb.push(c);
                aabb.push(d);
                accumulate_triangle(&mut area_sum, &mut weighted, a, b, c);
                accumulate_triangle(&mut area_sum, &mut weighted, a, c, d);
            }
        }
        let centroid = if area_sum > 0.0 {
            Point3::from_vec(weighted / area_sum)
        } else {
            let mut sum = Vector3::new(0.0, 0.0, 0.0);
            let mut count = 0.0;
            for i in 0..=GRID {
                for j in 0..=GRID {
                    let p = surface.subs(
                        u0 + (u1 - u0) * (i as f64 / GRID as f64),
                        v0 + (v1 - v0) * (j as f64 / GRID as f64),
                    );
                    sum += p.to_vec();
                    count += 1.0;
                }
            }
            Point3::from_vec(if count > 0.0 { sum / count } else { sum })
        };
        return FaceFact {
            centroid,
            aabb,
            method: Method::Float,
        };
    }
    boundary_facts(face)
}

/// Accumulates one triangle's signed double-area and area-weighted centroid.
fn accumulate_triangle(
    area_sum: &mut f64,
    weighted: &mut Vector3,
    a: Point3,
    b: Point3,
    c: Point3,
) {
    let double_area = (b - a).cross(c - a).magnitude();
    *area_sum += double_area;
    let centroid = (a.to_vec() + b.to_vec() + c.to_vec()) * (1.0 / 3.0);
    *weighted += centroid * double_area;
}

/// The fallback facts from the face's boundary polylines alone: the mean of
/// the sampled points as the centroid, the polyline AABB (Float).
fn boundary_facts(face: &Face) -> FaceFact {
    let mut sum = Vector3::new(0.0, 0.0, 0.0);
    let mut count = 0.0;
    let mut aabb = BoundingBox::new();
    for wire in face.absolute_boundaries() {
        for edge in wire.edge_iter() {
            for point in sample_edge(edge) {
                sum += point.to_vec();
                count += 1.0;
                aabb.push(point);
            }
        }
    }
    let centroid = if count > 0.0 {
        Point3::from_vec(sum / count)
    } else {
        Point3::new(0.0, 0.0, 0.0)
    };
    FaceFact {
        centroid,
        aabb,
        method: Method::Float,
    }
}

/// The signed-area centroid of a closed polygon given in affine coordinates:
/// returns `(centroid_u, centroid_v, twice_area)` or `None` for a degenerate
/// polygon. Valid for any simple polygon (concave included) in either winding.
fn shoelace_centroid(uvs: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    if uvs.len() < 3 {
        return None;
    }
    let mut twice = 0.0;
    let mut su = 0.0;
    let mut sv = 0.0;
    for i in 0..uvs.len() {
        let j = (i + 1) % uvs.len();
        let (x0, y0) = *uvs.get(i)?;
        let (x1, y1) = *uvs.get(j)?;
        let cross = x0 * y1 - x1 * y0;
        twice += cross;
        su += (x0 + x1) * cross;
        sv += (y0 + y1) * cross;
    }
    if twice == 0.0 {
        return None;
    }
    Some((su / (3.0 * twice), sv / (3.0 * twice), twice))
}

/// The absolute distance of `point` from the plane through `origin` with unit
/// normal `normal`.
fn dist_to_plane(point: Point3, origin: Point3, normal: Vector3) -> f64 {
    (point - origin).dot(normal).abs()
}

/// Samples one absolute boundary wire into a closed polyline of world points:
/// every line edge contributes its endpoints, every curved edge additionally
/// `EDGE_SAMPLES` interior points along its oriented curve.
fn sample_wire_loop(wire: &Wire) -> Vec<Point3> {
    let mut points: Vec<Point3> = Vec::new();
    for edge in wire.edge_iter() {
        points.extend(sample_edge(edge));
    }
    points
}

/// The world-space samples along one edge from its absolute front to its
/// absolute back.
fn sample_edge(edge: &Edge) -> Vec<Point3> {
    let mut points: Vec<Point3> = Vec::new();
    match edge.curve() {
        Curve::Line(_) => {
            let (a, b) = edge.absolute_ends();
            points.push(a.point());
            points.push(b.point());
        }
        _ => {
            let curve = edge.oriented_curve();
            let (t0, t1) = curve.range_tuple();
            for k in 0..=EDGE_SAMPLES {
                let u = k as f64 / EDGE_SAMPLES as f64;
                points.push(curve.subs((1.0 - u) * t0 + u * t1));
            }
        }
    }
    points
}
