# SEM-ARRANGE-DYADIC-CONTRACT-001 — arrange's certified predicates require a dyadic extended-line intersection lattice; violations surface as a misleading RootNotIsolated

**Family** `SEM` · **Manifestation** `MISATTRIBUTION`, `REFUSAL`
**Contracts** `BREP_GENERATION_API.md` §6 documents "dyadic vertices" as v1
scope — but the implemented requirement is strictly stronger than the
documentation states, and the refusal a violating client receives names the
wrong cause.

## 1. Status

```
Mechanism established (13-case empirical isolation; internal predicate site not yet localized)
```

## 2. Mathematical objects

A closed line-segment profile for `arrange` (the certified planar
arrangement, `truck-geometry/src/arrange.rs`). The certified predicates run
over `Dyad { num, exp }` exact arithmetic (`arrange.rs:1590`). The
empirically established admissibility rule:

> **Every non-adjacent segment pair's EXTENDED lines must intersect in an
> exactly-representable (dyadic) point.** With segment slopes $m_i$ and
> offsets $b_i$, the intersection $x^* = (b_j - b_i)/(m_i - m_j)$ must be
> dyadic for every non-parallel cross pair; parallel pairs and shared
> vertices are fine.

Sufficient (not necessary): all slopes in $\pm 2^k \cup \{0\} \cup
\{\infty\}$ — then every difference is a power of two and every quotient
stays dyadic.

## 3. Required obligation

Input outside the documented envelope must be refused at the boundary with
a typed, *accurate* refusal naming the violated obligation (which pair /
which lattice point). The documented contract ("dyadic vertices") must also
be corrected to state the lattice requirement — the vertex-level statement
is strictly weaker and misleads authors.

## 4. What the implementation did

`recognize` (`arrange.rs:718-759`) accepts arbitrary finite planar
coordinates. The failure surfaces later, inside the certified predicate
chain, as `NumericallyUnresolved { spent: Budget::new(0,0,0), witness:
RootNotIsolated }` — a witness claiming root-isolation failure, independent
of the actual lattice violation. The generic `numerically_unresolved()`
helper (`arrange.rs:1618`) stamps `RootNotIsolated` for every unresolved
predicate regardless of cause.

## 5. Minimal counterexample

`showcases/examples/arrange_probe.rs` — 13 closed profiles, one variable at
a time. All coordinates dyadic unless stated:

| profile | slopes (dy/dx) | lattice | result |
|---|---|---|---|
| landed `revolve_p5` tube | {0, −2, 0, ∞} | dyadic | Ok |
| vessel quad control | {4, 0, 8, 0} | dyadic ((0,−8), (1,4)) | Ok |
| vessel quad, walls 0.9° apart | {16, 0, 64/3, 0} | dyadic | Ok — near-parallelism exonerated |
| vessel quad, exactly-parallel walls | {16, 0, 16, 0} | none needed | Ok — exact parallelism exonerated |
| vessel quad, non-dyadic vertices | {20, 0, 25, 0} | non-dyadic | **Refused** |
| 5-gon slanted | {0, 2, 2, 0, 8} | −2/3 | **Refused** |
| 5-gon with vertical | {0, ∞, 0, −1, −2} | dyadic | Ok |
| outer-wall 5-gon (teapot) | {0.75, 0.25, −0.5, −1, ∞} | 27/22, 105/184 | **Refused** |
| outer-wall 4-gon | {0.75, 0.25, −2, 6} | 27/22 | **Refused** |
| lattice-clean quad | {2, 0, 4, 0} | (−1, −6) | Ok |
| lattice-dirty quad | {4, 0, 16/3, 0} | −0.75 (dyadic — prediction corrected) | Ok |
| **rhombic vessel** (teapot fix) | {1, −1, −1, 1, 0} | dyadic | **Ok** |
| full 10-point silhouette | mixed | multiple non-dyadic | **Refused** |

The real-world trigger: any hollow-vessel silhouette authored from physical
proportions (the teapot body).

## 6. Control / oracle

Twin profiles identical in topology and conditioning, differing only in
whether some cross-pair lattice point is dyadic (rows 10/12 vs 5/7/8/9).
The oracle is the lattice rule itself; the probe is permanent.

## 7. Measurements

All rows above, fresh runs this session (dev profile). The teapot body hit
the same refusal through the full `arrange → revolve_profile` path before
the rhombic redesign.

## 8. First divergent checkpoint

**Input recognition** accepts arbitrary coordinates; the lattice assumption
first bites in the certified `Dyad`-exact cross-pair predicates; the
generic `numerically_unresolved()` helper mislabels the cause.

## 9. Causal derivation

```
recognize accepts arbitrary f64 coordinates
→ certified predicates assume the extended-line intersection lattice is dyadic
→ an arbitrary-slope profile produces non-dyadic lattice points
→ a cross-pair predicate goes Unresolved
→ the generic helper mislabels it RootNotIsolated
→ the client debugs numeric conditioning / near-parallelism that is not the problem
```

## 10. Proposed correction

1. **Validation**: at the arrange entry, compute the pairwise lattice
   symbols and refuse with a typed case naming the offending pair and
   point (mechanical; the probe is the regression test).
2. **Documentation**: correct §6's "dyadic vertices" to state the lattice
   requirement.
3. **Long term**: predicates that do not require an exact lattice (the BIE
   program's interval machinery is one route), so arbitrary measured
   profiles arrange.

## 11. Experimental correction

Client-side redesign validated: the teapot body re-authored as a rhombic
vessel (slopes {±1, 0}, lattice clean) arranges and revolves end to end.

## 12. Production correction

None yet — candidate for the loop (mechanical validation + doc fix).

## 13. Regression tests

`showcases/examples/arrange_probe.rs` (the 13-row table).

## 14. Corpus-wide effect

Every profile authored from measured proportions is refused until
hand-designed into the lattice class. The landed kernel tests pass because
axis-aligned profiles are lattice-clean by accident.

## 15. Known exclusions

Sweeps (`spine_sweep`/`facet_sweep`) do not consume `arrange` and are
unaffected; only the `extrude`/`revolve` family is gated.

## 16. Relationship to other defects

Same family as the five `CC-DEF-BREP-FIXES` items (typed-refusal fidelity),
one layer deeper (profile arrangement rather than realization). Supersedes
this file's earlier "dyadic vertices" framing — the near-parallel and
exact-parallel hypotheses were tested and exonerated (rows 3–4).

## 17. Claim status

- **(D)** The 13-row isolation — measured; the probe is permanent.
- **(D)** The misattribution (witness names root isolation; the actual
  variable is the lattice class) — measured by the twin rows.
- **(A)** The lattice rule as the exact admissibility criterion — consistent
  with all 13 rows; the packet that lands validation owns the precise
  statement from the predicate source.

## 18. Links

- `truck-geometry/src/arrange.rs` (`recognize` :718, `numerically_unresolved` :1618, `Dyad` :1590)
- `docs/BREP_GENERATION_API.md` §6 (the incomplete "dyadic vertices" statement)
- `showcases/examples/arrange_probe.rs`, `showcases/src/teapot.rs`
