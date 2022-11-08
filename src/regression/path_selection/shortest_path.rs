use std::collections::{HashMap, HashSet, VecDeque};

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

        for source_index in valid_nodes.clone() {
            let mut distance = HashMap::new();
            let mut queue = VecDeque::new();

            queue.push_back(source_index);
            distance.insert(source_index, 0);

            while !queue.is_empty() {
                let current_index = queue.pop_front().unwrap();
                let current_distance: i32 = distance[&current_index];

                let children = graph
                    .graph
                    .children(current_index.clone())
                    .iter(&graph.graph);
                for (_, child_index) in children {
                    match distance.get(&child_index) {
                        Some(child_distance) => {
                            if current_distance + 1 > child_distance.clone() {
                                distance.insert(child_index, current_distance + 1);
                                if targets_indices.contains(&child_index) {
                                    shortest_path.change_priority(
                                        &(source_index, child_index),
                                        -(current_distance + 1),
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
                                    .push((source_index, child_index), -(current_distance + 1));
                            }
                        }
                    }
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
}
