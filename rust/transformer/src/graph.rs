use std::collections::{BTreeMap, BTreeSet};

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

    /// Remove every node not in `keep`, together with all edges to and from it.
    pub fn restrict(&mut self, keep: &BTreeSet<String>) {
        self.0.retain(|node, _| keep.contains(node));
        for dependencies in self.0.values_mut() {
            dependencies.retain(|dependency| keep.contains(dependency));
        }
    }

    /// Remove every node not in `keep`, connecting its dependents directly to its dependencies.
    ///
    /// Each kept node depends on every kept node it reached directly or only through removed nodes.
    pub fn contract(&mut self, keep: &BTreeSet<String>) {
        self.0 = self
            .0
            .keys()
            .filter(|node| keep.contains(*node))
            .map(|node| (node.clone(), self.nearest_kept(node, keep)))
            .collect();
    }

    /// The kept nodes `node` reaches without passing through another kept node.
    fn nearest_kept(&self, node: &str, keep: &BTreeSet<String>) -> BTreeSet<String> {
        let mut reached = BTreeSet::new();
        let mut removed = BTreeSet::new();
        let mut todo: Vec<&String> = self.0.get(node).into_iter().flatten().collect();
        while let Some(dependency) = todo.pop() {
            if keep.contains(dependency) {
                if dependency != node {
                    reached.insert(dependency.clone());
                }
            } else if removed.insert(dependency) {
                todo.extend(self.0.get(dependency).into_iter().flatten());
            }
        }
        reached
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
    fn restrict_drops_nodes_and_edges() {
        let mut g = graph(&[("a", "b"), ("b", "c"), ("a", "c")]);
        g.restrict(&set(&["a", "c"]));
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([("a".into(), set(&["c"])), ("c".into(), set(&[]))])
        );
    }

    #[test]
    fn contract_connects_through_removed_nodes() {
        let mut g = graph(&[("a", "b"), ("b", "c"), ("c", "d"), ("a", "e")]);
        g.contract(&set(&["a", "d", "e"]));
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
        g.contract(&set(&["target", "app", "server"]));
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
    fn contract_keeps_direct_edges() {
        let mut g = graph(&[("a", "c"), ("a", "b"), ("b", "c")]);
        g.contract(&set(&["a", "b", "c"]));
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
        g.contract(&set(&["a", "d"]));
        assert_eq!(
            g.into_inner(),
            BTreeMap::from([("a".into(), set(&["d"])), ("d".into(), set(&[]))])
        );
    }
}
