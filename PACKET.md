# WORK PACKET CG-BINDING — the pyo3 translation over the stabilized facade (the dependency-wall resolution)

The door gap audit (loop/audits/DOOR_GAP_AUDIT-2026-09-09.md) recorded the
wall: `truck123d` cannot name the certified funnel, so no corpus row can
reach the landed admission/boolean/volume machinery — the machinery exists,
the path to it does not. AGENTS.md books exactly this packet: "the pyo3
binding translation over the stabilized facade" (deferred behind the CG
core, which is LANDED: BG-CG-000..009 + the BG-KV2 swarm). This packet is
that translation. It adds ONE sanctioned manifest edge, lands the binding
module, and proves it with conformance tests. The door shim's consumer flips
(booleans, loft facts, trim) are FOLLOW-ON packets, deliberately out of
scope.

```yaml
id:          CG-BINDING
contract:    [CG-BINDING]
class:       design
crates:      [truck123d]
depends_on:  [SWEEP-PATH]
write_allow:
  - truck123d/Cargo.toml
  - truck123d/src/binding.rs
  - truck123d/src/marshal.rs
  - truck123d/src/python.rs
read_allow:
  - loop/audits/DOOR_GAP_AUDIT-2026-09-09.md
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - docs/CERTIFICATE_MAPPING.md
  - vendor/truck/truck-evidence/src/contact/
  - vendor/truck/truck-certified/src/
tests_required:
  - binding_exports_the_certified_boolean_entry
  - kernel_refusals_marshall_to_typed_door_vocabulary
  - canonical_pair_end_to_end_through_the_export
  - binding_is_deterministic_bit_identical_rerun
anchors:
  - {id: A1, expect: 4, cmd: "grep -c 'RestrictedSolverEntry' vendor/truck/truck-evidence/src/contact/solver_entry.rs"}
  - {id: A2, expect: 3, cmd: "grep -c 'SplineSsiEntry' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'truck-certified' truck123d/Cargo.toml"}
budget:      {turns: 80, ctx_tokens: 200000}
```

## Scope decisions (pre-decided)

1. **The manifest-edge amendment (record it, do not relitigate).** The
   recorded law "the loop-side crate cannot name truck-certified" is
   AMENDED by this packet per the AGENTS.md booking: exactly one edge,
   `truck123d -> truck-certified`, added in `Cargo.toml` with the C2
   precedent's amendment record in the module doc (the loop-side crate may
   not name truck-certified *except* through this one recorded edge, added
   once, never extended without a spec amendment). The traits stay in
   truck-evidence; the impls stay in truck-certified; the binding calls the
   impls through the trait objects. No funnel code moves.
2. **Export surface (minimal).** Three pyo3 exports over the stabilized
   facade: (a) the certified boolean dispatch — a carrier pair + mode in,
   `Routed`/`Refused` verdict out (the facade mirror's runtime twin);
   (b) the volume-facts entry — an admitted construction's certified volume
   bracket out; (c) the trim-facts entry for the algebraic-trim path. The
   exports consume EXACT data rows (the bridge's recorded carrier shapes);
   no floats cross without an enclosing bracket.
3. **Refusal marshaling is total.** Every kernel refusal kind maps to the
   door's typed exception vocabulary per `docs/CERTIFICATE_MAPPING.md` —
   the binding NEVER panics, never returns a bare string, never approximates.
   An unmapped refusal kind is itself a typed `UnmappedRefusal` (a defect
   signal, never silent).
4. **Determinism (N4 carries).** Same row in → identical verdict + identical
   bracket out, bit-for-bit, across reruns (the named test pins it). Fixed
   order everywhere; no hash iteration on any output path.
5. **Out of scope, verbatim:** any door.py edit, any bd_bridge.rs edit, any
   shim flip, any admission widening. The consumers are follow-on packets
   that dispatch against this landed surface.

## Done when

```
cargo check --locked -p truck123d
cargo test --locked -p truck123d --lib --tests   (serial, interpreter dir on PATH)
```

with the four named tests green: the exports exist and dispatch a canonical
pair end to end (Routed with a certified verdict), kernel refusals marshal
to the typed door vocabulary exhaustively for every landed refusal kind the
funnel can return, the canonical-pair run is bit-identical on rerun, and the
whole existing truck123d suite stays green (V5 net — the binding is additive;
nothing existing changes verdict).

## Forbidden

Any edit outside the write set. Moving funnel code. Approximate brackets.
Silent refusal mapping. Widening admission. Touching the door shim.

## Stop conditions

- The stabilized facade cannot express one of the three exports without
  moving funnel code → SPEC_GAP naming the seam.
- The edge would need a second edge (e.g. truck123d → truck-evidence too) →
  SPEC_GAP; one edge is the amendment's budget.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(binding): the pyo3 translation over the stabilized facade — one sanctioned edge, three exports, total refusal marshaling (CG-BINDING)`.
