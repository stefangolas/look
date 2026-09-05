#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The BIE unit-shape fixture kit (BIE-000-CONTRACT, spine §3): closed-form
//! unit shapes for the restricted pair `SpineFrameSweep × canonical` whose
//! interaction ground truths are stated here and machine-checked in the
//! in-module tests. No solver is called to build or check a fixture — every
//! ground truth is a closed-form constant derived in the doc comments and
//! asserted under `// H-3` tolerance discipline.
//!
//! **TEST SUPPORT ONLY.** The module is `#[doc(hidden)] pub`, excluded from
//! the certified API surface, but reachable by BIE wave packets' tests through
//! the crate's public path (`crate::construct::bie::fixtures`).
//!
//! Each fixture is a pair of carriers with a stated, machine-checked ground
//! truth:
//!
//! 1. **plane × sphere** ([`plane_sphere_fixture`]) — the plane z = 1 cuts a
//!    sphere of radius 2 about the origin. The intersection is the circle
//!    whose centre is the perpendicular foot of the sphere centre onto the
//!    plane and whose radius is `sqrt(R² − δ²)`, `δ` the signed distance of
//!    the sphere centre to the plane.
//! 2. **plane × cylinder** ([`plane_cylinder_fixture`]) — a transverse plane
//!    of incident angle `θ` cuts a canonical cylinder of radius `r` about the
//!    z-axis in an ellipse with semi-axes `r` and `r / |sin θ|` (the packet's
//!    closed form), centred at the axis ∩ plane point.
//! 3. **sweep × plane** ([`sweep_plane_fixture`]) — a straight-spine,
//!    `Scale`-of-a-circle `SpineFrameSweep` unit shape (continuous circular
//!    section; see [`ScaleCircleSweepUnit`]) crossing a plane perpendicular to
//!    the spine: the section is the ring at the station `s*` the plane
//!    equation selects, a circle of radius `radius(s*)` about the spine point
//!    `C(s*)`.
//! 4. **Determinism** — [`unit_shape_kit`] builds the whole kit from ordered,
//!    dyadic data with no hash iteration, so two constructions are equal.
//!
//! Every stored ground truth is tagged [`CertificateValue`] with
//! `Method::Float` (H-6: a closed form evaluated in `f64` is never recorded
//! `Exact`).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, no `panic!`, and
//! no module-level `allow`.

use crate::construct::bie::{CertificateValue, WitnessCell};
use crate::construct::refusal::ConstructRefusal;
use truck_base::evidence::Method;
use truck_geometry::prelude::{Cylinder, InnerSpace, Plane, Point3, Sphere, Vector3};

/// The certified intersection circle of a plane × sphere unit shape.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaneSphereFixture {
    /// The transverse plane (z = 1): origin `(0, 0, 1)`, u-axis `+x`, v-axis
    /// `+y`, unit normal `+z`.
    pub plane: Plane,
    /// The sphere of radius 2 about the origin.
    pub sphere: Sphere,
    /// The `(u, v) × (s, t)` product parameter cell of the pair — the search
    /// box a solver bisects. `(u, v)` is the plane's parameter box (in plane
    /// coordinates) covering the section circle; `(s, t)` is the sphere's
    /// (latitude, longitude) box.
    pub cell: WitnessCell,
    /// The certified section-circle centre (the perpendicular foot of the
    /// sphere centre onto the plane).
    pub centre: CertificateValue,
    /// The certified section-circle radius.
    pub radius: CertificateValue,
}

/// Fixture 1 (plane × sphere): the plane `z = 1` through a sphere of radius 2
/// about the origin.
///
/// Closed form. Let the plane be `n · x = d` (unit normal `n`, `d = n · o`),
/// the sphere centre `c` and radius `R`, and let `δ = (c − o) · n` be the
/// signed distance of `c` to the plane. The section is the circle
///
/// ```text
/// centre c′ = c − δ·n          (the perpendicular foot of c on the plane)
/// radius  ρ = sqrt(R² − δ²)
/// ```
///
/// with `|δ| < R` (the plane meets the shell). With `c = (0,0,0)`, `R = 2`,
/// `o = (0,0,1)`, `n = +z`: `δ = −1`, `c′ = (0,0,1)`, `ρ = sqrt(3)`. The
/// circle lies in the plane (its centre is on it and it is traced in the
/// plane's own basis) and every one of its points is at distance `R` from `c`.
pub fn plane_sphere_fixture() -> PlaneSphereFixture {
    let plane = plane_z(1.0);
    let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
    let (centre, radius) = sphere_plane_section_circle(&sphere, &plane);
    let cell = WitnessCell::new(
        (-2.0, 2.0),
        (-2.0, 2.0),
        (0.0, std::f64::consts::PI),
        (0.0, std::f64::consts::TAU),
    );
    PlaneSphereFixture {
        plane,
        sphere,
        cell,
        centre: CertificateValue::point(centre, Method::Float),
        radius: CertificateValue::scalar(radius, Method::Float),
    }
}

/// The certified intersection ellipse of a plane × cylinder unit shape.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaneCylinderFixture {
    /// The canonical cylinder: radius 2 about the z-axis through the origin.
    pub cylinder: Cylinder,
    /// The transverse plane through the origin with unit normal
    /// `(sin β, 0, cos β)`, `β = π/3`, so the incident angle `θ` between the
    /// plane and the cylinder axis satisfies `|sin θ| = 1/2`. The plane's
    /// u-axis is the in-plane unit direction `(cos β, 0, −sin β)` (the ellipse
    /// major direction), its v-axis is `+y` (the ellipse minor direction).
    pub plane: Plane,
    /// The `(u, v) × (s, t)` product parameter cell of the pair. `(u, v)` is
    /// the cylinder's (angle, height) box; `(s, t)` is the plane's parameter
    /// box (in plane coordinates) covering the ellipse.
    pub cell: WitnessCell,
    /// The certified `|sin θ|` of the plane's incidence against the cylinder
    /// axis.
    pub sin_theta: CertificateValue,
    /// The certified section-centre: the point where the cylinder axis meets
    /// the plane.
    pub centre: CertificateValue,
    /// The certified ellipse semi-axis along the plane u-axis: `r / |sin θ|`.
    pub semi_major: CertificateValue,
    /// The certified ellipse semi-axis along the plane v-axis: `r`.
    pub semi_minor: CertificateValue,
}

/// Fixture 2 (plane × cylinder): the transverse plane through the origin with
/// unit normal `(sin β, 0, cos β)`, `β = π/3`, cuts the canonical cylinder of
/// radius `r = 2` about the z-axis.
///
/// Closed form. Let the cylinder axis be the z-axis, `r` the radius, and let
/// the cutting plane make incident angle `θ` with the axis (`|sin θ|` is the
/// axial component of the plane's unit normal). The section is an ellipse
/// centred where the axis meets the plane (the origin here), with
///
/// ```text
/// minor semi-axis  b = r            along the in-plane direction ⊥ the axis
/// major semi-axis  a = r / |sin θ|  along the in-plane direction ⊥ that one
/// ```
///
/// With `r = 2` and `|sin θ| = |n · ẑ| = |cos β| = 1/2`: `a = 4` (along the
/// plane u-axis `(1/2, 0, −√3/2)`) and `b = 2` (along the plane v-axis
/// `(0, 1, 0)`). Every traced ellipse point is at distance `r` from the
/// cylinder axis and lies on the plane — the two carrier memberships the test
/// machine-checks.
pub fn plane_cylinder_fixture() -> Result<PlaneCylinderFixture, ConstructRefusal> {
    let half = 0.5_f64;
    let root3 = 3.0_f64.sqrt();
    let cylinder = cylinder_r(Point3::new(0.0, 0.0, 0.0), 2.0)?;
    // Plane through the origin, u-axis (cos β, 0, −sin β), v-axis +y. The
    // u/v axes are unit and orthogonal, so the normal is exactly
    // u × v = (sin β, 0, cos β).
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(half, 0.0, -root3 / 2.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    let n = plane.normal();
    let axis = Vector3::unit_z();
    let sin_theta = n.dot(axis).abs();
    let semi_major = cylinder.radius() / sin_theta;
    let semi_minor = cylinder.radius();
    let cell = WitnessCell::new(
        (0.0, std::f64::consts::TAU),
        (-4.0, 4.0),
        (-5.0, 5.0),
        (-5.0, 5.0),
    );
    Ok(PlaneCylinderFixture {
        cylinder,
        plane,
        cell,
        sin_theta: CertificateValue::scalar(sin_theta, Method::Float),
        centre: CertificateValue::point(Point3::new(0.0, 0.0, 0.0), Method::Float),
        semi_major: CertificateValue::scalar(semi_major, Method::Float),
        semi_minor: CertificateValue::scalar(semi_minor, Method::Float),
    })
}

/// The straight-spine, `Scale`-of-a-circle `SpineFrameSweep` unit shape, in
/// its continuous circular-section limit.
///
/// The landed windowed sweep realizes a polygonal profile ring; the unit shape
/// this record names is the continuous circular section the restricted engine
/// solves, `X(s, v) = C(s) + radius(s)·(cos 2πv, sin 2πv, 0)` over the
/// windowed domain `[s0, s1] × [v0, v1]`, where `C(s)` is the straight spine
/// `spine_from + s·(spine_to − spine_from)`, `s ∈ [s0, s1]`, and the radius
/// follows the linear `Scale` law `radius(s) = radius_start +
/// (radius_end − radius_start)·(s − s0)/(s1 − s0)`. This is the
/// `ProfileLaw::Scale`-of-a-circle sweep of the packet's fixture list.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleCircleSweepUnit {
    /// The first spine point `C(s0)`.
    pub spine_from: Point3,
    /// The last spine point `C(s1)`.
    pub spine_to: Point3,
    /// The scale radius at station `s0`.
    pub radius_start: f64,
    /// The scale radius at station `s1`.
    pub radius_end: f64,
    /// The spine window start station.
    pub s0: f64,
    /// The spine window end station.
    pub s1: f64,
    /// The ring window start.
    pub v0: f64,
    /// The ring window end.
    pub v1: f64,
}

impl ScaleCircleSweepUnit {
    /// The straight-spine point at station `s`: `C(s) = from + (s − s0)/(s1 −
    /// s0)·(to − from)` (closed form; total arithmetic).
    pub fn spine_point(&self, s: f64) -> Point3 {
        let t = (s - self.s0) / (self.s1 - self.s0);
        let to = self.spine_to - self.spine_from;
        self.spine_from + t * to
    }

    /// The scale radius at station `s` (linear law, closed form).
    pub fn radius_at(&self, s: f64) -> f64 {
        let t = (s - self.s0) / (self.s1 - self.s0);
        self.radius_start + (self.radius_end - self.radius_start) * t
    }

    /// The unit-shape surface point `X(s, v)` at station `s` and ring
    /// parameter `v ∈ [v0, v1]`. The ring is the exact circle of radius
    /// `radius(s)` in the plane through `C(s)` perpendicular to the spine:
    /// `X(s, v) = C(s) + radius(s)·(cos 2πv, sin 2πv, 0)` in the z-aligned
    /// placement of this unit shape.
    pub fn point(&self, s: f64, v: f64) -> Point3 {
        let r = self.radius_at(s);
        let angle = std::f64::consts::TAU * v;
        let (sine, cosine) = angle.sin_cos();
        self.spine_point(s) + r * Vector3::new(cosine, sine, 0.0)
    }
}

/// The certified sweep × plane section of the unit shape.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepPlaneFixture {
    /// The straight-spine `Scale`-of-a-circle sweep unit shape over
    /// `[0, 1] × [0, 1]`.
    pub sweep: ScaleCircleSweepUnit,
    /// The transverse plane `z = 3/4` (unit normal `+z`, parallel to the
    /// spine).
    pub plane: Plane,
    /// The `(u, v) × (s, t)` product parameter cell of the pair. `(u, v)` is
    /// the sweep's `(s, v)` window; `(s, t)` is the plane's parameter box
    /// covering the section ring.
    pub cell: WitnessCell,
    /// The certified station `s*` the plane selects.
    pub station: CertificateValue,
    /// The certified section centre: the spine point `C(s*)`.
    pub centre: CertificateValue,
    /// The certified section radius: `radius(s*)`.
    pub radius: CertificateValue,
}

/// Fixture 3 (sweep × plane): the straight-spine, `Scale`-of-a-circle sweep
/// unit shape over the window `[0, 1] × [0, 1]` crossing the transverse plane
/// `z = 3/4`.
///
/// Closed form. The spine is `C(s) = (0, 0, s)` (from `(0,0,0)` to
/// `(0,0,1)`), the plane is `n · x = d` with `n = +z`, `d = 3/4`, and the
/// ring is perpendicular to the spine, so `n · X(s, v) = n · C(s)` for every
/// ring parameter `v`. A sweep point lies on the plane exactly when its
/// station satisfies `n · C(s) = d`, i.e.
///
/// ```text
/// s* = (d − n·from) / (n·(to − from)) = 3/4
/// ```
///
/// so the section is the whole windowed ring at `s*`: the circle of radius
/// `radius(s*) = radius_start + (radius_end − radius_start)·s*` about
/// `C(s*)`. With `radius_start = 1`, `radius_end = 1/2`: `radius(3/4) = 5/8`.
pub fn sweep_plane_fixture() -> SweepPlaneFixture {
    let sweep = ScaleCircleSweepUnit {
        spine_from: Point3::new(0.0, 0.0, 0.0),
        spine_to: Point3::new(0.0, 0.0, 1.0),
        radius_start: 1.0,
        radius_end: 0.5,
        s0: 0.0,
        s1: 1.0,
        v0: 0.0,
        v1: 1.0,
    };
    let plane = plane_z(0.75);
    let station = station_of_plane(&sweep, &plane);
    let centre = sweep.spine_point(station);
    let radius = sweep.radius_at(station);
    let cell = WitnessCell::new((0.0, 1.0), (0.0, 1.0), (-1.0, 1.0), (-1.0, 1.0));
    SweepPlaneFixture {
        sweep,
        plane,
        cell,
        station: CertificateValue::scalar(station, Method::Float),
        centre: CertificateValue::point(centre, Method::Float),
        radius: CertificateValue::scalar(radius, Method::Float),
    }
}

/// The whole unit-shape kit, in a fixed order: building it twice yields equal
/// values (no hash iteration, no unordered collection in construction).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitShapeKit {
    /// The plane × sphere unit shape.
    pub plane_sphere: PlaneSphereFixture,
    /// The plane × cylinder unit shape.
    pub plane_cylinder: PlaneCylinderFixture,
    /// The sweep × plane unit shape.
    pub sweep_plane: SweepPlaneFixture,
}

/// Builds the unit-shape kit once. Deterministic: every fixture is built from
/// ordered, dyadic data by fixed-order float reductions.
pub fn unit_shape_kit() -> Result<UnitShapeKit, ConstructRefusal> {
    Ok(UnitShapeKit {
        plane_sphere: plane_sphere_fixture(),
        plane_cylinder: plane_cylinder_fixture()?,
        sweep_plane: sweep_plane_fixture(),
    })
}

// ---------------------------------------------------------------------------
// Closed-form helpers shared by the fixture builders (no solver anywhere).
// ---------------------------------------------------------------------------

/// The plane `z = d`: origin `(0, 0, d)`, u-axis `+x`, v-axis `+y`, unit
/// normal `+z`.
fn plane_z(d: f64) -> Plane {
    Plane::new(
        Point3::new(0.0, 0.0, d),
        Point3::new(1.0, 0.0, d),
        Point3::new(0.0, 1.0, d),
    )
}

/// Builds a canonical cylinder, refusing a non-positive or non-finite radius
/// through the landed `Cylinder::new` gate (which cannot refuse a valid dyadic
/// radius; the `?` is for totality only).
fn cylinder_r(center: Point3, radius: f64) -> Result<Cylinder, ConstructRefusal> {
    match Cylinder::new(center, radius) {
        Ok(certified) => Ok(certified.value),
        Err(_) => Err(ConstructRefusal::InvalidInput),
    }
}

/// The plane × sphere section circle (closed form, fixture 1): returns the
/// centre `c − δ·n` and radius `sqrt(R² − δ²)` for `δ = (c − o)·n`.
fn sphere_plane_section_circle(sphere: &Sphere, plane: &Plane) -> (Point3, f64) {
    let n = plane.normal();
    let delta = (sphere.center() - plane.origin()).dot(n);
    let centre = sphere.center() - delta * n;
    let radius = (sphere.radius() * sphere.radius() - delta * delta).sqrt();
    (centre, radius)
}

/// The station `s*` at which the plane `n · x = d` meets the straight spine of
/// the unit sweep: solving `n · C(s*) = d` for the affine spine `C(s)`.
fn station_of_plane(sweep: &ScaleCircleSweepUnit, plane: &Plane) -> f64 {
    let n = plane.normal();
    let o = plane.origin();
    let direction = sweep.spine_to - sweep.spine_from;
    let numerator = (o - sweep.spine_from).dot(n);
    let denominator = direction.dot(n);
    sweep.s0 + numerator / denominator * (sweep.s1 - sweep.s0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit-scale position tolerance for the closed-form machine checks
    /// (fixtures are unit-scale dyadic geometry).
    const POS_TOL: f64 = 1.0e-9; // H-3: unit-scale position tolerance for closed-form ground-truth checks

    /// Test-only unwrap stand-in: asserts the option holds, then returns it.
    /// The `None` arm is the divergent tail the H-1 test files use.
    fn expect_some<T>(option: Option<T>, what: &str) -> Option<T> {
        assert!(option.is_some(), "{what}");
        option
    }

    /// The point payload of a point-valued certificate (asserts the kind).
    fn expect_point(certificate: CertificateValue, what: &str) -> Option<Point3> {
        expect_some(certificate.point_value(), what)
    }

    /// The scalar payload of a scalar-valued certificate (asserts the kind).
    fn expect_scalar(certificate: CertificateValue, what: &str) -> Option<f64> {
        expect_some(certificate.scalar_value(), what)
    }

    /// Machine-check: two points agree within `tol`.
    fn assert_point_close(a: Point3, b: Point3, tol: f64, what: &str) {
        assert!(
            (a - b).magnitude() <= tol,
            "{what}: {a:?} diverged from {b:?} by {}",
            (a - b).magnitude()
        );
    }

    /// Machine-check: two scalars agree within `tol`.
    fn assert_scalar_close(a: f64, b: f64, tol: f64, what: &str) {
        assert!(
            (a - b).abs() <= tol,
            "{what}: {a} diverged from {b} by {}",
            (a - b).abs()
        );
    }

    #[test]
    fn fixture_plane_sphere_ground_truth() {
        let fixture = plane_sphere_fixture();
        let plane = fixture.plane;
        let sphere = fixture.sphere;
        let n = plane.normal();
        let o = plane.origin();
        let c = sphere.center();
        let radius_sphere = sphere.radius();

        // Closed form (fixture doc): δ = (c − o)·n, centre c′ = c − δ·n,
        // radius ρ = sqrt(R² − δ²).
        let delta = (c - o).dot(n);
        let centre = c - delta * n;
        let radius = (radius_sphere * radius_sphere - delta * delta).sqrt();

        // The stored ground truth matches the closed form...
        let stored_centre = match expect_point(fixture.centre, "the section centre is a point") {
            Some(centre) => centre,
            None => return,
        };
        let stored_radius = match expect_scalar(fixture.radius, "the section radius is a scalar") {
            Some(radius) => radius,
            None => return,
        };
        assert_point_close(stored_centre, centre, POS_TOL, "stored section centre");
        assert_scalar_close(stored_radius, radius, POS_TOL, "stored section radius");
        // ...and the derivation itself is concrete: δ = −1, c′ = (0,0,1),
        // ρ = sqrt(3).
        assert_scalar_close(
            delta,
            -1.0,
            POS_TOL,
            "signed distance of the centre to the plane",
        );
        assert_point_close(
            centre,
            Point3::new(0.0, 0.0, 1.0),
            POS_TOL,
            "closed-form section centre",
        );
        assert_scalar_close(
            radius,
            3.0_f64.sqrt(),
            POS_TOL,
            "closed-form section radius",
        );

        // Carrier machine check: trace the section circle in the plane's own
        // unit basis and verify every traced point lies on BOTH carriers —
        // in the plane and at distance R from the sphere centre.
        let u_axis = plane.u_axis().normalize();
        let v_axis = plane.v_axis().normalize();
        const SAMPLES: usize = 16;
        for i in 0..SAMPLES {
            let angle = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64);
            let (sine, cosine) = angle.sin_cos();
            let traced = centre + radius * (cosine * u_axis + sine * v_axis);
            let on_plane = (traced - o).dot(n);
            let on_sphere = (traced - c).magnitude() - radius_sphere;
            assert!(
                on_plane.abs() <= POS_TOL && on_sphere.abs() <= POS_TOL,
                "traced circle point escaped a carrier at angle {angle}: \
                 plane residual {on_plane}, sphere residual {on_sphere}"
            );
        }
    }

    #[test]
    fn fixture_plane_cylinder_ground_truth() {
        let built = plane_cylinder_fixture();
        assert!(
            built.is_ok(),
            "the plane × cylinder fixture refused unexpectedly"
        );
        let fixture = match built {
            Ok(fixture) => fixture,
            Err(_) => return,
        };
        let plane = fixture.plane;
        let cylinder = fixture.cylinder;
        let n = plane.normal();
        let o = plane.origin();
        let radius_cyl = cylinder.radius();
        let axis = Vector3::unit_z();

        // Closed form (fixture doc): |sin θ| = |n · ẑ|, semi-major r/|sin θ|
        // along the plane u-axis, semi-minor r along the plane v-axis.
        let sin_theta = n.dot(axis).abs();
        let semi_major = radius_cyl / sin_theta;
        let semi_minor = radius_cyl;

        let stored_sin_theta =
            match expect_scalar(fixture.sin_theta, "the incidence sine is a scalar") {
                Some(value) => value,
                None => return,
            };
        let stored_major =
            match expect_scalar(fixture.semi_major, "the semi-major axis is a scalar") {
                Some(value) => value,
                None => return,
            };
        let stored_minor =
            match expect_scalar(fixture.semi_minor, "the semi-minor axis is a scalar") {
                Some(value) => value,
                None => return,
            };
        assert_scalar_close(stored_sin_theta, sin_theta, POS_TOL, "stored |sin θ|");
        assert_scalar_close(stored_major, semi_major, POS_TOL, "stored semi-major axis");
        assert_scalar_close(stored_minor, semi_minor, POS_TOL, "stored semi-minor axis");

        // The concrete derivation: r = 2, |sin θ| = 1/2 → a = 4, b = 2,
        // centred at the origin (axis ∩ plane).
        assert_scalar_close(
            sin_theta,
            0.5,
            POS_TOL,
            "incidence sine of the fixture plane",
        );
        assert_scalar_close(semi_major, 4.0, POS_TOL, "closed-form semi-major axis");
        assert_scalar_close(semi_minor, 2.0, POS_TOL, "closed-form semi-minor axis");

        // Carrier machine check: trace the ellipse in the plane's unit basis
        // (major along the plane u-axis, minor along the v-axis) and verify
        // every traced point lies on BOTH carriers — in the plane and at
        // distance r from the cylinder axis.
        let u_axis = plane.u_axis().normalize();
        let v_axis = plane.v_axis().normalize();
        const SAMPLES: usize = 16;
        for i in 0..SAMPLES {
            let angle = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64);
            let (sine, cosine) = angle.sin_cos();
            let traced = o + semi_major * cosine * u_axis + semi_minor * sine * v_axis;
            let on_plane = (traced - o).dot(n);
            let offset = traced - cylinder.center();
            let radial = (offset - offset.dot(axis) * axis).magnitude();
            let on_cylinder = radial - radius_cyl;
            assert!(
                on_plane.abs() <= POS_TOL && on_cylinder.abs() <= POS_TOL,
                "traced ellipse point escaped a carrier at angle {angle}: \
                 plane residual {on_plane}, cylinder residual {on_cylinder}"
            );
        }
    }

    #[test]
    fn fixture_sweep_plane_ground_truth() {
        let fixture = sweep_plane_fixture();
        let sweep = fixture.sweep;
        let plane = fixture.plane;
        let n = plane.normal();
        let o = plane.origin();

        // Derive s* from the plane equation and the spine parameterization
        // (the algebra, from the fixture doc). The straight spine is
        // C(s) = from + s·(to − from) with s ∈ [0, 1]; here from = (0,0,0),
        // to = (0,0,1), so C(s) = (0,0,s). The plane is n·x = d with n = +z,
        // o = (0,0,d), d = 3/4. Solving n·C(s*) = d:
        //
        //     n·(from + s*·(to − from)) = d
        //     s*·(n·(to − from)) = d − n·from
        //     s* = (d − 0) / (n·(0,0,1)) = (3/4) / 1 = 3/4
        let direction = sweep.spine_to - sweep.spine_from;
        let station = (o - sweep.spine_from).dot(n) / direction.dot(n);
        // Radius at the station follows the linear scale law:
        // radius(s) = radius_start + (radius_end − radius_start)·s.
        let section_radius = sweep.radius_at(station);
        let section_centre = sweep.spine_point(station);

        let stored_station = match expect_scalar(fixture.station, "the station is a scalar") {
            Some(value) => value,
            None => return,
        };
        let stored_centre = match expect_point(fixture.centre, "the section centre is a point") {
            Some(centre) => centre,
            None => return,
        };
        let stored_radius = match expect_scalar(fixture.radius, "the section radius is a scalar") {
            Some(radius) => radius,
            None => return,
        };
        assert_scalar_close(stored_station, station, POS_TOL, "stored station s*");
        assert_point_close(
            stored_centre,
            section_centre,
            POS_TOL,
            "stored section centre",
        );
        assert_scalar_close(
            stored_radius,
            section_radius,
            POS_TOL,
            "stored section radius",
        );

        // Concrete derivation: s* = 3/4, centre C(s*) = (0,0,3/4),
        // radius = 1 + (1/2 − 1)·(3/4) = 5/8.
        assert_scalar_close(station, 0.75, POS_TOL, "derived station s*");
        assert_point_close(
            section_centre,
            Point3::new(0.0, 0.0, 0.75),
            POS_TOL,
            "derived section centre",
        );
        assert_scalar_close(section_radius, 0.625, POS_TOL, "derived section radius");

        // Carrier machine check: the sweep surface is the set of points whose
        // radial distance from the spine equals radius(z) at height z = s
        // (the straight spine runs along z, the ring is perpendicular). The
        // plane z = d therefore meets the surface exactly at the ring s*:
        //
        //     section = { q : q·ẑ = d ∧ |q − (q·ẑ)ẑ| = radius(d) }
        //
        // Trace that circle and verify each traced point satisfies the sweep
        // radial profile; verify a ring point at any other station leaves the
        // plane by the predicted signed distance.
        const SAMPLES: usize = 16;
        for i in 0..SAMPLES {
            let angle = std::f64::consts::TAU * (i as f64) / (SAMPLES as f64);
            let (sine, cosine) = angle.sin_cos();
            let traced = section_centre + section_radius * Vector3::new(cosine, sine, 0.0);
            let on_plane = (traced - o).dot(n);
            let offset = traced - sweep.spine_point(0.0);
            let radial = (offset - offset.dot(axis_z()) * axis_z()).magnitude();
            let profile = sweep.radius_at(traced.z);
            assert!(
                on_plane.abs() <= POS_TOL && (radial - profile).abs() <= POS_TOL,
                "traced section point escaped at angle {angle}: plane residual \
                 {on_plane}, radial {radial} vs profile {profile}"
            );

            // A ring point at station s′ ≠ s* is off the plane by (s′ − s*):
            // the plane contains exactly the station the algebra selects.
            let other_station = 0.25_f64;
            let v = 0.5_f64;
            let other = sweep.point(other_station, v);
            let residual = (other - o).dot(n);
            assert!(
                (residual - (other_station - station)).abs() <= POS_TOL,
                "a non-section station left the plane by {residual}, expected \
                 {other_station} − {station}"
            );
        }
    }

    /// The z-axis unit direction (the sweep spine direction of this unit
    /// shape).
    fn axis_z() -> Vector3 {
        Vector3::unit_z()
    }

    #[test]
    fn fixture_kit_is_deterministic() {
        let first = unit_shape_kit();
        let second = unit_shape_kit();
        assert!(first.is_ok(), "the first kit construction refused");
        assert!(second.is_ok(), "the second kit construction refused");
        let first = match first {
            Ok(kit) => kit,
            Err(_) => return,
        };
        let second = match second {
            Ok(kit) => kit,
            Err(_) => return,
        };
        // Identical ordered input → identical values: no hash iteration, no
        // unordered collection in construction.
        assert_eq!(
            first.plane_sphere.centre, second.plane_sphere.centre,
            "plane × sphere centre is not deterministic"
        );
        assert_eq!(
            first.plane_sphere.radius, second.plane_sphere.radius,
            "plane × sphere radius is not deterministic"
        );
        assert_eq!(
            first.plane_cylinder.semi_major, second.plane_cylinder.semi_major,
            "plane × cylinder semi-major axis is not deterministic"
        );
        assert_eq!(
            first.sweep_plane.station, second.sweep_plane.station,
            "sweep × plane station is not deterministic"
        );
        assert_eq!(
            first.sweep_plane.centre, second.sweep_plane.centre,
            "sweep × plane centre is not deterministic"
        );
        assert_eq!(
            first.sweep_plane.radius, second.sweep_plane.radius,
            "sweep × plane radius is not deterministic"
        );
        assert_eq!(first, second, "two kit constructions must compare equal");
    }
}
