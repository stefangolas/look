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

//! The §7 R8/R9 square residuals (BG-KV2-202-S1A): the curve–surface
//! difference `H(t,u,v) = C(t) − S(u,v) : R³ → R³` (arity 3, square C1) and
//! the one-chart curve–curve difference `J(t,r) = C₁(t) − C₂(r) : R² → R²`
//! (arity 2, square C1), both in D-homogeneous cross-multiplied form over the
//! S2A frozen seam ([`SquareResidualEval`] + [`krawczyk_c1`] /
//! [`krawczyk_c1_n3`]).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`. Where a `Result` must carry the
//! frozen `Refusal` (which holds `Option<PartialGraph>`), the large-`Err` lint
//! is allowed item-level only, exactly as the shim files do.
//!
//! **N5 / no premature division.** Every residual is evaluated on its
//! HOMOGENEOUS polynomial grids: `H_k = Cw(t)·S_k(u,v) − C_k(t)·Sw(u,v)`
//! (R8, `k in {x,y,z}`) and `H_k = C1w(t)·C2_k(r) − C1_k(t)·C2w(r)` (R9,
//! `k in {x,y}`). No weight-bearing interval expression is divided anywhere in
//! this module; the §7.1 positive weight bounds arrive as the VALUE argument
//! to the C1 entries and are only checked for non-emptiness by the engine
//! (never re-derived, never divided by). Where the true range is
//! dehomogenized (the residual's geometric meaning is the divided
//! difference), the cross-multiplied zero set equals it exactly while the
//! weights stay positive — the caller certifies that positivity per box.
//!
//! **N4 / bit-reproducibility.** This module performs no transcendental call:
//! no `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, `powf`, and no `sqrt`
//! anywhere. The evaluations and Jacobian enclosures are deterministic
//! `CertifiedInterval` sequences over the landed hull kernels
//! ([`hull_bernstein_1d`] / [`hull_bernstein_2d`]), outward-rounded only.
//!
//! **Eval discipline (leaf.rs / hull.rs).** The box `b` received by `eval` /
//! `jac_encl` is the joint interval box over ALL variables. Because the stored
//! leaf data is a 1-var (curve) or 2-var (surface) Bernstein net, the joint
//! enclosure is composed as products of per-factor hulls: the certified range
//! of `Cw` over the `t`-axis times the certified range of `S_k` over the
//! `(u,v)`-rectangle, and so on. The product of supersets of the factor ranges
//! is a superset of the product range, so the composed interval provably
//! encloses the residual (and its partials) over the whole box. A box axis
//! that leaves the leaf's `[0,1]` unit domain — or a hull refusal — yields the
//! vacuous unbounded enclosure, exactly the leaf.rs convention, so the C1
//! entry sees a non-finite image and answers Inconclusive rather than an
//! unsound bound.
//!
//! **Regularity by construction.** The C1 certification itself is the
//! transversality certificate: a `Proven` arm requires the midpoint Jacobian
//! of the cross-multiplied residual to be invertible over the box, which (with
//! certified-positive weights) is equivalent to `C'(t) ∉ T(u,v)` for R8 (det
//! `DH ≠ 0`) and `det[C₁′, −C₂′] ≠ 0` for R9. A tangential input is
//! therefore never `Proven` (see the S1A backing table).

use crate::hull::{
    bernstein_derivative_1d, bernstein_derivative_2d, hull_bernstein_1d, hull_bernstein_2d,
};
use crate::kernel::engine::SquareResidualEval;
use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::graph::ChartId;
use crate::kernel::leaf::BezierLeaf;
use crate::kernel::Interval;

/// The certified-interval enclosure of the full real line: the vacuous bound
/// used when a box axis leaves a leaf's `[0,1]` unit domain or a hull kernel
/// refuses (the leaf.rs "vacuously true enclosure" convention).
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// A named predicate refusal: the kind names the violated class, the predicate
/// name the precise invariant.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}

/// The certified range of the Bernstein polynomial `coeffs` over the compact
/// `[0,1]` sub-interval `i`, or the vacuous unbounded enclosure when the hull
/// kernel refuses (a box axis outside the leaf's unit domain, or a non-finite
/// hull).
fn hull1(coeffs: &[f64], i: &Interval) -> Interval {
    match hull_bernstein_1d(coeffs, (i.lo, i.hi)) {
        Ok(hull) => hull,
        Err(_) => unbounded(),
    }
}

/// The certified range of the tensor-Bernstein polynomial `grid` over the
/// `(u,v)` sub-rectangle, or the vacuous unbounded enclosure on a hull
/// refusal.
fn hull2(grid: &[Vec<f64>], u: &Interval, v: &Interval) -> Interval {
    match hull_bernstein_2d(grid, (u.lo, u.hi), (v.lo, v.hi)) {
        Ok(hull) => hull,
        Err(_) => unbounded(),
    }
}

/// The provenance of a 1-var curve leaf's source carrier (§3.2/N4). The R8/R9
/// residuals certify RATIONAL homogeneous leaves only: a leaf whose source is
/// a transcendental-only carrier is refused at the residual constructors with
/// [`RefusalKind::TranscendentalCarrier`] (Disproven). Rational carriers are
/// the closed §3.2 family, so no rational leaf data can ever carry the
/// transcendental marker by construction; the marker exists so a
/// transcendental-only source is refused where a later wave translates one
/// (the refusal kind is constructible by callers per the build-spec §3.2
/// note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveCarrierKind {
    /// The leaf is a rational homogeneous Bézier net (admitted).
    Rational,
    /// The leaf lifts a transcendental-only carrier; no rational
    /// certification is possible.
    Transcendental,
}

/// A 1-var rational Bézier leaf (the §7 R8 curve leaf and the §7 R9 chart
/// curve leaf): the homogeneous `xyzw` control polygon over the integer grid
/// `(degree + 1)`, plus the lifted chart its parameters live in and the
/// source-carrier provenance.
///
/// Construct only through [`BezierLeaf1::try_new`], which refuses a zero
/// degree, a mismatched control count, non-finite coordinates, and a
/// non-positive control weight — the same discipline as the landed
/// [`BezierLeaf`]. The `carrier` marker is set to
/// [`CurveCarrierKind::Rational`] by construction; the fields are public so a
/// leaf-extraction wave can carry the marker, exactly as `BezierLeaf`'s fields
/// are public.
#[derive(Debug, Clone, PartialEq)]
pub struct BezierLeaf1 {
    /// The polynomial degree in the curve parameter.
    pub degree: usize,
    /// The homogeneous `xyzw` control points, in increasing parameter order.
    pub control: Vec<[f64; 4]>,
    /// The lifted chart the curve's parameters belong to (§3.3).
    pub chart: ChartId,
    /// The source-carrier provenance (rational only is certified).
    pub carrier: CurveCarrierKind,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl BezierLeaf1 {
    /// Build a 1-var leaf, refusing a zero degree, a mismatched control count,
    /// non-finite coordinates, or a non-positive control weight.
    pub fn try_new(degree: usize, control: Vec<[f64; 4]>, chart: ChartId) -> Construction<Self> {
        if degree == 0 {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "bezier1_zero_degree",
                format!("curve leaf degree {degree} must be positive"),
            ));
        }
        let expected = degree + 1;
        if control.len() != expected {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "bezier1_control_count_mismatch",
                format!(
                    "control polygon has {} points, degree {degree} requires {expected}",
                    control.len()
                ),
            ));
        }
        validate_curve_net(degree, &control)?;
        Ok(Self {
            degree,
            control,
            chart,
            carrier: CurveCarrierKind::Rational,
        })
    }

    /// The `comp`-coordinate coefficient list (`0..=3`, `3 == w`).
    fn coeff(&self, comp: usize) -> Vec<f64> {
        self.control.iter().map(|p| p[comp]).collect()
    }

    /// The Bernstein coefficients of the `comp`-coordinate first derivative.
    fn coeff_deriv(&self, comp: usize) -> Vec<f64> {
        bernstein_derivative_1d(&self.coeff(comp))
    }
}

/// Validate a curve homogeneous net: the control count, non-finite
/// coordinates, and strictly-positive control weights.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn validate_curve_net(degree: usize, control: &[[f64; 4]]) -> Result<(), Refusal> {
    let expected = degree + 1;
    if control.len() != expected {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier1_control_count_mismatch",
            format!(
                "control polygon has {} points, degree {degree} requires {expected}",
                control.len()
            ),
        ));
    }
    for (i, p) in control.iter().enumerate() {
        for c in p {
            if !c.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "bezier1_coordinate_not_finite",
                    format!("control point {i} has a non-finite coordinate: {p:?}"),
                ));
            }
        }
        if p[3] <= 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "bezier1_control_weight_not_positive",
                format!("control point {i} has weight {} which is not > 0", p[3]),
            ));
        }
    }
    Ok(())
}

/// Validate a curve leaf for admission to an R8/R9 residual: re-runs the
/// constructor's structural checks (the fields are public, so a raw leaf can
/// reach a residual constructor) and refuses a transcendental-carrier source.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn validate_curve_leaf(leaf: &BezierLeaf1) -> Result<(), Refusal> {
    if leaf.degree == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier1_zero_degree",
            format!("curve leaf degree {} must be positive", leaf.degree),
        ));
    }
    if leaf.carrier == CurveCarrierKind::Transcendental {
        return Err(refusal(
            RefusalKind::TranscendentalCarrier,
            "r89_curve_transcendental_carrier",
            "a transcendental-only curve carrier cannot be certified by the R8 and R9 residuals"
                .to_string(),
        ));
    }
    validate_curve_net(leaf.degree, &leaf.control)
}

/// Validate a surface leaf for admission to the R8 residual: re-runs the
/// landed `BezierLeaf::try_new` structural checks so a raw leaf cannot reach
/// the residual.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn validate_surface_leaf(leaf: &BezierLeaf) -> Result<(), Refusal> {
    if leaf.degree_u == 0 || leaf.degree_v == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_zero_degree",
            format!(
                "leaf degrees ({}, {}) must be positive",
                leaf.degree_u, leaf.degree_v
            ),
        ));
    }
    let expected = (leaf.degree_u + 1) * (leaf.degree_v + 1);
    if leaf.control.len() != expected {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_control_count_mismatch",
            format!(
                "control net has {} points, degrees ({}, {}) require {expected}",
                leaf.control.len(),
                leaf.degree_u,
                leaf.degree_v
            ),
        ));
    }
    for (i, p) in leaf.control.iter().enumerate() {
        for c in p {
            if !c.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "bezier_coordinate_not_finite",
                    format!("control point {i} has a non-finite coordinate: {p:?}"),
                ));
            }
        }
        if p[3] <= 0.0 {
            return Err(refusal(
                RefusalKind::WeightDegenerate,
                "bezier_control_weight_not_positive",
                format!("control point {i} has weight {} which is not > 0", p[3]),
            ));
        }
    }
    Ok(())
}

/// The `comp`-coordinate coefficient grid of a surface leaf, rows over `u` and
/// columns over `v` (the layout the hull kernels consume).
fn surface_grid(leaf: &BezierLeaf, comp: usize) -> Vec<Vec<f64>> {
    let width = leaf.degree_v + 1;
    (0..=leaf.degree_u)
        .map(|i| {
            (0..=leaf.degree_v)
                .map(|j| leaf.control[i * width + j][comp])
                .collect()
        })
        .collect()
}

/// The `comp`-coordinate grid with one derivative taken along `axis`
/// (`0 == u`, `1 == v`).
fn surface_grid_deriv(leaf: &BezierLeaf, comp: usize, axis: usize) -> Vec<Vec<f64>> {
    bernstein_derivative_2d(&surface_grid(leaf, comp), axis)
}

// ---------------------------------------------------------------------------
// R8 — H(t, u, v) = C(t) − S(u, v), square arity 3
// ---------------------------------------------------------------------------

/// The §7 R8 curve–surface residual (BG-KV2-202-S1A): three equations in
/// `(t, u, v)` over the D-homogeneous cross-multiplied grids
/// `H_k = Cw(t)·S_k(u,v) − C_k(t)·Sw(u,v)` for `k in {x, y, z}`, where `C` is
/// the 1-var curve leaf ([`BezierLeaf1`]) and `S` the landed 2-var surface
/// leaf ([`BezierLeaf`]).
///
/// Eval and Jacobian enclosures are composed from per-factor hulls over the
/// joint box (module-doc "Eval discipline"), so no division by any weight
/// enclosure ever occurs (N5). A `Proven` C1 over a box IS the transversality
/// certificate `det DH ≠ 0`, i.e. `C'(t) ∉ T(u,v)` on the box.
#[derive(Debug, Clone, PartialEq)]
pub struct R8System {
    /// The 1-var curve leaf (homogeneous `xyzw` controls over `t ∈ [0,1]`).
    curve: BezierLeaf1,
    /// The 2-var surface leaf (homogeneous `xyzw` control net over
    /// `(u, v) ∈ [0,1]²`).
    surface: BezierLeaf,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl R8System {
    /// Build the R8 system, refusing a curve or surface leaf that is not a
    /// certified rational homogeneous net (non-finite, zero-degree, or
    /// non-positive-weight data) or a transcendental-carrier curve.
    pub fn try_new(curve: &BezierLeaf1, surface: &BezierLeaf) -> Construction<R8System> {
        validate_curve_leaf(curve)?;
        validate_surface_leaf(surface)?;
        Ok(R8System {
            curve: curve.clone(),
            surface: surface.clone(),
        })
    }

    /// The curve leaf the system certifies over.
    pub fn curve(&self) -> &BezierLeaf1 {
        &self.curve
    }

    /// The surface leaf the system certifies over.
    pub fn surface(&self) -> &BezierLeaf {
        &self.surface
    }
}

impl SquareResidualEval for R8System {
    fn arity(&self) -> usize {
        3
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        if b.len() != 3 {
            return vec![unbounded(); 3];
        }
        let curve = &self.curve;
        let surface = &self.surface;
        let cw = hull1(&curve.coeff(3), &b[0]);
        let sw = hull2(&surface_grid(surface, 3), &b[1], &b[2]);
        let mut out = Vec::with_capacity(3);
        for k in 0..3 {
            let ck = hull1(&curve.coeff(k), &b[0]);
            let sk = hull2(&surface_grid(surface, k), &b[1], &b[2]);
            out.push(cw.mul(&sk).sub(&ck.mul(&sw)));
        }
        out
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        if b.len() != 3 {
            return vec![vec![unbounded(); 3]; 3];
        }
        let curve = &self.curve;
        let surface = &self.surface;
        // Weight and its partials (shared across all three rows).
        let cw = hull1(&curve.coeff(3), &b[0]);
        let cwt = hull1(&curve.coeff_deriv(3), &b[0]);
        let sw = hull2(&surface_grid(surface, 3), &b[1], &b[2]);
        let swu = hull2(&surface_grid_deriv(surface, 3, 0), &b[1], &b[2]);
        let swv = hull2(&surface_grid_deriv(surface, 3, 1), &b[1], &b[2]);
        let mut rows = Vec::with_capacity(3);
        for k in 0..3 {
            let ck = hull1(&curve.coeff(k), &b[0]);
            let ckt = hull1(&curve.coeff_deriv(k), &b[0]);
            let sk = hull2(&surface_grid(surface, k), &b[1], &b[2]);
            let sku = hull2(&surface_grid_deriv(surface, k, 0), &b[1], &b[2]);
            let skv = hull2(&surface_grid_deriv(surface, k, 1), &b[1], &b[2]);
            // dH_k/dt = Cw'(t)·S_k − C_k'(t)·Sw.
            let dt = cwt.mul(&sk).sub(&ckt.mul(&sw));
            // dH_k/du = Cw(t)·S_{k,u} − C_k(t)·S_{w,u}.
            let du = cw.mul(&sku).sub(&ck.mul(&swu));
            // dH_k/dv = Cw(t)·S_{k,v} − C_k(t)·S_{w,v}.
            let dv = cw.mul(&skv).sub(&ck.mul(&swv));
            rows.push(vec![dt, du, dv]);
        }
        rows
    }
}

// ---------------------------------------------------------------------------
// R9 — J(t, r) = C1(t) − C2(r) in one chart, square arity 2
// ---------------------------------------------------------------------------

/// The §7 R9 one-chart curve–curve residual (BG-KV2-202-S1A): two equations
/// in `(t, r)` over the D-homogeneous cross-multiplied grids
/// `H_k = C1w(t)·C2_k(r) − C1_k(t)·C2w(r)` for `k in {x, y}`. Both curve
/// leaves must live in the SAME lifted chart ([`BezierLeaf1::chart`]), which
/// the system records; a mismatched pair is refused at construction.
///
/// The n=2 C1 entry (`krawczyk_c1`) composes the 2D Krawczyk arm of the S2A
/// seam (the landed `formal/bezier_isect.rs` square 2x2 discipline, reached
/// through the seam's n=2 arm) — no second 2D engine is forked here.
#[derive(Debug, Clone, PartialEq)]
pub struct R9System {
    /// The chart both curve leaves are certified in (§3.3 one-chart
    /// discipline).
    pub chart: ChartId,
    /// The first curve leaf (parameter `t`).
    a: BezierLeaf1,
    /// The second curve leaf (parameter `r`).
    b: BezierLeaf1,
}

// Refusal carries Option<PartialGraph> by frozen §2 shape; large-Err is allowed (BG-KV2-000).
#[allow(clippy::result_large_err)]
impl R9System {
    /// Build the R9 system from two curve leaves, refusing a non-rational
    /// leaf (non-finite, zero-degree, or non-positive-weight data, or a
    /// transcendental-carrier curve) and refusing leaves in different lifted
    /// charts with the `r9_requires_one_chart` predicate.
    pub fn try_new(a: &BezierLeaf1, b: &BezierLeaf1) -> Construction<R9System> {
        validate_curve_leaf(a)?;
        validate_curve_leaf(b)?;
        if a.chart != b.chart {
            return Err(refusal(
                RefusalKind::ChartExhausted,
                "r9_requires_one_chart",
                format!(
                    "R9 certifies two curves in ONE lifted chart, but the leaves are in charts \
                     {:?} and {:?}",
                    a.chart, b.chart
                ),
            ));
        }
        Ok(R9System {
            chart: a.chart,
            a: a.clone(),
            b: b.clone(),
        })
    }

    /// The first curve leaf (parameter `t`).
    pub fn a(&self) -> &BezierLeaf1 {
        &self.a
    }

    /// The second curve leaf (parameter `r`).
    pub fn b(&self) -> &BezierLeaf1 {
        &self.b
    }
}

impl SquareResidualEval for R9System {
    fn arity(&self) -> usize {
        2
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        if b.len() != 2 {
            return vec![unbounded(); 2];
        }
        let a = &self.a;
        let b_curve = &self.b;
        let aw = hull1(&a.coeff(3), &b[0]);
        let bw = hull1(&b_curve.coeff(3), &b[1]);
        let mut out = Vec::with_capacity(2);
        for k in 0..2 {
            let ak = hull1(&a.coeff(k), &b[0]);
            let bk = hull1(&b_curve.coeff(k), &b[1]);
            out.push(aw.mul(&bk).sub(&ak.mul(&bw)));
        }
        out
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        if b.len() != 2 {
            return vec![vec![unbounded(); 2]; 2];
        }
        let a = &self.a;
        let b_curve = &self.b;
        let aw = hull1(&a.coeff(3), &b[0]);
        let awt = hull1(&a.coeff_deriv(3), &b[0]);
        let bw = hull1(&b_curve.coeff(3), &b[1]);
        let bwr = hull1(&b_curve.coeff_deriv(3), &b[1]);
        let mut rows = Vec::with_capacity(2);
        for k in 0..2 {
            let ak = hull1(&a.coeff(k), &b[0]);
            let akt = hull1(&a.coeff_deriv(k), &b[0]);
            let bk = hull1(&b_curve.coeff(k), &b[1]);
            let bkr = hull1(&b_curve.coeff_deriv(k), &b[1]);
            // dH_k/dt = C1w'(t)·C2_k(r) − C1_k'(t)·C2w(r).
            let dt = awt.mul(&bk).sub(&akt.mul(&bw));
            // dH_k/dr = C1w(t)·C2_k'(r) − C1_k(t)·C2w'(r).
            let dr = aw.mul(&bkr).sub(&ak.mul(&bwr));
            rows.push(vec![dt, dr]);
        }
        rows
    }
}
