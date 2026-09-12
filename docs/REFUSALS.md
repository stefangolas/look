# Refusal records (`ttc_door_run.v1` / `ttc_door_run.v2`)

The corpus door (`corpus/ttc/door.py`) runs one geometry entry per process and
prints one JSON record on stdout. A green run has `"ok": true`; a refused or
failed run has `"ok": false` and an `error` block. This document is the
schema of that block, the stable refusal-code vocabulary, and the rules that
keep the records machine-readable for agents that hit a refusal without
codebase familiarity.

Nothing here changes the geometry contract: a refusal is still a typed
`truck123d.Refused` / `truck123d.Unresolved` exception, never a bare
`Exception`, and never a silent fallback.

## Envelope

Both schema versions share the outer record:

```json
{
  "schema": "ttc_door_run.v2",
  "door_version": "3",
  "engine": "truck",
  "ok": false,
  "entry": "build_airbox",
  "module": "lib.engine_cover",
  "error": { "...": "see below" }
}
```

A green record additionally carries `type`, `build_seconds`, `facts`, and
`stl`. `schema` is `ttc_door_run.v2` for the enriched record described here;
the pre-enrichment shape was `ttc_door_run.v1`. The `payload`, `message` and
`kind` members are byte-identical across the two versions — only additive
siblings were introduced — so v1 consumers keep parsing v2 records.

## The `error` block

### v1

```json
{
  "kind": "Refused",
  "message": "kernel refusal: unsupported_envelope",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

### v2

```json
{
  "kind": "Refused",
  "refusal_code": "E_UNSUPPORTED_ENVELOPE",
  "typed": true,
  "verb": "cut",
  "carrier": "spline_loft*spline_loft",
  "phase": "admission",
  "client_site": {"module": "lib.engine_cover", "via": "boolean"},
  "message": "kernel refusal: unsupported_envelope",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"},
  "known_gap": {
    "register": "docs/F1_HYPERCAR_GAP_REGISTER.md",
    "section": "1",
    "hint": "canonical cylinder unions need rational-patch flux - tracked"
  }
}
```

| field | presence | meaning |
|---|---|---|
| `kind` | always | The Python exception class name: `Refused`, `Unresolved`, or an untyped die-off class (`AttributeError`, `RuntimeError`, ...). |
| `refusal_code` | typed only | The stable machine slug (below). Absent on an untyped failure. |
| `typed` | always | `true` for every marshal / `_refuse` path; `false` for an untyped die-off class reaching the record. |
| `verb` | optional | The operation in scope at the refusal site (`fuse`, `cut`, `revolve`, `extrude`, `probe`, ...). Omitted when the site cannot supply one. |
| `carrier` | optional | The carrier class (or carrier pair `a*b`) in scope. Omitted when the site cannot supply one. |
| `phase` | optional | One of `authoring`, `extraction`, `admission`, `solver`, `facts`, `emit` (the register's category boundaries). |
| `client_site` | optional | `{module, via}`: the corpus module and the drop-in method/function name in scope at the refuse site. No new instrumentation — the door already knows both. |
| `message` | always | The human-readable exception message. Frozen literal for each case. |
| `payload` | typed only | The frozen v1 payload (`case` plus the case-specific fields). Its shape never changes. |
| `known_gap` | optional | The hand-maintained register pointer (below). Omitted when no rule matches. |

An untyped failure (a register gap by definition) carries only `kind`,
`typed: false`, `client_site`, and `message`:

```json
{
  "kind": "AttributeError",
  "typed": false,
  "client_site": {"module": "lib.falcon_common", "via": "build_vehicle"},
  "message": "'Compound' object has no attribute 'moved'"
}
```

## Refusal codes

`refusal_code` is the stable machine slug. It is derived from the marshaled
payload's `case`; one slug per case/verb class. The authoritative table is
`truck123d/src/marshal.rs::refusal_code_for_case`; `corpus/ttc/door.py`
mirrors it as data (`REFUSAL_CODES`) for the door-side `_refuse` path.

| `refusal_code` | payload `case` | meaning |
|---|---|---|
| `E_EMPTY` | `empty` | An empty operation domain. |
| `E_UNSUPPORTED_ENVELOPE` | `unsupported_envelope` | A carrier/envelope outside the supported funnel. |
| `E_CONSTRUCT_REFUSED` | `construct_refused` | A construct-stage refusal (the variant rides `payload.stage`). |
| `E_UNMAPPED_REFUSAL` | `unmapped_refusal` | A certified refusal kind with no booked door mapping (a defect signal). |
| `E_NUMERICALLY_UNRESOLVED` | `numerically_unresolved` | The budget was exhausted before certification. |
| `E_COMPOSITION_MARGIN_EXHAUSTED` | `composition_margin_exhausted` | A composition stage exhausted its margin. |
| `E_INPUT_OUTSIDE_BACKWARD_BUDGET` | `input_outside_backward_budget` | An input outside the backward budget. |
| `E_CONTRADICTORY` | `contradictory` | Conflicting evidence on a property. |
| `E_COLLAPSED` | `collapsed` | An exact object collapsed. |
| `E_FORWARD_TOLERANCE_EXCEEDED` | `forward_tolerance_exceeded` | The forward tolerance was exceeded. |

The door additionally uses semantic codes for refusals whose payload case is
the generic envelope slug:

| `refusal_code` | site | meaning |
|---|---|---|
| `E_NEEDS_CLOSED_PROFILE` | `revolve` | The profile does not close (e.g. `hypercar/brakes`). |
| `E_NOT_A_KERNEL_ROW` | data-row / OCC-probe / loft sites | A name or attribute the kernel executor does not carry. |
| `E_SINGULAR_PARAMETRIZATION` | admission | A degenerate parametrization (reserved; e.g. the planar fan cap). |
| `E_RATIONAL_FLUX_INCONCLUSIVE` | solver | Rational-patch flux not yet certified (reserved for FHC-G1). |

## Payload `case` / envelope vocabulary

The payload `case` is the marshaled `Refusal` variant (snake_case). The
`envelope` field (present only for `unsupported_envelope`) is one of the
`EnvelopeCase` names:

`chart_degenerate`, `reach_too_small`, `non_canonical_carrier`,
`non_positive_nurbs_weight`, `contact_reduction_deferred`, `construct_refused`.

Other cases carry their own fields exactly as the v1 table defined them
(`stage`, `prop`/`left`/`right`, `reason`/`certificate`,
`bound`/`allowed`, or the κ `kappa`/`witness` for `Unresolved`). Absent fields
are omitted.

## Typed / untyped contract

- Every kernel refusal is marshaled to `truck123d.Refused` or
  `truck123d.Unresolved` and emits `typed: true` with a `refusal_code`.
- An untyped die-off class (`AttributeError`, `RuntimeError`, ...) reaching
  the record emits `typed: false` and no `refusal_code`. By definition these
  are **register gaps**: a swallowed or unmapped refusal to be classified, not
  a permanent answer. They are catalogued in
  [`docs/F1_HYPERCAR_GAP_REGISTER.md`](F1_HYPERCAR_GAP_REGISTER.md) §4
  (DIAGNOSIS).
- `typed` is mechanical honesty: it is set by the marshaling path itself, not
  inferred from prose.

## Known-gap table

`known_gap` is a door-side lookup table keyed by
`(refusal_code, verb, carrier-prefix)`. It is maintained **by hand as data**
(it is documentation, not behavior). Unknown combinations simply omit the
block, and hints are never auto-generated. The table lives in
`corpus/ttc/door.py` (`KNOWN_GAPS`) and points into
[`docs/F1_HYPERCAR_GAP_REGISTER.md`](F1_HYPERCAR_GAP_REGISTER.md):

| code | verb | carrier prefix | register section |
|---|---|---|---|
| `E_UNSUPPORTED_ENVELOPE` | — | `spline_loft` | §1 rational-patch flux |
| `E_UNSUPPORTED_ENVELOPE` | — | `rotational` | §1 rational-patch flux |
| `E_UNSUPPORTED_ENVELOPE` | `probe` | `occ_probe` | §2 probe query surface |
| `E_UNSUPPORTED_ENVELOPE` | — | `face_data_row` | §3 data-row attributes |
| `E_NEEDS_CLOSED_PROFILE` | — | — | §4 named-carrier admission |
| `E_SINGULAR_PARAMETRIZATION` | — | — | §4b degenerate planar fan cap |
| `E_EMPTY` | — | — | §4 diagnosis |
| `E_RATIONAL_FLUX_INCONCLUSIVE` | — | — | §1 rational-patch flux (FHC-G1) |
| `E_NOT_A_KERNEL_ROW` | — | — | §3 data-row surface |

**Maintenance rule.** A new rule is added only when a census row's refusal is
classified against a register section; the `hint` is copied from the register
entry, never synthesized. A code/verb/carrier combination with no classified
section stays absent.

## Verbatim examples

`f1/airbox` (`E_UNSUPPORTED_ENVELOPE`, the rational-patch flux gap):

```json
{
  "kind": "Refused",
  "refusal_code": "E_UNSUPPORTED_ENVELOPE",
  "typed": true,
  "verb": "cut",
  "carrier": "spline_loft*spline_loft",
  "phase": "admission",
  "client_site": {"module": "lib.engine_cover", "via": "boolean"},
  "message": "kernel refusal: unsupported_envelope",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"},
  "known_gap": {
    "register": "docs/F1_HYPERCAR_GAP_REGISTER.md",
    "section": "1",
    "hint": "canonical cylinder unions need rational-patch flux - tracked"
  }
}
```

`hypercar/brakes` (`E_NEEDS_CLOSED_PROFILE`):

```json
{
  "kind": "Refused",
  "refusal_code": "E_NEEDS_CLOSED_PROFILE",
  "typed": true,
  "verb": "revolve",
  "carrier": "open_profile",
  "phase": "extraction",
  "client_site": {"module": "lib.brakes", "via": "revolve"},
  "message": "revolve needs a closed profile",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"},
  "known_gap": {
    "register": "docs/F1_HYPERCAR_GAP_REGISTER.md",
    "section": "4",
    "hint": "revolve needs a closed profile - named carrier (hypercar/brakes)"
  }
}
```

`f1/corner_fl` (`E_UNSUPPORTED_ENVELOPE`, no registered combination):

```json
{
  "kind": "Refused",
  "refusal_code": "E_UNSUPPORTED_ENVELOPE",
  "typed": true,
  "verb": "cut",
  "carrier": "lathe*cylinder",
  "phase": "admission",
  "client_site": {"module": "lib.wheels", "via": "boolean"},
  "message": "kernel refusal: unsupported_envelope",
  "payload": {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
}
```

An untyped die-off (`falcon_heavy/second_stage`):

```json
{
  "kind": "AttributeError",
  "typed": false,
  "client_site": {"module": "lib.falcon_common", "via": "build_vehicle"},
  "message": "'Compound' object has no attribute 'moved'"
}
```

## Consumers

Census scripts and the register anchors grep the frozen `payload` / `message`
/ `kind` literals; those are unchanged. New consumers should branch on
`refusal_code` and dedupe on it. The `known_gap` block is the bridge from a
code to the register section that owns the fix.
