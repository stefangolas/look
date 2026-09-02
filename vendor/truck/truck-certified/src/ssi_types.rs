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

//! The SSI wave shim, part 1: the shared shapes the Phase-2 wave packets
//! exchange (BG-CK-P2-CONTRACT).
//!
//! This is a freeze in the P0-FREEZE pattern: only the shared shapes
//! (`SquareSystem3`, `KrawczykCertificate3`, `TraceStep`/`TraceOutcome`/
//! `TraceRefusal`) land here, with refusing constructors and verbatim
//! accessors. The four implementation packets (SYSTEM / KRAWCZYK3 / TRACE /
//! RESIDUAL) build against this module; nothing here evaluates, solves,
//! isolates, or certifies numerically. The mathematics is frozen in
//! `docs/CERTIFIED_PHASE2_BOOKING.md` and the frozen F3 contract
//! (`src/contract.rs`): this packet adds no decisions and invents no evidence
//! kinds.
//!
//! **D-shim.** Types and refusing constructors only. Any method that would
//! evaluate, solve, isolate, or certify NUMERICALLY refuses
//! (`InvalidInput`-shaped or a named case from the existing vocabularies). The
//! module doc says verbatim: "This module freezes shapes; BG-CK-P2-SYSTEM /
//! KRAWCZYK3 / TRACE implement against it and never restate it."
//!
//! **D-reuse.** `contract.rs`'s frozen F3 types are the vocabulary:
//! [`ContinuationCoordinate`], [`CoordinateSwitch`], [`SquareSystemInput`],
//! [`Refusal::ConditioningBelowThreshold`]. `formal/span.rs`'s [`BranchGerm`],
//! `formal/contact.rs`'s [`BranchIncidence`]. The shim wraps/aliases; it never
//! duplicates a landed type under a new name. Refusals wrap the landed named
//! causes verbatim — no new top-level evidence kinds (mapping section C).
//!
//! **D-homogeneous.** [`SquareSystem3`] carries the cross-multiplied
//! homogeneous system `F_k = W2*P1_k − W1*P2_k` (k ∈ x,y,z) as tensor-Bernstein
//! coefficient grids over `(u,v) x (s,t)` — the KRAWCZYK3 packet's `K(X)`
//! contract operates on this. The weight certificates `W1, W2 > 0` are INPUTS
//! (carried as the patches' own landed certificates), never re-derived here.

use crate::contract::{ContinuationCoordinate, Refusal};
use crate::formal::contact::{BranchIncidence, GenericUnresolved};
use crate::formal::span::BranchGerm;
use crate::hull::HullRefusal;

/// The stored square-system representation (SYSTEM's output contract).
///
/// `F_k(u,v,s,t) = W2*P1_k − W1*P2_k` as tensor-Bernstein grids over the
/// product chart `(u,v) x (s,t)` (D-homogeneous); the 3x4 Jacobian is DERIVED
/// by consumers via the landed hull kernels, not stored. Constructed only
/// through [`SquareSystem3::new`], which refuses ragged/empty grids, non-finite
/// coefficients, and degree-0 inputs.
///
/// # Grid layout
///
/// A grid is a flat rectangular coefficient table. `degrees = (m1, n1, m2, n2)`
/// is the bidegree of patch 1 in `(u, v)` and of patch 2 in `(s, t)`. The grid
/// of one component has `(m1+1)*(n1+1)` rows and `(m2+1)*(n2+1)` columns; the
/// coefficient of `B^m1_a(u) B^n1_b(v) B^m2_i(s) B^n2_j(t)` sits at
/// `row = a*(n1+1) + b`, `column = i*(n2+1) + j`. `domain_maps` carries the two
/// chart rectangles as
/// `(u0,u1,v0,v1,s0,s1,t0,t1)` (the affine map from each patch's unit-square
/// Bernstein coordinates onto the trace-chart rectangle).
///
/// This is a certificate *carrier*, not an interval algebra: it performs no
/// arithmetic (D-shim). Consumers read the grids through the accessors and own
/// every hull/interval evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct SquareSystem3 {
    /// One tensor-Bernstein coefficient grid per component `k` (x, y, z).
    grids: [Vec<Vec<f64>>; 3],
    /// `(m1, n1, m2, n2)` — bidegree of patch 1 in `(u,v)` and patch 2 in
    /// `(s,t)`. Every axis degree is at least 1 by construction.
    degrees: (usize, usize, usize, usize),
    /// `(u0,u1,v0,v1,s0,s1,t0,t1)` — the two chart rectangles.
    domain_maps: (f64, f64, f64, f64, f64, f64, f64, f64),
}

impl SquareSystem3 {
    /// Construct a square system from three preformed grids plus the degree and
    /// chart metadata.
    ///
    /// Refuses (each as [`Refusal::InvalidInput`], a construction outside a
    /// frozen rule):
    /// - a degree-0 input (any of `m1, n1, m2, n2` zero);
    /// - an empty, ragged, or degree-mismatched grid (any grid whose row or
    ///   column count does not equal the shape the degrees demand);
    /// - any non-finite coefficient;
    /// - a non-finite, misordered, or degenerate (zero-width) chart interval in
    ///   `domain_maps`.
    pub fn new(
        grids: [Vec<Vec<f64>>; 3],
        degrees: (usize, usize, usize, usize),
        domain_maps: (f64, f64, f64, f64, f64, f64, f64, f64),
    ) -> Result<Self, Refusal> {
        let (m1, n1, m2, n2) = degrees;
        if m1 == 0 || n1 == 0 || m2 == 0 || n2 == 0 {
            return Err(Refusal::InvalidInput);
        }
        let rows = (m1 + 1) * (n1 + 1);
        let cols = (m2 + 1) * (n2 + 1);
        for grid in &grids {
            if grid.len() != rows {
                return Err(Refusal::InvalidInput);
            }
            for row in grid {
                if row.len() != cols || !row.iter().all(|c| c.is_finite()) {
                    return Err(Refusal::InvalidInput);
                }
            }
        }
        let [u0, u1, v0, v1, s0, s1, t0, t1] = [
            domain_maps.0,
            domain_maps.1,
            domain_maps.2,
            domain_maps.3,
            domain_maps.4,
            domain_maps.5,
            domain_maps.6,
            domain_maps.7,
        ];
        for (lo, hi) in [(u0, u1), (v0, v1), (s0, s1), (t0, t1)] {
            if !lo.is_finite() || !hi.is_finite() || lo >= hi {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self {
            grids,
            degrees,
            domain_maps,
        })
    }

    /// The three stored component grids, in `(x, y, z)` order.
    pub fn grids(&self) -> &[Vec<Vec<f64>>; 3] {
        &self.grids
    }

    /// `(m1, n1, m2, n2)` — the stored degrees, verbatim.
    pub fn degrees(&self) -> (usize, usize, usize, usize) {
        self.degrees
    }

    /// `(u0,u1,v0,v1,s0,s1,t0,t1)` — the stored chart rectangles, verbatim.
    pub fn domain_maps(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
        self.domain_maps
    }
}

/// The Krawczyk unique-root certificate (KRAWCZYK3's output contract).
///
/// Constructed ONLY from a strict inclusion: [`KrawczykCertificate3::new`]
/// refuses a non-strict or boundary inclusion (K(X) must be component-wise
/// strictly inside X) — the frozen emission rule made typecheckable. Carries
/// the box `X`, the K(X) enclosure, and the determinant enclosure (0 excluded).
///
/// The determinant enclosure's excluding zero is part of construction, not a
/// later check: a determinant enclosure containing zero is a box the operator
/// may not certify, and the constructor refuses it. This is a carrier only
/// (D-shim); the enclosure values arrive from the consumers' certified
/// interval work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrawczykCertificate3 {
    /// The box `X`: three axis intervals.
    box_x: [(f64, f64); 3],
    /// The K(X) enclosure, component-wise strictly inside `box_x`.
    k_x: [(f64, f64); 3],
    /// The determinant enclosure over `box_x`, with 0 strictly excluded.
    det: (f64, f64),
}

impl KrawczykCertificate3 {
    /// Build the certificate from a strict inclusion and an orientation
    /// enclosure.
    ///
    /// Refuses (each as [`Refusal::InvalidInput`]):
    /// - any non-finite or misordered (`lo > hi`) box or K(X) interval;
    /// - a non-strict or boundary inclusion — K(X) is not component-wise
    ///   STRICTLY inside X on every axis;
    /// - a non-finite, misordered determinant interval, or one containing zero
    ///   (inclusive of a `0` endpoint) — the orientation precondition is part
    ///   of construction.
    pub fn new(
        box_x: [(f64, f64); 3],
        k_x: [(f64, f64); 3],
        det: (f64, f64),
    ) -> Result<Self, Refusal> {
        for (box_axis, k_axis) in box_x.iter().zip(k_x.iter()) {
            let (b_lo, b_hi) = *box_axis;
            let (k_lo, k_hi) = *k_axis;
            let interval_ok = |(lo, hi): (f64, f64)| lo.is_finite() && hi.is_finite() && lo <= hi;
            if !interval_ok(*box_axis) || !interval_ok(*k_axis) {
                return Err(Refusal::InvalidInput);
            }
            // Strict inclusion: K(X) strictly inside X on this axis. A
            // boundary-touching or reversed enclosure refuses.
            let strictly_inside = b_lo < k_lo && k_hi < b_hi;
            if !strictly_inside {
                return Err(Refusal::InvalidInput);
            }
        }
        let (d_lo, d_hi) = det;
        if !d_lo.is_finite() || !d_hi.is_finite() || d_lo > d_hi {
            return Err(Refusal::InvalidInput);
        }
        // 0 excluded STRICTLY: an endpoint of 0 is still a determinant
        // enclosure that contains zero.
        if d_lo <= 0.0 && 0.0 <= d_hi {
            return Err(Refusal::InvalidInput);
        }
        Ok(Self { box_x, k_x, det })
    }

    /// The box `X`: three axis intervals, verbatim.
    pub fn box_x(&self) -> [(f64, f64); 3] {
        self.box_x
    }

    /// The K(X) enclosure, verbatim.
    pub fn k_x(&self) -> [(f64, f64); 3] {
        self.k_x
    }

    /// The determinant enclosure (0 excluded), verbatim.
    pub fn det(&self) -> (f64, f64) {
        self.det
    }
}

/// One traced branch box (TRACE's per-step output): the parameter box in the
/// 4D chart, the germ class, the branch incidence record, and the
/// continuation-coordinate certificate the frozen F3 rule carries at the box.
///
/// `box` holds the four axis intervals of the trace box in chart order
/// `(u, v, s, t)` — patch 1's two axes then patch 2's two axes. The germ and
/// the continuation coordinate are carried as the landed [`BranchGerm`] and
/// [`ContinuationCoordinate`] values; the incidence is the landed
/// [`BranchIncidence`] record (D-reuse). Constructed through
/// [`TraceStep::new`], which refuses a non-finite or misordered box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceStep {
    /// The trace box in the 4D chart, as `(u,v,s,t)` axis intervals.
    chart_box: [(f64, f64); 4],
    /// The germ class of the branch at this box.
    germ: BranchGerm,
    /// The branch incidence record.
    incidence: BranchIncidence,
    /// The certified continuation coordinate for this box.
    coordinate: ContinuationCoordinate,
}

impl TraceStep {
    /// Build one trace step from the landed types plus the box.
    ///
    /// Refuses ([`Refusal::InvalidInput`]) a box containing a non-finite or
    /// misordered (`lo > hi`) axis interval. The germ, incidence and
    /// coordinate are already-typed landed values and are carried verbatim.
    pub fn new(
        chart_box: [(f64, f64); 4],
        germ: BranchGerm,
        incidence: BranchIncidence,
        coordinate: ContinuationCoordinate,
    ) -> Result<Self, Refusal> {
        for (lo, hi) in chart_box {
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                return Err(Refusal::InvalidInput);
            }
        }
        Ok(Self {
            chart_box,
            germ,
            incidence,
            coordinate,
        })
    }

    /// The trace box in the 4D chart, as `(u,v,s,t)` axis intervals, verbatim.
    pub fn chart_box(&self) -> [(f64, f64); 4] {
        self.chart_box
    }

    /// The germ class carried at this box.
    pub fn germ(&self) -> BranchGerm {
        self.germ
    }

    /// The branch incidence record.
    pub fn incidence(&self) -> BranchIncidence {
        self.incidence
    }

    /// The certified continuation coordinate for this box.
    pub fn coordinate(&self) -> ContinuationCoordinate {
        self.coordinate
    }
}

/// The outcome of tracing one branch from one seed.
///
/// Shape mirrors the landed pair-contact results ([`crate::formal::contact::PairContactResult`]):
/// named cases, no catch-all. The refusal vocabulary wraps the landed named
/// causes (D-reuse) — no new top-level evidence kinds (mapping section C).
#[derive(Debug, Clone, PartialEq)]
pub enum TraceOutcome {
    /// The branch closed on itself (identity recurrence) — the loop's first
    /// box id equals the closing box id.
    ClosedLoop {
        /// The steps traced around the closed loop.
        steps: Vec<TraceStep>,
    },
    /// The branch terminated at a certified boundary/refusal-free end.
    Terminated {
        /// The steps traced to the end.
        steps: Vec<TraceStep>,
    },
    /// A certified turning-point switch occurred mid-branch.
    Switched {
        /// The steps traced up to and including the switch box.
        steps: Vec<TraceStep>,
        /// The switch event, carrying BOTH required certificates (F3).
        switch: crate::contract::CoordinateSwitch,
    },
    /// A named refusal case.
    Refused(TraceRefusal),
}

/// The trace refusal vocabulary: aliases/wraps of LANDED named cases.
///
/// No new top-level evidence kinds (mapping section C): each variant carries a
/// landed cause type verbatim. A variant exists per refusal family — the
/// conditioning refusal (F3, `ConditioningBelowThreshold`-shaped), the hull
/// enclosure failures, and the generic-unresolved causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceRefusal {
    /// An F3 conditioning refusal: a box where the frozen coordinate-selection
    /// rule could not certify any coordinate away-from-zero. Wraps the landed
    /// [`Refusal`] verbatim; the trace-relevant value is
    /// [`Refusal::ConditioningBelowThreshold`].
    Conditioning(Refusal),
    /// A certified enclosure could not be produced by the hull layer. Wraps
    /// [`HullRefusal`] verbatim (the `EnclosureUnavailable` / `DomainNotCompact`
    /// named cases).
    Hull(HullRefusal),
    /// The branch could not be certified under the declared numerical policy.
    /// Wraps [`GenericUnresolved`] verbatim (the landed named causes).
    Unresolved(GenericUnresolved),
}

/// One trace refusal's stable diagnostic tag.
impl TraceRefusal {
    /// A short stable tag, for diagnostics.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Conditioning(Refusal::ConditioningBelowThreshold) => "trace_refused_conditioning",
            Self::Conditioning(Refusal::InvalidInput) => "trace_refused_invalid_input",
            Self::Conditioning(Refusal::Unfrozen) => "trace_refused_unfrozen",
            Self::Hull(HullRefusal::EnclosureUnavailable) => {
                "trace_refused_hull_enclosure_unavailable"
            }
            Self::Hull(HullRefusal::DomainNotCompact) => "trace_refused_hull_domain_not_compact",
            Self::Unresolved(cause) => cause.tag(),
        }
    }
}
