use std::{process::{Command, Output, ExitStatus}, time::Duration};

use crate::graph::Radag;

use self::benchmark::Benchmark;

pub mod git;
pub mod benchmark;

pub trait DVCS {
    fn commit_graph(repository: &str) -> Result<Radag<String, ()>, ()>;
    fn create_worktree(repository: &str, name: &str, external_location: Option<String>) -> Result<Worktree, ()>;
    fn remove_worktree(worktree: &Worktree) -> Result<(), ()>;
    fn checkout(worktree: &Worktree, commit: &str) -> Result<(), ()>;
    fn get_commit_info(repository: &str, commit: &str) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}

/**
 * BENCHMARK OVERRIDE
 */
pub fn run_script_sync(location: &str, script_path: &str) -> std::io::Result<i32> {
    let rec = Benchmark::register_job(location);

    let code = if rec.recv().expect("channel closed!") {
        0
    } else {
        -1
    }; 

    Ok(code)
}

pub fn run_command_sync(location: &str, command: &mut Command) -> std::io::Result<Output> {
    command.current_dir(location).output()
}