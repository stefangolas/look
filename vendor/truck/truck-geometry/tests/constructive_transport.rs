#![deny(clippy::unwrap_used)]

//! BG-CG-003-TRANSPORT — the parallel-transport (Bishop, double-reflection)
//! frame law: behavior tests through the recipe dispatcher.

use truck_base::tolerance::TOLERANCE;
use truck_geometry::base::*;
use truck_geometry::constructive::*;

/// The unit-circle arc spine about the Z axis: `C(s) = (cos θ, sin θ, 0)` with
/// `θ = phi0 + s · delta`, `s ∈ [0, 1]`. Closed when `delta = 2π`.
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
        if !(0.0..=1.0).contains(&s) {
            return Err(ConstructError::InvalidInput);
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Point3::new(theta.cos(), theta.sin(), 0.0))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        if !(0.0..=1.0).contains(&s) {
            return Err(ConstructError::InvalidInput);
        }
        let theta = self.phi0 + s * self.delta;
        Ok(Vector3::new(
            -self.delta * theta.sin(),
            self.delta * theta.cos(),
            0.0,
        ))
    }
}

/// The S-shaped spine: an upper semicircle about the origin joined C¹ to a
/// lower semicircle about `(-2, 0, 0)`. Opposite curvature with a continuous
/// tangent at the join at `s = 0.5`.
#[derive(Debug, Clone, Copy)]
struct SSpine;

impl Spine for SSpine {
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
        if s < 0.5 {
            let theta = 2.0 * std::f64::consts::PI * s;
            Ok(Point3::new(theta.cos(), theta.sin(), 0.0))
        } else {
            let u = 2.0 * s - 1.0;
            Ok(Point3::new(
                -2.0 + (std::f64::consts::PI * (-u)).cos(),
                (std::f64::consts::PI * (-u)).sin(),
                0.0,
            ))
        }
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        if !s.is_finite() {
            return Err(ConstructError::NonFinite { at: s });
        }
        if !(0.0..=1.0).contains(&s) {
            return Err(ConstructError::InvalidInput);
        }
        let two_pi = 2.0 * std::f64::consts::PI;
        if s < 0.5 {
            let theta = 2.0 * std::f64::consts::PI * s;
            Ok(Vector3::new(
                -two_pi * theta.sin(),
                two_pi * theta.cos(),
                0.0,
            ))
        } else {
            let u = 2.0 * s - 1.0;
            Ok(Vector3::new(
                two_pi * (std::f64::consts::PI * (-u)).sin(),
                -two_pi * (std::f64::consts::PI * (-u)).cos(),
                0.0,
            ))
        }
    }
}

/// The helix spine `C(s) = (cos θ, sin θ, c · θ)` with `θ = 2π s`, `s ∈ [0, 1]`.
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

/// The Constant-profile triangle used by every transport fixture.
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
fn transport_starts_from_orthonormalized_initial_normal() {
    let initial_normal = Vector3::new(1.0, 1.0, 0.5);
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport { initial_normal },
    );
    let t0 = Vector3::new(1.0, 0.0, 0.0);
    let tol = DirectTolerance::default().position;
    let expected = (initial_normal - initial_normal.dot(t0) * t0).normalize();
    let ok = match recipe.frame(0.0) {
        Ok(f) => {
            (f.normal - expected).magnitude() <= tol
                && (f.normal.magnitude() - 1.0).abs() <= tol
                && f.normal.dot(f.tangent).abs() <= tol
                && (f.tangent.cross(f.normal) - f.binormal).magnitude() <= tol
        }
        Err(_) => false,
    };
    assert!(
        ok,
        "frame(0.0) is not the orthonormalized initial normal frame"
    );
}

#[test]
fn straight_spine_has_constant_frame() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::unit_z(),
        },
    );
    let tol = DirectTolerance::default().position;
    let mut constant = true;
    for &s in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        let ok = match (recipe.frame(0.0), recipe.frame(s)) {
            (Ok(base), Ok(frame)) => {
                (frame.tangent - base.tangent).magnitude() <= tol
                    && (frame.normal - base.normal).magnitude() <= tol
                    && (frame.binormal - base.binormal).magnitude() <= tol
            }
            _ => false,
        };
        constant = constant && ok;
    }
    assert!(
        constant,
        "straight-spine frame is not constant along the spine"
    );
}

#[test]
fn circular_loop_has_trivial_holonomy() {
    let spine = CircleSpine {
        phi0: 0.0,
        delta: 2.0 * std::f64::consts::PI,
    };
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::unit_z(),
        },
    );
    let tol = DirectTolerance::default().position;
    let closed = match (spine.position_at(0.0), spine.position_at(1.0)) {
        (Ok(start), Ok(end)) => (end - start).magnitude() <= tol,
        _ => false,
    };
    assert!(closed, "circle fixture premise: the loop does not close");
    let bound = 64.0 * TOLERANCE;
    let ok = match (recipe.frame(0.0), recipe.frame(1.0)) {
        (Ok(start), Ok(end)) => {
            (end.tangent - start.tangent).magnitude() <= bound
                && (end.normal - start.normal).magnitude() <= bound
                && (end.binormal - start.binormal).magnitude() <= bound
        }
        _ => false,
    };
    assert!(
        ok,
        "closed planar circle does not have trivial frame holonomy"
    );
}

#[test]
fn frame_is_evaluation_order_independent() {
    let recipe = SpineFrameRecipe::new(
        HelixSpine { c: 1.0 },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::unit_z(),
        },
    );

    // Order A: frame(0.9) then frame(0.3) on one recipe.
    match recipe.frame(0.9) {
        Ok(_) => {}
        Err(_) => return,
    }
    let answer = match recipe.frame(0.3) {
        Ok(frame) => frame,
        Err(_) => return,
    };

    // Order B: frame(0.3) then frame(0.9) on a clone with identical fields.
    let reordered = recipe.clone();
    let answer_reordered = match reordered.frame(0.3) {
        Ok(frame) => frame,
        Err(_) => return,
    };
    match reordered.frame(0.9) {
        Ok(_) => {}
        Err(_) => return,
    }

    // Order C: fifty intermediate queries before frame(0.3).
    for i in 1..=50 {
        match recipe.frame((i as f64) / 51.0) {
            Ok(_) => {}
            Err(_) => return,
        }
    }
    let answer_interleaved = match recipe.frame(0.3) {
        Ok(frame) => frame,
        Err(_) => return,
    };

    assert_eq!(answer, answer_reordered);
    assert_eq!(answer, answer_interleaved);
}

#[test]
fn s_spine_survives_inflection() {
    let spine = SSpine;
    let recipe = SpineFrameRecipe::new(
        spine,
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::new(1.0, 0.0, 0.0),
        },
    );
    let tol = DirectTolerance::default().position;

    let join_ok = match spine.position_at(0.5) {
        Ok(p) => (p - Point3::new(-1.0, 0.0, 0.0)).magnitude() <= tol,
        Err(_) => false,
    };
    assert!(
        join_ok,
        "S-spine fixture premise: the join is not at (-1, 0, 0)"
    );
    let c1_ok = match spine.derivative_at(0.5) {
        Ok(d) => (d - Vector3::new(0.0, -2.0 * std::f64::consts::PI, 0.0)).magnitude() <= tol,
        Err(_) => false,
    };
    assert!(
        c1_ok,
        "S-spine fixture premise: the join tangent is not continuous"
    );

    let mut orthonormal = true;
    for &s in &[0.0, 0.25, 0.5, 0.75, 1.0] {
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
        orthonormal = orthonormal && ok;
    }
    assert!(
        orthonormal,
        "S-spine frame is not Ok and orthonormal at the queried stations"
    );

    let before = 0.5 - 1.0 / 64.0;
    let variation = match (recipe.frame(before), recipe.frame(0.5)) {
        (Ok(a), Ok(b)) => a.normal.angle(b.normal).0,
        _ => std::f64::consts::PI,
    };
    assert!(
        variation < 0.5,
        "S-spine normal flips across the inflection (variation = {variation} rad)"
    );
}

#[test]
fn parallel_initial_normal_is_singular() {
    let recipe = SpineFrameRecipe::new(
        LineSpine {
            start: Point3::new(0.0, 0.0, 0.0),
            end: Point3::new(1.0, 0.0, 0.0),
        },
        ProfileLaw::Constant(triangle()),
        FrameLaw::ParallelTransport {
            initial_normal: Vector3::new(1.0, 0.0, 0.0),
        },
    );
    assert!(matches!(
        recipe.frame(0.5),
        Err(ConstructError::FrameSingular {
            law: "ParallelTransport",
            ..
        })
    ));
}
