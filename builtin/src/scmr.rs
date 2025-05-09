use std::{
    collections::{HashMap, HashSet},
    iter::empty,
};

use itertools::{sorted, Itertools};
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

pub fn square_sparse_layout(alg_qubit_count: usize) -> ScmrArchitecture {
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
impl Transition<ScmrGateImplementation, ScmrArchitecture> for IdTransition {
    fn apply(&self, step: &ScmrStep) -> ScmrStep {
        return ScmrStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self, _arch: &ScmrArchitecture) -> f64 {
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
) -> Option<ScmrGateImplementation> {
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
    let (starts, ends) = match &gate.operation {
        Operation::CX => {
            let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
            (
                vertical_neighbors(cpos, arch.width, arch.height),
                horizontal_neighbors(tpos, arch.width),
            )
        }
        Operation::T => {
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
        _ => (vec![], vec![]),
    };
    let mut best: Option<(i32, Vec<NodeIndex>)> = None;

    for start in &starts {
        for end in &ends {
            if loc_to_node.contains_key(start) && loc_to_node.contains_key(end) {
                let res = petgraph::algo::astar(
                    &graph,
                    loc_to_node[&start],
                    |finish| finish == loc_to_node[&end],
                    |_e| 1,
                    |_| 0,
                );
                if best.is_none()
                    || ((&res).is_some() && &res.as_ref().unwrap().0 < &best.as_ref().unwrap().0)
                {
                    best = res;
                }
            }
        }
    }
    return best.map(|(_cost, path)| ScmrGateImplementation {
        path: path.into_iter().map(|n| graph[n]).collect(),
    });
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
    let blocked = mapped
        .into_iter()
        .chain(magic_states.into_iter())
        .chain(paths.into_iter())
        .collect();
    let (starts, ends) = match &gate.operation {
        Operation::CX => {
            let (cpos, tpos) = (step.map[&gate.qubits[0]], step.map[&gate.qubits[1]]);
            (
                vertical_neighbors(cpos, arch.width, arch.height),
                horizontal_neighbors(tpos, arch.width),
            )
        }
        Operation::T => {
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
        _ => (vec![], vec![]),
    };
    all_paths(arch, starts, ends, blocked).map(|p| ScmrGateImplementation { path: p })
}

fn mapping_heuristic(arch: &ScmrArchitecture, circ: &Circuit, map: &QubitMap) -> f64 {
    struct Range {
        x: (usize, usize),
        y: (usize, usize),
    }
    let mut overlaps = 0;
    fn get_gate_range(gate: &Gate, arch: &ScmrArchitecture, map: &QubitMap) -> Range {
        match &gate.operation {
            Operation::CX => {
                let (ctrl_x, ctrl_y) = (
                    map[&gate.qubits[0]].get_index() % arch.width,
                    (map[&gate.qubits[0]].get_index() / arch.width),
                );
                let (tar_x, tar_y) = (
                    map[&gate.qubits[0]].get_index() % arch.width,
                    (map[&gate.qubits[0]].get_index() / arch.width),
                );
                let x_range = if ctrl_x < tar_x {
                    (ctrl_x, tar_x)
                } else {
                    (tar_x, ctrl_x)
                };
                let y_range = if ctrl_y < tar_y {
                    (ctrl_y, tar_y)
                } else {
                    (tar_y, ctrl_y)
                };
                return Range {
                    x: x_range,
                    y: y_range,
                };
            }
            Operation::T => {
                let (qubit_x, qubit_y) = (
                    map[&gate.qubits[0]].get_index() % arch.width,
                    (map[&gate.qubits[0]].get_index() / arch.width),
                );
                let magic_states_2d = arch
                    .magic_state_qubits
                    .iter()
                    .map(|s| (s.get_index() % arch.width, s.get_index() / arch.width));
                let (msf_x, msf_y) = magic_states_2d
                    .min_by_key(|(x, y)| {
                        (*x as isize - qubit_x as isize).abs()
                            + (*y as isize - qubit_y as isize).abs()
                    })
                    .expect("should not be routing T gates with no magic states");
                let x_range = if msf_x < qubit_x {
                    (msf_x, qubit_x)
                } else {
                    (qubit_x, msf_x)
                };
                let y_range = if msf_y < qubit_y {
                    (msf_y, qubit_y)
                } else {
                    (qubit_y, msf_y)
                };
                return Range {
                    x: x_range,
                    y: y_range,
                };
            }
            Operation::PauliRot { axis, angle } => panic!("did not expect PauliRot gate"),
            Operation::PauliMeasurement { sign, axis } => {
                panic!("did not expect PauliMeasure gate")
            }
        }
    }
    fn overlap(r1: Range, r2: Range) -> bool {
        if r1.x.0 < r2.x.1 || r2.x.1 < r1.x.0 {
            return false;
        }
        if r1.y.1 < r2.y.0 || r2.y.1 < r1.y.0 {
            return false;
        }
        return true;
    }
    let mut c = circ.clone();
    let layers = circuit_to_layers(&mut c);
    for layer in layers {
        for (g1, g2) in layer.iter().tuple_combinations() {
            let r1 = get_gate_range(g1, arch, map);
            let r2 = get_gate_range(g2, arch, map);
            if overlap(r1, r2) {
                overlaps += 1;
            }
        }
    }
    return overlaps as f64;
}

pub fn scmr_solve(c: &Circuit, a: &ScmrArchitecture) -> CompilerResult<ScmrGateImplementation> {
    return solve(
        c,
        a,
        &scmr_transitions,
        scmr_implement_gate_alt,
        scmr_step_cost,
        Some(mapping_heuristic),
        true,
    );
}
