// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Certified torus annular atlas cell (rank 2) — B2/B3.
//!
//! Built on [`super::torus::CertifiedRankTwoDeck`]. Admits the largest sound
//! homogeneous torus population the corpus presents: a regular ring torus
//! bounded by **two complete parallel circles** (winding `(±1, 0)` each) that
//! are disjoint and bound a unique annular material region — the
//! `1[Ci1];1[Ci1]` / double-outer-declared population (2,375 faces in the
//! post-cone ABC corpus).
//!
//! # The material-authority rule (B3)
//!
//! Two disjoint loops representing the same primitive class `h ∈ H₁(T²)` bound
//! a valid annulus only as `C₁ − C₂` (or its negative). The source boundary
//! chain `∂M` of the material 2-chain `M` must be null-homologous:
//!
//! - **opposite effective orientations** (`[C₁] = h`, `[C₂] = −h`): the chain
//!   is `h + (−h) = 0`, null-homologous; exactly **one** of the two
//!   complementary annuli has this boundary → `Resolved`.
//! - **same effective orientations** (`[C₁] = h`, `[C₂] = h`): the chain is
//!   `2h ≠ 0` (h primitive), **not** null-homologous; it cannot bound either
//!   annulus → `InconsistentBoundaryHomology` (zero solutions), **not**
//!   ambiguity.
//! - **effective orientation unavailable/undecidable**: both complementary
//!   annuli match the unsigned chain → `UnresolvedMaterialAuthority` (two
//!   solutions).
//!
//! The double-outer-bound malformation is a *discarded invalid qualifier*, not
//! an annulus selector: it never implies "material lies between the loops."
//! Admission follows only after the cell proves a unique material annulus; the
//! malformation is recorded as a conformance tag.
//!
//! # Effective orientation
//!
//! The orientation sign each loop carries is the **effective** bound
//! orientation, not the raw circle parameter direction. The caller (look-side
//! adapter) folds each contribution exactly once:
//!
//! ```text
//! effective = curve_traversal × edge_orientation
//!           × loop_traversal × face_bound_orientation
//!           × face/surface_orientation_convention
//! ```
//!
//! A sign of `0` means the effective orientation could not be decided
//! (`UnresolvedMaterialAuthority`).
//!
//! # What is NOT done here
//!
//! No cut-open realization, no triangulation, no production outcome wiring.
//! Meridian-bounded cells (`(0, ±1)`) and mixed classes are refused: the first
//! cell is the parallel-parallel annulus only.

use super::torus::CertifiedRankTwoDeck;
use super::torus_circle::{CircleFamily, OnTorusWitness};
use std::f64::consts::TAU;
use truck_geometry::prelude::{InnerSpace, Point3, Vector3};

/// Dimensionless floor for the parallel/meridian plane-orientation test and
/// the on-axis / coordinate-consistency tests.
const MINIMUM_TORUS_CELL_PARALLELISM: f64 = 1e-9;

/// A boundary loop's geometric placement on the torus plus its **effective**
/// orientation sign (see the module docs for the folding chain).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryLoopPlacement {
    /// The circle's centre.
    pub center: Point3,
    /// The circle's plane normal (caller-certified unit).
    pub normal: Vector3,
    /// The circle's radius.
    pub radius: f64,
    /// The effective bound orientation sign: `+1`, `-1`, or `0` (undecidable).
    pub effective_orientation_sign: i8,
}

/// The primitive homology class of a torus boundary loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveWinding {
    /// A parallel: winding `(±1, 0)`; the circle's plane is perpendicular to
    /// the torus axis (a latitude).
    Parallel,
    /// A meridian: winding `(0, ±1)`; the circle's plane contains the axis.
    Meridian,
}

/// A boundary loop certified as an essential (non-contractible) loop on the
/// torus quotient, carrying its `Z²` winding and the developed coordinate it is
/// constant at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedEssentialLoop {
    placement: BoundaryLoopPlacement,
    primitive: PrimitiveWinding,
    winding: [i64; 2],
    /// The torus parameter the loop is constant at: the minor angle `v` for a
    /// parallel, the major angle `u` for a meridian, reduced to `[0, 2π)`.
    constant_coordinate: f64,
}

impl CertifiedEssentialLoop {
    /// The certified `Z²` winding of this loop (with effective orientation).
    pub fn winding(&self) -> [i64; 2] {
        self.winding
    }
    /// The primitive class (unsigned).
    pub fn primitive(&self) -> PrimitiveWinding {
        self.primitive
    }
    /// The developed coordinate the loop is constant at.
    pub fn constant_coordinate(&self) -> f64 {
        self.constant_coordinate
    }
}

/// The source-boundary composition facts the look-side establishes and the
/// cell requires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceBoundaryComposition {
    /// The number of source boundary components on the face. Must be exactly 2.
    pub component_count: usize,
    /// Whether any extra source edge or hidden bound participates. Must be
    /// false.
    pub extra_source_edge: bool,
    /// The double-outer-bound malformation, if present: two
    /// `FACE_OUTER_BOUND` qualifiers on one face, which are contradictory.
    pub outer_bound_malformation: Option<TwoOuterBoundMalformation>,
}

/// The malformed-source fact: two `FACE_OUTER_BOUND` qualifiers on one face.
///
/// Retained authority (loop identity, traversal, effective orientation,
/// winding, incidence, torus embedding) is unaffected; only the invalid outer
/// qualifiers are discarded by the normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoOuterBoundMalformation;

/// The conformance tag recording whether the certified cell relied on a
/// malformed-source normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceTag {
    /// The source boundary was well-formed.
    SourceClean,
    /// Two contradictory `FACE_OUTER_BOUND` qualifiers were discarded; the
    /// unique material annulus was proved independently of them.
    MalformedTwoOuterBoundsOnCertifiedTorusAnnulus,
}

/// The source-derived material authority: the unique annular 2-chain whose
/// boundary equals the effective source boundary chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertifiedMaterialAuthority {
    conformance: ConformanceTag,
}

impl CertifiedMaterialAuthority {
    /// The conformance tag.
    pub fn conformance(&self) -> ConformanceTag {
        self.conformance
    }
    /// A stable diagnostic tag.
    pub fn tag(&self) -> &'static str {
        match self.conformance {
            ConformanceTag::SourceClean => "torus_annulus_authority_source_clean",
            ConformanceTag::MalformedTwoOuterBoundsOnCertifiedTorusAnnulus => {
                "malformed:two_outer_bounds_on_certified_torus_annulus"
            }
        }
    }
}

/// Why a torus annular cell could not be certified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TorusCellFailure {
    /// A loop's plane is neither parallel nor perpendicular to the axis, or its
    /// centre is off the axis for a parallel: not a primitive torus loop.
    NonprimitiveWinding,
    /// The two loops are not the same unsigned primitive class (inhomologous).
    InhomologousLoops,
    /// The two loops coincide (same developed coordinate mod `2π`): not disjoint.
    IntersectingBoundaries,
    /// A loop's radius/height are inconsistent with the torus: it does not lie
    /// on the surface, or is not a torus-coordinate circle.
    SourceContradiction,
    /// The source does not present exactly two boundary components.
    WrongSourceBoundaryComponentCount,
    /// An extra source edge or hidden bound participates in the face.
    ExtraSourceEdgePresent,
    /// Same effective orientations: the boundary chain is `2h ≠ 0`, not
    /// null-homologous; it cannot bound either annulus (zero solutions).
    InconsistentBoundaryHomology,
    /// The effective orientation of a loop is unavailable/undecidable: both
    /// complementary annuli match the unsigned chain (two solutions).
    UnresolvedMaterialAuthority,
}

/// A certified torus annular atlas cell: a regular torus, a rank-two deck, two
/// homologous essential boundary loops that are disjoint and bound a unique
/// annular material region whose authority is source-derived.
#[derive(Debug, Clone, PartialEq)]
pub struct CertifiedTorusAnnularCell {
    deck: CertifiedRankTwoDeck,
    boundary_a: CertifiedEssentialLoop,
    boundary_b: CertifiedEssentialLoop,
    primitive_class: PrimitiveWinding,
    material_authority: CertifiedMaterialAuthority,
}

impl CertifiedTorusAnnularCell {
    /// The rank-two deck.
    pub fn deck(&self) -> &CertifiedRankTwoDeck {
        &self.deck
    }
    /// The first boundary loop.
    pub fn boundary_a(&self) -> &CertifiedEssentialLoop {
        &self.boundary_a
    }
    /// The second boundary loop.
    pub fn boundary_b(&self) -> &CertifiedEssentialLoop {
        &self.boundary_b
    }
    /// The primitive class shared by both loops.
    pub fn primitive_class(&self) -> PrimitiveWinding {
        self.primitive_class
    }
    /// The source-derived material authority.
    pub fn material_authority(&self) -> &CertifiedMaterialAuthority {
        &self.material_authority
    }
    /// A short stable tag, for probe records.
    pub fn tag(&self) -> &'static str {
        "torus_annular_cell"
    }
}

/// Certify a torus annular cell from a rank-two deck, two boundary loop
/// placements, and the source boundary composition.
///
/// Admission proves the B2 obligations (regular torus, rank-two deck, two
/// closed essential loops, certified primitive `Z²` winding, homologous,
/// disjoint, embedded, exactly two source components, no extra bound, distinct
/// transverse coordinates, torus-coordinate circles) and the B3 material
/// authority (unique annular 2-chain, source-derived from effective
/// orientation, never by size). See the module docs.
pub fn certify_torus_annular_cell(
    deck: &CertifiedRankTwoDeck,
    loop_a: BoundaryLoopPlacement,
    loop_b: BoundaryLoopPlacement,
    composition: &SourceBoundaryComposition,
) -> Result<CertifiedTorusAnnularCell, TorusCellFailure> {
    // Source composition: exactly two components, no extra bound.
    if composition.component_count != 2 {
        return Err(TorusCellFailure::WrongSourceBoundaryComponentCount);
    }
    if composition.extra_source_edge {
        return Err(TorusCellFailure::ExtraSourceEdgePresent);
    }

    let schema = deck.schema();
    let axis = schema.axis();
    let center = schema.center();
    let large = schema.large_radius().get();
    let small = schema.small_radius().get();
    let scale = large + small;

    let cert_a = certify_loop(loop_a, axis, center, large, small, scale)?;
    let cert_b = certify_loop(loop_b, axis, center, large, small, scale)?;

    // Unsigned geometric classes must match (homologous).
    if cert_a.primitive != cert_b.primitive {
        return Err(TorusCellFailure::InhomologousLoops);
    }
    // h is primitive: each loop is a complete source circle (one occurrence,
    // closed), which by the closed-edge rule is exactly one traversal, so its
    // unsigned winding is (±1, 0). The effective orientation sign (which may be
    // 0 = unavailable) is folded in separately below — it is not a statement
    // about primitiveness.
    // Disjoint: distinct transverse (minor) coordinates, not coincident mod 2π.
    let dv = (cert_a.constant_coordinate - cert_b.constant_coordinate).abs();
    let dv_mod = dv % TAU;
    let dv_wrapped = dv_mod.min(TAU - dv_mod);
    if dv_wrapped < MINIMUM_TORUS_CELL_PARALLELISM * scale.max(1.0) {
        return Err(TorusCellFailure::IntersectingBoundaries);
    }

    // B3 material authority: the unique annular 2-chain M with ∂M = effective
    // source boundary. 0/1/2 solutions → Inconsistent/Resolved/Unresolved.
    // The sign lives on the primitive component: winding[0] for parallels,
    // winding[1] for meridians.
    let (s_a, s_b) = match cert_a.primitive {
        PrimitiveWinding::Parallel => (cert_a.winding[0], cert_b.winding[0]),
        PrimitiveWinding::Meridian => (cert_a.winding[1], cert_b.winding[1]),
    };
    let authority = resolve_material_authority(s_a, s_b, composition.outer_bound_malformation)?;

    Ok(CertifiedTorusAnnularCell {
        deck: deck.clone(),
        boundary_a: cert_a,
        boundary_b: cert_b,
        primitive_class: cert_a.primitive,
        material_authority: authority,
    })
}

/// Certify a torus annular cell from pre-certified circle witnesses, skipping
/// the cell's own scale-relative on-torus check.
///
/// The whole-interval Fourier test ([`super::torus_circle::certify_circle_on_torus`])
/// is scale-invariant and more robust than the cell's `certify_loop` check
/// (`cos²v + sin²v - 1 > 1e-9`), which falsely rejects parallels of small-radius
/// tori. When both circles have already been certified on the torus, this
/// function uses the witnesses' winding to determine the primitive class and
/// computes the constant coordinate from the placement geometry, without
/// re-checking on-torus membership.
///
/// The disjointness and material authority checks are identical to
/// [`certify_torus_annular_cell`].
pub fn certify_torus_annular_cell_with_witnesses(
    deck: &CertifiedRankTwoDeck,
    loop_a: BoundaryLoopPlacement,
    loop_b: BoundaryLoopPlacement,
    witness_a: OnTorusWitness,
    witness_b: OnTorusWitness,
    composition: &SourceBoundaryComposition,
) -> Result<CertifiedTorusAnnularCell, TorusCellFailure> {
    if composition.component_count != 2 {
        return Err(TorusCellFailure::WrongSourceBoundaryComponentCount);
    }
    if composition.extra_source_edge {
        return Err(TorusCellFailure::ExtraSourceEdgePresent);
    }

    let schema = deck.schema();
    let axis = schema.axis();
    let center = schema.center();
    let large = schema.large_radius().get();
    let small = schema.small_radius().get();
    let scale = large + small;

    let cert_a = certify_loop_with_witness(loop_a, witness_a, axis, center, large, small, scale)?;
    let cert_b = certify_loop_with_witness(loop_b, witness_b, axis, center, large, small, scale)?;

    if cert_a.primitive != cert_b.primitive {
        return Err(TorusCellFailure::InhomologousLoops);
    }
    let dv = (cert_a.constant_coordinate - cert_b.constant_coordinate).abs();
    let dv_mod = dv % TAU;
    let dv_wrapped = dv_mod.min(TAU - dv_mod);
    if dv_wrapped < MINIMUM_TORUS_CELL_PARALLELISM * scale.max(1.0) {
        return Err(TorusCellFailure::IntersectingBoundaries);
    }

    let (s_a, s_b) = match cert_a.primitive {
        PrimitiveWinding::Parallel => (cert_a.winding[0], cert_b.winding[0]),
        PrimitiveWinding::Meridian => (cert_a.winding[1], cert_b.winding[1]),
    };
    let authority = resolve_material_authority(s_a, s_b, composition.outer_bound_malformation)?;

    Ok(CertifiedTorusAnnularCell {
        deck: deck.clone(),
        boundary_a: cert_a,
        boundary_b: cert_b,
        primitive_class: cert_a.primitive,
        material_authority: authority,
    })
}

/// Certify one boundary loop from a pre-certified witness, skipping the
/// on-torus check. The primitive class comes from the witness's family, and
/// the constant coordinate is computed from the placement geometry.
fn certify_loop_with_witness(
    lp: BoundaryLoopPlacement,
    witness: OnTorusWitness,
    axis: Vector3,
    center: Point3,
    large: f64,
    small: f64,
    scale: f64,
) -> Result<CertifiedEssentialLoop, TorusCellFailure> {
    let primitive = match witness.family {
        CircleFamily::Parallel => PrimitiveWinding::Parallel,
        CircleFamily::Meridian => PrimitiveWinding::Meridian,
        _ => return Err(TorusCellFailure::NonprimitiveWinding),
    };
    let rel = lp.center - center;
    let height = rel.dot(axis);
    let radial_offset = rel - height * axis;
    let (winding, constant_coordinate) = match primitive {
        PrimitiveWinding::Parallel => {
            if radial_offset.magnitude() > MINIMUM_TORUS_CELL_PARALLELISM * scale.max(1.0) {
                return Err(TorusCellFailure::NonprimitiveWinding);
            }
            let cos_v = (lp.radius - large) / small;
            let sin_v = height / small;
            let v = sin_v.atan2(cos_v).rem_euclid(TAU);
            ([lp.effective_orientation_sign as i64, 0], v)
        }
        PrimitiveWinding::Meridian => {
            let u = radial_offset.y.atan2(radial_offset.x).rem_euclid(TAU);
            ([0, lp.effective_orientation_sign as i64], u)
        }
    };
    Ok(CertifiedEssentialLoop {
        placement: lp,
        primitive,
        winding,
        constant_coordinate,
    })
}

/// Resolve the material authority from the two loops' effective winding signs.
///
/// Both loops are parallels with winding `(s_a, 0)` and `(s_b, 0)` where
/// `s_a, s_b ∈ {-1, 0, +1}` (`0` = orientation undecidable). The source
/// boundary chain is null-homologous iff `s_a + s_b == 0` (opposite signs).
fn resolve_material_authority(
    s_a: i64,
    s_b: i64,
    malformation: Option<TwoOuterBoundMalformation>,
) -> Result<CertifiedMaterialAuthority, TorusCellFailure> {
    if s_a == 0 || s_b == 0 {
        // Orientation unavailable: both complementary annuli match the unsigned
        // chain — two solutions.
        return Err(TorusCellFailure::UnresolvedMaterialAuthority);
    }
    if s_a == s_b {
        // Same effective orientation: chain is 2h ≠ 0, not null-homologous —
        // zero solutions. This is an inconsistency, not ambiguity.
        return Err(TorusCellFailure::InconsistentBoundaryHomology);
    }
    // Opposite effective orientations: exactly one annulus matches — Resolved.
    let conformance = match malformation {
        Some(_) => ConformanceTag::MalformedTwoOuterBoundsOnCertifiedTorusAnnulus,
        None => ConformanceTag::SourceClean,
    };
    Ok(CertifiedMaterialAuthority { conformance })
}

/// Certify one boundary loop's primitive class, winding, and constant
/// coordinate.
fn certify_loop(
    lp: BoundaryLoopPlacement,
    axis: Vector3,
    center: Point3,
    large: f64,
    small: f64,
    scale: f64,
) -> Result<CertifiedEssentialLoop, TorusCellFailure> {
    let rel = lp.center - center;
    let height = rel.dot(axis); // signed axial offset from the torus centre
    let radial_offset = rel - height * axis; // ~0 for a parallel (centre on axis)

    let dot = lp.normal.dot(axis);
    let abs_dot = dot.abs();
    let tol = MINIMUM_TORUS_CELL_PARALLELISM;

    let primitive = if abs_dot > 1.0 - tol {
        PrimitiveWinding::Parallel
    } else if abs_dot < tol {
        PrimitiveWinding::Meridian
    } else {
        return Err(TorusCellFailure::NonprimitiveWinding);
    };

    let (winding, constant_coordinate) = match primitive {
        PrimitiveWinding::Parallel => {
            // The centre of a torus parallel lies on the axis.
            if radial_offset.magnitude() > tol * scale.max(1.0) {
                return Err(TorusCellFailure::NonprimitiveWinding);
            }
            // cos v = (r - large)/small,  sin v = height/small.
            let cos_v = (lp.radius - large) / small;
            let sin_v = height / small;
            // The circle must lie on the torus: cos²v + sin²v = 1. This also
            // guarantees it is the torus-coordinate circle at v, not an
            // arbitrary planar circle.
            if (cos_v * cos_v + sin_v * sin_v - 1.0).abs() > 1e-9 {
                return Err(TorusCellFailure::SourceContradiction);
            }
            let v = sin_v.atan2(cos_v).rem_euclid(TAU);
            ([lp.effective_orientation_sign as i64, 0], v)
        }
        PrimitiveWinding::Meridian => {
            let u = radial_offset.y.atan2(radial_offset.x).rem_euclid(TAU);
            ([0, lp.effective_orientation_sign as i64], u)
        }
    };

    Ok(CertifiedEssentialLoop {
        placement: lp,
        primitive,
        winding,
        constant_coordinate,
    })
}

#[cfg(test)]
mod tests {
    use super::super::torus::{identify_torus, TorusIdentification};
    use super::*;
    use truck_geometry::prelude::Torus;

    fn deck() -> CertifiedRankTwoDeck {
        let t = Torus::new(Point3::new(0.0, 0.0, 0.0), 5.0, 1.0);
        match identify_torus(&t) {
            TorusIdentification::Torus(d) => d,
            other => panic!("need a deck, got {other:?}"),
        }
    }

    fn clean_composition() -> SourceBoundaryComposition {
        SourceBoundaryComposition {
            component_count: 2,
            extra_source_edge: false,
            outer_bound_malformation: None,
        }
    }

    fn double_outer_composition() -> SourceBoundaryComposition {
        SourceBoundaryComposition {
            component_count: 2,
            extra_source_edge: false,
            outer_bound_malformation: Some(TwoOuterBoundMalformation),
        }
    }

    /// A parallel circle at minor angle `v`, effective orientation `sign`, on
    /// the canonical z-axis torus (large=5, small=1, centre origin).
    fn parallel(v: f64, sign: i8) -> BoundaryLoopPlacement {
        let r = 5.0 + 1.0 * v.cos();
        let z = 1.0 * v.sin();
        BoundaryLoopPlacement {
            center: Point3::new(0.0, 0.0, z),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: r,
            effective_orientation_sign: sign,
        }
    }

    #[test]
    fn opposite_effective_orientations_resolve_a_unique_annulus() {
        let d = deck();
        let cell = certify_torus_annular_cell(
            &d,
            parallel(0.0, 1),
            parallel(1.2, -1),
            &clean_composition(),
        )
        .unwrap();
        assert_eq!(cell.primitive_class(), PrimitiveWinding::Parallel);
        assert_eq!(cell.boundary_a().winding(), [1, 0]);
        assert_eq!(cell.boundary_b().winding(), [-1, 0]);
        assert_eq!(
            cell.material_authority().conformance(),
            ConformanceTag::SourceClean
        );
    }

    #[test]
    fn same_effective_orientations_are_inconsistent_not_ambiguous() {
        let d = deck();
        let err = certify_torus_annular_cell(
            &d,
            parallel(0.0, 1),
            parallel(1.2, 1),
            &clean_composition(),
        );
        assert_eq!(err, Err(TorusCellFailure::InconsistentBoundaryHomology));
    }

    #[test]
    fn unavailable_orientation_is_unresolved() {
        let d = deck();
        // One loop's effective orientation undecidable (sign = 0).
        let err = certify_torus_annular_cell(
            &d,
            parallel(0.0, 0),
            parallel(1.2, -1),
            &clean_composition(),
        );
        assert_eq!(err, Err(TorusCellFailure::UnresolvedMaterialAuthority));
    }

    #[test]
    fn double_outer_malformation_is_a_conformance_tag_not_a_selector() {
        // Same geometry as the resolved case, but the source carried two
        // FACE_OUTER_BOUND qualifiers. The malformation is discarded; the
        // unique annulus is still proved from orientation.
        let d = deck();
        let cell = certify_torus_annular_cell(
            &d,
            parallel(0.0, 1),
            parallel(1.2, -1),
            &double_outer_composition(),
        )
        .unwrap();
        assert_eq!(
            cell.material_authority().conformance(),
            ConformanceTag::MalformedTwoOuterBoundsOnCertifiedTorusAnnulus
        );
        assert_eq!(
            cell.material_authority().tag(),
            "malformed:two_outer_bounds_on_certified_torus_annulus"
        );
    }

    #[test]
    fn double_outer_does_not_save_an_inconsistent_chain() {
        // Same orientations + double-outer is still inconsistent: the
        // malformation never implies "material lies between the loops."
        let d = deck();
        let err = certify_torus_annular_cell(
            &d,
            parallel(0.0, 1),
            parallel(1.2, 1),
            &double_outer_composition(),
        );
        assert_eq!(err, Err(TorusCellFailure::InconsistentBoundaryHomology));
    }

    #[test]
    fn coincident_parallels_are_intersecting() {
        let d = deck();
        let err = certify_torus_annular_cell(
            &d,
            parallel(0.7, 1),
            parallel(0.7, -1),
            &clean_composition(),
        );
        assert_eq!(err, Err(TorusCellFailure::IntersectingBoundaries));
    }

    #[test]
    fn a_full_turn_apart_parallels_are_coincident_mod_two_pi() {
        let d = deck();
        let err = certify_torus_annular_cell(
            &d,
            parallel(0.4, 1),
            parallel(0.4 + TAU, -1),
            &clean_composition(),
        );
        assert_eq!(err, Err(TorusCellFailure::IntersectingBoundaries));
    }

    #[test]
    fn an_off_axis_circle_is_not_a_primitive_parallel() {
        let d = deck();
        let bad = BoundaryLoopPlacement {
            center: Point3::new(2.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: 5.0,
            effective_orientation_sign: 1,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, parallel(0.0, 1), bad, &clean_composition()),
            Err(TorusCellFailure::NonprimitiveWinding)
        );
    }

    #[test]
    fn a_circle_not_on_the_torus_is_a_source_contradiction() {
        let d = deck();
        let bad = BoundaryLoopPlacement {
            center: Point3::new(0.0, 0.0, 1.0),
            normal: Vector3::new(0.0, 0.0, 1.0),
            radius: 7.0, // cos v = 2 (impossible)
            effective_orientation_sign: 1,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, parallel(0.0, 1), bad, &clean_composition()),
            Err(TorusCellFailure::SourceContradiction)
        );
    }

    #[test]
    fn a_tilted_circle_is_nonprimitive() {
        let d = deck();
        let tilted = BoundaryLoopPlacement {
            center: Point3::new(0.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 1.0).normalize(),
            radius: 5.0,
            effective_orientation_sign: 1,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, parallel(0.0, 1), tilted, &clean_composition()),
            Err(TorusCellFailure::NonprimitiveWinding)
        );
    }

    #[test]
    fn a_meridian_pair_with_opposite_orientations_resolves() {
        // Two meridians (plane contains the axis, normal ⊥ axis), centre off
        // axis at radius large, radius small. The first cell now admits
        // meridian pairs (homologous) as well as parallel pairs.
        let d = deck();
        let m1 = BoundaryLoopPlacement {
            center: Point3::new(5.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            effective_orientation_sign: 1,
        };
        let m2 = BoundaryLoopPlacement {
            center: Point3::new(0.0, 5.0, 0.0),
            normal: Vector3::new(1.0, 0.0, 0.0),
            radius: 1.0,
            effective_orientation_sign: -1,
        };
        let cell = certify_torus_annular_cell(&d, m1, m2, &clean_composition()).unwrap();
        assert_eq!(cell.primitive_class(), PrimitiveWinding::Meridian);
        assert_eq!(cell.boundary_a().winding(), [0, 1]);
        assert_eq!(cell.boundary_b().winding(), [0, -1]);
    }

    #[test]
    fn a_meridian_pair_with_same_orientations_is_inconsistent() {
        let d = deck();
        let m1 = BoundaryLoopPlacement {
            center: Point3::new(5.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            effective_orientation_sign: 1,
        };
        let m2 = BoundaryLoopPlacement {
            center: Point3::new(0.0, 5.0, 0.0),
            normal: Vector3::new(1.0, 0.0, 0.0),
            radius: 1.0,
            effective_orientation_sign: 1,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, m1, m2, &clean_composition()),
            Err(TorusCellFailure::InconsistentBoundaryHomology)
        );
    }

    #[test]
    fn wrong_component_count_is_refused() {
        let d = deck();
        let bad = SourceBoundaryComposition {
            component_count: 3,
            extra_source_edge: false,
            outer_bound_malformation: None,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, parallel(0.0, 1), parallel(1.2, -1), &bad),
            Err(TorusCellFailure::WrongSourceBoundaryComponentCount)
        );
    }

    #[test]
    fn extra_source_edge_is_refused() {
        let d = deck();
        let bad = SourceBoundaryComposition {
            component_count: 2,
            extra_source_edge: true,
            outer_bound_malformation: None,
        };
        assert_eq!(
            certify_torus_annular_cell(&d, parallel(0.0, 1), parallel(1.2, -1), &bad),
            Err(TorusCellFailure::ExtraSourceEdgePresent)
        );
    }
}
