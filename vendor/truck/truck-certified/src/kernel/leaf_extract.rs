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

//! Knot-span extraction of homogeneous Bézier leaves from landed B-spline /
//! NURBS surfaces, plus the two direct leaf constructors (BG-KV2-102-LEAF).
//!
//! **H-1.** The crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers
//! this module. The module carries no `unwrap`, no `expect`, and no `panic!`
//! calls, and adds no module-level `allow`.
//!
//! **Reuse, never restate.** The leaf shape is the shim
//! [`BezierLeaf`](crate::kernel::leaf::BezierLeaf) (homogeneous `xyzw` control
//! net over the unit square). This module produces leaves by standard Bézier
//! extraction (knot insertion to full multiplicity) applied **directly to the
//! landed control net** — no added dependency. The arithmetic is plain `f64`
//! coefficient transport (Boehm knot insertion / de Casteljau), never the
//! certified hull kernels; certification is the `CertifiedPatch` implementor's
//! job in [`leaf`](crate::kernel::leaf).
//!
//! **Input type (frozen-signature decision).** [`extract_bezier_leaves`] takes
//! a concrete `&NurbsSurface<Vector4>`: the landed homogeneous NURBS carrier
//! whose control points already carry the weight in their fourth coordinate.
//! A non-rational `BSplineSurface<Vector3>` lifts to it through the landed
//! `From` conversion (unit weights), so the single concrete signature covers
//! both families. One input type is exposed — not two.
//!
//! **Clamped precondition.** Bézier extraction is carried out over the knot
//! vector as stored. A non-clamped (open) knot vector has "tail" spans whose
//! end control points are not aligned with full-multiplicity cell blocks, so
//! the extraction refuses a surface whose knot vector is not clamped to its
//! degree (the CAD/STEP norm). This is a structural precondition of this
//! implementation, recorded in the RESULT notes of the packet.
//!
//! **Weight-sign discipline.** Leaves are returned only for strictly positive
//! homogeneous weights ([`RefusalKind::WeightDegenerate`], Disproven, for a
//! non-positive weight — the §7.1 constructor-side gate). The pass-through
//! [`leaf_from_control`] deliberately does NOT enforce positivity: it exists
//! so the fixture kit can represent data whose weight net straddles zero for
//! `weight_bound` classification (§7.4), where the *certificate* decides.

use crate::kernel::evidence::{Construction, Refusal, RefusalEvidence, RefusalKind};
use crate::kernel::leaf::BezierLeaf;
use truck_geometry::nurbs::NurbsSurface;
use truck_geometry::prelude::Vector4;

/// Per knot span (in `u` then `v`) extract one [`BezierLeaf`] that reproduces
/// the surface on that span, with the span affinely mapped onto the leaf's
/// unit-square domain.
///
/// Extraction is the standard Bézier extraction: every interior distinct knot
/// is inserted until it has full multiplicity `degree + 1`, turning the
/// clamped B-spline surface into its piecewise-Bézier form; each nonempty
/// knot-span cell then owns a contiguous `(degree_u + 1) x (degree_v + 1)`
/// block of the refined control net. Leaves are returned `u`-span major,
/// `v`-span minor.
///
/// Refusals (all backing Disproven unless noted): non-finite control data
/// ([`RefusalKind::NonFinite`]); a non-positive control weight
/// ([`RefusalKind::WeightDegenerate`]); a zero degree or a non-clamped knot
/// vector ([`RefusalKind::ClaimRefuted`]).
///
/// The frozen signature takes the concrete homogeneous carrier
/// `&NurbsSurface<Vector4>` (see the module docs); a polynomial B-spline
/// surface is lifted to it through the landed `From<BSplineSurface<Point3>>`
/// conversion first.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
pub fn extract_bezier_leaves(surface: &NurbsSurface<Vector4>) -> Construction<Vec<BezierLeaf>> {
    let bsp = surface.non_rationalized();
    let degree_u = bsp.udegree();
    let degree_v = bsp.vdegree();
    if degree_u == 0 || degree_v == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "spline_zero_degree",
            format!("leaf extraction requires positive degrees, got ({degree_u}, {degree_v})"),
        ));
    }
    let ctrl = bsp.control_points();
    for (i, row) in ctrl.iter().enumerate() {
        for (j, pt) in row.iter().enumerate() {
            if !pt.x.is_finite() || !pt.y.is_finite() || !pt.z.is_finite() || !pt.w.is_finite() {
                return Err(refusal(
                    RefusalKind::NonFinite,
                    "spline_control_not_finite",
                    format!("control point ({i}, {j}) of {pt:?} is not finite"),
                ));
            }
            if pt.w <= 0.0 {
                return Err(refusal(
                    RefusalKind::WeightDegenerate,
                    "spline_control_weight_not_positive",
                    format!(
                        "control point ({i}, {j}) has weight {} which is not > 0",
                        pt.w
                    ),
                ));
            }
        }
    }

    let uknots: Vec<f64> = bsp.uknot_vec().iter().copied().collect();
    let vknots: Vec<f64> = bsp.vknot_vec().iter().copied().collect();
    if !is_clamped(&uknots, degree_u) || !is_clamped(&vknots, degree_v) {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "spline_not_clamped",
            "Bézier extraction requires knot vectors clamped to their degree \
             (the CAD/STEP norm); open knot vectors are refused by this implementation"
                .to_string(),
        ));
    }

    // Control grid, rows over `u`, columns over `v`, homogeneous `[x, y, z, w]`.
    let mut grid: Vec<Vec<[f64; 4]>> = ctrl
        .iter()
        .map(|row| row.iter().map(|pt| [pt.x, pt.y, pt.z, pt.w]).collect())
        .collect();
    let mut uk = uknots;
    let mut vk = vknots;
    refine_axis_u(&mut grid, &mut uk, degree_u)?;
    refine_axis_v(&mut grid, &mut vk, degree_v)?;

    let u_bounds = distinct(&uk);
    let v_bounds = distinct(&vk);
    if u_bounds.len() < 2 || v_bounds.len() < 2 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "spline_no_span",
            "the clamped surface has no nonempty knot span to extract".to_string(),
        ));
    }

    let v_span = degree_v + 1;
    let mut leaves = Vec::with_capacity((u_bounds.len() - 1) * (v_bounds.len() - 1));
    for u_window in u_bounds.windows(2) {
        let u_row0 = first_index_of(&uk, u_window[0]);
        let u_row = match u_row0 {
            Some(r) => r,
            None => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "spline_cell_boundary_lost",
                    "an extraction cell boundary vanished during refinement".to_string(),
                ))
            }
        };
        for v_window in v_bounds.windows(2) {
            let v_col0 = first_index_of(&vk, v_window[0]);
            let v_col = match v_col0 {
                Some(c) => c,
                None => {
                    return Err(refusal(
                        RefusalKind::ClaimRefuted,
                        "spline_cell_boundary_lost",
                        "an extraction cell boundary vanished during refinement".to_string(),
                    ))
                }
            };
            let mut control = Vec::with_capacity((degree_u + 1) * (degree_v + 1));
            for row in grid.iter().skip(u_row).take(degree_u + 1) {
                control.extend_from_slice(&row[v_col..v_col + v_span]);
            }
            // Weights are positive and data finite by the up-front validation;
            // the refined net is positive convex combinations of them.
            leaves.push(BezierLeaf::try_new(degree_u, degree_v, control)?);
        }
    }
    Ok(leaves)
}

/// A direct, structurally-validated leaf constructor for clients that already
/// hold a homogeneous Bézier control net (`xyzw`, row-major over `(u, v)`).
///
/// Unlike [`BezierLeaf::try_new`], this pass-through does NOT enforce strictly
/// positive control weights: it exists so the fixture kit can represent nets
/// whose weight field straddles zero and let the `weight_bound` certificate
/// (§7.1) classify them. It refuses a zero degree (ClaimRefuted), a control
/// count that does not match the degrees (ClaimRefuted), and any non-finite
/// coordinate ([`RefusalKind::NonFinite`]).
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
pub fn leaf_from_control(
    degree_u: usize,
    degree_v: usize,
    control: Vec<[f64; 4]>,
) -> Construction<BezierLeaf> {
    if degree_u == 0 || degree_v == 0 {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_zero_degree",
            format!("leaf degrees ({degree_u}, {degree_v}) must be positive"),
        ));
    }
    let expected = (degree_u + 1) * (degree_v + 1);
    if control.len() != expected {
        return Err(refusal(
            RefusalKind::ClaimRefuted,
            "bezier_control_count_mismatch",
            format!(
                "control net has {} points, degrees ({degree_u}, {degree_v}) require {expected}",
                control.len()
            ),
        ));
    }
    for (i, p) in control.iter().enumerate() {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(refusal(
                RefusalKind::NonFinite,
                "bezier_coordinate_not_finite",
                format!("control point {i} has a non-finite coordinate: {p:?}"),
            ));
        }
    }
    Ok(BezierLeaf {
        degree_u,
        degree_v,
        control,
    })
}

/// The affine reparameterization primitive of §4.2 Rule B.
///
/// `affine` is `[[scale_u, shift_u], [scale_v, shift_v]]`: the returned leaf
/// `g` satisfies `g(s, t) = leaf(scale_u * s + shift_u, scale_v * t + shift_v)`
/// on the unit square, i.e. it is the original leaf transported to (a sub-box
/// of) its own domain. This is how Rule B carries a leaf restriction between
/// affine-related charts. The composition is performed exactly on the control
/// net (blossom / de Casteljau in `f64`); the result keeps the leaf's degrees.
/// Degenerate transports are legal data — the result is a leaf whose image is
/// collapsed along the degenerate axis — and are validated structurally by
/// [`leaf_from_control`], never certified here.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
pub fn reparam(leaf: &BezierLeaf, affine: [[f64; 2]; 2]) -> Construction<BezierLeaf> {
    let [su, bu] = affine[0];
    let [sv, bv] = affine[1];
    let (du, dv) = (leaf.degree_u, leaf.degree_v);
    let width = dv + 1;
    let grid: Vec<Vec<[f64; 4]>> = (0..=du)
        .map(|i| leaf.control[i * width..i * width + width].to_vec())
        .collect();
    // Compose the u-axis map: each v-column is a degree-duu curve in u.
    let columns: Vec<Vec<[f64; 4]>> = (0..=dv)
        .map(|j| (0..=du).map(|i| grid[i][j]).collect())
        .collect();
    let mut u_transport: Vec<Vec<[f64; 4]>> = Vec::with_capacity(dv + 1);
    for column in &columns {
        u_transport.push(reparam_curve_1d(column, su, bu));
    }
    // Compose the v-axis map: each u-row is a degree-dv curve in v.
    let transported: Vec<Vec<[f64; 4]>> = (0..=du)
        .map(|i| {
            let row: Vec<[f64; 4]> = (0..=dv).map(|j| u_transport[j][i]).collect();
            reparam_curve_1d(&row, sv, bv)
        })
        .collect();
    let control = transported.into_iter().flatten().collect();
    leaf_from_control(du, dv, control)
}

/// Insert every interior distinct knot of a clamped knot vector until it has
/// multiplicity `degree + 1`, refining the control grid's rows (the `u` axis)
/// in place. The knots vector is updated in place to stay in the
/// `rows = knots - degree - 1` correspondence.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn refine_axis_u(
    grid: &mut Vec<Vec<[f64; 4]>>,
    knots: &mut Vec<f64>,
    degree: usize,
) -> Construction<()> {
    for value in distinct(knots).windows(2) {
        let x = value[1];
        if knots.first() == Some(&x) || knots.last() == Some(&x) {
            continue;
        }
        let mut needed = (degree + 1).saturating_sub(count_equal(knots, x));
        while needed > 0 {
            insert_u_knot(grid, knots, degree, x)?;
            needed -= 1;
        }
    }
    Ok(())
}

/// Insert every interior distinct knot of a clamped knot vector until it has
/// multiplicity `degree + 1`, refining the control grid's columns (the `v`
/// axis) in place.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn refine_axis_v(
    grid: &mut Vec<Vec<[f64; 4]>>,
    knots: &mut Vec<f64>,
    degree: usize,
) -> Construction<()> {
    for value in distinct(knots).windows(2) {
        let x = value[1];
        if knots.first() == Some(&x) || knots.last() == Some(&x) {
            continue;
        }
        let mut needed = (degree + 1).saturating_sub(count_equal(knots, x));
        while needed > 0 {
            insert_v_knot(grid, knots, degree, x)?;
            needed -= 1;
        }
    }
    Ok(())
}

/// One Boehm knot insertion along `u` on every column of the control grid.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn insert_u_knot(
    grid: &mut Vec<Vec<[f64; 4]>>,
    knots: &mut Vec<f64>,
    degree: usize,
    x: f64,
) -> Construction<()> {
    let row_count = grid.len();
    let column_count = match grid.first() {
        Some(row) => row.len(),
        None => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "spline_empty_control_net",
                "cannot refine an empty control net".to_string(),
            ))
        }
    };
    let columns: Vec<Vec<[f64; 4]>> = (0..column_count)
        .map(|j| (0..row_count).map(|i| grid[i][j]).collect())
        .collect();
    let mut refined: Vec<Vec<[f64; 4]>> = Vec::with_capacity(column_count);
    for column in &columns {
        match insert_boehm(knots, degree, column, x) {
            Some(new_column) => refined.push(new_column),
            None => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "spline_knot_insertion_failed",
                    format!("Boehm knot insertion at {x} failed along u"),
                ))
            }
        }
    }
    let new_rows = match refined.first() {
        Some(row) => row.len(),
        None => {
            return Err(refusal(
                RefusalKind::ClaimRefuted,
                "spline_no_columns",
                "".to_string(),
            ))
        }
    };
    let next: Vec<Vec<[f64; 4]>> = (0..new_rows)
        .map(|i| (0..column_count).map(|j| refined[j][i]).collect())
        .collect();
    *grid = next;
    let k = insert_knot_index(knots, x);
    knots.insert(k + 1, x);
    Ok(())
}

/// One Boehm knot insertion along `v` on every row of the control grid.
#[allow(clippy::result_large_err)] // Refusal carries Option<PartialGraph> by frozen §2 shape (BG-KV2-000).
fn insert_v_knot(
    grid: &mut Vec<Vec<[f64; 4]>>,
    knots: &mut Vec<f64>,
    degree: usize,
    x: f64,
) -> Construction<()> {
    let mut refined: Vec<Vec<[f64; 4]>> = Vec::with_capacity(grid.len());
    for row in grid.iter() {
        match insert_boehm(knots, degree, row, x) {
            Some(new_row) => refined.push(new_row),
            None => {
                return Err(refusal(
                    RefusalKind::ClaimRefuted,
                    "spline_knot_insertion_failed",
                    format!("Boehm knot insertion at {x} failed along v"),
                ))
            }
        }
    }
    *grid = refined;
    let k = insert_knot_index(knots, x);
    knots.insert(k + 1, x);
    Ok(())
}

/// Boehm knot insertion of the single value `x` into a degree-`degree` B-spline
/// control sequence. Returns the updated control points (one longer); the
/// caller appends `x` to the knot vector at `insert_knot_index + 1`.
fn insert_boehm(knots: &[f64], degree: usize, pts: &[[f64; 4]], x: f64) -> Option<Vec<[f64; 4]>> {
    let n = pts.len();
    if n == 0 || knots.len() != n + degree + 1 {
        return None;
    }
    let k = insert_knot_index(knots, x);
    if k + 1 >= knots.len() {
        // Inserting a knot with no greater knot to its right is out of scope
        // for clamped extraction (end clamps already have full multiplicity).
        return None;
    }
    if k + 1 > n {
        return None;
    }
    let band_lo = if k >= degree { k - degree + 1 } else { 1 };
    let mut out = Vec::with_capacity(n + 1);
    out.extend_from_slice(&pts[..band_lo]);
    for i in band_lo..=k.min(n - 1) {
        let den = knots[i + degree] - knots[i];
        let alpha = if den == 0.0 {
            0.0
        } else {
            (x - knots[i]) / den
        };
        out.push(lerp4(pts[i - 1], pts[i], alpha));
    }
    for i in (k + 1)..=n {
        out.push(pts[i - 1]);
    }
    if out.len() == n + 1 {
        Some(out)
    } else {
        None
    }
}

/// The knot index to insert `x` after: the largest `i` with `knots[i] <= x`.
fn insert_knot_index(knots: &[f64], x: f64) -> usize {
    let mut k = 0usize;
    for (i, &u) in knots.iter().enumerate() {
        if u <= x {
            k = i;
        }
    }
    k
}

/// The distinct ascending values of a sorted slice.
fn distinct(knots: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    for &k in knots {
        if out.last() != Some(&k) {
            out.push(k);
        }
    }
    out
}

/// Count the exact occurrences of `value` in the sorted slice.
fn count_equal(knots: &[f64], value: f64) -> usize {
    knots.iter().filter(|&&k| k == value).count()
}

/// The first index at which `value` occurs, if present.
fn first_index_of(knots: &[f64], value: f64) -> Option<usize> {
    knots.iter().position(|&k| k == value)
}

/// A knot vector is clamped to its degree when its first `degree + 1` knots
/// coincide and its last `degree + 1` knots coincide.
fn is_clamped(knots: &[f64], degree: usize) -> bool {
    let n = knots.len();
    if n < 2 * (degree + 1) {
        return false;
    }
    let first = knots[0];
    let last = knots[n - 1];
    knots[0..=degree].iter().all(|&k| k == first)
        && knots[n - 1 - degree..n].iter().all(|&k| k == last)
}

/// Convex interpolation of two homogeneous control points.
fn lerp4(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [
        s * a[0] + t * b[0],
        s * a[1] + t * b[1],
        s * a[2] + t * b[2],
        s * a[3] + t * b[3],
    ]
}

/// Compose a Bézier control sequence with the affine map `u = scale * x + shift`
/// in `x`. Returns the degree-preserving control sequence of `c(scale * x + shift)`.
fn reparam_curve_1d(pts: &[[f64; 4]], scale: f64, shift: f64) -> Vec<[f64; 4]> {
    let degree = pts.len() - 1;
    let a = shift;
    let b = shift + scale;
    let mut out = Vec::with_capacity(degree + 1);
    for k in 0..=degree {
        let mut args = Vec::with_capacity(degree);
        for _ in 0..(degree - k) {
            args.push(a);
        }
        for _ in 0..k {
            args.push(b);
        }
        out.push(blossom(pts, &args));
    }
    out
}

/// The polar (blossom) form of a degree-`args.len()` Bézier curve evaluated at
/// the given parameter arguments. `pts.len() - 1 == args.len()` is required.
fn blossom(pts: &[[f64; 4]], args: &[f64]) -> [f64; 4] {
    let mut level: Vec<[f64; 4]> = pts.to_vec();
    for &u in args {
        let mut next = Vec::with_capacity(level.len() - 1);
        for w in level.windows(2) {
            next.push(lerp4(w[0], w[1], u));
        }
        level = next;
    }
    level[0]
}

/// A named predicate refusal.
fn refusal(kind: RefusalKind, name: &'static str, detail: String) -> Refusal {
    Refusal::new(kind, RefusalEvidence::Predicate { name, detail })
}
