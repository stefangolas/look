# Autoresearch loop: grinding down render time on UR10 / core_xy / formula1

Design date: 2026-08-14. Static audit of the existing harness; no builds or
benchmark runs were performed to produce this.

The short version: the loop itself is the easy part. Three things in the current
setup will make an autoresearch loop produce confident garbage, and they have to
be fixed before the first agent starts. They are in Phase 0.

---

## Phase 0 — Blockers. Do not start the loop until these are closed.

### B1. The entire baseline is marked untrusted

All 36 records in `benchmarks/corpus_regression_baseline.json` carry:

```
"untrusted": "recorded below the free-memory threshold"
```

`free_gb_before` across the run ranges 1.75–3.29 GB. There is no trusted
baseline to grind against. An agent loop pointed at this file would chase
memory-pressure noise and report wins, which is the documented failure mode on
this machine already (one 136s sample on a 5.5s workload).

**Close it by:** freeing space to a level where the harness's own threshold
stops firing, then re-recording. The evaluator in this design refuses to run
below threshold (see E3) rather than recording and flagging, because a flagged
record still ends up in a mean.

### B2. ~~UR10 does not render at all~~ — ALREADY CLOSED (2026-08-14)

Superseded by `UNSEEN_MODEL_PERFORMANCE_AUDIT.md` (commit `f1869aa`, 18:05) and
the truck bump in `1d29299`. The 600 s timeout in the baseline file is a **stale
record**, not a live blocker:

```
before   >90 s direct, >600 s corpus timeout
after    wall 3.2 s, step_parse 143 ms, step_table 138 ms,
         step_tessellate 2,173 ms, total 3,166 ms, peak ~396 MB, 502,130 tris
```

Root cause was two B-spline faces (#88144, #89705) meshing an artificial
declared-range rectangle and spending ~99% of tessellation time on a
super-linear constraint explosion; fixed by promoting the face-local
`working_range` to the production synthetic-closure domain in
`PolyBoundary::new_with_join`. No faces lost — UR10's 8 missing faces are the
same 8 `EdgeTraversalUnresolved` R01 refusals as before.

**UR10 enters `ACTIVE_MODELS` from the start.** What this does change is B1:
the baseline file is now stale as well as untrusted, which strengthens the
case for re-recording rather than repairing it.

### B3. The harness discards 24 of 27 timing stages

`look` records 27 stages. `occt_corpus_inner.py` keeps three (`step_parse`,
`step_table`, `step_tessellate`) because it was written to compare against
OCCT's equivalent phases. For core_xy that means:

| | ms |
|---|---:|
| `total` | 2416.1 |
| `step_parse` | 48.8 |
| `step_table` | 56.6 |
| `step_tessellate` | 718.4 |
| **recorded but discarded** | **~1592 (66%)** |

formula1 is the same shape: 1348 of 2967 ms in tessellation, the rest dropped.

The dropped stages exist and are already emitted — `gpu_adapter`, `gpu_device`,
`gpu_upload`, `gpu_render`, `gpu_readback`, `png_encode_write`, `compile`,
`hash`, `read`, and others.

**But the conclusion is not "there is hidden work to optimize."** Most of that
gap is GPU adapter/device init, which is a **model-independent constant** — the
audit notes core_xy's control wall rose from 2.42 s to 4.96 s between runs
purely because "the GPU adapter init lands on the measured wall," with no
change to the model or the code. It is not worth optimizing: it is invariant to
exactly the geometry work this loop exists to improve, and it is large enough to
swamp the 2–5% deltas the loop will be chasing.

**Close it by:** recording the full stage map, then defining the objective over
the model-dependent stages only (see below). The point of the instrumentation is
to *exclude* the constant, not to chase it.

---

## The objective function

One scalar, defined once, computed by the harness and never by the agent.

```
cpu_ms(model) = step_parse + step_table + step_tessellate
score = geometric_mean( min_of_N cpu_ms(model) for model in ACTIVE_MODELS )
```

`ACTIVE_MODELS = {core_xy, formula1, ur10}` — all three from the start, since
B2 is closed. Geometric mean so a 46 MB model does not dominate a 9 MB one.

**Score on the model-dependent CPU stages, not on `wall_ms`.** GPU adapter and
device init are a large model-independent constant that has already been
observed moving a control wall by 2.5 s with no code change (B3). Including it
adds variance without adding signal and will drown the effect sizes this loop
is chasing. `wall_ms` and `peak_mb` are still **recorded** every run and are
still gated for blowups — they are just not the thing being minimized.

On current numbers the objective is roughly: core_xy ~2.30 s, formula1 ~2.03 s,
ur10 ~2.45 s. Comparable magnitudes, and every millisecond in them is work the
loop can actually influence.

Sampling, per the standing rule on this machine: **alternate configurations
A/B/A/B and take the minimum of >= 5 reps.** Not the mean, not batched. Batching
all reps of A then all of B has already produced a fabricated 2.6x here. The
existing `benchmarks/tileset-f3d.ps1` already implements alternation and
conditioning launches correctly — reuse its structure rather than writing a new
sampler.

### Gates — the part that stops the agent cheating

A candidate is **INVALID**, not "worse", if any gate fails. Invalid results are
never comparable to the baseline and must never enter the score. Every one of
these is a way to make rendering faster by rendering less:

| Gate | core_xy | formula1 | ur10 |
|---|---|---|---|
| `exit == 0` | 0 | 0 | 0 |
| faces lost must not increase | 13 of 5670 | 142 of 5235 | 8 of 6048 |
| triangle count (see warning) | 406,096 | 365,809 | 502,130 |
| no new warning kinds | 3 | 2 | — |
| `peak_mb` blowup gate | ~422 MB | ~402 MB | ~396 MB |
| output PNG perceptually unchanged | — | — | — |

**The face-loss gate is the real one, and the triangle gate must not be
strict.** This repo's whole history is face recovery; a change that speeds up
tessellation by dropping 200 faces would score as a large win against time
alone. Faces lost is the invariant that catches that.

Triangle count is *not* such an invariant, and a ±0.1% gate on it would have
been actively wrong. The face-local closure fix deliberately moved core_xy from
668,351 to 406,096 triangles (−262k) by removing artificial geometry between
the true trim boundary and the carrier's declared rectangle — a correctness
improvement that a tight triangle gate would have rejected as INVALID. The same
class of fix may well recur. Treat triangle count as a **wide** blowup gate
(flag order-of-magnitude changes for human review) rather than an equality
check, and let faces-lost plus the PNG comparison carry the correctness load.

All numbers above are post-fix values from `UNSEEN_MODEL_PERFORMANCE_AUDIT.md`,
recorded on single runs. Re-derive them from the first trusted multi-rep
baseline before wiring them in.

For the PNG gate, reuse whatever pixel comparison already backs the F3D
comparisons in `docs/BENCHMARKS.md` (foreground bounds + linear-RGB RMSE); the
threshold should be tight enough to catch dropped geometry and loose enough to
survive nondeterministic rasterization order.

---

## The evaluator contract

The agent must never measure anything itself. It calls one command and reads one
small JSON object. This is what keeps the loop honest and the context small.

```
benchmarks/autoresearch/evaluate.ps1 -ExperimentId <id>
```

**E1.** Builds the current tree, `--release`, and fails loudly if the build
fails. Uses `cargo build --release` — see `BUILD_SPEEDUP_PLAN.md`; the recorded
`--target x86_64-pc-windows-gnullvm` invocation is stale and forces a second
proc-macro build.

**E2.** Refuses to run if `.cargo/config.toml` has a live `paths` override,
unless explicitly invoked with `-AllowLocalTruck`. A measurement taken through
the override is not a measurement of anything pushed; that has already corrupted
one manifest here for 37 commits.

**E3.** Refuses to run — exit non-zero, no record written — if free disk or free
RAM is below threshold. Refuse, do not flag. B1 exists because flagging was not
enough.

**E4.** Runs the alternating min-of-5 sampler over `ACTIVE_MODELS`, capturing
the **full** stage map per run.

**E5.** Evaluates every gate. Emits exactly one JSON object, and appends one row
to the ledger:

```json
{"id":"E017","score":2712.4,"delta_pct":-3.1,"valid":true,
 "gates":{"exit":"pass","triangles":"pass","faces":"pass","warnings":"pass","png":"pass"},
 "per_model":{"core_xy":{"wall":2455.1,"stages":{...}},"formula1":{...}},
 "verdict":"IMPROVEMENT"}
```

`verdict` is one of `IMPROVEMENT`, `NEUTRAL`, `REGRESSION`, `INVALID`,
`BUILD_FAILED`, `REFUSED`.

**E6.** Prints a summary of at most 30 lines. The agent reads that. The full
record goes to disk. This is the primary context-budget control.

---

## Durable state: the ledger and STATE.md

The core discipline: **nothing the loop knows lives only in a context window.**
An agent that is about to be restarted has already written everything down.

```
research/
  STATE.md          # small, rewritten every session — the handoff
  LEDGER.jsonl      # append-only, one row per experiment, never rewritten
  experiments/E017/ # diff, full JSON, stderr, rendered PNGs
```

`STATE.md` is capped at roughly 100 lines and holds only:

- current best score + the commit/diff that produced it
- the hypothesis queue, ranked, with a one-line rationale each
- dead ends: hypothesis, what was measured, why it is dead — so a later session
  does not re-run it
- the invariants an agent must not violate (see below)

`LEDGER.jsonl` is the full history and is **never** read whole by an agent. A
session reads `STATE.md` plus the last 5 ledger rows. That bound is what keeps
the context flat as the experiment count grows.

Every experiment is a git branch or stash entry, so any candidate can be
reverted or re-measured. Nothing is committed to `main` by the loop.

---

## Session restart mechanics

This is the part you specifically asked about, and the honest answer is that
`/loop` is the wrong primitive for it. `/loop` continues one session — context
grows monotonically, which is the failure you want to avoid.

**Use an external orchestrator that spawns a fresh headless process per
experiment.** One experiment, one process, one context that starts empty and is
discarded:

```powershell
# benchmarks/autoresearch/orchestrate.ps1  (sketch)
for ($i = 0; $i -lt $MaxExperiments; $i++) {
    claude -p (Get-Content prompts/experiment.md -Raw) --max-turns 40
    $v = (Get-Content research/last_verdict.json | ConvertFrom-Json).verdict
    if ($v -eq 'REFUSED') { break }   # E2/E3 tripped: a human must look
    if ($v -eq 'BUILD_FAILED') { $fails++ } else { $fails = 0 }
    if ($fails -ge 2) { break }
}
```

Context reset is then a property of the architecture rather than something the
agent has to be trusted to manage. `prompts/experiment.md` is fixed and does not
accumulate: it says read `STATE.md`, read the last 5 ledger rows, pick the top
hypothesis, implement it, call `evaluate.ps1` once, write the ledger row, rewrite
`STATE.md`, stop.

**One experiment per session.** Not three. The temptation is to amortize the
4–5 minute build across several experiments per session, but multi-experiment
sessions are exactly where context grows and where an agent starts reasoning
about its own earlier reasoning instead of the measurements. Fix the build cost
in `BUILD_SPEEDUP_PLAN.md` instead — that is the right lever, and the two plans
compose.

### Anti-spin-out rules (encode in `prompts/experiment.md`)

- **Turn cap** per session (`--max-turns`), so a stuck agent dies rather than
  thrashing.
- **One `evaluate.ps1` call per session.** Multiple calls mean the agent is
  tuning against the benchmark interactively, which is overfitting to
  measurement noise on a machine with documented 15–23 s outliers.
- **Two consecutive `NEUTRAL`/`REGRESSION` results on one hypothesis branch**
  → the agent must write a dead-end entry and move to a different branch. This
  is the anti-rabbit-hole rule.
- **Never read** `LEDGER.jsonl` whole, the corpus JSON whole, or any
  `benchmarks/*_report.py` output whole.
- **Forbidden actions:** committing to `main`; editing `.cargo/config.toml`;
  deleting `target/research` (holds the Sponza and NYC models); relaxing a gate;
  editing the baseline file. Relaxing a gate is the most likely form of
  reward hacking here and should be called out explicitly.
- **On `REFUSED`, stop the whole loop.** E2/E3 refusals mean the environment is
  wrong, and every subsequent measurement would be garbage.

---

## Seeded hypothesis queue

Ranked by expected value given what the current data actually shows. The first
entry is not an optimization.

Tessellation is now unambiguously the target: it is 2,116 / 1,541 / 2,173 ms on
core_xy / formula1 / ur10, against parse+table totalling 180–490 ms. There is no
longer a mystery bucket to chase.

1. **Instrument and re-baseline (B1 + B3).** Full stage map, trusted conditions,
   min-of-N. Cheap, and everything downstream depends on it.
2. **Tessellation parallelism.** The dominant term in all three models. `rayon`
   is already a dependency and per-face tessellation is the natural unit. The
   audit's own numbers say UR10 averages ~4 ms/face across ~6,000 faces of
   sequential CPU — that is close to an ideal parallel workload. This is the
   single highest-value item and is well suited to an agent loop: many small
   independent changes, each cheaply measured against a hard gate.
3. **The remaining super-linear tail.** The face-local closure fix removed the
   two pathological faces, but the audit records the underlying growth curve —
   96 ms at 2,604 constraints → 15.8 s at 19,722 → >100 s — as a property of the
   CDT path, not of those two faces. Other models will hit it. Worth a
   constraint-count ceiling with a typed refusal rather than a hang.
4. **Parse and table.** 267 + 219 ms on formula1, the largest of the three.
   Real, but an order of magnitude below tessellation.
5. **Not in this loop:** GPU init. Model-independent constant (B3). If cold-start
   latency matters for small models, that is the session/atlas path the tileset
   harness already exercises, and it is a separate piece of work.

The audit's two open follow-ups — `DEGENERATE_TOROIDAL_SURFACE` (formula1's 142
faces) and `EdgeCurveConversionFailed`/`AllBoundsCollapsed` (core_xy's 13) — are
face-recovery work, not speed work. They will *raise* the objective by meshing
more geometry. Keep them out of this loop or they will fight the score.

---

## Where I am uncertain

- **Whether the loop is worth building for one hypothesis.** With UR10 fixed and
  the GPU constant excluded, the queue is short and item 2 dominates it.
  Tessellation parallelism is a design decision plus a correctness argument
  about shared state, which a human does better in an afternoon than a loop does
  in fifty iterations. The honest read: build the *evaluator* (Phase 0 + the
  gates) first regardless — it pays for itself as a regression harness — and
  decide on the agent loop only once there are enough independent hypotheses to
  justify the orchestration. An evaluator with hard gates is the durable asset
  here; the loop around it is optional.
- **Whether per-face tessellation is actually parallel-safe.** I have not read
  the tessellation path. Shared mutable state, the `catch_unwind` containment
  that PLANAR-C depends on, and per-face diagnostic ordering could each
  complicate it. Check before promising the win.
- **The PNG gate threshold.** I do not know how deterministic `look`'s output is
  run-to-run on this GPU. If it is bit-identical, the gate is a hash and this is
  easy; if not, the threshold needs calibrating against repeated identical runs
  before it can distinguish "dropped geometry" from "normal jitter". Calibrate
  before trusting it.
- **The min-of-5 sample count.** Inherited from the existing rule of thumb. With
  documented 15–23 s outliers, 5 may be too few to resolve the 2–5% deltas this
  loop will be chasing. Worth checking the spread on the first trusted baseline
  and raising the count if the noise floor exceeds the effect size — otherwise
  the loop will manufacture wins.
- **`claude -p` flag names** in the orchestrator sketch above are from memory and
  should be checked against `claude --help` before use.
