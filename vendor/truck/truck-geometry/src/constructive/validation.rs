#![deny(clippy::unwrap_used)]

//! Shared V1 profile-law-domain validation (SEM-FACET-SCALE-ZERO-001,
//! SEM-FACET-CORRESPONDENCE-TRUNCATION-001).
//!
//! Both realization backends (`spine_sweep` and `facet_sweep`) must enforce
//! the same admission gate on a `SpineFrameRecipe`'s profile law. This module
//! is the shared home of those checks, so backend symmetry is structural
//! rather than per-entry convention:
//!
//! - [`validate_scalar_law_range`] refuses a `Scale` law whose signed scalar
//!   touches zero anywhere on the CLOSED realization window (the profile
//!   collapses and reflects through the spine there). The reasoning is
//!   interval-style over the law's declared form — endpoint evaluation and
//!   sign bracketing — never station sampling, which is exactly the sampling
//!   hole the defect record names.
//! - [`validate_correspondence`] refuses a `LinearCorrespondence` whose
//!   declared vertex counts differ, mirroring the check
//!   `ProfileLaw::try_linear_correspondence` enforces at construction. A law
//!   built as a struct literal bypasses that constructor; the count gate is
//!   enforced here at evaluation/entry time, never inferred and never
//!   truncating.

use super::errors::ConstructError;
use super::ScalarLaw;

/// Refuses a `ScalarLaw` that is zero, or changes sign, anywhere on the closed
/// parameter window `domain = (s_first, s_last)`, as
/// `Err(ConstructError::ProfileCollapse { at })` with `at` the parameter
/// where the scalar vanishes. `Ok(())` otherwise.
///
/// The detection is interval-style endpoint + sign reasoning over the law's
/// declared form — never station sampling. A `Constant` law collapses iff its
/// value is zero; a `Linear` law collapses iff an endpoint is exactly zero or
/// the endpoint values bracket zero (the linear zero is then interior and its
/// parameter is solved exactly). Comparison semantics are the NaN-refusing
/// `(x > 0.0)`-style: a non-finite endpoint is not a sign change and does not
/// collapse here (the per-station evaluator handles non-finite propagation).
pub fn validate_scalar_law_range(
    law: &ScalarLaw,
    domain: (f64, f64),
) -> Result<(), ConstructError> {
    let (s_first, s_last) = domain;
    match *law {
        ScalarLaw::Constant(c) => {
            if c == 0.0 {
                return Err(ConstructError::ProfileCollapse { at: s_first });
            }
            Ok(())
        }
        ScalarLaw::Linear { start, end } => {
            let a = start + (end - start) * s_first;
            let b = start + (end - start) * s_last;
            if a == 0.0 {
                return Err(ConstructError::ProfileCollapse { at: s_first });
            }
            if b == 0.0 {
                return Err(ConstructError::ProfileCollapse { at: s_last });
            }
            if (a < 0.0 && b > 0.0) || (a > 0.0 && b < 0.0) {
                // The linear law crosses zero exactly once on the window; the
                // crossing parameter is the closed-form root.
                let at = s_first + (s_last - s_first) * (0.0 - a) / (b - a);
                return Err(ConstructError::ProfileCollapse { at });
            }
            Ok(())
        }
    }
}

/// Refuses a `LinearCorrespondence` pairing whose vertex counts differ, as
/// `Err(ConstructError::ProfileCorrespondenceMismatch)`. `start` and `end`
/// are the two profiles' declared vertex counts. The check is exactly the one
/// `ProfileLaw::try_linear_correspondence` enforces at construction, re-applied
/// at evaluation/entry for struct-literal laws that bypass the constructor.
pub fn validate_correspondence(start: usize, end: usize) -> Result<(), ConstructError> {
    if start != end {
        return Err(ConstructError::ProfileCorrespondenceMismatch);
    }
    Ok(())
}
