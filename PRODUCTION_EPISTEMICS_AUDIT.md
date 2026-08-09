# PRODUCTION EPISTEMICS AUDIT — P1 / P2 / P3a / P3b

**Date:** 2026-08-08
**Audit scope:** truck-fork `d7bb5166` (= P1 `17ac0f15` + P2 `f9e06c64` + P3b `d7bb5166`, on top of FACE-VALIDITY `018bd469`), as consumed by `look` (Cargo pins at `d7bb5166`, `.cargo/config.toml` override re-commented).
**Method:** static derivation from source. No production code modified. One corpus read (saved `look-corpus` NIST/ABC sweep artifacts) used only to confirm/falsify suspected overclaims after the code-derived epistemic status.
**Corpus evidence used:** `look-corpus\nist-rec\p3b` (NIST), `look-corpus\nist-rec\abc` (ABC-20 after P3b), `look-corpus\planar-c\abc` (baseline).

---

## A. Executive conclusion

**P1 is formally faithful as a mechanism, with a scope-of-use caveat and one unresolved cross-corpus risk.**
`evaluation_range()` certifies exactly the interior knot span on which the basis is a partition of unity. Its use at the boundary-polyline and presearch sites requires only evaluability, so the mechanism is sound where it fires. It does **not** certify `D_eval = D_source_edge`, and the packet's own P1 limitation (§3, critical limitation) is respected in the code: no production line claims that equality. The unresolved risk is that switching boundary sampling from `range_tuple()` to `evaluation_range()` changes *which portion of a legitimate source curve is sampled* for curves whose source closure/trim lives outside the interior knot span; the ABC-20 rendered→lost cluster (193 plane + 71 cylinder faces) is currently *unattributable* to P1 without an intermediate-pin sweep, and the plane/cylinder families have no other P1/P2/P3b mechanism.

**P2 is partially faithful and contains one confirmed epistemic overclaim.**
The `Continue` branch is a constructive chart-lift (branch fixed by the leaving edge's plane, never by the pole's undefined longitude) and it correctly discards synthetic bisection midpoints. But (i) the theorem's "certified period `T`" is not established in production — the sphere targets carry only a *declared*, uncertified period (`deck_status=Unavailable` on the recovered faces), and (ii) the `CertifiedAmbiguous → RejectedAmbiguous` branch asserts a *source-level* ambiguity certificate from the negative evidence "the leaving edge's own first sample is also a pole / no leaving point found". Per the P2 ambiguity theorem that inference is invalid: it at most proves *this mechanism cannot determine a continuation*. **Confirmed on the ABC-20 corpus: 7 sphere faces carry `terminal_reason=RejectedAmbiguous`** (00000959 ×4, 00001075 ×2, 00005760 ×1), all with `bound_count=1`, `deck_status=Unavailable`, previously classified `NoSurfaceProduced` (an unresolved loss). These are classified as certified-ambiguous without positive evidence of two source-consistent continuations.

**P3a is respected.** No production code promotes chart-planar degeneracy into physical/source degeneracy or rejection. `NoOddParityRegion` remains a loss bucket, not a rejection; the FACE-VALIDITY certificate is world-rank based, not UV-area based. The negative invariant holds.

**P3b is partially faithful: the construction is correct, the activation is heuristic, and one threshold is mislabeled "certified".**
The chart-closure cell (`build_cap_cell`) is provably equivalent to the theorem's rectangle: seams attach to the actual cut-path endpoints (`B − A = kT` is carried by the loop's own normalized last point), source segments keep provenance, closure segments are `ChartClosure`/`UnresolvedSyntheticClosure` with no forged identity, and the closure toggles exactly as the completion of the single material boundary. The material-side composition `n × t` is constructively consistent (internally validated; relies on `surface.invert()` for `s_f`, which the project's own `source_evidence` layer marks *suspect*). The activation hypotheses H1 (genuine period) and H4 (true collapsed orbit) are **heuristic**: the period is the declared/uncertified accessor value, and the "certified collapse" is a two-sample orbit-diameter scan under a `1e-4 × r_loop` relative threshold — a numerical predicate the comments and handoff call "certified".

**Bottom line:** do not reduce this to render counts. NIST 7,901/7,902 is real and mechanism-correct; the ABC net +67 hides a 308-face rendered→lost population whose causation is *not* established for P1/P2/P3b, and a 7-face population (RejectedAmbiguous) that is an epistemic overclaim.

---

## B. Production proof map

Every row: evidence → proposition → justification → classification.

### P1 rows

| # | Evidence | Proposition | Justification | Class |
|---|---|---|---|---|
| B1 | Cox–de Boor recursion; knot span `[knot[p], knot[n−1−p]]` | Basis is a partition of unity on the interior knot span | Interior-knot property of a degree-`p` B-spline (`bspcurve.rs:410-422`, `surface_knot_domain` `bspsurface.rs:1697`) | CERTIFIED |
| B2 | Evaluating outside the support yields all-zero basis → `subs` = origin | `u ∉ D_eval` ⇒ "do not treat as an ordinary represented point" | NIST-verified (`triangulation.rs` P1 test at 8857); the packet's P1 theorem | CERTIFIED |
| B3 | Boundary polyline built over `evaluation_range()` (`triangulation.rs:1496`, 919, 977) | The sampled polyline represents the drawn curve | Boundary sampling requires only evaluability; every sampled point is an ordinary represented point | CERTIFIED |
| B4 | Hintless presearch over `evaluation_range()` (`bspsurface.rs:1675,1809`, `nurbssurface.rs:569,853`; `IncludeCurve` 1819/1854/1889) | Search domain covers the inverse's basin for genuine on-surface points | Any genuine surface point's parameter lies in the evaluable rectangle; sliver evaluates to garbage so it cannot contain a valid inverse | CONSTRUCTIVE |
| B5 | `PolicyCurve::evaluation_range` forwards (`policy_geometry.rs:159-164`) | The production curve type samples over the evaluable domain | Direct forwarding of the trait method | CERTIFIED |
| B6 | `probe_nearest`/`admit_outcome` `in_domain` read `try_range_tuple` (`triangulation.rs:283-286, 427-429`) | "In declared range" is the admission predicate | This is a *declared-domain* admission; the sliver is guarded by the re-evaluation residual (`admit_outcome` 433-441). Slightly loose but not semantically overclaiming | CONSTRUCTIVE |
| B7 | `find_cap_pole` non-periodic scan range from `try_range_tuple` (`triangulation.rs:6067-6070`) | The pole-search band is the non-periodic axis extent | Uses the *raw* declared range, not `evaluation_range()` | HEURISTIC (latent P1/P3b clash, §C7) |
| B8 | `domain/projection.rs` `TraversalSemantics::resolve` uses `range_tuple` (`projection.rs:61,199,242`) | Curve traversal semantics over the declared range | Test-only module; not on the production path | NOT PRODUCTION |

### P2 rows

| # | Evidence | Proposition | Justification | Class |
|---|---|---|---|---|
| B9 | Exhausted ambiguous step at a sample whose periodic-axis partial is `so_small` (`triangulation.rs:5143-5151`) | The step departs from / enters a chart singularity in the periodic direction | `so_small()` is an absolute `1e-6` test on the partial derivative; at an exact sphere pole it is 0, but a merely-small derivative also passes | HEURISTIC (candidate gate) |
| B10 | Great circle through a pole has constant longitude = plane azimuth | The leaving edge's plane fixes the outgoing longitude | Analytic property of great circles; true for the sphere-pole configuration | CONSTRUCTIVE |
| B11 | `get_mindiff(outgoing, incoming, declared_period)` (`triangulation.rs:5162-5165, 4990-4996`) | The branch is the nearest periodic copy of the outgoing longitude | Nearest-copy representative is the correct chart-lift choice **provided** `T` is a genuine period; the production period is *declared*, not certified (lattice `Uncertified` for spheres) | CONSTRUCTIVE **with uncertified H** |
| B12 | Resume from the leaving edge's first *real* sample, discarding synthetic midpoints (`triangulation.rs:4561-4562`) | Synthetic bisection points are not source evidence | Strong P2 rule; correct | CERTIFIED |
| B13 | Leaving edge's first sample also a pole / no leaving point found (`triangulation.rs:5172-5177, 5198-5209`) | "Source-level singular ambiguity" ⇒ `RejectedAmbiguous` | Negative search evidence does **not** prove multiple source-consistent continuations | **INVALID** (overclaim; confirmed §D4) |
| B14 | `RejectedAmbiguous` is terminal; recovery routes blocked (`triangulation.rs:1778-1785`) | A certified rejection is not touchable by recovery | Sound *if* the certificate were valid; invalid when the certificate is overclaimed | INVALID downstream |

### P3a rows

| # | Evidence | Proposition | Justification | Class |
|---|---|---|---|---|
| B15 | `NoOddParityRegion` from empty parity flood (`triangulation.rs:8548-8550`) | No material cell in the *chart*; nothing about source | Chart-level outcome; census classifies as unresolved loss, not rejection | CERTIFIED (as a chart statement) |
| B16 | `validity.rs` Detector B rank certificate (world-space rank < 2) | The *represented trim* is degenerate | Rank/conditioning, not UV area; "small or thin is valid" | CERTIFIED (off by default) |
| B17 | `DEGENERATE_LOOP_AREA` two-loop join gate (`triangulation.rs:6373`) | Two degenerate chart loops join into a band | Chart construction; no source claim | CONSTRUCTIVE |

### P3b rows

| # | Evidence | Proposition | Justification | Class |
|---|---|---|---|---|
| B18 | Displacement `|k|=1` in one declared-period axis (`triangulation.rs:5947-5955`) | The loop winds once around the periodic axis | `periodic_displacement` uses the *declared* period (uncertified for spheres) + UV tolerance `1e-3` | HEURISTIC (H2 uncertified period) |
| B19 | `signed_area < 1e-4` and non-periodic span `< 0.1·period` (`triangulation.rs:5958, 5975`) | The loop's chart image is a 1D line (cap-like) | Numerical gates; candidate-only | HEURISTIC |
| B20 | `n × t` material side (`triangulation.rs:6042-6075`) | `n×t` points into the source material | `n` = effective normal (carries `s_f` via `surface.invert()`), `t` = walk tangent (carries `s_b·s_o·s_e·s_c`); internally consistent and test-validated; relies on the invert mechanism `source_evidence` marks suspect | CONSTRUCTIVE (not independently certified) |
| B21 | Orbit-diameter scan + `1e-4·r_loop` threshold (`triangulation.rs:6085-6138`) | A true collapsed orbit exists on the material side | Two samples half a period apart; `q(r,0)≈q(r,T/2)` does **not** prove `q(r,θ)=const`; threshold is heuristic | HEURISTIC (mislabeled "certified" in comments/handoff) |
| B22 | Cell corners at `(r_pole, p1)`/`(r_pole, p0)` with `p1 = p0 + kT` (`triangulation.rs:6156-6175, 6307/6314`) | Seams attach to the actual cut-path endpoints | The loop's normalized last point carries `p0 + kT`; the seam/pole-line geometry is provably the theorem's rectangle | CONSTRUCTIVE (verified, §F5) |
| B23 | Closure toggles as `UnresolvedSyntheticClosure` (`triangulation.rs:5331-5338, 7375`) | Chart closure completes the single material boundary, never a second region | The source walk is an open chart path; one closed toggling loop selects one interior | CONSTRUCTIVE (per §6 semantics) |

---

## C. P1 findings

### C1. What `evaluation_range()` mathematically certifies

For a degree-`p` B-spline/NURBS curve, `[knot[p], knot[n−1−p]]` (`bspcurve.rs:410-422`); for a surface, the tensor product rectangle (`surface_knot_domain`, `bspsurface.rs:1697-1715`, `nurbssurface.rs:684-692`). On that domain the Cox–de Boor basis is a partition of unity, so `subs` returns ordinary represented points and the metric is the drawn geometry's. It does **not** certify:
- that the source trim / edge-use parameter interval equals this span;
- that the source intended any particular sampling of it;
- that a declared range outside it is "unrepresentable" in any semantic sense beyond "evaluation there yields the zero basis".

The trait default (`curve.rs:58`, `surface.rs:221`) is `range_tuple()`, so every non-spline family is untouched. This matches the packet's "critical limitation".

### C2. Every important production use

| Site | Domain the caller needs | What `evaluation_range()` gives | Equality proved? | Verdict |
|---|---|---|---|---|
| `tessellate_edge` `PolylineCurve::from_curve(curve, evaluation_range(), tol)` `triangulation.rs:1496` (also 919, 977) | evaluability of every sample | exactly that | not required | CERTIFIED use |
| `BSplineSurface/NurbsSurface::search_parameter`/`search_nearest_parameter` presearch `bspsurface.rs:1675,1809`; `nurbssurface.rs:569,853` | a domain covering the inverse's basin for on-surface points | the image-domain of genuine points | on-surface points live in the evaluable rectangle; the sliver evaluates to garbage and cannot contain a valid inverse | CONSTRUCTIVE |
| `IncludeCurve` helpers `bspsurface.rs:1819,1854,1889` | same as above | same | same | CONSTRUCTIVE |
| `PolicyCurve::evaluation_range` `policy_geometry.rs:159-164` | forwarding only | — | — | CERTIFIED |

### C3. Is a source edge-use/trim interval available but ignored?

Yes, structurally: the edge's trim interval is the edge-use parameter interval recovered by the converter (`truck-stepio` `EdgeCurveHolder::sub_parse_curve3d`), but `tessellate_edge` ignores it and always samples `evaluation_range()`. For spline edges the converter's trim was recovered from the *vertex solves on the converted curve*, which for unclamped closed splines already lands inside the interior span, so in practice the two agree on the NIST corpus. The code does **not** prove `D_eval = D_source_edge`; it assumes it for spline edges.

### C4. Can replacing `range_tuple()` by `evaluation_range()` alter which portion of a legitimate source curve is sampled?

Yes — this is the packet's question 5, and it is the only plausible P1 loss mechanism. For a curve whose source closure/trim was written in the sliver (the exporter's periodic-closure convention not matching the interior-knot assumption), the corrected sampling shortens the drawn loop. Whether any ABC faces hit this is **unresolved** (§G2, falsification plan). The NIST evidence (P1 recovered 5, regressed 0) supports the assumption on NIST but does not prove it on ABC.

### C5. Can a search domain be validly restricted to `evaluation_range()` while preserving completeness of the inverse problem?

Yes for genuine on-surface points (C2), and the admit gate re-certifies by re-evaluation (`admit_outcome` `triangulation.rs:433-441`). The domain-classification and stage-C recovery read the *declared* range (`try_range_tuple`) for classification (`classify_domain_point` `triangulation.rs:547`, `residual_certified_domain_recovery` 627), which is a different (declared-domain) question; their candidates are re-certified by the same residual gate, so no semantic overclaim.

### C6. Are P1 semantics accidentally propagated into unrelated operations?

Two latent sites read the *declared* range where the *evaluable* range would be the semantically relevant one:
- `probe_nearest`/`admit_outcome` `in_domain` (`triangulation.rs:283-286, 427-429`) — a candidate in the sliver passes the domain test but is caught by the residual; diagnostic classification only, no production overclaim.
- `find_cap_pole` non-periodic scan range (`triangulation.rs:6067-6070`) — the P3b pole scan uses the raw declared range; on a periodic spline/swept surface the scan could traverse a non-evaluable sliver (see C7).

No production site uses `evaluation_range()` where a *source* semantics is claimed.

### C7. Cross-packet range inconsistency (P1/P3b)

`find_cap_pole` obtains its non-periodic scan band from `surface.try_range_tuple()` rather than `evaluation_range()`. Through `look::lattice_of`, splines are `NON_PERIODIC` so P3b cannot activate on them today; however `SweptCurve`/`OffsetSurface` are carried as `Unevidenced` and can declare a period from accessors, and their `try_range_tuple()` can be spline-backed with unclamped ends. On such a surface P3b's pole search would scan the non-evaluable sliver — exactly the P1 defect class. Status: latent (HEURISTIC), reachable only via a periodic swept/offset surface, which no corpus case is known to hit. It is a genuine code-level inconsistency between the P1 rule ("use the evaluable domain") and the P3b pole search.

---

## D. P2 findings

### D1. Singular activation is certified or heuristic?

**Heuristic (candidate gate).** `collapsed_axis` (`triangulation.rs:5143-5151`) uses `surface.vder/uder(u,v).so_small()`, and `so_small` is an absolute `1e-6` magnitude test (`tolerance.rs:6, 134-143`). It is:
1. **not scale-aware** (a unitless `1e-6` on a derivative with physical units; a radius-1000 sphere needs the sample within ~1e-9 of the pole to trigger);
2. **not a certified quotient collapse** — a merely-small derivative at a near-pole regular point passes the test;
3. gated only by bisection exhaustion, whose equivalence to "rank-deficient transition" is an empirical claim (comment `triangulation.rs:4464-4472`), not a theorem — a general periodic surface whose chord midpoints project off the surface could exhaust without being singular.
The doc comment on `reconcile_singular_transition` (`triangulation.rs:4040-4043`) correctly says "a small derivative only proposes a chart substitution"; the P2 branch's `collapsed_axis` has no such caveat and is used as the singularity classifier.

### D2. Does the `Continue` branch satisfy the source/chart theorem?

Conditionally yes:
- outgoing longitude from a real non-singular sample of the leaving edge ✓ (`origin`, `leaving_pt`);
- incoming longitude from the last accepted non-collapsed sample ✓ (`incoming_longitude`);
- branch = nearest period copy ✓ (`get_mindiff`);
- pole gets the branch as bookkeeping ✓ (`pole_uv`);
- resume from a real sample, synthetic midpoints discarded ✓.
The missing hypothesis is **certified `T`**: the periods come from `lattice.declared_u_period()/declared_v_period()` (`triangulation.rs:4136`), and for sphere targets `look::lattice_of` returns `Uncertified` (the ABC RejectedAmbiguous faces report `deck_status=Unavailable`, `periodic_axes {u:false, v:true}`). The continuation is still *correct* for spheres because 2π is genuinely the longitude period, but the mechanism does not establish it.

### D3. Is synthetic midpoint evidence correctly discarded?

Yes. On `Continue`, `pending.clear()` then `pending.push((resume_point, false, resume_tag))` (`triangulation.rs:4561-4562`) — the walk resumes from the real sample and all invented bisection midpoints are dropped. This matches the strong P2 rule.

### D4. Is `CertifiedAmbiguous` actually source-certified?

**No — this is the principal P2 finding.** The three `CertifiedAmbiguous` returns (`triangulation.rs:5176`, 5199, 5208) all fire on negative search evidence:
- Case A: `collapsed_axis(origin)` — the leaving edge's own first sample is also a pole;
- Case B: no leaving point found ahead on `bdry3d`, or the leaving point is also a pole.

None of these produces two inequivalent *source-consistent* continuations `A ≠ B mod ≡`. In the great-circle geometry that motivates the branch, the leaving edge's plane always fixes the outgoing longitude; if the first sampled point is itself a pole, the plane is merely not yet readable at that index — continuing along the source boundary is still the unique continuation. So `RejectedAmbiguous` here asserts "source-level singular ambiguity" (`triangulation.rs:7851-7861`) from evidence that at most proves "this mechanism cannot determine a continuation at this position". This is the forbidden promotion **insufficient evidence → source ambiguity** and **terminal failure reason → source validity certificate**.

**Confirmed on ABC-20:** 7 sphere faces with `terminal_reason=RejectedAmbiguous` (00000959: #28614 #16822 #27298 +1; 00001075: 2; 00005760: 1). All have `bound_count=1`, `chart_rank=1`, `periodic_axes {u:false, v:true}`, `deck_status=Unavailable`, `lift_status=Ambiguous`, `source_segment_count=0`, and — in the PLANAR-C baseline — `stage=tessellate reason=NoSurfaceProduced` (an *unresolved* loss). P2 promoted these from "unresolved" to "certified rejected-ambiguous". The census (`face_census.rs:497-505`) and diagnosis (`LossBucket::IntrinsicAmbiguous`, `diagnosis.rs:935`) faithfully report the overclaimed certificate.

### D5. Do rejection semantics exceed the evidence?

Yes, downstream: `RejectedAmbiguous` is terminal (`rejected_terminal`, `triangulation.rs:1778-1785`) — it blocks every recovery route and classifies the face as `rejected` rather than `failed_renderable_or_unknown`. Because the certificate is overclaimed, the blocking is also unearned. (In the observed 7 faces the routes were `PreconditionUnmet` anyway, so the practical render loss is nil; the classification harm is real.)

---

## E. P3a findings

Searched all production logic for `signed_area`, `DEGENERATE_LOOP_AREA`, `EuclideanClosed`, `PeriodicClosed`, `winding`, `NoOddParityRegion`, `ContradictoryDualParity`:

1. **No production code promotes chart degeneracy → source/material degeneracy.**
   - `NoOddParityRegion` is produced when the parity flood selects no cell (`triangulation.rs:8548-8550`) — a chart statement. The census maps it to `failed_renderable_or_unknown` (an unresolved loss), never `rejected`.
   - FACE-VALIDITY's rejection is world-rank based (`validity.rs:19-34`: Detector B rank certificate from coordinate conditioning), **never** UV-area based. "Small or thin is valid" is enforced by the module contract.
   - `InconsistentFaceReason::ContradictoryDualParity` is documented as "**Not currently emitted** — ... not yet certified against the source" (`validity.rs:86-90`). The parity contradiction is explicitly kept algorithmic.
2. `DEGENERATE_LOOP_AREA` (1e-4) is used at `triangulation.rs:5958` (P3b cap gate) and `6373` (two-loop join gate). Both are chart-construction decisions, not source claims. The two-loop join builds a band from two degenerate loops — a chart closure, correctly labeled with `Seam` origins.
3. The P2 handoff's P3a "verify-first" conclusion is consistent with the code: the nist_29 equator trace (`u=0 constant, v 2π→4π`, `signed_area=0.0`) is the equator in the inverted convention, a genuine 1D chart image of a valid positive-area face. No equator→pole mapping is fabricated, and no source invalidity is inferred from the collapsed chart.

**P3a verdict: the negative invariant is respected. No finding.**

---

## F. P3b findings

### F1. Certified lattice period (H1) — NOT certified

`try_build` reads `lattice.declared_u_period()/declared_v_period()` (`triangulation.rs:5953-5954`). Through `look::lattice_of`:
- sphere → `unevidenced_elementary` → `AxisPeriodStatus::Uncertified` (`lattice.rs:79, 104-107`); the 2π longitude period is *declared* but not a certified generator (`certified_rank() == 0`).
- cylinder/cone → `Exact` (2π on the revolution axis) — certified; but a cylinder has no collapse so the cap declines.
So the P3b theorem's H1 ("genuine period") is only heuristically established for the actual NIST targets (spheres). The value is correct for spheres, but the activation rests on an uncertified accessor.

### F2. Winding calculation (H2) — heuristic

`periodic_displacement` (`triangulation.rs:5272-5283`) computes `k = round((end−start)/period)` with residual tolerance `1e-3` (UV). The `|k|=1` gate (`triangulation.rs:5947-5950`) is a genuine lattice integer when the period is certified; for the sphere targets it uses the declared period. For a near-closing latitude loop the `1e-3` UV residual is robust, but the integer is only as certified as the period.

### F3. Cap-class activation (H3) — numerical gates, candidate-only

- `signed_area < 1e-4` (absolute UV area) — scale-dependent, but for a genuine latitude loop the image is a 1D line so this is 0; the gate correctly rejects 2D images.
- `n_max − n_min > 0.1 * period → reject` (`triangulation.rs:5975`) — compares the **non-periodic** axis span to the **periodic** period, two axes with no commensurate physical meaning. On a sphere, `0.1·2π ≈ 0.63` radians of latitude spread would pass the gate; the signed-area gate is what actually constrains the class. This is a candidate gate, **not** a proof the loop is a latitude/cap boundary. The doc/comment does not overclaim here — it says "classified as ... and built" — but the handoff's "latitude-parallel" framing for gated loops is stronger than the gates alone warrant. (The subsequent collapse + material-side requirements narrow it further.)

### F4. True orbit-collapse certificate (H4) — HEURISTIC, mislabeled "certified"

`orbit_diameter` (`triangulation.rs:6006-6023`) samples `q(r,0)` and `q(r,T/2)`; `find_cap_pole` accepts a collapse if `best_rr < 1e-4 * r_loop` after a coarse scan + golden-section (`triangulation.rs:6089-6138`). 
- `q(r,0) ≈ q(r,T/2)` does **not** prove `q(r,θ) = P ∀θ` for an arbitrary periodic surface;
- the `1e-4·r_loop` threshold is a relative heuristic with no error model;
- for the sphere it is exact (the orbit genuinely → 0 at the pole), so the NIST mechanism is sound;
- for a generic `PreMeshableSurface` the theorem does not hold; a non-collapsing surface whose orbit dips to 1e-4 relative would be accepted.
The handoff's "then **certified** to genuinely collapse (relative threshold `1e-4 ×` ...)" and the doc "Confirm the orbit genuinely collapses" both promote a numerical predicate to a certificate. This is the packet's §8.2 concern and is a genuine (naming/classification) overclaim, though benign on the corpus because the only activated surfaces are exact-collapse spheres.

### F5. Chart-closure endpoints (H6) — CORRECT, verified

`build_cap_cell` (`triangulation.rs:6146-6198`): `path = loop_.into_path_cutting_wrap()` (drop the wrap segment's origins, keep all points, `triangulation.rs:5383-5396`); `A = path.points.first()`, `B = path.points.last()`. Corners `c = (r_pole, p1)`, `d = (r_pole, p0)` where `p1 = periodic_comp(loop_.points.last())` and `p0 = periodic_comp(loop_.points[0])`. Because `PolyBoundary::new` normalizes a `PeriodicClosed` loop so `last.periodic = first.periodic + kT` (`triangulation.rs:6303-6316`), `p1 − p0 = kT` exactly. The two meridians (`B→c` and `d→A`) therefore lie on the *same physical meridian* traversed in opposite directions, and the pole line `c→d` collapses to the pole — precisely `q_*(∂R) = B`. The unused `_k` parameter is provably redundant: the endpoint displacement is carried by the normalized last point, so the construction is correct *by the invariant of `into_path_cutting_wrap` + the `PeriodicClosed` normalization*, not by assumption. **This high-priority audit point is clean.**

### F6. Material-side orientation composition (H5) — CONSTRUCTIVE, not independently certified

`n = surface.normal(pb.uv)` (`triangulation.rs:6057`) is the *effective* normal: the face's surface copy is inverted when `same_sense = .F.` (`convert.rs:470-472`), and `Processor::normal` negates and axis-swaps under inversion (`processor.rs:288-298`). `t` is the walk tangent of the lifted loop, which carries `s_b·s_o` (bound orientation folded into order + edge orientation, `convert.rs:244, 260-262`) plus `s_e·s_c` folded into the converted curve. So `n × t` = material-on-left viewed from the effective-normal side — the STEP convention. Internal consistency is validated by the 8 tests, including the inverted-surface case (`triangulation.rs:12825-12843`), and by the NIST render check (south cap for the `same_sense=.F.` #1954). 

Two caveats (both epistemic, neither falsified on corpus):
- `s_f` is applied via `surface.invert()`, which `source_evidence.rs:174-179` marks **suspect** ("breaks curve-on-surface incidence rather than only reversing the parameterization"); the cap's material side therefore rests on exactly the mechanism the project's own evidence layer distrusts.
- `face.orientation` (ORIENTED_FACE) is not part of the composition. This is *arguably* correct (bounds are defined relative to the face's own front side, so the shell-use flag does not change the material side), but the code never states or proves it; `source_evidence.rs` keeps the normalized sign `None` on the compressed path (`computable_normalized_sign_count() == 0`), so no independent certificate supports the material side.
Verdict: CONSTRUCTIVE (a built witness, internally validated), **not** CERTIFIED.

### F7. Validity of synthetic chart closure — CORRECT per §6 semantics

The closure segments are `SegmentOrigin::ChartClosure` with empty contributor sets (`triangulation.rs:6178-6195`); `role()` maps to `UnresolvedSyntheticClosure` (`triangulation.rs:5331-5338`), and `toggles_material` returns `Some(true)` for it (`triangulation.rs:7375`). The closure therefore toggles. Per the packet's §6 this is legitimate **provided** it is only the chart-boundary completion and never a second material region. Verified: the cell is one closed loop; the source walk is an open chart path (its periodic wrap replaced by the meridian seam); the flood selects the single interior. The NIST results (`nist_29 #1954` → 91 triangles, `nist_6 #353` → 157 triangles) are consistent with a single hemisphere/small cap. **No forged source identity and no second material region.**

### F8. P1/P3b compatibility — one latent clash (see C7)

The pole search's non-periodic band is `try_range_tuple()` (`triangulation.rs:6067-6070`), not `evaluation_range()`. Not reachable for the sphere/cone/cylinder targets (fully evaluable ranges); reachable in principle for a periodic spline-backed swept/offset surface. This is the one place where P1 semantics and P3b semantics disagree in the code as written.

---

## G. Epistemic violations

### G1. P2 `CertifiedAmbiguous` → source-level ambiguity certificate

```
CODE:
    triangulation.rs:5172-5177 (Case A), 5198-5199 and 5207-5209 (Case B);
    reason doc triangulation.rs:7851-7861; terminal gating triangulation.rs:1778-1785;
    census face_census.rs:497-505.

EVIDENCE AVAILABLE:
    The exhausted ambiguous step's start is a chart singularity (so_small periodic
    partial). The leaving edge's first sampled point is also a pole, or no leaving
    point was found at the expected index. No second candidate continuation is
    ever constructed or shown to be source-consistent.

PRODUCTION CLAIM:
    "The oriented incident source geometry does not determine a unique
    continuation, so the lift branch is a *source-level* ambiguity" — a certified
    `RejectedAmbiguous`.

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    Does not. "No usable non-singular outgoing sample at the expected index" at
    most proves this mechanism cannot determine a continuation at that position.
    A source-level ambiguity certificate requires positive evidence of two
    inequivalent source-consistent continuations (∃A≠B, both source-consistent,
    indistinguishable by source evidence). The great-circle geometry that motivates
    the branch always fixes the outgoing longitude by the leaving plane; if the
    first sample is itself a pole, the plane is simply not yet readable at that
    index, and the source continuation along the leaving edge is unique.

CORRECT EPISTEMIC STATUS:
    INVALID (overclaim). The branch should return the ordinary `AmbiguousLift`
    (unresolved) unless it constructs a genuine second continuation.

POTENTIAL FAILURE CLASS:
    Terminal-failure-reason → source-validity-certificate promotion;
    insufficient-evidence → source-ambiguity promotion.

EXPECTED ABC SIGNATURE:
    Sphere faces, single bound, pole-meeting great-circle edges, `deck_status`
    Unavailable, previously `AmbiguousLift`/`NoSurfaceProduced`, now
    `RejectedAmbiguous`. CONFIRMED: 7 faces (00000959 ×4, 00001075 ×2, 00005760 ×1).
```

### G2. P1 sampling change may alter the sampled portion of a legitimate source curve

```
CODE:
    triangulation.rs:1496 (and 919, 977): boundary polyline over evaluation_range().

EVIDENCE AVAILABLE:
    evaluation_range() certifies only evaluability on the interior knot span.
    No production line proves D_eval = D_source_edge for spline boundary edges.

PRODUCTION CLAIM:
    Sampling the boundary over evaluation_range() represents "exactly the drawn
    curve and keeps the closure".

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    Implies it for curves whose source closure lives inside the interior span
    (the NIST structure). Does not for curves whose closure/trim was written in
    the sliver region; for those, the corrected sampling shortens the loop and can
    change the realized material region or produce a degenerate 2-point bound.

CORRECT EPISTEMIC STATUS:
    UNRESOLVED on ABC. P1 is CERTIFIED as a mechanism; the claim that every source
    spline edge's drawn closure is inside the interior span is an assumption that
    the ABC rendered→lost cluster (193 plane + 71 cylinder + 15 cone, dominated by
    00007705) does not yet exclude.

POTENTIAL FAILURE CLASS:
    D_eval promoted to D_source_edge without proof; "drawn curve" overclaim for
    sliver-closure curves.

EXPECTED ABC SIGNATURE:
    Plane/cylinder faces whose boundary curves are splines with unclamped ends,
    currently meshing to nothing with a degenerate 2-point boundary
    (NoOddParityRegion). OBSERVED: representative JSONL rows show exactly a 2-point,
    zero-area, 0-constraint boundary for the regressed plane faces — but the
    surface families have no other P1/P2/P3b mechanism, so an intermediate-pin
    sweep (018bd469 → 17ac0f15) is required to attribute (falsification plan §I).
```

### G3. P3b "certified" orbit-collapse threshold

```
CODE:
    triangulation.rs:6089 (threshold = 1e-4 * r_loop), 6102/6137-6138 (acceptance);
    handoff P3 §3 "then certified to genuinely collapse".

EVIDENCE AVAILABLE:
    orbit_diameter samples only q(r,0) and q(r,T/2). A small value does not prove
    q(r,θ) = const for all θ; no error model; exact for spheres, heuristic for
    generic PreMeshableSurface.

PRODUCTION CLAIM:
    "the orbit ... is certified to genuinely collapse" and the pole is used as an
    exact collapse point in build_cap_cell.

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    Does not imply "certified" for the generic domain the function accepts. It
    implies "candidate collapse found by two-sample scan under a relative
    threshold". For the activated corpus (spheres) the collapse is exact, so the
    constructed cell is correct; the classification "certified" is not.

CORRECT EPISTEMIC STATUS:
    HEURISTIC (activation), with a naming overclaim ("certified").

POTENTIAL FAILURE CLASS:
    small-numerical-quantity → exact-source-degeneracy promotion; fabricated cap
    on a near-collapse non-spherical surface would render silently (wrong
    geometry, not a loss).

EXPECTED ABC SIGNATURE:
    Wrong-geometry caps on surfaces whose orbit dips below 1e-4·r_loop without
    collapsing. Would appear as *newly rendered* faces (invisible to the loss
    ledger); requires a geometry-side census to detect. No corpus case currently
    known.
```

### G4. Declared (uncertified) period used as the theorem's "certified T"

```
CODE:
    triangulation.rs:4136 (lift periods from declared_*), 5953-5954 (cap period),
    5272-5283 (winding). look lattice.rs:79, 104-107 (sphere → Uncertified).

EVIDENCE AVAILABLE:
    CertifiedLattice::generator() is None for spheres; declared_period() returns
    the accessor value. deck_status=Unavailable on the 7 ABC RejectedAmbiguous faces.

PRODUCTION CLAIM:
    P2/P3b apply theorems whose hypothesis is "certified period T" using the
    declared value.

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    Does not. The value is correct for spheres, but the code never certifies it;
    the mechanisms work by luck of the exact sphere primitive, and would not be
    valid for a generic surface with an accessor-declared period.

CORRECT EPISTEMIC STATUS:
    HEURISTIC (activation), CONSTRUCTIVE in the sense that the sphere's period is
    exact by the representation the wrapper hides.

POTENTIAL FAILURE CLASS:
    insufficient-evidence → exact-period promotion; wrong branch/closure on a
    non-certified-period surface.

EXPECTED ABC SIGNATURE:
    None directly observable (sphere values are correct); a latent fault line if a
    future lattice adapter declares a period for a spline/swept surface.
```

### G5. P2 `so_small` singularity activation is not scale-aware / not a certificate

```
CODE:
    triangulation.rs:5143-5151; tolerance.rs:6, 134-143.

EVIDENCE AVAILABLE:
    Absolute 1e-6 magnitude test on the periodic partial derivative; no rank or
    analytic certificate; gate is "bisection exhaustion", an empirical proxy.

PRODUCTION CLAIM:
    "The pole is a chart point where the *periodic* axis's partial collapses" —
    used to select singular-transition recovery.

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    A small derivative is necessary at an exact pole but not sufficient; the
    activation is a candidate gate, not a certified quotient collapse. The recovery
    still only fires after bisection exhaustion, so false positives would need a
    regular surface whose chord midpoints stay off-surface — not established to be
    impossible.

CORRECT EPISTEMIC STATUS:
    HEURISTIC (activation).

POTENTIAL FAILURE CLASS:
    small-derivative → certified-singularity promotion; wrong branch on a
    regular-but-slow periodic surface.

EXPECTED ABC SIGNATURE:
    Sphere/revolution faces near a pole incorrectly routed into singular recovery.
    No corpus case currently observed beyond the intended sphere cohort.
```

### G6. Cap classifier `0.1 * period` non-commensurate-axis comparison

```
CODE:
    triangulation.rs:5975.

EVIDENCE AVAILABLE:
    The non-periodic axis span is compared to the periodic period; the two axes
    have no intrinsic physical scale relation (on a sphere, 0.1·2π ≈ 0.63 rad of
    latitude spread would pass).

PRODUCTION CLAIM:
    The gate's passing selects "a closed latitude-parallel boundary ... the
    non-periodic coordinate essentially constant across the loop".

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    The gate is a candidate filter; the signed-area gate and the collapse search
    actually constrain the class. As a *proof* the loop is a cap boundary it is
    insufficient; as a candidate gate it is acceptable.

CORRECT EPISTEMIC STATUS:
    HEURISTIC (candidate gate).

POTENTIAL FAILURE CLASS:
    chart-degeneracy → cap-topology promotion on a near-1D loop that is not a
    latitude parallel.

EXPECTED ABC SIGNATURE:
    Fabricated caps on loops whose non-periodic coordinate drifts up to 0.1·period
    with near-zero UV area; would render (wrong geometry), invisible to the loss
    ledger.
```

### G7. Material side rests on the `surface.invert()` mechanism the project flags suspect

```
CODE:
    triangulation.rs:6057 (surface.normal), 6062 (n×t);
    source_evidence.rs:174-179 (is_suspect), 544-549 (normalized_sign None on the
    compressed path).

EVIDENCE AVAILABLE:
    s_f is folded via Processor::invert() at convert.rs:470-472; source_evidence
    marks FaceSurfaceSenseFoldedViaSurfaceInvert as unsound ("breaks curve-on-surface
    incidence"). The normalized traversal sign is not computable on the compressed
    path.

PRODUCTION CLAIM:
    "The material side is decided by the STEP face orientation convention ... n is
    the effective surface normal (which already carries FACE_SURFACE.same_sense via
    surface.invert())".

WHY EVIDENCE DOES / DOES NOT IMPLY CLAIM:
    The composition is internally consistent and test-validated; but the mechanism
    used to carry s_f is the one the project's own evidence model records as
    suspect, and face.orientation is excluded without proof. The claim is
    constructive, not independently certified.

CORRECT EPISTEMIC STATUS:
    CONSTRUCTIVE (built witness, internally validated); not CERTIFIED.

POTENTIAL FAILURE CLASS:
    material-side inversion for a face whose same_sense/ORIENTED_FACE folding the
    converter mishandles; wrong cap (north/south) rendered silently.

EXPECTED ABC SIGNATURE:
    Caps on the wrong hemisphere for inverted faces; would render wrong geometry,
    invisible to the loss ledger. The two NIST targets are verified correct by the
    render check.
```

---

## H. Ranked audit findings

Ranked by (1) semantic severity, (2) breadth of possible activation, (3) likelihood of explaining ABC churn.

| Rank | Finding | Severity | Breadth | ABC-churn likelihood | Status |
|---|---|---|---|---|---|
| 1 | **G1 P2 `CertifiedAmbiguous` → `RejectedAmbiguous` source-ambiguity certificate** | High (classifies unresolved tessellation as certified source ambiguity; terminal, blocks recovery) | Medium (pole-meeting great-circle edges; confirmed on 7 ABC faces) | Low for rendered counts (the 7 were already lost), **High for census semantics** (rejected_ambiguous 0 → 7) | **INVALID — confirmed** |
| 2 | **G2 P1 `evaluation_range()` vs source trim/closure** | High if real (193 plane + 71 cylinder rendered→lost) | High (every unclamped spline edge) | **High candidate** for the 308 rendered→lost cluster; attribution blocked by baseline-provenance confounder | **UNRESOLVED** — needs intermediate-pin sweep |
| 3 | **G3 P3b "certified" orbit collapse** | Medium (would fabricate wrong caps on near-collapse generic surfaces) | Medium (any single |k|=1 loop with a near-collapse on the material side) | Low for losses (renders wrong geometry instead); undetectable in the loss ledger | **HEURISTIC — naming overclaim** |
| 4 | **G4 declared (uncertified) period used as certified `T`** | Medium (theorem hypothesis unmet on production targets) | High (every P2/P3b activation on spheres) | Low (sphere values correct) | **HEURISTIC** |
| 5 | **G5 `so_small` absolute singularity activation** | Medium | Medium (any exhausted step with a small periodic partial) | Low–medium (could route regular faces into recovery) | **HEURISTIC** |
| 6 | **G6 `0.1 * period` cap classifier** | Low–medium | Low (narrowed by signed-area and collapse gates) | Low | **HEURISTIC** |
| 7 | **C7/G7 P1/P3b range clash + material-side invert reliance** | Medium (latent) | Low today (splines non-periodic in production; swept/offset surfaces periodic-spline-backed in principle) | None observed | **HEURISTIC / CONSTRUCTIVE** |

**What the ABC numbers actually support:** the 375 gained faces are dominated by bspline (164) + nurbs (109) + sphere (34) — exactly P1 (splines) and P2/P3b (spheres) recoveries. The 308 lost faces are dominated by plane (193) + cylinder (71) — families with **no intended P1/P2/P3b mechanism** and, for the representative cylinder band case (00005760 #120321, chart_rank=1, circular edges), no plausible P1 edge-sampling mechanism either. **On the current evidence the rendered→lost population cannot be causally attributed to P1/P2/P3b and may be a baseline-provenance artifact** (the packet's §11 warning applies). Only finding 1 (7 faces) and findings 3–6 (semantic/classification) are directly established by code + corpus.

---

## I. Minimal falsification plan

No fixes implemented. For each RED/UNKNOWN finding, the smallest existing artifact or static check that would confirm/refute it:

**I1 (G1, INVALID — P2 RejectedAmbiguous).** Already falsified by the saved corpus: `look-corpus\nist-rec\abc\{00000959,00001075,00005760}.jsonl` rows with `terminal_reason=RejectedAmbiguous` and `deck_status=Unavailable`. Confirm refutation: re-run those three models at `f9e06c64` with `TRUCK_LIFT_SINGULAR_RECOVERY=0` and confirm they return to `NoSurfaceProduced`/`AmbiguousLift` (unresolved), and that no `source_segment_count`/`boundary_pieces` evidence exists that could support two continuations. Static check: grep `singular_transition_branch` for any construction of a second continuation — none exists (returns are `Continue`/`CertifiedAmbiguous`/`NotApplicable` only).

**I2 (G2, UNRESOLVED — P1 sampling vs source closure).** Smallest decisive experiment (do not run during this audit): sweep 00007705 only, at pins `018bd469` (baseline) and `17ac0f15` (P1 only), same census binary, and diff the rendered→lost ledger. If 00007705's 247 faces regress between `018bd469` and `17ac0f15`, P1 is causally implicated and the fix is scoped; if they regress between the *stored planar-c ledgers* and `018bd469`, the planar-c baseline is stale and the 308 number is invalid as a P1/P2/P3b regression. Static cross-check: for one regressed plane face, confirm the boundary edge curve is a `BSplineCurve` with unclamped end knots (inspect `00007705_5e85263979fb44d18725b575_step_003.step` around the face's edge_curve) — if the edges are conics/lines, P1 is exonerated for the plane cluster.

**I3 (G3, HEURISTIC — P3b collapse certificate).** Static check: enumerate every surface family that can reach `find_cap_pole` with a declared period through `look::lattice_of` (sphere, swept, offset). For each, verify an exact-collapse witness exists (analytic for sphere; none for swept/offset — confirming the certificate is heuristic there). Corpus query: count ABC faces whose mesh contains a `ChartClosure` segment whose pole-line endpoints are >1e-4·r_loop apart in world space (would indicate a non-collapse accepted as a pole). Use `look-corpus\nist-rec\abc\*.jsonl` `seam_segment_count`/`boundary_pieces` if populated.

**I4 (G4, HEURISTIC — uncertified period).** Static check already decisive: `lattice.rs:79,104-107` returns `Uncertified` for spheres while P2/P3b read `declared_period()`. Confirm refutation: assert `deck_status=Unavailable` for every `RejectedAmbiguous` and every cap-recovered sphere face in the saved JSONL (both already true). No further experiment needed for classification; a certified-period path would require reading `Sphere`'s primitive structurally in `look::lattice_of`.

**I5 (G5, HEURISTIC — so_small activation).** Static check: no rank/analytic witness is attached to `collapsed_axis`. Corpus query: among faces reaching the P2 branch (probe `TRUCK_PROBE_LIFT` with `TRUCK_LIFT_SINGULAR_RECOVERY` on/off on ABC), verify no face that `Continue`s is later proved non-singular by the CDT/material stage (i.e., no wrong-branch mesh appears).

**I6 (G6, HEURISTIC — cap classifier).** Static check: for the two NIST caps, confirm `n_max−n_min` is ~0 (exact latitude) and that the gate's 0.1·period slack is never the deciding factor on the corpus. Corpus query: list all ABC faces with exactly one closed |k|=1 loop and zero signed area (use saved JSONL `boundary_pieces`); each is either a genuine cap (verified material side) or a falsification candidate.

**I7 (C7/G7, HEURISTIC/CONSTRUCTIVE).** Static check: `find_cap_pole` must switch its scan band to `evaluation_range()` for parity with P1; verify no production lattice adapter can currently declare a period on a spline-backed surface (it cannot today). Material side: the definitive falsifier is a corpus of same_sense=.F. sphere caps with an independently known hemisphere; compare the produced mesh's bounding-box side. The two NIST targets already pass this check.

**Baseline-provenance gate (applies to I2 and to the §11 falsification corpus):** before any ABC rendered→lost attribution, re-derive the PLANAR-C baseline numbers from a clean build of `018bd469` (the state this recovery sequence started from). The stored `look-corpus\planar-c\abc` ledgers must reproduce 837103 rendered / 2076 lost; if they do not, every rendered→lost claim against them is void.

---

## Stopping condition

> For every semantic operation introduced by or materially interacting with P1, P2, and P3b: the proposition production claims, the evidence that supports it, and whether the implication is formally valid.

Answer:
- **P1** claims "boundary sampling and projection presearch use the evaluable domain". Supported: interior-knot partition-of-unity + NIST structure. Formally valid for the *evaluability* claim; the assumption `D_eval = D_source_edge` for every spline edge is **not** proved and is the one live risk (UNRESOLVED on ABC).
- **P2** claims "unique continuation from the leaving edge's plane" (CONSTRUCTIVE, sound, but with an uncertified period) and "source-level ambiguity certificate" for the underdetermined-leaving-edge branch (**INVALID**; confirmed on 7 ABC faces).
- **P3a** claims nothing positive; the negative invariant is respected (no promotion of chart degeneracy to source degeneracy).
- **P3b** claims "the cap cell is source/material equivalent under hypotheses H1∧…∧H6". The construction (H6, closure roles, material side) is correct and provably equivalent; the hypotheses H1 (period) and H4 (collapse) are heuristic for the production targets, and H4 is mislabeled "certified".

The next implementation packet should (a) demote P2's `CertifiedAmbiguous` to the ordinary unresolved `AmbiguousLift` unless a genuine second continuation is constructed, (b) reword/guard the "certified" collapse language and, if feasible, add a derivative-rank or analytic-collapse witness for the pole (sphere primitive reader), (c) align the P3b pole-search band with `evaluation_range()` and read the sphere period structurally so H1 becomes certified, and (d) attribute the ABC rendered→lost population at the intermediate pins before any further change. This session implements none of that.
