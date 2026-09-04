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

//! The certified loft core (CC-010-LOFT-CORE, spine S8): tensor-product loft
//! construction over the landed `truck_geometry::nurbs` types.
//!
//! The construction is ordinary tensor-product B-spline interpolation of
//! section curves in homogeneous coordinates. No new carrier is introduced:
//! the theory-seam "`Point4`" homogeneous point `(x, y, z, w)` is spelled with
//! the landed homogeneous carrier [`Vector4`] of `truck_geometry` (the S8
//! reuse contract, `CERTIFIED_CONSTRUCTION_CONTRACTS.md` F1). A section is a
//! `BSplineCurve<Vector4>` over a clamped `KnotVec`; the delivered loft is a
//! `BSplineSurface<Vector4>` whose `u` axis is the section parameter and whose
//! `v` axis is the station axis. The four homogeneous channels are carried
//! verbatim through the certified banded solve, so rational sections loft
//! without any weight-field special casing here (that is CC-011's
//! certification, not this packet's construction).
//!
//! # The construction (theory §2.1)
//!
//! 1. **Compatibility** ([`make_compatible`]): exact degree elevation to
//!    `p = max_k p_k`, then exact knot-vector union by knot insertion. Knot
//!    equality is exact `f64` value equality — no tolerance merging anywhere;
//!    unequal near-equal knots are BOTH retained and BOTH inserted. The cost is
//!    additive in the total knot count: every section absorbs exactly its
//!    deficit against the union knot multiset, so the total insertion work is
//!    `O(K · |U|)` control updates, never a re-solve and never a re-fit.
//! 2. **Stationing** ([`chord_length_stations`]): per-section polyline chord
//!    lengths accumulated in section order and normalized by the total. The
//!    polyline sample count is the module constant [`CHORD_STATION_SAMPLES`];
//!    the accumulation order is documented there and is fixed (reproducibility
//!    contract). [`averaged_knot_vector`] turns the stations into the `v` knot
//!    vector by de Boor averaging.
//! 3. **The solve** ([`loft_sections`]): the collocation band storage
//!    `A_{kj} = M_{j,q}(v_k)` is assembled in a fixed evaluation order
//!    ([`loft_collocation_bands`]), factorized through
//!    [`factor_banded_tp`](crate::construct::banded::factor_banded_tp) (a
//!    refusal there is a Schoenberg–Whitney violation of the stationing
//!    policy and propagates as
//!    [`ConstructRefusal::SingularInterpolationSystem`]; there is NEVER a
//!    fallback to a dense or pivoting solve), and every homogeneous control
//!    row is solved through [`BandedFactor::solve_homogeneous`]. The delivered
//!    surface control net is assembled row-major: row `i` holds the `v`
//!    control row that interpolates the `i`-th control column of the sections.
//!
//! # The L2 enclosure
//!
//! [`LoftOutput::epsilon`] is the maximum enclosure width delivered by the
//! solve — the `ε` downstream predicates consume (L2). The surface's control
//! points are the centers of their certified enclosures, so the delivered
//! surface deviates from the interpolant by at most `ε/2` on any isoparametric
//! section; the L1 gate (`loft_reproduces_sections_identically_up_to_epsilon`)
//! asserts the deviation stays within `ε`.
//!
//! # House rules
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::banded::BandedFactor;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, KnotVec, ParametricCurve, Vector4};

/// The per-section polyline chord sample count of [`chord_length_stations`].
///
/// Each section is sampled at this many equal parametric steps over its own
/// declared knot range; the chord lengths of the resulting edges are summed in
/// ascending-parameter order. 64 samples per section is well above the
/// bandwidth of a cubic section while keeping the stationing cost linear in
/// the section count.
pub const CHORD_STATION_SAMPLES: usize = 64;

/// The delivered loft surface and its L2 enclosure width `ε`.
///
/// `surface` is the tensor-product loft over the shared section `u` knot
/// vector and the station `v` knot vector; `epsilon` is the maximum control
/// enclosure width delivered by the certified solve and is what downstream
/// predicates consume — they may not assume exactness.
#[derive(Debug, Clone)]
pub struct LoftOutput {
    /// The delivered tensor-product loft surface (homogeneous control net).
    pub surface: BSplineSurface<Vector4>,
    /// The L2 enclosure width `ε`: the maximum control-entry width.
    pub epsilon: f64,
}

/// Compatibility: exact degree elevation then exact knot-vector union.
///
/// Every section is elevated exactly to `p = max_k p_k` and then absorbs, by
/// knot insertion, exactly the knots it lacks against the union knot multiset
/// of the whole input. Knot equality is exact `f64` value equality: unequal
/// near-equal knots are both retained and both inserted, never tolerance
/// merged. The cost is additive in the total knot count.
///
/// Refuses [`ConstructRefusal::InvalidInput`] on an empty input, or when any
/// section is not clamped (an unclamped knot vector cannot be unioned by knot
/// insertion without changing the section's domain).
pub fn make_compatible(
    sections: &[BSplineCurve<Vector4>],
) -> Result<Vec<BSplineCurve<Vector4>>, ConstructRefusal> {
    if sections.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let mut curves = Vec::with_capacity(sections.len());
    for section in sections {
        if !section.is_clamped() {
            return Err(ConstructRefusal::InvalidInput);
        }
        curves.push(section.clone());
    }

    // Exact degree elevation to the common maximum degree.
    let mut degree = curves[0].degree();
    for curve in &curves {
        if curve.degree() > degree {
            degree = curve.degree();
        }
    }
    for curve in curves.iter_mut() {
        while curve.degree() < degree {
            curve.elevate_degree();
        }
    }

    // Exact union knot multiset: for each exactly-equal value the maximum
    // multiplicity over the (elevated) sections. No tolerance is involved.
    let mut union_counts: Vec<(f64, usize)> = Vec::new();
    for curve in &curves {
        for (value, count) in exact_run_counts(curve.knot_vec()) {
            match union_counts.iter_mut().find(|(v, _)| *v == value) {
                Some((_, stored)) if count > *stored => *stored = count,
                Some(_) => {}
                None => union_counts.push((value, count)),
            }
        }
    }
    union_counts.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Every section absorbs exactly its deficit against the union multiset.
    for curve in curves.iter_mut() {
        for &(value, target) in &union_counts {
            let present = curve.knot_vec().iter().filter(|&&k| k == value).count();
            for _ in present..target {
                curve.add_knot(value);
            }
        }
    }
    Ok(curves)
}

/// Stationing: per-section polyline chord lengths, cumulated in section order
/// and normalized by the total.
///
/// For each section (in input order) the section's own polyline chord length
/// is accumulated by sequential `f64` adds over the sampled edges, and the
/// running total is pushed as that section's station. After every section the
/// running total is the normalization denominator, so the last station is
/// exactly `1.0`. The polyline sample count is [`CHORD_STATION_SAMPLES`].
///
/// The stations are deterministic (fixed summation order everywhere) and lie
/// in `[0, 1]`. Non-degenerate sections give strictly increasing stations; a
/// caller whose sections are all degenerate gets a flat station vector and
/// must treat the loft as invalid (that is the caller's precondition to check
/// — see [`loft_sections`]).
pub fn chord_length_stations(sections: &[BSplineCurve<Vector4>]) -> Vec<f64> {
    let mut stations = Vec::with_capacity(sections.len());
    let mut running = 0.0_f64;
    for section in sections {
        let first = section.knot_vec()[0];
        let last = section.knot_vec()[section.knot_vec().len() - 1];
        let step = (last - first) / (CHORD_STATION_SAMPLES as f64);
        let mut previous = euclid(section.subs(first));
        let mut chord = 0.0_f64;
        for i in 1..=CHORD_STATION_SAMPLES {
            let t = first + step * (i as f64);
            let current = euclid(section.subs(t));
            chord += edge_length(previous, current);
            previous = current;
        }
        running += chord;
        stations.push(running);
    }
    let total = running;
    if total > 0.0 {
        for station in stations.iter_mut() {
            *station /= total;
        }
    }
    stations
}

/// De Boor-averaged `v` knot vector of the stationing (L0 construction).
///
/// Given strictly increasing stations `v_0 .. v_{K-1}` and degree `q`, the
/// clamped knot vector of length `K + q + 1` repeats `v_0` at the front and
/// `v_{K-1}` at the back `q + 1` times and places the interior knots
/// `ξ_{j+q} = (1/q) · Σ_{r=j}^{j+q-1} v_r` in the fixed accumulation order
/// `j .. j+q-1`.
///
/// Strictly increasing stations with `degree + 1 <= stations.len()` are a
/// caller precondition; [`loft_sections`] and [`loft_collocation_bands`] check
/// it and refuse [`ConstructRefusal::InvalidInput`] on a violation.
pub fn averaged_knot_vector(stations: &[f64], degree: usize) -> KnotVec {
    let count = stations.len();
    let mut knots = Vec::with_capacity(count + degree + 1);
    for _ in 0..=degree {
        knots.push(stations[0]);
    }
    for p in (degree + 1)..count {
        let j = p - degree;
        let mut acc = 0.0_f64;
        for r in j..(j + degree) {
            acc += stations[r];
        }
        knots.push(acc / (degree as f64));
    }
    for _ in 0..=degree {
        knots.push(stations[count - 1]);
    }
    KnotVec::from(knots)
}

/// The row-major collocation band storage `A_{kj} = M_{j,q}(v_k)`.
///
/// Rows are stations `k` in input order, columns are the `v`-control indices
/// `j`; every entry outside a station's active basis window is the exact zero
/// point interval. Evaluation uses the averaged knot vector of the stationing
/// and the land-mounted basis evaluator, in a fixed station-major order.
///
/// Refuses [`ConstructRefusal::InvalidInput`] when the stations are not
/// strictly increasing finite values, when `degree` is zero or not at least
/// one below the station count, or when the averaged knot vector cannot be
/// evaluated at a station.
pub fn loft_collocation_bands(
    stations: &[f64],
    degree: usize,
) -> Result<Vec<Interval>, ConstructRefusal> {
    validate_stations(stations, degree)?;
    let knot = averaged_knot_vector(stations, degree);
    let order = stations.len();
    let mut bands = vec![Interval { lo: 0.0, hi: 0.0 }; order * order];
    for (row, &station) in stations.iter().enumerate() {
        let window = knot
            .try_bspline_basis_functions(degree, 0, station)
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        let base = window.base();
        for (offset, &value) in window.as_slice().iter().enumerate() {
            bands[row * order + base + offset] = Interval::point(value);
        }
    }
    Ok(bands)
}

/// The loft solve (spine S8): interpolate the sections' homogeneous control
/// columns across the stations through the given collocation factor.
///
/// `sections` must already be compatible (identical clamped knot vector and
/// degree — [`make_compatible`] produces exactly this); `stations` must be
/// strictly increasing with `stations.len() == sections.len()` and
/// `degree + 1 <= stations.len()`; and `factor` must be the banded-TP
/// factorization of the collocation storage `A_{kj} = M_{j,q}(v_k)` built from
/// the SAME stations and degree (see [`loft_collocation_bands`]). Any
/// violation is [`ConstructRefusal::InvalidInput`]; a singular factor refuses
/// inside the solve as `SingularInterpolationSystem` — never a fallback solve.
///
/// Each homogeneous control column (one `u`-index of every section) is solved
/// in one `solve_homogeneous` call; the tensor-product net is assembled from
/// the solved rows in row-major order (row = `u`-index, entry = `v`-control).
/// The delivered `epsilon` is [`BandedFactor::max_control_error`] after the
/// solve — the L2 enclosure width downstream predicates consume.
pub fn loft_sections(
    sections: &[BSplineCurve<Vector4>],
    stations: &[f64],
    degree: usize,
    factor: &BandedFactor,
) -> Result<LoftOutput, ConstructRefusal> {
    if sections.is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let station_count = sections.len();
    if stations.len() != station_count {
        return Err(ConstructRefusal::InvalidInput);
    }
    validate_stations(stations, degree)?;

    let common_knot = sections[0].knot_vec().clone();
    let common_degree = sections[0].degree();
    for section in sections {
        if section.degree() != common_degree || section.knot_vec() != &common_knot {
            return Err(ConstructRefusal::InvalidInput);
        }
    }

    let v_knot = averaged_knot_vector(stations, degree);
    let u_count = common_knot.len() - common_degree - 1;

    // Solve every homogeneous control column through the shared factor.
    let mut net = Vec::with_capacity(u_count);
    let mut best_index = 0usize;
    let mut best_error = -1.0_f64;
    for i in 0..u_count {
        let rhs = control_column(sections, station_count, i);
        let solved = factor.solve_homogeneous(&rhs)?;
        let row_error = factor.max_control_error();
        if row_error > best_error {
            best_error = row_error;
            best_index = i;
        }
        net.push(solved.iter().map(interval4_to_vector4).collect());
    }

    // Re-run the widest control column last so the factor's interior cache —
    // and therefore `max_control_error` — reports the GLOBAL maximum width,
    // which is the honest L2 bound for the whole net.
    if u_count > 1 {
        let rhs = control_column(sections, station_count, best_index);
        let _ = factor.solve_homogeneous(&rhs)?;
    }
    let epsilon = factor.max_control_error();

    let surface = match BSplineSurface::try_new((common_knot, v_knot), net) {
        Ok(surface) => surface,
        Err(_) => return Err(ConstructRefusal::InvalidInput),
    };
    Ok(LoftOutput { surface, epsilon })
}

/// Validate the stationing preconditions shared by the solve entry points.
fn validate_stations(stations: &[f64], degree: usize) -> Result<(), ConstructRefusal> {
    if stations.len() < 2 || degree == 0 || degree + 1 > stations.len() {
        return Err(ConstructRefusal::InvalidInput);
    }
    for pair in stations.windows(2) {
        if !(pair[0].is_finite() && pair[1].is_finite() && pair[0] < pair[1]) {
            return Err(ConstructRefusal::InvalidInput);
        }
    }
    Ok(())
}

/// The `i`-th homogeneous control column of every section, as interval rows in
/// section order (one row per station).
fn control_column(
    sections: &[BSplineCurve<Vector4>],
    station_count: usize,
    i: usize,
) -> Vec<[Interval; 4]> {
    let mut rhs = Vec::with_capacity(station_count);
    for section in sections {
        let point = *section.control_point(i);
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

/// Project a homogeneous point to its Euclidean coordinate triple.
#[inline]
fn euclid(homogeneous: Vector4) -> [f64; 3] {
    [
        homogeneous.x / homogeneous.w,
        homogeneous.y / homogeneous.w,
        homogeneous.z / homogeneous.w,
    ]
}

/// The Euclidean length of the edge between two projected samples.
#[inline]
fn edge_length(p: [f64; 3], q: [f64; 3]) -> f64 {
    let dx = q[0] - p[0];
    let dy = q[1] - p[1];
    let dz = q[2] - p[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// The distinct-value runs of an ascending knot slice, with exact `f64`
/// equality.
fn exact_run_counts(knots: impl AsRef<[f64]>) -> Vec<(f64, usize)> {
    let knots = knots.as_ref();
    let mut runs: Vec<(f64, usize)> = Vec::new();
    for &knot in knots {
        match runs.last_mut() {
            Some((value, count)) if *value == knot => *count += 1,
            _ => runs.push((knot, 1)),
        }
    }
    runs
}
