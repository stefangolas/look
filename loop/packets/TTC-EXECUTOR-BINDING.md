# WORK PACKET TTC-EXECUTOR-BINDING — the drop-in Python surface: corpus scripts run unmodified on the kernel

The corpus→kernel executor (docs/TT_MODEL_CODEPATH_AUDIT.md, gap 1). Today
`door.py` answers `cadgen.build123d` with real build123d (OCC). This packet
lands the truck answer: a pyo3 drop-in module covering the census
vocabulary, and the door's kernel-engine regime. After this packet, a
manifest row runs the SAME vendored script through BOTH engines.

```yaml
id:          TTC-EXECUTOR-BINDING
contract:    [TTC-EXECUTOR-BINDING]
class:       design
crates:      [truck123d]
depends_on:  [PB-011-TTC-PARITY-CHURN]
write_allow:
  - truck123d/src/python.rs
  - truck123d/src/exceptions.rs
  - truck123d/src/marshal.rs
  - truck123d/src/gil.rs
  - truck123d/src/tables.rs
  - truck123d/src/facade.rs
  - truck123d/src/lib.rs
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
read_allow:
  - corpus/ttc/
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - docs/TT_MODEL_CODEPATH_AUDIT.md
tests_required:
  - the drop-in module answers the census vocabulary name-for-name
    (same signatures, never improved — the spec 8 contract)
  - at least one canonical Falcon-Heavy row runs end to end through the
    kernel engine regime and its geometry facts match the recorded
    reference within tolerance
  - a refusal surfaces as the mapped Python exception, never a panic
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'def install_cadgen_alias' corpus/ttc/door.py"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn truck123d' truck123d/src/lib.rs"}
budget:      {turns: 70, ctx_tokens: 190000}
```

## Scope

1. **The kernel-engine regime in `door.py`**: an engine switch
   (`--engine truck`) alongside the OCC baseline — the alias installs the
   truck bridge module instead of build123d. The OCC path is UNTOUCHED
   (it stays the differential oracle and the reference recorder).
2. **The pyo3 bridge module** (`bd_bridge.rs`): the census vocabulary
   (~20-30 names from the PB-010 survey — primitives, profile authoring,
   extrude/revolve/revolve_arc/sweep/loft, the algebra operators via the
   facade's `Mode` encoding, placement, export) mapped onto the facade
   ops. Name-for-name fidelity; refusals map to the landed Python
   exception types (exceptions.rs).
3. **Geometry behind the facade**: the op arms gain their kernel calls
   (PB-011 landed the boolean routing; this packet completes the
   construction/export arms) — the classifier stays as the pre-flight.
4. **Determinism**: the op log order is load-bearing; identical script
   runs produce identical facts (the existing facade contract, now with
   geometry attached).
5. H-1/H-3/H-6 as always. The GIL policy follows gil.rs's landed rules.

## Done when

```
cargo test -p truck123d
```

plus the door regime test: one canonical row through `--engine truck`
matches its reference facts.

## Forbidden

Wrapping or improving build123d names beyond the census vocabulary.
Touching the OCC door path. Silent geometry degradation (an unmapped name
is a loud import/attribute error, never a silent fallback to OCC).

## Stop conditions

- A census-vocabulary name cannot map onto a landed facade/kernel op →
  SPEC_GAP naming it (that is a capability gap, booked — not bridged).
- Facts mismatch beyond tolerance on the pilot row → STOP with the delta.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `feat(bridge): the drop-in executor binding — corpus scripts run unmodified on the kernel (TTC-EXECUTOR-BINDING)`.
