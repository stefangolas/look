# WORK PACKET RG-4-CANONICAL-BOOLEAN-PRODUCT — boolean facts must measure the product, not the base operand

AUDIT RG-4 (VERTICAL_GAP, correctness-shaped). The defect is confirmed
at bd_bridge.rs:3107: `top_volume` on a `TreeNode::Boolean` calls
`dispatch_boolean` and then returns `top_volume(&boolean.a)` — the BASE
operand's volume, with the tool's contribution never applied. The same
pattern repeats in the group loop (:3115). So today
`cut(canonical, canonical)` reports V(A) for V(A−B), and every
`fuse(swept, canonical)` row measures only its base. MEASURE FIRST
(reproduce), then fix through the landed machinery — never invent a
volume computation.

```yaml
id:          RG-4-CANONICAL-BOOLEAN-PRODUCT
contract:    [RG-4-CANONICAL-BOOLEAN-PRODUCT]
class:       design
crates:      [truck123d]
depends_on:  []
write_allow:
  - truck123d/src/bd_bridge.rs
  - truck123d/src/facade.rs
  - truck123d/tests/rg4_boolean_product.rs
read_allow:
  - truck123d/src/binding.rs
  - docs/SOLVER_COVERAGE_AUDIT.md
tests_required: [truck123d/tests/rg4_boolean_product.rs]
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'fn top_volume' truck123d/src/bd_bridge.rs"}
  - {id: A2, expect: 2, cmd: "grep -c 'swept_pair_is_admitted' truck123d/src/facade.rs"}
budget:      {turns: 60, ctx_tokens: 180000}
```

## Method

1. **Reproduce first (this is the packet's first deliverable).** A test
   that submits `cut(box A, box B)` with A=[0,2]^3, B=[0.5,1.5]^3 and
   asserts the CURRENT (wrong) behaviour — V(A)=8 reported for V(A−B)=7
   — is a defect demonstration, kept in the test file as
   `#[ignore]`-free RED evidence OR recorded in RESULT notes; then the
   fix flips it to assert 7 within the canonical facts tolerance.
2. **Follow the landed machinery.** `dispatch_boolean` (bd_bridge.rs)
   and `boolean_dispatch` (binding.rs) already compute the canonical
   boolean result — inspect what the binding returns (result-solid
   facts? a verdict string?) and wire the boolean node's `top_volume`
   to the PRODUCT's certified facts. For canonical carriers the
   landed PB-011B path is the volume authority. Do NOT approximate, do
   NOT compute an unceremonied union/subtraction yourself.
3. **Swept operands stay honest:** where the tool operand is swept,
   the product volume is a MONO-8 concern (certified bracket) — this
   packet must make the UNMEASURED case refuse typed naming the open
   carrier (`BooleanProductVolumeUnavailable`) rather than silently
   reporting the base volume. A refusal is the correct interim
   verdict; a wrong number is not.
4. **V5 net:** previously-green rows that measure single parts (no
   booleans) are untouched. The facade path gets the same product
   facts fix if the audit's disjoint-path finding repeats there; keep
   the change minimal and symmetrical.

## Done when

The reproduction test asserts the CORRECT product volume for the
canonical cut and one canonical fuse; the swept-tool case refuses
typed with `BooleanProductVolumeUnavailable`; `cargo check -p
truck123d --lib --locked` green; the existing bridge batteries stay
green. Anchors hold (A1 unchanged, A2 unchanged or grown). Write
RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No `vendor/truck/**` changes. No volume interpolation or tolerance
relaxation to make numbers match. No multi-operand fold (MONO-9). If
the binding's boolean result carries insufficient facts to measure the
product, STOP with QUESTION.md — that is a finding, not something to
work around.
