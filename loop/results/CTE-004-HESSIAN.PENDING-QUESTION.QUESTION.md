# QUESTION — CTE-004-HESSIAN stopped at anchor re-check (ANCHOR_MISMATCH)

## Status

`ANCHOR_MISMATCH`. No code written, no commit made (stopped pre-build at zero
cost, the CTE-000 r1 precedent, `8002d23`).

## What was checked

The packet's anchor table was re-checked on the current branch
(`packet/CTE-004-HESSIAN`) before building, per "Anchors — measured this
session; re-check on your branch before building".

| id | file | pattern | expect | measured |
|---|---|---|---|---|
| A1 | `tangency/shapes.rs` | `pub struct GraphEnclosure` | 1 | 1 |
| A2 | `tangency/shapes.rs` | `pub enum ContactVerdict` | 1 | 1 |
| A3 | `tangency/shapes.rs` | `pub enum Definiteness` | 1 | 1 (type-name boundary; naive substring also hits `DefinitenessSign`) |
| A4 | `tangency/tsystem.rs` | `impl KrawczykSystem<4> for TSystem` | 1 | 1 |
| A5 | `tangency/graph.rs` | `pub struct CertifiedGraph` | 1 | 1 |
| A6 | `kernel/contact.rs` | `fn det2` | 1 | 1 |
| A7 | `tangency/witness.rs` | `pub fn verify` | 1 | **0 — file does not exist** |

## The blocker

`vendor/truck/truck-certified/src/tangency/witness.rs` is absent from the tree
(`git ls-files` confirms it was never tracked; `git log --all` shows no
CTE-001 landing). It is the CTE-001 file that implements the frozen one-verifier
contract `ExactWitnessVerifier<P>` (`tangency/shapes.rs:591-605`,
`CTE_BUILD_SPINE.md` §2 "ExactVanishingWitness — CTE-000 data, CTE-001
verifier"). Only CTE-000-SPINE, CTE-002-GRAPH and CTE-003-MINORS have landed;
`loop/packets/CTE-001-QPOLY.md` is registered but never dispatched/landed.

Why this blocks CTE-004 beyond the anchor gate:

- Scope decision 4 / theory §4.1 (D7): `A1Isolated` requires an exact-zero
  witness, and interval methods alone cannot certify isolation.
- `A1Cert::new` (`shapes.rs:759`) requires an A1-instantiated
  `ExactVanishingWitness<P>`; its construction presumes the ℚ-polynomial
  substrate + one verifier that CTE-001 owns.
- Required test `f3_isolated_contact_verdict` demands "F3 → `A1Isolated` with
  the exact-zero witness **verified**" — the verification step is CTE-001's
  `verify`, which does not exist here (no `pub fn verify` anywhere in the
  tree).
- The packet's own `depends_on: [CTE-002-GRAPH, CTE-003-MINORS]` omits CTE-001,
  so dispatch fired without it — authoring/registry rot of the same class as
  `8002d23` (CTE-000 r1) and `aaf7fff` (CTE-006 dispatch before CTE-000).

## Ask

1. Add `CTE-001-QPOLY` to CTE-004's `depends_on` (and re-check the other
   CTE-00x rows for the same omission — CTE-005 already lists it).
2. Land CTE-001-QPOLY (`tangency/qpoly.rs` + `tangency/witness.rs`, plus the
   two `pub mod` lines in `tangency/mod.rs`) first, then redispatch CTE-004.
3. Optionally re-point anchor A7 so a future pre-CTE-001 dispatch fails
   cleanly at the dependency gate rather than the anchor gate.
