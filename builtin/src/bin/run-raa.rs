use solver::utils::{self, IOError};
use builtin::raa::{self, raa_joint_optimize_parallel, raa_solve, raa_solve_sabre};
use serde_json;


fn run_raa(circ_path: &str, solve_mode : &str) -> Result<(), IOError> {
    let circ = utils::extract_cnots(circ_path);
    let size = (circ.gates.len() as f64).sqrt().ceil() as usize;
    let arch = raa::RaaArchitecture { width : size, height : size};
    let res =   match solve_mode {
        "--sabre" => Ok(raa_solve_sabre(&circ, &arch)),
        "--onepass" => Ok(raa_solve(&circ, &arch)),
        "--joint-optimize-par" => Ok(raa_joint_optimize_parallel(&circ, &arch)),
        _ => Err(IOError::InputErr)
    }?;
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError>  {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
    println!("Usage: run-raa <circuit> <arch>");
}
    run_raa(&args[1], &args[2])
}
