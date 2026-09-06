# WORK PACKET PB-012-HYPERCAR-VENDOR — vendor the Hypercar tree into the corpus

You are vendoring the earthtojake text-to-cad **Hypercar** model tree into
`corpus/ttc/trees/hypercar/` following the exact conventions the two
landed trees established. Everything you need is in this document,
`corpus/ttc/PROVENANCE.md`, `corpus/ttc/MANIFEST.json`, and
`loop/results/PB-010-TTC-PARITY-AUDIT.json` §q3 (the vendoring
requirements, derived from the landed conventions). Do not read other spec
files. A genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          PB-012-HYPERCAR-VENDOR
contract:    [PB-012-HYPERCAR-VENDOR]
class:       mechanical
crates:      [truck123d]
depends_on:  [PB-010-TTC-PARITY-AUDIT]
write_allow:
  - corpus/ttc/trees/hypercar/
  - corpus/ttc/MANIFEST.json
  - corpus/ttc/SKIPS.json
  - corpus/ttc/PROVENANCE.md
read_allow:
  - corpus/ttc/
  - loop/results/PB-010-TTC-PARITY-AUDIT.json
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
anchors:
  - {id: A1, expect: 0, cmd: "ls corpus/ttc/trees | grep -c hypercar"}
  - {id: A2, expect: 2, cmd: "ls corpus/ttc/trees | wc -l"}
budget:      {turns: 40, ctx_tokens: 100000}
```

## Problem

The parity program names three corpus models (F1, Hypercar, Falcon X /
Falcon-Heavy). Two are vendored; Hypercar is absent (survey q3:
`corpus/ttc/trees` contains exactly `f1/` and `falcon_heavy/` +
`reference/` sibling). The corpus cannot claim three-model parity until
the third tree lands with the same rigor.

## Scope decisions — pre-made, do not relitigate

1. **Conventions are copied from the landed trees, not invented** — the
   audit's q3 derived them: (a) the upstream license header preserved
   byte-for-byte in every vendored source file (MIT, Thompson Labs LLC —
   verify the Hypercar tree's actual license header at fetch time; if it
   differs from the other trees, STOP: SPEC_GAP with the header quoted);
   (b) a `PROVENANCE.md` entry naming upstream repo path, commit sha,
   license; (c) `MANIFEST.json` rows under `ttc_manifest.v1` (family,
   tree path, module, entry, args, stage), each canonical row carrying a
   recorded `reference/*.json` fact file and each skipped row a
   `SKIPS.json` reason row; (d) skip predictions use the landed reason
   vocabulary (`booleans-on-swept-carriers` / `step-out`) — a
   boolean-free part stages per door tractability.
2. **Fetch discipline**: the tree comes from the upstream source the
   owner identified (same repo as the vendored trees, per
   PROVENANCE.md). Vendor the sources ONLY — no build artifacts, no
   generated outputs, no vendored third-party deps beyond what the two
   landed trees' convention allows (compare their file inventory first).
3. **Staging honesty**: rows whose ops land on the compat surface stage
   `canonical` with a reference fact file generated through the
   OCC-based door (the landed harness's baseline regime — see
   PROVENANCE.md:30-37 and door.py); rows whose ops hit recorded
   boundaries stage `skipped` with a typed reason row. NO row stages
   canonical without a green door run.
4. **No kernel or bridge code changes.** This packet touches corpus
   data only. If the Hypercar tree uses an op outside the compat
   surface, the row skips with the typed reason and the gap is noted in
   RESULT (the parity-churn packet owns facade gaps).

## Tests required

The harness's own gates re-run as the test: `ttc_harness.rs`'s manifest
and skips checks must pass with the new tree registered (no new test file
needed — the existing machine checks ARE the tests). Additionally record
per-row door runs in the MANIFEST stage fields.

## Done when — run these, all must pass

```
cargo test -p truck123d --test ttc_harness
python corpus/ttc/census.py
```

Plus: every canonical row's reference fact file exists and
`SKIPS.json`/`MANIFEST.json`/`PROVENANCE.md` are mutually consistent.

## Forbidden

Anything outside write_allow. Any kernel/bridge source change. Removing
or altering the landed trees' files or rows. Weakening the manifest/skip
schemas. Committing to main.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the Hypercar tree's license header differs from the landed
  conventions → SPEC_GAP with the header quoted
- the tree's op vocabulary needs a facade/kernel change to stage
  canonical → stage it `skipped` with the typed reason and note it in
  RESULT (facade gaps belong to PB-011's program, not this packet)

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-012-HYPERCAR-VENDOR","status":"DONE","contracts":["PB-012-HYPERCAR-VENDOR"],
 "tests_added":0,"anchors_verified":{"A1":1,"A2":3},
 "notes":"rows staged canonical vs skipped and why; provenance entry sha; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `corpus: vendor the Hypercar tree with provenance, manifest, and staged skips (PB-012-HYPERCAR-VENDOR)`.
