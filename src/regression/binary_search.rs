use std::collections::{HashSet, VecDeque};

use crate::graph::length_of_path;

use super::{AssignedRegressionPoint, RegressionAlgorithm, RegressionPoint, PathAlgorithm};

//TODO: Replace String with generics.

pub struct BinarySearch {
    path: VecDeque<String>,
    target: String,
    left: String,
    right: String,
    step: Option<Step>,
    candidates: Vec<RegressionPoint>,
    regression: Option<String>,
}

struct Step {
    job_queue: VecDeque<String>,
    job_await: HashSet<String>,
    jobs: VecDeque<String>,
    lowest: Option<String>,
}

impl PathAlgorithm for BinarySearch {
    fn new(path: VecDeque<String>) -> Self {
        if path.len() <= 1 {
            panic!("Path is too short for a regression point!");
        }

        let left = path.front().unwrap().clone();
        let right = path.back().unwrap().clone();

        let mut bin = BinarySearch {
            path,
            target: right.to_string(),
            left,
            right,
            candidates: Vec::new(),
            regression: None,
            step: None,
        };

        bin.check_done();

        bin
    }
}

impl BinarySearch {
    fn check_done(&mut self) {
        let len = length_of_path(&self.path, &self.left, &self.right);
        println!("{} to {} len: {:?}", self.left, self.right, len);
        match length_of_path(&self.path, &self.left, &self.right) {
            Ok(len) => {
                if len <= 2 {
                    self.regression = Some(self.right.to_string());
                }
            }
            Err(_) => panic!("Error at calculation length of path!"),
        }
    }
}

impl RegressionAlgorithm for BinarySearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        if self.step.is_none() {
            panic!("binary_search: No active step!")
        } else {
            if self.step.as_mut().unwrap().job_await.remove(&commit) {
                match result {
                    super::TestResult::True => {
                        let lowest = &self.step.as_ref().unwrap().lowest;
                        self.step.as_mut().unwrap().lowest = self
                            .step
                            .as_ref()
                            .unwrap()
                            .jobs
                            .iter()
                            .reduce(|acc, current| {
                                if current == &commit
                                    || (lowest.is_some() && lowest.as_ref().unwrap() == current)
                                {
                                    current
                                } else {
                                    acc
                                }
                            })
                            .cloned();
                    }
                    super::TestResult::False => {}
                };

                //If all jobs responded, we are ready to evaluate the result and
                //to adapt the range.
                if self.step.as_ref().unwrap().job_await.is_empty() {
                    match &self.step.as_ref().unwrap().lowest {
                        Some(lowest) => {
                            let mut next_stop = false;
                            let mut next = None;
                            for job in &self.step.as_ref().unwrap().jobs {
                                if next_stop {
                                    next = Some(job);
                                    break;
                                }
                                if job == lowest {
                                    next_stop = true;
                                }
                            }

                            self.left = lowest.clone();

                            //If we find a following sample point that is
                            //invalid we set it as the right boundary of the
                            //next step. Otherwise we keep the current right
                            //value.
                            if let Some(n) = next {
                                self.right = n.clone();
                            }
                        }
                        //If we didn't find a valid sample. Then we keep the
                        //left value and set the right boundary to the first
                        //sample point.
                        None => {
                            self.right = self
                                .step
                                .as_ref()
                                .unwrap()
                                .jobs
                                .front()
                                .expect("Samples are missing!")
                                .clone();
                        }
                    }
                    self.step = None;
                    self.check_done();
                }
            } else {
                //Should not happen: We didn't expect a result for this commit.
                //It's not a critical error, we could just ignore it, but it
                //indicates a faulty behavior of the program.
                panic!("binary_search: Didn't expect a result for {}", commit);
            }
        }
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        if self.step.is_none() {
            let jobs = take_uniform_sample(&self.path, &self.left, &self.right, capacity as usize)
                .expect("couldn't take samples!");

            let step = Step {
                job_queue: VecDeque::from(jobs.clone()),
                job_await: HashSet::new(),
                jobs: jobs,
                lowest: None,
            };

            self.step = Some(step);
        }

        match self.step.as_mut().unwrap().job_queue.pop_back() {
            Some(job) => {
                self.step.as_mut().unwrap().job_await.insert(job.clone());
                super::AlgorithmResponse::Job(job.clone())
            }
            None => {
                if self.step.as_mut().unwrap().job_await.is_empty() {
                    super::AlgorithmResponse::InternalError("Miss next step!")
                } else {
                    super::AlgorithmResponse::WaitForResult
                }
            }
        }
    }

    fn done(&self) -> bool {
        self.regression.is_some()
    }

    fn results(&self) -> Vec<RegressionPoint> {
        let mut regs = self.candidates.clone();

        if let Some(point) = self.regression.as_ref() {
            regs.push(RegressionPoint::Point(AssignedRegressionPoint {
                target: self.target.to_string(),
                regression_point: point.to_string(),
            }));
        }

        regs
    }
}

fn take_uniform_sample<S: Clone + Eq + std::fmt::Debug>(
    path: &VecDeque<S>,
    left: &S,
    right: &S,
    sample_size: usize,
) -> Result<VecDeque<S>, ()> {
    let mut left_index = None;
    let mut right_index = None;

    let mut found = false;
    for (index, node) in path.iter().enumerate() {
        if node == left {
            left_index = Some(index)
        }
        if node == right {
            right_index = Some(index)
        }
        if left_index.is_some() && right_index.is_some() {
            found = true;
            break;
        }
    }

    if found {
        let l = std::cmp::min(left_index.unwrap(), right_index.unwrap());
        let r = std::cmp::max(left_index.unwrap(), right_index.unwrap());

        let length = r - l;
        let ss = std::cmp::min(length, sample_size + 1);
        let delta = (length as f64) / (ss as f64);

        let mut res = VecDeque::new();
        let mut current = l as f64;
        while res.len() <= ss {
            let index = current.round() as usize;
            res.push_back(
                path.get(index)
                    .expect("take_uniform_sample: invalid index")
                    .clone(),
            );

            current += delta;
        }

        //Remove boundaries.
        res.pop_front();
        res.pop_back();

        Ok(res)
    } else {
        Err(())
    }
}
