use crate::dvcs::{run_script_async, Worktree, DVCS};
use crate::regression::TestResult;
use crate::log::output_path;
use std::marker::PhantomData;
use std::sync::mpsc;
use std::{thread, fmt};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ProcessError {
    DVCSError(String),
    ScriptError(String),
    TimeError,
    Code,
    Interrupt,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::process::ProcessError::DVCSError(s) => write!(f, "DVCS Error {}", s),
            crate::process::ProcessError::ScriptError(s) => write!(f, "Script Error {}", s),
            crate::process::ProcessError::TimeError => write!(f, "Time Error"),
            crate::process::ProcessError::Code => write!(f, "Exit Code"),
            crate::process::ProcessError::Interrupt => write!(f, "Interrupt"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionData {
    pub setup: Duration,
    pub query: Duration,
    pub all: Duration,
}

pub struct ProcessResponse {
    pub pid: u32,
    pub commit: String,
    pub result: Result<(TestResult, ExecutionData), ProcessError>,
}

pub struct LocalProcess<S> {
    pub id: u32,
    pub worktree: Worktree,
    interrupt_transmitter: Option<mpsc::Sender<()>>,
    _marker: PhantomData<S>,
}

impl<S: DVCS> LocalProcess<S> {
    pub fn new(id: u32, repository: &str, external_location: Option<String>) -> Self {
        let worktree = S::create_worktree(repository, &format!("crs_{}", id), external_location)
            .expect(format!("Couldn't create worktree for {}!", id).as_str());

        LocalProcess {
            id,
            worktree,
            interrupt_transmitter: None,
            _marker: PhantomData,
        }
    }

    pub fn run(
        &mut self,
        commit: String,
        trans: mpsc::Sender<ProcessResponse>,
        script_path: String,
        setup_time: Instant,
        log_directory: Option<&std::path::PathBuf>,
    ) {
        let id = self.id;
        let worktree = self.worktree.clone();
        let (interrupt_transmitter, interrupt_receiver) = mpsc::channel();
        self.interrupt_transmitter = Some(interrupt_transmitter.clone());
        let log_stdout = log_directory.map(|p| output_path(p).join(format!("{}_stdout", commit)));
        let log_stderr = log_directory.map(|p| output_path(p).join(format!("{}_stderr", commit)));

        thread::spawn(move || {
            if interrupt_receiver.try_recv().is_ok() {
                error(&trans, id, commit, ProcessError::Interrupt);
                return;
            }

            match S::get_commit_info(&worktree.location, &commit) {
                Some(message) => eprintln!("Process {}:\n{}----", id, message),
                None => eprintln!("Process {}: {}\n----", id, commit),
            }

            if interrupt_receiver.try_recv().is_ok() {
                error(&trans, id, commit, ProcessError::Interrupt);
                return;
            }

            if interrupt_receiver.try_recv().is_ok() {
                error(&trans, id, commit, ProcessError::Interrupt);
                return;
            }

            if S::checkout(&worktree, commit.as_str()).is_err() {
                let message = format!("{} couldn't checkout {}", id, commit);
                trans
                    .send(ProcessResponse {
                        pid: id,
                        commit,
                        result: Err(ProcessError::DVCSError(message)),
                    })
                    .expect("transmitter broken!");
                return;
            }

            if interrupt_receiver.try_recv().is_ok() {
                error(&trans, id, commit, ProcessError::Interrupt);
                return;
            }

            let after_setup_time = Instant::now();


            let mut child = match run_script_async(&worktree.location, &script_path, log_stdout, log_stderr) {
                Ok(child) => child,
                Err(err) => {
                    scerror(&trans, id, commit, err.to_string());
                    return;
                }
            };

            loop {
                let response = child.try_wait();

                let op_code = match response {
                    Ok(op_code) => op_code,
                    Err(err) => {
                        scerror(&trans, id, commit.clone(), err.to_string());
                        return;
                    }
                };

                let code = match op_code {
                    Some(code) => code.code().unwrap(),
                    None => match interrupt_receiver.try_recv() {
                        Ok(_) => {
                            child.kill().expect("Terminating process killed!");
                            error(&trans, id, commit, ProcessError::Interrupt);
                            return;
                        }
                        Err(_) => continue,
                    },
                };

                let result = if code == 0 {
                    TestResult::True
                } else if code == 125 {
                    TestResult::Ignore
                } else if code >= 128 {
                    cderror(&trans, id, commit);
                    return;
                } else {
                    TestResult::False
                };

                let after_query_time = Instant::now();
                let checkout_duration = after_setup_time.checked_duration_since(setup_time);
                let query_duration = after_query_time.checked_duration_since(after_setup_time);
                let overall_duration = after_query_time.checked_duration_since(setup_time);

                if checkout_duration.is_none()
                    || query_duration.is_none()
                    || overall_duration.is_none()
                {
                    error(&trans, id, commit.clone(), ProcessError::TimeError);
                } else {
                    let execution_time = ExecutionData {
                        all: overall_duration.unwrap(),
                        setup: checkout_duration.unwrap(),
                        query: query_duration.unwrap(),
                    };
                    trans
                        .send(ProcessResponse {
                            pid: id,
                            commit: commit.clone(),
                            result: Ok((result, execution_time)),
                        })
                        .expect("transmitter broken!");
                };
                break;
            }
        });
    }

    pub fn interrupt(&mut self) {
        if let Some(trans) = self.interrupt_transmitter.as_ref() {
            trans.send(()).expect("transmitter broken!");
            self.interrupt_transmitter = None;
        }
    }

    pub fn clean_up(&self) {
        if S::remove_worktree(&self.worktree).is_err() {
            eprintln!("Can not remove worktree of process {}", self.id);
        }
    }
}

fn scerror(transmitter: &mpsc::Sender<ProcessResponse>, id: u32, commit: String, message: String) {
    error(transmitter, id, commit, ProcessError::ScriptError(message));
}

fn cderror(transmitter: &mpsc::Sender<ProcessResponse>, id: u32, commit: String) {
    error(transmitter, id, commit, ProcessError::Code);
}

fn error(
    transmitter: &mpsc::Sender<ProcessResponse>,
    id: u32,
    commit: String,
    message: ProcessError,
) {
    transmitter
        .send(ProcessResponse {
            pid: id,
            commit: commit.to_string(),
            result: Err(message),
        })
        .expect("transmitter broken!");
}
