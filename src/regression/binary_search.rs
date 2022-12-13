use std::collections::VecDeque;

use super::{PathAlgorithm, RegressionAlgorithm, RegressionPoint, TestResult, interval_search::IntervalSearch};

pub struct BinarySearch {
    search: IntervalSearch
}

impl PathAlgorithm for BinarySearch {
    fn new(path: VecDeque<String>) -> Self {
        BinarySearch { search: IntervalSearch::new(path) }
    }
}

impl RegressionAlgorithm for BinarySearch {
    fn add_result(&mut self, commit: String, result: TestResult) {
        self.search.add_result(commit, result)
    }

    fn next_job(&mut self, capacity: u32) -> super::AlgorithmResponse {
        self.search.next_job(capacity as usize, take_uniform_sample)
    }

    fn done(&self) -> bool {
        self.search.done()
    }

    fn results(&self) -> Vec<RegressionPoint> {
        self.search.results()
    }

    fn interrupts(&mut self) -> Vec<String> {
        self.search.interrupts()
    }
}

fn take_uniform_sample<S: Clone + Eq>(
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
