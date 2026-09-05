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
// The fixed-size 3×3 / 4×4 arrays below are indexed by constants, fixed
// loop ranges, and iterator-derived indices that are in bounds by construction
// (never a geometry-derived index); the polar gradient sums are dense
// fixed-index reductions.
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::large_enum_variant
)] // fixed-array index math / closed-form reductions; see above

//! BIE-004-CLOSURE: the completeness layer of the Certified Interaction
//! Engine (BIE program, theory §4.2/§5.4/§13.3, spine §3–§8).
//!
//! BIE-002 certifies individual interaction branches of the restricted-pair
//! F-form `F(x) = X_A(u, v) − X_B(s, t)`; completeness needs the two layers
//! this module adds over the **landed** solver ([`ssi4`]): the **polar
//! exclusion** oracle that disposes of the interior-loop regions boundary
//! seeding cannot see, and the **escalation scheduler** (with the §5.4
//! certified slope diagnostic) that decides when subdivision/retry continues
//! and when it stops in a typed [`InteractionOutcome::Unresolved`].
//!
//! # Polar exclusion (Theorem E)
//!
//! Let `Z = F⁻¹(0) ∩ R` be the interaction set inside a product region `R`
//! (a `(u, v) × (s, t)` cell). Boundary seeding finds the components of `Z`
//! that meet `∂R`; the components it **cannot** see are the closed curves
//! lying strictly inside `R` (planted interior loops). Restrict the first
//! product coordinate `x_free` of a fixed free axis to such a closed loop:
//! the continuous function has an interior extremum on the loop, where the
//! loop's tangent is parallel to the level set `{x_free = const}` — the
//! tangent null direction `τ ∈ ker DF` has `τ_free = 0`, which is exactly
//! the vanishing of the 3×3 Jacobian minor that deletes column `free`
//! ([`polar_scalar`]). The **polar-augmented square system**
//!
//! ```text
//! P(x) = ( F(x),  det[ DF(x) without column free ] )
//! ```
//!
//! (4 equations in the 4 unknowns `x`, the same restricted F forms) therefore
//! has a root in `R` whenever an interior closed loop is present. Conversely,
//! a certified no-root verdict of the landed `krawczyk::<4>` operator over
//! `R` proves the region contains no interior closed loop — the exclusion.
//! This module instantiates the landed machinery ([`PolarSystem4`] implements
//! the landed [`KrawczykSystem`]); no homotopy or deflation stack is built.
//!
//! The polar gradient row `∇ det[minorfree]` is the derivative of a Jacobian
//! minor along the chart, so it needs the **second partials** of the two
//! carriers; those closed forms live on the F-form (`ssi4` exposes the
//! outward-rounded `second_columns_*`).
//!
//! # The no-loop property (Theorem B) and covers
//!
//! A cover of a region is a deterministic subdivision (axis order `u, v, s,
//! t`, low-before-high, always). On every cover in this module's tests the
//! oracle below is asserted: a region proven empty by exclusion contains no
//! certified branch, and every planted interior loop is found by the cover
//! (a fold-containing region is never proven empty).
//!
//! # The escalation scheduler (§13.3) and the slope diagnostic (§5.4)
//!
//! The face-tangency retry loop (a degenerate pair keeps refusing
//! certification under subdivision) must terminate or escalate, never spin.
//! [`should_escalate`] is the escalate-iff-predicted-cost decision: escalate
//! when the certified slope magnitude of the region is below the certified
//! transversality floor (the §5.4 slope diagnostic, [`slope_bound`]), when the
//! retry depth cap is reached, or when the predicted cost of refining the
//! region to the certified metric width exceeds the certified-progress budget.
//! [`solve_pair_region`] drives the landed solver under that scheduler and
//! returns a typed [`RegionVerdict`].
//!
//! **H-1.** This file carries no `unwrap`, no `expect`, no `panic!`, and no
//! out-of-range indexing reachable from geometry; every array index is a
//! constant or iterator-derived and in bounds by construction.
//!
//! **H-6.** No float-computed value is ever recorded as `Method::Exact`; the
//! certified statements here are the Krawczyk boxes and the slope bound
//! intervals.
//!
//! **Determinism.** Identical ordered input → identical verdicts: the polar
//! scalar and its gradient reduce fixed axis orders, the cover subdivision is
//! axis order then low-before-high, the slope bound resolves ties toward the
//! lowest free axis, and the scheduler refines toward the witness-cell
//! midpoint deterministically.

use crate::construct::bie::ssi4::{certify_restricted_pair, FForm, Ssi4Parameters};
use crate::construct::bie::{InteractionOutcome, WitnessCell};
use crate::formal::exact::CertifiedSign;
use truck_base::evidence::{Budget, Certified, Outcome, Refusal, UnresolvedWitness};
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};

/// A degenerate interval from a finite float. A non-finite input degrades to
/// the empty interval (a caller bug, never a panic).
fn iv(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// An interval from two ordered finite floats.
fn iv_lo_hi(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The 4-D parameter box of a product cell.
fn cell_box(cell: &WitnessCell) -> [Interval; 4] {
    [
        iv_lo_hi(cell.u.0, cell.u.1),
        iv_lo_hi(cell.v.0, cell.v.1),
        iv_lo_hi(cell.s.0, cell.s.1),
        iv_lo_hi(cell.t.0, cell.t.1),
    ]
}

/// The widest per-axis width of a product cell (the box diameter in the
/// axis-wise max metric).
fn cell_width(cell: &WitnessCell) -> f64 {
    let w = |a: (f64, f64)| a.1 - a.0;
    w(cell.u).max(w(cell.v)).max(w(cell.s)).max(w(cell.t))
}

/// The free-column index set: the three axes left after deleting `free`.
fn free_columns(free: usize) -> [usize; 3] {
    let mut out = [0usize; 3];
    let mut k = 0usize;
    for axis in 0..4 {
        if axis != free {
            out[k] = axis;
            k += 1;
        }
    }
    out
}

/// The 3×3 float minor of the F-form Jacobian columns after deleting column
/// `free`, in row-major order (`minor[r][c] = ∂F_r/∂x_{fc[c]}`).
fn minor3_f(cols: &[[f64; 3]; 4], fc: &[usize; 3]) -> [[f64; 3]; 3] {
    [
        [cols[fc[0]][0], cols[fc[1]][0], cols[fc[2]][0]],
        [cols[fc[0]][1], cols[fc[1]][1], cols[fc[2]][1]],
        [cols[fc[0]][2], cols[fc[1]][2], cols[fc[2]][2]],
    ]
}

/// The 3×3 interval minor of the interval F-form Jacobian columns after
/// deleting column `free` (row-major).
fn minor3_iv(cols: &[[Interval; 3]; 4], fc: &[usize; 3]) -> [[Interval; 3]; 3] {
    [
        [cols[fc[0]][0], cols[fc[1]][0], cols[fc[2]][0]],
        [cols[fc[0]][1], cols[fc[1]][1], cols[fc[2]][1]],
        [cols[fc[0]][2], cols[fc[1]][2], cols[fc[2]][2]],
    ]
}

/// The float determinant of a 3×3 matrix.
fn det3_f(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The interval determinant of a 3×3 matrix (outward-rounded).
fn det3_iv(m: &[[Interval; 3]; 3]) -> Interval {
    let a = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]);
    let b = m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);
    let c = m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    a - b + c
}

/// The `(r, k)` cofactor of a 3×3 float matrix: `(−1)^(r+k)` times the
/// determinant of the 2×2 submatrix that deletes row `r` and column `k`.
fn cofactor_f(m: &[[f64; 3]; 3], r: usize, k: usize) -> f64 {
    let (r0, r1) = other_rows(r);
    let (k0, k1) = other_rows(k);
    let d = m[r0][k0] * m[r1][k1] - m[r0][k1] * m[r1][k0];
    if (r + k).is_multiple_of(2) {
        d
    } else {
        -d
    }
}

/// The `(r, k)` cofactor of a 3×3 interval matrix.
fn cofactor_iv(m: &[[Interval; 3]; 3], r: usize, k: usize) -> Interval {
    let (r0, r1) = other_rows(r);
    let (k0, k1) = other_rows(k);
    let d = m[r0][k0] * m[r1][k1] - m[r0][k1] * m[r1][k0];
    if (r + k).is_multiple_of(2) {
        d
    } else {
        -d
    }
}

/// The two indices of `{0, 1, 2}` other than `x`, in ascending order.
fn other_rows(x: usize) -> (usize, usize) {
    match x {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// The polar scalar of the free axis at a point: the determinant of the 3×3
/// F-form Jacobian minor that deletes column `free`. It vanishes exactly where
/// the interaction tangent is parallel to the level set `{x_free = const}`.
fn polar_scalar_f(cols: &[[f64; 3]; 4], free: usize) -> f64 {
    let fc = free_columns(free);
    let minor = minor3_f(cols, &fc);
    det3_f(&minor)
}

/// The float gradient of the polar scalar along the chart:
/// `∂φ/∂x_j = Σ_{r,k} cof_{r,k} · ∂²F_r/(∂x_{fc[k]} ∂x_j)` with `cof` the
/// cofactor of the deleted-column minor. This is the fourth row of the polar
/// system's Jacobian (a derivative of a Jacobian minor, hence the second
/// partial columns).
fn polar_grad_f(cols: &[[f64; 3]; 4], second: &[[[f64; 3]; 4]; 4], free: usize) -> [f64; 4] {
    let fc = free_columns(free);
    let minor = minor3_f(cols, &fc);
    let mut grad = [0.0f64; 4];
    for r in 0..3 {
        for k in 0..3 {
            let cof = cofactor_f(&minor, r, k);
            let a = fc[k];
            for j in 0..4 {
                grad[j] += cof * second[a][j][r];
            }
        }
    }
    grad
}

/// The interval gradient of the polar scalar over a box (outward-rounded),
/// from the interval Jacobian columns and the interval second partial
/// columns.
fn polar_grad_iv(
    cols: &[[Interval; 3]; 4],
    second: &[[[Interval; 3]; 4]; 4],
    free: usize,
) -> [Interval; 4] {
    let fc = free_columns(free);
    let minor = minor3_iv(cols, &fc);
    let mut grad = [iv(0.0); 4];
    for r in 0..3 {
        for k in 0..3 {
            let cof = cofactor_iv(&minor, r, k);
            let a = fc[k];
            for j in 0..4 {
                grad[j] += cof * second[a][j][r];
            }
        }
    }
    grad
}

/// The float inverse of a 4×4 matrix by Gauss–Jordan with partial pivoting.
/// `None` on a (near-)singular matrix.
fn invert4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut a = *m;
    let mut inv = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    for col in 0..4 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..4 {
            let cand = a[r][col].abs();
            if cand > best {
                best = cand;
                pivot = r;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            for c in 0..4 {
                let t = a[col][c];
                a[col][c] = a[pivot][c];
                a[pivot][c] = t;
                let t = inv[col][c];
                inv[col][c] = inv[pivot][c];
                inv[pivot][c] = t;
            }
        }
        let d = a[col][col];
        for c in 0..4 {
            a[col][c] /= d;
            inv[col][c] /= d;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let f = a[r][col];
            if f == 0.0 {
                continue;
            }
            for c in 0..4 {
                a[r][c] -= f * a[col][c];
                inv[r][c] -= f * inv[col][c];
            }
        }
    }
    Some(inv)
}

// ---------------------------------------------------------------------------
// The polar-augmented square system (Theorem E)
// ---------------------------------------------------------------------------

/// The N=4 polar-augmented square system of the restricted-pair F-form:
/// `P(x) = (F(x), φ_free(x))` with `φ_free` the deleted-column Jacobian minor
/// (the polar scalar). An interior closed loop of the interaction inside a
/// region forces a root of this system, so a certified no-root verdict is the
/// exclusion; a certified unique verdict locates a loop fold. Implements the
/// landed [`KrawczykSystem`] — the operator is instantiated, never extended.
#[derive(Clone, Debug)]
pub struct PolarSystem4 {
    /// The restricted-pair F-form.
    pub form: FForm,
    /// The free product axis (`0..4`) whose polar scalar is the fourth
    /// equation.
    pub free: usize,
}

impl KrawczykSystem<4> for PolarSystem4 {
    fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
        let f = self.form.residual_f(x);
        let cols = self.form.partial_columns_f(x);
        let phi = polar_scalar_f(&cols, self.free);
        [iv(f[0]), iv(f[1]), iv(f[2]), iv(phi)]
    }

    fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
        let cols = self.form.partial_columns_iv(b);
        let second = self.form.second_columns_iv(b);
        let grad = polar_grad_iv(&cols, &second, self.free);
        let mut out = [[Interval::EMPTY; 4]; 4];
        for r in 0..3 {
            for c in 0..4 {
                out[r][c] = cols[c][r];
            }
        }
        out[3] = grad;
        out
    }

    fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
        let cols = self.form.partial_columns_f(x);
        let second = self.form.second_columns_f(x);
        let grad = polar_grad_f(&cols, &second, self.free);
        let mut m = [[0.0f64; 4]; 4];
        for r in 0..3 {
            for c in 0..4 {
                m[r][c] = cols[c][r];
            }
        }
        m[3] = grad;
        invert4(&m)
    }
}

/// What the polar-exclusion operator proved about a region.
#[derive(Clone, Debug, PartialEq)]
pub enum ExclusionVerdict {
    /// A certified no-root verdict: the region contains no closed interaction
    /// loop the boundary seeding could miss (Theorem E exclusion).
    Excluded,
    /// A certified unique root of the polar-augmented system: a loop fold is
    /// located in the region.
    LoopRoot,
}

/// The polar-exclusion oracle over one region (Theorem E): run the landed
/// `krawczyk::<4>` operator on the polar-augmented square system over the
/// region's 4-D box. `NoRoot` proves the exclusion; `Unique` locates a loop
/// fold; the operator's typed refusal propagates unchanged (H-2). The free
/// axis is fixed by the caller (deterministic; any single axis excludes, the
/// loop gives folds for every axis).
pub fn polar_exclusion(
    form: &FForm,
    region: &WitnessCell,
    free: usize,
    budget: &mut Budget,
) -> Outcome<ExclusionVerdict> {
    let start = cell_box(region);
    let system = PolarSystem4 {
        form: form.clone(),
        free,
    };
    match krawczyk::<4>(&system, &start, budget) {
        Ok(Certified {
            value: KrawczykProof::Unique,
            cert,
        }) => Ok(Certified::new(ExclusionVerdict::LoopRoot, cert)),
        Ok(Certified {
            value: KrawczykProof::NoRoot,
            cert,
        }) => Ok(Certified::new(ExclusionVerdict::Excluded, cert)),
        Err(refusal) => Err(refusal),
    }
}

// ---------------------------------------------------------------------------
// Covers (determinism: axis order, low-before-high)
// ---------------------------------------------------------------------------

/// The deterministic cover of a region: subdivide axis `u`, then `v`, then
/// `s`, then `t` into `parts[i]` equal pieces, low-before-high within every
/// axis, and enumerate the product cells lexicographically by ascending axis
/// index. A part count of 0 or 1 leaves that axis whole. Identical ordered
/// input yields identical output order.
pub fn cover_of(cell: &WitnessCell, parts: [usize; 4]) -> Vec<WitnessCell> {
    let mut out: Vec<WitnessCell> = Vec::new();
    let n = [
        parts[0].max(1),
        parts[1].max(1),
        parts[2].max(1),
        parts[3].max(1),
    ];
    let axis = [
        (cell.u.0, cell.u.1),
        (cell.v.0, cell.v.1),
        (cell.s.0, cell.s.1),
        (cell.t.0, cell.t.1),
    ];
    for i0 in 0..n[0] {
        for i1 in 0..n[1] {
            for i2 in 0..n[2] {
                for i3 in 0..n[3] {
                    let idx = [i0, i1, i2, i3];
                    let mut lo = [0.0f64; 4];
                    let mut hi = [0.0f64; 4];
                    for k in 0..4 {
                        let width = axis[k].1 - axis[k].0;
                        let step = width / (n[k] as f64);
                        lo[k] = axis[k].0 + step * (idx[k] as f64);
                        hi[k] = axis[k].0 + step * ((idx[k] + 1) as f64);
                    }
                    out.push(WitnessCell::new(
                        (lo[0], hi[0]),
                        (lo[1], hi[1]),
                        (lo[2], hi[2]),
                        (lo[3], hi[3]),
                    ));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The §5.4 certified slope diagnostic
// ---------------------------------------------------------------------------

/// A certified bound on the §5.4 slope of a region: the dimensionless ratio
/// `det(DF minor) / (column-norm product)` over the region's box, enclosed in
/// interval arithmetic (Hadamard: the true ratio always lies in `[-1, 1]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CertifiedSlope {
    /// The outward-rounded lower slope bound over the region.
    pub lo: f64,
    /// The outward-rounded upper slope bound over the region.
    pub hi: f64,
    /// The certified sign of the slope (away from zero), `None` when the
    /// slope interval straddles zero (the region is degenerate).
    pub sign: Option<CertifiedSign>,
    /// The certified lower bound of `|slope|` over the region: positive only
    /// when [`Self::sign`] certifies a sign, else `0.0`.
    pub mag: f64,
}

impl CertifiedSlope {
    /// The signed witness value that feeds the frozen κ/cell/slope witness:
    /// the certified magnitude with the certified sign, `0.0` when the sign is
    /// not certified (never a guess — the field is a diagnostic).
    pub fn witness_value(&self) -> f64 {
        match self.sign {
            Some(CertifiedSign::Positive) => self.mag,
            Some(CertifiedSign::Negative) => -self.mag,
            _ => 0.0,
        }
    }
}

/// The interval 2-norm of one interval 3-vector.
fn norm3_iv(v: &[Interval; 3]) -> Interval {
    let sum = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    sum.sqrt()
}

/// The §5.4 certified slope bound over a region: enclose the deleted-column
/// minor determinant and the product of its column norms over the region's
/// box, choose the free axis deterministically (the axis whose slope interval
/// certifies the largest magnitude away from zero, ties toward the lowest
/// index), and return the certified slope interval with its sign. A region
/// whose every minor is degenerate (e.g. a face tangency) returns a sign-less
/// bound with `mag == 0`.
pub fn slope_bound(form: &FForm, region: &WitnessCell) -> CertifiedSlope {
    let box4 = cell_box(region);
    let cols = form.partial_columns_iv(&box4);
    let mut best: Option<(f64, Interval)> = None;
    for free in 0..4 {
        let fc = free_columns(free);
        let minor = minor3_iv(&cols, &fc);
        let det = det3_iv(&minor);
        if !det.inf().is_finite() || !det.sup().is_finite() {
            continue;
        }
        let n0 = norm3_iv(&[minor[0][0], minor[1][0], minor[2][0]]);
        let n1 = norm3_iv(&[minor[0][1], minor[1][1], minor[2][1]]);
        let n2 = norm3_iv(&[minor[0][2], minor[1][2], minor[2][2]]);
        if !n0.inf().is_finite() || !n1.inf().is_finite() || !n2.inf().is_finite() {
            continue;
        }
        let nprod = n0 * n1 * n2;
        // Hadamard: the true slope always lies in [-1, 1], so that is the
        // sound fallback when the column-scale product can vanish.
        let unit = iv_lo_hi(-1.0, 1.0);
        let slope = if nprod.inf() > 0.0 { det / nprod } else { unit };
        let slope = slope.intersection(unit);
        let mag = if slope.inf() > 0.0 {
            slope.inf()
        } else if slope.sup() < 0.0 {
            -slope.sup()
        } else {
            0.0
        };
        let better = match best {
            None => true,
            Some((best_mag, _)) => mag > best_mag,
        };
        if better {
            best = Some((mag, slope));
        }
    }
    match best {
        Some((mag, slope)) => {
            let sign = if slope.inf() > 0.0 {
                Some(CertifiedSign::Positive)
            } else if slope.sup() < 0.0 {
                Some(CertifiedSign::Negative)
            } else {
                None
            };
            CertifiedSlope {
                lo: slope.inf(),
                hi: slope.sup(),
                sign,
                mag,
            }
        }
        None => CertifiedSlope {
            lo: -1.0,
            hi: 1.0,
            sign: None,
            mag: 0.0,
        },
    }
}

/// Builds a typed [`InteractionOutcome::Unresolved`] witness over a cell whose
/// §5.4 slope diagnostic is `bound` and whose conditioning witness `kappa`
/// follows the landed solver's convention (the reciprocal of the certified
/// best minor magnitude; `1.0e12` when nothing certifies). This is the hook
/// that feeds the certified slope bound into the frozen κ/cell/slope witness.
pub fn unresolved_with_slope(
    bound: &CertifiedSlope,
    kappa: f64,
    cell: WitnessCell,
) -> InteractionOutcome {
    InteractionOutcome::Unresolved {
        kappa,
        cell,
        slope: bound.witness_value(),
    }
}

// ---------------------------------------------------------------------------
// The escalation scheduler (§13.3) and the region driver
// ---------------------------------------------------------------------------

/// The escalate-iff-predicted-cost scheduler policy. The constants are the
/// BIE-004 derivation (RESULT notes): the certified transversality floor, the
/// retry depth cap, and the certified-progress floor (the certified box-width
/// halvings one retry round must deliver for subdivision to continue).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EscalationPolicy {
    /// The certified transversality floor: a region whose certified slope
    /// magnitude is below this value escalates (the face-tangency regime, in
    /// which no transverse branch can certify).
    pub slope_floor: f64,
    /// The retry depth cap of the face-tangency retry loop.
    pub max_retry_depth: u32,
    /// The certified-progress floor: the certified box-width halvings one
    /// retry round must deliver for the next subdivision to be justified.
    pub progress_floor: f64,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        EscalationPolicy {
            // Derived (RESULT notes): a dimensionless slope below 1e-3 means
            // the best certified 3×3 minor is at most a thousandth of its
            // column-scale product over the region — the Krawczyk operator
            // cannot certify a transverse root there, so subdivision only
            // refines a degenerate configuration.
            slope_floor: 1.0e-3,
            // Derived: each retry level refines toward the unresolved cell;
            // beyond this depth escalation is cheaper than the refinement
            // fan-out of the solver's own metric boxes.
            max_retry_depth: 2,
            // Derived: a retry round that does not certify a certified box at
            // least half as wide as the region is below certified progress —
            // the operator's own metric box is the width a healthy round
            // certifies, so one halving per round is the floor.
            progress_floor: 1.0,
        }
    }
}

/// Why the scheduler stopped subdivision/retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscalationReason {
    /// The certified slope magnitude fell below the transversality floor.
    SlopeFloor,
    /// The retry depth cap was reached.
    DepthCap,
    /// The predicted cost of the remaining refinement exceeds the budget the
    /// certified progress rate supports.
    PredictedCost,
}

/// The scheduler's decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escalation {
    /// Subdivide/retry once more.
    Continue,
    /// Stop in a typed unresolved (escalation).
    Escalate(EscalationReason),
}

/// The escalate-iff-predicted-cost decision (§13.3). Escalation fires when any
/// of the three stops triggers:
///
/// 1. **Slope floor.** `slope_mag < slope_floor`: the region's certified
///    slope cannot support a transverse branch.
/// 2. **Depth cap.** `depth >= max_retry_depth`.
/// 3. **Predicted cost.** The certified-progress rate of the last retry is
///    `log2(width / certified_width)` width halvings per round (zero when the
///    attempt certified no narrower box); the predicted cost of reaching the
///    certified metric width `target_width` is `ceil(log2(width /
///    target_width))` targeted refinement attempts. Escalate when a retried
///    round (`depth >= 1`) delivered fewer halvings than the
///    `progress_floor`, or when the predicted attempt count exceeds the
///    `subdiv_left` budget one subdivision can pay for.
pub fn should_escalate(
    policy: &EscalationPolicy,
    depth: u32,
    width: f64,
    target_width: f64,
    slope_mag: f64,
    certified_width: f64,
    subdiv_left: u32,
) -> Escalation {
    if slope_mag < policy.slope_floor {
        return Escalation::Escalate(EscalationReason::SlopeFloor);
    }
    if depth >= policy.max_retry_depth {
        return Escalation::Escalate(EscalationReason::DepthCap);
    }
    let predicted = if width <= target_width {
        1u32
    } else {
        (width / target_width).log2().ceil().max(1.0) as u32
    };
    if predicted > subdiv_left {
        return Escalation::Escalate(EscalationReason::PredictedCost);
    }
    if depth >= 1 {
        let certified = if certified_width >= width || certified_width <= 0.0 {
            0.0
        } else {
            (width / certified_width).log2()
        };
        if certified < policy.progress_floor {
            return Escalation::Escalate(EscalationReason::PredictedCost);
        }
    }
    Escalation::Continue
}

/// The typed outcome of the region solver under the escalation scheduler.
#[derive(Clone, Debug)]
pub enum RegionVerdict {
    /// The solver certified branch samples over the region.
    Certified(crate::construct::bie::ssi4::CertifiedChartCurve),
    /// The retry loop stopped: a typed unresolved witness (never a guess) with
    /// the budget spent by the retries.
    Escalated {
        /// The typed unresolved witness (κ/cell/slope).
        outcome: InteractionOutcome,
        /// What the retries spent before escalating.
        spent: Budget,
    },
}

/// Whether the whole region should still be retried: subdivide the widest
/// axis of `region` into its two halves and return the half containing `p`
/// (deterministic: an interior point on the split goes to the low half).
fn refine_toward(region: &WitnessCell, p: [f64; 4]) -> Option<WitnessCell> {
    let axis = [
        (region.u.0, region.u.1),
        (region.v.0, region.v.1),
        (region.s.0, region.s.1),
        (region.t.0, region.t.1),
    ];
    let mut widest = 0usize;
    let mut best = f64::NEG_INFINITY;
    for (k, &(lo, hi)) in axis.iter().enumerate() {
        if hi - lo > best {
            best = hi - lo;
            widest = k;
        }
    }
    let (lo, hi) = axis[widest];
    let mid = 0.5 * lo + 0.5 * hi;
    let low = p[widest] <= mid;
    let mut lo2 = axis;
    let mut hi2 = axis;
    if low {
        lo2[widest] = (lo, mid);
    } else {
        hi2[widest] = (mid, hi);
    }
    let out = if low { lo2 } else { hi2 };
    Some(WitnessCell::new(out[0], out[1], out[2], out[3]))
}

/// The cell of the unresolved witness (the whole region when no witness
/// arrived).
fn witness_cell_of(outcome: &InteractionOutcome, fallback: &WitnessCell) -> WitnessCell {
    match outcome {
        InteractionOutcome::Unresolved { cell, .. } => *cell,
        _ => *fallback,
    }
}

/// The chart midpoint of a cell.
fn mid_of(cell: &WitnessCell) -> [f64; 4] {
    [
        0.5 * (cell.u.0 + cell.u.1),
        0.5 * (cell.v.0 + cell.v.1),
        0.5 * (cell.s.0 + cell.s.1),
        0.5 * (cell.t.0 + cell.t.1),
    ]
}

/// The face-tangency retry driver (§13.3): run the landed restricted-pair
/// solver over the region under the escalation scheduler. A certified branch
/// returns [`RegionVerdict::Certified`]; a region the solver cannot certify is
/// either retried once toward the unresolved cell (charging one subdivision
/// per retry round from `budget`) or escalated to a typed
/// [`InteractionOutcome::Unresolved`] — the loop always terminates or
/// escalates, never spins, and never exhausts the budget by retry alone.
pub fn solve_pair_region(
    a: crate::construct::bie::ssi4::RestrictedChart,
    b: crate::construct::bie::ssi4::RestrictedChart,
    region: WitnessCell,
    params: &Ssi4Parameters,
    policy: &EscalationPolicy,
    budget: &mut Budget,
) -> Outcome<RegionVerdict> {
    let form = FForm { a, b };
    let initial = *budget;
    let mut depth = 0u32;
    let mut current = region;
    loop {
        if budget.subdiv == 0 {
            return Err(Refusal::NumericallyUnresolved {
                spent: spent(initial, budget),
                witness: UnresolvedWitness::KrawczykIndeterminate,
            });
        }
        budget
            .spend_subdiv(1)
            .map_err(|_| Refusal::NumericallyUnresolved {
                spent: spent(initial, budget),
                witness: UnresolvedWitness::KrawczykIndeterminate,
            })?;
        let curve = certify_restricted_pair(
            form.a.clone(),
            form.b.clone(),
            current,
            params,
            &mut Budget::new(0, 0, 0),
        )?
        .value;
        if !curve.samples.is_empty() {
            return Ok(Certified::new(
                RegionVerdict::Certified(curve),
                interval_certificate(),
            ));
        }
        let witness = match curve.witness {
            Some(outcome) => outcome,
            None => {
                let bound = slope_bound(&form, &current);
                let kappa = if bound.mag > 0.0 {
                    1.0 / bound.mag
                } else {
                    1.0e12
                };
                unresolved_with_slope(&bound, kappa, current)
            }
        };
        let slope = slope_bound(&form, &current);
        let width = cell_width(&current);
        let target = 2.0 * params.metric_radius;
        let certified = cell_width(&witness_cell_of(&witness, &current));
        let decision = should_escalate(
            policy,
            depth,
            width,
            target,
            slope.mag,
            certified,
            budget.subdiv,
        );
        match decision {
            Escalation::Escalate(_) => {
                return Ok(Certified::new(
                    RegionVerdict::Escalated {
                        outcome: witness,
                        spent: spent(initial, budget),
                    },
                    interval_certificate(),
                ));
            }
            Escalation::Continue => {
                let cell = witness_cell_of(&witness, &current);
                let point = mid_of(&cell);
                match refine_toward(&current, point) {
                    Some(half) if cell_width(&half) < cell_width(&current) => {
                        current = half;
                        depth += 1;
                    }
                    _ => {
                        return Ok(Certified::new(
                            RegionVerdict::Escalated {
                                outcome: witness,
                                spent: spent(initial, budget),
                            },
                            interval_certificate(),
                        ));
                    }
                }
            }
        }
    }
}

/// The interval-method certificate of a driver verdict (untouched budget; the
/// solver spends its own per-stage budgets, and the retries spend from the
/// caller's shared budget).
fn interval_certificate() -> truck_base::evidence::Certificate {
    truck_base::evidence::Certificate {
        props: truck_base::evidence::PropMap::new(),
        method: truck_base::evidence::Method::Interval,
        budget_left: Budget::new(0, 0, 0),
        margin: truck_base::evidence::Margin::UNBOUNDED,
        modulus: truck_base::evidence::Modulus::Unbounded,
    }
}

/// Spend since entry: initial minus remaining.
fn spent(initial: Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct::bie::ssi4::RestrictedChart;
    use truck_geometry::prelude::{Plane, Point3, Sphere};

    /// The vertical plane `x = x0` charted by its own `(y, z)` coordinates:
    /// `X(u, v) = (x0, u, v)`.
    fn vertical_plane(x0: f64) -> Plane {
        Plane::new(
            Point3::new(x0, 0.0, 0.0),
            Point3::new(x0, 1.0, 0.0),
            Point3::new(x0, 0.0, 1.0),
        )
    }

    /// The planted-interior-loop fixture (BIE-004, Theorem E): the vertical
    /// plane `x = d` (`d = 3/2`) cuts the sphere of radius `2` about the
    /// origin in the closed circle
    ///
    /// ```text
    /// x = d,  y² + z² = ρ²,  ρ = sqrt(4 − d²) = sqrt(7)/2
    /// ```
    ///
    /// The windowed chart `(u, v) = (y, z)` on the plane and the sphere window
    /// `s ∈ [0.6, π − 0.6]`, `t ∈ (−0.95, 0.95)` contain the whole lift of the
    /// circle strictly in their interiors (the loop never touches any of the
    /// eight product-box faces), so boundary seeding cannot see it: the lift is
    /// a planted interior closed loop. The polar folds of the free axis `u`
    /// (the `u`-extremes of the loop, `τ_u = 0`) sit at
    ///
    /// ```text
    /// (u, v, s, t) = (±ρ, 0, π/2, atan2(±ρ, d))
    /// ```
    ///
    /// Each is a root of the polar-augmented square system (Theorem E).
    struct PlantedLoop {
        form: FForm,
        cell: WitnessCell,
        roots: [[f64; 4]; 2],
    }

    fn planted_loop() -> PlantedLoop {
        let d: f64 = 1.5;
        let rho = (4.0 - d * d).sqrt();
        let form = FForm {
            a: RestrictedChart::from_plane(vertical_plane(d)),
            b: RestrictedChart::from_sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0)),
        };
        let cell = WitnessCell::new((-2.0, 2.0), (-2.0, 2.0), (0.6, 2.6), (-0.95, 0.95));
        let plus = [rho, 0.0, std::f64::consts::FRAC_PI_2, rho.atan2(d)];
        let minus = [-rho, 0.0, std::f64::consts::FRAC_PI_2, (-rho).atan2(d)];
        PlantedLoop {
            form,
            cell,
            roots: [plus, minus],
        }
    }

    /// The empty sibling of the planted loop: the plane `x = 3` never meets
    /// the sphere (`3 > 2`), so the F-form has no zero anywhere in the same
    /// windows (the x-residual is at least 1 over the whole region).
    fn empty_pair() -> (FForm, WitnessCell) {
        let form = FForm {
            a: RestrictedChart::from_plane(vertical_plane(3.0)),
            b: RestrictedChart::from_sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0)),
        };
        let cell = WitnessCell::new((-2.0, 2.0), (-2.0, 2.0), (0.6, 2.6), (-0.95, 0.95));
        (form, cell)
    }

    /// A `WitnessCell` box of half-width `hw` about a chart point.
    fn box_around(p: [f64; 4], hw: f64) -> WitnessCell {
        WitnessCell::new(
            (p[0] - hw, p[0] + hw),
            (p[1] - hw, p[1] + hw),
            (p[2] - hw, p[2] + hw),
            (p[3] - hw, p[3] + hw),
        )
    }

    /// Whether a chart point lies inside the cell (inclusive bounds).
    fn contains(cell: &WitnessCell, p: &[f64; 4]) -> bool {
        cell.u.0 <= p[0]
            && p[0] <= cell.u.1
            && cell.v.0 <= p[1]
            && p[1] <= cell.v.1
            && cell.s.0 <= p[2]
            && p[2] <= cell.s.1
            && cell.t.0 <= p[3]
            && p[3] <= cell.t.1
    }

    /// Runs the exclusion oracle and returns its verdict (the operator's typed
    /// refusal is returned as `Err`).
    fn exclusion(
        form: &FForm,
        cell: &WitnessCell,
        free: usize,
        budget: &mut Budget,
    ) -> Result<ExclusionVerdict, Refusal> {
        match polar_exclusion(form, cell, free, budget) {
            Ok(Certified { value, .. }) => Ok(value),
            Err(refusal) => Err(refusal),
        }
    }

    /// The plane `z = 1` × sphere `r = 2` fixture F-form of the BIE-000 kit.
    fn transverse_form() -> FForm {
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        );
        FForm {
            a: RestrictedChart::from_plane(plane),
            b: RestrictedChart::from_sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0)),
        }
    }

    /// The tangency pair of the BIE-002 unresolved fixture: the plane `z = 2`
    /// tangent to the sphere `r = 2` at its pole.
    fn tangency_pair() -> (RestrictedChart, RestrictedChart) {
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        (
            RestrictedChart::from_plane(plane),
            RestrictedChart::from_sphere(Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0)),
        )
    }

    #[test]
    fn polar_exclusion_disposes_known_loop() {
        // The planted loop (plane x = 3/2 × sphere r = 2): the closed section
        // circle whose chart lift never touches the product-box faces, so the
        // boundary-seeded solver cannot see it (verified separately in the
        // no-loop-property test). The polar-augmented square system must
        // dispose of it: it is FOUND, never silently excluded.
        let planted = planted_loop();
        for root in planted.roots.iter() {
            assert!(
                contains(&planted.cell, root),
                "the fold root must lie strictly inside the planted-loop cell"
            );
        }

        // The empty sibling region is PROVEN empty: the exclusion certifies a
        // no-root verdict over the whole region.
        let (empty_form, empty_cell) = empty_pair();
        let mut budget = Budget::new(8192, 0, 0);
        let verdict = exclusion(&empty_form, &empty_cell, 0, &mut budget);
        assert!(
            matches!(verdict, Ok(ExclusionVerdict::Excluded)),
            "an empty region must be proven empty by the polar exclusion, got {verdict:?}"
        );

        // The planted-loop region is NOT proven empty: a fold root of the
        // polar system lies inside it, so the exclusion cannot dispose of it.
        let mut budget = Budget::new(2048, 0, 0);
        let verdict = exclusion(&planted.form, &planted.cell, 0, &mut budget);
        assert!(
            !matches!(verdict, Ok(ExclusionVerdict::Excluded)),
            "the polar exclusion must not dispose of a planted interior loop, got {verdict:?}"
        );

        // Each analytic fold root is a certified root of the polar-augmented
        // square system: the loop is FOUND, not guessed.
        // H-3: probe-box half-width in parameter units, not a model length.
        let hw = 1.0e-2; // H-3: probe-box half-width about the fold root, parameter units
        for root in planted.roots.iter() {
            let probe = box_around(*root, hw);
            let mut budget = Budget::new(16384, 0, 0);
            let verdict = exclusion(&planted.form, &probe, 0, &mut budget);
            assert!(
                matches!(verdict, Ok(ExclusionVerdict::LoopRoot)),
                "the polar system must certify the loop fold at {root:?}, got {verdict:?}"
            );
        }
    }

    #[test]
    fn no_loop_property_holds_on_every_cover() {
        // Theorem B oracle: on every cover the tests construct, a region
        // proven empty by exclusion contains no certified branch, and every
        // planted loop is found by the cover.
        let planted = planted_loop();
        let (empty_form, empty_cell) = empty_pair();

        // The restricted solver cannot see either branch: the planted loop is
        // strictly interior (no boundary seed exists) and the empty pair has
        // no zero at all, so neither base cell yields certified samples.
        let params = Ssi4Parameters::default();
        let planted_solve = certify_restricted_pair(
            planted.form.a.clone(),
            planted.form.b.clone(),
            planted.cell,
            &params,
            &mut Budget::new(0, 0, 0),
        );
        let empty_solve = certify_restricted_pair(
            empty_form.a.clone(),
            empty_form.b.clone(),
            empty_cell,
            &params,
            &mut Budget::new(0, 0, 0),
        );
        let planted_curve = match planted_solve {
            Ok(Certified { value, .. }) => value,
            Err(_) => return,
        };
        let empty_curve = match empty_solve {
            Ok(Certified { value, .. }) => value,
            Err(_) => return,
        };
        assert!(
            planted_curve.samples.is_empty(),
            "the planted interior loop must be invisible to boundary seeding"
        );
        assert!(
            empty_curve.samples.is_empty(),
            "the empty pair must certify no branch samples"
        );

        let covers: [[usize; 4]; 5] = [
            [2, 1, 1, 1],
            [3, 1, 1, 1],
            [4, 1, 1, 1],
            [2, 1, 2, 1],
            [2, 2, 2, 2],
        ];
        for parts in covers.iter() {
            let regions = cover_of(&planted.cell, *parts);
            let empties = cover_of(&empty_cell, *parts);
            assert_eq!(
                regions.len(),
                empties.len(),
                "covers of equal cells have equal size"
            );
            for (region, empty_region) in regions.iter().zip(empties.iter()) {
                // Planted loop: a region that contains a fold root is FOUND —
                // the exclusion never proves it empty (soundness: a true root
                // of the polar system lies inside the closed box).
                let has_fold = planted.roots.iter().any(|root| contains(region, root));
                let mut budget = Budget::new(1024, 0, 0);
                let verdict = exclusion(&planted.form, region, 0, &mut budget);
                if has_fold {
                    assert!(
                        !matches!(verdict, Ok(ExclusionVerdict::Excluded)),
                        "a cover region holding a planted-loop fold must not be \
                         proven empty, got {verdict:?}"
                    );
                }
                if matches!(verdict, Ok(ExclusionVerdict::Excluded)) {
                    assert!(
                        planted_curve.samples.is_empty(),
                        "an excluded region must contain no certified branch"
                    );
                }

                // Empty pair: every cover region is PROVEN empty, and no
                // excluded region contains a certified branch.
                let mut budget = Budget::new(2048, 0, 0);
                let verdict = exclusion(&empty_form, empty_region, 0, &mut budget);
                assert!(
                    matches!(verdict, Ok(ExclusionVerdict::Excluded)),
                    "every empty-pair cover region must be proven empty, got {verdict:?}"
                );
                assert!(
                    empty_curve.samples.is_empty(),
                    "an excluded empty-pair region must contain no certified branch"
                );
            }
        }
    }

    #[test]
    fn slope_diagnostic_orders_escalation() {
        // The §5.4 certified slope bound ranks two degenerate boxes: a small
        // box about a transverse section point (plane z = 1 × sphere r = 2,
        // the circle point (√3, 0, π/3, 0) of the fixture algebra) certifies a
        // sign and a positive magnitude, while a box about the face-tangency
        // pole (plane z = 2 tangent to the sphere) is sign-less with zero
        // magnitude. The transverse box orders above the tangential one.
        let form = transverse_form();
        let s3 = 3.0_f64.sqrt();
        // H-3: box half-width around the section point, parameter units.
        let hw = 1.0e-2; // H-3: transverse-box half-width, parameter units
        let transverse = box_around([s3, 0.0, std::f64::consts::FRAC_PI_3, 0.0], hw);
        let trans = slope_bound(&form, &transverse);
        assert!(
            trans.sign.is_some(),
            "a transverse box must certify a slope sign"
        );
        assert!(
            trans.mag > 0.0,
            "a transverse box must certify a positive slope magnitude"
        );

        let (a, b) = tangency_pair();
        let tang_form = FForm { a, b };
        let tang_cell =
            WitnessCell::new((0.0, 2.0e-2), (0.0, 2.0e-2), (0.0, 2.0e-2), (0.0, 2.0e-2));
        let tang = slope_bound(&tang_form, &tang_cell);
        assert_eq!(
            tang.sign, None,
            "the tangency box must not certify a slope sign"
        );
        assert_eq!(
            tang.mag, 0.0,
            "the tangency box must certify no slope magnitude"
        );

        // Ordering: the certified magnitude of the transverse box strictly
        // exceeds that of the tangential box.
        assert!(
            trans.mag > tang.mag,
            "the slope diagnostic must rank the transverse box above the tangential one"
        );

        // The bound feeds the frozen κ/cell/slope witness (never a guess).
        let kappa = 1.0e2;
        let w_trans = unresolved_with_slope(&trans, kappa, transverse);
        let w_tang = unresolved_with_slope(&tang, 1.0e12, tang_cell);
        assert!(
            matches!(&w_trans, InteractionOutcome::Unresolved { slope, .. } if *slope > 0.0),
            "the certified transverse slope must feed the witness"
        );
        assert!(
            matches!(&w_tang, InteractionOutcome::Unresolved { slope, .. } if *slope == 0.0),
            "the sign-less tangential slope must feed the witness as zero"
        );
        assert!(
            matches!(
                w_tang.clone().into_landed_refusal(),
                Some(Refusal::NumericallyUnresolved { .. })
            ),
            "the typed witness maps onto the landed NumericallyUnresolved refusal"
        );

        // The slope diagnostic orders the escalation decision: the tangential
        // magnitude trips the certified slope floor; the transverse one does
        // not.
        let policy = EscalationPolicy::default();
        assert_eq!(
            should_escalate(&policy, 0, 0.2, 0.04, tang.mag, 0.2, 100),
            Escalation::Escalate(EscalationReason::SlopeFloor),
            "the tangential slope magnitude must escalate on the slope floor"
        );
        assert_eq!(
            should_escalate(&policy, 0, 0.2, 0.04, trans.mag, 0.1, 100),
            Escalation::Continue,
            "the transverse slope magnitude must not escalate on the slope floor"
        );
    }

    #[test]
    fn retry_terminates_or_escalates() {
        // The face-tangency retry loop (§13.3): on the tangency fixture the
        // scheduler escalates to a typed Unresolved within the budget — it
        // never spins and never exhausts the budget by retry alone.
        let (a, b) = tangency_pair();
        // A small region about the tangency point.
        let region = WitnessCell::new((0.0, 0.1), (0.0, 0.1), (0.0, 0.1), (0.0, 0.1));
        let params = Ssi4Parameters::default();

        // Default policy: the certified slope magnitude of the tangency region
        // is zero, below the certified transversality floor, so the very first
        // unresolved round escalates — no retry spin.
        let mut budget = Budget::new(128, 0, 0);
        let verdict = solve_pair_region(
            a.clone(),
            b.clone(),
            region,
            &params,
            &EscalationPolicy::default(),
            &mut budget,
        );
        match verdict {
            Ok(Certified {
                value: RegionVerdict::Escalated { outcome, spent },
                ..
            }) => {
                assert!(
                    matches!(&outcome, InteractionOutcome::Unresolved { .. }),
                    "the escalation must be a typed Unresolved, got {outcome:?}"
                );
                assert!(
                    matches!(
                        outcome.clone().into_landed_refusal(),
                        Some(Refusal::NumericallyUnresolved { .. })
                    ),
                    "the escalation maps onto the landed NumericallyUnresolved refusal"
                );
                assert!(
                    spent.subdiv <= 1 && budget.subdiv > 0,
                    "the tangency escalates on the first unresolved round without \
                     exhausting the budget"
                );
            }
            other => panic!("the tangency driver must escalate, got {other:?}"),
        }

        // Retry policy with the slope floor disabled: the scheduler must
        // refine at most `max_retry_depth` times toward the unresolved cell
        // and then escalate typed (terminates, never spins), spending no more
        // than the depth cap plus one from the shared budget.
        let retry_policy = EscalationPolicy {
            slope_floor: 0.0,
            max_retry_depth: 1,
            progress_floor: 1.0,
        };
        let mut budget = Budget::new(128, 0, 0);
        let verdict = solve_pair_region(
            a.clone(),
            b.clone(),
            region,
            &params,
            &retry_policy,
            &mut budget,
        );
        match verdict {
            Ok(Certified {
                value: RegionVerdict::Escalated { outcome, spent },
                ..
            }) => {
                assert!(
                    matches!(&outcome, InteractionOutcome::Unresolved { .. }),
                    "the retry must end in a typed Unresolved, got {outcome:?}"
                );
                assert!(
                    spent.subdiv <= retry_policy.max_retry_depth + 1,
                    "the retry loop must stop at the depth cap, spent {}",
                    spent.subdiv
                );
                assert!(
                    budget.subdiv > 0,
                    "the retry loop must not exhaust the budget by retry alone"
                );
            }
            other => panic!("the retried tangency driver must escalate, got {other:?}"),
        }

        // Control: the transverse plane × sphere circle certifies under the
        // default scheduler (no escalation on a healthy pair).
        let form = transverse_form();
        let cell = WitnessCell::new(
            (-2.0, 2.0),
            (-2.0, 2.0),
            (0.0, std::f64::consts::PI),
            (0.0, std::f64::consts::TAU),
        );
        let mut budget = Budget::new(128, 0, 0);
        let verdict = solve_pair_region(
            form.a,
            form.b,
            cell,
            &params,
            &EscalationPolicy::default(),
            &mut budget,
        );
        match verdict {
            Ok(Certified {
                value: RegionVerdict::Certified(curve),
                ..
            }) => assert!(
                !curve.samples.is_empty(),
                "the transverse fixture must certify branch samples"
            ),
            other => panic!("the transverse driver must certify, got {other:?}"),
        }
    }
}
