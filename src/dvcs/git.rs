use crate::dvcs::DVCS;
use crate::graph::{Adag, prune_downwards};
use daggy::{Dag, NodeIndex};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::process::Command;
use std::{collections::HashMap, hash::Hash};

use super::{run_command_sync, Worktree};

#[derive(Debug, Clone)]
pub struct Git;

impl DVCS for Git {
    //TODO: Right now when passing a repository as string/path, we assume that
    //the path is valid. We should check that the path
    //is valid.

    fn commit_graph(
        repository: &str,
        sources: Vec<String>,
        targets: Vec<String>,
    ) -> Result<Adag<String, ()>, ()> {
        let mut command = Command::new("git");
        command
            .args(["rev-list", "--parents"])
            .args(targets.clone());

        let rev_list = match run_command_sync(repository, &mut command) {
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

        return parse_rev_list(rev_list?, &sources, &targets);
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
        Ok(_) => {},
        Err(err) => panic!("git panicked {}", err),
    }

    match run_command_sync(&worktree.location, &mut command_reset) {
        Ok(_) => {},
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

fn parse_rev_list(rev_list: String, source_hashes: &Vec<String>, targets: &Vec<String>) -> Result<Adag<String, ()>, ()> {
    let mut indexation = HashMap::new();
    let mut graph = Dag::new();

    println!("{}", rev_list);
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

    let source_indices: Vec<NodeIndex> = source_hashes.iter().filter_map(|h| indexation.get(h).cloned()).collect();
    let (pruned_graph, indexation2) = prune_downwards(&graph, &source_indices);

    if source_indices.len() == 0 {
        eprintln!("Graph has no source!");
        return Err(());
    }

    let sources = source_hashes.iter().filter(|h| indexation2.contains_key(&h.to_string())).cloned().collect();

    Ok(
        Adag {
            sources: sources,
            targets: targets.clone(),
            graph: pruned_graph,
            indexation: indexation2,
        }
    )
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
