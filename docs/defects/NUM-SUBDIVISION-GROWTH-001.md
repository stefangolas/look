# NUM-SUBDIVISION-GROWTH-001 — Sample count from imported geometry, unbounded

**Family** `NUM` · **Manifestation** `NONTERMINATION` → `MISATTRIBUTION`
**Contracts** discharges `RES-001`, `RES-004`. **Violates `RES-003`.**

## 1. Status

```
Correction experimentally validated  — the abort is gone, measured
Not closed                           — the cap lies, per RES-003
```

## 2. Mathematical objects

The angular sample count for tessellating a `RevolutedCurve`:

$$N = 1 + \left\lfloor \frac{v_1 - v_0}{\arccos\!\bigl(1 - \tau/r_{\max}\bigr)} \right\rfloor$$

with $\tau$ the chord tolerance, $r_{\max}$ the revolved radius, and
$[v_0, v_1]$ the angular range. $N$ is then used as an **allocation size**.

## 3. Required obligation

Any count derived from imported (untrusted) geometry and used to allocate must
be bounded and total:

$$N \in [1, N_{\max}],\quad N \text{ finite for all inputs, including } r = 0,\ \tau \ge r_{\max},\ v_1 - v_0 \gg 2\pi.$$

`RES-001` (checked `SampleCount` constructor), `RES-004` (total work bound).
And `RES-003`: **a result that hit the cap must be reported as
`ResourceCapped { requested, used, achieved_error }` and must not claim the
tolerance it was asked for.**

## 4. What the implementation did

`RevolutedCurve::parameter_division` computed $N$ with **no ceiling**, and the
expression is reached with untrusted numbers from both directions:

- $\arccos$ collapses toward zero as the revolved radius grows against the
  tolerance, so $N \to \infty$;
- $v_1 - v_0$ is the bounding box of a **lifted boundary**, which a bad lift can
  make span many periods;
- a degenerate radius makes the $\arccos$ argument infinite and the result NaN,
  which the old `as usize` cast turned into a division by zero.

`sub_parameter_division` already had `MAX_DIVISION_CELLS` for exactly this
reason; **this specialised path bypassed it.**

## 5. Minimal counterexample

ABC `00000730`: a request for **6,638,692,106,004,871,184 bytes**. Four
synthetic cases in `parameter_division_bounds` — each one aborted the process
before the cap existed, so the assertions matter less than the tests returning
at all.

## 6. Control / oracle

Any ordinary revolved face on the same model, where $N$ lands in the tens.

## 7. Measurements

`00000730` renders with the residual gate off at **425,328 triangles** —
*better* than the 423,170 it produced with `GEO-INCIDENCE-ACCEPTANCE-001`'s
gate masking the fault by rejecting the offending faces before tessellation.

Cap: `MAX_CIRCLE_DIVISION = 4096`, a chord error of about $2.3\times10^{-7}$ of
the radius — finer than any tolerance reaching this code. Non-finite requests
fall back to a usable division.

## 8. First divergent checkpoint

**K/L — sampling for triangulation.** Note the *input* to the divergence can be
manufactured at **H (periodic lifting)**: a bad lift widens $v_1 - v_0$ and this
code trusts it. The bound is the right fix at K regardless, because the input is
untrusted by definition.

## 9. Causal derivation

```
N derived from imported radius, tolerance, and a lifted angular range
→ no ceiling, and a NaN path through `as usize`
→ N reaches ~10^18
→ allocation of 6.6 exabytes
→ abort, with no attribution to a face or an entity
```

## 10. Proposed correction

`RES-001`'s checked `SampleCount` constructor, which retires the whole class.

## 11. Experimental correction

None; the cap went straight in.

## 12. Production correction

`stefangolas/truck` `23207c20` *"Bound a sample count derived from imported
geometry"* — `truck-geometry/src/decorators/revolved_curve.rs`. Predecessors
`c2eb36b3`, `d7de2303` bound the grid and the per-curve division.

> **The cap is safe but it lies.** A face capped at `MAX_CIRCLE_DIVISION`
> returns its approximation as **ordinary success**, so a mesh that could not
> reach the requested tolerance is indistinguishable from one that did.
> `MAX_DIVISION_CELLS` has the same defect and predates this work. This is
> exactly the failure mode the architecture exists to prevent: a plausible
> answer where an honest refusal belongs. `RES-003` is violated by the fix
> itself, and that is why this record is not `Closed`.

## 13. Regression tests

Four in `parameter_division_bounds` (`truck-geometry`). Not ID-named. The
missing fourth-kind test is the corpus assertion that no model in either corpus
silently hits the cap — **which cannot be written until `ResourceCapped` exists,
because today hitting the cap is unobservable.**

## 14. Corpus-wide effect

`00000730` goes from abort to 425,328 triangles. The other five swept models are
unaffected. **How many faces across the corpus are silently capped is unknown
and unmeasurable today.**

## 15. Known exclusions

Two caps are known instances of this shape; **there is no reason to assume they
are the last.** Any count derived from imported geometry and used as an
allocation size is this bug. That is a search instruction, not a closed list.

## 16. Relationship to other defects

Discovered only because [`GEO-INCIDENCE-ACCEPTANCE-001`](GEO-INCIDENCE-ACCEPTANCE-001.md)'s
gate was turned off — the gate had been silently rejecting the offending faces.
See that record §16.

Also: **a crash is a measurement result.** The corpus sweep runs six models, not
one, because `00009190` was clean while `00000730` aborted outright on the same
build.

## 17. Claim status

- **(D)** The abort, the allocation size, the four aborting test cases, and the
  post-fix triangle count.
- **(D)** The capped result is reported as ordinary success — code reading plus
  the absence of any `ResourceCapped` type.
- **(U)** The population of faces currently capped, corpus-wide. Unmeasurable
  until `RES-003` is implemented.

## 18. Links

- `truck` `23207c20`, `c2eb36b3`, `d7de2303`
- [`PLAN.md` § Unbounded sample counts](../../PLAN.md)
- `MATHEMATICAL_FOUNDATION.md` `RES-001`–`RES-006`, §33a item 5 (next work)
