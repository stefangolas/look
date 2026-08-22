use crate::*;

impl<P> Vertex<P> {
    /// constructor
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v0 = Vertex::new(()); // a vertex whose geometry is the empty tuple.
    /// let v1 = Vertex::new(()); // another vertex
    /// let v2 = v0.clone(); // a cloned vertex
    /// assert_ne!(v0, v1);
    /// assert_eq!(v0, v2);
    /// ```
    #[inline(always)]
    pub fn new(point: P) -> Vertex<P> {
        Vertex {
            point: Arc::new(point),
        }
    }

    /// Creates `len` distinct vertices and return them by vector.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v = Vertex::news(&[(), (), ()]);
    /// assert_eq!(v.len(), 3);
    /// assert_ne!(v[0], v[2]);
    /// ```
    #[inline(always)]
    pub fn news(points: impl AsRef<[P]>) -> Vec<Vertex<P>>
    where
        P: Copy,
    {
        points.as_ref().iter().map(|p| Vertex::new(*p)).collect()
    }

    /// Returns the point of vertex.
    ///
    /// Geometry is immutable: to change the point, construct a new vertex
    /// with [`Vertex::new`]; existing handles keep the old point.
    #[inline(always)]
    pub fn point(&self) -> P
    where
        P: Clone,
    {
        (*self.point).clone()
    }

    /// Returns vertex whose point is converted by `point_mapping`.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v0 = Vertex::new(0);
    /// // Reading a vertex's own point inside the closure is safe: geometry
    /// // is immutable, so there is nothing to lock.
    /// let v1 = v0.try_mapped(|p| {
    ///     let _ = v0.point();
    ///     Some(*p + 1)
    /// });
    /// assert_eq!(v1.map(|v| v.point()), Some(1));
    /// ```
    #[inline(always)]
    pub fn try_mapped<Q>(
        &self,
        mut point_mapping: impl FnMut(&P) -> Option<Q>,
    ) -> Option<Vertex<Q>> {
        Some(Vertex::new(point_mapping(&*self.point)?))
    }

    /// Returns vertex whose point is converted by `point_mapping`.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// let v0 = Vertex::new(2);
    /// // Reading a vertex's own point inside the closure is safe: geometry
    /// // is immutable, so there is nothing to lock.
    /// let v1 = v0.mapped(|p| {
    ///     let _ = v0.point();
    ///     *p as f64 + 0.5
    /// });
    /// assert_eq!(v1.point(), 2.5);
    /// ```
    #[inline(always)]
    pub fn mapped<Q>(&self, mut point_mapping: impl FnMut(&P) -> Q) -> Vertex<Q> {
        Vertex::new(point_mapping(&*self.point))
    }

    /// Returns the id of the vertex.
    #[inline(always)]
    pub fn id(&self) -> VertexID<P> {
        ID::new(Arc::as_ptr(&self.point))
    }

    /// Returns how many same vertices.
    ///
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// // Create one vertex
    /// let v0 = Vertex::new(());
    /// assert_eq!(v0.count(), 1);
    /// // Create another vertex, independent from v0
    /// let v1 = Vertex::new(());
    /// assert_eq!(v0.count(), 1);
    /// // Clone v0, count will be 2
    /// let v2 = v0.clone();
    /// assert_eq!(v0.count(), 2);
    /// assert_eq!(v2.count(), 2);
    /// // drop v2, count will be 1
    /// drop(v2);
    /// assert_eq!(v0.count(), 1);
    /// ```
    #[inline(always)]
    pub fn count(&self) -> usize {
        Arc::strong_count(&self.point)
    }

    /// Create display struct for debugging the vertex.
    /// # Examples
    /// ```
    /// use truck_topology::*;
    /// use VertexDisplayFormat as VDF;
    /// let v = Vertex::new([0, 2]);
    /// assert_eq!(
    ///     format!("{:?}", v.display(VDF::Full)),
    ///     format!("Vertex {{ id: {:?}, entity: [0, 2] }}", v.id()),
    /// );
    /// assert_eq!(
    ///     format!("{:?}", v.display(VDF::IDTuple)),
    ///     format!("Vertex({:?})", v.id()),
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", v.display(VDF::PointTuple)),
    ///     "Vertex([0, 2])",
    /// );
    /// assert_eq!(
    ///     &format!("{:?}", v.display(VDF::AsPoint)),
    ///     "[0, 2]",
    /// );
    /// ```
    #[inline(always)]
    pub fn display(
        &self,
        format: VertexDisplayFormat,
    ) -> DebugDisplay<'_, Self, VertexDisplayFormat> {
        DebugDisplay {
            entity: self,
            format,
        }
    }
}

impl<P> Clone for Vertex<P> {
    #[inline(always)]
    fn clone(&self) -> Vertex<P> {
        Vertex {
            point: Arc::clone(&self.point),
        }
    }
}

impl<P> PartialEq for Vertex<P> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl<P> Eq for Vertex<P> {}

impl<P> Hash for Vertex<P> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.point), state);
    }
}

impl<P: Debug> Debug for DebugDisplay<'_, Vertex<P>, VertexDisplayFormat> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.format {
            VertexDisplayFormat::Full => f
                .debug_struct("Vertex")
                .field("id", &Arc::as_ptr(&self.entity.point))
                .field("entity", &*self.entity.point)
                .finish(),
            VertexDisplayFormat::IDTuple => {
                f.debug_tuple("Vertex").field(&self.entity.id()).finish()
            }
            VertexDisplayFormat::PointTuple => {
                f.debug_tuple("Vertex").field(&*self.entity.point).finish()
            }
            VertexDisplayFormat::AsPoint => f.write_fmt(format_args!("{:?}", *self.entity.point)),
        }
    }
}

#[cfg(test)]
mod vertex_tests {
    #![deny(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn vertex_replacement_changes_id_not_old_handles() {
        let v0 = Vertex::new(0);
        let h = v0.clone();
        let v2 = Vertex::new(1);
        assert_ne!(v0.id(), v2.id());
        assert_eq!(v0.point(), 0);
        assert_eq!(h.point(), 0);
    }

    #[test]
    fn mapped_closure_may_access_geometry() {
        let v0 = Vertex::new(0);
        let v1 = v0.mapped(|p| {
            let _ = v0.point();
            *p
        });
        assert_eq!(v1.point(), 0);
    }
}
