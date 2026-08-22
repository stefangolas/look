//! BG-ENC-004-ISC: `EnclosureCurve` for the `IntersectionCurve` decorator.
//!
//! `IntersectionCurve<C, S0, S1>` is the curve a transversal surface-surface
//! intersection produces. `subs(t)` runs `search_triple`: a 4-variable Newton
//! solve on the unknowns `q = (x, y, z, w) = ((x, y) on S0, (z, w) on S1)` for
//!
//! ```text
//! F(t; q) = [ S0(x, y) − S1(z, w) ;  L'(t)·((S0(x, y) + S1(z, w))/2 − L(t)) ]
//! ```
//!
//! (a 3-vector equation plus one scalar plane equation; `L` is the leader
//! curve), returning the midpoint of the two surface points. The leader is an
//! *approximation*, so the leader hull alone under-estimates the true curve
//! (the projection travel is the missing part), and the sound enclosure needs
//! the **certified parameter images**: a box `Q0 × Q1` (parameter boxes on each
//! surface) that provably contains the system's solution for every `t` in the
//! span. The 3-D enclosure is then pure composition:
//!
//! ```text
//! midpoint(S0.enclose(Q0), S1.enclose(Q1))
//! ```
//!
//! The certificate that produces `Q` is a **parametric Krawczyk operator** —
//! for every `t` in a t-cell, existence AND uniqueness of the solution in `Q` —
//! evaluated in interval arithmetic over the landed `EnclosureSurface` /
//! `EnclosureCurve` impls. The center term is a **point** evaluation
//! (`f_iv` at the midpoints, never at the boxes): the interval `F` over `Q`
//! drags the `p0 − p1` decorrelation (two copies of the solution arc's width)
//! into the center and doubles the linear part against the contraction term,
//! so no box certifies with it.
//!
//! Cells are **knot-aligned**: when the leader reports an exact spline, the
//! span is split at the interior knots, because a cell straddling a leader
//! knot sees the kink's derivative fan in `leader.enclose_der(1, ·)` and can
//! never certify. Measured against the real carriers (this packet's witnesses):
//!
//! - sphere-sphere (two √2-spheres at `(0, 0, ±1)`, unit-circle intersection,
//!   8- and 16-segment chord leaders): 6–12 cells per span, **0.3–2.6 ms per
//!   `enclose`**, 0 containment escapes of `subs` on 100-point grids;
//! - plane-sphere (plane `z = 0.3` cutting the unit sphere): the slice's
//!   z-width certifies to ±1.1 × 10⁻⁶;
//! - the degenerate negative (identical spheres — the system is rank-deficient
//!   everywhere): certification honestly fails and `enclose` returns the
//!   unbounded box;
//! - float `search_triple` results over 200-point grids: 0 parameter escapes
//!   from the certified boxes.
//!
//! Derivative orders 2 and above are refused with the unbounded box: the
//! carrier's `ders` recursion differentiates the 4×4 system implicitly per
//! order, and composing that in intervals is not derived — a sound widest box
//! is the honest answer (the `PCurve` fourth-order precedent). Every refusal —
//! a cell that cannot seed, a cell that cannot certify even at the bisection
//! floor, a derivative that degenerates — is the unbounded box: over-estimation
//! is always acceptable (BG-ENC-001), and a partial derivative enclosure would
//! be unsound.
//!
//! ```
//! use truck_evidence::{EnclosureCurve, Interval};
//! use truck_geometry::decorators::IntersectionCurve;
//! use truck_geometry::nurbs::{BSplineCurve, KnotVec};
//! use truck_geometry::specifieds::Sphere;
//! use truck_base::cgmath64::Point3;
//! use truck_geotrait::ParametricCurve;
//!
//! let s0 = Sphere::new(Point3::new(0.0, 0.0, 1.0), f64::sqrt(2.0));
//! let s1 = Sphere::new(Point3::new(0.0, 0.0, -1.0), f64::sqrt(2.0));
//! let mut knots = vec![0.0, 0.0];
//! for i in 1..8 {
//!     knots.push(i as f64 / 8.0);
//! }
//! knots.push(1.0);
//! knots.push(1.0);
//! let ctrl: Vec<Point3> = (0..=8)
//!     .map(|i| {
//!         let th = 0.3 + 0.7 * (i as f64) / 8.0;
//!         Point3::new(th.cos(), th.sin(), 0.0)
//!     })
//!     .collect();
//! let isc = IntersectionCurve::new(s0, s1, BSplineCurve::new(KnotVec::from(knots), ctrl));
//! let tt = Interval::try_from((0.15, 0.85)).unwrap_or(Interval::EMPTY);
//! let b = isc.enclose(tt);
//! for t in [0.2, 0.5, 0.8] {
//!     assert!(b.contains(isc.subs(t)), "subs({t}) escaped the enclosure");
//! }
//! ```

use crate::enclosure::{Box3, DirCone, EnclosureCurve, EnclosureSurface};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, SquareMatrix, Vector3, Vector4};
use truck_geometry::decorators::IntersectionCurve;
use truck_geotrait::{
    ParametricCurve, ParametricCurve3D, ParametricSurface, ParametricSurface3D,
    SearchNearestParameter, D2,
};

/// The Krawczyk inflation budget. A box that certifies at all certifies by
/// steps 5–7 (pad 0.001 … 0.016 on the sphere-sphere witness); 24 leaves ample
/// margin without making the worst case much heavier. (Decision 2.)
const MAX_INFLATIONS: usize = 24; // H-3: the inflation-step count, a dimensionless count, not a length

/// The initial `Q`-widening scale: each inflation widens the certified box
/// candidate `Q` by `pad·(1 + max(|lo|, |hi|))` outward on every axis. (Decision
/// 2 step 1.)
const INITIAL_PAD: f64 = 1.0e-6; // H-3: the dimensionless relative Q-widening scale at inflation step 0, not a length

/// Per-inflation growth of the `Q`-widening scale (decision 2).
const GROWTH: f64 = 4.0; // H-3: the dimensionless inflation-step growth factor of the Q-widening pad, not a length

/// The smallest t-cell half-width the certification bisection explores before
/// the whole call is refused (decision 3 step 4). A cell this fine that still
/// cannot certify is a genuine failure, and over-estimation refusals are the
/// unbounded box.
const CELL_FLOOR: f64 = 1.0e-12; // H-3: the dimensionless t-cell half-width bisection floor, not a length

/// The per-call budget of processed cells (decision 3 step 4's 512 cap). It
/// bounds both the certification bisection and the seed bisection on
/// rank-deficient systems (the identical-spheres negative witness), where no
/// cell anywhere can seed or certify; without it the seed bisection would
/// blow up exponentially to the `f64::EPSILON` floor.
const MAX_CELLS: usize = 512; // H-3: the processed-cell worklist cap, a dimensionless count, not a length

/// The float-evaluation guard on each composed cell box (decision 3 step 3):
/// `subs`'s float `S0.subs`/`S1.subs` at parameters the certificate proved
/// inside `Q` can drift a few ulps past the certified box's surface image.
/// `64 EPSILON (1 + |mid|)` covers the measured ulp-class float-image drift —
/// exactly `HULL_PAD`'s epistemic status in `bspline.rs`.
const NEWTON_PAD: f64 = 64.0 * f64::EPSILON; // H-3: the dimensionless relative outward pad per box endpoint, not a length

/// The largest half-angle a [`DirCone`] can report. `asin` saturates at π/2,
/// but the ulp nudge that guards the cone against inward rounding can push a
/// near-π/2 half-angle slightly past it, so the clamp keeps the half-angle
/// within its meaning as an angle. Copied from `pcurve.rs`.
const MAX_HALF_ANGLE: f64 = core::f64::consts::PI;

/// A degenerate interval from a runtime `f64`. Finite coordinates always
/// construct; a NaN widens to the empty interval rather than panicking (H-1).
/// Duplicated from the sibling decorators, which have disjoint write sets.
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The interval `[lo, hi]`. A malformed pair (NaN or `lo > hi`) degrades to the
/// empty interval rather than panicking (H-1).
fn iv_interval(lo: f64, hi: f64) -> Interval {
    Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
}

/// The unbounded box, the sound enclosure of any image the certificate refuses
/// to bound. Returned directly, never forwarded to the inner surfaces: the
/// surface carriers' behavior on unbounded parameter boxes is not uniform.
/// Copied from `pcurve.rs`.
fn unbounded_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// `a · b` in intervals, componentwise.
fn dot3(a: &Box3, b: &Box3) -> Interval {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// `a − b` per axis in intervals.
fn sub3(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

/// `a × b` componentwise in intervals.
fn cross3(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// An interval 4-vector.
type IVec4 = [Interval; 4];

/// An interval 4×4 matrix, row-major `[row][column]` unless noted.
type IMat4 = [[Interval; 4]; 4];

/// Row `r` of a cgmath `Matrix4` by explicit `match` on the row: H-1 bans
/// indexing, and a `match` is total.
fn row4(y: &Matrix4, r: usize) -> [f64; 4] {
    match r {
        0 => [y.x.x, y.y.x, y.z.x, y.w.x],
        1 => [y.x.y, y.y.y, y.z.y, y.w.y],
        2 => [y.x.z, y.y.z, y.z.z, y.w.z],
        _ => [y.x.w, y.y.w, y.z.w, y.w.w],
    }
}

/// `Interval · scalar`. inari implements `Mul` for `Interval` only, not for
/// `f64` operands, so the scalar side is wrapped.
fn mul_scalar(iv: Interval, s: f64) -> Interval {
    iv * interval_at(s)
}

/// `v · s` — the interval 4-vector `v` dotted with the float 4-vector `s`.
fn dot4iv(v: &IVec4, s: [f64; 4]) -> Interval {
    let [v0, v1, v2, v3] = *v;
    let [s0, s1, s2, s3] = s;
    mul_scalar(v0, s0) + mul_scalar(v1, s1) + mul_scalar(v2, s2) + mul_scalar(v3, s3)
}

/// `a · b` for two interval 4-vectors.
fn dot_ivec4(a: &IVec4, b: &IVec4) -> Interval {
    let [a0, a1, a2, a3] = *a;
    let [b0, b1, b2, b3] = *b;
    a0 * b0 + a1 * b1 + a2 * b2 + a3 * b3
}

/// `Y · f` — the float 4×4 `Y` times the interval 4-vector `f`.
fn y_times_fvec(y: &Matrix4, f: &IVec4) -> IVec4 {
    [
        dot4iv(f, row4(y, 0)),
        dot4iv(f, row4(y, 1)),
        dot4iv(f, row4(y, 2)),
        dot4iv(f, row4(y, 3)),
    ]
}

/// `Y · J` — the float 4×4 `Y` times the interval 4×4 `J`. `J` is stored
/// `[param][equation]` (column-major, the `j_iv` construction); the result is
/// row-major `[equation][param]`, so `out[r][c] = Σ_k Y[r][k]·J[c][k]` — note
/// `J[c][k]`, NOT `J[k][c]`. Getting this transposed computes `Y·Jᵀ`; its
/// widths look healthy and its centers are O(1) off — the failure is silent
/// and total. (Decision 2.)
fn y_times_imat(y: &Matrix4, j: &IMat4) -> IMat4 {
    let [j0, j1, j2, j3] = *j;
    let [r0, r1, r2, r3] = [row4(y, 0), row4(y, 1), row4(y, 2), row4(y, 3)];
    [
        [
            dot4iv(&j0, r0),
            dot4iv(&j1, r0),
            dot4iv(&j2, r0),
            dot4iv(&j3, r0),
        ],
        [
            dot4iv(&j0, r1),
            dot4iv(&j1, r1),
            dot4iv(&j2, r1),
            dot4iv(&j3, r1),
        ],
        [
            dot4iv(&j0, r2),
            dot4iv(&j1, r2),
            dot4iv(&j2, r2),
            dot4iv(&j3, r2),
        ],
        [
            dot4iv(&j0, r3),
            dot4iv(&j1, r3),
            dot4iv(&j2, r3),
            dot4iv(&j3, r3),
        ],
    ]
}

/// `I − a` elementwise, `a` row-major.
fn identity_minus(a: &IMat4) -> IMat4 {
    let [a0, a1, a2, a3] = *a;
    let [a00, a01, a02, a03] = a0;
    let [a10, a11, a12, a13] = a1;
    let [a20, a21, a22, a23] = a2;
    let [a30, a31, a32, a33] = a3;
    let one = interval_at(1.0);
    let zero = interval_at(0.0);
    [
        [one - a00, zero - a01, zero - a02, zero - a03],
        [zero - a10, one - a11, zero - a12, zero - a13],
        [zero - a20, zero - a21, one - a22, zero - a23],
        [zero - a30, zero - a31, zero - a32, one - a33],
    ]
}

/// `a · v` — row-major `a` times the interval 4-vector `v`.
fn imat_times_ivec(a: &IMat4, v: &IVec4) -> IVec4 {
    let [a0, a1, a2, a3] = *a;
    [
        dot_ivec4(&a0, v),
        dot_ivec4(&a1, v),
        dot_ivec4(&a2, v),
        dot_ivec4(&a3, v),
    ]
}

/// The double-projection system, holding the three carrier references.
struct Sys<'a, C, S0, S1> {
    leader: &'a C,
    s0: &'a S0,
    s1: &'a S1,
}

impl<'a, C, S0, S1> Sys<'a, C, S0, S1>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    /// The interval `F` over the parameter boxes and the t-cell (decision 1).
    fn f_iv(
        &self,
        xb: Interval,
        yb: Interval,
        zb: Interval,
        wb: Interval,
        tt: Interval,
    ) -> [Interval; 4] {
        let p0 = self.s0.enclose(xb, yb);
        let p1 = self.s1.enclose(zb, wb);
        let d = Box3 {
            x: p0.x - p1.x,
            y: p0.y - p1.y,
            z: p0.z - p1.z,
        };
        let l = self.leader.enclose(tt);
        let n = self.leader.enclose_der(1, tt);
        let half = interval_at(0.5);
        let m = Box3 {
            x: (p0.x + p1.x) * half - l.x,
            y: (p0.y + p1.y) * half - l.y,
            z: (p0.z + p1.z) * half - l.z,
        };
        let f4 = dot3(&n, &m);
        [d.x, d.y, d.z, f4]
    }

    /// The interval Jacobian, stored `[param][equation]` (column-major) because
    /// the natural construction builds one array per parameter column. The S1
    /// columns negate the 3-D part ONLY; the fourth component keeps `+n·d/2`,
    /// exactly the carrier's `(-uder1).extend(plane_normal.dot(uder1) / 2.0)`
    /// (decision 1 — negating the whole `Vector4` makes the two azimuthal
    /// columns exactly parallel on symmetric witnesses and nothing certifies).
    fn j_iv(
        &self,
        xb: Interval,
        yb: Interval,
        zb: Interval,
        wb: Interval,
        tt: Interval,
    ) -> [[Interval; 4]; 4] {
        let n = self.leader.enclose_der(1, tt);
        let half = interval_at(0.5);
        let u0 = self.s0.enclose_der(1, 0, xb, yb);
        let v0 = self.s0.enclose_der(0, 1, xb, yb);
        let u1 = self.s1.enclose_der(1, 0, zb, wb);
        let v1 = self.s1.enclose_der(0, 1, zb, wb);
        let col = |d: &Box3| [d.x, d.y, d.z, (n.x * d.x + n.y * d.y + n.z * d.z) * half];
        let col_neg = |d: &Box3| [-d.x, -d.y, -d.z, (n.x * d.x + n.y * d.y + n.z * d.z) * half];
        [col(&u0), col(&v0), col_neg(&u1), col_neg(&v1)]
    }

    /// The float Jacobian at a point, mirroring the same column-major
    /// convention. `der_mn(m, n, u, v)` is u-order `m`, v-order `n`. The
    /// `Option` is for total-behaviour symmetry with `invert`; this body cannot
    /// fail and returns `Some` directly.
    fn j_fl(&self, q: [f64; 4], t: f64) -> Option<Matrix4> {
        let n = self.leader.der(t);
        let [x, y, z, w] = q;
        let u0 = self.s0.der_mn(1, 0, x, y);
        let v0 = self.s0.der_mn(0, 1, x, y);
        let u1 = self.s1.der_mn(1, 0, z, w);
        let v1 = self.s1.der_mn(0, 1, z, w);
        let colf = |d: Vector3| Vector4::new(d.x, d.y, d.z, n.dot(d) / 2.0);
        let colf_neg = |d: Vector3| Vector4::new(-d.x, -d.y, -d.z, n.dot(d) / 2.0);
        Some(Matrix4::from_cols(
            colf(u0),
            colf(v0),
            colf_neg(u1),
            colf_neg(v1),
        ))
    }
}

/// Certifies ONE t-cell by the parametric Krawczyk operator (decision 2):
/// existence AND uniqueness of the system's solution in the returned box for
/// every `t` in `cell`. `None` means the cell cannot certify (the caller
/// bisects). Freestanding, with the impl's bounds.
fn certify_cell<C, S0, S1>(
    sys: &Sys<C, S0, S1>,
    cell: Interval,
    q_lo: [f64; 4],
    q_hi: [f64; 4],
    t_mid: f64,
) -> Option<[Interval; 4]>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let [l0, l1, l2, l3] = q_lo;
    let [h0, h1, h2, h3] = q_hi;
    let mut pad = INITIAL_PAD;
    for _ in 0..MAX_INFLATIONS {
        // 1. Q: per axis, the HULL of `q_lo[a]`, `q_hi[a]` (the seed parameters
        //    can DECREASE with t, so the hull is min/max — an assumed ordering
        //    builds an inverted interval) widened by `pad·(1 + max(|lo|, |hi|))`
        //    on each side; `m` the float midpoints.
        let widen = |lo: f64, hi: f64| {
            let (a, b) = (lo.min(hi), lo.max(hi));
            let p = pad * (1.0 + a.abs().max(b.abs()));
            Interval::try_from((a - p, b + p)).unwrap_or(Interval::EMPTY)
        };
        let q = [widen(l0, h0), widen(l1, h1), widen(l2, h2), widen(l3, h3)];
        let m = [
            (l0 + h0) / 2.0,
            (l1 + h1) / 2.0,
            (l2 + h2) / 2.0,
            (l3 + h3) / 2.0,
        ];
        let [m0, m1, m2, m3] = m;
        // 2. The inverse float Jacobian at the midpoints; a singular Jacobian
        //    fails the cell (the caller bisects).
        let y = sys.j_fl(m, t_mid)?.invert()?;
        // 3. The center term is a POINT evaluation — `f_iv` at the four
        //    degenerate midpoints and `t_mid`, NEVER at the boxes Q/cell: the
        //    interval F over Q drags the p0−p1 decorrelation into the center
        //    and doubles the linear part against the contraction term.
        let f = sys.f_iv(
            interval_at(m0),
            interval_at(m1),
            interval_at(m2),
            interval_at(m3),
            interval_at(t_mid),
        );
        // 4. The interval Jacobian over the boxes Q, cell (the t-dependence
        //    enters here, soundly, as the interval over the cell).
        let [q0, q1, q2, q3] = q;
        let j = sys.j_iv(q0, q1, q2, q3, cell);
        // 5. K = m − Y·f + (I − Y·J)·(q − m), elementwise.
        let yf = y_times_fvec(&y, &f);
        let yj = y_times_imat(&y, &j);
        let qminusm = [
            q0 - interval_at(m0),
            q1 - interval_at(m1),
            q2 - interval_at(m2),
            q3 - interval_at(m3),
        ];
        let k = imat_times_ivec(&identity_minus(&yj), &qminusm);
        let [yf0, yf1, yf2, yf3] = yf;
        let [k0, k1, k2, k3] = k;
        let ka = [
            interval_at(m0) - yf0 + k0,
            interval_at(m1) - yf1 + k1,
            interval_at(m2) - yf2 + k2,
            interval_at(m3) - yf3 + k3,
        ];
        // 6. Certify iff STRICT interior containment on all four axes
        //    (non-strict proves existence, not uniqueness).
        let [ka0, ka1, ka2, ka3] = ka;
        if ka0.inf() > q0.inf()
            && ka0.sup() < q0.sup()
            && ka1.inf() > q1.inf()
            && ka1.sup() < q1.sup()
            && ka2.inf() > q2.inf()
            && ka2.sup() < q2.sup()
            && ka3.inf() > q3.inf()
            && ka3.sup() < q3.sup()
        {
            return Some(q);
        }
        pad *= GROWTH;
    }
    None
}

/// The cell's 3-D box from a certified parameter box (decision 3 step 3):
/// `midpoint(S0.enclose(Q0), S1.enclose(Q1))`, widened by `NEWTON_PAD` — the
/// float-evaluation guard for `subs`'s float surface evaluation at parameters
/// the certificate proved inside `Q`.
fn compose_cell_box<C, S0, S1>(sys: &Sys<C, S0, S1>, q: &[Interval; 4]) -> Box3
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let [qx, qy, qz, qw] = *q;
    let p0 = sys.s0.enclose(qx, qy);
    let p1 = sys.s1.enclose(qz, qw);
    let half = interval_at(0.5);
    let b = Box3 {
        x: (p0.x + p1.x) * half,
        y: (p0.y + p1.y) * half,
        z: (p0.z + p1.z) * half,
    };
    let widen = |iv: Interval| {
        if !iv.inf().is_finite() || !iv.sup().is_finite() {
            return iv;
        }
        let pad = NEWTON_PAD * (1.0 + iv.mid().abs());
        Interval::try_from((iv.inf() - pad, iv.sup() + pad)).unwrap_or(Interval::EMPTY)
    };
    Box3 {
        x: widen(b.x),
        y: widen(b.y),
        z: widen(b.z),
    }
}

/// The `n = 1` derivative box for one cell (decision 4): the carrier's `der`
/// formula `der(t) = n̂·k` with `n̂ = (S0_u×S0_v)×(S1_u×S1_v)` and
/// `k = (|L'|² − (c − L)·L'') / (n̂·L')`, composed in intervals over the
/// certified `q` and the cell. If the denominator contains 0 the leader's
/// tangent lies in the constraint plane and the parametrization degenerates —
/// the whole `der1` result is the unbounded box (the family's `None` condition
/// arriving numerically; inari's division would return ENTIRE anyway, but the
/// check is explicit).
fn der1_box<C, S0, S1>(sys: &Sys<C, S0, S1>, q: &[Interval; 4], cell: Interval) -> Box3
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let [qx, qy, qz, qw] = *q;
    let u0 = sys.s0.enclose_der(1, 0, qx, qy);
    let v0 = sys.s0.enclose_der(0, 1, qx, qy);
    let u1 = sys.s1.enclose_der(1, 0, qz, qw);
    let v1 = sys.s1.enclose_der(0, 1, qz, qw);
    let nbox = cross3(&cross3(&u0, &v0), &cross3(&u1, &v1));
    let c = compose_cell_box(sys, q);
    let l = sys.leader.enclose(cell);
    let l1 = sys.leader.enclose_der(1, cell);
    let l2 = sys.leader.enclose_der(2, cell);
    let num = dot3(&l1, &l1) - dot3(&sub3(&c, &l), &l2);
    let den = dot3(&nbox, &l1);
    if den.inf() <= 0.0 && den.sup() >= 0.0 {
        return unbounded_box();
    }
    let k = num / den;
    Box3 {
        x: nbox.x * k,
        y: nbox.y * k,
        z: nbox.z * k,
    }
}

/// One seed: `search_triple` at `t`, mapped to `[uv0.x, uv0.y, uv1.x, uv1.y]`.
fn seed_at<C, S0, S1>(isc: &IntersectionCurve<C, S0, S1>, t: f64) -> Option<[f64; 4]>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let (_, uv0, uv1) = isc.search_triple(t, 100)?;
    Some([uv0.x, uv0.y, uv1.x, uv1.y])
}

/// Seeds the cell (decision 3 step 2): `search_triple` at both endpoints,
/// falling back to the midpoint when either fails; on total failure, bisect
/// the cell while its half-width exceeds `f64::EPSILON`. `None` means the seed
/// never exists — even at the floor — and the whole call is refused. The
/// budget bounds the seed bisection on systems with no seeds anywhere (the
/// rank-deficient negative witness).
fn seed_pair<C, S0, S1>(
    isc: &IntersectionCurve<C, S0, S1>,
    cell: Interval,
    budget: &mut usize,
) -> Option<([f64; 4], [f64; 4])>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    *budget += 1;
    if *budget > MAX_CELLS {
        return None;
    }
    let t_mid = (cell.inf() + cell.sup()) / 2.0;
    let lo = seed_at(isc, cell.inf());
    let hi = seed_at(isc, cell.sup());
    match (lo, hi) {
        (Some(l), Some(h)) => Some((l, h)),
        (l, h) => match seed_at(isc, t_mid) {
            Some(m) => Some(match (l, h) {
                (Some(l), None) => (l, m),
                (None, Some(h)) => (m, h),
                _ => (m, m),
            }),
            None => {
                let half = (cell.sup() - cell.inf()) / 2.0;
                if half > f64::EPSILON {
                    let mid = (cell.inf() + cell.sup()) / 2.0;
                    let left = iv_interval(cell.inf(), mid);
                    let right = iv_interval(mid, cell.sup());
                    match seed_pair(isc, left, budget) {
                        Some(pair) => Some(pair),
                        None => seed_pair(isc, right, budget),
                    }
                } else {
                    None
                }
            }
        },
    }
}

/// The knot-aligned initial cells (decision 3 step 1): when the leader reports
/// an exact spline, split `tt` at every interior knot strictly inside it (a
/// cell straddling a leader knot sees the kink's derivative fan and cannot
/// certify); otherwise one cell covering `tt`, paying bisection instead.
fn initial_cells<C, S0, S1>(isc: &IntersectionCurve<C, S0, S1>, tt: Interval) -> Vec<Interval>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let mut cuts: Vec<f64> = Vec::new();
    if let Some(bsp) = isc.leader().exact_spline() {
        for k in bsp.knot_vec().iter() {
            if *k > tt.inf() && *k < tt.sup() {
                cuts.push(*k);
            }
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup();
    let mut cells = Vec::new();
    let mut lo = tt.inf();
    for hi in cuts {
        cells.push(iv_interval(lo, hi));
        lo = hi;
    }
    cells.push(iv_interval(lo, tt.sup()));
    cells
}

/// The cell's certified 3-D box (seeding + Krawczyk + composition), or `None`
/// when the cell cannot certify at its current size (the caller bisects or
/// refuses).
fn certify_cell_box<C, S0, S1>(
    isc: &IntersectionCurve<C, S0, S1>,
    cell: Interval,
    budget: &mut usize,
) -> Option<Box3>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let (q_lo, q_hi) = seed_pair(isc, cell, budget)?;
    let sys = Sys {
        leader: isc.leader(),
        s0: isc.surface0(),
        s1: isc.surface1(),
    };
    let t_mid = (cell.inf() + cell.sup()) / 2.0;
    let q = certify_cell(&sys, cell, q_lo, q_hi, t_mid)?;
    Some(compose_cell_box(&sys, &q))
}

/// The cell's certified `n = 1` box (seeding + Krawczyk + the `der` formula).
fn der1_cell<C, S0, S1>(
    isc: &IntersectionCurve<C, S0, S1>,
    cell: Interval,
    budget: &mut usize,
) -> Option<Box3>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let (q_lo, q_hi) = seed_pair(isc, cell, budget)?;
    let sys = Sys {
        leader: isc.leader(),
        s0: isc.surface0(),
        s1: isc.surface1(),
    };
    let t_mid = (cell.inf() + cell.sup()) / 2.0;
    let q = certify_cell(&sys, cell, q_lo, q_hi, t_mid)?;
    Some(der1_box(&sys, &q, cell))
}

/// Holds `b` into `acc`, per-axis `convex_hull`.
fn hull_accumulate(acc: Option<Box3>, b: Box3) -> Option<Box3> {
    Some(match acc {
        None => b,
        Some(prev) => Box3 {
            x: prev.x.convex_hull(b.x),
            y: prev.y.convex_hull(b.y),
            z: prev.z.convex_hull(b.z),
        },
    })
}

/// The whole-span worklist (decisions 3): seed + certify each cell, bisecting
/// certification failures while the half-width exceeds `CELL_FLOOR`, capping
/// the worklist at `MAX_CELLS` processed cells. `None` is the unbounded-box
/// refusal — every path out of here either succeeds for every cell or refuses
/// the whole call.
fn enclose_span<C, S0, S1>(isc: &IntersectionCurve<C, S0, S1>, tt: Interval) -> Option<Box3>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let mut budget = 0usize;
    let mut stack = initial_cells(isc, tt);
    let mut acc: Option<Box3> = None;
    while let Some(cell) = stack.pop() {
        budget += 1;
        if budget > MAX_CELLS {
            return None;
        }
        match certify_cell_box(isc, cell, &mut budget) {
            Some(b) => acc = hull_accumulate(acc, b),
            None => {
                let half = (cell.sup() - cell.inf()) / 2.0;
                if half > CELL_FLOOR {
                    let mid = (cell.inf() + cell.sup()) / 2.0;
                    stack.push(iv_interval(cell.inf(), mid));
                    stack.push(iv_interval(mid, cell.sup()));
                } else {
                    return None;
                }
            }
        }
    }
    acc
}

/// The whole-span `n = 1` worklist (decision 4): re-derive the certification
/// per cell (deterministic; the same cost as `enclose`), then compose the
/// carrier's `der` formula in intervals. If ANY cell fails, the whole result is
/// the unbounded box — a partial derivative enclosure would be unsound.
fn der1_span<C, S0, S1>(isc: &IntersectionCurve<C, S0, S1>, tt: Interval) -> Option<Box3>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    let mut budget = 0usize;
    let mut stack = initial_cells(isc, tt);
    let mut acc: Option<Box3> = None;
    while let Some(cell) = stack.pop() {
        budget += 1;
        if budget > MAX_CELLS {
            return None;
        }
        match der1_cell(isc, cell, &mut budget) {
            Some(b) => acc = hull_accumulate(acc, b),
            None => {
                let half = (cell.sup() - cell.inf()) / 2.0;
                if half > CELL_FLOOR {
                    let mid = (cell.inf() + cell.sup()) / 2.0;
                    stack.push(iv_interval(cell.inf(), mid));
                    stack.push(iv_interval(mid, cell.sup()));
                } else {
                    return None;
                }
            }
        }
    }
    acc
}

impl<C, S0, S1> EnclosureCurve for IntersectionCurve<C, S0, S1>
where
    C: ParametricCurve<Point = Point3, Vector = Vector3> + ParametricCurve3D + EnclosureCurve,
    S0: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
    S1: ParametricSurface<Point = Point3, Vector = Vector3>
        + ParametricSurface3D
        + EnclosureSurface
        + SearchNearestParameter<D2, Point = Point3>,
{
    fn enclose(&self, tt: Interval) -> Box3 {
        // Empty or non-finite `tt` (NaN bounds, inf > sup) → the empty box,
        // mirroring `pcurve.rs`.
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        match enclose_span(self, tt) {
            Some(b) => b,
            None => unbounded_box(),
        }
    }

    fn enclose_der(&self, n: usize, tt: Interval) -> Box3 {
        match n {
            0 => return self.enclose(tt),
            1 => {}
            // n >= 2 → the unbounded box: the carrier's `ders` recursion
            // differentiates the 4×4 system implicitly per order, and composing
            // that in intervals is not derived — a sound widest box is the
            // honest answer (the PCURVE fourth-order precedent).
            _ => return unbounded_box(),
        }
        if tt.is_empty() || !tt.inf().is_finite() || !tt.sup().is_finite() {
            return Box3::empty();
        }
        match der1_span(self, tt) {
            Some(b) => b,
            None => unbounded_box(),
        }
    }

    fn tangent_cone(&self, tt: Interval) -> Option<DirCone> {
        // The ball-around-midpoint cone off the n = 1 box (decision 5): let c
        // be the box's midpoint vector and h its half-width vector; every
        // element of the box lies within rho = ‖h‖ of c, so if rho < ‖c‖ every
        // element makes an angle of at most asin(rho / ‖c‖) with c. rho >= ‖c‖
        // is exactly the case where the box may contain the zero vector or
        // straddle enough directions that no cone bounds it — the derivative
        // crossing zero (a cusp, a transversal failure, or the k-degeneracy of
        // decision 4) — so None is the contract. rho rounds UP (.sup()) and
        // ‖c‖ DOWN (.inf()); the ratio, the asin and the normalize() are then
        // nudged up by a few ulps so none of them can round the cone too
        // narrow. `enclose_der(1, ·)` returning the unbounded box makes the
        // hull contain 0 → None — correct by construction.
        let b = self.enclose_der(1, tt);
        let c = Vector3::new(b.x.mid(), b.y.mid(), b.z.mid());
        let h = Vector3::new(b.x.wid() / 2.0, b.y.wid() / 2.0, b.z.wid() / 2.0);
        let norm = |x: Interval, y: Interval, z: Interval| (x.sqr() + y.sqr() + z.sqr()).sqrt();
        let rho = norm(interval_at(h.x), interval_at(h.y), interval_at(h.z)).sup();
        let cn = norm(interval_at(c.x), interval_at(c.y), interval_at(c.z)).inf();
        // The guard in the packet's order and form; `!(cn > rho)` is rejected
        // by clippy's neg_cmp_op_on_partial_ord, and the finiteness tests make
        // the two equivalent (they differ only on NaN). The empty box or a
        // hull containing the origin yields NaN or zero cn.
        if !cn.is_finite() || !rho.is_finite() || cn <= rho {
            return None;
        }
        let axis = c.normalize();
        let half_angle = (rho / cn).asin();
        let half_angle = half_angle * (1.0 + 8.0 * f64::EPSILON) + 8.0 * f64::EPSILON;
        Some(DirCone {
            axis,
            half_angle: half_angle.min(MAX_HALF_ANGLE),
        })
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::harness::assert_encloses_curve;
    use truck_base::cgmath64::Point3;
    use truck_geometry::nurbs::{BSplineCurve, KnotVec};
    use truck_geometry::specifieds::{Plane, Sphere};

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The certified z-slice width budget of the plane-sphere witness: the
    /// measured certified z-width is 2.2e-6, so 10⁻⁴ leaves room for the
    /// certification's pad choice without blessing a collapse to the unbounded
    /// box. (The width is a length on the unit-scale witness, but the gate is
    /// a model-space-relative tightness bound, not a tolerance compared
    /// against a length.)
    const SLICE_SLACK: f64 = 1.0e-4; // H-3: the certified z-slice width budget of the unit-scale plane-sphere witness, not a length

    /// The finite-difference step in the `der1` test: small enough that the
    /// central difference tracks the tangent, large enough that round-off does
    /// not dominate. A dimensionless parameter offset.
    const FD_STEP: f64 = 1.0e-5; // H-3: the dimensionless finite-difference parameter step, not a length

    /// Cone containment by angle: cos(angle between axis and d) >=
    /// cos(half_angle). A half_angle at or near π/2 needs the `>=` with float
    /// tolerance to survive rounding, so the slack lives here in the test,
    /// never in the cone.
    fn cone_contains(cone: &DirCone, d: Vector3) -> bool {
        let cos_angle = cone.axis.dot(d.normalize());
        cos_angle >= cone.half_angle.cos() - 1.0e-12 // H-3: float slack between two direction cosines, not a length
    }

    /// Two sqrt(2)-spheres at (0, 0, ±1): the intersection is the unit circle
    /// in the z = 0 plane.
    fn sphere_pair() -> (Sphere, Sphere) {
        (
            Sphere::new(Point3::new(0.0, 0.0, 1.0), f64::sqrt(2.0)),
            Sphere::new(Point3::new(0.0, 0.0, -1.0), f64::sqrt(2.0)),
        )
    }

    /// A chord-polyline leader on the unit circle, `theta in [0.3, 1.0]`,
    /// `segs` equal segments, as a clamped degree-1 BSplineCurve<Point3> with
    /// control points ON the circle (chord sag = the leader's coarseness).
    fn chord_leader(segs: usize) -> BSplineCurve<Point3> {
        let mut knots = vec![0.0, 0.0];
        for i in 1..segs {
            knots.push(i as f64 / segs as f64);
        }
        knots.push(1.0);
        knots.push(1.0);
        let ctrl: Vec<Point3> = (0..=segs)
            .map(|i| {
                let th = 0.3 + 0.7 * (i as f64) / (segs as f64);
                Point3::new(th.cos(), th.sin(), 0.0)
            })
            .collect();
        BSplineCurve::new(KnotVec::from(knots), ctrl)
    }

    /// The sphere-sphere witness with an `segs`-segment chord leader.
    fn sphere_sphere(segs: usize) -> IntersectionCurve<BSplineCurve<Point3>, Sphere, Sphere> {
        let (s0, s1) = sphere_pair();
        IntersectionCurve::new(s0, s1, chord_leader(segs))
    }

    /// The plane-sphere witness: the plane z = 0.3 cutting the unit sphere,
    /// with a 12-segment chord leader over theta in [0.2, 1.2] (the slice
    /// circle has radius sqrt(1 − 0.09) at z = 0.3).
    fn plane_sphere() -> IntersectionCurve<BSplineCurve<Point3>, Plane, Sphere> {
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 0.3),
            Point3::new(1.0, 0.0, 0.3),
            Point3::new(0.0, 1.0, 0.3),
        );
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0);
        let segs = 12usize;
        let mut knots = vec![0.0, 0.0];
        for i in 1..segs {
            knots.push(i as f64 / segs as f64);
        }
        knots.push(1.0);
        knots.push(1.0);
        let ctrl: Vec<Point3> = (0..=segs)
            .map(|i| {
                let th = 0.2 + (i as f64) / (segs as f64);
                Point3::new(th.cos(), th.sin(), 0.0)
            })
            .collect();
        let leader = BSplineCurve::new(KnotVec::from(knots), ctrl);
        IntersectionCurve::new(plane, sphere, leader)
    }

    /// The degenerate negative witness: two identical unit spheres. The double
    /// projection's system is rank-deficient everywhere, so no cell can
    /// certify.
    fn identical_spheres() -> IntersectionCurve<BSplineCurve<Point3>, Sphere, Sphere> {
        let s = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0);
        IntersectionCurve::new(s, s, chord_leader(8))
    }

    #[test]
    fn isc_encloses_sampled_sphere_sphere() {
        for isc in [sphere_sphere(8), sphere_sphere(16)] {
            // The full span and an interior span, on both leader coarsenesses.
            assert_encloses_curve(&isc, iv(0.15, 0.85), 40);
            assert_encloses_curve(&isc, iv(0.3, 0.7), 40);
            // The box must be FINITE — a certification collapse to the
            // unbounded box must fail this test, not silently pass soundness.
            let b = isc.enclose(iv(0.15, 0.85));
            for axis in [b.x, b.y, b.z] {
                assert!(
                    axis.sup() - axis.inf() < 1.0,
                    "axis not finite-class: {axis:?}"
                );
            }
        }
    }

    #[test]
    fn isc_plane_sphere_slice_is_tight() {
        let isc = plane_sphere();
        assert_encloses_curve(&isc, iv(0.1, 0.9), 40);
        let b = isc.enclose(iv(0.1, 0.9));
        let z_width = b.z.sup() - b.z.inf();
        assert!(
            z_width <= SLICE_SLACK,
            "z-slice width {z_width} exceeds the certified budget {SLICE_SLACK}"
        );
    }

    #[test]
    fn isc_identical_surfaces_refuse_whole() {
        let isc = identical_spheres();
        let b = isc.enclose(iv(0.1, 0.9));
        assert_eq!(
            b.x,
            Interval::ENTIRE,
            "x not unbounded for the negative witness"
        );
        assert_eq!(
            b.y,
            Interval::ENTIRE,
            "y not unbounded for the negative witness"
        );
        assert_eq!(
            b.z,
            Interval::ENTIRE,
            "z not unbounded for the negative witness"
        );
        // A whole-box enclose whose cone still returned Some would mean the
        // cone is not wired off the refused derivative enclosure.
        assert!(
            isc.tangent_cone(iv(0.1, 0.9)).is_none(),
            "negative-witness cone must be None"
        );
    }

    #[test]
    fn isc_out_of_range_span_is_unbounded() {
        let isc = sphere_sphere(8);
        // Outside the leader's [0, 1] range the leader hull is unbounded and no
        // certification can hold; the sound answer propagates.
        for tt in [iv(-1.0, 0.5), iv(0.5, 2.0), iv(-10.0, 10.0)] {
            let b = isc.enclose(tt);
            assert_eq!(b.x, Interval::ENTIRE, "x not unbounded for {tt:?}");
            assert_eq!(b.y, Interval::ENTIRE, "y not unbounded for {tt:?}");
            assert_eq!(b.z, Interval::ENTIRE, "z not unbounded for {tt:?}");
        }
    }

    #[test]
    fn isc_der1_contains_finite_differences() {
        let isc = plane_sphere();
        let tt = iv(0.2, 0.8);
        let enc = isc.enclose_der(1, tt);
        assert!(
            enc.x.sup().is_finite() && enc.y.sup().is_finite() && enc.z.sup().is_finite(),
            "der1 enclosure not finite: {enc:?}"
        );
        // The central difference at 5 grid t's is contained per axis. The grid
        // avoids the 12-segment leader's knots (k/12): `subs` is only smooth
        // inside a span, and a central difference straddling a chord kink
        // captures the kink's slope jump, not the tangent the enclosure bounds.
        for t in [0.2, 0.32, 0.44, 0.56, 0.68] {
            let d = (isc.subs(t + FD_STEP) - isc.subs(t - FD_STEP)) / (2.0 * FD_STEP);
            assert!(
                enc.x.contains(d.x) && enc.y.contains(d.y) && enc.z.contains(d.z),
                "central difference at t={t} escaped the der1 enclosure: {d:?}"
            );
        }
    }

    #[test]
    fn isc_der_above_one_is_unbounded() {
        let isc = sphere_sphere(8);
        for n in [2usize, 3] {
            let b = isc.enclose_der(n, iv(0.2, 0.8));
            assert_eq!(b.x, Interval::ENTIRE, "x not unbounded for n = {n}");
            assert_eq!(b.y, Interval::ENTIRE, "y not unbounded for n = {n}");
            assert_eq!(b.z, Interval::ENTIRE, "z not unbounded for n = {n}");
        }
    }

    #[test]
    fn isc_tangent_cone_contains_exact_circle() {
        let isc = sphere_sphere(8);
        let tt = iv(0.2, 0.8);
        let cone = isc
            .tangent_cone(tt)
            .expect("the sphere-sphere cone must exist over [0.2, 0.8]");
        // The ISC's tangent direction IS the circle tangent; the leader's
        // k-factor only scales it.
        for i in 0..9 {
            let t = tt.inf() + (tt.sup() - tt.inf()) * (i as f64) / 8.0;
            let th = 0.3 + 0.7 * t;
            let d = Vector3::new(-th.sin(), th.cos(), 0.0);
            assert!(
                cone_contains(&cone, d),
                "unit-circle tangent at theta={th} outside the cone"
            );
        }
    }
}
