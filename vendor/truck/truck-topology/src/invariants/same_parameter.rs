//! BG-INV-104: same-parameter / same-range (§1.1 invariant 4).
//!
//! Scaffolding only — the packet fills this module. Certifies
//! `||Γ_f(pc_u(t)) − c_e(φ_u(t))|| ≤ τ_e` over the whole span for every edge
//! use that carries a pcurve, by BG-CE-002's `certify_deviation`. Edge uses
//! with `pcurve() == None` (the `PC = ()` default) are vacuously satisfied.
