use std::{
    fs::File,
    io::Error,
    process::{Child, Command, Output, Stdio},
};

use crate::graph::Adag;

pub mod git;

pub trait DVCS {
    fn commit_graph(
        repository: &str,
        start: Vec<String>,
        targets: Vec<String>,
    ) -> Result<Adag<String, ()>, ()>;
    fn create_worktree(
        repository: &str,
        name: &str,
        external_location: Option<String>,
    ) -> Result<Worktree, ()>;
    fn remove_worktree(worktree: &Worktree) -> Result<(), ()>;
    fn checkout(worktree: &Worktree, commit: &str) -> Result<(), ()>;
    fn get_commit_info(repository: &str, commit: &str) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}

pub fn run_script_async(
    location: &str,
    script_path: &str,
    log_stdout: Option<std::path::PathBuf>,
    log_stderr: Option<std::path::PathBuf>,
) -> Result<Child, Error> {
    let mut command = Command::new(script_path);
    command.current_dir(location);

    match log_stdout {
        Some(path) => {
            let stdout = File::create(path).unwrap();
            command.stdout(stdout);
        }
        None => {
            command.stdout(Stdio::null());
        }
    }

    match log_stderr {
        Some(path) => {
            let stderr = File::create(path).unwrap();
            command.stderr(stderr);
        }
        None => {
            command.stderr(Stdio::null());
        }
    }

    let x = command.spawn();
    x
}

pub fn run_command_sync(location: &str, command: &mut Command) -> std::io::Result<Output> {
    command.current_dir(location).output()
}
