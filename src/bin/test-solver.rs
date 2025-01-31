use qmrl::{nisq, raa, scmr, utils};
include!(concat!(env!("OUT_DIR"), "/custom.rs"));

fn nisq_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/3_17_13.qasm");
    let g = utils::path_graph(10);
    let arch = nisq::NisqArchitecture::new(g);
    println!("{:?}", nisq::nisq_solve(&circ, &arch));
}

fn custom_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/3_17_13.qasm");
    let g = utils::path_graph(10);
    let arch = CustomArch::new(g);
    println!("{:?}", my_solve(&circ, &arch));
}

fn raa_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/3_17_13.qasm");
    let arch = raa::RaaArchitecture {
        width: 3,
        height: 2,
    };
    println!("{:?}", raa::raa_solve(&circ, &arch));
}

fn scmr_test() {
    let circ = utils::extract_scmr_gates("/home/abtin/qmrsl/3_17_13.qasm");
    let arch = utils::compact_layout(circ.qubits.len());
    println!("{:?}", scmr::scmr_solve(&circ, &arch));
}

fn main() {
    custom_test();
    // raa_test();
    // scmr_test();
}
