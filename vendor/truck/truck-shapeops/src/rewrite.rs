//! BG-CAD-P6-REWRITE — the LocalBoundaryRewrite engine, proven on plane-plane
//! chamfer (docs/BUILD123D_COVERAGE_PLAN.md P6, Tier 0).
//!
//! build123d's `chamfer` decomposes as closed-form trim-loci replacement +
//! rewrite (the probe recipe, quoted in the packet). Each spec edge's two
//! adjacent faces are trimmed by a closed-form line offset (D3 step 2), the
//! four trim points are shared across the adjacent faces and the cap faces at
//! the edge's endpoints, and the solid is rebuilt with the original edge
//! instances where they survive and minted shared instances where they do not
//! (D3 step 4). `Solid::try_new` is the acceptance gate (D6).
//!
//! v1 envelope (D3/D4): every face must be a canonical `Plane` carrier with a
//! single convex wire of `Line` edges; each spec edge must have exactly two
//! adjacent faces and box-like endpoints (three incident faces); each trim
//! must stay strictly inside the two adjacent boundary edges. Anything else
//! refuses `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift or
//! `Refusal::Empty` for degenerate requests (D5). The general-dihedral
//! distance-angle form is a booked follow-up; only the right-dihedral form
//! (D4) ships here.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::recognize::{
    recognize_surface, CanonicalCarrier, CanonicalCarrierWitness, CanonicalSurface,
};
use truck_geometry::specifieds::{Line, Plane};
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
fn resolve(lifted: &[LiftedFace], specs: &[ChamferSpec]) -> Result<Vec<ResolvedSpec>, Refusal> {
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
            let matched =
                (near(spec.a, p0) && near(spec.b, p1)) || (near(spec.a, p1) && near(spec.b, p0));
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
            d: [spec.d_first, spec.d_second],
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

/// Computes every spec's trim points and records each cut on its original edge
/// keyed by (edge id, near-vertex point), so the cap faces at the spec edge's
/// endpoints share the adjacent faces' cuts. Two adjacent spec edges in a
/// face's wire (the shared-vertex chamfer) refuse `Empty`.
fn compute_trims(
    lifted: &[LiftedFace],
    resolved: &[ResolvedSpec],
) -> Result<(CutMap, Vec<SpecTrims>), Refusal> {
    let mut spec_edges: HashSet<EdgeID<Curve>> = HashSet::default();
    for r in resolved {
        spec_edges.insert(r.edge.id());
    }
    let mut cuts: CutMap = HashMap::default();
    let mut all = Vec::new();
    for r in resolved {
        let [f0, f1] = r.faces;
        let [p0w, p1w] = r.pos;
        let [d0, d1] = r.d;
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

/// The shared construction pools: original vertices by point, minted vertices
/// and edges.
struct Rebuild {
    orig_verts: HashMap<PointKey, Vertex<Point3>>,
    vert_pool: VertexPool,
    edge_pool: EdgePool,
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

    /// D3 step 4 — rebuilds one trimmed face's polygon: original edges that
    /// survive are reused verbatim; trimmed edges, chamfer segments, and cap
    /// corner segments are minted once and shared through the pools. Returns
    /// `None` for an untouched face (the caller keeps the original). An empty,
    /// inverted, or non-convex kept region refuses `Empty` (D3 step 3).
    fn rebuild_face(
        &mut self,
        face: &LiftedFace,
        spec_positions: &HashSet<usize>,
        cuts: &CutMap,
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
                    segments.push(Segment::New { from: ec, to: lc });
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
}

impl Segment {
    fn from(&self) -> Point3 {
        match *self {
            Segment::Reuse(ref e) => e.front().point(),
            Segment::New { from, .. } => from,
        }
    }
}

/// The certificate of a chamfer: the structure is float arithmetic (H-6),
/// claims nothing, and spends no caller budget.
fn chamfer_certificate(budget_left: Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// D1/D3/D4 — the LocalBoundaryRewrite chamfer: lift, resolve, trim, rebuild,
/// with `Solid::try_new` as the acceptance gate. An empty request list refuses
/// `Empty`; a non-plane/non-convex solid refuses
/// `UnsupportedEnvelope(NonCanonicalCarrier)` at the lift before any
/// construction.
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
    };

    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();
    for (fi, face) in lifted.iter().enumerate() {
        let positions = spec_positions.get(fi).ok_or(non_canonical())?;
        match rebuild.rebuild_face(face, positions, &cuts)? {
            Some(new_face) => faces.push(new_face),
            None => faces.push(face.original.clone()),
        }
    }
    for trims in &trims {
        faces.push(rebuild.chamfer_face(trims)?);
    }

    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let result = Solid::try_new(vec![shell]).map_err(|_| invalid_shell())?;
    Ok(Certified::new(result, chamfer_certificate(*budget)))
}
