mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;

use dvcs::{git::Git, DVCS};
use regression::{rpa::RPA};

use crate::manage::start;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
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
}

fn main() {

    let args = Args::parse();

    let repository_path = &args.repository.display().to_string();
    let test_path = &args.test.display().to_string();

    let g = Git::commit_graph(repository_path).unwrap();
    let mut rpa = RPA::new(g, args.start, args.targets);
    start::<_, Git>(&mut rpa, repository_path, args.processes, test_path);
}