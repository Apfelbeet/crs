use std::collections::{VecDeque, HashSet};

use daggy::NodeIndex;
use priority_queue::PriorityQueue;
use crate::graph::Adag;
use super::rpa_util::RPANode;

pub mod shortest_path;
pub mod longest_path;

pub trait PathSelection {
    fn calculate_distances<E: Clone>(graph: &Adag<RPANode, E>, targets: &HashSet<NodeIndex>, valid_nodes: &HashSet<NodeIndex>) -> PriorityQueue<(NodeIndex, NodeIndex), i32>;
    fn extract_path<E>(graph: &Adag<RPANode, E>, source: NodeIndex, target: NodeIndex) -> VecDeque<NodeIndex>;
}