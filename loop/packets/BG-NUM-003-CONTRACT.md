# WORK PACKET BG-NUM-003-CONTRACT — correct the Krawczyk contraction to the true matrix product

You are fixing a documented latent defect in the vendored kernel's foundational
numeric operator. Everything you need is in this document. **Do not read
`docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other spec file** — they are not
on your allowlist and this packet is self-contained. If something you need is
genuinely missing, that is a SPEC_GAP (see "Stop conditions"): you stop and
report, you do not research it.

```json
{"id":"BG-NUM-003-CONTRACT","status":"DONE","contracts":["BG-NUM-003-CONTRACT"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. **If anything below contradicts what you find in the code, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-NUM-003-CONTRACT
contract:    [BG-NUM-003]
class:       mechanical
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
read_allow:
  - vendor/truck/truck-evidence/src/num/mod.rs
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/fid/rep.rs
tests_required:
  - coupled_system_certifies_after_matrix_contraction
  - entrywise_form_would_not_have_certified
  - diagonal_system_still_certifies
budget:      {turns: 25, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn k_image' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'd\\[r\\]\\[c\\]' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn krawczyk' vendor/truck/truck-evidence/src/num/krawczyk.rs"}
```

## Problem

`num/krawczyk.rs::k_image` (BG-NUM-003, spec line 148) computes the
contraction row as

```text
d[r][c] = δ(r,c) − y[r][c]·j[r][c]
```

pairing ONLY the same column index on `Y` and `J`. The standard interval
Krawczyk operator requires the MATRIX PRODUCT:

```text
K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)
(I − Y·J)[r][c] = δ(r,c) − Σ_k y[r][k]·j[k][c]     # sum over the inner index
```

The spec's formula omits the inner sum over `k`, so `k_image` computes
`I − Y∘J` (Hadamard/elementwise), not `I − YJ`. For a system whose interval
Jacobian is effectively diagonal the two agree, which is why every user so far
(canonic parametric projections) certified fine. The defect surfaced only when
the solver family's general validated FF stage (BG-SOL-S7-GFF-COVER, two
SPEC_GAP attempts: 836b704 3×3 augmented, 05fe95d 2×2 z-slab) brought a
GENUINELY COUPLED 2×2 system — the z-aligned cylinder×sphere slab Jacobian
`[[2x, 2y],[2(x−cx), 2(y−cy)]]` — whose entrywise contraction can never be
interior (`I − Y∘J` row sums ~1.1–3.9 > 1 at every box scale, measured by the
worker), so `KrawczykProof::Unique` is unreachable regardless of budget.

The fix is the one-line-formula correction inside `k_image`. The operator's
public contract (`KrawczykSystem`, `krawczyk`, budget/spent semantics) is
untouched. This change can only STRENGTHEN certification: where the matrix
product equals the entrywise form (diagonal Jacobians) behavior is identical;
where they differ, systems that previously refused `NumericallyUnresolved`
can now prove `Unique`. No existing `Unique`/`NoRoot` can flip the other way.

## Decisions already made for you

**Only `vendor/truck/truck-evidence/src/num/krawczyk.rs` changes.** (The
module doc comment repeats the wrong formula; fix it in the same commit.)

### 1. The corrected contraction, verbatim replacement inside `k_image`:

Replace the `dq` fold so row `r` contracts against the true matrix-product
column:

```rust
let dq = (0..N)
    .map(|c| {
        // (I - Y*J)[r][c] = delta(r,c) - sum_k y[r][k] * j[k][c]
        let delta = interval_at(if c == r { 1.0 } else { 0.0 });
        let inner = (0..N).fold(delta, |acc, k| {
            acc - interval_at(y.get(r).copied().unwrap_or([0.0; N])[k])
                * j.get(k).copied().unwrap_or([Interval::EMPTY; N])[c]
        });
        inner * (*qc - interval_at(m.get(c).copied().unwrap_or(0.0)))
    })
    .fold(interval_at(0.0), |acc, term| acc + term);
```

In words: for row `r`, column `c`, the coefficient is
`δ(r,c) − Σ_k y[r][k]·j[k][c]` — `y`'s row `r` dotted with `j`'s column `c`.
Read the existing `k_image` first; preserve its structure (the `center` term
and the `yf = Σ_c y[r][c]·f[c]` matrix-vector product are already correct —
only the `dq` coefficient changes). `y` is the float preconditioner supplied
by the system, `j` the interval Jacobian over the box, both row-major as
documented.

### 2. The module doc comment (lines ~158-161), verbatim replacement:

```rust
/// The Krawczyk image `K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)`. Row `r`:
/// `iv(m[r]) − Σ_c y[r][c]·f[c] + Σ_c d[r][c]·(q[c] − iv(m[c]))` with
/// `d[r][c] = δ(r,c) − Σ_k y[r][k]·j[k][c]`, row-major throughout — the
/// system's row-major Jacobian convention is relied on, never transposed.
/// (BG-NUM-003-CONTRACT: the original spec wrote `d[r][c] = δ(r,c) −
/// y[r][c]·j[r][c]`, the Hadamard form; it agrees with the matrix product
/// only for diagonal Jacobians and could not certify the coupled slab
/// systems the general FF stage needs.)
```

### 3. Tests (witnesses machine-checked at packet-writing time)

Add to the module's existing `#[cfg(test)]` (keep the existing tests; do not
weaken any assertion):

- `coupled_system_certifies_after_matrix_contraction` — the two-equation
  system in `(x, y)`:
  `f1 = x² + y² − 1`, `f2 = (x−3)² + y² + z0² − 9` at fixed `z0 = √2` (the
  transversal sphere/cylinder slab witness; the crossing `(1/2, −√3/2)`
  satisfies both to f64 rounding — machine-checked: f1 = 0.25+0.75−1 = 0,
  f2 = 6.25+0.75+2−9 = 0). Jacobian `[[2x, 2y],[2(x−3), 2y]]`, determinant
  `12y` — genuinely coupled. Assert `krawczyk(...)` returns
  `KrawczykProof::Unique` over a box of width 1e-2 centered on the crossing,
  with `preconditioner` the exact 2×2 inverse (`1/det·[[d,−b],[−c,a]]`).
- `entrywise_form_would_not_have_certified` — same system/box, but compute
  the K image with the OLD entrywise formula in a small inline helper inside
  the test (reproducing the deleted `d[r][c] = δ − y[r][c]·j[r][c]`) and
  assert it does NOT satisfy strict interior containment. This pins the
  regression: the old form must not certify this system.
- `diagonal_system_still_certifies` — a genuinely diagonal 2×2 system
  (`f1 = x² − 1`, `f2 = y² − 4` say, box around (1, 2), width 1e-2) certifies
  `Unique` exactly as before the change (both formulas agree).

H-3: no added bare `1e-N` literals without a same-line `// H-3` opt-out; the
box width 1e-2 in tests must carry `// H-3` on its line.

### 4. Ripple check

The only in-tree users of `krawczyk` are `fid/one_sheet.rs` and
`fid/rep.rs` (and the module's own tests). Their systems have diagonal-ish
Jacobians, so the correction is numerically identical there; the full
truck-evidence suite re-verifies them. Run the whole lib suite, not just the
module tests.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --workspace --all-targets
cargo test -p truck-evidence --lib num::krawczyk --no-fail-fast
cargo test -p truck-evidence --lib --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run a bare `cargo test`.

## Forbidden

Changing the `KrawczykSystem` trait, `krawczyk()`'s control flow, budget or
spent accounting, `preconditioner` semantics, or the `push_children` refusal.
Editing any other file (including `fid/` — the ripple check is read-only
there; report anything surprising in `disagreements` instead). Adding
dependencies.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the file and what you saw
- the coupled test cannot certify after the fix AND you have verified the
  contraction formula by hand → `SPEC_GAP` naming what you observed
- an existing krawczyk test FAILS after the fix (a `Unique` flipping) →
  `SPEC_GAP` — that would mean the change weakened something; stop and report
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. On any
non-`DONE` status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`fix(evidence): Krawczyk contraction is the matrix product, not the Hadamard form (BG-NUM-003-CONTRACT)`.
