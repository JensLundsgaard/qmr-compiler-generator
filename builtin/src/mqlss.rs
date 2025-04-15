use itertools::Itertools;
use rustworkx_core::{
    petgraph::{self, graph::NodeIndex},
    steiner_tree::steiner_tree,
    Result,
};
use serde::Serialize;
use solver::{backend::solve, structures::*, utils::*};
use std::{
    collections::{HashMap, HashSet},
    iter::empty,
};
#[derive(Clone)]
pub struct MQLSSArchitecture {
    pub width: usize,
    pub height: usize,
    pub alg_qubits: Vec<Location>,
    pub magic_state_qubits: Vec<Location>,
}

impl Architecture for MQLSSArchitecture {
    fn locations(&self) -> Vec<Location> {
        return self.alg_qubits.clone();
    }

    fn graph(
        &self,
    ) -> (
        petgraph::Graph<Location, ()>,
        HashMap<Location, petgraph::graph::NodeIndex>,
    ) {
        return self.get_graph();
    }
}
impl MQLSSArchitecture {
    fn get_graph(
        &self,
    ) -> (
        petgraph::Graph<Location, ()>,
        HashMap<Location, petgraph::graph::NodeIndex>,
    ) {
        let mut g = petgraph::Graph::new();
        let mut index_map = HashMap::new();
        for i in 0..self.height {
            for j in 0..self.width {
                let loc = Location::new(i * self.width + j);
                let v = g.add_node(loc);
                index_map.insert(loc, v);
            }
        }
        for i in 0..self.height {
            for j in 0..self.width {
                // edge to above
                if i > 0 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new((i - 1) * self.width + j)];
                    g.update_edge(v1, v2, ());
                    g.update_edge(v2, v1, ());
                }
                // edge to below
                if i < self.height - 1 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new((i + 1) * self.width + j)];
                    g.add_edge(v1, v2, ());
                    g.update_edge(v2, v1, ());
                }
                // edge to left
                if j > 0 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new(i * self.width + j - 1)];
                    g.update_edge(v1, v2, ());
                    g.update_edge(v2, v1, ());
                }
                // edge to right
                if j < self.width - 1 {
                    let v1 = index_map[&Location::new(i * self.width + j)];
                    let v2 = index_map[&Location::new(i * self.width + j + 1)];
                    g.update_edge(v1, v2, ());
                    g.update_edge(v2, v1, ());
                }
            }
        }
        return (g, index_map);
    }
}
#[derive(Debug, Serialize, Clone, Hash, PartialEq, Eq)]
pub struct MQLSSGateImplementation {
    used_nodes: Vec<Location>,
}
pub fn compact_layout(alg_qubit_count: usize) -> MQLSSArchitecture {
    let width = (2 * alg_qubit_count.div_ceil(2)) + 1;
    let height = 5;
    let mut alg_qubits = Vec::new();
    for i in (1..width - 1).step_by(2) {
        alg_qubits.push(Location::new(width + i));
        alg_qubits.push(Location::new(i + width * 3));
    }
    let mut perimeter = Vec::new();
    let top_edge = (0..width).map(|i| Location::new(i));
    let right_edge = (1..height).map(|i| Location::new(i * width + width - 1));
    let bottom_edge = (0..width - 1)
        .rev()
        .map(|i| Location::new(i + width * (height - 1)));
    let left_edge = (1..height - 1).rev().map(|i| Location::new(i * width));
    perimeter.extend(top_edge);
    perimeter.extend(right_edge);
    perimeter.extend(bottom_edge);
    perimeter.extend(left_edge);
    // iterate over every other location on the perimeter
    let mut magic_state_qubits = Vec::new();
    for i in (1..perimeter.len()).step_by(2) {
        magic_state_qubits.push(perimeter[i]);
    }
    return MQLSSArchitecture {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
    };
}

pub fn square_sparse_layout(alg_qubit_count: usize) -> MQLSSArchitecture {
    let agc = alg_qubit_count as f64;
    let width = 2 * (agc.sqrt().ceil() as usize) + 3;
    let height = width;
    let mut alg_qubits = Vec::new();
    let interior = |coord| coord > 0 && coord < width - 1;
    for i in 0..width * height {
        let (x, y) = (i % width, i / width);
        if interior(x) && interior(y) && x % 2 == 0 && y % 2 == 0 {
            alg_qubits.push(Location::new(i));
        }
    }
    let mut perimeter = Vec::new();
    let top_edge = (0..width).map(|i| Location::new(i));
    let right_edge = (1..height).map(|i| Location::new(i * width + width - 1));
    let bottom_edge = (0..width - 1)
        .rev()
        .map(|i| Location::new(i + width * (height - 1)));
    let left_edge = (1..height - 1).rev().map(|i| Location::new(i * width));
    perimeter.extend(top_edge);
    perimeter.extend(right_edge);
    perimeter.extend(bottom_edge);
    perimeter.extend(left_edge);
    let mut magic_state_qubits = Vec::new();
    for i in (1..perimeter.len()).step_by(2) {
        magic_state_qubits.push(perimeter[i]);
    }
    return MQLSSArchitecture {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
    };
}
impl GateImplementation for MQLSSGateImplementation {}
#[derive(Debug)]
struct IdTransition;
type MQLSSStep = Step<MQLSSGateImplementation>;
impl Transition<MQLSSGateImplementation, MQLSSArchitecture> for IdTransition {
    fn apply(&self, step: &MQLSSStep) -> MQLSSStep {
        return MQLSSStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self, _arch: &MQLSSArchitecture) -> f64 {
        0.0
    }
}

fn mqlss_transitions(_step: &MQLSSStep) -> Vec<IdTransition> {
    return vec![IdTransition];
}

fn mqlsss_step_cost(_step: &MQLSSStep, _arch: &MQLSSArchitecture) -> f64 {
    return 1.0;
}

fn mqlss_implement_gate(
    step: &MQLSSStep,
    arch: &MQLSSArchitecture,
    gate: &Gate,
) -> Vec<MQLSSGateImplementation> {
    let (mut graph, mut loc_to_node) = arch.get_graph();
    let mut impls = vec![];
    for loc in &arch.magic_state_qubits {
        assert!(!arch.alg_qubits.clone().into_iter().any(|l| l == *loc));
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }
    for loc in step.map.values().into_iter() {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[loc]);
        loc_to_node.insert(old_last, loc_to_node[loc]);
        loc_to_node.remove(loc);
    }
    for loc in step
        .implemented_gates
        .iter()
        .map(|x| x.implementation.used_nodes.clone())
        .flatten()
    {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[&loc]);
        loc_to_node.insert(old_last, loc_to_node[&loc]);
        loc_to_node.remove(&loc);
    }
    let mut qubit_terminals = vec![];
    match &gate.operation {
        Operation::PauliRot { axis, angle } if *angle == (1, 8) || *angle == (-1, 8) => {
            let msf_neighbors = arch
                .magic_state_qubits
                .clone()
                .into_iter()
                .map(|m| horizontal_neighbors(m, arch.width))
                .flatten()
                .collect();
            qubit_terminals.push(msf_neighbors);
            
            for i in 0..gate.qubits.len() {
                match axis[i] {
                    PauliTerm::PauliX => {
                        qubit_terminals.push(horizontal_neighbors(step.map[&gate.qubits[i]], arch.width));
                    }
                    PauliTerm::PauliY => {
                        qubit_terminals.push(vertical_neighbors(step.map[&gate.qubits[i]], arch.width, arch.height));
                        qubit_terminals.push(horizontal_neighbors(step.map[&gate.qubits[i]], arch.width));
                    }
                    PauliTerm::PauliZ => {
                        qubit_terminals.push(vertical_neighbors(step.map[&gate.qubits[i]], arch.width, arch.height));
                    }
                    PauliTerm::PauliI => {}
                }
            }
        }
        Operation::PauliMeasurement { sign: _, axis } => {
            for i in 0..gate.qubits.len() {
                match axis[i] {
                    PauliTerm::PauliX => {
                        qubit_terminals.push(horizontal_neighbors(step.map[&gate.qubits[i]], arch.width));
                    }
                    PauliTerm::PauliY => {
                        qubit_terminals.push(vertical_neighbors(step.map[&gate.qubits[i]], arch.width, arch.height));
                        qubit_terminals.push(horizontal_neighbors(step.map[&gate.qubits[i]], arch.width));
                    }
                    PauliTerm::PauliZ => {
                        qubit_terminals.push(vertical_neighbors(step.map[&gate.qubits[i]], arch.width, arch.height));
                    }
                    PauliTerm::PauliI => {}
                }
            }
        }
        _ => {panic!("Tried to do MQLSS with gate {:?}, which is not supported", gate)},
    }
        let terminal_sets = qubit_terminals.into_iter().multi_cartesian_product().filter(|v|v.iter().all(|l| loc_to_node.contains_key(l)));

        for terminal_set in terminal_sets {
            let indices: Vec<NodeIndex> =
                terminal_set.into_iter().map(|x| loc_to_node[&x]).collect();
            let steiner_tree_res = steiner_tree(&graph, &indices, |_| Ok::<f64, ()>(1.0));

            if let Ok(Some(tree)) = steiner_tree_res {
                let locations = tree
                    .used_node_indices
                    .into_iter()
                    .map(|n| &graph[NodeIndex::new(n)])
                    .cloned()
                    .collect();
                impls.push(MQLSSGateImplementation {
                    used_nodes: locations,
                });
            }
        }
        return impls;
}

pub fn mqlss_solve(c: &Circuit, a: &MQLSSArchitecture) -> CompilerResult<MQLSSGateImplementation> {
    return solve(
        c,
        a,
        &mqlss_transitions,
        mqlss_implement_gate,
        mqlsss_step_cost,
        None,
        true,
    );
}
