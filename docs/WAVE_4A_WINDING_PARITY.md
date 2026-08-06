# WAVE-4A — `ContradictoryDualParity` is a winding-number bug

Package A of the three-session plan. The handoff sized it at 1,422 faces and
called it "one line". The one line was wrong, and finding out why took about an
hour and produced the actual mechanism, which is also small.

---

## What the handoff predicted, and what happened

The standing hypothesis, deferred across two waves:

> `toggles_material` returns `Some(true)` for
> `ConstraintRole::UnresolvedSyntheticClosure`, generating `μ_L = 1, μ_R = 0`.
> An artificial cut should generate `μ_L = μ_R` (§X Definition 20, second
> bullet).

**Measured, and falsified.** Read as non-toggling, all 126 contradicting faces
on `00009190` still contradict — the retry fires 126 times and resolves zero.
It is not a near miss: the obstruction count *rises* under the artificial-cut
reading (see below), so the synthesised segments sit inside a properly closed
cycle and the contradiction is somewhere else entirely.

The `Some(true)` is left in place, with the measurement recorded beside it.
Changing it on the strength of the definition alone would have been a change
that recovers nothing and moves an obstruction in the wrong direction.

## The measurement that found the mechanism

A parity flood over a planar CDT is self-consistent **iff every vertex has an
even number of incident toggling constraint edges**. Walking the faces around
one vertex returns to where it started, so parity closes there only if the walk
crosses an even number of toggling edges; one odd vertex anywhere makes the
flood contradict itself regardless of visit order.

That turns "contradictory parity" from a symptom into a located, countable
obstruction — and it separates *"some role's material reading is wrong"* from
*"the constraint set is not a closed boundary at all"*, which no count of
failures can distinguish. On `00009190`'s 126 faces:

```
  82   odd_legacy=2    odd_artificial_cut=4
  31   odd_legacy=4    odd_artificial_cut=4
   6   odd_legacy=6    odd_artificial_cut=4
   7   odd_legacy=18..48 (the large multi-bound faces)
```

Nonzero under both readings, so neither reading is the issue. And the majority
sit at exactly **2** — the toggling subgraph is a path, not a cycle: one single
edge missing from an otherwise closed boundary.

## The mechanism

`ConstraintRoles::roles` is a **set**. When two boundary segments of one face
realize onto the same CDT edge, the second `record` is a no-op — first claim
wins, which is right for the *role* — and the second traversal leaves no trace
at all. But material parity is the boundary's winding number mod 2, so an edge
the boundary crossed **twice** separates nothing, while one realized edge in a
set toggles once.

That is the whole cell. Counting traversals rather than claims:

| | faces | with a repeated traversal |
|---|---:|---:|
| flood clean (`00009190`) | 23,258 | **0** |
| `ContradictoryDualParity` | 126 | **126** |

Perfect separation, both directions, no exceptions.

The existing `overlapping` guard in `insert_to` cannot see it: it inspects only
the direct edge `(vi, vj)` and not the rest of the chain Spade realized, and
`try_add_constraint` returns a *chain* whenever an existing vertex lies on the
requested segment.

## The fix

`ConstraintRoles` gains a `traversals` count beside the role set.
`ParityReading::TraversalParity` makes a toggling role toggle only on an odd
number of crossings. Two traversals cancel whether they run the same way (a
doubled segment) or opposite ways (a slit) — in both cases the edge genuinely
separates nothing.

Shipped behind `TRUCK_FORMAL_RECOVERY_PARITY`, default-on, nested under the
master gate.

## The trap: where the retry runs decides whether it is a regression

Run as a branch inside the first tessellation — the obvious place, where the
contradiction is detected — the retry recovers the same 126 faces on
`00009190`, `rendered -> lost = 0`, and looks clean. It is not:

```
  #58586 torus  64 -> 2 triangles
  #27220 torus  64 -> 2 triangles
  ... 8 faces, all torus, all 64 -> 1..2 triangles
```

Those 8 were already being recovered by the torus annulus route with a
validated 64-triangle mesh. Succeeding early meant the face had a mesh, so the
route that had been rescuing it never ran. `rendered -> lost` stayed empty
because the faces still rendered — as a 1–2 triangle remnant of an annulus.

**This is exactly what the handoff's "watch triangles per face, not just
rendered/lost" warning is for**, and it fired on the first measurement.

The cause is not a flaw in the winding rule; the rule is right about those
faces. All 8 are `two_outer_bounds_on_certified_torus_annulus`: the source
declares the whole bound twice, so *every* edge is traversed twice and the
winding reading correctly cancels the entire boundary. The face is not a slit,
it is malformed, and the repair belongs to the route that knows that.

So the retry runs **last** — after the planar slice, the cylinder slice, the
band, the cone and the torus routes, on a face that all of them have already
declined. It is a second whole tessellation of one face rather than a branch
inside the first, which is why the reading travels as a thread-local
(`PARITY_READING`) rather than a parameter. That costs one re-tessellation on
the ~1,400 faces in this cell and buys a refinement-only property that holds
against the whole pipeline instead of against one function.

With the retry placed last, on `00009190`: **118 recovered, 0 lost, 0 triangle
counts changed on any already-rendered face.** 118 is the model's entire
`ContradictoryDualParity` population — cylinder 93, torus 18, cone 6, sphere 1.

## Corpus result

**+1,404 faces, 0 regressions, 0 triangle-count changes**, over all 20 models,
each measured as one subtraction against `TRUCK_FORMAL_RECOVERY_PARITY=0`.

```
model           faces  lost(off)  recovered      model           faces  lost(off)  recovered
00007705        22076       2602        413      00006483        23049        462         55
00005760        43986       1363        228      00003902        26045        828         27
00001075        30276       1458        139      00007744        12030        551         26
00009190        24202        935        118      00008001        12030        551         26
00007667         7713        688        100      00005586         2280         87         10
00000959        10298        333         97      00001116         1674         68          3
00000730        30302       1396         67      00000414        19187       1733          0
00003172        22971       1537         59      00005427        15412        723          0
00005641       179656        297         36      00005642 / 00009272 / 00009972   (0 in cell)
                                                 TOTAL          15,620 lost   +1,404
```

By family: cylinder 1,180, torus 122, cone 81, sphere 11, extruded 6, nurbs 2,
revolved 2. 146,802 triangles added.

**This is essentially the whole declared cell.** The handoff sized
`ContradictoryDualParity` at 1,422 faces with cylinder 1,180 / torus 122 /
cone 81; those three numbers are recovered *exactly*. The 18-face shortfall is
in the sphere row (11 of 28) and is the only part of the cell that survives.

Corpus loss goes **15,620 -> 14,216**; rendered 823,559 -> 824,963 of 839,179
(98.14% -> 98.31%). `ContradictoryDualParity` drops from 9.1% of the residual
to roughly 0.1%.

This was the one package in the plan expected to be cheap and to fail. It was
cheap and it worked — but not for the reason it was scoped for, and the
scoped reason is now measured dead rather than still deferred.

## Reproduce

```bash
# built at the pin; the sweep alternates configs per model
p1-out/parity-sweep.sh
python p1-out/parityA/tab.py p1-out/parityA/corpus

# the obstruction count, per face, on one model
TRUCK_PROBE_PARITY=1 face_census.exe abc/00009190/*.step 2>probe.txt >/dev/null
```

`TRUCK_PROBE_PARITY` emits one `PARITY` line per face reaching the flood, with
`repeated_traversals`, the odd-vertex count under both readings, and the
outcome. It is the measurement this document is built on and it is cheap —
`odd_toggling_vertices` is one pass over the constraint edges.
