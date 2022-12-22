use std::collections::{HashMap, HashSet, VecDeque, hash_map::Entry::Vacant};

use daggy::{NodeIndex, Walker};
use priority_queue::PriorityQueue;

use crate::{graph::Adag, regression::rpa_util::RPANode};

use super::PathSelection;

pub struct ShortestPath;

impl PathSelection for ShortestPath {
    fn calculate_distances<E: Clone>(
        graph: &Adag<RPANode, E>,
        targets: &HashSet<NodeIndex>,
        valid_nodes: &HashSet<NodeIndex>,
    ) -> PriorityQueue<(NodeIndex, NodeIndex), i32> {
        let mut shortest_path = PriorityQueue::new();
        let targets_indices: HashSet<NodeIndex> = targets.clone();
        let mut queue: VecDeque<(NodeIndex, NodeIndex, i32)> = VecDeque::new();
        let mut visited: HashSet<NodeIndex> = HashSet::new();

        for index in valid_nodes {
            queue.push_back((*index, *index, 0));
            visited.insert(*index);
        }

        while !queue.is_empty() {
            let (current_index, current_parent_index, current_distance) =
                queue.pop_front().unwrap();
            if targets_indices.contains(&current_index) {
                shortest_path.push((current_parent_index, current_index), -current_distance);
            }

            for (_, child) in graph.graph.children(current_index).iter(&graph.graph) {
                if visited.insert(child) {
                    queue.push_back((child, current_parent_index, current_distance + 1));
                }
            }
        }

        shortest_path
    }

    fn extract_path<E>(
        graph: &Adag<RPANode, E>,
        source: NodeIndex,
        target: NodeIndex,
    ) -> std::collections::VecDeque<daggy::NodeIndex> {
        let mut queue = VecDeque::new();
        let mut parent = HashMap::new();

        queue.push_back(source);

        while !queue.is_empty() {
            let current = queue.pop_front().unwrap();

            for (_, child) in graph.graph.children(current).iter(&graph.graph) {
                //Goes into that branch if the entry does not exist.
                if let Vacant(e) = parent.entry(child) {
                    e.insert(current);
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

        path
    }
}
