mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;

use dvcs::{git::Git, DVCS};
use regression::{rpa::{RPA, Settings}, binary_search::BinarySearch, linear_search::LinearSearch, multiplying_search::MultiplyingSearch};

use crate::manage::start;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(parse(from_os_str))]
    repository: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    test: std::path::PathBuf,

    #[clap(short, long, value_parser, value_name = "AMOUNT", default_value_t = 1)]
    processes: u32,
    
    #[clap(short, long, value_parser)]
    start: String,

    #[clap(value_parser, last = true)]
    targets: Vec<String>,

    #[clap(long, action)]
    no_propagate: bool,

    #[clap(parse(from_os_str), long)]
    worktree_location: Option<std::path::PathBuf>,

    #[clap(long, value_parser, value_name = "MODE", default_value = "rpa-binary")]
    search_mode: String,
}

fn main() {

    let args = Args::parse();

    let worktree_location = match args.worktree_location {
        Some(path) => Some(path.display().to_string()),
        None => None,
    };

    let repository_path = &args.repository.display().to_string();
    let test_path = &args.test.display().to_string();

    let g = Git::commit_graph(repository_path).unwrap();
    //TODO: There has to be a nicer way.
    match args.search_mode.as_str() {
        "rpa-binary" => {
            let mut rpa = RPA::<BinarySearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, worktree_location);
        },
        "rpa-linear" => {
            let mut rpa = RPA::<LinearSearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, worktree_location);
        },
        "rpa-multi" => {
            let mut rpa = RPA::<MultiplyingSearch, ()>::new(g, args.start, args.targets, Settings{propagate: !args.no_propagate });
            start::<_, Git>(&mut rpa, repository_path, args.processes, test_path, worktree_location);
        },
        &_ => {
            panic!("Invalid search mode! Pick (rpa-binary, rpa-linear, rpa-multi)");
        }
    };

    
}