# WORK PACKET F1-MACHINERY-AUDIT — survey: landed theory machinery for the F1 monocoque path

Survey class. No vendor writes, no cargo. The deliverable is SURVEY.json at
the WORKTREE ROOT: one row per site, and a `summary` object answering the
four questions below. The orchestrator will run validate_survey.py and
review every low-confidence row; invented counts or runtimes are worse than
omissions.

```yaml
id:          F1-MACHINERY-AUDIT
contract:    [F1-MACHINERY-AUDIT]
class:       survey
crates:      []
depends_on:  []
write_allow: []
read_allow:
  - vendor/truck/truck-certified/src/
  - vendor/truck/truck-evidence/src/
  - vendor/truck/truck-geometry/src/
  - vendor/truck/truck-shapeops/src/
  - docs/BENCHMARKS.md
  - docs/OP_CAPABILITY_MATRIX.md
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - docs/CERTIFIED_TANGENCY_EXACT_CONTACT_SPEC.md
  - docs/FSSI_BUILD_SPEC.md
  - docs/TT_MODEL_CODEPATH_AUDIT.md
  - docs/TORUS_CONTACT_PROGRAM.md
  - corpus/ttc/SKIPS.json
  - loop/results/
tests_required: []
anchors:
  - {id: A1, expect: 15, cmd: "grep -c 'pub fn' vendor/truck/truck-certified/src/construct/bie/ssi4.rs"}
  - {id: A2, expect: 6, cmd: "ls vendor/truck/truck-evidence/src/num | wc -l"}
budget:      {turns: 60, ctx_tokens: 170000}
```

## The four questions (answer in SURVEY.json `summary`, row-backed in `rows`)

1. **Machinery inventory.** Every landed theory module relevant to the F1
   monocoque path, classified into: (a) authoring-carrier admission (Plane,
   Spline, make_face, loft/revolve, partial arc, rotation handling) — the
   bridge/door layer; (b) swept-carrier boolean interaction — ssi4.rs,
   closure.rs, the num/ stack (krawczyk, parallelotope, cluster, roots,
   sweep_sigma), broadphase/branch/BVH, the escalation lattice and CFP-008
   stagnation verdicts; (c) boolean funnel tier routing (which pair classes
   route where, per contract.rs vocabularies).
2. **Input coverage per module.** For each row: which carrier pair classes /
   forms the landed tests or contracts admit, with the ADMITTING TEST named
   (file:line). Distinguish "admitted by landed test" from "claimed in
   prose" — a claim without a test is `confidence: low` by definition.
3. **Runtimes.** Every runtime number that exists ANYWHERE in the repo for
   these modules (docs/BENCHMARKS.md sections, loop/results/*.json evidence,
   probe logs cited in RESULTs), each marked MEASURED with its source path.
   If a module has NO measured runtime anywhere, its row says
   `runtime: UNMEASURED` — do not estimate, do not invent. The orchestrator
   will treat a fabricated number as a packet fault.
4. **Monocoque path mapping.** The tub is: loft skin from spline section
   faces, cut(loft skin, loft cavity), then fuse with 14 proud bosses.
   List, in execution order, which inventoried modules this path WOULD hit
   once the authoring bridge arms exist, and name where the first
   uncertified step sits.

## Method

Read the landed tests, not just the module headers — the admitting test IS
the input coverage claim. The capability matrix (docs/OP_CAPABILITY_MATRIX.md)
is a starting map, not the answer: spot-check its rows against the tree and
mark any matrix row you cannot reproduce from code or tests as
`confidence: low, matrix_row_unverified`.

## Stop conditions

- A module you cannot classify into (a)/(b)/(c) → record it as row with
  classification `unknown` and the reason; do not guess.
- SURVEY.json would exceed ~120 rows → prioritize by (4) execution order,
  note the truncation in `summary`.

## Finish by writing SURVEY.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `survey: F1 monocoque theory-machinery audit — inventory, input coverage, runtimes (F1-MACHINERY-AUDIT)`.
