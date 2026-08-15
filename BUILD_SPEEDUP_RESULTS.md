# BUILD_SPEEDUP_RESULTS

Implementation + measurement report for the Look/Truck edit-rebuild loop.
Session date: 2026-08-14.

## Provenance

| Item | Value |
|---|---|
| Look SHA before | `f1869aa741d94e57ab9f46274f4a964a3de167c2` |
| Look SHA after (working tree) | `f1869aa` + uncommitted: `Cargo.toml`, `.cargo/config.toml`, `README.md`, `AGENTS.md` |
| Truck SHA (resolved by Look) | `09726a9e20c3ddb6cb09ec82bd2fbd24d3ab7cfc` (matches Cargo.toml pin) |
| rustc | `1.97.1 (8bab26f4f 2026-07-14)`, LLVM 22.1.6 |
| cargo | `1.97.1 (c980f4866 2026-06-30)` |
| Host target | `x86_64-pc-windows-gnullvm` (active host; no `--target` used anywhere) |
| Adapter | NVIDIA GeForce RTX 5050 Laptop GPU, DX12 |

Control models: `look-corpus/core_xy/core_xy.step` (9.1 MB, 5,670 faces),
`look-corpus/formula1/formula1.step` (44 MB), `look-corpus/ur10/ur10.step`,
33-file NIST corpus under `look-corpus/nist`.

The Truck path override in `.cargo/config.toml` was temporarily enabled for the
L2 measurements (per its standing rule) and re-commented afterwards. All L2
numbers below were measured with the override live against the working
`truck-fork` tree at the pinned revision.

## profile.quick settings (Cargo.toml)

```toml
[profile.quick]
inherits = "dev"
opt-level = 2
debug = false
lto = "off"
incremental = true
codegen-units = 256
```

`[profile.release]` is unchanged (`lto="thin"`, `codegen-units=1`,
`strip="symbols"`, `panic="unwind"`). `panic="unwind"` is inherited from `dev`
so PLANAR-C's `catch_unwind` containment still works under Quick.

## MEASURED — Release baselines (real edit → rebuild)

| Workload | median | min | max |
|---|---|---|---|
| L1 Look-edit rebuild (`cargo build --release --locked` after comment-only edit in `src/step.rs`) | 49.52 s | 48.81 s | 84.41 s |
| L2 Truck-edit rebuild (comment-only edit in `truck-meshalgo/src/tessellation/triangulation.rs`, then Look release build) | 77.52 s | 56.21 s | 93.78 s |

## MEASURED — Quick rebuilds

| Workload | median | min | max | speedup vs Release |
|---|---|---|---|---|
| L1 Look-edit rebuild (`cargo build --profile quick --locked`) | 6.98 s | 6.88 s | 9.14 s | 7.1× |
| L2 Truck-edit rebuild (same truck edit, `--profile quick`) | 7.65 s | 7.64 s | 24.00 s | 10.1× |

Rep 1 of each Quick series is slightly elevated (incremental cache warm-up after
the edit path; 9.14 s / 24.00 s). Steady-state L2 Quick is ~7.6 s.

Headline:

```
Truck edit → usable Look binary:
before = 77.5 s   (release)
after  =  7.6 s   (quick)
speedup = ~10.2×
```

## MEASURED — Cold Quick build (one-time)

`cargo build --profile quick --locked --timings`: **261.4 s**, 289 units, max
concurrency 17 (jobs=16, ncpu=16).

Largest compilation units (one-time cost, cached afterwards):

| crate | total | frontend | codegen |
|---|---|---|---|
| naga v30.0.0 | 118.5 s | 100.8 s | 17.7 s |
| truck-stepio v0.3.0 | 104.7 s | 88.1 s | 16.6 s |
| pxfm v0.1.30 | 98.4 s | 68.2 s | 30.2 s |
| gltf-json v1.4.1 | 98.2 s | 6.9 s | 91.3 s |
| glam v0.30.10 | 94.1 s | 91.9 s | 2.2 s |
| look v0.1.0 | 52.4 s | — | — |
| truck-meshalgo v0.4.0 | 37.1 s | — | — |

Frontend time dominates naga/truck-stepio/pxfm/glam (huge mono crates with many
generic instantiations); codegen dominates gltf-json. The 261 s cold build is a
one-time tax; every warm L1/L2 Quick cycle is 7–9 s.

## MEASURED — test-target narrowing (Quick, warm)

| Command | wall | notes |
|---|---|---|
| `cargo test --profile quick --locked --no-run` (full, incl. 56 examples) | 5.97 s | warm; cold was 302 s |
| `cargo test --profile quick --tests --locked --no-run` | 3.19 s | skips the probe/example collection |
| `cargo test --profile quick --lib --locked --no-run` | 2.61 s | |
| `cargo test --profile quick --lib part21 --locked` (focused, run) | 3.48 s | 22 passed, 0 failed |

`qtest` (`--tests`) avoids compiling the 56 `examples/` probe binaries on every
dev-test run. The examples remain in the tree and build under the full gate
(`cargo test --locked --all-targets`); nothing was deleted or relocated.

## MEASURED — Quick runtime validation

| model | Release wall | Quick wall | triangles Release | triangles Quick | ratio |
|---|---|---|---|---|---|
| core_xy | 2.12 s | 3.37 s | 406,096 | 406,096 | 1.6× |
| formula1 | 3.05 s | 4.76 s | 365,809 | 365,809 | 1.6× |
| ur10 | 4.21 s | 5.36 s | 502,130 | 502,130 | 1.3× |

Identical scene statistics on all three (nodes, mesh primitives, vertices,
triangles, bounds, materials). Quick runtime is ~1.3–1.6× Release, well inside
the <=2.5× target, so `codegen-units = 256` was kept (no 64-CGU alternative
tested because 256 is comfortably within budget).

## MEASURED — NIST regression (Release vs Quick)

| | files | ok | warnings | triangle mismatches |
|---|---|---|---|---|
| Release | 33 | 33 | 2 (geometry may be incomplete) | — |
| Quick | 33 | 33 | 2 (geometry may be incomplete) | 0 |

All 33 files: identical outcome and triangle count between Release and Quick.

## rust-lld experiment — REJECTED

Tested via `scratch/build_speed/lld.toml` (`[target.x86_64-pc-windows-gnullvm]`
`linker = "rust-lld"`) with `cargo --config scratch\build_speed\lld.toml build
--profile quick --locked`.

Result: **failed**. `rust-lld` on the gnullvm host cannot find the MinGW import
libraries:

```
lld: error: unable to find library -ladvapi32
lld: error: unable to find library -lole32
lld: error: unable to find library -loleaut32
error: could not compile `lzma-sys` (build script)
```

This is the native-linker/library incompatibility the protocol anticipated. The
experiment was discarded as instructed (no A/B benchmark possible, no permanent
linker change, no `.cargo/config.toml` target-linking entry). The default MinGW
linker remains in use for both Quick and Release.

## Final aliases (.cargo/config.toml)

```toml
[alias]
qcheck = "check --profile quick --locked"
qbuild = "build --profile quick --locked"
qtest = "test --profile quick --tests --locked"
```

Development workflow: `cargo qcheck` / `cargo qbuild` / `cargo qtest`; narrow
kernel test `cargo test --profile quick --lib <filter> --locked`. Authoritative
build: `cargo build --release --locked` (unchanged). Documented in README
(Development) and AGENTS.md (Build modes). AGENTS.md now also states the active
host is gnullvm and ordinary native builds must not pass
`--target x86_64-pc-windows-gnullvm`.

## MEASURED — Release regression gate

Only the Quick profile and aliases changed (linker not adopted), so Release
semantics are untouched. Verified:

- `cargo build --release --locked` completes and produces `target/release/look.exe`.
- Release binary renders core_xy / formula1 / ur10 with the same triangle counts
  as before (406,096 / 365,809 / 502,130).
- NIST corpus under Release: 33/33 ok, 2 warnings, same as the pre-session
  baseline. Triangle counts per file match the recorded corpus baseline
  (`benchmarks/corpus_regression_baseline.json`).

## MEASURED — disk usage

| tree | size |
|---|---|
| target/debug | 1.73 GB |
| target/quick | 3.56 GB |
| target/release | 1.51 GB |

`target/quick` grew during the session (cold build + all-targets test warm-up).
No `cargo clean` was run during measurement. Free disk at session end: ~12.9 GB.

## HYPOTHESIZED / REJECTED

- **Dependency optimization override** (`[profile.dev.package."*"] opt-level=2`
  or a Quick equivalent): explicitly deferred per plan. Quick runtime is already
  <=1.6× Release, so it is not needed. REJECTED for this session.
- **rust-lld linker**: measured-in-failure, REJECTED (cannot link MinGW import
  libs on this host).
- **codegen-units = 64**: not needed; 256 keeps Quick runtime within budget. NOT
  TESTED, by design (no broad CGU sweep).
- **Removing `--target x86_64-pc-windows-gnullvm`**: not counted as a speedup —
  the current workflow already builds without it; only documented.

## Success criteria check

1. `cargo build --release --locked` semantically unchanged: PASS
2. Separate `target/quick/` development build exists: PASS
3. Real Look-edit and Truck-edit rebuild times measured: PASS (5 Release + 3 Quick reps each)
4. Quick materially improves a rebuild loop: PASS (L1 7.1×, L2 10.2×)
5. Focused tests no longer compile the probe/example collection: PASS (`qtest` uses `--tests`; examples intact)
6. rust-lld measured/rejected: PASS (rejected on native-lib incompatibility)
7. Quick matches Release outcome/triangle accounting on controls: PASS (core_xy, formula1, ur10, 33 NIST files)
8. Linker did not change globally, so full NIST/corpus gate not re-run as a condition: PASS (Release spot-check clean)
9. Docs tell agents which build mode to use: PASS (README + AGENTS.md)
10. Measured development-loop speedup reported: PASS

## Summary

Routine Truck/Look edit-build cycles go from **~77.5 s (median) to ~7.6 s
(median)** — a **~10×** reduction — while `target/release/look.exe` remains the
authoritative production artifact with unchanged semantics and performance.
