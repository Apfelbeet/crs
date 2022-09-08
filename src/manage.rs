use crate::dvcs::DVCS;
use crate::process::{LocalProcess, ProcessResponse};
use crate::regression::{RegressionAlgorithm, TestResult, RegressionPoint, AssignedRegressionPoint};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::mpsc::{self, RecvError, TryRecvError};

struct ProcessPool<T> 
 {
    next_id: u32,
    empty_slots: u32,
    idle_processes: Vec<LocalProcess<T>>,
    active_processes: HashMap<u32, LocalProcess<T>>,
    _marker: PhantomData<T>,
}

pub fn start<S: RegressionAlgorithm, T: DVCS>(core: &mut S, repository: &str, threads: u32, script_path: &str, worktree_location: Option<String>) {
    let (send, recv) = mpsc::channel::<ProcessResponse>();

    let mut pool = ProcessPool::<T> {
        next_id: 0,
        empty_slots: threads,
        idle_processes: Vec::new(),
        active_processes: HashMap::new(),
        _marker: PhantomData,
    };

    //We assume that there is at least on process available in the first
    //iteration.
    while !core.done() {
        let mut wait = false;
        match core.next_job(pool.idle_processes.len() as u32 + pool.empty_slots) {
            crate::regression::AlgorithmResponse::Job(commit) => {
                let process = load_process(&mut pool, repository, worktree_location.clone());
                process.run(commit, send.clone(), script_path.to_string());
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
            // println!("Waiting …");
            match recv_response(&recv, &mut pool) {
                Ok((commit, result)) => core.add_result(commit, result),
                Err(err) => {
                    eprintln!("{}", err);
                    break;
                }
            }
            // println!("Done waiting …");
        }

        loop {
            match try_recv_response(&recv, &mut pool) {
                Ok((commit, result)) => core.add_result(commit, result),
                Err(err) => match err {
                    TryRecvError::Empty => break,
                    TryRecvError::Disconnected => {
                        eprintln!("Receiver disconnected!");
                        break;
                    }
                },
            }
        }
    } //END LOOP

   
    //Wait for active processes to be done and clean up.
    println!("Wait for active processes to finish!");
    while !pool.active_processes.is_empty() {
        recv_response(&recv, &mut pool).expect("Process crashed!");
    }
    for process in pool.idle_processes {
        process.clean_up();
    }

    let results = core.results();
    let points: Vec<&AssignedRegressionPoint> = results.iter().filter_map(|reg| {
        if let RegressionPoint::Point(point) = reg {
            Some(point)
        } else {
            None
        }
    }).collect();

    println!("----\n");
    match points.len() {
        0 => println!("No regression point was found!"),
        1 => println!("1 regression point was found!"),
        _ => println!("{} regression points were found!\n", points.len()),
    }

    println!("\ntarget, regression point");
    for point in points {
        println!("\n----\n");
        println!("Target: {}", point.target);
        println!("Regression Point: {}", point.regression_point);
        if let Some(message) = T::get_commit_info(repository, &point.regression_point) {
            println!("{}", message);
        }
        println!("----\n");
    }
}

fn load_process<'a, T: DVCS>(pool: &'a mut ProcessPool<T>, repository: &str, worktree_location: Option<String>) -> &'a LocalProcess<T> {
    let available_process = if !pool.idle_processes.is_empty() {
        pool.idle_processes.pop().unwrap()
    } else if pool.empty_slots > 0 {
        let process = LocalProcess::new(pool.next_id, repository, worktree_location);
        pool.next_id += 1;
        pool.empty_slots -= 1;
        process
    } else {
        panic!("No free slot for a new process!");
    };

    let id = available_process.id;
    pool.active_processes.insert(id, available_process);
    pool.active_processes.get(&id).unwrap()
}

fn try_recv_response<T: DVCS>(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool<T>,
) -> Result<(String, TestResult), TryRecvError> {
    let (id, commit, result) = recv.try_recv()?;
    deactivate_process(id, pool);
    Ok((commit, result))
}

fn recv_response<T: DVCS>(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool<T>,
) -> Result<(String, TestResult), RecvError> {
    let (id, commit, result) = recv.recv()?;
    deactivate_process(id, pool); //FIXME: If the process panics, it will not be deactivated.
    Ok((commit, result))
}

fn deactivate_process<T: DVCS>(id: u32, pool: &mut ProcessPool<T>) {
    let process = pool
        .active_processes
        .remove(&id)
        .expect(format!("Couldn't find process {} in pool of active processes!", id).as_str());

    pool.idle_processes.push(process);
}