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

//! The deflated square system `T = (G1, G2, M1, M2)` (CTE-003-MINORS; theory
//! §2.4).
//!
//! `TSystem` instantiates the landed generic
//! [`KrawczykSystem`](truck_evidence::num::krawczyk::KrawczykSystem) for
//! `N = 4`: the four rows are stored as the CTE substrate's polynomial grids
//! ([`PolyGrid`](crate::tangency::minors::PolyGrid) — the two `G` component
//! grids of the chart plus the two composed chart-minor nets), expressed over
//! the UNIT chart of the stored [`SquareSystem3`]. The trait methods provide
//! the point residual (`f_point`), the ROW-MAJOR interval Jacobian (rows
//! `G1, G2, M1, M2`; columns the four chart axes in order), and a float
//! preconditioner — exactly what the landed Krawczyk operator consumes, and
//! nothing else. The N=4 precedent is `construct/bie/ssi4.rs`'s `Ssi4System`:
//! the operator is instantiated, never extended, and `num/krawczyk.rs` is not
//! edited.
//!
//! The M-row Jacobian entries are the first partials of the COMPOSED chart
//! minor polynomials, which are exactly the second partials of `F` that the
//! determinant composition certifies (theory §2.4's "the M rows' Jacobian
//! entries need second partials of F"). Differentiating the composed grid is
//! the same polynomial by construction.
//!
//! **H-1.** No unwrap, expect, panic or out-of-range indexing is reachable
//! from geometry here.
//!
//! **R1 (termination argument).** The T1.4 identity
//! `det DT = det A³·det H_h` appears ONLY as an exact F1 fixture identity test
//! (test-side) and in doc comments. No runtime function evaluates it;
//! Krawczyk consumes the honest interval Jacobian.

use std::array;

use crate::formal::exact::CertifiedInterval;
use crate::tangency::minors::{from_net, from_system_grid, PolyGrid};
use crate::tangency::shapes::Rank2Chart;
use crate::tangency::TangencyRefusal;
use crate::SquareSystem3;
use truck_evidence::enclosure::Interval;
use truck_evidence::num::krawczyk::KrawczykSystem;

/// A degenerate certified interval from a float (sound wrap, never panics).
fn iv_of(c: CertifiedInterval) -> Interval {
    Interval::try_from((c.lo, c.hi)).unwrap_or(Interval::EMPTY)
}

/// A degenerate interval of a finite float; an empty interval for a non-finite
/// value (a caller bug, never a panic — H-1).
fn iv_scalar(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// The deflated square system `T = (G1, G2, M1, M2)` over the unit chart.
///
/// Rows are the two `G` components of the chart (in ascending component
/// order) and the two chart-minor polynomials `M1, M2`. Constructed through
/// [`TSystem::from_deflated`] from a stored square system and its chart; the
/// internal row constructor [`TSystem::from_rows`] also serves the exclusion
/// driver's 4-of-5 subsystems.
#[derive(Clone, Debug)]
pub struct TSystem {
    /// The four rows `(G1, G2, M1, M2)` as unit-chart polynomial grids.
    rows: [PolyGrid; 4],
}

impl TSystem {
    /// Build the deflated system of a chart from the stored square system and
    /// the chart's pivot and chart-minor grids.
    pub fn from_deflated(
        system: &SquareSystem3,
        chart: &Rank2Chart,
    ) -> Result<TSystem, TangencyRefusal> {
        let pivot = chart.pivot();
        let f = pivot.component();
        let gs: Vec<usize> = (0..3).filter(|&c| c != f).collect();
        let g1 = from_system_grid(&system.grids()[gs[0]], system.degrees())?;
        let g2 = from_system_grid(&system.grids()[gs[1]], system.degrees())?;
        let m1 = from_net(chart.minors().m1())?;
        let m2 = from_net(chart.minors().m2())?;
        Ok(TSystem {
            rows: [g1, g2, m1, m2],
        })
    }

    /// Build a four-row Krawczyk system from arbitrary row grids (the
    /// exclusion driver's square subsystems).
    pub(crate) fn from_rows(rows: [PolyGrid; 4]) -> TSystem {
        TSystem { rows }
    }
}

impl KrawczykSystem<4> for TSystem {
    fn f_point(&self, x: &[f64; 4]) -> [Interval; 4] {
        array::from_fn(|r| match self.rows[r].float_value(*x) {
            Ok(v) => iv_scalar(v),
            Err(_) => Interval::EMPTY,
        })
    }

    fn jacobian(&self, b: &[Interval; 4]) -> [[Interval; 4]; 4] {
        let mut out = [[Interval::EMPTY; 4]; 4];
        let box_: [(f64, f64); 4] = array::from_fn(|a| {
            let axis = b.get(a).copied().unwrap_or(Interval::EMPTY);
            if axis.is_empty() || !axis.inf().is_finite() || !axis.sup().is_finite() {
                return (f64::NAN, f64::NAN);
            }
            (axis.inf(), axis.sup())
        });
        for (r, grid) in self.rows.iter().enumerate() {
            for (c, cell) in out[r].iter_mut().enumerate() {
                match grid
                    .partial_axis(c)
                    .and_then(|derived| derived.hull_unit(box_))
                {
                    Ok(enc) => *cell = iv_of(enc),
                    Err(_) => *cell = Interval::EMPTY,
                }
            }
        }
        out
    }

    fn preconditioner(&self, x: &[f64; 4]) -> Option<[[f64; 4]; 4]> {
        let mut j = [[0.0f64; 4]; 4];
        for (r, grid) in self.rows.iter().enumerate() {
            for (c, cell) in j[r].iter_mut().enumerate() {
                let derived = grid.partial_axis(c).ok()?;
                *cell = derived.float_value(*x).ok()?;
            }
        }
        invert4(&j)
    }
}

/// The float inverse of a 4x4 matrix by Gauss–Jordan with partial pivoting.
/// `None` on a singular or non-finite matrix (the operator BISECTS on `None`,
/// it does not refuse).
#[allow(clippy::needless_range_loop)] // fixed-size 4x4 Gauss-Jordan matrix index math; the index form is the algebra (ssi4.rs precedent)
fn invert4(m: &[[f64; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut a = *m;
    let mut inv = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    for col in 0..4 {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..4 {
            let cand = a[r][col].abs();
            if cand > best {
                best = cand;
                pivot = r;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            for c in 0..4 {
                let t = a[col][c];
                a[col][c] = a[pivot][c];
                a[pivot][c] = t;
                let t = inv[col][c];
                inv[col][c] = inv[pivot][c];
                inv[pivot][c] = t;
            }
        }
        let d = a[col][col];
        for c in 0..4 {
            a[col][c] /= d;
            inv[col][c] /= d;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            if factor == 0.0 {
                continue;
            }
            for c in 0..4 {
                a[r][c] -= factor * a[col][c];
                inv[r][c] -= factor * inv[col][c];
            }
        }
    }
    if inv.iter().all(|row| row.iter().all(|c| c.is_finite())) {
        Some(inv)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal::exact::CertifiedInterval;
    use crate::tangency::minors::build_chart_minors;
    use crate::tangency::minors::support::{f1_pivot, f1_system, template_chart};
    use crate::tangency::shapes::Rank2Chart;

    /// Build the real F1 chart (pivot + computed chart-minor grids) over the
    /// F1 system.
    fn f1_chart() -> Rank2Chart {
        let system = f1_system().expect("the F1 system admits");
        let pivot = f1_pivot().expect("the F1 pivot admits");
        let template = template_chart(pivot, 1.0, 1.0).expect("the template chart admits");
        let minors = build_chart_minors(&system, &template).expect("the F1 minors build");
        Rank2Chart::new(pivot, CertifiedInterval { lo: 1.0, hi: 1.0 }, minors)
            .expect("the F1 chart admits")
    }

    #[test]
    fn tsystem_admits_krawczyk4_instantiation() {
        let system = f1_system().expect("the F1 system admits");
        let chart = f1_chart();
        let t = TSystem::from_deflated(&system, &chart).expect("the F1 deflated system builds");
        // The trait impl compiles and is callable through the landed surface.
        fn require_krawczyk_system<S: KrawczykSystem<4>>(_s: &S) {}
        require_krawczyk_system(&t);

        // F1's critical point: the chart coordinates (y1, y2, z1, z2) = (0, 0,
        // 0, 0). At it every row of T vanishes: G1 = y1, G2 = y2, M1 = 6·z1,
        // M2 = −10·z2 (det A = 1, h = 3·z1² − 5·z2²).
        let critical = [0.0f64, 0.0, 0.0, 0.0];
        let f = t.f_point(&critical);
        for (i, v) in f.iter().enumerate() {
            assert!(
                !v.is_empty(),
                "row {} must evaluate at the critical point",
                i
            );
            assert_eq!(
                v.inf(),
                0.0,
                "row {} must vanish at the F1 critical point",
                i
            );
            assert_eq!(
                v.sup(),
                0.0,
                "row {} must vanish at the F1 critical point",
                i
            );
        }

        // Off the critical point the composed minors carry their signed
        // values: at (y1, y2, z1, z2) = (0, 0, 1, 0) the rows are (0, 0, 6,
        // 0). This pins the chart-minor orientation/sign of the composed
        // polynomial grids.
        const MINOR_VALUE_TOL: f64 = 1e-9; // H-3
        let off = [0.0f64, 0.0, 1.0, 0.0];
        let f = t.f_point(&off);
        for (i, v) in f.iter().enumerate() {
            assert!(
                !v.is_empty(),
                "row {} must evaluate off the critical point",
                i
            );
        }
        assert!(f[0].inf().abs() <= MINOR_VALUE_TOL && f[0].sup().abs() <= MINOR_VALUE_TOL);
        assert!(f[1].inf().abs() <= MINOR_VALUE_TOL && f[1].sup().abs() <= MINOR_VALUE_TOL);
        assert!((f[2].inf() - 6.0).abs() <= MINOR_VALUE_TOL);
        assert!((f[2].sup() - 6.0).abs() <= MINOR_VALUE_TOL);
        assert!(f[3].inf().abs() <= MINOR_VALUE_TOL && f[3].sup().abs() <= MINOR_VALUE_TOL);
    }

    #[test]
    fn t14_identity_holds_on_f1_admission() {
        // The R1 gate: the deflation-determinant identity det DT = det A³ ·
        // det H_h is asserted here EXACTLY, test-side, over the F1 fixture's
        // integer data. No runtime function anywhere evaluates it — Krawczyk
        // consumes the honest interval Jacobian (theory §2.6 Remark).
        let fixture = crate::tangency::fixtures::F1DeflationFixture::new();
        fixture
            .admit()
            .expect("the F1 ground truth is internally consistent");

        // det H_h = 4·a·c − b² for h = a·z1² + b·z1·z2 + c·z2².
        let det_h = 4 * fixture.h_z1sq * fixture.h_z2sq - fixture.h_z1z2 * fixture.h_z1z2;
        assert_eq!(det_h, -60);

        // det DT at the critical point, by the F1 chart's constant Jacobian
        // (G1 = y1, G2 = y2, M1 = 6·z1, M2 = −10·z2): the diagonal product.
        let det_dt: i64 = [1, 1, 6, -10].iter().product();
        assert_eq!(det_dt, -60);

        // T1.4: det DT = det A³ · det H_h (sign + under the §2.6 ordering).
        let det_a_cubed = fixture.det_a * fixture.det_a * fixture.det_a;
        assert_eq!(det_a_cubed * det_h, det_dt);
        assert_eq!(det_dt, fixture.det_dt);
        assert_eq!(det_h, fixture.det_h);
    }
}
