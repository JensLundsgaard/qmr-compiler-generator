use std::collections::{HashMap, HashSet};

use petgraph::{graph::NodeIndex, Graph};
use serde::Serialize;
use solver::{
    backend::solve,
    structures::{
        Architecture, Circuit, CompilerResult, Gate, GateImplementation, Location, Operation,
        QubitMap, Step, Transition,
    },
    utils::{horizontal_neighbors, vertical_neighbors},
};

#[derive(Clone)]
pub struct ILQArch {
    stack_depth: usize,
    width: usize,
    height: usize,
    alg_qubits: Vec<Location>,
    pub magic_state_qubits: Vec<Location>,
}

impl Architecture for ILQArch {
    fn locations(&self) -> Vec<Location> {
        return self.alg_qubits.clone();
    }

    fn graph(
        &self,
    ) -> (
        petgraph::Graph<Location, ()>,
        std::collections::HashMap<Location, petgraph::prelude::NodeIndex>,
    ) {
        return self.get_graph();
    }
}

impl ILQArch {
    fn get_graph(&self) -> (Graph<Location, ()>, HashMap<Location, NodeIndex>) {
        let mut g = Graph::new();
        let mut index_map = HashMap::new();
        for i in 0..self.height {
            for j in 0..self.width {
                for k in 0..self.stack_depth {
                    let loc =
                        Location::new(i * self.width * self.stack_depth + j * self.stack_depth + k);
                    let v = g.add_node(loc);
                    index_map.insert(loc, v);
                }
            }
        }
        for i in 0..self.height {
            for j in 0..self.width {
                // edge to above
                if i > 0 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                (i - 1) * self.width * self.stack_depth + j * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to below
                if i < self.height - 1 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                (i + 1) * self.width * self.stack_depth + j * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to left
                if j > 0 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                i * self.width * self.stack_depth + (j - 1) * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
                // edge to right
                if j < self.width - 1 {
                    for k1 in 0..self.stack_depth {
                        for k2 in 0..self.stack_depth {
                            let v1 = index_map[&Location::new(
                                i * self.width * self.stack_depth + j * self.stack_depth + k1,
                            )];
                            let v2 = index_map[&Location::new(
                                i * self.width * self.stack_depth + (j + 1) * self.stack_depth + k2,
                            )];
                            g.update_edge(v1, v2, ());
                            g.update_edge(v2, v1, ());
                        }
                    }
                }
            }
        }
        return (g, index_map);
    }
}

pub fn compact_layout(alg_qubit_count: usize, stack_depth: usize) -> ILQArch {
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
    return ILQArch {
        width,
        height,
        alg_qubits,
        magic_state_qubits,
        stack_depth,
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
pub enum ILQGateImplementation {
    Transversal { ctrl: Location, tar: Location },
    LatticeSurgery { path: Vec<Location> },
}
impl GateImplementation for ILQGateImplementation {}
type ILQStep = Step<ILQGateImplementation>;
#[derive(Debug)]
struct IdTransition;
impl Transition<ILQGateImplementation, ILQArch> for IdTransition {
    fn apply(&self, step: &ILQStep) -> ILQStep {
        return ILQStep {
            implemented_gates: HashSet::new(),
            map: step.map.clone(),
        };
    }
    fn repr(&self) -> String {
        return "id".to_string();
    }

    fn cost(&self, _arch: &ILQArch) -> f64 {
        0.0
    }
}

fn ilq_transitions(_step: &ILQStep) -> Vec<IdTransition> {
    return vec![IdTransition];
}

fn ilq_step_cost(_step: &ILQStep, _arch: &ILQArch) -> f64 {
    return 1.0;
}

fn ilq_implement_gate(
    step: &ILQStep,
    arch: &ILQArch,
    gate: &Gate,
) -> Option<ILQGateImplementation> {
    let (mut graph, mut loc_to_node) = arch.get_graph();
    if gate.operation == Operation::CX
        && (step.map[&gate.qubits[0]].get_index() / arch.stack_depth)
            == (step.map[&gate.qubits[1]].get_index() / arch.stack_depth)
    {
        return Some(ILQGateImplementation::Transversal {
            ctrl: step.map[&gate.qubits[0]],
            tar: step.map[&gate.qubits[1]],
        });
    } else {
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
                        || ((&res).is_some()
                            && &res.as_ref().unwrap().0 < &best.as_ref().unwrap().0)
                    {
                        best = res;
                    }
                }
            }
        }
        return best.map(|(_cost, path)| ILQGateImplementation::LatticeSurgery {
            path: path.into_iter().map(|n| graph[n]).collect(),
        });
    }
}

fn mapping_heuristic(_a: &ILQArch, c: &Circuit, m: &QubitMap) -> f64 {
    let mut cost = 0;
    for gate in &c.gates {
        if gate.operation == Operation::CX
            && m[&gate.qubits[0]].get_index() / 4 != m[&gate.qubits[1]].get_index() / 4
        {
            cost += 1;
        }
    }
    return cost as f64;
}

pub fn ilq_solve(c: &Circuit, a: &ILQArch) -> CompilerResult<ILQGateImplementation> {
    return solve(
        c,
        a,
        &ilq_transitions,
        ilq_implement_gate,
        ilq_step_cost,
        Some(mapping_heuristic),
        true,
    );
}
