//! CC-022-STARS integration tests (spine seam S6 consumer; theory §4.1 stars,
//! §4.3 broad phase): the closed-star embedding certificate over the CC-021
//! constructed strata (a two-plane wedge star certifies embedded, a star with
//! a folded piece refuses `StarNotEmbedded`, and a glue-seam identity
//! mismatch refuses `StarNotEmbedded` BEFORE the graph-disk reduction runs)
//! and the certified reach-bound broad phase (a disjoint pair never enters
//! the candidate list; a close pair is retained). The test names are the
//! contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::graphdisk::GraphDiskCert;
use truck_certified::construct::offset_strata::{face_stratum, OffsetStratum};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stars::{
    certify_star, reach_prune, BoundaryRef, FaceSide, Glue, GluePlan, SharedBoundary, Star,
};
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3};

/// A declared positive tau for the certified fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// Admit a surface fixture with the given declared tau, panicking only on a
/// test-bug refusal.
fn admitted(surface: &BSplineSurface<Point3>, value: f64) -> CertifiedSurfaceMap {
    admit_surface(surface, tau(value)).expect("the surface fixture admits")
}

/// The flat unit-square plane `(u, v, 0)` over `[0, 1]^2` (normal `+z`).
fn z_plane() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// The flat plane `(u + shift, v, 0)` over `[0, 1]^2`: the `z_plane` shifted
/// along `x` by `shift`, so its source bounding box is exactly
/// `[shift, shift + 1] × [0, 1] × [0, 0]`.
fn shifted_plane(shift: f64) -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(shift, 0.0, 0.0), Point3::new(shift, 1.0, 0.0)],
        vec![
            Point3::new(shift + 1.0, 0.0, 0.0),
            Point3::new(shift + 1.0, 1.0, 0.0),
        ],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// The flat plane `(1 - u, v, 0)` over `[0, 1]^2`: the `z_plane` with the `u`
/// axis reversed (area normal `-z`) — the mirror fold used to build a star
/// whose pieces double back over each other.
fn reversed_z_plane() -> BSplineSurface<Point3> {
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// The tilted plane `(u, -(1 - v)·c, (1 - v)·s)` over `[0, 1]^2` with
/// `(c, s) = (cos φ, sin φ)`, φ = 45°: the second leaf of the open-book
/// wedge. It shares the `u`-axis crease `(u, 0, 0)` (its `v = 1` side) with
/// the `z_plane`'s `v = 0` side and sweeps the `y < 0` half-space rising with
/// `z`; its area normal is `(0, s, c)` (a positive `z` component, like the
/// `z_plane`'s `+z`).
fn tilted_leaf() -> BSplineSurface<Point3> {
    let c = 0.7071067811865476;
    let s = 0.7071067811865476;
    let uknot = KnotVec::bezier_knot(1);
    let vknot = KnotVec::bezier_knot(1);
    let ctrl = vec![
        vec![Point3::new(0.0, -c, s), Point3::new(0.0, 0.0, 0.0)],
        vec![Point3::new(1.0, -c, s), Point3::new(1.0, 0.0, 0.0)],
    ];
    BSplineSurface::new((uknot, vknot), ctrl)
}

/// One face stratum over an admitted surface at the given offset.
fn face(map: &CertifiedSurfaceMap, offset: f64) -> OffsetStratum {
    face_stratum(map, offset).expect("the flat face stratum certifies at this offset")
}

/// The shared-boundary identity of the wedge crease.
const CREASE: SharedBoundary = SharedBoundary::new(7);

/// The two-plane wedge star: the `z_plane` leaf (stratum 0) and the tilted
/// leaf (stratum 1) glued along the shared crease — the `z_plane`'s `v = 0`
/// side to the tilted leaf's `v = 1` side — both referencing the crease
/// identity.
fn wedge_star() -> Star {
    let z = face(&admitted(&z_plane(), 0.5), 0.0);
    let tilt = face(&admitted(&tilted_leaf(), 0.5), 0.0);
    let seams = vec![Glue {
        a: BoundaryRef {
            stratum: 0,
            side: FaceSide::VMin,
            boundary: CREASE,
        },
        b: BoundaryRef {
            stratum: 1,
            side: FaceSide::VMax,
            boundary: CREASE,
        },
    }];
    Star {
        strata: vec![z, tilt],
        glue_plan: GluePlan { seams },
    }
}

/// The wedge seam glue, but with the two sides disagreeing on the shared
/// boundary's identity (a caller-inconsistent plan).
fn mismatched_wedge_star() -> Star {
    let z = face(&admitted(&z_plane(), 0.5), 0.0);
    let tilt = face(&admitted(&tilted_leaf(), 0.5), 0.0);
    let seams = vec![Glue {
        a: BoundaryRef {
            stratum: 0,
            side: FaceSide::VMin,
            boundary: SharedBoundary::new(1),
        },
        b: BoundaryRef {
            stratum: 1,
            side: FaceSide::VMax,
            boundary: SharedBoundary::new(2),
        },
    }];
    Star {
        strata: vec![z, tilt],
        glue_plan: GluePlan { seams },
    }
}

/// The folded star: the `z_plane` leaf (stratum 0, normal `+z`) glued to the
/// reversed `z_plane` leaf (stratum 1, normal `-z`) along their `v = 0`
/// sides. The two pieces occupy the same unit square with opposite
/// orientation — a doubled sheet — so no projection can give both pieces a
/// strictly positive determinant: the star is not embedded.
fn folded_star() -> Star {
    let z = face(&admitted(&z_plane(), 0.5), 0.0);
    let fold = face(&admitted(&reversed_z_plane(), 0.5), 0.0);
    let seams = vec![Glue {
        a: BoundaryRef {
            stratum: 0,
            side: FaceSide::VMin,
            boundary: CREASE,
        },
        b: BoundaryRef {
            stratum: 1,
            side: FaceSide::VMin,
            boundary: CREASE,
        },
    }];
    Star {
        strata: vec![z, fold],
        glue_plan: GluePlan { seams },
    }
}

/// Extract the `Ok` certificate; a refusal here is a test-bug panic.
fn cert_of(star: &Star) -> GraphDiskCert {
    match certify_star(star) {
        Ok(cert) => cert,
        Err(refusal) => panic!("the star must certify embedded, refused: {refusal:?}"),
    }
}

/// Assert the star refuses with exactly `StarNotEmbedded`; anything else is a
/// test-bug panic.
fn refuses_star_not_embedded(star: &Star) {
    match certify_star(star) {
        Err(ConstructRefusal::StarNotEmbedded) => {}
        Ok(cert) => panic!("the star must refuse StarNotEmbedded, certified: {cert:?}"),
        Err(refusal) => panic!("expected StarNotEmbedded, got: {refusal:?}"),
    }
}

#[test]
fn two_plane_wedge_star_certifies_embedded() {
    // The two-plane wedge star: two flat leaves glued along their shared
    // crease, their normals both with a positive `z` component. The frozen
    // candidate sequence's first member — the area-weighted average normal —
    // certifies every piece's projected determinant strictly positive, the
    // seam identities agree, and the outer rim discharges simple under that
    // projection: certify_star returns the per-piece witness records.
    let star = wedge_star();
    let cert = cert_of(&star);
    assert_eq!(cert.pieces.len(), 2, "one witness record per face stratum");
    for (index, piece) in cert.pieces.iter().enumerate() {
        assert!(
            piece.det_lower.lo > 0.0,
            "piece {index} projects with a strictly positive determinant"
        ); // H-3: winning-projection determinant margin (wedge normals share +z)
        assert!(piece.seam_glued, "piece {index} seam is glued");
        assert!(
            piece.boundary_simple,
            "piece {index} boundary is certified simple"
        );
    }
    for stratum in &star.strata {
        assert_eq!(stratum.reach_bound(), 0.0); // H-3: wedge leaves sit at offset 0
    }
}

#[test]
fn star_with_folded_piece_refuses_star_not_embedded() {
    // The folded star: the second piece is the first piece's mirror (area
    // normal `-z`), lying back over the same square. Every seam identity
    // agrees, but no projection admits both pieces with a strictly positive
    // determinant — the glued fan is not an embedded graph star, and
    // certify_star refuses StarNotEmbedded.
    let star = folded_star();
    refuses_star_not_embedded(&star);
}

#[test]
fn reach_pruning_disjoint_pair_never_enters_candidate_list() {
    // Two flat face strata at offset |t| = 0.5, source boxes separated by an
    // axis gap of 5.0 in `x`. The certified reach sum is 1.0, so the CC-004
    // axis-gap bound certifies the realizations disjoint (5.0 > 1.0): the
    // pair never enters the candidate list.
    let a = face(&admitted(&z_plane(), 0.5), 0.5);
    let b = face(&admitted(&shifted_plane(6.0), 0.5), 0.5);
    let gap = 6.0 - 1.0; // H-3: exact source-box axis gap 5.0
    let reach_sum = a.reach_bound() + b.reach_bound(); // H-3: exact reach sum 1.0
    assert!(
        gap > reach_sum,
        "the disjoint separation certifies the prune"
    );
    let candidates = reach_prune(&[a, b]);
    assert!(
        candidates.is_empty(),
        "a disjoint pair never enters the candidate list: {candidates:?}"
    );
}

#[test]
fn reach_pruning_close_pair_is_retained() {
    // Two flat face strata at offset |t| = 0.5, source boxes separated by an
    // axis gap of 0.5 in `x`. The certified reach sum is 1.0, so the axis-gap
    // bound cannot certify the realizations disjoint (0.5 <= 1.0): the close
    // pair is retained and goes to the CC-023 contact funnel.
    let a = face(&admitted(&z_plane(), 0.5), 0.5);
    let b = face(&admitted(&shifted_plane(1.5), 0.5), 0.5);
    let gap = 1.5 - 1.0; // H-3: exact source-box axis gap 0.5
    let reach_sum = a.reach_bound() + b.reach_bound(); // H-3: exact reach sum 1.0
    assert!(
        gap <= reach_sum,
        "the close separation does not certify a prune"
    );
    assert_eq!(reach_prune(&[a, b]), vec![(0, 1)]);
}

#[test]
fn glue_seam_mismatch_refuses_before_graphdisk() {
    // The SAME two-plane wedge whose geometry certifies embedded (test 1),
    // but whose glue plan names two DIFFERENT identities for the shared
    // crease on the two sides. The pre-made glue gate runs BEFORE the
    // graph-disk reduction: had the projection search run on this geometry it
    // would have certified, so the StarNotEmbedded refusal proves the gate
    // fired first.
    let star = mismatched_wedge_star();
    refuses_star_not_embedded(&star);
}
