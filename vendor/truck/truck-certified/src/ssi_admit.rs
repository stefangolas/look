//! The spline×analytic SSI dispatch bridge (CL-001-SPLINE-LIFT).
//!
//! The general-pair SSI engine ([`crate::ssi`]) is landed and stranded: no
//! funnel caller feeds it. CL-000-SPLINE-ADMIT landed the spline-side
//! admission ([`crate::patch_admit`]) plus the `BSplineSurface` derivative
//! enclosures; this module is the dispatch arm that admits a **spline ×
//! analytic** carrier pair into the engine.
//!
//! # The pair form
//!
//! [`certify_spline_analytic_pair`] takes the contact layer's recognized
//! carrier pair directly:
//!
//! * the **spline side** is a clamped `BSplineSurface<Vector4>` (the corpus
//!   loft carrier, unit-weight homogeneous net), admitted through CL-000's
//!   [`admit_surface`](crate::patch_admit::admit_surface) into its per-knot-span
//!   `RationalBipatch` stack;
//! * the **analytic side** is an exact affine carrier (a `Plane` over its
//!   window), the class carried by the landed `EnclosureSurface::as_plane`
//!   hook — a plane is exactly a bidegree-`(1,1)` rational tensor-Bernstein
//!   patch whose coefficient grid is its corner lattice, so the analytic side
//!   becomes an `SsiParticipant::RationalBipatch` with no new math.
//!
//! The pair's square system is [`construct_square_system`] and the certified
//! path is [`krawczyk3_certificate`] (instantiated, never extended —
//! `ssi.rs` / `ssi_types.rs` are untouched). Where the engine cannot certify
//! (conditioning, determinant spanning zero, non-strict inclusion, hull
//! failure) or where admission refused (bidegree budget, ragged, non-finite,
//! non-affine analytic side, a spline carrier with more than one knot span),
//! the dispatch emits the typed [`SsiAdmitOutcome::Unresolved`] carrying the
//! κ / cell / slope witness slot — never a bare refusal and never a guess.
//!
//! # House rules
//!
//! **H-1** (no `unwrap`/`expect`/`panic!` reachable from geometry) applies:
//! the crate-level `#![deny(clippy::unwrap_used)]` in `lib.rs` covers this
//! module and every fallible step unpacks through named outcomes.

use crate::patch_admit::{admit_surface, SplineAdmissionRefusal};
use crate::ssi::{
    construct_square_system, krawczyk3_certificate, RationalBipatch, SsiParticipant, SsiRefusal,
};
use crate::ssi_types::{KrawczykCertificate3, SquareSystem3};
use truck_base::cgmath64::Point3;
use truck_base::evidence::{Budget, Certificate, Margin, Method, Modulus, PropMap};
use truck_evidence::contact::{
    set_spline_ssi_entry, solver_entry::ParameterCell, SplineSsiEntry, SplineSsiSetError,
    SsiCertifiedRoot, SsiSplineSolve,
};
use truck_geometry::nurbs::BSplineSurface;
use truck_geometry::prelude::Vector4;
use truck_geometry::recognize::CanonicalSurface;
use truck_geometry::specifieds::Plane;
use truck_geotrait::ParametricSurface;

/// Why a spline×analytic dispatch could not certify the box.
///
/// Named causes only — the landed admission refusal vocabulary and the landed
/// SSI engine refusal vocabulary, plus the two envelope conditions this arm
/// owns. There is no catch-all and no new `truck-base` `Refusal` arm.
///
/// `Eq` is intentionally absent (FSSI-000 decision 3): the wrapped
/// [`SsiRefusal`] now carries `(f64, f64)` suspicion evidence, and nothing in
/// the crate uses this cause as a map key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SsiAdmitCause {
    /// The spline carrier's admission refused (CL-000's named gates).
    SplineAdmission(SplineAdmissionRefusal),
    /// The analytic side is not an exactly-affine carrier (only the `Plane`
    /// class is carried by the landed `EnclosureSurface::as_plane` hook), so
    /// it has no exact rational tensor-Bernstein patch over its window.
    AnalyticNotAffine,
    /// The spline carrier is not a single knot-span patch (cross-patch
    /// assembly is CL-002's booked content).
    WindowNotOneSpan,
    /// The landed SSI engine refused the box (its own named cases).
    Ssi(SsiRefusal),
}

/// The typed outcome of one spline×analytic SSI dispatch (CL-001).
///
/// Exactly two arms: a certified unique root of the pair's square system on
/// the given box (the [`SquareSystem3`] that produced it plus its
/// [`KrawczykCertificate3`]), or the typed unresolved verdict carrying the
/// κ / cell / slope witness slot of the box that could not be certified.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)] // The Certified arm carries the constructed SquareSystem3 by value (the packet's "produces a SquareSystem3" contract); boxing would complicate the dispatch outcome.
pub enum SsiAdmitOutcome {
    /// A certified unique root of the pair's square system on the box.
    Certified {
        /// The constructed square system (spline patch × analytic patch).
        system: SquareSystem3,
        /// The Krawczyk unique-root certificate over the box.
        certificate: KrawczykCertificate3,
        /// The float chart centre of the certified box (diagnostics; the
        /// certificate is the certified statement — H-6).
        chart: [f64; 4],
        /// The model point at the chart centre on the spline carrier (float
        /// diagnostics; the certificate is the certified statement — H-6).
        centre: Point3,
    },
    /// The box could not be certified; the κ / cell / slope witness slot
    /// records why (never a bare refusal, never a guess).
    Unresolved {
        /// The conditioning-style witness (reciprocal of the dominant
        /// reduced-minor magnitude at the box centre; large when every
        /// candidate continuation is near-degenerate).
        kappa: f64,
        /// The `(u, v) × (s, t)` box that stayed unresolved, verbatim.
        cell: [(f64, f64); 4],
        /// The §5.4 slope diagnostic of the unresolved cell.
        slope: f64,
        /// The named cause that kept the box from a certificate.
        cause: SsiAdmitCause,
    },
}

/// Certifies one spline × plane box of the pair's square system.
///
/// * `spline` is the clamped homogeneous spline carrier (the corpus loft
///   shape); the pair is certified on its single admitted knot-span patch. A
///   multi-span carrier emits [`SsiAdmitCause::WindowNotOneSpan`] — cross-patch
///   assembly is CL-002's booked content.
/// * `plane` is the exact affine analytic side over `analytic_window`
///   `((s0, s1), (t0, t1))` in the plane's own chart. The window maps onto
///   the patch's unit chart affinely; the corner lattice of the window is the
///   patch's coefficient grid (exact for an affine map).
/// * `box_` is the four-interval box in the shared unit chart of the two
///   patches (`(u, v)` on the spline patch, `(s, t)` on the plane patch).
///
/// Every admission refusal and every engine refusal propagates as the typed
/// [`SsiAdmitOutcome::Unresolved`] (never a panic). The certified arm carries
/// the system and the Krawczyk certificate. Deterministic: the candidate
/// continuation axes are scanned in ascending order and the first strict
/// inclusion wins.
pub fn certify_spline_analytic_pair(
    spline: &BSplineSurface<Vector4>,
    plane: &Plane,
    analytic_window: ((f64, f64), (f64, f64)),
    box_: [(f64, f64); 4],
) -> SsiAdmitOutcome {
    let stack = match admit_surface(spline) {
        Ok(stack) => stack,
        Err(refusal) => {
            return unresolved(box_, SsiAdmitCause::SplineAdmission(refusal));
        }
    };
    let Some(span) = single_span(&stack) else {
        return unresolved(box_, SsiAdmitCause::WindowNotOneSpan);
    };
    let spline_patch = span.patch.clone();

    let analytic_patch = match plane_patch(plane, analytic_window) {
        Some(patch) => patch,
        None => return unresolved(box_, SsiAdmitCause::AnalyticNotAffine),
    };

    let system = match construct_square_system(
        &SsiParticipant::RationalBipatch(spline_patch.clone()),
        &SsiParticipant::RationalBipatch(analytic_patch),
    ) {
        Ok(system) => system,
        Err(refusal) => return unresolved(box_, SsiAdmitCause::Ssi(refusal)),
    };

    // The certified path is krawczyk3_certificate over the box; scan the
    // candidate continuation axes in ascending order (determinism) and take
    // the first strict inclusion. None certifying is a typed unresolved with
    // the κ / cell / slope witness of the box.
    let mut conditioning: Option<SsiRefusal> = None;
    let mut first_failure: Option<SsiRefusal> = None;
    for axis in 0..4 {
        match krawczyk3_certificate(&system, axis, box_) {
            Ok(certificate) => {
                let chart = box_centre(box_);
                let centre = model_centre(&spline_patch, (chart[0], chart[1]));
                return SsiAdmitOutcome::Certified {
                    system,
                    certificate,
                    chart,
                    centre,
                };
            }
            Err(SsiRefusal::Conditioning(cause)) => {
                if conditioning.is_none() {
                    conditioning = Some(SsiRefusal::Conditioning(cause));
                }
            }
            Err(failure) => {
                if first_failure.is_none() {
                    first_failure = Some(failure);
                }
            }
        }
    }
    let cause = match conditioning {
        Some(refusal) => SsiAdmitCause::Ssi(refusal),
        None => match first_failure {
            Some(failure) => SsiAdmitCause::Ssi(failure),
            None => SsiAdmitCause::Ssi(SsiRefusal::DeterminantSpansZero),
        },
    };
    let (kappa, slope) = cell_diagnostics(&system, box_);
    SsiAdmitOutcome::Unresolved {
        kappa,
        cell: box_,
        slope,
        cause,
    }
}

/// The typed unresolved for a box whose admission refused before any square
/// system existed.
fn unresolved(box_: [(f64, f64); 4], cause: SsiAdmitCause) -> SsiAdmitOutcome {
    SsiAdmitOutcome::Unresolved {
        kappa: 1.0e12,
        cell: box_,
        slope: 0.0,
        cause,
    }
}

/// The admitted spline span of a carrier, when it is exactly one knot-span
/// patch (the CL-001 envelope: cross-patch assembly is CL-002's content).
fn single_span(
    stack: &crate::patch_admit::SplinePatchStack,
) -> Option<&crate::patch_admit::AdmittedPatch> {
    let mut patches = stack.patches().iter();
    let first = patches.next()?;
    if patches.next().is_some() {
        return None;
    }
    Some(first)
}

/// The exact bidegree-`(1,1)` rational patch of an affine plane over its
/// window. An affine map's Bernstein coefficients are its corner lattice, so
/// the coefficient grid is the four window-corner images; every weight is 1
/// (the plane is non-rational). `None` when the window is degenerate.
fn plane_patch(plane: &Plane, window: ((f64, f64), (f64, f64))) -> Option<RationalBipatch> {
    let ((s0, s1), (t0, t1)) = window;
    if !finite_ordered((s0, s1)) || !finite_ordered((t0, t1)) {
        return None;
    }
    let corner = |a: usize, b: usize| -> Point3 {
        let s = if a == 0 { s0 } else { s1 };
        let t = if b == 0 { t0 } else { t1 };
        plane.subs(s, t)
    };
    let mut x = vec![vec![0.0f64; 2]; 2];
    let mut y = vec![vec![0.0f64; 2]; 2];
    let mut z = vec![vec![0.0f64; 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            let p = corner(a, b);
            x[a][b] = p.x;
            y[a][b] = p.y;
            z[a][b] = p.z;
        }
    }
    let weights = vec![vec![1.0f64; 2]; 2];
    RationalBipatch::new(1, 1, [x, y, z], weights).ok()
}

/// Whether an interval is finite and non-degenerate (`lo < hi`).
fn finite_ordered((lo, hi): (f64, f64)) -> bool {
    lo.is_finite() && hi.is_finite() && lo < hi
}

/// The centre of a four-interval box.
fn box_centre(box_: [(f64, f64); 4]) -> [f64; 4] {
    let mut centre = [0.0f64; 4];
    for (k, (lo, hi)) in box_.iter().enumerate() {
        centre[k] = 0.5 * (lo + hi);
    }
    centre
}

/// The model point of the spline patch at its unit-chart parameter, evaluated
/// from the admitted numerator grid (the certified statement is the cell; this
/// point is float diagnostics — H-6).
fn model_centre(patch: &RationalBipatch, at: (f64, f64)) -> Point3 {
    let num = patch.numerator();
    let x = bern2d(&num[0], at.0, at.1).unwrap_or(0.0);
    let y = bern2d(&num[1], at.0, at.1).unwrap_or(0.0);
    let z = bern2d(&num[2], at.0, at.1).unwrap_or(0.0);
    Point3::new(x, y, z)
}

/// The κ / slope diagnostics of an unresolved box: κ is the reciprocal of the
/// dominant 3×3 reduced-minor magnitude of the float Jacobian at the box
/// centre (a conditioning-style witness), slope the signed ratio of that minor
/// to its column-norm product. Computed from the stored system only (float
/// diagnostics, H-6; the box is the typed witness).
fn cell_diagnostics(system: &SquareSystem3, box_: [(f64, f64); 4]) -> (f64, f64) {
    let centre = box_centre(box_);
    let jacobian = float_jacobian(system, centre);
    let Some((best_det, minor)) = dominant_minor(&jacobian) else {
        return (1.0e12, 0.0);
    };
    let n0 = col_norm(&[minor[0][0], minor[1][0], minor[2][0]]);
    let n1 = col_norm(&[minor[0][1], minor[1][1], minor[2][1]]);
    let n2 = col_norm(&[minor[0][2], minor[1][2], minor[2][2]]);
    let scale = n0 * n1 * n2;
    let slope = if scale > 0.0 { best_det / scale } else { 0.0 };
    let kappa = if best_det.abs() > 0.0 {
        1.0 / best_det.abs()
    } else {
        1.0e12
    };
    (kappa, slope)
}

/// The float Jacobian `∂F_component / ∂(chart axis)` of the stored square
/// system at a point, evaluated exactly from the stored Bernstein grids.
fn float_jacobian(system: &SquareSystem3, point: [f64; 4]) -> [[f64; 4]; 3] {
    let mut out = [[0.0f64; 4]; 3];
    for (component, row) in out.iter_mut().enumerate() {
        for (axis, cell) in row.iter_mut().enumerate() {
            *cell = point_partial(system, component, axis, point);
        }
    }
    out
}

/// The value of one component's partial derivative along one chart axis of the
/// stored square system at a point. The axis derivative lowers that axis's
/// Bernstein degree by one and applies the exact difference operator
/// `d·(c[k+1] − c[k])` to the coefficients before evaluation.
fn point_partial(system: &SquareSystem3, component: usize, axis: usize, point: [f64; 4]) -> f64 {
    let grid = match system.grids().get(component) {
        Some(g) => g,
        None => return 0.0,
    };
    let (m1, n1, m2, n2) = system.degrees();
    let rows = (m1 + 1) * (n1 + 1);
    if grid.len() != rows {
        return 0.0;
    }
    let cols = match grid.first() {
        Some(row) => row.len(),
        None => return 0.0,
    };
    if cols != (m2 + 1) * (n2 + 1) || grid.iter().any(|row| row.len() != cols) {
        return 0.0;
    }
    // Basis vectors at the point; the differentiated axis uses the reduced
    // degree `d − 1` basis.
    let u_full = bernstein_basis(m1, point[0]);
    let v_full = bernstein_basis(n1, point[1]);
    let s_full = bernstein_basis(m2, point[2]);
    let t_full = bernstein_basis(n2, point[3]);
    let mut acc = 0.0f64;
    match axis {
        0 => {
            if m1 == 0 {
                return 0.0;
            }
            let u_red = bernstein_basis(m1 - 1, point[0]);
            for (a, ua) in u_red.iter().enumerate() {
                for (b, vb) in v_full.iter().enumerate() {
                    for (i, si) in s_full.iter().enumerate() {
                        for (j, tj) in t_full.iter().enumerate() {
                            let col = i * (n2 + 1) + j;
                            let lo = grid[a * (n1 + 1) + b][col];
                            let hi = grid[(a + 1) * (n1 + 1) + b][col];
                            acc += (hi - lo) * m1 as f64 * ua * vb * si * tj;
                        }
                    }
                }
            }
        }
        1 => {
            if n1 == 0 {
                return 0.0;
            }
            let v_red = bernstein_basis(n1 - 1, point[1]);
            for (a, ua) in u_full.iter().enumerate() {
                for (b, vb) in v_red.iter().enumerate() {
                    for (i, si) in s_full.iter().enumerate() {
                        for (j, tj) in t_full.iter().enumerate() {
                            let col = i * (n2 + 1) + j;
                            let lo = grid[a * (n1 + 1) + b][col];
                            let hi = grid[a * (n1 + 1) + b + 1][col];
                            acc += (hi - lo) * n1 as f64 * ua * vb * si * tj;
                        }
                    }
                }
            }
        }
        2 => {
            if m2 == 0 {
                return 0.0;
            }
            let s_red = bernstein_basis(m2 - 1, point[2]);
            for (a, ua) in u_full.iter().enumerate() {
                for (b, vb) in v_full.iter().enumerate() {
                    for (i, si) in s_red.iter().enumerate() {
                        for (j, tj) in t_full.iter().enumerate() {
                            let row = a * (n1 + 1) + b;
                            let lo = grid[row][i * (n2 + 1) + j];
                            let hi = grid[row][(i + 1) * (n2 + 1) + j];
                            acc += (hi - lo) * m2 as f64 * ua * vb * si * tj;
                        }
                    }
                }
            }
        }
        _ => {
            if n2 == 0 {
                return 0.0;
            }
            let t_red = bernstein_basis(n2 - 1, point[3]);
            for (a, ua) in u_full.iter().enumerate() {
                for (b, vb) in v_full.iter().enumerate() {
                    for (i, si) in s_full.iter().enumerate() {
                        for (j, tj) in t_red.iter().enumerate() {
                            let row = a * (n1 + 1) + b;
                            let lo = grid[row][i * (n2 + 1) + j];
                            let hi = grid[row][i * (n2 + 1) + j + 1];
                            acc += (hi - lo) * n2 as f64 * ua * vb * si * tj;
                        }
                    }
                }
            }
        }
    }
    acc
}

/// The degree-`d` Bernstein basis vector at `x`: `B_k^d(x)` for `0..=d`.
fn bernstein_basis(degree: usize, x: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(degree + 1);
    for k in 0..=degree {
        out.push(bernstein(degree, k, x));
    }
    out
}

/// One Bernstein basis function `C(d,k)·x^k·(1−x)^(d−k)`.
fn bernstein(degree: usize, k: usize, x: f64) -> f64 {
    if k > degree {
        return 0.0;
    }
    let binom = binomial(degree, k);
    if binom == 0.0 {
        return 0.0;
    }
    let pow = |base: f64, e: usize| {
        let mut acc = 1.0f64;
        for _ in 0..e {
            acc *= base;
        }
        acc
    };
    binom * pow(x, k) * pow(1.0 - x, degree - k)
}

/// The exact small binomial `C(n, k)` as `f64`.
fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    let mut acc = 1.0f64;
    for i in 0..k {
        acc = acc * (n - i) as f64 / (i + 1) as f64;
    }
    acc
}

/// The 2-D Bernstein evaluation of a grid at `(p, q)` by de Casteljau along
/// the second parameter then the first. `None` for an empty or ragged grid.
fn bern2d(grid: &[Vec<f64>], p: f64, q: f64) -> Option<f64> {
    let rows = grid.len();
    if rows == 0 {
        return None;
    }
    let cols = grid.first()?.len();
    if cols == 0 || grid.iter().any(|row| row.len() != cols) {
        return None;
    }
    // Reduce every row (fixed first-parameter index) at `q`, then the reduced
    // list at `p`.
    let mut reduced = Vec::with_capacity(rows);
    for row in grid {
        reduced.push(decasteljau(row, q)?);
    }
    decasteljau(&reduced, p)
}

/// One-dimensional de Casteljau evaluation of a coefficient list at `x`.
fn decasteljau(coeffs: &[f64], x: f64) -> Option<f64> {
    if coeffs.is_empty() {
        return None;
    }
    let mut work: Vec<f64> = coeffs.to_vec();
    let n = work.len();
    for level in 1..n {
        for i in 0..(n - level) {
            work[i] = (1.0 - x) * work[i] + x * work[i + 1];
        }
    }
    Some(work[0])
}

/// The four 3×3 minors of the float 3×4 Jacobian (drop column `axis`), and the
/// dominant one by absolute determinant.
fn dominant_minor(jacobian: &[[f64; 4]; 3]) -> Option<(f64, [[f64; 3]; 3])> {
    let mut best: Option<(f64, [[f64; 3]; 3])> = None;
    for drop in 0..4 {
        let mut m = [[0.0f64; 3]; 3];
        for (r, row) in jacobian.iter().enumerate() {
            let mut col = 0usize;
            for (c, entry) in row.iter().enumerate() {
                if c == drop {
                    continue;
                }
                m[r][col] = *entry;
                col += 1;
            }
        }
        let d = det3(m);
        let better = match best {
            Some((best_d, _)) => d.abs() > best_d.abs(),
            None => true,
        };
        if better {
            best = Some((d, m));
        }
    }
    best
}

/// The determinant of a float 3×3 matrix.
fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// The Euclidean norm of a float 3-vector.
fn col_norm(v: &[f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

// ---------------------------------------------------------------------------
// The registered funnel entry (CL-001 dispatch arm).
// ---------------------------------------------------------------------------

/// The certified spline×analytic SSI dispatch engine (CL-001), registered into
/// the `truck-evidence` funnel registry slot. It adapts
/// [`certify_spline_analytic_pair`] onto the funnel's typed outcome
/// vocabulary; a non-affine analytic carrier stays a typed unresolved (the
/// engine never ran the certified path for it).
#[derive(Clone, Copy, Debug, Default)]
pub struct SsiAdmitSolver;

impl SplineSsiEntry for SsiAdmitSolver {
    fn certify_spline_box(
        &self,
        spline: &BSplineSurface<Vector4>,
        analytic: &CanonicalSurface,
        analytic_window: ((f64, f64), (f64, f64)),
        box_: [(f64, f64); 4],
        budget: &mut Budget,
    ) -> SsiSplineSolve {
        let plane = match analytic {
            CanonicalSurface::Plane(plane) => plane,
            // Only the exact affine class carries a rational tensor-Bernstein
            // patch over its window; everything else is a typed unresolved.
            _ => {
                return SsiSplineSolve::Unresolved {
                    kappa: 1.0e12,
                    cell: entry_cell(box_),
                    slope: 0.0,
                    spent: *budget,
                }
            }
        };
        match certify_spline_analytic_pair(spline, plane, analytic_window, box_) {
            SsiAdmitOutcome::Certified { chart, centre, .. } => {
                SsiSplineSolve::Certified(SsiCertifiedRoot {
                    cell: entry_cell(box_),
                    chart,
                    centre,
                    certificate: dispatch_certificate(budget),
                })
            }
            SsiAdmitOutcome::Unresolved {
                kappa, cell, slope, ..
            } => SsiSplineSolve::Unresolved {
                kappa,
                cell: entry_cell(cell),
                slope,
                spent: *budget,
            },
        }
    }
}

/// Maps a four-interval box onto the funnel's parameter-cell record.
fn entry_cell(box_: [(f64, f64); 4]) -> ParameterCell {
    ParameterCell {
        u: box_[0],
        v: box_[1],
        s: box_[2],
        t: box_[3],
    }
}

/// The solve certificate of a certified dispatch: an interval certificate
/// whose remaining budget is the caller's unchanged budget (a single-box
/// certificate spends nothing from the caller's ledger).
fn dispatch_certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// Registers the certified spline×analytic SSI dispatch engine into the
/// `truck-evidence` funnel registry slot (CL-001-SPLINE-LIFT). The call is the
/// explicit, `#[ctor]`-free registration the kernel's entry point wires: with
/// the engine registered the funnel certifies spline×analytic pairs through
/// the SSI engine; without it, the funnel answers today's typed unresolved.
///
/// Set-once: a second registration while the slot is occupied refuses.
pub fn register_ssi_admit_solver() -> Result<(), SplineSsiSetError> {
    set_spline_ssi_entry(SsiAdmitSolver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Refusal;
    use truck_evidence::contact::{spline_analytic_contact, take_spline_ssi_entry, ContactLocus};
    use truck_geometry::nurbs::KnotVec;
    use truck_geometry::prelude::Vector4;
    use truck_geometry::specifieds::Plane;

    /// The analytic plane `(s, t, 1/4 + s/2)` over the unit window.
    fn analytic_plane() -> Plane {
        Plane::new(
            Point3::new(0.0, 0.0, 0.25),
            Point3::new(1.0, 0.0, 0.75),
            Point3::new(0.0, 1.0, 0.25),
        )
    }

    /// A bilinear spline carrier from its four corner points, rows indexing
    /// the first parameter.
    fn bilinear_spline(corners: [[[f64; 3]; 2]; 2]) -> BSplineSurface<Vector4> {
        let knot = KnotVec::bezier_knot(1);
        let ctrl = corners.map(|row| row.map(|[x, y, z]| Vector4::new(x, y, z, 1.0)).to_vec());
        BSplineSurface::new((knot.clone(), knot), ctrl.to_vec())
    }

    /// The spline carrier whose single bilinear patch is `(u, v, v)`.
    fn spline_carrier() -> BSplineSurface<Vector4> {
        bilinear_spline([
            [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0]],
            [[1.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
        ])
    }

    /// The conditioning fixture's patch 1 `(0, u, u+v)` as a spline carrier.
    fn conditioning_spline() -> BSplineSurface<Vector4> {
        bilinear_spline([
            [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0, 1.0, 1.0], [0.0, 1.0, 2.0]],
        ])
    }

    /// The conditioning fixture's patch 2 `(s+t−1, t, 1)` as the exact affine
    /// analytic side over the unit window.
    fn conditioning_plane() -> Plane {
        Plane::new(
            Point3::new(-1.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        )
    }

    #[test]
    fn spline_analytic_pair_dispatches_through_ssi() {
        // A spline×analytic pair (CL-000's admitted extraction on the spline
        // side, the exact affine patch on the analytic side) produces a
        // SquareSystem3 and a Krawczyk verdict on a dyadic fixture.
        let spline = spline_carrier();
        let plane = analytic_plane();
        let outcome = certify_spline_analytic_pair(
            &spline,
            &plane,
            ((0.0, 1.0), (0.0, 1.0)),
            [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)],
        );
        let SsiAdmitOutcome::Certified {
            system,
            certificate,
            centre,
            ..
        } = &outcome
        else {
            panic!("the dyadic spline×plane pair must certify, got {outcome:?}");
        };
        // The system's degrees are the two bilinear patches' bidegrees, and
        // the spline side really dispatched through CL-000's admission: the
        // stack of the same carrier admits to exactly one patch.
        assert_eq!(system.degrees(), (1, 1, 1, 1));
        let stack = admit_surface(&spline).expect("the carrier admits");
        assert_eq!(stack.len(), 1);
        // The certificate is a strict Krawczyk inclusion with the determinant
        // enclosure away from zero.
        for (b, k) in certificate.box_x().iter().zip(certificate.k_x().iter()) {
            assert!(b.0 < k.0 && k.1 < b.1, "K(X) strictly inside X");
        }
        let (d_lo, d_hi) = certificate.det();
        assert!(
            (d_lo > 0.0 && d_hi > 0.0) || (d_lo < 0.0 && d_hi < 0.0),
            "det excludes zero"
        );
        assert!(centre.x.is_finite() && centre.y.is_finite() && centre.z.is_finite());
    }

    #[test]
    fn unresolved_carries_kappa_cell_slope() {
        // A conditioning-refused pair yields the typed Unresolved witness
        // (κ / cell / slope), never a bare refusal and never a guess. The
        // fixture pair `(0, u, u+v)` × `(s+t−1, t, 1)` refuses the frozen
        // coordinate rule over the unit box on every continuation axis.
        let spline = conditioning_spline();
        let plane = conditioning_plane();
        let box_: [(f64, f64); 4] = [(0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)];
        let outcome = certify_spline_analytic_pair(&spline, &plane, ((0.0, 1.0), (0.0, 1.0)), box_);
        let SsiAdmitOutcome::Unresolved {
            kappa,
            cell,
            slope,
            cause,
        } = &outcome
        else {
            panic!("a conditioning-refused pair must stay unresolved, got {outcome:?}");
        };
        assert_eq!(*cell, box_, "the unresolved cell is the box, verbatim");
        assert!(
            kappa.is_finite() && *kappa > 0.0,
            "κ is a positive finite conditioning witness, got {kappa}"
        );
        assert!(slope.is_finite(), "slope is finite");
        match cause {
            SsiAdmitCause::Ssi(SsiRefusal::Conditioning(Refusal::ConditioningBelowThreshold)) => {}
            other => panic!("the refused cause is the conditioning refusal, got {other:?}"),
        }

        // Admission-level refusals also stay typed unresolved: a rational
        // spline carrier (weight ≠ 1) refuses at the admission gate.
        let knot = KnotVec::bezier_knot(1);
        let rational = BSplineSurface::new(
            (knot.clone(), knot),
            vec![
                vec![
                    Vector4::new(0.0, 0.0, 0.0, 2.0),
                    Vector4::new(0.0, 1.0, 1.0, 2.0),
                ],
                vec![
                    Vector4::new(1.0, 0.0, 0.0, 2.0),
                    Vector4::new(1.0, 1.0, 1.0, 2.0),
                ],
            ],
        );
        let refused =
            certify_spline_analytic_pair(&rational, &plane, ((0.0, 1.0), (0.0, 1.0)), box_);
        match refused {
            SsiAdmitOutcome::Unresolved {
                cause: SsiAdmitCause::SplineAdmission(SplineAdmissionRefusal::RationalWeights),
                ..
            } => {}
            other => panic!("a rational carrier refuses typed at admission, got {other:?}"),
        }
    }

    #[test]
    fn registered_solver_dispatches_spline_box_through_the_funnel() {
        // With the certified engine registered, the contact funnel's
        // spline×analytic entry certifies the dyadic well-conditioned pair:
        // one certified Point0 contact record carrying the certified root's
        // model centre (the CL-001 dispatch arm end to end).
        let _cleared = take_spline_ssi_entry();
        register_ssi_admit_solver().expect("the engine registers into the cleared slot");
        let spline = spline_carrier();
        let analytic = CanonicalSurface::Plane(analytic_plane());
        let mut budget = Budget::new(100, 100, 100);
        let out = spline_analytic_contact(
            &spline,
            &analytic,
            ((0.0, 1.0), (0.0, 1.0)),
            [(0.4, 0.6), (0.4, 0.6), (0.4, 0.6), (0.4, 0.6)],
            &mut budget,
        );
        let certified = out.expect("the registered engine certifies the dyadic pair");
        assert_eq!(certified.value.contacts.len(), 1);
        let record = &certified.value.contacts[0];
        assert_eq!(
            record.dimension,
            truck_base::contact::ContactDimension::Point0
        );
        assert_eq!(
            record.kind,
            truck_base::contact::ContactEventKind::Transverse
        );
        let ContactLocus::Point(point) = record.locus else {
            panic!(
                "a certified root emits a Point locus, got {:?}",
                record.locus
            );
        };
        assert!(
            point.x.is_finite() && point.y.is_finite() && point.z.is_finite(),
            "the certified model centre is finite"
        );
        assert_eq!(certified.cert.method, Method::Interval);
    }
}
