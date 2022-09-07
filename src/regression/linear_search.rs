use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::regression::TestResult;

use super::{AssignedRegressionPoint, PathAlgorithm, RegressionAlgorithm, RegressionPoint};

pub struct LinearSearch {
    path: VecDeque<String>,
    index: usize,
    regressions: BTreeMap<usize, RegressionPoint>,
    job_await: HashMap<String, (String, usize)>,
}

impl PathAlgorithm for LinearSearch {
    fn new(path: VecDeque<String>) -> Self {
        if path.len() <= 1 {
            panic!("Path is too short for a regression point!");
        }

        let index = path.len() - 2;

        let mut lin = LinearSearch {
            path,
            index,
            regressions: BTreeMap::new(),
            job_await: HashMap::new(),
        };

        if index == 2 {
            lin.regressions.insert(
                0,
                RegressionPoint::Point(AssignedRegressionPoint {
                    regression_point: lin.path.back().unwrap().to_string(),
                    target: lin.path.back().unwrap().to_string(),
                }),
            );
        }

        lin
    }
}

impl RegressionAlgorithm for LinearSearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        match self.job_await.remove(&commit) {
            Some((last_commit, index)) => {
                if result == TestResult::True {
                    self.regressions.insert(
                        index,
                        RegressionPoint::Point(AssignedRegressionPoint {
                            regression_point: last_commit,
                            target: self.path.back().unwrap().clone(),
                        }),
                    );
                }
            }
            None => eprintln!("Result for {} is not expected. Will ignore it!", commit),
        }
    }

    fn next_job(&mut self, _: u32) -> super::AlgorithmResponse {
        if self.index > 0 {
            let commit = self.path.get(self.index).unwrap();
            self.job_await.insert(
                commit.to_string(),
                (
                    self.path.get(self.index + 1).unwrap().to_string(),
                    self.index,
                ),
            );
            self.index -= 1;
            super::AlgorithmResponse::Job(commit.to_string())
        } else if self.job_await.is_empty() {
            super::AlgorithmResponse::InternalError("No jobs left!")
        } else {
            super::AlgorithmResponse::WaitForResult
        }
    }

    fn done(&self) -> bool {
        self.job_await.is_empty() && self.regressions.len() > 0
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        let mut res = Vec::new();
        let mut last = None;
        for (k, v) in self.regressions.iter().rev() {
            if last.is_none() || (last.is_some() && last.unwrap() > &(k + 1)) {
                res.push(v.clone());
            }
            last = Some(k);
        }

        res
    }
}
