use rustc_hash::FxHashSet as HashSet;
use std::collections::VecDeque;
use truck_base::{cgmath64::*, tolerance::*};
use truck_meshalgo::prelude::PolylineCurve;

pub fn construct_polylines(lines: &[(Point3, Point3)]) -> Vec<PolylineCurve<Point3>> {
    let mut graph: Graph = lines.iter().collect();
    let mut res = Vec::new();
    while !graph.is_empty() {
        let (start_idx, node) = graph.get_one();
        let mut idx = start_idx;
        let mut wire: VecDeque<_> = vec![node.coord].into();
        while let Some((idx0, pt)) = graph.get_a_next_node(idx) {
            idx = idx0;
            wire.push_back(pt);
        }
        // The backward pass resumes from the START node: `wire[0]` is the
        // start node's own coordinate, and canonical representatives are
        // pairwise beyond the weld tolerance, so the start index IS that
        // node — no rescan needed.
        let mut idx = start_idx;
        while let Some((idx0, pt)) = graph.get_a_next_node(idx) {
            idx = idx0;
            wire.push_front(pt);
        }
        res.push(PolylineCurve(wire.into()));
    }
    res
}

struct Node {
    coord: Point3,
    adjacency: HashSet<usize>,
}

impl Node {
    #[inline(always)]
    fn new(coord: Point3, adjacency: HashSet<usize>) -> Node {
        Node { coord, adjacency }
    }

    fn pop_one_adjacency(&mut self) -> usize {
        let idx = *self.adjacency.iter().next().unwrap();
        self.adjacency.remove(&idx);
        idx
    }
}

struct Graph {
    nodes: Vec<Node>,
}

impl Graph {
    // O(n²) insertion is accepted: these are contact-network polylines, tens
    // of nodes. A spatial index would not pay for itself at that scale.
    //
    // BG-NUM-004 / F-2: node identity is a CANONICAL REPRESENTATIVE scanned
    // in insertion order — the FIRST stored representative within the legacy
    // near_pt tolerance IS the same node. This is position-independent
    // Euclidean welding, the exact defect class F-2 names: the old hash grid
    // split one logical node into different cells at some absolute positions
    // and welded distinct nodes at others.
    fn representative(&self, pt: Point3, ctx: &ToleranceCtx) -> usize {
        // BG-TOL-001: model — node identity welds at the legacy model-space
        // representation tolerance (F-2, BG-NUM-004).
        self.nodes
            .iter()
            .position(|node| ctx.near_pt(pt, node.coord))
            .unwrap_or(self.nodes.len())
    }

    fn add_half_edge(&mut self, pt0: Point3, pt1: Point3, ctx: &ToleranceCtx) {
        let idx0 = self.representative(pt0, ctx);
        if idx0 == self.nodes.len() {
            self.nodes.push(Node::new(pt0, HashSet::default()));
        }
        let idx1 = self.representative(pt1, ctx);
        if idx1 == self.nodes.len() {
            self.nodes.push(Node::new(pt1, HashSet::default()));
        }
        self.nodes[idx0].adjacency.insert(idx1);
    }

    fn add_edge(&mut self, line: (Point3, Point3)) {
        let ctx = ToleranceCtx::unscaled_legacy();
        if !ctx.near_pt(line.0, line.1) {
            // BG-TOL-001: model
            self.add_half_edge(line.0, line.1, &ctx);
            self.add_half_edge(line.1, line.0, &ctx);
        }
    }

    fn is_empty(&self) -> bool {
        self.nodes.iter().all(|node| node.adjacency.is_empty())
    }

    #[inline(always)]
    fn get_one(&self) -> (usize, &Node) {
        let idx = self
            .nodes
            .iter()
            .position(|node| !node.adjacency.is_empty())
            .unwrap();
        (idx, &self.nodes[idx])
    }

    fn get_a_next_node(&mut self, idx: usize) -> Option<(usize, Point3)> {
        let idx0 = {
            let node = self.nodes.get_mut(idx)?;
            if node.adjacency.is_empty() {
                return None;
            }
            node.pop_one_adjacency()
        };
        let node = self.nodes.get_mut(idx0)?;
        node.adjacency.remove(&idx);
        let pt = node.coord;
        Some((idx0, pt))
    }
}

impl<'a> FromIterator<&'a (Point3, Point3)> for Graph {
    fn from_iter<I: IntoIterator<Item = &'a (Point3, Point3)>>(iter: I) -> Graph {
        let mut res = Graph { nodes: Vec::new() };
        iter.into_iter().for_each(|line| res.add_edge(*line));
        res
    }
}

#[cfg(test)]
mod tests;
