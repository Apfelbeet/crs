use daggy::{petgraph::graph, Dag, EdgeIndex, NodeIndex, Walker};
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet, VecDeque},
};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct Adag<N, E> {
    pub sources: Vec<String>,
    pub targets: Vec<String>,
    pub graph: Dag<N, E>,
    pub indexation: HashMap<String, NodeIndex>,
}

impl<N: Clone,E> Adag<N, E>  {
    pub fn node(&self, hash: &String) -> N {
        self.graph.node_weight(self.index(hash)).expect("Radag seems corrupted!").clone()
    }

    pub fn node_from_index(&self, index: NodeIndex) -> N {
        self.graph.node_weight(index.clone()).expect("Radag seems corrupted!").clone()
    }

    pub fn index(&self, hash: &String) -> NodeIndex {
        self.indexation.get(hash).expect(&format!("{} is not a node in the graph!", hash)).clone()
    }
}

// type KeypointEdge = u32;

// #[derive(Debug, Clone, PartialEq)]
// enum PruneDirection {
//     Up,
//     Down,
// }

// pub fn prune<E: Clone>(
//     old_graph: &Adag<String, E>,
//     roots: &Vec<String>,
//     leaves: &Vec<String>,
// ) -> Adag<String, E> {
//     let top_down = prune_general(old_graph, roots, get_children, PruneDirection::Down);
//     let new_graph = prune_general(&top_down, leaves, get_parents, PruneDirection::Up);

//     new_graph
// }

// fn get_children<E>(graph: &Dag<String, E>, node: NodeIndex) -> Vec<(EdgeIndex, NodeIndex)> {
//     graph.children(node).iter(graph).collect()
// }

// fn get_parents<E>(graph: &Dag<String, E>, node: NodeIndex) -> Vec<(EdgeIndex, NodeIndex)> {
//     graph.parents(node).iter(graph).collect()
// }

// fn prune_general<F, E: Clone>(
//     old_graph: &Adag<String, E>,
//     origin_nodes: &Vec<String>,
//     func_next: F,
//     direction: PruneDirection,
// ) -> Adag<String, E>
// where
//     F: Fn(&Dag<String, E>, NodeIndex) -> Vec<(EdgeIndex, NodeIndex)>,
// {
//     //If we have more than one origin node for the DAG and we're pruning from
//     //top to bottom, then we can not longer ensure that the resulting graph has
//     //one root.
//     //
//     //A valid case would be all origin nodes are children of one origin node.
//     //But this case is annoying to check and right now it's not relevant.
//     if direction == PruneDirection::Down && origin_nodes.len() > 1 {
//         panic!("Can not prune rooted dag from top to bottom with more than one origins!");
//     }

//     let mut new_graph = Dag::<String, E>::new();
//     let mut indexation = HashMap::<String, NodeIndex>::new();
//     let mut queued = HashMap::<NodeIndex, NodeIndex>::new();
//     let mut visit_stack = Vec::<(NodeIndex, NodeIndex)>::new();

//     for origin in origin_nodes {
//         match old_graph.indexation.get(origin) {
//             Some(index) => {
//                 let new_index = new_graph.add_node(origin.to_string());

//                 queued.insert(index.clone(), new_index);
//                 visit_stack.push((index.clone(), new_index));
//                 indexation.insert(origin.clone(), new_index);
//             }
//             None => eprintln!(
//                 "prune_general: Didn't find node for {}. Will ignore it!",
//                 origin
//             ),
//         }
//     }

//     while !visit_stack.is_empty() {
//         let (old_current_index, new_current_index) = visit_stack.pop().unwrap();

//         let next_nodes = func_next(&old_graph.graph, old_current_index);
//         for (edge_to_next, next_old_index) in next_nodes {
//             let edge = old_graph
//                 .graph
//                 .edge_weight(edge_to_next)
//                 .expect("Didn't found edge in dvcs graph!");

//             match queued.get(&next_old_index) {
//                 // The node was never visited, thus we have to add it.
//                 None => {
//                     let child_hash = old_graph
//                         .graph
//                         .node_weight(next_old_index)
//                         .expect("Didn't found node in dvcs graph");

//                     let (_, child_new_index) = match direction {
//                         PruneDirection::Up => new_graph.add_parent(
//                             new_current_index.clone(),
//                             edge.clone(),
//                             child_hash.to_string(),
//                         ),
//                         PruneDirection::Down => new_graph.add_child(
//                             new_current_index.clone(),
//                             edge.clone(),
//                             child_hash.to_string(),
//                         ),
//                     };

//                     queued.insert(next_old_index, child_new_index);
//                     visit_stack.push((next_old_index, child_new_index));
//                     indexation.insert(child_hash.to_string(), child_new_index);
//                 }

//                 //We already visited this node. So we don't have to revisit it,
//                 //but we need to add an edge from the current node.
//                 Some(next_new_index) => {
//                     match direction {
//                         PruneDirection::Up => new_graph
//                             .add_edge(next_new_index.clone(), new_current_index, edge.clone())
//                             .expect("Pruning error"),
//                         PruneDirection::Down => new_graph
//                             .add_edge(new_current_index, next_new_index.clone(), edge.clone())
//                             .expect("Pruning error"),
//                     };
//                 }
//             }
//         }
//     }

//     let root = if direction == PruneDirection::Down {
//         origin_nodes.first().unwrap().clone()
//     } else {
//         old_graph.root.clone()
//     };

//     Adag {
//         root,
//         graph: new_graph,
//         indexation,
//     }
// }

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
    indexation.into_iter().map(|(k,v)| (v.clone(), k.clone())).collect()
}
