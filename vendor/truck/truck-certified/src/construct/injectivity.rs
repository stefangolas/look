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

//! CC-002-INJECTIVITY (spine seam S4): the P2 local injectivity radius
//! `δ = 2σ/L` over the certified map types (`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md`
//! §1 P2). Consumers: loft L5 near-diagonal discharge (CC-014), offset star
//! certificates (CC-021/022), and blend spine self-intersection (CC-030).
//!
//! This primitive makes self-contact testing terminate: no contact test is
//! required for parameter pairs with `‖p − q‖ < δ`. Both constants are
//! certified over the queried region:
//!
//! - `σ` is the certified LOWER bound of the map's rank margin on the region
//!   (`map.rank_margin(sub)`, the `|S_u × S_v|` lower bound for a surface and
//!   the `|C'|` lower bound for a curve).
//! - `L` is a certified UPPER bound of `sup ‖D²S‖` on the region, built from
//!   the landed `hull.rs` derivative kernels over the map's Bézier
//!   decomposition (`patch_grids`/`patch_boxes` for a surface,
//!   `piece_grids`/`piece_intervals` for a curve).
//!
//! Pre-made decisions (packet tags; do not relitigate):
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. It carries no `unwrap`, no `expect`, and no `panic!`, and adds
//! no module-level `allow`.
//!
//! **Refusal — one variant, three triggers.** Every refusal leaves this module
//! as [`ConstructRefusal::InvalidInput`], and never propagates a NaN:
//!
//! - `σ ≤ 0` over the region is a degenerate parameterization — an input
//!   defect. (The map's own admit-time check uses the distinct
//!   `MapRefusal::ParameterizationDegenerate`; do not conflate the two.)
//! - `L = 0` (a flat region, e.g. a planar patch) carries no curvature-driven
//!   self-contact: `δ` is the `Interval` at `+∞` (`lo = hi = f64::INFINITY`).
//!   Flatness is decided on the EXACT zero coefficient grids of the second
//!   partials (every touched patch's derived second-derivative coefficients are
//!   identically `0.0`) — never on the rounding slivers their interval hulls
//!   would otherwise report.
//! - A non-finite intermediate (a refused hull, a refused margin, or a
//!   non-finite derivative bound) refuses `InvalidInput`.
//!
//! **L — fixed, deterministic accumulation order.** Per patch, the three
//! second partials `S_uu`, `S_vv`, `S_uv` are formed as SOURCE-derivative
//! coefficient grids (unit-parameter second derivative scaled by the patch
//! width products), each coordinate hull is bounded by
//! `hull::hull_bernstein_2d` over the region overlap, and the three partials
//! are each assigned the certified sup of their Euclidean norm over that
//! coordinate enclosure (componentwise bound). The patch contribution is the
//! max of the three partial sups; the region `L` is the max over the patches
//! the region touches. The curve variant is the same reduction over the 1-D
//! `hull::hull_bernstein_1d` hulls of each piece's twice-differentiated
//! coefficient vector. No new hull kernel is written; both variants compose
//! the landed `bernstein_derivative_{1d,2d}` + `hull_bernstein_{1d,2d}`.
//!
//! **Determinism.** All reductions run in fixed order (patch order, then
//! partial order `uu`, `vv`, `uv`, then coordinate order), with directed
//! rounding at every step.

use crate::certified_map::{CertifiedCurveMap, CertifiedSurfaceMap, CurveRegion, SurfaceRegion};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use crate::hull::{
    bernstein_derivative_1d, bernstein_derivative_2d, hull_bernstein_1d, hull_bernstein_2d,
};

/// The certified P2 injectivity radius `δ = 2σ/L` of a surface map over a
/// compact subregion of its declared domain.
///
/// `σ` is the certified `|S_u × S_v|` lower bound from
/// `map.rank_margin(sub)`; `L` is a certified upper bound of `sup ‖D²S‖` over
/// `sub` (second partials `S_uu`, `S_vv`, `S_uv`, hulled per patch over the
/// region overlap). Returns `Err(ConstructRefusal::InvalidInput)` when the
/// region's certified margin is not strictly positive (a degenerate
/// parameterization over the region) or a certified intermediate is
/// non-finite; returns the `Interval` at `+∞` when `L = 0` (a flat region).
pub fn injectivity_radius(
    map: &CertifiedSurfaceMap,
    sub: SurfaceRegion,
) -> Result<Interval, ConstructRefusal> {
    let sigma = surface_margin_lower_bound(map, sub)?;
    let curvature = surface_curvature_upper_bound(map, sub)?;
    delta_interval(sigma, curvature)
}

/// The certified 1-D P2 injectivity radius `δ = 2σ/L` of a curve map over a
/// compact subinterval of its declared domain.
///
/// `σ` is the certified `|C'|` lower bound from `map.rank_margin(sub)`; `L` is
/// a certified upper bound of `sup ‖C″‖` over `sub` (second-derivative hulls
/// of the map's Bézier pieces, via `map.piece_grids`). Refusal and flat-region
/// conventions match [`injectivity_radius`].
pub fn curve_injectivity_radius(
    map: &CertifiedCurveMap,
    sub: CurveRegion,
) -> Result<Interval, ConstructRefusal> {
    let sigma = curve_margin_lower_bound(map, sub)?;
    let curvature = curve_curvature_upper_bound(map, sub)?;
    delta_interval(sigma, curvature)
}

/// The certified `|S_u × S_v|` lower bound of the region: the margin interval's
/// lower endpoint. A non-positive certified margin is a degenerate
/// parameterization — an input defect, refused as `InvalidInput`.
fn surface_margin_lower_bound(
    map: &CertifiedSurfaceMap,
    sub: SurfaceRegion,
) -> Result<f64, ConstructRefusal> {
    let margin = map
        .rank_margin(sub)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let sigma = margin.lo;
    if sigma <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(sigma)
}

/// The certified `|C'|` lower bound of the region (the curve mirror of
/// [`surface_margin_lower_bound`]).
fn curve_margin_lower_bound(
    map: &CertifiedCurveMap,
    sub: CurveRegion,
) -> Result<f64, ConstructRefusal> {
    let margin = map
        .rank_margin(sub)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let sigma = margin.lo;
    if sigma <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(sigma)
}

/// The certified `δ = 2σ/L` interval from a certified margin lower bound `σ`
/// and a certified curvature upper bound `L`, with the flat (`L = 0`) and
/// non-finite conventions.
fn delta_interval(sigma: f64, curvature_upper: f64) -> Result<Interval, ConstructRefusal> {
    if curvature_upper == 0.0 {
        return Ok(Interval {
            lo: f64::INFINITY,
            hi: f64::INFINITY,
        });
    }
    if !sigma.is_finite() || !curvature_upper.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let two_sigma = Interval::point(2.0).mul(&Interval::point(sigma));
    let delta = two_sigma
        .div(&Interval::point(curvature_upper))
        .ok_or(ConstructRefusal::InvalidInput)?;
    if delta.is_finite() {
        Ok(delta)
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// A certified upper bound of `sup ‖D²S‖` over `sub`: decompose the region
/// over the map's Bézier patches and take, per patch, the max over the three
/// second partials of the certified sup of their norm over the region overlap.
fn surface_curvature_upper_bound(
    map: &CertifiedSurfaceMap,
    sub: SurfaceRegion,
) -> Result<f64, ConstructRefusal> {
    let patch_boxes = map.patch_boxes();
    let patch_grids = map.patch_grids();
    let mut l = 0.0_f64;
    for (patch_box, grids) in patch_boxes.iter().zip(patch_grids.iter()) {
        let Some(overlap) = rectangle_overlap(*patch_box, sub) else {
            continue;
        };
        let (s_lo, s_hi) = unit_image(patch_box.0, overlap.0)?;
        let (t_lo, t_hi) = unit_image(patch_box.1, overlap.1)?;
        let width_u = patch_box.0 .1 - patch_box.0 .0;
        let width_v = patch_box.1 .1 - patch_box.1 .0;
        if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_u = 1.0 / width_u;
        let inv_v = 1.0 / width_v;
        let patch_l = second_partial_sup_2d(grids, (s_lo, s_hi), (t_lo, t_hi), inv_u, inv_v)?;
        l = l.max(patch_l);
    }
    Ok(l)
}

/// The certified per-patch curvature sup: for each of the three second partials
/// `S_uu`, `S_vv`, `S_uv` (source derivatives), hull each coordinate over the
/// unit subbox and take the sup of the norm over the coordinate enclosure; the
/// patch contributes the max over the three partials. A partial whose derived
/// coefficient grid is EXACTLY zero contributes zero (its hulls are only
/// rounding slivers around zero and must not be mistaken for curvature); when
/// every partial is exactly zero the patch is flat (`L = 0` on it).
/// `inv_u`/`inv_v` are the inverse patch widths that convert unit-parameter
/// derivative grids into source-derivative grids.
fn second_partial_sup_2d(
    grids: &[Vec<Vec<f64>>; 3],
    s: (f64, f64),
    t: (f64, f64),
    inv_u: f64,
    inv_v: f64,
) -> Result<f64, ConstructRefusal> {
    let mut patch_l = 0.0_f64;
    for partial in 0..3 {
        let mut components = [Interval::point(0.0); 3];
        let mut flat_partial = true;
        for (k, grid) in grids.iter().enumerate() {
            let coeffs = match partial {
                0 => derived_grid_2d(grid, 0, 0, inv_u * inv_u),
                1 => derived_grid_2d(grid, 1, 1, inv_v * inv_v),
                _ => derived_grid_2d(grid, 0, 1, inv_u * inv_v),
            };
            if coeffs.iter().any(|row| row.iter().any(|c| *c != 0.0)) {
                flat_partial = false;
            }
            components[k] =
                hull_bernstein_2d(&coeffs, s, t).map_err(|_| ConstructRefusal::InvalidInput)?;
        }
        if !flat_partial {
            patch_l = patch_l.max(norm_sup(&components)?);
        }
    }
    Ok(patch_l)
}

/// A second-derivative Bernstein coefficient grid in SOURCE units: derive the
/// tensor grid twice along the two fixed axes (`axis_a` then `axis_b` of the
/// unit parameter) and scale every coefficient by `scale` (the inverse patch
/// width product).
fn derived_grid_2d(grid: &[Vec<f64>], axis_a: usize, axis_b: usize, scale: f64) -> Vec<Vec<f64>> {
    let first = bernstein_derivative_2d(grid, axis_a);
    let second = bernstein_derivative_2d(&first, axis_b);
    second
        .iter()
        .map(|row| row.iter().map(|c| c * scale).collect())
        .collect()
}

/// A certified upper bound of `sup ‖C″‖` over `sub`: per piece, hull the twice
/// differentiated coefficient vectors (source units) over the region overlap
/// and take the certified sup of the norm over the coordinate enclosure; `L`
/// is the max over the pieces. A piece whose second-derivative coefficient
/// vectors are EXACTLY zero is flat and contributes zero (never a rounding
/// sliver); when every touched piece is flat the region has `L = 0`.
fn curve_curvature_upper_bound(
    map: &CertifiedCurveMap,
    sub: CurveRegion,
) -> Result<f64, ConstructRefusal> {
    let intervals = map.piece_intervals();
    let grids = map.piece_grids();
    let mut l = 0.0_f64;
    for (interval, coeffs) in intervals.iter().zip(grids.iter()) {
        let (t0, t1) = *interval;
        if sub.0 > t1 || sub.1 < t0 {
            continue;
        }
        let overlap = (sub.0.max(t0), sub.1.min(t1));
        let (u_lo, u_hi) = unit_image(*interval, overlap)?;
        let width = t1 - t0;
        if !width.is_finite() || width <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_width = 1.0 / width;
        let mut components = [Interval::point(0.0); 3];
        let mut flat_piece = true;
        for (k, vector) in coeffs.iter().enumerate() {
            let first = bernstein_derivative_1d(vector);
            let second: Vec<f64> = bernstein_derivative_1d(&first)
                .iter()
                .map(|c| c * inv_width * inv_width)
                .collect();
            if second.iter().any(|c| *c != 0.0) {
                flat_piece = false;
            }
            components[k] = hull_bernstein_1d(&second, (u_lo, u_hi))
                .map_err(|_| ConstructRefusal::InvalidInput)?;
        }
        if !flat_piece {
            l = l.max(norm_sup(&components)?);
        }
    }
    Ok(l)
}

/// The certified sup of the Euclidean norm over the coordinate enclosure of a
/// vector-valued field: `sqrt(Σ_k (max(|lo_k|, |hi_k|))²)`, every square, sum
/// and root rounded upward so the result certifies the norm's supremum.
fn norm_sup(components: &[Interval; 3]) -> Result<f64, ConstructRefusal> {
    let mut sum = 0.0_f64;
    for component in components {
        if !component.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let max_abs = component.lo.abs().max(component.hi.abs());
        let square = (max_abs * max_abs).next_up();
        if !square.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        sum = (sum + square).next_up();
        if !sum.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    let root = sum.sqrt();
    if !root.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(root.next_up())
}

/// The source-parameter overlap of `sub` with `patch_box`, per axis; `None`
/// when the rectangle is untouched.
fn rectangle_overlap(patch_box: SurfaceRegion, sub: SurfaceRegion) -> Option<SurfaceRegion> {
    let overlap_u = axis_overlap(patch_box.0, sub.0)?;
    let overlap_v = axis_overlap(patch_box.1, sub.1)?;
    Some((overlap_u, overlap_v))
}

/// The overlap of two closed intervals on one axis; `None` when disjoint.
fn axis_overlap(box_axis: (f64, f64), sub_axis: (f64, f64)) -> Option<(f64, f64)> {
    let (a0, a1) = box_axis;
    let (lo, hi) = sub_axis;
    if hi < a0 || lo > a1 {
        return None;
    }
    Some((lo.max(a0), hi.min(a1)))
}

/// The exact unit-parameter image of an overlap under the span's own
/// source-to-unit affine map, enclosed in `Interval` arithmetic and clamped to
/// `[0, 1]` (the `certified_map.rs` `unit_sub` discipline, re-derived here
/// because that helper is private to its module).
fn unit_image(span: (f64, f64), overlap: (f64, f64)) -> Result<(f64, f64), ConstructRefusal> {
    let (a, b) = span;
    let (lo, hi) = overlap;
    let a_iv = Interval::point(a);
    let span_iv = Interval::point(b).sub(&a_iv);
    let lo_u = Interval::point(lo)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let hi_u = Interval::point(hi)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    if u_lo.is_finite() && u_hi.is_finite() && u_lo <= u_hi {
        Ok((u_lo, u_hi))
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}
