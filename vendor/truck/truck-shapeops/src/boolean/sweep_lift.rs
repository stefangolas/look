//! BIE-006-CLASSIFY — the sweep lift/path adapters + windowed sweep output
//! (the pipeline-tie-in packet of the Certified Interaction Engine).
//!
//! The landed LIFT stage refused swept faces: `recognize_surface` has no
//! canonical arm for a `Surface::SpineFrameSurface`, so a sweep face stopped
//! at the `Unrecognized` gate with `NonCanonicalCarrier`. This module is the
//! BIE-006 adapter set that lets a `SpineFrameSweep` face through the landed
//! boolean funnel:
//!
//! - [`sweep_face_stratum`] recognizes a sweep face at the lift boundary and
//!   lifts it to `BoundedStratum::Sweep` (the closed value carries the whole
//!   recipe and the realized window `[s0, s1] × [v0, v1]`), instead of the
//!   `Unrecognized` refusal.
//! - [`classify_from_cells`] seeds the fragment bits from the BIE-005
//!   arrangement-cell containment semantics (the inside/out bit of a fragment
//!   is the containment of its `(s, v)` region representative in the OTHER
//!   solid's closure), so a sweep-carrier shell is classifiable without the
//!   landed canonical-carrier ray gate. The parity propagation rules
//!   (`Same`/`Flip` along the adjacency graph) are unchanged; the classifier
//!   logic itself is not edited.
//! - [`windowed_sweep_face`] emits a kept sweep fragment as a face whose
//!   carrier is a windowed `SpineFrameSweep` (the type already carries
//!   windowed domains) — ASSEMBLE emits output faces directly, no new surface
//!   type.
//! - [`fragment_provenance`] builds the `EntityId`/`Op` row of a kept output
//!   fragment: the output entity's `Op` cites the input strata and the boolean
//!   operation (§8.3).
//!
//! House rules H-1..H-8 apply.

use super::classify::FragmentClassification;
use super::split::{
    create_parameter_boundary, region_representative, AdjacencyParity, FragmentMesh, FragmentOrigin,
};
use super::BoolOp;
use rustc_hash::FxHashMap as HashMap;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point2, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth, UnresolvedWitness,
};
use truck_evidence::contact::BoundedStratum;
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::constructive::SpineFrameSweep;
use truck_geotrait::{ParametricSurface, SearchParameter};
use truck_meshalgo::prelude::PolylineCurve;
use truck_topology::entity_id::{EntityId, Op, OpKind, OpParams};
use truck_topology::{EdgeID, Face, Shell, Wire};

/// The re-window endpoint snap tolerance (H-3): a fragment whose parameter box
/// endpoint sits within this of an endpoint of its parent face's window is the
/// parent's own boundary (a ring/trajectory edge endpoint), so the windowed
/// output face snaps to the exact stored endpoint.
const WINDOW_SNAP: f64 = 1.0e-6; // H-3: dimensionless window-snap slack

/// The number of `search_parameter` trials for a window-vertex inversion.
const SEARCH_TRIALS: usize = 100;

/// Unwraps a periodic parameter onto the branch of `previous`.
fn unwrap_periodic(period: f64, previous: f64, value: f64) -> f64 {
    let mut delta = value - previous;
    delta -= (delta / period).round() * period;
    previous + delta
}

/// Recognizes a sweep face at the lift boundary and lifts it to its bounded
/// sweep stratum (BIE-006).
///
/// A stored face whose surface is the whole-sweep closed value
/// (`Surface::SpineFrameSurface`) lifts to `BoundedStratum::Sweep` carrying
/// that closed value — recipe once, realized window, placement. Any other
/// face returns `None` and rides the landed canonical lift unchanged.
pub(crate) fn sweep_face_stratum(face: &Face<Point3, Curve, Surface>) -> Option<BoundedStratum> {
    match face.surface() {
        Surface::SpineFrameSurface(sweep) => Some(BoundedStratum::Sweep { sweep }),
        _ => None,
    }
}

/// The `(u, v)` parameter box of a face: the min/max over the parameter
/// polygons of its absolute boundary wires, in the stored frame. For a sweep
/// face this is its realized `(s, v)` window extent.
pub(crate) fn face_parameter_box(
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    let mut u_lo = f64::INFINITY;
    let mut u_hi = f64::NEG_INFINITY;
    let mut v_lo = f64::INFINITY;
    let mut v_hi = f64::NEG_INFINITY;
    for wire in face.absolute_boundaries() {
        // The boundary's parameter samples; a sample inversion that fails on
        // a full wire falls back to the wire's own vertices (a window piece's
        // extremes are its corners).
        let points: Vec<Point2> = match create_parameter_boundary(face, wire, &mut cache, tol) {
            Some(poly) => poly.iter().copied().collect(),
            None => wire_vertex_parameters(face, wire)?,
        };
        for p in points {
            u_lo = u_lo.min(p.x);
            u_hi = u_hi.max(p.x);
            v_lo = v_lo.min(p.y);
            v_hi = v_hi.max(p.y);
        }
    }
    if !(u_lo < u_hi && v_lo < v_hi) {
        return None;
    }
    Some(((u_lo, u_hi), (v_lo, v_hi)))
}

/// The `(u, v)` parameters of a wire's vertices on the face's carrier.
fn wire_vertex_parameters(
    face: &Face<Point3, Curve, Surface>,
    wire: &Wire<Point3, Curve>,
) -> Option<Vec<Point2>> {
    let surface = face.surface();
    let mut out: Vec<Point2> = Vec::new();
    for vertex in wire.vertex_iter() {
        let mut uv: Point2 = surface
            .search_parameter(vertex.point(), None, SEARCH_TRIALS)?
            .into();
        if let Some(period) = surface.u_period() {
            if let Some(last) = out.last() {
                uv.x = unwrap_periodic(period, last.x, uv.x);
            }
        }
        out.push(uv);
    }
    Some(out)
}

/// Emits a kept sweep fragment as a face whose carrier is a WINDOWED
/// `SpineFrameSweep` (BIE-006 decision 3).
///
/// A split fragment of a sweep face is a trimmed sub-region of the parent
/// whole-sweep window; when that region is a parameter box (the interaction
/// curves are spine-station/ring-aligned, so the fragment's own `(s, v)`
/// extent is its window), the output face's carrier is re-windowed to exactly
/// that box — the same window the closed `SpineFrameSweep` value already
/// carries (spec §5.10). No new surface type: the swept-face type already
/// carries windowed domains.
///
/// A face that is not a sweep, or whose trimmed region is not a clean window
/// box, returns `None` and the caller emits the fragment face unchanged (its
/// boundary wires already trim it correctly; the re-window is an honest window
/// report, never a geometry change).
pub(crate) fn windowed_sweep_face(
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<Face<Point3, Curve, Surface>> {
    let Surface::SpineFrameSurface(sweep) = face.surface() else {
        return None;
    };
    let ((u_lo, u_hi), (v_lo, v_hi)) = face_parameter_box(face, tol)?;

    // The fragment's region lives inside the parent's window; snap a box
    // endpoint that sits on the parent boundary to the exact stored endpoint
    // so the windowed value closes on the shared ring/trajectory edges.
    let s0 = snap_to(sweep.s0(), u_lo, WINDOW_SNAP);
    let s1 = snap_to(sweep.s1(), u_hi, WINDOW_SNAP);
    let w0 = snap_to(sweep.v0(), v_lo, WINDOW_SNAP);
    let w1 = snap_to(sweep.v1(), v_hi, WINDOW_SNAP);
    if !(s0 < s1 && w0 < w1) {
        return None;
    }
    if s0 < sweep.s0() - WINDOW_SNAP
        || s1 > sweep.s1() + WINDOW_SNAP
        || w0 < sweep.v0() - WINDOW_SNAP
        || w1 > sweep.v1() + WINDOW_SNAP
    {
        // The trimmed region escapes the parent window: not a windowed piece.
        return None;
    }

    let windowed = SpineFrameSweep::try_new(sweep.recipe().clone(), s0, s1, w0, w1).ok()?;
    let surface = Surface::SpineFrameSurface(windowed);
    let raw = face.absolute_boundaries().clone();
    let mut emitted = Face::try_new(raw, surface).ok()?;
    if !face.orientation() {
        emitted.invert();
    }
    Some(emitted)
}

/// Snaps `x` to `endpoint` when they agree within `eps`, else returns `x`.
fn snap_to(endpoint: f64, x: f64, eps: f64) -> f64 {
    if (x - endpoint).abs() <= eps {
        endpoint
    } else {
        x
    }
}

/// The BIE-006 arrangement-cell classification: one inside/out bit per
/// fragment, seeded from the BIE-005 arrangement-cell containment semantics.
///
/// The bit of a fragment is the containment of its `(s, v)` region
/// representative (lifted to its face carrier) in the OTHER solid's closure.
/// The seeds are then verified against the parity graph: every `Same`
/// adjacency must join equal bits and every `Flip` adjacency must join
/// different bits, exactly as the landed classifier verifies. The classifier's
/// seed-and-propagate logic is unchanged — this is the sweep path's seed
/// source, applied when a sweep-carrier shell makes the landed
/// canonical-carrier ray seed inapplicable.
///
/// Containment is decided through the convex plane-cell oracle
/// [`shell_contains_point`]; a shell with a non-planar (curved) face, or a
/// region whose representative cannot be resolved, refuses typed
/// `NumericallyUnresolved`.
pub(crate) fn classify_from_cells(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    mesh: &FragmentMesh,
    tol: f64,
) -> Outcome<FragmentClassification> {
    let mut bits: Vec<bool> = Vec::with_capacity(mesh.fragments.len());
    for fragment in &mesh.fragments {
        let face = &fragment.face;
        let p = fragment_seed_point(face, tol).ok_or_else(numerically_unresolved)?;
        let other = match fragment.origin {
            FragmentOrigin::A { .. } => shell_b,
            FragmentOrigin::B { .. } => shell_a,
        };
        let bit = shell_contains_point(other, p, tol).ok_or_else(numerically_unresolved)?;
        bits.push(bit);
    }

    // Parity verification, unchanged semantics: a `Same` shared edge joins
    // equal bits, a `Flip` (contact-introduced) shared edge joins different
    // bits. The first violation in `mesh.adjacency` order refuses.
    for adj in &mesh.adjacency {
        let lhs = bits.get(adj.lhs).copied().unwrap_or(false);
        let rhs = bits.get(adj.rhs).copied().unwrap_or(false);
        let implied_rhs = lhs ^ (adj.parity == AdjacencyParity::Flip);
        if rhs != implied_rhs {
            return Err(Refusal::Contradictory(ContradictionWitness {
                prop: Prop::FragmentInsideOther,
                left: Truth::Unknown,
                right: Truth::Unknown,
            }));
        }
    }

    Ok(Certified::new(
        FragmentClassification { inside_other: bits },
        Certificate {
            props: PropMap::new(),
            method: Method::Float,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The wire parameter polygons of a face's absolute boundary wires, in wire
/// order.
fn face_parameter_polygons(
    face: &Face<Point3, Curve, Surface>,
    tol: f64,
) -> Option<Vec<PolylineCurve<Point2>>> {
    let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    let mut out = Vec::new();
    for wire in face.absolute_boundaries() {
        out.push(create_parameter_boundary(face, wire, &mut cache, tol)?);
    }
    Some(out)
}

/// The 3-D seed point of one fragment: the carrier point at its `(s, v)`
/// region representative.
///
/// The primary measurement inverts the fragment's boundary wires into the
/// face's `(s, v)` chart and takes the region representative. A sweep-carrier
/// face whose boundary inversion the parameter search cannot certify falls
/// back to the center of the carrier's own stored window — the arrangement
/// cell of an (as yet) uncut sweep face. The point is then tested for
/// containment in the other shell.
fn fragment_seed_point(face: &Face<Point3, Curve, Surface>, tol: f64) -> Option<Point3> {
    if let Some(polys) = face_parameter_polygons(face, tol) {
        if let Some(rep) = region_representative(&polys, tol) {
            return Some(face.surface().subs(rep.x, rep.y));
        }
    }
    let Surface::SpineFrameSurface(sweep) = face.surface() else {
        return None;
    };
    let s = 0.5 * (sweep.s0() + sweep.s1());
    let v = 0.5 * (sweep.v0() + sweep.v1());
    Some(sweep.subs(s, v))
}

/// The convex plane-cell containment oracle: whether `p` lies inside the
/// closure of `shell`, decided by the shell's faces as plane cells.
///
/// Every face is fitted to the plane of its boundary vertices; a face whose
/// vertices are not coplanar within `tol` (a genuinely curved face) refuses
/// `None`. For a convex single-shell solid the inside test is the conjunction
/// over the outward halfspaces. The outward direction of a face is taken from
/// the shell's vertex centroid, so the result does not depend on the face's
/// stored orientation flag. This is the restricted (prismatic) class the
/// junction fixtures exercise; the general curved-carrier containment is the
/// certified engine's domain, not this packet's.
fn shell_contains_point(
    shell: &Shell<Point3, Curve, Surface>,
    p: Point3,
    tol: f64,
) -> Option<bool> {
    let mut centroid = Vector3::new(0.0, 0.0, 0.0);
    let mut count = 0u64;
    for face in shell.face_iter() {
        for vertex in face.vertex_iter() {
            centroid += vertex.point().to_vec();
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    let centroid = Point3::from_vec(centroid / count as f64);

    let mut inside = true;
    for face in shell.face_iter() {
        let (origin, normal) = face_cell_plane(face, tol)?;
        // Point the cell normal away from the shell centroid (outward).
        let outward = if normal.dot(centroid - origin) > 0.0 {
            -normal
        } else {
            normal
        };
        if outward.dot(p - origin) > tol {
            inside = false;
        }
    }
    Some(inside)
}

/// The plane cell of one face: an origin on the face and a unit normal, from
/// its boundary vertices. `None` when the face has no usable non-collinear
/// triple or its vertices are not coplanar within `tol`.
fn face_cell_plane(face: &Face<Point3, Curve, Surface>, tol: f64) -> Option<(Point3, Vector3)> {
    let mut vertices: Vec<Point3> = Vec::new();
    for vertex in face.vertex_iter() {
        let point = vertex.point();
        if vertices
            .iter()
            .all(|v: &Point3| (v - point).magnitude() > tol)
        {
            vertices.push(point);
        }
    }
    if vertices.len() < 3 {
        return None;
    }
    let origin = vertices.first().copied()?;
    let mut normal = None;
    for i in 1..vertices.len() {
        for j in (i + 1)..vertices.len() {
            let a = vertices.get(i).copied()?;
            let b = vertices.get(j).copied()?;
            let candidate = (a - origin).cross(b - origin);
            let magnitude = candidate.magnitude();
            if magnitude > tol {
                normal = Some(candidate / magnitude);
                break;
            }
        }
        if normal.is_some() {
            break;
        }
    }
    let normal = normal?;
    for vertex in &vertices {
        if (vertex - origin).dot(normal).abs() > tol {
            return None;
        }
    }
    Some((origin, normal))
}

/// One provenance row of a kept output fragment (BIE-006 decision 4): the
/// fragment's entity id and the `Op` row that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentProvenance {
    /// The output fragment entity.
    pub entity: EntityId,
    /// The operation row: the boolean verb over the input strata.
    pub op: Op,
}

/// Builds the provenance row of one kept output fragment.
///
/// The row's `Op` cites the boolean operation, and the entity's `Op` inputs
/// are the input strata this fragment derives from (its own parent face and
/// the other solid's parent it was classified against), exactly the §8.3
/// propagation rule: each output fragment's `Op` cites the input strata and
/// the boolean op.
pub(crate) fn fragment_provenance(
    origin: FragmentOrigin,
    other_parent: usize,
    op: BoolOp,
) -> FragmentProvenance {
    // The boolean op and the other parent are construction params, so two
    // identical decisions produce identical rows (determinism, spine §8).
    let op_index = match op {
        BoolOp::Union => 0u32,
        BoolOp::Intersection => 1u32,
        BoolOp::Difference => 2u32,
        BoolOp::Xor => 3u32,
    };
    let operation = Op {
        kind: OpKind::Boolean,
        params: OpParams::List(vec![
            OpParams::Index(op_index),
            OpParams::Index(other_parent as u32),
        ]),
    };
    let self_id = match origin {
        FragmentOrigin::A { parent } => EntityId::src(parent as u64),
        FragmentOrigin::B { parent } => EntityId::src(parent as u64),
    };
    let other_id = EntityId::src(other_parent as u64);
    let entity = operation.output(&[self_id, other_id], 0);
    FragmentProvenance {
        entity,
        op: operation,
    }
}

/// The numerically-unresolved refusal for a seed that cannot be certified.
fn numerically_unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect/panic on paths reachable from
// untrusted geometry. Unit-test assertions on hand-built witnesses are not
// such a path; the unwraps and indexing below cannot fire for the values
// constructed.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn provenance_records_sweep_fragments() {
        // A sweep fragment's row cites the input strata (its parent face and
        // the other solid's parent it was classified against) and the boolean
        // op (BIE-006 §8.3).
        let row = fragment_provenance(FragmentOrigin::A { parent: 2 }, 5, BoolOp::Difference);
        assert_eq!(row.op.kind, OpKind::Boolean);
        assert_eq!(
            row.op.params,
            OpParams::List(vec![OpParams::Index(2), OpParams::Index(5)])
        );
        let EntityId::Op { op, inputs, slot } = &row.entity else {
            unreachable!("the output entity is an operation entity");
        };
        assert_eq!(*slot, 0);
        assert_eq!(*op, row.op.id(), "the entity names this row's operation");
        assert_eq!(inputs.len(), 2);
        assert_eq!(*inputs.first().unwrap(), EntityId::src(2));
        assert_eq!(*inputs.last().unwrap(), EntityId::src(5));

        // Determinism: an identical decision yields an identical row.
        let again = fragment_provenance(FragmentOrigin::A { parent: 2 }, 5, BoolOp::Difference);
        assert_eq!(row.entity, again.entity);
        assert_eq!(row.op, again.op);

        // A distinct decision (other parent / op) yields a distinct row.
        let distinct = fragment_provenance(FragmentOrigin::B { parent: 0 }, 3, BoolOp::Union);
        assert_ne!(row.entity, distinct.entity);
        assert_ne!(row.op, distinct.op);
    }
}
