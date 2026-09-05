# WORK PACKET PB-004-PYO3-CORE — the truck123d crate: marshaling, exceptions, GIL policy

You are implementing the pyo3 core of the Python Bridge (PB) program.
Everything you need is in this document and `docs/TRUCK123D_PY_BRIDGE_SPEC.md`
+ `docs/PY_BRIDGE_CONTRACT.md` (the frozen exception mapping and table
schema). If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```yaml
id:          PB-004-PYO3-CORE
contract:    [PB-004-PYO3-CORE]
class:       mechanical
crates:      [truck123d]
depends_on:  [PB-000-CONTRACT]
write_allow:
  - truck123d/
  - Cargo.toml
  - Cargo.lock
read_allow:
  - docs/PY_BRIDGE_CONTRACT.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - vendor/truck/truck-base/src/evidence.rs
  - showcases/src/harness.rs
tests_required:
  - refusal_maps_to_typed_exception
  - unresolved_carries_witness_payload
  - table_serde_round_trip
  - gil_released_during_kernel_call
budget:      {turns: 55, ctx_tokens: 130000}
```

**New crate** (`truck123d/`): H-1 applies. This is the workspace's ONLY
pyo3 dependency — the spec's zero-pyo3 invariant ends with you, by
owner directive.

## Problem

The bridge crate: module init, `Refusal` → typed exception hierarchy
(exactly the two-class mapping PB-000 froze: `Refused` carrying the
EnvelopeCase/witness payload, `Unresolved` carrying κ/cell/slope),
table serde round-trip, GIL policy.

## Scope decisions — pre-made, do not relitigate

1. **pyo3 version**: use the current stable pyo3 (choose at authoring
   time, state the version in RESULT notes); `abi3` feature for wheel
   portability. `Cargo.toml` gains exactly one workspace member + one
   new dependency edge (pyo3 via truck123d's own Cargo.toml — the ROOT
   manifest gains only the member line).
2. **GIL policy**: every kernel call runs with the GIL released
   (`py.allow_threads`); no kernel type crosses the boundary except via
   opaque handles (a `PyTruckSolid` wrapper holding the Rust value — no
   `#[pyclass]` on kernel types themselves).
3. **Exceptions**: one base `TruckError`; `Refused` and `Unresolved`
   subclasses per the frozen mapping; payloads are Python dataclasses
   built from the witness structs via serde — never pickled internals.
4. **Tables**: serde round-trip of the v1 table schema (PB-000's doc) —
   Python dict → table struct → identical dict.
5. **No geometry**: zero geometric content in this crate (spec §5) — every
   kernel-touching function delegates to a landed entry; if you find
   yourself writing math, STOP (`SPEC_GAP`).
6. **The wheel builds**: `maturin develop` (or `maturin build`) in CI is
   out of scope; a `cargo test -p truck123d` Rust-side test suite + a
   `python -c "import truck123d"` smoke instruction in the README is the
   done-when. If the environment lacks a Python interpreter for the smoke
   test, record it and rely on the Rust suite (do not install toolchains).

## Anchors — measured 2026-09-05, counts are exact

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `Cargo.toml` | `showcases` | 1 |
| A2 | `Cargo.toml` | `truck123d` | 0 |
| A3 | `vendor/truck/truck-base/src/evidence.rs` | `pub enum Refusal` | 1 |
| A4 | `docs/PY_BRIDGE_CONTRACT.md` | `schema` (case-insensitive) | >= 3 |

A2 becomes 1 when you add the workspace member.

## House rules

- **H-1** no unwrap/expect/panic in kernel-reachable paths (pyo3
  boundary conversion may use `?`; the GIL-released closure must not
  panic across the FFI — convert errors, never unwind).
- **H-3** same-line `// H-3` for any test epsilon.
- **Refusal fidelity** (spec §5): nothing degrades to a bare `Exception`.
- **All cargo through the queue shim.** Scoped commands only.

## Tests required

1. `refusal_maps_to_typed_exception` — every variant of the landed
   `Refusal` enum (exhaustive match) converts to the `Refused` exception
   payload without loss (a round-trip through serde to JSON and back).
2. `unresolved_carries_witness_payload` — an `Unresolved` with
   κ/cell/slope serializes the full witness; no field dropped.
3. `table_serde_round_trip` — all three showcases tables round-trip
   dict → struct → dict, equal.
4. `gil_released_during_kernel_call` — a kernel call issued from a
   GIL-holding thread completes with the GIL released (structure the test
   with a second thread proving the GIL was free; document the mechanism
   in notes).

No existing test may be deleted, `#[ignore]`d, or weakened.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d
cargo check --workspace --all-targets
```

The last one proves the workspace member landed cleanly.

## Forbidden

Anything outside `write_allow` — especially any `vendor/**` file, the
showcases crate, landed test files, `scripts/kernel-gates.sh`. Geometry
code (spec §5). Adding `#[ignore]`. Unjustified `#[allow]`. Committing to
`main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- pyo3 + the landed workspace's edition/toolchain conflict → `SPEC_GAP`,
  naming the incompatibility
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-004-PYO3-CORE","status":"DONE","contracts":["PB-004-PYO3-CORE"],
 "tests_added":4,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":"3+"},
 "notes":"pyo3 version pinned, and the GIL-release mechanism you used"}
```

Commit subject: `feat(bridge): truck123d pyo3 crate — exceptions, marshaling, GIL policy (PB-004-PYO3-CORE)`.
