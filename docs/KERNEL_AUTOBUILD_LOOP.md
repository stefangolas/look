# Kernel autobuild loop — outer loop over subagents for the BG- build spec

Design date: 2026-08-16. Static design against
[`GENERATION_KERNEL_BUILD_SPEC.md`](GENERATION_KERNEL_BUILD_SPEC.md) (rev synced
to formal system r4) and [`FORMAL_SYSTEM_BREP_GENERATION.md`](FORMAL_SYSTEM_BREP_GENERATION.md).
No builds were run to produce it.

## 0. What this loop is, and how it differs from the perf loop

`AUTORESEARCH_LOOP_PLAN.md` is a **search** loop: the objective is a scalar
(`cpu_ms`), the hypotheses are guesses, and the danger is reward hacking against
a noisy measurement.

This is a **build** loop. The objective is discharging named contracts
(`BG-S0-001` … `BG-FID-008`) whose acceptance test is *mechanical and
deterministic*: the spec already states the algorithm, the anchors, the
contract, and the required tests per item. There is no measurement noise and no
score to game. What replaces the score is a **verifier**, and what replaces
"pick the top hypothesis" is a **packet DAG** derived from §9 of the build spec.

That difference drives every design choice below:

- workers are cheap, mechanical, and interchangeable → **deepseek v4 flash**;
- the scarce resources are **spec clarity**, **disk**, and **build wall-time**,
  not model capability;
- the loop's second output — as valuable as the code — is a stream of
  **spec-gap reports**, because a packet an agent cannot execute unambiguously
  is a defect in the spec, and the spec is the artefact we are actually
  finishing.

---

## 1. Architecture

```
                 ┌─────────────────────────────────────────────┐
                 │ orchestrate.ps1  (one long-lived process)    │
                 │  • reads loop/PACKETS.jsonl (the DAG)        │
                 │  • disk janitor + free-space floor           │
                 │  • dispatches ready packets to free slots    │
                 │  • merges ACCEPTED packets, appends LEDGER   │
                 └───────┬──────────────┬──────────────┬────────┘
       slot-0            │   slot-1     │   slot-2     │        (3 concurrent)
  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐
  │ git worktree   │ │ git worktree   │ │ git worktree   │
  │ CARGO_TARGET_  │ │ …/slot-1       │ │ …/slot-2       │
  │  DIR=slots/0   │ │                │ │                │
  │ claude -p      │ │                │ │                │
  │  (deepseek)    │ │                │ │                │
  │  + packet.md   │ │                │ │                │
  └───────┬────────┘ └───────┬────────┘ └───────┬────────┘
          │ writes RESULT.json + diff           │
          ▼                                     ▼
  ┌─────────────────────────────────────────────────────┐
  │ verify.ps1 <slot>   — the only acceptance authority │
  └─────────────────────────────────────────────────────┘
          │ ACCEPTED → merge         │ BLOCKED/SPEC_GAP →
          ▼                          ▼
     integration branch        loop/questions/<packet>.md
                                     │
                                     ▼   (batched, one Claude session per wave)
                               arbiter: amends the spec, re-emits the packet
```

One packet per process. Context reset is a property of the architecture, not of
agent discipline — the same rule that made the perf loop honest, and here it is
also what keeps deepseek under its context ceiling.

**"Sticky slots" defined.** A *slot* is a fixed, long-lived directory pair — one
git worktree plus one `CARGO_TARGET_DIR` — that is **reused across many
packets**. Between packets only the branch checked out inside it changes; the
directories are never deleted.

The alternative (a fresh worktree per packet) means every packet starts with an
empty target dir and compiles all 240 dependency crates before it can run one
test: several minutes and ~2.5 GB of writes, paid 23 times in wave W4 alone.
With a sticky slot that cost is paid **once per slot**, and each subsequent
packet recompiles only the crate it touched. It is the single assumption behind
the "~20 min per packet" figure in §6; without it the loop is dominated by
`cargo` and the parallelism buys nothing.

The cost of stickiness is contamination — a slot carries whatever the previous
packet left behind. That is why `git worktree` is the isolation unit (the
orchestrator hard-resets the branch between packets) and why V1 gates on
`git diff --name-only` rather than trusting the worker's account of what it
changed.

---

## 2. Model wiring — `opencode run` with deepseek v4 flash

**Harness: `opencode run`, not `claude -p`.** Resolved 2026-08-16 — opencode is
already installed and authenticated for deepseek, and its `run` subcommand has
exactly the four things a worker needs:

```powershell
opencode run --dir $slotPath `
             -m deepseek/deepseek-v4-flash `
             --agent kernel-worker `
             --format json `
             --auto `
             (Get-Content $packet -Raw)
```

- `--dir` **is** the slot mechanism: the worker runs inside the slot worktree.
- `--format json` gives machine-readable events for the ledger.
- `--agent` selects a config-defined agent, which is where the tool allowlist
  and permission restrictions live (`opencode.jsonc` `permission: {edit, bash}`).
- `--auto` auto-approves non-denied permissions, so the run is unattended.
- `--pure` is available if plugin noise becomes a problem.

**Credentials — already present, nothing to provision.** The deepseek key is in
`~/.local/share/opencode/auth.json` (`type=api`, 35 chars, `sk-0a3…a691`),
alongside a `zai` key. No `DEEPSEEK_API_KEY` env var is set and none is needed:
opencode reads its own auth store. The earlier `claude -p` +
`ANTHROPIC_BASE_URL` sketch is dropped — it existed only to reach deepseek
through an Anthropic-compatibility shim, and going native avoids the shim
entirely.

**Model id confirmed: `deepseek/deepseek-v4-flash`.** `opencode models` also
lists `deepseek/deepseek-v4-pro`, `deepseek/deepseek-chat`,
`deepseek/deepseek-reasoner`, and — worth a look before the wide waves —
`opencode/deepseek-v4-flash-free`, which would run W4's 23 packets at no API
cost if its rate limits tolerate three concurrent workers.

**What this costs us:** Claude Code's hooks. The `PostToolUse` token ledger and
the `PreToolUse` hard cap in §3 were a Claude Code mechanism and do not exist
here. See §3 — with the real context window they matter far less than they did
under the assumed one.

### Division of labour: Claude orchestrates, deepseek codes

This is the intended shape and it is worth stating as a rule rather than leaving
it implicit in the diagram. **deepseek never plans.** It receives one packet, in
one process, and its entire decision space is *how to write the code the packet
already describes*. It does not choose what to work on, does not read the DAG,
does not decide whether its own work is acceptable, and does not amend the spec.

Claude owns everything that requires judgement:

| step | who | why it cannot be deepseek |
|---|---|---|
| packet generation | Claude | picking anchors, computing the write allowlist, and sharding a wide item (BG-TOL-001's 184 sites) *is* the design work |
| dispatch / scheduling | orchestrator script + Claude | mechanical from `depends_on`, but wave boundaries and re-emission after a SPEC_GAP are judgement |
| the coding packet | **deepseek v4 flash** | transcription + tests against a template; ~85% of packets |
| verification | `verify.ps1` (deterministic), Claude on failure | the gates are scripts; adjudicating *why* V5 failed is not |
| the arbiter turn | Claude | resolving an ambiguity and amending the spec is the loop's actual product |

Claude-as-orchestrator can be a headless loop or an interactive session (this
one) driving `orchestrate.ps1` a wave at a time. Start interactive: the first
two waves are where the packet template is still wrong, and you want a human
reading the SPEC_GAP files as they appear rather than after 23 of them.

**Model assignment is per packet class**, declared in the packet DAG:

| class | who runs it | why |
|---|---|---|
| `mechanical` | deepseek v4 flash | algorithm is given verbatim; the work is transcription + tests. ~85% of packets. |
| `wide-mechanical` | deepseek v4 flash | many call sites, one judgement rule (BG-TOL-001, BG-CE-001 migration). Needs a *sharding* rule, not intelligence. |
| `design` | Claude (opus/sonnet) | the packet would have to invent a shape: BG-EVD r3 amendment, the `Surface` enum unification, BG-FID-001/003/005. |

Do not let a `design` packet leak into the deepseek pool. The tell is in the
spec itself: if the item's body contains a `rust` block that *is* the answer, it
is mechanical; if it contains prose about what must not be conflated, it is not.

---

## 3. Context budget — keeping a worker under 40%

**The context window is 1M, not the 128k this section originally assumed.** That
changes the problem rather than shrinking it:

| threshold | tokens (1M) | note |
|---|---|---|
| soft warn | 30% ≈ 300k | |
| close-off | 36% ≈ 360k | |
| hard cap | 40% ≈ 400k | |

**No packet built to the §4 envelope will come close to 400k.** A packet is one
crate, ≤5 files, ≤600 changed lines, with the spec item inlined — call it 5–15k
of prompt and, with cargo output kept out of the transcript, well under 50k at
exit. The 40% ceiling stops being a survival constraint and becomes a **runaway
detector**: a worker approaching even 100k has stopped doing the packet and
started exploring, and the useful response is to kill it, not to warn it.

So the thresholds stay, their purpose changes, and the mechanism changes with
it. Since opencode gives us no `PreToolUse` hook to deny on, enforcement moves
outward to the orchestrator: parse the `--format json` event stream, and abort
the worker on whichever comes first — cumulative tokens past 100k, 40 assistant
turns, or a wall-clock cap. That is coarser than a hook and entirely sufficient
for a detector.

The reason to keep packets tight is now **quality and cost, not survival**.
A flash-class model given 400k of context does not use it well, and every token
is billed; the structural rules below are what keep a packet at 15k instead of
150k, and they are worth just as much as before.

Enforcement is two mechanisms, and the first matters more:

**(a) Structural — the packet is self-contained.** A worker never opens the
1,432-line build spec or the 903-line formal system. The packet generator
**inlines** the item body verbatim (algorithm block, contract text, test list),
so the entire spec cost is ~80–200 lines inside the prompt. This is the single
biggest lever: an agent that greps the spec for context will burn 20k tokens
before it writes a line. The packet states explicitly: *the spec is not on your
allowlist; everything you need is in this document; if something is missing that
is a SPEC_GAP, not a research task.*

Additional structural rules baked into every packet:
- `Read` only files named in `anchors` or `template`, and only with
  `offset`/`limit` when the file exceeds 400 lines.
- Locate with `rg -n <pattern> <file>` (the packet supplies the pattern and the
  expected hit count — H-8), never by reading the file to find the symbol.
- Build output goes to a file: `cargo test -p X --lib 2>&1 | Tee-Object
  loop/out.txt`, then read only the tail. Never paste a full cargo log back.
- Packet size cap: **one crate, ≤5 files, ≤600 changed lines**. A packet that
  cannot be stated inside that envelope must be split by the generator.

**(b) Mechanical — the orchestrator watches the event stream.** It reads
opencode's JSON events, accumulates reported token usage per worker, and kills
the process on the runaway conditions above, recording `CTX_EXHAUSTED` in the
ledger. This is a supervisor, not a hook: it cannot warn the worker or shape its
behaviour mid-run, only end it. Given that no compliant packet approaches the
threshold, an abort is the correct response anyway.

---

## 4. The work packet

One file, generated by `loop/gen-packet.ps1` from `PACKETS.jsonl` + the spec
item. Schema:

```yaml
id:            BG-ENC-002-CYL
contract:      [BG-ENC-001, BG-ENC-002]     # what this discharges; goes in the commit msg
class:         mechanical
crate:         vendor/truck/truck-evidence
depends_on:    [BG-EVD-r3, BG-CE-006-CYL]
write_allow:   [src/cylinder.rs, src/lib.rs, tests/cylinder.rs]   # ENFORCED by verify
read_allow:    [src/plane.rs, src/enclosure.rs, src/harness.rs, ../truck-geometry/src/specifieds/cylinder.rs]
template:      src/plane.rs                  # copy its shape exactly
anchors:
  - file: src/enclosure.rs
    symbol: trait EnclosureSurface
    rg: 'fn normal_cone'
    expect: 1
house_rules:   [H-1, H-2, H-3, H-5, H-6, H-7]     # inlined verbatim, not referenced
spec_body:     <verbatim §3 BG-ENC-002 text, incl. the sin/cos interval warning>
tests_required:
  - "property: 10^4 sampled points in a random box all lie in enclose(box)"
  - "property: BG-ENC-002 convergence under bisection (harness::assert_converges)"
  - "unit: an interval spanning pi/2 encloses sin = 1"
done_when:     # exact commands; the worker runs these, verify.ps1 re-runs them
  - cargo fmt --check -p truck-evidence
  - cargo clippy -p truck-evidence --all-targets -- -D warnings
  - cargo test -p truck-evidence --lib --tests
  - bash scripts/kernel-gates.sh $BASE
stop_conditions:
  - anchor hit count != expect            -> ANCHOR_MISMATCH, do not patch around it
  - a required test cannot be written without inventing a rule -> SPEC_GAP
  - 3 consecutive failed `cargo test` runs on the same error   -> BLOCKED
budget:        {turns: 40, ctx_pct: 40}
```

The worker's terminal act, in every outcome, is to write
`loop/slots/<n>/RESULT.json`:

```json
{"id":"BG-ENC-002-CYL","status":"DONE","commit":"a1b2c3d",
 "contracts":["BG-ENC-001","BG-ENC-002"],"tests_added":4,
 "notes":"cos over [0,2pi) needed explicit k*pi/2 extrema; see cylinder.rs:enclose"}
```

`status ∈ {DONE, ANCHOR_MISMATCH, SPEC_GAP, BLOCKED, CTX_EXHAUSTED}`. Anything
that is not `DONE` also writes `loop/questions/<id>.md` with: what was
attempted, the exact ambiguity, and the two or more readings the agent could not
choose between. **That file is the loop's research output.**

### Why the packets are this rigid

The spec is unusually well-suited to this because it already does the hard part
— every item names its anchors with expected hit counts, its contract, and its
tests, including the *negative* tests that catch the specific wrong
implementation. Three things the packet generator must add, because the spec
leaves them implicit:

1. **The write allowlist.** The spec says what to change, not what not to. Two
   parallel workers editing `enclosure.rs` is the main way this loop breaks.
2. **The verbatim template pointer.** P-6 says "point the agent at it"; the
   packet must name the file and say *copy its structure exactly, including the
   test module layout*.
3. **The `done_when` command list.** The spec's house rules H-1/H-3/H-4 are
   enforced by `scripts/kernel-gates.sh`, but a worker will not run it unless
   told. Put it in `done_when` and re-run it in `verify.ps1`.

---

## 5. Verification — the only acceptance authority

`verify.ps1 <slot>` runs in the slot worktree and is the analogue of the perf
loop's evaluator. It never trusts `RESULT.json`.

| gate | check |
|---|---|
| V1 scope | `git diff --name-only` ⊆ `write_allow`. Any other path → REJECT. |
| V2 build | `cargo check --locked -p <crate>` (workspace-wide check on the final packet of a wave). |
| V3 lint | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`. |
| V4 house rules | `scripts/kernel-gates.sh <base>` — H-1/H-3/H-4, diff-scoped, already in CI. |
| V5 tests | `cargo test -p <crate> --lib --tests`. Never a bare `cargo test`: 56 examples build otherwise. |
| V6 test-reality | `tests_added >= len(tests_required)`, **and** each required test's name appears in the diff. A packet whose tests all pass but whose negative test is absent is REJECTED. |
| V7 mutation spot-check | for items whose spec text names a negative test (BG-FID-008 double cover, BG-NUM-002 double root, BG-NUM-004 F-2 both directions, BG-CE-002 offset pcurve): re-run that one test against a deliberately weakened implementation and assert it **fails**. |
| V8 no-regression | the previously accepted wave's tests still pass. |

V6 and V7 exist because this spec's failure mode is not a broken build, it is a
**vacuous checker** — the document says so repeatedly ("a checker never
exercised against a violator is assumed broken"). A loop that only gates on
green tests will manufacture exactly that. V7 is expensive, so it runs only for
the enumerated items, and the weakened implementation is checked in beside the
test as a `#[cfg(test)] mod vacuity` harness rather than being generated on the
fly.

Anti-reward-hacking, encoded in the packet's forbidden list: never edit
`scripts/kernel-gates.sh`, `.cargo/config.toml`, `Cargo.lock`, another packet's
files, or any existing test; never add `#[ignore]`; never add `#[allow]` without
the justification comment the gate requires; never widen a tolerance to make a
test pass.

---

## 6. Parallelization plan

Derived from build-spec §9, split so that concurrently-running packets have
**disjoint write sets**. Concurrency 3 (disk- and CPU-bound, not API-bound).

| wave | packets | conc. | class | notes |
|---|---|---|---|---|
| W0 | infra: orchestrator, verify, hooks, packet generator | — | human/Claude | nothing runs before this |
| W0b | `BG-EVD-r3` (Modulus struct + `propagate` + `Pole` + `ForwardToleranceExceeded`) | 1 | design | P-6 amendment; **everything below types against it** |
| W1 | `BG-S0-002`, `BG-S0-003` | 2 | mechanical | different crates (`truck-shapeops`+`truck-geometry` / `truck-modeling`); independent of W0b. **BG-S0-001 itself is already landed — see §9.** |
| W2a | `BG-TOL-001-TYPE` (`ToleranceCtx` itself) | 1 | design | small, but it sets the model/param convention |
| W2b | `BG-TOL-001-{geom-specifieds, geom-nurbs, geom-decorators, topology, shapeops, meshalgo, modeling}` | 3 | wide-mechanical | 184 sites sharded by module; each packet gets the *rule* (model-space scales, dimensionless does not) plus its own site list generated by `rg` at packet-build time. Ambiguous sites → `FIXME(BG-TOL-001)` + listed in RESULT notes, never guessed. |
| W3a | `BG-CE-006-CYLINDER`, `BG-CE-006-CONE` | 2 | mechanical | new files under `specifieds/`, template `sphere.rs`/`torus.rs` |
| W3b | `BG-CE-006-ENUM` (delete truck-modeling's competing enum) | 1 | design | wide, breaking, must be alone in the tree |
| W3c | `BG-CE-001` (pcurve field), `BG-CE-003` (`Arc<Mutex>`→`Arc`) | 1 then 1 | design head + wide-mechanical tail | type change by Claude, ~200 call-site fixups by deepseek as a follow-on packet |
| **W4** | `BG-ENC-002-{LINE,CIRCLE,CYLINDER,CONE,SPHERE,TORUS}`; `BG-ANA-001-{PP,PS,PCYL,PCONE,SS,COAX,PARCYL,EQRCYL}`; `BG-INV-{101..109}` | 3 | mechanical | **the wide wave — 23 packets, one file each, one template each.** This is where the loop earns its keep. |
| W5 | `BG-ENC-003` (bspline, nurbs), `BG-ENC-004-{processor,revolved,extruded,pcurve,offset,isc}`, `BG-NUM-001`, `BG-NUM-002`, `BG-NUM-003` | 3 | mechanical (NUM-002/003: design head) | |
| W6 | `BG-FID-001`, `BG-NUM-004`, `BG-FID-003`, `BG-FID-008`, `BG-FID-005` | 1–2 | design | Claude writes the implementation; deepseek writes the test batteries as separate packets against the landed signature |
| W7 | `BG-TEST-001..008` global obligations | 3 | mechanical | each is an independent test file |

Two scheduling notes the spec supports and that matter here:

- **BG-ANA-001 needs only W0b+W2a+W3a** (spec §6: "runs in parallel, not
  downstream"). It can start as soon as W3a lands, in parallel with all of W4's
  enclosure packets, and it produces the **test oracle** for BG-NUM-003 later —
  so it is not just early capability, it de-risks W5.
- **Do not reorder W4 before W3** (enclosures against the old carrier set) or
  BG-FID-001 before the enclosures. That is the spec's own warning and the
  orchestrator enforces it via `depends_on`, not by convention.

Serial-equivalent estimate: W4 alone is ~23 packets × ~20 min ≈ 7.5 h serial,
≈ 2.5 h at concurrency 3 — assuming the build cost is amortized by sticky slots
(§7), which is the assumption that makes or breaks the throughput claim.

---

## 7. Disk — the binding constraint

Current: `C:` has ~20 GB free (df) against a stated working assumption of ~10 GB;
`target/` alone is 4.1 GB. Three worker slots each doing full builds would
exhaust the machine on the first wave.

### 7.1 What the loop needs, measured

| item | size | note |
|---|---:|---|
| 3 × slot `CARGO_TARGET_DIR` | **7.5 GB** | budget 2.5 GB each. Measured reference: the repo's full `target/debug` is 3.9 GB, `target/quick` 0.67 GB; slots run `--profile quick`, `CARGO_INCREMENTAL=0`, `-p <crate>` scoped |
| 3 × slot worktree source | **0.8 GB** | repo excl. `target/` is 386 MB; `git worktree` shares the object store, so this is working files only |
| shared `~/.cargo/registry` | 1.5 GB | already present, not additional |
| free-space floor (§7 rule 5) | **8 GB** | the orchestrator refuses to dispatch below this |
| | **≈ 16.5 GB free required at start** | |

Free space at design time: **19 GB**. That is nominally enough and practically
not — the janitor would be firing on the first wave and the floor would trip
mid-W4. §7.2 is therefore a precondition, not housekeeping.

### 7.2 Reclaimed 2026-08-16 — **executed: 18.9 GB → 43.9 GB free (25.0 GB)**

Everything in the table below except the last two rows has been deleted. The
overshoot against the 23.7 GB estimate is `proc-macro-srv*`: there were 41 such
dirs, not the 6 that exceeded the 200 MB reporting threshold.

| what | size | safety |
|---|---:|---|
| `~/.cargo/git/checkouts/truck-885f4dcc04f583da` — **56 per-rev checkouts** at ~115 MB each | **6.4 GB** | **dead.** The only `source = "git` line in `Cargo.lock` is ruststep; truck is vendored at `vendor/truck` (6.3 MB of source). Includes a checkout of `c5f4b6e`, the vendored rev itself. |
| remaining truck git cache (`truck-fork-*` checkout+db, `truck-f79b*`, `db/truck-885f*`, `resources4truck`) | 1.1 GB | same reasoning |
| `~/AppData/Local/npm-cache` | 4.4 GB | rebuildable (`npm cache clean --force`) |
| `~/AppData/Local/pip/cache` | 4.3 GB | rebuildable (`pip cache purge`) |
| `~/AppData/Local/Temp/proc-macro-srv*` (6 dirs) | 1.8 GB | rust-analyzer leftovers; safe with VS Code closed |
| `~/AppData/Local/Temp/claude` — 34 session dirs, oldest from January | 1.8 GB | stale sessions safe; keep the live one |
| `look/target/debug` | 3.9 GB | rebuildable, and the loop uses per-slot target dirs regardless. Keep `target/quick`. |
| **subtotal** | **≈ 23.7 GB** | → ~43 GB free |
| `~/AppData/Local/Temp/opencode` — `sgc` 588, `trackb_final` 527, `accuracy-census` 379, `phase_census2` 169 … | 2.9 GB | **review first, do not sweep.** These are named result directories from the census/corpus era, and the ABC corpus itself is already gone. |
| `~/truck-fork` (sibling working copy) | 136 MB | superseded by `vendor/truck`; the branch is pushed to `stefangolas/truck`. Low value, keep. |

Rules, in the orchestrator, not in the agents:

1. **`CARGO_INCREMENTAL=0` in every slot.** `target/debug/incremental` was
   389 MB in the last audit and buys nothing in a one-packet-per-process world.
2. **Sticky per-slot target dirs**, `loop/slots/<n>/target`, reused across
   packets. Budget ~2.5 GB each ≈ 7.5 GB steady state. They are the reason a
   packet costs 20 min and not 45.
3. **`--profile quick`** (already defined in `Cargo.toml`) for iteration; the
   release profile is never built by this loop.
4. **Packet-scoped commands only** — `-p <crate> --lib --tests`. A bare
   `cargo test` builds 56 examples in every slot.
5. **Free-space floor: refuse, do not flag.** Before dispatching a packet the
   orchestrator requires ≥ 8 GB free. Below that it runs the janitor; if still
   below, it stops the whole loop (`REFUSED`) rather than dispatching. This is
   the direct lesson from `AUTORESEARCH_LOOP_PLAN.md` B1/E3 — measurements taken
   under memory/disk pressure on this machine have already produced fabricated
   results once.
6. **Janitor ladder**, in order, stopping as soon as the floor is met: delete
   `**/incremental/`; `cargo clean -p <crate>` for the kernel crates in idle
   slots (deps survive); delete the least-recently-used slot target entirely;
   delete `loop/slots/*/out.txt` and old worktrees. Never touch `target/research`
   (Sponza/NYC models) — same forbidden-list entry as the perf loop.
7. **Disk is checked between packets, not during.** A mid-build ENOSPC is
   indistinguishable from a compile error to the worker, and it will "fix" it.

---

## 8. State, and what survives a restart

```
loop/
  PACKETS.jsonl        # the DAG: id, deps, class, write_allow, status
  LEDGER.jsonl         # append-only, one row per packet attempt, never rewritten
  STATE.md             # <=100 lines: wave in progress, blocked packets, open SPEC_GAPs
  questions/<id>.md    # the research output — one per non-DONE packet
  packets/<id>.md      # generated packet, kept for reproducibility
  slots/<n>/           # worktree, target dir, ctx.json, RESULT.json, out.txt
```

Nothing the loop knows lives only in a context window. A worker reads its packet
and nothing else; the orchestrator reads `PACKETS.jsonl` and the last N ledger
rows; the arbiter reads `questions/*.md` for the wave. No agent ever reads
`LEDGER.jsonl` whole.

Merging: each packet commits on `packet/<id>` in its slot worktree; the
orchestrator rebases onto `integration/kernel-bg` after V1–V8 pass. Commit
message names the discharged `BG-` ids, per the spec's standing rule. Nothing
lands on `main` from the loop.

**The arbiter turn.** At the end of each wave, one Claude session reads every
`questions/*.md` from that wave, decides each ambiguity, **amends the build
spec** (this is the point — the spec is the deliverable being finished), and
re-emits the affected packets into the next wave. This is the outer loop's
learning step; without it, a SPEC_GAP just stalls a packet forever.

---

## 9. Ground truth in the tree — where the spec has gone stale

Measured 2026-08-16 against the working tree, not against the spec's prose:

- **`BG-S0-001` is DONE and the spec has not been updated.** `Surface::include`
  in `vendor/truck/truck-modeling/src/geometry.rs` returns `Outcome<bool>`,
  `include_intersection_curve` exists, the `ssi-carrier` / `leader-witness`
  certificates are in the tree, and `boolean_derived_face_consistency_returns`
  is the landed regression test. The item's stated "expect 6 hits" anchor now
  yields **0**, which under H-8 is a stop condition — a packet generated from
  the current spec text would correctly halt with `ANCHOR_MISMATCH` on its first
  command. Amend §1 to closed before generating anything.
- **The 7th site is still open.** `unimplemented!()` survives at
  `impl ToSameGeometry<Surface> for ExtrudedCurve<Curve, Vector3>` on the
  `(IntersectionCurve, IntersectionCurve)` pair — the extrude case the spec said
  was "handled separately below" and then did not handle. Now written up as
  **BG-S0-003** in the build spec, anchor verified at 1 hit.
- **`BG-S0-002` is NOT done.** Six `unwrap()` remain in
  `truck-shapeops/src/fillet/mod.rs`. The spec's counts here still stand.
- **~~Nothing is committed~~ — closed 2026-08-16 in `da72cd5`.** `vendor/` was a single untracked entry: the P-1
  vendoring, the P-6 `truck-evidence` crate, and the landed BG-S0-001 work all
  exist only in the working tree. Two consequences, both blocking:
  `scripts/kernel-gates.sh` is diff-scoped and **silently no-ops** while the
  baseline lacks `vendor/truck/` (it says so itself), so V4 is currently
  vacuous; and a slot worktree checked out from any committed ref would contain
  **no kernel at all**. Verified after the commit: `kernel-gates.sh` passes
  live against a baseline containing `vendor/truck/`, and still reports the
  no-op against `origin/main` — so packet verification (V4, baseline = branch
  tip) is armed, while CI stays a no-op until this lands on `main`.

- **No kernel crate was a workspace member** — found by running the gate the
  loop depends on. `look/Cargo.toml` had no `[workspace]` section at all, so
  every truck crate was a bare path dependency, and
  `cargo test -p truck-evidence` fails outright: *"requires dev-dependencies
  and is not a member of the workspace"*. That is gate **V5** for every packet
  in every wave. Fixed by adding a `[workspace] members` list covering all 12
  vendored crates. This is the kind of blocker only found by executing the
  bootstrap rather than designing it.

- **Workspace membership exposed a second gap: `truck-polymesh`'s two
  integration test targets (`tests/obj-io.rs`, `tests/stl-io.rs`) do not
  compile.** Both `include_bytes!` fixtures from a sibling `resources/`
  directory that upstream truck keeps in a separate repo and this vendor
  snapshot never pulled in. Invisible while the crate was a non-member path
  dependency, since `cargo check --all-targets` only reaches workspace
  members. Fixed with `autotests = false` in `truck-polymesh/Cargo.toml`,
  since every test in both files needs the missing fixtures and none could
  be re-enabled individually.

**The general lesson for the loop, not just for these three items:** the spec's
own H-8 convention makes staleness detectable but not self-correcting. So the
packet generator must run every anchor's `rg` command *at generation time* and
refuse to emit a packet whose counts disagree with the spec — routing it to the
arbiter instead. Otherwise the loop's first act in each wave is to burn worker
sessions rediscovering that the document moved.

## 10. Bootstrap order

0. **Commit the kernel.** `vendor/truck/` (incl. `truck-evidence`), the
   `Cargo.toml`/`Cargo.lock` path deps, `.cargo/config.toml`, and
   `scripts/kernel-gates.sh` onto `integration/kernel-bg`, and make that the
   gates' baseline ref. Until this lands, V4 is vacuous and worktrees are empty.
1. **Reconcile the spec with §9 above** — close BG-S0-001, add
   `BG-S0-001-EXTRUDE`, confirm BG-S0-002's counts.
2. `loop/` skeleton + `verify.ps1` + `gen-packet.ps1` (with the anchor
   re-verification of §9) + the two hooks. These are the durable asset, exactly
   as the perf plan concluded about its evaluator.
3. **Dry-run on `BG-S0-002` with a Claude worker**, not deepseek — it is now the
   smallest genuinely-open mechanical item, its anchor counts (4 + 2 + 1) are
   checkable in one command, and it exercises V1 scope and V4 gates. Fix the
   harness here. (BG-S0-001's landed diff is the *reference answer* for what a
   good packet output looks like — read it when writing the packet template.)
4. Re-run the same packet with deepseek from a clean branch. Diff the two
   RESULT.json and the two diffs. That comparison — not a benchmark — tells you
   whether the packet format is unambiguous enough for the cheaper model. If
   deepseek needed information the packet did not contain, the template is
   wrong, not the model.
5. `BG-EVD-r3` (design, Claude) — everything types against it.
6. Open the throttle on W1/W2, then W4 at concurrency 3.

---

## 11. Where I am uncertain

- **~~The deepseek model id and endpoint shape~~ — resolved 2026-08-16.** The
  harness is `opencode run`, the id is `deepseek/deepseek-v4-flash`, the key is
  already in opencode's auth store, and the window is 1M. What remains open is
  whether `opencode/deepseek-v4-flash-free` is usable for the wide waves, and
  what opencode's JSON event stream actually reports for token usage — the
  supervisor in §3 depends on that field existing.

- **Whether a flash-class model can hold the H-rules while editing.** H-2
  (`Outcome` everywhere), H-3 (no literals in predicates) and H-6 (never record
  float as `Exact`) are the kind of standing constraint small models drop around
  turn 20. Mitigation is that they are all machine-checkable — H-1/H-3/H-4 by
  `kernel-gates.sh` today, H-2 and H-6 are **not** yet gated. Adding a grep gate
  for `-> Option<` in new kernel signatures and for `Method::Exact` in a function
  that mentions `f64` arithmetic is cheap and probably necessary before W4.
- **V7's cost.** Mutation-testing every gated item is unaffordable; the
  enumerated shortlist is a judgement call and may miss the item that actually
  ships vacuous.
- **BG-TOL-001 sharding.** 184 sites split across ~7 packets assumes the
  model/param judgement is genuinely per-site local. If one module's sites are
  correlated (e.g. a shared helper that is called from both a model-space and a
  parameter-space context), the shard boundary is wrong and the fix is one
  bigger packet, not three retries.
- **Turn caps and aborts.** opencode exposes no per-run turn cap, so the
  orchestrator-side supervisor of §3 is the only limit. Untested.
- **Whether W6 belongs in the loop at all.** BG-FID-001/003/005 are the spec's
  root and its subtlest items (the `lfs_lower` naming discipline exists because
  a call site reading it as equality is a silent unsoundness). The honest read
  is the same as the perf plan's: run the loop hard through W5, where the work
  is genuinely mechanical and the parallelism is real, and treat W6 as
  human-plus-Claude work that the loop only writes *tests* for.
