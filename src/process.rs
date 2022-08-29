use std::sync::mpsc;
use std::thread;

use crate::dvcs::git::Git;
use crate::dvcs::{Worktree, DVCS, run_script_sync};
use crate::regression::{TestResult, RegressionAlgorithm};

pub type ProcessResponse = (u32, String, TestResult);

pub struct LocalProcess {
    pub id: u32,
    worktree: Worktree,
    repo: String,
}

impl LocalProcess {
    pub fn new(id: u32, repo: String) -> Self {
        println!("Spawn {}", id);

        let dvcs = Git::new(repo.to_string());
        let worktree = dvcs
            .create_worktree(id.to_string().as_str())
            .expect(format!("Couldn't create worktree for {}!", id).as_str());

        LocalProcess {
            id,
            worktree,
            repo,
        }
    }

    pub fn run(&self, commit: String, transmitter: mpsc::Sender<ProcessResponse>, script_path: String) {
        let id = self.id;
        let dvcs = Git::new(self.repo.to_string());
        let worktree = self.worktree.clone();

        thread::spawn(move || {
            //TODO: panic in a thread will only stop this thread, but we need to
            //handle this error also in the main thread. Solution: Add some kind
            //of error message, that is send to the receiver instead of panicking. 
            println!("{} checkout {}", id, commit);
            if dvcs.checkout(&worktree, commit.as_str()).is_err() {
                panic!("{} couldn't checkout {}", id, commit);
            }
            
            println!("{} is testing {}", id, commit);
            let result = match run_script_sync(&worktree.location, &script_path) {
                Ok(output) => match output.status.code() {
                    Some(code) => if code == 0 {
                        TestResult::True
                    } else {
                        TestResult::False
                    },
                    None => panic!("test case responded weird: {:?}", output),
                },
                Err(err) => panic!("test for failed {:?}", err),
            };
            println!("{} DONE testing {} -> {:?}", id, commit, result);

            transmitter.send((id, commit, result))
        });
    }

    pub fn clean_up(&self) {
        println!("Clean up process {}", self.id);
        let dvcs = Git::new(self.repo.to_string());
        if dvcs.remove_worktree(&self.worktree).is_err() {
            eprintln!("Can not remove worktree of process {}", self.id);
        }
    }
}
