use std::collections::{BTreeMap, BTreeSet};

/// Compute a topologically sorted list of every strongly connected component in
/// the given graph. The graph is provided as a mapping from nodes to their
/// outgoing edges.
///
/// Implements Tarjan's algorithm, which produces a topological ordering of
/// every strongly connected component in a graph. This is used generically by
/// other parts of the compiler.
pub fn find<T>(graph: &BTreeMap<T, BTreeSet<T>>) -> Vec<BTreeSet<&T>>
where
    T: Eq + Ord,
{
    let mut finder = ComponentFinder::new();
    for vertex in graph.keys() {
        finder.connect(graph, vertex);
    }

    finder.components
}

struct ComponentFinder<'a, T> {
    index: usize,

    indicies: BTreeMap<&'a T, usize>,
    lowlinks: BTreeMap<&'a T, usize>,

    stack: Vec<&'a T>,
    on_stack: BTreeSet<&'a T>,

    visited: BTreeSet<&'a T>,
    components: Vec<BTreeSet<&'a T>>,
}

impl<'a, T> ComponentFinder<'a, T>
where
    T: Eq + Ord,
{
    fn new() -> Self {
        Self {
            index: 0,
            indicies: BTreeMap::new(),
            lowlinks: BTreeMap::new(),
            stack: Vec::new(),
            on_stack: BTreeSet::new(),

            visited: BTreeSet::new(),
            components: Vec::new(),
        }
    }

    fn connect(&mut self, graph: &'a BTreeMap<T, BTreeSet<T>>, vertex: &'a T) {
        if !self.visited.insert(vertex) {
            return;
        }

        self.indicies.insert(vertex, self.index);
        self.lowlinks.insert(vertex, self.index);
        self.index += 1;

        self.stack.push(vertex);
        self.on_stack.insert(vertex);

        for child in graph.get(vertex).into_iter().flatten() {
            if !self.indicies.contains_key(child) {
                self.connect(graph, child);
                let lowlink = *self
                    .lowlinks
                    .get(&vertex)
                    .unwrap()
                    .min(self.lowlinks.get(child).unwrap());
                self.lowlinks.insert(vertex, lowlink);
            } else if self.on_stack.contains(child) {
                let lowlink = *self
                    .lowlinks
                    .get(&vertex)
                    .unwrap()
                    .min(self.indicies.get(child).unwrap());
                self.lowlinks.insert(vertex, lowlink);
            }
        }

        if self.lowlinks.get(&vertex) == self.indicies.get(&vertex) {
            let mut component = BTreeSet::new();
            while let Some(child) = self.stack.pop() {
                self.on_stack.remove(&child);
                component.insert(child);

                if child == vertex {
                    break;
                }
            }

            self.components.push(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::find;

    #[test]
    fn chain() {
        let graph = BTreeMap::from([
            (0, BTreeSet::new()),
            (1, BTreeSet::from([0])),
            (2, BTreeSet::from([1])),
        ]);

        let expected = vec![
            BTreeSet::from([&0]),
            BTreeSet::from([&1]),
            BTreeSet::from([&2]),
        ];

        let actual = find(&graph);

        assert_eq!(expected, actual);
    }

    #[test]
    fn cycle() {
        let graph = BTreeMap::from([
            (0, BTreeSet::from([2])),
            (1, BTreeSet::from([0])),
            (2, BTreeSet::from([1])),
        ]);

        let expected = vec![BTreeSet::from([&0, &1, &2])];

        let actual = find(&graph);

        assert_eq!(expected, actual);
    }

    #[test]
    fn depend_on_cycle() {
        let graph = BTreeMap::from([
            (0, BTreeSet::new()),
            (1, BTreeSet::from([2, 0])),
            (2, BTreeSet::from([1])),
            (3, BTreeSet::from([1])),
        ]);

        let expected = vec![
            BTreeSet::from([&0]),
            BTreeSet::from([&1, &2]),
            BTreeSet::from([&3]),
        ];

        let actual = find(&graph);

        assert_eq!(expected, actual);
    }

    #[test]
    fn disjoint() {
        let graph = BTreeMap::from([
            (0, BTreeSet::new()),
            (1, BTreeSet::new()),
            (2, BTreeSet::new()),
        ]);

        let actual = find(&graph);

        assert_eq!(3, actual.len());
        assert!(actual.contains(&BTreeSet::from([&0])));
        assert!(actual.contains(&BTreeSet::from([&1])));
        assert!(actual.contains(&BTreeSet::from([&2])));
    }
}
