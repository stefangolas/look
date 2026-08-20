//! BG-ENC-002 property tests for the `Plane` reference (H-7 layer 2).
//!
//! Layer 1 (named-witness units) lives in `src/plane.rs`. Layer 3 (margin
//! sweep) is a no-op for a plane because the affine enclosure has no margin
//! parameter; the total-outcome property is asserted here instead.

use inari::Interval;
use proptest::prelude::*;
use truck_base::cgmath64::{Point3, Vector3};
use truck_evidence::enclosure::EnclosureSurface;
use truck_geometry::specifieds::Plane;
use truck_geotrait::ParametricSurface;

/// A non-degenerate random plane and a random box.
fn plane_and_box() -> impl Strategy<Value = (Plane, Interval, Interval)> {
    // Origin, u-axis, v-axis: keep them comfortably non-degenerate by bounding
    // magnitudes and requiring the u-axis x-component to clear a floor. The
    // floor is a dimensionless orientation test, hence exempt from the H-3
    // length-literal rule.
    let point =
        (-10.0f64..10.0, -10.0f64..10.0, -10.0f64..10.0).prop_map(|(x, y, z)| Point3::new(x, y, z));
    let axis = (0.1f64..5.0, -5.0f64..5.0, -5.0f64..5.0)
        .prop_filter("non-degenerate axis", |(x, _, _)| x.abs() > 1.0e-3)
        .prop_map(|(x, y, z)| Vector3::new(x, y, z));

    let box_iv = (0.01f64..3.0).prop_flat_map(|w| {
        (0.0f64..10.0).prop_map(move |lo| {
            let hi = lo + w;
            (lo, hi)
        })
    });

    (point, axis.clone(), axis, box_iv.clone(), box_iv).prop_map(
        |(o, pu, pv, (u0, u1), (v0, v1))| {
            let plane = Plane::new(o, o + pu, o + pv);
            let uu = Interval::try_from((u0, u1)).expect("valid u box");
            let vv = Interval::try_from((v0, v1)).expect("valid v box");
            (plane, uu, vv)
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// BG-ENC-001 soundness: every sampled surface point lies in the enclosure.
    #[test]
    fn plane_enclose_is_sound((plane, uu, vv) in plane_and_box()) {
        let box3 = plane.enclose(uu, vv);
        let (u0, u1) = (uu.inf(), uu.sup());
        let (v0, v1) = (vv.inf(), vv.sup());
        // Deterministic 21x21 grid: enough to catch under-estimation without
        // making each proptest case heavy. Soundness covers parameters INSIDE
        // the box only: the last grid point, computed as
        // `u0 + (u1 - u0) * 20.0 / 20.0`, is a multiply-then-divide round
        // trip that can land one ulp ABOVE u1 (persisted seed e2369bfc:
        // u1 = 1.6356989675203588 samples at 1.635698967520359), and the
        // point evaluated there may sit one ulp outside the correctly
        // rounded enclosure. Clamp pins the rounded grid back into the box;
        // interior points are a full grid step inside and never clamp.
        for i in 0..21 {
            for j in 0..21 {
                let u = (u0 + (u1 - u0) * (i as f64) / 20.0).clamp(u0, u1);
                let v = (v0 + (v1 - v0) * (j as f64) / 20.0).clamp(v0, v1);
                let pt = plane.subs(u, v);
                prop_assert!(box3.contains(pt), "({u},{v}) -> {pt:?} escaped {box3:?}");
            }
        }
    }

    /// BG-ENC-002 convergence: bisection never widens the enclosure.
    #[test]
    fn plane_enclose_converges_monotonically((plane, uu, vv) in plane_and_box()) {
        let mut cur_uu = uu;
        let mut cur_vv = vv;
        let mut prev = plane.enclose(cur_uu, cur_vv).width();
        for _ in 0..12 {
            // Bisect the wider axis.
            let (du, dv) = (cur_uu.sup() - cur_uu.inf(), cur_vv.sup() - cur_vv.inf());
            if du >= dv {
                let mid = (cur_uu.inf() + cur_uu.sup()) / 2.0;
                cur_uu = Interval::try_from((cur_uu.inf(), mid)).expect("valid bisection");
            } else {
                let mid = (cur_vv.inf() + cur_vv.sup()) / 2.0;
                cur_vv = Interval::try_from((cur_vv.inf(), mid)).expect("valid bisection");
            }
            let cur = plane.enclose(cur_uu, cur_vv).width();
            prop_assert!(cur <= prev, "widened under bisection: {prev} -> {cur}");
            prev = cur;
        }
    }

    /// Totality: the affine enclosure never produces NaN bounds on valid input.
    #[test]
    fn plane_enclose_is_total((plane, uu, vv) in plane_and_box()) {
        let box3 = plane.enclose(uu, vv);
        prop_assert!(box3.x.inf().is_finite() && box3.x.sup().is_finite());
        prop_assert!(box3.y.inf().is_finite() && box3.y.sup().is_finite());
        prop_assert!(box3.z.inf().is_finite() && box3.z.sup().is_finite());
    }
}
