use std::{env, fs};

use nisq::{nisq_solve, NisqArchitecture};
// use raa::{raa_solve, RaaArchitecture};
mod backend;
mod nisq;
mod raa;
mod utils;

fn nisq_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/test.qasm");
    let g = utils::path_graph(10);
    let arch = NisqArchitecture::new(g);
    println!("{:?}", nisq_solve(&circ, &arch));
}

fn raa_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/test.qasm");
    let arch = raa::RaaArchitecture {
        width: 3,
        height: 2,
    };
    println!("{:?}", raa::raa_solve(&circ, &arch));
}

fn run_nisq(circ_path : &str, graph_path : &str) {
    let circ = utils::extract_cnots(circ_path);
    let g = utils::graph_from_file(graph_path);
    let arch = NisqArchitecture::new(g);
    serde_json::to_writer(std::io::stdout(),  &nisq_solve(&circ, &arch)).unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: qmrl <circuit> <graph>");
        return
    }
    run_nisq(&args[1], &args[2])
}
