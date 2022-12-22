use std::collections::{HashMap, VecDeque};

use priority_queue::DoublePriorityQueue;

use crate::regression::TestResult;

use super::{PathAlgorithm, RegressionAlgorithm, RegressionPoint};

pub struct LinearSearch {
    path: VecDeque<String>,
    results: Vec<Option<TestResult>>,
    index: usize,
    valid_nodes: DoublePriorityQueue<String, usize>,
    regression_point: Option<String>,
    job_await: HashMap<String, usize>,
    interrupts: Vec<String>,
}

impl PathAlgorithm for LinearSearch {
    fn new(path: VecDeque<String>) -> Self {
        if path.len() <= 1 {
            panic!("Path is too short for a regression point!");
        }

        let index = path.len() - 2;
        let mut results = vec![None; path.len()];
        results[0] = Some(TestResult::True);
        results[path.len() - 1] = Some(TestResult::False);
        let mut valid_nodes = DoublePriorityQueue::new();
        valid_nodes.push(path.front().unwrap().to_string(), 0);

        let mut lin = LinearSearch {
            path,
            results,
            index,
            valid_nodes,
            regression_point: None,
            job_await: HashMap::new(),
            interrupts: vec![],
        };

        if index == 0 {
            lin.regression_point = Some(lin.path.back().unwrap().to_string());
        }

        lin
    }
}

impl RegressionAlgorithm for LinearSearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        match self.job_await.remove(&commit) {
            Some(index) => {
                self.results[index] = Some(result.clone());
                if result == TestResult::True {
                    self.valid_nodes.push(commit, index);
                }
                // Traverse from the lowest valid node (highest index) to the next invalid node.
                // If every commit in between has a result, then we found the
                // lowest regression point
                let (_, i) = self.valid_nodes.peek_max().unwrap();
                for (ni, hash) in self
                    .path
                    .range((*i + 1)..self.path.len())
                    .enumerate()
                {
                    match &self.results[i + 1 + ni] {
                        Some(res) => {
                            if res == &TestResult::False {
                                let inters = self.job_await.keys().map(|a| a.to_string());
                                self.interrupts.extend(inters);
                                self.regression_point = Some(hash.to_string());
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
            None => eprintln!("Result for {} is not expected. Will ignore it!", commit),
        }
    }

    fn next_job(&mut self, _: u32, _: u32) -> super::AlgorithmResponse {
        //If there are still unchecked nodes and no regression point has been
        //found yet, then we want to continue with the next node on the path.
        if self.index > 0 && self.valid_nodes.len() < 2 {
            let commit = self.path.get(self.index).unwrap();
            self.job_await.insert(commit.to_string(), self.index);
            self.index -= 1;
            super::AlgorithmResponse::Job(commit.to_string())
        } else if self.job_await.is_empty() {
            super::AlgorithmResponse::InternalError("No jobs left!")
        } else {
            super::AlgorithmResponse::WaitForResult
        }
    }

    fn interrupts(&mut self) -> Vec<String> {
        let i = self.interrupts.clone();
        self.interrupts = vec![];
        i
    }

    fn done(&self) -> bool {
        self.regression_point.is_some()
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        let regression_point = self
            .regression_point
            .as_ref()
            .expect("No regression point!")
            .to_string();
        let target = self.path.back().unwrap().clone();
        vec![RegressionPoint {
            regression_point,
            target,
        }]
    }
}
