use std::collections::VecDeque;

pub mod rpa;
pub mod binary_search;
pub mod linear_search;
pub mod multiplying_search;
mod general_binary_search;

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
pub struct RegressionPoint {
    pub target: String,
    pub regression_point: String,
}

pub trait RegressionAlgorithm {
    fn add_result(&mut self, commit: String, result: TestResult);
    fn next_job(&mut self, capacity: u32) -> AlgorithmResponse;
    fn done(&self) -> bool;
    fn results(&self) -> Vec<RegressionPoint>;
}

pub trait PathAlgorithm {
    fn new(path: VecDeque<String>) -> Self;
}