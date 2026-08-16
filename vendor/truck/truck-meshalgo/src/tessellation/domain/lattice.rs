//! The certified deck lattice — the production type boundary for periodicity.
//!
//! `REFINEMENT_AUDIT.md` §6 found seven live reads of `u_period()` / `v_period()`
//! in the executed path, and three different qualities of evidence arriving at
//! them as indistinguishable `Option<f64>`:
//!
//! - **exact** — a `RevolutedCurve`'s angular axis. `subs(u,v)` applies
//!   `rotation_matrix(v)`, so `2π` is a property of the map, for every
//!   generatrix. This is why cylinder, cone, sphere and torus are one case.
//! - **accessor-only** — a `RevolutedCurve`'s generatrix axis, which is just
//!   `curve.period()` forwarded. Nothing establishes it.
//! - **absent** — planes and splines, which declare no period.
//!
//! This type keeps them apart. It carries *only* lattice information: §6.3
//! established that the parameter domain is an **output** of lifting, not an
//! input, so there is deliberately no domain, no extent and no `FaceContext`
//! here. `domain/ambient.rs` is the frozen prototype that got that wrong; it is
//! not used by production and is retained only as a record.
//!
//! # An uncertified period is not a generator
//!
//! [`CertifiedLattice::generator`] returns a value only for an axis with
//! representation-derived evidence. The declared value of an uncertified axis
//! is still reachable through [`CertifiedLattice::declared_period`], which is
//! what the executed path uses today, so introducing this boundary changes no
//! behaviour. Moving a call site from `declared_period` to `generator` is a
//! *semantic* change and must be measured on its own.

/// Why an axis's period is known.
///
/// There is no numerically-verified variant, and there must not be: sampled
/// agreement at finitely many points establishes nothing about the surface
/// between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodWitness {
    /// The parameterisation *is* a rotation. `RevolutedCurve::subs(u,v)` is
    /// `origin + rotation_matrix(v) · (curve(u) − origin)`, so this axis has
    /// period `2π` by construction of the map, independently of the generatrix.
    ExactRevolutionAngle,
    /// The parameterisation is a sphere's azimuth. `Sphere::subs(u, v)` enters
    /// `v` only through `(cos v, sin v)` — a rotation about the polar axis — so
    /// the azimuth has period `2π` by construction of the primitive, for every
    /// latitude. Distinct from [`ExactRevolutionAngle`]: a sphere is a
    /// `Processor<Sphere, Matrix4>`, not a revolved curve, and the witness
    /// names the primitive it was read from.
    ExactSphereAzimuth,
    /// The parameter axis is declared closed by the *source representation*,
    /// and the converted evaluator satisfies the seam identification required
    /// for periodic continuation.
    ///
    /// This is the witness for a source-declared-closed B-spline or NURBS
    /// surface, whose generic evaluator is bounded to its native knot domain
    /// rather than globally periodic. The theorem is: the STEP source declares
    /// the axis closed (`.T.`), and the converted spline satisfies the
    /// representation-level seam conditions `S(u, a) == S(u, b)` and
    /// `Sv(u, a) == Sv(u, b)` over the active interval `[a, b]`; therefore the
    /// deck translation `P = b - a` is a valid topological lattice generator.
    ///
    /// The source declaration is the authority; the seam evaluation is a
    /// *compatibility check that may reject an incompatible conversion*, never
    /// the source of the closure. This witness is deliberately distinct from
    /// [`ExactRevolutionAngle`] — a spline is not a rotation, and the period is
    /// the native span `b - a`, not `2π`.
    SourceDeclaredClosedSplineAxis,
}

/// Why a polar orbit is known to collapse to a single point.
///
/// A *candidate* recognizer (a small numerical orbit diameter sampled at a few
/// angular positions) is deliberately not a witness. The generic `find_cap_pole`
/// scan stays a heuristic that may nominate a pole location; only this enum
/// carries a representation-derived certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseWitness {
    /// A sphere's polar latitude. `Sphere::subs` enters the azimuth only through
    /// `(cos v, sin v)` (times the latitude's sine), so at the polar latitudes
    /// the physical map is independent of the azimuth — the orbit collapses to a
    /// single point for every azimuth. Read from the primitive's own
    /// parameterisation, not from a numerical sample. Preserved under any
    /// affine map, so it survives the `Processor`'s transform and inversion.
    ExactSpherePole,
}

/// A certified orbit collapse on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedCollapse {
    /// The collapsing (polar) axis.
    pub polar: Axis,
    /// Why the collapse is certified.
    pub witness: CollapseWitness,
}

/// What is known about one parameter axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AxisPeriodStatus {
    /// Periodic, with representation-derived evidence.
    Exact {
        /// The period.
        period: f64,
        /// Why it is known.
        witness: PeriodWitness,
    },
    /// The surface declares no period on this axis.
    NonPeriodic,
    /// A period is declared but rests on an accessor result rather than on the
    /// representation. Retained for the legacy path and for a future explicit
    /// policy; **never** offered as a deck generator.
    Uncertified {
        /// The value the surface reported.
        declared: f64,
    },
}

impl AxisPeriodStatus {
    /// The declared period, certified or not. What the executed path reads
    /// today.
    pub fn declared_period(self) -> Option<f64> {
        match self {
            Self::Exact { period, .. } => Some(period),
            Self::Uncertified { declared } => Some(declared),
            Self::NonPeriodic => None,
        }
    }

    /// The period *as a deck generator*. `None` unless representation-derived.
    pub fn generator(self) -> Option<f64> {
        match self {
            Self::Exact { period, .. } => Some(period),
            Self::Uncertified { .. } | Self::NonPeriodic => None,
        }
    }

    /// Build from a raw accessor result, with no evidence. Every axis reached
    /// this way is uncertified by definition — that is the point of the name.
    pub fn from_unevidenced_accessor(declared: Option<f64>) -> Self {
        match declared {
            Some(declared) if declared.is_finite() && declared > 0.0 => {
                Self::Uncertified { declared }
            }
            _ => Self::NonPeriodic,
        }
    }

    /// Exchange nothing; used by [`CertifiedLattice::swapped`].
    fn identity(self) -> Self {
        self
    }
}

/// The deck lattice of one face's supporting surface, in the **caller's** axis
/// convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedLattice {
    /// The `u` axis.
    pub u: AxisPeriodStatus,
    /// The `v` axis.
    pub v: AxisPeriodStatus,
}

impl CertifiedLattice {
    /// Neither axis is periodic. Planes and non-periodic splines.
    pub const NON_PERIODIC: Self = Self {
        u: AxisPeriodStatus::NonPeriodic,
        v: AxisPeriodStatus::NonPeriodic,
    };

    /// The exact angular period of a surface of revolution, on the axis the
    /// revolution parameterises, with the generatrix axis left to the caller.
    pub fn revolution(angular: Axis, generatrix: AxisPeriodStatus) -> Self {
        let exact = AxisPeriodStatus::Exact {
            period: std::f64::consts::PI * 2.0,
            witness: PeriodWitness::ExactRevolutionAngle,
        };
        match angular {
            Axis::U => Self {
                u: exact,
                v: generatrix,
            },
            Axis::V => Self {
                u: generatrix,
                v: exact,
            },
        }
    }

    /// The exact azimuth period of a sphere, on the axis the sphere
    /// parameterises as longitude, with the polar axis left to the caller.
    ///
    /// `Sphere::subs(u, v)` enters the azimuth only through `(cos v, sin v)`,
    /// so `2π` is a property of the primitive, not of an accessor. The polar
    /// axis (latitude) has no period.
    pub fn sphere_azimuth(azimuth: Axis) -> Self {
        let exact = AxisPeriodStatus::Exact {
            period: std::f64::consts::PI * 2.0,
            witness: PeriodWitness::ExactSphereAzimuth,
        };
        match azimuth {
            Axis::U => Self {
                u: exact,
                v: AxisPeriodStatus::NonPeriodic,
            },
            Axis::V => Self {
                u: AxisPeriodStatus::NonPeriodic,
                v: exact,
            },
        }
    }

    /// Build from raw accessors with no evidence at all. Every axis is
    /// `Uncertified`. This is the legacy path: it preserves today's behaviour
    /// exactly while making the absence of evidence visible in the type.
    pub fn from_unevidenced_accessors(u: Option<f64>, v: Option<f64>) -> Self {
        Self {
            u: AxisPeriodStatus::from_unevidenced_accessor(u),
            v: AxisPeriodStatus::from_unevidenced_accessor(v),
        }
    }

    /// Restate with `u` and `v` exchanged.
    ///
    /// An inverted `Processor` evaluates `entity.subs(v, u)`, so a fact the
    /// entity states about its own `u` is a fact about the caller's `v`.
    /// Forwarding without this would place the exact `2π` on the wrong axis —
    /// a silent error the scattered-accessor design could not even express.
    pub fn swapped(self) -> Self {
        Self {
            u: self.v.identity(),
            v: self.u.identity(),
        }
    }

    /// The declared `u` period, certified or not.
    pub fn declared_u_period(&self) -> Option<f64> {
        self.u.declared_period()
    }

    /// The declared `v` period, certified or not.
    pub fn declared_v_period(&self) -> Option<f64> {
        self.v.declared_period()
    }

    /// The `u` deck generator, if representation-derived.
    pub fn u_generator(&self) -> Option<f64> {
        self.u.generator()
    }

    /// The `v` deck generator, if representation-derived.
    pub fn v_generator(&self) -> Option<f64> {
        self.v.generator()
    }

    /// The rank of the certified lattice — generators only, so an uncertified
    /// axis does not inflate it.
    pub fn certified_rank(&self) -> usize {
        usize::from(self.u_generator().is_some()) + usize::from(self.v_generator().is_some())
    }

    /// The certified polar orbit collapse, if the representation establishes
    /// one.
    ///
    /// A sphere's azimuth witness names the primitive it was read from, and that
    /// primitive collapses at its polar latitudes by construction — the azimuth
    /// enters `Sphere::subs` only through `(cos v, sin v)`, so at the poles the
    /// physical map is a single point. The polar axis is therefore the axis
    /// that does **not** carry the certified azimuth, in the caller's own
    /// convention (`swapped()` restates it with the axes exchanged).
    ///
    /// Everything else — a revolution's angular axis, an accessor value, a
    /// numerically-shrunk orbit — returns `None`: the presence of a pole is
    /// certified here, never inferred from how small an orbit got.
    pub fn certified_collapse(&self) -> Option<CertifiedCollapse> {
        match (&self.u, &self.v) {
            (
                AxisPeriodStatus::Exact {
                    witness: PeriodWitness::ExactSphereAzimuth,
                    ..
                },
                _,
            ) => Some(CertifiedCollapse {
                polar: Axis::V,
                witness: CollapseWitness::ExactSpherePole,
            }),
            (
                _,
                AxisPeriodStatus::Exact {
                    witness: PeriodWitness::ExactSphereAzimuth,
                    ..
                },
            ) => Some(CertifiedCollapse {
                polar: Axis::U,
                witness: CollapseWitness::ExactSpherePole,
            }),
            _ => None,
        }
    }
}

/// Which parameter axis a fact concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// The `u` axis.
    U,
    /// The `v` axis.
    V,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distinction the whole patch exists for: an accessor-derived period
    /// is usable by the legacy path but is not a deck generator.
    #[test]
    fn an_uncertified_period_is_never_a_generator() {
        let lattice = CertifiedLattice::from_unevidenced_accessors(Some(1.5), None);
        assert_eq!(lattice.declared_u_period(), Some(1.5));
        assert_eq!(lattice.u_generator(), None);
        assert_eq!(lattice.certified_rank(), 0);
    }

    /// A revolution's angular axis is exact; its generatrix axis is whatever
    /// the caller could establish, which for an accessor result is nothing.
    #[test]
    fn a_revolution_certifies_only_its_angular_axis() {
        let lattice = CertifiedLattice::revolution(
            Axis::V,
            AxisPeriodStatus::from_unevidenced_accessor(Some(4.0)),
        );
        assert_eq!(lattice.v_generator(), Some(std::f64::consts::PI * 2.0));
        assert_eq!(
            lattice.u_generator(),
            None,
            "generatrix rests on an accessor"
        );
        assert_eq!(lattice.declared_u_period(), Some(4.0), "still usable");
        assert_eq!(lattice.certified_rank(), 1);
    }

    /// Inversion moves every axis-indexed fact to the other axis.
    #[test]
    fn swapping_moves_the_exact_period_to_the_other_axis() {
        let lattice =
            CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic).swapped();
        assert_eq!(lattice.u_generator(), Some(std::f64::consts::PI * 2.0));
        assert_eq!(lattice.v_generator(), None);
    }

    /// A sphere certifies a pole collapse on the *polar* axis — the axis that
    /// does not carry the certified azimuth. The collapse is read from the
    /// primitive's own parameterisation, never inferred from a small orbit.
    #[test]
    fn a_sphere_certifies_its_polar_collapse() {
        let lattice = CertifiedLattice::sphere_azimuth(Axis::U);
        let collapse = lattice
            .certified_collapse()
            .expect("a sphere certifies a pole");
        assert_eq!(collapse.polar, Axis::V);
        assert_eq!(collapse.witness, CollapseWitness::ExactSpherePole);
    }

    /// An inverted sphere restates the polar collapse on the other axis, in the
    /// caller's convention, exactly as it restates the azimuth period.
    #[test]
    fn an_inverted_sphere_puts_the_certified_pole_on_the_other_axis() {
        let lattice = CertifiedLattice::sphere_azimuth(Axis::U).swapped();
        let collapse = lattice
            .certified_collapse()
            .expect("a sphere certifies a pole");
        assert_eq!(collapse.polar, Axis::U);
        assert_eq!(collapse.witness, CollapseWitness::ExactSpherePole);
    }

    /// A revolution's angular axis is a rotation, but that does not make the
    /// generatrix axis a collapsed pole: a cylinder has no pole and a cone's
    /// apex is not certified by the revolution witness alone. Numerically
    /// shrinking an orbit is a candidate recognizer, never this certificate.
    #[test]
    fn a_revolution_does_not_certify_a_polar_collapse() {
        for lattice in [
            CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic),
            CertifiedLattice::revolution(Axis::V, AxisPeriodStatus::NonPeriodic).swapped(),
            CertifiedLattice::NON_PERIODIC,
            CertifiedLattice::from_unevidenced_accessors(Some(6.28), None),
        ] {
            assert_eq!(
                lattice.certified_collapse(),
                None,
                "an accessor or revolution value never certifies a pole",
            );
        }
    }
}
