# BG-KV2-207B-TRACER-REST — audit of the escalation-lattice terrain (2026-09-07, orchestrator)

Basis: BG-KV2-207-S4A landed 5/8 (d6282a9, "5/8 green, rank-screen
refinements"); BG-KV2-307-ENGINEREACH landed partial (55607f2) — its
remainder needed tracer.rs, outside its write_allow. The three red tests
were NEVER green. Working tree is clean for both files; current behavior ==
as-landed behavior.

## Constants and bucket logic (tracer.rs)

- RANK_SUBBOXES = 16 (line 83; was 8 before d6282a9)
- SCREEN_PERP = 0.05 (line 90, `// H-3`)
- ISOLATED_CAP = RANK_SUBBOXES / 4 = 4 (line 95; was literal 2)
- PERP_RATIO = 3.0 (line 74), CLOSE_TOL (line 80)
- TracePolicy::default (115-128): arc_step0 0.05, grow 2.0, shrink 0.5,
  max_halvings 3, max_frame_rebuilds 2, max_steps 4000
- escalate() 739-868: rung-2 probe 771-787 (value_enclosure per component;
  no certified zero -> parametric_degeneracy_chart_or_carrier/Conditioning);
  rank screen 789-831 (hs = SCREEN_PERP*dtau at 792; screen_iv = y_cur +- hs
  793-806; per sub-box tube_chart_box 590-620 + minor_enclosures 821-830;
  None also counts as collapse); buckets 833-867: 0 -> Rebuild, ALL ->
  TangentialCurve/r2_contact_zero_set_one_dimensional, <= ISOLATED_CAP ->
  isolated_contact_is_s5a/Conditioning, else -> HighOrderJet/
  rank_screen_not_isolated_not_curve.
- Escalation runs after 3 halvings: dtau = 0.05*0.5^3 = 0.00625,
  tau_local = 0, y_cur = [0,0,0] (no successful steps; recorded
  steps: 0, halvings: 3).
- minor_enclosures 559-580: four Theorem-6.4 maximal minors via
  partial_enclosure 527-540 + det3_iv 544-554. contains_zero 583-585.
- Verified algebraic identity (matches kernel_minors engine.rs:730-745):
  for F = (u-s, v-t, g(u,v)) the four minors are exactly
  d = (gv, -gu, gv, gu), so all-four-contain-zero <=> enclosure(gu) ∋ 0 AND
  enclosure(gv) ∋ 0. Interval type = CertifiedInterval: ulp-level widening
  only — for the degree<=1 partials below the enclosures are EXACT ranges.
- Seed frame build_frame4 (engine.rs:775-934): q_tau = normalize((gv, -gu,
  gv, gu)) at the seed; for all three node/parabola fixtures gv = 0 exactly,
  giving q_tau = (0, 1/sqrt2, 0, 1/sqrt2), q_perp = [e_u, (0,1/sqrt2,0,
  -1/sqrt2), e_s].

## Mechanism per test

### dtau_grows_on_success_and_halves_on_failure (net_parabola(16.0), tests 226-248)

g = v - 16(u - 1/sqrt2)^2. gv = 1 everywhere => NO rank collapse anywhere;
the ladder can only Rebuild -> tracer_frame_rebuilds_exhausted
(Conditioning). The frozen seam (c2_certify_tube4) certifies no arc of the
k=16 quadratic branch at any width down to the 1e-6 probe floor (recorded:
tube_reach_envelope_measured.parabola_k16 tau_reach = 0.0, BG-KV2-307
RESULT:33). All four attempts fail; the run exits Completed with 0 certified
steps (the ArcAttempt::OutOfChart route, tracer.rs:959-962); the first
failing assert is line 237 (steps.len() >= 3). What the test wants is a
SEAM CAPABILITY (certifying curved arcs) or an honest terminal class — not
a screen bug. BG-KV2-307 RESULT:19-23, 43-50 named this gap.

### escalation_routes_isolated_r2_to_the_contact_future (net_nodes(0.3,[vb]), tests 283-300)

g = (u - ua)(v - vb); one isolated node at (ua, vb).
floor = 0.05*0.5^3 = 0.00625 = the halving floor; vb = v0 + (floor/2)/sqrt2
puts the node at tau = dtau_esc/2 = 8/16 of the failed arc — exactly ON the
sub-box 7/8 boundary (tests 291-292).
Screen geometry at escalation: dtau = 0.00625, hs = 3.125e-4, sub-box tau
width 3.906e-4. With the verified frame, per sub-box: v-advance =
q_tau[1]*dtau/16 = 2.762e-4; v-smear = 2*hs*sum_c|q_perp[c][1]| = 4.419e-4;
u-smear = 6.25e-4 centered at ua => enclosure(gv) = [-3.125e-4, +3.125e-4]
contains 0 in ALL 16 sub-boxes (screen centered on the branch). So collapse
<=> enclosure(gu = v - vb) straddles vb (exact for degree 1): k=6 misses by
5.6e-5, k=7 and k=8 straddle, k=9 misses by 5.5e-4.
STATIC PREDICTION: collapse = 2 <= 4 -> isolated_contact_is_s5a — the test
SHOULD pass. Observed: rank_screen_not_isolated_not_curve. A genuine
model/reality discrepancy of 2-3 sub-boxes.
Sensitivity: straddle count ~= 1 + 2*SCREEN_PERP*RANK_SUBBOXES*Sigma_v/b
(scale-invariant in dtau); observed >=5 requires Sigma_v/b >= 2.5 (ideal
frame gives 1.0; orthonormal worst case at b = 0.707 gives 3.77 -> up to
4). Candidate explanations to MEASURE, not assume: (1) live q_perp/q_tau
orientation differs from the axis-aligned GS prediction; (2) extra absolute
widening of the gu enclosure >= 5.6e-5 (ulp rounding cannot supply this —
check the to_unit_box clamping 491-511 and hull.div(&width) 527-540);
(3) the escalation arc is not the first arc; (4) node placement at the
exact 8/16 sub-box boundary maximizes the footprint.
STRUCTURAL DEFECT: the gv leg identically straddles, so "all four minors"
silently degenerates to the gu leg alone; the isolated/HighOrderJet
boundary has no designed margin.

### high_order_singularity_refuses (net_nodes(0.5, 3 nodes), tests 302-326)

g = (u - 0.5)*P(v), roots at v-offsets f*floor/sqrt2, f in {0.15, 0.5, 0.85}
-> sub-boxes ~{2,3}, {7,8}, {13,14}. gu = P(v): degree 3, between-root dips
~1.8e-9; the single-pass interval de Casteljau hull (hull_component_unit
391-444, no subdivision) over v-width ~7.2e-4 has absolute width
O(width * coefficient spread) ~= 7e-5 — five orders of magnitude above the
dips => every sub-box between/around the roots reads gu ∋ 0. gv contains 0
in every sub-box unconditionally. collapse = 16 -> TangentialCurve instead
of HighOrderJet. SCREEN RESOLUTION, not constants; the fixture is
geometrically correct.

## Constraints on edits

- H-1 (19-21): no unwrap/expect/panic, no module-level allow; crate denies
  clippy::unwrap_used.
- H-3 markers stay on defining lines: PERP_RATIO 74, CLOSE_TOL 80,
  SCREEN_PERP 90.
- D4 doctrine (23-29) + ladder contract (46-59): rung decisions from
  certified enclosures only. The doc drift "at most two sub-boxes"
  (57-58) is stale since d6282a9 raised the cap to 4 — fix with any cap
  change.
- Frozen seam (37-40): never construct ArcCert; never touch
  c2_certify_tube4/build_frame4 behavior. Mechanically enforced by
  tracer_output_never_claims_certification (tests 345-365): exactly one
  c2_certify_tube4( call site; the literals certified: Option<ArcCert<4>>
  and certified: Some( must remain in tracer.rs.
- TracePolicy::default values are load-bearing for the five passing tests.
- vendor/truck/** edits ride the packet loop only (AGENTS.md).

## Principled fix options (audit's preference order)

A. Screen resolution: certified bisection inside the screen (subdivide
   until hull width < designed fraction of the box's geometric scale;
   collapse only if every child still has all four minors containing zero).
   OR a common-zero test (gu and gv vanish SIMULTANEOUSLY — removes the gv
   degeneracy). OR an enclosure-width scale bound with an explicit third
   bucket refusing Budget/Residual.
B. Designed margin: derive SCREEN_PERP from ISOLATED_CAP
   (SCREEN_PERP <= (ISOLATED_CAP-1)/(2*RANK_SUBBOXES*sqrt(3)), e.g. 0.02
   -> bound 2.1) or derive the cap from the smear bound; fix the stale doc.
C. Parabola: add the rebuild-exhausted-with-zero-collapse -> Budget/Residual
   terminal refusal in trace_march (984-1016) — the honest class 307 named.
   NOTE: after A, three separated nodes may yield 6-9 collapses — still the
   middle bucket; rung-5 semantics may need to key on footprint STRUCTURE
   (multiplicity of separated collapse clusters) rather than raw count.
   The packet must decide this explicitly.
D. Diagnostic first: instrumented float_trace_impl run dumping live frame,
   tau_local/dtau at escalate, per-sub-box widths, collapse count — BEFORE
   committing the redesign.
