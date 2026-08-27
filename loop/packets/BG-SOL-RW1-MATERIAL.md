# WORK PACKET BG-SOL-RW1-MATERIAL - the material-state primitive

Land the Boundary Rewrite's heart first, pure and standalone: the §13.1
material-state fragment-selection primitive. No topology, no geometry, no
classification - just the four material witnesses, the Boolean truth
functions, and the keep/discard/orient decision, with the classical
orientation table reproduced as the soundness check. If live code
contradicts this packet, report it in `disagreements`.

```json
{"id":"BG-SOL-RW1-MATERIAL","status":"DONE","contracts":["BG-SOL-RW1-MATERIAL"],
 "tests_added":0,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],"notes":"free text"}
```

```yaml
id:          BG-SOL-RW1-MATERIAL
contract:    [BG-SOL-RW1-MATERIAL]
class:       design
crates:      [truck-shapeops]
write_allow:
  - vendor/truck/truck-shapeops/src/boolean/mod.rs
  - vendor/truck/truck-shapeops/src/lib.rs
read_allow:
  - vendor/truck/truck-shapeops/src/transversal/faces_classification/mod.rs
  - vendor/truck/truck-shapeops/src/transversal/loops_store/mod.rs
tests_required:
  - material_state_reproduces_regularized_orientation_table
  - material_state_decides_coincident_fragments
  - material_state_flips_orient_toward_the_empty_side
budget:      {turns: 18, ctx_tokens: 60000}
anchors:
  - {id: A1, expect: 0, cmd: "ls vendor/truck/truck-shapeops/src/boolean 2>/dev/null | wc -l"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub mod fillet' vendor/truck/truck-shapeops/src/lib.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'mod transversal' vendor/truck/truck-shapeops/src/lib.rs"}
  - {id: A4, expect: 0, cmd: "grep -rc 'BoolOp' vendor/truck/truck-shapeops/src/lib.rs"}
  - {id: A5, expect: 0, cmd: "grep -rc 'MaterialState4' vendor/truck/truck-shapeops/src/lib.rs"}
```

A1 becomes 1 (the new directory with `mod.rs`); A2 and A3 stay; A4 and A5
become 1 (the `pub use` re-exports - or omit the re-export and keep them 0;
your choice, but `pub mod boolean;` is required so the module is public).

## Problem

The plan's Phase 4 (Boundary Rewrite) and the spec's §13.1 fix the
primitive: material state, not an orientation table. Encode the four
material witnesses around one boundary fragment, evaluate the Boolean
truth function on each side, keep the fragment iff the two sides differ
(it is on the result's boundary), and orient it toward the empty
(`m_R = 0`) side. No case enumeration. This packet lands exactly that
primitive in a new `truck-shapeops` module, with the soundness check the
spec demands: it must reproduce the classical orientation table for two
regularized solids in general position - and must decide the coincident
cases the classical table cannot express.

## Decisions already made

### 1. Module

`vendor/truck/truck-shapeops/src/boolean/mod.rs` (new; the crate gains
`pub mod boolean;` in `lib.rs` - one line, nothing else in lib.rs
changes). The crate warns on `missing_docs`: every public item carries a
doc comment. Carry the H-1 deny header inside the module like the
truck-evidence modules do (`clippy::unwrap_used`, `clippy::expect_used`,
`clippy::panic`, `clippy::todo`, `clippy::unimplemented`,
`clippy::indexing_slicing`). Pure logic: no imports beyond
`std`-level (nothing from truck-topology/geometry; this module decides,
it does not touch shapes).

### 2. The vocabulary (plan Phase 4 + spec 13.1, exact shapes)

```rust
/// Material membership of one side of a boundary fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    /// The side is inside solid A.
    pub in_a: bool,
    /// The side is inside solid B.
    pub in_b: bool,
}

/// The regularized Boolean operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    /// Union.
    Union,
    /// Intersection.
    Intersection,
    /// Difference: A minus B.
    Difference,
    /// Symmetric difference.
    Xor,
}

impl BoolOp {
    /// The truth function: whether a point in this state is material in
    /// the result.
    pub fn eval(&self, s: State) -> bool {
        match self {
            BoolOp::Union => s.in_a || s.in_b,
            BoolOp::Intersection => s.in_a && s.in_b,
            BoolOp::Difference => s.in_a && !s.in_b,
            BoolOp::Xor => s.in_a ^ s.in_b,
        }
    }
}

/// The four material witnesses around ONE boundary fragment, in the
/// fragment's own orientation: `-` is the side its normal points AWAY
/// from, `+` the side it points TO. For a fragment of A's boundary the
/// A pair is `(true, false)` (inside, outside); a coincident fragment
/// carries all four from the classification stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialState4 {
    /// The `-` side is inside A.
    pub a_minus: bool,
    /// The `+` side is inside A.
    pub a_plus: bool,
    /// The `-` side is inside B.
    pub b_minus: bool,
    /// The `+` side is inside B.
    pub b_plus: bool,
}

/// What the Boundary Rewrite does with one boundary fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentDecision {
    /// The fragment is on the result's boundary. `flip` says whether its
    /// orientation must be reversed so the normal points toward the
    /// empty (`m_R = 0`) side.
    Keep { flip: bool },
    /// Both sides have the same result material: interior or exterior of
    /// the result - the fragment is not on the boundary.
    Discard,
}

/// The §13.1 primitive: keep iff the sides differ; orient toward the
/// empty side.
pub fn fragment_decision(op: BoolOp, m: MaterialState4) -> FragmentDecision;
```

### 3. The decision (already derived - do not relitigate)

```text
m_R_minus  = op.eval(State { in_a: m.a_minus, in_b: m.b_minus })
m_R_plus   = op.eval(State { in_a: m.a_plus,  in_b: m.b_plus  })
if m_R_minus == m_R_plus -> Discard
otherwise                -> Keep { flip: !m_R_minus }
```

`flip = !m_R_minus` because `m_R_minus != m_R_plus`: when the `-` side is
material (`true`) the empty side is `+` and the stored orientation already
points at it (no flip); when the `-` side is empty the normal points into
material and must be reversed.

## Tests required

1. `material_state_reproduces_regularized_orientation_table`: the 16-cell
   classical table for two regularized solids in general position. A
   fragment of A's boundary has `(a_minus, a_plus) = (true, false)`; a
   fragment of B's boundary has `(b_minus, b_plus) = (true, false)`. Four
   fragment classes:
   - A's face outside B: `(A: 1,0; B: 0,0)`;
   - A's face inside B: `(A: 1,0; B: 1,1)`;
   - B's face outside A: `(A: 0,0; B: 1,0)`;
   - B's face inside A: `(A: 1,1; B: 1,0)`.
   For each of the four ops assert the classical outcome:
   - Union keeps A-outside-B and B-outside-A unflipped, discards the rest;
   - Intersection keeps A-inside-B and B-inside-A unflipped, discards the
     rest;
   - Difference (A−B) keeps A-outside-B unflipped and B-inside-A FLIPPED,
     discards the rest;
   - Xor keeps A-outside-B, B-outside-A, A-inside-B FLIPPED, and
     B-inside-A FLIPPED (each solid's own exterior faces plus the cavity
     walls re-oriented outward of the respective remainder).
   The A-inside-B-flipped Xor cell is worth one inline comment deriving it
   from the truth function (it is the cell most likely to be
   misremembered): xor keeps the side pairs
   `op(1,1)=0 / op(0,1)=1` -> keep, flip = !0 = true.
2. `material_state_decides_coincident_fragments`: the cases the classical
   table cannot express, all decided with NO special-casing:
   - identical orientation (`A: 1,0; B: 1,0`): Union keeps unflipped
     (A∪A=A), Intersection keeps unflipped, Difference discards
     (A−A=∅), Xor discards;
   - anti-oriented (`A: 1,0; B: 0,1` - the solids butt against each other
     at this fragment): Union discards (the fragment is interior to the
     union), Intersection discards, Difference (A−B) keeps unflipped,
     Xor keeps unflipped (each side is in exactly one solid);
   - `A: 1,1; B: 1,1` (a coincident fragment fully interior to both -
     degenerate but decidable): Union/Intersection keep... derive each
     from the rule and assert what the rule says, with a one-line
     comment per cell.
3. `material_state_flips_orient_toward_the_empty_side`: for a set of
   `MaterialState4` values covering every `Keep` cell above, assert the
   definitional property directly: with `m_R_minus != m_R_plus`, the
   kept fragment's outward side (after applying `flip`) is the `false`
   side. Also assert `Discard` iff `m_R_minus == m_R_plus` by evaluating
   both sides through `BoolOp::eval`.

Preserve every pre-existing test function name. H-3 does not apply (no
float literals anywhere in this module).

## Done when

```console
cargo fmt --check -p truck-shapeops
cargo clippy -p truck-shapeops --all-targets --no-deps
cargo check --locked -p truck-shapeops --all-targets
cargo test -p truck-shapeops --lib boolean --no-fail-fast
bash scripts/kernel-gates.sh <your base commit>
```

Never run bare `cargo test` or a workspace-wide cargo command.

**Commit your work on the current branch** (subject
`shapeops: material-state fragment-selection primitive (BG-SOL-RW1-MATERIAL)`)
**before** writing `RESULT.json`: the verifier measures the committed
diff, and an uncommitted tree reads as an interrupted run.

## Forbidden

Editing outside `write_allow`; importing truck-topology, truck-geometry,
or truck-base types (the module is pure); adding an orientation table,
case enumeration, tolerances, or floats; adding classification,
splitting, or assembly logic (later packets); adding `#[ignore]`;
loosening a gate; changing the GATE-4 ceiling; renaming or deleting a
pre-existing test.

## Stop conditions

- anchor mismatch -> `ANCHOR_MISMATCH` with observed count;
- the classical table cannot be reproduced from the rule -> `SPEC_GAP`
  with the failing cell and both derivations;
- three consecutive cargo failures with one cause -> `BLOCKED`.

Finish by writing `RESULT.json` in the worktree root, not `loop/results/`.
Record in `notes` the full 16-cell table you asserted (op x class ->
keep/discard/flip) so a reviewer can diff it against this packet's prose.
