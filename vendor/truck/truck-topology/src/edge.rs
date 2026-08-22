use crate::{errors::Error, *};
use thiserror::Error;

impl<P, C> Edge<P, C> {
    /// Generates the edge from `front` to `back`.  
    /// # Failures
    /// If `front == back`, then returns `Error::SameVertex`.
    /// ```
    /// use truck_topology::*;
    /// use truck_topology::errors::Error;
    /// let v = Vertex::news(&[(), ()]);
    /// assert!(Edge::try_new(&v[0], &v[1], ()).is_ok());
    /// assert_eq!(Edge::try_new(&v[0], &v[0], ()), Err(Error::SameVertex));
    /// ```
    #[inline(always)]
    pub fn try_new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Result<Edge<P, C>> {
        if front == back {
            Err(Error::SameVertex)
        } else {
            Ok(Edge::new_unchecked(front, back, curve))
        }
    }
    /// Generates the edge from `front` to `back`.
    /// # Panic
    /// The condition `front == back` is not allowed.
    /// ```should_panic
    /// use truck_topology::*;
    /// let v = Vertex::new(());
    /// Edge::new(&v, &v, ()); // panic occurs
    /// ```
    #[inline(always)]
    pub fn new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        Edge::try_new(front, back, curve).remove_try()
    }
    /// Generates the edge from `front` to `back`.
    /// # Remarks
    /// This method is prepared only for performance-critical development and is not recommended.  
    /// This method does NOT check the condition `front == back`.  
    /// The programmer must guarantee this condition before using this method.
    #[inline(always)]
    pub fn new_unchecked(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        Edge {
            vertices: (front.clone(), back.clone()),
            orientation: true,
            pcurve: None,
            curve: Arc::new(curve),
        }
    }

    /// Generates the edge from `front` to `back`.
    /// # Remarks
    /// This method check the condition `front == back` in the debug mode.  
    /// The programmer must guarantee this condition before using this method.
    #[inline(always)]
    pub fn debug_new(front: &Vertex<P>, back: &Vertex<P>, curve: C) -> Edge<P, C> {
        match cfg!(debug_assertions) {
            true => Edge::new(front, back, curve),
            false => Edge::new_unchecked(front, back, curve),
        }
    }

    /// Inverts the direction of edge.
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// let mut inv_edge = edge.clone();
    /// inv_edge.invert();
    ///
    /// // Two edges are the same edge.
    /// edge.is_same(&inv_edge);
    ///
    /// // the front and back are exchanged.
    /// assert_eq!(edge.front(), inv_edge.back());
    /// assert_eq!(edge.back(), inv_edge.front());
    /// ```
    #[inline(always)]
    pub fn invert(&mut self) -> &mut Self {
        self.orientation = !self.orientation;
        self
    }

    /// Returns the front vertex
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(edge.front(), &v[0]);
    /// ```
    #[inline(always)]
    pub fn front(&self) -> &Vertex<P> {
        match self.orientation {
            true => &self.vertices.0,
            false => &self.vertices.1,
        }
    }

    /// Returns the back vertex
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(edge.back(), &v[1]);
    /// ```
    #[inline(always)]
    pub fn back(&self) -> &Vertex<P> {
        match self.orientation {
            true => &self.vertices.1,
            false => &self.vertices.0,
        }
    }

    /// Returns the vertices at both ends.
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(edge.ends(), (&v[0], &v[1]));
    /// ```
    #[inline(always)]
    pub fn ends(&self) -> (&Vertex<P>, &Vertex<P>) {
        match self.orientation {
            true => (&self.vertices.0, &self.vertices.1),
            false => (&self.vertices.1, &self.vertices.0),
        }
    }

    /// Returns the vertices at both absolute ends.
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let mut edge = Edge::new(&v[0], &v[1], ());
    /// edge.invert();
    /// assert_eq!(edge.ends(), (&v[1], &v[0]));
    /// assert_eq!(edge.absolute_ends(), (&v[0], &v[1]));
    /// ```
    #[inline(always)]
    pub const fn absolute_ends(&self) -> (&Vertex<P>, &Vertex<P>) {
        (&self.vertices.0, &self.vertices.1)
    }

    /// Returns the clone of the curve.
    /// # Remarks
    /// This method returns absolute curve i.e. does not consider the orientation of curve.
    /// If you want to get a curve compatible with edge's orientation, use `Edge::oriented_curve`.
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[0, 1]);
    /// let mut edge = Edge::new(&v[0], &v[1], (0, 1));
    /// edge.invert();
    ///
    /// // absolute curve
    /// assert_eq!(edge.curve(), (0, 1));
    /// // oriented curve
    /// assert_eq!(edge.oriented_curve(), (1, 0));
    /// ```
    #[inline(always)]
    pub fn curve(&self) -> C
    where
        C: Clone,
    {
        (*self.curve).clone()
    }

    /// Returns how many same edges.
    ///
    /// # Examples
    /// ```
    /// use truck_topology::*;
    ///
    /// // Create one edge
    /// let v = Vertex::news(&[(), ()]);
    /// let e0 = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(e0.count(), 1);
    ///
    /// // Create another edge, independent from e0
    /// let e1 = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(e0.count(), 1);
    ///
    /// // Clone e0, count will be 2
    /// let e2 = e0.clone();
    /// assert_eq!(e0.count(), 2);
    /// assert_eq!(e2.count(), 2);
    ///
    /// // drop e2, count will be 1
    /// drop(e2);
    /// assert_eq!(e0.count(), 1);
    /// ```
    #[inline(always)]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.curve)
    }

    /// Returns the cloned curve in edge.
    /// If edge is inverted, then the returned curve is also inverted.
    #[inline(always)]
    pub fn oriented_curve(&self) -> C
    where
        C: Clone + Invertible,
    {
        match self.orientation {
            true => (*self.curve).clone(),
            false => (*self.curve).inverse(),
        }
    }

    /// Returns a new edge whose curve is mapped by `curve_mapping` and
    /// whose end points are mapped by `point_mapping`.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v0 = Vertex::new(0);
    /// let v1 = Vertex::new(1);
    /// let edge0 = Edge::new(&v0, &v1, 2);
    /// // Reading the edge's own curve inside the closure is safe: geometry
    /// // is immutable, so there is nothing to lock.
    /// let edge1 = edge0
    ///     .try_mapped(
    ///         |i: &usize| {
    ///             let _ = v0.point();
    ///             Some(*i + 1)
    ///         },
    ///         |j: &usize| {
    ///             let _ = edge0.curve();
    ///             Some(*j + 1)
    ///         },
    ///     )
    ///     .unwrap();
    ///
    /// assert_eq!(edge1.front().point(), 1);
    /// assert_eq!(edge1.back().point(), 2);
    /// assert_eq!(edge1.curve(), 3);
    /// ```
    #[inline(always)]
    pub fn try_mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
        mut curve_mapping: impl FnMut(&C) -> Option<D>,
    ) -> Option<Edge<Q, D>> {
        let v0 = self.absolute_front().try_mapped(&mut point_mapping)?;
        let v1 = self.absolute_back().try_mapped(&mut point_mapping)?;
        let curve = curve_mapping(&*self.curve)?;
        let mut edge = Edge::debug_new(&v0, &v1, curve);
        if !self.orientation() {
            edge.invert();
        }
        Some(edge)
    }

    /// Returns a new edge whose curve is mapped by `curve_mapping` and
    /// whose end points are mapped by `point_mapping`.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v0 = Vertex::new(0);
    /// let v1 = Vertex::new(1);
    /// let edge0 = Edge::new(&v0, &v1, 2);
    /// // Reading the edge's own curve inside the closure is safe: geometry
    /// // is immutable, so there is nothing to lock.
    /// let edge1 = edge0.mapped(
    ///     |i: &usize| {
    ///         let _ = v0.point();
    ///         *i as f64 + 0.5
    ///     },
    ///     |j: &usize| {
    ///         let _ = edge0.curve();
    ///         *j as f64 + 0.5
    ///     },
    /// );
    ///
    /// assert_eq!(edge1.front().point(), 0.5);
    /// assert_eq!(edge1.back().point(), 1.5);
    /// assert_eq!(edge1.curve(), 2.5);
    /// ```
    #[inline(always)]
    pub fn mapped<Q, D>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Q,
        mut curve_mapping: impl FnMut(&C) -> D,
    ) -> Edge<Q, D> {
        let v0 = self.absolute_front().mapped(&mut point_mapping);
        let v1 = self.absolute_back().mapped(&mut point_mapping);
        let curve = curve_mapping(&*self.curve);
        let mut edge = Edge::debug_new(&v0, &v1, curve);
        if edge.orientation() != self.orientation() {
            edge.invert();
        }
        edge
    }

    /// Returns the consistence of the geometry of end vertices
    /// and the geometry of edge.
    #[inline(always)]
    pub fn is_geometric_consistent(&self) -> bool
    where
        P: Tolerance,
        C: BoundedCurve<Point = P>,
    {
        let curve = &*self.curve;
        let geom_front = curve.front();
        let geom_back = curve.back();
        let top_front = &*self.absolute_front().point;
        let top_back = &*self.absolute_back().point;
        // FIXME(BG-TOL-001): generic P is bounded Tolerance, not MetricSpace; the bound change is cross-crate and belongs to Stage B
        geom_front.near(top_front) && geom_back.near(top_back)
    }

    /// Cuts the edge at `vertex`.
    /// # Failures
    /// Returns `None` if:
    /// - cannot find the parameter `t` such that `edge.curve().subs(t) == vertex.point()`, or
    /// - the found parameter is not in the parameter range without end points.
    pub fn cut(&self, vertex: &Vertex<P>) -> Option<(Self, Self)>
    where
        P: Clone,
        C: Cut<Point = P> + SearchParameter<D1, Point = P>,
    {
        let ctx = ToleranceCtx::unscaled_legacy();
        let curve0 = self.curve();
        let t = curve0.search_parameter(vertex.point(), None, SEARCH_PARAMETER_TRIALS)?;
        let (t0, t1) = curve0.range_tuple();
        if t < t0 + ctx.ratio_margin() || t1 - ctx.ratio_margin() < t {
            // BG-TOL-001: param
            return None;
        }
        Some(self.pre_cut(vertex, curve0, t))
    }

    /// Cuts the edge at `vertex` with parameter `t`.
    /// # Failure
    /// Returns `None` if `!edge.curve().subs(t).near(&vertex.point())`.
    pub fn cut_with_parameter(&self, vertex: &Vertex<P>, t: f64) -> Option<(Self, Self)>
    where
        P: Clone + Tolerance,
        C: Cut<Point = P>,
    {
        let ctx = ToleranceCtx::unscaled_legacy();
        let curve0 = self.curve();
        // FIXME(BG-TOL-001): generic P is bounded Tolerance, not MetricSpace; the bound change is cross-crate and belongs to Stage B
        if !curve0.subs(t).near(&vertex.point()) {
            return None;
        }
        let (t0, t1) = curve0.range_tuple();
        if t < t0 + ctx.ratio_margin() || t1 - ctx.ratio_margin() < t {
            // BG-TOL-001: param
            return None;
        }
        Some(self.pre_cut(vertex, curve0, t))
    }

    /// Concats two edges.
    pub fn concat(&self, rhs: &Self) -> std::result::Result<Self, ConcatError<P>>
    where
        P: Debug,
        C: Concat<C, Point = P, Output = C> + Invertible + ParameterTransform,
    {
        if self.back() != rhs.front() {
            return Err(ConcatError::DisconnectedVertex(
                self.back().clone(),
                rhs.front().clone(),
            ));
        }
        if self.front() == rhs.back() {
            return Err(ConcatError::SameVertex(self.front().clone()));
        }
        let curve0 = self.oriented_curve();
        let mut curve1 = rhs.oriented_curve();
        let t0 = curve0.range_tuple().1;
        let t1 = curve1.range_tuple().0;
        curve1.parameter_transform(1.0, t0 - t1);
        let curve = curve0.try_concat(&curve1)?;
        Ok(Edge::debug_new(self.front(), rhs.back(), curve))
    }

    /// Create display struct for debugging the edge.
    ///
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// use EdgeDisplayFormat as Edf;
    ///
    /// let vertex_format = VertexDisplayFormat::AsPoint;
    /// let edge = Edge::new(&Vertex::new(0), &Vertex::new(1), 2);
    /// let id = edge.id();
    ///
    /// assert_eq!(
    ///     format!("{:?}", edge.display(Edf::Full { vertex_format })),
    ///     format!("Edge {{ id: {id:?}, vertices: (0, 1), entity: 2 }}"),
    /// );
    /// assert_eq!(
    ///     format!("{:?}", edge.display(Edf::VerticesTupleAndID { vertex_format })),
    ///     format!("Edge {{ id: {id:?}, vertices: (0, 1) }}"),
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", edge.display(Edf::VerticesTupleAndCurve { vertex_format })),
    ///     "Edge { vertices: (0, 1), entity: 2 }",
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", edge.display(Edf::VerticesTupleStruct { vertex_format })),
    ///     "Edge(0, 1)",
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", edge.display(Edf::VerticesTuple { vertex_format })),
    ///     "(0, 1)",
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", edge.display(Edf::AsCurve)),
    ///     "2",
    /// );
    /// ```
    #[inline(always)]
    pub fn display(&self, format: EdgeDisplayFormat) -> DebugDisplay<'_, Self, EdgeDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

impl<P, C, PC> Edge<P, C, PC> {
    /// Returns the orientation of the curve.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge0 = Edge::new(&v[0], &v[1], ());
    /// let edge1 = edge0.inverse();
    /// assert!(edge0.orientation());
    /// assert!(!edge1.orientation());
    /// ```
    #[inline(always)]
    pub const fn orientation(&self) -> bool {
        self.orientation
    }

    /// Returns the front vertex which is generated by constructor
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ()).inverse();
    /// assert_eq!(edge.front(), &v[1]);
    /// assert_eq!(edge.absolute_front(), &v[0]);
    /// ```
    #[inline(always)]
    pub const fn absolute_front(&self) -> &Vertex<P> {
        &self.vertices.0
    }
    /// Returns the back vertex which is generated by constructor
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ()).inverse();
    /// assert_eq!(edge.back(), &v[0]);
    /// assert_eq!(edge.absolute_back(), &v[1]);
    /// ```
    #[inline(always)]
    pub const fn absolute_back(&self) -> &Vertex<P> {
        &self.vertices.1
    }

    /// BG-CE-003: replacement, never in-place mutation. A fresh edge with the
    /// same vertices (same handles — the topology is shared, not copied),
    /// the same orientation and pcurve payload, and the given curve: a new id.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge0 = Edge::new(&v[0], &v[1], 0);
    /// let edge1 = edge0.with_curve(1);
    ///
    /// // The old handle keeps its curve; the replacement has a new id and
    /// // the same end vertices.
    /// assert_eq!(edge0.curve(), 0);
    /// assert_eq!(edge1.curve(), 1);
    /// assert_ne!(edge0.id(), edge1.id());
    /// assert_eq!(edge0.front(), edge1.front());
    /// assert_eq!(edge0.back(), edge1.back());
    /// ```
    #[inline(always)]
    pub fn with_curve(&self, curve: C) -> Edge<P, C, PC>
    where
        PC: Clone,
    {
        Edge {
            vertices: self.vertices.clone(),
            orientation: self.orientation,
            pcurve: self.pcurve.clone(),
            curve: Arc::new(curve),
        }
    }

    /// The shared entity curve by reference — no lock, no clone.
    #[inline(always)]
    pub fn shared_curve(&self) -> &C {
        &self.curve
    }

    /// Returns the parametric trace of this edge use on its owning face,
    /// if one has been attached.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// assert_eq!(edge.pcurve(), None);
    /// let edge = edge.with_pcurve(42i32);
    /// assert_eq!(edge.pcurve(), Some(&42i32));
    /// ```
    #[inline(always)]
    pub fn pcurve(&self) -> Option<&PC> {
        self.pcurve.as_ref()
    }

    /// Attaches `pcurve` to this edge use, returning the updated handle.
    /// The curve, the vertices and the orientation are untouched: this is
    /// the same use of the same curve, now carrying its trace.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let original = Edge::new(&v[0], &v[1], ());
    /// let edge = original.clone().with_pcurve(42i32);
    /// assert_eq!(edge.pcurve(), Some(&42i32));
    /// assert!(edge.is_same(&original));
    /// assert_eq!(edge.id(), original.id());
    /// assert_eq!(original.pcurve(), None);
    /// ```
    #[inline(always)]
    pub fn with_pcurve<Q>(self, pcurve: Q) -> Edge<P, C, Q> {
        Edge {
            vertices: self.vertices,
            orientation: self.orientation,
            pcurve: Some(pcurve),
            curve: self.curve,
        }
    }

    /// Creates the inverse oriented edge.
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge = Edge::new(&v[0], &v[1], ());
    /// let inv_edge = edge.inverse();
    ///
    /// // Two edges are the same edge.
    /// assert!(edge.is_same(&inv_edge));
    ///
    /// // Two edges has the same id.
    /// assert_eq!(edge.id(), inv_edge.id());
    ///
    /// // the front and back are exchanged.
    /// assert_eq!(edge.front(), inv_edge.back());
    /// assert_eq!(edge.back(), inv_edge.front());
    /// ```
    #[inline(always)]
    pub fn inverse(&self) -> Edge<P, C, PC>
    where
        PC: Clone,
    {
        Edge {
            vertices: self.vertices.clone(),
            orientation: !self.orientation,
            pcurve: self.pcurve.clone(),
            curve: Arc::clone(&self.curve),
        }
    }

    /// Returns a clone of the edge without inversion.
    /// # Examples
    /// ```
    /// use truck_topology::{Vertex, Edge};
    /// let v = Vertex::news(&[(), ()]);
    /// let edge0 = Edge::new(&v[0], &v[1], ());
    /// let edge1 = edge0.inverse();
    /// let edge2 = edge1.absolute_clone();
    /// assert_eq!(edge0, edge2);
    /// assert_ne!(edge1, edge2);
    /// assert!(edge1.is_same(&edge2));
    /// ```
    #[inline(always)]
    pub fn absolute_clone(&self) -> Self
    where
        PC: Clone,
    {
        Self {
            vertices: self.vertices.clone(),
            orientation: true,
            pcurve: self.pcurve.clone(),
            curve: Arc::clone(&self.curve),
        }
    }

    /// Returns whether two edges are the same. Returns `true` even if the orientaions are different.
    /// ```
    /// use truck_topology::{Vertex, Edge};
    /// let v = Vertex::news(&[(), ()]);
    /// let edge0 = Edge::new(&v[0], &v[1], ());
    /// let edge1 = Edge::new(&v[0], &v[1], ());
    /// let edge2 = edge0.clone();
    /// let edge3 = edge0.inverse();
    /// assert!(!edge0.is_same(&edge1)); // edges whose ids are different are not the same.
    /// assert!(edge0.is_same(&edge2)); // The cloned edge is the same edge.
    /// assert!(edge0.is_same(&edge3)); // The inversed edge is the "same" edge
    /// ```
    #[inline(always)]
    pub fn is_same<Q>(&self, other: &Edge<P, C, Q>) -> bool {
        self.id() == other.id()
    }

    /// Returns the id that does not depend on the direction of the edge.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), ()]);
    /// let edge0 = Edge::new(&v[0], &v[1], ());
    /// let edge1 = edge0.inverse();
    /// assert_ne!(edge0, edge1);
    /// assert_eq!(edge0.id(), edge1.id());
    /// ```
    #[inline(always)]
    pub fn id(&self) -> EdgeID<C> {
        ID::new(Arc::as_ptr(&self.curve))
    }

    #[inline(always)]
    fn pre_cut(&self, vertex: &Vertex<P>, mut curve0: C, t: f64) -> (Self, Self)
    where
        C: Cut<Point = P>,
    {
        let curve1 = curve0.cut(t);
        // Restricting an arbitrary `PC` needs a `Cut` bound this packet does not
        // add, and carrying the *full* trace on both halves would over-approximate,
        // so the halves drop the pcurve; the packet that wires real pcurves owns
        // trace splitting.
        let edge0 = Edge {
            vertices: (self.absolute_front().clone(), vertex.clone()),
            orientation: self.orientation,
            pcurve: None,
            curve: Arc::new(curve0),
        };
        let edge1 = Edge {
            vertices: (vertex.clone(), self.absolute_back().clone()),
            orientation: self.orientation,
            pcurve: None,
            curve: Arc::new(curve1),
        };
        match self.orientation {
            true => (edge0, edge1),
            false => (edge1, edge0),
        }
    }
}

/// Error for concat
#[derive(Clone, Debug, Error)]
pub enum ConcatError<P: Debug> {
    /// Failed to concat edges since the end point of the first curve is different from the start point of the second curve.
    #[error("The end point {0:?} of the first curve is different from the start point {1:?} of the second curve.")]
    DisconnectedVertex(Vertex<P>, Vertex<P>),
    #[error("The end vertices are the same vertex {0:?}.")]
    SameVertex(Vertex<P>),
    /// From geometric error.
    #[error("{0}")]
    FromGeometry(truck_geotrait::ConcatError<P>),
}

impl<P: Debug> From<truck_geotrait::ConcatError<P>> for ConcatError<P> {
    fn from(err: truck_geotrait::ConcatError<P>) -> ConcatError<P> {
        ConcatError::FromGeometry(err)
    }
}

impl<P, C, PC: Clone> Clone for Edge<P, C, PC> {
    #[inline(always)]
    fn clone(&self) -> Edge<P, C, PC> {
        Edge {
            vertices: self.vertices.clone(),
            orientation: self.orientation,
            pcurve: self.pcurve.clone(),
            curve: Arc::clone(&self.curve),
        }
    }
}

impl<P, C, PC> PartialEq for Edge<P, C, PC> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(Arc::as_ptr(&self.curve), Arc::as_ptr(&other.curve))
            && self.orientation == other.orientation
    }
}

impl<P, C, PC> Eq for Edge<P, C, PC> {}

impl<P, C, PC> Hash for Edge<P, C, PC> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.curve), state);
        self.orientation.hash(state);
    }
}

impl<P: Debug, C: Debug> Debug for DebugDisplay<'_, Edge<P, C>, EdgeDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            EdgeDisplayFormat::Full { vertex_format } => f
                .debug_struct("Edge")
                .field("id", &Arc::as_ptr(&self.entity.curve))
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .field("entity", &*self.entity.curve)
                .finish(),
            EdgeDisplayFormat::VerticesTupleAndID { vertex_format } => f
                .debug_struct("Edge")
                .field("id", &self.entity.id())
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .finish(),
            EdgeDisplayFormat::VerticesTupleAndCurve { vertex_format } => f
                .debug_struct("Edge")
                .field(
                    "vertices",
                    &(
                        self.entity.front().display(vertex_format),
                        self.entity.back().display(vertex_format),
                    ),
                )
                .field("entity", &*self.entity.curve)
                .finish(),
            EdgeDisplayFormat::VerticesTupleStruct { vertex_format } => f
                .debug_tuple("Edge")
                .field(&self.entity.front().display(vertex_format))
                .field(&self.entity.back().display(vertex_format))
                .finish(),
            EdgeDisplayFormat::VerticesTuple { vertex_format } => f.write_fmt(format_args!(
                "({:?}, {:?})",
                self.entity.front().display(vertex_format),
                self.entity.back().display(vertex_format),
            )),
            EdgeDisplayFormat::AsCurve => f.write_fmt(format_args!("{:?}", *self.entity.curve)),
        }
    }
}

#[cfg(test)]
mod coedge_tests {
    #![deny(clippy::unwrap_used)]
    use super::*;
    use std::ops::Bound;

    #[derive(Clone, Debug)]
    struct TestCutCurve(usize, usize);
    impl ParametricCurve for TestCutCurve {
        type Point = usize;
        type Vector = usize;
        fn subs(&self, t: f64) -> usize {
            if t < 0.5 {
                self.0
            } else {
                self.1
            }
        }
        fn der(&self, _: f64) -> usize {
            self.1 - self.0
        }
        fn der2(&self, _: f64) -> usize {
            self.1 - self.0
        }
        fn der_n(&self, _: usize, _: f64) -> usize {
            self.1 - self.0
        }
        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(0.0), Bound::Included(1.0))
        }
    }
    impl BoundedCurve for TestCutCurve {}
    impl Cut for TestCutCurve {
        fn cut(&mut self, _t: f64) -> Self {
            self.clone()
        }
    }

    #[test]
    fn pcurve_defaults_to_none_and_stays_out_of_identity() {
        let v = Vertex::news([(), ()]);
        let base = Edge::new(&v[0], &v[1], ());
        assert_eq!(base.pcurve(), None);

        let e0 = base.clone().with_pcurve(1i32);
        let e1 = base.clone().with_pcurve(2i32);
        assert_eq!(e0, e1);
        let mut set = std::collections::HashSet::new();
        set.insert(e0);
        set.insert(e1);
        assert_eq!(set.len(), 1);

        assert_ne!(base, base.inverse());
    }

    #[test]
    fn with_pcurve_sets_payload_and_shares_the_curve() {
        let v = Vertex::news([(), ()]);
        let original = Edge::new(&v[0], &v[1], ());
        let edge = original.clone().with_pcurve(42i32);
        assert_eq!(edge.pcurve(), Some(&42i32));
        assert!(edge.is_same(&original));
        assert_eq!(edge.id(), original.id());
        assert_eq!(original.pcurve(), None);
    }

    #[test]
    fn inverse_absolute_clone_and_clone_carry_pcurve() {
        let v = Vertex::news([(), ()]);
        let edge = Edge::new(&v[0], &v[1], ()).with_pcurve(7i32);
        assert_eq!(edge.inverse().pcurve(), Some(&7i32));
        assert_eq!(edge.absolute_clone().pcurve(), Some(&7i32));
        assert_eq!(edge.clone().pcurve(), Some(&7i32));
    }

    #[test]
    fn with_curve_preserves_topology() {
        let v = Vertex::news([(), ()]);
        let e = Edge::new(&v[0], &v[1], ());
        let e2 = e.with_curve(());
        assert_eq!(e.front().id(), e2.front().id());
        assert_eq!(e.back().id(), e2.back().id());
        assert_eq!(e.orientation(), e2.orientation());
        assert_eq!(e.pcurve(), e2.pcurve());
        assert_ne!(e.id(), e2.id());
        assert_eq!(e.curve(), ());
    }

    #[test]
    fn cut_drops_pcurve_on_both_halves() {
        let v = Vertex::news([0usize, 1usize]);
        let cut_vertex = Vertex::new(2usize);
        let edge = Edge::new(&v[0], &v[1], TestCutCurve(0, 1)).with_pcurve(5i32);
        let (h0, h1) = edge.pre_cut(&cut_vertex, TestCutCurve(0, 1), 0.5);
        assert_eq!(h0.pcurve(), None);
        assert_eq!(h1.pcurve(), None);
    }
}
