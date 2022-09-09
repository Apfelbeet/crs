use priority_queue::DoublePriorityQueue;
use std::{
    cmp::min,
    collections::{HashMap, HashSet, VecDeque},
};

use daggy::{NodeIndex, Walker};

use crate::graph::{prune, shortest_path, Radag};

use super::{
    AssignedRegressionPoint, PathAlgorithm, RegressionAlgorithm, RegressionPoint, TestResult,
};

pub struct Settings {
    pub propagate: bool,
}

#[derive(Debug, Clone)]
pub struct RPANode {
    pub result: Option<TestResult>,
    pub hash: String,
    // pub distance: u32,
}

pub struct RPA<S: PathAlgorithm + RegressionAlgorithm, E> {
    commits: Radag<RPANode, E>,
    shortest_paths: DoublePriorityQueue<(NodeIndex, NodeIndex), u32>,
    remaining_targets: HashSet<NodeIndex>,
    current_search: Option<S>,
    regressions: Vec<RegressionPoint>,
    settings: Settings,
}

impl<S: PathAlgorithm + RegressionAlgorithm, E: Clone > RPA<S, E> {
    pub fn new(
        dvcs: Radag<String, E>,
        root: String,
        targets: Vec<String>,
        settings: Settings,
    ) -> Self {
        let pruned = prune(&dvcs, &vec![root.to_string()], &targets);

        if pruned.graph.node_count() == 0 {
            panic!("Graph is empty!");
        }

        let root_index = pruned
            .indexation
            .get(&root)
            .expect("Node for root hash missing!")
            .clone();

        let targets_index = HashSet::from_iter(
            targets
                .iter()
                .filter_map(|hash| pruned.indexation.get(hash))
                .map(|reference| reference.clone()),
        );

        let (annotated, shortest_path) = annotate_graph(
            pruned,
            root_index.clone(),
            &HashSet::from_iter(targets_index.iter().cloned()),
        );

        RPA {
            commits: annotated,
            remaining_targets: targets_index,
            shortest_paths: shortest_path,
            current_search: None,
            regressions: vec![],
            settings,
        }
    }
}

fn annotate_graph<E: Clone>(
    dvcs: Radag<String, E>,
    root: NodeIndex,
    targets: &HashSet<NodeIndex>,
) -> (
    Radag<RPANode, E>,
    DoublePriorityQueue<(NodeIndex, NodeIndex), u32>,
) {
    let mut shortest_path = DoublePriorityQueue::new();
    let mut distance = HashMap::new();
    let mut queue = VecDeque::new();

    queue.push_back(root);
    distance.insert(root, 0);

    while !queue.is_empty() {
        let current_index = queue.pop_front().unwrap();
        let current_distance = distance
            .get(&current_index)
            .expect("Distance of already visited node is missing!")
            .clone();

        for (_, child_index) in dvcs.graph.children(current_index.clone()).iter(&dvcs.graph) {
            match distance.get(&child_index) {
                Some(child_distance) => {
                    let shorter_distance = min(child_distance.clone(), current_distance + 1);
                    distance.insert(child_index, shorter_distance);

                    if targets.contains(&child_index) {
                        shortest_path.change_priority(&(root, child_index), shorter_distance);
                    }
                }
                None => {
                    distance.insert(child_index, current_distance + 1);
                    queue.push_back(child_index);
                    if targets.contains(&child_index) {
                        shortest_path.push((root, child_index), current_distance + 1);
                    }
                }
            };
        }
    }

    let mapped = dvcs.graph.map(
        |index, node| RPANode {
            result: if index == root {
                Some(TestResult::True)
            } else if targets.contains(&index) {
                Some(TestResult::False)
            } else {
                None
            },
            hash: node.to_string(),
        },
        |_, edge| edge.clone(),
    );

    (
        Radag {
            root: dvcs.root,
            graph: mapped,
            indexation: dvcs.indexation,
        },
        shortest_path,
    )
}

impl<S: PathAlgorithm + RegressionAlgorithm, E> RegressionAlgorithm for RPA<S, E> {
    fn add_result(&mut self, commit_hash: String, result: TestResult) {
        let index = self
            .commits
            .indexation
            .get(&commit_hash)
            .expect("Unknown commit hash!")
            .clone();

        let node = self
            .commits
            .graph
            .node_weight_mut(index)
            .expect("Invalid index!");

        node.result = Some(result.clone());

        if result == TestResult::True {
            self.update_paths(index);
        }

        let mut remove = false;
        if let Some(search) = self.current_search.as_mut() {
            search.add_result(commit_hash, result);
            if search.done() {
                remove = true;
                for reg in search.results() {
                    //TODO: When getting a candidate, we might be able reuse
                    //this candidate for some other targets. For that we would
                    //need to propagate the result down.
                    if let RegressionPoint::Point(assigned_point) = reg {
                        if self.settings.propagate {
                            self.propagate_results(
                                self.commits.indexation[&assigned_point.regression_point],
                            );
                        } else {
                            self.remaining_targets.remove(
                                &self
                                    .commits
                                    .indexation
                                    .get(&assigned_point.target)
                                    .expect("key missing"),
                            );
                            self.regressions
                                .push(RegressionPoint::Point(assigned_point));
                        }
                    }
                }
            }
        }

        if remove {
            self.current_search = None;
        }
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        //If there is no active search right now, we have to pick a new path and
        //start another search.
        if self.current_search.is_none() {
            let mut path_indices = None;

            while !self.shortest_paths.is_empty() {
                let (start, end) = self.shortest_paths.pop_min().unwrap().0;

                if self.remaining_targets.contains(&end) {
                    path_indices = Some((start, end));
                    break;
                }
            }

            let (start, end) = path_indices.expect("No relevant path was found!");

            let path = shortest_path(&self.commits.graph, start, end)
                .iter()
                .map(|index| {
                    self.commits
                        .graph
                        .node_weight(index.clone())
                        .unwrap()
                        .hash
                        .to_string()
                })
                .collect::<VecDeque<String>>();

            let search = S::new(path);
            println!(
                "RPA - Algorithm:\npicked new path\n{:?} to {:?}\n----\n",
                self.commits.graph.node_weight(start).unwrap().hash,
                self.commits.graph.node_weight(end).unwrap().hash
            );
            self.current_search = Some(search);
        }

        self.current_search.as_mut().unwrap().next_job(capacity)
    }

    fn done(&self) -> bool {
        self.remaining_targets.is_empty()
    }

    fn results(&self) -> Vec<RegressionPoint> {
        self.regressions.clone()
    }
}

impl<S: PathAlgorithm + RegressionAlgorithm, E> RPA<S,E> {
    fn update_paths(&mut self, origin: NodeIndex) {
        //TODO: The origin should never be a target. This is given by the fact
        //that this function will only be called, if the result is true and by
        //the assumption that all targets are false. But it would be better if we
        //explicitly handle this case. (e.g. throwing an error, warning or
        //returning a result value)
        let mut queue = VecDeque::<(NodeIndex, u32)>::new();
        let mut visited = HashSet::new();

        queue.push_back((origin, 0));
        visited.insert(origin);

        while !queue.is_empty() {
            let (index, distance) = queue.pop_front().unwrap();

            if self.remaining_targets.contains(&index) {
                self.shortest_paths.push((origin, index), distance);
            }

            for (_, next) in self.commits.graph.children(index).iter(&self.commits.graph) {
                if !visited.contains(&next) {
                    queue.push_back((next, distance + 1));
                    visited.insert(next);
                }
            }
        }
    }

    fn propagate_results(&mut self, regression: NodeIndex) {
        let mut queue = VecDeque::<NodeIndex>::new();
        let mut visited = HashSet::new();

        let regression_hash = self.node_from_index_unchecked(&regression).hash.to_string();

        queue.push_back(regression);
        visited.insert(regression);

        while !queue.is_empty() {
            let current = queue.pop_front().unwrap();

            if self.remaining_targets.contains(&current) {
                self.remaining_targets.remove(&current);
                let target_hash = self.node_from_index_unchecked(&current).hash.to_string();
                self.regressions
                    .push(RegressionPoint::Point(AssignedRegressionPoint {
                        target: target_hash,
                        regression_point: regression_hash.to_string(),
                    }));
            }

            for (_, next) in self
                .commits
                .graph
                .children(current)
                .iter(&self.commits.graph)
            {
                if !visited.contains(&next) {
                    queue.push_back(next);
                    visited.insert(next);
                }
            }
        }
    }

    fn node_from_index_unchecked(&mut self, index: &NodeIndex) -> &RPANode {
        self.commits
            .graph
            .node_weight(index.clone())
            .expect("node_from_index_unchecked() failed!")
    }
}
