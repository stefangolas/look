# Certified-kernel interleave build spec — census / shim / Phase-2 wave

**Owner-authored 2026-09-01 (session 49), per the delegation recorded in
`docs/CERTIFIED_PHASE2_BOOKING.md` amendment (fourth input gate): "the
interleave of the census, DISPATCH-2, the recognizer family, and Phase 2
will be decided in a NEW BUILD SPEC, not here."** This is that spec.

## Correction of record first (found by command, before any spend)

The session-48 close claim "BG-CK-P2-CONTRACT LANDED at 37b0267" is wrong on
three counts, each machine-checked at session-49 open:

1. `git show --stat 37b0267` touches ONLY `loop/packets/BG-CK-P2-CONTRACT.md`
   — the packet doc, not code.
2. `git grep -l SquareSystem3 -- vendor` is empty at HEAD and at 37b0267;
   `git log --all -S SquareSystem3` matches only loop-side commits. The shim
   TYPES DO NOT EXIST in any tree.
3. `loop/PACKETS.jsonl` ends at `BG-CK-P1-FLOOR` — the shim was never
   registered, never dispatched, never verified.

What DOES exist: the shim packet doc is complete, anchored, and lint-clean
as authored. Recovery is exactly the normal-loop path the wave-mode section
already prescribes: the shim dispatches as the pre-wave packet, and **the
wave base is the shim's landing merge commit on `integration/kernel-bg`,
not 37b0267** (ORCHESTRATOR wave mode: "The wave base is taken AFTER the
shim lands"). Forking wave branches from 37b0267 was never possible — the
shim code is absent from that tree.

## The interleave (the decision this spec exists to record)

```text
[1] census (measurement)  ─┐  dispatched in parallel NOW (disjoint write
[2] shim (normal loop)    ─┘  sets; both must clear the wave's gates)
           │
           ▼  wave opens ONLY when BOTH hold:
           │     (a) shim verified (full verify, normal loop) and merged;
           │     (b) census RESULT.json filed (booking gate 4: census
           │         before the first Phase-2 dispatch).
           ▼
[3] Phase-2 wave (ONE wave, three members, max 2 concurrent — disk):
      W1 SYSTEM (collapsed with KRAWCZYK3)  ─┐ parallel
      W2 TRACE                              ─┘
      W3 RESIDUAL  — dispatches when a slot frees
           │
           ▼  integration in dependency order (W1 → W2 → W3),
              fast cargo check between merges,
              ONE full verify at the composed HEAD.
```

- **DISPATCH-2** (cone/torus special-position arms) and the **recognizer
  family (DISPATCH-3)** are decided AFTER the wave, from
  `docs/CERTIFIED_SPLINE_CENSUS.md` + the wave's RESIDUAL numbers. Neither
  blocks the wave; the wave does not book them. The booking's own language
  already says the census makes either outcome a win.
  - **Owner amendment, session 49 (second): the census gate is WAIVED.**
    After ~4h of census worker time the owner cancelled the measurement
    (row BLOCKED, WIP archived) and directed: build the specced wave
    directly. The census may be re-run later as a cheap measurement; it
    gates nothing. DISPATCH-2/recognizer decisions will be made from the
    RESIDUAL numbers and geometric naturalness instead.
  - **Owner amendment, session 49:** the corpus-mass rule is demoted from
    a gate to an ordering device. Everything on this program is built
    speculatively — the corpus is a 38-file proxy for the domain, not
    demand. Geometrically natural recognizers are worth building
    regardless of measured mass; the census's counts only RANK the arm
    order and size the win. Never again cite corpus mass as a reason NOT
    to build a capability.
- The **FLOOR anomaly** (4,381 certified_disjoint on adjacent pairs; the
  dispatch's screens vs the census's adjacency enumeration disagreeing
  about what a pair IS) is a Phase-1 dispatch/census disagreement. It is
  NOT wave work and is NOT folded into the spline census packet. It stays
  an open owner decision with its STOP filing as the record.
- **Census placement decided: census-first as a normal measurement packet**
  (booking gate 4 is explicit: "Runs BEFORE DISPATCH-2 spends and before
  the Phase-2 first dispatch"). It runs in parallel with the shim only
  because its write set is disjoint and the pagefile is now
  size-restrained; its RESULT gates the wave, so the gate is honored
  regardless of which worker finishes first.

## The shim's ride-along correction

`BG-CK-P2-CONTRACT` dispatches as authored (commit 37b0267's doc), with two
bookkeeping amendments only: (a) the registry row is written AT DISPATCH
(the session-47/48 rule), and (b) the row note records the session-49
correction so no future session re-trusts the "landed at 37b0267" claim.

## Write-set pre-matrix (run BEFORE dispatch; wave-mode law)

| pair | W1 SYSTEM(+K3) | W2 TRACE | W3 RESIDUAL | census | shim |
|------|----------------|----------|-------------|--------|------|
| W1 SYSTEM(+K3) | — | lib.rs one-line (expected textual conflict) | disjoint | disjoint | consumes shim types (frozen contract) |
| W2 TRACE | see left | — | disjoint | disjoint | consumes shim types (frozen contract) |
| W3 RESIDUAL | disjoint | disjoint | — | disjoint (different test/doc files) | consumes shim types via dev-dep |
| census | disjoint | disjoint | disjoint | — | disjoint |
| shim | defines W1/W2/W3 contracts | " | " | " | — |

- W1 writes `vendor/truck/truck-certified/src/ssi.rs` (NEW) + one
  `pub mod ssi;` line in lib.rs + `tests/ssi_system.rs` (NEW).
- W2 writes `vendor/truck/truck-certified/src/ssi_trace.rs` (NEW) + one
  `pub mod ssi_trace;` line in lib.rs + `tests/ssi_trace.rs` (NEW).
- W3 writes `tests/certified_phase2_floor.rs` (NEW, root) +
  `docs/CERTIFIED_PHASE2_FLOOR.md`. Root `Cargo.toml` needs NOTHING (the
  FLOOR r2 dev-dep edge is landed).
- Census writes `tests/certified_spline_census.rs` (NEW, root) +
  `docs/CERTIFIED_SPLINE_CENSUS.md`.
- Shim writes `ssi_types.rs`, `ssi_fixtures.rs`, lib.rs two mod lines,
  `tests/ssi_contract.rs` (all per its packet).

**Collapse decision (booking pre-made decision 6's escape hatch, invoked):**
SYSTEM and KRAWCZYK3 collapse into ONE packet and ONE branch, W1. Reason:
wave mode forbids two workers designing the same new file, and both
book `src/ssi.rs` as their write set — the 2D Krawczyk inner loop's
dimension-raised certificate is a continuation of the system constructor's
module by the booking's own text ("same module"). KRAWCZYK3 is never
registered as a separate row; its booked content is W1's Sections 2–3 and
this spec is the record. TRACE does NOT collapse into W1: it writes a
DIFFERENT new module (`ssi_trace.rs`), its contract dependency (shim types)
is frozen, and its seed/step needs are met by `ssi_fixtures` during the
wave — production substitution happens at integration.

## Concurrency decision (disk and cache, per wave-mode law) — REVISED BY MEASUREMENT

Measured at open: 32.6 GB free; ALL cargo targets cold (watchdog reclaimed
repo-root and slot targets); pagefile size-restrained (6 GB initial /
12 GB max, automatic management off — the session-46 blocker is gone);
sccache NOT installed at session open → **installed first** (wave-mode law:
"either install it first or run fewer workers"), `RUSTC_WRAPPER` set for
every worker spawn (worker spawn inherits the orchestrator environment),
per-worker `CARGO_TARGET_DIR`s as the slot machinery already provides.

**REVISED at first dispatch (session 49, measured twice): the binding
constraint is RAM, not disk.** The machine has 15.7 GB total RAM; with
ambient load (three opencode sessions ~4.6 GB, chrome) a single
default-parallelism `cargo check --workspace --all-targets` OOMs
(`rustc-LLVM ERROR: out of memory`, `STATUS_STACK_BUFFER_OVERRUN` cascade
on proc-macro crates — first the slot-0 warm at default jobs, then a
2-job warm CONCURRENT with the dispatched shim worker). Conclusions,
both machine-checked:

1. **The pre-wave and wave stages run ONE worker at a time** (warm
   builds included) until RAM frees. The wave's parallelism collapses to
   1-concurrent on this machine today; the wave-mode STRUCTURE (frozen
   contracts, one authoritative verify, amendment-not-redispatch) is
   unchanged — only the wall-clock parallelism is given up. If the owner
   frees ≥4 GB ambient RAM (close browser/editor sessions), 2-concurrent
   can be re-attempted with `CARGO_BUILD_JOBS=2` per slot and the
   watchdog watched; the disk budget (32 GB) is NOT the limiter.
2. **`CARGO_BUILD_JOBS` is capped for every cargo invocation** (2 for
   warm builds; ≤4 for a lone worker's inner loop). Never leave it at
   default on this machine.

A warm slot target measured 0.9 GB (dev `cargo check` profile) — the
old 10–12 GB estimate was release/test-tree shape; disk headroom is
comfortable either way. No worker ever runs the global gates; the
prewarm happens once per slot fork (`new_slot.py` default warm build).

## Wave execution rules in force (verbatim carriers)

LOCAL_GREEN is not DONE; RESULT.json is a claim; `verify.py` is the only
acceptance authority; no wave worker runs global gates; all wave branches
fork from the SAME base (the shim's landing merge SHA); rebase-only on
packet branches; integration in dependency order with fast
`cargo check -p truck-certified` between merges; seam mismatches go back
to the owning worker's session via `--resume` as amendments, never new
packets; the full verifier runs ONCE at the composed HEAD; registry rows
flip to DONE only after it passes.

## Wave manifest (filled at integration; the record of record)

- base SHA (shim landing merge): **a27edaa** ("merge: BG-CK-P2-CONTRACT
  (ACCEPTED, verified at 1b5ca21)"; filing commit 6932197; ceiling 111→111)
- shim BG-CK-P2-CONTRACT: packet authored 37b0267; r1 STOPPED (condition 3,
  filed loop/results/BG-CK-P2-CONTRACT.STOP.json — the packet's own false
  Section 3 premise, worker caught it, E0433 recorded); r2 amendment
  f2fca2c → resumed session landed the dev-dep edge + complete shim
  (worker commit 1b5ca21); one orchestrator H-3 one-liner amendment (V4);
  full verify ACCEPTED; **row DONE**
- W1 packet SHA / worker commit SHA: _TBD_
- W2 packet SHA / worker commit SHA: _TBD_
- W3 packet SHA / worker commit SHA: _TBD_
- census packet SHA / worker commit SHA: _TBD_
- integration amendments (worker, failure, fix): _TBD_
- verifier version + final integrated SHA: _TBD_
- concurrency actually used + disk low-water mark: 1-concurrent (RAM-bound,
  two OOM crashes recorded); CARGO_BUILD_JOBS=2–4; disk low-water ~26 GB;
  one transient STATUS_STACK_BUFFER_OVERRUN rustc crash during the shim
  worker's own check run (retried clean)
