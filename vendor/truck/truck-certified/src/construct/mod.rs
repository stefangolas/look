#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The CC program shim (CC-000-CONTRACT): the frozen construct-layer shapes —
//! the interval universe, the refusal vocabulary, the config constants, the
//! sole inari bridge, the seam stub types, and the machine-checked fixture
//! kit. No solver body lands here.
//!
//! **C1 — one home.** All Phase A/B/C/D construction modules live in this one
//! module tree of `truck-certified` (spine decision C1). The loft, offset /
//! shell, and blend wave packets write only into `construct/**`.
//!
//! **C2 — one manifest edge.** This packet adds the single sanctioned manifest
//! edge `truck-certified → truck-evidence` to this crate's `Cargo.toml`:
//! `truck-evidence = { version = "0.1.0", path = "../truck-evidence" }`
//! (spine decision C2, the recorded escape hatch). It is added once, here, and
//! is never extended further without a spec amendment. The kernel-v2 "zero new
//! manifest edges" doctrine is scoped to the kernel module; this module
//! carries the amendment record.
//!
//! **C3 — no inari in this crate.** The construct layer uses [`Interval`] —
//! the SAME alias as `kernel::Interval` (`crate::formal::exact::CertifiedInterval`),
//! never a second interval type — and `kernel::patch::IBox{2,3}` for parameter
//! boxes. The inari world is only ever reached through the C2 edge's boundary
//! types; the sole bridge is [`convert`] (`from_inari` / `box3_to_ibox`).
//!
//! **C7 — stub posture.** Types and refusing constructors only. Every public
//! production function returns `Err(ConstructRefusal::Unfrozen)` (the
//! refusing-stub marker, matching the `contract::Refusal::Unfrozen` precedent)
//! until its owning wave packet lands.
//!
//! **C9 — determinism house rules.** Fixed-order float reductions, no
//! hash-iteration-dependent output, no bare absolute literals (H-3 same-line
//! opt-out in tests), no `unwrap` / `expect` / `panic!` in shipped code, and
//! COMMIT BEFORE RESULT.json. These carry into `construct/**` verbatim.
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module and every submodule. The new files carry no `unwrap`, no
//! `expect`, and no `panic!`, and add no module-level `allow`.

/// The certified-interval primitive of the construct layer (C3): the SAME
/// alias as `kernel::Interval` — one interval type, never a second one.
pub type Interval = crate::formal::exact::CertifiedInterval;

/// The P1 banded fast path (CC-001-BANDED, seam S3): interval no-pivot
/// Gaussian elimination for banded totally-positive collocation matrices.
pub mod banded;

/// The P4 argmin-with-margin operator (seam S5): the strict-separation argmin
/// over interval enclosures, refusing `AmbiguousEventOrdering` on overlap.
pub mod argmin;

/// The C6 normative config constants (the `kernel/config.rs` pattern).
pub mod config;

/// The only sanctioned inari bridge (C3): exact, order-preserving field
/// copies that add no width.
pub mod convert;

/// The §6 machine-checked fixture kit.
///
/// This module is `#[doc(hidden)] pub`: TEST SUPPORT ONLY, excluded from the
/// certified API surface, but reachable by wave packets' integration tests
/// through the crate's public path.
#[doc(hidden)]
pub mod fixtures;

/// The P2 local injectivity radius (`δ = 2σ/L`) over the certified map types
/// (CC-002-INJECTIVITY, spine seam S4).
pub mod injectivity;

/// The construct refusal vocabulary (C4), frozen here and grown only by CC-000
/// amendment.
pub mod refusal;

/// The Rump / Ogita / Oishi residual fallback (CC-001-BANDED, seam S3): the
/// certified enclosure for dense systems outside the banded-TP class.
pub mod residual_solve;

/// The loft core (CC-010-LOFT-CORE, seam S8): tensor-product loft construction
/// over the landed `truck_geometry::nurbs` types — compatibility, stationing,
/// and the collocation solve through the P1 banded factor.
pub mod loft;

/// The certified positive weight field of a delivered loft (CC-011-LOFT-WEIGHTS,
/// spine S8 consumer): certify-or-refuse strict positivity of the homogeneous
/// weight field, refining dyadically and returning the applied refinements.
pub mod loft_weights;

/// The closed-wire loft as strips (CC-012-LOFT-STRIPS, spine S8/S9 consumer):
/// r strip lofts over matched edges sharing one banded factorization, with
/// P6 identity-keyed split values and a bitwise seam gate between strips.
pub mod loft_strips;

/// The S9 cyclic correspondence resolver (CC-013-CORRESPONDENCE, spine S9 /
/// theory §2.2 L4): wire production over the S9 [`stubs::WireComplex`] and the
/// fixed-order resolution — caller anchor, combinatorially forced unique
/// isomorphism, then the P4 separation-margin argmin over the r cyclic shifts
/// of the declared [`stubs::ShiftFunctionalKind::VertexSumSq`] functional.
pub mod correspondence;

/// The L5 loft validity certificate (CC-014-LOFT-VALIDITY, spine S4/S6/S7
/// consumers): the three-valued regularity + self-contact postcondition over
/// the closed-wire strip loft, composing rank margins, the P2 injectivity
/// radius, the P3 graph-disk decider, and the evidence contact funnel.
pub mod loft_validity;

/// The Gordon Boolean-sum construction (CC-015-GORDON, spine S8 consumer):
/// `S = S_u + S_v − S_uv` over a compatible profile/guide network, reusing the
/// CC-010 loft machinery and its two direction factorizations.
pub mod gordon;

/// The S10 canal-surface regularity seam (CC-025-CANAL): the radius-law
/// production evaluators and the closed-form arc-restricted regularity
/// criterion over a certified spine, refusing `CanalSingular`.
pub mod canal;

/// The S11 three-support constrained contact system (CC-020-CONTACT-K3): the
/// ≤4-unknown reduced-variable mapping over the offset-centre chart, the
/// arity-4 Krawczyk solve (`krawczyk_c1_n4`), and the typed node outcome.
pub mod contact3;

/// The seam stub types (S6/S9/S10/S11/S12): frozen shapes and refusing
/// constructors only, no production logic.
pub mod stubs;

/// The P3 graph-disk embedding certificate and its normative projection search
/// (CC-005-GRAPHDISK, spine seam S6).
pub mod graphdisk;

/// The rounded-offset contact-complex strata (CC-021-OFFSET-STRATA): the
/// k=1 face, k=2 edge (canal), and k=3 corner (triple-node) strata with their
/// certified reach bounds and per-stratum refusals.
pub mod offset_strata;

/// The closed-star embedding certificate and the certified broad phase over
/// the constructed strata (CC-022-STARS, spine S6 consumer): the glued
/// [`stars::Star`] reduced to the P3 graph-disk machinery, and the
/// reach-bound [`stars::reach_prune`] broad phase (sound but not complete).
pub mod stars;

/// The two-support rolling-ball blend trace (CC-030-BLEND-SPINE, seam S12):
/// the certified predictor/corrector walk of each branch to its certified
/// events, with event isolation and the P6 shared-node discipline.
pub mod blend;

/// The variable-radius blend trace (CC-031-BLEND-VARRADIUS, seam S12 / theory
/// §5.3): the amended CC-030 walk closed by the foot-point pair of a certified
/// guide curve and radius law, with the foot-point uniqueness gate (Section 1).
pub mod blend_varradius;

/// The face-consumption outcome of the trim arrangement (CC-032-FACE-CONSUMPTION,
/// seam S12 consumer / theory §5.4): `F_i_new = F_i \ R_i` decided by the
/// arrangement of the contact pcurves, with certified per-cell blend-side
/// classification and the surviving cells' trim provenance.
pub mod face_consumption;

/// The S1 embedding certificate on the stratum quotient and the S1′ solid
/// corollary (CC-023-SHELL-BRIDGE, spine S7 consumer): the shell certificate —
/// per-pair three-valued verdicts (stars → broad phase → the evidence contact
/// funnel), the certified-star count, and the pre-made closed/connected/
/// orientation checks that decide [`shell::SolidOutcome`].
pub mod shell;

/// The conservative certified shell thickness bound (CC-026-THICKNESS, spine
/// S7 consumer; theory §7.1): the focal term from the per-patch interval
/// quadratic solve and the bottleneck term over the non-adjacent strata.
pub mod thickness;

/// The n-valent corner setback patch (CC-033-SETBACK, Phase D, spine S6/S3
/// consumers / theory §5.5): the deterministic corner-advance fill of a 2n
/// sided corner loop with Hermite ribbon patches per boundary arc, certified
/// on four counts (boundary, G¹ ribbons, local regularity, global
/// embeddedness).
pub mod setback;

/// The Certified Interaction Engine contract shim (BIE-000-CONTRACT): the
/// frozen restricted-pair outcome vocabulary, the recorded §8.1 carrier
/// decision, and the unit-shape fixture kit that later BIE wave packets build
/// their synthetic fixtures and grading tests against.
pub mod bie;
