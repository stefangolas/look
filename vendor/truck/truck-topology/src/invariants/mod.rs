//! BG-INV-001: the §1.1 invariant checkers, one submodule per invariant.
//!
//! Every checker returns [`Outcome<()>`](truck_base::evidence::Outcome):
//!
//! - `Ok(Certified(()))` — the invariant **holds** on the input, certified
//!   with the invariant's [`Prop`] set `True` in the certificate's property
//!   map;
//! - `Err(Refusal::Contradictory(ContraditionWitness { prop, left, right }))`
//!   — the invariant is **violated**: the input claims to be a realisation
//!   (`left: Truth::True`) and the checker measured the opposite (`right:
//!   Truth::False`), with `prop` naming WHICH invariant failed. Where the
//!   tree already has a localising function (`singular_vertices` for the
//!   vertex-link checker, for example) the checker's docs point at it; where
//!   it does not, the module exposes its own listing function.
//! - any other `Err` variant means the checker could **not decide** — never
//!   that the invariant fails. A checker that returns
//!   `Refusal::NumericallyUnresolved` on a healthy input is a defect.
//!
//! The checkers never panic and never mutate their input. House rules
//! H-1..H-7 (spec §0) apply throughout.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

pub mod coedge_pairing;
pub mod domain_boundary;
pub mod euler_poincare;
pub mod same_parameter;
pub mod shell_nesting;
pub mod vertex_link;
pub mod wedge;
