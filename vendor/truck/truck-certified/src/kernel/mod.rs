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

//! The kernel-v2 shim: the shared shapes, the refusing constructors, and the
//! machine-checked fixture kit (BG-KV2-000-CONTRACT).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module and every submodule. The new files carry no `unwrap`, no
//! `expect`, and no `panic!` calls, and add no module-level `allow`.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses with a named
//! `RefusalKind` (or returns `RefusalKind`-carrying data for later use). This
//! module freezes the kernel-v2 shapes; the wave packets (BG-KV2-1xx/2xx/3xx/4xx)
//! implement against it and never restate it.
//!
//! **D-reuse.** [`Interval`] and [`SignCert`] alias the landed
//! `formal/exact.rs` primitives — zero new manifest edges (no inari). The
//! landed refusal vocabularies (`truck_base::evidence::Refusal`,
//! `contract::Refusal`) are NOT widened and NOT re-exported through this
//! module.
//!
//! **D-spelling.** The spec's §16 spellings are used INSIDE this module
//! (`Refusal`, `Arc`, `Sheet`, `Node`, ...). At the crate root only
//! `kernel::evidence::Refusal` is re-exported, under the name
//! [`crate::KernelRefusal`] (avoiding `contract::Refusal` / base `Refusal`
//! ambiguity); `ClaimVerdict`, `Construction`, `ResidualId`,
//! `CertifiedPatch`, `IBox`, and `PointCert` are also crate-root re-exports
//! (none collide). `kernel::graph::Arc<const N>` shadows `std::sync::Arc`
//! module-locally — acceptable, noted in `graph.rs`. `Frame<const N>` does not
//! collide (`Frame3` lives in truck-geometry, a different crate).
//!
//! **D-fixtures-public.** [`fixtures`] is `#[doc(hidden)] pub`: test support
//! only, excluded from the certified API surface, but reachable by wave
//! workers' integration tests through the crate's public path.

/// The certified-interval primitive of the kernel (D-reuse): aliases the
/// landed `CertifiedInterval`.
pub type Interval = crate::formal::exact::CertifiedInterval;
/// The certified-sign primitive of the kernel (D-reuse): aliases the landed
/// `CertifiedSign`.
pub type SignCert = crate::formal::exact::CertifiedSign;

/// §14.2 segment gluing, deck identification, and §16 graph assembly
/// (BG-KV2-303-S9A): the Rules A/B/C endpoint identity, the C1-agreement tube
/// overlap, the deck-step breaks of a closed chain, and the certified-graph
/// constructor over the frozen topology shapes.
pub mod assemble;
/// §3.3/§3.4 the lifted atlas over the rational carriers (BG-KV2-405-K2B): the
/// finite atlas of regular charts per carrier kind with chart ids, overlap
/// regions, and exact affine/rational transition data, the pole-chart
/// sphere atlas, the cone/torus chart families joining the admitted carrier
/// family, the `SwitchChart`-vs-`CarrierSingular` degeneracy doctrine, and the
/// unwrapped K2 pcurve lifts with the deck integer as a first-class coordinate
/// and [`crate::kernel::config::DECK_MAX`] as the termination bound.
pub mod atlas;
pub mod certs;
/// §15 authored-topology verification (BG-KV2-503-S10): the claim vocabulary
/// ([`claims::TopologyClaim`], [`claims::ClaimedComponent`],
/// [`claims::ClaimRefutation`]) and the verification entry
/// [`claims::certify_claimed`] with the trusted/non-exhaustive claimed-graph
/// path [`claims::claim_claimed`], over the shared-chart graph-arrangement
/// leaf pair ([`claims::LeafPair`]).
pub mod claims;
pub mod config;
/// The §10.3 isolated-contact classifier (BG-KV2-302-S5A): the tolerance-
/// tagged contact claim (Corollary 10.2 + Prop 10.3 over the frozen C2/C3
/// seams and `krawczyk_c1`).
pub mod contact;
pub mod coons_patch;
/// The certificate-calculus engine (BG-KV2-201-S2A): Lemma 8.0's rho, the
/// generic square C1, the C2 tube, and frame construction. This is the wave-2
/// real engine over the landed interval core; the shim shapes it emits are
/// frozen in [`certs`].
pub mod engine;
pub mod evidence;
/// The machine-checked fixture kit — test support only.
#[doc(hidden)]
pub mod fixtures;
pub mod graph;
pub mod identity;
pub mod leaf;
pub mod leaf_extract;
/// The §6.3 maximal-minor algebra (BG-KV2-301-S03A): Theorem 6.4's `m` vector
/// (`m_j = (−1)^j det(DF with column j deleted)`) as a certified enclosure
/// over a per-box 3x4 Jacobian, with the `DF·m = 0` and `a·m` checkables.
pub mod minor_algebra;
pub mod patch;
/// The §8.5/§8.6 projection certificates (BG-KV2-305-S2B): GraphCert's cone
/// test (Theorem 8.3, no solve), the R5 enclosure contract's five steps over
/// the frozen `R5Enclosure` shim shape, and the packaged §7 R4 / R4′ square
/// projection solves.
pub mod projection;
/// §14.3 promotion of an assembled arc to a model edge (BG-KV2-502-S9B): the
/// eight refusing promotion conditions walked as one entry over the landed
/// assemble output, emitting the spec 14.3 record ([`promote::PromotedEdge`]) —
/// a KERNEL RECORD, deliberately not a live `truck_topology::Edge` handle.
pub mod promote;
pub mod rational;
pub mod residual;
/// The §7 R8/R9 square residuals (BG-KV2-202-S1A): the curve–surface system
/// (arity 3) and the one-chart curve–curve system (arity 2) over the S2A C1
/// seam, plus the 1-var homogeneous curve leaf they consume.
pub mod residuals_r89;
/// The §13 R6 self-intersection residual (BG-KV2-404-S8): the deflated
/// divided-difference residual on the Bézier net, the exact-cover charts A/B
/// (Theorem 13.1), the Theorem 13.3 transition seams emitting the frozen
/// `R6ChartSwitch` / `R6BaseSwap` segment breaks, and the Theorem 13.4 λ = 0
/// routing (chart or carrier, never the contact classifier).
pub mod selfint;
/// The §11 exact-overlap sheet classifier (BG-KV2-403-S6): `SheetCert` for
/// real over the recognized carriers (plane/plane, cylinder/coaxial,
/// sphere/concentric) and the certified leaf-pair affine map, with the real
/// `PsiMap`, the four §11 conditions, and the `NearOverlap` disproof.
pub mod sheet;
/// The Tier-1 loop-free certificate and the §9.3 R8 boundary-stratum seeds
/// (BG-KV2-301-S03A): the two-cone LP of Theorem 9.1 (cos-space cone
/// separation) and the R8 subdivision seeds over caller-supplied boundary
/// edges.
pub mod tier1;
/// The Tier-2 critical-point start set (BG-KV2-304-S3B): the §7 R3 minor-form
/// residual `Psi_a(x) = (F(x), a·m(x))` (arity 4, square) over the frozen
/// seam's additive arity-4 C1 entry, and the §9.2 subdivision start set with
/// the a-posteriori `k_a` direction-perturbation retry rule (Corollary 9.3's
/// composition with 301's boundary seeds).
pub mod tier2;
/// The D4 float predictor-corrector (BG-KV2-207-S4A): the fast, UNCERTIFIED
/// branch tracer whose accept/reject path always goes through the certified
/// seam ([`engine::build_frame4`] + [`engine::c2_certify_tube4`]), with the
/// §10.2 escalation ladder.
pub mod tracer;
/// The §9.4 trim clip (BG-KV2-401-S3C): certified R9 crossings between the
/// leaf-product 1-complex arcs and the closed trim loops of the same chart,
/// arc splitting at the certified crossings, and inside/outside classification
/// of the sub-arcs by the winding number of the closed trim loop about one
/// certified-off interior sample; outside sub-arcs are discarded and the trim
/// boundary endpoints become `TopoNode::TrimCrossing` nodes.
pub mod trimclip;
