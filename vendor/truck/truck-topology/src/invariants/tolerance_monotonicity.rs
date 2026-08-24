//! BG-INV-107: tolerance monotonicity (§1.1 invariant 7).
//!
//! An entity's tolerance dominates its boundary's, and a preserved entity's
//! record never decreases across an operation. The listings below localise
//! violations; the entry points return the house `Contradictory` refusal on
//! any violation, so a caller can tell WHICH invariant failed and that the
//! input claims to be a realisation while the checker measured the opposite.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::entity_id::EntityId;
use crate::tolerance_store::EntityToleranceStore;
use truck_base::evidence::{
    Budget, Certificate, Certified, ContradictionWitness, Margin, Method, Modulus, Outcome, Prop,
    PropMap, Refusal, Truth,
};

/// Ids whose recorded value is not finite and nonnegative. Deserialisation
/// bypasses `raise`, so this is reachable for tampered input.
pub fn invalid_records(store: &EntityToleranceStore) -> Vec<EntityId> {
    let mut bad: Vec<EntityId> = store
        .iter()
        .filter(|(_, tol)| !tol.value.is_finite() || tol.value < 0.0)
        .map(|(id, _)| id.clone())
        .collect();
    bad.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    bad
}

/// `(sel, base)` pairs where BOTH are recorded and record(sel) >
/// record(base) — invariant 7's "entity τ ≥ boundary τ" over the
/// identity algebra's Selector paths. Only IMMEDIATE Sel bases are
/// compared; an unrecorded intermediate breaks the chain (missing is
/// not zero, so no constraint is invented).
pub fn boundary_violations(store: &EntityToleranceStore) -> Vec<(EntityId, EntityId)> {
    let mut out = Vec::new();
    for (sel, sel_tol) in store.iter() {
        let EntityId::Sel { base, .. } = sel else {
            continue;
        };
        let base: &EntityId = base;
        if let Some(base_tol) = store.get(base) {
            if sel_tol.value > base_tol.value {
                out.push((sel.clone(), base.clone()));
            }
        }
    }
    out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    out
}

/// Ids recorded in `before` whose record in `after` is strictly lower —
/// a preserved entity's tolerance decreased.
pub fn decreased_records(
    before: &EntityToleranceStore,
    after: &EntityToleranceStore,
) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (id, before_tol) in before.iter() {
        if let Some(after_tol) = after.get(id) {
            if after_tol.value < before_tol.value {
                out.push(id.clone());
            }
        }
    }
    out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    out
}

/// BG-INV-107 (single-store half): every recorded value is finite and
/// nonnegative, and no recorded Sel exceeds its recorded base.
pub fn check_store(store: &EntityToleranceStore) -> Outcome<()> {
    if !invalid_records(store).is_empty() || !boundary_violations(store).is_empty() {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::ToleranceMonotonicity,
            left: Truth::True,
            right: Truth::False,
        }));
    }
    let mut props = PropMap::new();
    props.set(Prop::ToleranceMonotonicity, Truth::True);
    Ok(Certified::new(
        (),
        Certificate {
            props,
            method: Method::None,
            budget_left: Budget::new(0, 0, 0),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// BG-INV-107 (transition half): `check_store(after)` AND no preserved
/// id decreased from `before` to `after`. Ids only in `after` are fresh;
/// ids only in `before` were deleted; neither is constrained.
pub fn check_transition(
    before: &EntityToleranceStore,
    after: &EntityToleranceStore,
) -> Outcome<()> {
    if !invalid_records(after).is_empty()
        || !boundary_violations(after).is_empty()
        || !decreased_records(before, after).is_empty()
    {
        return Err(Refusal::Contradictory(ContradictionWitness {
            prop: Prop::ToleranceMonotonicity,
            left: Truth::True,
            right: Truth::False,
        }));
    }
    let mut props = PropMap::new();
    props.set(Prop::ToleranceMonotonicity, Truth::True);
    Ok(Certified::new(
        (),
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
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use crate::entity_id::{End, EntityId, Op, OpKind, OpParams, Selector};

    #[test]
    fn boundary_monotonicity_flags_sel_above_base() {
        let face = EntityId::src(11);
        let wire0 = EntityId::sel(face.clone(), Selector::BoundaryWire(0));
        let edge1 = EntityId::sel(wire0.clone(), Selector::WireEdge(1));
        let mut store = EntityToleranceStore::new();
        assert!(store.raise(wire0.clone(), 1.0).is_ok());
        assert!(store.raise(edge1.clone(), 5.0).is_ok());
        assert_eq!(boundary_violations(&store), vec![(edge1, wire0)]);
        assert!(matches!(
            check_store(&store),
            Err(Refusal::Contradictory(ContradictionWitness { prop, .. }))
                if prop == Prop::ToleranceMonotonicity
        ));
    }

    #[test]
    fn boundary_monotonicity_accepts_chain_and_gap() {
        let face = EntityId::src(11);
        let wire0 = EntityId::sel(face.clone(), Selector::BoundaryWire(0));
        let edge1 = EntityId::sel(wire0.clone(), Selector::WireEdge(1));
        let vend = EntityId::sel(edge1.clone(), Selector::End(End::Front));

        let mut store = EntityToleranceStore::new();
        assert!(store.raise(face.clone(), 4.0).is_ok());
        assert!(store.raise(wire0.clone(), 3.0).is_ok());
        assert!(store.raise(edge1.clone(), 2.0).is_ok());
        assert!(store.raise(vend.clone(), 1.0).is_ok());
        let certified = check_store(&store).expect("monotone chain holds");
        assert_eq!(
            certified.cert.props.get(Prop::ToleranceMonotonicity),
            Truth::True
        );

        let mut store = EntityToleranceStore::new();
        assert!(store.raise(face.clone(), 4.0).is_ok());
        assert!(store.raise(vend.clone(), 9.0).is_ok());
        assert!(check_store(&store).is_ok());

        let mut store = EntityToleranceStore::new();
        assert!(store.raise(face.clone(), 3.0).is_ok());
        assert!(store.raise(wire0.clone(), 3.0).is_ok());
        assert!(check_store(&store).is_ok());
    }

    #[test]
    fn transition_flags_decrease() {
        let src7 = EntityId::src(7);
        let mut before = EntityToleranceStore::new();
        assert!(before.raise(src7.clone(), 5.0).is_ok());
        let mut after = EntityToleranceStore::new();
        assert!(after.raise(src7.clone(), 3.0).is_ok());
        assert_eq!(decreased_records(&before, &after), vec![src7]);
        assert!(matches!(
            check_transition(&before, &after),
            Err(Refusal::Contradictory(ContradictionWitness { prop, .. }))
                if prop == Prop::ToleranceMonotonicity
        ));
    }

    #[test]
    fn transition_accepts_raise_fresh_deleted() {
        let src7 = EntityId::src(7);
        let face = EntityId::src(11);
        let swept = Op {
            kind: OpKind::Sweep,
            params: OpParams::Scalar(2.5),
        }
        .output(&[EntityId::src(7)], 0);

        let mut before = EntityToleranceStore::new();
        assert!(before.raise(src7.clone(), 3.0).is_ok());
        assert!(before.raise(face.clone(), 2.0).is_ok());
        let mut after = EntityToleranceStore::new();
        assert!(after.raise(src7.clone(), 5.0).is_ok());
        assert!(after.raise(swept.clone(), 1.0).is_ok());
        assert!(check_transition(&before, &after).is_ok());

        let mut before = EntityToleranceStore::new();
        assert!(before.raise(src7.clone(), 3.0).is_ok());
        let mut after = EntityToleranceStore::new();
        assert!(after.raise(src7.clone(), 3.0).is_ok());
        assert!(check_transition(&before, &after).is_ok());
    }
}
