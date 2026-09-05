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

//! CC-025-CANAL (spine seam S10): the canal-surface exact regularity seam over
//! a certified spine (`docs/CERTIFIED_LOFT_AND_SHELL_THEORY_SPEC.md` §6).
//!
//! The edge strata of a rounded offset and the surfaces of all rolling-ball
//! blends are canal surfaces: the envelope of the spheres of radius `r(s)`
//! centred on a spine `c(s)`. For a unit-speed `C²` spine the envelope is
//! parameterized by the tilted characteristic circle
//! `X(s, θ) = c(s) − r(s)p(s)T(s) + r(s)a(s)e_θ(s)`, with `p = r′`, `q = r″`
//! and `a = √(1 − p²)`, and the exact regularity of the map is the closed form
//! (theory §6.3, torsion cancels — no Frenet frame and no torsion term appear):
//!
//! ```text
//! ‖X_s × X_θ‖ = r · |a² − rq − ra (c″ · e_θ)|
//! ```
//!
//! so `X` is regular at `s` for all `θ` iff `|a² − rq| > ra‖c″‖`, and over a
//! whole spine arc the certified criterion value is
//! `min over the arc of |a² − rq| − max over the arc of r·a·‖c″‖` with a
//! FIXED min/max accumulation order over the arc's Bézier pieces. The first
//! gate (§6.1) is immediate: a law whose `|r′|` enclosure reaches `1`
//! anywhere degenerates the characteristic circle (`a = 0`) and refuses
//! [`ConstructRefusal::CanalSingular`]. The only refusal of this module is
//! `CanalSingular` (on a degenerate profile, a degenerate characteristic
//! circle, or a criterion value that does not certify strictly positive);
//! invalid *requests* (non-compact regions, non-finite data) refuse
//! `ConstructRefusal::InvalidInput`.
//!
//! **Radius-law coordinate.** [`RadiusLaw`] profiles are functions over the
//! CALLER'S ARC normalized to the unit parameter `u ∈ [0, 1]` (`u = 0` at the
//! arc start, `u = 1` at the arc end): `Constant(c)` is the constant `c`,
//! `Linear { r0, r1 }` interpolates linearly from `r0` to `r1` (constant slope
//! `r1 − r0` over the declared arc), `CubicHermite` / `MonotoneCubic` are
//! evaluated by cubic Hermite interpolation on the unit sub-interval, and
//! `VertexControl` is the monotone cubic through the control radii at the
//! uniformly spaced control-vertex stations. The evaluators' slope gate is
//! expressed in that normalized coordinate. On a unit-length spine arc (the
//! canonical canal-model arc, `L = 1`) the normalized coordinate is exactly
//! the arc-length coordinate of theory §6.1, and the composition below is then
//! the packet's Section 2 prescription verbatim: `r`, `r′`, `r″` from the law
//! evaluators and `‖c″‖` from the map's Bernstein 1-D derivative hulls
//! (`hull.rs`, the CC-002 discipline — no second hull path is written).
//!
//! **Arc restriction.** [`canal_regularity`] certifies a patch over the
//! requested sub-arc of the spine (the form CC-021 offset edge strata and
//! CC-030/031 blend spines call). [`canal_regularity_closed_pipe`] is the
//! same body over the whole map domain — the correct all-θ criterion for a
//! closed pipe, which spans the entire spine loop — with the identical body
//! minus the arc restriction. The permissiveness gap is observable: a spine
//! whose pipe condition `r·‖c″‖ < 1` holds only on a sub-arc certifies on that
//! arc but refuses as a closed pipe.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. It carries no `unwrap`, no `expect`, and no `panic!`, and adds
//! no module-level `allow`. Every float reduction runs in a fixed order with
//! directed rounding (C9).

use crate::certified_map::CertifiedCurveMap;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::RadiusLaw;
use crate::construct::Interval;
use crate::hull::{bernstein_derivative_1d, hull_bernstein_1d};

/// A certified radius-law evaluation over an arc sub-interval.
///
/// Evaluates the radius profile of `law` over the compact sub-interval `s` of
/// the caller's normalized arc (`0 <= s.lo <= s.hi <= 1`) and returns a
/// certified enclosure of the radius over that interval. Applies the §6.1
/// first gate: a profile whose `|r′|` enclosure reaches `1` anywhere in `s`
/// (the characteristic circle degenerates) refuses
/// [`ConstructRefusal::CanalSingular`] immediately.
pub fn radius_eval(law: &RadiusLaw, s: Interval) -> Result<Interval, ConstructRefusal> {
    let (r, _, _) = radius_jet(law, s)?;
    Ok(r)
}

/// A certified radius-law evaluation and slope over an arc sub-interval.
///
/// Returns the pair `(r, r′)` — the radius enclosure and the first-derivative
/// enclosure of the profile over the compact sub-interval `s` of the caller's
/// normalized arc — for the `Constant(c) → (c, 0)` CC-000 semantics. Applies
/// the same §6.1 slope gate as [`radius_eval`]; the second derivative `r″`
/// (where the law carries it) is composed inside the regularity criterion by
/// the seam, so this seam's evaluator pair stays the declared
/// `(value, slope)` shape.
pub fn radius_derivs(
    law: &RadiusLaw,
    s: Interval,
) -> Result<(Interval, Interval), ConstructRefusal> {
    let (r, rp, _) = radius_jet(law, s)?;
    Ok((r, rp))
}

/// The S10 arc-restricted canal regularity criterion (theory §6.3).
///
/// Over the arc's Bézier pieces (the map's landed decomposition) this bounds
/// `r`, `r′`, `r″` from the law evaluators and `‖c″‖` from the Bernstein 1-D
/// derivative hulls exactly as CC-002 bounds second derivatives, composes
/// `a = √(1 − r′²)` (refusing when the radicand's enclosure is not strictly
/// positive), and computes the certified arc value
/// `min over the arc of |a² − rq| − max over the arc of r·a·‖c″‖` with a
/// fixed accumulation order over pieces. When the enclosure's lower endpoint
/// is strictly positive the patch is regular over the arc and the enclosure is
/// returned; an enclosure straddling zero or lying at or below zero refuses
/// [`ConstructRefusal::CanalSingular`]. A non-positive radius profile over the
/// arc also refuses `CanalSingular` (the sphere envelope degenerates).
pub fn canal_regularity(
    spine: &CertifiedCurveMap,
    radius: &RadiusLaw,
    arc: (f64, f64),
) -> Result<Interval, ConstructRefusal> {
    let intervals = spine.piece_intervals();
    if intervals.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let domain = (intervals[0].0, intervals[intervals.len() - 1].1);
    canal_criterion(spine, radius, arc, domain)
}

/// The all-θ closed-pipe canal regularity criterion (theory §6.3).
///
/// The identical body of [`canal_regularity`] minus the arc restriction: a
/// closed pipe spans the map's entire spine loop, so the criterion is
/// evaluated over the whole map domain (the arc argument has no meaning for a
/// closed pipe and is dropped). A pipe that is only regular over a sub-arc of
/// its loop therefore refuses here while the arc-restricted form certifies —
/// the permissiveness gap is observed, not assumed.
pub fn canal_regularity_closed_pipe(
    spine: &CertifiedCurveMap,
    radius: &RadiusLaw,
) -> Result<Interval, ConstructRefusal> {
    let intervals = spine.piece_intervals();
    if intervals.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let domain = (intervals[0].0, intervals[intervals.len() - 1].1);
    canal_criterion(spine, radius, domain, domain)
}

/// One Bernstein segment of a radius profile: the profile piece over
/// `[start, end]`, with `value` the Bézier coefficients (in the segment-local
/// parameter) of the radius over that piece.
#[derive(Debug, Clone)]
struct Segment {
    /// The segment's left endpoint in the normalized arc coordinate.
    start: f64,
    /// The segment's right endpoint in the normalized arc coordinate.
    end: f64,
    /// The radius Bézier coefficients over the segment-local parameter.
    value: Vec<f64>,
}

/// The certified radius jet `(r, r′, r″)` of a law over a compact sub-interval
/// of the normalized arc, with the §6.1 slope gate applied.
///
/// `s` must be a compact sub-interval of `[0, 1]`; anything else is an invalid
/// request. Every arithmetic step is outward-rounded through the landed
/// `hull::hull_bernstein_1d` kernels over the piecewise Bernstein profile.
fn radius_jet(
    law: &RadiusLaw,
    s: Interval,
) -> Result<(Interval, Interval, Interval), ConstructRefusal> {
    if !s.is_finite() || s.lo < 0.0 || s.hi > 1.0 || s.lo > s.hi {
        return Err(ConstructRefusal::InvalidInput);
    }
    let segments = law_segments(law)?;
    let mut r_lo = f64::INFINITY;
    let mut r_hi = f64::NEG_INFINITY;
    let mut rp_lo = f64::INFINITY;
    let mut rp_hi = f64::NEG_INFINITY;
    let mut rpp_lo = f64::INFINITY;
    let mut rpp_hi = f64::NEG_INFINITY;
    for segment in &segments {
        if segment.end < s.lo || segment.start > s.hi {
            continue;
        }
        let overlap_lo = segment.start.max(s.lo);
        let overlap_hi = segment.end.min(s.hi);
        if overlap_hi < overlap_lo {
            continue;
        }
        let width = segment.end - segment.start;
        if !width.is_finite() || width <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        let inv_width = 1.0 / width;
        let t_lo = ((overlap_lo - segment.start) * inv_width).clamp(0.0, 1.0);
        let t_hi = ((overlap_hi - segment.start) * inv_width).clamp(0.0, 1.0);
        let value = hull_bernstein_1d(&segment.value, (t_lo, t_hi))
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        let first: Vec<f64> = bernstein_derivative_1d(&segment.value)
            .iter()
            .map(|c| c * inv_width)
            .collect();
        let first_hull =
            hull_bernstein_1d(&first, (t_lo, t_hi)).map_err(|_| ConstructRefusal::InvalidInput)?;
        let second: Vec<f64> = bernstein_derivative_1d(&first)
            .iter()
            .map(|c| c * inv_width)
            .collect();
        let second_hull =
            hull_bernstein_1d(&second, (t_lo, t_hi)).map_err(|_| ConstructRefusal::InvalidInput)?;
        r_lo = r_lo.min(value.lo);
        r_hi = r_hi.max(value.hi);
        rp_lo = rp_lo.min(first_hull.lo);
        rp_hi = rp_hi.max(first_hull.hi);
        rpp_lo = rpp_lo.min(second_hull.lo);
        rpp_hi = rpp_hi.max(second_hull.hi);
    }
    if !r_lo.is_finite() || !r_hi.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let r = Interval { lo: r_lo, hi: r_hi };
    let rp = Interval {
        lo: rp_lo,
        hi: rp_hi,
    };
    let rpp = Interval {
        lo: rpp_lo,
        hi: rpp_hi,
    };
    if !rp.is_finite() || !rpp.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    if rp.lo <= -1.0 || rp.hi >= 1.0 {
        return Err(ConstructRefusal::CanalSingular);
    }
    Ok((r, rp, rpp))
}

/// The piecewise-Bernstein profile of a radius law over the normalized arc
/// `[0, 1]`. Every produced segment has a strictly positive width and the
/// pieces tile `[0, 1]` (the monotone cubics require station coverage from `0`
/// to `1`).
fn law_segments(law: &RadiusLaw) -> Result<Vec<Segment>, ConstructRefusal> {
    match law {
        RadiusLaw::Constant(c) => {
            if !c.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            Ok(vec![Segment {
                start: 0.0,
                end: 1.0,
                value: vec![*c],
            }])
        }
        RadiusLaw::Linear { r0, r1 } => {
            if !r0.is_finite() || !r1.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            Ok(vec![Segment {
                start: 0.0,
                end: 1.0,
                value: vec![*r0, *r1],
            }])
        }
        RadiusLaw::CubicHermite { r0, r1, m0, m1 } => {
            if !r0.is_finite() || !r1.is_finite() || !m0.is_finite() || !m1.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            let p1 = r0 + m0 / 3.0;
            let p2 = r1 - m1 / 3.0;
            if !p1.is_finite() || !p2.is_finite() {
                return Err(ConstructRefusal::InvalidInput);
            }
            Ok(vec![Segment {
                start: 0.0,
                end: 1.0,
                value: vec![*r0, p1, p2, *r1],
            }])
        }
        RadiusLaw::MonotoneCubic(points) => monotone_cubic_segments(points),
        RadiusLaw::VertexControl(radii) => {
            if radii.len() < 2 {
                return Err(ConstructRefusal::InvalidInput);
            }
            if radii.iter().any(|r| !r.is_finite()) {
                return Err(ConstructRefusal::InvalidInput);
            }
            let count = radii.len();
            let points: Vec<(f64, f64)> = radii
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let station = (i as f64) / ((count - 1) as f64);
                    (station, *r)
                })
                .collect();
            monotone_cubic_segments(&points)
        }
    }
}

/// Build the shape-preserving monotone cubic segments through `points` (the
/// Fritsch–Carlson slope selection).
///
/// The points are sorted by station and must be strictly increasing finite
/// stations that cover the unit arc (`0.0` to `1.0`); any degenerate,
/// non-covering, or non-finite input is invalid. Segment endpoint slopes are
/// the one-sided three-point estimates clamped to the segment monotonicity
/// range; interior slopes are the Fritsch–Carlson weighted harmonic mean
/// (zero at sign changes), so a monotone data set yields a monotone radius
/// profile.
fn monotone_cubic_segments(points: &[(f64, f64)]) -> Result<Vec<Segment>, ConstructRefusal> {
    let mut pts = points.to_vec();
    if pts.len() < 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    if pts.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err(ConstructRefusal::InvalidInput);
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    if pts[0].0 != 0.0 || pts[pts.len() - 1].0 != 1.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let count = pts.len();
    let mut heights = vec![0.0_f64; count - 1];
    let mut chords = vec![0.0_f64; count - 1];
    for i in 0..count - 1 {
        let height = pts[i + 1].0 - pts[i].0;
        if !height.is_finite() || height <= 0.0 {
            return Err(ConstructRefusal::InvalidInput);
        }
        heights[i] = height;
        chords[i] = (pts[i + 1].1 - pts[i].1) / height;
    }
    let mut slopes = vec![0.0_f64; count];
    if count == 2 {
        slopes[0] = chords[0];
        slopes[1] = chords[0];
    } else {
        for i in 1..count - 1 {
            slopes[i] = if chords[i - 1] * chords[i] <= 0.0 {
                0.0
            } else {
                let w1 = 2.0 * heights[i] + heights[i - 1];
                let w2 = heights[i] + 2.0 * heights[i - 1];
                (w1 + w2) / (w1 / chords[i - 1] + w2 / chords[i])
            };
        }
        slopes[0] = monotone_endpoint_slope(chords[0], chords[1], heights[0], heights[1]);
        slopes[count - 1] = monotone_endpoint_slope(
            chords[count - 2],
            chords[count - 3],
            heights[count - 2],
            heights[count - 3],
        );
    }
    let mut segments = Vec::with_capacity(count - 1);
    for i in 0..count - 1 {
        let height = heights[i];
        let p1 = pts[i].1 + slopes[i] * height / 3.0;
        let p2 = pts[i + 1].1 - slopes[i + 1] * height / 3.0;
        if !p1.is_finite() || !p2.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        segments.push(Segment {
            start: pts[i].0,
            end: pts[i + 1].0,
            value: vec![pts[i].1, p1, p2, pts[i + 1].1],
        });
    }
    Ok(segments)
}

/// The Fritsch–Carlson endpoint slope: the three-point estimate through the
/// adjacent and next chord, clamped to the monotonicity range of the adjacent
/// segment (`[0, 3·d_adj]` for a rising chord, its mirror for a falling one).
fn monotone_endpoint_slope(d_adj: f64, d_prev: f64, h_adj: f64, h_prev: f64) -> f64 {
    let denom = h_adj + h_prev;
    if denom == 0.0 {
        return 0.0;
    }
    let raw = ((2.0 * h_adj + h_prev) * d_adj - h_adj * d_prev) / denom;
    if d_adj > 0.0 {
        raw.clamp(0.0, 3.0 * d_adj)
    } else if d_adj < 0.0 {
        raw.clamp(3.0 * d_adj, 0.0)
    } else {
        0.0
    }
}

/// The shared S10 criterion body over `arc` (a compact subset of `domain`, the
/// map's landed domain; for the closed-pipe form the two coincide).
fn canal_criterion(
    spine: &CertifiedCurveMap,
    radius: &RadiusLaw,
    arc: (f64, f64),
    domain: (f64, f64),
) -> Result<Interval, ConstructRefusal> {
    if !arc.0.is_finite() || !arc.1.is_finite() || arc.0 < domain.0 || arc.1 > domain.1 {
        return Err(ConstructRefusal::InvalidInput);
    }
    if !domain.0.is_finite() || !domain.1.is_finite() || domain.0 >= domain.1 {
        return Err(ConstructRefusal::InvalidInput);
    }
    if arc.0 >= arc.1 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let intervals = spine.piece_intervals();
    let grids = spine.piece_grids();
    let mut min_abs_lo = f64::INFINITY;
    let mut min_abs_hi = f64::INFINITY;
    let mut max_term = 0.0_f64;
    for (interval, grid) in intervals.iter().zip(grids.iter()) {
        let (t0, t1) = *interval;
        if arc.0 > t1 || arc.1 < t0 {
            continue;
        }
        let overlap = (arc.0.max(t0), arc.1.min(t1));
        if overlap.0 >= overlap.1 {
            continue;
        }
        let (u_lo, u_hi) = unit_image(*interval, overlap)?;
        let (tau_lo, tau_hi) = unit_image(arc, overlap)?;
        let (r, rp, rpp) = radius_jet(
            radius,
            Interval {
                lo: tau_lo,
                hi: tau_hi,
            },
        )?;
        // The radius gate refuses exactly the negated greater-than test on
        // `r.lo` against zero: a non-positive or NaN lower endpoint refuses. A
        // bare `r.lo <= 0.0` would silently admit a NaN endpoint, so the
        // negated comparison is retained verbatim below.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        // the radius positivity gate's negated-greater-than semantics are the contract: NaN and non-positive lower endpoints must refuse
        if !(r.lo > 0.0) {
            return Err(ConstructRefusal::CanalSingular);
        }
        let rp2 = rp.mul(&rp);
        let a2 = Interval::point(1.0).sub(&rp2);
        // `a2 = 1 − rp²` must be strictly positive; spelled without the
        // negation as `<= 0 || NaN`, which is exactly the negated-greater-than
        // semantics of the radius gate above.
        if a2.lo <= 0.0 || a2.lo.is_nan() {
            return Err(ConstructRefusal::CanalSingular);
        }
        let a = a2.sqrt().ok_or(ConstructRefusal::CanalSingular)?;
        let rq = r.mul(&rpp);
        let z = a2.sub(&rq);
        if !z.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let abs_lo = abs_lower(z);
        let abs_hi = abs_upper(z);
        if !abs_lo.is_finite() || !abs_hi.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        min_abs_lo = min_abs_lo.min(abs_lo);
        min_abs_hi = min_abs_hi.min(abs_hi);
        let ra = r.mul(&a);
        if !ra.is_finite() || !ra.hi.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let curvature = piece_second_derivative_sup(grid, *interval, (u_lo, u_hi))?;
        let term = (ra.hi * curvature).next_up();
        if !term.is_finite() {
            return Err(ConstructRefusal::InvalidInput);
        }
        max_term = max_term.max(term);
    }
    if min_abs_lo.is_infinite() || min_abs_hi.is_infinite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let lo = (min_abs_lo - max_term).next_down();
    let hi = min_abs_hi;
    if !lo.is_finite() || !hi.is_finite() || lo > hi {
        return Err(ConstructRefusal::InvalidInput);
    }
    let enclosure = Interval { lo, hi };
    if lo > 0.0 {
        Ok(enclosure)
    } else {
        Err(ConstructRefusal::CanalSingular)
    }
}

/// A certified lower bound of `min |x|` over an enclosure: `0` when the
/// enclosure contains zero, else the nearer endpoint's magnitude.
fn abs_lower(v: Interval) -> f64 {
    if v.lo > 0.0 {
        v.lo
    } else if v.hi < 0.0 {
        -v.hi
    } else {
        0.0
    }
}

/// A certified upper bound of `max |x|` over an enclosure.
fn abs_upper(v: Interval) -> f64 {
    if v.lo > 0.0 {
        v.hi
    } else if v.hi < 0.0 {
        -v.lo
    } else {
        (-v.lo).max(v.hi)
    }
}

/// A certified upper bound of `sup ‖c″‖` over the piece-overlap: per
/// coordinate, the twice-differentiated Bernstein coefficient vector in SOURCE
/// units (unit-parameter second derivative scaled by the inverse piece width
/// squared), hulled over the unit sub-image of the overlap, then the certified
/// sup of the Euclidean norm over the coordinate enclosure. A piece whose
/// second-derivative coefficients are EXACTLY zero is flat and contributes
/// zero (the CC-002 flatness discipline — rounding slivers are never
/// curvature).
fn piece_second_derivative_sup(
    coeffs: &[Vec<f64>; 3],
    interval: (f64, f64),
    u: (f64, f64),
) -> Result<f64, ConstructRefusal> {
    let (t0, t1) = interval;
    let width = t1 - t0;
    if !width.is_finite() || width <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv_width = 1.0 / width;
    let inv_width2 = inv_width * inv_width;
    let mut components = [Interval::point(0.0); 3];
    let mut flat = true;
    for (k, vector) in coeffs.iter().enumerate() {
        let first = bernstein_derivative_1d(vector);
        let second: Vec<f64> = bernstein_derivative_1d(&first)
            .iter()
            .map(|c| c * inv_width2)
            .collect();
        if second.iter().any(|c| *c != 0.0) {
            flat = false;
        }
        components[k] =
            hull_bernstein_1d(&second, u).map_err(|_| ConstructRefusal::InvalidInput)?;
    }
    if flat {
        Ok(0.0)
    } else {
        norm_sup(&components)
    }
}

/// The certified sup of the Euclidean norm over the coordinate enclosure of a
/// vector-valued field: `√(Σ_k (max(|lo_k|, |hi_k|))²)`, every square, sum and
/// root rounded upward so the result certifies the norm's supremum.
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

/// The exact unit-parameter image of an overlap under the span's own
/// source-to-unit affine map, enclosed in `Interval` arithmetic and clamped to
/// `[0, 1]` (the `certified_map.rs` / `injectivity.rs` discipline, re-derived
/// because the helper is private to those modules).
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
