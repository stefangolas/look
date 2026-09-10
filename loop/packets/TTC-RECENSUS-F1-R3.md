# WORK PACKET TTC-RECENSUS-F1-R3 — the re-census under the kernel-internal oracle policy

The R2 census (0 green / 15 typed / 6 DNF) was adjudicated under the OLD
policy — facts compared EXACTLY against the recorded OCC references. The
owner's ORACLE POLICY CHANGE (annex C of docs/MONO_CLOSURE_BOOKING.md,
commit 4e6694d) dissolves that gate: **kernel certificates ARE the
certification.** A row is GREEN when the kernel constructs it AND its
volume carries its own certificate bracket AND solid_count is
constructive AND bbox is carrier-derived AND the mesh emits. The
recorded OCC references are DIAGNOSTICS — reported per row, gating
nothing. The 1e-4 band vs OCC is retired as a gate (retained as a
reported delta). This packet re-judges every F1-family manifest row —
including the six rows staged tonight (front_wing, cockpit, nose,
sidepod_left, sidepod_right, halo) — under the new policy, with the
MONO-7 row-assembly facts/timing and the fillet/closed-loop arms landed.

```yaml
id:          TTC-RECENSUS-F1-R3
contract:    [TTC-RECENSUS-F1-R3]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-7-ROW-ASSEMBLY, AUTHOR-EXT-FILLET-HALO]
write_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TT_TIMING_RESULTS.md
  - docs/MONO_CLOSURE_BOOKING.md
  - docs/EXCLUDED_SIX_DEMAND_MAP.md
  - corpus/ttc/
tests_required: []
anchors:
  - {id: A1, expect: 54, cmd: "grep -c '\"id\"' corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 21, cmd: "grep -c median docs/TT_TIMING_RESULTS.md"}
  - {id: A3, expect: 1,  cmd: "grep -c 'kernel-internal' docs/MONO_CLOSURE_BOOKING.md"}
budget:      {turns: 70, ctx_tokens: 200000}
```

## Method

1. **The green predicate (the whole policy in one line):** kernel
   constructs + volume certificate bracket present + solid_count
   constructive + bbox carrier-derived + mesh emits. For spline-carrier
   rows the bracket is the landed certified volume machinery (the
   `VolumeRow` facts with their own enclosure); for canonical rows it is
   the constructive facts path. The OCC reference files are read and
   REPORTED (volume_rel delta as a diagnostic column) — a mismatch never
   flips a verdict.
2. **The staged six run FIRST for references-as-diagnostics:** run
   `python corpus/ttc/record_references.py`-equivalent recording for
   f1/front_wing, f1/cockpit, f1/nose, f1/sidepod_left,
   f1/sidepod_right, f1/halo IF the OCC door can record on this machine;
   if the recording fails environmentally, record the reference as
   absent-diagnostic and run the kernel census anyway. Under the new
   policy a missing reference cannot block a row.
3. **The census (PB-011C protocol verbatim):** one fresh python per row,
   serial, quiet machine, kernel engine (`door.py --engine truck`,
   current door_version). Every row lands in exactly one class:
   - **GREEN** — the predicate in method 1 holds. Record: facts, the
     certificate bracket (lo/hi) where the carrier is spline-class, the
     timing columns from MONO-7's `bd_facts` (construct_ms/facts_ms;
     mesh_ms for the mesh), and the OCC delta as diagnostic.
   - **TYPED REFUSAL** — the refusing carrier named verbatim. NOT a
     failure: the measured boundary and the booking evidence for the
     follow-up packet.
   - **DNF** — with the recorded reason.
4. **Timing (BENCHMARKS protocol) for GREEN rows only:** release regime,
   one unmeasured conditioning run, five measured runs, median, raw
   samples retained. The release build happens ONCE at this HEAD. The
   timing columns come from the bridge's own `timing` output plus the
   wall-clock series; both recorded.
5. **Delta table:** update `docs/TTC_CENSUS_FINAL.md` — per row: the R2
   verdict, the R3 verdict, WHAT changed the verdict (MONO-7 row
   assembly / the fillet / closed-loop arms / the policy re-judgement
   itself), or the still-open carrier. Rows whose verdict did not move
   are recorded too — a re-census that moves nothing is a result.
6. **V5 net:** rows green under R2 must remain green under R3 (the
   policy only WIDENS what can go green — a flip from green to
   typed/DNF is a defect record, not a census outcome).
7. **Determinism:** fresh python per run, no concurrent door runs,
   medians only, no cross-row averages, DNF rows kept in the table.

## Done when

`docs/TTC_CENSUS_FINAL.md` carries all 54 rows' R3 verdicts under the
kernel-internal predicate with the delta-vs-R2 table and per-row
diagnostics (OCC delta, certificate bracket, timing);
`docs/TT_TIMING_RESULTS.md` carries the kernel timing series for every
green row; anchors hold. SUCCESS is every row being one of:
green-under-the-predicate / typed-refusal-naming-carrier /
DNF-with-recorded-reason.

## Forbidden

No OCC gate resurrected under another name (no "within band of the
reference" pass condition — the delta is reported, never compared). No
verdict flips on rows green under R2. No kernel changes (this packet
writes two docs files). No concurrent door runs during timing.
