//! The real conical-surface evidence adapter.
//!
//! `truck-stepio` realizes STEP `conical_surface` as
//! `ConicalSurface = Processor<RevolutedCurve<Line<Point3>>, Matrix4>` — the
//! *identical* representation it uses for `cylindrical_surface` (see
//! `truck_stepio::r#in::step_geometry::mod`). `identify_cone`
//! ([`truck_meshalgo::tessellation::formal::identify_cone`]) reads a bare
//! `RevolutedCurve<Line<Point3>>` structurally; this module supplies the narrow
//! adapter between the two.
//!
//! # Reassembly, and the condition that makes it exact
//!
//! `Processor::contract(self) -> E` requires `E: Transformed<T> + Invertible`,
//! and `RevolutedCurve<Line<Point3>>` implements neither, so this adapter
//! applies `Processor`'s own evaluation contract by hand to the entity's atomic
//! pieces:
//!
//! - the revolution **origin** is a point: `transform.transform_point`;
//! - the revolution **axis** is a direction: `transform.transform_vector`;
//! - the generatrix **line's two endpoints** are points:
//!   `transform.transform_point`;
//! - `Processor::orientation() == false` is folded in exactly as
//!   `RevolutedCurve::invert()` folds it — by negating the transformed axis.
//!
//! Reassembling a revolution from those pieces is **not** unconditionally the
//! same surface the `Processor` evaluates, and the condition is worth writing
//! down because it is the seam this cell's soundness rests on. With `A` the
//! transform's linear part, the `Processor` evaluates
//! `M·(O + R_axis(v)·(L(u) − O))` and the reassembly evaluates
//! `M·O + R_{A·axis}(v)·(A·(L(u) − O))`. The two agree for every `(u, v)`
//! exactly when `A·R_axis(v) = R_{A·axis}(v)·A`, and that identity holds for
//! `A = s·Q` with `Q` orthogonal — a **similarity** — and fails otherwise.
//!
//! Under a non-uniform scale or a shear the image of a circular cone is an
//! *elliptic* cone: still a ruled surface through an apex, but one whose
//! parallels are ellipses and whose half-angle is not constant. Reassembling
//! would then produce a circular cone that `identify_cone` would happily
//! certify, and every downstream obligation — the radius the half-angle
//! predicts at a level, the carrier enclosures, the nappe verdict — would be
//! checked against a surface the file does not describe. So this adapter
//! refuses a non-similarity placement by name
//! ([`ConeSurfaceAdapterFailure::PlacementNotASimilarity`]) rather than
//! certifying through it.
//!
//! No conforming file reaches that refusal: `Matrix4::from(&Axis2Placement3d)`
//! builds an orthonormal frame plus a translation, which is a rigid motion and
//! therefore a similarity with `s = 1`. The gate exists because the `Processor`
//! carries an unconstrained `Matrix4` and `transform_by` composes freely, so
//! nothing in the *type* prevents the case.
//!
//! A similarity may include a reflection, and one is admitted. A reflection
//! carries circles to circles and cones to cones; what it reverses is the sense
//! of the revolution parameter `v`, and nothing in the band path reads `v`. The
//! developed chart is built from the certified apex, axis and radial frame in
//! world space, and
//! [`truck_meshalgo::tessellation::formal::ConeSchema::point_at`] inverts
//! [`truck_meshalgo::tessellation::formal::ConeSchema::angular_coordinate`]
//! exactly, whichever handedness the placement had.
//!
//! That last point matters more here than it does for a cylinder.
//! `truck-stepio`'s `conical_surface` conversion calls `processor.invert()`
//! unconditionally, so **every** conical surface arriving from a real file has
//! `orientation() == false` and reaches `identify_cone` with a negated axis.
//! Negating the axis negates the generator coordinate and therefore swaps the
//! two nappe labels — which is exactly what it should do, and is why
//! [`truck_meshalgo::tessellation::formal::Nappe`] is a label relative to a
//! certified axis rather than an absolute claim about the part.
//!
//! Nothing here fits a cone numerically or reads a mesh. `identify_cone` then
//! re-verifies every structural obligation on the reassembled revolution — a
//! tilt strictly between parallel and perpendicular, a generatrix that meets
//! the axis, an apex confirmed on it, a half-angle that reproduces the
//! generatrix, and a verified `2π` angular period — so a transform that
//! destroyed the cone (a non-uniform scale, a shear) is refused rather than
//! certified from the untransformed numbers.

use truck_meshalgo::prelude::{InnerSpace, Matrix4, Transform, Vector3};
use truck_meshalgo::tessellation::formal::{
    identify_cone, CertifiedEmbeddedCone, ConeIdentification, ConeIdentificationFailure,
};
use truck_stepio::r#in::step_geometry::{ElementarySurface, Line, RevolutedCurve, Surface};

/// Relative bound for confirming that a placement's linear part is a
/// similarity.
///
/// The same `1e-9` the formal subtree confirms its other structural algebra at.
/// Its job here is confirmation, not classification: a STEP placement is built
/// from an orthonormal frame, so on any conforming file the residuals this
/// bounds are floating-point drift in the importer's own normalization. A
/// genuinely non-uniform or shearing transform misses it by orders of
/// magnitude.
const SIMILARITY_RESIDUAL: f64 = 1e-9;

/// Why a `Surface` was not certified as an embedded cone by this adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum ConeSurfaceAdapterFailure {
    /// The representation is not `ElementarySurface::ConicalSurface`.
    NotConicalSurface,
    /// The placement's linear part is not a similarity, so the revolution
    /// cannot be reassembled from its transformed pieces: the surface the
    /// `Processor` evaluates is an elliptic cone, and no circular cone
    /// certificate describes it. See the module docs.
    PlacementNotASimilarity,
    /// The representation is a conical surface, but `identify_cone` refused it
    /// after reading the contracted entity structurally.
    NotACone(ConeIdentificationFailure),
}

impl ConeSurfaceAdapterFailure {
    /// A short stable tag, for diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NotConicalSurface => "surface_not_conical",
            Self::PlacementNotASimilarity => "cone_placement_not_a_similarity",
            Self::NotACone(cause) => cause.tag(),
        }
    }
}

/// Whether a placement's linear part is a similarity: a uniform scale composed
/// with an orthogonal map.
///
/// Checked as the property itself rather than through a decomposition — the
/// three columns must be mutually orthogonal and of equal length — so a
/// reflection passes (it is orthogonal) and a non-uniform scale or a shear does
/// not. Both residuals are taken relative to the mean squared column length, so
/// the verdict does not depend on the units the file is written in.
///
/// A singular linear part fails: with a zero column the equal-length test
/// cannot hold against the others unless every column is zero, and a zero
/// transform has no scale to be uniform.
fn is_similarity(transform: &Matrix4) -> bool {
    let column = |c: [f64; 4]| Vector3::new(c[0], c[1], c[2]);
    let columns = [
        column([transform.x.x, transform.x.y, transform.x.z, 0.0]),
        column([transform.y.x, transform.y.y, transform.y.z, 0.0]),
        column([transform.z.x, transform.z.y, transform.z.z, 0.0]),
    ];
    let squared: Vec<f64> = columns.iter().map(|c| c.dot(*c)).collect();
    let scale = (squared[0] + squared[1] + squared[2]) / 3.0;
    if !(scale > 0.0) || !scale.is_finite() {
        return false;
    }
    // Equal column lengths: a uniform scale.
    for value in &squared {
        if !((value - scale).abs() <= SIMILARITY_RESIDUAL * scale) {
            return false;
        }
    }
    // Mutually orthogonal columns: no shear.
    for (i, j) in [(0usize, 1usize), (0, 2), (1, 2)] {
        if !(columns[i].dot(columns[j]).abs() <= SIMILARITY_RESIDUAL * scale) {
            return false;
        }
    }
    true
}

/// Read a `Surface` structurally and certify an embedded cone, when the
/// representation is `ElementarySurface::ConicalSurface`.
///
/// Refuses (never fits or infers): any other surface representation
/// ([`ConeSurfaceAdapterFailure::NotConicalSurface`] — cylinders, planes,
/// spheres, tori, splines, swept and offset surfaces are all out of scope), and
/// a conical surface whose contracted entity `identify_cone` itself refuses
/// ([`ConeSurfaceAdapterFailure::NotACone`] — a cylinder or a hyperboloid
/// smuggled in by a degenerate transform, a semi-angle of zero or `π/2`, a
/// non-finite coordinate, or an unverified `2π` angular period).
pub fn identify_source_cone(
    surface: &Surface,
) -> Result<CertifiedEmbeddedCone, ConeSurfaceAdapterFailure> {
    let Surface::ElementarySurface(ElementarySurface::ConicalSurface(processor)) = surface else {
        return Err(ConeSurfaceAdapterFailure::NotConicalSurface);
    };
    let entity = processor.entity();
    let transform = *processor.transform();
    // Before anything is transformed: only a similarity makes the reassembly
    // below the surface this `Processor` actually evaluates. See the module
    // docs — under a non-uniform scale or a shear the true surface is an
    // elliptic cone, and a circular-cone certificate would describe something
    // the file does not contain.
    if !is_similarity(&transform) {
        return Err(ConeSurfaceAdapterFailure::PlacementNotASimilarity);
    }

    let origin = transform.transform_point(entity.origin());
    let mut axis = transform.transform_vector(entity.axis());
    if !processor.orientation() {
        axis = -axis;
    }
    let Line(p, q) = *entity.entity_curve();
    let generatrix = Line(transform.transform_point(p), transform.transform_point(q));

    let revolution = RevolutedCurve::by_revolution(generatrix, origin, axis);
    match identify_cone(&revolution) {
        ConeIdentification::Cone(cone) => Ok(cone),
        ConeIdentification::NotACone(cause) => Err(ConeSurfaceAdapterFailure::NotACone(cause)),
    }
}

/// [`identify_source_cone`], with the failure reduced to its stable tag.
///
/// The `Result<_, &'static str>` shape
/// [`truck_meshalgo::tessellation::LatticeMeshableShape::robust_triangulation_with_cone_outcome`]
/// expects: `truck-meshalgo` cannot depend on `look`'s
/// [`ConeSurfaceAdapterFailure`] enum directly, but the *tag* survives the crate
/// boundary, so diagnostics built from it still distinguish "not a
/// `ConicalSurface` representation" from "a `ConicalSurface` representation
/// `identify_cone` itself refused", and among the latter which obligation
/// failed.
pub fn identify_source_cone_opt(surface: &Surface) -> Result<CertifiedEmbeddedCone, &'static str> {
    identify_source_cone(surface).map_err(|failure| failure.tag())
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_meshalgo::prelude::{Invertible, InnerSpace, Point3, Vector3};
    use truck_meshalgo::tessellation::formal::Nappe;
    use truck_stepio::r#in::step_geometry::{Processor, ToroidalSurface, Torus};

    /// A cone about the z-axis with its apex at the origin, as a `Processor`
    /// with the identity transform and forward orientation.
    fn z_cone_surface(slope: f64) -> Surface {
        let revo = RevolutedCurve::by_revolution(
            Line(
                Point3::new(slope * 1.0, 0.0, 1.0),
                Point3::new(slope * 6.0, 0.0, 6.0),
            ),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        Surface::ElementarySurface(ElementarySurface::ConicalSurface(Processor::new(revo)))
    }

    #[test]
    fn a_real_step_conical_surface_is_certified() {
        let cone = identify_source_cone(&z_cone_surface(0.5)).expect("a conical surface certifies");
        assert!((cone.schema().slope().get() - 0.5).abs() < 1e-9);
        assert!((cone.schema().apex() - Point3::new(0.0, 0.0, 0.0)).magnitude() < 1e-9);
        assert!((cone.schema().axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-9);
    }

    /// The `Processor` transform must be folded in before `identify_cone` sees
    /// the entity — the apex moves with the placement, and a certificate built
    /// from the untransformed numbers would put it in the wrong place.
    #[test]
    fn a_transformed_conical_surface_is_contracted_and_certified() {
        use truck_meshalgo::prelude::{Matrix4, Rad};
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let translation = Vector3::new(10.0, -3.0, 4.0);
        let transform = Matrix4::from_translation(translation) * Matrix4::from_angle_x(Rad(0.4));
        let processor = Processor::with_transform(revo, transform);
        let surface = Surface::ElementarySurface(ElementarySurface::ConicalSurface(processor));
        let cone = identify_source_cone(&surface).expect("a transformed cone certifies");
        assert!((cone.schema().slope().get() - 0.5).abs() < 1e-9);
        // The apex was at the origin before placement, so it is at the
        // translation after it.
        assert!(
            (cone.schema().apex() - Point3::new(0.0, 0.0, 0.0) - translation).magnitude() < 1e-8
        );
    }

    /// The inversion every real conical surface arrives with. The cone is the
    /// same cone and the apex is unmoved; the axis is negated, so the nappe
    /// labels swap. Both facts are asserted, because the second is the one that
    /// would silently corrupt a same-nappe verdict if the fold were applied
    /// twice or not at all.
    #[test]
    fn the_importers_inversion_negates_the_axis_and_swaps_the_nappe_labels() {
        let forward = identify_source_cone(&z_cone_surface(0.5)).expect("forward");
        let mut processor = match z_cone_surface(0.5) {
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(processor)) => processor,
            _ => unreachable!("the fixture is a conical surface"),
        };
        processor.invert();
        let inverted = identify_source_cone(&Surface::ElementarySurface(
            ElementarySurface::ConicalSurface(processor),
        ))
        .expect("an inverted cone is still a cone");

        assert!((forward.schema().apex() - inverted.schema().apex()).magnitude() < 1e-9);
        assert!((forward.schema().slope().get() - inverted.schema().slope().get()).abs() < 1e-9);
        assert!((forward.schema().axis() + inverted.schema().axis()).magnitude() < 1e-9);

        let sample = forward.schema().point_at(3.0, 0.6);
        assert_eq!(
            forward
                .schema()
                .nappe_of(forward.schema().generator_coordinate(sample)),
            Some(Nappe::Positive)
        );
        assert_eq!(
            inverted
                .schema()
                .nappe_of(inverted.schema().generator_coordinate(sample)),
            Some(Nappe::Negative)
        );
    }

    /// The seam this adapter exists to close. A non-uniform scale carries a
    /// circular cone to an *elliptic* one, so the reassembled revolution is not
    /// the surface the `Processor` evaluates — and it is a perfectly
    /// well-formed circular cone, which `identify_cone` would certify without
    /// complaint. It must be refused here, before that happens.
    #[test]
    fn a_non_uniform_scale_is_refused_before_a_circular_cone_is_certified() {
        use truck_meshalgo::prelude::{Matrix4, ParametricSurface};
        use truck_meshalgo::tessellation::formal::{identify_cone, ConeIdentification};
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        // Squashing in y leaves the generatrix (which lies in the xz-plane),
        // the origin and the axis all fixed, so the reassembly this adapter
        // would perform is *bit-identical* to the untransformed revolution --
        // and `identify_cone` certifies it without complaint.
        let squash = Matrix4::from_nonuniform_scale(1.0, 0.5, 1.0);
        let reassembled = identify_cone(&revo);
        assert!(
            matches!(reassembled, ConeIdentification::Cone(_)),
            "the premise of this test is that the reassembly looks like a clean cone"
        );
        let ConeIdentification::Cone(reassembled) = reassembled else {
            unreachable!()
        };

        // But the surface the `Processor` actually evaluates is an elliptic
        // cone, and its points are nowhere near the circular cone above. This
        // is the seam: without the similarity gate the adapter would hand
        // downstream a certificate describing a surface the file does not
        // contain, and every carrier radius, nappe verdict and apex clearance
        // would be checked against the wrong surface.
        let processor = Processor::with_transform(revo, squash);
        let off_axis = processor.subs(0.5, std::f64::consts::FRAC_PI_2);
        assert!(
            reassembled.schema().radial_gap(off_axis) > 0.1,
            "the squashed surface must genuinely leave the circular cone"
        );

        let surface =
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(processor));
        assert_eq!(
            identify_source_cone(&surface).unwrap_err(),
            ConeSurfaceAdapterFailure::PlacementNotASimilarity
        );
    }

    /// A shear is the other way to leave the similarity group, and it is
    /// refused on the orthogonality residual rather than the length one.
    #[test]
    fn a_shearing_placement_is_refused() {
        use truck_meshalgo::prelude::Matrix4;
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let mut shear = Matrix4::from_scale(1.0);
        shear.y.x = 0.3;
        let surface = Surface::ElementarySurface(ElementarySurface::ConicalSurface(
            Processor::with_transform(revo, shear),
        ));
        assert_eq!(
            identify_source_cone(&surface).unwrap_err(),
            ConeSurfaceAdapterFailure::PlacementNotASimilarity
        );
    }

    /// A uniform scale *is* a similarity and is admitted — the gate must not
    /// have become "rigid motions only", which would refuse a legitimately
    /// scaled placement. The apex and the half-angle follow the scale
    /// correctly: the apex is a point and moves, the half-angle is an angle and
    /// does not.
    #[test]
    fn a_uniform_scale_is_admitted_and_scales_the_apex_but_not_the_half_angle() {
        use truck_meshalgo::prelude::Matrix4;
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 2.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        // The apex of this cone sits at the local origin; scale by 4 about the
        // world origin and translate, and it must land at the translation.
        let translation = Vector3::new(1.0, 2.0, -5.0);
        let transform = Matrix4::from_translation(translation) * Matrix4::from_scale(4.0);
        let surface = Surface::ElementarySurface(ElementarySurface::ConicalSurface(
            Processor::with_transform(revo, transform),
        ));
        let cone = identify_source_cone(&surface).expect("a uniform scale is a similarity");
        assert!((cone.schema().slope().get() - 0.5).abs() < 1e-9);
        assert!(
            (cone.schema().apex() - Point3::new(0.0, 0.0, 0.0) - translation).magnitude() < 1e-8
        );
    }

    /// A reflection is orthogonal, so it is a similarity and is admitted. What
    /// it reverses is the sense of the revolution parameter, which nothing in
    /// the band path reads: the chart is built in world space from the apex,
    /// the axis and the radial frame, and it still round-trips.
    #[test]
    fn a_reflected_placement_is_admitted_and_its_chart_still_round_trips() {
        use truck_meshalgo::prelude::Matrix4;
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let reflect = Matrix4::from_nonuniform_scale(1.0, 1.0, -1.0);
        let surface = Surface::ElementarySurface(ElementarySurface::ConicalSurface(
            Processor::with_transform(revo, reflect),
        ));
        let cone = identify_source_cone(&surface).expect("a reflection is a similarity");
        let schema = cone.schema();
        assert!((schema.slope().get() - 0.5).abs() < 1e-9);
        for s in [1.5_f64, 4.0] {
            for theta in [0.0_f64, 1.1, -2.2] {
                let point = schema.point_at(s, theta);
                assert!(schema.radial_gap(point) < 1e-9);
                assert!((schema.generator_coordinate(point) - s).abs() < 1e-9);
            }
        }
    }

    /// A cylinder typed as a conical surface — a zero semi-angle, or a
    /// degenerate transform — is refused rather than certified with a
    /// zero half-angle and an apex at infinity.
    #[test]
    fn a_cylinder_smuggled_as_conical_surface_is_refused() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let surface =
            Surface::ElementarySurface(ElementarySurface::ConicalSurface(Processor::new(revo)));
        assert_eq!(
            identify_source_cone(&surface).unwrap_err(),
            ConeSurfaceAdapterFailure::NotACone(ConeIdentificationFailure::CylindricalRevolution)
        );
    }

    #[test]
    fn a_non_conical_surface_representation_is_refused_by_name() {
        let torus = Surface::ElementarySurface(ElementarySurface::ToroidalSurface(
            ToroidalSurface::new(Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0)),
        ));
        assert_eq!(
            identify_source_cone(&torus).unwrap_err(),
            ConeSurfaceAdapterFailure::NotConicalSurface
        );
    }

    /// A *cylindrical* surface is refused by representation, not by geometry.
    /// The two adapters partition the same `Processor<RevolutedCurve<Line>>`
    /// shape between them, and neither will certify the other's surface.
    #[test]
    fn a_cylindrical_surface_representation_is_refused_by_name() {
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 5.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        let surface = Surface::ElementarySurface(ElementarySurface::CylindricalSurface(
            Processor::new(revo),
        ));
        assert_eq!(
            identify_source_cone(&surface).unwrap_err(),
            ConeSurfaceAdapterFailure::NotConicalSurface
        );
    }
}
