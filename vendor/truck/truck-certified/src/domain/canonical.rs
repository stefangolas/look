//! Gauge-Invariant Canonical Region Key & Atlas Presentation Labels.
//!
//! Derives canonical topological invariants (genus, Euler characteristic, winding,
//! generator side pairs, holes) independent of STEP seam position or edge segmentation.

/// Presentation label for canonical Atlas Cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AtlasCellId {
    /// Full-period cone apex disk (1 closed loop, 1 apex attachment).
    ApexDisk,
    /// Partial cone sector with apex attachment and 2 generator sides.
    ApexSector,
    /// Truncated cone sector without apex.
    TruncatedSector,
    /// Truncated cone annulus (2 essential loops).
    TruncatedAnnulus,
    /// Apex disk with interior holes.
    ApexDiskWithHoles,
    /// Regular planar disk with interior holes.
    RegularDiskWithHoles,
    /// Complex trimming requiring cover arrangement.
    ArrangementRequired,
    /// Contradictory boundary topology.
    InconsistentBoundary,
    /// Unresolved projection.
    UnresolvedProjection,
}

impl AtlasCellId {
    /// Returns the human-readable canonical cell name string.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ApexDisk => "C-APEX-DISK",
            Self::ApexSector => "C-APEX-SECTOR",
            Self::TruncatedSector => "C-TRUNC-SECTOR",
            Self::TruncatedAnnulus => "C-TRUNC-ANNULUS",
            Self::ApexDiskWithHoles => "C-APEX-DISK-H",
            Self::RegularDiskWithHoles => "C-REGULAR-DISK-H",
            Self::ArrangementRequired => "C-ARRANGEMENT-REQUIRED",
            Self::InconsistentBoundary => "C-INCONSISTENT",
            Self::UnresolvedProjection => "C-UNRESOLVED-PROJECTION",
        }
    }
}

/// Essential periodic boundary invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EssentialBoundaryKey {
    /// Essential winding count.
    pub winding: i64,
    /// Orientation flag.
    pub orientation: bool,
}

/// Canonical key for a single connected component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalComponentKey {
    /// Surface genus.
    pub genus: usize,
    /// Euler characteristic χ.
    pub euler_characteristic: i32,
    /// Essential periodic boundary components.
    pub essential_boundaries: Vec<EssentialBoundaryKey>,
    /// Physical generator side pairs.
    pub generator_side_pairs: usize,
    /// Circular arcs.
    pub circular_arcs: usize,
    /// Interior holes.
    pub holes: usize,
    /// Whether component contains a singular stratum.
    pub contains_singular_stratum: bool,
}

/// Overall canonical region key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRegionKey {
    /// Connected component keys.
    pub components: Vec<CanonicalComponentKey>,
    /// Total connected components count.
    pub total_connected_components: usize,
}

impl CanonicalRegionKey {
    /// Derives the canonical presentation atlas label.
    pub fn derive_atlas_label(&self) -> AtlasCellId {
        if self.total_connected_components == 0 {
            return AtlasCellId::UnresolvedProjection;
        }
        if self.total_connected_components > 1 {
            return AtlasCellId::ArrangementRequired;
        }

        let comp = &self.components[0];
        match (
            comp.contains_singular_stratum,
            comp.essential_boundaries.len(),
            comp.generator_side_pairs,
            comp.holes,
        ) {
            (true, 1, 0, 0) => AtlasCellId::ApexDisk,
            (true, 1, 0, h) if h > 0 => AtlasCellId::ApexDiskWithHoles,
            (false, 2, 0, 0) => AtlasCellId::TruncatedAnnulus,
            (true, 0, 1, 0) => AtlasCellId::ApexSector,
            (false, 0, 1, 0) => AtlasCellId::TruncatedSector,
            (false, 0, 0, h) if h > 0 => AtlasCellId::RegularDiskWithHoles,
            _ => AtlasCellId::ArrangementRequired,
        }
    }
}
