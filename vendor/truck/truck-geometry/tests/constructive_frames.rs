#![deny(clippy::unwrap_used)]

//! BG-CG-002-FRAMES-ANALYTIC — the three analytic frame laws: behavior tests
//! for `FixedPlane`, `ArchitecturalUp`, and `RadialAboutAxis` through the
//! recipe dispatcher, plus the refused `ParallelTransport` envelope line.

use truck_base::tolerance::TOLERANCE;
use truck_geometry::base::*;
use truck_geometry::constructive::*;

/// The unit-circle arc spine about the Z axis (r2): `C(s) = (cos θ, sin θ, 0)`
/// with `θ = phi0 + s · delta`, `s ∈ [0, 1]`. The analytic tangent is
/// `delta · (−sin θ, cos θ, 0)`.
#[derive(Debug, Clone, Copy)]
struct CircleSpine {
    phi0: f64,
    delta: f64,
}

impl Spine for CircleSpine {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Point3::new(theta.cos(), theta.sin(), 0.0))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Vector3::new(
            -self.delta * theta.sin(),
            self.delta * theta.cos(),
            0.0,
        ))
    }
}

/// The same circle arc with every spine point rotated `rot` radians about the
/// Z axis. The axis itself is unchanged.
#[derive(Debug, Clone, Copy)]
struct RotatedSpine {
    base: CircleSpine,
    rot: f64,
}

impl Spine for RotatedSpine {
    fn domain(&self) -> (f64, f64) {
        self.base.domain()
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        let p = self.base.position_at(s)?;
        let v = rotated_about_z(Vector3::new(p.x, p.y, p.z), self.rot);
        Ok(Point3::new(v.x, v.y, v.z))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        let d = self.base.derivative_at(s)?;
        Ok(rotated_about_z(d, self.rot))
    }
}

/// Rotates a vector `rot` radians about the world Z axis.
fn rotated_about_z(v: Vector3, rot: f64) -> Vector3 {
    let (c, sn) = (rot.cos(), rot.sin());
    Vector3::new(v.x * c - v.y * sn, v.x * sn + v.y * c, v.z)
}

/// The Constant-profile triangle used by every frame fixture.
fn triangle() -> Profile2D {
    Profile2D {
        vertices: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ],
    }
}

#[test]
fn fixed_plane_frame_matches_spec_formula() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::FixedPlane {
            normal: Vector3::new(0.0, 1.0, 1.0),
        },
    );
    let t = Vector3::new(1.0, 0.0, 0.0);
    let b = Vector3::new(0.0, 1.0, 1.0).normalize();
    let n = b.cross(t);
    let tol = DirectTolerance::default().position;
    for s in [0.0, 0.5, 1.0] {
        let ok = match recipe.frame(s) {
            Ok(f) => {
                (f.tangent - t).magnitude() <= tol
                    && (f.binormal - b).magnitude() <= tol
                    && (f.normal - n).magnitude() <= tol
                    && (f.tangent.cross(f.normal) - f.binormal).magnitude() <= tol
            }
            Err(_) => false,
        };
        assert!(ok, "frame({s}) does not match the FixedPlane spec formula");
    }
}

#[test]
fn fixed_plane_refuses_zero_tangent() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(1.0, 2.0, 3.0),
            end: Point3::new(1.0, 2.0, 3.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::FixedPlane {
            normal: Vector3::unit_z(),
        },
    );
    for s in [0.0, 0.5, 1.0] {
        assert!(matches!(
            recipe.frame(s),
            Err(ConstructError::ZeroTangent { at }) if at == s
        ));
    }
}

#[test]
fn fixed_plane_refuses_degenerate_normal() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::FixedPlane {
            normal: Vector3::zero(),
        },
    );
    assert!(matches!(
        recipe.frame(0.5),
        Err(ConstructError::FrameSingular {
            law: "FixedPlane",
            ..
        })
    ));
}

#[test]
fn architectural_up_matches_spec_formula() {
    let up = Vector3::unit_z();
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 1.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ArchitecturalUp { up },
    );
    let t = Vector3::new(1.0, 1.0, 0.0).normalize();
    let b = up.cross(t).normalize();
    let n = t.cross(b);
    let tol = DirectTolerance::default().position;
    for s in [0.0, 0.5, 1.0] {
        let ok = match recipe.frame(s) {
            Ok(f) => {
                (f.tangent - t).magnitude() <= tol
                    && (f.binormal - b).magnitude() <= tol
                    && (f.normal - n).magnitude() <= tol
            }
            Err(_) => false,
        };
        assert!(
            ok,
            "frame({s}) does not match the ArchitecturalUp spec formula"
        );
    }
}

#[test]
fn architectural_up_refuses_parallel_up() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(0.0, 0.0, 1.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ArchitecturalUp {
            up: Vector3::unit_z(),
        },
    );
    assert!(matches!(
        recipe.frame(0.5),
        Err(ConstructError::FrameSingular {
            law: "ArchitecturalUp",
            ..
        })
    ));
}

#[test]
fn radial_frame_matches_spec_formula() {
    let spine = CircleSpine {
        phi0: 0.0,
        delta: 1.0,
    };
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(triangle()),
        FrameLaw::RadialAboutAxis {
            origin: Point3::new(0.0, 0.0, 0.0),
            axis: Vector3::unit_z(),
        },
    );
    let tol = DirectTolerance::default().position;
    for s in [0.0, 0.5, 1.0] {
        let theta = spine.phi0 + s * spine.delta;
        let c = Point3::new(theta.cos(), theta.sin(), 0.0);
        let t = Vector3::new(-theta.sin(), theta.cos(), 0.0);
        let n = Vector3::new(theta.cos(), theta.sin(), 0.0);
        assert!(matches!(
            spine.position_at(s),
            Ok(p) if (p - c).magnitude() <= tol
        ));
        assert!(matches!(
            spine.derivative_at(s),
            Ok(d) if (d.normalize() - t).magnitude() <= tol
        ));
        let ok = match recipe.frame(s) {
            Ok(f) => {
                (f.tangent - t).magnitude() <= tol
                    && (f.normal - n).magnitude() <= tol
                    && (f.binormal - t.cross(n)).magnitude() <= tol
            }
            Err(_) => false,
        };
        assert!(
            ok,
            "frame({s}) does not match the RadialAboutAxis spec formula"
        );
    }
}

#[test]
fn radial_frame_refuses_axis_incident_point() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(-1.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::RadialAboutAxis {
            origin: Point3::new(0.0, 0.0, 0.0),
            axis: Vector3::unit_z(),
        },
    );
    let tol = DirectTolerance::default().position;
    assert!(matches!(
        recipe.spine.position_at(0.5),
        Ok(p) if (p - Point3::new(0.0, 0.0, 0.0)).magnitude() <= tol
    ));
    assert!(matches!(
        recipe.frame(0.5),
        Err(ConstructError::FrameSingular {
            law: "RadialAboutAxis",
            ..
        })
    ));
}

#[test]
fn parallel_transport_still_refuses_in_cg002() {
    // BG-CG-003-TRANSPORT: in-place amendment — the ParallelTransport law
    // landed; the name is historical (it pinned the CG-002 envelope line this
    // packet retires). The body is now the positive helix orthonormality form.
    #[derive(Debug, Clone, Copy)]
    struct HelixSpine {
        c: f64,
    }
    impl Spine for HelixSpine {
        fn domain(&self) -> (f64, f64) {
            (0.0, 1.0)
        }

        fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
            if !s.is_finite() {
                return Err(ConstructError::NonFinite { at: s });
            }
            if !(0.0..=1.0).contains(&s) {
                return Err(ConstructError::InvalidInput);
            }
            let theta = 2.0 * std::f64::consts::PI * s;
            Ok(Point3::new(theta.cos(), theta.sin(), self.c * theta))
        }

        fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
            if !s.is_finite() {
                return Err(ConstructError::NonFinite { at: s });
            }
            if !(0.0..=1.0).contains(&s) {
                return Err(ConstructError::InvalidInput);
            }
            let two_pi = 2.0 * std::f64::consts::PI;
            let theta = 2.0 * std::f64::consts::PI * s;
            Ok(Vector3::new(
                -two_pi * theta.sin(),
                two_pi * theta.cos(),
                two_pi * self.c,
            ))
        }
    }

    let recipe = SpineFrameRecipe::new(
        HelixSpine { c: 1.0 },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::unit_z(),
        },
    );
    let tol = DirectTolerance::default().position;
    for i in 0..=16 {
        let s = (i as f64) / 16.0;
        let ok = match recipe.frame(s) {
            Ok(f) => {
                (f.tangent.magnitude() - 1.0).abs() <= tol
                    && (f.normal.magnitude() - 1.0).abs() <= tol
                    && (f.binormal.magnitude() - 1.0).abs() <= tol
                    && f.tangent.dot(f.normal).abs() <= tol
                    && f.tangent.dot(f.binormal).abs() <= tol
                    && f.normal.dot(f.binormal).abs() <= tol
                    && (f.tangent.cross(f.normal) - f.binormal).magnitude() <= tol
            }
            Err(_) => false,
        };
        assert!(ok, "helix frame at s = {s} is not Ok and orthonormal");
    }
}

#[test]
fn radial_frame_is_equivariant_under_rotation() {
    let base = CircleSpine {
        phi0: 0.0,
        delta: 1.0,
    };
    let rot = std::f64::consts::FRAC_PI_2;
    let origin = Point3::new(0.0, 0.0, 0.0);
    let axis = Vector3::unit_z();
    let recipe = SpineFrameRecipe::new(
        base,
        ProfileLaw::Constant(triangle()),
        FrameLaw::RadialAboutAxis { origin, axis },
    );
    let rotated = SpineFrameRecipe::new(
        RotatedSpine { base, rot },
        ProfileLaw::Constant(triangle()),
        FrameLaw::RadialAboutAxis { origin, axis },
    );
    let tol = DirectTolerance::default().position;
    let half = 0.5f64;
    assert!(matches!(
        rotated.spine.position_at(0.5),
        Ok(p) if (p - Point3::new(-half.sin(), half.cos(), 0.0)).magnitude() <= tol
    ));
    let bound = 64.0 * TOLERANCE;
    for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let ok = match (recipe.frame(s), rotated.frame(s)) {
            (Ok(f), Ok(g)) => {
                (g.tangent - rotated_about_z(f.tangent, rot)).magnitude() <= bound
                    && (g.normal - rotated_about_z(f.normal, rot)).magnitude() <= bound
                    && (g.binormal - rotated_about_z(f.binormal, rot)).magnitude() <= bound
            }
            _ => false,
        };
        assert!(
            ok,
            "radial frame is not equivariant under the 90° rotation at s = {s}"
        );
    }
}
