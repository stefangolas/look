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
use truck_base::evidence::{
    Budget, Certificate, Certified, Method, Outcome, PropMap, Refusal, UnresolvedWitness,
};
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
    /// Whether the CFP-007 sample-constant machinery runs (caller-side float
    /// preconditioner reuse across consecutive samples plus the bounded
    /// ε-inflation ladder). The machinery is a pure cost optimization: it only
    /// ever re-runs the landed Krawczyk operator on the caller's chosen `Y`
    /// and box radius, so verdicts and certificate boxes are unchanged whether
    /// it is enabled or not (the reuse path is deterministic because the
    /// invalidation rule is).
    pub sample_constants: bool,
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
            sample_constants: true,
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

/// Scales a float 3-vector by a scalar.
fn scale3(s: f64, v: &[f64; 3]) -> [f64; 3] {
    [s * v[0], s * v[1], s * v[2]]
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

    /// The float second partials `(X_{00}, X_{01}, X_{11})` of the carrier at
    /// the parameter point `(p0, p1)`: the three symmetric second partial
    /// 3-vectors of the surface map. These feed the BIE-004 polar-augmented
    /// Jacobian row (the derivative of a Jacobian minor along the chart).
    fn second_partials_f(&self, p0: f64, p1: f64) -> [[f64; 3]; 3] {
        match self {
            RestrictedChart::Plane { .. } => [[0.0; 3]; 3],
            RestrictedChart::Sphere { center: _, radius } => {
                let (ss, cs) = p0.sin_cos();
                let (st, ct) = p1.sin_cos();
                let r = *radius;
                [
                    [-r * ss * ct, -r * ss * st, -r * cs],
                    [-r * cs * st, r * cs * ct, 0.0],
                    [-r * ss * ct, -r * ss * st, 0.0],
                ]
            }
            RestrictedChart::Cylinder { center: _, radius } => {
                let (ss, cs) = p0.sin_cos();
                let r = *radius;
                [[-r * cs, -r * ss, 0.0], [0.0; 3], [0.0; 3]]
            }
            RestrictedChart::CircularSweep(sweep) => {
                let r = sweep.radius_at(p0);
                let dr = (sweep.radius_end - sweep.radius_start) / (sweep.s1 - sweep.s0);
                let angle = std::f64::consts::TAU * p1;
                let (sa, ca) = angle.sin_cos();
                let e0 = [sweep.ring0.x, sweep.ring0.y, sweep.ring0.z];
                let e1 = [sweep.ring1.x, sweep.ring1.y, sweep.ring1.z];
                let two_pi = std::f64::consts::TAU;
                let ring_t = [
                    -two_pi * sa * e0[0] + two_pi * ca * e1[0],
                    -two_pi * sa * e0[1] + two_pi * ca * e1[1],
                    -two_pi * sa * e0[2] + two_pi * ca * e1[2],
                ];
                let ring = [
                    ca * e0[0] + sa * e1[0],
                    ca * e0[1] + sa * e1[1],
                    ca * e0[2] + sa * e1[2],
                ];
                let ring_tt = [
                    -two_pi * two_pi * ring[0],
                    -two_pi * two_pi * ring[1],
                    -two_pi * two_pi * ring[2],
                ];
                // The straight spine and the linear scale radius make the
                // second s-partials vanish (BIE-004 closure fixture algebra).
                [[0.0; 3], scale3(dr, &ring_t), scale3(r, &ring_tt)]
            }
        }
    }

    /// The interval second partials `(X_{00}, X_{01}, X_{11})` over the
    /// parameter box `(p0, p1)` (outward-rounded).
    fn second_partials_iv(&self, p0: Interval, p1: Interval) -> [[Interval; 3]; 3] {
        match self {
            RestrictedChart::Plane { .. } => [[iv(0.0); 3]; 3],
            RestrictedChart::Sphere { center: _, radius } => {
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                let (st, ct) = (isin(p1), icos(p1));
                [
                    [-r * ss * ct, -r * ss * st, -r * cs],
                    [-r * cs * st, r * cs * ct, iv(0.0)],
                    [-r * ss * ct, -r * ss * st, iv(0.0)],
                ]
            }
            RestrictedChart::Cylinder { center: _, radius } => {
                let r = iv(*radius);
                let (ss, cs) = (isin(p0), icos(p0));
                [[-r * cs, -r * ss, iv(0.0)], [iv(0.0); 3], [iv(0.0); 3]]
            }
            RestrictedChart::CircularSweep(sweep) => {
                let span = iv(sweep.s1 - sweep.s0);
                let tau = (p0 - iv(sweep.s0)) / span;
                let r =
                    iv(sweep.radius_start) + (iv(sweep.radius_end) - iv(sweep.radius_start)) * tau;
                let dr = (iv(sweep.radius_end) - iv(sweep.radius_start)) / span;
                let angle = iv(std::f64::consts::TAU) * p1;
                let (sa, ca) = (isin(angle), icos(angle));
                let two_pi = iv(std::f64::consts::TAU);
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
                let ring_tt = [
                    -(two_pi * two_pi) * ring[0],
                    -(two_pi * two_pi) * ring[1],
                    -(two_pi * two_pi) * ring[2],
                ];
                [
                    [iv(0.0); 3],
                    [dr * ring_t[0], dr * ring_t[1], dr * ring_t[2]],
                    [r * ring_tt[0], r * ring_tt[1], r * ring_tt[2]],
                ]
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
    pub(crate) fn residual_f(&self, x: &[f64; 4]) -> [f64; 3] {
        let pa = self.a.point_f(x[0], x[1]);
        let pb = self.b.point_f(x[2], x[3]);
        sub3_f(&pa, &pb)
    }

    /// The float 3×4 Jacobian columns `(X_u, X_v, −X_s, −X_t)` at the point.
    pub(crate) fn partial_columns_f(&self, x: &[f64; 4]) -> [[f64; 3]; 4] {
        let (au, av) = self.a.partials_f(x[0], x[1]);
        let (bs, bt) = self.b.partials_f(x[2], x[3]);
        [au, av, [-bs[0], -bs[1], -bs[2]], [-bt[0], -bt[1], -bt[2]]]
    }

    /// The interval residual over the 4-D box (outward-rounded).
    pub(crate) fn residual_iv(&self, x: &[Interval; 4]) -> [Interval; 3] {
        let pa = self.a.point_iv(x[0], x[1]);
        let pb = self.b.point_iv(x[2], x[3]);
        sub3(&pa, &pb)
    }

    /// The interval 3×4 Jacobian columns over the 4-D box.
    pub(crate) fn partial_columns_iv(&self, x: &[Interval; 4]) -> [[Interval; 3]; 4] {
        let (au, av) = self.a.partials_iv(x[0], x[1]);
        let (bs, bt) = self.b.partials_iv(x[2], x[3]);
        [au, av, neg3(&bs), neg3(&bt)]
    }

    /// The float second partial columns: `out[a][j]` is the 3-vector
    /// `∂(∂F/∂x_a)/∂x_j = ∂²F/∂x_a∂x_j` of the residual at the chart point
    /// (the derivative of the 3×4 Jacobian columns along the chart). Columns
    /// of the two carriers are decoupled, so the mixed (side-A axis, side-B
    /// axis) entries vanish exactly.
    pub(crate) fn second_columns_f(&self, x: &[f64; 4]) -> [[[f64; 3]; 4]; 4] {
        let ha = self.a.second_partials_f(x[0], x[1]);
        let hb = self.b.second_partials_f(x[2], x[3]);
        let mut out = [[[0.0; 3]; 4]; 4];
        for (i, j, h) in [(0usize, 0usize, &ha[0]), (0, 1, &ha[1]), (1, 1, &ha[2])] {
            out[i][j] = *h;
            out[j][i] = *h;
        }
        for (i, j, h) in [(2usize, 2usize, &hb[0]), (2, 3, &hb[1]), (3, 3, &hb[2])] {
            let neg = [-h[0], -h[1], -h[2]];
            out[i][j] = neg;
            out[j][i] = neg;
        }
        out
    }

    /// The interval second partial columns over the 4-D box (outward-rounded):
    /// `out[a][j]` encloses `∂²F/∂x_a∂x_j` on the box.
    pub(crate) fn second_columns_iv(&self, x: &[Interval; 4]) -> [[[Interval; 3]; 4]; 4] {
        let ha = self.a.second_partials_iv(x[0], x[1]);
        let hb = self.b.second_partials_iv(x[2], x[3]);
        let mut out = [[[Interval::EMPTY; 3]; 4]; 4];
        for k in 0..4 {
            for l in 0..4 {
                out[k][l] = [iv(0.0); 3];
            }
        }
        for (i, j, h) in [(0usize, 0usize, &ha[0]), (0, 1, &ha[1]), (1, 1, &ha[2])] {
            out[i][j] = *h;
            out[j][i] = *h;
        }
        for (i, j, h) in [(2usize, 2usize, &hb[0]), (2, 3, &hb[1]), (3, 3, &hb[2])] {
            let neg = [-h[0], -h[1], -h[2]];
            out[i][j] = neg;
            out[j][i] = neg;
        }
        out
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

// ---------------------------------------------------------------------------
// CFP-007-SAMPLE-CONSTANTS: caller-side float-preconditioner reuse + ε-inflation
// ---------------------------------------------------------------------------
//
// Two landed facts make most of the steady-state per-sample Krawczyk input cost
// redundant on a branch (build spec §3):
//
// 1. **Y-independence.** The strict inclusion test `K(X) ⊂ int(X)` proves a
//    unique root in `X` for ANY invertible float preconditioner `Y`; `Y`
//    affects tightness only, never soundness. The continuation loop therefore
//    caches the `Y` of the last accepted sample and re-uses it for the next
//    sample, where conditioning varies slowly; on an inclusion failure the
//    cache is invalidated (never re-used after a failure) and a fresh `Y` is
//    recomputed at the new point.
// 2. **ε-inflation (Rump).** A sample that fails inclusion at radius `r` is
//    often certifiable at `2r`; the retry sequence `r·2^k` for `k = 0..=3`
//    (four attempts, low-first) is a FIXED, BOUNDED named constant — never
//    adaptive, never geometry-dependent. If the whole ladder fails the sample
//    falls back to the landed bisection discipline ([`theta_rho_step`]).
//
// Both are SFC-clean: floats shape the *search* (which `Y`, which box radius),
// and every emitted certificate is the landed Krawczyk operator's own interval
// proof of the box. `num/krawczyk.rs` is FROZEN — all of this is caller-side.

/// The fixed ε-inflation multipliers of one continuation sample: the box
/// radius is `r · 2^k` for `k = 0..=3` (radii `r`, `2r`, `4r`, `8r`), four
/// attempts total, low-first. A named constant schedule — never adaptive,
/// never geometry-dependent (H-3).
const EPS_INFLATION_RADII: [f64; 4] = [1.0, 2.0, 4.0, 8.0]; // H-3: radii r·2^k, k=0..=3

/// The ε-inflation attempt count of one sample: exactly the length of the
/// named constant ladder [`EPS_INFLATION_RADII`] (H-3).
const EPS_INFLATION_ATTEMPTS: usize = 4; // H-3: fixed bounded attempt count per sample

/// The one accepted float preconditioner of a branch trace: the float inverse
/// `Y` that certified a continuation sample, and the index of that sample.
///
/// `Y` is a float search aid (H-6: it never enters evidence); the certified
/// statement is always the box the landed operator proves.
#[derive(Clone, Copy, Debug)]
struct AcceptedPreconditioner<const N: usize> {
    /// The accepted float inverse `Y`.
    y: [[f64; N]; N],
    /// The index of the continuation sample `y` certified.
    sample: usize,
}

/// The deterministic preconditioner-reuse cache of one branch trace
/// (CFP-007). A `Y` is accepted exactly when the sample it was used for
/// certified; it is re-used for a later sample only while the accepted sample
/// is the immediately preceding one, and is dropped forever on the first
/// inclusion failure (the invalidation rule).
#[derive(Clone, Copy, Debug, Default)]
struct PreconditionerCache<const N: usize> {
    /// The accepted preconditioner, absent before the first certified sample.
    accepted: Option<AcceptedPreconditioner<N>>,
}

/// A [`KrawczykSystem`] adapter that pins the preconditioner to a fixed float
/// `Y` (CFP-007 reuse): the operator runs its inclusion test against the SAME
/// `Y` instead of recomputing the wrapped system's preconditioner at every box
/// midpoint. `f_point`/`jacobian` delegate to the wrapped system, so the
/// certificate remains the landed operator's own interval proof (sound by
/// Y-independence).
struct FixedPreconditioner<'a, S: KrawczykSystem<N>, const N: usize> {
    /// The wrapped system.
    system: &'a S,
    /// The fixed float preconditioner the operator uses.
    y: [[f64; N]; N],
}

impl<S: KrawczykSystem<N>, const N: usize> KrawczykSystem<N> for FixedPreconditioner<'_, S, N> {
    fn f_point(&self, x: &[f64; N]) -> [Interval; N] {
        self.system.f_point(x)
    }

    fn jacobian(&self, b: &[Interval; N]) -> [[Interval; N]; N] {
        self.system.jacobian(b)
    }

    fn preconditioner(&self, _x: &[f64; N]) -> Option<[[f64; N]; N]> {
        Some(self.y)
    }
}

/// The outcome of one single-box Krawczyk inclusion test
/// ([`one_shot_krawczyk`]).
enum OneShotOutcome {
    /// The operator proved exactly one solution in the box.
    Unique { cert: Certificate },
    /// The operator proved no solution in the searched box.
    NoRoot,
    /// The single-box test was inconclusive: the operator could not certify
    /// strict inclusion one-shot (it would need to bisect, which the caller's
    /// retry ladder owns).
    Inconclusive,
}

/// One single-box Krawczyk run: the landed operator over `cell` with a
/// zero-subdivision local budget, so it certifies strict inclusion one-shot,
/// proves no root, or refuses — it never bisects. `fixed` pins the
/// preconditioner to the caller's `Y`; `None` lets the system compute its own
/// fresh preconditioner at the box midpoint (the landed path). Sound in every
/// case: the returned proofs are the operator's own.
fn one_shot_krawczyk<S: KrawczykSystem<N>, const N: usize>(
    system: &S,
    cell: &[Interval; N],
    fixed: Option<[[f64; N]; N]>,
) -> OneShotOutcome {
    let mut budget = Budget::new(0, 0, 0);
    let outcome = match fixed {
        Some(y) => {
            let pinned = FixedPreconditioner { system, y };
            krawczyk(&pinned, cell, &mut budget)
        }
        None => krawczyk(system, cell, &mut budget),
    };
    match outcome {
        Ok(Certified {
            value: KrawczykProof::Unique,
            cert,
        }) => OneShotOutcome::Unique { cert },
        Ok(Certified {
            value: KrawczykProof::NoRoot,
            ..
        }) => OneShotOutcome::NoRoot,
        Err(_) => OneShotOutcome::Inconclusive,
    }
}

/// The float midpoint of an interval box (the operator's own midpoint
/// convention).
fn box_midpoint<const N: usize>(box_: &[Interval; N]) -> [f64; N] {
    std::array::from_fn(|a| {
        let c = box_[a];
        0.5 * (c.inf() + c.sup())
    })
}

/// The refusal a θρ-style step answers when no certified box can be built
/// about the center (mirrors [`theta_rho_step`]).
fn step_box_refusal<const N: usize>(budget: &Budget) -> StepVerdict<N> {
    StepVerdict::Refused(Refusal::NumericallyUnresolved {
        spent: *budget,
        witness: UnresolvedWitness::KrawczykIndeterminate,
    })
}

/// The CFP-007 certified θρ step: preconditioner reuse + bounded ε-inflation
/// over the parallelotope about `center`, caller-side. Returns the same
/// [`StepVerdict`] vocabulary as [`theta_rho_step`], which the continuation
/// loop consumes unchanged.
///
/// Per sample, deterministically:
///
/// 1. **Reuse attempt.** If the cache holds a `Y` that certified the
///    immediately preceding sample, the operator runs one-shot over the base
///    parallelotope with that cached `Y`. Strict inclusion certifies the box
///    (and re-accepts the same `Y`); an empty image intersection is the
///    operator's certified `NoRoot`; anything else is an inclusion failure,
///    which invalidates the cached `Y` forever (never reused after a failure).
/// 2. **Fresh retry + ε-inflation.** A fresh `Y` is computed at the box
///    midpoint and the fixed ladder [`EPS_INFLATION_RADII`] (`r`, `2r`, `4r`,
///    `8r`) is attempted one-shot, low-first, each against that fresh `Y`. The
///    first strict inclusion certifies its (inflated) box.
/// 3. **Landed fallback.** If every bounded attempt is inconclusive, the
///    frozen [`theta_rho_step`] runs (the landed bisection discipline) — the
///    behavior is bit-identical to a run with `sample_constants` disabled.
///
/// `budget` is the caller's per-sample step budget; the one-shot attempts use
/// their own zero-subdivision local budget and only the landed fallback spends
/// from `budget`.
fn certified_theta_rho_step<S: KrawczykSystem<N>, const N: usize>(
    system: &S,
    center: [f64; N],
    radii: [f64; N],
    budget: &mut Budget,
    cache: &mut PreconditionerCache<N>,
    sample_index: usize,
) -> StepVerdict<N> {
    let base = match box_around(center, radii) {
        Some(cell) => cell,
        None => return step_box_refusal(budget),
    };

    // (1) reuse attempt with the cached Y of the immediately preceding sample.
    if let Some(accepted) = cache.accepted.filter(|a| a.sample + 1 == sample_index) {
        match one_shot_krawczyk(system, &base, Some(accepted.y)) {
            OneShotOutcome::Unique { cert } => {
                cache.accepted = Some(AcceptedPreconditioner {
                    y: accepted.y,
                    sample: sample_index,
                });
                let mut cert = cert;
                // The one-shot ran on a local zero budget; the caller's step
                // budget was untouched, so the certificate's budget_left is
                // the true remaining budget.
                cert.budget_left = *budget;
                return StepVerdict::Certified {
                    cell: base,
                    center,
                    cert,
                };
            }
            OneShotOutcome::NoRoot => return StepVerdict::NoRoot,
            OneShotOutcome::Inconclusive => {
                // An inclusion failure with the reused Y: never reuse it again
                // (the invalidation rule). Fall through to a fresh retry.
                cache.accepted = None;
            }
        }
    }

    // (2) fresh Y at the base midpoint, then the fixed inflation ladder.
    let midpoint = box_midpoint(&base);
    let Some(fresh) = system.preconditioner(&midpoint) else {
        // No float inverse at the point (singular derivative): the operator's
        // bisection-on-None discipline is the landed path.
        return theta_rho_step(system, center, radii, budget);
    };
    for k in 0..EPS_INFLATION_ATTEMPTS {
        let scale = EPS_INFLATION_RADII[k];
        let scaled: [f64; N] = std::array::from_fn(|a| radii[a] * scale);
        let Some(cell) = box_around(center, scaled) else {
            break;
        };
        match one_shot_krawczyk(system, &cell, Some(fresh)) {
            OneShotOutcome::Unique { cert } => {
                cache.accepted = Some(AcceptedPreconditioner {
                    y: fresh,
                    sample: sample_index,
                });
                let mut cert = cert;
                // See the reuse arm: the one-shot did not spend the caller's
                // step budget, so budget_left is the true remaining budget.
                cert.budget_left = *budget;
                return StepVerdict::Certified { cell, center, cert };
            }
            OneShotOutcome::NoRoot => return StepVerdict::NoRoot,
            OneShotOutcome::Inconclusive => {}
        }
    }

    // (3) the landed bisection discipline.
    theta_rho_step(system, center, radii, budget)
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
    // CFP-007: the branch-local preconditioner cache (reuse across consecutive
    // samples; deterministic state, invalidated on every inclusion failure).
    let mut preconditioner_cache = PreconditionerCache::<4>::default();

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
        let sample_index = samples.len();
        let verdict = if params.sample_constants {
            certified_theta_rho_step(
                &system,
                predicted,
                radii,
                &mut budget,
                &mut preconditioner_cache,
                sample_index,
            )
        } else {
            theta_rho_step(&system, predicted, radii, &mut budget)
        };
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

// ---------------------------------------------------------------------------
// CL-006-SOLVER-ENTRY: the certified impl of the dependency-inverted entry.
// ---------------------------------------------------------------------------

/// The certified restricted-pair engine (BIE-002) behind the
/// [`RestrictedSolverEntry`](truck_evidence::contact::solver_entry::RestrictedSolverEntry)
/// impl: the restricted-pair solve with its parameters. The engine is a
/// `truck-certified` construction the boolean crates cannot name, so the impl
/// is registered into the `truck-evidence` registry slot and the funnel never
/// names this type.
#[derive(Clone, Debug, Default)]
pub struct RestrictedPairSolver {
    /// The restricted-pair solve parameters (the certified geometry scale and
    /// continuation cadence of the landed engine).
    pub params: Ssi4Parameters,
}

/// Spend since entry: the entry budget minus what remains.
fn spent_since(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// The typed unresolved refusal a non-solvable restricted pair answers (the
/// engine never ran for it): the landed `NumericallyUnresolved` with the
/// Krawczyk witness — never a guess, never `NonCanonicalCarrier`.
fn engine_default_refusal(initial: &Budget, budget: &Budget) -> Refusal {
    Refusal::NumericallyUnresolved {
        spent: spent_since(initial, budget),
        witness: UnresolvedWitness::KrawczykIndeterminate,
    }
}

/// Maps a certified 4-D parameter cell onto the entry's parameter-cell record.
fn entry_cell(cell: WitnessCell) -> truck_evidence::contact::solver_entry::ParameterCell {
    truck_evidence::contact::solver_entry::ParameterCell {
        u: cell.u,
        v: cell.v,
        s: cell.s,
        t: cell.t,
    }
}

/// A canonical surface carrier restricted to the certified engine's canonical
/// family (plane / sphere / cylinder). `None` for a cone, torus, or placed
/// carrier: those stay outside the restricted envelope of this entry.
fn restricted_face_chart(
    surface: &truck_geometry::recognize::CanonicalSurface,
) -> Option<RestrictedChart> {
    use truck_geometry::recognize::CanonicalSurface;
    match surface {
        CanonicalSurface::Plane(plane) => Some(RestrictedChart::from_plane(*plane)),
        CanonicalSurface::Sphere(sphere) => Some(RestrictedChart::from_sphere(*sphere)),
        CanonicalSurface::Cylinder(cylinder) => Some(RestrictedChart::from_cylinder(*cylinder)),
        _ => None,
    }
}

/// Reduces a stored whole-sweep value to the restricted engine's pole-free
/// circular-section sweep chart, when the stored sweep is the class BIE-002
/// certifies: a straight (line) spine with a circular ring whose radius
/// follows the profile law's scale. `None` for every other sweep — a polygonal
/// (prismatic) profile is not the continuous circular-section limit, and
/// certifying a different surface than the stored one would be a guess.
///
/// The stored sweep realizes a polygonal profile ring (the constructive
/// kernel's profiles are polygons); a windowed sweep face whose profile ring is
/// numerically a circle is certified as its continuous circular-section limit
/// (`X(s, v) = C(s) + radius(s)·(cos 2πv·e0 + sin 2πv·e1)` over the window) —
/// exactly the unit shape of BIE-000 fixture 3.
fn restricted_sweep_chart(
    sweep: &truck_geometry::constructive::SpineFrameSweep,
) -> Option<RestrictedChart> {
    use truck_geometry::canonical::Curve;
    use truck_geometry::constructive::ProfileLaw;
    use truck_geometry::specifieds::Line;

    let recipe = sweep.recipe();
    let s0 = sweep.s0();
    let s1 = sweep.s1();
    let v0 = sweep.v0();
    let v1 = sweep.v1();
    if !(s1 > s0 && v1 > v0 && v0 >= 0.0 && v1 <= 1.0) {
        return None;
    }
    // The straight spine: `Curve::Line` over `[0, 1]`, so the spine point at
    // station `s` is the affine point `from + s·(to − from)`.
    let Curve::Line(Line(from, to)) = &*recipe.spine else {
        return None;
    };
    let axis = *to - *from;
    let length = axis.magnitude();
    if !(length.is_finite() && length > 0.0) {
        return None;
    }
    // The profile ring and its scale law. The ring radius at a station is the
    // profile's circumradius about the sweep axis (the profile origin) times
    // the law's scalar at that station.
    let (profile, scalar) = match &recipe.profile_law {
        ProfileLaw::Constant(profile) => (profile, None),
        ProfileLaw::Scale { profile, scale } => (profile, Some(scale)),
        ProfileLaw::LinearCorrespondence { .. } => return None,
    };
    // The circumradius about the axis, requiring the ring to be numerically a
    // circle: every vertex at the same radius, so the continuous circular-
    // section limit is well defined (a prismatic profile has unequal radii and
    // is refused, never silently replaced by a round tube).
    let circumradius = circumradius_about_axis(profile)?;
    let ring_radius = |s: f64| -> Option<f64> {
        match scalar {
            None => Some(circumradius),
            Some(law) => {
                let c = law.at(s);
                if c.is_finite() && c > 0.0 {
                    Some(circumradius * c)
                } else {
                    None
                }
            }
        }
    };
    let radius_start = ring_radius(s0)?;
    let radius_end = ring_radius(s1)?;
    RestrictedChart::circular_sweep(
        *from + axis * s0,
        *from + axis * s1,
        radius_start,
        radius_end,
        s0,
        s1,
        v0,
        v1,
    )
}

/// The circumradius of a profile ring about the sweep axis (the profile-plane
/// origin). `None` when a vertex is at the axis, non-finite, or the ring is not
/// numerically a circle about the axis.
fn circumradius_about_axis(profile: &truck_geometry::constructive::Profile2D) -> Option<f64> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for vertex in &profile.vertices {
        let radius = (vertex.x * vertex.x + vertex.y * vertex.y).sqrt();
        if !radius.is_finite() || radius == 0.0 {
            return None;
        }
        if radius < lo {
            lo = radius;
        }
        if radius > hi {
            hi = radius;
        }
    }
    if !(lo.is_finite() && hi.is_finite()) {
        return None;
    }
    // Ring-circularity slack: a stored circle-approximation polygon places
    // every vertex at the same radius up to float rounding, while a prismatic
    // ring spans radii from its inradius to its circumradius.
    let ring_tol = 1.0e-9; // H-3: dimensionless ring-circularity relative slack
    if hi - lo > ring_tol * hi {
        return None;
    }
    Some(0.5 * (lo + hi))
}

/// The product parameter cell of a sweep×sweep pair: each side's stored window
/// (station × ring) is one carrier's parameter box, the lhs sweep the A side.
fn sweep_sweep_cell(
    a: &truck_geometry::constructive::SpineFrameSweep,
    b: &truck_geometry::constructive::SpineFrameSweep,
) -> WitnessCell {
    WitnessCell::new(
        (a.s0(), a.s1()),
        (a.v0(), a.v1()),
        (b.s0(), b.s1()),
        (b.v0(), b.v1()),
    )
}

/// The typed unresolved verdict of a sweep×sweep pair whose sweeps the
/// restricted family cannot express (no chart exists, so the engine never
/// ran): the κ / cell / slope witness with the degenerate diagnostics of a
/// chart-less pair (the `unresolved_diagnostics` no-certified-minor values) —
/// never a refusal arm, never a guess.
fn sweep_sweep_unresolved(
    a: &truck_geometry::constructive::SpineFrameSweep,
    b: &truck_geometry::constructive::SpineFrameSweep,
    initial: &Budget,
    budget: &Budget,
) -> truck_evidence::contact::solver_entry::RestrictedSolve {
    use truck_evidence::contact::solver_entry::RestrictedSolve;
    RestrictedSolve::Unresolved {
        kappa: 1.0e12,
        cell: entry_cell(sweep_sweep_cell(a, b)),
        slope: 0.0,
        spent: spent_since(initial, budget),
    }
}

impl RestrictedPairSolver {
    /// Maps one certified restricted-pair solve onto the entry vocabulary: a
    /// certified interaction branch adapts its chart samples; a certified
    /// solve with no branch, an untyped witness, or a refusal passes the typed
    /// witness / refusal through (never a panic).
    fn solve_chart_pair(
        &self,
        a: RestrictedChart,
        b: RestrictedChart,
        cell: WitnessCell,
        initial: &Budget,
        budget: &mut Budget,
    ) -> truck_evidence::contact::solver_entry::RestrictedSolve {
        use truck_evidence::contact::solver_entry::{CertifiedChart, ChartSample, RestrictedSolve};
        match certify_restricted_pair(a, b, cell, &self.params, budget) {
            Ok(Certified { value: curve, cert }) => {
                if !curve.samples.is_empty() {
                    // A certified interaction branch: adapt the certified chart
                    // samples onto the entry vocabulary (H-2).
                    let samples = curve
                        .samples
                        .iter()
                        .map(|s| ChartSample {
                            cell: entry_cell(s.cell),
                            chart: s.chart,
                            centre: s.centre,
                            certificate: s.cert.clone(),
                        })
                        .collect();
                    RestrictedSolve::Certified(CertifiedChart {
                        samples,
                        certificate: cert.clone(),
                    })
                } else {
                    // No certified branch: the typed witness of the engine.
                    match curve.witness {
                        Some(InteractionOutcome::Unresolved { kappa, cell, slope }) => {
                            RestrictedSolve::Unresolved {
                                kappa,
                                cell: entry_cell(cell),
                                slope,
                                spent: spent_since(initial, budget),
                            }
                        }
                        Some(InteractionOutcome::Refused(refusal)) => {
                            RestrictedSolve::Refused(refusal)
                        }
                        Some(InteractionOutcome::Certified(_)) | None => {
                            RestrictedSolve::Refused(engine_default_refusal(initial, budget))
                        }
                    }
                }
            }
            Err(refusal) => RestrictedSolve::Refused(refusal),
        }
    }
}

impl truck_evidence::contact::solver_entry::RestrictedSolverEntry for RestrictedPairSolver {
    /// Certifies one restricted sweep pair: the sweep side(s) reduced to their
    /// circular-section charts, the canonical side to its restricted chart,
    /// over the product cell of the two strata's windows. Both-sides-sweep
    /// pairs (CL-004-SWEEP-SWEEP) reduce both sweeps; a sweep the restricted
    /// family cannot express answers the typed Unresolved (κ/cell/slope), the
    /// rest of a non-restricted pair (a cone/torus/placed carrier, a sweep
    /// against a non-face stratum) answers the typed unresolved refusal — the
    /// engine never ran for it (H-1: total, never a panic).
    fn certify_sweep_pair(
        &self,
        lhs: &truck_evidence::contact::BoundedStratum,
        rhs: &truck_evidence::contact::BoundedStratum,
        budget: &mut Budget,
    ) -> truck_evidence::contact::solver_entry::RestrictedSolve {
        use truck_evidence::contact::solver_entry::RestrictedSolve;
        use truck_evidence::contact::BoundedStratum;

        let initial = *budget;
        // CL-004-SWEEP-SWEEP: both sides sweep. Each side reduces to its
        // circular-section chart over its stored window (the lhs sweep the A
        // side, the rhs the B side — fixed deterministic order); a sweep the
        // restricted family cannot express (a polygonal ring, a curved spine)
        // answers the typed Unresolved, never a refusal and never a guess.
        if let (BoundedStratum::Sweep { sweep: a }, BoundedStratum::Sweep { sweep: b }) = (lhs, rhs)
        {
            let a_chart = match restricted_sweep_chart(a) {
                Some(chart) => chart,
                None => return sweep_sweep_unresolved(a, b, &initial, budget),
            };
            let b_chart = match restricted_sweep_chart(b) {
                Some(chart) => chart,
                None => return sweep_sweep_unresolved(a, b, &initial, budget),
            };
            return self.solve_chart_pair(
                a_chart,
                b_chart,
                sweep_sweep_cell(a, b),
                &initial,
                budget,
            );
        }
        // Identify the sweep stratum and the canonical face stratum, order-
        // insensitively; anything else stays outside the restricted family.
        let (sweep, face) = match (lhs, rhs) {
            (BoundedStratum::Sweep { sweep }, BoundedStratum::Face { .. }) => (sweep, rhs),
            (BoundedStratum::Face { .. }, BoundedStratum::Sweep { sweep }) => (sweep, lhs),
            _ => return RestrictedSolve::Refused(engine_default_refusal(&initial, budget)),
        };
        let BoundedStratum::Face {
            surface,
            u_range,
            v_range,
        } = face
        else {
            return RestrictedSolve::Refused(engine_default_refusal(&initial, budget));
        };
        let Some(a) = restricted_sweep_chart(sweep) else {
            return RestrictedSolve::Refused(engine_default_refusal(&initial, budget));
        };
        let Some(b) = restricted_face_chart(surface) else {
            return RestrictedSolve::Refused(engine_default_refusal(&initial, budget));
        };
        // The sweep is the A side: its (station, ring) window is the A
        // parameter cell; the face's `(u, v)` box is the B side.
        let cell = WitnessCell::new(
            (sweep.s0(), sweep.s1()),
            (sweep.v0(), sweep.v1()),
            *u_range,
            *v_range,
        );
        self.solve_chart_pair(a, b, cell, &initial, budget)
    }
}

/// Registers the certified restricted-pair engine into the `truck-evidence`
/// registry slot (CL-006-SOLVER-ENTRY decision 3). The call is the explicit,
/// `#[ctor]`-free registration the kernel's entry point wires: with the engine
/// registered the funnel dispatches restricted sweep pairs to the certified
/// engine; without it, the funnel answers today's typed unresolved.
///
/// Set-once: a second registration while the slot is occupied refuses.
pub fn register_restricted_pair_solver(
) -> Result<(), truck_evidence::contact::solver_entry::RestrictedSolverSetError> {
    truck_evidence::contact::solver_entry::set_restricted_solver(RestrictedPairSolver::default())
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

    // -----------------------------------------------------------------------
    // CL-006-SOLVER-ENTRY: the dependency-inverted entry tests.
    // -----------------------------------------------------------------------

    /// Registers the certified engine once for the whole test binary (tests
    /// run on parallel threads against one process-wide registry slot).
    static REGISTER_ENGINE: std::sync::Once = std::sync::Once::new();
    fn ensure_engine_registered() {
        REGISTER_ENGINE.call_once(|| {
            let _ = register_restricted_pair_solver();
        });
    }

    /// The straight-spine `Scale`-of-a-circle `SpineFrameSweep` unit shape of
    /// the BIE-000 fixture kit's sweep × plane transversal pair, stored as a
    /// windowed whole-sweep value over one ring edge of a 32-edge circle
    /// polygon. The restricted reduction certifies this sweep as its
    /// continuous circular-section limit (radius 1 → 1/2 over the window).
    fn circle_sweep_fixture() -> Option<truck_geometry::constructive::SpineFrameSweep> {
        use truck_base::cgmath64::Point2;
        use truck_geometry::canonical::Curve;
        use truck_geometry::constructive::{
            FrameLaw, Profile2D, ProfileLaw, ScalarLaw, SpineFrameRecipe,
        };
        use truck_geometry::specifieds::Line;
        let vertices: Vec<Point2> = (0..32)
            .map(|i| {
                let angle = std::f64::consts::TAU * (i as f64) / (32.0);
                Point2::new(angle.cos(), angle.sin())
            })
            .collect();
        let profile = Profile2D::try_closed(vertices).ok()?;
        let spine = Box::new(Curve::Line(Line(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Scale {
                profile,
                scale: ScalarLaw::Linear {
                    start: 1.0,
                    end: 0.5,
                },
            },
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        // One ring edge of the 32-gon: the v-window a stored sweep face spans.
        truck_geometry::constructive::SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 1.0 / 32.0)
            .ok()
    }

    /// A straight-spine, constant-`Scale`-of-a-circle `SpineFrameSweep`: an
    /// `sides`-gon ring on the radius-`ring_radius` circle (first vertex at
    /// profile angle 0, so the reduced chart's ring phase matches the stored
    /// one) about the `+z` spine through `(axis_x, axis_y)`, over the `(s, v)`
    /// window `[0, 1] × [v0, v1]`.
    fn circle_ring_sweep(
        sides: u32,
        ring_radius: f64,
        axis_x: f64,
        axis_y: f64,
        v0: f64,
        v1: f64,
    ) -> Option<truck_geometry::constructive::SpineFrameSweep> {
        use truck_base::cgmath64::Point2;
        use truck_geometry::canonical::Curve;
        use truck_geometry::constructive::{FrameLaw, Profile2D, ProfileLaw, SpineFrameRecipe};
        use truck_geometry::specifieds::Line;
        let vertices: Vec<Point2> = (0..sides)
            .map(|i| {
                let angle = std::f64::consts::TAU * (i as f64) / (sides as f64);
                Point2::new(ring_radius * angle.cos(), ring_radius * angle.sin())
            })
            .collect();
        let profile = Profile2D::try_closed(vertices).ok()?;
        let spine = Box::new(Curve::Line(Line(
            Point3::new(axis_x, axis_y, 0.0),
            Point3::new(axis_x, axis_y, 1.0),
        )));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Constant(profile),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        truck_geometry::constructive::SpineFrameSweep::try_new(recipe, 0.0, 1.0, v0, v1).ok()
    }

    /// The dyadic sweep×sweep pair of CL-004's certified dispatch fixture: two
    /// straight-spine, constant-radius circular-section sweeps (8-gon rings)
    /// whose z-parallel axes are offset by the dyadic distance 1 and whose
    /// ring windows hold the two cylinders' transversal intersection line.
    ///
    /// A is the radius-1 cylinder arc about the z-axis over ring edge 0
    /// (window `v ∈ [0, 1/8]`, angles 0..45°); B is the radius-1/2 cylinder
    /// arc about the axis `x = 1` over ring edge 2 (window `v ∈ [2/8, 3/8]`,
    /// angles 90..135°). The reduced cylinders (radii 1 and 1/2, axes at
    /// distance 1) meet in the two lines `x = 7/8, y = ±√15/8`; exactly the
    /// `+y` line (A-angle `arctan(√15/7) ≈ 29°`, B-angle `≈ 104.5°`) lies
    /// inside both ring windows, so the pair certifies one transversal branch.
    fn sweep_sweep_pair_fixture() -> Option<(
        truck_geometry::constructive::SpineFrameSweep,
        truck_geometry::constructive::SpineFrameSweep,
    )> {
        let a = circle_ring_sweep(8, 1.0, 0.0, 0.0, 0.0, 1.0 / 8.0)?;
        let b = circle_ring_sweep(8, 0.5, 1.0, 0.0, 2.0 / 8.0, 3.0 / 8.0)?;
        Some((a, b))
    }

    /// The transverse plane `z = 3/4` of the sweep × plane fixture.
    fn fixture_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.75),
            Point3::new(1.0, 0.0, 0.75),
            Point3::new(0.0, 1.0, 0.75),
        )
    }

    /// The sweep stratum of the fixture kit's transversal pair.
    fn sweep_stratum_fixture() -> Option<truck_evidence::contact::BoundedStratum> {
        let sweep = circle_sweep_fixture()?;
        Some(truck_evidence::contact::sweep_stratum(sweep))
    }

    /// The plane face stratum of the fixture kit's transversal pair.
    fn plane_stratum_fixture() -> truck_evidence::contact::BoundedStratum {
        use truck_geometry::recognize::CanonicalSurface;
        truck_evidence::contact::BoundedStratum::Face {
            surface: CanonicalSurface::Plane(fixture_plane()),
            u_range: (-2.0, 2.0),
            v_range: (-2.0, 2.0),
        }
    }

    #[test]
    fn entry_trait_implemented_by_certified_engine() {
        // The trait object resolves to the certified impl and returns a
        // certified chart for the BIE-000 fixture kit's transversal pair
        // (CL-006 test 1).
        ensure_engine_registered();
        let lhs = match sweep_stratum_fixture() {
            Some(lhs) => lhs,
            None => return,
        };
        let rhs = plane_stratum_fixture();
        let mut budget = Budget::new(4096, 0, 0);
        let Some(solve) = truck_evidence::contact::solver_entry::dispatch_restricted_sweep(
            &lhs,
            &rhs,
            &mut budget,
        ) else {
            unreachable!("the engine is registered by ensure_engine_registered");
        };
        let truck_evidence::contact::solver_entry::RestrictedSolve::Certified(chart) = solve else {
            unreachable!("the fixture transversal pair must certify, got {solve:?}");
        };
        assert!(
            !chart.samples.is_empty(),
            "the certified chart must carry certified samples"
        );
    }

    #[test]
    fn funnel_closes_sweep_pair_end_to_end() {
        // Through the FUNNEL entry (the dispatch site BIE-006 landed), a
        // sweep × canonical pair that today answers
        // NumericallyUnresolved-by-absence now returns the certified engine's
        // outcome — distinguishable from the absence default (CL-006 test 2).
        ensure_engine_registered();
        let sweep_stratum = match sweep_stratum_fixture() {
            Some(stratum) => stratum,
            None => return,
        };
        let plane_stratum = plane_stratum_fixture();
        let mut budget = Budget::new(4096, 0, 0);
        let out = truck_evidence::contact::contact(&sweep_stratum, &plane_stratum, &mut budget);
        let certified = match out {
            Ok(EvCertified { value, .. }) => value,
            Err(refusal) => {
                unreachable!(
                    "the registered engine must close the fixture pair, got refusal {refusal:?}"
                );
            }
        };
        // The certified contact came from the engine: one Arc1 record carrying
        // the certified chart samples (never the absence typed-unresolved).
        assert!(
            certified.contacts.len() == 1,
            "the certified sweep contact must carry one Arc1 record"
        );
        let record = certified
            .contacts
            .first()
            .unwrap_or_else(|| unreachable!("one record asserted above"));
        let truck_evidence::contact::ContactLocus::ValidatedBranchCover(cover) = &record.locus
        else {
            unreachable!("the engine's certified chart rides the ValidatedBranchCover locus");
        };
        assert!(
            !cover.points.is_empty(),
            "the certified cover must carry the engine's chart samples"
        );
    }

    #[test]
    fn sweep_sweep_pair_dispatches() {
        // CL-004 required test 1: two `SpineFrameSweep` carriers route to the
        // restricted solver and produce a certified verdict on a dyadic
        // fixture (the BIE-000 kit's sweep unit shapes, paired). Through the
        // FUNNEL entry (the CL-004 dispatch arm in `contact/mod.rs`), the
        // sweep × sweep pair certifies one Arc1 branch carrying the engine's
        // chart samples.
        ensure_engine_registered();
        let (a, b) = match sweep_sweep_pair_fixture() {
            Some(pair) => pair,
            None => return,
        };
        let lhs = truck_evidence::contact::sweep_stratum(a);
        let rhs = truck_evidence::contact::sweep_stratum(b);
        let mut budget = Budget::new(4096, 0, 0);
        let out = truck_evidence::contact::contact(&lhs, &rhs, &mut budget);
        let certified = match out {
            Ok(EvCertified { value, .. }) => value,
            Err(refusal) => {
                unreachable!(
                    "the sweep×sweep pair must certify through the restricted \
                     solver, got refusal {refusal:?}"
                );
            }
        };
        assert_eq!(
            certified.contacts.len(),
            1,
            "the certified sweep×sweep contact must carry one Arc1 record"
        );
        let record = certified
            .contacts
            .first()
            .unwrap_or_else(|| unreachable!("one record asserted above"));
        let truck_evidence::contact::ContactLocus::ValidatedBranchCover(cover) = &record.locus
        else {
            unreachable!("the engine's certified chart rides the ValidatedBranchCover locus");
        };
        assert!(
            !cover.points.is_empty(),
            "the certified sweep×sweep cover must carry the engine's chart samples"
        );
        // The machine check of the certified verdict: every certified sample
        // centre lies on both reduced circular-section surfaces — distance
        // `ring_radius` from each sweep's axis (radii 1 and 1/2, axes
        // distance 1). H-3: model-space tolerance on the certified centres.
        let tol = 5.0e-2; // H-3: distance tolerance from both cylinders, model units
        for point in &cover.points {
            let da = (point.x * point.x + point.y * point.y).sqrt();
            let db = ((point.x - 1.0) * (point.x - 1.0) + point.y * point.y).sqrt();
            assert!(
                (da - 1.0).abs() <= tol,
                "sample centre escaped the A cylinder: radial {da} at {point:?}"
            );
            assert!(
                (db - 0.5).abs() <= tol,
                "sample centre escaped the B cylinder: radial {db} at {point:?}"
            );
            assert!(
                (0.0..=1.0).contains(&point.z),
                "sample centre left the spine window: {point:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // CFP-007-SAMPLE-CONSTANTS: caller-side preconditioner reuse + ε-inflation
    // -----------------------------------------------------------------------

    use std::cell::Cell;
    use std::rc::Rc;

    /// A counting wrapper over the [`Ssi4System`] that tallies every FRESH
    /// float-preconditioner computation (a fixed-`Y` reuse never reaches the
    /// wrapped preconditioner). The counters are `Rc`-shared so a caller can
    /// observe the running total across consecutive samples of one branch.
    #[derive(Clone)]
    struct CountingSystem {
        /// The wrapped augmented system.
        inner: Ssi4System,
        /// Fresh-preconditioner computation count, shared across samples.
        fresh: Rc<Cell<usize>>,
        /// The last freshly computed `Y`, shared across samples.
        last_y: Rc<Cell<Option<[[f64; 4]; 4]>>>,
    }

    impl CountingSystem {
        /// Wraps `system` with fresh shared counters.
        fn new(system: Ssi4System) -> Self {
            CountingSystem {
                inner: system,
                fresh: Rc::new(Cell::new(0usize)),
                last_y: Rc::new(Cell::new(None)),
            }
        }

        /// A handle to the shared fresh-recompute counter.
        fn fresh(&self) -> Rc<Cell<usize>> {
            Rc::clone(&self.fresh)
        }

        /// A handle to the shared last-`Y` cell.
        fn last_y(&self) -> Rc<Cell<Option<[[f64; 4]; 4]>>> {
            Rc::clone(&self.last_y)
        }
    }

    impl KrawczykSystem<4> for CountingSystem {
        fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
            self.inner.f_point(x)
        }

        fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
            self.inner.jacobian(b)
        }

        fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
            let y = self.inner.preconditioner(x);
            if y.is_some() {
                self.fresh.set(self.fresh.get() + 1);
                self.last_y.set(y);
            }
            y
        }
    }

    /// The CFP-007 straight-branch fixture: the planes `z = x` (carrier A,
    /// `X_A(u, v) = (u, v, u)`) and `z = 1 − x` (carrier B,
    /// `X_B(s, t) = (s, t, 1 − s)`) meet along the straight branch
    /// `{u = s = 1/2, v = t}` in the 4-D chart. The augmented Jacobian is
    /// constant along the branch, so one sample's float preconditioner
    /// certifies every sample — a deterministic straight-branch fixture for
    /// the reuse tests.
    fn straight_branch_form() -> FForm {
        FForm {
            a: RestrictedChart::Plane {
                origin: Point3::new(0.0, 0.0, 0.0),
                u_axis: Vector3::new(1.0, 0.0, 1.0),
                v_axis: Vector3::new(0.0, 1.0, 0.0),
            },
            b: RestrictedChart::Plane {
                origin: Point3::new(0.0, 0.0, 1.0),
                u_axis: Vector3::new(1.0, 0.0, -1.0),
                v_axis: Vector3::new(0.0, 1.0, 0.0),
            },
        }
    }

    /// The unit tangent of the straight branch `(1/2, v, 1/2, v)`.
    fn straight_branch_tangent() -> [f64; 4] {
        [
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
        ]
    }

    /// The chart point of the straight branch at `v`.
    fn straight_branch_point(v: f64) -> [f64; 4] {
        [0.5, v, 0.5, v]
    }

    /// The augmented N=4 system of the straight branch through `v`, with the
    /// hyperplane `τ · (x − c) = 0` through the branch point at `v`.
    fn straight_branch_system(form: &FForm, v: f64) -> Ssi4System {
        let center = straight_branch_point(v);
        let tangent = straight_branch_tangent();
        let mut rhs = 0.0;
        for j in 0..4 {
            rhs += tangent[j] * center[j];
        }
        Ssi4System::new(form.clone(), tangent, rhs)
    }

    /// The identity 4×4 — an invertible but (for the straight-branch system)
    /// deliberately wrong preconditioner, whose Krawczyk image equals the box
    /// on the F-axes (never a strict inclusion).
    fn identity4() -> [[f64; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// Whether `verdict` is the certified step carrying exactly `cell`.
    fn certified_with_cell(verdict: &StepVerdict<4>, cell: &[Interval; 4]) -> bool {
        matches!(verdict, StepVerdict::Certified { cell: c, .. } if c == cell)
    }

    /// The box of half-width `r` about `center` (all axes).
    fn cube_cell(center: [f64; 4], r: f64) -> Option<[Interval; 4]> {
        box_around(center, [r; 4])
    }

    #[test]
    fn preconditioner_reused_across_consecutive_samples() {
        // The straight branch: two consecutive samples whose augmented systems
        // share a constant Jacobian. The first sample computes its float
        // preconditioner fresh (one recompute); the second sample's Krawczyk
        // call must receive the first sample's cached Y — observable through
        // the counting wrappers, whose fresh-recompute counters show the second
        // sample never recomputes.
        let form = straight_branch_form();
        let half_width = 0.05; // H-3: certified-box half-width, parameter units
        let radii = [half_width; 4];
        let sys0 = CountingSystem::new(straight_branch_system(&form, 0.3));
        let fresh0 = sys0.fresh();
        let last_y0 = sys0.last_y();
        let sys1 = CountingSystem::new(straight_branch_system(&form, 0.35));
        let fresh1 = sys1.fresh();
        let mut cache = PreconditionerCache::<4>::default();
        let mut budget = Budget::new(128, 0, 0);

        let point0 = straight_branch_point(0.3);
        let base0 = cube_cell(point0, half_width).expect("finite radii produce a box");
        let verdict0 = certified_theta_rho_step(&sys0, point0, radii, &mut budget, &mut cache, 1);
        assert!(
            certified_with_cell(&verdict0, &base0),
            "the first sample must certify its base parallelotope, got {verdict0:?}"
        );
        let first_y = last_y0.get().expect("the first sample computed a fresh Y");
        assert_eq!(
            fresh0.get(),
            1,
            "the first sample recomputes Y exactly once"
        );

        let point1 = straight_branch_point(0.35);
        let base1 = cube_cell(point1, half_width).expect("finite radii produce a box");
        let verdict1 = certified_theta_rho_step(&sys1, point1, radii, &mut budget, &mut cache, 2);
        assert!(
            certified_with_cell(&verdict1, &base1),
            "the second sample must certify its base parallelotope, got {verdict1:?}"
        );
        // The second sample never recomputed a preconditioner: its Krawczyk
        // call received the first sample's cached Y (bit-identical to it).
        assert_eq!(
            fresh1.get(),
            0,
            "a reused preconditioner must not be recomputed on the second sample"
        );
        assert_eq!(
            fresh0.get(),
            1,
            "two consecutive samples, one fresh preconditioner computation"
        );
        let cached = cache
            .accepted
            .expect("the accepted Y is cached after the second sample");
        assert_eq!(
            cached.sample, 2,
            "the cache records the certified sample index"
        );
        assert_eq!(cached.y, first_y, "the cached Y is the first sample's Y");
    }

    #[test]
    fn preconditioner_invalidated_on_inclusion_failure() {
        // Force an inclusion failure on the next sample by planting a
        // deliberately wrong (identity) cached Y: its Krawczyk image equals the
        // box on the F-axes, so the strict-inclusion one-shot fails. The next
        // attempt must use a freshly computed Y at the new point — observable
        // through the counting wrapper — and certify.
        let form = straight_branch_form();
        let half_width = 0.05; // H-3: certified-box half-width, parameter units
        let radii = [half_width; 4];
        let sys = CountingSystem::new(straight_branch_system(&form, 0.4));
        let fresh = sys.fresh();
        // A "previous sample" (index 1) whose Y is the identity: invertible but
        // never a strict inclusion for the straight-branch system.
        let mut cache = PreconditionerCache::<4> {
            accepted: Some(AcceptedPreconditioner {
                y: identity4(),
                sample: 1,
            }),
        };
        let mut budget = Budget::new(128, 0, 0);

        let point = straight_branch_point(0.4);
        let base = cube_cell(point, half_width).expect("finite radii produce a box");
        let verdict = certified_theta_rho_step(&sys, point, radii, &mut budget, &mut cache, 2);
        assert!(
            certified_with_cell(&verdict, &base),
            "the fresh-Y retry after the forced failure must certify, got {verdict:?}"
        );
        // The identity cached attempt fails inclusion; the retry recomputes Y
        // fresh at the new point exactly once.
        assert_eq!(
            fresh.get(),
            1,
            "after an inclusion failure the next attempt recomputes Y fresh"
        );
        let cached = cache.accepted.expect("the fresh Y is accepted and cached");
        assert_eq!(cached.sample, 2, "the cache records the recovered sample");
        assert_ne!(
            cached.y,
            identity4(),
            "the recovered Y is the freshly computed inverse, never the failed one"
        );
    }

    /// The F-C5 branch fixture: the plane × sphere circle of the BIE unit-shape
    /// kit, run whole by the restricted solver. Its branch sample count and
    /// certified box set are the F-C5-style branch data the optimization must
    /// leave unchanged.
    fn fc5_branch_fixture() -> (RestrictedChart, RestrictedChart, WitnessCell) {
        let fixture = plane_sphere_fixture();
        (
            RestrictedChart::from_plane(fixture.plane),
            RestrictedChart::from_sphere(fixture.sphere),
            fixture.cell,
        )
    }

    /// One whole restricted-pair solve on a battery fixture in one mode,
    /// reduced to the ordered certified box set and the witness presence.
    fn battery_run(
        a: RestrictedChart,
        b: RestrictedChart,
        cell: WitnessCell,
        on: bool,
    ) -> Option<(Vec<WitnessCell>, bool)> {
        let params = Ssi4Parameters {
            sample_constants: on,
            ..Ssi4Parameters::default()
        };
        let mut budget = Budget::new(0, 0, 0);
        match certify_restricted_pair(a, b, cell, &params, &mut budget) {
            Ok(EvCertified { value, .. }) => Some((
                value.samples.iter().map(|s| s.cell).collect(),
                value.witness.is_some(),
            )),
            Err(_) => None,
        }
    }

    #[test]
    fn verdicts_bit_identical_with_reuse_enabled() {
        // The landed ssi4 fixture battery, run whole with the CFP-007 machinery
        // on vs off: the full verdict + certificate box set is bit-identical
        // (reuse + inflation change cost, never results). Fixtures are
        // well-conditioned straight/circle branches, so no sample is marginal.
        let ps = fc5_branch_fixture();
        let sp = sweep_plane_fixture();
        let sp_a = RestrictedChart::circular_sweep(
            sp.sweep.spine_from,
            sp.sweep.spine_to,
            sp.sweep.radius_start,
            sp.sweep.radius_end,
            sp.sweep.s0,
            sp.sweep.s1,
            sp.sweep.v0,
            sp.sweep.v1,
        )
        .expect("the fixture sweep spine is non-degenerate");
        let battery: Vec<(&str, RestrictedChart, RestrictedChart, WitnessCell)> = vec![
            ("plane_x_sphere", ps.0, ps.1, ps.2),
            (
                "sweep_x_plane",
                sp_a,
                RestrictedChart::from_plane(sp.plane),
                sp.cell,
            ),
        ];
        for (name, a, b, cell) in battery {
            let on = battery_run(a.clone(), b.clone(), cell, true)
                .expect("battery certifies with constants on");
            let off = battery_run(a, b, cell, false).expect("battery certifies with constants off");
            assert_eq!(
                on.0, off.0,
                "the certified box set of {name} must be bit-identical with the cache on vs off"
            );
            assert_eq!(
                on.1, off.1,
                "the verdict (witness presence) of {name} must be unchanged"
            );
            assert!(!on.0.is_empty(), "{name} must certify branch samples");
        }
    }

    #[test]
    fn fc5_branch_fixture_prune_count_unchanged() {
        // F-C5 branch-fixture data: the branch sample count and ordered
        // certified box set on the fixture are unchanged by the optimization
        // (the reuse + inflation machinery never prunes a sample and never
        // re-boxes one).
        let (a, b, cell) = fc5_branch_fixture();
        fn curve(
            a: RestrictedChart,
            b: RestrictedChart,
            cell: WitnessCell,
            on: bool,
        ) -> CertifiedChartCurve {
            let params = Ssi4Parameters {
                sample_constants: on,
                ..Ssi4Parameters::default()
            };
            let mut budget = Budget::new(0, 0, 0);
            must_certified(certify_restricted_pair(a, b, cell, &params, &mut budget))
        }
        let with = curve(a.clone(), b.clone(), cell, true);
        let baseline = curve(a, b, cell, false);
        assert_eq!(with.samples.len(), baseline.samples.len());
        let boxes_with: Vec<WitnessCell> = with.samples.iter().map(|s| s.cell).collect();
        let boxes_baseline: Vec<WitnessCell> = baseline.samples.iter().map(|s| s.cell).collect();
        assert_eq!(boxes_with, boxes_baseline);
        assert_eq!(
            with.tangent_frames.len(),
            baseline.tangent_frames.len(),
            "one certified frame per sample, unchanged"
        );
    }

    #[test]
    fn eps_inflation_sequence_is_fixed_and_bounded() {
        // The inflation schedule is a named, bounded constant: radii `r·2^k`
        // for `k = 0..=3` (four attempts total, low-first), independent of any
        // geometry input — no adaptive, no geometry-dependent schedule.
        assert_eq!(
            EPS_INFLATION_RADII,
            [1.0, 2.0, 4.0, 8.0],
            "the fixed inflation multipliers are 2^k for k = 0..=3"
        );
        assert_eq!(
            EPS_INFLATION_ATTEMPTS,
            EPS_INFLATION_RADII.len(),
            "the attempt count is exactly the named constant"
        );
        assert_eq!(EPS_INFLATION_ATTEMPTS, 4);
        for (k, &scale) in EPS_INFLATION_RADII.iter().enumerate() {
            assert_eq!(
                scale,
                2.0_f64.powi(k as i32),
                "the k-th attempt doubles the base radius (radii r·2^k)"
            );
        }
    }

    /// One discovered marginal sample on the certified circle: the sample
    /// index, its chart centre, the base radius `r` (one-shot inclusion fails
    /// at `r`), and the box at `2r` that certifies.
    struct MarginalSample {
        /// The certified sample index (trace order, diagnostics).
        _idx: usize,
        /// The chart centre.
        center: [f64; 4],
        /// The base radius that fails one-shot.
        radius: f64,
        /// The doubled box that certifies.
        cell_2r: [Interval; 4],
        /// The augmented system at the sample.
        system: Ssi4System,
    }

    #[test]
    fn eps_inflation_recovers_marginal_sample() {
        // A marginal sample on the certified plane × sphere branch: one whose
        // box at the base radius `r` is NOT strictly certified one-shot by the
        // fresh preconditioner (the Krawczyk image touches the box boundary),
        // while the doubled box at `2r` certifies. The ε-inflation ladder must
        // recover exactly that sample at exactly `2r`.
        //
        // The marginal sample is constructed deterministically from the
        // fixture: the FIRST certified sample centre (in trace order) and
        // radius `r` from the fixed candidate list whose one-shot inclusion
        // fails at `r` and certifies at `2r`. Deterministic ordered search —
        // no randomness, no geometry-dependent schedule in the production code.
        let fixture = plane_sphere_fixture();
        let form = plane_sphere_form();
        let mut budget = Budget::new(0, 0, 0);
        let curve = must_certified(certify_restricted_pair(
            form.a.clone(),
            form.b.clone(),
            fixture.cell,
            &Ssi4Parameters::default(),
            &mut budget,
        ));
        assert!(
            curve.samples.len() > 64,
            "the certified circle provides sample centres"
        );

        // Candidate base radii, coarse to fine: the certified circle's sample
        // centres sit a few model units off the true root, so a base radius
        // just below the Newton offset is the marginal case; `2r` recovers.
        let candidate_radii = [5.0e-3, 2.5e-3, 1.25e-3, 1.0e-3]; // H-3: box half-widths, param units
        let mut marginal: Option<MarginalSample> = None;
        for (idx, sample) in curve.samples.iter().enumerate() {
            let center = sample.chart;
            let tangent = curve.tangent_frames[idx].tangent;
            let mut rhs = 0.0;
            for j in 0..4 {
                rhs += tangent[j] * center[j];
            }
            let system = Ssi4System::new(form.clone(), tangent, rhs);
            let Some(fresh) = system.preconditioner(&center) else {
                continue;
            };
            for &r in &candidate_radii {
                let Some(cell_r) = box_around(center, [r; 4]) else {
                    continue;
                };
                let Some(cell_2r) = box_around(center, [2.0 * r; 4]) else {
                    continue;
                };
                let at_r = one_shot_krawczyk(&system, &cell_r, Some(fresh));
                let at_2r = one_shot_krawczyk(&system, &cell_2r, Some(fresh));
                if matches!(at_r, OneShotOutcome::Inconclusive)
                    && matches!(at_2r, OneShotOutcome::Unique { .. })
                {
                    marginal = Some(MarginalSample {
                        _idx: idx,
                        center,
                        radius: r,
                        cell_2r,
                        system,
                    });
                    break;
                }
            }
            if marginal.is_some() {
                break;
            }
        }
        let MarginalSample {
            center,
            radius,
            cell_2r,
            system,
            ..
        } = marginal.expect("the certified circle provides a marginal sample");
        // The recovered certificate box is the inflated box (radius 2r), never
        // a re-boxed or bisected one: inflation widens the REQUESTED box.
        let mut cache = PreconditionerCache::<4>::default();
        let mut step_budget = Budget::new(128, 0, 0);
        let verdict = certified_theta_rho_step(
            &system,
            center,
            [radius; 4],
            &mut step_budget,
            &mut cache,
            1,
        );
        let StepVerdict::Certified { cell, .. } = &verdict else {
            unreachable!(
                "the ε-inflation ladder must certify the marginal sample (r {radius}), \
                 got {verdict:?}"
            );
        };
        assert_eq!(
            cell, &cell_2r,
            "the marginal sample is certified at exactly the doubled radius 2r"
        );
        let cached = cache.accepted.expect("the recovered sample is accepted");
        assert_eq!(cached.sample, 1);
        // The inflation sequence is bounded: the marginal sample certifies at
        // the SECOND attempt of the fixed ladder (k = 1), inside the named
        // attempt count.
        assert!(radius > 0.0 && cell_2r.iter().all(|c| c.inf().is_finite()));
    }
}
