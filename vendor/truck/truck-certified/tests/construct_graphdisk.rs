//! CC-005-GRAPHDISK conformance tests (spine seam S6): the P3 graph-disk
//! decision table over the CC-000 fixture ground truths, the two refusal
//! paths, the frozen normative projection-candidate sequence and the exact
//! 14-point spherical-code table, and the projected-boundary simplicity
//! discharge (planar exclusion + near-diagonal P2 radius). The test names are
//! the contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::{admit_curve, CertifiedCurveMap};
use truck_certified::construct::fixtures as fx;
use truck_certified::construct::graphdisk::{
    certify_graph_disk, projected_boundary_simplicity, projection_candidates, search_projection,
    AdmittedPiece, BoundaryArc, DiskPiece, SPHERICAL_CODE_14,
};
use truck_certified::construct::injectivity::curve_injectivity_radius;
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::construct::stubs::BoundaryPlan;
use truck_certified::construct::Interval;
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineCurve, KnotVec, Point3};

/// A certified interval from explicit lo/hi bounds (test-local helper).
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// Map the §6 `GraphDiskFixture` records onto the CC-005 `DiskPiece` shape.
fn fixture_pieces(fixture: &fx::GraphDiskFixture) -> Vec<DiskPiece> {
    fixture
        .pieces
        .iter()
        .map(|record| DiskPiece {
            det_lower: record.det_lower,
            seam_glued: record.seam_glued,
            boundary_simple: record.boundary_simple,
        })
        .collect()
}

/// A declared positive tau for the certified curve fixtures.
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// The certified map of one straight projected-boundary segment (a flat line
/// in the projection plane), used to compute the near-diagonal radius through
/// the real CC-002 `curve_injectivity_radius`.
fn segment_map(p0: Point3, p1: Point3) -> CertifiedCurveMap {
    let knot = KnotVec::bezier_knot(1);
    let points = vec![p0, p1];
    let curve = BSplineCurve::new(knot, points);
    admit_curve(&curve, tau(0.5)).expect("the line segment admits")
}

#[test]
fn genuine_star_certifies() {
    // Fixture 8: every determinant lower bound strictly positive, every seam
    // glued, every boundary simple — with a simple boundary plan the DECIDER
    // must return the witness records unchanged.
    let star = construct(fx::genuine_star());
    let pieces = fixture_pieces(&star);
    let plan = BoundaryPlan {
        boundary_simple: true,
        seams_glued: true,
    };
    let cert = match certify_graph_disk(&pieces, &plan) {
        Ok(cert) => cert,
        Err(refusal) => panic!("the genuine star must certify, refused: {refusal:?}"),
    };
    assert_eq!(
        cert.pieces.len(),
        2,
        "both witness records survive certification"
    );
    assert_eq!(
        cert.pieces, pieces,
        "the certificate mirrors the certified pieces"
    );
    for piece in &cert.pieces {
        assert!(
            piece.det_lower.lo > 0.0,
            "piece determinant strictly positive"
        ); // H-3: fixture inf bounds 1.0/0.5
        assert!(piece.seam_glued, "the star seams are glued");
        assert!(piece.boundary_simple, "the star boundaries are simple");
    }
}

#[test]
fn folded_corner_refuses_no_admissible_projection_or_star_not_embedded() {
    // Fixture 9: a constructed fold — opposite-signed determinant lower bounds
    // and a non-simple fold boundary. The decision table refuses on the first
    // failing clause: the fold's negative determinant fires
    // NoAdmissibleProjection before the non-simple boundary could fire
    // StarNotEmbedded; the contract accepts either refusal variant.
    let fold = construct(fx::folded_corner());
    let pieces = fixture_pieces(&fold);
    assert!(fold.sign_change, "the fold fixture records a sign change");
    let mut saw_negative = false;
    let mut saw_not_simple = false;
    for piece in &pieces {
        if piece.det_lower.hi < 0.0 {
            saw_negative = true;
        }
        if !piece.boundary_simple {
            saw_not_simple = true;
        }
    }
    assert!(
        saw_negative && saw_not_simple,
        "the fold ground truth is present"
    );
    let plan = BoundaryPlan {
        boundary_simple: true,
        seams_glued: true,
    };
    match certify_graph_disk(&pieces, &plan) {
        Err(ConstructRefusal::NoAdmissibleProjection) => {}
        Err(ConstructRefusal::StarNotEmbedded) => {}
        Ok(cert) => panic!("a folded corner must refuse, certified: {cert:?}"),
        Err(refusal) => panic!("a folded corner refuses the graph-disk path, got: {refusal:?}"),
    }
}

#[test]
fn non_simple_boundary_refuses_star_not_embedded() {
    // Every piece certifies (positive determinants, glued seams) but the
    // projected boundary is NOT simple: the plan verdict gates the embedding,
    // and the DECIDER refuses StarNotEmbedded.
    let star = construct(fx::genuine_star());
    let pieces = fixture_pieces(&star);
    let plan = BoundaryPlan {
        boundary_simple: false,
        seams_glued: true,
    };
    match certify_graph_disk(&pieces, &plan) {
        Err(ConstructRefusal::StarNotEmbedded) => {}
        Ok(cert) => panic!("a non-simple boundary must refuse, certified: {cert:?}"),
        Err(refusal) => {
            panic!("the non-simple boundary path refuses StarNotEmbedded, got: {refusal:?}")
        }
    }
}

#[test]
fn unglued_seam_refuses_star_not_embedded() {
    // Every determinant lower bound is strictly positive and the plan boundary
    // is simple, but one piece's seam is not glued: the seam clause is NOT
    // implied by the per-piece determinants, so the DECIDER refuses
    // StarNotEmbedded.
    let star = construct(fx::genuine_star());
    let mut pieces = fixture_pieces(&star);
    pieces[1].seam_glued = false;
    let plan = BoundaryPlan {
        boundary_simple: true,
        seams_glued: true,
    };
    match certify_graph_disk(&pieces, &plan) {
        Err(ConstructRefusal::StarNotEmbedded) => {}
        Ok(cert) => panic!("an unglued seam must refuse, certified: {cert:?}"),
        Err(refusal) => panic!("the unglued-seam path refuses StarNotEmbedded, got: {refusal:?}"),
    }
}

#[test]
fn projection_candidate_order_is_the_normative_sequence() {
    // Two admitted pieces whose normals are tilted so that no candidate of the
    // first two families certifies: the area-weighted average normal
    // (1, 0, 0) is orthogonal to the +Z piece, and both principal net
    // directions fail one piece. The first certifying candidate must be
    // SPHERICAL_CODE_14 entry 6 — the (1, 1, 1)/√3 diagonal — proving the
    // search walks the FROZEN sequence family by family and returns the first
    // admissible direction.
    let pieces = vec![
        AdmittedPiece {
            normal_box: [iv(0.0, 0.0), iv(0.0, 0.0), iv(1.0, 1.0)],
            area_lower: 1.0,
            net_u: [0.0, 0.0, 1.0],
            net_v: [0.0, 0.0, -1.0],
            seam_glued: true,
        },
        AdmittedPiece {
            normal_box: [iv(2.0, 2.0), iv(0.0, 0.0), iv(-1.0, -1.0)],
            area_lower: 1.0,
            net_u: [0.0, 0.0, 1.0],
            net_v: [0.0, 0.0, -1.0],
            seam_glued: true,
        },
    ];

    // The frozen sequence: family (1) = the area-weighted average normal,
    // family (2) = the two principal control-net directions, then the full
    // 14-point spherical-code table in table order.
    let candidates = projection_candidates(&pieces);
    assert_eq!(
        candidates.len(),
        17,
        "one + two + fourteen normative candidates"
    );
    assert_eq!(candidates[0], [1.0, 0.0, 0.0]); // H-3: normalized area-weighted average normal
    assert_eq!(candidates[1], [0.0, 0.0, 1.0]); // H-3: mean net-u principal direction
    assert_eq!(candidates[2], [0.0, 0.0, -1.0]); // H-3: mean net-v principal direction
    for (index, point) in SPHERICAL_CODE_14.iter().enumerate() {
        assert_eq!(
            candidates[3 + index],
            *point,
            "code entry {index} follows the families in table order"
        );
    }

    // The search walks the candidates in order, consulting the projected
    // boundary verdict once per candidate until the first certifying one.
    let mut queried: Vec<[f64; 3]> = Vec::new();
    let (w, disks) = search_projection(&pieces, |candidate| {
        queried.push(candidate);
        Ok(true)
    })
    .expect("the (1,1,1)/sqrt(3) diagonal certifies the two pieces");

    // Candidates 0..=2 (the two families) and code entries 0..=5 fail the
    // strictly-positive determinant clause; entry 6 is the first success.
    assert_eq!(
        queried.len(),
        10,
        "three family candidates + seven code entries walked"
    );
    for (index, candidate) in queried.iter().enumerate() {
        assert_eq!(
            *candidate, candidates[index],
            "candidate {index} queried in normative order"
        );
    }
    assert_eq!(
        w, SPHERICAL_CODE_14[6],
        "the winning projection is the first certifying code entry"
    );
    assert_eq!(disks.len(), 2, "one DiskPiece per admitted piece");
    for piece in &disks {
        assert!(
            piece.det_lower.lo > 0.0,
            "every winning piece certifies a positive determinant"
        ); // H-3: strict positivity of the certified lower bound
        assert!(
            piece.seam_glued,
            "the winning pieces keep their glued seams"
        );
        assert!(
            piece.boundary_simple,
            "the winning pieces certify a simple boundary"
        );
    }
}

#[test]
fn projected_boundary_simplicity_uses_planar_exclusion_and_near_diagonal_radius() {
    // A closed unit-square boundary of four flat projected segments. The
    // near-diagonal radius of every arc is the certified P2 radius of its
    // plane-projected segment (a flat curve: delta = +infinity), computed
    // through the real CC-002 `curve_injectivity_radius`; the non-adjacent
    // pairs (arc 0/arc 2, arc 1/arc 3) are planar-excluded by their boxes.
    let radii = [
        curve_injectivity_radius(
            &segment_map(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
            (0.0, 1.0),
        )
        .expect("the bottom segment certifies its radius"),
        curve_injectivity_radius(
            &segment_map(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
            (0.0, 1.0),
        )
        .expect("the right segment certifies its radius"),
        curve_injectivity_radius(
            &segment_map(Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
            (0.0, 1.0),
        )
        .expect("the top segment certifies its radius"),
        curve_injectivity_radius(
            &segment_map(Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
            (0.0, 1.0),
        )
        .expect("the left segment certifies its radius"),
    ];
    let square = |radii: &[Interval; 4]| -> Vec<BoundaryArc> {
        vec![
            BoundaryArc {
                planar_box: [iv(0.0, 1.0), iv(0.0, 0.0)],
                radius: radii[0],
                start: 0,
                end: 1,
            },
            BoundaryArc {
                planar_box: [iv(1.0, 1.0), iv(0.0, 1.0)],
                radius: radii[1],
                start: 1,
                end: 2,
            },
            BoundaryArc {
                planar_box: [iv(0.0, 1.0), iv(1.0, 1.0)],
                radius: radii[2],
                start: 2,
                end: 3,
            },
            BoundaryArc {
                planar_box: [iv(0.0, 0.0), iv(0.0, 1.0)],
                radius: radii[3],
                start: 3,
                end: 0,
            },
        ]
    };
    let clean = square(&radii);
    assert_eq!(
        projected_boundary_simplicity(&clean),
        Ok(true),
        "the clean square boundary discharges simple"
    );

    // Planar-exclusion clause: move the top arc's box so it overlaps the
    // bottom arc's box in both axes — the two non-adjacent arcs are no longer
    // separated, so the discharge refutes simplicity.
    let mut touching = square(&radii);
    touching[2].planar_box = [iv(0.0, 1.0), iv(0.0, 0.5)]; // H-3: intrusion box overlapping the bottom arc on both axes
    assert_eq!(
        projected_boundary_simplicity(&touching),
        Ok(false),
        "a non-adjacent planar-box overlap refutes simplicity"
    );

    // Near-diagonal-radius clause: keep the planar exclusion but give the top
    // arc a radius whose lower bound is not strictly positive (a fold arc);
    // the discharge refutes simplicity even though the boxes separate.
    let mut fold = square(&radii);
    fold[2].radius = iv(0.0, 0.5); // H-3: fold radius touching zero at its lower endpoint
    assert_eq!(
        projected_boundary_simplicity(&fold),
        Ok(false),
        "a non-positive near-diagonal radius refutes simplicity"
    );

    // The discharge refuses structurally corrupt input.
    let mut corrupt = square(&radii);
    corrupt[0].planar_box[0] = iv(f64::NAN, 1.0);
    match projected_boundary_simplicity(&corrupt) {
        Err(ConstructRefusal::InvalidInput) => {}
        Ok(simple) => panic!("a non-finite planar box must refuse, discharged: {simple}"),
        Err(refusal) => panic!("the corrupt boundary refuses InvalidInput, got: {refusal:?}"),
    }
}
