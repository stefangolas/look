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
// The dense fixed-size matrix and box code below indexes fixed arrays with
// constant or iterator-derived indices that are in bounds by construction
// (never a geometry-derived index); the two 8-argument constructors mirror the
// 8-field closed-form records they assemble.
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::large_enum_variant
)] // fixed-array index math / closed-form record constructors; see above

//! BIE-002-SSI4: the restricted-pair interaction solver.
//!
//! The solver certifies the zero set of the restricted-pair interaction form
//! `F(x) = X_A(u, v) − X_B(s, t)` over the 4-D product parameter chart `x =
//! (u, v, s, t)` of a restricted pair (a pole-free sweep or canonical surface
//! × a canonical surface). The zero set is generically a curve (the
//! interaction branch); everything the solver cannot certify is a typed
//! [`InteractionOutcome::Unresolved`] witness — never a guess.
//!
//! The pipeline (scope decisions 1–8):
//!
//! 1. **Direct F evaluation.** The residual and its Jacobian are evaluated on
//!    the carrier maps themselves (no cross-multiplication, no
//!    polynomialization); the canonical carriers and the circular-section
//!    sweep normal path are evaluated in outward-rounded interval arithmetic
//!    over the parameter boxes.
//! 2. **Metric normalization σ.** Box radii, prediction steps and frames are
//!    chosen in *model* units: the per-axis parameter radius is a target model
//!    radius divided by the axis's first-fundamental column scale. This makes
//!    the certified boxes isotropic in model space and keeps the Krawczyk
//!    contraction uniform across carriers.
//! 3. **Column choice (closed form).** The 3-of-4 search ([`choose_free_axis`])
//!    tries all four 3×3 minors of the 3×4 Jacobian; the transversal subset is
//!    the one whose minor sign is certified by the (R′) predicate — the exact
//!    [`Expansion`] determinant sign at the point ([`minor_sign_expansion`])
//!    plus the box-level interval sign ([`minor_det_sign_iv`]).
//! 4. **Boundary seeding (N=3).** On each of the 8 product-box boundary strata
//!    (`BoundedStratum::Face/Edge` enumeration, one product coordinate fixed to
//!    its box endpoint) the reduced system is square: 3 equations in the 3 free
//!    coordinates. Float Newton predicts, and a certified N=3 Krawczyk solve
//!    over the metric box seeds the branch.
//! 5. **Parallelotope continuation.** From each seed the [`ParallelotopeFrame`]
//!    tracker marches the branch by the θρ step (predict along the tangent,
//!    correct by the hyperplane-augmented square N=4 system
//!    `(F, τ·(x − c))`, certify by the N=4 Krawczyk operator), recording the
//!    certified sample cells and per-sample tangent frames. A closed branch is
//!    detected when the model point returns to the seed.
//!
//! The Krawczyk operator is **instantiated, never extended**: the two system
//! types [`Ssi3System`] (N=3, one coordinate fixed) and [`Ssi4System`] (N=4,
//! hyperplane-augmented) implement the landed
//! [`KrawczykSystem`](truck_evidence::num::krawczyk::KrawczykSystem) trait and
//! are driven through the landed
//! [`krawczyk`](truck_evidence::num::krawczyk::krawczyk) operator
//! (truck-evidence `num/krawczyk.rs`); that file is not edited. The parallelotope
//! continuation algebra lives in `truck-evidence/src/num/parallelotope.rs`
//! (new); this module supplies the pair-side systems and the scheduler.
//!
//! **H-1.** This file carries no `unwrap`, no `expect`, no `panic!`, and no
//! out-of-range indexing reachable from geometry; where fixed-size matrices
//! are indexed, the indices are constants or iterator-derived and in bounds by
//! construction.
//!
//! **H-6.** Float-computed values (predictors, tangents, model points) are
//! never recorded as `Method::Exact`: every certified sample carries the
//! Krawczyk certificate of its box, and the certified statement is always the
//! box, never a float.
//!
//! **Determinism.** Identical ordered input → identical verdicts: fixed
//! stratum order (surface 0 then 1, axis 0 then 1, `lo` then `hi`), fixed
//! Newton start grid, fixed axis scan order in the column choice, and the
//! bisection discipline of the landed operator (axis order, low-before-high).

use crate::construct::bie::{InteractionOutcome, WitnessCell};
use crate::formal::exact::{CertifiedSign, Expansion};
use truck_base::evidence::{Budget, Certificate, Certified, Method, Outcome, PropMap};
use truck_evidence::elementary::cos as icos;
use truck_evidence::elementary::sin as isin;
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use truck_evidence::num::parallelotope::{
    box_around, theta_rho_step, ParallelotopeFrame, StepVerdict,
};
use truck_geometry::prelude::{Cylinder, InnerSpace, Plane, Point3, Sphere, Vector3};

/// The restricted-pair solver parameters: the certified geometry scale, the
/// continuation cadence, and the per-stage budgets.
///
/// Every length is a *model* unit (never a bare absolute tolerance): the
/// certified boxes are made isotropic in model space by the σ metric
/// normalization, and the θ step is a model-space arc advance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ssi4Parameters {
    /// The target model-space half-width of a certified box (the σ-normalized
    /// parallelotope radius).
    pub metric_radius: f64,
    /// The model-space arc advance of one continuation sample.
    pub theta_step: f64,
    /// The subdivision budget of one seed certification (N=3).
    pub seed_budget: Budget,
    /// The subdivision budget of one continuation certification (N=4).
    pub step_budget: Budget,
    /// The maximum number of continuation samples on one branch.
    pub max_steps: usize,
    /// The number of per-axis samples of the deterministic seed-start grid.
    pub seed_grid_per_axis: usize,
    /// The model-space distance to the seed that closes a branch, as a
    /// multiple of [`Self::theta_step`].
    pub closure_radius_steps: f64,
}

impl Default for Ssi4Parameters {
    fn default() -> Self {
        Ssi4Parameters {
            metric_radius: 2.0e-2,
            theta_step: 1.5e-2,
            seed_budget: Budget::new(512, 0, 0),
            step_budget: Budget::new(128, 0, 0),
            max_steps: 2048,
            seed_grid_per_axis: 3,
            closure_radius_steps: 3.0,
        }
    }
}

/// A certified interaction-curve branch in the 4-D product chart (frozen
/// contract, spine §3; BIE-004 escalates, BIE-005 consumes).
#[derive(Clone, Debug)]
pub struct CertifiedChartCurve {
    /// Ordered samples along the branch (parameter cells, certified).
    pub samples: Vec<ChartSample>,
    /// Per-sample tangent frames (the parallelotope output).
    pub tangent_frames: Vec<ParallelotopeFrame>,
    /// The unresolved witness slot (κ/cell/slope) for escalation.
    pub witness: Option<InteractionOutcome>,
}

impl CertifiedChartCurve {
    /// Whether the branch is empty but typed unresolved (never a guess).
    pub fn is_unresolved(&self) -> bool {
        self.samples.is_empty() && self.witness.is_some()
    }
}

/// One certified sample of an interaction branch: the parameter cell (the
/// certified statement), the float chart centre, the float model point used
/// for ordering and diagnostics, and the Krawczyk certificate of the cell.
#[derive(Clone, Debug)]
pub struct ChartSample {
    /// The certified 4-D parameter cell: it contains exactly one solution of
    /// the localized (stratum or hyperplane-augmented) system — one certified
    /// sample of the branch.
    pub cell: WitnessCell,
    /// The float chart centre of the cell (diagnostics; the cell is the
    /// certified statement).
    pub chart: [f64; 4],
    /// The model point at the cell centre (float diagnostics, H-6: the cell is
    /// the certified statement).
    pub centre: Point3,
    /// The Krawczyk certificate of the cell.
    pub cert: Certificate,
}

/// A restricted-pair carrier chart: one of the pole-free sweep / canonical
/// carriers of the restricted normal path. Each carrier is a 2-parameter map
/// with closed-form point and partial evaluations in floats and in outward-
/// rounded interval arithmetic.
#[derive(Clone, Debug)]
pub enum RestrictedChart {
    /// An affine plane `X(u, v) = origin + u·u_axis + v·v_axis`.
    Plane {
        /// The plane origin.
        origin: Point3,
        /// The u axis vector.
        u_axis: Vector3,
        /// The v axis vector.
        v_axis: Vector3,
    },
    /// A sphere `X(s, t) = center + r·(sin s·cos t, sin s·sin t, cos s)`.
    Sphere {
        /// The sphere centre.
        center: Point3,
        /// The sphere radius.
        radius: f64,
    },
    /// A canonical z-axis cylinder `X(s, t) = center + (r·cos s, r·sin s, t)`.
    Cylinder {
        /// The cylinder centre (on the axis).
        center: Point3,
        /// The cylinder radius.
        radius: f64,
    },
    /// The pole-free circular-section sweep normal path: a straight spine
    /// `C(s)` with a linear scale radius and a circular ring perpendicular to
    /// the spine.
    CircularSweep(CircularSweepUnit),
}

impl RestrictedChart {
    /// A plane carrier from a landed [`Plane`] (its own origin/axis basis).
    pub fn from_plane(plane: Plane) -> Self {
        RestrictedChart::Plane {
            origin: plane.origin(),
            u_axis: plane.u_axis(),
            v_axis: plane.v_axis(),
        }
    }

    /// A sphere carrier from a landed [`Sphere`].
    pub fn from_sphere(sphere: Sphere) -> Self {
        RestrictedChart::Sphere {
            center: sphere.center(),
            radius: sphere.radius(),
        }
    }

    /// A cylinder carrier from a landed [`Cylinder`] (canonical axis-aligned
    /// cylinder about the z-axis through its center).
    pub fn from_cylinder(cylinder: Cylinder) -> Self {
        RestrictedChart::Cylinder {
            center: cylinder.center(),
            radius: cylinder.radius(),
        }
    }

    /// A pole-free circular-section sweep carrier from the windowed straight-
    /// spine `Scale`-of-a-circle data. `None` when the spine is degenerate
    /// (zero length) or a window is inverted (nothing to certify).
    pub fn circular_sweep(
        spine_from: Point3,
        spine_to: Point3,
        radius_start: f64,
        radius_end: f64,
        s0: f64,
        s1: f64,
        v0: f64,
        v1: f64,
    ) -> Option<Self> {
        CircularSweepUnit::try_new(
            spine_from,
            spine_to,
            radius_start,
            radius_end,
            s0,
            s1,
            v0,
            v1,
        )
        .map(RestrictedChart::CircularSweep)
    }
}

/// The closed-form pole-free circular-section sweep of the restricted normal
/// path (the continuous circular-section limit the restricted engine solves):
/// `X(s, v) = C(s) + radius(s)·(cos 2πv·e1 + sin 2πv·e2)` over the windowed
/// domain `[s0, s1] × [v0, v1]`, with the straight spine `C(s)` and the linear
/// scale radius.
#[derive(Clone, Debug)]
pub struct CircularSweepUnit {
    /// The first spine point `C(s0)`.
    pub spine_from: Point3,
    /// The last spine point `C(s1)`.
    pub spine_to: Point3,
    /// The scale radius at `s0`.
    pub radius_start: f64,
    /// The scale radius at `s1`.
    pub radius_end: f64,
    /// The spine window start.
    pub s0: f64,
    /// The spine window end.
    pub s1: f64,
    /// The ring window start.
    pub v0: f64,
    /// The ring window end.
    pub v1: f64,
    /// A unit ring direction perpendicular to the spine.
    ring0: Vector3,
    /// A second unit ring direction perpendicular to the spine and `ring0`.
    ring1: Vector3,
}

impl CircularSweepUnit {
    /// Assembles the unit sweep, computing the deterministic perpendicular
    /// ring frame of the spine direction. `None` on a zero-length spine or an
    /// inverted window.
    fn try_new(
        spine_from: Point3,
        spine_to: Point3,
        radius_start: f64,
        radius_end: f64,
        s0: f64,
        s1: f64,
        v0: f64,
        v1: f64,
    ) -> Option<Self> {
        if s1 <= s0 || v1 <= v0 {
            return None;
        }
        let d = spine_to - spine_from;
        let len = d.magnitude();
        if !len.is_finite() || len == 0.0 {
            return None;
        }
        let direction = d / len;
        let (ring0, ring1) = perpendicular_frame(&direction)?;
        Some(CircularSweepUnit {
            spine_from,
            spine_to,
            radius_start,
            radius_end,
            s0,
            s1,
            v0,
            v1,
            ring0,
            ring1,
        })
    }

    /// The spine fraction `(s − s0)/(s1 − s0)`.
    fn station(&self, s: f64) -> f64 {
        (s - self.s0) / (self.s1 - self.s0)
    }

    /// The linear scale radius at station `s`.
    fn radius_at(&self, s: f64) -> f64 {
        let tau = self.station(s);
        self.radius_start + (self.radius_end - self.radius_start) * tau
    }
}

/// A deterministic orthonormal perpendicular frame `(e0, e1)` of a unit vector
/// `d`: `e0` is the unit vector along the least-parallel coordinate axis after
/// projecting out `d`, `e1 = d × e0`. `None` when the projection degenerates
/// (cannot happen for a unit finite `d`).
fn perpendicular_frame(d: &Vector3) -> Option<(Vector3, Vector3)> {
    let axes = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];
    // The least-aligned axis is the most stable source of the first ring
    // direction.
    let mut best = 0usize;
    let mut best_dot = f64::INFINITY;
    for (i, a) in axes.iter().enumerate() {
        let c = a.dot(*d).abs();
        if c < best_dot {
            best_dot = c;
            best = i;
        }
    }
    let raw = axes[best] - axes[best].dot(*d) * *d;
    let len = raw.magnitude();
    if !len.is_finite() || len == 0.0 {
        return None;
    }
    let e0 = raw / len;
    let e1 = d.cross(e0);
    Some((e0, e1))
}

// ---------------------------------------------------------------------------
// Closed-form carrier evaluation: floats and outward-rounded intervals
// ---------------------------------------------------------------------------

/// A degenerate interval from a finite float. A non-finite input degrades to
/// the empty interval (a caller bug, never a panic).
fn iv(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// An interval from two ordered finite floats.
fn iv_lo_hi(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The outward-rounded negation of an interval 3-vector.
fn neg3(v: &[Interval; 3]) -> [Interval; 3] {
    [-v[0], -v[1], -v[2]]
}

/// The outward-rounded difference of two interval 3-vectors.
fn sub3(a: &[Interval; 3], b: &[Interval; 3]) -> [Interval; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The float difference of two float 3-vectors.
fn sub3_f(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

impl RestrictedChart {
    /// The float surface point at the carrier parameters `(p0, p1)`.
    fn point_f(&self, p0: f64, p1: f64) -> [f64; 3] {
        match self {
            RestrictedChart::Plane {
                origin,
                u_axis,
                v_axis,
            } => {
                let o = [origin.x, origin.y, origin.z];
                let u = [u_axis.x, u_axis.y, u_axis.z];
                let v = [v_axis.x, v_axis.y, v_axis.z];
                [
                    o[0] + p0 * u[0] + p1 * v[0],
                    o[1] + p0 * u[1] + p1 * v[1],
                    o[2] + p0 * u[2] + p1 * v[2],
                ]
            }
            RestrictedChart::Sphere { center, radius } => {
                let c = [center.x, center.y, center.z];
                let (ss, cs) = p0.sin_cos();
                let (st, ct) = p1.sin_cos();
                [
                    c[0] + radius * ss * ct,
                    c[1] + radius * ss * st,
                    c[2] + radius * cs,
                ]
            }
            RestrictedChart::Cylinder { center, radius } => {
                let c = [center.x, center.y, center.z];
                let (ss, cs) = p0.sin_cos();
                [c[0] + radius * cs, c[1] + radius * ss, c[2] + p1]
            }
            RestrictedChart::CircularSweep(sweep) => {
                let tau = sweep.station(p0);
                let r = sweep.radius_at(p0);
                let angle = std::f64::consts::TAU * p1;
                let (sa, ca) = angle.sin_cos();
                let spine = {
                    let from = [sweep.spine_from.x, sweep.spine_from.y, sweep.spine_from.z];
                    let to = [sweep.spine_to.x, sweep.spine_to.y, sweep.spine_to.z];
                    [
                        from[0] + tau * (to[0] - from[0]),
                        from[1] + tau * (to[1] - from[1]),
                        from[2] + tau * (to[2] - from[2]),
                    ]
                };
                let ring = {
                    let e0 = [sweep.ring0.x, sweep.ring0.y, sweep.ring0.z];
                    let e1 = [sweep.ring1.x, sweep.ring1.y, sweep.ring1.z];
                    [
                        ca * e0[0] + sa * e1[0],
                        ca * e0[1] + sa * e1[1],
                        ca * e0[2] + sa * e1[2],
                    ]
                };
                [
                    spine[0] + r * ring[0],
                    spine[1] + r * ring[1],
                    spine[2] + r * ring[2],
                ]
            }
        }
    }

    /// The float partials `(dX/dp0, dX/dp1)` at the carrier parameters.
    fn partials_f(&self, p0: f64, p1: f64) -> ([f64; 3], [f64; 3]) {
        match self {
            RestrictedChart::Plane {
                origin: _,
                u_axis,
                v_axis,
            } => (
                [u_axis.x, u_axis.y, u_axis.z],
                [v_axis.x, v_axis.y, v_axis.z],
            ),
            RestrictedChart::Sphere { center: _, radius } => {
                let (ss, cs) = p0.sin_cos();
                let (st, ct) = p1.sin_cos();
                let r = *radius;
                (
                    [r * cs * ct, r * cs * st, -r * ss],
                    [-r * ss * st, r * ss * ct, 0.0],
                )
            }
            RestrictedChart::Cylinder { center: _, radius } => {
                let (ss, cs) = p0.sin_cos();
                let r = *radius;
                ([-r * ss, r * cs, 0.0], [0.0, 0.0, 1.0])
            }
            RestrictedChart::CircularSweep(sweep) => {
                let span = sweep.s1 - sweep.s0;
                let inv_span = 1.0 / span;
                let r0 = sweep.radius_start;
                let r1 = sweep.radius_end;
                let tau = sweep.station(p0);
                let r = sweep.radius_at(p0);
                let angle = std::f64::consts::TAU * p1;
                let (sa, ca) = angle.sin_cos();
                let e0 = [sweep.ring0.x, sweep.ring0.y, sweep.ring0.z];
                let e1 = [sweep.ring1.x, sweep.ring1.y, sweep.ring1.z];
                let d = [
                    sweep.spine_to.x - sweep.spine_from.x,
                    sweep.spine_to.y - sweep.spine_from.y,
                    sweep.spine_to.z - sweep.spine_from.z,
                ];
                let ring = [
                    ca * e0[0] + sa * e1[0],
                    ca * e0[1] + sa * e1[1],
                    ca * e0[2] + sa * e1[2],
                ];
                let ring_t = [
                    -std::f64::consts::TAU * sa * e0[0] + std::f64::consts::TAU * ca * e1[0],
                    -std::f64::consts::TAU * sa * e0[1] + std::f64::consts::TAU * ca * e1[1],
                    -std::f64::consts::TAU * sa * e0[2] + std::f64::consts::TAU * ca * e1[2],
                ];
                let dr = (r1 - r0) * inv_span;
                let _ = tau;
                (
                    [
                        d[0] * inv_span + dr * ring[0],
                        d[1] * inv_span + dr * ring[1],
                        d[2] * inv_span + dr * ring[2],
                    ],
                    [r * ring_t[0], r * ring_t[1], r * ring_t[2]],
                )
            }
        }
    }

    /// The interval surface point over the parameter box `(p0, p1)`.
    fn point_iv(&self, p0: Interval, p1: Interval) -> [Interval; 3] {
        match self {
            RestrictedChart::Plane {
                origin,
                u_axis,
                v_axis,
            } => {
                let o = [iv(origin.x), iv(origin.y), iv(origin.z)];
                let u = [iv(u_axis.x), iv(u_axis.y), iv(u_axis.z)];
                let v = [iv(v_axis.x), iv(v_axis.y), iv(v_axis.z)];
                [
                    o[0] + p0 * u[0] + p1 * v[0],
                    o[1] + p0 * u[1] + p1 * v[1],
                    o[2] + p0 * u[2] + p1 * v[2],
                ]
            }
            RestrictedChart::Sphere { center, radius } => {
                let c = [iv(center.x), iv(center.y), iv(center.z)];
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                let (st, ct) = (isin(p1), icos(p1));
                [c[0] + r * ss * ct, c[1] + r * ss * st, c[2] + r * cs]
            }
            RestrictedChart::Cylinder { center, radius } => {
                let c = [iv(center.x), iv(center.y), iv(center.z)];
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                [c[0] + r * cs, c[1] + r * ss, c[2] + p1]
            }
            RestrictedChart::CircularSweep(sweep) => {
                let span = iv(sweep.s1 - sweep.s0);
                let tau = (p0 - iv(sweep.s0)) / span;
                let r =
                    iv(sweep.radius_start) + (iv(sweep.radius_end) - iv(sweep.radius_start)) * tau;
                let angle = iv(std::f64::consts::TAU) * p1;
                let (sa, ca) = (isin(angle), icos(angle));
                let from = [
                    iv(sweep.spine_from.x),
                    iv(sweep.spine_from.y),
                    iv(sweep.spine_from.z),
                ];
                let to = [
                    iv(sweep.spine_to.x),
                    iv(sweep.spine_to.y),
                    iv(sweep.spine_to.z),
                ];
                let spine = [
                    from[0] + tau * (to[0] - from[0]),
                    from[1] + tau * (to[1] - from[1]),
                    from[2] + tau * (to[2] - from[2]),
                ];
                let e0 = [iv(sweep.ring0.x), iv(sweep.ring0.y), iv(sweep.ring0.z)];
                let e1 = [iv(sweep.ring1.x), iv(sweep.ring1.y), iv(sweep.ring1.z)];
                let ring = [
                    ca * e0[0] + sa * e1[0],
                    ca * e0[1] + sa * e1[1],
                    ca * e0[2] + sa * e1[2],
                ];
                [
                    spine[0] + r * ring[0],
                    spine[1] + r * ring[1],
                    spine[2] + r * ring[2],
                ]
            }
        }
    }

    /// The interval partials `(dX/dp0, dX/dp1)` over the parameter box.
    fn partials_iv(&self, p0: Interval, p1: Interval) -> ([Interval; 3], [Interval; 3]) {
        match self {
            RestrictedChart::Plane {
                origin: _,
                u_axis,
                v_axis,
            } => (
                [iv(u_axis.x), iv(u_axis.y), iv(u_axis.z)],
                [iv(v_axis.x), iv(v_axis.y), iv(v_axis.z)],
            ),
            RestrictedChart::Sphere { center: _, radius } => {
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                let (st, ct) = (isin(p1), icos(p1));
                (
                    [r * cs * ct, r * cs * st, -r * ss],
                    [-r * ss * st, r * ss * ct, iv(0.0)],
                )
            }
            RestrictedChart::Cylinder { center: _, radius } => {
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                ([-r * ss, r * cs, iv(0.0)], [iv(0.0), iv(0.0), iv(1.0)])
            }
            RestrictedChart::CircularSweep(sweep) => {
                let span = iv(sweep.s1 - sweep.s0);
                let tau = (p0 - iv(sweep.s0)) / span;
                let r0 = iv(sweep.radius_start);
                let r1 = iv(sweep.radius_end);
                let dr = (r1 - r0) / span;
                let r = r0 + (r1 - r0) * tau;
                let angle = iv(std::f64::consts::TAU) * p1;
                let (sa, ca) = (isin(angle), icos(angle));
                let two_pi = iv(std::f64::consts::TAU);
                let d = [
                    iv(sweep.spine_to.x - sweep.spine_from.x),
                    iv(sweep.spine_to.y - sweep.spine_from.y),
                    iv(sweep.spine_to.z - sweep.spine_from.z),
                ];
                let e0 = [iv(sweep.ring0.x), iv(sweep.ring0.y), iv(sweep.ring0.z)];
                let e1 = [iv(sweep.ring1.x), iv(sweep.ring1.y), iv(sweep.ring1.z)];
                let ring = [
                    ca * e0[0] + sa * e1[0],
                    ca * e0[1] + sa * e1[1],
                    ca * e0[2] + sa * e1[2],
                ];
                let ring_t = [
                    -two_pi * sa * e0[0] + two_pi * ca * e1[0],
                    -two_pi * sa * e0[1] + two_pi * ca * e1[1],
                    -two_pi * sa * e0[2] + two_pi * ca * e1[2],
                ];
                let inv_span = iv(1.0) / span;
                (
                    [
                        d[0] * inv_span + dr * ring[0],
                        d[1] * inv_span + dr * ring[1],
                        d[2] * inv_span + dr * ring[2],
                    ],
                    [r * ring_t[0], r * ring_t[1], r * ring_t[2]],
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The restricted-pair F-form and the two Krawczyk systems
// ---------------------------------------------------------------------------

/// The restricted-pair F-form: `F(x) = X_A(u, v) − X_B(s, t)` over the 4-D
/// product chart `x = (u, v, s, t)`.
#[derive(Clone, Debug)]
pub struct FForm {
    /// The first carrier (parameters `(u, v)`).
    pub a: RestrictedChart,
    /// The second carrier (parameters `(s, t)`).
    pub b: RestrictedChart,
}

impl FForm {
    /// The float residual at the 4-D chart point.
    fn residual_f(&self, x: &[f64; 4]) -> [f64; 3] {
        let pa = self.a.point_f(x[0], x[1]);
        let pb = self.b.point_f(x[2], x[3]);
        sub3_f(&pa, &pb)
    }

    /// The float 3×4 Jacobian columns `(X_u, X_v, −X_s, −X_t)` at the point.
    fn partial_columns_f(&self, x: &[f64; 4]) -> [[f64; 3]; 4] {
        let (au, av) = self.a.partials_f(x[0], x[1]);
        let (bs, bt) = self.b.partials_f(x[2], x[3]);
        [au, av, [-bs[0], -bs[1], -bs[2]], [-bt[0], -bt[1], -bt[2]]]
    }

    /// The interval residual over the 4-D box (outward-rounded).
    fn residual_iv(&self, x: &[Interval; 4]) -> [Interval; 3] {
        let pa = self.a.point_iv(x[0], x[1]);
        let pb = self.b.point_iv(x[2], x[3]);
        sub3(&pa, &pb)
    }

    /// The interval 3×4 Jacobian columns over the 4-D box.
    fn partial_columns_iv(&self, x: &[Interval; 4]) -> [[Interval; 3]; 4] {
        let (au, av) = self.a.partials_iv(x[0], x[1]);
        let (bs, bt) = self.b.partials_iv(x[2], x[3]);
        [au, av, neg3(&bs), neg3(&bt)]
    }
}

/// The box of a carrier's parameter window from the product cell (index 0/1
/// for `A`, 2/3 for `B`).
fn carrier_param_box(cell: &WitnessCell, side: usize) -> [(f64, f64); 2] {
    match side {
        0 => [(cell.u.0, cell.u.1), (cell.v.0, cell.v.1)],
        _ => [(cell.s.0, cell.s.1), (cell.t.0, cell.t.1)],
    }
}

/// The 4-D box of the product cell.
fn cell_box(cell: &WitnessCell) -> [(f64, f64); 4] {
    [
        (cell.u.0, cell.u.1),
        (cell.v.0, cell.v.1),
        (cell.s.0, cell.s.1),
        (cell.t.0, cell.t.1),
    ]
}

/// The N=3 Krawczyk system over the F-form with one product coordinate fixed:
/// the square `E×F`/`F×E` boundary-stratum systems and the coordinate slice
/// solves of the continuation both take this shape.
#[derive(Clone, Debug)]
pub struct Ssi3System {
    /// The restricted-pair F-form.
    pub form: FForm,
    /// The fixed product axis (0..4).
    pub fixed_axis: usize,
    /// The fixed coordinate value.
    pub fixed_value: f64,
}

impl Ssi3System {
    /// Builds the square system over the F-form with `axis` fixed at `value`.
    pub fn new(form: FForm, fixed_axis: usize, fixed_value: f64) -> Self {
        Ssi3System {
            form,
            fixed_axis,
            fixed_value,
        }
    }

    /// The three free axes in ascending order.
    fn free_axes(&self) -> [usize; 3] {
        let mut out = [0usize; 3];
        let mut k = 0usize;
        for j in 0..4 {
            if j != self.fixed_axis {
                out[k] = j;
                k += 1;
            }
        }
        out
    }

    /// Embeds a 3-D free-coordinate point into the 4-D chart.
    fn embed(&self, x: &[f64; 3]) -> [f64; 4] {
        let free = self.free_axes();
        let mut out = [0.0; 4];
        for i in 0..4 {
            if i == self.fixed_axis {
                out[i] = self.fixed_value;
            }
        }
        for (k, &axis) in free.iter().enumerate() {
            out[axis] = x[k];
        }
        out
    }

    /// Embeds a 3-D free-coordinate box into the 4-D chart box.
    fn embed_box(&self, b: &[Interval; 3]) -> [Interval; 4] {
        let free = self.free_axes();
        let mut out = [Interval::EMPTY; 4];
        for i in 0..4 {
            out[i] = iv_lo_hi(self.fixed_value, self.fixed_value);
        }
        for (k, &axis) in free.iter().enumerate() {
            out[axis] = b[k];
        }
        out
    }
}

impl KrawczykSystem<3> for Ssi3System {
    fn f_point(&self, x: &[f64; 3]) -> [Interval; 3] {
        let x4 = self.embed(x);
        let box4 = [iv(x4[0]), iv(x4[1]), iv(x4[2]), iv(x4[3])];
        self.form.residual_iv(&box4)
    }

    fn jacobian(&self, b: &[Interval; 3]) -> [[Interval; 3]; 3] {
        let box4 = self.embed_box(b);
        let cols = self.form.partial_columns_iv(&box4);
        let free = self.free_axes();
        let mut out = [[Interval::EMPTY; 3]; 3];
        for r in 0..3 {
            for (c, &axis) in free.iter().enumerate() {
                out[r][c] = cols[axis][r];
            }
        }
        out
    }

    fn preconditioner(&self, x: &[f64; 3]) -> Option<[[f64; 3]; 3]> {
        let x4 = self.embed(x);
        let cols = self.form.partial_columns_f(&x4);
        let free = self.free_axes();
        let mut m = [[0.0; 3]; 3];
        for r in 0..3 {
            for (c, &axis) in free.iter().enumerate() {
                m[r][c] = cols[axis][r];
            }
        }
        invert3(&m)
    }
}

/// The N=4 Krawczyk system over the F-form augmented by the hyperplane
/// `τ·x = rhs` (the θρ corrector of the parallelotope continuation).
#[derive(Clone, Debug)]
pub struct Ssi4System {
    /// The restricted-pair F-form.
    pub form: FForm,
    /// The hyperplane normal (the unit tangent of the continuation).
    pub normal: [f64; 4],
    /// The hyperplane right-hand side.
    pub rhs: f64,
}

impl Ssi4System {
    /// Builds the augmented N=4 system.
    pub fn new(form: FForm, normal: [f64; 4], rhs: f64) -> Self {
        Ssi4System { form, normal, rhs }
    }
}

impl KrawczykSystem<4> for Ssi4System {
    fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
        let box4 = [iv(x[0]), iv(x[1]), iv(x[2]), iv(x[3])];
        let f = self.form.residual_iv(&box4);
        let mut aug = 0.0f64;
        for j in 0..4 {
            aug += self.normal[j] * x[j];
        }
        aug -= self.rhs;
        [f[0], f[1], f[2], iv(aug)]
    }

    fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
        let cols = self.form.partial_columns_iv(b);
        let mut out = [[Interval::EMPTY; 4]; 4];
        for r in 0..3 {
            for c in 0..4 {
                out[r][c] = cols[c][r];
            }
        }
        for c in 0..4 {
            out[3][c] = iv(self.normal[c]);
        }
        out
    }

    fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
        let cols = self.form.partial_columns_f(x);
        let mut m = [[0.0; 4]; 4];
        for r in 0..3 {
            for c in 0..4 {
                m[r][c] = cols[c][r];
            }
        }
        for c in 0..4 {
            m[3][c] = self.normal[c];
        }
        invert4(&m)
    }
}

/// The float inverse of a 3×3 matrix by cofactors over the determinant.
/// `None` on a (near-)singular matrix.
fn invert3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = det3_f(m);
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let adj = [
        [
            m[1][1] * m[2][2] - m[1][2] * m[2][1],
            m[0][2] * m[2][1] - m[0][1] * m[2][2],
            m[0][1] * m[1][2] - m[0][2] * m[1][1],
        ],
        [
            m[1][2] * m[2][0] - m[1][0] * m[2][2],
            m[0][0] * m[2][2] - m[0][2] * m[2][0],
            m[0][2] * m[1][0] - m[0][0] * m[1][2],
        ],
        [
            m[1][0] * m[2][1] - m[1][1] * m[2][0],
            m[0][1] * m[2][0] - m[0][0] * m[2][1],
            m[0][0] * m[1][1] - m[0][1] * m[1][0],
        ],
    ];
    let mut out = [[0.0; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            out[r][c] = adj[r][c] / det;
        }
    }
    Some(out)
}

/// The float inverse of a 4×4 matrix by Gauss–Jordan with partial pivoting.
/// `None` on a (near-)singular matrix.
fn invert4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut a = *m;
    let mut inv = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    for col in 0..4 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..4 {
            let cand = a[r][col].abs();
            if cand > best {
                best = cand;
                pivot = r;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            for c in 0..4 {
                let t = a[col][c];
                a[col][c] = a[pivot][c];
                a[pivot][c] = t;
                let t = inv[col][c];
                inv[col][c] = inv[pivot][c];
                inv[pivot][c] = t;
            }
        }
        let d = a[col][col];
        for c in 0..4 {
            a[col][c] /= d;
            inv[col][c] /= d;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let f = a[r][col];
            if f == 0.0 {
                continue;
            }
            for c in 0..4 {
                a[r][c] -= f * a[col][c];
                inv[r][c] -= f * inv[col][c];
            }
        }
    }
    Some(inv)
}

/// The float determinant of a 3×3 matrix.
fn det3_f(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

// ---------------------------------------------------------------------------
// The (R′) minor-sign predicate and the transversal column choice
// ---------------------------------------------------------------------------

/// The exact 3×3 determinant of a float matrix as an [`Expansion`]: every term
/// is an exact product, so the sign is an exact predicate over the `f64`
/// entries (no epsilon anywhere).
fn det3_expansion(m: &[[f64; 3]; 3]) -> Expansion {
    // A degenerate expansion holding exactly one float component.
    let sc = |x: f64| Expansion::zero().grow(x);
    // The exact product of three floats.
    let p3 = |x: f64, y: f64, z: f64| Expansion::from_product(x, y).mul_expansion(&sc(z));
    // det = a(ei − fh) − b(di − fg) + c(dh − eg)
    let aei = p3(m[0][0], m[1][1], m[2][2]);
    let afh = p3(m[0][0], m[1][2], m[2][1]);
    let bdi = p3(m[0][1], m[1][0], m[2][2]);
    let bfg = p3(m[0][1], m[1][2], m[2][0]);
    let cdh = p3(m[0][2], m[1][0], m[2][1]);
    let ceg = p3(m[0][2], m[1][1], m[2][0]);
    let mut acc = aei;
    acc = acc.merge(&afh.negate());
    acc = acc.merge(&bdi.negate());
    acc = acc.merge(&bfg);
    acc = acc.merge(&cdh);
    acc = acc.merge(&ceg.negate());
    acc
}

/// The (R′) minor-sign predicate at a point: the exact sign of the 3×3
/// determinant of the float minor, by the landed [`Expansion`] exact
/// arithmetic. `None` exactly when the determinant is exactly zero (the minor
/// is degenerate).
pub fn minor_sign_expansion(m3: &[[f64; 3]; 3]) -> Option<CertifiedSign> {
    let det = det3_expansion(m3);
    match det.sign() {
        CertifiedSign::Zero => None,
        sign => Some(sign),
    }
}

/// The interval 3×3 determinant (outward-rounded) of an interval minor.
fn det3_iv(m3: &[[Interval; 3]; 3]) -> Interval {
    let a = m3[0][0] * (m3[1][1] * m3[2][2] - m3[1][2] * m3[2][1]);
    let b = m3[0][1] * (m3[1][0] * m3[2][2] - m3[1][2] * m3[2][0]);
    let c = m3[0][2] * (m3[1][0] * m3[2][1] - m3[1][1] * m3[2][0]);
    a - b + c
}

/// The (R′) box-level minor sign: the certified sign of the 3×3 determinant
/// over an interval minor, `Some` only when the interval is strictly away
/// from zero (the minor is transversal on the whole box).
pub fn minor_det_sign_iv(m3: &[[Interval; 3]; 3]) -> Option<CertifiedSign> {
    let det = det3_iv(m3);
    if !det.inf().is_finite() || !det.sup().is_finite() {
        return None;
    }
    if det.inf() > 0.0 {
        Some(CertifiedSign::Positive)
    } else if det.sup() < 0.0 {
        Some(CertifiedSign::Negative)
    } else {
        None
    }
}

/// The 3×3 float minor of a 3×4 float Jacobian (given as columns) after
/// deleting column `free`.
pub fn minor3_of_jacobian(cols: &[[f64; 3]; 4], free: usize) -> [[f64; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for r in 0..3 {
        let mut c_out = 0usize;
        for c in 0..4 {
            if c == free {
                continue;
            }
            out[r][c_out] = cols[c][r];
            c_out += 1;
        }
    }
    out
}

/// The interval 3×3 minor of a 3×4 interval Jacobian (given as columns) after
/// deleting column `free`.
pub fn minor3_iv_of_jacobian(cols: &[[Interval; 3]; 4], free: usize) -> [[Interval; 3]; 3] {
    let mut out = [[Interval::EMPTY; 3]; 3];
    for r in 0..3 {
        let mut c_out = 0usize;
        for c in 0..4 {
            if c == free {
                continue;
            }
            out[r][c_out] = cols[c][r];
            c_out += 1;
        }
    }
    out
}

/// The closed-form transversal column choice (scope decision 5): try all four
/// 3-of-4 subsets of the 3×4 float Jacobian columns and return the free axis
/// (the deleted column) of the subset whose 3×3 minor sign is certified by the
/// (R′) exact predicate — deterministically the best-conditioned certified
/// subset (largest certified `|det|`), ties toward the lowest axis index.
/// `None` when no subset certifies (the chart is degenerate at the point: a
/// tangency or a pole, never a guess).
pub fn choose_free_axis(cols: &[[f64; 3]; 4]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for free in 0..4 {
        let minor = minor3_of_jacobian(cols, free);
        if minor_sign_expansion(&minor).is_some() {
            let mag = det3_f(&minor).abs();
            let better = match best {
                None => true,
                Some((_, best_mag)) => mag > best_mag,
            };
            if better {
                best = Some((free, mag));
            }
        }
    }
    best.map(|(free, _)| free)
}

/// The box-level transversal column choice: the free axis whose interval 3×3
/// minor is certified away from zero over the box. `None` when none is.
pub fn choose_free_axis_over_box(cols: &[[Interval; 3]; 4]) -> Option<usize> {
    for free in 0..4 {
        let minor = minor3_iv_of_jacobian(cols, free);
        if minor_det_sign_iv(&minor).is_some() {
            return Some(free);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Certificate helpers and diagnostics
// ---------------------------------------------------------------------------

/// A certificate for a certified interval-method result with the remaining
/// budget.
fn interval_certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: *budget,
        margin: truck_base::evidence::Margin::UNBOUNDED,
        modulus: truck_base::evidence::Modulus::Unbounded,
    }
}

/// The κ / slope diagnostics of a cell: κ is the reciprocal of the largest
/// certified 3×3 minor magnitude of the float Jacobian at the centre (a
/// conditioning-style witness; large when every minor is near-degenerate), and
/// slope is the signed ratio of that dominant minor to its column-norm product
/// (a dimensionless transversality diagnostic).
fn unresolved_diagnostics(form: &FForm, x: &[f64; 4]) -> (f64, f64) {
    let cols = form.partial_columns_f(x);
    let mut best_det = 0.0f64;
    let mut best_minor: Option<[[f64; 3]; 3]> = None;
    for free in 0..4 {
        let minor = minor3_of_jacobian(&cols, free);
        if minor_sign_expansion(&minor).is_some() {
            let d = det3_f(&minor);
            if d.abs() > best_det {
                best_det = d.abs();
                best_minor = Some(minor);
            }
        }
    }
    match best_minor {
        Some(minor) => {
            let d = det3_f(&minor);
            let n0 = col_norm(&[minor[0][0], minor[1][0], minor[2][0]]);
            let n1 = col_norm(&[minor[0][1], minor[1][1], minor[2][1]]);
            let n2 = col_norm(&[minor[0][2], minor[1][2], minor[2][2]]);
            let scale = n0 * n1 * n2;
            let slope = if scale > 0.0 { d / scale } else { 0.0 };
            let kappa = if best_det > 0.0 {
                1.0 / best_det
            } else {
                1.0e12
            };
            (kappa, slope)
        }
        None => (1.0e12, 0.0),
    }
}

/// The Euclidean norm of a float 3-vector.
fn col_norm(v: &[f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// A certified boundary-stratum seed.
struct CertifiedSeed {
    /// The 4-D centre (the float Newton root on the stratum).
    center: [f64; 4],
    /// The model point at the centre.
    model: Point3,
    /// The certified 4-D cell (the free-axis box plus the degenerate fixed
    /// axis).
    cell4: [Interval; 4],
    /// The Krawczyk certificate.
    cert: Certificate,
}

/// A cell the seed search could not certify (typed, never a guess).
struct UnresolvedCell {
    /// The 3-D free-axis box that stayed unresolved.
    box3: [Interval; 3],
    /// The stratum descriptor (fixed axis and value).
    fixed_axis: usize,
    fixed_value: f64,
}

/// The result of one boundary-stratum seed search.
struct StratumSeeds {
    /// Certified seeds found on the stratum.
    certified: Vec<CertifiedSeed>,
    /// A refusal encountered while searching (kept as an unresolved witness).
    unresolved: Option<UnresolvedCell>,
}

// ---------------------------------------------------------------------------
// Float seed prediction (uncertified predictor; certification is separate)
// ---------------------------------------------------------------------------

/// The 3-D float Newton iterate over a stratum. `None` when Newton does not
/// converge to a residual below the predictor tolerance.
fn newton_on_stratum(
    form: &FForm,
    fixed_axis: usize,
    fixed_value: f64,
    start: [f64; 3],
) -> Option<[f64; 3]> {
    let mut x = start;
    let mut last_step = f64::INFINITY;
    for _ in 0..128 {
        let x4 = embed_point(fixed_axis, fixed_value, &x);
        let f = form.residual_f(&x4);
        let mut m = [[0.0; 3]; 3];
        {
            let cols = form.partial_columns_f(&x4);
            let mut k = 0usize;
            for c in 0..4 {
                if c == fixed_axis {
                    continue;
                }
                for r in 0..3 {
                    m[r][k] = cols[c][r];
                }
                k += 1;
            }
        }
        let inv = invert3(&m)?;
        let mut dx = [0.0; 3];
        for r in 0..3 {
            let mut acc = 0.0f64;
            for c in 0..3 {
                acc += inv[r][c] * f[c];
            }
            dx[r] = -acc;
        }
        let mut step_mag = 0.0f64;
        for &d in &dx {
            step_mag = step_mag.max(d.abs());
        }
        if !step_mag.is_finite() {
            return None;
        }
        if step_mag > last_step {
            // Not contracting: abandon this start.
            return None;
        }
        last_step = step_mag;
        for (xi, d) in x.iter_mut().zip(dx.iter()) {
            *xi += *d;
        }
        let mut residual_mag = 0.0f64;
        for &fi in &f {
            residual_mag = residual_mag.max(fi.abs());
        }
        if step_mag < 1.0e-12 && residual_mag < 1.0e-8 {
            return Some(x);
        }
    }
    None
}

/// Embeds a 3-D free-coordinate point into the 4-D chart.
fn embed_point(fixed_axis: usize, fixed_value: f64, x: &[f64; 3]) -> [f64; 4] {
    let mut out = [fixed_value; 4];
    let mut k = 0usize;
    for j in 0..4 {
        if j == fixed_axis {
            continue;
        }
        out[j] = x[k];
        k += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Boundary seeding and the continuation driver
// ---------------------------------------------------------------------------

/// Searches one boundary stratum (one product coordinate fixed to one box
/// endpoint) for certified seeds.
fn seed_stratum(
    form: &FForm,
    fixed_axis: usize,
    fixed_value: f64,
    box4: &[(f64, f64); 4],
    params: &Ssi4Parameters,
) -> StratumSeeds {
    // Free-axis windows.
    let mut free_lo = [0.0f64; 3];
    let mut free_hi = [0.0f64; 3];
    {
        let mut k = 0usize;
        for j in 0..4 {
            if j == fixed_axis {
                continue;
            }
            free_lo[k] = box4[j].0;
            free_hi[k] = box4[j].1;
            k += 1;
        }
    }
    let grid = params.seed_grid_per_axis.max(2);
    let mut starts: Vec<[f64; 3]> = Vec::new();
    let mut t = [0.0f64; 3];
    generate_grid(&mut starts, &mut t, 0, grid, &free_lo, &free_hi);

    let mut seeds: Vec<CertifiedSeed> = Vec::new();
    let mut unresolved: Option<UnresolvedCell> = None;
    for start in starts {
        let Some(root) = newton_on_stratum(form, fixed_axis, fixed_value, start) else {
            continue;
        };
        if seeds.iter().any(|s| {
            let d = chart_distance(&root, &free_of(&s.center, fixed_axis));
            d < 1.0e-6
        }) {
            continue;
        }
        // Metric box: radii from the free-axis column scales at the root.
        let x4 = embed_point(fixed_axis, fixed_value, &root);
        let cols = form.partial_columns_f(&x4);
        let mut radii = [0.0f64; 3];
        let mut ok = true;
        {
            let mut k = 0usize;
            for c in 0..4 {
                if c == fixed_axis {
                    continue;
                }
                let scale = col_norm(&cols[c]);
                if !scale.is_finite() || scale <= 0.0 {
                    ok = false;
                } else {
                    radii[k] = params.metric_radius / scale;
                }
                k += 1;
            }
        }
        if !ok {
            continue;
        }
        let Some(box3) = box_around(root, radii) else {
            continue;
        };
        let system = Ssi3System::new(form.clone(), fixed_axis, fixed_value);
        let mut budget = params.seed_budget;
        match krawczyk::<3>(&system, &box3, &mut budget) {
            Ok(Certified {
                value: KrawczykProof::Unique,
                cert,
            }) => {
                let cell4 = embed_box(fixed_axis, fixed_value, &box3);
                seeds.push(CertifiedSeed {
                    center: x4,
                    model: Point3::new(x4[0], x4[1], x4[2]),
                    cell4,
                    cert,
                });
                // Correct the model point from the A carrier.
                if let Some(s) = seeds.last_mut() {
                    s.model = Point3::new(
                        form.a.point_f(s.center[0], s.center[1])[0],
                        form.a.point_f(s.center[0], s.center[1])[1],
                        form.a.point_f(s.center[0], s.center[1])[2],
                    );
                }
            }
            Ok(Certified {
                value: KrawczykProof::NoRoot,
                ..
            }) => {}
            Err(refusal) => {
                if unresolved.is_none() {
                    unresolved = Some(UnresolvedCell {
                        box3,
                        fixed_axis,
                        fixed_value,
                    });
                }
                let _ = refusal;
            }
        }
    }
    StratumSeeds {
        certified: seeds,
        unresolved,
    }
}

/// Embeds a 3-D free-axis box into the 4-D chart box (degenerate fixed axis).
fn embed_box(fixed_axis: usize, fixed_value: f64, b3: &[Interval; 3]) -> [Interval; 4] {
    let mut out = [iv(fixed_value); 4];
    let mut k = 0usize;
    for j in 0..4 {
        if j == fixed_axis {
            continue;
        }
        out[j] = b3[k];
        k += 1;
    }
    out
}

/// The free-axis coordinates of a 4-D point.
fn free_of(x: &[f64; 4], fixed_axis: usize) -> [f64; 3] {
    let mut out = [0.0f64; 3];
    let mut k = 0usize;
    for j in 0..4 {
        if j == fixed_axis {
            continue;
        }
        out[k] = x[j];
        k += 1;
    }
    out
}

/// The chart-space distance of two 3-vectors.
fn chart_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    (d0 * d0 + d1 * d1 + d2 * d2).sqrt()
}

/// Deterministically enumerates the product grid of `grid` evenly spaced
/// values per free axis into `out`.
fn generate_grid(
    out: &mut Vec<[f64; 3]>,
    acc: &mut [f64; 3],
    depth: usize,
    grid: usize,
    lo: &[f64; 3],
    hi: &[f64; 3],
) {
    if depth == 3 {
        out.push(*acc);
        return;
    }
    for i in 0..grid {
        let f = if grid == 1 {
            0.5
        } else {
            (i as f64) / ((grid - 1) as f64)
        };
        acc[depth] = lo[depth] + f * (hi[depth] - lo[depth]);
        generate_grid(out, acc, depth + 1, grid, lo, hi);
    }
}

/// The model-space distance between two model points.
fn model_distance(a: &Point3, b: &Point3) -> f64 {
    (*a - *b).magnitude()
}

/// Converts a certified interval cell into a [`WitnessCell`]. `None` when an
/// interval is not finite (cannot be recorded).
fn witness_from_intervals(iv4: &[Interval; 4]) -> Option<WitnessCell> {
    let mut out = [(0.0f64, 0.0f64); 4];
    for (k, i) in iv4.iter().enumerate() {
        let (lo, hi) = (i.inf(), i.sup());
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return None;
        }
        out[k] = (lo, hi);
    }
    Some(WitnessCell::new(out[0], out[1], out[2], out[3]))
}

/// The frame and first model point of a trace at a certified seed. `None`
/// when the tangent cannot be computed (a degenerate branch start).
fn trace_begin(
    form: &FForm,
    seed: &CertifiedSeed,
    cell4_box: &[(f64, f64); 4],
) -> Option<(ParallelotopeFrame, Point3)> {
    let cols = form.partial_columns_f(&seed.center);
    // A certified transversal subset must exist at the seed (the branch is
    // transverse there); otherwise the branch start is degenerate.
    let _ = choose_free_axis(&cols)?;
    let mut jac = [[0.0; 4]; 3];
    for r in 0..3 {
        for c in 0..4 {
            jac[r][c] = cols[c][r];
        }
    }
    let mut frame = ParallelotopeFrame::from_jacobian(seed.center, &jac)?;
    // Orient the tangent into the cell from the boundary stratum the seed sits
    // on.
    for j in 0..4 {
        let (lo, hi) = cell4_box[j];
        let tol = (hi - lo) * 1.0e-6;
        let at_lo = (seed.center[j] - lo).abs() <= tol;
        let at_hi = (seed.center[j] - hi).abs() <= tol;
        if at_lo || at_hi {
            // The fixed axis of the seed stratum: choose the direction that
            // moves into the box.
            let dir = if at_lo { 1.0 } else { -1.0 };
            if frame.tangent[j] * dir < 0.0 {
                frame.tangent = negate4(&frame.tangent);
                frame.transversal = negate_completion(&frame.transversal);
            }
        }
    }
    let model = model_point(form, &seed.center);
    Some((frame, model))
}

/// The model point of the first carrier at a 4-D chart coordinate.
fn model_point(form: &FForm, x: &[f64; 4]) -> Point3 {
    let p = form.a.point_f(x[0], x[1]);
    Point3::new(p[0], p[1], p[2])
}

/// Negates a 4-vector.
fn negate4(v: &[f64; 4]) -> [f64; 4] {
    [-v[0], -v[1], -v[2], -v[3]]
}

/// Negates an orthonormal completion (keeps it an orthonormal completion).
fn negate_completion(t: &[[f64; 4]; 3]) -> [[f64; 4]; 3] {
    [negate4(&t[0]), negate4(&t[1]), negate4(&t[2])]
}

/// The per-axis model column scale of the F-form at a chart point.
fn column_scales(form: &FForm, x: &[f64; 4]) -> [f64; 4] {
    let cols = form.partial_columns_f(x);
    [
        col_norm(&cols[0]),
        col_norm(&cols[1]),
        col_norm(&cols[2]),
        col_norm(&cols[3]),
    ]
}

/// The typed unresolved outcome for a cell whose certification was attempted at
/// `center` (κ / cell / slope diagnostics; never a guess).
fn unresolved_outcome(form: &FForm, center: &[f64; 4], cell: WitnessCell) -> InteractionOutcome {
    let (kappa, slope) = unresolved_diagnostics(form, center);
    InteractionOutcome::Unresolved { kappa, cell, slope }
}

/// Traces one branch from a certified seed by the parallelotope θρ step,
/// appending certified samples and frames. Returns the samples, the frames,
/// and the first typed unresolved witness if the trace could not certify
/// everywhere.
fn trace_branch(
    form: &FForm,
    seed: &CertifiedSeed,
    cell4_box: &[(f64, f64); 4],
    params: &Ssi4Parameters,
) -> (
    Vec<ChartSample>,
    Vec<ParallelotopeFrame>,
    Option<InteractionOutcome>,
) {
    let mut samples: Vec<ChartSample> = Vec::new();
    let mut frames: Vec<ParallelotopeFrame> = Vec::new();
    let mut witness: Option<InteractionOutcome> = None;

    let Some((mut frame, start_model)) = trace_begin(form, seed, cell4_box) else {
        // A degenerate branch start: typed unresolved over the seed cell.
        if let Some(cell) = witness_from_intervals(&seed.cell4) {
            witness = Some(unresolved_outcome(form, &seed.center, cell));
        }
        return (samples, frames, witness);
    };

    // Record the seed as the first ordered sample.
    if let Some(cell) = witness_from_intervals(&seed.cell4) {
        samples.push(ChartSample {
            cell,
            chart: seed.center,
            centre: start_model,
            cert: seed.cert.clone(),
        });
        frames.push(frame);
    } else {
        return (samples, frames, witness);
    }

    let mut step = params.theta_step;
    let closure_radius = params.closure_radius_steps * params.theta_step;
    let mut centre = seed.center;

    for _ in 0..params.max_steps {
        // Predict along the tangent by a model-space arc advance.
        let scales = column_scales(form, &centre);
        let speed = tangent_speed(form, &centre, &frame.tangent);
        if !speed.is_finite() || speed <= 0.0 {
            break;
        }
        let chart_step = step / speed;
        let predicted = frame.predict(chart_step);
        // A prediction that leaves the chart cell terminates an open branch.
        if !inside_box(&predicted, cell4_box) {
            break;
        }
        // The parallelotope radii: model radius over the per-axis scales.
        let mut radii = [0.0f64; 4];
        let mut scale_ok = true;
        for j in 0..4 {
            let s = scales[j];
            if !s.is_finite() || s <= 0.0 {
                scale_ok = false;
            } else {
                radii[j] = params.metric_radius / s;
            }
        }
        if !scale_ok {
            break;
        }
        // The parallelotope box about the prediction (also the witness cell if
        // the operator refuses).
        let Some(step_cell) = box_around(predicted, radii) else {
            break;
        };
        // The θρ corrector: hyperplane τ·(x − c) = 0 through the prediction.
        let mut rhs = 0.0f64;
        for j in 0..4 {
            rhs += frame.tangent[j] * predicted[j];
        }
        let system = Ssi4System::new(form.clone(), frame.tangent, rhs);
        let mut budget = params.step_budget;
        let verdict = theta_rho_step(&system, predicted, radii, &mut budget);
        match verdict {
            StepVerdict::Certified { cell, center, cert } => {
                let Some(wcell) = witness_from_intervals(&cell) else {
                    break;
                };
                let model = model_point(form, &center);
                // Reframe at the certified centre.
                let cols = form.partial_columns_f(&center);
                let mut jac = [[0.0; 4]; 3];
                for r in 0..3 {
                    for c in 0..4 {
                        jac[r][c] = cols[c][r];
                    }
                }
                let Some(new_frame) = ParallelotopeFrame::from_jacobian(center, &jac) else {
                    // The branch turned singular at this certified cell: typed
                    // unresolved over the certified cell.
                    if let Some(wcell) = witness_from_intervals(&cell) {
                        witness = Some(unresolved_outcome(form, &center, wcell));
                    }
                    break;
                };
                // Keep the orientation continuous with the marching direction.
                let mut tangent = new_frame.tangent;
                let mut dot = 0.0f64;
                for j in 0..4 {
                    dot += tangent[j] * frame.tangent[j];
                }
                if dot < 0.0 {
                    tangent = negate4(&tangent);
                }
                let new_frame = ParallelotopeFrame {
                    center,
                    tangent,
                    transversal: new_frame.transversal,
                };
                samples.push(ChartSample {
                    cell: wcell,
                    chart: center,
                    centre: model,
                    cert,
                });
                frames.push(new_frame);
                centre = center;
                frame = new_frame;
                // A closed branch returns to the seed's model point.
                if samples.len() > 4 && model_distance(&model, &start_model) <= closure_radius {
                    break;
                }
            }
            StepVerdict::NoRoot => {
                // The prediction overshot or the branch terminated: shrink the
                // step and retry (deterministic halving).
                step *= 0.5;
                if step < 1.0e-12 * params.metric_radius {
                    break;
                }
            }
            StepVerdict::Refused(_) => {
                // The operator refused on the parallelotope box: typed
                // unresolved over that box.
                if let Some(wcell) = witness_from_intervals(&step_cell) {
                    witness = Some(unresolved_outcome(form, &predicted, wcell));
                }
                break;
            }
        }
    }

    (samples, frames, witness)
}

/// The model-space speed of the branch when moving at the unit chart tangent
/// (the norm of the A-carrier velocity).
fn tangent_speed(form: &FForm, x: &[f64; 4], tangent: &[f64; 4]) -> f64 {
    let (au, av) = form.a.partials_f(x[0], x[1]);
    let v = [
        tangent[0] * au[0] + tangent[1] * av[0],
        tangent[0] * au[1] + tangent[1] * av[1],
        tangent[0] * au[2] + tangent[1] * av[2],
    ];
    col_norm(&v)
}

/// Whether a chart point lies (strictly, up to an ulp-scale slack) inside the
/// product box.
fn inside_box(x: &[f64; 4], box4: &[(f64, f64); 4]) -> bool {
    for j in 0..4 {
        let slack = (box4[j].1 - box4[j].0).abs() * 1.0e-9;
        if x[j] < box4[j].0 - slack || x[j] > box4[j].1 + slack {
            return false;
        }
    }
    true
}

/// The certified restricted-pair solve: seed every boundary stratum, trace each
/// certified seed's branch, and emit the certified chart curve.
///
/// When no certified sample can be produced, the returned curve is empty and
/// its `witness` is a typed [`InteractionOutcome::Unresolved`] (never a guess);
/// a partial trace records the first unresolved cell as the witness.
pub fn certify_restricted_pair(
    a: RestrictedChart,
    b: RestrictedChart,
    cell: WitnessCell,
    params: &Ssi4Parameters,
    budget: &mut Budget,
) -> Outcome<CertifiedChartCurve> {
    let form = FForm { a, b };
    let cell4_box = cell_box(&cell);
    let mut certified_seeds: Vec<CertifiedSeed> = Vec::new();
    let mut seed_unresolved: Option<UnresolvedCell> = None;

    // Fixed stratum order: side (A then B), axis (0 then 1), endpoint (lo then
    // hi) — determinism.
    for side in 0..2 {
        let lo = carrier_param_box(&cell, side);
        for axis in 0..2 {
            let fixed_axis = 2 * side + axis;
            for endpoint in 0..2 {
                let fixed_value = if endpoint == 0 {
                    lo[axis].0
                } else {
                    lo[axis].1
                };
                let result = seed_stratum(&form, fixed_axis, fixed_value, &cell4_box, params);
                for seed in result.certified {
                    // Deduplicate seeds that certify the same model point
                    // (e.g. a periodic seam).
                    let dup = certified_seeds
                        .iter()
                        .any(|s| model_distance(&s.model, &seed.model) < 1.0e-6);
                    if !dup {
                        certified_seeds.push(seed);
                    }
                }
                if seed_unresolved.is_none() {
                    seed_unresolved = result.unresolved;
                }
            }
        }
    }

    let mut samples: Vec<ChartSample> = Vec::new();
    let mut frames: Vec<ParallelotopeFrame> = Vec::new();
    let mut witness: Option<InteractionOutcome> = None;

    for seed in certified_seeds {
        let (mut s, mut f, trace_witness) = trace_branch(&form, &seed, &cell4_box, params);
        samples.append(&mut s);
        frames.append(&mut f);
        if witness.is_none() {
            witness = trace_witness;
        }
    }

    if samples.is_empty() && witness.is_none() {
        if let Some(unresolved) = seed_unresolved {
            // A seed stratum refused certification: a typed unresolved witness
            // over the stratum cell.
            let cell4 = embed_box(
                unresolved.fixed_axis,
                unresolved.fixed_value,
                &unresolved.box3,
            );
            if let Some(wcell) = witness_from_intervals(&cell4) {
                let center = mid_of_intervals(&cell4);
                witness = Some(unresolved_outcome(&form, &center, wcell));
            }
        }
    }

    if samples.is_empty() && witness.is_none() {
        // No certified branch could be produced anywhere: a typed unresolved
        // witness over the searched cell (never a fabricated answer).
        let center = mid_of(&cell4_box);
        witness = Some(unresolved_outcome(&form, &center, cell));
    }

    let curve = CertifiedChartCurve {
        samples,
        tangent_frames: frames,
        witness,
    };
    Ok(Certified::new(curve, interval_certificate(budget)))
}

/// The midpoint of a 4-D interval box.
fn mid_of_intervals(iv4: &[Interval; 4]) -> [f64; 4] {
    [
        0.5 * (iv4[0].inf() + iv4[0].sup()),
        0.5 * (iv4[1].inf() + iv4[1].sup()),
        0.5 * (iv4[2].inf() + iv4[2].sup()),
        0.5 * (iv4[3].inf() + iv4[3].sup()),
    ]
}

/// The midpoint of the product box.
fn mid_of(box4: &[(f64, f64); 4]) -> [f64; 4] {
    [
        0.5 * (box4[0].0 + box4[0].1),
        0.5 * (box4[1].0 + box4[1].1),
        0.5 * (box4[2].0 + box4[2].1),
        0.5 * (box4[3].0 + box4[3].1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct::bie::fixtures::{plane_sphere_fixture, sweep_plane_fixture};
    use truck_base::evidence::{Certified as EvCertified, Refusal};
    use truck_geometry::prelude::Vector3;

    /// The plane × sphere F-form of the fixture kit (plane on the A side).
    fn plane_sphere_form() -> FForm {
        let fixture = plane_sphere_fixture();
        FForm {
            a: RestrictedChart::from_plane(fixture.plane),
            b: RestrictedChart::from_sphere(fixture.sphere),
        }
    }

    /// The sweep × plane F-form of the fixture kit (sweep on the A side).
    fn sweep_plane_form() -> FForm {
        let fixture = sweep_plane_fixture();
        let sweep = fixture.sweep;
        let carrier = RestrictedChart::circular_sweep(
            sweep.spine_from,
            sweep.spine_to,
            sweep.radius_start,
            sweep.radius_end,
            sweep.s0,
            sweep.s1,
            sweep.v0,
            sweep.v1,
        )
        .unwrap_or_else(|| unreachable!("the fixture sweep spine is non-degenerate"));
        FForm {
            a: carrier,
            b: RestrictedChart::from_plane(fixture.plane),
        }
    }

    fn must_certified<T>(out: Outcome<T>) -> T {
        match out {
            Ok(EvCertified { value, .. }) => value,
            Err(e) => unreachable!("unit-test witness must certify, got {e:?}"),
        }
    }

    #[test]
    fn column_choice_finds_transversal_subset() {
        // The plane × sphere fixture: the branch point with the free axis t=0
        // has certified 3×3 minors on the transversal subsets.
        let form = plane_sphere_form();
        // On the fixture circle: (u, v) = (√3, 0) in the plane z = 1, and
        // (s, t) = (π/3, 0) on the sphere of radius 2.
        let s3 = 3.0_f64.sqrt();
        let x = [s3, 0.0, std::f64::consts::FRAC_PI_3, 0.0];
        let cols = form.partial_columns_f(&x);
        let free = choose_free_axis(&cols).unwrap_or_else(|| {
            unreachable!("a transverse point must certify a transversal subset")
        });
        // The chosen subset's exact (R′) sign is certified nonzero...
        let minor = minor3_of_jacobian(&cols, free);
        let sign = minor_sign_expansion(&minor)
            .unwrap_or_else(|| unreachable!("the chosen subset must certify its minor sign"));
        assert_ne!(sign, CertifiedSign::Zero);
        // ... and the box-level (R′) sign over a small parallelotope about the
        // point also certifies (the subset stays transversal on the box).
        // H-3: certified-box half-width in parameter units, not a length.
        let radius = 1.0e-3; // H-3: box half-width about the branch point, parameter units
        let box4 = box_around(x, [radius; 4])
            .unwrap_or_else(|| unreachable!("finite radii produce a finite box"));
        let cols_iv = form.partial_columns_iv(&box4);
        let minor_iv = minor3_iv_of_jacobian(&cols_iv, free);
        let sign_iv = minor_det_sign_iv(&minor_iv)
            .unwrap_or_else(|| unreachable!("the subset must stay transversal over the small box"));
        assert_ne!(sign_iv, CertifiedSign::Zero);
        assert_eq!(
            sign_iv, sign,
            "the box-level and point-level signs must agree"
        );
    }

    #[test]
    fn minor_sign_predicate_matches_expansion() {
        // The (R′) predicate must never contradict the exact expansion sign of
        // the same minor, on constructed 3×3 systems including an exact zero.
        let systems: [[[f64; 3]; 3]; 6] = [
            [[2.0, 0.0, 1.0], [1.0, 3.0, 0.0], [0.0, 1.0, 2.0]],
            [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 10.0]],
            [[1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
            [[-2.0, 0.5, 0.0], [0.0, -3.0, 1.0], [1.0, 0.0, -4.0]],
            [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            [[1.0e-10, 1.0, 2.0], [3.0, -4.0, 5.0], [6.0, 7.0, -1.0e-10]],
        ];
        for m in systems.iter() {
            // The landed expansion exact sign, computed independently here.
            let exact = det3_expansion(m).sign();
            match minor_sign_expansion(m) {
                Some(sign) => assert_eq!(
                    sign, exact,
                    "(R′) returned {sign:?} but the exact expansion sign is {exact:?}"
                ),
                None => assert_eq!(
                    exact,
                    CertifiedSign::Zero,
                    "(R′) refused a non-degenerate minor with exact sign {exact:?}"
                ),
            }
        }
    }

    #[test]
    fn boundary_seed_resolves_exf_and_fxe() {
        // E×F: the sweep's v = v0 ring edge (A-edge) × the plane face (B-face).
        let ef_form = sweep_plane_form();
        let ef_cell = sweep_plane_fixture().cell;
        let ef_box = cell_box(&ef_cell);
        let params = Ssi4Parameters::default();
        let ef = seed_stratum(&ef_form, 1, ef_box[1].0, &ef_box, &params);
        assert!(
            !ef.certified.is_empty(),
            "the sweep v = v0 edge × plane stratum must certify seeds"
        );
        if let Some(seed) = ef.certified.first() {
            // Expected: the plane z = 3/4 meets the sweep trajectory v = 0 at
            // the station s* = 3/4, at the plane coordinates (5/8, 0).
            // H-3: unit-scale parameter tolerance on the seed centre.
            let tol = 1.0e-4; // H-3: seed-centre tolerance, parameter units
            assert!((seed.center[0] - 0.75).abs() <= tol, "seed station");
            assert!((seed.center[2] - 0.625).abs() <= tol, "seed plane u");
            assert!(seed.center[3].abs() <= tol, "seed plane v");
        }
        // F×E: the sphere's t = 0 edge (B-edge) × the plane face (A-face).
        let fe_form = plane_sphere_form();
        let fe_cell = plane_sphere_fixture().cell;
        let fe_box = cell_box(&fe_cell);
        let fe = seed_stratum(&fe_form, 3, fe_box[3].0, &fe_box, &params);
        assert!(
            !fe.certified.is_empty(),
            "the plane face × sphere t = 0 edge stratum must certify seeds"
        );
        if let Some(seed) = fe.certified.first() {
            // Expected: the section circle meets the sphere t = 0 meridian at
            // (√3, 0) in the plane and latitude π/3 on the sphere.
            let s3 = 3.0_f64.sqrt();
            // H-3: unit-scale parameter tolerance on the seed centre.
            let tol = 1.0e-4; // H-3: seed-centre tolerance, parameter units
            assert!((seed.center[0] - s3).abs() <= tol, "seed plane u");
            assert!(seed.center[1].abs() <= tol, "seed plane v");
            assert!(
                (seed.center[2] - std::f64::consts::FRAC_PI_3).abs() <= tol,
                "seed sphere latitude"
            );
        }
    }

    #[test]
    fn continuation_tracks_known_curve() {
        // The plane × sphere circle of the fixture kit, tracked end to end by
        // the parallelotope continuation.
        let fixture = plane_sphere_fixture();
        let form = plane_sphere_form();
        let mut budget = Budget::new(0, 0, 0);
        let curve = must_certified(certify_restricted_pair(
            form.a,
            form.b,
            fixture.cell,
            &Ssi4Parameters::default(),
            &mut budget,
        ));
        assert!(
            curve.witness.is_none(),
            "the transverse fixture must certify without a witness, got {:?}",
            curve.witness
        );
        assert!(curve.samples.len() >= 64, "the circle needs many samples");
        // H-3: model-space tolerance on the certified sample centres.
        let tol = 1.5e-2; // H-3: distance tolerance from the section circle, model units
        let circle_centre = Point3::new(0.0, 0.0, 1.0);
        let circle_radius = 3.0_f64.sqrt();
        for sample in &curve.samples {
            let p = sample.centre;
            let radial = Vector3::new(p.x, p.y, 0.0).magnitude();
            let axial = p.z - circle_centre.z;
            let err = (radial - circle_radius).hypot(axial);
            assert!(
                err <= tol,
                "sample centre escaped the section circle by {err}"
            );
        }
        assert_eq!(curve.tangent_frames.len(), curve.samples.len());
        // Ordered along the branch: the sphere longitude t advances
        // monotonically and covers the whole ring.
        let mut last_t = f64::NEG_INFINITY;
        let mut first_t = f64::INFINITY;
        for sample in &curve.samples {
            let mid = 0.5 * (sample.cell.t.0 + sample.cell.t.1);
            assert!(
                mid > last_t - 1.0e-9,
                "samples must be ordered along the branch, got {mid} after {last_t}"
            );
            last_t = mid;
            first_t = first_t.min(mid);
        }
        // H-3: period slack for the closed ring in longitude units.
        let span = std::f64::consts::TAU;
        assert!(first_t < 0.1, "the ring must start near the seed seam");
        assert!(
            (span - last_t).abs() < 0.1,
            "the ring must be tracked end to end, last longitude {last_t}"
        );
        // Closed: the final model point returns to the start.
        let first = &curve.samples[0];
        let last = &curve.samples[curve.samples.len() - 1];
        let gap = model_distance(&first.centre, &last.centre);
        // H-3: model-space loop-closure tolerance.
        let close = 3.0 * tol; // H-3: loop-closure distance tolerance, model units
        assert!(gap <= close, "the loop must close, gap {gap}");
    }

    #[test]
    fn unresolved_elsewhere_is_typed() {
        // A tangent pair (the plane z = 2 tangent to the sphere of radius 2 at
        // its pole) is degenerate: the solver returns a typed Unresolved
        // witness, never a guess and never a panic.
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
        let cell = WitnessCell::new(
            (-2.0, 2.0),
            (-2.0, 2.0),
            (0.0, std::f64::consts::PI),
            (0.0, std::f64::consts::TAU),
        );
        let mut budget = Budget::new(0, 0, 0);
        let curve = must_certified(certify_restricted_pair(
            RestrictedChart::from_plane(plane),
            RestrictedChart::from_sphere(sphere),
            cell,
            &Ssi4Parameters::default(),
            &mut budget,
        ));
        assert!(
            curve.samples.is_empty(),
            "a tangent pair certifies no branch samples"
        );
        assert!(
            matches!(&curve.witness, Some(InteractionOutcome::Unresolved { .. })),
            "a tangent pair must produce a typed Unresolved witness, got {:?}",
            curve.witness
        );
        // The typed witness maps onto the landed refusal taxonomy.
        if let Some(outcome) = curve.witness {
            assert!(
                matches!(
                    outcome.clone().into_landed_refusal(),
                    Some(Refusal::NumericallyUnresolved { .. })
                ),
                "the Unresolved witness must map onto the landed NumericallyUnresolved refusal"
            );
        }
    }
}
