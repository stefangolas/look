//! Certified ambient schema — stage 1 of the refinement architecture.
//!
//! `REFINEMENT_AUDIT.md` names `raw surface accessors -> CertifiedParametricAmbient`
//! as the earliest missing transition, and its status as *absent* rather than
//! merely unproved. Every downstream object is parameterised by the ambient:
//! the lift consumes periods, the arrangement consumes the lattice, the
//! material solve consumes native boundaries. None of those facts is
//! established anywhere. `u_period()`, `v_period()` and `try_range_tuple()` are
//! read directly at points of use — three unrelated accessors with nothing
//! binding them and no evidence attached.
//!
//! FORMAL_SYSTEM Definition 7 requires the ambient to be one object
//! `(Ω, Λ, N, Σ, S, C)` where `C` carries certificates for its propositions.
//!
//! # Periodicity is proved from the representation, never from sampling
//!
//! An earlier draft of this module certified periodicity by checking
//! `‖S(u+P,v) − S(u,v)‖ < ε` on a sampled grid. **That is a compatibility
//! diagnostic, not a periodicity proof**, and it was wrong to call it a
//! certificate: agreement at finitely many points establishes nothing about
//! the surface between them, and a near-miss at a coarse tolerance would have
//! certified a period that does not exist.
//!
//! [`PeriodWitness`] therefore admits only representation-derived evidence, and
//! there is deliberately no `NumericallyVerified` variant. Where exact evidence
//! is unavailable the constructor returns [`SchemaFailure::PeriodUncertified`]
//! rather than inventing weaker grounds.
//!
//! # The declared range is not the face domain
//!
//! `try_range_tuple()` reports the range of the *primitive the surface was
//! built from*, not of any face referencing it (`PAR-RANGE-INHERITANCE-001`):
//! `Line::parameter_range` is `[0,1]` unconditionally and `RevolutedCurve`
//! inherits it, so a cone built from a revolved line declares one unit of
//! generatrix starting at whatever reference radius the exporter chose. A
//! primitive's default range must never silently become the face domain, so
//! [`CertifiedDomain`] records the *evidence* for every bound and a
//! non-periodic axis with no face evidence yields
//! [`SchemaFailure::DomainUnderdetermined`] rather than a synthesised rectangle
//! that can masquerade as physical trim geometry.

use crate::cgmath::{One, Vector2};
use truck_geometry::prelude::{BoundedCurve, ParametricCurve3D, ParametricSurface};
use truck_geometry::prelude::{Plane, Processor, RevolutedCurve};

use super::schema::DeckLattice;

/// Which parameter axis a fact concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamAxis {
    /// The `u` axis.
    U,
    /// The `v` axis.
    V,
}

/// Why an ambient schema could not be established.
///
/// Every variant is a refusal to assert something unproven. None of them is a
/// statement that the face is invalid — that judgement belongs to the caller
/// (MF §31: a detector establishes a fact, policy decides what to do).
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaFailure {
    /// The surface representation is not one this constructor can read
    /// structurally. Expanding coverage is the fix; guessing is not.
    UnsupportedSurfaceType,
    /// A period is declared but no representation-derived witness establishes
    /// it. Notably: a spline whose source does not explicitly declare
    /// periodicity, where the accessor's answer rests on nothing.
    PeriodUncertified {
        /// Which axis declared the uncertifiable period.
        axis: ParamAxis,
        /// The value the surface reported.
        declared: f64,
    },
    /// A non-periodic axis whose extent no face evidence determines. Its
    /// working window must come from the face's own bounds, not from the
    /// supporting primitive.
    DomainUnderdetermined {
        /// Which axis is underdetermined.
        axis: ParamAxis,
    },
    /// A collapsed stratum exists but is not in the exact schema family this
    /// constructor supports.
    UnsupportedSingularSchema,
}

/// How a period is known. Representation-derived evidence only.
#[derive(Debug, Clone, PartialEq)]
pub enum PeriodWitness {
    /// The parameterisation *is* a rotation: `RevolutedCurve::subs(u,v)` is
    /// `origin + rotation_matrix(v) · (curve(u) − origin)`, so the angular
    /// coordinate has period `2π` by construction of the map, for every
    /// generatrix. This is what makes cylinder, cone, sphere and torus one case
    /// rather than four — in `truck-modeling` they are all `RevolutedCurve`.
    ExactRevolutionAngle,
    /// The generatrix curve is itself periodic and the surface inherits it.
    /// Only as strong as the curve's own evidence, and recorded separately so
    /// it cannot be mistaken for the revolution witness.
    InheritedFromGeneratrix {
        /// The generatrix period.
        curve_period: f64,
    },
}

/// A period together with the evidence establishing it.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedPeriod {
    /// Which axis.
    pub axis: ParamAxis,
    /// The period. Guaranteed finite and strictly positive by construction.
    pub value: f64,
    /// Why it is known.
    pub witness: PeriodWitness,
}

/// Where a domain bound's authority comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainEvidence {
    /// The surface's own exact schema determines it — a full angular turn.
    ExactSurfaceSchema,
    /// Derived from certified collapsed strata.
    DerivedFromCertifiedStrata,
    /// Established by this face's own bounds. Supplied by the caller; the
    /// ambient cannot know it.
    StepFaceEvidence,
}

/// One axis's certified extent, with the source of its authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedAxisDomain {
    /// The interval.
    pub interval: (f64, f64),
    /// What establishes it.
    pub evidence: DomainEvidence,
}

/// The certified parameter domain.
///
/// Deliberately *not* `try_range_tuple()`. See the module comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedDomain {
    /// The `u` extent, if determined.
    pub u: Option<CertifiedAxisDomain>,
    /// The `v` extent, if determined.
    pub v: Option<CertifiedAxisDomain>,
}

/// Face-supplied evidence the ambient cannot derive for itself.
///
/// The working extent of a non-periodic axis is a property of the face, not of
/// the primitive, so the face must supply it. Passing `None` is a statement
/// that the face did not determine it, and yields
/// [`SchemaFailure::DomainUnderdetermined`] rather than a fabricated rectangle.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FaceContext {
    /// The `u` extent this face's own bounds occupy, if any.
    pub u_extent: Option<(f64, f64)>,
    /// The `v` extent this face's own bounds occupy, if any.
    pub v_extent: Option<(f64, f64)>,
}

/// The certified ambient schema of one face's supporting surface.
///
/// After construction the tessellation path reads ambient facts from here and
/// **not** from the raw surface accessors. That type boundary is the point of
/// this stage.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedParametricAmbient {
    periods: Vec<CertifiedPeriod>,
    domain: CertifiedDomain,
}

impl CertifiedParametricAmbient {
    /// The certified `u` period, if the axis is periodic.
    pub fn u_period(&self) -> Option<f64> {
        self.period(ParamAxis::U).map(|p| p.value)
    }

    /// The certified `v` period, if the axis is periodic.
    pub fn v_period(&self) -> Option<f64> {
        self.period(ParamAxis::V).map(|p| p.value)
    }

    /// The full certificate for an axis's period.
    pub fn period(&self, axis: ParamAxis) -> Option<&CertifiedPeriod> {
        self.periods.iter().find(|p| p.axis == axis)
    }

    /// The certified domain, with per-bound evidence.
    pub fn domain(&self) -> CertifiedDomain {
        self.domain
    }

    /// The deck lattice implied by the certified periods (FS Def. 7, `Λ = LZ^r`).
    pub fn lattice(&self) -> DeckLattice {
        match (self.u_period(), self.v_period()) {
            (Some(up), Some(vp)) => DeckLattice::Rank2 {
                u_generator: Vector2::new(up, 0.0),
                v_generator: Vector2::new(0.0, vp),
            },
            (Some(up), None) => DeckLattice::Rank1 {
                generator: Vector2::new(up, 0.0),
            },
            (None, Some(vp)) => DeckLattice::Rank1 {
                generator: Vector2::new(0.0, vp),
            },
            (None, None) => DeckLattice::Rank0,
        }
    }
}

/// A surface whose ambient schema can be read from its representation.
///
/// Implemented only for representations whose periods and strata follow from
/// their construction. A type with no implementation is
/// [`SchemaFailure::UnsupportedSurfaceType`], which is the honest answer and
/// the one that keeps coverage expansion explicit.
///
/// **Layering note.** This trait belongs in `truck-geotrait`, so that
/// `truck-modeling`'s `Surface` enum can implement it by forwarding to its
/// variants and the tessellation path can take it as a bound. It is defined
/// here for now because `truck-meshalgo` cannot see `truck-modeling` and
/// `truck-modeling` cannot see `truck-meshalgo`; moving it is the prerequisite
/// for routing production lifting through this stage.
pub trait AmbientSchemaSource {
    /// Establish the ambient schema, or say precisely what could not be established.
    fn certify_ambient(
        &self,
        face_context: &FaceContext,
    ) -> Result<CertifiedParametricAmbient, SchemaFailure>;
}

/// The angular axis of any revolved curve is exactly `2π`-periodic because the
/// parameterisation applies a rotation matrix to the generatrix. This holds for
/// every generatrix, which is why one impl covers cylinder, cone, sphere and
/// torus.
impl<C: ParametricCurve3D + BoundedCurve> AmbientSchemaSource for RevolutedCurve<C> {
    fn certify_ambient(
        &self,
        face_context: &FaceContext,
    ) -> Result<CertifiedParametricAmbient, SchemaFailure> {
        let mut periods = vec![CertifiedPeriod {
            axis: ParamAxis::V,
            value: 2.0 * std::f64::consts::PI,
            witness: PeriodWitness::ExactRevolutionAngle,
        }];

        // The generatrix axis is periodic only if the generatrix itself is, and
        // that evidence is the curve's, not the revolution's.
        let u_domain = match ParametricSurface::u_period(self) {
            Some(period) if period.is_finite() && period > 0.0 => {
                periods.push(CertifiedPeriod {
                    axis: ParamAxis::U,
                    value: period,
                    witness: PeriodWitness::InheritedFromGeneratrix {
                        curve_period: period,
                    },
                });
                Some(CertifiedAxisDomain {
                    interval: (0.0, period),
                    evidence: DomainEvidence::ExactSurfaceSchema,
                })
            }
            Some(bad) => {
                return Err(SchemaFailure::PeriodUncertified {
                    axis: ParamAxis::U,
                    declared: bad,
                })
            }
            // Not periodic: its extent is the face's business, never the
            // primitive's declared `[0,1]`.
            None => Some(CertifiedAxisDomain {
                interval: face_context
                    .u_extent
                    .ok_or(SchemaFailure::DomainUnderdetermined { axis: ParamAxis::U })?,
                evidence: DomainEvidence::StepFaceEvidence,
            }),
        };

        Ok(CertifiedParametricAmbient {
            periods,
            domain: CertifiedDomain {
                u: u_domain,
                v: Some(CertifiedAxisDomain {
                    interval: (0.0, 2.0 * std::f64::consts::PI),
                    evidence: DomainEvidence::ExactSurfaceSchema,
                }),
            },
        })
    }
}

/// A plane has no period and no collapsed stratum; both extents are the face's.
impl AmbientSchemaSource for Plane {
    fn certify_ambient(
        &self,
        face_context: &FaceContext,
    ) -> Result<CertifiedParametricAmbient, SchemaFailure> {
        let axis = |extent: Option<(f64, f64)>, which| {
            extent
                .map(|interval| CertifiedAxisDomain {
                    interval,
                    evidence: DomainEvidence::StepFaceEvidence,
                })
                .ok_or(SchemaFailure::DomainUnderdetermined { axis: which })
        };
        Ok(CertifiedParametricAmbient {
            periods: Vec::new(),
            domain: CertifiedDomain {
                u: Some(axis(face_context.u_extent, ParamAxis::U)?),
                v: Some(axis(face_context.v_extent, ParamAxis::V)?),
            },
        })
    }
}

impl ParamAxis {
    /// The other axis.
    fn swapped(self) -> Self {
        match self {
            Self::U => Self::V,
            Self::V => Self::U,
        }
    }
}

impl SchemaFailure {
    /// Restate a failure in the caller's axis convention.
    fn with_axes_swapped(self) -> Self {
        match self {
            Self::PeriodUncertified { axis, declared } => Self::PeriodUncertified {
                axis: axis.swapped(),
                declared,
            },
            Self::DomainUnderdetermined { axis } => Self::DomainUnderdetermined {
                axis: axis.swapped(),
            },
            other => other,
        }
    }
}

impl CertifiedParametricAmbient {
    /// Restate the schema with `u` and `v` exchanged.
    fn with_axes_swapped(self) -> Self {
        Self {
            periods: self
                .periods
                .into_iter()
                .map(|p| CertifiedPeriod {
                    axis: p.axis.swapped(),
                    ..p
                })
                .collect(),
            domain: CertifiedDomain {
                u: self.domain.v,
                v: self.domain.u,
            },
        }
    }
}

/// A `Processor` applies a 3D transform to its entity, which does not
/// reparameterise — but **an inverted `Processor` exchanges the parameter
/// axes**: `subs(u, v)` evaluates `entity.subs(v, u)` when `orientation` is
/// false, and `ParametricSurface::u_period` forwards `entity.v_period()`
/// accordingly.
///
/// Forwarding naively would therefore certify the revolution's exact `2π` onto
/// whichever axis the entity calls angular, which for an inverted processor is
/// the *generatrix* axis of the surface the caller sees. That is precisely the
/// class of silent axis error this stage exists to make unrepresentable, so the
/// swap is applied to the periods, the domain, and any failure's axis label.
impl<S: AmbientSchemaSource, T: One> AmbientSchemaSource for Processor<S, T> {
    fn certify_ambient(
        &self,
        face_context: &FaceContext,
    ) -> Result<CertifiedParametricAmbient, SchemaFailure> {
        let inverted = !self.orientation();
        if !inverted {
            return self.entity().certify_ambient(face_context);
        }
        // The entity names axes in its own order, so face evidence must be
        // handed down in that order and the answer restated in ours.
        let inner = FaceContext {
            u_extent: face_context.v_extent,
            v_extent: face_context.u_extent,
        };
        match self.entity().certify_ambient(&inner) {
            Ok(ambient) => Ok(ambient.with_axes_swapped()),
            Err(failure) => Err(failure.with_axes_swapped()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_geometry::prelude::*;

    fn unit_cylinder() -> RevolutedCurve<Line<Point3>> {
        RevolutedCurve::by_revolution(
            Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        )
    }

    fn face(u: Option<(f64, f64)>, v: Option<(f64, f64)>) -> FaceContext {
        FaceContext {
            u_extent: u,
            v_extent: v,
        }
    }

    /// The angular period is exact and structural. Its witness must be the
    /// revolution, not a numerical agreement, and it must not depend on the
    /// generatrix.
    #[test]
    fn the_revolution_angle_is_certified_structurally() {
        let ambient = unit_cylinder()
            .certify_ambient(&face(Some((0.0, 1.0)), None))
            .expect("a revolved line has an exact angular period");
        let angular = ambient.period(ParamAxis::V).expect("v is the angle");
        assert_eq!(angular.value, 2.0 * std::f64::consts::PI);
        assert_eq!(angular.witness, PeriodWitness::ExactRevolutionAngle);
        assert_eq!(ambient.lattice().rank(), 1);
    }

    /// A non-periodic axis with no face evidence must refuse, not inherit the
    /// primitive's `[0,1]`. This is `PAR-RANGE-INHERITANCE-001` made
    /// unrepresentable: measured, a fixed window recovered 348 NIST faces and
    /// destroyed 268 others in a disjoint set of models.
    #[test]
    fn a_generatrix_axis_without_face_evidence_is_underdetermined() {
        let failure = unit_cylinder()
            .certify_ambient(&face(None, None))
            .expect_err("no face extent means no certified domain");
        assert_eq!(
            failure,
            SchemaFailure::DomainUnderdetermined { axis: ParamAxis::U }
        );
    }

    /// The face's own extent is what authorises the non-periodic axis, and the
    /// evidence must say so rather than claiming surface schema.
    #[test]
    fn face_evidence_authorises_the_generatrix_axis() {
        let ambient = unit_cylinder()
            .certify_ambient(&face(Some((2.5, 4.0)), None))
            .expect("face evidence determines the axis");
        let u = ambient.domain().u.expect("u is determined");
        assert_eq!(u.interval, (2.5, 4.0));
        assert_eq!(u.evidence, DomainEvidence::StepFaceEvidence);
        // And the angular axis is still the surface's own.
        assert_eq!(
            ambient.domain().v.unwrap().evidence,
            DomainEvidence::ExactSurfaceSchema
        );
    }

    /// An inverted `Processor` evaluates `entity.subs(v, u)`, so the axis the
    /// caller calls angular is the entity's generatrix axis and vice versa.
    /// Forwarding without the swap would certify the revolution's exact `2π`
    /// onto the wrong axis — a silent error of exactly the kind this stage
    /// exists to make unrepresentable.
    #[test]
    fn an_inverted_processor_exchanges_the_certified_axes() {
        let upright: Processor<_, Matrix4> = Processor::new(unit_cylinder());
        let ambient = upright
            .certify_ambient(&face(Some((0.0, 1.0)), None))
            .expect("upright processor forwards its entity");
        assert_eq!(
            ambient.period(ParamAxis::V).map(|p| p.witness.clone()),
            Some(PeriodWitness::ExactRevolutionAngle)
        );

        let mut inverted: Processor<_, Matrix4> = Processor::new(unit_cylinder());
        inverted.invert();
        assert!(!inverted.orientation(), "invert must flip the flag");

        // The caller's u is now the entity's angular axis, so face evidence for
        // the generatrix must be supplied on v, and the exact period must come
        // back on u.
        let ambient = inverted
            .certify_ambient(&face(None, Some((0.0, 1.0))))
            .expect("inverted processor with face evidence on the swapped axis");
        assert_eq!(
            ambient.period(ParamAxis::U).map(|p| p.witness.clone()),
            Some(PeriodWitness::ExactRevolutionAngle),
            "the exact 2pi belongs to the caller's u once inverted"
        );
        assert!(ambient.period(ParamAxis::V).is_none());
        assert_eq!(ambient.u_period(), Some(2.0 * std::f64::consts::PI));

        // And the failure axis is restated in the caller's convention too.
        assert_eq!(
            inverted.certify_ambient(&face(None, None)),
            Err(SchemaFailure::DomainUnderdetermined { axis: ParamAxis::V })
        );
    }

    /// A plane is rank 0 and both axes are the face's business.
    #[test]
    fn a_plane_has_no_lattice_and_needs_face_evidence_on_both_axes() {
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(
            plane.certify_ambient(&face(Some((0.0, 1.0)), None)),
            Err(SchemaFailure::DomainUnderdetermined { axis: ParamAxis::V })
        );
        let ambient = plane
            .certify_ambient(&face(Some((0.0, 1.0)), Some((0.0, 2.0))))
            .expect("both extents supplied");
        assert_eq!(ambient.lattice().rank(), 0);
        assert!(ambient.u_period().is_none() && ambient.v_period().is_none());
    }
}
