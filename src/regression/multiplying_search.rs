use std::collections::{HashSet, VecDeque};

use crate::graph::length_of_path;

use super::{AssignedRegressionPoint, PathAlgorithm, RegressionAlgorithm, RegressionPoint};

pub struct MultiplyingSearch {
    path: VecDeque<String>,
    target: String,
    left: String,
    right: String,
    step: Option<Step>,
    regression: Option<String>,
    iteration: usize,
}

struct Step {
    job_queue: VecDeque<String>,
    job_await: HashSet<String>,
    jobs: VecDeque<String>,
    lowest: Option<String>,
}

impl PathAlgorithm for MultiplyingSearch {
    fn new(path: VecDeque<String>) -> Self {
        if path.len() <= 1 {
            panic!("Path is too short for a regression point!");
        }

        let left = path.front().unwrap().clone();
        let right = path.back().unwrap().clone();

        let mut mult = MultiplyingSearch {
            path,
            target: right.to_string(),
            left,
            right,
            regression: None,
            step: None,
            iteration: 0,
        };

        mult.check_done();

        mult
    }
}

impl MultiplyingSearch {
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
}

impl RegressionAlgorithm for MultiplyingSearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        if self.step.is_none() {
            panic!("multiplying_search: No active step!");
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
                            .find(|current| {
                                current.clone() == &commit
                                    || (lowest.is_some()
                                        && lowest.as_ref().unwrap() == current.clone())
                            })
                            .cloned();
                    }
                    super::TestResult::False => {}
                }

                if self.step.as_ref().unwrap().job_await.is_empty() {
                    match &self.step.as_ref().unwrap().lowest {
                        Some(lowest) => {
                            let mut prev = None;

                            for job in &self.step.as_ref().unwrap().jobs {
                                if job == lowest {
                                    break;
                                }
                                prev = Some(job);
                            }

                            self.left = lowest.clone();

                            if let Some(p) = prev {
                                self.right = p.clone();
                            }
                            self.iteration = 0;
                        }
                        None => {
                            self.right = self
                                .step
                                .as_ref()
                                .unwrap()
                                .jobs
                                .back()
                                .expect("Sample is missing!")
                                .clone();
                            // self.iteration += self.step.as_ref().unwrap().jobs.len();
                            self.iteration += 1;
                        }
                    }
        
                    self.step = None;
                    self.check_done();
                }
            } else {
                //Should not happen: We didn't expect a result for this commit.
                //It's not a critical error, we could just ignore it, but it
                //indicates a faulty behavior of the program.
                panic!("multiplying_search: Didn't expect a result for {}", commit);
            }
        } 
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        if self.step.is_none() {
            let jobs = take_samples(
                &self.path,
                &self.left,
                &self.right,
                capacity as usize,
                self.iteration,
            )
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
                    super::AlgorithmResponse::InternalError("Next step missing!")
                } else {
                    super::AlgorithmResponse::WaitForResult
                }
            }
        }
    }

    fn done(&self) -> bool {
        self.regression.is_some()
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        let mut regs = Vec::new();

        if let Some(point) = self.regression.as_ref() {
            regs.push(RegressionPoint::Point(AssignedRegressionPoint {
                target: self.target.to_string(),
                regression_point: point.to_string(),
            }));
        }

        regs
    }
}

fn take_samples<S: Clone + Eq + std::fmt::Debug>(
    path: &VecDeque<S>,
    left: &S,
    right: &S,
    sample_size: usize,
    iteration: usize,
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

        //Exclude outer points => -2
        let length = r - l - 1;
        //Find most efficient factor. We assume that the optimal factor is
        //equal to the capacity/sample size. But if the path is to short, we
        //can not utilize the whole capacity. So we want to decrease the
        //factor.
        let mut samples = VecDeque::<S>::new();
        let mut factor = sample_size + 1;
        while factor > 1 {
            let mut sum = 0;
            let mut summand = 1;
            for _ in 0..iteration {
                summand *= factor;
            }
            let mut invalid = false;
            for i in 0..sample_size {
                sum += summand;
                summand *= factor;

                //it is ok to make the area smaller, if it is the last area. 
                if i == sample_size - 1 {
                    sum = std::cmp::min(length, sum);
                } 
                //otherwise if any other sample point would be outside or the
                //last point of the range, we know that we have to decrease the
                //factor.
                else if sum >= length {
                    invalid = true;
                    break;
                }

                let index_on_path = r - sum;
                samples.push_back(path.get(index_on_path).expect("Invalid index!").clone());
            }

            if invalid {
                samples.clear();
                factor -= 1;
            } else {
                break;
            }
        }

        if factor == 1 {
            // path.iter().take(std::cmp::min(sample_size, length)).cloned()
            let range = path.range(r-std::cmp::min(sample_size, length)..r).rev();
            samples = VecDeque::from_iter(range.cloned());
        }

        Ok(samples)
    } else {
        Err(())
    }
}
