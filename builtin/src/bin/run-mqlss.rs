use solver::utils::{self, IOError};
use builtin::mqlss;
use serde_json;


fn run_mqlss(circ_path: &str, arch_type : &str) -> Result<(), IOError> {
    let circ = utils::extract_gates(circ_path, &["Pauli"]);
    let arch = match arch_type {
        "compact" => Ok(builtin::mqlss::compact_layout(circ.qubits.len())),
        "square_sparse" => Ok(builtin::mqlss::square_sparse_layout(circ.qubits.len())),
        _ => Err(IOError::InputErr)
    }?;
    let res = mqlss::mqlss_solve(&circ, &arch);
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError>  {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
    println!("Usage: run-mqlss <circuit> <arch>");
}
    run_mqlss(&args[1], &args[2])
}
