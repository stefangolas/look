# WORK PACKET BG-SOL-S7-GFF-CHART - adaptive regular charts for GFF

Recover regular surface/surface crossings that are singular only for the
current fixed z-slab chart. If live code contradicts this packet, report it in
`disagreements`.

```json
{"id":"BG-SOL-S7-GFF-CHART","status":"DONE","contracts":["BG-SOL-S7-GFF-CHART"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-S7-GFF-CHART
contract:    [BG-SOL-S7-GFF-CHART]
class:       design
crates:      [truck-evidence]
write_allow:
  - vendor/truck/truck-evidence/src/contact/gff.rs
read_allow:
  - vendor/truck/truck-evidence/src/contact/implicit.rs
  - vendor/truck/truck-evidence/src/enclosure.rs
  - vendor/truck/truck-evidence/src/num/krawczyk.rs
  - vendor/truck/truck-evidence/src/contact/mod.rs
  - vendor/truck/truck-base/src/evidence.rs
tests_required:
  - adaptive_minor_recovers_regular_horizontal_turn
  - adaptive_minor_is_order_insensitive
  - adaptive_minor_true_tangency_remains_singular
budget:      {turns: 35, ctx_tokens: 90000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn cover_branch' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'struct SlabFF' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A3, expect: 4, cmd: "grep -c 'z_stack' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'singular_boxes.push' vendor/truck/truck-evidence/src/contact/gff.rs"}
  - {id: A5, expect: 0, cmd: "grep -c 'adaptive_minor' vendor/truck/truck-evidence/src/contact/gff.rs"}
```

## Problem

`cover_branch` currently equates failure of the xy Jacobian minor with a
singular contact. That minor is only the z component of the tangent
`grad(f1) cross grad(f2)`. A regular intersection whose tangent is horizontal
has a zero xy minor even when an xz or yz minor is decisively invertible. Such
a box is a chart singularity, not a surface-contact singularity.

Generalize the existing 2x2 slab probe to select one certified regular
coordinate chart from all three minors over the input domain. This packet is
chart-artifact recovery only. A box where no minor excludes zero remains in
`singular_boxes` for later locus-dimension classification.

## Decisions already made

### 1. Proof boundary

For gradients `a = grad(f1)` and `b = grad(f2)`, form the three outward-rounded
2x2 minors, equivalently the components of `a cross b`:

```text
fixed X, solve (y,z):  m_x = a_y*b_z - a_z*b_y
fixed Y, solve (x,z):  m_y = a_z*b_x - a_x*b_z
fixed Z, solve (x,y):  m_z = a_x*b_y - a_y*b_x
```

A minor is usable only when its interval excludes zero. This proves the
corresponding 2x2 Jacobian is nonsingular everywhere in the domain. Merely
having a nonzero midpoint is not proof.

If no minor excludes zero, return the existing successful `BranchCover` shape
with the domain in `singular_boxes`. Do NOT call it proven rank deficiency:
interval dependency or an overly broad box may still be responsible. Do not
add a contact locus, event type, or dispatcher behavior in this packet.

### 2. Deterministic chart selection

For each zero-excluding minor, compute its certified distance from zero:

```text
inf > 0  => inf
sup < 0  => -sup
otherwise unusable
```

Choose the usable minor with the largest distance. Ties choose the lowest
fixed-axis order X, then Y, then Z. The distance only selects among already
certified charts; it does not decide contact topology. Swapping f1/f2 negates
all minors but preserves their distances and therefore selects the same chart.

Name the fixed axis with a small private enum. Match explicitly rather than
runtime-indexing gradient arrays; the crate denies indexing.

### 3. Generalized slab system

Replace the z-specific `SlabFF` with a chart-aware 2x2 `KrawczykSystem`:

- fixed X maps the solver variables to `(y,z)`;
- fixed Y maps them to `(x,z)`;
- fixed Z maps them to `(x,y)` (the current behavior).

Its `f_point`, `jacobian`, exact 2x2 closed-form inverse preconditioner, and
Newton point refinement must all use the same mapping. Spell each three-way
mapping with `match`; do not hide unchecked array indexing in a helper.

Generalize the continuation worklist in the same way: the outer interval is
the fixed coordinate, and the inner Krawczyk box contains the other two domain
intervals. Interval exclusion uses the reconstructed 3-D slab. When the inner
box cannot bisect, bisect the fixed-coordinate interval exactly as the current
z path does. Keep deterministic widest-axis bisection in the two solver
coordinates, ties toward the first solver coordinate.

The chart is selected once over the entire input domain. Because its chosen
minor excludes zero on that domain, it remains valid on every child leaf.
Do not implement a multi-chart atlas or connect cross-sections in this packet.

### 4. Budget and certificates

Preserve the existing caller-owned budget exactly:

- capture entry once;
- all Krawczyk and subdivision spend uses the caller budget;
- exhaustion reports entry minus remaining;
- no fixed private budget and no replenishment;
- the successful certificate remains `Method::Interval`, empty props, actual
  `budget_left`, unbounded margin/modulus.

Field exclusion, unresolved remainder, resolution-floor behavior, point
refinement, and public `BranchCover` fields keep their current semantics.

### 5. Machine-checked witness

Use the unit z-cylinder at the origin and sphere center `(3,0,0)`, radius `3`,
near

```text
p = (1, 0, sqrt(5)).
```

The equations are

```text
f = x^2 + y^2 - 1
g = (x-3)^2 + y^2 + z^2 - 9.
```

At p, `f=g=0`. On a finite box such as
`x in [0.9,1.1], y in [-0.1,0.1], z in [2.1,2.3]`, both field enclosures
contain zero. The old xy minor is `12y` and contains zero, but the xz minor is
`4xz` and is decisively positive (lower bound at least `4*0.9*2.1 = 7.56`).
Therefore the Y-fixed chart is regular and the slice `y=0` contains the unique
root `(x,z)=(1,sqrt(5))` in the box.

Before editing, machine-check these identities and the strict 7.56 minor
margin using the exact formulas above. Record the values in RESULT notes.

## Tests required

1. `adaptive_minor_recovers_regular_horizontal_turn`: use the witness box
   above with a healthy subdivision budget and a scale-appropriate tau.
   Assert non-empty proven points, empty singular/unresolved lists, and that a
   point satisfies both unit-scale equations within a named residual. This test
   must fail on the pre-packet fixed-z implementation because it reports the
   box singular.
2. `adaptive_minor_is_order_insensitive`: run the same domain with cylinder /
   sphere and sphere / cylinder. Assert both covers have no singular or
   unresolved boxes and compare point sets order-insensitively with a named
   unit-scale residual. Do not require discovery order.
3. `adaptive_minor_true_tangency_remains_singular`: unit cylinder and sphere
   center `(2,0,0)`, radius `1`, on a box enclosing `(1,0,0)` and straddling y
   and z. Every cross-gradient component contains zero. Assert the result has
   a singular box containing `(1,0,0)` and no falsely certified point.

Preserve every pre-existing test function name. H-3 rejects an added bare
`1e-N` unless the same line has a `// H-3` comment.

## Done when

```console
cargo fmt --check -p truck-evidence
cargo clippy -p truck-evidence --all-targets --no-deps
cargo check --locked -p truck-evidence --all-targets
cargo test -p truck-evidence --lib contact::gff --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

## Forbidden

Editing outside `write_allow`; changing `ImplicitField`, Krawczyk, enclosure or
dispatcher APIs; adding dependencies; changing `BranchCover` public fields;
claiming boxes are truly singular when all minors merely contain zero; adding
event/locus types; connecting points into curves; accepting unresolved boxes;
adding `#[ignore]`; loosening a gate; changing the GATE-4 ceiling; renaming or
deleting a pre-existing test.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- the regular horizontal-turn witness cannot certify under a healthy budget
  after checking all coordinate mappings -> `SPEC_GAP` with measured output;
- the true tangency produces a certified regular point -> `SPEC_GAP`;
- swapped field order selects a different fixed axis -> `SPEC_GAP`;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
