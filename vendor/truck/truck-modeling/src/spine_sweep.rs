#![deny(clippy::unwrap_used)]

//! BG-CG-009-BREP — the authored-topology sweep constructor.
//!
//! [`spine_sweep`] realizes a landed `SpineFrameRecipe` as a closed
//! `Solid<Point3, Curve, Surface>` with authored topology (build-spec §8B;
//! plan §4 CG-009): ONE side BREP face per profile edge — never one face per
//! spine sample — each carrying a whole-sweep closed value
//! ([`SpineFrameSweep`]) over `[s_first, s_last] × [v0_j, v1_j]` (the windowed
//! realization decorator is derived from that closed value, never stored
//! separately; BG-KV2-501-C6), its wire built from the SHARED trajectory
//! `Edge`s (each [`SpineFrameCurve`] is constructed once and cloned into both
//! adjacent faces' wires — identity, not coordinates), straight ring edges at
//! the first/last stations, and planar caps via the landed
//! [`builder::try_attach_plane`]. No sewing, no welding, no healing anywhere.
//!
//! V1 domain (pre-decided; outside it, typed refusals — never silent
//! clamping): `ProfileLaw::Constant` and `ProfileLaw::Scale`
//! (non-through-zero uniform scale) with straight profile edges;
//! `LinearCorrespondence` is accepted when both profiles are straight-edged
//! with identical edge counts. Curved profile edges, through-zero scale, and
//! mismatched correspondence refuse `ConstructError::InvalidInput` at the
//! entry — a booked boundary, not a bug. The spine may be any landed `SpineCurve`
//! (the C1 gate is the landed `PolylineSpine` refusal, which fires during the
//! recipe validation pass before any storage spine is converted); all four
//! landed frame laws are accepted (their singularity refusals are the landed
//! ones). Stations are sorted, deduped, at least two — the landed facet
//! entry's validation, reused verbatim.
//!
//! The landed `facet_sweep` is untouched: the facet backend is the rendering
//! fast path, this module is the topology stage.

use crate::{
    builder, Curve, Edge, Face, Line, Shell, Solid, SpineFrameCurve, Surface, Vertex, Wire,
};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_geometry::constructive::{
    ConstructError, DirectTolerance, FrameLaw, ProfileLaw, ScalarLaw, SpineCurve, SpineFrameRecipe,
    SpineFrameSweep,
};

/// The authored-topology sweep constructor (build-spec §8B; plan §4 CG-009).
///
/// Side faces per profile edge, trajectory edges shared by identity, caps via
/// `try_attach_plane`. No sewing anywhere.
///
/// # Failures
/// Every refusal is typed: invalid stations, a profile law outside the V1
/// domain, a recipe that refuses at a station (the C1/frame gates), a
/// nonplanar cap, or a shell `Solid::try_new` rejects all return a `Refusal`.
pub fn spine_sweep<S: SpineCurve + Into<Curve> + Clone>(
    recipe: &SpineFrameRecipe<S, ProfileLaw, FrameLaw>,
    stations: &[f64],
) -> Outcome<Solid> {
    // 1. Station validation — the landed facet entry's checks, verbatim.
    if stations.len() < 2 {
        return Err(refuse(ConstructError::InvalidInput));
    }
    if let Some(&bad) = stations.iter().find(|s| !s.is_finite()) {
        return Err(refuse(ConstructError::NonFinite { at: bad }));
    }
    if stations.windows(2).any(|w| w[1] <= w[0]) {
        return Err(refuse(ConstructError::InvalidInput));
    }
    let parameter_tol = DirectTolerance::default().parameter;
    let (s_min, s_max) = recipe.spine.domain();
    if stations
        .iter()
        .any(|&s| s < s_min - parameter_tol || s > s_max + parameter_tol)
    {
        return Err(refuse(ConstructError::InvalidInput));
    }
    let s_first = stations[0];
    let s_last = stations[stations.len() - 1];

    // 2. V1 profile-law domain validation. Straight profile edges only:
    // every landed `Profile2D` edge is straight (vertices connected by
    // segments), so the check is the law-family + scale/correspondence gates;
    // a future curved-edge law refuses here (a booked boundary).
    let k = profile_vertex_count(recipe);
    if k < 3 {
        return Err(refuse(ConstructError::InvalidInput));
    }
    match &recipe.profile_law {
        ProfileLaw::Constant(_) => {}
        ProfileLaw::Scale { scale, .. } => {
            if scale_touches_zero(scale, s_first, s_last) {
                // Through-zero scale collapses the profile (booked boundary):
                // refuse `InvalidInput` at the entry, never silent clamping.
                return Err(refuse(ConstructError::InvalidInput));
            }
        }
        ProfileLaw::LinearCorrespondence { start, end } => {
            if start.vertices.len() != end.vertices.len() {
                // Mismatched correspondence (booked boundary): refused at the
                // entry, never inferred.
                return Err(refuse(ConstructError::InvalidInput));
            }
        }
    }

    // 3. Recipe validation over the station window: every frame and profile
    // evaluation must succeed — the C1/FrameSingular/ProfileCollapse gates.
    for &s in stations {
        recipe.frame(s).map_err(refuse)?;
    }
    for &s in stations {
        recipe.position(s, 0.0).map_err(refuse)?;
        recipe.position(s, 1.0).map_err(refuse)?;
    }

    // 4. Storage spine: the recipe's spine converted to its canonical `Curve`
    // carrier, boxed (the closed enums store the decorators at `Box<Curve>`,
    // the indirection that breaks the enum recursion). All evaluation for the
    // stored surfaces/curves happens on this converted recipe; the gates above
    // ran on the ORIGINAL spine, so a non-C1 polyline still refuses.
    let storage_recipe = SpineFrameRecipe::new(
        Box::new(recipe.spine.clone().into()),
        recipe.profile_law.clone(),
        recipe.frame_law,
    );

    // 5. Ring vertices at the first and last stations — ONE `Vertex` instance
    // per profile vertex, shared by the two adjacent side faces and the cap.
    let mut start_vertex: Vec<Vertex> = Vec::with_capacity(k);
    let mut end_vertex: Vec<Vertex> = Vec::with_capacity(k);
    for j in 0..k {
        let v = ring_parameter(j, k);
        let start = storage_recipe.position(s_first, v).map_err(refuse)?;
        let end = storage_recipe.position(s_last, v).map_err(refuse)?;
        start_vertex.push(Vertex::new(start));
        end_vertex.push(Vertex::new(end));
    }

    // 6. Trajectory edges E_j, constructed ONCE and cloned into both adjacent
    // faces' wires (the `SpineFrameCurve` handle is the same identity; a face
    // uses `inverse()` for the opposite orientation).
    let mut trajectory: Vec<Edge> = Vec::with_capacity(k);
    for j in 0..k {
        let v = ring_parameter(j, k);
        let curve =
            SpineFrameCurve::try_new(storage_recipe.clone(), s_first, s_last, v).map_err(refuse)?;
        trajectory.push(
            Edge::try_new(
                &start_vertex[j],
                &end_vertex[j],
                Curve::SpineFrameCurve(curve),
            )
            .map_err(|_| refuse(ConstructError::InvalidInput))?,
        );
    }

    // 7. Ring edges at the first and last stations, constructed ONCE per
    // profile edge and shared by the side face and the cap (a straight profile
    // edge under a rigid frame / uniform scale stays straight — asserted in
    // the conformance test).
    let mut start_ring: Vec<Edge> = Vec::with_capacity(k);
    let mut end_ring: Vec<Edge> = Vec::with_capacity(k);
    for j in 0..k {
        let j2 = (j + 1) % k;
        start_ring.push(
            Edge::try_new(
                &start_vertex[j],
                &start_vertex[j2],
                Curve::Line(Line(start_vertex[j].point(), start_vertex[j2].point())),
            )
            .map_err(|_| refuse(ConstructError::InvalidInput))?,
        );
        end_ring.push(
            Edge::try_new(
                &end_vertex[j],
                &end_vertex[j2],
                Curve::Line(Line(end_vertex[j].point(), end_vertex[j2].point())),
            )
            .map_err(|_| refuse(ConstructError::InvalidInput))?,
        );
    }

    // 8. Side faces, one per profile edge j. The wire traverses the parameter
    // rectangle [s_first, s_last] × [v0_j, v1_j] CCW: the shared trajectory
    // edge, the end ring edge, the next trajectory edge reversed, the start
    // ring edge reversed. Every shared edge appears with OPPOSITE orientations
    // in its two faces (the P12 fixture rule). The face's stored surface is the
    // whole-sweep closed value over THAT face's window — the enum now carries
    // the sweep, window and all, not the windowed realization decorator
    // (BG-KV2-501-C6); the sweep's placement stays the identity the constructor
    // sets, shared across every face of one sweep (stop condition 2's guard).
    let mut faces: Vec<Face> = Vec::with_capacity(k + 2);
    for j in 0..k {
        let j2 = (j + 1) % k;
        let v0 = ring_parameter(j, k);
        // The v-window endpoint of edge j is (j+1)/k, NOT cyclic: the closing
        // edge (j = k-1) spans [ (k-1)/k, 1.0 ].
        let v1 = (j + 1) as f64 / k as f64;
        let sweep = SpineFrameSweep::try_new(storage_recipe.clone(), s_first, s_last, v0, v1)
            .map_err(refuse)?;
        let wire = Wire::from(vec![
            trajectory[j].clone(),
            end_ring[j].clone(),
            trajectory[j2].inverse(),
            start_ring[j].inverse(),
        ]);
        faces.push(
            Face::try_new(vec![wire], Surface::SpineFrameSurface(sweep))
                .map_err(|_| refuse(ConstructError::InvalidInput))?,
        );
    }

    // 9. Caps. The start cap consumes the start ring edges as-is (the side
    // faces use them reversed); the end cap consumes the end ring edges in
    // REVERSED order, each inverted, so the wire closes and every shared edge
    // stays opposite to its side-face use. A nonplanar ring refuses here
    // (`WireNotInOnePlane`) — typed, never silent. The cap face's surface is
    // the fitted `Plane`, so the cap is geometric-consistent by construction.
    let start_wire: Wire = Wire::from(start_ring);
    let end_wire: Wire = Wire::from(
        end_ring
            .iter()
            .rev()
            .map(|edge| edge.inverse())
            .collect::<Vec<_>>(),
    );
    let start_cap = builder::try_attach_plane(vec![start_wire]).map_err(refuse_topology)?;
    let end_cap = builder::try_attach_plane(vec![end_wire]).map_err(refuse_topology)?;
    faces.push(start_cap);
    faces.push(end_cap);

    // 10. Assembly through the landed validation path (the debug-only
    // face constructor is BANNED here — GATE-3/H-4). `Solid::try_new` refuses a shell whose
    // shared edges do not pair by identity; a refusal is a contradiction
    // witness, never weakened validation.
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

/// The number of profile ring vertices.
fn profile_vertex_count(recipe: &SpineFrameRecipe<impl SpineCurve, ProfileLaw, FrameLaw>) -> usize {
    match &recipe.profile_law {
        ProfileLaw::Constant(profile) => profile.vertices.len(),
        ProfileLaw::Scale { profile, .. } => profile.vertices.len(),
        ProfileLaw::LinearCorrespondence { start, .. } => start.vertices.len(),
    }
}

/// The ring parameter of profile vertex `j` out of `k`.
fn ring_parameter(j: usize, k: usize) -> f64 {
    j as f64 / k as f64
}

/// Whether a `ScalarLaw` reaches zero anywhere on `[s_first, s_last]`: a sign
/// change or an exact zero of the linear interpolation. A through-zero scale
/// collapses the profile (refused at the entry, V1).
fn scale_touches_zero(scale: &ScalarLaw, s_first: f64, s_last: f64) -> bool {
    match *scale {
        ScalarLaw::Constant(c) => c == 0.0,
        ScalarLaw::Linear { start, end } => {
            let a = start + (end - start) * s_first;
            let b = start + (end - start) * s_last;
            (a <= 0.0 && 0.0 <= b) || (b <= 0.0 && 0.0 <= a)
        }
    }
}

/// The construction-refusal mapping at the realization entry (CG-007's
/// pattern, mapping A row 1): every `ConstructError` becomes the constructive-
/// refusal envelope. The detailed `ConstructError` rides the realization
/// evidence record when that record lands (CG-007); until then the envelope
/// case — not the dropped detail — is what the closed `Refusal` can carry.
///
/// The match NAMES EVERY `ConstructError` variant with no catch-all arm: the
/// compiler proves the mapping is total (a new `ConstructError` variant stops
/// the build here for an explicit decision), and `refusal_mapping_is_total_no_
/// catch_all` asserts that property textually. Every landed variant has an
/// honest envelope case — `ConstructRefused`, the constructive-realization
/// envelope this entry point realizes — so stop condition 3 does not fire.
fn refuse(error: ConstructError) -> Refusal {
    match error {
        ConstructError::ZeroTangent { .. }
        | ConstructError::FrameSingular { .. }
        | ConstructError::SpineNotC1 { .. }
        | ConstructError::ProfileCorrespondenceMismatch
        | ConstructError::ProfileCollapse { .. }
        | ConstructError::NonFinite { .. }
        | ConstructError::InvalidInput => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused)
        }
    }
}

/// The cap-attachment refusal mapping: a nonplanar cap is outside the V1
/// envelope (typed, never silent clamping).
fn refuse_topology(error: crate::errors::Error) -> Refusal {
    match error {
        crate::errors::Error::WireNotInOnePlane => {
            Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
        }
        _other => Refusal::Contradictory(ContradictionWitness {
            prop: Prop::CoedgePairing,
            left: Truth::True,
            right: Truth::False,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::*;
    use truck_geometry::constructive::{LineSpine, Profile2D};

    /// The unit-square profile (CCW about +z in the frame plane).
    fn unit_square() -> Profile2D {
        match Profile2D::try_closed(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ]) {
            Ok(profile) => profile,
            Err(_) => panic!("the unit-square fixture must be a valid closed profile"),
        }
    }

    /// The recipe of a unit prism: `LineSpine` of height 1 along +z, the unit
    /// square profile, and a `FixedPlane` frame pinned to +x (the sweep is a
    /// 1×1×1 box, profile mapped to `(py, -px)`).
    fn unit_prism_recipe() -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
        let spine = LineSpine {
            start: Point3::origin(),
            end: Point3::new(0.0, 0.0, 1.0),
        };
        let frame = FrameLaw::FixedPlane {
            normal: Vector3::unit_x(),
        };
        SpineFrameRecipe::new(spine, ProfileLaw::Constant(unit_square()), frame)
    }

    #[test]
    fn refusal_mapping_is_total_no_catch_all() {
        // Every `ConstructError` variant maps to its honest envelope case —
        // `ConstructRefused`, the constructive-realization envelope (CG-000,
        // mapping A row 1). Building the refusal for every variant by name
        // exercises each arm.
        let errors = [
            ConstructError::ZeroTangent { at: 0.0 },
            ConstructError::FrameSingular {
                at: 0.0,
                law: "FixedPlane",
            },
            ConstructError::SpineNotC1 { at: 1.0 },
            ConstructError::ProfileCorrespondenceMismatch,
            ConstructError::ProfileCollapse { at: 0.5 },
            ConstructError::NonFinite { at: 0.0 },
            ConstructError::InvalidInput,
        ];
        for error in errors {
            match refuse(error) {
                Refusal::UnsupportedEnvelope(EnvelopeCase::ConstructRefused) => {}
                other => panic!("ConstructError::{other:?} refused the wrong envelope"),
            }
        }
        // The no-catch-all property, asserted textually: the `refuse` body must
        // name every variant and contain no wildcard (`_`) match arm — the
        // compiler already proves totality; this pins the arm shape so a later
        // "simplification" to a catch-all cannot land silently.
        let source = include_str!("spine_sweep.rs");
        let after_signature = match source.split_once("fn refuse(error: ConstructError)") {
            Some((_, tail)) => tail,
            None => panic!("the refuse signature is not present"),
        };
        let open_brace = match after_signature.find('{') {
            Some(index) => index,
            None => panic!("the refuse body has no opening brace"),
        };
        let mut depth = 0i32;
        let mut close = None;
        for (i, ch) in after_signature[open_brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open_brace + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = match close {
            Some(close) => &after_signature[open_brace..=close],
            None => panic!("the refuse body never closes"),
        };
        for name in [
            "ZeroTangent",
            "FrameSingular",
            "SpineNotC1",
            "ProfileCorrespondenceMismatch",
            "ProfileCollapse",
            "NonFinite",
            "InvalidInput",
        ] {
            assert!(
                body.contains(name),
                "the refuse body must name ConstructError::{name}"
            );
        }
        assert!(
            !body.contains("_ =>"),
            "refuse must have no wildcard match arm"
        );
        assert!(
            !body.contains("_other"),
            "refuse must have no named catch-all"
        );
    }

    #[test]
    fn landed_brep_constructor_semantics_unchanged() {
        // The landed constructor semantics are immovable: ONE side face per
        // profile edge carrying the whole-sweep closed value over exactly that
        // face's window, planar caps, every side sweep's placement the identity
        // the constructor sets. The enum payload moved onto the sweep
        // (BG-KV2-501-C6), but this is what the volume conformance measures,
        // so it must not move.
        let recipe = unit_prism_recipe();
        let certified = match spine_sweep(&recipe, &[0.0, 1.0]) {
            Ok(certified) => certified,
            Err(refusal) => panic!("the landed unit-prism sweep refused: {refusal:?}"),
        };
        let solid = certified.value;
        assert_eq!(solid.boundaries()[0].len(), 6, "4 side faces + 2 caps");
        let mut side_windows: Vec<(f64, f64)> = Vec::new();
        let mut cap_faces = 0usize;
        let tolerance = DirectTolerance::default().position;
        for face in solid.face_iter() {
            match face.surface() {
                Surface::SpineFrameSurface(sweep) => {
                    // The sweep-level placement is the shared identity the
                    // constructor sets — never per-face differing transforms
                    // (stop condition 2's guard).
                    assert_eq!(*sweep.transform(), Matrix4::identity());
                    assert_eq!(sweep.s0(), 0.0);
                    assert_eq!(sweep.s1(), 1.0);
                    side_windows.push((sweep.v0(), sweep.v1()));
                    // The per-face realization still evaluates the recipe over
                    // the window: X(s, v) = C(s) + n·px + b·py = (py, -px, s).
                    let s = 0.5;
                    let v = 0.5 * (sweep.v0() + sweep.v1());
                    let expected = edge_midpoint_position(sweep.v0(), sweep.v1(), s);
                    let got = sweep.subs(s, v);
                    assert!(
                        (got - expected).magnitude() <= tolerance,
                        "side realization diverged at ({s}, {v}): {got:?} vs {expected:?}"
                    );
                }
                Surface::Plane(_) => cap_faces += 1,
                other => panic!("a side face must be a sweep or a plane cap, got {other:?}"),
            }
        }
        side_windows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(
            side_windows,
            vec![(0.0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.0),],
            "the four side windows must tile the profile ring exactly once"
        );
        assert_eq!(cap_faces, 2, "both caps must attach as planar faces");
    }

    /// The analytic unit-square position at ring parameter `v` of the edge
    /// whose window is `[v0, v1]`, mapped through the prism fixture's frame:
    /// `X(s, v) = (py, -px, s)`.
    fn edge_midpoint_position(v0: f64, v1: f64, s: f64) -> Point3 {
        let vertices = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let j = (v0 * 4.0).round() as usize % 4;
        let a = vertices[j];
        let b = vertices[(j + 1) % 4];
        let width = v1 - v0;
        let v = v0 + 0.5 * width;
        let w = (v - v0) / width;
        let px = a.0 + (b.0 - a.0) * w;
        let py = a.1 + (b.1 - a.1) * w;
        Point3::new(py, -px, s)
    }
}
