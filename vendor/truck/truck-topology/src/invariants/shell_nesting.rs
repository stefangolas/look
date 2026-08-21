//! BG-INV-108: shell nesting is a forest (§1.1 invariant 8; audit F-1).
//!
//! The containment order of a solid's boundary shell components must be a
//! **nesting forest** — antisymmetric (a cycle is a contradiction), and each
//! maximal component is one solid whose immediate children are its inner
//! (cavity) shells. Fixes F-1: `Solid::new(connected_components())` today
//! packs every component into one solid, declaring disjoint lumps to be
//! cavities.
//!
//! The inside query is not yet certified (the certified winding is
//! BG-NUM-004's, unwritten), so this checker is **pure**: it takes the
//! containment relation as an injected oracle and certifies the GRAPH — the
//! part that is topology, not geometry.

use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth, UnresolvedWitness,
};

/// The containment oracle: `contains(i, j)` answers whether component
/// `i`'s witness point lies strictly inside component `j`.
/// `Some(true)` / `Some(false)` are certified answers; `None` is
/// undecided. The production implementation is BG-NUM-004's certified
/// winding; tests inject hand-built answers.
pub type Contains = dyn Fn(usize, usize) -> Option<bool>;

/// BG-INV-108: shell nesting is a forest (§1.1 invariant 8, audit F-1).
///
/// Given `n` connected shell components and a containment oracle over
/// them, certifies the containment relation is a strict partial order
/// (a cycle — including the two-cycle of mutual containment — is
/// `Contradictory` with `Prop::ShellNesting`) and returns the solid
/// partition: one entry `(outer, inner_shells)` per SOLID — the
/// even-depth components are solids, each with its immediately
/// contained (odd-depth) components as inner shells. A component at
/// depth 2 — inside a cavity — is its own solid again (the solid ⊃
/// void ⊃ solid case yields two solids).
///
/// Any `None` from the oracle is `NumericallyUnresolved`
/// (`UncertifiedContainment`): an undecided pair cannot be classified
/// either way, and an honest refusal beats a guess. This checker is
/// pure graph logic — the geometry lives in the oracle.
///
/// ```
/// use truck_topology::invariants::shell_nesting::nesting_forest;
///
/// // One component nested inside another: a single solid with one
/// // inner (cavity) shell.
/// let contains = |i: usize, j: usize| match (i, j) {
///     (1, 0) => Some(true),
///     _ => Some(false),
/// };
/// let out = nesting_forest(2, &contains);
/// assert!(out.is_ok());
/// if let Ok(certified) = out {
///     assert_eq!(certified.value, vec![(0, vec![1])]);
/// }
/// ```
pub fn nesting_forest(n: usize, contains: &Contains) -> Outcome<Vec<(usize, Vec<usize>)>> {
    // Decision 2, step 1: query every ordered pair ONCE and cache. An
    // undecided answer refuses before any graph work — an undecided pair
    // cannot be classified either way.
    let mut table: Vec<Vec<Option<bool>>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(n);
        for j in 0..n {
            let answer = if i == j { Some(false) } else { contains(i, j) };
            row.push(answer);
        }
        table.push(row);
    }
    for row in &table {
        for answer in row {
            if answer.is_none() {
                return Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::UncertifiedContainment,
                });
            }
        }
    }

    // `inside(i, j)`: component i's witness point is certified strictly
    // inside component j (`contains(i, j) == Some(true)`).
    let inside = |i: usize, j: usize| -> bool {
        table
            .get(i)
            .and_then(|row| row.get(j))
            .copied()
            .flatten()
            .unwrap_or(false)
    };

    // Decision 2, step 2: cycle detection by peeling a topological order
    // (Kahn's algorithm without an in-degree ledger). A component is
    // peelable once every component inside it is peeled; a cycle leaves the
    // order short — the violation of decision 4.
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut changed = true;
    while changed && order.len() < n {
        changed = false;
        for i in 0..n {
            if order.contains(&i) {
                continue;
            }
            let blocked = (0..n).any(|j| j != i && !order.contains(&j) && inside(j, i));
            if !blocked {
                order.push(i);
                changed = true;
            }
        }
    }
    if order.len() < n {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::ShellNesting,
            left: Truth::True,
            right: Truth::False,
        }));
    }

    // Decision 2, step 3: nesting depths, OUTSIDE-IN — outermost 0, each
    // nesting level +1 (the tests' algebra; see disagreements: the packet's
    // literal `contains(j, i)` recurrence runs the other way). depth(i) =
    // 1 + max(depth(j)) over the j that CONTAIN i (i inside j); a component
    // contained by nothing has depth 0. Processed outermost-first, i.e. the
    // reverse of the peeling order.
    let mut depth = vec![0usize; n];
    for &i in order.iter().rev() {
        let mut d = 0usize;
        for j in 0..n {
            if j != i && inside(i, j) {
                let dj = depth.get(j).copied().unwrap_or(0);
                if dj >= d {
                    d = dj + 1;
                }
            }
        }
        if let Some(slot) = depth.get_mut(i) {
            *slot = d;
        }
    }

    // Decision 2, step 4: immediate containment (the transitive reduction).
    // j is i's immediate child iff i contains j and no k sits between
    // (`contains(i, k)` and `contains(k, j)` both true).
    let is_immediate_child = |i: usize, j: usize| -> bool {
        if !inside(j, i) {
            return false;
        }
        !(0..n).any(|k| k != i && k != j && inside(k, i) && inside(j, k))
    };

    // Decision 2, step 5: the partition. Every even-depth component c is a
    // solid `(c, immediate children of c)` — the children are odd-depth by
    // construction and are c's inner shells. Iterate in index order and
    // sort each children list for determinism.
    let mut partition: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        if depth.get(i).copied().unwrap_or(0) % 2 == 0 {
            let mut children: Vec<usize> = (0..n)
                .filter(|&j| j != i && is_immediate_child(i, j))
                .collect();
            children.sort_unstable();
            partition.push((i, children));
        }
    }

    // Decision 3: the holds certificate — the house structural pattern.
    // Pure graph logic, no arithmetic.
    let mut props = PropMap::new();
    props.set(Prop::ShellNesting, Truth::True);
    Ok(Certified::new(
        partition,
        Certificate {
            props,
            method: Method::None,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

#[cfg(test)]
mod tests {
    #![deny(clippy::unwrap_used)]
    // H-1: this module is test-only, exercising hand-built oracle closures
    // over component-index tables — no geometry, no kernel path — so it
    // denies unwrap like the production code above.
    use super::*;

    #[test]
    fn nesting_disjoint_components_are_two_solids() {
        let contains = |_i: usize, _j: usize| Some(false);
        let out = nesting_forest(3, &contains);
        assert!(out.is_ok());
        if let Ok(cert) = out {
            assert_eq!(cert.value.len(), 3);
            assert_eq!(cert.value, vec![(0, vec![]), (1, vec![]), (2, vec![])]);
        }
    }

    #[test]
    fn nesting_nested_component_is_an_inner_shell() {
        let contains = |i: usize, j: usize| match (i, j) {
            (1, 0) => Some(true),
            (0, 1) => Some(false),
            _ => Some(false),
        };
        let out = nesting_forest(2, &contains);
        assert!(out.is_ok());
        if let Ok(cert) = out {
            assert_eq!(cert.value, vec![(0, vec![1])]);
        }
    }

    #[test]
    fn nesting_three_levels_yield_two_solids() {
        // 2 ⊂ 1 ⊂ 0; the oracle is transitive (contains(2, 0) too).
        let contains = |i: usize, j: usize| match (i, j) {
            (1, 0) | (2, 0) | (2, 1) => Some(true),
            _ => Some(false),
        };
        let out = nesting_forest(3, &contains);
        assert!(out.is_ok());
        if let Ok(cert) = out {
            assert_eq!(cert.value, vec![(0, vec![1]), (2, vec![])]);
        }
    }

    #[test]
    fn nesting_containment_cycle_is_contradictory() {
        // The two-cycle of mutual containment.
        let mutual = |i: usize, j: usize| match (i, j) {
            (0, 1) | (1, 0) => Some(true),
            _ => Some(false),
        };
        let out = nesting_forest(2, &mutual);
        assert!(matches!(out, Err(Refusal::Contradictory(_))));
        if let Err(Refusal::Contradictory(w)) = out {
            assert_eq!(w.prop, Prop::ShellNesting);
        }

        // A 3-cycle (0→1→2→0): the same refusal.
        let three_cycle = |i: usize, j: usize| match (i, j) {
            (0, 1) | (1, 2) | (2, 0) => Some(true),
            _ => Some(false),
        };
        let out = nesting_forest(3, &three_cycle);
        assert!(matches!(out, Err(Refusal::Contradictory(_))));
        if let Err(Refusal::Contradictory(w)) = out {
            assert_eq!(w.prop, Prop::ShellNesting);
        }
    }

    #[test]
    fn nesting_undecided_pair_is_unresolved() {
        let contains = |i: usize, j: usize| match (i, j) {
            (1, 0) => None,
            _ => Some(false),
        };
        let out = nesting_forest(2, &contains);
        assert!(matches!(out, Err(Refusal::NumericallyUnresolved { .. })));
        if let Err(Refusal::NumericallyUnresolved { witness, .. }) = out {
            assert_eq!(witness, UnresolvedWitness::UncertifiedContainment);
        }
    }

    #[test]
    fn nesting_antiparallel_pair_is_nested() {
        // Only BOTH-true is a cycle; the antiparallel pair is a legal
        // containment and partitions like the plain nested pair.
        let contains = |i: usize, j: usize| match (i, j) {
            (1, 0) => Some(true),
            (0, 1) => Some(false),
            _ => Some(false),
        };
        let out = nesting_forest(2, &contains);
        assert!(out.is_ok());
        if let Ok(cert) = out {
            assert_eq!(cert.value, vec![(0, vec![1])]);
        }
    }
}
