# WORK PACKET FHC-G2 — kernel answers for the corpus's OCC probe idioms

Gap-register category 2: the corpus builders call `is_valid_shape` / bounds
probes on kernel-engine rows; the door refuses them typed ("an OCC probe of a
kernel-engine row is not a kernel-engine row"). The kernel facts these probes
need EXIST — validity is the certificate bracket, bounds are the
carrier-derived enclosure — so this packet maps the probe idioms onto
already-certified facts. No new math.

```yaml
id:          FHC-G2-PROBE-QUERIES
contract:    [FHC-G2-PROBE-QUERIES]
class:       mechanical+
crates:      [truck123d]
depends_on:  [FHC-G3-DATA-ROW-ATTRIBUTES]
needs:       [FHC-G3-DATA-ROW-ATTRIBUTES]
write_allow:
  - corpus/ttc/door.py
  - truck123d/src/bd_bridge.rs
  - truck123d/tests/probe_queries.rs
read_allow:
  - docs/F1_HYPERCAR_GAP_REGISTER.md
  - docs/TTC_CENSUS_FINAL.md
tests_required: [truck123d/tests/probe_queries.rs]
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'OCC probe of a kernel-engine row' corpus/ttc/door.py"}
  - {id: A2, expect: 1, cmd: "grep -c 'fn bd_facts' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 160000}
```

Anchors measured 2026-09-12 at the EX-B-landed HEAD. **Re-measure at
dispatch** — the docket ahead (TRIM, MIRROR, BD-EMIT-MESH-CACHE, FHC-G3)
WILL drift them; update at dispatch, never dispatch stale.

## Rows this closes (interim census verdicts, verbatim)

- `f1/cockpit` — `an OCC probe of a kernel-engine row is not a kernel-engine row`
- `f1/engine_cover` — same
- `f1/monocoque` — same

## Pre-made judgements

1. **Probe semantics from certified facts, honestly.** A validity probe on a
   kernel row answers from the row's own certificate (bracket present and
   finite, solid_count constructive); a bounds probe answers from the
   carrier-derived bbox. A probe the facts cannot answer refuses TYPED
   naming which fact is missing — never approximated, never OCC-consulted.
2. **The refusal message stays for genuinely-OCC probes.** Probing a row
   that fell back to OCC is still refused; only kernel-engine rows gain
   kernel answers.
3. door.py owns the idiom mapping; bd_bridge.rs only if a facts accessor is
   missing. No corpus tree files.
4. `cockpit`'s underlying loft refusal may resurface once its probe passes —
   that is SUCCESS of the diagnosis kind: record the new typed refusal in
   RESULT; do not chase it here.

## Method

Map each probe idiom (validity, bounds, and any other idiom the three rows
actually call — enumerate from the door spot-checks first), one row
end-to-end (door round-trip: probe passes, row proceeds to its next honest
verdict), then the next. Tests: kernel-row validity probe answers from a
bracket; bounds probe answers carrier-derived; an OCC-probe of an OCC row
still refuses typed; one green round-trip per claimed row.

## Done when

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d --all-targets -- -D warnings
cargo test -p truck123d --test probe_queries --locked
```

plus door spot-checks per claimed row. All cargo through the queue (the
`cargo` on PATH IS the queue shim); interpreter dir on PATH if
STATUS_DLL_NOT_FOUND.

## Forbidden

Any file outside `write_allow`; editing `corpus/ttc/trees/**` or
`vendor/truck/**`; consulting OCC at probe time; `#[ignore]`, deleted or
weakened tests, bare `cargo test`; committing to `main`.

## Stop conditions

- a probe idiom whose answer is not derivable from certified facts → typed refusal + `SPEC_GAP` naming the fact
- anchor count differs at dispatch and was not re-measured → `ANCHOR_MISMATCH`
- three consecutive failed cargo runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` AT THE WORKTREE ROOT

```json
{"id":"FHC-G2-PROBE-QUERIES","status":"DONE","contracts":["FHC-G2-PROBE-QUERIES"],
 "anchors_verified":{"A1":3,"A2":1},
 "rows_flipped":[],"notes":"per-row verdict after probes pass"}
```

Commit subject: `truck123d: kernel answers for corpus probe idioms (FHC-G2)`.
