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
//! The contact curve is `C = { p : f1(p) = 0, f2(p) = 0 }`. The certified
//! probe is a **chart-aware 2×2 slab Krawczyk system** (BG-SOL-S7-GFF-CHART):
//! one of the three coordinate charts is certified regular over the whole
//! search box when the corresponding 2×2 minor of `grad(f1) × grad(f2)`
//! excludes zero on it (packet decision 2); the slab worklist then decomposes
//! the *fixed* coordinate's range into leaves, and at each leaf's mid-plane
//! solves the two remaining coordinates
//!
//! ```text
//! F(u, v) = [ f1(...), f2(...) ]   over the two solver-coordinate intervals
//! ```
//!
//! A `KrawczykProof::Unique` proves EXACTLY ONE crossing of C through the
//! slab's mid-plane. The Jacobian is the 2×2 minor of the chosen chart, which
//! excludes zero on the whole domain — a zero xy minor is a chart artifact of
//! a horizontal tangent, not a singular contact, and is recovered through the
//! xz or yz chart. A box on which every minor merely contains zero stays in
//! `singular_boxes` for later locus-dimension classification; it is never
//! called proven rank deficiency.
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
use truck_base::cgmath64::Point3;
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
    /// The box holds (part of) a singular locus: no 2×2 coordinate chart is
    /// certified regular over it AND neither field excludes zero on the box.
    /// Not further classified here.
    Singular,
    /// Krawczyk proved exactly one crossing of C through the slab mid-plane.
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
    // Select the certified regular chart ONCE over the entire input domain
    // (BG-SOL-S7-GFF-CHART decision 2): the 2×2 minor of the chosen chart
    // excludes zero on `domain`, so the chart stays valid on every child leaf
    // and no leaf-level singular re-screen is needed (decision 3).
    let Some(axis) = select_chart(d1, d2, domain) else {
        // No coordinate chart is certified regular over the box: every 2×2
        // minor merely contains zero. This is NOT proven rank deficiency —
        // interval dependency or an overly broad box may be responsible — so
        // the box stays in `singular_boxes` for later locus-dimension
        // classification (decision 1). A field whose enclosure excludes zero
        // over the whole box still proves it empty.
        if excludes_zero(d1.implicit(domain)) || excludes_zero(d2.implicit(domain)) {
            return Ok(Certified::new(cover, certificate(budget)));
        }
        cover.singular_boxes.push(*domain);
        return Ok(Certified::new(cover, certificate(budget)));
    };
    let fixed = match axis {
        FixedAxis::X => domain.x,
        FixedAxis::Y => domain.y,
        FixedAxis::Z => domain.z,
    };
    // The outer worklist is leaves of the FIXED coordinate; the inner Krawczyk
    // box holds the other two domain intervals.
    let mut fixed_stack: Vec<Interval> = vec![fixed];
    while let Some(fixed_leaf) = fixed_stack.pop() {
        let sys = ChartFF {
            f1: d1,
            f2: d2,
            axis,
            fixed: fixed_leaf.mid(),
        };
        let slab = reconstruct_slab(domain, axis, fixed_leaf);
        // (a) Interval exclusion: some field enclosure excludes zero on the
        // reconstructed 3-D slab.
        if excludes_zero(d1.implicit(&slab)) || excludes_zero(d2.implicit(&slab)) {
            continue;
        }
        // (b) Probe: the nested two-solver-coordinate worklist for this leaf.
        let mut inner_stack: Vec<[Interval; 2]> = vec![solver_box(domain, axis)];
        let mut fixed_bisected = false;
        while let Some(q) = inner_stack.pop() {
            // (c) The Krawczyk outcome decides this inner leaf.
            match krawczyk::<2>(&sys, &q, budget) {
                Ok(Certified {
                    value: KrawczykProof::Unique,
                    ..
                }) => {
                    // Exactly one crossing of C through the slab mid-plane.
                    // The recorded point is the solver-box midpoint refined to
                    // the certified root; the Krawczyk proof is the
                    // certificate.
                    let [q0, q1] = q;
                    let m = sys.rebuild(q0.mid(), q1.mid());
                    cover.points.push(refine_point(&sys, m));
                }
                // NoRoot: no crossing through this slab leaf → Empty.
                Ok(Certified {
                    value: KrawczykProof::NoRoot,
                    ..
                }) => {}
                // The probe could not certify: bisect the inner box
                // widest-axis-first (ties toward the first solver coordinate),
                // spending budget; when the inner box is at resolution, bisect
                // the fixed-coordinate leaf instead. A leaf that can bisect
                // neither way is the honest unresolved remainder.
                Err(Refusal::NumericallyUnresolved { .. }) => {
                    if let Some((lo, hi)) = bisect_solver(&q, tau) {
                        if budget.spend_subdiv(1).is_err() {
                            return Err(Refusal::NumericallyUnresolved {
                                spent: spent(&initial, budget),
                                witness: UnresolvedWitness::KrawczykIndeterminate,
                            });
                        }
                        inner_stack.push(lo);
                        inner_stack.push(hi);
                    } else if !fixed_bisected {
                        if let Some((lo, hi)) = bisect_interval(fixed_leaf, tau) {
                            if budget.spend_subdiv(1).is_err() {
                                return Err(Refusal::NumericallyUnresolved {
                                    spent: spent(&initial, budget),
                                    witness: UnresolvedWitness::KrawczykIndeterminate,
                                });
                            }
                            fixed_stack.push(lo);
                            fixed_stack.push(hi);
                            fixed_bisected = true;
                        } else {
                            cover.unresolved_boxes.push(slab);
                        }
                    } else {
                        cover.unresolved_boxes.push(slab);
                    }
                }
                // krawczyk's other refusal is `Empty` (an empty or non-finite
                // start box): the leaf decides nothing, treat as Empty.
                Err(_) => {}
            }
        }
    }
    Ok(Certified::new(cover, certificate(budget)))
}

/// The fixed coordinate of a certified regular chart: the slab worklist
/// decomposes this coordinate's range into leaves and solves the 2×2 Krawczyk
/// system over the other two coordinates at each leaf's mid-plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixedAxis {
    /// Fixed X: solve for (y, z).
    X,
    /// Fixed Y: solve for (x, z).
    Y,
    /// Fixed Z: solve for (x, y) (the pre-chart z-slab behavior).
    Z,
}

/// The certified distance of a 2×2 minor from zero: the lower bound when the
/// interval sits strictly above zero, the negated upper bound when strictly
/// below, and `None` when the minor merely contains zero (not certified
/// usable, BG-SOL-S7-GFF-CHART decision 1). A nonzero midpoint is not proof.
fn minor_distance(minor: Interval) -> Option<f64> {
    if minor.inf() > 0.0 {
        Some(minor.inf())
    } else if minor.sup() < 0.0 {
        Some(-minor.sup())
    } else {
        None
    }
}

/// Select the certified regular coordinate chart for the whole domain: the
/// usable 2×2 minor with the largest certified distance from zero, ties toward
/// the lowest fixed-axis order X, then Y, then Z (decision 2). `None` when no
/// minor excludes zero — the box is not provably regular in any chart. The
/// selection is order-insensitive: an f1/f2 swap negates all three minors,
/// preserving their distances and therefore the chosen chart.
fn select_chart(
    f1: &dyn ImplicitField,
    f2: &dyn ImplicitField,
    domain: &Box3,
) -> Option<FixedAxis> {
    let [a_x, a_y, a_z] = f1.grad(domain);
    let [b_x, b_y, b_z] = f2.grad(domain);
    // The three outward-rounded 2×2 minors, equivalently the components of
    // grad(f1) × grad(f2).
    let minors = [
        (FixedAxis::X, a_y * b_z - a_z * b_y), // fixed X: solve (y, z)
        (FixedAxis::Y, a_z * b_x - a_x * b_z), // fixed Y: solve (x, z)
        (FixedAxis::Z, a_x * b_y - a_y * b_x), // fixed Z: solve (x, y)
    ];
    let mut best: Option<(FixedAxis, f64)> = None;
    for (axis, minor) in minors {
        if let Some(distance) = minor_distance(minor) {
            // Strict `>` keeps the earlier axis (X, then Y, then Z) on ties.
            let better = match best {
                None => true,
                Some((_, d)) => distance > d,
            };
            if better {
                best = Some((axis, distance));
            }
        }
    }
    best.map(|(axis, _)| axis)
}

/// Reconstruct the 3-D slab of `domain` with `axis` pinned to the leaf and the
/// other two coordinates at their domain intervals.
fn reconstruct_slab(domain: &Box3, axis: FixedAxis, leaf: Interval) -> Box3 {
    match axis {
        FixedAxis::X => Box3 {
            x: leaf,
            y: domain.y,
            z: domain.z,
        },
        FixedAxis::Y => Box3 {
            x: domain.x,
            y: leaf,
            z: domain.z,
        },
        FixedAxis::Z => Box3 {
            x: domain.x,
            y: domain.y,
            z: leaf,
        },
    }
}

/// The inner Krawczyk box for a chart: the two solver-coordinate intervals, in
/// the chart's solver order.
fn solver_box(domain: &Box3, axis: FixedAxis) -> [Interval; 2] {
    match axis {
        FixedAxis::X => [domain.y, domain.z],
        FixedAxis::Y => [domain.x, domain.z],
        FixedAxis::Z => [domain.x, domain.y],
    }
}

/// The 2×2 slab probe system for a fixed regular chart: `F = [f1, f2]`
/// restricted to the mid-slab of the fixed coordinate, over the two solver
/// coordinates. `f_point`, `jacobian`, the exact 2×2 closed-form inverse
/// preconditioner, and the Newton refinement all use the same three-way
/// mapping (decision 3).
struct ChartFF<'a> {
    f1: &'a dyn ImplicitField,
    f2: &'a dyn ImplicitField,
    /// The fixed coordinate of the chart.
    axis: FixedAxis,
    /// The mid-slab value of the fixed coordinate.
    fixed: f64,
}

impl ChartFF<'_> {
    /// The 3-D point for the two solver coordinates at the fixed mid-slab.
    fn rebuild(&self, u: f64, v: f64) -> Point3 {
        match self.axis {
            FixedAxis::X => Point3::new(self.fixed, u, v),
            FixedAxis::Y => Point3::new(u, self.fixed, v),
            FixedAxis::Z => Point3::new(u, v, self.fixed),
        }
    }

    /// The two solver coordinates of a 3-D point (the non-fixed axes).
    fn solver_coords(&self, p: Point3) -> [f64; 2] {
        match self.axis {
            FixedAxis::X => [p.y, p.z],
            FixedAxis::Y => [p.x, p.z],
            FixedAxis::Z => [p.x, p.y],
        }
    }

    /// The reconstructed 3-D slab over a solver box: the fixed axis is
    /// degenerate at the mid-slab value, the other two are the solver
    /// intervals.
    fn slab_box(&self, b: &[Interval; 2]) -> Box3 {
        let [u, v] = *b;
        match self.axis {
            FixedAxis::X => Box3 {
                x: interval_at(self.fixed),
                y: u,
                z: v,
            },
            FixedAxis::Y => Box3 {
                x: u,
                y: interval_at(self.fixed),
                z: v,
            },
            FixedAxis::Z => Box3 {
                x: u,
                y: v,
                z: interval_at(self.fixed),
            },
        }
    }
}

impl KrawczykSystem<2> for ChartFF<'_> {
    /// Point evaluation: both implicit fields wrapped as degenerate intervals
    /// at the rebuilt slab mid-plane point.
    fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
        let [u, v] = *x;
        let boxed = Box3::point(self.rebuild(u, v));
        [self.f1.implicit(&boxed), self.f2.implicit(&boxed)]
    }

    /// The interval 2×2 Jacobian over the solver box: rows f1/f2, columns the
    /// two solver coordinates, evaluated over the slab whose fixed axis is
    /// degenerate at the mid-slab value.
    fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
        let boxed = self.slab_box(b);
        let [f1x, f1y, f1z] = self.f1.grad(&boxed);
        let [f2x, f2y, f2z] = self.f2.grad(&boxed);
        match self.axis {
            FixedAxis::X => [[f1y, f1z], [f2y, f2z]],
            FixedAxis::Y => [[f1x, f1z], [f2x, f2z]],
            FixedAxis::Z => [[f1x, f1y], [f2x, f2y]],
        }
    }

    /// The EXACT float inverse of `mid(J)` by the 2×2 closed form
    /// `1/det · [[d, −b], [−c, a]]`. `None` when `|det|` is degenerate
    /// (krawczyk then bisects per its contract).
    fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
        let [u, v] = *x;
        let boxed = Box3::point(self.rebuild(u, v));
        let [f1x, f1y, f1z] = self.f1.grad(&boxed);
        let [f2x, f2y, f2z] = self.f2.grad(&boxed);
        let (a, b, c, d) = match self.axis {
            FixedAxis::X => (f1y.mid(), f1z.mid(), f2y.mid(), f2z.mid()),
            FixedAxis::Y => (f1x.mid(), f1z.mid(), f2x.mid(), f2z.mid()),
            FixedAxis::Z => (f1x.mid(), f1y.mid(), f2x.mid(), f2y.mid()),
        };
        let det = a * d - b * c;
        if det.is_finite() && det != 0.0 {
            Some([[d / det, -b / det], [-c / det, a / det]])
        } else {
            None
        }
    }
}

/// The successful certificate of a cover: interval method, empty props, actual
/// remaining budget, unbounded margin/modulus (BG-SOL-S7-GFF-CHART decision 4).
fn certificate(budget: &Budget) -> Certificate {
    Certificate {
        props: PropMap::new(),
        method: Method::Interval,
        budget_left: *budget,
        margin: Margin::UNBOUNDED,
        modulus: Modulus::Unbounded,
    }
}

/// A Newton refinement of the certified crossing from the solver-box midpoint.
///
/// The Krawczyk proof guarantees a unique root of the 2×2 slab system in the
/// box and a contraction on it, so a few float Newton steps from `c` (at the
/// fixed mid-slab value) converge to that root. The certificate is the proof,
/// not the float iteration; this only sharpens the recorded location toward
/// the proven crossing.
fn refine_point(sys: &ChartFF<'_>, c: Point3) -> Point3 {
    let mut p = c;
    for _ in 0..MAX_NEWTON_STEPS {
        let [u, v] = sys.solver_coords(p);
        let Some(y) = sys.preconditioner(&[u, v]) else {
            break;
        };
        let f = sys.f_point(&[u, v]);
        let [f0, f1] = f;
        let [[y00, y01], [y10, y11]] = y;
        let du = y00 * f0.mid() + y01 * f1.mid();
        let dv = y10 * f0.mid() + y11 * f1.mid();
        let nu = u - du;
        let nv = v - dv;
        let correction = ((u - nu).powi(2) + (v - nv).powi(2)).sqrt();
        if !correction.is_finite() || correction <= NEWTON_TOL {
            return sys.rebuild(nu, nv);
        }
        p = sys.rebuild(nu, nv);
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

/// Splits a 2-D solver box on its widest axis (ties toward the first solver
/// coordinate) at the axis midpoint, as a convex combination so the halves
/// hull back to the original even near overflow. `None` when the box cannot
/// bisect: its widest axis is at or below `tau`, or its midpoint rounds onto
/// an edge (f64 resolution).
fn bisect_solver(q: &[Interval; 2], tau: f64) -> Option<([Interval; 2], [Interval; 2])> {
    let [q0, q1] = *q;
    let w0 = q0.sup() - q0.inf();
    let w1 = q1.sup() - q1.inf();
    let max = w0.max(w1);
    if !max.is_finite() || max <= tau {
        return None;
    }
    if max == w0 {
        let (inf, sup) = (q0.inf(), q0.sup());
        let mid = 0.5 * inf + 0.5 * sup;
        if mid == inf || mid == sup {
            return None;
        }
        let lo_0 = Interval::try_from((inf, mid)).unwrap_or(q0);
        let hi_0 = Interval::try_from((mid, sup)).unwrap_or(q0);
        Some(([lo_0, q1], [hi_0, q1]))
    } else {
        let (inf, sup) = (q1.inf(), q1.sup());
        let mid = 0.5 * inf + 0.5 * sup;
        if mid == inf || mid == sup {
            return None;
        }
        let lo_1 = Interval::try_from((inf, mid)).unwrap_or(q1);
        let hi_1 = Interval::try_from((mid, sup)).unwrap_or(q1);
        Some(([q0, lo_1], [q0, hi_1]))
    }
}

/// Splits a fixed-coordinate leaf interval at its midpoint. `None` when the
/// leaf is at or below `tau`, or its midpoint rounds onto an edge (f64
/// resolution).
fn bisect_interval(leaf: Interval, tau: f64) -> Option<(Interval, Interval)> {
    let width = leaf.sup() - leaf.inf();
    if !width.is_finite() || width <= tau {
        return None;
    }
    let mid = 0.5 * leaf.inf() + 0.5 * leaf.sup();
    if mid == leaf.inf() || mid == leaf.sup() {
        return None;
    }
    let lo = Interval::try_from((leaf.inf(), mid)).unwrap_or(leaf);
    let hi = Interval::try_from((mid, leaf.sup())).unwrap_or(leaf);
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
    use truck_base::cgmath64::{EuclideanSpace, InnerSpace};
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
        // y bounded strictly away from 0 so the 2×2 slab determinant
        // det = 4(y·cx − x·cy) = 12y excludes zero and the slab is not
        // screened as singular.
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
        // (2,0,0) and (−2,0,0) are antiparallel, so a slab around the
        // tangency screens singular rather than probing.
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

    /// The certified horizontal-turn witness of BG-SOL-S7-GFF-CHART: the unit
    /// z-cylinder meets the sphere center (3,0,0) radius 3 in the smooth curve
    /// z² = 6x − 1. Near p = (1, 0, √5) the tangent is horizontal: the xy
    /// minor m_z = 12y contains zero on the box, but the xz minor m_y = −4xz
    /// excludes zero (|m_y| ≥ 4·0.9·2.1 = 7.56), so the Y-fixed chart is
    /// certified regular and the slice y = 0 holds the unique crossing
    /// (x, z) = (1, √5).
    fn horizontal_turn_witness() -> (Cylinder, Sphere, Box3) {
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(3.0, 0.0, 0.0), 3.0);
        let domain = Box3 {
            x: iv(0.9, 1.1),
            y: iv(-0.1, 0.1),
            z: iv(2.1, 2.3),
        };
        (cyl, sph, domain)
    }

    #[test]
    fn adaptive_minor_recovers_regular_horizontal_turn() {
        // The old fixed-z probe equates the xy minor (m_z = 12y, zero on this
        // box) with a singular contact and reports the box singular; the
        // chart-aware probe certifies the Y-fixed chart and recovers the
        // crossing instead. This test fails on the pre-packet implementation.
        let (cyl, sph, domain) = horizontal_turn_witness();
        let mut budget = Budget::new(4096, 0, 0);
        let cover = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("a healthy budget certifies the regular horizontal turn");
        assert!(
            !cover.value.points.is_empty(),
            "the Y-fixed chart recovers certified crossings of the horizontal turn"
        );
        assert!(
            cover.value.singular_boxes.is_empty(),
            "a zero xy minor is a chart artifact, not a singular contact"
        );
        assert!(
            cover.value.unresolved_boxes.is_empty(),
            "a healthy budget certifies every leaf"
        );
        for p in &cover.value.points {
            let f_cyl = p.x * p.x + p.y * p.y - 1.0;
            let f_sph = (p.x - 3.0) * (p.x - 3.0) + p.y * p.y + p.z * p.z - 9.0;
            assert!(
                f_cyl.abs() <= RESIDUAL && f_sph.abs() <= RESIDUAL,
                "certified point {p:?} has residuals {f_cyl} {f_sph}"
            );
        }
    }

    /// Unit-scale residual between two certified point sets of the same
    /// horizontal-turn cover, for the order-insensitive comparison.
    const ORDER_RESIDUAL: f64 = 1.0e-6; // H-3: unit-scale residual between two certified point sets, not a length

    #[test]
    fn adaptive_minor_is_order_insensitive() {
        // The chart selection reads only the minors' certified distances, and
        // swapping f1/f2 negates all three minors while preserving their
        // distances — so both orders choose the Y-fixed chart and certify the
        // same crossing set order-insensitively.
        let (cyl, sph, domain) = horizontal_turn_witness();
        let mut budget = Budget::new(4096, 0, 0);
        let fwd = cover_branch(&cyl, &sph, &domain, TAU, &mut budget)
            .expect("the forward order certifies under healthy budget");
        let mut budget = Budget::new(4096, 0, 0);
        let rev = cover_branch(&sph, &cyl, &domain, TAU, &mut budget)
            .expect("the reversed order certifies under healthy budget");
        for cover in [&fwd.value, &rev.value] {
            assert!(
                cover.singular_boxes.is_empty(),
                "a regular chart certifies no singular boxes in either order"
            );
            assert!(
                cover.unresolved_boxes.is_empty(),
                "a healthy budget certifies every leaf in either order"
            );
        }
        assert_eq!(
            fwd.value.points.len(),
            rev.value.points.len(),
            "both orders certify the same number of crossings"
        );
        for p in &fwd.value.points {
            assert!(
                rev.value
                    .points
                    .iter()
                    .any(|q| (*p - *q).magnitude() <= ORDER_RESIDUAL),
                "forward point {p:?} has no match in the reversed cover"
            );
        }
        for q in &rev.value.points {
            assert!(
                fwd.value
                    .points
                    .iter()
                    .any(|p| (*p - *q).magnitude() <= ORDER_RESIDUAL),
                "reversed point {q:?} has no match in the forward cover"
            );
        }
    }

    #[test]
    fn adaptive_minor_true_tangency_remains_singular() {
        // The unit cylinder is tangent to the sphere center (2,0,0) radius 1
        // at (1,0,0). Every component of grad(f1) × grad(f2) vanishes at the
        // tangency, so on a box enclosing it all three 2×2 minors contain
        // zero: no chart is certified regular, the box stays singular, and no
        // regular crossing is falsely certified.
        let cyl = unit_cylinder();
        let sph = Sphere::new(Point3::new(2.0, 0.0, 0.0), 1.0);
        let domain = Box3 {
            x: iv(0.9, 1.1),
            y: iv(-0.1, 0.1),
            z: iv(-0.1, 0.1),
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
        assert!(
            cover.value.points.is_empty(),
            "a true tangency certifies no regular crossing"
        );
    }
}
