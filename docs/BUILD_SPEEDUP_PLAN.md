# Build speedup work plan

Audit date: 2026-08-14. Static audit only — no builds were run to produce this,
so every timing claim below is a **hypothesis with a measurement attached**, not
a result.

## Ground truth established (verified, no build required)

- `rustc -vV` reports `host: x86_64-pc-windows-gnullvm`; gnullvm is the active
  default toolchain. `rustc 1.97.1`, `cargo 1.97.1`.
- `target/` contains only `debug/` and `release/` — there is **no**
  `x86_64-pc-windows-gnullvm/` subdir, so recent builds already ran without
  `--target`.
- `.cargo/config.toml` contains **zero** active settings: 93 lines of provenance
  commentary around a commented-out `paths` block. No linker, no incremental
  setting, no rustflags.
- `Cargo.toml` has no `[profile.dev]`. `[profile.release]` is
  `lto="thin"`, `codegen-units=1`, `strip="symbols"`, `panic="unwind"`.
- `rust-lld.exe` is present in the gnullvm sysroot bin dir, unused.
- Dependency graph: 240 unique crates. `look` has no `[[bin]]`/`[lib]` sections
  (auto-discovered `src/lib.rs` + `src/main.rs`), 56 auto-discovered examples,
  5 integration test files.
- Disk: `target/debug` 1.1 GB, `target/release` 1.2 GB,
  `target/debug/incremental` 389 MB. `truck-fork/` has no `target/` at all.
- No-op `cargo build --release` is 0.36s warm. Not a problem area.

## Baseline that must be captured before any change

None of the items below can be judged without this. Run it once, in a quiet
window, and record it in this file.

Per the standing benchmarking rule: **alternate configs A/B/A/B, take the
minimum of >=5 reps**, and check free disk space first. Batching all reps of A
then all of B has already produced a fabricated 2.6x on this machine.

Measure three separate loops — they have different bottlenecks and different
fixes:

| Loop | Command | Why it matters |
|---|---|---|
| L1 leaf edit | touch `src/step.rs`, `cargo build --release` | the STEP dev loop |
| L2 truck edit | touch a `truck-fork/truck-meshalgo` source, `cargo build --release` | rebuilds truck + all downstream + look |
| L3 test run | `cargo test --release` | currently also builds 56 examples |

Record wall-clock minimum-of-5 for each. **L2 requires the `paths` override in
`.cargo/config.toml` to be live** — re-comment it afterwards, per the standing
rule in that file.

---

## Item 1 — Correct the stale build command (no risk, do first)

**Change:** none to the repo. Update the `look-build-quirks` memory.

The recorded command is:

```
cargo +stable-x86_64-pc-windows-gnullvm build --release --target x86_64-pc-windows-gnullvm
```

Both the toolchain prefix and `--target` were needed when the host was msvc.
The host is now gnullvm, so both are redundant, and `--target` is actively
harmful: when it equals the host, cargo stops sharing host artifacts and
compiles build scripts and proc macros twice, into a separate
`target/x86_64-pc-windows-gnullvm/` tree. Anyone following the note today eats a
full cold build (~4-5 min) and then keeps paying the proc-macro split forever.

**Correct command:** `cargo build --release`

**Verify:** `cargo build --release` succeeds and links; no new
`target/x86_64-pc-windows-gnullvm/` directory appears.

**Risk: none.** This changes what you type, not what is built. If the host ever
moves back to msvc, the old command is still correct and the failure is a loud
link error, not a silent wrong result.

---

## Item 2 — Stop building 56 examples during tests (likely largest win)

**Hypothesis:** L3 is dominated by compiling `examples/*.rs`, not by the tests.
56 example binaries, each linking the full 240-crate graph under
`codegen-units = 1` + thin LTO. The examples are probes — `face_census`,
`nist1167_*`, `spline_edge_00007667_*` — that are run deliberately, not part of
the test suite.

**Change A (zero risk, try first):** just use `cargo test --tests` for routine
runs. No repo change at all. Measure against L3.

**Change B (if A confirms the hypothesis):** in `Cargo.toml`, set
`autoexamples = false` under `[package]` and declare only the probes you
actually run as explicit `[[example]]` entries.

Change B has a real cost: an undeclared example silently stops building, so
bitrot in the other probes goes undetected until the day you need one. Given
these probes encode hard-won diagnostic work, prefer keeping A as a habit over
adopting B, unless the measured win is large.

**Verify:** `cargo test --tests` runs the same 5 integration test files
(`assembly`, `gpu_smoke`, `spline_carrier`, `step`, `torus_deck`) plus unit
tests, and reports the same pass count as `cargo test`.

**Risk of A: none** — it is a narrower invocation, not a config change.
**Risk of B: low but real**, described above.

---

## Item 3 — A separate iteration profile (do not touch `[profile.release]`)

**Change:** add to `Cargo.toml`:

```toml
[profile.quick]
inherits = "release"
lto = "off"
codegen-units = 16
```

Iterate with `cargo build --profile quick`; keep `--release` for anything whose
number gets recorded.

**Hypothesis:** `lto="thin"` + `codegen-units=1` is the bulk of L1 and L2. Both
exist to make the shipped binary fast; neither helps an edit-compile-look loop.

**Verify:** binary appears at `target/quick/look.exe`; a face census run under
it produces the **same face counts** as the release binary on one known model
(e.g. `00009190`).

**Risk — read this before adopting:**

- **This repo is a measurement instrument, and that is the real hazard.** A
  `quick` binary is slower at runtime than a `release` one. Any *timing*
  benchmark accidentally taken under it is not comparable to any recorded
  baseline. This is the same class of error as the `paths` override recording a
  rev the build never had. Mitigation: `quick` writes to `target/quick/`, a
  different path from `target/release/`, so a script pointing at
  `target/release/look.exe` cannot silently pick it up — unlike the override,
  this failure mode is visible in the path.
- **Face counts should be unaffected**, because Rust does not enable fast-math
  or FMA contraction at any opt level, so IEEE float results are
  optimization-independent. I am confident in that as a general rule but have
  **not** verified it against this geometry code, which is exactly why the
  verify step above compares census counts rather than assuming.
- **`panic = "unwind"` is inherited** from `release` — required, since PLANAR-C
  relies on `catch_unwind` to contain Spade panics as per-face failures. Do not
  override it in the new profile.
- **Disk: +~1.2 GB** for a third target tree. Free space has already produced
  garbage benchmark numbers here (one 136s sample); check before adopting.

---

## Item 4 — Faster debug builds

**Change:** add to `Cargo.toml`:

```toml
[profile.dev.package."*"]
opt-level = 2
```

This optimizes dependencies while leaving `look` itself unoptimized and fully
debuggable. Dependencies are compiled once and then cached, so the cost is paid
on the first build only.

**Hypothesis:** debug builds are currently unusable for STEP work because
unoptimized truck/geometry code is too slow at runtime, which is why everything
happens in `--release`. If so, this makes `cargo test` (debug) viable and moves
the whole loop off the expensive profile.

**Verify:** `cargo test --tests` in debug completes in reasonable wall-clock and
passes.

**Risk: low.** Does not affect the release binary or any recorded number.
One-time rebuild of all 240 deps in debug. Costs disk in `target/debug`, which
is already 1.1 GB.

---

## Item 5 — Switch the linker to `rust-lld` (most uncertain, do last)

**Change:** in `.cargo/config.toml`, **above** the commented `paths` block:

```toml
[target.x86_64-pc-windows-gnullvm]
rustflags = ["-Clink-self-contained=+linker", "-Zlinker-features=+lld"]
```

**Hypothesis:** linking an 11 MB binary against 240 crates with MinGW `ld` is a
meaningful slice of every rebuild, and `lld` is typically several times faster.

**Risk: this is the one I am least confident about.**

- `-Zlinker-features` is a **nightly-only** flag. On stable 1.97.1 this will be
  rejected. The stable path on gnullvm may require setting `linker = "rust-lld"`
  (or a `-fuse-ld=lld` wrapper) directly instead, and I have **not** confirmed
  which form works on this target. Treat Item 5 as an experiment with an unknown
  answer, not a prescription.
- gnullvm + lld is a less-travelled combination than msvc + lld. Failure could
  be a loud link error (fine) or a subtly miscompiled binary (not fine).
- **Because it lives in `.cargo/config.toml`, it applies to every build in this
  directory**, including release builds whose numbers get recorded. That is the
  one file where a mistake has historically gone unnoticed for 37 commits.

**Therefore:** if adopted, verify by running a face census and confirming counts
match the previous release binary before trusting any subsequent number. If the
win is small, do not take this one — the blast radius is worse than the others.

---

## Recommended order

1. **Item 1** — free, do now.
2. **Item 2 change A** — free, do now (habit change only).
3. Capture the L1/L2/L3 baseline.
4. **Item 4**, then **Item 3** — measure each against baseline.
5. **Item 5** only if L1/L2 still look link-bound, and only with the census check.

Items 1, 2A, and 4 cannot affect any recorded measurement. Item 3 can only do so
through a path confusion that is visible in the path. Item 5 is the only one
that changes the release binary itself.
