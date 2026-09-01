//! Homogeneous rational Bézier span substrate (GEN-001B).
//!
//! The certified machinery behind [`super::span::CurveSpan2::RationalBezier`].
//! Every operation is `pub(crate)`: the arrangement interface consumes
//! [`super::span::CurveSpan2`]'s solver-agnostic methods and the contact records
//! in [`super::contact`], never this module's Bernstein internals. The solver
//! strategy (Bernstein basis, de Casteljau) is an implementation detail that the
//! generic root-isolation layer (GEN-001C) reaches privately, and that a future
//! adaptive-precision or NURBS-decomposition substrate could replace without
//! changing the ARR-003-facing data model.
//!
//! # Representation
//!
//! `C(u) = [X(u) : Y(u) : W(u)]` for `u in [0, 1]`, each of `X`, `Y`, `W` a
//! Bernstein polynomial of degree `n`. The authoritative source parameter
//! domain `[t0, t1]` maps to `[0, 1]` affinely: `u = (t - t0) / (t1 - t0)`.
//!
//! # The weight certificate
//!
//! Construction certifies `W(u) > 0` on `[0, 1]`: the Bernstein weights are all
//! strictly one sign, normalized to positive (a global sign flip `[X:Y:W] ~
//! [-X:-Y:-W]` is projectively exact). By the convex-hull property `W` is then a
//! positive combination of positive weights on the whole span, so
//! dehomogenization has no pole. Mixed-sign or zero weights are refused
//! ([`BezierSpanError::WeightMayVanish`]): the span is not a regular rational
//! curve, and a pole is a singular branch the typed-outcome machinery (not a
//! tolerance) must handle.
//!
//! # Certification
//!
//! Point enclosures use the directed-rounding
//! [`super::exact::CertifiedInterval`]: each Bernstein evaluation is a sum of
//! widened products, so the returned interval provably contains the exact
//! dehomogenized coordinate over the `f64` inputs. Coefficients originating as
//! finite `f64` dyadic values are exact inputs; no epsilon appears.

use super::curve2d::CurveOccurrenceProvenance;
use super::exact::CertifiedInterval;

/// Why a Bézier span could not be constructed as a certified regular rational
/// span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BezierSpanError {
    /// Fewer than two control points (no curve).
    TooFewControlPoints,
    /// A coordinate or weight was not finite.
    NonFinite,
    /// The authoritative domain has zero length.
    DegenerateDomain,
    /// The Bernstein weights are not all strictly the same sign, so `W` may
    /// vanish on `[0, 1]` and the span has a pole. Refused as a regular span;
    /// subdivide to isolate the pole or classify it `Unsupported`/`Unresolved`.
    WeightMayVanish,
}

impl BezierSpanError {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::TooFewControlPoints => "bezier_too_few_control_points",
            Self::NonFinite => "bezier_non_finite",
            Self::DegenerateDomain => "bezier_degenerate_domain",
            Self::WeightMayVanish => "bezier_weight_may_vanish",
        }
    }
}

/// A homogeneous rational Bézier span `C(u) = [X(u) : Y(u) : W(u)]` in the
/// Bernstein basis over `[0, 1]`, with its authoritative source domain.
///
/// Constructed only through [`RationalBezierSpan2::new`], which certifies the
/// weights. The Bernstein fields are `pub(crate)` so the solver strategy never
/// reaches the arrangement interface.
#[derive(Debug, Clone, PartialEq)]
pub struct RationalBezierSpan2 {
    /// Homogeneous Bernstein control points `(X_i, Y_i, W_i)`, normalized so
    /// every weight is strictly positive.
    pub(crate) control: Vec<(f64, f64, f64)>,
    /// The degree `n = control.len() - 1`.
    degree: usize,
    /// The authoritative source parameter domain `[t0, t1]`, traversal order.
    pub(crate) domain: (f64, f64),
    /// The source occurrence this span represents.
    pub(crate) provenance: CurveOccurrenceProvenance,
}

impl RationalBezierSpan2 {
    /// Construct a certified regular rational Bézier span.
    ///
    /// `control` are homogeneous Bernstein control points over the unit
    /// interval; `domain` is the authoritative source parameter interval that
    /// maps to `[0, 1]`. Certifies that every weight is finite and strictly one
    /// sign, normalizing to all-positive (a projectively exact global sign flip).
    /// Refuses mixed-sign or zero weights, which would let `W` vanish.
    pub(crate) fn new(
        control: Vec<(f64, f64, f64)>,
        domain: (f64, f64),
        provenance: CurveOccurrenceProvenance,
    ) -> Result<Self, BezierSpanError> {
        if control.len() < 2 {
            return Err(BezierSpanError::TooFewControlPoints);
        }
        for &(x, y, w) in &control {
            if !x.is_finite() || !y.is_finite() || !w.is_finite() {
                return Err(BezierSpanError::NonFinite);
            }
        }
        if !domain.0.is_finite() || !domain.1.is_finite() || domain.0 == domain.1 {
            return Err(BezierSpanError::DegenerateDomain);
        }
        let all_positive = control.iter().all(|&(_, _, w)| w > 0.0);
        let all_negative = control.iter().all(|&(_, _, w)| w < 0.0);
        if !all_positive && !all_negative {
            return Err(BezierSpanError::WeightMayVanish);
        }
        let control = if all_positive {
            control
        } else {
            control.iter().map(|&(x, y, w)| (-x, -y, -w)).collect()
        };
        let degree = control.len() - 1;
        Ok(Self {
            control,
            degree,
            domain,
            provenance,
        })
    }

    /// The Bernstein degree `n`.
    pub(crate) fn degree(&self) -> usize {
        self.degree
    }

    /// The authoritative source parameter domain `[t0, t1]`.
    pub(crate) fn domain(&self) -> (f64, f64) {
        self.domain
    }

    /// Map a source parameter `t in [t0, t1]` to the unit Bernstein parameter.
    pub(crate) fn domain_to_unit(&self, t: f64) -> f64 {
        (t - self.domain.0) / (self.domain.1 - self.domain.0)
    }

    /// The `X` Bernstein coefficients over `[0, 1]`. Input to the generic
    /// bivariate root isolation (GEN-001C).
    #[allow(dead_code)]
    pub(crate) fn coeffs_x(&self) -> Vec<f64> {
        self.control.iter().map(|p| p.0).collect()
    }
    /// The `Y` Bernstein coefficients over `[0, 1]`. Input to the generic
    /// bivariate root isolation (GEN-001C).
    #[allow(dead_code)]
    pub(crate) fn coeffs_y(&self) -> Vec<f64> {
        self.control.iter().map(|p| p.1).collect()
    }
    /// The `W` Bernstein coefficients over `[0, 1]` (all strictly positive).
    pub(crate) fn coeffs_w(&self) -> Vec<f64> {
        self.control.iter().map(|p| p.2).collect()
    }

    /// Certified: every Bernstein weight is strictly positive, so `W(u) > 0` on
    /// `[0, 1]` and dehomogenization is pole-free. Enforced at construction;
    /// preserved by subdivision.
    pub(crate) fn w_positive_on_unit(&self) -> bool {
        self.control.iter().all(|&(_, _, w)| w > 0.0)
    }

    /// Certified enclosures `(x(u), y(u))` of the dehomogenized curve at `u in
    /// [0, 1]`.
    ///
    /// `None` if `u` is outside `[0, 1]` or the enclosure is non-finite. The
    /// weight enclosure is positive by construction, so the division is sound.
    pub(crate) fn evaluate_enclosure(&self, u: f64) -> Option<[CertifiedInterval; 2]> {
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let u_iv = CertifiedInterval::point(u);
        let (x, y, w) = self.eval_homogeneous(&u_iv)?;
        let px = x.div(&w)?;
        let py = y.div(&w)?;
        Some([px, py])
    }

    /// Certified enclosures of the dehomogenized first derivative `(x'(u),
    /// y'(u))` at `u in [0, 1]`.
    ///
    /// `x'(u) = (X'(u)W(u) - X(u)W'(u)) / W(u)^2`, the derivative numerator
    /// `X'W - XW'` evaluated at `u` over the certified `W^2`. This is the
    /// order-1 jet primitive the germ classifier (GEN-001C) reads: a certified
    /// nonzero result is a `Regular` germ; a certified zero result asks for the
    /// next jet order.
    pub(crate) fn first_derivative_enclosure(&self, u: f64) -> Option<[CertifiedInterval; 2]> {
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let u_iv = CertifiedInterval::point(u);
        let one_minus_u = CertifiedInterval::point(1.0).sub(&u_iv);
        let (x, y, w) = self.eval_homogeneous(&u_iv)?;
        let xp = self.component_derivative_eval(&u_iv, &one_minus_u, 0)?;
        let yp = self.component_derivative_eval(&u_iv, &one_minus_u, 1)?;
        let wp = self.component_derivative_eval(&u_iv, &one_minus_u, 2)?;
        let w_sq = w.mul(&w);
        let num_x = xp.mul(&w).sub(&x.mul(&wp));
        let num_y = yp.mul(&w).sub(&y.mul(&wp));
        let dx = num_x.div(&w_sq)?;
        let dy = num_y.div(&w_sq)?;
        if dx.is_finite() && dy.is_finite() {
            Some([dx, dy])
        } else {
            None
        }
    }

    /// The same occurrence traversed the other way: control coefficients
    /// reversed, the ordered domain inverted, and the traversal provenance
    /// reversed.
    ///
    /// Mirrors [`super::curve2d::LineSegment2::reverse_occurrence`] and
    /// [`super::curve2d::DirectedCircularArc2::reverse_occurrence`]. Because
    /// [`super::curve2d::CurveOccurrenceProvenance::reversed`] preserves the
    /// edge-use and source-edge ids, the derived span identity is unchanged:
    /// reparameterizing the same occurrence keeps the event identity and
    /// reverses only the branch-incidence orientation. A different twin edge
    /// use is a distinct B-rep incidence with a distinct span id and is never
    /// conflated here.
    pub(crate) fn reverse_occurrence(&self) -> Self {
        let mut control = self.control.clone();
        control.reverse();
        Self {
            control,
            degree: self.degree,
            domain: (self.domain.1, self.domain.0),
            provenance: self.provenance.reversed(),
        }
    }

    /// de Casteljau subdivision at `u in (0, 1)` into the left span over
    /// `[0, u]` and the right span over `[u, 1]`.
    ///
    /// Performed in homogeneous control space; convex combinations preserve the
    /// all-positive weight certificate, so both sub-spans are regular. The
    /// source domain is split at `t0 + u * (t1 - t0)` and the provenance is
    /// inherited verbatim. Deterministic: the same `u` always yields the same
    /// pair, and concatenating the sub-spans reproduces the original curve (to
    /// `f64` precision).
    pub(crate) fn subdivide(&self, u: f64) -> (Self, Self) {
        let n = self.degree;
        let mut column = self.control.clone();
        let mut left = Vec::with_capacity(n + 1);
        let mut right = Vec::with_capacity(n + 1);
        left.push(column[0]);
        right.push(column[n]);
        for _ in 1..=n {
            let mut next = Vec::with_capacity(column.len() - 1);
            for j in 0..column.len() - 1 {
                let (x0, y0, w0) = column[j];
                let (x1, y1, w1) = column[j + 1];
                next.push((
                    (1.0 - u) * x0 + u * x1,
                    (1.0 - u) * y0 + u * y1,
                    (1.0 - u) * w0 + u * w1,
                ));
            }
            column = next;
            left.push(column[0]);
            right.push(column[column.len() - 1]);
        }
        // `right` was collected [P_n, last(level1), ..., C(u)]; reverse to
        // [C(u), ..., P_n].
        right.reverse();
        let mid = self.domain.0 + u * (self.domain.1 - self.domain.0);
        let left_span = Self {
            control: left,
            degree: n,
            domain: (self.domain.0, mid),
            provenance: self.provenance,
        };
        let right_span = Self {
            control: right,
            degree: n,
            domain: (mid, self.domain.1),
            provenance: self.provenance,
        };
        (left_span, right_span)
    }

    // -- private certified Bernstein machinery --------------------------------

    /// Certified enclosures of `(X(u), Y(u), W(u))` (homogeneous, undivided).
    fn eval_homogeneous(
        &self,
        u: &CertifiedInterval,
    ) -> Option<(CertifiedInterval, CertifiedInterval, CertifiedInterval)> {
        let one_minus_u = CertifiedInterval::point(1.0).sub(u);
        let n = self.degree;
        let mut x = CertifiedInterval::point(0.0);
        let mut y = CertifiedInterval::point(0.0);
        let mut w = CertifiedInterval::point(0.0);
        for i in 0..=n {
            let basis = bernstein_basis_interval(n, i, u, &one_minus_u);
            let (cx, cy, cw) = self.control[i];
            x = x.add(&basis.mul(&CertifiedInterval::point(cx)));
            y = y.add(&basis.mul(&CertifiedInterval::point(cy)));
            w = w.add(&basis.mul(&CertifiedInterval::point(cw)));
        }
        if x.is_finite() && y.is_finite() && w.is_finite() {
            Some((x, y, w))
        } else {
            None
        }
    }

    /// Certified enclosure of the derivative of one homogeneous component at
    /// `u`. The degree-`(n-1)` Bernstein derivative coefficients are
    /// `n * (c_{i+1} - c_i)`; this evaluates their Bernstein form at `u`.
    /// `comp` is 0 for `X`, 1 for `Y`, 2 for `W`.
    fn component_derivative_eval(
        &self,
        u: &CertifiedInterval,
        one_minus_u: &CertifiedInterval,
        comp: usize,
    ) -> Option<CertifiedInterval> {
        let n = self.degree;
        if n == 0 {
            return Some(CertifiedInterval::point(0.0));
        }
        let dn = n - 1;
        let mut acc = CertifiedInterval::point(0.0);
        for i in 0..=dn {
            let ci = component(self.control[i], comp);
            let ci1 = component(self.control[i + 1], comp);
            let d = (n as f64) * (ci1 - ci);
            let basis = bernstein_basis_interval(dn, i, u, one_minus_u);
            acc = acc.add(&basis.mul(&CertifiedInterval::point(d)));
        }
        if acc.is_finite() {
            Some(acc)
        } else {
            None
        }
    }
}

fn component(c: (f64, f64, f64), idx: usize) -> f64 {
    match idx {
        0 => c.0,
        1 => c.1,
        _ => c.2,
    }
}

/// `C(n, i) * u^i * (1-u)^(n-i)` as a directed-rounded certified interval.
fn bernstein_basis_interval(
    n: usize,
    i: usize,
    u: &CertifiedInterval,
    one_minus_u: &CertifiedInterval,
) -> CertifiedInterval {
    let mut b = CertifiedInterval::point(binomial(n, i) as f64);
    for _ in 0..i {
        b = b.mul(u);
    }
    for _ in 0..(n - i) {
        b = b.mul(one_minus_u);
    }
    b
}

/// Exact binomial coefficient `C(n, k)` for the small degrees this substrate
/// serves.
fn binomial(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut acc: u128 = 1;
    for i in 0..k {
        acc = acc * (n - i) as u128 / (i + 1) as u128;
    }
    acc as u64
}

#[cfg(test)]
mod tests {
    use super::super::curve2d::{SourceEdgeId, SourceEntityId, SourceFaceId};
    use super::*;
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

    /// The parabola `C(u) = (u, u^2)` as a degree-2 polynomial Bézier
    /// (`W = 1`): `X` coeffs `(0, 1/2, 1)`, `Y` coeffs `(0, 0, 1)`.
    fn parabola() -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (0.5, 0.0, 1.0), (1.0, 1.0, 1.0)],
            (0.0, 1.0),
            provenance(),
        )
        .unwrap()
    }

    /// A noncircular rational quadratic: middle weight `2`, not the circular
    /// weight. Endpoints `(0, 0)` and `(2, 0)`.
    fn rational_quadratic() -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (1.0, 1.0, 2.0), (2.0, 0.0, 1.0)],
            (0.0, 1.0),
            provenance(),
        )
        .unwrap()
    }

    fn encloses(iv: &CertifiedInterval, value: f64) -> bool {
        iv.lo <= value && value <= iv.hi
    }

    #[test]
    fn polynomial_quadratic_reaches_the_substrate() {
        let span = parabola();
        assert_eq!(span.degree(), 2);
        assert!(span.w_positive_on_unit());
        assert_eq!(span.coeffs_w(), vec![1.0, 1.0, 1.0]);
        // Endpoints are exact.
        let [px, py] = span.evaluate_enclosure(0.0).unwrap();
        assert!(encloses(&px, 0.0) && encloses(&py, 0.0));
        let [px, py] = span.evaluate_enclosure(1.0).unwrap();
        assert!(encloses(&px, 1.0) && encloses(&py, 1.0));
        // Midpoint of (u, u^2) at u = 1/2 is (1/2, 1/4), inside the enclosure.
        let [px, py] = span.evaluate_enclosure(0.5).unwrap();
        assert!(encloses(&px, 0.5));
        assert!(encloses(&py, 0.25));
        // u = 1/4 -> (1/4, 1/16).
        let [px, py] = span.evaluate_enclosure(0.25).unwrap();
        assert!(encloses(&px, 0.25));
        assert!(encloses(&py, 0.0625));
    }

    #[test]
    fn rational_quadratic_reaches_the_substrate() {
        let span = rational_quadratic();
        assert_eq!(span.degree(), 2);
        assert!(span.w_positive_on_unit());
        assert_eq!(span.coeffs_w(), vec![1.0, 2.0, 1.0]);
        let [px, py] = span.evaluate_enclosure(0.0).unwrap();
        assert!(encloses(&px, 0.0) && encloses(&py, 0.0));
        let [px, py] = span.evaluate_enclosure(1.0).unwrap();
        assert!(encloses(&px, 2.0) && encloses(&py, 0.0));
        // Dehomogenized midpoint by hand. At u = 1/2: B0 = B2 = 1/4, B1 = 1/2.
        // X(u) = 0*1/4 + 1*1/2 + 2*1/4 = 1.0; Y(u) = 0*1/4 + 1*1/2 + 0 = 1/2;
        // W(u) = 1*1/4 + 2*1/2 + 1*1/4 = 3/2. So x = X/W = 2/3, y = Y/W = 1/3.
        let [px, py] = span.evaluate_enclosure(0.5).unwrap();
        assert!(encloses(&px, 2.0 / 3.0));
        assert!(encloses(&py, 1.0 / 3.0));
    }

    #[test]
    fn first_derivative_of_the_parabola_matches_known_jet() {
        // (u, u^2) -> (1, 2u). At u = 1/2 that is (1, 1).
        let span = parabola();
        let [dx, dy] = span.first_derivative_enclosure(0.5).unwrap();
        assert!(encloses(&dx, 1.0));
        assert!(encloses(&dy, 1.0));
        // At u = 1/4, (1, 1/2).
        let [dx, dy] = span.first_derivative_enclosure(0.25).unwrap();
        assert!(encloses(&dx, 1.0));
        assert!(encloses(&dy, 0.5));
    }

    #[test]
    fn subdivision_preserves_the_curve_and_the_weight_certificate() {
        let span = parabola();
        let (left, right) = span.subdivide(0.5);
        assert_eq!(left.domain(), (0.0, 0.5));
        assert_eq!(right.domain(), (0.5, 1.0));
        assert!(left.w_positive_on_unit() && right.w_positive_on_unit());

        // The split point: left's end (u = 1 in the left span) and right's start
        // (u = 0) both enclose the original at u = 1/2, which is (1/2, 1/4).
        let [lx, ly] = left.evaluate_enclosure(1.0).unwrap();
        assert!(encloses(&lx, 0.5) && encloses(&ly, 0.25));
        let [rx, ry] = right.evaluate_enclosure(0.0).unwrap();
        assert!(encloses(&rx, 0.5) && encloses(&ry, 0.25));

        // A point in the left half: original at u = 1/4 is (1/4, 1/16); the left
        // span at its midpoint (u = 1/2 of [0, 1/2]) is the same point.
        let [lx, ly] = left.evaluate_enclosure(0.5).unwrap();
        assert!(encloses(&lx, 0.25) && encloses(&ly, 0.0625));
        // A point in the right half: original at u = 3/4 is (3/4, 9/16); the
        // right span at its midpoint (u = 1/2 of [1/2, 1]) is the same point.
        let [rx, ry] = right.evaluate_enclosure(0.5).unwrap();
        assert!(encloses(&rx, 0.75) && encloses(&ry, 0.5625));
    }

    #[test]
    fn weight_may_vanish_is_refused() {
        // Mixed-sign weights: a pole on the span.
        let err = RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (1.0, 1.0, -1.0), (2.0, 0.0, 1.0)],
            (0.0, 1.0),
            provenance(),
        )
        .unwrap_err();
        assert_eq!(err, BezierSpanError::WeightMayVanish);
        // A zero weight is also refused (not strictly one sign).
        let err = RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (1.0, 1.0, 0.0), (2.0, 0.0, 1.0)],
            (0.0, 1.0),
            provenance(),
        )
        .unwrap_err();
        assert_eq!(err, BezierSpanError::WeightMayVanish);
    }

    #[test]
    fn all_negative_weights_are_normalized_to_positive() {
        // Projectively equivalent to the parabola; construction normalizes.
        let span = RationalBezierSpan2::new(
            vec![(0.0, 0.0, -1.0), (-0.5, 0.0, -1.0), (-1.0, -1.0, -1.0)],
            (0.0, 1.0),
            provenance(),
        )
        .unwrap();
        assert!(span.w_positive_on_unit());
        // Same image as the parabola after the sign flip.
        let [px, py] = span.evaluate_enclosure(0.5).unwrap();
        assert!(encloses(&px, 0.5) && encloses(&py, 0.25));
    }

    #[test]
    fn degenerate_and_non_finite_inputs_are_refused() {
        assert_eq!(
            RationalBezierSpan2::new(vec![(0.0, 0.0, 1.0)], (0.0, 1.0), provenance()).unwrap_err(),
            BezierSpanError::TooFewControlPoints
        );
        assert_eq!(
            RationalBezierSpan2::new(
                vec![(0.0, 0.0, 1.0), (f64::NAN, 1.0, 1.0)],
                (0.0, 1.0),
                provenance()
            )
            .unwrap_err(),
            BezierSpanError::NonFinite
        );
        assert_eq!(
            RationalBezierSpan2::new(
                vec![(0.0, 0.0, 1.0), (1.0, 1.0, 1.0)],
                (1.0, 1.0),
                provenance()
            )
            .unwrap_err(),
            BezierSpanError::DegenerateDomain
        );
    }

    #[test]
    fn reverse_occurrence_keeps_geometry_and_identity() {
        let span = parabola();
        let rev = span.reverse_occurrence();
        assert_eq!(rev.domain(), (1.0, 0.0));
        assert!(rev.w_positive_on_unit());
        // Reversing twice restores the original coefficients and domain.
        let back = rev.reverse_occurrence();
        assert_eq!(back.domain(), span.domain());
        assert_eq!(back.coeffs_x(), span.coeffs_x());
        assert_eq!(back.coeffs_y(), span.coeffs_y());
        // The geometry is unchanged: the reversed span at local u = 0 is the
        // original at local u = 1.
        let [lx, ly] = rev.evaluate_enclosure(0.0).unwrap();
        assert!(encloses(&lx, 1.0) && encloses(&ly, 1.0));
        let [rx, ry] = rev.evaluate_enclosure(1.0).unwrap();
        assert!(encloses(&rx, 0.0) && encloses(&ry, 0.0));
        // Reparameterization reversal preserves the occurrence identity: the
        // provenance reversal swaps only the traversal vertices.
        let span_id = super::super::span::SpanId::from_occurrence(&span.provenance);
        let rev_id = super::super::span::SpanId::from_occurrence(&rev.provenance);
        assert_eq!(span_id, rev_id);
        assert_eq!(
            rev.provenance.start_vertex_id,
            span.provenance.end_vertex_id
        );
    }

    #[test]
    fn source_domain_maps_to_the_unit_interval() {
        let span = RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (0.5, 0.0, 1.0), (1.0, 1.0, 1.0)],
            (2.0, 4.0),
            provenance(),
        )
        .unwrap();
        assert_eq!(span.domain_to_unit(2.0), 0.0);
        assert_eq!(span.domain_to_unit(4.0), 1.0);
        assert_eq!(span.domain_to_unit(3.0), 0.5);
    }
}
