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

//! The BIE-001 arithmetic substrate (BIE-001-ARITHMETIC): outward-rounded 4-D
//! interval boxes and certified range bounds over them.
//!
//! The scalar interval engine is the landed
//! [`formal::exact::CertifiedInterval`](crate::formal::exact::CertifiedInterval)
//! (outward-rounded `add`/`sub`/`neg`/`mul`/`div`/`sqrt`/`width`/`contains`);
//! this module adds no second interval algebra. It composes that primitive into
//!
//! 1. [`box4::IntervalBox4`] — a 4-D parameter box over `(u, v, s, t)` with a
//!    refusing constructor and axis bisection, whose component-wise
//!    `add`/`sub`/`mul` inherit the scalar outward rounding;
//! 2. [`bounds::mean_value_bound`] — the first-order (mean-value / Taylor)
//!    range bound of a function over a box from a certified derivative
//!    enclosure;
//! 3. [`bounds::TensorGrid4`] + [`bounds::bernstein_box4`] — range evaluation
//!    of a 4-D tensor-Bernstein polynomial over a sub-box, composed from the
//!    landed [`crate::hull::hull_bernstein_2d`].
//!
//! Refusal vocabulary: named cases only, no catch-all
//! ([`IntervalRefusal`]); fallible operations return [`Outcome`] (H-2).
//! **H-1.** Every module here carries no `unwrap`, no `expect`, no `panic!`,
//! and no out-of-range indexing reachable from geometry (`bisect` refuses
//! out-of-range axes, it never indexes). **H-6.** Every bound returned by this
//! module is certified by construction (outward rounding); the module applies
//! no `Method` tag — a float-computed bound is never recorded as
//! `Method::Exact` by this module or its consumers.

pub mod bounds;
pub mod box4;

/// The typed outcome of a fallible interval-box or range-bound operation
/// (H-2). `Ok` carries the certified value; `Err` carries a named
/// [`IntervalRefusal`].
pub type Outcome<T> = Result<T, IntervalRefusal>;

/// Why an interval-box or range-bound operation refused.
///
/// Named cases only — no catch-all — matching the refusal shape of the landed
/// [`crate::hull::HullRefusal`] and [`crate::contract::Refusal`]
/// vocabularies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalRefusal {
    /// The requested bounds are inverted (`lo > hi`).
    InvertedBounds,
    /// A bound or coefficient is not finite (`NaN` or `±inf`).
    NonFinite,
    /// A coefficient layout does not match the declared tensor-grid counts
    /// (an empty axis or a coefficient vector of the wrong length).
    InvalidLayout,
    /// The box is not a compact sub-box of the tensor grid's domain `[0, 1]^4`
    /// (the closed domain boundary is admissible).
    DomainNotCompact,
    /// The outward-rounded enclosure overflows the finite `f64` range.
    EnclosureUnavailable,
}

pub use bounds::{bernstein_box4, mean_value_bound, TensorGrid4};
pub use box4::IntervalBox4;
