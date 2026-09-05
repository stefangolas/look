//! CC-014-LOFT-VALIDITY integration tests (theory §2.2 L5): the three-valued
//! regularity + self-contact postcondition over the closed-wire strip loft.
//! A regularity margin below the CC-000 floor fails the postcondition as
//! recorded data; non-adjacent overlapping flat facets of a self-retracing
//! loft surface as certified unintended contact through the evidence funnel;
//! the same candidates as Inconclusive — never Certified — under an exhausted
//! budget; and the whole-region graph-disk discharge taking precedence over
//! the pairwise search on a flat single-strip loft. The near-diagonal
//! exclusion is covered in-module (the attempted-pair probe is `#[cfg(test)]`).
//! The test names are the contract.

#![deny(clippy::unwrap_used)]

use truck_base::evidence::Budget;
use truck_certified::certified_map::{admit_surface, CertifiedSurfaceMap};
use truck_certified::construct::config::{CC_DEPTH_MAX, CC_ETA_J};
use truck_certified::construct::loft::LoftOutput;
use truck_certified::construct::loft_strips::LoftStrips;
use truck_certified::construct::loft_validity::{certify_loft_validity, PairVerdict};
use truck_certified::construct::refusal::ConstructRefusal;
use truck_certified::formal::numeric::PositiveFinite;
use truck_geometry::prelude::{BSplineSurface, KnotVec, Point3, Vector4};

/// Extract the `Ok` of a fallible construction; the fixture data is valid by
/// construction, so the refusal arm is a test-bug panic (never an unwrap).
fn construct<T>(result: Result<T, ConstructRefusal>) -> T {
    match result {
        Ok(value) => value,
        Err(refusal) => panic!("a construction that must succeed was refused: {refusal:?}"),
    }
}

/// A homogeneous control point with unit weight (test helper).
fn p4(x: f64, y: f64, z: f64) -> Vector4 {
    Vector4::new(x, y, z, 1.0)
}

/// A declared positive admission tau (the certified-map `PositiveFinite`
/// proof made at the boundary).
fn tau(value: f64) -> PositiveFinite {
    PositiveFinite::new(value).expect("a positive declared tau")
}

/// A bilinear unit-weight strip surface over the given `(u, v)` ranges; the
/// strip surfaces only carry the region geometry of the loft decomposition.
fn strip_surface(u: (f64, f64), v: (f64, f64)) -> BSplineSurface<Vector4> {
    let u_knot = KnotVec::from(vec![u.0, u.0, u.1, u.1]);
    let v_knot = KnotVec::from(vec![v.0, v.0, v.1, v.1]);
    let points = vec![
        vec![p4(u.0, v.0, 0.0), p4(u.0, v.1, 0.0)],
        vec![p4(u.1, v.0, 0.0), p4(u.1, v.1, 0.0)],
    ];
    construct(
        BSplineSurface::try_new((u_knot, v_knot), points)
            .map_err(|_| ConstructRefusal::InvalidInput),
    )
}

/// The single-strip decomposition over the whole unit domain.
fn unit_strips() -> LoftStrips {
    LoftStrips {
        strips: vec![LoftOutput {
            surface: strip_surface((0.0, 1.0), (0.0, 1.0)),
            epsilon: 0.0,
        }],
        seam_ids: Vec::new(),
    }
}

/// The four-strip decomposition of the self-retracing section loft.
fn four_strips() -> LoftStrips {
    let quarters = [(0.0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.0)];
    let strips = quarters
        .iter()
        .map(|&(a, b)| LoftOutput {
            surface: strip_surface((a, b), (0.0, 1.0)),
            epsilon: 0.0,
        })
        .collect();
    LoftStrips {
        strips,
        seam_ids: Vec::new(),
    }
}

/// The bilinear plane `S(u, v) = (u/s, v/s, 0)` over `[0, 1]²` — a flat patch
/// whose certified `|Sᵤ × Sᵥ|` lower bound is `1/s²`.
fn thin_plane(scale: f64) -> BSplineSurface<Point3> {
    let inv = 1.0 / scale;
    let knot = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let points = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, inv, 0.0)],
        vec![Point3::new(inv, 0.0, 0.0), Point3::new(inv, inv, 0.0)],
    ];
    construct(
        BSplineSurface::try_new((knot.clone(), knot), points)
            .map_err(|_| ConstructRefusal::InvalidInput),
    )
}

/// The unit plane `S(u, v) = (u, v, 0)` over `[0, 1]²`.
fn unit_plane() -> BSplineSurface<Point3> {
    let knot = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let points = vec![
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ];
    construct(
        BSplineSurface::try_new((knot.clone(), knot), points)
            .map_err(|_| ConstructRefusal::InvalidInput),
    )
}

/// A flat surface in `z = 0` whose section retraces `x ∈ [0, 2]` four times
/// over `u ∈ [0, 1]`: the section path `0 → 2 → 0 → 2 → 0` over the four
/// quarter spans. Each strip is a flat parallelogram facet in `z = 0`; every
/// facet covers the SAME world rectangle `[0, 2] × [0, 1]`, so non-adjacent
/// strip pairs (e.g. strip 0 and strip 2) overlap in world space — a genuine
/// self-contact of the loft.
fn retracing_loft() -> BSplineSurface<Point3> {
    let u_knot = KnotVec::from(vec![0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0]);
    let v_knot = KnotVec::from(vec![0.0, 0.0, 1.0, 1.0]);
    let xs = [0.0_f64, 2.0, 0.0, 2.0, 0.0];
    let points = xs
        .iter()
        .map(|&x| vec![Point3::new(x, 0.0, 0.0), Point3::new(x, 1.0, 0.0)])
        .collect();
    construct(
        BSplineSurface::try_new((u_knot, v_knot), points)
            .map_err(|_| ConstructRefusal::InvalidInput),
    )
}

/// Admit a surface map with a tiny declared tau (the fixtures are far above
/// the tau floor but may be far below the CC-000 margin floor `CC_ETA_J`).
fn admit(surface: &BSplineSurface<Point3>) -> CertifiedSurfaceMap {
    match admit_surface(surface, tau(5.0e-30)) {
        Ok(map) => map,
        Err(refusal) => panic!("the fixture surface must admit: {refusal:?}"),
    }
}

#[test]
fn regular_margin_below_eta_j_fails_the_postcondition() {
    // A flat patch whose certified margin lower bound `1/s²` lies below the
    // CC-000 regularity floor `CC_ETA_J` (but far above the declared tau).
    // The certify call is Ok (the certificate is PRODUCED); the regularity
    // Result carries the margin ENCLOSURE, whose certified lower bound reads
    // below the floor — the recorded failure data. The self-contact arm is
    // not run on a loft whose regularity postcondition already fails.
    let map = admit(&thin_plane(1.0e7));
    let mut budget = Budget::new(1 << 10, 1 << 10, CC_DEPTH_MAX);
    let cert = construct(certify_loft_validity(&map, &unit_strips(), &mut budget));

    match &cert.regularity {
        Ok(margin) => assert!(
            margin.lo < CC_ETA_J,
            "the certified margin lower bound must read below the J floor"
        ), // H-3: certified bound against the normative floor
        Err(refusal) => {
            panic!("a certifiable margin must be carried as data, not refused: {refusal:?}")
        }
    }
    assert!(
        cert.pairs.is_empty(),
        "the self-contact arm must not run once the regularity postcondition fails"
    );
    assert!(!cert.discharged_by_graphdisk);
}

#[test]
fn far_pair_contact_found_reports_unintended_contact() {
    // The self-retracing loft: every strip is a flat facet in `z = 0` covering
    // the same world rectangle, so the NON-adjacent strip pairs (0, 2) and
    // (1, 3) overlap in world space. Their certified boxes are not separated;
    // the evidence contact funnel certifies the coplanar coincidence of the
    // two lifted plane facets and reports the unintended contact as a
    // `PairVerdict::Contact`.
    let map = admit(&retracing_loft());
    let mut budget = Budget::new(1 << 10, 1 << 10, CC_DEPTH_MAX);
    let cert = construct(certify_loft_validity(&map, &four_strips(), &mut budget));

    match &cert.regularity {
        Ok(margin) => assert!(margin.lo >= CC_ETA_J), // H-3: certified margin above the J floor
        Err(refusal) => panic!("the flat retracing loft must certify a margin: {refusal:?}"),
    }
    assert_eq!(
        cert.pairs.len(),
        2,
        "the two non-adjacent strip pairs are candidates"
    );
    assert!(
        cert.pairs
            .iter()
            .all(|verdict| *verdict == PairVerdict::Contact),
        "every overlapping non-adjacent facet pair reports the unintended contact: {:?}",
        cert.pairs
    );
}

#[test]
fn undecided_pairs_surface_as_inconclusive_never_certified() {
    // The SAME self-retracing loft under an EXHAUSTED budget: the overlapping
    // non-adjacent facet pairs require a funnel decision, the budget ledger is
    // empty, and each candidate pair surfaces as `Inconclusive` — never as
    // `Certified` (Section 3).
    let map = admit(&retracing_loft());
    let mut budget = Budget::new(0, 0, 0);
    let cert = construct(certify_loft_validity(&map, &four_strips(), &mut budget));

    assert!(
        !cert.pairs.is_empty(),
        "the overlapping non-adjacent pairs are candidates"
    );
    assert!(
        cert.pairs
            .iter()
            .all(|verdict| *verdict == PairVerdict::Inconclusive),
        "a budget-exhausted pair is Inconclusive, never Certified: {:?}",
        cert.pairs
    );
    assert!(
        !cert.pairs.contains(&PairVerdict::Certified),
        "no budget-exhausted pair may certify"
    );
}

#[test]
fn graphdisk_discharge_takes_precedence_over_pairwise_search() {
    // A single flat strip over the unit plane: the whole region certifies as
    // one affine parallelogram, so the graph-disk discharge (regime b) fires
    // BEFORE the pairwise search and the within-region search never runs:
    // the certificate records the graph-disk discharge and no pair verdicts.
    let map = admit(&unit_plane());
    let mut budget = Budget::new(1 << 10, 1 << 10, CC_DEPTH_MAX);
    let cert = construct(certify_loft_validity(&map, &unit_strips(), &mut budget));

    match &cert.regularity {
        Ok(margin) => assert!(margin.lo >= CC_ETA_J), // H-3: certified margin above the J floor
        Err(refusal) => panic!("the unit plane must certify a margin: {refusal:?}"),
    }
    assert!(
        cert.discharged_by_graphdisk,
        "the flat single-strip region must discharge through the graph-disk decider"
    );
    assert!(
        cert.pairs.is_empty(),
        "no pairwise search runs on the graph-disk-discharged region"
    );
}
