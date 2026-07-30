# QUO-EUCLIDEAN-CLOSURE-001 — Periodic closure tested by lifted Euclidean equality

**Family** `QUO` · **Manifestation** `OMISSION` (measured on the reproducer)
**Contracts** `QUO-002`, `QUO-005`

> Full record:
> [`../look-collapsed-boundary/FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md)
> §4 step 2, U6, U7.

```
Status: Mechanism established
```

## Obligation

A boundary on a periodic surface closes **in the quotient**
$Q = \Omega/\Lambda$, and the residual must be measured in the **physical**
metric, not in parameter space:

$$\tilde\gamma(1) - \tilde\gamma(0) = (k_u P_u,\, k_v P_v) + r,
\qquad \lVert r \rVert_G = \sqrt{r^{\mathsf T} G(q_0)\, r} \le \varepsilon .$$

## What the implementation did

`PolyBoundary::new` sorts sampled pieces into `closed` / `open` by whether a
piece returns to its own starting $uv$ **within a Euclidean tolerance in
parameter space**. No period lattice, no metric $G$, no winding vector.

A base circle running $v: 0 \to 2\pi$ at constant $u$ is physically closed and
has $\Delta q = (0, 2\pi)$, so it is classified **open**.

## Counterexample / control

`repro/apex_only.stp` face `#370`. Control `repro/plane_control.stp` (46
triangles) — a non-periodic chart, where the Euclidean test happens to be
correct.

## Measurement

```
PROBE piece pts=43 gap=6.283185e0 perimeter=6.283185e0 gap/perimeter=1.000000e0 closed=false
```

The gap is exactly $2\pi$ — *equal to the piece's own uv perimeter*, so no
tolerance can distinguish this from a genuinely open piece. The failure is
categorical, not numerical.

## First divergent checkpoint

**H — periodic lifting and winding** / the closure test at the head of **I**.

## Causal derivation

Arrow 3 of the chain in
[`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md): a physically closed
periodic loop is called open, which delivers exactly one open piece to the
stitching step — the precondition for
[`DOM-ARTIFICIAL-CLOSURE-001`](DOM-ARTIFICIAL-CLOSURE-001.md).

## Correction

**Proposed**: retain a winding vector $h(\gamma) = (k_u, k_v)$ and test closure
modulo the period lattice, with the residual measured in $G$. `QUO-002`.

**Forbidden**: loosening the closure tolerance until $2\pi$ counts as zero. The
gap equals the perimeter, so any tolerance that accepts this accepts everything.

**Experimental / production**: none.

## Tests

None. `quo_euclidean_closure_001_full_period_is_closed_in_quotient` on a
synthetic full-period circle is the direct test and does not need the corpus.

## Known exclusions

Independent of [`PAR-RANGE-INHERITANCE-001`](PAR-RANGE-INHERITANCE-001.md):
correcting the range leaves this test wrong, and correcting this test alone
leaves the stitched loop attached to a range edge that is still arbitrary. They
must both be fixed; neither subsumes the other.

## Claim status

- **(D)** The gap is exactly $2\pi$ and the piece is classified open, on the
  reproducer.
- **(A)** That the same misclassification drives the other apex faces — same
  code path, no per-face trace (`U3`).
- **(U)** `U7`: **no winding number is retained anywhere in the pipeline**, and
  no singular chart is represented as such. The closure test carries no metric,
  so it measures a parameter gap where the contract asks for a physical one.
- **(U)** `U6`: no seam shift or $2\pi$ translation has been run.

## Links

[`FORMALISM.md`](../../../look-collapsed-boundary/FORMALISM.md) §4, U6, U7 ·
`../look-collapsed-boundary/measurements/probe-apex-vs-control.txt` ·
`truck-meshalgo/src/tessellation/triangulation.rs` ·
related [`QUO-LIFT-TIEBREAK-001`](QUO-LIFT-TIEBREAK-001.md)
