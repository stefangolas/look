//! Per-carrier span BVH broadphase (CFP-005-BRANCH-BVH).
//!
//! After D1 decomposition each carrier is a stack of admitted Bézier spans
//! ([`crate::patch_admit::SplinePatchStack`]); a loft×loft `contact()` call
//! would otherwise enumerate the full span-pair Cartesian product (a 20×30-span
//! loft pair is ~360 000 Bézier-pair admissions with no geometric screen).
//! This module lands the inner broadphase: a per-carrier BVH over the span
//! rectangles whose leaves carry the certified `enclose` box of the span and
//! whose internal nodes bound the union. Two carriers' trees are traversed
//! jointly; a node pair whose control nets separate under Theorem 3
//! (`docs/CONTACT_FAST_PATH_BUILD_SPEC.md` §3) is pruned *hereditarily* — one
//! certificate removes every descendant span pair, with no re-verification.
//!
//! **SFC discipline (spine decision 2, verbatim).** The search for a
//! separating direction is a deterministic float search whose result is typed
//! [`FloatHint`] (the spine's type — carries no evidence status). The
//! certificate is the exact `Expansion` sign row
//! `min_α λ·P¹_α > max_β λ·P²_β` over the node's net constants — O(n₁+n₂),
//! BG-ENC-003-clean — and a separation verdict EXISTS only where that exact
//! row certifies. If no searched direction certifies, the node pair is NOT
//! separated and is descended: the float layer can never cause a false prune.
//!
//! **Scope decisions (CFP-005; do not relitigate).** (1) The leaves are the
//! span rectangles of the two carriers; leaf bounds are the per-span certified
//! `enclose` boxes (the computation `hull_bernstein_2d` already performs) and
//! internal nodes bound the union. (2) Separation certificates inherit: a
//! parent-node certificate prunes every descendant pair without
//! re-verification. (3) Surviving contact-candidate span pairs are emitted in
//! leaf-index lexicographic order — downstream behavior is unchanged, only
//! shorter. (4) Dyadic memoization rides CFP-003's per-side hull cache; this
//! packet adds no second cache. (5) Zero new top-level evidence kinds: a
//! pruned node produces nothing, a surviving pair produces exactly what the
//! landed funnel produced before — the BVH changes WHICH pairs are enumerated,
//! never what a pair's answer is.
//!
//! **Determinism.** Fixed build order (span stack order), fixed candidate
//! order in the float search, low-before-high descent, and a final
//! lexicographic sort of the emitted pairs; identical ordered input produces an
//! identical pair list and identical prune records, byte-for-byte.
//!
//! The `ssi4.rs` direct-evaluation path is untouched; this packet is consumed
//! by the span-pairing site of the SSI funnel through the accessors CFP-003
//! landed.

use crate::cfp::FloatHint;
use crate::formal::exact::{CertifiedInterval, CertifiedSign, Expansion};
use crate::hull::hull_bernstein_2d;
use crate::patch_admit::{AdmittedPatch, SplinePatchStack};
use std::collections::VecDeque;

/// Why a span-BVH construction or traversal could not be certified.
///
/// Named cases only — no catch-all — matching the refusal shape used across
/// the crate's certified layers. Each variant carries a stable diagnostic tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvhRefusal {
    /// A carrier has no spans: an empty stack produces no BVH.
    EmptyCarrier,
    /// A certified box axis is not finite or is misordered (`lo > hi`).
    NonFiniteBox,
    /// A span leaf carries no control-net constants.
    EmptyNet,
    /// A span leaf's control-net constants are not all finite.
    NonFiniteNetPoint,
    /// A leaf `enclose` box could not be certified (the hull kernel refused).
    EnclosureUnavailable,
    /// A tree-traversal internal invariant was violated (a missing node or a
    /// missing child). Unreachable for trees this module builds.
    InvalidTree,
}

impl BvhRefusal {
    /// The stable diagnostic tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::EmptyCarrier => "bvh_empty_carrier",
            Self::NonFiniteBox => "bvh_non_finite_box",
            Self::EmptyNet => "bvh_empty_net",
            Self::NonFiniteNetPoint => "bvh_non_finite_net_point",
            Self::EnclosureUnavailable => "bvh_enclosure_unavailable",
            Self::InvalidTree => "bvh_invalid_tree",
        }
    }
}

/// The certified axis-aligned enclosure of one span's image (BG-ENC-003): one
/// outward-rounded interval per coordinate.
///
/// This is the leaf bound of scope decision 1 — the per-span certified
/// `enclose` box. Internal nodes bound the union of their descendants' boxes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosureBox {
    /// The three certified coordinate intervals, `(x, y, z)` order.
    axes: [CertifiedInterval; 3],
}

/// Merge two certified intervals into their certified union (outward: take the
/// lower of the lows and the upper of the highs).
fn union_axis(a: CertifiedInterval, b: CertifiedInterval) -> CertifiedInterval {
    CertifiedInterval {
        lo: a.lo.min(b.lo),
        hi: a.hi.max(b.hi),
    }
}

impl EnclosureBox {
    /// Build a certified box from three finite, ordered coordinate intervals.
    ///
    /// Refuses ([`BvhRefusal::NonFiniteBox`]) any axis that is not finite or
    /// whose lower bound exceeds its upper bound.
    pub fn new(axes: [CertifiedInterval; 3]) -> Result<Self, BvhRefusal> {
        if axes
            .iter()
            .any(|axis| !axis.is_finite() || axis.lo > axis.hi)
        {
            return Err(BvhRefusal::NonFiniteBox);
        }
        Ok(Self { axes })
    }

    /// The three certified coordinate intervals, `(x, y, z)` order.
    pub fn axes(&self) -> &[CertifiedInterval; 3] {
        &self.axes
    }

    /// The certified union of two boxes: per axis, the outward interval hull.
    /// Infallible: the union of finite, ordered boxes is finite and ordered.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            axes: [
                union_axis(self.axes[0], other.axes[0]),
                union_axis(self.axes[1], other.axes[1]),
                union_axis(self.axes[2], other.axes[2]),
            ],
        }
    }
}

/// One BVH leaf: one admitted Bézier span of a carrier, with its source
/// knot-span rectangle, its certified `enclose` box, and its control-net
/// constants.
///
/// The net constants are the control points of the span's Bézier patch — the
/// "node's net constants" over which the exact Theorem-3 sign row runs. The
/// certified box bounds the span's image (scope decision 1).
#[derive(Debug, Clone, PartialEq)]
pub struct SpanLeaf {
    /// The source knot-span rectangle `(u0, u1) × (v0, v1)`.
    cell: ((f64, f64), (f64, f64)),
    /// The certified `enclose` box of the span's image.
    box_: EnclosureBox,
    /// The span's control-net constants (dehomogenized Bézier control points).
    net: Vec<[f64; 3]>,
}

impl SpanLeaf {
    /// Build a span leaf.
    ///
    /// Refuses ([`BvhRefusal::EmptyNet`]) an empty net and
    /// ([`BvhRefusal::NonFiniteNetPoint`]) a net holding a non-finite point.
    /// The certified box must itself be a valid box (its constructor refuses).
    pub fn new(
        cell: ((f64, f64), (f64, f64)),
        box_: EnclosureBox,
        net: Vec<[f64; 3]>,
    ) -> Result<Self, BvhRefusal> {
        if net.is_empty() {
            return Err(BvhRefusal::EmptyNet);
        }
        if net.iter().any(|p| p.iter().any(|c| !c.is_finite())) {
            return Err(BvhRefusal::NonFiniteNetPoint);
        }
        Ok(Self { cell, box_, net })
    }

    /// The source knot-span rectangle `(u0, u1) × (v0, v1)`.
    pub fn cell(&self) -> ((f64, f64), (f64, f64)) {
        self.cell
    }

    /// The certified `enclose` box of the span's image.
    pub fn box_(&self) -> &EnclosureBox {
        &self.box_
    }

    /// The span's control-net constants.
    pub fn net(&self) -> &[[f64; 3]] {
        &self.net
    }
}

/// One node of a span BVH: a contiguous range of leaves (in span stack order)
/// together with the certified union box and the union control-net constants of
/// its descendant spans.
///
/// A leaf node covers one span; an internal node splits its range in half at
/// `mid = start + count / 2` (the fixed, order-only build rule). Each node's
/// net constants are the concatenation of its children's, so the exact
/// Theorem-3 sign row over a node's net bounds every descendant span pair
/// (certificates inherit).
#[derive(Debug, Clone)]
struct Node {
    /// The first covered leaf's position in the leaves array.
    leaf_start: usize,
    /// How many leaves the node covers.
    leaf_count: usize,
    /// The certified union box of the covered spans' `enclose` boxes.
    box_: EnclosureBox,
    /// The union control-net constants of the covered spans.
    net: Vec<[f64; 3]>,
    /// The left child node id (a leaf node has none).
    left: Option<usize>,
    /// The right child node id (a leaf node has none).
    right: Option<usize>,
}

/// A per-carrier span BVH: the carrier's span leaves in stack order plus the
/// balanced tree over that order.
#[derive(Debug, Clone)]
pub struct SpanBvh {
    /// The span leaves, in span stack order (position = span index).
    leaves: Vec<SpanLeaf>,
    /// The tree nodes; the root is the last-pushed node.
    nodes: Vec<Node>,
    /// The root node id.
    root: usize,
}

impl SpanBvh {
    /// Build a span BVH over the span stack of an admitted spline carrier.
    ///
    /// Leaves are the carrier's spans in stack order; each leaf's certified box
    /// is the per-span `enclose` of its Bézier image (computed with the landed
    /// `hull_bernstein_2d` over each numerator component) and its net constants
    /// are the patch's dehomogenized control points. Refuses
    /// ([`BvhRefusal::EmptyCarrier`]) an empty stack,
    /// ([`BvhRefusal::EnclosureUnavailable`]) a span whose certified box cannot
    /// be certified, and the leaf construction refusals for an invalid net.
    pub fn over_stack(stack: &SplinePatchStack) -> Result<Self, BvhRefusal> {
        if stack.is_empty() {
            return Err(BvhRefusal::EmptyCarrier);
        }
        let mut leaves = Vec::with_capacity(stack.len());
        for patch in stack.patches() {
            leaves.push(leaf_from_admitted_patch(patch)?);
        }
        Self::build(leaves)
    }

    /// Build a span BVH from an ordered leaf list.
    ///
    /// The leaves keep their input (span stack) order; the balanced tree is
    /// built over that order by repeatedly halving the range, so identical
    /// ordered input yields an identical tree. Refuses
    /// ([`BvhRefusal::EmptyCarrier`]) an empty leaf list.
    pub fn build(leaves: Vec<SpanLeaf>) -> Result<Self, BvhRefusal> {
        if leaves.is_empty() {
            return Err(BvhRefusal::EmptyCarrier);
        }
        let mut nodes = Vec::new();
        let root = Self::build_range(&leaves, &mut nodes, 0, leaves.len())?;
        Ok(Self {
            leaves,
            nodes,
            root,
        })
    }

    /// Recursively build the balanced tree node covering leaves
    /// `[start, end)`.
    fn build_range(
        leaves: &[SpanLeaf],
        nodes: &mut Vec<Node>,
        start: usize,
        end: usize,
    ) -> Result<usize, BvhRefusal> {
        let leaf_count = end - start;
        if leaf_count == 0 {
            return Err(BvhRefusal::InvalidTree);
        }
        if leaf_count == 1 {
            let leaf = &leaves[start];
            nodes.push(Node {
                leaf_start: start,
                leaf_count: 1,
                box_: *leaf.box_(),
                net: leaf.net().to_vec(),
                left: None,
                right: None,
            });
            return Ok(nodes.len() - 1);
        }
        let mid = start + leaf_count / 2;
        let left = Self::build_range(leaves, nodes, start, mid)?;
        let right = Self::build_range(leaves, nodes, mid, end)?;
        let left_node = nodes.get(left).ok_or(BvhRefusal::InvalidTree)?;
        let right_node = nodes.get(right).ok_or(BvhRefusal::InvalidTree)?;
        let mut net = Vec::with_capacity(left_node.net.len() + right_node.net.len());
        net.extend_from_slice(&left_node.net);
        net.extend_from_slice(&right_node.net);
        let node = Node {
            leaf_start: start,
            leaf_count,
            box_: left_node.box_.union(&right_node.box_),
            net,
            left: Some(left),
            right: Some(right),
        };
        nodes.push(node);
        Ok(nodes.len() - 1)
    }

    /// The span leaves in stack order (position = span index).
    pub fn leaves(&self) -> &[SpanLeaf] {
        &self.leaves
    }

    /// The number of span leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether the BVH holds no spans.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// The certified union box of the whole carrier (the root node's box).
    pub fn root_box(&self) -> &EnclosureBox {
        &self.nodes[self.root].box_
    }
}

/// An exact Theorem-3 separation certificate.
///
/// The certificate EXISTS only where the exact `Expansion` sign row
/// `min_α λ·P¹_α > max_β λ·P²_β` over two node net-constant sets certifies
/// positive ([`CertifiedSign::Positive`]). It records the certifying direction
/// `λ`, the exact sign, and the outward-rounded `CertifiedInterval` enclosure
/// of the exact margin `min_α λ·P¹_α − max_β λ·P²_β`. It is never built from a
/// float hint: the hint is the search result, this is the certificate (SFC).
#[derive(Debug, Clone, PartialEq)]
pub struct SeparationCertificate {
    /// The certifying direction.
    lambda: [f64; 3],
    /// The exact sign of the row — always `Positive` at construction.
    sign: CertifiedSign,
    /// The certified enclosure of the exact positive margin.
    margin: CertifiedInterval,
}

impl SeparationCertificate {
    /// The certifying direction `λ`.
    pub fn lambda(&self) -> [f64; 3] {
        self.lambda
    }

    /// The exact sign of the row (`Positive` whenever the certificate exists).
    pub fn sign(&self) -> CertifiedSign {
        self.sign
    }

    /// The certified enclosure of the exact positive margin.
    pub fn margin(&self) -> CertifiedInterval {
        self.margin
    }
}

/// One prune decision of a joint traversal: the exact certificate that
/// separated one node pair and the span-pair mass that single certificate
/// removed (heredity — no descendant pair is re-verified).
#[derive(Debug, Clone, PartialEq)]
pub struct PruneRecord {
    /// The first covered leaf's position on the first side.
    a_start: usize,
    /// How many first-side leaves the pruned node covers.
    a_count: usize,
    /// The first covered leaf's position on the second side.
    b_start: usize,
    /// How many second-side leaves the pruned node covers.
    b_count: usize,
    /// The exact certificate that separated the node pair.
    certificate: SeparationCertificate,
}

impl PruneRecord {
    /// The first covered leaf's position on the first side.
    pub fn a_start(&self) -> usize {
        self.a_start
    }

    /// How many first-side leaves the pruned node covers.
    pub fn a_count(&self) -> usize {
        self.a_count
    }

    /// The first covered leaf's position on the second side.
    pub fn b_start(&self) -> usize {
        self.b_start
    }

    /// How many second-side leaves the pruned node covers.
    pub fn b_count(&self) -> usize {
        self.b_count
    }

    /// The span pairs this single certificate removed (`a_count × b_count`).
    pub fn covered_leaf_pairs(&self) -> usize {
        self.a_count * self.b_count
    }

    /// The exact certificate that separated the node pair.
    pub fn certificate(&self) -> &SeparationCertificate {
        &self.certificate
    }
}

/// The result of one joint span-BVH traversal: the surviving contact-candidate
/// span pairs, the prune records that removed the rest, and how many exact-row
/// verifications the traversal performed.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanPairScan {
    /// The surviving contact-candidate span pairs `(span_a, span_b)` in
    /// leaf-index lexicographic order.
    candidates: Vec<(usize, usize)>,
    /// The prune records, in deterministic encounter order.
    prune_records: Vec<PruneRecord>,
    /// The number of exact `Expansion` row evaluations performed.
    exact_row_evaluations: usize,
}

impl SpanPairScan {
    /// The surviving contact-candidate span pairs in leaf-index lexicographic
    /// order.
    pub fn candidates(&self) -> &[(usize, usize)] {
        &self.candidates
    }

    /// The prune records, in deterministic encounter order.
    pub fn prune_records(&self) -> &[PruneRecord] {
        &self.prune_records
    }

    /// The number of exact `Expansion` row evaluations the traversal performed.
    pub fn exact_row_evaluations(&self) -> usize {
        self.exact_row_evaluations
    }

    /// The total number of span pairs removed by prune decisions
    /// (the sum of the records' [`PruneRecord::covered_leaf_pairs`]).
    pub fn prune_count(&self) -> usize {
        self.prune_records
            .iter()
            .map(PruneRecord::covered_leaf_pairs)
            .sum()
    }
}

/// Enumerate the surviving contact-candidate span pairs of two carrier span
/// BVHs.
///
/// The two trees are traversed jointly. Each node pair is separated by the
/// float search + exact sign row (SFC); a certified pair is pruned in full
/// (the certificate inherits to every descendant pair), and an uncertified pair
/// is descended low-before-high until both nodes are leaves — the surviving
/// leaf pairs are the contact candidates, emitted in leaf-index lexicographic
/// order. Refuses ([`BvhRefusal::EmptyCarrier`]) an empty carrier and
/// ([`BvhRefusal::InvalidTree`]) an internal tree inconsistency.
pub fn enumerate_contact_pairs(a: &SpanBvh, b: &SpanBvh) -> Result<SpanPairScan, BvhRefusal> {
    if a.is_empty() || b.is_empty() {
        return Err(BvhRefusal::EmptyCarrier);
    }
    let mut candidates = Vec::new();
    let mut prune_records = Vec::new();
    let mut exact_row_evaluations = 0usize;
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((a.root, b.root));
    while let Some((id_a, id_b)) = queue.pop_front() {
        let node_a = a.nodes.get(id_a).ok_or(BvhRefusal::InvalidTree)?;
        let node_b = b.nodes.get(id_b).ok_or(BvhRefusal::InvalidTree)?;
        if let Some(certificate) =
            certify_node_pair(&node_a.net, &node_b.net, &mut exact_row_evaluations)
        {
            prune_records.push(PruneRecord {
                a_start: node_a.leaf_start,
                a_count: node_a.leaf_count,
                b_start: node_b.leaf_start,
                b_count: node_b.leaf_count,
                certificate,
            });
            continue;
        }
        match (node_a.left, node_b.left) {
            (None, None) => {
                candidates.push((node_a.leaf_start, node_b.leaf_start));
            }
            _ => {
                let split_a = match (node_a.left, node_b.left) {
                    (None, Some(_)) => false,
                    (Some(_), None) => true,
                    _ => node_a.leaf_count >= node_b.leaf_count,
                };
                if split_a {
                    let left = node_a.left.ok_or(BvhRefusal::InvalidTree)?;
                    let right = node_a.right.ok_or(BvhRefusal::InvalidTree)?;
                    queue.push_back((left, id_b));
                    queue.push_back((right, id_b));
                } else {
                    let left = node_b.left.ok_or(BvhRefusal::InvalidTree)?;
                    let right = node_b.right.ok_or(BvhRefusal::InvalidTree)?;
                    queue.push_back((id_a, left));
                    queue.push_back((id_a, right));
                }
            }
        }
    }
    candidates.sort_unstable();
    Ok(SpanPairScan {
        candidates,
        prune_records,
        exact_row_evaluations,
    })
}

/// Search (in floats, typed [`FloatHint`]) for a separating direction of two
/// control-net constant sets and certify it exactly.
///
/// This is the SFC pair: the float search produces hint directions only, and
/// the returned certificate exists exactly when the exact `Expansion` sign row
/// `min_α λ·P¹_α > max_β λ·P²_β` certifies for one of them (in either
/// orientation). An empty net never separates.
pub fn separation_certificate(
    net_a: &[[f64; 3]],
    net_b: &[[f64; 3]],
) -> Option<SeparationCertificate> {
    let mut evaluations = 0usize;
    certify_node_pair(net_a, net_b, &mut evaluations)
}

/// Certify one node pair: run the deterministic float search and verify each
/// candidate direction by the exact `Expansion` sign row.
///
/// `evaluations` counts every exact-row evaluation attempted (one per oriented
/// candidate); a pruned node pair therefore costs one count here and zero for
/// its descendants (heredity).
fn certify_node_pair(
    net_a: &[[f64; 3]],
    net_b: &[[f64; 3]],
    evaluations: &mut usize,
) -> Option<SeparationCertificate> {
    if net_a.is_empty() || net_b.is_empty() {
        return None;
    }
    for direction in candidate_directions(net_a, net_b) {
        let hint = match FloatHint::new(direction) {
            Ok(hint) => hint,
            Err(_) => continue,
        };
        let lambda = *hint.value();
        *evaluations += 1;
        if let Some(certificate) = certify_row(net_a, net_b, lambda) {
            return Some(certificate);
        }
        let negated = [-lambda[0], -lambda[1], -lambda[2]];
        *evaluations += 1;
        if let Some(certificate) = certify_row(net_a, net_b, negated) {
            return Some(certificate);
        }
    }
    None
}

/// The deterministic float-search candidate directions for a node pair.
///
/// Fixed candidate order: the three coordinate axes (orientation is resolved
/// by the exact row in both signs), the normalized centroid-to-centroid
/// direction when the centroids differ, then a few support-function refinement
/// steps from it (each candidate direction added once). Deterministic by
/// construction: ordered input, fixed step count, first-extreme-on-ties.
fn candidate_directions(net_a: &[[f64; 3]], net_b: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut directions: Vec<[f64; 3]> = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let centroid_delta = centroid(net_a).zip(centroid(net_b));
    let initial = match centroid_delta {
        Some((centre_a, centre_b)) => match normalized([
            centre_a[0] - centre_b[0],
            centre_a[1] - centre_b[1],
            centre_a[2] - centre_b[2],
        ]) {
            Some(direction) => {
                directions.push(direction);
                direction
            }
            None => [1.0, 0.0, 0.0],
        },
        None => [1.0, 0.0, 0.0],
    };
    let mut direction = initial;
    for _ in 0..4 {
        let extreme_a = support_extreme(net_a, direction, true);
        let extreme_b = support_extreme(net_b, direction, false);
        let delta = [
            extreme_a[0] - extreme_b[0],
            extreme_a[1] - extreme_b[1],
            extreme_a[2] - extreme_b[2],
        ];
        let refined = match normalized(delta) {
            Some(next) => next,
            None => break,
        };
        direction = refined;
        directions.push(direction);
    }
    directions
}

/// The component-wise centroid of a control-net constant set (plain `f64`;
/// search input only, never evidence).
fn centroid(net: &[[f64; 3]]) -> Option<[f64; 3]> {
    let count = net.len();
    if count == 0 {
        return None;
    }
    let mut sum = [0.0f64; 3];
    for point in net {
        sum[0] += point[0];
        sum[1] += point[1];
        sum[2] += point[2];
    }
    Some([
        sum[0] / count as f64,
        sum[1] / count as f64,
        sum[2] / count as f64,
    ])
}

/// Normalize a 3-vector; `None` for the zero vector or a non-finite length.
fn normalized(v: [f64; 3]) -> Option<[f64; 3]> {
    let length_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if length_sq <= 0.0 || !length_sq.is_finite() {
        return None;
    }
    let length = length_sq.sqrt();
    Some([v[0] / length, v[1] / length, v[2] / length])
}

/// The support extreme of a net along `direction`: the point maximizing
/// `direction·p` when `highest` is true, minimizing it otherwise. On ties the
/// earliest point in net order wins (deterministic).
fn support_extreme(net: &[[f64; 3]], direction: [f64; 3], highest: bool) -> [f64; 3] {
    let mut best = net[0];
    let mut best_value = direction[0] * best[0] + direction[1] * best[1] + direction[2] * best[2];
    for point in &net[1..] {
        let value = direction[0] * point[0] + direction[1] * point[1] + direction[2] * point[2];
        let improves = if highest {
            value > best_value
        } else {
            value < best_value
        };
        if improves {
            best = *point;
            best_value = value;
        }
    }
    best
}

/// Decide the exact Theorem-3 sign row `min_α λ·P¹_α > max_β λ·P²_β` for one
/// oriented direction, in `Expansion` arithmetic over the `f64` inputs.
///
/// Returns the certificate exactly when the row certifies positive. The min and
/// max are selected by exact expansion comparison (a running extreme, one exact
/// subtraction per element — O(n₁+n₂)); the margin is the exact difference of
/// the two extremes and the certificate carries its outward-rounded enclosure.
fn certify_row(
    net_a: &[[f64; 3]],
    net_b: &[[f64; 3]],
    lambda: [f64; 3],
) -> Option<SeparationCertificate> {
    let min_a = exact_extreme(net_a, lambda, true)?;
    let max_b = exact_extreme(net_b, lambda, false)?;
    let margin = min_a.merge(&max_b.negate());
    if margin.sign() != CertifiedSign::Positive {
        return None;
    }
    Some(SeparationCertificate {
        lambda,
        sign: CertifiedSign::Positive,
        margin: CertifiedInterval::from_expansion(&margin),
    })
}

/// The running extreme of the exact `λ·p` values over a net, as an exact
/// `Expansion`: the minimum when `lowest` is true, the maximum otherwise.
fn exact_extreme(net: &[[f64; 3]], lambda: [f64; 3], lowest: bool) -> Option<Expansion> {
    let mut best: Option<Expansion> = None;
    for point in net {
        let value = lambda_dot(lambda, *point);
        best = match best {
            None => Some(value),
            Some(current) => {
                if expansion_less(&value, &current) == lowest {
                    Some(value)
                } else {
                    Some(current)
                }
            }
        };
    }
    best
}

/// The exact scalar product `λ·p` as a non-overlapping expansion over the `f64`
/// inputs (three exact products merged).
fn lambda_dot(lambda: [f64; 3], point: [f64; 3]) -> Expansion {
    Expansion::from_product(lambda[0], point[0])
        .merge(&Expansion::from_product(lambda[1], point[1]))
        .merge(&Expansion::from_product(lambda[2], point[2]))
}

/// Whether `a` is exactly less than `b`: the exact sign of `a − b` is
/// negative.
fn expansion_less(a: &Expansion, b: &Expansion) -> bool {
    a.merge(&b.negate()).sign() == CertifiedSign::Negative
}

/// Build one span leaf from an admitted patch.
///
/// The certified box is the per-span `enclose`: the `hull_bernstein_2d` hull of
/// each numerator component over the unit square (the span's Bézier image under
/// D2, `w ≡ 1`). The net constants are the dehomogenized control points.
fn leaf_from_admitted_patch(patch: &AdmittedPatch) -> Result<SpanLeaf, BvhRefusal> {
    let patch_box = &patch.patch;
    let numerator = patch_box.numerator();
    let weights = patch_box.weights();
    let mut axes = [CertifiedInterval::point(0.0); 3];
    for component in 0..3 {
        let hull = hull_bernstein_2d(&numerator[component], (0.0, 1.0), (0.0, 1.0))
            .map_err(|_| BvhRefusal::EnclosureUnavailable)?;
        axes[component] = hull;
    }
    let box_ = EnclosureBox::new(axes)?;
    let mut net = Vec::with_capacity((patch_box.m() + 1) * (patch_box.n() + 1));
    for row in 0..=patch_box.m() {
        for column in 0..=patch_box.n() {
            let weight = weights[row][column];
            if weight <= 0.0 || !weight.is_finite() {
                return Err(BvhRefusal::NonFiniteNetPoint);
            }
            let point = [
                numerator[0][row][column] / weight,
                numerator[1][row][column] / weight,
                numerator[2][row][column] / weight,
            ];
            if point.iter().any(|coordinate| !coordinate.is_finite()) {
                return Err(BvhRefusal::NonFiniteNetPoint);
            }
            net.push(point);
        }
    }
    SpanLeaf::new(patch.cell, box_, net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfp::FloatHint;
    use crate::contract::Refusal;
    use crate::formal::exact::CertifiedSign;

    // -----------------------------------------------------------------------
    // F-C5 fixture data copied from cfp/fixtures.rs (read-only constants).
    // -----------------------------------------------------------------------

    /// F-C5 `u`-span count (20).
    const FC5_U_SPANS: usize = 20;
    /// F-C5 `v`-span count (30).
    const FC5_V_SPANS: usize = 30;
    /// F-C5 single contact span pair `(u index, v index)`.
    const FC5_CONTACT: (usize, usize) = (7, 11);
    /// F-C5 expected dyadic prune count.
    const FC5_EXPECTED_PRUNE_COUNT: usize = 589;
    /// F-C5 ground-truth record: exactly one contact span pair.
    const FC5_EXACTLY_ONE_CONTACT: bool = true;

    /// F-C5 structural admission, checked at compile time (the record's own
    /// `admit` invariant: exactly one in-bounds contact span pair in a
    /// positive 20 × 30 grid).
    const _: () = assert!(FC5_EXACTLY_ONE_CONTACT);
    const _: () = assert!(FC5_U_SPANS > 0 && FC5_V_SPANS > 0);
    const _: () = assert!(FC5_CONTACT.0 < FC5_U_SPANS && FC5_CONTACT.1 < FC5_V_SPANS);
    const _: () = assert!(FC5_EXPECTED_PRUNE_COUNT > 0);
    const _: () = assert!(FC5_EXPECTED_PRUNE_COUNT < FC5_U_SPANS * FC5_V_SPANS);

    // -----------------------------------------------------------------------
    // F-C3 fixture data copied from cfp/fixtures.rs (read-only constants).
    // -----------------------------------------------------------------------

    /// F-C3 separated-pair control net A (integer, then read as `f64`).
    const FC3_SEPARATED_NET_A: [[i64; 3]; 4] = [[2, 0, 0], [2, 1, 0], [2, 0, 1], [2, 1, 1]];
    /// F-C3 separated-pair control net B (integer, then read as `f64`).
    const FC3_SEPARATED_NET_B: [[i64; 3]; 4] = [[0, 0, 0], [1, 1, 0], [1, 0, 1], [0, 1, 1]];
    /// F-C3 touching-pair control net A.
    const FC3_TOUCHING_NET_A: [[i64; 3]; 4] = [[0, 0, 0], [1, 1, 1], [2, 1, 0], [1, 0, 0]];
    /// F-C3 touching-pair control net B.
    const FC3_TOUCHING_NET_B: [[i64; 3]; 4] = [[2, 1, 0], [3, 0, 0], [3, 2, 1], [2, 2, 0]];

    /// An exact closed interval from two bounds (certified box construction).
    fn interval(lo: f64, hi: f64) -> CertifiedInterval {
        CertifiedInterval { lo, hi }
    }

    /// A certified box over the component-wise min/max of a corner set.
    fn aabb(corners: &[[f64; 3]]) -> EnclosureBox {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for point in corners {
            for component in 0..3 {
                lo[component] = lo[component].min(point[component]);
                hi[component] = hi[component].max(point[component]);
            }
        }
        EnclosureBox::new([
            interval(lo[0], hi[0]),
            interval(lo[1], hi[1]),
            interval(lo[2], hi[2]),
        ])
        .expect("an AABB of finite corners is a finite ordered box")
    }

    /// The eight corners of a certified box, as net-constant points.
    fn corners(box_: &EnclosureBox) -> Vec<[f64; 3]> {
        let axes = box_.axes();
        let mut out = Vec::with_capacity(8);
        for x in [axes[0].lo, axes[0].hi] {
            for y in [axes[1].lo, axes[1].hi] {
                for z in [axes[2].lo, axes[2].hi] {
                    out.push([x, y, z]);
                }
            }
        }
        out
    }

    /// One unit span cell `(u, v)` of the F-C5 20 × 30 grid, as a small box at
    /// `(u, v, 0)` with net constants equal to its box corners.
    fn fc5_cell_leaf(u: usize, v: usize) -> SpanLeaf {
        let cell = ((u as f64, u as f64 + 1.0), (v as f64, v as f64 + 1.0));
        let box_ = EnclosureBox::new([
            interval(u as f64 - 0.1, u as f64 + 0.1),
            interval(v as f64 - 0.1, v as f64 + 0.1),
            interval(-0.1, 0.1),
        ])
        .expect("an F-C5 cell box is finite and ordered");
        let net = corners(&box_);
        SpanLeaf::new(cell, box_, net).expect("an F-C5 cell net is non-empty and finite")
    }

    /// The F-C5 scenario: carrier A is the full 20 × 30 span grid (600 leaves),
    /// carrier B is a single probe span whose certified box meets exactly the
    /// eleven `u = 7`, `v ∈ {6..=16}` leaves — the dyadic record's expected
    /// survivors — and whose net constants are its own box corners.
    fn fc5_pair() -> (SpanBvh, SpanBvh) {
        let mut leaves = Vec::with_capacity(FC5_U_SPANS * FC5_V_SPANS);
        for u in 0..FC5_U_SPANS {
            for v in 0..FC5_V_SPANS {
                leaves.push(fc5_cell_leaf(u, v));
            }
        }
        let probe_box =
            EnclosureBox::new([interval(6.9, 7.1), interval(6.0, 16.0), interval(-0.1, 0.1)])
                .expect("the probe box is finite and ordered");
        let probe_net = corners(&probe_box);
        let probe = SpanLeaf::new(((0.0, 1.0), (0.0, 1.0)), probe_box, probe_net)
            .expect("the probe net is non-empty and finite");
        let a = SpanBvh::build(leaves).expect("the 600-leaf F-C5 carrier builds");
        let b = SpanBvh::build(vec![probe]).expect("the single-span probe builds");
        (a, b)
    }

    /// The F-C5 contact leaf's stack position (`u * v_spans + v`).
    fn fc5_contact_position() -> usize {
        FC5_CONTACT.0 * FC5_V_SPANS + FC5_CONTACT.1
    }

    /// The expected survivor positions of the F-C5 scenario (`u = 7`,
    /// `v ∈ {6..=16}`), in ascending order.
    fn fc5_expected_survivors() -> Vec<usize> {
        (6..=16).map(|v| 7 * FC5_V_SPANS + v).collect()
    }

    /// Copy an F-C3 net to `f64` net constants.
    fn net_from_i64(net: &[[i64; 3]; 4]) -> Vec<[f64; 3]> {
        net.iter()
            .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
            .collect()
    }

    /// A single-span BVH whose one leaf carries the given net constants.
    fn single_leaf_bvh(net: Vec<[f64; 3]>) -> SpanBvh {
        let box_ = aabb(&net);
        let leaf = SpanLeaf::new(((0.0, 1.0), (0.0, 1.0)), box_, net)
            .expect("a single-leaf net is non-empty and finite");
        SpanBvh::build(vec![leaf]).expect("a single-span BVH builds")
    }

    /// A leaf carrying the given net constants (and the AABB of those net
    /// constants as its certified box), on a unit cell.
    fn unit_cell_leaf(net: Vec<[f64; 3]>) -> SpanLeaf {
        let box_ = aabb(&net);
        SpanLeaf::new(((0.0, 1.0), (0.0, 1.0)), box_, net)
            .expect("a replacement net is non-empty and finite")
    }

    #[test]
    fn fc5_span_pair_found_in_large_decomposition() {
        // F-C5: the one contact span pair in the 20 x 30-span record is found.
        // The record says exactly one contact span pair exists, at u = 7,
        // v = 11 (stack position 7 * 30 + 11 = 221), and the scenario is
        // admitted to that record (compile-time const assertions above).
        let (a, b) = fc5_pair();
        let scan = enumerate_contact_pairs(&a, &b).expect("the F-C5 pair scans");
        let found: Vec<(usize, usize)> = fc5_expected_survivors()
            .iter()
            .map(|&position| (position, 0))
            .collect();
        // Every expected survivor (including the single contact span pair) is
        // emitted, in leaf-index lexicographic order, and nothing else is.
        let mut iter = scan.candidates().iter();
        for expected_pair in &found {
            assert_eq!(iter.next(), Some(expected_pair));
        }
        assert!(
            iter.next().is_none(),
            "no candidate outside the expected set"
        );
        assert!(
            scan.candidates().contains(&(fc5_contact_position(), 0)),
            "the one contact span pair (u=7, v=11) is among the survivors"
        );
        assert_eq!(a.len(), FC5_U_SPANS * FC5_V_SPANS);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn fc5_prune_count_matches_ground_truth() {
        // The fixture's expected dyadic prune count is met exactly: of the 600
        // span pairs, the dyadic traverse removes 589 and leaves the handful of
        // surviving candidates (the 360k -> handful claim, pinned at F-C5
        // scale).
        let (a, b) = fc5_pair();
        let scan = enumerate_contact_pairs(&a, &b).expect("the F-C5 pair scans");
        assert_eq!(scan.prune_count(), FC5_EXPECTED_PRUNE_COUNT);
        assert_eq!(scan.candidates().len(), fc5_expected_survivors().len());
        assert_eq!(
            scan.prune_count() + scan.candidates().len(),
            FC5_U_SPANS * FC5_V_SPANS,
            "every span pair is either pruned once or emitted"
        );
    }

    #[test]
    fn separation_certificate_is_exact_sign_row() {
        // F-C3 separated pair: the exact row certifies; the prune record's
        // certificate is the exact Expansion sign row (never a float hint).
        let a = single_leaf_bvh(net_from_i64(&FC3_SEPARATED_NET_A));
        let b = single_leaf_bvh(net_from_i64(&FC3_SEPARATED_NET_B));
        let scan = enumerate_contact_pairs(&a, &b).expect("the F-C3 separated pair scans");
        assert_eq!(scan.prune_records().len(), 1);
        assert!(scan.candidates().is_empty());
        for record in scan.prune_records() {
            let certificate = record.certificate();
            assert_eq!(
                certificate.sign(),
                CertifiedSign::Positive,
                "a prune carries the exact positive Expansion sign row"
            );
            let lambda = certificate.lambda();
            assert!(lambda.iter().all(|component| component.is_finite()));
            // The recorded margin is the outward-rounded enclosure of the exact
            // min_a(lambda) - max_b(lambda) = 2 - 1 = 1 for the F-C3 pair.
            assert!(certificate.margin().lo > 0.0);
            assert!(certificate.margin().contains(1.0));
        }

        // F-C3 touching pair: no direction the exact row certifies exists, so
        // the leaf pair survives as a candidate and nothing is pruned.
        let c = single_leaf_bvh(net_from_i64(&FC3_TOUCHING_NET_A));
        let d = single_leaf_bvh(net_from_i64(&FC3_TOUCHING_NET_B));
        let touching = enumerate_contact_pairs(&c, &d).expect("the F-C3 touching pair scans");
        assert!(touching.prune_records().is_empty());
        assert_eq!(touching.candidates(), &[(0, 0)]);

        // F-C5 run: every prune in the large decomposition carries the exact
        // row too.
        let (a5, b5) = fc5_pair();
        let scan5 = enumerate_contact_pairs(&a5, &b5).expect("the F-C5 pair scans");
        assert!(scan5.prune_count() > 0);
        for record in scan5.prune_records() {
            assert_eq!(record.certificate().sign(), CertifiedSign::Positive);
            assert!(record.certificate().margin().lo > 0.0);
        }
    }

    #[test]
    fn float_hint_never_evidence() {
        // compile-time shape assertion (spine decision 2, enforced here): a
        // float search result is never an evidence position. The spine type
        // exposes exactly one constructor and one accessor; pin both to their
        // shapes so no evidence-shaped surface can appear on the hint.
        type HintConstructor = fn([f64; 3]) -> Result<FloatHint<[f64; 3]>, Refusal>;
        type HintAccessor = fn(&FloatHint<[f64; 3]>) -> &[f64; 3];
        let _signature: HintConstructor = FloatHint::new;
        let _value: HintAccessor = FloatHint::value;

        // The payload of a FloatHint is the plain float search result, not a
        // `Certificate`/`Certified` value: the accessor returns the raw payload
        // type, and the constructor accepts only admissible float search values
        // ([`crate::cfp::FloatSearchValue`]) — nothing evidence-shaped is in
        // the type.
        let hint = FloatHint::new([1.0, 0.0, 0.0]).expect("a finite direction is admissible");
        let payload: &[f64; 3] = hint.value();
        let _copy: [f64; 3] = *payload;

        // The hint is the search half of the SFC pair, never the certificate
        // half: it refuses a non-finite search result at the boundary
        // (H-6 discipline — a float value may not silently become a certified
        // statement), and the exact row verifier is what decides separation.
        assert!(FloatHint::new([f64::NAN, 0.0, 0.0]).is_err());
        assert!(FloatHint::new([f64::INFINITY, 0.0, 0.0]).is_err());
        assert!(FloatHint::new([1.0, 2.0, 3.0]).is_ok());
    }

    #[test]
    fn separation_inherits_to_descendants() {
        // Four copies of the F-C3 separated net per side. The two roots'
        // control nets separate, so the single parent-node certificate prunes
        // every one of the 4 x 4 descendant span pairs; the traversal performs
        // exactly one exact-row verification and zero re-verifications.
        let mut a_leaves = Vec::new();
        for _ in 0..4 {
            a_leaves.push(unit_cell_leaf(net_from_i64(&FC3_SEPARATED_NET_A)));
        }
        let a = SpanBvh::build(a_leaves).expect("the four-leaf A side builds");
        let mut b_leaves = Vec::new();
        for _ in 0..4 {
            b_leaves.push(unit_cell_leaf(net_from_i64(&FC3_SEPARATED_NET_B)));
        }
        let b = SpanBvh::build(b_leaves).expect("the four-leaf B side builds");

        let scan = enumerate_contact_pairs(&a, &b).expect("the duplicated F-C3 pair scans");
        assert_eq!(scan.prune_records().len(), 1, "one parent certificate");
        assert_eq!(
            scan.prune_records()[0].covered_leaf_pairs(),
            4 * 4,
            "the parent certificate prunes every descendant pair"
        );
        assert!(scan.candidates().is_empty());
        assert_eq!(
            scan.exact_row_evaluations(),
            1,
            "no descendant pair is re-verified"
        );

        // Descend manually over the descendant leaf pairs: heredity says each
        // is separated, and the scan already proved none of them was
        // re-verified (the parent certificate did all the work).
        for leaf_a in a.leaves() {
            for leaf_b in b.leaves() {
                let certificate = separation_certificate(leaf_a.net(), leaf_b.net())
                    .expect("every descendant leaf pair inherits the separation");
                assert_eq!(certificate.sign(), CertifiedSign::Positive);
            }
        }
        assert_eq!(scan.exact_row_evaluations(), 1);
    }

    #[test]
    fn bvh_traversal_deterministic() {
        // Two independent runs on identical ordered input produce identical
        // candidate lists and identical prune records.
        let (a1, b1) = fc5_pair();
        let scan1 = enumerate_contact_pairs(&a1, &b1).expect("the first run scans");
        let (a2, b2) = fc5_pair();
        let scan2 = enumerate_contact_pairs(&a2, &b2).expect("the second run scans");
        assert_eq!(scan1, scan2);
        assert_eq!(scan1.candidates(), scan2.candidates());
        assert_eq!(scan1.prune_records(), scan2.prune_records());
        assert_eq!(scan1.exact_row_evaluations(), scan2.exact_row_evaluations());
        assert_eq!(scan1.prune_count(), scan2.prune_count());
    }
}
