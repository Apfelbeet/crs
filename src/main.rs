mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;
mod benchmark;

use dvcs::{git::Git, DVCS};
use manage::Options;
use regression::{rpa::{RPA, Settings}, binary_search::BinarySearch, linear_search::LinearSearch, multiplying_search::MultiplyingSearch};

use crate::manage::start;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    #[clap(parse(from_os_str))]
    pub repository: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    pub test: std::path::PathBuf,

    #[clap(short, long, value_parser, value_name = "AMOUNT", default_value_t = 1)]
    pub processes: u32,
    
    #[clap(short, long, value_parser)]
    pub start: String,

    #[clap(value_parser, last = true)]
    pub targets: Vec<String>,

    #[clap(long, action)]
    pub no_propagate: bool,

    #[clap(parse(from_os_str), long)]
    pub worktree_location: Option<std::path::PathBuf>,

    #[clap(long, value_parser, value_name = "MODE", default_value = "rpa-binary")]
    pub search_mode: String,

    #[clap(parse(from_os_str), long, value_name = "DIRECTORY")]
    pub benchmark: Option<std::path::PathBuf>,

    #[clap(long, action)]
    pub interrupt: bool,
}

fn main() {

    let args = Args::parse();

    let benchmark_location = args.benchmark.as_ref().map(|b_dir| benchmark::write_header(b_dir, &args));

    let worktree_location = match args.worktree_location {
        Some(path) => Some(path.display().to_string()),
        None => None,
    };

    let options = Options {
        worktree_location,
        benchmark_location,
        do_interrupt: args.interrupt,
    };

    let repository_path = &args.repository.display().to_string();
    let test_path = &args.test.display().to_string();

    let g = Git::commit_graph(repository_path, vec![args.start.clone()], args.targets.clone()).unwrap();
    // TODO: There has to be a nicer way.
    match args.search_mode.as_str() {
        "rpa-binary" => {
            let mut rpa = RPA::<BinarySearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, options);
        },
        "rpa-linear" => {
            let mut rpa = RPA::<LinearSearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, options);
        },
        "rpa-multi" => {
            let mut rpa = RPA::<MultiplyingSearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, options);
        },
        &_ => {
            panic!("Invalid search mode! Pick (rpa-binary, rpa-linear, rpa-multi)");
        }
    };

    
}