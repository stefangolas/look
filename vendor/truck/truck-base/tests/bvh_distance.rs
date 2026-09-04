//! CC-004-CLEAR Section 1 — the additive BVH distance queries
//! (`distance_lower_bound`, `distance_lower_bound_self`).
//!
//! The pieces are degenerate point pieces, so a piece's box is exactly its
//! point and the brute-force minimum over all point pairs is the ground truth.
//! All coordinates are dyadic so every box bound is exact in `f64`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use truck_base::bounding_box::BoundingBox;
use truck_base::bvh::{BoundedPiece, Bvh, DerivativeBounds};
use truck_base::cgmath64::Point3;

/// A degenerate point piece: its bounding box is exactly its position.
#[derive(Clone, Copy, Debug)]
struct PointPiece {
    p: Point3,
}

impl BoundedPiece for PointPiece {
    fn bbox(&self) -> BoundingBox<Point3> {
        let mut b = BoundingBox::new();
        b.push(self.p);
        b
    }
    fn derivative_bounds(&self) -> DerivativeBounds {
        DerivativeBounds::new()
    }
    fn subdivide(&self) -> Vec<Self> {
        Vec::new()
    }
}

/// A deterministic LCG so a failure is reproducible.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A dyadic point in `[0, 8]^3` from the LCG stream.
fn lcg_point(state: &mut u64) -> Point3 {
    let x = ((lcg_next(state) % 65) as f64) / 8.0;
    let y = ((lcg_next(state) % 65) as f64) / 8.0;
    let z = ((lcg_next(state) % 65) as f64) / 8.0;
    Point3::new(x, y, z)
}

/// Euclidean distance between two points.
fn distance(a: Point3, b: Point3) -> f64 {
    let d = b - a;
    (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
}

/// Brute-force minimum distance between every point of `a` and every point of
/// `b`.
fn brute_min(a: &[PointPiece], b: &[PointPiece]) -> f64 {
    let mut best = f64::INFINITY;
    for pa in a {
        for pb in b {
            let d = distance(pa.p, pb.p);
            if d < best {
                best = d;
            }
        }
    }
    best
}

/// Float slack on the unit-scale distance bounds (a comparison slack, never a
/// model length).
const SLACK: f64 = 1.0e-9; // H-3: float slack on unit-scale distance bounds, not a length

#[test]
fn distance_lower_bound_never_exceeds_true_min_distance() {
    // Two 24-piece point sets that share one region, so the trees' root boxes
    // overlap and the dual traversal (not the disjoint-root sentinel) decides.
    // `b` is `a` shifted by a quarter unit per axis, so the true minimum is
    // positive and the sets are interleaved over the same extent.
    let mut state = 0x9E37_79B9_7F4A_7C15;
    let a: Vec<PointPiece> = (0..24)
        .map(|_| PointPiece {
            p: lcg_point(&mut state),
        })
        .collect();
    let mut state_b = 0x0123_4567_89AB_CDEF;
    let b: Vec<PointPiece> = (0..24)
        .map(|_| {
            let p = lcg_point(&mut state_b);
            PointPiece {
                p: Point3::new(p.x + 0.25, p.y + 0.25, p.z + 0.25),
            }
        })
        .collect();

    let bvh_a = Bvh::build(&a);
    let bvh_b = Bvh::build(&b);
    let brute = brute_min(&a, &b);

    let got = bvh_a.distance_lower_bound(&bvh_b);
    assert!(got >= 0.0, "a distance bound is never negative, got {got}"); // H-3: lower bound against zero, not a length
    assert!(
        got.is_finite(),
        "overlapping roots must give a finite bound"
    );
    assert!(
        got <= brute + SLACK, // H-3: certified lower bound vs brute-force minimum with float slack
        "bound {got} exceeds the true minimum {brute}"
    );

    // Deterministic: identical inputs produce an identical bound.
    let again = bvh_a.distance_lower_bound(&bvh_b);
    assert_eq!(got, again, "the distance bound must be deterministic"); // H-3: bit-identical repeats of one certified bound
}

#[test]
fn distance_lower_bound_is_infinite_for_disjoint_piece_sets() {
    // A cluster in [0, 8]^3 and a second cluster shifted by 1000 along x: the
    // root boxes are strictly separated, so the sets are provably disjoint at
    // the root and the separation is certified unbounded.
    let mut state = 0xDEAD_BEEF_CAFE_F00D;
    let near: Vec<PointPiece> = (0..24)
        .map(|_| PointPiece {
            p: lcg_point(&mut state),
        })
        .collect();
    let mut state_far = 0x1234_5678_9ABC_DEF0;
    let far: Vec<PointPiece> = (0..24)
        .map(|_| {
            let p = lcg_point(&mut state_far);
            PointPiece {
                p: Point3::new(p.x + 1000.0, p.y, p.z),
            }
        })
        .collect();

    let bvh_near = Bvh::build(&near);
    let bvh_far = Bvh::build(&far);
    assert!(
        bvh_near.distance_lower_bound(&bvh_far).is_infinite(),
        "disjoint piece sets certify an unbounded separation"
    );
    assert!(
        bvh_far.distance_lower_bound(&bvh_near).is_infinite(),
        "disjointness is symmetric"
    );

    // The empty set is disjoint from everything: d(A, ∅) = ∞ explicitly.
    let empty: Bvh<PointPiece> = Bvh::build(&[]);
    assert!(empty.distance_lower_bound(&bvh_near).is_infinite());
    assert!(bvh_near.distance_lower_bound(&empty).is_infinite());
}

#[test]
fn distance_lower_bound_self_sanity_on_small_trees() {
    // A single piece has no distinct pair: self-distance is +∞.
    let one = vec![PointPiece {
        p: Point3::new(1.0, 2.0, 3.0),
    }];
    let bvh_one = Bvh::build(&one);
    assert!(bvh_one.distance_lower_bound_self().is_infinite());

    // Two coincident pieces in one leaf: their distance is 0, and 0 is the
    // certified lower bound.
    let p = Point3::new(1.0, 2.0, 3.0);
    let coincident = vec![PointPiece { p }, PointPiece { p }];
    let bvh_two = Bvh::build(&coincident);
    assert_eq!(bvh_two.distance_lower_bound_self(), 0.0); // H-3: the exact zero separation of coincident pieces
}
