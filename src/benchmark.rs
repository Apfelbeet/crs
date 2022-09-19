use std::{
    fs::{self, OpenOptions},
    time::Duration,
};

use chrono::Utc;
use std::io::Write;

use crate::{process::ExecutionTime, Args};

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
    times: Vec<(u32, String, ExecutionTime)>,
) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();

    writeln!(&mut file, "overall execution time: {}", overall_duration.as_secs_f64()).unwrap();
    writeln!(&mut file, "---").unwrap();
    writeln!(&mut file, "commit,all,checkout,query").unwrap();
    for (pid, commit, time) in times {
        writeln!(&mut file, "{},{},{},{},{}", pid, commit, time.all.as_secs_f64(), time.checkout.as_secs_f64(), time.query.as_secs_f64()).unwrap();
    }
    file.flush().unwrap();
}
