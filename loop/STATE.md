# Autobuild loop — STATE

Rewritten at the end of every session. Capped at ~100 lines. If you are picking
this up cold, read **this file and the last 5 rows of `LEDGER.jsonl`** — nothing
else. Do not read `LEDGER.jsonl` whole.

Design: [`docs/KERNEL_AUTOBUILD_LOOP.md`](../docs/KERNEL_AUTOBUILD_LOOP.md).
Spec: [`docs/GENERATION_KERNEL_BUILD_SPEC.md`](../docs/GENERATION_KERNEL_BUILD_SPEC.md).

Updated 2026-08-16.

## Where we are

Bootstrap. **No packet has been dispatched yet.** W0 (infrastructure) is in
progress; W1 is ready to dispatch as soon as slot 0 is warm.

Branch: `integration/kernel-bg`. Nothing from the loop reaches `main`.

## Landed

| commit | what |
|---|---|
| `da72cd5` | vendored truck at `vendor/truck/` (12 crates) + kernel gates + `truck-evidence` (P-1, P-3, P-5, P-6) |
| `fddc62a` | vendored crates are workspace members — without this `cargo test -p <crate>` (gate V5) cannot run at all |
| `b1e9476` `ce0d037` `e19d8b1` | the spec, the loop design, BG-S0-001 closed / BG-S0-003 split out, opencode confirmed as the harness |

Contracts discharged so far: **BG-S0-001** (landed before the loop existed; it
is the reference answer every mechanical packet copies).

## Ready to dispatch

- **BG-S0-002** — packet written at `loop/packets/BG-S0-002.md`, anchors
  verified 4/2/1 on 2026-08-16. Waiting on slot 0.
- **BG-S0-003** — spec'd, no packet yet. Independent of BG-S0-002 (different
  crate), so it is the second packet and can run concurrently.

## Next after W1

W0b `BG-EVD-r3` (design class, Claude not deepseek): `Modulus` becomes a struct
with `domain` + shape-derived `is_subadditive` and a `propagate` recurrence,
`Refusal` gains `ForwardToleranceExceeded`, `ModulusShape` gains `Pole`.
**Everything below it types against this**, so it blocks W2 onward.

## Invariants a worker must never violate

Restated in every packet; repeated here because a session that edits the packet
template can silently drop them.

- Locate by `rg` pattern, never by line number. A count mismatch is a **stop
  condition**, not a nuisance.
- Never edit `scripts/kernel-gates.sh`, `.cargo/config.toml`, `Cargo.lock`, or
  any file outside the packet's `write_allow`.
- Never `#[ignore]` a test, delete a test, or weaken an assertion to get green.
- Never run a bare `cargo test` — it builds 56 examples. Always `-p <crate>
  --lib --tests`.
- Never commit to `main`.

## Dead ends and traps

- **The spec goes stale invisibly.** BG-S0-001 was already landed while the spec
  still listed it open, with an anchor count of 6 that is now 0. The packet
  generator must re-run every anchor's `rg` at generation time and refuse to
  emit on a mismatch.
- **`autotests = false` in `truck-polymesh`.** Its two integration test targets
  `include_bytes!` an upstream `resources/` directory that vendoring omitted, so
  they cannot compile. Any *new* test file in that crate needs an explicit
  `[[test]]` entry or it silently will not run — which is exactly the vacuous
  test gate V6 exists to catch.
- **`truck_base::evidence`, not `truck_evidence`.** The evidence module lives in
  `truck-base` to avoid a `truck-geotrait` → evidence dependency cycle;
  `truck-evidence` re-exports it. A worker importing the wrong path will fight
  the compiler for turns.
- **CI gates are still a no-op.** `kernel-gates.sh` is diff-scoped and
  `origin/main` does not yet contain `vendor/truck/`, so CI passes vacuously.
  Packet verification is unaffected (its baseline is the branch tip).

## Open questions

- `opencode/deepseek-v4-flash-free` — would run W4's 23 packets at no API cost.
  Untested against three concurrent workers.
- What opencode's `--format json` event stream reports for token usage. The
  orchestrator's runaway detector depends on that field existing.
