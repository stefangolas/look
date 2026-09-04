# CC-023-SHELL-BRIDGE — S1 embedding on the quotient + S1′ solid corollary

CC program Phase C (spine S7 consumer; theory §4.2–4.4). The bridge theorem:
with (1) the stratum complex a compact 2-manifold-with-corners, (2) exactly
consistent realizations on identified strata (P6), and (3) injectivity on
the quotient, F_t is a topological embedding. Hypothesis (3) is discharged
in three regimes — P2 near-diagonal, P3 on stars, contact funnel elsewhere.
S1′ adds the Jordan–Brouwer solid corollary for closed connected orientable
complexes.

```yaml
id:          CC-023-SHELL-BRIDGE
contract:    [CC-023-SHELL-BRIDGE]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-004-CLEAR, CC-022-STARS]
write_allow:
  - vendor/truck/truck-certified/src/construct/shell.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_shell.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
  - vendor/truck/truck-evidence/src/contact
budget:      {turns: 24, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn certify_star' vendor/truck/truck-certified/src/construct/stars.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn reach_prune' vendor/truck/truck-certified/src/construct/stars.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub fn contact' vendor/truck/truck-evidence/src/contact/mod.rs"}
  - {id: A4, expect: 1, cmd: "grep -c 'pub enum BallAdmissibility' vendor/truck/truck-evidence/src/clear.rs"}
tests_required:
  - pn_convex_prism_shells_certified_at_small_t
  - self_contact_pair_reports_unintended_contact
  - undecided_pair_surfaces_inconclusive_never_certified
  - open_complex_refuses_solid_corollary_with_typed_outcome
  - s1_prime_requires_closed_connected_orientable
```

Section 1: the shell certificate — THREE-VALUED per the CG verdict doctrine
(the CC-014 precedent): `pub enum ShellPairVerdict { Certified, Contact,
Inconclusive }`, `pub struct ShellCert { pub pairs: Vec<ShellPairVerdict>,
pub stars_certified: usize, pub solid: Option<SolidOutcome> }`, and `pub fn
certify_shell(strata: Vec<OffsetStratum>, glue: &GluePlan, budget: &mut
Budget) -> Result<ShellCert, ConstructRefusal>`. Ok always means the
certificate was produced; validity is read off it. Pipeline per theory
§4.4, in order: stars through `certify_star` (A1) → broad phase via
`reach_prune` (A2) → retained pairs through the landed evidence contact
funnel (A3) through the manifest edge (conversion at the `convert.rs`
bridge ONLY), with `ball_clearance` (A4) as the P5 admissibility predicate
where the pair reduces to a ball-vs-excluded-boundary question. Contact →
`Contact` (the caller refuses `UnintendedContact`); funnel refusals of the
`NumericallyUnresolved` family → `Inconclusive`; budget exhaustion →
`Inconclusive`, never `Certified`.

Section 2: S1′ solid corollary — `pub enum SolidOutcome { Solid,
SurfaceOnly { reason: SurfaceOnlyReason } }`, `pub enum SurfaceOnlyReason {
Open, Disconnected, OrientationUnresolved }`. Pre-made, checked in order:
the complex is closed (every glue edge identified pairwise — an open sheet
is `Open`); connected under the glue plan (`Disconnected`); orientation —
provenance-first: each stratum's offset side (the caller-supplied ε_i from
CC-021) determines the material side locally by construction; the check is
CONSISTENCY of the induced global nesting (theory §4.6's provenance-first
doctrine), not independent re-determination; unresolvable →
`OrientationUnresolved`. All three pass → `Solid`. A shell that certifies
embedding but fails S1′ is a certified SURFACE, not a solid — the type
makes that distinction unrepresentable to ignore.

Section 3: ground truth — a prism with plane/sphere-class faces (the
landed carriers) at a small outward t shells `Solid` with every pair
`Certified`; a deliberately open complex (one boundary left unidentified)
yields `SurfaceOnly { Open }`; a two-component complex yields
`Disconnected`.

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test construct_shell`.
No workspace builds. The `pub mod shell;` line in `construct/mod.rs` is the
DESIGNED one-line conflict. COMMIT BEFORE writing RESULT.json AT THE
WORKTREE ROOT.

Stop conditions: (1) this packet COMPOSES landed certificates — no new
contact solving, no new hull kernels, no ray casting (the provenance-first
orientation check makes the ray-casting fallback unnecessary in v1; if you
find yourself implementing one, stop and file QUESTION.md); (2) the
conversion of evidence-funnel `Refusal` variants onto the three-valued
verdict is the CC-014 mapping table — reuse its shape and record any
divergence in RESULT notes; (3) thickness queries are CC-026 — do not
compute t_focal or d_min here.
