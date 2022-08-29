use crate::dvcs::DVCS;
use crate::graph::Radag;
use daggy::{Dag, NodeIndex};
use std::collections::HashMap;
use std::process::Command;

use super::run_command_sync;

#[derive(Debug, Clone)]
pub struct Git {
    path: String,
}

impl Git {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

impl DVCS for Git {
    fn commit_graph(&self) -> Result<Radag<String>, ()> {
        let mut command = Command::new("git");
        command.args(["rev-list", "--all", "--parents"]);

        let rev_list = match run_command_sync(&self.path, &mut command) {
            Err(err) => {
                print_error(err.to_string().as_str());
                Err(())
            }
            Ok(output) => {
                if output.status.success() {
                    match String::from_utf8(output.stdout) {
                        Ok(r) => Ok(r),
                        Err(_) => Err(()),
                    }
                } else {
                    print_error(String::from_utf8(output.stderr).unwrap().as_str());
                    Err(())
                }
            }
        };

        return parse_rev_list(rev_list?);
    }

    fn create_worktree(&self, name: &str) -> Result<super::Worktree, ()> {
        let location = format!("./.crs/{}", name);
        let abs_location = format!("{}/.crs/{}", self.path, name);

        let mut command = Command::new("git");

        command.args([
            "worktree",
            "add",
            "--detach",
            location.as_str(),
            "--no-checkout",
        ]);

        // if init_commit.is_some() {
        //     command.arg("init_commit");
        // }

        return match run_command_sync(&self.path, &mut command) {
            Ok(output) => {
                if output.status.success() {
                    Ok(super::Worktree {
                        location: abs_location,
                        name: name.to_string(),
                    })
                } else {
                    print_error(String::from_utf8(output.stderr).unwrap().as_str());
                    Err(())
                }
            }
            Err(e) => {
                print_error(e.to_string().as_str());
                Err(())
            }
        };
    }

    fn remove_worktree(&self, worktree: &super::Worktree) -> Result<(), ()> {
        let mut rm_tree = Command::new("git");
        rm_tree.args(["worktree", "remove", worktree.name.as_str()]);

        return match run_command_sync(&self.path, &mut rm_tree) {
            Ok(o) => {
                if o.status.success() {
                    Ok(())
                } else {
                    print_error(String::from_utf8(o.stderr).unwrap().as_str());
                    Err(())
                }
            }
            Err(e) => {
                print_error(e.to_string().as_str());
                Err(())
            }
        };
    }

    fn checkout(&self, worktree: &super::Worktree, commit: &str) -> Result<(), ()> {
        let mut command = Command::new("git");
        command.args(["checkout", commit]);

        return match run_command_sync(&worktree.location, &mut command) {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    print_error(String::from_utf8(output.stderr).unwrap().as_str());
                    Err(())
                }
            }
            Err(e) => {
                print_error(e.to_string().as_str());
                Err(())
            }
        };
    }
}

fn print_error(msg: &str) {
    eprintln!("Git Error: {}", msg);
}

fn parse_rev_list(rev_list: String) -> Result<Radag<String>, ()> {
    let mut indexation = HashMap::new();
    let mut graph = Dag::new();

    let lines = rev_list.lines();

    for line in lines {
        let mut hashes = line.split(" ");
        let op_h1 = hashes.next();
        let op_h2 = hashes.next();

        //If the nodes aren't already in the graph they will be added an their
        //index will be returned.
        let index1 = try_add_hash(op_h1, &mut graph, &mut indexation);
        let mut index2 = try_add_hash(op_h2, &mut graph, &mut indexation);

        //We can only create an edge, if both are real nodes.
        if index1.is_some() {
            while index2.is_some() {
                if graph
                    .add_edge(index2.unwrap(), index1.unwrap(), ())
                    .is_err()
                {
                    eprintln!("Error while parsing commit graph from git!");
                    return Err(());
                }

                index2 = try_add_hash(hashes.next(), &mut graph, &mut indexation);
            }
        }
    }

    Ok(Radag { graph, indexation })
}

fn try_add_hash(
    op_hash: Option<&str>,
    dag: &mut Dag<String, ()>,
    added: &mut HashMap<String, NodeIndex>,
) -> Option<NodeIndex> {
    let hash = op_hash?;

    if !added.contains_key(hash) {
        let index = dag.add_node(String::from(hash));
        added.insert(String::from(hash), index);

        Some(index)
    } else {
        //UNWRAP: We checked before, that added has this key.
        Some(*added.get(hash).unwrap())
    }
}
