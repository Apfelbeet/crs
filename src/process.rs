use std::sync::mpsc;
use std::thread;

use crate::dvcs::git::Git;
use crate::dvcs::{Worktree, DVCS};
use crate::regression::TestResult;

pub type ProcessResponse = (u64, String, TestResult);

pub struct LocalProcess {
    pub id: u64,
    worktree: Worktree,
    repo: String,
}

impl LocalProcess {
    pub fn new(id: u64, repo: String) -> Self {
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

    pub fn run(&self, commit: String, transmitter: mpsc::Sender<ProcessResponse>) {
        let id = self.id;
        let dvcs = Git::new(self.repo.to_string());
        let worktree = self.worktree.clone();

        thread::spawn(move || {
            
            println!("{} checkout {}", id, commit);
            if dvcs.checkout(&worktree, commit.as_str()).is_err() {
                panic!("{} couldn't checkout {}", id, commit);
            }
            

            println!("{} is testing {}", id, commit);
            thread::sleep(std::time::Duration::from_secs(10));
            println!("{} DONE testing {}", id, commit);

            transmitter.send((id, commit, TestResult::True))
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
