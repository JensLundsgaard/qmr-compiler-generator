use nisq::{nisq_solve, NisqArchitecture};

mod backend;
mod nisq;
mod utils;
fn main() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/3_17_13.qasm");
    let g = utils::path_graph(3);
    let arch = NisqArchitecture::new(g);
    println!("{:?}", nisq_solve(&circ, &arch));
}
