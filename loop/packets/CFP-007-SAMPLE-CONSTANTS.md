# WORK PACKET CFP-007-SAMPLE-CONSTANTS — float-preconditioner reuse + ε-inflation

You are removing the steady-state per-sample certification overhead from the
branch-continuation loops, using the two landed-theorems-sanctioned
techniques. Everything you need is in this document and
`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3 (Krawczyk Y-independence) and §4
(packet row). Do not read other spec files. SPEC_GAP discipline applies.

```yaml
id:          CFP-007-SAMPLE-CONSTANTS
contract:    [CFP-007-SAMPLE-CONSTANTS]
class:       mechanical
crates:      [truck-certified]
depends_on:  [CFP-000-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
read_allow:
  - docs/CONTACT_FAST_PATH_BUILD_SPEC.md
  - vendor/truck/truck-certified/src/construct/bie/ssi4.rs
  - vendor/truck/truck-certified/src/ssi_trace.rs
  - vendor/truck/truck-certified/src/ssi.rs
  - vendor/truck/truck-certified/src/ssi_types.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
tests_required:
  - preconditioner_reused_across_consecutive_samples
  - preconditioner_invalidated_on_inclusion_failure
  - eps_inflation_recovers_marginal_sample
  - eps_inflation_sequence_is_fixed_and_bounded
  - verdicts_bit_identical_with_reuse_enabled
  - fc5_branch_fixture_prune_count_unchanged
budget:      {turns: 70, ctx_tokens: 160000}
```

## Problem

Every certified continuation sample currently pays the full Krawczyk input
cost, including the system's preconditioner computation. Two landed facts make
most of that work redundant:

1. **Y-independence** (`num/krawczyk.rs` header, Moore/Krawczyk): the strict
   inclusion test `K(X) ⊂ int(X)` proves a unique root for ANY invertible Y —
   Y's quality affects tightness only, never soundness. The landed operator
   already takes a float preconditioner from the system; nothing stops the
   CALLER from supplying the SAME float Y across consecutive samples of one
   branch, where conditioning varies slowly.
2. **ε-inflation (Rump)**: a sample that fails inclusion at radius `r` is
   often certifiable at `2r`; the inflation sequence is deterministic and the
   certificate remains the box.

Both are SFC-clean: floats shape the search; every emitted certificate is the
landed Krawczyk operator's own interval proof. `num/krawczyk.rs` is FROZEN —
all changes are caller-side, in the two continuation loops this packet owns.

## Scope decisions — pre-made, do not relitigate

1. **Preconditioner reuse, caller-side.** In `ssi4.rs`'s parallelotope
   continuation (and the corresponding per-box certifier in `ssi_trace.rs`),
   cache the last accepted float preconditioner `Y` and the sample index it
   certified. A new sample first attempts Krawczyk with the CACHED `Y`;
   on inclusion failure, recompute `Y` fresh at the new point, cache it, and
   retry once. Never reuse a `Y` after an inclusion failure — that is the
   invalidation rule, and it is what keeps tightness from silently
   degrading.
2. **ε-inflation with a FIXED, BOUNDED sequence.** On inclusion failure at
   radius `r`, retry with `r·2^k` for `k = 1..=3` (four attempts total
   including `k=0`), low-first, before declaring the sample uncertified and
   falling back to the landed bisection discipline. The sequence is a named
   constant; no adaptive or geometry-dependent schedule.
3. **The certified statement is unchanged**: every accepted sample carries
   the landed operator's own `KrawczykProof`; inflation widens the REQUESTED
   box, never the certificate's meaning. H-6 holds — `Y` and the inflation
   radii never enter evidence.
4. **`num/krawczyk.rs` is not edited.** The operator's contract is frozen
   (instantiated, never extended). If you find yourself wanting to change it,
   that is a SPEC_GAP.
5. **V5-pair scope**: canonical×canonical pair answers never traverse these
   loops; no V5 exposure. Determinism gate still applies: with reuse and
   inflation enabled, identical ordered input → identical verdicts AND
   identical certificate boxes (the reuse path is deterministic because the
   invalidation rule is).
6. **Kantorovich step length is NOT in this packet.** It needs the per-side
   second-difference machinery CFP-003 lands; booking it here would couple
   the packets. Deferred to the CFP-003 follow-up.

## Anchors — base-relative, drift-tolerant

Re-check on your branch:

| id | file | pattern | expect |
|---|---|---|---|
| A1 | `truck-certified/src/construct/bie/ssi4.rs` | `struct Ssi4Parameters` | 1 |
| A2 | `truck-certified/src/construct/bie/ssi4.rs` | `fn preconditioner` | ≥1 (the landed system impl you will thread the cache through) |
| A3 | `truck-certified/src/ssi_trace.rs` | `trait BranchCertifier` | 1 |
| A4 | `truck-evidence/src/num/krawczyk.rs` | `pub fn krawczyk` | 1 (read-only frozen anchor — any diff here is a violation) |

A4 must show NO diff in your branch's `--base` comparison. If your patch
touches `krawczyk.rs`: STOP, that is a spec violation, report as SPEC_GAP.

## House rules

- **H-1** No `unwrap`, `expect`, `panic!`, `todo!` reachable from geometry.
- **H-3** No absolute constants in predicates; the inflation sequence and
  retry counts carry `// H-3` on the SAME line as the literal.
- **Determinism**: the cache and inflation sequence are deterministic state;
  no hash iteration; identical ordered input → identical verdicts and
  identical boxes.
- **All cargo invocations go through the queue (the `cargo` on PATH IS the
  queue shim).**

## Tests required

Named `#[test]` fns; the verifier checks the names appear in your diff:

1. `preconditioner_reused_across_consecutive_samples` — on a straight-branch
   fixture, the second sample's Krawczyk call receives the first sample's
   cached `Y` (observable via a counting wrapper in the test module).
2. `preconditioner_invalidated_on_inclusion_failure` — after a forced
   inclusion failure, the next attempt uses a freshly computed `Y`.
3. `eps_inflation_recovers_marginal_sample` — a constructed marginal sample
   that fails at `r` certifies at `2r`.
4. `eps_inflation_sequence_is_fixed_and_bounded` — the attempt count is
   exactly the named constant; no geometry-dependent schedule.
5. `verdicts_bit_identical_with_reuse_enabled` — on the landed ssi4 fixture
   battery, the full verdict + certificate box set is bit-identical with the
   cache on vs off (reuse changes cost, never results).
6. `fc5_branch_fixture_prune_count_unchanged` — F-C5 data: the branch sample
   count and box set on the fixture are unchanged by the optimization.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-certified
cargo clippy -p truck-certified --all-targets -- -D warnings
cargo test -p truck-certified --lib construct::bie::ssi4
cargo test -p truck-certified --lib ssi_trace
```

Send cargo output to a file and read the tail.

## Forbidden

Editing any file outside `write_allow` — especially `num/krawczyk.rs` (FROZEN
— an edit there is a spec violation, report as SPEC_GAP), `ssi.rs`,
`ssi_types.rs`, `tangency/**`, `contact/**`, anything under `truck-evidence/`
or `truck-shapeops/`, `Cargo.lock`. Changing any verdict or certificate box.
Adaptive or geometry-dependent inflation schedules. Adding `#[ignore]`.
Adding `#[allow]` without a justification comment on the same line.
Committing to `main`.

## Stop conditions

- any anchor count differs → `ANCHOR_MISMATCH`
- any diff touching `num/krawczyk.rs` → SPEC_GAP (frozen file)
- reuse or inflation changes a single certificate box on the fixture battery
  → STOP and report — that is a soundness-relevant finding, not something to
  tune away
- three consecutive failed `cargo test` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` at the WORKTREE ROOT (then COMMIT first)

**COMMIT BEFORE writing `RESULT.json`.** Then write `RESULT.json` at the root
of your worktree.

```json
{"id":"CFP-007-SAMPLE-CONSTANTS","status":"DONE","contracts":["CFP-007-SAMPLE-CONSTANTS"],
 "tests_added":6,"anchors_verified":{"A1":1,"A2":1,"A3":1,"A4":1},
 "notes":"caller-side reuse + bounded inflation; krawczyk.rs untouched; any deviation stated here"}
```
