use std::fs;

use serde::Deserialize;
use once_cell::sync::Lazy;
#[derive(Deserialize, Debug)]
pub struct SolverConfig{
    pub alpha : f64,
    pub beta : f64,
    pub gamma : f64,
    pub delta : f64, 
    pub mapping_search_initial_temp : f64,
    pub mapping_search_term_temp : f64,
    pub mapping_search_cool_rate : f64,
    pub routing_search_initial_temp : f64,
    pub routing_search_term_temp : f64,
    pub routing_search_cool_rate : f64,
    pub sabre_iterations : usize,
    pub isom_search_timeout : u64



}



pub static CONFIG: Lazy<SolverConfig> = Lazy::new(|| {
    let data = fs::read_to_string("config.json").expect("Failed to read config file");
    serde_json::from_str(&data).expect("Failed to parse config file")
});