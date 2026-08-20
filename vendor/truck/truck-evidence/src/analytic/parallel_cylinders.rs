//! BG-ANA-001-PARCYL: parallel-axis cylinders — two lines, one tangent line,
//! or empty. This shard owns the **margin sweep** test of BG-ANA-002: two
//! cylinders walked through tangency (`|d| → r₀+r₁`) must switch cleanly
//! transverse → tangent → disjoint with no band of wrong-but-confident
//! answers near the crossing.
//!
//! Scaffolded empty by the orchestrator so that the eight ANA shards have
//! disjoint write sets; the packet replaces this comment with the
//! implementation. The result type is [`crate::analytic::AnalyticIntersection`]
//! and is shared across the whole family — it is not to be redefined here.
