use std::{collections::VecDeque, fmt};

mod rpa_extension;
mod generalized_rpa;

pub mod path_selection;
pub mod rpa_util;

pub mod rpa_search {
    pub use super::generalized_rpa::*;
}

pub mod binary_search;
pub mod linear_search;
pub mod multiplying_search;
mod interval_search1;
// mod interval_search2;
//mod interval_search3;
mod interval_search {
    pub use super::interval_search1::*;
    // pub use super::interval_search2::*;
    // pub use super::interval_search3::*;
}

pub const NAME: &str = interval_search::NAME; 


#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TestResult {
    True,
    False,
    Ignore,
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestResult::True => write!(f, "True"),
            TestResult::False => write!(f, "False"),
            TestResult::Ignore => write!(f, "Ignore"),
        }
    }
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
    fn interrupts(&mut self) -> Vec<String>;
    fn done(&self) -> bool;
    fn results(&self) -> Vec<RegressionPoint>;
}

pub trait PathAlgorithm {
    fn new(path: VecDeque<String>) -> Self;
}