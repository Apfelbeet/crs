use super::RegressionAlgorithm;

#[derive(Debug)]
pub struct Iter {
    counter: usize,
    counter2: usize,
    leaves: Vec<String>,
}

impl Iter {
    pub fn new(leaves: Vec<String>) -> Self {
        Iter {
            counter2: 0,
            counter: 0,
            leaves
        }
    }
}

impl RegressionAlgorithm for Iter {
    fn add_result(&mut self, commit: String, _: super::TestResult) {
        println!("{commit} received result!");
    }

    fn next_job(&mut self) -> super::AlgorithmResponse {
        
        if self.counter % 10 == 9 {
            return super::AlgorithmResponse::WaitForResult
        }

        let x = self.leaves.get(self.counter2);
        self.counter2 += 1;
        match x {
            Some(commit) => super::AlgorithmResponse::Job(commit.to_string()),
            None => super::AlgorithmResponse::InternalError("No jobs remaining!"),
        }
    }

    fn done(&self) -> bool {
        self.counter2 >= self.leaves.len()
    }
}

