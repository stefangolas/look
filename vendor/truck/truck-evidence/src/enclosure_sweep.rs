//! CL-003-SWEEP-ENCLOSURE: `EnclosureSurface` for the closed whole-sweep value
//! (`truck_geometry::constructive::SpineFrameSweep`).
//!
//! The sweep realizes `X(s, v) = C(s) + N(s)·Px(s, v) + B(s)·Py(s, v)` over the
//! windowed domain `[s0, s1] × [v0, v1]` (`N`, `B` the frame normal/binormal at
//! `s`, `P` the profile law's point in the frame plane). Its certified
//! enclosures are a **composition** — never a sample-and-hull of the sweep —
//! following the landed evaluators' own derivative structure
//! (`decorators/spine_frame.rs`):
//!
//! - `X_v = N(s)·∂P/∂v + B(s)·∂P/∂v`-image (`∂P/∂v` is the profile law's
//!   per-edge constant),
//! - `X_s = C′(s) + N(s)·∂P/∂s + B(s)·∂P/∂s` (the frame-twist term is outside
//!   the landed evaluators by design, so it is outside this certificate too),
//!
//! and every term is bounded **per piece**: the spine position/derivative from
//! the landed curve enclosures (`EnclosureCurve` on the canonical `Curve`
//! payload), the profile arithmetic (edge endpoints under the law, the ring
//! interpolation fraction, and the law's `s`-derivative) in outward-rounded
//! interval arithmetic over the window, and the frame axes by their validated
//! unit length — a landed `Frame3` is unit within
//! `DirectTolerance::default().position`, so each frame coordinate lies in
//! `[-(1 + τ), 1 + τ]`, `τ` that tolerance. No frame-law branch is needed: the
//! bound uses only the unit-vector property every landed frame satisfies. The
//! stored sweep-level placement is applied to the raw boxes afterwards in
//! interval arithmetic (the `Processor` precedent).
//!
//! Soundness is outward by construction (BG-ENC-001): every piece bound is
//! interval-isotonic in its window, so the composition converges under
//! bisection and is **additive** — splitting the window yields sub-enclosures
//! contained in the parent's (the solver's subdivision premise).
//!
//! **Envelope limits.** The composition certifies the windowed first partials
//! and the position only. A `vv` that is not contained in one profile-ring edge
//! (crossing a ring vertex, or outside `[0, 1]`), or a spine variant with no
//! landed curve enclosure, is answered with the whole-space box — sound, never
//! an under-estimate. Higher partials (`m + n ≥ 2`) are the honest
//! whole-space box: the sweep's own `der_mn` reaches them by central
//! differences, which the composition does not certify. At a window whose
//! right ring endpoint is a vertex, `∂P/∂v` is right-continuous by the landed
//! evaluator, so the derivative box unions the adjacent edge's value there.

use crate::enclosure::{
    cross_box, immersion_lower_bound_box, interval_at, midpoint_ball_cone, Box3, DirCone,
    EnclosureCurve, EnclosureSurface,
};
use inari::Interval;
use truck_base::cgmath64::{Matrix4, Point2};
use truck_base::tolerance::TOLERANCE;
use truck_geometry::canonical::Curve;
use truck_geometry::constructive::{ProfileLaw, ScalarLaw, SpineFrameSweep};

/// The bound on every validated frame coordinate: a landed `Frame3` is unit to
/// within `DirectTolerance::default().position` (= `TOLERANCE`), so each
/// coordinate of `N`/`B` lies in `[-(1 + τ), 1 + τ]` with `τ` that tolerance.
/// `N_c·a + B_c·b ⊆ [-(1+τ), 1+τ]·a + [-(1+τ), 1+τ]·b` for any profile
/// scalars `a`, `b` — the coordinate-wise frame bound used throughout.
fn frame_axis_interval() -> Interval {
    let bound = 1.0 + TOLERANCE;
    Interval::try_from((-bound, bound)).unwrap_or(Interval::ENTIRE)
}

/// The whole-space box: the honest answer when a box reaches outside the
/// certified envelope (out-of-edge `vv`, a spine variant with no enclosure, or
/// an uncertified higher partial).
fn whole_space_box() -> Box3 {
    Box3 {
        x: Interval::ENTIRE,
        y: Interval::ENTIRE,
        z: Interval::ENTIRE,
    }
}

/// Whether the parameter box is empty or has non-finite bounds: nothing to
/// certify (the BSplineSurface precedent returns the empty box there).
fn degenerate_window(ss: Interval, vv: Interval) -> bool {
    ss.is_empty()
        || vv.is_empty()
        || !ss.inf().is_finite()
        || !ss.sup().is_finite()
        || !vv.inf().is_finite()
        || !vv.sup().is_finite()
}

/// The composite bounds of a sweep over a window, *before* the sweep-level
/// placement: the position box and the two first-partial boxes.
#[derive(Clone, Copy, Debug)]
struct RawSweepBoxes {
    /// An enclosure of `{ X(s, v) : s ∈ ss, v ∈ vv }`.
    pos: Box3,
    /// An enclosure of `{ X_s(s, v) : s ∈ ss, v ∈ vv }` (the landed `uder`).
    der_s: Box3,
    /// An enclosure of `{ X_v(s, v) : s ∈ ss, v ∈ vv }` (the landed `vder`).
    der_v: Box3,
}

/// A pair of profile-plane intervals (one per ring coordinate).
#[derive(Clone, Copy, Debug)]
struct Iv2 {
    /// The profile-x (frame normal) coordinate interval.
    x: Interval,
    /// The profile-y (frame binormal) coordinate interval.
    y: Interval,
}

/// A degenerate `Iv2` from a 2-D point.
fn point_iv(p: Point2) -> Iv2 {
    Iv2 {
        x: interval_at(p.x),
        y: interval_at(p.y),
    }
}

/// The zero `Iv2`.
fn zero_iv() -> Iv2 {
    Iv2 {
        x: interval_at(0.0),
        y: interval_at(0.0),
    }
}

/// Componentwise interval sum of two `Iv2`.
fn add_iv(a: &Iv2, b: &Iv2) -> Iv2 {
    Iv2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

/// Componentwise interval difference of two `Iv2`.
fn sub_iv(a: &Iv2, b: &Iv2) -> Iv2 {
    Iv2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

/// Componentwise interval product of an `Iv2` and a scalar interval.
fn scale_iv(a: &Iv2, s: Interval) -> Iv2 {
    Iv2 {
        x: a.x * s,
        y: a.y * s,
    }
}

/// The ring vertex count of the profile law.
fn ring_size(law: &ProfileLaw) -> Option<usize> {
    let k = match law {
        ProfileLaw::Constant(profile) => profile.vertices.len(),
        ProfileLaw::Scale { profile, .. } => profile.vertices.len(),
        ProfileLaw::LinearCorrespondence { start, .. } => start.vertices.len(),
    };
    if k >= 3 {
        Some(k)
    } else {
        None
    }
}

/// The scalar law and its explicit `s`-derivative over `ss`, as intervals. A
/// `Linear` law is `start + (end − start)·s` (linear extrapolation outside
/// `[0, 1]`), so the range over `ss` is the exact interval extension.
fn scalar_bounds(law: &ScalarLaw, ss: Interval) -> (Interval, Interval) {
    match *law {
        ScalarLaw::Constant(c) => (interval_at(c), interval_at(0.0)),
        ScalarLaw::Linear { start, end } => {
            let slope = end - start;
            (
                interval_at(start) + interval_at(slope) * ss,
                interval_at(slope),
            )
        }
    }
}

/// The profile ring edge's frame-plane data over `s ∈ ss` for ring edge `e`
/// (running vertex `e` → vertex `(e + 1) % k` under the law at station `s`):
/// the edge start `A(s)`, the edge end `B(s)`, and their explicit
/// `s`-derivatives `A′(s)`, `B′(s)`. Every law's realized edge point is
/// piecewise-affine in the station (constant, scaled, or vertex-wise lerped),
/// so each quantity is an affine interval expression over `ss` — exact up to
/// outward rounding.
fn edge_intervals(
    law: &ProfileLaw,
    e: usize,
    k: usize,
    ss: Interval,
) -> Option<(Iv2, Iv2, Iv2, Iv2)> {
    match law {
        ProfileLaw::Constant(profile) => {
            let a = *profile.vertices.get(e)?;
            let b = *profile.vertices.get((e + 1) % k)?;
            Some((point_iv(a), point_iv(b), zero_iv(), zero_iv()))
        }
        ProfileLaw::Scale { profile, scale } => {
            let a0 = *profile.vertices.get(e)?;
            let b0 = *profile.vertices.get((e + 1) % k)?;
            let (c, dc) = scalar_bounds(scale, ss);
            Some((
                scale_iv(&point_iv(a0), c),
                scale_iv(&point_iv(b0), c),
                scale_iv(&point_iv(a0), dc),
                scale_iv(&point_iv(b0), dc),
            ))
        }
        ProfileLaw::LinearCorrespondence { start, end } => {
            let sa = *start.vertices.get(e)?;
            let ea = *end.vertices.get(e)?;
            let sb = *start.vertices.get((e + 1) % k)?;
            let eb = *end.vertices.get((e + 1) % k)?;
            let lerp = |u: Point2, w: Point2| Iv2 {
                x: interval_at(u.x) + interval_at(w.x - u.x) * ss,
                y: interval_at(u.y) + interval_at(w.y - u.y) * ss,
            };
            let a = lerp(sa, ea);
            let b = lerp(sb, eb);
            let ad = Iv2 {
                x: interval_at(ea.x - sa.x),
                y: interval_at(ea.y - sa.y),
            };
            let bd = Iv2 {
                x: interval_at(eb.x - sb.x),
                y: interval_at(eb.y - sb.y),
            };
            Some((a, b, ad, bd))
        }
    }
}

/// The frame-plane profile bounds over the window `ss × fv` on ring edge `e`:
/// the profile point `P`, its `v`-derivative `∂P/∂v` (the per-edge constant
/// `k·(B − A)`), and its explicit `s`-derivative `∂P/∂s`. All interval
/// arithmetic is outward.
struct ProfileBounds {
    /// `P` over the window.
    p: Iv2,
    /// `∂P/∂v` over the window.
    dv: Iv2,
    /// `∂P/∂s` over the window.
    ds: Iv2,
}

/// The profile bounds for ring edge `e` with ring fraction range `fv ⊂ [0, 1]`
/// over the station interval `ss`.
fn profile_bounds(
    law: &ProfileLaw,
    e: usize,
    k: usize,
    fv: Interval,
    ss: Interval,
) -> Option<ProfileBounds> {
    let (a, b, ap, bp) = edge_intervals(law, e, k, ss)?;
    let db = sub_iv(&b, &a);
    let p = add_iv(&a, &scale_iv(&db, fv));
    let ddp = sub_iv(&bp, &ap);
    let ds = add_iv(&ap, &scale_iv(&ddp, fv));
    let dv = scale_iv(&db, interval_at(k as f64));
    Some(ProfileBounds { p, dv, ds })
}

/// The window's ring edge index and fraction range. `None` when the box is not
/// contained in a single profile edge (it crosses a ring vertex, or leaves
/// `[0, 1]`): the certified envelope's per-edge formulas do not apply there and
/// the whole-space box answers instead.
fn edge_window(law: &ProfileLaw, vv: Interval) -> Option<(usize, Interval)> {
    let k = ring_size(law)?;
    let v_lo = vv.inf();
    let v_hi = vv.sup();
    if !(v_lo >= 0.0 && v_hi <= 1.0) {
        return None;
    }
    let kf = k as f64;
    let x0 = v_lo * kf;
    let x1 = v_hi * kf;
    // The ring-close station `v = 1` lands on the closing edge `k − 1` with
    // fraction 1; any other station's edge is `floor(v·k)`.
    let edge = if x0.floor() >= kf {
        k - 1
    } else {
        x0.floor() as usize
    };
    if edge >= k {
        return None;
    }
    let f0 = x0 - edge as f64;
    let f1 = x1 - edge as f64;
    if !(f0 >= 0.0 && f0 <= f1 && f1 <= 1.0) {
        return None;
    }
    let fv = Interval::try_from((f0, f1)).unwrap_or(Interval::EMPTY);
    if fv.is_empty() {
        return None;
    }
    Some((edge, fv))
}

/// Assembles the raw (unplaced) sweep boxes from the spine boxes and the
/// profile bounds. Each output coordinate is `unit-factor·a + unit-factor·b`
/// over the two profile coordinates, where the unit factor is the validated
/// frame-coordinate interval.
fn assemble_boxes(c: Box3, cd: Box3, pb: &ProfileBounds) -> RawSweepBoxes {
    let axis = frame_axis_interval();
    let pos = Box3 {
        x: c.x + axis * pb.p.x + axis * pb.p.y,
        y: c.y + axis * pb.p.x + axis * pb.p.y,
        z: c.z + axis * pb.p.x + axis * pb.p.y,
    };
    let der_s = Box3 {
        x: cd.x + axis * pb.ds.x + axis * pb.ds.y,
        y: cd.y + axis * pb.ds.x + axis * pb.ds.y,
        z: cd.z + axis * pb.ds.x + axis * pb.ds.y,
    };
    let der_v = Box3 {
        x: axis * pb.dv.x + axis * pb.dv.y,
        y: axis * pb.dv.x + axis * pb.dv.y,
        z: axis * pb.dv.x + axis * pb.dv.y,
    };
    RawSweepBoxes { pos, der_s, der_v }
}

/// Per-coordinate interval hull (convex hull) of two boxes.
fn hull_boxes(a: &Box3, b: &Box3) -> Box3 {
    Box3 {
        x: a.x.convex_hull(b.x),
        y: a.y.convex_hull(b.y),
        z: a.z.convex_hull(b.z),
    }
}

/// The spine position enclosure over `ss`. `None` for a canonical spine
/// variant with no landed curve enclosure (the whole-space box answers).
fn spine_position_box(spine: &Curve, ss: Interval) -> Option<Box3> {
    match spine {
        Curve::Line(line) => Some(EnclosureCurve::enclose(line, ss)),
        Curve::BSplineCurve(bsp) => Some(EnclosureCurve::enclose(bsp, ss)),
        _ => None,
    }
}

/// The spine derivative enclosure over `ss` (the first partial of `C`). `None`
/// for a variant with no landed curve enclosure.
fn spine_derivative_box(spine: &Curve, ss: Interval) -> Option<Box3> {
    match spine {
        Curve::Line(line) => Some(EnclosureCurve::enclose_der(line, 1, ss)),
        Curve::BSplineCurve(bsp) => Some(EnclosureCurve::enclose_der(bsp, 1, ss)),
        _ => None,
    }
}

/// The raw (identity-placement) sweep boxes over the window. `None` when the
/// window or a piece lies outside the certified envelope (an out-of-edge `vv`,
/// or a spine with no enclosure) — the whole-space box answers.
fn raw_sweep_boxes(sweep: &SpineFrameSweep, ss: Interval, vv: Interval) -> Option<RawSweepBoxes> {
    let recipe = sweep.recipe();
    let c = spine_position_box(&recipe.spine, ss)?;
    let cd = spine_derivative_box(&recipe.spine, ss)?;
    let law = &recipe.profile_law;
    let k = ring_size(law)?;
    let (edge, fv) = edge_window(law, vv)?;
    let pb = profile_bounds(law, edge, k, fv, ss)?;
    let mut raw = assemble_boxes(c, cd, &pb);
    // A window whose right ring endpoint is an interior vertex includes the
    // vertex's landed `vder`, which the evaluator takes on the NEXT edge (the
    // `floor(v·k)` convention). Union the adjacent edge's value at fraction 0.
    if fv.sup() == 1.0 && edge + 1 < k {
        let nf = Interval::try_from((0.0, 0.0)).unwrap_or(Interval::EMPTY);
        if let Some(pbn) = profile_bounds(law, edge + 1, k, nf, ss) {
            let nraw = assemble_boxes(c, cd, &pbn);
            raw.der_v = hull_boxes(&raw.der_v, &nraw.der_v);
        }
    }
    Some(raw)
}

/// Maps a raw position box through the stored placement in interval
/// arithmetic: each output row is `Σ_j m_{ij}·b_j` with the fourth homogeneous
/// coordinate the degenerate interval at 1.0, divided by the transformed `w`
/// row (the `Processor` `enclose` construction).
fn placed_position_box(transform: &Matrix4, raw: &RawSweepBoxes) -> Box3 {
    let t = *transform;
    let b = &raw.pos;
    let nx = interval_at(t.x.x) * b.x
        + interval_at(t.y.x) * b.y
        + interval_at(t.z.x) * b.z
        + interval_at(t.w.x);
    let ny = interval_at(t.x.y) * b.x
        + interval_at(t.y.y) * b.y
        + interval_at(t.z.y) * b.z
        + interval_at(t.w.y);
    let nz = interval_at(t.x.z) * b.x
        + interval_at(t.y.z) * b.y
        + interval_at(t.z.z) * b.z
        + interval_at(t.w.z);
    let w = interval_at(t.x.w) * b.x
        + interval_at(t.y.w) * b.y
        + interval_at(t.z.w) * b.z
        + interval_at(t.w.w);
    if w.contains(0.0) || w.is_empty() {
        whole_space_box()
    } else {
        Box3 {
            x: nx / w,
            y: ny / w,
            z: nz / w,
        }
    }
}

/// Maps a raw derivative box through the linear part of the stored placement
/// (mirroring `transform_vector`: no `w` column, no `w` divide).
fn placed_vector_box(transform: &Matrix4, d: &Box3) -> Box3 {
    let t = *transform;
    Box3 {
        x: interval_at(t.x.x) * d.x + interval_at(t.y.x) * d.y + interval_at(t.z.x) * d.z,
        y: interval_at(t.x.y) * d.x + interval_at(t.y.y) * d.y + interval_at(t.z.y) * d.z,
        z: interval_at(t.x.z) * d.x + interval_at(t.y.z) * d.y + interval_at(t.z.z) * d.z,
    }
}

/// Whether every coordinate interval of a box is finite (a finite bound; the
/// σ_G helper's boundedness assertion).
pub(crate) fn box_is_finite(b: &Box3) -> bool {
    b.x.inf().is_finite()
        && b.x.sup().is_finite()
        && b.y.inf().is_finite()
        && b.y.sup().is_finite()
        && b.z.inf().is_finite()
        && b.z.sup().is_finite()
}

impl EnclosureSurface for SpineFrameSweep {
    fn enclose(&self, ss: Interval, vv: Interval) -> Box3 {
        if degenerate_window(ss, vv) {
            return Box3::empty();
        }
        match raw_sweep_boxes(self, ss, vv) {
            Some(raw) => placed_position_box(self.transform(), &raw),
            None => whole_space_box(),
        }
    }

    fn enclose_der(&self, m: usize, n: usize, ss: Interval, vv: Interval) -> Box3 {
        if degenerate_window(ss, vv) {
            return Box3::empty();
        }
        if m == 0 && n == 0 {
            // `der_mn(0, 0)` returns `subs(u, v).to_vec()`: the zeroth
            // enclosure is the position.
            return self.enclose(ss, vv);
        }
        if m + n >= 2 {
            // The sweep's own higher partials are central differences of the
            // first partials; the composition does not certify them, and the
            // whole-space box is the honest answer (the ISC/PCURVE precedent).
            return whole_space_box();
        }
        let raw = match raw_sweep_boxes(self, ss, vv) {
            Some(raw) => raw,
            None => return whole_space_box(),
        };
        if m == 1 {
            placed_vector_box(self.transform(), &raw.der_s)
        } else {
            placed_vector_box(self.transform(), &raw.der_v)
        }
    }

    fn normal_cone(&self, uu: Interval, vv: Interval) -> Option<DirCone> {
        let a = self.enclose_der(1, 0, uu, vv);
        let b = self.enclose_der(0, 1, uu, vv);
        midpoint_ball_cone(&cross_box(&a, &b))
    }

    fn immersion_lower_bound(&self, uu: Interval, vv: Interval) -> f64 {
        let a = self.enclose_der(1, 0, uu, vv);
        let b = self.enclose_der(0, 1, uu, vv);
        immersion_lower_bound_box(&cross_box(&a, &b))
    }
}

/// The required-test sweep fixtures, shared by the two CL-003 test modules
/// (the `EnclosureSurface` tests here and the σ_G tests in `num/sweep_sigma`).
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use truck_base::cgmath64::{Point3, Vector3};
    use truck_geometry::constructive::{FrameLaw, Profile2D, SpineFrameRecipe};
    use truck_geometry::nurbs::{BSplineCurve, KnotVec};
    use truck_geometry::specifieds::Line;

    /// The unit-square profile ring (CCW in the frame plane).
    pub(crate) fn unit_square_profile() -> Option<Profile2D> {
        Profile2D::try_closed(vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ])
        .ok()
    }

    /// The straight-spine prism sweep: a `Line` spine from the origin to
    /// `(0, 0, 1)` with the constant unit-square profile, pinned to the `+x`
    /// plane, over the edge-zero window.
    pub(crate) fn straight_spine_sweep() -> Option<SpineFrameSweep> {
        let profile = unit_square_profile()?;
        let spine = Box::new(Curve::Line(Line(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Constant(profile),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).ok()
    }

    /// The curved-spine sweep: a quadratic B-spline spine lying in the plane
    /// `x = 0` (so the `+x`-pinned `FixedPlane` frame stays valid), constant
    /// unit-square profile, edge-zero window.
    pub(crate) fn curved_spine_sweep() -> Option<SpineFrameSweep> {
        let profile = unit_square_profile()?;
        let knot = KnotVec::uniform_knot(2, 1);
        let ctrl = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 1.0),
            Point3::new(0.0, 0.0, 2.0),
        ];
        let spine = Box::new(Curve::BSplineCurve(BSplineCurve::new(knot, ctrl)));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Constant(profile),
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).ok()
    }

    /// The scaled-profile sweep: the straight `Line` spine with the unit-square
    /// profile under a `Scale` law whose scalar runs linearly from `1` to `1/2`
    /// over the spine domain, edge-zero window.
    pub(crate) fn scaled_profile_sweep() -> Option<SpineFrameSweep> {
        let profile = unit_square_profile()?;
        let spine = Box::new(Curve::Line(Line(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )));
        let recipe = SpineFrameRecipe::new(
            spine,
            ProfileLaw::Scale {
                profile,
                scale: ScalarLaw::Linear {
                    start: 1.0,
                    end: 0.5,
                },
            },
            FrameLaw::FixedPlane {
                normal: Vector3::unit_x(),
            },
        );
        SpineFrameSweep::try_new(recipe, 0.0, 1.0, 0.0, 0.25).ok()
    }

    /// The three required-test sweeps: straight spine, curved spine, and
    /// scaled profile.
    pub(crate) fn required_sweeps() -> Vec<SpineFrameSweep> {
        let mut out = Vec::new();
        if let Some(s) = straight_spine_sweep() {
            out.push(s);
        }
        if let Some(s) = curved_spine_sweep() {
            out.push(s);
        }
        if let Some(s) = scaled_profile_sweep() {
            out.push(s);
        }
        out
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Rad, Vector3};
    use truck_geotrait::{ParametricSurface, Transformed};

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// An interior sample `k/(n + 1)` of the `n`-sample lattice over `[lo, hi]`
    /// (`k = 1..=n`): samples never touch the window boundary, so a ring-vertex
    /// or knot endpoint is never sampled.
    fn interior_sample(lo: f64, hi: f64, k: usize, n: usize) -> f64 {
        lo + (hi - lo) * ((k + 1) as f64) / ((n + 1) as f64)
    }

    /// Asserts the componentwise containment of two boxes (child ⊆ parent).
    fn assert_box_subset(child: &Box3, parent: &Box3, what: &str) {
        assert!(
            parent.x.inf() <= child.x.inf() && child.x.sup() <= parent.x.sup(),
            "{what}: x child {:?} escapes parent {:?}",
            child.x,
            parent.x
        );
        assert!(
            parent.y.inf() <= child.y.inf() && child.y.sup() <= parent.y.sup(),
            "{what}: y child {:?} escapes parent {:?}",
            child.y,
            parent.y
        );
        assert!(
            parent.z.inf() <= child.z.inf() && child.z.sup() <= parent.z.sup(),
            "{what}: z child {:?} escapes parent {:?}",
            child.z,
            parent.z
        );
    }

    /// CL-003 required test 1: for at least three sweeps (straight spine,
    /// curved spine, scaled profile), the gradient enclosures of the first
    /// partials bracket at least 1000 brute `der_mn` samples per component over
    /// the window.
    #[test]
    fn sweep_enclosure_brackets_brute_sample() {
        let sweeps = required_sweeps();
        assert!(
            sweeps.len() >= 3,
            "the required fixture set must hold three sweeps"
        );
        const GRID: usize = 40; // 1600 samples per component and order
        for sweep in &sweeps {
            let ss = iv(sweep.s0(), sweep.s1());
            let vv = iv(sweep.v0(), sweep.v1());
            let e_s = sweep.enclose_der(1, 0, ss, vv);
            let e_v = sweep.enclose_der(0, 1, ss, vv);
            let mut checked = 0usize;
            for i in 0..GRID {
                for j in 0..GRID {
                    let s = interior_sample(sweep.s0(), sweep.s1(), i, GRID);
                    let v = interior_sample(sweep.v0(), sweep.v1(), j, GRID);
                    let der_s = sweep.der_mn(1, 0, s, v);
                    let der_v = sweep.der_mn(0, 1, s, v);
                    assert!(
                        e_s.contains(Point3::new(der_s.x, der_s.y, der_s.z)),
                        "sweep s-derivative at ({s}, {v}) = {der_s:?} escaped {e_s:?}"
                    );
                    assert!(
                        e_v.contains(Point3::new(der_v.x, der_v.y, der_v.z)),
                        "sweep v-derivative at ({s}, {v}) = {der_v:?} escaped {e_v:?}"
                    );
                    checked += 2;
                }
            }
            assert!(
                checked >= 1000,
                "fewer than 1000 derivative samples bracketed per component"
            );
        }
    }

    /// CL-003 required test 3: bisecting the window and re-enclosing yields
    /// bounds contained in (or equal to) the parent's — the additive property
    /// the solver's subdivision relies on.
    #[test]
    fn sweep_enclosure_is_additive_over_window_split() {
        let sweeps = required_sweeps();
        assert!(
            sweeps.len() >= 3,
            "the required fixture set must hold three sweeps"
        );
        for sweep in &sweeps {
            let ss = iv(sweep.s0(), sweep.s1());
            let vv = iv(sweep.v0(), sweep.v1());
            let s_mid = sweep.s0() + (sweep.s1() - sweep.s0()) / 2.0;
            let v_mid = sweep.v0() + (sweep.v1() - sweep.v0()) / 2.0;
            let parent_encl = sweep.enclose(ss, vv);
            let parent_s = sweep.enclose_der(1, 0, ss, vv);
            let parent_v = sweep.enclose_der(0, 1, ss, vv);
            // Bisect the s-axis and the v-axis independently; every child
            // enclosure must be contained in the parent's.
            for child in [iv(sweep.s0(), s_mid), iv(s_mid, sweep.s1())] {
                assert_box_subset(&sweep.enclose(child, vv), &parent_encl, "s-split position");
                assert_box_subset(
                    &sweep.enclose_der(1, 0, child, vv),
                    &parent_s,
                    "s-split s-derivative",
                );
                assert_box_subset(
                    &sweep.enclose_der(0, 1, child, vv),
                    &parent_v,
                    "s-split v-derivative",
                );
            }
            for child in [iv(sweep.v0(), v_mid), iv(v_mid, sweep.v1())] {
                assert_box_subset(&sweep.enclose(ss, child), &parent_encl, "v-split position");
                assert_box_subset(
                    &sweep.enclose_der(1, 0, ss, child),
                    &parent_s,
                    "v-split s-derivative",
                );
                assert_box_subset(
                    &sweep.enclose_der(0, 1, ss, child),
                    &parent_v,
                    "v-split v-derivative",
                );
            }
        }
    }

    /// The placed sweep follows the raw geometry under the stored placement:
    /// every sampled placed derivative lies inside the placed enclosure (the
    /// identity placements above are covered by the brute-sample test; this
    /// covers a genuinely non-identity placement).
    #[test]
    fn placed_sweep_enclosure_brackets_brute_sample() {
        let mut sweep = straight_spine_sweep().expect("the prism sweep must build");
        let translation = Matrix4::from_translation(Vector3::new(2.0, -3.0, 4.0));
        let rotation = Matrix4::from_axis_angle(Vector3::new(1.0, 1.0, 0.0).normalize(), Rad(0.7));
        sweep.transform_by(rotation * translation);
        let ss = iv(sweep.s0(), sweep.s1());
        let vv = iv(sweep.v0(), sweep.v1());
        let e_s = sweep.enclose_der(1, 0, ss, vv);
        let e_v = sweep.enclose_der(0, 1, ss, vv);
        const GRID: usize = 30;
        for i in 0..GRID {
            for j in 0..GRID {
                let s = interior_sample(sweep.s0(), sweep.s1(), i, GRID);
                let v = interior_sample(sweep.v0(), sweep.v1(), j, GRID);
                let der_s = sweep.der_mn(1, 0, s, v);
                let der_v = sweep.der_mn(0, 1, s, v);
                assert!(
                    e_s.contains(Point3::new(der_s.x, der_s.y, der_s.z)),
                    "placed s-derivative at ({s}, {v}) = {der_s:?} escaped {e_s:?}"
                );
                assert!(
                    e_v.contains(Point3::new(der_v.x, der_v.y, der_v.z)),
                    "placed v-derivative at ({s}, {v}) = {der_v:?} escaped {e_v:?}"
                );
            }
        }
    }
}
