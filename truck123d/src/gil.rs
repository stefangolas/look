//! GIL policy and the canonical kernel-call slot (spec §5, PB-004 scope
//! decision 2).
//!
//! Every kernel call in the bridge runs with the GIL released. The release is
//! a single choke point — [`with_kernel_gil_released`] — so a reviewer can
//! verify the policy in one place and the test suite can prove it with a
//! second thread. The closure that runs detached must never panic across the
//! FFI (H-1): it converts errors, never unwinds.
//!
//! The exercised kernel call is the evidence algebra's `Modulus::compose`
//! (a landed `truck-base` entry, `evidence.rs` BG-EVD). It is deliberately
//! geometry-free — this crate has zero geometric content (spec §5) — and it
//! returns a real `Outcome`, so the full refusal→typed-exception path is
//! reachable from a marshaled call. A non-subadditive operand (`Pole`/
//! `Unbounded`) refuses with `ForwardToleranceExceeded`.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use pyo3::Python;
use pyo3::marker::Ungil;
use serde::{Deserialize, Serialize};
use truck_base::evidence::{Modulus, ModulusShape, Refusal};

/// A wire description of a kernel `Modulus` (evidence algebra ω). Marshaling
/// only: converting between this spec and the kernel type is pure data
/// plumbing with no arithmetic of its own.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ModulusSpec {
    /// ω(ε) = k·ε.
    Lipschitz {
        /// The Lipschitz constant.
        k: f64,
        /// ω's domain; omitted/`null` means global (`f64::INFINITY`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        domain: Option<f64>,
    },
    /// ω(ε) = k·ε^p.
    Holder {
        /// The leading constant.
        k: f64,
        /// The Hölder exponent.
        exponent: f64,
        /// ω's domain; omitted/`null` means global.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        domain: Option<f64>,
    },
    /// ω(ε) = k·ε/(domain − ε).
    Pole {
        /// The leading constant.
        k: f64,
        /// The (finite) domain edge.
        domain: f64,
    },
    /// No bound published.
    Unbounded,
}

impl ModulusSpec {
    /// Converts the spec into the kernel `Modulus`. Total: an absent or
    /// non-positive domain is normalized to the global domain rather than
    /// refused — refusal is the kernel's job, done inside the landed call.
    pub fn into_kernel(self) -> Modulus {
        match self {
            ModulusSpec::Lipschitz { k, domain } => Modulus {
                shape: ModulusShape::Lipschitz(k),
                domain: normalize_domain(domain),
            },
            ModulusSpec::Holder {
                k,
                exponent,
                domain,
            } => Modulus {
                shape: ModulusShape::Holder { k, exponent },
                domain: normalize_domain(domain),
            },
            ModulusSpec::Pole { k, domain } => Modulus {
                shape: ModulusShape::Pole { k },
                domain: normalize_domain(Some(domain)),
            },
            ModulusSpec::Unbounded => Modulus::Unbounded,
        }
    }
}

fn normalize_domain(domain: Option<f64>) -> f64 {
    match domain {
        Some(d) if d.is_finite() && d > 0.0 => d,
        _ => f64::INFINITY,
    }
}

/// Runs `f` with the GIL released (pyo3 0.29 `Python::detach`). The closure
/// and its result must be `Ungil` (nothing Python-touching crosses).
pub fn with_kernel_gil_released<F, T>(py: Python<'_>, f: F) -> T
where
    F: Ungil + FnOnce() -> T,
    T: Ungil,
{
    py.detach(f)
}

/// The landed kernel call: compose two moduli through the evidence algebra.
/// Returns the composed modulus, or the typed refusal the landed
/// `Modulus::compose` produced (it refuses a non-subadditive operand with
/// `ForwardToleranceExceeded`).
pub fn compose_kernel(a: ModulusSpec, b: ModulusSpec) -> Result<Modulus, Refusal> {
    let kernel_a = a.into_kernel();
    let kernel_b = b.into_kernel();
    // Landed entry (evidence.rs, BG-EVD): no arithmetic happens here.
    kernel_a.compose(&kernel_b).map(|certified| {
        // Compose's refusal set is exactly the `ForwardToleranceExceeded`
        // arm; mapping away the certificate loses no refusal information.
        certified.value
    })
}
