use std::{
    collections::{HashSet, VecDeque},
    marker::PhantomData,
};

use crate::log;
use daggy::{NodeIndex, Walker};
use priority_queue::PriorityQueue;

use crate::graph::Adag;

use super::{
    path_selection::PathSelection,
    rpa_extension::ExtendedSearch,
    rpa_util::{RPANode, Settings},
    AlgorithmResponse, PathAlgorithm, RegressionAlgorithm, RegressionPoint, TestResult,
};

pub struct RPA<P: PathSelection, S: PathAlgorithm + RegressionAlgorithm, E: Clone> {
    commits: Adag<RPANode, E>,
    ordering: PriorityQueue<(NodeIndex, NodeIndex), i32>,
    remaining_targets: HashSet<NodeIndex>,
    valid_nodes: HashSet<NodeIndex>,
    current_search: Option<S>,
    extended_search: Option<(RegressionPoint, ExtendedSearch<P, S, E>)>,
    regressions: Vec<RegressionPoint>,
    settings: Settings,
    interrupts: Vec<String>,
    log_path: Option<std::path::PathBuf>,
    counter: usize,
    _marker: PhantomData<P>,
}

impl<P: PathSelection, S: PathAlgorithm + RegressionAlgorithm, E: Clone + std::fmt::Debug>
    RPA<P, S, E>
{
    pub fn new(
        input_graph: Adag<String, E>,
        settings: Settings,
        log_path: Option<std::path::PathBuf>,
    ) -> Self {
        let targets_index = HashSet::from_iter(
            input_graph
                .targets
                .iter()
                .filter_map(|hash| input_graph.indexation.get(hash))
                .map(|reference| reference.clone()),
        );

        let sources_index = HashSet::from_iter(
            input_graph
                .sources
                .iter()
                .filter_map(|hash| input_graph.indexation.get(hash))
                .map(|reference| reference.clone()),
        );

        let annotated = annotate_graph(input_graph);

        eprintln!(
            "----
RPA initialized
{} Commits
----",
            annotated.graph.node_count()
        );

        let ordering = P::calculate_distances(&annotated, &targets_index, &sources_index);
        let exrpa_log_dir = log_path.map(|p| log::add_dir("exrpa", &p));
        if let Some(log_dir) = &exrpa_log_dir {
            log::create_file("summary", log_dir);
        }

        RPA {
            commits: annotated,
            remaining_targets: targets_index,
            valid_nodes: sources_index,
            ordering,
            current_search: None,
            extended_search: None,
            regressions: vec![],
            interrupts: vec![],
            settings,
            log_path: exrpa_log_dir,
            counter: 0,
            _marker: PhantomData,
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

impl<P: PathSelection, S: PathAlgorithm + RegressionAlgorithm, E: Clone> RegressionAlgorithm
    for RPA<P, S, E>
{
    fn add_result(&mut self, commit_hash: String, result: TestResult) {
        let index = self.commits.index(&commit_hash);

        let node = self
            .commits
            .graph
            .node_weight_mut(index)
            .expect("Invalid index!");

        if result == TestResult::True {
            self.valid_nodes.insert(index);
        }
        node.result = Some(result.clone());

        let mut reg_point = None;
        if let Some((_, ex_search)) = self.extended_search.as_mut() {
            ex_search.add_result(commit_hash, result.clone());
            self.interrupts
                .extend(ex_search.interrupts().iter().cloned());
        } else if let Some(search) = self.current_search.as_mut() {
            search.add_result(commit_hash, result.clone());
            self.interrupts.extend(search.interrupts().iter().cloned());
            if search.done() {
                if self.settings.extended_search {
                    let temp_reg = search.results()[0].clone();
                    self.counter += 1;
                    self.extended_search = Some((
                        temp_reg,
                        ExtendedSearch::new(
                            self.commits.clone(),
                            search.results()[0].clone(),
                            &self.valid_nodes,
                            self.log_path.clone(),
                            self.counter,
                        ),
                    ));
                } else {
                    reg_point = Some(search.results()[0].clone());
                }
                self.current_search = None;
            }
        };

        while self.extended_search.is_some() {
            let (ex_reg, ex_search) = self.extended_search.as_ref().unwrap();
            if ex_search.done() {
                let regs = ex_search.results();
                if regs.is_empty() {
                    reg_point = Some(ex_reg.clone());
                    self.extended_search = None;
                } else {
                    let new_reg = regs[0].clone();
                    self.counter += 1;
                    self.extended_search = Some((
                        new_reg.clone(),
                        ExtendedSearch::new(
                            self.commits.clone(),
                            new_reg.clone(),
                            &self.valid_nodes,
                            self.log_path.clone(),
                            self.counter,
                        ),
                    ));
                }
            } else {
                break;
            }
        }

        if let Some(reg) = reg_point {
            if self.settings.propagate {
                self.propagate_results(self.commits.index(&reg.regression_point));
            } else {
                self.remaining_targets
                    .remove(&self.commits.index(&reg.target));
                self.regressions.push(reg);
            }
        }

        if result == TestResult::True {
            self.ordering =
                P::calculate_distances(&self.commits, &self.remaining_targets, &self.valid_nodes);
        }
    }

    fn next_job(&mut self, capacity: u32, expected_capacity: u32) -> super::AlgorithmResponse {
        //If there is no active search right now, we have to pick a new path and
        //start another search.
        if self.current_search.is_none() && self.extended_search.is_none() {
            self.counter += 1;
            let mut path_indices = None;

            while !self.ordering.is_empty() {
                let (start, end) = self.ordering.pop().unwrap().0;

                if self.remaining_targets.contains(&end) {
                    path_indices = Some((start, end));
                    break;
                }
            }

            let (start, end) = path_indices.expect("No relevant path was found!");

            let path = P::extract_path(&self.commits, start, end)
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
            let source_hash = &self.commits.graph.node_weight(start).unwrap().hash;
            let target_hash = &self.commits.graph.node_weight(end).unwrap().hash;

            if let Some(log_path) = &self.log_path {
                log::write_to_file(&format!("Path Search ({}): From {} to {}. Path Length: {}\n", self.counter, source_hash, target_hash, len), &main_log_file(log_path));
                let path_file = log::create_file(&format!("{}_path", self.counter), &log_path);
                let path_string = path.clone().make_contiguous().join("\n");
                log::write_to_file(&path_string, &path_file);
            }

            let search = S::new(path);
            eprintln!(
                "RPA - Algorithm:
picked new path
{:?} to {:?}
length: {}
----",
                source_hash, target_hash, len,
            );

            self.current_search = Some(search);
        }

        if let Some(search) = self.current_search.as_mut() {
            search.next_job(capacity, expected_capacity)
        } else if let Some((_, ex_search)) = self.extended_search.as_mut() {
            ex_search.next_job(capacity, expected_capacity)
        } else {
            AlgorithmResponse::InternalError("No active search!")
        }
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

impl<P: PathSelection, S: PathAlgorithm + RegressionAlgorithm, E: Clone> RPA<P, S, E> {
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

pub fn main_log_file(path: &std::path::PathBuf) -> std::path::PathBuf {
    path.join("summary")
}
