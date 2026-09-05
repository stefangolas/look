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

//! The loft validity certificate (CC-014-LOFT-VALIDITY, theory §2.2 L5, spine
//! S4/S6/S7 consumers).
//!
//! Once correspondence is an orientation-preserving combinatorial
//! homeomorphism, a loft can fail geometrically in exactly two ways:
//! **regularity loss** (`Sᵤ × Sᵥ = 0`) or **self-contact** (`S(p) = S(q)`,
//! `p ≠ q`). There is no separate pinch or twist theory. The postcondition is
//! THREE-VALUED per the CG verdict doctrine — certified / failed / inconclusive
//! is surfaced, never converted into success.
//!
//! This packet COMPOSES landed certificates. It implements no new contact
//! solver, no new hull kernel, and no new SSI: the regularity arm composes
//! [`rank_margin`](crate::certified_map::CertifiedSurfaceMap::rank_margin),
//! the near-diagonal discharge composes
//! [`injectivity_radius`](crate::construct::injectivity::injectivity_radius),
//! the whole-region discharge composes
//! [`certify_graph_disk`](crate::construct::graphdisk::certify_graph_disk),
//! and the far-pair funnel composes the landed evidence
//! [`contact`](truck_evidence::contact::contact) through the CC-000 manifest
//! edge, with the inari/certified boundary at
//! [`convert`](crate::construct::convert) (the ONLY sanctioned bridge).
//!
//! # Section 1 — the certificate
//!
//! [`certify_loft_validity`] returns `Ok` ALWAYS when the certificate was
//! PRODUCED; whether the loft is valid is read OFF the certificate. A loft
//! that fails validity is a valid certificate about an invalid loft. The
//! verdict data:
//!
//! - [`LoftValidityCert::regularity`] carries the certified regularity margin
//!   ENCLOSURE (the minimum certified `|Sᵤ × Sᵥ|` lower bound over the strip
//!   regions) whenever every region's margin certified. The regularity
//!   postcondition — the CC-000 margin floor [`CC_ETA_J`] — is the caller's
//!   threshold read (`margin.lo ≥ CC_ETA_J`): a loft whose certified margin
//!   lies below the floor FAILS the regularity postcondition, and that failure
//!   is data (the enclosure below the floor is carried in the certificate,
//!   never dropped). `Err(ConditioningBelowThreshold)` is produced only when a
//!   region's margin could not be certified at all (the map refused a
//!   degenerate region); the self-contact arm is not run on such a loft.
//! - [`PairVerdict`] entries are data, one per candidate pair the self-contact
//!   arm actually decided. `Contact` and `Inconclusive` pairs are data, and
//!   the CALLER decides to refuse — the three-valued doctrine.
//!
//! # Section 2 — the two arms, in the pre-made order
//!
//! The regions of the self-contact analysis are the strips' subregions of the
//! loft's admitted map: strip `j`'s region is the `(u, v)` rectangle spanned by
//! the delivered strip surface's own knot ranges, a compact subregion of the
//! map's declared domain. The closed-wire strip cycle makes strip `j` adjacent
//! to strips `j − 1` and `j + 1` (modulo the strip count); adjacent strips
//! share their glued seam by construction (CC-012 P6) and are never
//! self-contact candidates.
//!
//! **Regularity arm.** [`rank_margin`](crate::certified_map::CertifiedSurfaceMap::rank_margin)
//! over the admitted map on every strip region; the minimum certified lower
//! bound is the certificate's regularity data. A margin query refused on a
//! region is a conditioning failure recorded as
//! [`ConstructRefusal::ConditioningBelowThreshold`] inside the produced
//! certificate.
//!
//! **Self-contact arm** — three discharge regimes in the theory's order:
//!
//! 1. (b) whole-region graph-disk discharge FIRST: a region whose certified
//!    map data is one affine parallelogram (every touched Bézier patch exactly
//!    flat, one shared tangent frame) with a positive certified area margin is
//!    discharged through [`certify_graph_disk`] (a single whole-region piece,
//!    simple projected rim). Within-region search runs only on UNDISCHARGED
//!    regions.
//! 2. (a) near-diagonal discharge within a stratum:
//!    [`injectivity_radius`] per region; pairs of search cells whose farthest
//!    parameter separation is within the certified radius `δ` are EXCLUDED
//!    from the candidate list by construction and never searched.
//! 3. (c) everything else: a certified broad phase over the region/cell world
//!    enclosures (`[map.enclosure]`) followed by the landed evidence contact
//!    funnel ([`contact`](truck_evidence::contact::contact)) through the CC-000
//!    manifest edge. Its outcomes convert as
//!
//!    | Evidence outcome | Pair verdict |
//!    |---|---|
//!    | `Ok` with a non-empty `ContactComplex` | `PairVerdict::Contact` |
//!    | `Ok` with an empty `ContactComplex` | `PairVerdict::Certified` |
//!    | `Err(Refusal::NumericallyUnresolved { .. })` | `PairVerdict::Inconclusive` |
//!    | any other `Err(Refusal)` (e.g. `UnsupportedEnvelope`) | propagated `Err(ConstructRefusal::InvalidInput)` |
//!
//!    The funnel is only reachable on canonical analytic carriers, which this
//!    packet obtains by lifting a flat affine region to an exact
//!    [`Plane`](truck_geometry::specifieds::Plane) from the map's patch data
//!    (the S12 affine-support precedent). A non-liftable candidate (a generic
//!    spline cell) is a refusal of the non-`NumericallyUnresolved` family and
//!    propagates as `Err(ConstructRefusal::InvalidInput)` — recorded here as
//!    the actual mapping table, never collapsed into `Inconclusive`.
//!
//! # Section 3 — budget discipline
//!
//! Every funnel decision draws from the caller's [`Budget`]
//! (entry-minus-remaining reporting; the funnel itself reports its spend the
//! same way). The within-region cell split depth is capped by
//! [`CC_DEPTH_MAX`]. A candidate pair whose budget is exhausted is
//! `Inconclusive`, never `Certified`.
//!
//! # House rules
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. **H-3** float comparisons in the tests carry
//! the opt-out on the SAME line. The near-diagonal attempted-pair counter is a
//! debug-only counting hook (`probe`, `#[cfg(test)]`) and does not exist in
//! shipped signatures.

use crate::certified_map::{CertifiedSurfaceMap, MapRefusal, SurfaceRegion};
use crate::construct::config::{CC_DEPTH_MAX, CC_ETA_J};
use crate::construct::graphdisk::{certify_graph_disk, DiskPiece};
use crate::construct::injectivity::injectivity_radius;
use crate::construct::loft_strips::LoftStrips;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::BoundaryPlan;
use crate::construct::Interval;
use crate::hull::bernstein_derivative_2d;
use truck_base::cgmath64::Point3;
use truck_base::evidence::{Budget, Certified, Refusal};
use truck_evidence::contact::{contact, BoundedStratum};
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::Plane;

/// The within-region cell split depth of the near-diagonal search (each axis
/// splits into `2^REGION_SEARCH_DEPTH` cells), capped by the normative
/// [`CC_DEPTH_MAX`].
const REGION_SEARCH_DEPTH: u32 = 3;

/// The near-diagonal attempted-pair probe (stop condition 3): a debug-only
/// counting hook that does not exist in shipped signatures.
#[cfg(test)]
pub mod probe {
    use std::cell::Cell;

    std::thread_local! {
        static ATTEMPTED: Cell<u64> = const { Cell::new(0) };
    }

    /// Reset the attempted-pair count.
    pub fn reset() {
        ATTEMPTED.with(|cell| cell.set(0));
    }

    /// The number of near-diagonal-excluded candidate pairs actually attempted
    /// (submitted to the pair decision).
    pub fn attempted() -> u64 {
        ATTEMPTED.with(|cell| cell.get())
    }

    /// Bump the attempted-pair count.
    pub(crate) fn bump() {
        ATTEMPTED.with(|cell| cell.set(cell.get() + 1));
    }
}

/// The per-pair verdict of the self-contact arm (theory §2.2 L5).
///
/// `Certified` records a certified contact-free pair; `Contact` records a
/// certified unintended self-contact; `Inconclusive` records a pair that could
/// not be decided (a numerically unresolved funnel outcome or a
/// budget-exhausted pair). Verdicts are data: the caller decides to refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairVerdict {
    /// The pair is certified free of contact.
    Certified,
    /// A certified unintended contact of the pair.
    Contact,
    /// The pair could not be decided; never certified as either clean or
    /// contacting.
    Inconclusive,
}

/// The loft validity certificate (Section 1).
#[derive(Debug, Clone)]
pub struct LoftValidityCert {
    /// The regularity arm's data: `Ok(margin)` carries the certified margin
    /// enclosure of the loft (the minimum certified `|Sᵤ × Sᵥ|` lower bound
    /// over the strip regions). The regularity postcondition — the margin floor
    /// [`CC_ETA_J`] — is the caller's threshold read on the enclosure; a margin
    /// below the floor is recorded failure data (a valid certificate about an
    /// invalid loft). `Err(ConditioningBelowThreshold)` records a region whose
    /// margin could not be certified at all.
    pub regularity: Result<Interval, ConstructRefusal>,
    /// The per-candidate-pair verdicts of the self-contact arm, in decision
    /// order. Empty when every candidate was discharged by the near-diagonal /
    /// whole-region regimes (or when the regularity arm refused).
    pub pairs: Vec<PairVerdict>,
    /// Whether the whole-region graph-disk discharge took precedence over the
    /// pairwise search for at least one region (all within-region pairs of the
    /// discharged regions were never searched).
    pub discharged_by_graphdisk: bool,
}

/// Certify the validity of the closed-wire loft `strips` over its admitted
/// certified map `map`.
///
/// `map` is the certified Euclidean map of the loft over its admitted domain;
/// `strips` is the CC-012 strip decomposition. Each strip's region is the
/// `(u, v)` rectangle spanned by the delivered strip surface's own knot
/// ranges (a compact subregion of the map domain). `Ok` ALWAYS means the
/// certificate was PRODUCED; whether the loft is valid is read off the
/// returned certificate (the three-valued doctrine).
///
/// Refuses [`ConstructRefusal::InvalidInput`] on structurally inconsistent
/// input (empty strips, a strip region outside the map's declared domain).
pub fn certify_loft_validity(
    map: &CertifiedSurfaceMap,
    strips: &LoftStrips,
    budget: &mut Budget,
) -> Result<LoftValidityCert, ConstructRefusal> {
    let regions = strip_regions(map, strips)?;

    // --- Regularity arm (Section 2), in the pre-made order. ---
    let mut min_margin_lo = f64::INFINITY;
    for region in &regions {
        match map.rank_margin(*region) {
            Ok(margin) => min_margin_lo = min_margin_lo.min(margin.lo),
            Err(MapRefusal::DomainNotCompact) => return Err(ConstructRefusal::InvalidInput),
            Err(_) => {
                // A degenerate / un-enclosable region: the margin could not be
                // certified at all. Recorded as conditioning failure data; the
                // self-contact arm is not run on a loft whose regularity cannot
                // be certified.
                return Ok(LoftValidityCert {
                    regularity: Err(ConstructRefusal::ConditioningBelowThreshold),
                    pairs: Vec::new(),
                    discharged_by_graphdisk: false,
                });
            }
        }
    }
    let regularity_interval = Interval {
        lo: min_margin_lo,
        hi: f64::INFINITY,
    };
    if !regularity_interval.lo.is_finite() {
        return Err(ConstructRefusal::InvalidInput);
    }
    if regularity_interval.lo < CC_ETA_J {
        // Below the CC-000 margin floor: the regularity postcondition fails.
        // Recorded failure data — the enclosure is carried in the produced
        // certificate and the self-contact arm is not run.
        return Ok(LoftValidityCert {
            regularity: Ok(regularity_interval),
            pairs: Vec::new(),
            discharged_by_graphdisk: false,
        });
    }

    // --- Self-contact arm (Section 2). ---
    let mut pairs: Vec<PairVerdict> = Vec::new();
    let n = regions.len();

    let mut deltas = Vec::with_capacity(n);
    for region in &regions {
        deltas.push(injectivity_radius(map, *region)?);
    }

    // Regime (b) FIRST: the whole-region graph-disk discharge. A region whose
    // certified data is one affine parallelogram discharges; within-region
    // pairwise search then never runs on it.
    let mut discharged = Vec::with_capacity(n);
    for region in &regions {
        discharged.push(whole_region_graph_disk(map, *region)?);
    }
    let discharged_by_graphdisk = discharged.iter().any(|flag| *flag);

    // Within-region near-diagonal + pairwise search on UNDISCHARGED regions.
    for (index, region) in regions.iter().enumerate() {
        if discharged[index] {
            continue;
        }
        let within = within_region_search(map, *region, deltas[index], budget)?;
        pairs.extend(within);
    }

    // Cross-region candidates: every non-adjacent strip pair. Adjacent strips
    // of the closed wire share their glued seam (CC-012 P6) and are never
    // self-contact candidates.
    for i in 0..n {
        for j in (i + 1)..n {
            if cyclically_adjacent(i, j, n) {
                continue;
            }
            pairs.push(decide_pair(map, regions[i], regions[j], budget)?);
        }
    }

    Ok(LoftValidityCert {
        regularity: Ok(regularity_interval),
        pairs,
        discharged_by_graphdisk,
    })
}

/// The strip regions of the loft, in strip order.
///
/// Strip `j`'s region is the `(u, v)` rectangle spanned by the delivered strip
/// surface's own clamped knot ranges, validated as a compact subregion of the
/// map's declared domain. An empty strip set or a region outside the map's
/// domain is [`ConstructRefusal::InvalidInput`].
fn strip_regions(
    map: &CertifiedSurfaceMap,
    strips: &LoftStrips,
) -> Result<Vec<SurfaceRegion>, ConstructRefusal> {
    if strips.strips.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let domain = map_declared_domain(map);
    let mut regions = Vec::with_capacity(strips.strips.len());
    for strip in &strips.strips {
        let region = strip_surface_region(&strip.surface)?;
        if !region_in_domain(region, domain) {
            return Err(ConstructRefusal::InvalidInput);
        }
        regions.push(region);
    }
    Ok(regions)
}

/// The `(u, v)` rectangle spanned by a delivered strip surface's own knot
/// ranges.
fn strip_surface_region(
    surface: &truck_geometry::prelude::BSplineSurface<truck_geometry::prelude::Vector4>,
) -> Result<SurfaceRegion, ConstructRefusal> {
    let u_knot = surface.uknot_vec();
    let v_knot = surface.vknot_vec();
    let region = (
        (u_knot[0], u_knot[u_knot.len() - 1]),
        (v_knot[0], v_knot[v_knot.len() - 1]),
    );
    if region_finite(region) {
        Ok(region)
    } else {
        Err(ConstructRefusal::InvalidInput)
    }
}

/// The map's declared domain, read off its patch table (the patches tile the
/// declared domain exactly).
fn map_declared_domain(map: &CertifiedSurfaceMap) -> SurfaceRegion {
    let boxes = map.patch_boxes();
    let first = boxes[0];
    let last = boxes[boxes.len() - 1];
    ((first.0 .0, last.0 .1), (first.1 .0, last.1 .1))
}

/// Whether a region is finite and non-degenerate on both axes.
fn region_finite(region: SurfaceRegion) -> bool {
    let ((u0, u1), (v0, v1)) = region;
    u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite() && u0 < u1 && v0 < v1
}

/// Whether `region` is a compact subset of `domain` (inclusive edges).
fn region_in_domain(region: SurfaceRegion, domain: SurfaceRegion) -> bool {
    let ((u0, u1), (v0, v1)) = region;
    let ((du0, du1), (dv0, dv1)) = domain;
    du0 <= u0 && u1 <= du1 && dv0 <= v0 && v1 <= dv1
}

/// Whether strip indices `i` and `j` share a glued seam of the closed-wire
/// strip cycle.
fn cyclically_adjacent(i: usize, j: usize, n: usize) -> bool {
    if n < 2 {
        return false;
    }
    let diff = i.abs_diff(j);
    diff == 1 || diff == n - 1
}

/// The whole-region graph-disk discharge (regime b): certify a single
/// whole-region [`DiskPiece`] through the landed decider
/// [`certify_graph_disk`].
///
/// A region discharges only when its certified map data is ONE affine
/// parallelogram — every touched Bézier patch exactly flat with one shared
/// tangent frame — and its certified area margin is strictly positive. A flat
/// region with positive margin is a graph disk over its own plane (the affine
/// map is injective and its rim projects to a simple parallelogram), so the
/// caller-built piece records `boundary_simple` and `seam_glued` and the
/// decider gates the discharge.
fn whole_region_graph_disk(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
) -> Result<bool, ConstructRefusal> {
    let Some(frame) = affine_frame(map, region)? else {
        return Ok(false);
    };
    if frame.degenerate {
        return Ok(false);
    }
    let margin = map
        .rank_margin(region)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    if margin.lo <= 0.0 || !margin.lo.is_finite() {
        return Ok(false);
    }
    let piece = DiskPiece {
        det_lower: Interval {
            lo: margin.lo,
            hi: f64::INFINITY,
        },
        boundary_simple: true,
        seam_glued: true,
    };
    let plan = BoundaryPlan {
        boundary_simple: true,
        seams_glued: true,
    };
    match certify_graph_disk(&[piece], &plan) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// A certified affine parallelogram over a map region.
struct AffineFrame {
    /// The affine value at the region's lower-left source corner.
    base: [f64; 3],
    /// The constant source-unit `u` tangent.
    su: [f64; 3],
    /// The constant source-unit `v` tangent.
    sv: [f64; 3],
    /// Whether the parallelogram is degenerate (zero tangent or collinear
    /// tangents).
    degenerate: bool,
}

/// The affine frame of a region whose certified map data is one parallelogram:
/// every touched Bézier patch is exactly flat (its second-partial coefficient
/// grids are exactly zero) and all touched patches share one constant tangent
/// frame (the S12 affine-support precedent). `None` when the region is not an
/// affine parallelogram (curved or creased data).
fn affine_frame(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
) -> Result<Option<AffineFrame>, ConstructRefusal> {
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    let mut frame: Option<([f64; 3], [f64; 3])> = None;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        if !boxes_touch(*patch_box, region) {
            continue;
        }
        if !patch_flat(patch_grids) {
            return Ok(None);
        }
        let (su, sv) = patch_tangents(patch_grids, *patch_box)?;
        match frame {
            None => frame = Some((su, sv)),
            Some((su0, sv0)) => {
                if su0 != su || sv0 != sv {
                    return Ok(None);
                }
            }
        }
    }
    let Some((su, sv)) = frame else {
        return Ok(None);
    };
    let cross = cross3(su, sv);
    let norm = norm3(cross);
    if !norm.is_finite() || norm == 0.0 {
        return Ok(Some(AffineFrame {
            base: [0.0; 3],
            su,
            sv,
            degenerate: true,
        }));
    }
    // The value at the region's lower-left corner, from the first touched
    // patch's exact affine data (the frame is shared, so the affine formula
    // holds over the whole region).
    let mut base: Option<[f64; 3]> = None;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        if !boxes_touch(*patch_box, region) {
            continue;
        }
        let origin = patch_base(patch_grids)?;
        let (u0, v0) = (patch_box.0 .0, patch_box.1 .0);
        let corner = add3_scaled(
            add3_scaled(origin, su, region.0 .0 - u0),
            sv,
            region.1 .0 - v0,
        );
        base = Some(corner);
        break;
    }
    let base = match base {
        Some(base) => base,
        None => return Ok(None),
    };
    Ok(Some(AffineFrame {
        base,
        su,
        sv,
        degenerate: false,
    }))
}

/// Lift a flat affine region to an exact canonical plane face stratum for the
/// evidence funnel, or `None` when the region is not an affine parallelogram.
fn lift_region(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
) -> Result<Option<BoundedStratum>, ConstructRefusal> {
    let Some(frame) = affine_frame(map, region)? else {
        return Ok(None);
    };
    if frame.degenerate {
        return Ok(None);
    }
    let o = point3(frame.base);
    let one = point3(add3_scaled(frame.base, frame.su, region.0 .1 - region.0 .0));
    let another = point3(add3_scaled(frame.base, frame.sv, region.1 .1 - region.1 .0));
    Ok(Some(BoundedStratum::Face {
        surface: CanonicalSurface::Plane(Plane::new(o, one, another)),
        u_range: (0.0, 1.0),
        v_range: (0.0, 1.0),
    }))
}

/// Decide one candidate pair (region or cell pair): the certified broad phase
/// over the world enclosures, then the budget-gated evidence funnel.
fn decide_pair(
    map: &CertifiedSurfaceMap,
    a: SurfaceRegion,
    b: SurfaceRegion,
    budget: &mut Budget,
) -> Result<PairVerdict, ConstructRefusal> {
    let box_a = map
        .enclosure(a)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    let box_b = map
        .enclosure(b)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    if certified_separated(&box_a, &box_b) {
        return Ok(PairVerdict::Certified);
    }
    if budget.subdiv == 0 && budget.newton == 0 {
        // A budget-exhausted pair is Inconclusive, never Certified (Section 3).
        return Ok(PairVerdict::Inconclusive);
    }
    funnel_pair(map, a, b, budget)
}

/// The within-region near-diagonal + pairwise search of one undischarged
/// region.
///
/// The region is split into a dyadic grid of cells (`2^REGION_SEARCH_DEPTH` per
/// axis, capped by [`CC_DEPTH_MAX`]); every unordered cell pair whose farthest
/// parameter separation lies within the region's certified injectivity radius
/// `δ` is EXCLUDED by construction and never searched (regime a). The remaining
/// far pairs are decided by [`decide_pair`].
fn within_region_search(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
    delta: Interval,
    budget: &mut Budget,
) -> Result<Vec<PairVerdict>, ConstructRefusal> {
    let depth = REGION_SEARCH_DEPTH.min(CC_DEPTH_MAX);
    let cells = split_cells(region, depth)?;
    let mut out = Vec::new();
    for i in 0..cells.len() {
        for j in (i + 1)..cells.len() {
            if near_diagonal_excluded(cells[i], cells[j], delta.lo) {
                continue;
            }
            #[cfg(test)]
            probe::bump();
            out.push(decide_pair(map, cells[i], cells[j], budget)?);
        }
    }
    Ok(out)
}

/// Whether a cell pair is excluded by the near-diagonal discharge: their
/// farthest parameter separation is strictly within the certified injectivity
/// radius `δ`, so no point pair of the two cells can coincide.
fn near_diagonal_excluded(a: SurfaceRegion, b: SurfaceRegion, delta_lo: f64) -> bool {
    if !delta_lo.is_finite() {
        // δ = +∞ (a certified-flat region): every finite pair lies within δ.
        return true;
    }
    let du = farthest_axis(a.0, b.0);
    let dv = farthest_axis(a.1, b.1);
    if !du.is_finite() || !dv.is_finite() {
        return false;
    }
    (du * du + dv * dv).sqrt() < delta_lo
}

/// The farthest separation of two intervals on one axis.
fn farthest_axis(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.1 - b.0).abs().max((a.0 - b.1).abs())
}

/// Split a region into a dyadic grid of `2^depth` × `2^depth` cells, in
/// ascending row-major order.
fn split_cells(region: SurfaceRegion, depth: u32) -> Result<Vec<SurfaceRegion>, ConstructRefusal> {
    let count = 1usize
        .checked_shl(depth)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let ((u0, u1), (v0, v1)) = region;
    let u_step = (u1 - u0) / (count as f64);
    let v_step = (v1 - v0) / (count as f64);
    if !u_step.is_finite() || !v_step.is_finite() || u_step <= 0.0 || v_step <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut cells = Vec::with_capacity(count * count);
    for i in 0..count {
        for j in 0..count {
            cells.push((
                (u0 + u_step * (i as f64), u0 + u_step * ((i + 1) as f64)),
                (v0 + v_step * (j as f64), v0 + v_step * ((j + 1) as f64)),
            ));
        }
    }
    Ok(cells)
}

/// Whether two certified world boxes are strictly separated on an axis (the
/// certified broad-phase discharge).
fn certified_separated(a: &[Interval; 3], b: &[Interval; 3]) -> bool {
    for axis in 0..3 {
        if a[axis].hi < b[axis].lo || b[axis].hi < a[axis].lo {
            return true;
        }
    }
    false
}

/// The evidence contact funnel (regime c): run the landed
/// [`contact`](truck_evidence::contact::contact) on the two candidate regions
/// lifted to canonical plane face strata, and map its outcome onto the pair
/// verdicts (the Section 2 mapping table).
fn funnel_pair(
    map: &CertifiedSurfaceMap,
    a: SurfaceRegion,
    b: SurfaceRegion,
    budget: &mut Budget,
) -> Result<PairVerdict, ConstructRefusal> {
    let Some(stratum_a) = lift_region(map, a)? else {
        // A candidate the funnel cannot lift is a refusal of the
        // non-`NumericallyUnresolved` family (a generic spline cell): recorded
        // as a propagated refusal, never collapsed into `Inconclusive`.
        return Err(ConstructRefusal::InvalidInput);
    };
    let Some(stratum_b) = lift_region(map, b)? else {
        return Err(ConstructRefusal::InvalidInput);
    };
    match contact(&stratum_a, &stratum_b, budget) {
        Ok(Certified { value, .. }) => {
            if value.contacts.is_empty() {
                Ok(PairVerdict::Certified)
            } else {
                Ok(PairVerdict::Contact)
            }
        }
        Err(Refusal::NumericallyUnresolved { .. }) => Ok(PairVerdict::Inconclusive),
        Err(_) => Err(ConstructRefusal::InvalidInput),
    }
}

/// Whether two rectangles overlap with positive area on both axes.
fn boxes_touch(a: SurfaceRegion, b: SurfaceRegion) -> bool {
    axis_overlap(a.0, b.0) && axis_overlap(a.1, b.1)
}

/// Whether two closed intervals overlap with positive length (a shared
/// boundary point alone is not an overlap: the adjacent patch's data does not
/// extend into this one).
fn axis_overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    let (lo, hi) = (a.0.max(b.0), a.1.min(b.1));
    lo < hi
}

/// The CC-002 flatness gate on one Bézier patch: the three second-partial
/// coefficient grids are EXACTLY zero.
fn patch_flat(grids: &[Vec<Vec<f64>>; 3]) -> bool {
    for grid in grids {
        if !flat_grid(grid) {
            return false;
        }
    }
    true
}

/// Whether one coefficient grid is exactly affine over its patch.
fn flat_grid(grid: &[Vec<f64>]) -> bool {
    let duu = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 0);
    let dvv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 1), 1);
    let duv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 1);
    all_zero(&duu) && all_zero(&dvv) && all_zero(&duv)
}

/// Whether every entry of a grid is exactly zero.
fn all_zero(grid: &[Vec<f64>]) -> bool {
    grid.iter().all(|row| row.iter().all(|c| *c == 0.0))
}

/// The source-unit tangent pair of a flat patch.
fn patch_tangents(
    grids: &[Vec<Vec<f64>>; 3],
    patch_box: SurfaceRegion,
) -> Result<([f64; 3], [f64; 3]), ConstructRefusal> {
    let width_u = patch_box.0 .1 - patch_box.0 .0;
    let width_v = patch_box.1 .1 - patch_box.1 .0;
    if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv_u = 1.0 / width_u;
    let inv_v = 1.0 / width_v;
    let mut su = [0.0_f64; 3];
    let mut sv = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let du = bernstein_derivative_2d(grid, 0);
        let dv = bernstein_derivative_2d(grid, 1);
        su[k] = constant_of(&du, inv_u)?;
        sv[k] = constant_of(&dv, inv_v)?;
    }
    Ok((su, sv))
}

/// Read the (exact) common value of a constant coefficient grid, scaled by
/// `scale`. A non-constant grid is refused.
fn constant_of(grid: &[Vec<f64>], scale: f64) -> Result<f64, ConstructRefusal> {
    let first = match grid.first().and_then(|row| row.first()) {
        Some(value) => *value,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    for row in grid {
        for value in row {
            if *value != first {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
    }
    Ok(first * scale)
}

/// The value of a flat patch at its lower-left source corner.
fn patch_base(grids: &[Vec<Vec<f64>>; 3]) -> Result<[f64; 3], ConstructRefusal> {
    let mut base = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let value = match grid.first().and_then(|row| row.first()) {
            Some(value) => *value,
            None => return Err(ConstructRefusal::InvalidInput),
        };
        base[k] = value;
    }
    Ok(base)
}

/// Scaled vector addition `a + s·d`.
fn add3_scaled(a: [f64; 3], d: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] + s * d[0], a[1] + s * d[1], a[2] + s * d[2]]
}

/// The cross product of two vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The Euclidean norm of a vector.
fn norm3(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The truck-base point from coordinates.
fn point3(p: [f64; 3]) -> Point3 {
    Point3::new(p[0], p[1], p[2])
}

#[cfg(test)]
mod tests {
    use super::probe;
    use super::{certify_loft_validity, PairVerdict};
    use crate::certified_map::admit_surface;
    use crate::construct::loft::LoftOutput;
    use crate::construct::loft_strips::LoftStrips;
    use crate::construct::refusal::ConstructRefusal;
    use crate::formal::numeric::PositiveFinite;
    use truck_base::evidence::Budget;
    use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3, Vector4};

    /// Extract the `Ok` of a fallible construction; the fixture data is valid
    /// by construction, so the refusal arm is a test-bug panic.
    fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
        match result {
            Ok(value) => value,
            Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
        }
    }

    /// A homogeneous control point with unit weight (test helper).
    fn p4(x: f64, y: f64, z: f64) -> Vector4 {
        Vector4::new(x, y, z, 1.0)
    }

    /// A trivial unit-weight bilinear strip surface over the given ranges.
    fn strip_surface(u: (f64, f64), v: (f64, f64)) -> BSplineSurface<Vector4> {
        let u_knot = KnotVec::from(vec![u.0, u.0, u.1, u.1]);
        let v_knot = KnotVec::from(vec![v.0, v.0, v.1, v.1]);
        let points = vec![
            vec![p4(u.0, v.0, 0.0), p4(u.0, v.1, 0.0)],
            vec![p4(u.1, v.0, 0.0), p4(u.1, v.1, 0.0)],
        ];
        construct(
            BSplineSurface::try_new((u_knot, v_knot), points)
                .map_err(|_| ConstructRefusal::InvalidInput),
        )
    }

    /// The single-strip decomposition over the given region.
    fn one_strip(u: (f64, f64), v: (f64, f64)) -> LoftStrips {
        LoftStrips {
            strips: vec![LoftOutput {
                surface: strip_surface(u, v),
                epsilon: 0.0,
            }],
            seam_ids: Vec::new(),
        }
    }

    /// A bilinear surface `(u, v, k·u·v)` over the unit square (a single
    /// non-flat Bézier patch; a certified map).
    fn twisted_patch(k: f64) -> BSplineSurface<Point3> {
        let knot = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
        let points = vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, k)],
        ];
        construct(
            BSplineSurface::try_new((knot.clone(), knot), points)
                .map_err(|_| ConstructRefusal::InvalidInput),
        )
    }

    /// The expected number of beyond-`δ` cell pairs of the 8 × 8 grid over
    /// `[0, 1]²` under the near-diagonal discharge (the test's own evaluation
    /// of the exclusion predicate).
    fn expected_far_pairs(delta_lo: f64) -> u64 {
        let mut count = 0u64;
        for i in 0..8 {
            for j in 0..8 {
                let a = (
                    (i as f64 / 8.0, (i + 1) as f64 / 8.0),
                    (j as f64 / 8.0, (j + 1) as f64 / 8.0),
                );
                for p in 0..8 {
                    for q in 0..8 {
                        if (i, j) >= (p, q) {
                            continue;
                        }
                        let b = (
                            (p as f64 / 8.0, (p + 1) as f64 / 8.0),
                            (q as f64 / 8.0, (q + 1) as f64 / 8.0),
                        );
                        let du = (a.0 .1 - b.0 .0).abs().max((a.0 .0 - b.0 .1).abs());
                        let dv = (a.1 .1 - b.1 .0).abs().max((a.1 .0 - b.1 .1).abs());
                        let within_delta = (du * du + dv * dv).sqrt() < delta_lo;
                        if !within_delta {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn near_diagonal_pairs_are_excluded_by_radius_never_searched() {
        // A curved (non-flat, hence NOT graph-disk-dischargeable) single strip
        // whose certified injectivity radius δ is finite. Pairs of 8 × 8
        // within-region cells whose farthest parameter separation lies within
        // δ are EXCLUDED by the near-diagonal discharge and are never searched:
        // the debug-only attempted-pair probe counts only the beyond-δ pairs.
        let surface = twisted_patch(5.0);
        let tau = PositiveFinite::new(5.0e-30).expect("a positive declared tau");
        let map =
            construct(admit_surface(&surface, tau).map_err(|_| ConstructRefusal::InvalidInput));
        let strips = one_strip((0.0, 1.0), (0.0, 1.0));

        let mut budget = Budget::new(0, 0, 0);
        probe::reset();
        let cert = construct(certify_loft_validity(&map, &strips, &mut budget));
        assert!(
            !cert.discharged_by_graphdisk,
            "the twisted patch is not a graph disk"
        );
        let Ok(regularity) = cert.regularity else {
            panic!("the twisted patch must certify a positive margin");
        };
        assert!(regularity.lo >= crate::construct::config::CC_ETA_J); // H-3: certified margin above the J floor

        // The delta = 2σ/L lower bound (finite, well below the region
        // diameter): the certified exclusion radius of the near-diagonal arm.
        let radius = construct(crate::construct::injectivity::injectivity_radius(
            &map,
            ((0.0, 1.0), (0.0, 1.0)),
        ));
        assert!(radius.lo.is_finite()); // H-3: the twisted patch carries a finite radius
        let expected = expected_far_pairs(radius.lo);
        assert!(
            expected > 0,
            "the fixture must produce beyond-δ candidate pairs"
        );
        assert!(
            expected < 64 * 63 / 2,
            "the near-diagonal discharge must exclude at least one cell pair"
        );

        // Every beyond-δ pair was attempted (one verdict per attempted pair);
        // the within-δ pairs were excluded by construction and NEVER searched.
        // The exclusion is the point: the attempted-pair probe equals exactly
        // the beyond-δ count, never the full cell-pair count.
        assert_eq!(
            probe::attempted(),
            expected,
            "only beyond-δ pairs are attempted"
        );
        assert_eq!(cert.pairs.len() as u64, expected);
        assert!(
            !cert.pairs.contains(&PairVerdict::Contact),
            "the twisted patch has no certified contact"
        );
    }
}
