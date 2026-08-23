# WORK PACKET BG-FID-008-r2 — sound disc membership and floor-complete enumeration

You are amending one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

**Your prior work is already committed on this branch** (commit `c6a833e`,
"feat(evidence,fid): one-sheet condition (iv-a) for curves via Krawczyk fibre
isolation (BG-FID-008)"). It passed every mechanical gate. It is NOT being
thrown away: this packet amends two specific soundness defects in it, both of
which originate in the ORIGINAL packet's text, not in your implementation of
it. Your RESULT.json disagreements #2 and #3 diagnosed exactly these defects;
this packet is the adjudication: you were right, the packet prose was wrong,
and the shipped code must now match the sound reading.

```json
{"id":"BG-FID-008-r2","status":"DONE","contracts":["BG-FID-008"],
 "tests_added":7,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-FID-008-r2
covers:      [BG-FID-008]
contract:    [BG-FID-008]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/fid/one_sheet.rs
  - vendor/truck/truck-evidence/src/fid/mod.rs
read_allow:
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/elementary.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/num/roots.rs
  - vendor/truck/truck-evidence/src/fid/lfs.rs
budget:      {turns: 30, ctx_tokens: 100000}
anchors:
  # Pinned to THIS branch's tip c6a833e with `git show`, because the packet is
  # dispatched onto a branch carrying prior work (the main worktree does not
  # have one_sheet.rs). A count mismatch is a stop condition (ANCHOR_MISMATCH).
  - {id: R1, expect: 3, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'DISC_DECIDE_WIDTH'"}
  - {id: R2, expect: 1, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'fn box_distance'"}
  - {id: R3, expect: 0, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'fn sup_distance'"}
  - {id: R4, expect: 4, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'SINGLE_SHEET_RADIUS'"}
  - {id: R5, expect: 1, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/one_sheet.rs | grep -c 'KrawczykProof::Unique'"}
  - {id: R6, expect: 1, cmd: "git show c6a833e:vendor/truck/truck-evidence/src/fid/mod.rs | grep -c 'pub mod one_sheet'"}
```

## Problem — the two soundness reversals, precisely

The module's public docs promise **certified** fibre cardinality: `NotOne.count`
is "the CERTIFIED lower bound on distinct geometric intersections", `ExactlyOne`
is "exactly one approximant point on the closed normal disc". Both promises are
currently breakable by inputs none of the shipped tests exercise. This is the
same defect class the loop killed BG-FID-001's first dispatch for: unsound
bounds that every test passes.

### Defect A — disc-membership inclusion decided by the INFIMUM distance

`box_distance(a, b)` (yours, duplicated from lfs.rs) is the **infimum**
point-set distance: the gap between the boxes, zero when they overlap. The
shipped decision at the narrow stop is `box_distance(image, x) <= eps`, which
proves only that the box **intersects** the closed ball. The certified root
lies *somewhere in the box*, so the root itself may be up to the box's image
radius BEYOND `eps` while the test passes. The original packet's step 4
parenthetical ("whole box inside the closed ball ⇒ the root lies in-disc")
described the containment reading and prescribed the intersection test — the
conflation was the packet's, and your disagreement #3 said so.

The false certificate, end to end: a single-sheet approximant of radius
`R + eps + 2e-5` crosses the normal plane once near the disc, at distance
`eps + 2e-5` — a **coverage violation** (true in-disc count 0, the equally
fatal case the enum documents). A width-`1e-4` decision box around that
crossing has image radius ≈ `R * 1e-4 = 2e-4`, so its inf-distance is about
`eps + 2e-5 - 2e-4 < eps`: the root is certified in-disc, count = 1, and the
function returns `Ok(ExactlyOne)` — a false degree-one certificate on a
coverage violation. The convergence argument in the `DISC_DECIDE_WIDTH` doc
comment is true for the shipped witnesses (their margins are ≥ 3e-3) but it is
a witness statement, not a certificate.

### Defect B — the narrow stop discards UNEXAMINED box regions

The shipped NUM-003 `krawczyk` returns `Unique` on the FIRST internal sub-box
that proves strict interior containment (krawczyk.rs: `return Ok(... Unique
...)` inside the loop) — it does not enumerate the rest of the query box. So
`Unique` on a box `tt` certifies "at least one root in `tt`, exactly one in
some sub-box of `tt`" — never "exactly one in `tt`". Your disagreement #2
diagnosed this and your worklist correctly keeps subdividing a WIDE `Unique`
box; but the narrow stop (`width <= DISC_DECIDE_WIDTH` → count 1 and DROP
`tt`) reintroduces the same hole at ≤ 1e-4 scale: the unexamined region of
`tt` may hold a second root. An approximant whose plane coordinate crosses
zero twice within `1e-4` of parameter (a tiny wiggle — exactly the adversarial
geometry this checker exists for) certifies `ExactlyOne` with two distinct
in-disc intersections. `DISC_DECIDE_WIDTH` must go; the only principled stop
is the width floor.

## Decisions already made for you

### Decision 0 — API and semantics UNCHANGED, one doc paragraph added

`FibreMultiplicity`, `OneSheetError`, `fibre_degree_one` signatures, the
`@feeds-open-lemma` / `@establishes` / `@does-not-establish` blocks, and the
verbatim `@precondition BG-FID-003 (i)-(iii)` line all stay exactly as you
wrote them. Add ONE paragraph to the module docs stating the resolution limit
honestly: distinct roots separated by less than `WIDTH_FLOOR` in parameter are
counted once, and a root whose distance to the disc boundary is below its
floor-box image radius refuses as `SheetCountUnresolved`. A certificate that
states its own resolution is stricter than one that does not.

### Decision 1 — the engine, replaced (prunes stay, stops change)

Keep: the witness/tangent derivation, the unbounded-range refusal, the
`FibreSystem` (f_point / jacobian / preconditioner exactly as shipped — they
were right), the h-interval prune (step 1) and the prune at
`box_distance(image, x) > eps` (step 2 — infimum > eps proves the WHOLE box
beyond the ball; sound as written), `push_children`, `WIDTH_FLOOR`.

Replace the per-box body after the two prunes with:

1. `krawczyk(&system, &[tt], budget)` returns:
   - `NoRoot` → discard `tt` (the whole box is root-free; sound).
   - `Unique` → `tt` holds ≥ 1 root with an unexamined remainder →
     **subdivide `tt` unconditionally** (unless at the floor, case 3). The
     child holding the root re-proves `Unique`; empty children prove `NoRoot`;
     the same root re-found from adjacent sub-boxes merges by the dedupe rule.
   - `Err(_)` → indeterminate; if `width > WIDTH_FLOOR` subdivide, else
     `Err(SheetCountUnresolved)`.
2. **There is no width shortcut. `DISC_DECIDE_WIDTH` is deleted** (const,
   comment, and both uses).
3. **Terminal case** — a popped box with `width <= WIDTH_FLOOR` that returns
   `Unique` contributes exactly one root at floor resolution and takes the
   DISC DECISION on its image `B = approx.enclose(tt)`:
   - `sup_distance(B, x) <= eps` → every point of `B` is in the closed ball,
     so the root (in `B`, and `h == 0` puts it on the normal plane) is IN the
     disc → count it (dedupe by point-box overlap against already-counted
     boxes; count > 1 → early exit `NotOne { count }`).
   - `box_distance(B, x_b) > eps` → root outside the disc → does not count.
   - neither (sup > eps AND inf <= eps: the box straddles the sphere) →
     `Err(SheetCountUnresolved)`. Closed-ball membership for a point on the
     sphere is not certifiable by interval arithmetic; guessing a direction is
     exactly the false pass this module refuses.
4. Worklist drained: count == 1 → `ExactlyOne`; else `NotOne { count }`
   (0 included).

`sup_distance` (add it; the R3 anchor pins its current absence):

```text
sup_distance(B, x) = sqrt( Σ_i max( (B.i.inf() − x_i)², (B.i.sup() − x_i)² ) )
```

per axis `(b.x.inf() - p.x).abs().max((b.x.sup() - p.x).abs())`, squared,
summed, `sqrt` — the farthest corner of the box from `x`, an upper bound on
every point's distance. Write the comparisons in the exact forms above
(`<= eps`, `> eps`); no negated comparisons.

### Decision 2 — dedupe rule, kept, with its resolution stated

Two counted roots whose point-boxes overlap are the same geometric point at
floor resolution and count once (a closed curve re-hits at `t*` and
`t* + period`; bisection re-finds the same root across sibling boundaries).
The overlap merge is sound at the floor because two non-overlapping floor-box
images certify points farther apart than the floor image radius. This is part
of the resolution-limit paragraph from Decision 0. No code change beyond what
Decision 1 already describes.

### Decision 3 — tests (same module, all floats named consts with same-line `// H-3:`)

Exact circle radius `R = 2`, `eps = 0.05`, witness `t_x ≈ 0.7` — unchanged.
Your `Circle`, `DoubleCover`, `Tangential`, `Cusp` local curves and every
existing const stay.

1. `single_sheet_circle_certifies_degree_one` — **witness changes**: radius
   `R + eps/2` (`SINGLE_SHEET_RADIUS = RADIUS + 0.5 * DISC_RADIUS`). The
   crossing at `t_x` sits at distance exactly `eps/2 = 0.025`, decidably
   in-disc; the antipodal crossing at `2R + eps/2` is excluded. `Ok(ExactlyOne)`.
   (The old radius `R + eps` put the crossing exactly ON the sphere — the one
   distance interval arithmetic cannot decide; see test 5.)
2. `double_cover_witness_refuses` — unchanged: `NotOne { count: 2 }`. The
   in-disc crossings sit at `eps * cos(t_x/2) ≈ 0.04697`, margin ≈ 3.0e-3
   from the boundary — decidably in; the crossings near `t_x + π`, `t_x + 3π`
   sit ~`2R` out — excluded by the (sound) inf prune.
3. `offset_sheet_outside_disc_ignored` — unchanged: `NotOne { count: 0 }`.
4. `tangential_contact_is_unresolved_not_degree_one` — unchanged (your
   parabola witness was correct): `Err(SheetCountUnresolved)`, never `Ok`.
5. **NEW** `boundary_root_on_disc_edge_is_unresolved` — the OLD test-1
   witness, now asserting the strict behaviour: circle of radius `R + eps`
   over `[0, 2π]`, crossing at `t_x` at distance exactly `eps`. Every box
   around it has `sup > eps` and `inf <= eps` at every width, so the run must
   drain to `Err(SheetCountUnresolved)` — NEVER `Ok`. This is the regression
   test for Defect A: an implementation that decides inclusion by
   `inf <= eps` returns `Ok(ExactlyOne)` here and fails this test.
6. `zero_budget_refuses_unresolved` — unchanged.
7. `invalid_witness_refuses` — unchanged.

Machine-check every reference number again with a script BEFORE writing
RESULT.json (crossing distances 0.025 / `eps*cos(0.35)` / `3*eps` / `eps`;
double-cover distinctness `2*eps*cos(t_x/2)` ≈ 0.0939 between the two in-disc
points), through THIS module's curve formulas, not a scratch variant.

### Decision 4 — module layout, unchanged

Same two files in `write_allow`; `fid/mod.rs` stays at exactly one added line
plus its doc note; `#![deny(clippy::unwrap_used)]` including the test module;
local helpers duplicated, enclosure.rs visibility untouched.

## H-3, the house rule that rejects bare float literals

GATE-2 of `scripts/kernel-gates.sh` fails any **added** line carrying a bare
`1e-N` literal unless that same line ends with an `// H-3` comment. EVERY new
or changed epsilon, radius, margin and slack is a named const whose defining
line carries a same-line `// H-3:` comment naming the dimensionless quantity.
Run `bash scripts/kernel-gates.sh <your base>` before writing RESULT.json.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo test -p truck-evidence --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh fc8925f    # base = the ORIGINAL packet's base; your branch carries its commit
```

truck-evidence is green at baseline. Any baseline failure you did not cause is
a stop condition. Send cargo output to a file and read the tail. Never run a
bare `cargo test`.

## Forbidden

Editing files outside `write_allow`. Reintroducing any width-based decision
shortcut (the defect this packet removes). Deciding disc inclusion by
infimum/intersect distance. Naming anything certificate/isotopy/homeomorphism-
flavored beyond the types already shipped. Claiming any bridge lemma as proved.
Bare float literals without `// H-3`. `unwrap()`/`expect()` on fallible
production paths. Weakening or deleting an existing test to make this pass.
Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- the floor-resolution semantics cannot be expressed without changing a
  signature in `write_allow` → `SPEC_GAP` naming the mismatch
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch (which already carries your `c6a833e`) with subject

```
fix(evidence,fid): sound disc membership by containment and floor-complete enumeration (BG-FID-008-r2)
```
