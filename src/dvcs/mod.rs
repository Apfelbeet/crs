use std::collections::HashMap;

use daggy::{Dag, NodeIndex};

pub mod git;

pub trait DVCS {
    fn commit_graph(&self) -> Result<DVCSGraph, ()>;
    fn create_worktree(&self, name: &str) -> Result<Worktree, ()>;
    fn remove_worktree(&self, worktree: &Worktree) -> Result<(), ()>;
    fn checkout(&self, worktree: &Worktree, commit: &str) -> Result<(), ()>;
}

#[derive(Debug)]
pub struct DVCSGraph {
    pub graph: Dag<String, ()>,
    pub indexation: HashMap<String, NodeIndex> 
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub location: String,
    pub name: String,
}