mod dvcs;
mod graph;
mod log;
mod manage;
mod process;
mod regression;

use dvcs::{git::Git, DVCS};
use manage::Options;
use regression::{
    binary_search::BinarySearch,
    linear_search::LinearSearch,
    multiplying_search::MultiplyingSearch,
    path_selection::{longest_path::LongestPath, shortest_path::ShortestPath},
    rpa_search::RPA,
    rpa_util::Settings,
};

use crate::{manage::start, regression::git_bisect::GitBisect};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    pub repository: std::path::PathBuf,
    pub test: std::path::PathBuf,

    #[clap(short, long, value_parser, value_name = "AMOUNT", default_value_t = 1)]
    pub processes: u32,

    #[clap(short, long, value_parser)]
    pub source: String,

    #[clap(short, long, value_parser)]
    pub target: String,

    #[clap(long, action)]
    pub no_propagate: bool,

    #[clap(parse(from_os_str), long)]
    pub worktree_location: Option<std::path::PathBuf>,

    #[clap(
        long,
        value_parser,
        value_name = "MODE",
        default_value = "exrpa-long-bin"
    )]
    pub search_mode: String,

    #[clap(parse(from_os_str), short, long, value_name = "DIRECTORY")]
    pub log: Option<std::path::PathBuf>,

    #[clap(long, action)]
    pub interrupt: bool,

    #[clap(long, action)]
    pub no_extended: bool,
}

fn main() {
    let args = Args::parse();

    let sources: Vec<String> = args.source.split(',').map(|s| s.to_string()).collect();
    let targets: Vec<String> = args.target.split(',').map(|s| s.to_string()).collect();

    let log_location = args
        .log
        .as_ref()
        .map(|b_dir| log::write_header(b_dir, &args, &sources, &targets));

    let worktree_location = args.worktree_location.map(|path| path.display().to_string());

    let options = Options {
        worktree_location,
        log_location: log_location.clone(),
        do_interrupt: args.interrupt,
    };

    let repo_path = &args.repository.display().to_string();
    let test_path = &args.test.display().to_string();

    eprintln!("Processing commit graph ...");
    let g = Git::commit_graph(repo_path, sources, targets).unwrap();
    // TODO: There has to be a nicer way.
    eprintln!("Starting search ...");
    match args.search_mode.as_str() {
        "exrpa-long-bin" => {
            let mut rpa = RPA::<LongestPath, BinarySearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "exrpa-long-lin" => {
            let mut rpa = RPA::<LongestPath, LinearSearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "exrpa-long-mul" => {
            let mut rpa = RPA::<LongestPath, MultiplyingSearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "exrpa-short-bin" => {
            let mut rpa = RPA::<ShortestPath, BinarySearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "exrpa-short-lin" => {
            let mut rpa = RPA::<ShortestPath, LinearSearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "exrpa-short-mul" => {
            let mut rpa = RPA::<ShortestPath, MultiplyingSearch, ()>::new(
                g,
                Settings {
                    propagate: !args.no_propagate,
                    extended_search: !args.no_extended,
                },
                log_location
            );
            start::<_, Git>(&mut rpa, repo_path, args.processes, test_path, options);
        }
        "bisect" => {
            let mut bisect = GitBisect::new(g, args.processes as usize, log_location);
            start::<_, Git>(&mut bisect, repo_path, args.processes, test_path, options)
        }
        &_ => {
            panic!("Invalid search mode! Pick (exrpa-long-bin, exrpa-long-lin, exrpa-long-mul, exrpa-short-bin, exrpa-short-lin, exrpa-short-mul)");
        }
    };
}
