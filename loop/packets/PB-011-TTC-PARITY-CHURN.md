# WORK PACKET PB-011-TTC-PARITY-CHURN — corpus parity: route swept-carrier booleans into the landed certified funnel, lift MONOCOQUE

You are making the F1 monocoque — the survival tub — run through the
truck123d bridge on the paths the original repo's code exercises, per the
audit's findings. The audit (`loop/results/PB-010-TTC-PARITY-AUDIT.json`)
is NORMATIVE for scope. The pre-digested context below is measured — you do
NOT need to re-read the full census; cite it, don't re-derive it. A genuine
gap is a SPEC_GAP: stop and report.

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
  - monocoque_row_lifts_green
anchors:
  - {id: A1, expect: 3, cmd: "grep -c NonCanonicalCarrier truck123d/src/facade.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A3, expect: 35, cmd: "grep -c booleans-on-swept-carriers corpus/ttc/SKIPS.json"}
budget:      {turns: 60, ctx_tokens: 170000}
```

H-1: no unwrap/expect without a justified same-line opt-out. H-3
same-line `// H-3`. H-6: never record Float as Exact.

## Pre-digested context (measured 2026-09-07 — cite, don't re-derive)

- **The lift target: `f1/monocoque`** — `corpus/ttc/trees/f1/src/lib/mono_tub.py`,
  manifest row `f1/monocoque` (module `lib.monocoque`, entry
  `build_monocoque`). Its op form: the lofted tub skin cut by a cavity
  (`surfaces.cut` = `cut(swept,canonical)`, census cell 5 — the LARGEST
  liftable cell, 8 rows) plus proud bosses fused on
  (`_fuse_proud` = `fuse(swept,canonical)`, cell 2, 2 rows). Both are
  canonical-tool pairs → the 2-D implicit-reduction path (Theorem 4,
  `truck-evidence/src/contact/implicit2d.rs`), NOT the 4-D deep end.
- **The funnel entry to route into:**
  `truck-evidence/src/contact/solver_entry.rs` (CL-006), reached through
  the sweep-lift adapters (`truck-shapeops/src/boolean/sweep_lift.rs`,
  BIE-006) which admit `SpineFrameSweep` faces. Loft faces are
  BSplineSurface carriers admitted via `truck-certified/src/patch_admit.rs`.
- **The refusal to replace:** `boolean_op` on a non-canonical carrier
  returns `Refusal::UnsupportedEnvelope(NonCanonicalCarrier)` in
  `truck123d/src/facade.rs`; the doc row S1 reads `deferred-bie`.
- **PB-013 finding (normative):** the G1 capability cells are pinned at
  the STEP-out refusal proxy because `run_facade` has NO native geometry
  boolean row for swept carriers — the flip requires BOTH (a) exposing
  swept-carrier boolean rows through `run_facade`'s table validation and
  (b) the dispatch itself.
- **The lift mechanics (evidence-gated):** the monocoque row lifts from
  SKIPS.json `rows` to the runnable manifest ONLY after its script runs
  green through the harness door with geometry facts matching the
  recorded reference (the reference is recorded by the orchestrator's
  parallel batch — if `corpus/ttc/reference/monocoque.json` exists, use
  it; if not, run the door once to record it, same as the canonical
  rows). A run that disagrees with the reference does NOT lift — it
  files a defect record (new file under docs/defects/, added to
  write_allow ONLY via an orchestrator amendment — stop and report).
- **Cell 8 `heal(boolean-result)`:** RULED client-layer identity
  (owner 2026-09-07; provenance `corpus/ttc/trees/f1/src/lib/
  surfaces.py:172` — the corpus helper's own valid-shape early return).
  Not this packet's flip unless free; the ruling is recorded in the
  spec 7a.
- **G2 is NOT work** — TR-NRB-001 STEP-out refusal is the recorded
  boundary. **G4 is covered by PB-009** — verify parity, do not
  re-implement. **The remaining F1 rows are PB-011B's** (separate
  packet, dispatched after this one) — do not lift them here.

## Scope decisions — pre-made, do not relitigate

1. **Routing is additive at the facade boundary.** `boolean_op` keeps its
   signature; the `NonCanonicalCarrier` arm becomes a dispatch into the
   landed certified entry (solver_entry) for carrier classes the funnel
   supports (spline/swept/revolved per CFP-004's stage contract); classes
   the funnel refuses (torus, A7) KEEP the typed refusal. Fail-closed:
   every refusal carries the stratum-pair/localization identity.
2. **Skip lifts are evidence-gated, one row at a time.** Monocoque is
   this packet's lift. A lifted row moves from SKIPS.json `rows` to the
   runnable manifest ONLY after its script runs green through the harness
   door with facts matching the reference.
3. **The compat surface S1 row's `deferred-bie` status text updates to
   `landed` for the routed forms** — the machine-check
   (`compat_surface_table_is_complete`) is satisfied by the existing
   SURFACE_ROWS; you update doc status text + the `surface.rs` row
   descriptions if needed, never the 7-row structure.
4. **V5, absolute**: landed harness tests (`ttc_harness.rs` existing
   tests) are byte-identical constraints — you ADD tests, never modify
   landed ones. The `staged_skips_carry_reasons` test may need the
   lifted row removed from its EXPECTED set — that is the one sanctioned
   edit, recorded in RESULT.
5. **Determinism/cargo rules** as every packet: queue shim, scoped
   commands, no hash-ordered output.

## Tests required

1. `swept_carrier_booleans_route_to_certified_entry` — a lofted/revolved
   carrier pair that refused `NonCanonicalCarrier` now routes through the
   certified entry and returns a certified verdict (or the typed
   localized refusal — assert WHICH, never a bare `Err`).
2. `monocoque_row_lifts_green` — the monocoque row's green door run +
   facts match, iterated from the lifted set (an empty lifted set fails
   the test — lifting is the point).

## Done when

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
Lifting rows other than monocoque (they are PB-011B's).

## Stop conditions

- any anchor count differs → ANCHOR_MISMATCH
- the monocoque run disagrees with its recorded reference fact → stop,
  file the finding in RESULT (defect-record authoring is an orchestrator
  amendment)
- the kernel run requires kernel changes outside truck-evidence's landed
  contact surface → SPEC_GAP naming the gap (do NOT widen into
  vendor/truck yourself)
- three consecutive failed cargo test runs on the same error → BLOCKED

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

```json
{"id":"PB-011-TTC-PARITY-CHURN","status":"DONE","contracts":["PB-011-TTC-PARITY-CHURN"],
 "tests_added":2,"anchors_verified":{"A1":3,"A2":1,"A3":35},
 "notes":"monocoque lift evidence (door facts vs kernel facts); the S1 status text change; any deviation"}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-DONE status also write `QUESTION.md` beside it.

Commit subject: `feat(bridge): corpus parity — swept-carrier boolean routing, monocoque lift (PB-011-TTC-PARITY-CHURN)`.
