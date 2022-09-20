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
pub struct ExecutionData {
    pub setup: Duration,
    pub query: Duration,
    pub all: Duration,
    pub diff: u32,
}

pub type ProcessResponse = (
    u32,
    String,
    Result<(TestResult, ExecutionData), ProcessError>,
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
        setup_time: Instant,
    ) {
        let id = self.id;
        let worktree = self.worktree.clone();

        thread::spawn(move || {
            match S::get_commit_info(&worktree.location, &commit) {
                Some(message) => println!("Process {}:\n{}----", id, message),
                None => println!("Process {}: {}\n----", id, commit),
            }

            let distance = S::distance(&worktree, &commit);

            if S::checkout(&worktree, commit.as_str()).is_err() {
                let message = format!("{} couldn't checkout {}", id, commit);
                transmitter
                    .send((id, commit, Err(ProcessError::DVCSError(message))))
                    .expect("transmitter broken!");
                return;
            }

            let after_setup_time = Instant::now();

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

            let checkout_duration = after_setup_time.checked_duration_since(setup_time);
            let query_duration = after_query_time.checked_duration_since(after_setup_time);
            let overall_duration = after_query_time.checked_duration_since(setup_time);

            if checkout_duration.is_none() || query_duration.is_none() || overall_duration.is_none()
            {
                transmitter
                    .send((id, commit, Err(ProcessError::TimeError)))
                    .expect("transmitter broken!");
            } else {
                let execution_time = ExecutionData {
                    all: overall_duration.unwrap(),
                    setup: checkout_duration.unwrap(),
                    query: query_duration.unwrap(),
                    diff: distance,
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
