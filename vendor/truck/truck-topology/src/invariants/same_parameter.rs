//! BG-INV-104: same-parameter / same-range (§1.1 invariant 4).
//!
//! Every edge use's parametric trace agrees with the edge's leader curve:
//! `||Γ_f(pc_u(t)) − c_e(φ_u(t))|| ≤ τ_e` over the WHOLE span. BG-CE-002's
//! [`certify_deviation`](truck_evidence::certify_deviation) is the
//! certificate; this module is the checker that applies it to an [`Edge`]'s
//! pcurve payload (the `PC` field BG-CE-001 landed) and speaks the invariants
//! contract. The correspondence `φ_u` is the ATTACHMENT CONTRACT — it is not
//! recorded in the tree (the pcurve field carries only the curve), so the
//! checker takes it as a parameter.
//!
//! An edge whose `pcurve()` is `None` (the `PC = ()` default, today's every
//! edge) is vacuously satisfied: there is no trace to disagree with the
//! leader.

use crate::Edge;
use std::ops::Bound;

use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, Prop, PropMap, Refusal, Truth,
};
use truck_evidence::{certify_deviation, EnclosureCurve, ParamMap};
use truck_geometry::decorators::PCurve;
use truck_geotrait::ParametricCurve;

/// BG-INV-104: same-parameter / same-range (§1.1 invariant 4) for ONE
/// edge use.
///
/// Certifies `||pc(t) − curve(phi(t))|| ≤ tau` for ALL t in the pcurve's
/// parameter span, by BG-CE-002's whole-span certificate. The parameter
/// correspondence `phi` is the attachment contract, supplied by the
/// caller — the tree does not record it. An edge whose `pcurve()` is
/// `None` (the `PC = ()` default, today's every edge) is vacuously
/// satisfied: there is no trace to disagree with the leader.
///
/// Refusals: `ForwardToleranceExceeded { bound, allowed }` is the
/// VIOLATION (a certified lower bound on the deviation exceeds `tau` —
/// this checker keeps the quantitative refusal rather than collapsing it
/// to `Contradictory`, because the bound localises by magnitude);
/// `NumericallyUnresolved` means neither could be established within
/// budget; `Empty` means the pcurve's span is empty or unbounded — trim
/// before certifying.
///
/// ```
/// use truck_base::cgmath64::Point3;
/// use truck_base::evidence::Budget;
/// use truck_base::tolerance::{TOLERANCE, ToleranceCtx};
/// use truck_evidence::ParamMap;
/// use truck_geometry::nurbs::{BSplineCurve, KnotVec};
/// use truck_topology::invariants::same_parameter::check_edge;
/// use truck_topology::{Edge, Vertex};
///
/// let v = Vertex::news([0usize, 1usize]);
/// let leader = BSplineCurve::new(
///     KnotVec::bezier_knot(2),
///     vec![
///         Point3::new(0.0, 0.0, 0.0),
///         Point3::new(0.5, 0.0, 0.5),
///         Point3::new(1.0, 1.0, 2.0),
///     ],
/// );
/// // The vacuous case: the `PC = ()` default carries no pcurve, so there is
/// // no trace to disagree with the leader.
/// let edge: Edge<usize, BSplineCurve<Point3>, ()> = Edge::new(&v[0], &v[1], leader);
/// let tau = match ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE) {
///     Ok(ctx) => ctx.value.entity_tau(TOLERANCE),
///     Err(_) => return,
/// };
/// let mut budget = Budget::new(1 << 16, 0, 0);
/// assert!(check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget).is_ok());
/// ```
#[allow(private_bounds)] // H-1: PcurveTrace is internal dispatch, not caller API
pub fn check_edge<P, C, PC>(
    edge: &Edge<P, C, PC>,
    phi: ParamMap,
    tau: f64,
    budget: &mut Budget,
) -> Outcome<()>
where
    C: EnclosureCurve + Clone,
    PC: PcurveTrace,
{
    let Some(pc) = edge.pcurve() else {
        return vacuous_holds();
    };
    // The `curve()` accessor lives on the `PC = ()` impl only; for a general
    // pcurve payload the shared leader curve is behind the field's mutex.
    let leader = edge.curve.lock();
    pc.certify_trace(&*leader, phi, tau, budget)
}

/// The vacuous-holds certificate: `method: Method::None` (nothing was
/// computed), `SameParameter` `True` because there is no trace to disagree
/// with the leader.
fn vacuous_holds() -> Outcome<()> {
    let mut props = PropMap::new();
    props.set(Prop::SameParameter, Truth::True);
    Ok(Certified::new(
        (),
        Certificate {
            props,
            method: Method::None,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// Dispatches the attached-pcurve path by carrier type. `check_edge` needs no
/// `PC: EnclosureCurve` to accept the vacuous `()` default; the impls below
/// carry the bound where it is actually required. `certify_trace` extracts
/// the pcurve's span and certifies the whole-span deviation against the
/// leader by BG-CE-002.
trait PcurveTrace {
    /// Certify this pcurve trace against the leader over its whole parameter
    /// span.
    fn certify_trace<L: EnclosureCurve>(
        &self,
        leader: &L,
        phi: ParamMap,
        tau: f64,
        budget: &mut Budget,
    ) -> Outcome<()>;
}

impl PcurveTrace for () {
    fn certify_trace<L: EnclosureCurve>(
        &self,
        _leader: &L,
        _phi: ParamMap,
        _tau: f64,
        _budget: &mut Budget,
    ) -> Outcome<()> {
        vacuous_holds()
    }
}

impl<C, S> PcurveTrace for PCurve<C, S>
where
    PCurve<C, S>: EnclosureCurve,
{
    fn certify_trace<L: EnclosureCurve>(
        &self,
        leader: &L,
        phi: ParamMap,
        tau: f64,
        budget: &mut Budget,
    ) -> Outcome<()> {
        // The pcurve's span: both bounds finite (Included or Excluded) with
        // t0 < t1, else there is nothing certifiable — an unbounded or
        // inverted span is `Empty` (H-1: no unwrap anywhere in this path).
        let (lo, hi) = self.parameter_range();
        let (t0, t1) = match (lo, hi) {
            (Bound::Included(t0), Bound::Included(t1))
            | (Bound::Included(t0), Bound::Excluded(t1))
            | (Bound::Excluded(t0), Bound::Included(t1))
            | (Bound::Excluded(t0), Bound::Excluded(t1)) => (t0, t1),
            (Bound::Unbounded, _) | (_, Bound::Unbounded) => return Err(Refusal::Empty),
        };
        if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
            return Err(Refusal::Empty);
        }
        let tt = match Interval::try_from((t0, t1)) {
            Ok(tt) => tt,
            Err(_) => return Err(Refusal::Empty),
        };
        // The LEADER is the edge's 3D curve, the CARRIER is the pcurve.
        // Getting this backwards inverts the correspondence, and the offset
        // tests are designed to catch it.
        match certify_deviation(leader, self, phi, tt, tau, budget) {
            Ok(certified) => {
                // Keep the bound's certificate (method, budget_left,
                // SoundEnclosure, margin, modulus) and join the invariant's
                // prop onto its map.
                let mut props = certified.cert.props.clone();
                props.set(Prop::SameParameter, Truth::True);
                Ok(Certified::new(
                    (),
                    Certificate {
                        props,
                        method: certified.cert.method,
                        budget_left: certified.cert.budget_left,
                        margin: certified.cert.margin,
                        modulus: certified.cert.modulus,
                    },
                ))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
// H-1: test-only indexing of the two-element hand-built witness vertex pair,
// not a kernel path.
#[allow(clippy::indexing_slicing)]
mod tests {
    // H-1: unwrap/expect stay banned here too — the checker path must never
    // reach for a panicking extractor, and the hand-built witnesses make each
    // assertion direct rather than unwrap-driven.
    #![deny(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::Vertex;
    use truck_base::cgmath64::{Point2, Point3, Vector3};
    use truck_base::evidence::UnresolvedWitness;
    use truck_base::tolerance::{ToleranceCtx, TOLERANCE};
    use truck_geometry::nurbs::{BSplineCurve, KnotVec};
    use truck_geometry::specifieds::{Plane, Sphere};
    use truck_geotrait::Cut;

    /// The route-2 tolerance for the sphere-pcurve pair, a dimensionless
    /// deviation tolerance at unit scale.
    const ROUTE2_TAU: f64 = 1.0e-3; // H-3: a dimensionless deviation tolerance for the sphere-pcurve pair, not a length

    /// The route-2 leader's z-offset above the carrier trace, in multiples of
    /// the route-2 tolerance: `4.0 * ROUTE2_TAU`.
    const ROUTE2_OFFSET_MULT: f64 = 4.0;

    /// The route-2 span of the parameter curve, cut out of the full range.
    const SPAN_LO: f64 = 0.1;
    const SPAN_HI: f64 = 0.9;

    /// The dimensionless entity tolerance `entity_tau(TOLERANCE)` of the
    /// numerically-legacy context, built through the REAL constructor
    /// `ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE)` — not
    /// `unscaled_legacy()`, whose constructor call sites GATE-4 ratchets. The
    /// constant arguments are valid by construction.
    fn legacy_tau() -> f64 {
        match ToleranceCtx::new(1.0, TOLERANCE, TOLERANCE, TOLERANCE) {
            Ok(ctx) => ctx.value.entity_tau(TOLERANCE),
            Err(_) => unreachable!("the constant context arguments are valid"),
        }
    }

    /// The plane witness's surface: `S(u, v) = (u, v, u + v)`, an oblique slab
    /// whose two partials are distinct. Copied from `truck-evidence`'s
    /// deviation and pcurve test modules.
    fn plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        )
    }

    /// The quadratic Bézier `c(t) = (t, t²)` on `[0, 1]`, control points
    /// `(0, 0), (1/2, 0), (1, 1)`.
    fn parabola2() -> BSplineCurve<Point2> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(0.5, 0.0),
                Point2::new(1.0, 1.0),
            ],
        )
    }

    /// The leader witness: the SAME curve as the flattened plane pcurve, as a
    /// `BSplineCurve<Point3>` with control points `(0,0,0), (1/2,0,1/2),
    /// (1,1,2)` on `KnotVec::bezier_knot(2)` — bit-exact agreement with the
    /// composed carrier.
    fn leader_witness() -> BSplineCurve<Point3> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.5, 0.0, 0.5),
                Point3::new(1.0, 1.0, 2.0),
            ],
        )
    }

    /// The sphere witness's parameter curve, from `pcurve.rs`'s tests:
    /// `c(t) = (u(t), 0)` with `u(t) = 1/4 + 3t/4` on `[0, 1]`.
    fn sphere_param() -> BSplineCurve<Point2> {
        BSplineCurve::new(
            KnotVec::bezier_knot(2),
            vec![
                Point2::new(0.25, 0.0),
                Point2::new(0.625, 0.0),
                Point2::new(1.0, 0.0),
            ],
        )
    }

    /// The sphere surface of the route-2 carrier: `S(u, v) = c + 2·(sin u·cos
    /// v, sin u·sin v, cos u)`, so the `v = 0` trace is a circle of radius 2
    /// in the plane `y = −1`.
    fn sphere_surface() -> Sphere {
        Sphere::new(Point3::new(1.0, -1.0, 0.5), 2.0)
    }

    /// The sphere witness restricted to `[lo, hi]` of the parameter curve: the
    /// pcurve span the checker reads off `parameter_range()`.
    fn sphere_restricted(lo: f64, hi: f64) -> PCurve<BSplineCurve<Point2>, Sphere> {
        let mut curve = sphere_param();
        let _tail = curve.cut(hi);
        let mid = curve.cut(lo);
        PCurve::new(mid, sphere_surface())
    }

    /// The composed sphere-arc point at parameter `t`: `S(u(t), 0)` with
    /// `u(t) = 1/4 + 3t/4`.
    fn arc_point(t: f64) -> Point3 {
        let u = 0.25 + 0.75 * t;
        let (sinu, cosu) = u.sin_cos();
        Point3::new(1.0 + 2.0 * sinu, -1.0, 0.5 + 2.0 * cosu)
    }

    /// The route-2 leader: the sphere witness's meridional arc as a single
    /// quadratic B-spline on `[lo, hi]`, interpolating the arc at `lo`, the
    /// midpoint and `hi`, offset by `dz` in z. The interpolation error is tiny
    /// next to the offset, so the whole-span enclosure cannot read the
    /// violation off a loose box: the bisection route must prove it.
    fn sphere_arc_leader(lo: f64, hi: f64, dz: f64) -> BSplineCurve<Point3> {
        let mid = (lo + hi) / 2.0;
        let (p0, p2) = (arc_point(lo), arc_point(hi));
        let pm = arc_point(mid);
        let p1 = Point3::new(
            2.0 * pm.x - (p0.x + p2.x) / 2.0,
            2.0 * pm.y - (p0.y + p2.y) / 2.0,
            2.0 * pm.z - (p0.z + p2.z) / 2.0,
        );
        let mut kv = KnotVec::bezier_knot(2);
        kv.transform(hi - lo, lo);
        let mut leader = BSplineCurve::new(kv, vec![p0, p1, p2]);
        leader.transform_control_points(|p| *p += Vector3::unit_z() * dz);
        leader
    }

    /// The route-1 exact edge: the plane-pcurve carrier against the leader it
    /// composes to bit-exactly, on an edge carrying both.
    fn exact_edge() -> Edge<usize, BSplineCurve<Point3>, PCurve<BSplineCurve<Point2>, Plane>> {
        let v = Vertex::news([0usize, 1usize]);
        Edge::new(&v[0], &v[1], leader_witness()).with_pcurve(PCurve::new(parabola2(), plane()))
    }

    #[test]
    fn same_parameter_exact_pcurve_edge_holds() {
        let tau = legacy_tau();
        let mut budget = Budget::new(1 << 16, 0, 0);
        let out = check_edge(&exact_edge(), ParamMap::IDENTITY, tau, &mut budget);
        let certified = match out {
            Ok(certified) => certified,
            Err(_) => unreachable!("the exact pair must certify"),
        };
        assert_eq!(certified.cert.props.get(Prop::SameParameter), Truth::True);
        assert_eq!(certified.cert.method, Method::Interval);
    }

    #[test]
    fn same_parameter_offset_pcurve_edge_violates() {
        let tau = legacy_tau();
        let v = Vertex::news([0usize, 1usize]);
        // The leader translated by 2·tau in z: the difference spline's z
        // control points sit at −2·tau, so the hull's lower norm bound proves
        // the violation outright.
        let mut leader = leader_witness();
        leader.transform_control_points(|p| *p += Vector3::unit_z() * (2.0 * tau));
        let edge = Edge::new(&v[0], &v[1], leader).with_pcurve(PCurve::new(parabola2(), plane()));
        let mut budget = Budget::new(1 << 16, 0, 0);
        let err = match check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget) {
            Ok(_) => unreachable!("the offset edge must violate"),
            Err(e) => e,
        };
        match err {
            Refusal::ForwardToleranceExceeded { bound, allowed } => {
                assert!(bound > tau, "bound {bound} must exceed tau {tau}");
                assert_eq!(allowed, tau);
            }
            other => unreachable!("expected ForwardToleranceExceeded, got {other:?}"),
        }
    }

    #[test]
    fn same_parameter_none_pcurve_is_vacuously_ok() {
        let tau = legacy_tau();
        let v = Vertex::news([0usize, 1usize]);
        let edge: Edge<usize, BSplineCurve<Point3>, ()> = Edge::new(&v[0], &v[1], leader_witness());
        let mut budget = Budget::new(1 << 16, 0, 0);
        let certified = match check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget) {
            Ok(certified) => certified,
            Err(_) => unreachable!("the vacuous edge must hold"),
        };
        assert_eq!(certified.cert.method, Method::None);
    }

    #[test]
    fn same_parameter_route2_offset_violates() {
        let tau = ROUTE2_TAU;
        let v = Vertex::news([0usize, 1usize]);
        // The carrier is a sphere pcurve — a curved surface, not flattenable —
        // so BG-CE-002's route 1 bails and the bisection route must prove the
        // z-offset violation.
        let edge = Edge::new(
            &v[0],
            &v[1],
            sphere_arc_leader(SPAN_LO, SPAN_HI, ROUTE2_OFFSET_MULT * tau),
        )
        .with_pcurve(sphere_restricted(SPAN_LO, SPAN_HI));
        let mut budget = Budget::new(1 << 20, 0, 0);
        let err = match check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget) {
            Ok(_) => unreachable!("the offset sphere pair must violate"),
            Err(e) => e,
        };
        match err {
            Refusal::ForwardToleranceExceeded { bound, allowed } => {
                assert!(bound > tau, "bound {bound} must exceed tau {tau}");
                assert_eq!(allowed, tau);
            }
            other => unreachable!("expected ForwardToleranceExceeded, got {other:?}"),
        }
    }

    #[test]
    fn same_parameter_zero_budget_is_unresolved() {
        let tau = ROUTE2_TAU;
        let v = Vertex::news([0usize, 1usize]);
        let edge = Edge::new(
            &v[0],
            &v[1],
            sphere_arc_leader(SPAN_LO, SPAN_HI, ROUTE2_OFFSET_MULT * tau),
        )
        .with_pcurve(sphere_restricted(SPAN_LO, SPAN_HI));
        // The whole-span enclosure is too loose to decide, so the first
        // bisection spend fails immediately and the deviation certificate is
        // the witness.
        let mut budget = Budget::new(0, 0, 0);
        let err = match check_edge(&edge, ParamMap::IDENTITY, tau, &mut budget) {
            Ok(_) => unreachable!("the zero-budget pair must refuse"),
            Err(e) => e,
        };
        match err {
            Refusal::NumericallyUnresolved { spent, witness } => {
                assert_eq!(spent.subdiv, 0);
                assert_eq!(witness, UnresolvedWitness::DeviationUncertified);
            }
            other => unreachable!("expected NumericallyUnresolved, got {other:?}"),
        }
    }
}
