//! BG-FID-001: stratified reach and local feature size (§6.1).
//!
//! Scaffolded only. The contract:
//!
//! ```text
//! lfs_lower(x, stratum)
//!   = min( intrinsic_lower(stratum), separation_lower(x), wedge_lower(x) )
//!   <= lfs(x, stratum)          # the true value is never computed
//! ```
//!
//! | stratum | intrinsic (lower bound on reach) | separation | incident structure |
//! |---|---|---|---|
//! | face interior | `min(1/rho_max_upper, mu_self_lower)` | lower bound on distance to NON-incident strata | lower bound on distance to own boundary wires |
//! | edge interior | lower bound on curve reach of `c_e` | as above | `theta_wedge(e)`, -> 0 as theta -> 0 or 2pi |
//! | vertex | 0-dimensional | star separation | min incident edge length, min angular separation, min dihedral over star |
//!
//! **Per stratum, never global** (BG-FID-001): the global reach of a
//! mechanical B-rep is ZERO - it collapses at every sharp edge - so any code
//! path using a global reach is a defect. This is the specific error §6.1
//! exists to correct, and it is easy to reintroduce.
//!
//! **Positivity needs BG-INV-109** (BG-FID-002): a knife edge (dihedral -> 0)
//! or a crack (-> 2pi) drives `wedge_lower` to zero. Faces whose bound is 0
//! route to COLLAPSE (§5), not to a certificate.
//!
//! **Bound direction** (BG-FID-007): every downstream gate has the form
//! `q < c * lfs_lower`, so substituting a LOWER bound is conservative: it can
//! refuse an instance the true value would admit, and can never admit one the
//! true value would refuse. Two consequences:
//!
//! - Federer's equality `reach = min(1/rho_max, mu_bottleneck)` holds only for
//!   a CLOSED C^2 submanifold. A trimmed patch has boundary, `rho_max_upper`
//!   is a computed upper bound and `mu_self_lower` a computed lower bound -
//!   so no API may return this quantity under any name asserting equality
//!   with reach, and no test may assert equality against a hand-computed
//!   reach.
//! - **Refusals are epistemic.** `ReachLowerBoundTooSmall` asserts the bound
//!   could not be CERTIFIED large enough, not that the feature is small. A
//!   diagnostic saying "feature too small" when the bound merely failed to
//!   converge is a wrong answer with a confident label.
//!
//! Inputs available in this crate: `EnclosureSurface::enclose_der(m, n, ..)`
//! supplies interval second partials for the curvature upper bound
//! (`rho_max_upper` via the second fundamental form over the face cell);
//! `EnclosureCurve` likewise for edge curve reach; `truck-topology`'s
//! BG-INV-109 wedge machinery supplies the dihedral term at edges and the
//! star terms at vertices.
//!
//! Tests owed by this item (spec §6.1):
//! - Unit: a cube. Face-interior point -> distance-to-nearest-edge bound;
//!   edge -> wedge bound; vertex -> star separation. Hand-computed values are
//!   UPPER bounds on what the function may return: assertions are `<=`,
//!   never `==`.
//! - Anti-regression: the GLOBAL reach of a cube is 0 while the stratified
//!   `lfs_lower` is positive everywhere.
//! - Property: 1-homogeneous under uniform scale, invariant under rigid motion.
//! - Property: positive on any solid passing BG-INV-109, zero on a deliberate
//!   knife edge.
//! - Soundness property: brute-force sample the true lfs on small hand-built
//!   cases; assert `lfs_lower <= lfs_sampled` ALWAYS. Over-refusal is
//!   acceptable; OVER-ESTIMATION IS A SILENT-WRONG-ANSWER BUG, because every
//!   downstream gate compares against this bound.
