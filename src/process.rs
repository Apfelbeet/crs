use std::marker::PhantomData;
use std::sync::mpsc;
use std::thread;
use crate::dvcs::{Worktree, DVCS, run_script_sync};
use crate::regression::TestResult;

pub type ProcessResponse = (u32, String, TestResult);

pub struct LocalProcess<S> {
    pub id: u32,
    pub worktree: Worktree,
    _marker: PhantomData<S>,
}

impl<S: DVCS> LocalProcess<S> {
    pub fn new(id: u32, repository: &str, external_location: Option<String>) -> Self {

        let worktree = S::create_worktree(repository, &format!("crs_{}", id), external_location)
            .expect(format!("Couldn't create worktree for {}!", id).as_str());

        LocalProcess {
            id,
            worktree,
            _marker: PhantomData,
        }
    }

    pub fn run(&self, commit: String, transmitter: mpsc::Sender<ProcessResponse>, script_path: String) {
        let id = self.id;
        let worktree = self.worktree.clone();

        thread::spawn(move || {
            //TODO: panic in a thread will only stop this thread, but we need to
            //handle this error also in the main thread. Solution: Add some kind
            //of error message, that is send to the receiver instead of panicking. 
            
            match S::get_commit_info(&worktree.location, &commit) {
                Some(message) => println!("Process {}:\n{}----", id, message),
                None => println!("Process {}: {}\n----", id, commit),
            }

            if S::checkout(&worktree, commit.as_str()).is_err() {
                panic!("{} couldn't checkout {}", id, commit);
            }
            
            let result = match run_script_sync(&worktree.location, &script_path) {
                Ok(output) => match output.status.code() {
                    Some(code) => if code == 0 {
                        TestResult::True
                    } else if code == 125 {
                        TestResult::Ignore
                    } else {
                        TestResult::False
                    },
                    None => panic!("test case responded weird: {:?}", output),
                },
                Err(err) => panic!("test for failed {:?}", err),
            };

            transmitter.send((id, commit, result))
        });
    }

    pub fn clean_up(&self) {
        if S::remove_worktree(&self.worktree).is_err() {
            eprintln!("Can not remove worktree of process {}", self.id);
        }
    }
}
