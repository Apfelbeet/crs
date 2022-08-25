use crate::process::{LocalProcess, ProcessResponse};
use crate::regression::iter::Iter;
use crate::regression::{RegressionAlgorithm, TestResult};
use std::collections::HashMap;
use std::sync::mpsc::{self, RecvError, TryRecvError};

struct ProcessPool {
    next_id: u64,
    empty_slots: u64,
    idle_processes: Vec<LocalProcess>,
    active_processes: HashMap<u64, LocalProcess>,
}

pub fn start(repo: String, root: String, leaves: Vec<String>, threads: u64) {
    let mut alg = Iter::new(leaves);
    let (send, recv) = mpsc::channel::<ProcessResponse>();

    let mut pool = ProcessPool {
        next_id: 0,
        empty_slots: threads,
        idle_processes: Vec::new(),
        active_processes: HashMap::new(),
    };

    //We assume that there is at least on process available in the first
    //iteration.
    while !alg.done() {
        let mut wait = false;
        match alg.next_job() {
            crate::regression::AlgorithmResponse::Job(commit) => {
                let process = load_process(&mut pool, repo.to_string());
                process.run(commit, send.clone());
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
            println!("Waiting …");
            match recv_response(&recv, &mut pool) {
                Ok((commit, result)) => alg.add_result(commit, result),
                Err(err) => {
                    eprintln!("{}", err);
                    break;
                }
            }
            println!("Done waiting …");
        }

        loop {
            match try_recv_response(&recv, &mut pool) {
                Ok((commit, result)) => alg.add_result(commit, result),
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
}

fn load_process(pool: &mut ProcessPool, repo: String) -> &LocalProcess {
    let available_process = if !pool.idle_processes.is_empty() {
        pool.idle_processes.pop().unwrap()
    } else if pool.empty_slots > 0 {
        let process = LocalProcess::new(pool.next_id, repo);
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

fn try_recv_response(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool,
) -> Result<(String, TestResult), TryRecvError> {
    let (id, commit, result) = recv.try_recv()?;
    deactivate_process(id, pool);
    Ok((commit, result))
}

fn recv_response(
    recv: &mpsc::Receiver<ProcessResponse>,
    pool: &mut ProcessPool,
) -> Result<(String, TestResult), RecvError> {
    let (id, commit, result) = recv.recv()?;
    deactivate_process(id, pool); //FIXME: If the process panics, it will not be deactivated.
    Ok((commit, result))
}

fn deactivate_process(id: u64, pool: &mut ProcessPool) {
    let process = pool
        .active_processes
        .remove(&id)
        .expect(format!("Couldn't find process {} in pool of active processes!", id).as_str());

    pool.idle_processes.push(process);
}
