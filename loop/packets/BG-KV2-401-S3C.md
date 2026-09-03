# BG-KV2-401-S3C — trim clipping: R9 crossings + winding classification (section 9.4)

Wave-4 packet (build spec section 4; section 19 row 15; spec section 9.4
VERBATIM order). The clip between the certified leaf-product 1-complex and
the trimmed faces: certified R9 crossings, arc splitting, winding-number
inside/outside classification (the SOUND use — a closed plane curve about
a point certified OFF the loop), discard outside, TrimCrossing nodes.

```yaml
id:          BG-KV2-401-S3C
contract:    [BG-KV2-401-S3C]
class:       design
crates:      [truck-certified]
depends_on:  [BG-KV2-202-S1A, BG-KV2-303-S9A]
write_allow:
  - vendor/truck/truck-certified/src/kernel/trimclip.rs
  - vendor/truck/truck-certified/src/kernel/mod.rs
  - vendor/truck/truck-certified/tests/kernel_trimclip.rs
read_allow:
  - docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
  - vendor/truck/truck-certified/src/kernel
  - vendor/truck/truck-certified/src/kernel/residuals_r89.rs
budget:      {turns: 28, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct R9System' vendor/truck/truck-certified/src/kernel/residuals_r89.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub struct CertifiedGraph' vendor/truck/truck-certified/src/kernel/graph.rs"}
  - {id: A3, expect: 0, cmd: "grep -c 'trimclip' vendor/truck/truck-certified/src/kernel/mod.rs"}
tests_required:
  - r9_crossings_certify_between_arc_pcurve_and_trim_loop
  - arc_splits_at_certified_trim_crossings
  - winding_classification_inside_and_outside_on_fixture
  - interior_loop_crossing_no_leaf_boundary_is_clipped
  - sample_certified_off_the_loop
  - depth_max_failure_refuses_trim_clip_failed
  - no_transcendental_call_in_trimclip_module
```

Section 1: `pub fn trim_clip(graph: &CertifiedGraph, trims: &[TrimLoop])
-> Construction<CertifiedGraph>` where `TrimLoop { chart: ChartId,
curve: BezierLeaf1, closed: bool }` (trim loops as certified curves in
lifted charts — the caller supplies them from the leaf/B-rep side; this
packet does not extract trims from topology). Steps 3-6 of section 9.4:
per arc's pcurve vs each trim curve IN THE SAME CHART, certified R9
crossings (`R9System` + `krawczyk_c1`, the S1A seam) = TopoNode
TrimCrossing nodes (identified by Rule A); split arcs at crossings;
classify sub-arcs by the winding number of the closed trim loop about one
certified-off interior sample (the sample's off-loop property certified
by R9 distance-positivity data from the crossing certificates); discard
outside; failure to isolate at DEPTH_MAX -> `Refused(TrimClipFailed)`
(Inconclusive). The `interior_loop` test: the spec's named no-special-case
case (loop crossing a trim but missing every leaf boundary). Winding
computation: exact integer crossings on the certified Bernstein
representation (ray-crossing count in the plane with certified
sign discipline) — polynomial arithmetic only.

QUEUE RULE + H-1 + H-3 same-line + fmt/clippy exact-verify-form clean +
workspace check green + CARGO_BUILD_JOBS=2-4 + COMMIT BEFORE RESULT.json
AT THE WORKTREE ROOT (standing house rules for ALL Wave-4 packets;
repeated here once — every packet in this wave carries them).

Stop conditions: 1. frozen seam differs — stop, record. 2. winding needs
a non-poly evaluation (transcendental) — stop, the loop representation is
wrong for N4. 3. an isolation genuinely fails at DEPTH_MAX — the named
refusal IS the deliverable; stop only if the FIXTURE was supposed to
isolate (record the numbers).

Commit subject: `feat(certified): trim clipping via R9 + winding
(BG-KV2-401-S3C)`.
