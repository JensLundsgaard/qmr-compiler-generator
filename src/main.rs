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

fn main() {
    raa_test();
    nisq_test();
}
