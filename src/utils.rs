use petgraph::Graph;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use serde_json::Value;
use serde::{Serialize, Deserialize};
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize)]
pub struct Qubit(usize);

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default, Serialize)]
pub struct Location(usize);

pub type QubitMap = HashMap<Qubit, Location>;

impl Location {
    pub fn new(i: usize) -> Self {
        return Location(i);
    }
    pub fn get_index(&self) -> usize {
        return self.0;
    }
}
#[derive(Clone, Debug, Eq, Hash)]
pub struct Gate {
    name: String,
    pub qubits: Vec<Qubit>,
    id: usize,
}
impl Serialize for Gate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{} {:?}", self.name, self.qubits))
    }
}

#[derive(Clone, Debug)]
pub struct Circuit {
    pub gates: Vec<Gate>,
    pub qubits: HashSet<Qubit>,
}

impl PartialEq for Gate {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Circuit {
    pub fn get_front_layer(&self) -> Vec<Gate> {
        let mut blocked_qubits: HashSet<Qubit> = HashSet::new();
        let mut gates = Vec::new();
        for g in &self.gates {
            let gate_qubits = &g.qubits;
            let not_blocked = gate_qubits.iter().all(|q| !blocked_qubits.contains(q));
            if not_blocked {
                gates.push(g.clone());
            }
            blocked_qubits.extend(gate_qubits);
        }
        return gates;
    }
    pub fn remove_gates(&mut self, gates: &Vec<Gate>) {
        self.gates.retain(|g| !gates.contains(g));
    }
}

pub fn circuit_from_gates(gates: Vec<Gate>) -> Circuit {
    let mut qubits = HashSet::new();
    for gate in &gates {
        for qubit in &gate.qubits {
            qubits.insert(*qubit);
        }
    }
    return Circuit { gates, qubits };
}

pub trait GateImplementation {}

#[derive(Clone, Debug, Serialize)]
pub struct Step<T: GateImplementation> {
    pub map: QubitMap,
    pub implementation: HashMap<Gate, T>,
}

impl<G: GateImplementation> Step<G> {
    pub fn max_step<A: Architecture>(
        &mut self,
        executable: &Vec<Gate>,
        arch: &A,
        implement_gate: fn(&Step<G>, &A, &Gate) -> Option<G>,
    ) {
        assert!(self.implementation.is_empty());
        for gate in executable {
            let implementation = implement_gate(self, arch, gate);
            match implementation {
                None => continue,
                Some(implementation) => {
                    self.implementation.insert(gate.clone(), implementation);
                }
            }
        }
    }

    pub fn gates(&self) -> Vec<Gate> {
        return self.implementation.keys().cloned().collect();
    }
}

pub trait Transition<T: GateImplementation> {
    fn apply(&self, step: &Step<T>) -> Step<T>;
    fn repr(&self) -> String;
    fn cost(&self) -> f64;
}

pub trait Architecture {
    fn get_locations(&self) -> Vec<Location>;
}

pub fn extract_cnots(filename: &str) -> Circuit {
    let file = File::open(filename).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut gates = Vec::new();
    let mut qubits = HashSet::new();
    let mut id = 0;
    let re = Regex::new(r"cx\s+q\[(\d+)\],\s*q\[(\d+)\];").unwrap();
    for line in lines {
        let line_str = line.unwrap();
        let caps = re.captures(&line_str);
        match caps {
            None => continue,
            Some(c) => {
                let q1 = Qubit(c.get(1).unwrap().as_str().parse::<usize>().unwrap());
                let q2 = Qubit(c.get(2).unwrap().as_str().parse::<usize>().unwrap());
                qubits.insert(q1);
                qubits.insert(q2);
                let gate = Gate {
                    name: "cx".to_string(),
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
        nodes.push(g.add_node(Location(i)));
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
        g.add_edge(nodes[a], nodes[b], ());

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