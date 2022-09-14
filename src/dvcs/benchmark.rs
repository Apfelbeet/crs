use std::{
    collections::HashMap,
    fs,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
    time::Duration,
};

use daggy::{Dag, Walker};
use priority_queue::DoublePriorityQueue;
use rand::distributions::Distribution;
use regex::Regex;
use statrs::distribution::Normal;
use std::sync::mpsc;

use crate::graph::Radag;

use super::{Worktree, DVCS};

pub struct Benchmark;

pub struct BObject {
    times: HashMap<String, Duration>,
    graph: Radag<String, ()>,
    hash_valid: HashMap<String, bool>,
    jobs: DoublePriorityQueue<String, u128>,
    jobs_transmitter: HashMap<String, Sender<bool>>,
    location_commit: HashMap<String, String>,
    current_time: u128,
}

// static mut times: Option<HashMap<String, Duration>> = None;
// static mut graph: Option<Radag<String, ()>> = None;
// static mut hash_valid: Option<HashMap<String, bool>> = None;
// static mut jobs: Option<DoublePriorityQueue<String, u128>> = None;
// static mut jobs_transmitter: Option<HashMap<String, Sender<bool>>> = None;
// static mut location_commit: Option<HashMap<String, String>> = None;
// static mut current_time: u128 = 0;
static MUTEX_B: Mutex<Option<BObject>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct TimeProfile {
    pub valid: NormalDistribution,
    pub invalid: NormalDistribution,
}

#[derive(Debug, Clone)]
pub struct NormalDistribution {
    pub mean: f64,
    pub std_dev: f64,
}

impl Benchmark {
    pub fn reset(graph_file: std::path::PathBuf, time_profile: TimeProfile) {
        let mut bmo = MUTEX_B.lock().unwrap();
        let raw_graph = fs::read_to_string(graph_file).expect("graph file missing!");

        let (radag, valid) = parse_dot(&raw_graph);

        let mut r = rand::thread_rng();
        let dis_valid = Normal::new(time_profile.valid.mean, time_profile.valid.std_dev).unwrap();
        let dis_invalid =
            Normal::new(time_profile.invalid.mean, time_profile.invalid.std_dev).unwrap();

        let mut l_times = HashMap::<String, Duration>::new();

        for (hash, value) in &valid {
            let time_f = if value.clone() {
                dis_valid.sample(&mut r)
            } else {
                dis_invalid.sample(&mut r)
            };

            let time = time_f.max(1.0) as u64;
            l_times.insert(hash.clone(), Duration::from_millis(time));
        }

        let b = BObject {
            times: l_times,
            graph: radag,
            hash_valid: valid,
            location_commit: HashMap::new(),
            jobs: DoublePriorityQueue::new(),
            jobs_transmitter: HashMap::new(),
            current_time: 0,
        };

        *bmo = Some(b);
    }

    pub fn register_job(location: &str) -> Receiver<bool> {
        let mut bmo = MUTEX_B.lock().unwrap();
        let bm = bmo.as_mut().unwrap();
        let j = &mut bm.jobs;
        let commit = bm.location_commit.get(location).unwrap();
        j.push(
            location.to_string(),
            bm.current_time + bm.times.get(commit).unwrap().as_millis(),
        );

        let (tx, rx) = mpsc::channel::<bool>();
        bm.jobs_transmitter.insert(location.to_string(), tx);
        rx
    }

    pub fn next() {
        
        loop {
            let wait = {
                let mut bmo = MUTEX_B.lock().unwrap();
                let bm = bmo.as_ref().unwrap();
                bm.jobs.is_empty()
            };

            if wait {
                std::thread::sleep(Duration::from_secs(1));
            } else {
                break;
            }
        }
        let mut bmo = MUTEX_B.lock().unwrap();
        let bm = bmo.as_mut().unwrap();
        println!("NEXT {}", bm.current_time);
        let j = &mut bm.jobs;
        let (location, time) = j.pop_min().expect("no more processes!");

        bm.current_time = time;
        println!("It's now {}", bm.current_time);
        let tx = bm.jobs_transmitter.remove(&location).unwrap();
        let commit = bm.location_commit.get(&location).unwrap();
        let val = bm.hash_valid.get(commit).unwrap();
        if let Err(e) = tx.send(val.clone()) {
            panic!("{:?}", e);
        }
    }
}

impl DVCS for Benchmark {
    fn commit_graph(_: &str) -> Result<Radag<String, ()>, ()> {
        let mut bmo = MUTEX_B.lock().unwrap();
        let bm = bmo.as_mut().unwrap();
        Ok(bm.graph.clone())
    }

    fn create_worktree(
        repository: &str,
        name: &str,
        _: Option<String>,
    ) -> Result<super::Worktree, ()> {
        Ok(Worktree {
            location: name.to_string(),
            name: name.to_string(),
        })
    }

    fn remove_worktree(worktree: &super::Worktree) -> Result<(), ()> {
        Ok(())
    }

    fn checkout(worktree: &super::Worktree, commit: &str) -> Result<(), ()> {
        let mut bmo = MUTEX_B.lock().unwrap();
        let bm = bmo.as_mut().unwrap();
        bm.location_commit
            .insert(worktree.location.clone(), commit.to_string());

        Ok(())
    }

    fn get_commit_info(repository: &str, commit: &str) -> Option<String> {
        None
    }
}

//This function is tailored for files from another paper. I is not capable of
//parsing any other dot files.
fn parse_dot(dot: &str) -> (Radag<String, ()>, HashMap<String, bool>) {
    let mut indexation = HashMap::new();
    let mut l_graph = Dag::new();
    let mut h_valid = HashMap::new();

    //Group 1: hash, Group 2: color
    let re_node = Regex::new(r###""(\S*)"\[color="(\S*)"\];"###).unwrap();
    //Group 1: hash1, Group 2: hash2
    let re_edge = Regex::new(r###""(\S*)" -> "(\S*)";"###).unwrap();

    for cap in re_node.captures_iter(dot) {
        let valid = match &cap[2] {
            "green" => true,
            "red" => false,
            &_ => panic!("Invalid color!"),
        };
        let index = l_graph.add_node(cap[1].to_string());
        indexation.insert(cap[1].to_string(), index);
        h_valid.insert(cap[1].to_string(), valid);
    }

    for cap in re_edge.captures_iter(dot) {
        let index1 = indexation.get(&cap[1].to_string()).unwrap().clone();
        let index2 = indexation.get(&cap[2].to_string()).unwrap().clone();
        l_graph.add_edge(index1, index2, ()).unwrap();
    }

    let mut roots = Vec::new();
    for (h, i) in indexation.iter() {
        if l_graph.parents(i.clone()).iter(&l_graph).next().is_none() {
            roots.push(h.clone())
        }
    }

    let rh = if roots.len() == 0 {
        panic!("Graph has no root");
    } else if roots.len() == 1 {
        let root_hash = roots.pop().unwrap();
        root_hash
    } else {
        let root_hash = String::from("<root>");
        let root_index = l_graph.add_node(root_hash.clone());
        indexation.insert(root_hash.clone(), root_index);
        for r in roots {
            let r_index = indexation.get(&r).unwrap().clone();
            l_graph.add_edge(root_index, r_index, ()).unwrap();
            
        }
        root_hash
    };

    (
        Radag {
            graph: l_graph,
            indexation,
            root: rh,
        },
        h_valid,
    )
}

pub fn parse_targets(file: std::path::PathBuf) -> Vec<String> {
    let x = std::fs::read_to_string(file).expect("Target file invalid");
    x.lines()
        .filter(|l| l.len() > 5)
        .map(|l| l.to_string())
        .collect()
}
