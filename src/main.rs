mod dvcs;
mod manage;
mod regression;
mod process;
mod graph;

use dvcs::{git::Git, DVCS};
use regression::{rpa::RPA};

use crate::manage::start;



fn main() {

    // let root = "73d930e2a2219de39fc9ccf4fbc326ab7d2a8e7e".to_string();
    // let leaves = vec![
    //     "095f43d373b3aabc275e7575fc9b99c70105c143".to_string(),
    //     "20789d338c08157799e3708d770f24ada297aa24".to_string(),
    // ];

    let root = "3650efd8c0cb03d469bb7e6a2ba5b14bbdf1522c".to_string();
    // let path = vec![
    //     "73d930e2a2219de39fc9ccf4fbc326ab7d2a8e7e".to_string(),
    //     "64b69016253f47c3d83d18b6d51107fbd788290c".to_string(),
    //     "d9079cbd6b172047e7d72285909b142929130469".to_string(),
    //     "0cdefd145a250736be3b63e351bfbbf4e3895689".to_string(),
    //     "105b9684ebbe2b92688716c082a33ba58fbe0e9d".to_string(),
    //     "1d2dff873bac48881a5fd9c2f6df321a21addc5d".to_string(),
    //     "3f3c61bc33cead2d3c2eebc0c3365353cae4abe8".to_string(),
    //     "820481578e175387393bc3a4b178a0d6b2feb69e".to_string(),
    //     "147b12150c34d45696a5f825fb43d4e25495a177".to_string(),
    //     "561a70aa773a0c667e03d84b3545ceacbc8e5bc0".to_string(),
    //     "b5af1df01ffce8fb0c57507a6462f448db2d93a6".to_string(),
    //     "4c16477ec94b396905f638f30cadf11ee5de20f4".to_string(),
    //     "095f43d373b3aabc275e7575fc9b99c70105c143".to_string(),
    // ];
        
    let targets = vec![
        "cb095c35a17fc2799f7f4d8a9733915efe635c7e".to_string(),
        "20789d338c08157799e3708d770f24ada297aa24".to_string(),
    ];

    let rep = "/mnt/i/Tum/22_BT/temp_repos/tournament-scheduler";
    let g = Git::commit_graph(rep).unwrap();
    let mut rpa = RPA::new(g, root, targets);
    start::<_, Git>(&mut rpa, rep, 2, "/home/matthias/crs_test.sh");
}