//! Targeted linear-plus-angular meshing policy for common analytic STEP
//! geometry.
//!
//! [`parse_step`](super::parse_step) used to hand Truck a single scalar chord
//! tolerance, `0.001 × model diagonal`, and Truck subdivided every curve and
//! surface against it. That value is a *geometric-error* bound: it controls
//! chord/sagitta deviation. On a thin, multiscale part it is set by the scene
//! diagonal rather than by any local feature, so a common small hole whose
//! radius is a tiny fraction of the diagonal receives only a handful of
//! segments and renders as a coarse polygon. Tightening the scalar globally
//! refines large, already-adequate features far more than necessary.
//!
//! This module replaces the bare scalar with an explicit policy. The linear
//! tolerance is retained as the geometric-error bound and as protection for
//! large-radius curves; a new *angular floor* — a maximum circumferential
//! angular interval, equivalently a minimum segment count per revolution — is
//! the mechanism that stops small circles from collapsing.
//!
//! The policy is applied per feature, not globally: see
//! [`crate::step::policy_geometry`], which wraps each STEP edge curve and
//! surface so that only circular curves and cylindrical/conical circumferential
//! directions see the angular floor. Straight edges, axial directions, and
//! non-elementary surfaces keep their existing treatment.

use serde::{Deserialize, Serialize};

/// Truck's own hard minimum on a subdivision tolerance. `parameter_division`
/// panics below it, so any effective tolerance fed back to Truck is clamped to
/// this regardless of what the angular floor would otherwise ask for.
const TRUCK_MINIMUM_TOLERANCE: f64 = 1.0e-6;

/// Bumped whenever the tessellation algorithm or its Truck entry point changes
/// in a way that can alter the mesh for an unchanged input. The inspect
/// metadata cache folds this into its validity key
/// ([`crate::cache::CachedInspection`]) so a rebuild with a new algorithm
/// revision invalidates stale statistics even when the package version is
/// unchanged.
pub const TESSELLATION_ALGORITHM_REVISION: u32 = 2;

/// The Truck revision the build resolves against, as pinned in `Cargo.toml`.
/// Folded into the cache identity for the same reason as
/// [`TESSELLATION_ALGORITHM_REVISION`]: a Truck change can move triangle counts
/// without a Look version bump, and the cache must not paper over it. Keep this
/// in sync with the `rev =` pin in `Cargo.toml`.
pub const TRUCK_REVISION: &str = "8db73b07955e512437e8afb5dfec17dd8566b4b1";

/// A targeted meshing policy for common analytic STEP geometry.
///
/// See the module docs for the motivation. The defaults target roughly 32
/// segments per full revolution on ordinary circular mechanical features: a
/// [`maximum_angular_deflection`] of `11.25°` corresponds to `360 / 11.25 = 32`
/// segments, and [`minimum_segments_per_revolution`] is the weaker `24` floor.
/// The two are combined as a maximum, so the angular interval dominates and the
/// segment floor only binds if it is set finer than the interval would imply.
///
/// [`maximum_angular_deflection`]: MeshingPolicy::maximum_angular_deflection
/// [`minimum_segments_per_revolution`]: MeshingPolicy::minimum_segments_per_revolution
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeshingPolicy {
    /// Chord/sagitta deviation as a fraction of the model diagonal. This is the
    /// legacy scalar tolerance, retained as the geometric-error bound for every
    /// curve and surface. It is what keeps output density stable across files
    /// that use different units.
    pub relative_linear_deflection: f64,
    /// Largest permitted absolute chord deviation, in model units. Caps the
    /// linear tolerance for large-radius curves so a feature many times the
    /// model's characteristic size is not held to the (coarse) model-relative
    /// value. The angular floor already keeps small features round; this keeps
    /// large features from being too coarse.
    pub maximum_absolute_deflection: f64,
    /// Largest permitted angular interval, in radians, between successive
    /// samples on a circular arc. `11.25°` ⇒ 32 segments per revolution. This
    /// is the floor that prevents small circles from collapsing to coarse
    /// polygons when `relative_linear_deflection × diagonal` is large relative
    /// to the radius.
    pub maximum_angular_deflection: f64,
    /// Fewest segments a full revolution may be discretized into, regardless of
    /// the angular interval. Combined with [`maximum_angular_deflection`] as a
    /// maximum of the two implied counts.
    pub minimum_segments_per_revolution: usize,
    /// Most segments any single curve may be discretized into. A safety cap so
    /// a pathological radius cannot force an unbounded subdivision; it rarely
    /// binds in practice.
    pub maximum_segments_per_curve: usize,
}

impl MeshingPolicy {
    /// Defaults targeting approximately 24 segments per revolution on ordinary
    /// circular mechanical features — the gentler end of the 24-32 range. A
    /// `15°` maximum angular interval ⇒ `360 / 15 = 24` segments, and
    /// [`minimum_segments_per_revolution`] is set to match. Twenty-four turns
    /// the common small holes from coarse polygons into visibly round circles
    /// for a modest triangle-count increase, while keeping the density jump
    /// small enough to avoid disturbing the constrained-Delaunay triangulator
    /// on borderline faces.
    ///
    /// [`minimum_segments_per_revolution`]: MeshingPolicy::minimum_segments_per_revolution
    pub const DEFAULT: Self = Self {
        relative_linear_deflection: 0.001,
        maximum_absolute_deflection: 1.0,
        maximum_angular_deflection: 15.0_f64.to_radians(),
        minimum_segments_per_revolution: 24,
        maximum_segments_per_curve: 1024,
    };

    /// The strongest segment count per full revolution the angular policy
    /// demands: the maximum of the count implied by
    /// [`maximum_angular_deflection`] and [`minimum_segments_per_revolution`].
    pub fn angular_floor_segments(&self) -> usize {
        let from_interval = if self.maximum_angular_deflection.is_finite()
            && self.maximum_angular_deflection > 0.0
        {
            (std::f64::consts::TAU / self.maximum_angular_deflection).ceil() as usize
        } else {
            0
        };
        self.minimum_segments_per_revolution.max(from_interval)
    }

    /// Effective world-space chord tolerance for a circular curve of `radius`,
    /// encoding the linear cap, the angular floor, and the segment ceiling.
    ///
    /// The angular floor "at least `N` segments per revolution" is expressed as
    /// the sagitta that yields exactly `N` segments on a circle of this radius,
    /// `R·(1 − cos(π/N))`, and the *tightest* (smallest) of the linear and
    /// angular tolerances wins. The segment ceiling is the dual sagitta for
    /// [`maximum_segments_per_curve`], applied as a lower bound so the result
    /// never asks Truck for finer subdivision than the ceiling allows. The
    /// returned value is fed straight to Truck's existing `parameter_division`,
    /// which then handles partial arcs, full circles, reversed ranges, and
    /// periodic seam crossings unchanged.
    ///
    /// [`maximum_segments_per_curve`]: MeshingPolicy::maximum_segments_per_curve
    pub fn effective_curve_tolerance(&self, model_tol: f64, radius: f64) -> f64 {
        let linear = model_tol.min(self.maximum_absolute_deflection);
        if !radius.is_finite() || radius <= 0.0 {
            return linear.max(TRUCK_MINIMUM_TOLERANCE);
        }
        let floor = self.angular_floor_segments();
        let angular_sagitta = if floor > 0 {
            radius * (1.0 - (std::f64::consts::PI / floor as f64).cos())
        } else {
            f64::INFINITY
        };
        let mut eff = linear.min(angular_sagitta);
        if self.maximum_segments_per_curve > 0 {
            let ceiling_sagitta = radius
                * (1.0 - (std::f64::consts::PI / self.maximum_segments_per_curve as f64).cos());
            eff = eff.max(ceiling_sagitta);
        }
        eff.max(TRUCK_MINIMUM_TOLERANCE)
    }

    /// Fewest circumferential samples the angular policy demands for a surface
    /// whose circumferential parameter spans `u_span` radians. A partial arc
    /// (a quarter-cylinder face, for instance) receives its proportional share
    /// of a full revolution's floor, not the full count.
    pub fn surface_floor_segments(&self, u_span: f64) -> usize {
        let floor = self.angular_floor_segments();
        if !u_span.is_finite() || u_span <= 0.0 || floor == 0 {
            return 0;
        }
        let fraction = (u_span / std::f64::consts::TAU).clamp(0.0, 1.0);
        ((fraction * floor as f64).ceil()).max(2.0) as usize
    }

    /// A stable identity string capturing the policy values, the tessellation
    /// algorithm revision, and the resolved Truck revision. The inspect
    /// metadata cache stores this and invalidates when it changes, so an edit
    /// to the policy or a Truck pin bump is visible to `look inspect` without a
    /// package version bump.
    pub fn identity(&self) -> String {
        format!(
            "rld={:e}|aad={:e}|mad={:e}|nspr={}|mspc={}|algo={}|truck={}",
            self.relative_linear_deflection,
            self.maximum_absolute_deflection,
            self.maximum_angular_deflection,
            self.minimum_segments_per_revolution,
            self.maximum_segments_per_curve,
            TESSELLATION_ALGORITHM_REVISION,
            TRUCK_REVISION,
        )
    }
}

impl Default for MeshingPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The current cache identity, computed from the production policy.
pub fn current_identity() -> String {
    MeshingPolicy::DEFAULT.identity()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_targets_24_segments_via_the_angular_interval() {
        // 15° ⇒ 24 segments, matching the minimum-segments floor.
        let policy = MeshingPolicy::DEFAULT;
        assert_eq!(policy.angular_floor_segments(), 24);
    }

    #[test]
    fn a_common_hole_gets_approximately_24_segments() {
        // ftc_09's Ø6.35 mm holes have radius 3.175 mm. The angular floor's
        // sagitta must bind below the model-relative linear tolerance so the
        // effective tolerance yields ~24 segments, not the ~7 the bare scalar
        // produces.
        let policy = MeshingPolicy::DEFAULT;
        let model_tol = 0.338; // ~0.001 × the ftc_09 diagonal, in mm
        let r = 3.175;
        let eff = policy.effective_curve_tolerance(model_tol, r);
        let segments = std::f64::consts::PI / (1.0 - eff / r).acos();
        assert!(
            (23.0..=25.0).contains(&segments),
            "eff={eff}, segments={segments}"
        );
        // And the effective tolerance is well below the model tolerance, so the
        // floor — not the linear term — is what binds.
        assert!(eff < model_tol);
    }

    #[test]
    fn a_large_radius_curve_is_governed_by_the_linear_term_not_the_floor() {
        // A 100 mm radius circle: the angular floor's sagitta (≈0.48 mm) is
        // above the model tolerance, so the linear term binds and the floor
        // does not. The curve gets MORE than the floor's 32 segments, not
        // fewer, because the linear tolerance is the finer control here.
        let policy = MeshingPolicy::DEFAULT;
        let model_tol = 0.338;
        let r = 100.0;
        let eff = policy.effective_curve_tolerance(model_tol, r);
        // model_tol (0.338) is below the absolute cap (1.0), so the linear term
        // is exactly the model tolerance, and it is below the angular sagitta,
        // so it wins.
        assert!((eff - model_tol).abs() < 1e-12, "eff={eff}");
        let segments = std::f64::consts::PI / (1.0 - eff / r).acos();
        assert!(
            segments > 32.0,
            "segments={segments} should exceed the 32-segment floor"
        );
    }

    #[test]
    fn the_absolute_cap_engages_for_a_very_large_model_tolerance() {
        // A huge model tolerance (5.0, as if the diagonal were ~5000 mm) is
        // capped by `maximum_absolute_deflection` (1.0). For a 100 mm circle
        // the capped linear (1.0) is still coarser than the angular floor's
        // sagitta (≈0.48), so the floor binds and the curve still gets 32
        // segments: the cap prevents the linear term from being the sole
        // control on a huge model, and the floor guarantees the minimum.
        let policy = MeshingPolicy::DEFAULT;
        let r = 100.0;
        let eff = policy.effective_curve_tolerance(5.0, r);
        assert!(eff <= 1.0, "eff={eff} must respect the absolute cap");
        let segments = std::f64::consts::PI / (1.0 - eff / r).acos();
        assert!(
            (23.0..=25.0).contains(&segments),
            "segments={segments} should be the 24-segment floor"
        );
    }

    #[test]
    fn a_partial_arc_gets_its_proportional_share() {
        let policy = MeshingPolicy::DEFAULT;
        // A quarter revolution (π/2) gets a quarter of 24, rounded up.
        assert_eq!(
            policy.surface_floor_segments(std::f64::consts::FRAC_PI_2),
            6
        );
        // A full revolution gets the full floor.
        assert_eq!(policy.surface_floor_segments(std::f64::consts::TAU), 24);
        // A tiny sliver still gets at least 2.
        assert_eq!(policy.surface_floor_segments(1e-4), 2);
    }

    #[test]
    fn identity_changes_when_the_policy_changes() {
        let a = MeshingPolicy::DEFAULT.identity();
        let mut b = MeshingPolicy::DEFAULT;
        b.minimum_segments_per_revolution = 48;
        assert_ne!(a, b.identity());
    }

    #[test]
    fn a_non_finite_radius_falls_back_to_the_capped_linear_tolerance() {
        let policy = MeshingPolicy::DEFAULT;
        let eff = policy.effective_curve_tolerance(0.338, f64::NAN);
        assert!((eff - 0.338).abs() < 1e-12);
    }
}
