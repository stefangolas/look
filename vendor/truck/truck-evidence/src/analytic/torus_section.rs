//! BG-ANA-001-TOR: plane Ã— torus sections â€” the exact circle loci of the
//! Villarceau factorization certificate (TOR-A).
//!
//! A plane cuts a ring torus in exactly one of the N1 families of the
//! frontier-reviewed classification (theory v2 Â§1; the section statement in
//! `docs/TORUS_SECTION_THEORY.md`): **axis-perpendicular** planes cut two
//! coaxial circles (`Empty` / one double tangent circle on the boundary),
//! **axial** planes (through the axis and the centre) cut two profile circles
//! of radius `r`, and central oblique **Villarceau** planes cut two circles of
//! radius `R`. Every other plane restricts to an **irreducible quartic** (the
//! spiric section) that this cell never approximates: it routes to the traced
//! path through a typed refusal.
//!
//! # The runtime-factorization certificate (the point of the module)
//!
//! The classification is deliberately NOT trusted. For every circle-emitting
//! instance the module
//!
//! 1. restricts the torus implicit `Î¦(q) = (qÂ·q + RÂ² âˆ’ rÂ²)Â² âˆ’ 4RÂ²(qÂ·q âˆ’
//!    (qÂ·a)Â²)` (the scale-invariant T-form, `K = aÂ·a = 1` on the canonical
//!    z-axis torus) to the plane, in the orthonormal in-plane frame built by
//!    [`frame_for_normal`], forming the section quartic `Q(u, v)` with
//!    **exact rational** coefficients (section-theory Lemma 1: the
//!    orthonormal coordinates cancel the cross terms, `qÂ·q = dÂ² + uÂ² + vÂ²`);
//! 2. forms the candidate circle conics `Câ‚`, `Câ‚‚` from the recovered
//!    centres/radii â€” over â„š, or over the certified quadratic extension
//!    `â„š(âˆšk)` recorded on the factors when the radius is `R Â± âˆšk` (the
//!    axis-perpendicular `ÏÂ± = R Â± âˆš(rÂ² âˆ’ dÂ²)` family);
//! 3. multiplies `Câ‚Â·Câ‚‚` back and requires `Q = Câ‚Â·Câ‚‚` **coefficient-exact**.
//!
//! The identity `Q = Câ‚Â·Câ‚‚` *is* the proof per instance. When the exact
//! arithmetic cannot certify the identity â€” the carrier data are not exact
//! rationals, an extension beyond `â„š(âˆšk)` would be needed, or the rounding of
//! a non-dyadic normal broke the exact relation â€” the module refuses to emit
//! and routes to the tracer, exactly as if the section were a residual
//! quartic. An emitted circle locus is always backed by a checked identity;
//! there is no float predicate and no approximate circle fitting anywhere
//! (BG-ANA-002, H-6).
//!
//! # Canonical carrier convention
//!
//! Like the sibling plane Ã— quadric cells this cell consumes the *canonical*
//! carriers: `truck-geometry`'s `Torus` is the ring torus whose symmetry axis
//! is the z axis through `center` (radius `R > r`), and the plane is any
//! `Plane`. A placed (rotated) torus is handled by exact conjugation to this
//! canonical representative upstream, exactly as the N1 predicates transform
//! covariantly under rigid placements (placement isometry: the factorization
//! structure is invariant, only the coefficients transform). Horn/spindle
//! tori (`r â‰¥ R`) refuse typed â€” the landed `formal/torus.rs` rule carried
//! forward.
//!
//! # Exact predicates, exact values
//!
//! Every decision and every certificate coefficient is computed in exact
//! rational arithmetic over the carrier's `f64` data (each finite `f64` is a
//! dyadic rational, so the conversion is exact). The *emitted geometry* is
//! the closed form evaluated in `f64`; the obligation BG-ANA-002 places on
//! the cell is "lies on both carriers to machine precision", asserted in the
//! conformance tests with an H-3-commented slack. A decision the exact
//! arithmetic cannot certify is a typed refusal, never a guess.

#![deny(clippy::unwrap_used)]

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::f64::consts::TAU;

use truck_base::cgmath64::{Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Prop, PropMap, Refusal,
    Truth,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Plane, Torus, UnitCircle};

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve, PlacedCircle};

// ---------------------------------------------------------------------------
// Exact rational arithmetic over the carrier f64s
// ---------------------------------------------------------------------------

/// An exact rational `n / d` in reduced form, `d > 0`. Every finite `f64` is
/// a dyadic rational, so [`Rat::from_f64`] converts exactly; all arithmetic
/// is checked (`None` on overflow) so a certificate step that would leave the
/// representable range refuses rather than wrapping silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rat {
    n: i128,
    d: i128,
}

impl Rat {
    const ZERO: Self = Self { n: 0, d: 1 };
    const ONE: Self = Self { n: 1, d: 1 };

    /// Reduces `n / d` to canonical form (`d > 0`).
    fn reduce(n: i128, d: i128) -> Option<Self> {
        if d == 0 {
            return None;
        }
        if n == 0 {
            return Some(Self::ZERO);
        }
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd(n, d);
        Some(Self { n: n / g, d: d / g })
    }

    /// The exact value of a small integer.
    fn from_i64(v: i64) -> Self {
        match Self::reduce(v as i128, 1) {
            Some(r) => r,
            None => Self::ZERO,
        }
    }

    /// The exact dyadic value of a finite `f64`. `None` for a non-finite
    /// value or one whose power-of-two denominator exceeds the `i128` range
    /// (only subnormal-scale coordinates).
    fn from_f64(v: f64) -> Option<Self> {
        if v == 0.0 {
            return Some(Self::ZERO);
        }
        if !v.is_finite() {
            return None;
        }
        let bits = v.to_bits();
        let negative = bits >> 63 == 1;
        let exp_bits = ((bits >> 52) & 0x7ff) as i32;
        let frac = bits & ((1u64 << 52) - 1);
        let (mant, exponent): (i128, i32) = if exp_bits == 0 {
            (frac as i128, -1074)
        } else {
            ((frac | (1u64 << 52)) as i128, exp_bits - 1023 - 52)
        };
        if exponent >= 0 {
            let shifted = mant.checked_shl(exponent as u32)?;
            let n = if negative { -shifted } else { shifted };
            Self::reduce(n, 1)
        } else {
            let shift = (-exponent) as u32;
            if shift >= 127 {
                return None;
            }
            let n = if negative { -mant } else { mant };
            Self::reduce(n, 1i128 << shift)
        }
    }

    fn is_zero(self) -> bool {
        self.n == 0
    }

    fn neg(self) -> Option<Self> {
        Some(Self {
            n: -self.n,
            d: self.d,
        })
    }

    fn add(self, o: Self) -> Option<Self> {
        let g = gcd(self.d, o.d);
        let d1 = self.d / g;
        let d2 = o.d / g;
        let n = self.n.checked_mul(d2)?.checked_add(o.n.checked_mul(d1)?)?;
        Self::reduce(n, self.d.checked_mul(d2)?)
    }

    fn sub(self, o: Self) -> Option<Self> {
        self.add(o.neg()?)
    }

    fn mul(self, o: Self) -> Option<Self> {
        Self::reduce(self.n.checked_mul(o.n)?, self.d.checked_mul(o.d)?)
    }

    fn sq(self) -> Option<Self> {
        self.mul(self)
    }

    fn div(self, o: Self) -> Option<Self> {
        if o.is_zero() {
            return None;
        }
        Self::reduce(self.n.checked_mul(o.d)?, self.d.checked_mul(o.n)?)
    }

    /// The exact square root, when the value is a perfect rational square.
    fn sqrt_exact(self) -> Option<Self> {
        if self.n < 0 {
            return None;
        }
        let root_n = isqrt(self.n)?;
        let root_d = isqrt(self.d)?;
        if root_n.checked_mul(root_n)? != self.n || root_d.checked_mul(root_d)? != self.d {
            return None;
        }
        Self::reduce(root_n, root_d)
    }

    /// Orders two rationals exactly. `None` when the exact comparison would
    /// leave the fixed-width digit range (only pathologically large carrier
    /// values); the caller treats that as a certificate refusal.
    fn cmp(self, o: Self) -> Option<Ordering> {
        if self.n == o.n && self.d == o.d {
            return Some(Ordering::Equal);
        }
        let sa = signum(self.n);
        let sb = signum(o.n);
        if sa != sb {
            return Some(sa.cmp(&sb));
        }
        if sa == 0 {
            return Some(Ordering::Equal);
        }
        let mag = cmp_magnitude(self, o)?;
        Some(if sa > 0 { mag } else { mag.reverse() })
    }

    fn to_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

fn signum(n: i128) -> i32 {
    if n > 0 {
        1
    } else if n < 0 {
        -1
    } else {
        0
    }
}

/// Binary Euclidean algorithm over `i128` magnitudes.
fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The integer square root of a non-negative `i128`, or `None` when `n` is
/// not a perfect square.
fn isqrt(n: i128) -> Option<i128> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return Some(0);
    }
    let mut x = (n as f64).sqrt() as i128;
    while x > 0 && x.checked_mul(x).is_none_or(|s| s > n) {
        x -= 1;
    }
    loop {
        let next = x.checked_add(1)?;
        match next.checked_mul(next) {
            Some(s) if s <= n => x = next,
            _ => break,
        }
    }
    if x.checked_mul(x)? == n {
        Some(x)
    } else {
        None
    }
}

/// Compares two positive rationals exactly. Integer parts first; when they
/// agree, the fractional parts are compared through their reciprocals â€” the
/// Euclidean recursion shrinks the operands every step, so nothing overflows
/// (`qÂ·d â‰¤ n` at each step).
fn cmp_magnitude(a: Rat, b: Rat) -> Option<Ordering> {
    Some(cmp_pos(a.n, a.d, b.n, b.d))
}

/// `n1/d1` versus `n2/d2`, both positive.
fn cmp_pos(n1: i128, d1: i128, n2: i128, d2: i128) -> Ordering {
    let q1 = n1 / d1;
    let r1 = n1 % d1;
    let q2 = n2 / d2;
    let r2 = n2 % d2;
    if q1 != q2 {
        return q1.cmp(&q2);
    }
    if r1 == 0 && r2 == 0 {
        return Ordering::Equal;
    }
    if r1 == 0 {
        return Ordering::Less;
    }
    if r2 == 0 {
        return Ordering::Greater;
    }
    // r1/d1 vs r2/d2 equals the reverse of d1/r1 vs d2/r2.
    cmp_pos(d1, r1, d2, r2).reverse()
}

/// An exact rational vector.
#[derive(Clone, Copy, Debug)]
struct RatVec {
    x: Rat,
    y: Rat,
    z: Rat,
}

impl RatVec {
    fn from_vec3(v: Vector3) -> Option<Self> {
        Some(Self {
            x: Rat::from_f64(v.x)?,
            y: Rat::from_f64(v.y)?,
            z: Rat::from_f64(v.z)?,
        })
    }

    fn from_point(p: Point3) -> Option<Self> {
        Some(Self {
            x: Rat::from_f64(p.x)?,
            y: Rat::from_f64(p.y)?,
            z: Rat::from_f64(p.z)?,
        })
    }

    fn from_points(a: Point3, b: Point3) -> Option<Self> {
        Some(Self {
            x: Rat::from_f64(a.x)?.sub(Rat::from_f64(b.x)?)?,
            y: Rat::from_f64(a.y)?.sub(Rat::from_f64(b.y)?)?,
            z: Rat::from_f64(a.z)?.sub(Rat::from_f64(b.z)?)?,
        })
    }

    fn cross(self, o: Self) -> Option<Self> {
        Some(Self {
            x: self.y.mul(o.z)?.sub(self.z.mul(o.y)?)?,
            y: self.z.mul(o.x)?.sub(self.x.mul(o.z)?)?,
            z: self.x.mul(o.y)?.sub(self.y.mul(o.x)?)?,
        })
    }

    fn dot(self, o: Self) -> Option<Rat> {
        self.x
            .mul(o.x)?
            .add(self.y.mul(o.y)?)?
            .add(self.z.mul(o.z)?)
    }

    fn scale(self, s: Rat) -> Option<Self> {
        Some(Self {
            x: self.x.mul(s)?,
            y: self.y.mul(s)?,
            z: self.z.mul(s)?,
        })
    }

    fn is_zero(self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }

    fn to_vec3(self) -> Vector3 {
        Vector3::new(self.x.to_f64(), self.y.to_f64(), self.z.to_f64())
    }
}

// ---------------------------------------------------------------------------
// The certificate's polynomial ring over â„š(âˆšk)
// ---------------------------------------------------------------------------

/// `a + bÂ·âˆšk`: a coefficient of the certificate's ring. `k` is fixed per
/// instance (the radicand `rÂ² âˆ’ dÂ²` of the axis-perpendicular radii, or the
/// trivial radicand `1` when every radius is rational) and is threaded
/// through the ring products rather than stored per element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fx {
    a: Rat,
    b: Rat,
}

impl Fx {
    fn rational(r: Rat) -> Self {
        Self { a: r, b: Rat::ZERO }
    }

    fn is_zero(self) -> bool {
        self.a.is_zero() && self.b.is_zero()
    }

    fn neg(self) -> Option<Self> {
        Some(Self {
            a: self.a.neg()?,
            b: self.b.neg()?,
        })
    }

    fn add(self, o: Self) -> Option<Self> {
        Some(Self {
            a: self.a.add(o.a)?,
            b: self.b.add(o.b)?,
        })
    }

    fn mul(self, o: Self, k: Rat) -> Option<Self> {
        Some(Self {
            a: self.a.mul(o.a)?.add(self.b.mul(o.b)?.mul(k)?)?,
            b: self.a.mul(o.b)?.add(self.b.mul(o.a)?)?,
        })
    }
}

/// A bivariate polynomial as a sparse exact map `u^iÂ·v^j â†’ coefficient`,
/// total degree â‰¤ 4.
type Poly = BTreeMap<(u32, u32), Fx>;

fn poly_const(c: Rat) -> Poly {
    let mut p = Poly::new();
    p.insert((0, 0), Fx::rational(c));
    p
}

/// `uÂ² + vÂ² + c` â€” the quadratic part every in-plane circle shares.
fn poly_uv_sq(c: Rat) -> Poly {
    let mut p = poly_const(c);
    p.insert((2, 0), Fx::rational(Rat::ONE));
    p.insert((0, 2), Fx::rational(Rat::ONE));
    p
}

/// `Î±Â·u + Î²Â·v + Î³`.
fn poly_linear(alpha: Rat, beta: Rat, gamma: Rat) -> Poly {
    let mut p = poly_const(gamma);
    p.insert((1, 0), Fx::rational(alpha));
    p.insert((0, 1), Fx::rational(beta));
    p
}

fn poly_sub(a: &Poly, b: &Poly) -> Option<Poly> {
    let mut out = a.clone();
    for (key, term) in b {
        let neg = term.neg()?;
        let sum = match out.get(key) {
            Some(existing) => existing.add(neg)?,
            None => neg,
        };
        if sum.is_zero() {
            out.remove(key);
        } else {
            out.insert(*key, sum);
        }
    }
    Some(out)
}

fn poly_scale(a: &Poly, s: Rat) -> Option<Poly> {
    let mut out = Poly::new();
    for (key, term) in a {
        let scaled = Fx {
            a: term.a.mul(s)?,
            b: term.b.mul(s)?,
        };
        if !scaled.is_zero() {
            out.insert(*key, scaled);
        }
    }
    Some(out)
}

fn poly_mul(a: &Poly, b: &Poly, k: Rat) -> Option<Poly> {
    let mut out = Poly::new();
    for ((i1, j1), c1) in a {
        for ((i2, j2), c2) in b {
            let key = (i1 + i2, j1 + j2);
            let prod = c1.mul(*c2, k)?;
            let sum = match out.get(&key) {
                Some(existing) => existing.add(prod)?,
                None => prod,
            };
            if sum.is_zero() {
                out.remove(&key);
            } else {
                out.insert(key, sum);
            }
        }
    }
    Some(out)
}

fn poly_sq(a: &Poly, k: Rat) -> Option<Poly> {
    poly_mul(a, a, k)
}

/// One circle factor `(u âˆ’ uâ‚€)Â² + vÂ² âˆ’ ÏÂ²` as a polynomial in the in-plane
/// frame: quadratic part `uÂ² + vÂ²`, linear part `âˆ’2uâ‚€Â·u`, and the constant
/// `uâ‚€Â² âˆ’ ÏÂ²`. `ÏÂ²` lives in `â„š(âˆšk)`, so the whole certificate runs in that
/// ring; a rational radius simply carries `b = 0`.
fn circle_factor(u0: Rat, rho_sq: Fx, k: Rat) -> Option<Poly> {
    let mut p = poly_uv_sq(Rat::ZERO);
    let constant = Fx {
        a: u0.sq()?.sub(rho_sq.a)?,
        b: rho_sq.b.neg()?,
    };
    if !constant.is_zero() {
        p.insert((0, 0), constant);
    }
    p.insert((1, 0), Fx::rational(u0.mul(Rat::from_i64(-2))?));
    let _ = k;
    Some(p)
}

/// The section scalars of the orthonormal in-plane frame:
/// `q = dÂ·nÌ‚ + uÂ·e1 + vÂ·e2` with `(qÂ·a) = dÂ·s + az1Â·u + az2Â·v`. `a = áº‘` is the
/// unit torus axis; Lemma 1 makes `qÂ·q = dÂ² + uÂ² + vÂ²` exact.
#[derive(Clone, Copy, Debug)]
struct SectionAlgebra {
    d: Rat,
    s: Rat,
    az1: Rat,
    az2: Rat,
}

/// Verifies the runtime factorization certificate: forms the section quartic
/// `Q(u, v)` from the frame scalars and the torus `(major, minor)`, forms the
/// product of the two recovered circle factors `Câ‚Â·Câ‚‚` in `â„š(âˆšk)`, and
/// requires `Q = Câ‚Â·Câ‚‚` coefficient-exact. `Ok(true)` is the certificate; a
/// failure (or an overflow in the exact arithmetic) refuses emission.
fn certificate_holds(
    major_sq: Rat,
    minor_sq: Rat,
    algebra: &SectionAlgebra,
    factor0: (Rat, Fx),
    factor1: (Rat, Fx),
    k: Rat,
) -> Option<bool> {
    let d_sq = algebra.d.sq()?;
    let c0 = d_sq.add(major_sq)?.sub(minor_sq)?;
    // (qÂ·q + RÂ² âˆ’ rÂ²) = uÂ² + vÂ² + c0.
    let qa = poly_uv_sq(c0);
    // qÂ·q âˆ’ (qÂ·a)Â² = (uÂ² + vÂ² + dÂ²) âˆ’ (dÂ·s + az1Â·u + az2Â·v)Â².
    let b = poly_uv_sq(d_sq);
    let ds = algebra.d.mul(algebra.s)?;
    let g = poly_linear(algebra.az1, algebra.az2, ds);
    let g_sq = poly_sq(&g, k)?;
    let inner = poly_sub(&b, &g_sq)?;
    let four_major_sq = major_sq.mul(Rat::from_i64(4))?;
    let q = poly_sub(&poly_sq(&qa, k)?, &poly_scale(&inner, four_major_sq)?)?;

    let c0 = circle_factor(factor0.0, factor0.1, k)?;
    let c1 = circle_factor(factor1.0, factor1.1, k)?;
    let product = poly_mul(&c0, &c1, k)?;
    Some(product == q)
}

// ---------------------------------------------------------------------------
// Emission
// ---------------------------------------------------------------------------

/// The affine placement of a unit circle: columns `u`, `v`, `n` and origin
/// `o`, scaled in-plane by `radius` in both directions.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, radius: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(radius, radius, 1.0)
}

/// A circle of the emitted locus: world centre, radius, and the orthonormal
/// in-plane axes of its containing plane (`normal = nÌ‚`).
#[derive(Clone, Copy, Debug)]
struct PlacedExactCircle {
    center: Point3,
    radius: f64,
    u: Vector3,
    v: Vector3,
    n: Vector3,
}

impl PlacedExactCircle {
    fn to_curve(self) -> PlacedCircle {
        Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            frame(self.u, self.v, self.n, self.center, self.radius),
        )
    }
}

/// The typed section outcome before the evidence certificate is attached.
enum Section {
    /// The plane misses the torus exactly.
    Empty,
    /// One double circle: the plane is tangent along a whole circle.
    Tangent(PlacedExactCircle),
    /// Two exact circles.
    Two(PlacedExactCircle, PlacedExactCircle),
}

/// The world-space unit axes of an in-plane orthonormal frame.
struct WorldFrame {
    e1: Vector3,
    e2: Vector3,
    n: Vector3,
}

// ---------------------------------------------------------------------------
// Frames and the exact rational unit normal
// ---------------------------------------------------------------------------

/// The exact rational unit normal of the plane, when the raw normal `m` has a
/// rational length. `m` is `u_axis Ã— v_axis` in exact arithmetic, so this is
/// exact whenever the carrier points are exact rationals; a plane whose
/// normal length is not a perfect rational square cannot build the
/// orthonormal frame the exact certificate needs and routes to the tracer.
fn unit_normal(m: RatVec) -> Option<RatVec> {
    let m2 = m.dot(m)?;
    let norm = m2.sqrt_exact()?;
    if norm.is_zero() {
        return None;
    }
    m.scale(Rat::ONE.div(norm)?)
}

/// Builds the exact orthonormal in-plane frame for a plane whose unit normal
/// is `nÌ‚` and whose signed centre-to-plane offset is `d`:
/// `e1 = (a Ã— nÌ‚)/âˆšH` (`H = 1 âˆ’ sÂ²`, `s = nÌ‚Â·a`) is the in-plane direction âŠ¥
/// the torus axis along which the profile-circle centres lie, and
/// `e2 = nÌ‚ Ã— e1`. Both are exact rational unit vectors when `âˆšH` is
/// rational.
fn frame_for_normal(n: RatVec, d: Rat) -> Option<Frame> {
    let s = n.z;
    let h = Rat::ONE.sub(s.sq()?)?;
    let root_h = h.sqrt_exact()?;
    if root_h.is_zero() {
        // nÌ‚ âˆ¥ a (axis-perpendicular) â€” the caller never routes here.
        return None;
    }
    // e1 = (a Ã— nÌ‚)/âˆšH = (âˆ’nÌ‚.y, nÌ‚.x, 0)/âˆšH, in the plane and âŠ¥ the axis.
    let e1 = RatVec {
        x: n.y.neg()?.div(root_h)?,
        y: n.x.div(root_h)?,
        z: Rat::ZERO,
    };
    let e2 = n.cross(e1)?;
    // e1 âŠ¥ a, so az1 = e1Â·a = 0 by construction; az2 = e2Â·a = e2.z.
    Some(Frame {
        e1,
        e2,
        n,
        algebra: SectionAlgebra {
            d,
            s,
            az1: Rat::ZERO,
            az2: e2.z,
        },
    })
}

/// The exact orthonormal frame of the cutting plane plus its algebra.
#[derive(Clone, Copy, Debug)]
struct Frame {
    e1: RatVec,
    e2: RatVec,
    n: RatVec,
    algebra: SectionAlgebra,
}

// ---------------------------------------------------------------------------
// The exact classification + factorization
// ---------------------------------------------------------------------------

/// The plane Ã— torus section classified and certified per instance.
fn section_circles(plane: &Plane, torus: &Torus) -> Result<Section, Refusal> {
    let major = Rat::from_f64(torus.large_radius()).ok_or_else(non_canonical)?;
    let minor = Rat::from_f64(torus.small_radius()).ok_or_else(non_canonical)?;
    let center = torus.center();
    let center_v = RatVec::from_point(center).ok_or_else(non_canonical)?;
    let origin = plane.origin();
    let u_axis = RatVec::from_vec3(plane.u_axis()).ok_or_else(non_canonical)?;
    let v_axis = RatVec::from_vec3(plane.v_axis()).ok_or_else(non_canonical)?;

    // Ring torus only: major > minor > 0 (the constructor guarantees > 0;
    // horn/spindle refuse typed â€” the landed formal/torus.rs rule).
    if !(major.cmp(minor).ok_or_else(trace)? == Ordering::Greater) {
        return Err(non_canonical());
    }

    // The plane normal in exact arithmetic (the scale-invariant T-form: the
    // raw cross product, never the rounded unit normal).
    let m = u_axis.cross(v_axis).ok_or_else(non_canonical)?;
    if m.is_zero() {
        return Err(non_canonical());
    }
    // H = K(nÂ·n) âˆ’ sÂ² with K = 1: zero exactly when nÌ‚ âˆ¥ a (the plane is
    // perpendicular to the torus axis).
    let hx = m.x.sq().ok_or_else(trace)?;
    let hy = m.y.sq().ok_or_else(trace)?;
    let h_raw = hx.add(hy).ok_or_else(trace)?;
    let s_raw = m.z;
    // dÂ·|m| = (o âˆ’ c)Â·m, the signed centre-to-plane offset in raw units.
    let oc = RatVec::from_points(origin, center).ok_or_else(trace)?;
    let d_raw = oc.dot(m).ok_or_else(trace)?;

    if h_raw.is_zero() {
        return axis_perpendicular(&center_v, &oc, major, minor);
    }

    let major_sq = major.sq().ok_or_else(trace)?;
    let minor_sq = minor.sq().ok_or_else(trace)?;
    let central = d_raw.is_zero();
    if central && s_raw.is_zero() {
        // Axial: the plane contains the axis and the centre â€” two profile
        // circles of radius r at O Â± RÂ·(a Ã— nÌ‚).
        return axial(&center_v, m, major, minor);
    }
    if central && !s_raw.is_zero() && {
        // Villarceau predicate, all-exact in the raw normal (homogeneous
        // degree 2, so scale-invariant): (RÂ² âˆ’ rÂ²)Â·H = rÂ²Â·sÂ².
        let lhs = major_sq
            .sub(minor_sq)
            .ok_or_else(trace)?
            .mul(h_raw)
            .ok_or_else(trace)?;
        let rhs = minor_sq
            .mul(s_raw.sq().ok_or_else(trace)?)
            .ok_or_else(trace)?;
        lhs == rhs
    } {
        return villarceau_circles(&center_v, m, major, minor);
    }

    // --- residual quartic -------------------------------------------------
    // A plane strictly farther from the centre than R + r meets the torus
    // nowhere: |d| > R + r is an exact, scale-invariant exclusion.
    let reach = major.add(minor).ok_or_else(trace)?;
    let lhs = d_raw.sq().ok_or_else(trace)?;
    let rhs = reach
        .sq()
        .ok_or_else(trace)?
        .mul(m.dot(m).ok_or_else(trace)?)
        .ok_or_else(trace)?;
    if lhs.cmp(rhs).ok_or_else(trace)? == Ordering::Greater {
        return Ok(Section::Empty);
    }
    // Any other residual section is a genuine quartic (the spiric family);
    // traced, never approximated.
    Err(trace())
}

/// The axial family: the plane contains the axis and the torus centre â€” two
/// profile circles of radius `minor` at `O Â± majorÂ·e1`.
fn axial(center: &RatVec, m: RatVec, major: Rat, minor: Rat) -> Result<Section, Refusal> {
    let major_sq = major.sq().ok_or_else(trace)?;
    let minor_sq = minor.sq().ok_or_else(trace)?;
    let frame = frame_for_normal(unit_normal(m).ok_or_else(trace)?, Rat::ZERO).ok_or_else(trace)?;
    let k = Rat::ONE;
    let rho_sq = Fx::rational(minor_sq);
    let ok = certificate_holds(
        major_sq,
        minor_sq,
        &frame.algebra,
        (major.neg().ok_or_else(trace)?, rho_sq),
        (major, rho_sq),
        k,
    )
    .ok_or_else(trace)?;
    if !ok {
        return Err(trace());
    }
    let world = world_frame(&frame);
    let (c0, c1) = two_centred_circles(center_point(*center), world, major, minor);
    Ok(Section::Two(c0, c1))
}

/// The Villarceau family: a central oblique bitangent plane â€” two circles of
/// radius `major` at `O Â± minorÂ·e1`, meeting at the two bitangency points.
fn villarceau_circles(
    center: &RatVec,
    m: RatVec,
    major: Rat,
    minor: Rat,
) -> Result<Section, Refusal> {
    let major_sq = major.sq().ok_or_else(trace)?;
    let minor_sq = minor.sq().ok_or_else(trace)?;
    let frame = frame_for_normal(unit_normal(m).ok_or_else(trace)?, Rat::ZERO).ok_or_else(trace)?;
    let k = Rat::ONE;
    let rho_sq = Fx::rational(major_sq);
    let ok = certificate_holds(
        major_sq,
        minor_sq,
        &frame.algebra,
        (minor.neg().ok_or_else(trace)?, rho_sq),
        (minor, rho_sq),
        k,
    )
    .ok_or_else(trace)?;
    if !ok {
        return Err(trace());
    }
    let world = world_frame(&frame);
    let (c0, c1) = two_centred_circles(center_point(*center), world, minor, major);
    Ok(Section::Two(c0, c1))
}

/// The axis-perpendicular family: the plane is perpendicular to the torus
/// axis. Two coaxial circles `ÏÂ± = R Â± âˆš(rÂ² âˆ’ dÂ²)`, an exact Empty, or the
/// double tangent circle.
fn axis_perpendicular(
    center: &RatVec,
    oc: &RatVec,
    major: Rat,
    minor: Rat,
) -> Result<Section, Refusal> {
    let major_sq = major.sq().ok_or_else(trace)?;
    let minor_sq = minor.sq().ok_or_else(trace)?;
    // The horizontal plane sits at height o.z; the signed centre offset along
    // +áº‘ is oc.z (only dÂ² enters the classification below, so the sign is
    // immaterial). The canonical frame is nÌ‚ = áº‘, e1 = xÌ‚, e2 = Å·.
    let d = oc.z;
    let d_sq = d.sq().ok_or_else(trace)?;
    let order = d_sq.cmp(minor_sq).ok_or_else(trace)?;
    // Radicand of the coaxial radii: k = rÂ² âˆ’ dÂ².
    let k = minor_sq.sub(d_sq).ok_or_else(trace)?;
    let algebra = SectionAlgebra {
        d,
        s: Rat::ONE,
        az1: Rat::ZERO,
        az2: Rat::ZERO,
    };

    let centre = center_point(*center);
    let height = centre.z + d.to_f64();
    let normal = Vector3::unit_z();
    let e1 = Vector3::unit_x();
    let e2 = Vector3::unit_y();

    if order == Ordering::Greater {
        // |d| > r: the plane clears the tube; no real section.
        return Ok(Section::Empty);
    }
    if order == Ordering::Equal {
        // |d| = r: the two coaxial circles coalesce into the double circle of
        // radius R, tangent along the whole ring. Q = (uÂ² + vÂ² âˆ’ RÂ²)Â².
        let rho_sq = Fx::rational(major_sq);
        let ok = certificate_holds(
            major_sq,
            minor_sq,
            &algebra,
            (Rat::ZERO, rho_sq),
            (Rat::ZERO, rho_sq),
            k,
        )
        .ok_or_else(trace)?;
        if !ok {
            return Err(trace());
        }
        let double_center = Point3::new(centre.x, centre.y, height);
        return Ok(Section::Tangent(PlacedExactCircle {
            center: double_center,
            radius: major.to_f64(),
            u: e1,
            v: e2,
            n: normal,
        }));
    }

    // |d| < r: two coaxial circles, ÏÂ± = R Â± âˆš(rÂ² âˆ’ dÂ²) âˆˆ â„š(âˆšk), both
    // centred on the axis at the plane height. Their factors are
    // (uÂ² + vÂ² âˆ’ ÏÂ±Â²) with ÏÂ±Â² = (RÂ² + k) Â± 2Râˆšk.
    let radius_plus = Fx {
        a: major_sq.add(k).ok_or_else(trace)?,
        b: major.mul(Rat::from_i64(2)).ok_or_else(trace)?,
    };
    let radius_minus = Fx {
        a: major_sq.add(k).ok_or_else(trace)?,
        b: major.mul(Rat::from_i64(-2)).ok_or_else(trace)?,
    };
    let ok = certificate_holds(
        major_sq,
        minor_sq,
        &algebra,
        (Rat::ZERO, radius_minus),
        (Rat::ZERO, radius_plus),
        k,
    )
    .ok_or_else(trace)?;
    if !ok {
        return Err(trace());
    }
    let axis_center = Point3::new(centre.x, centre.y, height);
    // The f64 radii from the closed form (values, not predicates; the exact
    // radii live in â„š(âˆšk), which the certificate already checked).
    let root = k.to_f64().sqrt();
    let c_inner = PlacedExactCircle {
        center: axis_center,
        radius: major.to_f64() - root,
        u: e1,
        v: e2,
        n: normal,
    };
    let c_outer = PlacedExactCircle {
        center: axis_center,
        radius: major.to_f64() + root,
        u: e1,
        v: e2,
        n: normal,
    };
    Ok(Section::Two(c_inner, c_outer))
}

/// Two circles of radius `radius`, centred at `center âˆ“ offsetÂ·e1`, in the
/// plane spanned by `e1`, `e2` with normal `nÌ‚`. Returns the `âˆ’` then `+`
/// offsets in deterministic order.
fn two_centred_circles(
    center: Point3,
    frame: WorldFrame,
    offset: Rat,
    radius: Rat,
) -> (PlacedExactCircle, PlacedExactCircle) {
    let off = offset.to_f64();
    let rad = radius.to_f64();
    let minus = PlacedExactCircle {
        center: center + (-off) * frame.e1,
        radius: rad,
        u: frame.e1,
        v: frame.e2,
        n: frame.n,
    };
    let plus = PlacedExactCircle {
        center: center + off * frame.e1,
        radius: rad,
        u: frame.e1,
        v: frame.e2,
        n: frame.n,
    };
    (minus, plus)
}

fn world_frame(frame: &Frame) -> WorldFrame {
    WorldFrame {
        e1: frame.e1.to_vec3(),
        e2: frame.e2.to_vec3(),
        n: frame.n.to_vec3(),
    }
}

fn center_point(c: RatVec) -> Point3 {
    Point3::new(c.x.to_f64(), c.y.to_f64(), c.z.to_f64())
}

fn non_canonical() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::NonCanonicalCarrier)
}

fn trace() -> Refusal {
    Refusal::UnsupportedEnvelope(EnvelopeCase::ContactReductionDeferred)
}

fn exact_certificate(arm: AnalyticIntersection) -> AnalyticOutcome {
    let mut props = PropMap::new();
    props.set(Prop::AnalyticCarrier, Truth::True);
    Ok(Certified::new(
        arm,
        Certificate {
            props,
            method: Method::Exact,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// Classifies the plane Ã— torus pair and emits the exact circle loci.
///
/// `Method::Exact` here means: the arm is decided by exact rational predicates
/// on the `f64` carrier parameters (each finite `f64` is a dyadic rational, so
/// the decisions are exact), and every emitted circle is backed by the
/// per-instance factorization certificate `Q = Câ‚Â·Câ‚‚` checked coefficient-
/// exact in `â„š(âˆšk)`. The emitted coordinates are the closed form evaluated in
/// `f64`; the obligation is "lies on both carriers to machine precision",
/// asserted with an H-3-commented slack in the conformance tests. A section
/// that is not an exactly-certified circle pair â€” a residual quartic (spiric
/// section), or a circle cut whose data the exact arithmetic cannot certify â€”
/// routes to the tracer as
/// `Refusal::UnsupportedEnvelope(ContactReductionDeferred)`, never
/// approximated. Horn/spindle tori refuse `NonCanonicalCarrier` typed.
pub fn plane_torus_section(plane: &Plane, torus: &Torus) -> AnalyticOutcome {
    match section_circles(plane, torus) {
        Ok(Section::Empty) => exact_certificate(AnalyticIntersection::Empty),
        Ok(Section::Tangent(circle)) => {
            exact_certificate(AnalyticIntersection::TangentCircle(circle.to_curve()))
        }
        Ok(Section::Two(c0, c1)) => exact_certificate(AnalyticIntersection::TwoCurves([
            ExactCurve::Circle(c0.to_curve()),
            ExactCurve::Circle(c1.to_curve()),
        ])),
        Err(refusal) => Err(refusal),
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built dyadic witnesses are not such
// a path; these unwraps cannot fire for the values constructed.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;

    fn z_plane(z: f64) -> Plane {
        Plane::new(
            Point3::new(-2.0, -2.0, z),
            Point3::new(2.0, -2.0, z),
            Point3::new(-2.0, 2.0, z),
        )
    }

    #[test]
    fn rat_from_f64_is_exact_and_nonfinite_refuses() {
        assert_eq!(Rat::from_f64(0.25), Rat::reduce(1, 4));
        assert_eq!(Rat::from_f64(0.5), Rat::reduce(1, 2));
        assert_eq!(Rat::from_f64(-1.5), Rat::reduce(-3, 2));
        assert_eq!(Rat::from_f64(0.0), Some(Rat::ZERO));
        assert!(Rat::from_f64(f64::NAN).is_none());
        assert!(Rat::from_f64(f64::INFINITY).is_none());
        assert!(Rat::from_f64(f64::NEG_INFINITY).is_none());
    }

    #[test]
    fn rat_sqrt_exact_detects_perfect_squares() {
        // (3/5)^2 and (1/4)^2 are exact rational squares; 1/2 and 3/16 are not.
        let nines = Rat::reduce(9, 25).unwrap();
        assert_eq!(nines.sqrt_exact(), Rat::reduce(3, 5));
        let quarter_sq = Rat::reduce(1, 16).unwrap();
        assert_eq!(quarter_sq.sqrt_exact(), Rat::reduce(1, 4));
        assert!(Rat::reduce(1, 2).unwrap().sqrt_exact().is_none());
        assert!(Rat::reduce(3, 16).unwrap().sqrt_exact().is_none());
    }

    #[test]
    fn rat_compare_is_exact_across_denominator_sizes() {
        // Cross-multiplying these would overflow; the Euclidean recursion must
        // still order them exactly.
        let a = Rat::from_f64(0.7).unwrap().sq().unwrap();
        let b = Rat::from_f64(0.5).unwrap().sq().unwrap();
        assert_eq!(a.cmp(b), Some(Ordering::Greater));
        assert_eq!(b.cmp(a), Some(Ordering::Less));
        let doubled = Rat::from_f64(0.7).unwrap().mul(Rat::from_i64(2)).unwrap();
        let other = Rat::from_f64(1.4).unwrap();
        assert_eq!(doubled.cmp(other), Some(Ordering::Equal));
    }

    #[test]
    fn the_axial_certificate_identity_is_machine_checked() {
        // The axial section on exact data: major = 2, minor = 1/2, profile
        // circles (u âˆ’/+ 2)^2 + v^2 = 1/4. The identity must hold exactly and
        // fail when the plane is shifted off the centre.
        let algebra = SectionAlgebra {
            d: Rat::ZERO,
            s: Rat::ZERO,
            az1: Rat::ZERO,
            az2: Rat::ONE,
        };
        let major_sq = Rat::from_i64(4);
        let minor_sq = Rat::from_f64(0.25).unwrap();
        let rho_sq = Fx::rational(minor_sq);
        let two = Rat::from_i64(2);
        let ok = certificate_holds(
            major_sq,
            minor_sq,
            &algebra,
            (two.neg().unwrap(), rho_sq),
            (two, rho_sq),
            Rat::ONE,
        )
        .unwrap();
        assert!(ok, "the axial Q = C1*C2 identity must hold exactly");

        let shifted = SectionAlgebra {
            d: Rat::from_i64(1),
            ..algebra
        };
        let bad = certificate_holds(
            major_sq,
            minor_sq,
            &shifted,
            (two.neg().unwrap(), rho_sq),
            (two, rho_sq),
            Rat::ONE,
        )
        .unwrap();
        assert!(!bad, "a non-central plane is not the profile-circle pair");
    }

    #[test]
    fn horn_and_spindle_tori_refuse_typed() {
        let horn = Torus::new(Point3::origin(), 2.0, 2.0);
        let out = plane_torus_section(&z_plane(0.25), &horn);
        assert!(matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
        let spindle = Torus::new(Point3::origin(), 2.0, 5.0);
        let out = plane_torus_section(&z_plane(0.25), &spindle);
        assert!(matches!(
            out,
            Err(Refusal::UnsupportedEnvelope(
                EnvelopeCase::NonCanonicalCarrier
            ))
        ));
    }
}
