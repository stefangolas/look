//! BG-NUM-003: the Krawczyk existence/uniqueness operator.
//!
//! The contract (spec §Stage 3):
//!
//! ```text
//! K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)
//!   m = midpoint(Q) (float),  Y = float inverse of J(m)
//!   K ⊆ strict interior(Q)  ->  Proven(unique root in Q)   # existence AND uniqueness
//!   K ∩ Q = ∅              ->  Proven(no root in Q)
//!   otherwise              ->  bisect under Budget
//! ```
//!
//! **The center term `F(m)` is a point evaluation** — never the interval `F`
//! over `Q`, which decorrelates the linear part against the contraction term
//! and certifies nothing (measured on the BG-ENC-004-ISC carrier: K ≥ 5×
//! width(Q) at every scale with the interval center, second-order width with
//! the point center). `Proven(unique)` is emitted **only** on strict interior
//! containment — `K ⊆ Q` non-strict proves existence, not uniqueness.
//! The parameterized form (system additionally depending on `t ∈ T`) follows
//! the same rule with `F(m, t_mid)` and `J(Q, T)`.
//!
//! # The trait contract
//!
//! [`KrawczykSystem`] splits the operator's three inputs: the point evaluation
//! [`KrawczykSystem::f_point`], the interval Jacobian [`KrawczykSystem::jacobian`]
//! over a box in **row-major** `[row][col] = [dF_row/dx_col]` order — the
//! operator relies on that convention and never transposes — and the float
//! approximate inverse [`KrawczykSystem::preconditioner`] at a point. The
//! system supplies its own float inverse, so the operator holds no
//! linear-algebra machinery.
//!
//! # A `None` preconditioner bisects
//!
//! A `None` preconditioner (a vanishing derivative at the box midpoint, e.g.
//! x²+1 at m = 0) says nothing about the box, so the operator **bisects** on
//! `None` rather than refusing: refusing there turns every symmetric no-root
//! instance into a spurious `NumericallyUnresolved`, and bisecting costs
//! nothing when the answer is `NoRoot` (the children prune).
//!
//! # Worklist and bisection shape under `Budget`
//!
//! A `Vec<[Interval; N]>` worklist, initialised with `start`, pops a box and
//! either certifies (strict interior containment → [`KrawczykProof::Unique`]),
//! prunes (`K ∩ Q = ∅` on any axis → no root in this box), or bisects the
//! widest axis (ties toward the lowest index, deterministically), spending one
//! subdivision per bisection. The two terminal states: the worklist empties
//! without certification → [`KrawczykProof::NoRoot`]; a bisection cannot spend,
//! the box is a degenerate point, or the widest axis cannot be bisected in f64
//! (the midpoint rounds onto one of the box's own edges — it is at resolution)
//! → `Refusal::NumericallyUnresolved` carrying spend. Over-estimation never
//! occurs silently: every non-proof exit is a typed [`Refusal`] carrying spend.

use std::array;

use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
    UnresolvedWitness,
};

/// A system the Krawczyk operator can prove things about.
pub trait KrawczykSystem<const N: usize> {
    /// F at a POINT, evaluated exactly (each component wrapped as a
    /// degenerate interval). Never evaluate F over the whole box here.
    fn f_point(&self, x: &[f64; N]) -> [Interval; N];
    /// The interval Jacobian over a box. ROW-MAJOR:
    /// `jacobian(b)[r][c] = lower..upper of dF_r/dx_c over b`.
    /// This convention is yours to rely on — the operator never transposes.
    fn jacobian(&self, b: &[Interval; N]) -> [[Interval; N]; N];
    /// A float approximate inverse of J at a point. `None` means the
    /// system cannot supply one here (singular derivative) — the operator
    /// BISECTS on None, it does not refuse (decision 4).
    fn preconditioner(&self, x: &[f64; N]) -> Option<[[f64; N]; N]>;
}

/// What the operator proved about the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KrawczykProof {
    /// Exactly one solution in the box (strict-interior rule, decision 5).
    Unique,
    /// No solution in the searched region.
    NoRoot,
}

/// The Krawczyk existence/uniqueness operator over a worklist.
pub fn krawczyk<const N: usize>(
    system: &impl KrawczykSystem<N>,
    start: &[Interval; N],
    budget: &mut Budget,
) -> Outcome<KrawczykProof> {
    // Spend is reported as initial − remaining (decision 2), so the entry
    // budget is captured once.
    let initial = *budget;
    let mut stack: Vec<[Interval; N]> = vec![*start];
    while let Some(q) = stack.pop() {
        // (decision 1 step 1) an empty or non-finite component refuses: there
        // is nothing to certify.
        if q.iter()
            .any(|c| c.is_empty() || !c.inf().is_finite() || !c.sup().is_finite())
        {
            return Err(Refusal::Empty);
        }
        // (decision 1 step 2) float midpoints, verified inside the box — a
        // naive `0.5·(inf + sup)` can round outside its own box at extreme
        // magnitudes.
        let mut m = [0.0; N];
        let mut midpoint_ok = true;
        for (mi, axis) in m.iter_mut().zip(q.iter()) {
            *mi = 0.5 * (axis.inf() + axis.sup());
            if !(axis.inf() <= *mi && *mi <= axis.sup()) {
                midpoint_ok = false;
            }
        }
        if !midpoint_ok {
            // A midpoint rounded outside its box cannot feed the
            // preconditioner; take the bisection path (decision 1 step 2),
            // refusing only at zero width.
            push_children(&q, &mut stack, &initial, budget)?;
            continue;
        }
        // (decision 1 step 3)
        let Some(y) = system.preconditioner(&m) else {
            // (decision 4) a None preconditioner says nothing about the box:
            // bisect rather than refuse. Bisecting costs nothing when the
            // answer is NoRoot (the children prune).
            push_children(&q, &mut stack, &initial, budget)?;
            continue;
        };
        // (decision 1 steps 4–6) point center, interval Jacobian, K image.
        let f = system.f_point(&m);
        let j = system.jacobian(&q);
        let k = k_image(&q, &m, &y, &f, &j);
        // (decision 1 step 7 / decision 5) STRICT interior containment on all
        // axes, no empty k — non-strict containment proves existence only.
        let strict = k
            .iter()
            .zip(q.iter())
            .all(|(kv, qv)| !kv.is_empty() && kv.inf() > qv.inf() && kv.sup() < qv.sup());
        if strict {
            return Ok(Certified::new(KrawczykProof::Unique, certificate(budget)));
        }
        // (decision 1 step 8) any axis with an empty intersection proves no
        // root in this box: discard it and continue.
        if k.iter()
            .zip(q.iter())
            .any(|(kv, qv)| kv.intersection(*qv).is_empty())
        {
            continue;
        }
        // (decision 1 step 9) otherwise bisect.
        push_children(&q, &mut stack, &initial, budget)?;
    }
    // The worklist emptied without certification: no root in the searched
    // region.
    Ok(Certified::new(KrawczykProof::NoRoot, certificate(budget)))
}

/// The Krawczyk image `K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)`. Row `r`:
/// `iv(m[r]) − Σ_c y[r][c]·f[c] + Σ_c d[r][c]·(q[c] − iv(m[c]))` with
/// `d[r][c] = δ(r,c) − Σ_k y[r][k]·j[k][c]`, row-major throughout — the
/// system's row-major Jacobian convention is relied on, never transposed.
/// (BG-NUM-003-CONTRACT: the original spec wrote `d[r][c] = δ(r,c) −
/// y[r][c]·j[r][c]`, the Hadamard form; it agrees with the matrix product
/// only for diagonal Jacobians and could not certify the coupled slab
/// systems the general FF stage needs.)
fn k_image<const N: usize>(
    q: &[Interval; N],
    m: &[f64; N],
    y: &[[f64; N]; N],
    f: &[Interval; N],
    j: &[[Interval; N]; N],
) -> [Interval; N] {
    array::from_fn(|r| {
        let center = interval_at(m.get(r).copied().unwrap_or(0.0));
        let y_row: [f64; N] = y.get(r).copied().unwrap_or([0.0; N]);
        let yf = y_row
            .iter()
            .zip(f.iter())
            .fold(interval_at(0.0), |acc, (yrc, fc)| {
                acc + interval_at(*yrc) * *fc
            });
        let dq = (0..N)
            .map(|c| {
                // (I - Y*J)[r][c] = delta(r,c) - sum_k y[r][k] * j[k][c]
                let delta = interval_at(if c == r { 1.0 } else { 0.0 });
                let inner = (0..N).fold(delta, |acc, k| {
                    let yrk = y.get(r).and_then(|row| row.get(k)).copied().unwrap_or(0.0);
                    let jkc = j
                        .get(k)
                        .and_then(|row| row.get(c))
                        .copied()
                        .unwrap_or(Interval::EMPTY);
                    acc - interval_at(yrk) * jkc
                });
                let qc = q.get(c).copied().unwrap_or(Interval::EMPTY);
                inner * (qc - interval_at(m.get(c).copied().unwrap_or(0.0)))
            })
            .fold(interval_at(0.0), |acc, term| acc + term);
        center - yf + dq
    })
}

/// Bisects `q` on its widest axis (ties toward the lowest index,
/// deterministically), spending one subdivision, and pushes the two halves.
/// `Err` is the `NumericallyUnresolved` refusal carrying spend: either the
/// budget ran out, or the widest axis has zero width (a degenerate point box
/// that cannot subdivide) — the operator refuses rather than spinning or
/// panicking (decision 3).
fn push_children<const N: usize>(
    q: &[Interval; N],
    stack: &mut Vec<[Interval; N]>,
    initial: &Budget,
    budget: &mut Budget,
) -> Result<(), Refusal> {
    let mut axis = 0usize;
    let mut width = f64::NEG_INFINITY;
    let mut a = 0.0;
    let mut b = 0.0;
    for (i, c) in q.iter().enumerate() {
        let w = c.sup() - c.inf();
        if w.total_cmp(&width).is_gt() || (w.total_cmp(&width).is_eq() && i < axis) {
            axis = i;
            width = w;
            (a, b) = (c.inf(), c.sup());
        }
    }
    if width == 0.0 {
        return Err(Refusal::NumericallyUnresolved {
            spent: spent(initial, budget),
            witness: UnresolvedWitness::KrawczykIndeterminate,
        });
    }
    // Split the widest axis at its midpoint as a convex combination, so the
    // halves hull back to the original box even where `0.5·(inf + sup)` would
    // overflow. A midpoint that rounds onto one of the axis's own edges means
    // the widest axis cannot be bisected in f64: the box is at resolution, so
    // refuse immediately — before spending — instead of pushing a zero-width
    // child plus the parent and looping until the budget is exhausted.
    let mid = 0.5 * a + 0.5 * b;
    if mid == a || mid == b {
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
    let mut lo = *q;
    let mut hi = *q;
    for (i, (l, h)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
        if i == axis {
            *l = Interval::try_from((a, mid)).unwrap_or(*l);
            *h = Interval::try_from((mid, b)).unwrap_or(*h);
        }
    }
    stack.push(lo);
    stack.push(hi);
    Ok(())
}

/// Spend since entry: the initial budget minus what remains (decision 2).
/// Never the REMAINING budget as `spent` — that hides exhaustion.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// The operator's certificate: interval method, remaining budget, unbounded
/// margin and modulus — the operator is unparameterized.
fn certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// A degenerate interval from a runtime `f64`. A non-finite `x` would make an
/// invalid interval, so it degrades to the empty interval rather than
/// panicking (H-1).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// `a·x² + b·x + c`, univariate — the transverse, tangential and no-root
    /// witnesses of the packet's validated scratch runs.
    struct Quad(f64, f64, f64);

    impl KrawczykSystem<1> for Quad {
        fn f_point(&self, x: &[f64; 1]) -> [Interval; 1] {
            let [x0] = *x;
            [interval_at(self.0 * x0 * x0 + self.1 * x0 + self.2)]
        }
        fn jacobian(&self, b: &[Interval; 1]) -> [[Interval; 1]; 1] {
            let [b0] = *b;
            [[interval_at(2.0 * self.0) * b0 + interval_at(self.1)]]
        }
        fn preconditioner(&self, x: &[f64; 1]) -> Option<[[f64; 1]; 1]> {
            let [x0] = *x;
            let d = 2.0 * self.0 * x0 + self.1;
            if d == 0.0 {
                None
            } else {
                Some([[1.0 / d]])
            }
        }
    }

    /// `A·x − w`, the nonsingular linear-system witness.
    struct Lin2([[f64; 2]; 2], [f64; 2]);

    impl KrawczykSystem<2> for Lin2 {
        fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
            let [x0, x1] = *x;
            let [[a00, a01], [a10, a11]] = self.0;
            let [w0, w1] = self.1;
            [
                interval_at(a00 * x0 + a01 * x1 - w0),
                interval_at(a10 * x0 + a11 * x1 - w1),
            ]
        }
        fn jacobian(&self, _b: &[Interval; 2]) -> [[Interval; 2]; 2] {
            let [[a00, a01], [a10, a11]] = self.0;
            [
                [interval_at(a00), interval_at(a01)],
                [interval_at(a10), interval_at(a11)],
            ]
        }
        fn preconditioner(&self, _x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
            let [[a00, a01], [a10, a11]] = self.0;
            let det = a00 * a11 - a01 * a10;
            if det == 0.0 {
                None
            } else {
                Some([[a11 / det, -a01 / det], [-a10 / det, a00 / det]])
            }
        }
    }

    /// The transversal sphere/cylinder slab witness in `(x, y)`:
    /// `f1 = x² + y² − 1`, `f2 = (x−3)² + y² + z0² − 9` at fixed `z0`. Its
    /// Jacobian `[[2x, 2y],[2(x−3), 2y]]` (determinant `12y`) is genuinely
    /// coupled, so only the matrix-product contraction can certify it.
    struct Coupled {
        z0: f64,
    }

    impl KrawczykSystem<2> for Coupled {
        fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
            let [x0, x1] = *x;
            [
                interval_at(x0 * x0 + x1 * x1 - 1.0),
                interval_at((x0 - 3.0) * (x0 - 3.0) + x1 * x1 + self.z0 * self.z0 - 9.0),
            ]
        }
        fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
            let [b0, b1] = *b;
            [
                [interval_at(2.0) * b0, interval_at(2.0) * b1],
                [
                    interval_at(2.0) * (b0 - interval_at(3.0)),
                    interval_at(2.0) * b1,
                ],
            ]
        }
        fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
            let [x0, x1] = *x;
            let det = 12.0 * x1;
            if det == 0.0 {
                None
            } else {
                // Exact inverse of `[[2x, 2y],[2(x−3), 2y]]`: `1/det·[[d,−b],[−c,a]]`.
                Some([
                    [2.0 * x1 / det, -2.0 * x1 / det],
                    [-2.0 * (x0 - 3.0) / det, 2.0 * x0 / det],
                ])
            }
        }
    }

    /// A genuinely diagonal 2×2 system `f1 = x² − 1`, `f2 = y² − 4`. The
    /// matrix-product contraction reduces to the Hadamard form exactly here, so
    /// certification is unchanged by the BG-NUM-003-CONTRACT fix.
    struct Diag2;

    impl KrawczykSystem<2> for Diag2 {
        fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
            let [x0, x1] = *x;
            [interval_at(x0 * x0 - 1.0), interval_at(x1 * x1 - 4.0)]
        }
        fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
            let [b0, b1] = *b;
            [
                [interval_at(2.0) * b0, interval_at(0.0)],
                [interval_at(0.0), interval_at(2.0) * b1],
            ]
        }
        fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
            let [x0, x1] = *x;
            if x0 == 0.0 || x1 == 0.0 {
                None
            } else {
                Some([[1.0 / (2.0 * x0), 0.0], [0.0, 1.0 / (2.0 * x1)]])
            }
        }
    }

    /// A closed interval (test-side: `super::interval_at` degrades to `EMPTY`,
    /// which would silently change the witnesses).
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap()
    }

    /// `L`: the nonzero root of the witness `x·(x − L)` (`Quad(1.0, -L, 0.0)`).
    /// H-3: a unit-magnitude root location in parameter units, not a
    /// model-space length.
    const ROOT_L: f64 = 1.0; // H-3: root L of Quad(1.0, -L, 0.0)

    /// `w`: the half-width of the centered root box `[L − w, L + w]`, and the
    /// right-edge offset of the left-edge box `[L, L + w]`.
    /// H-3: a dimensionless parameter-unit offset, not a model-space length.
    const ROOT_BOX_HALF_WIDTH: f64 = 4.0 * f64::EPSILON; // H-3: half-width w around the root

    #[test]
    fn transverse_quadratic_certifies_unique_one_shot() {
        let system = Quad(1.0, 0.0, -2.0);
        let start = [iv(1.0, 2.0)];
        let mut budget = Budget::new(4, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        assert_eq!(out.value, KrawczykProof::Unique);
        // Certification needs no subdivision, so the budget is untouched — the
        // one-shot claim, and the guard against the wrong exhaustion premise.
        assert_eq!(budget.subdiv, 4);
    }

    #[test]
    fn tangential_double_root_refuses_indeterminate() {
        let system = Quad(1.0, 0.0, 0.0);
        let start = [iv(-1.0, 1.0)];
        let mut budget = Budget::new(64, 0, 0);
        let err = krawczyk(&system, &start, &mut budget).unwrap_err();
        // The strict-interior rule never fires near the double root; every
        // subdivision is consumed before the operator gives up.
        assert!(matches!(
            err,
            Refusal::NumericallyUnresolved {
                spent,
                witness: UnresolvedWitness::KrawczykIndeterminate,
            } if spent.subdiv == 64
        ));
    }

    #[test]
    fn budget_exhaustion_carries_spend() {
        let system = Quad(1.0, 0.0, 0.0);
        let start = [iv(-1.0, 1.0)];
        let mut budget = Budget::new(3, 0, 0);
        let err = krawczyk(&system, &start, &mut budget).unwrap_err();
        // Only a case that actually bisects can exhaust; the tangential one
        // does, and the refusal reports what was spent, not what remains.
        assert!(matches!(
            err,
            Refusal::NumericallyUnresolved {
                spent,
                witness: UnresolvedWitness::KrawczykIndeterminate,
            } if spent.subdiv == 3
        ));
    }

    #[test]
    fn linear_system_certifies_one_shot() {
        let system = Lin2([[2.0, 1.0], [1.0, 3.0]], [5.0, 10.0]);
        let start = [iv(-10.0, 10.0), iv(-10.0, 10.0)];
        let mut budget = Budget::new(16, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        assert_eq!(out.value, KrawczykProof::Unique);
        // A nonsingular affine system certifies in one shot: subdiv untouched.
        assert_eq!(budget.subdiv, 16);
    }

    #[test]
    fn no_root_box_proves_no_root() {
        let system = Quad(1.0, 0.0, 1.0);
        let start = [iv(-2.0, 2.0)];
        let mut budget = Budget::new(1024, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        // The m = 0 midpoint has a None preconditioner (2x = 0); decision 4
        // bisects rather than refusing, so the search prunes to NoRoot. A
        // regression to refusal dies here.
        assert_eq!(out.value, KrawczykProof::NoRoot);
    }

    #[test]
    fn empty_input_refuses_empty() {
        let system = Quad(1.0, 0.0, -2.0);
        let start = [iv(1.0, 1.0).intersection(iv(2.0, 3.0))];
        let mut budget = Budget::new(4, 0, 0);
        let err = krawczyk(&system, &start, &mut budget).unwrap_err();
        assert!(matches!(err, Refusal::Empty));
    }

    #[test]
    fn unsplittable_box_refuses_without_burning_budget() {
        // x·(x − L): the root sits exactly on the start box's left edge, so
        // strict interior can never hold and the descent reaches a box of width
        // ~1 ulp whose float midpoint rounds onto an edge. The widest axis
        // cannot be bisected in f64, so push_children must refuse immediately
        // instead of re-pushing the parent until the budget is exhausted.
        let system = Quad(1.0, -ROOT_L, 0.0);
        let start = [iv(ROOT_L, ROOT_L + ROOT_BOX_HALF_WIDTH)];
        let mut budget = Budget::new(1024, 0, 0);
        let err = krawczyk(&system, &start, &mut budget).unwrap_err();
        assert!(matches!(
            err,
            Refusal::NumericallyUnresolved {
                spent,
                witness: UnresolvedWitness::KrawczykIndeterminate,
            } if spent.subdiv < 16
        ));
        // The spend was refused, not consumed: the caller's shared budget keeps
        // nearly everything for its other work.
        assert!(budget.subdiv > 1000);
    }

    #[test]
    fn centered_root_box_still_certifies() {
        // Same witness, but the root is strictly interior to `[L − w, L + w]`:
        // the operator certifies Unique one-shot with zero spend.
        let system = Quad(1.0, -ROOT_L, 0.0);
        let start = [iv(
            ROOT_L - ROOT_BOX_HALF_WIDTH,
            ROOT_L + ROOT_BOX_HALF_WIDTH,
        )];
        let mut budget = Budget::new(1024, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        assert_eq!(out.value, KrawczykProof::Unique);
        assert_eq!(budget.subdiv, 1024);
    }

    #[test]
    fn coupled_system_certifies_after_matrix_contraction() {
        // The transversal sphere/cylinder slab witness at fixed z0 = √2: the
        // crossing (1/2, −√3/2) satisfies f1 = 0.25 + 0.75 − 1 = 0 and
        // f2 = 6.25 + 0.75 + 2 − 9 = 0 to f64 rounding. With the exact 2×2
        // inverse preconditioner the MATRIX-PRODUCT contraction certifies
        // Unique over the box below; the Hadamard form cannot (pinned by
        // entrywise_form_would_not_have_certified).
        let system = Coupled { z0: 2.0_f64.sqrt() };
        let [cx, cy] = [0.5, -(3.0_f64.sqrt()) / 2.0];
        let width = 1e-2; // H-3: box width around the crossing, parameter units
        let hw = width / 2.0;
        let start = [iv(cx - hw, cx + hw), iv(cy - hw, cy + hw)];
        let mut budget = Budget::new(64, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        assert_eq!(out.value, KrawczykProof::Unique);
    }

    #[test]
    fn entrywise_form_would_not_have_certified() {
        // Reproduce the DELETED Hadamard contraction `d[r][c] = δ(r,c) −
        // y[r][c]·j[r][c]` in a local helper and show the coupled witness does
        // NOT satisfy strict interior containment — pinning the regression the
        // matrix product fixes.
        fn k_image_entrywise<const N: usize>(
            q: &[Interval; N],
            m: &[f64; N],
            y: &[[f64; N]; N],
            f: &[Interval; N],
            j: &[[Interval; N]; N],
        ) -> [Interval; N] {
            array::from_fn(|r| {
                let center = interval_at(m.get(r).copied().unwrap_or(0.0));
                let y_row: [f64; N] = y.get(r).copied().unwrap_or([0.0; N]);
                let j_row: [Interval; N] = j.get(r).copied().unwrap_or([Interval::EMPTY; N]);
                let yf = y_row
                    .iter()
                    .zip(f.iter())
                    .fold(interval_at(0.0), |acc, (yrc, fc)| {
                        acc + interval_at(*yrc) * *fc
                    });
                let dq = y_row
                    .iter()
                    .zip(j_row.iter())
                    .zip(q.iter())
                    .enumerate()
                    .fold(interval_at(0.0), |acc, (c, ((yrc, jrc), qc))| {
                        let delta = interval_at(if c == r { 1.0 } else { 0.0 });
                        let d = delta - interval_at(*yrc) * *jrc;
                        acc + d * (*qc - interval_at(m.get(c).copied().unwrap_or(0.0)))
                    });
                center - yf + dq
            })
        }
        let system = Coupled { z0: 2.0_f64.sqrt() };
        let [cx, cy] = [0.5, -(3.0_f64.sqrt()) / 2.0];
        let width = 1e-2; // H-3: box width around the crossing, parameter units
        let hw = width / 2.0;
        let [q0, q1] = [iv(cx - hw, cx + hw), iv(cy - hw, cy + hw)];
        let q = [q0, q1];
        let m = [0.5 * (q0.inf() + q0.sup()), 0.5 * (q1.inf() + q1.sup())];
        let f = system.f_point(&m);
        let j = system.jacobian(&q);
        let y = system.preconditioner(&m).unwrap();
        let k = k_image_entrywise(&q, &m, &y, &f, &j);
        let strict = k
            .iter()
            .zip(q.iter())
            .all(|(kv, qv)| !kv.is_empty() && kv.inf() > qv.inf() && kv.sup() < qv.sup());
        assert!(!strict);
    }

    #[test]
    fn diagonal_system_still_certifies() {
        // A genuinely diagonal 2×2 system: the matrix product and the Hadamard
        // form agree exactly, so Unique certification is unchanged by the fix.
        let system = Diag2;
        let [cx, cy] = [1.0, 2.0];
        let width = 1e-2; // H-3: box width around (1, 2), parameter units
        let hw = width / 2.0;
        let start = [iv(cx - hw, cx + hw), iv(cy - hw, cy + hw)];
        let mut budget = Budget::new(64, 0, 0);
        let out = krawczyk(&system, &start, &mut budget).unwrap();
        assert_eq!(out.value, KrawczykProof::Unique);
    }
}
