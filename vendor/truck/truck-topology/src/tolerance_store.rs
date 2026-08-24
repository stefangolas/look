//! BG-TOL-003: per-entity tolerance as sidecar state keyed by [`EntityId`].
//!
//! `truck-topology` entities stay immutable and carry no tolerance field; the
//! store is pure data beside `entity_id.rs`. Updates are raise-only (max), so
//! temporal monotonicity is a property of the type. A missing record means "no
//! entity-specific uncertainty recorded", never "τ = 0".

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::collections::HashMap;

use crate::entity_id::EntityId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use truck_base::evidence::{
    Budget, Certificate, Certified, EnvelopeCase, Margin, Method, Modulus, Outcome, PropMap,
    Refusal,
};

/// One per-entity tolerance record: a length-valued upper bound on the
/// accumulated geometric uncertainty associated with that entity. NOT
/// "the tolerance all predicates use" — combining this with ToleranceCtx
/// policy is a deliberate later decision, not a default.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityTolerance {
    /// The bound's value. Finite and nonnegative whenever it entered the
    /// store through `raise`; deserialisation bypasses `raise`, so the
    /// CHECKER re-validates (invariants::tolerance_monotonicity).
    pub value: f64,
}

/// BG-TOL-003 storage: per-entity tolerance as sidecar state keyed by
/// `EntityId`. Topology entities carry no tolerance field; updates are
/// raise-only (max), which makes temporal monotonicity a property of the
/// type. A missing record means "no entity-specific uncertainty
/// recorded", never "τ = 0".
///
/// ```
/// let mut store = truck_topology::tolerance_store::EntityToleranceStore::new();
/// let src = truck_topology::EntityId::src(7);
/// assert!(store.raise(src.clone(), 3.0).is_ok());
/// assert!(store.raise(src.clone(), 1.0).is_ok());
/// assert_eq!(
///     store.get(&src),
///     Some(truck_topology::tolerance_store::EntityTolerance { value: 3.0 })
/// );
/// assert!(truck_topology::invariants::tolerance_monotonicity::check_store(&store).is_ok());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntityToleranceStore {
    values: HashMap<EntityId, EntityTolerance>,
}

impl EntityToleranceStore {
    /// An empty store: every id reads `None`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The record for `id`, or `None` when no entity-specific
    /// uncertainty is recorded. `None` is NOT zero.
    pub fn get(&self, id: &EntityId) -> Option<EntityTolerance> {
        self.values.get(id).copied()
    }

    /// All records in arbitrary order. The monotonicity checker enumerates
    /// the store through this; the backing map stays private.
    pub fn iter(&self) -> impl Iterator<Item = (&EntityId, &EntityTolerance)> {
        self.values.iter()
    }

    // raise: decision 1b below
    /// Raise `id`'s record to `max(old, candidate)`, inserting when absent
    /// (this is also the initial-assignment route for construction/import).
    /// Refuses a non-finite or negative candidate with the typed refusal
    /// `ToleranceCtx::new` uses for the same class of invalid input — the
    /// landed precedent — and leaves the store unchanged on refusal. Never
    /// panics (H-1).
    pub fn raise(&mut self, id: EntityId, candidate: f64) -> Outcome<()> {
        if !candidate.is_finite() || candidate < 0.0 {
            return Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate));
        }
        match self.values.get_mut(&id) {
            Some(record) => {
                if candidate > record.value {
                    record.value = candidate;
                }
            }
            None => {
                self.values.insert(id, EntityTolerance { value: candidate });
            }
        }
        Ok(Certified::new(
            (),
            Certificate {
                props: PropMap::new(),
                method: Method::None,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ))
    }
}

impl Serialize for EntityToleranceStore {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.values.iter())
    }
}

impl<'de> Deserialize<'de> for EntityToleranceStore {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = Vec::<(EntityId, EntityTolerance)>::deserialize(deserializer)?;
        let mut values = HashMap::with_capacity(entries.len());
        for (id, tol) in entries {
            values.insert(id, tol); // duplicate ids: last wins, documented
        }
        Ok(Self { values })
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use crate::entity_id::{End, EntityId, Op, OpKind, OpParams, Selector};
    use crate::invariants::tolerance_monotonicity::{check_store, invalid_records};
    use truck_base::evidence::{ContradictionWitness, Prop};

    #[test]
    fn raise_is_monotone_and_idempotent() {
        let src7 = EntityId::src(7);
        let face = EntityId::src(11);
        let mut store = EntityToleranceStore::new();
        assert!(store.raise(src7.clone(), 3.0).is_ok());
        assert_eq!(store.get(&src7), Some(EntityTolerance { value: 3.0 }));
        assert!(store.raise(src7.clone(), 1.0).is_ok());
        assert_eq!(store.get(&src7), Some(EntityTolerance { value: 3.0 }));
        assert!(store.raise(src7.clone(), 5.0).is_ok());
        assert_eq!(store.get(&src7), Some(EntityTolerance { value: 5.0 }));
        assert!(store.raise(face.clone(), 2.0).is_ok());
        assert_eq!(store.get(&src7), Some(EntityTolerance { value: 5.0 }));
        assert_eq!(store.get(&face), Some(EntityTolerance { value: 2.0 }));
    }

    #[test]
    fn raise_refuses_invalid_candidates() {
        let src7 = EntityId::src(7);
        let mut store = EntityToleranceStore::new();
        assert!(matches!(
            store.raise(src7.clone(), -1.0),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
        assert!(matches!(
            store.raise(src7.clone(), f64::NAN),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
        assert!(matches!(
            store.raise(src7.clone(), f64::INFINITY),
            Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))
        ));
        assert_eq!(store.get(&src7), None);
        assert!(store.raise(src7.clone(), 0.0).is_ok());
        assert_eq!(store.get(&src7), Some(EntityTolerance { value: 0.0 }));
    }

    #[test]
    fn missing_record_is_none_not_zero() {
        let swept = Op {
            kind: OpKind::Sweep,
            params: OpParams::Scalar(2.5),
        }
        .output(&[EntityId::src(7)], 0);
        let mut store = EntityToleranceStore::new();
        assert_eq!(store.get(&swept), None);
        assert!(store.raise(swept.clone(), 0.0).is_ok());
        assert_eq!(store.get(&swept), Some(EntityTolerance { value: 0.0 }));
    }

    #[test]
    fn serde_round_trip_preserves_records() {
        let src7 = EntityId::src(7);
        let face = EntityId::src(11);
        let wire0 = EntityId::sel(face.clone(), Selector::BoundaryWire(0));
        let edge1 = EntityId::sel(wire0.clone(), Selector::WireEdge(1));
        let vend = EntityId::sel(edge1.clone(), Selector::End(End::Front));
        let mut store = EntityToleranceStore::new();
        assert!(store.raise(src7.clone(), 5.0).is_ok());
        assert!(store.raise(face.clone(), 1.0).is_ok());
        assert!(store.raise(wire0.clone(), 1.0).is_ok());
        assert!(store.raise(edge1.clone(), 0.5).is_ok());
        assert!(store.raise(vend.clone(), 0.25).is_ok());
        let text = serde_json::to_string(&store).expect("store serialises");
        let back: EntityToleranceStore = serde_json::from_str(&text).expect("store deserialises");
        assert_eq!(store, back);
        assert!(check_store(&back).is_ok());
    }

    #[test]
    fn checker_flags_invalid_deserialized_value() {
        let src7 = EntityId::src(7);
        let tampered = r#"[[{"Src":7},{"value":-1.0}]]"#;
        let store: EntityToleranceStore = serde_json::from_str(tampered).expect("plain JSON pairs");
        assert_eq!(invalid_records(&store), vec![src7]);
        assert!(matches!(
            check_store(&store),
            Err(Refusal::Contradictory(ContradictionWitness { prop, .. }))
                if prop == Prop::ToleranceMonotonicity
        ));
    }
}
