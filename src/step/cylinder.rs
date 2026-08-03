//! The real cylindrical-surface evidence adapter (Task 1).
//!
//! `truck-stepio` realizes STEP `cylindrical_surface` as
//! `CylindricalSurface = Processor<RevolutedCurve<Line<Point3>>, Matrix4>`
//! (see `truck_stepio::r#in::step_geometry::mod`). `identify_cylinder`
//! ([`truck_meshalgo::tessellation::formal::identify_cylinder`]) reads a
//! bare `RevolutedCurve<Line<Point3>>` structurally; this module supplies
//! the narrow adapter between the two.
//!
//! # Why this does not call `Processor::contract`
//!
//! `Processor::contract(self) -> E` requires `E: Transformed<T> +
//! Invertible`, and `RevolutedCurve<Line<Point3>>` does not implement
//! `Transformed<Matrix4>` (no blanket or derived impl exists in
//! `truck-geometry` for a revolved curve) — only its atomic pieces do. So
//! this adapter applies `Processor`'s own evaluation contract by hand, to
//! those pieces, rather than to the whole entity:
//!
//! - the revolution **origin** is a point: `transform.transform_point`;
//! - the revolution **axis** and the generatrix line's own direction are
//!   directions: `transform.transform_vector` (the linear part only, no
//!   translation);
//! - the generatrix **line's two endpoints** are points:
//!   `transform.transform_point`;
//! - `Processor::orientation() == false` is folded in exactly as
//!   `RevolutedCurve::invert()` folds it (see its module docs: "negates the
//!   revolution axis") — by negating the transformed axis, never by
//!   swapping the generatrix or re-deriving the sign from anything else.
//!
//! Reassembling `RevolutedCurve::by_revolution(transformed_line,
//! transformed_origin, transformed_axis)` from these pieces is exactly the
//! surface `Processor`'s own `ParametricSurface` impl evaluates — nothing
//! here fits a cylinder numerically or reads a mesh — and
//! `identify_cylinder` then re-verifies the structural obligations (regular
//! radius, axis-parallel generatrix, a verified `2*pi` angular period) on
//! the result, the identical checks a non-`Processor`-wrapped test fixture
//! is held to.
use truck_meshalgo::prelude::Transform;
use truck_meshalgo::tessellation::formal::{identify_cylinder, CertifiedEmbeddedCylinder, CylinderIdentification};
use truck_stepio::r#in::step_geometry::{ElementarySurface, Line, RevolutedCurve, Surface};

/// Why a `Surface` was not certified as an embedded cylinder by this
/// adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum CylinderSurfaceAdapterFailure {
    /// The representation is not `ElementarySurface::CylindricalSurface`.
    NotCylindricalSurface,
    /// The representation is a cylindrical surface, but
    /// `identify_cylinder` refused it after reading the contracted entity
    /// structurally.
    NotACylinder(truck_meshalgo::tessellation::formal::CylinderIdentificationFailure),
}

impl CylinderSurfaceAdapterFailure {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NotCylindricalSurface => "surface_not_cylindrical",
            Self::NotACylinder(cause) => match cause {
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::NonFiniteCoordinate { .. } => {
                    "cylinder_non_finite_coordinate"
                }
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::DegenerateAxis => {
                    "cylinder_degenerate_axis"
                }
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::DegenerateGeneratrix => {
                    "cylinder_degenerate_generatrix"
                }
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::ConeOrNonCylindricalRevolution => {
                    "cylinder_cone_or_non_cylindrical_revolution"
                }
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::DegenerateRadius => {
                    "cylinder_degenerate_radius"
                }
                truck_meshalgo::tessellation::formal::CylinderIdentificationFailure::UnverifiedParameterConvention => {
                    "cylinder_unverified_parameter_convention"
                }
            },
        }
    }
}

/// Read a `Surface` structurally and certify an embedded cylinder, when the
/// representation is `ElementarySurface::CylindricalSurface`.
///
/// Refuses (never fits or infers): any other surface representation
/// ([`CylinderSurfaceAdapterFailure::NotCylindricalSurface`] — cones,
/// spheres, tori, splines, swept and offset surfaces are all out of scope
/// for this adapter, exactly as [`super::lattice::support_schema_of`]
/// refuses them by name for the planar rank-0 path), and a cylindrical
/// surface whose contracted entity `identify_cylinder` itself refuses
/// ([`CylinderSurfaceAdapterFailure::NotACylinder`] — a cone smuggled in as
/// `CylindricalSurface` by a degenerate transform, a zero radius, a
/// non-finite coordinate, or an unverified `2*pi` angular period).
pub fn identify_source_cylinder(
    surface: &Surface,
) -> Result<CertifiedEmbeddedCylinder, CylinderSurfaceAdapterFailure> {
    let Surface::ElementarySurface(ElementarySurface::CylindricalSurface(processor)) = surface
    else {
        return Err(CylinderSurfaceAdapterFailure::NotCylindricalSurface);
    };
    let entity = processor.entity();
    let transform = *processor.transform();

    let origin = transform.transform_point(entity.origin());
    let mut axis = transform.transform_vector(entity.axis());
    if !processor.orientation() {
        axis = -axis;
    }
    let Line(p, q) = *entity.entity_curve();
    let generatrix = Line(transform.transform_point(p), transform.transform_point(q));

    let revolution = RevolutedCurve::by_revolution(generatrix, origin, axis);
    match identify_cylinder(&revolution) {
        CylinderIdentification::Cylinder(cylinder) => Ok(cylinder),
        CylinderIdentification::NotACylinder(cause) => {
            Err(CylinderSurfaceAdapterFailure::NotACylinder(cause))
        }
    }
}

/// [`identify_source_cylinder`], with the failure reduced to `None`.
///
/// The `Option`-returning shape [`truck_meshalgo::tessellation::LatticeMeshableShape::robust_triangulation_with_cylinder_outcome`]
/// expects: `truck-meshalgo` cannot depend on `look`'s
/// [`CylinderSurfaceAdapterFailure`], so the specific reason is available
/// only from [`identify_source_cylinder`] directly (used by this module's
/// own tests and by any future diagnostic that wants it).
pub fn identify_source_cylinder_opt(
    surface: &Surface,
) -> Option<CertifiedEmbeddedCylinder> {
    identify_source_cylinder(surface).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_meshalgo::prelude::{InnerSpace, Point3, Vector3};
    use truck_stepio::r#in::step_geometry::{Line, Processor, RevolutedCurve, ToroidalSurface, Torus};

    fn z_cylinder_surface(radius: f64, h: f64) -> Surface {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, h)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        Surface::ElementarySurface(ElementarySurface::CylindricalSurface(Processor::new(revo)))
    }

    #[test]
    fn a_real_step_cylindrical_surface_is_certified() {
        let cylinder = identify_source_cylinder(&z_cylinder_surface(2.0, 5.0))
            .expect("a cylindrical surface certifies");
        assert!((cylinder.schema().radius().get() - 2.0).abs() < 1e-9);
        assert!((cylinder.schema().axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-9);
    }

    #[test]
    fn a_transformed_cylindrical_surface_is_contracted_and_certified() {
        // A translated, rotated Processor around the same revolved line —
        // exactly the `Processor<RevolutedCurve<Line>, Matrix4>` shape a
        // placed STEP instance produces. `contract()` must fold the
        // transform in before `identify_cylinder` sees it.
        use truck_meshalgo::prelude::{Matrix4, Rad};
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let transform = Matrix4::from_translation(Vector3::new(10.0, -3.0, 4.0))
            * Matrix4::from_angle_x(Rad(0.4));
        let processor = Processor::with_transform(revo, transform);
        let surface = Surface::ElementarySurface(ElementarySurface::CylindricalSurface(processor));
        let cylinder =
            identify_source_cylinder(&surface).expect("a transformed cylinder certifies");
        assert!((cylinder.schema().radius().get() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_cone_smuggled_as_cylindrical_surface_is_refused() {
        // Tilted generatrix meeting the axis: a cone, not a cylinder, even
        // though it arrives typed as `CylindricalSurface`.
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(0.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let surface =
            Surface::ElementarySurface(ElementarySurface::CylindricalSurface(Processor::new(revo)));
        let result = identify_source_cylinder(&surface);
        assert!(matches!(
            result.unwrap_err(),
            CylinderSurfaceAdapterFailure::NotACylinder(_)
        ));
    }

    #[test]
    fn a_non_cylindrical_surface_representation_is_refused_by_name() {
        let torus = Surface::ElementarySurface(ElementarySurface::ToroidalSurface(
            ToroidalSurface::new(Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0)),
        ));
        assert_eq!(
            identify_source_cylinder(&torus).unwrap_err(),
            CylinderSurfaceAdapterFailure::NotCylindricalSurface
        );
    }
}
