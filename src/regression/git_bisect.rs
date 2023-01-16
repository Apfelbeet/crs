use std::collections::{HashMap, HashSet, VecDeque};

use daggy::NodeIndex;

use crate::graph::{associated_value_bisection, Adag};

use super::{RegressionAlgorithm, RegressionPoint, TestResult};

pub struct GitBisect {
    graph: Adag<String, ()>,
    results: HashMap<NodeIndex, TestResult>,
    valid_nodes: HashSet<NodeIndex>,
    ignored_nodes: HashSet<NodeIndex>,
    bisection_tree: BisectionTree,
    jobs: VecDeque<NodeIndex>,
    jobs_await: HashSet<NodeIndex>,
    target: NodeIndex,
    current_target: NodeIndex,
    _log_path: Option<std::path::PathBuf>,
}

type BisectionTree = Option<Box<Node>>;

fn new_bisection_tree(
    graph: &Adag<String, ()>,
    valid_nodes: &HashSet<NodeIndex>,
    ignored_nodes: &HashSet<NodeIndex>,
    target: NodeIndex,
) -> Option<Node> {
    let root = associated_value_bisection(graph, valid_nodes, ignored_nodes, target);
    root.map(Node::new)
}

struct Node {
    index: NodeIndex,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn new(index: NodeIndex) -> Self {
        Node {
            index,
            left: None,
            right: None,
        }
    }
}

impl From<Node> for Option<Box<Node>> {
    fn from(node: Node) -> Self {
        Some(Box::new(node))
    }
}

impl GitBisect {
    pub fn new(graph: Adag<String, ()>, log_path: Option<std::path::PathBuf>) -> Self {
        let sources_index = HashSet::from_iter(
            graph
                .sources
                .iter()
                .filter_map(|hash| graph.indexation.get(hash))
                .copied(),
        );

        let target_index = graph.index(&graph.targets[0]);
        let ignored_nodes = HashSet::new();
        let results = HashMap::new();
        let mut jobs = VecDeque::new();
        let tree = new_bisection_tree(&graph, &sources_index, &ignored_nodes, target_index);
        let tree = tree.map(Box::new);
        if let Some(node) = &tree {
            jobs.push_back(node.index);
        }

        eprintln!(
            "----\nBisect initialized\n{} Commits\n----",
            graph.graph.node_count()
        );

        GitBisect {
            graph,
            valid_nodes: sources_index,
            ignored_nodes,
            results,
            target: target_index,
            bisection_tree: tree,
            jobs_await: HashSet::new(),
            jobs,
            current_target: target_index,
            _log_path: log_path,
        }
    }
}

fn estimate_new_jobs(bisection_tree: &BisectionTree, results: &HashMap<NodeIndex, TestResult>) -> usize {
    let mut number_of_jobs = 0;
    let mut stack = Vec::<&Box<Node>>::new();

    if let Some(ref root_node) = bisection_tree {
        stack.push(root_node);
    }

    while let Some(node) = stack.pop() {
        let result = results.get(&node.index);
        
        if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
            match &node.right {
                Some(t) => stack.push(t),
                None => number_of_jobs += 1,
            };
        }

        if let Some(TestResult::False) | None = result {
            match &node.left {
                Some(l) => stack.push(l),
                None => number_of_jobs += 1,
            };
        }
    };

    number_of_jobs
}

fn generate_new_jobs(
    bisection_tree: &mut BisectionTree,
    results: &HashMap<NodeIndex, TestResult>,
    capacity: usize,
    graph: &Adag<String, ()>,
    target: NodeIndex,
    valid_nodes: &HashSet<NodeIndex>,
    ignored_nodes: &HashSet<NodeIndex>,
) -> VecDeque<NodeIndex> {
    
    let number_of_jobs = estimate_new_jobs(bisection_tree, results);

    if capacity < number_of_jobs {
        return VecDeque::default();
    }

    let mut jobs = HashSet::new();
    let mut stack = Vec::<(&mut Box<Node>, HashSet<NodeIndex>, NodeIndex)>::new();

    if let Some(root_node) = bisection_tree {
        stack.push((root_node, HashSet::new(), target));
    }

    while let Some((node, mut vs, t)) = stack.pop() {
        let result = results.get(&node.index);

        if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
            let mut vs2 = vs.clone();
            if let Some(TestResult::True) | None = result {
                vs2.insert(node.index);
            }

            match node.right {
                Some(ref mut r) => stack.push((r, vs2, t)),
                ref mut right @ None => {
                    vs2.extend(valid_nodes.iter());
                    let bisect = associated_value_bisection(graph, &vs2, ignored_nodes, t);

                    if let Some(bisect_point) = bisect {
                        jobs.insert(bisect_point);
                        *right = Node::new(bisect_point).into();
                    }
                }
            }
        }

        if let Some(TestResult::False) | None = result {
            match node.left {
                Some(ref mut l) => stack.push((l, vs, node.index)),
                ref mut left @ None => {
                    vs.extend(valid_nodes.iter());
                    let bisect = associated_value_bisection(graph, &vs, ignored_nodes, node.index);

                    if let Some(bisect_point) = bisect {
                        jobs.insert(bisect_point);
                        *left = Node::new(bisect_point).into();
                    }
                }
            }
        }
    }

    VecDeque::from_iter(jobs.into_iter())
}

impl RegressionAlgorithm for GitBisect {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.results.insert(self.graph.index(&commit), result);

        let mut current = self.bisection_tree.take();
        while current.is_some() {
            let c = current.as_ref().unwrap();
            match self.results.get(&c.index) {
                Some(TestResult::True) => {
                    self.valid_nodes.insert(c.index);
                    current = current.unwrap().right;
                }
                Some(TestResult::False) => {
                    self.current_target = c.index;
                    current = current.unwrap().left;
                }
                Some(TestResult::Ignore) => {
                    self.ignored_nodes.insert(c.index);
                    current = current.unwrap().right;
                }
                None => break,
            }
        }

        self.bisection_tree = current;
        //todo: remove all jobs that are no longer relevant. (+ add them to the interrupt list)
        if self.bisection_tree.is_none() {
            self.bisection_tree = new_bisection_tree(
                &self.graph,
                &self.valid_nodes,
                &self.ignored_nodes,
                self.current_target,
            )
            .map(Box::new);
            self.jobs = VecDeque::new();
            if let Some(node) = &self.bisection_tree {
                self.jobs.push_back(node.index);
            }
        }
    }

    fn next_job(&mut self, capacity: u32, _expected_capacity: u32) -> super::AlgorithmResponse {
        if self.jobs.is_empty() {
            //We first calculate the number of jobs we will have in the next step. If the number is higher than our capacity,
            //we will not continue and will not calculate the bisection point (because this operation is to expensive).
            //There are some cases, where number_of_jobs is higher than the actual number of bisection points we will find.
            //So sometimes we will still wait, although we could already continue.
            //If we continue, we calculate the bisection points and will augment the tree with them.
            self.jobs = generate_new_jobs(
                &mut self.bisection_tree,
                &self.results,
                capacity as usize,
                &self.graph,
                self.current_target,
                &self.valid_nodes,
                &self.ignored_nodes,
            );
        }
        //println!("current jobs {:?}", self.jobs.iter().map(|x| self.graph.node_from_index(*x)).collect::<Vec<_>>());

        match (self.jobs.pop_front(), self.jobs_await.is_empty()) {
            (Some(job), _) => {
                self.jobs_await.insert(job);
                let hash = self.graph.node_from_index(job);
                super::AlgorithmResponse::Job(hash)
            }
            (None, false) => super::AlgorithmResponse::WaitForResult,
            (None, true) => super::AlgorithmResponse::InternalError("Bisect: Search error!"),
        }
    }

    fn interrupts(&mut self) -> Vec<String> {
        vec![]
    }

    fn done(&self) -> bool {
        self.bisection_tree.is_none()
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        vec![RegressionPoint {
            target: self.graph.node_from_index(self.target),
            regression_point: self.graph.node_from_index(self.current_target),
        }]
    }
}
