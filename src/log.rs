use std::{
    fs::{self, OpenOptions, create_dir_all},
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
    len: u32,
}

pub fn write_header(directory: &std::path::Path, args: &Args, sources: &Vec<String>, targets: &Vec<String>) -> std::path::PathBuf {
    let date = Utc::now();
    let directory_name = date.format("%Y%m%d_%H%M%S").to_string();

    let header = format!(
        "date: {}
repository: {}
test: {}
worktree location: {:?}
processes: {},
no propagate: {},
interrupt: {},
no extended: {}
search mode: {},
scheduling: {}, 
start: {:?},
targets: {:?},
",
        date.format("%Y-%m-%d %H:%M:%S"),
        args.repository.display(),
        args.test.display(),
        args.worktree_location,
        args.processes,
        args.no_propagate,
        args.interrupt,
        args.no_extended,
        args.search_mode,
        regression::NAME,
        sources,
        targets
    );

    let header2 = "pid,commit,status,all,setup,query\n";

    let inner_path = directory.join(directory_name);
    fs::create_dir_all(&inner_path).expect("Couldn't create log directory!");
    fs::create_dir_all(output_path(&inner_path)).expect("Couldn't create log directory!");

    fs::write(summary_path(&inner_path), header).expect("Couldn't create benchmark file!");
    fs::write(query_path(&inner_path), header2).expect("Couldn't create benchmark file!");
    inner_path
}

pub fn empty() -> TemporalLogData {
    TemporalLogData {
        all_sum: 0.0,
        setup_sum: 0.0,
        query_sum: 0.0,
        len: 0,
    }
}

pub fn add_result(
    result: &ProcessResponse,
    path: &std::path::Path,
    log_data: &mut TemporalLogData,
) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(query_path(path))
        .unwrap();

    match &result.result {
        Ok((res, exe_data)) => {
            log_data.all_sum += exe_data.all.as_secs_f64();
            log_data.setup_sum += exe_data.setup.as_secs_f64();
            log_data.query_sum += exe_data.query.as_secs_f64();
            log_data.len += 1;

            writeln!(
                &mut file,
                "{},{},{},{},{},{}",
                result.pid,
                result.commit,
                res,
                exe_data.all.as_secs_f64(),
                exe_data.setup.as_secs_f64(),
                exe_data.query.as_secs_f64(),
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
    path: &std::path::Path,
    log_data: &mut TemporalLogData,
) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(query_path(path))
        .unwrap();

    writeln!(&mut file, "---").unwrap();

    writeln!(&mut file, "regression point,target").unwrap();
    for reg in regression_points {
        writeln!(&mut file, "{},{}", reg.regression_point, reg.target).unwrap();
    }

    writeln!(&mut file, "---").unwrap();
    writeln!(
        &mut file,
        "-,-,-,{},{},{}",
        log_data.all_sum, log_data.setup_sum, log_data.query_sum
    )
    .unwrap();
    writeln!(
        &mut file,
        "-,-,-,{},{},{}",
        log_data.all_sum / log_data.len as f64,
        log_data.setup_sum / log_data.len as f64,
        log_data.query_sum / log_data.len as f64,
    )
    .unwrap();

    writeln!(
        &mut file,
        "overall execution time: {}",
        overall_duration.as_secs_f64()
    )
    .unwrap();
}

fn summary_path(path: &std::path::Path) -> std::path::PathBuf {
    path.join("arguments")
}

fn query_path(path: &std::path::Path) -> std::path::PathBuf {
    path.join("queries")
}

pub fn output_path(path: &std::path::Path) -> std::path::PathBuf {
    path.join("output")
}

pub fn add_dir(name: &str, path: &std::path::Path) -> std::path::PathBuf {
    let new_path = path.join(name);
    create_dir_all(&new_path).expect("couldn't create log directory");
    new_path
}

pub fn create_file(name: &str, path: &std::path::Path) -> std::path::PathBuf {
    let new_path = path.join(name);
    fs::File::create(&new_path).unwrap_or_else(|_| panic!("Creating {:?} failed", &new_path));
    new_path
}

pub fn write_to_file(text: &str, path: &std::path::PathBuf) {
    let mut file = OpenOptions::new()
    .append(true)
    .open(path)
    .unwrap();
    
    file.write_all(text.as_bytes()).unwrap_or_else(|_| panic!("Couldn't write to {:?}", path));
}