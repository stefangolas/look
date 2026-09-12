# WORK PACKET FHC-G7-REFUSAL-METADATA — machine-checkable refusal records + the missing schema documentation

Owner directive 2026-09-12: the door's refusal records carry their semantics
in prose (`message`) with only a two-field payload, and the record schema
(`ttc_door_run.v1`) is documented NOWHERE — not README, not AGENTS.md, not
any tracked doc. This packet makes refusals self-describing for agents that
hit them without codebase familiarity, and documents the schema.

```yaml
id:          FHC-G7-REFUSAL-METADATA
contract:    [FHC-G7-REFUSAL-METADATA]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G2-PROBE-QUERIES]
needs:       [FHC-G2-PROBE-QUERIES]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/binding.rs
  - truck123d/src/marshal.rs
  - truck123d/tests/refusal_metadata.rs
  - docs/REFUSALS.md
  - README.md
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/refusal_metadata.rs]
anchors:
  - {id: A1, expect: 1,  cmd: "grep -c 'def _refuse' corpus/ttc/door.py"}
  - {id: A2, expect: 7,  cmd: "grep -c 'payload' corpus/ttc/door.py"}
  - {id: A3, expect: 32, cmd: "grep -cE 'Refused[^A-Za-z0-9_]|Refused$' truck123d/src/marshal.rs"}
  - {id: A4, expect: 28, cmd: "grep -cE 'payload[^A-Za-z0-9_]|payload$' truck123d/src/marshal.rs"}
budget:      {turns: 55, ctx_tokens: 170000}
```

Anchors measured 2026-09-12 at the G3-landed HEAD. **Re-measure at
dispatch** — the docket ahead (G4/G5, both door.py lanes) WILL drift A1/A2;
update at dispatch, never dispatch stale.

## The enriched record (target schema, `ttc_door_run.v2`)

```json
{
  "schema": "ttc_door_run.v2",
  "door_version": 3, "engine": "truck", "ok": false, "entry": "...",
  "error": {
    "kind": "Refused",
    "refusal_code": "E_UNSUPPORTED_ENVELOPE",
    "typed": true,
    "verb": "fuse",
    "carrier": "spline_loft*spline_loft",
    "phase": "admission",
    "client_site": {"module": "lib.suspension", "via": "surfaces.loft_solid"},
    "message": "kernel refusal: unsupported_envelope",
    "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"},
    "known_gap": {"register": "docs/F1_HYPERCAR_GAP_REGISTER.md", "section": "1",
                  "hint": "canonical cylinder unions need rational-patch flux - tracked"}
  }
}
```

## Pre-made judgements

1. **`refusal_code` is the stable machine slug** — one per case/verb class
   (`E_UNSUPPORTED_ENVELOPE`, `E_EMPTY`, `E_SINGULAR_PARAMETRIZATION`,
   `E_NEEDS_CLOSED_PROFILE`, `E_NOT_A_KERNEL_ROW`, `E_RATIONAL_FLUX_INCONCLUSIVE`
   reserved for FHC-G1, ...). Agents branch on codes; bug reports dedupe on
   them. The code table lives in ONE place (marshal.rs) and door.py maps it.
2. **`verb`/`carrier`/`phase` come from the Rust refusal sites** — each
   marshal.rs site already knows the verb and carrier in scope; thread them
   into the payload instead of leaving them baked in prose. `phase` is one
   of `authoring | extraction | admission | solver | facts | emit` (the
   register's category boundaries).
3. **`typed` is mechanical honesty** — true for every marshal/`_refuse`
   path; the untyped die-off classes (AttributeError/RuntimeError reaching
   the record) emit `typed: false` and are REGISTER GAPS by definition.
4. **`known_gap` is a door-side lookup table** — mapping
   (refusal_code, verb, carrier-prefix) -> {register, section, hint},
   maintained BY HAND as data (it is documentation, not behavior). Unknown
   combinations simply omit the block. Never auto-generate hints.
5. **`client_site`** — the corpus module/entry the door already has plus the
   drop-in method name in scope at the refuse site. No new instrumentation.
6. **`carrier_trace`** stays a flag (`--trace-carriers`), NOT the default
   record — trace volume belongs behind the flag; everything above is
   default.
7. **`v1` records stay valid**: the schema field bumps to
   `ttc_door_run.v2`, `payload`/`message`/`kind` keep their exact shapes and
   literals (the loop greps them), and every existing consumer path
   (census scripts, the register anchors) must keep parsing. The R-series
   census scripts are read-only for this packet — verify, do not edit.

## Documentation deliverable (part of done-when)

`docs/REFUSALS.md`: the full record schema (v1 and v2), the refusal_code
table, the case/envelope vocabulary, the typed/untyped contract, real
verbatim examples per code, and the known_gap table's maintenance rule.
Plus a short README section pointing at it (the README's one prose sentence
gains the link). The record schema has been undocumented since inception —
this closes it.

## Tests required

`truck123d/tests/refusal_metadata.rs`: v2 record shape for a typed refusal
(all fields present, code from the table); untyped path emits
`typed: false`; `known_gap` present for a registered combination and absent
for an unknown one; `payload`/`message`/`kind` byte-identical to the v1
literals; the three census-row spot-check refusals (airbox, brakes,
corner_fl) marshal with their codes.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test refusal_metadata --locked
```

plus three door spot-checks (one green row, one typed refusal, one untyped
if any exists) with records pasted in RESULT, and the docs committed. All
cargo through the queue (the `cargo` on PATH IS the queue shim); interpreter
dir on PATH if STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; changing existing `payload`/`message`/
`kind` literals (loop anchors grep them); editing corpus trees;
`#[ignore]`, deleted or weakened tests, bare `cargo test`; committing to
`main`.

## Stop conditions

- a Rust refusal site cannot supply verb/carrier without a refactor beyond marshalling → omit the field there (partial enrichment is acceptable; record it in RESULT)
- the census scripts break on v2 → the schema bump is wrong; fall back to additive-only fields and note it
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G7-REFUSAL-METADATA","status":"DONE","contracts":["FHC-G7-REFUSAL-METADATA"],
 "anchors_verified":{"A1":1,"A2":7,"A3":32,"A4":28},
 "notes":"codes table size; sites enriched vs omitted; docs committed"}
```

Commit subject: `truck123d: self-describing refusal records + schema docs (FHC-G7)`.
