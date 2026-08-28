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
//! BG-CAD-P2-EXTRUDE generalizes the landed entry in place. The internal
//! interval form `extrude_interval` spans `[base, tip]` (translation offsets
//! of the z = 0 profile) with an optional draft `taper`; the public entries
//! `extrude_profile_vector` (a direction vector, `both`) and
//! `extrude_profile_taper` (height + draft) delegate to it, and the landed
//! scalar entry is the degenerate interval `[0, height·ẑ]`. Every emitted
//! carrier stays in the canonical set: line edges sweep to canonical `Plane`s
//! in ANY sweep direction, circle edges sweep to canonical `Cylinder`s only
//! for z-parallel sweeps (an oblique sweep of a circle edge is refused as
//! `NonCanonicalCarrier` — an oblique cylinder is a `Placed` carrier, Tier
//! 1's unlock), and a taper offsets the profile curves and re-arranges them
//! (the parsimony move — the top cap is never hand-constructed), turning
//! circle walls into z-aligned `Cone`s.
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
    Cone, Curve, Cylinder, Edge, Face, Homogeneous, InnerSpace, Line, Plane, Point2, Point3,
    Processor, Shell, Solid, Surface, Vector3, Vertex, Wire, TOLERANCE,
};
use std::collections::HashMap;
use std::f64::consts::FRAC_PI_2;
use truck_base::evidence::{
    Budget, Certificate, Certified, Collapse, CollapseReason, ContradictionWitness, EnvelopeCase,
    Margin, Method, Modulus, Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_geometry::arrange::{arrange, ArrRegion, Arrangement};
use truck_geometry::recognize::{recognize_curve, recognize_surface, CanonicalCarrierWitness};
use truck_geotrait::ParametricCurve;

/// The number of samples used to polygonize a circle loop for the material
/// representative / containment predicates.
const CIRCLE_SAMPLES: usize = 32;

/// Extrudes the material region(s) of a planar arrangement by `height` along
/// +z into a closed solid. v1 scope: exactly ONE material region (the
/// containment-based rule below). The landed entry is the degenerate interval
/// `[0, height·ẑ]` of [`extrude_interval`] with no draft; its signature and
/// behavior are frozen.
pub fn extrude_profile(
    profile: &[Curve],
    arrangement: &Arrangement,
    height: f64,
) -> Outcome<Solid> {
    if !height.is_finite() || height <= 0.0 {
        return Err(Refusal::Empty);
    }
    extrude_interval(
        profile,
        arrangement,
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, height),
        0.0,
    )
}

/// Extrudes the material region of a planar arrangement along `dir` into a
/// closed solid (the vector form of build123d's `extrude`).
///
/// - `dir.z == 0` (or a non-finite component) refuses `Empty`: a z = 0
///   profile swept within its own plane has zero volume.
/// - `both == false` spans the interval `[0, dir]`; `both == true` spans
///   `[−dir, +dir]` — the same amount each way.
/// - A circle boundary edge with a non-z-parallel `dir` refuses
///   `UnsupportedEnvelope(NonCanonicalCarrier)`: the wall would be an
///   oblique cylinder, a `Placed` carrier outside the canonical set.
pub fn extrude_profile_vector(
    profile: &[Curve],
    arrangement: &Arrangement,
    dir: Vector3,
    both: bool,
) -> Outcome<Solid> {
    if !dir.x.is_finite() || !dir.y.is_finite() || !dir.z.is_finite() || dir.z == 0.0 {
        return Err(Refusal::Empty);
    }
    let (base, tip) = if both {
        (-dir, dir)
    } else {
        (Vector3::new(0.0, 0.0, 0.0), dir)
    };
    extrude_interval(profile, arrangement, base, tip, 0.0)
}

/// Extrudes the material region of a planar arrangement by `height` along +z
/// with the draft angle `taper` (build123d's `taper`), into a closed solid.
///
/// - `height <= 0` or non-finite refuses `Empty` (the landed convention).
/// - `|taper| >= π/2` or non-finite refuses `Empty` (the tangent is
///   undefined or sign-flipped there).
/// - The signed offset is `d = height · tan(taper)`: `taper > 0` shrinks the
///   material (positive draft), `taper < 0` grows it.
/// - The top profile is the signed 2-D offset of the material boundary,
///   re-arranged; a collapse of that offset (an inverted or emptied top
///   region, a vanished circle radius) refuses `Collapsed` — a topology
///   event, never a hand-built polygon.
pub fn extrude_profile_taper(
    profile: &[Curve],
    arrangement: &Arrangement,
    height: f64,
    taper: f64,
) -> Outcome<Solid> {
    if !height.is_finite() || height <= 0.0 {
        return Err(Refusal::Empty);
    }
    if !taper.is_finite() || taper.abs() >= FRAC_PI_2 {
        return Err(Refusal::Empty);
    }
    extrude_interval(
        profile,
        arrangement,
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, height),
        taper,
    )
}

/// The certified refusal for a collapse of the taper's offset construction:
/// the exact object collapsed (§5), so nothing is realized.
fn collapsed(reason: CollapseReason) -> Refusal {
    Refusal::Collapsed(
        Collapse { reason },
        Certificate {
            props: PropMap::new(),
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    )
}

/// The generalized interval extrude: the z = 0 profile is translated to
/// `base` (bottom cap on the plane z = `base.z`) and to `tip` (top cap on
/// z = `tip.z`); `taper != 0` offsets the top profile inward (outward for a
/// negative draft) by `d = sweep.z · tan(taper)` and re-arranges it. Side
/// faces: a `Line` edge spans the canonical plane through its bottom edge and
/// its top edge (in any sweep direction); a `Circle` edge is a canonical
/// `Cylinder` for a z-parallel sweep with no draft and a canonical z-aligned
/// `Cone` with a draft; an oblique sweep of a circle edge refuses
/// `NonCanonicalCarrier`.
fn extrude_interval(
    profile: &[Curve],
    arrangement: &Arrangement,
    base: Vector3,
    tip: Vector3,
    taper: f64,
) -> Outcome<Solid> {
    // The sweep must carry the z = 0 profile out of its own plane; a
    // z-neutral sweep has zero volume (the extrude.rs refusal convention).
    let sweep = tip - base;
    if !sweep.z.is_finite() || sweep.z == 0.0 {
        return Err(Refusal::Empty);
    }
    // The cap/side orientation conventions below assume an upward sweep. A
    // downward interval spans the same solid, flipped; swap the offsets so
    // the conventions apply unchanged — the point set is identical.
    let (base, tip) = if sweep.z < 0.0 {
        (tip, base)
    } else {
        (base, tip)
    };
    let sweep = tip - base;
    // The draft is defined against a z-parallel sweep (the taper entry takes
    // no direction parameter; oblique + taper is Tier 1).
    if taper != 0.0 && (sweep.x != 0.0 || sweep.y != 0.0) {
        return Err(Refusal::Empty);
    }
    let d = sweep.z * taper.tan();
    if taper != 0.0 && !d.is_finite() {
        return Err(Refusal::Empty);
    }

    let material_idx = select_material(profile, arrangement)?;
    let material = arrangement
        .regions
        .get(material_idx)
        .ok_or(Refusal::Empty)?;

    // Cycle roles by containment (never the winding sign: S1 normalizes
    // every loop to CCW, so winding cannot distinguish a hole from its
    // plate): a cycle is a hole iff its polygon lies inside another cycle's
    // polygon of the same region.
    let cycle_holes: Vec<bool> = material
        .boundaries
        .iter()
        .enumerate()
        .map(|(ci, _)| cycle_is_hole(&material.boundaries, ci, profile, arrangement))
        .collect();
    // The curve -> cycle-role map of the material boundary.
    let mut roles: HashMap<usize, bool> = HashMap::new();
    for (ci, cycle) in material.boundaries.iter().enumerate() {
        let is_hole = match cycle_holes.get(ci) {
            Some(is_hole) => *is_hole,
            None => false,
        };
        for &h in cycle {
            let he = arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            match roles.get(&he.curve) {
                Some(prev) if *prev != is_hole => return Err(Refusal::Empty),
                Some(_) => {}
                None => {
                    roles.insert(he.curve, is_hole);
                }
            }
        }
    }

    // A circle boundary edge swept obliquely is an oblique cylinder — a
    // `Placed` carrier, Tier 1's unlock, not ours (D1).
    if sweep.x != 0.0 || sweep.y != 0.0 {
        for curve_idx in roles.keys() {
            if matches!(profile.get(*curve_idx), Some(Curve::Circle(_))) {
                return Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier,
                ));
            }
        }
    }

    // The top profile: the identity for a neutral draft; otherwise the
    // signed 2-D offset, re-arranged (D4 — the top cap is never
    // hand-constructed).
    let offset_profile: Vec<Curve> = if d == 0.0 {
        Vec::new()
    } else {
        offset_profile_for_taper(profile, &roles, d)?
    };
    let top_arranged: Option<Certified<Arrangement>> = if d == 0.0 {
        None
    } else {
        match arrange(&offset_profile, None) {
            Ok(ok) => Some(ok),
            // An arrangement refusal of the offset profile (e.g. the
            // exactly-collinear inset) is a topology event of the offset.
            Err(_) => return Err(collapsed(CollapseReason::KnifeEdge)),
        }
    };
    let (top_profile, top_arrangement, top_material, cycle_pair): (
        &[Curve],
        &Arrangement,
        &ArrRegion,
        Vec<usize>,
    ) = match top_arranged.as_ref() {
        None => (profile, arrangement, material, Vec::new()),
        Some(ok) => {
            let top_arrangement = &ok.value;
            let top_material_idx = select_material(&offset_profile, top_arrangement)
                .map_err(|_| collapsed(CollapseReason::KnifeEdge))?;
            let top_material = top_arrangement
                .regions
                .get(top_material_idx)
                .ok_or_else(|| collapsed(CollapseReason::KnifeEdge))?;
            // The top material region structure (cycle count) must equal the
            // bottom's; any difference is a topology event.
            if top_material.boundaries.len() != material.boundaries.len() {
                return Err(collapsed(CollapseReason::KnifeEdge));
            }
            // Cycle correspondence by carrier identity, then the cyclic
            // carrier order (the offset preserves it; a difference is a
            // topology event).
            let mut cycle_pair = vec![usize::MAX; material.boundaries.len()];
            for (j, cycle) in material.boundaries.iter().enumerate() {
                let bottom_carriers = cycle_carriers(cycle, arrangement);
                let mut sorted = bottom_carriers.clone();
                sorted.sort_unstable();
                let mut matched = usize::MAX;
                for (k, top_cycle) in top_material.boundaries.iter().enumerate() {
                    let mut top_sorted = cycle_carriers(top_cycle, top_arrangement);
                    top_sorted.sort_unstable();
                    if top_sorted == sorted {
                        if matched != usize::MAX {
                            return Err(collapsed(CollapseReason::KnifeEdge));
                        }
                        matched = k;
                    }
                }
                let fresh = matched != usize::MAX && !cycle_pair.contains(&matched);
                if !fresh {
                    return Err(collapsed(CollapseReason::KnifeEdge));
                }
                let top_cycle = top_material
                    .boundaries
                    .get(matched)
                    .ok_or_else(|| collapsed(CollapseReason::KnifeEdge))?;
                if !cyclic_eq(
                    &bottom_carriers,
                    &cycle_carriers(top_cycle, top_arrangement),
                ) {
                    return Err(collapsed(CollapseReason::KnifeEdge));
                }
                if let Some(slot) = cycle_pair.get_mut(j) {
                    *slot = matched;
                }
            }
            // The material-side check: the top material region's
            // representative must lie on the material side of every offset
            // line carrier — this is what detects an inverted inset region
            // (the re-arranged crossing square of a past-collapse draft).
            // Circle carriers cannot invert silently: their radius sign is
            // gated at offset time and their wall cone at construction time.
            let rep = region_representative(top_material, &offset_profile, top_arrangement)
                .ok_or_else(|| collapsed(CollapseReason::KnifeEdge))?;
            for (curve_idx, is_hole) in &roles {
                match offset_profile.get(*curve_idx) {
                    Some(Curve::Line(Line(oa, ob))) => {
                        let cr = (ob.x - oa.x) * (rep.y - oa.y) - (ob.y - oa.y) * (rep.x - oa.x);
                        let on_material_side = if *is_hole { cr < 0.0 } else { cr > 0.0 };
                        if !on_material_side {
                            return Err(collapsed(CollapseReason::KnifeEdge));
                        }
                    }
                    Some(Curve::Circle(_)) => {}
                    _ => return Err(collapsed(CollapseReason::KnifeEdge)),
                }
            }
            (
                offset_profile.as_slice(),
                top_arrangement,
                top_material,
                cycle_pair,
            )
        }
    };

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
    // `Vertex::new(point + base)` per arrangement vertex of the material
    // boundary, and a NEW top `Vertex::new(top point + tip)` per top
    // arrangement vertex. Distinct instances for coincident geometric points
    // would leave the shell open (the CE-003-MIGRATE trap).
    let mut bottom_vertex: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &v_indices {
        let point = arrangement.vertices.get(v_idx).ok_or(Refusal::Empty)?.point;
        bottom_vertex.insert(v_idx, Vertex::new(point + base));
    }
    let mut top_v_indices: Vec<usize> = Vec::new();
    for cycle in &top_material.boundaries {
        for &h in cycle {
            let he = top_arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
            if !top_v_indices.contains(&he.origin) {
                top_v_indices.push(he.origin);
            }
        }
    }
    let mut top_vertex: HashMap<usize, Vertex> = HashMap::new();
    for &v_idx in &top_v_indices {
        let point = top_arrangement
            .vertices
            .get(v_idx)
            .ok_or(Refusal::Empty)?
            .point;
        top_vertex.insert(v_idx, Vertex::new(point + tip));
    }

    // Bottom and top boundary edges, built ONCE per cycle and shared by every
    // face that references them (rule 4 again: the cap's rect edge IS the side
    // face's bottom edge IS the same instance).
    let mut cycle_bottom: Vec<Vec<Edge>> = Vec::new();
    for cycle in &material.boundaries {
        cycle_bottom.push(cycle_boundary_edges(
            cycle,
            profile,
            arrangement,
            &bottom_vertex,
            base,
        )?);
    }
    let mut cycle_top: Vec<Vec<Edge>> = Vec::new();
    for cycle in &top_material.boundaries {
        cycle_top.push(cycle_boundary_edges(
            cycle,
            top_profile,
            top_arrangement,
            &top_vertex,
            tip,
        )?);
    }

    // The bottom-edge -> top-edge pairing: the top edge lying on the offset
    // carrier of bottom edge i pairs with bottom edge i (carrier identity,
    // not index luck), plus its direction relative to the bottom edge. Also
    // the bottom-vertex -> top-instance map (rule 4: the seam edge, the side
    // face and the top cap share one top instance per bottom vertex).
    let mut top_pos_by_carrier: Vec<HashMap<usize, usize>> = Vec::new();
    if d != 0.0 {
        for top_cycle in &top_material.boundaries {
            let mut map: HashMap<usize, usize> = HashMap::new();
            for (p, &h) in top_cycle.iter().enumerate() {
                let he = top_arrangement.half_edges.get(h).ok_or(Refusal::Empty)?;
                if map.insert(he.curve, p).is_some() {
                    // A carrier split into several top pieces is a topology
                    // event of the offset.
                    return Err(collapsed(CollapseReason::KnifeEdge));
                }
            }
            top_pos_by_carrier.push(map);
        }
    }
    let mut paired_top: Vec<Vec<(Edge, bool)>> = Vec::new();
    let mut top_by_bottom: HashMap<usize, Vertex> = HashMap::new();
    for (j, cycle) in material.boundaries.iter().enumerate() {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        let k = if d == 0.0 {
            j
        } else {
            match cycle_pair.get(j) {
                Some(&k) if k != usize::MAX => k,
                _ => return Err(collapsed(CollapseReason::KnifeEdge)),
            }
        };
        let top_edges = cycle_top.get(k).ok_or(Refusal::Empty)?;
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
            let te = if d == 0.0 {
                match top_edges.get(i) {
                    Some(e) => e.clone(),
                    None => return Err(Refusal::Empty),
                }
            } else {
                let pos = match top_pos_by_carrier
                    .get(k)
                    .and_then(|map| map.get(&he_i.curve))
                {
                    Some(&pos) => pos,
                    None => return Err(collapsed(CollapseReason::KnifeEdge)),
                };
                match top_edges.get(pos) {
                    Some(e) => e.clone(),
                    None => return Err(collapsed(CollapseReason::KnifeEdge)),
                }
            };
            // A circle self-loop pairs by carrier identity; its direction is
            // undefined (front == back).
            let is_circle = matches!(profile.get(he_i.curve), Some(Curve::Circle(_)));
            let (te, forward) = if is_circle {
                (te, true)
            } else {
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
                let dot = (te.back().point() - te.front().point()).dot(pb - pa);
                if dot == 0.0 {
                    return Err(collapsed(CollapseReason::KnifeEdge));
                }
                let forward = dot > 0.0;
                (te, forward)
            };
            let (a_top, b_top) = if forward {
                (te.front().clone(), te.back().clone())
            } else {
                (te.back().clone(), te.front().clone())
            };
            // The pairing must agree on every shared bottom vertex (rule 4).
            for (bv, tv) in [(he_i.origin, a_top), (he_next.origin, b_top)] {
                match top_by_bottom.get(&bv) {
                    Some(prev) if *prev != tv => {
                        return Err(collapsed(CollapseReason::KnifeEdge));
                    }
                    Some(_) => {}
                    None => {
                        top_by_bottom.insert(bv, tv);
                    }
                }
            }
            row.push((te, forward));
        }
        paired_top.push(row);
    }

    // Vertical seams (bottom → top), one per boundary vertex, created lazily
    // and reused so two adjacent side faces share the same instance.
    let mut seams: HashMap<usize, Edge> = HashMap::new();
    let mut faces: Vec<Face> = Vec::new();

    // Bottom cap: surface Plane(origin, +x, +y) lifted to z = base.z, wires =
    // the material region's boundary cycles in order (outer first, holes
    // after), as the arrangement traced them. The face is stored INVERTED
    // (the multi_sweep seed-face convention): the plane's natural normal is
    // +z, but the outward normal of the solid at the bottom cap is −z.
    // Inverting the face also flips its effective boundary edges, which is
    // what the side faces and the walls pair against.
    let bottom_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, base.z),
        Point3::new(1.0, 0.0, base.z),
        Point3::new(0.0, 1.0, base.z),
    ));
    let mut bottom_wires = Vec::new();
    for edges in &cycle_bottom {
        bottom_wires.push(Wire::from(edges.clone()));
    }
    let mut bottom_face =
        Face::try_new(bottom_wires, bottom_surface).map_err(|_| Refusal::Empty)?;
    bottom_face.invert();
    faces.push(bottom_face);

    // Top cap: the top cycles (the identity profile's cycles for a neutral
    // draft, the offset profile's re-arranged cycles otherwise) lifted to
    // z = tip.z, stored in the arrangement's traced direction (NOT reversed)
    // with `orientation == true`, so its outward normal stays +z. The bottom
    // cap is stored inverted, so the two caps' EFFECTIVE boundary edges run
    // opposite — which is exactly what the Closed condition pairs. Built
    // explicitly — never by mapping the bottom wires, because `Wire::mapped`
    // panics on the circle self-loop in debug builds.
    let top_surface = Surface::Plane(Plane::new(
        Point3::new(0.0, 0.0, tip.z),
        Point3::new(1.0, 0.0, tip.z),
        Point3::new(0.0, 1.0, tip.z),
    ));
    let mut top_wires = Vec::new();
    for edges in &cycle_top {
        top_wires.push(Wire::from(edges.clone()));
    }
    faces.push(Face::try_new(top_wires, top_surface).map_err(|_| Refusal::Empty)?);

    // Side faces and walls, one per boundary edge of the material region.
    for (ci, cycle) in material.boundaries.iter().enumerate() {
        let n = cycle.len();
        if n == 0 {
            return Err(Refusal::Empty);
        }
        let bottom_edges = cycle_bottom.get(ci).ok_or(Refusal::Empty)?;
        let is_hole = match cycle_holes.get(ci) {
            Some(is_hole) => *is_hole,
            None => false,
        };
        for i in 0..n {
            let h_i = *cycle.get(i).ok_or(Refusal::Empty)?;
            let he_i = arrangement.half_edges.get(h_i).ok_or(Refusal::Empty)?;
            let be = bottom_edges.get(i).ok_or(Refusal::Empty)?;
            let (te, forward) = match paired_top.get(ci).and_then(|row| row.get(i)) {
                Some((te, forward)) => (te.clone(), *forward),
                None => return Err(Refusal::Empty),
            };
            match profile.get(he_i.curve) {
                // A line boundary edge sweeps to the planar quad on
                // Plane(a, b, a_top) — the plane through the bottom edge and
                // the top edge on the same carrier, spanned by (b − a) and
                // the top offset+sweep — EXACTLY the recognizer's
                // `ExtrudedCurve(Line) → Plane` mapping, built directly, and
                // a canonical Plane in ANY sweep direction.
                Some(Curve::Line(_)) => {
                    let h_next = *cycle.get((i + 1) % n).ok_or(Refusal::Empty)?;
                    let he_next = arrangement.half_edges.get(h_next).ok_or(Refusal::Empty)?;
                    let seam_o = get_or_create_seam(
                        he_i.origin,
                        &bottom_vertex,
                        &top_by_bottom,
                        &mut seams,
                    )?;
                    let seam_n = get_or_create_seam(
                        he_next.origin,
                        &bottom_vertex,
                        &top_by_bottom,
                        &mut seams,
                    )?;
                    let a = arrangement
                        .vertices
                        .get(he_i.origin)
                        .ok_or(Refusal::Empty)?
                        .point
                        + base;
                    let b = arrangement
                        .vertices
                        .get(he_next.origin)
                        .ok_or(Refusal::Empty)?
                        .point
                        + base;
                    let a_top = top_by_bottom
                        .get(&he_i.origin)
                        .ok_or(Refusal::Empty)?
                        .point();
                    let surface = Surface::Plane(Plane::new(a, b, a_top));
                    // The quad [bottom edge, next seam up, top edge reversed,
                    // origin seam down] — the edge instances are SHARED with
                    // the caps (bottom edge with the bottom cap, top edge
                    // with the top cap) and the two adjacent side faces share
                    // each seam (opposite orientation). This pairing matches
                    // the inverted bottom cap and the un-reversed top cap.
                    let top_segment = if forward { te.inverse() } else { te };
                    let wire = Wire::from(vec![be.clone(), seam_n, top_segment, seam_o.inverse()]);
                    faces.push(Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?);
                }
                // A circle boundary edge is the wall of the swept circle: an
                // ANNULUS with two boundary wires (the bottom self-loop, the
                // top self-loop) and NO vertical seam edges. Each circle edge
                // is shared by exactly two faces with opposite orientations
                // (bottom: cap + wall; top: cap + wall), which is what closes
                // the shell. The wall's orientation is keyed on the cycle's
                // containment role: the outer cycle's wall carries the
                // carrier's natural outward normal (stored UNINVERTED), the
                // hole's wall is stored INVERTED. The carrier is a canonical
                // `Cylinder` for a neutral draft, a canonical z-aligned
                // `Cone` (apex on the axis, derived from the bottom circle
                // and the offset top circle) for a draft.
                Some(Curve::Circle(p)) => {
                    let center = p.transform().w.to_point() + base;
                    let radius = p.transform().x.magnitude();
                    let surface = if d == 0.0 {
                        let cylinder = match Cylinder::new(center, radius) {
                            Ok(c) => c.value,
                            Err(_) => return Err(Refusal::Empty),
                        };
                        Surface::Cylinder(cylinder)
                    } else {
                        let top_radius = match top_profile.get(he_i.curve) {
                            Some(Curve::Circle(q)) => q.transform().x.magnitude(),
                            _ => return Err(Refusal::Empty),
                        };
                        let dr = top_radius - radius;
                        if dr == 0.0 {
                            return Err(Refusal::Empty);
                        }
                        let apex_z = base.z - radius * (tip.z - base.z) / dr;
                        let half_angle = (dr / (tip.z - base.z)).abs().atan();
                        if !apex_z.is_finite() || !half_angle.is_finite() {
                            return Err(Refusal::Empty);
                        }
                        match Cone::new(Point3::new(center.x, center.y, apex_z), half_angle) {
                            Ok(c) => Surface::Cone(c.value),
                            Err(_) => return Err(Refusal::Empty),
                        }
                    };
                    let (wire_bot, wire_top) = if !is_hole {
                        (Wire::from(vec![be.clone()]), Wire::from(vec![te.inverse()]))
                    } else {
                        (Wire::from(vec![be.inverse()]), Wire::from(vec![te.clone()]))
                    };
                    let mut wall = Face::try_new(vec![wire_bot, wire_top], surface)
                        .map_err(|_| Refusal::Empty)?;
                    if is_hole {
                        // The hole wall is stored INVERTED: the carrier's
                        // natural normal points away from the axis but the
                        // outward normal of the solid at the hole wall points
                        // into the hole. Inverting the face also flips its
                        // effective boundary edges so the caps' circle
                        // self-loops pair against them.
                        wall.invert();
                    }
                    faces.push(wall);
                }
                _ => return Err(Refusal::Empty),
            }
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

/// Whether cycle `ci` of a region is a hole, by the containment rule (the
/// rule the arrangement's own nesting and `select_material`'s region logic
/// use): a cycle is a hole iff its polygon lies inside another cycle's
/// polygon of the same region. The winding sign is never consulted — S1
/// normalizes every loop to CCW, so winding cannot distinguish a hole from
/// its plate (the session-28 trap).
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

/// The signed 2-D offset of the profile for a draft of `d` (D4): a
/// material-cycle line translates toward the material (outer cycle, the left
/// normal of the CCW trace) or away from the hole's interior (hole cycle) by
/// the signed `d`, and both endpoints extend by `2|d|` along the segment so
/// consecutive offsets cover any corner; a material-cycle circle takes radius
/// `r − d` (outer) or `r + d` (hole). Curves off the material boundary are
/// kept. The result is index-aligned with `profile`, so a top half-edge's
/// carrier index identifies the bottom edge it offsets. A vanished circle
/// radius (r ∓ d <= 0) refuses `Collapsed`.
fn offset_profile_for_taper(
    profile: &[Curve],
    roles: &HashMap<usize, bool>,
    d: f64,
) -> Result<Vec<Curve>, Refusal> {
    let mut out = Vec::with_capacity(profile.len());
    for (idx, curve) in profile.iter().enumerate() {
        let is_hole = match roles.get(&idx) {
            Some(is_hole) => *is_hole,
            None => {
                out.push(curve.clone());
                continue;
            }
        };
        match curve {
            Curve::Line(Line(a, b)) => {
                let dir = *b - *a;
                let len = dir.magnitude();
                if len == 0.0 {
                    return Err(Refusal::Empty);
                }
                let along = dir / len;
                let left = Vector3::new(-dir.y, dir.x, 0.0) / len;
                let sign = if is_hole { -d } else { d };
                let off = sign * left;
                let ext = 2.0 * d.abs() * along;
                out.push(Curve::Line(Line(*a + off - ext, *b + off + ext)));
            }
            Curve::Circle(p) => {
                let radius = p.transform().x.magnitude();
                let next = if is_hole { radius + d } else { radius - d };
                if next <= 0.0 {
                    // The circle boundary collapsed: the apex-vanishing of the
                    // tapered wall cone.
                    return Err(collapsed(CollapseReason::ApexVanishing));
                }
                let mut m = *p.transform();
                let scale = next / radius;
                m.x *= scale;
                m.y *= scale;
                out.push(Curve::Circle(Processor::with_transform(*p.entity(), m)));
            }
            _ => return Err(Refusal::Empty),
        }
    }
    Ok(out)
}

/// The carrier index of every half-edge of a cycle, in cycle order.
fn cycle_carriers(cycle: &[usize], arrangement: &Arrangement) -> Vec<usize> {
    cycle
        .iter()
        .filter_map(|&h| arrangement.half_edges.get(h).map(|he| he.curve))
        .collect()
}

/// Removes consecutive repeated carriers (an arc-split circle).
fn collapse_repeats(seq: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for c in seq {
        match out.last() {
            Some(last) if last == c => {}
            _ => out.push(*c),
        }
    }
    out
}

/// Whether the carrier sequences are equal up to rotation (after collapsing
/// consecutive repeats).
fn cyclic_eq(a: &[usize], b: &[usize]) -> bool {
    let x = collapse_repeats(a);
    let y = collapse_repeats(b);
    if x.len() != y.len() {
        return false;
    }
    let n = x.len();
    (0..n).any(|s| (0..n).all(|t| x.get(t) == y.get((s + t) % n)))
}

/// The boundary edges of one cycle, in cycle order: `origin(h_i) →
/// origin(h_{i+1})`, translated by `offset`. A line piece becomes a
/// `Curve::Line` through the translated vertex points; a circle piece keeps
/// the profile's circle processor translated by `offset`.
fn cycle_boundary_edges(
    cycle: &[usize],
    profile: &[Curve],
    arrangement: &Arrangement,
    vertex: &HashMap<usize, Vertex>,
    offset: Vector3,
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
        let curve = match profile.get(he_i.curve) {
            Some(Curve::Line(_)) => Curve::Line(Line(v0.point(), v1.point())),
            Some(Curve::Circle(p)) => {
                let mut m = *p.transform();
                m.w.x += offset.x;
                m.w.y += offset.y;
                m.w.z += offset.z;
                Curve::Circle(Processor::with_transform(*p.entity(), m))
            }
            _ => return Err(Refusal::Empty),
        };
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

/// The seam edge (bottom → top) of a boundary vertex, created once and
/// reused by the two adjacent side faces (rule 4). The top instance is the
/// paired top corner of the bottom vertex (the translated top vertex for a
/// neutral draft, the miter corner of the offset top profile otherwise).
fn get_or_create_seam(
    v_idx: usize,
    bottom_vertex: &HashMap<usize, Vertex>,
    top_by_bottom: &HashMap<usize, Vertex>,
    seams: &mut HashMap<usize, Edge>,
) -> Result<Edge, Refusal> {
    if let Some(e) = seams.get(&v_idx) {
        return Ok(e.clone());
    }
    let b = bottom_vertex.get(&v_idx).ok_or(Refusal::Empty)?;
    let t = top_by_bottom.get(&v_idx).ok_or(Refusal::Empty)?;
    let edge =
        Edge::try_new(b, t, Curve::Line(Line(b.point(), t.point()))).map_err(|_| Refusal::Empty)?;
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

    // ---- BG-CAD-P2-EXTRUDE: generalized extrusion (vector / both / taper) ----

    /// A line-only `s × s` CCW rectangle at z = 0 with its arrangement.
    fn rect_profile(s: f64) -> (Vec<Curve>, Arrangement) {
        let profile = vec![
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(s, 0.0, 0.0))),
            Curve::Line(Line(Point3::new(s, 0.0, 0.0), Point3::new(s, s, 0.0))),
            Curve::Line(Line(Point3::new(s, s, 0.0), Point3::new(0.0, s, 0.0))),
            Curve::Line(Line(Point3::new(0.0, s, 0.0), Point3::new(0.0, 0.0, 0.0))),
        ];
        let arrangement = arrange(&profile, None).unwrap().value;
        (profile, arrangement)
    }

    /// The disk profile: a full circle r = 1 at (2, 2) with its arrangement.
    fn disk_profile() -> (Vec<Curve>, Arrangement) {
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
        (profile, arrangement)
    }

    /// The exact bounding corners of a solid: the min/max vertex coordinates.
    fn solid_corners(solid: &Solid) -> (Point3, Point3) {
        let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for v in solid.vertex_iter() {
            let p = v.point();
            lo.x = lo.x.min(p.x);
            lo.y = lo.y.min(p.y);
            lo.z = lo.z.min(p.z);
            hi.x = hi.x.max(p.x);
            hi.y = hi.y.max(p.y);
            hi.z = hi.z.max(p.z);
        }
        (lo, hi)
    }

    /// The boundary vertex points of a face, deduplicated and sorted.
    fn face_corner_points(face: &Face) -> Vec<Point3> {
        let mut pts: Vec<Point3> = Vec::new();
        for wire in face.boundaries() {
            for edge in wire.edge_iter() {
                for p in [edge.front().point(), edge.back().point()] {
                    if !pts.contains(&p) {
                        pts.push(p);
                    }
                }
            }
        }
        pts.sort_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then(a.y.total_cmp(&b.y))
                .then(a.z.total_cmp(&b.z))
        });
        pts
    }

    /// The face whose plane is horizontal (normal parallel to z) with the
    /// plane origin at z = `z` — the cap on that plane.
    fn cap_at(solid: &Solid, z: f64) -> Option<Face> {
        for face in solid.face_iter() {
            if let Surface::Plane(plane) = face.surface() {
                let horizontal = plane.normal().cross(Vector3::unit_z()).magnitude() == 0.0;
                if horizontal && plane.origin().z == z {
                    return Some(face.clone());
                }
            }
        }
        None
    }

    /// Re-passes the closure validation on a solid's boundary shell.
    fn assert_shell_closes(solid: &Solid) {
        let shell = solid
            .boundaries()
            .first()
            .expect("one boundary shell")
            .clone();
        assert!(Solid::try_new(vec![shell]).is_ok());
    }

    #[test]
    fn vector_z_matches_scalar_extrude() {
        let (profile, arrangement) = rect_profile(4.0);
        let scalar = extrude_profile(&profile, &arrangement, 2.0).unwrap().value;
        let vector =
            extrude_profile_vector(&profile, &arrangement, Vector3::new(0.0, 0.0, 2.0), false)
                .unwrap()
                .value;
        // Congruent: same face count, same bounding corners, both accepted by
        // `Solid::try_new`.
        assert_eq!(scalar.face_iter().count(), vector.face_iter().count());
        assert_eq!(scalar.face_iter().count(), 6);
        assert_eq!(solid_corners(&scalar), solid_corners(&vector));
        let (lo, hi) = solid_corners(&scalar);
        assert_eq!(lo, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(hi, Point3::new(4.0, 4.0, 2.0));
        assert_shell_closes(&scalar);
        assert_shell_closes(&vector);
    }

    #[test]
    fn oblique_extrude_of_polygon_is_planar_sided() {
        let (profile, arrangement) = rect_profile(4.0);
        let solid =
            extrude_profile_vector(&profile, &arrangement, Vector3::new(1.0, 0.0, 1.0), false)
                .unwrap()
                .value;
        assert_eq!(solid.face_iter().count(), 6);
        let mut sides = 0usize;
        for face in solid.face_iter() {
            let Surface::Plane(plane) = face.surface() else {
                unreachable!("an oblique polygon extrude must be planar-sided");
            };
            let is_cap = plane.normal().cross(Vector3::unit_z()).magnitude() == 0.0;
            if !is_cap {
                sides += 1;
                // Every side carrier recognizes to an exact canonical Plane.
                let witness = recognize_surface(&face.surface());
                assert!(matches!(
                    witness,
                    CanonicalCarrierWitness::ExactCanonical {
                        carrier: CanonicalCarrier::Surface(CanonicalSurface::Plane(_)),
                        ..
                    }
                ));
            }
        }
        assert_eq!(sides, 4);
        // The top cap is the profile translated by the sweep (1, 0, 1).
        let top = cap_at(&solid, 1.0).expect("a top cap on z = 1");
        let expected = vec![
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 4.0, 1.0),
            Point3::new(5.0, 0.0, 1.0),
            Point3::new(5.0, 4.0, 1.0),
        ];
        assert_eq!(face_corner_points(&top), expected);
        assert_shell_closes(&solid);
    }

    #[test]
    fn both_extrude_is_symmetric_interval() {
        let (profile, arrangement) = rect_profile(4.0);
        let solid =
            extrude_profile_vector(&profile, &arrangement, Vector3::new(0.0, 0.0, 2.0), true)
                .unwrap()
                .value;
        assert_eq!(solid.face_iter().count(), 6);
        // The box spans z ∈ [−h, +h] exactly.
        let (lo, hi) = solid_corners(&solid);
        assert_eq!(lo, Point3::new(0.0, 0.0, -2.0));
        assert_eq!(hi, Point3::new(4.0, 4.0, 2.0));
        assert_shell_closes(&solid);
    }

    #[test]
    fn oblique_circle_refuses_noncanonical() {
        let (profile, arrangement) = disk_profile();
        let err =
            extrude_profile_vector(&profile, &arrangement, Vector3::new(1.0, 0.0, 1.0), false)
                .unwrap_err();
        assert!(matches!(
            err,
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        ));
    }

    #[test]
    fn taper_rectangle_top_is_offset() {
        let (profile, arrangement) = rect_profile(4.0);
        // tan(taper) = 0.5, height 1: the top cap is the 0.5-inset rectangle.
        let taper = f64::atan(0.5);
        let solid = extrude_profile_taper(&profile, &arrangement, 1.0, taper)
            .unwrap()
            .value;
        assert_eq!(solid.face_iter().count(), 6);
        for face in solid.face_iter() {
            assert!(matches!(face.surface(), Surface::Plane(_)));
        }
        let bottom = cap_at(&solid, 0.0).expect("a bottom cap on z = 0");
        let expected_bottom = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 4.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            Point3::new(4.0, 4.0, 0.0),
        ];
        assert_eq!(face_corner_points(&bottom), expected_bottom);
        let top = cap_at(&solid, 1.0).expect("a top cap on z = 1");
        let top_pts = face_corner_points(&top);
        assert_eq!(top_pts.len(), 4);
        let expected_top = [(0.5, 0.5), (0.5, 3.5), (3.5, 0.5), (3.5, 3.5)];
        for (p, (x, y)) in top_pts.iter().zip(expected_top.iter()) {
            assert!(
                (p.x - x).abs() <= TOLERANCE && (p.y - y).abs() <= TOLERANCE && p.z == 1.0,
                "top corner {p:?} is not the expected inset corner ({x}, {y})"
            );
        }
        assert_shell_closes(&solid);
    }

    #[test]
    fn taper_circle_side_is_canonical_cone() {
        let (profile, arrangement) = disk_profile();
        // tan(taper) = 0.25, height 2: the top radius is r − d = 0.5.
        let taper = f64::atan(0.25);
        let solid = extrude_profile_taper(&profile, &arrangement, 2.0, taper)
            .unwrap()
            .value;
        let mut cones = 0usize;
        let mut planes = 0usize;
        let mut top_radius = None;
        for face in solid.face_iter() {
            match face.surface() {
                Surface::Cone(_) => {
                    cones += 1;
                    // The side carrier recognizes to an exact canonical Cone.
                    let witness = recognize_surface(&face.surface());
                    let CanonicalCarrierWitness::ExactCanonical { carrier, .. } = witness else {
                        unreachable!("the cone wall must recognize exactly");
                    };
                    let CanonicalCarrier::Surface(CanonicalSurface::Cone(cone)) = carrier else {
                        unreachable!("expected a Cone carrier");
                    };
                    // The apex sits on the circle's axis; for r = 1 at z = 0
                    // and r' = 0.5 at z = 2 the apex is at z = 4 and the half
                    // angle is atan(0.25) = the draft itself.
                    assert_eq!(cone.apex().x, 2.0);
                    assert_eq!(cone.apex().y, 2.0);
                    assert!((cone.apex().z - 4.0).abs() <= TOLERANCE);
                    assert!((cone.half_angle() - taper).abs() <= TOLERANCE);
                }
                Surface::Plane(plane) => {
                    planes += 1;
                    if plane.origin().z == 2.0 {
                        for wire in face.boundaries() {
                            for edge in wire.edge_iter() {
                                if let Curve::Circle(p) = edge.curve() {
                                    top_radius = Some(p.transform().x.magnitude());
                                }
                            }
                        }
                    }
                }
                _ => unreachable!("unexpected surface"),
            }
        }
        assert_eq!(cones, 1);
        assert_eq!(planes, 2);
        let r = top_radius.expect("a top circle edge");
        assert!((r - 0.5).abs() <= TOLERANCE);
        assert_shell_closes(&solid);
    }

    #[test]
    fn taper_topology_event_refuses_collapsed() {
        let (profile, arrangement) = rect_profile(4.0);
        // tan(taper) = 3, height 1: d = 3 >= 2 on the 4-wide rectangle — the
        // inset is past the collapse (the re-arranged offset square is
        // inverted). A topology event, refused as `Collapsed`.
        let err = extrude_profile_taper(&profile, &arrangement, 1.0, f64::atan(3.0)).unwrap_err();
        assert!(matches!(err, Refusal::Collapsed(..)));
    }

    /// The hole-grows fixture: a 4×4 CCW rectangle minus a full circle
    /// r = 0.75 at (2, 2). With the draft d = 0.5 the grown top radius
    /// r + d = 1.25 stays strictly clear of the 0.5-inset boundary lines
    /// (distance 1.5), so the offset re-arrangement is exact.
    fn plate_with_small_hole() -> (Vec<Curve>, Arrangement) {
        let circle = Curve::Circle(Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            Matrix4 {
                x: Vector4::new(0.75, 0.0, 0.0, 0.0),
                y: Vector4::new(0.0, 0.75, 0.0, 0.0),
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
        let arrangement = arrange(&profile, None).unwrap().value;
        (profile, arrangement)
    }

    #[test]
    fn taper_hole_grows() {
        let (profile, arrangement) = plate_with_small_hole();
        // tan(taper) = 0.5, height 1: d = 0.5 and the hole's top radius is
        // r + d = 0.75 + 0.5 = 1.25.
        let taper = f64::atan(0.5);
        let d = taper.tan();
        let solid = extrude_profile_taper(&profile, &arrangement, 1.0, taper)
            .unwrap()
            .value;
        let top = cap_at(&solid, 1.0).expect("a top cap on z = 1");
        assert_eq!(top.boundaries().len(), 2);
        let mut radius = None;
        for wire in top.boundaries() {
            for edge in wire.edge_iter() {
                if let Curve::Circle(p) = edge.curve() {
                    radius = Some(p.transform().x.magnitude());
                }
            }
        }
        let r = radius.expect("the hole's top circle");
        assert!((r - (0.75 + d)).abs() <= TOLERANCE);
        assert_shell_closes(&solid);
    }

    #[test]
    fn zero_height_vector_refuses_empty() {
        let (profile, arrangement) = rect_profile(4.0);
        let err =
            extrude_profile_vector(&profile, &arrangement, Vector3::new(3.0, 0.0, 0.0), false)
                .unwrap_err();
        assert!(matches!(err, Refusal::Empty));
        let err = extrude_profile_vector(&profile, &arrangement, Vector3::new(3.0, 0.0, 0.0), true)
            .unwrap_err();
        assert!(matches!(err, Refusal::Empty));
    }

    #[test]
    fn negative_taper_expands_material() {
        // tan(taper) = −0.5, height 1: d = −0.5 and the top cap is the
        // 0.5-outset rectangle of the 6×6 base.
        let (profile, arrangement) = rect_profile(6.0);
        let solid = extrude_profile_taper(&profile, &arrangement, 1.0, -f64::atan(0.5))
            .unwrap()
            .value;
        let top = cap_at(&solid, 1.0).expect("a top cap on z = 1");
        let top_pts = face_corner_points(&top);
        assert_eq!(top_pts.len(), 4);
        let expected_top = [(-0.5, -0.5), (-0.5, 6.5), (6.5, -0.5), (6.5, 6.5)];
        for (p, (x, y)) in top_pts.iter().zip(expected_top.iter()) {
            assert!(
                (p.x - x).abs() <= TOLERANCE && (p.y - y).abs() <= TOLERANCE && p.z == 1.0,
                "top corner {p:?} is not the expected outset corner ({x}, {y})"
            );
        }
        assert_shell_closes(&solid);
    }
}
