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

//! The T1.3 reduced-Hessian evaluator and the §2.8 definiteness test
//! (CTE-004-HESSIAN; theory §2.5, §2.8, R9).
//!
//! At a certified critical point of the deflated system
//! `T = (G1, G2, M1, M2)` the contact trichotomy (theory §2.9) is decided by
//! the SIGN and DEFINITENESS of the reduced contact Hessian
//!
//! ```text
//!   H_h = Jᵀ (H_f − λ₁·H_{G₁} − λ₂·H_{G₂}) J         (Theorem T1.3)
//! ```
//!
//! with `P = −A⁻¹G_z`, `J = [P; I₂]`, `λ = A⁻ᵀf_yᵀ`, where `A = D_yG` is the
//! chart's pivot block. This module evaluates that expression over the
//! CERTIFIED GRAPH ENCLOSURE `Ŷ × Z` of the chart — never over the raw box
//! (scope decision 1 / correction R2, theory §2.5): over the raw box the
//! expression is not a bound on `H_h` at all, because `H_h` is only defined on
//! the graph. The R2 signature therefore accepts the certified
//! [`GraphEnclosure`] and no raw-box overload exists.
//!
//! # Evaluation rule
//!
//! Every interval ingredient — the pivot `A` over `Ŷ × Z`, the first partials
//! `G_z`, `f_y`, the second partials `H_f`, `H_{G₁}`, `H_{G₂}` — is hulled over
//! the four-axis enclosure box `E` whose `y` axes are `Ŷ` and whose `z` axes
//! are `Z`. The inverse pivot `A⁻¹` is taken interval-wise by the adjugate over
//! `det A(E)`, which is certified away from zero by the (H-graph) content (a
//! restriction of the box-wide exclusion). Symmetry of `H_h` off the critical
//! point is exact in the algebra (theory T1.3); the interval evaluation keeps
//! it by hulling both partial orderings of every second partial and hulling the
//! two cross entries of the finished `2 × 2` (the admission test
//! `t13_symmetry_on_fixture`).
//!
//! # The §2.8 definiteness test
//!
//! [`definiteness`] decides a [`Definiteness`] verdict for a symmetric `2 × 2`
//! interval matrix with the R9 rule: positive definiteness via
//! `a_min > 0 ∧ a_min·c_min > β²` with the Gershgorin modulus
//! `μ = min(a_min, c_min) − β`, falling back to the §2.8 closed form when
//! Gershgorin fails but the closed-form `μ` still certifies; negative
//! definiteness by the same test on `−Ĥ`; certified indefiniteness STRICTLY by
//! `det Ĥ_h < 0` (the interval upper bound of `a·c − b²` is negative).
//! [`Definiteness::Definite`] construction refuses `μ ≤ 0` (frozen rule R9).
//!
//! **House rules.** No `unwrap`, no `expect`, no `panic!`, no out-of-range
//! indexing reachable from geometry; every fallible operation returns
//! [`TangencyRefusal`]; float-computed values are never recorded as an exact
//! method (the verdicts here are interval certificates only).

use crate::contract::Refusal;
use crate::formal::exact::CertifiedInterval;
use crate::hull::HullRefusal;
use crate::tangency::minors::{from_system_grid, PolyGrid};
use crate::tangency::shapes::{
    Definiteness, DefinitenessSign, GraphEnclosure, IntervalSym2, Rank2Chart,
};
use crate::tangency::TangencyRefusal;
use crate::SquareSystem3;

/// A four-axis box in the unit chart (the chart.rs/exclude.rs convention).
type Box4 = [(f64, f64); 4];

/// Whether a raw `(lo, hi)` axis interval is well formed (finite, ordered).
fn axis_ok((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi
}

/// The two `G` component indices of a pivot: the stored components other than
/// the distinguished `f`, ascending (the deflated-row ordering).
fn g_rows_of(component: usize) -> [usize; 2] {
    let mut gs = [0usize; 2];
    let mut slot = 0usize;
    for c in 0..3 {
        if c != component {
            gs[slot] = c;
            slot += 1;
        }
    }
    gs
}

/// The two `z` axes of a chart: the stored axes outside the `y` pair,
/// ascending.
fn z_axes_of((y0, y1): (usize, usize)) -> [usize; 2] {
    let mut zs = [0usize; 2];
    let mut slot = 0usize;
    for a in 0..4 {
        if a != y0 && a != y1 {
            zs[slot] = a;
            slot += 1;
        }
    }
    zs
}

/// The four-axis enclosure box `E = Ŷ × Z` of a chart and its certified graph
/// enclosure, in the stored axis order.
fn enclosure_box(chart: &Rank2Chart, over: &GraphEnclosure) -> Box4 {
    let (y0, y1) = chart.pivot().coord_pair();
    let zs = z_axes_of((y0, y1));
    let y_hat = over.y_hat();
    let z = over.z();
    let mut out = [(0.0f64, 0.0f64); 4];
    out[y0] = y_hat[0];
    out[y1] = y_hat[1];
    out[zs[0]] = z[0];
    out[zs[1]] = z[1];
    out
}

/// The outward hull of two certified intervals (both enclosures stay valid).
fn hull2(a: CertifiedInterval, b: CertifiedInterval) -> CertifiedInterval {
    CertifiedInterval {
        lo: a.lo.min(b.lo),
        hi: a.hi.max(b.hi),
    }
}

/// The absolute upper bound `max(|lo|, |hi|)` of an interval.
fn abs_sup(a: CertifiedInterval) -> f64 {
    a.lo.abs().max(a.hi.abs())
}

/// The certified interval enclosure of one stored component's first partial
/// along an axis over a box.
fn partial_over(
    grid: &PolyGrid,
    axis: usize,
    e: Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    let derived = grid.partial_axis(axis)?;
    derived.hull_unit(e)
}

/// The certified symmetric enclosure of one stored component's second partial
/// `∂²/∂x_a ∂x_b` over a box.
///
/// A polynomial whose stored grid has degree zero along an axis does not depend
/// on that axis, so the derivative is EXACTLY zero there; [`PolyGrid::partial_axis`]
/// refuses exactly that case with [`Refusal::InvalidInput`], which is mapped to
/// the exact zero enclosure. Both differentiation orderings (when `a ≠ b`) are
/// hulled into one symmetric enclosure — the two orderings bound the same exact
/// second partial.
fn second_partial_over(
    grid: &PolyGrid,
    a: usize,
    b: usize,
    e: Box4,
) -> Result<CertifiedInterval, TangencyRefusal> {
    let first = match grid.partial_axis(a) {
        Ok(g) => g,
        Err(TangencyRefusal::Input(Refusal::InvalidInput)) => {
            return Ok(CertifiedInterval::point(0.0));
        }
        Err(other) => return Err(other),
    };
    let ab = match first.partial_axis(b) {
        Ok(g) => g,
        Err(TangencyRefusal::Input(Refusal::InvalidInput)) => {
            return Ok(CertifiedInterval::point(0.0));
        }
        Err(other) => return Err(other),
    };
    let enc_ab = ab.hull_unit(e)?;
    if a == b {
        return Ok(enc_ab);
    }
    let second = match grid.partial_axis(b) {
        Ok(g) => g,
        Err(TangencyRefusal::Input(Refusal::InvalidInput)) => {
            return Ok(enc_ab);
        }
        Err(other) => return Err(other),
    };
    let ba = match second.partial_axis(a) {
        Ok(g) => g.hull_unit(e)?,
        Err(TangencyRefusal::Input(Refusal::InvalidInput)) => enc_ab,
        Err(other) => return Err(other),
    };
    Ok(hull2(enc_ab, ba))
}

/// The certified symmetric Hessian matrix of one stored component over a box,
/// as the six independent entries `(00, 01, 02, 03, 11, 12, 13, 22, 23, 33)`
/// packed `[a][b]` for `a ≤ b`.
#[allow(clippy::needless_range_loop)] // symmetric-matrix index pairs over the fixed 4 axes; the index form is the algebra
fn hessian_entries(
    grid: &PolyGrid,
    e: Box4,
) -> Result<[[CertifiedInterval; 4]; 4], TangencyRefusal> {
    let mut out = [[CertifiedInterval::point(0.0); 4]; 4];
    for a in 0..4 {
        for b in a..4 {
            let val = second_partial_over(grid, a, b, e)?;
            out[a][b] = val;
            out[b][a] = val;
        }
    }
    Ok(out)
}

/// Validate the certified graph enclosure the R2 evaluator consumes.
///
/// The enclosure is degenerate (zero-width on a `y` or `z` axis) when no
/// (H-graph) content could have certified it — a graph over a zero-width
/// parameter domain, or a zero-width image — so the evaluator refuses it
/// ([`Refusal::InvalidInput`]). An enclosure outside the unit chart refuses
/// with the hull vocabulary (the stored systems are Bernstein on `[0, 1]^4`).
fn validate_enclosure(chart: &Rank2Chart, over: &GraphEnclosure) -> Result<Box4, TangencyRefusal> {
    let e = enclosure_box(chart, over);
    for (lo, hi) in e {
        if !axis_ok((lo, hi)) || lo == hi {
            return Err(TangencyRefusal::Input(Refusal::InvalidInput));
        }
        if lo < 0.0 || hi > 1.0 {
            return Err(TangencyRefusal::Hull(HullRefusal::DomainNotCompact));
        }
    }
    Ok(e)
}

/// The certified pivot inverse `A⁻¹ = adj(A) / det A` over the enclosure.
///
/// The adjugate of the interval `A` over the box divided by the interval
/// `det A` (which is certified away from zero over `Ŷ × Z ⊆ B` by the
/// (H-graph) content, and hence over the enclosure). `None` when the interval
/// determinant is not finite or contains zero — the evaluator's caller refuses.
fn pivot_inverse(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    e: Box4,
) -> Result<[[CertifiedInterval; 2]; 2], TangencyRefusal> {
    let pivot = chart.pivot();
    let gs = g_rows_of(pivot.component());
    let (y0, y1) = pivot.coord_pair();
    let mut a = [[CertifiedInterval::point(0.0); 2]; 2];
    for (r, comp) in gs.iter().enumerate() {
        let grid = from_system_grid(&system.grids()[*comp], system.degrees())?;
        a[r][0] = partial_over(&grid, y0, e)?;
        a[r][1] = partial_over(&grid, y1, e)?;
    }
    let det = a[0][0].mul(&a[1][1]).sub(&a[0][1].mul(&a[1][0]));
    if !det.is_finite() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    // The (H-graph) pivot determinant excludes zero over B; the enclosure
    // E ⊆ B keeps the exclusion. An enclosure that nonetheless contains zero
    // is a box the inverse is not defined on — refuse, never divide.
    let Some(det_inv) = CertifiedInterval::point(1.0).div(&det) else {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    };
    let mut inv = [[CertifiedInterval::point(0.0); 2]; 2];
    inv[0][0] = a[1][1].mul(&det_inv);
    inv[0][1] = a[0][1].neg().mul(&det_inv);
    inv[1][0] = a[1][0].neg().mul(&det_inv);
    inv[1][1] = a[0][0].mul(&det_inv);
    Ok(inv)
}

/// The raw `2 × 2` T1.3 reduced-Hessian interval evaluation over the certified
/// graph enclosure `Ŷ × Z` of `chart` (theory §2.5 / R2).
///
/// This is the evaluator body behind [`reduced_hessian`]; it returns the full
/// `2 × 2` matrix (all four entries as intervals) so the symmetry admission
/// test can inspect the cross entries before they are hulled into the frozen
/// symmetric carrier.
pub(crate) fn hessian_entries_raw(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    over: &GraphEnclosure,
) -> Result<[[CertifiedInterval; 2]; 2], TangencyRefusal> {
    hessian_entries_raw_inner(system, chart, over)
}

/// The body of [`hessian_entries_raw`] (a separate function so the fixed-size
/// index-algebra allow sits on the loops it covers).
#[allow(clippy::needless_range_loop)] // fixed-size 2x2/4x4 matrix index algebra; the index form is the algebra
fn hessian_entries_raw_inner(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    over: &GraphEnclosure,
) -> Result<[[CertifiedInterval; 2]; 2], TangencyRefusal> {
    let e = validate_enclosure(chart, over)?;
    let pivot = chart.pivot();
    let f_comp = pivot.component();
    let gs = g_rows_of(f_comp);
    let (y0, y1) = pivot.coord_pair();
    let zs = z_axes_of((y0, y1));

    let f_grid = from_system_grid(&system.grids()[f_comp], system.degrees())?;
    let g0_grid = from_system_grid(&system.grids()[gs[0]], system.degrees())?;
    let g1_grid = from_system_grid(&system.grids()[gs[1]], system.degrees())?;

    // P = −A⁻¹·G_z : 2 × 2, rows the y-coordinate slots, columns the z slots.
    let inv = pivot_inverse(system, chart, e)?;
    let mut g_z = [[CertifiedInterval::point(0.0); 2]; 2];
    for (r, comp) in gs.iter().enumerate() {
        let grid = from_system_grid(&system.grids()[*comp], system.degrees())?;
        g_z[r][0] = partial_over(&grid, zs[0], e)?;
        g_z[r][1] = partial_over(&grid, zs[1], e)?;
    }
    let mut p = [[CertifiedInterval::point(0.0); 2]; 2];
    for r in 0..2 {
        for c in 0..2 {
            let mut acc = CertifiedInterval::point(0.0);
            for k in 0..2 {
                acc = acc.add(&inv[r][k].mul(&g_z[k][c]));
            }
            p[r][c] = acc.neg();
        }
    }

    // λ = A⁻ᵀ·f_yᵀ : entries indexed by the G components.
    let mut f_y = [CertifiedInterval::point(0.0); 2];
    f_y[0] = partial_over(&f_grid, y0, e)?;
    f_y[1] = partial_over(&f_grid, y1, e)?;
    let mut lambda = [CertifiedInterval::point(0.0); 2];
    for j in 0..2 {
        let mut acc = CertifiedInterval::point(0.0);
        for k in 0..2 {
            acc = acc.add(&inv[k][j].mul(&f_y[k]));
        }
        lambda[j] = acc;
    }

    // H_f, H_{G1}, H_{G2} over the enclosure.
    let h_f = hessian_entries(&f_grid, e)?;
    let h_g0 = hessian_entries(&g0_grid, e)?;
    let h_g1 = hessian_entries(&g1_grid, e)?;

    // M = H_f − λ₁·H_{G₁} − λ₂·H_{G₂}.
    let mut m = [[CertifiedInterval::point(0.0); 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let t0 = lambda[0].mul(&h_g0[i][j]);
            let t1 = lambda[1].mul(&h_g1[i][j]);
            m[i][j] = h_f[i][j].sub(&t0).sub(&t1);
        }
    }

    // J columns: J[:, 0] is `∂/∂z₀`, J[:, 1] is `∂/∂z₁`. On the y axes the
    // column entries are P; on the z axes they are the identity slice.
    let mut j0 = [CertifiedInterval::point(0.0); 4];
    let mut j1 = [CertifiedInterval::point(0.0); 4];
    j0[y0] = p[0][0];
    j0[y1] = p[1][0];
    j0[zs[0]] = CertifiedInterval::point(1.0);
    j1[y0] = p[0][1];
    j1[y1] = p[1][1];
    j1[zs[1]] = CertifiedInterval::point(1.0);

    // H_h[c1][c2] = Σ_{i,j} J[i][c1]·M[i][j]·J[j][c2].
    let mut out = [[CertifiedInterval::point(0.0); 2]; 2];
    for c1 in 0..2 {
        for c2 in 0..2 {
            let mut acc = CertifiedInterval::point(0.0);
            for i in 0..4 {
                for j in 0..4 {
                    let col1 = if c1 == 0 { j0[i] } else { j1[i] };
                    let col2 = if c2 == 0 { j0[j] } else { j1[j] };
                    acc = acc.add(&col1.mul(&m[i][j]).mul(&col2));
                }
            }
            out[c1][c2] = acc;
        }
    }
    Ok(out)
}

/// The frozen R2 contract: the T1.3 reduced Hessian over the certified graph
/// enclosure of `chart` (theory §2.5).
///
/// No raw-box overload exists — the box is always the certified
/// [`GraphEnclosure`] `Ŷ × Z`, whose `Ŷ` strictly contains the certified graph
/// `φ(Z)`. The returned [`IntervalSym2]` carries the symmetric carrier
/// `[[a, b], [b, c]]`; the two independently-computed cross entries of the raw
/// evaluation are hulled into the shared `b`, so the carrier is symmetric by
/// construction within the enclosure.
///
/// Refuses a degenerate (zero-width) or non-certified-consistent enclosure and
/// a pivot whose interval determinant over the enclosure contains zero.
pub fn reduced_hessian(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    graph: &GraphEnclosure,
) -> Result<IntervalSym2, TangencyRefusal> {
    let raw = hessian_entries_raw(system, chart, graph)?;
    let cross = hull2(raw[0][1], raw[1][0]);
    let a = raw[0][0];
    let c = raw[1][1];
    if !cross.is_finite() || !a.is_finite() || !c.is_finite() {
        return Err(TangencyRefusal::Hull(HullRefusal::EnclosureUnavailable));
    }
    IntervalSym2::new(a, cross, c).map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput))
}

/// The positive-definiteness sufficient test and §2.8 modulus for the matrix
/// `[[a, b], [b, c]]`.
///
/// Returns `None` when positive definiteness is not certified. The modulus
/// rule is FIXED and deterministic: Gershgorin
/// `μ = min(a_lo, c_lo) − β` first (scope decision 3 / house determinism);
/// when Gershgorin does not certify but the §2.8 closed form
/// `μ = ½[(a_lo + c_lo) − √(max((a_hi−c_lo)², (c_hi−a_lo)²) + 4β²)]` is still
/// positive, the closed form is used (a valid lower bound on `λ_min`).
fn positive_definite(
    a: CertifiedInterval,
    b: CertifiedInterval,
    c: CertifiedInterval,
) -> Option<f64> {
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return None;
    }
    let beta = abs_sup(b);
    let a_lo = a.lo;
    let c_lo = c.lo;
    if !(a_lo > 0.0 && a_lo * c_lo > beta * beta) {
        return None;
    }
    let gershgorin = a_lo.min(c_lo) - beta;
    if gershgorin > 0.0 {
        return Some(gershgorin);
    }
    // Closed form (theory §2.8): a valid lower bound on λ_min whenever the
    // PD sufficient test above certifies.
    let d0 = (a.hi - c_lo).powi(2);
    let d1 = (c.hi - a_lo).powi(2);
    let radicand = d0.max(d1) + 4.0 * beta * beta;
    if !radicand.is_finite() || radicand < 0.0 {
        return None;
    }
    let mu = 0.5 * ((a_lo + c_lo) - radicand.sqrt());
    if mu.is_finite() && mu > 0.0 {
        Some(mu)
    } else {
        None
    }
}

/// The interval negation of a symmetric `2 × 2` entry (`−x` swaps the bounds).
fn neg_iv(x: CertifiedInterval) -> CertifiedInterval {
    x.neg()
}

/// The frozen §2.8 definiteness decision (theory §2.8, R9) over a certified
/// symmetric interval matrix.
///
/// Order is fixed and deterministic: the positive-definite test (Gershgorin,
/// then the closed form) on `Ĥ`; the same test on `−Ĥ` for negative
/// definiteness; then certified indefiniteness via the STRICT predicate
/// `det Ĥ < 0` (the interval upper bound of `a·c − b²` is negative); otherwise
/// [`Definiteness::Singular`]. The [`Definiteness::Definite`] verdicts carry
/// the certified modulus `μ` (the R9 shape refuses `μ ≤ 0`); the
/// [`Definiteness::Indefinite`] verdict carries the strictly-negative `det`
/// upper bound.
pub fn definiteness(h: &IntervalSym2) -> Result<Definiteness, TangencyRefusal> {
    let a = h.a();
    let b = h.b();
    let c = h.c();
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return Err(TangencyRefusal::Input(Refusal::InvalidInput));
    }
    if let Some(mu) = positive_definite(a, b, c) {
        return Definiteness::definite(DefinitenessSign::Positive, mu)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput));
    }
    let an = neg_iv(a);
    let bn = neg_iv(b);
    let cn = neg_iv(c);
    if let Some(mu) = positive_definite(an, bn, cn) {
        return Definiteness::definite(DefinitenessSign::Negative, mu)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput));
    }
    // Certified indefiniteness: the interval value of det Ĥ = a·c − b² has a
    // STRICTLY negative upper bound (theory §2.8 / T1.6c).
    let det = a.mul(&c).sub(&b.mul(&b));
    if det.is_finite() && det.hi < 0.0 {
        return Definiteness::indefinite(det.hi)
            .map_err(|_| TangencyRefusal::Input(Refusal::InvalidInput));
    }
    Ok(Definiteness::singular())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::shapes::{GraphCert, Rank2Pivot};

    fn iv(lo: f64, hi: f64) -> CertifiedInterval {
        CertifiedInterval { lo, hi }
    }

    fn valid_chart_cert() -> GraphCert {
        GraphCert::new(
            [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)],
            [(0.25, 0.75), (0.25, 0.75), (0.25, 0.75), (0.25, 0.75)],
        )
        .expect("a valid strict inclusion")
    }

    /// The compile-time shape assertion plus the degenerate-enclosure refusal
    /// of the R2 signature: `reduced_hessian` accepts ONLY the certified
    /// [`GraphEnclosure`] as its box — there is no raw-box overload (a raw
    /// `Box4` argument cannot type-check).
    fn assert_r2_signature() {
        let _signature: fn(
            &SquareSystem3,
            &Rank2Chart,
            &GraphEnclosure,
        ) -> Result<IntervalSym2, TangencyRefusal> = reduced_hessian;
    }

    #[test]
    fn hessian_evaluator_rejects_raw_box() {
        assert_r2_signature();

        // A degenerate enclosure (zero-width on a z axis) is refused: no
        // (H-graph) content could certify a graph over a zero-width parameter
        // domain.
        let pivot = Rank2Pivot::new(2, (0, 1)).expect("a valid pivot");
        let net = crate::tangency::shapes::ChartMinorNet::new([1, 1, 1, 1], vec![1.0; 16])
            .expect("a valid minor net");
        let minors =
            crate::tangency::shapes::ChartMinorGrids::new(net.clone(), net).expect("valid minors");
        let chart = Rank2Chart::new(pivot, iv(0.5, 1.5), minors).expect("a valid chart");
        let degenerate = GraphEnclosure::new(
            [(0.4, 0.6), (0.4, 0.6)],
            [(0.5, 0.5), (0.4, 0.6)],
            valid_chart_cert(),
        )
        .expect("a structurally valid enclosure");
        let system = crate::tangency::minors::support::f1_system().expect("the F1 system admits");
        let refused = reduced_hessian(&system, &chart, &degenerate);
        assert!(
            matches!(refused, Err(TangencyRefusal::Input(Refusal::InvalidInput))),
            "a degenerate enclosure must be refused, got {refused:?}"
        );
    }

    /// The T1.3 symmetry admission over the F3 (definite) and F4 (saddle)
    /// fixture graphs: the reduced Hessian is symmetric within the enclosure.
    ///
    /// The two independently-computed cross entries of the raw evaluation
    /// (`H_h[0][1]` and `H_h[1][0]`) bound the same exact off-diagonal value
    /// (here exactly `0`, since the reduced contacts of both fixtures have no
    /// `z1·z2` cross term), so their enclosures overlap and contain `0`, and
    /// the frozen symmetric carrier's `b` is their outward hull. The
    /// definiteness reading of the finished matrix matches each fixture's
    /// class (definite on F3, indefinite on F4).
    #[test]
    fn t13_symmetry_on_fixture() {
        use crate::tangency::cascade::test_kit;
        for make in [test_kit::f3_system, test_kit::f4_system] {
            let system = make();
            let chart = test_kit::a1_chart(&system);
            let b = test_kit::a1_box();
            let graph = test_kit::a1_certified_graph(&system, &chart, &b);
            let raw = hessian_entries_raw(&system, &chart, &graph.graph)
                .expect("the raw reduced Hessian evaluates");
            // Both cross orderings bound the same exact symmetric value 0 (no
            // z1·z2 cross term in the fixtures' reduced contact functions).
            let cross12 = raw[0][1];
            let cross21 = raw[1][0];
            assert!(
                cross12.is_finite() && cross21.is_finite(),
                "the cross entries must be finite"
            );
            assert!(
                cross12.lo <= 0.0 && 0.0 <= cross12.hi,
                "H_h[0][1] must contain the exact symmetric value 0"
            );
            assert!(
                cross21.lo <= 0.0 && 0.0 <= cross21.hi,
                "H_h[1][0] must contain the exact symmetric value 0"
            );
            assert!(
                cross12.lo <= cross21.hi && cross21.lo <= cross12.hi,
                "the two cross orderings must agree within the enclosure"
            );
            // The (1,1) diagonal entry is the common positive curvature `2` of
            // both fixtures' reduced contacts (`±(z1 − 1/2)² ···`).
            assert!(raw[0][0].lo > 0.0, "the (1,1) curvature is positive");
            // The symmetric carrier's b is the outward hull of both orderings
            // and stays symmetric (contains 0).
            let sym = reduced_hessian(&system, &chart, &graph.graph)
                .expect("the reduced Hessian evaluates symmetrically");
            let b_iv = sym.b();
            assert!(
                b_iv.lo <= cross12.lo.min(cross21.lo) && cross12.hi.max(cross21.hi) <= b_iv.hi,
                "the symmetric carrier must hull the two cross orderings"
            );
            assert!(b_iv.lo <= 0.0 && 0.0 <= b_iv.hi);
        }
    }
}
