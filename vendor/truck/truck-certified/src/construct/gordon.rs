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

//! The Gordon surface construction (CC-015-GORDON, spine S8 consumer): the
//! Boolean-sum surface `S = S_u + S_v − S_uv` of two loft families.
//!
//! A Gordon input is a network of two independent curve families over one
//! shared parameter rectangle: the **profiles** `P_j(u)` (a family of curves
//! running in the `u` direction, one per `v` station) and the **guides**
//! `G_i(v)` (a family running in the `v` direction, one per `u` station).
//! When the network is consistent — `P_j(u_i) = G_i(v_j)` at every network
//! point `(u_i, v_j)` — the Boolean sum passes through both families exactly:
//!
//! ```text
//! S(u, v) = S_u(u, v) + S_v(u, v) − S_uv(u, v)
//! ```
//!
//! where `S_u` is the profile loft (interpolates the profiles across the `v`
//! stations), `S_v` is the guide loft (interpolates the guides across the `u`
//! stations), and `S_uv` is the correction surface: the tensor-product
//! interpolation AT the network points `(u_i, v_j)` of the network values
//! (the amount the two lofts double-count at every crossing). Because the
//! blending functions are the interpolation cardinal functions of the two
//! station grids, `S_u` restricted to `v = v_j` is `P_j`, `S_v` restricted to
//! `u = u_i` is `G_i`, and the correction restricted to either grid equals the
//! coincident network value, so the double count cancels exactly at every
//! network point.
//!
//! # No new carrier
//!
//! Like the loft core (CC-010, the S8 reuse contract), the theory-seam
//! "`Point4`" homogeneous point `(x, y, z, w)` is spelled with the landed
//! homogeneous carrier [`Vector4`] of `truck_geometry`, and the delivered
//! surface is the CC-010 output type [`LoftOutput`] — no new carrier and no
//! new certification type are introduced. The output is certified by CC-014
//! like any other surface.
//!
//! # The decomposition (theory §2.4, all objects from CC-010)
//!
//! 1. **Compatibility.** Each family goes through
//!    [`make_compatible`](crate::construct::loft::make_compatible) separately
//!    (the families are independent; nothing in the code couples their knot
//!    vectors). After compatibility every profile shares one `u` basis and
//!    every guide shares one `v` basis.
//! 2. **The two direction factorizations.** The construction needs exactly
//!    two banded-TP factorizations — one per direction:
//!    `factor_u` over `stations_u` at the profile degree and `factor_v` over
//!    `stations_v` at the guide degree. This is the complexity claim: the
//!    `I × J` network is never solved as a tensor system; only the two 1-D
//!    factorizations exist and both are reused.
//! 3. **The component lofts.** `S_u = loft_sections(profiles, stations_v, …,
//!    &factor_v)` and `S_v` is `loft_sections(guides, stations_u, …,
//!    &factor_u)` with the two axes swapped, so that the delivered `S_v` runs
//!    in `u` across the `v`-direction guide basis. Both lofts therefore share
//!    the two factorizations.
//! 4. **The correction.** Each network row (one profile sampled at the `u`
//!    stations) is interpolated through `factor_u` — the same cached
//!    factorization used by the guide loft — producing one correction curve
//!    per `v` station, and those correction curves are lofted through
//!    `factor_v` — the same cached factorization used by the profile loft.
//!    One factorization per direction, asserted by the identical `epsilon` the
//!    shared factor reports for the profile loft and for the correction's
//!    direction solve.
//! 5. **Combination.** The three component control nets are added pointwise in
//!    homogeneous `R4` with the fixed accumulation order `(S_u + S_v) − S_uv`
//!    per control entry, iterated `u`-major then `v`. This is only meaningful
//!    when the three component surfaces already share one `(u, v)` basis
//!    pair; the code checks that the profile `u` basis equals the averaged
//!    `u`-station knot vector and the guide `v` basis equals the averaged
//!    `v`-station knot vector, and refuses
//!    [`ConstructRefusal::InvalidInput`] on a mismatch — never a silent
//!    re-basis.
//!
//! The delivered [`LoftOutput::epsilon`] is the sum of the three component
//! enclosure widths: a control-entry enclosure width of the combined net is
//! at most the sum of the component widths, so the sum keeps the CC-010 L2
//! meaning for the Boolean-summed net.
//!
//! # House rules
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::banded::factor_banded_tp;
use crate::construct::loft::{
    averaged_knot_vector, loft_collocation_bands, loft_sections, make_compatible, LoftOutput,
};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, ParametricCurve, Vector4};

/// The Gordon network input (CC-015-GORDON).
///
/// `profiles` is the `u`-direction curve family (one profile per `v` station,
/// so `profiles.len() == stations_v.len()`); `guides` is the `v`-direction
/// curve family (one guide per `u` station, so
/// `guides.len() == stations_u.len()`). The homogeneous `(x, y, z, w)`
/// carrier is `truck_geometry`'s [`Vector4`] (the S8 spelling of the
/// theory-seam `Point4`).
#[derive(Debug, Clone)]
pub struct GordonInput {
    /// The profile family: curves running in `u`, stationed along `v`.
    pub profiles: Vec<BSplineCurve<Vector4>>,
    /// The guide family: curves running in `v`, stationed along `u`.
    pub guides: Vec<BSplineCurve<Vector4>>,
    /// The strictly increasing `u` stations (one per guide).
    pub stations_u: Vec<f64>,
    /// The strictly increasing `v` stations (one per profile).
    pub stations_v: Vec<f64>,
}

/// The Gordon Boolean-sum construction over a compatible profile/guide
/// network, delivered as the CC-010 [`LoftOutput`] (no new carrier).
///
/// All construction objects come from CC-010: each family through
/// [`make_compatible`](crate::construct::loft::make_compatible), the two
/// component lofts through
/// [`loft_sections`](crate::construct::loft::loft_sections) over the two
/// direction factorizations, and the correction surface by interpolating the
/// network values at the network points through those SAME two factorizations
/// (one factorization per direction, reused). The three component control nets
/// are combined pointwise `S = S_u + S_v − S_uv` in homogeneous `R4` with the
/// fixed accumulation order `(S_u + S_v) − S_uv`.
///
/// Refuses [`ConstructRefusal::InvalidInput`] on an empty family, on family /
/// station-count mismatches, on station vectors that are not strictly
/// increasing finite values with `degree + 1 <= count`, and — the basis gate —
/// when the compatible profile `u` basis is not exactly the averaged `u`
/// station knot vector or the compatible guide `v` basis is not exactly the
/// averaged `v` station knot vector. The gate is an exact knot-vector
/// equality: incompatible component bases refuse, never a silent re-basis. A
/// singular collocation factor refuses inside the solve as
/// [`ConstructRefusal::SingularInterpolationSystem`].
pub fn gordon_surface(input: &GordonInput) -> Result<LoftOutput, ConstructRefusal> {
    if input.profiles.is_empty() || input.guides.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    if input.profiles.len() != input.stations_v.len()
        || input.guides.len() != input.stations_u.len()
    {
        return Err(ConstructRefusal::InvalidInput);
    }

    // Step 1: independent compatibility of the two families.
    let profiles = make_compatible(&input.profiles)?;
    let guides = make_compatible(&input.guides)?;

    // The two direction degrees are the family degrees: the profile family
    // carries the final `u` degree and the guide family the final `v` degree.
    let u_degree = profiles[0].degree();
    let v_degree = guides[0].degree();

    // Step 2: the two direction factorizations (one factorization per
    // direction; every later solve reuses one of these two).
    let factor_u = factor_banded_tp(&loft_collocation_bands(&input.stations_u, u_degree)?)?;
    let factor_v = factor_banded_tp(&loft_collocation_bands(&input.stations_v, v_degree)?)?;

    // The basis-compatibility gate: each component surface must already live
    // on one shared (u, v) basis pair for the control-net combination to be
    // meaningful. The profile `u` basis must equal the averaged `u`-station
    // knot vector and the guide `v` basis the averaged `v`-station knot
    // vector; anything else is an incompatible component basis and refuses.
    let w_u = averaged_knot_vector(&input.stations_u, u_degree);
    let w_v = averaged_knot_vector(&input.stations_v, v_degree);
    if profiles[0].knot_vec() != &w_u || guides[0].knot_vec() != &w_v {
        return Err(ConstructRefusal::InvalidInput);
    }

    // Step 3: the profile loft S_u (u basis × averaged v-station basis) and
    // the guide loft S_v (averaged u-station basis × guide v basis).
    let su = loft_sections(&profiles, &input.stations_v, v_degree, &factor_v)?;
    let sv_raw = loft_sections(&guides, &input.stations_u, u_degree, &factor_u)?;
    let mut sv = sv_raw.surface;
    sv.swap_axes();

    // Step 4: the correction surface S_uv. Each network row is interpolated
    // through factor_u (the SAME cached factorization the guide loft used),
    // giving one u-basis correction curve per v station; those correction
    // curves are then lofted through factor_v (the SAME cached factorization
    // the profile loft used).
    let mut corrections = Vec::with_capacity(profiles.len());
    for profile in &profiles {
        let solved = factor_u.solve_homogeneous(&network_row(profile, &input.stations_u))?;
        let controls: Vec<Vector4> = solved.iter().map(interval4_to_vector4).collect();
        let curve = BSplineCurve::try_new(w_u.clone(), controls)
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        corrections.push(curve);
    }
    let suv = loft_sections(&corrections, &input.stations_v, v_degree, &factor_v)?;

    // Step 5: pointwise combination S = S_u + S_v − S_uv on the shared net.
    let net = combine_control_nets(&su.surface, &sv, &suv.surface)?;
    let surface = BSplineSurface::try_new(
        (
            su.surface.uknot_vec().clone(),
            su.surface.vknot_vec().clone(),
        ),
        net,
    )
    .map_err(|_| ConstructRefusal::InvalidInput)?;

    // The delivered enclosure width: per control entry the width of the
    // Boolean sum is at most the sum of the three component widths, so the
    // sum keeps the CC-010 L2 meaning for the combined net.
    let epsilon = su.epsilon + sv_raw.epsilon + suv.epsilon;
    Ok(LoftOutput { surface, epsilon })
}

/// One network row: the homogeneous values of `profile` at every `u` station,
/// as point intervals in the fixed station order.
fn network_row(profile: &BSplineCurve<Vector4>, stations_u: &[f64]) -> Vec<[Interval; 4]> {
    let mut rhs = Vec::with_capacity(stations_u.len());
    for &station in stations_u {
        let point = profile.subs(station);
        rhs.push([
            Interval::point(point.x),
            Interval::point(point.y),
            Interval::point(point.z),
            Interval::point(point.w),
        ]);
    }
    rhs
}

/// The center point of a certified interval (the delivered control value).
#[inline]
fn interval4_to_vector4(value: &[Interval; 4]) -> Vector4 {
    Vector4::new(
        value[0].lo + (value[0].hi - value[0].lo) * 0.5,
        value[1].lo + (value[1].hi - value[1].lo) * 0.5,
        value[2].lo + (value[2].hi - value[2].lo) * 0.5,
        value[3].lo + (value[3].hi - value[3].lo) * 0.5,
    )
}

/// Combine three control nets pointwise: `(S_u + S_v) − S_uv` per entry, in
/// fixed `u`-major, `v`-minor order.
///
/// The three surfaces must already share one basis pair; a net-shape mismatch
/// is a defensive [`ConstructRefusal::InvalidInput`] (the basis gate upstream
/// makes it unreachable).
fn combine_control_nets(
    su: &BSplineSurface<Vector4>,
    sv: &BSplineSurface<Vector4>,
    suv: &BSplineSurface<Vector4>,
) -> Result<Vec<Vec<Vector4>>, ConstructRefusal> {
    let su_pts = su.control_points();
    let sv_pts = sv.control_points();
    let suv_pts = suv.control_points();
    if su_pts.len() != sv_pts.len() || su_pts.len() != suv_pts.len() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut net = Vec::with_capacity(su_pts.len());
    for i in 0..su_pts.len() {
        let su_row = &su_pts[i];
        let sv_row = &sv_pts[i];
        let suv_row = &suv_pts[i];
        if su_row.len() != sv_row.len() || su_row.len() != suv_row.len() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let mut row = Vec::with_capacity(su_row.len());
        for j in 0..su_row.len() {
            let entry = (su_row[j] + sv_row[j]) - suv_row[j];
            row.push(entry);
        }
        net.push(row);
    }
    Ok(net)
}
