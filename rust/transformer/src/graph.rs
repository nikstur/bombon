use std::collections::{BTreeMap, BTreeSet};

/// The edges [`Graph::contract`] adds where a node reached another one only through removed nodes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IndirectEdges {
    /// Add every such edge.
    All,
    /// Add only the edges to nodes that are not reachable from the node otherwise.
    Reduced,
}

/// A directed dependency graph over bom-refs, mapping each node to the nodes it depends on.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Graph(BTreeMap<String, BTreeSet<String>>);

impl Graph {
    /// Add a node without any edges. Does nothing if the node already exists.
    pub fn add_node(&mut self, node: String) {
        self.0.entry(node).or_default();
    }

    /// Add an edge from `dependent` to `dependency`, adding both nodes if necessary.
    ///
    /// Self-edges are ignored.
    pub fn add_edge(&mut self, dependent: String, dependency: String) {
        if dependent == dependency {
            self.add_node(dependent);
            return;
        }
        self.add_node(dependency.clone());
        self.0.entry(dependent).or_default().insert(dependency);
    }

    /// Add all nodes and edges of `other`.
    pub fn extend(&mut self, other: Graph) {
        for (dependent, dependencies) in other.0 {
            self.add_node(dependent.clone());
            for dependency in dependencies {
                self.add_edge(dependent.clone(), dependency);
            }
        }
    }

    /// Merge each node in `aliases` into the node it maps to, combining their edges.
    pub fn merge(&mut self, aliases: &BTreeMap<String, String>) {
        let resolve = |node: String| aliases.get(&node).cloned().unwrap_or(node);
        for (dependent, dependencies) in std::mem::take(&mut self.0) {
            let dependent = resolve(dependent);
            self.add_node(dependent.clone());
            for dependency in dependencies {
                self.add_edge(dependent.clone(), resolve(dependency));
            }
        }
    }

    /// Remove every node not in `keep`, connecting its dependents directly to its dependencies.
    ///
    /// Each kept node depends on every kept node it depended on directly, and on the kept nodes it
    /// reached only through removed nodes as selected by `indirect_edges`. Every node a kept node
    /// reached stays reachable from it.
    pub fn contract(&mut self, keep: &BTreeSet<String>, indirect_edges: IndirectEdges) {
        let mut indirect = Vec::new();
        self.0 = self
            .0
            .iter()
            .filter(|(node, _)| keep.contains(*node))
            .map(|(node, dependencies)| {
                let mut reached = BTreeSet::new();
                let mut removed = BTreeSet::new();
                let mut todo: Vec<&String> = dependencies.iter().collect();
                while let Some(dependency) = todo.pop() {
                    if keep.contains(dependency) {
                        if dependency != node {
                            reached.insert(dependency.clone());
                        }
                    } else if removed.insert(dependency) {
                        todo.extend(self.0.get(dependency).into_iter().flatten());
                    }
                }
                for dependency in reached.difference(dependencies) {
                    indirect.push((node.clone(), dependency.clone()));
                }
                (node.clone(), reached)
            })
            .collect();

        if indirect_edges == IndirectEdges::Reduced {
            // Each edge is checked against the current graph rather than all at once, which stays
            // correct when the graph has cycles.
            for (node, dependency) in indirect {
                if let Some(dependencies) = self.0.get_mut(&node) {
                    dependencies.remove(&dependency);
                }
                if !self.reaches(&node, &dependency) {
                    self.add_edge(node, dependency);
                }
            }
        }
    }

    fn reaches(&self, from: &str, to: &str) -> bool {
        let mut visited = BTreeSet::new();
        let mut todo = vec![from];
        while let Some(node) = todo.pop() {
            if node == to {
                return true;
            }
            if visited.insert(node) {
                todo.extend(self.0.get(node).into_iter().flatten().map(String::as_str));
            }
        }
        false
    }

    /// Nodes from which every node of the graph is reachable.
    ///
    /// These are the nodes no other node depends on, plus one node of each cycle that cannot be
    /// reached from them.
    pub fn entry_points(&self) -> Vec<String> {
        let dependencies: BTreeSet<&String> = self.0.values().flatten().collect();
        let mut entry_points = Vec::new();
        let mut reached = BTreeSet::new();
        let unreferenced = self.0.keys().filter(|node| !dependencies.contains(node));
        let rest = self.0.keys().filter(|node| dependencies.contains(node));
        for node in unreferenced.chain(rest) {
            if !reached.contains(node.as_str()) {
                entry_points.push(node.clone());
                self.extend_reachable(node, &mut reached);
            }
        }
        entry_points
    }

    /// The nodes that are not reachable from `from`.
    pub fn unreachable_from(&self, from: &str) -> Vec<&String> {
        let mut reached = BTreeSet::new();
        self.extend_reachable(from, &mut reached);
        self.0
            .keys()
            .filter(|node| !reached.contains(node.as_str()))
            .collect()
    }

    fn extend_reachable<'a>(&'a self, from: &'a str, reached: &mut BTreeSet<&'a str>) {
        let mut todo = vec![from];
        while let Some(node) = todo.pop() {
            if reached.insert(node) {
                todo.extend(self.0.get(node).into_iter().flatten().map(String::as_str));
            }
        }
    }

    /// The nodes and the dependencies of each node.
    pub fn into_inner(self) -> BTreeMap<String, BTreeSet<String>> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &str)]) -> Graph {
        let mut graph = Graph::default();
        for (dependent, dependency) in edges {
            graph.add_edge((*dependent).to_string(), (*dependency).to_string());
        }
        graph
    }

    fn set(nodes: &[&str]) -> BTreeSet<String> {
        nodes.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn self_edges_are_ignored() {
        let g = graph(&[("a", "a")]).into_inner();
        assert_eq!(g, BTreeMap::from([("a".into(), set(&[]))]));
    }

    #[test]
    fn merge_combines_edges_without_self_edges() {
        // `a-bin` is a duplicate of `a`, and depends on it like a second output would.
        let mut g = graph(&[("x", "a-bin"), ("a-bin", "a"), ("a-bin", "c"), ("a", "d")]);
        g.merge(&BTreeMap::from([("a-bin".into(), "a".into())]));
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("a".into(), set(&["c", "d"])),
                ("c".into(), set(&[])),
                ("d".into(), set(&[])),
                ("x".into(), set(&["a"])),
            ])
        );
    }

    #[test]
    fn contract_connects_through_removed_nodes() {
        let mut g = graph(&[("a", "b"), ("b", "c"), ("c", "d"), ("a", "e")]);
        g.contract(&set(&["a", "d", "e"]), IndirectEdges::All);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("a".into(), set(&["d", "e"])),
                ("d".into(), set(&[])),
                ("e".into(), set(&[])),
            ])
        );
    }

    #[test]
    fn contract_keeps_indirect_edges_to_otherwise_reachable_nodes() {
        // `unit` is not a component, and `target` reaches `server` through `app` as well.
        let mut g = graph(&[
            ("target", "unit"),
            ("unit", "server"),
            ("target", "app"),
            ("app", "server"),
        ]);
        g.contract(&set(&["target", "app", "server"]), IndirectEdges::All);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("app".into(), set(&["server"])),
                ("server".into(), set(&[])),
                ("target".into(), set(&["app", "server"])),
            ])
        );
    }

    #[test]
    fn contract_reduced_skips_indirect_edges_to_otherwise_reachable_nodes() {
        // `stdenv` is not a component, and `hello` reaches `gcc` through `make` as well.
        let mut g = graph(&[
            ("hello", "stdenv"),
            ("stdenv", "gcc"),
            ("stdenv", "make"),
            ("make", "gcc"),
        ]);
        g.contract(&set(&["hello", "make", "gcc"]), IndirectEdges::Reduced);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("gcc".into(), set(&[])),
                ("hello".into(), set(&["make"])),
                ("make".into(), set(&["gcc"])),
            ])
        );
    }

    #[test]
    fn contract_reduced_keeps_direct_edges() {
        let mut g = graph(&[("a", "c"), ("a", "b"), ("b", "c")]);
        g.contract(&set(&["a", "b", "c"]), IndirectEdges::Reduced);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("a".into(), set(&["b", "c"])),
                ("b".into(), set(&["c"])),
                ("c".into(), set(&[])),
            ])
        );
    }

    #[test]
    fn contract_keeps_direct_edges() {
        let mut g = graph(&[("a", "c"), ("a", "b"), ("b", "c")]);
        g.contract(&set(&["a", "b", "c"]), IndirectEdges::All);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("a".into(), set(&["b", "c"])),
                ("b".into(), set(&["c"])),
                ("c".into(), set(&[])),
            ])
        );
    }

    #[test]
    fn contract_handles_cycles_through_removed_nodes() {
        let mut g = graph(&[("a", "b"), ("b", "c"), ("c", "b"), ("c", "a"), ("c", "d")]);
        g.contract(&set(&["a", "d"]), IndirectEdges::Reduced);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([("a".into(), set(&["d"])), ("d".into(), set(&[]))])
        );
    }

    #[test]
    fn contract_reduced_handles_cycles_between_kept_nodes() {
        // `gcc-stage1` built `glibc` and is merged into `gcc`, which depends on `glibc`.
        let mut g = graph(&[
            ("hello", "stdenv"),
            ("stdenv", "gcc"),
            ("stdenv", "glibc"),
            ("gcc", "glibc"),
            ("glibc", "stdenv-stage1"),
            ("stdenv-stage1", "gcc-stage1"),
        ]);
        g.merge(&BTreeMap::from([("gcc-stage1".into(), "gcc".into())]));
        g.contract(&set(&["hello", "gcc", "glibc"]), IndirectEdges::Reduced);
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([
                ("gcc".into(), set(&["glibc"])),
                ("glibc".into(), set(&["gcc"])),
                ("hello".into(), set(&["glibc"])),
            ])
        );
    }

    #[test]
    fn entry_points_are_unreferenced_nodes() {
        let g = graph(&[("a", "b"), ("c", "b"), ("b", "d")]);
        assert_eq!(g.entry_points(), ["a", "c"]);
    }

    #[test]
    fn entry_points_include_unreachable_cycles() {
        let g = graph(&[("a", "b"), ("c", "d"), ("d", "c"), ("d", "e")]);
        assert_eq!(g.entry_points(), ["a", "c"]);
    }

    #[test]
    fn unreachable_from_lists_unreachable_nodes() {
        let g = graph(&[("a", "b"), ("c", "b")]);
        assert_eq!(g.unreachable_from("a"), ["c"]);
    }
}
