//! BG-NUM-003: the Krawczyk existence/uniqueness operator.
//!
//! Scaffolded only. The contract (spec §Stage 3):
//!
//! ```text
//! K(Q) = m − Y·F(m) + (I − Y·J(Q))·(Q − m)
//!   m = midpoint(Q) (float),  Y = float inverse of J(m)
//!   K ⊆ strict interior(Q)  ->  Proven(unique root in Q)   # existence AND uniqueness
//!   K ∩ Q = ∅              ->  Proven(no root in Q)
//!   otherwise              ->  bisect under Budget
//! ```
//!
//! **The center term `F(m)` is a point evaluation** — never the interval `F`
//! over `Q`, which decorrelates the linear part against the contraction term
//! and certifies nothing (measured on the BG-ENC-004-ISC carrier: K ≥ 5×
//! width(Q) at every scale with the interval center, second-order width with
//! the point center). `Proven(unique)` is emitted **only** on strict interior
//! containment — `K ⊆ Q` non-strict proves existence, not uniqueness.
//! The parameterized form (system additionally depending on `t ∈ T`) follows
//! the same rule with `F(m, t_mid)` and `J(Q, T)`.
