use std::{
    fs::{self, OpenOptions},
    time::Duration,
};

use chrono::Utc;
use std::io::Write;

use crate::{process::ExecutionData, Args, regression};

pub fn write_header(directory: &std::path::PathBuf, args: &Args) -> std::path::PathBuf {
    let date = Utc::now();
    let file_name = date.format("%Y%m%d_%H%M%S").to_string();
    let mut path = directory.clone();

    if !directory.is_dir() {
        panic!("{} isn't a directory", directory.display());
    }

    let header = format!(
        "/** Header
* date: {}
* repository: {}
* test: {}
* worktree location: {:?}
* processes: {},
* no propagate: {},
* search mode: {},
* scheduling: {}, 
* start: {},
* targets: {:?},
*/
---
",
        date.format("%Y-%m-%d %H:%M:%S").to_string(),
        args.repository.display(),
        args.test.display(),
        args.worktree_location,
        args.processes,
        args.no_propagate,
        args.search_mode,
        regression::NAME,
        args.start,
        args.targets
    );

    path.set_file_name(file_name);
    fs::write(&path, &header).expect("Couldn't create benchmark file!");
    path
}

pub fn write_data(
    path: std::path::PathBuf,
    overall_duration: Duration,
    times: Vec<(u32, String, ExecutionData)>,
) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();

    writeln!(&mut file, "overall execution time: {}", overall_duration.as_secs_f64()).unwrap();
    writeln!(&mut file, "---").unwrap();
    writeln!(&mut file, "commit,all,setup,query,distance").unwrap();
    let mut all_sum = 0.0;
    let mut setup_sum = 0.0;
    let mut query_sum = 0.0;
    let mut diff_sum = 0;
    let len = times.len();
    for (pid, commit, time) in times {
        all_sum += time.all.as_secs_f64();
        setup_sum += time.setup.as_secs_f64();
        query_sum += time.query.as_secs_f64();
        diff_sum += time.diff as u128;

        writeln!(&mut file, "{},{},{},{},{},{}", pid, commit, time.all.as_secs_f64(), time.setup.as_secs_f64(), time.query.as_secs_f64(), time.diff).unwrap();
    }
    writeln!(&mut file, "---").unwrap();
    writeln!(&mut file, "-,-,{},{},{},{}", all_sum, setup_sum, query_sum, diff_sum).unwrap();
    writeln!(&mut file, "-,-,{},{},{},{}", all_sum / len as f64, setup_sum / len as f64, query_sum / len as f64, diff_sum / len as u128).unwrap();
    file.flush().unwrap();
}
