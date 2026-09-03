#![deny(clippy::unwrap_used)]

//! BG-KV2-203-C1DELTA — the Pythagorean-hodograph fast-path spines
//! (spec §5.2's `PhSpine`), r3-rescoped.
//!
//! # H-1
//! truck-geometry's crate header (`src/lib.rs`) does not deny `unwrap_used`
//! crate-wide; this module follows the sibling constructive modules' header
//! style (`#![deny(clippy::unwrap_used)]`) and carries no `unwrap`/`expect`/
//! `panic!` and no module-level `allow`.
//!
//! # The two subclasses
//!
//! Only Pythagorean-hodograph (PH) curves can carry rational rotation-
//! minimizing frames (rational unit tangent is necessary, not sufficient; a
//! general PH curve's RMF involves logarithmic terms). The spec names exactly
//! the two characterized subclasses:
//!
//! - [`RmErfSeptic`] — a degree-7 PH curve whose Euler–Rodrigues (ER) frame is
//!   rotation-minimizing. Membership is the polynomial identity `tau == 0`,
//!   where `tau` is the ER-frame spin about the tangent. This module
//!   implements exactly that identity check on the cubic-quaternion preimage
//!   coefficients recovered from the Bézier net (coefficient identities, no
//!   transcendental). The admitted test fixtures are the PLANAR degenerate
//!   family (`A(w) = u(w) + p(w)·j`, whose ER frame is the planar RMF);
//!   flagged as degenerate in the fixture doc. A spatial non-degenerate
//!   fixture family is the published M3 characterization (Farouki and
//!   co-authors, spec §23) and is DEFERRED, not fabricated.
//!
//! - [`RrmfQuintic`] — a quintic PH curve satisfying the RRMF condition
//!   (rational RMF). The enum variant FREEZES per the spec's spelling; its
//!   constructor REFUSES with named evidence [`PendingMembership`]: the M1
//!   membership condition and the M2 closed-form rational RMF are external
//!   published mathematics (Farouki et al.) that neither the spec body nor
//!   the packet supplies, and §23's own rule forbids landing an unproved
//!   external dependency. The refusal is a recorded deferral with a named
//!   trigger; it is never approximated by double reflection (that would erase
//!   the fast path's reason to exist).
//!
//! # The frame algebra (r2 derivation, adopted as packet content)
//!
//! With quaternion preimage `A(w) = u + v·i + p·j + q·k` (real polynomials):
//!
//! - hodograph `c'(w) = A·i·A*` = `(u²+v²−p²−q², 2(vp+uq), 2(vq−up))`;
//! - parametric speed `σ = |A|² = u²+v²+p²+q²` is a polynomial, so
//!   `|c'|² = σ²`;
//! - the ER frame is rational in `w`:
//!   `e1 = c'/σ`, `e2 = (2(vp−uq), u²−v²+p²−q², 2(pq+uv))/σ`,
//!   `e3 = (2(vq+up), 2(pq−uv), u²−v²−p²+q²)/σ`, orthonormal and
//!   right-handed (`e1 × e2 = e3`);
//! - the ER spin about the tangent is `τ = 2(u·v' − v·u' − p·q' + q·p')/σ`;
//! - the RMF is the ER frame rotated by `θ` about the tangent with
//!   `θ' = −τ`, so the ER frame IS the rotation-minimizing frame iff
//!   `τ(w) == 0` as a polynomial identity in the preimage coefficients.

use super::errors::ConstructError;
use super::recipe::SpineCurve;
use super::{DirectTolerance, Frame3};
use truck_base::cgmath64::*;

/// Coefficient-identity tolerance (relative): the polynomial checks
/// (`|c'|² == σ²`, `τ == 0`) hold to rounding (~1e-12 relative for the
/// exact planar fixtures); anything below this bound is read as the identity.
/// // H-3
const IDENTITY_RELATIVE_TOL: f64 = 1.0e-7;

/// The membership (and structural) validation outcome of a degree-7 net.
#[derive(Debug, Clone, PartialEq)]
pub enum SepticMembership {
    /// The net is a degree-7 PH curve whose Euler–Rodrigues frame is
    /// rotation-minimizing (`τ == 0`).
    ErfRmf,
    /// The net is not a PH curve at all (`|c'|²` is not a perfect square).
    NotPh,
    /// A PH curve whose ER frame is NOT rotation-minimizing: `τ` is not the
    /// zero polynomial. Carries the supremum of `|τ(w)|` over `[0, 1]` in
    /// units of the parametric-speed scale.
    NotErfRmf {
        /// The maximum `|τ(w)|` sampled over the domain, speed-scaled.
        tau_sup: f64,
    },
}

/// The typed refusal of a [deferred `RrmfQuintic` construction](RrmfQuintic).
///
/// Geometry-local spelling of the §2/§17 named refusal: the K1
/// `Refusal`/`RefusalKind`/`RefusalEvidence` types live in `truck-certified`,
/// which `truck-geometry` cannot depend on (the dependency direction is the
/// reverse), so the deferral is carried here as the stable predicate name and
/// maps onto `RefusalKind::Budget` / `VerdictClass::Inconclusive` /
/// `RefusalEvidence::Predicate` at the certified boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingMembership {
    /// The stable, machine-readable predicate name (kebab-case): the named
    /// trigger of the deferral.
    pub predicate: &'static str,
}

impl PendingMembership {
    /// The stable predicate name of the deferred RRMF membership
    /// characterization.
    pub const RRMF_MEMBERSHIP_PENDING: &'static str =
        "rrmf_membership_pending_external_characterization";
    /// The §17 refusal kind this deferral maps onto (`Budget`).
    pub const KIND: &'static str = "Budget";
    /// The §17 backing class of that kind (`Inconclusive`).
    pub const BACKING: &'static str = "Inconclusive";
}

/// A quintic PH curve satisfying the RRMF condition — the enum variant
/// FREEZES (spec §5.2: "the enum names exactly those two"); the constructor
/// refuses with named evidence while the RRMF membership condition (M1) and
/// the closed-form rational rotation-minimizing frame (M2) remain external
/// published mathematics not supplied in-tree (spec §23; do not approximate
/// with double reflection).
#[derive(Debug, Clone, PartialEq)]
pub struct RrmfQuintic {
    /// The degree-5 Bézier control net of the quintic PH curve.
    control_points: [Point3; 6],
}

impl RrmfQuintic {
    /// Attempts to validate and build an RRMF quintic from its degree-5
    /// Bézier control net.
    ///
    /// This ALWAYS refuses while the RRMF membership characterization and the
    /// rational-RMF closed form are external (see [`PendingMembership`]). The
    /// refusal is a recorded deferral with a named trigger; when the
    /// characterization lands, this constructor's validation body is the
    /// change point.
    pub fn try_new(control_points: [Point3; 6]) -> Result<RrmfQuintic, PendingMembership> {
        let _ = control_points;
        Err(PendingMembership {
            predicate: PendingMembership::RRMF_MEMBERSHIP_PENDING,
        })
    }
}

impl SpineCurve for RrmfQuintic {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        validate_parameter(s)?;
        Ok(de_casteljau(&self.control_points, s))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        validate_parameter(s)?;
        let mut net = [Vector3::zero(); 6];
        for (i, slot) in net.iter_mut().enumerate() {
            *slot = (self.control_points[i + 1] - self.control_points[i]) * 5.0;
        }
        Ok(de_casteljau_vec(&net[..5], s))
    }
}

/// A degree-7 PH curve whose Euler–Rodrigues frame is rotation-minimizing
/// (spec §5.2's `RmErfSeptic`): membership is the `τ == 0` polynomial
/// identity, implemented exactly on the cubic-quaternion preimage recovered
/// from the Bézier net.
///
/// Data: the degree-7 Bézier control net (PH curves are polynomial — Bézier
/// form is exact). The constructor validates the PH property
/// (`|c'(w)|² == σ(w)²`, a control-coefficient identity) and the membership
/// identity `τ == 0`. For a member the ER frame IS the exact rational
/// rotation-minimizing frame (no ODE, no double reflection), and the
/// parametric speed is polynomial, so arc length is exact.
///
/// Membership is COMPLETE for every net whose cubic preimage is recovered by
/// the identity solve. The non-degenerate spatial fixture family remains the
/// deferred M3 characterization; the admitted test fixtures are the planar
/// degenerate family (`A = u(w) + p(w)·j`, `v = q = 0`), flagged degenerate
/// in the fixture doc.
#[derive(Debug, Clone, PartialEq)]
pub struct RmErfSeptic {
    /// The degree-7 Bézier control net (8 points).
    control_points: [Point3; 8],
    /// The cubic quaternion preimage coefficients recovered at validation
    /// (power basis, `[c0, c1, c2, c3]` per component).
    preimage: [Poly; 4],
    /// The parametric speed `σ = |c'|` (power-basis sextic, 7 coefficients).
    speed: Poly,
}

/// A power-basis real polynomial, ascending coefficients.
type Poly = Vec<f64>;

impl RmErfSeptic {
    /// Validates and builds the degree-7 ERF-RMF spine from its Bézier
    /// control net.
    ///
    /// The validation is three coefficient-identity checks, no transcendental:
    ///
    /// 1. the sextic hodograph `d = c'` (net differences) satisfies
    ///    `d·d == σ²` for a real sextic `σ` — the PH property;
    /// 2. the cubic quaternion preimage `A` is recovered from the linear
    ///    identity `d·A == σ·A·i` (constant-complex gauge is irrelevant to
    ///    the frame and the spin);
    /// 3. membership: `τ = 2(u v' − v u' − p q' + q p') == 0` as a
    ///    polynomial identity — the ER frame is the exact rotation-minimizing
    ///    frame.
    ///
    /// Refusals: structural (non-finite, degenerate speed) → `NotPh`; a net
    /// that fails the PH square check → `NotPh`; a genuine PH net whose ER
    /// frame is not rotation-minimizing → `NotErfRmf`.
    pub fn try_new(control_points: [Point3; 8]) -> Result<RmErfSeptic, SepticMembership> {
        if control_points.iter().any(|p| !p.is_finite()) {
            return Err(SepticMembership::NotPh);
        }
        let hodograph = sextic_hodograph_power(&control_points);
        let speed = match ph_speed_of(&hodograph) {
            Some(speed) => speed,
            None => return Err(SepticMembership::NotPh),
        };
        if speed.iter().any(|c| !c.is_finite()) {
            return Err(SepticMembership::NotPh);
        }
        if !positive_on_unit_interval(&speed) {
            return Err(SepticMembership::NotPh);
        }
        let preimage = match recover_preimage(&hodograph, &speed) {
            Some(preimage) => preimage,
            None => return Err(SepticMembership::NotPh),
        };
        let spin = spin_numerator(&preimage);
        let scale = speed_scale(&speed);
        let max_abs = spin.iter().fold(0.0f64, |m, c| m.max(c.abs()));
        if max_abs > IDENTITY_RELATIVE_TOL * scale {
            let tau_sup = spin_sup_over_unit_interval(&spin, &speed);
            return Err(SepticMembership::NotErfRmf { tau_sup });
        }
        Ok(RmErfSeptic {
            control_points,
            preimage,
            speed,
        })
    }

    /// The stored degree-7 Bézier control net.
    pub fn control_points(&self) -> [Point3; 8] {
        self.control_points
    }

    /// The recovered parametric-speed polynomial coefficients (power basis).
    pub fn speed(&self) -> &[f64] {
        &self.speed
    }

    /// The recovered cubic-quaternion preimage component polynomials, in the
    /// order `[u, v, p, q]` (each a power-basis cubic).
    pub fn preimage(&self) -> [&[f64]; 4] {
        [
            &self.preimage[0],
            &self.preimage[1],
            &self.preimage[2],
            &self.preimage[3],
        ]
    }

    /// The exact polynomial arc length from `0` to `s`: for a PH curve the
    /// speed `σ` is a polynomial, so `L(s) = ∫₀ˢ σ` is a closed-form
    /// polynomial antiderivative (exact arc length for chord sampling).
    pub fn arc_length(&self, s: f64) -> Result<f64, ConstructError> {
        validate_parameter(s)?;
        let mut length = 0.0;
        let mut power = s;
        for (k, &c) in self.speed.iter().enumerate() {
            length += c * power / (k as f64 + 1.0);
            power *= s;
        }
        Ok(length)
    }

    /// The exact rational rotation-minimizing frame at `s`: for a member the
    /// ER frame IS the RMF, so `(e1, e2, e3)` (normal = `e2`, binormal =
    /// `e3`, `t × n == b`) is the frame — no ODE, no double reflection.
    pub fn frame_at(&self, s: f64) -> Result<Frame3, ConstructError> {
        validate_parameter(s)?;
        let [u, v, p, q] = eval_preimage(&self.preimage, s);
        let sigma = u * u + v * v + p * p + q * q;
        if sigma <= DirectTolerance::default().position {
            return Err(ConstructError::ZeroTangent { at: s });
        }
        let tangent = Vector3::new(
            u * u + v * v - p * p - q * q,
            2.0 * (v * p + u * q),
            2.0 * (v * q - u * p),
        ) / sigma;
        let normal = Vector3::new(
            2.0 * (v * p - u * q),
            u * u - v * v + p * p - q * q,
            2.0 * (p * q + u * v),
        ) / sigma;
        let binormal = Vector3::new(
            2.0 * (v * q + u * p),
            2.0 * (p * q - u * v),
            u * u - v * v - p * p + q * q,
        ) / sigma;
        Ok(Frame3 {
            tangent,
            normal,
            binormal,
        })
    }
}

impl SpineCurve for RmErfSeptic {
    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        validate_parameter(s)?;
        Ok(de_casteljau(&self.control_points, s))
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        validate_parameter(s)?;
        let mut net = [Vector3::zero(); 7];
        for (i, slot) in net.iter_mut().enumerate() {
            *slot = (self.control_points[i + 1] - self.control_points[i]) * 7.0;
        }
        Ok(de_casteljau_vec(&net, s))
    }
}

/// The `PhSpine` enum: the two characterized PH subclasses (spec §5.2).
///
/// The payloads are boxed so the `Spine` enum's `Ph` variant stays small (the
/// crate denies `clippy::all`, whose `large_enum_variant` would otherwise
/// fire against the multi-hundred-byte ERF-RMF net data; the `Box` is the
/// canonical truck indirection for exactly this).
#[derive(Debug, Clone, PartialEq)]
pub enum PhSpine {
    /// A quintic PH curve satisfying the RRMF condition. The variant freezes;
    /// its constructor refuses pending the external membership
    /// characterization (see [`RrmfQuintic`]).
    RrmfQuintic(Box<RrmfQuintic>),
    /// A degree-7 PH curve whose Euler–Rodrigues frame is rotation-minimizing.
    RmErfSeptic(Box<RmErfSeptic>),
}

impl SpineCurve for PhSpine {
    fn domain(&self) -> (f64, f64) {
        match self {
            PhSpine::RrmfQuintic(spine) => spine.domain(),
            PhSpine::RmErfSeptic(spine) => spine.domain(),
        }
    }

    fn position_at(&self, s: f64) -> Result<Point3, ConstructError> {
        match self {
            PhSpine::RrmfQuintic(spine) => spine.position_at(s),
            PhSpine::RmErfSeptic(spine) => spine.position_at(s),
        }
    }

    fn derivative_at(&self, s: f64) -> Result<Vector3, ConstructError> {
        match self {
            PhSpine::RrmfQuintic(spine) => spine.derivative_at(s),
            PhSpine::RmErfSeptic(spine) => spine.derivative_at(s),
        }
    }
}

// ---------------------------------------------------------------------------
// Polynomial helpers (power basis, ascending coefficients).
// ---------------------------------------------------------------------------

/// Validates a spine parameter: finite and inside the closed `[0, 1]` domain
/// (within the parameter tolerance).
fn validate_parameter(s: f64) -> Result<(), ConstructError> {
    if !s.is_finite() {
        return Err(ConstructError::NonFinite { at: s });
    }
    let tolerance = DirectTolerance::default().parameter;
    if s < -tolerance || s > 1.0 + tolerance {
        return Err(ConstructError::InvalidInput);
    }
    Ok(())
}

/// Trims trailing (near-)zero coefficients.
fn trim(poly: &mut Vec<f64>) {
    while poly.len() > 1 && poly[poly.len() - 1].abs() <= f64::MIN_POSITIVE {
        poly.pop();
    }
}

/// The Bernstein→power conversion coefficient of `B_i^n` at `w^j`.
fn bernstein_to_power_coefficient(n: usize, i: usize, j: usize) -> f64 {
    if j < i {
        return 0.0;
    }
    let sign = if (j - i).is_multiple_of(2) { 1.0 } else { -1.0 };
    sign * binom(n, i) as f64 * binom(n - i, j - i) as f64
}

/// The sextic hodograph `c'` of a degree-7 Bézier net, as three power-basis
/// polynomials of degree ≤ 6 (coefficients `[c0..c6]`).
fn sextic_hodograph_power(control_points: &[Point3; 8]) -> [Poly; 3] {
    let mut out: [Poly; 3] = [vec![0.0; 7], vec![0.0; 7], vec![0.0; 7]];
    let n = 6usize;
    for i in 0..7 {
        let diff = (control_points[i + 1] - control_points[i]) * 7.0;
        let comps = [diff.x, diff.y, diff.z];
        for (axis, value) in comps.iter().enumerate() {
            for (j, slot) in out[axis].iter_mut().enumerate().skip(i) {
                *slot += value * bernstein_to_power_coefficient(n, i, j);
            }
        }
    }
    for axis in out.iter_mut() {
        trim(axis);
    }
    out
}

/// `binom(n, k)` as `f64` via the multiplicative form (small `n`).
fn binom(n: usize, k: usize) -> u64 {
    let k = k.min(n - k);
    let mut value: u64 = 1;
    for i in 0..k {
        value = value * (n - i) as u64 / (i + 1) as u64;
    }
    value
}

/// The coefficient-wise polynomial square root of `d·d`: returns `Some(σ)`
/// (a real sextic, non-negative leading coefficient) when `d·d == σ²`
/// coefficient-wise within the identity tolerance, else `None`. `d·d` is the
/// degree-12 sum of the three component squares.
fn ph_speed_of(hodograph: &[Poly; 3]) -> Option<Poly> {
    let mut dd: Poly = vec![0.0; 13];
    for axis in hodograph {
        let sq = poly_mul(axis, axis);
        for (k, &c) in sq.iter().enumerate() {
            dd[k] += c;
        }
    }
    trim(&mut dd);
    let scale = dd.iter().fold(0.0f64, |m, c| m.max(c.abs())).max(1.0);
    if dd.is_empty() || dd[0] <= 0.0 {
        return None;
    }
    let mut sigma: Poly = vec![0.0; 7];
    sigma[0] = dd[0].sqrt();
    for k in 1..=6 {
        let mut acc = 0.0;
        for i in 1..k {
            acc += sigma[i] * sigma[k - i];
        }
        sigma[k] = (dd[k] - acc) / (2.0 * sigma[0]);
    }
    // Verify the remaining (over-determined) coefficients 7..=12 of σ² == d·d.
    let rebuilt = poly_mul(&sigma, &sigma);
    for k in 0..=12 {
        let want = if k < dd.len() { dd[k] } else { 0.0 };
        let got = if k < rebuilt.len() { rebuilt[k] } else { 0.0 };
        if (got - want).abs() > IDENTITY_RELATIVE_TOL * scale {
            return None;
        }
    }
    Some(sigma)
}

/// A quick positivity sanity scan of `σ` over `[0, 1]`: the parametric speed
/// must be positive so the tangent never vanishes on the spine domain.
fn positive_on_unit_interval(sigma: &[f64]) -> bool {
    for i in 0..=64 {
        let w = i as f64 / 64.0;
        if poly_eval(sigma, w) <= 0.0 {
            return false;
        }
    }
    true
}

/// The recovered cubic-quaternion preimage `[u, v, p, q]` (each a power-basis
/// cubic) solving `d·A == σ·A·i`, or `None` when the identity is not
/// recoverable (not a PH curve, or a degenerate solve).
fn recover_preimage(hodograph: &[Poly; 3], speed: &[f64]) -> Option<[Poly; 4]> {
    // Unknowns: A(w) = Σ_{k=0..3} A_k w^k, A_k a quaternion (scalar, i, j, k).
    const UNKNOWNS: usize = 16;
    // Rows: degree m in 0..=9 (max deg d + deg A = 6 + 3), 4 quaternion rows.
    let mut matrix: Vec<[f64; UNKNOWNS]> = Vec::with_capacity(40);
    for m in 0..=9usize {
        let mut rows = [[0.0f64; UNKNOWNS]; 4];
        for j in 0..=m {
            let k = m - j;
            if k > 3 || j > 6 {
                continue;
            }
            let d_j = [
                if j < hodograph[0].len() {
                    hodograph[0][j]
                } else {
                    0.0
                },
                if j < hodograph[1].len() {
                    hodograph[1][j]
                } else {
                    0.0
                },
                if j < hodograph[2].len() {
                    hodograph[2][j]
                } else {
                    0.0
                },
            ];
            let sigma_j = if j < speed.len() { speed[j] } else { 0.0 };
            let left = left_mul_matrix(d_j);
            let right = right_mul_i_matrix();
            for r in 0..4 {
                for c in 0..4 {
                    let base = k * 4 + c;
                    rows[r][base] += left[r][c] - sigma_j * right[r][c];
                }
            }
        }
        for row in rows.iter() {
            matrix.push(*row);
        }
    }

    let null_basis = null_space(&matrix);
    if null_basis.is_empty() {
        return None;
    }
    let vector = &null_basis[0];
    let mut preimage: [Poly; 4] = [vec![0.0; 4], vec![0.0; 4], vec![0.0; 4], vec![0.0; 4]];
    for k in 0..4 {
        for c in 0..4 {
            preimage[c][k] = vector[k * 4 + c];
        }
    }
    for poly in preimage.iter_mut() {
        trim(poly);
    }
    // The solve must have produced a non-trivial preimage whose hodograph
    // reproduces the input direction (up to the constant complex gauge).
    let scale = hodograph_scale(hodograph);
    if preimage.iter().all(|poly| poly.is_empty()) {
        return None;
    }
    let [u, v, p, q] = &preimage;
    let tangent_ok = (0..=8).all(|i| {
        let w = i as f64 / 8.0;
        let (a, b, c, d) = (
            poly_eval(u, w),
            poly_eval(v, w),
            poly_eval(p, w),
            poly_eval(q, w),
        );
        let sigma_a = a * a + b * b + c * c + d * d;
        if sigma_a <= 1.0e-30 {
            return false;
        }
        let hod = [
            poly_eval(&hodograph[0], w),
            poly_eval(&hodograph[1], w),
            poly_eval(&hodograph[2], w),
        ];
        let speed_w = poly_eval(speed, w);
        // A·i·A* must be parallel to d and carry the same speed relation:
        // with σ_A = |A|² we need (A·i·A*)·σ == d·σ_A componentwise.
        let recovered = Vector3::new(
            a * a + b * b - c * c - d * d,
            2.0 * (b * c + a * d),
            2.0 * (b * d - a * c),
        );
        let recovered_ok = (recovered.x * speed_w - hod[0] * sigma_a).abs()
            + (recovered.y * speed_w - hod[1] * sigma_a).abs()
            + (recovered.z * speed_w - hod[2] * sigma_a).abs();
        recovered_ok <= IDENTITY_RELATIVE_TOL * scale * (1.0 + sigma_a)
    });
    if !tangent_ok {
        return None;
    }
    Some(preimage)
}

/// The left-multiplication matrix `L_v` of a pure-vector quaternion
/// `v = (0, x, y, z)` on the 4-vector `(scalar, i, j, k)`.
fn left_mul_matrix(v: [f64; 3]) -> [[f64; 4]; 4] {
    let [x, y, z] = v;
    [
        [0.0, -x, -y, -z],
        [x, 0.0, -z, y],
        [y, z, 0.0, -x],
        [z, -y, x, 0.0],
    ]
}

/// The right-multiplication-by-`i` matrix: `A·i = (−a1, a0, a3, −a2)`.
fn right_mul_i_matrix() -> [[f64; 4]; 4] {
    [
        [0.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0, 0.0],
    ]
}

/// Returns a basis of the (approximate) right null space of `matrix` by
/// Gauss–Jordan elimination with column pivoting. Rows that exceed the
/// identity tolerance are treated as zero.
fn null_space(matrix: &[[f64; 16]]) -> Vec<[f64; 16]> {
    let rows = matrix.len();
    let cols = 16;
    let global_scale = matrix
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0f64, |m, c| m.max(c.abs()))
        .max(1.0);
    let epsilon = IDENTITY_RELATIVE_TOL * global_scale;
    let mut work: Vec<[f64; 16]> = matrix.to_vec();
    let mut pivot_row_of_col = [usize::MAX; 16];
    let mut row = 0usize;
    for col in 0..cols {
        // Find the pivot row: the largest |entry| in this column at or below
        // the current row.
        let mut best = row;
        let mut best_value = 0.0f64;
        for (r, wrow) in work.iter().enumerate().skip(row) {
            let magnitude = wrow[col].abs();
            if magnitude > best_value {
                best_value = magnitude;
                best = r;
            }
        }
        if best_value <= epsilon {
            continue; // free column
        }
        work.swap(row, best);
        let pivot = work[row][col];
        for entry in work[row].iter_mut() {
            *entry /= pivot;
        }
        let pivot_row: [f64; 16] = work[row];
        for (r, wrow) in work.iter_mut().enumerate() {
            if r == row {
                continue;
            }
            let factor = wrow[col];
            if factor.abs() <= epsilon {
                continue;
            }
            for (c, entry) in wrow.iter_mut().enumerate() {
                *entry -= factor * pivot_row[c];
            }
        }
        pivot_row_of_col[col] = row;
        row += 1;
        if row >= rows {
            break;
        }
    }

    let free_cols: Vec<usize> = (0..cols)
        .filter(|&c| pivot_row_of_col[c] == usize::MAX)
        .collect();

    let mut basis = Vec::new();
    for &free in &free_cols {
        let mut vector = [0.0f64; 16];
        vector[free] = 1.0;
        // Back-substitute the pivot variables from their (normalized) rows.
        for (pivot_col, pivot_row) in pivot_row_of_col.iter().enumerate() {
            if *pivot_row == usize::MAX {
                continue;
            }
            vector[pivot_col] = -work[*pivot_row][free];
        }
        // Normalize so the largest magnitude entry is 1.
        let magnitude = vector.iter().fold(0.0f64, |m, c| m.max(c.abs()));
        if magnitude > 0.0 {
            for entry in vector.iter_mut() {
                *entry /= magnitude;
            }
            basis.push(vector);
        }
    }
    basis
}

/// The spin numerator `2(u v' − v u' − p q' + q p')` of the ER frame.
fn spin_numerator(preimage: &[Poly; 4]) -> Poly {
    let [u, v, p, q] = preimage;
    let u_der = poly_deriv(u);
    let v_der = poly_deriv(v);
    let p_der = poly_deriv(p);
    let q_der = poly_deriv(q);
    let mut numerator = poly_sub(&poly_mul(u, &v_der), &poly_mul(v, &u_der));
    numerator = poly_sub(&numerator, &poly_mul(p, &q_der));
    numerator = poly_add(&numerator, &poly_mul(q, &p_der));
    for c in numerator.iter_mut() {
        *c *= 2.0;
    }
    numerator
}

/// A speed-relative scale for the coefficient identities.
fn speed_scale(speed: &[f64]) -> f64 {
    speed.iter().fold(0.0f64, |m, c| m.max(c.abs())).max(1.0)
}

/// A scale of the hodograph coefficients.
fn hodograph_scale(hodograph: &[Poly; 3]) -> f64 {
    hodograph
        .iter()
        .flat_map(|p| p.iter())
        .fold(0.0f64, |m, c| m.max(c.abs()))
        .max(1.0)
}

/// The maximum of `|τ(w)|` over a coarse scan of `[0, 1]`, speed-scaled.
fn spin_sup_over_unit_interval(spin: &[f64], speed: &[f64]) -> f64 {
    let scale = speed_scale(speed);
    let mut sup = 0.0f64;
    for i in 0..=256 {
        let w = i as f64 / 256.0;
        sup = sup.max(poly_eval(spin, w).abs() / scale);
    }
    sup
}

/// Evaluates the preimage components at `w`.
fn eval_preimage(preimage: &[Poly; 4], w: f64) -> [f64; 4] {
    [
        poly_eval(&preimage[0], w),
        poly_eval(&preimage[1], w),
        poly_eval(&preimage[2], w),
        poly_eval(&preimage[3], w),
    ]
}

fn poly_add(a: &[f64], b: &[f64]) -> Poly {
    let mut out = a.to_vec();
    if b.len() > out.len() {
        out.resize(b.len(), 0.0);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] += c;
    }
    trim(&mut out);
    out
}

fn poly_sub(a: &[f64], b: &[f64]) -> Poly {
    let mut out = a.to_vec();
    if b.len() > out.len() {
        out.resize(b.len(), 0.0);
    }
    for (i, &c) in b.iter().enumerate() {
        out[i] -= c;
    }
    trim(&mut out);
    out
}

fn poly_mul(a: &[f64], b: &[f64]) -> Poly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ca) in a.iter().enumerate() {
        for (j, &cb) in b.iter().enumerate() {
            out[i + j] += ca * cb;
        }
    }
    trim(&mut out);
    out
}

fn poly_deriv(a: &[f64]) -> Poly {
    let mut out = Vec::with_capacity(a.len().saturating_sub(1));
    for (i, &c) in a.iter().enumerate().skip(1) {
        out.push(c * i as f64);
    }
    trim(&mut out);
    out
}

fn poly_eval(coeffs: &[f64], x: f64) -> f64 {
    coeffs.iter().rev().fold(0.0f64, |acc, &c| acc * x + c)
}

/// De Casteljau evaluation of a Bézier control polygon at `t`.
fn de_casteljau(points: &[Point3], t: f64) -> Point3 {
    let mut work: Vec<Point3> = points.to_vec();
    let mut level = points.len();
    while level > 1 {
        for i in 0..level - 1 {
            work[i] = work[i] + (work[i + 1] - work[i]) * t;
        }
        level -= 1;
    }
    work[0]
}

/// De Casteljau evaluation of a Bézier control polygon of vectors at `t`.
fn de_casteljau_vec(points: &[Vector3], t: f64) -> Vector3 {
    let mut work: Vec<Vector3> = points.to_vec();
    let mut level = points.len();
    while level > 1 {
        for i in 0..level - 1 {
            work[i] = work[i] + (work[i + 1] - work[i]) * t;
        }
        level -= 1;
    }
    work[0]
}
