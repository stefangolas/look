//! BG-ANA-001-PCONE: plane × cone — conic sections.
//!
//! A plane cutting the canonical double cone (apex at `apex`, opening along
//! `+z`, half angle α, `v` unbounded both ways) produces the conic sections:
//! a **circle** (plane perpendicular to the axis), an **ellipse** (plane
//! steeper than the generators, off the apex), a **parabola** (plane parallel
//! to exactly one generator), a **two-branched hyperbola** (one branch per
//! nappe), or a degenerate section — the apex point, one generator line, or
//! two generator lines (a plane through the apex).
//!
//! # The technique (pre-decided by the packet)
//!
//! The classification runs on **exact interval predicates over the carrier
//! parameters**, never on the 2D discriminant (`Δ2 = B² − 4AC` is a multi-step
//! polynomial in non-dyadic components whose parabola-boundary value is
//! `[−ε, 0]`/`[0, ε]`, never the `[0, 0]` that [`decisively_zero`] requires —
//! see the 2026-08-20 spec amendment). The raw normal `N = u_axis × v_axis`
//! and the apex-through height `h = (apex − origin) · n̂` are computed in inari
//! from the carrier's f64 parameters; `three_way(Nz², t²·(Nx² + Ny²))` picks
//! the family (ellipse / hyperbola / parabola) and `h` decides the degeneracy.
//! An undecidable enclosure refuses as `NumericallyUnresolved` instead of
//! guessing.
//!
//! The 2D reduction is what *emits* the geometry: substituting the plane's
//! orthonormal parameterization `o + u·û + v·v̂` into the cone equation
//! `(x − p.x)² + (y − p.y)² = (z − p.z)²·t²` yields a quadratic
//! `A u² + B uv + C v² + D u + E v + F = 0` whose coefficients are degree-≤2
//! polynomials in the carrier parameters. Its classification is deliberately
//! **not** used to pick the arm; the emission (centre solve, eigen-frame,
//! semi-axes) runs in f64 against the closed form of the family the invariant
//! chose.
//!
//! # What `Method::Exact` means here
//!
//! The arm selection is exact: decisive interval predicates on the f64 carrier
//! parameters (`three_way` on the family invariant, `decisively_zero` /
//! `excludes_zero` on the apex-through height). The emitted conic is the
//! closed-form section under the plane's affine frame; its coordinates are
//! computed in f64, and the obligation BG-ANA-002 places on the cell is "lies
//! on both carriers to machine precision", asserted with an H-3-commented
//! slack in the tests. There is no `τ_rep` anywhere in this module.

use crate::analytic::{AnalyticIntersection, AnalyticOutcome, ExactCurve};
use inari::Interval;
use std::cmp::Ordering;
use std::f64::consts::TAU;
use truck_base::cgmath64::{InnerSpace, Matrix4, Point3, Vector3, Vector4};
use truck_base::evidence::{
    Budget, Certificate, Certified, Margin, Method, Modulus, Prop, PropMap, Refusal, Truth,
    UnresolvedWitness,
};
use truck_geometry::decorators::{Processor, TrimmedCurve};
use truck_geometry::specifieds::{Cone, Line, Plane, UnitCircle, UnitHyperbola, UnitParabola};

/// A hyperbola branch has no natural finite parameter range (its parameter is
/// unbounded both ways); this symmetric trim is wide enough for sampling. The
/// choice is recorded in RESULT.json's `deviations`.
const HYPERBOLA_TRIM: (f64, f64) = (-3.0, 3.0);
/// The unit parabola likewise has an unbounded parameter; same symmetric
/// trim, also recorded in `deviations`.
const PARABOLA_TRIM: (f64, f64) = (-3.0, 3.0);

/// A degenerate interval enclosing a single `f64`. Non-finite input would make
/// an invalid interval, so it degrades to `EMPTY` instead of panicking (the
/// crate denies `unwrap`/`panic`; a NaN carrier parameter is a caller's bug).
fn interval_at(x: f64) -> Interval {
    Interval::try_from((x, x)).unwrap_or(Interval::EMPTY)
}

/// `[0, 0]`: the interval proves the enclosed value is exactly zero.
fn decisively_zero(i: &Interval) -> bool {
    i.inf() == 0.0 && i.sup() == 0.0
}

/// The interval lies entirely off zero on one side: the enclosed value is
/// decisively nonzero.
fn excludes_zero(i: &Interval) -> bool {
    i.inf() > 0.0 || i.sup() < 0.0
}

/// Three-way comparison of two intervals: `Less` when `a` lies wholly below
/// `b`, `Greater` when wholly above, `Equal` when both are the same degenerate
/// point, and `None` when the enclosures overlap without pinning the
/// comparison down. Dyadic-clean inputs produce degenerate intervals, so exact
/// classifications stay exact; an enclosure that merely contains zero proves
/// nothing.
fn three_way(a: &Interval, b: &Interval) -> Option<Ordering> {
    if a.sup() < b.inf() {
        Some(Ordering::Less)
    } else if b.sup() < a.inf() {
        Some(Ordering::Greater)
    } else if a.inf() == a.sup() && b.inf() == b.sup() && a.inf() == b.inf() {
        Some(Ordering::Equal)
    } else {
        None
    }
}

/// Places a unit conic under the affine frame `(u, v, n, o)` with the
/// x-direction scaled by `ru` and the y-direction by `rv`: a point `(x, y, 0)`
/// of the unit curve lands at `o + ru·x·u + rv·y·v`. `n` is left unscaled, so
/// the placed conic lies in the plane through `o` spanned by `u` and `v`.
fn frame(u: Vector3, v: Vector3, n: Vector3, o: Point3, ru: f64, rv: f64) -> Matrix4 {
    Matrix4::from_cols(
        Vector4::new(u.x, u.y, u.z, 0.0),
        Vector4::new(v.x, v.y, v.z, 0.0),
        Vector4::new(n.x, n.y, n.z, 0.0),
        Vector4::new(o.x, o.y, o.z, 1.0),
    ) * Matrix4::from_nonuniform_scale(ru, rv, 1.0)
}

/// The exact section of the cone by a plane (BG-ANA-001).
///
/// Every `Ok` carries the exact certificate (`Method::Exact`, `AnalyticCarrier`
/// = `True`) built field-by-field at each return site (BG-EVD-002); every
/// `Err` is an undecidable predicate refusing rather than guessing.
pub fn plane_cone(plane: &Plane, cone: &Cone) -> AnalyticOutcome {
    let apex = cone.apex();
    let o = plane.origin();
    let t = cone.half_angle().tan();
    let n_hat = plane.normal();

    // --- Horizontal plane special case (component test, no intervals) ---
    //
    // A plane perpendicular to the cone axis cuts a circle at its own height;
    // `o.z == apex.z` is the apex-through case, where the circle collapses to
    // the apex point. Handled before the general reduction so the witnesses
    // stay dyadic end to end.
    if n_hat.z == 1.0 || n_hat.z == -1.0 {
        if o.z == apex.z {
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            return Ok(Certified::new(
                AnalyticIntersection::TangentPoint(apex),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ));
        }
        let radius = (o.z - apex.z).abs() * t;
        let center = Point3::new(apex.x, apex.y, o.z);
        let u = Vector3::unit_x();
        let v = n_hat.cross(u);
        let circle = Processor::with_transform(
            TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
            frame(u, v, n_hat, center, radius, radius),
        );
        let mut props = PropMap::new();
        props.set(Prop::AnalyticCarrier, Truth::True);
        return Ok(Certified::new(
            AnalyticIntersection::Curve(ExactCurve::Circle(circle)),
            Certificate {
                props,
                method: Method::Exact,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: Modulus::Unbounded,
            },
        ));
    }

    // --- The raw normal and the family invariant, in intervals ---
    //
    // `N = u_axis × v_axis` is computed componentwise in inari from the
    // carrier's f64 axis vectors (each difference is the carrier's own
    // `u_axis()`/`v_axis()`, enclosed before the cross — never an f64 cross
    // enclosed after the fact). `t` is the cone's own slope tan(α), exactly
    // the value `der_mn` uses for the radial law.
    let ua = plane.u_axis();
    let va = plane.v_axis();
    let ux = interval_at(ua.x);
    let uy = interval_at(ua.y);
    let uz = interval_at(ua.z);
    let vx = interval_at(va.x);
    let vy = interval_at(va.y);
    let vz = interval_at(va.z);
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let t_i = interval_at(t);
    // `N.z²` vs `t²·(N.x² + N.y²)`: the family boundary, exactly equivalent to
    // `|n̂.z|` vs `sin α` (for `cos α > 0`).
    let nz_sq = nz.sqr();
    let bound = t_i.sqr() * (nx.sqr() + ny.sqr());
    let Some(family) = three_way(&nz_sq, &bound) else {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    };

    // --- Degeneracy: the plane through the apex ---
    //
    // For this carrier the apex-through plane is the only degeneracy — the
    // classical result that a plane through the apex of a non-degenerate cone
    // degenerates the section. Measured as the signed distance of the apex
    // from the plane, `h = (apex − origin) · n̂`, computed componentwise in
    // inari — never an f64 dot product enclosed after the fact, which would
    // not contain the exact dot. (A plane with `o == apex` gives the zero
    // displacement and `h = [0, 0]` regardless of `n̂`'s dyadicity — that is
    // why the through-apex witnesses place `o` at the apex.)
    let d = apex - o;
    let h = interval_at(d.x) * interval_at(n_hat.x)
        + interval_at(d.y) * interval_at(n_hat.y)
        + interval_at(d.z) * interval_at(n_hat.z);
    let degenerate = decisively_zero(&h);
    let non_degenerate = excludes_zero(&h);
    if !degenerate && !non_degenerate {
        return Err(Refusal::NumericallyUnresolved {
            spent: Budget::new(0, 0, 0),
            witness: UnresolvedWitness::RootNotIsolated,
        });
    }

    // --- The plane's orthonormal frame ---
    //
    // `û = normalize(n̂ × ẑ)` is the horizontal direction lying *in* the plane
    // (`ẑ × (n̂ × ẑ)` would be the horizontal projection of `n̂`, which is not
    // in the plane for tilted planes). The exact-parallel case `n̂ = ±ẑ`
    // already returned above, so the cross is nonzero here.
    let u_hat = (n_hat.cross(Vector3::unit_z())).normalize();
    let v_hat = n_hat.cross(u_hat);

    // --- The 2D reduction's coefficients ---
    //
    // Substituting `P(u, v) = o + u·û + v·v̂` into the cone equation
    // `(x − p.x)² + (y − p.y)² = (z − p.z)²·t²` with `e = o − apex` gives the
    // quadratic `A u² + B uv + C v² + D u + E v + F`, each coefficient a
    // degree-≤2 polynomial in the carrier parameters. The interval versions
    // are computed here (all inari arithmetic, as the packet requires) and are
    // used for the squared-semi-axis sanity rule; the emission below solves in
    // f64 against the f64 evaluation of the same polynomials.
    let e = o - apex;
    let ux_i = interval_at(u_hat.x);
    let uy_i = interval_at(u_hat.y);
    let uz_i = interval_at(u_hat.z);
    let vx_i = interval_at(v_hat.x);
    let vy_i = interval_at(v_hat.y);
    let vz_i = interval_at(v_hat.z);
    let ex_i = interval_at(e.x);
    let ey_i = interval_at(e.y);
    let ez_i = interval_at(e.z);
    let t2 = t_i.sqr();
    let at2 = interval_at(2.0);
    let ai = ux_i * ux_i + uy_i * uy_i - t2 * uz_i * uz_i;
    let bi = at2 * (ux_i * vx_i + uy_i * vy_i - t2 * uz_i * vz_i);
    let ci = vx_i * vx_i + vy_i * vy_i - t2 * vz_i * vz_i;
    let di = at2 * (ex_i * ux_i + ey_i * uy_i - t2 * ez_i * uz_i);
    let ei = at2 * (ex_i * vx_i + ey_i * vy_i - t2 * ez_i * vz_i);
    let fi = ex_i * ex_i + ey_i * ey_i - t2 * ez_i * ez_i;

    let a = u_hat.x * u_hat.x + u_hat.y * u_hat.y - t * t * u_hat.z * u_hat.z;
    let b = 2.0 * (u_hat.x * v_hat.x + u_hat.y * v_hat.y - t * t * u_hat.z * v_hat.z);
    let c = v_hat.x * v_hat.x + v_hat.y * v_hat.y - t * t * v_hat.z * v_hat.z;
    let d = 2.0 * (e.x * u_hat.x + e.y * u_hat.y - t * t * e.z * u_hat.z);
    let e2d = 2.0 * (e.x * v_hat.x + e.y * v_hat.y - t * t * e.z * v_hat.z);
    let f = e.x * e.x + e.y * e.y - t * t * e.z * e.z;

    // Verification predicate for a solved generator direction `(c, s)` on the
    // azimuth circle: the line `(t·c, t·s, 1)` (proportional to
    // `(sin α·c, sin α·s, cos α)`) must lie in the plane, i.e. be
    // perpendicular to the raw normal. Computed in inari against the exact
    // interval normal; for the dyadic witnesses it is exactly `[0, 0]`.
    let in_plane = |c: f64, s: f64| t_i * (nx * interval_at(c) + ny * interval_at(s)) + nz;

    match family {
        Ordering::Greater => {
            // Ellipse family.
            if degenerate {
                // Plane through the apex, steeper than the generators: the
                // section is the apex alone.
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                return Ok(Certified::new(
                    AnalyticIntersection::TangentPoint(apex),
                    Certificate {
                        props,
                        method: Method::Exact,
                        budget_left: Budget::new(0, 0, 0),
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ));
            }
            let det = 4.0 * a * c - b * b;
            if det == 0.0 {
                return Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::RootNotIsolated,
                });
            }
            // Centre of the 2D conic: Cramer's rule on the gradient system
            // `2A u + B v + D = 0`, `B u + 2C v + E = 0`.
            let uc = (-2.0 * c * d + b * e2d) / det;
            let vc = (b * d - 2.0 * a * e2d) / det;
            let f_prime = a * uc * uc + b * uc * vc + c * vc * vc + d * uc + e2d * vc + f;
            // Eigen-frame of the quadratic form [[A, B/2], [B/2, C]].
            let half_trace = 0.5 * (a + c);
            let radius = 0.5 * ((a - c) * (a - c) + b * b).sqrt();
            let lambda1 = half_trace + radius;
            let lambda2 = half_trace - radius;
            let theta = 0.5 * b.atan2(a - c);
            let (e1x, e1y) = (theta.cos(), theta.sin());
            let (e2x, e2y) = (-theta.sin(), theta.cos());
            let sa2 = -f_prime / lambda1;
            let sb2 = -f_prime / lambda2;

            // Sanity rule: the squared semi-axes recomputed in intervals must
            // not be decisively negative — a negative `a²` would mean the
            // reduction is wrong, and that is a SPEC_GAP, not something to
            // patch silently.
            let half_trace_i = (ai + ci) * interval_at(0.5);
            let radius_i = ((ai - ci) * (ai - ci) + bi * bi).sqrt() * interval_at(0.5);
            let lambda1_i = half_trace_i + radius_i;
            let lambda2_i = half_trace_i - radius_i;
            let uc_i = interval_at(uc);
            let vc_i = interval_at(vc);
            let f_prime_i =
                ai * uc_i * uc_i + bi * uc_i * vc_i + ci * vc_i * vc_i + di * uc_i + ei * vc_i + fi;
            let sa2_i = -f_prime_i / lambda1_i;
            let sb2_i = -f_prime_i / lambda2_i;
            if sa2_i.sup() < 0.0 || sb2_i.sup() < 0.0 {
                return Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::RootNotIsolated,
                });
            }

            let center3 = o + uc * u_hat + vc * v_hat;
            let e1_3d = u_hat * e1x + v_hat * e1y;
            let e2_3d = u_hat * e2x + v_hat * e2y;
            let curve = if sa2 == sb2 {
                // Equal semi-axes within exact f64 equality of the squared
                // values: the section is a circle.
                let radius = sa2.sqrt();
                ExactCurve::Circle(Processor::with_transform(
                    TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
                    frame(e1_3d, e2_3d, n_hat, center3, radius, radius),
                ))
            } else {
                let (a_len, b_len) = (sa2.sqrt(), sb2.sqrt());
                ExactCurve::Ellipse(Processor::with_transform(
                    TrimmedCurve::new(UnitCircle::<Point3>::new(), (0.0, TAU)),
                    frame(e1_3d, e2_3d, n_hat, center3, a_len, b_len),
                ))
            };
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Curve(curve),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Ordering::Less => {
            // Hyperbola family.
            if degenerate {
                // The two generator lines through the apex lying in the plane.
                // A generator direction `(sin α·cos φ, sin α·sin φ, cos α)`
                // lies in the plane iff it is perpendicular to `n̂`, which is a
                // quadratic in the azimuth. Solved on the unit azimuth circle
                // `(c, s)`: `n̂x·c + n̂y·s = −n̂z/t` has the two roots
                // `(c, s) = δ·(c0, s0) ± w·(−s0, c0)` with
                // `(c0, s0) = (n̂x, n̂y)/r`, `δ = −n̂z/(t·r)`,
                // `w = √(1 − δ²)` — solve in f64, verify each root decisively
                // in inari against the raw normal, and emit both.
                let r = (n_hat.x * n_hat.x + n_hat.y * n_hat.y).sqrt();
                let (c0, s0) = (n_hat.x / r, n_hat.y / r);
                let delta = -n_hat.z / (t * r);
                let w = (1.0 - delta * delta).sqrt();
                let (c1, s1) = (delta * c0 - w * s0, delta * s0 + w * c0);
                let (c2, s2) = (delta * c0 + w * s0, delta * s0 - w * c0);
                if excludes_zero(&in_plane(c1, s1)) || excludes_zero(&in_plane(c2, s2)) {
                    return Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::RootNotIsolated,
                    });
                }
                let (sin_a, cos_a) = cone.half_angle().sin_cos();
                let dir1 = Vector3::new(sin_a * c1, sin_a * s1, cos_a);
                let dir2 = Vector3::new(sin_a * c2, sin_a * s2, cos_a);
                let line1 = Line(apex, apex + dir1);
                let line2 = Line(apex, apex + dir2);
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                return Ok(Certified::new(
                    AnalyticIntersection::TwoCurves([
                        ExactCurve::Line(line1),
                        ExactCurve::Line(line2),
                    ]),
                    Certificate {
                        props,
                        method: Method::Exact,
                        budget_left: Budget::new(0, 0, 0),
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ));
            }
            let det = 4.0 * a * c - b * b;
            if det == 0.0 {
                return Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::RootNotIsolated,
                });
            }
            let uc = (-2.0 * c * d + b * e2d) / det;
            let vc = (b * d - 2.0 * a * e2d) / det;
            let f_prime = a * uc * uc + b * uc * vc + c * vc * vc + d * uc + e2d * vc + f;
            let half_trace = 0.5 * (a + c);
            let radius = 0.5 * ((a - c) * (a - c) + b * b).sqrt();
            let lambda1 = half_trace + radius;
            let lambda2 = half_trace - radius;
            let theta = 0.5 * b.atan2(a - c);
            let (e1x, e1y) = (theta.cos(), theta.sin());
            let (e2x, e2y) = (-theta.sin(), theta.cos());
            // `λ1 x² + λ2 y² = −F'` with `λ1 > 0 > λ2`. The sign of `−F'`
            // decides which axis is the transverse one: the hyperbola opens
            // along the eigenvalue whose semi-axis `a² = −F'/λ` is positive
            // (both cases give `a² = −F'/λ_t > 0`, `b² = F'/λ_o > 0`).
            let (e_tx, e_ty, e_ox, e_oy, lambda_t, lambda_o) = if -f_prime > 0.0 {
                (e1x, e1y, e2x, e2y, lambda1, lambda2)
            } else {
                (e2x, e2y, e1x, e1y, lambda2, lambda1)
            };
            let a_len = (-f_prime / lambda_t).sqrt();
            let b_len = (f_prime / lambda_o).sqrt();
            let e_t3 = u_hat * e_tx + v_hat * e_ty;
            let e_o3 = u_hat * e_ox + v_hat * e_oy;
            let center3 = o + uc * u_hat + vc * v_hat;
            // Each branch is `(cosh t, sinh t, 0)` under the frame; the second
            // branch reflects the transverse axis (`ru = −a`).
            let branch_plus = Processor::with_transform(
                TrimmedCurve::new(UnitHyperbola::<Point3>::new(), HYPERBOLA_TRIM),
                frame(e_t3, e_o3, n_hat, center3, a_len, b_len),
            );
            let branch_minus = Processor::with_transform(
                TrimmedCurve::new(UnitHyperbola::<Point3>::new(), HYPERBOLA_TRIM),
                frame(e_t3, e_o3, n_hat, center3, -a_len, b_len),
            );
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::TwoCurves([
                    ExactCurve::Hyperbola(branch_plus),
                    ExactCurve::Hyperbola(branch_minus),
                ]),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
        Ordering::Equal => {
            // Parabola family.
            if degenerate {
                // Exactly one generator lies in the plane. The azimuth
                // quadratic has a double root here; its discriminant is a
                // rounding-away-from-zero `±ε` in f64, so the root is taken in
                // the stable vertex form — on the azimuth circle that is the
                // single point `(c, s) = δ·(c0, s0)` (`w = √(1 − δ²) = 0`),
                // computed without ever forming the discriminant.
                let r = (n_hat.x * n_hat.x + n_hat.y * n_hat.y).sqrt();
                let (c0, s0) = (n_hat.x / r, n_hat.y / r);
                let delta = -n_hat.z / (t * r);
                let (c, s) = (delta * c0, delta * s0);
                if excludes_zero(&in_plane(c, s)) {
                    return Err(Refusal::NumericallyUnresolved {
                        spent: Budget::new(0, 0, 0),
                        witness: UnresolvedWitness::RootNotIsolated,
                    });
                }
                let (sin_a, cos_a) = cone.half_angle().sin_cos();
                let dir = Vector3::new(sin_a * c, sin_a * s, cos_a);
                let line = Line(apex, apex + dir);
                let mut props = PropMap::new();
                props.set(Prop::AnalyticCarrier, Truth::True);
                return Ok(Certified::new(
                    AnalyticIntersection::TangentLine(line),
                    Certificate {
                        props,
                        method: Method::Exact,
                        budget_left: Budget::new(0, 0, 0),
                        margin: Margin::UNBOUNDED,
                        modulus: Modulus::Unbounded,
                    },
                ));
            }
            // The classic `(B u + 2C v)² = −4C F' (…)` reduction: multiplying
            // the conic by `4C` and using `B² = 4AC` turns the quadratic part
            // into the perfect square `(B u + 2C v)²`, whose gradient is the
            // transverse direction, leaving a term linear along the axis.
            // In the principal frame (`x` along `e1`, `y` along `e2`) the
            // quadratic form is `λ1 x² + D' x + E' y + F`; completing the
            // square in `x` and shifting to the vertex
            // (`X = x + D'/(2λ1)`, `Y = y + F''/E'`, `F'' = F − D'²/(4λ1)`)
            // gives `X² = c·Y` with `c = −E'/λ1`. The unit parabola `(t², 2t)`
            // placed through `frame` with `ru = 4/c` along the axis (`e2`) and
            // `rv = 1` along the transverse direction (`e1`) traces exactly
            // `X² = cY`: its axis coordinate is `ru·t²` and its transverse
            // coordinate `2t`, with `(2t)² = c·ru·t² ⟺ ru = 4/c`. Verified by
            // sampling in the tests.
            let half_trace = 0.5 * (a + c);
            let radius = 0.5 * ((a - c) * (a - c) + b * b).sqrt();
            let lambda1 = half_trace + radius;
            let theta = 0.5 * b.atan2(a - c);
            let (e1x, e1y) = (theta.cos(), theta.sin());
            let (e2x, e2y) = (-theta.sin(), theta.cos());
            let dp = d * e1x + e2d * e1y;
            let ep = d * e2x + e2d * e2y;
            if ep == 0.0 {
                // A non-degenerate parabola needs its linear term along the
                // axis; a vanishing `E'` means the reduction did not produce a
                // parabola (a pair of lines or an empty conic would be a
                // through-apex degeneracy, already excluded above).
                return Err(Refusal::NumericallyUnresolved {
                    spent: Budget::new(0, 0, 0),
                    witness: UnresolvedWitness::RootNotIsolated,
                });
            }
            let fpp = f - dp * dp / (4.0 * lambda1);
            let vertex_u = -dp / (2.0 * lambda1);
            let vertex_v = -fpp / ep;
            let vertex3 = o + vertex_u * u_hat + vertex_v * v_hat;
            let c_coef = -ep / lambda1;
            let ru = 4.0 / c_coef;
            let e1_3d = u_hat * e1x + v_hat * e1y;
            let e2_3d = u_hat * e2x + v_hat * e2y;
            let parabola = Processor::with_transform(
                TrimmedCurve::new(UnitParabola::<Point3>::new(), PARABOLA_TRIM),
                frame(e2_3d, e1_3d, n_hat, vertex3, ru, 1.0),
            );
            let mut props = PropMap::new();
            props.set(Prop::AnalyticCarrier, Truth::True);
            Ok(Certified::new(
                AnalyticIntersection::Curve(ExactCurve::Parabola(parabola)),
                Certificate {
                    props,
                    method: Method::Exact,
                    budget_left: Budget::new(0, 0, 0),
                    margin: Margin::UNBOUNDED,
                    modulus: Modulus::Unbounded,
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use truck_base::cgmath64::EuclideanSpace;
    use truck_geotrait::ParametricCurve;

    /// tan α = 3/4, dyadic: the family invariant becomes exact on integer raw
    /// normals (the alternative `sin α = 3/5` is not dyadic, which is exactly
    /// why it could not classify the parabola witness).
    const SLOPE: f64 = 3.0f64 / 4.0f64;
    /// Dimensionless slack for unit-scale residuals and direction vectors.
    const SLACK: f64 = 1.0e-12; // H-3: dimensionless slack on unit-scale residuals and direction vectors, not a length

    /// Builds the test cone with the dyadic slope and asserts the `tan ∘ atan`
    /// round trip holds on this host's libm — it fails loudly if a libm ever
    /// breaks it.
    fn test_cone(apex: Point3) -> Cone {
        let half_angle = (3.0f64 / 4.0f64).atan();
        let cone = match Cone::new(apex, half_angle) {
            Ok(certified) => certified.value,
            Err(refusal) => unreachable!(
                "Cone::new refuses an out-of-range half angle; (3/4).atan() lies in (0, PI/2): {refusal:?}"
            ),
        };
        assert!(
            cone.half_angle().tan() == SLOPE,
            "tan ∘ atan must round-trip exactly on this host: got {}",
            cone.half_angle().tan()
        );
        cone
    }

    /// The cone's own radial law `x² + y² = (z·tan α)²`, to machine precision.
    fn assert_on_cone(pt: Point3, cone: &Cone) {
        let r = pt - cone.apex();
        let radial2 = r.x * r.x + r.y * r.y;
        let slope = cone.half_angle().tan();
        let law2 = (slope * r.z) * (slope * r.z);
        assert!(
            (radial2 - law2).abs() < SLACK,
            "{pt:?} is off the cone (radial² = {radial2}, law² = {law2})"
        );
    }

    /// Incidence on the plane, to machine precision.
    fn assert_on_plane(pt: Point3, plane: &Plane) {
        let off = (pt - plane.origin()).dot(plane.normal()).abs();
        assert!(off < SLACK, "{pt:?} is off the plane (offset {off})");
    }

    fn unwrap_outcome(out: &AnalyticOutcome) -> &AnalyticIntersection {
        match out {
            Ok(certified) => &certified.value,
            Err(refusal) => unreachable!("plane_cone refused this witness: {refusal:?}"),
        }
    }

    fn cert_of(out: &AnalyticOutcome) -> &Certificate {
        match out {
            Ok(certified) => &certified.cert,
            Err(refusal) => unreachable!("plane_cone refused this witness: {refusal:?}"),
        }
    }

    #[test]
    fn pcone_horizontal_planes_cut_circles() {
        let cone = test_cone(Point3::origin());
        // Plane z = 2, normal +ẑ.
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        let out = plane_cone(&plane, &cone);
        let AnalyticIntersection::Curve(ExactCurve::Circle(circle)) = unwrap_outcome(&out) else {
            unreachable!("a horizontal plane cuts the double cone in a circle");
        };
        // Radius 2·(3/4) = 3/2, dyadic: the t = 0 point is computed exactly.
        assert_eq!(circle.subs(0.0), Point3::new(1.5, 0.0, 2.0));
        for i in 0..64 {
            let t = TAU * (i as f64) / 64.0;
            let pt = circle.subs(t);
            assert_eq!(pt.z, 2.0, "the circle lies in the plane z = 2");
            assert_on_cone(pt, &cone);
        }
        // Plane z = 0 through the apex → the section is the apex point alone.
        let through = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let out = plane_cone(&through, &cone);
        assert!(matches!(
            unwrap_outcome(&out),
            AnalyticIntersection::TangentPoint(pt) if *pt == Point3::origin()
        ));
    }

    #[test]
    fn pcone_vertical_plane_through_axis_two_lines() {
        let cone = test_cone(Point3::origin());
        // Plane y = 0 through the z axis, hence through the apex.
        let plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        let out = plane_cone(&plane, &cone);
        let AnalyticIntersection::TwoCurves([ExactCurve::Line(line0), ExactCurve::Line(line1)]) =
            unwrap_outcome(&out)
        else {
            unreachable!("a plane through the axis and the apex yields the two generators");
        };
        let (sin_a, cos_a) = cone.half_angle().sin_cos();
        let e0 = Vector3::new(sin_a, 0.0, cos_a);
        let e1 = Vector3::new(-sin_a, 0.0, cos_a);
        let d0 = line0.1 - line0.0;
        let d1 = line1.1 - line1.0;
        assert!(
            (d0 - e0).magnitude() < SLACK || (d0 - e1).magnitude() < SLACK, // H-3: angle slack between unit direction vectors, not a length
            "line direction {d0:?} is not one of the two generators"
        );
        assert!(
            (d1 - e0).magnitude() < SLACK || (d1 - e1).magnitude() < SLACK, // H-3: angle slack between unit direction vectors, not a length
            "line direction {d1:?} is not one of the two generators"
        );
        let d0_matches_e0 = (d0 - e0).magnitude() < SLACK; // H-3: angle slack between unit direction vectors, not a length
        let d1_matches_e0 = (d1 - e0).magnitude() < SLACK; // H-3: angle slack between unit direction vectors, not a length
        assert!(
            d0_matches_e0 ^ d1_matches_e0,
            "the two emitted generators must be the two distinct directions"
        );
        for line in [line0, line1] {
            for i in 0..32 {
                let s = -2.0 + 4.0 * (i as f64) / 31.0;
                let pt = line.0 + (line.1 - line.0) * s;
                assert_eq!(pt.y, 0.0, "sampled point off the plane y = 0");
                assert_on_cone(pt, &cone);
            }
        }
    }

    #[test]
    fn pcone_vertical_plane_two_hyperbola_branches() {
        let cone = test_cone(Point3::origin());
        // Plane x = 1, normal +x̂.
        let plane = Plane::new(
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
        );
        let out = plane_cone(&plane, &cone);
        let AnalyticIntersection::TwoCurves(
            [ExactCurve::Hyperbola(branch0), ExactCurve::Hyperbola(branch1)],
        ) = unwrap_outcome(&out)
        else {
            unreachable!("the plane x = 1 cuts the double cone in two hyperbola branches");
        };
        for branch in [branch0, branch1] {
            for i in 0..64 {
                let t = -3.0 + 6.0 * (i as f64) / 63.0;
                let pt = branch.subs(t);
                assert_eq!(pt.x, 1.0, "branch point off the plane x = 1");
                assert_on_cone(pt, &cone);
            }
        }
        // One branch sits on the z > 0 nappe, the other on z < 0.
        assert!(
            branch0.subs(0.0).z * branch1.subs(0.0).z < 0.0,
            "the two branches must sit on opposite nappes"
        );
    }

    #[test]
    fn pcone_tilted_ellipse() {
        let cone = test_cone(Point3::origin());
        // Plane through (0, 0, 4) with normal normalize((1, 0, 1)) — steeper
        // than the generators, off the apex.
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 4.0),
            Point3::new(1.0, 0.0, 3.0),
            Point3::new(0.0, 1.0, 4.0),
        );
        let out = plane_cone(&plane, &cone);
        let AnalyticIntersection::Curve(ExactCurve::Ellipse(ellipse)) = unwrap_outcome(&out) else {
            unreachable!("a plane steeper than the generators cuts an ellipse");
        };
        for i in 0..64 {
            let t = TAU * (i as f64) / 64.0;
            let pt = ellipse.subs(t);
            assert_on_plane(pt, &plane);
            assert_on_cone(pt, &cone);
        }
    }

    #[test]
    fn pcone_boundary_parabola() {
        let cone = test_cone(Point3::origin());
        // Raw normal (4, 0, 3) from integer differences: (p − o) × (q − o) =
        // (0,1,0) × (−3,0,4) = (4, 0, 3) exactly, so Nz² = t²(Nx²+Ny²) exactly
        // (9 = (9/16)·16) and the family invariant lands on the parabola
        // boundary. The plane clears the apex: h = (0,0,−5)·(0.8,0,0.6) = −3.
        let plane = Plane::new(
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.0, 1.0, 5.0),
            Point3::new(-3.0, 0.0, 9.0),
        );
        let out = plane_cone(&plane, &cone);
        let AnalyticIntersection::Curve(ExactCurve::Parabola(parabola)) = unwrap_outcome(&out)
        else {
            unreachable!("the boundary plane cuts a parabola");
        };
        for i in 0..64 {
            let t = -3.0 + 6.0 * (i as f64) / 63.0;
            let pt = parabola.subs(t);
            assert_on_plane(pt, &plane);
            assert_on_cone(pt, &cone);
        }
    }

    #[test]
    fn pcone_through_apex_degenerates() {
        let cone = test_cone(Point3::origin());
        // Ellipse family through the apex → the section is the apex point.
        let steep = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, -1.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let out = plane_cone(&steep, &cone);
        assert!(matches!(
            unwrap_outcome(&out),
            AnalyticIntersection::TangentPoint(pt) if *pt == Point3::origin()
        ));
        // Hyperbola family through the apex → two generators. The arm is
        // asserted here; the direction-level assertions live in
        // pcone_vertical_plane_through_axis_two_lines.
        let shallow = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        let out = plane_cone(&shallow, &cone);
        assert!(matches!(
            unwrap_outcome(&out),
            AnalyticIntersection::TwoCurves([ExactCurve::Line(_), ExactCurve::Line(_)])
        ));
        // Parabola family through the apex → the single tangent generator.
        // The plane carries the same raw normal (4, 0, 3) as the boundary
        // test; o at the apex gives the zero displacement, so h = [0, 0]
        // exactly and the parabola degenerates to one line.
        let boundary = Plane::new(
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(-3.0, 0.0, 4.0),
        );
        let out = plane_cone(&boundary, &cone);
        let AnalyticIntersection::TangentLine(line) = unwrap_outcome(&out) else {
            unreachable!("the boundary plane through the apex yields one tangent generator");
        };
        for i in 0..32 {
            let s = -2.0 + 4.0 * (i as f64) / 31.0;
            let pt = line.0 + (line.1 - line.0) * s;
            assert_on_cone(pt, &cone);
        }
    }

    #[test]
    fn pcone_certificate_is_exact() {
        let cone = test_cone(Point3::origin());
        // One outcome per arm shape: circle, two lines, hyperbola pair,
        // parabola.
        let circle_plane = Plane::new(
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(1.0, 0.0, 2.0),
            Point3::new(0.0, 1.0, 2.0),
        );
        let lines_plane = Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        );
        let hyperbola_plane = Plane::new(
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 0.0, 1.0),
        );
        let parabola_plane = Plane::new(
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.0, 1.0, 5.0),
            Point3::new(-3.0, 0.0, 9.0),
        );
        for out in [
            plane_cone(&circle_plane, &cone),
            plane_cone(&lines_plane, &cone),
            plane_cone(&hyperbola_plane, &cone),
            plane_cone(&parabola_plane, &cone),
        ] {
            let cert = cert_of(&out);
            assert_eq!(cert.method, Method::Exact);
            assert_eq!(cert.props.get(Prop::AnalyticCarrier), Truth::True);
        }
    }
}
