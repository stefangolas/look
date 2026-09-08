# BREP-002-DSC-SCREENS — STOP: ANCHOR_MISMATCH (the same correction already landed in this tree)

Status: STOPPED at the packet's stop condition #1 — *"any anchor count differs
→ ANCHOR_MISMATCH"*. No code written and no test added; the write targets
(`boolean/assemble.rs`, `tests/defect_regressions.rs`) were left untouched.

## Anchors re-checked on this branch (`packet/BREP-002-DSC-SCREENS` @ `9db9232`)

| id | file | pattern | expected | measured |
|---|---|---|---|---|
| A1 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_aabb` | 1 | **0** |
| A2 | `truck-shapeops/src/boolean/assemble.rs` | `fn face_uv_box` | 1 | 1 |
| A3 | `truck-evidence/src/enclosure_sweep.rs` | `impl EnclosureSurface for SpineFrameSweep` | 1 | 1 |

A1 differs: `assemble.rs` on this branch no longer defines `fn face_aabb`
(only a doc comment at line 546 mentions the name). The packet was authored
against the pre-fix `face_aabb`/`face_uv_box` boundary-sampling screens
(anchors measured 2026-09-06 morning), but the dispatch base is the
integration/kernel-bg tip, which already carries the fix for the **same
defect record** (`docs/defects/DSC-BOUNDARY-SAMPLE-EXTENT-001.md`) from the
overlapping row **CFP-001-LIFT-ENCLOSURE** (feat `930db75`, landed via
`865432e`/`fb8b7d0`/`9db9232`; note in `loop/PACKETS.jsonl`:
"same defect … same file … probe shows the exact pair now clashing").

## Evidence that the defect's correction is already present at this base

- `assemble.rs` at HEAD carries the CFP-001 certified lift screen:
  `certified_surface_box` (L550), `face_enclosure` (L579),
  `boundary_enclosure_accumulate` (L592), `bulging_carrier` (L402),
  `natural_extent` (L418), and `face_uv_box` returning the trim's true
  parameter extent for bulging carriers — the exact "Replace the sampled
  extents with certified enclosures" scope decision 2 of this packet, with the
  doc header "CFP-001 decision 3".
- The lift (`lift_faces`/`sweep_contact_events`) consumes `face_enclosure`
  AABBs, not a boundary-sample `face_aabb`.
- The defect record's own status block already reads "Correction landed —
  validated by F-C0/F-C2" / "Corrected by `CFP-001-LIFT-ENCLOSURE`".
- The CFP-001 unit tests pinning the fix are already present in
  `assemble.rs` (`fc0_defect_counterexample_red_on_sampled_green_on_certified`,
  `param_box_derived_from_trim_not_boundary`,
  `face_enclosure_used_by_lift_screen`, per CONTEXT.md).

## Why ANCHOR_MISMATCH (not SPEC_GAP, not DONE, not code work)

The packet's precondition — the defect-era `face_aabb` boundary-sampling
screen still present so it can be replaced — does not hold on this branch,
because the identical correction already landed here. Re-running the
replacement would relitigate landed CFP-001 work on the hot file and is
exactly the clash this packet's own dispatch note warns about. Under the
packet's stop conditions, an anchor count differing is a hard stop; nothing is
missing from the write set that would make the packet constructible (so not a
SPEC_GAP).

## Recommended unblock

Re-derive the BREP-002-DSC-SCREENS row against a pre-CFP-001 base (where
`fn face_aabb` = 1 still holds and the DSC correction is absent), or fold the
row's remaining work (the two required tests
`dsc_boundary_sample_extent_001_sphere_cap_screen_admits_the_contact` and
`dsc_boundary_sample_extent_001_planar_faces_screen_unchanged` in the missing
`tests/defect_regressions.rs`) into the already-landed CFP-001 row and close
BREP-002 as superseded. The DONE-side RESULT template's
`"tests_added":2` was not met, so no RESULT.json with status DONE is filed.
