//! BG-FID: the certified feature-size substrate - the formal system's ROOT.
//!
//! Everything downstream compares against a **lower bound** on local feature
//! size: BG-FID-003's isotopy conditions, BG-NUM-004's clustering radii,
//! BG-FID-008's tube widths, and through them the whole Stage-4 interface
//! stack. The module exists to make one specific error impossible to
//! reintroduce: the GLOBAL reach of a mechanical B-rep boundary is ZERO (it
//! collapses at every sharp edge), so any code path that computes or consumes
//! a single global reach is a defect by definition.
//!
//! - **`lfs`** (BG-FID-001) - `lfs_lower(x, stratum)`: the stratified local
//!   feature size LOWER bound. Named `lfs_lower` and typed `LfsLowerBound`,
//!   never a bare `lfs`; the naming is the enforcement, because a bare name
//!   invites a future call site to read the bound as an equality.
//!
//! Scaffolded empty (the num/ precedent): this file records the contract and
//! waits for its packet. House rules H-1..H-8 apply, plus the two FID-local
//! rules recorded in the submodule docs: per-stratum computation, and
//! conservative bound direction everywhere.

/// BG-FID-001: stratified reach and local feature size lower bounds.
/// Scaffolded empty; the packet fills it.
pub mod lfs;
