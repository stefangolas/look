# Certified Flux Calculus — integration map into the existing codebase

2026-09-13, orchestrator session. Companion to `docs/BOOLEAN_MACHINERY_THEORY_GAP.md`
and `docs/BOOLEAN_MACHINERY_THEORY_GAP.md`'s successor theory document (the
owner's "Certified Flux Calculus" proposal, which supersedes G1's inline T1–T6
as normative theory for the integration layer). This file maps every
architecture element of that proposal onto the code as it exists today, names
what is REUSED vs NEW, and assigns each element to a packet vehicle with
difficulty and expected LOC. All anchors below were measured at HEAD
2026-09-13; re-measure at dispatch (they drift per landing).

## 0. The reuse inventory (what the proposal consumes that already exists)

| existing machinery | location | consumed by |
|---|---|---|
| Exact Bernstein flux kernel (`cell_flux_exact`, 6 refs) | `bd_bridge.rs` | Primitive A (poly_integral) — its semantics are preserved verbatim; the kernel is the `W=const` special case |
| Face-form certified bracket (`binding_volume_facts`, 3 refs) | `binding.rs` / `bd_bridge.rs` | Primitive D's per-patch certification entry point |
| Patch extraction (`node_box_patches` ×3, `placed_box_patches` ×2) | `bd_bridge.rs` | the router's input; unchanged |
| Boolean funnel + fold (`boolean_product_volume_certified` ×4, `fold_operands`/`fold_collect`) | `bd_bridge.rs` | unchanged; consumes intervals instead of scalars (Minkowski addition) |
| Content-hash memo pattern (`facts_memo`/`patch_memo`/`product_memo` ×3 each) | `bd_bridge.rs:4307-4340` | §17 caches (intrinsic-key extension of the same pattern) |
| Phase instrumentation (`PHASE_*` accumulators, `timing.phases` columns) | `bd_bridge.rs:4369+` (G6 landing) | §20's E1–E5 measurement harness — already live |
| Planar end-cap exact terms (`(1/3)·d·A`, `spline_loop_area_vector` ×11) | `bd_bridge.rs:2436-2457` | §8's planarity router generalizes exactly this pattern |
| `VolumeRow { numerator, weights, orientation }` (41 refs) | `bd_bridge.rs`/`binding.rs` | the weight channel EXISTS, pinned `1.0` — the rational arm un-pins it |
| MONO-5 membership margins, MONO-9 fold, MONO-8 admission | `bd_bridge.rs` | unchanged interfaces; MONO-9 gains interval addition; MONO-8's admission test is replaced per §6.1 |
| Packet-test culture (byte-identity gates, typed refusals, `cert_cost_scale.rs`) | `truck123d/tests/` | the proposal's T1–T12 acceptance suite lands here |

## 1. Element map: proposal → code → packet

| proposal element | lands where | reuse vs new | packet | difficulty | est. LOC |
|---|---|---|---|---|---|
| §2 two-channel certificate `(ℓ,h;w_m,w_i)`, Cor 2.3 migration | `facade.rs` (types) + `bd_bridge.rs` (plumbing) | types NEW; migration mechanical (exact outputs embed as `(q,q;0,0)`) | **G1 (amended)** | design | ~120 |
| §3.4/3.5 sum-factorized exact contraction (`O(d⁴)`, no degree-tripled grid) | inside the exact kernel, `bd_bridge.rs` | replaces the inner loop; **byte-identical outputs** (exact arithmetic) — the existing byte-identity gate is the test | **G1 (amended)** | mechanical+ | ~250 |
| §3.2/3.6 denominator clearing + integer contraction | same | NEW table `Λ_{m,n}` + integer path | **G1 (amended)** | mechanical+ | (in the ~250 above) |
| §4.1 exact degree reduction (homogeneous `(A,W)`) | admission path, `bd_bridge.rs` | NEW `O(r)` reducibility test; Bernstein elevation ops may exist in the patch layer — reuse where present | **G1 (amended)** | mechanical | ~100 |
| §4.2 floating filter + §4.3 convex-hull culling | predicates in the funnel | culling: extends existing box culling; filter: NEW, exact-fallback keeps soundness | **G1 (amended)** | mechanical | ~120 |
| §5 reciprocal-power kernel (Thm 5.2, 1D + 2D, any k) | `bd_bridge.rs` (new `cell_flux_certified` + `reciprocal_power_integral`) | the `(L,U)` bounds reuse the existing Bernstein-range certification; the expansion mechanism is NEW (replaces G1's L5-based draft; L5 dependency dropped) | **G1 (amended)** | **design (the core)** | ~350 |
| §6 rational surface arm `((P₃,W),k=3)` + §6.1 sign admission | `cell_flux_certified` + MONO-8 amendment | `VolumeRow.weights` un-pinned; homogeneous placement/subdivision (`placed_*` gains the `(X,Y,Z,W)` arm) | **G1 (amended)** | design | ~300 |
| MONO-9 interval fold (Minkowski) | `fold_operands` | extension of existing fold; `Exact(x)=[x,x]` | **G1 (amended)** | mechanical | ~80 |
| §8 planarity router (coefficient test `ν·A=cw` + area-vector boundary route + 1D `k=2` rational edges) | `bd_bridge.rs`, routed from the per-patch certification site | **generalizes the landed `(1/3)·d·A` end-cap pattern** (reuse `spline_loop_area_vector`); the FHC-E fan cap is the polynomial-planar special case — parametrization-independent, the degenerate centroid dissolves | **G8-PLANARITY-ROUTER** (absorbs FHC-E) | mechanical+ | ~350 |
| §9.1–9.3 Green reduction (exact Bernstein antidifferentiation, trim pullbacks, rational trims → 1D `k≤r+s+2`) | `bd_bridge.rs` (new Green module section) | antidifferentiation NEW (Lemma 9.2 is a coefficient map on existing Bernstein ops); rational-trim 1D reuses the G1 kernel | **G9-GREEN-TRIM-INTEGRATION** | design-leaning mechanical+ | ~700 |
| §10 approximate trims (`w_m` channel) | certificate plumbing | NEW; deferred until the SSI/pullback layer can certify Assumption 10.1 | **deferred** | — | — |
| §11 placement covariance (Φ₀, A⃗ invariants; Thm 11.1 transform) | `bd_bridge.rs` memo layer | extends the **landed `patch_memo`/`facts_memo` pattern** with intrinsic (unplaced) keys; `spline_loop_area_vector` already computes A⃗-class quantities | **G10-PLACEMENT-COVARIANCE-CACHE** | mechanical | ~250 |
| §12 adaptive controller (two actions, nesting [C]) | controller over the certification layer | needs E1–E5 data from the landed phase timers; the value-mode/decision-mode split answers T2's output-contract question | **deferred — author after G1 lands** | design | ~500 est. |
| §18 kernel API (primitives A–D) | the four functions above ARE the API | no separate surface — they live beside `cell_flux_exact` | (spread across packets) | — | — |
| §19 acceptance tests T1–T12 | `truck123d/tests/rational_flux.rs` (G1), `planarity_router.rs` (G8), `green_trims.rs` (G9), `placement_cache.rs` (G10) | G1's existing 9-test suite embeds (its test 9 = the end-to-end CYLINDER+CYLINDER union remains the crown jewel) | per packet | — | ~400 total |

## 2. Packet vehicles, difficulty, LOC

| packet | status | depends | difficulty | est. LOC (kernel + tests) |
|---|---|---|---|---|
| **FHC-G1-RATIONAL-FLUX (amended)** | authored this session | G6 (unlanded — adjudication pending) | design — the soundness demonstration + homogeneous plumbing | ~1,100 (≈700 kernel + 400 tests) |
| **FHC-G8-PLANARITY-ROUTER** (absorbs FHC-E) | authored this session | G1 (needs the 1D `k=2` rational-edge path) | mechanical+ — the router is a coefficient test + reuse of the landed end-cap pattern | ~450 (≈250 + 200) |
| **FHC-G9-GREEN-TRIM-INTEGRATION** | authored this session | G1 (1D reciprocal for rational trims) | design-leaning mechanical+ — Green is exact machinery; the trim pullback carriers are new representation | ~800 (≈500 + 300) |
| **FHC-G10-PLACEMENT-COVARIANCE-CACHE** | authored this session | none (exact polynomial quantities today) | mechanical — extends the landed memo pattern; bit-identical under placement by an exact law | ~300 (≈150 + 150) |
| Adaptive controller (§12) | **deferred** | G1 landed + E1–E5 data | design | ~500 est. |
| Approximate trims (§10) | **deferred** | SSI/pullback layer | design | — |

All kernel-side packets write `truck123d/src/bd_bridge.rs` (+ facade for G1), so
dispatch_ready serializes them on the write set: G1 → G8 → G9 → G10 is the
expected landing order, with G10 legally dispatchable earlier if a slot frees
before G1 (no needs edge encodes posture — the serializer handles it).

## 3. What stays out of scope (unchanged from the theory doc §22)

SSI construction/certification, certified 3D→(u,v) pullback, membership
topology, non-manifold and n-ary arrangement outputs, corpus-optimal
order-vs-subdivision policy (E-measured). The funnel's contact-cover layer owns
these seams; the integration theory consumes their outputs.

## 4. Governance note

The proposal supersedes G1's inline T1–T6 as normative theory (owner adoption
2026-09-13). The L5/`Modulus::compose` precondition in the original G1 packet
is DROPPED — Theorem 5.2 does not consume composite reciprocals. FHC-E is
folded into G8 (its row targets move; the register's 4b entry resolves when G8
lands). FHC-D is orthogonal (door-side marshalling) and unchanged.
