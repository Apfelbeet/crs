pub mod iter;
pub mod rpa;

#[derive(Debug)]
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

pub trait RegressionAlgorithm {
    fn add_result(&mut self, commit: String, result: TestResult);
    fn next_job(&mut self) -> AlgorithmResponse;
    fn done(&self) -> bool; 
}