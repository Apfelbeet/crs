use crate::dvcs::{run_script_sync, Worktree, DVCS};
use crate::regression::TestResult;
use std::marker::PhantomData;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ProcessError {
    DVCSError(String),
    ScriptError(String),
    TimeError,
}

#[derive(Debug, Clone)]
pub struct ExecutionTime {
    pub checkout: Duration,
    pub query: Duration,
    pub all: Duration,
}

pub type ProcessResponse = (
    u32,
    String,
    Result<(TestResult, ExecutionTime), ProcessError>,
);

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

    pub fn run(
        &self,
        commit: String,
        transmitter: mpsc::Sender<ProcessResponse>,
        script_path: String,
    ) {
        let id = self.id;
        let worktree = self.worktree.clone();

        thread::spawn(move || {
            match S::get_commit_info(&worktree.location, &commit) {
                Some(message) => println!("Process {}:\n{}----", id, message),
                None => println!("Process {}: {}\n----", id, commit),
            }

            let start_time = Instant::now();

            if S::checkout(&worktree, commit.as_str()).is_err() {
                let message = format!("{} couldn't checkout {}", id, commit);
                transmitter
                    .send((id, commit, Err(ProcessError::DVCSError(message))))
                    .expect("transmitter broken!");
                return;
            }

            let after_checkout_time = Instant::now();

            let result = match run_script_sync(&worktree.location, &script_path) {
                Ok(output) => match output.status.code() {
                    Some(code) => {
                        if code == 0 {
                            TestResult::True
                        } else if code == 125 {
                            TestResult::Ignore
                        } else {
                            TestResult::False
                        }
                    }
                    None => {
                        transmitter
                            .send((
                                id,
                                commit,
                                Err(ProcessError::ScriptError(format!("{:?}", output))),
                            ))
                            .expect("transmitter broken!");
                        return;
                    }
                },
                Err(err) => {
                    transmitter
                        .send((id, commit, Err(ProcessError::ScriptError(err.to_string()))))
                        .expect("transmitter broken!");
                    return;
                }
            };

            let after_query_time = Instant::now();

            let checkout_duration = after_checkout_time.checked_duration_since(start_time);
            let query_duration = after_query_time.checked_duration_since(after_checkout_time);
            let overall_duration = after_query_time.checked_duration_since(start_time);

            if checkout_duration.is_none() || query_duration.is_none() || overall_duration.is_none()
            {
                transmitter
                    .send((id, commit, Err(ProcessError::TimeError)))
                    .expect("transmitter broken!");
            } else {
                let execution_time = ExecutionTime {
                    all: overall_duration.unwrap(),
                    checkout: checkout_duration.unwrap(),
                    query: query_duration.unwrap(),
                };
                transmitter
                    .send((id, commit, Ok((result, execution_time))))
                    .expect("transmitter broken!");
            };
        });
    }

    pub fn clean_up(&self) {
        if S::remove_worktree(&self.worktree).is_err() {
            eprintln!("Can not remove worktree of process {}", self.id);
        }
    }
}
