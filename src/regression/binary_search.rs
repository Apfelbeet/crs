use std::collections::VecDeque;

use crate::graph::center_of_path;

use super::{AssignedRegressionPoint, RegressionAlgorithm};

//TODO: Replace String with generics.

pub struct BinarySearch {
    path: VecDeque<String>,
    target: String,
    left: String,
    right: String,
    current: Option<String>,
    waiting: bool,
    regression: Option<String>,
}

impl BinarySearch {
    pub fn new(path: VecDeque<String>) -> Result<Self, String> {
        if path.len() <= 1 {
            return Err("Path is too short for a regression point!".to_string());
        }

        let left = path.front().unwrap().clone();
        let right = path.back().unwrap().clone();

        let mut bin = BinarySearch {
            path,
            target: right.to_string(),
            left,
            right,
            waiting: false,
            regression: None,
            current: None,
        };

        bin.step();

        Ok(bin)
    }

    fn step(&mut self) {
        //Find mid point of the new range
        match center_of_path(&self.path, &self.left, &self.right) {
            Ok(mid) => {
                if mid == self.left || mid == self.right {
                    //Done
                    self.regression = Some(self.right.to_string());
                } else {
                    self.current = Some(mid);
                }
            }
            Err(_) => panic!("Error at calculation mid point of path!"),
        };
    }
}

impl RegressionAlgorithm for BinarySearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.waiting = false;

        if commit != self.current.as_ref().unwrap().to_string() {
            panic!("Did not expected a result for this hash right now!");
        } else {
            //Adapt range
            match result {
                super::TestResult::True => {
                    self.left = commit;
                }
                super::TestResult::False => {
                    self.right = commit;
                }
                super::TestResult::Ignore => todo!(),
            };

            //New mid point
            self.step();
        }
    }

    fn next_job(&mut self, _capacity: u32) -> super::AlgorithmResponse {
        if self.waiting {
            super::AlgorithmResponse::WaitForResult
        } else {
            self.waiting = true;

            match self.current.as_ref() {
                Some(index) => super::AlgorithmResponse::Job(index.to_string()),
                None => super::AlgorithmResponse::InternalError("Miss next step!"),
            }
        }
    }

    fn done(&self) -> bool {
        self.regression.is_some()
    }

    fn results(&self) -> Vec<super::AssignedRegressionPoint> {
        match self.regression.as_ref() {
            Some(point) => vec![AssignedRegressionPoint {
                target: self.target.to_string(),
                regression_point: point.to_string(),
            }],
            None => vec![],
        }
    }
}
