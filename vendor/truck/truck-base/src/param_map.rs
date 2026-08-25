//! BG-SOL-P0-REC — the certified parameter correspondence φ, moved from
//! `truck-evidence/src/deviation.rs` (BG-CE-002).
//!
//! `phi(t) = scale * t + offset`. Lives in `truck-base` because the structural
//! recognizer's witness (`truck-geometry/src/recognize.rs`) carries a `map:
//! ParamMap`, and `truck-geometry` depends on `truck-base` but not on
//! `truck-evidence`. `truck-evidence` re-exports this module's type from
//! `deviation`, so `use truck_evidence::ParamMap;` keeps resolving.
//!
//! The interval application `phi.apply(tt)` is deliberately NOT here: it needs
//! `inari`, which `truck-base` does not depend on. It lives in `truck-evidence`
//! as the private `deviation::apply_param_map`, where the outward-rounded
//! application is certified.
//!
//! Scaffolded with the type (moved verbatim from BG-CE-002); the recognizer
//! packet fills its consumers.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// The parameter correspondence phi between two parameterizations:
/// phi(t) = scale * t + offset, computed in plain f64.
///
/// ```
/// use truck_base::param_map::ParamMap;
/// let phi = ParamMap::from_ranges(0.0, 1.0, 0.0, 2.0).expect("non-degenerate range");
/// assert_eq!(phi.apply_f64(0.5), 1.0);
/// assert_eq!(ParamMap::IDENTITY.apply_f64(0.5), 0.5);
/// assert_eq!(ParamMap::flip(0.0, 1.0).apply_f64(0.25), 0.75);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamMap {
    /// The scale factor.
    pub scale: f64,
    /// The offset.
    pub offset: f64,
}

impl ParamMap {
    /// phi(t) = t.
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        offset: 0.0,
    };

    /// phi(t) = t0 + t1 - t, the orientation flip over [t0, t1].
    pub const fn flip(t0: f64, t1: f64) -> Self {
        Self {
            scale: -1.0,
            offset: t0 + t1,
        }
    }

    /// The affine map sending [a0, a1] onto [b0, b1]; `None` when a0 == a1.
    pub fn from_ranges(a0: f64, a1: f64, b0: f64, b1: f64) -> Option<Self> {
        if a0 == a1 {
            None
        } else {
            let scale = (b1 - b0) / (a1 - a0);
            Some(Self {
                scale,
                offset: b0 - a0 * scale,
            })
        }
    }

    /// phi(t) in f64 (for sampling guards and tests, never for certification).
    pub fn apply_f64(&self, t: f64) -> f64 {
        self.scale * t + self.offset
    }
}
