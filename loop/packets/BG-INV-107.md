# WORK PACKET BG-INV-107 — per-entity tolerance store + invariant checker 7: tolerance monotonicity

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-107","status":"DONE","contracts":["BG-TOL-003"],
 "tests_added":9,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-107
contract:    [BG-TOL-003]
class:       design
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/tolerance_store.rs
  - vendor/truck/truck-topology/src/invariants/tolerance_monotonicity.rs
  - vendor/truck/truck-topology/src/lib.rs
  - vendor/truck/truck-topology/src/invariants/mod.rs
read_allow:
  - vendor/truck/truck-topology/src/entity_id.rs
  - vendor/truck/truck-topology/src/invariants/coedge_pairing.rs
  - vendor/truck/truck-topology/Cargo.toml
  - vendor/truck/truck-base/src/evidence.rs
  - vendor/truck/truck-base/src/tolerance.rs
tests_required:
  - raise_is_monotone_and_idempotent
  - raise_refuses_invalid_candidates
  - missing_record_is_none_not_zero
  - boundary_monotonicity_flags_sel_above_base
  - boundary_monotonicity_accepts_chain_and_gap
  - transition_flags_decrease
  - transition_accepts_raise_fresh_deleted
  - serde_round_trip_preserves_records
  - checker_flags_invalid_deserialized_value
budget:      {turns: 35, ctx_tokens: 95000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub mod entity_id' vendor/truck/truck-topology/src/lib.rs"}
  - {id: A2, expect: 8, cmd: "grep -c 'pub mod' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'ToleranceMonotonicity' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub fn src' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'pub fn sel' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn output' vendor/truck/truck-topology/src/entity_id.rs"}
  - {id: A7, expect: 0, cmd: "grep -c 'tolerance_store\\|tolerance_monotonicity' vendor/truck/truck-topology/src/lib.rs"}
```

(A7 pins BOTH new modules as not yet declared — `grep -c` exits 1 on zero
matches, which IS the expected count.)

## Problem

§1.1 invariant 7 / BG-TOL-003: tolerance must behave monotonically — an
entity's tolerance dominates its boundary's, and a preserved entity's record
never decreases across an operation. Nothing in the tree can state this,
because no per-entity tolerance storage exists (the landed BG-TOL-001 waves
migrated predicates only). The owner decision of 2026-08-24 settled the
architecture: per-entity tolerance is **sidecar state keyed by `EntityId`** —
`truck-topology` entities stay immutable and carry no tolerance field; the
store is pure data beside `entity_id.rs`; updates are **raise-only** (max).
This packet lands the store and the checker as pure modules. **Wiring the
store into operations — who raises, when — is explicitly out of scope**
(Stage B / CE-005 territory): there is no caller to find because there is
deliberately no caller yet.

`EntityId`, `Op`, `OpKind`, `OpParams`, `Selector`, `End` already exist in
`truck-topology/src/entity_id.rs` — read that file first; the store keys on
`EntityId` and the checker walks `Sel` arms. `Prop::ToleranceMonotonicity`
already exists in `truck-base/src/evidence.rs` (anchor A3) — no cross-crate
edit is needed or allowed.

## Decisions already made for you

0. **Module declarations, exactly:**
   - `lib.rs`: `pub mod tolerance_store;` between `mod solid;` and
     `mod vertex;` (alphabetical — a misplaced `pub mod` trips
     `reorder_modules` under `cargo fmt --check`).
   - `invariants/mod.rs`: `pub mod tolerance_monotonicity;` between
     `pub mod shell_nesting;` and `pub mod vertex_link;`.
   - **Both new files open with the house deny block** (GATE-1 fails a new
     `vendor/truck/**` module without it):

     ```rust
     #![deny(
         clippy::unwrap_used,
         clippy::expect_used,
         clippy::panic,
         clippy::todo,
         clippy::unimplemented,
         clippy::indexing_slicing
     )]
     ```

1. **The store, verbatim** — `truck-topology/src/tolerance_store.rs`:

   ```rust
   use std::collections::HashMap;
   use crate::entity_id::EntityId;
   use serde::{Deserialize, Deserializer, Serialize, Serializer};
   use truck_base::evidence::{EnvelopeCase, Outcome, Refusal};

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
   #[derive(Clone, Debug, Default, PartialEq)]
   pub struct EntityToleranceStore {
       values: HashMap<EntityId, EntityTolerance>,
   }

   impl EntityToleranceStore {
       /// An empty store: every id reads `None`.
       pub fn new() -> Self { Self::default() }

       /// The record for `id`, or `None` when no entity-specific
       /// uncertainty is recorded. `None` is NOT zero.
       pub fn get(&self, id: &EntityId) -> Option<EntityTolerance> {
           self.values.get(id).copied()
       }

       // raise: decision 1b below
   }
   ```

   **1b. The `raise` method, exactly** (also on `EntityToleranceStore`):

   ```rust
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
       Ok(())
   }
   ```

   Note `candidate > record.value` (NOT `>=`): an equal raise is a no-op, and
   the strict form keeps a record bit-identical when nothing changes.

2. **Serde is a pairs-sequence, NOT a JSON object** — this is a design fact,
   do not re-derive it: `serde_json` requires map keys to be strings, and
   `EntityId` serializes as `{"Src":7}` / `{"Sel":{...}}` — a map — so a
   derived `HashMap<EntityId, _>` Serialize FAILS at runtime with
   "key must be a string". The store therefore serializes as a sequence of
   `(EntityId, EntityTolerance)` pairs (order-insensitive on the way back;
   a duplicate id in the sequence is last-wins):

   ```rust
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
   ```

   `serde` (with `derive`) and `serde_json` (dev) are already declared in
   `truck-topology/Cargo.toml` — **do not touch the manifest**.

3. **The checker, verbatim** — `invariants/tolerance_monotonicity.rs`.
   Three localising listings plus two entry points; the listings are
   deterministic (sort each by the `Debug` string of the offender):

   ```rust
   /// Ids whose recorded value is not finite and nonnegative. Deserialisation
   /// bypasses `raise`, so this is reachable for tampered input.
   pub fn invalid_records(store: &EntityToleranceStore) -> Vec<EntityId>;

   /// `(sel, base)` pairs where BOTH are recorded and record(sel) >
   /// record(base) — invariant 7's "entity τ ≥ boundary τ" over the
   /// identity algebra's Selector paths. Only IMMEDIATE Sel bases are
   /// compared; an unrecorded intermediate breaks the chain (missing is
   /// not zero, so no constraint is invented).
   pub fn boundary_violations(store: &EntityToleranceStore) -> Vec<(EntityId, EntityId)>;

   /// Ids recorded in `before` whose record in `after` is strictly lower —
   /// a preserved entity's tolerance decreased.
   pub fn decreased_records(
       before: &EntityToleranceStore,
       after: &EntityToleranceStore,
   ) -> Vec<EntityId>;

   /// BG-INV-107 (single-store half): every recorded value is finite and
   /// nonnegative, and no recorded Sel exceeds its recorded base.
   pub fn check_store(store: &EntityToleranceStore) -> Outcome<()>;

   /// BG-INV-107 (transition half): `check_store(after)` AND no preserved
   /// id decreased from `before` to `after`. Ids only in `after` are fresh;
   /// ids only in `before` were deleted; neither is constrained.
   pub fn check_transition(
       before: &EntityToleranceStore,
       after: &EntityToleranceStore,
   ) -> Outcome<()>;
   ```

4. **Certificate and refusal shapes** — the house structural pattern, shared
   with the wave's other checkers: on success
   `props.set(Prop::ToleranceMonotonicity, Truth::True)`,
   `method: Method::None` (order tests on recorded values — no certified
   arithmetic), `budget_left: Budget::new(0, 0, 0)`,
   `margin: Margin::UNBOUNDED`, `modulus: Modulus::Unbounded`. Every failure
   is one refusal, verbatim:

   ```rust
   Err(Refusal::Contradictory(ContradictionWitness {
       prop: Prop::ToleranceMonotonicity,
       left: Truth::True,
       right: Truth::False,
   }))
   ```

   Check order in `check_transition`: invalid values first, then boundary
   violations, then decreases (any one alone is already Contradictory; the
   order only fixes WHICH the doctests observe).

5. **Tests** — one `#[cfg(test)]` module per file, each opening with

   ```rust
   #[cfg(test)]
   // Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
   // geometry. Unit-test assertions on hand-built witnesses are not such a path.
   #[allow(clippy::unwrap_used, clippy::expect_used)]
   mod tests {
       #![deny(clippy::unwrap_used)]
       use super::*;
   ```

   All witnesses are hand-built ids — no geometry. Build them exactly like
   this (they exercise the real `entity_id` API):

   ```rust
   use crate::entity_id::{End, EntityId, Op, OpKind, OpParams, Selector};

   let src7 = EntityId::src(7);
   let face = EntityId::src(11);
   let wire0 = EntityId::sel(face.clone(), Selector::BoundaryWire(0));
   let edge1 = EntityId::sel(wire0.clone(), Selector::WireEdge(1));
   let vend = EntityId::sel(edge1.clone(), Selector::End(End::Front));
   let swept = Op { kind: OpKind::Sweep, params: OpParams::Scalar(2.5) }
       .output(&[EntityId::src(7)], 0);
   ```

   Every expected value below was machine-checked against a model of this
   design before this packet was written; if a test disagrees with the code,
   suspect the CODE, and say so in `disagreements`.

   - `raise_is_monotone_and_idempotent` — `raise(src7, 3.0)` then
     `get(&src7) == Some(3.0)`; `raise(src7, 1.0)` leaves it `Some(3.0)`;
     `raise(src7, 5.0)` makes it `Some(5.0)`; a `raise(face, 2.0)` leaves
     `get(&src7)` at `Some(5.0)` and `get(&face)` at `Some(2.0)`.
   - `raise_refuses_invalid_candidates` — `raise(src7, -1.0)`,
     `raise(src7, f64::NAN)`, `raise(src7, f64::INFINITY)` each return
     `Err(Refusal::UnsupportedEnvelope(EnvelopeCase::ChartDegenerate))` and
     leave `get(&src7) == None`; then `raise(src7, 0.0)` succeeds and
     `get(&src7) == Some(0.0)` — zero is a REAL record, distinct from
     missing.
   - `missing_record_is_none_not_zero` — `get(&swept) == None` on a fresh
     store; after `raise(swept, 0.0)`, `get(&swept) == Some(0.0)`.
   - `boundary_monotonicity_flags_sel_above_base` — build via the PUBLIC
     api: `raise(wire0, 1.0)` and `raise(edge1, 5.0)` (nothing else). Then
     `boundary_violations` is exactly `[(edge1, wire0)]` (face unrecorded —
     no constraint from it), and `check_store` is `Err(Contradictory)` with
     `witness.prop == Prop::ToleranceMonotonicity`.
   - `boundary_monotonicity_accepts_chain_and_gap` — (a) the full chain
     `face=4.0, wire0=3.0, edge1=2.0, vend=1.0`: `check_store` is `Ok` and
     the certificate maps `Prop::ToleranceMonotonicity` to `Truth::True`;
     (b) the GAP case `face=4.0, vend=9.0` only (wire0 and edge1
     unrecorded): `check_store` is still `Ok` — a missing intermediate is
     not zero and invents no constraint; (c) equality:
     `face=3.0, wire0=3.0` is `Ok`.
   - `transition_flags_decrease` — `before` with `raise(src7, 5.0)`;
     `after` (a FRESH store) with `raise(src7, 3.0)`. Both stores are
     entirely public-api-built. `decreased_records(&before, &after)` is
     exactly `[src7]` and `check_transition` is `Err(Contradictory)`.
   - `transition_accepts_raise_fresh_deleted` — `before`:
     `src7=3.0, face=2.0`; `after`: `src7=5.0` (raised), `swept=1.0`
     (fresh), face absent (deleted). `check_transition` is `Ok`. Also the
     unchanged case `src7=3.0 → src7=3.0` is `Ok` (equality is allowed).
   - `serde_round_trip_preserves_records` — a store with
     `src7=5.0, face=1.0, wire0=1.0, edge1=0.5, vend=0.25`;
     `serde_json::to_string` then `from_str` back; assert the stores are
     `==` (HashMap equality is order-insensitive) and `check_store` is
     `Ok`.
   - `checker_flags_invalid_deserialized_value` — deserialise this exact
     tampered pairs-form payload (the shape of decision 2; `Src(7)` is
     externally tagged `{"Src":7}`, the record is `{"value":-1.0}`):

     ```rust
     let tampered = r#"[[{"Src":7},{"value":-1.0}]]"#;
     let store: EntityToleranceStore =
         serde_json::from_str(tampered).expect("plain JSON pairs");
     ```

     (`.expect` is fine here: this is a test on a literal.) Deserialisation
     SUCCEEDS — serde does not validate; `invalid_records(&store)` is exactly
     `[src7]` and `check_store(&store)` is `Err(Contradictory)` with
     `prop == Prop::ToleranceMonotonicity`.

6. **One doctest**, on `EntityToleranceStore` itself: raise twice (3.0 then
   1.0), assert `get` reads `Some(3.0)`, and assert
   `truck_topology::invariants::tolerance_monotonicity::check_store` on it
   is `Ok`.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N`-shaped literal unless that line ends with an `// H-3` comment. This
packet's tests use plain literals (`3.0`, `5.0`, `-1.0`, `2.5`, `0.25`) which
do not match the pattern — **do not introduce any `1e-N` literal, and never
an epsilon comparison**: the values pass through `max` and `>` unchanged, so
equality assertions are exact and need no slack. Run
`bash scripts/kernel-gates.sh` yourself before writing `RESULT.json`.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --tests --no-fail-fast
cargo test -p truck-topology --doc
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`. The crate is clean at baseline (clippy zero
findings; 59 lib tests + 4 test binaries green, 1 pre-existing ignored,
measured at HEAD 59a7348 on 2026-08-24); your bar is everything above stays
green plus your nine tests and one doctest.

## Forbidden

Editing any file outside `write_allow` (the manifest is correct as it stands
— no dependency changes). Wiring the store into any operation, `Edge`,
`Face`, `Shell` or builder — there is deliberately no caller yet. Touching
`entity_id.rs` (it is a read-only dependency of this packet). Adding
provenance/metadata to `EntityTolerance` (a recorded non-goal). Treating a
missing record as zero anywhere. Comparing through an unrecorded
intermediate (the chain rule of decision 3). Adding `#[ignore]`, or
`unwrap()`/`expect()` outside the test modules. Changing the refusal shape,
the certificate fields, or the `raise` semantics of decisions 1-4.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the doctest or a test cannot be expressed because a signature here does not
  compile against the real `entity_id` / `evidence` APIs → `SPEC_GAP`, with
  the exact mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`feat(topology): per-entity tolerance store + monotonicity checker (BG-INV-107)`.
