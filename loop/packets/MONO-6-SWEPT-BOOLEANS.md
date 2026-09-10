# WORK PACKET MONO-6-SWEPT-BOOLEANS — certified boolean volume by contact covers

The wave-3 solver, per the ACCEPTED frontier theory ("Certified Boolean
Volume by Contact Covers", review verdict 9be72b7 in
docs/MONO_WAVE3_THEORY_BRIEF.md — the returned document is the spec; its
four review amendments are incorporated HERE as normative method items).
Objective: for solids A, B whose boundaries are finite oriented 2-cycles of
bidegree-(3,3) tensor-Bernstein patches, compute a certified bracket
[V_lo, V_hi] ∋ V(A\B) with width <= eps (the corpus facts band, 1e-4
relative), or a typed refusal. The central design constraint: NO
intersection-curve reconstruction, no trim loops, no chaining — a shrinking
cover of the contact locus plus exact flux integration over classified cells
plus area-based error accounting.

```yaml
id:          MONO-6-SWEPT-BOOLEANS
contract:    [MONO-6-SWEPT-BOOLEANS]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-5-RAY-CLASSIFY, MONO-2-NSTATION-LOFT]
write_allow:
  - truck123d/src/bd_bridge.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO_WAVE3_THEORY_BRIEF.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: []
anchors:
  - {id: A1, expect: 0,  cmd: "grep -c 'contact_cover' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 13, cmd: "grep -c 'VolumeRow' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 7,  cmd: "grep -cE 'fn member_[a-z_]+\\(' truck123d/src/bd_bridge.rs"}
budget:      {turns: 70, ctx_tokens: 200000}
```

## Method

1. **Reduction (theory sections 1-2, verified in review).** For each patch P
   of dA and Q of dB: `g_P = (1/3) P · (P_u x P_v)` — degree (8,8), exact
   polynomial integration per rectangular cell through the landed
   `volume_facts` machinery (subdivide the patch domain into cells; exact
   child control nets). `V(A∩B) = Σ_P ∫ 1_B(P) g_P + Σ_Q ∫ 1_A(Q) g_Q`;
   `V(A\B) = V(A) − V(A∩B)`; V(A) from the landed loft/member certificates.
2. **Contact cover (theory section 3, Theorem 1).** For each patch pair,
   product boxes `Z = R_P x R_Q`; Bernstein range exclusion per coordinate
   (theory eq. 11 — control-hull min/max of P_i and Q_i; valid as a
   superset bracket). Surviving boxes project onto P's domain forming the
   cover U_P. **Amendment 2 (Lemma-3 termination, separable test):** the
   exclusion that must eventually fire is coordinate-range separation at the
   closest pair — at the minimizing (p0, q0) some coordinate differs by
   >= d/sqrt(3); boxes shrinking around that pair separate ranges in that
   coordinate. Implement the worklist so the closest-pair box is always
   subdivided (error-directed priority below satisfies this implicitly);
   the proof obligation is discharged in a code comment citing the argument.
3. **Error-directed refinement (theory section 4, Theorem 2).** Unresolved
   cell R contributes the bracket (5): `[|R| min(0, g_lo), |R| max(0, g_hi)]`;
   `E(R) = |R| (max(0,g_hi) − min(0,g_lo))`. Worklist priority = E(R)
   (descending). Termination: `Σ E(R) <= eps` (eq. 8). Record depth and
   per-phase work (broad-phase / exclusion / refinement) for attribution.
4. **Clear-component witnesses (theory sections 1, 9).** Components of
   D_P \ U_P have constant membership (Theorem 1); ONE certified query per
   component via the landed MONO-5-RAY-CLASSIFY primitive, with its retry
   contract. Indeterminate after retries -> typed refusal, never a guess.
5. **Broad phase (theory section 7).** Control-hull BVH over both patch
   sets (the landed CFP-005 span-BVH pattern) — candidate pairs only; most
   patch pairs exclude at the hull test.
6. **AMENDMENT 1 — facts-gate sufficiency lemmas** (the row gate wants
   solid_count, volume, bbox; the solver produces volume):
   - `solid_count` is CONSTRUCTIVE: the drop-in's boolean row answers
     solids() = [self] (door.py:1153 semantics) — no topology certification.
   - `bbox(A\B)`: EXTREMES-SURVIVE theorem — for each of the 6 axis slabs of
     A's certified bbox, interval-certify that B's certified bbox does not
     reach the arg-extreme region (separation check on landed bbox bounds);
     then bbox(A\B) = bbox(A) exactly. Any slab violated -> typed refusal
     (new named case `ExtremeSlabContaminated`) — the corpus's tub/cavity
     pair passes with margin (verify and record the margin).
7. **AMENDMENT 4 — weights admission:** every consumed patch certifies
   weights ≡ 1 (corpus lofts/members are non-rational) or consumes the
   VolumeRow weight path; state which per code path. The (8,8) polynomial
   flux integrand assumes non-rational.
8. **Typed refusals (theory section 10):** NonRegularPatch,
   TransversalityUncertified (the sigma-min margin check on admitted pairs),
   MembershipIndeterminate, BudgetExceeded, ExtremeSlabContaminated. Near
   tangency / coincident surfaces refuse; no fallback to tolerance behavior.
9. **Validation (theory section 13, binding):** closed-form pairs
   (box/box, coaxial cylinders, extrusion differences) exact within
   certificates; Bernstein pairs with crossing-patch-boundary and
   interior-loop contacts; small-angle-but-admitted pairs; the real
   tub-skin/cavity pair end to end with V_reference ∈ [V_lo, V_hi] and
   width <= 1e-4 relative. Subdivision depth + pair counts recorded per case.
10. All cargo through the queue; no OCC anywhere; corpus references
    read-only.

## Done when

- check + full lib tests green serial including the validation suite above;
  fmt/clippy clean on added lines.
- The tub-skin/cavity case: certified bracket contains the recorded
  reference volume with width <= 1e-4 relative — the monocoque cut's
  volume cell is thereby closed.
- Anchors: A1 drifts 0 -> >= 1; A2/A3 drift (record post-work counts).
- RESULT.json notes carry the per-case depth/attribution table and the
  refusal inventory exercised.

## Stop conditions

- A validation case whose certified bracket fails to close within budget on
  an ADMITTED (transversal, regular) pair: STOP, record the pair and the
  residual — Theorem 4 guarantees termination, so a failure is an
  implementation defect to be located, never a tolerance stretch.
- A corpus pair refused by the transversality admission: record the measured
  sigma margin — that is a TRUE carrier boundary, not a defect.

Write RESULT.json AT THE WORKTREE ROOT.
