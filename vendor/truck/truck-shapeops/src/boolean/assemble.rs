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

// CFP-009-BROADPHASE: the uniform-grid broadphase pair screen lives in its
// own module file, `boolean/broadphase.rs` (the packet's write_allow pins
// that path). It is registered here as a child of the assembler rather than
// in `boolean/mod.rs` so this packet's diff stays inside its write_allow set.
#[path = "broadphase.rs"]
mod broadphase;

use rustc_hash::FxHashSet as HashSet;
use truck_base::cgmath64::{InnerSpace, Point2, Point3};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};
use truck_evidence::analytic::AnalyticIntersection;
use truck_evidence::contact::{contact, face_stratum, BoundedStratum, ContactLocus};
use truck_evidence::enclosure::{Box3, EnclosureCurve, EnclosureSurface, Interval};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::recognize::{
    recognize_curve, recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalCurve,
};
use truck_geometry::specifieds::UnitCircle;
use truck_geotrait::{BoundedCurve, ParameterDivision1D, ParametricSurface, SearchParameter};
use truck_topology::{Edge, EdgeID, EntityId, Face, Shell, Solid};

use self::broadphase::{candidate_touching_pairs, HasBox};
use super::classify::{classify_fragments, FragmentClassification};
use super::split::{
    split_fragments, CoincidentOrientation, ContactEvent, FragmentMesh, FragmentOrigin, SolidRef,
    StratumRef,
};
use super::sweep_lift::{
    classify_from_cells, fragment_provenance, sweep_face_stratum, windowed_sweep_face,
    FragmentProvenance,
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
    // CLASSIFY (step 4). A sweep-carrier shell is classified through the
    // BIE-006 arrangement-cell seeds ([`classify_from_cells`]); the canonical
    // path rides the landed seed-and-propagate classifier unchanged (V5).
    let has_sweep = shell_a
        .face_iter()
        .any(|face| matches!(face.surface(), Surface::SpineFrameSurface(_)))
        || shell_b
            .face_iter()
            .any(|face| matches!(face.surface(), Surface::SpineFrameSurface(_)));
    let classification = if has_sweep {
        classify_from_cells(shell_a, shell_b, &mesh, INSERTION_TOL)?.value
    } else {
        classify_fragments(shell_a, shell_b, &mesh, INSERTION_TOL)?.value
    };
    // DECIDE + ASSEMBLE (step 5, decision 4). The provenance rows of the kept
    // fragments are recorded ([`FragmentProvenance`]); the sealed output
    // carries the solid, the rows are the next integration seam.
    let mut provenance_rows: Vec<FragmentProvenance> = Vec::new();
    let faces = decide_and_assemble(op, &mesh, &classification, &mut provenance_rows)?;

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

/// The self-pair entry gate (T2 §5.8 arm 1): a Boolean whose two operands are
/// CERTIFIED to be the same construction node rewrites before the sweep.
///
/// `a_id`/`b_id` are the operands' certified [`EntityId`]s (construction-node
/// content identity, `truck-topology`). When the two ids are equal the
/// self-pair identities hold — `A ∪ A = A`, `A ∩ A = A`, `A − A = ∅`,
/// `A △ A = ∅` — and the operation is answered by a rewrite that never enters
/// the sweep: there is no SSI to certify, and nothing to prove about the
/// intra-solid adjacency event classes. The certificate budget is untouched.
///
/// When the ids differ the call falls back to [`boolean`], where an identity
/// that is NOT certifiable at this boundary keeps its typed refusal (the
/// §5.8 arm-2 route through the common-carrier contact calculus is the
/// certified layer's, and stays out of this shapeops boundary).
pub fn boolean_certified_operands(
    a_id: &EntityId,
    a: &Solid<Point3, Curve, Surface>,
    op: BoolOp,
    b_id: &EntityId,
    b: &Solid<Point3, Curve, Surface>,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let cert = Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    if a_id == b_id {
        // Certified-identical operands (§5.8): the idempotent and the
        // complement-self rewrite, never the sweep.
        let result = match op {
            BoolOp::Union | BoolOp::Intersection => a.clone(),
            BoolOp::Difference | BoolOp::Xor => {
                let empty = Solid::try_new(Vec::new()).map_err(|_| unsupported())?;
                return Ok(Certified::new(empty, cert));
            }
        };
        return Ok(Certified::new(result, cert));
    }
    boolean(a, op, b, budget)
}

/// The AABB-screened cross-solid contact sweep over the two solids' lifted
/// strata (decision 3, step 2; unit-testable). The sweep runs `contact()` on
/// a fresh budget: every flagship event takes the no-budget
/// exact/identity/FE arms. The screen is the uniform-grid broadphase
/// (CFP-009): the indexed pair set is exactly the flat loop's pair set,
/// emitted in the flat loop's canonical order.
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

    // The pair screen (CFP-009): the uniform-grid broadphase replaces the flat
    // O(n·m) `touches()` double loops. It returns EXACTLY the touching pair
    // set the flat loops admitted (same predicate, same boxes) and emits it in
    // the flat loops' canonical order — `(index_a, index_b)` ascending per
    // screen. The edge special cases move WITH the pairs: the FF/FE/EF/EE arm
    // routing and the `ee_circle_circle` skip still live at these call sites,
    // applied to the indexed candidates exactly as the flat loop applied them.
    // FF: a-face x b-face.
    for (ia, ib) in candidate_touching_pairs(&faces_a, &faces_b) {
        let fa = faces_a.get(ia).ok_or_else(unsupported)?;
        let fb = faces_b.get(ib).ok_or_else(unsupported)?;
        emit_contact(
            &fa.stratum,
            &fb.stratum,
            fa.provenance,
            fb.provenance,
            &mut budget,
            &mut events,
        )?;
    }
    // FE: a-face x b-edge (the splitter's `collect_sew` normalizes the
    // `(Face, Edge)` order either way).
    for (ia, ib) in candidate_touching_pairs(&faces_a, &edges_b) {
        let fa = faces_a.get(ia).ok_or_else(unsupported)?;
        let eb = edges_b.get(ib).ok_or_else(unsupported)?;
        emit_contact(
            &fa.stratum,
            &eb.stratum,
            fa.provenance,
            eb.provenance,
            &mut budget,
            &mut events,
        )?;
    }
    // EF: a-edge x b-face.
    for (ia, ib) in candidate_touching_pairs(&edges_a, &faces_b) {
        let ea = edges_a.get(ia).ok_or_else(unsupported)?;
        let fb = faces_b.get(ib).ok_or_else(unsupported)?;
        emit_contact(
            &ea.stratum,
            &fb.stratum,
            ea.provenance,
            fb.provenance,
            &mut budget,
            &mut events,
        )?;
    }
    // EE: a-edge x b-edge (the `ee_circle_circle` skip rides the candidate
    // pairs exactly as the flat loop applied it).
    for (ia, ib) in candidate_touching_pairs(&edges_a, &edges_b) {
        let ea = edges_a.get(ia).ok_or_else(unsupported)?;
        let eb = edges_b.get(ib).ok_or_else(unsupported)?;
        if !ee_circle_circle(&ea.stratum, &eb.stratum) {
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
    #[allow(dead_code)] // retained as the flat-loop predicate the CFP-009 exactness fixture gate compares the index against
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

// CFP-009-BROADPHASE: the lifted strata expose their screen boxes to the
// uniform-grid index through [`HasBox`]; the index never sees the strata
// themselves, only the box corners.
impl HasBox for LiftedFace {
    fn lo(&self) -> Point3 {
        self.aabb.lo
    }
    fn hi(&self) -> Point3 {
        self.aabb.hi
    }
}

impl HasBox for LiftedEdge {
    fn lo(&self) -> Point3 {
        self.aabb.lo
    }
    fn hi(&self) -> Point3 {
        self.aabb.hi
    }
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
        // RW-INTERIOR-LOOP recombination: seam records the splitter cannot act
        // on. An `EndpointTouch` point sits at an existing stratum boundary
        // vertex (the seam of a prior Boolean), and an `Arc1 Coincident` from
        // the identity EE arm is a same-edge coincidence the splitter's point
        // and loop machinery already receives through the shared instances.
        // Emitting either would trip a landed refusal on a zero-measure seam
        // touch.
        let seam = matches!(record.kind, ContactEventKind::EndpointTouch)
            || matches!(
                (&record.locus, record.dimension),
                (ContactLocus::Coincident, ContactDimension::Arc1)
                    | (
                        ContactLocus::Analytic(AnalyticIntersection::Coincident),
                        ContactDimension::Arc1
                    )
            );
        if seam {
            continue;
        }
        events.push(ContactEvent {
            record,
            lhs: lhs_ref,
            rhs: rhs_ref,
        });
    }
    Ok(())
}

/// Whether an EE stratum pair is the deferred Circle x Circle cell
/// (RW-INTERIOR-LOOP recombination): the Contact Layer's EE solver has no
/// Circle x Circle arm, and the pair represents a seam/coincident touch of two
/// circle edges that the splitter already receives through the identity and
/// FF/Region2 records. Skipping it keeps the through-cut recombination
/// (`boolean(plus, Union, minus)`) inside the v1 envelope instead of deferring
/// on a zero-measure seam contact.
fn ee_circle_circle(lhs: &BoundedStratum, rhs: &BoundedStratum) -> bool {
    matches!(
        (lhs, rhs),
        (
            BoundedStratum::Edge {
                curve: CanonicalCurve::Circle(_),
                ..
            },
            BoundedStratum::Edge {
                curve: CanonicalCurve::Circle(_),
                ..
            }
        )
    )
}

// ---------------------------------------------------------------------------
// the lift
// ---------------------------------------------------------------------------

/// Unwraps a periodic parameter onto the branch of `previous` (the
/// `sweep_lift` rule: a circle rim crossing the seam keeps its `u` hull
/// continuous instead of snapping back to `[0, 2π)`).
fn unwrap_periodic(period: f64, previous: f64, value: f64) -> f64 {
    let mut delta = value - previous;
    delta -= (delta / period).round() * period;
    previous + delta
}

/// The number of `search_parameter` trials for one trim-extent inversion.
const EXTENT_SEARCH_TRIALS: usize = 100;

/// Whether the carrier's image over its own natural domain can bulge away from
/// its boundary curves (the DSC-BOUNDARY-SAMPLE-EXTENT class): a sphere cap's
/// pole, a torus band, a spline panel's interior. For these, the boundary
/// polygon hull is not a sound cover, so the trim's parameter extent is the
/// carrier's own (clamped) parameter domain.
fn bulging_carrier(surface: &Surface) -> bool {
    match surface {
        Surface::Sphere(_)
        | Surface::Torus(_)
        | Surface::BSplineSurface(_)
        | Surface::NurbsSurface(_) => true,
        Surface::Processor(processor) => bulging_carrier(processor.entity()),
        _ => false,
    }
}

/// The carrier's natural parameter extent, when both axes are finite: the
/// certified superset of any trim of that carrier (a sphere is `[0, π] ×
/// [0, 2π]`, a clamped spline its knot domain). `None` when an axis is
/// unbounded (a `Plane`, `Cylinder`, `Cone` — whose region extremes always lie
/// on the boundary, so the boundary-derived trim box below is the right box).
fn natural_extent(surface: &Surface) -> Option<((f64, f64), (f64, f64))> {
    let (urange, vrange) = surface.parameter_range();
    let u = range_interval(urange)?;
    let v = range_interval(vrange)?;
    if u.0 < u.1 && v.0 < v.1 {
        Some((u, v))
    } else {
        None
    }
}

/// A parameter-axis bound pair into a finite `(lo, hi)` pair; `None` for an
/// unbounded axis.
fn range_interval(range: (std::ops::Bound<f64>, std::ops::Bound<f64>)) -> Option<(f64, f64)> {
    let lo = match range.0 {
        std::ops::Bound::Included(x) | std::ops::Bound::Excluded(x) => x,
        std::ops::Bound::Unbounded => return None,
    };
    let hi = match range.1 {
        std::ops::Bound::Included(x) | std::ops::Bound::Excluded(x) => x,
        std::ops::Bound::Unbounded => return None,
    };
    if lo.is_finite() && hi.is_finite() {
        Some((lo, hi))
    } else {
        None
    }
}

/// The `(u, v)` box of a face — the trim's TRUE parameter extent in the
/// stored frame (CFP-001 decision 3; the `DSC-BOUNDARY-SAMPLE-EXTENT-001`
/// parameter twin). For a curved carrier whose interior can leave the boundary
/// polygon (a cap around a pole, a spline panel), the extent is the carrier's
/// own clamped parameter domain — the certified superset the certified stages
/// and enclosures reason over (F-C1: `param_box_derived_from_trim_not_boundary`).
/// For the classes whose region extremes are always on the boundary (`Plane`,
/// `Cylinder`, `Cone`), the extent is the boundary-derived trim box — exact
/// there, and the DSC record itself grants the boundary rule for planar faces.
fn face_uv_box(face: &Face<Point3, Curve, Surface>, tol: f64) -> Option<((f64, f64), (f64, f64))> {
    let surface = face.surface();
    if bulging_carrier(&surface) {
        if let Some(natural) = natural_extent(&surface) {
            return Some(natural);
        }
    }
    boundary_parameter_extent(&surface, face, tol)
}

/// The exact boundary-derived trim box: the min/max over the `(u, v)` images
/// of the boundary wires' parameter-division samples, in the stored frame
/// (periodic `u` unwrapped onto one branch). This is the splitter's
/// parameter-polygon hull semantics, inlined here so the lift no longer
/// depends on the splitter/classifier's shared parameter-boundary builder (its
/// remaining call sites all live outside the lift path).
fn boundary_parameter_extent(
    surface: &Surface,
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let u_period = surface.u_period();
    let mut u_lo = f64::INFINITY;
    let mut u_hi = f64::NEG_INFINITY;
    let mut v_lo = f64::INFINITY;
    let mut v_hi = f64::NEG_INFINITY;
    let mut any = false;
    for wire in face.absolute_boundaries() {
        let mut previous: Option<Point2> = None;
        let front = wire.front_vertex()?.point();
        let mut p: Point2 = surface
            .search_parameter(front, None, EXTENT_SEARCH_TRIALS)?
            .into();
        u_lo = u_lo.min(p.x);
        u_hi = u_hi.max(p.x);
        v_lo = v_lo.min(p.y);
        v_hi = v_hi.max(p.y);
        any = true;
        for edge in wire.edge_iter() {
            let curve = edge.curve();
            let div = curve.parameter_division(curve.range_tuple(), tol).1;
            for q in div.iter() {
                let mut uv: Point2 = surface
                    .search_parameter(*q, Some(p.into()), EXTENT_SEARCH_TRIALS)?
                    .into();
                if let Some(period) = u_period {
                    if let Some(prev) = previous {
                        uv.x = unwrap_periodic(period, prev.x, uv.x);
                    } else {
                        uv.x = unwrap_periodic(period, p.x, uv.x);
                    }
                }
                previous = Some(uv);
                p = uv;
                u_lo = u_lo.min(p.x);
                u_hi = u_hi.max(p.x);
                v_lo = v_lo.min(p.y);
                v_hi = v_hi.max(p.y);
            }
        }
    }
    if any && u_lo < u_hi && v_lo < v_hi {
        Some(((u_lo, u_hi), (v_lo, v_hi)))
    } else {
        None
    }
}

/// The parameter-division sample points of a curve (its 3-D polyline).
fn curve_samples(curve: &Curve, tol: f64) -> Vec<Point3> {
    curve.parameter_division(curve.range_tuple(), tol).1
}

/// An interval pair from finite `(lo, hi)` bounds; the empty interval on a
/// malformed pair (H-1: no panic).
fn iv(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The outward-rounded image of the box under the affine placement `m`
/// (`world = m · (x, y, z, 1)`; the matrix columns are the affine columns).
fn transform_box3(b: &Box3, m: truck_base::cgmath64::Matrix4) -> Box3 {
    let s = |v: f64| Interval::try_from((v, v)).unwrap_or(Interval::EMPTY);
    let x = s(m.x.x) * b.x + s(m.y.x) * b.y + s(m.z.x) * b.z + s(m.w.x);
    let y = s(m.x.y) * b.x + s(m.y.y) * b.y + s(m.z.y) * b.z + s(m.w.y);
    let z = s(m.x.z) * b.x + s(m.y.z) * b.y + s(m.z.z) * b.z + s(m.w.z);
    Box3 { x, y, z }
}

/// The certified 3-D box of the carrier over the face's parameter extent
/// (CFP-001 decision 3): `face_aabb`'s boundary sampling is replaced by the
/// `EnclosureSurface` boxes — per-carrier exact for the canonical carriers,
/// per-span sub-box hulls for a spline carrier, all unioned over the trim's
/// true extent.
fn certified_surface_box(surface: &Surface, uv: ((f64, f64), (f64, f64))) -> Option<Box3> {
    let (uu, vv) = (iv(uv.0 .0, uv.0 .1), iv(uv.1 .0, uv.1 .1));
    match surface {
        Surface::Plane(s) => Some(s.enclose(uu, vv)),
        Surface::Cylinder(s) => Some(s.enclose(uu, vv)),
        Surface::Cone(s) => Some(s.enclose(uu, vv)),
        Surface::Sphere(s) => Some(s.enclose(uu, vv)),
        Surface::Torus(s) => Some(s.enclose(uu, vv)),
        Surface::BSplineSurface(s) => Some(s.enclose(uu, vv)),
        Surface::NurbsSurface(_) => None,
        Surface::Processor(processor) => {
            let inner = certified_surface_box(processor.entity(), uv)?;
            Some(transform_box3(&inner, *processor.transform()))
        }
        // A sweep face carries its realized window on the whole-sweep closed
        // value; the sweep's certified enclosure over that window is the
        // certified cover of the face.
        Surface::SpineFrameSurface(sweep) => {
            Some(sweep.enclose(iv(sweep.s0(), sweep.s1()), iv(sweep.v0(), sweep.v1())))
        }
        // A derived-of-revolution carrier keeps its boundary-curve cover below.
        Surface::RevolutedCurve(_) | Surface::ExtrudedCurve(_) => None,
    }
}

/// The 3-D AABB of a face: the certified carrier enclosure over the face's
/// true parameter extent (per-span unioned for a spline carrier), grown over
/// the certified enclosures of the boundary curves where the carrier itself
/// has no certified enclosure (the sweep/derived-carrier fallback).
fn face_enclosure(face: &Face<Point3, Curve, Surface>, tol: f64) -> Aabb {
    let mut aabb = Aabb::empty();
    let surface = face.surface();
    if let Some(b) = boundary_enclosure_accumulate(&surface, face, tol) {
        grow_aabb(&mut aabb, &b);
    }
    aabb
}

/// The certified enclosure of one face: the union of the carrier's certified
/// box over the trim extent and the certified boxes of the boundary curves
/// (for carriers whose own box is unavailable or, for full-face canonical
/// classes, always equal to the boundary cover — the cad.rs rule).
fn boundary_enclosure_accumulate(
    surface: &Surface,
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<Box3> {
    let carrier_box = match face_uv_box(face, tol) {
        Some(uv) => certified_surface_box(surface, uv),
        None => None,
    };
    let mut acc: Option<Box3> = carrier_box;
    let mut push = |b: Box3| {
        acc = Some(match acc {
            Some(a) => union_box3(&a, &b),
            None => b,
        });
    };
    for wire in face.absolute_boundaries() {
        for edge in wire.edge_iter() {
            if let Some(b) = certified_edge_box(&edge.curve()) {
                push(b);
            }
        }
    }
    acc
}

/// Grows the AABB to contain the interval box (finite bounds only; a
/// non-finite enclosure contributes nothing — it is not a usable screen bound).
fn grow_aabb(aabb: &mut Aabb, b: &Box3) {
    let (x0, x1) = (b.x.inf(), b.x.sup());
    let (y0, y1) = (b.y.inf(), b.y.sup());
    let (z0, z1) = (b.z.inf(), b.z.sup());
    if x0.is_finite()
        && x1.is_finite()
        && y0.is_finite()
        && y1.is_finite()
        && z0.is_finite()
        && z1.is_finite()
    {
        aabb.grow(Point3::new(x0, y0, z0));
        aabb.grow(Point3::new(x1, y1, z1));
    }
}

/// The outward-rounded union of two interval boxes (coordinate-wise hull).
fn union_box3(a: &Box3, b: &Box3) -> Box3 {
    let union = |x: Interval, y: Interval| -> Interval {
        Interval::try_from((x.inf().min(y.inf()), x.sup().max(y.sup()))).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: union(a.x, b.x),
        y: union(a.y, b.y),
        z: union(a.z, b.z),
    }
}

/// The certified 3-D box of one boundary edge over its own bounded parameter
/// range (`EnclosureCurve` per carrier; the section.rs rule).
fn certified_edge_box(curve: &Curve) -> Option<Box3> {
    match curve {
        Curve::Line(line) => {
            let (t0, t1) = line.range_tuple();
            let tt = iv(t0, t1);
            Some(line.enclose(tt))
        }
        Curve::Circle(placed) => {
            let (t0, t1) = placed.range_tuple();
            let tt = iv(t0, t1);
            let local = UnitCircle::<Point3>::new().enclose(tt);
            Some(transform_box3(&local, *placed.transform()))
        }
        Curve::BSplineCurve(_)
        | Curve::NurbsCurve(_)
        | Curve::IntersectionCurve(_)
        | Curve::SpineFrameCurve(_)
        | Curve::CertifiedImplicitIntersectionCurve(_) => None,
    }
}

/// The 3-D AABB of an edge: min/max over its curve's sample points.
fn edge_aabb(edge: &Edge<Point3, Curve>, tol: f64) -> Aabb {
    let mut aabb = Aabb::empty();
    for p in curve_samples(&edge.curve(), tol) {
        aabb.grow(p);
    }
    aabb
}

/// Lifts every face of a shell to a bounded stratum, refusing a non-canonical
/// carrier at the lift boundary (before `contact()` is ever reached). A sweep
/// face lifts to `BoundedStratum::Sweep` (BIE-006), never the `Unrecognized`
/// refusal.
fn lift_faces(
    solid: SolidRef,
    shell: &Shell<Point3, Curve, Surface>,
    tol: f64,
) -> Result<Vec<LiftedFace>, Refusal> {
    let mut out = Vec::new();
    for (fi, face) in shell.face_iter().enumerate() {
        if let Some(stratum) = sweep_face_stratum(face) {
            // A sweep face: the whole-sweep closed value carries the recipe,
            // the realized window and the placement — no parameter-box
            // projection is needed. The certified enclosure of the sweep over
            // its realized window screens the pair (CFP-001 decision 3).
            let aabb = face_enclosure(face, tol);
            out.push(LiftedFace {
                provenance: StratumRef::Face { solid, index: fi },
                stratum,
                aabb,
            });
            continue;
        }
        let witness = recognize_surface(&face.surface());
        if matches!(witness, CanonicalCarrierWitness::Unrecognized) {
            return Err(non_canonical());
        }
        let Some((u_range, v_range)) = face_uv_box(face, tol) else {
            return Err(numerically_unresolved());
        };
        let stratum = face_stratum(witness, u_range, v_range).map_err(|_| non_canonical())?;
        let aabb = face_enclosure(face, tol);
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
// CC-024 concave-trim provenance (the `StratumRef` convention this module
// landed). The arrangement engine's concave trim (`concave_trim` in
// truck-geometry) emits surviving cells and trim curves tagged by source face
// section (0/1 over the two adjacent source edges of the concave vertex).
// This bridge resolves each tagged section onto the shell `StratumRef` whose
// edge realises it, so the planar concave-trim output carries the boolean
// provenance the rest of this module consumes.
// ---------------------------------------------------------------------------

/// Resolves each source section of a concave trim to the shell
/// `StratumRef::Edge` that realises it, using the [`lift_edges`] convention:
/// the edge's FIRST occurrence across `face_iter()` /
/// `face.absolute_boundaries()` order, with `edge` its flat position there.
/// The refs are returned in input order; a source section the shell does not
/// realise (geometry not matched within `tol`) is a refusal.
pub fn trim_provenance(
    shell: &Shell<Point3, Curve, Surface>,
    sources: &[Curve],
    tol: f64,
) -> Outcome<Vec<StratumRef>> {
    let mut out = Vec::with_capacity(sources.len());
    for source in sources {
        let found = find_shell_edge_ref(shell, source, tol);
        match found {
            Some(r) => out.push(r),
            None => return Err(Refusal::Empty),
        }
    }
    let cert = Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    };
    Ok(Certified::new(out, cert))
}

/// Finds the shell edge whose sampled section geometry equals `source`,
/// returning its `StratumRef::Edge` under the [`lift_edges`] convention.
fn find_shell_edge_ref(
    shell: &Shell<Point3, Curve, Surface>,
    source: &Curve,
    tol: f64,
) -> Option<StratumRef> {
    let s_points = curve_samples(source, tol);
    let s0 = s_points.first().copied()?;
    let s1 = s_points.last().copied()?;
    let mut seen: HashSet<EdgeID<Curve>> = HashSet::default();
    for (fi, face) in shell.face_iter().enumerate() {
        let mut flat = 0usize;
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                if !seen.insert(edge.id()) {
                    flat += 1;
                    continue;
                }
                let e_points = curve_samples(&edge.curve(), tol);
                let same = match (e_points.first().copied(), e_points.last().copied()) {
                    (Some(e0), Some(e1)) => section_matches(e0, e1, s0, s1, tol),
                    _ => false,
                };
                if same {
                    return Some(StratumRef::Edge {
                        solid: SolidRef::A,
                        face: fi,
                        edge: flat,
                    });
                }
                flat += 1;
            }
        }
    }
    None
}

/// Whether two sampled sections coincide as undirected segments.
fn section_matches(a0: Point3, a1: Point3, b0: Point3, b1: Point3, tol: f64) -> bool {
    let forward = (a0 - b0).magnitude() <= tol && (a1 - b1).magnitude() <= tol;
    let reverse = (a0 - b1).magnitude() <= tol && (a1 - b0).magnitude() <= tol;
    forward || reverse
}

// ---------------------------------------------------------------------------
// decision + assembly
// ---------------------------------------------------------------------------

/// The §13.1 decision and the sewing of the kept fragments (decision 4).
///
/// Coincident pairs are resolved ONCE: their verdicts must agree, their flips
/// must match their orientation, and the pair's `a` fragment is emitted (with
/// its flip applied). Non-pair fragments are kept iff
/// [`fragment_decision`] says `Keep`. A kept sweep fragment is emitted as a
/// windowed `SpineFrameSweep` face (BIE-006), and every kept fragment records
/// its `EntityId`/`Op` provenance row.
fn decide_and_assemble(
    op: BoolOp,
    mesh: &FragmentMesh,
    classification: &FragmentClassification,
    rows: &mut Vec<FragmentProvenance>,
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
                let other_parent = match b_origin {
                    FragmentOrigin::A { parent } => parent,
                    FragmentOrigin::B { parent } => parent,
                };
                rows.push(fragment_provenance(a_origin, other_parent, op));
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
            // A kept sweep fragment is emitted as a windowed
            // `SpineFrameSweep` face (BIE-006 decision 3); a fragment whose
            // region is not a clean window keeps its carrier unchanged.
            let mut face = fragment.face.clone();
            if let Some(windowed) = windowed_sweep_face(&face, INSERTION_TOL) {
                face = windowed;
            }
            if flip {
                face.invert();
            }
            kept.push(face);
            // §8.3: the kept fragment's row cites its parent face and the
            // boolean op. A non-paired fragment's other-shell reference is not
            // tracked (there is no event partner); the row cites the parent
            // face, the row's other-slot refinement lands with the sweep path.
            let parent = match fragment.origin {
                FragmentOrigin::A { parent } => parent,
                FragmentOrigin::B { parent } => parent,
            };
            rows.push(fragment_provenance(fragment.origin, parent, op));
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
    use std::f64::consts::{PI, TAU};
    use truck_base::cgmath64::{Matrix4, Point2, Vector4};
    use truck_base::contact::{ContactDimension, ContactEventKind};
    use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
    use truck_evidence::contact::{sweep_stratum, ContactLocus};
    use truck_geometry::arrange::{arrange, Arrangement};
    use truck_geometry::constructive::{
        FrameLaw, LineSpine, Profile2D, ProfileLaw, SpineFrameRecipe, SpineFrameSweep,
    };
    use truck_geometry::nurbs::{BSplineSurface, KnotVec};
    use truck_geometry::prelude::*;
    use truck_geometry::recognize::CanonicalSurface;
    use truck_geometry::specifieds::{Line, Plane, Sphere, UnitCircle};
    use truck_modeling::extrude::extrude_profile;
    use truck_modeling::spine_sweep::spine_sweep;
    use truck_topology::{Vertex, Wire};

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

    // ---------------------------------------------------------------------------
    // BIE-006-CLASSIFY: sweep lift/path adapters + windowed sweep output.
    // ---------------------------------------------------------------------------

    /// The unit-square profile (CCW), the prism fixture's constant section.
    fn unit_square_profile() -> Profile2D {
        Profile2D::try_closed(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ])
        .unwrap()
    }

    /// The unit-prism sweep recipe over the straight spine
    /// `(0, 0, z0) → (0, 0, z1)` with a `FixedPlane` frame: the straight-spine
    /// `SpineFrameSweep` of the junction fixture (the teapot-junction-class
    /// straight-spine × canonical pair). Every side face is a
    /// `Surface::SpineFrameSurface` over `[0, 1] × [j/k, (j+1)/k]`, `k = 4`;
    /// the spine runs along z, so `X(s, v).z = z0 + s·(z1 − z0)`.
    fn prism_recipe(z0: f64, z1: f64) -> SpineFrameRecipe<LineSpine, ProfileLaw, FrameLaw> {
        SpineFrameRecipe::new(
            LineSpine {
                start: Point3::new(0.0, 0.0, z0),
                end: Point3::new(0.0, 0.0, z1),
            },
            ProfileLaw::Constant(unit_square_profile()),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        )
    }

    /// The authored-topology prism sweep solid over `[z0, z1]` along the spine.
    fn prism_sweep_solid(z0: f64, z1: f64) -> Solid<Point3, Curve, Surface> {
        let recipe = prism_recipe(z0, z1);
        spine_sweep(&recipe, &[0.0, 1.0]).unwrap().value
    }

    /// The storage recipe of a prism sweep: the spine converted to its closed
    /// `Curve` carrier and boxed (the form `SpineFrameSweep` stores).
    fn prism_storage_recipe(
        z0: f64,
        z1: f64,
    ) -> SpineFrameRecipe<Box<Curve>, ProfileLaw, FrameLaw> {
        let recipe = prism_recipe(z0, z1);
        SpineFrameRecipe::new(
            Box::new(recipe.spine.into()),
            recipe.profile_law.clone(),
            recipe.frame_law,
        )
    }

    #[test]
    fn sweep_stratum_lifts_with_window() {
        // A straight-spine whole-sweep value over a nontrivial (profile-edge)
        // window.
        let recipe = prism_storage_recipe(0.0, 1.0);
        let sweep = SpineFrameSweep::try_new(recipe, 0.25, 0.75, 0.25, 0.5).unwrap();

        // The lift adapter recognizes the sweep as a bounded `Sweep` stratum
        // carrying the whole-sweep closed value and therefore the same window.
        let stratum = sweep_stratum(sweep.clone());
        let BoundedStratum::Sweep { sweep: lifted } = &stratum else {
            unreachable!("a sweep must lift to the Sweep arm");
        };
        assert_eq!((lifted.s0(), lifted.s1()), (0.25, 0.75));
        assert_eq!((lifted.v0(), lifted.v1()), (0.25, 0.5));

        // The shapeops lift boundary recognizes a stored sweep face the same
        // way (the path an assembled sweep solid rides).
        let solid = prism_sweep_solid(0.0, 1.0);
        let shell = solid.boundaries().first().unwrap();
        let mut sweep_faces = 0usize;
        for face in shell.face_iter() {
            if matches!(face.surface(), Surface::SpineFrameSurface(_)) {
                let Some(BoundedStratum::Sweep { sweep }) = sweep_face_stratum(face) else {
                    unreachable!("a stored sweep face lifts to the Sweep arm");
                };
                assert_eq!(sweep.s0(), 0.0);
                assert_eq!(sweep.s1(), 1.0);
                sweep_faces += 1;
            }
        }
        assert_eq!(sweep_faces, 4, "the prism has four side sweep faces");

        // The `Unrecognized` refusal still fires for a genuinely non-canonical
        // face (the other arm is asserted unchanged).
        let refusal = face_stratum(
            CanonicalCarrierWitness::Unrecognized,
            (0.0, 1.0),
            (0.0, 1.0),
        );
        assert!(
            matches!(
                refusal,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "a genuinely non-canonical face must still refuse at the lift"
        );
    }

    #[test]
    fn sweep_pairs_reach_interaction_solver() {
        // The junction pair: a straight-spine sweep stratum against a canonical
        // plane stratum (BIE-000 kit recipe class).
        let recipe = prism_storage_recipe(0.0, 1.0);
        let sweep = SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).unwrap();
        let sweep_stratum = sweep_stratum(sweep);
        let plane_stratum = BoundedStratum::Face {
            surface: CanonicalSurface::Plane(Plane::xy()),
            u_range: (0.0, 1.0),
            v_range: (0.0, 1.0),
        };

        // Both orders dispatch to the restricted sweep path (BIE-002), never
        // the old `NonCanonicalCarrier` gate and never the deferred envelope:
        // the pair answers with the typed `NumericallyUnresolved` outcome.
        let mut budget_a = Budget::new(0, 0, 0);
        let out = contact(&sweep_stratum, &plane_stratum, &mut budget_a);
        assert!(
            matches!(out, Err(Refusal::NumericallyUnresolved { .. })),
            "the sweep × canonical pair must answer typed-Unresolved, got {out:?}"
        );

        let mut budget_b = Budget::new(0, 0, 0);
        let out = contact(&plane_stratum, &sweep_stratum, &mut budget_b);
        assert!(
            matches!(out, Err(Refusal::NumericallyUnresolved { .. })),
            "the canonical × sweep order must answer typed-Unresolved, got {out:?}"
        );
        assert!(
            !matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier
                ))
            ),
            "a lifted sweep is never refused as NonCanonicalCarrier"
        );
    }

    #[test]
    fn classifier_seeds_from_arrangement_cells() {
        // A sweep solid A strictly inside a canonical box B, no contact
        // events: every face is one fragment. The BIE-006 arrangement-cell
        // seeds decide each fragment's inside/out bit by the containment of its
        // `(s, v)` region representative in the other shell — the sweep shell
        // is classifiable even though a sweep-carrier shell cannot ride the
        // landed canonical-carrier ray gate.
        let a = prism_sweep_solid(0.5, 1.5);
        let shell_a = a.boundaries().first().unwrap().clone();
        let (pb, ab) = box_profile(-1.0, -2.0, 2.0, 1.0);
        let shell_b = extrude_shell(&pb, &ab, 3.0);

        let mesh = split_fragments(&shell_a, &shell_b, &[], TOL).unwrap().value;
        assert_eq!(mesh.fragments.len(), 12, "six sweep faces + six box faces");

        let seeds = classify_from_cells(&shell_a, &shell_b, &mesh, TOL)
            .unwrap()
            .value
            .inside_other;
        assert_eq!(seeds.len(), 12);

        // A is strictly inside B: every sweep fragment's seed is INSIDE
        // (true); the containing box's own fragments are outside A (false).
        for bit in seeds.iter().take(6) {
            assert!(bit, "a sweep fragment inside B must seed inside");
        }
        for bit in seeds.iter().skip(6) {
            assert!(!bit, "a box fragment must seed outside the enclosed sweep");
        }

        // Parity unchanged: the canonical-only control rides the LANDED
        // classifier and reproduces the landed disjoint answer (every bit
        // outside) — the propagation machinery is untouched.
        let (pa, aa) = block_profile();
        let shell_block = extrude_shell(&pa, &aa, 2.0);
        let (pd, ad) = disk_profile(Point2::new(6.0, 6.0), 1.0);
        let shell_disk = extrude_shell(&pd, &ad, 2.0);
        let control_mesh = split_fragments(&shell_block, &shell_disk, &[], TOL)
            .unwrap()
            .value;
        let control = classify_fragments(&shell_block, &shell_disk, &control_mesh, TOL)
            .unwrap()
            .value
            .inside_other;
        assert_eq!(control.len(), 9);
        assert!(
            control.iter().all(|bit| !bit),
            "the landed disjoint canonical control classifies every fragment outside"
        );
    }

    /// The two junction halves of one prism side face: a straight-spine sweep
    /// side face cut at the station `s_mid`. Each half is a genuine face whose
    /// carrier is the ORIGINAL whole-window `SpineFrameSweep` (the split
    /// fragment's carrier — a fragment keeps its parent face's surface) and
    /// whose wire is the rectangular sub-region `[s0, s_mid]` / `[s_mid, s1]`
    /// over the face's ring window. The assembler's windowed emission must
    /// re-window each kept half to exactly that box.
    fn junction_halves(
        face: &Face<Point3, Curve, Surface>,
        sweep: &SpineFrameSweep,
        s_mid: f64,
    ) -> (Face<Point3, Curve, Surface>, Face<Point3, Curve, Surface>) {
        let w0 = sweep.v0();
        let w1 = sweep.v1();
        let s0 = sweep.s0();
        let s1 = sweep.s1();
        let at = |s: f64, v: f64| sweep.subs(s, v);
        let (a00, a01) = (at(s0, w0), at(s0, w1));
        let (b00, b01) = (at(s_mid, w0), at(s_mid, w1));
        let (c00, c01) = (at(s1, w0), at(s1, w1));

        // A ring of four shared corner vertices (closed and simple).
        let ring = |v0: Point3, v1: Point3, v2: Point3, v3: Point3| -> Wire<Point3, Curve> {
            let (a, b, c, d) = (
                Vertex::new(v0),
                Vertex::new(v1),
                Vertex::new(v2),
                Vertex::new(v3),
            );
            let edges = [
                Edge::try_new(&a, &b, Curve::Line(Line(a.point(), b.point()))).unwrap(),
                Edge::try_new(&b, &c, Curve::Line(Line(b.point(), c.point()))).unwrap(),
                Edge::try_new(&c, &d, Curve::Line(Line(c.point(), d.point()))).unwrap(),
                Edge::try_new(&d, &a, Curve::Line(Line(d.point(), a.point()))).unwrap(),
            ];
            Wire::from(edges.to_vec())
        };
        let below_wire = ring(a00, b00, b01, a01);
        let above_wire = ring(b00, c00, c01, b01);
        let surface = face.surface();
        let below = Face::try_new(vec![below_wire], surface.clone()).unwrap();
        let above = Face::try_new(vec![above_wire], surface).unwrap();
        (below, above)
    }

    #[test]
    fn assembler_emits_windowed_sweep_faces() {
        // The junction fixture (teapot-junction class): a straight-spine prism
        // sweep A cut by a canonical plane perpendicular to the spine at the
        // station s_mid (the Difference over the junction removes the sweep
        // material past the cut). The kept halves are the assembler's output
        // fragments: their carriers are still the whole-window sweep, and
        // ASSEMBLE emits them as WINDOWED `SpineFrameSweep` faces (the type
        // already carries windowed domains) with the expected window bounds.
        let a = prism_sweep_solid(0.3, 1.3);
        let shell_a = a.boundaries().first().unwrap().clone();
        let s_mid = 0.5;

        let mut emitted: Vec<(f64, f64, f64, f64)> = Vec::new();
        let mut raw: Vec<(f64, f64, f64, f64)> = Vec::new();
        for face in shell_a.face_iter() {
            let Surface::SpineFrameSurface(sweep) = face.surface() else {
                continue;
            };
            let (below, above) = junction_halves(face, &sweep, s_mid);
            for half in [below, above] {
                let Some(emitted_face) = windowed_sweep_face(&half, TOL) else {
                    let boxed = super::super::sweep_lift::face_parameter_box(&half, TOL);
                    panic!(
                        "windowed emission returned None for a junction half; parameter box: {boxed:?}"
                    );
                };
                let Surface::SpineFrameSurface(emitted_sweep) = emitted_face.surface() else {
                    unreachable!("the emitted face is a windowed SpineFrameSweep");
                };
                emitted.push((
                    emitted_sweep.s0(),
                    emitted_sweep.s1(),
                    emitted_sweep.v0(),
                    emitted_sweep.v1(),
                ));
                let Surface::SpineFrameSurface(raw_sweep) = half.surface() else {
                    unreachable!("the half fragment's carrier is the whole-window sweep");
                };
                raw.push((
                    raw_sweep.s0(),
                    raw_sweep.s1(),
                    raw_sweep.v0(),
                    raw_sweep.v1(),
                ));
            }
        }

        // Four side faces × two halves: every emitted sweep face carries a
        // windowed domain (its own station/ring extent), never the whole
        // `[0, 1] × [j/4, (j+1)/4]` parent window twice.
        assert_eq!(
            emitted.len(),
            8,
            "four side faces emit two windowed halves each"
        );
        assert_eq!(raw.len(), 8);
        let below_windows: Vec<(f64, f64)> = emitted
            .iter()
            .map(|(s0, s1, _, _)| (*s0, *s1))
            .filter(|(_s0, s1)| (*s1 - s_mid).abs() <= TOL)
            .collect();
        let above_windows: Vec<(f64, f64)> = emitted
            .iter()
            .map(|(s0, s1, _, _)| (*s0, *s1))
            .filter(|(s0, _s1)| (*s0 - s_mid).abs() <= TOL)
            .collect();
        assert_eq!(
            below_windows.len(),
            4,
            "the below-cut halves are windowed to [0, s_mid]"
        );
        assert_eq!(
            above_windows.len(),
            4,
            "the above-cut halves are windowed to [s_mid, 1]"
        );
        for (s0, s1) in below_windows.iter().chain(above_windows.iter()) {
            assert!(
                *s0 >= 0.0 && *s1 <= 1.0,
                "windows stay inside the parent sweep"
            );
        }
    }

    // -----------------------------------------------------------------------
    // CFP-001 fixture data, copied from `truck-certified/src/cfp/fixtures.rs`
    // as read-only constants (F1 — never import the module). F-C0 exercises the
    // lift screen (`face_enclosure`) on the DSC defect counterexample; F-C1 the
    // parameter box on its twin.
    // -----------------------------------------------------------------------

    /// The F-C0 sphere-cap fixture data, verbatim (sphere centre, radius, cap
    /// plane `z = cap_plane_z`, rim radius, apex, slab box, and the screen
    /// records).
    const FC0_SPHERE_CENTRE: [f64; 3] = [0.0, 0.0, 0.0];
    const FC0_SPHERE_RADIUS: f64 = 5.0;
    const FC0_CAP_PLANE_Z: f64 = -3.0;
    const FC0_CAP_RIM_RADIUS: f64 = 4.0;
    const FC0_APEX: [f64; 3] = [0.0, 0.0, -5.0];
    const FC0_SLAB_LO: [f64; 3] = [-10.0, -10.0, -6.0];
    const FC0_SLAB_HI: [f64; 3] = [10.0, 10.0, -4.0];
    const FC0_SAMPLED_LO: [f64; 3] = [-4.0, -4.0, -3.0];
    const FC0_SAMPLED_HI: [f64; 3] = [4.0, 4.0, -3.0];

    /// The F-C1 parameter-twin data, verbatim.
    const FC1_TRUE_EXTENT: ((f64, f64), (f64, f64)) = ((0.0, 1.0), (0.0, 1.0));
    const FC1_BOUNDARY_POLYGON: [[f64; 2]; 4] = [[0.1, 0.1], [0.4, 0.1], [0.4, 0.4], [0.1, 0.4]];

    /// Whether an AABB contains the point (inclusive).
    fn aabb_contains(b: &Aabb, p: Point3) -> bool {
        b.lo.x <= p.x
            && p.x <= b.hi.x
            && b.lo.y <= p.y
            && p.y <= b.hi.y
            && b.lo.z <= p.z
            && p.z <= b.hi.z
    }

    /// The F-C0 cap face: the unit-5 sphere about the origin, bounded by the
    /// rim circle at `z = −3` (radius 4). A whole-carrier face: the trim region
    /// is the southern cap whose apex `(0, 0, −5)` is strictly below the rim —
    /// the DSC boundary-sample screen hulls only the rim and silently drops the
    /// cap's interior.
    fn fc0_cap_face() -> Face<Point3, Curve, Surface> {
        let sphere = Surface::Sphere(Sphere::new(
            Point3::new(
                FC0_SPHERE_CENTRE[0],
                FC0_SPHERE_CENTRE[1],
                FC0_SPHERE_CENTRE[2],
            ),
            FC0_SPHERE_RADIUS,
        ));
        let arc = |t0: f64, t1: f64| -> Curve {
            Curve::Circle(Processor::with_transform(
                TrimmedCurve::new(UnitCircle::<Point3>::new(), (t0, t1)),
                Matrix4 {
                    x: Vector4::new(FC0_CAP_RIM_RADIUS, 0.0, 0.0, 0.0),
                    y: Vector4::new(0.0, FC0_CAP_RIM_RADIUS, 0.0, 0.0),
                    z: Vector4::new(0.0, 0.0, 1.0, 0.0),
                    w: Vector4::new(
                        FC0_SPHERE_CENTRE[0],
                        FC0_SPHERE_CENTRE[1],
                        FC0_CAP_PLANE_Z,
                        1.0,
                    ),
                },
            ))
        };
        let v0 = Vertex::new(Point3::new(FC0_CAP_RIM_RADIUS, 0.0, FC0_CAP_PLANE_Z));
        let v1 = Vertex::new(Point3::new(-FC0_CAP_RIM_RADIUS, 0.0, FC0_CAP_PLANE_Z));
        let edge0 = Edge::try_new(&v0, &v1, arc(0.0, PI)).unwrap();
        let edge1 = Edge::try_new(&v1, &v0, arc(PI, TAU)).unwrap();
        Face::try_new(vec![Wire::from(vec![edge0, edge1])], sphere).unwrap()
    }

    /// The F-C0 slab's top face: the plane at `z = −4` spanning the slab's
    /// `[−10, 10]²` footprint.
    fn fc0_slab_top_face() -> Face<Point3, Curve, Surface> {
        let plane = Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, FC0_SLAB_HI[2]),
            Point3::new(1.0, 0.0, FC0_SLAB_HI[2]),
            Point3::new(0.0, 1.0, FC0_SLAB_HI[2]),
        ));
        let (x0, y0, z) = (FC0_SLAB_LO[0], FC0_SLAB_LO[1], FC0_SLAB_HI[2]);
        let (x1, y1) = (FC0_SLAB_HI[0], FC0_SLAB_HI[1]);
        let corners = [
            Point3::new(x0, y0, z),
            Point3::new(x1, y0, z),
            Point3::new(x1, y1, z),
            Point3::new(x0, y1, z),
        ];
        let verts: Vec<Vertex<Point3>> = corners.iter().map(|&p| Vertex::new(p)).collect();
        let line = |a: &Vertex<Point3>, b: &Vertex<Point3>| {
            Edge::try_new(a, b, Curve::Line(Line(a.point(), b.point()))).unwrap()
        };
        let n = verts.len();
        let mut edges = Vec::new();
        for i in 0..n {
            edges.push(line(&verts[i], &verts[(i + 1) % n]));
        }
        Face::try_new(vec![Wire::from(edges)], plane).unwrap()
    }

    /// The F-C1 face: a cubic B-spline carrier over the clamped unit square,
    /// trimmed to the recorded boundary polygon (the F-C1 twin: the boundary
    /// polygon's hull is strictly inside the trim's true parameter extent, the
    /// carrier's own domain).
    fn fc1_trim_face() -> Face<Point3, Curve, Surface> {
        let net = [
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 2.0, 0.0],
                [0.0, 3.0, 0.0],
            ],
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 2.0, 2.0],
                [1.0, 3.0, 3.0],
            ],
            [
                [2.0, 0.0, 0.0],
                [2.0, 1.0, 2.0],
                [2.0, 2.0, 4.0],
                [2.0, 3.0, 6.0],
            ],
            [
                [3.0, 0.0, 0.0],
                [3.0, 1.0, 3.0],
                [3.0, 2.0, 6.0],
                [3.0, 3.0, 9.0],
            ],
        ];
        let ctrl: Vec<Vec<Point3>> = net
            .iter()
            .map(|row| row.iter().map(|&p| Point3::new(p[0], p[1], p[2])).collect())
            .collect();
        let bsp = BSplineSurface::new((KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)), ctrl);
        let surface = Surface::BSplineSurface(bsp);
        // The trim's boundary polygon in the carrier's own `(u, v)` frame,
        // realized on the carrier: vertices at the recorded polygon corners.
        let pts: Vec<Point3> = FC1_BOUNDARY_POLYGON
            .iter()
            .map(|v| surface.subs(v[0], v[1]))
            .collect();
        let verts: Vec<Vertex<Point3>> = pts.iter().map(|&p| Vertex::new(p)).collect();
        let line = |a: &Vertex<Point3>, b: &Vertex<Point3>| {
            Edge::try_new(a, b, Curve::Line(Line(a.point(), b.point()))).unwrap()
        };
        let n = verts.len();
        let mut edges = Vec::new();
        for i in 0..n {
            edges.push(line(&verts[i], &verts[(i + 1) % n]));
        }
        Face::try_new(vec![Wire::from(edges)], surface).unwrap()
    }

    /// F-C0 (the DSC defect counterexample): the boundary-sample screen drops
    /// the cap × slab pair; the certified `face_enclosure` screen admits it
    /// (BG-ENC-001 — the sampled AABB does not contain the cap apex, the
    /// certified one does).
    #[test]
    fn fc0_defect_counterexample_red_on_sampled_green_on_certified() {
        let cap = fc0_cap_face();
        let slab_top = fc0_slab_top_face();

        // Red: the recorded boundary-sample AABBs do not touch — the sampled
        // screen silently drops the pair (the defect).
        let sampled_cap = Aabb {
            lo: Point3::new(FC0_SAMPLED_LO[0], FC0_SAMPLED_LO[1], FC0_SAMPLED_LO[2]),
            hi: Point3::new(FC0_SAMPLED_HI[0], FC0_SAMPLED_HI[1], FC0_SAMPLED_HI[2]),
        };
        let slab_solid = Aabb {
            lo: Point3::new(FC0_SLAB_LO[0], FC0_SLAB_LO[1], FC0_SLAB_LO[2]),
            hi: Point3::new(FC0_SLAB_HI[0], FC0_SLAB_HI[1], FC0_SLAB_HI[2]),
        };
        assert!(
            !sampled_cap.touches(&slab_solid),
            "the sampled cap screen box must not reach the slab (the defect)"
        );
        assert!(
            !sampled_cap.touches(&face_enclosure(&slab_top, TOL)),
            "the sampled cap screen box must not touch the slab's certified box"
        );
        // The sampled box does not even contain the cap apex — the interior the
        // defect record says leaves the boundary hull.
        assert!(!aabb_contains(
            &sampled_cap,
            Point3::new(FC0_APEX[0], FC0_APEX[1], FC0_APEX[2])
        ));

        // Green: the certified enclosures cover the true image and the pair is
        // admitted by the screen (`face_enclosure` on both faces touches).
        let cap_box = face_enclosure(&cap, TOL);
        assert!(
            aabb_contains(&cap_box, Point3::new(FC0_APEX[0], FC0_APEX[1], FC0_APEX[2])),
            "the certified cap enclosure must contain the apex"
        );
        assert!(
            cap_box.touches(&face_enclosure(&slab_top, TOL)),
            "the certified screens must admit the cap × slab pair"
        );
    }

    /// F-C1: the face's parameter box (`face_uv_box`) covers the trim's true
    /// parameter extent; the boundary polygon hull does not.
    #[test]
    fn param_box_derived_from_trim_not_boundary() {
        let face = fc1_trim_face();
        let uv = face_uv_box(&face, TOL).expect("the trim face has a parameter box");
        // The recorded boundary-polygon hull is strictly inside the true extent
        // (the F-C1 ground truth).
        let poly_lo = (0.1f64, 0.1f64);
        let poly_hi = (0.4f64, 0.4f64);
        assert!(
            poly_lo.0 > FC1_TRUE_EXTENT.0 .0 && poly_hi.0 < FC1_TRUE_EXTENT.0 .1,
            "the F-C1 boundary hull is strictly inside the true extent"
        );
        // The face's parameter box is derived from the carrier (the trim's true
        // extent), so it covers the full recorded extent...
        assert!(
            uv.0 .0 <= FC1_TRUE_EXTENT.0 .0 + 1.0e-12 // H-3: parameter slack on the recorded extent endpoint
                && uv.0 .1 >= FC1_TRUE_EXTENT.0 .1 - 1.0e-12
                && uv.1 .0 <= FC1_TRUE_EXTENT.1 .0 + 1.0e-12
                && uv.1 .1 >= FC1_TRUE_EXTENT.1 .1 - 1.0e-12,
            "the trim parameter box must cover the true extent, got {uv:?}"
        );
        // ...and, unlike the boundary hull, strictly covers it somewhere.
        assert!(
            uv.0 .0 < poly_lo.0 && uv.1 .0 < poly_lo.1,
            "the trim parameter box must leave the boundary hull: {uv:?}"
        );
    }

    /// Reach assertion: the sweep screen's per-face AABB is the certified
    /// `face_enclosure`, not a sampled boundary box — running the sweep on the
    /// flagship pair reaches it, and every lifted face's screen box certifiably
    /// contains the image of its own carrier's parameter mid-point.
    #[test]
    fn face_enclosure_used_by_lift_screen() {
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);
        let solid_a = Solid::try_new(vec![shell_a.clone()]).unwrap();
        let solid_b = Solid::try_new(vec![shell_b.clone()]).unwrap();
        // The sweep consumes face_enclosure for its screen; the flagship pair
        // still resolves its six-event complex through it.
        let events = sweep_contact_events(&solid_a, &solid_b, TOL).unwrap().value;
        assert_eq!(events.len(), 6, "the certified screen reaches the oracle");
        for shell in [&shell_a, &shell_b] {
            for face in shell.face_iter() {
                let b = face_enclosure(face, TOL);
                let Some(uv) = face_uv_box(face, TOL) else {
                    continue;
                };
                let u = (uv.0 .0 + uv.0 .1) / 2.0;
                let v = (uv.1 .0 + uv.1 .1) / 2.0;
                let p = face.surface().subs(u, v);
                assert!(
                    aabb_contains(&b, p),
                    "the screen box must certifiably contain the carrier at the extent mid-point"
                );
            }
        }
    }

    #[test]
    fn self_pair_rewrites_before_sweep() {
        // The §5.8 entry gate: certified-identical operands (equal EntityId)
        // rewrite before the sweep — A ∪ A = A, A ∩ A = A, A − A = ∅, A △ A =
        // ∅ — while the landed typed refusal remains for an identity the
        // boundary cannot certify.
        let (profile, arr) = box_profile(0.0, 0.0, 2.0, 2.0);
        let a = Solid::try_new(vec![extrude_shell(&profile, &arr, 2.0)]).expect("cube A");
        let id = EntityId::src(7);
        let a_faces = a
            .boundaries()
            .first()
            .expect("one shell")
            .face_iter()
            .count();

        let mut union_budget = Budget::new(1000, 1000, 1000);
        let union = boolean_certified_operands(&id, &a, BoolOp::Union, &id, &a, &mut union_budget)
            .expect("A ∪ A rewrites as A")
            .value;
        assert_eq!(union.boundaries().len(), 1, "A ∪ A is one shell");
        assert_eq!(
            union
                .boundaries()
                .first()
                .expect("one shell")
                .face_iter()
                .count(),
            a_faces,
            "A ∪ A keeps A's face record"
        );

        let mut inter_budget = Budget::new(1000, 1000, 1000);
        let intersection =
            boolean_certified_operands(&id, &a, BoolOp::Intersection, &id, &a, &mut inter_budget)
                .expect("A ∩ A rewrites as A")
                .value;
        assert_eq!(intersection.boundaries().len(), 1);
        assert_eq!(
            intersection
                .boundaries()
                .first()
                .expect("one shell")
                .face_iter()
                .count(),
            a_faces,
            "A ∩ A keeps A's face record"
        );

        for op in [BoolOp::Difference, BoolOp::Xor] {
            let mut budget = Budget::new(1000, 1000, 1000);
            let empty = boolean_certified_operands(&id, &a, op, &id, &a, &mut budget)
                .unwrap_or_else(|e| panic!("A {op:?} A rewrites as ∅, got {e:?}"))
                .value;
            assert!(
                empty.boundaries().is_empty(),
                "A {op:?} A is the empty solid"
            );
        }

        // The landed typed refusal remains for identity this boundary cannot
        // certify: `boolean` on the same solid value, with no certified ids,
        // still refuses the typed envelope (never a panic, never a guess).
        let mut budget = Budget::new(1000, 1000, 1000);
        let uncertified = boolean(&a, BoolOp::Union, &a, &mut budget);
        assert!(
            matches!(
                uncertified,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "uncertified self-pair identity keeps the typed refusal, got {uncertified:?}"
        );
    }

    // ---------------------------------------------------------------------------
    // CTE-007-T2ARRANGE: the coplanar full-face butt-join union drops the
    // shared wall (T2 §5.9; R2 addition, commit 4d59d5b).
    // ---------------------------------------------------------------------------

    /// The `(axis, coord)` of a plane face: `axis` 0/1/2 = x/y/z, `coord` the
    /// plane's constant coordinate.
    fn plane_axis_coord(face: &Face<Point3, Curve, Surface>) -> Option<(usize, f64)> {
        let Surface::Plane(plane) = face.surface() else {
            return None;
        };
        let n = plane.normal();
        let axis = if n.x.abs() > 0.5 {
            0
        } else if n.y.abs() > 0.5 {
            1
        } else {
            2
        };
        let coord = match axis {
            0 => plane.origin().x,
            1 => plane.origin().y,
            _ => plane.origin().z,
        };
        Some((axis, coord))
    }

    #[test]
    fn butt_join_union_drops_shared_wall() {
        // The coplanar full-face butt join, x- and y-axis (R2 addition, commit
        // 4d59d5b): two 2-cubes sharing an ENTIRE vertical face. The two seam
        // side faces are stored with the SAME orientation flag yet their
        // outward normals oppose, so the geometric orientation test must label
        // the Region2 coincident pair `Anti` — the flag-only test would keep
        // the wall interior to the union. With the pair `Anti`, the §5.9 union
        // row `10 ∨ 01 = 11` drops the shared wall on both members, exactly as
        // the landed orientation-consistency fold (`assemble.rs`) already does
        // for the z-axis twin. The splitter now resolves the seam (the
        // recorded `split.rs::finish` refusal is gone) and the assembled kept
        // set carries no face on the seam plane.
        for (profile, seam_axis, what) in [
            (
                box_profile(2.0, 0.0, 4.0, 2.0),
                0,
                "x-axis full-face butt join",
            ),
            (
                box_profile(0.0, 2.0, 2.0, 4.0),
                1,
                "y-axis full-face butt join",
            ),
        ] {
            let (b_profile, b_arr) = profile;
            let (a_profile, a_arr) = box_profile(0.0, 0.0, 2.0, 2.0);
            let a = Solid::try_new(vec![extrude_shell(&a_profile, &a_arr, 2.0)]).expect("cube A");
            let b = Solid::try_new(vec![extrude_shell(&b_profile, &b_arr, 2.0)]).expect("cube B");
            let shell_a = a.boundaries().first().expect("one shell").clone();
            let shell_b = b.boundaries().first().expect("one shell").clone();

            // The seam is a single Region2 coincident event between the two
            // vertical side faces.
            let events = sweep_contact_events(&a, &b, TOL)
                .expect("the seam sweep resolves")
                .value;
            let region2: Vec<_> = events
                .iter()
                .filter(|e| e.record.dimension == ContactDimension::Region2)
                .collect();
            assert_eq!(
                region2.len(),
                1,
                "{what}: exactly one coincident seam event"
            );

            // The splitter resolves the seam (the recorded finish refusal is
            // gone) and emits ONE coincident pair, geometrically Anti.
            let mesh = split_fragments(&shell_a, &shell_b, &events, TOL)
                .unwrap_or_else(|e| panic!("{what}: the seam split must certify, got {e:?}"))
                .value;
            assert_eq!(mesh.coincident.len(), 1, "{what}: one coincident seam pair");
            let pair = mesh.coincident.first().expect("the seam pair");
            assert_eq!(
                pair.orientation,
                CoincidentOrientation::Anti,
                "{what}: the seam wall pair must be geometrically Anti"
            );

            // Classify and decide the union: the `10 ∨ 01 = 11` row drops the
            // shared wall — no kept face lies on the seam plane.
            let classification = classify_fragments(&shell_a, &shell_b, &mesh, TOL)
                .unwrap_or_else(|e| panic!("{what}: the seam mesh classifies, got {e:?}"))
                .value;
            let mut rows: Vec<FragmentProvenance> = Vec::new();
            let kept = decide_and_assemble(BoolOp::Union, &mesh, &classification, &mut rows)
                .unwrap_or_else(|e| panic!("{what}: the union decides, got {e:?}"));
            assert!(!kept.is_empty(), "{what}: exterior faces survive");
            for face in &kept {
                let Some((axis, coord)) = plane_axis_coord(face) else {
                    continue;
                };
                if axis == seam_axis {
                    assert!(
                        (coord - 2.0).abs() > TOL,
                        "{what}: the shared wall at the seam plane must be dropped"
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // CFP-009-BROADPHASE: the uniform-grid broadphase replaces the flat AABB
    // double loop at the boolean entry. The observable contract is EXACTNESS:
    // the indexed sweep admits precisely the pair set the flat loop admits, in
    // the flat loop's canonical order. Test 1 compares the two pair sets
    // exhaustively (both computed in the test) on every landed boolean
    // fixture; the determinism of the index itself is exercised both here and
    // in `broadphase.rs`; test 3 pins the empty short-circuit.
    // -----------------------------------------------------------------------

    /// The flat screen's admitted pair set, tagged by screen (0 = FF, 1 = FE,
    /// 2 = EF, 3 = EE) in the flat loop's canonical emission order. The EE
    /// screen applies the `ee_circle_circle` skip exactly as the sweep does.
    fn flat_screen_pairs(
        faces_a: &[LiftedFace],
        edges_a: &[LiftedEdge],
        faces_b: &[LiftedFace],
        edges_b: &[LiftedEdge],
    ) -> Vec<(u8, usize, usize)> {
        let mut out: Vec<(u8, usize, usize)> = Vec::new();
        for (i, fa) in faces_a.iter().enumerate() {
            for (j, fb) in faces_b.iter().enumerate() {
                if fa.aabb.touches(&fb.aabb) {
                    out.push((0, i, j));
                }
            }
        }
        for (i, fa) in faces_a.iter().enumerate() {
            for (j, eb) in edges_b.iter().enumerate() {
                if fa.aabb.touches(&eb.aabb) {
                    out.push((1, i, j));
                }
            }
        }
        for (i, ea) in edges_a.iter().enumerate() {
            for (j, fb) in faces_b.iter().enumerate() {
                if ea.aabb.touches(&fb.aabb) {
                    out.push((2, i, j));
                }
            }
        }
        for (i, ea) in edges_a.iter().enumerate() {
            for (j, eb) in edges_b.iter().enumerate() {
                if ea.aabb.touches(&eb.aabb) && !ee_circle_circle(&ea.stratum, &eb.stratum) {
                    out.push((3, i, j));
                }
            }
        }
        out
    }

    /// The broadphase screen's admitted pair set, tagged exactly as
    /// [`flat_screen_pairs`] and in the same canonical order.
    fn indexed_screen_pairs(
        faces_a: &[LiftedFace],
        edges_a: &[LiftedEdge],
        faces_b: &[LiftedFace],
        edges_b: &[LiftedEdge],
    ) -> Vec<(u8, usize, usize)> {
        let mut out: Vec<(u8, usize, usize)> = Vec::new();
        for (i, j) in candidate_touching_pairs(faces_a, faces_b) {
            out.push((0, i, j));
        }
        for (i, j) in candidate_touching_pairs(faces_a, edges_b) {
            out.push((1, i, j));
        }
        for (i, j) in candidate_touching_pairs(edges_a, faces_b) {
            out.push((2, i, j));
        }
        for (i, j) in candidate_touching_pairs(edges_a, edges_b) {
            if !ee_circle_circle(&edges_a[i].stratum, &edges_b[j].stratum) {
                out.push((3, i, j));
            }
        }
        out
    }

    /// One fixture's exhaustive pair-set gate: lifts both solids and compares
    /// the indexed sweep's pair set with the flat loop's pair set, screen by
    /// screen and element by element (the comparison is ordered, so it also
    /// pins the canonical emission order). Re-runs the indexed computation to
    /// pin determinism on the real lifted data.
    fn assert_fixture_pairs_match(
        label: &str,
        a: &Solid<Point3, Curve, Surface>,
        b: &Solid<Point3, Curve, Surface>,
    ) {
        let shell_a = a.boundaries().first().expect("A is single-shell");
        let shell_b = b.boundaries().first().expect("B is single-shell");
        let faces_a = lift_faces(SolidRef::A, shell_a, TOL).expect("A faces lift");
        let edges_a = lift_edges(SolidRef::A, shell_a, TOL).expect("A edges lift");
        let faces_b = lift_faces(SolidRef::B, shell_b, TOL).expect("B faces lift");
        let edges_b = lift_edges(SolidRef::B, shell_b, TOL).expect("B edges lift");
        let flat = flat_screen_pairs(&faces_a, &edges_a, &faces_b, &edges_b);
        let indexed = indexed_screen_pairs(&faces_a, &edges_a, &faces_b, &edges_b);
        assert_eq!(
            flat, indexed,
            "{label}: the indexed pair set must equal the flat pair set exactly"
        );
        let rerun = indexed_screen_pairs(&faces_a, &edges_a, &faces_b, &edges_b);
        assert_eq!(
            indexed, rerun,
            "{label}: identical input must reproduce the pair sequence"
        );
    }

    /// Required test 1: on every landed boolean fixture, the indexed sweep's
    /// pair set equals the flat loop's pair set exactly (both computed in the
    /// test; exhaustive ordered comparison per screen).
    #[test]
    fn indexed_pair_set_identical_to_flat_loop() {
        // The flagship: the 4x4 block x the (2, 2) r=1 disk (the six-event
        // complex; exercises FF/FE/EE screens over planes, the cylinder wall
        // and the rim circles).
        let (pa, aa) = block_profile();
        let a_block = Solid::try_new(vec![extrude_shell(&pa, &aa, 2.0)]).expect("flagship A");
        let (pb, ab) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let b_disk = Solid::try_new(vec![extrude_shell(&pb, &ab, 2.0)]).expect("flagship B");
        assert_fixture_pairs_match("flagship block x disk", &a_block, &b_disk);

        // The coplanar full-face butt joins (x and y axes) of the R2 addition:
        // two 2-cubes sharing an entire vertical face.
        let (px, ax) = box_profile(0.0, 0.0, 2.0, 2.0);
        let a_cube = Solid::try_new(vec![extrude_shell(&px, &ax, 2.0)]).expect("cube A");
        let (px2, ax2) = box_profile(2.0, 0.0, 4.0, 2.0);
        let b_butt_x = Solid::try_new(vec![extrude_shell(&px2, &ax2, 2.0)]).expect("cube Bx");
        assert_fixture_pairs_match("x-axis butt join", &a_cube, &b_butt_x);
        let (py, ay) = box_profile(0.0, 2.0, 2.0, 4.0);
        let b_butt_y = Solid::try_new(vec![extrude_shell(&py, &ay, 2.0)]).expect("cube By");
        assert_fixture_pairs_match("y-axis butt join", &a_cube, &b_butt_y);

        // Two separated 2-cubes (the multishell/disjoint pair, as one-shell
        // solids): no certified box can touch.
        let (pd, ad) = box_profile(10.0, 10.0, 12.0, 12.0);
        let b_far = Solid::try_new(vec![extrude_shell(&pd, &ad, 2.0)]).expect("far cube");
        assert_fixture_pairs_match("disjoint cubes", &a_cube, &b_far);

        // The disk x itself (two independent co-located disk solids): the EE
        // circle-circle rim pair exercises the `ee_circle_circle` skip on both
        // the flat and the indexed screen.
        let b_disk2 = Solid::try_new(vec![extrude_shell(&pb, &ab, 2.0)]).expect("disk A'");
        assert_fixture_pairs_match("co-located disk x disk", &b_disk, &b_disk2);

        // A cube partially overlapping the disk's footprint (not a full-face
        // seam, not a containment): a general-position boolean screen.
        let (pc, ac) = box_profile(-1.0, 0.0, 1.5, 2.0);
        let b_overlap = Solid::try_new(vec![extrude_shell(&pc, &ac, 2.0)]).expect("overlap cube");
        assert_fixture_pairs_match("block x overlapping cube", &a_block, &b_overlap);
    }

    /// Required test 3: a boolean of two solids whose certified boxes cannot
    /// touch produces zero candidates and the same (empty-event) outcome as
    /// the flat loop.
    #[test]
    fn empty_index_short_circuits_clean() {
        let (pa, aa) = box_profile(0.0, 0.0, 2.0, 2.0);
        let a = Solid::try_new(vec![extrude_shell(&pa, &aa, 2.0)]).expect("cube A");
        let (pb, ab) = box_profile(10.0, 10.0, 12.0, 12.0);
        let b = Solid::try_new(vec![extrude_shell(&pb, &ab, 2.0)]).expect("cube B");

        // The indexed sweep short-circuits to zero contact events.
        let events = sweep_contact_events(&a, &b, TOL)
            .expect("the sweep resolves")
            .value;
        assert!(
            events.is_empty(),
            "no certified box can touch across the gap, got {} events",
            events.len()
        );

        // The flat loop admits the same (empty) pair set on the same lifts.
        let shell_a = a.boundaries().first().expect("A is single-shell");
        let shell_b = b.boundaries().first().expect("B is single-shell");
        let faces_a = lift_faces(SolidRef::A, shell_a, TOL).expect("A faces lift");
        let edges_a = lift_edges(SolidRef::A, shell_a, TOL).expect("A edges lift");
        let faces_b = lift_faces(SolidRef::B, shell_b, TOL).expect("B faces lift");
        let edges_b = lift_edges(SolidRef::B, shell_b, TOL).expect("B edges lift");
        let flat = flat_screen_pairs(&faces_a, &edges_a, &faces_b, &edges_b);
        let indexed = indexed_screen_pairs(&faces_a, &edges_a, &faces_b, &edges_b);
        assert!(
            flat.is_empty(),
            "the flat screen admits nothing across the gap"
        );
        assert!(
            indexed.is_empty(),
            "the indexed screen admits nothing across the gap"
        );

        // The same empty-event outcome as the flat loop end to end: the
        // intersection of two separated solids is the empty solid.
        let mut budget = Budget::new(1000, 1000, 1000);
        let inter = boolean(&a, BoolOp::Intersection, &b, &mut budget)
            .expect("the intersection of separated solids assembles");
        assert!(
            inter.value.boundaries().is_empty(),
            "the intersection of separated solids is the empty solid"
        );
    }
}
