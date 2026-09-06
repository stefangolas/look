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

//! The (H-graph) parametric Newton/Krawczyk certification (CTE-002-GRAPH,
//! part two).
//!
//! Under hypothesis (H-graph) (theory §2.2), on `B = Y x Z` with `0 ∉ det A(B)`
//! for the recorded chart, a parametric interval Newton/Krawczyk certifies that
//! for EVERY `z ∈ Z` there is a UNIQUE `y = φ(z) ∈ Y` with `G(φ(z), z) = 0`,
//! together with the graph enclosure `Ŷ ⊇ φ(Z)`, `Ŷ ⊆ Y`.
//!
//! # The parametric operator
//!
//! The operator follows the landed point-box discipline ([`KrawczykSystem`]'s
//! parameterized rule) with `z` as the parameter. Over a parameter piece
//! `Z_i ⊆ Z` (all pieces share the same `Y`):
//!
//! ```text
//! K = m_y − C · G(m_y, Z_i) + (I − C · D_yG(Y, Z_i)) · (Y − m_y)
//! ```
//!
//! with `m_y` the float midpoint of `Y`, the residual `G(m_y, Z_i)` evaluated
//! as an INTERVAL over the parameter piece (the point-center decorrelation rule
//! applied in `y`, parameter range applied in `z` — a point residual at `z_mid`
//! alone could not enclose `φ(Z_i)`, so the interval-over-`Z_i` residual is the
//! certified form), `D_yG(Y, Z_i)` the interval Jacobian over the piece box, and
//! `C` the float inverse of `D_yG` at the midpoint pair. A component-wise
//! STRICT inclusion `K ⊂ int(Y)` certifies the piece; the certified image then
//! contains `φ(Z_i)` (RESULT note: the certified residual is the interval over
//! the parameter piece — the point-`z_mid` reading of the landed doc cannot
//! enclose the graph over the whole piece, which the probe-grid regression
//! exists to pin).
//!
//! When a piece does not certify, the box is bisected on the widest `z` axis
//! (ties toward the lowest chart-axis index, low half processed before high —
//! the landed operator's discipline, scope decision 6) under a subdivision
//! budget; the graph enclosure `Ŷ` is the outward hull of the certified piece
//! images, clamped into `Y` (scope decision 3). A degenerate piece that still
//! fails, or budget exhaustion, refuses [`TangencyRefusal::GraphFailure`] —
//! never an infinite split.
//!
//! The returned [`CertifiedGraph`] carries the recorded chart and the certified
//! [`GraphEnclosure`]; the enclosure's [`GraphCert`] evidence records the
//! four-axis strict inclusion of the certified graph restricted to a
//! strictly-interior parameter sub-box (RESULT note: the whole-`Z` graph fills
//! the `z`-domain, so no four-axis box strictly inside `B` can contain it; the
//! strict-inclusion evidence records the certified interior restriction, while
//! [`GraphEnclosure::y_hat`] carries the full-`Z` coverage).
//!
//! Every numeric refusal wraps a landed [`SsiRefusal`] (zero new top-level
//! evidence kinds).

use crate::formal::exact::CertifiedInterval;
use crate::ssi::SsiRefusal;
use crate::ssi_types::SquareSystem3;
use crate::tangency::chart::{
    box_in_chart, certified_value, float_partial, g_rows_of, pivot_det, z_axes_of, Box4,
    TangencyRefusal,
};
use crate::tangency::shapes::{GraphCert, GraphEnclosure, Rank2Chart};

/// The subdivision budget of one [`certify_graph`] call: the maximum number of
/// `z`-axis bisections a failing parametric inclusion may spend before refusing
/// [`TangencyRefusal::GraphFailure`].
///
/// This is an algorithmic budget bound (scope decision 6's "under Budget"), not
/// a numeric predicate tolerance.
const GRAPH_SUBDIVISION_BUDGET: u32 = 64;

/// The certified (H-graph) content over one box (theory §2.2).
///
/// `chart` is the recorded [`Rank2Chart`] (pivot + `det A` enclosure with `0`
/// excluded over the box); `graph` is the certified graph enclosure
/// `Ŷ ⊇ φ(Z)`, `Ŷ ⊆ Y` produced by [`certify_graph`].
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedGraph {
    /// The recorded chart over the box.
    pub chart: Rank2Chart,
    /// The certified graph enclosure `Ŷ ⊇ φ(Z)`, `Ŷ ⊆ Y`.
    pub graph: GraphEnclosure,
}

impl CertifiedGraph {
    /// Assemble a certified graph from its chart and enclosure.
    pub fn new(chart: Rank2Chart, graph: GraphEnclosure) -> Self {
        Self { chart, graph }
    }
}

/// Whether one parametric inclusion step certified a piece.
enum StepOutcome {
    /// The piece certified; the image is component-wise strictly inside `Y`.
    Certified([CertifiedInterval; 2]),
    /// The piece did not certify; bisect the `z`-domain if the budget allows.
    NotCertified,
}

/// Certify the (H-graph) content of the chart over the box (theory §2.2).
///
/// Preconditions (each a named refusal, never a retry on the same box):
/// - `b` must lie inside the stored system's chart rectangle;
/// - both `z`-axes of `b` (the two axes complementary to the chart's `y` pair)
///   must have positive width — a graph over a zero-width parameter domain is
///   not an (H-graph) request ([`SsiRefusal::InvalidInput`] wrapped);
/// - the chart's certified `det A(B)` must strictly exclude `0` over `b`
///   ([`SsiRefusal::DeterminantSpansZero`] wrapped).
///
/// The parametric operator then certifies each parameter piece under the
/// subdivision budget ([`TangencyRefusal::GraphFailure`] on exhaustion or on a
/// degenerate piece that still fails). The returned graph enclosure's `y_hat`
/// is the outward-rounded hull of the certified piece images clamped into `Y`.
pub fn certify_graph(
    system: &SquareSystem3,
    chart: &Rank2Chart,
    b: Box4,
) -> Result<CertifiedGraph, TangencyRefusal> {
    if !box_in_chart(system, b) {
        return Err(SsiRefusal::Hull(crate::hull::HullRefusal::DomainNotCompact).into());
    }
    let pivot = chart.pivot();
    let coord_pair = pivot.coord_pair();
    let y_axes = [coord_pair.0, coord_pair.1];
    let z_axes = z_axes_of(coord_pair);
    if b[z_axes[0]].1 - b[z_axes[0]].0 <= 0.0 || b[z_axes[1]].1 - b[z_axes[1]].0 <= 0.0 {
        return Err(SsiRefusal::InvalidInput.into());
    }

    // Hypothesis (H-graph), first clause: 0 strictly outside det A(B).
    let det = pivot_det(system, b, pivot)?;
    if !det.is_finite() || !(det.lo > 0.0 || det.hi < 0.0) {
        return Err(SsiRefusal::DeterminantSpansZero.into());
    }

    let rows = g_rows_of(pivot.component());
    let y = [b[y_axes[0]], b[y_axes[1]]];

    // Deterministic worklist over z-parameter pieces, low half before high.
    let mut stack: Vec<Box4> = vec![b];
    let mut budget: u32 = GRAPH_SUBDIVISION_BUDGET;
    let mut piece_images: Vec<[CertifiedInterval; 2]> = Vec::new();
    while let Some(box_) = stack.pop() {
        match parametric_step(system, rows, y_axes, z_axes, y, box_)? {
            StepOutcome::Certified(image) => piece_images.push(image),
            StepOutcome::NotCertified => {
                if budget == 0 {
                    return Err(TangencyRefusal::GraphFailure);
                }
                match split_z(box_, z_axes) {
                    Some((low, high)) => {
                        budget -= 1;
                        stack.push(high);
                        stack.push(low);
                    }
                    None => {
                        // A degenerate piece that still fails refuses; never an
                        // infinite split.
                        return Err(TangencyRefusal::GraphFailure);
                    }
                }
            }
        }
    }

    // The graph enclosure: outward hull of the certified images, clamped into Y.
    let mut y_hat_lo = [f64::INFINITY; 2];
    let mut y_hat_hi = [f64::NEG_INFINITY; 2];
    for image in &piece_images {
        for (axis, iv) in image.iter().enumerate() {
            if !iv.is_finite() {
                return Err(TangencyRefusal::GraphFailure);
            }
            y_hat_lo[axis] = y_hat_lo[axis].min(iv.lo);
            y_hat_hi[axis] = y_hat_hi[axis].max(iv.hi);
        }
    }
    let mut y_hat = [(0.0f64, 0.0f64); 2];
    for axis in 0..2 {
        // Clamp into Y (decision 3). The certified images are already strictly
        // inside Y, so this is an identity clamp on the success path.
        let clamped_lo = y_hat_lo[axis].max(y[axis].0);
        let clamped_hi = y_hat_hi[axis].min(y[axis].1);
        if !(y[axis].0 < clamped_lo && clamped_hi < y[axis].1) {
            // The enclosure is not strictly inside Y: the (H-graph) hypothesis
            // fails for this box — named refusal, never a retry on the same box.
            return Err(TangencyRefusal::GraphFailure);
        }
        y_hat[axis] = (clamped_lo, clamped_hi);
    }

    let graph = GraphEnclosure::new(
        y_hat,
        [b[z_axes[0]], b[z_axes[1]]],
        build_evidence(b, y_axes, z_axes, y_hat)?,
    )
    .map_err(|_| TangencyRefusal::Ssi(SsiRefusal::InvalidInput))?;
    Ok(CertifiedGraph::new(chart.clone(), graph))
}

/// One parametric Newton/Krawczyk inclusion step over a piece box.
///
/// The piece box carries the full `Y` on the two `y` axes and the piece's
/// `z`-domain on the two `z` axes. Returns the certified image when the strict
/// inclusion `K ⊂ int(Y)` holds, [`StepOutcome::NotCertified`] otherwise.
fn parametric_step(
    system: &SquareSystem3,
    rows: [usize; 2],
    y_axes: [usize; 2],
    z_axes: [usize; 2],
    y: [(f64, f64); 2],
    box_: Box4,
) -> Result<StepOutcome, TangencyRefusal> {
    // Float midpoints of the piece's Y and Z domains.
    let mut m_y = [0.0f64; 2];
    let mut m_z = [0.0f64; 2];
    for (axis, &(lo, hi)) in y.iter().enumerate() {
        m_y[axis] = 0.5 * (lo + hi);
        if !(lo <= m_y[axis] && m_y[axis] <= hi) {
            return Ok(StepOutcome::NotCertified);
        }
    }
    for (axis, chart_axis) in z_axes.iter().enumerate() {
        let (lo, hi) = box_[*chart_axis];
        m_z[axis] = 0.5 * (lo + hi);
        if !(lo <= m_z[axis] && m_z[axis] <= hi) {
            return Ok(StepOutcome::NotCertified);
        }
    }

    // Interval Jacobian D_yG over the piece box.
    let mut j = [[CertifiedInterval::point(0.0); 2]; 2];
    for (k, row) in j.iter_mut().enumerate() {
        for (col, cell) in row.iter_mut().enumerate() {
            *cell = crate::ssi::partial_enclosure(system, rows[k], y_axes[col], box_)?;
            if !cell.is_finite() {
                return Err(
                    SsiRefusal::Hull(crate::hull::HullRefusal::EnclosureUnavailable).into(),
                );
            }
        }
    }

    // Residual G(m_y, Z_i): certified interval over the piece with the y axes
    // pinned at the y midpoint.
    let mut res = [CertifiedInterval::point(0.0); 2];
    for (k, cell) in res.iter_mut().enumerate() {
        let mut res_box = box_;
        res_box[y_axes[0]] = (m_y[0], m_y[0]);
        res_box[y_axes[1]] = (m_y[1], m_y[1]);
        *cell = certified_value(system, rows[k], res_box)?;
        if !cell.is_finite() {
            return Err(SsiRefusal::Hull(crate::hull::HullRefusal::EnclosureUnavailable).into());
        }
    }

    // Float approximate inverse of D_yG at the midpoint pair.
    let mut center = [0.0f64; 4];
    center[y_axes[0]] = m_y[0];
    center[y_axes[1]] = m_y[1];
    center[z_axes[0]] = m_z[0];
    center[z_axes[1]] = m_z[1];
    let mut m = [[0.0f64; 2]; 2];
    for k in 0..2 {
        for col in 0..2 {
            match float_partial(system, rows[k], y_axes[col], center) {
                Some(v) => m[k][col] = v,
                None => return Ok(StepOutcome::NotCertified),
            }
        }
    }
    let det_m = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if det_m == 0.0 {
        return Ok(StepOutcome::NotCertified);
    }
    let inv = [
        [m[1][1] / det_m, -m[0][1] / det_m],
        [-m[1][0] / det_m, m[0][0] / det_m],
    ];

    // K = m_y − C·res + (I − C·J)·(Y − m_y), all directed rounding.
    let mut c_res = [CertifiedInterval::point(0.0); 2];
    for (r, cell) in c_res.iter_mut().enumerate() {
        let mut acc = CertifiedInterval::point(0.0);
        for k in 0..2 {
            acc = acc.add(&CertifiedInterval::point(inv[r][k]).mul(&res[k]));
        }
        *cell = acc;
    }
    let mut dy = [CertifiedInterval::point(0.0); 2];
    for (axis, &(lo, hi)) in y.iter().enumerate() {
        let d_lo = CertifiedInterval::point(lo).sub(&CertifiedInterval::point(m_y[axis]));
        let d_hi = CertifiedInterval::point(hi).sub(&CertifiedInterval::point(m_y[axis]));
        dy[axis] = CertifiedInterval {
            lo: d_lo.lo.min(d_hi.lo),
            hi: d_lo.hi.max(d_hi.hi),
        };
    }
    let mut correction = [CertifiedInterval::point(0.0); 2];
    for (r, cell) in correction.iter_mut().enumerate() {
        let mut acc = CertifiedInterval::point(0.0);
        for c in 0..2 {
            let delta = if r == c {
                CertifiedInterval::point(1.0)
            } else {
                CertifiedInterval::point(0.0)
            };
            let mut cj = CertifiedInterval::point(0.0);
            for k in 0..2 {
                cj = cj.add(&CertifiedInterval::point(inv[r][k]).mul(&j[k][c]));
            }
            acc = acc.add(&delta.sub(&cj).mul(&dy[c]));
        }
        *cell = acc;
    }

    let mut k_image = [CertifiedInterval::point(0.0); 2];
    for axis in 0..2 {
        k_image[axis] = CertifiedInterval::point(m_y[axis])
            .sub(&c_res[axis])
            .add(&correction[axis]);
    }

    // STRICT inclusion on both y axes.
    for (axis, kv) in k_image.iter().enumerate() {
        if !kv.is_finite() {
            return Ok(StepOutcome::NotCertified);
        }
        if !(y[axis].0 < kv.lo && kv.hi < y[axis].1) {
            return Ok(StepOutcome::NotCertified);
        }
    }
    Ok(StepOutcome::Certified(k_image))
}

/// Split a failing piece on its widest `z` axis (ties toward the lowest
/// chart-axis index), returning the low and high halves. `None` when the widest
/// `z` axis cannot be split in `f64` (the piece is at resolution).
fn split_z(box_: Box4, z_axes: [usize; 2]) -> Option<(Box4, Box4)> {
    let mut axis_slot = 0usize;
    let mut width = f64::NEG_INFINITY;
    let mut a = 0.0;
    let mut b = 0.0;
    for (slot, chart_axis) in z_axes.iter().enumerate() {
        let (lo, hi) = box_[*chart_axis];
        let w = hi - lo;
        // Ties toward the lower chart-axis index; the z axes are listed in
        // ascending chart-axis order, so slot order is already the tie order.
        if w.total_cmp(&width).is_gt() {
            axis_slot = slot;
            width = w;
            a = lo;
            b = hi;
        }
    }
    if width <= 0.0 {
        return None;
    }
    let mid = 0.5 * a + 0.5 * b;
    if mid == a || mid == b {
        return None;
    }
    let mut low = box_;
    let mut high = box_;
    let chart_axis = z_axes[axis_slot];
    low[chart_axis] = (a, mid);
    high[chart_axis] = (mid, b);
    Some((low, high))
}

/// Build the four-axis strict-inclusion evidence for the certified graph.
///
/// The parametric step certifies the whole-`Z` graph; the [`GraphCert`] shape
/// demands a four-axis image component-wise strictly inside the working box.
/// The certified content expressible that way is the graph restricted to a
/// strictly-interior parameter sub-box `Z_int ⊂ Z`: over `Z_int` the certified
/// graph is contained in `y_hat` (a restriction of a certified graph is
/// certified), so the image `(y_hat on the y axes) x (Z_int on the z axes)` is a
/// truthful four-axis strict inclusion. `Z_int` is the central half of each
/// `z` axis.
fn build_evidence(
    b: Box4,
    y_axes: [usize; 2],
    z_axes: [usize; 2],
    y_hat: [(f64, f64); 2],
) -> Result<GraphCert, TangencyRefusal> {
    let mut image = b;
    for (axis, chart_axis) in y_axes.iter().enumerate() {
        image[*chart_axis] = y_hat[axis];
    }
    for chart_axis in z_axes.iter() {
        let (lo, hi) = b[*chart_axis];
        let quarter = (hi - lo) * 0.25;
        image[*chart_axis] = (lo + quarter, hi - quarter);
    }
    GraphCert::new(b, image).map_err(|_| TangencyRefusal::Ssi(SsiRefusal::InvalidInput))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tangency::chart::test_kit as kit;
    use crate::tangency::chart::{find_chart, g_rows_of, z_axes_of};
    use crate::tangency::fixtures::F3PlaneSphereFixture;

    /// The certified (H-graph) inclusion holds on the F3 (A₁⁺) box: the
    /// parametric Newton/Krawczyk certifies a unique `y = φ(z)` for every
    /// `z ∈ Z`, at a workable subdivision depth (one-shot here), and the graph
    /// enclosure `Ŷ` is strictly inside `Y`.
    #[test]
    fn parametric_newton_certifies_graph_on_f3() {
        F3PlaneSphereFixture::new()
            .admit()
            .expect("the F3 ground truth admits");
        let (system, box_) = kit::f3_plane_bowl();
        let chart = find_chart(&system, box_).expect("the F3 box admits a chart");
        let certified = certify_graph(&system, &chart, box_)
            .expect("the parametric Newton certifies the F3 graph");

        let pivot = certified.chart.pivot();
        let coord_pair = pivot.coord_pair();
        let y_axes = [coord_pair.0, coord_pair.1];
        let z_axes = z_axes_of(coord_pair);

        // The strict inclusion content: Ŷ strictly inside Y on both y axes.
        let y_hat = certified.graph.y_hat();
        for (axis, chart_axis) in y_axes.iter().enumerate() {
            let (y_lo, y_hi) = box_[*chart_axis];
            assert!(
                y_lo < y_hat[axis].0 && y_hat[axis].1 < y_hi,
                "Ŷ must be strictly inside Y on y-axis {axis}"
            );
        }
        // The z-domain of the enclosure is the box's Z.
        let z = certified.graph.z();
        assert_eq!(z, [box_[z_axes[0]], box_[z_axes[1]]]);

        // The evidence carries a strict four-axis inclusion.
        let evidence = certified.graph.evidence();
        for (work, image) in evidence.work_box().iter().zip(evidence.image().iter()) {
            assert!(
                work.0 < image.0 && image.1 < work.1,
                "the evidence image is strictly inside the working box"
            );
        }
    }

    /// The regression net: for a dyadic probe grid of `z` values across the F3
    /// box's `Z`, the plain-float Newton solution `φ(z)` of `G(·, z) = 0` lies
    /// inside the certified graph enclosure `Ŷ` for every probe (H-3
    /// tolerance). The certificate itself is the Krawczyk strict inclusion; this
    /// test is the consistency net that pins the parametric residual form.
    #[test]
    fn graph_enclosure_contains_phi_on_probe_grid() {
        let (system, box_) = kit::f3_plane_bowl();
        let chart = find_chart(&system, box_).expect("the F3 box admits a chart");
        let certified = certify_graph(&system, &chart, box_)
            .expect("the parametric Newton certifies the F3 graph");

        let pivot = certified.chart.pivot();
        let coord_pair = pivot.coord_pair();
        let y_axes = [coord_pair.0, coord_pair.1];
        let z_axes = z_axes_of(coord_pair);
        let y_hat = certified.graph.y_hat();
        let z = certified.graph.z();

        // H-3: a dimensionless membership slack for the float-Newton regression
        // net, far above the certified enclosure's directed-rounding width.
        const PROBE_TOL: f64 = 1e-9; // H-3

        let grid = |lo: f64, hi: f64| -> Vec<f64> {
            let mut out = Vec::new();
            for k in 0..=4 {
                out.push(lo + (hi - lo) * (k as f64) / 4.0);
            }
            out
        };
        let z0s = grid(z[0].0, z[0].1);
        let z1s = grid(z[1].0, z[1].1);
        let mut probes = 0usize;
        for &z0 in &z0s {
            for &z1 in &z1s {
                let mut point = [0.5f64; 4];
                point[z_axes[0]] = z0;
                point[z_axes[1]] = z1;
                // Start the float Newton at the y-domain centre.
                for axis in 0..2 {
                    point[y_axes[axis]] = 0.5 * (box_[y_axes[axis]].0 + box_[y_axes[axis]].1);
                }
                let phi = float_newton_phi(&system, pivot, y_axes, point);
                for axis in 0..2 {
                    let p = phi[y_axes[axis]];
                    assert!(
                        y_hat[axis].0 - PROBE_TOL <= p && p <= y_hat[axis].1 + PROBE_TOL,
                        "probe z = ({z0}, {z1}): φ on y-axis {axis} = {p} escaped Ŷ \
                         = [{}, {}]",
                        y_hat[axis].0,
                        y_hat[axis].1
                    );
                }
                probes += 1;
            }
        }
        assert_eq!(probes, 25);
    }

    /// Plain-float Newton solve of `G(y, z) = 0` in the two `y` coordinates for
    /// a fixed `z`, seeded from the given point's `y` centre. Test support only.
    fn float_newton_phi(
        system: &SquareSystem3,
        pivot: crate::tangency::shapes::Rank2Pivot,
        y_axes: [usize; 2],
        mut point: [f64; 4],
    ) -> [f64; 4] {
        let rows = g_rows_of(pivot.component());
        let degrees = system.degrees();
        let mut converged = false;
        let newton_eps = 1e-12; // H-3: float Newton convergence threshold
        for _ in 0..24 {
            let f =
                crate::ssi_fixtures::eval_system(system, (point[0], point[1], point[2], point[3]))
                    .expect("the system evaluates at an interior point");
            let g = [f[rows[0]], f[rows[1]]];
            if g[0].abs() < newton_eps && g[1].abs() < newton_eps {
                converged = true;
                break;
            }
            let mut m = [[0.0f64; 2]; 2];
            for k in 0..2 {
                for col in 0..2 {
                    m[k][col] = crate::ssi_fixtures::partial_grid4_axis(
                        &system.grids()[rows[k]],
                        degrees,
                        y_axes[col],
                        (point[0], point[1], point[2], point[3]),
                    )
                    .expect("a float partial at an interior point");
                }
            }
            let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
            assert!(det != 0.0, "the float Newton Jacobian is singular");
            let dy0 = (m[1][1] * g[0] - m[0][1] * g[1]) / det;
            let dy1 = (-m[1][0] * g[0] + m[0][0] * g[1]) / det;
            point[y_axes[0]] -= dy0;
            point[y_axes[1]] -= dy1;
        }
        assert!(converged, "the float Newton solve did not converge");
        point
    }
}
