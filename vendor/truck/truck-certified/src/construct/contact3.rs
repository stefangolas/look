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

//! CC-020-CONTACT-K3 (spine seam S11): the three-support constrained contact
//! system — the reduced-variable mapping, the arity-4 Krawczyk solve, and the
//! typed node outcome (theory §3.1–3.3).
//!
//! # The reduction (the packet's Section 1, realized here)
//!
//! The raw k = 3 contact equations `c = S_i(u_i, v_i) + ε_i r n̂_i(u_i, v_i)`
//! (`i = 1..3`) plus the radius-law closure are seven unknowns in a
//! seven-equation square form, above the landed engine's arity. The packet's
//! reduction eliminates the centre and the per-support parameter pairs and
//! leaves a **four-unknown square system in `(r, c)`** — the radius `r` and the
//! three coordinates of the contact centre `c`. This is the named chart choice
//! of this packet (recorded in RESULT as the worker's one judgement): at a
//! transverse triple tangency the centre determines the three contact
//! parameters (the normal-field / closest-point parametrization is a local
//! diffeomorphism when the three certified surface normals span ℝ³), so the
//! centre's three coordinates are exactly the three independent tangential
//! degrees of freedom the packet's reduced system keeps ("one tangent
//! parameter per support", in the classical normal-field sense — for a
//! trihedral of coordinate planes the centre coordinates *are* the in-plane
//! contact parameters, `q_1 = (0, c_y, c_z)` etc.). The reduction is therefore
//! the ≤4-unknown square form the spine's S11 arity claim books, and its
//! submersion is certifiable (Section 3 ground truth, `η_F` margin below).
//!
//! The three equations are the **signed offset conditions** to the three
//! supports: with the oriented unit normal `n̂_i` of support `i` (the certified
//! map's `S_u × S_v` orientation) and any base point `a_i` on the surface,
//!
//! ```text
//! F_i(c, r) = (c − a_i) · n̂_i − ε_i r = 0,   i = 1..3,
//! F_4(c, r) = r − ρ = 0                        (the radius-law closure)
//! ```
//!
//! The fourth equation closes through `radius_eval` (the A4 anchor). The
//! packet's pre-made projection choice — "the two supports with the largest
//! certified rank margins, evaluated first in a fixed order" — is realized as
//! the deterministic [`ReducedSystem`] chart record `projection`: the supports
//! sorted by certified rank margin (ties by index). The centre chart keeps all
//! three offset rows (they are symmetric; the margin ordering is the
//! deterministic tie-break and the record consumers read), so the certified
//! submersion margin is over the full 4×4 Jacobian whose three top rows are the
//! oriented normals.
//!
//! **Support class (scope guard).** Each support must be *affine over its
//! certified region* (identically zero second-partial Bernstein coefficient
//! grids over every touched patch, with one shared tangent frame): the flat
//! offset-corner class that CC-021 offset corner strata and CC-033 setback
//! corners consume. A support that is not affine over its region is refused
//! [`ConstructRefusal::InvalidInput`] at [`ReducedSystem::try_new`] — the
//! certified signed-distance model is exact only on that class, and a curved
//! support would need the parameter-space system a later packet books.
//!
//! **Conventions recorded here (caller-level facts until CC-021 books them).**
//! The `ε_i` side sign is `+1` when the contact centre lies on the `+n̂_i` side
//! of support `i` at distance `r`, `−1` otherwise — supplied by the caller per
//! support, since a `SurfaceRegion` carries no side data. The radius-law
//! closure `ρ` is [`radius_eval`] over the law's full normalized arc, and a
//! non-degenerate (non-point) closure is refused: an isolated junction node
//! fixes a single radius, which only a point-valued law (the constant family)
//! certifies; an arc-parametrized law needs CC-021's junction stratum.
//!
//! # The solver (the packet's Section 2)
//!
//! [`ReducedSystem`] carries the chart record, the four-variable box
//! [`IBox4`], and the Krawczyk evaluation of the reduced `F` and `DF` over
//! that box (the [`SquareResidualEval`] impl), consuming
//! [`krawczyk_c1_n4`] — the engine's Lemma-8.0 contraction, never a
//! hand-rolled Newton. [`solve_triple_node`] refines the S11 seam's result:
//! contraction with strict interior → the node packaged as
//! [`TripleContactNode`], with the submersion margin `η_F` certified from the
//! interval `DF` (`DF·DFᵀ ⪰ η_F²·I` componentwise through the interval
//! principal minors; `η_F` below the `CC_ETA_J`-class floor →
//! [`ConstructRefusal::RankDeficientContact`]); a Krawczyk disproof is a
//! **certified** [`TripleNodeOutcome::Empty`], not a refusal; an unresolved
//! cell is bisected to [`CC_DEPTH_MAX`] and the depth cap refuses
//! `RankDeficientContact`. The rank-deficiency refusal fires before any
//! iteration (the whole-seed submersion pre-check).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`. Every float reduction runs in a fixed order
//! with directed rounding (C9).

use crate::certified_map::{CertifiedSurfaceMap, SurfaceRegion};
use crate::construct::canal::radius_eval;
use crate::construct::config::{CC_DEPTH_MAX, CC_ETA_J};
use crate::construct::refusal::ConstructRefusal;
use crate::construct::stubs::{RadiusLaw, TripleContactNode};
use crate::construct::Interval;
use crate::hull::bernstein_derivative_2d;
use crate::kernel::certs::IBox4;
use crate::kernel::engine::{krawczyk_c1_n4, SquareResidualEval};
use crate::kernel::evidence::ClaimVerdict;
use crate::kernel::patch::CertifiedPositive;
use truck_base::evidence::Budget;

/// The reduced-variable mapping of the k = 3 contact system (spine S11): the
/// chart record, the four-variable box, and the certified evaluation of the
/// reduced `F` and `DF` over that box.
///
/// The four reduced variables are ordered `(c_x, c_y, c_z, r)` — the centre of
/// the contact ball and its radius. The chart is constructed from three
/// certified support maps that are affine over their certified regions (the
/// offset-corner class), the caller's signed offset sides `ε_i`, and a
/// point-valued radius-law closure.
#[derive(Debug, Clone, PartialEq)]
pub struct ReducedSystem {
    /// The per-support chart records, in the caller's index order.
    supports: [SupportData; 3],
    /// The deterministic chart selection: the two supports with the largest
    /// certified rank margins, ordered by certified margin (ties by index).
    projection: [usize; 2],
    /// The four-variable box `(c_x, c_y, c_z, r)` over which the system runs.
    reduced: IBox4,
    /// The point-valued radius-law closure `ρ` of the isolated junction.
    radius: Interval,
}

/// One flat-support chart record: the affine data of a support over its
/// certified region, recovered exactly from the certified map's Bézier grids.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SupportData {
    /// A base point `a_i` on the support plane (a patch-lower-left value).
    base: [f64; 3],
    /// The source-parameter tangent vector `S_u`.
    su: [f64; 3],
    /// The source-parameter tangent vector `S_v`.
    sv: [f64; 3],
    /// The oriented unit normal `n̂_i = (S_u × S_v)/|S_u × S_v|`.
    normal: [f64; 3],
    /// The signed offset side `ε_i ∈ {+1, −1}`.
    eps: f64,
    /// The source parameter origin `(u_0, v_0)` of the recovered affine chart.
    origin: (f64, f64),
}

/// The S11 k=3 typed solver outcome (the packet's seam refinement).
///
/// S11's `Result<TripleContactNode, _>` is refined: an `Empty` arm is a
/// **certified** answer (Krawczyk disproof or a root-free exclusion cleared
/// the whole box), never a refusal. `solve_triple_node` returns
/// `Result<TripleNodeOutcome, ConstructRefusal>`.
#[derive(Debug, Clone, PartialEq)]
pub enum TripleNodeOutcome {
    /// A certified triple-contact node.
    Node(TripleContactNode),
    /// A certified absence: no node in the reduced box.
    Empty,
}

impl ReducedSystem {
    /// Build the reduced system over the three certified support maps.
    ///
    /// `maps` are the three admitted surfaces, `regions` their certified
    /// regions (in the same index order), `eps` the signed offset sides
    /// (`+1` = centre on the `+n̂` side), `radius` the radius law, and `reduced`
    /// the four-variable seed box `(c_x, c_y, c_z, r)`.
    ///
    /// Refuses [`ConstructRefusal::InvalidInput`] on a non-affine support
    /// region, a non-compact or degenerate region request, a non-±1 side, a
    /// non-positive or non-degenerate radius-law closure, or a zero certified
    /// rank margin on any support.
    pub fn try_new(
        maps: [&CertifiedSurfaceMap; 3],
        regions: [SurfaceRegion; 3],
        eps: [f64; 3],
        radius: &RadiusLaw,
        reduced: IBox4,
    ) -> Result<ReducedSystem, ConstructRefusal> {
        let mut supports = [SupportData::dummy(); 3];
        let mut margins = [0.0_f64; 3];
        for (i, (map, region)) in maps.iter().zip(regions.iter()).enumerate() {
            if !eps[i].is_finite() || (eps[i] != 1.0 && eps[i] != -1.0) {
                return Err(ConstructRefusal::InvalidInput);
            }
            margins[i] = region_margin(map, *region)?;
            supports[i] = support_data(map, *region, eps[i])?;
        }
        let radius_value = radius_eval(radius, Interval { lo: 0.0, hi: 1.0 })
            .map_err(|_| ConstructRefusal::InvalidInput)?;
        if radius_value.lo <= 0.0 || !radius_value.is_degenerate() {
            return Err(ConstructRefusal::InvalidInput);
        }
        let projection = projection_order(&margins);
        Ok(ReducedSystem {
            supports,
            projection,
            reduced,
            radius: radius_value,
        })
    }

    /// The reduced box this system was built over.
    pub fn reduced_box(&self) -> IBox4 {
        self.reduced
    }

    /// The deterministic chart selection: the two supports with the largest
    /// certified rank margins (fixed order), the packet's pre-made choice.
    pub fn projection(&self) -> [usize; 2] {
        self.projection
    }

    /// Whether the interval Jacobian certifies the submersion margin floor
    /// `DF·DFᵀ ⪰ η_F²·I` with `η_F ≥ CC_ETA_J` (the `CC_ETA_J`-class floor).
    ///
    /// The certificate is componentwise: over the interval Gram matrix
    /// `G = □(DF)·□(DF)ᵀ` the shifted matrix `G − CC_ETA_J²·I` must certify
    /// positive semidefinite, checked through the interval enclosures of ALL
    /// of its principal minors (a symmetric matrix is PSD iff every principal
    /// minor is non-negative, and an interval enclosure with a non-negative
    /// lower endpoint certifies that for every matrix in the family). For the
    /// affine class the interval `DF` is the exact constant Jacobian, so the
    /// certificate decides cleanly: a transverse triple (spanning normals)
    /// passes with a wide margin; a structural rank drop (e.g. two coincident
    /// planes) has a zero smallest eigenvalue and the determinant minor of the
    /// shifted Gram is certified negative.
    fn submersion_certified(&self) -> bool {
        let jac = self.interval_jacobian();
        let mut gram = [[Interval::point(0.0); 4]; 4];
        // Fixed 4x4 matrix indices over the interval Jacobian; the index form
        // is the Gram algebra (the engine's matrix discipline).
        #[allow(clippy::needless_range_loop)]
        for r in 0..4 {
            for c in 0..4 {
                let mut acc = Interval::point(0.0);
                for k in 0..4 {
                    acc = acc.add(&jac[r][k].mul(&jac[c][k]));
                }
                gram[r][c] = acc;
            }
        }
        let eta2 = Interval::point(CC_ETA_J * CC_ETA_J);
        let mut shifted = [[Interval::point(0.0); 4]; 4];
        // Fixed 4x4 matrix indices; the diagonal shift is the PSD certificate.
        #[allow(clippy::needless_range_loop)]
        for r in 0..4 {
            for c in 0..4 {
                shifted[r][c] = if r == c {
                    gram[r][c].sub(&eta2)
                } else {
                    gram[r][c]
                };
            }
        }
        principal_minors_nonneg(&shifted)
    }

    /// The exact interval Jacobian of the reduced system: rows `0..3` are the
    /// oriented normals with the `−ε_i` radius entry, row `3` is the radius-law
    /// row `[0, 0, 0, 1]`. Affine supports make the Jacobian constant over any
    /// box, so the interval Jacobian is box-independent.
    fn interval_jacobian(&self) -> [[Interval; 4]; 4] {
        let mut jac = [[Interval::point(0.0); 4]; 4];
        for (i, support) in self.supports.iter().enumerate() {
            for (k, nk) in support.normal.iter().enumerate() {
                jac[i][k] = Interval::point(*nk);
            }
            jac[i][3] = Interval::point(-support.eps);
        }
        jac[3][3] = Interval::point(1.0);
        jac
    }

    /// The certified residual enclosure over the box `(c, r)`.
    fn residual(&self, c: &[Interval; 3], r: &Interval) -> [Interval; 4] {
        let mut f = [Interval::point(0.0); 4];
        for (i, support) in self.supports.iter().enumerate() {
            let mut dot = Interval::point(0.0);
            for (ck, (nk, bk)) in c.iter().zip(support.normal.iter().zip(support.base.iter())) {
                let d = ck.sub(&Interval::point(*bk));
                dot = dot.add(&d.mul(&Interval::point(*nk)));
            }
            f[i] = dot.sub(&Interval::point(support.eps).mul(r));
        }
        f[3] = r.sub(&self.radius);
        f
    }

    /// Whether the residual excludes zero in some component over the whole
    /// box (a certified root-free exclusion).
    fn excludes_zero(&self, b: &IBox4) -> bool {
        let (c, r) = box_ivs(b);
        self.residual(&c, &r)
            .iter()
            .any(|component| !component.contains(0.0))
    }

    /// Package a certified node from the proven box: the centre and radius
    /// enclosures are the box axes, and the per-support contact parameter
    /// enclosures come from the certified orthogonal projection of the centre
    /// onto each support plane, inverted through the affine chart.
    fn node_from(&self, b: &IBox4) -> Result<TripleContactNode, ConstructRefusal> {
        let centre = [
            Interval {
                lo: b.lo[0],
                hi: b.hi[0],
            },
            Interval {
                lo: b.lo[1],
                hi: b.hi[1],
            },
            Interval {
                lo: b.lo[2],
                hi: b.hi[2],
            },
        ];
        let radius = Interval {
            lo: b.lo[3],
            hi: b.hi[3],
        };
        let mut contacts = [[Interval::point(0.0); 2]; 3];
        for (i, support) in self.supports.iter().enumerate() {
            let pair = self.contact_parameters(support, &centre)?;
            contacts[i] = pair;
        }
        Ok(TripleContactNode {
            centre,
            radius,
            contacts,
        })
    }

    /// The certified contact parameter pair of one support over the centre
    /// box: project the centre orthogonally onto the support plane, then
    /// invert the affine chart `S(u, v) = base + (u − u_0) su + (v − v_0) sv`
    /// through the 2×2 Gram solve. Refuses when the chart Gram determinant
    /// cannot certify non-zero (a degenerate chart).
    fn contact_parameters(
        &self,
        support: &SupportData,
        centre: &[Interval; 3],
    ) -> Result<[Interval; 2], ConstructRefusal> {
        let n = support.normal;
        let base = support.base;
        let mut dist = Interval::point(0.0);
        for (ck, (bk, nk)) in centre.iter().zip(base.iter().zip(n.iter())) {
            let d = ck.sub(&Interval::point(*bk));
            dist = dist.add(&d.mul(&Interval::point(*nk)));
        }
        let mut foot = [Interval::point(0.0); 3];
        for (fk, (ck, nk)) in foot.iter_mut().zip(centre.iter().zip(n.iter())) {
            *fk = ck.sub(&dist.mul(&Interval::point(*nk)));
        }
        let p = support.su;
        let q = support.sv;
        let mut w = [Interval::point(0.0); 3];
        for (wk, (fk, bk)) in w.iter_mut().zip(foot.iter().zip(base.iter())) {
            *wk = fk.sub(&Interval::point(*bk));
        }
        let d0 = Interval::point(p[0])
            .mul(&w[0])
            .add(&Interval::point(p[1]).mul(&w[1]))
            .add(&Interval::point(p[2]).mul(&w[2]));
        let d1 = Interval::point(q[0])
            .mul(&w[0])
            .add(&Interval::point(q[1]).mul(&w[1]))
            .add(&Interval::point(q[2]).mul(&w[2]));
        let g11 = Interval::point(dot3(&p, &p));
        let g12 = Interval::point(dot3(&p, &q));
        let g22 = Interval::point(dot3(&q, &q));
        let det = g11.mul(&g22).sub(&g12.mul(&g12));
        let t0 = g22
            .mul(&d0)
            .sub(&g12.mul(&d1))
            .div(&det)
            .ok_or(ConstructRefusal::RankDeficientContact)?;
        let t1 = g11
            .mul(&d1)
            .sub(&g12.mul(&d0))
            .div(&det)
            .ok_or(ConstructRefusal::RankDeficientContact)?;
        let u = Interval::point(support.origin.0).add(&t0);
        let v = Interval::point(support.origin.1).add(&t1);
        Ok([u, v])
    }
}

impl SquareResidualEval for ReducedSystem {
    fn arity(&self) -> usize {
        4
    }

    fn eval(&self, b: &[Interval]) -> Vec<Interval> {
        let boxed = match box_from_slice(b) {
            Some(boxed) => boxed,
            None => return vec![unbounded(); 4],
        };
        let (c, r) = box_ivs(&boxed);
        self.residual(&c, &r).to_vec()
    }

    fn jac_encl(&self, b: &[Interval]) -> Vec<Vec<Interval>> {
        if b.len() != 4 || b.iter().any(|iv| !iv.is_finite()) {
            return vec![vec![unbounded(); 4]; 4];
        }
        self.interval_jacobian()
            .iter()
            .map(|row| row.to_vec())
            .collect()
    }
}

/// Run the S11 refined solver over a reduced system: certify submersion on
/// the whole seed (before any iteration), then subdivide-and-certify with the
/// engine's arity-4 Krawczyk contraction, honoring [`Budget`] and the
/// [`CC_DEPTH_MAX`] cap.
///
/// * Whole-seed submersion below the floor → [`ConstructRefusal::RankDeficientContact`]
///   (fires before any iteration — the structural rank-drop path).
/// * Contraction with strict interior → the node, after re-certifying the
///   submersion floor over the proven box.
/// * Krawczyk disproof or a root-free residual exclusion clears the cell; an
///   exhausted stack is the certified [`TripleNodeOutcome::Empty`].
/// * An unresolved cell is bisected to [`CC_DEPTH_MAX`]; the depth cap refuses
///   `RankDeficientContact`. Budget exhaustion refuses the same way.
pub fn solve_triple_node(
    system: &ReducedSystem,
    budget: &mut Budget,
) -> Result<TripleNodeOutcome, ConstructRefusal> {
    if !system.submersion_certified() {
        return Err(ConstructRefusal::RankDeficientContact);
    }
    let weight = match CertifiedPositive::try_new(1.0) {
        Ok(weight) => weight,
        Err(_) => return Err(ConstructRefusal::InvalidInput),
    };
    let mut stack: Vec<(IBox4, u32)> = vec![(system.reduced, 0)];
    while let Some((b, depth)) = stack.pop() {
        if system.excludes_zero(&b) {
            continue;
        }
        budget
            .spend_newton(1)
            .map_err(|_| ConstructRefusal::RankDeficientContact)?;
        match krawczyk_c1_n4(system, b, &[weight]) {
            ClaimVerdict::Proven(cert) => {
                if !system.submersion_certified() {
                    return Err(ConstructRefusal::RankDeficientContact);
                }
                let node = system.node_from(&cert.box_)?;
                return Ok(TripleNodeOutcome::Node(node));
            }
            ClaimVerdict::Disproven(_) => {}
            ClaimVerdict::Inconclusive(_) => {
                if depth >= CC_DEPTH_MAX {
                    return Err(ConstructRefusal::RankDeficientContact);
                }
                let children = bisect4(&b);
                if children.is_empty() {
                    return Err(ConstructRefusal::RankDeficientContact);
                }
                budget
                    .spend_depth()
                    .map_err(|_| ConstructRefusal::RankDeficientContact)?;
                for child in children {
                    budget
                        .spend_subdiv(1)
                        .map_err(|_| ConstructRefusal::RankDeficientContact)?;
                    stack.push((child, depth + 1));
                }
            }
        }
    }
    Ok(TripleNodeOutcome::Empty)
}

/// The deterministic projection-pair selection: sort the support indices by
/// certified rank margin (descending), ties broken by index, keep the two
/// largest (the packet's pre-made "two supports with the largest certified
/// rank margins, evaluated first in a fixed order").
fn projection_order(margins: &[f64; 3]) -> [usize; 2] {
    let mut order = [0usize, 1, 2];
    order.sort_by(|a, b| {
        let by_margin = margins[*b].total_cmp(&margins[*a]);
        if by_margin == std::cmp::Ordering::Equal {
            a.cmp(b)
        } else {
            by_margin
        }
    });
    [order[0], order[1]]
}

/// The certified rank margin lower bound of a support region.
fn region_margin(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
) -> Result<f64, ConstructRefusal> {
    if !region_finite(region) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let margin = map
        .rank_margin(region)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    if margin.lo <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    Ok(margin.lo)
}

/// The per-support flat-chart recovery from a certified surface map.
fn support_data(
    map: &CertifiedSurfaceMap,
    region: SurfaceRegion,
    eps: f64,
) -> Result<SupportData, ConstructRefusal> {
    if !region_finite(region) {
        return Err(ConstructRefusal::InvalidInput);
    }
    let margin = map
        .rank_margin(region)
        .map_err(|_| ConstructRefusal::InvalidInput)?;
    if margin.lo <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let boxes = map.patch_boxes();
    let grids = map.patch_grids();
    let mut first: Option<SupportData> = None;
    for (patch_box, patch_grids) in boxes.iter().zip(grids.iter()) {
        if !touches(*patch_box, region) {
            continue;
        }
        if !patch_flat(patch_grids) {
            return Err(ConstructRefusal::InvalidInput);
        }
        let (su, sv) = patch_tangents(patch_grids, *patch_box)?;
        if let Some(proto) = first.as_ref() {
            if proto.su != su || proto.sv != sv {
                return Err(ConstructRefusal::InvalidInput);
            }
        } else {
            let origin = (patch_box.0 .0, patch_box.1 .0);
            let base = patch_base(patch_grids)?;
            first = Some(SupportData {
                base,
                su,
                sv,
                normal: [0.0; 3],
                eps,
                origin,
            });
        }
    }
    let mut data = match first {
        Some(data) => data,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    data.normal = unit_normal(&data.su, &data.sv)?;
    Ok(data)
}

/// Whether a region is finite and non-degenerate on both axes.
fn region_finite(region: SurfaceRegion) -> bool {
    let ((u0, u1), (v0, v1)) = region;
    u0.is_finite() && u1.is_finite() && v0.is_finite() && v1.is_finite() && u0 <= u1 && v0 <= v1
}

/// Whether a patch box and the region share a point.
fn touches(patch: SurfaceRegion, region: SurfaceRegion) -> bool {
    axis_touches(patch.0, region.0) && axis_touches(patch.1, region.1)
}

/// Whether two closed intervals overlap (inclusive).
fn axis_touches(a: (f64, f64), b: (f64, f64)) -> bool {
    !(a.1 < b.0 || b.1 < a.0)
}

/// The CC-002 flatness gate on one Bézier patch: the three second-partial
/// coefficient grids (`S_uu`, `S_vv`, `S_uv`) are EXACTLY zero.
fn patch_flat(grids: &[Vec<Vec<f64>>; 3]) -> bool {
    for grid in grids {
        if !flat_grid(grid) {
            return false;
        }
    }
    true
}

fn flat_grid(grid: &[Vec<f64>]) -> bool {
    let duu = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 0);
    let dvv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 1), 1);
    let duv = bernstein_derivative_2d(&bernstein_derivative_2d(grid, 0), 1);
    all_zero(&duu) && all_zero(&dvv) && all_zero(&duv)
}

fn all_zero(grid: &[Vec<f64>]) -> bool {
    grid.iter().all(|row| row.iter().all(|c| *c == 0.0))
}

/// The source-unit tangent pair `(S_u, S_v)` of a flat patch: the first
/// derivative coefficient grids are constant (a flat patch is affine), and the
/// unit-parameter derivative constants scaled by the inverse patch widths are
/// the source-parameter tangents.
fn patch_tangents(
    grids: &[Vec<Vec<f64>>; 3],
    patch_box: SurfaceRegion,
) -> Result<([f64; 3], [f64; 3]), ConstructRefusal> {
    let width_u = patch_box.0 .1 - patch_box.0 .0;
    let width_v = patch_box.1 .1 - patch_box.1 .0;
    if !width_u.is_finite() || !width_v.is_finite() || width_u <= 0.0 || width_v <= 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv_u = 1.0 / width_u;
    let inv_v = 1.0 / width_v;
    let mut su = [0.0_f64; 3];
    let mut sv = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let du = bernstein_derivative_2d(grid, 0);
        let dv = bernstein_derivative_2d(grid, 1);
        su[k] = constant_of(&du, inv_u)?;
        sv[k] = constant_of(&dv, inv_v)?;
    }
    Ok((su, sv))
}

/// Read the (exact) common value of a constant coefficient grid, scaled by
/// `scale`. A non-constant grid is a surface that is not affine over the
/// patch, refused.
fn constant_of(grid: &[Vec<f64>], scale: f64) -> Result<f64, ConstructRefusal> {
    let first = match grid.first().and_then(|row| row.first()) {
        Some(value) => *value,
        None => return Err(ConstructRefusal::InvalidInput),
    };
    for row in grid {
        for value in row {
            if *value != first {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
    }
    Ok(first * scale)
}

/// The value of the flat patch at its lower-left source corner: the `(0, 0)`
/// Bernstein coefficient, which for any Bézier patch is the exact corner
/// value.
fn patch_base(grids: &[Vec<Vec<f64>>; 3]) -> Result<[f64; 3], ConstructRefusal> {
    let mut base = [0.0_f64; 3];
    for (k, grid) in grids.iter().enumerate() {
        let value = match grid.first().and_then(|row| row.first()) {
            Some(value) => *value,
            None => return Err(ConstructRefusal::InvalidInput),
        };
        base[k] = value;
    }
    Ok(base)
}

/// The oriented unit normal of the tangent pair, refusing a zero or
/// non-finite cross product.
fn unit_normal(su: &[f64; 3], sv: &[f64; 3]) -> Result<[f64; 3], ConstructRefusal> {
    let cross = cross3(su, sv);
    let norm = norm3(&cross);
    if !norm.is_finite() || norm == 0.0 {
        return Err(ConstructRefusal::InvalidInput);
    }
    let inv = 1.0 / norm;
    Ok([cross[0] * inv, cross[1] * inv, cross[2] * inv])
}

fn cross3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(a: &[f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl SupportData {
    /// The provisional support record used to initialise the per-index slots
    /// before the builder fills them (never observable outside construction).
    fn dummy() -> SupportData {
        SupportData {
            base: [0.0; 3],
            su: [0.0; 3],
            sv: [0.0; 3],
            normal: [0.0; 3],
            eps: 1.0,
            origin: (0.0, 0.0),
        }
    }
}

/// All principal minors of a symmetric 4×4 interval matrix have a
/// non-negative lower endpoint — the certificate that every matrix in the
/// interval family is PSD.
fn principal_minors_nonneg(m: &[[Interval; 4]; 4]) -> bool {
    for mask in 1u8..16 {
        let sub = submatrix(m, mask);
        if det_iv(&sub).lo < 0.0 {
            return false;
        }
    }
    true
}

/// Build the principal submatrix on the index subset encoded in `mask`.
fn submatrix(m: &[[Interval; 4]; 4], mask: u8) -> Vec<Vec<Interval>> {
    let mut indices = Vec::new();
    for i in 0..4 {
        if mask & (1 << i) != 0 {
            indices.push(i);
        }
    }
    let n = indices.len();
    let mut out = vec![vec![Interval::point(0.0); n]; n];
    for (r, &ri) in indices.iter().enumerate() {
        for (c, &ci) in indices.iter().enumerate() {
            out[r][c] = m[ri][ci];
        }
    }
    out
}

/// The interval determinant of a square matrix by cofactor expansion along
/// the first row, fixed column order, with directed rounding at every step
/// (the engine's determinant discipline at n ≤ 4).
fn det_iv(m: &[Vec<Interval>]) -> Interval {
    let n = m.len();
    if n == 1 {
        return m[0][0];
    }
    let mut acc = Interval::point(0.0);
    for (c, first_row_entry) in m[0].iter().enumerate() {
        let mut sub: Vec<Vec<Interval>> = Vec::with_capacity(n - 1);
        for row in m.iter().skip(1) {
            let mut sub_row = Vec::with_capacity(n - 1);
            for (ci, value) in row.iter().enumerate() {
                if ci != c {
                    sub_row.push(*value);
                }
            }
            sub.push(sub_row);
        }
        let sign = if c % 2 == 0 { 1.0 } else { -1.0 };
        acc = acc.add(
            &Interval::point(sign)
                .mul(first_row_entry)
                .mul(&det_iv(&sub)),
        );
    }
    acc
}

/// The centre/radius interval split of a reduced box.
fn box_ivs(b: &IBox4) -> ([Interval; 3], Interval) {
    let c = [
        Interval {
            lo: b.lo[0],
            hi: b.hi[0],
        },
        Interval {
            lo: b.lo[1],
            hi: b.hi[1],
        },
        Interval {
            lo: b.lo[2],
            hi: b.hi[2],
        },
    ];
    let r = Interval {
        lo: b.lo[3],
        hi: b.hi[3],
    };
    (c, r)
}

/// A length-4 interval box from a slice; `None` on any other length or a
/// non-finite entry.
fn box_from_slice(b: &[Interval]) -> Option<IBox4> {
    if b.len() != 4 {
        return None;
    }
    let lo = [b[0].lo, b[1].lo, b[2].lo, b[3].lo];
    let hi = [b[0].hi, b[1].hi, b[2].hi, b[3].hi];
    if lo.iter().chain(hi.iter()).all(|v| v.is_finite()) {
        IBox4::try_new(lo, hi).ok()
    } else {
        None
    }
}

/// The vacuous enclosure of the full real line (the tier2.rs convention for
/// an invalid evaluation request).
fn unbounded() -> Interval {
    Interval {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    }
}

/// Bisect a box along its widest finite axis at the midpoint, producing the
/// two closed children. An empty result means the box has no positive width
/// on any axis.
fn bisect4(b: &IBox4) -> Vec<IBox4> {
    let mut axis = 0usize;
    let mut width = b.hi[0] - b.lo[0];
    for i in 1..4 {
        let w = b.hi[i] - b.lo[i];
        if w > width {
            width = w;
            axis = i;
        }
    }
    if !(width.is_finite() && width > 0.0) {
        return Vec::new();
    }
    let mid = 0.5 * (b.lo[axis] + b.hi[axis]);
    let mut lo_child = *b;
    let mut hi_child = *b;
    lo_child.hi[axis] = mid;
    hi_child.lo[axis] = mid;
    vec![lo_child, hi_child]
}
