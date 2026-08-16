//! The shared sampling-soundness harness (P-6).
//!
//! BG-ENC-001's soundness test — every sampled point of a carrier lies inside
//! its enclosure — must run for *every* carrier impl. Writing it twenty times
//! badly is how under-estimating enclosures go unnoticed. This module gives one
//! generic helper; the per-carrier tests call it over random boxes.
//!
//! Sampling is a *necessary* soundness test, not a proof: an enclosure that
//! under-estimates on a set of measure zero can pass every finite sample. The
//! real guarantee is interval arithmetic's outward rounding; this harness is
//! the guard against a regression that reintroduces an inward-rounding fast
//! path. BG-FID-003/004 later add the certified whole-span checks.

use crate::enclosure::{Box3, EnclosureCurve, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::Point3;

/// Asserts that every point of the curve over `tt` sampled at `samples`
/// equidistant parameters lies inside the enclosure.
///
/// # Panics
/// If any sampled `subs(t)` escapes `enclose(tt)` — i.e. the enclosure
/// under-estimates. `samples` must be at least 2.
pub fn assert_encloses_curve<C: EnclosureCurve>(c: &C, tt: Interval, samples: usize) {
    assert!(samples >= 2, "assert_encloses_curve needs >= 2 samples");
    let box3 = c.enclose(tt);
    let lo = tt.inf();
    let hi = tt.sup();
    let step = (hi - lo) / (samples as f64 - 1.0);
    for i in 0..samples {
        let t = lo + step * (i as f64);
        let pt: Point3 = c.subs(t);
        assert!(
            box3.contains(pt),
            "curve point at t={t} escaped enclose({tt:?}): {pt:?} not in {box3:?}"
        );
    }
}

/// Asserts that every point of the surface over `uu × vv` sampled on an
/// `samples × samples` grid lies inside the enclosure.
///
/// # Panics
/// If any sampled `subs(u, v)` escapes `enclose(uu, vv)`. `samples` must be at
/// least 2.
pub fn assert_encloses_surface<S: EnclosureSurface>(
    s: &S,
    uu: Interval,
    vv: Interval,
    samples: usize,
) {
    assert!(samples >= 2, "assert_encloses_surface needs >= 2 samples");
    let box3 = s.enclose(uu, vv);
    let (u0, u1) = (uu.inf(), uu.sup());
    let (v0, v1) = (vv.inf(), vv.sup());
    let us = (u1 - u0) / (samples as f64 - 1.0);
    let vs = (v1 - v0) / (samples as f64 - 1.0);
    for i in 0..samples {
        for j in 0..samples {
            let (u, v) = (u0 + us * (i as f64), v0 + vs * (j as f64));
            let pt: Point3 = s.subs(u, v);
            assert!(
                box3.contains(pt),
                "surface point at ({u},{v}) escaped enclose({uu:?},{vv:?}): {pt:?} not in {box3:?}"
            );
        }
    }
}

/// Asserts that a refinement of the box shrinks the enclosure width toward
/// zero (BG-ENC-002 convergence): for a non-degenerate carrier, bisecting a box
/// must never widen the enclosure, and the width must go to zero as the box
/// does.
///
/// # Panics
/// If bisection ever widens the enclosure, or if the width fails to converge
/// to zero within `depth` bisections of a box of initial width `initial`.
pub fn assert_converges<S: EnclosureSurface>(
    s: &S,
    uu: Interval,
    vv: Interval,
    initial: f64,
    depth: usize,
) {
    let mut uu = uu;
    let mut vv = vv;
    let mut prev = s.enclose(uu, vv).width();
    for _ in 0..depth {
        // Bisect the wider axis so the box actually shrinks. Bisecting a valid
        // interval always yields a valid interval (lo <= mid <= hi); the
        // failure branch is an internal invariant (H-1: assert carries why).
        let (du, dv) = (uu.sup() - uu.inf(), vv.sup() - vv.inf());
        if du >= dv {
            let mid = (uu.inf() + uu.sup()) / 2.0;
            uu = match Interval::try_from((uu.inf(), mid)) {
                Ok(bisected) => bisected,
                Err(_) => {
                    // Invariant: lo <= mid <= hi for any valid interval.
                    // Not reachable from any input; assert documents why.
                    assert!((uu.inf()..=uu.sup()).contains(&mid));
                    uu
                }
            };
        } else {
            let mid = (vv.inf() + vv.sup()) / 2.0;
            vv = match Interval::try_from((vv.inf(), mid)) {
                Ok(bisected) => bisected,
                Err(_) => {
                    assert!((vv.inf()..=vv.sup()).contains(&mid));
                    vv
                }
            };
        }
        let cur = s.enclose(uu, vv).width();
        assert!(
            cur <= prev,
            "enclosure widened under bisection: {prev} -> {cur}"
        );
        prev = cur;
    }
    assert!(
        prev < initial,
        "enclosure did not converge below initial width {initial}: {prev}"
    );
}

/// Asserts that a box contains a point, with a message naming the box.
pub fn assert_box_contains(b: &Box3, pt: Point3, what: &str) {
    assert!(b.contains(pt), "{what} {pt:?} escaped {b:?}");
}
