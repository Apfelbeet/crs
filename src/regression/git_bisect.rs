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
    original_target: NodeIndex,
    current_target: NodeIndex,
    _log_path: Option<std::path::PathBuf>,
}

type BisectionTree = Child<Node>;

fn new_root(
    graph: &Adag<String, ()>,
    results: &HashMap<NodeIndex, TestResult>,
    valid_nodes: &mut HashSet<NodeIndex>,
    ignored_nodes: &mut HashSet<NodeIndex>,
    target: &mut NodeIndex,
) -> Child<Node> {
    let mut fictive_valids = valid_nodes.clone(); //fixme: bad! Might not work correctly

    //Given the current results, we can calculate a new root
    //That root might already have a result, so we have to repeat this process until
    //we find a unevaluated root or we know that there is nothing left.
    let root = loop {
        let node = associated_value_bisection(graph, valid_nodes, &fictive_valids, *target);
        match node {
            None => break node,
            Some(n) => match results.get(&n) {
                Some(TestResult::True) => {
                    valid_nodes.insert(n);
                    fictive_valids.insert(n);
                }
                Some(TestResult::False) => {
                    *target = n;
                }
                Some(TestResult::Ignore) => {
                    fictive_valids.insert(n);
                    ignored_nodes.insert(n);
                }
                None => break node,
            },
        }
    };

    root.into()
}

struct Node {
    index: NodeIndex,
    left: Child<Node>,
    right: Child<Node>,
}
enum Child<T> {
    Next(Box<T>),
    Unknown,
    End,
}

impl<T> Child<T> {
    fn unwrap(self) -> Box<T> {
        match self {
            Child::Next(val) => val,
            _ => {
                panic!("called `Child::unwrap()` on a empty value");
            }
        }
    }
}

impl Node {
    fn new(index: NodeIndex) -> Self {
        Node {
            index,
            left: Child::Unknown,
            right: Child::Unknown,
        }
    }
}

impl From<Node> for Child<Node> {
    fn from(node: Node) -> Self {
        Child::Next(Box::new(node))
    }
}

impl From<Option<NodeIndex>> for Child<Node> {
    fn from(op: Option<NodeIndex>) -> Self {
        match op {
            Some(index) => Node::new(index).into(),
            None => Child::End,
        }
    }
}

impl GitBisect {
    pub fn new(graph: Adag<String, ()>, log_path: Option<std::path::PathBuf>) -> Self {
        let mut sources_index = HashSet::from_iter(
            graph
                .sources
                .iter()
                .filter_map(|hash| graph.indexation.get(hash))
                .copied(),
        );

        let mut target_index = graph.index(&graph.targets[0]);
        let mut ignored_nodes = HashSet::new();
        let results = HashMap::new();
        let tree = new_root(
            &graph,
            &results,
            &mut sources_index,
            &mut ignored_nodes,
            &mut target_index,
        );

        eprintln!(
            "----\nBisect initialized\n{} Commits\n----",
            graph.graph.node_count()
        );

        GitBisect {
            graph,
            valid_nodes: sources_index,
            ignored_nodes,
            results,
            original_target: target_index,
            bisection_tree: tree,
            jobs_await: HashSet::new(),
            jobs: VecDeque::new(),
            current_target: target_index,
            _log_path: log_path,
        }
    }

    fn extend_speculation_tree(&mut self) -> bool {
        let mut changed = false;
        match self.bisection_tree {
            Child::End => {}

            Child::Unknown => {
                changed = true;
                self.bisection_tree = new_root(
                    &self.graph,
                    &self.results,
                    &mut self.valid_nodes,
                    &mut self.ignored_nodes,
                    &mut self.current_target,
                );
            }

            Child::Next(ref mut root) => {
                let mut q =
                    VecDeque::<(&mut Box<Node>, usize, HashSet<NodeIndex>, NodeIndex)>::new();
                let mut limit = None;
                q.push_back((root, 0, HashSet::new(), self.current_target));

                while let Some((node, depth, mut vs, t)) = q.pop_front() {
                    if let Some(lim) = limit {
                        if lim < depth {
                            break;
                        }
                    }

                    let result = self.results.get(&node.index);

                    if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
                        let mut vs2 = vs.clone();
                        if let Some(TestResult::True) | None = result {
                            vs2.insert(node.index);
                        }

                        match node.right {
                            Child::Next(ref mut r) => q.push_back((r, depth + 1, vs2, t)),

                            ref mut right @ Child::Unknown => {
                                limit.get_or_insert(depth);

                                vs2.insert(node.index); //second time for the ignore case.
                                vs2.extend(self.valid_nodes.iter());
                                let bisect = associated_value_bisection(
                                    &self.graph,
                                    &vs2,
                                    &self.ignored_nodes,
                                    t,
                                );

                                changed = true;
                                *right = bisect.into();
                            }

                            Child::End => {}
                        }
                    }

                    if let Some(TestResult::False) | None = result {
                        match node.left {
                            Child::Next(ref mut l) => q.push_back((l, depth + 1, vs.clone(), node.index)),

                            ref mut left @ Child::Unknown => {
                                limit.get_or_insert(depth);

                                vs.extend(self.valid_nodes.iter());
                                let bisect = associated_value_bisection(
                                    &self.graph,
                                    &vs,
                                    &self.ignored_nodes,
                                    node.index,
                                );

                                changed = true;
                                *left = bisect.into();
                            }

                            Child::End => {}
                        }
                    }
                }
            }
        }
        changed
    }

    fn extract_jobs(&self) -> HashSet<NodeIndex> {
        if let Child::Next(root) = &self.bisection_tree {
            let mut jobs = HashSet::new();
            let mut limit = None;
            let mut q: VecDeque<(&Box<Node>, usize)> = VecDeque::new();
            q.push_back((root, 0));

            while let Some((node, depth)) = q.pop_front() {
                if let Some(lim) = limit {
                    if lim < depth {
                        break;
                    }
                }

                if self.is_unprocessed(node.index) {
                    limit.get_or_insert(depth);
                    jobs.insert(node.index);
                    continue;
                }

                let result = self.results.get(&node.index);
                if let Some(TestResult::True) | Some(TestResult::Ignore) | None = result {
                    if let Child::Next(ref r) = node.right {
                        q.push_back((r, depth + 1));
                    }
                }

                if let Some(TestResult::False) | None = result {
                    if let Child::Next(ref l) = node.left {
                        q.push_back((l, depth + 1));
                    }
                }
            }
            jobs
        } else {
            HashSet::default()
        }
    }

    fn is_unprocessed(&self, index: NodeIndex) -> bool {
        !self.results.contains_key(&index) && !self.jobs_await.contains(&index)
    }
}

impl RegressionAlgorithm for GitBisect {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.results.insert(self.graph.index(&commit), result);

        //temporarily move bisection_tree into current.
        //Child::Unknown is only a placeholder, we'll override it later again.
        let mut current = std::mem::replace(&mut self.bisection_tree, Child::Unknown);
        while let Child::Next(ref c) = current {
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
        self.jobs = VecDeque::new();
        //todo: (+ add them to the interrupt list)

        if let Child::Unknown = self.bisection_tree {
            self.bisection_tree = new_root(
                &self.graph,
                &self.results,
                &mut self.valid_nodes,
                &mut self.ignored_nodes,
                &mut self.current_target,
            )
            .into();
        }
    }

    fn next_job(&mut self, capacity: u32, _expected_capacity: u32) -> super::AlgorithmResponse {
        if self.jobs.is_empty() {
            //First look if there are any jobs left in the tree.
            let mut jobs = VecDeque::from_iter(self.extract_jobs().into_iter());

            //Otherwise try to extend the tree and collect the new jobs
            while jobs.is_empty() {
                let changed = self.extend_speculation_tree();
                jobs = VecDeque::from_iter(self.extract_jobs().into_iter());

                if !changed {
                    break;
                }
            }

            if (capacity as usize) >= jobs.len() {
                self.jobs = jobs;
            }
        }

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
        if let Child::End = self.bisection_tree {
            true
        } else {
            false
        }
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        vec![RegressionPoint {
            target: self.graph.node_from_index(self.original_target),
            regression_point: self.graph.node_from_index(self.current_target),
        }]
    }
}
