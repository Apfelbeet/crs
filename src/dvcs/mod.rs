use std::process::{Command, Output};

use crate::graph::Radag;

pub mod git;

pub trait DVCS {
    fn commit_graph(&self) -> Result<Radag<String>, ()>;
    fn create_worktree(&self, name: &str) -> Result<Worktree, ()>;
    fn remove_worktree(&self, worktree: &Worktree) -> Result<(), ()>;
    fn checkout(&self, worktree: &Worktree, commit: &str) -> Result<(), ()>;
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}

pub fn run_script_sync(location: &String, script_path: &String) -> std::io::Result<Output> {
    let mut command = Command::new(script_path);
    command.current_dir(location.as_str());

    command.output()
}

pub fn run_command_sync(location: &String, command: &mut Command) -> std::io::Result<Output> {
    command.current_dir(location.as_str()).output()
}