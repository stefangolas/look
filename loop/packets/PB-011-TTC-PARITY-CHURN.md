# WORK PACKET PB-011-TTC-PARITY-CHURN — corpus parity: route swept-carrier booleans into the landed certified funnel, lift the skips, run everything

You are making the vendored text-to-cad corpus (F1, Falcon-Heavy) run
through the truck123d bridge **on the paths the original repo's code
exercises**, per the audit's findings. The audit
(`loop/results/PB-010-TTC-PARITY-AUDIT.json`) is NORMATIVE for scope — read
it first; its gap_list G1/G3/G5 plus the q1/q2 summaries define the work.
Do not read other spec files. A genuine gap is a SPEC_GAP: stop and report.

```yaml
id:          PB-011-TTC-PARITY-CHURN
contract:    [PB-011-TTC-PARITY-CHURN]
class:       design
crates:      [truck123d, truck-evidence]
depends_on:  [PB-010-TTC-PARITY-AUDIT, PB-009-GLB-EMIT, PB-014-AUTHORING-REVOLVE, CFP-004-IMPLICIT-REDUCTION, CFP-006-CONE-CERTIFICATES, CFP-008-STAGNATION, CFP-010-GATES]
write_allow:
  - truck123d/src/facade.rs
  - truck123d/compat/surface.rs
  - truck123d/tests/ttc_harness.rs
  - truck123d/tests/pb_parity.rs
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - corpus/ttc/SKIPS.json
read_allow:
  - loop/results/PB-010-TTC-PARITY-AUDIT.json
  - truck123d/src/
  - truck123d/compat/
  - truck123d/tests/
  - vendor/truck/truck-evidence/src/contact/
  - docs/PY_BRIDGE_COMPAT_SURFACE.md
  - docs/TRUCK123D_PY_BRIDGE_SPEC.md
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - corpus/ttc/
tests_required:
  - swept_carrier_booleans_route_to_certified_entry
  - lifted_skip_rows_run_green
  - cutaway_partial_arc_revolve_coverage
anchors:
  - {id: A1, expect: 3, cmd: "grep -c NonCanonicalCarrier truck123d/src/facade.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A3, expect: 35, cmd: "grep -c booleans-on-swept-carriers corpus/ttc/SKIPS.json"}
budget:      {turns: 90, ctx_tokens: 200000}
```

H-1: no unwrap/expect without a justified same-line opt-out. H-3
same-line `// H-3`. H-6: never record Float as Exact.

## Problem — the audit's findings, verbatim scope

- **G1 (the core).** The corpus's dominant verb is swept-carrier boolean
  composition (S1 census: 2,780 binary operators; 21 F1 skip rows +
  cutaway). The kernel machinery LANDED (BIE family, CL-002/004 dispatch,
  CL-006 solver entry at `truck-evidence/src/contact/solver_entry.rs`,
  CFP-004/006/008 funnel stages) — but the FACADE still refuses:
  `boolean_op` on a non-canonical carrier returns
  `Refusal::UnsupportedEnvelope(NonCanonicalCarrier)` (facade.rs). The
  doc row S1 still reads `deferred-bie`. Both are now STALE: route the
  facade's swept-carrier pairs into the landed certified entry and lift
  the skips.
- **G3.** `bd.Circle` closed-profile section authoring has no facade row
  (S5 lists Spline authoring only) — needed by merlin_common.py:247,
  cockpit.py:129, wheels/drivetrain rings. Additive authoring entry.
- **G5.** SKIPS.json's `falcon_heavy/cutaway` note mis-describes the code
  path: the vendored code is partial-arc revolves
  (`falcon_common.py:202-206`, `:277-282`), NOT boolean sectioning.
  Correct the note; lift-readiness keys on partial-arc revolve coverage.
- **G2 is NOT work** — TR-NRB-001 STEP-out refusal is the recorded
  boundary (STL/GLB cover render). Do not touch it.
- **G4 is covered by PB-009** (GLB color/occurrence emission, landed
  before this packet) — verify parity, do not re-implement.

## Scope decisions — pre-made, do not relitigate

1. **Routing is additive at the facade boundary.** `boolean_op` keeps its
   signature; the `NonCanonicalCarrier` arm becomes a dispatch into the
   landed certified entry (solver_entry) for carrier classes the funnel
   supports (spline/swept/revolved per CFP-004's stage contract); classes
   the funnel refuses (torus, A7) KEEP the typed refusal. Fail-closed:
   every refusal carries the stratum-pair/localization identity.
   **PB-013 finding (RESULT, normative for this packet):** the G1
   capability cells were pinned at the STEP-out refusal because
   `run_facade` has NO native geometry boolean row for swept carriers —
   the cells are not executable through the runner at all today. The
   flip therefore requires BOTH: (a) expose swept-carrier boolean rows
   through `run_facade`'s table validation (Mode rows on non-canonical
   carriers must reach the certified entry instead of being refused
   pre-execution), and (b) the dispatch itself. The matrix doc's
   `flipped-by: PB-011` cells assert through that exposed row.
2. **Skip lifts are evidence-gated, one row at a time.** A lifted row
   moves from SKIPS.json `rows` to the runnable manifest ONLY after its
   script runs green through the harness door (geometry facts + report
   JSON + STL). A row that runs but disagrees with its recorded reference
   fact does NOT lift — it files a defect record (new file under
   docs/defects/, added to write_allow ONLY via an orchestrator
   amendment — stop and report instead).
3. **The compat surface S1 row's `deferred-bie` status text updates to
   `landed` for the routed forms** — the machine-check
   (`compat_surface_table_is_complete`) is satisfied by the existing
   SURFACE_ROWS; you update doc status text + the `surface.rs` row
   descriptions if needed, never the 7-row structure.
4. **Partial-arc revolve (cutaway)** — MOVED TO PB-014-AUTHORING-REVOLVE
   (landed ahead of this packet): the capability (arc_deg/start_deg
   trimmed-shell construction) and the SKIPS.json note correction are
   PB-014's. This packet LIFTS the cutaway row — the green door run on
   top of PB-014's capability, per the lift discipline below.
5. **V5, absolute**: landed harness tests (`ttc_harness.rs` existing
   tests) are byte-identical constraints — you ADD tests, never modify
   landed ones. The `staged_skips_carry_reasons` test may need the lifted
   rows removed from its EXPECTED set — that is the one sanctioned edit,
   recorded in RESULT.
6. **Determinism/cargo rules** as every packet: queue shim, scoped
   commands, no hash-ordered output.

## Tests required

1. `swept_carrier_booleans_route_to_certified_entry` — a lofted/revolved
   carrier pair that refused `NonCanonicalCarrier` now routes through the
   certified entry and returns a certified verdict (or the typed
   localized refusal — assert WHICH, never a bare `Err`).
2. `lifted_skip_rows_run_green` — every row you lifted from SKIPS.json
   runs through the harness door green (this test iterates the lifted
   set; an empty lifted set fails the test — lifting is the point).
3. `cutaway_partial_arc_revolve_coverage` — falcon cutaway's
   partial-arc revolve path runs (or the coverage gap is typed and
   booked, per decision 4).

## Done when — run these, all must pass

```
cargo fmt --check -p truck123d
cargo clippy -p truck123d -p truck-evidence --all-targets -- -D warnings
cargo test -p truck123d
python corpus/ttc/census.py  (row census re-counts clean)
```

Scoped commands only, through the queue shim. Send output to a file and
read the tail.

## Forbidden

Anything outside write_allow — especially `vendor/truck/truck-certified/**`
(the funnel landed; consume it), `corpus/ttc/trees/**` (read-only vendored
python), landed test files (V5), `truck123d/Cargo.toml`/`Cargo.lock`.
Weakening any landed assertion. Adding `#[ignore]`. Unjustified
`#[allow]`. Committing to main. Lifting a skip row whose run is not green.

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- a lifted-candidate row's green run requires kernel changes outside
  truck-evidence's landed contact surface → SPEC_GAP naming the gap (do
  NOT widen into vendor/truck yourself)
- a run disagrees with its recorded reference fact → stop, file the
  finding in RESULT (defect-record authoring is an orchestrator
  amendment)
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-011-TTC-PARITY-CHURN","status":"DONE","contracts":["PB-011-TTC-PARITY-CHURN"],
 "tests_added":3,"anchors_verified":{"A1":3,"A2":1,"A3":35},
 "notes":"rows lifted (each with its green-run evidence); rows still skipped and why; the S1 status text change; the cutaway partial-arc finding; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): corpus parity churn — swept-carrier boolean routing, skip lifts, circle authoring (PB-011-TTC-PARITY-CHURN)`.
