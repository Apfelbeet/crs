use priority_queue::DoublePriorityQueue;
use std::{
    collections::{HashSet, VecDeque},
};

use daggy::{NodeIndex, Walker};

use crate::graph::{shortest_path, Adag};

use super::{PathAlgorithm, RegressionAlgorithm, RegressionPoint, TestResult};

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
    commits: Adag<RPANode, E>,
    shortest_paths: DoublePriorityQueue<(NodeIndex, NodeIndex), u32>,
    remaining_targets: HashSet<NodeIndex>,
    current_search: Option<S>,
    regressions: Vec<RegressionPoint>,
    settings: Settings,
    interrupts: Vec<String>,
}

impl<S: PathAlgorithm + RegressionAlgorithm, E: Clone + std::fmt::Debug> RPA<S, E> {
    pub fn new(
        input_graph: Adag<String, E>,
        settings: Settings,
    ) -> Self {
        let targets_index = HashSet::from_iter(
            input_graph.targets
                .iter()
                .filter_map(|hash| input_graph.indexation.get(hash))
                .map(|reference| reference.clone()),
        );

        let annotated = annotate_graph(input_graph);
        let shortest_path = annotated.calculate_distances();

        eprintln!(
            "----
RPA initialized
{} Commits
----",
            annotated.graph.node_count()
        );

        RPA {
            commits: annotated,
            remaining_targets: targets_index,
            shortest_paths: shortest_path,
            current_search: None,
            regressions: vec![],
            interrupts: vec![],
            settings,
        }
    }
}

fn annotate_graph<E: Clone>(dvcs: Adag<String, E>) -> Adag<RPANode, E> {
    let mapped = dvcs.graph.map(
        |_, hash| {
            let is_source = dvcs.sources.contains(hash);
            let is_target = dvcs.targets.contains(hash);

            if is_source && is_target {
                panic!("{} is a source as well as a target!", hash);
            }

            let result = if is_source {
                Some(TestResult::True)
            } else if is_target {
                Some(TestResult::False)
            } else {
                None
            };

            RPANode {
                hash: hash.to_string(),
                result,
            }
        },
        |_, edge| edge.clone(),
    );

    Adag {
        sources: dvcs.sources,
        targets: dvcs.targets,
        graph: mapped,
        indexation: dvcs.indexation,
    }
}



impl<S: PathAlgorithm + RegressionAlgorithm, E> RegressionAlgorithm for RPA<S, E> {
    fn add_result(&mut self, commit_hash: String, result: TestResult) {
        let index = self.commits.index(&commit_hash);

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
            self.interrupts.extend(search.interrupts().iter().cloned());
            if search.done() {
                remove = true;
                for reg in search.results() {
                    if self.settings.propagate {
                        self.propagate_results(self.commits.index(&reg.regression_point));
                    } else {
                        self.remaining_targets
                            .remove(&self.commits.index(&reg.target));
                        self.regressions.push(reg);
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

            let len = path.len();
            let search = S::new(path);
            eprintln!(
                "RPA - Algorithm:
picked new path
{:?} to {:?}
length: {}
----",
                self.commits.graph.node_weight(start).unwrap().hash,
                self.commits.graph.node_weight(end).unwrap().hash,
                len,
            );
            self.current_search = Some(search);
        }

        self.current_search.as_mut().unwrap().next_job(capacity)
    }

    fn interrupts(&mut self) -> Vec<String> {
        let i = self.interrupts.clone();
        self.interrupts = vec![];
        i
    }

    fn done(&self) -> bool {
        self.remaining_targets.is_empty()
    }

    fn results(&self) -> Vec<RegressionPoint> {
        self.regressions.clone()
    }
}

impl<S: PathAlgorithm + RegressionAlgorithm, E> RPA<S, E> {
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

        let regression_hash = self.commits.node_from_index(regression).hash;

        queue.push_back(regression);
        visited.insert(regression);

        while !queue.is_empty() {
            let current = queue.pop_front().unwrap();

            if self.remaining_targets.contains(&current) {
                self.remaining_targets.remove(&current);
                let target_hash = self.node_from_index_unchecked(&current).hash.to_string();
                self.regressions.push(RegressionPoint {
                    target: target_hash,
                    regression_point: regression_hash.to_string(),
                });
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
