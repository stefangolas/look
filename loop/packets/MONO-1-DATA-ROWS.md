# WORK PACKET MONO-1-DATA-ROWS — kernel-native probe data rows in the drop-in

The corpus helpers (`corpus/ttc/trees/f1/src/lib/surfaces.py`) probe
`shape.wrapped` for `bbox`, `obox`, `is_valid_shape` and face counts. On
kernel-engine rows the drop-in's `wrapped` refuses typed (an OCC probe of a
kernel-engine row is not a kernel-engine row) or returns `None` (a kernel
`Face`), so `f1/monocoque` (`surfaces.obox` via `mono_tub.py:526`),
`f1/engine_cover` (`surfaces.bbox` via `engine_cover.py:163`) and the four
corner rows (`surfaces.bbox` on a kernel `Face` -> OCP `TypeError`) die at
probes instead of at real carriers. This packet lands kernel-native data rows
so the probes answer from the kernel's own certificates.

```yaml
id:          MONO-1-DATA-ROWS
contract:    [MONO-1-DATA-ROWS]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - truck123d/src/binding.rs
  - truck123d/src/marshal.rs
read_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - corpus/ttc/trees/f1/src/lib/surfaces.py
  - corpus/ttc/trees/f1/src/lib/mono_tub.py
tests_required: []
anchors:
  - {id: A1, expect: 8,  cmd: "grep -c '\\<bbox\\>' truck123d/src/binding.rs"}
  - {id: A2, expect: 0,  cmd: "grep -c 'obox' truck123d/src/binding.rs"}
  - {id: A3, expect: 0,  cmd: "grep -c 'is_valid' truck123d/src/binding.rs"}
  - {id: A4, expect: 0,  cmd: "grep -c 'n_faces' truck123d/src/binding.rs"}
budget:      {turns: 45, ctx_tokens: 140000}
```

## Method

1. `bbox` / `obox` on kernel rows: carrier-derived **rigorous** box. Mechanism:
   per Bernstein patch, the control net's coordinate-wise interval hull bounds
   the patch (convex hull property); where hull tightness exceeds the corpus
   guards' slack (mono_tub checks against a 6 mm margin), subdivide the patch
   (exact Bernstein subdivision, geometry-preserving) until the union hull is
   tight enough — target: bound within 1 mm of a dense surface sampling on the
   packet's fixtures. `obox` = the same bound in the recorded frame the corpus
   helper expects (it consumes `lo, hi` corner points). Every returned number
   is a certified bound, never a sampling.
2. `is_valid_shape` on kernel rows: kernel rows are constructive; validity is
   the landed closed/oriented 2-cycle invariant — return it from the existing
   invariant, never a probe.
3. Face count row: the number of patches of the row's boundary grid.
4. `Face`/`Shell` rows in the drop-in carry the same data-row surface (the
   corner rows call `surfaces.bbox` on a lofted `Face`).
5. All cargo through the queue (the `cargo` on PATH IS the queue shim). No OCC
   process anywhere in this packet.

## Done when

- `cargo check --locked -p truck123d` clean; `cargo test --locked -p
  truck123d --lib -- --test-threads=1` green including the new tests:
  - `bbox_is_control_hull_bound` (a known patch, bracket tightness asserted),
  - `bbox_subdivides_to_slack` (tightness within 1 mm of dense sampling),
  - `validity_row_answers_constructive_invariant`,
  - `face_count_row_matches_grid`,
  - the refusal path stays typed (`an OCC probe` wording only where the row
    genuinely cannot answer).
- `rustfmt --check` clean on touched files; clippy zero findings on added
  lines.
- Anchors: A1 may drift (post-work note the count); A2/A3/A4 move 0 -> >= 1.

## Stop conditions

- If a corpus helper needs a probe the certificate cannot answer rigorously,
  stop and record the helper + line — the row's refusal is then REAL carrier
  work, not probe work. Typed refusal, no approximation.

Write RESULT.json AT THE WORKTREE ROOT.
