//! BG-CAD-P3-SPLIT — section + split by plane via the landed Boolean.
//!
//! build123d's `split(S, Pi)` and `section()` are the next Tier 0 operations
//! (docs/BUILD123D_COVERAGE_PLAN.md P3). The parsimony identity
//! `split(S, Pi) = Contact + classify + caps + rewrite` is ALREADY landed as
//! the certified 3-D Boolean, so this module only:
//!
//! - lifts the solid to a conservative over-box (D2),
//! - constructs the two halfspace solids over the cutting plane (D3),
//! - composes the two `boolean()` calls (D4), and
//! - extracts the cap faces of the wall plane by exact `Plane` identity (D6).
//!
//! No cutting, classifying, or capping machinery is written here; every
//! emitted solid and face rides the landed material-state pipeline and its
//! `Solid::try_new` acceptance gate (D6). `plus` and `minus` are the two
//! `boolean()` results against the negative halfspace box (`Difference` for
//! the side the plane's normal points to, `Intersection` for the other), so
//! the booked metamorphic `split_+ ∪ split_- ≅ S` falls out of the landed
//! assembler.
//!
//! v1 envelope (D5): a non-canonical solid face refuses
//! `UnsupportedEnvelope(NonCanonicalCarrier)` at the over-box lift, before
//! any Boolean is paid for; an oblique plane over a cylinder wall lets the
//! landed RW-CONIC refusal surface from inside `boolean()` (the booked
//! `Curve`-ellipse follow-up); coplanar and unrecognized profile carriers
//! answer exactly what the landed entry answers.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use rustc_hash::FxHashMap as HashMap;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3, Zero};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, EnvelopeCase, Margin, Method, Modulus,
    Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_evidence::enclosure::{Box3, EnclosureCurve, Interval};
use truck_geometry::canonical::{Curve, Surface};
use truck_geometry::recognize::{recognize_surface, CanonicalCarrier, CanonicalCarrierWitness};
use truck_geometry::specifieds::{Line, Plane};
use truck_geotrait::BoundedCurve;
use truck_topology::{Edge, Face, Shell, Solid, Vertex, Wire};

use crate::boolean::assemble::boolean;
use crate::boolean::BoolOp;

/// The three coordinate ranges of an axis-aligned box.
type Ranges = ((f64, f64), (f64, f64), (f64, f64));

/// The shared-edge pool of one box construction: one `Edge` instance per
/// unordered vertex-index pair, so the two faces on each edge share the
/// instance (opposite orientations) and the shell closes.
type EdgePool = HashMap<(usize, usize), Edge<Point3, Curve>>;

/// The non-canonical-carrier refusal (D2, at the over-box lift).
fn non_canonical() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

/// A `Solid::try_new`-gate refusal: the constructed box is topologically
/// invalid (the extruded-box construction recipe's gate).
fn invalid_shell() -> Refusal {
    Refusal::Contradictory(ContradictionWitness {
        prop: Prop::CoedgePairing,
        left: Truth::True,
        right: Truth::False,
    })
}

/// D2 — the solid's over-box, a local helper (truck-shapeops does not depend
/// on truck-modeling, so the per-face carrier table of P1's D2 is
/// reimplemented here; adding that dependency would invert the layering).
///
/// Per-face carrier table:
/// - `Plane` / `Cylinder` faces → the hull of the boundary edges'
///   `EnclosureCurve::enclose` boxes over each edge's own range.
/// - `Sphere` faces → the full carrier box `[c-r, c+r]^3`.
/// - `Cone` faces → the hull of the boundary edge boxes plus the apex.
/// - `Torus`, `CanonicalSurface::Placed`, `Unrecognized` →
///   `UnsupportedEnvelope(NonCanonicalCarrier)`.
///
/// Stored wires via `Face::absolute_boundaries()` (session-38 naming trap:
/// `Face::boundaries()` is orientation-flipped, `absolute_boundaries()` is
/// the stored one).
fn solid_over_box(solid: &Solid<Point3, Curve, Surface>) -> Outcome<Ranges> {
    let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for face in solid.face_iter() {
        match recognize_surface(&face.surface()) {
            CanonicalCarrierWitness::Unrecognized => return Err(non_canonical()),
            CanonicalCarrierWitness::Derived {
                carrier: CanonicalCarrier::Surface(_),
                ..
            } => {
                // The only derived surface carrier is `Placed` (recognize.rs);
                // a placed analytic carrier is outside the canonical set (D2).
                return Err(non_canonical());
            }
            CanonicalCarrierWitness::ExactCanonical {
                carrier:
                    CanonicalCarrier::Surface(truck_geometry::recognize::CanonicalSurface::Torus(_)),
                ..
            } => return Err(non_canonical()),
            CanonicalCarrierWitness::ExactCanonical {
                carrier:
                    CanonicalCarrier::Surface(truck_geometry::recognize::CanonicalSurface::Sphere(
                        sphere,
                    )),
                ..
            } => {
                let r = sphere.radius();
                let c = sphere.center();
                grow_point(&mut lo, &mut hi, Point3::new(c.x - r, c.y - r, c.z - r));
                grow_point(&mut lo, &mut hi, Point3::new(c.x + r, c.y + r, c.z + r));
                continue;
            }
            CanonicalCarrierWitness::ExactCanonical {
                carrier:
                    CanonicalCarrier::Surface(truck_geometry::recognize::CanonicalSurface::Cone(cone)),
                ..
            } => {
                grow_point(&mut lo, &mut hi, cone.apex());
            }
            _ => {}
        }
        for wire in face.absolute_boundaries() {
            for edge in wire.edge_iter() {
                let box3 = curve_box(&edge.curve())?;
                grow_box(&mut lo, &mut hi, &box3);
            }
        }
    }
    Ok(Certified::new(
        ((lo.x, hi.x), (lo.y, hi.y), (lo.z, hi.z)),
        box_certificate(),
    ))
}

/// The 3-D axis-aligned box of one stored curve over its own range: the
/// `EnclosureCurve::enclose` box for a line, and the placement-matrix carrier
/// box for a placed circle (the canonical `Curve` carries no `EnclosureCurve`
/// impl, so the circle is boxed from its transform: for each axis the extent
/// is `|x-col| + |y-col|`, a sound hull of the full circle). A spline or
/// intersection-curve carrier is a `NonCanonicalCarrier` refusal here, the
/// same boundary the boolean lift enforces.
fn curve_box(curve: &Curve) -> Result<Box3, Refusal> {
    match curve {
        Curve::Line(line) => {
            let tt = Interval::try_from(line.range_tuple()).map_err(|_| non_canonical())?;
            Ok(line.enclose(tt))
        }
        Curve::Circle(processor) => {
            let m = *processor.transform();
            let center = Point3::new(m.w.x, m.w.y, m.w.z);
            let ex = m.x.x.abs() + m.y.x.abs();
            let ey = m.x.y.abs() + m.y.y.abs();
            let ez = m.x.z.abs() + m.y.z.abs();
            let from = |lo: f64, hi: f64| Interval::try_from((lo, hi)).map_err(|_| non_canonical());
            Ok(Box3 {
                x: from(center.x - ex, center.x + ex)?,
                y: from(center.y - ey, center.y + ey)?,
                z: from(center.z - ez, center.z + ez)?,
            })
        }
        Curve::BSplineCurve(_) | Curve::NurbsCurve(_) | Curve::IntersectionCurve(_) => {
            Err(non_canonical())
        }
    }
}

/// Grows the box to contain the point `p`.
fn grow_point(lo: &mut Point3, hi: &mut Point3, p: Point3) {
    lo.x = lo.x.min(p.x);
    lo.y = lo.y.min(p.y);
    lo.z = lo.z.min(p.z);
    hi.x = hi.x.max(p.x);
    hi.y = hi.y.max(p.y);
    hi.z = hi.z.max(p.z);
}

/// Grows the box to contain the interval box `b`.
fn grow_box(lo: &mut Point3, hi: &mut Point3, b: &Box3) {
    grow_point(lo, hi, Point3::new(b.x.inf(), b.y.inf(), b.z.inf()));
    grow_point(lo, hi, Point3::new(b.x.sup(), b.y.sup(), b.z.sup()));
}

/// D3 — the pad: twice the largest over-box dimension, so the halfspace box's
/// non-wall faces sit beyond any tangency with the solid.
fn pad_for(over: &Ranges) -> f64 {
    let dx = over.0 .1 - over.0 .0;
    let dy = over.1 .1 - over.1 .0;
    let dz = over.2 .1 - over.2 .0;
    2.0 * dx.max(dy).max(dz)
}

/// The axis-aligned pad-box: the over-box extended by `pad` on every side.
fn pad_box(over: &Ranges, pad: f64) -> Ranges {
    (
        (over.0 .0 - pad, over.0 .1 + pad),
        (over.1 .0 - pad, over.1 .1 + pad),
        (over.2 .0 - pad, over.2 .1 + pad),
    )
}

/// D3 — the halfspace box: the intersection of the axis-aligned `boxes` with
/// the closed halfspace `{ (p - o) . n <= 0 }` (`negative == true`) or
/// `{ (p - o) . n >= 0 }` (`negative == false`), as a convex polyhedron built
/// from 8-vertex / 12-edge / 6-face hexahedron topology where the cut allows,
/// `Solid::try_new` as the gate. The wall face (the one lying IN the cutting
/// plane) carries the caller's `Plane` value, so cap identification later is
/// exact-equality (D4/D6).
///
/// The construction: collect the pad-box corners on the interior side plus
/// the crossing of every pad-box edge whose endpoints straddle the plane
/// (exact at the plane's origin coordinate for an axis-aligned plane, so the
/// happy-path dyadic tests compare exactly), dedup by exact point equality,
/// then build one outward-oriented face per surviving pad-box plane and the
/// wall. Adjacent faces share one `Edge` instance per vertex pair.
fn halfspace_box(
    boxes: Ranges,
    plane: &Plane,
    negative: bool,
) -> Outcome<Solid<Point3, Curve, Surface>> {
    let o = plane.origin();
    let n = plane.normal();
    let (xs, ys, zs) = boxes;
    let side_of = |p: Point3| (p - o).dot(n);
    let inside = |s: f64| if negative { s <= 0.0 } else { s >= 0.0 };

    let corners = [
        Point3::new(xs.0, ys.0, zs.0),
        Point3::new(xs.0, ys.0, zs.1),
        Point3::new(xs.0, ys.1, zs.0),
        Point3::new(xs.0, ys.1, zs.1),
        Point3::new(xs.1, ys.0, zs.0),
        Point3::new(xs.1, ys.0, zs.1),
        Point3::new(xs.1, ys.1, zs.0),
        Point3::new(xs.1, ys.1, zs.1),
    ];
    let corner_side: Vec<f64> = corners.iter().map(|c| side_of(*c)).collect();

    // The plane's most-aligned axis: for an axis-aligned plane the crossing
    // of every straddling pad-box edge sits exactly at the plane's origin
    // coordinate, so it is taken directly (no interpolation rounding).
    let aligned = if n.x.abs() == 1.0 {
        Some(0usize)
    } else if n.y.abs() == 1.0 {
        Some(1usize)
    } else if n.z.abs() == 1.0 {
        Some(2usize)
    } else {
        None
    };

    // Interior vertices: the pad-box corners on the interior side plus the
    // crossing of every pad-box edge whose endpoints straddle the plane.
    let mut pts: Vec<Point3> = Vec::new();
    for (c, s) in corners.iter().zip(corner_side.iter()) {
        if inside(*s) {
            push_pt(&mut pts, *c);
        }
    }
    // The 12 pad-box edges, as corner-index pairs (z-, y-, then x-parallel).
    let edge_pairs: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (a, b) in edge_pairs {
        // a and b are compile-time corner indices into the 8-corner arrays
        // above; the get-chain only satisfies the indexing lint.
        let (Some(ca), Some(cb)) = (corners.get(a), corners.get(b)) else {
            continue;
        };
        let (Some(sa), Some(sb)) = (corner_side.get(a), corner_side.get(b)) else {
            continue;
        };
        let (sa, sb) = (*sa, *sb);
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let crossing = match aligned {
                Some(0) => Point3::new(o.x, ca.y, ca.z),
                Some(1) => Point3::new(ca.x, o.y, ca.z),
                Some(2) => Point3::new(ca.x, ca.y, o.z),
                _ => {
                    let t = sa / (sa - sb);
                    *ca + t * (*cb - *ca)
                }
            };
            push_pt(&mut pts, crossing);
        }
    }

    let verts: Vec<Vertex<Point3>> = pts.iter().map(|p| Vertex::new(*p)).collect();
    let mut pool: EdgePool = HashMap::default();
    let mut faces: Vec<Face<Point3, Curve, Surface>> = Vec::new();

    // The six pad-box planes; the outward normal of each face points away
    // from the box interior.
    for (axis, coord, outward) in [
        (0usize, xs.0, Vector3::new(-1.0, 0.0, 0.0)),
        (0usize, xs.1, Vector3::new(1.0, 0.0, 0.0)),
        (1usize, ys.0, Vector3::new(0.0, -1.0, 0.0)),
        (1usize, ys.1, Vector3::new(0.0, 1.0, 0.0)),
        (2usize, zs.0, Vector3::new(0.0, 0.0, -1.0)),
        (2usize, zs.1, Vector3::new(0.0, 0.0, 1.0)),
    ] {
        let indices: Vec<usize> = pts
            .iter()
            .enumerate()
            .filter(|(_, p)| match axis {
                0 => p.x == coord,
                1 => p.y == coord,
                _ => p.z == coord,
            })
            .map(|(i, _)| i)
            .collect();
        if let Some(face) = face_from_indices(&pts, &verts, &indices, outward, None, &mut pool)? {
            faces.push(face);
        }
    }

    // The wall: the vertices lying exactly in the cutting plane. Its outward
    // normal points away from the interior: +n for the negative side, -n for
    // the positive side, and its surface is the caller's `Plane` value.
    let wall_indices: Vec<usize> = pts
        .iter()
        .enumerate()
        .filter(|(_, p)| side_of(**p) == 0.0)
        .map(|(i, _)| i)
        .collect();
    let wall_outward = if negative { n } else { -n };
    if let Some(face) = face_from_indices(
        &pts,
        &verts,
        &wall_indices,
        wall_outward,
        Some(*plane),
        &mut pool,
    )? {
        faces.push(face);
    }

    if faces.is_empty() {
        return Err(Refusal::Empty);
    }
    let shell: Shell<Point3, Curve, Surface> = faces.into();
    let solid = Solid::try_new(vec![shell]).map_err(|_| invalid_shell())?;
    Ok(Certified::new(solid, box_certificate()))
}

/// Builds one outward-oriented face from a vertex-index list: orders the
/// vertices by angle around the face centroid in the face plane, orients the
/// polygon so its normal agrees with `outward`, and builds the boundary edges
/// through the shared-edge pool. The surface is the caller's `Plane` when
/// given, otherwise a plane through the ordered vertices. `None` for a
/// degenerate (fewer than three) face.
fn face_from_indices(
    pts: &[Point3],
    verts: &[Vertex<Point3>],
    indices: &[usize],
    outward: Vector3,
    surface: Option<Plane>,
    pool: &mut EdgePool,
) -> Result<Option<Face<Point3, Curve, Surface>>, Refusal> {
    let points: Vec<(usize, Point3)> = indices
        .iter()
        .filter_map(|i| pts.get(*i).map(|p| (*i, *p)))
        .collect();
    if points.len() < 3 {
        return Ok(None);
    }
    let n = outward.normalize();
    let sum: Vector3 = points
        .iter()
        .map(|(_, p)| Vector3::new(p.x, p.y, p.z))
        .fold(Vector3::zero(), |acc, v| acc + v);
    let scale = 1.0 / points.len() as f64;
    let c = Point3::new(sum.x * scale, sum.y * scale, sum.z * scale);

    // An orthonormal frame of the face plane.
    let aux = if n.x.abs() <= n.y.abs() && n.x.abs() <= n.z.abs() {
        Vector3::unit_x()
    } else if n.y.abs() <= n.z.abs() {
        Vector3::unit_y()
    } else {
        Vector3::unit_z()
    };
    let e1 = (aux - n * aux.dot(n)).normalize();
    let e2 = n.cross(e1);

    let mut order = points;
    order.sort_by(|(_, pa), (_, pb)| {
        let a = (*pa - c).dot(e2).atan2((*pa - c).dot(e1));
        let b = (*pb - c).dot(e2).atan2((*pb - c).dot(e1));
        a.total_cmp(&b)
    });

    // The polygon's normal (the wedge sum); reverse the trace if it points
    // the wrong way.
    let mut poly_n = Vector3::zero();
    for k in 0..order.len() {
        let (_, p) = *order.get(k).ok_or(Refusal::Empty)?;
        let (_, q) = *order.get((k + 1) % order.len()).ok_or(Refusal::Empty)?;
        poly_n += (p - c).cross(q - c);
    }
    if poly_n.dot(n) < 0.0 {
        order.reverse();
    }

    let surface = match surface {
        Some(plane) => Surface::Plane(plane),
        None => {
            let (_, v0) = *order.first().ok_or(Refusal::Empty)?;
            let (_, v1) = *order.get(1).ok_or(Refusal::Empty)?;
            let (_, v2) = *order.get(2).ok_or(Refusal::Empty)?;
            Surface::Plane(Plane::new(v0, v1, v2))
        }
    };

    let mut edges: Vec<Edge<Point3, Curve>> = Vec::new();
    for k in 0..order.len() {
        let (ia, pa) = *order.get(k).ok_or(Refusal::Empty)?;
        let (ib, pb) = *order.get((k + 1) % order.len()).ok_or(Refusal::Empty)?;
        let edge = shared_edge(pool, verts, pts, ia, ib, pa, pb)?;
        edges.push(edge);
    }
    let wire = Wire::from(edges);
    let face = Face::try_new(vec![wire], surface).map_err(|_| Refusal::Empty)?;
    Ok(Some(face))
}

/// The shared edge of the vertex-index pair `(a, b)`: one instance per
/// unordered pair, reused (inverted as needed) by the two adjacent faces.
fn shared_edge(
    pool: &mut EdgePool,
    verts: &[Vertex<Point3>],
    pts: &[Point3],
    a: usize,
    b: usize,
    pa: Point3,
    pb: Point3,
) -> Result<Edge<Point3, Curve>, Refusal> {
    let lo = a.min(b);
    let hi = a.max(b);
    if let Some(edge) = pool.get(&(lo, hi)) {
        return if a == lo {
            Ok(edge.clone())
        } else {
            Ok(edge.inverse().clone())
        };
    }
    let va = verts.get(lo).ok_or(Refusal::Empty)?;
    let vb = verts.get(hi).ok_or(Refusal::Empty)?;
    let point_a = pts.get(lo).copied().unwrap_or(pa);
    let point_b = pts.get(hi).copied().unwrap_or(pb);
    let edge =
        Edge::try_new(va, vb, Curve::Line(Line(point_a, point_b))).map_err(|_| Refusal::Empty)?;
    pool.insert((lo, hi), edge.clone());
    if a == lo {
        Ok(edge)
    } else {
        Ok(edge.inverse().clone())
    }
}

/// Pushes a point onto the vertex list if no existing point is exactly equal
/// to it (the load-bearing instance rule: coincident geometric points share
/// one `Vertex`, or the shell stays open).
fn push_pt(pts: &mut Vec<Point3>, p: Point3) {
    if !pts.contains(&p) {
        pts.push(p);
    }
}

/// The certificate of a box construction: the structure is float arithmetic
/// (H-6), claims nothing, and spends no caller budget.
fn box_certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// D4 — splits `solid` by `plane` into `(plus, minus)`: `plus` on the side
/// the plane's normal points to, `minus` on the other. Both halves are two
/// `boolean()` calls against the negative halfspace box, so the landed
/// pipeline's `Solid::try_new` gate IS the certificate (D6). A plane that
/// does not touch the solid is a NORMAL result: `plus ~= S`, `minus` the
/// empty solid (zero shells, the landed assembler's all-discarded rule).
/// The `(plus, minus)` halves of a split; the alias keeps the signature
/// under the type-complexity lint.
type SplitHalves = (Solid<Point3, Curve, Surface>, Solid<Point3, Curve, Surface>);

/// D4 — splits `solid` by `plane` into `(plus, minus)`: `plus` on the side
/// the plane's normal points to, `minus` on the other. Both halves are two
/// `boolean()` calls against the negative halfspace box, so the landed
/// pipeline's `Solid::try_new` gate IS the certificate (D6). A plane that
/// does not touch the solid is a NORMAL result: `plus ~= S`, `minus` the
/// empty solid (zero shells, the landed assembler's all-discarded rule).
pub fn split_by_plane(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
    budget: &mut Budget,
) -> Outcome<SplitHalves> {
    let over = solid_over_box(solid)?.value;
    let pad = pad_for(&over);
    let padded = pad_box(&over, pad);
    let minus_box = halfspace_box(padded, plane, true)?.value;
    let plus = boolean(solid, BoolOp::Difference, &minus_box, budget)?;
    let minus = boolean(solid, BoolOp::Intersection, &minus_box, budget)?;
    let cert = plus
        .cert
        .accumulate(&minus.cert)
        .map_err(Refusal::Contradictory)?;
    Ok(Certified::new((plus.value, minus.value), cert))
}

/// D4 — the section faces: the cap faces of the wall plane, extracted from
/// the `Difference` half by exact `Plane` identity with the halfspace box's
/// wall (C0 identity with our own construction — no tolerance, no recognize
/// beyond the exact match). A plane that does not touch the solid yields no
/// cap and refuses `Refusal::Empty` (no section exists).
pub fn section_faces(
    solid: &Solid<Point3, Curve, Surface>,
    plane: &Plane,
    budget: &mut Budget,
) -> Outcome<Vec<Face<Point3, Curve, Surface>>> {
    let over = solid_over_box(solid)?.value;
    let pad = pad_for(&over);
    let padded = pad_box(&over, pad);
    let minus_box = halfspace_box(padded, plane, true)?.value;
    let plus = boolean(solid, BoolOp::Difference, &minus_box, budget)?;
    let caps: Vec<Face<Point3, Curve, Surface>> = plus
        .value
        .face_iter()
        .filter(|face| matches!(face.surface(), Surface::Plane(p) if p == *plane))
        .cloned()
        .collect();
    if caps.is_empty() {
        return Err(Refusal::Empty);
    }
    Ok(Certified::new(caps, plus.cert))
}
