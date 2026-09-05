# CC-021-OFFSET-STRATA â€” rounded offset strata k=1/2/3 with certified reach bounds

CC program Phase C (spine S10/S11 consumers; theory Â§3.3, Â§3.4, Â§4.1). The
rounded offset of a solid IS the constant-radius rolling-ball contact
complex: face strata (k=1), canal edge strata over certified spines (k=2),
spherical corner patches at P4-isolated centres (k=3). Every stratum carries
a certified reach bound Ï_A â‰¥ sup d(x, A); for ball strata Ï_A = |t|.

```yaml
id:          CC-021-OFFSET-STRATA
contract:    [CC-021-OFFSET-STRATA]
class:       design
crates:      [truck-certified]
depends_on:  [CC-000-CONTRACT, CC-020-CONTACT-K3, CC-025-CANAL]
write_allow:
  - vendor/truck/truck-certified/src/construct/offset_strata.rs
  - vendor/truck/truck-certified/src/construct/mod.rs
  - vendor/truck/truck-certified/tests/construct_offset_strata.rs
read_allow:
  - docs/CERTIFIED_CONSTRUCTION_CONTRACTS.md
  - vendor/truck/truck-certified/src/construct
budget:      {turns: 26, ctx_tokens: 100000}
anchors:
  - {id: A1, expect: 1, cmd: "grep -c 'pub fn canal_regularity(' vendor/truck/truck-certified/src/construct/canal.rs"}
  - {id: A2, expect: 1, cmd: "grep -c 'pub fn solve_triple_node' vendor/truck/truck-certified/src/construct/contact3.rs"}
  - {id: A3, expect: 1, cmd: "grep -c 'pub struct TripleContactNode' vendor/truck/truck-certified/src/construct/stubs.rs"}
  - {id: A4, expect: 2, cmd: "grep -c 'pub fn rank_margin' vendor/truck/truck-certified/src/certified_map.rs"}
tests_required:
  - face_stratum_focal_margin_bounds_j_t_from_below
  - face_stratum_refuses_focal_degeneracy_when_margin_straddles_zero
  - edge_stratum_routes_through_canal_regularity
  - corner_stratum_matches_triple_node_centre
  - reach_bound_is_exact_for_ball_strata
```

Section 1: the stratum record â€” `pub enum OffsetStratum { Face { map:
CertifiedSurfaceMap-side data, j_t_lower: Interval }, Edge { spine:
CertifiedCurveMap-side data, radius: f64, canal: Interval }, Corner { node:
TripleContactNode } }` â€” spelled with the landed carrier types this module
actually holds (CC-010's Vector4 convention; exact Rust adjustable, the
DISTINCTION between the three strata and their carried certificates is the
contract). Plus `pub fn reach_bound(&self) -> f64` â€” exact |t| for Face and
Edge (ball strata; theory Â§3.4) and the corner's certified centre-to-source
bound for Corner. `FocalDegeneracy` for Face, `CanalSingular` via the canal
fn for Edge â€” each refusal carries the stratum id.

Section 2: k=1 face stratum â€” from an admitted source face and offset t:
the offset Jacobian determinant J_t = 1 âˆ’ 2Ht + KtÂ² certified from below
over the face. Pre-made: v1 computes the J_t lower bound WITHOUT principal
curvature extraction, from interval enclosures composed on the map's BÃ©zier
patches: the first-form machinery the map already admits (its rank margin
certifies the immersion) plus second-derivative hulls â€” the SAME path
CC-002 uses for â€–DÂ²Sâ€–. If the composition cannot deliver a sound J_t lower
bound from the landed map accessors, STOP and file QUESTION.md (do not
build a second-form module here â€” that is CC-026's booked deliverable, and
this packet must not duplicate it). J_t lower bound â‰¤ 0 or straddling â†’
`Err(FocalDegeneracy)`.

Section 3: k=2 edge stratum â€” the spine (an edge of the source solid as an
admitted curve map) + constant radius t, routed through `canal_regularity`
(A1) with the `Constant` law; its refusal propagates as `CanalSingular`. k=3
corner stratum â€” `solve_triple_node` (A2) over the three incident face
regions; its `Node` outcome packages directly into the Corner stratum (A3),
its `Empty` outcome is a caller error â†’ `InvalidInput` (a corner was
expected).

Section 4: reach gate test â€” `reach_bound_is_exact_for_ball_strata`: for
face and edge strata the bound equals |t| exactly (assert equality, H-3
opt-outs); the corner bound is asserted â‰¥ the distance from the node centre
enclosure to each support's bounding box (a sound lower bound, not the
exact supremum â€” record that in the module doc).

House rules: **H-1: no `unwrap`/`expect`/`panic!` in shipped code, no
module-level `allow`.** **H-3: float comparisons in tests take the `// H-3`
opt-out ON THE SAME LINE.** **All cargo invocations go through the queue
(the `cargo` on PATH IS the queue shim). Do not invoke cargo by absolute
path; do not unset the shim.** Scoped checks only: `cargo check -p
truck-certified` and `cargo test -p truck-certified --test
construct_offset_strata`. No workspace builds. The `pub mod
offset_strata;` line in `construct/mod.rs` is the DESIGNED one-line
conflict. COMMIT BEFORE writing RESULT.json AT THE WORKTREE ROOT.

Stop conditions: (1) the offset SIDE signs (Îµ_i) are caller-supplied per
support â€” this packet does not infer convexity from geometry; if the input
shape cannot carry side signs, record the actual convention in RESULT
notes; (2) sharp/concave completions are CC-024's â€” if you find yourself
extending offset faces or intersecting them, stop; (3) this packet
certifies strata INDIVIDUALLY â€” stars, broad phase, and embedding are
CC-022/CC-023.
