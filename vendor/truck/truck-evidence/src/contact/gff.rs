//! BG-SOL-S7-GFF-COVER — the certified branch cover of the general validated
//! FF stage.
//!
//! Given two canonical carriers' implicit fields (BG-SOL-S6-IMPLICIT), decide
//! for a 3-D search box whether the shared zero set `{ f1 = 0, f2 = 0 }`
//! passes through it — and where — using ONLY certified steps: interval
//! exclusion and the Krawczyk existence/uniqueness operator
//! (`num/krawczyk.rs`, BG-NUM-003). The engine is **branch-cover
//! enumeration**: a deterministic decomposition of the search box into proven
//! curve points, proven-singular boxes, proven-empty regions, and
//! honestly-typed unresolved remainder.
//!
//! The contact curve is `C = { p : f1(p) = 0, f2(p) = 0 }`. At any regular
//! point the tangent direction is `t = ∇f1 × ∇f2`. The certified probe is a
//! 3×3 augmented Krawczyk system: pick `m` (the box midpoint) and
//! `g = ∇f1(m) × ∇f2(m)` (renormalized; degenerate → singular), and solve
//!
//! ```text
//! F(p) = [ f1(p), f2(p), g · (p − m) ]   over the box
//! ```
//!
//! A `KrawczykProof::Unique` proves EXACTLY ONE point of C in the box that
//! also lies in the plane `g·(p−m) = 0` — one certified crossing. Interval
//! soundness comes from `ImplicitField`; existence/uniqueness from krawczyk;
//! the composition decides nothing it cannot prove.
//!
//! This writes no dispatcher logic and no `ContactLocus` arms — wiring the
//! cover into `contact()` is the next packet's job.
//!
//! House rules H-1..H-8 apply.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::enclosure::interval_at;
use crate::enclosure::Box3;
use crate::num::krawczyk::{krawczyk, KrawczykProof, KrawczykSystem};
use inari::Interval;
use truck_base::cgmath64::{InnerSpace, Point3, Vector3};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Outcome, PropMap, Refusal,
    UnresolvedWitness,
};

use super::implicit::ImplicitField;

/// What the cover proved about one leaf of the decomposition.
#[derive(Clone, Debug, PartialEq)]
pub enum CellVerdict {
    /// The box contains no point of C: some f_i enclosure excludes zero.
    Empty,
    /// The box holds (part of) a singular locus: the gradient cross product
    /// enclosure contains zero at the box midpoint AND neither field excludes
    /// zero on the box. Not further classified here.
    Singular,
    /// Krawczyk proved exactly one crossing of C through the box's mid-plane.
    Point(Point3),
}

/// The certified branch cover of a search box.
#[derive(Clone, Debug, Default)]
pub struct BranchCover {
    /// Certified crossings, in discovery order (deterministic worklist).
    pub points: Vec<Point3>,
    /// Boxes holding provable-or-suspected singular loci.
    pub singular_boxes: Vec<Box3>,
    /// Leaves neither pruned nor certified before budget/resolution ran out.
    pub unresolved_boxes: Vec<Box3>,
}

/// Decompose `domain` into CellVerdict leaves for the shared zero set of two
/// implicit fields. Deterministic: widest-axis bisection, ties toward the
/// lowest axis index. `tau` is the resolution floor — a leaf narrower than
/// `tau` on its widest axis that still cannot be classified goes to
/// `unresolved_boxes` rather than bisecting further. Subdivision spend goes
/// through `budget`.
pub fn cover_branch(
    f1: &impl ImplicitField,
    f2: &impl ImplicitField,
    domain: &Box3,
    tau: f64,
    budget: &mut Budget,
) -> Outcome<BranchCover> {
    // Spend is reported as initial − remaining (decision 2, mirrored from
    // krawczyk), so the entry budget is captured once.
    let initial = *budget;
    let d1: &dyn ImplicitField = f1;
    let d2: &dyn ImplicitField = f2;
    let mut cover = BranchCover::default();
    let mut stack: Vec<Box3> = vec![*domain];
    while let Some(b) = stack.pop() {
        // (a) Interval exclusion: some field enclosure excludes zero.
        if excludes_zero(d1.implicit(&b)) || excludes_zero(d2.implicit(&b)) {
            continue;
        }
        // (b) Singularity screen: the interval cross product of the gradient
        // boxes over B. If every component contains zero the gradients may be
        // parallel somewhere in the box (tangency/singularity), so the box
        // classifies as singular instead of probing.
        let c = box_midpoint(&b);
        let [a0, a1, a2] = d1.grad(&b);
        let [bb0, bb1, bb2] = d2.grad(&b);
        let cross = Box3 {
            x: a1 * bb2 - a2 * bb1,
            y: a2 * bb0 - a0 * bb2,
            z: a0 * bb1 - a1 * bb0,
        };
        if cross.x.contains(0.0) && cross.y.contains(0.0) && cross.z.contains(0.0) {
            cover.singular_boxes.push(b);
            continue;
        }
        // (c) Probe: the augmented 3×3 system with m = c and g the normalized
        // midpoint of the cross enclosure. A degenerate midpoint (or a
        // non-finite one) treats the box as singular.
        let mut g = Vector3::new(cross.x.mid(), cross.y.mid(), cross.z.mid());
        if !g.x.is_finite() || !g.y.is_finite() || !g.z.is_finite() || g.magnitude() == 0.0 {
            cover.singular_boxes.push(b);
            continue;
        }
        g = g.normalize();
        let sys = AugmentedFF {
            f1: d1,
            f2: d2,
            g,
            m: c,
        };
        let start = [b.x, b.y, b.z];
        // (d) The Krawczyk outcome decides the leaf.
        match krawczyk::<3>(&sys, &start, budget) {
            Ok(Certified {
                value: KrawczykProof::Unique,
                ..
            }) => {
                // Exactly one crossing of C through the box's mid-plane. The
                // recorded point is the box midpoint c refined to the
                // certified root; the Krawczyk proof is the certificate.
                cover.points.push(refine_point(&sys, c));
            }
            // NoRoot: no point of C in the box → Empty.
            Ok(Certified {
                value: KrawczykProof::NoRoot,
                ..
            }) => {}
            // The probe could not certify: bisect the box widest-axis-first
            // (ties toward the lowest index), spending budget. A leaf that
            // cannot bisect (width ≤ tau on all axes, or f64 resolution) is
            // the honest unresolved remainder.
            Err(Refusal::NumericallyUnresolved { .. }) => {
                if let Some((lo, hi)) = bisect_pair(&b, tau) {
                    if budget.spend_subdiv(1).is_err() {
                        return Err(Refusal::NumericallyUnresolved {
                            spent: spent(&initial, budget),
                            witness: UnresolvedWitness::KrawczykIndeterminate,
                        });
                    }
                    stack.push(lo);
                    stack.push(hi);
                } else {
                    cover.unresolved_boxes.push(b);
                }
            }
            // krawczyk's other refusal is `Empty` (an empty or non-finite
            // start box): the box decides nothing, treat as Empty.
            Err(_) => {}
        }
    }
    Ok(Certified::new(
        cover,
        Certificate {
            props: PropMap::new(),
            method: Method::Interval,
            budget_left: *budget,
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Unbounded,
        },
    ))
}

/// The augmented 3×3 probe system: `F(p) = [f1(p), f2(p), g·(p−m)]`.
struct AugmentedFF<'a> {
    f1: &'a dyn ImplicitField,
    f2: &'a dyn ImplicitField,
    /// The normalized tangent-direction estimate.
    g: Vector3,
    /// The reference point (the search-box midpoint).
    m: Point3,
}

impl KrawczykSystem<3> for AugmentedFF<'_> {
    /// Point evaluation: both implicit fields wrapped as degenerate intervals
    /// at `x`, plus the plane term `g·(x−m)`.
    fn f_point(&self, x: &[f64; 3]) -> [Interval; 3] {
        let [x0, x1, x2] = *x;
        let boxed = Box3::point(Point3::new(x0, x1, x2));
        let f1 = self.f1.implicit(&boxed);
        let f2 = self.f2.implicit(&boxed);
        let gx = interval_at(self.g.x);
        let gy = interval_at(self.g.y);
        let gz = interval_at(self.g.z);
        let plane = gx * (interval_at(x0) - interval_at(self.m.x))
            + gy * (interval_at(x1) - interval_at(self.m.y))
            + gz * (interval_at(x2) - interval_at(self.m.z));
        [f1, f2, plane]
    }

    /// The interval Jacobian over the box: row per field, last row = g as
    /// constants, row-major `[row][col] = dF_row/dx_col`.
    fn jacobian(&self, b: &[Interval; 3]) -> [[Interval; 3]; 3] {
        let [qx, qy, qz] = *b;
        let boxed = Box3 {
            x: qx,
            y: qy,
            z: qz,
        };
        let [f1x, f1y, f1z] = self.f1.grad(&boxed);
        let [f2x, f2y, f2z] = self.f2.grad(&boxed);
        [
            [f1x, f1y, f1z],
            [f2x, f2y, f2z],
            [
                interval_at(self.g.x),
                interval_at(self.g.y),
                interval_at(self.g.z),
            ],
        ]
    }

    /// A float inverse of the 3×3 Jacobian at a point. `None` (singular) lets
    /// krawczyk bisect (its contract).
    fn preconditioner(&self, x: &[f64; 3]) -> Option<[[f64; 3]; 3]> {
        let [x0, x1, x2] = *x;
        let boxed = Box3::point(Point3::new(x0, x1, x2));
        let [f1x, f1y, f1z] = self.f1.grad(&boxed);
        let [f2x, f2y, f2z] = self.f2.grad(&boxed);
        let jac = [
            [f1x.mid(), f1y.mid(), f1z.mid()],
            [f2x.mid(), f2y.mid(), f2z.mid()],
            [self.g.x, self.g.y, self.g.z],
        ];
        invert3x3(&jac)
    }
}

/// The inverse of a 3×3 matrix by Gaussian elimination with partial pivoting.
/// `None` when the matrix is singular (a zero or non-finite pivot); the
/// Krawczyk contract bisects on `None`.
fn invert3x3(a: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let [[a00, a01, a02], [a10, a11, a12], [a20, a21, a22]] = *a;
    // Augmented [A | I], kept row-major as 6-vectors.
    let mut m = [
        [a00, a01, a02, 1.0, 0.0, 0.0],
        [a10, a11, a12, 0.0, 1.0, 0.0],
        [a20, a21, a22, 0.0, 0.0, 1.0],
    ];
    for col in 0..3 {
        // Partial pivot: the remaining row with the largest |entry|.
        let mut pivot = col;
        let mut best = m
            .get(col)
            .and_then(|row| row.get(col))
            .copied()
            .unwrap_or(0.0)
            .abs();
        for row in (col + 1)..3 {
            let value = m
                .get(row)
                .and_then(|r| r.get(col))
                .copied()
                .unwrap_or(0.0)
                .abs();
            if value > best {
                best = value;
                pivot = row;
            }
        }
        if !best.is_finite() || best == 0.0 {
            return None;
        }
        if pivot != col {
            m.swap(col, pivot);
        }
        let pivot_value = *m.get(col)?.get(col)?;
        // Eliminate column `col` from every other row.
        for row in 0..3 {
            if row == col {
                continue;
            }
            let factor = *m.get(row)?.get(col)? / pivot_value;
            for k in 0..6 {
                let entry = *m.get(row)?.get(k)? - factor * *m.get(col)?.get(k)?;
                *m.get_mut(row)?.get_mut(k)? = entry;
            }
        }
    }
    // Normalize each row to a unit diagonal and read the inverse off the
    // right half.
    let mut out = [[0.0; 3]; 3];
    for row in 0..3 {
        let diag = *m.get(row)?.get(row)?;
        if !diag.is_finite() || diag == 0.0 {
            return None;
        }
        for col in 0..3 {
            let entry = *m.get(row)?.get(col + 3)? / diag;
            *out.get_mut(row)?.get_mut(col)? = entry;
        }
    }
    Some(out)
}

/// A Newton refinement of the certified crossing from the box midpoint `c`.
///
/// The Krawczyk proof guarantees a unique root of the augmented system in the
/// box and a contraction on it, so a few float Newton steps from `c` converge
/// to that root. The certificate is the proof, not the float iteration; this
/// only sharpens the recorded location toward the proven crossing.
fn refine_point(sys: &AugmentedFF<'_>, c: Point3) -> Point3 {
    let mut p = c;
    for _ in 0..MAX_NEWTON_STEPS {
        let x = [p.x, p.y, p.z];
        let Some(y) = sys.preconditioner(&x) else {
            break;
        };
        let f = sys.f_point(&x);
        let [f0, f1, f2] = f;
        let [[y00, y01, y02], [y10, y11, y12], [y20, y21, y22]] = y;
        let step = Vector3::new(
            y00 * f0.mid() + y01 * f1.mid() + y02 * f2.mid(),
            y10 * f0.mid() + y11 * f1.mid() + y12 * f2.mid(),
            y20 * f0.mid() + y21 * f1.mid() + y22 * f2.mid(),
        );
        let next = p - step;
        let correction = (next - p).magnitude();
        if !correction.is_finite() || correction <= NEWTON_TOL {
            return next;
        }
        p = next;
    }
    p
}

/// How many Newton steps refine a certified crossing. The Krawczyk contraction
/// makes this a fixed small budget, not a geometry-dependent loop.
const MAX_NEWTON_STEPS: usize = 8;

/// The Newton correction floor below which the iterate is taken as the
/// crossing.
/// H-3: a dimensionless convergence floor on a float Newton iterate, not a
/// model-space length.
const NEWTON_TOL: f64 = 1.0e-10; // H-3: dimensionless Newton convergence floor, not a length

/// Whether the interval lies strictly away from zero.
fn excludes_zero(i: Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// The float midpoint of a box.
fn box_midpoint(b: &Box3) -> Point3 {
    Point3::new(b.x.mid(), b.y.mid(), b.z.mid())
}

/// Splits a box on its widest axis (ties toward the lowest axis index) at the
/// axis midpoint, as a convex combination so the halves hull back to the
/// original even near overflow. `None` when the box cannot bisect: its widest
/// axis is at or below `tau`, or its midpoint rounds onto an edge (f64
/// resolution).
fn bisect_pair(b: &Box3, tau: f64) -> Option<(Box3, Box3)> {
    let wx = b.x.sup() - b.x.inf();
    let wy = b.y.sup() - b.y.inf();
    let wz = b.z.sup() - b.z.inf();
    let max = wx.max(wy).max(wz);
    if !max.is_finite() || max <= tau {
        return None;
    }
    let axis = if max == wx {
        0
    } else if max == wy {
        1
    } else {
        2
    };
    let (inf, sup) = match axis {
        0 => (b.x.inf(), b.x.sup()),
        1 => (b.y.inf(), b.y.sup()),
        _ => (b.z.inf(), b.z.sup()),
    };
    let mid = 0.5 * inf + 0.5 * sup;
    if mid == inf || mid == sup {
        return None;
    }
    let mut lo = *b;
    let mut hi = *b;
    match axis {
        0 => {
            lo.x = Interval::try_from((inf, mid)).unwrap_or(lo.x);
            hi.x = Interval::try_from((mid, sup)).unwrap_or(hi.x);
        }
        1 => {
            lo.y = Interval::try_from((inf, mid)).unwrap_or(lo.y);
            hi.y = Interval::try_from((mid, sup)).unwrap_or(hi.y);
        }
        _ => {
            lo.z = Interval::try_from((inf, mid)).unwrap_or(lo.z);
            hi.z = Interval::try_from((mid, sup)).unwrap_or(hi.z);
        }
    }
    Some((lo, hi))
}

/// Spend since entry: the initial budget minus what remains (mirrored from
/// krawczyk). Never the REMAINING budget as `spent` — that hides exhaustion.
fn spent(initial: &Budget, budget: &Budget) -> Budget {
    Budget {
        subdiv: initial.subdiv - budget.subdiv,
        newton: initial.newton - budget.newton,
        depth: initial.depth - budget.depth,
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geometry::specifieds::{Cylinder, Sphere};

    /// Residual bound on the unit-scale witness values of a certified
    /// crossing, never a model-space length.
    const RESIDUAL: f64 = 1.0e-9; // H-3: unit-scale residual tolerance on f values, not a length

    /// The cover's resolution floor: a model-space length by definition.
    const TAU: f64 = 1.0e-2; // H-3: resolution floor, a model-space length

    /// Build a test interval, degrading to EMPTY (and failing its assertion)
    /// rather than panicking on a malformed bound.
    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_from((lo, hi)).unwrap_or(Interval::EMPTY)
    }

    /// The validated UNIT z-cylinder at the origin, matching the `Outcome`
    /// constructor's shape.
    fn unit_cylinder() -> Cylinder {
        Cylinder::new(Point3::origin(), 1.0)
            .expect("a positive finite radius is always a valid cylinder")
            .value
    }

    #[test]
    fn transversal_pair_yields_proven_points_on_curve() {
        // The UNIT z-cylinder at the origin meets the sphere center (3,0,0)
        // radius 3 in the smooth curve z² = 6x − 1 (subtract the cylinder
        // equation from the sphere's). The box hugs the y>0, z>0 branch, with
        // y bounded strictly away from 0 so the gradient cross product's
        // z-component excludes zero and the box is not screened as singular.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(3.0, 0.0, 0.0), 3.0);
        let domain = Box3 {
            x: iv(0.2, 1.0),
            y: iv(0.1, 0.95),
            z: iv(0.1, 2.4),
        };
        let mut budget = Budget::new(4096, 0, 0);
        let cover = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("a healthy budget certifies the transversal crossings");
        assert!(
            !cover.value.points.is_empty(),
            "the transversal pair yields certified points"
        );
        for p in &cover.value.points {
            let f_cyl = p.x * p.x + p.y * p.y - 1.0;
            let f_sph = (p.x - 3.0) * (p.x - 3.0) + p.y * p.y + p.z * p.z - 9.0;
            assert!(
                f_cyl.abs() <= RESIDUAL && f_sph.abs() <= RESIDUAL,
                "certified point {p:?} has residuals {f_cyl} {f_sph}"
            );
        }
        assert!(
            cover.value.unresolved_boxes.len() < 4096,
            "unresolved leaves stay bounded: {}",
            cover.value.unresolved_boxes.len()
        );
    }

    #[test]
    fn tangent_pair_classifies_singular() {
        // The sphere center (2,0,0) radius 1 is tangent to the cylinder at
        // exactly (1,0,0): both equations vanish there and the gradients
        // (2,0,0) and (−2,0,0) are antiparallel, so a box around the tangency
        // screens singular rather than probing.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0);
        let domain = Box3 {
            x: iv(0.5, 1.5),
            y: iv(-0.5, 0.5),
            z: iv(-0.5, 0.5),
        };
        let mut budget = Budget::new(1024, 0, 0);
        let cover = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("a tangent pair classifies, never probes");
        let tangency = Point3::new(1.0, 0.0, 0.0);
        assert!(
            cover
                .value
                .singular_boxes
                .iter()
                .any(|b| b.contains(tangency)),
            "some singular box contains the tangency (1,0,0)"
        );
    }

    #[test]
    fn disjoint_pair_proves_empty() {
        // The sphere center (10,0,0) radius 1 stays ≥ 8 away from every
        // cylinder-wall point, so the sphere's enclosure excludes zero over
        // the whole wall region and the cover prunes on rule (a) alone.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(10.0, 0.0, 0.0), 1.0);
        let domain = Box3 {
            x: iv(0.0, 1.0),
            y: iv(-1.0, 1.0),
            z: iv(-2.5, 2.5),
        };
        let mut budget = Budget::new(1024, 0, 0);
        let cover = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("a disjoint pair proves empty");
        assert!(cover.value.points.is_empty());
        assert!(cover.value.singular_boxes.is_empty());
        assert!(cover.value.unresolved_boxes.is_empty());
    }

    #[test]
    fn empty_boxes_prune_by_interval_exclusion() {
        // The transversal pair again, but the domain box sits entirely off the
        // cylinder wall: this path must exit on rule (a) alone.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(3.0, 0.0, 0.0), 3.0);
        let domain = Box3 {
            x: iv(3.0, 4.0),
            y: iv(3.0, 4.0),
            z: iv(0.0, 1.0),
        };
        let mut budget = Budget::new(1024, 0, 0);
        let cover = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("an off-wall box prunes by interval exclusion");
        assert!(cover.value.points.is_empty());
        assert!(cover.value.singular_boxes.is_empty());
        assert!(cover.value.unresolved_boxes.is_empty());
    }
}
