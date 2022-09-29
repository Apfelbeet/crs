use crate::benchmark::write_data;
use crate::dvcs::DVCS;
use crate::process::{ExecutionData, LocalProcess, ProcessError, ProcessResponse};
use crate::regression::{RegressionAlgorithm, TestResult};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::mpsc::{self, RecvError, TryRecvError};
use std::time::Instant;

pub struct Options {
    pub worktree_location: Option<String>,
    pub benchmark_location: Option<std::path::PathBuf>,
    pub do_interrupt: bool,
}
struct ProcessPool<T> {
    next_id: u32,
    empty_slots: u32,
    idle_processes: Vec<LocalProcess<T>>,
    active_processes: HashMap<u32, LocalProcess<T>>,
    commit_to_process: HashMap<String, u32>,
    _marker: PhantomData<T>,
}

struct Stats {
    number_jobs: u32,
    interrupted_tests: u32,
}

impl Stats {
    fn new() -> Self {
        Stats {
            number_jobs: 0,
            interrupted_tests: 0,
        }
    }
}

pub fn start<S: RegressionAlgorithm, T: DVCS>(
    core: &mut S,
    repository: &str,
    threads: u32,
    script_path: &str,
    options: Options,
) {
    let mut stats = Stats::new();

    let (send, recv) = mpsc::channel::<ProcessResponse>();

    let mut pool = ProcessPool::<T> {
        next_id: 0,
        empty_slots: threads,
        idle_processes: Vec::new(),
        active_processes: HashMap::new(),
        commit_to_process: HashMap::new(),
        _marker: PhantomData,
    };

    let mut benchmarks_times = Vec::<(u32, String, ExecutionData)>::new();
    let start_time = Instant::now();
    //We assume that there is at least one process available in the first
    //iteration.
    while !core.done() {
        let mut wait = false;
        match core.next_job(pool.idle_processes.len() as u32 + pool.empty_slots) {
            crate::regression::AlgorithmResponse::Job(commit) => {
                let setup_time = Instant::now();
                let process = load_process(
                    &mut pool,
                    repository,
                    options.worktree_location.clone(),
                    &commit,
                );
                process.run(commit, send.clone(), script_path.to_string(), setup_time);
                stats.number_jobs += 1;
            }
            crate::regression::AlgorithmResponse::WaitForResult => {
                wait = true;

                if pool.active_processes.is_empty() {
                    eprintln!("Algorithms suggests to wait, but there is nothing to wait for!");
                    break;
                }
            }
            crate::regression::AlgorithmResponse::InternalError(msg) => {
                eprintln!("{}", msg);
                break;
            }
        };

        if wait || (pool.idle_processes.is_empty() && pool.empty_slots == 0) {
            match recv_response(&recv, &mut pool) {
                Ok(res) => {
                    if !process_response(
                        res,
                        core,
                        &mut benchmarks_times,
                        &mut stats,
                        &mut pool,
                        options.do_interrupt,
                    ) {
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    break;
                }
            }
        }

        let mut stop = false;
        loop {
            match try_recv_response(&recv, &mut pool) {
                Ok(res) => {
                    if !process_response(
                        res,
                        core,
                        &mut benchmarks_times,
                        &mut stats,
                        &mut pool,
                        options.do_interrupt,
                    ) {
                        stop = true;
                        break;
                    }
                }
                Err(err) => match err {
                    TryRecvError::Empty => break,
                    TryRecvError::Disconnected => {
                        eprintln!("Receiver disconnected!");
                        stop = true;
                        break;
                    }
                },
            }
        }

        if stop {
            break;
        }
    } //END LOOP

    let overall_execution_time = start_time.elapsed();
    if let Some(benchmark_location) = options.benchmark_location {
        write_data(benchmark_location, overall_execution_time, benchmarks_times)
    }

    //Wait for active processes to be done and clean up.
    eprintln!("Wait for active processes to finish!");
    while !pool.active_processes.is_empty() {
        let _ = recv_response(&recv, &mut pool);
    }
    for process in pool.idle_processes {
        process.clean_up();
    }

    let points = core.results();

    println!("---- STATS ----\n");
    println!("Commits tested: {}", stats.number_jobs);
    println!("Regression points: {}", points.len());
    println!(
        "Runtime (seconds): {}",
        overall_execution_time.as_secs_f32()
    );
    println!("\n----\n");

    for point in points {
        println!("Target: {}", point.target);
        println!("Regression Point: {}", point.regression_point);
        if let Some(message) = T::get_commit_info(repository, &point.regression_point) {
            println!("{}", message);
        }
        println!("----");
    }
}

fn process_response<'a, S: RegressionAlgorithm, T: DVCS>(
    (pid, commit, res): ProcessResponse,
    core: &mut S,
    benchmarks_times: &mut Vec<(u32, String, ExecutionData)>,
    stats: &mut Stats,
    pool: &'a mut ProcessPool<T>,
    do_interrupt: bool,
) -> bool {
    match res {
        Ok((result, times)) => {
            benchmarks_times.push((pid, commit.clone(), times));
            core.add_result(commit, result);
        }
        Err(err) => match err {
            ProcessError::Interrupt => {
                eprintln!("{} interrupted", commit);
                stats.interrupted_tests += 1;
            }
            ProcessError::Code => {
                eprintln!("{} stops execution via exit code", commit);
                return false;
            }
            _ => {
                eprintln!("{} query failed: {:?}", commit, err);
                return false;
            }
        },
    };

    if do_interrupt {
        for commit in core.interrupts() {
            interrupt(&commit, pool);
        }
    }

    return true;
}

fn load_process<'a, T: DVCS>(
    pool: &'a mut ProcessPool<T>,
    repository: &str,
    worktree_location: Option<String>,
    commit: &str,
) -> &'a mut LocalProcess<T> {
    let available_process = if !pool.idle_processes.is_empty() {
        get_nearest_process(pool, commit)
    } else if pool.empty_slots > 0 {
        let process = LocalProcess::new(pool.next_id, repository, worktree_location);
        pool.next_id += 1;
        pool.empty_slots -= 1;
        process
    } else {
        panic!("No free slot for a new process!");
    };

    let id = available_process.id;
    pool.commit_to_process.insert(commit.to_string(), id);
    pool.active_processes.insert(id, available_process);
    pool.active_processes.get_mut(&id).unwrap()
}

fn get_nearest_process<'a, T: DVCS>(pool: &'a mut ProcessPool<T>, commit: &str) -> LocalProcess<T> {
    let mut min_distance = None;
    let mut min_index = None;
    for (i, p) in pool.idle_processes.iter().enumerate() {
        let distance = T::distance(&p.worktree, commit);
        if min_distance.is_none() || min_distance.unwrap() > distance {
            min_distance = Some(distance);
            min_index = Some(i);
        }
    }

    pool.idle_processes.remove(min_index.unwrap())
}

fn try_recv_response<T: DVCS>(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool<T>,
) -> Result<
    (
        u32,
        String,
        Result<(TestResult, ExecutionData), ProcessError>,
    ),
    TryRecvError,
> {
    let res = recv.try_recv()?;
    deactivate_process(res.0, &res.1, pool);
    Ok(res)
}

fn recv_response<T: DVCS>(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool<T>,
) -> Result<
    (
        u32,
        String,
        Result<(TestResult, ExecutionData), ProcessError>,
    ),
    RecvError,
> {
    let res = recv.recv()?;
    deactivate_process(res.0, &res.1, pool);
    Ok(res)
}

fn deactivate_process<T: DVCS>(id: u32, commit: &str, pool: &mut ProcessPool<T>) {
    let process = pool
        .active_processes
        .remove(&id)
        .expect(format!("Couldn't find process {} in pool of active processes!", id).as_str());
    pool.commit_to_process.remove(commit);

    pool.idle_processes.push(process);
}

fn interrupt<T: DVCS>(commit: &str, pool: &mut ProcessPool<T>) {
    let id_ = pool.commit_to_process.get(commit);
    if let Some(id) = id_ {
        let process_ = pool.active_processes.get_mut(&id);
        if let Some(process) = process_ {
            process.interrupt();
        }
    }
}
