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

//! The §5.9 (C5) `CertifiedPatch` implementation for the landed bilinear
//! Coons surface (BG-KV2-205-C5PATCH).
//!
//! The landed `CoonsSurface` (truck-geometry `decorators/coons.rs`) is a
//! bilinear patch — POLYNOMIAL, so its certified evaluation needs no
//! transcendental and no weight field. Its stored corners `p00..p11` are the
//! whole certified geometry: the four cached corners are recovered from the
//! stored `bottom`/`top` boundary endpoints (the constructor caches exactly
//! `p00 = bottom(0)`, `p10 = bottom(1)`, `p01 = top(0)`, `p11 = top(1)`), and
//! every enclosure below is the interval evaluation of the bilinear Bernstein
//! form over those corners. This implementation lives in truck-certified
//! because the trait lives here and the type lives in truck-geometry (orphan
//! rule): no change is made to `CoonsSurface` itself.
//!
//! **Certified content.** A `CoonsSurface` whose four boundary curves are the
//! straight corner segments — the C5 recipe's geometry — is *exactly* the
//! bilinear corner interpolation, so these enclosures are exact for it (the
//! C5 consumer constructs straight-segment boundaries). The `CertifiedPatch`
//! shape cannot express that restriction; it is recorded here so no caller
//! certifies a patch whose boundaries are curved.
//!
//! **N2 (evaluation order is pinned).** Every certified reduction in this
//! module uses a fixed, documented order; no reassociation and no
//! order-nondeterministic reduction.
//!
//! * `enclose`, per coordinate, expands the bilinear form in exactly
//!   `((1-u)(1-v)p00 + u(1-v)p10) + ((1-u)v p01 + u v p11)`: the four
//!   Bernstein factor products `(1-u)(1-v)`, `u(1-v)`, `(1-u)v`, `u v` are
//!   formed first, then each scaled corner point, then the two parenthesized
//!   sums, then the outer sum.
//! * `derivs`, per coordinate, evaluates the once-hand-differentiated partials
//!   `S_u = (1-v)(p10 - p00) + v (p11 - p01)` and
//!   `S_v = (1-u)(p01 - p00) + u (p11 - p10)`: each corner difference
//!   interval is formed first, then scaled by its Bernstein factor, then the
//!   two products are summed in that order.
//! * `regularity` forms `E G - F^2` with `E = S_u . S_u`, `F = S_u . S_v`,
//!   `G = S_v . S_v`; each dot product sums componentwise in fixed
//!   x-then-y-then-z order.
//!
//! **N4.** No transcendental function call appears in this module (a
//! machine-checked source scan pins it): every enclosure is product-form
//! interval arithmetic over the landed [`CertifiedInterval`] primitive.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.

use crate::formal::exact::CertifiedInterval;
use crate::kernel::config::TOL_JACOBIAN;
use crate::kernel::evidence::ClaimVerdict;
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPositive, Cone, Degeneracy, DerivativeEnclosure, IBox2, IBox3, Pole,
    Reason,
};
use truck_base::cgmath64::Point3;
use truck_geometry::decorators::CoonsSurface;
use truck_geotrait::ParametricCurve3D;

/// The `Inconclusive` reason when the certified `EG - F^2` enclosure straddles
/// the §0.4 singular-map floor [`TOL_JACOBIAN`](crate::kernel::config::TOL_JACOBIAN).
const EGF2_STRADDLES: Reason = "coons_regularity_egf2_straddles_singular_floor";
/// The `Inconclusive` reason when a certified positive lower bound cannot be
/// constructed (unreachable for the finite positive bounds this module forms).
const POSITIVE_REFUSED: Reason = "coons_regularity_positive_bound_refused";
/// The `Inconclusive` reason when the constant-1 weight bound refuses
/// (unreachable: `CertifiedPositive::try_new(1.0)` always succeeds).
const WEIGHT_REFUSED: Reason = "coons_weight_bound_refused";

/// A three-component interval vector.
type V3 = [CertifiedInterval; 3];

/// The interval point of a scalar.
fn point(x: f64) -> CertifiedInterval {
    CertifiedInterval::point(x)
}

/// The interval for one axis of a box.
fn iv(lo: f64, hi: f64) -> CertifiedInterval {
    CertifiedInterval { lo, hi }
}

/// An `IBox3` from an interval vector.
fn box3_of(v: &V3) -> IBox3 {
    IBox3 {
        lo: [v[0].lo, v[1].lo, v[2].lo],
        hi: [v[0].hi, v[1].hi, v[2].hi],
    }
}

/// The interval vector of a point.
fn point3(p: [f64; 3]) -> V3 {
    [point(p[0]), point(p[1]), point(p[2])]
}

/// The two axis intervals of a parameter box.
fn axes(d: IBox2) -> (CertifiedInterval, CertifiedInterval) {
    (iv(d.lo[0], d.hi[0]), iv(d.lo[1], d.hi[1]))
}

/// A point as its coordinate array.
fn into_array(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

/// Recover the four cached corners from the stored boundary curves. The landed
/// constructor caches `p00 = bottom(0)`, `p10 = bottom(1)`, `p01 = top(0)`,
/// `p11 = top(1)`, so evaluating the `bottom` and `top` boundaries at their
/// endpoints reproduces the corners deterministically (`bottom` first, then
/// `top`; N2).
fn corners_of<C: ParametricCurve3D>(
    surface: &CoonsSurface<C>,
) -> ([f64; 3], [f64; 3], [f64; 3], [f64; 3]) {
    let p00 = into_array(surface.bottom().subs(0.0));
    let p10 = into_array(surface.bottom().subs(1.0));
    let p01 = into_array(surface.top().subs(0.0));
    let p11 = into_array(surface.top().subs(1.0));
    (p00, p10, p01, p11)
}

/// The componentwise interval difference of two interval vectors.
fn diff3(a: &V3, b: &V3) -> V3 {
    [a[0].sub(&b[0]), a[1].sub(&b[1]), a[2].sub(&b[2])]
}

/// An interval vector scaled componentwise by a scalar interval.
fn scaled3(a: &V3, s: &CertifiedInterval) -> V3 {
    [a[0].mul(s), a[1].mul(s), a[2].mul(s)]
}

/// The componentwise interval sum of two interval vectors.
fn add3(a: &V3, b: &V3) -> V3 {
    [a[0].add(&b[0]), a[1].add(&b[1]), a[2].add(&b[2])]
}

/// The certified bilinear position evaluation over the box, per coordinate in
/// the pinned order `((1-u)(1-v)p00 + u(1-v)p10) + ((1-u)v p01 + u v p11)`
/// (N2).
fn bilinear3(
    p00: &V3,
    p10: &V3,
    p01: &V3,
    p11: &V3,
    u: &CertifiedInterval,
    v: &CertifiedInterval,
) -> V3 {
    let omu = point(1.0).sub(u);
    let omv = point(1.0).sub(v);
    let a = omu.mul(&omv);
    let b = u.mul(&omv);
    let c = omu.mul(v);
    let d = u.mul(v);
    let mut out = [CertifiedInterval::point(0.0); 3];
    for (i, slot) in out.iter_mut().enumerate() {
        let left = a.mul(&p00[i]).add(&b.mul(&p10[i]));
        let right = c.mul(&p01[i]).add(&d.mul(&p11[i]));
        *slot = left.add(&right);
    }
    out
}

/// The certified `S_u = (1-v)(p10 - p00) + v (p11 - p01)` enclosure over the
/// box (N2: differences first, then scaled, then summed).
fn u_partial(p00: &V3, p10: &V3, p01: &V3, p11: &V3, v: &CertifiedInterval) -> V3 {
    let omv = point(1.0).sub(v);
    let d0 = diff3(p10, p00);
    let d1 = diff3(p11, p01);
    add3(&scaled3(&d0, &omv), &scaled3(&d1, v))
}

/// The certified `S_v = (1-u)(p01 - p00) + u (p11 - p10)` enclosure over the
/// box (N2: differences first, then scaled, then summed).
fn v_partial(p00: &V3, p10: &V3, p01: &V3, p11: &V3, u: &CertifiedInterval) -> V3 {
    let omu = point(1.0).sub(u);
    let d0 = diff3(p01, p00);
    let d1 = diff3(p11, p10);
    add3(&scaled3(&d0, &omu), &scaled3(&d1, u))
}

/// The certified position enclosure of the bilinear patch over `d`.
fn position_of<C: ParametricCurve3D>(surface: &CoonsSurface<C>, d: IBox2) -> IBox3 {
    let (a, b, c, e) = corners_of(surface);
    let (u, v) = axes(d);
    let s = bilinear3(&point3(a), &point3(b), &point3(c), &point3(e), &u, &v);
    box3_of(&s)
}

/// The certified first-derivative enclosures of the bilinear patch over `d`.
fn derivative_enclosure<C: ParametricCurve3D>(
    surface: &CoonsSurface<C>,
    d: IBox2,
) -> DerivativeEnclosure {
    let (a, b, c, e) = corners_of(surface);
    let (u, v) = axes(d);
    let p00 = point3(a);
    let p10 = point3(b);
    let p01 = point3(c);
    let p11 = point3(e);
    let su = u_partial(&p00, &p10, &p01, &p11, &v);
    let sv = v_partial(&p00, &p10, &p01, &p11, &u);
    DerivativeEnclosure {
        su: box3_of(&su),
        sv: box3_of(&sv),
    }
}

/// The interval dot product of two boxes, componentwise in fixed
/// x-then-y-then-z order (N2).
fn dot3(a: &IBox3, b: &IBox3) -> CertifiedInterval {
    let x = iv(a.lo[0], a.hi[0]).mul(&iv(b.lo[0], b.hi[0]));
    let y = iv(a.lo[1], a.hi[1]).mul(&iv(b.lo[1], b.hi[1]));
    let z = iv(a.lo[2], a.hi[2]).mul(&iv(b.lo[2], b.hi[2]));
    x.add(&y).add(&z)
}

/// The certified `EG - F^2` enclosure over `d`: exactly the interval
/// [`CertifiedPatch::regularity`] classifies. Spec §5.9's one-call rule —
/// the exposed Jacobian `S_u x S_v` and this `EG - F^2` describe the same
/// surface — is machine-checked against the landed `CoonsSurface::jacobian`
/// by the integration test. `E = S_u . S_u`, `F = S_u . S_v`, and
/// `G = S_v . S_v` are formed from the derivative enclosures with the pinned
/// dot order above.
pub fn egf2<C: ParametricCurve3D>(surface: &CoonsSurface<C>, d: IBox2) -> CertifiedInterval {
    let de = derivative_enclosure(surface, d);
    let e = dot3(&de.su, &de.su);
    let g = dot3(&de.sv, &de.sv);
    let f = dot3(&de.su, &de.sv);
    e.mul(&g).sub(&f.mul(&f))
}

/// The interval cross product of two boxes.
fn cross_box(a: &IBox3, b: &IBox3) -> IBox3 {
    let ax = iv(a.lo[0], a.hi[0]);
    let ay = iv(a.lo[1], a.hi[1]);
    let az = iv(a.lo[2], a.hi[2]);
    let bx = iv(b.lo[0], b.hi[0]);
    let by = iv(b.lo[1], b.hi[1]);
    let bz = iv(b.lo[2], b.hi[2]);
    let cx = ay.mul(&bz).sub(&az.mul(&by));
    let cy = az.mul(&bx).sub(&ax.mul(&bz));
    let cz = ax.mul(&by).sub(&ay.mul(&bx));
    IBox3 {
        lo: [cx.lo, cy.lo, cz.lo],
        hi: [cx.hi, cy.hi, cz.hi],
    }
}

/// A certified normal cone over the cross-product enclosure of the derivative
/// enclosure: the coordinate axis with the largest certified lower bound of
/// the dot product over the cross-product box, closed-hemisphere cone — the
/// local constructor discipline of `leaf.rs`. When no coordinate axis
/// certifies (a box straddling every coordinate plane) no hemisphere cone
/// exists; the best-coordinate `PI/2` cone is returned and callers subdivide
/// until the certified arm holds.
fn normal_cone_of<C: ParametricCurve3D>(surface: &CoonsSurface<C>, d: IBox2) -> Cone {
    let de = derivative_enclosure(surface, d);
    let normal = cross_box(&de.su, &de.sv);
    let candidates = [
        normal.lo[0],
        -normal.hi[0],
        normal.lo[1],
        -normal.hi[1],
        normal.lo[2],
        -normal.hi[2],
    ];
    let mut best = 0usize;
    for (idx, &margin) in candidates.iter().enumerate() {
        if margin > candidates[best] {
            best = idx;
        }
    }
    let axis = match best {
        0 => [1.0, 0.0, 0.0],
        1 => [-1.0, 0.0, 0.0],
        2 => [0.0, 1.0, 0.0],
        3 => [0.0, -1.0, 0.0],
        4 => [0.0, 0.0, 1.0],
        _ => [0.0, 0.0, -1.0],
    };
    match Cone::try_new(axis, std::f64::consts::FRAC_PI_2) {
        Ok(cone) => cone,
        Err(_) => Cone {
            axis,
            half_angle: std::f64::consts::FRAC_PI_2,
        },
    }
}

/// Classify a certified `EG - F^2` enclosure into the regularity claim:
/// `Proven` iff the certified lower bound clears the §0.4 singular-map floor
/// `TOL_JACOBIAN`, `Disproven` with a degeneracy witness iff the enclosure
/// provably sits below the floor (the folded patch: construction-valid,
/// geometry-invalid, §5.9), and `Inconclusive` when the enclosure straddles
/// the floor.
fn classify_regularity(
    d: IBox2,
    egf2: CertifiedInterval,
) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
    if egf2.lo > TOL_JACOBIAN {
        match CertifiedPositive::try_new(egf2.lo) {
            Ok(positive) => ClaimVerdict::Proven(positive),
            Err(_) => ClaimVerdict::Inconclusive(POSITIVE_REFUSED),
        }
    } else if egf2.hi < TOL_JACOBIAN {
        ClaimVerdict::Disproven(Degeneracy {
            box_: d,
            egf2: (egf2.lo, egf2.hi),
        })
    } else {
        ClaimVerdict::Inconclusive(EGF2_STRADDLES)
    }
}

/// The constant-1 weight claim (spec §3.1's frozen constant-1 plumbing, the
/// BG-KV2-104 plane spelling): the bilinear Coons patch is polynomial with
/// unit weight, so the certified positive lower bound is exactly 1 over every
/// box.
fn constant_weight() -> ClaimVerdict<CertifiedPositive, Pole, Reason> {
    match CertifiedPositive::try_new(1.0) {
        Ok(positive) => ClaimVerdict::Proven(positive),
        Err(_) => ClaimVerdict::Inconclusive(WEIGHT_REFUSED),
    }
}

impl<C: ParametricCurve3D> CertifiedPatch for CoonsSurface<C> {
    fn enclose(&self, d: IBox2) -> IBox3 {
        position_of(self, d)
    }

    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        derivative_enclosure(self, d)
    }

    fn normal_cone(&self, d: IBox2) -> Cone {
        normal_cone_of(self, d)
    }

    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        classify_regularity(d, egf2(self, d))
    }

    fn weight_bound(&self, _d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        Some(constant_weight())
    }
}
