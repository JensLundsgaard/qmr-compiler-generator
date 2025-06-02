use core::arch;

use builtin::ilqaa::{ilq_solve, ilq_solve_joint_optimize_parallel};
use serde_json;
use solver::utils::{self, IOError};

fn run_ilq(
    circ_path: &str,
    arch_type: &str,
    stack_depth_arg: &str,
    solve_mode: &str,
) -> Result<(), IOError> {
    let circ = utils::extract_gates(circ_path, &["T", "CX"]);
    let stack_depth = stack_depth_arg
        .parse()
        .expect("stack depth should be usize");

    let arch = match arch_type {
        "compact" => Ok(builtin::ilqaa::compact_layout(
            circ.qubits.len(),
            stack_depth,
        )),
        "square_sparse" => Ok(builtin::ilqaa::square_sparse_layout(
            circ.qubits.len(),
            stack_depth,
        )),
        _ => Err(IOError::InputErr),
    }?;
    let res = match solve_mode {
        "--onepass" => Ok(ilq_solve(&circ, &arch)),
        "--joint-optimize-par" => Ok(ilq_solve_joint_optimize_parallel(&circ, &arch)),
        _ => Err(IOError::InputErr),
    }?;
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        println!("Usage: run-ilq <circuit> <arch> <stack-depth> <mode>");
    }
    run_ilq(&args[1], &args[2], &args[3], &args[4])
}
