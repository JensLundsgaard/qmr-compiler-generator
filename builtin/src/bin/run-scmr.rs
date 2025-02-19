use solver::utils::{self, IOError};
use builtin::scmr;
use serde_json;


fn run_scmr(circ_path: &str, arch_type : &str) -> Result<(), IOError> {
    let circ = utils::extract_scmr_gates(circ_path);
    let arch = match arch_type {
        "compact" => Ok(builtin::scmr::compact_layout(circ.qubits.len())),
        "square_sparse" => Ok(builtin::scmr::compact_layout(circ.qubits.len())),
        _ => Err(IOError::InputErr)
    }?;
    let res = scmr::scmr_solve(&circ, &arch);
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError>  {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
    println!("Usage: run-scmr <circuit> <arch>");
}
    run_scmr(&args[1], &args[2])
}
