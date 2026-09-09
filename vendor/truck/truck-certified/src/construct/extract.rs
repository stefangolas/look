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

//! The L1 exact Bézier-extraction kernel (ADM-L1-EXTRACT, spec T3′): the
//! production body of the per-knot-rectangle extraction over the landed
//! rational tensor-spline face carrier.
//!
//! ADM-SHIM froze the extracted-patch representation
//! ([`TensorBernsteinPatch`](crate::construct::patches::TensorBernsteinPatch))
//! and declared the L1 kernel REFUSING (`patches::extract_patches` returns
//! [`ConstructRefusal::Unfrozen`] there — the ADM-SHIM required test
//! `refusing_kernels_refuse` pins that refusal). This module lands the lemma
//! body: the L1 production `extract_patches` over the same frozen signature
//! and carrier, behind this packet's admitting conformance tests. The shim's
//! refusing twin is untouched (extend, never edit).
//!
//! # The lemma (what this module proves)
//!
//! Knot insertion to Bézier form is an EXACT local change of basis: on every
//! knot rectangle, the landed rational tensor-spline face equals the extracted
//! patch `Â/Ŵ` identically — no approximation, no sampling. The extraction is
//! the same tensor cut the certified-map layer lands for ordinary
//! `BSplineSurface<Point3>` (D-map, `certified_map.rs`), extended to the
//! HOMOGENEOUS carrier `BSplineSurface<Vector4>`: every row of the control
//! net is decomposed along `u` with the landed
//! [`BSplineCurve::bezier_decomposition`], then every column of each `u`-piece
//! along `v` the same way. Tensor cut operations commute across axes, so the
//! result is exactly the Bézier patch grid, and the homogeneous `Vector4`
//! channels split into the numerator net `Â` (the `x,y,z` entries) and the
//! weight net `Ŵ` (the `w` entries) that the frozen
//! [`TensorBernsteinPatch`](crate::construct::patches::TensorBernsteinPatch)
//! represents. Each span is re-parameterized to `[0, 1]²` (the Bernstein
//! control net of a rectangle is invariant under the affine source-to-unit
//! re-parameterization), so a patch's coefficients are exactly the rectangle's
//! Bézier net. The span's source-domain box (its knot rectangle) and the
//! parent face ordinal are recorded on every patch.
//!
//! **Certificate.** Each extracted span is admitted through
//! [`TensorBernsteinPatch::try_new`], which certifies the weight bracket
//! `[w₋, w₊]` as the Bernstein hull of the weight coefficients and refuses
//! [`ConstructRefusal::InvalidInput`] on any span whose bracket cannot be
//! certified strictly positive — the D-shim constructor rule, never a silent
//! non-positive weight. By the convex-hull property of the non-negative
//! partition-of-unity Bernstein basis, a strictly positive weight NET of an
//! admissible face extracts to strictly positive span nets (knot insertion is
//! convex), so no admissible face is refused; a face whose weight field is not
//! certifiably positive refuses typed.
//!
//! **Degree/range recording.** Each patch carries its tensor-Bernstein
//! bidegree (read off the grid shape by the refusing constructor) and its
//! certified weight bracket — the data ADM-003's Theorem D consumes.
//!
//! # Preconditions and refusals
//!
//! The face must be a clamped positive-weight homogeneous tensor-spline face:
//! a rectangular, finite control net over clamped knot vectors in both axes.
//! An empty, ragged, or non-finite net, an unclamped knot vector, a degenerate
//! knot range, or a decomposition that does not reproduce the unique-knot
//! grid each refuse [`ConstructRefusal::InvalidInput`]. This module is pure
//! exact algebra over the frozen representation: no corpus contact, no boolean
//! boundary, no float evaluation in any certificate (SFC — nothing here
//! searches).
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, no `panic!`, and no
//! module-level `allow`. **H-3.** No bare absolute literals; every named bound
//! is documented. **H-6.** No float evaluation enters any certificate — the
//! only certified quantity is the weight bracket of the refusing constructor.

use crate::construct::patches::{PatchParent, TensorBernsteinPatch};
use crate::construct::refusal::ConstructRefusal;
use crate::kernel::patch::IBox2;
use truck_geometry::prelude::{BSplineCurve, BSplineSurface, Vector4};

/// The L1 kernel: extract the per-knot-rectangle tensor-Bernstein patches of
/// a positive-weight homogeneous tensor-spline face (spec T3′).
///
/// The face is the landed homogeneous carrier of the Theorem A adapter
/// (`BSplineSurface<Vector4>`): a clamped tensor-spline face whose control
/// points are the homogeneous `(w·x, w·y, w·z, w)` net. Every knot rectangle
/// is Bézier-extracted (an exact local change of basis, spec Theorem A) by the
/// row-then-column tensor cut over the landed curve decomposition machinery,
/// re-parameterized to `[0, 1]²`, and returned as a
/// [`TensorBernsteinPatch`] carrying its source-domain box (the exact knot
/// rectangle per axis) and its parent identity (face ordinal 0 in the
/// single-face lemma; the ADM-001 pair adapter owns pair numbering). Each
/// span's weight bracket is certified strictly positive by the patch's
/// refusing constructor — a span whose bracket cannot be certified refuses
/// [`ConstructRefusal::InvalidInput`], and so does a malformed face.
///
/// Refuses [`ConstructRefusal::InvalidInput`] on an empty, ragged, or
/// non-finite control net; on a face whose knot vectors are not clamped (an
/// unclamped face's end spans are not exact knot rectangles of a declared
/// domain); on a degenerate knot range (fewer than two distinct knots per
/// axis); and on any decomposition whose piece count does not match the
/// unique-knot grid (the tensor cut did not reproduce the declared rectangles
/// losslessly).
pub fn extract_patches(
    face: &BSplineSurface<Vector4>,
) -> Result<Vec<TensorBernsteinPatch>, ConstructRefusal> {
    let ctrl = face.control_points();
    if ctrl.is_empty() || ctrl[0].is_empty() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let nu_total = ctrl.len();
    let nv_total = ctrl[0].len();
    if ctrl.iter().any(|row| row.len() != nv_total) {
        return Err(ConstructRefusal::InvalidInput);
    }
    for row in ctrl {
        for point in row {
            if !(point.x.is_finite()
                && point.y.is_finite()
                && point.z.is_finite()
                && point.w.is_finite())
            {
                return Err(ConstructRefusal::InvalidInput);
            }
        }
    }
    if !face.is_clamped() {
        return Err(ConstructRefusal::InvalidInput);
    }
    let udegree = face.udegree();
    let vdegree = face.vdegree();
    let (u_knots, _) = face.uknot_vec().to_single_multi();
    let (v_knots, _) = face.vknot_vec().to_single_multi();
    let nu_pieces = u_knots
        .len()
        .checked_sub(1)
        .ok_or(ConstructRefusal::InvalidInput)?;
    let nv_pieces = v_knots
        .len()
        .checked_sub(1)
        .ok_or(ConstructRefusal::InvalidInput)?;
    if nu_pieces == 0 || nv_pieces == 0 {
        return Err(ConstructRefusal::InvalidInput);
    }

    // Decompose every v-column (fixed net column j) along u. Each column is a
    // BSplineCurve in the u parameter over the surface's u knot vector, so its
    // Bezier decomposition cuts exactly at the u unique knots.
    let mut rows_u: Vec<Vec<BSplineCurve<Vector4>>> = Vec::with_capacity(nv_total);
    for j in 0..nv_total {
        let points: Vec<Vector4> = (0..nu_total).map(|i| *face.control_point(i, j)).collect();
        let column = BSplineCurve::new_unchecked(face.uknot_vec().clone(), points);
        rows_u.push(column.bezier_decomposition());
    }
    if rows_u.iter().any(|row| row.len() != nu_pieces) {
        return Err(ConstructRefusal::InvalidInput);
    }

    let mut patches: Vec<TensorBernsteinPatch> = Vec::new();
    for iu in 0..nu_pieces {
        // For the iu-th u-piece, every u Bernstein index a fixes a curve along
        // v (over the v knot vector): decompose those columns along v.
        let mut col_pieces: Vec<Vec<BSplineCurve<Vector4>>> = Vec::with_capacity(udegree + 1);
        for a in 0..=udegree {
            let points: Vec<Vector4> = (0..nv_total)
                .map(|j| *rows_u[j][iu].control_point(a))
                .collect();
            let row = BSplineCurve::new_unchecked(face.vknot_vec().clone(), points);
            col_pieces.push(row.bezier_decomposition());
        }
        if col_pieces.iter().any(|column| column.len() != nv_pieces) {
            return Err(ConstructRefusal::InvalidInput);
        }
        for iv in 0..nv_pieces {
            // Assemble the (iu, iv) rectangle's tensor net: rows over u (the
            // Bernstein index a), columns over v (the index b).
            let mut numerator: Vec<Vec<[f64; 3]>> = Vec::with_capacity(udegree + 1);
            let mut weights: Vec<Vec<f64>> = Vec::with_capacity(udegree + 1);
            for col_piece in &col_pieces {
                let controls = col_piece[iv].control_points();
                let mut num_row: Vec<[f64; 3]> = Vec::with_capacity(vdegree + 1);
                let mut w_row: Vec<f64> = Vec::with_capacity(vdegree + 1);
                for point in controls {
                    num_row.push([point.x, point.y, point.z]);
                    w_row.push(point.w);
                }
                numerator.push(num_row);
                weights.push(w_row);
            }
            let domain = IBox2 {
                lo: [u_knots[iu], v_knots[iv]],
                hi: [u_knots[iu + 1], v_knots[iv + 1]],
            };
            // The refusing constructor certifies the weight bracket: a span
            // whose bracket cannot be certified strictly positive refuses.
            patches.push(TensorBernsteinPatch::try_new(
                numerator,
                weights,
                domain,
                PatchParent::new(0, None),
            )?);
        }
    }
    Ok(patches)
}
