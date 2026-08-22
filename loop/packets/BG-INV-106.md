# WORK PACKET BG-INV-106 — representation invariant: carriers lie in G

You are implementing one item from a formal kernel specification. Everything you
need is in this document. **Do not read `docs/GENERATION_KERNEL_BUILD_SPEC.md`
or any other spec file** — they are not on your allowlist and this packet is
self-contained. If something you need is genuinely missing, that is a SPEC_GAP
(see "Stop conditions"): you stop and report, you do not research it.

```json
{"id":"BG-INV-106","status":"DONE","contracts":["BG-INV-106"],
 "tests_added":5,"sites_migrated":0,"sites_deferred":0,
 "deviations":[],"disagreements":[],"baseline_failures":[],
 "notes":"free text"}
```

Fill `deviations`, `disagreements` and `baseline_failures` as arrays, empty if
empty. `disagreements` is the highest-value field: **if anything below
contradicts what you find in the code as you work it, say so in
`disagreements` rather than making the code match the packet.**

```yaml
id:          BG-INV-106
contract:    [BG-INV-106]
class:       mechanical
crates:      [truck-topology]
write_allow:
  - vendor/truck/truck-topology/src/invariants/representation.rs
  - vendor/truck/truck-topology/src/invariants/mod.rs
read_allow:
  - vendor/truck/truck-topology/src/invariants/mod.rs
  - vendor/truck/truck-topology/src/invariants/domain_boundary.rs
  - vendor/truck/truck-topology/src/invariants/shell_nesting.rs
  - vendor/truck/truck-base/src/evidence.rs
budget:      {turns: 28, ctx_tokens: 80000}
anchors:
  # Measured under Git Bash on integration HEAD at packet-writing time.
  # A count mismatch is a stop condition (ANCHOR_MISMATCH), not a nuisance.
  - {id: A1, expect: 7, cmd: "grep -c '^pub mod' vendor/truck/truck-topology/src/invariants/mod.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'Representation,' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'NonCanonicalCarrier' vendor/truck/truck-base/src/evidence.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'fn edge_iter' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A5, expect: 1, cmd: "grep -c 'fn face_iter' vendor/truck/truck-topology/src/shell.rs"}
  - {id: A6, expect: 1, cmd: "grep -c 'pub fn curve' vendor/truck/truck-topology/src/edge.rs"}
  - {id: A7, expect: 1, cmd: "grep -c 'pub fn surface' vendor/truck/truck-topology/src/face.rs"}
  - {id: A8, expect: 0, cmd: "grep -c 'pub mod representation' vendor/truck/truck-topology/src/invariants/mod.rs"}
```

## Problem

§1.1 invariant 6 reads: "Carriers lie in G; geometry is certified within
tau_rep of the ideal object." Nothing in `truck-topology/src/invariants/`
checks any of it today: eight landed checkers, none named representation.
The invariant has two halves, and exactly one of them can be certified by
code that exists:

- **Structural half** — every edge's curve carrier and every face's surface
  carrier belongs to the canonical carrier set G. Membership is NOT statically
  decidable in general (the spec is explicit about this); what IS guaranteed
  is that the *classifier* is total. So the checker takes the classifier as an
  injected parameter and certifies the traversal + verdict machinery around
  it. This is the same oracle-injection shape BG-INV-108 already landed for
  `nesting_forest(n, contains)` — copy that house pattern, not a new one.
- **tau_rep half** — geometry within tau_rep of the ideal object. This needs
  rep certificates (BG-FID-005's operator), which do not exist yet. It is
  DEFERRED and documented, exactly as BG-INV-105 deferred its pcurve half.
  Do not fake it, do not stub it, do not emit a certificate claiming it.

The registry's dependency of this item on BG-FID-001 is an ordering edge
(certificate semantics for `Prop::Representation` were settled by the FID
theorem-map work); this packet consumes nothing from `truck_evidence::fid`
at runtime. That absence is expected — do not invent one.

## Decisions already made for you

### Decision 0 — injected total classifier, defined in this module

```rust
/// Total classifier for membership in the canonical carrier set G.
/// Totality is the contract (every input gets an answer), NOT decidability;
/// the concrete impl over real carrier types lands with a later wiring
/// packet, the way BG-INV-108's contains-oracle did.
pub trait CarrierClassifier<C, S> {
    fn classify_curve(&self, c: &C) -> CarrierClass;
    fn classify_surface(&self, s: &S) -> CarrierClass;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierClass {
    /// A member of G: an analytic primitive, or a spline within the declared
    /// degree/span caps, or a degenerate (constant) carrier.
    InG,
    /// Outside G, or a spline beyond its caps.
    OutOfG,
}
```

Exactly two arms. Totality means there is no "undecided" arm: an instance the
classifier cannot decide MUST be classified OutOfG by the classifier's own
implementor (a conservative answer), never by a third state here.

### Decision 1 — a violation is `Contradictory`, per the house checker pattern

Every landed checker signals a violated invariant as
`Err(Refusal::Contradictory(ContradictionWitness { prop: Prop::Representation,
left: Truth::True, right: Truth::False }))`. Use that. `Refusal::
NonCanonicalCarrier` exists in truck-base but belongs to constructor paths,
not checkers; do not reach for it. If you disagree after reading the evidence
module, say so in `disagreements` and proceed with Contradictory.

### Decision 2 — signature and traversal

```rust
pub fn check<P, C, S, K>(shell: &Shell<P, C, S>, classifier: &K) -> Outcome<()>
where K: CarrierClassifier<C, S>
```

Walk EVERY edge through `shell.edge_iter()` and classify `edge.curve()`;
walk EVERY face through `shell.face_iter()` and classify `face.surface()`.
Duplicates (one carrier shared by two faces) are classified twice — that is
fine, classification is pure; do not deduplicate. First offending entity
decides the verdict; localisation is by re-running the classification (same
as the other pure checkers — document this in the module docs).

### Decision 3 — degenerate edges are NOT special-cased

Cone-apex / sphere-pole edges carry constant carriers and are first-class
members of the data model. Whether a constant carrier is in G is the
CLASSIFIER's decision, not this checker's. No branch on endpoint equality.

### Decision 4 — module wiring

`invariants/mod.rs` gains exactly one line: `pub mod representation;` after
the existing declarations (respect rustfmt ordering). The new module carries
`#![deny(clippy::unwrap_used, clippy::expect_used)]` INCLUDING its test
module (GATE-1 gates new modules on unwrap_used; match the stricter pair the
sibling modules use where applicable). Doc-comment header follows
domain_boundary.rs's shape: what v1 certifies, what is deferred and why.

### Decision 5 — tests (all in representation.rs's test module)

Hand-built witnesses over `()` geometry, classifiers as unit structs — copy
coedge_pairing.rs's cube-shell builders verbatim where possible.

1. `representation_canonical_shell_holds` — a cube shell whose classifier
   answers InG for everything: `check` returns Ok, and the certificate's
   prop map sets `Prop::Representation` to `Truth::True`.
2. `representation_out_of_g_face_violates` — a classifier answering OutOfG
   for exactly one surface (build the cube, then flip one face's class via
   the classifier's own state): `Contradictory` witness naming
   `Prop::Representation`.
3. `representation_out_of_g_edge_violates` — same for one curve.
4. `representation_empty_shell_holds_vacuously` — an empty shell passes
   (nothing to classify).
5. `representation_certificate_names_the_invariant` — asserts the prop name
   survives into the certificate on both the Ok path and the Err path.

No floats anywhere in this module — nothing to H-3-escape.

## Done when — run these, all must pass

```
cargo fmt --check -p truck-topology
cargo clippy -p truck-topology --all-targets --no-deps
cargo test -p truck-topology --lib --no-fail-fast
cargo check --workspace --all-targets
bash scripts/kernel-gates.sh <base>        # base = merge-base with integration tip
```

truck-topology is green at baseline (measured this session). Any baseline
failure you did not cause is a stop condition. Send cargo output to a file
and read the tail. Never run a bare `cargo test`.

## Forbidden

Editing files outside `write_allow` (mod.rs gets ONLY the one `pub mod`
line). Implementing a concrete classifier over real carrier types (that is a
later packet; truck-modeling cannot be reached from here anyway). Faking or
stubbing the tau_rep half. Returning a third CarrierClass arm. Using
`Refusal::NonCanonicalCarrier` in the checker. Adding `unwrap()`/`expect()`
on fallible production paths. Committing to `main`.

## Stop conditions

- an anchor count differs → `ANCHOR_MISMATCH`, naming the anchor
- `Face::surface` or the shell iterators turn out unusable for this
  traversal (private, wrong types) → `SPEC_GAP` naming the gap
- three consecutive failed `cargo` runs on the same error → `BLOCKED`

## Finish by writing `RESULT.json` in the root of your worktree

`status` is one of `DONE`, `ANCHOR_MISMATCH`, `SPEC_GAP`, `BLOCKED`. Commit
on the current branch with subject

```
feat(topology): representation-invariant checker, injected carrier classifier (BG-INV-106)
```
