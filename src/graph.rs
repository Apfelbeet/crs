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

pub fn associated_value_bisection(
    graph: &Adag<String, ()>,
    sources: &HashSet<NodeIndex>,
    ignored: &HashSet<NodeIndex>,
    target: NodeIndex,
) -> Option<NodeIndex> {
    // find all nodes reachable from the target
    // let mut relevant_sources = HashSet::<NodeIndex>::new();
    let mut relevant = HashSet::<NodeIndex>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    let mut irrelevant_queue = VecDeque::<NodeIndex>::new();
    let mut irrelevant = HashSet::<NodeIndex>::new();
    queue.push_back(target);
    relevant.insert(target);

    while let Some(current_index) = queue.pop_front() {
        if sources.contains(&current_index) {
            if irrelevant.insert(current_index) {
                irrelevant_queue.push_back(current_index);
            }
            continue;
        }

        for (_, parent_index) in graph.graph.parents(current_index).iter(&graph.graph) {
            if relevant.insert(parent_index) {
                queue.push_back(parent_index);
            }
        }
    }
    drop(queue);

    while let Some(current_index) = irrelevant_queue.pop_front() {
        for (_, parent_index) in graph.graph.parents(current_index).iter(&graph.graph) {
            if irrelevant.insert(parent_index) {
                irrelevant_queue.push_back(parent_index);
            }
        }
    }
    drop(irrelevant_queue);

    let mut start_points = HashSet::<NodeIndex>::new();
    let mut visited = HashSet::<NodeIndex>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    queue.push_back(target);
    visited.insert(target);

    while let Some(current_index) = queue.pop_front() {
        let parents =
            graph
                .graph
                .parents(current_index)
                .iter(&graph.graph)
                .filter(|(_, parent_index)| {
                    !irrelevant.contains(parent_index) && relevant.contains(parent_index)
                }).collect::<Vec<_>>();

        if parents.is_empty() {
            start_points.insert(current_index);
        } else {
            for (_, parent_index) in parents {
                if visited.insert(parent_index) {
                    queue.push_back(parent_index);
                }
            }
        }
    }

    drop(queue);
    drop(visited);

    //starting from the relevant sources, we're now calculating the number of parents for all nodes
    let mut number_of_parents = HashMap::<NodeIndex, usize>::new();
    let mut queue = VecDeque::<NodeIndex>::new();
    let mut size = start_points.len();

    for source in &start_points {
        queue.push_back(*source);
        number_of_parents.insert(*source, 0);
    }

    while let Some(current_index) = queue.pop_front() {
        if current_index == target {
            continue;
        }

        for (_, child_index) in graph.graph.children(current_index).iter(&graph.graph) {
            if !irrelevant.contains(&child_index) && relevant.contains(&child_index) {
                match number_of_parents.get_mut(&child_index) {
                    Some(np) => {
                        *np += 1;
                    }
                    None => {
                        number_of_parents.insert(child_index, 1);
                        queue.push_back(child_index);
                        size += 1;
                    }
                };
            }
        }
    }
    drop(queue);

    if size <= 1 {
        return None;
    }
    //calculating the number of ancestors, by performing a topological sort
    //we keep track of simple and disjunct sets of nodes in the graph.
    //For each of such a section we store the size of it in an hashmap
    //The number of ancestors for a node, is the size of all sets that have ancestors of that node
    //  + the number of ancestors in the set of the active nodes.
    let mut section_sizes = HashMap::<NodeIndex, usize>::new();
    let mut sections_of_nodes = HashMap::<NodeIndex, (HashSet<NodeIndex>, usize)>::new();
    let mut number_of_ancestors = HashMap::<NodeIndex, usize>::new();
    let mut queue = VecDeque::<NodeIndex>::new();

    for source in &start_points {
        queue.push_back(*source);
        sections_of_nodes.insert(*source, (HashSet::new(), 1));
    }

    while let Some(current_index) = queue.pop_front() {
        let mut anc = 0;
        let (sections, offset) = sections_of_nodes
            .remove(&current_index)
            .unwrap_or((HashSet::new(), 1));

        anc += offset;
        anc += sections
            .iter()
            .filter_map(|section| section_sizes.get(section))
            .sum::<usize>();
        number_of_ancestors.insert(current_index, anc);

        if current_index == target {
            break;
        }

        if anc > size / 2 {
            continue;
        }

        let children: Vec<NodeIndex> = graph
            .graph
            .children(current_index)
            .iter(&graph.graph)
            .map(|(_, n)| n)
            .filter(|n| relevant.contains(n) && !irrelevant.contains(n))
            .collect();

        match children.len().cmp(&1) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                let child_index = children[0];
                match sections_of_nodes.get_mut(&child_index) {
                    Some((child_section, child_offset)) => {
                        *child_offset += offset;
                        child_section.extend(sections.into_iter());
                    }
                    None => {
                        sections_of_nodes.insert(child_index, (sections, offset + 1));
                    }
                }
            }
            std::cmp::Ordering::Greater => {
                section_sizes.insert(current_index, offset);
                let mut new_sections = sections.clone();
                new_sections.insert(current_index);
                for child_index in &children {
                    match sections_of_nodes.get_mut(child_index) {
                        Some((child_section, _)) => {
                            child_section.extend(new_sections.iter());
                        }
                        None => {
                            sections_of_nodes.insert(*child_index, (new_sections.clone(), 1));
                        }
                    }
                }
            }
        };

        for child_index in children {
            if let Some(parents) = number_of_parents.get_mut(&child_index) {
                *parents -= 1;
                if *parents == 0 {
                    queue.push_back(child_index);
                }
            }
        }
    }

    let (bisection_point, _associated_value) = number_of_ancestors
        .into_iter()
        .filter(|(n, _)| !ignored.contains(n))
        .map(|(n, v)| (n, min(v, size - v)))
        .max_by(|(_, v1), (_, v2)| v1.cmp(v2))
        .unwrap();

    Some(bisection_point)
}
