//! The canonical curve and surface model (BG-CE-006).
//!
//! `Curve` and `Surface` used to live in `truck-modeling`'s `geometry` module,
//! where `Surface` silently dropped the analytic carriers (`Cylinder`, `Cone`,
//! `Sphere`, `Torus`) from `specifieds` and degraded every analytic operation
//! to splines. They now live here, owned by `truck-geometry`, with the analytic
//! carriers as first-class variants; `truck-modeling`'s module is a re-export.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::constructive::{CertifiedImplicitIntersectionCurve, SpineFrameSweep};
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal, UnresolvedWitness,
};

/// 3-dimensional curve
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
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
    /// analytic circle: a placed (possibly full-range) trimmed unit circle
    Circle(Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>),
    /// 3-dimensional B-spline curve
    BSplineCurve(BSplineCurve<Point3>),
    /// 3-dimensional NURBS curve
    NurbsCurve(NurbsCurve<Vector4>),
    /// intersection curve
    IntersectionCurve(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>),
    /// The trajectory of one fixed profile point under a spine-frame recipe
    /// (BG-CG-009-BREP): `E(s) = X(s, p)`. Shared by adjacent side faces;
    /// never re-derived. The spine is boxed: the decorator stores the closed
    /// `Curve` spine, which would recurse without indirection (the
    /// `IntersectionCurve` precedent).
    SpineFrameCurve(SpineFrameCurve<Box<Curve>>),
    /// The certified implicit intersection curve carrier (BIE-003-CARRIER): a
    /// certified 3-D polyline with per-sample tangent frames and the
    /// unresolved witness slot. Procedural (continuously evaluable through its
    /// stored frames), polyline form consumed only at tessellation. Mirrors
    /// the `IntersectionCurve` decorator precedent (variant + `From` +
    /// method-delegation macro arms); the payload stores no `Curve`, so no
    /// boxing indirection is needed.
    CertifiedImplicitIntersectionCurve(CertifiedImplicitIntersectionCurve),
}

macro_rules! derive_curve_method {
    ($curve: expr, $method: expr, $($ver: ident),*) => {
        match $curve {
            Curve::Line(got) => $method(got, $($ver), *),
            Curve::Circle(got) => $method(got, $($ver), *),
            Curve::BSplineCurve(got) => $method(got, $($ver), *),
            Curve::NurbsCurve(got) => $method(got, $($ver), *),
            Curve::IntersectionCurve(got) => $method(got, $($ver), *),
            Curve::SpineFrameCurve(got) => $method(got, $($ver), *),
            Curve::CertifiedImplicitIntersectionCurve(got) => $method(got, $($ver), *),
        }
    };
}

macro_rules! derive_curve_self_method {
    ($curve: expr, $method: expr, $($ver: ident),*) => {
        match $curve {
            Curve::Line(got) => Curve::Line($method(got, $($ver), *)),
            Curve::Circle(got) => Curve::Circle($method(got, $($ver), *)),
            Curve::BSplineCurve(got) => Curve::BSplineCurve($method(got, $($ver), *)),
            Curve::NurbsCurve(got) => Curve::NurbsCurve($method(got, $($ver), *)),
            Curve::IntersectionCurve(got) => Curve::IntersectionCurve($method(got, $($ver), *)),
            Curve::SpineFrameCurve(got) => Curve::SpineFrameCurve($method(got, $($ver), *)),
            Curve::CertifiedImplicitIntersectionCurve(got) => {
                Curve::CertifiedImplicitIntersectionCurve($method(got, $($ver), *))
            }
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

impl From<Line<Point3>> for Curve {
    #[inline(always)]
    fn from(x: Line<Point3>) -> Self {
        Curve::Line(x)
    }
}

impl From<Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>> for Curve {
    #[inline(always)]
    fn from(x: Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4>) -> Self {
        Curve::Circle(x)
    }
}

impl From<BSplineCurve<Point3>> for Curve {
    #[inline(always)]
    fn from(x: BSplineCurve<Point3>) -> Self {
        Curve::BSplineCurve(x)
    }
}

impl From<NurbsCurve<Vector4>> for Curve {
    #[inline(always)]
    fn from(x: NurbsCurve<Vector4>) -> Self {
        Curve::NurbsCurve(x)
    }
}

impl From<IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>> for Curve {
    #[inline(always)]
    fn from(x: IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>) -> Self {
        Curve::IntersectionCurve(x)
    }
}

impl From<CertifiedImplicitIntersectionCurve> for Curve {
    #[inline(always)]
    fn from(x: CertifiedImplicitIntersectionCurve) -> Self {
        Curve::CertifiedImplicitIntersectionCurve(x)
    }
}

impl TryFrom<Curve> for Line<Point3> {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::Line(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Curve> for Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::Circle(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Curve> for BSplineCurve<Point3> {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::BSplineCurve(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Curve> for NurbsCurve<Vector4> {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::NurbsCurve(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Curve> for IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>> {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::IntersectionCurve(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Curve> for CertifiedImplicitIntersectionCurve {
    type Error = Curve;
    fn try_from(value: Curve) -> std::result::Result<Self, Self::Error> {
        match value {
            Curve::CertifiedImplicitIntersectionCurve(x) => Ok(x),
            _ => Err(value),
        }
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
        // BG-CE-006: the placed circle stays analytic; the old conversion
        // degraded it to a NURBS here.
        Curve::Circle(*self)
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
            Curve::Circle(processed) => {
                ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry(processed)
                    .non_rationalized()
                    .clone()
            }
            Curve::BSplineCurve(curve) => BSplineCurve::new(
                curve.knot_vec().clone(),
                curve
                    .control_points()
                    .iter()
                    .map(|pt| pt.to_vec().extend(1.0))
                    .collect(),
            ),
            Curve::NurbsCurve(curve) => curve.non_rationalized().clone(),
            // `unimplemented!` is moved code, kept verbatim (BG-CE-006): the
            // ISC carrier still has no homotopy lift, and `lift_up` has no
            // error channel to refuse through. The `#[allow]` keeps the moved
            // behaviour while the mandatory module lint denies the macro.
            #[allow(clippy::unimplemented)]
            Curve::IntersectionCurve(_) => {
                unimplemented!("intersection curve cannot connect by homotopy")
            }
            // BG-CG-009-BREP: a spine-frame trajectory has no spline lift
            // either (it is a recipe evaluation, not a control-point curve)
            // and `lift_up` has no error channel. Follow the ISC precedent
            // verbatim; every certified include path routes the variant away
            // before reaching here.
            #[allow(clippy::unimplemented)]
            Curve::SpineFrameCurve(_) => {
                unimplemented!("spine-frame trajectory cannot connect by homotopy")
            }
            // BIE-003-CARRIER: a certified implicit intersection curve has no
            // spline lift either (it is a certified sample/frame stream, not a
            // control-point curve) and `lift_up` has no error channel. Follow
            // the ISC/SpineFrameCurve precedent verbatim; every certified
            // include path routes the variant away before reaching here.
            #[allow(clippy::unimplemented)]
            Curve::CertifiedImplicitIntersectionCurve(_) => {
                unimplemented!("certified implicit intersection curve cannot connect by homotopy")
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
    ParametricSurface,
    ParameterDivision2D,
    Invertible,
    SearchParameterD2,
)]
pub enum Surface {
    /// Plane
    Plane(Plane),
    /// cylinder
    Cylinder(Cylinder),
    /// cone
    Cone(Cone),
    /// sphere
    Sphere(Sphere),
    /// torus
    Torus(Torus),
    /// revoluted curve
    RevolutedCurve(RevolutedCurve<Curve>),
    /// RESERVED: no conversion emits this yet (BG-CE-007 will); the variant
    /// exists now so this release is the last breaking one. Tessellation and
    /// STEP-out still handle it.
    ExtrudedCurve(ExtrudedCurve<Curve, Vector3>),
    /// 3-dimensional B-spline surface
    BSplineSurface(BSplineSurface<Point3>),
    /// 3-dimensional NURBS Surface
    NurbsSurface(NurbsSurface<Vector4>),
    /// A placed surface: the inner carrier composed with an affine map.
    /// Exact under affine; the honest home for a transformed z-canonical
    /// carrier (BG-CE-006-r2).
    ///
    /// Center/apex-only transforms were rejected for the analytic carriers:
    /// under a rotation they silently move the wrong point and produce a
    /// surface that is not the transformed image. A rotation or general
    /// affine map therefore wraps the carrier here, where every parameter
    /// evaluation, derivative and inverse composes the map exactly.
    ///
    /// The inner carrier is boxed: `Processor` stores its entity inline, so
    /// `Processor<Surface, Matrix4>` would be a recursive type of infinite
    /// size (BG-CE-006-r2 deviation).
    Processor(Processor<Box<Surface>, Matrix4>),
    /// The parametric spine/profile realization surface (BG-CG-009-BREP,
    /// spec §5.10 as amended by BG-KV2-501-C6). Realizes
    /// `X(s, v) = C(s) + frame(s)·P(s, v)` over the landed recipe evaluators.
    /// The variant now carries the WHOLE-SWEEP closed value
    /// ([`SpineFrameSweep`]): the recipe stored once on the canonical
    /// `Box<Curve>` spine carrier, the realized window `[s0, s1] × [v0, v1]`
    /// riding on the closed value, and the sweep-level placement. The
    /// windowed realization decorator is derived from that closed value, never
    /// stored here.
    SpineFrameSurface(SpineFrameSweep),
}

macro_rules! derive_surface_method {
    ($surface: expr, $method: expr, $($ver: ident),*) => {
        match $surface {
            Self::Plane(got) => $method(got, $($ver), *),
            Self::Cylinder(got) => $method(got, $($ver), *),
            Self::Cone(got) => $method(got, $($ver), *),
            Self::Sphere(got) => $method(got, $($ver), *),
            Self::Torus(got) => $method(got, $($ver), *),
            Self::RevolutedCurve(got) => $method(got, $($ver), *),
            Self::ExtrudedCurve(got) => $method(got, $($ver), *),
            Self::BSplineSurface(got) => $method(got, $($ver), *),
            Self::NurbsSurface(got) => $method(got, $($ver), *),
            Self::Processor(got) => $method(got, $($ver), *),
            Self::SpineFrameSurface(got) => $method(got, $($ver), *),
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
        *self = self.transformed(trans);
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        match self {
            // BG-CE-006-r2: the analytic carriers are placed exactly. A
            // matrix whose linear part is exactly the identity is a
            // translation: move the placement point and keep the scalars.
            // Any other affine map is not representable by the bare carrier
            // (a center/apex-only transform is silently wrong under
            // rotation), so the carrier is placed instead.
            Self::Cylinder(entity) => {
                transform_analytic_carrier(*entity, trans, |cylinder, matrix| {
                    Cylinder::new(matrix.transform_point(cylinder.center()), cylinder.radius())
                        .map(|cylinder| cylinder.value)
                })
            }
            Self::Cone(entity) => transform_analytic_carrier(*entity, trans, |cone, matrix| {
                Cone::new(matrix.transform_point(cone.apex()), cone.half_angle())
                    .map(|cone| cone.value)
            }),
            Self::Sphere(entity) => transform_analytic_carrier(*entity, trans, |sphere, matrix| {
                Ok(Sphere::new(
                    matrix.transform_point(sphere.center()),
                    sphere.radius(),
                ))
            }),
            Self::Torus(entity) => transform_analytic_carrier(*entity, trans, |torus, matrix| {
                Ok(Torus::new(
                    matrix.transform_point(torus.center()),
                    torus.large_radius(),
                    torus.small_radius(),
                ))
            }),
            Self::Processor(processor) => Self::Processor(processor.transformed(trans)),
            Self::Plane(entity) => Self::Plane(entity.transformed(trans)),
            // AUD-005: the image of a surface of revolution under a
            // non-uniform scale or shear is generally NOT a surface of
            // revolution (a circular cylinder scaled by `diag(1, 2, 1)` is an
            // elliptic cylinder), so a matrix whose linear part is not exactly
            // the identity cannot rebuild the bare carrier. Such a map places
            // the surface, composing it exactly at every evaluation — the same
            // rule the analytic carriers use. Only a translation (identity
            // linear part) keeps the bare carrier: its axis image is the axis,
            // never degenerate.
            Self::RevolutedCurve(entity) if !identity_linear_part(trans) => {
                placed_surface(Surface::RevolutedCurve(entity.clone()), trans)
            }
            Self::RevolutedCurve(entity) => Self::RevolutedCurve(entity.transformed(trans)),
            Self::ExtrudedCurve(entity) => Self::ExtrudedCurve(entity.transformed(trans)),
            Self::BSplineSurface(entity) => Self::BSplineSurface(entity.transformed(trans)),
            Self::NurbsSurface(entity) => Self::NurbsSurface(entity.transformed(trans)),
            // BG-CG-009-BREP: the placement composes into the stored matrix
            // (the `SpineFrameSweep` closed value and the `SpineFrameCurve`
            // decorator carry one — the sweep-level placement); every
            // evaluation is exact under any affine map.
            Self::SpineFrameSurface(entity) => Self::SpineFrameSurface(entity.transformed(trans)),
        }
    }
}

/// Whether the 3×3 linear part of `matrix` is exactly the identity (no
/// epsilon; a placement built from translations and z-rotations compares
/// exactly, and an approximate match must not be trusted to keep the carrier
/// intact).
#[inline(always)]
fn identity_linear_part(matrix: Matrix4) -> bool {
    matrix[0][0] == 1.0
        && matrix[0][1] == 0.0
        && matrix[0][2] == 0.0
        && matrix[1][0] == 0.0
        && matrix[1][1] == 1.0
        && matrix[1][2] == 0.0
        && matrix[2][0] == 0.0
        && matrix[2][1] == 0.0
        && matrix[2][2] == 1.0
}

/// The transformed analytic carrier (BG-CE-006-r2).
///
/// Under a matrix with an identity linear part, `rebuild` moves the placement
/// point and keeps the scalars; under any other affine map the carrier is
/// placed — `Processor::with_transform(entity, matrix)` — where every
/// evaluation composes the map exactly. The bare-carrier failure arm is for
/// totality: a translated valid carrier cannot violate its scalar invariants.
#[inline(always)]
fn transform_analytic_carrier<E>(
    entity: E,
    matrix: Matrix4,
    rebuild: impl FnOnce(E, Matrix4) -> std::result::Result<E, Refusal>,
) -> Surface
where
    E: Copy + Into<Surface>,
{
    if identity_linear_part(matrix) {
        match rebuild(entity, matrix) {
            Ok(rebuilt) => rebuilt.into(),
            Err(_) => placed_surface(entity.into(), matrix),
        }
    } else {
        placed_surface(entity.into(), matrix)
    }
}

/// A placed surface: the carrier composed with an affine map, boxed because
/// `Processor` stores its entity inline (BG-CE-006-r2).
#[inline(always)]
fn placed_surface(surface: Surface, matrix: Matrix4) -> Surface {
    Surface::Processor(Processor::with_transform(Box::new(surface), matrix))
}

// The analytic carriers are bare and stateless, so a `Matrix4` transform can
// only be represented exactly by moving the placement point. A rotation or
// non-uniform scale relative to the carrier's own frame is not representable
// (BG-CE-006); the placement point is still transformed so translation and
// rigid placement compose correctly. `Cylinder::new`/`Cone::new` validate their
// scalar parameters, which are carried over unchanged, so they cannot refuse
// here; the `Err` arms are for totality only.

impl Transformed<Matrix4> for Cylinder {
    fn transform_by(&mut self, trans: Matrix4) {
        let center = trans.transform_point(self.center());
        if let Ok(cylinder) = Cylinder::new(center, self.radius()) {
            *self = cylinder.value;
        }
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        let center = trans.transform_point(self.center());
        match Cylinder::new(center, self.radius()) {
            Ok(cylinder) => cylinder.value,
            Err(_) => *self,
        }
    }
}

impl Transformed<Matrix4> for Cone {
    fn transform_by(&mut self, trans: Matrix4) {
        let apex = trans.transform_point(self.apex());
        if let Ok(cone) = Cone::new(apex, self.half_angle()) {
            *self = cone.value;
        }
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        let apex = trans.transform_point(self.apex());
        match Cone::new(apex, self.half_angle()) {
            Ok(cone) => cone.value,
            Err(_) => *self,
        }
    }
}

impl Transformed<Matrix4> for Sphere {
    fn transform_by(&mut self, trans: Matrix4) {
        *self = Sphere::new(trans.transform_point(self.center()), self.radius());
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        Sphere::new(trans.transform_point(self.center()), self.radius())
    }
}

impl Transformed<Matrix4> for Torus {
    fn transform_by(&mut self, trans: Matrix4) {
        *self = Torus::new(
            trans.transform_point(self.center()),
            self.large_radius(),
            self.small_radius(),
        );
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        Torus::new(
            trans.transform_point(self.center()),
            self.large_radius(),
            self.small_radius(),
        )
    }
}

impl<C: Transformed<Matrix4> + Clone> Transformed<Matrix4> for RevolutedCurve<C> {
    fn transform_by(&mut self, trans: Matrix4) {
        // AUD-005: a degenerate axis image (a projection or zero matrix)
        // cannot rebuild the carrier, so it is refused by identity — the
        // surface is left unchanged. An arbitrary axis is never substituted.
        let Some(axis) = transform_revolution_axis(trans, self.axis()) else {
            return;
        };
        let curve = self.entity_curve().clone().transformed(trans);
        let origin = trans.transform_point(self.origin());
        *self = RevolutedCurve::by_revolution(curve, origin, axis);
    }
    fn transformed(&self, trans: Matrix4) -> Self {
        match transform_revolution_axis(trans, self.axis()) {
            Some(axis) => {
                let curve = self.entity_curve().clone().transformed(trans);
                let origin = trans.transform_point(self.origin());
                RevolutedCurve::by_revolution(curve, origin, axis)
            }
            // AUD-005: a degenerate axis image is refused by identity.
            None => self.clone(),
        }
    }
}

/// The normalized image of a revolution axis under `trans`, or `None` when the
/// image is degenerate (zero or NaN — a projection onto the axis plane). A
/// degenerate axis image cannot be represented by the bare carrier, so the
/// caller must refuse rather than substitute an arbitrary axis (AUD-005).
fn transform_revolution_axis(trans: Matrix4, axis: Vector3) -> Option<Vector3> {
    let axis = trans.transform_vector(axis);
    let magnitude = axis.magnitude();
    if magnitude.is_finite() && magnitude > 0.0 {
        Some(axis / magnitude)
    } else {
        None
    }
}

// The analytic carriers carry no orientation, so inverting them is the
// identity: the parametrization and the outward normal are unchanged. This is
// the best a stateless `Copy` carrier can do (BG-CE-006); the former
// `Processor` wrapper carried the orientation these types cannot.

impl Invertible for Cylinder {
    fn invert(&mut self) {}
}

impl Invertible for Cone {
    fn invert(&mut self) {}
}

impl Invertible for Sphere {
    fn invert(&mut self) {}
}

impl Invertible for Torus {
    fn invert(&mut self) {}
}

impl From<Plane> for Surface {
    #[inline(always)]
    fn from(x: Plane) -> Self {
        Surface::Plane(x)
    }
}

impl From<Cylinder> for Surface {
    #[inline(always)]
    fn from(x: Cylinder) -> Self {
        Surface::Cylinder(x)
    }
}

impl From<Cone> for Surface {
    #[inline(always)]
    fn from(x: Cone) -> Self {
        Surface::Cone(x)
    }
}

impl From<Sphere> for Surface {
    #[inline(always)]
    fn from(x: Sphere) -> Self {
        Surface::Sphere(x)
    }
}

impl From<Torus> for Surface {
    #[inline(always)]
    fn from(x: Torus) -> Self {
        Surface::Torus(x)
    }
}

impl From<RevolutedCurve<Curve>> for Surface {
    #[inline(always)]
    fn from(x: RevolutedCurve<Curve>) -> Self {
        Surface::RevolutedCurve(x)
    }
}

impl From<ExtrudedCurve<Curve, Vector3>> for Surface {
    #[inline(always)]
    fn from(x: ExtrudedCurve<Curve, Vector3>) -> Self {
        Surface::ExtrudedCurve(x)
    }
}

impl From<BSplineSurface<Point3>> for Surface {
    #[inline(always)]
    fn from(x: BSplineSurface<Point3>) -> Self {
        Surface::BSplineSurface(x)
    }
}

impl From<NurbsSurface<Vector4>> for Surface {
    #[inline(always)]
    fn from(x: NurbsSurface<Vector4>) -> Self {
        Surface::NurbsSurface(x)
    }
}

impl TryFrom<Surface> for Plane {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::Plane(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for Cylinder {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::Cylinder(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for Cone {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::Cone(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for Sphere {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::Sphere(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for Torus {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::Torus(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for RevolutedCurve<Curve> {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::RevolutedCurve(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for ExtrudedCurve<Curve, Vector3> {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::ExtrudedCurve(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for BSplineSurface<Point3> {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::BSplineSurface(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl TryFrom<Surface> for NurbsSurface<Vector4> {
    type Error = Surface;
    fn try_from(value: Surface) -> std::result::Result<Self, Self::Error> {
        match value {
            Surface::NurbsSurface(x) => Ok(x),
            _ => Err(value),
        }
    }
}

impl IncludeCurve<Curve> for Surface {
    fn include(&self, curve: &Curve) -> Outcome<bool> {
        // BG-CE-006: a placed circle is routed through the NURBS the
        // pre-packet conversion produced. Every carrier's certified include
        // path was written against that curve shape; routing the circle back
        // through it preserves the certified answers exactly, and the four
        // analytic carriers keep their honest refusal below.
        if let Curve::Circle(circle) = curve {
            let nurbs = ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry(circle);
            return self.include(&Curve::NurbsCurve(nurbs));
        }
        match self {
            Surface::BSplineSurface(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                // BG-CG-009-BREP: certifying a spine-frame trajectory's
                // containment in a spline/plane/revolution carrier is outside
                // the certified envelope; refuse, never abort.
                Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }),
                // BIE-003-CARRIER: a certified implicit intersection curve's
                // containment in a spline/plane/revolution carrier is outside
                // this packet's certified envelope (BIE-005 answers it); refuse
                // typed, never abort.
                Curve::CertifiedImplicitIntersectionCurve(_) => {
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    })
                }
                Curve::Circle(_) => unreachable!("circles are degraded above"),
            },
            Surface::NurbsSurface(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                // BG-CG-009-BREP: certifying a spine-frame trajectory's
                // containment in a spline/plane/revolution carrier is outside
                // the certified envelope; refuse, never abort.
                Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }),
                // BIE-003-CARRIER: a certified implicit intersection curve's
                // containment in a spline/plane/revolution carrier is outside
                // this packet's certified envelope (BIE-005 answers it); refuse
                // typed, never abort.
                Curve::CertifiedImplicitIntersectionCurve(_) => {
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    })
                }
                Curve::Circle(_) => unreachable!("circles are degraded above"),
            },
            Surface::Plane(surface) => match curve {
                &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                Curve::BSplineCurve(curve) => surface.include(curve),
                Curve::NurbsCurve(curve) => surface.include(curve),
                Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                // BG-CG-009-BREP: certifying a spine-frame trajectory's
                // containment in a spline/plane/revolution carrier is outside
                // the certified envelope; refuse, never abort.
                Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }),
                // BIE-003-CARRIER: a certified implicit intersection curve's
                // containment in a spline/plane/revolution carrier is outside
                // this packet's certified envelope (BIE-005 answers it); refuse
                // typed, never abort.
                Curve::CertifiedImplicitIntersectionCurve(_) => {
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    })
                }
                Curve::Circle(_) => unreachable!("circles are degraded above"),
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
                        // BG-CG-009-BREP: a revolved spine-frame trajectory is
                        // outside the certified envelope; refuse.
                        Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                            spent: Budget::new(0, 0, 0),
                            witness: UnresolvedWitness::UncertifiedContainment,
                        }),
                        // BIE-003-CARRIER: a revolved certified implicit
                        // intersection curve is outside this packet's certified
                        // envelope (BIE-005 answers it); refuse typed.
                        Curve::CertifiedImplicitIntersectionCurve(_) => {
                            Err(Refusal::NumericallyUnresolved {
                                spent: Budget::new(0, 0, 0),
                                witness: UnresolvedWitness::UncertifiedContainment,
                            })
                        }
                        Curve::Circle(_) => unreachable!("circles are degraded above"),
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
                        // BG-CG-009-BREP: a revolved spine-frame trajectory is
                        // outside the certified envelope; refuse.
                        Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                            spent: Budget::new(0, 0, 0),
                            witness: UnresolvedWitness::UncertifiedContainment,
                        }),
                        // BIE-003-CARRIER: a revolved certified implicit
                        // intersection curve is outside this packet's certified
                        // envelope (BIE-005 answers it); refuse typed.
                        Curve::CertifiedImplicitIntersectionCurve(_) => {
                            Err(Refusal::NumericallyUnresolved {
                                spent: Budget::new(0, 0, 0),
                                witness: UnresolvedWitness::UncertifiedContainment,
                            })
                        }
                        Curve::Circle(_) => unreachable!("circles are degraded above"),
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
                // A circle profile is the pre-packet NURBS arc: degrade it the
                // same way the boundary curves above are degraded, so the
                // certified revolution include path sees the curve shapes it
                // was written against (BG-CE-006).
                Curve::Circle(entity_curve) => {
                    let entity_curve =
                        ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry(entity_curve);
                    let surface = RevolutedCurve::by_revolution(
                        &entity_curve,
                        surface.origin(),
                        surface.axis(),
                    );
                    match curve {
                        &Curve::Line(curve) => surface.include(&BSplineCurve::from(curve)),
                        Curve::BSplineCurve(curve) => surface.include(curve),
                        Curve::NurbsCurve(curve) => surface.include(curve),
                        Curve::IntersectionCurve(ic) => self.include_intersection_curve(ic),
                        // BG-CG-009-BREP: a revolved spine-frame trajectory is
                        // outside the certified envelope; refuse.
                        Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                            spent: Budget::new(0, 0, 0),
                            witness: UnresolvedWitness::UncertifiedContainment,
                        }),
                        // BIE-003-CARRIER: a revolved certified implicit
                        // intersection curve is outside this packet's certified
                        // envelope (BIE-005 answers it); refuse typed.
                        Curve::CertifiedImplicitIntersectionCurve(_) => {
                            Err(Refusal::NumericallyUnresolved {
                                spent: Budget::new(0, 0, 0),
                                witness: UnresolvedWitness::UncertifiedContainment,
                            })
                        }
                        Curve::Circle(_) => unreachable!("circles are degraded above"),
                    }
                }
                // BG-CG-009-BREP: a surface of revolution whose profile is a
                // spine-frame trajectory is outside the certified envelope;
                // refuse, not abort.
                Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                }),
                // BIE-003-CARRIER: a surface of revolution whose profile is a
                // certified implicit intersection curve is outside this
                // packet's certified envelope; refuse typed.
                Curve::CertifiedImplicitIntersectionCurve(_) => {
                    Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::UncertifiedContainment,
                    })
                }
            },
            // Certified curve-in-analytic-surface containment is BG-CE-002 /
            // BG-ENC work; the analytic carriers refuse honestly for now.
            Surface::Cylinder(_)
            | Surface::Cone(_)
            | Surface::Sphere(_)
            | Surface::Torus(_)
            | Surface::ExtrudedCurve(_) => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::UncertifiedContainment,
            }),
            // A placed surface's containment question is its carrier's, which
            // the analytic carriers refuse honestly for now; the spline
            // carriers are reached through their own arms above, never wrapped
            // here (BG-CE-006-r2).
            Surface::Processor(_) => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::UncertifiedContainment,
            }),
            // BG-CG-009-BREP: the spine-frame surface includes exactly its
            // own boundary curves (structural equality); any other curve asks
            // a question the decorator answers honestly.
            Surface::SpineFrameSurface(surface) => surface.include(curve),
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
            // BG-CG-009-BREP: a spine-frame trajectory cannot be lifted either
            // (the ISC precedent); the plane containment question is a
            // numerical-search matter and refuses honestly rather than abort.
            Curve::SpineFrameCurve(_) => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::UncertifiedContainment,
            }),
            // BIE-003-CARRIER: a certified implicit intersection curve cannot
            // be lifted either (the ISC/SpineFrameCurve precedent); the plane
            // containment question is BIE-005's certified answer and refuses
            // honestly rather than abort.
            Curve::CertifiedImplicitIntersectionCurve(_) => Err(Refusal::NumericallyUnresolved {
                spent: Budget::new(0, 0, 0),
                witness: UnresolvedWitness::UncertifiedContainment,
            }),
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
        Surface::RevolutedCurve(self.clone())
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
            Surface::Cylinder(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::Cone(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::Sphere(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::Torus(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::BSplineSurface(bspsurface) => {
                bspsurface.search_nearest_parameter(point, hint, trials)
            }
            Surface::NurbsSurface(surface) => surface.search_nearest_parameter(point, hint, trials),
            Surface::ExtrudedCurve(surface) => {
                surface.search_nearest_parameter(point, hint, trials)
            }
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
            Surface::Processor(processor) => {
                processor.search_nearest_parameter(point, hint, trials)
            }
            Surface::SpineFrameSurface(surface) => {
                surface.search_nearest_parameter(point, hint, trials)
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
            (Curve::Circle(c0), Curve::Circle(c1)) => {
                // BG-CE-006: attempt the analytic cylinder. The placed circle
                // is a cylinder only for an exact z-preserving uniform
                // placement extruded along ±z; every condition below is an
                // exact comparison (no epsilon — such placements are built
                // from z-rotations and translations and compare exactly), and
                // any failure degrades to the homotopy NURBS — exactly the
                // pre-packet behaviour for every circle.
                let Matrix4 {
                    x: m1,
                    y: m2,
                    z: m3,
                    w: tw,
                } = *c0.transform();
                let radius = m1.magnitude();
                let t = tw.to_point();
                if m1.z == 0.0
                    && m2.z == 0.0
                    && m3.x == 0.0
                    && m3.y == 0.0
                    && radius == m2.magnitude()
                    && m1.dot(m2) == 0.0
                    && radius > 0.0
                    && vector.x == 0.0
                    && vector.y == 0.0
                {
                    // `Cylinder::new` re-validates the radius (H-1); an exact
                    // radius check passed above, so it cannot refuse here, but
                    // the failure degrades rather than unwrapping.
                    if let Ok(cylinder) = Cylinder::new(t, radius) {
                        return Surface::Cylinder(cylinder.value);
                    }
                }
                let curve0 = Curve::Circle(*c0).lift_up();
                let curve1 = Curve::Circle(c1).lift_up();
                NurbsSurface::new(BSplineSurface::homotopy(curve0, curve1)).into()
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
            // `curve1` is `curve0` pushed by the extrusion vector, so the two
            // section curves always share the entity curve's variant; mixed
            // pairs are impossible. The arm stays total (BG-CE-006) and
            // degrades like every near-miss rather than aborting.
            (curve0, curve1) => {
                let curve0 = curve0.lift_up();
                let curve1 = curve1.lift_up();
                NurbsSurface::new(BSplineSurface::homotopy(curve0, curve1)).into()
            }
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

#[cfg(test)]
// BG-CE-006 tests: the circle stays analytic through conversion, and a
// non-canonical extrusion degrades to exactly the old homotopy NURBS.
mod circle_conversion_tests {
    use super::*;
    use std::f64::consts::TAU;

    /// The placed unit circle at radius `RADIUS` from the origin in the
    /// xy-plane, full range.
    const RADIUS: f64 = 2.0;

    fn placed_circle() -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
        let trimmed = TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU));
        let mat = Matrix4::from_translation(Vector3::new(RADIUS, 0.0, 0.0));
        Processor::with_transform(trimmed, mat)
    }

    #[test]
    fn circle_conversion_preserves_variant() {
        let placed = placed_circle();
        let curve: Curve = placed.to_same_geometry();
        assert!(matches!(curve, Curve::Circle(_)));
        // The old conversion: the NURBS of the same placed circle. Sample its
        // own valid parameters and recover the angle on the placed circle, so
        // the two curves are compared at the same geometric points. The
        // full-range NURBS is degenerate at its midpoint parameter (w = 0), so
        // those samples are skipped.
        let nurbs: NurbsCurve<Vector4> =
            ToSameGeometry::<NurbsCurve<Vector4>>::to_same_geometry(&placed);
        if let Curve::Circle(processed) = &curve {
            const SAMPLES: usize = 32;
            for i in 0..=SAMPLES {
                let s = i as f64 / SAMPLES as f64;
                let p = nurbs.subs(s);
                // The full-range NURBS is degenerate at its midpoint parameter
                // (w = 0), so those samples are skipped.
                if !p.x.is_finite() {
                    continue;
                }
                // The placed circle is the exact carrier; the inverse search
                // recovers the angle of every on-curve point.
                if let Some(t) = processed.search_parameter(p, None, 100) {
                    assert_near!(processed.subs(t), p);
                }
            }
        }
    }

    #[test]
    fn full_circle_include_on_plane_is_true() {
        // AUD-009: a full circle lying in a plane must include as `true`
        // through `Surface::include(&Curve::Circle(_))`. The reachable arm is
        // `Surface::Plane` → `IncludeCurve<NurbsCurve<Vector4>> for Plane`
        // (the circle is routed through the NURBS conversion at the top of
        // `Surface::include`), whose control-point test skips the
        // weight-0-middle control points of the two half-circle spans. On the
        // buggy tree the single-arc conversion degraded to NaN at the antipode
        // and this answered `false`.
        let trimmed = TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU));
        let circle = Curve::Circle(Processor::with_transform(
            trimmed,
            Matrix4::from_translation(Vector3::new(0.0, 0.0, 1.0)),
        ));
        let plane = Surface::Plane(Plane::new(
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ));
        let out = plane.include(&circle);
        assert!(
            matches!(out, Ok(Certified { value: true, .. })),
            "a full circle in its plane must include as true, got {out:?}"
        );
    }
}

#[cfg(test)]
mod extruded_circle_tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_6, TAU};

    /// A placed circle whose plane is tilted `FRAC_PI_6` about the x axis:
    /// not a z-preserving placement, so no cylinder may be produced.
    fn tilted_circle() -> Processor<TrimmedCurve<UnitCircle<Point3>>, Matrix4> {
        let trimmed = TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU));
        let tilt = Matrix4::from_axis_angle(Vector3::unit_x(), Rad(FRAC_PI_6));
        Processor::with_transform(trimmed, tilt)
    }

    #[test]
    fn extruded_noncanonical_circle_degrades() {
        let circle: Curve = tilted_circle().to_same_geometry();
        // A non-z extrusion vector: the cylinder conditions require ±z.
        let vector = Vector3::new(1.0, 0.0, 1.0).normalize();
        let extruded = ExtrudedCurve::by_extrusion(circle.clone(), vector);
        let surface: Surface = extruded.to_same_geometry();
        assert!(
            !matches!(surface, Surface::Cylinder(_)),
            "a tilted circle must not become a cylinder"
        );
        // Today's behaviour: the homotopy NURBS of the lifted circles. Both
        // surfaces share the construction, so the parametrizations agree and
        // points can be compared at the same parameters.
        let trsl = Matrix4::from_translation(vector);
        let curve1 = circle.transformed(trsl);
        let reference =
            NurbsSurface::new(BSplineSurface::homotopy(circle.lift_up(), curve1.lift_up()));
        const SAMPLES: usize = 16;
        for i in 0..=SAMPLES {
            for j in 0..=SAMPLES {
                let s = i as f64 / SAMPLES as f64;
                let t = j as f64 / SAMPLES as f64;
                let a = surface.subs(s, t);
                let b = reference.subs(s, t);
                // The full-range circle's rational NURBS carries a weight
                // double-zero at its midpoint parameter (w(s) = (2s-1)^2), so
                // the old conversion itself evaluated to NaN there; skip the
                // samples where it did.
                if !a.x.is_finite() || !b.x.is_finite() {
                    continue;
                }
                assert_near!(a, b);
            }
        }
    }
}

#[cfg(test)]
// BG-CE-006-r2 tests. The carriers and transforms are hand-built witnesses —
// not paths reachable from untrusted geometry — so the mandatory H-1 lints on
// unwrap/expect/panic do not apply to the assertions here.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod placed_analytic_transform_tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, TAU};

    #[test]
    fn placed_analytic_transform_goes_to_processor() {
        // A translation has an exactly-identity linear part, so the bare
        // carrier survives and its center moves.
        let cylinder = Cylinder::new(Point3::origin(), 1.5)
            .expect("a unit-scale radius is valid")
            .value;
        let translation = Matrix4::from_translation(Vector3::new(2.0, -3.0, 4.0));
        let moved = Surface::Cylinder(cylinder).transformed(translation);
        let Surface::Cylinder(moved) = moved else {
            panic!("a translation must keep the bare cylinder");
        };
        assert_near!(moved.center(), Point3::new(2.0, -3.0, 4.0));
        assert_near!(moved.radius(), 1.5);

        // A rotation has a non-identity linear part, so the carrier must be
        // placed rather than silently deformed: `Surface::Processor`, whose
        // point set is the rotated original.
        let rotation = Matrix4::from_axis_angle(Vector3::unit_z(), Rad(FRAC_PI_2));
        let rotated = Surface::Cylinder(cylinder).transformed(rotation);
        let Surface::Processor(placed) = rotated else {
            panic!("a rotated cylinder must become a placed surface");
        };
        const SAMPLES: usize = 16;
        for i in 0..=SAMPLES {
            for j in 0..=SAMPLES {
                let u = TAU * i as f64 / SAMPLES as f64;
                let v = j as f64 / SAMPLES as f64;
                let expected = rotation.transform_point(cylinder.subs(u, v));
                assert_near!(placed.subs(u, v), expected);
            }
        }
    }

    #[test]
    fn revoluted_curve_nonconformal_transform_is_placed() {
        // AUD-005: the image of a surface of revolution under a non-uniform
        // scale is generally NOT a surface of revolution. A unit circular
        // cylinder scaled by `diag(1, 2, 1)` is the elliptic cylinder
        // `(cos v, 2 sin v, u)`; the transform must place the surface, never
        // rebuild the bare revolved carrier on a wrong axis.
        let cylinder = Surface::RevolutedCurve(RevolutedCurve::by_revolution(
            Curve::Line(Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0))),
            Point3::origin(),
            Vector3::unit_z(),
        ));
        let scale = Matrix4::from_nonuniform_scale(1.0, 2.0, 1.0);
        let placed = cylinder.transformed(scale);
        assert!(
            !matches!(placed, Surface::RevolutedCurve(_)),
            "a non-uniform scale must not come back as a bare revolved carrier"
        );
        let Surface::Processor(processor) = placed else {
            panic!("a non-uniform scale of a revolved surface must be placed");
        };
        const SAMPLES: usize = 16;
        for i in 0..=SAMPLES {
            for j in 0..=SAMPLES {
                let u = i as f64 / SAMPLES as f64;
                let v = TAU * j as f64 / SAMPLES as f64;
                assert_near!(processor.subs(u, v), Point3::new(v.cos(), 2.0 * v.sin(), u));
            }
        }
    }
}

#[cfg(test)]
// BIE-003-CARRIER tests. The certified carrier and its hand-built sample
// stream are test witnesses — the H-1 deny lints on unwrap/expect/panic do not
// apply to the assertions here, and every accessor path stays unwrap-free.
mod certified_carrier_canonical_tests {
    use super::*;
    use crate::constructive::intersection_carrier::{CertifiedSample, IntersectionFrame};
    use crate::constructive::{CertifiedImplicitIntersectionCurve, Frame3};

    /// The orthonormal +x-tangent frame shared by every straight-line station.
    fn x_frame() -> Option<IntersectionFrame> {
        let frame =
            Frame3::try_new(Vector3::unit_x(), Vector3::unit_z(), -Vector3::unit_y()).ok()?;
        Some(IntersectionFrame {
            tangent: frame.tangent,
            normal: frame.normal,
            binormal: frame.binormal,
        })
    }

    /// A certified straight carrier along +x at z = 0 with three stations.
    fn certified_line() -> Option<(Curve, CertifiedImplicitIntersectionCurve)> {
        let frame = x_frame()?;
        let positions = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let samples: Vec<CertifiedSample> = positions
            .iter()
            .map(|&position| CertifiedSample {
                position,
                frame,
                method: Method::Float,
            })
            .collect();
        match CertifiedImplicitIntersectionCurve::try_new(&samples, None) {
            Ok(certified) => {
                let value = certified.value;
                Some((value.clone().into(), value))
            }
            Err(_) => None,
        }
    }

    #[test]
    fn canonical_ripple_delegates_all_methods() {
        // The new variant is additive: the carrier enters the canonical enum
        // through the `From` impl and every landed method/macro arm delegates
        // (or refuses typed) for it — compile-enforced by this module building
        // against the full enum, asserted here on the carrier's behaviour.
        let (curve, carrier) = match certified_line() {
            Some(pair) => pair,
            None => return,
        };
        // Variant identity + the `TryFrom` downcast.
        assert!(matches!(
            &curve,
            Curve::CertifiedImplicitIntersectionCurve(_)
        ));
        let downcast: CertifiedImplicitIntersectionCurve =
            match CertifiedImplicitIntersectionCurve::try_from(curve.clone()) {
                Ok(carrier) => carrier,
                Err(_) => return,
            };
        assert_eq!(downcast.polyline(), carrier.polyline());

        // Parametric evaluation delegates through the enum.
        let tolerance = 1.0e-9; // H-3
        assert!((curve.subs(0.0) - Point3::new(0.0, 0.0, 0.0)).magnitude() <= tolerance);
        assert!((curve.subs(2.0) - Point3::new(2.0, 0.0, 0.0)).magnitude() <= tolerance);
        assert!((curve.der(1.0) - Vector3::unit_x()).magnitude() <= tolerance);
        assert_eq!(curve.range_tuple(), (0.0, 2.0));

        // `Cut` delegates: the split lands on the certified station at
        // parameter 1, and both halves keep the certified-carrier variant.
        let mut head = curve.clone();
        let tail = head.cut(1.0);
        assert!(matches!(head, Curve::CertifiedImplicitIntersectionCurve(_)));
        assert!(matches!(tail, Curve::CertifiedImplicitIntersectionCurve(_)));
        assert_eq!(head.range_tuple().1, 1.0);
        assert_eq!(tail.range_tuple().0, 1.0);
        assert_eq!(tail.range_tuple().1, 2.0);

        // `Invertible` delegates: the domain is unchanged, the geometry is
        // traversed back to front.
        let mut inverted = curve.clone();
        inverted.invert();
        assert_eq!(inverted.range_tuple(), (0.0, 2.0));
        assert!((inverted.subs(0.0) - Point3::new(2.0, 0.0, 0.0)).magnitude() <= tolerance);
        assert!((inverted.subs(2.0) - Point3::new(0.0, 0.0, 0.0)).magnitude() <= tolerance);

        // `Transformed` delegates through the macro arms.
        let translation = Matrix4::from_translation(Vector3::new(0.0, 0.0, 5.0));
        let moved = curve.transformed(translation);
        assert!((moved.front() - Point3::new(0.0, 0.0, 5.0)).magnitude() <= tolerance);
        assert!((moved.back() - Point3::new(2.0, 0.0, 5.0)).magnitude() <= tolerance);

        // Parameter search delegates.
        let param_tolerance = 1.0e-6; // H-3
        let found = curve.search_parameter(Point3::new(1.0, 0.0, 0.0), None, 50);
        match found {
            Some(t) => assert!((t - 1.0).abs() <= param_tolerance),
            None => return,
        }

        // Containment refuses typed on every carrier surface — the certified
        // envelope answers the containment question only for its own pair
        // (BIE-005); never a panic, never a fabricated `Proven`.
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let on_plane = Surface::Plane(plane).include(&curve);
        assert!(
            matches!(
                on_plane,
                Err(Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::UncertifiedContainment,
                    ..
                })
            ),
            "plane containment of a certified carrier must refuse typed, got {on_plane:?}"
        );
        let patch = BSplineSurface::new(
            (KnotVec::bezier_knot(1), KnotVec::bezier_knot(1)),
            vec![
                vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
                vec![Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
            ],
        );
        let on_patch = Surface::BSplineSurface(patch).include(&curve);
        assert!(
            matches!(
                on_patch,
                Err(Refusal::NumericallyUnresolved {
                    witness: UnresolvedWitness::UncertifiedContainment,
                    ..
                })
            ),
            "spline containment of a certified carrier must refuse typed, got {on_patch:?}"
        );
    }
}
