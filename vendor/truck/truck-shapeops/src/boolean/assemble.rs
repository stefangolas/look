//! BG-SOL-RW4-ASSEMBLE — the assembler and the `boolean()` entry (the
//! Boundary Rewrite's final topology packet).
//!
//! `boolean()` composes the whole pipeline: the single-shell guard, the lift
//! (`recognize_surface`/`recognize_curve` → bounded canonical strata), the
//! AABB-screened sweep over every cross-solid stratum pair
//! ([`sweep_contact_events`]), the splitter, the classifier, and the decision
//! + sewing of the kept fragments ([`fragment_decision`]). Every design
//! decision was prototyped and measured by `scratch/rw3probe` against the
//! landed splitter's six-event flagship mesh; the flagship sweep reproduces
//! exactly the six events and all four ops assemble.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use truck_base::cgmath64::Point3;
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};
use truck_evidence::contact::{contact, face_stratum, BoundedStratum};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::recognize::{
    recognize_curve, recognize_surface, CanonicalCarrier, CanonicalCarrierWitness,
};
use truck_geotrait::{BoundedCurve, ParameterDivision1D};
use truck_meshalgo::prelude::PolylineCurve;
use truck_topology::{Edge, EdgeID, Face, Shell, Solid};

use super::classify::{classify_fragments, FragmentClassification};
use super::split::{
    create_parameter_boundary, split_fragments, CoincidentOrientation, ContactEvent, FragmentMesh,
    FragmentOrigin, SolidRef, StratumRef,
};
use super::BoolOp;
use super::{fragment_decision, FragmentDecision, MaterialState4};

/// The insertion tolerance class (length), shared with the splitter and the
/// classifier. Tightening it is future work, never a test's lever.
const INSERTION_TOL: f64 = 1.0e-2; // H-3: the insertion tolerance class (length)

/// The regularized Boolean of two single-shell solids (plan §4 Phase 4).
///
/// Composes the lift, the AABB-screened sweep, the splitter, the classifier,
/// and the decision + sewing; `Solid::try_new` is the acceptance gate. A
/// refusal is always a typed [`Refusal`], never a panic.
pub fn boolean(
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    // GUARDS (decision 3, step 0): the v1 envelope accepts only single-shell
    // inputs; multi-shell is the RW-MULTISHELL fold.
    if a.boundaries().len() != 1 || b.boundaries().len() != 1 {
        return Err(unsupported());
    }
    let shell_a = a.boundaries().first().ok_or_else(unsupported)?;
    let shell_b = b.boundaries().first().ok_or_else(unsupported)?;

    // SWEEP (decision 3, step 2): the certified contact events over every
    // cross-solid stratum pair, AABB-screened.
    let events = sweep_contact_events(a, b, INSERTION_TOL)?.value;
    // SPLIT (step 3).
    let mesh = split_fragments(shell_a, shell_b, &events, INSERTION_TOL)?.value;
    // CLASSIFY (step 4).
    let classification = classify_fragments(shell_a, shell_b, &mesh, INSERTION_TOL)?.value;
    // DECIDE + ASSEMBLE (step 5, decision 4).
    let faces = decide_and_assemble(op, &mesh, &classification)?;

    let cert = Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    if faces.is_empty() {
        // All-discarded: the op's result is the empty solid (A − A = ∅, the
        // zero-shell solid).
        let solid = Solid::try_new(Vec::new()).map_err(|_| unsupported())?;
        return Ok(Certified::new(solid, cert));
    }
    let shell: Shell<Point3, Curve, Surface> = faces.into();
    if shell.connected_components().len() != 1 {
        // A multi-component kept shell is the multi-component fold.
        return Err(unsupported());
    }
    let solid = Solid::try_new(vec![shell]).map_err(|_| unsupported())?;
    Ok(Certified::new(solid, cert))
}

/// The AABB-screened cross-solid contact sweep over the two solids' lifted
/// strata (decision 3, step 2; unit-testable). The sweep runs `contact()` on
/// a fresh budget: every flagship event takes the no-budget
/// exact/identity/FE arms.
pub(crate) fn sweep_contact_events(
    a: &Solid<Point3, Curve, Surface>,
    b: &Solid<Point3, Curve, Surface>,
    tol: f64,
) -> Outcome<Vec<ContactEvent>> {
    let mut budget = Budget::new(0, 0, 0);
    let shell_a = a.boundaries().first().ok_or_else(unsupported)?;
    let shell_b = b.boundaries().first().ok_or_else(unsupported)?;
    let faces_a = lift_faces(SolidRef::A, shell_a, tol)?;
    let faces_b = lift_faces(SolidRef::B, shell_b, tol)?;
    let edges_a = lift_edges(SolidRef::A, shell_a, tol)?;
    let edges_b = lift_edges(SolidRef::B, shell_b, tol)?;
    let mut events: Vec<ContactEvent> = Vec::new();

    // FF: a-face x b-face.
    for fa in &faces_a {
        for fb in &faces_b {
            if fa.aabb.touches(&fb.aabb) {
                emit_contact(
                    &fa.stratum,
                    &fb.stratum,
                    fa.provenance,
                    fb.provenance,
                    &mut budget,
                    &mut events,
                )?;
            }
        }
    }
    // FE: a-face x b-edge, then a-edge x b-face (the splitter's `collect_sew`
    // normalizes the `(Face, Edge)` order either way).
    for fa in &faces_a {
        for eb in &edges_b {
            if fa.aabb.touches(&eb.aabb) {
                emit_contact(
                    &fa.stratum,
                    &eb.stratum,
                    fa.provenance,
                    eb.provenance,
                    &mut budget,
                    &mut events,
                )?;
            }
        }
    }
    for ea in &edges_a {
        for fb in &faces_b {
            if ea.aabb.touches(&fb.aabb) {
                emit_contact(
                    &ea.stratum,
                    &fb.stratum,
                    ea.provenance,
                    fb.provenance,
                    &mut budget,
                    &mut events,
                )?;
            }
        }
    }
    // EE: a-edge x b-edge.
    for ea in &edges_a {
        for eb in &edges_b {
            if ea.aabb.touches(&eb.aabb) {
                emit_contact(
                    &ea.stratum,
                    &eb.stratum,
                    ea.provenance,
                    eb.provenance,
                    &mut budget,
                    &mut events,
                )?;
            }
        }
    }

    let cert = Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    Ok(Certified::new(events, cert))
}

/// The 3-D axis-aligned bounding box of one lifted stratum.
struct Aabb {
    /// The componentwise minimum corner.
    lo: Point3,
    /// The componentwise maximum corner.
    hi: Point3,
}

impl Aabb {
    /// The empty box, which grows into any point.
    fn empty() -> Aabb {
        Aabb {
            lo: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            hi: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    /// Grows the box to contain `p`.
    fn grow(&mut self, p: Point3) {
        self.lo.x = self.lo.x.min(p.x);
        self.lo.y = self.lo.y.min(p.y);
        self.lo.z = self.lo.z.min(p.z);
        self.hi.x = self.hi.x.max(p.x);
        self.hi.y = self.hi.y.max(p.y);
        self.hi.z = self.hi.z.max(p.z);
    }

    /// Whether two boxes touch: INCLUSIVE overlap on all three axes (boundary
    /// touch counts — the real FF circle sits exactly on the wall's box
    /// boundary).
    fn touches(&self, other: &Aabb) -> bool {
        self.lo.x <= other.hi.x
            && other.lo.x <= self.hi.x
            && self.lo.y <= other.hi.y
            && other.lo.y <= self.hi.y
            && self.lo.z <= other.hi.z
            && other.lo.z <= self.hi.z
    }
}

/// One lifted face stratum with its provenance and 3-D AABB.
struct LiftedFace {
    /// The stratum provenance reference.
    provenance: StratumRef,
    /// The bounded canonical stratum.
    stratum: BoundedStratum,
    /// The 3-D AABB of the face's boundary curves.
    aabb: Aabb,
}

/// One lifted edge stratum with its provenance and 3-D AABB.
struct LiftedEdge {
    /// The stratum provenance reference.
    provenance: StratumRef,
    /// The bounded canonical stratum.
    stratum: BoundedStratum,
    /// The 3-D AABB of the edge curve.
    aabb: Aabb,
}

/// Runs `contact()` on one screened stratum pair and turns every record of an
/// `Ok` complex into a [`ContactEvent`] with the pair's provenance. A refusal
/// propagates as-is.
fn emit_contact(
    lhs: &BoundedStratum,
    rhs: &BoundedStratum,
    lhs_ref: StratumRef,
    rhs_ref: StratumRef,
    budget: &mut Budget,
    events: &mut Vec<ContactEvent>,
) -> Result<(), Refusal> {
    let out = contact(lhs, rhs, budget)?;
    for record in out.value.contacts {
        events.push(ContactEvent {
            record,
            lhs: lhs_ref,
            rhs: rhs_ref,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// the lift
// ---------------------------------------------------------------------------

/// The `(u, v)` box of a face: the min/max over the parameter polygons of its
/// boundary wires (the `create_parameter_boundary` hull), in the stored
/// frame.
fn face_uv_box(face: &Face<Point3, Curve, Surface>, tol: f64) -> Option<((f64, f64), (f64, f64))> {
    let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    let mut u_lo = f64::INFINITY;
    let mut u_hi = f64::NEG_INFINITY;
    let mut v_lo = f64::INFINITY;
    let mut v_hi = f64::NEG_INFINITY;
    for wire in face.absolute_boundaries() {
        let poly = create_parameter_boundary(face, wire, &mut cache, tol)?;
        for p in poly.iter() {
            u_lo = u_lo.min(p.x);
            u_hi = u_hi.max(p.x);
            v_lo = v_lo.min(p.y);
            v_hi = v_hi.max(p.y);
        }
    }
    Some(((u_lo, u_hi), (v_lo, v_hi)))
}

/// The parameter-division sample points of a curve (its 3-D polyline).
fn curve_samples(curve: &Curve, tol: f64) -> Vec<Point3> {
    curve.parameter_division(curve.range_tuple(), tol).1
}

/// The 3-D AABB of a face: min/max over the sample points of its boundary
/// curves (the trimmed region's closure lies inside it).
fn face_aabb(face: &Face<Point3, Curve, Surface>, tol: f64) -> Aabb {
    let mut aabb = Aabb::empty();
    for edge in face.edge_iter() {
        for p in curve_samples(&edge.curve(), tol) {
            aabb.grow(p);
        }
    }
    aabb
}

/// The 3-D AABB of an edge: min/max over its curve's sample points.
fn edge_aabb(edge: &Edge<Point3, Curve>, tol: f64) -> Aabb {
    let mut aabb = Aabb::empty();
    for p in curve_samples(&edge.curve(), tol) {
        aabb.grow(p);
    }
    aabb
}

/// Lifts every face of a shell to a bounded canonical stratum, refusing a
/// non-canonical carrier at the lift boundary (before `contact()` is ever
/// reached).
fn lift_faces(
    solid: SolidRef,
    shell: &Shell<Point3, Curve, Surface>,
    tol: f64,
) -> Result<Vec<LiftedFace>, Refusal> {
    let mut out = Vec::new();
    for (fi, face) in shell.face_iter().enumerate() {
        let witness = recognize_surface(&face.surface());
        if matches!(witness, CanonicalCarrierWitness::Unrecognized) {
            return Err(non_canonical());
        }
        let Some((u_range, v_range)) = face_uv_box(face, tol) else {
            return Err(numerically_unresolved());
        };
        let stratum = face_stratum(witness, u_range, v_range).map_err(|_| non_canonical())?;
        let aabb = face_aabb(face, tol);
        out.push(LiftedFace {
            provenance: StratumRef::Face { solid, index: fi },
            stratum,
            aabb,
        });
    }
    Ok(out)
}

/// Lifts every edge of a shell at its FIRST occurrence by `EdgeID` across
/// `face_iter()` order, with `StratumRef::Edge` provenance at its flat
/// position in that face's `absolute_boundaries()`.
fn lift_edges(
    solid: SolidRef,
    shell: &Shell<Point3, Curve, Surface>,
    tol: f64,
) -> Result<Vec<LiftedEdge>, Refusal> {
    let mut out = Vec::new();
    let mut seen: HashSet<EdgeID<Curve>> = HashSet::default();
    for (fi, face) in shell.face_iter().enumerate() {
        let mut flat = 0usize;
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                if !seen.insert(edge.id()) {
                    flat += 1;
                    continue;
                }
                let curve = match recognize_curve(&edge.curve()) {
                    CanonicalCarrierWitness::ExactCanonical { carrier, .. }
                    | CanonicalCarrierWitness::Derived { carrier, .. } => match carrier {
                        CanonicalCarrier::Curve(curve) => curve,
                        CanonicalCarrier::Surface(_) => return Err(non_canonical()),
                    },
                    CanonicalCarrierWitness::Unrecognized => return Err(non_canonical()),
                };
                let t_range = edge.curve().range_tuple();
                let stratum = BoundedStratum::Edge { curve, t_range };
                let aabb = edge_aabb(edge, tol);
                out.push(LiftedEdge {
                    provenance: StratumRef::Edge {
                        solid,
                        face: fi,
                        edge: flat,
                    },
                    stratum,
                    aabb,
                });
                flat += 1;
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// decision + assembly
// ---------------------------------------------------------------------------

/// The §13.1 decision and the sewing of the kept fragments (decision 4).
///
/// Coincident pairs are resolved ONCE: their verdicts must agree, their flips
/// must match their orientation, and the pair's `a` fragment is emitted (with
/// its flip applied). Non-pair fragments are kept iff
/// [`fragment_decision`] says `Keep`.
fn decide_and_assemble(
    op: BoolOp,
    mesh: &FragmentMesh,
    classification: &FragmentClassification,
) -> Result<Vec<Face<Point3, Curve, Surface>>, Refusal> {
    let n = mesh.fragments.len();
    // A fragment in two coincident pairs is the pair-dedup fold.
    let mut pair_of: Vec<Option<usize>> = vec![None; n];
    for (pi, pair) in mesh.coincident.iter().enumerate() {
        if pair_of.get(pair.a).is_some_and(Option::is_some)
            || pair_of.get(pair.b).is_some_and(Option::is_some)
        {
            return Err(unsupported());
        }
        if let Some(slot) = pair_of.get_mut(pair.a) {
            *slot = Some(pi);
        }
        if let Some(slot) = pair_of.get_mut(pair.b) {
            *slot = Some(pi);
        }
    }

    let mut handled: Vec<bool> = vec![false; n];
    let mut kept: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    for pair in &mesh.coincident {
        let a_origin = mesh.fragments.get(pair.a).ok_or_else(unsupported)?.origin;
        let b_origin = mesh.fragments.get(pair.b).ok_or_else(unsupported)?.origin;
        let a_bit = classification
            .inside_other
            .get(pair.a)
            .copied()
            .unwrap_or(false);
        let b_bit = classification
            .inside_other
            .get(pair.b)
            .copied()
            .unwrap_or(false);
        let da = fragment_decision(op, fragment_state(a_origin, a_bit, Some(pair.orientation)));
        let db = fragment_decision(op, fragment_state(b_origin, b_bit, Some(pair.orientation)));
        match (da, db) {
            (FragmentDecision::Discard, FragmentDecision::Discard) => {}
            (FragmentDecision::Keep { flip: fa }, FragmentDecision::Keep { flip: fb }) => {
                let flips_ok = match pair.orientation {
                    CoincidentOrientation::Identical => fa == fb,
                    CoincidentOrientation::Anti => fa != fb,
                };
                if !flips_ok {
                    // The pair's flips contradict its orientation (the
                    // orientation-consistency fold).
                    return Err(unsupported());
                }
                let mut face = mesh
                    .fragments
                    .get(pair.a)
                    .ok_or_else(unsupported)?
                    .face
                    .clone();
                if fa {
                    face.invert();
                }
                kept.push(face);
            }
            _ => {
                // The pair's verdicts disagree (the pair-consistency fold).
                return Err(unsupported());
            }
        }
        if let Some(slot) = handled.get_mut(pair.a) {
            *slot = true;
        }
        if let Some(slot) = handled.get_mut(pair.b) {
            *slot = true;
        }
    }

    for i in 0..n {
        if handled.get(i).copied() == Some(true) {
            continue;
        }
        let fragment = mesh.fragments.get(i).ok_or_else(unsupported)?;
        let bit = classification.inside_other.get(i).copied().unwrap_or(false);
        let decision = fragment_decision(op, fragment_state(fragment.origin, bit, None));
        if let FragmentDecision::Keep { flip } = decision {
            let mut face = fragment.face.clone();
            if flip {
                face.invert();
            }
            kept.push(face);
        }
    }
    Ok(kept)
}

/// The `MaterialState4` of one fragment (decision 4): own pair `(1, 0)` (the
/// fragment's own solid is on the minus side of its own effective normal),
/// other pair `(s, s)` from the classification — EXCEPT a coincident pair,
/// whose orientation-derived other pair takes precedence (`(1, 0)` Identical,
/// `(0, 1)` Anti).
fn fragment_state(
    origin: FragmentOrigin,
    s: bool,
    orientation: Option<CoincidentOrientation>,
) -> MaterialState4 {
    let other = match orientation {
        Some(CoincidentOrientation::Identical) => (true, false),
        Some(CoincidentOrientation::Anti) => (false, true),
        None => (s, s),
    };
    match origin {
        FragmentOrigin::A { .. } => MaterialState4 {
            a_minus: true,
            a_plus: false,
            b_minus: other.0,
            b_plus: other.1,
        },
        FragmentOrigin::B { .. } => MaterialState4 {
            a_minus: other.0,
            a_plus: other.1,
            b_minus: true,
            b_plus: false,
        },
    }
}

// ---------------------------------------------------------------------------
// refusal helpers
// ---------------------------------------------------------------------------

/// The deferred-envelope refusal (the v1 envelope's boundary).
fn unsupported() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

/// The non-canonical-carrier refusal at the lift boundary.
fn non_canonical() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

/// The numerically-unresolved refusal for a failed parameter projection.
fn numerically_unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built dyadic witnesses are
// not such a path; the unwraps and indexing below cannot fire for the values
// constructed.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;
    use truck_base::cgmath64::{Matrix4, Point2, Vector4};
    use truck_base::contact::{ContactDimension, ContactEventKind};
    use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
    use truck_evidence::contact::ContactLocus;
    use truck_geometry::arrange::{arrange, Arrangement};
    use truck_geometry::prelude::*;
    use truck_modeling::extrude::extrude_profile;

    /// The insertion tolerance class for the sweep/split/classify calls (H-3:
    /// dimensionless relative to the unit-scale witnesses; dyadic geometry
    /// decides exactly).
    const TOL: f64 = 1.0e-2; // H-3: tolerance class for insertion geometry

    /// A placed full-period circle at `center` with radius `r`.
    fn placed_circle(
        center: Point3,
        r: f64,
    ) -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
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

    /// The `[x0, x1] x [y0, y1]` axis-aligned box profile, CCW.
    fn box_profile(x0: f64, y0: f64, x1: f64, y1: f64) -> (Vec<Curve>, Arrangement) {
        let profile = vec![
            Curve::Line(Line(Point3::new(x0, y0, 0.0), Point3::new(x1, y0, 0.0))),
            Curve::Line(Line(Point3::new(x1, y0, 0.0), Point3::new(x1, y1, 0.0))),
            Curve::Line(Line(Point3::new(x1, y1, 0.0), Point3::new(x0, y1, 0.0))),
            Curve::Line(Line(Point3::new(x0, y1, 0.0), Point3::new(x0, y0, 0.0))),
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
    fn extrude_shell(
        profile: &[Curve],
        arr: &Arrangement,
        height: f64,
    ) -> Shell<Point3, Curve, Surface> {
        let solid = extrude_profile(profile, arr, height).unwrap().value;
        solid.boundaries().first().unwrap().clone()
    }

    /// The index of the orientation-true `Plane` face whose corner sits at z.
    fn plane_face_at_z(shell: &Shell<Point3, Curve, Surface>, z: f64) -> usize {
        shell
            .face_iter()
            .enumerate()
            .find(|(_, face)| {
                matches!(face.surface(), Surface::Plane(_))
                    && (face.surface().subs(0.0, 0.0).z - z).abs() < TOL
            })
            .map(|(i, _)| i)
            .unwrap()
    }

    /// The index of the `Cylinder` face.
    fn cylinder_face(shell: &Shell<Point3, Curve, Surface>) -> usize {
        shell
            .face_iter()
            .enumerate()
            .find(|(_, face)| matches!(face.surface(), Surface::Cylinder(_)))
            .map(|(i, _)| i)
            .unwrap()
    }

    /// The per-wire edge counts of a face's absolute boundary wires.
    fn wire_counts(face: &Face<Point3, Curve, Surface>) -> Vec<usize> {
        face.absolute_boundaries().iter().map(|w| w.len()).collect()
    }

    // ---------------------------------------------------------------------------
    // Test 1: the sweep produces the flagship's six events (decision 5).
    // ---------------------------------------------------------------------------

    #[test]
    fn boolean_sweep_produces_the_flagship_event_complex() {
        // a = the 4x4 block extrude (faces: 0 = bottom z=0 inverted, 1 = top
        // z=2, 2..5 = sides); b = the disk extrude at (2, 2) r=1 (faces:
        // 0 = bottom cap inverted, 1 = top cap, 2 = the wall).
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);
        let solid_a = Solid::try_new(vec![shell_a.clone()]).unwrap();
        let solid_b = Solid::try_new(vec![shell_b.clone()]).unwrap();

        let events = sweep_contact_events(&solid_a, &solid_b, TOL).unwrap().value;
        assert_eq!(events.len(), 6);

        // Decision 5's table: 2 Region2 `Coincident`, 2 FF Transverse circles,
        // 2 FE CoincidentInterval BoundedCurves (full-period).
        let region2: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.record.locus, ContactLocus::Coincident))
            .collect();
        let ff: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    &e.record.locus,
                    ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(_)))
                )
            })
            .collect();
        let fe: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.record.locus, ContactLocus::BoundedCurve { .. }))
            .collect();
        assert_eq!(region2.len(), 2);
        assert_eq!(ff.len(), 2);
        assert_eq!(fe.len(), 2);

        let a_bottom = plane_face_at_z(&shell_a, 0.0);
        let a_top = plane_face_at_z(&shell_a, 2.0);
        let b_bottom = plane_face_at_z(&shell_b, 0.0);
        let b_top = plane_face_at_z(&shell_b, 2.0);
        let wall_b = cylinder_face(&shell_b);

        // Region2 Coincident: the identity arm on a's plane at z and b's cap
        // at the same z, for z in {0, 2}.
        for e in &region2 {
            assert_eq!(e.record.dimension, ContactDimension::Region2);
            assert_eq!(e.record.kind, ContactEventKind::IdenticalCarrier);
            let (
                StratumRef::Face {
                    solid: sa,
                    index: fa,
                },
                StratumRef::Face {
                    solid: sb,
                    index: fb,
                },
            ) = (e.lhs, e.rhs)
            else {
                unreachable!("the Region2 Coincident event pairs two faces");
            };
            assert_eq!(sa, SolidRef::A);
            assert_eq!(sb, SolidRef::B);
            assert!(
                (fa == a_bottom && fb == b_bottom) || (fa == a_top && fb == b_top),
                "the coincident pair is a's plane at z with b's cap at the same z, got ({fa}, {fb})"
            );
        }

        // FF Transverse: the wall x plane circle on a's face at z and b's wall,
        // circles at (2, 2, 0) and (2, 2, 2), radius 1.
        let mut circle_zs: Vec<f64> = Vec::new();
        for e in &ff {
            assert_eq!(e.record.dimension, ContactDimension::Arc1);
            assert_eq!(e.record.kind, ContactEventKind::Transverse);
            let (
                StratumRef::Face {
                    solid: sa,
                    index: fa,
                },
                StratumRef::Face {
                    solid: sb,
                    index: fb,
                },
            ) = (e.lhs, e.rhs)
            else {
                unreachable!("the FF circle event pairs two faces");
            };
            assert_eq!(sa, SolidRef::A);
            assert_eq!(sb, SolidRef::B);
            assert!(
                fa == a_bottom || fa == a_top,
                "the a-side is a horizontal face"
            );
            assert_eq!(fb, wall_b);
            let ContactLocus::Analytic(AnalyticIntersection::Curve(ExactCurve::Circle(c))) =
                &e.record.locus
            else {
                unreachable!("the FF event is an exact circle");
            };
            let t = c.transform();
            let center = Point3::new(t.w.x, t.w.y, t.w.z);
            assert!((center.x - 2.0).abs() < TOL, "center x = {}", center.x);
            assert!((center.y - 2.0).abs() < TOL, "center y = {}", center.y);
            circle_zs.push(center.z);
            let radius = Vector3::new(t.x.x, t.x.y, t.x.z).magnitude();
            assert!((radius - 1.0).abs() < TOL, "radius = {radius}");
        }
        circle_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(circle_zs, vec![0.0, 2.0]);

        // FE CoincidentInterval: a's plane at z x b's rim edge (the cap that
        // carries the rim first in b's `face_iter()` order — b's caps), with
        // the full-period BoundedCurve.
        for e in &fe {
            assert_eq!(e.record.dimension, ContactDimension::Arc1);
            assert_eq!(e.record.kind, ContactEventKind::CoincidentInterval);
            let (
                StratumRef::Face {
                    solid: sa,
                    index: fa,
                },
                StratumRef::Edge {
                    solid: sb,
                    face: fb,
                    edge: fe_idx,
                },
            ) = (e.lhs, e.rhs)
            else {
                unreachable!("the FE event pairs an a-face with a b-edge");
            };
            assert_eq!(sa, SolidRef::A);
            assert_eq!(sb, SolidRef::B);
            assert_eq!(fe_idx, 0);
            let ContactLocus::BoundedCurve { curve, t_range } = &e.record.locus else {
                unreachable!("the FE event is a bounded curve");
            };
            assert_eq!(*t_range, (0.0, TAU));
            let ExactCurve::Circle(c) = curve else {
                unreachable!("the FE curve is a circle");
            };
            let center = Point3::new(c.transform().w.x, c.transform().w.y, c.transform().w.z);
            let expected_z = if fa == a_bottom && fb == b_bottom {
                0.0
            } else if fa == a_top && fb == b_top {
                2.0
            } else {
                unreachable!("unexpected FE provenance (a-face {fa}, b-edge face {fb})");
            };
            assert!(
                (center.z - expected_z).abs() < TOL,
                "FE circle z = {}, expected {expected_z}",
                center.z
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 2: Difference assembles the plate with hole (decision 4's measured
    // set).
    // ---------------------------------------------------------------------------

    #[test]
    fn boolean_difference_flagship_assembles_the_plate_with_hole() {
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);
        let solid_a = Solid::try_new(vec![shell_a]).unwrap();
        let solid_b = Solid::try_new(vec![shell_b]).unwrap();
        let mut budget = Budget::new(0, 0, 0);

        let result = boolean(&solid_a, BoolOp::Difference, &solid_b, &mut budget)
            .expect("the Difference flagship assembles");
        let solid = result.value;
        assert_eq!(solid.boundaries().len(), 1);
        let shell = solid.boundaries().first().unwrap();

        // Decision 4's measured set: 7 faces — two `[4, 2]`-wire annuli (the
        // plate at z=0 and z=2), four `[4]`-wire sides, one `[2, 2]`-wire hole
        // wall.
        assert_eq!(shell.face_iter().count(), 7);
        let mut annuli = 0usize;
        let mut annulus_zs: Vec<f64> = Vec::new();
        let mut sides = 0usize;
        let mut wall = None;
        for face in shell.face_iter() {
            let counts = wire_counts(face);
            match face.surface() {
                Surface::Plane(_) => match counts.as_slice() {
                    [4, 2] => {
                        annuli += 1;
                        annulus_zs.push(face.surface().subs(0.0, 0.0).z);
                    }
                    [4] => sides += 1,
                    other => unreachable!("unexpected plane wire structure {other:?}"),
                },
                Surface::Cylinder(_) => {
                    assert_eq!(counts, vec![2, 2], "the hole wall is a two-wire annulus");
                    wall = Some(face);
                }
                other => unreachable!("unexpected Difference result face {other:?}"),
            }
        }
        assert_eq!(annuli, 2);
        annulus_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(annulus_zs, vec![0.0, 2.0]);
        assert_eq!(sides, 4);
        let wall = wall.expect("a hole wall face");

        // The wall's EFFECTIVE normal points TOWARD the axis: sample the
        // effective normal (`surface.normal` negated iff `!face.orientation()`)
        // at the dyadic point (u=0, v=1) -> (3, 2, 1) and dot it with the
        // outward radial direction there. `Solid::try_new` already validated
        // the shell; this is the geometric sign check on the kept-flipped wall.
        let Surface::Cylinder(cyl) = wall.surface() else {
            unreachable!("the wall is a cylinder");
        };
        let normal = wall.surface().normal(0.0, 1.0);
        let effective = if wall.orientation() { normal } else { -normal };
        let p = wall.surface().subs(0.0, 1.0);
        let radial = Vector3::new(p.x - cyl.center().x, p.y - cyl.center().y, 0.0).normalize();
        assert!(
            effective.dot(radial) < 0.0,
            "the flipped wall's effective normal must point toward the axis"
        );
    }

    // ---------------------------------------------------------------------------
    // Test 3: Union / Intersection / Xor on the flagship (decision 4's
    // measured face counts and the Intersection identification).
    // ---------------------------------------------------------------------------

    #[test]
    fn boolean_union_intersection_xor_on_the_flagship() {
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);
        let solid_a = Solid::try_new(vec![shell_a]).unwrap();
        let solid_b = Solid::try_new(vec![shell_b]).unwrap();

        let mut union_budget = Budget::new(0, 0, 0);
        let union = boolean(&solid_a, BoolOp::Union, &solid_b, &mut union_budget)
            .expect("the Union flagship assembles")
            .value;
        let mut inter_budget = Budget::new(0, 0, 0);
        let intersection = boolean(&solid_a, BoolOp::Intersection, &solid_b, &mut inter_budget)
            .expect("the Intersection flagship assembles")
            .value;
        let mut xor_budget = Budget::new(0, 0, 0);
        let xor = boolean(&solid_a, BoolOp::Xor, &solid_b, &mut xor_budget)
            .expect("the Xor flagship assembles")
            .value;

        // Decision 4's measured face counts: Union 8 (the block, cosmetically
        // split), Intersection 3 (the cylinder), Xor 7 (= the Difference set).
        assert_eq!(union.boundaries().len(), 1);
        assert_eq!(union.boundaries().first().unwrap().face_iter().count(), 8);
        assert_eq!(intersection.boundaries().len(), 1);
        assert_eq!(
            intersection
                .boundaries()
                .first()
                .unwrap()
                .face_iter()
                .count(),
            3
        );
        assert_eq!(xor.boundaries().len(), 1);
        assert_eq!(xor.boundaries().first().unwrap().face_iter().count(), 7);

        // Intersection identifies the cylinder: the two deduped `[2]`-wire
        // disks (at z=0 and z=2) and the `[2, 2]`-wire wall, UNFLIPPED.
        let inter_shell = intersection.boundaries().first().unwrap();
        let mut disks: Vec<&Face<Point3, Curve, Surface>> = Vec::new();
        let mut wall = None;
        for face in inter_shell.face_iter() {
            let counts = wire_counts(face);
            match face.surface() {
                Surface::Plane(_) => {
                    assert_eq!(counts, vec![2], "an Intersection plane is a [2]-wire disk");
                    disks.push(face);
                }
                Surface::Cylinder(_) => {
                    assert_eq!(counts, vec![2, 2], "the wall is a two-wire annulus");
                    wall = Some(face);
                }
                other => unreachable!("unexpected Intersection result face {other:?}"),
            }
        }
        assert_eq!(disks.len(), 2);
        let mut disk_zs: Vec<f64> = disks.iter().map(|f| f.surface().subs(0.0, 0.0).z).collect();
        disk_zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(disk_zs, vec![0.0, 2.0]);
        let wall = wall.expect("the Intersection wall");

        // The wall is UNFLIPPED: its effective normal points OUTWARD from the
        // axis (dot with the outward radial direction is positive).
        let Surface::Cylinder(cyl) = wall.surface() else {
            unreachable!("the wall is a cylinder");
        };
        let normal = wall.surface().normal(0.0, 1.0);
        let effective = if wall.orientation() { normal } else { -normal };
        let p = wall.surface().subs(0.0, 1.0);
        let radial = Vector3::new(p.x - cyl.center().x, p.y - cyl.center().y, 0.0).normalize();
        assert!(
            effective.dot(radial) > 0.0,
            "the Intersection wall keeps the outward normal"
        );
    }

    // ---------------------------------------------------------------------------
    // Test 4: multi-shell input refuses at the guard.
    // ---------------------------------------------------------------------------

    #[test]
    fn boolean_refuses_multishell_input() {
        // Two disjoint 2x2 block extrudes, far apart, as one two-shell solid.
        let (pa, aa) = box_profile(0.0, 0.0, 2.0, 2.0);
        let s1 = extrude_shell(&pa, &aa, 2.0);
        let (pb, ab) = box_profile(10.0, 10.0, 12.0, 12.0);
        let s2 = extrude_shell(&pb, &ab, 2.0);
        let multi = Solid::try_new(vec![s1, s2]).unwrap();

        let (pc, ac) = block_profile();
        let block = Solid::try_new(vec![extrude_shell(&pc, &ac, 2.0)]).unwrap();

        let mut budget = Budget::new(0, 0, 0);
        let out = boolean(&multi, BoolOp::Union, &block, &mut budget);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "multi-shell input must refuse at the guard, before any sweep work"
        );
    }
}
