//! Spline-carrier admission (CL-000-SPLINE-ADMIT): the bridge from a landed
//! `BSplineSurface` spline carrier to the certified rational tensor-Bernstein
//! engine (`ssi.rs`).
//!
//! # Pre-made decisions (packet tags; do not relitigate)
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module: no `unwrap`, no `expect`, no `panic!`, and no module-level
//! `allow`. Options unpack through `ok_or(...)?` into named refusals.
//!
//! **D1 — patch decomposition is exact knot manipulation.** Bézier patches are
//! extracted by inserting interior knots to full multiplicity `degree + 1` on
//! both knot axes (the landed `KnotVec` insertion ops), never by new spline
//! math. The refined control net then partitions into contiguous
//! `(degree_u + 1) x (degree_v + 1)` Bézier cells.
//!
//! **D2 — the carrier is the non-rational spline net.** The corpus lofts are
//! non-rational: every homogeneous control weight is within tolerance of 1.
//! Such a surface is admitted carrying weight exactly 1 (numerator = the
//! dehomogenized control point). A rational input with a weight that deviates
//! from 1 beyond tolerance refuses typed, reusing the landed
//! `NonPositiveNurbsWeight` refusal semantics.
//!
//! **D3 — bidegree budget.** The engine's certified hull / Bernstein work is
//! exercised with modest tensor degrees; this admission enforces a documented
//! budget so a degree blow-up cannot leak past the funnel. The budget is a
//! module constant (recorded in RESULT notes).

use crate::ssi::{RationalBipatch, SsiRefusal};
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// Why a spline carrier could not be admitted.
///
/// Named cases only, no catch-all — matching the refusal shape used across the
/// crate's certified layers. Each variant carries a stable diagnostic tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineAdmissionRefusal {
    /// The control net is empty or ragged, or the knot vectors are empty.
    EmptyControlNet,
    /// A control point coordinate (or its weight) is not finite.
    NonFiniteControlPoint,
    /// The knot vectors are not both non-decreasing (inverted knots).
    InvertedKnotVector,
    /// A degree is zero, so no Bézier patch of positive bidegree exists.
    ZeroDegree,
    /// The bidegree exceeds the engine's admission budget (see
    /// [`MAX_BIDEGREE`]).
    DegreeBudgetExceeded,
    /// The surface has no non-empty knot span (no patch would be produced).
    NoSpan,
    /// The homogeneous carrier is rational: at least one control weight is not
    /// within tolerance of 1 (the landed `NonPositiveNurbsWeight` refusal).
    RationalWeights,
    /// A landed certified operation refused while building the patch grids.
    Certified,
}

impl SplineAdmissionRefusal {
    /// The stable diagnostic tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::EmptyControlNet => "spline_admission_empty_control_net",
            Self::NonFiniteControlPoint => "spline_admission_non_finite",
            Self::InvertedKnotVector => "spline_admission_inverted_knots",
            Self::ZeroDegree => "spline_admission_zero_degree",
            Self::DegreeBudgetExceeded => "spline_admission_degree_budget",
            Self::NoSpan => "spline_admission_no_span",
            Self::RationalWeights => "spline_admission_non_positive_nurbs_weight",
            Self::Certified => "spline_admission_certified",
        }
    }
}

impl From<SsiRefusal> for SplineAdmissionRefusal {
    fn from(_: SsiRefusal) -> Self {
        Self::Certified
    }
}

/// The documented bidegree admission budget (D3).
pub const MAX_BIDEGREE: usize = 8;

/// Relative tolerance for a homogeneous control weight to count as unit (D2).
const UNIT_WEIGHT_REL_TOL: f64 = 1.0e-7;

/// One admitted Bézier patch: its rational tensor-Bernstein form (over the
/// unit square) and the source knot-span rectangle it was extracted from.
#[derive(Debug, Clone)]
pub struct AdmittedPatch {
    /// The unit-square rational Bézier patch.
    pub patch: RationalBipatch,
    /// The source knot-span rectangle `(u0, u1) x (v0, v1)`.
    pub cell: ((f64, f64), (f64, f64)),
}

/// The admitted patch stack of a spline surface: one patch per knot-span
/// cell, `u`-span major then `v`-span minor.
#[derive(Debug, Clone)]
pub struct SplinePatchStack {
    patches: Vec<AdmittedPatch>,
}

impl SplinePatchStack {
    /// The admitted patches.
    pub fn patches(&self) -> &[AdmittedPatch] {
        &self.patches
    }

    /// The number of admitted patches.
    pub fn len(&self) -> usize {
        self.patches.len()
    }

    /// Whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }
}

/// Admit a clamped spline surface into its Bézier patch stack.
///
/// The carrier is the homogeneous spline net (control points in R4; the last
/// coordinate is the weight). Every weight must be within tolerance of 1 (D2);
/// rational carriers refuse [`SplineAdmissionRefusal::RationalWeights`].
///
/// # Refusals
///
/// * empty or ragged control net, or empty knots
///   ([`SplineAdmissionRefusal::EmptyControlNet`]);
/// * non-finite control coordinates ([`SplineAdmissionRefusal::NonFiniteControlPoint`]);
/// * inverted (non-monotone) knot vectors ([`SplineAdmissionRefusal::InvertedKnotVector`]);
/// * a zero degree ([`SplineAdmissionRefusal::ZeroDegree`]);
/// * a bidegree above [`MAX_BIDEGREE`] ([`SplineAdmissionRefusal::DegreeBudgetExceeded`]);
/// * no non-empty knot span ([`SplineAdmissionRefusal::NoSpan`]).
pub fn admit_surface(
    surface: &BSplineSurface<Vector4>,
) -> Result<SplinePatchStack, SplineAdmissionRefusal> {
    let (du, dv) = (surface.udegree(), surface.vdegree());
    if du == 0 || dv == 0 {
        return Err(SplineAdmissionRefusal::ZeroDegree);
    }
    if du > MAX_BIDEGREE || dv > MAX_BIDEGREE {
        return Err(SplineAdmissionRefusal::DegreeBudgetExceeded);
    }
    // Bézier extraction requires clamped (full-multiplicity-end) knot vectors;
    // an open knot vector has tail spans that cannot align to
    // `(degree + 1)`-wide Bézier cell blocks (the CG kernel's clamped
    // precondition, leaf_extract "spline_not_clamped").
    if !surface.is_clamped() {
        return Err(SplineAdmissionRefusal::InvertedKnotVector);
    }

    let control = surface.control_points();
    let width = control
        .first()
        .map(Vec::len)
        .ok_or(SplineAdmissionRefusal::EmptyControlNet)?;
    if width == 0 {
        return Err(SplineAdmissionRefusal::EmptyControlNet);
    }
    if control.iter().any(|row| row.len() != width) {
        return Err(SplineAdmissionRefusal::EmptyControlNet);
    }

    let uknot = distinct(surface.uknot_vec());
    let vknot = distinct(surface.vknot_vec());
    if uknot.len() < 2 || vknot.len() < 2 {
        return Err(SplineAdmissionRefusal::NoSpan);
    }
    if !non_decreasing(&uknot) || !non_decreasing(&vknot) {
        return Err(SplineAdmissionRefusal::InvertedKnotVector);
    }

    for pt in control.iter().flatten() {
        if !pt.x.is_finite() || !pt.y.is_finite() || !pt.z.is_finite() || !pt.w.is_finite() {
            return Err(SplineAdmissionRefusal::NonFiniteControlPoint);
        }
        let w = pt.w;
        if w <= 0.0 || (w - 1.0).abs() > UNIT_WEIGHT_REL_TOL {
            return Err(SplineAdmissionRefusal::RationalWeights);
        }
    }

    // Refine: insert every interior distinct knot to full multiplicity.
    let mut refined = surface.clone();
    for &x in uknot.iter() {
        if x == uknot[0] || x == *uknot.last().unwrap_or(&x) {
            continue;
        }
        while exact_multiplicity(refined.uknot_vec(), x) < du + 1 {
            refined.add_uknot(x);
        }
    }
    for &y in vknot.iter() {
        if y == vknot[0] || y == *vknot.last().unwrap_or(&y) {
            continue;
        }
        while exact_multiplicity(refined.vknot_vec(), y) < dv + 1 {
            refined.add_vknot(y);
        }
    }

    let refined_ctrl: Vec<Vec<[f64; 3]>> = refined
        .control_points()
        .iter()
        .map(|row| {
            row.iter()
                .map(|pt| [pt.x / pt.w, pt.y / pt.w, pt.z / pt.w])
                .collect()
        })
        .collect();

    let n_u_cells = uknot.len() - 1;
    let n_v_cells = vknot.len() - 1;
    let mut patches = Vec::with_capacity(n_u_cells * n_v_cells);
    for iu in 0..n_u_cells {
        for iv in 0..n_v_cells {
            let m = du;
            let n = dv;
            let mut xg = vec![vec![0.0f64; n + 1]; m + 1];
            let mut yg = vec![vec![0.0f64; n + 1]; m + 1];
            let mut zg = vec![vec![0.0f64; n + 1]; m + 1];
            for a in 0..=m {
                for b in 0..=n {
                    let c = refined_ctrl
                        .get(iu * (du + 1) + a)
                        .and_then(|row| row.get(iv * (dv + 1) + b))
                        .ok_or(SplineAdmissionRefusal::Certified)?;
                    xg[a][b] = c[0];
                    yg[a][b] = c[1];
                    zg[a][b] = c[2];
                }
            }
            let weights = vec![vec![1.0; n + 1]; m + 1];
            let patch = RationalBipatch::new(m, n, [xg, yg, zg], weights)?;
            patches.push(AdmittedPatch {
                patch,
                cell: ((uknot[iu], uknot[iu + 1]), (vknot[iv], vknot[iv + 1])),
            });
        }
    }
    Ok(SplinePatchStack { patches })
}

/// Whether a sorted-knot sequence is non-decreasing.
fn non_decreasing(knots: &[f64]) -> bool {
    knots.windows(2).all(|w| w[0] <= w[1])
}

/// The distinct ascending knot values of a clamped knot vector.
fn distinct(knots: &KnotVec) -> Vec<f64> {
    let mut out = Vec::new();
    for &k in knots.iter() {
        if out.last() != Some(&k) {
            out.push(k);
        }
    }
    out
}

/// Exact occurrence count of `x` (not tolerance-merged).
fn exact_multiplicity(knots: &KnotVec, x: f64) -> usize {
    knots.iter().filter(|&&k| k == x).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssi::{construct_square_system, SsiParticipant};
    use truck_geometry::prelude::{KnotVec, Vector4};

    fn surf(n: usize, w: f64) -> BSplineSurface<Vector4> {
        let uknot = KnotVec::bezier_knot(3);
        let vknot = KnotVec::bezier_knot(3);
        let ctrl = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        Vector4::new(i as f64 * 0.25, j as f64 * 0.25, (i * j) as f64 * 0.1, w)
                    })
                    .collect()
            })
            .collect();
        BSplineSurface::new((uknot, vknot), ctrl)
    }

    #[test]
    fn patch_admission_decomposes_and_refuses() {
        // A single-span bicubic (Bezier) surface decomposes into exactly one
        // patch whose numerator grid equals the dehomogenized control net.
        let surface = surf(4, 1.0);
        let stack = admit_surface(&surface).expect("a bicubic carrier admits");
        assert_eq!(stack.len(), 1);
        let admitted = &stack.patches()[0];
        assert_eq!(admitted.patch.m(), 3);
        assert_eq!(admitted.patch.n(), 3);
        let num = admitted.patch.numerator();
        for (k, _) in num.iter().enumerate() {
            for a in 0..=3 {
                for b in 0..=3 {
                    let expected = match k {
                        0 => a as f64 * 0.25,
                        1 => b as f64 * 0.25,
                        _ => (a * b) as f64 * 0.1,
                    };
                    let approx = (num[k][a][b] - expected).abs();
                    assert!(
                        approx < 1e-12,
                        "component {k} grid ({a},{b}) drift {approx}"
                    );
                }
            }
        }

        // Open (unclamped) knot layout — the CG "inverted knots" precondition:
        // tail spans cannot be Bézier-extracted into cell blocks.
        let open_u = KnotVec::from(vec![0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
        let vknot = KnotVec::from(vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);
        let ctrl = vec![vec![Vector4::new(0.0, 0.0, 0.0, 1.0); 4]; 4];
        let bad = BSplineSurface::new_unchecked((open_u, vknot), ctrl);
        assert!(matches!(
            admit_surface(&bad),
            Err(SplineAdmissionRefusal::InvertedKnotVector)
        ));

        // NaN control points refuse typed.
        let nan = BSplineSurface::new_unchecked(
            (KnotVec::bezier_knot(3), KnotVec::bezier_knot(3)),
            vec![vec![Vector4::new(f64::NAN, 0.0, 0.0, 1.0); 4]; 4],
        );
        assert!(matches!(
            admit_surface(&nan),
            Err(SplineAdmissionRefusal::NonFiniteControlPoint)
        ));

        // Rational weights (≠ 1) refuse typed (NonPositiveNurbsWeight).
        let rational = surf(4, 2.0);
        assert!(matches!(
            admit_surface(&rational),
            Err(SplineAdmissionRefusal::RationalWeights)
        ));
    }

    #[test]
    fn admitted_patches_feed_square_system() {
        // Two single-span bicubic carriers admit to one patch each; the SSI
        // engine accepts the bridge's output (end-to-end compile + run proof).
        let a = admit_surface(&surf(4, 1.0)).expect("surface A admits");
        let b = admit_surface(&surf(4, 1.0)).expect("surface B admits");
        let pa = &a.patches()[0].patch;
        let pb = &b.patches()[0].patch;
        let participant = |p: &RationalBipatch| SsiParticipant::RationalBipatch(p.clone());
        let system = construct_square_system(&participant(pa), &participant(pb))
            .expect("the engine accepts the bridge output");
        assert_eq!(system.degrees(), (3, 3, 3, 3));
    }
}
