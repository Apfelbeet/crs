use crate::dvcs::DVCS;
use crate::graph::{prune_downwards, Adag};
use daggy::{Dag, NodeIndex};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::process::{Command, Output};
use std::{collections::HashMap, hash::Hash};

use super::{run_command_sync, Worktree};

#[derive(Debug, Clone)]
pub struct Git;

impl DVCS for Git {
    fn commit_graph(
        repository: &str,
        sources: Vec<String>,
        targets: Vec<String>,
    ) -> Result<Adag<String, ()>, ()> {
        let mut graph = Dag::<String, ()>::new();
        let mut indexation = HashMap::<String, NodeIndex>::new();

        let lca = if sources.len() > 1 {
            let mut lca_command = Command::new("git");
            lca_command
                .arg("merge-base")
                .arg("--octopus")
                .args(&sources);

            handle_result(run_command_sync(repository, &mut lca_command))
        } else if sources.len() == 1 {
            Ok(sources[0].clone())
        } else {
            print_error("Missing source!");
            Err(())
        };

        let mut rev_command = Command::new("git");
        rev_command
            .args(["rev-list", "--parents"])
            .args(&targets)
            .arg("--not")
            .arg(lca?);

        let rev_list = handle_result(run_command_sync(repository, &mut rev_command));
        add_rev_list(&mut graph, &mut indexation, rev_list?);

        let source_indices = sources
            .iter()
            .filter_map(|h| indexation.get(h).cloned())
            .collect();
        let (pruned_graph, pruned_indexation) = prune_downwards(&graph, &source_indices);
        let remaining_sources = sources
            .iter()
            .filter(|h| pruned_indexation.contains_key(*h))
            .cloned()
            .collect();
        let remaining_targets = targets
            .iter()
            .filter(|h| pruned_indexation.contains_key(*h))
            .cloned()
            .collect();

        return Ok(Adag {
            graph: pruned_graph,
            indexation: pruned_indexation,
            sources: remaining_sources,
            targets: remaining_targets,
        });
    }

    fn create_worktree(
        repository: &str,
        name: &str,
        external_location: Option<String>,
    ) -> Result<super::Worktree, ()> {
        let wt_name = match &external_location {
            Some(loc) => {
                let mut s = DefaultHasher::new();
                loc.hash(&mut s);
                let hash = s.finish().to_string();
                format!("{}_{}", hash, name)
            }
            None => format!("{}", name),
        };

        let location = match &external_location {
            Some(loc) => format!("{}/{}", loc, wt_name),
            None => format!("{}/.crs/{}", repository, wt_name),
        };

        let worktree = super::Worktree {
            location: location.clone(),
            name: wt_name.clone(),
        };

        if !worktree_exists(repository, &wt_name) {
            let mut command = Command::new("git");

            command.args(["worktree", "add", "--detach", &location, "--no-checkout"]);

            match run_command_sync(repository, &mut command) {
                Ok(output) => {
                    if output.status.success() {
                        Ok(worktree)
                    } else {
                        print_error(String::from_utf8(output.stderr).unwrap().as_str());
                        Err(())
                    }
                }
                Err(e) => {
                    print_error(e.to_string().as_str());
                    Err(())
                }
            }
        } else {
            Ok(worktree)
        }
    }

    fn remove_worktree(worktree: &Worktree) -> Result<(), ()> {
        let mut rm_tree = Command::new("git");
        rm_tree.args(["worktree", "remove", worktree.name.as_str()]);

        worktree_clean(worktree);

        return match run_command_sync(&worktree.location, &mut rm_tree) {
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

    fn checkout(worktree: &Worktree, commit: &str) -> Result<(), ()> {
        let mut command = Command::new("git");
        command.args(["checkout", "-f", commit]);

        worktree_clean(worktree);

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

    fn get_commit_info(repository: &str, commit: &str) -> Option<String> {
        let mut command = Command::new("git");
        command.args(["log", "--pretty=reference", "-n", "1", commit]);

        match run_command_sync(repository, &mut command) {
            Ok(output) => {
                if output.status.success() {
                    match String::from_utf8(output.stdout) {
                        Ok(message) => Some(message),
                        Err(err) => {
                            eprintln!("couldn't parse response for commit information ({}) from git {:#?}", commit, err);
                            None
                        }
                    }
                } else {
                    match String::from_utf8(output.stderr) {
                        Ok(message) => eprintln!(
                            "git panicked while fetching commit information ({}): {}",
                            commit, message
                        ),
                        Err(_) => eprintln!(
                            "git panicked while fetching commit information ({})",
                            commit
                        ),
                    };
                    None
                }
            }
            Err(err) => {
                eprintln!(
                    "couldn't fetch commit information ({}) from git: {:#?}",
                    commit, err
                );
                None
            }
        }
    }

    fn distance(worktree: &Worktree, commit: &str) -> u32 {
        let mut command = Command::new("git");
        command.args(["diff", "--numstat", "HEAD", commit]);

        match run_command_sync(&worktree.location, &mut command) {
            Ok(output) => {
                if output.status.success() {
                    let text = String::from_utf8(output.stdout).unwrap();
                    let mut sum = 0;
                    for line in text.lines() {
                        let parts = line.split_whitespace();
                        for (i, part) in parts.enumerate() {
                            if let Ok(number) = part.parse::<u32>() {
                                sum += number;
                            }
                            if i == 1 {
                                break;
                            }
                        }
                    }
                    sum
                } else {
                    panic!("git panicked {}", String::from_utf8(output.stderr).unwrap())
                }
            }
            Err(err) => panic!("git panicked {}", err),
        }
    }
}

fn worktree_clean(worktree: &Worktree) {
    let mut command_clean = Command::new("git");
    command_clean.args(["clean", "-d", "-f", "-x"]);

    let mut command_reset = Command::new("git");
    command_reset.args(["restore", "."]);

    match run_command_sync(&worktree.location, &mut command_clean) {
        Ok(_) => {}
        Err(err) => panic!("git panicked {}", err),
    }

    match run_command_sync(&worktree.location, &mut command_reset) {
        Ok(_) => {}
        Err(err) => panic!("git panicked {}", err),
    }
}

fn worktree_exists(location: &str, name: &str) -> bool {
    let mut command = Command::new("git");

    command.args(["worktree", "list", "--porcelain"]);

    match run_command_sync(location, &mut command) {
        Ok(output) => {
            if output.status.success() {
                let response = String::from_utf8(output.stdout).unwrap();
                response.find(name).is_some()
            } else {
                panic!("{}", String::from_utf8(output.stderr).unwrap().as_str());
            }
        }
        Err(err) => {
            panic!("{}", err.to_string().as_str())
        }
    }
}

fn print_error(msg: &str) {
    eprintln!("Git Error: {}", msg);
}

fn add_rev_list(
    graph: &mut Dag<String, ()>,
    indexation: &mut HashMap<String, NodeIndex>,
    rev_list: String,
) {
    let lines = rev_list.lines();

    for line in lines {
        let mut hashes = line.split(" ");
        let op_h1 = hashes.next();
        let op_h2 = hashes.next();

        let index1 = try_add_hash(op_h1, graph, indexation);
        let mut index2 = try_add_hash(op_h2, graph, indexation);

        if let Some(i1) = index1 {
            while index2.is_some() {
                if graph.update_edge(index2.unwrap(), i1, ()).is_err() {
                    panic!("Error while parsing commit graph from git!");
                }
                index2 = try_add_hash(hashes.next(), graph, indexation);
            }
        }
    }
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

fn handle_result(res: std::io::Result<Output>) -> Result<String, ()> {
    match res {
        Ok(output) => {
            if output.status.success() {
                match String::from_utf8(output.stdout) {
                    Ok(r) => Ok(r.trim().to_string()),
                    Err(_) => Err(()),
                }
            } else {
                print_error(String::from_utf8(output.stderr).unwrap().as_str());
                Err(())
            }
        }
        Err(err) => {
            print_error(err.to_string().as_str());
            Err(())
        }
    }
}
