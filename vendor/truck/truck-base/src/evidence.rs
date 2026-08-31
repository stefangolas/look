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
    /// A forward error bound exceeded what the operation could certify
    /// (BG-EVD-004). Also raised when the split bound is requested for a chain
    /// that has not been shown subadditive.
    ForwardToleranceExceeded {
        /// The bound that was computed.
        bound: f64,
        /// The largest bound that would have been acceptable.
        allowed: f64,
    },
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
    /// A stratum pair whose contact reduction (FE, EE, general validated FF,
    /// singular event cells, or 2-D overlap) is not yet implemented in the
    /// Contact Layer (plan §4 Phase 3).
    ContactReductionDeferred,
    /// The envelope case for constructive-realization refusals (mapping A row 1).
    /// CG-007 adds it; every realization entry maps `ConstructError` onto
    /// `UnsupportedEnvelope(ConstructRefused)` and rides the details in
    /// `RealizationEvidence`.
    ConstructRefused,
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
    /// The fillet contact curve could not be located on the adjacent edge
    /// within budget (BG-S0-002).
    ContactCurveNotFound,
    /// A whole-span deviation bound (BG-CE-002) could not be certified within
    /// the subdivision budget: interval evaluation left at least one cell whose
    /// upper bound exceeds the tolerance and whose lower bound does not prove
    /// violation.
    DeviationUncertified,
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
    /// §1.1 invariant 1: coedge pairing — every non-degenerate edge has
    /// exactly 2 uses of opposite sense, a declared even number, or a
    /// declared 1 (BG-INV-101).
    CoedgePairing,
    /// §1.1 invariant 2: the vertex link is a single cycle (BG-INV-102).
    VertexLink,
    /// §1.1 invariant 3: the Euler–Poincaré relation holds (BG-INV-103).
    EulerPoincare,
    /// §1.1 invariant 4: same-parameter / same-range on every edge use
    /// (BG-INV-104).
    SameParameter,
    /// §1.1 invariant 5: domain–boundary correspondence (BG-INV-105).
    DomainBoundary,
    /// §1.1 invariant 6: representation in $\mathcal{G}$ within tau_rep
    /// (BG-INV-106).
    Representation,
    /// §1.1 invariant 7: tolerance monotonicity (BG-INV-107).
    ToleranceMonotonicity,
    /// §1.1 invariant 8: shell nesting is a forest (BG-INV-108).
    ShellNesting,
    /// §1.1 invariant 9: wedge non-degeneracy — dihedral bounded off 0 and
    /// 2π (BG-INV-109).
    WedgeNonDegeneracy,
    /// §12: a boundary fragment lies inside the other solid's closure
    /// (BG-SOL-RW3).
    FragmentInsideOther,
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

/// The shape of ω. Subadditivity is read off this and never declared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModulusShape {
    /// ω(ε) = k·ε.
    Lipschitz(f64),
    /// ω(ε) = k·ε^p. Tangency is p = 1/2.
    Holder {
        /// The Lipschitz-type constant `k`.
        k: f64,
        /// The Hölder exponent `p` (`1/2` at tangency, §9.2).
        exponent: f64,
    },
    /// ω(ε) = k·ε / (domain − ε): finite inside the domain, unbounded at its
    /// edge. This is what a near-degenerate cell publishes instead of
    /// `Unbounded` — an honest non-subadditive bound beats no bound at all.
    Pole {
        /// The leading constant `k`.
        k: f64,
    },
    /// No bound is published.
    Unbounded,
}

/// ω: modulus of continuity, valid on `[0, domain)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modulus {
    /// The shape, which decides subadditivity.
    pub shape: ModulusShape,
    /// ω is valid only on `[0, domain)`. `f64::INFINITY` for a global bound.
    pub domain: f64,
}

impl Modulus {
    /// Compatibility with the 38 `Modulus::Unbounded` call sites the r2 shape
    /// left behind; BG-EVD-r3b renames them and deletes this.
    #[allow(non_upper_case_globals)] // deliberate: it stands in for a variant path
    pub const Unbounded: Modulus = Modulus {
        shape: ModulusShape::Unbounded,
        domain: f64::INFINITY,
    };

    /// Whether ω is subadditive: ω(a+b) ≤ ω(a) + ω(b). Decided from the
    /// shape, never declared by a caller (BG-EVD-004). ω with ω(0) = 0 is
    /// subadditive when it is concave.
    pub fn is_subadditive(&self) -> bool {
        match self.shape {
            ModulusShape::Lipschitz(_) => true,
            ModulusShape::Holder { exponent, .. } => exponent <= 1.0,
            ModulusShape::Pole { .. } => false,
            ModulusShape::Unbounded => false,
        }
    }

    /// ω(ε). Returns `f64::INFINITY` outside `[0, domain)` and for `Unbounded`:
    /// "no bound available here" is a real answer, and a total function keeps
    /// this on the right side of H-1.
    pub fn eval(&self, eps: f64) -> f64 {
        if eps.is_nan() || eps < 0.0 || eps >= self.domain {
            return f64::INFINITY;
        }
        match self.shape {
            ModulusShape::Lipschitz(k) => k * eps,
            ModulusShape::Holder { k, exponent } => k * eps.powf(exponent),
            ModulusShape::Pole { k } => k * eps / (self.domain - eps),
            ModulusShape::Unbounded => f64::INFINITY,
        }
    }

    /// One step of the forward-error recurrence: ω(incoming) + tau. Always
    /// valid, subadditive or not.
    pub fn propagate(&self, incoming: f64, tau: f64) -> f64 {
        self.eval(incoming) + tau
    }

    /// Fold the recurrence over a chain of (modulus, tau) steps. Always valid.
    pub fn propagate_chain(steps: &[(Modulus, f64)]) -> f64 {
        steps
            .iter()
            .fold(0.0, |incoming, (m, tau)| m.propagate(incoming, *tau))
    }

    /// `self ∘ other` — `self` applied outside — the split-bound fast path.
    /// Refuses unless BOTH operands are subadditive (BG-EVD-004 M4): composing
    /// a non-subadditive operand would publish a bound that may under-report
    /// the forward error.
    pub fn compose(&self, other: &Self) -> Outcome<Modulus> {
        if !self.is_subadditive() || !other.is_subadditive() {
            return Err(Refusal::ForwardToleranceExceeded {
                bound: composed_constant(self, other),
                allowed: f64::INFINITY,
            });
        }
        // Subadditive operands are `Lipschitz` or `Holder { p <= 1 }` only; the
        // `Pole`/`Unbounded` arms below are unreachable and kept only to make
        // the match exhaustive.
        let shape = match (self.shape, other.shape) {
            (ModulusShape::Lipschitz(a), ModulusShape::Lipschitz(b)) => {
                ModulusShape::Lipschitz(a * b)
            }
            (ModulusShape::Lipschitz(a), ModulusShape::Holder { k, exponent }) => {
                ModulusShape::Holder { k: a * k, exponent }
            }
            (ModulusShape::Holder { k, exponent }, ModulusShape::Lipschitz(a)) => {
                ModulusShape::Holder {
                    k: k * a.powf(exponent),
                    exponent,
                }
            }
            (
                ModulusShape::Holder {
                    k: k1,
                    exponent: e1,
                },
                ModulusShape::Holder {
                    k: k2,
                    exponent: e2,
                },
            ) => ModulusShape::Holder {
                k: k1 * k2.powf(e1),
                exponent: e1 * e2,
            },
            (ModulusShape::Pole { .. }, _)
            | (_, ModulusShape::Pole { .. })
            | (ModulusShape::Unbounded, _)
            | (_, ModulusShape::Unbounded) => ModulusShape::Unbounded,
        };
        // A composite is valid only where both parts are.
        let composed = Modulus {
            shape,
            domain: self.domain.min(other.domain),
        };
        Ok(Certified::new(
            composed,
            Certificate {
                props: PropMap::new(),
                // The composition is float arithmetic on the two constants
                // (H-6): never `Exact`.
                method: Method::Float,
                budget_left: Budget::new(0, 0, 0),
                margin: Margin::UNBOUNDED,
                modulus: composed,
            },
        ))
    }

    /// The tightest subadditive modulus that dominates this one on its domain,
    /// so a caller holding a non-subadditive modulus can still reach the fast
    /// path by paying for a looser bound.
    pub fn concave_majorant(&self) -> Modulus {
        match self.shape {
            ModulusShape::Lipschitz(_) => *self,
            ModulusShape::Holder { exponent, .. } if exponent <= 1.0 => *self,
            ModulusShape::Holder { k, exponent } => {
                // Convex Hölder (p > 1): the Lipschitz chord over [0, d] —
                // through (0, 0) and (d, ω(d)) — has slope k·d^(p−1) and
                // dominates the convex ω there. An infinite domain admits no
                // finite chord, so no subadditive majorant is published.
                let d = self.domain;
                if d.is_finite() {
                    let slope = k * d.powf(exponent - 1.0);
                    Modulus {
                        shape: ModulusShape::Lipschitz(slope),
                        domain: d,
                    }
                } else {
                    Modulus::Unbounded
                }
            }
            ModulusShape::Pole { .. } => {
                // Convex, and unbounded at its domain edge: the chord through
                // (0, 0) and (d, ∞) has no finite slope, so no finite
                // Lipschitz line dominates a Pole over [0, d). Publish nothing
                // rather than a false bound.
                Modulus::Unbounded
            }
            ModulusShape::Unbounded => *self,
        }
    }
}

/// The characteristic constant of the would-be composite ω₂ ∘ ω₁, for the
/// `bound` of a `ForwardToleranceExceeded` refusal. `f64::INFINITY` where no
/// finite constant is defined (a `Pole` or `Unbounded` operand).
fn composed_constant(self_: &Modulus, other: &Modulus) -> f64 {
    match (self_.shape, other.shape) {
        (ModulusShape::Lipschitz(a), ModulusShape::Lipschitz(b)) => a * b,
        (ModulusShape::Lipschitz(a), ModulusShape::Holder { k, .. }) => a * k,
        (ModulusShape::Holder { k, exponent }, ModulusShape::Lipschitz(a)) => k * a.powf(exponent),
        (
            ModulusShape::Holder {
                k: k1,
                exponent: e1,
            },
            ModulusShape::Holder { k: k2, .. },
        ) => k1 * k2.powf(e1),
        (ModulusShape::Pole { .. }, _) | (_, ModulusShape::Pole { .. }) => f64::INFINITY,
        (ModulusShape::Unbounded, _) | (_, ModulusShape::Unbounded) => f64::INFINITY,
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
    /// - modulus: ω₂ ∘ ω₁ where both parts compose (both subadditive);
    ///   otherwise `Unbounded` — the honest conservative answer ("no bound
    ///   published") rather than a bound that might under-report.
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
        let modulus = match self.modulus.compose(&other.modulus) {
            Ok(composed) => composed.value,
            Err(_) => {
                // TODO(BG-EVD-r3b): thread propagate() through accumulation so a
                // non-subadditive chain still publishes its real bound instead of Unbounded.
                Modulus::Unbounded
            }
        };
        Ok(Certificate {
            props,
            method,
            budget_left,
            margin,
            modulus,
        })
    }
}

/// A Copy/Eq-safe projection of a constructive `ConstructError`. base cannot
/// name the error type (geometry depends on base, not vice versa), so the
/// error's identity rides as a tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstructErrorSummary {
    /// The error's identity tag: `"ZeroTangent"` | `"FrameSingular"`
    /// | `"SpineNotC1"` | `"ProfileCorrespondenceMismatch"`
    /// | `"ProfileCollapse"` | `"NonFinite"` | `"InvalidInput"`.
    pub kind: &'static str,
    /// The spine parameter where the error fired; `None` for structural
    /// refusals without a parameter.
    pub at: Option<f64>,
    /// Which frame law was singular; `FrameSingular` only.
    pub law: Option<&'static str>,
}

/// The three-valued realization verdict (mapping A row 4 / section B).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealizationVerdict {
    /// The mesh closed by construction and the audit found nothing.
    CertifiedWithinTolerance,
    /// The winding audit found violations — FAILED, never a warning.
    Failed,
    /// The audit could not decide; uncertainty is surfaced, never converted
    /// into success.
    Inconclusive,
}

/// Per-realization certificate (mapping A row 2). NOT a widening of
/// FaceValidityCertificate; the same separation doctrine as band_attempts vs
/// cone_band_attempts.
#[derive(Clone, Debug, PartialEq)]
pub struct RealizationCertificate {
    /// H-6: the facet path computes in floats.
    pub method: Method,
    /// Max bilinear-twist deviation over cells.
    pub max_cell_twist: f64,
    /// The audit's extent (the tolerance scale).
    pub extent: f64,
}

/// One shared-edge observation (mapping A row 3). Never a ProvenanceRecord
/// variant (that type is Copy + Eq; this payload carries f64s).
#[derive(Clone, Debug, PartialEq)]
pub struct SharedEdgePairEvidence {
    /// The measured position deviation of face A's sampled positions from the
    /// shared canonical sequence.
    pub error_a: f64,
    /// The measured position deviation of face B's sampled positions from the
    /// shared canonical sequence.
    pub error_b: f64,
}

/// The realization evidence record (mapping A row 1). Construct-stage
/// failures predate meshing and never enter MeshedShellOutcome; this is the
/// record that carries them, plus realization-stage facts.
#[derive(Clone, Debug, PartialEq)]
pub struct RealizationEvidence {
    /// The construct-stage error summary, when the realization refused at
    /// construct time.
    pub construct_error: Option<ConstructErrorSummary>,
    /// The per-realization certificate, when one was produced.
    pub certificate: Option<RealizationCertificate>,
    /// Shared-edge pair observations over sampled edges. Exactness is
    /// expressed by absence of rows.
    pub shared_edge_pairs: Vec<SharedEdgePairEvidence>,
    /// The three-valued realization verdict.
    pub verdict: RealizationVerdict,
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
            modulus: Modulus {
                shape: ModulusShape::Lipschitz(1.0),
                domain: f64::INFINITY,
            },
        };
        cert_a.props.set(Prop::AnalyticCarrier, Truth::True);
        let cert_b = Certificate {
            props: PropMap::new(),
            method: Method::Float,
            budget_left: Budget::new(5, 5, 5),
            margin: Margin::from_log2(1.0),
            modulus: Modulus {
                shape: ModulusShape::Lipschitz(2.0),
                domain: f64::INFINITY,
            },
        };
        let out = cert_a.accumulate(&cert_b).unwrap();
        // Exact ⊓ Float = Float (H-6).
        assert_eq!(out.method, Method::Float);
        // Margin: minimum.
        assert_eq!(out.margin.log2(), 1.0);
        // Modulus: Lipschitz(1)∘Lipschitz(2) = Lipschitz(2).
        assert_eq!(
            out.modulus,
            Modulus {
                shape: ModulusShape::Lipschitz(2.0),
                domain: f64::INFINITY,
            }
        );
        // Budget: sum of remainders.
        assert_eq!(out.budget_left.subdiv, 15);
    }

    #[test]
    fn modulus_composition_matches_numeric_evaluation() {
        let a = Modulus {
            shape: ModulusShape::Lipschitz(3.0),
            domain: f64::INFINITY,
        };
        let b = Modulus {
            shape: ModulusShape::Lipschitz(4.0),
            domain: f64::INFINITY,
        };
        let out = a.compose(&b).unwrap();
        assert_eq!(
            out.value,
            Modulus {
                shape: ModulusShape::Lipschitz(12.0),
                domain: f64::INFINITY,
            }
        );
        // The composite evaluates like the nested application, ω₂(ω₁(ε)).
        let eps = 0.5;
        assert!((out.value.eval(eps) - b.eval(a.eval(eps))).abs() < 1e-9); // H-3: float epsilon between two evaluations of one dimensionless modulus, not a length
                                                                           // The composition is float arithmetic, never stamped `Exact` (H-6).
        assert_eq!(out.cert.method, Method::Float);
        // `Unbounded` is not subadditive: the fast path refuses rather than
        // silently publishing the old "anything with Unbounded is Unbounded".
        assert!(matches!(
            a.compose(&Modulus::Unbounded),
            Err(Refusal::ForwardToleranceExceeded { .. })
        ));
        assert!(matches!(
            Modulus::Unbounded.compose(&a),
            Err(Refusal::ForwardToleranceExceeded { .. })
        ));
    }

    #[test]
    fn composition_matches_nested_application_on_every_arm() {
        // BG-EVD-004: on every arm the composed modulus evaluates like the
        // nested application with `self` outside: composed(ε) = outer(inner(ε)).
        // The inner constant is sampled both below and above 1 — the old
        // under-report only bites below 1, where the inner step contracts.
        let tol = 1e-9; // H-3: float epsilon between two evaluations of one dimensionless modulus, not a length
        let cases = [
            (lipschitz(2.0), lipschitz(0.01)),
            (lipschitz(2.0), lipschitz(4.0)),
            (lipschitz(2.0), holder(0.01, 0.5)),
            (lipschitz(2.0), holder(4.0, 0.5)),
            (holder(3.0, 0.5), lipschitz(0.01)),
            (holder(3.0, 0.5), lipschitz(4.0)),
            (holder(3.0, 0.5), holder(0.01, 0.5)),
            (holder(3.0, 0.5), holder(4.0, 0.5)),
        ];
        for (outer, inner) in cases {
            let composed = outer.compose(&inner).unwrap().value;
            for eps in [0.001, 0.01, 0.1, 0.5, 0.9] {
                let nested = outer.eval(inner.eval(eps));
                assert!(
                    (composed.eval(eps) - nested).abs() < tol,
                    "arm {outer:?} then {inner:?} disagrees with nested application at eps = {eps}"
                );
            }
        }
    }

    #[test]
    fn composition_constant_is_order_dependent() {
        // Holder{k, p} ∘ Lipschitz(a) has constant k·a^p and Lipschitz(a) ∘
        // Holder{k, p} has constant a·k; for a ≠ 1 these differ, so the
        // constants do not simply multiply (BG-EVD-004).
        let h_then_l = holder(3.0, 0.5).compose(&lipschitz(0.01)).unwrap().value;
        let l_then_h = lipschitz(0.01).compose(&holder(3.0, 0.5)).unwrap().value;
        assert_eq!(
            h_then_l,
            Modulus {
                shape: ModulusShape::Holder {
                    k: 3.0 * 0.01f64.powf(0.5),
                    exponent: 0.5,
                },
                domain: f64::INFINITY,
            }
        );
        assert_eq!(
            l_then_h,
            Modulus {
                shape: ModulusShape::Holder {
                    k: 0.01 * 3.0,
                    exponent: 0.5,
                },
                domain: f64::INFINITY,
            }
        );
        assert_ne!(h_then_l.shape, l_then_h.shape);
    }

    #[test]
    fn budget_exhaustion_is_typed() {
        let mut b = Budget::new(0, 0, 0);
        assert!(b.spend_subdiv(1).is_err());
        assert!(b.spend_newton(1).is_err());
        assert!(b.spend_depth().is_err());
    }

    #[test]
    fn modulus_shape_decides_subadditivity() {
        // The §2 table, every row: subadditivity is read off the shape.
        let lip = Modulus {
            shape: ModulusShape::Lipschitz(1.0),
            domain: f64::INFINITY,
        };
        let holder_concave = Modulus {
            shape: ModulusShape::Holder {
                k: 1.0,
                exponent: 0.5,
            },
            domain: f64::INFINITY,
        };
        let holder_linear = Modulus {
            shape: ModulusShape::Holder {
                k: 1.0,
                exponent: 1.0,
            },
            domain: f64::INFINITY,
        };
        let holder_convex = Modulus {
            shape: ModulusShape::Holder {
                k: 1.0,
                exponent: 1.5,
            },
            domain: 8.0,
        };
        let pole = Modulus {
            shape: ModulusShape::Pole { k: 1.0 },
            domain: 4.0,
        };
        let unbounded = Modulus::Unbounded;
        assert!(lip.is_subadditive());
        assert!(holder_concave.is_subadditive());
        assert!(holder_linear.is_subadditive());
        assert!(!holder_convex.is_subadditive());
        assert!(!pole.is_subadditive());
        assert!(!unbounded.is_subadditive());

        // §5: the concave majorant restores subadditivity from the shape, and
        // dominates the original at sampled points. The convex Hölder's
        // majorant is the Lipschitz chord over [0, d].
        let majorant = holder_convex.concave_majorant();
        assert!(majorant.is_subadditive());
        for i in 0..=63 {
            let eps = 8.0 * (i as f64) / 64.0;
            assert!(
                majorant.eval(eps) >= holder_convex.eval(eps) - 1e-9, // H-3: float slack on a dominance check, not a length
                "majorant under-reports at eps = {eps}"
            );
        }
        // A Pole is unbounded at its domain edge (ω(d) = ∞), so no finite
        // Lipschitz chord dominates it over [0, d); the majorant publishes
        // nothing.
        assert_eq!(pole.concave_majorant(), Modulus::Unbounded);
    }

    #[test]
    fn propagate_never_exceeds_split_bound_on_subadditive_chains() {
        // The recurrence is never looser than the split bound on subadditive
        // chains; that is why it is safe as the default path.
        let mut state: u64 = 0x5EED_2026_0816;
        for _ in 0..2000 {
            let len = 1 + lcg_next(&mut state) % 8;
            let mut steps = Vec::new();
            for _ in 0..len {
                steps.push((
                    random_subadditive_modulus(&mut state),
                    random_tau(&mut state),
                ));
            }
            let propagate = Modulus::propagate_chain(&steps);
            let split = split_bound(&steps);
            assert!(
                propagate <= split + 1e-9, // H-3: float slack between two error bounds, not a length
                "propagate {propagate} > split {split} on {steps:?}"
            );
        }
    }

    #[test]
    fn split_bound_under_reports_through_a_pole() {
        // The test this whole item exists for: with a `Pole` in the chain, the
        // split bound is STRICTLY smaller than the honest forward-error
        // recurrence — using it would under-report the error. The Pole is
        // convex, so the decoupling that subadditivity would justify runs the
        // wrong way: ω(a + b) > ω(a) + ω(b).
        let lip = Modulus {
            shape: ModulusShape::Lipschitz(1.0),
            domain: f64::INFINITY,
        };
        let pole = Modulus {
            shape: ModulusShape::Pole { k: 1.0 },
            domain: 4.0,
        };
        let steps = [(lip, 1.0), (lip, 1.0), (pole, 1.0)];
        let propagate = Modulus::propagate_chain(&steps);
        // The classic split bound, written out for this chain: each tolerance
        // is amplified by the moduli after it. The first two tolerances are
        // followed by the Pole (Lip(1) is the identity), the last by nothing.
        //  E₃ = pole(2) + 1 = 2.0, split = pole(1) + pole(1) + 1 = 5/3.
        let split = pole.eval(1.0) + pole.eval(1.0) + 1.0;
        assert!(
            split < propagate,
            "split {split} must be strictly smaller than propagate {propagate}"
        );
    }

    #[test]
    fn compose_refuses_a_non_subadditive_operand() {
        // Same chain as the under-report test: the fast path refuses rather
        // than returning the under-reporting bound.
        let lip = Modulus {
            shape: ModulusShape::Lipschitz(1.0),
            domain: f64::INFINITY,
        };
        let pole = Modulus {
            shape: ModulusShape::Pole { k: 1.0 },
            domain: 4.0,
        };
        // A `Pole` operand refuses in either position.
        assert!(matches!(
            lip.compose(&pole),
            Err(Refusal::ForwardToleranceExceeded {
                allowed,
                ..
            }) if allowed == f64::INFINITY
        ));
        assert!(matches!(
            pole.compose(&lip),
            Err(Refusal::ForwardToleranceExceeded { .. })
        ));
        // Folding the whole chain refuses at the step that meets the Pole.
        let out = lip.compose(&lip).unwrap().value.compose(&pole);
        assert!(matches!(out, Err(Refusal::ForwardToleranceExceeded { .. })));
    }

    #[test]
    fn pole_modulus_is_finite_inside_its_domain() {
        let pole = Modulus {
            shape: ModulusShape::Pole { k: 2.0 },
            domain: 4.0,
        };
        // Finite inside [0, domain).
        assert!(pole.eval(0.0).is_finite());
        assert!(pole.eval(1.0).is_finite());
        assert!(pole.eval(3.999).is_finite());
        assert!((pole.eval(1.0) - (2.0 * 1.0) / (4.0 - 1.0)).abs() < 1e-9); // H-3: float epsilon against a closed form, not a length
                                                                            // INFINITY at and beyond the domain edge.
        assert!(pole.eval(4.0).is_infinite());
        assert!(pole.eval(5.0).is_infinite());
        // INFINITY for negative and NaN input, without panicking.
        assert!(pole.eval(-0.5).is_infinite());
        assert!(pole.eval(f64::NAN).is_infinite());
    }

    /// Deterministic LCG so a failure is reproducible. Seeds the "random"
    /// subadditive chains and tolerances of the split-bound test.
    fn lcg_next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    /// A `Lipschitz` modulus on an infinite domain.
    fn lipschitz(a: f64) -> Modulus {
        Modulus {
            shape: ModulusShape::Lipschitz(a),
            domain: f64::INFINITY,
        }
    }

    /// A `Holder` modulus on an infinite domain.
    fn holder(k: f64, exponent: f64) -> Modulus {
        Modulus {
            shape: ModulusShape::Holder { k, exponent },
            domain: f64::INFINITY,
        }
    }

    /// A subadditive modulus: `Lipschitz` or `Holder` with `exponent <= 1.0`,
    /// published on an infinite (global) domain so every evaluation is finite.
    fn random_subadditive_modulus(state: &mut u64) -> Modulus {
        let kind = lcg_next(state) % 2;
        let k = 0.1 + ((lcg_next(state) % 10_000) as f64 / 10_000.0) * 2.9;
        let exponent = 0.1 + ((lcg_next(state) % 10_000) as f64 / 10_000.0) * 0.9;
        let shape = match kind {
            0 => ModulusShape::Lipschitz(k),
            _ => ModulusShape::Holder { k, exponent },
        };
        Modulus {
            shape,
            domain: f64::INFINITY,
        }
    }

    fn random_tau(state: &mut u64) -> f64 {
        ((lcg_next(state) % 100_000) as f64 / 100_000.0) * 0.5
    }

    /// The classic decoupled ("split") bound: each per-step tolerance is
    /// amplified by the moduli after it and the contributions are summed.
    /// Subadditivity of every modulus is exactly what makes this an upper bound
    /// on the recurrence (that is the theorem BG-EVD-004 leans on), and the
    /// `Pole` at the end of the under-report test breaks it in the unsafe
    /// direction.
    fn split_bound(steps: &[(Modulus, f64)]) -> f64 {
        let mut bound = 0.0;
        // `composite` is the composition of the moduli after the current step,
        // in chain order (ωₙ ∘ ... ∘ ωᵢ₊₁): later steps outermost. Built
        // backwards so `compose_math` (self ∘ other) keeps chain order.
        let mut composite: Option<Modulus> = None;
        for (m, tau) in steps.iter().rev() {
            bound += match composite {
                Some(c) => c.eval(*tau),
                None => *tau, // no moduli after this step: identity
            };
            composite = Some(match composite {
                Some(c) => compose_math(&c, m),
                None => *m,
            });
        }
        bound
    }

    /// True function composition, self ∘ other: x ↦ self(other(x)), as a
    /// `Modulus` shape. Production `compose` now uses the same arithmetic
    /// (a Hölder constant raised to the outer exponent: `k·a^p`, `k₁·k₂^p`),
    /// so this helper and the production fast path agree. It remains the
    /// reference implementation for the property tests. Test-only:
    /// `Pole`/`Unbounded` compositions are not single shapes and never arise
    /// in the subadditive chains exercised here.
    fn compose_math(self_: &Modulus, other: &Modulus) -> Modulus {
        let domain = self_.domain.min(other.domain);
        let shape = match (self_.shape, other.shape) {
            (ModulusShape::Lipschitz(a), ModulusShape::Lipschitz(b)) => {
                ModulusShape::Lipschitz(a * b)
            }
            (ModulusShape::Lipschitz(a), ModulusShape::Holder { k, exponent }) => {
                ModulusShape::Holder { k: a * k, exponent }
            }
            (ModulusShape::Holder { k, exponent }, ModulusShape::Lipschitz(a)) => {
                ModulusShape::Holder {
                    k: k * a.powf(exponent),
                    exponent,
                }
            }
            (
                ModulusShape::Holder {
                    k: k1,
                    exponent: e1,
                },
                ModulusShape::Holder {
                    k: k2,
                    exponent: e2,
                },
            ) => ModulusShape::Holder {
                k: k1 * k2.powf(e1),
                exponent: e1 * e2,
            },
            (ModulusShape::Pole { .. }, _) | (_, ModulusShape::Pole { .. }) => {
                ModulusShape::Unbounded
            }
            (ModulusShape::Unbounded, _) | (_, ModulusShape::Unbounded) => ModulusShape::Unbounded,
        };
        Modulus { shape, domain }
    }
}
