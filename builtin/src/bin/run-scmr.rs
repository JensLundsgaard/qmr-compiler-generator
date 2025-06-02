use builtin::scmr::{scmr_solve, scmr_solve_joint_optimize_parallel, scmr_solve_par};
use serde_json;
use solver::utils::{self, IOError};

fn run_scmr(circ_path: &str, arch_type: &str, solve_mode: &str) -> Result<(), IOError> {
    let circ = utils::extract_scmr_gates(circ_path);
    let arch = match arch_type {
        "compact" => Ok(builtin::scmr::compact_layout(circ.qubits.len())),
        "square_sparse" => Ok(builtin::scmr::square_sparse_layout(circ.qubits.len())),
        _ => Err(IOError::InputErr),
    }?;
    let res = match solve_mode {
        "--onepass" => Ok(scmr_solve(&circ, &arch)),
        "--parallel" => Ok(scmr_solve_par(&circ, &arch)),
        "--joint-optimize-par" => Ok(scmr_solve_joint_optimize_parallel(&circ, &arch)),
        _ => Err(IOError::InputErr),
    }?;
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: run-scmr <circuit> <arch> <mode>");
    }
    run_scmr(&args[1], &args[2], &args[3])
}
