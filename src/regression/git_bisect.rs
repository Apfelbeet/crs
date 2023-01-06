use std::collections::{HashMap, HashSet, VecDeque};

use daggy::NodeIndex;

use crate::graph::{associated_value_bisection, Adag};

use super::{RegressionAlgorithm, RegressionPoint, TestResult};

pub struct GitBisect {
    graph: Adag<String, ()>,
    valid_nodes: HashSet<NodeIndex>,
    ignored_nodes: HashSet<NodeIndex>,
    results: HashMap<String, TestResult>,
    target: String,
    current_target: NodeIndex,
    bisection_depth: usize,
    _log_path: Option<std::path::PathBuf>,
    step: Option<Step>,
}

struct Step {
    pub job_queue: VecDeque<String>,
    pub job_await: HashSet<String>,
    pub jobs: VecDeque<String>,
}

impl GitBisect {
    pub fn new(
        graph: Adag<String, ()>,
        capacity: usize,
        log_path: Option<std::path::PathBuf>,
    ) -> Self {
        let sources_index = HashSet::from_iter(
            graph
                .sources
                .iter()
                .filter_map(|hash| graph.indexation.get(hash))
                .copied(),
        );

        let target = graph.targets[0].clone();
        let target_index = graph.index(&graph.targets[0]);
        let bisection_depth = ((capacity + 1) as f64).log2() as usize;
        let ignored_nodes = HashSet::new();
        let step = next_step(&graph, &sources_index, &ignored_nodes, target_index, bisection_depth);

        GitBisect {
            graph,
            valid_nodes: sources_index,
            ignored_nodes,
            results: HashMap::new(),
            target,
            bisection_depth,
            current_target: target_index,
            _log_path: log_path,
            step,
        }
    }
}

fn next_step(
    graph: &Adag<String, ()>,
    valid_nodes: &HashSet<NodeIndex>,
    ignored_nodes: &HashSet<NodeIndex>,
    target: NodeIndex,
    depth: usize,
) -> Option<Step> {
    let mut points = VecDeque::<NodeIndex>::new();
    next_point(graph, valid_nodes.clone(), ignored_nodes, target, depth, &mut points);

    if points.is_empty() {
        None
    } else {
        let hashes = points
            .into_iter()
            .map(|index| graph.node_from_index(index))
            .collect::<VecDeque<_>>();

        Some(Step {
            job_await: HashSet::new(),
            job_queue: hashes.clone(),
            jobs: hashes,
        })
    }
}

fn next_point(
    graph: &Adag<String, ()>,
    valid_nodes: HashSet<NodeIndex>,
    ignored_nodes: &HashSet<NodeIndex>,
    target: NodeIndex,
    depth: usize,
    points: &mut VecDeque<NodeIndex>,
) {
    let bisection_point = associated_value_bisection(graph, &valid_nodes, ignored_nodes, target);
    if let Some(point) = bisection_point {
        let mut ex_valid = valid_nodes.clone();
        ex_valid.insert(point);

        if depth > 1 {
            next_point(graph, valid_nodes, ignored_nodes, point, depth - 1, points);
        }

        points.push_back(point);

        if depth > 1 {
            next_point(graph, ex_valid, ignored_nodes, target, depth - 1, points);
        }
    }
}

impl RegressionAlgorithm for GitBisect {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.results.insert(commit.clone(), result.clone());
        match result {
            super::TestResult::True => {
                self.valid_nodes.insert(self.graph.index(&commit));
            }
            super::TestResult::False => {}
            super::TestResult::Ignore => {
                self.ignored_nodes.insert(self.graph.index(&commit));
            },
        }

        if let Some(step) = self.step.as_mut() {
            step.job_await.remove(&commit);

            if step.job_await.is_empty() && step.job_queue.is_empty() {
                for job in step.jobs.iter().rev() {
                    match self.results[job] {
                        TestResult::True => break,
                        TestResult::False => self.current_target = self.graph.index(job),
                        TestResult::Ignore => panic!("Bisect: Untestable is unsupported!"),
                    }
                }
                self.step = next_step(&self.graph, &self.valid_nodes, &self.ignored_nodes, self.current_target, self.bisection_depth);
            }
        }
    }

    fn next_job(&mut self, _capacity: u32, _expected_capacity: u32) -> super::AlgorithmResponse {
        match self.step.as_mut() {
            Some(step) => match (step.job_queue.pop_front(), step.job_await.is_empty()) {
                (Some(hash), _) => {
                    step.job_await.insert(hash.clone());
                    super::AlgorithmResponse::Job(hash)
                },
                (None, false) => super::AlgorithmResponse::WaitForResult,
                (None, true) => super::AlgorithmResponse::InternalError("Bisect: Search error!"),
            },
            None => super::AlgorithmResponse::InternalError("Bisect: Search has already finished!"),
        }
    }

    fn interrupts(&mut self) -> Vec<String> {
        vec![]
    }

    fn done(&self) -> bool {
        self.step.is_none()
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        vec![RegressionPoint {
            target: self.target.clone(),
            regression_point: self.graph.node_from_index(self.current_target),
        }]
    }
}
