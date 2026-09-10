# WORK PACKET TTC-RECENSUS-F1-R2 — the post-frame/boolean/loft/trim re-census with kernel timing columns

With the full door-gap chain landed (AUTHOR-FRAME-CARRIERS, FRAME-REVOLVE,
SWEEP-PATH, BRIDGE-BOOLEANS, BRIDGE-LOFT-FACTS, TRIM-EXTRUDE-CTOR), re-run
ALL 21 F1 manifest rows through the kernel door. Rows flip green ONLY with
facts matching the recorded references EXACTLY; typed refusals stay recorded
verbatim with the refusing carrier. Every GREEN row gets the BENCHMARKS
timing series (kernel engine only — the OCC columns are the RECORDED
references, no OCC process runs, per owner directive).

```yaml
id:          TTC-RECENSUS-F1-R2
contract:    [TTC-RECENSUS-F1-R2]
class:       mechanical
crates:      [truck123d]
depends_on:  [TRIM-EXTRUDE-CTOR, BRIDGE-LOFT-FACTS, BRIDGE-BOOLEANS]
write_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TT_TIMING_RESULTS.md
read_allow:
  - docs/TTC_CENSUS_FINAL.md
  - docs/TT_TIMING_RESULTS.md
  - corpus/ttc/
tests_required: []
anchors:
  - {id: A1, expect: 48, cmd: "git grep -c '\"id\"' -- corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 21, cmd: "git grep -c median -- docs/TT_TIMING_RESULTS.md"}
budget:      {turns: 65, ctx_tokens: 180000}
```

## Method

1. **The census (PB-011C protocol verbatim):** one fresh python per row,
   serial, quiet machine, kernel engine (`door.py --engine truck`,
   door_version 2). Facts gates compare against the RECORDED references
   (`corpus/ttc/reference/*.json`) — solid_count exact, volume/bbox to the
   recorded doubles, STL triangles. NO OCC process runs anywhere in this
   packet; the recorded references are the sole oracle.
2. **Timing (BENCHMARKS protocol, kernel engine only):** for every row that
   goes GREEN with facts matched: release regime, one unmeasured
   conditioning run, five measured runs, median reported, raw samples
   retained. The release build of the native module happens ONCE at this
   HEAD. Rows refusing typed get NO timing — the typed verdict and refusing
   carrier are the recorded output.
3. **Delta table:** update `docs/TTC_CENSUS_FINAL.md` — per row: prior
   verdict, new verdict, the carrier that cured it (frame carriers /
   booleans / loft facts / trim constructor), or the still-open carrier.
   Rows refusing typed are NOT failures — they are the measured boundary,
   booking evidence for the follow-up packets.
4. **V5 net:** every previously-green row (the three FH timing rows and any
   landed F1 class) must answer bit-identically. A verdict flip on a landed
   row is a defect record, not a census outcome.
5. **Determinism of the timing table:** fresh python per run, no concurrent
   door runs, medians only, no cross-row averages, DNF rows kept.

## Done when

`docs/TTC_CENSUS_FINAL.md` carries all 21 rows' post-chain verdicts with the
delta-vs-prior table; `docs/TT_TIMING_RESULTS.md` carries a kernel timing
column for every green row (medians + raw samples); anchors hold. Expected
shape (from the door-gap chain): the spline-loft class flips green with
facts (engine_cover, beam_wing, drs_flap, floor/diffuser, sidepods, nose —
admission permitting); the boolean-stopped rows (airbox, details,
monocoque, wheel corners) flip green iff admission admits their real
carriers, else refuse typed naming the carrier; the trim rows flip iff the
trim constructor closes on the corpus's actual trim idioms. Whatever the
outcome, the census is SUCCESSFUL if every row is one of: green-with-facts
/ typed-refusal-naming-carrier / DNF-with-recorded-reason.

## Forbidden

OCC process runs. Editing any recorded reference or manifest. Tolerance
stretches. Publishing timing against a red facts gate. Concurrent door
runs. Concurrent cargo builds during measured runs.

## Stop conditions

- A green row's facts drift from the recorded reference → defect record,
  stop-and-file (oracle integrity).
- Zero rows flip green → record the census honestly; the boundary evidence
  is the deliverable (same as the first census).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `census: F1 post-chain re-census — facts-gated flips and kernel timing columns (TTC-RECENSUS-F1-R2)`.
