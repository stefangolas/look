//! Certified constructive geometry substrate: formal pipeline, quotient domain, evidence.

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

/// The per-carrier span BVH broadphase (CFP-005-BRANCH-BVH): a balanced BVH
/// over the admitted span stack of a carrier whose node pairs are pruned by
/// exact Theorem-3 sign rows (SFC float search + exact `Expansion`
/// certificate), so a loft×loft contact enumerates surviving span pairs
/// instead of the full span-pair Cartesian product.
pub mod bvh;
pub mod certified_map;
/// The Contact Fast-Path (CFP) program spine (CFP-000-SPINE): the frozen
/// shapes, fixture kit, gate definitions, and instrument schema of the
/// constructive contact program (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md`).
/// `fixtures` is `#[doc(hidden)]` test-support data.
pub mod cfp;
/// The construct layer (CC-000-CONTRACT): the frozen shapes of the CC program
/// (spine decision C1 — one home for all Phase A/B/C/D construction modules).
pub mod construct;
pub mod contract;
pub mod domain;
pub mod formal;
pub mod hull;
/// The BIE-001 arithmetic substrate: outward-rounded 4-D interval boxes and
/// certified range bounds over them (BIE-001-ARITHMETIC).
pub mod interval;
pub mod kernel;
pub mod meshable;
pub mod pair_dispatch;
/// The spline-carrier admission layer (CL-000-SPLINE-ADMIT): the bridge from a
/// landed `BSplineSurface` to the certified rational tensor-Bernstein engine
/// (`ssi.rs`), plus the certified derivative enclosures spline faces need.
pub mod patch_admit;
pub mod source_evidence;
pub mod ssi;
/// The spline×analytic SSI dispatch bridge (CL-001-SPLINE-LIFT): the funnel
/// arm that admits a spline carrier (CL-000's `patch_admit`) against an exact
/// affine analytic side into the landed SSI square-system engine, with typed
/// κ/cell/slope unresolved outcomes where the engine cannot certify.
pub mod ssi_admit;
#[doc(hidden)]
pub mod ssi_fixtures;
pub mod ssi_trace;
pub mod ssi_types;
/// The tangency layer of the Certified Tangency and Exact Contact (CTE)
/// program (CTE-000-SPINE): the frozen certificate vocabulary for singular SSI
/// closure (T1) and coincident-carrier Boolean classification (T2), plus the
/// F1–F7 fixture kit (booking §5).
pub mod tangency;

/// The SSI wave shim's shared shapes, re-exported at the crate root for the
/// look test target's reachability (BG-CK-P2-CONTRACT).
pub use ssi_types::{KrawczykCertificate3, SquareSystem3, TraceOutcome, TraceRefusal, TraceStep};

/// The kernel-v2 wave workers' import surface, re-exported at the crate root
/// (BG-KV2-000-CONTRACT). `kernel::evidence::Refusal` is re-exported only as
/// [`KernelRefusal`], avoiding the `contract::Refusal` / base `Refusal`
/// ambiguity.
pub use kernel::certs::PointCert;
/// The kernel-v2 refusal vocabulary's [`Refusal`](kernel::evidence::Refusal)
/// under its non-colliding crate-root spelling.
pub use kernel::evidence::Refusal as KernelRefusal;
pub use kernel::evidence::{ClaimVerdict, Construction};
pub use kernel::patch::{CertifiedPatch, IBox};
pub use kernel::residual::ResidualId;
