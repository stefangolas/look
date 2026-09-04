# QUESTION.md — CC-005-GRAPHDISK

**Status: STOP — Section 3 spine seam defect: the S6 `BoundaryPlan` stub
exposes no accessor for boundary simplicity.**

## What the packet requires

Section 1 (`certify_graph_disk`) decides, as the third pre-made rule, that a
boundary that is not simple must refuse `Err(StarNotEmbedded)` — with the
simplicity verdict read off the `boundary: &BoundaryPlan` argument ("boundary
not simple (`BoundaryPlan` says so)"). Section 3 states that `BoundaryPlan`
(the CC-000 S6 stub) "gains its PRODUCTION meaning here without changing the
stub file: `certify_graph_disk` consumes the stub's opaque verdict through the
accessor CC-000 books — read `construct/stubs.rs` first and use exactly the
accessor it exposes; if the stub lacks an accessor for boundary simplicity,
STOP and file QUESTION.md (spine seam defect)."

So the packet assumes the landed stub exposes an accessor that returns the
boundary-simplicity verdict of an opaque plan. The Section 2 projection search
discharges boundary simplicity through the near-diagonal planar machinery
(`formal/xmonotone` A3, `formal/intersection`, the P2
`curve_injectivity_radius` plane-projected analogue); that discharge must
produce a `BoundaryPlan` value carrying the verdict, and
`certify_graph_disk` must consume it.

## What the landed stub provides

`construct/stubs.rs:119-135` (CC-000-CONTRACT, landed, post-amendment tree):

```rust
/// The S6 boundary-simplicity input plan (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-005 graph-disk packet lands its production from
/// the planar machinery.
#[derive(Debug, Clone)]
pub struct BoundaryPlan {
    /// Sealed. Production data is CC-005's design.
    _sealed: (),
}

impl BoundaryPlan {
    /// The refusing stub constructor (C7): production belongs to CC-005.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}
```

The public surface of `BoundaryPlan` is exactly one refusing constructor. It
has **no accessor** returning a boundary-simplicity verdict, and it has **no
constructor that accepts a verdict**. Because the only field (`_sealed: ()`)
is private to `stubs`, no other module in the crate — including a CC-005
`graphdisk.rs` — can construct or read a `BoundaryPlan`, even by adding an
inherent `impl` block. The contract document (`CERTIFIED_CONSTRUCTION_
CONTRACTS.md` §3, seam S6) freezes the signature
`certify_graph_disk(pieces: &[DiskPiece], boundary: &BoundaryPlan)` and says
only that `BoundaryPlan` is "frozen in CC-000 as a stub type; its production
comes from the planar machinery ... inside CC-005, not across the seam" — it
does not name any landed accessor either.

## Why this is a stop, not a workaround

- `certify_graph_disk`'s third decision ("boundary not simple (`BoundaryPlan`
  says so)") is unimplementable: there is no way to read a simplicity verdict
  off the argument.
- No `BoundaryPlan` value can exist anywhere in the program (`try_new()` is
  the only constructor and always returns `Err(ConstructRefusal::Unfrozen)`),
  so no required test can even assemble the `boundary: &BoundaryPlan`
  argument — `genuine_star_certifies`,
  `folded_corner_refuses_no_admissible_projection_or_star_not_embedded`,
  `non_simple_boundary_refuses_star_not_embedded`,
  `unglued_seam_refuses_star_not_embedded`, and the two search tests all
  need a plan value or a produced plan.
- The Section 2 discharge cannot build the plan: the planar-machinery
  simplicity verdict has no carrier type to land in (`stubs::BoundaryPlan`
  is unconstructible and unreadable outside its module).
- Adding the accessor/constructor means editing `stubs.rs`, which is outside
  this packet's `write_allow`. The packet itself classifies this exact gap as
  a STOP: "if the stub lacks an accessor for boundary simplicity, STOP and
  file QUESTION.md (spine seam defect)."

Stop condition (1) is NOT the blocker: the CC-000 fixtures `genuine_star`
(A1 = 1) and `folded_corner` (A2 = 1) carry DiskPiece-shaped records whose
ground truths hold by direct data evaluation (positive/negative det_lower
bounds, seam flags, per-piece boundary_simple flags — machine-checked in
`tests/construct_contract.rs::fixture_ground_truths_hold`). The blocker is
purely the missing S6 accessor.

## Proposed amendment (minimal, stub-owner)

Give `BoundaryPlan` the consumable S6 surface, in the CC-000-owned
`stubs.rs`, mirroring the S11 posture precedent already in that file
(`TripleContactNode`: frozen `pub` fields + a refusing `try_new()` that the
CC-000 contract test keeps asserting):

```rust
/// The S6 boundary-simplicity input plan (production shape, CC-005).
///
/// The verdict field is frozen here (seam S6); values are produced by the
/// CC-005 planar machinery and consumed by `certify_graph_disk`. The
/// refusing constructor below is the C7 stub posture for callers that have
/// no verdict yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryPlan {
    /// Whether the projected boundary is simple (a Jordan curve).
    pub boundary_simple: bool,
}

impl BoundaryPlan {
    /// Construct a plan from a certified boundary-simplicity verdict.
    pub fn from_boundary_simple(boundary_simple: bool) -> Self {
        Self { boundary_simple }
    }

    /// The certified boundary-simplicity verdict of the plan.
    pub fn boundary_is_simple(&self) -> bool {
        self.boundary_simple
    }

    /// The refusing stub constructor (C7): kept so the CC-000 contract test
    /// `radius_law_stubs_carry_no_default_construction` stays green.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}
```

(Exact spelling is the stub owner's; the required surface is: a constructor
from a `bool` verdict or a `pub` verdict field, plus an accessor or `pub`
field that `certify_graph_disk` reads.) Then redispatch CC-005 unchanged:
`graphdisk.rs` will construct the plan from its planar discharge and read the
verdict in `certify_graph_disk`. If the resolution instead makes CC-005 the
owner of `BoundaryPlan`'s production shape, widen CC-005's `write_allow` to
include `vendor/truck/truck-certified/src/construct/stubs.rs` and note the
impact on the CC-000 contract test that asserts `BoundaryPlan::try_new()`
refuses.

## Pre-stop verification

- Anchors A1..A4 all matched the expected counts on this tree before the stop
  (A1 `pub fn genuine_star` in fixtures.rs = 1, A2 `pub fn folded_corner` in
  fixtures.rs = 1, A3 `pub fn make_x_monotone` in xmonotone.rs = 1, A4
  `pub fn injectivity_radius` in injectivity.rs = 1).
- `grep -c 'BoundaryPlan'` over the tree: only the packet text, the contract
  doc, the CC-000 stub file, its contract test (`assert_stub_refuses(
  BoundaryPlan::try_new())`), and the stub/contract records — no accessor
  anywhere.
- No kernel code was written; `construct/graphdisk.rs`,
  `construct/mod.rs`, and `tests/construct_graphdisk.rs` are untouched. The
  worktree is clean apart from this file and the dispatch-provided
  `PACKET.md` / `CONTEXT.md` (then `RESULT.json`, written after the commit).
