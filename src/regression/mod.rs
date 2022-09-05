use std::collections::VecDeque;

pub mod rpa;
pub mod binary_search;
pub mod linear_search;

#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    True,
    False,
}

#[derive(Debug)]
pub enum AlgorithmResponse<'a> {
    Job(String),
    WaitForResult,
    InternalError(&'a str)
}

#[derive(Debug, Clone)]
pub enum RegressionPoint {
    Point(AssignedRegressionPoint),
    Candidate(AssignedRegressionCandidate),
}

#[derive(Debug, Clone)]
pub struct AssignedRegressionPoint {
    target: String,
    regression_point: String,
}

#[derive(Debug, Clone)]
pub struct AssignedRegressionCandidate {
    target: String,
    lower_bound: String,
    upper_bound: String,
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