use std::{collections::{HashMap, HashSet}, iter::empty};

use petgraph::{algo::all_simple_paths, graph::NodeIndex, Graph};
use serde::Serialize;

use solver::{backend::solve, structures::*, utils::*};
#[derive(Debug, Serialize, Clone)]
pub struct ScmrArchitecture {
    pub width: usize,
    pub height: usize,
    pub alg_qubits: Vec<Location>,
    pub magic_state_qubits: Vec<Location>,
}

impl Architecture for ScmrArchitecture {
    fn locations(&self) -> Vec<Location> {
        return self.alg_qubits.clone();
    }

    fn graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        return self.get_graph();
    }
}
impl ScmrArchitecture {
    fn get_graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        let mut g = Graph::new();
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

pub fn compact_layout(alg_qubit_count: usize) -> ScmrArchitecture {
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
    return ScmrArchitecture {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
    };
}
#[derive(Debug, Serialize, Clone, Hash, PartialEq, Eq)]
pub struct ScmrGateImplementation {
    path: Vec<Location>,
}
impl GateImplementation for ScmrGateImplementation {}

type ScmrStep = Step<ScmrGateImplementation>;
#[derive(Debug)]
struct IdTransition;
impl Transition<ScmrGateImplementation> for IdTransition {
    fn apply(&self, step: &ScmrStep) -> ScmrStep {
        return ScmrStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self) -> f64 {
        0.0
    }
}

fn scmr_transitions(_step: &ScmrStep) -> Vec<IdTransition> {
    return vec![IdTransition];
}

fn scmr_step_cost(_step: &ScmrStep, _arch: &ScmrArchitecture) -> f64 {
    return 1.0;
}

fn scmr_implement_gate(
    step: &ScmrStep,
    arch: &ScmrArchitecture,
    gate: &Gate,
) -> Vec<ScmrGateImplementation> {
    let (mut graph, mut loc_to_node) = arch.get_graph();
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
        .map(|x| x.implementation.path.clone())
        .flatten()
    {
        let old_last = graph[graph.node_indices().last().unwrap()];
        graph.remove_node(loc_to_node[&loc]);
        loc_to_node.insert(old_last, loc_to_node[&loc]);
        loc_to_node.remove(&loc);
    }
    let (starts, ends) = match &gate.gate_type {
        GateType::CX => {
            let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
            (
                vertical_neighbors(cpos, arch.width, arch.height),
                horizontal_neighbors(tpos, arch.width),
            )
        }
        GateType::T => {
            let pos = step.map[&gate.qubits[0]];
            let target_neighbors = vertical_neighbors(pos, arch.width, arch.height);
            let msf_neighors = arch
                .magic_state_qubits
                .clone()
                .into_iter()
                .map(|m| horizontal_neighbors(m, arch.width))
                .flatten()
                .collect();
            (target_neighbors, msf_neighors)
        }
    };
    let mut paths:  Vec<Vec<NodeIndex>> = Vec::new();

    for start in &starts {
        for end in &ends {
            if loc_to_node.contains_key(start) && loc_to_node.contains_key(end) {
                let res: Vec<Vec<NodeIndex>> = petgraph::algo::all_simple_paths(
                    &graph,
                    loc_to_node[&start],
                    loc_to_node[&end],
                    0,
                    None,
                ).collect();
                paths.extend(res);
            }
        }
    }
return paths.into_iter().map(|path| ScmrGateImplementation { path: path.into_iter().map(|x| graph[x]).collect() }).collect();

}

fn scmr_implement_gate_alt(
    step: &ScmrStep,
    arch: &ScmrArchitecture,
    gate: &Gate,
) -> impl Iterator<Item = ScmrGateImplementation> {
    let paths: Vec<_> = step
        .implemented_gates
        .iter()
        .map(|x| x.implementation.path.clone())
        .flatten()
        .collect();
    let mapped: Vec<_> = step.map.values().cloned().collect();
    let magic_states = arch.magic_state_qubits.clone();
    let blocked  = mapped.into_iter().chain(magic_states.into_iter()).chain(paths.into_iter()).collect();
    let (starts, ends) = match &gate.gate_type {
        GateType::CX => {
            let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
            (
                vertical_neighbors(cpos, arch.width, arch.height),
                horizontal_neighbors(tpos, arch.width),
            )
        }
        GateType::T => {
            let pos = step.map[&gate.qubits[0]];
            let target_neighbors = vertical_neighbors(pos, arch.width, arch.height);
            let msf_neighors = arch
                .magic_state_qubits
                .clone()
                .into_iter()
                .map(|m| horizontal_neighbors(m, arch.width))
                .flatten()
                .collect();
            (target_neighbors, msf_neighors)
        }
    };
    all_paths(arch.clone(), starts, ends, blocked).map(|p| ScmrGateImplementation{path: p})

}


pub fn scmr_solve(c: &Circuit, a: &ScmrArchitecture) -> CompilerResult<ScmrGateImplementation> {
    return solve(
        c,
        a,
        &scmr_transitions,
        scmr_implement_gate_alt,
        scmr_step_cost,
        None,
        true
    );
}
