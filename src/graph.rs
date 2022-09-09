use daggy::{Dag, EdgeIndex, NodeIndex, Walker};
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet, VecDeque},
};

#[derive(Debug, Clone)]
pub struct Radag<T> {
    pub graph: Dag<T, ()>,
    pub indexation: HashMap<String, NodeIndex>,
}

#[derive(Debug, Clone)]
pub enum KeypointEdge {
    Keypoint(u32),
    Normal,
}

enum PruneDirection {
    Up,
    Down,
}

pub fn prune(
    old_graph: &Radag<String>,
    roots: &Vec<String>,
    leaves: &Vec<String>,
) -> Radag<String> {
    let top_down = prune_general(old_graph, roots, get_children, PruneDirection::Down);
    let new_graph = prune_general(&top_down, leaves, get_parents, PruneDirection::Up);

    new_graph
}

fn prune_general<F>(
    old_graph: &Radag<String>,
    origin_nodes: &Vec<String>,
    func_next: F,
    direction: PruneDirection,
) -> Radag<String>
where
    F: Fn(&Dag<String, ()>, NodeIndex) -> Vec<(EdgeIndex, NodeIndex)>,
{
    let mut new_graph = Dag::<String, ()>::new();
    let mut indexation = HashMap::<String, NodeIndex>::new();
    let mut queued = HashMap::<NodeIndex, NodeIndex>::new();
    let mut visit_stack = Vec::<(NodeIndex, NodeIndex)>::new();

    for origin in origin_nodes {
        match old_graph.indexation.get(origin) {
            Some(index) => {
                let new_index = new_graph.add_node(origin.to_string());

                queued.insert(index.clone(), new_index);
                visit_stack.push((index.clone(), new_index));
                indexation.insert(origin.clone(), new_index);
            }
            None => eprintln!(
                "prune_general: Didn't find node for {}. Will ignore it!",
                origin
            ),
        }
    }

    while !visit_stack.is_empty() {
        let (old_current_index, new_current_index) = visit_stack.pop().unwrap();

        let next_nodes = func_next(&old_graph.graph, old_current_index);
        for (_, next_old_index) in next_nodes {
            match queued.get(&next_old_index) {
                // The node was never visited, thus we have to add it.
                None => {
                    let child_hash = old_graph
                        .graph
                        .node_weight(next_old_index)
                        .expect("Didn't found node in dvcs graph");

                    let (_, child_new_index) = match direction {
                        PruneDirection::Up => new_graph.add_parent(
                            new_current_index.clone(),
                            (),
                            child_hash.to_string(),
                        ),
                        PruneDirection::Down => new_graph.add_child(
                            new_current_index.clone(),
                            (),
                            child_hash.to_string(),
                        ),
                    };

                    queued.insert(next_old_index, child_new_index);
                    visit_stack.push((next_old_index, child_new_index));
                    indexation.insert(child_hash.to_string(), child_new_index);
                }

                //We already visited this node. So we don't have to revisit it,
                //but we need to add an edge from the current node.
                Some(next_new_index) => {
                    match direction {
                        PruneDirection::Up => new_graph
                            .add_edge(next_new_index.clone(), new_current_index, ())
                            .expect("Pruning error"),
                        PruneDirection::Down => new_graph
                            .add_edge(new_current_index, next_new_index.clone(), ())
                            .expect("Pruning error"),
                    };
                }
            }
        }
    }

    Radag {
        graph: new_graph,
        indexation,
    }
}

fn get_children(graph: &Dag<String, ()>, node: NodeIndex) -> Vec<(EdgeIndex, NodeIndex)> {
    graph.children(node).iter(graph).collect()
}

fn get_parents(graph: &Dag<String, ()>, node: NodeIndex) -> Vec<(EdgeIndex, NodeIndex)> {
    graph.parents(node).iter(graph).collect()
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

pub fn generate_keypoint_graph<S: Clone>(
    graph: &Dag<S, ()>,
    root: NodeIndex,
    preserve: HashSet<NodeIndex>,
) -> Dag<S, KeypointEdge> {
    let mut keypoint_graph = graph.filter_map(
        |_, weight| Some(weight.clone()),
        |_, _| Some(KeypointEdge::Normal),
    );

    let mut stack = Vec::<NodeIndex>::new();
    let mut last_keypoint = HashMap::<NodeIndex, (NodeIndex, u32)>::new();
    let mut missing_parents = HashMap::<NodeIndex, usize>::new();

    last_keypoint.insert(root, (root, 1));
    missing_parents.insert(root, 0);

    for (_, child) in keypoint_graph.children(root).iter(&keypoint_graph) {
        let parent_count = keypoint_graph.parents(child).iter(&keypoint_graph).count();

        if !missing_parents.contains_key(&child) {
            missing_parents.insert(child, parent_count - 1);
        }

        if parent_count - 1 == 0 {
            stack.push(child);
        }
    }

    while !stack.is_empty() {
        let current = stack.pop().unwrap();
        let children = keypoint_graph
            .children(current)
            .iter(&keypoint_graph)
            .collect::<Vec<_>>();

        //If the current node has exactly one parent and one child (and is also
        //not on the preserve list), then it isn't a keypoint:
        // - Don't add it to the new graph
        // - Reference last keypoint as the last keypoint of the parent.
        // - Increase distance by 1
        let children_count = children.len();
        let parents_count = keypoint_graph
            .parents(current)
            .iter(&keypoint_graph)
            .count();

        if children_count == 1 && parents_count == 1 && !preserve.contains(&current) {
            //UNWRAP: We only enter this branch if we have exactly one parent
            //node.
            let (_, parent) = keypoint_graph
                .parents(current)
                .iter(&keypoint_graph)
                .next()
                .unwrap();

            //DIRECT ACCESS: Every time we visit a node, we reference a node in
            //last_keypoint. A node can only be queue, if it has no unvisited
            //parent -> last_keypoint has a value for &parent.  
            let (parent_keypoint, distance) = last_keypoint[&parent].clone();

            last_keypoint.insert(current, (parent_keypoint, distance + 1));
        }

        //keypoint:
        //Otherwise the current node is a keypoint:
        // - Add a edge to the keypoint of each parent. Although two parents
        //   might have the same parent. They are representing different path.
        //   Therefore we want both to be added.
        // - Reference the current node as its own keypoint.
        else {
            let parents = keypoint_graph
                .parents(current)
                .iter(&keypoint_graph)
                .collect::<Vec<_>>();

            for (_, parent) in parents {

                //DIRECT ACCESS: Every time we visit a node, we reference a node in
                //last_keypoint. A node can only be queue, if it has no unvisited
                //parent -> last_keypoint has a value for &parent.
                let (parent_keypoint, distance) = last_keypoint[&parent].clone();
                keypoint_graph
                    .add_edge(parent_keypoint, current, KeypointEdge::Keypoint(distance))
                    .expect("Couldn't add edge to the graph!");
            }

            last_keypoint.insert(current, (current, 1));
        }

        //Queue children
        // - Decrease number of unvisited parents of child by 1. 
        // - If every parent of the child was visited, we push it onto the
        //   stack.
        for (_, child) in children {
            if !missing_parents.contains_key(&child) {
                let pc = keypoint_graph.parents(child).iter(&keypoint_graph).count();
                missing_parents.insert(child, pc);
            }

            let old_value = missing_parents[&child];
            let new_value = old_value - 1;
            missing_parents.insert(child, new_value);

            if new_value == 0 {
                stack.push(child);
            }
        }
    }

    keypoint_graph
}
