//! BG-INV-109: wedge non-degeneracy (§1.1 invariant 9) at every interior
//! edge, v1: sampled at each edge's parameter midpoint.
//!
//! The dihedral angle at every interior edge of a valid solid boundary is
//! bounded away from 0 and 2π — no folded (knife-edge) or doubled-back
//! (crack) wedges. This is the condition local feature size needs to be
//! positive (BG-FID-001).
//!
//! **v1 samples the edge's midpoint.** The whole-span version needs the edge's
//! parameter image on each face — the pcurve (BG-CE-001's payload, unwired) —
//! to feed the surfaces' `normal_cone`s. What v1 certifies: the wedge is
//! non-degenerate AT the sampled point. **Interior edges only**: an edge used
//! by one face (an open boundary) has no wedge; an edge used by more than two
//! faces is BG-INV-101's violation, not this checker's — both are skipped.
//!
//! The self-contained `Line`/`Plane` witnesses below mirror
//! `truck-geometry`'s types so the tests and doctest do not need a
//! `truck-geometry` dependency (`truck-topology` does not have one).

use crate::{Edge, EdgeID, Face, Shell};
use std::collections::{HashMap, HashSet};
use std::ops::Bound;
use truck_base::cgmath64::{EuclideanSpace, InnerSpace, Point3, Vector3, Zero};
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth, UnresolvedWitness,
};
use truck_geotrait::{
    ParameterRange, ParametricCurve, ParametricSurface, ParametricSurface3D, SPHint2D,
    SearchParameter, D2,
};

/// The number of trials handed to `search_parameter` — the crate's own
/// `SEARCH_PARAMETER_TRIALS` in lib.rs is private, and a local const is the
/// H-4-clean equivalent.
const SEARCH_TRIALS: usize = 100;

/// A line-segment witness curve, mirroring `truck-geometry::Line<Point3>`
/// (parameter range `[0, 1]`). `truck-topology` has no `truck-geometry`
/// dependency, so the packet's witnesses are defined here instead.
#[derive(Clone, Debug)]
pub struct Line(
    /// The front point of the segment.
    pub Point3,
    /// The back point of the segment.
    pub Point3,
);

impl ParametricCurve for Line {
    type Point = Point3;
    type Vector = Vector3;

    fn subs(&self, t: f64) -> Point3 {
        self.0 + (self.1 - self.0) * t
    }

    fn der(&self, _t: f64) -> Vector3 {
        self.1 - self.0
    }

    fn der2(&self, _t: f64) -> Vector3 {
        Vector3::zero()
    }

    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        match n {
            0 => self.subs(t).to_vec(),
            1 => self.1 - self.0,
            _ => Vector3::zero(),
        }
    }

    fn parameter_range(&self) -> ParameterRange {
        (Bound::Included(0.0), Bound::Included(1.0))
    }
}

/// A plane witness surface, mirroring `truck-geometry::Plane`
/// (parameterization `S(u, v) = o + u·(p−o) + v·(q−o)` over `[0,1]²`).
#[derive(Clone, Debug)]
pub struct Plane {
    o: Point3,
    p: Point3,
    q: Point3,
}

impl Plane {
    /// Creates a plane through the three points `origin`, `one` and `another`.
    pub fn new(origin: Point3, one: Point3, another: Point3) -> Plane {
        Plane {
            o: origin,
            p: one,
            q: another,
        }
    }

    /// The plane's u-axis `p − o`.
    fn u_axis(&self) -> Vector3 {
        self.p - self.o
    }

    /// The plane's v-axis `q − o`.
    fn v_axis(&self) -> Vector3 {
        self.q - self.o
    }

    /// The unit normal `(p − o) × (q − o)`.
    fn normal(&self) -> Vector3 {
        self.u_axis().cross(self.v_axis()).normalize()
    }
}

impl ParametricSurface for Plane {
    type Point = Point3;
    type Vector = Vector3;

    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        match (m, n) {
            (0, 0) => self.subs(u, v).to_vec(),
            (1, 0) => self.p - self.o,
            (0, 1) => self.q - self.o,
            _ => Vector3::zero(),
        }
    }

    fn subs(&self, u: f64, v: f64) -> Point3 {
        self.o + u * (self.p - self.o) + v * (self.q - self.o)
    }

    fn uder(&self, _u: f64, _v: f64) -> Vector3 {
        self.p - self.o
    }

    fn vder(&self, _u: f64, _v: f64) -> Vector3 {
        self.q - self.o
    }

    fn uuder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }

    fn uvder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }

    fn vvder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }

    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        let range = (Bound::Included(0.0), Bound::Included(1.0));
        (range, range)
    }
}

impl ParametricSurface3D for Plane {
    fn normal(&self, _u: f64, _v: f64) -> Vector3 {
        self.normal()
    }

    fn normal_uder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }

    fn normal_vder(&self, _u: f64, _v: f64) -> Vector3 {
        Vector3::zero()
    }
}

/// The dimensionless height-to-size ratio past which a point counts as
/// off-plane in the witness's `search_parameter`.
const WITNESS_PLANE_RATIO: f64 = 1e-6; // H-3: dimensionless containment ratio of the test plane witness, not a length

impl SearchParameter<D2> for Plane {
    type Point = Point3;

    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        _hint: H,
        _trials: usize,
    ) -> Option<(f64, f64)> {
        let a = self.u_axis();
        let b = self.v_axis();
        let c = a.cross(b);
        let denom = c.magnitude();
        if !denom.is_finite() || denom <= 0.0 {
            return None;
        }
        let w = point - self.o;
        let u = w.cross(b).dot(c) / denom.powi(2);
        let v = a.cross(w).dot(c) / denom.powi(2);
        let h = w.dot(c) / denom;
        let tol = (a.magnitude() + b.magnitude()) * WITNESS_PLANE_RATIO;
        if h.abs() <= tol {
            Some((u, v))
        } else {
            None
        }
    }
}

/// BG-INV-109: wedge non-degeneracy (§1.1 invariant 9) at every interior
/// edge, v1: sampled at each edge's parameter midpoint.
///
/// For each edge used by exactly two faces: take the curve's midpoint
/// `p`, project `p` onto both faces' surfaces (`search_parameter`),
/// take both unit normals there, and require
/// `|n_A × n_B| >= sin_margin` — the sine of the normals' angle, zero
/// exactly for the folded (normals parallel) and doubled-back (normals
/// antiparallel) degenerate wedges. `sin_margin` is dimensionless; pass
/// `ToleranceCtx::sin_margin()` for the house default.
///
/// **v1 samples the midpoint only** — the whole-span certificate needs
/// the pcurve parameter images (BG-CE-001's payload, unwired) feeding
/// `normal_cone`. Edges used by one face (open boundary) are skipped;
/// edges used by more than two faces are BG-INV-101's to catch, skipped
/// here. A failed projection is `NumericallyUnresolved` (the point's
/// containment in the surface could not be certified), never a
/// violation. Localise: the refusal's `prop` names the invariant; the
/// offending edge is the first in `edge_iter` order whose check fails.
///
/// ```
/// use truck_topology::*;
/// use truck_topology::invariants::wedge::{check, Line, Plane};
/// use truck_base::cgmath64::Point3;
///
/// // The right-angle tent: shared edge on the x-axis, face A in the plane
/// // y = 0, face B in the plane z = 0.
/// let v0 = Vertex::new(0usize);
/// let v1 = Vertex::new(1usize);
/// let v2 = Vertex::new(2usize);
/// let v3 = Vertex::new(3usize);
/// let p0 = Point3::new(0.0, 0.0, 0.0);
/// let p1 = Point3::new(1.0, 0.0, 0.0);
/// let pa = Point3::new(0.5, 0.0, 1.0);
/// let pb = Point3::new(0.5, 1.0, 0.0);
/// let shared = Edge::new(&v0, &v1, Line(p0, p1));
/// let e1 = Edge::new(&v1, &v2, Line(p1, pa));
/// let e2 = Edge::new(&v2, &v0, Line(pa, p0));
/// let e3 = Edge::new(&v0, &v3, Line(p0, pb));
/// let e4 = Edge::new(&v3, &v1, Line(pb, p1));
/// let face_a = Face::new(vec![wire![&shared, &e1, &e2]], Plane::new(p0, p1, pa));
/// let face_b = Face::new(
///     vec![wire![&shared.inverse(), &e3, &e4]],
///     Plane::new(p0, p1, pb),
/// );
/// let shell = Shell::from(vec![face_a, face_b]);
/// assert!(check(&shell, 0.5).is_ok());
/// ```
pub fn check<P, C, S>(shell: &Shell<P, C, S>, sin_margin: f64) -> Outcome<()>
where
    C: ParametricCurve<Point = Point3> + Clone,
    S: ParametricSurface<Point = Point3>
        + ParametricSurface3D
        + SearchParameter<D2, Point = Point3>
        + Clone,
{
    let mut edge_uses: HashMap<EdgeID<C>, Vec<usize>> = HashMap::new();
    for (face_index, face) in shell.face_iter().enumerate() {
        for edge in face.boundary_iters().into_iter().flatten() {
            edge_uses.entry(edge.id()).or_default().push(face_index);
        }
    }

    let mut seen: HashSet<EdgeID<C>> = HashSet::new();
    for edge in shell.edge_iter() {
        let id = edge.id();
        if !seen.insert(id) {
            continue;
        }
        let (a, b) = match edge_uses.get(&id) {
            Some(uses) => match uses.as_slice() {
                [a, b] => (*a, *b),
                _ => continue,
            },
            None => continue,
        };
        let face_a = match shell.get(a) {
            Some(face) => face,
            None => continue,
        };
        let face_b = match shell.get(b) {
            Some(face) => face,
            None => continue,
        };
        test_edge(&edge, face_a, face_b, sin_margin)?;
    }

    let mut props = PropMap::new();
    props.set(Prop::WedgeNonDegeneracy, Truth::True);
    Ok(Certified::new(
        (),
        Certificate {
            props,
            method: Method::Float,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The wedge test of one interior edge: sample the curve's parameter midpoint,
/// project it onto both faces' surfaces and require the sine of the angle
/// between the unit normals to clear `sin_margin`.
fn test_edge<P, C, S>(
    edge: &Edge<P, C>,
    face_a: &Face<P, C, S>,
    face_b: &Face<P, C, S>,
    sin_margin: f64,
) -> Result<(), Refusal>
where
    C: ParametricCurve<Point = Point3> + Clone,
    S: ParametricSurface<Point = Point3>
        + ParametricSurface3D
        + SearchParameter<D2, Point = Point3>
        + Clone,
{
    let curve = edge.curve();
    let (t0, t1) = match curve.try_range_tuple() {
        Some(range) => range,
        None => return Err(unresolved()),
    };
    let t_mid = (t0 + t1) / 2.0;
    let p = curve.subs(t_mid);
    let surface_a = face_a.surface();
    let surface_b = face_b.surface();
    let n_a = surface_normal(&surface_a, p)?;
    let n_b = surface_normal(&surface_b, p)?;
    let sin_angle = n_a.cross(n_b).magnitude();
    if sin_angle < sin_margin {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::WedgeNonDegeneracy,
            left: Truth::True,
            right: Truth::False,
        }));
    }
    Ok(())
}

/// The unit normal of `surface` at the parameters `p` projects to, or
/// `NumericallyUnresolved` when `p`'s containment in the surface cannot be
/// certified.
fn surface_normal<S>(surface: &S, p: Point3) -> Result<Vector3, Refusal>
where
    S: ParametricSurface<Point = Point3>
        + ParametricSurface3D
        + SearchParameter<D2, Point = Point3>
        + Clone,
{
    let (u, v) = match surface.search_parameter(p, None, SEARCH_TRIALS) {
        Some(uv) => uv,
        None => return Err(unresolved()),
    };
    Ok(surface.normal(u, v).normalize())
}

/// The "could not certify" refusal shared by an unbounded curve parameter
/// range and a failed point projection.
fn unresolved() -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    }
}

#[cfg(test)]
mod tests {
    // H-1: unit tests assert on hand-built witnesses over the local
    // Line/Plane mirrors, not on a path reachable from untrusted geometry; the
    // parent module's deny already forbids panic and indexing, and these tests
    // use neither unwrap nor expect.
    #![deny(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::*;

    /// The test wedge margin: sin(90°) = 1 for the right-angle tent, so 0.5
    /// sits comfortably below the would-be violation boundary.
    const TEST_WEDGE_MARGIN: f64 = 0.5; // H-3: dimensionless test wedge margin (a sine), not a length

    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }

    /// The right-angle tent: shared edge on the x-axis from (0,0,0) to
    /// (1,0,0); face A in the plane y = 0 (apex (0.5,0,1)), face B in the
    /// plane z = 0 (apex (0.5,1,0)). Face B's wire traverses the shared edge
    /// inverted so the uses pair.
    fn right_angle_tent() -> Shell<usize, Line, Plane> {
        let v0 = Vertex::new(0usize);
        let v1 = Vertex::new(1usize);
        let v2 = Vertex::new(2usize);
        let v3 = Vertex::new(3usize);
        let shared = Edge::new(&v0, &v1, Line(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let e1 = Edge::new(&v1, &v2, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e2 = Edge::new(&v2, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let e3 = Edge::new(&v0, &v3, Line(p(0.0, 0.0, 0.0), p(0.5, 1.0, 0.0)));
        let e4 = Edge::new(&v3, &v1, Line(p(0.5, 1.0, 0.0), p(1.0, 0.0, 0.0)));
        let face_a = Face::new(
            vec![wire![&shared, &e1, &e2]],
            Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)),
        );
        let face_b = Face::new(
            vec![wire![&shared.inverse(), &e3, &e4]],
            Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 1.0, 0.0)),
        );
        Shell::from(vec![face_a, face_b])
    }

    #[test]
    fn wedge_right_angle_tent_holds() {
        let shell = right_angle_tent();
        let outcome = check(&shell, TEST_WEDGE_MARGIN);
        assert!(
            matches!(&outcome, Ok(Certified { value: (), .. })),
            "the 90° tent must certify a hold, got {outcome:?}"
        );
        if let Ok(certified) = &outcome {
            let cert = &certified.cert;
            assert_eq!(cert.props.get(Prop::WedgeNonDegeneracy), Truth::True);
            assert_eq!(cert.method, Method::Float);
        }
    }

    #[test]
    fn wedge_folded_coplanar_faces_violate() {
        // Both faces in the SAME plane y = 0: B's apex is moved to (0.5, 0, 1)
        // so the two surface normals are parallel — the folded (knife-edge)
        // wedge.
        let v0 = Vertex::new(0usize);
        let v1 = Vertex::new(1usize);
        let v2 = Vertex::new(2usize);
        let v3 = Vertex::new(3usize);
        let shared = Edge::new(&v0, &v1, Line(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let e1 = Edge::new(&v1, &v2, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e2 = Edge::new(&v2, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let e3 = Edge::new(&v0, &v3, Line(p(0.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e4 = Edge::new(&v3, &v1, Line(p(0.5, 0.0, 1.0), p(1.0, 0.0, 0.0)));
        let plane = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0));
        let face_a = Face::new(vec![wire![&shared, &e1, &e2]], plane.clone());
        let face_b = Face::new(vec![wire![&shared.inverse(), &e3, &e4]], plane);
        let shell = Shell::from(vec![face_a, face_b]);
        let outcome = check(&shell, TEST_WEDGE_MARGIN);
        assert!(
            matches!(
                &outcome,
                Err(Refusal::Contradictory(w))
                    if w.prop == Prop::WedgeNonDegeneracy
                        && w.left == Truth::True
                        && w.right == Truth::False
            ),
            "coplanar faces must refuse the wedge claim, got {outcome:?}"
        );
    }

    #[test]
    fn wedge_doubled_back_faces_violate() {
        // B in the same geometric plane but its wire traversed with the same
        // orientation as A's and its surface normal reversed: the two unit
        // normals are antiparallel across the shared edge — the crack.
        let v0 = Vertex::new(0usize);
        let v1 = Vertex::new(1usize);
        let v2 = Vertex::new(2usize);
        let v3 = Vertex::new(3usize);
        let shared = Edge::new(&v0, &v1, Line(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let e1 = Edge::new(&v1, &v2, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e2 = Edge::new(&v2, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let e3 = Edge::new(&v1, &v3, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e4 = Edge::new(&v3, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let plane_a = Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0));
        let plane_b = Plane::new(p(0.0, 0.0, 0.0), p(0.5, 0.0, 1.0), p(1.0, 0.0, 0.0));
        let face_a = Face::new(vec![wire![&shared, &e1, &e2]], plane_a);
        let face_b = Face::new(vec![wire![&shared, &e3, &e4]], plane_b);
        let shell = Shell::from(vec![face_a, face_b]);
        let outcome = check(&shell, TEST_WEDGE_MARGIN);
        assert!(
            matches!(
                &outcome,
                Err(Refusal::Contradictory(w))
                    if w.prop == Prop::WedgeNonDegeneracy
                        && w.left == Truth::True
                        && w.right == Truth::False
            ),
            "doubled-back faces must refuse the wedge claim, got {outcome:?}"
        );
    }

    #[test]
    fn wedge_boundary_edge_is_skipped() {
        // One triangular face only: every edge is used once, so there is no
        // interior edge and the checker certifies a hold (open boundaries are
        // skipped).
        let v0 = Vertex::new(0usize);
        let v1 = Vertex::new(1usize);
        let v2 = Vertex::new(2usize);
        let e0 = Edge::new(&v0, &v1, Line(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let e1 = Edge::new(&v1, &v2, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e2 = Edge::new(&v2, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let face = Face::new(
            vec![wire![&e0, &e1, &e2]],
            Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)),
        );
        let shell = Shell::from(vec![face]);
        let outcome = check(&shell, TEST_WEDGE_MARGIN);
        assert!(
            matches!(&outcome, Ok(Certified { value: (), .. })),
            "a shell with no interior edges must certify a hold, got {outcome:?}"
        );
        if let Ok(certified) = &outcome {
            assert_eq!(
                certified.cert.props.get(Prop::WedgeNonDegeneracy),
                Truth::True
            );
        }
    }

    #[test]
    fn wedge_projection_failure_is_unresolved() {
        // The tent with B's surface TRANSLATED off the shared edge (the plane
        // z = 0.1): the shared edge's midpoint (0.5, 0, 0) is not in B's
        // surface, so its containment cannot be certified — never a violation.
        let v0 = Vertex::new(0usize);
        let v1 = Vertex::new(1usize);
        let v2 = Vertex::new(2usize);
        let v3 = Vertex::new(3usize);
        let shared = Edge::new(&v0, &v1, Line(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)));
        let e1 = Edge::new(&v1, &v2, Line(p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)));
        let e2 = Edge::new(&v2, &v0, Line(p(0.5, 0.0, 1.0), p(0.0, 0.0, 0.0)));
        let e3 = Edge::new(&v0, &v3, Line(p(0.0, 0.0, 0.0), p(0.5, 1.0, 0.0)));
        let e4 = Edge::new(&v3, &v1, Line(p(0.5, 1.0, 0.0), p(1.0, 0.0, 0.0)));
        let face_a = Face::new(
            vec![wire![&shared, &e1, &e2]],
            Plane::new(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.5, 0.0, 1.0)),
        );
        let face_b = Face::new(
            vec![wire![&shared.inverse(), &e3, &e4]],
            Plane::new(p(0.5, 0.0, 0.1), p(1.5, 0.0, 0.1), p(0.5, 1.0, 0.1)),
        );
        let shell = Shell::from(vec![face_a, face_b]);
        let outcome = check(&shell, TEST_WEDGE_MARGIN);
        assert!(
            matches!(
                &outcome,
                Err(Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::UncertifiedContainment,
                    ..
                })
            ),
            "a plane off the shared edge must be numerically unresolved, got {outcome:?}"
        );
    }
}
