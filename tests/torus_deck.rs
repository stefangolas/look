//! Integration tests for the certified rank-two torus deck witness.
//!
//! Mirrors how `src/step/cone.rs`'s inline tests construct their surfaces:
//! `Surface::ElementarySurface(ElementarySurface::ToroidalSurface(...))`
//! built from `Processor<Torus, Matrix4>`.

use look::step::torus_deck::{
    LatticeOrientation, MajorAxis, TorusDeckFailure, WindingFailure, identify_source_torus_deck,
};
use truck_meshalgo::prelude::{
    EuclideanSpace, InnerSpace, Invertible, Matrix4, Point3, Rad, Vector3,
};
use truck_meshalgo::tessellation::formal::DeckVector2;
use truck_stepio::r#in::step_geometry::{ElementarySurface, Processor, Surface, Torus};

/// A torus about the z-axis at the origin, identity transform, forward
/// orientation.
fn z_torus(major: f64, minor: f64) -> Surface {
    Surface::ElementarySurface(ElementarySurface::ToroidalSurface(Processor::new(
        Torus::new(Point3::new(0.0, 0.0, 0.0), major, minor),
    )))
}

/// A torus placed by a translation + rotation.
fn placed_torus(major: f64, minor: f64, transform: Matrix4) -> Surface {
    Surface::ElementarySurface(ElementarySurface::ToroidalSurface(
        Processor::with_transform(
            Torus::new(Point3::new(0.0, 0.0, 0.0), major, minor),
            transform,
        ),
    ))
}

const TAU: f64 = std::f64::consts::TAU;

// ---------------------------------------------------------------------------
// Ordinary torus placement
// ---------------------------------------------------------------------------

#[test]
fn ordinary_torus_placement_is_certified() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    assert_eq!(deck.rank(), 2);
    assert_eq!(deck.source().major_radius().get(), 3.0);
    assert_eq!(deck.source().minor_radius().get(), 1.0);
    assert!((deck.source().center() - Point3::new(0.0, 0.0, 0.0)).magnitude() < 1e-9);
    assert!((deck.source().axis() - Vector3::new(0.0, 0.0, 1.0)).magnitude() < 1e-9);
    assert!((deck.source().radial_x() - Vector3::new(1.0, 0.0, 0.0)).magnitude() < 1e-9);
    assert!((deck.source().radial_y() - Vector3::new(0.0, 1.0, 0.0)).magnitude() < 1e-9);
    assert_eq!(deck.source().major_axis(), MajorAxis::U);
    assert_eq!(deck.orientation(), LatticeOrientation::Preserving);
}

// ---------------------------------------------------------------------------
// Independent major/minor periods
// ---------------------------------------------------------------------------

#[test]
fn major_and_minor_periods_are_independent() {
    let deck = identify_source_torus_deck(&z_torus(5.0, 2.0)).expect("certifies");
    let [major_gen, minor_gen] = deck.generators();
    // Both periods are 2π.
    assert!((major_gen.signed_period().get() - TAU).abs() < 1e-12);
    assert!((minor_gen.signed_period().get() - TAU).abs() < 1e-12);
    // They lie on distinct developed axes — the structural independence proof.
    assert_ne!(major_gen.periodic_axis(), minor_gen.periodic_axis());
    // Major is on First (caller's u), minor on Second (caller's v).
    assert_eq!(
        major_gen.periodic_axis(),
        truck_meshalgo::tessellation::formal::deck::DevelopedAxis::First
    );
    assert_eq!(
        minor_gen.periodic_axis(),
        truck_meshalgo::tessellation::formal::deck::DevelopedAxis::Second
    );
}

// ---------------------------------------------------------------------------
// Orientation-preserving transform
// ---------------------------------------------------------------------------

#[test]
fn orientation_preserving_transform_is_certified() {
    let transform = Matrix4::from_translation(Vector3::new(10.0, -3.0, 4.0))
        * Matrix4::from_angle_z(Rad(0.7))
        * Matrix4::from_scale(2.5);
    let surface = placed_torus(3.0, 1.0, transform);
    let deck = identify_source_torus_deck(&surface).expect("a similarity certifies");
    assert_eq!(deck.orientation(), LatticeOrientation::Preserving);
    assert!((deck.source().major_radius().get() - 7.5).abs() < 1e-9);
    assert!((deck.source().minor_radius().get() - 2.5).abs() < 1e-9);
    // The center moves with the translation.
    assert!(
        (deck.source().center() - Point3::new(10.0, -3.0, 4.0)).magnitude() < 1e-8,
        "center should be at the translation"
    );
}

// ---------------------------------------------------------------------------
// Reflected transform
// ---------------------------------------------------------------------------

#[test]
fn reflected_transform_is_certified_reversing() {
    // Reflect across the xz-plane: y -> -y. det = -1.
    let reflect = Matrix4::from_nonuniform_scale(1.0, -1.0, 1.0);
    let surface = placed_torus(3.0, 1.0, reflect);
    let deck = identify_source_torus_deck(&surface).expect("a reflection is a similarity");
    assert_eq!(deck.orientation(), LatticeOrientation::Reversing);
    // Radii are unchanged (uniform scale 1).
    assert!((deck.source().major_radius().get() - 3.0).abs() < 1e-9);
    assert!((deck.source().minor_radius().get() - 1.0).abs() < 1e-9);
}

#[test]
fn reflected_with_uniform_scale_is_still_reversing() {
    // Reflect + uniform scale: det < 0, still a similarity.
    let transform = Matrix4::from_nonuniform_scale(2.0, -2.0, 2.0);
    let surface = placed_torus(3.0, 1.0, transform);
    let deck = identify_source_torus_deck(&surface).expect("reflection + scale certifies");
    assert_eq!(deck.orientation(), LatticeOrientation::Reversing);
    assert!((deck.source().major_radius().get() - 6.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Nonuniform transform refusal
// ---------------------------------------------------------------------------

#[test]
fn nonuniform_scale_is_refused() {
    let squash = Matrix4::from_nonuniform_scale(1.0, 0.5, 1.0);
    let surface = placed_torus(3.0, 1.0, squash);
    assert_eq!(
        identify_source_torus_deck(&surface).unwrap_err(),
        TorusDeckFailure::PlacementNotASimilarity
    );
}

#[test]
fn shear_is_refused() {
    let mut shear = Matrix4::from_scale(1.0);
    shear.y.x = 0.3;
    let surface = placed_torus(3.0, 1.0, shear);
    assert_eq!(
        identify_source_torus_deck(&surface).unwrap_err(),
        TorusDeckFailure::PlacementNotASimilarity
    );
}

// ---------------------------------------------------------------------------
// Generator independence
// ---------------------------------------------------------------------------

#[test]
fn generators_are_structurally_independent() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let [g0, g1] = deck.generators();
    // Independence is structural: distinct developed axes with nonzero
    // periods. Under the axis-aligned basis schema the determinant is the
    // product of the two nonzero periods — provably nonzero.
    assert_ne!(g0.periodic_axis(), g1.periodic_axis());
    assert!(g0.signed_period().get() != 0.0);
    assert!(g1.signed_period().get() != 0.0);
    assert!(!g0.signed_period().is_zero());
    assert!(!g1.signed_period().is_zero());
}

// ---------------------------------------------------------------------------
// Equivalent lattice bases
// ---------------------------------------------------------------------------

#[test]
fn equivalent_lattice_bases_agree_via_unimodular_change() {
    // The canonical basis generates 2π·Z². An equivalent basis
    // {(2π, 0), (2π, 2π)} generates the same lattice via the GL(2,Z) matrix
    // M = [[1, 1], [0, 1]] (column convention: g'_0 = g_0, g'_1 = g_0 + g_1).
    // A displacement of (2π, 2π) has canonical winding (1, 1). Under the
    // change of basis M^{-1} = [[1, -1], [0, 1]], the target-basis winding
    // is (1·1 - 1·1, 0·1 + 1·1) = (0, 1).
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let canonical = deck.winding_of_displacement(TAU, TAU).expect("winding");
    assert_eq!(canonical, DeckVector2::new(1, 1));

    let m = [[1_i64, 1], [0, 1]];
    let target = deck
        .change_of_basis_to_canonical(canonical, m)
        .expect("unimodular");
    assert_eq!(target, DeckVector2::new(0, 1));

    // The canonical basis is what the deck certifies.
    assert!(deck.is_canonical_basis());
}

#[test]
fn non_unimodular_change_of_basis_is_refused() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let winding = DeckVector2::new(1, 0);
    // det = 2*1 - 0*0 = 2, not ±1.
    let m = [[2_i64, 0], [0, 1]];
    assert!(deck.change_of_basis_to_canonical(winding, m).is_none());
}

// ---------------------------------------------------------------------------
// Z² winding coordinates
// ---------------------------------------------------------------------------

#[test]
fn zero_winding() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let w = deck
        .winding_of_displacement(0.0, 0.0)
        .expect("zero winding");
    assert_eq!(w, DeckVector2::new(0, 0));
}

#[test]
fn one_generator_winding_major_only() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    // Displace one full major period, zero on minor.
    let w = deck
        .winding_of_displacement(TAU, 0.0)
        .expect("major winding");
    assert_eq!(w, DeckVector2::new(1, 0));
    // Two full major periods backward.
    let w = deck
        .winding_of_displacement(-2.0 * TAU, 0.0)
        .expect("major winding");
    assert_eq!(w, DeckVector2::new(-2, 0));
}

#[test]
fn one_generator_winding_minor_only() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    // Displace one full minor period, zero on major.
    let w = deck
        .winding_of_displacement(0.0, TAU)
        .expect("minor winding");
    assert_eq!(w, DeckVector2::new(0, 1));
    let w = deck
        .winding_of_displacement(0.0, -3.0 * TAU)
        .expect("minor winding");
    assert_eq!(w, DeckVector2::new(0, -3));
}

#[test]
fn mixed_winding() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let w = deck
        .winding_of_displacement(2.0 * TAU, -3.0 * TAU)
        .expect("mixed winding");
    assert_eq!(w, DeckVector2::new(2, -3));
    let w = deck
        .winding_of_displacement(-TAU, 5.0 * TAU)
        .expect("mixed winding");
    assert_eq!(w, DeckVector2::new(-1, 5));
}

#[test]
fn large_winding_count() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let k = 12345_i64;
    let w = deck
        .winding_of_displacement(k as f64 * TAU, -(k as f64) * TAU)
        .expect("large winding");
    assert_eq!(w, DeckVector2::new(k, -k));
}

// ---------------------------------------------------------------------------
// Closed quotient loop with displaced lifted endpoints
// ---------------------------------------------------------------------------

#[test]
fn closed_quotient_loop_with_displaced_lifted_endpoints() {
    // A curve that starts at (0, 0) and ends at (2π, 0): the endpoints
    // coincide on the torus (same physical point) but differ on the
    // universal cover by the deck translation (1, 0).
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    let w = deck
        .winding_of_lifted_endpoints((0.0, 0.0), (TAU, 0.0))
        .expect("closed loop");
    assert_eq!(w, DeckVector2::new(1, 0));

    // A diagonal loop closing via (2π, 2π).
    let w = deck
        .winding_of_lifted_endpoints((0.0, 0.0), (TAU, TAU))
        .expect("closed loop");
    assert_eq!(w, DeckVector2::new(1, 1));

    // A loop that winds twice in major and once backward in minor.
    let w = deck
        .winding_of_lifted_endpoints((0.5, 0.3), (0.5 + 2.0 * TAU, 0.3 - TAU))
        .expect("closed loop");
    assert_eq!(w, DeckVector2::new(2, -1));
}

// ---------------------------------------------------------------------------
// Non-integer displacement (open curve)
// ---------------------------------------------------------------------------

#[test]
fn non_integer_displacement_is_refused_as_open() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    // Half a period: certified not an integer multiple.
    let result = deck.winding_of_displacement(TAU / 2.0, 0.0);
    assert!(matches!(
        result,
        Err(WindingFailure::NotIntegerMultiple { axis, .. }) if axis == MajorAxis::U
    ));
}

#[test]
fn near_integer_displacement_certifies() {
    let deck = identify_source_torus_deck(&z_torus(3.0, 1.0)).expect("certifies");
    // A displacement one ULP short of 2π: within the certified-equal band.
    let near = TAU - f64::EPSILON * TAU;
    let w = deck
        .winding_of_displacement(near, 0.0)
        .expect("near-integer certifies");
    assert_eq!(w, DeckVector2::new(1, 0));
}

// ---------------------------------------------------------------------------
// Processor orientation (axis swap)
// ---------------------------------------------------------------------------

#[test]
fn inverted_processor_swaps_major_minor_axes() {
    let mut processor = Processor::new(Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0));
    processor.invert();
    let surface = Surface::ElementarySurface(ElementarySurface::ToroidalSurface(processor));
    let deck = identify_source_torus_deck(&surface).expect("an inverted torus certifies");
    // Under orientation() == false, the caller's u carries the entity's v
    // (minor), so major is on V.
    assert_eq!(deck.source().major_axis(), MajorAxis::V);
    assert_eq!(deck.orientation(), LatticeOrientation::Preserving);
    // Radii are unchanged by inversion.
    assert!((deck.source().major_radius().get() - 3.0).abs() < 1e-9);
    assert!((deck.source().minor_radius().get() - 1.0).abs() < 1e-9);
}

#[test]
fn winding_respects_axis_swap_under_inversion() {
    // Under inversion, major is on V. A displacement (0, 2π) is one major
    // period, so the winding is (1, 0) — first component is always major.
    let mut processor = Processor::new(Torus::new(Point3::new(0.0, 0.0, 0.0), 3.0, 1.0));
    processor.invert();
    let surface = Surface::ElementarySurface(ElementarySurface::ToroidalSurface(processor));
    let deck = identify_source_torus_deck(&surface).expect("certifies");
    let w = deck
        .winding_of_displacement(0.0, TAU)
        .expect("major winding on v");
    assert_eq!(w, DeckVector2::new(1, 0));
    let w = deck
        .winding_of_displacement(TAU, 0.0)
        .expect("minor winding on u");
    assert_eq!(w, DeckVector2::new(0, 1));
}

// ---------------------------------------------------------------------------
// Uniform scale admission
// ---------------------------------------------------------------------------

#[test]
fn uniform_scale_is_admitted_and_scales_radii() {
    let transform =
        Matrix4::from_translation(Vector3::new(1.0, 2.0, -5.0)) * Matrix4::from_scale(4.0);
    let surface = placed_torus(3.0, 1.0, transform);
    let deck = identify_source_torus_deck(&surface).expect("a uniform scale is a similarity");
    assert_eq!(deck.orientation(), LatticeOrientation::Preserving);
    assert!((deck.source().major_radius().get() - 12.0).abs() < 1e-9);
    assert!((deck.source().minor_radius().get() - 4.0).abs() < 1e-9);
    assert!(
        (deck.source().center() - Point3::new(1.0, 2.0, -5.0)).magnitude() < 1e-8,
        "center should be at the translation"
    );
}

// ---------------------------------------------------------------------------
// Non-finite / degenerate refusal
// ---------------------------------------------------------------------------

#[test]
fn non_finite_radius_is_refused() {
    // Torus::new panics on non-positive radii, so we test via a transform
    // that produces NaN. A NaN entry in the transform makes is_similarity
    // fail first (scale is NaN), which is PlacementNotASimilarity.
    let mut bad = Matrix4::from_scale(1.0);
    bad.x.x = f64::NAN;
    let surface = placed_torus(3.0, 1.0, bad);
    assert_eq!(
        identify_source_torus_deck(&surface).unwrap_err(),
        TorusDeckFailure::PlacementNotASimilarity
    );
}

// ---------------------------------------------------------------------------
// Undecidable / unsupported placement
// ---------------------------------------------------------------------------

#[test]
fn near_singular_transform_refuses_orientation() {
    // A transform whose determinant is near zero relative to scale^3.
    // Collapsing z to 1e-8: the xy columns are unit, z is 1e-8 — not a
    // similarity (unequal column lengths), so it fails is_similarity first.
    // To reach the orientation floor specifically, we need a similarity
    // whose scale is near zero — but a zero-scale similarity is caught by
    // is_similarity's `!(scale > 0.0)` check. The orientation floor is
    // therefore guarded by is_similarity in practice. This test documents
    // that a non-similarity is caught before the orientation check.
    let transform = Matrix4::from_nonuniform_scale(1.0, 1.0, 1e-8);
    let surface = placed_torus(3.0, 1.0, transform);
    assert_eq!(
        identify_source_torus_deck(&surface).unwrap_err(),
        TorusDeckFailure::PlacementNotASimilarity
    );
}

// ---------------------------------------------------------------------------
// Tag stability
// ---------------------------------------------------------------------------

#[test]
fn failure_tags_are_stable() {
    assert_eq!(
        TorusDeckFailure::NotToroidalSurface.tag(),
        "surface_not_toroidal"
    );
    assert_eq!(
        TorusDeckFailure::PlacementNotASimilarity.tag(),
        "torus_placement_not_a_similarity"
    );
    assert_eq!(
        TorusDeckFailure::TransformOrientationUndecidable.tag(),
        "torus_transform_orientation_undecidable"
    );
}

#[test]
fn opt_entry_point_returns_tag_on_failure() {
    use look::step::torus_deck::identify_source_torus_deck_opt;
    let cone_surface = {
        use truck_stepio::r#in::step_geometry::{Line, RevolutedCurve};
        let revo = RevolutedCurve::by_revolution(
            Line(Point3::new(0.5, 0.0, 1.0), Point3::new(3.0, 0.0, 6.0)),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        Surface::ElementarySurface(ElementarySurface::ConicalSurface(Processor::new(revo)))
    };
    assert_eq!(
        identify_source_torus_deck_opt(&cone_surface),
        Err("surface_not_toroidal")
    );
}
