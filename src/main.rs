mod dvcs;
mod manage;
mod regression;
mod process;
// use daggy::petgraph::dot::{Dot, Config};

use dvcs::git::Git;

use crate::manage::start;



fn main() {

    let root = "3650efd8c0cb03d469bb7e6a2ba5b14bbdf1522c".to_string();
    let leaves = vec![
        "73d930e2a2219de39fc9ccf4fbc326ab7d2a8e7e".to_string(),
        "1d2dff873bac48881a5fd9c2f6df321a21addc5d".to_string(),
        "820481578e175387393bc3a4b178a0d6b2feb69e".to_string(),
        "30b0d99e3726349d8b7e7b08587c3826f407b8e8".to_string(),
        "90343593fafcba0f6ed066efb1f5b1f5940144bd".to_string(),
        "9b1f93ff70e9870c860ded262a324ea893ba6af4".to_string(),
        // "443f452917f50d04375a03412089fca583b4db8a".to_string(),
        // "1bd7086c5c711ac66670f5fa5688f7c09deec0af".to_string(),
        // "67b49c23803fd5041b7d88fc1d8f1dd252f50027".to_string(),
        // "632a52133289674564831ea69f2132e882b4588b".to_string(),
        // "f71353fc7b94fab7f3b6ebcbe80c6ca52b3d3343".to_string(),
        // "3f3c61bc33cead2d3c2eebc0c3365353cae4abe8".to_string(),
        // "f72c6bb627993a2021bc9443b498867776be439d".to_string(),
        // "20789d338c08157799e3708d770f24ada297aa24".to_string(),
        // "62c1daf8cf6b25c7771ba9d64763c7ebd1c413ce".to_string(),
        // "c3e2cbdfc18331bb2c5b0bc20b186994638d9f8f".to_string(),
    ];

    start(String::from("/mnt/i/Tum/22_BT/temp_repos/tournament-scheduler"), root, leaves, 4);
}