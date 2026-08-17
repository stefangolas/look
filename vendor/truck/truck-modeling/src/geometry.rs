use super::*;
use derive_more::{From, TryInto};
use serde::{Deserialize, Serialize};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};
#[doc(hidden)]
pub use truck_geometry::prelude::{algo, inv_or_zero};
pub use truck_geometry::{decorators::*, nurbs::*, specifieds::*};

/// 3-dimensional curve
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    From,
    TryInto,
    ParametricCurve,
    BoundedCurve,
    ParameterDivision1D,
    Cut,
    Invertible,
    SearchNearestParameterD1,
    SearchParameterD1,
)]
pub enum Curve {
    /// line
    Line(Line<Point3>),
    /// 3-dimensional B-spline curve
    BSplineCurve(BSplineCurve<Point3>),
    /// 3-dimensional NURBS curve
    NurbsCurve(NurbsCurve<Vector4>),
    /// intersection curve
    IntersectionCurve(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>),
}

macro_rules! derive_curve_method {
    ($curve: expr, $method: expr, $($ver: ident),*) => {
        match $curve {
            Curve::Line(got) => $method(got, $($ver), *),
            Curve::BSplineCurve(got) => $method(got, $($ver), *),
            Curve::NurbsCurve(got) => $method(got, $($ver), *),
            Curve::IntersectionCurve(got) => $method(got, $($ver), *),
        }
    };
}

macro_rules! derive_curve_self_method {
    ($curve: expr, $method: expr, $($ver: ident),*) => {
        match $curve {
            Curve::Line(got) => Curve::Line($method(got, $($ver), *)),
            Curve::BSplineCurve(got) => Curve::BSplineCurve($method(got, $($ver), *)),
            Curve::NurbsCurve(got) => Curve::NurbsCurve($method(got, $($ver), *)),
            Curve::IntersectionCurve(got) => Curve::IntersectionCurve($method(got, $($ver), *)),
        }
    };
}

impl Transformed<Matrix4> for Curve {
    fn transform_by(&mut self, trans: Matrix4) {
        derive_curve_method!(self, Transformed::transform_by, trans);
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        derive_curve_self_method!(self, Transformed::transformed, trans)
    }
}

impl From<IntersectionCurve<BSplineCurve<Point3>, Surface, Surface>> for Curve {
    fn from(c: IntersectionCurve<BSplineCurve<Point3>, Surface, Surface>) -> Curve {
        let (surface0, surface1, leader) = c.destruct();
        Curve::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(leader.into()),
        ))
    }
}

impl ToSameGeometry<Curve> for Line<Point3> {
    #[inline]
    fn to_same_geometry(&self) -> Curve {
        Curve::from(*self)
    }
}

impl ToSameGeometry<Curve> for Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    #[inline]
    fn to_same_geometry(&self) -> Curve {
        Curve::NurbsCurve(self.to_same_geometry())
    }
}

impl ToSameGeometry<Curve> for BSplineCurve<Point3> {
    #[inline]
    fn to_same_geometry(&self) -> Curve {
        Curve::from(self.clone())
    }
}

impl Curve {
    /// Into non-ratinalized 4-dimensional B-spline curve
    pub fn lift_up(&self) -> BSplineCurve<Vector4> {
        match self {
            Curve::Line(curve) => Curve::BSplineCurve((*curve).into()).lift_up(),
            Curve::BSplineCurve(curve) => BSplineCurve::new(
                curve.knot_vec().clone(),
                curve
                    .control_points()
                    .iter()
                    .map(|pt| pt.to_vec().extend(1.0))
                    .collect(),
            ),
            Curve::NurbsCurve(curve) => curve.non_rationalized().clone(),
            Curve::IntersectionCurve(_) => {
                unimplemented!("intersection curve cannot connect by homotopy")
            }
        }
    }
}

/// 3-dimensional surfaces
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    From,
    TryInto,
    ParametricSurface,
    ParameterDivision2D,
    Invertible,
    SearchParameterD2,
)]
pub enum Surface {
    /// Plane
    Plane(Plane),
    /// 3-dimensional B-spline surface
    BSplineSurface(BSplineSurface<Point3>),
    /// 3-dimensional NURBS Surface
    NurbsSurface(NurbsSurface<Vector4>),
    /// revoluted curve
    RevolutedCurve(Processor<RevolutedCurve<Curve>, Matrix4>),
}

macro_rules! derive_surface_method {
    ($surface: expr, $method: expr, $($ver: ident),*) => {
        match $surface {
            Self::Plane(got) => $method(got, $($ver), *),
            Self::BSplineSurface(got) => $method(got, $($ver), *),
            Self::NurbsSurface(got) => $method(got, $($ver), *),
            Self::RevolutedCurve(got) => $method(got, $($ver), *),
        }
    };
}

macro_rules! derive_surface_self_method {
    ($surface: expr, $method: expr, $($ver: ident),*) => {
        match $surface {
            Self::Plane(got) => Self::Plane($method(got, $($ver), *)),
            Self::BSplineSurface(got) => Self::BSplineSurface($method(got, $($ver), *)),
            Self::NurbsSurface(got) => Self::NurbsSurface($method(got, $($ver), *)),
            Self::RevolutedCurve(got) => Self::RevolutedCurve($method(got, $($ver), *)),
        }
    };
}

impl ParametricSurface3D for Surface {
    #[inline(always)]
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        derive_surface_method!(self, ParametricSurface3D::normal, u, v)
    }
}

impl Transformed<Matrix4> for Surface {
    fn transform_by(&mut self, trans: Matrix4) {
        derive_surface_method!(self, Transformed::transform_by, trans);
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        derive_surface_self_method!(self, Transformed::transformed, trans)
    }
}

impl IncludeCurve<Curve> for Surface {
    fn include(&self, curve: &Curve) -> Outcome<bool> {
        match self {
            Surface::BSplineSurface(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
            },
            Surface::NurbsSurface(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
            },
            Surface::Plane(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
            },
            Surface::RevolutedCurve(surface) => match surface.entity_curve() {
                &Curve::Line(curve) => {
                    self.include(&Curve::BSplineCurve(BSplineCurve::from(curve)))
                }
                Curve::BSplineCurve(entity_curve) => {
                    let surface = RevolutedCurve::by_revolution(
                        entity_curve,
                        surface.origin(),
                        surface.axis(),
                    );
                    match curve {
                        &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                        Curve::BSplineCurve(curve) => surface.include(curve),
                        Curve::NurbsCurve(curve) => surface.include(curve),
                        Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                    }
                }
                Curve::NurbsCurve(entity_curve) => {
                    let surface = RevolutedCurve::by_revolution(
                        entity_curve,
                        surface.origin(),
                        surface.axis(),
                    );
                    match curve {
                        &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                        Curve::BSplineCurve(curve) => surface.include(curve),
                        Curve::NurbsCurve(curve) => surface.include(curve),
                        Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                    }
                }
                Curve::IntersectionCurve(_) => {
                    // BG-S0-001: `self` is a surface of revolution whose
                    // profile is itself an intersection curve. Its inclusion
                    // question has no certified answer yet (no carrier-identity
                    // mechanism, no enclosure machinery); refusal, not abort.
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    })
                }
            },
        }
    }
}

impl Surface {
    /// BG-S0-001: `include` of an `IntersectionCurve` must not abort.
    ///
    /// The spec's algorithm (surface-identity short-circuit, leader-polyline
    /// sampling, `NumericallyUnresolved`) is deliberately narrowed here for
    /// epistemic correctness:
    ///
    /// - The **ssi-carrier → `Proven(true, Exact)`** short-circuit is NOT
    ///   taken. It requires carrier identity (BG-CE-004) and the `EntityId`
    ///   mechanism of BG-CE-003, which are not yet implemented. Two
    ///   independently constructed surfaces with identical parameters are
    ///   distinct carriers; structural equality would manufacture a
    ///   `Proven(true)` where the answer is not certified. The branch lands
    ///   with BG-CE-003; until then the question is a refusal.
    /// - The **leader-witness negative → `Proven(false)`** is taken only where
    ///   exclusion is genuinely decidable: a `Plane` carrier, by signed normal
    ///   distance beyond a margin over the representation tolerance. A
    ///   numerical inverse-search failure on any other carrier is not proof of
    ///   non-membership, so those negatives are deferred to the
    ///   enclosure/certified-search machinery (BG-ENC, BG-NUM).
    fn include_intersection_curve(
        &self,
        ic: &IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>,
    ) -> Outcome<bool> {
        match self {
            Surface::Plane(plane) => plane_include_intersection_curve(plane, ic.leader()),
            _ => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::UncertifiedContainment,
            }),
        }
    }
}

/// BG-S0-001: decide `Plane ∋ leader` by signed normal distance.
///
/// A negative is conclusive: if a sampled point of the leader lies off the
/// plane by more than `LEADER_WITNESS_MARGIN × TOLERANCE`, the leader — and
/// hence the intersection curve it carries — is provably not in the plane
/// (`Proven(false)`, μ = Float, the "leader-witness" rule). A positive is NOT
/// conclusive: sampling cannot prove containment, so when every sample is
/// within tolerance the answer is `NumericallyUnresolved`
/// (`UncertifiedContainment`), never `Proven(true)`.
fn plane_include_intersection_curve(plane: &Plane, leader: &Curve) -> Outcome<bool> {
    let ctx = ToleranceCtx::unscaled_legacy();
    let origin = plane.origin();
    let normal = plane.normal();
    // Bounded uniform sample of the leader (H-5: a documented bound, not a
    // bare loop; the count is a dimensionless sample budget, not a length).
    const LEADER_WITNESS_SAMPLES: usize = 32;
    // Dimensionless margin over the representation tolerance; named for the
    // quantity it multiplies. `TOLERANCE` is now the `tau_rep` that
    // BG-TOL-001's `ToleranceCtx` supplies via `length_margin()` (H-3).
    const LEADER_WITNESS_MARGIN: f64 = 8.0;
    // Evaluating the leader of an intersection curve via `subs` can panic
    // (H-1): `IntersectionCurve::subs` unwraps its own projection search. A
    // nested intersection leader has no certified witness here, so refuse
    // rather than evaluate.
    if matches!(*leader, Curve::IntersectionCurve(_)) {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::UncertifiedContainment,
        });
    }
    let (t0, t1) = leader.range_tuple();
    for i in 0..LEADER_WITNESS_SAMPLES {
        let t = t0 + (t1 - t0) * (i as f64) / (LEADER_WITNESS_SAMPLES as f64);
        let signed = (leader.subs(t) - origin).dot(normal);
        if signed.abs() > LEADER_WITNESS_MARGIN * ctx.length_margin() {
            // BG-TOL-001: model
            return Ok(Certified::new(
                false,
                Certificate {
                    props: PropMap::new(),
                    method: Method::Float,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
    }
    Err(Refusal::NumericallyUnresolved {
        spent: Budget::new(0, 0, 0),
        witness: UnresolvedWitness::UncertifiedContainment,
    })
}

impl IncludeCurve<Curve> for Plane {
    fn include(&self, curve: &Curve) -> Outcome<bool> {
        match curve {
            // BG-S0-001: the lifted control-point test below cannot touch an
            // `IntersectionCurve` (`Curve::lift_up` aborts on it), so route it
            // through the plane negative witness.
            Curve::IntersectionCurve(ic) => plane_include_intersection_curve(self, ic.leader()),
            _ => Ok(Certified::new(
                curve.lift_up().control_points().iter().all(|v| {
                    let p = v.to_point();
                    self.search_parameter(p, None, 1).is_some()
                }),
                Certificate {
                    props: PropMap::new(),
                    method: Method::Float,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            )),
        }
    }
}

impl ToSameGeometry<Surface> for Plane {
    fn to_same_geometry(&self) -> Surface {
        (*self).into()
    }
}

impl ToSameGeometry<Surface> for RevolutedCurve<Curve> {
    fn to_same_geometry(&self) -> Surface {
        Surface::RevolutedCurve(Processor::new(self.clone()))
    }
}

impl SearchNearestParameter<D2> for Surface {
    type Point = Point3;
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        point: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        match self {
            Surface::Plane(plane) => plane.search_nearest_parameter(point, hint, trials),
            Surface::BSplineSurface(bspsurface) => {
                bspsurface.search_nearest_parameter(point, hint, trials)
            }
            Surface::NurbsSurface(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::RevolutedCurve(rotted) => {
                let hint = match hint.into() {
                    SPHint2D::Parameter(hint0, hint1) => (hint0, hint1),
                    SPHint2D::Range(x, y) => algo::surface::presearch(rotted, point, (x, y), 100),
                    SPHint2D::None => {
                        algo::surface::presearch(rotted, point, rotted.range_tuple(), 100)
                    }
                };
                algo::surface::search_nearest_parameter(rotted, point, hint, trials)
            }
        }
    }
}

impl ToSameGeometry<Surface> for HomotopySurface<Curve, Curve> {
    fn to_same_geometry(&self) -> Surface {
        let curve0 = self.curve0().clone().lift_up();
        let curve1 = self.curve1().clone().lift_up();
        NurbsSurface::new(BSplineSurface::homotopy(curve0, curve1)).into()
    }
}

impl ToSameGeometry<Surface> for ExtrudedCurve<Curve, Vector3> {
    fn to_same_geometry(&self) -> Surface {
        let (curve0, vector) = (self.entity_curve(), self.extruding_vector());
        let trsl = Matrix4::from_translation(vector);
        let curve1 = self.entity_curve().transformed(trsl);
        match (curve0, curve1) {
            (Curve::Line(line), Curve::Line(_)) => {
                Plane::new(line.0, line.1, line.0 + vector).into()
            }
            (Curve::BSplineCurve(curve0), Curve::BSplineCurve(curve1)) => {
                BSplineSurface::homotopy(curve0.clone(), curve1.clone()).into()
            }
            (Curve::NurbsCurve(curve0), Curve::NurbsCurve(curve1)) => {
                NurbsSurface::new(BSplineSurface::homotopy(
                    curve0.non_rationalized().clone(),
                    curve1.non_rationalized().clone(),
                ))
                .into()
            }
            (Curve::IntersectionCurve(_), Curve::IntersectionCurve(_)) => {
                // BG-S0-003: `to_same_geometry` has no error channel, and an
                // intersection-curve carrier cannot be evaluated here without
                // unwinding (`IntersectionCurve::subs` unwraps its own
                // projection search, H-1), so no approximation path exists.
                // The honest total behaviour is a documented degenerate
                // surface: the returned plane's image does NOT claim to match
                // the extrusion. The certified answer for this pair lives in
                // `try_to_same_geometry`, which refuses with
                // `UnsupportedEnvelope(NonCanonicalCarrier)`.
                Surface::Plane(Plane::xy())
            }
            _ => unreachable!(),
        }
    }

    fn try_to_same_geometry(&self) -> Outcome<Surface> {
        // BG-S0-003: the two section curves of an extrusion always share the
        // entity curve's variant (`curve1` is `curve0` pushed by the
        // extrusion vector), so an `IntersectionCurve` entity is exactly the
        // `(IntersectionCurve, IntersectionCurve)` pair. That carrier is
        // outside the canonical set (H-2): refuse rather than unwind.
        //
        // The non-ISC arm replicates the trait default's certificate because
        // an override cannot call the trait's default without recursing into
        // itself — the default body is shadowed by this override, not a
        // callable sibling (BG-S0-003).
        match self.entity_curve() {
            Curve::IntersectionCurve(_) => Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier,
            )),
            _ => Ok(Certified::new(
                self.to_same_geometry(),
                Certificate {
                    props: PropMap::new(),
                    method: Method::Float,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            )),
        }
    }
}

#[cfg(test)]
// BG-S0-001 tests. The certificates are inspected by pattern on hand-built
// witnesses — not paths reachable from untrusted geometry, so the H-1 deny
// lints on unwrap/expect do not apply to the assertions here.
mod include_intersection_curve_tests {
    use super::*;

    /// The plane z = 0 through the origin.
    fn zx_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// The plane x = 0 through the origin.
    fn yz_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
    }

    /// The plane y = 0 through the origin.
    fn xz_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
    }

    fn intersection_curve(surface0: Surface, surface1: Surface, leader: Curve) -> Curve {
        Curve::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(leader),
        ))
    }

    #[test]
    fn ssi_carrier_shortcut_is_deferred_until_entity_id() {
        // Spec test 1, interim: the ssi-carrier → `Proven(true, Exact)` branch
        // requires carrier identity (BG-CE-004 / the `EntityId` of BG-CE-003),
        // which is not implemented. Even though `surface0` IS the queried plane
        // (structurally identical value), `include` must refuse rather than
        // manufacture a `Proven(true)` from structural equality — two
        // independently constructed planes with identical parameters are
        // distinct carriers.
        let plane = zx_plane();
        let query = Surface::Plane(plane);
        let leader = Curve::Line(Line(
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ));
        let curve = intersection_curve(Surface::Plane(plane), Surface::Plane(yz_plane()), leader);
        let out = query.include(&curve);
        assert!(
            matches!(
                out,
                Err(Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::UncertifiedContainment,
                    ..
                })
            ),
            "expected NumericallyUnresolved, got {out:?}"
        );
    }

    #[test]
    fn isc_demonstrably_off_plane_is_proven_false() {
        // Spec test 2: an ISC lying off the plane → `Proven(false)`, μ = Float
        // (the "leader-witness" signed normal distance beyond the margin).
        let query = Surface::Plane(zx_plane());
        let leader = Curve::Line(Line(
            Point3::new(0.0, 0.0, -1.0),
            Point3::new(0.0, 0.0, 1.0),
        ));
        let curve = intersection_curve(
            Surface::Plane(xz_plane()),
            Surface::Plane(yz_plane()),
            leader,
        );
        let out = query.include(&curve);
        assert!(
            matches!(
                out,
                Ok(Certified {
                    value: false,
                    cert: Certificate {
                        method: Method::Float,
                        ..
                    }
                })
            ),
            "expected Proven(false, Float), got {out:?}"
        );
    }

    #[test]
    fn isc_of_other_surfaces_lying_in_plane_is_unresolved() {
        // Spec test 3 (epistemically critical): an ISC of two *other* surfaces
        // that happens to lie in the queried plane must be
        // `NumericallyUnresolved`, NOT `Proven(true)` — sampling cannot prove
        // containment. This is the test that catches a future "helpful"
        // strengthening of the sampling path into a wrong-but-confident answer.
        let query = Surface::Plane(zx_plane());
        let leader = Curve::Line(Line(
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        ));
        // Two planes whose intersection line is the x-axis, which lies in the
        // queried plane z = 0.
        let surface0 = Surface::Plane(Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 1.0),
        ));
        let surface1 = Surface::Plane(Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, -1.0),
        ));
        let curve = intersection_curve(surface0, surface1, leader);
        let out = query.include(&curve);
        assert!(
            matches!(
                out,
                Err(Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::UncertifiedContainment,
                    ..
                })
            ),
            "expected NumericallyUnresolved, got {out:?}"
        );
    }

    #[test]
    fn boolean_derived_face_consistency_returns() {
        // Spec regression: a face whose boundary carries an
        // `IntersectionCurve` (the variant Booleans produce) previously aborted
        // in `Surface::include` via `unimplemented!()`. It must now return —
        // here through `Face::is_geometric_consistent`, which fails closed on
        // `NumericallyUnresolved`.
        let v0 = Vertex::new(Point3::new(0.0, 0.0, -1.0));
        let v1 = Vertex::new(Point3::new(0.0, 0.0, 1.0));
        let isc = intersection_curve(
            Surface::Plane(xz_plane()),
            Surface::Plane(yz_plane()),
            Curve::Line(Line(
                Point3::new(0.0, 0.0, -1.0),
                Point3::new(0.0, 0.0, 1.0),
            )),
        );
        let wire: Wire = vec![Edge::new(&v0, &v1, isc.clone()), Edge::new(&v1, &v0, isc)].into();
        let face = Face::new(vec![wire], Surface::Plane(zx_plane()));
        // The ISC edge is off the capping plane, so the face is certified
        // inconsistent — the point of the regression is that this returns
        // instead of aborting.
        assert!(!face.is_geometric_consistent());
    }
}

#[cfg(test)]
// BG-S0-003 tests. The certificates and surfaces are inspected on hand-built
// witnesses — not paths reachable from untrusted geometry, so the H-1 deny
// lints on unwrap/expect do not apply to the assertions here.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod extrude_intersection_curve_tests {
    use super::*;

    /// The plane z = 0 through the origin.
    fn zx_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )
    }

    /// The plane x = 0 through the origin.
    fn yz_plane() -> Plane {
        Plane::new(
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
    }

    fn intersection_curve(surface0: Surface, surface1: Surface, leader: Curve) -> Curve {
        Curve::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(leader),
        ))
    }

    /// An `ExtrudedCurve` whose entity curve is an `IntersectionCurve` — the
    /// pair Booleans produce. `to_same_geometry` previously aborted on it.
    fn extruded_intersection_curve_pair() -> ExtrudedCurve<Curve, Vector3> {
        let isc = intersection_curve(
            Surface::Plane(zx_plane()),
            Surface::Plane(yz_plane()),
            Curve::Line(Line(
                Point3::new(0.0, 0.0, -1.0),
                Point3::new(0.0, 0.0, 1.0),
            )),
        );
        ExtrudedCurve::by_extrusion(isc, Vector3::unit_z())
    }

    #[test]
    fn extrude_intersection_curve_pair_refuses() {
        // The certified path refuses the (ISC, ISC) pair instead of aborting:
        // `UnsupportedEnvelope(NonCanonicalCarrier)`, never a panic.
        let extruded = extruded_intersection_curve_pair();
        let out: Outcome<Surface> = extruded.try_to_same_geometry();
        assert!(
            matches!(
                out,
                Err(Refusal::UnsupportedEnvelope(
                    EnvelopeCase::NonCanonicalCarrier
                ))
            ),
            "expected UnsupportedEnvelope(NonCanonicalCarrier), got {out:?}"
        );
    }

    #[test]
    fn extrude_intersection_curve_pair_does_not_unwind() {
        // `to_same_geometry` is infallible, so the same input must come back
        // through it without unwinding; the catch is asserted not to be
        // needed.
        let extruded = extruded_intersection_curve_pair();
        let result: std::thread::Result<Surface> =
            std::panic::catch_unwind(|| extruded.to_same_geometry());
        assert!(
            result.is_ok(),
            "to_same_geometry unwound on an intersection-curve pair"
        );
    }

    #[test]
    fn extrude_non_isc_pairs_unchanged() {
        // Every non-ISC pair must be semantically inert: `try_to_same_geometry`
        // succeeds and its surface equals what `to_same_geometry` produced.
        let vector = Vector3::unit_z();
        let pairs = [
            ExtrudedCurve::by_extrusion(
                Curve::Line(Line(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0))),
                vector,
            ),
            ExtrudedCurve::by_extrusion(
                Curve::BSplineCurve(BSplineCurve::new(
                    KnotVec::bezier_knot(1),
                    vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
                )),
                vector,
            ),
            ExtrudedCurve::by_extrusion(
                Curve::NurbsCurve(NurbsCurve::new(BSplineCurve::new(
                    KnotVec::bezier_knot(1),
                    vec![
                        Point3::new(0.0, 0.0, 0.0).to_vec().extend(1.0),
                        Point3::new(1.0, 0.0, 0.0).to_vec().extend(1.0),
                    ],
                ))),
                vector,
            ),
        ];
        for extruded in pairs {
            let certified: Certified<Surface> = extruded
                .try_to_same_geometry()
                .expect("non-ISC extrusion must not refuse");
            let before: Surface = extruded.to_same_geometry();
            for i in 0..=4 {
                let u = i as f64 / 4.0;
                for j in 0..=4 {
                    let v = j as f64 / 4.0;
                    assert!(
                        certified.value.subs(u, v) == before.subs(u, v),
                        "surface diverged from to_same_geometry at ({u}, {v})"
                    );
                }
            }
        }
    }
}
