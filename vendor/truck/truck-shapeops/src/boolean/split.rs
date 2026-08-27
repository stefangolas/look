//! BG-SOL-RW2-SPLIT — the fragment splitter (the Boundary Rewrite's first
//! topology packet).
//!
//! Every face a [`ContactEvent`] touches is split along the certified contact
//! loci, reusing the old transversal wire-mutation pattern (`add_edge` /
//! `add_independent_loop` / `cut_with_parameter` / `swap_edge_into_wire`, all
//! rebuilt here against the certified records — transversal is rewritten, not
//! extended). The split edges are SHARED INSTANCES between the two solids'
//! fragment wires: edge identity (the shared `Arc` curve) is what lets the
//! assembled shell close in RW4.
//!
//! The per-arm semantics (plan §4 Phase 4, session 36):
//!
//! - FF Transverse `Analytic(Curve | TwoCurves)` — the exact curve is inserted
//!   into BOTH named faces as shared edge instances. `Line` is open, a full
//!   period `Circle`/`Ellipse` is closed. A closed curve strictly inside a
//!   face's region enters as the doubled independent loop; a curve crossing
//!   the face's boundary is clipped at the two extreme crossings certified by
//!   `Point` events; `Parabola`/`Hyperbola` refuse.
//! - `Point` loci (Transverse) — cut the named edges at the parameter
//!   projection of the certified point.
//! - FE `BoundedCurve` (CoincidentInterval) — the sewing oracle: an edge of
//!   one solid lying on a face of the other is REUSED (cut to the arc's
//!   extent) when the face's split produces a boundary along its carrier.
//! - Region2 `Coincident` — the containment screen between the two faces; a
//!   containing face is split along the contained face's boundary wires and a
//!   `CoincidentPair` is emitted.
//! - `ValidatedBranchCover` loci and `Tangency`-kind records refuse
//!   (`ContactReductionDeferred`, the RW-ARC-CONT / RW-TANGENT follow-ups).
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

use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;
use rustc_hash::FxHashSet as HashSet;
use std::f64::consts::TAU;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Matrix4, Point2, Point3, Vector2, Vector3};
use truck_base::contact::{ContactDimension, ContactEventKind};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, PropMap,
    UnresolvedWitness,
};
use truck_evidence::analytic::{AnalyticIntersection, ExactCurve};
use truck_evidence::contact::{ContactLocus, ContactRecord};
use truck_evidence::{Outcome, Refusal};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Line, UnitCircle};
use truck_geotrait::{
    BoundedCurve, Cut, Invertible, ParameterDivision1D, ParametricCurve, ParametricSurface,
    SearchParameter,
};
use truck_meshalgo::prelude::PolylineCurve;
use truck_topology::EdgeID;
use truck_topology::{Edge, Face, Shell, Vertex, Wire};

/// The number of Newton trials for a curve `search_parameter` call in the
/// insertion geometry (tolerance-class).
const SEARCH_TRIALS: usize = 10;

/// The number of Newton trials for a surface `search_parameter` call.
const SURFACE_SEARCH_TRIALS: usize = 100;

/// Dimensionless slack on parameter comparisons (the full-period decision and
/// the clip guards); a parameter inside this of a range endpoint counts as the
/// endpoint.
const PARAM_SLACK: f64 = 1.0e-9; // H-3: dimensionless parameter slack, not a length

/// Which solid a stratum reference belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SolidRef {
    /// The first shell.
    A,
    /// The second shell.
    B,
}

/// Where a contact event's record came from. Faces index
/// `shell.face_iter()` order; an edge names its position in
/// `face.absolute_boundaries()` flattened wire-by-wire, edge-by-edge, in
/// order. Edge identity is resolved by `EdgeID` (the same instance
/// appears in adjacent faces).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StratumRef {
    /// A face of one solid.
    Face {
        /// Which solid.
        solid: SolidRef,
        /// The face index in `solid`'s shell.
        index: usize,
    },
    /// An edge of one solid.
    Edge {
        /// Which solid.
        solid: SolidRef,
        /// The face whose boundary carries the edge.
        face: usize,
        /// The edge's flat position in `face.absolute_boundaries()`.
        edge: usize,
    },
}

/// One contact record with the provenance the splitter needs.
#[derive(Clone, Debug)]
pub struct ContactEvent {
    /// The certified contact record.
    pub record: ContactRecord,
    /// The first stratum the record came from.
    pub lhs: StratumRef,
    /// The second stratum the record came from.
    pub rhs: StratumRef,
}

/// Which parent face a fragment came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentOrigin {
    /// A fragment of solid A's parent face.
    A {
        /// The parent face index in shell A.
        parent: usize,
    },
    /// A fragment of solid B's parent face.
    B {
        /// The parent face index in shell B.
        parent: usize,
    },
}

/// One fragment of a split face.
#[derive(Clone, Debug)]
pub struct Fragment {
    /// The fragment face, carrying the split boundary wires.
    pub face: Face<Point3, Curve, Surface>,
    /// The parent face this fragment came from.
    pub origin: FragmentOrigin,
}

/// The parity of a shared edge between two fragments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdjacencyParity {
    /// The shared edge is (a sub-edge of) one solid's original edge.
    Same,
    /// The shared edge is a contact-introduced arc (the boundary of the other
    /// solid's material within this face's carrier).
    Flip,
}

/// One adjacency entry between two fragments of the SAME solid, per shared
/// edge instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FragmentAdjacency {
    /// The first fragment index in `FragmentMesh::fragments`.
    pub lhs: usize,
    /// The second fragment index in `FragmentMesh::fragments`.
    pub rhs: usize,
    /// The shared edge's parity.
    pub parity: AdjacencyParity,
}

/// The relative orientation of a coincident fragment pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoincidentOrientation {
    /// The two faces' absolute normals agree.
    Identical,
    /// The two faces' absolute normals oppose.
    Anti,
}

/// A cross-solid coincident fragment pair (the seam of the assembled shell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoincidentPair {
    /// The containing solid's fragment covering the overlap.
    pub a: usize,
    /// The contained solid's fragment covering the overlap.
    pub b: usize,
    /// The absolute-normal orientation of the pair.
    pub orientation: CoincidentOrientation,
}

/// The output of the splitter.
#[derive(Clone, Debug)]
pub struct FragmentMesh {
    /// All fragments, in shell-A-face order then shell-B-face order.
    pub fragments: Vec<Fragment>,
    /// Same-solid adjacency entries, one per shared edge instance.
    pub adjacency: Vec<FragmentAdjacency>,
    /// Cross-solid coincident pairs from Region2 containment splits.
    pub coincident: Vec<CoincidentPair>,
}

/// Split both shells along the contact events.
///
/// Returns the [`FragmentMesh`]: every touched face subdivided along the
/// certified loci (with the split edges shared between the two shells'
/// fragment wires), every other face a single fragment, the same-solid
/// adjacencies and the cross-solid coincident pairs. Any arm the v1 envelope
/// does not implement refuses the whole call with
/// `UnsupportedEnvelope(ContactReductionDeferred)` (or a typed
/// `NumericallyUnresolved` for an unstable insertion projection).
pub fn split_fragments(
    shell_a: &Shell<Point3, Curve, Surface>,
    shell_b: &Shell<Point3, Curve, Surface>,
    events: &[ContactEvent],
    tol: f64,
) -> Outcome<FragmentMesh> {
    let mut engine = SplitEngine::new(shell_a, shell_b, tol);
    engine.collect_events(events)?;
    engine.run(events)?;
    engine.finish()
}

// ---------------------------------------------------------------------------
// internal machinery
// ---------------------------------------------------------------------------

/// The geometry of one sewing-oracle entry: an edge of one solid lying on a
/// face of the other, with the certified sub-range on the edge's curve.
#[derive(Clone, Debug)]
struct SewArc {
    /// The exact carrier of the coincident sub-arc.
    curve: ExactCurve,
    /// The certified parameter range on the curve.
    t_range: (f64, f64),
    /// The topology edge to reuse.
    edge: Edge<Point3, Curve>,
}

/// A pending Region2 coincident pair, resolved to fragment indices after the
/// division step.
#[derive(Clone, Copy, Debug)]
struct PendingCoincident {
    /// The containing solid.
    a_solid: SolidRef,
    /// The containing face index.
    a_face: usize,
    /// The contained solid.
    b_solid: SolidRef,
    /// The contained face index.
    b_face: usize,
    /// The absolute-normal orientation.
    orientation: CoincidentOrientation,
}

/// A canonical vertex: one instance per certified boundary point, shared
/// across every wire that touches that point.
#[derive(Clone, Debug)]
struct CanonicalVertex {
    /// The certified point.
    point: Point3,
    /// The canonical vertex instance.
    vertex: Vertex<Point3>,
}

/// A face and the certified crossing points of one FF curve's trace on it.
type CrossingFace = (SolidRef, usize, Vec<(f64, Point3)>);

/// The mutable boundary-wire collection of one face (the rebuilt `Loops`).
#[derive(Clone, Debug, Default)]
struct Loops(Vec<Wire<Point3, Curve>>);

impl Loops {
    /// Replaces every wire edge with id `edge_id` by the two halves
    /// `new_wire` (orientation-adjusted), across this face's wires.
    fn swap_edge_into_wire(&mut self, edge_id: EdgeID<Curve>, new_wire: &Wire<Point3, Curve>) {
        for wire in self.0.iter_mut() {
            let mut iter = wire.iter().enumerate();
            if let Some((idx, edge)) = iter.find(|(_, edge)| edge.id() == edge_id) {
                let replacement = if edge.orientation() {
                    new_wire.clone()
                } else {
                    new_wire.inverse()
                };
                let _ = wire.swap_edge_into_wire(idx, replacement);
            }
        }
    }

    /// The wire index whose `front()` vertex equals `vertex`.
    fn find_wire_with_front(&self, vertex: &Vertex<Point3>) -> Option<(usize, usize)> {
        self.0.iter().enumerate().find_map(|(wi, wire)| {
            wire.iter().enumerate().find_map(|(ei, edge)| {
                if edge.front() == vertex {
                    Some((wi, ei))
                } else {
                    None
                }
            })
        })
    }

    /// Splices the open arc `edge` into the face's boundary wires (the old
    /// `add_edge` pattern): find the wires whose boundary touches the arc's
    /// endpoints, rotate the touching wire so the arc enters at the gap, and
    /// split or merge wires so the arc is the shared boundary of the two new
    /// regions.
    fn add_edge(&mut self, edge: Edge<Point3, Curve>) -> Result<(), Refusal> {
        let a = self.find_wire_with_front(edge.back());
        let b = self.find_wire_with_front(edge.front());
        if let Some((wire_index0, edge_index0)) = a {
            let wire = self.0.get_mut(wire_index0).ok_or_else(unsupported)?;
            wire.rotate_left(edge_index0);
            wire.push_front(edge.clone());
            wire.push_back(edge.inverse());
        }
        match (a, b) {
            (Some((wire_index0, edge_index0)), Some((wire_index1, edge_index1))) => {
                if wire_index0 == wire_index1 {
                    let wire = self.0.get_mut(wire_index0).ok_or_else(unsupported)?;
                    let len = wire.len() - 2;
                    let edge_index1 = (len + edge_index1 - edge_index0) % len + 1;
                    let new_wire = wire.split_off(edge_index1);
                    self.0.push(new_wire);
                } else {
                    let mut new_wire0 = self.0.get(wire_index1).ok_or_else(unsupported)?.clone();
                    let mut new_wire1 = new_wire0.split_off(edge_index1);
                    let wire0 = self.0.get_mut(wire_index0).ok_or_else(unsupported)?;
                    let mut taken = std::mem::take(wire0);
                    new_wire0.append(&mut taken);
                    new_wire0.append(&mut new_wire1);
                    *wire0 = new_wire0;
                    self.0.swap_remove(wire_index1);
                }
            }
            (None, Some((wire_index1, edge_index1))) => {
                let wire = self.0.get_mut(wire_index1).ok_or_else(unsupported)?;
                wire.rotate_left(edge_index1);
                wire.push_front(edge.inverse());
                wire.push_back(edge);
            }
            (None, None) => self.0.push(Wire::from(vec![edge.inverse(), edge])),
            (Some(_), None) => {}
        }
        Ok(())
    }

    /// Adds the closed `wire` (and its inverse) as an independent pair of
    /// loops, so `divide_one_face` produces a bounded region and the
    /// containing region with the hole.
    fn add_independent_loop(&mut self, wire: &Wire<Point3, Curve>) {
        self.0.push(wire.inverse());
        self.0.push(wire.clone());
    }
}

/// The phase dispatch discriminates which arm family a pass processes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// `Point`-locus cuts.
    Point,
    /// FF transverse arcs.
    Ff,
    /// Region2 containment splits.
    Region2,
}

/// How a face's region relates to a curve's parameter trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CurveFaceRelation {
    /// The whole trace is strictly inside the face's region.
    Inside,
    /// The trace crosses the face's boundary.
    Crossing,
    /// The whole trace lies on the face's boundary edges.
    OnBoundary,
    /// The whole trace is outside the face's region.
    Outside,
}

/// The Region2 containment screen outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RegionScreen {
    /// The two regions are disjoint.
    Disjoint,
    /// The two regions partially overlap.
    PartialOverlap,
    /// The faces' boundary wires cross.
    Crossing,
    /// The first region contains the second.
    AContainsB,
    /// The second region contains the first.
    BContainsA,
}

/// The whole split state: both shells' mutable loop stores plus the shared
/// bookkeeping the certified records drive.
struct SplitEngine<'a> {
    /// Shell A.
    shell_a: &'a Shell<Point3, Curve, Surface>,
    /// Shell B.
    shell_b: &'a Shell<Point3, Curve, Surface>,
    /// The mutable boundary wires of shell A, one `Loops` per face.
    loops_a: Vec<Loops>,
    /// The mutable boundary wires of shell B, one `Loops` per face.
    loops_b: Vec<Loops>,
    /// The tolerance class for insertion geometry.
    tol: f64,
    /// The sewing-oracle records, keyed by the face they touch.
    sew: HashMap<(SolidRef, usize), Vec<SewArc>>,
    /// The certified point-event locus points, for FF crossing certification.
    certified_points: Vec<Point3>,
    /// One canonical vertex per certified boundary point.
    canonical_vertices: Vec<CanonicalVertex>,
    /// Faces of A that received a contact wire (need division).
    touched_a: HashSet<usize>,
    /// Faces of B that received a contact wire (need division).
    touched_b: HashSet<usize>,
    /// Edge ids that are contact arcs in A.
    contact_a: HashSet<EdgeID<Curve>>,
    /// Edge ids that are contact arcs in B.
    contact_b: HashSet<EdgeID<Curve>>,
    /// Pending Region2 coincident pairs, resolved after division.
    pending_coincident: Vec<PendingCoincident>,
}

impl<'a> SplitEngine<'a> {
    /// Builds the engine from the two shells.
    fn new(
        shell_a: &'a Shell<Point3, Curve, Surface>,
        shell_b: &'a Shell<Point3, Curve, Surface>,
        tol: f64,
    ) -> Self {
        let loops_a = shell_a
            .face_iter()
            .map(|face| Loops(face.absolute_boundaries().to_vec()))
            .collect();
        let loops_b = shell_b
            .face_iter()
            .map(|face| Loops(face.absolute_boundaries().to_vec()))
            .collect();
        Self {
            shell_a,
            shell_b,
            loops_a,
            loops_b,
            tol,
            sew: HashMap::default(),
            certified_points: Vec::new(),
            canonical_vertices: Vec::new(),
            touched_a: HashSet::default(),
            touched_b: HashSet::default(),
            contact_a: HashSet::default(),
            contact_b: HashSet::default(),
            pending_coincident: Vec::new(),
        }
    }

    // -- shell / store access -------------------------------------------------

    /// The shell of a solid.
    fn shell(&self, solid: SolidRef) -> &Shell<Point3, Curve, Surface> {
        match solid {
            SolidRef::A => self.shell_a,
            SolidRef::B => self.shell_b,
        }
    }

    /// A face of a solid by index.
    fn shell_face(&self, solid: SolidRef, idx: usize) -> Option<&Face<Point3, Curve, Surface>> {
        self.shell(solid).get(idx)
    }

    /// The mutable loops of a face.
    fn mut_loops(&mut self, solid: SolidRef, idx: usize) -> Result<&mut Loops, Refusal> {
        let list = match solid {
            SolidRef::A => &mut self.loops_a,
            SolidRef::B => &mut self.loops_b,
        };
        list.get_mut(idx).ok_or_else(unsupported)
    }

    /// The contact-edge id set of a solid.
    fn contact_set(&self, solid: SolidRef) -> &HashSet<EdgeID<Curve>> {
        match solid {
            SolidRef::A => &self.contact_a,
            SolidRef::B => &self.contact_b,
        }
    }

    /// The mutable contact-edge id set of a solid.
    fn contact_set_mut(&mut self, solid: SolidRef) -> &mut HashSet<EdgeID<Curve>> {
        match solid {
            SolidRef::A => &mut self.contact_a,
            SolidRef::B => &mut self.contact_b,
        }
    }

    /// The mutable touched-face set of a solid.
    fn touched_mut(&mut self, solid: SolidRef) -> &mut HashSet<usize> {
        match solid {
            SolidRef::A => &mut self.touched_a,
            SolidRef::B => &mut self.touched_b,
        }
    }

    /// Whether a face received a contact wire.
    fn is_touched(&self, solid: SolidRef, idx: usize) -> bool {
        match solid {
            SolidRef::A => self.touched_a.contains(&idx),
            SolidRef::B => self.touched_b.contains(&idx),
        }
    }

    /// Replaces every wire edge with id `edge_id` by `new_wire` in BOTH
    /// shells' stores.
    fn swap_edge_into_wire(&mut self, edge_id: EdgeID<Curve>, new_wire: &Wire<Point3, Curve>) {
        for loops in self.loops_a.iter_mut() {
            loops.swap_edge_into_wire(edge_id, new_wire);
        }
        for loops in self.loops_b.iter_mut() {
            loops.swap_edge_into_wire(edge_id, new_wire);
        }
    }

    /// After cutting edge `id` into the halves of `new_wire`, migrates the
    /// contact flags of `id` onto the halves.
    fn migrate_contact_on_cut(&mut self, id: EdgeID<Curve>, new_wire: &Wire<Point3, Curve>) {
        let a_was = self.contact_a.remove(&id);
        let b_was = self.contact_b.remove(&id);
        if a_was || b_was {
            for edge in new_wire.edge_iter() {
                if a_was {
                    self.contact_a.insert(edge.id());
                }
                if b_was {
                    self.contact_b.insert(edge.id());
                }
            }
        }
    }

    /// The unique face indices an event touches, deduplicated.
    fn event_faces(&self, ev: &ContactEvent) -> Vec<(SolidRef, usize)> {
        let mut out: Vec<(SolidRef, usize)> = Vec::new();
        for stratum in [ev.lhs, ev.rhs] {
            let (solid, face) = match stratum {
                StratumRef::Face { solid, index } => (solid, index),
                StratumRef::Edge { solid, face, .. } => (solid, face),
            };
            if !out.contains(&(solid, face)) {
                out.push((solid, face));
            }
        }
        out
    }

    /// The topology edge named by a stratum reference.
    fn edge_from_ref(&self, r: &StratumRef) -> Result<Edge<Point3, Curve>, Refusal> {
        let StratumRef::Edge { solid, face, edge } = r else {
            return Err(refused());
        };
        let shell = self.shell(*solid);
        let face = shell.get(*face).ok_or_else(unsupported)?;
        let boundaries = face.absolute_boundaries();
        let mut flat = 0usize;
        for wire in boundaries {
            let n = wire.len();
            if *edge < flat + n {
                return wire.get(*edge - flat).cloned().ok_or_else(unsupported);
            }
            flat += n;
        }
        Err(refused())
    }

    /// The canonical vertex for a certified point, creating it on first use.
    fn canonical_vertex(&mut self, point: Point3) -> Vertex<Point3> {
        for cv in &self.canonical_vertices {
            if near_pt(cv.point, point, self.tol) {
                return cv.vertex.clone();
            }
        }
        let vertex = Vertex::new(point);
        self.canonical_vertices.push(CanonicalVertex {
            point,
            vertex: vertex.clone(),
        });
        vertex
    }

    // -- event collection -----------------------------------------------------

    /// Collects the sewing-oracle records and the certified point-event
    /// points, and validates the stratum references.
    fn collect_events(&mut self, events: &[ContactEvent]) -> Result<(), Refusal> {
        for ev in events {
            let rec = &ev.record;
            if matches!(rec.kind, ContactEventKind::Transverse) {
                if let ContactLocus::Point(p) = rec.locus {
                    self.certified_points.push(p);
                }
            }
            if let (
                ContactDimension::Arc1,
                ContactEventKind::CoincidentInterval,
                ContactLocus::BoundedCurve { .. },
            ) = (&rec.dimension, &rec.kind, &rec.locus)
            {
                self.collect_sew(ev)?;
            }
            for (solid, face) in self.event_faces(ev) {
                let _ = self.shell_face(solid, face).ok_or_else(unsupported)?;
            }
        }
        Ok(())
    }

    /// Registers one FE `BoundedCurve` record into the sewing oracle.
    fn collect_sew(&mut self, ev: &ContactEvent) -> Result<(), Refusal> {
        let ContactLocus::BoundedCurve { curve, t_range } = &ev.record.locus else {
            return Err(refused());
        };
        // The identity predicate covers Line and Circle; other carriers refuse.
        match curve {
            ExactCurve::Line(_) | ExactCurve::Circle(_) | ExactCurve::Ellipse(_) => {}
            ExactCurve::Parabola(_) | ExactCurve::Hyperbola(_) => return Err(refused()),
        }
        let (face_side, edge_side) = match (ev.lhs, ev.rhs) {
            (StratumRef::Face { .. }, StratumRef::Edge { .. }) => (ev.lhs, ev.rhs),
            (StratumRef::Edge { .. }, StratumRef::Face { .. }) => (ev.rhs, ev.lhs),
            _ => return Err(refused()),
        };
        let (solid, face_idx) = match face_side {
            StratumRef::Face { solid, index } => (solid, index),
            _ => return Err(refused()),
        };
        let edge = self.edge_from_ref(&edge_side)?;
        let key = (solid, face_idx);
        self.sew.entry(key).or_default().push(SewArc {
            curve: curve.clone(),
            t_range: *t_range,
            edge,
        });
        Ok(())
    }

    // -- the phase passes -----------------------------------------------------

    /// Runs the three passes: point cuts, FF arcs, Region2 splits.
    fn run(&mut self, events: &[ContactEvent]) -> Result<(), Refusal> {
        for ev in events {
            self.dispatch(ev, Phase::Point)?;
        }
        for ev in events {
            self.dispatch(ev, Phase::Ff)?;
        }
        for ev in events {
            self.dispatch(ev, Phase::Region2)?;
        }
        Ok(())
    }

    /// Dispatches one event. Every `ContactLocus` arm is enumerated (no `_`
    /// arm), so rustc enforces that a future locus arm cannot be silently
    /// dropped.
    fn dispatch(&mut self, ev: &ContactEvent, phase: Phase) -> Result<(), Refusal> {
        let rec = &ev.record;
        match (&rec.locus, &rec.kind, &rec.dimension) {
            (
                ContactLocus::Analytic(AnalyticIntersection::Curve(exact)),
                ContactEventKind::Transverse,
                ContactDimension::Arc1,
            ) => {
                if phase == Phase::Ff {
                    self.ff_curve(ev, exact)
                } else {
                    Ok(())
                }
            }
            (
                ContactLocus::Analytic(AnalyticIntersection::TwoCurves([c0, c1])),
                ContactEventKind::Transverse,
                ContactDimension::Arc1,
            ) => {
                if phase == Phase::Ff {
                    self.ff_curve(ev, c0)?;
                    self.ff_curve(ev, c1)
                } else {
                    Ok(())
                }
            }
            (ContactLocus::Analytic(AnalyticIntersection::Curve(_)), _, _) => Err(refused()),
            (ContactLocus::Analytic(AnalyticIntersection::TwoCurves(_)), _, _) => Err(refused()),
            (ContactLocus::Analytic(AnalyticIntersection::TangentPoint(_)), _, _) => Err(refused()),
            (ContactLocus::Analytic(AnalyticIntersection::TangentLine(_)), _, _) => Err(refused()),
            (ContactLocus::Analytic(AnalyticIntersection::TangentCircle(_)), _, _) => {
                Err(refused())
            }
            (ContactLocus::Analytic(AnalyticIntersection::Parallel), _, _) => Ok(()),
            (ContactLocus::Analytic(AnalyticIntersection::Empty), _, _) => Ok(()),
            (
                ContactLocus::Analytic(AnalyticIntersection::Coincident),
                _,
                ContactDimension::Region2,
            ) => {
                if phase == Phase::Region2 {
                    self.region2(ev)
                } else {
                    Ok(())
                }
            }
            (ContactLocus::Analytic(AnalyticIntersection::Coincident), _, _) => Err(refused()),
            (ContactLocus::Coincident, _, ContactDimension::Region2) => {
                if phase == Phase::Region2 {
                    self.region2(ev)
                } else {
                    Ok(())
                }
            }
            (ContactLocus::Coincident, _, _) => Err(refused()),
            (ContactLocus::Point(p), ContactEventKind::Transverse, ContactDimension::Point0) => {
                if phase == Phase::Point {
                    self.point_cut(ev, *p)
                } else {
                    Ok(())
                }
            }
            (ContactLocus::Point(_), ContactEventKind::EndpointTouch, _) => Err(refused()),
            (ContactLocus::Point(_), _, _) => Err(refused()),
            (
                ContactLocus::BoundedCurve { .. },
                ContactEventKind::CoincidentInterval,
                ContactDimension::Arc1,
            ) => Ok(()),
            (ContactLocus::BoundedCurve { .. }, _, _) => Err(refused()),
            (ContactLocus::ValidatedBranchCover(_), _, _) => Err(refused()),
        }
    }

    // -- Point events ---------------------------------------------------------

    /// Cuts the face's boundary edges at the certified point `p`.
    fn point_cut(&mut self, ev: &ContactEvent, p: Point3) -> Result<(), Refusal> {
        let (solid, face_idx) = self
            .event_faces(ev)
            .first()
            .copied()
            .ok_or_else(unsupported)?;
        self.cut_edge_at_point(solid, face_idx, p)
    }

    /// Cuts the edge of `(solid, face_idx)` whose curve passes through `p` at
    /// an interior parameter, using the canonical vertex for `p`.
    fn cut_edge_at_point(
        &mut self,
        solid: SolidRef,
        face_idx: usize,
        p: Point3,
    ) -> Result<(), Refusal> {
        let vertex = self.canonical_vertex(p);
        let loops = self.mut_loops(solid, face_idx)?;
        let mut found: Option<(usize, usize, f64)> = None;
        for (wi, wire) in loops.0.iter().enumerate() {
            for (ei, edge) in wire.iter().enumerate() {
                let curve = edge.curve();
                if let Some(t) = curve.search_parameter(p, None, SEARCH_TRIALS) {
                    let (t0, t1) = curve.range_tuple();
                    if t > t0 + PARAM_SLACK && t < t1 - PARAM_SLACK {
                        found = Some((wi, ei, t));
                        break;
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        let Some((wi, ei, t)) = found else {
            // No interior edge passes through `p`: it is already a boundary
            // vertex of this face (or off-face); nothing to cut.
            return Ok(());
        };
        // Cut the ABSOLUTE clone (the loops_store pattern): `cut_with_parameter`
        // on the oriented use would hand the swap an orientation-folded wire
        // whose ends do not match the use's ends for inverted edges.
        let edge = loops
            .0
            .get(wi)
            .and_then(|wire| wire.get(ei))
            .ok_or_else(unsupported)?
            .absolute_clone();
        let (h0, h1) = edge
            .cut_with_parameter(&vertex, t)
            .ok_or_else(numerically_unresolved)?;
        let new_wire = Wire::from(vec![h0, h1]);
        let id = edge.id();
        self.swap_edge_into_wire(id, &new_wire);
        self.migrate_contact_on_cut(id, &new_wire);
        Ok(())
    }

    // -- FF transverse arcs ---------------------------------------------------

    /// Inserts one exact FF curve into both named faces' structures as shared
    /// edge instances.
    fn ff_curve(&mut self, ev: &ContactEvent, exact: &ExactCurve) -> Result<(), Refusal> {
        let curve = exact_to_curve(exact)?;
        let closed = is_full_period_circle(&curve);
        let faces = self.event_faces(ev);
        let mut inside_faces: Vec<(SolidRef, usize)> = Vec::new();
        let mut crossing_faces: Vec<CrossingFace> = Vec::new();
        for (solid, face_idx) in faces {
            let face = self.shell_face(solid, face_idx).ok_or_else(unsupported)?;
            let polys = self.face_parameter_polygons(solid, face_idx)?;
            let (relation, crossings) = self
                .classify_curve(face, &polys, &curve)
                .ok_or_else(numerically_unresolved)?;
            match relation {
                CurveFaceRelation::Inside => inside_faces.push((solid, face_idx)),
                CurveFaceRelation::Crossing => {
                    crossing_faces.push((solid, face_idx, crossings));
                }
                CurveFaceRelation::OnBoundary | CurveFaceRelation::Outside => {}
            }
        }
        if closed {
            if let Some((solid, face_idx)) = inside_faces.first().copied() {
                let wire = self.build_closed_loop_wire(solid, face_idx, &curve, exact)?;
                for (solid, face_idx) in &inside_faces {
                    self.add_doubled_loop(*solid, *face_idx, &wire)?;
                }
            }
            for (solid, face_idx, crossings) in crossing_faces {
                self.insert_clipped_arc(solid, face_idx, &curve, &crossings)?;
            }
        } else {
            // An open curve whose trace lies strictly inside a face's region
            // has no certified endpoints on that face's boundary: refuse.
            if !inside_faces.is_empty() {
                return Err(refused());
            }
            self.insert_open_arc_shared(&crossing_faces, &curve)?;
        }
        Ok(())
    }

    /// The parameter polygons of a face's ORIGINAL boundary wires, outer first
    /// then holes, in wire order.
    fn face_parameter_polygons(
        &self,
        solid: SolidRef,
        face_idx: usize,
    ) -> Result<Vec<PolylineCurve<Point2>>, Refusal> {
        let face = self.shell_face(solid, face_idx).ok_or_else(unsupported)?;
        let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
        let mut out = Vec::new();
        for wire in face.absolute_boundaries() {
            out.push(
                create_parameter_boundary(face, wire, &mut cache, self.tol)
                    .ok_or_else(numerically_unresolved)?,
            );
        }
        Ok(out)
    }

    /// The minimum number of samples used to classify a curve against a face's
    /// region. The analytic `parameter_division` of a line returns only its two
    /// endpoints, which would hide a boundary-to-boundary crossing; the
    /// classification always samples at least this many points.
    const CURVE_SAMPLES: usize = 24;

    /// Classifies a curve's parameter trace against a face's region and
    /// returns the trace's boundary crossings as `(curve parameter, 3-D
    /// point)` pairs.
    fn classify_curve(
        &self,
        face: &Face<Point3, Curve, Surface>,
        face_polys: &[PolylineCurve<Point2>],
        curve: &Curve,
    ) -> Option<(CurveFaceRelation, Vec<(f64, Point3)>)> {
        let (mut ts, mut pts) = curve.parameter_division(curve.range_tuple(), self.tol);
        if ts.len() < Self::CURVE_SAMPLES {
            let (t0, t1) = curve.range_tuple();
            ts = (0..Self::CURVE_SAMPLES)
                .map(|i| t0 + (t1 - t0) * (i as f64 / (Self::CURVE_SAMPLES - 1) as f64))
                .collect();
            pts = ts.iter().map(|t| curve.subs(*t)).collect();
        }
        let surface = face.surface();
        let u_period = surface.u_period();
        let mut prev: Option<Point2> = None;
        let mut mapped: Vec<(f64, Point2, Point3)> = Vec::new();
        for (t, pt) in ts.iter().zip(pts.iter()) {
            let p0: Point2 = prev.map_or(Point2::origin(), |p| p);
            let hint = prev.map(|p| p.into());
            let mut uv: Point2 = surface
                .search_parameter(*pt, hint, SURFACE_SEARCH_TRIALS)?
                .into();
            if let Some(period) = u_period {
                uv.x = unwrap_periodic_parameter(p0.x, uv.x, period);
                // The face's parameter polygons live on the principal branch
                // (unwrapped from the wire's front); fold the trace onto the same
                // branch so periodic coordinates compare in one frame.
                uv.x = uv.x.rem_euclid(period);
            }
            mapped.push((*t, uv, *pt));
            prev = Some(uv);
        }
        let mut all_on = true;
        let mut all_inside = true;
        let mut all_outside = true;
        let mut crossings: Vec<(f64, Point3)> = Vec::new();
        for (t, uv, pt) in &mapped {
            if self.on_face_boundary(face_polys, *uv, u_period) {
                all_inside = false;
                all_outside = false;
                crossings.push((*t, *pt));
            } else if region_contains(face_polys, *uv, u_period) {
                all_on = false;
                all_outside = false;
            } else {
                all_on = false;
                all_inside = false;
            }
        }
        // Inside<->outside transitions between consecutive samples are proper
        // boundary crossings; locate each by intersecting the sample segment
        // with the boundary.
        for pair in mapped.windows(2) {
            let (a, b) = (pair.first()?, pair.get(1)?);
            let (ta, aa, a3) = *a;
            let (tb, bb, b3) = *b;
            let in_a = region_contains(face_polys, aa, u_period);
            let in_b = region_contains(face_polys, bb, u_period);
            if in_a != in_b {
                if let Some(s) = segment_boundary_crossing(aa, bb, face_polys) {
                    let t = ta + s * (tb - ta);
                    let p3 = a3 + (b3 - a3) * s;
                    crossings.push((t, p3));
                }
            }
        }
        let relation = if all_on {
            CurveFaceRelation::OnBoundary
        } else if all_inside {
            CurveFaceRelation::Inside
        } else if all_outside {
            CurveFaceRelation::Outside
        } else {
            CurveFaceRelation::Crossing
        };
        Some((relation, crossings))
    }

    /// Whether `uv` lies within `tol` of one of the face's boundary polygon
    /// segments. When the face's surface is periodic in u (`u_period` is
    /// `Some(T)`), the test also runs at `uv.x ± T`: the boundary wires unwind
    /// from their own front vertex and can live on a different u-branch than
    /// the folded query point, so the ± period translates are what let a
    /// frame-mismatched query see the 3-D coincidence.
    fn on_face_boundary(
        &self,
        face_polys: &[PolylineCurve<Point2>],
        uv: Point2,
        u_period: Option<f64>,
    ) -> bool {
        let on = |q: Point2| {
            face_polys.iter().any(|poly| {
                poly.iter()
                    .circular_tuple_windows()
                    .any(|(a, b)| point_segment_distance(q, *a, *b) <= self.tol)
            })
        };
        if on(uv) {
            return true;
        }
        match u_period {
            Some(period) => {
                on(Point2::new(uv.x + period, uv.y)) || on(Point2::new(uv.x - period, uv.y))
            }
            None => false,
        }
    }

    /// Builds the closed loop wire for a closed FF curve inside a face's
    /// region: a cut of the sewing-oracle edge when its carrier matches, else
    /// fresh halves of the curve.
    fn build_closed_loop_wire(
        &mut self,
        solid: SolidRef,
        face_idx: usize,
        curve: &Curve,
        exact: &ExactCurve,
    ) -> Result<Wire<Point3, Curve>, Refusal> {
        if let Some((edge, range)) = self.sew_edge_for(solid, face_idx, exact) {
            let halves = self.cut_edge_to_arc(&edge, range).ok_or_else(unsupported)?;
            let wire = Wire::from(halves);
            self.swap_edge_into_wire(edge.id(), &wire);
            return Ok(wire);
        }
        create_independent_loop(curve.clone()).ok_or_else(unsupported)
    }

    /// Adds the closed `wire` as the doubled independent loop of a face and
    /// marks its edges as contact arcs.
    fn add_doubled_loop(
        &mut self,
        solid: SolidRef,
        face_idx: usize,
        wire: &Wire<Point3, Curve>,
    ) -> Result<(), Refusal> {
        {
            let loops = self.mut_loops(solid, face_idx)?;
            loops.add_independent_loop(wire);
        }
        for edge in wire.edge_iter() {
            self.contact_set_mut(solid).insert(edge.id());
        }
        self.touched_mut(solid).insert(face_idx);
        Ok(())
    }

    /// The sewing-oracle edge for a face whose split should reuse an existing
    /// edge along the carrier of `exact`.
    fn sew_edge_for(
        &self,
        solid: SolidRef,
        face_idx: usize,
        exact: &ExactCurve,
    ) -> Option<(Edge<Point3, Curve>, (f64, f64))> {
        let key = (solid, face_idx);
        let arcs = self.sew.get(&key)?;
        for arc in arcs {
            if exact_curves_identical(&arc.curve, exact, self.tol) {
                return Some((arc.edge.clone(), arc.t_range));
            }
        }
        None
    }

    /// Cuts `edge` to the certified arc extent `range`. A full-period circle
    /// is cut into its two half-edges (the doubled-loop form); a sub-range is
    /// cut into three pieces of which the middle is the arc.
    fn cut_edge_to_arc(
        &self,
        edge: &Edge<Point3, Curve>,
        range: (f64, f64),
    ) -> Option<Vec<Edge<Point3, Curve>>> {
        let curve = edge.curve();
        let (c0, c1) = curve.range_tuple();
        if (c1 - c0) >= TAU - PARAM_SLACK {
            let t = (c0 + c1) * 0.5;
            let vertex = Vertex::new(curve.subs(t));
            let (e0, e1) = edge.cut_with_parameter(&vertex, t)?;
            Some(vec![e0, e1])
        } else {
            let (r0, r1) = range;
            let v0 = Vertex::new(curve.subs(r0));
            let (e0, e1) = edge.cut_with_parameter(&v0, r0)?;
            let v1 = Vertex::new(curve.subs(r1));
            let (_, mid) = e1.cut_with_parameter(&v1, r1)?;
            let _ = e0;
            Some(vec![mid])
        }
    }

    /// Inserts a curve clipped to the certified extreme crossings into ONE
    /// face (the closed-curve-crosses-boundary path).
    fn insert_clipped_arc(
        &mut self,
        solid: SolidRef,
        face_idx: usize,
        curve: &Curve,
        crossings: &[(f64, Point3)],
    ) -> Result<(), Refusal> {
        let Some((t_min, t_max, p_min, p_max)) = self.certified_extremes(crossings) else {
            return Err(refused());
        };
        let v0 = self.canonical_vertex(p_min);
        let v1 = self.canonical_vertex(p_max);
        self.cut_edge_at_point(solid, face_idx, p_min)?;
        self.cut_edge_at_point(solid, face_idx, p_max)?;
        let sub = clip_curve(curve.clone(), t_min, t_max).ok_or_else(unsupported)?;
        let arc = Edge::try_new(&v0, &v1, sub).map_err(|_| refused())?;
        {
            let loops = self.mut_loops(solid, face_idx)?;
            loops.add_edge(arc.clone())?;
        }
        self.contact_set_mut(solid).insert(arc.id());
        self.touched_mut(solid).insert(face_idx);
        Ok(())
    }

    /// Inserts an open arc clipped to the certified extreme crossings as a
    /// SHARED instance across every crossing face.
    fn insert_open_arc_shared(
        &mut self,
        crossing_faces: &[CrossingFace],
        curve: &Curve,
    ) -> Result<(), Refusal> {
        // Union the certified crossings over all crossing faces.
        let mut certified: Vec<(f64, Point3)> = Vec::new();
        for (_, _, crossings) in crossing_faces {
            for (t, p) in crossings {
                if self
                    .certified_points
                    .iter()
                    .any(|c| near_pt(*c, *p, self.tol))
                {
                    certified.push((*t, *p));
                }
            }
        }
        if certified.is_empty() {
            return Err(refused());
        }
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;
        let mut p_min = Point3::origin();
        let mut p_max = Point3::origin();
        for (t, p) in certified {
            if t < t_min {
                t_min = t;
                p_min = p;
            }
            if t > t_max {
                t_max = t;
                p_max = p;
            }
        }
        let v0 = self.canonical_vertex(p_min);
        let v1 = self.canonical_vertex(p_max);
        for (solid, face_idx, _) in crossing_faces {
            self.cut_edge_at_point(*solid, *face_idx, p_min)?;
            self.cut_edge_at_point(*solid, *face_idx, p_max)?;
        }
        let sub = clip_curve(curve.clone(), t_min, t_max).ok_or_else(unsupported)?;
        let arc = Edge::try_new(&v0, &v1, sub).map_err(|_| refused())?;
        for (solid, face_idx, _) in crossing_faces {
            {
                let loops = self.mut_loops(*solid, *face_idx)?;
                loops.add_edge(arc.clone())?;
            }
            self.contact_set_mut(*solid).insert(arc.id());
            self.touched_mut(*solid).insert(*face_idx);
        }
        Ok(())
    }

    /// The certified extreme crossings of a curve against a face: the
    /// `(min, max)` certified crossing parameters and their points.
    fn certified_extremes(
        &self,
        crossings: &[(f64, Point3)],
    ) -> Option<(f64, f64, Point3, Point3)> {
        let mut found: Vec<(f64, Point3)> = Vec::new();
        for (t, p) in crossings {
            if self
                .certified_points
                .iter()
                .any(|c| near_pt(*c, *p, self.tol))
            {
                found.push((*t, *p));
            }
        }
        if found.is_empty() {
            return None;
        }
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;
        let mut p_min = Point3::origin();
        let mut p_max = Point3::origin();
        for (t, p) in found {
            if t < t_min {
                t_min = t;
                p_min = p;
            }
            if t > t_max {
                t_max = t;
                p_max = p;
            }
        }
        Some((t_min, t_max, p_min, p_max))
    }

    // -- Region2 coincident ---------------------------------------------------

    /// The Region2 containment screen between the two named faces.
    fn region2(&mut self, ev: &ContactEvent) -> Result<(), Refusal> {
        let faces = self.event_faces(ev);
        let (fa, fb) = match (faces.first(), faces.get(1)) {
            (Some(a), Some(b)) => (*a, *b),
            _ => return Err(refused()),
        };
        let polys_a = self.face_parameter_polygons(fa.0, fa.1)?;
        let polys_b = self.face_parameter_polygons(fb.0, fb.1)?;
        match containment_screen(&polys_a, &polys_b, self.tol) {
            RegionScreen::Disjoint => Ok(()),
            RegionScreen::PartialOverlap | RegionScreen::Crossing => Err(refused()),
            RegionScreen::AContainsB => {
                self.split_containing(fa, fb)?;
                let orientation = self.orientation_pair(fa, fb);
                self.pending_coincident.push(PendingCoincident {
                    a_solid: fa.0,
                    a_face: fa.1,
                    b_solid: fb.0,
                    b_face: fb.1,
                    orientation,
                });
                Ok(())
            }
            RegionScreen::BContainsA => {
                self.split_containing(fb, fa)?;
                let orientation = self.orientation_pair(fb, fa);
                self.pending_coincident.push(PendingCoincident {
                    a_solid: fb.0,
                    a_face: fb.1,
                    b_solid: fa.0,
                    b_face: fa.1,
                    orientation,
                });
                Ok(())
            }
        }
    }

    /// The `Identical`/`Anti` orientation of a coincident pair from the two
    /// faces' `orientation()` flags.
    fn orientation_pair(
        &self,
        a: (SolidRef, usize),
        b: (SolidRef, usize),
    ) -> CoincidentOrientation {
        let same = match (self.shell_face(a.0, a.1), self.shell_face(b.0, b.1)) {
            (Some(fa), Some(fb)) => fa.orientation() == fb.orientation(),
            _ => true,
        };
        if same {
            CoincidentOrientation::Identical
        } else {
            CoincidentOrientation::Anti
        }
    }

    /// Splits the CONTAINING face along the CONTAINED face's boundary wires,
    /// reusing already-inserted shared instances where the carriers identify.
    fn split_containing(
        &mut self,
        containing: (SolidRef, usize),
        contained: (SolidRef, usize),
    ) -> Result<(), Refusal> {
        let contained_face = self
            .shell_face(contained.0, contained.1)
            .ok_or_else(unsupported)?;
        let wires = contained_face.absolute_boundaries();
        if wires.len() != 1 {
            // v1 scope: the contained face is a single-wire region. A
            // multi-wire contained face (with holes) is deferred.
            return Err(refused());
        }
        let wire = wires.first().ok_or_else(unsupported)?.clone();
        let all_present = wire
            .edge_iter()
            .all(|e| self.loops_have_edge(containing.0, containing.1, e));
        if all_present {
            return Ok(());
        }
        // The contained wire is not yet shared with the containing face:
        // split a single self-loop into its two half-edges (the seam form),
        // propagate the cut, then insert as the doubled independent loop.
        let prepared = self.prepare_contained_wire(&wire)?;
        self.add_doubled_loop(containing.0, containing.1, &prepared)
    }

    /// Whether a face's loops already carry an edge with the same instance or
    /// a geometrically identical carrier as `edge`.
    fn loops_have_edge(
        &self,
        solid: SolidRef,
        face_idx: usize,
        edge: &Edge<Point3, Curve>,
    ) -> bool {
        let loops = match solid {
            SolidRef::A => self.loops_a.get(face_idx),
            SolidRef::B => self.loops_b.get(face_idx),
        };
        let Some(loops) = loops else {
            return false;
        };
        loops.0.iter().any(|wire| {
            wire.edge_iter().any(|w_e| {
                w_e.id() == edge.id()
                    || curves_geometrically_identical(&w_e.curve(), &edge.curve(), self.tol)
            })
        })
    }

    /// Prepares a contained boundary wire for insertion: a single self-loop
    /// edge is cut at its midpoint into the two half-edges (and the cut
    /// propagates to the contained solid's wires), so the seam instances are
    /// shared across the two solids.
    fn prepare_contained_wire(
        &mut self,
        wire: &Wire<Point3, Curve>,
    ) -> Result<Wire<Point3, Curve>, Refusal> {
        if wire.len() == 1 {
            let edge = wire.front_edge().ok_or_else(unsupported)?;
            if edge.front() == edge.back() {
                let curve = edge.curve();
                let (c0, c1) = curve.range_tuple();
                let t = (c0 + c1) * 0.5;
                let vertex = Vertex::new(curve.subs(t));
                let (e0, e1) = edge
                    .cut_with_parameter(&vertex, t)
                    .ok_or_else(numerically_unresolved)?;
                let halves = Wire::from(vec![e0, e1]);
                self.swap_edge_into_wire(edge.id(), &halves);
                return Ok(halves);
            }
        }
        Ok(wire.clone())
    }

    // -- division and output --------------------------------------------------

    /// Divides every face into fragments, then builds the adjacency and
    /// coincident outputs.
    fn finish(&mut self) -> Outcome<FragmentMesh> {
        let mut fragments: Vec<Fragment> = Vec::new();
        let mut origins: Vec<(SolidRef, usize)> = Vec::new();
        for solid in [SolidRef::A, SolidRef::B] {
            let face_count = self.shell(solid).face_iter().count();
            for fi in 0..face_count {
                if self.is_touched(solid, fi) {
                    let faces = self.divide_face(solid, fi)?;
                    for face in faces {
                        let origin = match solid {
                            SolidRef::A => FragmentOrigin::A { parent: fi },
                            SolidRef::B => FragmentOrigin::B { parent: fi },
                        };
                        fragments.push(Fragment { face, origin });
                        origins.push((solid, fi));
                    }
                } else {
                    let face = self.single_fragment(solid, fi)?;
                    let origin = match solid {
                        SolidRef::A => FragmentOrigin::A { parent: fi },
                        SolidRef::B => FragmentOrigin::B { parent: fi },
                    };
                    fragments.push(Fragment { face, origin });
                    origins.push((solid, fi));
                }
            }
        }
        let adjacency = self.build_adjacency(&fragments, &origins);
        let coincident = self.resolve_coincident(&fragments, &origins)?;
        let mesh = FragmentMesh {
            fragments,
            adjacency,
            coincident,
        };
        let cert = Certificate {
            props: PropMap::new(),
            method: Method::Float,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        };
        Ok(Certified::new(mesh, cert))
    }

    /// The single fragment of a face with no contact wire, built from the
    /// (possibly propagation-cut) mutated boundary wires.
    fn single_fragment(
        &self,
        solid: SolidRef,
        face_idx: usize,
    ) -> Result<Face<Point3, Curve, Surface>, Refusal> {
        let face = self.shell_face(solid, face_idx).ok_or_else(unsupported)?;
        let loops = match solid {
            SolidRef::A => self.loops_a.get(face_idx),
            SolidRef::B => self.loops_b.get(face_idx),
        }
        .ok_or_else(unsupported)?;
        let wires: Vec<Wire<Point3, Curve>> = loops.0.clone();
        let mut new_face = Face::new_unchecked(wires, face.surface());
        if !face.orientation() {
            new_face.invert();
        }
        Ok(new_face)
    }

    /// Divides a touched face into its fragments via the parameter-region
    /// decomposition.
    fn divide_face(
        &mut self,
        solid: SolidRef,
        face_idx: usize,
    ) -> Result<Vec<Face<Point3, Curve, Surface>>, Refusal> {
        let face = self.shell_face(solid, face_idx).ok_or_else(unsupported)?;
        let loops = match solid {
            SolidRef::A => self.loops_a.get(face_idx),
            SolidRef::B => self.loops_b.get(face_idx),
        }
        .ok_or_else(unsupported)?;
        divide_one_face(face, loops, self.tol).ok_or_else(numerically_unresolved)
    }

    /// The same-solid adjacency entries, one per shared edge instance.
    fn build_adjacency(
        &self,
        fragments: &[Fragment],
        origins: &[(SolidRef, usize)],
    ) -> Vec<FragmentAdjacency> {
        let mut adjacency = Vec::new();
        for i in 0..fragments.len() {
            for j in (i + 1)..fragments.len() {
                let (Some(fi), Some(fj), Some(oi), Some(oj)) = (
                    fragments.get(i),
                    fragments.get(j),
                    origins.get(i),
                    origins.get(j),
                ) else {
                    continue;
                };
                if oi.0 != oj.0 {
                    continue;
                }
                let ids_i = collect_edge_ids(&fi.face);
                let ids_j = collect_edge_ids(&fj.face);
                for id in ids_i {
                    if ids_j.contains(&id) {
                        let parity = if self.contact_set(oi.0).contains(&id) {
                            AdjacencyParity::Flip
                        } else {
                            AdjacencyParity::Same
                        };
                        adjacency.push(FragmentAdjacency {
                            lhs: i,
                            rhs: j,
                            parity,
                        });
                    }
                }
            }
        }
        adjacency
    }

    /// Resolves the pending coincident pairs to fragment indices by the
    /// region-membership of the contained face's representative point.
    fn resolve_coincident(
        &mut self,
        fragments: &[Fragment],
        origins: &[(SolidRef, usize)],
    ) -> Result<Vec<CoincidentPair>, Refusal> {
        let mut out = Vec::new();
        let pending = self.pending_coincident.clone();
        for pc in &pending {
            let contained_face = self
                .shell_face(pc.b_solid, pc.b_face)
                .ok_or_else(unsupported)?;
            let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
            let mut b_polys = Vec::new();
            for wire in contained_face.absolute_boundaries() {
                b_polys.push(
                    create_parameter_boundary(contained_face, wire, &mut cache, self.tol)
                        .ok_or_else(numerically_unresolved)?,
                );
            }
            let rep =
                region_representative(&b_polys, self.tol).ok_or_else(numerically_unresolved)?;
            let rep_3d = contained_face.surface().subs(rep.x, rep.y);
            let a_idx =
                self.fragment_covering(fragments, origins, pc.a_solid, pc.a_face, rep_3d)?;
            let b_idx =
                self.fragment_covering(fragments, origins, pc.b_solid, pc.b_face, rep_3d)?;
            out.push(CoincidentPair {
                a: a_idx,
                b: b_idx,
                orientation: pc.orientation,
            });
        }
        Ok(out)
    }

    /// The fragment of `(solid, face_idx)` whose region contains `point_3d`.
    fn fragment_covering(
        &mut self,
        fragments: &[Fragment],
        origins: &[(SolidRef, usize)],
        solid: SolidRef,
        face_idx: usize,
        point_3d: Point3,
    ) -> Result<usize, Refusal> {
        for (idx, (fragment, origin)) in fragments.iter().zip(origins.iter()).enumerate() {
            if origin.0 != solid || origin.1 != face_idx {
                continue;
            }
            let face = &fragment.face;
            let mut cache: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
            let mut polys = Vec::new();
            for wire in face.absolute_boundaries() {
                polys.push(
                    create_parameter_boundary(face, wire, &mut cache, self.tol)
                        .ok_or_else(numerically_unresolved)?,
                );
            }
            let Some(uv) = face
                .surface()
                .search_parameter(point_3d, None, SURFACE_SEARCH_TRIALS)
            else {
                continue;
            };
            let uv: Point2 = uv.into();
            let u_period = face.surface().u_period();
            if region_contains(&polys, uv, u_period) {
                return Ok(idx);
            }
        }
        Err(refused())
    }
}

// ---------------------------------------------------------------------------
// free helper functions
// ---------------------------------------------------------------------------

/// The deferred-envelope refusal.
fn refused() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

/// The numerically-unresolved refusal for a failed/unstable insertion
/// projection.
fn numerically_unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    }
}

/// A malformed stratum reference: the certified record cannot be mapped onto
/// the topology.
fn unsupported() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

/// Whether two points are within `tol` of each other.
pub(crate) fn near_pt(a: Point3, b: Point3, tol: f64) -> bool {
    (a - b).magnitude2() <= tol * tol
}

/// Shifts `value` by an integer multiple of `period` so the signed jump from
/// `previous` is at most half a period (the divide_face seam-unwrap helper).
#[inline(always)]
fn unwrap_periodic_parameter(previous: f64, value: f64, period: f64) -> f64 {
    value + ((previous - value) / period).round() * period
}

/// Projects the boundary edge's division points into the face's `(u, v)`
/// parameter space, caching the per-edge 3-D polylines (the rebuilt
/// `create_parameter_boundary`).
pub(crate) fn create_parameter_boundary(
    face: &Face<Point3, Curve, Surface>,
    wire: &Wire<Point3, Curve>,
    polys: &mut HashMap<EdgeID<Curve>, PolylineCurve<Point3>>,
    tol: f64,
) -> Option<PolylineCurve<Point2>> {
    let surface = face.surface();
    let u_period = surface.u_period();
    let pt = wire.front_vertex()?.point();
    let p: Point2 = surface
        .search_parameter(pt, None, SURFACE_SEARCH_TRIALS)?
        .into();
    let vec = wire.edge_iter().try_fold(vec![p], |mut vec, edge| {
        let poly = polys.entry(edge.id()).or_insert_with(|| {
            let curve = edge.curve();
            let div = curve.parameter_division(curve.range_tuple(), tol).1;
            PolylineCurve(div)
        });
        let mut p = *vec.last()?;
        let closure = |q: &Point3| -> Option<Point2> {
            let mut uv: Point2 = surface
                .search_parameter(*q, Some(p.into()), SURFACE_SEARCH_TRIALS)?
                .into();
            if let Some(period) = u_period {
                uv.x = unwrap_periodic_parameter(p.x, uv.x, period);
            }
            p = uv;
            Some(p)
        };
        let add: Option<Vec<Point2>> = match edge.orientation() {
            true => poly.iter().skip(1).map(closure).collect(),
            false => poly.iter().rev().skip(1).map(closure).collect(),
        };
        vec.append(&mut add?);
        Some(vec)
    })?;
    Some(PolylineCurve(vec))
}

/// A closed wire from a full-period curve, cut at its midpoint parameter into
/// two half-edges.
fn create_independent_loop(curve: Curve) -> Option<Wire<Point3, Curve>> {
    let (t0, t1) = curve.range_tuple();
    let t = (t0 + t1) * 0.5;
    let mut head = curve;
    let tail = head.cut(t);
    let v0 = Vertex::new(head.front());
    let v1 = Vertex::new(tail.front());
    let edge0 = Edge::try_new(&v0, &v1, head).ok()?;
    let edge1 = Edge::try_new(&v1, &v0, tail).ok()?;
    Some(Wire::from(vec![edge0, edge1]))
}

/// Clips `curve` to `[t_min, t_max]`, skipping degenerate end cuts.
fn clip_curve(curve: Curve, t_min: f64, t_max: f64) -> Option<Curve> {
    let (c0, c1) = curve.range_tuple();
    let cut_max = t_max < c1 - PARAM_SLACK;
    let cut_min = t_min > c0 + PARAM_SLACK;
    let mut keep = curve;
    if cut_max {
        let _tail = keep.cut(t_max);
        if cut_min {
            let sub = keep.cut(t_min);
            Some(sub)
        } else {
            Some(keep)
        }
    } else if cut_min {
        let sub = keep.cut(t_min);
        Some(sub)
    } else {
        Some(keep)
    }
}

/// Whether a curve is a closed full-period circle.
fn is_full_period_circle(curve: &Curve) -> bool {
    match curve {
        Curve::Circle(_) => {
            let (t0, t1) = curve.range_tuple();
            t1 - t0 >= TAU - PARAM_SLACK
        }
        _ => false,
    }
}

/// The `Curve` carrier of an exact intersection curve; `Parabola`/`Hyperbola`
/// have no `Curve` arm and refuse.
fn exact_to_curve(exact: &ExactCurve) -> Result<Curve, Refusal> {
    match exact {
        ExactCurve::Line(line) => Ok(Curve::Line(*line)),
        ExactCurve::Circle(placed) | ExactCurve::Ellipse(placed) => Ok(Curve::Circle(*placed)),
        ExactCurve::Parabola(_) | ExactCurve::Hyperbola(_) => Err(refused()),
    }
}

/// The parameter-region decomposition (the rebuilt `divide_one_face`): signed
/// polygon area groups the wires into regions and their holes (a REGION is a
/// positive-area wire on an orientation-true face, a negative-area wire on an
/// inverted one), unattachable negative wires become regions in their own
/// right, and each pre-region becomes a fragment face.
fn divide_one_face(
    face: &Face<Point3, Curve, Surface>,
    loops: &Loops,
    tol: f64,
) -> Option<Vec<Face<Point3, Curve, Surface>>> {
    let is_region = |area: f64| {
        if face.orientation() {
            area > 0.0
        } else {
            area < 0.0
        }
    };
    let (mut pre_faces, mut negative_wires) = (Vec::new(), Vec::new());
    let mut map: HashMap<EdgeID<Curve>, PolylineCurve<Point3>> = HashMap::default();
    for wire in &loops.0 {
        let poly = create_parameter_boundary(face, wire, &mut map, tol)?;
        if is_region(poly.area()) {
            pre_faces.push(vec![(poly, wire)]);
        } else {
            negative_wires.push((poly, wire));
        }
    }
    for (poly, wire) in negative_wires {
        let pt = poly.front();
        let op = pre_faces.iter_mut().find(|pre| {
            pre.first()
                .is_some_and(|(first_poly, _)| orient_ccw(first_poly).include(pt))
        });
        match op {
            Some(pre) => pre.push((poly, wire)),
            None => pre_faces.push(vec![(poly, wire)]),
        }
    }
    let surface = face.surface();
    let vec: Vec<_> = pre_faces
        .into_iter()
        .map(|pre_face| {
            let wires: Vec<Wire<Point3, Curve>> =
                pre_face.iter().map(|(_, w)| (*w).clone()).collect();
            let mut new_face = Face::new_unchecked(wires, surface.clone());
            if !face.orientation() {
                new_face.invert();
            }
            new_face
        })
        .collect();
    Some(vec)
}

/// All edge ids of a fragment face's boundary wires.
fn collect_edge_ids(face: &Face<Point3, Curve, Surface>) -> Vec<EdgeID<Curve>> {
    let mut out = Vec::new();
    for wire in face.absolute_boundaries() {
        for edge in wire.edge_iter() {
            out.push(edge.id());
        }
    }
    out
}

/// A CCW-oriented copy of a parameter polygon.
fn orient_ccw(poly: &PolylineCurve<Point2>) -> PolylineCurve<Point2> {
    if poly.area() > 0.0 {
        poly.clone()
    } else {
        poly.inverse()
    }
}

/// Whether the parameter point is strictly inside the region bounded by the
/// face's wire polygons (outer minus holes). When `u_period` is `Some(T)` the
/// test also runs at `p.x ± T`: a query parameter produced in one frame can
/// sit on a different u-branch than the unwrapped boundary polygons, so the ±
/// period translates of the QUERY are what let the frame-mismatched point see
/// its true region membership.
pub(crate) fn region_contains(
    polys: &[PolylineCurve<Point2>],
    p: Point2,
    u_period: Option<f64>,
) -> bool {
    let inside = |q: Point2| {
        let Some(outer) = polys.first() else {
            return false;
        };
        let outer = orient_ccw(outer);
        outer.include(q)
            && polys.iter().skip(1).all(|hole| {
                let hole = orient_ccw(hole);
                !hole.include(q)
            })
    };
    if inside(p) {
        return true;
    }
    match u_period {
        Some(period) => {
            inside(Point2::new(p.x + period, p.y)) || inside(Point2::new(p.x - period, p.y))
        }
        None => false,
    }
}

/// The perpendicular distance from `p` to the segment `a`-`b`.
pub(crate) fn point_segment_distance(p: Point2, a: Point2, b: Point2) -> f64 {
    let ab = Vector2::new(b.x - a.x, b.y - a.y);
    let ap = Vector2::new(p.x - a.x, p.y - a.y);
    let len2 = ab.dot(ab);
    if len2 == 0.0 {
        return ap.magnitude();
    }
    let t = (ap.dot(ab) / len2).clamp(0.0, 1.0);
    let proj = Vector2::new(a.x + t * ab.x, a.y + t * ab.y);
    Vector2::new(p.x - proj.x, p.y - proj.y).magnitude()
}

/// Whether the segment `a`-`b` crosses any face-boundary segment; returns the
/// parameter `s` of the first crossing along `a`-`b`.
fn segment_boundary_crossing(
    a: Point2,
    b: Point2,
    face_polys: &[PolylineCurve<Point2>],
) -> Option<f64> {
    for poly in face_polys {
        for (c, d) in poly.iter().circular_tuple_windows() {
            if let Some(s) = segment_cross_param(a, b, *c, *d) {
                return Some(s);
            }
        }
    }
    None
}

/// The proper-crossing parameter `s` of segment `a`-`b` with segment `c`-`d`
/// (strict interior crossing).
fn segment_cross_param(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<f64> {
    let orient =
        |p: Point2, q: Point2, r: Point2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    let proper = (o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
        && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0);
    if !proper {
        return None;
    }
    // s on [0, 1] along a->b: solve a + s(b-a) on the line through c,d.
    let denom = (b.x - a.x) * (d.y - c.y) - (b.y - a.y) * (d.x - c.x);
    if denom == 0.0 {
        return None;
    }
    let s = ((c.x - a.x) * (d.y - c.y) - (c.y - a.y) * (d.x - c.x)) / denom;
    Some(s.clamp(0.0, 1.0))
}

/// Whether any proper crossing exists between two faces' boundary polygons.
fn wires_cross(a: &[PolylineCurve<Point2>], b: &[PolylineCurve<Point2>]) -> bool {
    for pa in a {
        for (a0, a1) in pa.iter().circular_tuple_windows() {
            for pb in b {
                for (b0, b1) in pb.iter().circular_tuple_windows() {
                    if segment_cross_param(*a0, *a1, *b0, *b1).is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// The signed-area centroid of a parameter polygon.
fn polygon_centroid(poly: &PolylineCurve<Point2>) -> Option<Point2> {
    let mut iter = poly.iter().copied();
    let first = iter.next()?;
    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut prev = first;
    for cur in iter {
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

/// An interior representative point of the region, if one can be found: the
/// outer centroid, else inward-nudged outer edge midpoints.
pub(crate) fn region_representative(polys: &[PolylineCurve<Point2>], tol: f64) -> Option<Point2> {
    let outer = polys.first()?;
    let mut candidates: Vec<Point2> = Vec::new();
    if let Some(centroid) = polygon_centroid(outer) {
        candidates.push(centroid);
    }
    let nudge = tol.max(1.0e-9); // H-3: dimensionless nudging floor for the representative search
    for (a, b) in outer.iter().circular_tuple_windows() {
        let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
        let dir = Vector3::new(b.x - a.x, b.y - a.y, 0.0);
        let left = Vector3::new(-dir.y, dir.x, 0.0);
        let len = left.magnitude();
        if len > 0.0 {
            candidates.push(Point2::new(
                mid.x + left.x / len * nudge,
                mid.y + left.y / len * nudge,
            ));
        }
    }
    candidates
        .into_iter()
        .find(|p| region_contains(polys, *p, None))
}

/// The Region2 containment screen between two faces' wire polygons.
fn containment_screen(
    a: &[PolylineCurve<Point2>],
    b: &[PolylineCurve<Point2>],
    tol: f64,
) -> RegionScreen {
    if wires_cross(a, b) {
        return RegionScreen::Crossing;
    }
    let boundary_points = |polys: &[PolylineCurve<Point2>]| {
        let mut pts = Vec::new();
        for poly in polys {
            pts.extend(poly.iter().copied());
        }
        pts
    };
    let a_pts = boundary_points(a);
    let b_pts = boundary_points(b);
    let rep_a = region_representative(a, tol);
    let rep_b = region_representative(b, tol);
    let a_boundary_in_b = a_pts.iter().any(|p| region_contains(b, *p, None));
    let b_boundary_in_a = b_pts.iter().any(|p| region_contains(a, *p, None));
    let a_rep_in_b = rep_a.is_some_and(|p| region_contains(b, p, None));
    let b_rep_in_a = rep_b.is_some_and(|p| region_contains(a, p, None));
    match (b_boundary_in_a && b_rep_in_a, a_boundary_in_b && a_rep_in_b) {
        (true, _) => RegionScreen::AContainsB,
        (_, true) => RegionScreen::BContainsA,
        (false, false) => {
            let overlap = a_pts.iter().any(|p| region_contains(b, *p, None))
                || b_pts.iter().any(|p| region_contains(a, *p, None))
                || rep_a.is_some_and(|p| region_contains(b, p, None))
                || rep_b.is_some_and(|p| region_contains(a, p, None));
            if overlap {
                RegionScreen::PartialOverlap
            } else {
                RegionScreen::Disjoint
            }
        }
    }
}

/// Whether two placed circles are the same geometric circle: same center,
/// radius, and (parallel) plane.
fn circles_identical(
    a: &Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>,
    b: &Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>,
    tol: f64,
) -> bool {
    let ta = a.transform();
    let tb = b.transform();
    let center_a = Point3::new(ta.w.x, ta.w.y, ta.w.z);
    let center_b = Point3::new(tb.w.x, tb.w.y, tb.w.z);
    let ax = Vector3::new(ta.x.x, ta.x.y, ta.x.z);
    let ay = Vector3::new(ta.y.x, ta.y.y, ta.y.z);
    let bx = Vector3::new(tb.x.x, tb.x.y, tb.x.z);
    let by = Vector3::new(tb.y.x, tb.y.y, tb.y.z);
    let radius_a = ax.magnitude();
    let radius_b = bx.magnitude();
    let na = ax.cross(ay);
    let nb = bx.cross(by);
    let na_len = na.magnitude();
    let nb_len = nb.magnitude();
    if na_len == 0.0 || nb_len == 0.0 {
        return false;
    }
    let parallel = na.cross(nb).magnitude() <= tol * na_len * nb_len;
    near_pt(center_a, center_b, tol) && (radius_a - radius_b).abs() <= tol && parallel
}

/// The geometric identity of two exact curves (the sewing-oracle predicate).
fn exact_curves_identical(a: &ExactCurve, b: &ExactCurve, tol: f64) -> bool {
    match (a, b) {
        (ExactCurve::Line(la), ExactCurve::Line(lb)) => {
            let Line(a0, a1) = *la;
            let Line(b0, b1) = *lb;
            (near_pt(a0, b0, tol) && near_pt(a1, b1, tol))
                || (near_pt(a0, b1, tol) && near_pt(a1, b0, tol))
        }
        (ExactCurve::Circle(ca), ExactCurve::Circle(cb))
        | (ExactCurve::Circle(ca), ExactCurve::Ellipse(cb))
        | (ExactCurve::Ellipse(ca), ExactCurve::Circle(cb))
        | (ExactCurve::Ellipse(ca), ExactCurve::Ellipse(cb)) => circles_identical(ca, cb, tol),
        _ => false,
    }
}

/// The geometric identity of two stored curves.
fn curves_geometrically_identical(a: &Curve, b: &Curve, tol: f64) -> bool {
    match (a, b) {
        (Curve::Line(la), Curve::Line(lb)) => {
            let Line(a0, a1) = *la;
            let Line(b0, b1) = *lb;
            (near_pt(a0, b0, tol) && near_pt(a1, b1, tol))
                || (near_pt(a0, b1, tol) && near_pt(a1, b0, tol))
        }
        (Curve::Circle(ca), Curve::Circle(cb)) => circles_identical(ca, cb, tol),
        _ => false,
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
    use truck_geometry::arrange::arrange;
    use truck_geometry::arrange::Arrangement;
    use truck_geometry::prelude::*;
    use truck_modeling::extrude::extrude_profile;
    /// The insertion tolerance class for the splitter calls (H-3: dimensionless
    /// relative to the unit-scale witnesses; dyadic geometry decides exactly).
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

    /// The M1 plate-with-hole profile: the 4x4 rectangle plus a full circle r=1
    /// at (2, 2).
    fn plate_with_hole_profile() -> (Vec<Curve>, Arrangement) {
        let mut profile = vec![
            Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 0.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 0.0, 0.0), Point3::new(4.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(4.0, 4.0, 0.0), Point3::new(0.0, 4.0, 0.0))),
            Curve::Line(Line(Point3::new(0.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0))),
        ];
        let circle = Curve::Circle(placed_circle(Point3::new(2.0, 2.0, 0.0), 1.0));
        profile.push(circle);
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

    /// The flat edge index (in `face.absolute_boundaries()` wire-by-wire order)
    /// of the edge whose curve's midpoint sits at z.
    fn flat_edge_at_z(shell: &Shell<Point3, Curve, Surface>, face_idx: usize, z: f64) -> usize {
        let face = shell.get(face_idx).unwrap();
        let mut flat = 0usize;
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let curve = edge.curve();
                let (t0, t1) = curve.range_tuple();
                let mid = curve.subs((t0 + t1) * 0.5);
                if (mid.z - z).abs() < TOL {
                    return flat;
                }
                flat += 1;
            }
        }
        unreachable!("no edge at z = {z}")
    }

    /// The fragment indices whose origin is `(solid, parent)`.
    fn fragments_of_origin(mesh: &FragmentMesh, solid: SolidRef, parent: usize) -> Vec<usize> {
        mesh.fragments
            .iter()
            .enumerate()
            .filter(|(_, fragment)| match (fragment.origin, solid) {
                (FragmentOrigin::A { parent: p }, SolidRef::A)
                | (FragmentOrigin::B { parent: p }, SolidRef::B) => p == parent,
                _ => false,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The edge ids of a fragment's face wires, flattened wire-by-wire.
    fn fragment_edge_ids(mesh: &FragmentMesh, idx: usize) -> Vec<EdgeID<Curve>> {
        collect_edge_ids(&mesh.fragments[idx].face)
    }

    /// The count of edges in the i-th wire of a fragment face.
    fn wire_edge_counts(mesh: &FragmentMesh, idx: usize) -> Vec<usize> {
        mesh.fragments[idx]
            .face
            .absolute_boundaries()
            .iter()
            .map(|wire| wire.len())
            .collect()
    }

    /// A hand-built one-wire disk face on the z=2 plane, the self-loop pattern.
    fn disk_face(center: Point2, r: f64) -> Face<Point3, Curve, Surface> {
        let circle = placed_circle(Point3::new(center.x, center.y, 2.0), r);
        let v0 = Vertex::new(circle.subs(0.0));
        let edge = Edge::new_unchecked(&v0, &v0, Curve::Circle(circle));
        let wire: Wire<Point3, Curve> = Wire::from(vec![edge]);
        Face::new_unchecked(
            vec![wire],
            Surface::Plane(Plane::new(
                Point3::new(0.0, 0.0, 2.0),
                Point3::new(1.0, 0.0, 2.0),
                Point3::new(0.0, 1.0, 2.0),
            )),
        )
    }

    /// A contact event from its record and two strata.
    fn ev(record: ContactRecord, lhs: StratumRef, rhs: StratumRef) -> ContactEvent {
        ContactEvent { record, lhs, rhs }
    }

    /// The `{Arc1, Transverse, Analytic(Curve(exact))}` record.
    fn ff_curve_record(exact: ExactCurve) -> ContactRecord {
        ContactRecord {
            dimension: ContactDimension::Arc1,
            kind: ContactEventKind::Transverse,
            locus: ContactLocus::Analytic(AnalyticIntersection::Curve(exact)),
        }
    }

    // ---------------------------------------------------------------------------
    // Test 1: the flagship.
    // ---------------------------------------------------------------------------

    #[test]
    fn split_flagship_top_face_by_ff_circle() {
        // a = the 4x4 block extrude (6 faces: bottom, top, 4 sides).
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        // b = the disk extrude at (2, 2) r=1 (3 faces: bottom cap, top cap, wall).
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        // Derivation of the expected fragment structure:
        //   a's top face (the z=2 plane, orientation true) is split by the FF
        //   circle into the DISK (one wire of the two rim half-edges) and the
        //   ANNULUS (the square wire plus the hole wire of the same two
        //   half-edges inverted). a has 6 faces -> 2 + 5 = 7 fragments. b has 3
        //   faces -> 3 fragments. Total 10 fragments.
        //   Adjacency: disk<->annulus 2 Flip (per half-edge); a's untouched
        //   faces: annulus<->sides 4 Same, bottom<->sides 4 Same, sides<->sides
        //   4 Same = 12 Same; b's wall<->top-cap 2 Same + wall<->bottom-cap 1
        //   Same = 3 Same. Total 17 entries (2 Flip, 15 Same). The disk<->cap
        //   rim halves are cross-solid sewing, never adjacency.
        //   Coincident: exactly one pair {a: disk, b: top cap, Identical}.

        let top_a = plane_face_at_z(&shell_a, 2.0);
        let wall_b = cylinder_face(&shell_b);
        let cap_b = plane_face_at_z(&shell_b, 2.0);
        let rim_edge = flat_edge_at_z(&shell_b, wall_b, 2.0);

        let exact = ExactCurve::Circle(placed_circle(Point3::new(2.0, 2.0, 2.0), 1.0));

        // FF: the wall x plane intersection circle, on a's top face and b's wall.
        let ff = ev(
            ff_curve_record(exact.clone()),
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: wall_b,
            },
        );
        // FE: the same circle carried by b's wall top rim edge (the sewing oracle).
        let fe = ev(
            ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::BoundedCurve {
                    curve: exact,
                    t_range: (0.0, TAU),
                },
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Edge {
                solid: SolidRef::B,
                face: wall_b,
                edge: rim_edge,
            },
        );
        // Region2: a's top face and b's top cap are coincident.
        let r2 = ev(
            ContactRecord {
                dimension: ContactDimension::Region2,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::Coincident,
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: cap_b,
            },
        );

        let mesh = split_fragments(&shell_a, &shell_b, &[ff, fe, r2], TOL)
            .unwrap()
            .value;

        // Total fragments: 7 from a + 3 from b = 10.
        assert_eq!(mesh.fragments.len(), 10);

        // a's top face becomes exactly two fragments.
        let top_frags = fragments_of_origin(&mesh, SolidRef::A, top_a);
        assert_eq!(top_frags.len(), 2);
        // The DISK fragment has one wire of two half-edges; the ANNULUS has the
        // square wire (4 edges) plus the hole wire (2 half-edges).
        let mut annulus = None;
        let mut disk = None;
        for idx in top_frags {
            let counts = wire_edge_counts(&mesh, idx);
            match counts.as_slice() {
                [2] => disk = Some(idx),
                [4, 2] => annulus = Some(idx),
                other => unreachable!("unexpected top-face wire structure: {other:?}"),
            }
        }
        let annulus = annulus.unwrap();
        let disk = disk.unwrap();
        assert_ne!(annulus, disk);

        // Every other face is exactly one fragment: a's 5 untouched faces plus
        // b's 3 faces.
        assert_eq!(
            fragments_of_origin(&mesh, SolidRef::A, plane_face_at_z(&shell_a, 0.0)).len(),
            1
        );
        for side in 0..4 {
            let idx = 2 + side;
            assert_eq!(fragments_of_origin(&mesh, SolidRef::A, idx).len(), 1);
        }
        assert_eq!(fragments_of_origin(&mesh, SolidRef::B, wall_b).len(), 1);
        assert_eq!(fragments_of_origin(&mesh, SolidRef::B, cap_b).len(), 1);
        assert_eq!(
            fragments_of_origin(&mesh, SolidRef::B, plane_face_at_z(&shell_b, 0.0)).len(),
            1
        );

        // The two rim half-edge INSTANCES are EdgeID-identical across the disk
        // fragment, the annulus fragment, b's wall top wire, and b's top-cap
        // wire.
        let disk_ids = fragment_edge_ids(&mesh, disk);
        assert_eq!(disk_ids.len(), 2);
        // The edge IDs are shared INSTANCES; a wire may carry them in either
        // order, so compare as unordered pairs.
        let same_pair = |a: &[EdgeID<Curve>], b: &[EdgeID<Curve>]| {
            a.len() == b.len() && a.iter().all(|id| b.contains(id))
        };
        // The annulus's second wire (the hole) carries the same two halves.
        let annulus_hole_ids = mesh.fragments[annulus]
            .face
            .absolute_boundaries()
            .get(1)
            .unwrap()
            .edge_iter()
            .map(|e| e.id())
            .collect::<Vec<_>>();
        assert!(same_pair(&annulus_hole_ids, &disk_ids));
        // b's wall top wire and b's top cap wire carry the two halves too.
        let wall_frag = fragments_of_origin(&mesh, SolidRef::B, wall_b)[0];
        let wall_top_ids = mesh.fragments[wall_frag]
            .face
            .absolute_boundaries()
            .get(1)
            .unwrap()
            .edge_iter()
            .map(|e| e.id())
            .collect::<Vec<_>>();
        assert!(same_pair(&wall_top_ids, &disk_ids));
        let cap_frag = fragments_of_origin(&mesh, SolidRef::B, cap_b)[0];
        assert!(same_pair(&fragment_edge_ids(&mesh, cap_frag), &disk_ids));

        // The coincident pair pairs the disk fragment with b's top-cap fragment,
        // Identical (both faces have orientation() == true).
        assert_eq!(mesh.coincident.len(), 1);
        let pair = mesh.coincident[0];
        assert_eq!(pair.a, disk);
        assert_eq!(pair.b, cap_frag);
        assert_eq!(pair.orientation, CoincidentOrientation::Identical);

        // Adjacency: 2 Flip (disk<->annulus, once per half-edge), 15 Same, no
        // cross-solid pair.
        assert_eq!(mesh.adjacency.len(), 17);
        let flips = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Flip)
            .collect::<Vec<_>>();
        assert_eq!(flips.len(), 2);
        for a in &flips {
            assert!(
                (a.lhs == disk && a.rhs == annulus) || (a.lhs == annulus && a.rhs == disk),
                "the only Flip entries are disk<->annulus"
            );
        }
        let sames = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Same);
        assert_eq!(sames.count(), 15);
        for a in &mesh.adjacency {
            let lhs_origin = mesh.fragments[a.lhs].origin;
            let rhs_origin = mesh.fragments[a.rhs].origin;
            let lhs_solid = match lhs_origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            let rhs_solid = match rhs_origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            assert_eq!(
                lhs_solid, rhs_solid,
                "adjacency is same-solid only; cross-solid edges are sewing"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 2: point contacts cut edges.
    // ---------------------------------------------------------------------------

    #[test]
    fn split_cuts_edges_at_point_contacts() {
        // a = the 4x4 block. One synthetic Point event cuts a's top face's FIRST
        // boundary edge (the (0,0,2)->(4,0,2) line) at (2,0,2).
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let shell_b = extrude_shell(&profile_a, &arr_a, 2.0);

        let top_a = plane_face_at_z(&shell_a, 2.0);
        // The top face's first boundary edge is the (0,0,2)->(4,0,2) line.
        let cut = ev(
            ContactRecord {
                dimension: ContactDimension::Point0,
                kind: ContactEventKind::Transverse,
                locus: ContactLocus::Point(Point3::new(2.0, 0.0, 2.0)),
            },
            StratumRef::Edge {
                solid: SolidRef::A,
                face: top_a,
                edge: 0,
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
        );

        let mesh = split_fragments(&shell_a, &shell_b, &[cut], TOL)
            .unwrap()
            .value;

        // The top face is still ONE fragment; its wire now has 5 edges (the
        // first boundary edge split at (2,0,2): 3 + 1 = 4 original edges + 1 cut
        // vertex).
        let top_frags = fragments_of_origin(&mesh, SolidRef::A, top_a);
        assert_eq!(top_frags.len(), 1);
        let top_frag = top_frags[0];
        assert_eq!(wire_edge_counts(&mesh, top_frag), vec![5]);

        let wire = mesh.fragments[top_frag]
            .face
            .absolute_boundaries()
            .first()
            .unwrap();
        // The new vertex sits at (2,0,2).
        assert!(wire
            .vertex_iter()
            .any(|v| (v.point() - Point3::new(2.0, 0.0, 2.0)).magnitude2() <= TOL * TOL));
        // The two halves are Curve::Line with the right endpoints: the wire's
        // first two edges are (0,0,2)->(2,0,2) and (2,0,2)->(4,0,2).
        let edges: Vec<_> = wire.edge_iter().collect();
        let e0 = edges[0].curve();
        let e1 = edges[1].curve();
        let (Line(a0, b0), Line(a1, b1)) = match (e0, e1) {
            (Curve::Line(l0), Curve::Line(l1)) => (l0, l1),
            _ => unreachable!("the halves must be lines"),
        };
        assert_eq!(a0, Point3::new(0.0, 0.0, 2.0));
        assert_eq!(b0, Point3::new(2.0, 0.0, 2.0));
        assert_eq!(a1, Point3::new(2.0, 0.0, 2.0));
        assert_eq!(b1, Point3::new(4.0, 0.0, 2.0));

        // The cut propagates to the front side face (the y=0 face, shell index
        // 2): the top-face <-> front-side-face Same adjacency appears once per
        // half.
        let front_side = 2usize;
        let front_frags = fragments_of_origin(&mesh, SolidRef::A, front_side);
        assert_eq!(front_frags.len(), 1);
        let mut halves_same = 0usize;
        for adj in &mesh.adjacency {
            let pair = (adj.lhs, adj.rhs);
            if pair == (top_frag, front_frags[0]) || pair == (front_frags[0], top_frag) {
                assert_eq!(adj.parity, AdjacencyParity::Same);
                halves_same += 1;
            }
        }
        assert_eq!(halves_same, 2);
    }

    // ---------------------------------------------------------------------------
    // Test 3: open arcs are trimmed by certified point events.
    // ---------------------------------------------------------------------------

    #[test]
    fn split_open_arc_uses_point_events_for_trimming() {
        // a = the 4x4 block; b = the disk at (4, 2) r=1. The FF generators
        // x=4,y=1 and x=4,y=3 cross a's x=4 side face, certified by the four
        // Point events at (4,1,0), (4,3,0), (4,1,2), (4,3,2) on that face's
        // bottom/top edges.
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(4.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        // a's x=4 side face: the plane through (4,0,0),(4,4,0); its surface
        // origin identifies it.
        let x4_side = shell_a
            .face_iter()
            .enumerate()
            .find(|(_, face)| match face.surface() {
                Surface::Plane(p) => {
                    (p.origin().x - 4.0).abs() < TOL && (p.origin().y - 0.0).abs() < TOL
                }
                _ => false,
            })
            .map(|(i, _)| i)
            .unwrap();
        let wall_b = cylinder_face(&shell_b);
        // The side face's wire is [be, seam, te.inverse(), seam]: edge 0 is the
        // bottom (z=0) edge, edge 2 is the top (z=2) edge.
        let bottom_edge = flat_edge_at_z(&shell_a, x4_side, 0.0);
        let top_edge = flat_edge_at_z(&shell_a, x4_side, 2.0);
        assert_eq!(bottom_edge, 0);
        assert_eq!(top_edge, 2);

        // Derivation of the expected fragments: the two lines x=4,y=1 and
        // x=4,y=3 (each spanning z in [0,2]) split the side rectangle into THREE
        // strips [0,1], [1,3], [3,4]. Each strip's wire is a closed 4-edge loop
        // (bottom/top segments, seams, and the shared line edges). The middle
        // strip shares line1 with the lower strip and line2 with the upper strip,
        // so the adjacency has exactly 2 Flip entries; the outer strips share
        // their bottom/top segments and seams with the untouched neighbors
        // (Same). b's cylinder is split the same way by the same shared line
        // instances.

        let line1 = ExactCurve::Line(Line(Point3::new(4.0, 1.0, 0.0), Point3::new(4.0, 1.0, 2.0)));
        let line2 = ExactCurve::Line(Line(Point3::new(4.0, 3.0, 0.0), Point3::new(4.0, 3.0, 2.0)));

        let ff = ev(
            ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::Transverse,
                locus: ContactLocus::Analytic(AnalyticIntersection::TwoCurves([line1, line2])),
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: x4_side,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: wall_b,
            },
        );

        let mut events = vec![ff];
        for (y, z) in [(1.0, 0.0), (3.0, 0.0), (1.0, 2.0), (3.0, 2.0)] {
            let edge = if z == 0.0 { bottom_edge } else { top_edge };
            events.push(ev(
                ContactRecord {
                    dimension: ContactDimension::Point0,
                    kind: ContactEventKind::Transverse,
                    locus: ContactLocus::Point(Point3::new(4.0, y, z)),
                },
                StratumRef::Edge {
                    solid: SolidRef::A,
                    face: x4_side,
                    edge,
                },
                StratumRef::Face {
                    solid: SolidRef::A,
                    index: x4_side,
                },
            ));
        }

        let mesh = split_fragments(&shell_a, &shell_b, &events, TOL)
            .unwrap()
            .value;

        // The x=4 side face becomes THREE fragments.
        let side_frags = fragments_of_origin(&mesh, SolidRef::A, x4_side);
        assert_eq!(side_frags.len(), 3);

        // Each strip is a closed 4-edge loop.
        for idx in &side_frags {
            assert_eq!(wire_edge_counts(&mesh, *idx), vec![4]);
        }

        // The bottom and top edges are each cut at their two points (3 edges
        // each): across the three fragments, exactly 3 boundary segments sit at
        // z=0 and 3 at z=2.
        let mut bottom_segments = 0usize;
        let mut top_segments = 0usize;
        for idx in &side_frags {
            for edge in mesh.fragments[*idx].face.absolute_boundaries()[0].edge_iter() {
                if let Curve::Line(line) = edge.curve() {
                    let mid_z = (line.0.z + line.1.z) * 0.5;
                    if mid_z == 0.0 {
                        bottom_segments += 1;
                    } else if mid_z == 2.0 {
                        top_segments += 1;
                    }
                }
            }
        }
        assert_eq!(bottom_segments, 3);
        assert_eq!(top_segments, 3);

        // The two inserted line edges are SHARED instances: each line edge id
        // appears in exactly two of the three side fragments.
        let mut line_edges: Vec<EdgeID<Curve>> = Vec::new();
        for idx in &side_frags {
            for edge in mesh.fragments[*idx].face.absolute_boundaries()[0].edge_iter() {
                if let Curve::Line(line) = edge.curve() {
                    let at_y1 = (line.0.y - 1.0).abs() < TOL && (line.1.y - 1.0).abs() < TOL;
                    let at_y3 = (line.0.y - 3.0).abs() < TOL && (line.1.y - 3.0).abs() < TOL;
                    if (at_y1 || at_y3)
                        && line.0.z == 0.0
                        && line.1.z == 2.0
                        && !line_edges.contains(&edge.id())
                    {
                        line_edges.push(edge.id());
                    }
                }
            }
        }
        assert_eq!(line_edges.len(), 2);
        for id in &line_edges {
            let appearances = side_frags
                .iter()
                .filter(|idx| fragment_edge_ids(&mesh, **idx).contains(id))
                .count();
            assert_eq!(appearances, 2, "each line edge is shared by two strips");
        }

        // The adjacency across the two inserted lines is Flip (once per line,
        // per solid: a's side fragments and b's wall fragments each contribute
        // two); the outer strips' adjacencies to the neighboring untouched faces
        // are Same.
        let side_flips = mesh
            .adjacency
            .iter()
            .filter(|a| {
                a.parity == AdjacencyParity::Flip
                    && side_frags.contains(&a.lhs)
                    && side_frags.contains(&a.rhs)
            })
            .count();
        assert_eq!(side_flips, 2);
        let all_flips = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Flip)
            .count();
        assert_eq!(
            all_flips, 4,
            "both shells' splits contribute their two line arcs"
        );
        let sames = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Same)
            .count();
        assert!(
            sames > 0,
            "the outer strips share original edges with neighbors"
        );
        // No cross-solid adjacency pairs.
        for a in &mesh.adjacency {
            let lhs = match mesh.fragments[a.lhs].origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            let rhs = match mesh.fragments[a.rhs].origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            assert_eq!(lhs, rhs);
        }
    }

    // ---------------------------------------------------------------------------
    // Tests 4 & 5: the Region2 containment screen.
    // ---------------------------------------------------------------------------

    #[test]
    fn split_region2_disjoint_regions_is_no_coincidence() {
        // a = the plate-with-hole extrude; b's face = a hand-built disk r=0.8 at
        // (2,2), strictly inside the hole (r=1): the parameter boxes overlap but
        // the regions are disjoint, so the containment screen's rescue path emits
        // no coincident pair and no split.
        let (profile_a, arr_a) = plate_with_hole_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let top_a = plane_face_at_z(&shell_a, 2.0);

        let face = disk_face(Point2::new(2.0, 2.0), 0.8);
        let shell_b: Shell<Point3, Curve, Surface> = vec![face].into();

        let r2 = ev(
            ContactRecord {
                dimension: ContactDimension::Region2,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::Coincident,
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: 0,
            },
        );

        let mesh = split_fragments(&shell_a, &shell_b, &[r2], TOL)
            .unwrap()
            .value;

        // No coincident pair, no split: a's top face stays one fragment and b's
        // face stays one fragment.
        assert!(mesh.coincident.is_empty());
        assert_eq!(fragments_of_origin(&mesh, SolidRef::A, top_a).len(), 1);
        assert_eq!(fragments_of_origin(&mesh, SolidRef::B, 0).len(), 1);
        // No Flip (contact-arc) adjacency anywhere: no arc was inserted.
        assert!(
            mesh.adjacency
                .iter()
                .all(|a| a.parity == AdjacencyParity::Same),
            "no contact arc was inserted, so no Flip adjacency exists"
        );
    }

    #[test]
    fn split_region2_partial_overlap_refuses() {
        // a = the plate-with-hole extrude; b's face = a hand-built disk r=1.5 at
        // (2,2): the disk's boundary lies inside a's annulus region but its
        // interior wraps a's hole, so the regions PARTIALLY OVERLAP (the annulus
        // ring r in [1, 1.5]) and the whole call refuses.
        let (profile_a, arr_a) = plate_with_hole_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let top_a = plane_face_at_z(&shell_a, 2.0);

        let face = disk_face(Point2::new(2.0, 2.0), 1.5);
        let shell_b: Shell<Point3, Curve, Surface> = vec![face].into();

        let r2 = ev(
            ContactRecord {
                dimension: ContactDimension::Region2,
                kind: ContactEventKind::CoincidentInterval,
                locus: ContactLocus::Coincident,
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: 0,
            },
        );

        let out = split_fragments(&shell_a, &shell_b, &[r2], TOL);
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::ContactReductionDeferred
                ))
            ),
            "the partial-overlap family refuses with ContactReductionDeferred"
        );
    }

    // ---------------------------------------------------------------------------
    // Test 6: deferred loci refuse.
    // ---------------------------------------------------------------------------

    #[test]
    fn split_refuses_deferred_loci() {
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let shell_b = extrude_shell(&profile_a, &arr_a, 2.0);
        let top_a = plane_face_at_z(&shell_a, 2.0);

        // (a) a Tangency-kind record: Analytic(TangentLine) on the face.
        let tangency = ev(
            ContactRecord {
                dimension: ContactDimension::Arc1,
                kind: ContactEventKind::Tangency,
                locus: ContactLocus::Analytic(AnalyticIntersection::TangentLine(Line(
                    Point3::new(0.0, 0.0, 2.0),
                    Point3::new(1.0, 0.0, 2.0),
                ))),
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: top_a,
            },
        );

        // (b) a Transverse Parabola curve: no Curve arm.
        let parabola = ExactCurve::Parabola(Processor::with_transform(
            TrimmedCurve::new(UnitParabola::<Point3>::new(), (0.0, 1.0)),
            Matrix4::identity(),
        ));
        let parab = ev(
            ff_curve_record(parabola),
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: top_a,
            },
        );

        // (c) an EndpointTouch point.
        let endpoint = ev(
            ContactRecord {
                dimension: ContactDimension::Point0,
                kind: ContactEventKind::EndpointTouch,
                locus: ContactLocus::Point(Point3::new(2.0, 0.0, 2.0)),
            },
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Edge {
                solid: SolidRef::A,
                face: top_a,
                edge: 0,
            },
        );

        for events in [vec![tangency], vec![parab], vec![endpoint]] {
            let out = split_fragments(&shell_a, &shell_b, &events, TOL);
            assert!(
                matches!(
                    out,
                    Err(Refusal::UnsupportedEnvelope(
                        EnvelopeCase::ContactReductionDeferred
                    ))
                ),
                "the deferred locus family refuses with ContactReductionDeferred"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Test 7: an FF-only circle reads OnBoundary against the wall (skipped).
    // ---------------------------------------------------------------------------

    #[test]
    fn split_ff_only_circle_skips_the_on_boundary_wall() {
        // a = the 4x4 block extrude (height 2), b = the disk extrude at (2,2)
        // r=1 (height 2) — the flagship inputs, with ONLY the FF record between
        // a's top face and b's cylinder wall.
        let (profile_a, arr_a) = block_profile();
        let shell_a = extrude_shell(&profile_a, &arr_a, 2.0);
        let (profile_b, arr_b) = disk_profile(Point2::new(2.0, 2.0), 1.0);
        let shell_b = extrude_shell(&profile_b, &arr_b, 2.0);

        // Derivation of the expected fragment structure (BG-NUM-002 checked
        // against the construction): the r=1 circle at (2,2,2) is strictly
        // INSIDE a's top face (the doubled independent loop: a's top face -> the
        // DISK and the ANNULUS) and lies ON b's wall's top rim, so the wall is
        // SKIPPED (not split, keeps its two original self-loop wires). Fragments:
        // a = 1 bottom + 2 top + 4 sides = 7; b = 3 (bottom cap, top cap, wall).
        // Total 10 fragments.
        //   Adjacency: a's disk<->annulus 2 Flip (the fresh circle halves);
        //   a's annulus<->sides 4 Same + bottom<->sides 4 Same + sides<->sides 4
        //   Same = 12 Same; b's wall<->top-cap 1 Same + wall<->bottom-cap 1 Same
        //   = 2 Same (the wall's rims are NOT cut here: there is no FE sewing, so
        //   the wall and the top cap share the single original rim edge). Total 16
        //   entries (2 Flip, 14 Same).
        //   Coincident: EMPTY (no Region2 event). No sewing: the two circle
        //   half-edges appear in a's disk and annulus fragments but NOT in b's
        //   wall or cap wires.

        let top_a = plane_face_at_z(&shell_a, 2.0);
        let wall_b = cylinder_face(&shell_b);
        let cap_b = plane_face_at_z(&shell_b, 2.0);

        let exact = ExactCurve::Circle(placed_circle(Point3::new(2.0, 2.0, 2.0), 1.0));
        let ff = ev(
            ff_curve_record(exact),
            StratumRef::Face {
                solid: SolidRef::A,
                index: top_a,
            },
            StratumRef::Face {
                solid: SolidRef::B,
                index: wall_b,
            },
        );

        let mesh = split_fragments(&shell_a, &shell_b, &[ff], TOL)
            .unwrap()
            .value;

        // Total fragments: 7 from a + 3 from b = 10.
        assert_eq!(mesh.fragments.len(), 10);

        // a's top face becomes exactly two fragments.
        let top_frags = fragments_of_origin(&mesh, SolidRef::A, top_a);
        assert_eq!(top_frags.len(), 2);
        let mut annulus = None;
        let mut disk = None;
        for idx in top_frags {
            let counts = wire_edge_counts(&mesh, idx);
            match counts.as_slice() {
                [2] => disk = Some(idx),
                [4, 2] => annulus = Some(idx),
                other => unreachable!("unexpected top-face wire structure: {other:?}"),
            }
        }
        let annulus = annulus.unwrap();
        let disk = disk.unwrap();
        assert_ne!(annulus, disk);

        // Every other face is exactly one fragment: a's 5 untouched faces plus
        // b's 3 faces (the wall is NOT split).
        assert_eq!(
            fragments_of_origin(&mesh, SolidRef::A, plane_face_at_z(&shell_a, 0.0)).len(),
            1
        );
        for side in 0..4 {
            let idx = 2 + side;
            assert_eq!(fragments_of_origin(&mesh, SolidRef::A, idx).len(), 1);
        }
        assert_eq!(fragments_of_origin(&mesh, SolidRef::B, wall_b).len(), 1);
        assert_eq!(fragments_of_origin(&mesh, SolidRef::B, cap_b).len(), 1);
        assert_eq!(
            fragments_of_origin(&mesh, SolidRef::B, plane_face_at_z(&shell_b, 0.0)).len(),
            1
        );

        // The wall fragment keeps its two original self-loop wires (top rim,
        // bottom rim), one full-circle edge each.
        let wall_frag = fragments_of_origin(&mesh, SolidRef::B, wall_b)[0];
        assert_eq!(wire_edge_counts(&mesh, wall_frag), vec![1, 1]);

        // The two circle half-edge instances appear in a's disk and annulus
        // fragments but NOT in b's wall or cap wires (no sewing event).
        let disk_ids = fragment_edge_ids(&mesh, disk);
        assert_eq!(disk_ids.len(), 2);
        let annulus_hole_ids = mesh.fragments[annulus]
            .face
            .absolute_boundaries()
            .get(1)
            .unwrap()
            .edge_iter()
            .map(|e| e.id())
            .collect::<Vec<_>>();
        let same_pair = |a: &[EdgeID<Curve>], b: &[EdgeID<Curve>]| {
            a.len() == b.len() && a.iter().all(|id| b.contains(id))
        };
        assert!(same_pair(&annulus_hole_ids, &disk_ids));
        let wall_ids = fragment_edge_ids(&mesh, wall_frag);
        assert!(
            !wall_ids.iter().any(|id| disk_ids.contains(id)),
            "the fresh circle halves never appear in b's wall (no sewing)"
        );
        let cap_frag = fragments_of_origin(&mesh, SolidRef::B, cap_b)[0];
        assert!(
            !fragment_edge_ids(&mesh, cap_frag)
                .iter()
                .any(|id| disk_ids.contains(id)),
            "the fresh circle halves never appear in b's cap (no sewing)"
        );

        // No Region2 event, so no coincident pair.
        assert!(mesh.coincident.is_empty());

        // Adjacency: 2 Flip (disk<->annulus, once per half-edge), 14 Same, no
        // cross-solid pair.
        assert_eq!(mesh.adjacency.len(), 16);
        let flips = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Flip)
            .collect::<Vec<_>>();
        assert_eq!(flips.len(), 2);
        for a in &flips {
            assert!(
                (a.lhs == disk && a.rhs == annulus) || (a.lhs == annulus && a.rhs == disk),
                "the only Flip entries are disk<->annulus"
            );
        }
        let sames = mesh
            .adjacency
            .iter()
            .filter(|a| a.parity == AdjacencyParity::Same);
        assert_eq!(sames.count(), 14);
        for a in &mesh.adjacency {
            let lhs_solid = match mesh.fragments[a.lhs].origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            let rhs_solid = match mesh.fragments[a.rhs].origin {
                FragmentOrigin::A { .. } => SolidRef::A,
                FragmentOrigin::B { .. } => SolidRef::B,
            };
            assert_eq!(
                lhs_solid, rhs_solid,
                "adjacency is same-solid only; cross-solid edges are sewing"
            );
        }
    }
}
