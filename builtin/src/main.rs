use builtin::{nisq, raa, scmr, mqlss};
use solver::utils;

fn nisq_test() {
    let circ = utils::extract_cnots("/home/abtin/qmrsl/3_17_13.qasm");
    let g = utils::path_graph(3);
    let arch = nisq::NisqArchitecture::new(g);
    let res =  nisq::nisq_solve(&circ, &arch);
    println!("{:?}, {:?}, {:?}", res.cost, res.transitions, res.steps[0].map);
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
    let circ = utils::extract_scmr_gates("/home/abtin/qmrsl/circuits/3_17_13.qasm");
    let arch = scmr::compact_layout(circ.qubits.len());
    println!("{:?}", scmr::scmr_solve(&circ, &arch).cost);
}

fn mqlss_test(){
    let circ = utils::extract_gates("/home/abtin/qmrsl/pbc-circuits/3_17_13.pbc", &["Pauli"]);
    println!("{:?}", circ);
    let arch = mqlss::square_sparse_layout(circ.qubits.len());
    println!("{:?}", mqlss::mqlss_solve(&circ, &arch).cost);

}

fn main() {
    nisq_test();
    // raa_test();
    // mqlss_test();
}
