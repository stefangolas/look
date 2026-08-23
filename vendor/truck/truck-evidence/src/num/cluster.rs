//! BG-NUM-004: certified ball-overlap clustering (topology-free core).
//!
//! The replacement rule this module certifies: clusters are determined by
//! **certified ball overlap**, never grid quantisation and never transitive
//! closure of pairwise nearness-as-tolerance (p ~τ q is NOT transitive, so it
//! is never used as a predicate). Two endpoints `i`, `j` join one cluster iff
//! `d(X_i, X_j) <= (r_i + eps) + (r_j + eps)` — a certified, pairwise,
//! position-independent test, computed on squared distances. Connected
//! COMPONENTS of that graph are where transitivity legitimately lives: a chain
//! of overlapping balls is one cluster even when its ends are far apart.
//!
//! Every returned [`Cluster`] carries a CERTIFIED enclosing ball: the
//! coordinate-wise midpoint of the member balls' bounding box as its center,
//! and half the box diagonal as an UPPER bound on the cluster's extent. The
//! bound is coarse but certified (it contains every member ball),
//! deterministic, and order-independent up to member set — no
//! smallest-enclosing-ball optimization.
//!
//! Admissibility ([`cluster`]'s second obligation) refuses a cluster whose
//! enclosing radius exceeds its allowed ceiling instead of silently reporting
//! it. The spec's refine-before-refusal loop lives with the EMITTER (it owns
//! the solver that would produce smaller residuals); this core cannot
//! re-solve, so it must not pretend to — a violated cluster is a typed
//! [`Refusal`], and the refusal's witness names the admissibility failure.
//!
//! F-2 context: the polyline node-identity fix this packet also ships does NOT
//! route through [`cluster`] yet — the polyline use case has no solve residuals
//! to derive radii from. That wiring lands with the emitter packets that own
//! residuals; documented deferral, not an oversight.

#![deny(clippy::unwrap_used)]

use std::collections::HashSet;

use truck_base::cgmath64::*;
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
    UnresolvedWitness,
};

/// The spec's clustering collapse fraction, fixed here so callers cannot tune
/// it toward the degenerate limit: the spec requires theta < 1/2, and 0.25 is
/// chosen once, in this module.
pub const THETA: f64 = 0.25; // H-3: dimensionless collapse fraction, fixed below the 1/2 spec ceiling

/// A certified cluster: the member indices, a certified enclosing ball's
/// center, and an UPPER bound on the cluster's extent.
///
/// What this type certifies: `center` and `enclosing_radius` describe a ball
/// that contains every member ball `B(X_m, r_m)`. It does NOT certify feature
/// size, lfs, or any topological statement — `enclosing_radius` is an
/// enclosure quantity (an upper bound on extent), not a feature size.
#[derive(Clone, Debug)]
pub struct Cluster {
    /// Indices into the input slice, sorted ascending.
    pub members: Vec<usize>,
    /// Center of a CERTIFIED enclosing ball for all member balls.
    pub center: Point3,
    /// Radius of that certified enclosing ball: an UPPER bound on the
    /// cluster's extent. It is an enclosure quantity, not a feature size.
    pub enclosing_radius: f64,
}

/// The clustering policy: ball-inflation margin, admissibility ceiling, and
/// the caller-supplied certified scale bound with lfs-shaped semantics.
///
/// What this type certifies: nothing by itself — it is the caller's declared
/// envelope. The margins are validated (finite, non-negative) at entry so a
/// policy cannot silently poison the certified partition.
#[derive(Clone, Debug)]
pub struct ClusterPolicy {
    /// Ball-inflation margin applied per endpoint: i~j iff
    /// ball(X_i, r_i + eps) overlaps ball(X_j, r_j + eps).
    pub eps: f64,
    /// Collision tolerance ceiling from the caller's context.
    pub tau_col: f64,
    /// Caller-supplied certified scale bound with lfs-shaped semantics.
    /// None means unconstrained (+inf semantics): admissibility then
    /// degrades to tau_col alone. Wiring real stratified evidence into this
    /// slot is later work; `fid::lfs` is deliberately not imported here.
    pub scale_lower: Option<f64>,
}

/// Relative merge bias: the squared overlap threshold is inflated by this
/// dimensionless fraction so BORDERLINE cases merge. Over-merging degrades
/// precision; under-merging silently splits a feature, which is F-2's failure
/// direction — so the bias is one-sided, toward merging.
const MERGE_BIAS: f64 = 1.0e-9; // H-3: dimensionless relative slack on the squared overlap threshold, not a length

/// The squared-threshold factor, `1 + MERGE_BIAS`, applied on the squared
/// distance comparison so a borderline overlap merges.
const MERGE_FACTOR: f64 = 1.0 + MERGE_BIAS; // H-3: dimensionless one-plus-slack threshold factor, not a length

/// Coordinate-wise half, used for the certified enclosing-ball center and
/// radius.
const HALF: f64 = 0.5; // H-3: dimensionless half of an axis width, not a length

/// Clusters `points` into connected components of certified ball overlap,
/// each carrying a certified enclosing ball.
///
/// CERTIFIES: (1) the partition is position-independent — Euclidean distances
/// only, never grid quantisation, and never transitive closure of a
/// nearness-as-tolerance predicate (transitivity lives only in connected
/// COMPONENTS); and (2) every returned [`Cluster`] is enclosed by the ball it
/// reports — `cluster.center` and `cluster.enclosing_radius` are a certified
/// enclosing ball of all member balls.
///
/// Admissibility is checked before return: a cluster whose enclosing radius
/// exceeds `min(tau_col, THETA * scale_lower)` — or `tau_col` alone when
/// `scale_lower` is unconstrained (None, or non-positive) — is refused as
/// [`Refusal::NumericallyUnresolved`], never silently reported.
///
/// NEVER certifies: feature size, lfs, or any statement about a topology. The
/// refine-before-refusal loop is the caller's obligation: this core has no
/// solver, so it cannot refine, and it does not pretend to.
///
/// `budget` is carried for signature uniformity; no bisection happens here, so
/// it is reported unchanged (spent zero).
pub fn cluster(
    points: &[Point3],
    radii: &[f64],
    policy: &ClusterPolicy,
    budget: &mut Budget,
) -> Outcome<Vec<Cluster>> {
    let initial = *budget;
    // Decision 0 (validation): malformed input — mismatched lengths, a
    // non-finite or negative margin, or a non-finite point/radius — has
    // nothing to certify.
    if points.len() != radii.len()
        || !policy.eps.is_finite()
        || policy.eps < 0.0
        || !policy.tau_col.is_finite()
        || policy.tau_col < 0.0
        || matches!(policy.scale_lower, Some(s) if !s.is_finite())
    {
        return Err(Refusal::Empty);
    }
    if points.iter().zip(radii.iter()).any(|(p, r)| {
        !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() || !r.is_finite() || *r < 0.0
    }) {
        return Err(Refusal::Empty);
    }
    // An empty input is a certified empty clustering, not an error: no member
    // balls, no clusters, nothing to refuse.
    if points.is_empty() {
        return Ok(Certified::new(Vec::new(), certificate(budget)));
    }
    let clusters = components(points, radii, policy);
    // Decision 3 (admissibility): a `Some` positive scale_lower tightens the
    // ceiling to `min(tau_col, THETA * scale_lower)`; otherwise the ceiling
    // degrades to tau_col alone (None means +inf semantics, and a non-positive
    // Some is unconstrained exactly as the branch guard `s > 0` says).
    let ceiling = match policy.scale_lower {
        Some(s) if s > 0.0 => policy.tau_col.min(THETA * s),
        _ => policy.tau_col,
    };
    for cluster in &clusters {
        if cluster.enclosing_radius > ceiling {
            return Err(Refusal::NumericallyUnresolved {
                spent: spent(&initial, budget),
                witness: UnresolvedWitness::UncertifiedContainment,
            });
        }
    }
    Ok(Certified::new(clusters, certificate(budget)))
}

/// The connected components of certified ball overlap as [`Cluster`]s, in
/// deterministic order (sorted by smallest member).
fn components(points: &[Point3], radii: &[f64], policy: &ClusterPolicy) -> Vec<Cluster> {
    let n = points.len();
    let mut remaining: HashSet<usize> = (0..n).collect();
    let mut clusters: Vec<Cluster> = Vec::new();
    while let Some(&start) = remaining.iter().next() {
        remaining.remove(&start);
        let mut members: Vec<usize> = Vec::new();
        let mut stack: Vec<usize> = vec![start];
        while let Some(i) = stack.pop() {
            members.push(i);
            for j in 0..n {
                if remaining.contains(&j) && balls_overlap(points, radii, policy, i, j) {
                    remaining.remove(&j);
                    stack.push(j);
                }
            }
        }
        members.sort_unstable();
        clusters.push(build_cluster(points, radii, members));
    }
    clusters.sort_by(|a, b| a.members.first().cmp(&b.members.first()));
    clusters
}

/// Decision 1 (adjacency): i~j iff `d(X_i, X_j) <= (r_i + eps) + (r_j + eps)`,
/// computed on SQUARED distances with a small relative slack that biases
/// borderline cases toward merging (over-merge degrades precision;
/// under-merge silently splits a feature — F-2's failure direction).
fn balls_overlap(
    points: &[Point3],
    radii: &[f64],
    policy: &ClusterPolicy,
    i: usize,
    j: usize,
) -> bool {
    let Some(xi) = points.get(i) else {
        return false;
    };
    let Some(xj) = points.get(j) else {
        return false;
    };
    let Some(ri) = radii.get(i).copied() else {
        return false;
    };
    let Some(rj) = radii.get(j).copied() else {
        return false;
    };
    let d2 = (*xi - *xj).magnitude2();
    let s = (ri + policy.eps) + (rj + policy.eps);
    d2 <= s * s * MERGE_FACTOR
}

/// Decision 2 (certified enclosing ball): the coordinate-wise midpoint of the
/// bounding box of `{X_i ± r_i}` over members is the center, and half the box
/// diagonal is the enclosing radius. Coarse but CERTIFIED (contains every
/// member ball), deterministic, order-independent up to member set.
fn build_cluster(points: &[Point3], radii: &[f64], members: Vec<usize>) -> Cluster {
    let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &m in &members {
        let Some(p) = points.get(m) else {
            continue;
        };
        let Some(r) = radii.get(m).copied() else {
            continue;
        };
        min.x = min.x.min(p.x - r);
        min.y = min.y.min(p.y - r);
        min.z = min.z.min(p.z - r);
        max.x = max.x.max(p.x + r);
        max.y = max.y.max(p.y + r);
        max.z = max.z.max(p.z + r);
    }
    let center = Point3::new(
        HALF * (min.x + max.x),
        HALF * (min.y + max.y),
        HALF * (min.z + max.z),
    );
    let dx = max.x - min.x;
    let dy = max.y - min.y;
    let dz = max.z - min.z;
    let enclosing_radius = HALF * (dx * dx + dy * dy + dz * dz).sqrt();
    Cluster {
        members,
        center,
        enclosing_radius,
    }
}

/// Spend since entry: the initial budget minus what remains. No bisection
/// happens here, so the spent ledger is the zero budget.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

/// The clustering certificate: float method (squared distances, midpoints and
/// box diagonals are plain f64 arithmetic — never `Exact`, H-6), untouched
/// budget, unbounded margin and modulus.
fn certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Float,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

#[cfg(test)]
mod tests {
    // GATE-1: the cluster module (including its test module) stays under the
    // crate's unwrap denial; unit tests assert on hand-built witnesses, and
    // `must`/`must_err` below are the deny-clean spellings of unwrap/unwrap_err.
    #![deny(clippy::unwrap_used)]

    use super::*;

    /// Test-only unwrap that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a refusal here is a test bug.
    fn must<T>(r: Outcome<T>) -> T {
        match r {
            Ok(Certified { value, .. }) => value,
            Err(_) => unreachable!("unit-test witness must certify"),
        }
    }

    /// Test-only unwrap_err that stays under the crate's deny list: unit tests
    /// assert on hand-built witnesses, so a success here is a test bug.
    fn must_err<T>(r: Outcome<T>) -> Refusal {
        match r {
            Ok(_) => unreachable!("unit-test witness must refuse"),
            Err(e) => e,
        }
    }

    /// A policy from its three fields.
    fn policy(eps: f64, tau_col: f64, scale_lower: Option<f64>) -> ClusterPolicy {
        ClusterPolicy {
            eps,
            tau_col,
            scale_lower,
        }
    }

    /// The member-index partition, order-normalized for equivariance checks.
    fn member_sets(clusters: &[Cluster]) -> Vec<Vec<usize>> {
        let mut sets: Vec<Vec<usize>> = clusters.iter().map(|c| c.members.clone()).collect();
        sets.sort();
        sets
    }

    #[test]
    fn f2_close_points_cluster_at_any_translation() {
        // F-2 defect scale: two endpoints CLOSE_SEP apart land in DIFFERENT
        // grid cells at some absolute positions. Certified ball overlap must
        // weld them into ONE cluster at every translation, including large
        // ones where the old grid split nodes.
        const CLOSE_SEP: f64 = 1.0e-9; // H-3: F-2 defect separation, 1000x below the legacy tolerance (1e-6), not a length threshold
        const WELD_EPS: f64 = 1.0e-9; // H-3: ball-inflation margin, 1000x below the legacy tolerance
        const ZERO_RADIUS: f64 = 0.0; // H-3: degenerate zero radius for the F-2 witness, not a tolerance
        const ROOMY_TAU: f64 = 1.0e-3; // H-3: admissibility ceiling far above the F-2 cluster's extent
        const MID_TX: f64 = 1.0e3; // H-3: mid-scale translation in x, a position offset, not a tolerance
        const MID_TY: f64 = -2.0e3; // H-3: mid-scale translation in y, a position offset, not a tolerance
        const MID_TZ: f64 = 3.0e3; // H-3: mid-scale translation in z, a position offset, not a tolerance
        const FAR_TX: f64 = 1.0e6; // H-3: far-scale translation in x, a position offset, not a tolerance
        const FAR_TY: f64 = 5.0e5; // H-3: far-scale translation in y, a position offset, not a tolerance
        const FAR_TZ: f64 = -7.0e5; // H-3: far-scale translation in z, a position offset, not a tolerance
        let translations = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(MID_TX, MID_TY, MID_TZ),
            Vector3::new(FAR_TX, FAR_TY, FAR_TZ),
        ];
        for t in translations {
            let points = vec![
                Point3::new(0.0, 0.0, 0.0) + t,
                Point3::new(CLOSE_SEP, 0.0, 0.0) + t,
            ];
            let radii = vec![ZERO_RADIUS, ZERO_RADIUS];
            let mut budget = Budget::new(16, 0, 0);
            let out = must(cluster(
                &points,
                &radii,
                &policy(WELD_EPS, ROOMY_TAU, None),
                &mut budget,
            ));
            assert_eq!(out.len(), 1, "translation {t:?} must weld the close pair");
            assert_eq!(
                out.first().map(|c| c.members.as_slice()),
                Some([0, 1].as_slice())
            );
        }
    }

    #[test]
    fn f2_separated_points_stay_distinct_at_any_translation() {
        // Endpoints 3x the legacy tolerance apart, with tiny radii and a
        // LOOSE_EPS inflation margin, must stay TWO clusters at every
        // translation: the sum of inflated radii is far below the separation.
        const SEP: f64 = 3.0e-6; // H-3: endpoint separation, 3x the legacy tolerance, a distinct-feature scale
        const TINY_RADIUS: f64 = 1.0e-9; // H-3: ball radius, 1000x below the legacy tolerance
        const LOOSE_EPS: f64 = 1.0e-7; // H-3: ball-inflation margin, 10x below the legacy tolerance
        const ROOMY_TAU: f64 = 1.0e-3; // H-3: admissibility ceiling far above these clusters' extents
        const MID_TX: f64 = 1.0e3; // H-3: mid-scale translation in x, a position offset, not a tolerance
        const MID_TY: f64 = -2.0e3; // H-3: mid-scale translation in y, a position offset, not a tolerance
        const MID_TZ: f64 = 3.0e3; // H-3: mid-scale translation in z, a position offset, not a tolerance
        const FAR_TX: f64 = 1.0e6; // H-3: far-scale translation in x, a position offset, not a tolerance
        const FAR_TY: f64 = 5.0e5; // H-3: far-scale translation in y, a position offset, not a tolerance
        const FAR_TZ: f64 = -7.0e5; // H-3: far-scale translation in z, a position offset, not a tolerance
        let translations = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(MID_TX, MID_TY, MID_TZ),
            Vector3::new(FAR_TX, FAR_TY, FAR_TZ),
        ];
        for t in translations {
            let points = vec![
                Point3::new(0.0, 0.0, 0.0) + t,
                Point3::new(SEP, 0.0, 0.0) + t,
            ];
            let radii = vec![TINY_RADIUS, TINY_RADIUS];
            let mut budget = Budget::new(16, 0, 0);
            let out = must(cluster(
                &points,
                &radii,
                &policy(LOOSE_EPS, ROOMY_TAU, None),
                &mut budget,
            ));
            assert_eq!(out.len(), 2, "translation {t:?} must keep the pair apart");
            assert_eq!(
                out.first().map(|c| c.members.as_slice()),
                Some([0].as_slice())
            );
            assert_eq!(
                out.get(1).map(|c| c.members.as_slice()),
                Some([1].as_slice())
            );
        }
    }

    #[test]
    fn chain_of_pairs_is_one_component_not_transitive_tolerance() {
        // A chain of overlapping pairs A~B, B~C with d(A, C) beyond the
        // overlap threshold is ONE component of 3 members: connected
        // components are where transitivity legitimately lives, while pairwise
        // tolerance chaining as a PREDICATE is what is forbidden. A control
        // case where only A~B overlaps gives TWO clusters.
        const STEP: f64 = 1.0e-6; // H-3: chain step at the legacy tolerance scale, a position offset
        const CHAIN_EPS: f64 = 6.0e-7; // H-3: inflation margin making consecutive pairs overlap (60% of the legacy tolerance)
        const CONTROL_FAR: f64 = 5.0e-6; // H-3: control-case separation, 5x the legacy tolerance, past pairwise overlap reach
        const ROOMY_TAU: f64 = 1.0e-2; // H-3: admissibility ceiling far above these clusters' extents
        const ZERO_RADIUS: f64 = 0.0; // H-3: degenerate zero radius for the chain witness, not a tolerance
        let radii = vec![ZERO_RADIUS; 3];
        let chain = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(STEP, 0.0, 0.0),
            Point3::new(2.0 * STEP, 0.0, 0.0),
        ];
        let mut budget = Budget::new(16, 0, 0);
        let out = must(cluster(
            &chain,
            &radii,
            &policy(CHAIN_EPS, ROOMY_TAU, None),
            &mut budget,
        ));
        assert_eq!(
            out.len(),
            1,
            "a chain of overlapping balls is ONE component of 3 members"
        );
        assert_eq!(
            out.first().map(|c| c.members.as_slice()),
            Some([0, 1, 2].as_slice())
        );
        let control = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(STEP, 0.0, 0.0),
            Point3::new(CONTROL_FAR, 0.0, 0.0),
        ];
        let mut budget = Budget::new(16, 0, 0);
        let out = must(cluster(
            &control,
            &radii,
            &policy(CHAIN_EPS, ROOMY_TAU, None),
            &mut budget,
        ));
        assert_eq!(out.len(), 2, "only the overlapping pair merges");
        assert_eq!(
            out.first().map(|c| c.members.as_slice()),
            Some([0, 1].as_slice())
        );
        assert_eq!(
            out.get(1).map(|c| c.members.as_slice()),
            Some([2].as_slice())
        );
    }

    #[test]
    fn partition_equivariant_under_translation() {
        // A mixed 4-point input partitioned into two pairs: translating the
        // whole configuration by a large vector must leave the member INDEX
        // SETS exactly equal.
        const PAIR_GAP: f64 = 5.0e-7; // H-3: intra-pair separation, half the legacy tolerance
        const PAIR_EPS: f64 = 3.0e-7; // H-3: inflation margin, 30% of the legacy tolerance, above PAIR_GAP
        const PAIR_OFFSET: f64 = 1.0e-3; // H-3: inter-pair offset, 1000x the legacy tolerance, far beyond PAIR_EPS reach
        const ROOMY_TAU: f64 = 1.0e-1; // H-3: admissibility ceiling far above these clusters' extents
        const ZERO_RADIUS: f64 = 0.0; // H-3: degenerate zero radius for the equivariance witness, not a tolerance
        const EQ_TX: f64 = 1.0e4; // H-3: equivalence translation in x, a position offset, not a tolerance
        const EQ_TY: f64 = -2.0e4; // H-3: equivalence translation in y, a position offset, not a tolerance
        const EQ_TZ: f64 = 3.0e4; // H-3: equivalence translation in z, a position offset, not a tolerance
        let radii = vec![ZERO_RADIUS; 4];
        let base = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(PAIR_GAP, 0.0, 0.0),
            Point3::new(PAIR_OFFSET, PAIR_OFFSET, 0.0),
            Point3::new(PAIR_OFFSET + PAIR_GAP, PAIR_OFFSET, 0.0),
        ];
        let t = Vector3::new(EQ_TX, EQ_TY, EQ_TZ);
        let moved: Vec<Point3> = base.iter().map(|p| *p + t).collect();
        let mut budget = Budget::new(16, 0, 0);
        let a = must(cluster(
            &base,
            &radii,
            &policy(PAIR_EPS, ROOMY_TAU, None),
            &mut budget,
        ));
        let b = must(cluster(
            &moved,
            &radii,
            &policy(PAIR_EPS, ROOMY_TAU, None),
            &mut budget,
        ));
        assert_eq!(
            member_sets(&a),
            member_sets(&b),
            "the partition must be translation-equivariant"
        );
    }

    #[test]
    fn partition_equivariant_under_uniform_scale() {
        // The same input, every length scaled by a uniform factor: points,
        // radii, eps, tau_col and scale_lower ALL scale, so the overlap
        // predicate, the enclosing radius and the admissibility ceiling scale
        // together and the partition is unchanged.
        const PAIR_GAP: f64 = 5.0e-7; // H-3: intra-pair separation, half the legacy tolerance
        const PAIR_EPS: f64 = 3.0e-7; // H-3: inflation margin, 30% of the legacy tolerance, above PAIR_GAP
        const PAIR_OFFSET: f64 = 1.0e-3; // H-3: inter-pair offset, 1000x the legacy tolerance
        const ROOMY_TAU: f64 = 1.0e-1; // H-3: unconstrained admissibility ceiling far above the cluster extents
        const SCALE_LOWER: f64 = 1.0; // H-3: unconstrained certified scale bound, dimensionless vs the cluster scale
        const ZERO_RADIUS: f64 = 0.0; // H-3: degenerate zero radius for the equivariance witness, not a tolerance
        const K: f64 = 100.0; // H-3: dimensionless uniform scale factor applied to every length
        let radii = vec![ZERO_RADIUS; 4];
        let base = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(PAIR_GAP, 0.0, 0.0),
            Point3::new(PAIR_OFFSET, PAIR_OFFSET, 0.0),
            Point3::new(PAIR_OFFSET + PAIR_GAP, PAIR_OFFSET, 0.0),
        ];
        let scaled: Vec<Point3> = base.iter().map(|p| *p * K).collect();
        let radii_scaled: Vec<f64> = radii.iter().map(|r| r * K).collect();
        let mut budget = Budget::new(16, 0, 0);
        let a = must(cluster(
            &base,
            &radii,
            &policy(PAIR_EPS, ROOMY_TAU, Some(SCALE_LOWER)),
            &mut budget,
        ));
        let b = must(cluster(
            &scaled,
            &radii_scaled,
            &policy(PAIR_EPS * K, ROOMY_TAU * K, Some(SCALE_LOWER * K)),
            &mut budget,
        ));
        assert_eq!(
            member_sets(&a),
            member_sets(&b),
            "the partition must be scale-equivariant"
        );
    }

    #[test]
    fn admissibility_violation_refuses_with_witness() {
        // A cluster whose enclosing radius exceeds tau_col is refused as
        // NumericallyUnresolved with a named witness and a zero spent ledger
        // (no bisection happens in this core).
        const ADM_OFFSET: f64 = 1.0e-3; // H-3: inter-ball center distance, 1000x the legacy tolerance
        const ADM_RADIUS: f64 = 6.0e-4; // H-3: ball radius, 600x the legacy tolerance, enough for the balls to overlap
        const TIGHT_TAU: f64 = 1.0e-3; // H-3: admissibility ceiling below the enclosing radius, forcing the refusal
        const ZERO_EPS: f64 = 0.0; // H-3: zero inflation margin for this witness, not a tolerance
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(ADM_OFFSET, 0.0, 0.0),
        ];
        let radii = vec![ADM_RADIUS, ADM_RADIUS];
        let mut budget = Budget::new(16, 0, 0);
        let err = must_err(cluster(
            &points,
            &radii,
            &policy(ZERO_EPS, TIGHT_TAU, None),
            &mut budget,
        ));
        assert!(
            matches!(
                err,
                Refusal::NumericallyUnresolved {
                    spent,
                    witness: UnresolvedWitness::UncertifiedContainment,
                } if spent.subdiv == 0 && spent.newton == 0 && spent.depth == 0
            ),
            "the tight-tau cluster must refuse with the admissibility witness and zero spend, got {err:?}"
        );
    }

    #[test]
    fn scale_bound_tightens_admissibility() {
        // The same geometry: with scale_lower unconstrained (None) the cluster
        // is admissible; with a certified scale bound whose THETA fraction
        // falls below the enclosing radius, the same cluster refuses.
        const ADM_OFFSET: f64 = 1.0e-3; // H-3: inter-ball center distance, 1000x the legacy tolerance
        const ADM_RADIUS: f64 = 6.0e-4; // H-3: ball radius, 600x the legacy tolerance, enough for the balls to overlap
        const ROOMY_TAU: f64 = 1.0; // H-3: unconstrained admissibility ceiling far above the cluster's extent
        const TIGHT_SCALE: f64 = 2.0e-3; // H-3: certified scale bound whose THETA fraction (5e-4) falls below the enclosing radius
        const ZERO_EPS: f64 = 0.0; // H-3: zero inflation margin for this witness, not a tolerance
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(ADM_OFFSET, 0.0, 0.0),
        ];
        let radii = vec![ADM_RADIUS, ADM_RADIUS];
        let mut budget = Budget::new(16, 0, 0);
        let admitted = must(cluster(
            &points,
            &radii,
            &policy(ZERO_EPS, ROOMY_TAU, None),
            &mut budget,
        ));
        assert_eq!(
            admitted.len(),
            1,
            "unconstrained scale must admit the cluster"
        );
        let refused = must_err(cluster(
            &points,
            &radii,
            &policy(ZERO_EPS, ROOMY_TAU, Some(TIGHT_SCALE)),
            &mut budget,
        ));
        assert!(
            matches!(refused, Refusal::NumericallyUnresolved { .. }),
            "a scale bound below enclosing_radius/THETA must refuse, got {refused:?}"
        );
    }
}
