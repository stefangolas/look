# WORK PACKET MONO-5-RAY-CLASSIFY — certified point-vs-spline-solid membership

The wave-3 boolean solver consumes a certified membership primitive: for a
point `p` and a spline-patch-bounded solid `S` (the landed loft/member rows:
finite oriented 2-cycles of bicubic tensor-Bernstein patches), decide
`p ∈ int S` / `p ∉ S` / typed-indeterminate, with certificates. This is
wave-3 amendment 3 (docs/MONO_WAVE3_THEORY_BRIEF.md review verdict, 9be72b7):
the mechanism must be NAMED, not assumed. The mechanism is **certified ray ×
bicubic root isolation**: cast a recorded-direction ray from `p`, isolate the
ray's crossings with every patch by 1-D certified bracketing (Bernstein
clipping per patch over the ray parameter), count signed crossings with
outward-rounding interval arithmetic. 1-D bracketing is strictly easier than
the 4-D contact problem and reuses the landed interval discipline.

```yaml
id:          MONO-5-RAY-CLASSIFY
contract:    [MONO-5-RAY-CLASSIFY]
class:       mechanical
crates:      [truck123d]
depends_on:  [MONO-4-TRIM-IDIOMS]
write_allow:
  - truck123d/src/bd_bridge.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/MONO_WAVE3_THEORY_BRIEF.md
  - docs/MONO_CLOSURE_BOOKING.md
tests_required: []
anchors:
  - {id: A1, expect: 4,  cmd: "grep -c 'ray' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 0,  cmd: "grep -ci 'classify' truck123d/src/bd_bridge.rs"}
  - {id: A3, expect: 0,  cmd: "grep -ci 'bicubic' truck123d/src/bd_bridge.rs"}
budget:      {turns: 55, ctx_tokens: 170000}
```

## Method

1. **Patch-side intersection.** For one patch `P` (degree (3,3) Bernstein
   net, weights, orientation) and the ray `r(t) = p + t·d`: the crossing
   condition is 3 equations in 1 unknown — eliminate to the scalar polynomial
   system via the projected resultant OR (preferred, simpler to certify) the
   Bernstein-clipping loop: interval-evaluate `r(t) − P(u,v)` over the box
   `[t_lo,t_hi] x D_P`, subdivide the PATCH domain, discard boxes where any
   coordinate range excludes zero; surviving (t-interval, patch-subdomain)
   pairs are certified crossing enclosures. Termination: transversal
   ray-patch intersections (the admitted class) are isolated points in
   `(t,u,v)`; the hull property makes every box at positive distance from a
   crossing exclude at finite depth. Non-transversal (ray grazing/tangent to
   a patch) -> `Refusal::MembershipIndeterminate` — the caller retries with
   a different recorded direction (the packet's retry contract, matching
   wave-3 section 9).
2. **Signed crossing count.** Parity with orientation: each certified
   crossing contributes the sign of `d · n_P` (interval-certified nonzero,
   else indeterminate). Odd total -> inside; even -> outside. All arithmetic
   outward-rounded (the landed interval discipline; no naked f64 decisions).
3. **Weights admission.** Certify `weights ≡ 1` for every consumed patch
   (the corpus's lofts/members are non-rational) or consume the landed
   `VolumeRow` weight path — one of the two, stated in the code path.
4. **Retry contract.** Up to a small recorded bound of fresh rays (fixed
   pseudo-random-but-deterministic directions from the recorded seed, or the
   coordinate axes cycled); failure of all -> typed
   `MembershipIndeterminate`. No guessed classification.
5. **Tests.** Known-answer fixtures: a prism row (crossing counts by
   construction), a two-station loft (crossings hand-derivable), points
   near the boundary (bracket tightness), grazing rays (indeterminate, not
   wrong answers), and the landed rows' round-trip (inside centroid of a
   solid classified inside; a point certified outside a separated solid).
6. All cargo through the queue; no OCC anywhere.

## Done when

- check + full lib tests green serial, including the named fixtures;
  fmt/clippy clean on added lines.
- Anchors A2/A3 drift 0 -> >= 1 (the mechanism now exists in the bridge);
  A1 drifts (record post-work).
- The primitive's contract in RESULT.json notes: inputs (point, direction,
  patch set plain-data), outputs (Inside/Outside/Indeterminate + the
  crossing-evidence certificate), and the retry bound.

## Stop conditions

- If a ray-patch configuration in the fixture set cannot be certified OR
  refused within the loop discipline (i.e., the clipping loop does not
  terminate on an admitted transversal case), STOP and record the case —
  termination is a theorem obligation, not a tuning knob.

Write RESULT.json AT THE WORKTREE ROOT.
