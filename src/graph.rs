use daggy::{Dag, EdgeIndex, NodeIndex, Walker};
use priority_queue::DoublePriorityQueue;
use std::hash::Hash;
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet, VecDeque},
};

use crate::regression::TestResult;
use crate::regression::rpa_search::RPANode;

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
            .node_weight(index.clone())
            .expect("Radag seems corrupted!")
            .clone()
    }

    pub fn index(&self, hash: &String) -> NodeIndex {
        self.indexation
            .get(hash)
            .expect(&format!("{} is not a node in the graph!", hash))
            .clone()
    }

    pub fn hash_from_index(&self, index: NodeIndex) -> String {
        let (k, v) = self.indexation.iter().find(|(k, v)| v == &&index).unwrap();
        k.clone()
    }

    pub fn calculate_distances(&self) -> DoublePriorityQueue<(NodeIndex, NodeIndex), u32> {
        let mut shortest_path = DoublePriorityQueue::new();
        let targets_indices: HashSet<NodeIndex> =
            HashSet::from_iter(self.targets.iter().map(|hash| self.index(hash)));

        for source in self.sources.clone() {
            let source_index = self.index(&source);
            let mut distance = HashMap::new();
            let mut queue = VecDeque::new();

            queue.push_back(source_index);
            distance.insert(source_index, 0);

            while !queue.is_empty() {
                let current_index = queue.pop_front().unwrap();
                let current_distance: u32 = distance[&current_index];

                let children = self.graph.children(current_index.clone()).iter(&self.graph);
                for (_, child_index) in children {
                    match distance.get(&child_index) {
                        Some(child_distance) => {
                            if current_distance + 1 < child_distance.clone() {
                                distance.insert(child_index, current_distance + 1);
                                if targets_indices.contains(&child_index) {
                                    shortest_path.change_priority(
                                        &(source_index, child_index),
                                        current_distance + 1,
                                    );
                                }
                                queue.push_back(child_index);
                            }
                        }
                        None => {
                            distance.insert(child_index, current_distance + 1);
                            queue.push_back(child_index);
                            if targets_indices.contains(&child_index) {
                                shortest_path
                                    .push((source_index, child_index), current_distance + 1);
                            }
                        }
                    }
                }
            }
        }
        shortest_path
    }

    // pub fn pruned(&self) -> Adag<N, E> {
    //     prune(self, &self.sources, &self.targets)
    // }
}

// pub fn prune<N: Clone, E: Clone>(
//     graph: &Adag<N, E>,
//     sources: &Vec<String>,
//     targets: &Vec<String>,
// ) -> Adag<N, E> {
//     //Phase 1: start at the targets and mark all upwards reachable nodes.

//     let mut reachable_upwards: HashSet<NodeIndex> = HashSet::new();
//     let mut q: VecDeque<NodeIndex> = VecDeque::new();

//     for target in targets {
//         let target_index = graph.index(target);
//         reachable_upwards.insert(target_index);
//         q.push_back(target_index);
//     }

//     while !q.is_empty() {
//         let current_index = q.pop_front().unwrap();

//         if !sources.contains(&graph.hash_from_index(current_index)) {
//             for (_, parent_index) in graph.graph.parents(current_index).iter(&graph.graph) {
//                 if reachable_upwards.insert(parent_index) {
//                     q.push_back(parent_index);
//                 }
//             }
//         }
//     }

//     //Phase 2: start at the sources and mark all downwards reachable nodes.

//     let mut reachable_downwards: HashSet<NodeIndex> = HashSet::new();

//     for source in sources {
//         let source_index = graph.index(source);
//         reachable_downwards.insert(source_index);
//         q.push_back(source_index);
//     }

//     while !q.is_empty() {
//         let current_index = q.pop_front().unwrap();

//         for (_, parent_index) in graph.graph.children(current_index).iter(&graph.graph) {
//             if reachable_downwards.insert(parent_index) {
//                 q.push_back(parent_index);
//             }
//         }
//     }

//     //Phase 3: filter all nodes of the graph that are upwards and downwards
//     //reachable.
//     let temp_graph = graph.graph.filter_map(
//         |i, n| {
//             if reachable_downwards.contains(&i) && reachable_upwards.contains(&i) {
//                 Some((i, n.clone()))
//             } else {
//                 None
//             }
//         },
//         |_, e| Some(e.clone()),
//     );

//     let mut new_indexation = HashMap::new();
//     let new_graph = temp_graph.filter_map(
//         |i, (i_old, n)| {
//             new_indexation.insert(graph.hash_from_index(i_old.clone()), i);
//             Some(n.clone())
//         },
//         |_, e| Some(e.clone()),
//     );

//     let new_sources = sources
//         .iter()
//         .filter(|hash| {
//             let i = graph.index(hash);
//             reachable_downwards.contains(&i) && reachable_upwards.contains(&i)
//         })
//         .cloned()
//         .collect::<Vec<String>>();

//     let new_targets = targets
//         .iter()
//         .filter(|hash| {
//             let i = graph.index(hash);
//             reachable_downwards.contains(&i) && reachable_upwards.contains(&i)
//         })
//         .cloned()
//         .collect::<Vec<String>>();

//     Adag {
//         sources: new_sources,
//         targets: new_targets,
//         graph: new_graph,
//         indexation: new_indexation,
//     }
// }

pub fn bfs_valid<E: Clone>(graph: &Adag<RPANode, E>, start: NodeIndex) -> NodeIndex {
    let mut q = VecDeque::new();
    let mut vis = HashSet::new();

    let mut f = None;
    q.push_back(start);
    while !q.is_empty() {
        let current_index = q.pop_front().unwrap();
        
        
        let res = graph.node_from_index(current_index).result;
        if  res.is_some() && res.unwrap() == TestResult::True {
            f = Some(current_index);
            break;
        }
        
        let parents = graph.graph.parents(current_index);
        for (_, parent_index) in parents.iter(&graph.graph) {
            if vis.insert(parent_index) {
                q.push_back(parent_index);
            }
        }
    }

    return f.unwrap();
}

pub fn shortest_path<N, E>(
    graph: &Dag<N, E>,
    start: NodeIndex,
    target: NodeIndex,
) -> VecDeque<NodeIndex> {
    let mut queue = VecDeque::new();
    let mut parent = HashMap::new();

    queue.push_back(start);

    while !queue.is_empty() {
        let current = queue.pop_front().unwrap();

        for (_, child) in graph.children(current).iter(&graph) {
            if !parent.contains_key(&child) {
                parent.insert(child, current);
                queue.push_back(child);

                if child == target {
                    queue.clear();
                    break;
                }
            }
        }
    }

    let mut path = VecDeque::<NodeIndex>::new();
    let mut c = Some(target);
    while c.is_some() {
        path.push_front(c.unwrap());
        c = parent.get(&c.unwrap()).cloned();
    }

    return path;
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

// type KeypointEdge = u32;
/// Generates a rooted keypoint graph from a given rooted graph.
///
/// A keypoint graph merges all nodes that are not a fork or a merge. In other
/// words, every node that has exactly one parent and one child. Additionally it
/// keeps all nodes in the preserve set.
///
/// The weights of a edges is the length of the path in the original graph.
///
/// The keypoint graph keeps the indexation of the original graph for all nodes,
/// that still exist afterwards.
// pub fn generate_keypoint_graph<N: Clone, E>(
//     radag: &Adag<N, E>,
//     preserve: HashSet<NodeIndex>,
// ) -> Adag<N, KeypointEdge> {
//     //Indices ending with _o/_n are for the old/new graph, respectively.

//     // Clone the original graph without the edges.
//     let mut keypoint_graph = Dag::<N, KeypointEdge>::new();
//     let mut indexation_n = HashMap::<String, NodeIndex>::new();
//     let reversed_indexation_o = reverse_hash_map(&radag.indexation);
//     let root_index_o = radag
//         .indexation
//         .get(&radag.root)
//         .expect("Passed graph is invalid!")
//         .clone();

//     // Performing topological sort, while identifying keypoints and add edges
//     // between those.
//     let mut stack = Vec::<NodeIndex>::new();
//     let mut last_keypoint = HashMap::<NodeIndex, (NodeIndex, u32)>::new();
//     let mut missing_parents = HashMap::<NodeIndex, usize>::new();

//     let root_index_n =
//         keypoint_graph.add_node(radag.graph.node_weight(root_index_o).unwrap().clone());
//     indexation_n.insert(radag.root.clone(), root_index_n);
//     last_keypoint.insert(root_index_o, (root_index_n, 1));
//     missing_parents.insert(root_index_o, 0);

//     for (_, child_o) in radag.graph.children(root_index_o).iter(&radag.graph) {
//         let parent_count = radag.graph.parents(child_o).iter(&radag.graph).count();

//         //The node hasn't been visited yet.
//         // - add amount of unvisited parents
//         if !missing_parents.contains_key(&child_o) {
//             missing_parents.insert(child_o, parent_count - 1);
//         }

//         if parent_count - 1 == 0 {
//             stack.push(child_o);
//         }
//     }

//     while !stack.is_empty() {
//         let current_o = stack.pop().unwrap();
//         let children_o = radag
//             .graph
//             .children(current_o)
//             .iter(&radag.graph)
//             .collect::<Vec<_>>();

//         //If the current node has exactly one parent and one child (and is also
//         //not on the preserve list), then it isn't a keypoint:
//         // - Don't add it to the new graph
//         // - Reference last keypoint as the last keypoint of the parent.
//         // - Increase distance by 1
//         let children_count = children_o.len();
//         let parents_count = radag.graph.parents(current_o).iter(&radag.graph).count();

//         if children_count == 1 && parents_count == 1 && !preserve.contains(&current_o) {
//             //UNWRAP: We only enter this branch if we have exactly one parent
//             //node.
//             let (_, parent) = radag
//                 .graph
//                 .parents(current_o)
//                 .iter(&radag.graph)
//                 .next()
//                 .unwrap();

//             //DIRECT ACCESS: Every time we visit a node, we reference a node in
//             //last_keypoint. A node can only be queue, if it has no unvisited
//             //parent -> last_keypoint has a value for &parent.
//             let (parent_keypoint, distance) = last_keypoint[&parent].clone();

//             last_keypoint.insert(current_o, (parent_keypoint, distance + 1));
//         }
//         //keypoint:
//         //Otherwise the current node is a keypoint:
//         // - Add this node to the new graph.
//         // - Add a edge to the last keypoint of each parent.
//         // - Reference the current node as its own keypoint.
//         // - Add mapping from the old index to the new index.
//         else {
//             let weight = radag.graph.node_weight(current_o).unwrap().clone();
//             let hash = reversed_indexation_o.get(&current_o).unwrap().clone();
//             let current_n = keypoint_graph.add_node(weight);
//             indexation_n.insert(hash, current_n);

//             let parents = radag
//                 .graph
//                 .parents(current_o)
//                 .iter(&radag.graph)
//                 .collect::<Vec<_>>();
//             for (_, parent) in parents {
//                 //DIRECT ACCESS: Every time we visit a node, we reference a node in
//                 //last_keypoint. A node can only be queue, if it has no unvisited
//                 //parent -> last_keypoint has a value for &parent.
//                 let (parent_keypoint_n, distance) = last_keypoint[&parent].clone();
//                 keypoint_graph
//                     .add_edge(parent_keypoint_n, current_n, distance)
//                     .expect("Couldn't add edge to the graph!");
//             }

//             last_keypoint.insert(current_o, (current_n, 1));
//         }

//         //Queue children
//         // - Decrease number of unvisited parents of child by 1.
//         // - If every parent of the child was visited, we push it onto the
//         //   stack.
//         for (_, child) in children_o {
//             if !missing_parents.contains_key(&child) {
//                 let pc = radag.graph.parents(child).iter(&radag.graph).count();
//                 missing_parents.insert(child, pc);
//             }

//             let old_value = missing_parents[&child];
//             let new_value = old_value - 1;
//             missing_parents.insert(child, new_value);

//             if new_value == 0 {
//                 stack.push(child);
//             }
//         }
//     }

//     Adag {
//         graph: keypoint_graph,
//         root: radag.root.clone(),
//         indexation: indexation_n,
//     }
// }

fn reverse_hash_map<A: Clone, B: Clone + Eq + Hash>(indexation: &HashMap<A, B>) -> HashMap<B, A> {
    indexation
        .into_iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect()
}
