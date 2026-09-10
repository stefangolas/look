# WORK PACKET MONO-2-NSTATION-LOFT — the certified N-station smooth loft (pinned OCCT convention)

BRIDGE-LOFT-FACTS certifies only two-station smooth lofts (its honesty line:
a 3+-station smooth loft's station parameterization is a convention the
recorded data does not carry). The convention is now PINNED by measurement
(`docs/MONO_CLOSURE_BOOKING.md` annex A — synthetic-probe evidence): OCCT
`BRepOffsetAPI_ThruSections` smooth lofts interpolate sections with
**chord-length station parameters**, **exact section hit** (u-unification is
geometry-preserving knot insertion), and a v-direction that is a **global
polynomial of degree N-1 for N <= 9** and a **C2 cubic spline with knots AT
the chord-length station parameters for N >= 10**. This packet generalizes
the landed two-station certified arm to N stations under that law.

This is the dominant carrier of the F1 family: the post-chain census (R2)
names it the first refusing carrier of beam_wing, drs_flap, power_unit,
track_rod x2, floor, diffuser — and it is one of monocoque's three lofts.

```yaml
id:          MONO-2-NSTATION-LOFT
contract:    [MONO-2-NSTATION-LOFT]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - truck123d/src/bd_bridge.rs
read_allow:
  - truck123d/src/binding.rs
  - truck123d/src/facade.rs
  - docs/MONO_CLOSURE_BOOKING.md
  - corpus/ttc/trees/f1/src/lib/surfaces.py
tests_required: []
anchors:
  - {id: A1, expect: 3, cmd: "grep -c 'certified_spline_loft_volume' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'ThruSections' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 57, cmd: "grep -c '\\<loft\\>' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 170000}
```

## Method

1. **Sections, exactly.** Reuse the landed section reconstruction (the
   two-station arm's per-section loop: line/spline spans, chord-length clamped
   cubics, seam closure). Sections arrive as recorded data — no flattening.
2. **u-unification, exactly.** The section splines are unified to a common
   u-knot structure by knot insertion ONLY (geometry-preserving). No
   approximation anywhere in the u direction.
3. **Station law.** v_i = chord-length parameter of section i (cumulative
   centroid-to-centroid distance, normalized to [0,1] — probe annex A3).
4. **v-interpolation.** N <= 9: the unique global polynomial of degree N-1
   interpolating the section control rows at v_i, degree-elevated to Bernstein
   per span (no spans — one v segment). N >= 10: the C2 cubic interpolating
   spline with knots AT the v_i (the same banded second-derivative system
   OCCT solves; the annex pins the system). Both routes end in the same
   representation: a grid of bicubic tensor-Bernstein patches.
5. **Certified volume.** V = sum over patches of
   `crate::python::binding::volume_facts` (the sanctioned plain-data entry,
   one `VolumeRow` per patch) + the exact planar end-cap moments (the landed
   Green's-theorem cap terms, unchanged). Brackets sum rigorously (outward
   rounding).
6. **bbox + mesh.** Exact span extrema via the landed `cubic_roots` machinery,
   extended to the v spans; deterministic tessellation generalizes
   `spline_loft_mesh` (rows on the surface's own knots, per the corpus's
   section-law note).
7. **Fixtures (one-time, committed).** Record a small synthetic suite with
   OCC ONCE (the probe protocol, `scratch/probe_thrussections_*.py` is the
   pattern): N in {3, 5, 9, 10, 16} synthetic section stacks — recorded
   volume/bbox committed as fixtures. The kernel gate: facts within the
   recorded band, plus the kernel's own certificate brackets (the certificate
   is the primary oracle; the OCC fixture proves convention fidelity).
   NO OCC run happens inside the kernel or tests at runtime.
8. All cargo through the queue. `corpus/ttc/reference/*.json` and
   `corpus/ttc/MANIFEST.json` are read-only.

## Done when

- `cargo check --locked -p truck123d` clean; `cargo test --locked -p
  truck123d --lib -- --test-threads=1` green including:
  - `line_loft_rows_answer_bit_identically` (the landed two-station regression
    stays green, UNMODIFIED),
  - `nstation_volume_matches_recorded_fixture` for every N fixture,
  - `nstation_certificate_brackets_reference` (kernel bracket contains the
    recorded value, width within the band),
  - `nstation_refuses_still_typed_for_open_chains` (open chains keep the
    landed typed refusal),
  - `nstation_42_station_timing_kernel_class` (a 42-station synthetic completes
    in kernel-class time — the tub's scale).
- fmt/clippy clean on added lines; the two-station fast path remains the
  bit-identical entry for N = 2.
- Anchors re-measured and recorded in RESULT.json (A3 will drift — record the
  post-work count with the drift note; the drift is this packet's own prose).

## Stop conditions

- If OCCT's N=9/10 degree-law boundary or the knots-at-stations system does
  not reproduce on a fixture (i.e., the annex law is wrong at some N), STOP,
  record the failing N and the observed structure — the annex is amended
  before the packet proceeds. Never stretch a band to fit.
- If a section stack's u-unification cannot be completed by knot insertion
  alone (the corpus's crease-corner sections are two cubics meeting at an
  angle — verify they unify), record the exact obstruction; that is new
  carrier work, not tolerance territory.

Write RESULT.json AT THE WORKTREE ROOT.
