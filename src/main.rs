use nisq::{nisq_solve, NisqArchitecture};

mod backend;
mod nisq;
mod utils;
fn main() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/mod10_171.qasm");
    let g = utils::path_graph(10);
    let arch = NisqArchitecture::new(g);
    println!("{:?}", nisq_solve(&circ, &arch));
}
