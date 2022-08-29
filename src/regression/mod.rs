pub mod iter;
pub mod rpa;
pub mod binary_search;

#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    True,
    False,
    Ignore,
}

#[derive(Debug)]
pub enum AlgorithmResponse<'a> {
    Job(String),
    WaitForResult,
    InternalError(&'a str)
}

#[derive(Debug, Clone)]
pub struct AssignedRegressionPoint {
    target: String,
    regression_point: String,
}

pub trait RegressionAlgorithm {
    fn add_result(&mut self, commit: String, result: TestResult);
    fn next_job(&mut self, capacity: u32) -> AlgorithmResponse;
    fn done(&self) -> bool;
    fn results(&self) -> Vec<AssignedRegressionPoint>;
}