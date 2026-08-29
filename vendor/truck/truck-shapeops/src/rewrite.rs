//! BG-CAD-P6-REWRITE + BG-CAD-P7-FILLET — the LocalBoundaryRewrite engine,
//! proven on plane-plane chamfer (P6, Tier 0) and plane-plane fillet (P7,
//! Tier 0).
//!
//! build123d's `chamfer` decomposes as closed-form trim-loci replacement +
//! rewrite (the probe recipe, quoted in the packet). Each spec edge's two
//! adjacent faces are trimmed by a closed-form line offset (D3 step 2), the
//! four trim points are shared across the adjacent faces and the cap faces at
//! the edge's endpoints, and the solid is rebuilt with the original edge
//! instances where they survive and minted shared instances where they do not
//! (D3 step 4). `Solid::try_new` is the acceptance gate (D6).
//!
//! The `fillet` (P7, D1/D2) is the parsimony identity `fillet = offset +
//! Contact + realization + rewrite`: for two planes meeting at a convex edge
//! the rolling-ball contact loci are the two tangent lines — the SAME loci the
//! chamfer trim computes at `d = radius` — and the realized face is a canonical
//! z-axis `Cylinder` (the probe recipe generalized, D2). The three-plane corner
//! (F4, D3) realizes the corner region as a `Sphere` patch: the triple-offset
//! intersection, three junction quarter-arcs shared with the trimmed cylinders.
//!
//! v1 envelope (D3/D4): every face must be a canonical `Plane` carrier with a
//! single convex wire of `Line` edges; each spec edge must have exactly two
//! adjacent faces and box-like endpoints (three incident faces); each trim
//! must stay strictly inside the two adjacent boundary edges. Anything else
//! refuses `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift or
//! `Refusal::Empty` for degenerate requests (D5). The general-dihedral
//! distance-angle form is a booked follow-up; only the right-dihedral form
//! (D4) ships here. A partial corner (two spec edges meeting without the full
//! three-edge triple) refuses `Empty`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::f64::consts::FRAC_PI_2;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::recognize::{
    recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalSurface,
};
use truck_geometry::specifieds::{Cylinder, Line, Plane, Sphere, UnitCircle};
use truck_geotrait::Invertible;
use truck_topology::{Edge, EdgeID, Face, Shell, Solid, Vertex, Wire};

/// The insertion tolerance class (length), shared with the boolean assembler.
const INSERTION_TOL: f64 = 1.0e-2; // H-3: the insertion tolerance class (length)

/// One chamfered straight edge: the edge is named by its two endpoint
/// positions; `d_first` applies to the face whose outward normal is
/// lexicographically SMALLER (x, then y, then z), `d_second` to the other.
#[derive(Clone, Copy, Debug)]
pub struct ChamferSpec {
    /// One endpoint of the edge to chamfer.
    pub a: Point3,
    /// The other endpoint of the edge to chamfer (either order).
    pub b: Point3,
    /// The trim on the adjacent face with the lexicographically smaller
    /// outward normal.
    pub d_first: f64,
    /// The trim on the other adjacent face.
    pub d_second: f64,
}

impl ChamferSpec {
    /// D4 — the right-dihedral distance-angle form: `d` on the first face and
    /// the half-angle `alpha` measured from that face's plane give the second
    /// trim `d * tan(alpha)` (cross-section: trim (d, 0), chamfer line
    /// y = −tan(α)(x−d), hits the second face at d·tan(α)). The
    /// general-dihedral formula is a booked follow-up.
    pub fn by_angle(a: Point3, b: Point3, d: f64, alpha: f64) -> ChamferSpec {
        ChamferSpec {
            a,
            b,
            d_first: d,
            d_second: d * alpha.tan(),
        }
    }
}

/// One filleted straight edge (P7 D1); the radius is the single rolling-ball
/// radius applied to BOTH adjacent faces (fillets are symmetric —
/// distance-distance is the chamfer's business).
#[derive(Clone, Copy, Debug)]
pub struct FilletSpec {
    /// One endpoint of the edge to fillet.
    pub a: Point3,
    /// The other endpoint of the edge to fillet (either order).
    pub b: Point3,
    /// The rolling-ball radius applied to both adjacent faces.
    pub radius: f64,
}

/// The shared spec contract: the two edge endpoints and the trim per adjacent
/// face. The chamfer's two trims and the fillet's symmetric radius both
/// resolve through the same machinery.
trait EdgeSpec: Copy {
    /// One endpoint.
    fn a(&self) -> Point3;
    /// The other endpoint.
    fn b(&self) -> Point3;
    /// The trim applied to `faces[0]` and `faces[1]` (lexicographic normal
    /// order) after resolution.
    fn d(&self) -> [f64; 2];
}

impl EdgeSpec for ChamferSpec {
    fn a(&self) -> Point3 {
        self.a
    }
    fn b(&self) -> Point3 {
        self.b
    }
    fn d(&self) -> [f64; 2] {
        [self.d_first, self.d_second]
    }
}

impl EdgeSpec for FilletSpec {
    fn a(&self) -> Point3 {
        self.a
    }
    fn b(&self) -> Point3 {
        self.b
    }
    fn d(&self) -> [f64; 2] {
        [self.radius; 2]
    }
}

/// The exact bit key of a point: `f64` bits do not make a `Hash`/`Eq` key, so
/// points key as `(u64, u64, u64)`. Coincident dyadic points share one key.
type PointKey = (u64, u64, u64);

/// The cut map: per (edge id, near-vertex point), the trim point lying on that
/// original edge, shared across the adjacent face and the cap face at the
/// vertex.
type CutMap = HashMap<(EdgeID<Curve>, PointKey), Point3>;

/// The vertex pool: per exact point, the one shared `Vertex` instance (the
/// load-bearing instance rule: coincident geometric points share a vertex, or
/// the shell stays open).
type VertexPool = HashMap<PointKey, Vertex<Point3>>;

/// The edge pool: per unordered point pair, the one shared `Edge` instance.
type EdgePool = HashMap<(PointKey, PointKey), Edge<Point3, Curve>>;

/// The `UnsupportedEnvelope(NonCanonicalCarrier)` refusal (D5): at the lift
/// and for ambiguous edge resolution.
fn non_canonical() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

/// A `Solid::try_new`-gate refusal: the reconstructed shell is topologically
/// invalid.
fn invalid_shell() -> Refusal {
    Refusal::Contradictory(ContradictionWitness {
        prop: Prop::CoedgePairing,
        left: Truth::True,
        right: Truth::False,
    })
}

/// One lifted face: the canonical `Plane` carrier, the outward unit normal,
/// and the single convex wire of `Line` edges in stored order.
struct LiftedFace {
    /// The original face, reused verbatim when untouched.
    original: Face<Point3, Curve, Surface>,
    /// The canonical plane carrier.
    plane: Plane,
    /// The outward unit normal (the stored plane's normal, sign-flipped for an
    /// inverted face).
    outward: Vector3,
    /// The wire's polygon vertices in stored order.
    pts: Vec<Point3>,
    /// The wire's edge instances in stored order.
    edges: Vec<Edge<Point3, Curve>>,
    /// Whether the face is stored with its plane's natural orientation.
    orientation: bool,
}

/// D3 step 1 — the lift: every face must be a canonical `Plane` carrier whose
/// stored boundary is a single wire of `Line` edges forming a CONVEX polygon
/// (CCW-positive in the surface frame, the landed invariant). Anything else
/// refuses `NonCanonicalCarrier` before any construction.
fn lift(solid: &Solid<Point3, Curve, Surface>) -> Result<Vec<LiftedFace>, Refusal> {
    let mut out = Vec::new();
    for face in solid.face_iter() {
        let surface = face.surface();
        let plane = match recognize_surface(&surface) {
            CanonicalCarrierWitness::ExactCanonical {
                carrier: CanonicalCarrier::Surface(CanonicalSurface::Plane(plane)),
                ..
            } => plane,
            CanonicalCarrierWitness::Derived {
                carrier: CanonicalCarrier::Surface(CanonicalSurface::Plane(plane)),
                ..
            } => plane,
            _ => return Err(non_canonical()),
        };
        let wires = face.absolute_boundaries();
        if wires.len() != 1 {
            return Err(non_canonical());
        }
        let wire = wires.first().ok_or(non_canonical())?;
        let mut pts = Vec::new();
        let mut edges = Vec::new();
        for edge in wire.edge_iter() {
            match edge.curve() {
                Curve::Line(_) => {
                    pts.push(edge.front().point());
                    edges.push(edge.clone());
                }
                _ => return Err(non_canonical()),
            }
        }
        let n = plane.normal();
        let k = pts.len();
        if k < 3 {
            return Err(non_canonical());
        }
        // Convexity + orientation from the stored wire: every consecutive
        // corner's wedge points the same way as the stored plane's normal.
        for i in 0..k {
            let p0 = *pts.get(i).ok_or(non_canonical())?;
            let p1 = *pts.get((i + 1) % k).ok_or(non_canonical())?;
            let p2 = *pts.get((i + 2) % k).ok_or(non_canonical())?;
            if (p1 - p0).cross(p2 - p1).dot(n) <= 0.0 {
                return Err(non_canonical());
            }
        }
        let orientation = face.orientation();
        let outward = if orientation { n } else { -n };
        out.push(LiftedFace {
            original: face.clone(),
            plane,
            outward,
            pts,
            edges,
            orientation,
        });
    }
    Ok(out)
}

/// Whether `a` is lexicographically smaller than `b` (x, then y, then z).
fn normal_lt(a: &Vector3, b: &Vector3) -> bool {
    (a.x, a.y, a.z) < (b.x, b.y, b.z)
}

/// One resolved spec: the matched edge, its two adjacent faces (`faces[0]`'s
/// outward normal is lexicographically smaller), the trim per face, and the
/// wire position of the edge in each adjacent face.
struct ResolvedSpec {
    edge: Edge<Point3, Curve>,
    faces: [usize; 2],
    d: [f64; 2],
    pos: [usize; 2],
}

/// D2 — resolves every spec edge from the solid's topology: the unique edge
/// whose endpoints match `a`/`b` (either order, within the insertion
/// tolerance). Zero matches refuse `Empty`; multiple matches, a duplicate
/// spec edge, or an abnormal adjacency structure refuse `NonCanonicalCarrier`.
fn resolve<E: EdgeSpec>(lifted: &[LiftedFace], specs: &[E]) -> Result<Vec<ResolvedSpec>, Refusal> {
    let mut uses: HashMap<EdgeID<Curve>, Vec<(usize, usize)>> = HashMap::default();
    for (fi, face) in lifted.iter().enumerate() {
        for (pos, edge) in face.edges.iter().enumerate() {
            uses.entry(edge.id()).or_default().push((fi, pos));
        }
    }
    let mut resolved = Vec::new();
    for spec in specs {
        let mut matches: Vec<EdgeID<Curve>> = Vec::new();
        for (eid, edge_uses) in uses.iter() {
            let (fi, pos) = *edge_uses.first().ok_or(Refusal::Empty)?;
            let rep = lifted
                .get(fi)
                .ok_or(non_canonical())?
                .edges
                .get(pos)
                .ok_or(non_canonical())?;
            let (p0, p1) = (rep.absolute_ends().0.point(), rep.absolute_ends().1.point());
            let near = |x: Point3, y: Point3| (x - y).magnitude() <= INSERTION_TOL;
            let matched = (near(spec.a(), p0) && near(spec.b(), p1))
                || (near(spec.a(), p1) && near(spec.b(), p0));
            if matched {
                matches.push(*eid);
            }
        }
        let eid = match matches.len() {
            0 => return Err(Refusal::Empty),
            1 => *matches.first().ok_or(Refusal::Empty)?,
            _ => return Err(non_canonical()),
        };
        if resolved.iter().any(|r: &ResolvedSpec| r.edge.id() == eid) {
            return Err(Refusal::Empty);
        }
        let edge_uses = uses.get(&eid).ok_or(Refusal::Empty)?;
        if edge_uses.len() != 2 {
            return Err(non_canonical());
        }
        let (fi0, pos0) = *edge_uses.first().ok_or(Refusal::Empty)?;
        let (fi1, pos1) = *edge_uses.get(1).ok_or(Refusal::Empty)?;
        let rep = lifted
            .get(fi0)
            .ok_or(non_canonical())?
            .edges
            .get(pos0)
            .ok_or(non_canonical())?;
        // The spec edge has exactly two adjacent faces (checked above); each
        // endpoint is box-like: exactly three incident faces, so the cap face
        // at the endpoint shares one edge with each adjacent face.
        let (va, vb) = rep.absolute_ends();
        for v in [va, vb] {
            let count = lifted
                .iter()
                .filter(|face| face.pts.contains(&v.point()))
                .count();
            if count != 3 {
                return Err(non_canonical());
            }
        }
        let n0 = lifted.get(fi0).ok_or(non_canonical())?.outward;
        let n1 = lifted.get(fi1).ok_or(non_canonical())?.outward;
        let (faces, pos) = if normal_lt(&n0, &n1) {
            ([fi0, fi1], [pos0, pos1])
        } else {
            ([fi1, fi0], [pos1, pos0])
        };
        let [f0, _f1] = faces;
        let [p0w, _p1w] = pos;
        let edge = lifted
            .get(f0)
            .ok_or(non_canonical())?
            .edges
            .get(p0w)
            .ok_or(non_canonical())?
            .clone();
        resolved.push(ResolvedSpec {
            edge,
            faces,
            d: spec.d(),
            pos,
        });
    }
    Ok(resolved)
}

/// The intersection of the line through `p0` with direction `u` and the
/// `edge`'s segment, strictly inside the segment. A trim line parallel to the
/// edge, or a trim that exits the edge's extent (d reaching an endpoint),
/// refuses `Empty` (D3 step 3).
fn cut_on_segment(p0: Point3, u: Vector3, edge: &Edge<Point3, Curve>) -> Result<Point3, Refusal> {
    let q0 = edge.front().point();
    let q1 = edge.back().point();
    let w = q1 - q0;
    let uw = u.cross(w);
    let denom = uw.dot(uw);
    if denom == 0.0 {
        return Err(Refusal::Empty);
    }
    let s = (q0 - p0).cross(w).dot(uw) / denom;
    let p = p0 + s * u;
    let t = (p - q0).dot(w) / w.dot(w);
    if t <= 0.0 || t >= 1.0 {
        return Err(Refusal::Empty);
    }
    Ok(p)
}

/// The two trim points on one adjacent face: `front` on the edge entering the
/// spec edge's front vertex, `back` on the edge leaving its back vertex.
struct FaceTrims {
    front: Point3,
    back: Point3,
}

/// D3 step 2 — the closed-form trim on one adjacent face: the spec edge's line
/// offset into the polygon interior by `d`, intersected with the face's two
/// boundary edges adjacent to the spec edge. The offset direction is the one
/// whose trims land strictly inside the polygon; the sign rule is
/// `outward × wire_dir`, machine-checked by `cut_on_segment`'s extent check.
fn trim_face(face: &LiftedFace, pos: usize, d: f64) -> Result<FaceTrims, Refusal> {
    let k = face.pts.len();
    let front = *face.pts.get(pos).ok_or(non_canonical())?;
    let back = *face.pts.get((pos + 1) % k).ok_or(non_canonical())?;
    let prev_edge = face.edges.get((pos + k - 1) % k).ok_or(non_canonical())?;
    let next_edge = face.edges.get((pos + 1) % k).ok_or(non_canonical())?;
    let dir = (back - front).normalize();
    let inward = face.outward.cross(dir);
    let p0 = front + d * inward;
    let front_trim = cut_on_segment(p0, dir, prev_edge)?;
    let back_trim = cut_on_segment(p0, dir, next_edge)?;
    Ok(FaceTrims {
        front: front_trim,
        back: back_trim,
    })
}

/// The four trim points of one spec: on `faces[0]` and `faces[1]`, the `front`
/// (prev-edge) and `back` (next-edge) trims.
struct SpecTrims {
    f0: FaceTrims,
    f1: FaceTrims,
}

/// The P6 adjacency rule: a spec position whose wire neighbours are themselves
/// spec edges (two filleted/chamfered edges sharing a vertex in one face, a
/// partial corner) refuses `Empty` in the chamfer and the simple fillet path.
/// The P7 three-edge corner path is the one exception and does not call this.
fn refuse_adjacent_spec_edges(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
) -> Result<(), Refusal> {
    let mut spec_edges: HashSet<EdgeID<Curve>> = HashSet::default();
    for r in resolved {
        spec_edges.insert(r.edge.id());
    }
    for r in resolved {
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        let face0 = lifted.get(f0).ok_or(non_canonical())?;
        let face1 = lifted.get(f1).ok_or(non_canonical())?;
        for (face, pos) in [(face0, p0w), (face1, p1w)] {
            let k = face.pts.len();
            let prev = face.edges.get((pos + k - 1) % k).ok_or(non_canonical())?;
            let next = face.edges.get((pos + 1) % k).ok_or(non_canonical())?;
            if spec_edges.contains(&prev.id()) || spec_edges.contains(&next.id()) {
                return Err(Refusal::Empty);
            }
        }
    }
    Ok(())
}

/// Computes every spec's trim points and records each cut on its original edge
/// keyed by (edge id, near-vertex point), so the cap faces at the spec edge's
/// endpoints share the adjacent faces' cuts. The adjacency refusal is a
/// separate check (`refuse_adjacent_spec_edges`): the chamfer and the simple
/// fillet path refuse, the P7 corner path does not.
fn compute_trims(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
) -> Result<(CutMap, Vec<SpecTrims>), Refusal> {
    let mut cuts: CutMap = HashMap::default();
    let mut all = Vec::new();
    for r in resolved {
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        let [d0, d1] = r.d;
        let face0 = lifted.get(f0).ok_or(non_canonical())?;
        let face1 = lifted.get(f1).ok_or(non_canonical())?;
        let trims0 = trim_face(face0, p0w, d0)?;
        let trims1 = trim_face(face1, p1w, d1)?;
        let k0 = face0.pts.len();
        let front0 = *face0.pts.get(p0w).ok_or(non_canonical())?;
        let back0 = *face0.pts.get((p0w + 1) % k0).ok_or(non_canonical())?;
        let prev0 = face0
            .edges
            .get((p0w + k0 - 1) % k0)
            .ok_or(non_canonical())?;
        let next0 = face0.edges.get((p0w + 1) % k0).ok_or(non_canonical())?;
        cuts.insert((prev0.id(), point_bits(front0)), trims0.front);
        cuts.insert((next0.id(), point_bits(back0)), trims0.back);
        let k1 = face1.pts.len();
        let front1 = *face1.pts.get(p1w).ok_or(non_canonical())?;
        let back1 = *face1.pts.get((p1w + 1) % k1).ok_or(non_canonical())?;
        let prev1 = face1
            .edges
            .get((p1w + k1 - 1) % k1)
            .ok_or(non_canonical())?;
        let next1 = face1.edges.get((p1w + 1) % k1).ok_or(non_canonical())?;
        cuts.insert((prev1.id(), point_bits(front1)), trims1.front);
        cuts.insert((next1.id(), point_bits(back1)), trims1.back);
        all.push(SpecTrims {
            f0: trims0,
            f1: trims1,
        });
    }
    Ok((cuts, all))
}

/// The exact bit key of a point.
fn point_bits(p: Point3) -> PointKey {
    (p.x.to_bits(), p.y.to_bits(), p.z.to_bits())
}

/// The point of an exact bit key.
fn point_from_bits(k: PointKey) -> Point3 {
    Point3::new(
        f64::from_bits(k.0),
        f64::from_bits(k.1),
        f64::from_bits(k.2),
    )
}

/// The canonical order of a point pair, so pools key on an unordered pair.
fn point_pair_key(a: Point3, b: Point3) -> (PointKey, PointKey) {
    let ka = point_bits(a);
    let kb = point_bits(b);
    if ka < kb {
        (ka, kb)
    } else {
        (kb, ka)
    }
}

/// The locus of one fillet arc: the quarter circle from one tangent point to
/// the other, about `center`, in the plane perpendicular to `axis` (the spec
/// edge's direction). Every fillet arc is a quarter circle in the v1 envelope
/// (the two adjacent planes are orthogonal).
#[derive(Clone, Copy, Debug)]
struct ArcData {
    center: Point3,
    axis: Vector3,
    radius: f64,
}

/// The landed revolve arc recipe (D2, read-only reference:
/// `truck-modeling/src/revolve.rs:695-731`) generalized to an arbitrary axis:
/// the trimmed unit circle placed by the affine map whose first two columns
/// are the quarter arc's orthonormal frame scaled by `radius`, third column
/// the axis, and fourth the center. `subs(0) = from`, `subs(π/2) = to`.
fn arc_curve(locus: ArcData, from: Point3, to: Point3) -> Curve {
    let u_vec = (from - locus.center) / locus.radius;
    let v_vec = (to - locus.center) / locus.radius;
    let m = Matrix4 {
        x: Vector4::new(
            locus.radius * u_vec.x,
            locus.radius * u_vec.y,
            locus.radius * u_vec.z,
            0.0,
        ),
        y: Vector4::new(
            locus.radius * v_vec.x,
            locus.radius * v_vec.y,
            locus.radius * v_vec.z,
            0.0,
        ),
        z: Vector4::new(locus.axis.x, locus.axis.y, locus.axis.z, 0.0),
        w: Vector4::new(locus.center.x, locus.center.y, locus.center.z, 1.0),
    };
    Curve::from(Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, FRAC_PI_2)),
        m,
    ))
}

/// The shared construction pools: original vertices by point, minted vertices
/// and edges.
struct Rebuild {
    orig_verts: HashMap<PointKey, Vertex<Point3>>,
    vert_pool: VertexPool,
    edge_pool: EdgePool,
    arc_pool: HashMap<(PointKey, PointKey), Edge<Point3, Curve>>,
}

impl Rebuild {
    /// The one vertex instance for `p`: an original vertex if the point is one,
    /// else a minted instance (deduped by exact point equality).
    fn vertex(&mut self, p: Point3) -> Vertex<Point3> {
        let key = point_bits(p);
        if let Some(v) = self.orig_verts.get(&key) {
            return v.clone();
        }
        if let Some(v) = self.vert_pool.get(&key) {
            return v.clone();
        }
        let v = Vertex::new(p);
        self.vert_pool.insert(key, v.clone());
        v
    }

    /// The one edge instance for the unordered point pair `(a, b)`, minted on
    /// first request and shared (inverted as needed) by the two adjacent
    /// faces. The pool stores each instance oriented low→high in the point
    /// order, so every request returns the same instance oriented for its own
    /// direction. A degenerate pair refuses `Empty`.
    fn edge(&mut self, a: Point3, b: Point3) -> Result<Edge<Point3, Curve>, Refusal> {
        let key = point_pair_key(a, b);
        let (lo, hi) = key;
        let forward = point_bits(a) == lo;
        if let Some(e) = self.edge_pool.get(&key) {
            return Ok(if forward {
                e.clone()
            } else {
                e.inverse().clone()
            });
        }
        let lo_pt = point_from_bits(lo);
        let hi_pt = point_from_bits(hi);
        let vlo = self.vertex(lo_pt);
        let vhi = self.vertex(hi_pt);
        let e = Edge::try_new(&vlo, &vhi, Curve::Line(Line(lo_pt, hi_pt)))
            .map_err(|_| Refusal::Empty)?;
        self.edge_pool.insert(key, e.clone());
        Ok(if forward { e } else { e.inverse().clone() })
    }

    /// The one arc edge instance for the unordered point pair `(a, b)` swept
    /// as a quarter circle about `locus.center` in the plane perpendicular to
    /// `locus.axis`, minted on first request and shared (inverted as needed)
    /// by the cap face, the cylinder face, and (F4) the corner sphere face.
    /// The pool stores each instance oriented low→high in the point order with
    /// the arc curve for that direction, exactly like the line pool, so every
    /// request returns the same instance oriented for its own direction. In
    /// the fillet construction no point pair is ever minted once as a line and
    /// once as an arc.
    fn arc_edge(
        &mut self,
        a: Point3,
        b: Point3,
        locus: ArcData,
    ) -> Result<Edge<Point3, Curve>, Refusal> {
        let key = point_pair_key(a, b);
        let (lo, hi) = key;
        let forward = point_bits(a) == lo;
        if let Some(e) = self.arc_pool.get(&key) {
            return Ok(if forward {
                e.clone()
            } else {
                e.inverse().clone()
            });
        }
        let lo_pt = point_from_bits(lo);
        let hi_pt = point_from_bits(hi);
        let curve = arc_curve(locus, lo_pt, hi_pt);
        let vlo = self.vertex(lo_pt);
        let vhi = self.vertex(hi_pt);
        match Edge::try_new(&vlo, &vhi, curve) {
            Ok(e) => {
                self.arc_pool.insert(key, e.clone());
                Ok(if forward { e } else { e.inverse().clone() })
            }
            Err(_) => Err(Refusal::Empty),
        }
    }

    /// D3 step 4 — rebuilds one trimmed face's polygon: original edges that
    /// survive are reused verbatim; trimmed edges, chamfer segments, and cap
    /// corner segments are minted once and shared through the pools. Returns
    /// `None` for an untouched face (the caller keeps the original). An empty,
    /// inverted, or non-convex kept region refuses `Empty` (D3 step 3). When
    /// `arcs` is `Some`, a cap-corner segment at a spec endpoint whose point
    /// appears in the map is minted as the fillet's quarter-circle arc instead
    /// of a straight chamfer edge.
    fn rebuild_face(
        &mut self,
        face: &LiftedFace,
        spec_positions: &HashSet<usize>,
        cuts: &CutMap,
        arcs: Option<&HashMap<PointKey, ArcData>>,
    ) -> Result<Option<Face<Point3, Curve, Surface>>, Refusal> {
        let k = face.pts.len();
        let mut affected = !spec_positions.is_empty();
        if !affected {
            for i in 0..k {
                let edge = face.edges.get(i).ok_or(non_canonical())?;
                let p0 = *face.pts.get(i).ok_or(non_canonical())?;
                let p1 = *face.pts.get((i + 1) % k).ok_or(non_canonical())?;
                if cuts.contains_key(&(edge.id(), point_bits(p0)))
                    || cuts.contains_key(&(edge.id(), point_bits(p1)))
                {
                    affected = true;
                    break;
                }
            }
        }
        if !affected {
            return Ok(None);
        }
        let enter = |j: usize| -> Option<Point3> {
            let e = face.edges.get((j + k - 1) % k)?;
            let p = face.pts.get(j)?;
            cuts.get(&(e.id(), point_bits(*p))).copied()
        };
        let leave = |j: usize| -> Option<Point3> {
            let e = face.edges.get(j)?;
            let p = face.pts.get(j)?;
            cuts.get(&(e.id(), point_bits(*p))).copied()
        };

        // The polygon: one segment per wire position (the chamfer edge for a
        // spec position), plus the cap-corner segment C_{j+1} inserted after
        // S_j. The order S_0, C_1, S_1, ..., S_{k-1}, C_0 closes by
        // construction (adjacent spec edges were refused).
        let mut pts: Vec<Point3> = Vec::new();
        let mut segments: Vec<Segment> = Vec::new();
        for j in 0..k {
            let j1 = (j + 1) % k;
            let front = *face.pts.get(j).ok_or(non_canonical())?;
            let back = *face.pts.get(j1).ok_or(non_canonical())?;
            let edge = face.edges.get(j).ok_or(non_canonical())?;
            let is_spec = spec_positions.contains(&j);
            let segment = if is_spec {
                let from = enter(j).ok_or(Refusal::Empty)?;
                let to = leave(j1).ok_or(Refusal::Empty)?;
                Segment::New { from, to }
            } else if leave(j).is_some() || enter(j1).is_some() {
                Segment::New {
                    from: leave(j).unwrap_or(front),
                    to: enter(j1).unwrap_or(back),
                }
            } else {
                Segment::Reuse(edge.clone())
            };
            pts.push(segment.from());
            segments.push(segment);
            if let (Some(ec), Some(lc)) = (enter(j1), leave(j1)) {
                if ec != lc {
                    pts.push(ec);
                    let vertex = *face.pts.get(j1).ok_or(non_canonical())?;
                    let segment = match arcs.and_then(|arcs| arcs.get(&point_bits(vertex))) {
                        Some(locus) => Segment::Arc {
                            from: ec,
                            to: lc,
                            locus: *locus,
                        },
                        None => Segment::New { from: ec, to: lc },
                    };
                    segments.push(segment);
                }
            }
        }

        let (poly_pts, poly_edges, surface) = if face.orientation {
            let edges = self.materialize_segments(&segments)?;
            (pts, edges, Surface::Plane(face.plane))
        } else {
            let mut edges = self.materialize_segments(&segments)?;
            edges.reverse();
            for e in edges.iter_mut() {
                e.invert();
            }
            let rev_pts = pts.into_iter().rev().collect::<Vec<Point3>>();
            (rev_pts, edges, Surface::Plane(face.plane.inverse()))
        };

        // The kept region must be a non-degenerate convex polygon in the
        // outward frame.
        let n = face.outward;
        let m = poly_pts.len();
        if m < 3 {
            return Err(Refusal::Empty);
        }
        for i in 0..m {
            let p0 = *poly_pts.get(i).ok_or(non_canonical())?;
            let p1 = *poly_pts.get((i + 1) % m).ok_or(non_canonical())?;
            let p2 = *poly_pts.get((i + 2) % m).ok_or(non_canonical())?;
            if (p1 - p0).cross(p2 - p1).dot(n) <= 0.0 {
                return Err(Refusal::Empty);
            }
        }
        let wire = Wire::from(poly_edges);
        let face = Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?;
        Ok(Some(face))
    }

    /// The chamfer side face of one spec (D3 step 4): the quad connecting the
    /// two trim lines, by the cuboid side pattern
    /// `Plane::new(bottom_start, bottom_end, top_start)` — the chamfer plane
    /// data falls out of the construction exactly. Its four edges are shared
    /// with the two adjacent faces and the two cap faces.
    fn chamfer_face(&mut self, trims: &SpecTrims) -> Result<Face<Point3, Curve, Surface>, Refusal> {
        let a = trims.f0.front;
        let b = trims.f1.back;
        let c = trims.f1.front;
        let d = trims.f0.back;
        let wire = Wire::from(vec![
            self.edge(a, b)?,
            self.edge(b, c)?,
            self.edge(c, d)?,
            self.edge(d, a)?,
        ]);
        let plane = Plane::new(a, b, d);
        let n = plane.normal();
        for (p0, p1, p2) in [(a, b, c), (b, c, d), (c, d, a), (d, a, b)] {
            if (p1 - p0).cross(p2 - p1).dot(n) <= 0.0 {
                return Err(Refusal::Empty);
            }
        }
        let face = Face::try_new(vec![wire], Surface::Plane(plane)).map_err(|_| Refusal::Empty)?;
        Ok(face)
    }

    /// The F4 rebuild of one corner planar face (D3): the two spec positions
    /// (adjacent at the corner) are replaced by their tangent segments, which
    /// meet at the shared corner tangent point; the corner vertex itself is a
    /// regular wire vertex with no arc. Non-spec positions behave exactly as
    /// `rebuild_face`.
    fn corner_face(
        &mut self,
        face: &LiftedFace,
        face_index: usize,
        cuts: &CutMap,
        tangents: &HashMap<(usize, usize), (Point3, Point3)>,
        corner_vertex: Point3,
    ) -> Result<Face<Point3, Curve, Surface>, Refusal> {
        let k = face.pts.len();
        let enter = |j: usize| -> Option<Point3> {
            let e = face.edges.get((j + k - 1) % k)?;
            let p = face.pts.get(j)?;
            cuts.get(&(e.id(), point_bits(*p))).copied()
        };
        let leave = |j: usize| -> Option<Point3> {
            let e = face.edges.get(j)?;
            let p = face.pts.get(j)?;
            cuts.get(&(e.id(), point_bits(*p))).copied()
        };

        let mut pts: Vec<Point3> = Vec::new();
        let mut segments: Vec<Segment> = Vec::new();
        for j in 0..k {
            let j1 = (j + 1) % k;
            let front = *face.pts.get(j).ok_or(non_canonical())?;
            let back = *face.pts.get(j1).ok_or(non_canonical())?;
            let edge = face.edges.get(j).ok_or(non_canonical())?;
            let segment = if let Some((from, to)) = tangents.get(&(face_index, j)) {
                Segment::New {
                    from: *from,
                    to: *to,
                }
            } else if leave(j).is_some() || enter(j1).is_some() {
                Segment::New {
                    from: leave(j).unwrap_or(front),
                    to: enter(j1).unwrap_or(back),
                }
            } else {
                Segment::Reuse(edge.clone())
            };
            pts.push(segment.from());
            segments.push(segment);
            // The corner vertex carries no cap-corner arc: the sphere takes
            // over, and the two tangent segments already meet there.
            let vertex = *face.pts.get(j1).ok_or(non_canonical())?;
            if vertex != corner_vertex {
                if let (Some(ec), Some(lc)) = (enter(j1), leave(j1)) {
                    if ec != lc {
                        pts.push(ec);
                        segments.push(Segment::New { from: ec, to: lc });
                    }
                }
            }
        }

        let (poly_pts, poly_edges, surface) = if face.orientation {
            let edges = self.materialize_segments(&segments)?;
            (pts, edges, Surface::Plane(face.plane))
        } else {
            let mut edges = self.materialize_segments(&segments)?;
            edges.reverse();
            for e in edges.iter_mut() {
                e.invert();
            }
            let rev_pts = pts.into_iter().rev().collect::<Vec<Point3>>();
            (rev_pts, edges, Surface::Plane(face.plane.inverse()))
        };

        let n = face.outward;
        let m = poly_pts.len();
        if m < 3 {
            return Err(Refusal::Empty);
        }
        for i in 0..m {
            let p0 = *poly_pts.get(i).ok_or(non_canonical())?;
            let p1 = *poly_pts.get((i + 1) % m).ok_or(non_canonical())?;
            let p2 = *poly_pts.get((i + 2) % m).ok_or(non_canonical())?;
            if (p1 - p0).cross(p2 - p1).dot(n) <= 0.0 {
                return Err(Refusal::Empty);
            }
        }
        let wire = Wire::from(poly_edges);
        let face = Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?;
        Ok(face)
    }

    /// The F4 realized cylinder of one corner edge (D3): from the cap arc at
    /// the box-like endpoint to the junction arc where the sphere takes over.
    /// The wire direction is the one that pairs every edge opposite to the
    /// planar neighbor face; `tangents` pins the orientation.
    fn corner_cylinder_face(
        &mut self,
        lifted: &[LiftedFace],
        r: &ResolvedSpec,
        trims: &SpecTrims,
        tangents: &HashMap<(usize, usize), (Point3, Point3)>,
        corner_vertex: Point3,
        corner_center: Point3,
    ) -> Result<Face<Point3, Curve, Surface>, Refusal> {
        let radius = r.d[0];
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        let (va, vb) = r.edge.absolute_ends();
        let axis = (vb.point() - va.point()).normalize();
        let face0 = lifted.get(f0).ok_or(non_canonical())?;
        let face1 = lifted.get(f1).ok_or(non_canonical())?;
        let cap = if va.point() == corner_vertex {
            vb.point()
        } else {
            va.point()
        };
        let cap_center = rolling_center(face0, face1, axis, cap, radius)?;
        let c_f1 = cap_side_trim(face1, p1w, &trims.f1, corner_vertex)?;
        let c_f0 = cap_side_trim(face0, p0w, &trims.f0, corner_vertex)?;
        let t_f0 = corner_tangent(face0, p0w, &trims.f0, axis, corner_vertex, corner_center)?;
        let t_f1 = corner_tangent(face1, p1w, &trims.f1, axis, corner_vertex, corner_center)?;
        let f0_use = tangents.get(&(f0, p0w)).copied().ok_or(Refusal::Empty)?;
        let orientation_a = f0_use == (t_f0, c_f0);
        let e0 = (c_f1 - cap_center) / radius;
        let e1 = (c_f0 - cap_center) / radius;
        let surface = cylinder_surface(cap_center, radius, e0, e1, axis)?;
        let cap_locus = ArcData {
            center: cap_center,
            axis,
            radius,
        };
        let junction_locus = ArcData {
            center: corner_center,
            axis,
            radius,
        };
        let wire = if orientation_a {
            Wire::from(vec![
                self.arc_edge(c_f1, c_f0, cap_locus)?,
                self.edge(c_f0, t_f0)?,
                self.arc_edge(t_f0, t_f1, junction_locus)?,
                self.edge(t_f1, c_f1)?,
            ])
        } else {
            Wire::from(vec![
                self.edge(c_f1, t_f1)?,
                self.arc_edge(t_f1, t_f0, junction_locus)?,
                self.edge(t_f0, c_f0)?,
                self.arc_edge(c_f0, c_f1, cap_locus)?,
            ])
        };
        Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)
    }

    /// The F4 sphere patch face (D3): `Surface::Sphere` with the three junction
    /// quarter-arcs as its single closed wire. Each junction arc is shared
    /// (opposite) with the corresponding cylinder; the wire chains into a
    /// cycle through the three corner tangent points, with the pole — the
    /// sphere's parameter-frame `u = 0` boundary — a regular wire vertex.
    fn corner_sphere_face(
        &mut self,
        lifted: &[LiftedFace],
        resolved: &[ResolvedSpec],
        trims: &[SpecTrims],
        tangents: &HashMap<(usize, usize), (Point3, Point3)>,
        corner: &CornerCtx,
    ) -> Result<Face<Point3, Curve, Surface>, Refusal> {
        let radius = resolved
            .get(*corner.specs.first().ok_or(non_canonical())?)
            .ok_or(non_canonical())?
            .d[0];
        let mut sphere_arcs: Vec<(Point3, Point3, ArcData)> = Vec::new();
        for idx in corner.specs.iter() {
            let r = resolved.get(*idx).ok_or(non_canonical())?;
            let trims_i = trims.get(*idx).ok_or(non_canonical())?;
            let [f0, f1] = r.faces;
            let [p0w, p1w] = r.pos;
            let (va, vb) = r.edge.absolute_ends();
            let axis = (vb.point() - va.point()).normalize();
            let face0 = lifted.get(f0).ok_or(non_canonical())?;
            let face1 = lifted.get(f1).ok_or(non_canonical())?;
            let t_f0 = corner_tangent(face0, p0w, &trims_i.f0, axis, corner.vertex, corner.center)?;
            let t_f1 = corner_tangent(face1, p1w, &trims_i.f1, axis, corner.vertex, corner.center)?;
            let f0_use = tangents.get(&(f0, p0w)).copied().ok_or(Refusal::Empty)?;
            let c_f0 = cap_side_trim(face0, p0w, &trims_i.f0, corner.vertex)?;
            let orientation_a = f0_use == (t_f0, c_f0);
            let sphere_arc = if orientation_a {
                (t_f1, t_f0)
            } else {
                (t_f0, t_f1)
            };
            sphere_arcs.push((
                sphere_arc.0,
                sphere_arc.1,
                ArcData {
                    center: corner.center,
                    axis,
                    radius,
                },
            ));
        }
        let ordered = chain_arc_cycle(&sphere_arcs)?;
        let mut edges = Vec::new();
        for (from, to, locus) in ordered {
            edges.push(self.arc_edge(from, to, locus)?);
        }
        let wire = Wire::from(edges);
        let face = Face::try_new(
            vec![wire],
            Surface::Sphere(Sphere::new(corner.center, radius)),
        )
        .map_err(|_| Refusal::Empty)?;
        Ok(face)
    }

    /// Converts the segment list into edge instances.
    fn materialize_segments(
        &mut self,
        segments: &[Segment],
    ) -> Result<Vec<Edge<Point3, Curve>>, Refusal> {
        let mut edges = Vec::new();
        for segment in segments {
            match *segment {
                Segment::Reuse(ref e) => edges.push(e.clone()),
                Segment::New { from, to } => edges.push(self.edge(from, to)?),
                Segment::Arc { from, to, locus } => {
                    edges.push(self.arc_edge(from, to, locus)?);
                }
            }
        }
        Ok(edges)
    }
}

/// One polygon segment of a rebuilt face.
enum Segment {
    /// Reuse the original edge instance, oriented as stored in the wire.
    Reuse(Edge<Point3, Curve>),
    /// Mint (or look up) a new shared edge between the two points.
    New { from: Point3, to: Point3 },
    /// Mint (or look up) the shared fillet quarter-circle arc between the two
    /// tangent points.
    Arc {
        from: Point3,
        to: Point3,
        locus: ArcData,
    },
}

impl Segment {
    fn from(&self) -> Point3 {
        match *self {
            Segment::Reuse(ref e) => e.front().point(),
            Segment::New { from, .. } => from,
            Segment::Arc { from, .. } => from,
        }
    }
}

/// The certificate of a rewrite (chamfer or fillet): the structure is float
/// arithmetic (H-6), claims nothing, and spends no caller budget.
fn rewrite_certificate(budget_left: Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// Orders the three F4 sphere junction arcs into the single closed wire: each
/// arc connects two of the three corner tangent points, and each tangent point
/// belongs to exactly two arcs, so following `from`→`to` closes the cycle.
fn chain_arc_cycle(
    arcs: &[(Point3, Point3, ArcData)],
) -> Result<Vec<(Point3, Point3, ArcData)>, Refusal> {
    let mut remaining = arcs.to_vec();
    let first = remaining.remove(0);
    let mut ordered = vec![first];
    let mut back = first.1;
    let start = first.0;
    while !remaining.is_empty() {
        let idx = remaining
            .iter()
            .position(|(from, _, _)| *from == back)
            .ok_or(Refusal::Empty)?;
        let arc = remaining.remove(idx);
        back = arc.1;
        ordered.push(arc);
    }
    if back != start {
        return Err(Refusal::Empty);
    }
    Ok(ordered)
}

/// D1/D2/D3/D4 — the LocalBoundaryRewrite fillet: the symmetric rolling-ball
/// fillet on plane-plane edges, realized as the quarter cylinder (F1/F2/F3)
/// or, for the three edges of one solid corner, the `Sphere` patch (F4).
/// `Solid::try_new` is the acceptance gate. An empty request list, a
/// non-positive radius, or a partial corner refuses `Empty`; a non-plane or
/// non-convex solid refuses `UnsupportedEnvelope(NonCanonicalCarrier)` at the
/// lift before any construction.
pub fn fillet(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[FilletSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    if specs.is_empty() {
        return Err(Refusal::Empty);
    }
    for spec in specs {
        if !spec.radius.is_finite() || spec.radius <= 0.0 {
            return Err(Refusal::Empty);
        }
    }
    let lifted = lift(solid)?;
    let resolved = resolve(&lifted, specs)?;
    let (cuts, trims) = compute_trims(&lifted, &resolved)?;

    let radius = resolved.first().ok_or(non_canonical())?.d[0];
    match detect_corner(&lifted, &resolved, radius)? {
        Some(corner) => fillet_corner(&lifted, &resolved, &trims, &cuts, &corner, budget),
        None => {
            refuse_adjacent_spec_edges(&lifted, &resolved)?;
            fillet_edges(&lifted, &resolved, &cuts, &trims, budget)
        }
    }
}

/// D1/D3/D4 — the LocalBoundaryRewrite chamfer: lift, resolve, trim, rebuild,
/// with `Solid::try_new` as the acceptance gate. An empty request list refuses
/// `Empty`; a non-plane/non-convex solid refuses
/// `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift before any
/// construction; two chamfered edges sharing a vertex (adjacent spec edges)
/// refuse `Empty`.
pub fn chamfer(
    solid: &Solid<Point3, Curve, Surface>,
    specs: &[ChamferSpec],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    if specs.is_empty() {
        return Err(Refusal::Empty);
    }
    let lifted = lift(solid)?;
    let resolved = resolve(&lifted, specs)?;
    refuse_adjacent_spec_edges(&lifted, &resolved)?;

    let mut spec_positions: Vec<HashSet<usize>> = vec![HashSet::default(); lifted.len()];
    for r in resolved.iter() {
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        if let Some(set) = spec_positions.get_mut(f0) {
            set.insert(p0w);
        }
        if let Some(set) = spec_positions.get_mut(f1) {
            set.insert(p1w);
        }
    }

    let (cuts, trims) = compute_trims(&lifted, &resolved)?;

    let mut orig_verts: HashMap<PointKey, Vertex<Point3>> = HashMap::default();
    for face in &lifted {
        for (edge, pt) in face.edges.iter().zip(face.pts.iter()) {
            orig_verts.insert(point_bits(*pt), edge.front().clone());
        }
    }
    let mut rebuild = Rebuild {
        orig_verts,
        vert_pool: HashMap::default(),
        edge_pool: HashMap::default(),
        arc_pool: HashMap::default(),
    };

    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    for (fi, face) in lifted.iter().enumerate() {
        let positions = spec_positions.get(fi).ok_or(non_canonical())?;
        match rebuild.rebuild_face(face, positions, &cuts, None)? {
            Some(new_face) => faces.push(new_face),
            None => faces.push(face.original.clone()),
        }
    }
    for trims in &trims {
        faces.push(rebuild.chamfer_face(trims)?);
    }

    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let result = Solid::try_new(vec![shell]).map_err(|_| invalid_shell())?;
    Ok(Certified::new(result, rewrite_certificate(*budget)))
}

// ---------------------------------------------------------------------------
// BG-CAD-P7-FILLET — the plane-plane fillet on the rewrite engine.
// ---------------------------------------------------------------------------

/// The rolling-ball center at a spec endpoint (D2): the point at distance
/// `radius` from BOTH adjacent face planes on the material side, projected
/// onto the edge's normal plane at the endpoint. The 3×3 linear system
/// `c·n0 = o0·n0 − r·s0, c·n1 = o1·n1 − r·s1, c·axis = endpoint·axis` is
/// solved exactly by the dual-basis formula; a dependent system (non-right
/// dihedral) refuses `Empty`.
fn rolling_center(
    face0: &LiftedFace,
    face1: &LiftedFace,
    axis: Vector3,
    endpoint: Point3,
    radius: f64,
) -> Result<Point3, Refusal> {
    let n0 = face0.plane.normal();
    let o0 = face0.plane.origin();
    let s0 = face0.outward.dot(n0);
    let n1 = face1.plane.normal();
    let o1 = face1.plane.origin();
    let s1 = face1.outward.dot(n1);
    let b0 = o0.x * n0.x + o0.y * n0.y + o0.z * n0.z - radius * s0;
    let b1 = o1.x * n1.x + o1.y * n1.y + o1.z * n1.z - radius * s1;
    let b2 = endpoint.x * axis.x + endpoint.y * axis.y + endpoint.z * axis.z;
    let det = n0.dot(n1.cross(axis));
    if det == 0.0 {
        return Err(Refusal::Empty);
    }
    let v = (b0 * n1.cross(axis) + b1 * axis.cross(n0) + b2 * n0.cross(n1)) / det;
    Ok(Point3::new(v.x, v.y, v.z))
}

/// The F4 corner sphere center (D3): the triple offset — distance `radius`
/// from all three corner face planes on the material side. The three corner
/// normals of a right-dihedral trihedral are independent, so the dual-basis
/// formula is exact; a dependent system refuses `Empty`.
fn triple_offset(faces: &[&LiftedFace; 3], radius: f64) -> Result<Point3, Refusal> {
    let mut rows = [[0.0; 3]; 3];
    let mut b = [0.0; 3];
    for ((row, bslot), face) in rows.iter_mut().zip(b.iter_mut()).zip(faces.iter()) {
        let n = face.plane.normal();
        let o = face.plane.origin();
        let s = face.outward.dot(n);
        *row = [n.x, n.y, n.z];
        *bslot = o.x * n.x + o.y * n.y + o.z * n.z - radius * s;
    }
    let [r0, r1, r2] = rows;
    let n0 = Vector3::new(r0[0], r0[1], r0[2]);
    let n1 = Vector3::new(r1[0], r1[1], r1[2]);
    let n2 = Vector3::new(r2[0], r2[1], r2[2]);
    let det = n0.dot(n1.cross(n2));
    if det == 0.0 {
        return Err(Refusal::Empty);
    }
    let v = (b[0] * n1.cross(n2) + b[1] * n2.cross(n0) + b[2] * n0.cross(n1)) / det;
    Ok(Point3::new(v.x, v.y, v.z))
}

/// The validated cylinder carrier (A4); the entry validated `radius > 0`, so
/// `Cylinder::new` cannot refuse here.
fn make_cylinder(center: Point3, radius: f64) -> Result<Cylinder, Refusal> {
    match Cylinder::new(center, radius) {
        Ok(c) => Ok(c.value),
        Err(_) => Err(Refusal::Empty),
    }
}

/// The fillet cylinder's surface (D2): the canonical z-axis carrier when the
/// spec edge is z-aligned (the probe's canonical form), else a `Processor`
/// placing the carrier so `subs(u, v) = center + r(cos u·e0 + sin u·e1) +
/// v·axis` — the same affine frame the arc recipe uses.
fn cylinder_surface(
    center: Point3,
    radius: f64,
    e0: Vector3,
    e1: Vector3,
    axis: Vector3,
) -> Result<Surface, Refusal> {
    let cyl = make_cylinder(Point3::new(0.0, 0.0, 0.0), radius)?;
    if axis == Vector3::unit_z() {
        return Ok(Surface::Cylinder(make_cylinder(center, radius)?));
    }
    let m = Matrix4 {
        x: Vector4::new(radius * e0.x, radius * e0.y, radius * e0.z, 0.0),
        y: Vector4::new(radius * e1.x, radius * e1.y, radius * e1.z, 0.0),
        z: Vector4::new(axis.x, axis.y, axis.z, 0.0),
        w: Vector4::new(center.x, center.y, center.z, 1.0),
    };
    Ok(Surface::Processor(Processor::with_transform(
        Box::new(Surface::Cylinder(cyl)),
        m,
    )))
}

/// The cap-side trim on one adjacent face at the box-like (non-corner)
/// endpoint of a corner edge: the wire position's front vertex is either the
/// corner or the cap, so the cap-side trim is the back trim when the wire
/// front is the corner and the front trim otherwise.
fn cap_side_trim(
    face: &LiftedFace,
    pos: usize,
    trims_side: &FaceTrims,
    corner: Point3,
) -> Result<Point3, Refusal> {
    let front_vertex = face.pts.get(pos).copied().ok_or_else(non_canonical)?;
    if front_vertex == corner {
        Ok(trims_side.back)
    } else {
        Ok(trims_side.front)
    }
}

/// The two trim points of one adjacent face, split by the spec edge's absolute
/// ends: the wire position's front vertex is the spec edge's front end `va` in
/// exactly one of the two adjacent faces and its back end `vb` in the other,
/// so the mapping from `trims` to the edge ends depends on the wire
/// orientation. Returns `(point_at_va, point_at_vb)`.
fn end_points(
    face: &LiftedFace,
    pos: usize,
    trims_side: &FaceTrims,
    va: Point3,
) -> Result<(Point3, Point3), Refusal> {
    let front_vertex = face.pts.get(pos).copied().ok_or_else(non_canonical)?;
    if front_vertex == va {
        Ok((trims_side.front, trims_side.back))
    } else {
        Ok((trims_side.back, trims_side.front))
    }
}

/// The F4 corner tangent point on one adjacent face (D3): the point where the
/// face's tangent line meets the sphere∩cylinder junction — the projection of
/// the sphere center `center` onto the tangent line.
fn corner_tangent(
    face: &LiftedFace,
    pos: usize,
    trims_side: &FaceTrims,
    axis: Vector3,
    corner: Point3,
    center: Point3,
) -> Result<Point3, Refusal> {
    let cap_trim = cap_side_trim(face, pos, trims_side, corner)?;
    Ok(cap_trim + (center - cap_trim).dot(axis) * axis)
}

/// The shared `Rebuild` pool initialization.
fn rebuild_from_lifted(lifted: &[LiftedFace]) -> Rebuild {
    let mut orig_verts: HashMap<PointKey, Vertex<Point3>> = HashMap::default();
    for face in lifted {
        for (edge, pt) in face.edges.iter().zip(face.pts.iter()) {
            orig_verts.insert(point_bits(*pt), edge.front().clone());
        }
    }
    Rebuild {
        orig_verts,
        vert_pool: HashMap::default(),
        edge_pool: HashMap::default(),
        arc_pool: HashMap::default(),
    }
}

/// The P7 simple path — the F1/F2/F3 envelope: each spec edge is an
/// independent fillet whose realized face is the quarter cylinder, and the cap
/// faces' corner strips are the quarter arcs. Every planar face is rebuilt via
/// `rebuild_face` with the arc map; `Solid::try_new` is the acceptance gate.
fn fillet_edges(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
    cuts: &CutMap,
    trims: &[SpecTrims],
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let mut spec_positions: Vec<HashSet<usize>> = vec![HashSet::default(); lifted.len()];
    for r in resolved.iter() {
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        if let Some(set) = spec_positions.get_mut(f0) {
            set.insert(p0w);
        }
        if let Some(set) = spec_positions.get_mut(f1) {
            set.insert(p1w);
        }
    }
    let mut arcs: HashMap<PointKey, ArcData> = HashMap::default();
    for r in resolved.iter() {
        let radius = r.d[0];
        let (va, vb) = r.edge.absolute_ends();
        let axis = (vb.point() - va.point()).normalize();
        let [f0, f1] = r.faces;
        let face0 = lifted.get(f0).ok_or(non_canonical())?;
        let face1 = lifted.get(f1).ok_or(non_canonical())?;
        let center_a = rolling_center(face0, face1, axis, va.point(), radius)?;
        let center_b = rolling_center(face0, face1, axis, vb.point(), radius)?;
        arcs.insert(
            point_bits(va.point()),
            ArcData {
                center: center_a,
                axis,
                radius,
            },
        );
        arcs.insert(
            point_bits(vb.point()),
            ArcData {
                center: center_b,
                axis,
                radius,
            },
        );
    }

    let mut rebuild = rebuild_from_lifted(lifted);
    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    for (fi, face) in lifted.iter().enumerate() {
        let positions = spec_positions.get(fi).ok_or(non_canonical())?;
        match rebuild.rebuild_face(face, positions, cuts, Some(&arcs))? {
            Some(new_face) => faces.push(new_face),
            None => faces.push(face.original.clone()),
        }
    }
    for (r, trims_i) in resolved.iter().zip(trims.iter()) {
        faces.push(simple_cylinder_face(lifted, r, trims_i, &mut rebuild)?);
    }

    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let result = Solid::try_new(vec![shell]).map_err(|_| invalid_shell())?;
    Ok(Certified::new(result, rewrite_certificate(*budget)))
}

/// The realized quarter-cylinder face of one independent fillet edge (D2): the
/// wire follows the probe's cuboid side pattern in the (u, v) frame — bottom
/// arc forward at v = 0, up at u = π/2, top arc inverse at v = L, down at
/// u = 0 — with the surface normal outward.
fn simple_cylinder_face(
    lifted: &[LiftedFace],
    r: &ResolvedSpec,
    trims: &SpecTrims,
    rebuild: &mut Rebuild,
) -> Result<Face<Point3, Curve, Surface>, Refusal> {
    let radius = r.d[0];
    let [f0, f1] = r.faces;
    let (va, vb) = r.edge.absolute_ends();
    let axis = (vb.point() - va.point()).normalize();
    let face0 = lifted.get(f0).ok_or(non_canonical())?;
    let face1 = lifted.get(f1).ok_or(non_canonical())?;
    let center_a = rolling_center(face0, face1, axis, va.point(), radius)?;
    let center_b = rolling_center(face0, face1, axis, vb.point(), radius)?;
    let (p_f1_va, p_f1_vb) = end_points(face1, r.pos[1], &trims.f1, va.point())?;
    let (p_f0_va, p_f0_vb) = end_points(face0, r.pos[0], &trims.f0, va.point())?;
    let e0 = (p_f1_va - center_a) / radius;
    let e1 = (p_f0_va - center_a) / radius;
    let surface = cylinder_surface(center_a, radius, e0, e1, axis)?;
    let arc_va = ArcData {
        center: center_a,
        axis,
        radius,
    };
    let arc_vb = ArcData {
        center: center_b,
        axis,
        radius,
    };
    // The wire must pair every edge opposite to the adjacent face. The two
    // cyclic orders differ by whether the shared vertex of the two spec-edge
    // positions in `f0` is the edge's front end `va` (the (0,0)-style corner,
    // where the realized cylinder bulges toward −x/−y) or its back end `vb`
    // (the probe's (4,4) witness). `end_points` already mapped the trims by
    // the wire orientation, so `f0`'s wire-front vertex decides.
    let wire = if face0.pts.get(r.pos[0]).copied() == Some(va.point()) {
        Wire::from(vec![
            rebuild.arc_edge(p_f0_va, p_f1_va, arc_va)?,
            rebuild.edge(p_f1_va, p_f1_vb)?,
            rebuild.arc_edge(p_f1_vb, p_f0_vb, arc_vb)?,
            rebuild.edge(p_f0_vb, p_f0_va)?,
        ])
    } else {
        Wire::from(vec![
            rebuild.arc_edge(p_f1_va, p_f0_va, arc_va)?,
            rebuild.edge(p_f0_va, p_f0_vb)?,
            rebuild.arc_edge(p_f0_vb, p_f1_vb, arc_vb)?,
            rebuild.edge(p_f1_vb, p_f1_va)?,
        ])
    };
    Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)
}

// ---------------------------------------------------------------------------
// F4: the three-plane corner sphere.
// ---------------------------------------------------------------------------

/// The resolved F4 corner construction.
struct CornerCtx {
    /// The shared corner vertex.
    vertex: Point3,
    /// The sphere center (the triple offset).
    center: Point3,
    /// The three corner spec indices.
    specs: Vec<usize>,
    /// The three distinct corner faces.
    faces: Vec<usize>,
}

/// Detects the F4 corner triple among the resolved specs: a vertex shared by
/// exactly three specs whose adjacent faces at the corner are exactly three
/// distinct faces, each shared by exactly two of the three corner edges.
fn detect_corner(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
    radius: f64,
) -> Result<Option<CornerCtx>, Refusal> {
    let mut endpoint_specs: HashMap<PointKey, Vec<usize>> = HashMap::default();
    for (i, r) in resolved.iter().enumerate() {
        let (va, vb) = r.edge.absolute_ends();
        endpoint_specs
            .entry(point_bits(va.point()))
            .or_default()
            .push(i);
        endpoint_specs
            .entry(point_bits(vb.point()))
            .or_default()
            .push(i);
    }
    // A vertex with exactly two specs is a partial corner: refuse `Empty`
    // (D4). More than three spec edges at one vertex is abnormal.
    for specs in endpoint_specs.values() {
        match specs.len() {
            2 => return Err(Refusal::Empty),
            3 => {}
            _ if specs.len() > 3 => return Err(non_canonical()),
            _ => {}
        }
    }
    let (corner_key, corner_specs) = match endpoint_specs.iter().find(|(_, specs)| specs.len() == 3)
    {
        Some(entry) => (entry.0, entry.1.clone()),
        None => return Ok(None),
    };
    if corner_specs.len() != resolved.len() {
        return Err(Refusal::Empty);
    }
    // The F4 corner sphere is the triple offset at ONE radius (D3): the three
    // corner edges must share the same radius, or the realization is not a
    // sphere patch and refuses `Empty`.
    for idx in corner_specs.iter() {
        if resolved.get(*idx).ok_or(non_canonical())?.d[0] != radius {
            return Err(Refusal::Empty);
        }
    }
    let vertex = point_from_bits(*corner_key);
    // The three corner faces: distinct, each shared by exactly two edges.
    let mut corner_faces: Vec<usize> = Vec::new();
    for idx in corner_specs.iter() {
        let r = resolved.get(*idx).ok_or(non_canonical())?;
        for f in r.faces {
            if !corner_faces.contains(&f) {
                corner_faces.push(f);
            }
        }
    }
    if corner_faces.len() != 3 {
        return Err(non_canonical());
    }
    let face_refs: [&LiftedFace; 3] = [
        lifted
            .get(*corner_faces.first().ok_or(non_canonical())?)
            .ok_or(non_canonical())?,
        lifted
            .get(*corner_faces.get(1).ok_or(non_canonical())?)
            .ok_or(non_canonical())?,
        lifted
            .get(*corner_faces.get(2).ok_or(non_canonical())?)
            .ok_or(non_canonical())?,
    ];
    let center = triple_offset(&face_refs, radius)?;
    Ok(Some(CornerCtx {
        vertex,
        center,
        specs: corner_specs,
        faces: corner_faces,
    }))
}

/// The P7 corner path (D3): three spec edges meeting at a solid corner realize
/// the corner region as the `Sphere` patch, each cylinder is trimmed at its
/// junction circle, and the three planar faces trim at their tangent lines to
/// the corner-adjacent tangent points.
fn fillet_corner(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
    trims: &[SpecTrims],
    cuts: &CutMap,
    corner: &CornerCtx,
    budget: &mut Budget,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let radius = resolved.first().ok_or(non_canonical())?.d[0];
    // The per-(face, pos) tangent segments on the corner faces, and the cap
    // arc loci at the box-like endpoints.
    let mut tangents: HashMap<(usize, usize), (Point3, Point3)> = HashMap::default();
    let mut arcs: HashMap<PointKey, ArcData> = HashMap::default();
    for idx in corner.specs.iter() {
        let r = resolved.get(*idx).ok_or(non_canonical())?;
        let trims_i = trims.get(*idx).ok_or(non_canonical())?;
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        let (va, vb) = r.edge.absolute_ends();
        let axis = (vb.point() - va.point()).normalize();
        let face0 = lifted.get(f0).ok_or(non_canonical())?;
        let face1 = lifted.get(f1).ok_or(non_canonical())?;
        let cap = if va.point() == corner.vertex {
            vb.point()
        } else {
            va.point()
        };
        let cap_center = rolling_center(face0, face1, axis, cap, radius)?;
        arcs.insert(
            point_bits(cap),
            ArcData {
                center: cap_center,
                axis,
                radius,
            },
        );
        for (face_idx, pos, face_trims) in [(f0, p0w, &trims_i.f0), (f1, p1w, &trims_i.f1)] {
            let front_vertex = lifted
                .get(face_idx)
                .ok_or(non_canonical())?
                .pts
                .get(pos)
                .copied()
                .ok_or(non_canonical())?;
            let cap_trim = cap_side_trim(
                lifted.get(face_idx).ok_or(non_canonical())?,
                pos,
                face_trims,
                corner.vertex,
            )?;
            let t = cap_trim + (corner.center - cap_trim).dot(axis) * axis;
            let segment = if front_vertex == corner.vertex {
                (t, cap_trim)
            } else {
                (cap_trim, t)
            };
            tangents.insert((face_idx, pos), segment);
        }
    }

    let mut rebuild = rebuild_from_lifted(lifted);
    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    for (fi, face) in lifted.iter().enumerate() {
        if corner.faces.contains(&fi) {
            faces.push(rebuild.corner_face(face, fi, cuts, &tangents, corner.vertex)?);
        } else {
            let empty = HashSet::default();
            match rebuild.rebuild_face(face, &empty, cuts, Some(&arcs))? {
                Some(new_face) => faces.push(new_face),
                None => faces.push(face.original.clone()),
            }
        }
    }
    for idx in corner.specs.iter() {
        let r = resolved.get(*idx).ok_or(non_canonical())?;
        let trims_i = trims.get(*idx).ok_or(non_canonical())?;
        faces.push(rebuild.corner_cylinder_face(
            lifted,
            r,
            trims_i,
            &tangents,
            corner.vertex,
            corner.center,
        )?);
    }
    faces.push(rebuild.corner_sphere_face(lifted, resolved, trims, &tangents, corner)?);

    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let result = Solid::try_new(vec![shell]).map_err(|_| invalid_shell())?;
    Ok(Certified::new(result, rewrite_certificate(*budget)))
}
