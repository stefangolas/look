//! BG-EVD-001 — the outcome/evidence algebra.
//!
//! §4 of the formal system. Every fallible kernel operation returns `Outcome<T>`.
//! The shape is `Result<Certified<T>, Refusal>` (spec P-2) so `?` works
//! natively; `Proven` vs `CertifiedEquivalent` is a field of `Certificate`
//! guarded by BG-EVD-002.
//!
//! The algebra lives here, in `truck-base`, because `truck-geotrait` is a leaf
//! that both geometry and modeling build on, and `IncludeCurve` needs
//! `Outcome` in its signature (BG-S0-001). A `truck-geotrait` → `truck-evidence`
//! dependency would be a cycle (evidence builds on geometry and geotrait), so
//! the algebra is a `truck-base` module and `truck-evidence` re-exports it.
//!
//! House rules H-1..H-7 (spec §0) apply. In particular, constructing a
//! `Certificate` is explicit field-by-field at every site: there is deliberately
//! **no** convenience constructor that stamps a method label onto an empty
//! certificate, so "exact" cannot be manufactured casually (BG-EVD-002).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::fmt::Debug;

/// §4 total and mutually exclusive outcome of a kernel operation.
pub type Outcome<T> = Result<Certified<T>, Refusal>;

/// A certified value: the value plus the evidence that produced it.
#[derive(Clone, Debug)]
pub struct Certified<T> {
    /// The computed value.
    pub value: T,
    /// The evidence certificate for `value`.
    pub cert: Certificate,
}

impl<T> Certified<T> {
    /// Wraps a value with a certificate.
    pub const fn new(value: T, cert: Certificate) -> Self {
        Self { value, cert }
    }
}

/// Every non-success terminal outcome of §4.
#[derive(Clone, Debug)]
pub enum Refusal {
    /// The operation's domain was empty; there is nothing to certify.
    Empty,
    /// The input lies outside the envelope the kernel currently supports.
    UnsupportedEnvelope(EnvelopeCase),
    /// The operation exhausted its budget without a certified answer.
    NumericallyUnresolved {
        /// What was spent before giving up.
        spent: Budget,
        /// What the witness was.
        witness: UnresolvedWitness,
    },
    /// Composition consumed the topological stability margin.
    CompositionMarginExhausted(MarginWitness),
    /// The input violates the backward (repair) budget.
    InputOutsideBackwardBudget(RepairWitness),
    /// The evidence contradicts itself; the result is not a realisation.
    Contradictory(ContradictionWitness),
    /// The exact object collapsed (§5) — certified, but not a realisation.
    Collapsed(Collapse, Certificate),
}

/// The envelope case that refused an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeCase {
    /// A chart degeneracy (§9.1): the local frame is singular.
    ChartDegenerate,
    /// The reach is too small to certify (BG-FID-005).
    ReachTooSmall,
    /// A carrier outside the canonical set $\mathcal{G}$.
    NonCanonicalCarrier,
    /// A NURBS weight was non-positive; the hull property fails (BG-ENC-003).
    NonPositiveNurbsWeight,
}

/// Why a numerically unresolved result could not be certified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedWitness {
    /// Containment of a point in a carrier could not be certified.
    UncertifiedContainment,
    /// A root could not be isolated (multiple / tangential roots, BG-NUM-002).
    RootNotIsolated,
    /// Krawczyk's operator proved neither existence nor absence (BG-NUM-003).
    KrawczykIndeterminate,
}

/// Where the composition margin ran out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarginWitness {
    /// The stage that exhausted the margin.
    pub stage: &'static str,
}

/// Why the backward (repair) budget was exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairWitness {
    /// The stage that gave up.
    pub stage: &'static str,
}

/// A contradiction between two evidence tuples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContradictionWitness {
    /// The property whose truth values conflicted.
    pub prop: Prop,
    /// The two conflicting truth values.
    pub left: Truth,
    /// The two conflicting truth values.
    pub right: Truth,
}

/// A §5 collapse of the exact object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Collapse {
    /// Why the object collapsed.
    pub reason: CollapseReason,
}

/// Why a collapse was certified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseReason {
    /// A knife edge (dihedral → 0) or crack (→ 2π) made lfs = 0 (BG-INV-109).
    KnifeEdge,
    /// §16.1 apex-vanishing of a cone.
    ApexVanishing,
}

/// The evidence tuple (π, μ, β, 𝔪, ω) of §4.
#[derive(Clone, Debug)]
pub struct Certificate {
    /// π: Prop -> Truth, the property map.
    pub props: PropMap,
    /// μ: Exact | Interval | Float | None — how the value was computed.
    pub method: Method,
    /// β: remaining budget.
    pub budget_left: Budget,
    /// 𝔪: topological stability margin (§18).
    pub margin: Margin,
    /// ω: modulus of continuity (§18).
    pub modulus: Modulus,
}

/// The method by which a value was computed (§4). A value computed in floats
/// may never be recorded as `Exact` (H-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// Exact — computed in exact/interval arithmetic, no float rounding.
    Exact,
    /// Interval — computed by outward-rounded interval arithmetic.
    Interval,
    /// Float — computed in plain f64.
    Float,
    /// None — the value is a structural/empty construction.
    None,
}

/// §4 knowledge order: ⊥ ≤k {T, F} ≤k ⊤.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Truth {
    /// Unknown (⊥).
    Unknown,
    /// Known true.
    True,
    /// Known false.
    False,
    /// Both true and false (⊤) — evidence is contradictory.
    Both,
}

impl Truth {
    /// Join in the knowledge order: `True ⊔ False = Both`.
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Truth::Unknown, x) | (x, Truth::Unknown) => x,
            (Truth::Both, _) | (_, Truth::Both) => Truth::Both,
            (Truth::True, Truth::True) => Truth::True,
            (Truth::False, Truth::False) => Truth::False,
            _ => Truth::Both,
        }
    }
}

/// A property named by a certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prop {
    /// The carrier is analytic (in $\mathcal{G}$).
    AnalyticCarrier,
    /// The value is a sound enclosure of the true image (BG-ENC-001).
    SoundEnclosure,
    /// The result is a certified equivalent, not a proof (BG-EVD-002).
    Provisional,
    /// The exact result is analytic and preserved as such (BG-CE-007).
    AnalyticPreserved,
}

/// π: the property map of a certificate.
#[derive(Clone, Debug, Default)]
pub struct PropMap {
    map: Vec<(Prop, Truth)>,
}

impl PropMap {
    /// An empty property map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a property's truth value.
    pub fn set(&mut self, prop: Prop, truth: Truth) {
        if let Some(slot) = self.map.iter_mut().find(|(p, _)| *p == prop) {
            slot.1 = slot.1.join(truth);
        } else {
            self.map.push((prop, truth));
        }
    }

    /// Reads a property's truth value; `Unknown` if unset.
    pub fn get(&self, prop: Prop) -> Truth {
        self.map
            .iter()
            .find(|(p, _)| *p == prop)
            .map_or(Truth::Unknown, |(_, t)| *t)
    }

    /// Joins two property maps; a `Both` anywhere is a contradiction.
    pub fn join(&self, other: &Self) -> Result<PropMap, ContradictionWitness> {
        let mut out = self.clone();
        for (prop, truth) in &other.map {
            let existing = out.get(*prop);
            let joined = existing.join(*truth);
            if joined == Truth::Both {
                return Err(ContradictionWitness {
                    prop: *prop,
                    left: existing,
                    right: *truth,
                });
            }
            out.set(*prop, joined);
        }
        Ok(out)
    }
}

/// β: the budget ledger of §7 (BG-NUM-001). A hard-coded loop bound is a defect
/// (H-5); every geometry-dependent iteration spends from here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Subdivisions remaining.
    pub subdiv: u32,
    /// Newton iterations remaining.
    pub newton: u32,
    /// Recursion depth remaining.
    pub depth: u32,
}

/// Exhaustion of a budget counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exhausted {
    /// Which counter was exhausted.
    pub counter: BudgetCounter,
}

/// Which budget counter was spent past zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetCounter {
    /// Subdivision counter.
    Subdiv,
    /// Newton iteration counter.
    Newton,
    /// Recursion depth counter.
    Depth,
}

impl Budget {
    /// A fresh budget with the §7 default counts.
    pub const fn new(subdiv: u32, newton: u32, depth: u32) -> Self {
        Self {
            subdiv,
            newton,
            depth,
        }
    }

    /// Spends `n` subdivisions; `Err` means the caller must return
    /// `NumericallyUnresolved`.
    pub fn spend_subdiv(&mut self, n: u32) -> Result<(), Exhausted> {
        if self.subdiv >= n {
            self.subdiv -= n;
            Ok(())
        } else {
            Err(Exhausted {
                counter: BudgetCounter::Subdiv,
            })
        }
    }

    /// Spends `n` Newton iterations.
    pub fn spend_newton(&mut self, n: u32) -> Result<(), Exhausted> {
        if self.newton >= n {
            self.newton -= n;
            Ok(())
        } else {
            Err(Exhausted {
                counter: BudgetCounter::Newton,
            })
        }
    }

    /// Spends one depth level.
    pub fn spend_depth(&mut self) -> Result<(), Exhausted> {
        if self.depth > 0 {
            self.depth -= 1;
            Ok(())
        } else {
            Err(Exhausted {
                counter: BudgetCounter::Depth,
            })
        }
    }
}

/// 𝔪: topological stability margin (§18). Stored as its base-2 logarithm so it
/// composes additively and monotone-min is `min`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Margin(f64);

impl Margin {
    /// A margin representing "infinite stability" (e.g. a plane).
    pub const UNBOUNDED: Self = Self(f64::INFINITY);

    /// Constructs a margin from a stability exponent.
    pub const fn from_log2(value: f64) -> Self {
        Self(value)
    }

    /// The stability exponent.
    pub fn log2(self) -> f64 {
        self.0
    }

    /// The weaker of two margins (minimum).
    pub fn min(self, other: Self) -> Self {
        Self(f64::min(self.0, other.0))
    }
}

impl std::ops::Add for Margin {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

/// ω: modulus of continuity (§18).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modulus {
    /// ω(ε) = k·ε.
    Lipschitz(f64),
    /// ω(ε) = k·ε^p — tangency is p = 1/2.
    Holder {
        /// The Lipschitz-type constant `k`.
        k: f64,
        /// The Hölder exponent `p` (`1/2` at tangency, §9.2).
        exponent: f64,
    },
    /// May not participate in composition (OB-6).
    Unbounded,
}

impl Modulus {
    /// Composition, ω₂ ∘ ω₁. `Lipschitz(a) ∘ Lipschitz(b) = Lipschitz(ab)`;
    /// anything composed with `Unbounded` is `Unbounded`.
    pub fn compose(&self, other: &Self) -> Self {
        match (self, other) {
            (Modulus::Unbounded, _) | (_, Modulus::Unbounded) => Modulus::Unbounded,
            (Modulus::Lipschitz(a), Modulus::Lipschitz(b)) => Modulus::Lipschitz(a * b),
            (
                Modulus::Holder {
                    k: k1,
                    exponent: e1,
                },
                Modulus::Holder {
                    k: k2,
                    exponent: e2,
                },
            ) => Modulus::Holder {
                k: k1 * k2,
                exponent: e1 * e2,
            },
            (Modulus::Lipschitz(a), Modulus::Holder { k, exponent }) => Modulus::Holder {
                k: a * k,
                exponent: *exponent,
            },
            (Modulus::Holder { k, exponent }, Modulus::Lipschitz(a)) => Modulus::Holder {
                k: a * k,
                exponent: *exponent,
            },
        }
    }
}

impl Certificate {
    /// Accumulates two certificates into one (§4).
    ///
    /// - props: join in the knowledge order; any `Both` ⇒ `Contradictory`.
    /// - method: the weakest of the two (H-6) — weakest in the sense of least
    ///   certainty, so `Exact ⊓ Float = Float` and `None` dominates.
    /// - budget_left: the sum of the remainders.
    /// - margin: the minimum.
    /// - modulus: ω₂ ∘ ω₁.
    pub fn accumulate(&self, other: &Self) -> Result<Certificate, ContradictionWitness> {
        let props = self.props.join(&other.props)?;
        // Method is ordered weakest → strongest in the enum declaration; the
        // weakest of the two is the `max` (None dominates, then Float, ...).
        let method = self.method.max(other.method);
        let budget_left = Budget {
            subdiv: self.budget_left.subdiv + other.budget_left.subdiv,
            newton: self.budget_left.newton + other.budget_left.newton,
            depth: self.budget_left.depth + other.budget_left.depth,
        };
        let margin = self.margin.min(other.margin);
        let modulus = self.modulus.compose(&other.modulus);
        Ok(Certificate {
            props,
            method,
            budget_left,
            margin,
            modulus,
        })
    }
}

#[cfg(test)]
// Test-only allow: H-1 bans unwrap/expect on paths reachable from untrusted
// geometry. Unit-test assertions on hand-built witnesses are not such a path;
// these unwraps cannot fire for the values constructed below.
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn truth_join_true_false_is_both() {
        assert_eq!(Truth::True.join(Truth::False), Truth::Both);
        assert_eq!(Truth::Unknown.join(Truth::True), Truth::True);
        assert_eq!(Truth::True.join(Truth::Unknown), Truth::True);
    }

    #[test]
    fn propmap_contradiction_propagates() {
        let mut a = PropMap::new();
        a.set(Prop::AnalyticCarrier, Truth::True);
        let mut b = PropMap::new();
        b.set(Prop::AnalyticCarrier, Truth::False);
        let err = a.join(&b).unwrap_err();
        assert_eq!(err.prop, Prop::AnalyticCarrier);
        assert_eq!(err.left, Truth::True);
        assert_eq!(err.right, Truth::False);
    }

    #[test]
    fn accumulation_is_weakest_method() {
        let mut cert_a = Certificate {
            props: PropMap::new(),
            method: Method::Exact,
            budget_left: Budget::new(10, 10, 10),
            margin: Margin::UNBOUNDED,
            modulus: Modulus::Lipschitz(1.0),
        };
        cert_a.props.set(Prop::AnalyticCarrier, Truth::True);
        let cert_b = Certificate {
            props: PropMap::new(),
            method: Method::Float,
            budget_left: Budget::new(5, 5, 5),
            margin: Margin::from_log2(1.0),
            modulus: Modulus::Lipschitz(2.0),
        };
        let out = cert_a.accumulate(&cert_b).unwrap();
        // Exact ⊓ Float = Float (H-6).
        assert_eq!(out.method, Method::Float);
        // Margin: minimum.
        assert_eq!(out.margin.log2(), 1.0);
        // Modulus: Lipschitz(1)∘Lipschitz(2) = Lipschitz(2).
        assert_eq!(out.modulus, Modulus::Lipschitz(2.0));
        // Budget: sum of remainders.
        assert_eq!(out.budget_left.subdiv, 15);
    }

    #[test]
    fn modulus_composition_matches_numeric_evaluation() {
        let a = Modulus::Lipschitz(3.0);
        let b = Modulus::Lipschitz(4.0);
        assert_eq!(a.compose(&b), Modulus::Lipschitz(12.0));
        assert_eq!(a.compose(&Modulus::Unbounded), Modulus::Unbounded);
        assert_eq!(Modulus::Unbounded.compose(&a), Modulus::Unbounded);
    }

    #[test]
    fn budget_exhaustion_is_typed() {
        let mut b = Budget::new(0, 0, 0);
        assert!(b.spend_subdiv(1).is_err());
        assert!(b.spend_newton(1).is_err());
        assert!(b.spend_depth().is_err());
    }
}
