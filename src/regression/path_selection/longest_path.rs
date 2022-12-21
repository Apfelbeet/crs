use std::collections::{HashMap, HashSet, VecDeque};

use daggy::{NodeIndex, Walker};
use priority_queue::PriorityQueue;

use crate::{graph::Adag, regression::rpa_util::RPANode};

use super::PathSelection;

pub struct LongestPath;

impl PathSelection for LongestPath {
    // fn calculate_distances<E: Clone>(
    //     graph: &Adag<RPANode, E>,
    //     targets: &HashSet<NodeIndex>,
    //     _: &HashSet<NodeIndex>,
    // ) -> PriorityQueue<(NodeIndex, NodeIndex), i32> {
    //     let mut parents = HashMap::<NodeIndex, usize>::new();
    //     let mut queue = VecDeque::<(NodeIndex, NodeIndex, i32)>::new();
    //     let mut longest_paths = PriorityQueue::<(NodeIndex, NodeIndex), i32>::new();
    //     let mut distances = HashMap::<NodeIndex, (NodeIndex, i32)>::new();

    //     for (_, index) in &graph.indexation {
    //         let number_parents = graph.graph.parents(*index).iter(&graph.graph).count();
    //         parents.insert(*index, number_parents);

    //         if number_parents == 0 {
    //             queue.push_front((*index, *index, 0));
    //             distances.insert(*index, (*index, 0));
    //         }
    //     }

    //     while !queue.is_empty() {
    //         let (parent_index, current_index, distance) = queue.pop_front().unwrap();

    //         if targets.contains(&current_index) {
    //             longest_paths.push((parent_index, current_index), distance);
    //         }

    //         for (_, child_index) in graph.graph.children(current_index).iter(&graph.graph) {
    //             let pets = parents[&child_index];
    //             parents.insert(child_index, pets - 1);

    //             let dis = distances.get(&child_index);
    //             match dis {
    //                 Some((_, d)) => {
    //                     if *d < distance + 1 {
    //                         distances.insert(child_index, (parent_index, distance + 1));
    //                     }
    //                 }
    //                 None => {
    //                     distances.insert(child_index, (parent_index, distance + 1));
    //                 }
    //             }

    //             if parents[&child_index] == 0 {
    //                 let (i, d) = distances[&child_index];
    //                 queue.push_back((i, child_index, d));
    //             }
    //         }
    //     }
    //     return longest_paths;
    // }

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
            queue.push_back((index.clone(), index.clone(), 0));
            visited.insert(index.clone());
        }

        while !queue.is_empty() {
            let (current_index, current_parent_index, current_distance) = queue.pop_front().unwrap();
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
    ) -> VecDeque<NodeIndex> {
        let mut parents = HashMap::<NodeIndex, usize>::new();
        let mut queue = VecDeque::<NodeIndex>::new();

        queue.push_back(source);
        while !queue.is_empty() {
            let current = queue.pop_front().unwrap();

            for (_, child) in graph.graph.children(current).iter(&graph.graph) {
                let pets = parents.get(&child);
                match pets {
                    Some(p) => {
                        parents.insert(child, p + 1);
                    }
                    None => {
                        parents.insert(child, 1);
                        queue.push_back(child);
                    }
                }
            }
        }

        let mut queue2 = VecDeque::<(NodeIndex, i32)>::new();
        let mut distances = HashMap::<NodeIndex, (NodeIndex, i32)>::new();
        let mut path = VecDeque::<NodeIndex>::new();

        queue2.push_back((source, 0));
        while !queue2.is_empty() {
            let (current, distance) = queue2.pop_front().unwrap();

            if current == target {
                break;
            }

            for (_, child) in graph.graph.children(current).iter(&graph.graph) {
                let pets = &parents[&child].clone();
                parents.insert(child, pets - 1);

                match distances.get(&child) {
                    Some((_, old_d)) => {
                        if distance + 1 > *old_d {
                            distances.insert(child, (current, distance + 1));
                        }
                    },
                    None => {
                        distances.insert(child, (current, distance + 1));
                    },
                }

                if pets - 1 == 0 {
                    let (_, dis) = distances[&child];
                    queue2.push_back((child, dis));
                }
            }
        }

        let mut c = &target;
        loop {
            path.push_front(c.clone());
            match distances.get(c) {
                Some((p, _)) => c = p,
                None => break,
            }
        }

        return path;
    }
}
