//! BG-NUM-002: certified univariate root isolation.
//!
//! Scaffolded only. The contract (spec §Stage 3):
//!
//! - Input: a polynomial in the Bernstein basis on a domain, a tolerance
//!   `tau`, and a `Budget`.
//! - Descartes' rule on the Bernstein coefficients counts sign changes:
//!   `0` — no root in the box, prune; `1` — exactly one root, refine to
//!   width < `tau` and emit the isolating interval; otherwise bisect under
//!   the budget.
//! - **Multiple roots** (an even sign-change count that never reaches 1 at
//!   representable width) are `NumericallyUnresolved`, NEVER an empty list —
//!   reporting "no root" for a tangential double root is precisely the §9.2
//!   failure this module exists to prevent.
//! - Property: every returned interval contains exactly one root; the union
//!   contains all roots in the domain.
