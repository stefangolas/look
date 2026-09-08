"""STATE.md session-55 handoff rewrite: replace the volatile section
(header through the parallelism picture), prepend session-55 traps, keep
the accumulated session-54-and-earlier trap sections byte-for-byte."""
from pathlib import Path

STATE = Path(r"C:\Users\stefa\look\loop\STATE.md")
text = STATE.read_text(encoding="utf-8")
keep_from = text.index("### Session 54")
kept = text[keep_from:]

volatile = '''# Autobuild loop - STATE

Rewritten at the end of every session. The **volatile** part - everything
from "Where we are" through "The parallelism picture" - is capped at ~120
lines and must be rewritten each time. "Traps" and everything below it is
**stable and accumulates**: entries are added when something costs a
session and removed only when they stop being true, never for length. If
you are picking this up cold, read **this file, then
[`loop/ORCHESTRATOR.md`](ORCHESTRATOR.md)` for how to run the loop, then
`python loop/slot_status.py`** - nothing else. Do not read `LEDGER.jsonl`
whole.

Updated 2026-09-07 (late), session 55 - the DEF wave + the TTC chain.

## Where we are

- **CTE, CL, BIE, CC, P1-P12, base kernel, BREP, CFP: COMPLETE.**
- **PB program: PB-011 r1 LANDED 2a12623** (swept-carrier boolean routing
  +787 LOC; cutaway lifted green ~11 s / 409k tris; the facade dispatch is
  a MIRROR of the CL-006 solver-entry dispatch per the dependency law).
  **PB-011 r2 RUNNING** (slot 1): the MONOCOQUE lift - the first
  kernel-side corpus geometry - was blocked by a PRE-EXISTING
  python314.dll loader issue in the truck123d lib test exe under the
  cargoq server env (fails at base; verified by stashing); the worker is
  mid-fix (interpreter dir on PATH).
- **Reference capital COMPLETE: all 34 staged rows have recorded OCC
  reference facts** (`corpus/ttc/reference/*.json`; wall times in
  `scratch/reference_batch_log.json`). OCC column of the timing table is
  measured; **the truck column does not exist yet** - the first number is
  monocoque, from PB-011 r2's lift.
- **OCC is FLAKY on real geometry under load**: the monocoque door run
  threw `Null TopoDS_Shape` 1-in-3 under 4-way concurrency, deterministic
  (42.8 s, 2 solids, volume 6.348e8, 6971 tris) when quiet. Reference
  recorded; the flakiness is a finding (the corpus author's
  repair-everything culture exists for this; our determinism contract is
  the demo).
- **DEF wave: landed** - CDT invariant test repair, vendor fixtures (8
  shape JSONs via the committed generator examples; resources/shape was
  never vendored), fillet identity (complex_surface Closed again),
  SEEDRAY-A (interval ray-crossing primitives), SEEDRAY-C (avoidance
  audit doc + probe - **READY FOR THE USER'S FRONTIER REVIEW**,
  `docs/AUDIT_SEEDRAY_AVOIDANCE.md`), tracer escalation-lattice
  remainder (BG-KV2-207B: kernel_tracer 8/8 - the 3.65x single-pass hull
  inflation mechanism, net-restriction fix).
- **BLOCKED with findings:** DEF-TESS-ANALYTIC-SEAM r1 (density-agreement
  reverted on planar impact) -> **R2 booked** (EdgeID-keyed seam
  contracts, diagnose-first); DEF-SPINEFRAME-GRAZE r1 (pad ladder proved
  the funnel's unclamped Newton ranges 2300x out of domain) -> **R2
  registered** (search-layer clamp).
- **TTC chain booked in the right order:** PB-011B (12 canonical-cutter
  F1 lifts, 2-D path) -> PB-011C (8 swept-x-swept rows, census-first) ->
  per-model timing (TTC-TIMING-FH/MONO) through the executor binding.
  Torus-contact program booking doc: `docs/TORUS_CONTACT_PROGRAM.md`.
  The excluded six (front_wing, cockpit, nose, sidepods, cooling, halo)
  stay refused pending owner decision.

## Pick up here

0. `python loop/dispatch_ready.py --dry-run` FIRST. Two workers may be
   mid-flight or finished: **PB-011 r2** (slot 1 - adjudicate the
   monocoque lift: facts vs `corpus/ttc/reference/monocoque.json`: 2
   solids, volume 6.348e8, 6971 tris; capture the kernel WALL TIME - the
   first truck number) and **TTC-EXECUTOR-BINDING** (slot 0).
1. After r2 lands: **PB-011B dispatches** (write-set clash holds it while
   r2 runs - that is correct), then **PB-011C** after B.
2. After the binding lands: **TTC-TIMING-FH + TTC-TIMING-MONO dispatch**
   - per-model timing through `door.py --engine truck`, three columns
   (time/verdict/facts), BENCHMARKS protocol (release, quiet machine,
   alternating, medians). Results into `docs/TT_TIMING_RESULTS.md`.
3. **USER ACTION pending:** fire the frontier review of
   `docs/AUDIT_SEEDRAY_AVOIDANCE.md` -> unblocks **SEEDRAY-B** (already
   READY-gated on it per the M9 split; A is landed).
4. Dispatch **DEF-TESS-ANALYTIC-SEAM-R2** and **DEF-SPINEFRAME-GRAZE-R2**
   (both READY) into free slots - staggered, the warm-build memory deaths
   (0xc0000409) happen at 4 concurrent builders.
5. The monocoque comparison verdict: OCC-DNF(flaky) vs kernel - if the
   funnel certifies the tub, the facts gate is "builds at all +
   invariants" (OCC has no reference for it beyond the flaky one; the
   recorded one is good).
6. Adjudication reminders: the driver's 5-min cycles land finished rows
   themselves - check `git log --grep` before hand-merging; NEVER write
   "landed <hex>" in a registry note unless the row IS landed (the
   dispatcher's LANDED_RE marker check; bit us twice this session).

## State of the machine, as left

- cargoq + supervisor + watchdog + janitor + driver RUNNING (from 09-05).
- Workers: PB-011 r2 (slot 1) + TTC-EXECUTOR-BINDING (slot 0) RUNNING.
- Disk ~20 GB free. RAM 15.7 GB total, ~5 GB free at 2 workers - the
  0xc0000409 warm-build deaths happen at 4 concurrent builders; stagger.
- The reference batch (scratch/batch_references.py) is COMPLETE (34/34
  keys); its log is the complexity ranking (engine_cover 98.3 s, power_unit
  85.3 s, drivetrain 72.0 s, hypercar/body 52.6 s top measured; monocoque
  DNF).

## The parallelism picture

Unchanged: cargoq serializes all cargo; rolling dispatch via
dispatch_ready; cap 4 workers BUT stagger big warm builds (the memory
deaths); one-verify per program at integrated HEAD. The TTC chain is the
critical path: r2 -> B -> C with the binding in parallel; the timing
packets close it.

### Session 55 (the DEF wave + the TTC chain + the LANDED_RE trap) - paid in full

- **The LANDED_RE self-trap, twice.** The dispatcher treats a note
  matching `landed [0-9a-f]{7,}` (case-insensitive) as "row already
  landed" and silently skips it. Writing "PB-011 LANDED 2a12623" in the
  EXECUTOR-BINDING's note made the dispatcher skip the BINDING; writing
  "packet file landed" prose is safe, but appending real landing markers
  to notes of rows that are not landed is not. And the "fix" (rewording
  the marker) broke a LEGITIMATE marker - PB-011 was genuinely landed by
  the driver overnight, and the reword made its dependents unresolvable.
  Repair: `status = "DONE"` (landed() accepts status directly). RULE:
  never write "landed <hex>" in a note unless the row IS landed; prefer
  status changes for truth; check `git log --grep` before hand-merging
  (the driver's 5-min cycles land finished rows themselves).
- **The corpus builds are SECONDS, not minutes** - airbox 18 s,
  beam_wing 7 s, drivetrain 72 s, engine_cover 98 s. The "minutes each"
  assumption was wrong and inflated the PB-011 estimate by hours. The
  door records wall time on stdout; the batch log
  (`scratch/reference_batch_log.json`) is the complexity ranking.
- **The door emits PRETTY-PRINTED MULTI-LINE JSON on stdout** - parse the
  WHOLE stdout; a last-line parser crashed the first reference batch
  mid-run (and orphaned its door subprocesses).
- **OCC is nondeterministic under load** on real geometry (monocoque:
  1 Null TopoDS_Shape in 3 door runs at 4-way concurrency, clean when
  quiet). Any "OCC cannot build X" claim needs the quiet-machine rerun
  before it is a claim.
- **`f1.step.js`/`hypercar.step.js` are KINEMATICS CHOREOGRAPHY, not
  mesh previews** - upstream ships no prebuilt geometry; the trees are
  source-only. The answer keys could not be shortcut; they were recorded
  (34/34, parallel Python fan-out - door runs are NOT cargo, cargoq does
  not gate them, run them 4-8 wide).
- **run_facade was a classifier, not an executor** - the corpus-to-kernel
  flight layer did not exist; PB-011 landed the boolean routing (as a
  facade MIRROR of the solver-entry dispatch, per the dependency law) and
  TTC-EXECUTOR-BINDING lands the flight layer + the door engine switch.
  The timing packets gate on it - no hand-transcribed driver bypass
  (owner directive: the right order).
- **truck123d lib test exe + python314.dll under the cargoq server env**:
  pre-existing at base (verified by stash), integration exes unaffected.
  The lift tests need the interpreter dir on PATH.
- **PB-011's routing is a facade MIRROR of the solver-entry dispatch**
  (the loop-side crate cannot name truck-certified - dependency law) -
  the geometry runs in truck-evidence; the tests drive the certified
  entry directly.
- **The complexity ranking is MEASURED** (reference_batch_log): stop
  guessing build times; sort lifts by the log.
- **Monocoque reference facts**: 2 solids, volume 6.348e8, bbox
  recorded, 6971 tris. The kernel-side facts gate for it: builds at all +
  invariants (OCC's own run is the flaky one).

'''
STATE.write_text(volatile + kept, encoding="utf-8")
print("STATE.md rewritten; kept from:", kept[:60])
