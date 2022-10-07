use std::{process::{Command, Output, Child, Stdio}, io::Error};

use crate::graph::Radag;

pub mod git;

pub trait DVCS {
    fn commit_graph(repository: &str, start: Vec<String>, targets: Vec<String>) -> Result<Radag<String, ()>, ()>;
    fn create_worktree(repository: &str, name: &str, external_location: Option<String>) -> Result<Worktree, ()>;
    fn remove_worktree(worktree: &Worktree) -> Result<(), ()>;
    fn checkout(worktree: &Worktree, commit: &str) -> Result<(), ()>;
    fn get_commit_info(repository: &str, commit: &str) -> Option<String>;
    fn distance(worktree: &Worktree, commit: &str) -> u32;
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}

// pub fn run_script_sync(location: &str, script_path: &str) -> std::io::Result<Output> {
//     let mut command = Command::new(script_path);
//     command.current_dir(location);

//     let x = command.output();
//     x
// }

pub fn run_script_async(location: &str, script_path: &str) -> Result<Child, Error> {
    let mut command = Command::new(script_path);
    command.current_dir(location);
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    let x = command.spawn();
    x
}

pub fn run_command_sync(location: &str, command: &mut Command) -> std::io::Result<Output> {
    command.current_dir(location).output()
}