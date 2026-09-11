# WORK PACKET RDEF-M3-WITNESS-TIER — exact degeneracy classification, 2x2

Milestone M3 of `docs/RANK_DEFICIENT_CONTACT_SPEC.md` (§5.1-5.2, §9
M3 row — the spec is normative). NOTE: this packet's write set is in
vendor/truck/** — kernel code, going through the NORMAL loop exactly
so the packet discipline applies; the orchestrator may not edit these
files directly.

```yaml
id:          RDEF-M3-WITNESS-TIER
contract:    [RDEF-M3-WITNESS-TIER]
class:       design
crates:      [truck-certified]
depends_on:  [RDEF-M1-LATTICE-V2]
write_allow:
  - vendor/truck/truck-certified/src/tangency/a2.rs
  - vendor/truck/truck-certified/src/tangency/witness.rs
  - vendor/truck/truck-certified/src/tangency/fixtures.rs
  - vendor/truck/truck-certified/tests/rdef_m3_witness.rs
read_allow:
  - docs/RANK_DEFICIENT_CONTACT_SPEC.md
  - vendor/truck/truck-certified/src/tangency/mod.rs
tests_required: [vendor/truck/truck-certified/tests/rdef_m3_witness.rs]
anchors:
  - {id: A1, expect: 0, cmd: "grep -c 'quadric_pencil' vendor/truck/truck-certified/src/tangency/witness.rs"}
budget:      {turns: 70, ctx_tokens: 220000}
```

## Method

1. **W1 quadric pencils (spec §5.2):** exact classification for
   plane/sphere/cylinder/cone pairs — Dupont–Lazard–Lazard–Petitjean
   is the cited algorithm; tori excluded (quartic — numeric tier).
2. **W2 coincident carriers:** exact rational equality of carrier
   definitions or importer provenance (`FaceProvenance` /
   `SourceEntityId` if the importer carries it — REPORT if it does
   not; that is a finding, not something to fake by sampling).
3. **W3 construction witnesses:** fillet/blend constructions must
   EMIT the contact-curve certificate, not a flag.
4. **Arnold naming (spec CHK-5 note):** document the codebase's A1/A2
   convention in `a2.rs` or rename to `tangent_curve` semantics —
   the decision + rationale go in RESULT notes.
5. Fixtures T1-T5 (spec §10): exact classes + valid B-rep.

## Done when

Spec §9 M3 acceptance satisfied; scoped checks green (`cargo check -p
truck-certified --lib --locked` + the new test file serially); the
`valid_brep`/`complete_locus` rows re-audited in the checker. Write
RESULT.json AT THE WORKTREE ROOT.

## Forbidden

No tolerance. No numeric classification in this packet (M4). No
changes outside the write set, including `mod.rs` — if a module
registration ripple appears, STOP with QUESTION.md (the MONO-7 D1
precedent says record it; here, stop and ask because vendor scope is
strict).
