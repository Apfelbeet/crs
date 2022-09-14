mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;

use dvcs::{DVCS, benchmark::{NormalDistribution, TimeProfile, Benchmark, parse_targets}};
use regression::{rpa::{RPA, Settings}, binary_search::BinarySearch};

use crate::manage::start;
use clap::Parser;

/**
 * BENCHMARK OVERRIDE
 */
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(parse(from_os_str))]
    graph_path: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    targets_path: std::path::PathBuf,
    #[clap(parse(from_os_str))]
    time_profile: std::path::PathBuf,
}

fn main() {

    let args = Args::parse();

    let raw_time_profile = std::fs::read_to_string(args.time_profile).unwrap();
    let time_profile_json = json::parse(&raw_time_profile).unwrap();

    let dis_good = NormalDistribution {
        mean: time_profile_json["success_distribution"]["avg"].as_f64().unwrap(),
        std_dev: time_profile_json["success_distribution"]["std"].as_f64().unwrap(),
    };

    let dis_bad = NormalDistribution {
        mean: time_profile_json["failure_distribution"]["avg"].as_f64().unwrap(),
        std_dev: time_profile_json["failure_distribution"]["std"].as_f64().unwrap(),
    };

    let time_profile = TimeProfile{
        valid: dis_good,
        invalid: dis_bad,
    };

    let targets = parse_targets(args.targets_path);
    

    Benchmark::reset(args.graph_path, time_profile, Some(0));
    
    let g = Benchmark::commit_graph("/mnt/i/Tum/22_BT/implementations/benchmarks/data/data_ace_cleaned.dot").unwrap();
    let root = g.root.clone();
    let mut c = RPA::<BinarySearch, ()>::new(g, root, targets, Settings { propagate: true });
    start::<_, Benchmark>(&mut c, "", 4, "", None);
    
}