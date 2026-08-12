# EPISTEMIC HARDENING HANDOFF — P1 sampling domain, P2 ambiguity demotion, certified sphere period

**Date:** 2026-08-09
**truck-fork HEAD:** `7f8e4890` (pushed to `stefangolas/truck`, branch
`feature/cone-apex-lift-recovery`) — supersedes `d7bb5166` (P3b).
**look HEAD:** `f9befac` — truck pin bumped to `7f8e4890`, override
re-commented, probe examples and the epistemics audit committed.

This supersedes the audit-only state of `PRODUCTION_EPISTEMICS_AUDIT.md`
(audit at `d7bb5166`) with implemented fixes.

---

## 1. Regression attribution (the 308 rendered→lost population)

**Provenance gate passed.** The stored PLANAR-C ABC baseline (839,179 declared /
837,103 rendered / 2,076 lost) is a valid `018bd469` baseline: a clean
`018bd469` build reproduces it per-face for 00007705 (0 rendered/triangle
diffs), and the FACE-VALIDITY `fv_*.ledger` measurements reproduce it exactly
for 5 further models. The rendered→lost comparisons against it are valid.

**The dominant cluster (00007705: 247 rendered→lost, 41 gained) is
P1-caused and is a correctness win, not a regression.**

- Intermediate pins on 00007705: `018bd469`→`17ac0f15` (P1) flips 288 faces
  (247 rendered→lost, 41 lost→rendered). P2 (`f9e06c64`) and P3b (`d7bb5166`)
  change nothing on this model (both 21703/373).
- The 247 are closed-spline boundary faces (190 plane + 42 cylinder + 15 cone)
  whose boundary curve is a degree-3 `B_SPLINE_CURVE_WITH_KNOTS` with unclamped
  end knots (end multiplicity 2). Their **genuine** loop is tiny (all 35
  control points within a 0.005 ball — convex hull argument) and lives inside
  the interior knot span `[0,1]`; the declared extent `[-0.0625, 1.0625]`
  evaluates to basis-degenerate garbage in the sliver (all-zero basis window →
  `subs` = the origin (0,0,0)).
- Pre-P1, sampling the boundary over `range_tuple()` injected origin endpoints;
  the "rendered" mesh was a lens from the world origin to the genuine tiny
  patch. Verified geometrically on #120193: the forced-baseline mesh bbox
  spans `(-1.545, 0.0, -0.038)` to `(0.0, 0.125, 0.0)` — i.e. anchored at the
  origin, which is **0.125 off the face's plane** (plane is y≈0.125). The
  baseline counted 247 malformed meshes as rendered; P1 correctly stopped
  emitting them.
- **Conclusion: the 308-face population is not an accuracy regression.**
  `evaluation_range()` sampling is correct for every corpus closed spline.
  Every curve with `range_tuple ≠ evaluation_range` in the whole corpus (20
  ABC + 33 NIST models) has an origin/degenerate sliver; none has a
  legitimate closure arc beyond the interior span.

## 2. Implemented fixes

### 2.1 P1: basis-certified sampling domain (fork)
- `BoundedCurve::basis_is_partition_of_unity(t)` — the direct Cox–de Boor
  predicate (window non-empty and sums to 1). Default `true`; overridden on
  `BSplineCurve`/`NurbsCurve`; forwarded through the `BoundedCurve` derive
  (enum + single-field arms) and `look`'s `PolicyCurve`.
- `tessellate_edge` samples a closed edge over
  `D_source_edge_use ∩ D_basis_partition_of_unity`: the declared extent is
  used only when the basis certificate holds at its ends. Behaviour-preserving
  on the corpus (00007705 identical at 21703/373) — the extension never fires
  today, and is the principled rule for a legitimate closure-in-sliver curve.
- Tests: NIST closed-spline regression extended with the predicate; new
  ABC-00007705-style regression (unclamped ends, small closed loop).

### 2.2 P2: insufficient evidence is not a source-level ambiguity (fork)
- `SingularTransitionOutcome::CertifiedAmbiguous` → `InsufficientEvidence`.
  The three returns (leaving sample is a pole / no leaving sample) now leave
  the lift unresolved (`AmbiguousLift`), never `RejectedAmbiguous`.
- `RejectedAmbiguous` is retained only as the target type for a future
  certificate that constructs two continuations; no production path emits it.
- **Corpus: the 7 ABC sphere faces (00000959 ×4, 00001075 ×2, 00005760 ×1)
  previously `RejectedAmbiguous` are now `AmbiguousLift` (unresolved).**
  ABC aggregate: 837,170 rendered / 2,009 lost / rejected_ambiguous **0**
  (was 7). NIST unchanged at 7,901/7,902.

### 2.3 P3: sphere azimuth period structurally certified (fork + look)
- `PeriodWitness::ExactSphereAzimuth`, `CertifiedLattice::sphere_azimuth`,
  formal `AnalyticRule::SphereAzimuthPeriodIsTwoPi` +
  `AnalyticPremise::SupportSurfaceIsASphere` + `certify_sphere_azimuth_period`.
- `look::lattice_of`: `ElementarySurface::Sphere` → certified 2π on the
  azimuth axis (stepio wrapper puts longitude on caller-u), oriented by
  `processor.orientation()`.
- P3b cap construction (theorem H1) and the P2 singular continuation now read
  the **generator** (`u_generator`/`v_generator`), never the declared value.
  An uncertified accessor period can no longer certify either path.
- Tests: `a_sphere_certifies_its_azimuth_as_a_generator`,
  `an_inverted_sphere_puts_the_certified_azimuth_on_v` (look); P2/P3b test
  lattices switched to `CertifiedLattice::sphere_azimuth`.

## 3. Verification

- truck-fork: `cargo check --all-targets` clean; meshalgo lib tests 688/690
  (the two failures are the pre-existing PLANAR-C `cone_topology_tests`
  `duplicate_edge_creates_no_second_cdt_edge` and
  `test_parity_intersecting_constraints_rejected`, unchanged since PLANAR-C);
  `truck-geometry` lib 27/27; my changed files fmt-clean on stable.
- look: `cargo check --locked --all-targets` clean;
  `cargo test --locked --all-targets` all pass (163 lib tests, incl. the new
  sphere-lattice tests).
- Corpus (pinned `7f8e4890` build): NIST 7,901/7,902; ABC 837,170/2,009 with
  rejected_ambiguous=0; 00007705 21,703/373 (identical to P1/clean d7bb);
  00005760 43,833/153 rejected_ambiguous=0; nist_18 (P2 spheres) 637/637;
  nist_29 (P3b hemisphere) 144/144.

## 4. Remaining work from the session brief (not done)

The session's hard obligations (regression attribution + P2 epistemic fix +
certified sphere period) are satisfied. The following hardening items from the
audit remain open and are the natural next packet:

1. **P3b collapse certification (audit G3).** `find_cap_pole` still accepts a
   two-sample orbit-diameter scan under `1e-4 × r_loop` and the comments call
   it "certified". For the activated spheres it is exact; it is still a
   numerical candidate for generic surfaces. Separate candidate detection from
   an analytic/structural pole witness (the sphere's pole is the collapsed
   latitude; an analytic witness exists).
2. **P3b activation classification (audit G5/G6/B19).** `so_small()` (absolute
   `1e-6`) and `0.1 * period` remain candidate gates; no production claim
   should treat them as a mathematical class certificate.
3. **`find_cap_pole` scan band (audit C7/B7).** It reads `try_range_tuple()`
   for the non-periodic scan; P1's lesson is that the evaluable domain should
   drive surface sampling. Latent (splines are NON_PERIODIC today), but worth
   aligning with `evaluation_range()`.
4. **Material-side orientation (audit G7/B20).** `n × t` is constructively
   consistent but the `surface.invert()` mechanism is flagged suspect by
   `source_evidence.rs`. Add compact analytic tests over orientation
   permutations (normal/bound/edge flipped, combinations, hemisphere caps).
5. **Epistemic contract types.** The distinctions Candidate / Constructive /
   Certified / Unresolved are now visible in the lattice type and the P2
   outcome enum; a small typed contract in the P3b activation path would
   complete task 9 of the brief.

## 5. Discipline reminders (carry-forward)

- The `.cargo/config.toml` override is re-commented; the pin is `7f8e4890`.
  Keep it that way when reporting numbers.
- Do not update golden images/performance claims to make a test pass.
- No model IDs / `source_face_id`s in production code.
- `AmbiguousLift` is unresolved; only a certificate-carrying path may emit
  `RejectedAmbiguous`.
- The ABC rendered→lost ledger against the planar-c baseline must be read as
  malformed-mesh removal (a correctness win), not as a P1 regression.
