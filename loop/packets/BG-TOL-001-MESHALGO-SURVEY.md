# WORK PACKET BG-TOL-001-MESHALGO-SURVEY — classify, do not change

You are producing a **survey**, not a code change. Everything you need is in
this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md` or any other
spec file.** If something you need is genuinely missing, that is a SPEC_GAP (see
"Stop conditions"): you stop and report.

```yaml
id:          BG-TOL-001-MESHALGO-SURVEY
contract:    [BG-TOL-001]
class:       survey
crates:      [truck-meshalgo]
depends_on:  [BG-TOL-001-TYPE]
write_allow:
  - SURVEY.json
read_allow:
  - vendor/truck/truck-meshalgo/src/**
  - vendor/truck/truck-base/src/tolerance.rs
tests_required: []
budget:      {turns: 60, ctx_tokens: 150000}
```

## What this is and why it is not a code change

`truck-meshalgo` holds 30 tolerance predicates across 20 functions, and each one
has to be classified **`model`** (a length in model space — it must scale when
the model does) or **`param`** (dimensionless — a ratio, a sine, an angle, a
normalized parameter, a weight, a scale factor — it must *not* scale). That
classification is the entire value of a Stage-A migration shard and it is the
one part that cannot be done mechanically: it needs someone to read the
surrounding code and work out what the quantity physically *is*.

**Your output is a proposal, not a decision.** The orchestrator reviews every
row and writes the migration packet afterwards. You have **no write access to
`vendor/truck/**`** — do not edit a single Rust file. A survey that changes code
is rejected outright.

Because it is a proposal, **a disagreement is more valuable than a guess.** If a
site is genuinely ambiguous, say so in its `reason` and mark your confidence
low. Do not resolve an ambiguity by picking whichever answer sounds tidier.

## How to classify

Read `vendor/truck/truck-base/src/tolerance.rs` first — the `ToleranceCtx`
methods carry doc comments that state which quantities are dimensionless and
why. The rule in one line:

> If the number would change when the same part is exported in metres instead of
> millimetres, it is **`model`**. If it would not, it is **`param`**.

Worked examples, all from sites already migrated in this tree:

| expression | class | why |
|---|---|---|
| `p.near(&q)` on two `Point3` in model space | `model` | a distance between points |
| `rxy.so_small()` where `rxy = point - center` | `model` | a length |
| `v.cross(axis).so_small()`, `v` a displacement, `axis` unit | `model` | `\|v\| sin θ` still carries length units |
| `axis.cross(normal).so_small()`, both unit vectors | `param` | a pure sine |
| `r0.near(&r1)` where both are `transform[i].magnitude()` | `param` | scale factors are dimensionless |
| `v < u - TOLERANCE`, `u`,`v` angles on a unit circle | `param` | an angular margin |
| `kv.range_length().so_small()` | `param` | a knot range is parameter space |
| `pt[3].so_small()` on a homogeneous coordinate | `param` | a weight |

### Four things that are not `model` or `param`, and are not judgement calls

Each of these has already cost the loop a round trip. Classify them `excluded`
with the stated reason and do not agonise over them.

**1. A quantity that is not a length at all.** `model` means degree **1** in
length. If the quantity is degree 2 — a cross-product magnitude (twice a
triangle's area), a scalar triple product, a `Matrix3::determinant()` of two
displacements — then under a rescale by `k` it goes as `k²` while
`ctx.length_margin()` goes as `k`, and `is_small_len` on it is a migration that
is exactly right today and silently wrong the moment a real `model_scale` is
threaded. There is no predicate for it. Mark `excluded`, reason
`degree 2 in length (area)`, `proposed_rewrite` `null`. **If your own reason
contains the words "squared", "area", or "length-squared", the classification is
not `model`** — that sentence is the test, and a survey has already failed it by
writing "a length-squared quantity that scales with the model" and then
proposing `is_small_len` anyway.

**2. A value, not a comparison.** A `const` item (`const FOO: f64 = TOLERANCE;`),
a `use` import, a `.max(TOLERANCE)` floor, a `+ TOLERANCE` offset, a
spatial-hash bucket pitch. These compare nothing, so they have no class. A
`const` initializer in particular has no `ctx` in scope, so any `ctx.` rewrite
you propose for one cannot compile. Mark `excluded`, reason `not a predicate:
<what it is>`. Their *consumers* may well be sites; classify those.

**3. Squared-order sites** — any `near2` / `so_small2` / `TOLERANCE2` site;
`ToleranceCtx` has no squared-order predicate, and mapping one onto the
first-order `tau_rep` loosens it by six orders of magnitude while looking like a
migration. Deferred to BG-TOL-004; reason `squared order`.

**But recognise a squared-order site by its CONSTANT, not by its shape.**
`d.distance2(c) <= TOLERANCE * TOLERANCE` is *not* one: that is algebraically
`distance <= TOLERANCE`, a perfectly ordinary first-order predicate written
squared to skip a `sqrt`. It migrates, to `is_small_len` or `is_small_ratio` on
the un-squared distance. What cannot migrate is a comparison against the
*tighter* `TOLERANCE2` = 1e-12 token, because nothing on `ToleranceCtx`
reproduces that number. One survey excluded a live site by getting this
backwards.

**4. Not code:** anything inside a `#[cfg(test)]` module, a doc comment (`///`,
`//!`), or a `/* */` block. A test's own epsilon is the test's business, and a
doc example is prose. **Check for these before classifying** — a previous packet
listed a line inside a `/* */` block spanning 160 lines and a worker dutifully
migrated a comment.

## Where the sites are

30 production predicates in 20 functions. This inventory comes from
`loop/census_tol_sites.py` and is your starting point, not your answer — find
every site yourself with

```
grep -nE '\.near2?\(|so_small2?\(|TOLERANCE2?' <file>
```

| file | functions with a site |
|---|---|
| `src/tessellation/triangulation.rs` | `try_new`, `reconcile_singular_transition`, `singular_transition_branch`, `normalize_range`, `working_range`, `new_with_join`, `end_pts`, `on_boundary`, `include_along_ray`, `triangulation_into_polymesh_outcome`, `polyline_on_surface` |
| `src/analyzers/collision.rs` | `sorted_endpoints`, `collide_seg_triangle` |
| `src/analyzers/in_out_judge.rs` | `is_crossing` |
| `src/analyzers/point_cloud/mod.rs` | `distance2_point_triangle` |
| `src/analyzers/point_cloud/sort_end_points.rs` | `sorted_endpoints` |
| `src/filters/normal_filters.rs` | `normalize_normals` |
| `src/vtk.rs` | `hash_point`, plus one site at **file scope** |
| `src/tessellation/source_edge.rs` | one site at **file scope** |

The two file-scope sites sit outside any function — a `const` or a `static`.
Report them with `"symbol": "<file scope>"` and say in `reason` what the
constant is used for, because that determines whether it is a predicate at all.

`triangulation.rs` is very large. Work through it by grep hit, not by reading it
front to back, and do not load the whole file into context at once.

## Your output — `SURVEY.json` in the root of your worktree

One object, with a `sites` array. **Every field is required on every row.**

```json
{
  "id": "BG-TOL-001-MESHALGO-SURVEY",
  "crate": "truck-meshalgo",
  "sites": [
    {
      "file": "vendor/truck/truck-meshalgo/src/analyzers/collision.rs",
      "line": 79,
      "symbol": "sorted_endpoints",
      "expression": "if t.so_small() {",
      "classification": "param",
      "reason": "t is a normalized ray parameter in [0,1], not a distance",
      "confidence": "high",
      "proposed_rewrite": "ctx.is_small_ratio(t)"
    }
  ]
}
```

- `file` is **repo-relative** and must exist; `line` is 1-based and the
  `expression` must actually appear on it. A gate checks all three against the
  tree — an invented line number fails the packet.
- `expression` is the source line, trimmed. It may be a fragment.
- `classification` is exactly one of `model`, `param`, `excluded`.
- `reason` says what the quantity physically is, in one sentence. "It is a
  length" is not a reason; "it is the distance from the sample point to the
  triangle plane" is.
- `confidence` is `high`, `medium` or `low`. **Use `low` freely** — a low-
  confidence row that names the ambiguity is worth more than a high-confidence
  guess, and the orchestrator reads the low ones first.
- `predicates_on_line` is how many tolerance predicates the source line
  carries. Usually 1. Count them: `if !a.near(&b) && c.so_small()` is **2**.
- `proposed_rewrite` is the **complete replacement for the whole condition**,
  covering every predicate on the line — not just the one that decides the
  branch. For `excluded`, write `null`.

  **This is the field that has already gone wrong, and it is the reason a row
  can be right and useless at the same time.** A survey met a line reading

  ```rust
  if !previous_uv.x.near(&current_uv.x) && surface.uder(u, v).so_small() {
  ```

  and proposed `ctx.is_small_len(surface.uder(u, v).magnitude())`. That is the
  correct migration of the deciding predicate and it **deletes the guard**: a
  worker applying it verbatim changes what the function does. The survey knew —
  it said so in its own `reason` — and had nowhere to put it, because it was
  writing one rewrite per row.

  When the predicates on a line are of **different classes**, set
  `classification` to the one that decides the branch, set
  `mixed_classification` to `true`, and write both into `proposed_rewrite`:

  ```json
  "classification": "model",
  "mixed_classification": true,
  "predicates_on_line": 2,
  "proposed_rewrite": "!ctx.is_small_ratio(previous_uv.x - current_uv.x) && ctx.is_small_len(surface.uder(u, v).magnitude())",
  "reason": "the deciding test is the u-derivative magnitude (model-space length); the !near guard compares u parameters (dimensionless)"
  ```

  A row with `predicates_on_line` greater than 1 whose `proposed_rewrite`
  migrates fewer of them than that is the single defect this packet most wants
  you to avoid.

Include a `"functions"` count and, if you find sites the inventory above missed,
a `"not_in_inventory"` array naming them — that is a finding, not an error, and
the previous survey's three entries were all real. **Look for them deliberately**:
the inventory is built by a regex requiring a word boundary before `TOLERANCE`,
so a constant named `SOURCE_INCIDENCE_TOLERANCE` or `RELATIVE_TOLERANCE` is
invisible to it. Grep your crate for `_TOLERANCE` and `TOLERANCE_` yourself
and report what the inventory does not contain.

## Done when

Every grep hit in the eight files is accounted for by exactly one row —
classified or `excluded`. Nothing under `vendor/truck/` is modified:
`git status --porcelain` shows only `SURVEY.json`, `RESULT.json` and
`PACKET.md`. Commit `SURVEY.json` on the current branch.

You do not run `cargo` at all. There is nothing to build.

## Forbidden

Editing **any** file under `vendor/truck/`, or any file other than
`SURVEY.json` and `RESULT.json` — this is the whole point of a survey class and
it is checked. Editing `loop/` anything: your files go in the root of your
worktree. Migrating a site. Adding a `ToleranceCtx` call to any Rust file.
Committing to `main`.

## Stop conditions

- a file in the inventory does not exist, or a listed function is not in it →
  `ANCHOR_MISMATCH`, naming the file and what you found
- a site's physical meaning cannot be determined from the code you are allowed
  to read → **do not stop.** Emit the row with `"confidence": "low"` and say in
  `reason` exactly what you could not determine. That is a successful survey.
- the classification rule itself does not decide a case — e.g. a quantity that
  is a length times a dimensionless factor, where either answer changes
  behaviour → `SPEC_GAP`, naming the site. This is the one thing worth stopping
  for, and it is the most valuable output this packet can produce.

## Finish by writing `RESULT.json` in the root of your worktree

```json
{"id":"BG-TOL-001-MESHALGO-SURVEY","status":"DONE","contracts":["BG-TOL-001"],
 "sites_surveyed":0,"functions":0,"low_confidence":0,"excluded":0,
 "notes":"fill in the counts. Say which sites you found genuinely ambiguous and why, and whether any grep hit did not fit the model/param/excluded trichotomy at all -- that last one is the finding this packet most wants."}
```

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`. On any non-`DONE`
status also write `QUESTION.md` beside it.

Commit on the current branch with subject
`survey(meshalgo): classify 30 tolerance sites model or param (BG-TOL-001-MESHALGO-SURVEY)`.
