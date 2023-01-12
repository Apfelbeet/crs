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

use crate::{
    manage::start,
    regression::{git_bisect::GitBisect, RegressionAlgorithm},
};
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

    let worktree_location = args
        .worktree_location
        .as_ref()
        .map(|path| path.display().to_string());

    let options = Options {
        worktree_location,
        log_location: log_location.clone(),
        do_interrupt: args.interrupt,
    };

    let repo_path = &args.repository.display().to_string();
    let test_path = &args.test.display().to_string();

    eprintln!("Processing commit graph ...");
    let g = Git::commit_graph(repo_path, sources, targets).unwrap();
    eprintln!("Preparing core ...");
    let mut rpa = load_core(&args, g, log_location);
    eprintln!("Starting search ...");

    start::<Git>(rpa.as_mut(), repo_path, args.processes, test_path, options);
}

fn load_core(
    args: &Args,
    graph: graph::Adag<String, ()>,
    log_location: Option<std::path::PathBuf>,
) -> Box<dyn RegressionAlgorithm> {
    let settings = Settings {
        propagate: !args.no_propagate,
        extended_search: !args.no_extended,
    };

    match args.search_mode.as_str() {
        "exrpa-long-bin" => Box::new(RPA::<LongestPath, BinarySearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "exrpa-long-lin" => Box::new(RPA::<LongestPath, LinearSearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "exrpa-long-mul" => Box::new(RPA::<LongestPath, MultiplyingSearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "exrpa-short-bin" => Box::new(RPA::<ShortestPath, BinarySearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "exrpa-short-lin" => Box::new(RPA::<ShortestPath, LinearSearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "exrpa-short-mul" => Box::new(RPA::<ShortestPath, MultiplyingSearch, ()>::new(
            graph,
            settings,
            log_location,
        )),
        "bisect" => Box::new(GitBisect::new(graph, args.processes as usize, log_location)),
        &_ => {
            panic!("Invalid search mode! Pick (exrpa-long-bin, exrpa-long-lin, exrpa-long-mul, exrpa-short-bin, exrpa-short-lin, exrpa-short-mul)");
        }
    }
}
