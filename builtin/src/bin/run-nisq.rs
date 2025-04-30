use std::fs::File;

use solver::utils::{self, graph_from_json_entry, IOError};
use builtin::nisq::{self, nisq_solve, nisq_solve_sabre};
use serde_json::{self, Value};


fn run_nisq(circ_path: &str, arch_path : &str, solve_mode : &str) -> Result<(), IOError> {
    let circ = utils::extract_cnots(circ_path);
    let file = File::open(arch_path).expect("Opening architecture file");
    let parsed: Value = serde_json::from_reader(file)
        .expect("Parsing architecture file");
    let g = graph_from_json_entry(parsed["graph"].clone());
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
    if args.len() != 4 {
    println!("Usage: run-nisq <circuit> <arch> <solve-mode>");
}
    run_nisq(&args[1], &args[2], &args[3])
}
