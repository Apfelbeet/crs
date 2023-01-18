use daggy::{petgraph::visit::IntoNodeIdentifiers, Dag, NodeIndex, Walker};
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet, VecDeque},
};

#[derive(Debug, Clone)]
pub struct Adag<N, E> {
    pub sources: Vec<String>,
    pub targets: Vec<String>,
    pub graph: Dag<N, E>,
    pub indexation: HashMap<String, NodeIndex>,
}

impl<N: Clone, E: Clone> Adag<N, E> {
    pub fn node(&self, hash: &String) -> N {
        self.graph
            .node_weight(self.index(hash))
            .expect("Radag seems corrupted!")
            .clone()
    }

    pub fn node_from_index(&self, index: NodeIndex) -> N {
        self.graph
            .node_weight(index)
            .expect("Radag seems corrupted!")
            .clone()
    }

    pub fn index(&self, hash: &String) -> NodeIndex {
        *self
            .indexation
            .get(hash)
            .unwrap_or_else(|| panic!("{} is not a node in the graph!", hash))
    }

    pub fn hash_from_index(&self, index: NodeIndex) -> String {
        let (k, _) = self.indexation.iter().find(|(_, v)| v == &&index).unwrap();
        k.clone()
    }
}

pub fn length_of_path<S: Eq>(path: &VecDeque<S>, left: &S, right: &S) -> Result<usize, ()> {
    let mut left_index = None;
    let mut right_index = None;

    let mut found = false;
    for (index, node) in path.iter().enumerate() {
        if node == left {
            left_index = Some(index)
        }
        if node == right {
            right_index = Some(index)
        }
        if left_index.is_some() && right_index.is_some() {
            found = true;
            break;
        }
    }

    if found {
        let l = min(left_index.unwrap(), right_index.unwrap());
        let r = max(left_index.unwrap(), right_index.unwrap());

        Ok(r - l + 1)
    } else {
        Err(())
    }
}

pub fn prune_downwards(
    graph: &Dag<String, ()>,
    sources: &[NodeIndex],
) -> (Dag<String, ()>, HashMap<String, NodeIndex>) {
    let mut q: Vec<NodeIndex> = sources.to_owned();
    let mut marked: HashSet<NodeIndex> = HashSet::from_iter(sources.iter().cloned());

    while !q.is_empty() {
        let current = q.pop().unwrap();

        for (_, child) in graph.children(current).iter(graph) {
            if marked.insert(child) {
                q.push(child);
            }
        }
    }

    let new_graph = graph.filter_map(
        |n, s| {
            if marked.contains(&n) {
                Some(s.clone())
            } else {
                None
            }
        },
        |_, _| Some(()),
    );

    let mut indexation = HashMap::<String, NodeIndex>::new();
    for index in new_graph.node_identifiers() {
        indexation.insert(new_graph.node_weight(index).unwrap().clone(), index);
    }

    (new_graph, indexation)
}
