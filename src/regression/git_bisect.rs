use std::collections::{HashMap, HashSet, VecDeque};

use daggy::NodeIndex;

use crate::graph::{associated_value_bisection, Adag};

use super::{RegressionAlgorithm, RegressionPoint, TestResult};

pub struct GitBisect {
    graph: Adag<String, ()>,
    valid_nodes: HashSet<NodeIndex>,
    ignored_nodes: HashSet<NodeIndex>,
    results: HashMap<NodeIndex, TestResult>,
    target: String,
    current_target: NodeIndex,
    bisection_depth: usize,
    _log_path: Option<std::path::PathBuf>,
    step: Option<Step>,
}

type JobTree = HashMap<NodeIndex, (Option<NodeIndex>, Option<NodeIndex>)>;

struct Step {
    pub job_queue: VecDeque<String>,
    pub job_await: HashSet<String>,
    pub job_tree: (JobTree, NodeIndex),
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

        eprintln!(
            "----
Bisect initialized
{} Commits
----",
            graph.graph.node_count()
        );

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
    let mut tree = HashMap::new();
    let root = next_point(graph, valid_nodes.clone(), ignored_nodes, target, depth, &mut points, &mut tree);

    if points.is_empty() {
        None
    } else {
        let hashes = points
            .into_iter()
            .map(|index| graph.node_from_index(index))
            .collect::<VecDeque<_>>();

        Some(Step {
            job_await: HashSet::new(),
            job_queue: hashes,
            job_tree: (tree, root.unwrap()),

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
    tree: &mut JobTree,
) -> Option<NodeIndex> {
    let bisection_point = associated_value_bisection(graph, &valid_nodes, ignored_nodes, target);
    if let Some(point) = bisection_point {
        let mut ex_valid = valid_nodes.clone();
        ex_valid.insert(point);

        let left = if depth > 1 {
            next_point(graph, valid_nodes, ignored_nodes, point, depth - 1, points, tree)
        } else {
            None
        };

        points.push_back(point);

        let right = if depth > 1 {
            next_point(graph, ex_valid, ignored_nodes, target, depth - 1, points, tree)
        } else {
            None
        };

        tree.insert(point, (left, right));
    }
    bisection_point
}

impl RegressionAlgorithm for GitBisect {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.results.insert(self.graph.index(&commit), result);

        if let Some(step) = self.step.as_mut() {
            step.job_await.remove(&commit);

            if step.job_await.is_empty() && step.job_queue.is_empty() {
                let mut current_node = Some(step.job_tree.1);
                while let Some(node_pointer) = current_node {
                    match self.results[&node_pointer] {
                        TestResult::True => {
                            self.valid_nodes.insert(node_pointer);
                            if let Some((_, r)) = step.job_tree.0.get(&node_pointer) {
                                current_node = *r;
                            }
                        },
                        TestResult::False => {
                            self.current_target = node_pointer;
                            if let Some((l, _)) = step.job_tree.0.get(&node_pointer) {
                                current_node = *l;
                            }
                        },
                        TestResult::Ignore => {
                            self.ignored_nodes.insert(node_pointer);
                            if let Some((l, _)) = step.job_tree.0.get(&node_pointer) {
                                current_node = *l;
                            }
                        },
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
