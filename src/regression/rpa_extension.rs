use std::collections::{HashSet, VecDeque};

use daggy::{NodeIndex, Walker};

use crate::graph::{bfs_valid, shortest_path, Adag};

use super::{
    extended_rpa::RPANode, AlgorithmResponse, PathAlgorithm, RegressionAlgorithm, RegressionPoint,
    TestResult,
};

pub struct ExtendedSearch<S: PathAlgorithm + RegressionAlgorithm, E: Clone> {
    parents: Option<ParentsSearch>,
    sub: Option<S>,
    interrupts: Vec<String>,
    regression: Option<String>,
    target: String,
    graph: Adag<RPANode, E>,
}

pub struct ParentsSearch {
    parents: VecDeque<String>,
    parents_await: HashSet<String>,
}

impl<S: PathAlgorithm + RegressionAlgorithm, E: Clone> ExtendedSearch<S, E> {
    pub fn new(adag: Adag<RPANode, E>, reg: RegressionPoint) -> Self {
        let mut q = VecDeque::<NodeIndex>::new();
        let mut queued = HashSet::<NodeIndex>::new();

        let mut p: VecDeque<String> = VecDeque::new();
        let mut cached_parent = None;

        let reg_index = adag.index(&reg.regression_point);
        q.push_back(reg_index);
        queued.insert(reg_index);

        while !q.is_empty() {
            let current_index = q.pop_front().unwrap();
            let parents = adag.graph.parents(current_index);

            for (_, parent_index) in parents.iter(&adag.graph) {
                let node = adag.node_from_index(parent_index);

                match node.result {
                    Some(result) => match result {
                        super::TestResult::False => {
                            cached_parent = Some(parent_index);
                            break;
                        }
                        super::TestResult::Ignore => {
                            if queued.insert(parent_index) {
                                q.push_back(parent_index);
                            }
                        }
                        super::TestResult::True => {}
                    },
                    None => p.push_back(node.hash),
                }
            }

            if cached_parent.is_some() {
                break;
            }
        }

        if let Some(cp_index) = cached_parent {
            let cp = adag.hash_from_index(cp_index);
            let search = create_sub::<S, E>(&adag, cp);
            let mut search = ExtendedSearch {
                parents: None,
                sub: Some(search),
                interrupts: vec![],
                regression: None,
                target: reg.target,
                graph: adag,
            };

            search.check_sub_done();
            search
        } else if p.is_empty() {
            ExtendedSearch {
                parents: None,
                sub: None,
                interrupts: vec![],
                regression: None,
                target: reg.target,
                graph: adag,
            }
        } else {
            eprintln!(
                "ExRPA - Algorithm:
check parents
parents: {:?}
----",
                p
            );

            ExtendedSearch {
                parents: Some(ParentsSearch {
                    parents: p,
                    parents_await: HashSet::new(),
                }),
                sub: None,
                interrupts: vec![],
                regression: None,
                target: reg.target,
                graph: adag,
            }
        }
    }

    fn check_sub_done(&mut self) {
        if self.sub.is_some() && self.sub.as_ref().unwrap().done() {
            let reg: RegressionPoint = self.sub.as_ref().unwrap().results()[0].clone();
            self.regression = Some(reg.regression_point);
            self.sub = None;
        }
    }
}

impl<S: PathAlgorithm + RegressionAlgorithm, E: Clone> RegressionAlgorithm
    for ExtendedSearch<S, E>
{
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        let index = self.graph.index(&commit);
        let node = self.graph.graph.node_weight_mut(index).unwrap();
        node.result = Some(result.clone());

        let mut new_target = None;
        if let Some(ps) = self.parents.as_mut() {
            if ps.parents_await.remove(&commit) {
                let mut q = VecDeque::new();
                let mut vis = HashSet::new();

                q.push_back(commit);
                vis.extend(ps.parents.clone());
                vis.extend(ps.parents_await.clone());

                while !q.is_empty() {
                    let current = q.pop_front().unwrap();
                    let current_index = self.graph.index(&current);

                    let res = self.graph.node(&current).result;
                    match res {
                        Some(r) => match r {
                            TestResult::True => {}
                            TestResult::False => {
                                new_target = Some(current);
                                self.interrupts.extend(ps.parents_await.iter().cloned());
                                break;
                            }
                            TestResult::Ignore => {
                                let parents = self.graph.graph.parents(current_index);
                                for (_, parent_index) in parents.iter(&self.graph.graph) {
                                    let parent = self.graph.node_from_index(parent_index).hash;
                                    if vis.insert(parent.to_string()) {
                                        q.push_back(parent)
                                    }
                                }
                            }
                        },
                        None => {
                            ps.parents.push_back(current);
                        }
                    }
                }
            }
        } else if let Some(sub) = self.sub.as_mut() {
            sub.add_result(commit, result);
            self.interrupts.extend(sub.interrupts().iter().cloned());
            self.check_sub_done();
        }

        //If there is no invalid parent, we want to stop the extended search.
        let mut remove_parent = false;
        if let Some(ps) = self.parents.as_ref() {
            remove_parent = ps.parents.is_empty() && ps.parents_await.is_empty();
        }

        if remove_parent {
            self.parents = None;
        }

        //When we found a invalid parent, we start with the second phase.
        if let Some(nt) = new_target {
            let search = create_sub::<S, E>(&self.graph, nt);
            self.parents = None;
            self.sub = Some(search);
            self.check_sub_done();
        }
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        if let Some(p) = &mut self.parents {
            match p.parents.pop_front() {
                Some(hash) => {
                    p.parents_await.insert(hash.clone());
                    AlgorithmResponse::Job(hash)
                }
                None => {
                    if p.parents_await.is_empty() {
                        AlgorithmResponse::InternalError("Unexpected request!")
                    } else {
                        AlgorithmResponse::WaitForResult
                    }
                }
            }
        } else if let Some(sub) = &mut self.sub {
            sub.next_job(capacity)
        } else {
            AlgorithmResponse::InternalError("Unexpected request!")
        }
    }

    fn interrupts(&mut self) -> Vec<String> {
        let i = self.interrupts.clone();
        self.interrupts = vec![];
        i
    }

    fn done(&self) -> bool {
        self.parents.is_none() && self.sub.is_none()
    }

    fn results(&self) -> Vec<RegressionPoint> {
        match self.regression.as_ref() {
            Some(r) => vec![RegressionPoint {
                regression_point: r.clone(),
                target: self.target.clone(),
            }],
            None => vec![],
        }
    }
}

fn create_sub<S: PathAlgorithm, E: Clone>(graph: &Adag<RPANode, E>, target: String) -> S {
    let target_index = graph.index(&target);
    let valid_index = bfs_valid(graph, target_index);
    let path = shortest_path(&graph.graph, valid_index, target_index);
    let hash_path = path
        .iter()
        .map(|i| graph.node_from_index(*i).hash)
        .collect::<VecDeque<String>>();
    let path_len = hash_path.len();
    let search = S::new(hash_path);

    eprintln!(
        "ExRPA - Algorithm:
picked extended path
\"{}\" to \"{}\"
lenght: {}
----",
        graph.hash_from_index(valid_index),
        graph.hash_from_index(target_index),
        path_len
    );

    search
}
