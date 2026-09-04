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

//! The CC seam stub types (CC-000-CONTRACT, spine seams S6/S9/S10/S11/S12).
//!
//! This module freezes the shared shapes the later CC packets type against:
//! the admissible v1 [`RadiusLaw`] (theory §5.3), the [`EventKind`] event
//! vocabulary (theory §5.2), the opaque [`WireComplex`], [`ShiftFunctional`],
//! [`BoundaryPlan`], and [`BranchSeed`] stub types, and the S11
//! [`TripleContactNode`] output record.
//!
//! **C7 stub posture.** The opaque seam types carry private fields and
//! refusing constructors that always return
//! `Err(ConstructRefusal::Unfrozen)`: their production belongs to the named
//! wave packet (CC-013 for [`WireComplex`]/[`ShiftFunctional`], CC-005 for
//! [`BoundaryPlan`], CC-030 for [`BranchSeed`], CC-020 for the triple-contact
//! solve that produces [`TripleContactNode`] values). No production logic
//! lives anywhere in this file.
//!
//! **H-1.** This module carries no `unwrap`, no `expect`, and no `panic!`, and
//! adds no module-level `allow`.

use crate::construct::refusal::ConstructRefusal;
use crate::construct::Interval;

/// An admissible v1 canal radius law (theory §5.3), consumed by the S10 canal
/// regularity seam and by the S11/S12 contact and trace machinery.
#[derive(Debug, Clone, PartialEq)]
pub enum RadiusLaw {
    /// A constant radius along the whole spine arc.
    Constant(f64),
    /// A linear radius interpolation between the arc-end radii.
    Linear {
        /// The radius at the arc start.
        r0: f64,
        /// The radius at the arc end.
        r1: f64,
    },
    /// A cubic Hermite radius profile over the arc.
    CubicHermite {
        /// The radius at the arc start.
        r0: f64,
        /// The radius at the arc end.
        r1: f64,
        /// The radius slope at the arc start.
        m0: f64,
        /// The radius slope at the arc end.
        m1: f64,
    },
    /// A monotone cubic radius law through explicit `(station, radius)` pairs.
    MonotoneCubic(Vec<(f64, f64)>),
    /// A vertex-driven radius law over the control-vertex stations.
    VertexControl(Vec<f64>),
}

/// The S12 blend-event vocabulary (theory §5.2): one tag per kind of event a
/// blend trace may record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A trim event: the blend chain clips against a trim loop.
    Trim,
    /// A third-face event: the chain meets a third support face.
    ThirdFace,
    /// A focal event on the chain.
    Focal,
    /// A rank event on the chain (rank-deficient step).
    Rank,
    /// A collision event with a non-support face.
    Collision,
    /// A plain trace continuation point.
    Trace,
}

/// The S9 abstract oriented cyclic wire complex (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-013 correspondence packet lands its production.
#[derive(Debug, Clone)]
pub struct WireComplex {
    /// Sealed. Production data is CC-013's design.
    _sealed: (),
}

impl WireComplex {
    /// The refusing stub constructor (C7): production belongs to CC-013.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S9 declared geometric shift functional (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-013 correspondence packet lands its production.
#[derive(Debug, Clone)]
pub struct ShiftFunctional {
    /// Sealed. Production data is CC-013's design.
    _sealed: (),
}

impl ShiftFunctional {
    /// The refusing stub constructor (C7): production belongs to CC-013.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S6 boundary-simplicity input plan (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-005 graph-disk packet lands its production from
/// the planar machinery.
#[derive(Debug, Clone)]
pub struct BoundaryPlan {
    /// Sealed. Production data is CC-005's design.
    _sealed: (),
}

impl BoundaryPlan {
    /// The refusing stub constructor (C7): production belongs to CC-005.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S12 blend branch seed (stub posture C7).
///
/// Opaque: private fields only, constructible exclusively through the refusing
/// constructor until the CC-030 blend-trace packet lands its production.
#[derive(Debug, Clone)]
pub struct BranchSeed {
    /// Sealed. Production data is CC-030's design.
    _sealed: (),
}

impl BranchSeed {
    /// The refusing stub constructor (C7): production belongs to CC-030.
    pub fn try_new() -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}

/// The S11 k=3 contact output record: one three-face contact node.
///
/// The field shape is frozen here (seam S11); values are produced by
/// `solve_triple_node` when CC-020 lands. The refusing constructor below is
/// the C7 stub posture until then.
#[derive(Debug, Clone, PartialEq)]
pub struct TripleContactNode {
    /// The certified centre enclosure of the contact node.
    pub centre: [Interval; 3],
    /// The certified radius enclosure of the contact node.
    pub radius: Interval,
    /// The certified per-contact parameter enclosures, one pair per support.
    pub contacts: [[Interval; 2]; 3],
}

impl TripleContactNode {
    /// The refusing stub constructor (C7): production belongs to CC-020.
    pub fn try_new(
        _centre: [Interval; 3],
        _radius: Interval,
        _contacts: [[Interval; 2]; 3],
    ) -> Result<Self, ConstructRefusal> {
        Err(ConstructRefusal::Unfrozen)
    }
}
