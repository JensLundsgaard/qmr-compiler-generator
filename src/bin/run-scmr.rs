use qmrl::{scmr, structures::Architecture, utils};

fn run_scmr(circ_path: &str) {
    let circ = utils::extract_scmr_gates(circ_path);
    let arch = utils::compact_layout(circ.qubits.len());
    let res = scmr::scmr_solve(&circ, &arch);
    match serde_json::to_writer(std::io::stdout(), &res) {
        Ok(_) => (),
        Err(e) => panic!("Error writing compilation to stdout: {}", e),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: run-scmr <circuit>");
        return;
    }
    run_scmr(&args[1]);
}
