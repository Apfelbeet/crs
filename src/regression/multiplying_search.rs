use std::collections::VecDeque;

use super::{general_binary_search::GeneralBinarySearch, PathAlgorithm, RegressionAlgorithm};

pub struct MultiplyingSearch {
    search: GeneralBinarySearch,
    iteration: usize,
}

impl PathAlgorithm for MultiplyingSearch {
    fn new(path: VecDeque<String>) -> Self {
        MultiplyingSearch {
            search: GeneralBinarySearch::new(path),
            iteration: 0,
        }
    }
}

impl RegressionAlgorithm for MultiplyingSearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        let left_old = self.search.left.clone();

        self.search.add_result(commit, result);

        //If the current step is done and we didn't move the left border, we
        //want to increase the step size. Otherwise we want to reset it.
        if self.search.step.is_none() {
            if left_old == self.search.left {
                self.iteration += 1;
            } else {
                self.iteration = 0;
            }
        }
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        self.search
            .next_job(capacity as usize, self.iteration, take_samples)
    }

    fn done(&self) -> bool {
        self.search.done()
    }

    fn results(&self) -> Vec<super::RegressionPoint> {
        self.search.results()
    }
}

fn take_samples<S: Clone + Eq>(
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
                samples.push_front(path.get(index_on_path).expect("Invalid index!").clone());
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
            let range = path.range(r - std::cmp::min(sample_size, length)..r).rev();
            samples = VecDeque::from_iter(range.cloned());
        }

        Ok(samples)
    } else {
        Err(())
    }
}
