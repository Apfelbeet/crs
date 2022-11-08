use std::collections::{HashSet, VecDeque, HashMap};

use daggy::{NodeIndex, Walker};
use priority_queue::PriorityQueue;

use crate::{graph::Adag, regression::rpa_util::RPANode};

use super::PathSelection;


pub struct LongestPath;

impl PathSelection for LongestPath {
    fn calculate_distances<E: Clone>(graph: &Adag<RPANode, E>, targets: &HashSet<NodeIndex>, valid_nodes: &HashSet<NodeIndex>) -> PriorityQueue<(NodeIndex, NodeIndex), i32> {
        let mut pq: PriorityQueue<(NodeIndex, NodeIndex), i32> = PriorityQueue::from_iter(valid_nodes.iter().map(|x| ((x.clone(), x.clone()), 0)));
        let mut distance: HashMap<NodeIndex, (NodeIndex, i32)> = HashMap::new();

        while !pq.is_empty() {
            let ((current_source, current_target), current_distance) = pq.pop().unwrap();

            let insert = match distance.get(&current_target) {
                Some((_, old_distance)) => *old_distance != 0 && current_distance > *old_distance,
                None => true,
            };

            if insert {
                distance.insert(current_target, (current_source, current_distance));
                for (_, child) in graph.graph.children(current_target).iter(&graph.graph) {
                    pq.push((current_source, child), current_distance + 1);
                }
            }
        }

        let mut res = PriorityQueue::new();
        for target in targets {
            let (src, dis) = distance[target];
            res.push((src, *target), dis);
        }
        return res;
    }

    fn extract_path<E>(graph: &Adag<RPANode, E>, source: NodeIndex, target: NodeIndex) -> VecDeque<NodeIndex> {
        let mut pq: PriorityQueue<(NodeIndex, NodeIndex), u32> = PriorityQueue::new();
        let mut distance = HashMap::<NodeIndex, (NodeIndex, u32)>::new();
        pq.push((source, source), 0);

        while !pq.is_empty() {
            let ((parent, current_target), current_distance) = pq.pop().unwrap();

            let insert = match distance.get(&current_target) {
                Some((_, old_distance)) => current_distance > *old_distance,
                None => true,
            };

            if insert {
                distance.insert(current_target, (parent, current_distance));
                for (_, child) in graph.graph.children(current_target).iter(&graph.graph) {
                    pq.push((current_target, child), current_distance + 1);
                }
            }
        }

        let mut path = VecDeque::<NodeIndex>::new();
        let mut child = target;
        loop {
            path.push_front(child);
            let (parent, _) = distance[&child];

            if parent == child {
                break;
            } else {
                child = parent;
            }
        }

        return path;
    }
}