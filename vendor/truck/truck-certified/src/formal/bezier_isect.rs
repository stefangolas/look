// Grandfathered (orchestrator amendment, BG-CK-P0-CRATE r3): moved
// verbatim from truck-meshalgo, whose crate never denied
// clippy::unwrap_used. The crate-level deny in lib.rs is H-1's contract
// for AUTHORED certified code; this module's pre-existing unwraps are
// inherited baseline content and must not be force-rewritten by the
// move packet. Do not add new unwraps under this allow.
#![allow(clippy::unwrap_used)]

//! Certified bivariate root isolation for a pair of rational Bézier spans
//! (GEN-001C).
//!
//! For two homogeneous rational Bézier spans
//!
//! ```text
//! C1(s) = [X1(s) : Y1(s) : W1(s)]      C2(t) = [X2(t) : Y2(t) : W2(t)]
//! ```
//!
//! with certified positive weights, the denominator-cleared system is
//!
//! ```text
//! F(s,t) = X1(s) W2(t) - X2(t) W1(s)
//! G(s,t) = Y1(s) W2(t) - Y2(t) W1(s)
//! ```
//!
//! This module isolates the ordinary isolated regular roots of that system,
//! certifies a unique root in each candidate box by an interval Krawczyk
//! operator, certifies the branch germs from the tangent numerators
//! `(X'W - XW', Y'W - YW')`, certifies the transverse crossing orientation, and
//! builds a canonical pair-local event identity. Harder cases — clustered or
//! multiple roots, singular Jacobians, stationary or tangential contacts,
//! boundary roots — return typed [`GenericUnresolved`]; nothing is guessed.
//!
//! # The certification boundary
//!
//! The three states are kept distinct:
//!
//! ```text
//! box may contain a root      -> Subdivide
//! box contains no root        -> Excluded   (Bernstein range excludes zero)
//! box contains exactly one    -> Root       (Krawczyk inclusion K(X) ⊂ int X)
//! ```
//!
//! Only a valid Krawczyk inclusion may emit a root. Box width, low residual,
//! Newton convergence, recursion depth, visual separation, and an approximate
//! nonsingular midpoint Jacobian are never sufficient. If proof fails the box
//! is returned as an unresolved region and the pair result is `Unresolved`.
//!
//! # Identity
//!
//! The event identity is the canonical sorted span pair plus a deterministic
//! pair-local root ordinal, ordered by interval separation of the
//! orientation-normalized authoritative source parameter boxes. Operands are
//! canonicalized to source orientation before isolation, so reversal of either
//! or both spans and an operand swap all produce the same identity set.
//! Certified parameter boxes are evidence carried on the branch records, never
//! the identity itself.
//!
//! All tensor-Bernstein machinery here is solver-private; ARR-003-facing
//! records consume certified roots, germs, and contact components only.

use super::bezier::RationalBezierSpan2;
use super::contact::{
    BranchIncidence, ContactComponent2, CrossingClassification, EventIdentity, GenericUnresolved,
    IsolatedEvent2, IsolatedRootKey, PairContactResult,
};
use super::exact::CertifiedInterval;
use super::intersection::{ParameterEnclosure, ParameterLocation};
use super::quotient::{CanonicalBranchSide, CertifiedDeckLabel, DeckContext};
use super::span::{BranchGerm, SpanId};
use truck_geometry::prelude::Point2;

/// The maximum subdivision depth of the isolation quadtree.
const MAX_DEPTH: u32 = 50;
/// The total node budget of one isolation run.
const NODE_BUDGET: usize = 200_000;

/// A certified axis-aligned parameter box in canonical-local `[0, 1]²`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParamBox {
    pub s_lo: f64,
    pub s_hi: f64,
    pub t_lo: f64,
    pub t_hi: f64,
}

const ROOT_BOX: ParamBox = ParamBox {
    s_lo: 0.0,
    s_hi: 1.0,
    t_lo: 0.0,
    t_hi: 1.0,
};

// ---------------------------------------------------------------------------
// Canonical operands
// ---------------------------------------------------------------------------

/// A span normalized to canonical source orientation.
///
/// The authoritative source domain `[a, b]` with `a <= b` and a canonical local
/// parameter `v ∈ [0, 1]` mapping `t = a + v (b - a)`. A span whose declared
/// traversal domain runs high-to-low is reversed into this form, so a span and
/// its [`RationalBezierSpan2::reverse_occurrence`] share one canonical
/// representation. The original traversal domain and provenance are retained
/// for the orientation-dependent branch incidence records.
struct CanonicalOperand {
    /// Canonical-source-oriented `X` Bernstein coefficients over `[0, 1]`.
    x: Vec<CertifiedInterval>,
    /// Canonical-source-oriented `Y` Bernstein coefficients over `[0, 1]`.
    y: Vec<CertifiedInterval>,
    /// Canonical-source-oriented `W` Bernstein coefficients over `[0, 1]`.
    w: Vec<CertifiedInterval>,
    /// The canonical source domain `(a, b)` with `a <= b`.
    source_domain: (f64, f64),
    /// Whether the span was reversed relative to its canonical orientation.
    was_reversed: bool,
}

fn canonicalize(span: &RationalBezierSpan2) -> CanonicalOperand {
    let (d0, d1) = span.domain();
    let was_reversed = d1 < d0;
    let (a, b) = if was_reversed { (d1, d0) } else { (d0, d1) };
    let control = if was_reversed {
        let mut c = span.control.clone();
        c.reverse();
        c
    } else {
        span.control.clone()
    };
    let x = control
        .iter()
        .map(|c| CertifiedInterval::point(c.0))
        .collect();
    let y = control
        .iter()
        .map(|c| CertifiedInterval::point(c.1))
        .collect();
    let w = control
        .iter()
        .map(|c| CertifiedInterval::point(c.2))
        .collect();
    CanonicalOperand {
        x,
        y,
        w,
        source_domain: (a, b),
        was_reversed,
    }
}

/// The outward-rounded certified source-parameter intervals of a canonical-local
/// interval `[v_lo, v_hi]`, on the canonical source domain `[a, b]`.
///
/// The affine map `t = a + v (b - a)` is evaluated in directed-rounding
/// interval arithmetic (`b - a` and each product and sum are widened), so the
/// returned intervals provably contain the exact source parameters. This is
/// certified evidence wherever it is used.
fn source_parameter_intervals(
    op: &CanonicalOperand,
    v_lo: f64,
    v_hi: f64,
) -> (CertifiedInterval, CertifiedInterval) {
    let (a, b) = op.source_domain;
    let a_iv = CertifiedInterval::point(a);
    let span = CertifiedInterval::point(b).sub(&a_iv);
    (
        a_iv.add(&CertifiedInterval::point(v_lo).mul(&span)),
        a_iv.add(&CertifiedInterval::point(v_hi).mul(&span)),
    )
}

/// The certified authoritative source-parameter enclosure of a canonical-local
/// interval `[v_lo, v_hi]`, stored numerically ordered. This is evidence for the
/// branch record, not the event identity.
///
/// The occurrence's source parameter at canonical-local `v` is
/// `t = a + v (b - a)` on the canonical source domain `[a, b]`; the traversal
/// orientation only sets the direction, never the value. Computing via the
/// canonical map keeps the evidence bitwise-identical under span reversal.
fn source_enclosure(op: &CanonicalOperand, v_lo: f64, v_hi: f64) -> ParameterEnclosure {
    let (lo_iv, hi_iv) = source_parameter_intervals(op, v_lo, v_hi);
    ParameterEnclosure {
        lo: lo_iv.lo,
        hi: hi_iv.hi,
    }
}

// ---------------------------------------------------------------------------
// Univariate and bivariate Bernstein interval machinery
// ---------------------------------------------------------------------------

/// Exact binomial coefficient, `u128` accumulated for safety.
fn binomial(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut acc: u128 = 1;
    for i in 0..k {
        acc = acc * (n - i) as u128 / (i + 1) as u128;
    }
    acc
}

/// Outward-rounded enclosure of the exact rational `num / den`.
///
/// `num` and the division each round to within half an ulp; widening by two
/// ulps in each direction is a conservative rounding bound. For the Bernstein
/// product weights here `num <= den`, so the value lies in `(0, 1]` and stays
/// finite.
fn rational_interval(num: u128, den: u128) -> CertifiedInterval {
    if num == 0 {
        return CertifiedInterval::point(0.0);
    }
    let v = (num as f64) / (den as f64);
    if !v.is_finite() {
        return CertifiedInterval {
            lo: 0.0,
            hi: f64::MAX,
        };
    }
    CertifiedInterval {
        lo: v.next_down().next_down(),
        hi: v.next_up().next_up(),
    }
}

/// Product of two Bernstein polynomials in the Bernstein basis.
///
/// The coefficient of degree `k` in the product is the sum over `i + j = k` of
/// `C(m, i) C(n, j) / C(m + n, k) * a_i * b_j` — the naive coefficient
/// convolution is *not* valid in the Bernstein basis. The binomial weights are
/// exact rationals, outward-rounded; all coefficient arithmetic is interval
/// arithmetic, so the result soundly encloses the exact product over the
/// interval inputs.
fn bernstein_product(a: &[CertifiedInterval], b: &[CertifiedInterval]) -> Vec<CertifiedInterval> {
    let m = a.len() - 1;
    let n = b.len() - 1;
    let d = m + n;
    let mut c = vec![CertifiedInterval::point(0.0); d + 1];
    for i in 0..=m {
        for j in 0..=n {
            let num = binomial(m, i) * binomial(n, j);
            let den = binomial(d, i + j);
            let weight = rational_interval(num, den);
            let term = weight.mul(&a[i].mul(&b[j]));
            c[i + j] = c[i + j].add(&term);
        }
    }
    c
}

/// The range enclosure of a univariate Bernstein polynomial over a parameter
/// interval, by interval de Casteljau. Sound: each convex combination over
/// `u ⊆ [0, 1]` contains the polynomial's value for every `u` in the interval.
fn one_d_range(c: &[CertifiedInterval], u: CertifiedInterval) -> CertifiedInterval {
    debug_assert!(u.lo >= 0.0 && u.hi <= 1.0 && u.lo <= u.hi);
    let u_comp = CertifiedInterval::point(1.0).sub(&u);
    let mut pts = c.to_vec();
    while pts.len() > 1 {
        let mut next = Vec::with_capacity(pts.len() - 1);
        for w in pts.windows(2) {
            next.push(u_comp.mul(&w[0]).add(&u.mul(&w[1])));
        }
        pts = next;
    }
    pts[0]
}

/// The range enclosure of a bivariate tensor-Bernstein polynomial over a box.
///
/// `c[i][j]` holds the coefficient of `B^i_m(s) B^j_n(t)`.
fn bivariate_range(
    c: &[Vec<CertifiedInterval>],
    s_box: CertifiedInterval,
    t_box: CertifiedInterval,
) -> CertifiedInterval {
    let n = c[0].len() - 1;
    let mut col_evals = Vec::with_capacity(n + 1);
    for j in 0..=n {
        let col: Vec<CertifiedInterval> = (0..c.len()).map(|i| c[i][j]).collect();
        col_evals.push(one_d_range(&col, s_box));
    }
    one_d_range(&col_evals, t_box)
}

/// The point enclosure of a bivariate tensor-Bernstein polynomial.
fn bivariate_point(c: &[Vec<CertifiedInterval>], s: f64, t: f64) -> CertifiedInterval {
    bivariate_range(c, CertifiedInterval::point(s), CertifiedInterval::point(t))
}

/// The derivative of a tensor-Bernstein polynomial along one axis.
///
/// The degree-`d` coefficients `c[i]` have derivative coefficients
/// `d * (c[i + 1] - c[i])` of degree `d - 1`. `axis == 0` differentiates in
/// `s`, `axis == 1` in `t`.
fn tensor_derivative_axis(
    c: &[Vec<CertifiedInterval>],
    axis: usize,
) -> Vec<Vec<CertifiedInterval>> {
    let m = c.len() - 1;
    let n = c[0].len() - 1;
    if axis == 0 {
        if m == 0 {
            return vec![vec![CertifiedInterval::point(0.0); n + 1]; 1];
        }
        let scale = CertifiedInterval::point(m as f64);
        let mut out = Vec::with_capacity(m);
        for i in 0..m {
            let mut row = Vec::with_capacity(n + 1);
            for j in 0..=n {
                row.push(scale.mul(&c[i + 1][j].sub(&c[i][j])));
            }
            out.push(row);
        }
        out
    } else {
        if n == 0 {
            return vec![vec![CertifiedInterval::point(0.0); 1]; m + 1];
        }
        let scale = CertifiedInterval::point(n as f64);
        let mut out = Vec::with_capacity(m + 1);
        for i in 0..=m {
            let mut row = Vec::with_capacity(n);
            for j in 0..n {
                row.push(scale.mul(&c[i][j + 1].sub(&c[i][j])));
            }
            out.push(row);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// The denominator-cleared system
// ---------------------------------------------------------------------------

struct System {
    f: Vec<Vec<CertifiedInterval>>,
    g: Vec<Vec<CertifiedInterval>>,
    fs: Vec<Vec<CertifiedInterval>>,
    ft: Vec<Vec<CertifiedInterval>>,
    gs: Vec<Vec<CertifiedInterval>>,
    gt: Vec<Vec<CertifiedInterval>>,
}

impl System {
    fn new(op1: &CanonicalOperand, op2: &CanonicalOperand) -> System {
        let m = op1.x.len() - 1;
        let n = op2.x.len() - 1;
        let mut f = vec![vec![CertifiedInterval::point(0.0); n + 1]; m + 1];
        let mut g = vec![vec![CertifiedInterval::point(0.0); n + 1]; m + 1];
        for i in 0..=m {
            for j in 0..=n {
                f[i][j] = op1.x[i].mul(&op2.w[j]).sub(&op1.w[i].mul(&op2.x[j]));
                g[i][j] = op1.y[i].mul(&op2.w[j]).sub(&op1.w[i].mul(&op2.y[j]));
            }
        }
        let fs = tensor_derivative_axis(&f, 0);
        let ft = tensor_derivative_axis(&f, 1);
        let gs = tensor_derivative_axis(&g, 0);
        let gt = tensor_derivative_axis(&g, 1);
        System {
            f,
            g,
            fs,
            ft,
            gs,
            gt,
        }
    }

    fn range_excludes_zero(&self, b: &ParamBox) -> bool {
        let s_box = CertifiedInterval {
            lo: b.s_lo,
            hi: b.s_hi,
        };
        let t_box = CertifiedInterval {
            lo: b.t_lo,
            hi: b.t_hi,
        };
        range_excludes_zero(&bivariate_range(&self.f, s_box, t_box))
            || range_excludes_zero(&bivariate_range(&self.g, s_box, t_box))
    }
}

/// Whether a range enclosure is certifiably disjoint from zero.
fn range_excludes_zero(r: &CertifiedInterval) -> bool {
    r.lo > 0.0 || r.hi < 0.0
}

// ---------------------------------------------------------------------------
// The Krawczyk unique-root certificate
// ---------------------------------------------------------------------------

/// A verified Krawczyk certificate: `K(X) ⊂ int(X)` over directed rounding,
/// which proves the box contains exactly one root of the system.
///
/// The constructor verifies the inclusion; callers never manufacture one
/// directly. The fields beyond `image`/`domain` are retained validation
/// evidence (the operator inputs and enclosure) per the certificate contract;
/// they are read by tests and by ARR-003's diagnostics.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // evidence-retention fields are read by tests and ARR-003
pub(crate) struct KrawczykCertificate {
    /// The box the certificate applies to (canonical-local `[0, 1]²`).
    pub domain: ParamBox,
    /// The box midpoint.
    pub center: [f64; 2],
    /// The point preconditioner, an approximate inverse of the real Jacobian
    /// at the center (identity when the midpoint Jacobian is singular).
    pub preconditioner: [[f64; 2]; 2],
    /// `H(x0)`, the system evaluated at the center.
    pub function_at_center: [CertifiedInterval; 2],
    /// `J(X)`, the interval Jacobian enclosure over the whole box.
    pub jacobian_enclosure: [[CertifiedInterval; 2]; 2],
    /// `K(X)`, the certified root box (canonical-local), strictly inside
    /// `domain`.
    pub image: ParamBox,
}

/// Attempt a Krawczyk unique-root certificate on the box.
fn try_krawczyk(sys: &System, b: &ParamBox) -> Option<KrawczykCertificate> {
    let s_box = CertifiedInterval {
        lo: b.s_lo,
        hi: b.s_hi,
    };
    let t_box = CertifiedInterval {
        lo: b.t_lo,
        hi: b.t_hi,
    };
    let s0 = (b.s_lo + b.s_hi) / 2.0;
    let t0 = (b.t_lo + b.t_hi) / 2.0;

    let f0 = bivariate_point(&sys.f, s0, t0);
    let g0 = bivariate_point(&sys.g, s0, t0);

    let fs = bivariate_range(&sys.fs, s_box, t_box);
    let ft = bivariate_range(&sys.ft, s_box, t_box);
    let gs = bivariate_range(&sys.gs, s_box, t_box);
    let gt = bivariate_range(&sys.gt, s_box, t_box);
    let j = [[fs, ft], [gs, gt]];

    let c = preconditioner(sys, s0, t0);

    let h = [f0, g0];
    let x0 = [CertifiedInterval::point(s0), CertifiedInterval::point(t0)];
    // `(X - x0)` with directed rounding. The box endpoints and the midpoint
    // are f64s; their differences are outward-rounded and the two endpoint
    // differences are hulled, so the result soundly encloses `{x - x0 : x in X}`
    // for any box — including non-dyadic cluster hulls, not only dyadic
    // quadtree leaves.
    let dx = [
        dx_range(b.s_lo, b.s_hi, &x0[0]),
        dx_range(b.t_lo, b.t_hi, &x0[1]),
    ];

    let ch = matvec(&c, &h);
    let cj = matmul(&c, &j);
    let id_minus_cj = [
        [CertifiedInterval::point(1.0).sub(&cj[0][0]), cj[0][1].neg()],
        [cj[1][0].neg(), CertifiedInterval::point(1.0).sub(&cj[1][1])],
    ];
    let md = matvec_iv(&id_minus_cj, &dx);
    let k = [x0[0].sub(&ch[0]).add(&md[0]), x0[1].sub(&ch[1]).add(&md[1])];

    let inside = k[0].is_finite()
        && k[1].is_finite()
        && k[0].lo > b.s_lo
        && k[0].hi < b.s_hi
        && k[1].lo > b.t_lo
        && k[1].hi < b.t_hi;
    if inside {
        Some(KrawczykCertificate {
            domain: *b,
            center: [s0, t0],
            preconditioner: c,
            function_at_center: h,
            jacobian_enclosure: j,
            image: ParamBox {
                s_lo: k[0].lo,
                s_hi: k[0].hi,
                t_lo: k[1].lo,
                t_hi: k[1].hi,
            },
        })
    } else {
        None
    }
}

/// The outward-rounded enclosure of `[lo, hi] - center`, i.e. the range of
/// `x - center` for `x in [lo, hi]`.
fn dx_range(lo: f64, hi: f64, center: &CertifiedInterval) -> CertifiedInterval {
    let d_lo = CertifiedInterval::point(lo).sub(center);
    let d_hi = CertifiedInterval::point(hi).sub(center);
    CertifiedInterval {
        lo: d_lo.lo.min(d_hi.lo),
        hi: d_lo.hi.max(d_hi.hi),
    }
}

fn matvec(c: &[[f64; 2]; 2], v: &[CertifiedInterval; 2]) -> [CertifiedInterval; 2] {
    let c00 = CertifiedInterval::point(c[0][0]);
    let c01 = CertifiedInterval::point(c[0][1]);
    let c10 = CertifiedInterval::point(c[1][0]);
    let c11 = CertifiedInterval::point(c[1][1]);
    [
        c00.mul(&v[0]).add(&c01.mul(&v[1])),
        c10.mul(&v[0]).add(&c11.mul(&v[1])),
    ]
}

fn matvec_iv(
    c: &[[CertifiedInterval; 2]; 2],
    v: &[CertifiedInterval; 2],
) -> [CertifiedInterval; 2] {
    [
        c[0][0].mul(&v[0]).add(&c[0][1].mul(&v[1])),
        c[1][0].mul(&v[0]).add(&c[1][1].mul(&v[1])),
    ]
}

fn matmul(c: &[[f64; 2]; 2], j: &[[CertifiedInterval; 2]; 2]) -> [[CertifiedInterval; 2]; 2] {
    let c00 = CertifiedInterval::point(c[0][0]);
    let c01 = CertifiedInterval::point(c[0][1]);
    let c10 = CertifiedInterval::point(c[1][0]);
    let c11 = CertifiedInterval::point(c[1][1]);
    [
        [
            c00.mul(&j[0][0]).add(&c01.mul(&j[1][0])),
            c00.mul(&j[0][1]).add(&c01.mul(&j[1][1])),
        ],
        [
            c10.mul(&j[0][0]).add(&c11.mul(&j[1][0])),
            c10.mul(&j[0][1]).add(&c11.mul(&j[1][1])),
        ],
    ]
}

/// The real Jacobian at a point, evaluated on coefficient midpoints, inverted
/// to form the preconditioner. The identity matrix when the midpoint Jacobian
/// is singular or non-finite; Krawczyk remains sound with the identity, only
/// less efficient.
fn preconditioner(sys: &System, s0: f64, t0: f64) -> [[f64; 2]; 2] {
    let fs = eval_tensor_f64(&sys.fs, s0, t0);
    let ft = eval_tensor_f64(&sys.ft, s0, t0);
    let gs = eval_tensor_f64(&sys.gs, s0, t0);
    let gt = eval_tensor_f64(&sys.gt, s0, t0);
    let det = fs * gt - ft * gs;
    if det.is_finite() && det != 0.0 {
        let inv = 1.0 / det;
        if inv.is_finite() {
            return [[gt * inv, -ft * inv], [-gs * inv, fs * inv]];
        }
    }
    [[1.0, 0.0], [0.0, 1.0]]
}

fn eval_tensor_f64(c: &[Vec<CertifiedInterval>], s: f64, t: f64) -> f64 {
    let n = c[0].len() - 1;
    let mut col = Vec::with_capacity(n + 1);
    for j in 0..=n {
        let mut seq: Vec<f64> = (0..c.len()).map(|i| midpoint(&c[i][j])).collect();
        col.push(one_d_f64(&mut seq, s));
    }
    one_d_f64(&mut col, t)
}

fn midpoint(iv: &CertifiedInterval) -> f64 {
    (iv.lo + iv.hi) / 2.0
}

fn one_d_f64(pts: &mut Vec<f64>, u: f64) -> f64 {
    while pts.len() > 1 {
        let mut next = Vec::with_capacity(pts.len() - 1);
        for w in pts.windows(2) {
            next.push((1.0 - u) * w[0] + u * w[1]);
        }
        *pts = next;
    }
    pts[0]
}

// ---------------------------------------------------------------------------
// Isolation
// ---------------------------------------------------------------------------

enum NodeClass {
    Excluded,
    Root(KrawczykCertificate),
    Split,
}

fn classify(sys: &System, b: &ParamBox) -> NodeClass {
    if sys.range_excludes_zero(b) {
        return NodeClass::Excluded;
    }
    if let Some(cert) = try_krawczyk(sys, b) {
        return NodeClass::Root(cert);
    }
    NodeClass::Split
}

/// A certified isolated root of the pair: the grid cell it was certified in,
/// the tighter Krawczyk image, and the certificate itself.
#[derive(Debug, Clone, PartialEq)]
struct RootRecord {
    leaf: ParamBox,
    image: ParamBox,
    certificate: KrawczykCertificate,
    ordinal: u32,
}

/// Deterministic depth-limited branch-and-bound isolation over `[0, 1]²`.
///
/// Returns the certified root records and the pending leaf boxes that could
/// not be resolved within the budget. The traversal order is a fixed function
/// of the input: children are pushed in reverse so they pop in a fixed order.
fn isolate(sys: &System) -> (Vec<RootRecord>, Vec<ParamBox>) {
    let mut stack: Vec<(ParamBox, u32)> = vec![(ROOT_BOX, 0)];
    let mut certified = Vec::new();
    let mut pending = Vec::new();
    let mut nodes = 0usize;
    while let Some((b, depth)) = stack.pop() {
        nodes += 1;
        if nodes > NODE_BUDGET {
            pending.push(b);
            continue;
        }
        match classify(sys, &b) {
            NodeClass::Excluded => {}
            NodeClass::Root(cert) => certified.push(RootRecord {
                leaf: b,
                image: cert.image,
                certificate: cert,
                ordinal: 0,
            }),
            NodeClass::Split => {
                if depth >= MAX_DEPTH || b.s_hi - b.s_lo <= 0.0 || b.t_hi - b.t_lo <= 0.0 {
                    pending.push(b);
                } else {
                    let sm = (b.s_lo + b.s_hi) / 2.0;
                    let tm = (b.t_lo + b.t_hi) / 2.0;
                    let children = [
                        ParamBox {
                            s_lo: sm,
                            s_hi: b.s_hi,
                            t_lo: tm,
                            t_hi: b.t_hi,
                        },
                        ParamBox {
                            s_lo: b.s_lo,
                            s_hi: sm,
                            t_lo: tm,
                            t_hi: b.t_hi,
                        },
                        ParamBox {
                            s_lo: sm,
                            s_hi: b.s_hi,
                            t_lo: b.t_lo,
                            t_hi: tm,
                        },
                        ParamBox {
                            s_lo: b.s_lo,
                            s_hi: sm,
                            t_lo: b.t_lo,
                            t_hi: tm,
                        },
                    ];
                    for child in children {
                        stack.push((child, depth + 1));
                    }
                }
            }
        }
    }
    (certified, pending)
}

/// Two boxes touch when their intervals overlap-or-touch on both axes.
fn boxes_touch(a: &ParamBox, b: &ParamBox) -> bool {
    a.s_lo <= b.s_hi && b.s_lo <= a.s_hi && a.t_lo <= b.t_hi && b.t_lo <= a.t_hi
}

/// Merge adjacent pending leaves into connected unresolved-region hulls.
///
/// No pending component is ever promoted to a certified root. A leaf that
/// could not be certified within the budget — including a root lying exactly
/// on a subdivision boundary, for which no grid cell ever contains it strictly
/// — is reported as an unresolved region and the pair result becomes
/// `Unresolved`. Promoting a component hull to a root would risk re-certifying
/// a root already certified in a covered cell, or merging two distinct roots
/// inside one hull, both of which would emit duplicate or fused events. The
/// quadtree partition guarantees component hulls are disjoint from every
/// certified cell and from one another, so these hulls are exactly the
/// complement of the certified-and-excluded coverage.
fn group_pending(pending: &[ParamBox]) -> Vec<ParamBox> {
    let n = pending.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        let mut root = x;
        while parent[root] != root {
            root = parent[root];
        }
        let mut cur = x;
        while parent[cur] != cur {
            let next = parent[cur];
            parent[cur] = root;
            cur = next;
        }
        root
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if boxes_touch(&pending[i], &pending[j]) {
                let ri = find(&mut parent, i);
                let rj = find(&mut parent, j);
                if ri != rj {
                    parent[rj] = ri;
                }
            }
        }
    }
    let mut groups: std::collections::HashMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    groups
        .values()
        .map(|members| {
            let mut hull = ParamBox {
                s_lo: f64::INFINITY,
                s_hi: f64::NEG_INFINITY,
                t_lo: f64::INFINITY,
                t_hi: f64::NEG_INFINITY,
            };
            for &i in members {
                let b = &pending[i];
                hull.s_lo = hull.s_lo.min(b.s_lo);
                hull.s_hi = hull.s_hi.max(b.s_hi);
                hull.t_lo = hull.t_lo.min(b.t_lo);
                hull.t_hi = hull.t_hi.max(b.t_hi);
            }
            hull
        })
        .collect()
}

/// Classify an unresolved region. This is a diagnostic refinement; either way
/// the region is typed `Unresolved` and never produces topology.
///
/// A region touching the parameter-domain boundary is the signature of a
/// Bézier root lying exactly on the domain boundary (GEN-001C endpoint policy:
/// no endpoint certificate is implemented). An interior region is a cluster
/// the isolation could not separate or certify.
fn classify_region(hull: &ParamBox) -> GenericUnresolved {
    if hull.s_lo <= 0.0 || hull.s_hi >= 1.0 || hull.t_lo <= 0.0 || hull.t_hi >= 1.0 {
        GenericUnresolved::UnresolvedBoundaryRoot
    } else {
        GenericUnresolved::ClusteredRoots
    }
}

/// Order the certified roots canonically and assign the pair-local ordinals.
///
/// The operands have already been canonicalized into sorted order, so the
/// canonically-smaller span occupies the `s` axis and canonical-local `s`
/// order agrees with authoritative source order on that span. Roots are
/// ordered **only** by their certified canonical-local `s` isolators. For every
/// distinct pair, the isolators must be disjoint and ordered:
///
/// ```text
/// a.s_hi < b.s_lo  =>  a before b
/// b.s_hi < a.s_lo  =>  b before a
/// otherwise         =>  overlap
/// ```
///
/// Overlap of the `s` isolators does **not** certify equality of the `s`
/// parameters — it only means the order is currently unknown — so there is no
/// fallback to the `t` coordinate. The two roots' isolators are first refined
/// (bounded Krawczyk iteration) to tighten them; if a pair still overlaps after
/// the refinement budget, the pair is typed
/// [`GenericUnresolved::UnresolvedIdentityOrdering`]: two distinct parameter
/// roots that cannot be separated on the canonical first span may represent a
/// self-intersection, a repeated geometric event, or a multi-branch situation
/// that requires later event aggregation, and must not be independently
/// numbered as ordinary events. Because separation is decided on a single
/// fixed scalar coordinate, a separated relation is automatically a total
/// order and the same identity is stable under later refinement.
///
/// The operation is **transactional**: refinement, separation checks, sorting,
/// and ordinal assignment happen on a working copy, and the input records are
/// replaced only on success. A failure leaves the caller's records and their
/// certificates untouched. Sorting by `s_lo` here only realizes an order
/// already certified by the pairwise disjointness checks; it is not the proof
/// of that order.
fn assign_ordinals(roots: &mut [RootRecord], sys: &System) -> Result<(), GenericUnresolved> {
    let mut candidate = roots.to_vec();
    for root in &mut candidate {
        refine_record(sys, root);
    }
    for i in 0..candidate.len() {
        for j in (i + 1)..candidate.len() {
            certified_primary_cmp(&candidate[i], &candidate[j])?;
        }
    }
    candidate.sort_by(|a, b| a.image.s_lo.total_cmp(&b.image.s_lo));
    for pair in candidate.windows(2) {
        if certified_primary_cmp(&pair[0], &pair[1])? != std::cmp::Ordering::Less {
            return Err(GenericUnresolved::UnresolvedIdentityOrdering);
        }
    }
    for (ordinal, root) in candidate.iter_mut().enumerate() {
        root.ordinal = ordinal as u32;
    }
    roots.clone_from_slice(&candidate);
    Ok(())
}

/// The certified order of two roots by their canonical-local `s` isolators.
///
/// Reports `Less`/`Greater` only when the isolators are disjoint; any overlap
/// (which certifies neither equality nor order) fails with
/// [`GenericUnresolved::UnresolvedIdentityOrdering`]. Distinct roots are never
/// reported `Equal`.
fn certified_primary_cmp(
    a: &RootRecord,
    b: &RootRecord,
) -> Result<std::cmp::Ordering, GenericUnresolved> {
    if a.image.s_hi < b.image.s_lo {
        Ok(std::cmp::Ordering::Less)
    } else if b.image.s_hi < a.image.s_lo {
        Ok(std::cmp::Ordering::Greater)
    } else {
        Err(GenericUnresolved::UnresolvedIdentityOrdering)
    }
}

/// Tighten a certified root box by bounded Krawczyk iteration, keeping the
/// stored certificate in lockstep with the stored isolator.
///
/// Each successful certificate gives a box `K(X) ⊂ int(X)` still containing the
/// root, and it is installed together with the tightened isolator, so the
/// invariant "the stored certificate backs the stored isolator" is preserved at
/// every step. Iteration continues while any boundary contracts: contraction in
/// `t` can tighten the next Jacobian enclosure and permit a later `s`
/// contraction even when an `s` boundary is pinned at the working precision.
/// Refining before the ordinal separation check turns an isolator overlap that
/// a wider box merely *could not decide* into a certified order, and leaves
/// only genuine unresolved overlaps (identical or inseparable `s` parameters)
/// to fail the ordering.
fn refine_record(sys: &System, record: &mut RootRecord) {
    for _ in 0..REFINE_BUDGET {
        let current = record.image;
        let Some(cert) = try_krawczyk(sys, &current) else {
            break;
        };
        let next = cert.image;
        // A successful certificate proves next ⊂ int(current); defensively
        // refuse to trust a next box that is not contained.
        let contained = next.s_lo >= current.s_lo
            && next.s_hi <= current.s_hi
            && next.t_lo >= current.t_lo
            && next.t_hi <= current.t_hi;
        if !contained {
            break;
        }
        let made_progress = next.s_lo > current.s_lo
            || next.s_hi < current.s_hi
            || next.t_lo > current.t_lo
            || next.t_hi < current.t_hi;
        if !made_progress {
            break;
        }
        record.image = next;
        record.certificate = cert;
    }
}

/// The bounded refinement budget for ordinal separation.
const REFINE_BUDGET: usize = 10;

// ---------------------------------------------------------------------------
// Germs and transverse orientation
// ---------------------------------------------------------------------------

/// The derivative-numerator polynomials `(X'W - XW', Y'W - YW')` of a
/// canonical-local span, as univariate Bernstein interval polynomials over
/// `[0, 1]`. The numerator has the same direction and zero set as the true
/// derivative; `W > 0` is certified at construction, so no division appears.
fn numerator_polys(op: &CanonicalOperand) -> (Vec<CertifiedInterval>, Vec<CertifiedInterval>) {
    let m = op.x.len() - 1;
    if m == 0 {
        let zero = vec![CertifiedInterval::point(0.0)];
        return (zero.clone(), zero);
    }
    let scale = CertifiedInterval::point(m as f64);
    let xp: Vec<CertifiedInterval> = (0..m)
        .map(|i| scale.mul(&op.x[i + 1].sub(&op.x[i])))
        .collect();
    let yp: Vec<CertifiedInterval> = (0..m)
        .map(|i| scale.mul(&op.y[i + 1].sub(&op.y[i])))
        .collect();
    let wp: Vec<CertifiedInterval> = (0..m)
        .map(|i| scale.mul(&op.w[i + 1].sub(&op.w[i])))
        .collect();
    let xpw = bernstein_product(&xp, &op.w);
    let xwp = bernstein_product(&op.x, &wp);
    let ypw = bernstein_product(&yp, &op.w);
    let ywp = bernstein_product(&op.y, &wp);
    let nx: Vec<CertifiedInterval> = (0..xpw.len()).map(|i| xpw[i].sub(&xwp[i])).collect();
    let ny: Vec<CertifiedInterval> = (0..ypw.len()).map(|i| ypw[i].sub(&ywp[i])).collect();
    (nx, ny)
}

/// Certify a regular branch: the tangent-numerator range over the certified
/// parameter interval excludes the zero vector (at least one component
/// excludes zero). Containing zero is uncertainty, never a stationary proof;
/// the interval is subdivided into finer pieces to tighten the enclosure, and
/// the branch is only certified once the range excludes zero.
fn tangent_nonzero(
    (nx, ny): &(Vec<CertifiedInterval>, Vec<CertifiedInterval>),
    lo: f64,
    hi: f64,
) -> bool {
    for k in [1usize, 2, 4, 8] {
        let mut tx = CertifiedInterval {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        };
        let mut ty = tx;
        for i in 0..k {
            let a = lo + (i as f64 / k as f64) * (hi - lo);
            let b = lo + ((i + 1) as f64 / k as f64) * (hi - lo);
            let piece = CertifiedInterval { lo: a, hi: b };
            let xi = one_d_range(nx, piece);
            let yi = one_d_range(ny, piece);
            tx.lo = tx.lo.min(xi.lo);
            tx.hi = tx.hi.max(xi.hi);
            ty.lo = ty.lo.min(yi.lo);
            ty.hi = ty.hi.max(yi.hi);
        }
        if range_excludes_zero(&tx) || range_excludes_zero(&ty) {
            return true;
        }
    }
    false
}

/// The certified transverse determinant `det(T1, T2)` hull over the root box,
/// refined by subdividing the box into finer grids. `0 ∈ Δ` after refinement
/// means the contact cannot be certified as a regular crossing.
fn delta_hull(
    (nx1, ny1): &(Vec<CertifiedInterval>, Vec<CertifiedInterval>),
    (nx2, ny2): &(Vec<CertifiedInterval>, Vec<CertifiedInterval>),
    s_lo: f64,
    s_hi: f64,
    t_lo: f64,
    t_hi: f64,
) -> CertifiedInterval {
    let mut hull = CertifiedInterval {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    };
    for k in [1usize, 2, 4] {
        for i in 0..k {
            let sa = s_lo + (i as f64 / k as f64) * (s_hi - s_lo);
            let sb = s_lo + ((i + 1) as f64 / k as f64) * (s_hi - s_lo);
            let s_piece = CertifiedInterval { lo: sa, hi: sb };
            let t1x = one_d_range(nx1, s_piece);
            let t1y = one_d_range(ny1, s_piece);
            for j in 0..k {
                let ta = t_lo + (j as f64 / k as f64) * (t_hi - t_lo);
                let tb = t_lo + ((j + 1) as f64 / k as f64) * (t_hi - t_lo);
                let t_piece = CertifiedInterval { lo: ta, hi: tb };
                let t2x = one_d_range(nx2, t_piece);
                let t2y = one_d_range(ny2, t_piece);
                let d = t1x.mul(&t2y).sub(&t1y.mul(&t2x));
                hull.lo = hull.lo.min(d.lo);
                hull.hi = hull.hi.max(d.hi);
            }
        }
        if hull.lo > 0.0 || hull.hi < 0.0 {
            break;
        }
    }
    hull
}

/// The signed crossing orientation of a transverse contact, defined against the
/// canonically sorted operand order (see [`solve_pair`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrossingOrientation {
    Positive,
    Negative,
}

impl CrossingOrientation {
    fn from_delta(delta: &CertifiedInterval) -> Result<CrossingOrientation, GenericUnresolved> {
        if delta.lo > 0.0 {
            Ok(CrossingOrientation::Positive)
        } else if delta.hi < 0.0 {
            Ok(CrossingOrientation::Negative)
        } else {
            Err(GenericUnresolved::UnresolvedTangencyOrSingularity)
        }
    }
}

/// A certified isolated root ready for the arrangement-facing records.
///
/// The two participants are carried in canonical (sorted) span order; the
/// `first_*` fields belong to the canonically-smaller span and the `second_*`
/// fields to the canonically-larger span. The canonical source-parameter
/// enclosures are the orientation-normalized certified data the refer-back
/// plumbing and ARR-003's registry use to match a root found in a derived
/// (subdivided) sub-pair back to the immutable root id of the unsplit pair.
#[derive(Debug, Clone)]
#[allow(dead_code)] // certified evidence fields are read by tests and ARR-003
pub(crate) struct CertifiedIsolatedRoot2 {
    /// The grid cell where the root was certified.
    pub leaf: ParamBox,
    /// The tighter certified root box (Krawczyk image).
    pub image: ParamBox,
    /// The immutable root id of this canonical (unsplit) pair isolation.
    pub ordinal: u32,
    /// The canonically-smaller participating span.
    pub span_first_id: SpanId,
    /// The canonically-larger participating span.
    pub span_second_id: SpanId,
    /// Certified source-parameter enclosure of the root on the first span's
    /// canonical source domain (orientation-normalized).
    pub first_parameter: ParameterEnclosure,
    /// Certified source-parameter enclosure of the root on the second span's
    /// canonical source domain (orientation-normalized).
    pub second_parameter: ParameterEnclosure,
    /// The authoritative source-parameter evidence on the first span's
    /// original traversal domain.
    pub first_evidence: ParameterEnclosure,
    /// The authoritative source-parameter evidence on the second span's
    /// original traversal domain.
    pub second_evidence: ParameterEnclosure,
    /// The verified Krawczyk certificate.
    pub certificate: KrawczykCertificate,
    /// The certified crossing orientation (canonical operand order).
    pub crossing: CrossingOrientation,
    /// A representative point (evaluation hint, never identity).
    pub representative: Point2,
}

/// The pair-level isolation result. The arrangement layer may consume only
/// `Complete`; the certified subset of `Unresolved` is diagnostic because an
/// unresolved region may contain another root.
#[derive(Debug, Clone)]
pub(crate) enum PairSolveResult {
    /// The whole domain is covered by excluded regions and mutually distinct
    /// unique-root clusters; no root was missed.
    Complete(Vec<CertifiedIsolatedRoot2>),
    /// At least one region could not be certified. `certified` is diagnostic
    /// only and must not be consumed as a complete intersection relation.
    #[allow(dead_code)] // diagnostic subset and regions are read by tests/ARR-003
    Unresolved {
        certified: Vec<CertifiedIsolatedRoot2>,
        reason: GenericUnresolved,
        regions: Vec<ParamBox>,
    },
}

/// Certify the germ, transverse orientation, and evidence of one certified
/// root.
///
/// `op1`/`op2` are the canonical operands in sorted span order (`op1` on the
/// `s` axis), and `first`/`second` are the corresponding original spans. The
/// crossing orientation is therefore already canonical — no operand-order
/// flip is needed.
fn certify_event(
    op1: &CanonicalOperand,
    op2: &CanonicalOperand,
    first: &RationalBezierSpan2,
    second: &RationalBezierSpan2,
    record: &RootRecord,
) -> Result<CertifiedIsolatedRoot2, GenericUnresolved> {
    let s_lo = record.image.s_lo;
    let s_hi = record.image.s_hi;
    let t_lo = record.image.t_lo;
    let t_hi = record.image.t_hi;

    let n1 = numerator_polys(op1);
    let n2 = numerator_polys(op2);
    if !tangent_nonzero(&n1, s_lo, s_hi) {
        return Err(GenericUnresolved::UnresolvedStationaryBranch);
    }
    if !tangent_nonzero(&n2, t_lo, t_hi) {
        return Err(GenericUnresolved::UnresolvedStationaryBranch);
    }

    let delta = delta_hull(&n1, &n2, s_lo, s_hi, t_lo, t_hi);
    let crossing = CrossingOrientation::from_delta(&delta)?;

    let (first_lo, first_hi) = source_parameter_intervals(op1, s_lo, s_hi);
    let (second_lo, second_hi) = source_parameter_intervals(op2, t_lo, t_hi);
    let first_parameter = ParameterEnclosure {
        lo: first_lo.lo,
        hi: first_hi.hi,
    };
    let second_parameter = ParameterEnclosure {
        lo: second_lo.lo,
        hi: second_hi.hi,
    };

    let v_c = (s_lo + s_hi) / 2.0;
    let u_c = if op1.was_reversed { 1.0 - v_c } else { v_c };
    let representative = first
        .evaluate_enclosure(u_c)
        .map(|[px, py]| Point2::new(midpoint(&px), midpoint(&py)))
        .unwrap_or_else(|| Point2::new(0.0, 0.0));

    Ok(CertifiedIsolatedRoot2 {
        leaf: record.leaf,
        image: record.image,
        ordinal: record.ordinal,
        span_first_id: SpanId::from_occurrence(&first.provenance),
        span_second_id: SpanId::from_occurrence(&second.provenance),
        first_parameter,
        second_parameter,
        first_evidence: source_enclosure(op1, s_lo, s_hi),
        second_evidence: source_enclosure(op2, t_lo, t_hi),
        certificate: record.certificate.clone(),
        crossing,
        representative,
    })
}

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

fn solve_pair(lhs: &RationalBezierSpan2, rhs: &RationalBezierSpan2) -> PairSolveResult {
    if !lhs.w_positive_on_unit() || !rhs.w_positive_on_unit() {
        return PairSolveResult::Unresolved {
            certified: Vec::new(),
            reason: GenericUnresolved::UnsupportedSingularBranch,
            regions: vec![ROOT_BOX],
        };
    }
    // Canonicalize operand order so the canonical system is a pure function of
    // the unordered span pair: the canonically-smaller span always occupies the
    // `s` axis. Operand swap and span reversal then produce the same canonical
    // system and the same certified boxes, ordinals, and identities.
    let lhs_id = SpanId::from_occurrence(&lhs.provenance);
    let rhs_id = SpanId::from_occurrence(&rhs.provenance);
    let (first, second, op1, op2) = if lhs_id <= rhs_id {
        (lhs, rhs, canonicalize(lhs), canonicalize(rhs))
    } else {
        (rhs, lhs, canonicalize(rhs), canonicalize(lhs))
    };
    let sys = System::new(&op1, &op2);

    let (mut records, pending) = isolate(&sys);
    let mut regions: Vec<(ParamBox, GenericUnresolved)> = Vec::new();
    for hull in group_pending(&pending) {
        regions.push((hull, classify_region(&hull)));
    }

    if let Err(reason) = assign_ordinals(&mut records, &sys) {
        regions.push((ROOT_BOX, reason));
    }
    let mut roots = Vec::new();
    for r in records {
        match certify_event(&op1, &op2, first, second, &r) {
            Ok(root) => roots.push(root),
            Err(reason) => regions.push((r.image, reason)),
        }
    }

    if regions.is_empty() {
        PairSolveResult::Complete(roots)
    } else {
        let reason = regions[0].1;
        PairSolveResult::Unresolved {
            certified: roots,
            reason,
            regions: regions.into_iter().map(|(b, _)| b).collect(),
        }
    }
}

fn root_to_event(
    lhs: &RationalBezierSpan2,
    rhs: &RationalBezierSpan2,
    root: CertifiedIsolatedRoot2,
) -> IsolatedEvent2 {
    let lhs_span_id = SpanId::from_occurrence(&lhs.provenance);
    let rhs_span_id = SpanId::from_occurrence(&rhs.provenance);
    let identity = EventIdentity::IsolatedRoot(IsolatedRootKey::new(
        root.span_first_id,
        root.span_second_id,
        root.ordinal,
    ));
    // The record's participants are in sorted order; the branch records map
    // back to the caller's lhs/rhs arguments by span identity so the
    // orientation-dependent occurrence data stays attached to the right span.
    // The canonical branch side follows the sorted participant pair, so it is
    // stable under operand swap and reversal.
    let lhs_is_first = lhs_span_id == root.span_first_id;
    let (lhs_side, rhs_side) = if lhs_is_first {
        (CanonicalBranchSide::First, CanonicalBranchSide::Second)
    } else {
        (CanonicalBranchSide::Second, CanonicalBranchSide::First)
    };
    let (lhs_parameter, rhs_parameter) = if lhs_is_first {
        (root.first_evidence, root.second_evidence)
    } else {
        (root.second_evidence, root.first_evidence)
    };
    // The generic Bézier path has no certified ambient context yet, so the
    // event carries the unique rank-0 context and the branches the validated
    // rank-0 zero label. GEN-001D binds rank-1/rank-2 labels where a certified
    // lattice is present.
    let context = DeckContext::rank0();
    let lhs_branch = BranchIncidence {
        span_id: lhs_span_id,
        provenance: lhs.provenance,
        parameter: lhs_parameter,
        location: ParameterLocation::PieceInterior,
        germ: BranchGerm::Regular,
        side: lhs_side,
        deck: CertifiedDeckLabel::zero(context),
        representative: root.representative,
    };
    let rhs_branch = BranchIncidence {
        span_id: rhs_span_id,
        provenance: rhs.provenance,
        parameter: rhs_parameter,
        location: ParameterLocation::PieceInterior,
        germ: BranchGerm::Regular,
        side: rhs_side,
        deck: CertifiedDeckLabel::zero(context),
        representative: root.representative,
    };
    IsolatedEvent2 {
        identity,
        crossing: CrossingClassification::Transverse,
        branches: vec![lhs_branch, rhs_branch],
        deck_context: context,
        representative: root.representative,
    }
}

/// Re-bind a set of roots found in *derived* (subdivided) sub-pairs to the
/// immutable root ids of the parent (unsplit) pair's isolation.
///
/// Derived spans carry the same occurrence identity (span id) as their parent,
/// so a sub-pair solve mints fresh pair-local ordinals that are meaningless
/// across pieces. This plumbing matches each child root to the parent roots
/// whose certified source-parameter enclosures are consistent (overlap) on both
/// participants, and applies the strict registry rule:
///
/// ```text
/// exactly one compatible parent root -> inherit the parent's immutable id
/// zero compatible parent roots       -> Unresolved
/// multiple compatible parent roots   -> Unresolved
/// ```
///
/// A sub-span cannot acquire a geometric intersection its parent lacks, so an
/// unmatched or ambiguous child root is never treated as a new root: doing so
/// would let the same geometric event enter ARR-003 under two identities. The
/// caller must consume an `Err` as typed [`GenericUnresolved::UnresolvedIdentityReferBack`]
/// and must not use the child-local ordinals as identities.
///
/// This is the registry contract ARR-003 will drive; GEN-001C provides the
/// canonical root records and source-parameter isolators the matching operates
/// on.
#[allow(dead_code)] // ARR-003 registry contract; exercised by the subdivision tests
pub(crate) fn refer_back_to_parent(
    child_roots: &mut [CertifiedIsolatedRoot2],
    parent_roots: &[CertifiedIsolatedRoot2],
) -> Result<(), GenericUnresolved> {
    // Resolve every child's match before mutating anything, so a failure leaves
    // the child ordinals untouched.
    let mut resolved = Vec::with_capacity(child_roots.len());
    for child in child_roots.iter() {
        let mut candidates = parent_roots.iter().filter(|parent| {
            parent.span_first_id == child.span_first_id
                && parent.span_second_id == child.span_second_id
                && enclosures_overlap(&parent.first_parameter, &child.first_parameter)
                && enclosures_overlap(&parent.second_parameter, &child.second_parameter)
        });
        let first = candidates.next();
        let unique = first.is_some() && candidates.next().is_none();
        match (first, unique) {
            (Some(parent), true) => resolved.push(Some(parent.ordinal)),
            _ => return Err(GenericUnresolved::UnresolvedIdentityReferBack),
        }
    }
    for (child, ordinal) in child_roots.iter_mut().zip(resolved.iter()) {
        if let Some(ordinal) = ordinal {
            child.ordinal = *ordinal;
        }
    }
    Ok(())
}

/// Whether two certified source-parameter enclosures of the same participant
/// refer to the same geometric root: they must overlap, since both certifiably
/// contain the root's exact source parameter.
fn enclosures_overlap(a: &ParameterEnclosure, b: &ParameterEnclosure) -> bool {
    a.lo <= b.hi && b.lo <= a.hi
}

/// Certify all isolated roots of a pair of homogeneous rational Bézier spans,
/// lifted to the generic contact records.
///
/// Returns `Disjoint` when the whole parameter domain is exclusion-certified,
/// `Components` of isolated transverse events when every region is certified
/// as a distinct unique root, and typed `Unresolved` (or `Unsupported`) when
/// any region cannot be certified — clustered or multiple roots, singular
/// Jacobians, stationary or tangential contacts, and boundary roots are never
/// guessed.
///
/// **GEN-001E (CommonArc).** Before the isolated-root solver runs, the
/// certified CommonArc producer ([`super::common_arc::common_arc_for_pair`]) is
/// consulted. Two spans sharing one authoritative source occurrence (identical,
/// parent/child, or overlapping siblings) certify a positive-dimensional
/// overlap directly as one [`ContactComponent2::CommonArc`] — the isolated-root
/// solver cannot express it (identical spans leave the whole domain unresolved).
///
/// A failed CommonArc precheck **never** short-circuits to `Disjoint`:
/// `EmptyOverlap` (disjoint same-source parameter intervals) does not exclude an
/// endpoint contact or a self-intersection, `PointOnlyOverlap` does not exclude
/// a tangency, and `UnsupportedSupportIdentity` (distinct occurrences) still
/// admits isolated crossings. In every such case the certified isolated-root
/// solver runs and its `Complete`/`Unresolved`/`Unsupported` outcome is
/// authoritative. Operational or certification failures of the precheck are
/// likewise resolved by the solver, which is independent certification
/// machinery; the code never swallows them as if the pair were decided.
pub fn intersect_bezier_pair(
    lhs: &RationalBezierSpan2,
    rhs: &RationalBezierSpan2,
) -> PairContactResult {
    let lhs_span = super::span::CurveSpan2::RationalBezier(lhs.clone());
    let rhs_span = super::span::CurveSpan2::RationalBezier(rhs.clone());
    match super::common_arc::common_arc_for_pair(&lhs_span, &rhs_span) {
        Ok(arc) => return PairContactResult::Components(vec![ContactComponent2::CommonArc(arc)]),
        // Any other precheck outcome: no admitted positive-dimensional
        // CommonArc was certified, so the isolated-contact solver decides the
        // pair (it may find endpoint meetings, self-intersections, tangencies,
        // or report its own typed Unresolved/Unsupported for an uncertifiable
        // coincident pair).
        Err(_) => {}
    }
    match solve_pair(lhs, rhs) {
        PairSolveResult::Complete(roots) if roots.is_empty() => PairContactResult::Disjoint,
        PairSolveResult::Complete(roots) => {
            let components: Vec<ContactComponent2> = roots
                .into_iter()
                .map(|root| ContactComponent2::IsolatedEvent(root_to_event(lhs, rhs, root)))
                .collect();
            PairContactResult::Components(components)
        }
        PairSolveResult::Unresolved { reason, .. } => PairContactResult::Unresolved(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::super::curve2d::{
        CurveOccurrenceProvenance, SourceEdgeId, SourceEntityId, SourceFaceId,
    };
    use super::*;
    use crate::source_evidence::{BoundId, EdgeUseId, SourceVertexKey};

    fn provenance(edge_index: usize) -> CurveOccurrenceProvenance {
        CurveOccurrenceProvenance {
            source_face_id: Some(SourceFaceId(7)),
            bound_id: BoundId(0),
            edge_use_id: EdgeUseId::new(BoundId(0), edge_index),
            source_edge_id: SourceEdgeId(edge_index),
            start_vertex_id: SourceVertexKey::ShellVertex(edge_index),
            end_vertex_id: SourceVertexKey::ShellVertex(edge_index + 1),
            source_curve_entity_id: Some(SourceEntityId(100 + edge_index as u64)),
        }
    }

    /// The parabola `C(u) = (u, u^2)` as a polynomial quadratic, `W = 1`.
    fn parabola(edge_index: usize) -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, 0.0, 1.0), (0.5, 0.0, 1.0), (1.0, 1.0, 1.0)],
            (0.0, 1.0),
            provenance(edge_index),
        )
        .unwrap()
    }

    /// The horizontal line `C(t) = (t, y)` as a polynomial linear span.
    fn horizontal_line(y: f64, edge_index: usize) -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, y, 1.0), (1.0, y, 1.0)],
            (0.0, 1.0),
            provenance(edge_index),
        )
        .unwrap()
    }

    /// The slanted line `C(t) = (t, m t + c)` as a polynomial linear span.
    fn slanted_line(m: f64, c: f64, edge_index: usize) -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(0.0, c, 1.0), (1.0, m + c, 1.0)],
            (0.0, 1.0),
            provenance(edge_index),
        )
        .unwrap()
    }

    /// The vertical line `C(t) = (x, t)` as a polynomial linear span.
    fn vertical_line(x: f64, edge_index: usize) -> RationalBezierSpan2 {
        RationalBezierSpan2::new(
            vec![(x, 0.0, 1.0), (x, 1.0, 1.0)],
            (0.0, 1.0),
            provenance(edge_index),
        )
        .unwrap()
    }

    #[test]
    fn transverse_interior_crossing_reaches_isolated_event() {
        // Parabola C(s) = (s, s^2) vs. horizontal line y = 1/5: a single
        // transverse interior root at s = t = sqrt(1/5) (irrational, so the
        // certified root lies strictly inside a subdivision leaf).
        let parabola = parabola(0);
        let line = horizontal_line(0.2, 1);
        let result = intersect_bezier_pair(&parabola, &line);
        let PairContactResult::Components(comps) = &result else {
            panic!("expected components, got {result:?}");
        };
        assert_eq!(comps.len(), 1, "exactly one transverse crossing");
        let ContactComponent2::IsolatedEvent(event) = &comps[0] else {
            panic!("expected an isolated event");
        };
        assert_eq!(event.crossing, CrossingClassification::Transverse);
        assert_eq!(event.branches.len(), 2);
        assert!(event.branches.iter().all(|b| b.germ == BranchGerm::Regular));
        assert!(event
            .branches
            .iter()
            .all(|b| b.location == ParameterLocation::PieceInterior));
        // The canonical identity is a pair-local isolated root with ordinal 0.
        let EventIdentity::IsolatedRoot(key) = &event.identity else {
            panic!("expected IsolatedRoot identity, got {:?}", event.identity);
        };
        assert_eq!(key.ordinal, 0);
        // The source-parameter evidence encloses the true meeting parameter
        // sqrt(1/5) on both branches (source domains are (0, 1)).
        let r = (0.2f64).sqrt();
        for b in &event.branches {
            assert!(b.parameter.lo <= r && r <= b.parameter.hi);
        }
    }

    #[test]
    fn krawczyk_certificate_verifies_inclusion_and_retains_evidence() {
        let parabola = parabola(0);
        let line = horizontal_line(0.2, 1);
        let PairSolveResult::Complete(roots) = solve_pair(&parabola, &line) else {
            panic!("expected a complete solve");
        };
        assert_eq!(roots.len(), 1);
        let root = &roots[0];
        // The certificate proves K(X) ⊂ int(X): the image is strictly inside
        // the domain, and both are inside the leaf cell.
        let cert = &root.certificate;
        assert!(cert.image.s_lo > cert.domain.s_lo);
        assert!(cert.image.s_hi < cert.domain.s_hi);
        assert!(cert.image.t_lo > cert.domain.t_lo);
        assert!(cert.image.t_hi < cert.domain.t_hi);
        assert!(root.leaf.s_lo <= root.image.s_lo && root.image.s_hi <= root.leaf.s_hi);
        assert!(root.leaf.t_lo <= root.image.t_lo && root.image.t_hi <= root.leaf.t_hi);
        // The center lies in the domain and the evidence enclosures are finite.
        assert!(cert.center[0] >= cert.domain.s_lo && cert.center[0] <= cert.domain.s_hi);
        assert!(cert.center[1] >= cert.domain.t_lo && cert.center[1] <= cert.domain.t_hi);
        for f in &cert.function_at_center {
            assert!(f.is_finite());
        }
        for row in cert.jacobian_enclosure.iter() {
            for e in row {
                assert!(e.is_finite());
            }
        }
        // The crossing orientation is signed (canonical operand order).
        assert!(matches!(
            root.crossing,
            CrossingOrientation::Positive | CrossingOrientation::Negative
        ));
    }

    #[test]
    fn unresolved_pair_retains_diagnostic_regions() {
        let parabola = parabola(0);
        let tangent = slanted_line(1.0, -0.25, 1);
        let PairSolveResult::Unresolved {
            certified, regions, ..
        } = solve_pair(&parabola, &tangent)
        else {
            panic!("expected Unresolved for an interior tangency");
        };
        // An unresolved region must be reported; the certified subset is
        // diagnostic only and never consumed as a complete relation.
        assert!(!regions.is_empty());
        let _ = certified;
    }

    #[test]
    fn bernstein_hull_certified_disjoint_pair_is_disjoint() {
        // Parabola (range y in [0,1]) vs. horizontal line y = 2: G = s^2 - 2
        // is strictly negative on the whole domain.
        let parabola = parabola(0);
        let line = horizontal_line(2.0, 1);
        let result = intersect_bezier_pair(&parabola, &line);
        assert_eq!(result, PairContactResult::Disjoint);
    }

    #[test]
    fn two_interior_roots_get_distinct_canonical_identities() {
        // Parabola vs. line C(t) = (t, t - 1/10): F = s - t, G = s^2 - t + 0.1,
        // two roots at s = (1 ± sqrt(0.6))/2, both interior and transverse.
        let parabola = parabola(0);
        let line = slanted_line(1.0, -0.1, 1);
        let result = intersect_bezier_pair(&parabola, &line);
        let PairContactResult::Components(comps) = &result else {
            panic!("expected two components, got {result:?}");
        };
        assert_eq!(comps.len(), 2, "two transverse crossings");
        let mut keys: Vec<IsolatedRootKey> = comps
            .iter()
            .filter_map(|c| match c {
                ContactComponent2::IsolatedEvent(e) => match &e.identity {
                    EventIdentity::IsolatedRoot(k) => Some(*k),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(keys.len(), 2);
        keys.sort_by_key(|k| k.ordinal);
        assert_eq!((keys[0].ordinal, keys[1].ordinal), (0, 1));
        assert_ne!(
            keys[0], keys[1],
            "distinct roots must have distinct identities"
        );
        // Participants are canonical and identical for both roots of the pair.
        assert_eq!(keys[0].participants, keys[1].participants);
        // The ordinals follow the certified source-parameter order on the
        // canonically-first span: the smaller root parameter is ordinal 0.
        assert!(keys[0].participants[0].span_id <= keys[0].participants[1].span_id);
    }

    /// The identity of the event whose branch evidence contains the source
    /// parameter `s`. Identifies a geometric root by certified evidence, so a
    /// root-by-root comparison across solves is possible rather than comparing
    /// unordered identity sets.
    fn identity_containing(result: &PairContactResult, s: f64) -> EventIdentity {
        let PairContactResult::Components(comps) = result else {
            panic!("expected components");
        };
        for c in comps {
            if let ContactComponent2::IsolatedEvent(e) = c {
                if e.branches
                    .iter()
                    .any(|b| b.parameter.lo <= s && s <= b.parameter.hi)
                {
                    return e.identity.clone();
                }
            }
        }
        panic!("no event contains source parameter {s}");
    }

    #[test]
    fn identity_is_stable_per_root_under_swap_and_reversal() {
        let parabola = parabola(0);
        let line = slanted_line(1.0, -0.1, 1);
        let s1 = (1.0 - 0.6f64.sqrt()) / 2.0;
        let s2 = (1.0 + 0.6f64.sqrt()) / 2.0;

        // Identify each geometric root in the base solve by the analytic root
        // its certified source-parameter evidence contains, and record its
        // identity.
        let base = intersect_bezier_pair(&parabola, &line);
        let id1_base = identity_containing(&base, s1);
        let id2_base = identity_containing(&base, s2);
        assert_ne!(id1_base, id2_base, "the two geometric roots are distinct");

        // Operand swap.
        let swapped = intersect_bezier_pair(&line, &parabola);
        assert_eq!(identity_containing(&swapped, s1), id1_base);
        assert_eq!(identity_containing(&swapped, s2), id2_base);

        // Reversal of the first span only.
        let reversed_line = line.reverse_occurrence();
        let rev_one = intersect_bezier_pair(&parabola, &reversed_line);
        assert_eq!(identity_containing(&rev_one, s1), id1_base);
        assert_eq!(identity_containing(&rev_one, s2), id2_base);

        // Reversal of the second span only.
        let reversed_parabola = parabola.reverse_occurrence();
        let rev_two = intersect_bezier_pair(&reversed_parabola, &line);
        assert_eq!(identity_containing(&rev_two, s1), id1_base);
        assert_eq!(identity_containing(&rev_two, s2), id2_base);

        // Reversal of both spans.
        let rev_both = intersect_bezier_pair(&reversed_parabola, &reversed_line);
        assert_eq!(identity_containing(&rev_both, s1), id1_base);
        assert_eq!(identity_containing(&rev_both, s2), id2_base);

        // Deterministic repetition.
        let repeat = intersect_bezier_pair(&parabola, &line);
        assert_eq!(identity_containing(&repeat, s1), id1_base);
        assert_eq!(identity_containing(&repeat, s2), id2_base);
    }

    #[test]
    fn subdivision_preserves_immutable_root_ids() {
        // The two-root unsplit pair: parabola vs. line C(t) = (t, t - 1/10),
        // roots at s = (1 ± sqrt(0.6))/2, both interior and transverse.
        let parabola = parabola(0);
        let line = slanted_line(1.0, -0.1, 1);
        let PairSolveResult::Complete(parent_roots) = solve_pair(&parabola, &line) else {
            panic!("expected a complete unsplit-pair solve");
        };
        assert_eq!(parent_roots.len(), 2);
        let parent_ids: Vec<u32> = parent_roots.iter().map(|r| r.ordinal).collect();
        assert_eq!(parent_ids, vec![0, 1]);

        // Subdivide the parabola at u = 1/2, between the two roots, and solve
        // each derived sub-pair independently.
        let (a1, a2) = parabola.subdivide(0.5);
        let PairSolveResult::Complete(mut r1) = solve_pair(&a1, &line) else {
            panic!("sub-pair A1 must resolve to its one root");
        };
        let PairSolveResult::Complete(mut r2) = solve_pair(&a2, &line) else {
            panic!("sub-pair A2 must resolve to its one root");
        };
        assert_eq!(r1.len(), 1);
        assert_eq!(r2.len(), 1);
        // The derived spans keep the parent occurrence's span identity.
        assert_eq!(r1[0].span_first_id, parent_roots[0].span_first_id);
        assert_eq!(r1[0].span_second_id, parent_roots[0].span_second_id);

        // Without the refer-back, every sub-pair solve mints its own ordinal 0
        // — the collision the registry fixes.
        assert_eq!(r1[0].ordinal, 0);
        assert_eq!(r2[0].ordinal, 0);

        // Refer both derived roots back to the unsplit pair's immutable ids.
        let mut all = Vec::new();
        all.append(&mut r1);
        all.append(&mut r2);
        refer_back_to_parent(&mut all, &parent_roots)
            .expect("each derived root has a unique parent match");
        let mut sub_ids: Vec<u32> = all.iter().map(|r| r.ordinal).collect();
        sub_ids.sort();
        // Each retained geometric root preserves its original identity: the two
        // derived solves map back onto the parent's two distinct root ids.
        assert_eq!(sub_ids, vec![0, 1]);
        // The mapping is per-root: the two derived roots are mutually distinct.
        assert_ne!(all[0].ordinal, all[1].ordinal);

        // Deterministic repetition of the whole derivation refers back to the
        // same immutable ids.
        let PairSolveResult::Complete(mut s1) = solve_pair(&a1, &line) else {
            unreachable!()
        };
        let PairSolveResult::Complete(mut s2) = solve_pair(&a2, &line) else {
            unreachable!()
        };
        let mut again = Vec::new();
        again.append(&mut s1);
        again.append(&mut s2);
        refer_back_to_parent(&mut again, &parent_roots)
            .expect("repetition refers back to the same ids");
        let mut again_ids: Vec<u32> = again.iter().map(|r| r.ordinal).collect();
        again_ids.sort();
        assert_eq!(again_ids, vec![0, 1]);
    }

    #[test]
    fn refer_back_never_mints_a_new_root_identity() {
        // A derived root must not keep a child-local ordinal as a new identity.
        // Zero compatible parent roots (here: a wrong registry, a different
        // span pair) is Unresolved, never a new root.
        let parabola = parabola(0);
        let line = slanted_line(1.0, -0.1, 1);
        let PairSolveResult::Complete(parent_roots) = solve_pair(&parabola, &line) else {
            unreachable!()
        };
        // Solve an unrelated pair to obtain child roots that cannot match.
        let other = slanted_line(2.0, -0.1, 5);
        let PairSolveResult::Complete(mut children) = solve_pair(&parabola, &other) else {
            unreachable!()
        };
        let before: Vec<u32> = children.iter().map(|r| r.ordinal).collect();
        let err = refer_back_to_parent(&mut children, &parent_roots).unwrap_err();
        assert_eq!(err, GenericUnresolved::UnresolvedIdentityReferBack);
        // On failure the child ordinals are left untouched (never promoted).
        let after: Vec<u32> = children.iter().map(|r| r.ordinal).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn interior_root_on_internal_subdivision_boundary_is_unresolved() {
        // Regression: an ordinary interior root lying exactly on an internal
        // subdivision boundary (the dyadic point s = t = 1/2) is never promoted
        // and never disappears. It is typed Unresolved — the conservative
        // bounded policy for subdivision-boundary clusters.
        let parabola = parabola(0);
        let line = horizontal_line(0.25, 1);
        let result = intersect_bezier_pair(&parabola, &line);
        let PairContactResult::Unresolved(reason) = &result else {
            panic!("expected Unresolved for an internal subdivision-boundary root, got {result:?}");
        };
        assert_eq!(*reason, GenericUnresolved::ClusteredRoots);
    }

    /// A root record with a synthetic box and a placeholder certificate.
    ///
    /// Test-only: the certificate is never read by the comparator or ordering
    /// tests and cannot escape this module's `#[cfg(test)]` build.
    fn dummy_record(s_lo: f64, s_hi: f64, t_lo: f64, t_hi: f64) -> RootRecord {
        let b = ParamBox {
            s_lo,
            s_hi,
            t_lo,
            t_hi,
        };
        RootRecord {
            leaf: b,
            image: b,
            ordinal: 0,
            certificate: KrawczykCertificate {
                domain: b,
                center: [(s_lo + s_hi) / 2.0, (t_lo + t_hi) / 2.0],
                preconditioner: [[1.0, 0.0], [0.0, 1.0]],
                function_at_center: [CertifiedInterval::point(0.0), CertifiedInterval::point(0.0)],
                jacobian_enclosure: [
                    [CertifiedInterval::point(1.0), CertifiedInterval::point(0.0)],
                    [CertifiedInterval::point(0.0), CertifiedInterval::point(1.0)],
                ],
                image: b,
            },
        }
    }

    #[test]
    fn certified_primary_cmp_uses_only_the_s_isolator() {
        // Overlapping s isolators certify neither equality nor order, and the t
        // coordinate is never used as a fallback — even when the t intervals
        // are disjoint.
        let a = dummy_record(0.2, 0.3, 0.5, 0.6);
        let b = dummy_record(0.25, 0.35, 0.1, 0.2);
        assert_eq!(
            certified_primary_cmp(&a, &b).unwrap_err(),
            GenericUnresolved::UnresolvedIdentityOrdering
        );
        // Disjoint s isolators certify the order regardless of t.
        let c = dummy_record(0.1, 0.2, 0.0, 1.0);
        let d = dummy_record(0.3, 0.4, 0.0, 1.0);
        assert_eq!(
            certified_primary_cmp(&c, &d).unwrap(),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            certified_primary_cmp(&d, &c).unwrap(),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn ordinal_order_is_invariant_under_refinement() {
        // The certified s-isolator order must not change when the isolators are
        // refined (bounded Krawczyk iteration inside assign_ordinals). The
        // comparison is per geometric root, identified by which analytic root
        // its enclosure contains, not by vector position or by a re-run of the
        // whole assignment on an already-sorted vector.
        let parabola = parabola(0);
        let line = slanted_line(1.0, -0.1, 1);
        let s1 = (1.0 - 0.6f64.sqrt()) / 2.0;
        let s2 = (1.0 + 0.6f64.sqrt()) / 2.0;
        let op1 = canonicalize(&parabola);
        let op2 = canonicalize(&line);
        let sys = System::new(&op1, &op2);
        let (mut records, pending) = isolate(&sys);
        assert!(
            pending.is_empty(),
            "the two roots must be certified, not pending"
        );
        let contains_s = |r: &RootRecord, s: f64| r.image.s_lo <= s && s <= r.image.s_hi;

        assign_ordinals(&mut records, &sys).expect("two separated roots order");
        // Each geometric root is identified by the analytic root its enclosure
        // contains. Record its ordinal before the second (idempotent) pass.
        let r1 = records
            .iter()
            .find(|r| contains_s(r, s1))
            .expect("the s1 root is present");
        let r2 = records
            .iter()
            .find(|r| contains_s(r, s2))
            .expect("the s2 root is present");
        let ord1_first = r1.ordinal;
        let ord2_first = r2.ordinal;
        assert_eq!((ord1_first, ord2_first), (0, 1));

        assign_ordinals(&mut records, &sys).expect("refinement preserves order");
        let ord1_second = records
            .iter()
            .find(|r| contains_s(r, s1))
            .expect("the s1 root is present")
            .ordinal;
        let ord2_second = records
            .iter()
            .find(|r| contains_s(r, s2))
            .expect("the s2 root is present")
            .ordinal;
        // The s1 root kept ordinal 0 and the s2 root kept ordinal 1 across
        // refinement — the ordinals did not swap.
        assert_eq!(ord1_second, ord1_first);
        assert_eq!(ord2_second, ord2_first);
        assert_eq!((ord1_second, ord2_second), (0, 1));
        // The certified isolators are separated on the canonical s axis.
        let sorted: Vec<&RootRecord> = {
            let mut v = records.iter().collect::<Vec<_>>();
            v.sort_by_key(|r| r.ordinal);
            v
        };
        assert!(sorted[0].image.s_hi < sorted[1].image.s_lo);
    }

    #[test]
    fn separated_ordering_assigns_lower_s_to_ordinal_zero() {
        // Two records supplied in reverse vector order (higher s first) must be
        // ordered so the lower certified s root gets ordinal 0.
        let op1 = canonicalize(&parabola(0));
        let op2 = canonicalize(&slanted_line(1.0, -0.1, 1));
        let sys = System::new(&op1, &op2);
        let (mut records, pending) = isolate(&sys);
        assert!(pending.is_empty());
        // Reverse the vector: the higher-s root now comes first.
        records.reverse();
        assign_ordinals(&mut records, &sys).expect("separated roots order");
        let lower = records
            .iter()
            .find(|r| r.image.s_hi < 0.5)
            .expect("the lower-s root is present");
        let upper = records
            .iter()
            .find(|r| r.image.s_lo > 0.5)
            .expect("the higher-s root is present");
        assert_eq!(lower.ordinal, 0);
        assert_eq!(upper.ordinal, 1);
        // And the vector is now ordered by the certified s isolator.
        assert!(records[0].image.s_hi < records[1].image.s_lo);
    }

    #[test]
    fn certificate_backs_the_refined_isolator() {
        // The invariant that the stored certificate backs the stored isolator:
        // after refinement, record.image equals record.certificate.image.
        let op1 = canonicalize(&parabola(0));
        let op2 = canonicalize(&slanted_line(1.0, -0.1, 1));
        let sys = System::new(&op1, &op2);
        let (mut records, pending) = isolate(&sys);
        assert!(pending.is_empty());
        assign_ordinals(&mut records, &sys).expect("separated roots order");
        for r in &records {
            assert_eq!(r.image, r.certificate.image);
            // The certificate's domain still encloses its image.
            assert!(r.certificate.image.s_lo > r.certificate.domain.s_lo);
            assert!(r.certificate.image.s_hi < r.certificate.domain.s_hi);
            assert!(r.certificate.image.t_lo > r.certificate.domain.t_lo);
            assert!(r.certificate.image.t_hi < r.certificate.domain.t_hi);
        }
    }

    #[test]
    fn transactional_failure_leaves_records_untouched() {
        // Two records whose s isolators remain overlapping (each box contains
        // both roots, so no Krawczyk refinement can separate them). A failed
        // assignment must leave images, certificates, ordinals, and order
        // exactly as they were.
        let op1 = canonicalize(&parabola(0));
        let op2 = canonicalize(&slanted_line(1.0, -0.1, 1));
        let sys = System::new(&op1, &op2);
        let mut roots = vec![
            dummy_record(0.1, 0.9, 0.1, 0.9),
            dummy_record(0.2, 0.8, 0.2, 0.8),
        ];
        let before = roots.clone();
        let err = assign_ordinals(&mut roots, &sys).unwrap_err();
        assert_eq!(err, GenericUnresolved::UnresolvedIdentityOrdering);
        assert_eq!(
            roots, before,
            "a failed assignment must not mutate any record"
        );
    }

    #[test]
    fn overlapping_s_isolators_that_cannot_separate_are_unresolved() {
        // Two records whose certified boxes both contain both of the pair's
        // roots: no Krawczyk refinement can separate them (a two-root box never
        // certifies uniqueness), so the overlapping s isolators must be typed
        // UnresolvedIdentityOrdering, never numbered by another coordinate.
        let op1 = canonicalize(&parabola(0));
        let op2 = canonicalize(&slanted_line(1.0, -0.1, 1));
        let sys = System::new(&op1, &op2);
        let mut roots = vec![
            dummy_record(0.1, 0.9, 0.1, 0.9),
            dummy_record(0.2, 0.8, 0.2, 0.8),
        ];
        assert_eq!(
            assign_ordinals(&mut roots, &sys).unwrap_err(),
            GenericUnresolved::UnresolvedIdentityOrdering
        );
        // A failed assignment must not assign ordinals.
        assert!(roots.iter().all(|r| r.ordinal == 0));
    }

    #[test]
    fn interior_tangent_returns_typed_unresolved() {
        // The line y = s - 1/4 is tangent to the parabola at (1/2, 1/4):
        // G = s^2 - s + 0.25 = (s - 1/2)^2, a double root. The tangent
        // determinant vanishes, so the contact must be typed Unresolved, never
        // an ordinary transverse event. The exact unresolved label is a
        // diagnostic refinement (it may surface as a tangency, a cluster, or a
        // budget-exhausted region); what must never happen is an event.
        let parabola = parabola(0);
        let tangent = slanted_line(1.0, -0.25, 1);
        let result = intersect_bezier_pair(&parabola, &tangent);
        let PairContactResult::Unresolved(_reason) = &result else {
            panic!("expected Unresolved for an interior tangency, got {result:?}");
        };
    }

    #[test]
    fn boundary_root_returns_typed_unresolved_boundary() {
        // Parabola vs. vertical line x = 0: the root at (0, 0) lies exactly on
        // the parameter-domain boundary. Per the GEN-001C endpoint policy no
        // endpoint certificate is implemented, so it must be typed Unresolved,
        // never certified as an event.
        let parabola = parabola(0);
        let line = vertical_line(0.0, 1);
        let result = intersect_bezier_pair(&parabola, &line);
        let PairContactResult::Unresolved(reason) = &result else {
            panic!("expected Unresolved for a boundary root, got {result:?}");
        };
        assert_eq!(
            *reason,
            GenericUnresolved::UnresolvedBoundaryRoot,
            "boundary root must be typed boundary-unresolved"
        );
    }

    #[test]
    fn reversed_span_canonicalizes_to_the_same_system() {
        // The forward and reversed parabola share one canonical form, so the
        // roots, ordinals, identities and source-parameter evidence agree
        // exactly. The branch provenance on the reversed span is reversed while
        // the span id is preserved.
        let forward = parabola(0);
        let line = horizontal_line(0.2, 1);
        let rev = forward.reverse_occurrence();

        let PairContactResult::Components(fwd) = intersect_bezier_pair(&forward, &line) else {
            unreachable!()
        };
        let PairContactResult::Components(rvs) = intersect_bezier_pair(&rev, &line) else {
            unreachable!()
        };
        let ContactComponent2::IsolatedEvent(ef) = &fwd[0] else {
            unreachable!()
        };
        let ContactComponent2::IsolatedEvent(er) = &rvs[0] else {
            unreachable!()
        };
        // Same canonical identity (reversal preserves the span id) and exactly
        // the same certified source-parameter evidence, because operand
        // canonicalization builds the identical system either way.
        assert_eq!(ef.identity, er.identity);
        assert_eq!(ef.branches[0].parameter, er.branches[0].parameter);
        // The reversed occurrence's branch carries the reversed provenance
        // (start and end vertices swapped) while keeping the same span id.
        assert_ne!(ef.branches[0].provenance, er.branches[0].provenance);
        assert_eq!(ef.branches[0].span_id, er.branches[0].span_id);
    }

    #[test]
    fn large_box_containing_a_root_is_not_certified_by_size() {
        // A box that contains a root but is far too large for the Krawczyk
        // inclusion must classify as Split, never as a root. Only a Krawczyk
        // certificate may promote a box.
        let op1 = canonicalize(&parabola(0));
        let op2 = canonicalize(&horizontal_line(0.25, 1));
        let sys = System::new(&op1, &op2);
        let whole = ROOT_BOX;
        let cls = classify(&sys, &whole);
        assert!(
            matches!(cls, NodeClass::Split),
            "the whole domain contains the root but is not certified by size"
        );
        // A small box far from any root is excluded by the Bernstein range.
        let far = ParamBox {
            s_lo: 0.8,
            s_hi: 0.9,
            t_lo: 0.8,
            t_hi: 0.9,
        };
        let cls = classify(&sys, &far);
        assert!(
            matches!(cls, NodeClass::Excluded),
            "a root-free box must be exclusion-certified"
        );
    }

    #[test]
    fn bernstein_product_matches_degree_elevated_reference() {
        // The quadratic coefficients (0, 1/2, 1) represent the function u, and
        // (0, 1) represents u. Their Bernstein product is u^2, whose degree-3
        // Bernstein coefficients are (0, 0, 1/3, 1).
        let a = vec![
            CertifiedInterval::point(0.0),
            CertifiedInterval::point(0.5),
            CertifiedInterval::point(1.0),
        ];
        let b = vec![CertifiedInterval::point(0.0), CertifiedInterval::point(1.0)];
        let c = bernstein_product(&a, &b);
        assert_eq!(c.len(), 4);
        let expects = [0.0, 0.0, 1.0 / 3.0, 1.0];
        for (ci, e) in c.iter().zip(expects.iter()) {
            assert!(
                ci.lo <= *e && *e <= ci.hi,
                "coeff must enclose {e}, got {ci:?}"
            );
        }
    }
}
