# CC-032-FACE-CONSUMPTION — trim arrangement decides what survives the blend

CC program Phase D (spine S12 consumer; theory §5.4). Do not decide in
advance how much of a support face survives: construct the arrangement of
the original trimming pcurves, the new contact pcurves, and neighbouring
fillets' contact curves; mark the removed cells; F_i_new = F_i \ R_i. Face
consumption is an OUTCOME of the trim arrangement — including the classic
short intermediate face A-B-C, which vanishes when its cell does. No
cascading special-case solver exists.

```yaml
id:          CC-032-FACE-CONSUMPTION
contract:    [CC-032-FACE-CONSUMPTION]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-030-BLEND-SPINE]
write_allow:
  - vendor/truck/truck-certified/src/construct/face_consumption.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_face_consumption.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 24, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub struct BlendTrace' vendor/truck/truck-certified/src/construct/blend.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn arrange' vendor/truck/truck-geometry/src/arrange.rs"}
tests_required:
  - contact_pcurve_splits_the_support_domain
  - short_intermediate_face_is_fully_consumed
  - surviving_face_carries_trim_provenance
  - face_with_no_retained_cell_vanishes
```

Section 1: the arrangement input — `pub struct FaceConsumption { pub
support: SupportDescription, pub contact_pcurves: Vec<ContactPcurve>,
pub trim_provenance: Vec<SourceRef> }` where `ContactPcurve` is the
parameter-space projection of a certified contact curve q_i(s) from the
`BlendTrace` (A1) onto the support's chart, and `SourceRef` rides the
landed provenance ids. `pub fn consume_face(fc: &FaceConsumption) ->
Result<FaceOutcome, ConstructRefusal>` with `pub enum FaceOutcome {
Survived { retained: Vec<RetainedCell> }, Vanished }`. The 2-D arrangement
of pcurves routes through the LANDED arrangement machinery (A2's `arrange`
for the curve work; the cell classification here marks, on each cell,
whether it is inside the removed region R_i — the blend side — via the
contact curve's side signs carried by `BlendTrace`). Pre-made: cells are
classified by interval evaluation of the signed side data at the cell box;
an undecided cell → `Err` of the underlying refusal family, never a guessed
label.

Section 2: the A-B-C ground truth (test 2): three pairwise supports where
the centre path reaches the A/B/C triple node and departs — the
intermediate face B retains NO cell → `Vanished`, with no special-case
code: the fixture drives the ordinary arrangement and the empty-retained-
cell branch falls out. Test 1 pins the ordinary case: one contact pcurve
splits the domain into exactly the two expected cells (H-3 opt-outs).
Test 3: a surviving cell records its trim provenance (which contact curve,
which side) — the edit-graph requirement from the theory's output section.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_face_consumption`. No workspace builds. The `pub mod
face_consumption;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the concave-edge trim of the sharp offset variant
reuses this module — CC-024 owns the sharp-side entry; here only the
blend-side classification lands; (2) `BlendTrace` is CC-030's output — if
the contact pcurve projection needs data the trace does not carry, file
QUESTION.md (spine S12 refinement), do not re-trace branches here; (3) the
arrangement stage's Σ-determines-topology obligation (theory §10.1) is
carried by this packet's determinism: identical traces → identical cell
labels → identical B-rep combinatorics. State that in the module doc.
