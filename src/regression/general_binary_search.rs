use std::collections::{HashMap, HashSet, VecDeque};

use priority_queue::DoublePriorityQueue;

use crate::graph::length_of_path;

use super::{TestResult, RegressionPoint};

pub type SampleFunction = fn(&VecDeque<String>, &String, &String, usize, usize) -> Result<VecDeque<String>, ()>;
pub struct GeneralBinarySearch {
    pub path: VecDeque<String>,
    pub target: String,
    pub left: String,
    pub right: String,
    pub step: Option<Step>,
    pub regression: Option<String>,
    pub results: HashMap<String, TestResult>,
}

pub struct Step {
    pub job_queue: VecDeque<String>,
    pub job_await: HashSet<String>,
    pub jobs: VecDeque<String>,
    pub valid_nodes: DoublePriorityQueue<String, usize>,
}

impl GeneralBinarySearch {
    pub fn new(path: VecDeque<String>) -> Self {
        if path.len() <= 1 {
            panic!("Path is too short for a regression point!");
        }

        let left = path.front().unwrap().clone();
        let right = path.back().unwrap().clone();

        let mut results = HashMap::new();
        results.insert(left.clone(), TestResult::True);
        results.insert(right.clone(), TestResult::False);

        let mut bin = GeneralBinarySearch {
            path,
            target: right.to_string(),
            left,
            right,
            regression: None,
            step: None,
            results,
        };

        bin.check_done();

        bin
    }

    pub fn add_result(&mut self, commit: String, result: TestResult) {
        if self.step.is_none() {
            eprintln!("Result for {} is not expected. Will ignore it!", commit);
        } else {
            let step = self.step.as_mut().unwrap();
            self.results.insert(commit.clone(), result.clone());
            if step.job_await.remove(&commit) {
                if result == TestResult::True {
                    for (i, h) in step.jobs.iter().enumerate() {
                        if h.to_string() == commit {
                            step.valid_nodes.push(commit.clone(), i + 1);
                            break;
                        }
                    }
                }

                // Traverse from the lowest valid job (highest index) to the
                // next invalid job. If every job in between has a result,
                // then we found the lowest regression. (We artificially add the
                // left and right boarder as a job to this process.)
                let jobs_len = step.jobs.len();
                let (lowest_valid, i) = self
                    .step
                    .as_ref()
                    .unwrap()
                    .valid_nodes
                    .peek_max()
                    .unwrap_or((&self.left, &0));

                let mut regression = None;

                let mut incomplete = false;
                for hash in self
                    .step
                    .as_ref()
                    .unwrap()
                    .jobs
                    .range(i.clone()..jobs_len)
                {
                    match self.results.get(hash) {
                        Some(res) => {
                            if res == &TestResult::False {
                                regression = Some(hash.to_string());
                                break;
                            }
                        }
                        None => {
                            incomplete = true;
                            break;
                        }
                    }
                }
                if !incomplete {
                    if let Some(reg_point) = regression {
                        self.right = reg_point;
                    }
                    self.left = lowest_valid.to_string();
                    self.clean_path();
                    self.step = None;
                    self.check_done();
                }
            } else {
                eprintln!("Result for {} is not expected. Will ignore it!", commit);
            }
        }
    }

    pub fn next_job(
        &mut self,
        capacity: usize,
        iteration: usize,
        take_samples: SampleFunction,
    ) -> super::AlgorithmResponse {
        if self.step.is_none() {
            let jobs = take_samples(&self.path, &self.left, &self.right, capacity, iteration)
                .expect("couldn't take samples!");

            let step = Step {
                job_queue: VecDeque::from(jobs.clone()),
                job_await: HashSet::new(),
                jobs: jobs,
                valid_nodes: DoublePriorityQueue::new(),
            };

            self.step = Some(step);
        }

        let step_mut = self.step.as_mut().unwrap();

        match step_mut.job_queue.pop_back() {
            Some(job) => {
                step_mut.job_await.insert(job.clone());
                super::AlgorithmResponse::Job(job.clone())
            }
            None => {
                if step_mut.job_await.is_empty() {
                    super::AlgorithmResponse::InternalError("Next step missing!")
                } else {
                    super::AlgorithmResponse::WaitForResult
                }
            }
        }
    }

    pub fn done(&self) -> bool {
        self.regression.is_some()
    }

    pub fn results(&self) -> Vec<RegressionPoint> {
        match self.regression.as_ref() {
            Some(reg) => vec![RegressionPoint {
                regression_point: reg.to_string(),
                target: self.target.clone(),
            }],
            None => vec![],
        }
    }

    fn check_done(&mut self) {
        match length_of_path(&self.path, &self.left, &self.right) {
            Ok(len) => {
                if len <= 2 {
                    self.regression = Some(self.right.to_string());
                }
            }
            Err(_) => panic!("Error at calculation length of path!"),
        }
    }

    fn clean_path(&mut self) {
        self.path = self
            .path
            .iter()
            .filter(|hash| match self.results.get(hash.clone()) {
                Some(res) => res != &TestResult::Ignore,
                None => true,
            })
            .cloned()
            .collect();
    }
}
