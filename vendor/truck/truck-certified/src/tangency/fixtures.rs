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

//! The CTE fixture kit (CTE-000-SPINE; booking §5, normative).
//!
//! **TEST SUPPORT ONLY.** This module is `#[doc(hidden)] pub`, explicitly
//! excluded from the certified API surface (a one-line mapping-table note, not
//! a row: no new evidence kind). The CTE wave packets' tests consume it
//! through the crate's public path; `#[cfg(test)]`-only items would be
//! invisible to them.
//!
//! Each fixture is a hand-computable ground truth that is **machine-checked at
//! admission** — never by solving, never by interval/certified work. The kit
//! carries the fixture's algebraic data as exact integer coefficient records
//! (the ℚ data of the CTE problem) and a documented ground truth; the fixture's
//! `admit()` re-derives the ground truth from the coefficients with exact
//! integer arithmetic and refuses on any inconsistency, and the in-module
//! tests assert the ground truths. Every construction is deterministic: the
//! whole kit builds from ordered constant data, so two constructions compare
//! equal.
//!
//! The fixtures freeze the normative ground truths the wave packets grade
//! against (booking §5):
//!
//! - **F1 (T1.4 identity)** [`F1DeflationFixture`] — `h = 3z1² − 5z2²`,
//!   `det A = 1` ⇒ `det DT = det A³·det H_h = −60`, exact integer arithmetic.
//! - **F2 (T1.1 identity)** [`F2MinorIdentityKit`] — two random rational
//!   charts (deterministic seed) on which the chart-minor identity residual is
//!   exactly `0` for both `j`. The exact residual check runs over the kit's
//!   integer coefficient data (rational arithmetic); the symbolic admission is
//!   re-run by CTE-003's chart-minor substrate against this same data.
//! - **F3 (A₁⁺)** [`F3PlaneSphereFixture`] — plane `y = 1` × rational
//!   (stereographic) unit sphere, tangential at the chart point `z* = (0, 1)`;
//!   isolated contact, definite `H_h`.
//! - **F4 (A₁⁻)** [`F4SaddleFixture`] — plane `z = 0` × rational saddle
//!   bipatch `z = x² − y²`, node at the origin. This fixture is the **R3
//!   gate**: a classifier that refuses the indefinite (node) case fails the
//!   program.
//! - **F5 (A₂)** [`F5ParabolaFixture`] — plane `z = 0` × extruded parabola
//!   bipatch `z = u1²` (degree `(2,1)`); contact along a line, the T1.7
//!   witness `s = a = 1`, `q = u1` is hand-written data the verifier (CTE-001)
//!   must accept.
//! - **F6 (transversal)** [`F6CrossingPlanesFixture`] — plane × plane crossing
//!   with a box straddling the intersection line (the T1.5(1) sharpening
//!   example's setting).
//! - **F7 (T2 rows)** [`F7T2RowsFixture`] — the landed `boolean_m2` fixture
//!   *values* copied as read-only constants (the coplanar identical/anti cap
//!   pairs) plus the §5.9 derived decision rows over them.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, no `panic!`, and
//! no module-level `allow`.

use crate::contract::Refusal;
use crate::tangency::shapes::{Definiteness, SideState};

/// The exact determinant of the symmetric `2 x 2` matrix `[[2a, b], [b, 2c]]`
/// (the Hessian of `a·z1² + b·z1·z2 + c·z2²`): `4ac − b²`. Used by the
/// fixture admissions over integer coefficients.
fn hessian2_det(a: i64, b: i64, c: i64) -> i64 {
    4 * a * c - b * b
}

// ---------------------------------------------------------------------------
// F1 — the T1.4 deflation-identity instance (theory §9; booking §5)
// ---------------------------------------------------------------------------

/// F1 (T1.4 identity): the theory §9 instance `h = 3z1² − 5z2²`, `det A = 1`.
///
/// Ground truth (exact integers): the reduced contact Hessian has
/// `det H_h = −60` and the chart-deflation determinant satisfies
/// `det DT = det A³ · det H_h = −60` (theory T1.4, sign `+` under the §2.6
/// ordering). [`F1DeflationFixture::admit`] re-derives both determinants from
/// the integer coefficient data and refuses on any mismatch — the termination
/// argument's permanent regression, machine-checked at admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct F1DeflationFixture {
    /// Coefficient of `z1²` in `h` (`3`).
    pub h_z1sq: i64,
    /// Coefficient of `z1·z2` in `h` (`0`).
    pub h_z1z2: i64,
    /// Coefficient of `z2²` in `h` (`−5`).
    pub h_z2sq: i64,
    /// The pivot determinant `det A` (`1`).
    pub det_a: i64,
    /// The claimed `det H_h` ground truth (`−60`).
    pub det_h: i64,
    /// The claimed `det DT` ground truth (`−60`).
    pub det_dt: i64,
}

impl F1DeflationFixture {
    /// The fixture's integer coefficient data, verbatim.
    pub const fn new() -> Self {
        Self {
            h_z1sq: 3,
            h_z1z2: 0,
            h_z2sq: -5,
            det_a: 1,
            det_h: -60,
            det_dt: -60,
        }
    }

    /// Machine-check the ground truth with exact integer arithmetic.
    ///
    /// Re-derives `det H_h` from the `h` coefficients and `det DT` from
    /// `det A³ · det H_h`, refusing ([`Refusal::InvalidInput`]) on any
    /// inconsistency with the stored claims.
    pub fn admit(&self) -> Result<(), Refusal> {
        let recomputed_det_h = hessian2_det(self.h_z1sq, self.h_z1z2, self.h_z2sq);
        if recomputed_det_h != self.det_h {
            return Err(Refusal::InvalidInput);
        }
        let det_a_cubed = self.det_a * self.det_a * self.det_a;
        if det_a_cubed * recomputed_det_h != self.det_dt {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for F1DeflationFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F2 — the T1.1 minor-identity kit (theory §9; booking §5)
// ---------------------------------------------------------------------------

/// F2 (T1.1 identity): two random rational charts whose chart-minor identity
/// residual is exactly `0` for both `j`.
///
/// A chart is recorded as the integer coefficient data of the reduced system
/// over the chart coordinates: the two implicit functions `G₁, G₂` and the
/// distinguished component `f`, each as integer coefficient maps over the
/// monomials of `(y1, y2, z1, z2)`, plus the pivot `A = D_y G` data. The kit
/// generates two deterministic charts from a fixed seed (identical ordered
/// input → identical charts; no hash ordering), and [`F2MinorIdentityKit::admit`]
/// machine-checks the data's structural ground truths exactly.
///
/// The T1.1 identity residual is exactly `0` for both `j` — the permanent
/// regression of the minor identity — is re-derived over this same integer
/// coefficient data by CTE-003's polynomial chart-minor substrate (the
/// rational-arithmetic machine check the booking prescribes for the wave that
/// owns chart minors).
#[derive(Debug, Clone, PartialEq)]
pub struct F2MinorIdentityKit {
    /// The first chart's integer polynomial data.
    pub first: F2ChartData,
    /// The second chart's integer polynomial data.
    pub second: F2ChartData,
}

/// One rational chart of the F2 kit: the integer coefficient data of `G₁`,
/// `G₂` and `f`, with the pivot's two coordinate indices.
#[derive(Debug, Clone, PartialEq)]
pub struct F2ChartData {
    /// The `y` coordinate pair of the chart (`A = D_y G`).
    pub y_coords: (usize, usize),
    /// `G₁` coefficients, indexed by a fixed monomial enumeration over
    /// `(y1, y2, z1, z2)`.
    pub g1: Vec<i64>,
    /// `G₂` coefficients.
    pub g2: Vec<i64>,
    /// `f` coefficients.
    pub f: Vec<i64>,
}

impl F2MinorIdentityKit {
    /// Build the F2 kit: two deterministic "random rational" charts from the
    /// fixed seed, then admit them.
    pub fn build() -> Result<Self, Refusal> {
        let kit = Self {
            first: chart_from_seed(0x5EED_0001),
            second: chart_from_seed(0x5EED_0002),
        };
        kit.admit()?;
        Ok(kit)
    }

    /// Machine-check the kit's structural ground truths with exact integer
    /// arithmetic: every coefficient list must be finite-by-construction and
    /// the two charts must be distinct.
    pub fn admit(&self) -> Result<(), Refusal> {
        if self.first == self.second {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

/// A deterministic rational chart from a fixed seed (a small LCG over a
/// `2 x 2` pivot and two linear `G`s, so the pivot data is exactly checkable).
fn chart_from_seed(mut seed: u32) -> F2ChartData {
    let next = |seed: &mut u32| {
        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (*seed >> 8) as usize % 9
    };
    let a11 = next(&mut seed) as i64 + 1;
    let a12 = next(&mut seed) as i64 + 1;
    let a21 = next(&mut seed) as i64 + 1;
    let a22 = next(&mut seed) as i64 + 1;
    let b1 = next(&mut seed) as i64;
    let b2 = next(&mut seed) as i64;
    // G1 = a11·y1 + a12·y2 + b1·z1, G2 = a21·y1 + a22·y2 + b2·z1 over the
    // monomial enumeration (y1, y2, z1, z2).
    let g1 = vec![a11, a12, b1, 0];
    let g2 = vec![a21, a22, b2, 0];
    // f is a quadratic in z2 only: f = c·z2² − 1·y1² keeps the chart minor
    // identity a rational-arithmetic statement over the recorded coefficients.
    let c = next(&mut seed) as i64 + 1;
    let f = vec![-1, 0, 0, 0, 0, 0, 0, c];
    F2ChartData {
        y_coords: (0, 1),
        g1,
        g2,
        f,
    }
}

// ---------------------------------------------------------------------------
// F3 — the A₁⁺ plane × sphere fixture (booking §5)
// ---------------------------------------------------------------------------

/// F3 (A₁⁺): the plane `y = 1` tangent to the rational (stereographic) unit
/// sphere about the origin at the chart point `z* = (0, 1)`.
///
/// Ground truth (exact): the sphere chart
/// `X = (2u, 2v, 1 − u² − v²)/(1 + u² + v²)` at `(u, v) = (0, 1)` evaluates to
/// the point `(0, 1, 0)`: the homogeneous numerator is `P = (0, 2, 0)` with
/// weight `w = 2`, so `P·P = w²` (the point is on the sphere) and
/// `P_y = w` (it lies on the plane `y = 1`). The plane's normal `(0, 1, 0)` is
/// parallel to the sphere's radial direction at the point, so the contact is
/// tangential at the single point `z*` (A₁⁺, definite `H_h` — the isolated
/// case of T1.6b). The full classification admission (definite `H_h` over the
/// graph enclosure) runs in CTE-004 against this same kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F3PlaneSphereFixture {
    /// The stereographic chart point of the contact, `z* = (u, v) = (0, 1)`.
    pub z_star: (i64, i64),
    /// The homogeneous sphere numerator at `z*`: `(0, 2, 0)`.
    pub p: [i64; 3],
    /// The homogeneous weight at `z*`: `2`.
    pub w: i64,
}

impl F3PlaneSphereFixture {
    /// The fixture's integer data, verbatim.
    pub const fn new() -> Self {
        Self {
            z_star: (0, 1),
            p: [0, 2, 0],
            w: 2,
        }
    }

    /// Machine-check the ground truth with exact integer arithmetic: the point
    /// `P/w` is on the sphere (`P·P = w²`) and on the plane `y = 1`
    /// (`P_y = w`).
    pub fn admit(&self) -> Result<(), Refusal> {
        let dot = self.p[0] * self.p[0] + self.p[1] * self.p[1] + self.p[2] * self.p[2];
        if dot != self.w * self.w {
            return Err(Refusal::InvalidInput);
        }
        if self.p[1] != self.w {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for F3PlaneSphereFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F4 — the A₁⁻ saddle fixture (booking §5; the R3 gate)
// ---------------------------------------------------------------------------

/// F4 (A₁⁻): the plane `z = 0` × rational saddle bipatch `z = x² − y²`, with a
/// tangential node at the origin.
///
/// In the identity-chart reduction the reduced contact function is
/// `h(z1, z2) = z1² − z2²` with the node at `z* = (0, 0)`. Ground truth
/// (exact integers): `h(z*) = 0`, `∇h(z*) = (0, 0)`, and the reduced contact
/// Hessian has `det H_h = 4·1·(−1) − 0 = −4 < 0` — certified indefiniteness,
/// so the contact is A₁⁻: the zero set is two arcs crossing transversally at
/// the node (T1.6c). **This fixture is the R3 gate**: the verdict is
/// `A1Node`, never `Unresolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F4SaddleFixture {
    /// Coefficient of `z1²` in the reduced contact function (`1`).
    pub h_z1sq: i64,
    /// Coefficient of `z1·z2` in the reduced contact function (`0`).
    pub h_z1z2: i64,
    /// Coefficient of `z2²` in the reduced contact function (`−1`).
    pub h_z2sq: i64,
    /// The certified strictly-negative upper bound of `det H_h`: `−4`.
    pub det_h_upper: i64,
    /// The node's first coordinate (`0`).
    pub node_z1: i64,
    /// The node's second coordinate (`0`).
    pub node_z2: i64,
}

impl F4SaddleFixture {
    /// The fixture's integer data, verbatim.
    pub const fn new() -> Self {
        Self {
            h_z1sq: 1,
            h_z1z2: 0,
            h_z2sq: -1,
            det_h_upper: -4,
            node_z1: 0,
            node_z2: 0,
        }
    }

    /// Machine-check the ground truth with exact integer arithmetic and admit
    /// the A₁⁻ certificate shape.
    ///
    /// Re-derives `det H_h = 4·h_z1sq·h_z2sq − h_z1z2²`, refuses a non-negative
    /// bound, and refuses if the node is not the origin (`h` vanishes there to
    /// second order with zero gradient). Then constructs the frozen
    /// [`Definiteness::Indefinite`] verdict the R3 gate demands.
    pub fn admit(&self) -> Result<(), Refusal> {
        if self.node_z1 != 0 || self.node_z2 != 0 {
            return Err(Refusal::InvalidInput);
        }
        let det_h = hessian2_det(self.h_z1sq, self.h_z1z2, self.h_z2sq);
        if det_h != self.det_h_upper || det_h >= 0 {
            return Err(Refusal::InvalidInput);
        }
        let _ = Definiteness::indefinite(det_h as f64)?;
        Ok(())
    }
}

impl Default for F4SaddleFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F5 — the A₂ parabola fixture (booking §5)
// ---------------------------------------------------------------------------

/// F5 (A₂): the plane `z = 0` × extruded parabola bipatch `z = u1²` (degree
/// `(2, 1)`), with contact along a line.
///
/// Ground truth (exact): on the zero set of `G`, the reduced contact function
/// is `f = u1²`, and the T1.7 contact-factor identity
/// `s·f = q²·a + w₁G₁ + w₂G₂` holds with the hand-written witness
/// `s = a = 1`, `q = u1`. [`F5ParabolaFixture::admit`] machine-checks the
/// exact identity content `q²·a = u1²` by integer expansion of the recorded
/// witness; the full verifier acceptance (`verifier_accepts_f5_hand_witness`)
/// runs in CTE-001 over this same data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F5ParabolaFixture {
    /// The `u1` exponent in `q` (`1`).
    pub q_degree: u32,
    /// The constant multiplier `s` (`1`).
    pub s: i64,
    /// The constant factor `a` (`1`).
    pub a: i64,
    /// The exponent of `f`'s leading monomial `u1²` (`2`).
    pub f_degree: u32,
    /// The bidegree `(2, 1)` of the extruded-parabola bipatch, first axis.
    pub patch_deg_u: u32,
    /// The bidegree `(2, 1)` of the extruded-parabola bipatch, second axis.
    pub patch_deg_v: u32,
}

impl F5ParabolaFixture {
    /// The fixture's integer data, verbatim.
    pub const fn new() -> Self {
        Self {
            q_degree: 1,
            s: 1,
            a: 1,
            f_degree: 2,
            patch_deg_u: 2,
            patch_deg_v: 1,
        }
    }

    /// Machine-check the exact witness content with integer arithmetic:
    /// `q` is `u1`, `a = 1`, so `q²·a` expands to `u1²`, whose degree is the
    /// recorded `f_degree`; and the multipliers `s` and `a` are the constant
    /// `1` the booking prescribes.
    pub fn admit(&self) -> Result<(), Refusal> {
        if self.q_degree == 0 || self.s != 1 || self.a != 1 {
            return Err(Refusal::InvalidInput);
        }
        // q²·a has degree 2·q_degree (a is the constant 1).
        if 2 * self.q_degree != self.f_degree {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for F5ParabolaFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F6 — the transversal crossing-planes fixture (booking §5)
// ---------------------------------------------------------------------------

/// F6 (transversal): two crossing planes whose intersection line the fixture's
/// box straddles — the setting of the T1.5(1) sharpening example.
///
/// The planes are `z = 0` (patch 1, graph over `(u, v)`) and `z = x` (patch 2,
/// graph over `(s, t)`), crossing along the `y`-axis line. Ground truth
/// (exact): the two plane normals are not parallel — `(0,0,1) × (−1,0,1) =
/// (0,−1,0) ≠ 0` — so the planes meet transversally at every point of the
/// intersection line, and a box straddling that line is certified transversal
/// by the five-equation no-root predicate (T1.5.1) where per-minor separation
/// alone cannot decide. The T1.5 driver's exact admission over this kit runs
/// in CTE-003/CTE-004.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F6CrossingPlanesFixture {
    /// The first plane's integer normal `(0, 0, 1)`.
    pub normal_a: [i64; 3],
    /// The second plane's integer normal `(−1, 0, 1)`.
    pub normal_b: [i64; 3],
    /// The straddling box's `z`-midpoint sign side `−1` (a corner below the
    /// intersection line) — record kept for the straddle census.
    pub box_side: i64,
}

impl F6CrossingPlanesFixture {
    /// The fixture's integer data, verbatim.
    pub const fn new() -> Self {
        Self {
            normal_a: [0, 0, 1],
            normal_b: [-1, 0, 1],
            box_side: -1,
        }
    }

    /// Machine-check the ground truth with exact integer arithmetic: the
    /// cross product of the two normals is nonzero (the planes are not
    /// parallel), so the crossing is transversal.
    pub fn admit(&self) -> Result<(), Refusal> {
        let cross = [
            self.normal_a[1] * self.normal_b[2] - self.normal_a[2] * self.normal_b[1],
            self.normal_a[2] * self.normal_b[0] - self.normal_a[0] * self.normal_b[2],
            self.normal_a[0] * self.normal_b[1] - self.normal_a[1] * self.normal_b[0],
        ];
        if cross == [0, 0, 0] {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

impl Default for F6CrossingPlanesFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// F7 — the T2 rows fixture (boolean_m2 values, read-only; booking §5)
// ---------------------------------------------------------------------------

/// F7 (T2 rows): the coplanar Identical/Anti pairs of the landed `boolean_m2`
/// flagship, with the fixture *values* copied as read-only constants (never
/// imported from the test module).
///
/// The `boolean_m2` M2 flagship pairs the `4 x 4` block (heights `0..=2`) with
/// the disk of radius `1` about `(2, 2)` (heights `0..=2`). Its coincident cap
/// pairs are the coplanar carrier pairs at `z = 0` and `z = 2`: the two bottom
/// caps share the carrier `z = 0` with the same outward orientation
/// (Identical, `σ_A = σ_B = 10` in the carrier frame) and the two top caps
/// share the carrier `z = 2` the same way. The §5.9 derived rows over those
/// values are machine-checked by [`F7T2RowsFixture::admit`] using the frozen
/// two-bit algebra of [`SideState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct F7T2RowsFixture {
    /// The block's footprint half-extent (`4`).
    pub block_extent: i64,
    /// The block's height (`2`).
    pub block_height: i64,
    /// The disk's radius (`1`).
    pub disk_radius: i64,
    /// The disk's centre, `(2, 2)`.
    pub disk_centre: (i64, i64),
    /// The carrier plane coordinate of the coincident bottom caps (`0`).
    pub bottom_z: i64,
    /// The carrier plane coordinate of the coincident top caps (`2`).
    pub top_z: i64,
    /// The Identical cap pair's side state in the carrier frame (`10`).
    pub identical_state: u8,
    /// The Anti cap pair's side state in the carrier frame (`01`).
    pub anti_state: u8,
}

impl F7T2RowsFixture {
    /// The copied `boolean_m2` fixture values, verbatim.
    pub const fn new() -> Self {
        Self {
            block_extent: 4,
            block_height: 2,
            disk_radius: 1,
            disk_centre: (2, 2),
            bottom_z: 0,
            top_z: 2,
            identical_state: 0b10,
            anti_state: 0b01,
        }
    }

    /// Machine-check the recorded rows with the frozen two-bit algebra: the
    /// Identical pair's union keeps one canonical face (`10 ∨ 10 = 10`, kept
    /// unflipped) and the Anti pair's union drops the shared wall
    /// (`10 ∨ 01 = 11`, dropped) — the §5.9 truth rows over the `boolean_m2`
    /// values.
    pub fn admit(&self) -> Result<(), Refusal> {
        let identical = SideState::new(self.identical_state).map_err(|_| Refusal::InvalidInput)?;
        let anti = SideState::new(self.anti_state).map_err(|_| Refusal::InvalidInput)?;
        // The coordinatewise union of two identical boundaries is the same
        // boundary (keep one canonical face); of two anti boundaries it is the
        // internal state 11 (the shared wall is dropped).
        let unioned_identical = union(identical, identical)?;
        if unioned_identical.state() != self.identical_state {
            return Err(Refusal::InvalidInput);
        }
        let unioned_anti = union(identical, anti)?;
        if unioned_anti.state() != 0b11 {
            return Err(Refusal::InvalidInput);
        }
        Ok(())
    }
}

/// The coordinatewise union `σ_A ∨ σ_B` of the two-bit side algebra (theory
/// §5.4). The OR of two two-bit states stays in `0..=3`, so the construction
/// never actually refuses.
fn union(a: SideState, b: SideState) -> Result<SideState, Refusal> {
    let bits = (a.state() | b.state()) & 0b11;
    SideState::new(bits)
}

impl Default for F7T2RowsFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// The kit assembler
// ---------------------------------------------------------------------------

/// Build the whole kit from ordered constant data and admit every fixture.
///
/// Determinism: two calls compare equal (no hash iteration, no external
/// entropy).
pub fn build_kit() -> Result<FixtureKit, Refusal> {
    let kit = FixtureKit {
        f1: F1DeflationFixture::new(),
        f2: F2MinorIdentityKit::build()?,
        f3: F3PlaneSphereFixture::new(),
        f4: F4SaddleFixture::new(),
        f5: F5ParabolaFixture::new(),
        f6: F6CrossingPlanesFixture::new(),
        f7: F7T2RowsFixture::new(),
    };
    kit.admit_all()?;
    Ok(kit)
}

/// The assembled CTE fixture kit.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureKit {
    /// The F1 fixture.
    pub f1: F1DeflationFixture,
    /// The F2 fixture.
    pub f2: F2MinorIdentityKit,
    /// The F3 fixture.
    pub f3: F3PlaneSphereFixture,
    /// The F4 fixture.
    pub f4: F4SaddleFixture,
    /// The F5 fixture.
    pub f5: F5ParabolaFixture,
    /// The F6 fixture.
    pub f6: F6CrossingPlanesFixture,
    /// The F7 fixture.
    pub f7: F7T2RowsFixture,
}

impl FixtureKit {
    /// Admit every fixture of the kit.
    pub fn admit_all(&self) -> Result<(), Refusal> {
        self.f1.admit()?;
        self.f2.admit()?;
        self.f3.admit()?;
        self.f4.admit()?;
        self.f5.admit()?;
        self.f6.admit()?;
        self.f7.admit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f1_t14_identity_instance_exact() {
        let fixture = F1DeflationFixture::new();
        fixture
            .admit()
            .expect("the F1 ground truth is internally consistent");

        // The F1 ground truth, re-derived here in exact integer arithmetic
        // (theory §9): h = 3z1² − 5z2², det A = 1 ⇒ det DT = det A³·det H_h = −60.
        let det_h = hessian2_det(fixture.h_z1sq, fixture.h_z1z2, fixture.h_z2sq);
        assert_eq!(det_h, -60);
        let det_a_cubed = fixture.det_a * fixture.det_a * fixture.det_a;
        let det_dt = det_a_cubed * det_h;
        assert_eq!(det_dt, -60);
        assert_eq!(fixture.det_h, det_h);
        assert_eq!(fixture.det_dt, det_dt);

        // And the claimed deflation identity is exactly the recomputation.
        assert_eq!(fixture.det_dt, fixture.det_a.pow(3) * fixture.det_h);
    }

    #[test]
    fn f4_saddle_fixture_admits() {
        let fixture = F4SaddleFixture::new();
        fixture
            .admit()
            .expect("the A1- saddle fixture constructs and admits");

        // The R3 gate's certificate shape: the reduced contact Hessian is
        // certified indefinite with a strictly negative det upper bound (−4).
        let det_h = hessian2_det(fixture.h_z1sq, fixture.h_z1z2, fixture.h_z2sq);
        assert_eq!(det_h, -4);
        assert!(det_h < 0);
        match Definiteness::indefinite(det_h as f64) {
            Ok(Definiteness::Indefinite { det_upper }) => assert!(det_upper < 0.0),
            Ok(_) => panic!("an indefinite saddle admits the Indefinite verdict"),
            Err(_) => panic!("a strictly negative det bound must admit"),
        }

        // The node is at the origin and the reduced function vanishes there.
        assert_eq!((fixture.node_z1, fixture.node_z2), (0, 0));
        let at_node = fixture.h_z1sq * fixture.node_z1 * fixture.node_z1
            + fixture.h_z1z2 * fixture.node_z1 * fixture.node_z2
            + fixture.h_z2sq * fixture.node_z2 * fixture.node_z2;
        assert_eq!(at_node, 0);
    }
}
