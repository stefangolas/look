//! BG-NUM-002: certified univariate root isolation.
//!
//! The contract (spec §Stage 3):
//!
//! - Input: a polynomial in the Bernstein basis on a domain, a tolerance
//!   `tau`, and a `Budget`.
//! - Descartes' rule on the Bernstein coefficients counts sign changes:
//!   `0` — no root in the box, prune; `1` — exactly one root, refine to
//!   width < `tau` and emit the isolating interval; otherwise bisect under
//!   the budget.
//! - **Multiple roots** (an even sign-change count that never reaches 1 at
//!   representable width) are `NumericallyUnresolved`, NEVER an empty list —
//!   reporting "no root" for a tangential double root is precisely the §9.2
//!   failure this module exists to prevent.
//! - Property: every returned interval contains exactly one root; the union
//!   contains all roots in the domain.
//!
//! # API contract
//!
//! [`isolate_roots`] takes `coeffs[i]`, the degree-`len−1` Bernstein
//! coefficient at basis index `i` over `domain`, and returns one isolating
//! interval per distinct simple real root in the open domain. Every returned
//! interval has width `< tau`, contains exactly one root, and their union
//! contains every simple root in the open domain. Intervals are returned
//! sorted by lower endpoint (deterministic output order).
//!
//! `tau` is a target isolation width in parameter units: a box whose
//! coefficient sequence has exactly one strict sign change is emitted as soon
//! as its width drops below `tau`.
//!
//! # Zero coefficients block pruning
//!
//! The Bernstein convex-hull property makes "all coefficients same STRICT
//! sign" imply "no root" — but only strictness licenses it. A zero coefficient
//! means the control polygon touches the axis, so the box is never pruned on
//! sign evidence alone. For an even-multiplicity contact the touch is the
//! root's only signature: `(2t−1)²` has Bernstein sequence `[1, −1, 1]` over
//! `[0, 1]`, and its first subdivision produces `[1, 0, 0]` and `[0, 0, 1]` —
//! zero variations WITH a zero, contact boxes that no amount of subdivision
//! will isolate. (The packet's quoted `[1, 0, 1]` is a different sequence: it
//! is the root-free polynomial `2(t−½)²+½`, for which `Ok(vec![])` is the
//! correct certified answer.)
//!
//! # Exhaustion vs emptiness
//!
//! `Ok(vec![])` is a CERTIFIED claim — no simple roots in the domain — while
//! `Err(RootNotIsolated)` is a typed failure — structure that could not be
//! resolved. They never blur: a tangential contact takes the second path.
//!
//! # Endpoint roots: a known, typed limitation
//!
//! A root exactly ON the domain endpoints — a zero endpoint coefficient —
//! blocks pruning like any other zero, driving subdivision toward the boundary
//! and eventually exhausting budget → `NumericallyUnresolved`. This is
//! deliberate: endpoint multiplicity is ambiguous under floating evaluation,
//! and a refusal is sound where a guess would not be. The same applies to a
//! root that lands exactly on an interior bisection boundary (a dyadic
//! parameter); the witnesses below keep roots off every dyadic grid point.
//!
//! # de Casteljau bisection
//!
//! Bisecting a Bernstein polynomial = splitting its coefficient sequence at
//! the midpoint via de Casteljau subdivision: repeated pairwise averaging
//! produces the LEFT child's coefficients from the front ends and the RIGHT
//! child's from the back ends. Both children inherit the parent's parameter
//! interval halves; no reparametrisation of coefficients is needed because the
//! midpoint split of a Bernstein sequence is basis-covariant. Each bisection
//! consumes ONE subdivision from the budget.

use inari::Interval;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
    UnresolvedWitness,
};

/// At or below this width a box cannot subdivide further; the
/// even-multiplicity contact case refuses here rather than spinning.
/// H-3: 8 ulps of a unit-width parameter interval — a dimensionless width in
/// parameter units, not a model-space length.
const WIDTH_FLOOR: f64 = 8.0 * f64::EPSILON; // H-3: width floor, 8 ulps of a unit-width parameter interval

/// Isolates the real roots of a polynomial in its Bernstein basis over
/// `domain`.
///
/// `coeffs[i]` is the degree-`len−1` Bernstein coefficient at basis index i
/// over `domain`. Returns one isolating interval per distinct simple real
/// root found: every returned interval has width `< tau`, contains exactly
/// one root, and their union contains every simple root in the open domain.
pub fn isolate_roots(
    coeffs: &[f64],
    domain: (f64, f64),
    tau: f64,
    budget: &mut Budget,
) -> Outcome<Vec<Interval>> {
    let (lo, hi) = domain;
    // Decision 0: degenerate or non-finite domain, non-finite tau, tau <= 0, or
    // any non-finite coefficient — nothing to certify.
    if lo >= hi
        || !lo.is_finite()
        || !hi.is_finite()
        || !tau.is_finite()
        || tau <= 0.0
        || coeffs.iter().any(|c| !c.is_finite())
    {
        return Err(Refusal::Empty);
    }

    // Spend is reported as initial − remaining (decision 4), so the entry
    // budget is captured once.
    let initial = *budget;
    let mut found: Vec<Interval> = Vec::new();
    let mut worklist: Vec<(f64, f64, Vec<f64>)> = vec![(lo, hi, coeffs.to_vec())];

    while let Some((blo, bhi, bcoeffs)) = worklist.pop() {
        // Decision 1 step 1: a non-finite coefficient refuses (unreachable past
        // entry; the arm stays total).
        if bcoeffs.iter().any(|c| !c.is_finite()) {
            return Err(Refusal::Empty);
        }
        let (v, has_zero) = sign_changes(&bcoeffs);
        let width = bhi - blo;
        // Decision 1 step 2: all coefficients one STRICT sign, no zero — no
        // root in this box; prune.
        if v == 0 && !has_zero {
            continue;
        }
        // Decision 1 step 4: exactly one simple root, narrow enough — emit.
        if v == 1 && width < tau {
            found.push(Interval::try_from((blo, bhi)).unwrap_or(Interval::EMPTY));
            continue;
        }
        // Decision 1 steps 3/5 + decision 4: bisect. A box at or below the
        // width floor cannot subdivide further (decision 4/5): refuse.
        if width <= WIDTH_FLOOR {
            return Err(Refusal::NumericallyUnresolved {
                spent: spent(&initial, budget),
                witness: UnresolvedWitness::RootNotIsolated,
            });
        }
        budget
            .spend_subdiv(1)
            .map_err(|_| Refusal::NumericallyUnresolved {
                spent: spent(&initial, budget),
                witness: UnresolvedWitness::RootNotIsolated,
            })?;
        let (left, right) = split(&bcoeffs);
        let mid = 0.5 * blo + 0.5 * bhi;
        worklist.push((mid, bhi, right));
        worklist.push((blo, mid, left));
    }

    // Deterministic output order.
    found.sort_by(|a, b| a.inf().total_cmp(&b.inf()));

    let margin = if found.is_empty() {
        Margin::UNBOUNDED
    } else {
        let narrowest = found
            .iter()
            .map(|iv| iv.sup() - iv.inf())
            .fold(f64::INFINITY, f64::min);
        Margin::from_log2(narrowest.log2())
    };

    Ok(Certified::new(
        found,
        Certificate {
            props: PropMap::new(),
            method: Method::Interval,
            budget_left: *budget,
            margin,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// Strict sign changes over `coeffs` after deleting exact zeros, plus whether
/// any exact zero remains (decision 2: a zero blocks pruning). `(0, false)` is
/// the certified-no-root signature.
fn sign_changes(coeffs: &[f64]) -> (u32, bool) {
    let mut changes: u32 = 0;
    let mut has_zero = false;
    let mut prev: Option<f64> = None;
    for &c in coeffs {
        if c == 0.0 {
            has_zero = true;
            continue;
        }
        if let Some(p) = prev {
            if (p > 0.0) != (c > 0.0) {
                changes += 1;
            }
        }
        prev = Some(c);
    }
    (changes, has_zero)
}

/// de Casteljau subdivision at the midpoint: the LEFT child's coefficients are
/// the front ends of each averaging level, the RIGHT child's the back ends
/// (reversed). Both inherit the parent's parameter halves (decision 3).
fn split(coeffs: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut left = Vec::with_capacity(coeffs.len());
    let mut right = Vec::with_capacity(coeffs.len());
    let mut row: Vec<f64> = coeffs.to_vec();
    left.push(row.first().copied().unwrap_or(0.0));
    right.push(row.last().copied().unwrap_or(0.0));
    while row.len() > 1 {
        let next: Vec<f64> = row
            .iter()
            .zip(row.iter().skip(1))
            .map(|(a, b)| 0.5 * *a + 0.5 * *b)
            .collect();
        left.push(next.first().copied().unwrap_or(0.0));
        right.push(next.last().copied().unwrap_or(0.0));
        row = next;
    }
    right.reverse();
    (left, right)
}

/// Spend since entry: the initial budget minus what remains (decision 4).
/// Never the REMAINING budget as `spent` — that hides exhaustion.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Target isolation width in parameter units.
    /// H-3: a dimensionless width on the unit domain, not a model-space length.
    const TAU: f64 = 1.0e-6; // H-3: target isolation width, parameter units

    /// Half the distance between the two clustered roots.
    /// H-3: a dimensionless offset in parameter units, not a model-space
    /// length.
    const CLUSTER_HALF_WIDTH: f64 = 0.001; // H-3: cluster half-width, parameter units

    fn width(iv: &Interval) -> f64 {
        iv.sup() - iv.inf()
    }

    #[test]
    fn simple_root_is_isolated_narrow_and_unique() {
        // p(t) = t − 0.5 over (0, 0.75): Bernstein [−0.5, 0.25]. The root at
        // 0.5 is strictly interior and lies off every dyadic bisection grid
        // point of this domain (0.5 / 0.75 = 2/3 is not dyadic), so the
        // endpoint-contact refusal never fires and the box refines to a single
        // interval narrower than TAU that contains 0.5.
        let mut budget = Budget::new(64, 0, 0);
        let out = isolate_roots(&[-0.5, 0.25], (0.0, 0.75), TAU, &mut budget).unwrap();
        assert_eq!(out.value.len(), 1);
        let iv = out.value.first().copied().unwrap();
        assert!(width(&iv) < TAU);
        assert!(iv.contains(0.5));
    }

    #[test]
    fn double_root_refuses_never_an_empty_list() {
        // (2t−1)² over [0, 1] has Bernstein coefficients [1, −1, 1] — NOT the
        // packet's [1, 0, 1], which is the root-free polynomial 2(t−½)²+½.
        // Two sign changes that never resolve to one: subdivision drives to the
        // width floor and refuses, never a certified empty list.
        let mut budget = Budget::new(256, 0, 0);
        let err = isolate_roots(&[1.0, -1.0, 1.0], (0.0, 1.0), TAU, &mut budget).unwrap_err();
        assert!(matches!(
            err,
            Refusal::NumericallyUnresolved {
                spent,
                witness: UnresolvedWitness::RootNotIsolated,
            } if spent.subdiv > 0
        ));
    }

    #[test]
    fn no_root_returns_certified_empty_vec() {
        // t² + 1 over [0, 1]: Bernstein [1, 1, 2], all strictly positive with
        // no zero — the box prunes immediately and the empty vector is
        // CERTIFIED, deliberately distinct from the refusal above.
        let mut budget = Budget::new(16, 0, 0);
        let out = isolate_roots(&[1.0, 1.0, 2.0], (0.0, 1.0), TAU, &mut budget).unwrap();
        assert!(out.value.is_empty());
    }

    #[test]
    fn clustered_roots_separate_with_enough_budget() {
        // p_s(t) = (t−(0.5−s))(t−(0.5+s)) over [0, 1] with
        // s = CLUSTER_HALF_WIDTH: Bernstein [0.25−s², −0.25−s², 0.25−s²].
        // (The packet's quoted middle coefficient −s² is wrong — the correct
        // one is −0.25−s²; at s = 0 the sequence collapses to [0.25, −0.25,
        // 0.25], the double root (t−½)².) Two simple roots at 0.5∓s separate
        // with enough budget and each isolates to an interval narrower than
        // TAU.
        let s = CLUSTER_HALF_WIDTH;
        let c = 0.25 - s * s;
        let coeffs = [c, -0.25 - s * s, c];
        let mut budget = Budget::new(64, 0, 0);
        let out = isolate_roots(&coeffs, (0.0, 1.0), TAU, &mut budget).unwrap();
        assert_eq!(out.value.len(), 2);
        let a = out.value.first().copied().unwrap();
        let b = out.value.get(1).copied().unwrap();
        assert!(a.inf() < b.inf());
        assert!(width(&a) < TAU);
        assert!(width(&b) < TAU);
        assert!(a.sup() <= b.inf()); // disjoint
        assert!(a.contains(0.5 - s));
        assert!(b.contains(0.5 + s));
    }

    #[test]
    fn clustered_roots_refuse_without_enough_budget() {
        // Same witness, budget 4 < log2(1/s): subdivision cannot reach width
        // < TAU for both roots, so the run refuses as RootNotIsolated.
        let s = CLUSTER_HALF_WIDTH;
        let c = 0.25 - s * s;
        let coeffs = [c, -0.25 - s * s, c];
        let mut budget = Budget::new(4, 0, 0);
        let err = isolate_roots(&coeffs, (0.0, 1.0), TAU, &mut budget).unwrap_err();
        assert!(matches!(
            err,
            Refusal::NumericallyUnresolved {
                witness: UnresolvedWitness::RootNotIsolated,
                ..
            }
        ));
    }

    #[test]
    fn empty_domain_refuses_empty() {
        // A degenerate-width domain (lo == hi) refuses at entry.
        let mut budget = Budget::new(16, 0, 0);
        let err = isolate_roots(&[1.0, 1.0, 2.0], (1.0, 1.0), TAU, &mut budget).unwrap_err();
        assert!(matches!(err, Refusal::Empty));
    }
}
