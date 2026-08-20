//! BG-ENC-004-PCURVE: `EnclosureSurface` for the `PCurve` decorator.
//!
//! Scaffolding only. The impl is the body of work packet BG-ENC-004-PCURVE;
//! this file is declared in `mod.rs` up front so that the sibling decorator
//! packets have disjoint write sets and can run in parallel. The composition
//! pattern and the shared cone construction (interval cross product to a box,
//! mignitude norm for the immersion margin, ball-around-midpoint for the
//! cone) are the ones the three landed decorators use; see the BG-ENC-004
//! amendment for why that construction is the family's single shape.
