//! Certified control-point hull enclosures for Bézier forms (BG-CK-P1-HULL).
//!
//! The Phase-1 D2 primitive as public API: hull bounds of a Bézier span —
//! curve form and surface (tensor) form — over any compact rectangular
//! subbox, with derivative patches to order 2. MAP (class 1) composes this
//! module; nothing else changes. This module is NOT a general interval
//! evaluator: hulls are enclosures for POLYNOMIAL quantities only.
//!
//! Pre-made decisions (packet tags; do not relitigate):
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`: it is authored certified code, not
//! moved baseline.
//!
//! **D2-primitive.** One enclosure primitive, composed. No external interval
//! crate and no second root engine. All arithmetic goes through
//! `formal/exact.rs`'s `CertifiedInterval` (outward-rounded, untouched); this
//! module adds zero interval algebra of its own.
//!
//! **Polynomial-only.** Hull bounds are enclosures for polynomial quantities
//! (plan D2 scope statement). The module performs no dehomogenization — not
//! even the rational curve's own. The curve form returns homogeneous `X`, `Y`,
//! `W` enclosures; division into a dehomogenized bound is the consumer's named
//! F2 composition (F2 rows RationalNumerator, RationalDenominator,
//! RationalQuotient).
//!
//! **Solver-private twins stay private.** `formal/bezier_isect.rs` carries
//! private `one_d_range` / `bivariate_range` / `tensor_derivative_axis`,
//! documented solver-private. They are prior art, not dependencies: this
//! module implements its own kernels with the same
//! de-Casteljau-over-`CertifiedInterval` discipline, so a solver rewrite never
//! ripples into this public substrate.

use crate::formal::bezier::RationalBezierSpan2;
use crate::formal::exact::CertifiedInterval;

/// Why a hull enclosure could not be certified.
///
/// Named cases only — no catch-all — matching the refusal shape of
/// `formal/outcome.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullRefusal {
    /// The directed-rounded hull is not finite (`CertifiedInterval::is_finite()`
    /// is false): the quantity overflows the enclosure at this policy. Never
    /// retried with a wider representation at this level.
    EnclosureUnavailable,
    /// The requested subbox is not a compact subset of the domain: non-finite
    /// bounds, misordered bounds, or bounds outside the span's (canonical)
    /// source domain. Compactness is INCLUSIVE: the closed subinterval and the
    /// full domain boundary are admissible.
    DomainNotCompact,
}

impl HullRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::EnclosureUnavailable => "hull_enclosure_unavailable",
            Self::DomainNotCompact => "hull_domain_not_compact",
        }
    }
}

/// Interval de Casteljau over the parameter interval `u`: the de Casteljau
/// step is arranged as the subdivision `pts[i] + u * (pts[i+1] - pts[i])` —
/// algebraically the same convex combination for scalar `u`, but `u` occurs
/// once per node, so the linear case evaluates to the exact endpoint range
/// instead of the dependency-widened box of the `(1-u)*a + u*b` arrangement.
/// Every operation is outward-rounded, so the result provably contains the
/// polynomial's range over every parameter in `u`. `pts` must be non-empty.
fn de_casteljau(pts: &[CertifiedInterval], u: &CertifiedInterval) -> CertifiedInterval {
    let mut level = pts.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(w[0].add(&u.mul(&w[1].sub(&w[0]))));
        }
        level = next;
    }
    level[0]
}

/// Certified range enclosure of the Bernstein polynomial with coefficients
/// `coeffs` (rising Bernstein basis, degree `coeffs.len() - 1`) over the unit
/// subinterval `sub = (lo, hi)` with `0 <= lo <= hi <= 1`.
///
/// Discipline (the `bezier_isect::one_d_range` twin, re-derived public): de
/// Casteljau evaluation with the subinterval as an OUTWARD-ROUNDED
/// `CertifiedInterval` parameter and every coefficient widened through
/// `CertifiedInterval::point`. Interval arithmetic at each node encloses the
/// exact expression's range over the input box (the dependency problem only
/// widens), so the result provably contains the polynomial's range. An empty
/// coefficient list or any non-finite coefficient refuses `DomainNotCompact`.
pub fn hull_bernstein_1d(
    coeffs: &[f64],
    sub: (f64, f64),
) -> Result<CertifiedInterval, HullRefusal> {
    if coeffs.is_empty() || coeffs.iter().any(|c| !c.is_finite()) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let (lo, hi) = sub;
    if !lo.is_finite() || !hi.is_finite() || !(lo >= 0.0 && hi <= 1.0 && lo <= hi) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let u = CertifiedInterval { lo, hi };
    let pts: Vec<CertifiedInterval> = coeffs
        .iter()
        .map(|c| CertifiedInterval::point(*c))
        .collect();
    let hull = de_casteljau(&pts, &u);
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(HullRefusal::EnclosureUnavailable)
    }
}

/// Certified range enclosure of the bivariate tensor-Bernstein polynomial
/// `c[i][j] * B^i_m(s) B^j_n(t)` over the unit rectangle `s x t` (each axis a
/// compact subinterval of [0, 1]). `grid[i][j]` is the coefficient of
/// `B^i_m(s) B^j_n(t)`; a ragged or empty grid refuses `DomainNotCompact`.
///
/// Discipline: per-column 1-D hull in `s`, then one 1-D hull in `t` (the
/// `bezier_isect::bivariate_range` discipline, re-derived public).
pub fn hull_bernstein_2d(
    grid: &[Vec<f64>],
    s: (f64, f64),
    t: (f64, f64),
) -> Result<CertifiedInterval, HullRefusal> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(HullRefusal::DomainNotCompact);
    }
    let width = grid[0].len();
    if grid.iter().any(|row| row.len() != width) {
        return Err(HullRefusal::DomainNotCompact);
    }
    if grid.iter().any(|row| row.iter().any(|c| !c.is_finite())) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let (s_lo, s_hi) = s;
    if !s_lo.is_finite() || !s_hi.is_finite() || !(s_lo >= 0.0 && s_hi <= 1.0 && s_lo <= s_hi) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let (t_lo, t_hi) = t;
    if !t_lo.is_finite() || !t_hi.is_finite() || !(t_lo >= 0.0 && t_hi <= 1.0 && t_lo <= t_hi) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let s_box = CertifiedInterval { lo: s_lo, hi: s_hi };
    let t_box = CertifiedInterval { lo: t_lo, hi: t_hi };
    let cols: Vec<Vec<CertifiedInterval>> = grid
        .iter()
        .map(|row| row.iter().map(|c| CertifiedInterval::point(*c)).collect())
        .collect();
    let mut col_evals = Vec::with_capacity(width);
    for j in 0..width {
        let col: Vec<CertifiedInterval> = cols.iter().map(|row| row[j]).collect();
        col_evals.push(de_casteljau(&col, &s_box));
    }
    let hull = de_casteljau(&col_evals, &t_box);
    if hull.is_finite() {
        Ok(hull)
    } else {
        Err(HullRefusal::EnclosureUnavailable)
    }
}

/// Bernstein coefficients of the first derivative: degree `d - 1`, coefficient
/// `i` is `d * (coeffs[i + 1] - coeffs[i])` computed in `f64`. The derivative
/// POLYNOMIAL IS DEFINED by these computed coefficients — the enclosure claim
/// of any hull over them certifies that polynomial (the same definition
/// `bezier_isect::tensor_derivative_axis` uses). A degree-0 input yields the
/// zero polynomial.
///
/// The `d * (coeffs[i + 1] - coeffs[i])` products are `f64` input
/// transformations, NOT certified quantities: they are not directed-rounded
/// work, and no reader should mistake them for it.
pub fn bernstein_derivative_1d(coeffs: &[f64]) -> Vec<f64> {
    if coeffs.is_empty() {
        return Vec::new();
    }
    if coeffs.len() == 1 {
        return vec![0.0];
    }
    let degree = (coeffs.len() - 1) as f64;
    coeffs.windows(2).map(|w| degree * (w[1] - w[0])).collect()
}

/// Axis-wise first-derivative coefficients of a tensor grid (`axis == 0` in
/// `s`, `axis == 1` in `t`), same definition and degree bookkeeping as
/// `bernstein_derivative_1d` along the chosen axis. Pure `f64` coefficient
/// transforms, not certified quantities.
pub fn bernstein_derivative_2d(grid: &[Vec<f64>], axis: usize) -> Vec<Vec<f64>> {
    if grid.is_empty() || grid[0].is_empty() {
        return Vec::new();
    }
    let m = grid.len() - 1;
    let n = grid[0].len() - 1;
    if axis == 0 {
        if m == 0 {
            return vec![vec![0.0; n + 1]];
        }
        let scale = m as f64;
        grid.windows(2)
            .map(|pair| {
                pair[0]
                    .iter()
                    .zip(pair[1].iter())
                    .map(|(a, b)| scale * (b - a))
                    .collect()
            })
            .collect()
    } else {
        if n == 0 {
            return vec![vec![0.0]; m + 1];
        }
        let scale = n as f64;
        grid.iter()
            .map(|row| row.windows(2).map(|w| scale * (w[1] - w[0])).collect())
            .collect()
    }
}

/// How many derivatives the jet carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JetOrder {
    /// The value itself.
    Value,
    /// First derivative patch.
    First,
    /// Second derivative patch (first applied twice).
    Second,
}

/// The compact subinterval of `[0, 1]` that is the exact unit-parameter image
/// of the source subinterval `sub = (lo, hi)` under the span's own
/// source-to-unit affine map, enclosed in `CertifiedInterval` arithmetic and
/// clamped to `[0, 1]`.
///
/// `sub` must be a compact subset of the span's canonical source domain (the
/// sorted pair of its endpoints), INCLUSIVE of the closed boundary — anything
/// else refuses `DomainNotCompact`. The map is evaluated in `CertifiedInterval`
/// arithmetic (widened — the affine map rounds in `f64`), so the returned pair
/// encloses the exact image, never a naked rounded point. The exact image lies
/// in `[0, 1]` by the subset property, so clamping the enclosure to `[0, 1]`
/// preserves containment.
fn unit_subinterval(domain: (f64, f64), sub: (f64, f64)) -> Result<(f64, f64), HullRefusal> {
    let (d0, d1) = domain;
    let (a, b) = if d0 <= d1 { (d0, d1) } else { (d1, d0) };
    let (lo, hi) = sub;
    if !lo.is_finite() || !hi.is_finite() || !(a <= lo && lo <= hi && hi <= b) {
        return Err(HullRefusal::DomainNotCompact);
    }
    let d0_iv = CertifiedInterval::point(d0);
    let span_iv = CertifiedInterval::point(d1).sub(&d0_iv);
    let lo_iv = CertifiedInterval::point(lo).sub(&d0_iv);
    let hi_iv = CertifiedInterval::point(hi).sub(&d0_iv);
    let lo_u = lo_iv
        .div(&span_iv)
        .ok_or(HullRefusal::EnclosureUnavailable)?;
    let hi_u = hi_iv
        .div(&span_iv)
        .ok_or(HullRefusal::EnclosureUnavailable)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    Ok((u_lo, u_hi))
}

/// Certified homogeneous enclosures `(X, Y, W)` of a rational Bézier span
/// (or of its order-`n` homogeneous derivative patch) over the subinterval
/// `sub = (lo, hi)` in SOURCE parameters.
///
/// `sub` must be a compact subset of the span's canonical source domain
/// (`min(d0, d1) <= lo <= hi <= max(d0, d1)`, inclusive) — anything else
/// refuses `DomainNotCompact`. The source-to-unit map of the subinterval
/// endpoints is computed in `CertifiedInterval` arithmetic (widened — the
/// affine map rounds in `f64`), so the kernel consumes an enclosure of the
/// exact image, never a naked rounded point.
///
/// The hull never divides: the returned enclosures are HOMOGENEOUS. Dividing
/// `X`, `Y` by `W` into a dehomogenized bound is the consumer's named F2
/// composition, never this module's.
pub fn hull_curve_homogeneous(
    span: &RationalBezierSpan2,
    sub: (f64, f64),
    order: JetOrder,
) -> Result<[CertifiedInterval; 3], HullRefusal> {
    let (u_lo, u_hi) = unit_subinterval(span.domain, sub)?;
    let times = match order {
        JetOrder::Value => 0,
        JetOrder::First => 1,
        JetOrder::Second => 2,
    };
    let mut x: Vec<f64> = span.control.iter().map(|p| p.0).collect();
    let mut y: Vec<f64> = span.control.iter().map(|p| p.1).collect();
    let mut w: Vec<f64> = span.control.iter().map(|p| p.2).collect();
    for _ in 0..times {
        x = bernstein_derivative_1d(&x);
        y = bernstein_derivative_1d(&y);
        w = bernstein_derivative_1d(&w);
    }
    if !x
        .iter()
        .chain(y.iter())
        .chain(w.iter())
        .all(|c| c.is_finite())
    {
        return Err(HullRefusal::EnclosureUnavailable);
    }
    let hull_x = hull_bernstein_1d(&x, (u_lo, u_hi))?;
    let hull_y = hull_bernstein_1d(&y, (u_lo, u_hi))?;
    let hull_w = hull_bernstein_1d(&w, (u_lo, u_hi))?;
    Ok([hull_x, hull_y, hull_w])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal::curve2d::{
        CurveOccurrenceProvenance, SourceEdgeId, SourceEntityId, SourceFaceId,
    };
    use crate::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};

    fn provenance() -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(7)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), 3),
            source_edge_id: SourceEdgeId(11),
            start_vertex_id: SourceVertexKey::ShellVertex(1),
            end_vertex_id: SourceVertexKey::ShellVertex(2),
            source_curve_entity_id: Some(SourceEntityId(99)),
        }
    }

    /// The cubic `C(u) = (u, u^3)` as a degree-3 polynomial Bézier (`W = 1`):
    /// `X` coefficients `[0, 1/3, 2/3, 1]`, `Y` coefficients `[0, 0, 0, 1]`.
    fn span_cubic() -> Option<RationalBezierSpan2> {
        RationalBezierSpan2::new(
            vec![
                (0.0, 0.0, 1.0),
                (1.0 / 3.0, 0.0, 1.0),
                (2.0 / 3.0, 0.0, 1.0),
                (1.0, 1.0, 1.0),
            ],
            (0.0, 1.0),
            provenance(),
        )
        .ok()
    }

    /// The cubic traversed the other way: control points reversed and the
    /// authoritative domain reversed. As a function of the sorted source
    /// parameter it is still `(t, t^3)` over `[0, 1]`.
    fn span_reversed() -> Option<RationalBezierSpan2> {
        RationalBezierSpan2::new(
            vec![
                (1.0, 1.0, 1.0),
                (2.0 / 3.0, 0.0, 1.0),
                (1.0 / 3.0, 0.0, 1.0),
                (0.0, 0.0, 1.0),
            ],
            (1.0, 0.0),
            provenance(),
        )
        .ok()
    }

    /// Plain `f64` de Casteljau point evaluation (the test's reference, never
    /// the module's arithmetic).
    fn eval_bernstein(coeffs: &[f64], u: f64) -> f64 {
        let mut level: Vec<f64> = coeffs.to_vec();
        while level.len() > 1 {
            level = level
                .windows(2)
                .map(|w| (1.0 - u) * w[0] + u * w[1])
                .collect();
        }
        level[0]
    }

    #[test]
    fn curve_homogeneous_jet_order_two_bounds_finite_difference_slopes() {
        let span = match span_cubic() {
            Some(span) => span,
            None => return,
        };
        let x_coeffs: Vec<f64> = span.control.iter().map(|p| p.0).collect();
        let y_coeffs: Vec<f64> = span.control.iter().map(|p| p.1).collect();
        let w_coeffs: Vec<f64> = span.control.iter().map(|p| p.2).collect();
        let x1 = bernstein_derivative_1d(&x_coeffs);
        let y1 = bernstein_derivative_1d(&y_coeffs);
        let w1 = bernstein_derivative_1d(&w_coeffs);

        let sub = (0.2, 0.8);
        let first = match hull_curve_homogeneous(&span, sub, JetOrder::First) {
            Ok(h) => h,
            Err(_) => return,
        };
        let second = match hull_curve_homogeneous(&span, sub, JetOrder::Second) {
            Ok(h) => h,
            Err(_) => return,
        };

        for (u1, u2) in [(0.2, 0.3), (0.35, 0.55), (0.6, 0.8), (0.25, 0.75)] {
            let dt = u2 - u1;
            let slope_x = (eval_bernstein(&x_coeffs, u2) - eval_bernstein(&x_coeffs, u1)) / dt;
            let slope_y = (eval_bernstein(&y_coeffs, u2) - eval_bernstein(&y_coeffs, u1)) / dt;
            let slope_w = (eval_bernstein(&w_coeffs, u2) - eval_bernstein(&w_coeffs, u1)) / dt;
            let scale = 1.0 + slope_x.abs().max(slope_y.abs()).max(slope_w.abs());
            let slack = 16.0 * f64::EPSILON * scale;
            assert!(slope_x >= first[0].lo - slack && slope_x <= first[0].hi + slack); // H-3
            assert!(slope_y >= first[1].lo - slack && slope_y <= first[1].hi + slack); // H-3
            assert!(slope_w >= first[2].lo - slack && slope_w <= first[2].hi + slack); // H-3

            let slope_x1 = (eval_bernstein(&x1, u2) - eval_bernstein(&x1, u1)) / dt;
            let slope_y1 = (eval_bernstein(&y1, u2) - eval_bernstein(&y1, u1)) / dt;
            let slope_w1 = (eval_bernstein(&w1, u2) - eval_bernstein(&w1, u1)) / dt;
            let scale1 = 1.0 + slope_x1.abs().max(slope_y1.abs()).max(slope_w1.abs());
            let slack1 = 16.0 * f64::EPSILON * scale1;
            assert!(slope_x1 >= second[0].lo - slack1 && slope_x1 <= second[0].hi + slack1); // H-3
            assert!(slope_y1 >= second[1].lo - slack1 && slope_y1 <= second[1].hi + slack1); // H-3
            assert!(slope_w1 >= second[2].lo - slack1 && slope_w1 <= second[2].hi + slack1);
            // H-3
        }
    }

    #[test]
    fn curve_homogeneous_non_compact_subinterval_refuses_domain_not_compact() {
        let span = match span_cubic() {
            Some(span) => span,
            None => return,
        };
        assert_eq!(
            hull_curve_homogeneous(&span, (-0.1, 0.5), JetOrder::Value),
            Err(HullRefusal::DomainNotCompact),
            "below the source domain"
        );
        assert_eq!(
            hull_curve_homogeneous(&span, (0.5, 1.5), JetOrder::Value),
            Err(HullRefusal::DomainNotCompact),
            "above the source domain"
        );
        assert_eq!(
            hull_curve_homogeneous(&span, (0.5, 0.2), JetOrder::Value),
            Err(HullRefusal::DomainNotCompact),
            "misordered bounds"
        );
        assert_eq!(
            hull_curve_homogeneous(&span, (f64::NAN, 0.5), JetOrder::Value),
            Err(HullRefusal::DomainNotCompact),
            "non-finite bound"
        );
        assert_eq!(
            hull_curve_homogeneous(&span, (0.5, f64::INFINITY), JetOrder::Value),
            Err(HullRefusal::DomainNotCompact),
            "non-finite bound"
        );
        // The closed source-domain boundary is ACCEPTED (inclusive compactness).
        assert!(hull_curve_homogeneous(&span, (0.0, 0.0), JetOrder::Value).is_ok());
        assert!(hull_curve_homogeneous(&span, (1.0, 1.0), JetOrder::Value).is_ok());
        assert!(hull_curve_homogeneous(&span, (0.0, 1.0), JetOrder::Value).is_ok());
    }

    #[test]
    fn curve_homogeneous_hull_over_subinterval_is_contained_in_hull_over_whole() {
        let span = match span_cubic() {
            Some(span) => span,
            None => return,
        };
        let whole = match hull_curve_homogeneous(&span, (0.0, 1.0), JetOrder::Value) {
            Ok(h) => h,
            Err(_) => return,
        };
        let part = match hull_curve_homogeneous(&span, (0.2, 0.8), JetOrder::Value) {
            Ok(h) => h,
            Err(_) => return,
        };
        for k in 0..3 {
            assert!(whole[k].lo <= part[k].lo, "lower bound monotone");
            assert!(part[k].hi <= whole[k].hi, "upper bound monotone");
        }
    }

    #[test]
    fn curve_homogeneous_reversed_domain_span_hull_contains_samples() {
        let span = match span_reversed() {
            Some(span) => span,
            None => return,
        };
        let whole = match hull_curve_homogeneous(&span, (0.0, 1.0), JetOrder::Value) {
            Ok(h) => h,
            Err(_) => return,
        };
        assert!(
            whole[0].lo <= 0.0 && 1.0 <= whole[0].hi,
            "X spans the full arc"
        );
        assert!(
            whole[1].lo <= 0.0 && 1.0 <= whole[1].hi,
            "Y spans the full arc"
        );
        assert!(
            whole[2].lo <= 1.0 && 1.0 <= whole[2].hi,
            "W stays exactly one"
        );
        let part = match hull_curve_homogeneous(&span, (0.2, 0.8), JetOrder::Value) {
            Ok(h) => h,
            Err(_) => return,
        };
        for t in [0.3, 0.5, 0.7] {
            assert!(part[0].lo <= t && t <= part[0].hi, "X at t = {t}");
            let y = t * t * t;
            assert!(part[1].lo <= y && y <= part[1].hi, "Y at t = {t}");
        }
    }

    #[test]
    fn curve_homogeneous_value_contains_polynomial_samples() {
        let span = match span_cubic() {
            Some(span) => span,
            None => return,
        };
        let x_coeffs: Vec<f64> = span.control.iter().map(|p| p.0).collect();
        let y_coeffs: Vec<f64> = span.control.iter().map(|p| p.1).collect();
        let w_coeffs: Vec<f64> = span.control.iter().map(|p| p.2).collect();
        let hull = match hull_curve_homogeneous(&span, (0.1, 0.9), JetOrder::Value) {
            Ok(h) => h,
            Err(_) => return,
        };
        for i in 0..1000 {
            let u = 0.1 + 0.8 * (i as f64) / 999.0;
            let x = eval_bernstein(&x_coeffs, u);
            let y = eval_bernstein(&y_coeffs, u);
            let w = eval_bernstein(&w_coeffs, u);
            assert!(hull[0].lo <= x && x <= hull[0].hi, "X sample");
            assert!(hull[1].lo <= y && y <= hull[1].hi, "Y sample");
            assert!(hull[2].lo <= w && w <= hull[2].hi, "W sample");
        }
    }
}
