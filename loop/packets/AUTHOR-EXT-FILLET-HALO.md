# WORK PACKET AUTHOR-EXT-FILLET-HALO — the fillet recording arm + the closed-loop loft certificate

The owner-authorized extended scope of the authoring arm (annex
EXCLUDED_SIX_DEMAND_MAP section 6.1, decision 2, 2026-09-10). The annex's
original five refusal sites have mostly LANDED since it was measured
(loft/sweep/mirror arms are in door.py; the sweep arm already records a
loft chain per SWEEP-PATH) — the re-measured remaining scope is exactly
two arms: the **fillet recording arm** (the kernel blend machinery is
landed and certified, matrix row 14, but has NO bridge binding and the
door refuses at door.py:2045) and the **closed-loop loft certificate**
(halo's loop: the bridge already carries `Loft{closed}` and
`loft_seam_mismatch` bd_bridge.rs:2524, but the door emits
`"closed": False` unconditionally so the halo demand never reaches the
landed certificate).

```yaml
id:          AUTHOR-EXT-FILLET-HALO
contract:    [AUTHOR-EXT-FILLET-HALO]
class:       design
crates:      [truck123d]
depends_on:  [MONO-7-ROW-ASSEMBLY]
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/binding.rs
  - corpus/ttc/door.py
  - truck123d/tests/fillet_halo_arms.rs
read_allow:
  - truck123d/src/facade.rs
  - corpus/ttc/trees/f1/src/lib/sidepods.py
  - corpus/ttc/trees/f1/src/lib/mono_halo.py
  - corpus/ttc/trees/f1/src/lib/surfaces.py
  - docs/EXCLUDED_SIX_DEMAND_MAP.md
tests_required: [truck123d/tests/fillet_halo_arms.rs]
anchors:
  - {id: A1, expect: 1,  cmd: "grep -c 'def fillet' corpus/ttc/door.py"}
  - {id: A2, expect: 1,  cmd: "grep -c 'def safe_fillet' corpus/ttc/trees/f1/src/lib/surfaces.py"}
  - {id: A3, expect: 0,  cmd: "grep -c 'FilletRow' truck123d/src/binding.rs"}
  - {id: A4, expect: 0,  cmd: "grep -c 'closed_loop' truck123d/src/bd_bridge.rs"}
budget:      {turns: 60, ctx_tokens: 180000}
```

## Pre-made judgements (do not relitigate; deviations go in RESULT notes)

1. **Fillet is a depth-1 op node, not a SolidSpec variant.** The corpus
   applies `safe_fillet(part, edges)` to an EXISTING part after booleans
   (sidepods). Record it as a new `FilletNode` in the tree vocabulary
   beside `BooleanNode` (bd_bridge.rs:353): `{kind: "fillet", base: <part
   node>, radius: f64, edges: [...]}`. Depth-1 only: `base` must be a
   part (a fillet of a group or of another op node refuses typed naming
   the carrier) — same discipline as the depth-1 boolean rule at :345-350.
2. **The binding layer gets a `FilletRow` + `binding_fillet`.** Follow the
   established row pattern (`VolumeRow`/`volume_facts` binding.rs:199,
   :205): a serde row carrying the base solid's recorded carrier data +
   radius + the recorded edge selectors, dispatching into the LANDED
   construct blend machinery (`construct/blend.rs`, `blend_varradius.rs`,
   `setback.rs` — 15 pub fns, matrix row 14). Do NOT modify the kernel
   blend machinery; this packet only EXPOSES it. The facts path returns
   the blend's certified volume/face facts; a blend that cannot close
   refuses typed (the kernel's own refusal vocabulary), never
   approximates.
3. **Edge selectors, pre-decided:** the door records edges as the base
   part's own edge references it can resolve from the recorded vocabulary
   (the corpus's `safe_fillet` iterates an edge list taken from the part
   it fillets). An edge the recorded base cannot resolve refuses typed
   naming the open carrier — typed refusal is a VALID outcome and is the
   expected result for the real sidepods edge set on first landing. Do
   not invent an edge-selection geometry solver here.
4. **`safe_fillet`'s ladder stays client-side.** The descending radius
   ladder (surfaces.py `safe_fillet`) is the caller's retry loop over
   `fillet(edges, r)` for several r — the door arm records ONE radius per
   call. The ladder never crosses the bridge.
5. **Closed loop = exact section coincidence, certified at the facts
   arm.** The door's sweep/loft arms gain `closed=False` kwarg handling:
   `closed=True` is answered only when the caller's section sequence is
   loop-closing (first recorded section == the section that closes the
   chain, exact equality of the recorded profiles) — mono_halo's
   `_stations()` builds left + reversed(left[:-1]) precisely so the wrap
   closes (mono_halo.py:62-66). The recorded row emits `"closed": true`;
   the bridge runs the LANDED `loft_seam_mismatch(sections, true)`
   (:2524) and the facts arm refuses typed when the seam mismatch is
   non-zero (the T1× seam certificate failing is a refusal, never a
   tolerance pass). `closed=False` behavior is unchanged bit-for-bit.
6. **chamfer stays a refusal** (door.py:2050) — it is not in the
   authorized scope.
7. **V5 discipline:** no classifier/facade changes in this packet; the
   fillet op arrives inside the submitted tree, past the op classifier.

## Method

1. Read the landed blend entry points first (construct/blend*.rs pub
   fns) and pick the narrowest certified surface this packet exposes;
   record the choice + reasoning in RESULT notes (this is the packet's
   ONE delegated judgement).
2. binding.rs: `FilletRow` + `binding_fillet` following the row pattern;
   bd_bridge.rs: `FilletNode` + the facts path wiring (per judgement 1).
3. door.py: the fillet arm (replacing the :2045 refusal) and the
   closed-loop handling per judgement 5 (sweep arm :1998, loft arm :2019).
4. Tests in `truck123d/tests/fillet_halo_arms.rs`:
   - a synthetic closed loop (four sections, first == wrap-closer)
     records `closed: true`, facts green with `seam_mismatch` exactly
     zero;
   - the same loop with one section nudged refuses typed naming the
     seam;
   - a fillet of a recorded synthetic base (a box edge set resolvable in
     the recorded vocabulary) returns certified blend facts;
   - a fillet whose base carries an unresolvable edge refuses typed
     naming the carrier;
   - a fillet of a group refuses typed (depth-1 rule).
5. All cargo through the queue. Scoped checks: `cargo check -p
   truck123d --lib --locked` + the new test file + the door-side ttc
   battery. DLL workaround on PATH for the test binary if 0xc0000135.

## Done when

Scoped checks green; the new tests pass serially; anchors hold (A1/A2
unchanged at 1, A3/A4 grow from 0). Write RESULT.json AT THE WORKTREE
ROOT (the slot worktree root, not loop/results/ — the orchestrator
files it).

## Forbidden

No `vendor/truck/**` changes. No facade.rs changes. No chamfer arm. No
edge-selection geometry solver. No tolerance relaxation on the seam
certificate.
