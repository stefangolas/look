#![cfg_attr(not(debug_assertions), deny(warnings))]
#![deny(clippy::all, rust_2018_idioms)]
#![deny(clippy::unwrap_used)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

//! The S9 cyclic correspondence resolver (CC-013-CORRESPONDENCE, theory §2.2
//! L4, spine seam S9).
//!
//! Correspondence is an orientation, an anchor, and a cyclic edge matching on
//! an abstract oriented cyclic complex. Section 1 produces the S9
//! [`WireComplex`] data; Section 2 walks the FIXED resolution order over the
//! r cyclic shifts:
//!
//! 1. a caller-supplied anchor ([`ShiftFunctional::anchor`]) — returned
//!    immediately with that anchor and its explicit orientation, never
//!    second-guessed by geometry;
//! 2. a unique isomorphism forced by the combinatorial data — in v1 the only
//!    such case is `r = 2` (a digon), which resolves index-preserving
//!    WITHOUT invoking the geometric argmin;
//! 3. the P4 separation-margin argmin ([`argmin_margin`]) over the r cyclic
//!    shifts under the DECLARED geometric functional — v1 is closed at
//!    [`ShiftFunctionalKind::VertexSumSq`];
//! 4. refuse [`ConstructRefusal::AmbiguousCorrespondence`] when the argmin
//!    enclosures overlap — never a proximity tie-break.
//!
//! Twist minimization is not an objective anywhere; the argmin certifies
//! strict separation, never intent.
//!
//! # Shift convention
//!
//! A wire is a closed cyclic complex of `r = arc_count` arcs with `r`
//! per-vertex position enclosures, indexed in cyclic order. A correspondence
//! expresses each section's indexing as a cyclic re-parameterization of the
//! wire's. For a shift parameter `s`:
//!
//! * orientation-preserving (forward): wire vertex `i` is matched to section
//!   vertex `(i + s) mod r`;
//! * orientation-reversing (reversed): wire vertex `i` is matched to section
//!   vertex `(s - i) mod r`.
//!
//! In both cases `s` is the section vertex that the wire's vertex `0` is
//! matched to, so [`Correspondence::shifts`] has a uniform meaning whether it
//! came from the anchor or from the argmin. The automatic path searches only
//! the FORWARD orientation; an orientation-reversing match is taken only when
//! the caller supplied it explicitly in the anchor.
//!
//! # The step-2 forced case (`r = 2`)
//!
//! The packet fixes `r = 2` to resolve in step 2 without the geometric
//! functional (stop condition 3): a two-arc closed wire is a digon whose two
//! arcs run between the same two split vertices, so the only isomorphism
//! compatible with the matched-split identity the vertices carry upstream is
//! the index-preserving one — a cyclic shift by one would pair the wire's
//! split vertex `i` with the section's split vertex `i + 1`, contradicting
//! the matched split. The resolver therefore returns shift `0` for every
//! section without computing [`ShiftFunctionalKind::VertexSumSq`].
//!
//! # Errors
//!
//! * [`ConstructRefusal::InvalidInput`] on a malformed wire or section
//!   (`arc_count < 2`, vertex count unequal to `arc_count`, a non-finite or
//!   inverted enclosure), on a section that is not isomorphic to the wire
//!   (equal arc count is the only structural requirement), and on an anchor
//!   index outside `0..arc_count`.
//! * [`ConstructRefusal::AmbiguousCorrespondence`] when the step-3 argmin
//!   enclosures overlap (the S9 refusal — the P4 `AmbiguousEventOrdering` is
//!   mapped onto it here, never a fallback pick).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`,
//! and adds no module-level `allow`.

use crate::construct::argmin::argmin_margin;
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::{Correspondence, ShiftFunctional, ShiftFunctionalKind, WireComplex};
use crate::construct::Interval;

/// Build the S9 production [`WireComplex`] for a closed oriented cyclic wire.
///
/// `vertices` lists the per-vertex position enclosures in cyclic order; a
/// valid wire needs `arc_count >= 2` and exactly `arc_count` vertices (it is
/// a cycle — the vertex count always equals the arc count). Refuses
/// [`ConstructRefusal::InvalidInput`] on a malformed complex.
pub fn wire_complex_of(
    arc_count: usize,
    vertices: &[[Interval; 3]],
) -> Result<WireComplex, ConstructRefusal> {
    validate_complex(arc_count, vertices)?;
    Ok(WireComplex {
        arc_count,
        vertices: vertices.to_vec(),
    })
}

/// Resolve the cyclic correspondence of a wire against its sections (spine
/// S9, theory §2.2 L4).
///
/// The resolution order is fixed (Section 2 above): (1) a caller-supplied
/// [`ShiftFunctional::anchor`] returns immediately with that anchor and its
/// explicit orientation; (2) an `r = 2` wire resolves in step 2 without the
/// argmin; (3) otherwise the declared [`ShiftFunctionalKind::VertexSumSq`]
/// functional is evaluated over EACH of the r cyclic shifts as an
/// outward-rounded [`Interval`] enclosure and passed to [`argmin_margin`] —
/// strict separation selects that shift, overlap refuses
/// [`ConstructRefusal::AmbiguousCorrespondence`].
///
/// Every section must be isomorphic to the wire: equal arc count is the only
/// structural requirement (edge splitting already happened upstream; this
/// module never splits).
pub fn resolve_correspondence(
    wire: &WireComplex,
    sections: &[WireComplex],
    functional: &ShiftFunctional,
) -> Result<Correspondence, ConstructRefusal> {
    validate_complex(wire.arc_count, &wire.vertices)?;
    let r = wire.arc_count;
    for section in sections {
        validate_complex(section.arc_count, &section.vertices)?;
        if section.arc_count != r {
            return Err(ConstructRefusal::InvalidInput);
        }
    }

    if sections.is_empty() {
        return Ok(Correspondence {
            orientation: true,
            anchor: None,
            shifts: Vec::new(),
        });
    }

    // Resolution step 1: a caller-supplied anchor is never second-guessed.
    if let Some(anchor) = functional.anchor {
        if anchor.index >= r {
            return Err(ConstructRefusal::InvalidInput);
        }
        return Ok(Correspondence {
            orientation: !anchor.reversed,
            anchor: Some(anchor.index),
            shifts: vec![anchor.index; sections.len()],
        });
    }

    // Resolution step 2: `r = 2` is the combinatorially forced unique
    // isomorphism (digon split-stability); the argmin is not consulted.
    if r == 2 {
        return Ok(Correspondence {
            orientation: true,
            anchor: None,
            shifts: vec![0; sections.len()],
        });
    }

    // Resolution step 3: the declared functional over the r cyclic shifts.
    // v1 is closed at VertexSumSq (the enum stays closed).
    match functional.kind {
        ShiftFunctionalKind::VertexSumSq => {}
    }

    let mut shifts = Vec::with_capacity(sections.len());
    for section in sections {
        let mut enclosures = Vec::with_capacity(r);
        for shift in 0..r {
            enclosures.push(forward_vertex_sum_sq(wire, section, shift));
        }
        match argmin_margin(&enclosures) {
            Ok(shift) => shifts.push(shift),
            Err(ConstructRefusal::AmbiguousEventOrdering) => {
                return Err(ConstructRefusal::AmbiguousCorrespondence);
            }
            Err(refusal) => return Err(refusal),
        }
    }

    Ok(Correspondence {
        orientation: true,
        anchor: None,
        shifts,
    })
}

/// Validate the structural invariants of one wire complex (also used as the
/// `wire_complex_of` admission check).
fn validate_complex(arc_count: usize, vertices: &[[Interval; 3]]) -> Result<(), ConstructRefusal> {
    if arc_count < 2 {
        return Err(ConstructRefusal::InvalidInput);
    }
    if vertices.len() != arc_count {
        return Err(ConstructRefusal::InvalidInput);
    }
    for vertex in vertices {
        for axis in vertex {
            if !axis.is_finite() || axis.lo > axis.hi {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
    }
    Ok(())
}

/// The squared distance enclosure between two vertex position enclosures,
/// accumulated per axis with outward-rounded interval arithmetic.
fn squared_distance_between(a: &[Interval; 3], b: &[Interval; 3]) -> Interval {
    let dx = a[0].sub(&b[0]);
    let dy = a[1].sub(&b[1]);
    let dz = a[2].sub(&b[2]);
    dx.mul(&dx).add(&dy.mul(&dy)).add(&dz.mul(&dz))
}

/// The declared v1 functional ([`ShiftFunctionalKind::VertexSumSq`]) for one
/// forward cyclic shift: the sum of squared distances between matched
/// vertices, accumulated in index order over interval arithmetic.
fn forward_vertex_sum_sq(wire: &WireComplex, section: &WireComplex, shift: usize) -> Interval {
    let r = wire.arc_count;
    let mut total = Interval::point(0.0);
    for i in 0..r {
        let j = (i + shift) % r;
        let distance = squared_distance_between(&wire.vertices[i], &section.vertices[j]);
        total = total.add(&distance);
    }
    total
}
