use petgraph::Graph;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use serde_json::Value;
use serde::Serialize;
use crate::scmr::ScmrArchitecture;
use crate::structures::*;

pub fn extract_cnots(filename: &str) -> Circuit {
    let file = File::open(filename).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut gates = Vec::new();
    let mut qubits = HashSet::new();
    let mut id = 0;
    let cx_re = Regex::new(r"cx\s+q\[(\d+)\],\s*q\[(\d+)\];").unwrap();
    for line in lines {
        let line_str = line.unwrap();
        let cx_caps = cx_re.captures(&line_str);
        match cx_caps {
            None => continue,
            Some(c) => {
                let q1 = Qubit::new(c.get(1).unwrap().as_str().parse::<usize>().unwrap());
                let q2 = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                qubits.insert(q1);
                qubits.insert(q2);
                let gate = Gate {
                    gate_type : GateType::CX,
                    qubits: vec![q1, q2],
                    id,
                };
                gates.push(gate);
                id += 1;
            }
        }
    }
    return Circuit { gates, qubits };
}

pub fn extract_scmr_gates(filename: &str) -> Circuit {
    let file = File::open(filename).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut gates = Vec::new();
    let mut qubits = HashSet::new();
    let mut id = 0;
    let cx_re = Regex::new(r"cx\s+q\[(\d+)\],\s*q\[(\d+)\];").unwrap();
    let t_re = Regex::new(r"(t|tdg)\s+q\[(\d+)\];").unwrap();
    for line in lines {
        let line_str = line.unwrap();
        let cx_caps = cx_re.captures(&line_str);
        let t_caps = t_re.captures(&line_str);
        match cx_caps {
            None => {match t_caps {
                None => continue,
                Some(c) => {
                    let q = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                    qubits.insert(q);
                    let gate = Gate {
                        gate_type : GateType::T,
                        qubits: vec![q],
                        id,
                    };
                    gates.push(gate);
                    id += 1;
                }
            }},
            Some(c) => {
                let q1 = Qubit::new(c.get(1).unwrap().as_str().parse::<usize>().unwrap());
                let q2 = Qubit::new(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                qubits.insert(q1);
                qubits.insert(q2);
                let gate = Gate {
                    gate_type : GateType::CX,
                    qubits: vec![q1, q2],
                    id,
                };
                gates.push(gate);
                id += 1;
            }
        }
    }
    return Circuit { gates, qubits };
}

pub fn path_graph(n: usize) -> Graph<Location, ()> {
    let mut g = Graph::new();
    let mut nodes = Vec::new();
    for i in 0..n {
        nodes.push(g.add_node(Location::new(i)));
    }
    for i in 0..n - 1 {
        g.add_edge(nodes[i], nodes[i + 1], ());
        g.add_edge(nodes[i + 1], nodes[i], ());
    }
    return g;
}

pub fn drop_zeros_and_normalize<T: IntoIterator<Item = (f64, f64)> + Clone>(
    weighted_values: T,
) -> f64 {
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    for (w, v) in weighted_values.clone() {
        if v != 0.0 {
            total_weight += w;
        }
    }
    for (w, v) in weighted_values.clone() {
        {
            let normalized = w / total_weight;
            weighted_sum += normalized * v;
        }
    }
    return weighted_sum;
}

fn graph_from_edge_vec(edges: Vec<(Location, Location)>) -> Graph<Location, ()> {
    let mut nodes = HashMap::new();
    let mut g = Graph::new();
    for (a, b) in &edges {
        if !nodes.contains_key(a) {
            nodes.insert(a, g.add_node(*a));
        }
        if !nodes.contains_key(b) {
            nodes.insert(b, g.add_node(*b));
        }
        // edges are undirected
        g.update_edge(nodes[a], nodes[b], ());
        g.update_edge(nodes[b], nodes[a], ());

    }
    return  g;
}

pub fn graph_from_file(filename : &str) -> Graph<Location, ()> {
    let file = File::open(filename).unwrap();
    let parsed : Value = serde_json::from_reader(file).unwrap();
    let edges = parsed
        .as_array()
        .expect("Expected an array of arrays")
        .iter()
        .map(|inner| {
            let array = inner.as_array().expect("Inner element is not an array");
            if array.len() != 2 {
                panic!("Each edge must have exactly 2 elements");
            }
            let first = array[0].as_u64().expect("Element is not a positive integer") as usize;
            let second = array[1].as_u64().expect("Element is not a positive integer") as usize;
            (Location::new(first), Location::new(second))
        })
        .collect();
    return graph_from_edge_vec(edges);
}
#[derive(Serialize, Debug)]
pub struct CompilerResult<T : GateImplementation> {
    pub steps : Vec<Step<T>>,
    pub transitions : Vec<String>,
    pub cost : f64,

}

pub fn vertical_neighbors(loc : &Location, width : usize, height : usize) -> Vec<Location>{
    let mut neighbors = Vec::new();
    if loc.get_index() / width > 0 {
        neighbors.push(Location::new(loc.get_index() - width));
    }
    if loc.get_index() / width < height - 1 {
        neighbors.push(Location::new(loc.get_index() + width));
    }
    return neighbors;
}

pub fn horizontal_neighbors(loc : &Location, width : usize) -> Vec<Location>{
    let mut neighbors = Vec::new();
    if loc.get_index() % width > 0 {
        neighbors.push(Location::new(loc.get_index() - 1));
    }
    if loc.get_index() % width < width - 1 {
        neighbors.push(Location::new(loc.get_index()+1));
    }
    return neighbors;
}

pub fn compact_layout(alg_qubit_count : usize) -> ScmrArchitecture{
    let width =(2*alg_qubit_count.div_ceil(2))+1;
    let height = 5;
    let mut alg_qubits = Vec::new();
    for i in (1..width-1).step_by(2){
        alg_qubits.push(Location::new(width+i));
        alg_qubits.push(Location::new(i+width*3));
    }
    let mut perimeter = Vec::new();
    let top_edge = (0..width).map(|i| Location::new(i));
    let right_edge = (1..height).map(|i| Location::new(i*width+width-1));
    let bottom_edge = (0..width-1).rev().map(|i| Location::new(i+width*(height-1)));
    let left_edge = (1..height-1).rev().map(|i| Location::new(i*width));
    perimeter.extend(top_edge);
    perimeter.extend(right_edge);
    perimeter.extend(bottom_edge);
    perimeter.extend(left_edge);
    // iterate over every other location on the perimeter
    let mut magic_state_qubits = Vec::new();
    for i in (1..perimeter.len()).step_by(2){
        magic_state_qubits.push(perimeter[i]);
    }
    return ScmrArchitecture{
        width,
        height,
        alg_qubits,
        magic_state_qubits,
    };
}


fn swap_keys(
    map: &HashMap<Qubit, Location>,
    locs: (Location, Location),
) -> HashMap<Qubit, Location> {
    let mut new_map = map.clone();
    for (qubit, loc) in map {
        if loc == &locs.0 {
            new_map.insert(*qubit, locs.1);
        } else if loc == &locs.1 {
            new_map.insert(*qubit, locs.0);
        }
    }
    return new_map;
}