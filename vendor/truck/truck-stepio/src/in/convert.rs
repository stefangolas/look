use super::*;
use crate::common::PartAttrs;

impl Table {
    fn place_holder_edge_any_to_index_and_edge_curve(
        &self,
        edge: &PlaceHolder<EdgeAnyHolder>,
    ) -> Option<(u64, EdgeCurveHolder)> {
        use PlaceHolder::Ref;
        let Ref(Name::Entity(ref idx)) = edge else {
            return None;
        };
        self.oriented_edge
            .get(idx)
            .and_then(|oriented_edge| {
                Some((
                    oriented_edge.edge_element_idx()?,
                    oriented_edge.edge_element_holder(self)?,
                ))
            })
            .or_else(|| {
                self.edge_curve
                    .get(idx)
                    .map(|edge_curve| (*idx, edge_curve.clone()))
            })
    }
    fn face_any_to_orientation_and_face(
        &self,
        face: Option<FaceAnyHolder>,
    ) -> Option<(bool, FaceSurfaceHolder)> {
        let (orientation, _, face) = self.face_any_resolved(face)?;
        Some((orientation, face))
    }

    /// Resolve a shell's face reference to its orientation, the id of the
    /// `FACE_SURFACE` that *defines* it, and the definition itself.
    ///
    /// The definition id is deliberately kept apart from whatever the shell
    /// named to get here. An `ORIENTED_FACE` is a *use*: it contributes
    /// orientation, and several uses may resolve to one `FACE_SURFACE`.
    /// Reporting the use where the definition is meant — or the reverse — makes
    /// a wrong shell-use orientation indistinguishable from a wrong underlying
    /// face, which is precisely the distinction this stage exists to preserve.
    fn face_any_resolved(
        &self,
        face: Option<FaceAnyHolder>,
    ) -> Option<(bool, FaceReference, FaceSurfaceHolder)> {
        match face? {
            // The shell named the definition directly. There is no use entity
            // in this file at all, and claiming one — by copying the shell's
            // reference into the use slot — would be the very conflation this
            // split exists to remove.
            FaceAnyHolder::FaceSurface(face) => Some((true, FaceReference::Definition, face)),
            FaceAnyHolder::OrientedFace(oriented_face) => {
                let definition_id = match &oriented_face.face_element {
                    PlaceHolder::Ref(Name::Entity(idx)) => Some(*idx),
                    // An inlined definition has no id to report.
                    _ => None,
                };
                let face_element = oriented_face.face_element_holder(self)?;
                Some((
                    oriented_face.orientation,
                    FaceReference::Use { definition_id },
                    face_element,
                ))
            }
        }
    }

    fn shell_vertices(&self, shell: &ShellHolder) -> Arena<VertexKind, Point3> {
        use PlaceHolder::Ref;
        // This carried the reserve-before-convert defect that was already fixed
        // for edges: the position was inserted into the map, and only then was
        // `get_owned` called, which can fail. The point was never pushed but the
        // entry stayed, so every subsequent vertex was addressed one slot past
        // where it sat. The arena removes the ordering question entirely — there
        // is no way to express the claim before the conversion.
        let mut arena = Arena::new();
        let vertex_holders = shell
            .cfs_faces_holder(self)
            .filter_map(move |(_, face)| self.face_any_to_orientation_and_face(face))
            .flat_map(move |(_, face)| face.bounds_holder(self))
            .filter_map(move |bound| match bound?.bound_holder(self)? {
                FaceBoundLoop::Edges(loop_) => Some(loop_),
                // A collapsed bound has no edges to contribute.
                FaceBoundLoop::Collapsed(_) => None,
            })
            .flat_map(move |bound| bound.edge_list)
            .filter_map(move |edge| self.place_holder_edge_any_to_index_and_edge_curve(&edge))
            .flat_map(move |(_, edge)| [edge.edge_start, edge.edge_end]);
        for holder in vertex_holders {
            let Ref(Name::Entity(idx)) = holder else {
                continue;
            };
            arena.get_or_try_insert(VertexPointId::new(idx), || {
                let point = EntityTable::<VertexPointHolder>::get_owned(self, idx)
                    .map_err(|e| eprintln!("{e}"))
                    .ok()?;
                Some(Point3::from(&point.vertex_geometry))
            });
        }
        arena
    }

    /// The `TRUCK_PROBE_IDENTITY` probe used to live here, checking after the
    /// fact that every mapped index addressed the edge it named. It is gone
    /// because the question it asked can no longer have a bad answer:
    /// [`Arena::get_or_try_insert`] converts before it claims a position, so
    /// map and vector cannot disagree (`TOP-002`). That is the promotion from
    /// diagnostic to certificate the plan calls for — the check moved from
    /// runtime to the shape of the code, and its regression lives in
    /// `arena::tests`.
    fn shell_edges(
        &self,
        shell: &ShellHolder,
        vertices: &Arena<VertexKind, Point3>,
    ) -> Arena<EdgeKind, CompressedEdge<Curve3D>> {
        use PlaceHolder::Ref;
        let mut arena = Arena::new();
        let edge_curves = shell
            .cfs_faces_holder(self)
            .filter_map(move |(_, face)| self.face_any_to_orientation_and_face(face))
            .flat_map(move |(_, face)| face.bounds_holder(self))
            .filter_map(move |bound| match bound?.bound_holder(self)? {
                FaceBoundLoop::Edges(loop_) => Some(loop_),
                // A collapsed bound has no edges to contribute.
                FaceBoundLoop::Collapsed(_) => None,
            })
            .flat_map(move |bound| bound.edge_list)
            .filter_map(move |edge| self.place_holder_edge_any_to_index_and_edge_curve(&edge));
        for (idx, edge) in edge_curves {
            arena.get_or_try_insert(EdgeCurveId::new(idx), move || {
                let edge_curve = edge
                    .clone()
                    .into_owned(self)
                    .map_err(|e| eprintln!("{e}"))
                    .ok()?;
                let curve = edge_curve
                    .parse_curve3d()
                    .map_err(|e| eprintln!("{e}"))
                    .ok()?;
                let Ref(Name::Entity(front_idx)) = edge.edge_start else {
                    return None;
                };
                let Ref(Name::Entity(back_idx)) = edge.edge_end else {
                    return None;
                };
                // An edge whose endpoints did not convert is not an edge. The
                // bare `usize` pair is what `CompressedEdge` demands, so this is
                // one of the few places a `VertexIndex` has to be unwrapped, and
                // it happens only after the lookup proved the vertex exists.
                let endpoints = (
                    vertices.index_of(VertexPointId::new(front_idx))?.position(),
                    vertices.index_of(VertexPointId::new(back_idx))?.position(),
                );
                Some(CompressedEdge {
                    vertices: endpoints,
                    curve,
                })
            });
        }
        arena
    }
    /// Follow a source reference to the position of the edge it names, having
    /// checked that the position addresses that edge and no other.
    ///
    /// `TOP-001` at the point of use (`MATHEMATICAL_FOUNDATION.md` §22.2). The
    /// arena's construction already makes the answer right, so this compare
    /// never fires in a correct build; what it buys is that if it ever does,
    /// the model reports which entity was asked for and which one was stored,
    /// instead of rendering a curve from a neighbouring surface as a smooth
    /// wrong region. One integer comparison per edge use — structural tier.
    fn checked_edge_position(
        edges: &Arena<EdgeKind, CompressedEdge<Curve3D>>,
        named: EdgeCurveId,
    ) -> Option<usize> {
        let index = edges.index_of(named)?;
        if let Err(mismatch) = edges.get_checked(index, named) {
            eprintln!("{mismatch}");
            return None;
        }
        Some(index.position())
    }

    fn face_bound_to_edges(
        &self,
        bound: FaceBoundHolder,
        edges: &Arena<EdgeKind, CompressedEdge<Curve3D>>,
    ) -> Result<BoundOutcome, FaceLossReason> {
        use PlaceHolder::Ref;
        let ori = bound.orientation;
        let bound = match bound
            .bound_holder(self)
            .ok_or(FaceLossReason::LoopReferenceUnresolved)?
        {
            FaceBoundLoop::Edges(loop_) => loop_,
            // A collapsed boundary trims nothing. The apex or pole is closed by
            // the surface's own degeneracy, so the honest contribution is no
            // trim segment at all — not a synthesised loop of zero size, which
            // would trim the face by an empty region and delete it.
            FaceBoundLoop::Collapsed(vl) => {
                let pt = match &vl.loop_vertex {
                    PlaceHolder::Ref(Name::Entity(v_idx)) => {
                        EntityTable::<VertexPointHolder>::get_owned(self, *v_idx)
                            .map(|p| Point3::from(&p.vertex_geometry))
                            .unwrap_or_else(|_| Point3::origin())
                    }
                    _ => Point3::origin(),
                };
                return Ok(BoundOutcome::Collapsed(pt));
            }
        };
        // A bound missing an edge is a broken bound, not a shorter one.
        //
        // This collected through `filter_map`, so every `?` below dropped that
        // edge from the wire and let the rest through. The result is a wire that
        // no longer closes: the gap is bridged by whatever the next stage joins
        // it to, and the face is then trimmed by a region its own file never
        // described. Nothing downstream can detect this, because a short wire is
        // indistinguishable from a wire that was always that shape.
        //
        // Collecting into `Option<Vec<_>>` makes the conversion total — every
        // `ORIENTED_EDGE` the bound names resolves, or the bound does not exist.
        // That also discharges the source-use versus resolved-use count check by
        // construction rather than by assertion: the collect yields exactly as
        // many indices as `edge_list` held, or it yields nothing.
        let source_uses = bound.edge_list.len();
        let mut wire: Vec<CompressedEdgeIndex> = bound
            .edge_list
            .into_iter()
            .map(|edge| {
                let Ref(Name::Entity(ref idx)) = edge else {
                    return Err(FaceLossReason::EdgeUseUnresolved);
                };
                let edge_idx = if let Some(oriented_edge) = self.oriented_edge.get(idx) {
                    let named = EdgeCurveId::new(
                        oriented_edge
                            .edge_element_idx()
                            .ok_or(FaceLossReason::EdgeUseUnresolved)?,
                    );
                    CompressedEdgeIndex {
                        index: Self::checked_edge_position(edges, named)
                            .ok_or(FaceLossReason::EdgeCurveConversionFailed)?,
                        orientation: oriented_edge.orientation == ori,
                    }
                } else {
                    CompressedEdgeIndex {
                        index: Self::checked_edge_position(edges, EdgeCurveId::new(*idx))
                            .ok_or(FaceLossReason::EdgeCurveConversionFailed)?,
                        orientation: ori,
                    }
                };
                Ok(edge_idx)
            })
            .collect::<Result<Vec<_>, FaceLossReason>>()?;
        debug_assert!(
            wire.is_empty() || wire.len() == source_uses,
            "a resolved bound must use every edge its source named"
        );
        if !ori {
            wire.reverse();
        }
        // Every edge resolved, but that does not make this a boundary. The wire
        // has to close, and it is checked in traversal order — after the
        // reversal, since that is the order the face will be trimmed in.
        TopologicallyClosedWire::try_new(wire, |position| {
            edges.value_at(position).map(|edge| edge.vertices)
        })
        .map(BoundOutcome::Wire)
        .ok_or(FaceLossReason::WireNotClosed)
    }

    /// The supporting surface of a face, converted once per source entity.
    ///
    /// Surfaces are the third entity kind to resolve through the one generic
    /// [`Arena`], and §51a is the whole reason it is that arena rather than a
    /// third hand-written map: the reserve-before-convert defect was repaired
    /// in the edge path and survived untouched in the vertex path for exactly
    /// as long as the repair was site-local. A contract is discharged when the
    /// invalid transition has one implementation that cannot express the bad
    /// state — not when every site anyone has looked at is correct.
    ///
    /// The arena stores the surface **as the file describes it**. `same_sense`
    /// inversion is a property of the *face*, so it is applied to the copy a
    /// face takes and never to the canonical entity: two faces may share one
    /// `CYLINDRICAL_SURFACE` and disagree about sense, and inverting in place
    /// would let the first face rewrite the second's geometry.
    ///
    /// **Cost** (§29a): one retained surface per distinct source entity for the
    /// duration of one shell, against one conversion per *face* before. Shared
    /// surfaces now convert once; the arena is dropped when the shell's faces
    /// are built. `CompressedFace` still owns its surface by value, so the copy
    /// is unavoidable until §33a item 11 replaces it with a `SurfaceIndex`.
    ///
    /// **Contracts:** `TOP-001`, `TOP-002`, `TOP-007` for surfaces.
    fn face_surface(
        &self,
        face: &FaceSurfaceHolder,
        surfaces: &mut Arena<SurfaceKind, Surface>,
    ) -> Option<(Surface, Option<u64>)> {
        let convert = || {
            let step_surface: SurfaceAny = face
                .face_geometry
                .clone()
                .into_owned(self)
                .map_err(|e| eprintln!("{e}"))
                .ok()?;
            Surface::try_from(&step_surface)
                .map_err(|e| eprintln!("{e}"))
                .ok()
        };
        // An inline owned surface has no entity id, so there is no identity to
        // be canonical about: it belongs to this face alone, and it has no
        // surface provenance to report.
        let PlaceHolder::Ref(Name::Entity(idx)) = &face.face_geometry else {
            return Some((convert()?, None));
        };
        let named = SurfaceId::new(*idx);
        let index = surfaces.get_or_try_insert(named, convert)?;
        let surface = surfaces
            .get_checked(index, named)
            .map_err(|mismatch| eprintln!("{mismatch}"))
            .ok()
            .cloned()?;
        Some((surface, Some(*idx)))
    }

    /// Whether an untrimmed carrier is a closed bounded surface that a
    /// point-degenerate-only boundary can validly describe.
    ///
    /// A `VERTEX_LOOP` (or a face declaring no bounds at all) trims nothing, so
    /// the whole surface is the face. That is only a well-formed statement on a
    /// surface with no boundary of its own — a closed surface like a sphere.
    /// On an unbounded or open carrier it would invent geometry.
    fn closed_surface_accepts_untrimmed(surface: &Surface) -> bool {
        matches!(
            surface,
            Surface::ElementarySurface(ElementarySurface::Sphere(_))
        )
    }

    fn tune_conical_surface(
        surface: &mut Surface,
        wires: &[TopologicallyClosedWire],
        edges: &Arena<EdgeKind, CompressedEdge<Curve3D>>,
        collapsed_pts: &[Point3],
    ) {
        use cgmath::Transform;
        use step_geometry::*;
        let Surface::ElementarySurface(ElementarySurface::ConicalSurface(ref mut processor)) =
            surface
        else {
            return;
        };

        let mut points: Vec<Point3> = collapsed_pts.to_vec();
        for wire in wires {
            for edge_idx in wire.edges() {
                if let Some(edge) = edges.value_at(edge_idx.index) {
                    let (t0, t1) = edge.curve.range_tuple();
                    points.push(edge.curve.subs(t0));
                    points.push(edge.curve.subs(t1));
                    points.push(edge.curve.subs(t0 * 0.5 + t1 * 0.5));
                }
            }
        }

        if points.is_empty() {
            return;
        }

        let inv_mat = match processor.transform().invert() {
            Some(m) => m,
            None => return,
        };

        let rev = processor.entity();
        let line = rev.entity_curve();
        let p0 = line.0;
        let p1 = line.1;
        let dr = p1.x - p0.x;
        let dz = p1.z - p0.z;

        if dz.abs() < 1.0e-12 {
            return;
        }

        let tan = dr / dz;
        let r0 = p0.x - p0.z * tan;
        let z_apex = if tan.abs() > 1.0e-12 { -r0 / tan } else { 0.0 };

        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;

        for p in &points {
            let p_local = inv_mat.transform_point(*p);
            let z = p_local.z;
            z_min = z_min.min(z);
            z_max = z_max.max(z);
        }

        if !collapsed_pts.is_empty() || (z_min.is_finite() && (z_min - z_apex).abs() < 1.0e-3) {
            z_min = z_min.min(z_apex);
        }

        if !z_min.is_finite() || !z_max.is_finite() {
            z_min = z_apex.min(0.0);
            z_max = z_apex.max(0.0) + 1.0;
        }

        let span = (z_max - z_min).max(1.0);
        let pad = (0.2 * span).max(0.1);
        let u_min = z_min - pad;
        let u_max = z_max + pad;

        let new_p0 = Point3::new(r0 + u_min * tan, 0.0, u_min);
        let new_p1 = Point3::new(r0 + u_max * tan, 0.0, u_max);
        let new_rev = RevolutedCurve::by_revolution(
            Line(new_p0, new_p1),
            Point3::origin(),
            Vector3::unit_z(),
        );

        *processor.entity_mut() = new_rev;
    }

    /// Convert a shell's faces, and say why each one that failed did.
    ///
    /// The reasons come from the real conversion, not from a second
    /// reimplementation of it that reports instead of converting. That
    /// distinction is the whole design: a diagnostic which re-derives the
    /// pipeline can disagree with the pipeline, and this project has twice had
    /// a detector carrying the bug it was hunting. There is one path, and it
    /// either produces a face or a reason.
    fn shell_faces(
        &self,
        shell: &ShellHolder,
        edges: &Arena<EdgeKind, CompressedEdge<Curve3D>>,
        losses: &mut Vec<FaceLoss>,
        singular: &mut Vec<FaceProvenance>,
    ) -> Vec<CompressedFace<Surface>> {
        let mut surfaces = Arena::<SurfaceKind, Surface>::new();
        let mut faces = Vec::new();
        // One explicit loop rather than chained `filter_map`s: two closures
        // cannot both hold `&mut losses`, and threading a cell through to keep
        // the iterator style would be ceremony in exchange for nothing.
        for (shell_ref, face) in shell.cfs_faces_holder(self) {
            let Some((orientation, reference, face)) = self.face_any_resolved(face) else {
                // The shell named something that is not a face it can resolve.
                // All that is known is the reference itself.
                losses.push(FaceLoss {
                    provenance: FaceProvenance {
                        use_id: shell_ref.map(SourceEntityId::new),
                        ..FaceProvenance::default()
                    },
                    reason: FaceLossReason::FaceReferenceUnresolved,
                });
                continue;
            };
            // The shell's own reference lands in whichever slot it actually
            // names, and the other stays empty unless the file supplied it.
            let (use_id, definition_id) = match reference {
                FaceReference::Definition => (None, shell_ref),
                FaceReference::Use { definition_id } => (shell_ref, definition_id),
            };
            let partial = FaceProvenance {
                use_id: use_id.map(SourceEntityId::new),
                definition_id: definition_id.map(SourceEntityId::new),
                surface_id: None,
                // Established below, once the bounds have been read.
                outer_bound: OuterBoundStanding::NotRetained,
            };
            let Some((mut surface, surface_id)) = self.face_surface(&face, &mut surfaces) else {
                losses.push(FaceLoss {
                    provenance: partial,
                    reason: FaceLossReason::SurfaceConversionFailed,
                });
                continue;
            };
            let provenance = FaceProvenance {
                surface_id: surface_id.map(SourceEntityId::new),
                ..partial
            };
            if !face.same_sense && std::env::var_os("TRUCK_NO_INVERT").is_none() {
                surface.invert()
            }
            // Same rule one level up: a face missing a bound is a broken face,
            // not a simpler one. Dropping a failed bound here silently rewrites
            // what the solid is -- lose an inner bound and a hole fills in, lose
            // the outer bound and the remaining holes are read as the outline.
            // Both mesh perfectly happily.
            let outcomes = face
                .bounds_holder(self)
                .into_iter()
                .map(|bound| {
                    let bound = bound.ok_or(FaceLossReason::BoundReferenceUnresolved)?;
                    self.face_bound_to_edges(bound, edges)
                })
                .collect::<Result<Vec<BoundOutcome>, FaceLossReason>>();
            let outcomes = match outcomes {
                Ok(outcomes) => outcomes,
                Err(reason) => {
                    losses.push(FaceLoss { provenance, reason });
                    continue;
                }
            };
            let collapsed_pts: Vec<Point3> = outcomes
                .iter()
                .filter_map(|o| match o {
                    BoundOutcome::Collapsed(pt) => Some(*pt),
                    _ => None,
                })
                .collect();
            let collapsed = collapsed_pts.len();
            // The outer-bound standing, computed against the *surviving* wires
            // because `boundaries` below is the filtered list and an index into
            // the unfiltered one would name a different bound. A collapsed
            // outer bound is not silently reassigned to a neighbour: it simply
            // leaves no index, and the face reports `NoneDeclared`.
            let outer_flags = face.bound_outer_flags(self);
            let outer_bound = outer_bound_standing(&outer_flags, &outcomes);
            let provenance = FaceProvenance {
                outer_bound,
                ..provenance
            };
            let wires: Vec<TopologicallyClosedWire> = outcomes
                .into_iter()
                .filter_map(|o| match o {
                    BoundOutcome::Wire(wire) => Some(wire),
                    BoundOutcome::Collapsed(_) => None,
                })
                .collect();
            if wires.is_empty() && !Self::closed_surface_accepts_untrimmed(&surface) {
                // Every bound collapsed, so nothing describes where this face
                // ends. A cone that is only an apex is not a face, and trimming
                // by no boundary at all would emit the entire unbounded surface
                // — the blob failure mode this project exists to avoid.
                //
                // The one exception is a face whose every declared bound is a
                // genuine vertex loop (or which declares no bound at all) on a
                // *closed* bounded surface: the degenerate loop trims nothing,
                // so the whole surface is the face. That is the ball — a full
                // sphere trimmed by a `VERTEX_LOOP` at its pole. Any other
                // carrier (a plane, an open cylinder) would mesh an invented
                // region, so the refusal stands for them.
                losses.push(FaceLoss {
                    provenance,
                    reason: FaceLossReason::AllBoundsCollapsed,
                });
                continue;
            }
            if collapsed > 0 {
                // Recorded rather than silently absorbed: the face is now
                // rendered, but its domain has a singular point that nothing
                // downstream is told about. QUO-005 wants a type here.
                singular.push(provenance);
            }
            Self::tune_conical_surface(&mut surface, &wires, edges, &collapsed_pts);
            faces.push(CompressedFace {
                surface,
                // The proof is discharged here and nowhere earlier: truck's
                // `CompressedFace` takes bare index vectors, so this is the
                // boundary at which the guarantee stops travelling. Closing it
                // is §33a item 11 -- `boundaries` becomes
                // `Vec<TopologicallyClosedWire>`, which the owned-fork decision
                // (§31a) now permits.
                boundaries: wires
                    .into_iter()
                    .map(TopologicallyClosedWire::into_edges)
                    .collect(),
                orientation,
                // The whole reference chain, not one collapsed id. Every later
                // complaint about this face can name entities a reader can grep
                // out of the source file, and can say which layer it means.
                provenance,
            });
        }
        faces
    }

    /// Constructs `CompressedShell` of `truck` from `Shell` in STEP file
    /// # Example
    /// ```
    /// use truck_stepio::r#in::{*, step_geometry::*};
    /// // read file
    /// let step_string = include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-cube.step",
    /// ));
    /// // parse into Rust structs
    /// let table = Table::from_step(&step_string).unwrap();
    /// // take one shell (this is only one shell)
    /// let step_shell = table.shell.values().next().unwrap();
    /// // convert STEP shell to `CompressedShell`
    /// let cshell = table.to_compressed_shell(0, step_shell).unwrap();
    /// // The cube has 6 faces!
    /// assert_eq!(cshell.faces.len(), 6);
    /// ```
    pub fn to_compressed_shell(
        &self,
        shell_id: u64,
        shell: &impl StepShell,
    ) -> Result<CompressedShell<Point3, Curve3D, Surface>, StepConvertingError> {
        let mut cshell = shell.to_compressed_shell(self)?;
        cshell.source_geometric_uncertainty = self.source_geometric_uncertainty(shell_id);
        Ok(cshell)
    }

    /// As [`Self::to_compressed_shell`], and also why each lost face was lost.
    pub fn to_compressed_shell_with_losses(
        &self,
        shell_id: u64,
        shell: &impl StepShell,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, Vec<FaceLoss>), StepConvertingError>
    {
        let (mut cshell, losses) = shell.to_compressed_shell_with_losses(self)?;
        cshell.source_geometric_uncertainty = self.source_geometric_uncertainty(shell_id);
        Ok((cshell, losses))
    }

    /// Constructs `CompressedShell`s of `truck` from `ShellBasedSurfaceModel` in STEP file
    pub fn to_compressed_shells(
        &self,
        shells: &ShellBasedSurfaceModelHolder,
    ) -> Result<Vec<CompressedShell<Point3, Curve3D, Surface>>, StepConvertingError> {
        let mut res = Vec::new();
        for place_holder in &shells.sbsm_boundary {
            let PlaceHolder::Ref(Name::Entity(idx)) = place_holder else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            };
            if let Some(shell) = self.shell.get(idx) {
                res.push(self.to_compressed_shell(*idx, shell)?);
            } else if let Some(oriented_shell) = self.oriented_shell.get(idx) {
                res.push(self.to_compressed_shell(*idx, oriented_shell)?);
            } else {
                return Err("failed to reference an element of `sbsm_boundary`".into());
            }
        }
        Ok(res)
    }

    /// Constructs `CompressedSolid` of `truck` from `ManifoldSolidBrep` in STEP file
    /// # Example
    /// ```
    /// use truck_stepio::r#in::{*, step_geometry::*};
    /// truck_topology::prelude!(Point3, Curve3D, Surface);
    /// // read file
    /// let step_string = include_str!(concat!(
    ///     env!("CARGO_MANIFEST_DIR"),
    ///     "/../resources/step/occt-cube.step",
    /// ));
    /// // parse into Rust structs
    /// let table = Table::from_step(&step_string).unwrap();
    /// // take the solid
    /// let step_solid = table.manifold_solid_brep.values().next().unwrap();
    /// // convert STEP shell to `CompressedSolid`
    /// let csolid = table.to_compressed_solid(step_solid).unwrap();
    /// // Convert to truck `Solid`
    /// let solid = Solid::extract(csolid).unwrap();
    /// // The cube has 6 faces!
    /// assert_eq!(solid.boundaries()[0].len(), 6);
    /// ```
    pub fn to_compressed_solid(
        &self,
        solid: &ManifoldSolidBrepHolder,
    ) -> Result<CompressedSolid<Point3, Curve3D, Surface>, StepConvertingError> {
        let PlaceHolder::Ref(Name::Entity(outer_idx)) = &solid.outer else {
            return Err("failed to reference `solid.outer`".into());
        };
        let outer_shell = if let Some(step_shell) = self.shell.get(outer_idx) {
            self.to_compressed_shell(*outer_idx, step_shell)
        } else if let Some(step_shell) = self.oriented_shell.get(outer_idx) {
            self.to_compressed_shell(*outer_idx, step_shell)
        } else {
            Err("failed to reference `solid.outer`".into())
        }?;
        let mut boundaries = vec![outer_shell];
        for shell in &solid.voids {
            let PlaceHolder::Ref(Name::Entity(outer_idx)) = shell else {
                return Err("failed to reference an element of `solid.voids`".into());
            };
            let Some(oriented_shell) = self.oriented_shell.get(outer_idx) else {
                return Err("failed to reference an element of `solid.voids`".into());
            };
            boundaries.push(self.to_compressed_shell(*outer_idx, oriented_shell)?);
        }
        Ok(CompressedSolid { boundaries })
    }
}

#[derive(Clone, Debug, PartialEq, derive_more::From)]
pub enum NodeMatrix {
    Identity,
    Transform(Box<ItemDefinedTransformation>),
}

/// The definition geometry of a product node, kept definition-local.
///
/// The geometry variants also carry the source shell entity ids the geometry
/// was converted from, so a consumer can re-derive per-shell conversion losses
/// (the DIAG-002 stream) without re-resolving the assembly or re-walking the
/// source `SHAPE_REPRESENTATION_RELATIONSHIP`. The ids are authoritative: they
/// are read from the exact source entities the conversion walked.
#[derive(Clone, Debug, PartialEq)]
pub enum ProductShape {
    /// A `SHELL_BASED_SURFACE_MODEL`'s shells and their source boundary ids.
    Shells(Vec<CompressedShell<Point3, Curve3D, Surface>>, Vec<u64>),
    /// A `MANIFOLD_SOLID_BREP`'s boundaries and their source shell ids
    /// (the outer shell first, then each void shell).
    Solid(CompressedSolid<Point3, Curve3D, Surface>, Vec<u64>),
    Matrix(Matrix4),
}

pub type ProductEntity = NodeEntity<Vec<ProductShape>, PartAttrs>;
pub type AssembleEntity = EdgeEntity<NodeMatrix, PartAttrs>;
pub type StepAssembly = Assembly<Vec<ProductShape>, PartAttrs, NodeMatrix, PartAttrs>;

impl TryFrom<&NodeMatrix> for Matrix3 {
    type Error = StepConvertingError;
    fn try_from(value: &NodeMatrix) -> Result<Self, Self::Error> {
        match value {
            NodeMatrix::Identity => Ok(Self::identity()),
            NodeMatrix::Transform(trans) => (&**trans).try_into(),
        }
    }
}

impl TryFrom<&NodeMatrix> for Matrix4 {
    type Error = StepConvertingError;
    fn try_from(value: &NodeMatrix) -> Result<Self, Self::Error> {
        match value {
            NodeMatrix::Identity => Ok(Self::identity()),
            NodeMatrix::Transform(trans) => (&**trans).try_into(),
        }
    }
}

impl ProductShape {
    pub fn try_from_index(idx: u64, table: &Table) -> Result<Self, StepConvertingError> {
        if let Some(step_solid) = table.manifold_solid_brep.get(&idx) {
            let mut source_shell_ids = Vec::new();
            if let PlaceHolder::Ref(Name::Entity(outer_idx)) = &step_solid.outer {
                source_shell_ids.push(*outer_idx);
            }
            for shell in &step_solid.voids {
                if let PlaceHolder::Ref(Name::Entity(void_idx)) = shell {
                    source_shell_ids.push(*void_idx);
                }
            }
            Ok(ProductShape::Solid(
                table.to_compressed_solid(step_solid)?,
                source_shell_ids,
            ))
        } else if let Some(step_shells) = table.shell_based_surface_model.get(&idx) {
            let source_shell_ids = step_shells
                .sbsm_boundary
                .iter()
                .filter_map(|place_holder| match place_holder {
                    PlaceHolder::Ref(Name::Entity(idx)) => Some(*idx),
                    _ => None,
                })
                .collect::<Vec<_>>();
            Ok(ProductShape::Shells(
                table.to_compressed_shells(step_shells)?,
                source_shell_ids,
            ))
        } else if table.axis2_placement_3d.contains_key(&idx) {
            let axis = EntityTable::<Axis2Placement3dHolder>::get_owned(table, idx)?;
            Ok(ProductShape::Matrix(Matrix4::from(&axis)))
        } else {
            Err("Unknown Shape".into())
        }
    }
}

impl Table {
    fn product_node_entity(
        &self,
        pds_idx: u64,
        pd: &ProductDefinitionHolder,
    ) -> Result<ProductEntity, StepConvertingError> {
        let PlaceHolder::Ref(Name::Entity(pdf_idx)) = &pd.formation else {
            return Err("failed to reference `product_definition.formation`".into());
        };
        let Some(pdf) = self.product_definition_formation.get(pdf_idx) else {
            return Err("failed to reference `prouct_definition_formation`".into());
        };
        let PlaceHolder::Ref(Name::Entity(p_idx)) = &pdf.of_product else {
            return Err("failed to reference `product_definition_formation.of_product`".into());
        };
        let Some(product) = self.product.get(p_idx) else {
            return Err("failed to reference `product`".into());
        };
        let attrs = PartAttrs {
            id: product.id.clone(),
            name: product.name.clone(),
            description: product.description.clone(),
        };

        let Some(sdr) = self.shape_definition_representation.values().find(|sdr| {
            let &PlaceHolder::Ref(Name::Entity(idx)) = &sdr.definition else {
                return false;
            };
            pds_idx == idx
        }) else {
            return Err("failed to find `shape_definition_representation` corresp. to `product_definition_shape`".into());
        };
        let PlaceHolder::Ref(Name::Entity(sr_idx)) = &sdr.used_representation else {
            return Err(
                "failed to reference `shape_definition_representation.used_representation`".into(),
            );
        };
        let Some(sr) = self.shape_representation.get(sr_idx) else {
            return Err("failed to reference `shape_representation`".into());
        };
        let mut shape = sr
            .items
            .iter()
            .map(|place_holder| {
                if let &PlaceHolder::Ref(Name::Entity(item_idx)) = place_holder {
                    ProductShape::try_from_index(item_idx, self).ok()
                } else {
                    None
                }
            })
            .collect::<Option<Vec<_>>>()
            .ok_or::<StepConvertingError>(
                "failed to reference an element of `shape_representation.items`".into(),
            )?;

        // The SDR's used representation can be a placement/frame-only
        // representation while the definition geometry lives in a separate
        // `ADVANCED_BREP_SHAPE_REPRESENTATION`. The two are connected by an
        // explicit `SHAPE_REPRESENTATION_RELATIONSHIP` whose `rep_1` is the
        // used representation and whose `rep_2` is the geometry representation.
        // Follow that source relationship so the definition BREP becomes
        // reachable alongside the retained frame. The placement information is
        // preserved: both shapes stay in the node's shape vector.
        let mut linked = self
            .shape_representation_relationship
            .values()
            .filter_map(|relationship| {
                let PlaceHolder::Ref(Name::Entity(rep_1)) = &relationship.rep_1 else {
                    return None;
                };
                (*rep_1 == *sr_idx).then_some(relationship)
            })
            .collect::<Vec<_>>();
        linked.sort_by_key(|relationship| match &relationship.rep_2 {
            PlaceHolder::Ref(Name::Entity(rep_2)) => *rep_2,
            _ => u64::MAX,
        });
        for relationship in linked {
            let PlaceHolder::Ref(Name::Entity(rep_2)) = &relationship.rep_2 else {
                return Err(
                    "failed to reference the geometry `shape_representation` of a `shape_representation_relationship`"
                        .into(),
                );
            };
            let Some(geometry_rep) = self.shape_representation.get(rep_2) else {
                return Err("failed to reference the geometry `shape_representation`".into());
            };
            let geometry = geometry_rep
                .items
                .iter()
                .map(|place_holder| {
                    if let &PlaceHolder::Ref(Name::Entity(item_idx)) = place_holder {
                        ProductShape::try_from_index(item_idx, self).ok()
                    } else {
                        None
                    }
                })
                .collect::<Option<Vec<_>>>()
                .ok_or::<StepConvertingError>(
                    "failed to reference an element of the geometry `shape_representation.items`"
                        .into(),
                )?;
            shape.extend(geometry);
        }

        Ok(NodeEntity { shape, attrs })
    }

    fn assy_node_entity(
        &self,
        pds_idx: u64,
        next_assy: &NextAssemblyUsageOccurrenceHolder,
    ) -> Result<(AssembleEntity, (u64, u64)), StepConvertingError> {
        let &PlaceHolder::Ref(Name::Entity(parent_idx)) = &next_assy.relating_product_definition
        else {
            return Err("failed to reference the parent node".into());
        };
        let &PlaceHolder::Ref(Name::Entity(child_idx)) = &next_assy.related_product_definition
        else {
            return Err("failed to reference the child node".into());
        };

        let attrs = PartAttrs {
            id: next_assy.id.clone(),
            name: next_assy.name.clone(),
            description: next_assy.description.clone(),
        };

        let Some(cdsr) = self
            .context_dependent_shape_representation
            .values()
            .find(|cdsr| {
                let &PlaceHolder::Ref(Name::Entity(idx)) = &cdsr.represented_product_relation
                else {
                    return false;
                };
                pds_idx == idx
            })
        else {
            return Err("".into());
        };

        let PlaceHolder::Ref(Name::Entity(srrwt_idx)) = &cdsr.representation_relation else {
            return Err("failed to reference `context_dependent_shape_representation.representation_relation`".into());
        };

        let Some(srrwt) = self
            .shape_representation_relationship_with_transformation
            .get(srrwt_idx)
        else {
            return Err("failed to reference `shape_representation_relationship`".into());
        };
        let idtf = srrwt.transformation_operator.clone().into_owned(self)?;

        let entity = AssembleEntity {
            matrix: NodeMatrix::Transform(idtf.into()),
            attrs,
        };

        Ok((entity, (parent_idx, child_idx)))
    }

    pub fn step_assy(&self) -> Result<StepAssembly, StepConvertingError> {
        let mut product_entities = Vec::<ProductEntity>::new();
        let mut indices_map = HashMap::<u64, usize>::new();
        let mut assy_nodes = Vec::<(AssembleEntity, (u64, u64))>::new();
        // Iterate in entity-id order so the node numbering of the graph is a
        // function of the document, not of HashMap iteration order. A
        // downstream consumer assigns geometry and occurrence indices from
        // these node numbers; without the sort the same file could enumerate
        // its assembly differently on every process.
        let mut pds_entries = self.product_definition_shape.iter().collect::<Vec<_>>();
        pds_entries.sort_by_key(|(&id, _)| id);
        for (&pds_idx, pds) in pds_entries {
            let &PlaceHolder::Ref(Name::Entity(idx)) = &pds.definition else {
                return Err("failed to reference `product_definition_shape.definition`".into());
            };
            if let Some(pd) = self.product_definition.get(&idx) {
                product_entities.push(self.product_node_entity(pds_idx, pd)?);
                indices_map.insert(idx, product_entities.len() - 1);
            } else if let Some(next_assy) = self.next_assembly_usage_occurrence.get(&idx) {
                assy_nodes.push(self.assy_node_entity(pds_idx, next_assy)?);
            }
        }

        let adjacency = assy_nodes
            .into_iter()
            .map(|(entity, (from, to))| {
                let from = *indices_map.get(&from)?;
                let to = *indices_map.get(&to)?;
                Some((from, to, entity))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or::<StepConvertingError>("failed to reference `product_definiion_shape`".into())?;

        StepAssembly::try_from_adjacency(product_entities, adjacency)
            .ok_or("maybe the graph has a cycle.".into())
    }
}

pub trait StepShell {
    fn to_compressed_shell(
        &self,
        table: &Table,
    ) -> Result<CompressedShell<Point3, Curve3D, Surface>, StepConvertingError> {
        self.to_compressed_shell_with_losses(table)
            .map(|(shell, _)| shell)
    }

    /// The shell, plus one record per source face that did not survive.
    ///
    /// The plain conversion delegates to this rather than the reverse, so there
    /// is exactly one conversion path and the census cannot drift away from
    /// what the renderer actually does.
    fn to_compressed_shell_with_losses(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, Vec<FaceLoss>), StepConvertingError>;
}

impl StepShell for ShellHolder {
    fn to_compressed_shell_with_losses(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, Vec<FaceLoss>), StepConvertingError>
    {
        let vertices = table.shell_vertices(self);
        let edges = table.shell_edges(self, &vertices);
        // Faces are resolved while the arenas are still arenas, so a face can
        // only name an edge that converted. Only then are they flattened to the
        // bare vectors `CompressedShell` requires.
        let mut losses = Vec::new();
        let mut singular = Vec::new();
        let faces = table.shell_faces(self, &edges, &mut losses, &mut singular);
        if !singular.is_empty() && std::env::var_os("TRUCK_PROBE_SINGULAR").is_some() {
            eprintln!(
                "SINGULAR {} faces have a collapsed bound (apex or pole)",
                singular.len()
            );
        }
        Ok((
            CompressedShell {
                vertices: vertices.into_items(),
                edges: edges.into_items(),
                faces,
                // Set by `Table::to_compressed_shell(_with_losses)` from the
                // shell's shape representation; the trait conversion itself has
                // no shell id to resolve it against.
                source_geometric_uncertainty: None,
            },
            losses,
        ))
    }
}

impl StepShell for OrientedShellHolder {
    fn to_compressed_shell_with_losses(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, Vec<FaceLoss>), StepConvertingError>
    {
        let PlaceHolder::Ref(Name::Entity(idx)) = &self.shell_element else {
            return Err("failed to reference shell".into());
        };
        let Some(shell) = table.shell.get(idx) else {
            return Err("failed to reference shell".into());
        };
        let (mut res, losses) = shell.to_compressed_shell_with_losses(table)?;
        if !self.orientation {
            for face in &mut res.faces {
                face.orientation = !face.orientation;
            }
        }
        Ok((res, losses))
    }
}

impl StepShell for ShellAnyHolder {
    fn to_compressed_shell_with_losses(
        &self,
        table: &Table,
    ) -> Result<(CompressedShell<Point3, Curve3D, Surface>, Vec<FaceLoss>), StepConvertingError>
    {
        match self {
            ShellAnyHolder::OrientedShell(shell) => shell.to_compressed_shell_with_losses(table),
            ShellAnyHolder::Shell(shell) => shell.to_compressed_shell_with_losses(table),
        }
    }
}

/// Which layer of the face reference chain a shell named directly.
///
/// STEP shells may reference either layer, and the two are not
/// interchangeable: an `ORIENTED_FACE` is a use that contributes orientation
/// and may share its `FACE_SURFACE` with other uses. Recording which one was
/// found keeps `FaceProvenance` from claiming a use entity that the file never
/// wrote.
enum FaceReference {
    /// The shell named a `FACE_SURFACE`. There is no use entity.
    Definition,
    /// The shell named an `ORIENTED_FACE`, which resolved to this definition.
    Use { definition_id: Option<u64> },
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    /// A `FACE_SURFACE` with no bounds. Only its identity matters here.
    fn face_surface() -> FaceSurfaceHolder {
        FaceSurfaceHolder {
            label: String::new(),
            bounds: Vec::new(),
            face_geometry: PlaceHolder::Ref(Name::Entity(700)),
            same_sense: true,
        }
    }

    /// A shell naming an `ORIENTED_FACE` must yield a use id *and* a separately
    /// resolved definition id.
    ///
    /// This is the branch no file in either corpus exercises — all 33 NIST
    /// models and the ABC models checked name `ADVANCED_FACE` directly — so
    /// without this test the half of the split that motivated it would be
    /// unrun. Compiling is not evidence.
    #[test]
    fn an_oriented_face_reports_use_and_definition_separately() {
        let mut table = Table::default();
        table.face_surface.insert(402, face_surface());
        table.oriented_face.insert(
            811,
            OrientedFaceHolder {
                label: String::new(),
                face_element: PlaceHolder::Ref(Name::Entity(402)),
                orientation: false,
            },
        );

        let holder = table.oriented_face.get(&811).cloned().unwrap();
        let (orientation, reference, _) = table
            .face_any_resolved(Some(FaceAnyHolder::OrientedFace(holder)))
            .expect("the oriented face resolves");

        assert!(!orientation, "the use carries the orientation flag");
        match reference {
            FaceReference::Use { definition_id } => assert_eq!(
                definition_id,
                Some(402),
                "the definition is the FACE_SURFACE, not the use"
            ),
            FaceReference::Definition => panic!("an ORIENTED_FACE is a use, not a definition"),
        }
    }

    /// A shell naming a `FACE_SURFACE` directly has no use entity at all, and
    /// must not invent one by copying the reference into the use slot — which
    /// is exactly what an earlier version did, printing "face use #43172 of
    /// face #43172".
    #[test]
    fn a_direct_face_surface_reference_has_no_use() {
        let table = Table::default();
        let (orientation, reference, _) = table
            .face_any_resolved(Some(FaceAnyHolder::FaceSurface(face_surface())))
            .expect("the face surface resolves");

        assert!(
            orientation,
            "a definition carries no orientation of its own"
        );
        assert!(
            matches!(reference, FaceReference::Definition),
            "a direct reference names the definition"
        );
    }

    /// The surface reference is a third, independent identity: one surface is
    /// commonly shared by many faces, so "which surface did this face name" is
    /// not answered by either of the other two ids.
    #[test]
    fn the_surface_identity_is_recorded_separately() {
        let table = Table::default();
        let provenance = FaceProvenance {
            use_id: Some(SourceEntityId::new(811)),
            definition_id: Some(SourceEntityId::new(402)),
            surface_id: Some(SourceEntityId::new(91)),
            // This test predates the field and stopped compiling when it
            // landed. What it is about is the three *identities*; the
            // outer-bound standing is not one of them.
            outer_bound: OuterBoundStanding::NotRetained,
        };
        assert_eq!(
            provenance.to_string(),
            "face use #811 of face #402, surface #91"
        );
        assert_eq!(provenance.best_id(), Some(SourceEntityId::new(402)));
        let _ = table;
    }
}

#[cfg(test)]
mod plane_angle_unit_tests {
    use super::*;

    /// A file writing angles in degrees must report the degree factor.
    #[test]
    fn a_degree_file_reports_the_degree_factor() {
        let mut table = Table::default();
        // #20 = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.0174532925), #18)
        table
            .plane_angle_measures
            .insert(20, (0.0174532925, Some(18)));
        // #18 is the radian SI unit the conversion is expressed in.
        table.plane_angle_units.push((18, PlaneAngleUnit::Radian));
        // #24 = CONVERSION_BASED_UNIT('DEGREE', #20)
        table
            .plane_angle_units
            .push((24, PlaneAngleUnit::Converted { measure: 20 }));

        assert_eq!(table.plane_angle_factor(), 0.0174532925);
    }

    /// The regression for the defect this resolution *introduced*.
    ///
    /// A degree unit is defined as a multiple of a radian unit, so every degree
    /// file necessarily also contains a radian `SI_UNIT`. Counting that base as
    /// a competing declaration made the agreement rule refuse every file it
    /// existed to fix — observed on the first run against `ftc_07`, which
    /// printed "plane angle units disagree (1 vs 0.0174532925)" and left the
    /// blob exactly as it was. The base must be excluded, not compared.
    #[test]
    fn the_base_unit_of_a_conversion_does_not_count_as_disagreement() {
        let mut table = Table::default();
        table
            .plane_angle_measures
            .insert(20, (0.0174532925, Some(18)));
        table.plane_angle_units.push((18, PlaneAngleUnit::Radian));
        table
            .plane_angle_units
            .push((24, PlaneAngleUnit::Converted { measure: 20 }));

        assert_ne!(
            table.plane_angle_factor(),
            1.0,
            "the radian base of the degree unit must not veto the conversion"
        );
    }

    /// A radian file is left alone, and costs nothing.
    #[test]
    fn a_radian_file_needs_no_conversion() {
        let mut table = Table::default();
        table.plane_angle_units.push((18, PlaneAngleUnit::Radian));
        assert_eq!(table.plane_angle_factor(), 1.0);
    }

    /// A file with no unit declarations at all is assumed to be in radians,
    /// which is what the standard says and what every file did before this
    /// existed.
    #[test]
    fn no_declaration_means_radians() {
        assert_eq!(Table::default().plane_angle_factor(), 1.0);
    }

    /// Two *independently assigned* angle units genuinely conflict, and the
    /// resolution refuses rather than picking one.
    ///
    /// Choosing correctly needs the geometry's own
    /// `GEOMETRIC_REPRESENTATION_CONTEXT`, which is not resolved here. Guessing
    /// would convert a file whose geometry is already in radians and break it —
    /// worse than leaving it as found, because the failure would be new.
    #[test]
    fn independently_assigned_conflicting_units_are_refused() {
        let mut table = Table::default();
        // Two conversions with different factors, neither the base of the other.
        table
            .plane_angle_measures
            .insert(20, (0.0174532925, Some(18)));
        table.plane_angle_measures.insert(30, (0.5, Some(28)));
        table
            .plane_angle_units
            .push((24, PlaneAngleUnit::Converted { measure: 20 }));
        table
            .plane_angle_units
            .push((34, PlaneAngleUnit::Converted { measure: 30 }));

        assert_eq!(
            table.plane_angle_factor(),
            1.0,
            "a real conflict leaves the file as found"
        );
    }

    /// A conical surface's semi-angle is converted into radians on import.
    ///
    /// This is the attribute that produced the `ftc_07` blob: a 2° draft cone
    /// read as 2 radians has slope `tan(2) = -2.185` instead of `0.0349` —
    /// wrong by 63x and inverted in sign, which flares the cone backwards into
    /// a fan.
    #[test]
    fn a_cone_semi_angle_is_normalized_into_radians() {
        let mut table = Table::default();
        table
            .plane_angle_measures
            .insert(20, (0.0174532925, Some(18)));
        table.plane_angle_units.push((18, PlaneAngleUnit::Radian));
        table
            .plane_angle_units
            .push((24, PlaneAngleUnit::Converted { measure: 20 }));
        table.conical_surface.insert(
            686,
            ConicalSurfaceHolder {
                label: String::new(),
                position: PlaceHolder::Ref(Name::Entity(685)),
                radius: 0.282184119986423,
                semi_angle: 2.0,
            },
        );

        table.normalize_angle_units();

        let converted = table.conical_surface[&686].semi_angle;
        assert!(
            (converted - 2.0 * 0.0174532925).abs() < 1.0e-15,
            "2 degrees must become {} radians, got {converted}",
            2.0 * 0.0174532925
        );
        assert!(
            f64::tan(converted) > 0.0,
            "the corrected slope must not be negative; that inversion is the blob"
        );
    }
}

/// Why a source face produced no `CompressedFace`.
///
/// Coarse on purpose. The point of a census is to order a repair queue, and a
/// dozen categories that each name a real code path beat forty that require
/// judgement to assign. Split a variant when its count is large enough that the
/// split would change what gets fixed next, not before.
///
/// These cover conversion only. A face that converts and then meshes to nothing
/// is lost later, in tessellation, and is counted there — the two populations
/// have different causes and must not be summed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaceLossReason {
    /// The shell named something that does not resolve to a face at all.
    FaceReferenceUnresolved,
    /// The face's surface reference did not resolve, or its geometry could not
    /// be converted. `OFFSET_SURFACE` lands here: it parses and then has no
    /// conversion arm.
    SurfaceConversionFailed,
    /// A `FACE_BOUND`/`FACE_OUTER_BOUND` reference did not resolve.
    BoundReferenceUnresolved,
    /// Every bound of the face collapsed to a point, so nothing bounds it.
    AllBoundsCollapsed,
    /// The bound resolved, but the `EDGE_LOOP` it names did not.
    ///
    /// Split from `BoundReferenceUnresolved` once the census showed the pair
    /// accounted for 45% of all lost faces on `00009190`: they are different
    /// defects — a missing bound entity against a missing loop entity — and
    /// deciding what to fix next requires knowing which.
    LoopReferenceUnresolved,
    /// An `ORIENTED_EDGE` in a bound did not resolve to an edge.
    EdgeUseUnresolved,
    /// The edge resolved as a reference but its curve did not convert, so no
    /// arena position exists for it.
    EdgeCurveConversionFailed,
    /// Every edge resolved and the wire still does not close on vertex
    /// identity, so it bounds nothing (`TOP-004`).
    WireNotClosed,
}

impl FaceLossReason {
    /// A short stable tag, for grouping a census.
    pub fn tag(self) -> &'static str {
        match self {
            Self::FaceReferenceUnresolved => "FaceReferenceUnresolved",
            Self::SurfaceConversionFailed => "SurfaceConversionFailed",
            Self::BoundReferenceUnresolved => "BoundReferenceUnresolved",
            Self::LoopReferenceUnresolved => "LoopReferenceUnresolved",
            Self::AllBoundsCollapsed => "AllBoundsCollapsed",
            Self::EdgeUseUnresolved => "EdgeUseUnresolved",
            Self::EdgeCurveConversionFailed => "EdgeCurveConversionFailed",
            Self::WireNotClosed => "WireNotClosed",
        }
    }
}

/// One source face that did not survive conversion, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceLoss {
    /// As much of the reference chain as was resolved before the failure.
    ///
    /// Partial by nature: a face whose surface reference failed has no surface
    /// id to report, and reporting a fabricated one would defeat the purpose.
    pub provenance: FaceProvenance,
    pub reason: FaceLossReason,
}

/// What one face bound contributed to the trimming domain.
/// The face's outer-bound standing, stated against the surviving wires.
///
/// `flags` is per declared bound, in source order: `Some(true)` for a
/// `FACE_OUTER_BOUND`, `Some(false)` for a plain `FACE_BOUND`, `None` for a
/// bound the document inlined, whose entity type this reader cannot recover.
///
/// Any `None` makes the whole face `NotRetained`. That is deliberate: a face
/// with one inlined bound and one referenced `FACE_BOUND` would otherwise
/// report `NoneDeclared`, which is a *claim* about the source, and the reader
/// is not in a position to make it.
fn outer_bound_standing(flags: &[Option<bool>], outcomes: &[BoundOutcome]) -> OuterBoundStanding {
    if flags.len() != outcomes.len() || flags.iter().any(Option::is_none) {
        return OuterBoundStanding::NotRetained;
    }
    let declared_count = flags.iter().filter(|flag| **flag == Some(true)).count();
    // Index among the wires that survive, since those are what `boundaries`
    // will hold. A collapsed outer bound contributes no wire and so no index.
    let mut surviving = 0u32;
    let mut bound_index = None;
    for (flag, outcome) in flags.iter().zip(outcomes) {
        match outcome {
            BoundOutcome::Wire(_) => {
                if *flag == Some(true) && bound_index.is_none() {
                    bound_index = Some(surviving);
                }
                surviving += 1;
            }
            BoundOutcome::Collapsed(_) => {}
        }
    }
    match (declared_count, bound_index) {
        (0, _) => OuterBoundStanding::NoneDeclared,
        (_, None) => OuterBoundStanding::NoneDeclared,
        (count, Some(bound_index)) => OuterBoundStanding::Declared {
            bound_index,
            declared_count: count as u32,
        },
    }
}

enum BoundOutcome {
    /// An ordinary closed wire of edges.
    Wire(TopologicallyClosedWire),
    /// Nothing: the bound is a single vertex, and the surface closes itself
    /// there. See `FaceBoundLoop::Collapsed`. Retains the 3D vertex position.
    Collapsed(Point3),
}

#[cfg(test)]
mod vertex_loop_tests {
    use super::*;

    fn vertex_point() -> VertexPointHolder {
        VertexPointHolder {
            label: String::new(),
            vertex_geometry: PlaceHolder::Ref(Name::Entity(1820)),
        }
    }

    /// A bound naming a `VERTEX_LOOP` resolves as collapsed, not as unresolved.
    ///
    /// This is the regression for the largest single cause of missing faces
    /// found in either corpus: 272 of 604 on ABC `00009190` and 132 across
    /// NIST, with the `VERTEX_LOOP` count matching the failure count exactly in
    /// all eight files containing one. `bound_holder` checked `edge_loop` alone,
    /// so a cone apex took its whole face with it.
    #[test]
    fn a_vertex_loop_bound_resolves_as_collapsed() {
        let mut table = Table::default();
        table.vertex_loop.insert(
            286,
            VertexLoopHolder {
                label: String::new(),
                loop_vertex: PlaceHolder::Owned(vertex_point()),
            },
        );
        let bound = FaceBoundHolder {
            label: String::new(),
            bound: PlaceHolder::Ref(Name::Entity(286)),
            orientation: true,
        };

        assert!(matches!(
            bound.bound_holder(&table),
            Some(FaceBoundLoop::Collapsed(_))
        ));
    }

    /// An `EDGE_LOOP` still resolves as edges, and the two are distinguished
    /// rather than both being "a loop".
    #[test]
    fn an_edge_loop_bound_still_resolves_as_edges() {
        let mut table = Table::default();
        table.edge_loop.insert(
            4930,
            EdgeLoopHolder {
                label: String::new(),
                edge_list: Vec::new(),
            },
        );
        let bound = FaceBoundHolder {
            label: String::new(),
            bound: PlaceHolder::Ref(Name::Entity(4930)),
            orientation: true,
        };

        assert!(matches!(
            bound.bound_holder(&table),
            Some(FaceBoundLoop::Edges(_))
        ));
    }

    /// A bound naming neither is still unresolved, so a genuinely missing loop
    /// is not quietly reclassified as an apex.
    #[test]
    fn an_unknown_loop_reference_is_still_unresolved() {
        let bound = FaceBoundHolder {
            label: String::new(),
            bound: PlaceHolder::Ref(Name::Entity(99999)),
            orientation: true,
        };
        assert!(bound.bound_holder(&Table::default()).is_none());
    }
}

/// Assembly definition-geometry attachment through the explicit
/// `SHAPE_REPRESENTATION_RELATIONSHIP`.
///
/// The fixture models the `core_xy.step` encoding: every part definition's
/// `SHAPE_DEFINITION_REPRESENTATION` points at a placement-only
/// `SHAPE_REPRESENTATION`, and the actual BREP lives in a separate
/// `ADVANCED_BREP_SHAPE_REPRESENTATION` reached through an explicit
/// `SHAPE_REPRESENTATION_RELATIONSHIP` whose `rep_1` is the placement
/// representation.
#[cfg(test)]
mod step_assy_geometry_tests {
    use super::*;

    const FIXTURE: &str = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Fixture'),'2;1');
FILE_NAME('assembly_geometry_fixture','2026-01-01T00:00:00',(''),(''),
  '','','');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));
ENDSEC;
DATA;
#1 = APPLICATION_CONTEXT('assembly context');
#3 = PRODUCT_CONTEXT('',#1,'mechanical');
#4 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');
#5 = REPRESENTATION_CONTEXT('Context #1','3D Context');
#10 = PRODUCT('assembly','assembly','',(#3));
#11 = PRODUCT_DEFINITION_FORMATION('','',#10);
#12 = PRODUCT_DEFINITION('','',#11,#4);
#13 = PRODUCT_DEFINITION_SHAPE('','',#12);
#20 = PRODUCT('partA','partA','',(#3));
#21 = PRODUCT_DEFINITION_FORMATION('','',#20);
#22 = PRODUCT_DEFINITION('','',#21,#4);
#23 = PRODUCT_DEFINITION_SHAPE('','',#22);
#30 = PRODUCT('partB','partB','',(#3));
#31 = PRODUCT_DEFINITION_FORMATION('','',#30);
#32 = PRODUCT_DEFINITION('','',#31,#4);
#33 = PRODUCT_DEFINITION_SHAPE('','',#32);
#40 = PRODUCT('partC','partC','',(#3));
#41 = PRODUCT_DEFINITION_FORMATION('','',#40);
#42 = PRODUCT_DEFINITION('','',#41,#4);
#43 = PRODUCT_DEFINITION_SHAPE('','',#42);
#50 = PRODUCT('partD','partD','',(#3));
#51 = PRODUCT_DEFINITION_FORMATION('','',#50);
#52 = PRODUCT_DEFINITION('','',#51,#4);
#53 = PRODUCT_DEFINITION_SHAPE('','',#52);
#100 = DIRECTION('',(0.,0.,1.));
#101 = DIRECTION('',(1.,0.,0.));
#102 = CARTESIAN_POINT('',(0.,0.,0.));
#103 = AXIS2_PLACEMENT_3D('',#102,#100,#101);
#104 = PLANE('',#103);
#105 = CARTESIAN_POINT('',(0.,0.,0.));
#106 = CARTESIAN_POINT('',(1.,0.,0.));
#107 = CARTESIAN_POINT('',(0.,1.,0.));
#108 = VERTEX_POINT('',#105);
#109 = VERTEX_POINT('',#106);
#110 = VERTEX_POINT('',#107);
#111 = DIRECTION('',(1.,0.,0.));
#112 = VECTOR('',#111,1.);
#113 = LINE('',#105,#112);
#114 = DIRECTION('',(-1.,1.,0.));
#115 = VECTOR('',#114,1.);
#116 = LINE('',#106,#115);
#117 = DIRECTION('',(0.,-1.,0.));
#118 = VECTOR('',#117,1.);
#119 = LINE('',#107,#118);
#120 = EDGE_CURVE('',#108,#109,#113,.T.);
#121 = EDGE_CURVE('',#109,#110,#116,.T.);
#122 = EDGE_CURVE('',#110,#108,#119,.T.);
#123 = ORIENTED_EDGE('',*,*,#120,.T.);
#124 = ORIENTED_EDGE('',*,*,#121,.T.);
#125 = ORIENTED_EDGE('',*,*,#122,.T.);
#126 = EDGE_LOOP('',(#123,#124,#125));
#127 = FACE_OUTER_BOUND('',#126,.T.);
#128 = ADVANCED_FACE('',(#127),#104,.T.);
#129 = CLOSED_SHELL('',(#128));
#130 = MANIFOLD_SOLID_BREP('',#129);
#131 = MANIFOLD_SOLID_BREP('',#129);
#132 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#130),#5);
#133 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#131),#5);
#134 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#141,#130),#5);
#140 = CARTESIAN_POINT('',(0.,0.,0.));
#141 = AXIS2_PLACEMENT_3D('',#140,#100,#101);
#143 = CARTESIAN_POINT('',(1.,0.,0.));
#144 = AXIS2_PLACEMENT_3D('',#143,#100,#101);
#145 = CARTESIAN_POINT('',(-1.,0.,0.));
#146 = AXIS2_PLACEMENT_3D('',#145,#100,#101);
#147 = CARTESIAN_POINT('',(2.,0.,0.));
#149 = AXIS2_PLACEMENT_3D('',#147,#100,#101);
#150 = CARTESIAN_POINT('',(0.,0.,1.));
#153 = AXIS2_PLACEMENT_3D('',#150,#100,#101);
#160 = SHAPE_REPRESENTATION('partA',(#141),#5);
#161 = SHAPE_REPRESENTATION('partB',(#141),#5);
#162 = SHAPE_REPRESENTATION('partC',(#141),#5);
#163 = SHAPE_REPRESENTATION('main',(#141,#144,#146,#149,#153),#5);
#170 = SHAPE_REPRESENTATION_RELATIONSHIP('','',#160,#132);
#171 = SHAPE_REPRESENTATION_RELATIONSHIP('','',#161,#133);
#180 = SHAPE_DEFINITION_REPRESENTATION(#23,#160);
#181 = SHAPE_DEFINITION_REPRESENTATION(#33,#161);
#182 = SHAPE_DEFINITION_REPRESENTATION(#43,#162);
#183 = SHAPE_DEFINITION_REPRESENTATION(#53,#134);
#184 = SHAPE_DEFINITION_REPRESENTATION(#13,#163);
#600 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occA','','',#12,#22,'');
#601 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occB1','','',#12,#32,'');
#602 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occB2','','',#12,#32,'');
#603 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occC','','',#12,#42,'');
#604 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occD','','',#12,#52,'');
#610 = PRODUCT_DEFINITION_SHAPE('','',#600);
#611 = PRODUCT_DEFINITION_SHAPE('','',#601);
#612 = PRODUCT_DEFINITION_SHAPE('','',#602);
#613 = PRODUCT_DEFINITION_SHAPE('','',#603);
#614 = PRODUCT_DEFINITION_SHAPE('','',#604);
#630 = ITEM_DEFINED_TRANSFORMATION('','',#141,#144);
#631 = ITEM_DEFINED_TRANSFORMATION('','',#141,#146);
#632 = ITEM_DEFINED_TRANSFORMATION('','',#141,#149);
#633 = ITEM_DEFINED_TRANSFORMATION('','',#141,#153);
#634 = ITEM_DEFINED_TRANSFORMATION('','',#141,#144);
#620 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#160,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#630)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#621 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#161,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#631)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#622 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#161,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#632)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#623 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#162,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#633)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#624 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#134,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#634)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#700 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#620,#610);
#701 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#621,#611);
#702 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#622,#612);
#703 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#623,#613);
#704 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#624,#614);
ENDSEC;
END-ISO-10303-21;
"#;

    fn mapped_assembly() -> (
        StepAssembly,
        Assembly<Vec<ProductShape>, PartAttrs, Matrix4, PartAttrs>,
    ) {
        let table = Table::from_step(FIXTURE).expect("fixture must parse");
        let assy = table.step_assy().expect("assembly must build");
        let mapped = assy.map(
            |node: &NodeEntity<Vec<ProductShape>, PartAttrs>| NodeEntity {
                shape: node.shape.clone(),
                attrs: node.attrs.clone(),
            },
            |edge: &EdgeEntity<NodeMatrix, PartAttrs>| EdgeEntity {
                matrix: Matrix4::try_from(&edge.matrix).expect("edge transform must convert"),
                attrs: edge.attrs.clone(),
            },
        );
        (assy, mapped)
    }

    fn node_shape<'a>(
        mapped: &'a Assembly<Vec<ProductShape>, PartAttrs, Matrix4, PartAttrs>,
        name: &str,
    ) -> &'a Vec<ProductShape> {
        mapped
            .all_nodes()
            .find(|node| node.entity().attrs.name == name)
            .expect("node must exist")
            .shape()
    }

    fn count_variants(shape: &[ProductShape]) -> (usize, usize, usize) {
        let mut matrix = 0;
        let mut solid = 0;
        let mut shells = 0;
        for shape in shape {
            match shape {
                ProductShape::Matrix(_) => matrix += 1,
                ProductShape::Solid(..) => solid += 1,
                ProductShape::Shells(..) => shells += 1,
            }
        }
        (matrix, solid, shells)
    }

    /// T2 — an SDR whose used representation is placement-only still exposes
    /// the linked definition BREP *and* keeps the placement frame.
    #[test]
    fn a_placement_representation_retains_placement_and_gains_geometry() {
        let (_assy, mapped) = mapped_assembly();
        let (matrix, solid, shells) = count_variants(node_shape(&mapped, "partA"));
        assert_eq!(
            (matrix, solid, shells),
            (1, 1, 0),
            "partA must carry both its placement frame and its linked solid"
        );
        assert_eq!(
            (count_variants(node_shape(&mapped, "partB"))),
            (1, 1, 0),
            "partB must carry both its placement frame and its linked solid"
        );
    }

    /// The linked solid exposes the source shell entity ids it was converted
    /// from, so a consumer can re-derive the DIAG-002 conversion-loss stream
    /// without re-resolving the assembly.
    #[test]
    fn a_linked_solid_exposes_its_source_shell_ids() {
        let (_assy, mapped) = mapped_assembly();
        let shape = node_shape(&mapped, "partA");
        let (_, source_ids) = shape
            .iter()
            .find_map(|shape| match shape {
                ProductShape::Solid(solid, source) => Some((solid, source)),
                _ => None,
            })
            .expect("partA carries a solid");
        assert_eq!(
            source_ids,
            &vec![129],
            "the solid's outer shell is the source shell entity id"
        );
    }

    /// T1 — an SDR already pointing directly at geometry keeps working.
    #[test]
    fn a_direct_geometry_representation_still_converts() {
        let (_assy, mapped) = mapped_assembly();
        let (matrix, solid, shells) = count_variants(node_shape(&mapped, "partD"));
        assert_eq!(
            (matrix, solid, shells),
            (1, 1, 0),
            "partD's used representation carries a frame and a solid directly"
        );
    }

    /// T3 — a product with no source geometry relationship gets no invented
    /// attachment.
    #[test]
    fn a_product_without_a_geometry_relationship_gets_no_geometry() {
        let (_assy, mapped) = mapped_assembly();
        let (matrix, solid, shells) = count_variants(node_shape(&mapped, "partC"));
        assert_eq!(
            (matrix, solid, shells),
            (1, 0, 0),
            "partC has only a frame; no geometry may be invented"
        );
    }

    /// T4 — two occurrences of one definition share one node (one geometry),
    /// stay distinct occurrences, and carry distinct world transforms.
    #[test]
    fn a_repeated_definition_produces_distinct_occurrences_sharing_one_node() {
        let (assy, mapped) = mapped_assembly();
        let top = mapped.top_nodes().next().expect("a root must exist");
        let paths = mapped.paths_iter(top.index()).collect::<Vec<_>>();
        let occurrences = paths
            .iter()
            .filter(|path| !path.edges().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(
            occurrences.len(),
            5,
            "five source occurrences, one per NAUO edge"
        );
        assert_eq!(
            assy.len(),
            5,
            "five nodes: root, partA, partB, partC, partD"
        );

        let b_index = mapped
            .all_nodes()
            .find(|node| node.entity().attrs.name == "partB")
            .expect("partB must exist")
            .index();
        let b_paths = occurrences
            .iter()
            .filter(|path| path.terminal_node().index() == b_index)
            .collect::<Vec<_>>();
        assert_eq!(
            b_paths.len(),
            2,
            "both occurrences terminate at the same definition node"
        );
        let translations = b_paths
            .iter()
            .map(|path| path.matrix().w)
            .collect::<Vec<_>>();
        assert!(
            translations[0] != translations[1],
            "the two placements differ"
        );
        let mut xs = translations.iter().map(|t| t.x).collect::<Vec<_>>();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            (xs[0] - (-1.0)).abs() < 1.0e-9 && (xs[1] - 2.0).abs() < 1.0e-9,
            "the two B placements must be the source frames: {xs:?}"
        );
    }

    /// The linked geometry is reachable and the occurrence world transform
    /// equals the source `ITEM_DEFINED_TRANSFORMATION` result.
    #[test]
    fn occurrence_world_transform_matches_the_source_frame() {
        let (_assy, mapped) = mapped_assembly();
        let top = mapped.top_nodes().next().expect("a root must exist");
        let a_index = mapped
            .all_nodes()
            .find(|node| node.entity().attrs.name == "partA")
            .expect("partA must exist")
            .index();
        let path = mapped
            .paths_iter(top.index())
            .find(|path| !path.edges().is_empty() && path.terminal_node().index() == a_index)
            .expect("partA occurrence must exist");
        let world = path.matrix();
        assert!(
            (world.w.x - 1.0).abs() < 1.0e-9
                && world.w.y.abs() < 1.0e-9
                && world.w.z.abs() < 1.0e-9,
            "partA occurrence world transform must be the source placement, got {:?}",
            world.w
        );
    }
}
