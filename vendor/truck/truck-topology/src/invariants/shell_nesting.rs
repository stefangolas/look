//! BG-INV-108: shell nesting is a forest (§1.1 invariant 8; audit F-1).
//!
//! Scaffolding only — the packet fills this module. The containment order of
//! connected shell components is a forest: antisymmetric, cycle-free, inner
//! shells mutually disjoint. Fixes F-1: disjoint lumps are two solids, not
//! one solid with a phantom cavity.
