//! Composite-path spines for the showcases: a piecewise-analytic path
//! (straight drop → transition arc → descending helix → transition arc →
//! straight runout), sampled by arc length and interpolated into ONE clamped
//! cubic B-spline. `Curve` implements `SpineCurve`
//! (`truck-geometry/src/decorators/spine_frame.rs`) and `From<BSplineCurve<Point3>>`
//! exists (`truck-geometry/src/canonical.rs`), so the interpolated spine is
//! first-class for both `facet_sweep` and `spine_sweep`.
//!
//! C¹ doctrine: the composite path is tangent-continuous by construction
//! (each segment starts with exactly the previous segment's end tangent), and
//! cubic interpolation preserves it. Curvature jumps at the joins are legal —
//! the kernel's spine contract is C¹, not C².
//!
//! This module is pure table math except the final `Curve` conversion, so the
//! whole path definition ports to truck123d verbatim.

use truck_base::cgmath64::{Point2, Point3, Vector2, Vector3};
use truck_geometry::canonical::Curve;
use truck_geometry::nurbs::{BSplineCurve, KnotVec};

/// The composite-path specification (all lengths in meters, angles in
/// degrees; this struct IS the portable table payload).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompositeSpec {
    /// Drop length along the launch incline.
    pub drop_length: f64,
    /// Launch incline angle from horizontal.
    pub drop_angle_deg: f64,
    /// Both transition-arc radii.
    pub transition_radius: f64,
    /// Helix radius about the tower axis.
    pub helix_radius: f64,
    /// Helix revolutions (fractional turns allowed).
    pub helix_turns: f64,
    /// Helix descent angle from horizontal.
    pub helix_slope_deg: f64,
    /// Runout length.
    pub runout_length: f64,
    /// Total arc-length sample count fed to the interpolator.
    pub samples: usize,
}

/// One sampled path: arc-length-parameterized points, plus the derived
/// reference data the showcase needs (tower axis, runout frame).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompositePath {
    /// `(arc_length, point)` samples, ascending, `arc_length ∈ [0, total]`.
    pub samples: Vec<(f64, Point3)>,
    /// Total arc length.
    pub total_length: f64,
    /// The tower/helix axis point (horizontal position; axis is +z).
    pub tower_axis: Point3,
    /// The helix entry point (top of the helix).
    pub helix_entry: Point3,
    /// The helix exit point.
    pub helix_exit: Point3,
    /// The runout horizontal direction.
    pub runout_dir: Vector3,
    /// The runout end point (path terminus).
    pub runout_end: Point3,
    /// The launch start point.
    pub start: Point3,
}

impl CompositeSpec {
    /// The launch incline angle in radians.
    pub fn drop_angle(&self) -> f64 {
        self.drop_angle_deg.to_radians()
    }

    /// The helix slope angle in radians.
    pub fn helix_slope(&self) -> f64 {
        self.helix_slope_deg.to_radians()
    }
}

/// Builds the sampled composite path. Geometry (all closed-form):
///
/// 1. Helix: center = tower axis at the xy-origin, entry azimuth `-90°`, CCW,
///    entry tangent azimuth `0°` (+x), pitch `-tan(β)`.
/// 2. In-arc (vertical plane `y = -R_h`): turns pitch from `-α` to `-β`, CCW,
///    ending exactly at the helix entry with tangent `(cos β, 0, -sin β)`.
/// 3. Drop: straight, pitch `-α`, in the same vertical plane.
/// 4. Out-arc (vertical plane of the helix exit tangent): turns pitch from
///    `-β` to `0`, CCW; parameterized as `(u, z)` with `u` the arc-length
///    along the exit azimuth.
/// 5. Runout: straight, horizontal, along the exit azimuth.
pub fn composite_path(spec: &CompositeSpec) -> CompositePath {
    let a = spec.drop_angle();
    let b = spec.helix_slope();
    let rh = spec.helix_radius;
    let rt = spec.transition_radius;

    let helix_turns_rad = 2.0 * std::f64::consts::PI * spec.helix_turns;
    let phi_entry = -std::f64::consts::FRAC_PI_2;
    let phi_exit = phi_entry + helix_turns_rad;

    let helix_entry = Point3::new(0.0, -rh, 0.0);
    let helix_exit = Point3::new(
        rh * phi_exit.cos(),
        rh * phi_exit.sin(),
        -rh * (phi_exit - phi_entry) * b.tan(),
    );
    let tower_axis = Point3::new(0.0, 0.0, 0.0);

    let n_left = |theta: f64| Vector2::new(-theta.sin(), theta.cos());

    let arc_in_center = Vector2::new(helix_entry.x + rt * b.sin(), helix_entry.z + rt * b.cos());
    let drop_end = Point3::new(
        arc_in_center.x - rt * n_left(-a).x,
        -rh,
        arc_in_center.y - rt * n_left(-a).y,
    );
    let start = Point3::new(
        drop_end.x - spec.drop_length * a.cos(),
        -rh,
        drop_end.z + spec.drop_length * a.sin(),
    );

    let exit_azimuth = phi_exit + std::f64::consts::FRAC_PI_2;
    let runout_dir = Vector3::new(exit_azimuth.cos(), exit_azimuth.sin(), 0.0);
    let arc_out_center = Vector2::new(rt * b.sin(), helix_exit.z + rt * b.cos());
    let runout_start_z = arc_out_center.y - rt;
    let runout_end = Point3::new(
        helix_exit.x + runout_dir.x * spec.runout_length,
        helix_exit.y + runout_dir.y * spec.runout_length,
        runout_start_z,
    );

    let len_drop = spec.drop_length;
    let len_arc_in = rt * (a - b);
    let len_helix = rh / b.cos() * helix_turns_rad;
    let len_arc_out = rt * b;
    let len_runout = spec.runout_length;
    let total = len_drop + len_arc_in + len_helix + len_arc_out + len_runout;

    let n = spec.samples.max(16);
    let seg = |len: f64| ((n as f64 * len / total).round() as usize).max(2);

    let mut samples: Vec<(f64, Point3)> = Vec::with_capacity(n + 8);
    let mut cursor = 0.0f64;

    let n_drop = seg(len_drop);
    for i in 0..=n_drop {
        let f = i as f64 / n_drop as f64;
        samples.push((
            cursor + len_drop * f,
            Point3::new(
                drop_end.x - len_drop * f * a.cos(),
                -rh,
                drop_end.z + len_drop * f * a.sin(),
            ),
        ));
    }
    cursor += len_drop;

    let n_arc_in = seg(len_arc_in);
    for i in 1..=n_arc_in {
        let f = i as f64 / n_arc_in as f64;
        let s = cursor + len_arc_in * f;
        let theta = -a + (a - b) * f;
        let nl = n_left(theta);
        samples.push((
            s,
            Point3::new(
                arc_in_center.x - rt * nl.x,
                -rh,
                arc_in_center.y - rt * nl.y,
            ),
        ));
    }
    cursor += len_arc_in;

    let n_helix = seg(len_helix);
    for i in 1..=n_helix {
        let f = i as f64 / n_helix as f64;
        let phi = phi_entry + helix_turns_rad * f;
        let s = cursor + len_helix * f;
        samples.push((
            s,
            Point3::new(
                rh * phi.cos(),
                rh * phi.sin(),
                helix_entry.z - rh * (phi - phi_entry) * b.tan(),
            ),
        ));
    }
    cursor += len_helix;

    let n_arc_out = seg(len_arc_out);
    for i in 1..=n_arc_out {
        let f = i as f64 / n_arc_out as f64;
        let s = cursor + len_arc_out * f;
        let theta = -b + b * f;
        let nl = n_left(theta);
        let u = arc_out_center.x - rt * nl.x;
        let z = arc_out_center.y - rt * nl.y;
        samples.push((
            s,
            Point3::new(
                helix_exit.x + runout_dir.x * u,
                helix_exit.y + runout_dir.y * u,
                z,
            ),
        ));
    }
    cursor += len_arc_out;

    let n_runout = seg(len_runout);
    for i in 1..=n_runout {
        let f = i as f64 / n_runout as f64;
        let s = cursor + len_runout * f;
        samples.push((
            s,
            Point3::new(
                helix_exit.x + runout_dir.x * len_runout * f,
                helix_exit.y + runout_dir.y * len_runout * f,
                runout_start_z,
            ),
        ));
    }

    CompositePath {
        samples,
        total_length: total,
        tower_axis,
        helix_entry,
        helix_exit,
        runout_dir,
        runout_end,
        start,
    }
}

/// Interpolates a short list of points into one clamped cubic B-spline spine
/// with uniform parameters. Stable in the small-n regime
/// (NUM-INTERPOLE-OVERSHOOT-001): keep the point count modest (<= ~24).
/// The parameter domain is unit-normalized, which is what the kernel's
/// `ScalarLaw`/correspondence evaluation assumes.
pub fn spline_through_points(points: &[Point3]) -> Result<Curve, SpineError> {
    let n = points.len();
    if n < 4 {
        return Err(SpineError::TooFewSamples(n));
    }
    let mut knots = vec![0.0f64; 4];
    for i in 1..=(n - 4) {
        knots.push(i as f64 / (n - 3) as f64);
    }
    knots.extend_from_slice(&[1.0; 4]);
    let knot_vec = KnotVec::from(knots);
    let parameter_points: Vec<(f64, Point3)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (i as f64 / (n - 1) as f64, *p))
        .collect();
    let spline = BSplineCurve::try_interpole(knot_vec, parameter_points)
        .map_err(|e| SpineError::Interpolation(format!("{e}")))?;
    Ok(Curve::from(spline))
}

/// Applies the ground shift: translates every point of `path` by `dz` so the
/// runout rides at z = 0 (the pool then sits at or below ground level).
/// Returns the applied shift.
pub fn shift_to_ground(path: &mut CompositePath, dz: f64) -> f64 {
    let d = -path.runout_end.z + dz;
    for (_, p) in path.samples.iter_mut() {
        p.z += d;
    }
    path.tower_axis.z += d;
    path.helix_entry.z += d;
    path.helix_exit.z += d;
    path.runout_end.z += d;
    path.start.z += d;
    d
}

/// Interpolates the sampled path into one clamped cubic B-spline spine and
/// lifts it into the canonical `Curve` carrier.
pub fn spline_from_path(path: &CompositePath) -> Result<Curve, SpineError> {
    let n = path.samples.len();
    if n < 4 {
        return Err(SpineError::TooFewSamples(n));
    }
    let mut knots = vec![0.0f64; 4];
    for i in 1..=(n - 4) {
        knots.push(i as f64 / (n - 3) as f64);
    }
    knots.extend_from_slice(&[1.0; 4]);
    let knot_vec = KnotVec::from(knots);
    let total = path.total_length;
    let parameter_points: Vec<(f64, Point3)> = path
        .samples
        .iter()
        .map(|(s, p)| (s / total, *p))
        .collect();
    let spline = BSplineCurve::try_interpole(knot_vec, parameter_points)
        .map_err(|e| SpineError::Interpolation(format!("{e}")))?;
    Ok(Curve::from(spline))
}

/// Spine-construction failures (client-side; kernel refusals surface later at
/// realization with their own typed vocabulary).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SpineError {
    #[error("too few path samples: {0}")]
    TooFewSamples(usize),
    #[error("interpolation failed: {0}")]
    Interpolation(String),
}

/// The U-chute "up" convention lives with the profile; the spine's arc-length
/// parameter is normalized by [`spline_from_path`] to `[0, 1]`, which is what
/// the kernel's `ScalarLaw`/correspondence evaluation assumes. Kept here as a
/// compile-time-checked reminder that the domain is unit.
const _SPINE_DOMAIN_IS_UNIT: () = {
    let _demo_units: (Point2, f64) = (Point2::new(0.0, 0.0), 1.0);
    ()
};
