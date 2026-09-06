# WORK PACKET PB-010-TTC-PARITY-AUDIT ??? earthtojake text-to-cad corpus: code-path sufficiency census (survey)

You are auditing ??? with NO write access to any Rust code ??? whether the
earthtojake text-to-cad corpus models (F1, Falcon-Heavy; Hypercar is NOT
vendored ??? confirm) can run through the truck123d bridge **identically to
how they run in the original repo's code, and render fully**. Your
deliverable is `SURVEY.json` at the WORKTREE ROOT: one row per corpus
operation, a classification, and a gap list with evidence. You propose
classifications; you do not decide kernel changes. Every file/line you
cite must resolve against the tree (a fabricated citation is worse than no
audit ??? the V10-class check runs at filing).

```yaml
id:          PB-010-TTC-PARITY-AUDIT
contract:    [PB-010-TTC-PARITY-AUDIT]
class:       survey
crates:      []
depends_on:  [PB-008-TTC-HARNESS]
write_allow: []
read_allow:
  - corpus/ttc/
  - truck123d/src/
  - truck123d/compat/
  - truck123d/tests/ttc_harness.rs
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - docs/defects/
anchors:
  - {id: A1, expect: 36, cmd: "grep -c id corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 23, cmd: "grep -c booleans-on-swept-carriers corpus/ttc/SKIPS.json"}
  - {id: A3, expect: 2, cmd: "ls corpus/ttc/trees | wc -l"}
  - {id: A4, expect: 0, cmd: "ls corpus/ttc/trees | grep -c hypercar"}
budget:      {turns: 70, ctx_tokens: 160000}
```

Anchors ??? measured 2026-09-06; re-check on your branch

| id | file | pattern | expect |
|----|------|---------|--------|
| A1 | corpus/ttc/MANIFEST.json | `"id"` | ???30 (measured 35 rows) |
| A2 | corpus/ttc/SKIPS.json | `booleans-on-swept-carriers` | ???20 (measured 21 rows + 1 reason def) |
| A3 | corpus/ttc/trees | f1 + falcon_heavy + reference dirs; NO hypercar | 3 dirs, hypercar absent |

## What each SURVEY.json row is

One row per distinct OPERATION in the vendored python (a builder call,
boolean, loft/sweep/revolve, placement, selector, color, assembly
insertion, export) across all corpus scripts ??? not per script. Fields:

- `id`: `<tree>/<script>:<op-short-name>`
- `file`, `line`: the vendored python site (must resolve)
- `op`: the construct (e.g. `surfaces.cut`, `_fuse`, `loft_ribs`,
  `Plane.offset`, `shape.color`, `export_step`/`export_stl`)
- `surface`: the compat surface row it exercises (S1..S8) or `none`
- `classification`: one of
  - `runs-today` ??? the harness executes this op on this row now
  - `skip-listed` ??? carried in SKIPS.json; name the reason id
  - `lift-candidate` ??? skip-listed BUT the resolving machinery has since
    landed (see below); name the landed pieces the path needs
  - `missing-facade` ??? no compat/facade entry exists for this op
  - `missing-kernel` ??? facade exists but the kernel path is absent/refuses
  - `client-layer` ??? data row only (colors, names), no kernel geometry
- `evidence`: one sentence with the facade/kernel file+symbol the
  classification rests on.

## The load-bearing questions (answer all, in the survey's `summary` field)

1. **Skip-lift readiness.** 21 rows skip on `booleans-on-swept-carriers`,
   `resolved_by: BIE-006`. The BIE program + CL-002/CL-004 + CL-006
   solver-entry landed 2026-09-05/06. For each of the 21: does the exact
   boolean form the row needs (sweep??canonical, sweep??sweep, revolved
   carrier??canonical, ...) have a landed kernel path through
   `boolean_op` ??? the CL-006 certified entry ??? the BIE-002-SSI4 /
   CL-004 machinery? Classify `lift-candidate` only with the symbol path
   named; otherwise `missing-kernel` naming what is absent.
2. **Render parity payload.** The corpus render stack consumes assembly
   names, per-solid colors, and transforms (f1.step.js pins `#o1.N`).
   Enumerate what PB-006's landed assembly emit + the booked PB-009 GLB
   emission cover, and name anything in the vendored scripts' color/name
   usage that neither captures.
3. **Hypercar.** Confirm it is absent from `corpus/ttc/trees/`. From the
   PROVENANCE.md conventions + MANIFEST schema, name exactly what
   vendoring it requires (license header, provenance entry, manifest
   rows, skip predictions). Do not fetch anything; read-only inference
   from the existing two trees' conventions.
4. **Identical-to-original semantics.** For the ops the corpus runs on
   BOTH paths (ours and OCCT's), list any place our bridge's geometry
   semantics could diverge observably (tolerance-visible facet drift is
   expected and `client-layer`; kernel refusals, missing ops, wrong
   carrier handling are parity gaps). TR-NRB-001 (STEP-out refused,
   STL/GLB instead) is a RECORDED boundary ??? cite it, do not count it a
   gap.

## House rules

- All cargo through the queue shim if you run any (you should need none
  or almost none ??? this is a read-and-cite audit).
- You do NOT write Rust. You do not modify any file outside
  loop/packets/CONTEXT.md artifacts. SURVEY.json + QUESTION.md (if any)
  at the WORKTREE ROOT are your only outputs.

## Stop conditions

- any anchor count differs ??? ANCHOR_MISMATCH
- a required citation cannot be resolved from the allowed reads ??? record
  the row as `unresolved` with the missing access named (do not guess)
- the census exceeds 400 rows ??? stop at 400, note the truncation point

## Finish by writing SURVEY.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `survey: ttc parity code-path census (PB-010-TTC-PARITY-AUDIT)`.

On any non-survey-able state also write `QUESTION.md` beside it.
