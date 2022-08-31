use daggy::{Dag, EdgeIndex, NodeIndex, Walker};
use std::{collections::{HashMap, VecDeque}, cmp::{min, max}};

#[derive(Debug, Clone)]
pub struct Radag<T> {
    pub graph: Dag<T, ()>,
    pub indexation: HashMap<String, NodeIndex>,
}

enum PruneDirection {
    Up,
    Down
}

pub fn prune(old_graph: &Radag<String>, roots: &Vec<String>, leaves: &Vec<String>) -> Radag<String> {
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
                        PruneDirection::Up => new_graph.add_parent(new_current_index.clone(), (), child_hash.to_string()),
                        PruneDirection::Down => new_graph.add_child(new_current_index.clone(), (), child_hash.to_string()),
                    };
                    
                    queued.insert(next_old_index, child_new_index);
                    visit_stack.push((next_old_index, child_new_index));
                    indexation.insert(child_hash.to_string(), child_new_index);
                },

                //We already visited this node. So we don't have to revisit it,
                //but we need to add an edge from the current node.
                Some(next_new_index) => {
                    match direction {
                        PruneDirection::Up => new_graph.add_edge(next_new_index.clone(), new_current_index, ()).expect("Pruning error"),
                        PruneDirection::Down => new_graph.add_edge(new_current_index, next_new_index.clone(), ()).expect("Pruning error"),
                    };
                },
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

pub fn shortest_path<N, E>(graph: &Dag<N, E>, start: NodeIndex, target: NodeIndex) -> VecDeque<NodeIndex> {
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