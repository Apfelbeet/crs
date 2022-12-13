use std::collections::VecDeque;

use super::{interval_search::IntervalSearch, PathAlgorithm, RegressionAlgorithm};

pub struct MultiplyingSearch {
    search: IntervalSearch,
}

impl PathAlgorithm for MultiplyingSearch {
    fn new(path: VecDeque<String>) -> Self {
        MultiplyingSearch {
            search: IntervalSearch::new(path),
        }
    }
}

impl RegressionAlgorithm for MultiplyingSearch {
    fn add_result(&mut self, commit: String, result: super::TestResult) {
        self.search.add_result(commit, result);
    }

    fn next_job(&mut self, _: u32, expected_capacity: u32) -> super::AlgorithmResponse {
        self.search
            .next_job(expected_capacity as usize, take_samples)
    }

    fn interrupts(&mut self) -> Vec<String> {
        self.search.interrupts()
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
        let mut samples = VecDeque::<S>::new();
        let mut factor = sample_size + 1;
        while factor > 0 && samples.len() < sample_size {
            samples.clear();
            let mut sum = 1;
            let mut summand = 1;
            while sum <= length {
                let index_on_path = r - sum;
                let hash = path.get(index_on_path).expect("Invalid index!").clone();
                samples.push_front(hash);
                
                if sum == length {
                    break;
                }

                summand *= factor;
                sum = std::cmp::min(sum + summand, length);
            }
            factor -= 1;
        }
        Ok(samples)
    } else {
        Err(())
    }
}
