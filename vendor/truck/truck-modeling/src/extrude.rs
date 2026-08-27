//! BG-SOL-S2-EXTRUDE — direct certified B-rep extrude of a planar arrangement.
//!
//! `extrude_profile` turns the material region of an `Arrangement` (S1,
//! `truck-geometry/src/arrange.rs`) into a closed `Solid<Point3, Curve,
//! Surface>` with NO tool-body Boolean: the bottom/top caps (each carrying the
//! hole's wire as an inner boundary), the outer planar side faces, and the
//! single cylindrical hole wall — built combinatorially with SHARED vertex
//! instances and canonical surfaces. The second half of M1 (certified planar
//! construction, docs/SOLVER_FAMILY_PLAN.md §4 Phase 2 + §7): rectangle −
//! circle → arrangement → profile with hole → direct extrude → valid B-rep.
//!
//! Booked API (plan §4 Phase 2, amended by SPEC_GAP resolution — the §4
//! header already records it): the landed S1 `Arrangement` carries no carrier
//! geometry — `ArrHalfEdge.curve` is an INDEX into the profile slice, and a
//! full circle is not determined by its seam vertex plus a `2π` parameter
//! window — so the profile is a second argument, the same slice the
//! arrangement was built from.
//!
//! v1 scope: exactly one material region (bounded, `winding == 1`, not
//! strictly inside another bounded `winding == 1` region's boundary cycle);
//! `PC = ()` (no pcurves — a documented later refinement). House rules
//! H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::{
    Curve, Cylinder, Edge, Face, Homogeneous, InnerSpace, Line, Plane, Point2, Point3, Processor,
    Shell, Solid, Surface, Vector3, Vertex, Wire, TOLERANCE,
};
use std::collections::HashMap;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};
use truck_geometry::arrange::{ArrHalfEdge, ArrRegion, Arrangement};
use truck_geotrait::ParametricCurve;

/// The number of samples used to polygonize a circle loop for the material
/// representative / containment predicates.
const CIRCLE_SAMPLES: usize = 32;

/// Extrudes the material region(s) of a planar arrangement by `height` along
/// +z into a closed solid. v1 scope: exactly ONE material region (the
/// containment-based rule below).
pub fn extrude_profile(
    profile: &[Curve],
    arrangement: &Arrangement,
    height: f64,
) -> Outcome<Solid> {
    if !height.is_finite() || height <= 0.0 {
        return Err(Refusal::Empty);
    }
    let material_idx = select_material(profile, arrangement)?;
    let material = arrangement
        .regions
        .get(material_idx)
        .ok_or(Refusal::Empty)?;

    // The distinct arrangement vertices on the material boundary cycles.
    let mut v_indices: Vec<usize> = Vec::new();
    for cycle in &material.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if !v_indices.contains(&he.origin) {
                v_indices.push(he.origin);
            }
        }
    }

    // Vertex identity (rule 4 — the load-bearing instance rule): one bottom
    // `Vertex::new(point)` per arrangement vertex of the material boundary
    // (z = 0), and a NEW top `Vertex::new(point + height·ẑ)` per bottom one.
    // Distinct instances for coincident geometric points would leave the shell
    // open (the CE-003-MIGRATE trap).
    let mut bottom_vertex: HashMap<usize, Vertex> = HashMap::new();
    let mut top_vertex: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &v_indices {
        let point = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        bottom_vertex.insert(v_idx, Vertex::new(point));
        top_vertex.insert(v_idx, Vertex::new(point + height * Vector3::unit_z()));
    }

    // Bottom and top boundary edges, built ONCE per cycle and shared by every
    // face that references them (rule 4 again: the cap's rect edge IS the side
    // face's bottom edge IS the same instance).
    let mut cycle_bottom: Vec<Vec<Edge>> = Vec::new();
    let mut cycle_top: Vec<Vec<Edge>> = Vec::new();
    for cycle in &material.boundaries {
        cycle_bottom.push(cycle_bottom_edges(
            cycle,
            profile,
            arrangement,
            &bottom_vertex,
        )?);
        cycle_top.push(cycle_top_edges(
            cycle,
            profile,
            arrangement,
            &top_vertex,
            height,
        )?);
    }

    // Vertical seams (bottom → top), one per boundary vertex, created lazily
    // and reused so two adjacent side faces share the same instance.
    let mut seams: HashMap<usize, Edge> = HashMap::new();
    let mut faces: Vec<Face> = Vec::new();

    // Bottom cap: surface Plane(origin, +x, +y), wires = the material region's
    // boundary cycles in order (outer first, holes after), as the arrangement
    // traced them. The face is stored INVERTED (the multi_sweep seed-face
    // convention): the plane's natural normal is +z, but the outward normal of
    // the solid at z = 0 is −z. Inverting the face also flips its effective
    // boundary edges, which is what the side faces and cylinder pair against.
    let bottom_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ));
    let mut bottom_wires = Vec::new();
    for edges in &cycle_bottom {
        bottom_wires.push(Wire::from(edges.clone()));
    }
    let mut bottom_face =
        Face::try_new(bottom_wires, bottom_surface).map_err(|_| Refusal::Empty)?;
    bottom_face.invert();
    faces.push(bottom_face);

    // Top cap: the SAME cycles translated to z = height, stored in the
    // arrangement's traced direction (NOT reversed) with `orientation == true`,
    // so its outward normal stays +z. The bottom cap is stored inverted, so the
    // two caps' EFFECTIVE boundary edges run opposite — which is exactly what
    // the Closed condition pairs. Built explicitly — never by mapping the
    // bottom wires, because `Wire::mapped` panics on the circle self-loop in
    // debug builds.
    let top_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, height),
        Point3::new(1.0, 0.0, height),
        Point3::new(0.0, 1.0, height),
    ));
    let mut top_wires = Vec::new();
    for edges in &cycle_top {
        top_wires.push(Wire::from(edges.clone()));
    }
    faces.push(Face::try_new(top_wires, top_surface).map_err(|_| Refusal::Empty)?);

    // Side faces and hole walls, one per boundary edge of the material region.
    for (ci, cycle) in material.boundaries.iter().enumerate() {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        let bottom_edges = cycle_bottom.get(ci).ok_or(Refusal::Empty)?;
        let top_edges = cycle_top.get(ci).ok_or(Refusal::Empty)?;
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            match profile.get(he_i.curve) {
                // A line boundary edge extrudes to a planar quad on
                // Plane(a, b, a + height·ẑ) — EXACTLY the recognizer's
                // `ExtrudedCurve(Line) → Plane` mapping, built directly.
                Some(Curve::Line(_)) => {
                    let be = bottom_edges.get(i).ok_or(Refusal::Empty)?;
                    let te = top_edges.get(i).ok_or(Refusal::Empty)?;
                    let seam_o = get_or_create_seam(
                        he_i.origin,
                        height,
                        arrangement,
                        &bottom_vertex,
                        &top_vertex,
                        &mut seams,
                    )?;
                    let seam_n = get_or_create_seam(
                        he_next.origin,
                        height,
                        arrangement,
                        &bottom_vertex,
                        &top_vertex,
                        &mut seams,
                    )?;
                    let a = arrangement
                        .vertices
                        .get(he_i.origin)
                        .ok_or(Refusal::Empty)?
                        .point;
                    let b = arrangement
                        .vertices
                        .get(he_next.origin)
                        .ok_or(Refusal::Empty)?
                        .point;
                    let surface = Surface::Plane(Plane::new(a, b, a + height * Vector3::unit_z()));
                    // The quad [bottom edge, next seam up, top edge reversed,
                    // origin seam down] — the edge instances are SHARED with
                    // the caps (bottom edge with the bottom cap, top edge with
                    // the top cap) and the two adjacent side faces share each
                    // seam (opposite orientation). This pairing matches the
                    // inverted bottom cap and the un-reversed top cap.
                    let wire = Wire::from(vec![be.clone(), seam_n, te.inverse(), seam_o.inverse()]);
                    faces.push(Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?);
                }
                // A circle boundary edge is the wall of the extruded circle:
                // an ANNULUS with two boundary wires (the bottom self-loop, the
                // top self-loop) and NO vertical seam edges. Each circle edge is
                // shared by exactly two faces with opposite orientations
                // (bottom: cap + cylinder; top: cap + cylinder), which is what
                // closes the shell. The wall's orientation is keyed on the
                // cycle's role (arrange.rs): index 0 is the region's OUTER
                // boundary (the pure disk profile), indices ≥ 1 are holes.
                Some(Curve::Circle(p)) => {
                    let be = bottom_edges.get(i).ok_or(Refusal::Empty)?;
                    let te = top_edges.get(i).ok_or(Refusal::Empty)?;
                    // The canonical carrier, read off the profile's `Curve::Circle`
                    // (the canonical.rs conventions).
                    let center = p.transform().w.to_point();
                    let radius = p.transform().x.magnitude();
                    let cylinder = match Cylinder::new(center, radius) {
                        Ok(c) => c.value,
                        Err(_) => return Err(Refusal::Empty),
                    };
                    // Outer boundary: the cylinder's natural +r̂ normal is
                    // already outward, so the face is stored UNINVERTED with the
                    // bottom wire in trace direction and the top wire reversed.
                    // Hole: the hole arm keeps today's form exactly.
                    let (wire_bot, wire_top) = if ci == 0 {
                        (Wire::from(vec![be.clone()]), Wire::from(vec![te.inverse()]))
                    } else {
                        (Wire::from(vec![be.inverse()]), Wire::from(vec![te.clone()]))
                    };
                    let mut cylinder_face =
                        Face::try_new(vec![wire_bot, wire_top], Surface::Cylinder(cylinder))
                            .map_err(|_| Refusal::Empty)?;
                    if ci > 0 {
                        // The hole wall is stored INVERTED: the cylinder's natural
                        // normal is +r (away from the axis) but the outward normal
                        // of the solid at the hole wall is −r (into the hole).
                        // Inverting the face also flips its effective boundary
                        // edges so the caps' circle self-loops pair against them.
                        cylinder_face.invert();
                    }
                    faces.push(cylinder_face);
                }
                _ => return Err(Refusal::Empty),
            }
        }
    }

    // Assembly and validation (rule 6): the shell MUST pass `Solid::try_new` —
    // closed, connected, no singular vertices. If it refuses, the topology is
    // wrong (a missing shared vertex, a reversed wire, a missing face) — never
    // weaken the validation.
    let mut shell = Shell::new();
    for face in faces {
        shell.push(face);
    }
    let solid = match Solid::try_new(vec![shell]) {
        Ok(solid) => solid,
        Err(_) => {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::CoedgePairing,
                left: Truth::True,
                right: Truth::False,
            }));
        }
    };

    let mut props = PropMap::new();
    props.set(Prop::CoedgePairing, Truth::True);
    props.set(Prop::VertexLink, Truth::True);
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        solid,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// Selects the single material region (section 3). A material region is a
/// bounded `ArrRegion` with `winding == 1` that is NOT strictly inside another
/// bounded `winding == 1` region's boundary cycle. v1 accepts exactly one
/// material region; anything else is `Refusal::Empty`.
fn select_material(profile: &[Curve], arrangement: &Arrangement) -> Result<usize, Refusal> {
    let mut found: Option<usize> = None;
    for (idx, region) in arrangement.regions.iter().enumerate() {
        if !region.bounded || region.winding != 1 {
            continue;
        }
        let rep = match region_representative(region, profile, arrangement) {
            Some(p) => p,
            None => return Err(Refusal::Empty),
        };
        let inside_other = arrangement
            .regions
            .iter()
            .enumerate()
            .any(|(other_idx, other)| {
                other_idx != idx
                    && other.bounded
                    && other.winding == 1
                    && other
                        .boundaries
                        .iter()
                        .any(|cycle| point_in_cycle(rep, cycle, profile, arrangement))
            });
        if inside_other {
            continue;
        }
        if found.is_some() {
            return Err(Refusal::Empty);
        }
        found = Some(idx);
    }
    match found {
        Some(idx) => Ok(idx),
        None => Err(Refusal::Empty),
    }
}

/// A representative point of the region's material: strictly inside the outer
/// boundary cycle and strictly outside every hole cycle.
fn region_representative(
    region: &ArrRegion,
    profile: &[Curve],
    arrangement: &Arrangement,
) -> Option<Point2> {
    let outer = region.boundaries.first()?;
    let outer_poly = cycle_polygon(outer, profile, arrangement);
    if outer_poly.is_empty() {
        return None;
    }
    let holes: Vec<Vec<Point2>> = region
        .boundaries
        .iter()
        .skip(1)
        .map(|c| cycle_polygon(c, profile, arrangement))
        .collect();
    let mut candidates = Vec::new();
    if let Some(c) = polygon_centroid(&outer_poly) {
        candidates.push(c);
    }
    // Inward-nudged edge midpoints (the outer cycle is CCW, so the left normal
    // of each edge points into the region).
    let mut first: Option<Point2> = None;
    let mut prev: Option<Point2> = None;
    for cur in &outer_poly {
        if let Some(a) = prev {
            push_left_midpoint(a, *cur, &mut candidates);
        }
        if first.is_none() {
            first = Some(*cur);
        }
        prev = Some(*cur);
    }
    if let (Some(a), Some(b)) = (prev, first) {
        push_left_midpoint(a, b, &mut candidates);
    }
    if let Some((lo, hi)) = bbox_limits(&outer_poly) {
        const GRID: usize = 8;
        for gi in 0..=GRID {
            for gj in 0..=GRID {
                candidates.push(Point2::new(
                    lo.x + (hi.x - lo.x) * (gi as f64 / GRID as f64),
                    lo.y + (hi.y - lo.y) * (gj as f64 / GRID as f64),
                ));
            }
        }
    }
    for c in candidates {
        if point_in_poly(c, &outer_poly) {
            let in_hole = holes.iter().any(|h| point_in_poly(c, h));
            if !in_hole {
                return Some(c);
            }
        }
    }
    None
}

/// Pushes the midpoint of `a→b` nudged along the left normal by the
/// representation tolerance onto `candidates`.
fn push_left_midpoint(a: Point2, b: Point2, candidates: &mut Vec<Point2>) {
    let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    let dir = Vector3::new(b.x - a.x, b.y - a.y, 0.0);
    let left = Vector3::new(-dir.y, dir.x, 0.0);
    let len = left.magnitude();
    if len > 0.0 {
        let nudge = 64.0 * TOLERANCE;
        candidates.push(Point2::new(
            mid.x + left.x / len * nudge,
            mid.y + left.y / len * nudge,
        ));
    }
}

/// The signed-area polygon centroid of a (not necessarily closed) polygon.
fn polygon_centroid(poly: &[Point2]) -> Option<Point2> {
    let mut iter = poly.iter();
    let first = match iter.next() {
        Some(&f) => f,
        None => return None,
    };
    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut prev = first;
    for &cur in iter {
        let cross = prev.x * cur.y - prev.y * cur.x;
        area += cross;
        cx += (prev.x + cur.x) * cross;
        cy += (prev.y + cur.y) * cross;
        prev = cur;
    }
    let cross = prev.x * first.y - prev.y * first.x;
    area += cross;
    cx += (prev.x + first.x) * cross;
    cy += (prev.y + first.y) * cross;
    if area == 0.0 {
        return None;
    }
    Some(Point2::new(cx / (3.0 * area), cy / (3.0 * area)))
}

/// The polygonized parameter-space loop of a boundary cycle: each half-edge is
/// sampled over its parameter window (lines at their endpoints, arcs finely).
fn cycle_polygon(cycle: &[usize], profile: &[Curve], arrangement: &Arrangement) -> Vec<Point2> {
    let mut out = Vec::new();
    for &h in cycle {
        let he = match arrangement.half_edges.get(h) {
            Some(he) => he,
            None => continue,
        };
        let curve = match profile.get(he.curve) {
            Some(c) => c,
            None => continue,
        };
        let (u0, u1) = he.u_range;
        match curve {
            Curve::Line(_) => {
                out.push(pt2(curve.subs(u0)));
                out.push(pt2(curve.subs(u1)));
            }
            Curve::Circle(_) => {
                for k in 0..=CIRCLE_SAMPLES {
                    let t = u0 + (u1 - u0) * (k as f64 / CIRCLE_SAMPLES as f64);
                    out.push(pt2(curve.subs(t)));
                }
            }
            _ => {}
        }
    }
    out
}

/// Whether the point `p` is strictly inside the polygonized cycle (nonzero
/// winding / odd parity).
fn point_in_cycle(
    p: Point2,
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
) -> bool {
    let poly = cycle_polygon(cycle, profile, arrangement);
    point_in_poly(p, &poly)
}

/// Even-odd point-in-polygon by horizontal ray casting, without any indexing.
fn point_in_poly(p: Point2, poly: &[Point2]) -> bool {
    let mut inside = false;
    let mut iter = poly.iter();
    let first = match iter.next() {
        Some(&f) => f,
        None => return false,
    };
    let mut prev = first;
    for &cur in iter {
        if (prev.y > p.y) != (cur.y > p.y) {
            let x_cross = (cur.x - prev.x) * (p.y - prev.y) / (cur.y - prev.y) + prev.x;
            if p.x < x_cross {
                inside = !inside;
            }
        }
        prev = cur;
    }
    if (prev.y > p.y) != (first.y > p.y) {
        let x_cross = (first.x - prev.x) * (p.y - prev.y) / (first.y - prev.y) + prev.x;
        if p.x < x_cross {
            inside = !inside;
        }
    }
    inside
}

/// The bounding-box limits of a polygon, `None` if any coordinate is non-finite.
fn bbox_limits(poly: &[Point2]) -> Option<(Point2, Point2)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in poly {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return None;
    }
    Some((Point2::new(min_x, min_y), Point2::new(max_x, max_y)))
}

/// The bottom boundary edges of a cycle, in cycle order: `origin(h_i) →
/// origin(h_{i+1})`. A line piece gets a `Curve::Line(Line(a, b))` from the
/// arrangement vertex points; a circle piece keeps the profile's `Curve::Circle`
/// processor.
fn cycle_bottom_edges(
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
    bottom_vertex: &HashMap<usize, Vertex>,
) -> Result<Vec<Edge>, Refusal> {
    let n = cycle.len();
    if n == 0 {
        return Err(Refusal::Empty);
    }
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
        let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
        let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
        let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
        let v0 = bottom_vertex.get(&he_i.origin).ok_or(Refusal::Empty)?;
        let v1 = bottom_vertex.get(&he_next.origin).ok_or(Refusal::Empty)?;
        let curve = bottom_edge_curve(he_i, profile, arrangement)?;
        let edge = match profile.get(he_i.curve) {
            // The closed circle edge's front and back are the SAME vertex; the
            // self-loop IS the seam, and `Edge::new_unchecked` is the
            // sanctioned construction (the BG-TOL-001-MESHALGO precedent).
            Some(Curve::Circle(_)) => Edge::new_unchecked(v0, v1, curve),
            _ => Edge::try_new(v0, v1, curve).map_err(|_| Refusal::Empty)?,
        };
        edges.push(edge);
    }
    Ok(edges)
}

/// The top boundary edges of a cycle, translated to z = height.
fn cycle_top_edges(
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
    top_vertex: &HashMap<usize, Vertex>,
    height: f64,
) -> Result<Vec<Edge>, Refusal> {
    let n = cycle.len();
    if n == 0 {
        return Err(Refusal::Empty);
    }
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
        let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
        let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
        let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
        let v0 = top_vertex.get(&he_i.origin).ok_or(Refusal::Empty)?;
        let v1 = top_vertex.get(&he_next.origin).ok_or(Refusal::Empty)?;
        let curve = top_edge_curve(he_i, profile, arrangement, height)?;
        let edge = match profile.get(he_i.curve) {
            // The closed circle edge's front and back are the SAME vertex; the
            // self-loop IS the seam, and `Edge::new_unchecked` is the
            // sanctioned construction (the BG-TOL-001-MESHALGO precedent).
            Some(Curve::Circle(_)) => Edge::new_unchecked(v0, v1, curve),
            _ => Edge::try_new(v0, v1, curve).map_err(|_| Refusal::Empty)?,
        };
        edges.push(edge);
    }
    Ok(edges)
}

/// The bottom edge curve of a half-edge: a line from the two endpoint points,
/// or the profile's circle processor.
fn bottom_edge_curve(
    he: &ArrHalfEdge,
    profile: &[Curve],
    arrangement: &Arrangement,
) -> Result<Curve, Refusal> {
    match profile.get(he.curve) {
        Some(Curve::Line(_)) => {
            let p0 = arrangement
                .vertices
                .get(he.origin)
                .ok_or(Refusal::Empty)?
                .point;
            let twin = arrangement.half_edges.get(he.twin).ok_or(Refusal::Empty)?;
            let p1 = arrangement
                .vertices
                .get(twin.origin)
                .ok_or(Refusal::Empty)?
                .point;
            Ok(Curve::Line(Line(p0, p1)))
        }
        Some(Curve::Circle(p)) => Ok(Curve::Circle(*p)),
        _ => Err(Refusal::Empty),
    }
}

/// The top edge curve of a half-edge: the bottom line translated by height·ẑ,
/// or the profile's circle processor placed at z = height.
fn top_edge_curve(
    he: &ArrHalfEdge,
    profile: &[Curve],
    arrangement: &Arrangement,
    height: f64,
) -> Result<Curve, Refusal> {
    match profile.get(he.curve) {
        Some(Curve::Line(_)) => {
            let p0 = arrangement
                .vertices
                .get(he.origin)
                .ok_or(Refusal::Empty)?
                .point
                + height * Vector3::unit_z();
            let twin = arrangement.half_edges.get(he.twin).ok_or(Refusal::Empty)?;
            let p1 = arrangement
                .vertices
                .get(twin.origin)
                .ok_or(Refusal::Empty)?
                .point
                + height * Vector3::unit_z();
            Ok(Curve::Line(Line(p0, p1)))
        }
        Some(Curve::Circle(p)) => {
            let mut m = *p.transform();
            m.w.z += height;
            Ok(Curve::Circle(Processor::with_transform(*p.entity(), m)))
        }
        _ => Err(Refusal::Empty),
    }
}

/// The vertical seam edge (bottom → top) of a boundary vertex, created once and
/// reused by the two adjacent side faces (rule 4).
fn get_or_create_seam(
    v_idx: usize,
    height: f64,
    arrangement: &Arrangement,
    bottom_vertex: &HashMap<usize, Vertex>,
    top_vertex: &HashMap<usize, Vertex>,
    seams: &mut HashMap<usize, Edge>,
) -> Result<Edge, Refusal> {
    if let Some(e) = seams.get(&v_idx) {
        return Ok(e.clone());
    }
    let b = bottom_vertex.get(&v_idx).ok_or(Refusal::Empty)?;
    let t = top_vertex.get(&v_idx).ok_or(Refusal::Empty)?;
    let p0 = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
    let p1 = p0 + height * Vector3::unit_z();
    let edge = Edge::try_new(b, t, Curve::Line(Line(p0, p1))).map_err(|_| Refusal::Empty)?;
    seams.insert(v_idx, edge.clone());
    Ok(edge)
}

/// The 2-D (x, y) projection of a 3-D point.
fn pt2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{Matrix4, ShellCondition, TrimmedCurve, UnitCircle, Vector4};
    use std::f64::consts::TAU;
    use truck_geometry::arrange::arrange;
    use truck_geometry::recognize::{
        recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalSurface,
    };
    use truck_geotrait::BoundedCurve;

    /// The M1 profile: a 4×4 CCW rectangle plus a full circle r = 1 at (2, 2)
    /// in its natural (CCW) parameterization. The material selection is
    /// containment-based, so the circle's orientation is NOT required to be
    /// reversed. Returns the profile slice AND its arrangement.
    fn plate_with_hole() -> (Vec<Curve>, Arrangement) {
        let circle = Curve::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            Matrix4 {
                x: Vector4::new(1.0, 0.0, 0.0, 0.0),
                y: Vector4::new(0.0, 1.0, 0.0, 0.0),
                z: Vector4::new(0.0, 0.0, 1.0, 0.0),
                w: Vector4::new(2.0, 2.0, 0.0, 1.0),
            },
        ));
        let profile = vec![
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
            circle,
        ];
        let ok = arrange(&profile, None).unwrap();
        let arrangement = ok.value;
        (profile, arrangement)
    }

    /// A point-in-solid winding test over the closed boundary: cast a ray from
    /// `point` along +z, count the transversal crossings of each face's
    /// interior, and return whether the winding number is nonzero. The plate
    /// with hole is a torus, so parity alone would not decide — the signed
    /// winding does.
    fn point_in_solid(solid: &Solid, point: Point3) -> bool {
        let d = Vector3::new(0.0, 0.0, 1.0);
        let mut winding = 0i32;
        for face in solid.face_iter() {
            let surface = face.surface();
            for (t, q) in face_ray_crossings(&surface, point, d) {
                if t <= TOLERANCE {
                    continue;
                }
                if !point_in_face(&surface, face, q) {
                    continue;
                }
                let n = surface_normal_at(&surface, q);
                let n = if face.orientation() { n } else { -n };
                let sign = if d.dot(n) > 0.0 { -1 } else { 1 };
                winding += sign;
            }
        }
        winding != 0
    }

    /// The ray-surface crossings of `p + t·d` with an analytic surface.
    fn face_ray_crossings(surface: &Surface, p: Point3, d: Vector3) -> Vec<(f64, Point3)> {
        match surface {
            Surface::Plane(plane) => {
                let n = plane.normal();
                let denom = d.dot(n);
                if denom.abs() < TOLERANCE {
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
                if a < TOLERANCE {
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
            _ => Vec::new(),
        }
    }

    /// Whether the crossing point `q` lies strictly inside the face's bounded
    /// region in the surface's parameter space (inside the outer boundary
    /// loop, outside every inner loop; for a cylinder annulus, between the two
    /// boundary self-loops' v-values).
    fn point_in_face(surface: &Surface, face: &Face, q: Point3) -> bool {
        let (u, v) = match surface {
            Surface::Plane(plane) => {
                let prm = plane.get_parameter(q);
                (prm.x, prm.y)
            }
            Surface::Cylinder(cyl) => {
                let c = cyl.center();
                let u = f64::atan2(q.y - c.y, q.x - c.x);
                (u, q.z - c.z)
            }
            _ => return false,
        };
        let mut loops: Vec<Vec<Point2>> = Vec::new();
        for wire in face.boundaries() {
            let mut lp = Vec::new();
            for edge in wire.edge_iter() {
                let curve = edge.curve();
                match curve {
                    Curve::Line(Line(a, b)) => {
                        lp.push(sample_params(surface, a));
                        lp.push(sample_params(surface, b));
                    }
                    Curve::Circle(p) => {
                        let (t0, t1) = p.range_tuple();
                        for k in 0..=CIRCLE_SAMPLES {
                            let t = t0 + (t1 - t0) * (k as f64 / CIRCLE_SAMPLES as f64);
                            lp.push(sample_params(surface, p.subs(t)));
                        }
                    }
                    _ => {}
                }
            }
            loops.push(lp);
        }
        match surface {
            Surface::Cylinder(_) => {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for lp in &loops {
                    for (_, vv) in lp.iter().map(|&pt| (pt.x, pt.y)) {
                        lo = lo.min(vv);
                        hi = hi.max(vv);
                    }
                }
                v > lo && v < hi
            }
            _ => match loops.first() {
                None => false,
                Some(outer) => {
                    point_in_poly(Point2::new(u, v), outer)
                        && loops
                            .iter()
                            .skip(1)
                            .all(|lp| !point_in_poly(Point2::new(u, v), lp))
                }
            },
        }
    }

    /// The outward normal of a surface at a point on it.
    fn surface_normal_at(surface: &Surface, q: Point3) -> Vector3 {
        match surface {
            Surface::Plane(plane) => plane.normal(),
            Surface::Cylinder(cyl) => {
                let r = q - cyl.center();
                let n = Vector3::new(r.x, r.y, 0.0);
                let len = n.magnitude();
                if len == 0.0 {
                    Vector3::unit_z()
                } else {
                    n / len
                }
            }
            _ => Vector3::unit_z(),
        }
    }

    /// The surface parameter pair of a point on the surface.
    fn sample_params(surface: &Surface, pt: Point3) -> Point2 {
        match surface {
            Surface::Plane(plane) => {
                let prm = plane.get_parameter(pt);
                Point2::new(prm.x, prm.y)
            }
            Surface::Cylinder(cyl) => {
                let c = cyl.center();
                Point2::new(f64::atan2(pt.y - c.y, pt.x - c.x), pt.z - c.z)
            }
            _ => Point2::new(0.0, 0.0),
        }
    }

    /// The `(center, radius)` of an exact-canonical cylinder witness.
    fn exact_cylinder(witness: &CanonicalCarrierWitness) -> Option<(Point3, f64)> {
        match witness {
            CanonicalCarrierWitness::ExactCanonical {
                carrier: CanonicalCarrier::Surface(CanonicalSurface::Cylinder(cyl)),
                ..
            } => Some((cyl.center(), cyl.radius())),
            _ => None,
        }
    }

    #[test]
    fn extrude_plate_with_hole_is_a_closed_solid() {
        let (profile, arrangement) = plate_with_hole();
        let solid = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;
        // The solid was built through `Solid::try_new`; re-assert the three
        // closure conditions directly.
        let shell = solid.boundaries().first().expect("one boundary shell");
        assert_eq!(shell.shell_condition(), ShellCondition::Closed);
        assert!(shell.is_connected());
        assert!(shell.singular_vertices().is_empty());
        // A point in the plate material is inside; a point in the hole's air
        // column (the hole runs through the whole height) is not.
        assert!(point_in_solid(&solid, Point3::new(1.0, 1.0, 1.0)));
        assert!(!point_in_solid(&solid, Point3::new(2.0, 2.0, 1.0)));
    }

    #[test]
    fn extrude_plate_hole_wall_is_a_cylinder() {
        let (profile, arrangement) = plate_with_hole();
        let solid = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;
        let mut cylinders: Vec<Cylinder> = Vec::new();
        for face in solid.face_iter() {
            let surface = face.surface();
            if let Surface::Cylinder(cyl) = surface {
                cylinders.push(cyl);
            }
        }
        assert_eq!(cylinders.len(), 1);
        let cyl = cylinders.first().expect("one cylinder wall");
        // The carrier read off the profile's `Curve::Circle` (the section 5
        // construction): center (2,2,0), radius 1.0.
        assert_eq!(cyl.center(), Point3::new(2.0, 2.0, 0.0));
        assert_eq!(cyl.radius(), 1.0);
        // The recognizer verifies the canonical carrier — the plan's
        // "canonicalization: recognize (circle × straight path) => Cylinder"
        // exercised as a test, not a second code path.
        let witness = recognize_surface(&Surface::Cylinder(*cyl));
        let (center, radius) =
            exact_cylinder(&witness).expect("expected an exact canonical cylinder witness");
        assert_eq!(center, Point3::new(2.0, 2.0, 0.0));
        assert_eq!(radius, 1.0);
    }

    #[test]
    fn extrude_face_and_edge_counts_are_exact() {
        let (profile, arrangement) = plate_with_hole();
        let solid = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;
        let mut planes = 0usize;
        let mut cylinders = 0usize;
        let mut caps = 0usize;
        for face in solid.face_iter() {
            let surface = face.surface();
            match surface {
                Surface::Plane(_) => {
                    planes += 1;
                    // The bottom/top caps each have 2 boundary wires: the outer
                    // rectangle wire with 4 edges and the inner circle wire
                    // with 1 edge.
                    let wires = face.boundaries();
                    if wires.len() == 2
                        && wires.first().map(|w| w.len()) == Some(4)
                        && wires.get(1).map(|w| w.len()) == Some(1)
                    {
                        caps += 1;
                    }
                }
                Surface::Cylinder(_) => {
                    cylinders += 1;
                    // The cylinder annulus has the same two circle self-loops
                    // as its two boundary wires.
                    let wires = face.boundaries();
                    assert_eq!(wires.len(), 2);
                    assert!(wires.iter().all(|w| w.len() == 1));
                }
                _ => {}
            }
        }
        // 1 bottom + 1 top + 4 rect sides + 1 cylinder annulus.
        assert_eq!(planes, 6);
        assert_eq!(cylinders, 1);
        assert_eq!(caps, 2);
    }

    #[test]
    fn extrude_zero_or_negative_height_is_refused() {
        let (profile, arrangement) = plate_with_hole();
        assert!(extrude_profile(&profile, &arrangement, 0.0).is_err());
        assert!(extrude_profile(&profile, &arrangement, -1.0).is_err());
    }

    #[test]
    fn extrude_all_face_normals_point_outward() {
        const EPS: f64 = 1.0e-3; // H-3: step from each face into/out of the material in the regression test
        let (profile, arrangement) = plate_with_hole();
        let solid = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;
        let mut checked = 0usize;
        for face in solid.face_iter() {
            let surface = face.surface();
            // A strictly-interior sample point `q` of the face's domain and the
            // direction the outward normal of the solid must take there.
            let (q, expected) = match &surface {
                Surface::Plane(plane) => {
                    let o = plane.origin();
                    let is_cap = face.boundaries().len() == 2;
                    if is_cap && o.z == 0.0 {
                        (Point3::new(1.0, 1.0, 0.0), Vector3::new(0.0, 0.0, -1.0))
                    } else if is_cap && o.z == 2.0 {
                        (Point3::new(1.0, 1.0, 2.0), Vector3::new(0.0, 0.0, 1.0))
                    } else if o.x == 0.0 && o.y == 0.0 {
                        (Point3::new(1.0, 0.0, 1.0), Vector3::new(0.0, -1.0, 0.0))
                    } else if o.x == 4.0 && o.y == 0.0 {
                        (Point3::new(4.0, 1.0, 1.0), Vector3::new(1.0, 0.0, 0.0))
                    } else if o.x == 4.0 && o.y == 4.0 {
                        (Point3::new(1.0, 4.0, 1.0), Vector3::new(0.0, 1.0, 0.0))
                    } else if o.x == 0.0 && o.y == 4.0 {
                        (Point3::new(0.0, 1.0, 1.0), Vector3::new(-1.0, 0.0, 0.0))
                    } else {
                        unreachable!("unrecognized plane face at origin {o:?}");
                    }
                }
                Surface::Cylinder(_) => (Point3::new(3.0, 2.0, 1.0), Vector3::new(-1.0, 0.0, 0.0)),
                _ => {
                    unreachable!("unexpected surface {surface:?}");
                }
            };
            let n_eff = if face.orientation() {
                surface_normal_at(&surface, q)
            } else {
                -surface_normal_at(&surface, q)
            };
            assert!(
                n_eff.dot(expected) > 0.9,
                "face normal {n_eff:?} does not point outward; expected ~{expected:?}"
            );
            // The load-bearing check: stepping from the face INTO the material
            // (along −n_eff) lands inside the solid; stepping OUT (along +n_eff)
            // lands outside it.
            assert!(point_in_solid(&solid, q - EPS * n_eff));
            assert!(!point_in_solid(&solid, q + EPS * n_eff));
            checked += 1;
        }
        assert_eq!(checked, 7);
    }

    /// The M2 disk: the SAME circle as `plate_with_hole`, extruded ALONE. The
    /// circle cycle is the material region's OUTER boundary, so the cylinder
    /// wall must carry the cylinder's natural +r̂ normal (orientation == true)
    /// and NOT the hole convention (BG-SOL-S2-DISK-ORIENT).
    #[test]
    fn extrude_disk_wall_normal_points_outward() {
        let circle = Curve::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            Matrix4 {
                x: Vector4::new(1.0, 0.0, 0.0, 0.0),
                y: Vector4::new(0.0, 1.0, 0.0, 0.0),
                z: Vector4::new(0.0, 0.0, 1.0, 0.0),
                w: Vector4::new(2.0, 2.0, 0.0, 1.0),
            },
        ));
        let profile = vec![circle];
        let arrangement = arrange(&profile, None).unwrap().value;
        let solid = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;

        // 3 faces: one cylinder wall, two planar caps.
        let mut cylinders: Vec<Face> = Vec::new();
        let mut planes: Vec<Face> = Vec::new();
        for face in solid.face_iter() {
            match face.surface() {
                Surface::Cylinder(_) => cylinders.push(face.clone()),
                Surface::Plane(_) => planes.push(face.clone()),
                _ => unreachable!("unexpected surface"),
            }
        }
        assert_eq!(cylinders.len(), 1);
        assert_eq!(planes.len(), 2);

        // The wall is stored UNINVERTED: orientation == true, effective normal
        // +x̂ at (3, 2, 1) — the natural radial normal, away from the material.
        let wall = cylinders.first().expect("one cylinder wall");
        assert!(
            wall.orientation(),
            "the disk's cylinder wall must carry orientation == true"
        );
        let surface = wall.surface();
        let q = Point3::new(3.0, 2.0, 1.0);
        let n_eff = if wall.orientation() {
            surface_normal_at(&surface, q)
        } else {
            -surface_normal_at(&surface, q)
        };
        let expected = Vector3::new(1.0, 0.0, 0.0);
        assert!(
            (n_eff - expected).magnitude() < TOLERANCE,
            "wall effective normal {n_eff:?} must be +x̂ at (3, 2, 1)"
        );

        // The caps: bottom stored inverted (orientation == false), top not.
        for cap in &planes {
            let Surface::Plane(plane) = cap.surface() else {
                unreachable!("cap surface is not a plane");
            };
            if plane.origin().z == 0.0 {
                assert!(!cap.orientation(), "bottom cap must be stored inverted");
            } else {
                assert!(cap.orientation(), "top cap must not be inverted");
            }
        }

        // Inside the disk material (r < 1 at z = 1) and outside it.
        assert!(point_in_solid(&solid, Point3::new(2.0, 2.0, 1.0)));
        assert!(!point_in_solid(&solid, Point3::new(5.0, 5.0, 1.0)));

        // The boundary shell re-passes the closure validation.
        let shell = solid.boundaries().first().expect("one boundary shell");
        assert!(Solid::try_new(vec![shell.clone()]).is_ok());
    }
}
