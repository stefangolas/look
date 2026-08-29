# Build123d coverage program (Phase 7)

Status: **approved design, not yet dispatched.** Written 2026-08-28 as the
program amendment for the solver family's next phase. This doc books the
packet graph, the API additions, and the anchor claims; per-packet detail
belongs in `docs/GENERATION_KERNEL_BUILD_SPEC.md` extensions and
`loop/packets/`. Every tree claim below was verified by command on
2026-08-28 at `integration/kernel-bg` HEAD `973234d`; re-derive before quoting
in a packet (plan §3 rule).

## 1. Framing: build123d is a semantic conformance target, not a component

The kernel grows a **Rust facade** — build123d-shaped operations
(`extrude`, `revolve`, `fillet`, `chamfer`, `mirror`, `scale`, `section`,
`split`, `bounding_box`, ...) that are a naming + semantics table over landed
kernel machinery. The facade carries **zero geometric content of its own**;
every operation either composes landed primitives or refuses with a typed
`Refusal`.

The public API contract follows build123d's ordinary signatures *semantically*:
no restricted alternative names (`revolve_analytic`, `fillet_planar`, ...);
unsupported geometric cases refuse rather than approximate or fall back.
No silent alternate-kernel fallback; provenance stays visible.

The **Python binding is deferred and booked as a separate mechanical program**
(pyo3 translation of the stabilized facade; refusal → typed exception). It has
no geometric content and must never appear inside a kernel packet. Until it
lands, "build123d coverage" means semantics conformance exercised by Rust
tests written against build123d's documented behavior.

## 2. The parsimony lemma

The program adds **no new continuous solver mathematics**. Everything is a
composition of four proof-bearing primitives plus one normalization rule:

$$
\boxed{
\begin{aligned}
&\textbf{Similarity fold:} && g(\mathcal{G}) \subseteq \mathcal{G} \\
&\textbf{Offset fold:} && \mathcal{O}_r(\mathcal{G}_{\text{supported}}) \subseteq \mathcal{G} \\
&\textbf{Sweep reduction:} && Q_d(X,T) = \mathrm{Contact}(\mathrm{Extrude}(X,d),\, T) \\
&\textbf{Topology:} && \text{all feature surgery} \rightarrow \text{LocalBoundaryRewrite}
\end{aligned}}
$$

**Similarity fold** — the canonical carrier set $\mathcal{G}$ is closed under
rigid motions, reflections, and uniform positive scale (a similarity-group
statement, deliberately *not* general affine: a general affine map sends
sphere → ellipsoid and leaves the set). Each carrier's parameter map is exact:

```text
Plane:    n → g⁻ᵀn (renormalized), d folded     Sphere:   c → gc, r → s·r
Cylinder: axis point/dir → g, r → s·r           Cone:     apex/axis → g, half-angle invariant
```

**Offset fold** — signed normal offset is closed on the supported carriers:

```text
Plane → Plane (d shifted)          Sphere → Sphere (r ± d)
Cylinder → Cylinder (same axis, r ± d)
Cone → Cone (same half-angle, apex shifted along axis)
Torus → Torus(R, r ± d)            # major radius R is INVARIANT; only the tube radius shifts
```

(Torus correction recorded: `S(u,v) + dN = Torus(R, r±d)` — "both radii ±d"
was wrong and is not to be reintroduced.)

Regularity semantics, precisely worded: `|d| <
FaceScaleComponents::curvature_radius_lower` certifies **avoidance of the
local focal singularity only** (`1 − dκ = 0` family). It is NOT a tube/reach
theorem — global self-contact/clearance is a separate question answered by
Contact. Two existing mechanisms, two different questions; S8's
"not a reach theorem until the bridge lemma exists" discipline is preserved.

**Sweep reduction** — directional queries are Contact on the swept stratum:

$$
Q_d(X,T) = \mathrm{Contact}(\mathrm{Extrude}(X,d),\, T)
$$

where the dimension of the source `X` (vertex / edge / face) determines the
dimensionality of the swept stratum (ray-FE / curtain-FF / shell). There is no
`DirectionalHitComplex` type: the contact records, ordered by their certified
extrusion parameter `t`, ARE the hit complex. `Until.NEXT/LAST/FIRST/PREVIOUS`
is pure ordering/filtering afterward.

**LocalBoundaryRewrite** — the one genuinely new type: the booked Boundary
Rewrite transaction boundary (`B_in → W_scratch → B_out`) applied to a face
neighborhood. Reuses `FragmentMesh`, `Same`/`Flip` adjacency parity, and
`Solid::try_new` as the acceptance gate. All feature surgery (chamfer, fillet,
later draft, eventual shelling) is expressed through it.

**Frame conjugation is a Contact normalization rule, not a fifth primitive.**
For a stratum pair `(F₁@g₁, F₂@g₂)` compute the relative frame
`h = g₁⁻¹·g₂`; intersection depends on relative pose only. Dispatch:

```text
recognize pair
    ↓
cheap relative-frame canonicalization (h)
    ↓
conjugated analytic fast path if h lands the pair in a supported
    relative configuration (coaxial / parallel / tangent-plane / ...)
    ↓
existing analytic/general contact dispatch (unchanged)
    ↓
defer only if NEITHER applies   (ContactReductionDeferred, unchanged deferral)
```

Conjugation must never become a gate IN FRONT of an already-supported solver.
Payoff on day one: the landed-but-unreachable `equal_radius_cylinders`
intersecting-axes cell (verified unwired — `contact/mod.rs` imports only
`coaxial` + `parallel_cylinders`) becomes reachable for the first time,
because canonical z-aligned carriers are always parallel-axis today.

## 3. Architecture

```text
CONTINUOUS GEOMETRY                     DISCRETE TOPOLOGY
  similarity / offset folds               ContactRecords
        ↓                                   ↓ select / cluster / order
     Contact                                ↓ LocalBoundaryRewrite
        ↓                                   ↓
  existing witnesses                    Solid::try_new
```

The original coverage proposal's four intermediate types decompose as:

```text
SectionComplex        ≈ selected ContactRecords + clustered incidence
DirectionalHitComplex ≈ ContactRecords carrying an extrusion parameter
OffsetContactComplex  ≈ ContactRecords whose inputs are signed offsets
LocalBoundaryRewrite  ≠ redundant — the one persistent new type
```

Only the last deserves an architectural type; the first three are views over
the Contact Layer (`contact()`, `ContactRecord`, `ContactLocus` — landed) and
must not be introduced as parallel vocabularies. In particular do NOT define a
second `ContactLocus`.

## 4. Operation decomposition

```text
section       = Contact
split         = Contact + implicit-sign classify + caps(make_face) + rewrite
until/project = swept Contact + certified-t ordering + rewrite
fillet spine  = contact(offset(F₁,r), offset(F₂,r))
fillet check  = Contact against unselected strata (global clearance)
fillet        = offset + Contact + realization table + rewrite
chamfer       = closed-form trim-loci replacement + rewrite
revolve       = closed carrier realization + sew   (line edges: Tier 0; circles: Tier 2)
mirror/scale  = similarity fold
make_hull     = 2-D convex hull + wire + make_face (facade-level)
bounding_box  = Box3 from EnclosureSurface::enclose
mode ADD/SUBTRACT/INTERSECT = landed BoolOp; REPLACE/PRIVATE = builder state (facade)
```

## 5. Verified anchors (2026-08-28, HEAD `973234d`)

Re-derive each before quoting in a packet.

- `Refusal` / `EnvelopeCase` / `UnresolvedWitness`:
  `truck-base/src/evidence.rs:51/84/101`. Present arms used by this program:
  `ReachTooSmall` (:88), `NonCanonicalCarrier` (:90),
  `ContactReductionDeferred` (:96), `Collapsed` (:70), `Contradictory` (:68),
  `NumericallyUnresolved` (:57).
- The recognizer ALREADY represents placements:
  `CanonicalSurface::Placed(Processor<Box<CanonicalSurface>, Matrix4>)`
  (`truck-geometry/src/recognize.rs:60`), with the bare carriers documented
  z-axes-only (:56-59). Tier 1 is funnel-side work, not a representation gap.
- `ExtrudedCurve` → carrier recognition already landed:
  line-extrusion → `Plane` (`recognize.rs:180-200`); the module header books
  line/circle extrusion recognition (`recognize.rs:12-13`). The sweep
  reduction's curtain strata are therefore already recognizable —
  `ExtrudedCurve` *emission* is not on the critical path.
- `RevolutedCurve` is currently `Unrecognized` (`recognize.rs:151-155`), whose
  comment already books "revolve recognition lands with S2's
  `revolve_profile`" — which does not exist yet. The revolve packet owns it.
- `equal_radius_cylinders` landed and unwired:
  `truck-evidence/src/analytic/equal_radius_cylinders.rs:60`; dispatcher
  imports at `contact/mod.rs:40-41` are `coaxial` + `parallel_cylinders`
  only. The coaxial/parallel dispatch with `coaxial_axes` sits at
  `contact/mod.rs:383-479`.
- Landed analytic cells the curtain table reuses: `plane_plane`
  (`analytic/plane_plane.rs:39`), `plane_cylinder` (:110), `plane_cone`
  (:117).
- Landed entries: `contact()` (`contact/mod.rs:184`), `boolean()`
  (`truck-shapeops/src/boolean/assemble.rs:56`, single-shell guards :64),
  `extrude_profile` (`truck-modeling/src/extrude.rs:52` — scalar +z,
  height > 0, one material region; this program generalizes it).
- Substrate: `ImplicitField` (`contact/implicit.rs:51`; torus quartic per
  plan §4 SING-SUBSTRATE), `FaceScaleComponents` (`fid/lfs.rs:73`),
  `curvature_radius_lower` (:147), `Offset` decorator with
  `EnclosureSurface` impl (`decorators/offset.rs:257`),
  `num::cluster` (`num/cluster.rs:119` — endpoint pairing).
- `Surface::RevolutedCurve` / `ExtrudedCurve` variants exist
  (`truck-geometry/src/canonical.rs:258/262`); the span cache treats them as
  non-canonical (`span.rs:20`).

## 6. Closure / realization tables

Each row is a construction formula + certificate; off-table refuses.

### 6.1 Extrusion curtain (feeds the sweep reduction)

```text
line edge    × dir → Plane      (plane_plane cell covers target contact)
circle edge  × dir → Cylinder   (framed at Tier 1; plane_cylinder covers)
circle+taper × dir → Cone       (plane_cone covers)
```

For analytic profile edges the curtain is canonical without emitting
`ExtrudedCurve`. Non-analytic profile edges refuse upstream as today.

### 6.2 Revolve, line edges (Tier 0 — carriers stay in the landed FF funnel)

```text
line ∥ axis             → Cylinder
line intersecting axis  → Cone
radial perpendicular    → Plane
degenerate axis contact → collapsed edge becomes vertex; certify or refuse
```

This scoping is what decouples revolve from the torus funnel.

### 6.3 Revolve, circle edges (Tier 2 — the recognition table, not one row)

```text
axis external to circle      → Torus
axis through circle center   → Sphere
axis tangent to circle       → horn — singular topology event, typed refusal
axis meets the disk interior → spindle/self-overlap — certify or refuse
```

### 6.4 Blend realization (constant-radius rolling-ball: envelope of radius-r
spheres centered on the spine)

```text
center locus Line   → Cylinder
center locus Circle → Torus (constant-frame case)
center locus Point  → Sphere
anything else       → general canal surface — DEFERRED (Tier 2 boundary)
```

F2 scope is therefore "canonical-output curved fillets whose spine realizes
to Cylinder/Torus/Sphere", never "all curved face pairs". The
downstream-consumability invariant holds by construction.

### 6.5 F4 three-plane corner

Triple offset intersection gives center `p`; corner patch = `Sphere(p, r)`.
`p` lies on each pairwise bisector line, so each incoming cylindrical edge
blend is coaxial with the corner sphere at the junction — the junction is
continuity-friendly without solving the broad F2 problem. F1 + F4-three-plane
ship together, ahead of F2/F3.

## 7. Refusal mapping — zero new arms (verified)

Every refusal this program needs maps onto the landed algebra
(`evidence.rs:51-97`); **no `Refusal`/`EnvelopeCase`/`UnresolvedWitness` arm
is added**:

```text
noncanonical carrier or target        → UnsupportedEnvelope(NonCanonicalCarrier)
oversized fillet / offset focal / reach → UnsupportedEnvelope(ReachTooSmall)
hole collapse / zero-area top / horn  → Collapsed(Collapse, Certificate)
adjacency parity contradiction        → Contradictory(ContradictionWitness)
budget exhaustion                     → NumericallyUnresolved { .. }
deferred pair (torus, skew cylinders) → UnsupportedEnvelope(ContactReductionDeferred)
```

If a packet believes it needs a new arm, that is a SPEC_GAP, not an edit.

## 8. Packet graph (compressed)

Packet = unit of work sized so the worker stays within ~50% of its context
window. Compression rule: merge only along same-module dependency chains
(they could never run in parallel anyway); never merge across crates or
across parallel branches (write-set disjointness is the scheduler's law).

### Tier 0 — no funnel change, no new continuous math (~12–14k LOC)

- **P1 — utility + planar face construction.** `bounding_box`,
  similarity-fold-lite (translate + uniform scale + axis-aligned mirror as
  parameter folds), `project_workplane` (affine curve map; line/circle →
  line/ellipse), `make_face` from arrangement, `make_hull` (2-D hull → wire →
  face). New modeling-utility module. Difficulty 2/10.
- **P2 — generalized extrude.** Vector form of `extrude_profile`, `both`
  (interval extrusion), `mode` (facade over `BoolOp`), `taper` (curtain table
  6.1; `Collapsed` for hole collapse / zero-area top / side self-overlap).
  Difficulty 3/10.
- **P3 — section + split by plane.** Contact lift (plane stratum × face
  strata) + `num::cluster` endpoint pairing + wire assembly; split = section
  + certified implicit-sign fragment classification + caps via P1 + the
  assembler pattern. One module. Difficulty 4/10.
- **P4 — sweep reduction: until + project.** Curtain realization table 6.1,
  `contact()` on curtain × target strata, certified-t ordering (`Until.*`),
  cap-patch extraction + rewrite at termination curves; `project` = locus
  extraction from the same records. Vertex sources keep the FE/ray path.
  Difficulty 5/10.
- **P5 — revolve, line-edge profiles.** Table 6.2 + `RevolutedCurve`
  recognition wiring (owned per `recognize.rs:151-155`) + seam/partial/full
  handling + axis-contact cases. Difficulty 5/10.
- **P6 — LocalBoundaryRewrite + chamfer PP.** The design packet: neighborhood
  rewrite engine (trim, replace, rewire, delete consumed topology, orient,
  sew, `Solid::try_new` gate), proven on plane-plane chamfer (symmetric,
  distance-distance, distance-angle; independent edges and simple chains).
  MANDATORY pre-dispatch num3-scratch probe. Difficulty 5/10.
- **P7 — fillet F1 + F4 three-plane corner.** Offset fold + spine contact +
  realization 6.4 (cylinder/sphere rows) + clearance = Contact + rewrite via
  P6. Curved pairs and non-circular spines refuse
  `ContactReductionDeferred`/canal-deferred. Difficulty 6/10.
- **P8 — facade + integration battery.** The build123d-shaped facade over
  P1–P7 plus the conformance battery: constructive sequences, metamorphic
  algebra (§9), adversarial/refusal cases for every feature, and
  downstream-consumability checks (Boolean/section/selector/tessellation on
  every generated surface). Difficulty 5/10.

### Tier 1 — frames by conjugation (~2–3k LOC)

- **P9 — conjugation normalization.** Relative-frame canonicalization in the
  Contact dispatch (rule §2), admitting `Placed` carriers that conjugate to
  supported relative configurations; makes `equal_radius_cylinders`
  reachable. Touches `contact/mod.rs` — hot file; serializes against other
  contact work. Difficulty 4/10.
- **P10 — framed emission + general transforms.** Similarity folds emit
  `Placed` carriers (representation already exists, `recognize.rs:60`);
  general mirror/rotate, oblique `dir` extrusion of circle profiles, revolve
  about arbitrary axis (line edges). Metamorphic gate:
  `T(A op B) = T(A) op T(B)`. Difficulty 4/10.

### Tier 2 — torus funnel + canonical-output blends (own program, ~5k LOC)

- **P11 — torus FF pairs.** Implicit-field path (torus quartic +
  Hessian already landed) + certified arc continuation; the one genuinely new
  solver-math packet family in the program. Difficulty 7–8/10; expect the
  Krawczyk conditioning round trips the sphere/cylinder pairs needed.
- **P12 — canonical-output curved fillets (F2) + chains (F3).** Spine
  realization 6.4 extended to Torus; F3 chains via offset-complex incidence +
  the Boolean splitter's `Same`/`Flip` constraint-graph reasoning; revolve
  circle edges (table 6.3) unlocked as a byproduct. Difficulty 7/10.

Deferred unchanged (derived from the closure table, not asserted): general
sweep, general loft, offset/thicken/shell, post-hoc draft, general canal
surfaces, topology-changing/face-consuming fillets, `ExtrudedCurve` emission.

Deferred with a full instrumented diagnosis (session 41, three stops):
**vertex-touch cuts** — a cut plane through the solid's edge graph (P3's
diagonal fixture, plane x + y = 2 through opposite box edges). The typed
refusal is the booked v1 boundary (same class as the RW-CONIC Ellipse and
the Region2 `Crossing` screen); landing it requires four kernel decisions
the v1 envelope does not make: (1) canonical-vertex unification so
corner-endpoint arcs splice (`add_edge` needs instance-equal vertices),
(2) seam-edge replacement of contact-plane-coincident boundary edges with
arc instances (Flip parity + shared instances, direction-matched),
(3) per-face arc certification in the open-arc path (conflicts with the
sew-completion corner-touch skip — needs a joint redesign), and
(4) Region2 handling of coplanar-adjacent (edge-sharing, non-overlapping)
regions, which the `Crossing` screen deliberately refuses. Evidence chain:
`loop/results/RW-VERTEX-CLIP.STOP-r1.json`, `.STOP-r2.json`,
`RW-SEED-DIAGONAL.STOP.json`.

Dependency sketch:

```text
P1 ─┬─ P2 ─┬─ P4 ──┐
    ├─ P3 ─┘       ├─ P8 (facade + battery)
    └─ P5 ─────────┘
P6 ── P7 ──────────┘        (P6 has a mandatory probe)
P9 ── P10                   (Tier 1; P9 touches the hot contact dispatcher)
P11 ── P12                  (Tier 2, own program)
```

P1–P5 are mutually write-disjoint (new modules) and may run as a parallel
wave, disk permitting (~2–3 GB/slot; verifies sequential).

## 9. Test algebra (metamorphic, per packet + battery)

```text
T(A op B) = T(A) op T(B)                for similarity g        (P10's gate)
contact(A,B) ≅ contact(g·A, g·B)         under exact conjugation (P9's gate)
split(S,Π)₊ ∪ split(S,Π)₋ ≅ S            transversal             (P3)
extrudeUntil(P, Π) ≅ extrude(P, h_Π)     parallel target         (P4)
revolve(line polygon) ≅ analytic primitive                        (P5)
fillet round trip: offset adjacent faces back by r
                  reconstructs the original neighborhood          (P6/P7)
chamfer of the two trim loci reconstructs the original edge       (P6)
A∪A=A, A−A=∅, A∪B≅B∪A                    landed, reasserted via facade (P8)
```

Every hand-derived witness in a packet is machine-checked (BG-NUM-002 rule,
including negative witnesses and f64 representability).

## 10. Estimates

| Tier | Packets | LOC (code+tests) | Sessions |
|---|---|---|---|
| Tier 0 (P1–P8) | 8 | ~12–14k | 5–7 |
| Tier 1 (P9–P10) | 2 | ~2–3k | 1–2 |
| Tier 2 (P11–P12) | 2–3 | ~5k | 3–5 |
| **Total** | **12–13** | **~20k** | **~9–14** |

Versus the fine-grained graph (21–28 packets, 13–17 sessions): compression
saves ~10 worker cold-starts and ~10 verify round trips; LOC is invariant.

## 11. Execution rules

- **Worker model: `zai/glm-5.3-flash`** (switched from
  `deepseek/deepseek-v4-flash`; `run_packet.py`'s `--model` default, 2026-08-28).
  The verifier is model-independent — `verify.py` reads the diff, not the
  claim — so the swap risks schedule, not correctness. Provenance:
  `run_packet.py` now stamps `worker.model` per slot and `land_packet.py`
  records it in each ledger row (no more hardcoded model), so per-packet
  model + round-trip rate stays derivable from the ledger. Calibration:
  re-rate P3–P7 (the ~1.5–2.5k LOC packets) against the new model's context;
  treat the first packet's `RESULT.json` deviations as the recalibration
  data point.
- **Do not relax the pre-decide rule for the stronger model.** Every
  judgement pre-made in the packet; at most one named judgement left to the
  worker with reasoning required in notes. The recorded failure ("a worker
  designing unsupervised inside a write set went badly") is not waived.
- **P6 runs its num3-scratch probe before dispatch**; P9's dispatcher change
  gets a probe on the conjugation precondition's refusal boundaries. Scratch
  targets deleted at design-session end (disk rules).
- Each packet ships its own battery (§9 rows it owns); P8 folds them into the
  integration battery as regressions.
- Packets quote §5 anchors only after re-deriving them by command. The
  zero-new-refusal-arms claim (§7) is re-verified at P6 and P9 dispatch.
- Renaming a pre-existing passing test is a regression (session 34 rule);
  packets that supersede refusal contracts keep test identities.
