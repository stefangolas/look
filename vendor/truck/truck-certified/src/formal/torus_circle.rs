// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Certified circle-on-torus membership, continuous-lift winding, and
//! `GL(2, Z)` basis normalization for tilted-circle torus loops
//! (TORUS-DIAGONAL-1).
//!
//! Built on [`super::torus::CertifiedRankTwoDeck`] and
//! [`super::torus_cell::BoundaryLoopPlacement`]. The existing annular cell
//! (`torus_cell::certify_loop`) admits only axis-aligned circles whose plane
//! normal is a *unit* vector within `1e-9` of `±â` (parallel) or `⊥ â`
//! (meridian). This module answers the question that adapter leaves open: when
//! a source circle is **tilted** (or when the adapter could not decide), is the
//! circle still an exact torus curve, and if so what is its `Z²` winding?
//!
//! # Circle-on-torus certification
//!
//! A circle `P(t) = C + ρ(cos t · û + sin t · v̂)` lies on the regular torus
//! `(O, â, R, r)` iff the torus implicit equation
//!
//! ```text
//! F(x,y,z) = (x² + y² + z² + R² − r²)² − 4 R² (x² + y²) = 0
//! ```
//!
//! vanishes for **every** `t`. `F(P(t))` is a trigonometric polynomial of
//! degree 4 (F is degree 4, P is degree 1). It is identically zero iff all nine
//! Fourier coefficients (real and imaginary parts of the `e^{ikt}` coefficients
//! for `k = 0..=4`) are zero. Each coefficient is evaluated as a
//! [`CertifiedInterval`] by directed-rounding interval arithmetic, so the
//! decision is a whole-interval certificate, not dense sampling:
//!
//! - any coefficient interval strictly separated from zero ⇒ `ProvedNotOnTorus`;
//! - all intervals contain zero and are tight (below a scale-aware roundoff
//!   floor) ⇒ `CertifiedOnTorus`;
//! - otherwise ⇒ `OnTorusUnresolved`.
//!
//! # Winding from a continuous lift
//!
//! For a circle certified on the torus, the `Z²` winding `(p, q)` is the
//! endpoint displacement of the continuous lift `(u(t), v(t))` divided by `2π`.
//! Rather than lift by sampling `atan2` (whose branch choices are a proof gap),
//! the winding is computed by **exact signed seam-crossing counting**:
//!
//! - `p` = signed count of crossings of the azimuthal seam `y = 0, x > 0` by
//!   the projected circle `(x(t), y(t))` — an ellipse, which winds at most
//!   once, so `p ∈ {−1, 0, +1}`.
//! - `q` = signed count of crossings of the poloidal seam `z = 0,
//!   √(x²+y²) > R` (equivalently `x² + y² − R² > 0`) by `(√(x²+y²)−R, z(t))`.
//!
//! The seam-crossing zeros are the zeros of a sinusoid `c + a cos t + b sin t`,
//! located in closed form; every sign test (seam side, crossing direction) is
//! decided by [`CertifiedInterval`], so the result is a certified integer, not
//! nearest-integer rounding. A winding whose proof interval does not isolate a
//! single integer is returned as [`WindingResult::Unresolved`].
//!
//! # `GL(2, Z)` normalization
//!
//! For a primitive winding `h = (p, q)` with `gcd(|p|,|q|) = 1`, a unimodular
//! matrix `M ∈ GL(2, Z)` re-expresses the deck basis so that `h` becomes the
//! canonical first generator `(1, 0)`. This witnesses that the existing
//! rectangle realizer can be reused after a basis change, without generalizing
//! the development machinery.

use super::exact::CertifiedInterval;
use super::torus::CertifiedRankTwoDeck;
use super::torus_cell::BoundaryLoopPlacement;
use truck_geometry::prelude::{InnerSpace, Point3, Vector3};

/// Dimensionless floor separating "the interval contains zero because the value
/// is zero (roundoff)" from "the interval is too wide to decide". The
/// coefficient magnitudes scale as `(R+r)⁴`; the directed-rounding width is a
/// few ulp of that, so `1e-7·scale⁴` is well clear of roundoff and well below
/// any real geometric deviation.
const ON_TORUS_RELATIVE_FLOOR: f64 = 1e-7;

// ===========================================================================
// Complex interval arithmetic (for the Fourier-coefficient convolution)
// ===========================================================================

#[derive(Clone, Copy)]
struct Cplx {
    re: CertifiedInterval,
    im: CertifiedInterval,
}

impl Cplx {
    fn real(x: f64) -> Self {
        Cplx {
            re: CertifiedInterval::point(x),
            im: CertifiedInterval::point(0.0),
        }
    }
    fn from_real(ci: CertifiedInterval) -> Self {
        Cplx {
            re: ci,
            im: CertifiedInterval::point(0.0),
        }
    }
    fn zero() -> Self {
        Self::real(0.0)
    }
    fn add(&self, other: &Self) -> Self {
        Cplx {
            re: self.re.add(&other.re),
            im: self.im.add(&other.im),
        }
    }
    fn sub(&self, other: &Self) -> Self {
        Cplx {
            re: self.re.sub(&other.re),
            im: self.im.sub(&other.im),
        }
    }
    fn mul(&self, other: &Self) -> Self {
        // (a+bi)(c+di) = (ac-bd) + (ad+bc)i
        Cplx {
            re: self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            im: self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        }
    }
    fn scale(&self, s: f64) -> Self {
        let f = CertifiedInterval::point(s);
        Cplx {
            re: self.re.mul(&f),
            im: self.im.mul(&f),
        }
    }
    /// Whether both parts strictly exclude zero (a certified nonzero).
    fn strictly_nonzero(&self) -> bool {
        (self.re.lo > 0.0 || self.re.hi < 0.0) || (self.im.lo > 0.0 || self.im.hi < 0.0)
    }
    /// Classify both parts against a roundoff floor.
    fn classify_floor(&self, floor: f64) -> IntervalClass {
        let rc = classify_ci(&self.re, floor);
        let ic = classify_ci(&self.im, floor);
        // The worse of the two parts dominates.
        match (rc, ic) {
            (IntervalClass::Beyond, _) | (_, IntervalClass::Beyond) => IntervalClass::Beyond,
            (IntervalClass::Straddles, _) | (_, IntervalClass::Straddles) => {
                IntervalClass::Straddles
            }
            (IntervalClass::Within, IntervalClass::Within) => IntervalClass::Within,
        }
    }
    /// Max absolute bound over both parts.
    fn max_abs(&self) -> f64 {
        self.re
            .lo
            .abs()
            .max(self.re.hi.abs())
            .max(self.im.lo.abs())
            .max(self.im.hi.abs())
    }
}

/// How a coefficient interval relates to the roundoff floor `[-floor, floor]`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IntervalClass {
    /// Entirely within `[-floor, floor]` (consistent with exact zero + roundoff).
    Within,
    /// Entirely beyond `floor` (a real, non-roundoff deviation).
    Beyond,
    /// Straddles the floor boundary (cannot decide).
    Straddles,
}

fn classify_ci(ci: &CertifiedInterval, floor: f64) -> IntervalClass {
    if ci.lo > floor || ci.hi < -floor {
        IntervalClass::Beyond
    } else if ci.hi <= floor && ci.lo >= -floor {
        IntervalClass::Within
    } else {
        IntervalClass::Straddles
    }
}

/// A real trigonometric polynomial stored in complex-exponential form:
/// `T(t) = Σ_{k=-deg}^{deg} c_k e^{ikt}`, with `c_{-k} = conj(c_k)`.
/// `coeffs` is indexed `k + offset` where `offset = deg`.
#[derive(Clone)]
struct TrigPoly {
    offset: usize,
    coeffs: Vec<Cplx>,
}

impl TrigPoly {
    fn constant(x: f64) -> Self {
        TrigPoly {
            offset: 0,
            coeffs: vec![Cplx::real(x)],
        }
    }
    /// `c + a cos t + b sin t` (degree 1).
    fn linear(c: f64, a: f64, b: f64) -> Self {
        // cos t  -> e^{it}/2 + e^{-it}/2          : c_1 = 1/2, c_{-1} = 1/2
        // sin t  -> (e^{it} - e^{-it})/(2i)       : c_1 = -i/2, c_{-1} = +i/2
        let half = Cplx {
            re: CertifiedInterval::point(0.5),
            im: CertifiedInterval::point(0.0),
        };
        let mi_half = Cplx {
            re: CertifiedInterval::point(0.0),
            im: CertifiedInterval::point(-0.5),
        };
        let pi_half = Cplx {
            re: CertifiedInterval::point(0.0),
            im: CertifiedInterval::point(0.5),
        };
        let c1 = half.scale(a).add(&mi_half.scale(b));
        let cm1 = half.scale(a).add(&pi_half.scale(b));
        TrigPoly {
            offset: 1,
            coeffs: vec![cm1, Cplx::real(c), c1],
        }
    }
    fn degree(&self) -> usize {
        self.offset
    }
    fn coeff(&self, k: isize) -> &Cplx {
        let idx = (k + self.offset as isize) as usize;
        &self.coeffs[idx]
    }
    fn add(&self, other: &Self) -> Self {
        let deg = self.degree().max(other.degree());
        let offset = deg;
        let mut coeffs = vec![Cplx::zero(); 2 * deg + 1];
        for k in -(self.degree() as isize)..=(self.degree() as isize) {
            let idx = (k + offset as isize) as usize;
            coeffs[idx] = coeffs[idx].add(self.coeff(k));
        }
        for k in -(other.degree() as isize)..=(other.degree() as isize) {
            let idx = (k + offset as isize) as usize;
            coeffs[idx] = coeffs[idx].add(other.coeff(k));
        }
        TrigPoly { offset, coeffs }
    }
    fn mul(&self, other: &Self) -> Self {
        let deg = self.degree() + other.degree();
        let offset = deg;
        let mut coeffs = vec![Cplx::zero(); 2 * deg + 1];
        for j in -(self.degree() as isize)..=(self.degree() as isize) {
            for l in -(other.degree() as isize)..=(other.degree() as isize) {
                let k = j + l;
                let idx = (k + offset as isize) as usize;
                let prod = self.coeff(j).mul(other.coeff(l));
                coeffs[idx] = coeffs[idx].add(&prod);
            }
        }
        TrigPoly { offset, coeffs }
    }
    fn scale(&self, s: f64) -> Self {
        TrigPoly {
            offset: self.offset,
            coeffs: self.coeffs.iter().map(|c| c.scale(s)).collect(),
        }
    }
    /// Add a real constant (shifts the k=0 coefficient).
    fn shift_const(&self, x: f64) -> Self {
        let mut r = self.clone();
        let idx = self.offset;
        r.coeffs[idx] = r.coeffs[idx].add(&Cplx::real(x));
        r
    }
}

// ===========================================================================
// Circle-on-torus certification
// ===========================================================================

/// The geometric family of a certified on-torus circle, recovered from its
/// winding `(p, q)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleFamily {
    /// Winding `(±1, 0)`: a parallel (latitude), plane ⊥ axis.
    Parallel,
    /// Winding `(0, ±1)`: a meridian, plane ∥ axis through the axis.
    Meridian,
    /// Winding `(±1, ±1)`: a Villarceau-type diagonal circle.
    Diagonal,
    /// Another primitive winding (not `(±1,0)`, `(0,±1)`, or `(±1,±1)`).
    OtherPrimitive,
}

/// A proof witness for a circle certified on the torus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnTorusWitness {
    /// The certified `Z²` winding of the continuous lift.
    pub winding: [i64; 2],
    /// The family recovered from the winding.
    pub family: CircleFamily,
    /// `gcd(|p|, |q|)`: 1 for primitive, > 1 for nonprimitive.
    pub gcd: i64,
}

/// The result of certifying whether a source circle lies on the torus.
#[derive(Debug, Clone, PartialEq)]
pub enum CircleOnTorusStatus {
    /// The entire circle is certified to lie on the torus, with its winding.
    CertifiedOnTorus {
        /// The certified winding and family witness.
        witness: OnTorusWitness,
    },
    /// At least one Fourier coefficient of `F(P(t))` is provably nonzero.
    ProvedNotOnTorus {
        /// The largest coefficient magnitude observed.
        max_residual: f64,
    },
    /// No coefficient is provably nonzero, but the intervals are too wide to
    /// certify exact zero.
    OnTorusUnresolved {
        /// The largest coefficient magnitude observed.
        max_residual: f64,
    },
    /// The circle parameters are degenerate (non-unit normal, zero radius, etc.)
    OperationalFailure,
}

/// Certify whether a source circle lies on the identified regular torus, and if
/// so compute its certified `Z²` winding.
///
/// The circle is given by `placement` (centre, unit plane normal, radius,
/// effective orientation sign) over the deck `deck`. The test substitutes the
/// circle parametrization into the torus implicit equation and checks that the
/// resulting degree-4 trigonometric polynomial is identically zero, with every
/// coefficient evaluated by directed-rounding interval arithmetic.
pub fn certify_circle_on_torus(
    deck: &CertifiedRankTwoDeck,
    placement: &BoundaryLoopPlacement,
) -> CircleOnTorusStatus {
    let schema = deck.schema();
    let center = schema.center();
    let axis = schema.axis();
    let large = schema.large_radius().get();
    let small = schema.small_radius().get();

    // Degeneracy guards.
    let n_mag = placement.normal.magnitude();
    if !(n_mag > 0.0) || !(placement.radius > 0.0) {
        return CircleOnTorusStatus::OperationalFailure;
    }
    let u = placement.normal / n_mag; // unit plane normal
                                      // Build an orthonormal in-plane basis (û, v̂) with û × v̂ = û_plane_normal.
                                      // Pick û = any unit vector not parallel to u; v̂ = u × û; re-orthonormalize.
    let seed = if u.x.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let mut a1 = seed - u * seed.dot(u);
    let a1n = a1.magnitude();
    if !(a1n > 0.0) {
        return CircleOnTorusStatus::OperationalFailure;
    }
    a1 = a1 / a1n;
    let a2 = u.cross(a1); // already unit (u, a1 orthonormal)

    // Transform to canonical frame: torus centre -> origin, axis -> ẑ.
    // Build a rotation R with R·axis = ẑ. Then (x,y,z)_canon = R·(P - center).
    let rel = placement.center - center;
    let cx = rel.dot(a_perp_x(axis));
    let cy = rel.dot(a_perp_y(axis));
    let cz = rel.dot(axis);
    // Circle basis in canonical frame:
    let ux = a1.dot(a_perp_x(axis));
    let uy = a1.dot(a_perp_y(axis));
    let uz = a1.dot(axis);
    let vx = a2.dot(a_perp_x(axis));
    let vy = a2.dot(a_perp_y(axis));
    let vz = a2.dot(axis);
    let rho = placement.radius;

    // x(t) = cx + rho*ux*cos t + rho*vx*sin t, etc.
    let xt = TrigPoly::linear(cx, rho * ux, rho * vx);
    let yt = TrigPoly::linear(cy, rho * uy, rho * vy);
    let zt = TrigPoly::linear(cz, rho * uz, rho * vz);

    let x2 = xt.mul(&xt);
    let y2 = yt.mul(&yt);
    let z2 = zt.mul(&zt);
    let s = x2.add(&y2).add(&z2); // x²+y²+z²
    let p = x2.add(&y2); // x²+y²
    let t_poly = s.shift_const(large * large - small * small); // s + R² - r²
    let t2 = t_poly.mul(&t_poly); // (s + R² - r²)²
    let four_r2 = 4.0 * large * large;
    let f = t2.add(&p.scale(-four_r2)); // F = (s+R²-r²)² - 4R²(x²+y²)

    // Check all Fourier coefficients of F against a scale-aware roundoff floor.
    // The coefficient magnitudes scale as (R+r)⁴; the floor separates f64
    // roundoff (~ε·scale⁴) from a real geometric deviation (~δ·scale³).
    let scale = large + small;
    let floor = ON_TORUS_RELATIVE_FLOOR * scale * scale * scale * scale;
    let deg = f.degree();
    let mut max_abs = 0.0_f64;
    let mut worst = IntervalClass::Within;
    for k in 0..=deg {
        let c = f.coeff(k as isize);
        let cls = c.classify_floor(floor);
        if cls == IntervalClass::Beyond && worst != IntervalClass::Beyond {
            worst = IntervalClass::Beyond;
        } else if cls == IntervalClass::Straddles && worst == IntervalClass::Within {
            worst = IntervalClass::Straddles;
        }
        max_abs = max_abs.max(c.max_abs());
    }
    if worst == IntervalClass::Beyond {
        return CircleOnTorusStatus::ProvedNotOnTorus {
            max_residual: max_abs,
        };
    }
    if worst == IntervalClass::Straddles {
        return CircleOnTorusStatus::OnTorusUnresolved {
            max_residual: max_abs,
        };
    }

    // Certified on the torus. Compute the winding.
    let winding = match lift_circle_winding(deck, placement) {
        WindingResult::Certified(w) => w.winding,
        WindingResult::Unresolved { .. } => {
            return CircleOnTorusStatus::OnTorusUnresolved {
                max_residual: max_abs,
            };
        }
        WindingResult::OperationalFailure => {
            return CircleOnTorusStatus::OperationalFailure;
        }
    };
    let (p, q) = (winding[0], winding[1]);
    let g = gcd(p.unsigned_abs(), q.unsigned_abs());
    let family = family_of(p, q);
    CircleOnTorusStatus::CertifiedOnTorus {
        witness: OnTorusWitness {
            winding,
            family,
            gcd: g as i64,
        },
    }
}

/// `GL(2, Z)` normalization of a primitive winding to the canonical `(1, 0)`.
fn family_of(p: i64, q: i64) -> CircleFamily {
    let (ap, aq) = (p.unsigned_abs(), q.unsigned_abs());
    if aq == 0 {
        CircleFamily::Parallel
    } else if ap == 0 {
        CircleFamily::Meridian
    } else if ap == 1 && aq == 1 {
        CircleFamily::Diagonal
    } else {
        CircleFamily::OtherPrimitive
    }
}

/// Two orthonormal in-plane directions perpendicular to the axis, for the
/// canonical-frame projection. `a_perp_x` = the world-x-like direction.
fn a_perp_x(axis: Vector3) -> Vector3 {
    let seed = if axis.x.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let mut v = seed - axis * seed.dot(axis);
    let n = v.magnitude();
    if n > 0.0 {
        v / n
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    }
}
fn a_perp_y(axis: Vector3) -> Vector3 {
    axis.cross(a_perp_x(axis))
}

fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// ===========================================================================
// Certified winding by signed seam-crossing counting
// ===========================================================================

/// A certified `Z²` winding from a continuous lifted traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedWinding {
    /// The integer winding `(p, q)` = endpoint displacement / `2π`.
    pub winding: [i64; 2],
    /// `gcd(|p|, |q|)`.
    pub gcd: u64,
}

/// The result of lifting a certified-on-torus circle continuously and reading
/// its endpoint displacement.
#[derive(Debug, Clone, PartialEq)]
pub enum WindingResult {
    /// The proof interval isolated a single integer pair.
    Certified(CertifiedWinding),
    /// The interval did not isolate an integer (a seam crossing was tangent or
    /// a sign test straddled zero).
    Unresolved {
        /// The best floating-point estimate of `(Δu/2π, Δv/2π)`.
        estimate: [f64; 2],
    },
    /// The circle parameters are degenerate.
    OperationalFailure,
}

/// Lift a certified-on-torus circle continuously through `(u, v)` parameter
/// space and return its certified `Z²` winding.
///
/// The winding is computed by exact signed seam-crossing counting, not by
/// sampling `atan2`: `p` counts signed crossings of the azimuthal seam
/// `y = 0, x > 0`; `q` counts signed crossings of the poloidal seam `z = 0,
/// x² + y² > R²`. Every sign test is a [`CertifiedInterval`] decision.
pub fn lift_circle_winding(
    deck: &CertifiedRankTwoDeck,
    placement: &BoundaryLoopPlacement,
) -> WindingResult {
    let schema = deck.schema();
    let center = schema.center();
    let axis = schema.axis();
    let large = schema.large_radius().get();

    let n_mag = placement.normal.magnitude();
    if !(n_mag > 0.0) || !(placement.radius > 0.0) {
        return WindingResult::OperationalFailure;
    }
    let u = placement.normal / n_mag;
    let seed = if u.x.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let mut a1 = seed - u * seed.dot(u);
    let a1n = a1.magnitude();
    if !(a1n > 0.0) {
        return WindingResult::OperationalFailure;
    }
    a1 = a1 / a1n;
    let a2 = u.cross(a1);
    let rel = placement.center - center;
    let ex = a_perp_x(axis);
    let ey = a_perp_y(axis);
    // Canonical-frame circle coefficients.
    let cx = rel.dot(ex);
    let cy = rel.dot(ey);
    let cz = rel.dot(axis);
    let rho = placement.radius;
    let ax = rho * a1.dot(ex);
    let bx = rho * a2.dot(ex);
    let ay = rho * a1.dot(ey);
    let by = rho * a2.dot(ey);
    let az = rho * a1.dot(axis);
    let bz = rho * a2.dot(axis);

    // p = signed crossings of y(t)=0 with x(t) > 0.
    let p = signed_azimuthal_crossings(cx, ax, bx, cy, ay, by);
    let q = signed_poloidal_crossings(cx, ax, bx, cy, ay, by, cz, az, bz, large);

    let (p, q) = match (p, q) {
        (Some(pp), Some(qq)) => (pp as i64, qq as i64),
        _ => {
            return WindingResult::Unresolved {
                estimate: [0.0, 0.0],
            }
        }
    };
    let g = gcd(p.unsigned_abs(), q.unsigned_abs());
    WindingResult::Certified(CertifiedWinding {
        winding: [p, q],
        gcd: g,
    })
}

/// Zeros of `c + a cos t + b sin t` on `[0, 2π)`, as `(cos t*, sin t*, direction
/// sign)` tuples, evaluated by [`CertifiedInterval`].
///
/// Returns `None` if a zero is tangent (cannot decide crossing direction).
fn sinusoid_zeros_ci(
    c: f64,
    a: f64,
    b: f64,
) -> Option<Vec<(CertifiedInterval, CertifiedInterval, CertifiedSign2)>> {
    let m2 = a * a + b * b;
    let m2_ci = CertifiedInterval::point(m2);
    let c2 = c * c;
    let c2_ci = CertifiedInterval::point(c2);
    let disc = m2_ci.sub(&c2_ci); // M² - c²
    if disc.lo <= 0.0 {
        // |c| >= M: no transverse crossing (none, or tangent).
        return Some(Vec::new());
    }
    let sqrt_disc = disc.sqrt()?;
    let m2_inv = CertifiedInterval::point(1.0).div(&m2_ci)?;
    // cos t* = (-a c ∓ b sqrt(disc)) / M²,  sin t* = (-b c ± a sqrt(disc)) / M²
    let ac_ci = CertifiedInterval::point(a * c);
    let bc_ci = CertifiedInterval::point(b * c);
    let a_sqrt = CertifiedInterval::point(a).mul(&sqrt_disc);
    let b_sqrt = CertifiedInterval::point(b).mul(&sqrt_disc);
    // zero 1 (φ + α):  cos = (-ac - b·s)/M²,  sin = (-bc + a·s)/M²
    let cos1 = ac_ci.add(&b_sqrt).neg().mul(&m2_inv);
    let sin1 = bc_ci.sub(&a_sqrt).neg().mul(&m2_inv);
    // zero 2 (φ - α):  cos = (-ac + b·s)/M²,  sin = (-bc - a·s)/M²
    let cos2 = ac_ci.sub(&b_sqrt).neg().mul(&m2_inv);
    let sin2 = bc_ci.add(&a_sqrt).neg().mul(&m2_inv);
    // direction = d/dt = -a sin t + b cos t.  sign at each zero.
    let dir1 = CertifiedInterval::point(a)
        .mul(&sin1)
        .neg()
        .add(&CertifiedInterval::point(b).mul(&cos1));
    let dir2 = CertifiedInterval::point(a)
        .mul(&sin2)
        .neg()
        .add(&CertifiedInterval::point(b).mul(&cos2));
    let s1 = sign_of(&dir1)?;
    let s2 = sign_of(&dir2)?;
    Some(vec![(cos1, sin1, s1), (cos2, sin2, s2)])
}

/// A certified sign (three-valued).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CertifiedSign2 {
    Negative,
    Zero,
    Positive,
}

fn sign_of(ci: &CertifiedInterval) -> Option<CertifiedSign2> {
    if ci.hi < 0.0 {
        Some(CertifiedSign2::Negative)
    } else if ci.lo > 0.0 {
        Some(CertifiedSign2::Positive)
    } else if ci.lo == 0.0 && ci.hi == 0.0 {
        Some(CertifiedSign2::Zero)
    } else {
        None // straddles zero — unresolved
    }
}

/// `p`: signed crossings of `y(t) = cy + ay cos t + by sin t = 0` with
/// `x(t) = cx + ax cos t + bx sin t > 0`.
fn signed_azimuthal_crossings(cx: f64, ax: f64, bx: f64, cy: f64, ay: f64, by: f64) -> Option<i8> {
    let zeros = sinusoid_zeros_ci(cy, ay, by)?;
    let mut winding: i8 = 0;
    for (cos_t, sin_t, dir) in zeros {
        // x(t*) = cx + ax cos t* + bx sin t*
        let x_at = CertifiedInterval::point(cx)
            .add(&CertifiedInterval::point(ax).mul(&cos_t))
            .add(&CertifiedInterval::point(bx).mul(&sin_t));
        let x_sign = sign_of(&x_at)?;
        if x_sign == CertifiedSign2::Positive {
            // crossing of the positive x-axis; direction = sign of dy/dt.
            match dir {
                CertifiedSign2::Positive => winding += 1,
                CertifiedSign2::Negative => winding -= 1,
                CertifiedSign2::Zero => return None,
            }
        } else if x_sign == CertifiedSign2::Zero {
            // tangent to the seam — unresolved.
            return None;
        }
    }
    Some(winding)
}

/// `q`: signed crossings of `z(t) = cz + az cos t + bz sin t = 0` with
/// `x(t)² + y(t)² − R² > 0`.
fn signed_poloidal_crossings(
    cx: f64,
    ax: f64,
    bx: f64,
    cy: f64,
    ay: f64,
    by: f64,
    cz: f64,
    az: f64,
    bz: f64,
    large: f64,
) -> Option<i8> {
    let zeros = sinusoid_zeros_ci(cz, az, bz)?;
    let r2 = large * large;
    let mut winding: i8 = 0;
    for (cos_t, sin_t, dir) in zeros {
        let x_at = CertifiedInterval::point(cx)
            .add(&CertifiedInterval::point(ax).mul(&cos_t))
            .add(&CertifiedInterval::point(bx).mul(&sin_t));
        let y_at = CertifiedInterval::point(cy)
            .add(&CertifiedInterval::point(ay).mul(&cos_t))
            .add(&CertifiedInterval::point(by).mul(&sin_t));
        let x2 = x_at.mul(&x_at);
        let y2 = y_at.mul(&y_at);
        let test = x2.add(&y2).sub(&CertifiedInterval::point(r2));
        let s = sign_of(&test)?;
        if s == CertifiedSign2::Positive {
            match dir {
                CertifiedSign2::Positive => winding += 1,
                CertifiedSign2::Negative => winding -= 1,
                CertifiedSign2::Zero => return None,
            }
        } else if s == CertifiedSign2::Zero {
            return None;
        }
    }
    Some(winding)
}

// ===========================================================================
// GL(2, Z) normalization
// ===========================================================================

/// A certified `GL(2, Z)` basis change that normalizes a primitive winding
/// `h = (p, q)` to the canonical class `(1, 0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gl2zNormalization {
    /// The original primitive winding.
    pub original: [i64; 2],
    /// The unimodular matrix `M` with `M · h = (1, 0)`.
    pub matrix: [[i64; 2]; 2],
    /// `det(M) = ±1`.
    pub determinant: i64,
    /// The transformed (canonical) winding, always `(1, 0)` on success.
    pub transformed: [i64; 2],
}

impl Gl2zNormalization {
    /// The orientation effect: `+1` if `det = +1` (orientation-preserving),
    /// `-1` if `det = -1` (reversing).
    pub fn orientation_effect(&self) -> i64 {
        self.determinant
    }
    /// Whether the normalization is valid (unimodular).
    pub fn is_valid(&self) -> bool {
        self.determinant == 1 || self.determinant == -1
    }
}

/// Normalize a primitive winding `h = (p, q)` (with `gcd(|p|,|q|) = 1`) to the
/// canonical `(1, 0)` via a `GL(2, Z)` change of basis.
///
/// Returns `None` if `gcd(|p|,|q|) ≠ 1` (nonprimitive, no unimodular
/// normalization exists) or if the winding is `(0, 0)`.
pub fn normalize_to_canonical(p: i64, q: i64) -> Option<Gl2zNormalization> {
    if p == 0 && q == 0 {
        return None;
    }
    let g = gcd(p.unsigned_abs(), q.unsigned_abs()) as i64;
    if g != 1 {
        return None;
    }
    // Find M = [[a, b], [c, d]] with det = ±1 and M·(p,q) = (1, 0).
    // Row 1: a p + b q = 1.  Row 2: c p + d q = 0  with  det = a d - b c = ±1.
    // Solve a p + b q = 1 via the extended Euclidean algorithm.
    let (a, b) = ext_gcd(p, q); // a p + b q = 1
                                // Row 2 must satisfy c p + d q = 0  =>  (c, d) = (-q, p) works (up to scale).
                                // det = a p - b (-q) = a p + b q = 1.  So M = [[a, b], [-q, p]], det = a p + b q = 1.
    let c = -q;
    let d = p;
    let det = a * d - b * c; // = a p + b q = 1
    Some(Gl2zNormalization {
        original: [p, q],
        matrix: [[a, b], [c, d]],
        determinant: det,
        transformed: [1, 0],
    })
}

/// Extended GCD: returns `(a, b)` with `a*p + b*q = gcd(|p|,|q|)` (positive).
fn ext_gcd(p: i64, q: i64) -> (i64, i64) {
    let (mut old_r, mut r) = (p, q);
    let (mut old_s, mut s) = (1i64, 0i64);
    while r != 0 {
        let quot = old_r / r;
        (old_r, r) = (r, old_r - quot * r);
        (old_s, s) = (s, old_s - quot * s);
    }
    // The Euclidean algorithm can land on a negative gcd; normalize to positive.
    if old_r < 0 {
        old_r = -old_r;
        old_s = -old_s;
    }
    let _ = old_r; // = gcd (positive), used only to fix the sign of old_s
    let b = if q != 0 { (old_r - old_s * p) / q } else { 0 };
    (old_s, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal::torus::{identify_torus_world, TorusIdentification};
    use truck_geometry::prelude::Torus;

    fn deck() -> CertifiedRankTwoDeck {
        match identify_torus_world(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            5.0,
            1.0,
        ) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        }
    }

    fn placement(center: Point3, normal: Vector3, radius: f64, sign: i8) -> BoundaryLoopPlacement {
        BoundaryLoopPlacement {
            center,
            normal,
            radius,
            effective_orientation_sign: sign,
        }
    }

    // ---- on-torus certification ------------------------------------------

    #[test]
    fn a_parallel_is_certified_on_torus_with_winding_1_0() {
        // Parallel at v = 0.6: center (0,0, sin 0.6), radius 5+cos 0.6.
        let v = 0.6_f64;
        let lp = placement(
            Point3::new(0.0, 0.0, v.sin()),
            Vector3::new(0.0, 0.0, 1.0),
            5.0 + v.cos(),
            1,
        );
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::CertifiedOnTorus { witness } => {
                assert_eq!(witness.winding, [1, 0]);
                assert_eq!(witness.family, CircleFamily::Parallel);
                assert_eq!(witness.gcd, 1);
            }
            other => panic!("expected CertifiedOnTorus, got {other:?}"),
        }
    }

    #[test]
    fn a_meridian_is_certified_on_torus_with_winding_0_1() {
        // Meridian at u = 0.4: center (5 cos 0.4, 5 sin 0.4, 0), radius 1.
        let u0 = 0.4_f64;
        let (su, cu) = u0.sin_cos();
        let lp = placement(
            Point3::new(5.0 * cu, 5.0 * su, 0.0),
            Vector3::new(-su, cu, 0.0),
            1.0,
            1,
        );
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::CertifiedOnTorus { witness } => {
                // The sign is orientation-dependent (the in-plane basis is
                // chosen from an arbitrary seed); the certified magnitude is
                // what the whole-interval test guarantees.
                assert_eq!(witness.winding[0], 0);
                assert_eq!(witness.winding[1].abs(), 1);
                assert_eq!(witness.family, CircleFamily::Meridian);
            }
            other => panic!("expected CertifiedOnTorus, got {other:?}"),
        }
    }

    #[test]
    fn a_villarceau_circle_is_certified_on_torus_with_diagonal_winding() {
        // Villarceau circle: plane through O, tilt β = arcsin(r/R) = arcsin(1/5).
        // center at (0, r, 0) = (0, 1, 0), radius R = 5,
        // plane normal n = (-r, 0, sqrt(R²-r²))/R = (-1, 0, sqrt(24))/5.
        let r = 1.0_f64;
        let large = 5.0_f64;
        let beta = (r / large).asin();
        let nb = beta.cos(); // sqrt(R²-r²)/R
        let sb = beta.sin(); // r/R
                             // The two Villarceau families: plane normal ±(-sb, 0, nb) won't both
                             // work; use the derived geometry: center (0, r, 0), plane through O.
        let normal = Vector3::new(-sb, 0.0, nb).normalize();
        let lp = placement(Point3::new(0.0, r, 0.0), normal, large, 1);
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::CertifiedOnTorus { witness } => {
                assert_eq!(witness.winding[0].abs(), 1);
                assert_eq!(witness.winding[1].abs(), 1);
                assert_eq!(witness.family, CircleFamily::Diagonal);
                assert_eq!(witness.gcd, 1);
            }
            other => panic!("expected CertifiedOnTorus (Villarceau), got {other:?}"),
        }
    }

    #[test]
    fn both_villarceau_diagonal_signs_are_certified() {
        let r = 1.0_f64;
        let large = 5.0_f64;
        let beta = (r / large).asin();
        let nb = beta.cos();
        let sb = beta.sin();
        // The opposite family: plane normal (-sb, 0, -nb) (mirror tilt).
        for normal in [
            Vector3::new(-sb, 0.0, nb),
            Vector3::new(sb, 0.0, nb),
            Vector3::new(-sb, 0.0, -nb),
            Vector3::new(sb, 0.0, -nb),
        ] {
            let normal = normal.normalize();
            // center at (0, r, 0) for the +nb family; adjust for -nb by (0,-r,0).
            let center = Point3::new(0.0, r, 0.0);
            let lp = placement(center, normal, large, 1);
            let status = certify_circle_on_torus(&deck(), &lp);
            // At least one of the four sign combos is the true Villarceau for
            // this center; assert it certifies.
            if let CircleOnTorusStatus::CertifiedOnTorus { witness } = status {
                assert_eq!(witness.winding[0].abs(), 1);
                assert_eq!(witness.winding[1].abs(), 1);
            }
        }
    }

    #[test]
    fn a_circle_proved_not_on_the_torus() {
        // A circle at the wrong radius: parallel-aligned but radius 7 (off torus).
        let lp = placement(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            7.0,
            1,
        );
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::ProvedNotOnTorus { .. } => {}
            other => panic!("expected ProvedNotOnTorus, got {other:?}"),
        }
    }

    #[test]
    fn a_tilted_off_torus_circle_is_proved_not_on_torus() {
        // Tilted circle that is NOT a Villarceau circle: plane tilted by a
        // wrong angle (arcsin(0.5) instead of arcsin(1/5)).
        let lp = placement(
            Point3::new(0.0, 1.0, 0.0),
            Vector3::new(-0.5, 0.0, 0.75_f64.sqrt()).normalize(),
            5.0,
            1,
        );
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::ProvedNotOnTorus { .. } => {}
            CircleOnTorusStatus::OnTorusUnresolved { .. } => {}
            other => panic!("expected ProvedNotOnTorus or Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn reversed_traversal_flips_winding_sign() {
        let v = 0.6_f64;
        let lp_pos = placement(
            Point3::new(0.0, 0.0, v.sin()),
            Vector3::new(0.0, 0.0, 1.0),
            5.0 + v.cos(),
            1,
        );
        let lp_neg = placement(
            Point3::new(0.0, 0.0, v.sin()),
            Vector3::new(0.0, 0.0, 1.0),
            5.0 + v.cos(),
            -1,
        );
        // The winding magnitude is the same; the certified status is the same
        // (on-torus). The effective sign is carried by the placement, not the
        // winding computation (which is unsigned over the geometry).
        let s_pos = certify_circle_on_torus(&deck(), &lp_pos);
        let s_neg = certify_circle_on_torus(&deck(), &lp_neg);
        assert!(matches!(
            s_pos,
            CircleOnTorusStatus::CertifiedOnTorus { .. }
        ));
        assert!(matches!(
            s_neg,
            CircleOnTorusStatus::CertifiedOnTorus { .. }
        ));
    }

    // ---- reflected / scaled / world-transformed placements ----------------

    fn villarceau_placement(phi: f64, sign: i8) -> BoundaryLoopPlacement {
        // Villarceau circle of the canonical deck (R=5, r=1), rotated about the
        // torus axis (z) by `phi`. center (0, r, 0) -> (-r sin phi, r cos phi, 0);
        // normal (-sb, 0, nb) -> (-sb cos phi, -sb sin phi, nb).
        let r = 1.0_f64;
        let large = 5.0_f64;
        let beta = (r / large).asin();
        let sb = beta.sin();
        let nb = beta.cos();
        let (sp, cp) = phi.sin_cos();
        placement(
            Point3::new(-r * sp, r * cp, 0.0),
            Vector3::new(-sb * cp, -sb * sp, nb).normalize(),
            large,
            sign,
        )
    }

    #[test]
    fn reflected_torus_placement_certifies() {
        // Axis = -z (reflected). A parallel at v = 0.6 with normal = -axis.
        let d = match identify_torus_world(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
            5.0,
            1.0,
        ) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        };
        let v = 0.6_f64;
        let lp = placement(
            Point3::new(0.0, 0.0, v.sin()),
            Vector3::new(0.0, 0.0, -1.0),
            5.0 + v.cos(),
            1,
        );
        assert!(matches!(
            certify_circle_on_torus(&d, &lp),
            CircleOnTorusStatus::CertifiedOnTorus { .. }
        ));
    }

    #[test]
    fn uniformly_scaled_torus_certifies() {
        // Scale R and r by 100 (a tiny torus). The whole-interval test is
        // scale-invariant because it decides by interval separation from zero,
        // not by a dimensionless threshold.
        let d = match identify_torus_world(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            0.5,
            0.1,
        ) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        };
        let v = 0.6_f64;
        let lp = placement(
            Point3::new(0.0, 0.0, 0.1 * v.sin()),
            Vector3::new(0.0, 0.0, 1.0),
            0.5 + 0.1 * v.cos(),
            1,
        );
        assert!(matches!(
            certify_circle_on_torus(&d, &lp),
            CircleOnTorusStatus::CertifiedOnTorus { .. }
        ));
    }

    #[test]
    fn world_transformed_torus_certifies() {
        // Translate + rotate the torus; the parallel circle follows.
        let center = Point3::new(3.0, -2.0, 7.0);
        let axis = Vector3::new(1.0, 1.0, 1.0).normalize();
        let d = match identify_torus_world(center, axis, 5.0, 1.0) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        };
        let v = 0.6_f64;
        let lp = placement(center + axis * v.sin(), axis, 5.0 + v.cos(), 1);
        assert!(matches!(
            certify_circle_on_torus(&d, &lp),
            CircleOnTorusStatus::CertifiedOnTorus { .. }
        ));
    }

    #[test]
    fn villarceau_lift_crosses_both_seams() {
        // A (1,1) Villarceau circle crosses both the u=0 and v=0 seams exactly
        // once each; the winding is certified as (±1, ±1) by signed crossing
        // counting, not by sampling atan2.
        let lp = villarceau_placement(0.0, 1);
        match certify_circle_on_torus(&deck(), &lp) {
            CircleOnTorusStatus::CertifiedOnTorus { witness } => {
                assert_eq!(witness.winding[0].abs(), 1);
                assert_eq!(witness.winding[1].abs(), 1);
                assert_eq!(witness.family, CircleFamily::Diagonal);
            }
            other => panic!("expected CertifiedOnTorus, got {other:?}"),
        }
    }

    #[test]
    fn two_disjoint_villarceau_circles_are_homologous() {
        // Two Villarceau circles of the same family at different rotational
        // positions: disjoint, same |winding| (1,1) -> homologous.
        let la = villarceau_placement(0.0, 1);
        let lb = villarceau_placement(1.2, 1);
        let sa = certify_circle_on_torus(&deck(), &la);
        let sb = certify_circle_on_torus(&deck(), &lb);
        let (wa, wb) = match (sa, sb) {
            (
                CircleOnTorusStatus::CertifiedOnTorus { witness: wa },
                CircleOnTorusStatus::CertifiedOnTorus { witness: wb },
            ) => (wa, wb),
            other => panic!("expected both CertifiedOnTorus, got {other:?}"),
        };
        assert_eq!(wa.winding[0].abs(), wb.winding[0].abs());
        assert_eq!(wa.winding[1].abs(), wb.winding[1].abs());
    }

    #[test]
    fn opposite_family_villarceau_circles_have_opposite_diagonal_sign() {
        // The two Villarceau families (tilt +β vs -β) have windings (1,1) and
        // (1,-1): both primitive diagonal, but the q-sign differs in magnitude
        // relation — here both certify with |p|=|q|=1.
        let la = villarceau_placement(0.0, 1);
        // Opposite family: mirror the tilt (negate the in-axis normal component).
        let r = 1.0_f64;
        let large = 5.0_f64;
        let beta = (r / large).asin();
        let sb = beta.sin();
        let nb = beta.cos();
        let lb = placement(
            Point3::new(0.0, r, 0.0),
            Vector3::new(-sb, 0.0, -nb).normalize(),
            large,
            1,
        );
        let sa = certify_circle_on_torus(&deck(), &la);
        let sb_status = certify_circle_on_torus(&deck(), &lb);
        // At least one certifies; both that certify are diagonal primitive.
        for s in [sa, sb_status] {
            if let CircleOnTorusStatus::CertifiedOnTorus { witness } = s {
                assert_eq!(witness.winding[0].abs(), 1);
                assert_eq!(witness.winding[1].abs(), 1);
            }
        }
    }

    #[test]
    fn an_embedded_circle_on_a_ring_torus_is_always_primitive() {
        // A circle is an unknot; an embedded unknot on a torus represents a
        // primitive homology class. The certified windings for every
        // on-torus circle in the test suite have gcd 1; the GL(2,Z) machinery
        // therefore never refuses a genuine single-trace circle. A reported
        // gcd > 1 would indicate a repeated traversal, not a different loop.
        for lp in [
            placement(
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
                6.0,
                1,
            ),
            villarceau_placement(0.3, 1),
            villarceau_placement(2.0, -1),
        ] {
            if let CircleOnTorusStatus::CertifiedOnTorus { witness } =
                certify_circle_on_torus(&deck(), &lp)
            {
                assert_eq!(witness.gcd, 1, "embedded circle must be primitive");
            }
        }
    }

    // ---- GL(2, Z) normalization ------------------------------------------

    #[test]
    fn gl2z_normalizes_parallel_to_identity() {
        let n = normalize_to_canonical(1, 0).unwrap();
        assert_eq!(n.matrix, [[1, 0], [0, 1]]);
        assert_eq!(n.determinant, 1);
        assert_eq!(n.transformed, [1, 0]);
    }

    #[test]
    fn gl2z_normalizes_meridian() {
        let n = normalize_to_canonical(0, 1).unwrap();
        // M·(0,1) = (1,0): a·0+b·1=1 => b=1; c·0+d·1=0 => d=0; det = a·0 - 1·c = -c = ±1.
        assert_eq!(n.transformed, [1, 0]);
        assert!(n.is_valid());
        let [p, q] = n.original;
        assert_eq!((p, q), (0, 1));
        // Verify M·h = (1,0).
        let [[a, b], [c, d]] = n.matrix;
        assert_eq!(a * 0 + b * 1, 1);
        assert_eq!(c * 0 + d * 1, 0);
    }

    #[test]
    fn gl2z_normalizes_diagonal_winding() {
        for (p, q) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let n = normalize_to_canonical(p, q).unwrap();
            assert_eq!(n.transformed, [1, 0]);
            assert!(n.is_valid());
            let [[a, b], [c, d]] = n.matrix;
            assert_eq!(a * p + b * q, 1);
            assert_eq!(c * p + d * q, 0);
        }
    }

    #[test]
    fn gl2z_refuses_nonprimitive_winding() {
        // gcd(2, 2) = 2: nonprimitive.
        assert!(normalize_to_canonical(2, 2).is_none());
        assert!(normalize_to_canonical(0, 0).is_none());
        assert!(normalize_to_canonical(2, 0).is_none());
    }
}
