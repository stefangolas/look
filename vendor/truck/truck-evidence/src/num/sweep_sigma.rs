//! CL-003-SWEEP-ENCLOSURE: the certified sweep-side σ_G helper.
//!
//! The first fundamental form of a sweep over its windowed domain
//! `[s0, s1] × [v0, v1]` is the symmetric 2×2 tensor whose entries are the
//! inner products of the two first partials,
//!
//! ```text
//! g_ss = ⟨X_s, X_s⟩,   g_sv = g_vs = ⟨X_s, X_v⟩,   g_vv = ⟨X_v, X_v⟩
//! ```
//!
//! [`sweep_sigma_g`] returns outward-rounded interval bounds on all four
//! entries over a window, composed from the sweep's certified derivative
//! enclosures (`EnclosureSurface::enclose_der`, CL-003's `enclosure_sweep`
//! module): each entry is an interval expression over the derivative boxes —
//! squares and inner products of the box coordinates — so it is certified
//! outward and **additive** (a sub-window's bound is contained in its
//! parent's, the solver's subdivision premise).
//!
//! Sweeps are pole-free, so the derivative enclosures are finite on any
//! certifiable window; a window whose bound would be non-finite (an empty or
//! unbounded box — a window outside the certified envelope) is refused as
//! [`Refusal::Empty`] rather than emitting a non-finite bound (the
//! boundedness assertion, scope decision 3). The certificate is `Float`
//! (H-6: bounds are certified outward composition, never `Exact`).

use super::enclosure_sweep::box_is_finite;
use crate::enclosure::{EnclosureSurface, Interval};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
};
use truck_geometry::constructive::SpineFrameSweep;

/// The certified first fundamental form (σ_G) of a sweep over a window: an
/// interval bound on each of the tensor's four entries.
#[derive(Clone, Copy, Debug)]
pub struct SweepSigmaG {
    /// `g_ss = ⟨X_s, X_s⟩`: the bound on the s-column squared scale.
    pub g_ss: Interval,
    /// `g_sv = ⟨X_s, X_v⟩`: the bound on the s×v metric entry.
    pub g_sv: Interval,
    /// `g_vs = ⟨X_v, X_s⟩`: the mirror entry (equal in value to `g_sv`; kept
    /// for the four-entry tensor form).
    pub g_vs: Interval,
    /// `g_vv = ⟨X_v, X_v⟩`: the bound on the v-column squared scale.
    pub g_vv: Interval,
}

/// The σ_G certificate: float method (all arithmetic is outward-rounded
/// interval composition over the certified derivative boxes — never `Exact`,
/// H-6), untouched budget, unbounded margin and modulus.
fn certificate() -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: Budget::new(0, 0, 0),
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// The certified sweep-side σ_G over `ss × vv`: interval bounds on the first
/// fundamental form's four entries, composed from the sweep's derivative
/// enclosures.
///
/// CERTIFIES: each returned interval contains the corresponding entry
/// `⟨∂X, ∂X⟩` evaluated at every parameter point of the window. Refuses
/// `Refusal::Empty` when a window lies outside the certified envelope (so a
/// derivative box is empty or unbounded) — never emits a non-finite bound.
pub fn sweep_sigma_g(sweep: &SpineFrameSweep, ss: Interval, vv: Interval) -> Outcome<SweepSigmaG> {
    let bs = EnclosureSurface::enclose_der(sweep, 1, 0, ss, vv);
    let bv = EnclosureSurface::enclose_der(sweep, 0, 1, ss, vv);
    if !box_is_finite(&bs) || !box_is_finite(&bv) {
        return Err(Refusal::Empty);
    }
    // Entrywise interval arithmetic over the derivative boxes: the actual
    // partial at every window point lies in the box (the certified
    // enclosures), so the squares and inner products of the box coordinates
    // bound the actual metric entries (BG-ENC-001; the boxes may decorrelate,
    // which over-estimates and is acceptable).
    let g_ss = bs.x.sqr() + bs.y.sqr() + bs.z.sqr();
    let g_sv = bs.x * bv.x + bs.y * bv.y + bs.z * bv.z;
    let g_vv = bv.x.sqr() + bv.y.sqr() + bv.z.sqr();
    Ok(Certified::new(
        SweepSigmaG {
            g_ss,
            g_sv,
            g_vs: g_sv,
            g_vv,
        },
        certificate(),
    ))
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::num::enclosure_sweep::fixtures::required_sweeps;
    use truck_base::cgmath64::Vector3;
    use truck_geotrait::ParametricSurface;

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// An interior sample `k/(n + 1)` of the `n`-sample lattice over `[lo, hi]`
    /// (samples never touch the window boundary).
    fn interior_sample(lo: f64, hi: f64, k: usize, n: usize) -> f64 {
        lo + (hi - lo) * ((k + 1) as f64) / ((n + 1) as f64)
    }

    /// The brute first fundamental form entries at a sample: `E = ⟨u, u⟩`,
    /// `F = ⟨u, v⟩`, `G = ⟨v, v⟩` for the landed first partials `u = uder`,
    /// `v = vder`.
    fn brute_metric(sweep: &SpineFrameSweep, s: f64, v: f64) -> (f64, f64, f64) {
        let u = sweep.uder(s, v);
        let w = sweep.vder(s, v);
        let dot = |a: Vector3, b: Vector3| a.x * b.x + a.y * b.y + a.z * b.z;
        (dot(u, u), dot(u, w), dot(w, w))
    }

    /// Test-only unwrap of the σ_G outcome: a windowed fixture sweep always
    /// certifies, so a refusal here is a test bug.
    fn must(r: Outcome<SweepSigmaG>) -> SweepSigmaG {
        match r {
            Ok(Certified { value, .. }) => value,
            Err(_) => unreachable!("a windowed fixture sweep must certify"),
        }
    }

    /// CL-003 required test 2: the σ_G bounds bracket the brute
    /// first-fundamental-form entries on the same three sweeps.
    #[test]
    fn sweep_sigma_g_bounds_brute_metric() {
        let sweeps = required_sweeps();
        assert!(
            sweeps.len() >= 3,
            "the required fixture set must hold three sweeps"
        );
        const GRID: usize = 40;
        for sweep in &sweeps {
            let ss = iv(sweep.s0(), sweep.s1());
            let vv = iv(sweep.v0(), sweep.v1());
            let sigma = must(sweep_sigma_g(sweep, ss, vv));
            for i in 0..GRID {
                for j in 0..GRID {
                    let s = interior_sample(sweep.s0(), sweep.s1(), i, GRID);
                    let v = interior_sample(sweep.v0(), sweep.v1(), j, GRID);
                    let (e, f, g) = brute_metric(sweep, s, v);
                    assert!(
                        sigma.g_ss.contains(e),
                        "g_ss entry {e} at ({s}, {v}) escaped {:?}",
                        sigma.g_ss
                    );
                    assert!(
                        sigma.g_sv.contains(f) && sigma.g_vs.contains(f),
                        "off-diagonal entry {f} at ({s}, {v}) escaped {:?}/{:?}",
                        sigma.g_sv,
                        sigma.g_vs
                    );
                    assert!(
                        sigma.g_vv.contains(g),
                        "g_vv entry {g} at ({s}, {v}) escaped {:?}",
                        sigma.g_vv
                    );
                }
            }
        }
    }

    /// The σ_G bounds are additive over a window split: a sub-window's entries
    /// are contained in the parent's.
    #[test]
    fn sweep_sigma_g_is_additive_over_window_split() {
        let sweeps = required_sweeps();
        for sweep in &sweeps {
            let ss = iv(sweep.s0(), sweep.s1());
            let vv = iv(sweep.v0(), sweep.v1());
            let parent = must(sweep_sigma_g(sweep, ss, vv));
            let s_mid = sweep.s0() + (sweep.s1() - sweep.s0()) / 2.0;
            let v_mid = sweep.v0() + (sweep.v1() - sweep.v0()) / 2.0;
            for window in [
                iv(sweep.s0(), s_mid),
                iv(s_mid, sweep.s1()),
                iv(sweep.v0(), v_mid),
                iv(v_mid, sweep.v1()),
            ] {
                let (cs, cv) = if window.inf() >= sweep.v0() && window.sup() <= sweep.v1() {
                    (ss, window)
                } else {
                    (window, vv)
                };
                let child = must(sweep_sigma_g(sweep, cs, cv));
                assert!(
                    parent.g_ss.inf() <= child.g_ss.inf() && child.g_ss.sup() <= parent.g_ss.sup(),
                    "child g_ss {:?} escapes parent {:?}",
                    child.g_ss,
                    parent.g_ss
                );
                assert!(
                    parent.g_sv.inf() <= child.g_sv.inf() && child.g_sv.sup() <= parent.g_sv.sup(),
                    "child g_sv {:?} escapes parent {:?}",
                    child.g_sv,
                    parent.g_sv
                );
                assert!(
                    parent.g_vv.inf() <= child.g_vv.inf() && child.g_vv.sup() <= parent.g_vv.sup(),
                    "child g_vv {:?} escapes parent {:?}",
                    child.g_vv,
                    parent.g_vv
                );
            }
        }
    }
}
