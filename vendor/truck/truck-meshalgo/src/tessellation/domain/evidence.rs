//! Preservation of Source STEP Representation & Provenance.
//!
//! Retains raw STEP boundary and entity evidence without contaminating geometry code.

/// Unique entity identifier from STEP file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceEntityId(pub usize);

/// Vertex evidence from STEP entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceVertexEvidence {
    /// STEP entity ID.
    pub entity_id: SourceEntityId,
}

/// Oriented edge use evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeUseEvidence {
    /// STEP edge curve entity ID.
    pub edge_id: SourceEntityId,
    /// Edge loop orientation flag.
    pub orientation: bool,
    /// Edge curve same sense flag.
    pub same_sense: bool,
}

/// Boundary evidence (EdgeLoop or VertexLoop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundSourceEvidence {
    /// Standard edge loop.
    EdgeLoop {
        /// Bound entity ID.
        bound_id: SourceEntityId,
        /// Loop orientation.
        orientation: bool,
        /// Edges in loop.
        edges: Vec<EdgeUseEvidence>,
    },
    /// Collapsed point vertex loop.
    VertexLoop {
        /// Bound entity ID.
        bound_id: SourceEntityId,
        /// Loop orientation.
        orientation: bool,
        /// Collapsed vertex evidence.
        vertex: SourceVertexEvidence,
    },
}

/// Source face topological evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceSourceEvidence {
    /// Source face entity ID.
    pub face_id: SourceEntityId,
    /// Underlying surface entity ID.
    pub surface_id: SourceEntityId,
    /// Surface same sense flag.
    pub face_same_sense: bool,
    /// Boundaries of the face.
    pub bounds: Vec<BoundSourceEvidence>,
}
