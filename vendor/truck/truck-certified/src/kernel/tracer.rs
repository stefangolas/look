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

//! The D4 float predictor-corrector (BG-KV2-207-S4A): fast, UNCERTIFIED
//! proposals whose accept/reject path always goes through the certified seam
//! ([`crate::kernel::engine::build_frame4`] + [`crate::kernel::engine::c2_certify_tube4`]).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **D4 — float proposes, intervals dispose.** The march (the Gauss–Newton
//! predictor, the adaptive `dtau` growth/halving, the closed-loop float
//! recurrence test) runs in floating point. Interval arithmetic appears only
//! on the accept/reject path: every retained step carries a genuine
//! [`crate::kernel::certs::ArcCert`] emitted by the frozen tube seam, and the
//! §10.2 escalation routes are decided from certified enclosures alone (never
//! a float heuristic on the decision).
//!
//! **Doctrine (§10.1, normative).** Monotone in `tau` only — no 3D strong
//! monotonicity is ever imposed. Long arcs: the largest `I_tau` that passes
//! C2 is accepted and `dtau` grows aggressively on success. The predictor
//! reuses the last factorization (`Frame::a`) and re-factors only at a frame
//! rebuild.
//!
//! **The certified seam is frozen verbatim.** This module never constructs an
//! `ArcCert` itself: a [`FloatStep`] carries `certified = Some(cert)` only
//! where `cert` is the `Proven` arm of a `c2_certify_tube4` call. The
//! predictor's points are float data, never certificates.
//!
//! **Frame switches are not invented here.** A `FrameSwitch` segment break is
//! S9a's event; this packet records a rebuild only in the next step's `dtau`
//! reset and in the reported [`TraceStats`].
//!
//! **The §10.2 escalation ladder** classifies a C2 failure (once halving is
//! exhausted) in the normative order:
//!
//! 1. `sigma_min(DF) > 0` certified on the box (the F3 margin on the selected
//!    continuation block, evaluated as the four maximal minors of the stored
//!    Jacobian over a `tau` partition): rebuild the frame and retry; after
//!    `TracePolicy::max_frame_rebuilds` the trace refuses `Conditioning`.
//! 2. Parametric-regularity floor fails (no certified zero in the failed
//!    parameter box): refuse predicate `parametric_degeneracy_chart_or_carrier`.
//! 3. The R2 rank screen shows the contact zero set is 1-dimensional over the
//!    whole failed box: refuse [`RefusalKind::TangentialCurve`] (§10.4).
//! 4. The R2 zero set is isolated (rank collapse confined to at most two
//!    sub-boxes): refuse predicate `isolated_contact_is_s5a` (Wave-3 seam).
//! 5. Otherwise: refuse [`RefusalKind::HighOrderJet`].
//!
//! **H-3.** The predictor tolerances carry their `// H-3` markers on the
//! defining lines.

use crate::kernel::certs::{ArcCert, Frame};
use crate::kernel::engine::{build_frame4, c2_certify_tube4};
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::patch::{CertifiedPositive, IBox};
use crate::kernel::Interval;
use crate::SquareSystem3;

/// The perpendicular half-width of a proposed tube as a multiple of the arc's
/// `tau` width. A proposal tolerance (H-3): the C2 seam accepts or rejects the
/// resulting box; this constant only shapes the proposal.
const PERP_RATIO: f64 = 3.0; // H-3: predictor perpendicular half-width ratio

/// The float closed-loop recurrence tolerance (proposal tolerance, H-3): a
/// certified step whose chart point returns within this distance of the seed
/// is read as an identity recurrence (CERTIFIED closure is the promotion
/// path's job).
const CLOSE_TOL: f64 = 1e-3; // H-3: closed-loop recurrence detection tolerance

/// The number of `tau` sub-boxes of the escalation ladder's rank screen.
const RANK_SUBBOXES: usize = 8;

/// The run policy of the float tracer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TracePolicy {
    /// The initial (and post-rebuild) proposal `tau` width.
    pub arc_step0: f64,
    /// The multiplicative growth factor applied after a successful C2.
    pub grow: f64,
    /// The multiplicative shrink factor applied after a failed C2.
    pub shrink: f64,
    /// The number of consecutive halvings before the escalation ladder runs.
    pub max_halvings: u32,
    /// The number of frame rebuilds the ladder may order before refusing.
    pub max_frame_rebuilds: u32,
    /// The step cap (the tracer's depth cap is `policy.max_steps`).
    pub max_steps: usize,
}

impl TracePolicy {
    /// The §10.1 defaults: `arc_step0` 0.05, `grow` 2.0, `shrink` 0.5,
    /// `max_halvings` 3, `max_frame_rebuilds` 2, `max_steps` 4000.
    // Packet-spelled constructor name; not the std trait (BG-KV2-207-S4A).
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        TracePolicy {
            arc_step0: 0.05,
            grow: 2.0,
            shrink: 0.5,
            max_halvings: 3,
            max_frame_rebuilds: 2,
            max_steps: 4000,
        }
    }
}

/// One retained step of the float trace: the end `tau`, the float chart point
/// (predictor data, never a certificate), the certified width of the arc that
/// ended at this step, and the optional genuine [`ArcCert`] the frozen seam
/// emitted for it.
#[derive(Debug, Clone)]
pub struct FloatStep {
    /// The arc parameter at the end of this step (monotone along the trace).
    pub tau: f64,
    /// The chart point of this step in the four chart coordinates.
    pub point: [f64; 4],
    /// The certified width of the arc ending at this step.
    pub dtau: f64,
    /// `Some` exactly when the C2 attempt SUCCEEDED for the arc ending at this
    /// step. Never constructed outside the `c2_certify_tube4` call path.
    pub certified: Option<ArcCert<4>>,
}

/// The outcome of a [`float_trace`] run.
#[derive(Debug, Clone)]
pub enum FloatOutcome {
    /// The trace marched to the box boundary.
    Completed {
        /// The certified steps of the trace.
        steps: Vec<FloatStep>,
    },
    /// The trace returned to the seed (float tolerance detection; certified
    /// closure is the promotion path's job).
    ClosedLoop {
        /// The certified steps of the trace.
        steps: Vec<FloatStep>,
    },
    /// The trace refused with a named refusal.
    Refused(Refusal),
}

/// Diagnostic counters of one trace run (test support; [`float_trace`]
/// discards them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceStats {
    /// The number of certified steps retained.
    pub steps: usize,
    /// The number of frame rebuilds ordered by the escalation ladder.
    pub rebuilds: u32,
    /// The number of `dtau` halvings applied.
    pub halvings: u32,
}

/// Build the diagnostics of a run from its counters.
fn make_stats(steps: usize, rebuilds: u32, halvings: u32) -> TraceStats {
    TraceStats {
        steps,
        rebuilds,
        halvings,
    }
}

/// Trace the branch through `seed` with the float predictor-corrector.
///
/// Seed certification first: `build_frame4` + a C2 attempt on a small initial
/// `I_tau` (the certified seam). A seed that cannot frame refuses
/// `Conditioning` (Inconclusive).
pub fn float_trace(sys: &SquareSystem3, seed: [f64; 4], policy: &TracePolicy) -> FloatOutcome {
    float_trace_impl(sys, seed, policy).0
}

/// [`float_trace`] plus the diagnostic counters, for the fixture tests and the
/// packet's RESULT notes.
#[doc(hidden)]
pub fn float_trace_impl(
    sys: &SquareSystem3,
    seed: [f64; 4],
    policy: &TracePolicy,
) -> (FloatOutcome, TraceStats) {
    if !seed.iter().all(|c| c.is_finite()) {
        return (
            FloatOutcome::Refused(Refusal::new(
                RefusalKind::NonFinite,
                RefusalEvidence::Predicate {
                    name: "tracer_seed_not_finite",
                    detail: format!("the seed {seed:?} is not finite"),
                },
            )),
            make_stats(0, 0, 0),
        );
    }
    let weight = match CertifiedPositive::try_new(1.0) {
        Ok(w) => w,
        Err(refusal) => return (FloatOutcome::Refused(refusal), make_stats(0, 0, 0)),
    };
    let weights = [weight];

    let frame = match build_frame4(sys, seed) {
        Ok((frame, _m)) => frame,
        Err(refusal) => return (FloatOutcome::Refused(refusal), make_stats(0, 0, 0)),
    };
    trace_march(sys, seed, frame, &weights, policy)
}

/// One frame-reference point in chart coordinates from its `(tau, y)`
/// frame-coordinate pair. Float data (the predictor's proposal), never a
/// certificate.
fn chart_point(frame: &Frame<4>, tau: f64, y: &[f64; 3]) -> [f64; 4] {
    let mut out = [0.0f64; 4];
    for (j, out_j) in out.iter_mut().enumerate() {
        let mut v = frame.z_hat[j] + frame.q_tau[j] * tau;
        for (c, y_c) in y.iter().enumerate() {
            v += frame.q_perp[c][j] * y_c;
        }
        *out_j = v;
    }
    out
}

/// The four chart rectangles of the stored system.
fn chart_rect(sys: &SquareSystem3) -> [(f64, f64); 4] {
    let maps = sys.domain_maps();
    [
        (maps.0, maps.1),
        (maps.2, maps.3),
        (maps.4, maps.5),
        (maps.6, maps.7),
    ]
}

/// Whether a chart coordinate lies strictly inside the chart rectangle.
fn inside_rect(rect: &[(f64, f64); 4], p: &[f64; 4]) -> bool {
    rect.iter()
        .zip(p.iter())
        .all(|((lo, hi), c)| *lo < *c && *c < *hi)
}

/// Map a chart coordinate onto the axis's unit parameter.
fn to_unit_coord(p: f64, d0: f64, d1: f64) -> Option<f64> {
    let width = d1 - d0;
    if !p.is_finite() || !width.is_finite() || width <= 0.0 {
        return None;
    }
    let u = (p - d0) / width;
    if u.is_finite() {
        Some(u)
    } else {
        None
    }
}

/// Map a chart point onto the unit chart `[0, 1]^4`.
fn to_unit_point(sys: &SquareSystem3, p: &[f64; 4]) -> Option<[f64; 4]> {
    let rect = chart_rect(sys);
    let mut out = [0.0f64; 4];
    for a in 0..4 {
        out[a] = to_unit_coord(p[a], rect[a].0, rect[a].1)?;
    }
    Some(out)
}

/// Float evaluation of one stored component at a unit-chart point, in the
/// deterministic tensor summation order.
fn eval_component_unit(sys: &SquareSystem3, component: usize, u: &[f64; 4]) -> Option<f64> {
    let (m1, n1, m2, n2) = sys.degrees();
    let rows = sys.grids().get(component)?;
    let ba = bern_basis(m1, u[0]);
    let bb = bern_basis(n1, u[1]);
    let bi = bern_basis(m2, u[2]);
    let bj = bern_basis(n2, u[3]);
    let mut acc = 0.0f64;
    for a in 0..=m1 {
        for b in 0..=n1 {
            let row = &rows[a * (n1 + 1) + b];
            for i in 0..=m2 {
                for j in 0..=n2 {
                    acc += row[i * (n2 + 1) + j] * ba[a] * bb[b] * bi[i] * bj[j];
                }
            }
        }
    }
    if acc.is_finite() {
        Some(acc)
    } else {
        None
    }
}

/// Float evaluation of the stored system at a chart point.
fn eval_chart(sys: &SquareSystem3, p: &[f64; 4]) -> Option<[f64; 3]> {
    let u = to_unit_point(sys, p)?;
    let mut out = [0.0f64; 3];
    for (k, out_k) in out.iter_mut().enumerate() {
        *out_k = eval_component_unit(sys, k, &u)?;
    }
    Some(out)
}

/// The 1-D Bernstein basis values at `t`, computed by de Casteljau on the
/// standard basis.
fn bern_basis(deg: usize, t: f64) -> Vec<f64> {
    (0..=deg)
        .map(|i| {
            let mut c = vec![0.0f64; deg + 1];
            c[i] = 1.0;
            let mut level = c;
            while level.len() > 1 {
                let mut next = Vec::with_capacity(level.len() - 1);
                for w in level.windows(2) {
                    next.push(w[0] + t * (w[1] - w[0]));
                }
                level = next;
            }
            level[0]
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Certified interval machinery over the stored tensor grids (the tracer's own
// enclosure kernels: the same de-Casteljau-over-CertifiedInterval discipline
// as the engine, kept local because the engine's kernels are module-private).
// ---------------------------------------------------------------------------

/// The interval de Casteljau step over a list of coefficients. `None` when the
/// result is not finite.
fn dcast1_iv(pts: &[Interval], u: &Interval) -> Option<Interval> {
    if pts.is_empty() {
        return None;
    }
    let mut level = pts.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(w[0].add(&u.mul(&w[1].sub(&w[0]))));
        }
        level = next;
    }
    if level[0].is_finite() {
        Some(level[0])
    } else {
        None
    }
}

/// Certified range enclosure of an interval-valued bivariate tensor grid over
/// the parameter rectangle `(s, t)`. `grid[i][j]` is the coefficient of
/// `B_i^m(s) B_j^n(t)`; `s` runs along rows and `t` along columns.
fn hull2_iv(grid: &[Vec<Interval>], s: &Interval, t: &Interval) -> Option<Interval> {
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }
    let width = grid[0].len();
    if grid.iter().any(|row| row.len() != width) {
        return None;
    }
    let mut col_evals = Vec::with_capacity(width);
    for j in 0..width {
        let col: Vec<Interval> = grid.iter().map(|row| row[j]).collect();
        col_evals.push(dcast1_iv(&col, s)?);
    }
    dcast1_iv(&col_evals, t)
}

/// Certified range enclosure of a stored component over a unit-chart sub-box.
/// Axes are collapsed in the fixed order 0,1,2,3.
fn hull_component_unit(
    rows: &[Vec<f64>],
    deg: (usize, usize, usize, usize),
    box_: &[(f64, f64); 4],
) -> Option<Interval> {
    if rows.is_empty() || rows[0].is_empty() {
        return None;
    }
    for &(lo, hi) in box_ {
        if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
            return None;
        }
    }
    let (d1, d2, _d3, d4) = deg;
    let cols = rows[0].len();
    let u_iv = Interval {
        lo: box_[0].0,
        hi: box_[0].1,
    };
    let v_iv = Interval {
        lo: box_[1].0,
        hi: box_[1].1,
    };
    // Collapse axis 0 (u, index a) for each (b, column) pair.
    let mut u_cols = vec![Vec::<Interval>::with_capacity(d2 + 1); cols];
    for b in 0..=d2 {
        for (c, slot) in u_cols.iter_mut().enumerate() {
            let mut pts = Vec::with_capacity(d1 + 1);
            for a in 0..=d1 {
                pts.push(Interval::point(rows[a * (d2 + 1) + b][c]));
            }
            slot.push(dcast1_iv(&pts, &u_iv)?);
        }
    }
    // Collapse axis 1 (v, index b).
    let mut v_collapsed = Vec::with_capacity(cols);
    for col in u_cols {
        v_collapsed.push(dcast1_iv(&col, &v_iv)?);
    }
    // Reshape into (i, j) over axes 2, 3.
    let mut grid2: Vec<Vec<Interval>> = Vec::with_capacity(v_collapsed.len() / (d4 + 1));
    for row_slice in v_collapsed.chunks(d4 + 1) {
        grid2.push(row_slice.to_vec());
    }
    let s_iv = Interval {
        lo: box_[2].0,
        hi: box_[2].1,
    };
    let t_iv = Interval {
        lo: box_[3].0,
        hi: box_[3].1,
    };
    hull2_iv(&grid2, &s_iv, &t_iv)
}

/// The first-partial coefficient grid of a stored component along one chart
/// axis, in the flat layout. `None` when the axis degree is zero.
#[allow(clippy::type_complexity)]
fn partial_grid(
    rows: &[Vec<f64>],
    deg: (usize, usize, usize, usize),
    axis: usize,
) -> Option<(Vec<Vec<f64>>, (usize, usize, usize, usize))> {
    let (m1, n1, m2, n2) = deg;
    let base = [m1, n1, m2, n2][axis];
    if base == 0 {
        return None;
    }
    let scale = base as f64;
    let new_deg = match axis {
        0 => (m1 - 1, n1, m2, n2),
        1 => (m1, n1 - 1, m2, n2),
        2 => (m1, n1, m2 - 1, n2),
        _ => (m1, n1, m2, n2 - 1),
    };
    let (nm1, nn1, nm2, nn2) = new_deg;
    let mut out = vec![vec![0.0f64; (nm2 + 1) * (nn2 + 1)]; (nm1 + 1) * (nn1 + 1)];
    for a in 0..=nm1 {
        for b in 0..=nn1 {
            for i in 0..=nm2 {
                for j in 0..=nn2 {
                    let (a0, b0, i0, j0, a1, b1, i1, j1) = match axis {
                        0 => (a, b, i, j, a + 1, b, i, j),
                        1 => (a, b, i, j, a, b + 1, i, j),
                        2 => (a, b, i, j, a, b, i + 1, j),
                        _ => (a, b, i, j, a, b, i, j + 1),
                    };
                    let lo = rows[a0 * (n1 + 1) + b0][i0 * (n2 + 1) + j0];
                    let hi = rows[a1 * (n1 + 1) + b1][i1 * (n2 + 1) + j1];
                    out[a * (nn1 + 1) + b][i * (nn2 + 1) + j] = scale * (hi - lo);
                }
            }
        }
    }
    Some((out, new_deg))
}

/// Map a chart sub-box onto the unit chart `[0, 1]^4`, outward-rounded and
/// clamped. `None` when the sub-box is not a compact subset of the chart
/// rectangle.
fn to_unit_box(sys: &SquareSystem3, box_: &[(f64, f64); 4]) -> Option<[(f64, f64); 4]> {
    let rect = chart_rect(sys);
    let mut out = [(0.0f64, 0.0f64); 4];
    for a in 0..4 {
        let (d0, d1) = rect[a];
        let (lo, hi) = box_[a];
        if !lo.is_finite() || !hi.is_finite() || !(d0 <= lo && lo <= hi && hi <= d1) {
            return None;
        }
        let span = Interval::point(d1).sub(&Interval::point(d0));
        if span.lo <= 0.0 {
            return None;
        }
        let lo_u = Interval::point(lo).sub(&Interval::point(d0)).div(&span)?;
        let hi_u = Interval::point(hi).sub(&Interval::point(d0)).div(&span)?;
        let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
        let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
        out[a] = (u_lo, u_hi);
    }
    Some(out)
}

/// Certified value enclosure of one component over a chart sub-box.
fn value_enclosure(
    sys: &SquareSystem3,
    component: usize,
    box_: &[(f64, f64); 4],
) -> Option<Interval> {
    let unit = to_unit_box(sys, box_)?;
    let grid = sys.grids().get(component)?;
    hull_component_unit(grid, sys.degrees(), &unit)
}

/// Certified chart-coordinate partial enclosure of one component along one
/// chart axis over a chart sub-box (unit-axis derivative scaled by the inverse
/// chart width).
fn partial_enclosure(
    sys: &SquareSystem3,
    component: usize,
    axis: usize,
    box_: &[(f64, f64); 4],
) -> Option<Interval> {
    let unit = to_unit_box(sys, box_)?;
    let grid = sys.grids().get(component)?;
    let (deriv, deg) = partial_grid(grid, sys.degrees(), axis)?;
    let hull = hull_component_unit(&deriv, deg, &unit)?;
    let rect = chart_rect(sys);
    let width = Interval::point(rect[axis].1).sub(&Interval::point(rect[axis].0));
    hull.div(&width)
}

/// The interval determinant of a 3x3 interval matrix (the same cofactor
/// expansion as the engine's `det3_f64`, over interval arithmetic).
fn det3_iv(m: &[[Interval; 3]; 3]) -> Option<Interval> {
    let a = m[0][0].mul(&m[1][1].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][1])));
    let b = m[0][1].mul(&m[1][0].mul(&m[2][2]).sub(&m[1][2].mul(&m[2][0])));
    let c = m[0][2].mul(&m[1][0].mul(&m[2][1]).sub(&m[1][1].mul(&m[2][0])));
    let det = a.sub(&b).add(&c);
    if det.is_finite() {
        Some(det)
    } else {
        None
    }
}

/// The four Theorem 6.4 maximal-minor intervals of the stored system's
/// Jacobian over a chart sub-box, `m_j = (-1)^j det(DF with column j
/// deleted)`.
fn minor_enclosures(sys: &SquareSystem3, box_: &[(f64, f64); 4]) -> Option<[Interval; 4]> {
    let mut partials = [[Interval::point(0.0); 4]; 3];
    for (r, row) in partials.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            *cell = partial_enclosure(sys, r, c, box_)?;
        }
    }
    let minor = |cols: [usize; 3]| -> Option<Interval> {
        let mut m = [[Interval::point(0.0); 3]; 3];
        for (r, row) in m.iter_mut().enumerate() {
            for (k, &c) in cols.iter().enumerate() {
                row[k] = partials[r][c];
            }
        }
        det3_iv(&m)
    };
    let d0 = minor([1, 2, 3])?;
    let d1 = minor([0, 2, 3])?.neg();
    let d2 = minor([0, 1, 3])?;
    let d3 = minor([0, 1, 2])?.neg();
    Some([d0, d1, d2, d3])
}

/// Whether an interval contains zero.
fn contains_zero(iv: &Interval) -> bool {
    iv.lo <= 0.0 && 0.0 <= iv.hi
}

/// The chart-space box of a proposed tube: the hull over the frame origin
/// `z_hat`, the tangent interval `i_tau` and the perpendicular box `y_iv`.
/// `None` when the box is not a compact subset of the chart rectangle.
fn tube_chart_box(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    i_tau: Interval,
    y_iv: &[Interval; 3],
) -> Option<[(f64, f64); 4]> {
    let mut acc: [Interval; 4] = [
        Interval::point(frame.z_hat[0]),
        Interval::point(frame.z_hat[1]),
        Interval::point(frame.z_hat[2]),
        Interval::point(frame.z_hat[3]),
    ];
    for (j, acc_j) in acc.iter_mut().enumerate() {
        *acc_j = acc_j.add(&Interval::point(frame.q_tau[j]).mul(&i_tau));
        for (c, axis_c) in y_iv.iter().enumerate() {
            *acc_j = acc_j.add(&Interval::point(frame.q_perp[c][j]).mul(axis_c));
        }
    }
    let rect = chart_rect(sys);
    let mut out = [(0.0f64, 0.0f64); 4];
    for (j, out_j) in out.iter_mut().enumerate() {
        if !acc[j].is_finite() {
            return None;
        }
        if acc[j].lo < rect[j].0 || acc[j].hi > rect[j].1 {
            return None;
        }
        *out_j = (acc[j].lo, acc[j].hi);
    }
    Some(out)
}

fn sub_interval(k: usize, count: usize) -> (f64, f64) {
    (k as f64 / count as f64, (k + 1) as f64 / count as f64)
}

// ---------------------------------------------------------------------------
// The marching loop
// ---------------------------------------------------------------------------

/// The outcome of one tube attempt over `[tau_lo, tau_lo + dtau]`.
// ArcAttempt::Proven carries the ArcCert; the size spread is allowed.
#[allow(clippy::large_enum_variant)]
enum ArcAttempt {
    /// The C2 seam certified the arc; `y_pred` is the predictor's box centre.
    Proven { cert: ArcCert<4>, y_pred: [f64; 3] },
    /// The tube box left the chart rectangle.
    OutOfChart,
    /// Any other C2 failure.
    Other,
}

fn is_out_of_chart(reason: &str) -> bool {
    reason == "tube_joint_box_outside_chart_domain"
        || reason == "tube_slice_box_outside_chart_domain"
}

/// One Gauss–Newton predictor step reusing the last factorization (`Frame::a`)
/// — the cheap-predictor rule of §10.1. Float proposal data only.
fn predict_y(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    tau_hi: f64,
    y0: &[f64; 3],
) -> Option<[f64; 3]> {
    let p = chart_point(frame, tau_hi, y0);
    let f = eval_chart(sys, &p)?;
    let mut y1 = [0.0f64; 3];
    for r in 0..3 {
        let mut acc = y0[r];
        for (aa, ff) in frame.a[r][..3].iter().zip(f.iter()) {
            acc -= aa * ff;
        }
        y1[r] = acc;
    }
    if y1.iter().all(|v| v.is_finite()) {
        Some(y1)
    } else {
        None
    }
}

/// Attempt a tube over `[tau_lo, tau_lo + dtau]` from the current frame and
/// perpendicular centre `y`.
fn attempt_arc(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    weights: &[CertifiedPositive],
    tau_lo: f64,
    y: &[f64; 3],
    dtau: f64,
) -> ArcAttempt {
    let tau_hi = tau_lo + dtau;
    let y_pred = match predict_y(sys, frame, tau_hi, y) {
        Some(y) => y,
        None => return ArcAttempt::Other,
    };
    let h = PERP_RATIO * dtau;
    let lo = [y_pred[0] - h, y_pred[1] - h, y_pred[2] - h];
    let hi = [y_pred[0] + h, y_pred[1] + h, y_pred[2] + h];
    let b_perp = match IBox::<3>::try_new(lo, hi) {
        Ok(b) => b,
        Err(_) => return ArcAttempt::Other,
    };
    let i_tau = Interval {
        lo: tau_lo,
        hi: tau_hi,
    };
    match c2_certify_tube4(sys, frame, i_tau, b_perp, weights) {
        ClaimVerdict::Proven(cert) => ArcAttempt::Proven { cert, y_pred },
        ClaimVerdict::Inconclusive(reason) if is_out_of_chart(reason) => ArcAttempt::OutOfChart,
        ClaimVerdict::Inconclusive(_) | ClaimVerdict::Disproven(_) => ArcAttempt::Other,
    }
}

/// The decision of the escalation ladder.
enum Escalation {
    /// Rung 1 fired: rebuild the frame and retry C2 (conditioning, not
    /// geometry).
    Rebuild,
    /// A rung 2-5 refusal.
    Refuse(Refusal),
}

/// The tau distance from `point` along `frame.q_tau` before the ray leaves the
/// chart rectangle.
fn tau_room(frame: &Frame<4>, point: &[f64; 4], rect: &[(f64, f64); 4]) -> f64 {
    let mut room = f64::INFINITY;
    for j in 0..4 {
        let d = frame.q_tau[j];
        if d == 0.0 {
            continue;
        }
        let rem = if d > 0.0 {
            (rect[j].1 - point[j]) / d
        } else {
            (rect[j].0 - point[j]) / d
        };
        if rem.is_finite() {
            room = room.min(rem);
        }
    }
    room
}

/// The §10.2 escalation ladder over a failed arc `[tau_lo, tau_lo + dtau]`
/// whose tube box (in the current frame) is `[y_lo, y_hi]`. Decided from
/// certified enclosures alone.
fn escalate(
    sys: &SquareSystem3,
    frame: &Frame<4>,
    tau_lo: f64,
    dtau: f64,
    y_lo: &[f64; 3],
    y_hi: &[f64; 3],
) -> Escalation {
    let y_iv: [Interval; 3] = [
        Interval {
            lo: y_lo[0],
            hi: y_hi[0],
        },
        Interval {
            lo: y_lo[1],
            hi: y_hi[1],
        },
        Interval {
            lo: y_lo[2],
            hi: y_hi[2],
        },
    ];
    let i_tau = Interval {
        lo: tau_lo,
        hi: tau_lo + dtau,
    };
    let chart_box = match tube_chart_box(sys, frame, i_tau, &y_iv) {
        Some(box_) => box_,
        None => return Escalation::Rebuild,
    };

    // Rung-2 probe: certified branch absence over the whole failed box.
    for component in 0..3 {
        match value_enclosure(sys, component, &chart_box) {
            Some(iv) if !contains_zero(&iv) => {
                return Escalation::Refuse(Refusal::new(
                    RefusalKind::Conditioning,
                    RefusalEvidence::Predicate {
                        name: "parametric_degeneracy_chart_or_carrier",
                        detail: "no certified zero of the system in the failed parameter box: the \
                                 parametric-regularity floor fails (the chart-switch route is §3.4's)"
                            .to_string(),
                    },
                ));
            }
            _ => {}
        }
    }

    // Partition the failed arc into tau sub-boxes and run the enclosure rank
    // screen.
    let mut collapse = 0usize;
    for k in 0..RANK_SUBBOXES {
        let (f0, f1) = sub_interval(k, RANK_SUBBOXES);
        let sub_tau = Interval {
            lo: tau_lo + f0 * dtau,
            hi: tau_lo + f1 * dtau,
        };
        let sub_box = match tube_chart_box(sys, frame, sub_tau, &y_iv) {
            Some(b) => b,
            None => {
                collapse += 1;
                continue;
            }
        };
        let minors = match minor_enclosures(sys, &sub_box) {
            Some(m) => m,
            None => {
                collapse += 1;
                continue;
            }
        };
        if minors.iter().all(contains_zero) {
            collapse += 1;
        }
    }

    match collapse {
        0 => Escalation::Rebuild,
        c if c == RANK_SUBBOXES => Escalation::Refuse(Refusal::new(
            RefusalKind::TangentialCurve,
            RefusalEvidence::Predicate {
                name: "r2_contact_zero_set_one_dimensional",
                detail: format!(
                    "the four minor enclosures jointly contain zero on all {c} sub-boxes of the \
                     failed arc: the R2 contact zero set is 1-dimensional (§10.4, never trace)"
                ),
            },
        )),
        c if c <= 2 => Escalation::Refuse(Refusal::new(
            RefusalKind::Conditioning,
            RefusalEvidence::Predicate {
                name: "isolated_contact_is_s5a",
                detail: format!(
                    "the rank collapse is confined to {c} sub-box(es): the R2 zero set is \
                     isolated — the contact-certificate path is S5a's (Wave 3), the refusal is \
                     the seam"
                ),
            },
        )),
        c => Escalation::Refuse(Refusal::new(
            RefusalKind::HighOrderJet,
            RefusalEvidence::Predicate {
                name: "rank_screen_not_isolated_not_curve",
                detail: format!(
                    "the rank screen shows rank collapse on {c} of {RANK_SUBBOXES} sub-boxes: \
                     neither a clean isolated contact nor a full-arc tangency — a higher-order \
                     jet is required"
                ),
            },
        )),
    }
}

/// The core marching loop shared by [`float_trace`].
fn trace_march(
    sys: &SquareSystem3,
    seed: [f64; 4],
    frame0: Frame<4>,
    weights: &[CertifiedPositive],
    policy: &TracePolicy,
) -> (FloatOutcome, TraceStats) {
    let rect = chart_rect(sys);
    let floor_dtau = policy.arc_step0 * policy.shrink.powi(policy.max_halvings as i32);

    let mut frame = frame0;
    let mut steps: Vec<FloatStep> = Vec::new();
    let mut halvings = 0u32;
    let mut rebuilds = 0u32;
    let mut dtau = policy.arc_step0;
    let mut tau_local = 0.0f64;
    let mut tau_global = 0.0f64;
    let mut y = [0.0f64; 3];
    let mut point = seed;

    loop {
        if steps.len() >= policy.max_steps {
            let st = make_stats(steps.len(), rebuilds, halvings);
            return (
                FloatOutcome::Refused(Refusal::new(
                    RefusalKind::Budget,
                    RefusalEvidence::Predicate {
                        name: "tracer_max_steps",
                        detail: format!("the trace exceeded the step cap {}", policy.max_steps),
                    },
                )),
                st,
            );
        }

        // The branch has reached the box boundary when even the smallest arc
        // cannot fit ahead along the current tangent.
        if !inside_rect(&rect, &point) || tau_room(&frame, &point, &rect) <= floor_dtau {
            let st = make_stats(steps.len(), rebuilds, halvings);
            return (FloatOutcome::Completed { steps }, st);
        }

        match attempt_arc(sys, &frame, weights, tau_local, &y, dtau) {
            ArcAttempt::Proven { cert, y_pred } => {
                let mut dt = dtau;
                let mut cert = cert;
                let mut yp = y_pred;
                loop {
                    let gdt = dt * policy.grow;
                    match attempt_arc(sys, &frame, weights, tau_local, &y, gdt) {
                        ArcAttempt::Proven {
                            cert: gcert,
                            y_pred: gyp,
                        } => {
                            dt = gdt;
                            cert = gcert;
                            yp = gyp;
                        }
                        _ => break,
                    }
                }
                let p_new = chart_point(&frame, tau_local + dt, &yp);
                steps.push(FloatStep {
                    tau: tau_global + dt,
                    point: p_new,
                    dtau: dt,
                    certified: Some(cert),
                });
                tau_local += dt;
                tau_global += dt;
                point = p_new;
                y = yp;
                dtau = dt;
                halvings = 0;
                rebuilds = 0;
                if tau_global >= 2.0 * policy.arc_step0
                    && (0..4).all(|j| (point[j] - seed[j]).abs() <= CLOSE_TOL)
                {
                    let st = make_stats(steps.len(), rebuilds, halvings);
                    return (FloatOutcome::ClosedLoop { steps }, st);
                }
            }
            failed => {
                if halvings < policy.max_halvings {
                    dtau *= policy.shrink;
                    halvings += 1;
                    continue;
                }
                if matches!(failed, ArcAttempt::OutOfChart) {
                    let st = make_stats(steps.len(), rebuilds, halvings);
                    return (FloatOutcome::Completed { steps }, st);
                }
                let y_pred = match predict_y(sys, &frame, tau_local + dtau, &y) {
                    Some(y) => y,
                    None => {
                        let st = make_stats(steps.len(), rebuilds, halvings);
                        return (
                            FloatOutcome::Refused(Refusal::new(
                                RefusalKind::Conditioning,
                                RefusalEvidence::Predicate {
                                    name: "tracer_predictor_not_finite",
                                    detail: "the float predictor produced non-finite data"
                                        .to_string(),
                                },
                            )),
                            st,
                        );
                    }
                };
                let h = PERP_RATIO * dtau;
                let y_lo = [y_pred[0] - h, y_pred[1] - h, y_pred[2] - h];
                let y_hi = [y_pred[0] + h, y_pred[1] + h, y_pred[2] + h];
                match escalate(sys, &frame, tau_local, dtau, &y_lo, &y_hi) {
                    Escalation::Rebuild => {
                        if rebuilds >= policy.max_frame_rebuilds {
                            let st = make_stats(steps.len(), rebuilds, halvings);
                            return (
                                FloatOutcome::Refused(Refusal::new(
                                    RefusalKind::Conditioning,
                                    RefusalEvidence::Predicate {
                                        name: "tracer_frame_rebuilds_exhausted",
                                        detail: format!(
                                            "the escalation ladder rebuilt the frame {} times \
                                             without progress",
                                            policy.max_frame_rebuilds
                                        ),
                                    },
                                )),
                                st,
                            );
                        }
                        match build_frame4(sys, point) {
                            Ok((new_frame, _m)) => {
                                frame = new_frame;
                                rebuilds += 1;
                                dtau = policy.arc_step0;
                                halvings = 0;
                                tau_local = 0.0;
                                y = [0.0f64; 3];
                            }
                            Err(refusal) => {
                                let st = make_stats(steps.len(), rebuilds, halvings);
                                return (FloatOutcome::Refused(refusal), st);
                            }
                        }
                    }
                    Escalation::Refuse(refusal) => {
                        let st = make_stats(steps.len(), rebuilds, halvings);
                        return (FloatOutcome::Refused(refusal), st);
                    }
                }
            }
        }
    }
}
