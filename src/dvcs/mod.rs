use std::process::{Command, Output};

use crate::graph::Radag;

pub mod git;

pub trait DVCS {
    fn commit_graph(repository: &str) -> Result<Radag<String>, ()>;
    fn create_worktree(repository: &str, name: &str, external_location: Option<String>) -> Result<Worktree, ()>;
    fn remove_worktree(worktree: &Worktree) -> Result<(), ()>;
    fn checkout(worktree: &Worktree, commit: &str) -> Result<(), ()>;
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}

pub fn run_script_sync(location: &str, script_path: &str) -> std::io::Result<Output> {
    let mut command = Command::new(script_path);
    command.current_dir(location);

    let x = command.output();
    x
}

pub fn run_command_sync(location: &str, command: &mut Command) -> std::io::Result<Output> {
    command.current_dir(location).output()
}