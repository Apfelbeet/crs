use std::{
    fs::{self, OpenOptions},
    time::Duration,
};

use chrono::Utc;
use std::io::Write;

use crate::{
    process::ProcessResponse,
    regression::{self, RegressionPoint},
    Args,
};

pub struct TemporalLogData {
    all_sum: f64,
    setup_sum: f64,
    query_sum: f64,
    diff_sum: u128,
    len: u32,
}

pub fn write_header(directory: &std::path::PathBuf, args: &Args) -> std::path::PathBuf {
    let date = Utc::now();
    let directory_name = date.format("%Y%m%d_%H%M%S").to_string();
    let mut path = directory.clone();

    if !directory.is_dir() {
        panic!("{} isn't a directory", directory.display());
    }

    let header = format!(
        "date: {}
repository: {}
test: {}
worktree location: {:?}
processes: {},
no propagate: {},
interrupt: {},
search mode: {},
scheduling: {}, 
start: {},
targets: {:?},
---
pid,commit,status,all,setup,query,distance
",
        date.format("%Y-%m-%d %H:%M:%S").to_string(),
        args.repository.display(),
        args.test.display(),
        args.worktree_location,
        args.processes,
        args.no_propagate,
        args.interrupt,
        args.search_mode,
        regression::NAME,
        args.start,
        args.targets
    );

    path = path.join(directory_name);
    fs::create_dir(&path).expect("Couldn't create log directory!");

    fs::write(&summary_path(&path), &header).expect("Couldn't create benchmark file!");
    path
}

pub fn empty() -> TemporalLogData {
    TemporalLogData {
        all_sum: 0.0,
        setup_sum: 0.0,
        query_sum: 0.0,
        diff_sum: 0,
        len: 0,
    }
}

pub fn add_result(
    result: &ProcessResponse,
    path: &std::path::PathBuf,
    log_data: &mut TemporalLogData,
) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(summary_path(&path))
        .unwrap();

    match &result.result {
        Ok((res, exe_data)) => {
            log_data.all_sum += exe_data.all.as_secs_f64();
            log_data.setup_sum += exe_data.setup.as_secs_f64();
            log_data.query_sum += exe_data.query.as_secs_f64();
            log_data.diff_sum += exe_data.diff as u128;
            log_data.len += 1;

            writeln!(
                &mut file,
                "{},{},{},{},{},{},{}",
                result.pid,
                result.commit,
                res,
                exe_data.all.as_secs_f64(),
                exe_data.setup.as_secs_f64(),
                exe_data.query.as_secs_f64(),
                exe_data.diff
            )
            .unwrap();
        }
        Err(err) => {
            writeln!(&mut file, "{},{},{}", result.pid, result.commit, err).unwrap();
        }
    };
}

pub fn write_summary(
    overall_duration: &Duration,
    regression_points: &Vec<RegressionPoint>,
    path: &std::path::PathBuf,
    log_data: &mut TemporalLogData,
) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(summary_path(&path))
        .unwrap();

    writeln!(&mut file, "---").unwrap();

    writeln!(&mut file, "regression point,target").unwrap();
    for reg in regression_points {
        writeln!(&mut file, "{},{}", reg.regression_point, reg.target).unwrap();
    }

    writeln!(&mut file, "---").unwrap();
    writeln!(
        &mut file,
        "-,-,-,{},{},{},{}",
        log_data.all_sum, log_data.setup_sum, log_data.query_sum, log_data.diff_sum
    )
    .unwrap();
    writeln!(
        &mut file,
        "-,-,-,{},{},{},{}",
        log_data.all_sum / log_data.len as f64,
        log_data.setup_sum / log_data.len as f64,
        log_data.query_sum / log_data.len as f64,
        log_data.diff_sum / log_data.len as u128
    )
    .unwrap();

    writeln!(
        &mut file,
        "overall execution time: {}",
        overall_duration.as_secs_f64()
    )
    .unwrap();
}

fn summary_path(path: &std::path::PathBuf) -> std::path::PathBuf {
    path.join("summary")
}
