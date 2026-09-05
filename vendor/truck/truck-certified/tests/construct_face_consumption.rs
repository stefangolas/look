//! CC-032-FACE-CONSUMPTION integration tests (spine seam S12 consumer): the
//! trim arrangement decides what survives the blend. The ordinary case — one
//! contact pcurve splits the support domain into exactly its two cells, one
//! of them removed; the short intermediate face A-B-C whose cells all fall on
//! the blend side and vanish with no special-case code; a surviving cell's
//! trim provenance (which contact curve, which side); and the empty-retained-
//! cell branch reporting `Vanished`. The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_certified::certified_map::SurfaceRegion;
use truck_certified::construct::face_consumption::{
    classify_cells, consume_face, BlendSide, ContactPcurve, FaceConsumption, FaceOutcome,
    RetainedCell, SourceRef, SupportDescription, TrimBound, TrimSide,
};
use truck_certified::construct::refusal::ConstructRefusal;

/// The full certified parameter region of the fixtures.
const REGION: SurfaceRegion = ((0.0, 1.0), (0.0, 1.0));

/// A support description over the unit chart.
fn support() -> SupportDescription {
    SupportDescription {
        domain: REGION,
        source: SourceRef { id: 1 },
    }
}

/// A landed provenance reference.
fn src(id: u64) -> SourceRef {
    SourceRef { id }
}

/// A full vertical contact chord `u = const` across the unit chart.
fn vchord(coord: f64, blend_side: BlendSide) -> ContactPcurve {
    ContactPcurve {
        from: [coord, 0.0],
        to: [coord, 1.0],
        blend_side,
    }
}

/// A face consumption over the unit chart with one provenance id per pcurve.
fn consumption(pcurves: Vec<ContactPcurve>, ids: Vec<u64>) -> FaceConsumption {
    FaceConsumption {
        support: support(),
        trim_provenance: ids.into_iter().map(src).collect(),
        contact_pcurves: pcurves,
    }
}

/// Destructure a `Survived` outcome; a different outcome here is a test-bug
/// panic.
fn expect_survived(result: Result<FaceOutcome, ConstructRefusal>) -> Vec<RetainedCell> {
    match result {
        Ok(FaceOutcome::Survived { retained }) => retained,
        Ok(FaceOutcome::Vanished) => panic!("a face that must survive vanished"),
        Err(refusal) => panic!("a face consumption that must succeed refused: {refusal:?}"),
    }
}

#[test]
fn contact_pcurve_splits_the_support_domain() {
    // One vertical contact chord at `u = 0.5` with the blend side Upper (the
    // removed region `R_i` is the right half) splits the unit chart into
    // EXACTLY the two expected cells: the left half `[0, 0.5] x [0, 1]` is
    // retained and the right half `[0.5, 1] x [0, 1]` is removed — the
    // ordinary arrangement, nothing special-cased.
    let fc = consumption(vec![vchord(0.5, BlendSide::Upper)], vec![7]);
    let cells = classify_cells(&fc).expect("the single-chord face classifies");

    assert_eq!(
        cells.len(),
        2,
        "the chord splits the domain into exactly two cells"
    );
    let retained = cells.iter().filter(|cell| !cell.removed).count();
    let removed = cells.iter().filter(|cell| cell.removed).count();
    assert_eq!(retained, 1, "exactly one surviving cell");
    assert_eq!(removed, 1, "exactly one removed cell");

    // The fixed enumeration order is `v` bands then `u` slabs, low to high:
    // the left cell first, then the right cell.
    assert_eq!(cells[0].cell.u.0, 0.0); // H-3: exact chart boundary cut
    assert_eq!(cells[0].cell.u.1, 0.5); // H-3: exact chord cut coordinate
    assert_eq!(cells[1].cell.u.0, 0.5); // H-3: exact chord cut coordinate
    assert_eq!(cells[1].cell.u.1, 1.0); // H-3: exact chart boundary cut
    assert_eq!(cells[0].cell.v, (0.0, 1.0)); // H-3: exact chart span
    assert!(!cells[0].removed, "the left cell survives the blend side");
    assert!(
        cells[1].removed,
        "the right cell is inside the removed region"
    );

    // The face survives with exactly the left half as its retained cell.
    let retained_cells = expect_survived(consume_face(&fc));
    assert_eq!(retained_cells.len(), 1);
    assert_eq!(retained_cells[0].cell.u.0, 0.0); // H-3: exact surviving-cell boundary
    assert_eq!(retained_cells[0].cell.u.1, 0.5); // H-3: exact surviving-cell boundary
    assert_eq!(retained_cells[0].cell.v, (0.0, 1.0)); // H-3: exact chart span
    assert_eq!(
        retained_cells[0].bounds.len(),
        1,
        "the surviving cell is bounded by the single contact chord"
    );
}

#[test]
fn short_intermediate_face_is_fully_consumed() {
    // The A-B-C ground truth: the intermediate face B carries two contact
    // pcurves — the projections of the two blend chains that reach the A/B/C
    // triple node and depart. B is so short that the two removed strips
    // overlap across the whole chart: the left chain trims up to `u = 0.55`
    // (blend side Lower) and the right chain trims from `u = 0.45` (blend
    // side Upper), so no cell lies on the surviving side of both. The ORDINARY
    // arrangement marks every cell removed and the empty-retained branch falls
    // out — B retains NO cell and vanishes, with no special-case code.
    let fc = consumption(
        vec![
            vchord(0.55, BlendSide::Lower),
            vchord(0.45, BlendSide::Upper),
        ],
        vec![8, 9],
    );
    let cells = classify_cells(&fc).expect("the intermediate face classifies");

    assert_eq!(cells.len(), 3, "two chords leave three cells of the chart");
    for cell in &cells {
        assert!(
            cell.removed,
            "every cell of the short intermediate face is removed"
        );
    }

    match consume_face(&fc) {
        Ok(FaceOutcome::Vanished) => {}
        Ok(FaceOutcome::Survived { retained }) => {
            panic!("the intermediate face must vanish, not survive with {retained:?}")
        }
        Err(refusal) => panic!("the intermediate face consumption refused: {refusal:?}"),
    }
}

#[test]
fn surviving_face_carries_trim_provenance() {
    // A face between two non-overlapping blend strips: the left chain trims
    // up to `u = 0.25` (blend side Lower) and the right chain trims from
    // `u = 0.75` (blend side Upper). The middle cell `[0.25, 0.75] x [0, 1]`
    // survives, and it records its trim provenance for the edit graph: WHICH
    // contact curve bounds it and ON WHICH SIDE the cell lies — curve 0 on
    // its Upper side and curve 1 on its Lower side, each with its landed
    // provenance reference.
    let fc = consumption(
        vec![
            vchord(0.25, BlendSide::Lower),
            vchord(0.75, BlendSide::Upper),
        ],
        vec![11, 12],
    );
    let retained_cells = expect_survived(consume_face(&fc));

    assert_eq!(retained_cells.len(), 1, "exactly the middle cell survives");
    let middle = &retained_cells[0];
    assert_eq!(middle.cell.u.0, 0.25); // H-3: exact chord cut coordinate
    assert_eq!(middle.cell.u.1, 0.75); // H-3: exact chord cut coordinate
    assert_eq!(middle.cell.v, (0.0, 1.0)); // H-3: exact chart span

    let expected = vec![
        TrimBound {
            curve: 0,
            side: TrimSide::Upper,
            source: src(11),
        },
        TrimBound {
            curve: 1,
            side: TrimSide::Lower,
            source: src(12),
        },
    ];
    assert_eq!(
        middle.bounds, expected,
        "the surviving cell records its trim provenance"
    );

    // The two outer cells are removed and carry no retained-cell record.
    let cells = classify_cells(&fc).expect("the two-chord face classifies");
    assert_eq!(cells.len(), 3);
    assert!(cells[0].removed && !cells[1].removed && cells[2].removed);
}

#[test]
fn face_with_no_retained_cell_vanishes() {
    // A face that lies entirely inside the removed region: the two blend
    // strips cover the whole chart (the left strip removed side is Upper from
    // `u = 0.2`, the right strip's removed side is Lower from `u = 0.8`; the
    // strips overlap across the entire face). Every cell of the ordinary
    // arrangement is marked removed, so the face has no retained cell and
    // consumes to `Vanished` — never an empty `Survived`.
    let fc = consumption(
        vec![vchord(0.2, BlendSide::Upper), vchord(0.8, BlendSide::Lower)],
        vec![3, 4],
    );
    let cells = classify_cells(&fc).expect("the fully consumed face classifies");
    assert_eq!(cells.len(), 3);
    assert!(cells.iter().all(|cell| cell.removed), "no cell is retained");

    match consume_face(&fc) {
        Ok(FaceOutcome::Vanished) => {}
        Ok(FaceOutcome::Survived { retained }) => {
            panic!("a face with no retained cell must vanish, not survive with {retained:?}")
        }
        Err(refusal) => panic!("the fully consumed face consumption refused: {refusal:?}"),
    }
}
