use daggy::{Dag, NodeIndex};
use std::{
    cmp::{max, min},
    collections::{HashMap, VecDeque},
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
