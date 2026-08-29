//! BG-CAD-P4-UNTIL — the Phase 7 sweep reduction: `until` + `project`.
//!
//! build123d's `extrude(until=...)` and `project`, decomposed as the plan's
//! "swept Contact + certified-t ordering + rewrite": the swept curtain of a
//! line/circle profile is canonical (table 6.1 row 1), the landed exact FF
//! arms answer curtain × target-plane pairs (lines for plane walls), and the
//! termination is a closed-form rewrite. No new solving machinery.
//!
//! Frame conventions are the extrude family's: the profile's material region
//! lies in the z = 0 plane and exactly ONE material region is accepted
//! (`Empty` otherwise, mirroring `extrude_interval`'s guard). The certified
//! crossing parameter of a point `p` against the target plane Π (origin `o`,
//! unit normal `n`) is `t(p) = (n·o − n·p) / (n·dir)`; the truncated solid is
//! the sweep over `t ∈ [0, t(p)]` pointwise — the prism cut by the halfspace
//! `{x : n·x ≤ n·o}` when `n·dir > 0` (the sense mirrors when negative; the
//! two cases are machine-checked below).
//!
//! - A **parallel target** (`n ∥ dir`) is the §9 metamorphic case: `t*` is
//!   uniform and the solid is exactly the landed
//!   `extrude_profile_vector(profile, arrangement, t*·dir, false)` — the
//!   identity is structural, certified by the metamorphic test against a
//!   direct call.
//! - An **oblique target** over a line-edge convex region: every curtain wall
//!   is a Plane, so every wall × Π termination locus is a Line (the landed
//!   `plane_plane` exact arm, called through `truck-evidence::contact`), the
//!   oblique cap is the planar polygon in Π bounded by the termination lines,
//!   and the bottom cap / walls / cap are assembled combinatorially with
//!   shared vertex instances; `Solid::try_new` is the acceptance gate.
//! - **Refusals (typed, zero new arms):** a non-convex region boundary, a
//!   circle edge with an oblique target (the termination would be an
//!   Ellipse — the RW-CONIC boundary), or a parallel sweep that never
//!   terminates. With a parallel target a circle profile is fine: the parallel
//!   case rides the landed extrude, which already handles circle walls.
//!
//! `project_profile` returns the projected boundary of the region onto Π along
//! `dir` — the same termination loci, as curves. A parallel target translates
//! the profile by `t*·dir` (translation preserves the `Curve` type); an
//! oblique target maps each Line edge to the Line between its endpoints'
//! images, and refuses a Circle edge as above. The returned carriers are
//! canonical (`Line`/`Circle` only) — that IS the refusal rule, not a
//! post-check.
//!
//! House rules H-1..H-8 apply; every topology is built through the validated
//! construction recipes that pass `Solid::try_new`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::extrude::extrude_profile_vector;
use crate::{
    Curve, Edge, Face, InnerSpace, Line, Plane, Point2, Point3, Processor, Shell, Solid, Surface,
    Vector3, Vertex, Wire,
};
use std::collections::HashMap;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
use truck_evidence::contact::{contact, BoundedStratum, ContactLocus};
use truck_geometry::arrange::{ArrRegion, Arrangement};
use truck_geometry::recognize::{
    recognize_curve, recognize_surface, CanonicalCarrierWitness, CanonicalSurface,
};
use truck_geotrait::ParametricCurve;

/// The number of samples used to polygonize a circle loop for the material
/// representative / containment predicates.
const CIRCLE_SAMPLES: usize = 32;

/// The certified sweep target. v1: planes only.
#[derive(Clone, Copy, Debug)]
pub enum Until {
    /// A target plane; the sweep terminates where the region's leading front
    /// reaches it.
    Plane(Plane),
}

/// The dimensionless slack of the closed-form plane-residual machine-checks
/// (BG-NUM-002 applies to the `t` values exactly as to geometry): the signed
/// crossing parameters of the dyadic witnesses satisfy the plane equation to
/// exactly zero, so a residual beyond this slack is a witness failure, never
/// a rounding noise.
const PLANE_RESIDUAL_SLACK: f64 = 1.0e-9; // H-3: dimensionless plane-residual slack of unit-scale witnesses

/// The length slack of the "cap vertex lies on the certified termination
/// line" machine-checks (D4): the cap vertices of the dyadic witnesses are
/// exactly on the contact lines, so a point-to-line distance beyond this
/// slack is a witness failure.
const TERMINATION_RESIDUAL_SLACK: f64 = 1.0e-9; // H-3: length slack on unit-scale witness coordinates

/// Extrudes the material region of a planar arrangement along `dir`, stopping
/// where it reaches the certified target.
///
/// - A non-finite `dir`, `dir.z == 0` (the landed vector convention), or a
///   target plane parallel to the sweep direction (`n · dir == 0`) refuses
///   `Empty`: the sweep never terminates on the plane.
/// - If no boundary point of the region crosses along `dir` in the positive
///   direction (the plane is behind the profile: every `t(p) < 0`), the sweep
///   has no termination and refuses `Empty`.
/// - A parallel target (`n ∥ dir`) is the §9 metamorphic case: the certified
///   height `t*` is uniform and the solid is exactly the landed
///   `extrude_profile_vector` construction at `t*·dir`.
/// - An oblique target requires a strictly convex region boundary of line
///   edges; a reflex vertex or a circle edge refuses
///   `UnsupportedEnvelope(NonCanonicalCarrier)`. The cap face's Plane data
///   equals the target's exactly (the same `Plane` value).
pub fn extrude_until(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    target: &Until,
) -> Outcome<Solid> {
    let (pi, denom) = sweep_gates(dir, target)?;
    let n = pi.normal();
    let o = pi.origin();
    let material_idx = select_material(profile, arrangement)?;
    let material = arrangement
        .regions
        .get(material_idx)
        .ok_or(Refusal::Empty)?;
    let ts = boundary_ts(material, arrangement, dir, n, o, denom)?;
    // The plane is behind the profile along +dir: no termination.
    if ts.values().all(|&t| t < 0.0) {
        return Err(Refusal::Empty);
    }
    // Parallel target: the §9 metamorphic case. t* is uniform; the solid is
    // exactly the landed extrude at the certified height component.
    let cross = n.cross(dir);
    let parallel = cross.x == 0.0 && cross.y == 0.0 && cross.z == 0.0;
    if parallel {
        let mut t_star: Option<f64> = None;
        for &t in ts.values() {
            match t_star {
                Some(prev) if prev != t => {
                    // The claimed uniform t* contradicts the geometry: an
                    // unbooked parallel-target case (the plane is not
                    // perpendicular to the sweep over the whole region).
                    return Err(Refusal::Empty);
                }
                Some(_) => {}
                None => t_star = Some(t),
            }
        }
        let t_star = match t_star {
            Some(t) => t,
            None => return Err(Refusal::Empty),
        };
        let h = t_star * dir;
        if !h.x.is_finite() || !h.y.is_finite() || !h.z.is_finite() || h.z == 0.0 {
            return Err(Refusal::Empty);
        }
        return extrude_profile_vector(profile, arrangement, h, false);
    }

    // Oblique target. A circle boundary edge would terminate in an Ellipse
    // (the RW-CONIC boundary) — refuse at the lift, before any cap math.
    for cycle in &material.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if matches!(profile.get(he.curve), Some(Curve::Circle(_))) {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
        }
    }
    // A reflex vertex anywhere on the region boundary: the cap polygon's
    // region-structure is not v1.
    for cycle in &material.boundaries {
        let pts = cycle_vertex_points(cycle, arrangement)?;
        if !strictly_convex_ccw(&pts) {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
    }
    // A boundary point starting on the far side of Π (t < 0) trims the
    // bottom cap via the halfspace cut — a trimmed-cap construction that is a
    // booked follow-up. The D4 construction (bottom = the full region face)
    // is sound only when every boundary t >= 0.
    for &t in ts.values() {
        if t < 0.0 {
            return Err(Refusal::Empty);
        }
    }

    // Shared vertex instances (rule 4): one bottom vertex at z = 0 and one
    // top vertex on Π per arrangement vertex of the material boundary.
    let mut bottom_vertex: HashMap<usize, Vertex> = HashMap::new();
    let mut top_vertex: HashMap<usize, Vertex> = HashMap::new();
    for (&v_idx, &t) in &ts {
        let p = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        bottom_vertex.insert(v_idx, Vertex::new(p));
        top_vertex.insert(v_idx, Vertex::new(p + t * dir));
    }

    // The certified termination lines (D4): every wall × Π pair answers with
    // a Line through the landed contact(). Machine-check each cap vertex on
    // BOTH adjacent termination lines and in Π.
    for cycle in &material.boundaries {
        let n_edges = cycle.len();
        if n_edges == 0 {
            return Err(Refusal::Empty);
        }
        let mut lines: Vec<Line<Point3>> = Vec::with_capacity(n_edges);
        for i in 0..n_edges {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n_edges).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
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
            let a_top = top_vertex.get(&he_i.origin).ok_or(Refusal::Empty)?.point();
            let wall = Plane::new(a, b, a_top);
            lines.push(termination_line(wall, pi)?);
        }
        for i in 0..n_edges {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let p = arrangement
                .vertices
                .get(he_i.origin)
                .ok_or(Refusal::Empty)?
                .point;
            let t = ts.get(&he_i.origin).copied().ok_or(Refusal::Empty)?;
            let cap_v = p + t * dir;
            let prev = lines
                .get((i + n_edges - 1) % n_edges)
                .ok_or(Refusal::Empty)?;
            let next = lines.get(i).ok_or(Refusal::Empty)?;
            if prev.distance_to_point(cap_v) > TERMINATION_RESIDUAL_SLACK
                || next.distance_to_point(cap_v) > TERMINATION_RESIDUAL_SLACK
                || (cap_v - o).dot(n).abs() > PLANE_RESIDUAL_SLACK
            {
                return Err(Refusal::Empty);
            }
        }
    }

    // Bottom cap: the region's boundary cycles on the z = 0 plane, stored
    // INVERTED (the extrude seed-face convention).
    let mut faces: Vec<Face> = Vec::new();
    let bottom_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ));
    let mut cycle_bottom: Vec<Vec<Edge>> = Vec::new();
    for cycle in &material.boundaries {
        cycle_bottom.push(cycle_boundary_edges(cycle, arrangement, &bottom_vertex)?);
    }
    let mut bottom_wires = Vec::new();
    for edges in &cycle_bottom {
        bottom_wires.push(Wire::from(edges.clone()));
    }
    let mut bottom_face =
        Face::try_new(bottom_wires, bottom_surface).map_err(|_| Refusal::Empty)?;
    bottom_face.invert();
    faces.push(bottom_face);

    // The termination edges (one per boundary edge, on Π), shared by the cap
    // wire and the walls' top segments.
    let mut cycle_top: Vec<Vec<Edge>> = Vec::new();
    for cycle in &material.boundaries {
        let n_edges = cycle.len();
        if n_edges == 0 {
            return Err(Refusal::Empty);
        }
        let mut edges = Vec::with_capacity(n_edges);
        for i in 0..n_edges {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n_edges).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let a_top = top_vertex.get(&he_i.origin).ok_or(Refusal::Empty)?;
            let b_top = top_vertex.get(&he_next.origin).ok_or(Refusal::Empty)?;
            let curve = Curve::Line(Line(a_top.point(), b_top.point()));
            edges.push(Edge::try_new(a_top, b_top, curve).map_err(|_| Refusal::Empty)?);
        }
        cycle_top.push(edges);
    }

    // Oblique cap: the planar polygon in Π bounded by the termination lines,
    // on the target plane EXACTLY (the same Plane value).
    let cap_surface = Surface::Plane(pi);
    let mut cap_wires = Vec::new();
    for edges in &cycle_top {
        cap_wires.push(Wire::from(edges.clone()));
    }
    faces.push(Face::try_new(cap_wires, cap_surface).map_err(|_| Refusal::Empty)?);

    // Curtain walls, one per boundary edge: the planar quad [bottom edge, next
    // seam up, termination segment reversed, origin seam down] on
    // Plane(a, b, a_top). The seam edges are shared with the two adjacent
    // walls; the bottom edge with the bottom cap; the termination segment with
    // the cap.
    let cycle_holes: Vec<bool> = material
        .boundaries
        .iter()
        .enumerate()
        .map(|(ci, _)| cycle_is_hole(&material.boundaries, ci, profile, arrangement))
        .collect();
    let mut seams: HashMap<usize, Edge> = HashMap::new();
    for (ci, cycle) in material.boundaries.iter().enumerate() {
        let n_edges = cycle.len();
        if n_edges == 0 {
            return Err(Refusal::Empty);
        }
        let bottom_edges = cycle_bottom.get(ci).ok_or(Refusal::Empty)?;
        let top_edges = cycle_top.get(ci).ok_or(Refusal::Empty)?;
        let is_hole = match cycle_holes.get(ci) {
            Some(is_hole) => *is_hole,
            None => false,
        };
        for i in 0..n_edges {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n_edges).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let be = bottom_edges.get(i).ok_or(Refusal::Empty)?;
            let te = top_edges.get(i).ok_or(Refusal::Empty)?;
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
            let a_top = top_vertex.get(&he_i.origin).ok_or(Refusal::Empty)?.point();
            let b_top = top_vertex
                .get(&he_next.origin)
                .ok_or(Refusal::Empty)?
                .point();
            let surface = Surface::Plane(Plane::new(a, b, a_top));
            let dot = (b_top - a_top).dot(b - a);
            if dot == 0.0 {
                return Err(Refusal::Empty);
            }
            let forward = dot > 0.0;
            let top_segment = if forward { te.inverse() } else { te.clone() };
            let seam_o = get_or_create_seam(he_i.origin, &bottom_vertex, &top_vertex, &mut seams)?;
            let seam_n =
                get_or_create_seam(he_next.origin, &bottom_vertex, &top_vertex, &mut seams)?;
            let wire = Wire::from(vec![be.clone(), seam_n, top_segment, seam_o.inverse()]);
            let mut wall = Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?;
            if is_hole {
                wall.invert();
            }
            faces.push(wall);
        }
    }

    // Certificates (D5): every emitted carrier must be recognized as
    // canonical — the construction above cannot produce anything else, so an
    // unrecognized carrier is an envelope refusal, never a silent generic
    // surface (defensive).
    for face in &faces {
        if matches!(
            recognize_surface(&face.surface()),
            CanonicalCarrierWitness::Unrecognized
        ) {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ));
        }
        for wire in face.boundaries() {
            for edge in wire.edge_iter() {
                if matches!(
                    recognize_curve(&edge.curve()),
                    CanonicalCarrierWitness::Unrecognized
                ) {
                    return Err(Refusal::UnsupportedEnvelope(
                        EnvelopeCase::NonCanonicalCarrier,
                    ));
                }
            }
        }
    }

    // Assembly and validation (rule 6): the shell MUST pass `Solid::try_new`.
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

/// Projects the material region's boundary onto the certified target along
/// `dir`, as curves — the same termination loci `extrude_until` sweeps to.
///
/// - A parallel target translates each boundary curve by `t*·dir`;
///   translation preserves the `Curve` type, so lines and circles alike.
/// - An oblique target maps each Line edge to the Line between its endpoints'
///   images (the closed form); a Circle edge refuses
///   `UnsupportedEnvelope(NonCanonicalCarrier)` (the termination would be an
///   Ellipse). The returned carriers are canonical by construction.
pub fn project_profile(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    target: &Until,
) -> Outcome<Vec<Curve>> {
    let (pi, denom) = sweep_gates(dir, target)?;
    let n = pi.normal();
    let o = pi.origin();
    let material_idx = select_material(profile, arrangement)?;
    let material = arrangement
        .regions
        .get(material_idx)
        .ok_or(Refusal::Empty)?;
    let ts = boundary_ts(material, arrangement, dir, n, o, denom)?;
    let cross = n.cross(dir);
    let parallel = cross.x == 0.0 && cross.y == 0.0 && cross.z == 0.0;
    let mut out: Vec<Curve> = Vec::new();
    if parallel {
        let mut t_star: Option<f64> = None;
        for &t in ts.values() {
            match t_star {
                Some(prev) if prev != t => return Err(Refusal::Empty),
                Some(_) => {}
                None => t_star = Some(t),
            }
        }
        let t_star = match t_star {
            Some(t) => t,
            None => return Err(Refusal::Empty),
        };
        let h = t_star * dir;
        if !h.x.is_finite() || !h.y.is_finite() || !h.z.is_finite() {
            return Err(Refusal::Empty);
        }
        for cycle in &material.boundaries {
            for &h_e in cycle {
                let he = arrangement.half_edges.get(h_e).ok_or(Refusal::Empty)?;
                let curve = profile.get(he.curve).ok_or(Refusal::Empty)?;
                out.push(translate_curve(curve, h)?);
            }
        }
    } else {
        for cycle in &material.boundaries {
            let n_edges = cycle.len();
            if n_edges == 0 {
                return Err(Refusal::Empty);
            }
            for i in 0..n_edges {
                let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
                let h_next = *cycle.get((i + 1) % n_edges).ok_or(Refusal::Empty)?;
                let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
                let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
                match profile.get(he_i.curve) {
                    Some(Curve::Line(_)) => {
                        let ta = ts.get(&he_i.origin).copied().ok_or(Refusal::Empty)?;
                        let tb = ts.get(&he_next.origin).copied().ok_or(Refusal::Empty)?;
                        let pa = arrangement
                            .vertices
                            .get(he_i.origin)
                            .ok_or(Refusal::Empty)?
                            .point;
                        let pb = arrangement
                            .vertices
                            .get(he_next.origin)
                            .ok_or(Refusal::Empty)?
                            .point;
                        out.push(Curve::Line(Line(pa + ta * dir, pb + tb * dir)));
                    }
                    Some(Curve::Circle(_)) => {
                        return Err(Refusal::UnsupportedEnvelope(
                            EnvelopeCase::NonCanonicalCarrier,
                        ));
                    }
                    _ => return Err(Refusal::Empty),
                }
            }
        }
    }
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        out,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

// ---------------------------------------------------------------------------
// The certified-t ordering (D3).
// ---------------------------------------------------------------------------

/// The input gates shared by both entries: a non-finite `dir`, a z-neutral
/// sweep, or a target plane parallel to the sweep direction (`n · dir == 0`)
/// refuses `Empty` — the sweep never terminates on the plane. Returns the
/// target plane and the denominator `n · dir`.
fn sweep_gates(dir: Vector3, target: &Until) -> Result<(Plane, f64), Refusal> {
    if !dir.x.is_finite() || !dir.y.is_finite() || !dir.z.is_finite() || dir.z == 0.0 {
        return Err(Refusal::Empty);
    }
    let pi = match target {
        Until::Plane(pi) => *pi,
    };
    let n = pi.normal();
    let denom = n.dot(dir);
    if denom == 0.0 {
        return Err(Refusal::Empty);
    }
    Ok((pi, denom))
}

/// The signed crossing parameters `t(p) = (n·o − n·p) / (n·dir)` of the
/// material boundary vertices, each machine-checked against the closed form:
/// the projected point `p + t·dir` must lie on Π (BG-NUM-002 applies to the
/// `t` values exactly as to geometry).
fn boundary_ts(
    material: &ArrRegion,
    arrangement: &Arrangement,
    dir: Vector3,
    n: Vector3,
    o: Point3,
    denom: f64,
) -> Result<HashMap<usize, f64>, Refusal> {
    let mut ts: HashMap<usize, f64> = HashMap::new();
    for cycle in &material.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if ts.contains_key(&he.origin) {
                continue;
            }
            let p = arrangement
                .vertices
                .get(he.origin)
                .ok_or(Refusal::Empty)?
                .point;
            let t = (o - p).dot(n) / denom;
            if !t.is_finite() {
                return Err(Refusal::Empty);
            }
            let cap_v = p + t * dir;
            if (cap_v - o).dot(n).abs() > PLANE_RESIDUAL_SLACK {
                return Err(Refusal::Empty);
            }
            ts.insert(he.origin, t);
        }
    }
    Ok(ts)
}

// ---------------------------------------------------------------------------
// The termination-line certificate (D4).
// ---------------------------------------------------------------------------

/// The termination locus of a curtain wall against the target plane: the
/// wall × Π pair answers with a Line through the landed `contact()` exact FF
/// arm — never a raw intersection.
fn termination_line(wall: Plane, pi: Plane) -> Result<Line<Point3>, Refusal> {
    let wall_stratum = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(wall),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    };
    let pi_stratum = BoundedStratum::Face {
        surface: CanonicalSurface::Plane(pi),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    };
    let mut budget = Budget::new(0, 0, 0);
    let Certified { value, .. } = contact(&wall_stratum, &pi_stratum, &mut budget)?;
    for record in &value.contacts {
        if let ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Line(line))) =
            &record.locus
        {
            return Ok(*line);
        }
    }
    Err(Refusal::UnsupportedEnvelope(
        EnvelopeCase::NonCanonicalCarrier,
    ))
}

// ---------------------------------------------------------------------------
// The region machinery (the extrude.rs session-28 helpers, local to this
// module: `extrude.rs`'s copies are private by design and stay private).
// ---------------------------------------------------------------------------

/// Selects the single material region: a bounded `winding == 1` region not
/// strictly inside another bounded `winding == 1` region's boundary cycle.
/// v1 accepts exactly one; anything else is `Refusal::Empty`.
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
        let nudge = 64.0 * crate::TOLERANCE;
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

/// Whether cycle `ci` of a region is a hole, by the containment rule: a cycle
/// is a hole iff its polygon lies inside another cycle's polygon of the same
/// region. The winding sign is never consulted.
fn cycle_is_hole(
    cycles: &[Vec<usize>],
    ci: usize,
    profile: &[Curve],
    arrangement: &Arrangement,
) -> bool {
    let cycle = match cycles.get(ci) {
        Some(c) => c,
        None => return false,
    };
    let poly = cycle_polygon(cycle, profile, arrangement);
    if poly.is_empty() {
        return false;
    }
    cycles.iter().enumerate().any(|(cj, other)| {
        if cj == ci {
            return false;
        }
        let outer = cycle_polygon(other, profile, arrangement);
        !outer.is_empty() && poly.iter().all(|p| point_in_poly(*p, &outer))
    })
}

/// The exact arrangement vertex points of a boundary cycle, in cycle order.
fn cycle_vertex_points(cycle: &[usize], arrangement: &Arrangement) -> Result<Vec<Point3>, Refusal> {
    let mut pts = Vec::with_capacity(cycle.len());
    for &h in cycle {
        let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
        let p = arrangement
            .vertices
            .get(he.origin)
            .ok_or(Refusal::Empty)?
            .point;
        pts.push(p);
    }
    Ok(pts)
}

/// Whether a z = 0 polygon is strictly convex in the CCW trace: every
/// consecutive triple turns strictly left (a reflex vertex — a right turn or a
/// collinear vertex — anywhere refuses).
fn strictly_convex_ccw(pts: &[Point3]) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = match pts.get(i % n) {
            Some(p) => *p,
            None => return false,
        };
        let b = match pts.get((i + 1) % n) {
            Some(p) => *p,
            None => return false,
        };
        let c = match pts.get((i + 2) % n) {
            Some(p) => *p,
            None => return false,
        };
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross <= 0.0 {
            return false;
        }
    }
    true
}

/// The boundary edges of one cycle on the z = 0 plane, in cycle order: a line
/// edge between the shared vertex points (circle edges are refused before the
/// construction ever reaches this helper).
fn cycle_boundary_edges(
    cycle: &[usize],
    arrangement: &Arrangement,
    vertex: &HashMap<usize, Vertex>,
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
        let v0 = vertex.get(&he_i.origin).ok_or(Refusal::Empty)?;
        let v1 = vertex.get(&he_next.origin).ok_or(Refusal::Empty)?;
        let edge = Edge::try_new(v0, v1, Curve::Line(Line(v0.point(), v1.point())))
            .map_err(|_| Refusal::Empty)?;
        edges.push(edge);
    }
    Ok(edges)
}

/// The seam edge (bottom → top on Π) of a boundary vertex, created once and
/// reused by the two adjacent walls.
fn get_or_create_seam(
    v_idx: usize,
    bottom_vertex: &HashMap<usize, Vertex>,
    top_vertex: &HashMap<usize, Vertex>,
    seams: &mut HashMap<usize, Edge>,
) -> Result<Edge, Refusal> {
    if let Some(e) = seams.get(&v_idx) {
        return Ok(e.clone());
    }
    let b = bottom_vertex.get(&v_idx).ok_or(Refusal::Empty)?;
    let t = top_vertex.get(&v_idx).ok_or(Refusal::Empty)?;
    let edge =
        Edge::try_new(b, t, Curve::Line(Line(b.point(), t.point()))).map_err(|_| Refusal::Empty)?;
    seams.insert(v_idx, edge.clone());
    Ok(edge)
}

/// The translated image of a boundary curve under `h`: a line shifts both
/// endpoints; a placed circle shifts the placement's translation column
/// (translation preserves the `Curve` type).
fn translate_curve(curve: &Curve, h: Vector3) -> Result<Curve, Refusal> {
    match curve {
        Curve::Line(Line(a, b)) => Ok(Curve::Line(Line(*a + h, *b + h))),
        Curve::Circle(p) => {
            let mut m = *p.transform();
            m.w.x += h.x;
            m.w.y += h.y;
            m.w.z += h.z;
            Ok(Curve::Circle(Processor::with_transform(*p.entity(), m)))
        }
        _ => Err(Refusal::Empty),
    }
}

/// The 2-D (x, y) projection of a 3-D point.
fn pt2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}
