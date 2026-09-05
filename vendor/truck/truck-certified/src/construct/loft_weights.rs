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

//! CC-011-LOFT-WEIGHTS (theory §2.2 L1r): the certified positive weight field
//! of a delivered loft (spine S8 consumer).
//!
//! Collocation is not weight-preserving: even when every input section weight
//! is strictly positive, the interpolated homogeneous control net of a loft can
//! carry non-positive `w_ij`, and a strictly positive input net does NOT imply
//! a positive interpolated weight field — the inverse of a totally positive
//! collocation matrix has a checkerboard sign pattern, so negative `w_ij`
//! appear exactly where the interpolation overshoots. The delivered rational
//! surface `S(u, v) = Σ N_i(u) N_j(v) (X_ij, Y_ij, Z_ij, W_ij)` has the scalar
//! weight field `w(u, v) = Σ N_i(u) N_j(v) W_ij` as its denominator: a
//! non-positive `w` anywhere is a pole inside the domain. This module certifies
//! strict positivity of the delivered weight field, or refuses.
//!
//! **Carrier note (S8).** The theory seam writes the homogeneous control point
//! as "`Point4`". Per the S8 amendment the landed homogeneous carrier of
//! `truck_geometry` is [`Vector4`], and the loft delivers a
//! `BSplineSurface<Vector4>`; this packet certifies exactly that type. The
//! weight field is the `w`-channel of the homogeneous control net.
//!
//! **Fast path (free, sufficient).** If `min w_ij > 0` over the control net in
//! row-major order, the B-spline basis is a non-negative partition of unity
//! over the (clamped) domain, so the weight field is a convex combination of
//! the net weights at every parameter and is strictly positive everywhere.
//! Admit with `refined: false` and no refinements — no subdivision is spent.
//!
//! **Refinement fallback.** When the net is not all-positive, the field is
//! decomposed into its Bézier patches (each knot raised to its full
//! multiplicity — the exact patch extraction, no tolerance) and every patch's
//! weight grid is certified through
//! [`hull_bernstein_2d`](crate::hull::hull_bernstein_2d) over the full unit
//! patch. A patch whose certified hull lies strictly above zero admits (the
//! convex-hull property: the field over the patch is a convex combination of
//! its Bézier weights). A patch whose certified hull lies at or below zero is
//! a certified non-positive (or certified-zero) region of the field — refuse
//! [`ConstructRefusal::NonPositiveWeightField`] immediately: no refinement of
//! the same surface can ever make a ≤ 0 field positive there. A patch whose
//! hull straddles zero is not decidable and is subdivided.
//!
//! **Dyadic subdivision only (stop condition 3).** Every split halves one
//! parameter span of one patch at its exact midpoint, so the refinement knots
//! are exact midpoints of representable knots (dyadic in the patch ancestry)
//! and are exactly representable. Each split spends exactly one
//! [`Budget::spend_subdiv`]; the depth cap is the normative
//! [`CC_DEPTH_MAX`](crate::construct::config::CC_DEPTH_MAX). Budget exhaustion,
//! a patch whose enclosure is still straddling at the cap, or a certified
//! non-positive patch all refuse `NonPositiveWeightField` — never a geometry
//! failure: this is a failure of THIS field's admissibility, and the refusal
//! says exactly that.
//!
//! **The storage rule (theory D4-clause-(a) lineage).** A certificate produced
//! under refinement is valid ONLY if the identical knot insertions are applied
//! to the shipped surface. [`certify_weight_field`] never mutates its input:
//! every knot insertion it applies is returned inside [`WeightCert::refinements`]
//! as an `(axis, index, knot)` triple replayable through
//! `BSplineSurface::add_uknot` / `add_vknot` (see [`WeightCert`]). The caller
//! (CC-012/CC-014) applies `refinements` to the shipped net; applying them is a
//! CC-012 obligation, booked there.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every reduction runs in a fixed order (C9).
//! `hull.rs` is not modified: the weight grids are plain rectangular
//! `Vec<Vec<f64>>` extractions and are hulled directly (stop condition 2).

use crate::construct::config::CC_DEPTH_MAX;
use crate::construct::refusal::ConstructRefusal;
use crate::hull::hull_bernstein_2d;
use truck_base::evidence::Budget;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Vector4};

/// The certified positive-weight-field verdict.
///
/// `min_control_weight` is the minimum `w`-control value of the certified net:
/// the net the certificate speaks about. On the fast path that is the input
/// net and `refined` is `false`. Under refinement it is the minimum of the
/// refined net — the net obtained by applying `refinements` to the input — and
/// `refined` is `true`. `refinements` holds every knot insertion the
/// certificate applied, in application order.
#[derive(Debug, Clone)]
pub struct WeightCert {
    /// The minimum `w`-control value of the certified (possibly refined) net.
    pub min_control_weight: f64,
    /// Whether the certificate required subdivision (`true`) or admitted on
    /// the free fast path (`false`).
    pub refined: bool,
    /// The knot insertions the certificate applied, in application order. Each
    /// element is `(axis, index, knot)`: `axis == false` selects the `u` axis
    /// (replay with `add_uknot`), `axis == true` selects the `v` axis (replay
    /// with `add_vknot`); `knot` is the exact inserted value (a boundary raised
    /// to its full multiplicity, or the exact dyadic midpoint of a certified
    /// patch span); `index` is the landing index of the knot in that axis's
    /// knot vector immediately before the insertion — the index
    /// `KnotVec::add_knot` reports — recorded for determinism, while replay is
    /// positional by value in recorded order. Applying the whole list in order
    /// to the shipped surface reproduces the certified refined net exactly.
    pub refinements: Vec<(bool, usize, f64)>,
}

/// The certified verdict over one Bézier patch of the weight field.
enum LeafVerdict {
    /// The patch's hull lies strictly above zero: the field is strictly
    /// positive over the whole patch (convex-hull property).
    Positive,
    /// The patch's hull straddles zero: not decidable at this level, the patch
    /// must be subdivided (or refused at the depth cap).
    Straddling,
}

/// Certify strict positivity of the weight field of a delivered homogeneous
/// loft surface, refining (dyadically) or refusing.
///
/// `surface` must be a clamped tensor-product B-spline (the loft delivery
/// form). The free fast path admits any net with `min w_ij > 0` without
/// touching the budget. Otherwise the weight field is Bézier-extracted and
/// certified patch-by-patch through `hull_bernstein_2d`, subdividing
/// straddling patches at exact midpoints; every subdivision spends one
/// `budget.spend_subdiv()` and the depth cap is [`CC_DEPTH_MAX`]. Any refusal
/// — budget exhaustion, a certified non-positive / certified-zero patch, or a
/// patch still straddling at the cap — is
/// [`ConstructRefusal::NonPositiveWeightField`]: the weight field of THIS
/// surface is not admissible, never a statement about the geometry.
pub fn certify_weight_field(
    surface: &BSplineSurface<Vector4>,
    budget: &mut Budget,
) -> Result<WeightCert, ConstructRefusal> {
    if !surface.is_clamped() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let du = surface.udegree();
    let dv = surface.vdegree();
    let points = surface.control_points();
    if points.is_empty() || points[0].is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }

    // Fast path: a strictly positive net certifies the field (convex hull of a
    // clamped B-spline is a non-negative partition of unity). Free and
    // sufficient: admit without subdivision.
    let coarse_min = net_min(surface);
    if coarse_min > 0.0 {
        return Ok(WeightCert {
            min_control_weight: coarse_min,
            refined: false,
            refinements: Vec::new(),
        });
    }

    // Refinement fallback. The certification speaks about the refined net, so
    // the working copy is mutated and every insertion is recorded.
    //
    // Layout rule: a degree-0 axis with more than one span is piecewise
    // constant (discontinuous) and has no full-multiplicity Bézier layout;
    // such a weight field is outside the certified patch form.
    if du == 0 && distinct_knots(surface.uknot_vec()).len() > 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    if dv == 0 && distinct_knots(surface.vknot_vec()).len() > 2 {
        return Err(ConstructRefusal::InvalidInput);
    }

    let mut work = surface.clone();
    let mut refinements: Vec<(bool, usize, f64)> = Vec::new();
    raise_u_multiplicities(&mut work, &mut refinements);
    raise_v_multiplicities(&mut work, &mut refinements);

    // The distinct knot values (the patch spans) and each span's subdivision
    // depth (number of dyadic splits on its ancestry from the input surface).
    let mut du_vals = distinct_knots(work.uknot_vec());
    let mut dv_vals = distinct_knots(work.vknot_vec());
    let mut du_gen: Vec<u32> = vec![0; du_vals.len() - 1];
    let mut dv_gen: Vec<u32> = vec![0; dv_vals.len() - 1];

    // The full-multiplicity Bézier layout is required for the patch blocks to
    // be the surface's control-point blocks. A clamped input with end
    // multiplicity above degree + 1 (or any other layout drift) refuses.
    if work.control_points().len() != (du_vals.len() - 1) * du + 1
        || work.control_points()[0].len() != (dv_vals.len() - 1) * dv + 1
    {
        return Err(ConstructRefusal::InvalidInput);
    }

    loop {
        let mut first_straddling: Option<(usize, usize)> = None;
        for iu in 0..(du_vals.len() - 1) {
            for iv in 0..(dv_vals.len() - 1) {
                match leaf_verdict(&work, iu, du, iv, dv)? {
                    LeafVerdict::Positive => {}
                    LeafVerdict::Straddling => {
                        let depth = du_gen[iu] + dv_gen[iv];
                        if depth >= CC_DEPTH_MAX {
                            // A patch whose enclosure is still straddling at
                            // the depth cap cannot certify: refuse.
                            return Err(ConstructRefusal::NonPositiveWeightField);
                        }
                        if first_straddling.is_none() {
                            first_straddling = Some((iu, iv));
                        }
                    }
                }
            }
        }
        let (iu, iv) = match first_straddling {
            Some(leaf) => leaf,
            None => {
                // Every patch certified positive: the refined net is
                // all-positive and is the certificate.
                let refined_min = net_min(&work);
                return Ok(WeightCert {
                    min_control_weight: refined_min,
                    refined: true,
                    refinements,
                });
            }
        };

        budget
            .spend_subdiv(1)
            .map_err(|_| ConstructRefusal::NonPositiveWeightField)?;

        // Split the straddling patch along the wider of its two spans (a
        // degree-0 axis is never split: it carries no refinement freedom), at
        // the exact span midpoint. Each split raises the new knot to its full
        // multiplicity so the Bézier layout is preserved.
        let split_u = choose_split_axis(&du_vals, &dv_vals, iu, iv, du, dv);
        if split_u {
            let a = du_vals[iu];
            let b = du_vals[iu + 1];
            let mid = a + (b - a) * 0.5;
            for _ in 0..du {
                insert_u_knot(&mut work, &mut refinements, mid);
            }
            du_vals.insert(iu + 1, mid);
            let generation = du_gen[iu] + 1;
            du_gen[iu] = generation;
            du_gen.insert(iu + 1, generation);
        } else {
            let a = dv_vals[iv];
            let b = dv_vals[iv + 1];
            let mid = a + (b - a) * 0.5;
            for _ in 0..dv {
                insert_v_knot(&mut work, &mut refinements, mid);
            }
            dv_vals.insert(iv + 1, mid);
            let generation = dv_gen[iv] + 1;
            dv_gen[iv] = generation;
            dv_gen.insert(iv + 1, generation);
        }
    }
}

/// The minimum `w`-control value of the net, in row-major order (fixed
/// reduction order, C9).
fn net_min(surface: &BSplineSurface<Vector4>) -> f64 {
    let mut min = f64::INFINITY;
    for row in surface.control_points() {
        for point in row {
            if point.w < min {
                min = point.w;
            }
        }
    }
    min
}

/// The distinct knot values of `knots`, ascending and exactly deduplicated.
fn distinct_knots(knots: &KnotVec) -> Vec<f64> {
    let mut values: Vec<f64> = Vec::new();
    for &knot in knots.as_slice() {
        match values.last() {
            Some(&last) if last == knot => {}
            _ => values.push(knot),
        }
    }
    values
}

/// The landing index of `knot` in `knots` immediately before an insertion:
/// the index `KnotVec::add_knot` reports (one past the last knot `<= knot`).
fn insertion_index(knots: &KnotVec, knot: f64) -> usize {
    match knots.floor(knot) {
        Some(index) => index + 1,
        None => 0,
    }
}

/// Insert `knot` into the `u` axis of `work` and record the insertion.
fn insert_u_knot(
    work: &mut BSplineSurface<Vector4>,
    refinements: &mut Vec<(bool, usize, f64)>,
    knot: f64,
) {
    let index = insertion_index(work.uknot_vec(), knot);
    work.add_uknot(knot);
    refinements.push((false, index, knot));
}

/// Insert `knot` into the `v` axis of `work` and record the insertion.
fn insert_v_knot(
    work: &mut BSplineSurface<Vector4>,
    refinements: &mut Vec<(bool, usize, f64)>,
    knot: f64,
) {
    let index = insertion_index(work.vknot_vec(), knot);
    work.add_vknot(knot);
    refinements.push((true, index, knot));
}

/// The number of exact occurrences of `knot` in `knots`.
fn knot_multiplicity(knots: &KnotVec, knot: f64) -> usize {
    knots.as_slice().iter().filter(|&&k| k == knot).count()
}

/// Raise every interior knot of the `u` axis to its full multiplicity `du`,
/// recording each insertion. Idempotent once the axis is fully Bézier.
fn raise_u_multiplicities(
    work: &mut BSplineSurface<Vector4>,
    refinements: &mut Vec<(bool, usize, f64)>,
) {
    let degree = work.udegree();
    if degree == 0 {
        return;
    }
    let interior = distinct_interior(work.uknot_vec());
    for value in interior {
        let deficit = degree.saturating_sub(knot_multiplicity(work.uknot_vec(), value));
        for _ in 0..deficit {
            insert_u_knot(work, refinements, value);
        }
    }
}

/// Raise every interior knot of the `v` axis to its full multiplicity `dv`,
/// recording each insertion. Idempotent once the axis is fully Bézier.
fn raise_v_multiplicities(
    work: &mut BSplineSurface<Vector4>,
    refinements: &mut Vec<(bool, usize, f64)>,
) {
    let degree = work.vdegree();
    if degree == 0 {
        return;
    }
    let interior = distinct_interior(work.vknot_vec());
    for value in interior {
        let deficit = degree.saturating_sub(knot_multiplicity(work.vknot_vec(), value));
        for _ in 0..deficit {
            insert_v_knot(work, refinements, value);
        }
    }
}

/// The interior distinct knot values of `knots` (all but the first and last).
fn distinct_interior(knots: &KnotVec) -> Vec<f64> {
    let values = distinct_knots(knots);
    if values.len() <= 2 {
        Vec::new()
    } else {
        values[1..values.len() - 1].to_vec()
    }
}

/// The certified hull verdict over the patch whose `u` span is `iu` and whose
/// `v` span is `iv` (full-Bézier layout assumed).
fn leaf_verdict(
    work: &BSplineSurface<Vector4>,
    iu: usize,
    du: usize,
    iv: usize,
    dv: usize,
) -> Result<LeafVerdict, ConstructRefusal> {
    let grid = leaf_weight_grid(work, iu, du, iv, dv);
    let hull = hull_bernstein_2d(&grid, (0.0, 1.0), (0.0, 1.0))
        .map_err(|_| ConstructRefusal::NonPositiveWeightField)?;
    if hull.lo > 0.0 {
        Ok(LeafVerdict::Positive)
    } else if hull.hi <= 0.0 {
        // Certified non-positive (or certified zero) over the whole patch: the
        // field is not strictly positive here, and no refinement of the same
        // surface can change that.
        Err(ConstructRefusal::NonPositiveWeightField)
    } else {
        Ok(LeafVerdict::Straddling)
    }
}

/// The `w`-weight grid of the patch `(iu, iv)`: the `(du + 1) × (dv + 1)`
/// control block, rows along `u`, columns along `v`.
fn leaf_weight_grid(
    work: &BSplineSurface<Vector4>,
    iu: usize,
    du: usize,
    iv: usize,
    dv: usize,
) -> Vec<Vec<f64>> {
    work.control_points()
        .iter()
        .skip(iu * du)
        .take(du + 1)
        .map(|row| {
            row.iter()
                .skip(iv * dv)
                .take(dv + 1)
                .map(|point| point.w)
                .collect()
        })
        .collect()
}

/// Choose the split axis of the straddling patch `(iu, iv)`: never a degree-0
/// axis (it carries no refinement freedom); otherwise the axis whose current
/// span is wider, `u` on a tie (determinism, C9).
fn choose_split_axis(
    du_vals: &[f64],
    dv_vals: &[f64],
    iu: usize,
    iv: usize,
    du: usize,
    dv: usize,
) -> bool {
    match (du == 0, dv == 0) {
        (true, false) => false,
        (false, true) => true,
        (true, true) => false,
        (false, false) => {
            let width_u = du_vals[iu + 1] - du_vals[iu];
            let width_v = dv_vals[iv + 1] - dv_vals[iv];
            width_u >= width_v
        }
    }
}
