# QUESTION.md — CC-013-CORRESPONDENCE

**Status: STOP — stop condition (1): the S9 seam stub types `WireComplex` and
`ShiftFunctional` are uninhabitable for CC-013 production, and the
`Correspondence` carrier the resolver must return does not exist anywhere in
the crate.**

This is the third dispatch of CC-013 onto the same unamended seam. The first
two attempts stopped identically on this branch (slot 0 commits `2019b2e` and
`29ff8fc`, both "loop: CC-013-CORRESPONDENCE STOP - S9 stub seam defect
(WireComplex/ShiftFunctional uninhabitable; Correspondence absent)"); their
QUESTION.md files were caught in the evidence-loss incident closed by
`afd6575` / `1ee3123`, no stub amendment landed, and the row stayed READY and
was re-dispatched. Every fact below was re-derived by command on this
dispatch's tree (base `56ef2eb` = `integration/kernel-bg`).

## What the packet requires

Section 1 (`wire_complex_of`) must give `WireComplex` its production meaning:
"an oriented cyclic sequence of matched edges with per-vertex positions as
`Interval` enclosures", built by `pub fn wire_complex_of(arc_count: usize,
vertices: &[[Interval; 3]]) -> Result<WireComplex, ConstructRefusal>`
(arc_count >= 2; vertex count == arc_count). Section 2
(`resolve_correspondence`) must then walk the fixed order — caller anchor →
combinatorially forced unique isomorphism (r = 2, or a labeled asymmetric
vertex pattern) → argmin over the r cyclic shifts of the DECLARED
`VertexSumSq` functional → refuse — reading per-vertex data off `&WireComplex`
wires and the anchor/functional off `&ShiftFunctional`, passing the r
enclosures to `argmin_margin` (S5), and returning a `Correspondence`. Section
2 states that `Correspondence` "is the CC-000 stub type; here it is CONSUMED,
not defined — read `stubs.rs` first." Section 3 requires orientation-reversing
matches to be taken only on an explicit caller-supplied anchor.

Stop condition (1): "`WireComplex`/`ShiftFunctional`/`Correspondence` stubs own
the type surface — extend nothing in `stubs.rs`; if a field you need is
missing, file QUESTION.md (spine seam defect)."

The five required tests all assemble their inputs through these types:
`anchor_supplied_wins`, `unique_isomorphism_resolves_without_argmin`,
`four_arc_two_circle_shift_resolves_by_separated_argmin`,
`overlapping_shift_values_refuse_ambiguous_correspondence`, and
`orientation_reversal_requires_explicit_caller_consent`.

## What the landed stubs provide

`vendor/truck/truck-certified/src/construct/stubs.rs:85-117`
(CC-000-CONTRACT, landed; last touched by CC-005's accepted S6 amendment,
`ade5c45`):

```rust
#[derive(Debug, Clone)]
pub struct WireComplex {
    /// Sealed. Production data is CC-013's design.
    _sealed: (),
}

impl WireComplex {
    /// The refusing stub constructor (C7): production belongs to CC-013.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

#[derive(Debug, Clone)]
pub struct ShiftFunctional {
    /// Sealed. Production data is CC-013's design.
    _sealed: (),
}

impl ShiftFunctional {
    /// The refusing stub constructor (C7): production belongs to CC-013.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}
```

The public surface of each type is exactly one refusing constructor. Neither
type has any data field reachable outside `stubs`, no constructor that accepts
wire/functional data, and no accessor. Because `_sealed: ()` is private to
`stubs`, no other module in the crate — including CC-013's `correspondence.rs`
— can construct or read a `WireComplex` or a `ShiftFunctional`, even by
attaching an inherent `impl` block. An `impl WireComplex` written from
`correspondence.rs` cannot touch a private field.

There is also no `Correspondence` type anywhere in the crate, and no
`VertexSumSq` discriminant, no `wire_complex_of`, and no
`resolve_correspondence`. A tree-wide search over `vendor/**/*.rs` finds only
the two stub struct declarations above (`stubs.rs:90`, `stubs.rs:107`); every
other reference is packet text, the S9 seam record in
`docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md` §3, or the
`AmbiguousCorrespondence` refusal variant in `construct/refusal.rs`. The
construct-module doc list and `construct/mod.rs` module declarations confirm
no `correspondence` module exists and no `pub mod correspondence;` line is
present.

`tests/construct_contract.rs:15-17,209-210` (CC-000) imports `WireComplex` and
`ShiftFunctional` from `stubs` and asserts both `try_new()` constructors
refuse `ConstructRefusal::Unfrozen` (the refusing markers must survive any
amendment).

## Why this is a stop, not a workaround

- Section 1's `wire_complex_of` cannot build any real wire: `WireComplex` has
  no habitable carrier for `arc_count` and the per-vertex `[Interval; 3]`
  positions, and its only constructor always refuses `Unfrozen`.
- Section 2's resolver cannot read a wire's vertices to evaluate the
  `VertexSumSq` functional over the r cyclic shifts, and cannot read an
  anchor / orientation / functional discriminant off `ShiftFunctional` —
  neither stub exposes data.
- Section 2's resolver cannot return a `Correspondence`: the type is absent,
  and the packet forbids defining it here ("CONSUMED, not defined"). Its seam
  consumers (CC-012 / CC-014 per spine S9) cannot type against it either.
- None of the five required tests can assemble its inputs.
- The fix means editing `stubs.rs`, which is outside this packet's
  `write_allow` and which stop condition (1) explicitly freezes ("extend
  nothing in `stubs.rs`"). Duplicating parallel public types under the same
  names inside `correspondence.rs` is not an available reading either: it
  would split the S9 seam identity (the cross-packet contract types are
  `construct::stubs::{WireComplex, ShiftFunctional}` — anchors A3/A4) and the
  packet ties its production to the CC-000 stub types.

This is the exact defect class CC-005-GRAPHDISK stopped on (session 51): a
CC-000 seam stub uninhabitable for its owning wave packet (private `_sealed`,
refusing constructor only). That question was accepted and resolved by the S11
`TripleContactNode` posture — frozen `pub` fields landed in `stubs.rs` by the
owning packet (the file now carries `BoundaryPlan { pub boundary_simple,
pub seams_glued }` plus a refusing `try_new()`), the CC-000 contract test kept
asserting the refusing constructor. CC-013 is the S9 analogue, with the added
missing record (`Correspondence`).

## Proposed amendment (minimal, stub-owner, mirroring the accepted CC-005 one)

1. Widen CC-013's `write_allow` to include
   `vendor/truck/truck-certified/src/construct/stubs.rs`.
2. In `stubs.rs`, give `WireComplex` and `ShiftFunctional` frozen PUB-field
   production shapes (the S11 posture already in that file), keeping the
   refusing `try_new()` markers so `tests/construct_contract.rs:209-210`
   stays green. Required surfaces:
   - `WireComplex` constructible with, and readable for, `arc_count` plus
     per-vertex `[Interval; 3]` positions (cyclic: vertex count == arc_count);
   - `ShiftFunctional` carrying the declared closed functional discriminant
     (`VertexSumSq`; the enum stays closed — the packet names it
     `ShiftFunctional::VertexSumSq` and reserves other functionals as later
     amendments), plus the optional caller-supplied anchor and its explicit
     orientation (orientation-preserving automatic path; reversing only when
     the caller supplied it in the anchor).
3. Land the missing `Correspondence` record. The frozen S9 seam shape from
   `docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md` §3 is
   `pub struct Correspondence { pub orientation: bool, pub anchor: usize,
   pub shifts: Vec<usize> }`. Either freeze it in `stubs.rs` with that shape,
   or — if the owner instead assigns CC-013 ownership of the carrier — amend
   the packet's Section 2 wording ("CONSUMED, not defined") and record the
   seam impact on CC-012 / CC-014.

Exact spelling is the stub owner's; the required surface is the three items
above. Then redispatch CC-013 unchanged: `correspondence.rs` will construct
wires and functionals through the pub shapes and read them in the resolver.
`tests/construct_contract.rs` needs no change in either resolution (the
refusing `try_new()` markers are kept).

## Pre-stop verification (re-derived by command on this dispatch)

- Anchors A1..A4 all match on this tree: A1 `pub fn argmin_margin` in
  `construct/argmin.rs` = 1; A2 `pub struct LoftStrips` in
  `construct/loft_strips.rs` = 1; A3 `pub struct WireComplex` in
  `construct/stubs.rs` = 1; A4 `pub struct ShiftFunctional` in
  `construct/stubs.rs` = 1.
- `pub struct Correspondence`, `pub enum ShiftFunctional`, `VertexSumSq`,
  `pub fn wire_complex_of`, `pub fn resolve_correspondence`: zero definitions
  anywhere under `vendor` (grep over `vendor/**/*.rs`).
- `construct/mod.rs` has no `pub mod correspondence;` declaration.
- `tests/construct_contract.rs` (CC-000) asserts the refusing constructors of
  both stubs and imports both names from `stubs`; it would stay green under
  the proposed amendment.
- Dispatch base is `56ef2eb` (`integration/kernel-bg`); the worktree is clean
  apart from the dispatch-provided `PACKET.md` / `CONTEXT.md` and this file
  (then committed).
- No kernel code was written: `construct/correspondence.rs`,
  `construct/mod.rs`, and `tests/construct_correspondence.rs` are untouched.
