use solver::utils::{self, IOError};
use builtin::nisq::{self, nisq_solve, nisq_solve_sabre};
use serde_json;


fn run_nisq(circ_path: &str, arch_path : &str, solve_mode : &str) -> Result<(), IOError> {
    let circ = utils::extract_scmr_gates(circ_path);
    let g = utils::graph_from_file(arch_path);
    let arch = nisq::NisqArchitecture::new(g);
    let res =   match solve_mode {
        "--sabre" => Ok(nisq_solve_sabre(&circ, &arch)),
        "--onepass" => Ok(nisq_solve(&circ, &arch)),
        _ => Err(IOError::InputErr)
    }?;
    serde_json::to_writer(std::io::stdout(), &res).map_err(IOError::OutputErr)
}
fn main() -> Result<(), IOError>  {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
    println!("Usage: run-nisq <circuit> <arch>");
}
    run_nisq(&args[1], &args[2], &args[3])
}
