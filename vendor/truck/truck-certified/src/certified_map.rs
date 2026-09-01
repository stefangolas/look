//! CertifiedMap (BG-CK-P1-MAP): class-1 admission of a compact rectangular
//! parameter domain, the enclosure oracle, and the rank margin (plan §2 class
//! 1; `docs/CERTIFIED_PHASE1_BOOKING.md` "BG-CK-P1-MAP").
//!
//! This module admits a B-spline curve or surface over its clamped knot range,
//! decomposed to Bézier pieces (D-map), and answers two certified queries over
//! any compact subbox of the declared domain: the **enclosure** (hull of the
//! value patches, via the landed `hull.rs` kernels) and the **rank margin**
//! (interval evaluation of the Jacobian minor against a declared τ).
//!
//! Pre-made decisions (packet tags; do not relitigate):
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. It carries no `unwrap`, no `expect`, and no `panic!`, and adds
//! no module-level `allow`: authored certified code, not moved baseline.
//!
//! **D1 — admission lives here.** No truck-geometry change; its manifest and
//! sources are read-only. The surface Bézier decomposition is built INSIDE
//! this module from the landed per-axis curve machinery; it is NOT
//! contributed back to truck-geometry.
//!
//! **D2 — one primitive, named compositions.** Every enclosure goes through
//! the landed `hull.rs` kernels (`hull_bernstein_1d`, `hull_bernstein_2d`,
//! `bernstein_derivative_1d`, `bernstein_derivative_2d`). The rank margin is
//! exactly two named compositions, pre-decided:
//!
//! - Curve rank margin: hulls of the three first-derivative coefficient
//!   vectors (per coordinate) give interval components `C'`; the certified
//!   lower bound of `|C'|²` is the sum over coordinates of `d_i²` where
//!   `d_i = 0` if the component's enclosure contains 0, else the distance
//!   from 0 to the nearer endpoint (each square and sum through
//!   `CertifiedInterval::mul`/`add` — outward-rounded).
//! - Surface rank margin: hulls of the six first-derivative patches
//!   (`Sᵤ`, `Sᵥ` per coordinate) give interval vectors; the interval cross
//!   product (three fixed coordinate expressions through
//!   `CertifiedInterval::mul`/`sub`) gives the interval normal `Sᵤ × Sᵥ`;
//!   its norm lower bound by the same component rule as above.
//!
//! The margin DECISION compares the certified lower bound against the
//! declared τ in `f64` — a certified bound against a declared threshold,
//! never a naked `f64` comparison of raw geometry (the F3 pattern).
//!
//! **D-tau — declared, never inferred.** τ arrives as `PositiveFinite`
//! (`formal/numeric.rs`) on every admission call. No default, no module
//! constant, no auto-tuning. A region whose certified margin lower bound is
//! ≤ τ refuses `ParameterizationDegenerate` — this covers both the truly
//! degenerate case and the cannot-decide case (the enclosure straddles τ).
//! The refusal is PER REGION: the caller's remedy is a smaller region (a new
//! admission attempt over a subbox), never a weakened τ and never a retry
//! with the same box. This is the honest discipline and matches F3's "never
//! retried with a weaker test".
//!
//! **D-map — the piece table is the module's spine.** The declared domain is
//! a B-spline's clamped knot range. Curve: the landed
//! `BSplineCurve::bezier_decomposition()` gives the pieces; each piece's
//! coefficient vectors and its subinterval `[t_i, t_{i+1}]` are recorded in
//! the map's piece table. Surface: truck-geometry has NO surface
//! decomposition (anchor A7), so this module builds one mechanically from
//! the landed curve machinery: decompose every row of control points along
//! `u` with `BSplineCurve::bezier_decomposition` (each row is a BSplineCurve
//! in the `u` parameter), then for each `u`-piece decompose every column
//! along `v` the same way. Tensor cut operations commute across axes, so the
//! result is exactly the Bézier patch grid; the conformance suite asserts the
//! patches' subboxes tile the declared domain exactly (adjacent shared
//! endpoints, no gaps) and that patch evaluation agrees with the surface's
//! own substitution beyond ulp noise. Rational (weighted) B-splines are OUT
//! OF SCOPE for this module: admission takes ordinary `BSplineCurve<Point3>`
//! / `BSplineSurface<Point3>`; the homogeneous path composes later per the
//! F2 rational rows when a consumer needs it.
//!
//! **D-region — queries are per-piece, combined conservatively.** A subbox
//! of the declared domain may span piece boundaries. The enclosure is the
//! component-wise union (min lower, max upper) of the per-piece hulls over
//! the pieces the subbox touches; the rank margin is the MINIMUM of the
//! per-piece margins over those pieces. Sound because the clamped pieces
//! cover the domain exactly. `EnclosureUnavailable` propagates from any
//! piece whose directed-rounded hull overflows (`HullRefusal::EnclosureUnavailable`
//! maps 1:1 onto `MapRefusal::EnclosureUnavailable`); `DomainNotCompact`
//! maps the same way and additionally fires for a region outside the declared
//! domain.

use crate::formal::exact::CertifiedInterval;
use crate::formal::numeric::PositiveFinite;
use crate::hull::{
    bernstein_derivative_1d, bernstein_derivative_2d, hull_bernstein_1d, hull_bernstein_2d,
    HullRefusal,
};
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, Point3};

/// A curve region: a compact subinterval of the declared domain.
pub type CurveRegion = (f64, f64);

/// A surface region: a compact rectangle of the declared domain.
pub type SurfaceRegion = ((f64, f64), (f64, f64));

/// Why a map query could not be certified.
///
/// Exactly three named cases (plan §2 class 1); no catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapRefusal {
    /// The certified rank margin is not above the declared tau on this
    /// region (covers both true degeneracy and cannot-decide). Remedy: a
    /// smaller region, never a weaker tau.
    ParameterizationDegenerate,
    /// A directed-rounded hull overflowed on this region.
    EnclosureUnavailable,
    /// The region is not a compact subset of the declared domain
    /// (non-finite, misordered, or outside bounds; inclusive edges).
    DomainNotCompact,
}

impl MapRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ParameterizationDegenerate => "map_parameterization_degenerate",
            Self::EnclosureUnavailable => "map_enclosure_unavailable",
            Self::DomainNotCompact => "map_domain_not_compact",
        }
    }
}

/// The admission answer for a region: the certified margin lower bound (a
/// [`CertifiedInterval`]) and the region box, with accessors only. `B` is the
/// region's box type — `(f64, f64)` for a curve, `((f64, f64), (f64, f64))`
/// for a surface.
#[derive(Debug, Clone)]
pub struct CertifiedRegionRank<B> {
    margin: CertifiedInterval,
    region: B,
}

impl<B> CertifiedRegionRank<B> {
    /// The certified margin lower bound of the region.
    pub fn margin(&self) -> CertifiedInterval {
        self.margin
    }

    /// The region the margin was certified over.
    pub fn region(&self) -> &B {
        &self.region
    }
}

/// One Bézier curve piece: its source subinterval `[t_i, t_{i+1}]` and the
/// per-coordinate Bernstein coefficient vectors (over the unit parameter, the
/// affine image of the subinterval).
#[derive(Debug, Clone)]
struct CurvePiece {
    interval: CurveRegion,
    coeffs: [Vec<f64>; 3],
}

/// One Bézier surface patch: its source subbox and the per-coordinate tensor
/// Bernstein coefficient grids (`grid[a][b]`, first axis `u`, second axis `v`).
#[derive(Debug, Clone)]
struct SurfacePatch {
    patch_box: SurfaceRegion,
    grid: [Vec<Vec<f64>>; 3],
}

/// A certified curve map `C: [t0, t1] -> R^3`, admitted over its declared
/// domain. Constructed only through [`admit_curve`].
#[derive(Debug, Clone)]
pub struct CertifiedCurveMap {
    /// The declared τ, held as a plain `f64` for the admission comparison
    /// (the `PositiveFinite` proof was made at the call boundary).
    tau: f64,
    /// The declared domain: the B-spline's clamped knot range.
    domain: CurveRegion,
    /// The piece table (D-map): subintervals + coefficient vectors.
    pieces: Vec<CurvePiece>,
}

/// A certified surface map `S: [u0, u1] x [v0, v1] -> R^3`, admitted over its
/// declared domain. Constructed only through [`admit_surface`].
#[derive(Debug, Clone)]
pub struct CertifiedSurfaceMap {
    /// The declared τ, held as a plain `f64` for the admission comparison.
    tau: f64,
    /// The declared domain: the clamped knot ranges of both axes.
    domain: SurfaceRegion,
    /// The patch table (D-map): subboxes + coefficient grids.
    patches: Vec<SurfacePatch>,
}

/// Admit a curve map over the B-spline's clamped knot range. Decomposes to
/// Bézier pieces (landed `bezier_decomposition`), then certifies the rank
/// margin over the WHOLE domain against `tau`. Refuses
/// `ParameterizationDegenerate` if the whole-domain margin is not above tau —
/// admit a sub-region for locally-degenerate maps.
pub fn admit_curve(
    curve: &BSplineCurve<Point3>,
    tau: PositiveFinite,
) -> Result<CertifiedCurveMap, MapRefusal> {
    let decomposed = curve.bezier_decomposition();
    if decomposed.is_empty() {
        return Err(MapRefusal::DomainNotCompact);
    }
    let first = &decomposed[0];
    let last = &decomposed[decomposed.len() - 1];
    let domain = (
        first.knot_vec()[0],
        last.knot_vec()[last.knot_vec().len() - 1],
    );
    let pieces = decomposed
        .iter()
        .map(|piece| {
            let knot_vec = piece.knot_vec();
            let interval = (knot_vec[0], knot_vec[knot_vec.len() - 1]);
            let points = piece.control_points();
            let coeffs = [
                points.iter().map(|pt| pt.x).collect(),
                points.iter().map(|pt| pt.y).collect(),
                points.iter().map(|pt| pt.z).collect(),
            ];
            CurvePiece { interval, coeffs }
        })
        .collect();
    let map = CertifiedCurveMap {
        tau: tau.get(),
        domain,
        pieces,
    };
    let whole = map.rank_margin(map.domain)?;
    if whole.lo <= map.tau {
        return Err(MapRefusal::ParameterizationDegenerate);
    }
    Ok(map)
}

/// Admit a curve map over a compact subinterval of an already-decomposed
/// domain (the per-region remedy). The map carries its piece table; the
/// region may span pieces (D-region).
pub fn admit_curve_region(
    map: &CertifiedCurveMap,
    sub: CurveRegion,
) -> Result<CertifiedRegionRank<CurveRegion>, MapRefusal> {
    let margin = map.rank_margin(sub)?;
    if margin.lo <= map.tau {
        return Err(MapRefusal::ParameterizationDegenerate);
    }
    Ok(CertifiedRegionRank {
        margin,
        region: sub,
    })
}

/// Admit a surface map over the B-spline's clamped knot ranges. Builds the
/// Bézier patch grid in-module (D-map), then certifies the rank margin over
/// the WHOLE domain against `tau`. Refuses `ParameterizationDegenerate` if
/// the whole-domain margin is not above tau.
pub fn admit_surface(
    surface: &BSplineSurface<Point3>,
    tau: PositiveFinite,
) -> Result<CertifiedSurfaceMap, MapRefusal> {
    let patches = build_surface_patches(surface)?;
    let first = &patches[0];
    let last = &patches[patches.len() - 1];
    let domain = (
        ((first.patch_box.0).0, (last.patch_box.0).1),
        ((first.patch_box.1).0, (last.patch_box.1).1),
    );
    let map = CertifiedSurfaceMap {
        tau: tau.get(),
        domain,
        patches,
    };
    let whole = map.rank_margin(map.domain)?;
    if whole.lo <= map.tau {
        return Err(MapRefusal::ParameterizationDegenerate);
    }
    Ok(map)
}

/// The surface per-region remedy, mirroring `admit_curve_region`.
pub fn admit_surface_region(
    map: &CertifiedSurfaceMap,
    sub: SurfaceRegion,
) -> Result<CertifiedRegionRank<SurfaceRegion>, MapRefusal> {
    let margin = map.rank_margin(sub)?;
    if margin.lo <= map.tau {
        return Err(MapRefusal::ParameterizationDegenerate);
    }
    Ok(CertifiedRegionRank {
        margin,
        region: sub,
    })
}

impl CertifiedCurveMap {
    /// Certified enclosure of `C(t)` over a compact subinterval: per-piece
    /// hulls of the value patches, combined conservatively (D-region).
    pub fn enclosure(&self, sub: CurveRegion) -> Result<[CertifiedInterval; 3], MapRefusal> {
        check_compact_1d(self.domain, sub)?;
        let mut acc = [CertifiedInterval {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        }; 3];
        for piece in &self.pieces {
            let (t0, t1) = piece.interval;
            if sub.0 > t1 || sub.1 < t0 {
                continue;
            }
            let overlap = (sub.0.max(t0), sub.1.min(t1));
            let hull = curve_value_hull(piece, overlap)?;
            for (cell, h) in acc.iter_mut().zip(hull.iter()) {
                cell.lo = cell.lo.min(h.lo);
                cell.hi = cell.hi.max(h.hi);
            }
        }
        if acc[0].lo.is_finite() && acc[0].hi.is_finite() {
            Ok(acc)
        } else {
            Err(MapRefusal::DomainNotCompact)
        }
    }

    /// Certified LOWER bound of `|C'(t)|` over the subinterval (D2 named
    /// composition). Above-tau certification is the caller's comparison.
    pub fn rank_margin(&self, sub: CurveRegion) -> Result<CertifiedInterval, MapRefusal> {
        check_compact_1d(self.domain, sub)?;
        let mut min_lb = f64::INFINITY;
        for piece in &self.pieces {
            let (t0, t1) = piece.interval;
            if sub.0 > t1 || sub.1 < t0 {
                continue;
            }
            let overlap = (sub.0.max(t0), sub.1.min(t1));
            let lb = curve_rank_margin_piece(piece, overlap)?;
            min_lb = min_lb.min(lb);
        }
        if min_lb.is_finite() {
            Ok(CertifiedInterval {
                lo: min_lb,
                hi: f64::INFINITY,
            })
        } else {
            Err(MapRefusal::EnclosureUnavailable)
        }
    }

    /// The piece subintervals of the map, in domain order (D-map structural
    /// accessor).
    pub fn piece_intervals(&self) -> Vec<(f64, f64)> {
        self.pieces.iter().map(|piece| piece.interval).collect()
    }
}

impl CertifiedSurfaceMap {
    /// Certified enclosure of `S(u, v)` over a compact rectangle.
    pub fn enclosure(&self, sub: SurfaceRegion) -> Result<[CertifiedInterval; 3], MapRefusal> {
        check_compact_2d(self.domain, sub)?;
        let mut acc = [CertifiedInterval {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        }; 3];
        for patch in &self.patches {
            let unit = patch_overlap_unit(patch.patch_box, sub)?;
            let Some((s, t)) = unit else {
                continue;
            };
            for (cell, grid_k) in acc.iter_mut().zip(patch.grid.iter()) {
                let hull = hull_bernstein_2d(grid_k, s, t).map_err(map_hull_refusal)?;
                cell.lo = cell.lo.min(hull.lo);
                cell.hi = cell.hi.max(hull.hi);
            }
        }
        if acc[0].lo.is_finite() && acc[0].hi.is_finite() {
            Ok(acc)
        } else {
            Err(MapRefusal::DomainNotCompact)
        }
    }

    /// Certified LOWER bound of `|Sᵤ × Sᵥ|` over the rectangle.
    pub fn rank_margin(&self, sub: SurfaceRegion) -> Result<CertifiedInterval, MapRefusal> {
        check_compact_2d(self.domain, sub)?;
        let mut min_lb = f64::INFINITY;
        for patch in &self.patches {
            let unit = patch_overlap_unit(patch.patch_box, sub)?;
            let Some((s, t)) = unit else {
                continue;
            };
            let lb = surface_rank_margin_patch(patch, s, t)?;
            min_lb = min_lb.min(lb);
        }
        if min_lb.is_finite() {
            Ok(CertifiedInterval {
                lo: min_lb,
                hi: f64::INFINITY,
            })
        } else {
            Err(MapRefusal::EnclosureUnavailable)
        }
    }

    /// The patch subboxes of the map (D-map structural accessor).
    pub fn patch_boxes(&self) -> Vec<SurfaceRegion> {
        self.patches.iter().map(|patch| patch.patch_box).collect()
    }

    /// The per-coordinate tensor coefficient grids of the patches, in the same
    /// order as [`Self::patch_boxes`] (D-map structural accessor; the
    /// conformance suite uses it to verify tensor commutation against the
    /// surface's own substitution).
    pub fn patch_grids(&self) -> Vec<[Vec<Vec<f64>>; 3]> {
        self.patches
            .iter()
            .map(|patch| patch.grid.clone())
            .collect()
    }
}

/// Whether `sub` is a compact subset of the 1-D domain, inclusive edges.
fn check_compact_1d(domain: CurveRegion, sub: (f64, f64)) -> Result<(), MapRefusal> {
    let (d0, d1) = domain;
    let (lo, hi) = sub;
    if lo.is_finite() && hi.is_finite() && d0 <= lo && lo <= hi && hi <= d1 {
        Ok(())
    } else {
        Err(MapRefusal::DomainNotCompact)
    }
}

/// Whether `sub` is a compact subset of the 2-D domain, inclusive edges.
fn check_compact_2d(domain: SurfaceRegion, sub: SurfaceRegion) -> Result<(), MapRefusal> {
    check_compact_1d(domain.0, sub.0)?;
    check_compact_1d(domain.1, sub.1)
}

/// The 1:1 refusal map from the landed hull kernel.
fn map_hull_refusal(refusal: HullRefusal) -> MapRefusal {
    match refusal {
        HullRefusal::EnclosureUnavailable => MapRefusal::EnclosureUnavailable,
        HullRefusal::DomainNotCompact => MapRefusal::DomainNotCompact,
    }
}

/// The exact unit-parameter image of `overlap` under the span's own
/// source-to-unit affine map, enclosed in `CertifiedInterval` arithmetic and
/// clamped to `[0, 1]` (the `hull.rs` `unit_subinterval` discipline, re-derived
/// here because that helper is private to `hull.rs` and the write set does not
/// include it).
fn unit_sub(interval: CurveRegion, overlap: (f64, f64)) -> Result<(f64, f64), MapRefusal> {
    let (a, b) = interval;
    let (lo, hi) = overlap;
    let a_iv = CertifiedInterval::point(a);
    let span_iv = CertifiedInterval::point(b).sub(&a_iv);
    let lo_u = CertifiedInterval::point(lo)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(MapRefusal::EnclosureUnavailable)?;
    let hi_u = CertifiedInterval::point(hi)
        .sub(&a_iv)
        .div(&span_iv)
        .ok_or(MapRefusal::EnclosureUnavailable)?;
    let u_lo = lo_u.lo.min(hi_u.lo).clamp(0.0, 1.0);
    let u_hi = lo_u.hi.max(hi_u.hi).clamp(0.0, 1.0);
    if u_lo.is_finite() && u_hi.is_finite() {
        Ok((u_lo, u_hi))
    } else {
        Err(MapRefusal::EnclosureUnavailable)
    }
}

/// The certified value-patch hull of a curve piece over `overlap` (a compact
/// subset of the piece's subinterval).
fn curve_value_hull(
    piece: &CurvePiece,
    overlap: CurveRegion,
) -> Result<[CertifiedInterval; 3], MapRefusal> {
    let (u_lo, u_hi) = unit_sub(piece.interval, overlap)?;
    let mut out = [CertifiedInterval::point(0.0); 3];
    for (cell, coeffs) in out.iter_mut().zip(piece.coeffs.iter()) {
        *cell = hull_bernstein_1d(coeffs, (u_lo, u_hi)).map_err(map_hull_refusal)?;
    }
    Ok(out)
}

/// The certified LOWER bound of `|C'(t)|` over `overlap`, for one piece.
///
/// The first-derivative coefficients are formed in the SOURCE parameter: the
/// raw `bernstein_derivative_1d` output (derivative w.r.t. the unit parameter)
/// is scaled by the inverse piece width, a pure `f64` coefficient transform —
/// the derivative polynomial is then the derivative w.r.t. `t` (D2).
fn curve_rank_margin_piece(piece: &CurvePiece, overlap: CurveRegion) -> Result<f64, MapRefusal> {
    let (u_lo, u_hi) = unit_sub(piece.interval, overlap)?;
    let (t0, t1) = piece.interval;
    let width = t1 - t0;
    if !width.is_finite() || width <= 0.0 {
        return Err(MapRefusal::EnclosureUnavailable);
    }
    let inv_width = 1.0 / width;
    let mut lb = CertifiedInterval::point(0.0);
    for k in 0..3 {
        let derivative: Vec<f64> = bernstein_derivative_1d(&piece.coeffs[k])
            .iter()
            .map(|c| c * inv_width)
            .collect();
        let hull = hull_bernstein_1d(&derivative, (u_lo, u_hi)).map_err(map_hull_refusal)?;
        let component = component_lower_bound(&hull);
        let squared = CertifiedInterval::point(component).mul(&CertifiedInterval::point(component));
        lb = lb.add(&squared);
    }
    // `lb.lo` is a certified lower bound of `|C'|²`, which is always >= 0, so a
    // directed-rounded negative sliver (e.g. `next_down(0)`) may be replaced by
    // zero before the (monotone) square root.
    let lo = lb.lo.max(0.0).sqrt().next_down();
    if lo.is_finite() {
        Ok(lo)
    } else {
        Err(MapRefusal::EnclosureUnavailable)
    }
}

/// The certified LOWER bound of `|Sᵤ × Sᵥ|` over the unit subbox `(s, t)`, for
/// one patch. The derivative patches are formed in the SOURCE parameters
/// (per-axis inverse-width scaling of the raw `bernstein_derivative_2d` grids,
/// a pure `f64` coefficient transform); the interval normal is the three fixed
/// cross-product coordinate expressions through `CertifiedInterval::mul`/`sub`;
/// the norm lower bound is the same component rule as the curve case (D2).
fn surface_rank_margin_patch(
    patch: &SurfacePatch,
    s: (f64, f64),
    t: (f64, f64),
) -> Result<f64, MapRefusal> {
    let ((u0, u1), (v0, v1)) = patch.patch_box;
    let width_u = u1 - u0;
    let width_v = v1 - v0;
    if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
        return Err(MapRefusal::EnclosureUnavailable);
    }
    let inv_u = 1.0 / width_u;
    let inv_v = 1.0 / width_v;
    let mut su = [CertifiedInterval::point(0.0); 3];
    let mut sv = [CertifiedInterval::point(0.0); 3];
    for k in 0..3 {
        let du: Vec<Vec<f64>> = bernstein_derivative_2d(&patch.grid[k], 0)
            .iter()
            .map(|row| row.iter().map(|c| c * inv_u).collect())
            .collect();
        let dv: Vec<Vec<f64>> = bernstein_derivative_2d(&patch.grid[k], 1)
            .iter()
            .map(|row| row.iter().map(|c| c * inv_v).collect())
            .collect();
        su[k] = hull_bernstein_2d(&du, s, t).map_err(map_hull_refusal)?;
        sv[k] = hull_bernstein_2d(&dv, s, t).map_err(map_hull_refusal)?;
    }
    let normal0 = su[1].mul(&sv[2]).sub(&su[2].mul(&sv[1]));
    let normal1 = su[2].mul(&sv[0]).sub(&su[0].mul(&sv[2]));
    let normal2 = su[0].mul(&sv[1]).sub(&su[1].mul(&sv[0]));
    let components = [
        component_lower_bound(&normal0),
        component_lower_bound(&normal1),
        component_lower_bound(&normal2),
    ];
    let mut lb = CertifiedInterval::point(0.0);
    for component in components {
        let squared = CertifiedInterval::point(component).mul(&CertifiedInterval::point(component));
        lb = lb.add(&squared);
    }
    // Same nonnegativity clamp as the curve case: `lb.lo` is a certified lower
    // bound of `|Sᵤ × Sᵥ|² >= 0`, so a directed-rounded negative sliver may be
    // zeroed before the (monotone) square root.
    let lo = lb.lo.max(0.0).sqrt().next_down();
    if lo.is_finite() {
        Ok(lo)
    } else {
        Err(MapRefusal::EnclosureUnavailable)
    }
}

/// The certified lower bound of `|x|` over an enclosure: zero when the
/// enclosure contains zero, else the distance from zero to the nearer endpoint
/// (the D2 component rule).
fn component_lower_bound(interval: &CertifiedInterval) -> f64 {
    let (lo, hi) = (interval.lo, interval.hi);
    if lo <= 0.0 && hi >= 0.0 {
        0.0
    } else {
        lo.abs().min(hi.abs())
    }
}

/// The unit subbox of a patch the query rectangle touches; `None` when the
/// patch is untouched.
fn patch_overlap_unit(
    patch_box: SurfaceRegion,
    sub: SurfaceRegion,
) -> Result<Option<SurfaceRegion>, MapRefusal> {
    let ((u0, u1), (v0, v1)) = patch_box;
    let ((s_lo, s_hi), (t_lo, t_hi)) = sub;
    if s_lo > u1 || s_hi < u0 || t_lo > v1 || t_hi < v0 {
        return Ok(None);
    }
    let overlap_u = (s_lo.max(u0), s_hi.min(u1));
    let overlap_v = (t_lo.max(v0), t_hi.min(v1));
    let s = unit_sub((u0, u1), overlap_u)?;
    let t = unit_sub((v0, v1), overlap_v)?;
    Ok(Some((s, t)))
}

/// Builds the Bézier patch grid of a surface (D-map): every row of control
/// points is decomposed along `u`, then every column of each `u`-piece along
/// `v`. Tensor cut operations commute across axes, so the result is the exact
/// Bézier patch grid; the conformance suite verifies the tiling and the
/// evaluation agreement with the surface's own substitution.
fn build_surface_patches(
    surface: &BSplineSurface<Point3>,
) -> Result<Vec<SurfacePatch>, MapRefusal> {
    let ctrl = surface.control_points();
    if ctrl.is_empty() || ctrl[0].is_empty() {
        return Err(MapRefusal::DomainNotCompact);
    }
    let (u_knots, _) = surface.uknot_vec().to_single_multi();
    let (v_knots, _) = surface.vknot_vec().to_single_multi();
    let nu_pieces = u_knots
        .len()
        .checked_sub(1)
        .ok_or(MapRefusal::DomainNotCompact)?;
    let nv_pieces = v_knots
        .len()
        .checked_sub(1)
        .ok_or(MapRefusal::DomainNotCompact)?;
    if nu_pieces == 0 || nv_pieces == 0 {
        return Err(MapRefusal::DomainNotCompact);
    }
    let nu_total = ctrl.len();
    let nv_total = ctrl[0].len();
    let udegree = surface.udegree();

    let mut rows_u: Vec<Vec<BSplineCurve<Point3>>> = Vec::with_capacity(nv_total);
    for j in 0..nv_total {
        let points: Vec<Point3> = (0..nu_total)
            .map(|i| *surface.control_point(i, j))
            .collect();
        let row = BSplineCurve::new_unchecked(surface.uknot_vec().clone(), points);
        rows_u.push(row.bezier_decomposition());
    }
    if rows_u.iter().any(|row| row.len() != nu_pieces) {
        return Err(MapRefusal::EnclosureUnavailable);
    }

    let mut patches: Vec<SurfacePatch> = Vec::with_capacity(nu_pieces * nv_pieces);
    for iu in 0..nu_pieces {
        let mut col_pieces: Vec<Vec<BSplineCurve<Point3>>> = Vec::with_capacity(udegree + 1);
        for a in 0..=udegree {
            let points: Vec<Point3> = (0..nv_total)
                .map(|j| rows_u[j][iu].control_points()[a])
                .collect();
            let column = BSplineCurve::new_unchecked(surface.vknot_vec().clone(), points);
            col_pieces.push(column.bezier_decomposition());
        }
        if col_pieces.iter().any(|column| column.len() != nv_pieces) {
            return Err(MapRefusal::EnclosureUnavailable);
        }
        for iv in 0..nv_pieces {
            let mut grid: [Vec<Vec<f64>>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for (k, coordinate) in grid.iter_mut().enumerate() {
                for col_piece in &col_pieces {
                    coordinate.push(
                        col_piece[iv]
                            .control_points()
                            .iter()
                            .map(|point| match k {
                                0 => point.x,
                                1 => point.y,
                                _ => point.z,
                            })
                            .collect(),
                    );
                }
            }
            patches.push(SurfacePatch {
                patch_box: (
                    (u_knots[iu], u_knots[iu + 1]),
                    (v_knots[iv], v_knots[iv + 1]),
                ),
                grid,
            });
        }
    }
    Ok(patches)
}
