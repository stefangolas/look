# WORK PACKET DEF-SPINEFRAME-GRAZE-R2 — clamp the search iterates at the sweep's own domain boundary

r1's SPEC_GAP (loop/results/DEF-SPINEFRAME-GRAZE.SPEC_GAP.json — READ FIRST)
measured the truth: the v-domain pad is necessary but NOT sufficient. The
boolean funnel's unclamped Newton iterates wander FAR outside the validated
window on BOTH axes — each pad rescue revealed a deeper iterate, ending at
s = -2.3052e-3 (2300x DirectTolerance::parameter) on the SPINE axis. The
fix belongs at the SEARCH layer: the sweep owns its certified window, so
the search's evaluations must happen inside it.

```yaml
id:          DEF-SPINEFRAME-GRAZE-R2
contract:    [DEF-SPINEFRAME-GRAZE-R2]
class:       design
crates:      [truck-geometry]
depends_on:  []
write_allow:
  - vendor/truck/truck-geometry/src/constructive/profile.rs
  - vendor/truck/truck-geometry/src/decorators/spine_frame.rs
  - vendor/truck/truck-geometry/src/constructive/sweep_surface.rs
  - vendor/truck/truck-geometry/src/constructive/mod.rs
  - vendor/truck/truck-geometry/src/constructive/errors.rs
read_allow:
  - vendor/truck/truck-geotrait/src/algo/surface.rs
  - vendor/truck/truck-shapeops/src/boolean/
  - loop/results/DEF-SPINEFRAME-GRAZE.SPEC_GAP.json
  - docs/defects/SEM-SPINEFRAME-WINDOW-GRAZE-001.md
tests_required:
  - showcases battery_waterslide + pb_contract suites green (the original
    10 panic tests)
  - new regression: a search hitting an out-of-window iterate evaluates
    clamped and returns the certified-window answer or typed refusal
anchors:
  - {id: A1, expect: 5, cmd: "grep -c 'panic!' vendor/truck/truck-geometry/src/decorators/spine_frame.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'DirectTolerance::parameter' vendor/truck/truck-geometry/src/constructive/recipe.rs"}
budget:      {turns: 65, ctx_tokens: 180000}
```

## Scope decisions

1. **Clamp at the sweep's own boundary.** SpineFrameSweep /
   SpineFrameSurface own their validated window
   (`validate_surface_window`); their evaluation paths are only certified
   inside it. Sanctioned implementations, in preference order:
   (a) a domain-clamped evaluation view wrapping the sweep for the SEARCH
   path (iterates clamp into the window before `subs/uder/vder`; the
   accepted result still goes through the residual gate
   `ctx.near_points`, and the clamp is a recorded, named operation);
   (b) a local search implemented in `sweep_surface.rs` over the window
   (keeps truck-geotrait untouched entirely). Pick (a) or (b) on
   simplicity; either must keep the clamp NAMED and RECORDED (no silent
   clamping), per the doctrine.
2. **Re-land the r1 pad** as defense in depth: the three v-domain sites
   (profile.rs:21, spine_frame.rs:156/:176) get the ONE named H-3
   constant sized to the r1 witness class (the r1 RESULT's measured
   ladder is the sizing evidence). The pad is the second net, not the
   fix.
3. **Do NOT** touch truck-geotrait's generic `algo::surface::
   search_parameter` (read-only) unless BOTH (a) and (b) prove
   impractical — then stop and report instead of widening.
4. The `evaluate_position` match-unwrap stays (the doc premise —
   evaluations inside the validated window — is now actually enforced
   by the clamp).

## Done when

```
cargo test -p truck-geometry --lib
cargo test -p showcases --tests
cargo test -p truck-geometry --test constructive_spine_enum (frame-law tests)
```

## Forbidden

vendor/truck outside write_allow. Weakening landed assertions. Silent
clamps. Touching the boolean funnel or the frozen seam.

## Stop conditions

- A showcase test still dies at the match-unwrap with a CLAMPED search →
  the funnel reaches the sweep through a second path: diagnose, record,
  SPEC_GAP with the path named.
- A landed kernel test flips red from the clamp → SPEC_GAP.

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `fix(geometry): clamp seed-search evaluation to the certified sweep window — named, recorded; v-pad as defense in depth (DEF-SPINEFRAME-GRAZE-R2)`.
