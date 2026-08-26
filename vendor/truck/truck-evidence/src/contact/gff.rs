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
//! probe is a **2×2 z-slab Krawczyk system** (packet amendment r2): decompose
//! the search box's z-range into leaves; for each z-leaf, at its mid-plane
//! `z0`, solve
//!
//! ```text
//! F(x, y) = [ f1(x, y, z0), f2(x, y, z0) ]   over the (x, y) box
//! ```
//!
//! A `KrawczykProof::Unique` proves EXACTLY ONE crossing of C through the
//! slab's mid-plane. The Jacobian is the 2×2 `∂(f1,f2)/∂(x,y)`; for the
//! z-aligned quadric pairs this stage exists for, its determinant is
//! `4(y·cx − x·cy)`-type — non-singular exactly away from the singular locus.
//! Slabs whose determinant enclosure contains zero classify as `Singular`.
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
    /// The box holds (part of) a singular locus: the slab Jacobian
    /// determinant enclosure contains zero AND neither field excludes zero
    /// on the box. Not further classified here.
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
    // The outer worklist is z-leaves (intervals partitioning domain.z).
    let mut z_stack: Vec<Interval> = vec![domain.z];
    while let Some(z_leaf) = z_stack.pop() {
        let z0 = z_leaf.mid();
        let slab = Box3 {
            x: domain.x,
            y: domain.y,
            z: z_leaf,
        };
        // (a) Interval exclusion: some field enclosure excludes zero.
        if excludes_zero(d1.implicit(&slab)) || excludes_zero(d2.implicit(&slab)) {
            continue;
        }
        // (b) Singularity screen: the 2×2 slab Jacobian determinant over the
        // slab. If it contains zero the gradients may be parallel somewhere in
        // the slab (tangency/singularity), so the slab classifies as singular
        // instead of probing.
        let [f1x, f1y, _] = d1.grad(&slab);
        let [f2x, f2y, _] = d2.grad(&slab);
        let det = f1x * f2y - f1y * f2x;
        if det.contains(0.0) {
            cover.singular_boxes.push(slab);
            continue;
        }
        // (c) Probe: the nested (x, y) worklist for this z-leaf.
        let sys = SlabFF { f1: d1, f2: d2, z0 };
        let mut xy_stack: Vec<[Interval; 2]> = vec![[domain.x, domain.y]];
        let mut z_bisected = false;
        while let Some(q) = xy_stack.pop() {
            // (d) The Krawczyk outcome decides this (x, y) leaf.
            match krawczyk::<2>(&sys, &q, budget) {
                Ok(Certified {
                    value: KrawczykProof::Unique,
                    ..
                }) => {
                    // Exactly one crossing of C through the slab mid-plane.
                    // The recorded point is the (x, y) box midpoint refined to
                    // the certified root; the Krawczyk proof is the
                    // certificate.
                    let [qx, qy] = q;
                    let m = Point3::new(qx.mid(), qy.mid(), z0);
                    cover.points.push(refine_point(&sys, m));
                }
                // NoRoot: no crossing through this slab leaf → Empty.
                Ok(Certified {
                    value: KrawczykProof::NoRoot,
                    ..
                }) => {}
                // The probe could not certify: bisect the (x, y) box
                // widest-axis-first (ties toward the lowest index), spending
                // budget; when the (x, y) box is at resolution, bisect the
                // z-leaf instead. A leaf that can bisect neither way is the
                // honest unresolved remainder.
                Err(Refusal::NumericallyUnresolved { .. }) => {
                    if let Some((lo, hi)) = bisect_xy(&q, tau) {
                        if budget.spend_subdiv(1).is_err() {
                            return Err(Refusal::NumericallyUnresolved {
                                spent: spent(&initial, budget),
                                witness: UnresolvedWitness::KrawczykIndeterminate,
                            });
                        }
                        xy_stack.push(lo);
                        xy_stack.push(hi);
                    } else if !z_bisected {
                        if let Some((lo, hi)) = bisect_interval(z_leaf, tau) {
                            if budget.spend_subdiv(1).is_err() {
                                return Err(Refusal::NumericallyUnresolved {
                                    spent: spent(&initial, budget),
                                    witness: UnresolvedWitness::KrawczykIndeterminate,
                                });
                            }
                            z_stack.push(lo);
                            z_stack.push(hi);
                            z_bisected = true;
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

/// The 2×2 z-slab probe system: `F(x, y) = [f1(x, y, z0), f2(x, y, z0)]`.
struct SlabFF<'a> {
    f1: &'a dyn ImplicitField,
    f2: &'a dyn ImplicitField,
    /// The slab's mid-plane height.
    z0: f64,
}

impl KrawczykSystem<2> for SlabFF<'_> {
    /// Point evaluation: both implicit fields wrapped as degenerate intervals
    /// at `(x, y, z0)`.
    fn f_point(&self, x: &[f64; 2]) -> [Interval; 2] {
        let [x0, x1] = *x;
        let boxed = Box3::point(Point3::new(x0, x1, self.z0));
        [self.f1.implicit(&boxed), self.f2.implicit(&boxed)]
    }

    /// The interval 2×2 Jacobian over the box: rows f1/f2, cols ∂/∂x ∂/∂y,
    /// evaluated over `q × [z0, z0]` (the slab mid-plane is degenerate in z).
    fn jacobian(&self, b: &[Interval; 2]) -> [[Interval; 2]; 2] {
        let [qx, qy] = *b;
        let boxed = Box3 {
            x: qx,
            y: qy,
            z: interval_at(self.z0),
        };
        let [f1x, f1y, _] = self.f1.grad(&boxed);
        let [f2x, f2y, _] = self.f2.grad(&boxed);
        [[f1x, f1y], [f2x, f2y]]
    }

    /// The EXACT float inverse of `mid(J)` by the 2×2 closed form
    /// `1/det · [[d, −b], [−c, a]]`. `None` when `|det|` is degenerate
    /// (krawczyk then bisects per its contract).
    fn preconditioner(&self, x: &[f64; 2]) -> Option<[[f64; 2]; 2]> {
        let [x0, x1] = *x;
        let boxed = Box3::point(Point3::new(x0, x1, self.z0));
        let [f1x, f1y, _] = self.f1.grad(&boxed);
        let [f2x, f2y, _] = self.f2.grad(&boxed);
        let a = f1x.mid();
        let b = f1y.mid();
        let c = f2x.mid();
        let d = f2y.mid();
        let det = a * d - b * c;
        if det.is_finite() && det != 0.0 {
            Some([[d / det, -b / det], [-c / det, a / det]])
        } else {
            None
        }
    }
}

/// A Newton refinement of the certified crossing from the (x, y) box midpoint.
///
/// The Krawczyk proof guarantees a unique root of the 2×2 slab system in the
/// box and a contraction on it, so a few float Newton steps from `c` (at the
/// fixed slab height `z0`) converge to that root. The certificate is the
/// proof, not the float iteration; this only sharpens the recorded location
/// toward the proven crossing.
fn refine_point(sys: &SlabFF<'_>, c: Point3) -> Point3 {
    let mut p = c;
    for _ in 0..MAX_NEWTON_STEPS {
        let x = [p.x, p.y];
        let Some(y) = sys.preconditioner(&x) else {
            break;
        };
        let f = sys.f_point(&x);
        let [f0, f1] = f;
        let [[y00, y01], [y10, y11]] = y;
        let dx = y00 * f0.mid() + y01 * f1.mid();
        let dy = y10 * f0.mid() + y11 * f1.mid();
        let next = Point3::new(p.x - dx, p.y - dy, p.z);
        let correction = ((p.x - next.x).powi(2) + (p.y - next.y).powi(2)).sqrt();
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

/// Splits a 2-D (x, y) box on its widest axis (ties toward the lowest axis
/// index) at the axis midpoint, as a convex combination so the halves hull
/// back to the original even near overflow. `None` when the box cannot
/// bisect: its widest axis is at or below `tau`, or its midpoint rounds onto
/// an edge (f64 resolution).
fn bisect_xy(q: &[Interval; 2], tau: f64) -> Option<([Interval; 2], [Interval; 2])> {
    let [qx, qy] = *q;
    let wx = qx.sup() - qx.inf();
    let wy = qy.sup() - qy.inf();
    let max = wx.max(wy);
    if !max.is_finite() || max <= tau {
        return None;
    }
    if max == wx {
        let (inf, sup) = (qx.inf(), qx.sup());
        let mid = 0.5 * inf + 0.5 * sup;
        if mid == inf || mid == sup {
            return None;
        }
        let lo_x = Interval::try_from((inf, mid)).unwrap_or(qx);
        let hi_x = Interval::try_from((mid, sup)).unwrap_or(qx);
        Some(([lo_x, qy], [hi_x, qy]))
    } else {
        let (inf, sup) = (qy.inf(), qy.sup());
        let mid = 0.5 * inf + 0.5 * sup;
        if mid == inf || mid == sup {
            return None;
        }
        let lo_y = Interval::try_from((inf, mid)).unwrap_or(qy);
        let hi_y = Interval::try_from((mid, sup)).unwrap_or(qy);
        Some(([qx, lo_y], [qx, hi_y]))
    }
}

/// Splits a z-leaf interval at its midpoint. `None` when the leaf is at or
/// below `tau`, or its midpoint rounds onto an edge (f64 resolution).
fn bisect_interval(z: Interval, tau: f64) -> Option<(Interval, Interval)> {
    let width = z.sup() - z.inf();
    if !width.is_finite() || width <= tau {
        return None;
    }
    let mid = 0.5 * z.inf() + 0.5 * z.sup();
    if mid == z.inf() || mid == z.sup() {
        return None;
    }
    let lo = Interval::try_from((z.inf(), mid)).unwrap_or(z);
    let hi = Interval::try_from((mid, z.sup())).unwrap_or(z);
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
}
