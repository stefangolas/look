//! BG-CAD-P5-REVOLVE — revolve of line-edge profiles via the carrier table.
//!
//! `revolve_profile` turns the material region of a planar arrangement into a
//! closed `Solid<Point3, Curve, Surface>` by revolution about the z-axis. The
//! profile's material region lives in the xz-plane (y = 0); the sweep turns
//! from the +x direction toward +y (right-handed about +z). This is the same
//! "canonical frame, general forms later" posture as the landed scalar
//! `extrude_profile` (extrude.rs:70, FROZEN): P9/P10 conjugation is the booked
//! unlock for arbitrary axes.
//!
//! Line-edge profiles only (table 6.3, Tier 2 books circle edges): every
//! profile line becomes a canonical carrier per plan §6.2 — a vertical edge
//! (x = c const) sweeps a canonical `Cylinder` about the axis, a horizontal
//! edge (z = c const) sweeps the canonical `Plane` z = c, and a slanted edge
//! extends to its z-axis crossing and sweeps the canonical `Cone` whose apex
//! sits on the axis. The carrier for every wall is the CANONICAL analytic type
//! directly — `Surface::RevolutedCurve` is never emitted (the same "canonical
//! without emitting the decorator" posture as the plan's curtain table 6.1).
//!
//! `angle == TAU` emits a closed annulus-style face per boundary edge, with
//! self-loop circle boundary wires (the landed extrude cylinder-wall recipe);
//! the two end caps coincide at the profile region's face and are interior, so
//! no cap is emitted. `0 < angle < TAU` emits the two end caps (the profile
//! region's face at y = 0 and its rotated image) plus one wall per boundary
//! edge, each wall carrying a 4-edge boundary wire of two meridian Line edges
//! (shared with the caps and, through the shared-arc rule, with the adjacent
//! walls) and two arc edges.
//!
//! v1 scope: exactly ONE material region (the landed extrude v1 rule), strictly
//! at x > 0 (an axis-crossing profile refuses `NonCanonicalCarrier`; an edge
//! endpoint exactly on the axis refuses `Collapsed` — REV-AXIS-CROSS and the
//! table 6.2 "collapsed edge becomes vertex" row are booked follow-ups), and
//! `PC = ()`. House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::{
    Cone, Curve, Cylinder, Edge, Face, InnerSpace, Line, Matrix4, Plane, Point2, Point3, Processor,
    Shell, Solid, Surface, TrimmedCurve, UnitCircle, Vector3, Vector4, Vertex, Wire, TOLERANCE,
};
use std::collections::HashMap;
use std::f64::consts::TAU;
use truck_base::evidence::{
    Budget, Certificate, Certified, Collapse, CollapseReason, ContradictionWitness, EnvelopeCase,
    Margin, Method, Modulus, Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_geometry::arrange::{ArrRegion, Arrangement};
use truck_geometry::recognize::{recognize_curve, recognize_surface, CanonicalCarrierWitness};
use truck_geotrait::{ParametricCurve, ParametricSurface3D};

/// The number of samples used to polygonize a circle loop for the material
/// representative / containment predicates.
const CIRCLE_SAMPLES: usize = 32;

/// Revolves the material region of a planar arrangement by `angle` about the
/// z-axis into a closed solid.
///
/// - The profile's material region lies in the **xz-plane (y = 0)**; the
///   revolve axis is the **z-axis**; the sweep turns from the +x direction
///   toward +y (right-handed about +z). `arrangement` is the arrangement of
///   the working copy (each point `(x, 0, z)` mapped to `(x, z, 0)`), the same
///   slice semantics the landed `extrude_profile` uses.
/// - `angle <= 0.0`, non-finite, or `angle > TAU` refuses `Refusal::Empty`
///   (the landed extrude non-positive-height convention).
/// - The profile region must lie strictly at x > 0. Any vertex with x < 0
///   refuses `UnsupportedEnvelope(NonCanonicalCarrier)` (REV-AXIS-CROSS is the
///   booked formal follow-up); an edge ENDPOINT exactly at x = 0 refuses
///   `Refusal::Collapsed` with a certificate naming the collapsed edge (the
///   table 6.2 "collapsed edge becomes vertex" row is the booked follow-up).
/// - A Circle (or any non-Line) edge in the profile region's boundary refuses
///   `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift, before any
///   construction is paid for (table 6.3 is Tier 2).
/// - Exactly ONE material region (the landed extrude v1 rule); a different
///   count refuses `Refusal::Empty`.
pub fn revolve_profile(profile: &[Curve], arrangement: &Arrangement, angle: f64) -> Outcome<Solid> {
    if !angle.is_finite() || angle <= 0.0 || angle > TAU {
        return Err(Refusal::Empty);
    }
    // The arrangement is built over the working copy (x, 0, z) -> (x, z, 0),
    // so every region computation below runs on the index-aligned working
    // copy, whose (x, y) are the profile's (radius, height).
    let working = work_profile(profile)?;
    let material_idx = select_material(&working, arrangement)?;
    let material = arrangement
        .regions
        .get(material_idx)
        .ok_or(Refusal::Empty)?;

    // Cycle roles by containment (never the winding sign: S1 normalizes every
    // loop to CCW, so winding cannot distinguish a hole from its plate).
    let cycle_holes: Vec<bool> = material
        .boundaries
        .iter()
        .enumerate()
        .map(|(ci, _)| cycle_is_hole(&material.boundaries, ci, &working, arrangement))
        .collect();

    // The v1 boundary checks (D2): line-only boundary edges, strictly at
    // x > 0, no edge endpoint on the axis.
    validate_boundary(&working, arrangement, material)?;

    let faces = if angle == TAU {
        full_turn_faces(&working, arrangement, material, &cycle_holes)?
    } else {
        partial_faces(&working, arrangement, material, &cycle_holes, angle)?
    };

    // Certificates (D5): every emitted carrier must be recognized as canonical
    // — the construction above cannot produce anything else, so an
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

    // Assembly and validation: the shell MUST pass `Solid::try_new` — closed,
    // connected, no singular vertices. If it refuses, the topology is wrong,
    // never weakened.
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

/// The index-aligned working copy of the profile: the xz-plane profile (y = 0)
/// mapped to the z = 0 plane by the coordinate swap `(x, 0, z) -> (x, z, 0)`,
/// which the arrangement's 2-D frame reads as (radius, height).
fn work_profile(profile: &[Curve]) -> Result<Vec<Curve>, Refusal> {
    let mut out = Vec::with_capacity(profile.len());
    for c in profile {
        out.push(work_curve(c)?);
    }
    Ok(out)
}

/// The working copy of one curve: a line's points swap their z into y; a
/// circle's placement matrix columns swap their y/z components (the circle's
/// y-basis points along the axis ±y, so the swap lands it in the z = 0 plane
/// with its second basis along +y).
fn work_curve(c: &Curve) -> Result<Curve, Refusal> {
    match c {
        Curve::Line(Line(a, b)) => {
            if a.y != 0.0 || b.y != 0.0 {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
            Ok(Curve::Line(Line(
                Point3::new(a.x, a.z, 0.0),
                Point3::new(b.x, b.z, 0.0),
            )))
        }
        Curve::Circle(p) => {
            let m = *p.transform();
            let swapped = Matrix4 {
                x: swap_z(m.x),
                y: swap_z(m.y),
                z: swap_z(m.z),
                w: swap_z(m.w),
            };
            Ok(Curve::Circle(Processor::with_transform(
                *p.entity(),
                swapped,
            )))
        }
        _ => Err(Refusal::UnsupportedEnvelope(
            EnvelopeCase::NonCanonicalCarrier,
        )),
    }
}

/// The coordinate swap `(x, y, z, w) -> (x, z, y, w)` of one matrix column.
fn swap_z(v: Vector4) -> Vector4 {
    Vector4::new(v.x, v.z, v.y, v.w)
}

/// The working-frame point of an arrangement vertex: the arrangement's (x, y)
/// are the profile's (radius, height), i.e. the output point at y = 0.
fn revolve_point(p: Point3) -> Point3 {
    Point3::new(p.x, 0.0, p.y)
}

/// The rotation of a vector about the z-axis by `angle`.
fn rotate_z(v: Vector3, angle: f64) -> Vector3 {
    Vector3::new(
        v.x * angle.cos() - v.y * angle.sin(),
        v.x * angle.sin() + v.y * angle.cos(),
        v.z,
    )
}

/// The rotation of a point about the z-axis by `angle`.
fn rotate_z_point(q: Point3, angle: f64) -> Point3 {
    Point3::new(
        q.x * angle.cos() - q.y * angle.sin(),
        q.x * angle.sin() + q.y * angle.cos(),
        q.z,
    )
}

/// The D2 boundary validation: every material-boundary edge is a `Line`, every
/// boundary vertex lies strictly at x > 0 (x < 0 refuses `NonCanonicalCarrier`,
/// REV-AXIS-CROSS; x == 0 refuses `Collapsed`, the axis-touch row).
fn validate_boundary(
    working: &[Curve],
    arrangement: &Arrangement,
    material: &ArrRegion,
) -> Result<(), Refusal> {
    for cycle in &material.boundaries {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            if !matches!(working.get(he_i.curve), Some(Curve::Line(_))) {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
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
            for p in [a, b] {
                if p.x < 0.0 {
                    return Err(Refusal::UnsupportedEnvelope(
                        EnvelopeCase::NonCanonicalCarrier,
                    ));
                }
                if p.x == 0.0 {
                    return Err(axis_touch_collapse());
                }
            }
        }
    }
    Ok(())
}

/// The certified axis-touch refusal: an edge endpoint exactly on the revolve
/// axis. The revolved edge collapses to a single apex/disk-center vertex (a
/// topology event this packet does not build; the table 6.2 "collapsed edge
/// becomes vertex" row is the booked follow-up). The certificate names the
/// collapsed edge's wedge: its dihedral collapses to zero.
fn axis_touch_collapse() -> Refusal {
    let mut props = PropMap::new();
    props.set(Prop::WedgeNonDegeneracy, Truth::False);
    Refusal::Collapsed(
        Collapse {
            reason: CollapseReason::KnifeEdge,
        },
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    )
}

/// The full-turn construction: one closed annulus-style face per material
/// boundary edge, each with two self-loop circle boundary wires (the landed
/// extrude cylinder-wall recipe). The two end caps coincide at the profile
/// region's face and are interior, so no cap is emitted.
fn full_turn_faces(
    working: &[Curve],
    arrangement: &Arrangement,
    material: &ArrRegion,
    cycle_holes: &[bool],
) -> Result<Vec<Face>, Refusal> {
    let mut v_indices: Vec<usize> = Vec::new();
    for cycle in &material.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if !v_indices.contains(&he.origin) {
                v_indices.push(he.origin);
            }
        }
    }
    // One `Vertex::new(point)` per arrangement vertex of the material boundary
    // (rule 4 — the load-bearing instance rule): the self-loop's front and
    // back are the SAME instance, and the two walls sharing a loop must share
    // that instance.
    let mut vertex: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &v_indices {
        let point = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        vertex.insert(v_idx, Vertex::new(revolve_point(point)));
    }
    // One self-loop circle edge per boundary vertex, stored with orientation
    // true; the walls reference it (or its inverse) per the pairing scheme.
    let mut circle_edge: HashMap<usize, Edge> = HashMap::new();
    for &v_idx in &v_indices {
        let p = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        let v = vertex.get(&v_idx).ok_or(Refusal::Empty)?;
        let curve = full_circle_curve(p.x, p.y)?;
        circle_edge.insert(v_idx, Edge::new_unchecked(v, v, curve));
    }

    let mut faces = Vec::new();
    for (ci, cycle) in material.boundaries.iter().enumerate() {
        let is_hole = match cycle_holes.get(ci) {
            Some(&is_hole) => is_hole,
            None => false,
        };
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let v_start = he_i.origin;
            let v_end = he_next.origin;
            let p_start = arrangement
                .vertices
                .get(v_start)
                .ok_or(Refusal::Empty)?
                .point;
            let p_end = arrangement.vertices.get(v_end).ok_or(Refusal::Empty)?.point;

            // The outward direction of the solid at the wall's profile (θ = 0):
            // the right normal of the edge's direction d = (dx, 0, dz) in the
            // xz-plane is ŷ × d; a hole cycle's material sits on the opposite
            // side, so its outward is −(ŷ × d).
            let dx = p_end.x - p_start.x;
            let dz = p_end.y - p_start.y;
            let mut outward = Vector3::new(dz, 0.0, -dx);
            if is_hole {
                outward = -outward;
            }

            let surface = wall_surface(working, he_i, p_start, p_end)?;
            let natural = wall_natural_normal(&surface, p_start, p_end);
            let invert = natural.dot(outward) < 0.0;

            // The pairing scheme: wall(e) uses its START loop with the flag
            // `!o` and its END loop with the flag `o` (base flag true), which
            // makes every shared loop's two effective uses opposite.
            let start_edge = circle_edge.get(&v_start).ok_or(Refusal::Empty)?;
            let end_edge = circle_edge.get(&v_end).ok_or(Refusal::Empty)?;
            let (start_wire_edge, end_wire_edge) = if invert {
                (start_edge.clone(), end_edge.inverse())
            } else {
                (start_edge.inverse(), end_edge.clone())
            };
            let wires = vec![
                Wire::from(vec![start_wire_edge]),
                Wire::from(vec![end_wire_edge]),
            ];
            let mut face = Face::try_new(wires, surface).map_err(|_| Refusal::Empty)?;
            if invert {
                face.invert();
            }
            faces.push(face);
        }
    }
    Ok(faces)
}

/// The partial-angle construction: the two end caps (the profile region's face
/// at y = 0, stored inverted, and its rotated image, stored uninverted) plus
/// one wall per boundary edge. All walls are stored UNINVERTED — the shared
/// arcs' pairing forces one orientation across the whole wall cycle — and each
/// wall's carrier is built so its natural normal is the wall's outward normal
/// where the carrier allows it (the canonical analytic carriers' fixed
/// normals).
fn partial_faces(
    working: &[Curve],
    arrangement: &Arrangement,
    material: &ArrRegion,
    _cycle_holes: &[bool],
    angle: f64,
) -> Result<Vec<Face>, Refusal> {
    let mut v_indices: Vec<usize> = Vec::new();
    for cycle in &material.boundaries {
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if !v_indices.contains(&he.origin) {
                v_indices.push(he.origin);
            }
        }
    }
    // One vertex per arrangement vertex of the material boundary at θ = 0 and
    // one at θ = angle (rule 4 again: the cap and the two adjacent walls share
    // each instance).
    let mut v0: HashMap<usize, Vertex> = HashMap::new();
    let mut va: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &v_indices {
        let point = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        let q0 = revolve_point(point);
        let qa = rotate_z_point(q0, angle);
        v0.insert(v_idx, Vertex::new(q0));
        va.insert(v_idx, Vertex::new(qa));
    }

    // The meridian and arc edges, built ONCE and shared: the θ = 0 meridian of
    // wall(e) IS the cap's edge; the arc at a vertex is shared by the two
    // adjacent walls.
    let mut m0: HashMap<usize, Edge> = HashMap::new();
    let mut ma: HashMap<usize, Edge> = HashMap::new();
    let mut arc: HashMap<usize, Edge> = HashMap::new();
    for cycle in &material.boundaries {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let v_s = he_i.origin;
            let v_e = he_next.origin;
            let p_s = arrangement.vertices.get(v_s).ok_or(Refusal::Empty)?.point;
            let p_e = arrangement.vertices.get(v_e).ok_or(Refusal::Empty)?.point;
            let q0_s = revolve_point(p_s);
            let q0_e = revolve_point(p_e);
            let qa_s = rotate_z_point(q0_s, angle);
            let qa_e = rotate_z_point(q0_e, angle);
            let v0_s = v0.get(&v_s).ok_or(Refusal::Empty)?;
            let v0_e = v0.get(&v_e).ok_or(Refusal::Empty)?;
            let va_s = va.get(&v_s).ok_or(Refusal::Empty)?;
            let va_e = va.get(&v_e).ok_or(Refusal::Empty)?;
            let m0_edge = Edge::try_new(v0_s, v0_e, Curve::Line(Line(q0_s, q0_e)))
                .map_err(|_| Refusal::Empty)?;
            let ma_edge = Edge::try_new(va_s, va_e, Curve::Line(Line(qa_s, qa_e)))
                .map_err(|_| Refusal::Empty)?;
            m0.insert(h_i, m0_edge);
            ma.insert(h_i, ma_edge);
        }
    }
    for &v_idx in &v_indices {
        let p = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        let v0_v = v0.get(&v_idx).ok_or(Refusal::Empty)?;
        let va_v = va.get(&v_idx).ok_or(Refusal::Empty)?;
        let curve = arc_curve(p.x, p.y, angle)?;
        let edge = Edge::try_new(v0_v, va_v, curve).map_err(|_| Refusal::Empty)?;
        arc.insert(v_idx, edge);
    }

    let mut faces = Vec::new();

    // The θ = 0 cap: the profile region's face at y = 0, built from the
    // arrangement's boundary cycles, stored INVERTED so its effective normal is
    // −y (the outward normal of the swept solid at the start cap).
    let cap0_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
        Point3::new(1.0, 0.0, 0.0),
    ));
    let mut cap0_wires = Vec::new();
    for cycle in &material.boundaries {
        let mut edges = Vec::new();
        for &h in cycle {
            edges.push(m0.get(&h).ok_or(Refusal::Empty)?.clone());
        }
        cap0_wires.push(Wire::from(edges));
    }
    let mut cap0 = Face::try_new(cap0_wires, cap0_surface).map_err(|_| Refusal::Empty)?;
    cap0.invert();
    faces.push(cap0);

    // The θ = angle cap: the rotated profile region's face, stored UNINVERTED.
    // Its plane is built with natural normal +θ (the outward normal of the
    // swept solid at the end cap).
    let outer = material.boundaries.first().ok_or(Refusal::Empty)?;
    let h_first = *outer.first().ok_or(Refusal::Empty)?;
    let he_first = arrangement.half_edges.get(h_first).ok_or(Refusal::Empty)?;
    let o = va.get(&he_first.origin).ok_or(Refusal::Empty)?.point();
    let cap_a_surface = Surface::Plane(Plane::new(
        o,
        o + Vector3::new(0.0, 0.0, 1.0),
        o + rotate_z(Vector3::new(1.0, 0.0, 0.0), angle),
    ));
    let mut cap_a_wires = Vec::new();
    for cycle in &material.boundaries {
        let mut edges = Vec::new();
        for &h in cycle {
            edges.push(ma.get(&h).ok_or(Refusal::Empty)?.clone());
        }
        cap_a_wires.push(Wire::from(edges));
    }
    faces.push(Face::try_new(cap_a_wires, cap_a_surface).map_err(|_| Refusal::Empty)?);

    // The walls, one per boundary edge, each a 4-edge wire: [θ0 meridian, arc
    // at the end vertex, θ1 meridian reversed, arc at the start vertex
    // reversed] — the landed extrude wall wiring, with the arcs as the seams.
    for cycle in &material.boundaries {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let v_s = he_i.origin;
            let v_e = he_next.origin;
            let p_s = arrangement.vertices.get(v_s).ok_or(Refusal::Empty)?.point;
            let p_e = arrangement.vertices.get(v_e).ok_or(Refusal::Empty)?.point;
            let surface = partial_wall_surface(working, he_i, p_s, p_e, angle)?;
            let m0_i = m0.get(&h_i).ok_or(Refusal::Empty)?.clone();
            let ma_i = ma.get(&h_i).ok_or(Refusal::Empty)?.clone();
            let arc_e = arc.get(&v_e).ok_or(Refusal::Empty)?.clone();
            let arc_s = arc.get(&v_s).ok_or(Refusal::Empty)?.clone();
            let wire = Wire::from(vec![m0_i, arc_e, ma_i.inverse(), arc_s.inverse()]);
            faces.push(Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?);
        }
    }
    Ok(faces)
}

/// The canonical carrier of a boundary edge's full-turn wall (D3): a vertical
/// edge sweeps a `Cylinder` about the axis, a horizontal edge the `Plane`
/// z = c, a slanted edge the `Cone` whose apex sits at the edge's z-axis
/// crossing.
fn wall_surface(
    working: &[Curve],
    he: &truck_geometry::arrange::ArrHalfEdge,
    p_start: Point3,
    p_end: Point3,
) -> Result<Surface, Refusal> {
    match working.get(he.curve) {
        Some(Curve::Line(_)) => {}
        _ => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ))
        }
    }
    let dx = p_end.x - p_start.x;
    let dz = p_end.y - p_start.y;
    if dx == 0.0 {
        // Vertical edge (x = c const, c > 0): the cylinder about the axis.
        let cylinder = match Cylinder::new(Point3::new(0.0, 0.0, 0.0), p_start.x) {
            Ok(c) => c.value,
            Err(_) => return Err(Refusal::Empty),
        };
        Ok(Surface::Cylinder(cylinder))
    } else if dz == 0.0 {
        // Horizontal edge (z = c const): the plane z = c, natural normal +z.
        let z = p_start.y;
        Ok(Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, z),
            Point3::new(1.0, 0.0, z),
            Point3::new(0.0, 1.0, z),
        )))
    } else {
        // Slanted edge: extend to the z-axis crossing (x = 0 at some z*); the
        // cone has apex (0, 0, z*) and half angle atan(|dx/dz|).
        let slope = (p_end.x - p_start.x) / (p_end.y - p_start.y);
        let apex_z = p_start.y - p_start.x / slope;
        let half_angle = slope.abs().atan();
        if !apex_z.is_finite() || !half_angle.is_finite() {
            return Err(Refusal::Empty);
        }
        match Cone::new(Point3::new(0.0, 0.0, apex_z), half_angle) {
            Ok(c) => Ok(Surface::Cone(c.value)),
            Err(_) => Err(Refusal::Empty),
        }
    }
}

/// The carrier's natural normal at the wall's profile (θ = 0): a cylinder's
/// +r̂, a horizontal plane's +z, a cone's nappe-dependent normal evaluated at
/// the edge's mid-height.
fn wall_natural_normal(surface: &Surface, p_start: Point3, p_end: Point3) -> Vector3 {
    match surface {
        Surface::Cylinder(cyl) => cyl.normal(0.0, 0.0),
        Surface::Plane(plane) => plane.normal(),
        Surface::Cone(cone) => {
            let v = (p_start.y + p_end.y) * 0.5 - cone.apex().z;
            if v == 0.0 {
                Vector3::unit_x()
            } else {
                cone.normal(0.0, v)
            }
        }
        _ => Vector3::unit_x(),
    }
}

/// The partial-angle wall's carrier: the same D3 table as the full-turn wall,
/// except a horizontal edge's plane is built through the swept sector so its
/// natural normal IS the wall's outward normal (the partial-angle walls are
/// all stored uninverted).
fn partial_wall_surface(
    working: &[Curve],
    he: &truck_geometry::arrange::ArrHalfEdge,
    p_start: Point3,
    p_end: Point3,
    angle: f64,
) -> Result<Surface, Refusal> {
    match working.get(he.curve) {
        Some(Curve::Line(_)) => {}
        _ => {
            return Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            ))
        }
    }
    let dx = p_end.x - p_start.x;
    let dz = p_end.y - p_start.y;
    if dx == 0.0 {
        let cylinder = match Cylinder::new(Point3::new(0.0, 0.0, 0.0), p_start.x) {
            Ok(c) => c.value,
            Err(_) => return Err(Refusal::Empty),
        };
        Ok(Surface::Cylinder(cylinder))
    } else if dz == 0.0 {
        // The plane through the swept sector: Plane::new(P, P', Q) has natural
        // normal −sign(dx)·ẑ = the wall's outward direction.
        let q0 = revolve_point(p_start);
        let qa = rotate_z_point(q0, angle);
        let q1 = revolve_point(p_end);
        Ok(Surface::Plane(Plane::new(q0, qa, q1)))
    } else {
        let slope = (p_end.x - p_start.x) / (p_end.y - p_start.y);
        let apex_z = p_start.y - p_start.x / slope;
        let half_angle = slope.abs().atan();
        if !apex_z.is_finite() || !half_angle.is_finite() {
            return Err(Refusal::Empty);
        }
        match Cone::new(Point3::new(0.0, 0.0, apex_z), half_angle) {
            Ok(c) => Ok(Surface::Cone(c.value)),
            Err(_) => Err(Refusal::Empty),
        }
    }
}

/// A full circle self-loop curve of radius `r` at height `z` about the axis.
fn full_circle_curve(r: f64, z: f64) -> Result<Curve, Refusal> {
    if !r.is_finite() || !z.is_finite() || r <= 0.0 {
        return Err(Refusal::Empty);
    }
    Ok(Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(0.0, 0.0, z, 1.0),
        },
    )))
}

/// The arc of the circle of radius `r` at height `z` from θ = 0 to `angle`.
fn arc_curve(r: f64, z: f64, angle: f64) -> Result<Curve, Refusal> {
    if !r.is_finite()
        || !z.is_finite()
        || !angle.is_finite()
        || r <= 0.0
        || angle <= 0.0
        || angle > TAU
    {
        return Err(Refusal::Empty);
    }
    Ok(Curve::Circle(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, angle)),
        Matrix4 {
            x: Vector4::new(r, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, r, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 1.0, 0.0),
            w: Vector4::new(0.0, 0.0, z, 1.0),
        },
    )))
}

// ---------------------------------------------------------------------------
// The arrangement-side helpers (the session-28 material machinery, local to
// this module: `extrude.rs`'s copies are private by design and stay private).
// ---------------------------------------------------------------------------

/// Selects the single material region. A material region is a bounded
/// `ArrRegion` with `winding == 1` that is NOT strictly inside another bounded
/// `winding == 1` region's boundary cycle. v1 accepts exactly one material
/// region; anything else is `Refusal::Empty`.
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

/// Whether cycle `ci` of a region is a hole, by the containment rule (the rule
/// the arrangement's own nesting and `select_material`'s region logic use): a
/// cycle is a hole iff its polygon lies inside another cycle's polygon of the
/// same region. The winding sign is never consulted.
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

/// The 2-D (x, y) projection of a 3-D point.
fn pt2(p: Point3) -> Point2 {
    Point2::new(p.x, p.y)
}
