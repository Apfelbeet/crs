use super::TestResult;

pub struct Settings {
    pub propagate: bool,
    pub extended_search: bool,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct RPANode {
    pub result: Option<TestResult>,
    pub hash: String,
    // pub distance: u32,
}