// The crate-level deny list includes clippy::indexing_slicing. The dense
// 3x4 / 4x4 arithmetic below indexes only fixed-size arrays with constant or
// iterator-derived indices that are in bounds by construction (never a value
// derived from geometry), so the lint is re-allowed for this module only.
#![allow(clippy::indexing_slicing)]

//! BIE-002-SSI4: the parallelotope continuation tracker (theory §3.3 θρ step).
//!
//! The restricted-pair solver certifies the zero set of a 3-vector residual
//! `F(x)` over a 4-D parameter chart, a set that is generically a curve. The
//! tracker owns the *continuation algebra* of that curve, expressed entirely
//! over [`Interval`] boxes and the landed Krawczyk operator
//! ([`crate::num::krawczyk::krawczyk`]) — no geometry, no pair semantics, and
//! no dependency on any certified-crate type. The pair-side system
//! construction lives with the caller ([`KrawczykSystem<4>`]); this module
//! turns a certified solution point and its Jacobian into the *next* certified
//! solution box.
//!
//! The θρ step is a certified predictor–corrector:
//!
//! 1. **θ (predict).** From a certified point `p_k` on the branch and its unit
//!    tangent `τ_k` (the kernel direction of the 3×4 Jacobian `DF(p_k)`), the
//!    next centre is predicted along the tangent: `c = p_k + step·τ_k`. The
//!    `step` is a *metric* advance (see [`ParallelotopeFrame::predict`]); the
//!    caller picks the parallelotope radii.
//! 2. **ρ (correct).** The branch is re-localized by a hyperplane normal to the
//!    tangent through the prediction: the augmented square system
//!    `G(x) = (F(x), τ_k·(x − c))`. The caller builds the [`KrawczykSystem<4>`]
//!    for that augmentation; [`theta_rho_step`] boxes the prediction with the
//!    caller's radii and runs the Krawczyk operator. A strict-interior result
//!    certifies that the parallelotope box contains exactly one solution of the
//!    augmented system, i.e. the next point of the branch.
//! 3. **Re-frame.** [`ParallelotopeFrame::from_jacobian`] recomputes the
//!    tangent (and a deterministic orthonormal transversal completion) at the
//!    new centre, feeding the next θ step.
//!
//! The "parallelotope" is the metric box spanned by the tangent direction (θ)
//! and the three transversal (ρ) directions about the prediction; the frame
//! record ([`ParallelotopeFrame`]) is its oriented description.
//!
//! **House rules.** H-1 (no unwrap/expect/panic reachable from geometry; the
//! only indexing is on fixed arrays), H-2 (fallible operations return
//! [`Outcome`]), H-6 (float-computed tangents and predictions are never
//! recorded as certified values — only the Krawczyk-certified box is), and the
//! determinism rule (fixed-order Gram–Schmidt, no hash iteration; the tangent
//! is computed by the cofactor/minor formula, never an SVD).
//!
//! This module's own unit tests exercise the algebra on toy systems built in
//! this file; the restricted-pair integration lives with the caller packet.

use crate::enclosure::Interval;
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use truck_base::evidence::{Budget, Certificate, Refusal};

/// The 3×4 Jacobian convention of the restricted-pair F-form: row-major
/// `[component][axis]`, `axis` ordered over the two carriers' parameter
/// coordinates.
pub type Jacobian4 = [[f64; 4]; 3];

/// The outcome of one certified θρ step ([`theta_rho_step`]).
#[derive(Clone, Debug)]
pub enum StepVerdict<const N: usize> {
    /// The parallelotope box contains exactly one solution of the augmented
    /// system: the next certified branch sample.
    Certified {
        /// The certified N-D parameter box: exactly one solution of the
        /// augmented slice system lies in it (one certified branch sample).
        cell: [Interval; N],
        /// The float centre used for the certification (predictor/corrector
        /// bookkeeping; the certified statement is `cell`).
        center: [f64; N],
        /// The Krawczyk certificate.
        cert: Certificate,
    },
    /// The operator proved the box contains no solution of the augmented
    /// system.
    NoRoot,
    /// The operator refused (budget exhaustion, an unsplittable box, or an
    /// empty component). This is a typed refusal, never a guess.
    Refused(Refusal),
}

/// The frame record of a certified branch point: the centre, the unit tangent
/// (the θ direction of the θρ step), and a deterministic orthonormal
/// transversal completion (the ρ directions).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParallelotopeFrame {
    /// The float centre of the certified point (bookkeeping).
    pub center: [f64; 4],
    /// The unit tangent of the branch at `center`.
    pub tangent: [f64; 4],
    /// Three unit vectors orthogonal to `tangent` and to each other, in fixed
    /// index order (deterministic Gram–Schmidt, no hash iteration).
    pub transversal: [[f64; 4]; 3],
}

impl ParallelotopeFrame {
    /// Builds the frame at `center` from the float Jacobian `jac` of the
    /// 3-vector residual. `None` when the Jacobian's kernel is not certified
    /// computable (rank < 3: the tangent direction vanishes), which is the
    /// typed-degeneracy condition of the caller.
    pub fn from_jacobian(center: [f64; 4], jac: &Jacobian4) -> Option<Self> {
        let tangent = tangent_from_jacobian(jac)?;
        Some(ParallelotopeFrame {
            center,
            tangent,
            transversal: orthogonal_completion(tangent),
        })
    }

    /// The θ prediction: the centre advanced by `step` along the unit tangent.
    pub fn predict(&self, step: f64) -> [f64; 4] {
        let mut out = self.center;
        for (o, (c, t)) in out
            .iter_mut()
            .zip(self.center.iter().zip(self.tangent.iter()))
        {
            *o = *c + step * *t;
        }
        out
    }

    /// The predicted centre with the tangent kept unit under the recorded
    /// frame after moving to a new centre (used when a step's certified centre
    /// is adopted verbatim).
    pub fn moved_to(&self, center: [f64; 4]) -> Self {
        ParallelotopeFrame {
            center,
            tangent: self.tangent,
            transversal: self.transversal,
        }
    }
}

/// The unit kernel direction of a 3×4 float Jacobian, by the cofactor/minor
/// formula: `v_j = (−1)^j·det(J with column j deleted)` is orthogonal to every
/// row, and the direction is the normalized vector. `None` when every 3×3
/// minor vanishes (the Jacobian does not have full rank: no well-defined
/// tangent).
///
/// Deterministic: the alternating sign pattern and the fixed column order are
/// the whole algorithm — no pivot search, no SVD, no iteration order.
pub fn tangent_from_jacobian(jac: &Jacobian4) -> Option<[f64; 4]> {
    let mut v = [0.0; 4];
    for (j, vi) in v.iter_mut().enumerate() {
        let m = minor3(&jac[0], &jac[1], &jac[2], j);
        let d = det3(&m);
        *vi = if j % 2 == 0 { d } else { -d };
    }
    let mut norm = 0.0;
    for &c in &v {
        norm += c * c;
    }
    if !norm.is_finite() || norm == 0.0 {
        return None;
    }
    let len = norm.sqrt();
    for c in v.iter_mut() {
        *c /= len;
    }
    Some(v)
}

/// A deterministic orthonormal completion of the unit `tangent`: three unit,
/// mutually orthogonal vectors each orthogonal to `tangent`, produced by
/// modified Gram–Schmidt over the canonical basis in fixed axis order.
fn orthogonal_completion(tangent: [f64; 4]) -> [[f64; 4]; 3] {
    let basis: [[f64; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut vectors: Vec<[f64; 4]> = Vec::new();
    for b in basis.iter() {
        let mut w = *b;
        // Project out the tangent and the vectors already chosen.
        let d = dot4(b, &tangent);
        for k in 0..4 {
            w[k] = b[k] - d * tangent[k];
        }
        for v in &vectors {
            let d = dot4(&w, v);
            for k in 0..4 {
                w[k] -= d * v[k];
            }
        }
        let mut norm = 0.0;
        for &c in &w {
            norm += c * c;
        }
        if norm.is_finite() && norm > 0.0 {
            let len = norm.sqrt();
            for c in w.iter_mut() {
                *c /= len;
            }
            vectors.push(w);
            if vectors.len() == 3 {
                break;
            }
        }
    }
    let empty = [0.0; 4];
    [
        vectors.first().copied().unwrap_or(empty),
        vectors.get(1).copied().unwrap_or(empty),
        vectors.get(2).copied().unwrap_or(empty),
    ]
}

/// The 3×3 minor of the rows `r0, r1, r2` after deleting column `col`.
fn minor3(r0: &[f64; 4], r1: &[f64; 4], r2: &[f64; 4], col: usize) -> [[f64; 3]; 3] {
    let take = |r: &[f64; 4]| -> [f64; 3] {
        let mut out = [0.0; 3];
        let mut k = 0usize;
        for (c, val) in r.iter().enumerate() {
            if c == col {
                continue;
            }
            out[k] = *val;
            k += 1;
        }
        out
    };
    [take(r0), take(r1), take(r2)]
}

/// The determinant of a 3×3 float matrix by the explicit 6-term expansion.
fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The Euclidean dot product of two 4-vectors.
fn dot4(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

/// A closed interval from two finite, ordered floats. A non-finite or inverted
/// pair degrades to the empty interval (a caller bug downstream, never a
/// panic).
fn interval(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The outward-rounded box of half-width `radii` about `center`. `None` when a
/// radius is not positive and finite (there is nothing to certify).
pub fn box_around<const N: usize>(center: [f64; N], radii: [f64; N]) -> Option<[Interval; N]> {
    let mut out = [Interval::EMPTY; N];
    for i in 0..N {
        let r = radii[i];
        if !r.is_finite() || r <= 0.0 {
            return None;
        }
        out[i] = interval(center[i] - r, center[i] + r);
    }
    Some(out)
}

/// The certified θρ step (theory §3.3) over the parallelotope about `center`:
/// box the prediction with `radii`, run the Krawczyk operator over the
/// caller-supplied [`KrawczykSystem<N>`] (the augmented square system whose
/// zero is the next branch point), and return the typed verdict.
///
/// The returned certified `cell` is the parallelotope box the operator proved
/// contains exactly one solution; the `center` is the float prediction that
/// produced it (H-6: the certified statement is the box, never the float).
pub fn theta_rho_step<const N: usize>(
    system: &impl KrawczykSystem<N>,
    center: [f64; N],
    radii: [f64; N],
    budget: &mut Budget,
) -> StepVerdict<N> {
    let cell = match box_around(center, radii) {
        Some(cell) => cell,
        None => {
            return StepVerdict::Refused(Refusal::NumericallyUnresolved {
                spent: *budget,
                witness: truck_base::evidence::UnresolvedWitness::KrawczykIndeterminate,
            })
        }
    };
    use truck_base::evidence::Certified;
    match krawczyk::<N>(system, &cell, budget) {
        Ok(Certified {
            value: KrawczykProof::Unique,
            cert,
        }) => StepVerdict::Certified { cell, center, cert },
        Ok(Certified {
            value: KrawczykProof::NoRoot,
            ..
        }) => StepVerdict::NoRoot,
        Err(refusal) => StepVerdict::Refused(refusal),
    }
}

// ---------------------------------------------------------------------------
// Unit tests (algebraic, on toy systems built here — no certified-crate types)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![deny(clippy::unwrap_used)]
    #![allow(clippy::needless_range_loop)] // test-only matrix loops over fixed-size arrays; never geometry-derived indices
    use super::*;

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The unit tangent direction of the plane × sphere circle of the BIE-000
    /// fixture kit, at a chart point on the branch: proportional to
    /// `(−sin t, cos t, 0, 1)` in `(u, v, s, t)` order.
    fn plane_sphere_tangent(t: f64) -> [f64; 4] {
        let s3 = 3.0_f64.sqrt();
        let raw = [-s3 * t.sin(), s3 * t.cos(), 0.0, 1.0];
        let mut len = 0.0;
        for &c in &raw {
            len += c * c;
        }
        let len = len.sqrt();
        [raw[0] / len, raw[1] / len, raw[2] / len, raw[3] / len]
    }

    /// The float 3×4 Jacobian of `F = X_plane − X_sphere` at a point on the
    /// circle `(u, v, s) = (√3 cos t, √3 sin t, π/3)`: columns are
    /// `(X_u, X_v, −X_s, −X_t)`.
    fn plane_sphere_jacobian(t: f64) -> Jacobian4 {
        // X_u = (1,0,0), X_v = (0,1,0).
        // −X_s = −(cos s cos t, cos s sin t, −sin s)·R with s = π/3, R = 2.
        let s = std::f64::consts::FRAC_PI_3;
        let r = 2.0;
        [
            [1.0, 0.0, -r * s.cos() * t.cos(), r * s.sin() * t.sin()],
            [0.0, 1.0, -r * s.cos() * t.sin(), -r * s.sin() * t.cos()],
            [0.0, 0.0, r * s.sin(), 0.0],
        ]
    }

    #[test]
    fn tangent_from_jacobian_is_unit_and_in_kernel() {
        for &t in &[0.0, 0.7, 1.3, 2.9, 4.2, 5.8] {
            let jac = plane_sphere_jacobian(t);
            let tan = tangent_from_jacobian(&jac).unwrap_or([0.0; 4]);
            let mut norm = 0.0;
            for &c in &tan {
                norm += c * c;
            }
            let norm = norm.sqrt();
            // H-3: unit-magnitude tolerance on a normalized direction, not a length.
            let tol = 1.0e-9; // H-3: normalized-direction unit tolerance, dimensionless
            assert!(
                (norm - 1.0).abs() <= tol,
                "tangent must be unit, got {norm}"
            );
            for row in jac.iter() {
                let mut dot = 0.0;
                for k in 0..4 {
                    dot += row[k] * tan[k];
                }
                assert!(dot.abs() <= tol, "row·tangent = {dot} must vanish");
            }
            // The kernel direction agrees (up to sign) with the known tangent,
            // oriented so the t-component is positive.
            let expected = plane_sphere_tangent(t);
            let mut dot = 0.0;
            for k in 0..4 {
                dot += expected[k] * tan[k];
            }
            assert!(
                dot.abs() >= 1.0 - tol,
                "kernel direction must align with the true tangent, |dot| = {dot}"
            );
        }
    }

    #[test]
    fn frame_completion_is_orthonormal() {
        let jac = plane_sphere_jacobian(0.0);
        let frame = ParallelotopeFrame::from_jacobian([1.732, 0.0, 1.047, 0.0], &jac).unwrap_or(
            ParallelotopeFrame {
                center: [0.0; 4],
                tangent: [0.0; 4],
                transversal: [[0.0; 4]; 3],
            },
        );
        let vectors: Vec<[f64; 4]> = std::iter::once(frame.tangent)
            .chain(frame.transversal.iter().copied())
            .collect();
        for i in 0..vectors.len() {
            let v = &vectors[i];
            let mut norm = 0.0;
            for &c in v {
                norm += c * c;
            }
            let norm = norm.sqrt();
            // H-3: unit-magnitude tolerance on normalized frame vectors.
            let tol = 1.0e-9; // H-3: orthonormal-frame unit tolerance, dimensionless
            assert!((norm - 1.0).abs() <= tol, "frame vector {i} not unit");
            for j in (i + 1)..vectors.len() {
                let w = &vectors[j];
                let mut dot = 0.0;
                for k in 0..4 {
                    dot += v[k] * w[k];
                }
                assert!(dot.abs() <= tol, "frame vectors {i},{j} not orthogonal");
            }
        }
    }

    #[test]
    fn predict_advances_along_the_tangent() {
        let jac = plane_sphere_jacobian(0.0);
        let frame = ParallelotopeFrame::from_jacobian([1.732, 0.0, 1.047, 0.0], &jac).unwrap_or(
            ParallelotopeFrame {
                center: [1.732, 0.0, 1.047, 0.0],
                tangent: [0.0, 0.0, 0.0, 1.0],
                transversal: [[0.0; 4]; 3],
            },
        );
        // H-3: dimensionless parameter advance along a unit tangent.
        let step = 0.1; // H-3: θ-step length in parameter units, not a model length
        let predicted = frame.predict(step);
        let mut moved = [0.0; 4];
        for k in 0..4 {
            moved[k] = predicted[k] - frame.center[k];
        }
        let mut norm = 0.0;
        for &c in &moved {
            norm += c * c;
        }
        let norm = norm.sqrt();
        // H-3: dimensionless unit-step tolerance on the prediction.
        let tol = 1.0e-9; // H-3: θ-step length tolerance, dimensionless
        assert!(
            (norm - step).abs() <= tol,
            "prediction must advance by |step|"
        );
    }

    /// A linear 4×4 augmented system `G(x) = M·(x − x*)` with a known root:
    /// the one-shot Unique / NoRoot witnesses of the operator over a box.
    struct Linear4 {
        m: [[f64; 4]; 4],
        root: [f64; 4],
    }

    impl KrawczykSystem<4> for Linear4 {
        fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
            let mut out = [Interval::EMPTY; 4];
            for r in 0..4 {
                let mut acc = 0.0f64;
                for c in 0..4 {
                    acc += self.m[r][c] * (x[c] - self.root[c]);
                }
                out[r] = iv(acc, acc);
            }
            out
        }
        fn jacobian(&self, _b: &[Interval; 4]) -> [[Interval; 4]; 4] {
            let mut out = [[Interval::EMPTY; 4]; 4];
            for r in 0..4 {
                for c in 0..4 {
                    let v = self.m[r][c];
                    out[r][c] = iv(v, v);
                }
            }
            out
        }
        fn preconditioner(&self, _x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
            inverse4(&self.m)
        }
    }

    /// The exact float inverse of a 4×4 matrix by Gauss–Jordan with partial
    /// pivoting. `None` on a singular matrix.
    fn inverse4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
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
            if best == 0.0 || !best.is_finite() {
                return None;
            }
            if pivot != col {
                for c in 0..4 {
                    a.swap(col, c);
                    inv.swap(col, c);
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

    #[test]
    fn theta_rho_step_certifies_a_known_solution() {
        // A mildly coupled 4×4 linear system with an exact root.
        let m = [
            [4.0, 1.0, 0.0, 1.0],
            [0.0, 3.0, 1.0, 0.0],
            [1.0, 0.0, 5.0, 1.0],
            [0.0, 1.0, 0.0, 2.0],
        ];
        let root = [0.5, -0.25, 0.75, 0.125];
        let system = Linear4 { m, root };
        let mut budget = Budget::new(16, 0, 0);
        // H-3: dimensionless certified-box half-width in parameter units.
        let radius = 1.0e-3; // H-3: parallelotope half-width, parameter units, not a length
        let radii = [radius; 4];
        let verdict = theta_rho_step(&system, root, radii, &mut budget);
        assert!(
            matches!(
                verdict,
                StepVerdict::Certified { cell, center, .. }
                    if center == root
                        && cell.iter().enumerate().all(|(k, iv)| iv.contains(root[k]))
            ),
            "a nonsingular linear system must certify one-shot about its root"
        );
    }

    #[test]
    fn theta_rho_step_proves_no_root_away_from_the_solution() {
        let m = [
            [4.0, 1.0, 0.0, 1.0],
            [0.0, 3.0, 1.0, 0.0],
            [1.0, 0.0, 5.0, 1.0],
            [0.0, 1.0, 0.0, 2.0],
        ];
        let root = [0.5, -0.25, 0.75, 0.125];
        let system = Linear4 { m, root };
        let mut budget = Budget::new(64, 0, 0);
        // A far-away parallelotope contains no solution of the linear system.
        let far = [10.0, 10.0, 10.0, 10.0];
        // H-3: dimensionless certified-box half-width in parameter units.
        let radius = 1.0e-2; // H-3: parallelotope half-width, parameter units, not a length
        let verdict = theta_rho_step(&system, far, [radius; 4], &mut budget);
        assert!(
            matches!(verdict, StepVerdict::NoRoot),
            "a box far from the linear root must prove NoRoot"
        );
    }

    #[test]
    fn singular_system_refuses_typed() {
        // A rank-deficient M: the preconditioner is None, the operator bisects
        // toward a typed refusal instead of a guess.
        let m = [
            [1.0, 2.0, 3.0, 4.0],
            [2.0, 4.0, 6.0, 8.0],
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
        ];
        let root = [0.0; 4];
        let system = Linear4 { m, root };
        let mut budget = Budget::new(4, 0, 0);
        // H-3: dimensionless certified-box half-width in parameter units.
        let radius = 1.0e-2; // H-3: parallelotope half-width, parameter units, not a length
        let verdict = theta_rho_step(&system, root, [radius; 4], &mut budget);
        assert!(
            matches!(verdict, StepVerdict::Refused(_)),
            "a singular augmented system must refuse typed"
        );
    }
}
