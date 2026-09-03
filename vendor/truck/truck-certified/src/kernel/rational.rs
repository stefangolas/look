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

//! The Wave-1 rational half-angle carriers: `CertifiedPatch` for the shim's
//! `RationalCarrier` over its Plane, Sphere, and Cylinder forms
//! (BG-KV2-104-RATCARRIER).
//!
//! **N4 by construction.** No transcendental function appears anywhere in this
//! module: the reparameterizations below are polynomial / interval-rational
//! over the `CertifiedInterval` primitive of `formal/exact.rs`, so no
//! `sin`, `cos`, `atan2`, `exp`, `ln`, `log`, `powf`, or `sqrt` call can
//! appear on any enclosure path. The landed `EnclosureSurface` implementations
//! that use interval `sin`/`cos` from `elementary.rs` are the audit's
//! quarantine population, NOT this module's template.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **N5 homogeneous evaluation.** Position evaluation carries the pair
//! `(numerator, weight)` and never divides inside an enclosure. The weight
//! enclosure's positive lower bound is certified first — the split check is
//! still made (N6 discipline) even where the denominator cannot contain zero
//! by construction — and the single final division happens only after it.
//!
//! **Chart model.** The box coordinates are the rational chart parameters
//! directly. Plane: `X(u, v) = origin + u·u_dir + v·v_dir` (rational of
//! degree 1). Sphere: the stereographic chart
//! `X = center + radius·(2u, 2v, 1 − u² − v²)/(1 + u² + v²)`; the charts of
//! the sphere are the stereographic atlas pieces and the carrier's `domain`
//! names the chart box. Cylinder: the half-angle chart in the angular
//! direction, `X = origin + v·axis + radius·((1 − u²)·e₁ + 2u·e₂)/(1 + u²)`,
//! with the axial coordinate linear in `v`; the seam/wrap is a deck
//! translation, not an event. The cylinder needs an exactly rational
//! orthonormal circle frame `(e₁, e₂)` in the plane orthogonal to the axis;
//! such a frame is only constructible without a transcendental normalization
//! when the axis is exactly a coordinate axis, so a cylinder whose axis is
//! not exactly `±eₓ`, `±e_y`, or `±e_z` is not certified here (Wave-1 scope,
//! see [`admit`]).
//!
//! **Degeneration.** The chart denominators are `1 + u² + v²` and `1 + u²`,
//! both bounded below by 1, and the charts have no finite degeneration point:
//! the sphere's single missing point is the point at infinity of its chart.
//! A query box that reaches that chart degeneration — coordinates so large
//! that the certified arithmetic is no longer finite — cannot be certified,
//! and `regularity` refuses the `Proven` claim over it. Chart switching
//! across the degeneration is §3.4's business (later wave).
//!
//! **Refusal surface.** The frozen `CertifiedPatch` methods cannot carry a
//! `Refusal`. Where a refusal is representable the module refuses through
//! [`admit`], which gates every carrier before certified use: the Cone and
//! Torus rational half-angle forms are Wave-4 work and refuse with the named
//! pending refusal `cone_torus_carrier_packet_pending`; a cylinder axis that
//! is not exactly a coordinate axis refuses with
//! `cylinder_axis_not_coordinate_wave1`. Inside the claim methods the same
//! situations surface as `Inconclusive` reasons carrying the same names (the
//! claim layer's nearest refusal-shaped channel), and the box-valued methods
//! (`enclose`, `derivs`, `normal_cone`) return the module's NaN "no certified
//! patch" markers for carriers that never pass [`admit`], so nothing silently
//! half-implements them.

use crate::formal::exact::CertifiedInterval;
use crate::kernel::evidence::{ClaimVerdict, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::leaf::{CarrierData, RationalCarrier};
use crate::kernel::patch::{
    CertifiedPatch, CertifiedPositive, Cone, Degeneracy, DerivativeEnclosure, IBox2, IBox3, Pole,
    Reason,
};

/// The named pending refusal shared by the Cone and Torus rational forms
/// (their half-angle parameterizations are Wave-4 work: the cone's apex
/// straddling and the torus's rational form).
const CONE_TORUS_PENDING: Reason = "cone_torus_carrier_packet_pending";
/// The named refusal for a cylinder whose axis is not exactly a coordinate
/// axis: an exactly rational orthonormal circle frame would require a
/// normalization that N4 forbids this module from performing.
const CYLINDER_AXIS_PENDING: Reason = "cylinder_axis_not_coordinate_wave1";
/// Claim-layer reason when a certificate cannot be produced over the box
/// (the chart degeneration is reached, or the certified arithmetic is not
/// finite there).
const UNCERTIFIED: Reason = "rational_carrier_not_certifiable_over_box";
/// The widest legal certified half-angle: just under `PI`, so a `Cone` value
/// that certifies nothing can still be represented.
const NEAR_PI: f64 = std::f64::consts::PI * 0.999_999_999_999_999_9;

/// The chart form a carrier evaluates to: the three rational forms of this
/// module, or the pending form that [`admit`] refuses.
#[derive(Debug, Clone, Copy)]
enum Form {
    /// The plane form over `u_dir`/`v_dir`.
    Plane {
        /// A point on the plane.
        origin: [f64; 3],
        /// The `u` direction.
        u_dir: [f64; 3],
        /// The `v` direction.
        v_dir: [f64; 3],
    },
    /// The stereographic sphere form.
    Sphere {
        /// The sphere center.
        center: [f64; 3],
        /// The sphere radius.
        radius: f64,
    },
    /// The half-angle cylinder form.
    Cylinder {
        /// A point on the cylinder axis.
        origin: [f64; 3],
        /// The exactly-coordinate unit cylinder axis.
        axis: [f64; 3],
        /// An exactly rational unit circle frame vector.
        e1: [f64; 3],
        /// An exactly rational unit circle frame vector.
        e2: [f64; 3],
        /// The cylinder radius.
        radius: f64,
    },
    /// A carrier the Wave-1 module does not certify, with its refusal name.
    Pending(Reason),
}

/// Classify a carrier into its chart form. The Cone and Torus forms (and a
/// cylinder whose axis is not exactly a coordinate axis) are pending.
fn form(carrier: &RationalCarrier) -> Form {
    match carrier.data {
        CarrierData::Plane {
            origin,
            u_dir,
            v_dir,
        } => Form::Plane {
            origin,
            u_dir,
            v_dir,
        },
        CarrierData::Sphere { center, radius } => Form::Sphere { center, radius },
        CarrierData::Cylinder {
            origin,
            axis,
            radius,
            height: _,
        } => match circle_frame(axis) {
            Some((e1, e2)) => Form::Cylinder {
                origin,
                axis,
                e1,
                e2,
                radius,
            },
            None => Form::Pending(CYLINDER_AXIS_PENDING),
        },
        CarrierData::Cone { .. } | CarrierData::Torus { .. } => Form::Pending(CONE_TORUS_PENDING),
    }
}

/// An exactly rational orthonormal circle frame `(e1, e2)` for the plane
/// orthogonal to a coordinate axis. Any other axis would need a
/// transcendental normalization to frame, which N4 forbids.
fn circle_frame(axis: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
    let x = axis[0];
    let y = axis[1];
    let z = axis[2];
    if x == 0.0 && y == 0.0 && (z == 1.0 || z == -1.0) {
        return Some(([1.0, 0.0, 0.0], [0.0, z, 0.0]));
    }
    if y == 0.0 && z == 0.0 && (x == 1.0 || x == -1.0) {
        return Some(([0.0, 1.0, 0.0], [0.0, 0.0, x]));
    }
    if x == 0.0 && z == 0.0 && (y == 1.0 || y == -1.0) {
        return Some(([0.0, 0.0, 1.0], [y, 0.0, 0.0]));
    }
    None
}

/// A three-component interval vector.
type V3 = [CertifiedInterval; 3];

/// The interval point of a scalar.
fn point(x: f64) -> CertifiedInterval {
    CertifiedInterval::point(x)
}

/// The certified square of an interval: `{x² : x ∈ i}` with outward rounding.
/// An interval multiplied by itself through [`CertifiedInterval::mul`] would
/// treat the two factors as independent and widen across zero; the square
/// range is monotone on each side of zero and is evaluated endpoint-wise.
fn square(i: &CertifiedInterval) -> CertifiedInterval {
    let lo2 = i.lo * i.lo;
    let hi2 = i.hi * i.hi;
    if i.lo >= 0.0 {
        CertifiedInterval {
            lo: lo2.next_down(),
            hi: hi2.next_up(),
        }
    } else if i.hi <= 0.0 {
        CertifiedInterval {
            lo: hi2.next_down(),
            hi: lo2.next_up(),
        }
    } else {
        CertifiedInterval {
            lo: 0.0,
            hi: (if lo2 >= hi2 { lo2 } else { hi2 }).next_up(),
        }
    }
}

/// The two axis intervals of a parameter box.
fn axes(d: IBox2) -> (CertifiedInterval, CertifiedInterval) {
    (
        CertifiedInterval {
            lo: d.lo[0],
            hi: d.hi[0],
        },
        CertifiedInterval {
            lo: d.lo[1],
            hi: d.hi[1],
        },
    )
}

/// An `IBox3` from three interval components.
fn box3(x: CertifiedInterval, y: CertifiedInterval, z: CertifiedInterval) -> IBox3 {
    IBox3 {
        lo: [x.lo, y.lo, z.lo],
        hi: [x.hi, y.hi, z.hi],
    }
}

/// The interval point vector of a `[f64; 3]`.
fn point_v3(p: [f64; 3]) -> V3 {
    [point(p[0]), point(p[1]), point(p[2])]
}

/// An `IBox3` from an interval vector.
fn v3_to_ibox3(v: &V3) -> IBox3 {
    IBox3 {
        lo: [v[0].lo, v[1].lo, v[2].lo],
        hi: [v[0].hi, v[1].hi, v[2].hi],
    }
}

/// The interval dot product of two interval vectors.
fn dot3(a: &V3, b: &V3) -> CertifiedInterval {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

/// The exact cross product of two point vectors.
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The module's "no certified patch" marker box: NaN bounds mean no certified
/// enclosure exists over this box (used only for carriers that never pass
/// [`admit`]).
fn no_patch_box() -> IBox3 {
    IBox3 {
        lo: [f64::NAN; 3],
        hi: [f64::NAN; 3],
    }
}

/// The module's "no certified patch" derivative marker.
fn no_patch_derivs() -> DerivativeEnclosure {
    DerivativeEnclosure {
        su: no_patch_box(),
        sv: no_patch_box(),
    }
}

/// The module's "no certified patch" normal-cone marker.
fn no_patch_cone() -> Cone {
    Cone {
        axis: [f64::NAN; 3],
        half_angle: f64::NAN,
    }
}

/// The widest-legal cone used when a normal cone cannot be meaningfully
/// certified over the box (it certifies nothing useful).
fn near_pi_cone() -> Cone {
    Cone {
        axis: [1.0, 0.0, 0.0],
        half_angle: NEAR_PI,
    }
}

// ---------------------------------------------------------------------------
// Plane
// ---------------------------------------------------------------------------

/// The certified plane position enclosure over `d`.
fn plane_position(origin: [f64; 3], u_dir: [f64; 3], v_dir: [f64; 3], d: IBox2) -> IBox3 {
    let (u, v) = axes(d);
    let x = point(origin[0])
        .add(&point(u_dir[0]).mul(&u))
        .add(&point(v_dir[0]).mul(&v));
    let y = point(origin[1])
        .add(&point(u_dir[1]).mul(&u))
        .add(&point(v_dir[1]).mul(&v));
    let z = point(origin[2])
        .add(&point(u_dir[2]).mul(&u))
        .add(&point(v_dir[2]).mul(&v));
    box3(x, y, z)
}

/// The certified `EG − F²` enclosure of the plane: a constant over every box.
fn plane_egf2(u_dir: [f64; 3], v_dir: [f64; 3]) -> CertifiedInterval {
    let u = point_v3(u_dir);
    let v = point_v3(v_dir);
    let e = dot3(&u, &u);
    let g = dot3(&v, &v);
    let f = dot3(&u, &v);
    e.mul(&g).sub(&square(&f))
}

// ---------------------------------------------------------------------------
// Sphere
// ---------------------------------------------------------------------------

/// The unit-radius rational sphere point `q(u, v) = P/w` over the box, with
/// `P = (2u, 2v, 1 − u² − v²)` and `w = 1 + u² + v²`. The single division by
/// `w` happens here, at the end (N5); `None` when the division is not
/// certifiable (the weight enclosure ever containing zero or a non-finite
/// quotient — impossible on a finite chart box, the check is still made, N6).
fn unit_sphere_q(
    u: &CertifiedInterval,
    v: &CertifiedInterval,
    u2: &CertifiedInterval,
    v2: &CertifiedInterval,
    weight: &CertifiedInterval,
) -> Option<V3> {
    let px = point(2.0).mul(u);
    let py = point(2.0).mul(v);
    let pz = point(1.0).sub(u2).sub(v2);
    Some([px.div(weight)?, py.div(weight)?, pz.div(weight)?])
}

/// The certified sphere position enclosure over `d`.
fn sphere_position(center: [f64; 3], radius: f64, d: IBox2) -> IBox3 {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let weight = point(1.0).add(&u2).add(&v2);
    let q = match unit_sphere_q(&u, &v, &u2, &v2, &weight) {
        Some(q) => q,
        None => return no_patch_box(),
    };
    let x = point(center[0]).add(&point(radius).mul(&q[0]));
    let y = point(center[1]).add(&point(radius).mul(&q[1]));
    let z = point(center[2]).add(&point(radius).mul(&q[2]));
    box3(x, y, z)
}

/// The certified first-derivative enclosures of the sphere over `d`, from the
/// hand-differentiated closed forms `q_u`/`q_v` divided by `w²` once at the
/// end.
fn sphere_derivs(radius: f64, d: IBox2) -> DerivativeEnclosure {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let v2 = square(&v);
    let weight = point(1.0).add(&u2).add(&v2);
    let weight2 = square(&weight);
    let ru0 = point(2.0).mul(&point(1.0).sub(&u2).add(&v2));
    let ru1 = point(-4.0).mul(&u).mul(&v);
    let ru2 = point(-4.0).mul(&u);
    let rv0 = point(-4.0).mul(&u).mul(&v);
    let rv1 = point(2.0).mul(&point(1.0).add(&u2).sub(&v2));
    let rv2 = point(-4.0).mul(&v);
    let divided = |n: &CertifiedInterval| -> Option<CertifiedInterval> {
        let q = n.div(&weight2)?;
        Some(q.mul(&point(radius)))
    };
    match (
        divided(&ru0),
        divided(&ru1),
        divided(&ru2),
        divided(&rv0),
        divided(&rv1),
        divided(&rv2),
    ) {
        (Some(su0), Some(su1), Some(su2), Some(sv0), Some(sv1), Some(sv2)) => DerivativeEnclosure {
            su: box3(su0, su1, su2),
            sv: box3(sv0, sv1, sv2),
        },
        _ => no_patch_derivs(),
    }
}

/// The certified `EG − F²` enclosure of the sphere over `d`. The sphere chart
/// is conformal with `E = G = 4·r²/w²` and `F = 0` exactly, so
/// `EG − F² = 16·r⁴/w⁴`, a positive rational function on the chart. The
/// collapsed identity is the hand algebra of the derivative closed forms; the
/// interval evaluation of the raw derivative-enclosure products would be so
/// dependency-widened that no finite chart box would certify (N7 keeps the
/// two-stage discipline: tight form first).
fn sphere_egf2(radius: f64, d: IBox2) -> Option<CertifiedInterval> {
    let (u, v) = axes(d);
    let weight = point(1.0).add(&square(&u)).add(&square(&v));
    let weight4 = square(&square(&weight));
    let r2 = radius * radius;
    point(16.0 * r2 * r2).div(&weight4)
}

// ---------------------------------------------------------------------------
// Cylinder
// ---------------------------------------------------------------------------

/// The unit-radius half-angle radial point `q(u) = P/w` of the cylinder over
/// the box, with `P = (1 − u²)·e1 + 2u·e2` and `w = 1 + u²`. Single division
/// by `w` at the end (N5).
fn unit_radial_q(
    u: &CertifiedInterval,
    u2: &CertifiedInterval,
    e1: [f64; 3],
    e2: [f64; 3],
    weight: &CertifiedInterval,
) -> Option<V3> {
    let c = point(1.0).sub(u2);
    let s = point(2.0).mul(u);
    let px = point(e1[0]).mul(&c).add(&point(e2[0]).mul(&s));
    let py = point(e1[1]).mul(&c).add(&point(e2[1]).mul(&s));
    let pz = point(e1[2]).mul(&c).add(&point(e2[2]).mul(&s));
    Some([px.div(weight)?, py.div(weight)?, pz.div(weight)?])
}

/// The certified cylinder position enclosure over `d`.
fn cylinder_position(
    origin: [f64; 3],
    axis: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    radius: f64,
    d: IBox2,
) -> IBox3 {
    let (u, v) = axes(d);
    let u2 = square(&u);
    let weight = point(1.0).add(&u2);
    let q = match unit_radial_q(&u, &u2, e1, e2, &weight) {
        Some(q) => q,
        None => return no_patch_box(),
    };
    let axial = |i: usize| point(origin[i]).add(&v.mul(&point(axis[i])));
    let x = axial(0).add(&point(radius).mul(&q[0]));
    let y = axial(1).add(&point(radius).mul(&q[1]));
    let z = axial(2).add(&point(radius).mul(&q[2]));
    box3(x, y, z)
}

/// The certified first-derivative enclosures of the cylinder over `d`: the
/// angular derivative `X_u = radius·(−4u·e1 + 2·(1 − u²)·e2)/w²` from the
/// hand-differentiated closed form, and the constant axial derivative
/// `X_v = axis`.
fn cylinder_derivs(
    axis: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    radius: f64,
    d: IBox2,
) -> DerivativeEnclosure {
    let (u, _v) = axes(d);
    let u2 = square(&u);
    let weight = point(1.0).add(&u2);
    let weight2 = square(&weight);
    let numerator = |i: usize| -> CertifiedInterval {
        let a = point(-4.0).mul(&u).mul(&point(e1[i]));
        let b = point(2.0).mul(&point(1.0).sub(&u2)).mul(&point(e2[i]));
        a.add(&b)
    };
    let divided = |n: &CertifiedInterval| -> Option<CertifiedInterval> {
        let q = n.div(&weight2)?;
        Some(q.mul(&point(radius)))
    };
    let su0 = divided(&numerator(0));
    let su1 = divided(&numerator(1));
    let su2 = divided(&numerator(2));
    match (su0, su1, su2) {
        (Some(su0), Some(su1), Some(su2)) => DerivativeEnclosure {
            su: box3(su0, su1, su2),
            sv: v3_to_ibox3(&point_v3(axis)),
        },
        _ => no_patch_derivs(),
    }
}

/// The certified `EG − F²` enclosure of the cylinder over `d`. The half-angle
/// chart is orthogonal with `E = 4·r²/w²`, `G = 1`, `F = 0` exactly (the axis
/// is an exactly unit coordinate axis), so `EG − F² = 4·r²/w²`.
fn cylinder_egf2(radius: f64, d: IBox2) -> Option<CertifiedInterval> {
    let (u, _v) = axes(d);
    let weight = point(1.0).add(&square(&u));
    let weight2 = square(&weight);
    point(4.0 * radius * radius).div(&weight2)
}

// ---------------------------------------------------------------------------
// Normal cones
// ---------------------------------------------------------------------------

/// A certified normal cone for a single constant direction `w`: the axis is
/// the dominant coordinate direction and the half-angle is the certified
/// rational bound `θ ≤ 2·atan(t) ≤ 2·t` with `t = |w − (w·a)·a|/(|w| + w·a)`
/// bounded above by the `1`-norm over the positive `w·a` split (no
/// transcendental appears).
fn plane_normal_cone(u_dir: [f64; 3], v_dir: [f64; 3]) -> Cone {
    let w = cross3(u_dir, v_dir);
    let a0 = w[0].abs();
    let a1 = w[1].abs();
    let a2 = w[2].abs();
    if a0 + a1 + a2 == 0.0 {
        return near_pi_cone();
    }
    let (index, magnitude) = if a0 >= a1 && a0 >= a2 {
        (0usize, a0)
    } else if a1 >= a2 {
        (1usize, a1)
    } else {
        (2usize, a2)
    };
    let mut axis = [0.0; 3];
    axis[index] = if w[index] >= 0.0 { 1.0 } else { -1.0 };
    let other = match index {
        0 => a1 + a2,
        1 => a0 + a2,
        _ => a0 + a1,
    };
    let tangent = other / magnitude;
    let half = if 2.0 * tangent < NEAR_PI {
        2.0 * tangent
    } else {
        NEAR_PI
    };
    Cone {
        axis,
        half_angle: half,
    }
}

/// The radial direction of the sphere chart at the box midpoint: a
/// mathematically unit vector whose `f64` evaluation stays within rounding of
/// unit (no normalization call is needed).
fn radial_axis(u0: f64, v0: f64) -> Option<[f64; 3]> {
    let s = u0 * u0 + v0 * v0;
    let d = 1.0 + s;
    let q = [2.0 * u0 / d, 2.0 * v0 / d, (1.0 - s) / d];
    if q[0].is_finite() && q[1].is_finite() && q[2].is_finite() {
        Some(q)
    } else {
        None
    }
}

/// A certified normal cone for the sphere over `d`: the radial direction at
/// the box midpoint, with a certified half-angle. The radial directions of
/// the chart satisfy `θ ≤ (π/2)·chord` with `chord ≤ 2·|Δ|` over the box, so
/// `half-angle ≤ (π/2)·(|u-span| + |v-span|)` is certified; the bound is
/// clamped to the widest legal cone where the box reaches the chart
/// degeneration (such a box is not regular-certifiable anyway).
fn sphere_normal_cone(d: IBox2) -> Cone {
    let mid = [(d.lo[0] + d.hi[0]) * 0.5, (d.lo[1] + d.hi[1]) * 0.5];
    let axis = match radial_axis(mid[0], mid[1]) {
        Some(axis) => axis,
        None => return near_pi_cone(),
    };
    let bound = std::f64::consts::FRAC_PI_2 * ((d.hi[0] - d.lo[0]) + (d.hi[1] - d.lo[1]));
    let half_angle = if bound < NEAR_PI { bound } else { NEAR_PI };
    Cone { axis, half_angle }
}

/// The radial (circle) direction of the cylinder chart at `u0`: a
/// mathematically unit vector in the `(e1, e2)` frame.
fn circle_axis(u0: f64, e1: [f64; 3], e2: [f64; 3]) -> Option<[f64; 3]> {
    if !u0.is_finite() {
        return None;
    }
    let d = 1.0 + u0 * u0;
    let c = (1.0 - u0 * u0) / d;
    let s = (2.0 * u0) / d;
    let q = [
        c * e1[0] + s * e2[0],
        c * e1[1] + s * e2[1],
        c * e1[2] + s * e2[2],
    ];
    if q[0].is_finite() && q[1].is_finite() && q[2].is_finite() {
        Some(q)
    } else {
        None
    }
}

/// A certified normal cone for the cylinder over `d`: the radial direction at
/// the box midpoint `u0`, with a certified half-angle. The cylinder normals
/// vary only in `u`, and `tan((θ − θ0)/2) = (u − u0)/(1 + u·u0)` gives the
/// certified bound `half-angle ≤ 2·max|u − u0|/min(1 + u·u0)`; clamped to the
/// widest legal cone where the box crosses the chart's antipodal direction.
fn cylinder_normal_cone(d: IBox2, e1: [f64; 3], e2: [f64; 3]) -> Cone {
    let u0 = (d.lo[0] + d.hi[0]) * 0.5;
    let axis = match circle_axis(u0, e1, e2) {
        Some(axis) => axis,
        None => return near_pi_cone(),
    };
    let f_lo = 1.0 + d.lo[0] * u0;
    let f_hi = 1.0 + d.hi[0] * u0;
    if f_lo <= 0.0 || f_hi <= 0.0 {
        return near_pi_cone();
    }
    let farthest = (u0 - d.lo[0]).max(d.hi[0] - u0);
    let tangent = farthest / f_lo.min(f_hi);
    let half_angle = if 2.0 * tangent < NEAR_PI {
        2.0 * tangent
    } else {
        NEAR_PI
    };
    Cone { axis, half_angle }
}

// ---------------------------------------------------------------------------
// Claim classification
// ---------------------------------------------------------------------------

/// A `CertifiedPositive` construction from a certified positive lower bound;
/// `None` is unreachable for the bounds passed here (finite and strictly
/// positive by the calling claim) and is mapped to an inconclusive claim.
fn positive_bound(lo: f64) -> Option<CertifiedPositive> {
    CertifiedPositive::try_new(lo).ok()
}

/// Classify an `EG − F²` enclosure into the regularity claim: `Proven` iff
/// the certified lower bound is strictly positive, `Disproven` with a
/// degeneracy witness iff the enclosure provably excludes the positive
/// half-line, and `Inconclusive` otherwise (the degeneration is reached or
/// the arithmetic is not finite there).
fn classify_egf2(
    d: IBox2,
    egf2: Option<CertifiedInterval>,
) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
    match egf2 {
        Some(enclosure) if enclosure.lo > 0.0 && enclosure.is_finite() => {
            match positive_bound(enclosure.lo) {
                Some(bound) => ClaimVerdict::Proven(bound),
                None => ClaimVerdict::Inconclusive(UNCERTIFIED),
            }
        }
        Some(enclosure) if enclosure.hi <= 0.0 => ClaimVerdict::Disproven(Degeneracy {
            box_: d,
            egf2: (enclosure.lo, enclosure.hi),
        }),
        _ => ClaimVerdict::Inconclusive(UNCERTIFIED),
    }
}

/// Classify a weight (denominator) enclosure into the weight-bound claim:
/// `Proven` with the certified positive lower bound iff the enclosure is
/// strictly positive and finite, `Disproven` with a pole witness iff it
/// provably excludes the positive half-line, and `Inconclusive` otherwise.
fn classify_weight(
    d: IBox2,
    weight: &CertifiedInterval,
) -> ClaimVerdict<CertifiedPositive, Pole, Reason> {
    if weight.lo > 0.0 && weight.is_finite() {
        match positive_bound(weight.lo) {
            Some(bound) => ClaimVerdict::Proven(bound),
            None => ClaimVerdict::Inconclusive(UNCERTIFIED),
        }
    } else if weight.hi <= 0.0 {
        ClaimVerdict::Disproven(Pole {
            box_: d,
            w: (weight.lo, weight.hi),
        })
    } else {
        ClaimVerdict::Inconclusive(UNCERTIFIED)
    }
}

// ---------------------------------------------------------------------------
// The §3.1 implementation
// ---------------------------------------------------------------------------

impl CertifiedPatch for RationalCarrier {
    fn enclose(&self, d: IBox2) -> IBox3 {
        match form(self) {
            Form::Plane {
                origin,
                u_dir,
                v_dir,
            } => plane_position(origin, u_dir, v_dir, d),
            Form::Sphere { center, radius } => sphere_position(center, radius, d),
            Form::Cylinder {
                origin,
                axis,
                e1,
                e2,
                radius,
            } => cylinder_position(origin, axis, e1, e2, radius, d),
            Form::Pending(_) => no_patch_box(),
        }
    }

    fn derivs(&self, d: IBox2) -> DerivativeEnclosure {
        match form(self) {
            Form::Plane { u_dir, v_dir, .. } => DerivativeEnclosure {
                su: v3_to_ibox3(&point_v3(u_dir)),
                sv: v3_to_ibox3(&point_v3(v_dir)),
            },
            Form::Sphere { radius, .. } => sphere_derivs(radius, d),
            Form::Cylinder {
                axis,
                e1,
                e2,
                radius,
                ..
            } => cylinder_derivs(axis, e1, e2, radius, d),
            Form::Pending(_) => no_patch_derivs(),
        }
    }

    fn normal_cone(&self, d: IBox2) -> Cone {
        match form(self) {
            Form::Plane { u_dir, v_dir, .. } => plane_normal_cone(u_dir, v_dir),
            Form::Sphere { .. } => sphere_normal_cone(d),
            Form::Cylinder { e1, e2, .. } => cylinder_normal_cone(d, e1, e2),
            Form::Pending(_) => no_patch_cone(),
        }
    }

    fn regularity(&self, d: IBox2) -> ClaimVerdict<CertifiedPositive, Degeneracy, Reason> {
        match form(self) {
            Form::Plane { u_dir, v_dir, .. } => classify_egf2(d, Some(plane_egf2(u_dir, v_dir))),
            Form::Sphere { radius, .. } => classify_egf2(d, sphere_egf2(radius, d)),
            Form::Cylinder { radius, .. } => classify_egf2(d, cylinder_egf2(radius, d)),
            Form::Pending(reason) => ClaimVerdict::Inconclusive(reason),
        }
    }

    fn weight_bound(&self, d: IBox2) -> Option<ClaimVerdict<CertifiedPositive, Pole, Reason>> {
        match form(self) {
            Form::Plane { .. } => Some(classify_weight(d, &point(1.0))),
            Form::Sphere { .. } => {
                let (u, v) = axes(d);
                let weight = point(1.0).add(&square(&u)).add(&square(&v));
                Some(classify_weight(d, &weight))
            }
            Form::Cylinder { .. } => {
                let (u, _v) = axes(d);
                let weight = point(1.0).add(&square(&u));
                Some(classify_weight(d, &weight))
            }
            Form::Pending(reason) => Some(ClaimVerdict::Inconclusive(reason)),
        }
    }
}

/// The Wave-1 admission gate for a carrier: Plane, Sphere, and Cylinder
/// (coordinate-axis) carriers certify over their chart boxes; the Cone and
/// Torus rational forms and non-coordinate cylinder axes refuse with the
/// named pending refusal so nothing silently half-implements them.
// `Refusal` carries `Option<PartialGraph>` per the frozen §2 shape; boxing it
// would deviate from the contract, so the refusing constructor is allowed the
// large-Err lint (BG-KV2-000-CONTRACT).
#[allow(clippy::result_large_err)]
pub fn admit(carrier: &RationalCarrier) -> Result<(), Refusal> {
    match form(carrier) {
        Form::Pending(name) => Err(Refusal::new(
            RefusalKind::CarrierSingularity,
            RefusalEvidence::Predicate {
                name,
                detail: format!(
                    "the {name} rational carrier is not certified by the Wave-1 \
                     rational-carrier module (rational half-angle parameterizations)"
                ),
            },
        )),
        Form::Plane { .. } | Form::Sphere { .. } | Form::Cylinder { .. } => Ok(()),
    }
}
