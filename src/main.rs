mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;


use dvcs::{git::Git, DVCS};
use regression::rpa::RPA;

use crate::manage::start;



fn main() {

    let root = "73d930e2a2219de39fc9ccf4fbc326ab7d2a8e7e".to_string();
    let leaves = vec![
        "095f43d373b3aabc275e7575fc9b99c70105c143".to_string(),
        "20789d338c08157799e3708d770f24ada297aa24".to_string(),
    ];

    let rep = "/mnt/i/Tum/22_BT/temp_repos/tournament-scheduler";
    let g = Git::commit_graph(rep).unwrap();
    let mut rpa = RPA::new(g, root, leaves);
    start::<_, Git>(&mut rpa, rep, 3, "/home/matthias/crs_test.sh");
}